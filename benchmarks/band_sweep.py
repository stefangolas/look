"""Sweep the ABC corpus through the cylinder-band route in both gate states.

This is a *census* harness. It runs the shipped `face_census` and
`band_curve_probe` binaries unmodified, in one isolated process per run, and
writes ledgers. It admits nothing, relaxes nothing, and cannot move a face
between rendered and lost — every number it reports comes from a binary built
from the pinned revisions.

Why each guard is here:

- **One process per (file, mode).** A 540 MB model that crashes, hangs, or
  exhausts memory is its own outcome and must not cost the census of the other
  nineteen. The outcome vocabulary is explicit — `completed`, `parse_failure`,
  `crash`, `timeout`, `resource_failure` — because "no row in the output" is
  not a finding.
- **Resumable, content-addressed cache.** The key is the file's SHA-256, the
  `look` revision, the truck revision, the `Cargo.lock` hash, and the mode. A
  measurement taken over a different truck is not the same measurement, and
  `.cargo/config.toml`'s `paths` override is invisible in `Cargo.lock` — so the
  key alone cannot prove a run was clean and `--check-pins` is run separately.
- **Smallest file first.** Free physical memory on this machine is the binding
  constraint; ordering by size means a resource failure on the largest model
  arrives after nineteen good results rather than instead of them.
- **Peak working set is recorded, never differenced across runs.** Windows
  trims working sets under pressure, so two peaks taken under different machine
  states are not comparable (see `step_corpus.py`, which learned this first).

Usage:

    python benchmarks/band_sweep.py --dir C:/Users/stefa/look-corpus/abc \\
        --out sweep-out --exe-dir target/x86_64-pc-windows-gnullvm/release/examples

    python benchmarks/band_sweep.py --out sweep-out --check-pins
"""

from __future__ import annotations

import argparse
import gzip
import hashlib
import json
import os
import shutil
import subprocess
import sys
import time
from pathlib import Path

# `face_census` with the band gate closed, then open. `TRUCK_FORMAL_RECOVERY`
# alone opens the *planar* rank-0 route, which is a different population; the
# band gate is standalone precisely so its delta is the band's own count.
MODES = {
    "gate_closed": {},
    "band_enabled": {"TRUCK_FORMAL_RECOVERY_BAND": "1"},
}

DEFAULT_TIMEOUT_S = 3600


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1 << 22), b""):
            digest.update(chunk)
    return digest.hexdigest()


def free_physical_gb() -> float:
    """Free physical memory, or -1 where it cannot be read.

    Reported rather than enforced: Windows counts reclaimable standby pages as
    used, so a low reading is a caution to record beside a result, not grounds
    to refuse to take it.
    """
    try:
        import ctypes
        from ctypes import wintypes

        class Status(ctypes.Structure):
            _fields_ = [
                ("dwLength", wintypes.DWORD),
                ("dwMemoryLoad", wintypes.DWORD),
                ("ullTotalPhys", ctypes.c_ulonglong),
                ("ullAvailPhys", ctypes.c_ulonglong),
                ("ullTotalPageFile", ctypes.c_ulonglong),
                ("ullAvailPageFile", ctypes.c_ulonglong),
                ("ullTotalVirtual", ctypes.c_ulonglong),
                ("ullAvailVirtual", ctypes.c_ulonglong),
                ("ullAvailExtendedVirtual", ctypes.c_ulonglong),
            ]

        status = Status()
        status.dwLength = ctypes.sizeof(Status)
        ctypes.windll.kernel32.GlobalMemoryStatusEx(ctypes.byref(status))
        return status.ullAvailPhys / (1 << 30)
    except Exception:
        return -1.0


def git_rev(repo: Path) -> str:
    try:
        out = subprocess.run(
            ["git", "-C", str(repo), "rev-parse", "HEAD"],
            capture_output=True,
            text=True,
            check=True,
        )
        return out.stdout.strip()[:12]
    except Exception:
        return "unknown"


