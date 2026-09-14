#!/usr/bin/env python3
"""Supervisor-side write-set enforcement and transactional landing.

LOOP-MACH-1 items 3 and 4. A worker edits its slot's worktree and publishes
its claim to `slots/<N>/outbox/RESULT.json`; it never commits, and it never
decides what belongs on the integration branch. The supervisor does, and this
module is where that decision is mechanical:

  * `diff_paths(base, tip)` is every path the attempt changed -- both sides of
    a rename included -- computed against the base_sha frozen at fork.
  * `check(...)` is the gate: that set must be a subset of the packet's
    `write_allow`. Anything else is `WRITESET_VIOLATION`, and the attempt is
    rejected before a single ref moves. This is the class `git add -A`
    dissolved: undeclared paths are excluded rather than swept, and a worker
    that forgot to commit cannot lose its work because the supervisor commits
    the declared paths as-delivered.
  * `transactional_land(...)` performs the merge in a detached temp worktree,
    runs the merged-HEAD scoped gates there, and only then advances the
    integration ref with ONE atomic `git update-ref`. A crash or a failed
    gate anywhere before that call leaves the integration ref untouched; the
    temp worktree is torn down on every exit path.

Usage:
  python loop/check_writeset.py --base <ref> --tip <ref> PACKET
  python loop/check_writeset.py --base <ref> --tip <ref> --allow a b c
"""

import argparse
import fnmatch
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
INTEGRATION_REF = "integration/kernel-bg"


def git(repo, *args, check=False, env=None):
    res = subprocess.run(
        ["git", "-C", str(repo), *args],
        capture_output=True, text=True, encoding="utf-8", errors="replace",
        env=env)
    if check and res.returncode != 0:
        raise RuntimeError(
            f"git {' '.join(args)} failed in {repo}: {res.stderr.strip()}")
    return res


def _front_block(text):
    """The packet's front block, in either of the two formats the repo uses."""
    m = re.search(r"```yaml\n(.*?)```", text, re.S)
    if m:
        return m.group(1)
    m = re.match(r"\A---\s*\n(.*?)\n---\s*\n", text, re.S)
    return m.group(1) if m else ""


def _yaml_list(yaml_text, field):
    m = re.search(rf"(?m)^{field}:\s*\[([^\]]*)\]", yaml_text)
    if m:
        return [x.strip().strip("\"'") for x in m.group(1).split(",") if x.strip()]
    m = re.search(rf"(?m)^{field}:\s*\n((?:\s+-\s+\S+\n?)+)", yaml_text)
    if m:
        return re.findall(r"-\s+(\S+)", m.group(1))
    return []


def parse_write_allow(packet_path):
    text = Path(packet_path).read_text(encoding="utf-8", errors="replace")
    return _yaml_list(_front_block(text), "write_allow")


def diff_paths(repo, base, tip):
    """Every path changed between `base` and `tip`, both sides of renames.

    Two-dot, not three-dot: `base` is the base_sha frozen at fork, so this is
    the attempt's own diff even after the integration branch has moved. A
    three-dot diff would silently re-base onto the merge-base and miss (or
    invent) changes.
    """
    res = git(repo, "diff", "--name-status", "-M", base, tip)
    if res.returncode != 0:
        raise RuntimeError(
            f"git diff {base} {tip} failed in {repo}: {res.stderr.strip()}")
    paths = []
    for line in res.stdout.splitlines():
        if not line.strip():
            continue
        parts = line.split("\t")
        status = parts[0]
        if status[:1] in ("R", "C"):
            paths.extend(parts[1:3])
        else:
            paths.extend(parts[1:2])
    return sorted({p for p in paths if p})


def _match_allow(entry, path):
    entry = entry.strip()
    if not entry:
        return False
    if entry == path:
        return True
    if entry.endswith("/**"):
        root = entry[:-3]
        return path == root or path.startswith(root + "/")
    if entry.endswith("/"):
        return path.startswith(entry)
    if fnmatch.fnmatch(path, entry):
        return True
    # An allow entry naming a directory covers everything beneath it.
    return path.startswith(entry.rstrip("/") + "/")


