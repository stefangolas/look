"""Run ONE corpus row with full process visibility (owner directive
2026-09-14): live call-stack sampling via py-spy (native frames included),
a 120 s performance ceiling with a faulthandler stack dump at the ceiling,
and the tree-kill the plain subprocess.run(timeout=) gets wrong on Windows
(leaked grandchildren hold the stdout pipe and hang the driver).

Usage:
    python loop/nightly_ops/row_run.py <row_id> [ceiling_seconds]

Outputs under loop/nightly_ops/row_runs/<safe_id>.*:
    record.json     the door record (verbatim)
    stackdump.txt   faulthandler dump if the row hit the ceiling
    profile.svg     py-spy flamegraph of the whole run (native + python)
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
import time

REPO = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
MANIFEST = os.path.join(REPO, "corpus", "ttc", "MANIFEST.json")
DOOR = os.path.join(REPO, "corpus", "ttc", "door.py")
PYSPY = r"C:\Users\stefa\AppData\Local\Programs\Python\Python313\Scripts\py-spy.exe"
OUT = os.path.join(REPO, "loop", "nightly_ops", "row_runs")


def find_row(row_id: str) -> dict:
    with open(MANIFEST, encoding="utf-8") as fh:
        for row in json.load(fh)["rows"]:
            if row["id"] == row_id:
                return row
    raise SystemExit(f"row not in manifest: {row_id}")


def main() -> int:
    row_id = sys.argv[1]
    ceiling = float(sys.argv[2]) if len(sys.argv) > 2 else 120.0
    row = find_row(row_id)
    safe = row_id.replace("/", "__")
    os.makedirs(OUT, exist_ok=True)
    stl = os.path.join(OUT, f"{safe}.stl")
    dump_path = os.path.join(OUT, f"{safe}.stackdump.txt")
    profile_path = os.path.join(OUT, f"{safe}.profile.svg")
    record_path = os.path.join(OUT, f"{safe}.record.json")

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
    env = dict(os.environ)
    env["LOOK_DOOR_STACKDUMP"] = dump_path
    env["LOOK_DOOR_TIMEOUT"] = str(ceiling)

    if os.path.exists(PYSPY):
        # Folded raw profile: machine-analyzable (greppable callee counts).
        # The .pyd's Rust frames collapse under PyInit_truck123d, but leaf
        # callees (ucrt/ntdll/OCP) still separate.
        profile_path = os.path.join(OUT, f"{safe}.profile.folded")
        cmd = [PYSPY, "record", "--native", "--rate", "20",
               "--format", "raw", "-o", profile_path, "--"] + cmd

    print(f"row {row_id} ceiling {ceiling}s profile {os.path.basename(profile_path)}")
    t0 = time.time()
    proc = subprocess.Popen(
        cmd,
        cwd=os.path.join(REPO, "corpus", "ttc"),
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        env=env,
    )
    try:
        stdout, stderr = proc.communicate(timeout=ceiling + 30)
    except subprocess.TimeoutExpired:
        # The faulthandler watchdog inside the door should have exited it at
        # the ceiling with a stack dump; this is the backstop kill. Tree-kill
        # so a leaked grandchild cannot hold the pipes (the R4 lighting hang).
        subprocess.run(["taskkill", "/PID", str(proc.pid), "/T", "/F"],
                       capture_output=True)
        stdout, stderr = proc.communicate()
    wall = round(time.time() - t0, 1)

    record = None
    try:
        record = json.loads(stdout)
    except json.JSONDecodeError:
        record = {"schema": "row_run_error", "ok": False,
                  "stdout_tail": (stdout or "")[-1500:],
                  "stderr_tail": (stderr or "")[-1500:]}
    record["row_run"] = {
        "row_id": row_id,
        "wall_seconds": wall,
        "ceiling_seconds": ceiling,
        "returncode": proc.returncode,
        "hit_ceiling": wall > ceiling,
    }
    with open(record_path, "w", encoding="utf-8") as fh:
        json.dump(record, fh, indent=2)

    hit = record["row_run"]["hit_ceiling"]
    print(f"wall {wall}s  ceiling-hit {hit}  rc {proc.returncode}")
    if hit and os.path.exists(dump_path):
        print("--- stack dump (at ceiling) ---")
        with open(dump_path, encoding="utf-8") as fh:
            print(fh.read()[:4000])
    if os.path.exists(profile_path):
        print(f"folded profile: {profile_path}")
    print(f"record: {record_path}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