def truck_rev(repo: Path) -> str:
    """The truck revision `Cargo.lock` actually resolved.

    Read from the lock file, not from `Cargo.toml`: the lock records what was
    built, and a `[patch]` or a path override can make the two disagree.
    """
    lock = (repo / "Cargo.lock").read_text(encoding="utf-8", errors="replace")
    for line in lock.splitlines():
        if "stefangolas/truck" in line and "rev=" in line:
            return line.split("rev=")[1].split("#")[0].strip('"').strip()
    return "unknown"


def check_pins(repo: Path) -> list[str]:
    """Refuse to report a number taken through a path override.

    The override is invisible in `Cargo.lock`, so this reads the two places it
    can live and reports what it finds. Returns a list of complaints.
    """
    complaints = []
    config = repo / ".cargo" / "config.toml"
    if config.exists():
        for raw in config.read_text(encoding="utf-8").splitlines():
            line = raw.strip()
            if line.startswith("#") or not line:
                continue
            if line.startswith("paths"):
                complaints.append(f"{config}: live `paths` override: {line}")
    cargo_toml = (repo / "Cargo.toml").read_text(encoding="utf-8")
    for raw in cargo_toml.splitlines():
        line = raw.strip()
        if line.startswith("#") or not line:
            continue
        if "path =" in line and "truck" in line:
            complaints.append(f"Cargo.toml: local path patch: {line}")
    lock = (repo / "Cargo.lock").read_text(encoding="utf-8", errors="replace")
    if "stefangolas/truck" not in lock:
        complaints.append("Cargo.lock: no stefangolas/truck git source")
    return complaints


def classify(returncode: int, timed_out: bool, stdout: str, stderr: str) -> str:
    """The explicit per-run outcome.

    A nonzero exit with a parse diagnostic is a statement about the *file*; a
    nonzero exit without one is a statement about the *program*. Collapsing
    them would hide a regression inside a corpus-quality excuse.
    """
    if timed_out:
        return "timeout"
    if returncode == 0:
        return "completed"
    blob = (stdout + stderr).lower()
    if "failed to parse step" in blob or "no such file" in blob:
        return "parse_failure"
    for marker in ("memory allocation of", "out of memory", "cannot allocate"):
        if marker in blob:
            return "resource_failure"
    return "crash"


def run_one(exe: Path, model: Path, env_extra: dict, timeout: int, ledger: bool):
    env = dict(os.environ)
    env.update(env_extra)
    args = [str(exe)]
    if ledger:
        args.append("--ledger")
    args.append(str(model))
    started = time.time()
    timed_out = False
    try:
        proc = subprocess.run(
            args,
            capture_output=True,
            text=True,
            errors="replace",
            env=env,
            timeout=timeout,
        )
        stdout, stderr, code = proc.stdout, proc.stderr, proc.returncode
    except subprocess.TimeoutExpired as expired:
        timed_out = True
        stdout = expired.stdout or ""
        stderr = expired.stderr or ""
        if isinstance(stdout, bytes):
            stdout = stdout.decode("utf-8", "replace")
        if isinstance(stderr, bytes):
            stderr = stderr.decode("utf-8", "replace")
        code = -1
    return {
        "outcome": classify(code, timed_out, stdout, stderr),
        "returncode": code,
        "seconds": round(time.time() - started, 2),
        "free_gb_before": None,
        "stdout": stdout,
        "stderr": stderr,
    }


