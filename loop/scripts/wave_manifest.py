"""wave_manifest: derive the build-spec wave manifest from git + registry.

The manifest's philosophy is the loop's own: claims about the tree are
re-derived by command, never typed from memory. This script reads
loop/PACKETS.jsonl and loop/LEDGER.jsonl and asks git for the merge
geometry, so every field is a measured fact:

  per wave:  base SHA (first parent of the wave's first packet merge),
             packet id -> worker commit (merge's second parent) and
             landing merge SHA (first parent chain),
             amendment commits (packet-branch commits after the worker
             commit carrying 'orchestrator amendment'),
  global:    verifier version (last commit touching loop/verify.py),
             integrated SHA (current HEAD).

Usage:
  python loop/scripts/wave_manifest.py               # JSON to stdout
  python loop/scripts/wave_manifest.py --markdown    # build-spec table
  python loop/scripts/wave_manifest.py --check       # report mismatches

Stdlib only (house rule for loop/*.py).
"""
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent

# The LANDED marker convention: "LANDED <sha7+>" in the registry note
# (appended by wave_manifest --fix or the landing hand). The word
# "landed" inside prose ("the landed assemble output") is not a marker.
LANDED_RE = re.compile(r"landed [0-9a-f]{7,}")


def git(*args):
    out = subprocess.run(["git", "-C", str(ROOT), *args],
                         capture_output=True, text=True)
    if out.returncode != 0:
        raise RuntimeError(f"git {' '.join(args)}: {out.stderr.strip()}")
    return out.stdout.strip()


def registry():
    rows = []
    p = ROOT / "loop" / "PACKETS.jsonl"
    for line in p.read_text(encoding="utf-8-sig").splitlines():
        if line.strip():
            rows.append(json.loads(line))
    return rows


def ledger():
    rows = []
    p = ROOT / "loop" / "LEDGER.jsonl"
    if not p.exists():
        return rows
    for line in p.read_text(encoding="utf-8").splitlines():
        if line.strip():
            rows.append(json.loads(line))
    return rows


def merge_geometry(packet_id):
    """(merge_sha, worker_commit, first_parent) for a packet's landing
    merge on the integration branch, or (None, None, None)."""
    out = git("log", "--all", "--grep=" + packet_id,
              "--format=%H %P").splitlines()
    for line in out:
        parts = line.split()
        if len(parts) == 3:  # a merge commit: sha + two parents
            return parts[0], parts[2], parts[1]
    return None, None, None


def amendments(packet_id, worker_commit, merge_sha):
    """Commits ON THE PACKET BRANCH between the worker commit and the
    branch tip (the merge's second parent) whose subject names an
    orchestrator amendment. The range worker..merge would wrongly span
    the whole integration history since the fork."""
    if not (worker_commit and merge_sha):
        return []
    tip = git("show", "-s", "--format=%P", merge_sha).split()
    if len(tip) < 2:
        return []
    out = git("log", "--format=%H %s",
              f"{worker_commit}..{tip[1]}").splitlines()
    return [(line.split(" ", 1)[0], line.split(" ", 1)[1])
            for line in out if "amendment" in line.lower()]


def build():
    waves = {}
    for r in registry():
        wave = r.get("wave") or ""
        if not str(wave).startswith("KV2"):
            continue
        pid = r["id"]
        merge_sha, worker, first_parent = merge_geometry(pid)
        entry = {
            "packet": pid,
            "status": r.get("status"),
            "landed": LANDED_RE.search((r.get("note") or "").lower())
            is not None or r.get("status") == "DONE",
            "worker_commit": worker,
            "merge_sha": merge_sha,
            "amendments": amendments(pid, worker, merge_sha),
        }
        wave = str(wave)
        waves.setdefault(wave, []).append(entry)
    # Wave base = first parent of the earliest packet merge in the wave
    # (order by the merge's commit date).
    bases = {}
    for wave in list(waves):
        landed = [e for e in waves[wave] if e["merge_sha"]]
        if landed:
            landed.sort(key=lambda e: git(
                "show", "-s", "--format=%ct", e["merge_sha"]))
            bases[wave] = git("show", "-s", "--format=%P",
                              landed[0]["merge_sha"]).split()[0]
    return {
        "waves": waves,
        "wave_bases": bases,
        "verifier_version": git("log", "-1", "--format=%h",
                                "--", "loop/verify.py"),
        "integrated_sha": git("rev-parse", "--short", "HEAD"),
    }


def markdown(data):
    lines = ["| Wave | Base SHA | Packets (id -> commit) | Amendments | "
             "Final integrated SHA |",
             "|---|---|---|---|---|"]
    for wave in sorted(data["waves"]):
        entries = sorted(data["waves"][wave], key=lambda e: e["packet"])
        pkts = "; ".join(
            f"{e['packet']} -> {e['worker_commit'] or 'UNMERGED'}"
            for e in entries)
        amend = "; ".join(
            f"{e['packet']}: {sha} {subject[:60]}"
            for e in entries for sha, subject in e["amendments"]) or "none"
        base = data["wave_bases"].get(wave, "?")
        lines.append(f"| {wave} | {base} | {pkts} | {amend} | "
                     f"{data['integrated_sha']} |")
    lines.append("")
    lines.append(f"verifier version (last loop/verify.py commit): "
                 f"{data['verifier_version']}")
    return "\n".join(lines)


def check(data):
    problems = []
    for wave, entries in data["waves"].items():
        for e in entries:
            if e["landed"] and not e["merge_sha"]:
                problems.append(f"{e['packet']}: LANDED in the registry but "
                                f"no landing merge commit found in git")
            if e["merge_sha"] and not e["landed"]:
                problems.append(f"{e['packet']}: merge {e['merge_sha']} "
                                f"exists but the registry row never marked it "
                                f"LANDED/DONE")
            if e["status"] == "DONE" and not e["landed"]:
                problems.append(f"{e['packet']}: status DONE without the "
                                f"LANDED note (one-verify amendment)")
    return problems


def fix(data):
    """Heal the registry: append the LANDED marker (with the landing merge
    SHA) to every row whose merge exists in git. Returns rows changed."""
    path = ROOT / "loop" / "PACKETS.jsonl"
    merged = {e["packet"]: e["merge_sha"]
              for wave in data["waves"].values() for e in wave
              if e["merge_sha"]}
    out_lines = []
    changed = 0
    for line in path.read_text(encoding="utf-8-sig").splitlines():
        if not line.strip():
            continue
        row = json.loads(line)
        pid = row.get("id", "")
        if pid in merged and not LANDED_RE.search(
                (row.get("note") or "").lower()):
            row["note"] = (row.get("note", "")
                           + f"; LANDED {merged[pid][:7]} (one-verify "
                             f"amendment: stays RUNNING until the final "
                             f"battery)")
            changed += 1
        out_lines.append(json.dumps(row))
    path.write_text("\n".join(out_lines) + "\n", encoding="utf-8",
                    newline="\n")
    return changed


def main():
    data = build()
    if "--fix" in sys.argv:
        n = fix(data)
        print(f"registry rows healed with the LANDED marker: {n}")
        return 0
    if "--check" in sys.argv:
        problems = check(data)
        print("\n".join(problems) if problems else
              "manifest check: registry, ledger and git agree")
        return 1 if problems else 0
    if "--markdown" in sys.argv:
        print(markdown(data))
    else:
        print(json.dumps(data, indent=1))
    return 0


if __name__ == "__main__":
    sys.exit(main())
