#!/usr/bin/env python3
"""Autonomous slot watchdog for the BG- autobuild loop.

Owns the WAITING so the orchestrator does not have to sleep in tool calls.
Polls every POLL_SECONDS; on confirmed worker death or wedge, kills the shim
tree, archives the abandoned diff and redispatches the same packet, up to
MAX_RESTARTS per (slot, packet). Also guards disk by reclaiming idle slots'
target directories below DISK_SOFT and every slot's inner wt/target below
DISK_HARD.

The death rules are calibrated against the recorded history in STATE.md:
- A healthy worker has shown a 12.9-minute silent thinking gap that then
  resumed and finished normally. STAGNANT_SECONDS (20 min) sits above that
  with margin; the observed losses to silent death were 36-87 minutes.
- Rule A (wedge): the recorded pid is alive but events.jsonl has not grown
  for STAGNANT_SECONDS -> kill the tree, archive, redispatch.
- Rule B (hard death): the pid is gone, no RESULT.json in the worktree, and
  events have been stagnant for STAGNANT_SECONDS -> redispatch (the BLOCKED
  path, automated).
- A finished worker (RESULT.json present) is NEVER touched: adjudication is
  the orchestrator's, not this script's.
- Likewise this script never reaps on the STALLED label alone and never
  touches a slot whose events are still growing.

Stdlib only (house rule for loop/*.py). Single instance via watchdog.lock.
All actions append to loop/watchdog.log; state persists in
loop/watchdog-state.json so a restarted watchdog keeps its restart budget.
"""

import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path

sys.stdout.reconfigure(encoding="utf-8")

ROOT = Path(__file__).resolve().parent.parent
SLOTS = ROOT / "loop" / "slots"
LOG = ROOT / "loop" / "watchdog.log"
STATE = ROOT / "loop" / "watchdog-state.json"
LOCK = ROOT / "loop" / "watchdog.lock"

POLL_SECONDS = 60
STAGNANT_SECONDS = int(os.environ.get("LOOK_WATCHDOG_STAGNANT", 20 * 60))
HEARTBEAT_EVERY = 5  # polls
MAX_RESTARTS = 3
DISK_SOFT_GB = 9.0
DISK_HARD_GB = 7.0


def log(msg):
    stamp = time.strftime("%Y-%m-%dT%H:%M:%S")
    with open(LOG, "a", encoding="utf-8") as f:
        f.write(f"{stamp} {msg}\n")


def pid_alive(pid):
    if not pid:
        return False
    try:
        out = subprocess.run(
            ["tasklist", "/FI", f"PID eq {pid}"],
            capture_output=True, text=True, timeout=30,
        ).stdout
    except Exception as exc:  # noqa: BLE001 - a failed probe is not a verdict
        log(f"WARNING tasklist probe failed for pid {pid}: {exc}")
        return True  # never act on a failed probe
    return str(pid) in out


def kill_tree(pid):
    try:
        subprocess.run(
            ["taskkill", "/PID", str(pid), "/T", "/F"],
            capture_output=True, text=True, timeout=60,
        )
        return True
    except Exception as exc:  # noqa: BLE001
        log(f"WARNING taskkill failed for pid {pid}: {exc}")
        return False


def packet_is_done(packet_path):
    """True if this packet's row in PACKETS.jsonl already reads DONE.

    A landed slot looks exactly like a dead worker: `land_packet.py` moves
    RESULT.json out of the worktree into loop/results/, the worker's pid is
    long gone, and events.jsonl stopped growing when the worker finished. Rule
    B then fires and redispatches a packet that is already merged. That
    happened at 01:34 on 2026-08-20 to BG-ENC-002-LINE, minutes after it
    landed: the redispatched worker rebuilt the slot, took a lock on
    events.jsonl and blocked the next real dispatch.

    The registry is the authority on what is finished, so ask it.
    """
    if not packet_path:
        return False
    stem = Path(packet_path).stem
    registry = ROOT / "loop" / "PACKETS.jsonl"
    try:
        for line in registry.read_text(encoding="utf-8").splitlines():
            if not line.strip():
                continue
            row = json.loads(line)
            if row.get("id") == stem:
                if row.get("status") == "DONE":
                    return True
                # One-verify amendment convention: rows stay RUNNING with a
                # "LANDED <sha>" note until the final battery flips them.
                # Session-51 bug: a LANDED packet was re-dispatched because
                # only the DONE status counted and its worker looked dead.
                if re.search(r"landed [0-9a-f]{7,}",
                             (row.get("note") or "").lower()):
                    return True
                return False
    except (OSError, ValueError):
        return False
    return False


