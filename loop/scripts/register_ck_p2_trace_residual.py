import json
from pathlib import Path

P = Path(__file__).resolve().parent.parent / 'PACKETS.jsonl'
rows = [json.loads(l) for l in P.read_text(encoding='utf-8-sig').splitlines() if l.strip()]

w2 = {
    "id": "BG-CK-P2-TRACE",
    "wave": "CK-P2",
    "class": "design",
    "needs": ["BG-CK-P2-CONTRACT"],
    "status": "RUNNING",
    "writes": [
        "vendor/truck/truck-certified/src/ssi_trace.rs",
        "vendor/truck/truck-certified/src/lib.rs",
        "vendor/truck/truck-certified/tests/ssi_trace.rs",
    ],
    "packet": "loop/packets/BG-CK-P2-TRACE.md",
    "slot": 2,
    "note": "Wave member W2 - certified branch tracing + the frozen both-certificate CoordinateSwitch rule, implemented against the landed shim types + ssi_fixtures via a solver-private BranchCertifier seam (integration adapters W1's evaluator). Runs in PARALLEL with W1 per owner direction (contracts frozen, write sets disjoint; lib.rs one-line textual conflict expected and resolved at integration). Registered at dispatch time.",
}
w3 = {
    "id": "BG-CK-P2-RESIDUAL",
    "wave": "CK-P2",
    "class": "mechanical",
    "needs": ["BG-CK-P2-CONTRACT", "BG-CK-P1-FLOOR"],
    "status": "READY",
    "writes": ["tests/certified_phase2_floor.rs", "docs/CERTIFIED_PHASE2_FLOOR.md"],
    "packet": "loop/packets/BG-CK-P2-RESIDUAL.md",
    "slot": 3,
    "note": "Wave member W3 - the Phase-2 gate measurement harness, FLOOR shape, wave-phase scope (single marked integration seam; the corpus walk reports integration_pending until the composed chain lands). Dispatches staggered after W2 to avoid cold-build RAM collision. Registered at dispatch time (row pre-written READY; flip at dispatch).",
}
for row in (w2, w3):
    assert row['id'] not in {r['id'] for r in rows}, f"row exists: {row['id']}"
    rows.append(row)
with P.open('w', encoding='utf-8', newline='\n') as f:
    for r in rows:
        f.write(json.dumps(r, ensure_ascii=False) + '\n')
print('registered BG-CK-P2-TRACE (slot 2, RUNNING) and BG-CK-P2-RESIDUAL (slot 3, READY->flip at dispatch)')
