import json
from pathlib import Path
P = Path(__file__).resolve().parent.parent / 'PACKETS.jsonl'
rows = [json.loads(l) for l in P.read_text(encoding='utf-8-sig').splitlines() if l.strip()]
for r in rows:
    if r['id'] == 'BG-CK-P0-CRATE':
        r['status'] = 'RUNNING'
with P.open('w', encoding='utf-8', newline='\n') as f:
    for r in rows:
        f.write(json.dumps(r, ensure_ascii=False) + '\n')
print('CRATE -> RUNNING')
