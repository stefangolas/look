import json
from pathlib import Path

P = Path(__file__).resolve().parent.parent / 'PACKETS.jsonl'
lines = [l for l in P.read_text(encoding='utf-8-sig').splitlines() if l.strip()]
rows = [json.loads(l) for l in lines]
by_id = {r['id']: r for r in rows}

crate = by_id['BG-CK-P0-CRATE']
crate.update({
    'wave': 'CK-P0', 'class': 'mechanical', 'needs': [],
    'writes': [
        'vendor/truck/truck-certified/**',
        'vendor/truck/truck-meshalgo/src/tessellation/mod.rs',
        'vendor/truck/truck-meshalgo/src/tessellation/source_evidence.rs',
        'vendor/truck/truck-meshalgo/Cargo.toml',
        'Cargo.toml', 'Cargo.lock',
    ],
    'packet': 'loop/packets/BG-CK-P0-CRATE.md', 'slot': 0,
})

prev = by_id['BG-CK-P0-PREVALENCE']
prev.update({
    'wave': 'CK-P0', 'class': 'mechanical', 'needs': [],
    'writes': ['tests/certified_prevalence.rs', 'docs/CERTIFIED_PREVALENCE.md'],
    'packet': 'loop/packets/BG-CK-P0-PREVALENCE.md', 'slot': 1,
})

with P.open('w', encoding='utf-8', newline='\n') as f:
    for r in rows:
        f.write(json.dumps(r, ensure_ascii=False) + '\n')
print('updated', len(rows), 'rows; CK-P0 rows now carry the full scheduling schema')
