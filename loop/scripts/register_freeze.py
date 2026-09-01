import json
from pathlib import Path

P = Path(__file__).resolve().parent.parent / 'PACKETS.jsonl'
rows = [json.loads(l) for l in P.read_text(encoding='utf-8-sig').splitlines() if l.strip()]
if any(r['id'] == 'BG-CK-P0-FREEZE' for r in rows):
    raise SystemExit('BG-CK-P0-FREEZE already registered')
rows.append({
    'id': 'BG-CK-P0-FREEZE',
    'wave': 'CK-P0',
    'class': 'design',
    'needs': ['BG-CK-P0-CRATE'],
    'status': 'RUNNING',
    'writes': [
        'vendor/truck/truck-certified/src/contract.rs',
        'vendor/truck/truck-certified/src/lib.rs',
        'vendor/truck/truck-certified/tests/contract_freeze.rs',
    ],
    'packet': 'loop/packets/BG-CK-P0-FREEZE.md',
    'slot': 0,
    'note': 'Certified-kernel Phase 0 contract freeze (plan F1/F2/F3, all decisions pre-made in the packet, CG-000 shape): F1 WitnessEdge (pcurve pair + both surface handles + interval enclosures, no spline carrier - export view is a future type), F2 five-row BoundPolicy table (interval composition for normal/curvature/NURBS value; AUXILIARY ROOT ISOLATION for the curvature denominator well-definedness guard; Unfrozen refuses), F3 continuation-coordinate contract (square 3x3 Krawczyk only; deterministic lowest-index-on-ties coordinate selection by relative margin; turning-point switching = CoordinateSwitch carrying BOTH certificates; no-coordinate-certified refuses ConditioningBelowThreshold). Types as refusing signatures + contract-pinning tests; no numerical implementations. Registered at dispatch time.',
})
with P.open('w', encoding='utf-8', newline='\n') as f:
    for r in rows:
        f.write(json.dumps(r, ensure_ascii=False) + '\n')
print('registered BG-CK-P0-FREEZE (RUNNING, slot 0)')
