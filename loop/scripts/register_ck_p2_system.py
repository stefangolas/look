import json
from pathlib import Path

P = Path(__file__).resolve().parent.parent / 'PACKETS.jsonl'
rows = [json.loads(l) for l in P.read_text(encoding='utf-8-sig').splitlines() if l.strip()]
row = {
    "id": "BG-CK-P2-SYSTEM",
    "wave": "CK-P2",
    "class": "design",
    "needs": ["BG-CK-P2-CONTRACT"],
    "status": "RUNNING",
    "writes": [
        "vendor/truck/truck-certified/src/ssi.rs",
        "vendor/truck/truck-certified/src/lib.rs",
        "vendor/truck/truck-certified/tests/ssi_system.rs",
    ],
    "packet": "loop/packets/BG-CK-P2-SYSTEM.md",
    "slot": 1,
    "note": "Wave member W1 - the booking's SYSTEM and KRAWCZYK3 COLLAPSED (both book src/ssi.rs; wave mode forbids two workers on one new file; collapse invoked under booking decision 6's escape hatch, recorded in docs/CERTIFIED_INTERLEAVE_BUILD_SPEC.md). Forks the wave base a27edaa lineage (baaf93d at dispatch). LOCAL_GREEN is not DONE: no wave worker runs global gates; rows flip at the composed-HEAD verify. KRAWCZYK3 is intentionally never a separate row. Registered at dispatch time.",
}
existing = {r['id'] for r in rows}
assert 'BG-CK-P2-SYSTEM' not in existing, 'row already exists'
rows.append(row)
with P.open('w', encoding='utf-8', newline='\n') as f:
    for r in rows:
        f.write(json.dumps(r, ensure_ascii=False) + '\n')
print('registered BG-CK-P2-SYSTEM (slot 1), RUNNING')
