import json

rows = json.load(open('loop/nightly_ops/r4_verdicts.json', encoding='utf-8'))
ref = [r for r in rows if r['verdict'] == 'TYPED-REFUSAL']
print(len(ref), 'typed refusals:')
for r in ref:
    print(f"  {r['id']:34} case={r['case']}")
print()
dnf = [r for r in rows if r['verdict'] == 'DNF']
print('DNF:', [(r['id'], r['wall_s']) for r in dnf])
print()
ceil = [r for r in rows if r['verdict'] == 'CEILING']
print('CEILING:', [(r['id'], r['wall_s']) for r in ceil])
