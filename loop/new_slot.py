"""Create or reset a sticky slot: one git worktree + one CARGO_TARGET_DIR,
reused across many packets (§1 "sticky slots"). The point of stickiness is
that the ~240-crate dependency build is paid once per slot, not once per
packet, so this script's job is to get a slot from "doesn't exist" or "left
dirty by a previous packet" to "clean checkout of --branch, warm target dir"
in one idempotent call.

Usage: python loop/new_slot.py --slot 0 --branch packet/BG-S0-002
"""
import argparse
import shutil
import subprocess
import sys
import time
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent


def git(cwd, *args):
    return subprocess.run(['git', '-C', str(cwd), *args], capture_output=True, text=True)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--slot', type=int, required=True)
    ap.add_argument('--branch', required=True)
    ap.add_argument('--min-free-gb', type=float, default=8)
    args = ap.parse_args()

    slot_root = REPO_ROOT / 'loop' / 'slots' / str(args.slot)
    wt = slot_root / 'wt'
    target_dir = slot_root / 'target'

    # §7 rule 5: refuse, don't flag. A slot warm build is the single largest
    # single write burst in the loop (target/quick-scale, ~2.5 GB), so check
    # the floor before touching disk at all rather than failing partway
    # through.
    free_gb = shutil.disk_usage('C:\\').free / 2**30
    if free_gb < args.min_free_gb:
        sys.exit(f"new_slot: {free_gb:.1f} GB free on C:, below the {args.min_free_gb:.1f} GB floor. "
                  "Run the janitor (see docs §7.2) before creating or warming a slot.")

    # The branch a fresh or reset slot forks from. In normal use this is
    # whatever branch the orchestrator itself is on (integration/kernel-bg);
    # parameterizing it as "current HEAD of the repo you invoked this from"
    # keeps the script usable from a detached-HEAD CI checkout too.
    base_ref = git(REPO_ROOT, 'rev-parse', '--abbrev-ref', 'HEAD').stdout.strip()
    if not base_ref or base_ref == 'HEAD':
        base_ref = git(REPO_ROOT, 'rev-parse', 'HEAD').stdout.strip()

    slot_root.mkdir(parents=True, exist_ok=True)

    is_existing_worktree = False
    if wt.exists():
        # Confirm it's actually a live git worktree and not just a stray
        # directory (e.g. left behind by a manual rm that missed .git/worktrees).
        res = git(wt, 'rev-parse', '--is-inside-work-tree')
        if res.returncode == 0:
            is_existing_worktree = True

    if is_existing_worktree:
        # Idempotent path: reuse the worktree and target dir, just repoint the
        # branch. -B creates the branch if it doesn't exist and force-resets
        # it to base_ref if it does -- this is the "reset rather than error"
        # contract.
        print(f"Slot {args.slot} worktree exists at {wt}; resetting branch {args.branch} to {base_ref}")
        res = git(wt, 'checkout', '-B', args.branch, base_ref)
        if res.returncode != 0:
            sys.exit(f"git checkout -B {args.branch} failed in {wt}: {res.stderr}")
        res = git(wt, 'reset', '--hard', base_ref)
        if res.returncode != 0:
            sys.exit(f"git reset --hard {base_ref} failed in {wt}: {res.stderr}")
        # Drop whatever the previous packet left uncommitted, but never the
        # target dir -- it lives outside wt (CARGO_TARGET_DIR points
        # elsewhere) so it is never at risk here; -e is defensive in case
        # that ever changes.
        git(wt, 'clean', '-fdx', '-e', 'target')
    else:
        if wt.exists():
            sys.exit(f"loop/slots/{args.slot}/wt exists but is not a git worktree; remove it manually before retrying.")
        branch_exists = bool(git(REPO_ROOT, 'branch', '--list', args.branch).stdout.strip())
        if branch_exists:
            print(f"Branch {args.branch} already exists; attaching worktree and resetting to {base_ref}")
            res = git(REPO_ROOT, 'worktree', 'add', str(wt), args.branch)
            if res.returncode != 0:
                sys.exit(f"git worktree add {wt} {args.branch} failed: {res.stderr}")
            git(wt, 'reset', '--hard', base_ref)
        else:
            print(f"Creating worktree {wt} on new branch {args.branch} from {base_ref}")
            res = git(REPO_ROOT, 'worktree', 'add', '-b', args.branch, str(wt), base_ref)
            if res.returncode != 0:
                sys.exit(f"git worktree add -b {args.branch} {wt} {base_ref} failed: {res.stderr}")

    target_dir.mkdir(parents=True, exist_ok=True)

    # CARGO_INCREMENTAL=0 per §7 rule 1 -- incremental state buys nothing in a
    # one-packet-per-process world and was 389 MB dead weight in the last audit.
    import os
    env = dict(os.environ)
    env['CARGO_INCREMENTAL'] = '0'
    env['CARGO_TARGET_DIR'] = str(target_dir)

    print(f"Warming slot {args.slot} ({wt}), target={target_dir} ...")
    start = time.monotonic()
    res = subprocess.run(['cargo', 'check', '--workspace', '--all-targets'], cwd=str(wt), env=env)
    elapsed_min = (time.monotonic() - start) / 60

    if res.returncode != 0:
        sys.exit(f"warm build failed (cargo check --workspace --all-targets exit {res.returncode}) in slot {args.slot}")

    target_size_gb = sum(f.stat().st_size for f in target_dir.rglob('*') if f.is_file()) / 2**30

    print(f"Warm build: {elapsed_min:.1f} min")
    print(f"Target dir size: {target_size_gb:.2f} GB")
    print(f"Free disk after warm: {shutil.disk_usage('C:\\\\').free / 2**30:.1f} GB")


if __name__ == '__main__':
    main()
