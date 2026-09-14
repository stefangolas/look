import json

rows = []
for line in open('loop/LEDGER.jsonl', encoding='utf-8'):
    line = line.strip()
    if not line:
        continue
    try:
        rows.append(json.loads(line))
    except json.JSONDecodeError:
        continue

attributed = [r for r in rows if r.get('fault') and r['fault'] != 'NONE']
total = len(rows)
print(f"ledger rows: {total}, attributed faults: {len(attributed)}")
by_fault = {}
for r in attributed:
    by_fault[r['fault']] = by_fault.get(r['fault'], 0) + 1
print("fault mix:", by_fault)

# Size proxy: tests_added. Round-trip proxy: 'attempts' > 1 or fault_note mentions round trip.
sized = [(r.get('tests_added'), r) for r in rows if isinstance(r.get('tests_added'), int)]
small = [(t, r) for t, r in sized if t <= 6]
large = [(t, r) for t, r in sized if t >= 12]

def roundtrips(r):
    note = str(r.get('fault_note', ''))
    hits = note.lower().count('round trip')
    att = r.get('attempts')
    return max(hits, (att - 1) if isinstance(att, int) and att > 1 else 0)

def rate(group, label):
    if not group:
        return
    rt = sum(1 for _, r in group if roundtrips(r) > 0 or r.get('fault') not in (None, 'NONE'))
    print(f"{label}: n={len(group)}, with round trips/faults: {rt} ({100*rt/len(group):.0f}%)")

rate(small, 'small packets (tests_added <= 6)')
rate(large, 'large packets (tests_added >= 12)')
