"""Text-to-cad corpus door (PB-008-TTC-HARNESS).

A plain, process-per-script door: the harness never reimplements cadgen's
daemon/store (spec boundary), it spawns one fresh python process per corpus
row and this module runs that row's geometry entry end to end.

Regime: the OCC baseline engine. The vendored corpus scripts are genuine
build123d scripts that import ``cadgen.build123d``; that module is a lazy,
transparent re-export of the genuine ``build123d`` package, so this door
answers the ``cadgen`` import with the installed build123d module (PEP 562
style) and never wraps or "improves" a build123d name. ``compound_from_instances``
(a cadgen composition helper the Falcon-Heavy tree uses to place repeated
engine instances) is answered with the equivalent plain build123d composition;
no cadgen daemon/store surface is reimplemented.

Outputs for one row:

* an STL file at the requested path (STL, never STEP — the TR-NRB-001
  boundary for swept/constructive parts holds on the corpus side too);
* a JSON record on stdout: door version, entry, type, build wall time
  (local evidence only, never published — BENCHMARKS doctrine), the geometry
  facts (solid count, volume, bounding box) and the STL triangle count.

The tessellation linear-deflection tolerance is scaled to the part's bounding
box diagonal so the same door serves a 0.5 m engine nozzle and a 70 m launch
vehicle (an absolute deflection that suits one makes the other absurd).

Usage:

    python door.py <tree_src_dir> <module> <entry> <args_json> <stl_path>

No cargo, no kernel, no GIL policy applies here: this file is a fixture-side
test harness, not production code. Determinism: identical ordered input yields
identical geometry facts; wall time is recorded but never asserted.
"""

from __future__ import annotations

import importlib
import json
import math
import os
import struct
import sys
import time
import types

DOOR_VERSION = "1"


def install_cadgen_alias() -> None:
    """Answer the corpus imports ``cadgen.build123d`` (and the placement helper
    ``cadgen.compound_from_instances``) with the genuine build123d package."""
    import build123d as real

    cadgen = types.ModuleType("cadgen")
    cadgen.build123d = real
    cadgen.__path__ = []
    sys.modules["cadgen"] = cadgen

    def _compound_from_instances(name, instances):
        objs = []
        for prototype, location, instance_name in instances:
            placed = prototype.moved(location)
            placed.label = instance_name
            objs.append(placed)
        return real.Compound(obj=objs, children=objs, label=name)

    cadgen.compound_from_instances = _compound_from_instances


def run_entry(tree_src: str, module: str, entry: str, args: list) -> object:
    """Import the corpus module and call its geometry entry, fresh per process."""
    sys.path.insert(0, tree_src)
    install_cadgen_alias()
    mod = importlib.import_module(module)
    fn = getattr(mod, entry)
    result = fn(*args) if args else fn()
    if isinstance(result, list):
        # A corpus entry may return the model's parts as a list of groups (the
        # top-level Falcon wrapper wraps them in one Compound); wrap here so the
        # door always exports one shape.
        import build123d as bd

        result = bd.Compound(obj=result, children=result, label=entry)
    return result


def geometry_facts(obj) -> dict:
    facts = {}
    try:
        facts["solid_count"] = len(obj.solids())
    except Exception as exc:  # noqa: BLE001 - door reports, never crashes
        facts["solid_count"] = f"err:{type(exc).__name__}"
    try:
        facts["volume"] = obj.volume
    except Exception as exc:  # noqa: BLE001
        facts["volume"] = f"err:{type(exc).__name__}"
    try:
        bb = obj.bounding_box()
        mn = tuple(bb.min.to_tuple())
        mx = tuple(bb.max.to_tuple())
        facts["bbox"] = [list(mn), list(mx)]
        facts["diag"] = math.sqrt(sum((mx[i] - mn[i]) ** 2 for i in range(3)))
    except Exception as exc:  # noqa: BLE001
        facts["bbox"] = f"err:{type(exc).__name__}"
    return facts


def triangle_count(stl_path: str) -> int:
    with open(stl_path, "rb") as fh:
        header = fh.read(80)
        count = fh.read(4)
    if len(header) < 80 or len(count) < 4:
        return -1
    return struct.unpack("<I", count)[0]


def main() -> int:
    tree_src = sys.argv[1]
    module = sys.argv[2]
    entry = sys.argv[3]
    args = json.loads(sys.argv[4])
    stl_path = sys.argv[5]

    t0 = time.time()
    try:
        obj = run_entry(tree_src, module, entry, args)
    except Exception as exc:  # noqa: BLE001 - the door reports typed failure
        record = {
            "schema": "ttc_door_run.v1",
            "door_version": DOOR_VERSION,
            "ok": False,
            "entry": entry,
            "error": {"kind": type(exc).__name__, "message": str(exc)},
        }
        json.dump(record, sys.stdout, indent=2)
        return 1

    built = time.time() - t0
    import build123d as bd

    facts = geometry_facts(obj)
    diag = float(facts.get("diag") or 1.0)
    tolerance = max(diag * 2.0e-4, 0.01)
    bd.export_stl(obj, stl_path, tolerance=tolerance, angular_tolerance=0.5)

    record = {
        "schema": "ttc_door_run.v1",
        "door_version": DOOR_VERSION,
        "ok": True,
        "entry": entry,
        "module": module,
        "type": type(obj).__name__,
        "build_seconds": round(built, 3),
        "facts": facts,
        "stl": {
            "path": os.path.basename(stl_path),
            "triangles": triangle_count(stl_path),
        },
    }
    json.dump(record, sys.stdout, indent=2)
    return 0


if __name__ == "__main__":
    sys.exit(main())
