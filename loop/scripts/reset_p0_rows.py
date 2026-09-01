import json
from pathlib import Path

P = Path(__file__).resolve().parent.parent / 'PACKETS.jsonl'
lines = [l for l in P.read_text(encoding='utf-8-sig').splitlines() if l.strip()]
rows = [json.loads(l) for l in lines]
changed = 0
for r in rows:
    if r['id'] in ('BG-CK-P0-CRATE', 'BG-CK-P0-PREVALENCE') and r['status'] == 'RUNNING':
        r['status'] = 'READY'
        r['note'] = r.get('note', '') + ' | Post-reboot: workers killed on disk emergency (pagefile), WIP checkpointed at f190e8c/ab4a57f (wip/ refs); redispatching fresh per the one-worker-at-a-time pagefile rule.'
        changed += 1
with P.open('w', encoding='utf-8', newline='\n') as f:
    for r in rows:
        f.write(json.dumps(r, ensure_ascii=False) + '\n')
print('reset to READY:', changed)