def read_slot(slot_dir):
    """Return the observable state of one slot, or None if not a live slot."""
    pid_file = slot_dir / "worker.pid"
    packet_file = slot_dir / "worker.packet"
    events = slot_dir / "events.jsonl"
    if not pid_file.exists():
        return None
    pid = None
    try:
        pid = int(pid_file.read_text().strip() or 0)
    except ValueError:
        pid = None
    packet = None
    if packet_file.exists():
        packet = packet_file.read_text().strip()
    size, mtime = 0, 0.0
    if events.exists():
        st = events.stat()
        size, mtime = st.st_size, st.st_mtime
    result_json = (slot_dir / "wt" / "RESULT.json").exists()
    return {
        "pid": pid, "packet": packet, "size": size,
        "mtime": mtime, "result": result_json,
    }


def redispatch(slot_dir, slot_no, reason):
    state = load_state()
    key = f"slot{slot_no}"
    packet = (slot_dir / "worker.packet").read_text().strip()
    budget = state.setdefault("restarts", {}).setdefault(key, {})
    count = budget.get(packet, 0)
    if count >= MAX_RESTARTS:
        log(f"CRITICAL slot {slot_no}: restart budget exhausted for {packet} "
            f"({count}); leaving it for the orchestrator")
        return
    # Session-51 self-diagnosis (owner directive): from the second restart
    # of the same packet on, autopsy the dead sessions first. Some killers
    # (RAM pressure, provider balance) kill the next run too - redispatch
    # would only burn the budget. RAM_PRESSURE defers; balance stops.
    strategy = "fresh"
    if count >= 1:
        try:
            out = subprocess.run(
                [sys.executable, str(ROOT / "loop" / "autopsy.py"),
                 str(slot_no)],
                capture_output=True, text=True, timeout=120, cwd=str(ROOT))
            verdict = (out.stdout or "").strip().splitlines()
            verdict = verdict[-1] if verdict else "NO_EVIDENCE"
        except Exception as exc:  # noqa: BLE001
            verdict = f"NO_EVIDENCE (autopsy failed: {exc})"
        log(f"ACTION slot {slot_no}: autopsy: {verdict}")
        if verdict.startswith("RAM_PRESSURE"):
            log(f"DEFERRED slot {slot_no}: autopsy says RAM_PRESSURE - not "
                f"spending restart {count + 1}; will retry next poll")
            return
        if "balance" in verdict.lower():
            log(f"CRITICAL slot {slot_no}: autopsy says provider balance - "
                f"not redispatching; orchestrator must reup")
            return
        if count == 1 and "resume-interrupted" in verdict:
            strategy = "resume"
    log(f"ACTION slot {slot_no}: {reason}; killing and redispatching {packet} "
        f"(restart {count + 1}/{MAX_RESTARTS}, strategy={strategy})")
    pid = None
    try:
        pid = int((slot_dir / "worker.pid").read_text().strip() or 0)
    except ValueError:
        pass
    if pid:
        kill_tree(pid)
        time.sleep(3)
    argv = [sys.executable, str(ROOT / "loop" / "run_packet.py"),
            "--slot", str(slot_no), "--packet", packet]
    if strategy == "resume":
        # Death recovery: continue the dead session in its own worktree -
        # the WIP stays, the context survives, no cold re-read.
        argv.append("--resume-interrupted")
    else:
        argv.append("--reset")
    try:
        out = subprocess.run(argv, capture_output=True, text=True,
                             timeout=300, cwd=str(ROOT))
        tail = (out.stdout or "").strip().splitlines()
        log(f"ACTION slot {slot_no}: redispatch output: "
            + (tail[-1] if tail else "(none)"))
        if out.returncode != 0:
            log(f"WARNING slot {slot_no}: run_packet exited "
                f"{out.returncode}: {(out.stderr or '').strip()[:300]}")
    except Exception as exc:  # noqa: BLE001
        log(f"WARNING slot {slot_no}: redispatch failed: {exc}")
        return
    budget[packet] = count + 1
    save_state(state)


