"""Register MONO-5-RAY-CLASSIFY + MONO-6-SWEPT-BOOLEANS rows (append; last-wins dedup)."""

import json
from pathlib import Path

ROWS = [
    {
        "id": "MONO-5-RAY-CLASSIFY",
        "status": "BLOCKED",
        "contract": ["MONO-5-RAY-CLASSIFY"],
        "class": "mechanical",
        "crates": ["truck123d"],
        "depends_on": ["MONO-4-TRIM-IDIOMS"],
        "write_allow": ["truck123d/src/bd_bridge.rs"],
        "packet": "loop/packets/MONO-5-RAY-CLASSIFY.md",
        "note": "registered 2026-09-10 (orchestrator): certified point-vs-spline-solid membership via ray x bicubic 1-D certified bracketing (Bernstein clipping) with signed parity + retry contract - wave-3 amendment 3 (named membership mechanism). Serializes after MONO-4 on bd_bridge.rs.",
        "registered": "2026-09-10",
    },
    {
        "id": "MONO-6-SWEPT-BOOLEANS",
        "status": "BLOCKED",
        "contract": ["MONO-6-SWEPT-BOOLEANS"],
        "class": "mechanical",
        "crates": ["truck123d"],
        "depends_on": ["MONO-5-RAY-CLASSIFY", "MONO-2-NSTATION-LOFT"],
        "write_allow": ["truck123d/src/bd_bridge.rs"],
        "packet": "loop/packets/MONO-6-SWEPT-BOOLEANS.md",
        "note": "registered 2026-09-10 (orchestrator): the wave-3 solver per the ACCEPTED frontier theory (contact covers + Bernstein exclusion + exact flux integration + area-based error accounting); all four review amendments incorporated as normative method items (facts-gate sufficiency lemmas incl. extremes-survive bbox, Lemma-3 separable-test termination, named membership via MONO-5, weights admission). Tub/cavity validation binding at 1e-4.",
        "registered": "2026-09-10",
    },
]

path = Path("loop/PACKETS.jsonl")
with path.open("a", encoding="utf-8") as f:
    for row in ROWS:
        f.write(json.dumps(row, ensure_ascii=False) + "\n")
print("registered", [r["id"] for r in ROWS])
