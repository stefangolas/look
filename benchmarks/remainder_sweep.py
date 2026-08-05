"""Sweep the ABC corpus for the post-circle remainder diagnosis (REMAINDER-DIAG-001).

This is an *observation-only* harness. Per model it takes three readings, all
from binaries built at the pinned revisions and all run in their own process:

- `face_census --ledger` with the band gate open — the rendered/lost verdict and
  the per-face band attempt, exactly as `band_sweep.py` takes it;
- the structured `FailedFaceDiagnosis` JSONL the same run emits, which is where
  the *realizer* stopped (typed terminal reason, chart rank, lift/deck/projection
  status, constraint-conflict witnesses);
- `remainder_probe` — what *source authority* each imported face carries, which
  no tessellation outcome can tell you.

`TRUCK_FACE_DIAG_JSONL` is set on the census run. That adds no behavioural
change on top of the band gate: `diagnosis::diag_enabled()` is already true
whenever `TRUCK_FORMAL_RECOVERY_BAND` is set, because the band route derives its
own admission bucket from the same sink. The two runs are therefore the same
measurement `band_sweep.py` takes, plus a file.

Every guard `band_sweep.py` documents applies here for the same reasons — one
process per reading, explicit outcome vocabulary, content-addressed resumable
cache keyed on the file digest and both revisions, smallest file first, and a
`--check-pins` gate so no number is reported through a path override.

Usage:

    python benchmarks/remainder_sweep.py --dir C:/Users/stefa/look-corpus/abc \\
        --out C:/Users/stefa/look-corpus/remainder-out
"""

from __future__ import annotations

import argparse
import gzip
import hashlib
import json
import os
import subprocess
import sys
import tempfile
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from band_sweep import (  # noqa: E402
    check_pins,
    classify,
    free_physical_gb,
    git_rev,
    sha256_file,
    truck_rev,
    write_gz,
)

DEFAULT_TIMEOUT_S = 3600

SCHEMA_VERSION = "remainder-diag-1"


def run(argv, env_extra, timeout):
    env = dict(os.environ)
    env.update(env_extra)
    started = time.time()
    timed_out = False
    try:
        proc = subprocess.run(
            argv, capture_output=True, text=True, errors="replace",
            env=env, timeout=timeout,
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
        "stdout": stdout,
        "stderr": stderr,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--dir", default="C:/Users/stefa/look-corpus/abc")
    parser.add_argument("--out", default="C:/Users/stefa/look-corpus/remainder-out")
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
    probe_exe = exe_dir / "remainder_probe.exe"
    for exe in (census_exe, probe_exe):
        if not exe.exists():
            print(f"missing binary: {exe}", file=sys.stderr)
            return 1

    look_rev = git_rev(repo)
    truck = truck_rev(repo)
    lock_hash = hashlib.sha256((repo / "Cargo.lock").read_bytes()).hexdigest()[:16]
    print(f"look={look_rev} truck={truck} Cargo.lock={lock_hash} schema={SCHEMA_VERSION}")

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
        stem = "|".join([digest, look_rev, truck, lock_hash, SCHEMA_VERSION])

        key = f"{stem}|census_diag"
        if key not in index or args.force:
            free = free_physical_gb()
            print(
                f"[{position}/{len(models)}] {model_id} census+diag: "
                f"{size_mb:.0f} MB, {free:.1f} GB free ... ",
                end="", flush=True,
            )
            # The JSONL goes to a scratch file the census writes itself; it is
            # read straight back and stored gzipped beside the ledger, so no
            # uncompressed copy outlives the run.
            with tempfile.TemporaryDirectory() as scratch:
                jsonl = Path(scratch) / "diag.jsonl"
                result = run(
                    [str(census_exe), "--ledger", str(model)],
                    {
                        "TRUCK_FORMAL_RECOVERY_BAND": "1",
                        "TRUCK_FACE_DIAG_JSONL": str(jsonl),
                    },
                    args.timeout,
                )
                diag_text = jsonl.read_text(encoding="utf-8", errors="replace") if jsonl.exists() else ""
                meta = Path(f"{jsonl}.meta.json")
                meta_text = meta.read_text(encoding="utf-8") if meta.exists() else "{}"
            diag_rows = sum(1 for line in diag_text.splitlines() if line.strip())
            ledger_hash = write_gz(out / model_id / "ledger.tsv.gz", result.pop("stderr"))
            summary_hash = write_gz(out / model_id / "summary.txt.gz", result.pop("stdout"))
            diag_hash = write_gz(out / model_id / "diag.jsonl.gz", diag_text)
            index[key] = {
                "model_id": model_id, "path": str(model),
                "bytes": model.stat().st_size, "sha256": digest,
                "reading": "census_diag", "look_rev": look_rev, "truck_rev": truck,
                "cargo_lock": lock_hash, "schema": SCHEMA_VERSION,
                "free_gb_before": round(free, 2),
                "ledger_sha256_16": ledger_hash, "summary_sha256_16": summary_hash,
                "diag_sha256_16": diag_hash, "diag_rows": diag_rows,
                "diag_meta": json.loads(meta_text or "{}"),
                **result,
            }
            index_path.write_text(json.dumps(index, indent=1))
            print(f"{index[key]['outcome']} in {index[key]['seconds']}s, {diag_rows} diag rows")
        else:
            print(f"[{position}/{len(models)}] {model_id} census+diag: cached")

        key = f"{stem}|source_probe"
        if key not in index or args.force:
            print(f"[{position}/{len(models)}] {model_id} source_probe ... ", end="", flush=True)
            result = run([str(probe_exe), str(model)], {}, args.timeout)
            faces_hash = write_gz(out / model_id / "faces.tsv.gz", result.pop("stdout"))
            probe_hash = write_gz(out / model_id / "faces.summary.txt.gz", result.pop("stderr"))
            index[key] = {
                "model_id": model_id, "path": str(model), "sha256": digest,
                "reading": "source_probe", "look_rev": look_rev, "truck_rev": truck,
                "cargo_lock": lock_hash, "schema": SCHEMA_VERSION,
                "faces_sha256_16": faces_hash, "summary_sha256_16": probe_hash,
                **result,
            }
            index_path.write_text(json.dumps(index, indent=1))
            print(f"{index[key]['outcome']} in {index[key]['seconds']}s")
        else:
            print(f"[{position}/{len(models)}] {model_id} source_probe: cached")

    print(f"\nindex: {index_path} ({len(index)} runs)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
