"""Parallel OCC reference-recording batch for the staged TTC rows.

Runs corpus/ttc/door.py per staged manifest row (fan-out, bounded width),
converts each door stdout record into the ttc_reference.v1 schema, and
logs wall times per row. Monocoque first (the calibration row).

Output:
- corpus/ttc/reference/<row_id>.json  (facts + standard tolerances)
- scratch/reference_batch_log.json    (per-row wall seconds, status)

Row status: ok | TIMEOUT | DOOR_ERROR. Existing references are skipped
unless --force. This is a MEASUREMENT: wall times are recorded, never
tuned, never published (BENCHMARKS doctrine).
"""
import json
import subprocess
import sys
import time
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

ROOT = Path(r"C:\Users\stefa\look")
CORPUS = ROOT / "corpus" / "ttc"
MANIFEST = json.loads((CORPUS / "MANIFEST.json").read_text(encoding="utf-8"))
REF = CORPUS / "reference"
LOG = ROOT / "scratch" / "reference_batch_log.json"
WIDTH = 4
TIMEOUT_S = 30 * 60
FIRST = "f1/monocoque"

TOLERANCES = {
    "solid_count": "exact",
    "volume_rel": 0.0001,
    "volume_abs": 1e-06,
    "bbox_abs": 0.001,
}


def rows():
    staged = [r for r in MANIFEST["rows"] if r["stage"] == "skipped"]
    staged.sort(key=lambda r: (r["id"] != FIRST, r["id"]))
    return staged


def run_row(row):
    rid = row["id"]
    ref_path = REF / (rid.split("/")[1] + ".json")
    if ref_path.exists():
        return {"row": rid, "status": "SKIP-EXISTS"}
    args_json = json.dumps(row.get("args", []))
    stl = ROOT / "scratch" / "ref_stl" / (rid.replace("/", "_") + ".stl")
    stl.parent.mkdir(parents=True, exist_ok=True)
    cmd = [sys.executable, str(CORPUS / "door.py"),
           str(ROOT / "corpus" / "ttc" / row["tree"]),
           row["module"], row["entry"], args_json, str(stl)]
    t0 = time.time()
    try:
        proc = subprocess.run(cmd, capture_output=True, text=True,
                              timeout=TIMEOUT_S, cwd=str(CORPUS))
    except subprocess.TimeoutExpired:
        return {"row": rid, "status": "TIMEOUT", "wall_s": round(time.time() - t0, 1)}
    wall = round(time.time() - t0, 1)
    if proc.returncode != 0:
        return {"row": rid, "status": "DOOR_ERROR", "wall_s": wall,
                "stderr_tail": ((proc.stderr or "") + (proc.stdout or ""))[-600:]}
    try:
        rec = json.loads(proc.stdout)
    except Exception as exc:  # noqa: BLE001 - record and continue
        return {"row": rid, "status": "PARSE_ERROR", "wall_s": wall,
                "detail": f"{type(exc).__name__}",
                "stdout_head": (proc.stdout or "")[:300]}
    out = {"schema": "ttc_reference.v1", "row_id": rid,
           "door_version": rec.get("door_version", "1"),
           "facts": rec["facts"], "tolerances": TOLERANCES}
    ref_path.write_text(json.dumps(out, indent=2) + "\n", encoding="utf-8")
    return {"row": rid, "status": "ok", "wall_s": wall,
            "solid_count": rec["facts"].get("solid_count"),
            "triangles": rec.get("triangle_count")}


def main():
    staged = rows()
    print(f"staged rows: {len(staged)} (first: {staged[0]['id']}), width {WIDTH}")
    results = []
    with ThreadPoolExecutor(max_workers=WIDTH) as ex:
        for res in ex.map(run_row, staged):
            print(json.dumps(res), flush=True)
            results.append(res)
    LOG.write_text(json.dumps(results, indent=2) + "\n", encoding="utf-8")
    ok = sum(1 for r in results if r["status"] == "ok")
    print(f"DONE: {ok}/{len(results)} ok; log at {LOG}")


if __name__ == "__main__":
    main()
