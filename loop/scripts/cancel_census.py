import json
from pathlib import Path
P = Path('loop/PACKETS.jsonl')
rows = [json.loads(l) for l in P.read_text(encoding='utf-8-sig').splitlines() if l.strip()]
for r in rows:
    if r['id'] == 'BG-CK-SPLINE-CENSUS':
        r['status'] = 'BLOCKED'
        r['note'] += ' | CANCELLED BY OWNER (session 49, ~4h in): the census gate is waived - build the specced wave directly. The session-49 spec amendment already demoted corpus mass to an ordering device; recognizers will be booked on geometric-naturalness grounds if/when needed. The census may be re-run as a cheap measurement later; its WIP is archived at loop/slots/1/abandoned-20260902-142536.patch.'
with P.open('w', encoding='utf-8', newline='\n') as f:
    for r in rows:
        f.write(json.dumps(r, ensure_ascii=False) + '\n')
print('census row -> BLOCKED (owner-cancelled)')
