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


CREATE_BREAKAWAY_FROM_JOB = 0x01000000


def spawn_detached(argv, events_log, err_log, env, slot_root, tag='worker'):
    """Start a long-running process that outlives us and streams to a file.

    Two things went wrong with the obvious `Popen(argv, stdout=fh,
    creationflags=DETACHED_PROCESS)` and both are recorded here because both
    presented as silence rather than as an error.

    **The redirect has to happen inside cmd, not in Popen.** `opencode` is a
    .cmd shim, so the process doing the real work is a *grandchild*. Handing
    Popen a file object redirects the shim's own stdout -- the last run proved
    that much, the file received cmd.exe's `Terminate batch job (Y/N)?` -- but
    the worker's JSON never arrived, while seven source files were being edited
    in the worktree. A `>` written into a command file binds the whole command
    chain, grandchildren included. It also leaves the slot holding the exact
    command line that was run, which is worth having when a dispatch misbehaves.

    **DETACHED_PROCESS is what silenced the worker, and it was there to detach
    it.** Measured across eight flag combinations against a counting child:
    every combination containing DETACHED_PROCESS produced zero bytes, and every
    combination without it streamed (80 bytes at 3s, 143 at 5s). A batch file
    with no console apparently cannot get its own child's output to an inherited
    handle. Whatever the mechanism, the flag that was supposed to make the
    worker independent is the one that made it invisible -- and since liveness
    is inferred from that stream, invisible means reaped as stalled.

    What actually gives independence is CREATE_BREAKAWAY_FROM_JOB. A harness
    that runs its tools inside a Windows job kills every process in that job
    when the tool call ends, which is how a Ctrl-C in the orchestrator reached a
    worker that was supposed to be fire-and-forget; DETACHED_PROCESS never
    addressed that at all. CREATE_NO_WINDOW keeps it out of sight without
    taking the console away. Not every job permits breakaway, so failure falls
    back and says so: a worker that dies with its parent is worse than one that
    survives, but better than no worker.
    """
    runner = slot_root / f'{tag}-cmd.bat'
    # No argument the loop passes contains a double quote (the prompt is fixed
    # text and the packet itself goes via PACKET.md), so quoting spaces is
    # enough; a quote in an argument would need escaping and is refused above.
    for a in argv:
        if '"' in a:
            raise ValueError(f"argument contains a double quote, which this launcher cannot quote: {a!r}")
    quoted = ' '.join(f'"{a}"' if ' ' in a else a for a in argv)
    runner.write_text(
        "@echo off\r\n"
        f'{quoted} > "{events_log}" 2> "{err_log}"\r\n',
        encoding='utf-8')

    flags = (CREATE_NO_WINDOW | subprocess.CREATE_NEW_PROCESS_GROUP
             | CREATE_BREAKAWAY_FROM_JOB)
    try:
        proc = subprocess.Popen(['cmd.exe', '/c', str(runner)], env=env,
                                creationflags=flags, close_fds=True)
    except OSError:
        print("warning: this job forbids breakaway; the worker will not outlive a "
              "kill of its parent")
        proc = subprocess.Popen(
            ['cmd.exe', '/c', str(runner)], env=env,
            creationflags=CREATE_NO_WINDOW | subprocess.CREATE_NEW_PROCESS_GROUP,
            close_fds=True)
    return proc.pid


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

    # Fire and forget. A worker runs for tens of minutes; anything that waits
    # on it is a long-lived process of its own, and when that waiter was
    # killed it took the worker with it mid-run. The orchestrator polls
    # slot_status.py instead -- short calls, no parent to lose -- and that is
    # also where the stall check lives now.
    worker_pid = spawn_detached(
        [launcher, 'run', '--dir', str(wt), '-m', args.model, '--format', 'json', '--auto', packet_text],
        events_log, err_log, env, slot_root)

    (slot_root / 'worker.pid').write_text(str(worker_pid), encoding='ascii')
    (slot_root / 'worker.packet').write_text(args.packet, encoding='ascii')
    # Records which branch this dispatch actually landed on -- new_slot.py
    # decides that, not this script, so this is a read of the worktree's
    # current branch rather than a value this script chose. Without it, the
    # only way to find a packet's attempt branch after the fact is prose in
    # STATE.md, which is what happened landing BG-S0-002.
    dispatch_branch = git_lines(wt, 'rev-parse', '--abbrev-ref', 'HEAD')
    (slot_root / 'worker.branch').write_text(dispatch_branch[0] if dispatch_branch else '?', encoding='ascii')
    print(f"started pid {worker_pid}; poll with loop/slot_status.py")
    sys.exit(0)


if __name__ == '__main__':
    main()
