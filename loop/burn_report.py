"""Measure what a packet cycle actually costs, instead of arguing about it.

The orchestrator has no counter on its own token use, so the efficiency work in
session 8 was justified with an ESTIMATE -- "roughly 2x" with a stated +-50%
uncertainty. An estimate nobody can check is exactly the kind of number STATE
warns about ("a number in STATE that no command reproduces is the default
outcome"), so this reports the proxies that ARE measurable and attributes them
per packet.

Measured here, all of it from artifacts already on disk:

  * worker cost and token use, per packet, from each slot's events.jsonl --
    exact, the provider reports it;
  * the payload the orchestrator must READ per packet: survey bytes, packet
    bytes, RESULT.json notes bytes, VERDICT.json bytes. This is the proxy for
    frontier input, and it is the thing the session-8 changes attack;
  * rows and reason-duplication per survey, which is what the scope rule and
    reason codes are meant to move.

What it deliberately does NOT do is guess at orchestrator tokens. The honest
claim is "the payload I must read fell from X to Y", which is checkable, not
"my burn halved", which is not.

    python loop/burn_report.py
"""
import collections
import json
import io
import os
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent


def toks(n):
    return n // 4


def worker_cost(events_path):
    cost = 0.0
    steps = 0
    billed = 0
    if not events_path.exists():
        return None
    for line in io.open(events_path, encoding='utf-8', errors='replace'):
        line = line.strip()
        if not line:
            continue
        try:
            d = json.loads(line)
        except Exception:
            continue
        if d.get('type') != 'step_finish':
            continue
        p = d.get('part', {}) or {}
        steps += 1
        cost += p.get('cost', 0) or 0
        t = p.get('tokens', {}) or {}
        c = t.get('cache', {}) or {}
        billed += ((t.get('input') or 0) + (t.get('output') or 0)
                   + (c.get('read') or 0) + (c.get('write') or 0))
    return {'steps': steps, 'cost': cost, 'billed': billed}


def survey_stats(path):
    raw = io.open(path, encoding='utf-8', errors='replace').read()
    try:
        d = json.loads(raw)
    except Exception:
        return None
    sites = d.get('sites', [])
    live = [s for s in sites if s.get('classification') in ('model', 'param')]
    reasons = [s.get('reason', '') or '' for s in sites]
    uniq = collections.Counter(reasons)
    dup = sum(len(r) * (c - 1) for r, c in uniq.items() if c > 1)
    return {
        'id': d.get('id', Path(path).stem), 'bytes': len(raw),
        'rows': len(sites), 'live': len(live),
        'per_live': len(raw) // max(len(live), 1),
        'reason_bytes': sum(len(r) for r in reasons), 'dup_bytes': dup,
        'coded': sum(1 for s in sites if s.get('reason_code')),
    }


def main():
    try:
        import sys
        sys.stdout.reconfigure(encoding='utf-8')
    except (AttributeError, OSError):
        pass

    print('READ PAYLOAD PER SURVEY  (the proxy for orchestrator input)')
    print('%-34s %8s %6s %6s %10s %9s %7s' %
          ('survey', 'bytes', 'rows', 'live', 'per-live', 'dup-bytes', 'coded'))
    surveys = sorted((REPO_ROOT / 'loop' / 'surveys').glob('*.json'))
    for sl in range(4):
        p = REPO_ROOT / 'loop' / 'slots' / str(sl) / 'wt' / 'SURVEY.json'
        if p.exists():
            surveys.append(p)
    for p in surveys:
        st = survey_stats(p)
        if st:
            print('%-34s %8d %6d %6d %10d %9d %7d' %
                  (st['id'][:34], st['bytes'], st['rows'], st['live'],
                   st['per_live'], st['dup_bytes'], st['coded']))

    print('\nPACKET SIZES  (43% of this was measured templatable)')
    for p in sorted((REPO_ROOT / 'loop' / 'packets').glob('*.md')):
        n = p.stat().st_size
        print('  %-46s %7d bytes  ~%5d tok' % (p.name[:46], n, toks(n)))

    print('\nWORKER COST PER SLOT  (exact -- the provider reports it)')
    total = 0.0
    for sl in range(4):
        ev = REPO_ROOT / 'loop' / 'slots' / str(sl) / 'events.jsonl'
        w = worker_cost(ev)
        if w and w['steps']:
            total += w['cost']
            print('  slot %d: %4d steps  $%.4f  billed %s tok (mostly cache reads)'
                  % (sl, w['steps'], w['cost'], f"{w['billed']:,}"))
    print('  total across live slots: $%.4f' % total)

    print('\nLEDGER FAULT ATTRIBUTION  (why did packets take round trips)')
    led = REPO_ROOT / 'loop' / 'LEDGER.jsonl'
    faults = collections.Counter()
    n = 0
    for line in io.open(led, encoding='utf-8', errors='replace'):
        line = line.strip()
        if not line:
            continue
        try:
            r = json.loads(line)
        except Exception:
            continue
        n += 1
        faults[r.get('fault', '(unrecorded)')] += 1
    for k, v in faults.most_common():
        print('  %-14s %d' % (k, v))
    if faults.get('(unrecorded)'):
        print('  -- rows predating --fault carry no attribution; that is expected,')
        print('     not a gap to backfill from memory.')


if __name__ == '__main__':
    main()
