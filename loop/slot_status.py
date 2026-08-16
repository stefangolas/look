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
    res = subprocess.run(['git', '-C', str(wt), 'status', '--porcelain'], capture_output=True, text=True, encoding='utf-8', errors='replace')
    return len([l for l in res.stdout.splitlines() if l != ''])


def git_branch_and_head(wt):
    """Branch + short HEAD, and whether HEAD is still sitting at the slot's
    fork point -- i.e. nothing has been committed in it yet. Landing
    BG-S0-002 needed exactly this and didn't have it: `packet/BG-S0-002`
    pointed at the base commit while the accepted work was on a
    differently-named attempt branch, and that had to be reconstructed from
    prose in STATE.md instead of read off the slot.

    The fork point isn't recorded anywhere on disk -- new_slot.py forks from
    whatever the orchestrator's branch happened to be at creation time and
    never writes that down -- so "no work" can't just be
    `merge-base(HEAD, integration/kernel-bg) == HEAD`, tempting as that looks
    (it's the same computation verify.py's own --base default makes). That
    equality is *also* true of any already-merged, real-work branch: once
    `--no-ff` lands a branch, its tip stays an ancestor of kernel-bg forever,
    so a live packet that never diverged and a landed packet from three
    sessions ago read identically by that test. Landing BG-S0-002 and
    BG-S0-003 are exactly the slots this would misreport on.

    The distinguishing fact is *how* HEAD is reachable from kernel-bg, not
    *whether* it is: `--no-ff` (which is what the orchestrator always uses --
    see STATE.md step 2) makes the branch tip the merge commit's *second*
    parent, so it never appears on kernel-bg's first-parent chain. A commit
    that sits directly on that chain got there without ever diverging -- the
    fork-point commit itself is the only way for HEAD to land there while
    still being "this slot's own tip".
    """
    def out(*args):
        res = subprocess.run(['git', '-C', str(wt), *args], capture_output=True, text=True, encoding='utf-8', errors='replace')
        return res.stdout.strip()

    branch = out('rev-parse', '--abbrev-ref', 'HEAD') or '?'
    head_short = out('rev-parse', '--short', 'HEAD') or '?'
    head_full = out('rev-parse', 'HEAD')

    is_ancestor = subprocess.run(
        ['git', '-C', str(wt), 'merge-base', '--is-ancestor', 'HEAD', 'integration/kernel-bg'],
        capture_output=True, text=True, encoding='utf-8', errors='replace').returncode == 0

    if is_ancestor:
        # Reachable from kernel-bg somehow -- check whether it's on the
        # direct (first-parent) line, which is where a never-diverged fork
        # point sits and a --no-ff-merged branch tip never does.
        first_parent_line = set(out('rev-list', '--first-parent', 'integration/kernel-bg').splitlines())
        no_work = head_full in first_parent_line
    else:
        # Not merged yet -- HEAD == merge-base means nothing has been
        # committed since the fork.
        fork_base = out('merge-base', 'HEAD', 'integration/kernel-bg')
        no_work = bool(fork_base) and fork_base == head_full

    return branch, head_short, no_work


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
        branch, head, no_work = git_branch_and_head(wt) if wt.is_dir() else ('-', '-', False)

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
        git_col = f"{branch}@{head}" + (' (=base, no work)' if no_work else '')
        print("slot {:<3} {:<9} packet={:<28} pid={:<7} events={:>8} bytes, {} min old  changed={}  {}  git={}".format(
            slot.name, state, Path(packet).name, pid_col, size, age_col, dirty, result, git_col))

        if state == 'STALLED' and args.kill_stalled:
            subprocess.run(['taskkill', '/PID', str(worker_pid), '/F'], capture_output=True, text=True, encoding='utf-8', errors='replace')
            print(f"  killed pid {worker_pid} after {age_min} min of silence")


if __name__ == '__main__':
    main()
