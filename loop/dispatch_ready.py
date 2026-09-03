"""dispatch_ready: the mechanical rolling dispatcher (session 50).

One command replaces the manual new_slot/run_packet dances:
  1. READY packets whose deps are LANDED and whose write set is disjoint
     from every RUNNING row.
  2. Free slots (IDLE/FINISHED adjudicated, or a missing slot dir).
  3. gen_packet --check (anchor reality) + packet_lint before dispatch.
  4. new_slot + run_packet with the cargoq shim first in PATH and the
     CARGO_BUILD_JOBS cap.

NOT automated (judgment stays with the orchestrator): adjudicating
RESULTs, merging, amending. This script only fills the machine.

Usage:
  python loop/dispatch_ready.py --dry-run
  python loop/dispatch_ready.py            # dispatch for real
  python loop/dispatch_ready.py --max-workers 6
"""
import json
import os
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
QUEUE = os.path.join(ROOT, "loop", "cargoq")


def sh(args, **kw):
    return subprocess.run(args, capture_output=True, text=True, **kw)


def rows():
    out = []
    with open(os.path.join(ROOT, "loop", "PACKETS.jsonl"),
              encoding="utf-8-sig") as f:
        for line in f:
            line = line.strip()
            if line:
                out.append(json.loads(line))
    return out


def landed(r):
    s = (r.get("status") or "").lower()
    note = (r.get("note") or "").lower()
    return s == "done" or (s == "running" and "landed" in note)


def slots():
    d = os.path.join(ROOT, "loop", "slots")
    res = {}
    if os.path.isdir(d):
        for name in sorted(os.listdir(d), key=lambda x: int(x) if
                           x.isdigit() else 999):
            st = sh([sys.executable, os.path.join(ROOT, "loop",
                    "slot_status.py")], )
            break  # slot_status prints all; parse below instead
    return res


def slot_states():
    out = sh([sys.executable, os.path.join(ROOT, "loop", "slot_status.py")])
    states = {}
    assigned = set()
    dead = set()   # FINISHED with no RESULT.json in the wt = dead dispatch
    slot_of = {}   # packet id -> slot number
    for line in out.stdout.splitlines():
        parts = line.split()
        if len(parts) >= 2 and parts[0] == "slot" and parts[1].isdigit():
            states[parts[1]] = parts[2]
            pkt = None
            if len(parts) >= 4 and parts[3].startswith("packet="):
                pkt = parts[3][len("packet="):].removesuffix(".md")
            if pkt:
                slot_of[pkt] = parts[1]
                res = os.path.join(ROOT, "loop", "slots", parts[1],
                                   "wt", "RESULT.json")
                res_id = None
                if os.path.isfile(res):
                    try:
                        with open(res, encoding="utf-8-sig") as f:
                            rj = json.load(f)
                        res_id = (rj.get("packet") or rj.get("id") or "")
                    except Exception:
                        res_id = "?unreadable"
                if parts[2] == "RUNNING" or res_id == pkt:
                    assigned.add(pkt)
                elif res_id is None:
                    dead.add(pkt)  # no RESULT: the dispatch died
                else:
                    assigned.add(pkt)  # a STALE RESULT from another packet:
                    # the slot needs manual cleanup, not an auto-dispatch
    return states, assigned, dead, slot_of


def main():
    dry = "--dry-run" in sys.argv
    max_workers = 6
    for a in sys.argv:
        if a.startswith("--max-workers="):
            max_workers = int(a.split("=")[1])
    rs = rows()
    by_id = {r["id"]: r for r in rs}
    running = [r for r in rs if r.get("status") == "RUNNING"
               and not landed(r)]
    running_writes = {w for r in running for w in r.get("writes", [])}
    states, assigned, dead, slot_of = slot_states()
    free = [s for s, st in states.items() if st in ("IDLE", "FINISHED")]
    next_slot = max((int(s) for s in states), default=-1) + 1
    busy = sum(1 for st in states.values() if st == "RUNNING")
    print(f"slots: {len(states)} ({busy} running, {len(free)} free); "
          f"slot-assigned packets: {len(assigned)}")
    acted = 0
    for r in rs:
        if acted + busy >= max_workers or not free and acted == 0:
            if acted + busy >= max_workers:
                break
        if r.get("status") != "READY":
            continue
        if r["id"] in assigned:
            continue  # already in a slot (ground truth beats row status)
        if r["id"] in dead:
            # A dead dispatch holds nothing (no RESULT, no code): reset is
            # safe and the re-dispatch proceeds through the normal path.
            s = slot_of.get(r["id"])
            if s:
                sh([sys.executable, os.path.join(ROOT, "loop",
                    "run_packet.py"), "--slot", s, "--reset-only",
                    "--packet", os.path.join(ROOT, "loop", "packets",
                    r["id"] + ".md")])
                free.append(s)
        needs = r.get("needs", [])
        unmet = [n for n in needs
                 if n not in by_id or not landed(by_id[n])]
        if unmet:
            print(f"  {r['id']}: blocked on {unmet}")
            continue
        writes = set(r.get("writes", []))
        clash = writes & running_writes
        if clash:
            print(f"  {r['id']}: write-set clash with a RUNNING row: "
                  f"{sorted(clash)[:2]}")
            continue
        pk = os.path.join(ROOT, "loop", "packets", r["id"] + ".md")
        chk = sh([sys.executable, os.path.join(ROOT, "loop",
                  "gen_packet.py"), "--check", pk])
        if chk.returncode != 0:
            print(f"  {r['id']}: ANCHOR CHECK FAILED - fix before "
                  f"dispatch:\n{chk.stdout[-400:]}")
            continue
        lint = sh([sys.executable, os.path.join(ROOT, "loop",
                   "packet_lint.py"), pk])
        if "FAIL" in lint.stdout:
            print(f"  {r['id']}: LINT FAIL:\n{lint.stdout[-300:]}")
            continue
        slot = free.pop(0) if free else str(next_slot)
        next_slot = max(next_slot, int(slot) + 1)
        branch = "packet/" + r["id"]
        print(f"  {r['id']} -> slot {slot} ({branch})")
        if dry:
            acted += 1
            continue
        sh([sys.executable, os.path.join(ROOT, "loop", "run_packet.py"),
            "--slot", slot, "--reset-only", "--packet", pk])
        ns = sh([sys.executable, os.path.join(ROOT, "loop", "new_slot.py"),
                 "--slot", slot, "--branch", branch])
        if ns.returncode != 0:
            print(f"  {r['id']}: new_slot FAILED:\n{ns.stderr[-300:]}")
            continue
        env_path = QUEUE + os.pathsep + os.environ.get("PATH", "")
        rp = subprocess.run(
            [sys.executable, os.path.join(ROOT, "loop", "run_packet.py"),
             "--slot", slot, "--packet", pk],
            env={**os.environ, "PATH": env_path,
                 "CARGO_BUILD_JOBS": "2"})
        if rp.returncode == 0:
            # register the dispatch in PACKETS.jsonl
            r["status"] = "RUNNING"
            acted += 1
            running_writes |= writes
        else:
            print(f"  {r['id']}: run_packet FAILED (see above)")
    print(f"dispatched {acted}; workers now ~{busy + acted}/{max_workers}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
