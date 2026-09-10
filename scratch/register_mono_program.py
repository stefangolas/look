"""Register MONO-1..4 rows in loop/PACKETS.jsonl (append; last-wins dedup)."""

import json
from datetime import date
from pathlib import Path

ROWS = [
    {
        "id": "MONO-1-DATA-ROWS",
        "status": "READY",
        "contract": ["MONO-1-DATA-ROWS"],
        "class": "mechanical",
        "crates": ["truck123d"],
        "depends_on": [],
        "write_allow": ["truck123d/src/binding.rs", "truck123d/src/marshal.rs"],
        "packet": "loop/packets/MONO-1-DATA-ROWS.md",
        "note": "registered 2026-09-10 (orchestrator): kernel-native bbox/obox/is_valid/n_faces data rows in the drop-in; closes the probe deaths (monocoque obox, engine_cover bbox, corner Face bbox). Convex-hull-certified bounds; no OCC.",
        "registered": "2026-09-10",
    },
    {
        "id": "MONO-2-NSTATION-LOFT",
        "status": "READY",
        "contract": ["MONO-2-NSTATION-LOFT"],
        "class": "mechanical",
        "crates": ["truck123d"],
        "depends_on": [],
        "write_allow": ["truck123d/src/bd_bridge.rs"],
        "packet": "loop/packets/MONO-2-NSTATION-LOFT.md",
        "note": "registered 2026-09-10 (orchestrator): certified N-station smooth loft under the PINNED OCCT ThruSections convention (docs/MONO_CLOSURE_BOOKING.md annex A: chord-length stations, exact section hit, degree N-1 for N<=9, C2 cubic knots-at-stations for N>=10). Generalizes BRIDGE-LOFT-FACTS two-station arm; per-patch volume_facts accounting.",
        "registered": "2026-09-10",
    },
    {
        "id": "MONO-3-BLADE-MEMBERS-MIRROR",
        "status": "BLOCKED",
        "contract": ["MONO-3-BLADE-MEMBERS-MIRROR"],
        "class": "mechanical",
        "crates": ["truck123d"],
        "depends_on": ["MONO-2-NSTATION-LOFT"],
        "write_allow": ["truck123d/src/bd_bridge.rs"],
        "packet": "loop/packets/MONO-3-BLADE-MEMBERS-MIRROR.md",
        "note": "registered 2026-09-10 (orchestrator): plate-section member carriers (swept_plate/blade_path/blade_member idioms) + exact-isometry mirror_y of kernel rows. Serializes after MONO-2 on the shared bd_bridge.rs write set.",
        "registered": "2026-09-10",
    },
    {
        "id": "MONO-4-TRIM-IDIOMS",
        "status": "BLOCKED",
        "contract": ["MONO-4-TRIM-IDIOMS"],
        "class": "mechanical",
        "crates": ["truck123d"],
        "depends_on": ["MONO-2-NSTATION-LOFT"],
        "write_allow": ["truck123d/src/bd_bridge.rs"],
        "packet": "loop/packets/MONO-4-TRIM-IDIOMS.md",
        "note": "registered 2026-09-10 (orchestrator): trace the corpus's actual trim idioms (rear_wing louvres, suspension _plate - reached by composition, zero trim() calls) then extend the landed TRIM-EXTRUDE-CTOR envelope to them. Serializes after MONO-2.",
        "registered": "2026-09-10",
    },
]

path = Path("loop/PACKETS.jsonl")
with path.open("a", encoding="utf-8") as f:
    for row in ROWS:
        f.write(json.dumps(row, ensure_ascii=False) + "\n")
print("registered", [r["id"] for r in ROWS])
