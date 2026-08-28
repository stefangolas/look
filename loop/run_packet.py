"""Dispatch one packet to a warm slot's worker (S2: opencode run, deepseek by
default). One packet, one process, one context reset - the architecture's
isolation unit, not agent discipline.

This script only launches the worker and captures its event stream; it does
not judge the result. verify.py is the only acceptance authority (S5).

Usage: python loop/run_packet.py --slot 0 --packet loop/packets/BG-S0-002.md [--model ...] [--reset] [--dry-run]
"""
import argparse
import datetime
import re
import os
import shutil
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(Path(__file__).resolve().parent))

CREATE_NO_WINDOW = 0x08000000


def git_lines(wt, *args):
    res = subprocess.run(['git', '-C', str(wt), *args], capture_output=True, text=True, encoding='utf-8', errors='replace')
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


def gate4_state(wt):
    """GATE-4's two numbers, read the way the gate itself reads them.

    Both come from the SLOT's worktree at HEAD, not from the orchestrator's
    checkout, and that is the whole point of the check. `scripts/kernel-gates.sh`
    runs inside the worktree under verification, so the ceiling that will judge a
    worker is the one committed on the branch it was forked onto -- raising the
    ceiling on integration/kernel-bg *after* a slot was forked leaves the slot
    with the old value and rejects the packet anyway. The count uses the gate's
    own pathspec and exclusion so the two cannot drift apart.

    Returns (count, ceiling); ceiling is None when the file is absent at HEAD.
    """
    res = subprocess.run(
        ['git', '-C', str(wt), 'grep', '-oh', 'unscaled_legacy(', 'HEAD', '--',
         'vendor/truck/*/src/*', ':(exclude)vendor/truck/truck-base/src/tolerance.rs'],
        capture_output=True, text=True, encoding='utf-8', errors='replace')
    # git grep exits 1 when it matches nothing, which is the healthy state here
    # and not an error -- the same trap that killed kernel-gates.sh silently on
    # a clean tree. The return code is deliberately not checked.
    count = len([l for l in res.stdout.splitlines() if l.strip()])

    show = subprocess.run(
        ['git', '-C', str(wt), 'show', 'HEAD:scripts/unscaled_legacy_ceiling.txt'],
        capture_output=True, text=True, encoding='utf-8', errors='replace')
    if show.returncode != 0:
        return count, None
    digits = ''.join(c for c in show.stdout if c.isdigit())
    return count, (int(digits) if digits else 0)


def check_unscaled_legacy_budget(packet_path, wt):
    """Refuse to dispatch a Stage-A shard whose ceiling has not been raised yet.

    A packet states a fact about the repo -- "the ceiling has been raised to
    cover at most 12 new call sites" -- and nothing checked that the fact was
    true at the moment of dispatch, or had ever been. It is the same defect as
    an anchor going stale, and it fails the same expensive way: the worker does
    exactly what the packet told it to, GATE-4 (through V4) rejects the commit
    for exceeding a ceiling nobody moved, and the rejection reads as a bad
    worker rather than a bad dispatch. That costs a full worker run -- tens of
    minutes -- to discover something two `git show`s settle here.

    Opt-in: a packet with no `unscaled_legacy_budget:` in its front block is not
    a Stage-A shard and is not checked.
    """
    text = packet_path.read_text(encoding='utf-8')
    m = re.search(r"(?m)^unscaled_legacy_budget:\s*(\d+)", text)
    if not m:
        return
    budget = int(m.group(1))

    count, ceiling = gate4_state(wt)
    if ceiling is None:
        sys.exit("this packet declares unscaled_legacy_budget, but the slot's HEAD has no "
                 "scripts/unscaled_legacy_ceiling.txt -- GATE-4 cannot be satisfied from here.")

    if count + budget > ceiling:
        sys.exit(
            "refusing to dispatch: GATE-4 would reject this packet's own work.\n"
            f"  unscaled_legacy() sites at the slot's HEAD: {count}\n"
            f"  this packet is budgeted to add:             {budget}\n"
            f"  ceiling committed on the slot's branch:     {ceiling}\n"
            f"  {count} + {budget} = {count + budget} > {ceiling}\n"
            f"Raise the ceiling to at least {count + budget} in "
            "scripts/unscaled_legacy_ceiling.txt, commit it on the branch this slot forks\n"
            "from, re-run new_slot.py so the raise is in the slot's HEAD, and lower the\n"
            "ceiling to the true count in the commit that closes the packet. The ceiling\n"
            "is a ratchet, not a target.")

    print(f"GATE-4 preflight: {count} unscaled_legacy site(s) + {budget} budgeted "
          f"<= ceiling {ceiling}, read from the slot's HEAD.")


