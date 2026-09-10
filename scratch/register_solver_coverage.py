"""Register SOLVER-SURVEY-A..D + SOLVER-CHECKER rows (append; last-wins dedup)."""

import json
from pathlib import Path

SURVEY_NOTE = "read-only rule-table extraction per docs/SOLVER_COVERAGE_SPEC.md (frozen RULES schema + semantic axes); fragment {} to loop/solver_coverage/fragments/; class survey, V10-validated at filing"

ROWS = [
    {"id": "SOLVER-SURVEY-A", "status": "READY", "contract": ["SOLVER-SURVEY-A"], "class": "survey", "crates": [],
     "depends_on": [], "write_allow": ["loop/solver_coverage/fragments/A.json"], "packet": "loop/packets/SOLVER-SURVEY-A.md",
     "note": SURVEY_NOTE.format("A") + " Scope: kernel surface-relation solvers (tangency/*, ssi*, selfint, contact, canal, pair_dispatch).",
     "registered": "2026-09-10"},
    {"id": "SOLVER-SURVEY-B", "status": "READY", "contract": ["SOLVER-SURVEY-B"], "class": "survey", "crates": [],
     "depends_on": [], "write_allow": ["loop/solver_coverage/fragments/B.json"], "packet": "loop/packets/SOLVER-SURVEY-B.md",
     "note": SURVEY_NOTE.format("B") + " Scope: certified construction + admission (construct/admission, bie/*, contact3, blend*, tangency gates/tsystem, formal/* with theory-only marking).",
     "registered": "2026-09-10"},
    {"id": "SOLVER-SURVEY-C", "status": "READY", "contract": ["SOLVER-SURVEY-C"], "class": "survey", "crates": [],
     "depends_on": [], "write_allow": ["loop/solver_coverage/fragments/C.json"], "packet": "loop/packets/SOLVER-SURVEY-C.md",
     "note": SURVEY_NOTE.format("C") + " Scope: boolean stage machinery (trimclip, assemble, atlas, sheet, engine, residuals_r89, promote, patch/certs/evidence/claims) with vertical-gap marking.",
     "registered": "2026-09-10"},
    {"id": "SOLVER-SURVEY-D", "status": "READY", "contract": ["SOLVER-SURVEY-D"], "class": "survey", "crates": [],
     "depends_on": [], "write_allow": ["loop/solver_coverage/fragments/D.json"], "packet": "loop/packets/SOLVER-SURVEY-D.md",
     "note": SURVEY_NOTE.format("D") + " Scope: the routing layer W_code (bd_bridge dispatch arms, binding/marshal/facade, door.py engine switch, capability-matrix cell cross-refs).",
     "registered": "2026-09-10"},
    {"id": "SOLVER-CHECKER", "status": "BLOCKED", "contract": ["SOLVER-CHECKER"], "class": "mechanical", "crates": [],
     "depends_on": ["SOLVER-SURVEY-A", "SOLVER-SURVEY-B", "SOLVER-SURVEY-C", "SOLVER-SURVEY-D"],
     "write_allow": ["loop/solver_coverage", "docs/SOLVER_COVERAGE_AUDIT.md"], "packet": "loop/packets/SOLVER-CHECKER.md",
     "note": "registered 2026-09-10 (orchestrator): AND-OR fixed-point checker (loop/solver_coverage/checker.py) + the first audit (docs/SOLVER_COVERAGE_AUDIT.md); binding demonstrations: retrodiction of the rank-deficient residual gap + deliberate-gap run; conflicts adjudicated by the orchestrator, never merged.",
     "registered": "2026-09-10"},
]

path = Path("loop/PACKETS.jsonl")
with path.open("a", encoding="utf-8") as f:
    for row in ROWS:
        f.write(json.dumps(row, ensure_ascii=False) + "\n")
print("registered", [r["id"] for r in ROWS])