def path_covered(path, write_allow):
    """Is `path` covered by any declared write set entry?

    Entries may be repo-relative (`loop/dispatch_ready.py`) or crate-relative
    (`truck-shapeops/src/fillet/**`, the shorthand the registry rows use for
    `vendor/truck/...`); both are accepted so the packet and the registry can
    spell the same fact differently without one of them reading as a
    violation.
    """
    candidates = {path}
    prefix = "vendor/truck/"
    if path.startswith(prefix):
        candidates.add(path[len(prefix):])
    for entry in write_allow:
        for cand in candidates:
            if _match_allow(entry, cand):
                return True
    return False


def check(repo, base, tip, write_allow):
    """Returns (ok, violations): the changed paths not covered by the allow."""
    changed = diff_paths(repo, base, tip)
    violations = [p for p in changed if not path_covered(p, write_allow)]
    return (not violations, violations)


def check_report(repo, base, tip, write_allow):
    """The check plus the provenance a later reader needs: the exact commits
    the anchors were measured against (`checked_at` is the tip)."""
    base_sha = git(repo, "rev-parse", base).stdout.strip()
    tip_sha = git(repo, "rev-parse", tip).stdout.strip()
    changed = diff_paths(repo, base_sha, tip_sha)
    violations = [p for p in changed if not path_covered(p, write_allow)]
    return {"ok": not violations, "base": base_sha, "tip": tip_sha,
            "checked_at": tip_sha, "changed": changed,
            "violations": violations}


def _ref(ref):
    return ref if ref.startswith("refs/") else f"refs/heads/{ref}"


def transactional_land(repo, packet_path, branch, base,
                       integration_ref=INTEGRATION_REF, gate=None,
                       message=None, keep_worktree=False):
    """Land `branch` onto `integration_ref`, or leave the ref untouched.

    Steps, in order: (1) the write-set gate; (2) a detached temp worktree at
    the integration tip; (3) `merge --no-ff` there; (4) the merged-HEAD scoped
    gates (the `gate` callable, `gate(merged_sha, worktree_path) -> bool`);
    (5) ONE atomic `update-ref` with the old tip as the compare-and-swap
    guard. The metadata commit (results, registry, ledger) is the caller's
    business and follows a successful landing.

    Returns a dict with a `status` of `WRITESET_VIOLATION`, `MERGE_FAILED`,
    `GATE_FAILED`, `UPDATE_REF_FAILED`, or `LANDED`.
    """
    repo = Path(repo)
    tip = git(repo, "rev-parse", branch).stdout.strip()
    base_sha = git(repo, "rev-parse", base).stdout.strip()
    if not tip or not base_sha:
        return {"status": "MERGE_FAILED",
                "stderr": f"cannot resolve {branch!r} or {base!r}"}

    write_allow = parse_write_allow(packet_path)
    report = check_report(repo, base_sha, tip, write_allow)
    if not report["ok"]:
        return {"status": "WRITESET_VIOLATION",
                "violations": report["violations"], "base": base_sha,
                "tip": tip, "checked_at": tip}

    old = git(repo, "rev-parse", integration_ref).stdout.strip()
    if not old:
        return {"status": "UPDATE_REF_FAILED",
                "stderr": f"integration ref {integration_ref!r} does not exist"}

    tmp_parent = tempfile.mkdtemp(prefix="look-land-")
    wt = Path(tmp_parent) / "wt"
    try:
        add = git(repo, "worktree", "add", "--detach", str(wt), integration_ref)
        if add.returncode != 0:
            return {"status": "MERGE_FAILED", "stderr": add.stderr.strip(),
                    "base": base_sha, "tip": tip}

        merge_msg = message or f"merge: {branch} onto {integration_ref}"
        env = dict(os.environ)
        env.setdefault("GIT_AUTHOR_NAME", "look-loop")
        env.setdefault("GIT_AUTHOR_EMAIL", "loop@localhost")
        env.setdefault("GIT_COMMITTER_NAME", "look-loop")
        env.setdefault("GIT_COMMITTER_EMAIL", "loop@localhost")
        merge = subprocess.run(
            ["git", "-C", str(wt), "merge", "--no-ff", branch, "-m", merge_msg],
            capture_output=True, text=True, encoding="utf-8", errors="replace",
            env=env)
        if merge.returncode != 0:
            return {"status": "MERGE_FAILED", "stderr": merge.stderr.strip(),
                    "stdout": merge.stdout, "base": base_sha, "tip": tip}

        merged = git(wt, "rev-parse", "HEAD").stdout.strip()

        if gate is not None:
            if not gate(merged, wt):
                return {"status": "GATE_FAILED", "merged_sha": merged,
                        "base": base_sha, "tip": tip}

        upd = git(repo, "update-ref", _ref(integration_ref), merged, old)
        if upd.returncode != 0:
            return {"status": "UPDATE_REF_FAILED", "stderr": upd.stderr.strip(),
                    "merged_sha": merged}
        return {"status": "LANDED", "merged_sha": merged, "old_sha": old,
                "base": base_sha, "tip": tip, "checked_at": tip}
    finally:
        if not keep_worktree:
            git(repo, "worktree", "remove", "--force", str(wt))
            shutil.rmtree(tmp_parent, ignore_errors=True)


