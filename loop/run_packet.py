"""Dispatch one packet to a warm slot's worker (S2: opencode run, deepseek by
default). One packet, one process, one context reset - the architecture's
isolation unit, not agent discipline.

This script only launches the worker and captures its event stream; it does
not judge the result. verify.py is the only acceptance authority (S5).

Usage: python loop/run_packet.py --slot 0 --packet loop/packets/BG-S0-002.md [--model ...] [--reset] [--dry-run]
"""
import argparse
import datetime
import os
import shutil
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent

CREATE_NO_WINDOW = 0x08000000


def git_lines(wt, *args):
    res = subprocess.run(['git', '-C', str(wt), *args], capture_output=True, text=True)
    return [l for l in res.stdout.splitlines() if l != '']


def find_opencode_launcher():
    # `opencode` on this machine is an npm shim (opencode.ps1 / opencode.cmd),
    # not an exe. A .ps1 shim needs a PowerShell host to run at all, and this
    # port exists so the orchestrator does not depend on one being present.
    # The .cmd shim is a real process Windows (or Popen) can start directly
    # and whose exit code propagates.
    for name in ('opencode.cmd', 'opencode.exe'):
        found = shutil.which(name)
        if found:
            return found
    return None


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--slot', type=int, required=True)
    ap.add_argument('--packet', required=True)
    ap.add_argument('--model', default='deepseek/deepseek-v4-flash')
    ap.add_argument('--stall-minutes', type=int, default=12)  # unused here; slot_status.py owns the stall check
    ap.add_argument('--reset', action='store_true')
    ap.add_argument('--dry-run', action='store_true')
    args = ap.parse_args()

    slot_root = REPO_ROOT / 'loop' / 'slots' / str(args.slot)
    wt = slot_root / 'wt'
    target_dir = slot_root / 'target'
    events_log = slot_root / 'events.jsonl'

    if not wt.is_dir():
        sys.exit(f"slot {args.slot} has no worktree at {wt}; run new_slot.py --slot {args.slot} --branch NAME first")
    packet_path = Path(args.packet)
    if not packet_path.is_file():
        sys.exit(f"packet not found: {args.packet}")

    # A worker that died mid-packet (V0 preflight: BLOCKED) leaves edits in the
    # worktree, and dispatching on top of them mixes a dead run's work into a
    # live one's diff. --reset clears the slot, but never silently: the
    # abandoned work is written to a patch beside the slot first, because a
    # run that got far enough to edit files is evidence about the packet even
    # when it is not usable code. Deciding to discard work stays an explicit
    # act, which is why this is a flag and not the default.
    porcelain = git_lines(wt, 'status', '--porcelain')
    import re
    dirty = [l for l in porcelain if not re.search(r'(?i)\s(PACKET\.md|worker\.(pid|err|packet))$', l)]

    if dirty:
        if not args.reset:
            sys.exit(f"slot {args.slot} has {len(dirty)} uncommitted change(s) from an earlier run. "
                      "Inspect them, or pass --reset to archive and discard them before dispatching.")
        stamp = datetime.datetime.now().strftime('%Y%m%d-%H%M%S')
        archive = slot_root / f"abandoned-{stamp}.patch"
        diff_res = subprocess.run(['git', '-C', str(wt), 'diff', 'HEAD'], capture_output=True, text=True)
        with archive.open('w', encoding='utf-8', newline='\n') as f:
            f.write(diff_res.stdout)
            untracked = git_lines(wt, 'ls-files', '--others', '--exclude-standard')
            if untracked:
                f.write("\n# untracked, not captured above:\n# " + "\n# ".join(untracked))
        print(f"archived {len(dirty)} abandoned change(s) to {archive}")
        subprocess.run(['git', '-C', str(wt), 'reset', '--hard', 'HEAD'], capture_output=True, text=True)
        subprocess.run(['git', '-C', str(wt), 'clean', '-fd', '-e', 'PACKET.md'], capture_output=True, text=True)

    # The packet is copied into the worktree and the prompt points at it,
    # rather than being passed as the prompt itself (S2). A packet is ~9 KB
    # and the launcher is a .cmd shim, whose command line dies at 8191
    # characters with "The command line is too long" -- which arrives as an
    # empty event stream, not as an error the worker can report. Handing over
    # a file also leaves the slot holding an exact record of what its worker
    # was given.
    #
    # The worker still reads only this one file: not PACKETS.jsonl, not the
    # build spec (S3a).
    packet_copy = wt / 'PACKET.md'
    shutil.copyfile(packet_path, packet_copy)
    packet_text = ("Read the file PACKET.md in the root of this repository and carry out the "
                    "work packet it describes, exactly and completely. It is self-contained: do "
                    "not read any other specification file. Follow its stop conditions, and "
                    "finish by writing RESULT.json as it instructs.")

    if args.dry_run:
        print("DRY RUN -- would execute:")
        print(f'  opencode run --dir "{wt}" -m {args.model} --format json --auto (contents of {args.packet})')
        print(f"  (CARGO_TARGET_DIR={target_dir}, CARGO_INCREMENTAL=0)")
        print(f"  event stream teed to: {events_log}")
        sys.exit(0)

    # CARGO_INCREMENTAL=0 / CARGO_TARGET_DIR so any cargo invocations the
    # worker makes land in this slot's sticky target dir, per S7 rule 1/2.
    env = dict(os.environ)
    env['CARGO_INCREMENTAL'] = '0'
    env['CARGO_TARGET_DIR'] = str(target_dir)

    print(f"Running packet '{args.packet}' in slot {args.slot} (model={args.model})...")

    # --format json gives the machine-readable event stream the orchestrator's
    # S3(b) context-budget supervisor reads (cumulative tokens, turn count)
    # and that the ledger records.
    #
    # Launched detached with the stream redirected to a file rather than
    # piped, because a pipe gives us no handle to kill: BG-S0-002's first run
    # stopped emitting events mid-step and sat there for 45 minutes on a hung
    # API call, holding a slot and its write set while producing nothing.
    # Growth of the event log is the liveness signal -- an idle worker and a
    # working one look identical from CPU time, since both are mostly waiting
    # on the model.
    if events_log.exists():
        events_log.unlink()
    err_log = slot_root / 'worker.err'

    launcher = find_opencode_launcher()
    if not launcher:
        sys.exit("cannot find an opencode.cmd or opencode.exe to launch")

    events_fh = open(events_log, 'wb')
    err_fh = open(err_log, 'wb')

    # Fire and forget. A worker runs for tens of minutes; anything that waits
    # on it is a long-lived process of its own, and when that waiter was
    # killed it took the worker with it mid-run. The orchestrator polls
    # slot_status.py instead -- short calls, no parent to lose -- and that is
    # also where the stall check lives now.
    #
    # DETACHED_PROCESS + CREATE_NEW_PROCESS_GROUP: the worker gets its own
    # console and process group, so it is not a child of this Python process
    # in any sense that matters -- this process exiting (or being killed) does
    # not touch it.
    proc = subprocess.Popen(
        [launcher, 'run', '--dir', str(wt), '-m', args.model, '--format', 'json', '--auto', packet_text],
        stdout=events_fh, stderr=err_fh, env=env,
        creationflags=subprocess.DETACHED_PROCESS | subprocess.CREATE_NEW_PROCESS_GROUP,
        close_fds=True,
    )
    events_fh.close()
    err_fh.close()

    (slot_root / 'worker.pid').write_text(str(proc.pid), encoding='ascii')
    (slot_root / 'worker.packet').write_text(args.packet, encoding='ascii')
    print(f"started pid {proc.pid}; poll with loop/slot_status.py")
    sys.exit(0)


if __name__ == '__main__':
    main()