def load_state():
    try:
        return json.loads(STATE.read_text(encoding="utf-8"))
    except Exception:  # noqa: BLE001 - fresh state on any parse failure
        return {}


def save_state(state):
    STATE.write_text(json.dumps(state, indent=1), encoding="utf-8")


def dir_size_gb(path):
    total = 0
    for p in path.rglob("*"):
        try:
            if p.is_file():
                total += p.stat().st_size
        except OSError:
            continue
    return total / (1000 ** 3)


def verify_active():
    """[(slot_dir, pid)] for slots where a verify.py is building right now.

    A slot's `worker.pid` disappears the moment the worker writes RESULT.json,
    but verify then spends many minutes compiling in that same `target/`. On
    2026-08-19 this function did not exist, `guard_disk` read those slots as
    idle, and rmtree'd `loop/slots/0/target` three times under a live cargo.
    The resulting `error[E0786] found invalid metadata files for crate` and
    `failed to write ...dep-lib-truck_meshalgo` were then diagnosed as code
    regressions in the packet under test. Nothing this script reclaims is
    worth that; verify.py writes `verify.pid` for exactly this check.
    """
    live = []
    for slot_dir in sorted(SLOTS.iterdir()):
        if not slot_dir.is_dir():
            continue
        pid_file = slot_dir / "verify.pid"
        try:
            pid = int(pid_file.read_text(encoding="utf-8").strip())
        except (OSError, ValueError):
            continue
        if pid_alive(pid):
            live.append((slot_dir, pid))
    return live


def reclaim_leaked_baselines(free_gb):
    """Delete baseline worktrees a killed verify left in %TEMP%.

    These are pure garbage -- a dead process's throwaway copy of the tree --
    and each is 2-4 GB, so they are the right thing to take first. Deleting a
    slot's warm target instead frees the same disk and then charges the next
    verify a full cold rebuild, which is how the previous session made every
    retry more expensive than the last.
    """
    for p in sorted(Path(tempfile.gettempdir()).glob("look-verify-baseline-*")):
        if not p.is_dir():
            continue
        gb = dir_size_gb(p)
        try:
            shutil.rmtree(p)
            free_gb += gb
            log(f"ACTION disk {free_gb:.1f} GB free: reclaimed {gb:.1f} GB of "
                f"leaked baseline worktree at {p}")
        except OSError as exc:
            log(f"WARNING could not reclaim {p}: {exc}")
    subprocess.run(["git", "-C", str(ROOT), "worktree", "prune"],
                   capture_output=True, text=True)
    return free_gb


