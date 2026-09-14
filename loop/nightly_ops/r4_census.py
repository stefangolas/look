"""TTC census R4 - re-run every enrolled MANIFEST row at the current HEAD.

Owner directive 2026-09-14: run R4, then registry reconcile. This is the R3
procedure (TTC-RECENSUS-F1-R3 packet method 3, PB-011C protocol verbatim):
one fresh python per row, serial, quiet machine, kernel engine
(door.py --engine truck). Adjudication happens AFTER collection - this script
only collects door records verbatim, so the verdict pass is re-runnable from
the saved records without re-paying the runs.

OCC reference recording is SKIPPED for R4: under the oracle policy change the
references are diagnostics and the R3 recordings stand (the sidepod rows'
OCC runs exceed any sane bound; re-recording gates nothing).

Run:
    python loop/nightly_ops/r4_census.py
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
import time

REPO = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
OUT = os.path.join(REPO, "loop", "nightly_ops", "r4")
MANIFEST = os.path.join(REPO, "corpus", "ttc", "MANIFEST.json")
DOOR = os.path.join(REPO, "corpus", "ttc", "door.py")
TIMEOUT_S = 900


def main() -> int:
    os.makedirs(OUT, exist_ok=True)
    with open(MANIFEST, encoding="utf-8") as fh:
        rows = json.load(fh)["rows"]
    print(f"R4 census: {len(rows)} rows, timeout {TIMEOUT_S}s/row", flush=True)

    failures = 0
    for i, row in enumerate(rows, 1):
        rid = row["id"].replace("/", "__")
        out_path = os.path.join(OUT, f"{rid}.json")
        if os.path.exists(out_path):
            print(f"[{i}/{len(rows)}] {row['id']}: already collected, skip", flush=True)
            continue
        stl = os.path.join(OUT, f"{rid}.stl")
        cmd = [
            sys.executable,
            DOOR,
            "--engine",
            "truck",
            row["tree"],
            row["module"],
            row["entry"],
            json.dumps(row.get("args", [])),
            stl,
        ]
        t0 = time.time()
        try:
            # door.py resolves the manifest's relative tree paths against the
            # process CWD, and the manifest paths are relative to corpus/ttc/
            # (the R3 protocol ran from there) - so the door runs with that cwd.
            proc = subprocess.run(
                cmd,
                cwd=os.path.join(REPO, "corpus", "ttc"),
                capture_output=True,
                text=True,
                timeout=TIMEOUT_S,
            )
            elapsed = round(time.time() - t0, 1)
            try:
                record = json.loads(proc.stdout)
            except json.JSONDecodeError:
                record = {
                    "schema": "r4_driver_error",
                    "ok": False,
                    "error": {
                        "kind": "DriverJSONDecode",
                        "typed": False,
                        "message": (proc.stdout or proc.stderr)[-2000:],
                    },
                }
        except subprocess.TimeoutExpired:
            elapsed = TIMEOUT_S
            record = {
                "schema": "r4_driver_error",
                "ok": False,
                "error": {
                    "kind": "DriverTimeout",
                    "typed": False,
                    "message": f"exceeded {TIMEOUT_S}s",
                },
            }
        record["r4_wall_seconds"] = elapsed
        record["r4_row_id"] = row["id"]
        with open(out_path, "w", encoding="utf-8") as fh:
            json.dump(record, fh, indent=2)
        state = "ok" if record.get("ok") else f"refused/dnf ({record['error'].get('kind')})"
        print(f"[{i}/{len(rows)}] {row['id']}: {state} in {elapsed}s", flush=True)
        if not record.get("ok"):
            failures += 1

    print(f"DONE: {len(rows) - failures} constructed, {failures} refused/dnf", flush=True)
    return 0


if __name__ == "__main__":
    sys.exit(main())
