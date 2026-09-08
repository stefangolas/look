"""Repro probe for f1/monocoque: run build_monocoque with the cadgen alias
installed and print the FULL traceback so the failing operation is named."""
import sys
import types
import traceback
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
SRC = Path(r"C:\Users\stefa\look\corpus\ttc\trees\f1\src")
sys.path.insert(0, str(SRC))

import build123d as real  # noqa: E402

cadgen = types.ModuleType("cadgen")
cadgen.build123d = real
cadgen.__path__ = []
sys.modules["cadgen"] = cadgen

import importlib  # noqa: E402

mono = importlib.import_module("lib.monocoque")
try:
    result = mono.build_monocoque()
    print("BUILD OK:", type(result))
except Exception:
    traceback.print_exc()
    sys.exit(1)