def guard_disk(memory):
    try:
        free_gb = shutil.disk_usage(str(ROOT)).free / (1000 ** 3)
    except OSError:
        return
    if free_gb >= DISK_SOFT_GB:
        return

    live = verify_active()
    if live:
        # A verify owns a target dir it did not register a worker pid for.
        # Reclaiming anything under loop/slots while it builds corrupts it.
        # Leaked %TEMP% baselines are not safe either: the live verify's own
        # baseline worktree lives there under the same prefix.
        log(f"WARNING disk {free_gb:.1f} GB free but verify is live in "
            + ", ".join(f"{d.name} (pid {p})" for d, p in live)
            + "; reclaiming nothing -- deleting a target under a running "
              "cargo corrupts it and the orchestrator must decide")
        memory["last_disk_gb"] = free_gb
        return

    free_gb = reclaim_leaked_baselines(free_gb)
    if free_gb >= DISK_SOFT_GB:
        memory["last_disk_gb"] = free_gb
        return

    # Then idle slots: no live pid, or pid dead.
    for slot_dir in sorted(SLOTS.iterdir()):
        if not slot_dir.is_dir():
            continue
        info = read_slot(slot_dir)
        if info and info["pid"] and pid_alive(info["pid"]):
            continue
        for target in (slot_dir / "target", slot_dir / "wt" / "target"):
            if target.exists():
                gb = dir_size_gb(target)
                try:
                    shutil.rmtree(target)
                    log(f"ACTION disk {free_gb:.1f} GB free: reclaimed "
                        f"{gb:.1f} GB at {target}")
                    free_gb += gb
                except OSError as exc:
                    log(f"WARNING could not reclaim {target}: {exc}")
        if free_gb >= DISK_SOFT_GB:
            return
    if free_gb < DISK_HARD_GB:
        for slot_dir in sorted(SLOTS.iterdir()):
            target = slot_dir / "wt" / "target"
            if target.exists():
                gb = dir_size_gb(target)
                log(f"WARNING disk {free_gb:.1f} GB free, below hard floor: "
                    f"would reclaim {gb:.1f} GB at {target} but a live "
                    f"worker may be building there; orchestrator must decide")
    memory["last_disk_gb"] = free_gb


def main():
    # Single instance: a lock holding a pid whose process is alive.
    if LOCK.exists():
        try:
            old = int(LOCK.read_text().strip())
            if pid_alive(old):
                print(f"watchdog already running as pid {old}; exiting")
                return
        except ValueError:
            pass
    LOCK.write_text(str(os.getpid()))
    log(f"START watchdog pid {os.getpid()} stagnant={STAGNANT_SECONDS}s "
        f"poll={POLL_SECONDS}s max_restarts={MAX_RESTARTS}")

    memory = {"seen": {}, "polls": 0}
    try:
        while True:
            for slot_dir in sorted(SLOTS.iterdir()):
                if not slot_dir.is_dir():
                    continue
                try:
                    slot_no = int(slot_dir.name)
                except ValueError:
                    continue
                info = read_slot(slot_dir)
                if info is None or not info["packet"]:
                    continue
                key = f"slot{slot_no}"
                seen = memory["seen"].setdefault(
                    key, {"size": -1, "since": time.time()})
                grew = info["size"] != seen["size"]
                if grew:
                    seen["size"] = info["size"]
                    seen["since"] = time.time()
                    continue
                stagnant = time.time() - seen["since"]
                if stagnant < STAGNANT_SECONDS:
                    continue
                if info["result"]:
                    # Finished run awaiting adjudication: never touch.
                    continue
                if packet_is_done(info["packet"]):
                    # Already landed. The slot only *looks* dead because
                    # land_packet.py took RESULT.json out of the worktree.
                    # Redispatching here re-does merged work and, worse, takes
                    # a lock on events.jsonl that blocks the next real
                    # dispatch. Leave it for new_slot.py to repoint.
                    continue
                alive = pid_alive(info["pid"])
                if alive:
                    redispatch(
                        slot_dir, slot_no,
                        f"wedged (pid {info['pid']} alive, events stagnant "
                        f"{int(stagnant)}s)",
                    )
                elif info["pid"]:
                    redispatch(
                        slot_dir, slot_no,
                        f"hard death (pid {info['pid']} gone, no RESULT.json, "
                        f"events stagnant {int(stagnant)}s)",
                    )
                # pid file empty/zero: dispatch in progress, wait.
                memory["seen"][key]["since"] = time.time()
            memory["polls"] += 1
            if memory["polls"] % HEARTBEAT_EVERY == 0:
                try:
                    free_gb = shutil.disk_usage(str(ROOT)).free / (1000 ** 3)
                    log(f"HEARTBEAT poll {memory['polls']} "
                        f"disk={free_gb:.1f}GB")
                except OSError:
                    log("HEARTBEAT poll (disk probe failed)")
            guard_disk(memory)
            time.sleep(POLL_SECONDS)
    finally:
        try:
            if LOCK.read_text().strip() == str(os.getpid()):
                LOCK.unlink()
        except OSError:
            pass
        log("STOP watchdog")


if __name__ == "__main__":
    main()