def write_gz(path: Path, text: str) -> str:
    path.parent.mkdir(parents=True, exist_ok=True)
    data = text.encode("utf-8", "replace")
    with gzip.open(path, "wb") as handle:
        handle.write(data)
    return hashlib.sha256(data).hexdigest()[:16]


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--dir", default="C:/Users/stefa/look-corpus/abc")
    parser.add_argument("--out", default="sweep-out")
    parser.add_argument("--repo", default=".")
    parser.add_argument(
        "--exe-dir", default="target/x86_64-pc-windows-gnullvm/release/examples"
    )
    parser.add_argument("--timeout", type=int, default=DEFAULT_TIMEOUT_S)
    parser.add_argument("--check-pins", action="store_true")
    parser.add_argument("--force", action="store_true", help="ignore the cache")
    parser.add_argument("--limit", type=int, default=0)
    args = parser.parse_args()

    repo = Path(args.repo).resolve()
    complaints = check_pins(repo)
    if args.check_pins:
        for complaint in complaints:
            print(f"PIN PROBLEM: {complaint}")
        print("pins clean" if not complaints else f"{len(complaints)} problem(s)")
        return 1 if complaints else 0
    if complaints:
        for complaint in complaints:
            print(f"PIN PROBLEM: {complaint}", file=sys.stderr)
        print("refusing to sweep through an override", file=sys.stderr)
        return 1

    out = Path(args.out).resolve()
    out.mkdir(parents=True, exist_ok=True)
    exe_dir = (repo / args.exe_dir).resolve()
    census_exe = exe_dir / "face_census.exe"
    probe_exe = exe_dir / "band_curve_probe.exe"
    for exe in (census_exe, probe_exe):
        if not exe.exists():
            print(f"missing binary: {exe}", file=sys.stderr)
            return 1

    look_rev = git_rev(repo)
    truck = truck_rev(repo)
    lock_hash = hashlib.sha256((repo / "Cargo.lock").read_bytes()).hexdigest()[:16]
    print(f"look={look_rev} truck={truck} Cargo.lock={lock_hash}")

    index_path = out / "index.json"
    index = {}
    if index_path.exists() and not args.force:
        index = json.loads(index_path.read_text())

    models = sorted(
        (p for p in Path(args.dir).rglob("*") if p.suffix.lower() in (".step", ".stp")),
        key=lambda p: p.stat().st_size,
    )
    if args.limit:
        models = models[: args.limit]
    print(f"{len(models)} model(s) discovered under {args.dir}")

    for position, model in enumerate(models, 1):
        model_id = model.parent.name
        size_mb = model.stat().st_size / (1 << 20)
        digest = sha256_file(model)
        for mode, env_extra in MODES.items():
            key = "|".join([digest, look_rev, truck, lock_hash, mode])
            if key in index and not args.force:
                print(f"[{position}/{len(models)}] {model_id} {mode}: cached")
                continue
            free = free_physical_gb()
            print(
                f"[{position}/{len(models)}] {model_id} {mode}: "
                f"{size_mb:.0f} MB, {free:.1f} GB free ... ",
                end="",
                flush=True,
            )
            result = run_one(census_exe, model, env_extra, args.timeout, ledger=True)
            result["free_gb_before"] = round(free, 2)
            ledger_hash = write_gz(
                out / model_id / f"{mode}.ledger.tsv.gz", result.pop("stderr")
            )
            summary_hash = write_gz(
                out / model_id / f"{mode}.summary.txt.gz", result.pop("stdout")
            )
            index[key] = {
                "model_id": model_id,
                "path": str(model),
                "bytes": model.stat().st_size,
                "sha256": digest,
                "mode": mode,
                "look_rev": look_rev,
                "truck_rev": truck,
                "cargo_lock": lock_hash,
                "ledger_sha256_16": ledger_hash,
                "summary_sha256_16": summary_hash,
                **result,
            }
            index_path.write_text(json.dumps(index, indent=1))
            print(f"{result['outcome']} in {result['seconds']}s")

        # The curve probe is gate-independent: it reads the imported
        # representation, which no recovery gate can change. One run per file.
        key = "|".join([digest, look_rev, truck, lock_hash, "curve_probe"])
        if key in index and not args.force:
            print(f"[{position}/{len(models)}] {model_id} curve_probe: cached")
            continue
        print(f"[{position}/{len(models)}] {model_id} curve_probe ... ", end="", flush=True)
        result = run_one(probe_exe, model, {}, args.timeout, ledger=False)
        curves_hash = write_gz(
            out / model_id / "curves.tsv.gz", result.pop("stdout")
        )
        probe_hash = write_gz(
            out / model_id / "curves.summary.txt.gz", result.pop("stderr")
        )
        index[key] = {
            "model_id": model_id,
            "path": str(model),
            "sha256": digest,
            "mode": "curve_probe",
            "look_rev": look_rev,
            "truck_rev": truck,
            "cargo_lock": lock_hash,
            "curves_sha256_16": curves_hash,
            "summary_sha256_16": probe_hash,
            **result,
        }
        index_path.write_text(json.dumps(index, indent=1))
        print(f"{result['outcome']} in {result['seconds']}s")

    print(f"\nindex: {index_path} ({len(index)} runs)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