def first_session_id(events_log):
    """The sessionID of the run that wrote this events log (first event line).

    opencode's JSON event stream carries `sessionID` on every line; the first
    one names the session the whole run lived in, which `--resume` hands back
    to `opencode run -s` so an amendment continues the worker that already
    knows the code."""
    try:
        with events_log.open(encoding='utf-8', errors='replace') as f:
            for line in f:
                if not line.strip():
                    continue
                try:
                    import json
                    r = json.loads(line)
                except ValueError:
                    continue
                sid = r.get('sessionID')
                if sid:
                    return str(sid)
    except OSError:
        pass
    return None


def archive_and_reset(slot_root, wt, dirty):
    """Archive a dead run's edits to a patch beside the slot, then hard-reset
    the worktree. Shared by the --reset dispatch path and --reset-only; the
    archive exists because a run that got far enough to edit files is evidence
    about the packet even when it is not usable code."""
    stamp = datetime.datetime.now().strftime('%Y%m%d-%H%M%S')
    archive = slot_root / f"abandoned-{stamp}.patch"
    diff_res = subprocess.run(['git', '-C', str(wt), 'diff', 'HEAD'], capture_output=True, text=True, encoding='utf-8', errors='replace')
    with archive.open('w', encoding='utf-8', newline='\n') as f:
        f.write(diff_res.stdout)
        untracked = git_lines(wt, 'ls-files', '--others', '--exclude-standard')
        if untracked:
            f.write("\n# untracked, not captured above:\n# " + "\n# ".join(untracked))
    print(f"archived {len(dirty)} abandoned change(s) to {archive}")
    subprocess.run(['git', '-C', str(wt), 'reset', '--hard', 'HEAD'], capture_output=True, text=True, encoding='utf-8', errors='replace')
    subprocess.run(['git', '-C', str(wt), 'clean', '-fd', '-e', 'PACKET.md'], capture_output=True, text=True, encoding='utf-8', errors='replace')


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--slot', type=int, required=True)
    ap.add_argument('--packet', required=True)
    ap.add_argument('--model', default='deepseek/deepseek-v4-flash')
    ap.add_argument('--stall-minutes', type=int, default=12)  # unused here; slot_status.py owns the stall check
    ap.add_argument('--reset', action='store_true')
    ap.add_argument('--reset-only', action='store_true',
                    help='archive-and-reset the slot WITHOUT spawning a worker '
                         '(--reset is archive-and-DISPATCH; resetting only '
                         'previously meant killing a spawned worker by hand)')
    ap.add_argument('--dry-run', action='store_true')
    ap.add_argument('--resume', action='store_true',
                    help="resume the slot's previous worker session (opencode "
                         "-s) instead of a fresh context -- the amendment "
                         "path: the prior worker already knows the code, the "
                         "packet and its own prior findings")
    ap.add_argument('--session-id',
                    help='explicit session id to resume (overrides --resume)')
    ap.add_argument('--context-diff',
                    help='git range (e.g. fc8925f..HEAD) whose log/diffstat '
                         'CONTEXT.md should carry for an amendment dispatch')
    args = ap.parse_args()

    slot_root = REPO_ROOT / 'loop' / 'slots' / str(args.slot)
    wt = slot_root / 'wt'
    target_dir = slot_root / 'target'
    events_log = slot_root / 'events.jsonl'

    if not wt.is_dir():
        sys.exit(f"slot {args.slot} has no worktree at {wt}; run new_slot.py --slot {args.slot} --branch NAME first")

    if args.reset_only:
        porcelain = git_lines(wt, 'status', '--porcelain')
        dirty = [l for l in porcelain if not re.search(r'(?i)\s(PACKET\.md|CONTEXT\.md|worker\.(pid|err|packet))$', l)]
        if dirty:
            archive_and_reset(slot_root, wt, dirty)
        else:
            print(f"slot {args.slot} is clean; nothing to reset")
        sys.exit(0)

    packet_path = Path(args.packet)
    if not packet_path.is_file():
        sys.exit(f"packet not found: {args.packet}")

    check_unscaled_legacy_budget(packet_path, wt)

    # Anchors and budget are claims about the tree, and both have shipped wrong:
    # GEOM-SPECIFIEDS had three of seven anchor counts wrong on files that had
    # not changed, and a budget estimated at 12 against a true 19. gen_packet
    # runs them. A mismatch is a stop condition (H-8), so this refuses to
    # dispatch rather than warning into a log nobody reads -- a worker told a
    # wrong count stops with ANCHOR_MISMATCH, after the run has been paid for.
    import gen_packet
    _problems = gen_packet.check(packet_path, quiet=True)
    if _problems:
        _detail = "\n  ".join(_problems)
        sys.exit(
            "refusing to dispatch: this packet's claims no longer hold.\n  "
            + _detail
            + "\nFix the packet, or -- if the tree moved -- re-scope it."
        )

    # A worker that died mid-packet (V0 preflight: BLOCKED) leaves edits in the
    # worktree, and dispatching on top of them mixes a dead run's work into a
    # live one's diff. --reset clears the slot, but never silently: the
    # abandoned work is written to a patch beside the slot first, because a
    # run that got far enough to edit files is evidence about the packet even
    # when it is not usable code. Deciding to discard work stays an explicit
    # act, which is why this is a flag and not the default.
    porcelain = git_lines(wt, 'status', '--porcelain')
    # CONTEXT.md is dispatch scaffolding beside PACKET.md (regenerated from
    # the tree on every dispatch, never committed, ignored by verify.py by
    # name); a dead run's stale copy must not block the next dispatch the
    # same way a dead run's source edit must.
    dirty = [l for l in porcelain if not re.search(r'(?i)\s(PACKET\.md|CONTEXT\.md|worker\.(pid|err|packet))$', l)]

    if dirty:
        if not args.reset:
            sys.exit(f"slot {args.slot} has {len(dirty)} uncommitted change(s) from an earlier run. "
                      "Inspect them, or pass --reset to archive and discard them before dispatching.")
        archive_and_reset(slot_root, wt, dirty)

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

    # CONTEXT.md: a deterministic bundle of the allow-listed files'
    # signatures, callers and tests, generated from THIS tree at dispatch
    # time (session 20). It cannot go stale because it is never committed;
    # verify.py ignores it by name like PACKET.md. Failures to generate are
    # non-fatal -- a dispatch without a bundle beats no dispatch.
    try:
        import gen_context
        ctx = gen_context.generate(packet_path, wt, args.context_diff)
        print(f"context bundle: {ctx.name} written ({len(ctx.read_text(encoding='utf-8').splitlines())} lines)")
    except Exception as exc:  # noqa: BLE001 - deliberately best-effort
        print(f"context bundle skipped: {exc}")

    resume_session = None
    if args.session_id:
        resume_session = args.session_id.strip()
    elif args.resume:
        resume_session = first_session_id(events_log)
        if not resume_session:
            sys.exit('--resume: no sessionID in this slot\'s events.jsonl (the '
                     'previous run never emitted an event, or the log was '
                     'rotated away). Dispatch without --resume, or pass '
                     '--session-id explicitly.')

    lead = ("You are continuing your previous session in this repository; your "
            "earlier work is committed on this branch and the packet amends "
            "it. " if resume_session else "")
    packet_text = (lead +
                   "Read the files PACKET.md and CONTEXT.md in the root of this "
                   "repository and carry out the work packet PACKET.md describes, "
                   "exactly and completely. PACKET.md is self-contained: do not "
                   "read any other specification file. CONTEXT.md is a "
                   "machine-generated index of relevant signatures, callers and "
                   "tests -- use it to skip the initial search, but read any file "
                   "you actually edit. Follow the packet's stop conditions, and "
                   "finish by writing RESULT.json as it instructs.")

    if args.dry_run:
        print("DRY RUN -- would execute:")
        print(f'  opencode run --dir "{wt}" -m {args.model} --format json --auto (contents of {args.packet})')
        print(f"  (CARGO_TARGET_DIR={target_dir}, CARGO_INCREMENTAL=0)")
        print(f"  event stream teed to: {events_log}")
        sys.exit(0)

    # CARGO_TARGET_DIR keeps every cargo invocation in this slot's sticky
    # target dir (S7 rule 1). CARGO_INCREMENTAL is deliberately ON for the
    # worker (session 20, reversing the earlier `= 0`): a packet performs
    # 5-15 edit-rebuild cycles and incremental compilation is the difference
    # between a one-second and a minute-scale inner loop. The verifier does
    # its own authoritative builds in separate baselines, a slot's target is
    # reclaimed on re-fork anyway, and the watchdog only touches idle slots.
    env = dict(os.environ)
    env['CARGO_INCREMENTAL'] = '1'
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
    worker_argv = [launcher, 'run', '--dir', str(wt), '-m', args.model,
                   '--format', 'json', '--auto']
    if resume_session:
        worker_argv += ['-s', resume_session]
    worker_argv.append(packet_text)
    worker_pid = spawn_detached(worker_argv, events_log, err_log, env, slot_root)

    (slot_root / 'worker.pid').write_text(str(worker_pid), encoding='ascii')
    (slot_root / 'worker.packet').write_text(args.packet, encoding='ascii')
    # The dispatching model, recorded so land_packet's ledger row is the truth
    # instead of a hardcoded default. The worker-model switch is otherwise
    # invisible after the fact (the BG-S0-002 lesson: prose is not provenance).
    (slot_root / 'worker.model').write_text(args.model, encoding='ascii')
    if resume_session:
        (slot_root / 'worker.session').write_text(resume_session, encoding='ascii')
        print(f'resuming worker session {resume_session}')
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
