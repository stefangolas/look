"""One short call that says what every slot is doing. The orchestrator polls
this instead of waiting on a worker: a packet runs for tens of minutes, and
any process that waits that long is itself something that can be killed --
when one was, it took its worker down mid-run.

Liveness is the growth of the event log, not CPU time: a worker waiting on
the model and a worker whose API call will never return look identical from
the outside. --stall-minutes without a byte means hung.

Usage: python loop/slot_status.py [--stall-minutes 12] [--kill-stalled]
"""
import argparse
import ctypes
import datetime
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent

PROCESS_QUERY_LIMITED_INFORMATION = 0x1000


def process_alive(pid):
    """True if a process with this pid exists. OpenProcess rather than
    shelling out to tasklist -- one syscall instead of a subprocess and a
    text parse for a yes/no question."""
    handle = ctypes.windll.kernel32.OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, False, pid)
    if not handle:
        return False
    ctypes.windll.kernel32.CloseHandle(handle)
    return True


def git_status_count(wt):
    res = subprocess.run(['git', '-C', str(wt), 'status', '--porcelain'], capture_output=True, text=True)
    return len([l for l in res.stdout.splitlines() if l != ''])


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--stall-minutes', type=int, default=12)
    ap.add_argument('--kill-stalled', action='store_true')
    args = ap.parse_args()

    slots_dir = REPO_ROOT / 'loop' / 'slots'

    if not slots_dir.is_dir():
        print("no slots yet")
        sys.exit(0)

    for slot in sorted((p for p in slots_dir.iterdir() if p.is_dir()), key=lambda p: p.name):
        events_log = slot / 'events.jsonl'
        pid_file = slot / 'worker.pid'
        pkt_file = slot / 'worker.packet'
        wt = slot / 'wt'

        packet = pkt_file.read_text(encoding='ascii').strip() if pkt_file.is_file() else '-'

        worker_pid = None
        if pid_file.is_file():
            worker_pid = int(pid_file.read_text(encoding='ascii').strip())
        alive = process_alive(worker_pid) if worker_pid else False

        size = 0
        age_min = None
        if events_log.is_file():
            st = events_log.stat()
            size = st.st_size
            age_min = round((datetime.datetime.now().timestamp() - st.st_mtime) / 60, 1)

        if (wt / 'RESULT.json').is_file():
            result = 'RESULT.json'
        elif (wt / 'QUESTION.md').is_file():
            result = 'QUESTION.md'
        else:
            result = '-'

        dirty = git_status_count(wt) if wt.is_dir() else 0

        if alive and age_min is not None and age_min > args.stall_minutes:
            state = 'STALLED'
        elif alive:
            state = 'RUNNING'
        elif result != '-':
            state = 'FINISHED'
        else:
            state = 'IDLE'

        pid_col = worker_pid if alive else '-'
        age_col = age_min if age_min is not None else '-'
        print("slot {:<3} {:<9} packet={:<28} pid={:<7} events={:>8} bytes, {} min old  changed={}  {}".format(
            slot.name, state, Path(packet).name, pid_col, size, age_col, dirty, result))

        if state == 'STALLED' and args.kill_stalled:
            subprocess.run(['taskkill', '/PID', str(worker_pid), '/F'], capture_output=True, text=True)
            print(f"  killed pid {worker_pid} after {age_min} min of silence")


if __name__ == '__main__':
    main()
