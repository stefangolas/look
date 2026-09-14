#!/usr/bin/env python3
"""The exact CI entry sequence, runnable locally before push.

LOOP-MACH-1 item 6. `.github/workflows/cross-platform.yml` runs, in order:

    cargo fmt --all -- --check
    scripts/kernel-gates.sh origin/main      # H-1/H-3/H-4/GATE-4
    cargo clippy --locked --all-targets -- -D warnings
    cargo check --locked --all-targets
    cargo test --all-targets

The cheap entry checks -- formatting, the executable-bit invariant, and the
H-gates -- are the ones a push should never fail. This script runs exactly
those, so the failure is local and free instead of a CI round trip:

  1. executable-bit invariant: every tracked shell script under `scripts/`
     is mode 100755 in the index. The CI job runs `scripts/kernel-gates.sh`
     directly (`scripts/kernel-gates.sh origin/main`), which fails with
     "Permission denied" the moment a checkout loses the bit; the index mode
     is the only thing git preserves across platforms, so that is what is
     checked. `install.sh` at the repo root is invoked as `sh install.sh`
     and is deliberately not part of the invariant.
  2. `cargo fmt --all -- --check`.
  3. `scripts/kernel-gates.sh <base>` -- the H-1/H-3/H-4/GATE-4 gates. Git's
     bash is used when present (WSL's `bash` launcher is not a shell on this
     host); when no usable bash exists the step is reported SKIPPED rather
     than silently passed, so a green line is never a false one.

Usage:
  python loop/ci_gate.py                 # fmt + exec-bit + H-gates
  python loop/ci_gate.py --base <ref>    # explicit H-gate baseline
  python loop/ci_gate.py --no-h-gates    # skip the shell gate
  python loop/ci_gate.py --json
"""

import argparse
import json
import shutil
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
SHELL_SCRIPT_DIR = "scripts"

_GIT_BASH_CANDIDATES = (
    r"C:\Program Files\Git\bin\bash.exe",
    r"C:\Program Files\Git\usr\bin\bash.exe",
    r"C:\Program Files (x86)\Git\bin\bash.exe",
)


def git(repo, *args):
    return subprocess.run(
        ["git", "-C", str(repo), *args], capture_output=True, text=True,
        encoding="utf-8", errors="replace")


def tracked_modes(repo):
    """{path: index mode} for every tracked file."""
    res = git(repo, "ls-files", "-s")
    modes = {}
    for line in res.stdout.splitlines():
        if not line.strip() or "\t" not in line:
            continue
        meta, path = line.split("\t", 1)
        fields = meta.split()
        if fields:
            modes[path] = fields[0]
    return modes


def exec_bit_violations(repo, directory=SHELL_SCRIPT_DIR):
    """Tracked shell scripts under `directory` whose index mode is not 100755.

    Returns a sorted list of (path, mode). A fresh checkout that lost the
    executable bit is exactly what this catches: the CI job invokes
    `scripts/kernel-gates.sh` directly, and the mode lives in the index.
    """
    violations = []
    for path, mode in sorted(tracked_modes(repo).items()):
        if not path.endswith(".sh"):
            continue
        parts = Path(path).parts
        if not parts or parts[0] != directory:
            continue
        if mode != "100755":
            violations.append((path, mode))
    return violations


def find_bash():
    """A bash that is a real shell (Git's), not the WSL launcher.

    On this host `bash` on PATH is the Windows Store WSL relay, which fails
    with `execvpe(/bin/bash) failed` when no distro is installed. Git ships a
    working bash next to the git it already uses; prefer that, and only fall
    back to PATH if a probe actually runs.
    """
    candidates = list(_GIT_BASH_CANDIDATES)
    which = shutil.which("bash")
    if which and "WindowsApps" not in which:
        candidates.append(which)
    for cand in candidates:
        if not Path(cand).exists():
            continue
        try:
            probe = subprocess.run([cand, "-c", "echo ok"], capture_output=True,
                                   text=True, encoding="utf-8", errors="replace",
                                   timeout=20)
        except (OSError, subprocess.SubprocessError):
            continue
        if probe.returncode == 0 and "ok" in probe.stdout:
            return cand
    return None


def run_fmt(repo):
    res = subprocess.run(["cargo", "fmt", "--all", "--", "--check"],
                         cwd=str(repo), capture_output=True, text=True,
                         encoding="utf-8", errors="replace")
    return res.returncode == 0, (res.stdout + res.stderr).strip()


def resolve_base(repo):
    for ref in ("origin/main", "origin/HEAD"):
        if git(repo, "rev-parse", "--verify", "--quiet", ref).returncode == 0:
            return ref
    return None


def run_h_gates(repo, base):
    bash = find_bash()
    if bash is None:
        return None, "no usable bash found (Git bash not installed)"
    if base is None:
        return None, "no baseline ref (origin/main absent); pass --base"
    res = subprocess.run([bash, "scripts/kernel-gates.sh", base],
                         cwd=str(repo), capture_output=True, text=True,
                         encoding="utf-8", errors="replace")
    return res.returncode == 0, (res.stdout + res.stderr).strip()


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--repo", default=str(REPO_ROOT))
    ap.add_argument("--base", help="H-gate baseline ref (default origin/main)")
    ap.add_argument("--no-fmt", action="store_true")
    ap.add_argument("--no-h-gates", action="store_true")
    ap.add_argument("--no-exec-bit", action="store_true")
    ap.add_argument("--json", action="store_true")
    args = ap.parse_args(argv)

    repo = Path(args.repo)
    report = {"repo": str(repo),
              "checked_at": git(repo, "rev-parse", "HEAD").stdout.strip(),
              "steps": []}
    failed = False

    if not args.no_exec_bit:
        bad = exec_bit_violations(repo)
        if bad:
            failed = True
            for path, mode in bad:
                report["steps"].append(
                    {"step": "exec-bit", "status": "FAIL", "path": path,
                     "mode": mode})
        else:
            report["steps"].append({"step": "exec-bit", "status": "PASS"})

    if not args.no_fmt:
        ok, detail = run_fmt(repo)
        if not ok:
            failed = True
            report["steps"].append({"step": "fmt", "status": "FAIL",
                                    "detail": detail})
        else:
            report["steps"].append({"step": "fmt", "status": "PASS"})

    if not args.no_h_gates:
        base = args.base or resolve_base(repo)
        ok, detail = run_h_gates(repo, base)
        if ok is None:
            report["steps"].append({"step": "h-gates", "status": "SKIPPED",
                                    "detail": detail})
        elif not ok:
            failed = True
            report["steps"].append({"step": "h-gates", "status": "FAIL",
                                    "base": base, "detail": detail})
        else:
            report["steps"].append({"step": "h-gates", "status": "PASS",
                                    "base": base, "detail": detail})

    report["ok"] = not failed
    if args.json:
        print(json.dumps(report, indent=2))
    else:
        for step in report["steps"]:
            line = f"ci_gate: {step['status']:<7} {step['step']}"
            if step.get("path"):
                line += f" {step['path']} (mode {step['mode']})"
            print(line)
            if step["status"] == "FAIL" and step.get("detail"):
                print("  " + step["detail"].replace("\n", "\n  "))
            if step["status"] == "SKIPPED":
                print(f"  {step['detail']}")
        print("ci_gate: " + ("FAILED" if failed else "green"))
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