def record_metadata(repo, packet_id, attempt_id, result, merged_sha=None,
                    ledger_path=None, results_root=None):
    """File the accepted attempt's evidence and ledger row (item 2).

    Results land at `loop/results/<packet_id>/<NNNN>.json` and the ledger row
    carries `attempt_id`, so a later re-booking can find the exact attempt
    that was accepted and the one it superseded. The write is immutable: an
    existing attempt file is never overwritten.
    """
    repo = Path(repo)
    no = int(str(attempt_id).rsplit("/", 1)[-1])
    results_root = Path(results_root) if results_root else repo / "loop" / "results"
    dest = results_root / packet_id / f"{no:04d}.json"
    if dest.exists():
        raise FileExistsError(
            f"attempt evidence is immutable: {dest} already exists")
    dest.parent.mkdir(parents=True, exist_ok=True)
    dest.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n",
                    encoding="utf-8")

    ledger = Path(ledger_path) if ledger_path else repo / "loop" / "LEDGER.jsonl"
    row = {"id": packet_id, "attempt_id": attempt_id,
           "execution_status": result.get("execution_status"),
           "outcome": result.get("outcome"),
           "findings": result.get("findings", [])}
    if merged_sha:
        row["merged_sha"] = merged_sha
    with ledger.open("a", encoding="utf-8", newline="\n") as f:
        f.write(json.dumps(row) + "\n")
    return dest


def annotate_registry(repo, packet_id, attempt_id, merged_sha=None,
                      status="DONE", packet_path=None, note=None):
    """Append a registry row naming the accepted attempt.

    Appends; it never rewrites PACKETS.jsonl history. Readers take the last
    row for an id, and the `landed <sha>` note is the dispatch marker the
    existing loop already understands.
    """
    repo = Path(repo)
    path = repo / "loop" / "PACKETS.jsonl"
    base = None
    lines = path.read_text(encoding="utf-8").splitlines() if path.is_file() else []
    for line in lines:
        if line.strip() and json.loads(line).get("id") == packet_id:
            base = json.loads(line)
    row = dict(base) if base else {"id": packet_id}
    row["status"] = status
    if packet_path:
        row["packet"] = packet_path
    row["attempt_id"] = attempt_id
    if merged_sha:
        row["note"] = (note or f"landed {merged_sha}") + f" (attempt {attempt_id})"
    elif note:
        row["note"] = note
    with path.open("a", encoding="utf-8", newline="\n") as f:
        f.write(json.dumps(row) + "\n")
    return row


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--repo", default=str(REPO_ROOT))
    ap.add_argument("--base", required=True)
    ap.add_argument("--tip", default="HEAD")
    ap.add_argument("--allow", nargs="+")
    ap.add_argument("packet", nargs="?")
    args = ap.parse_args(argv)

    allow = args.allow
    if allow is None:
        if not args.packet:
            ap.error("either a PACKET or --allow is required")
        allow = parse_write_allow(args.packet)

    report = check_report(args.repo, args.base, args.tip, allow)
    print(f"checked_at: {report['checked_at']}")
    if report["ok"]:
        print(f"writeset ok: {len(report['changed'])} changed path(s) all "
              f"within write_allow")
        return 0
    print(f"WRITESET_VIOLATION: {len(report['violations'])} undeclared path(s):")
    for p in report["violations"]:
        print(f"  {p}")
    return 1


if __name__ == "__main__":
    sys.exit(main())
