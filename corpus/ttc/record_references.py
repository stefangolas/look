"""Records reference geometry facts for the canonical corpus rows by running
the process-per-script door over each row (OCC baseline regime) and writing
corpus/ttc/reference/<id>.json. Run by the harness author on the recording
machine; the crate test re-derives the facts by running the same door and
compares within the recorded tolerances. Not part of the cargo suite.
"""
import json
import os
import subprocess
import sys
import tempfile

ROOT = os.path.dirname(os.path.abspath(__file__))
DOOR = os.path.join(ROOT, "door.py")
TREES = os.path.join(ROOT, "trees")

ROWS = [
    ("falcon_heavy/nozzle_assembly", "nozzle_assembly", "lib.merlin_common", "make_nozzle_assembly", []),
    ("falcon_heavy/chamber_assembly", "chamber_assembly", "lib.merlin_common", "make_chamber_assembly", []),
    ("falcon_heavy/thrust_structure", "thrust_structure", "lib.merlin_common", "make_thrust_structure", []),
    ("falcon_heavy/turbopump_assembly", "turbopump_assembly", "lib.merlin_common", "make_turbopump_assembly", []),
    ("falcon_heavy/gas_generator_assembly", "gas_generator_assembly", "lib.merlin_common", "make_gas_generator_assembly", []),
    ("falcon_heavy/turbine_exhaust_assembly", "turbine_exhaust_assembly", "lib.merlin_common", "make_turbine_exhaust_assembly", []),
    ("falcon_heavy/feed_assembly", "feed_assembly", "lib.merlin_common", "make_feed_assembly", []),
    ("falcon_heavy/harness_assembly", "harness_assembly", "lib.merlin_common", "make_harness_assembly", []),
    ("falcon_heavy/engine_prototype", "engine_prototype", "lib.falcon_common", "engine_prototype", []),
    ("falcon_heavy/mvac", "mvac", "lib.falcon_common", "make_mvac", []),
    ("falcon_heavy/second_stage", "second_stage", "lib.falcon_common", "make_second_stage", []),
    ("falcon_heavy/fairing", "fairing", "lib.falcon_common", "make_fairing", []),
    ("falcon_heavy/vehicle", "vehicle", "lib.falcon_common", "build_vehicle", []),
]


def main():
    out_dir = os.path.join(ROOT, "reference")
    os.makedirs(out_dir, exist_ok=True)
    for row_id, file_base, module, entry, args in ROWS:
        with tempfile.TemporaryDirectory() as tmp:
            stl = os.path.join(tmp, file_base + ".stl")
            family = row_id.split("/", 1)[0]
            tree = os.path.join(TREES, family, "src")
            proc = subprocess.run(
                [sys.executable, DOOR, tree, module, entry, json.dumps(args), stl],
                capture_output=True, text=True, check=True)
            record = json.loads(proc.stdout)
        if not record["ok"]:
            print(f"FAILED {row_id}: {record['error']}")
            return 1
        facts = record["facts"]
        ref = {
            "schema": "ttc_reference.v1",
            "row_id": row_id,
            "door_version": record["door_version"],
            "facts": {
                "solid_count": facts["solid_count"],
                "volume": facts["volume"],
                "bbox": facts["bbox"],
            },
            "tolerances": {
                "solid_count": "exact",
                "volume_rel": 1.0e-4,
                "volume_abs": 1.0e-6,
                "bbox_abs": 1.0e-3,
            },
        }
        path = os.path.join(out_dir, file_base + ".json")
        with open(path, "w", encoding="utf-8") as fh:
            json.dump(ref, fh, indent=2)
        print("recorded", row_id, "solids", facts["solid_count"])


if __name__ == "__main__":
    sys.exit(main())
