"""Stage-by-stage probe: build_monocoque, then the door's exact export
pipeline (facts -> deflection -> export_stl), catching at each stage so the
exact throwing call is named."""
import sys
import types
import traceback
from pathlib import Path

SRC = Path(r"C:\Users\stefa\look\corpus\ttc\trees\f1\src")
sys.path.insert(0, str(SRC))

import build123d as real  # noqa: E402

cadgen = types.ModuleType("cadgen")
cadgen.build123d = real
cadgen.__path__ = []
sys.modules["cadgen"] = cadgen

import importlib  # noqa: E402

import math  # noqa: E402

mono = importlib.import_module("lib.monocoque")


def stage(name, fn):
    try:
        value = fn()
        print(f"[ok]   {name}: {value}")
        return value
    except Exception as exc:
        print(f"[FAIL] {name}: {type(exc).__name__}: {exc}")
        traceback.print_exc()
        return None


obj = stage("build_monocoque", mono.build_monocoque)
if obj is None:
    sys.exit(1)


def f_solids():
    return len(obj.solids())


def f_volume():
    return obj.volume


def f_bbox():
    bb = obj.bounding_box()
    return tuple(bb.min.to_tuple()), tuple(bb.max.to_tuple())


solids = stage("facts.solid_count", f_solids)
volume = stage("facts.volume", f_volume)
bb = stage("facts.bbox", f_bbox)
if bb:
    (mn, mx) = bb
    diag = math.sqrt(sum((mx[i] - mn[i]) ** 2 for i in range(3)))
    tolerance = max(diag * 2.0e-4, 0.01)
    print(f"     deflection tolerance: {tolerance}")

    def f_export():
        real.export_stl(obj, r"C:\Users\stefa\look\scratch\ref_stl\f1_monocoque_probe.stl",
                        tolerance=tolerance, angular_tolerance=0.5)
        return "stl written"

    stage("export_stl", f_export)
