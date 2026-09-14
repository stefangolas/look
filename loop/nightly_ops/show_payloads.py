import json
import glob

for path in sorted(glob.glob('loop/nightly_ops/r4/*.json')):
    r = json.load(open(path, encoding='utf-8'))
    if r.get('ok') or not isinstance(r.get('error'), dict):
        continue
    e = r['error']
    if 'unsupported_envelope' not in str(e.get('message', '')):
        continue
    interesting = {k: v for k, v in e.items() if k not in ('kind', 'message', 'typed')}
    print(r.get('entry', path))
    print('   ', json.dumps(interesting)[:300])
