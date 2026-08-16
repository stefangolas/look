"""Census of BG-TOL-001 migration sites: production predicates vs noise.

The spec says 184 call sites. A raw grep counts doc-comment examples, proptest
strategy bounds and in-module test assertions alongside real predicates, and
those are not migration work at all -- a doc example is prose, a `#[strategy =
TOLERANCE..]` is a test input range, and a test's epsilon is the test's own
business. This splits them so a shard can be sized honestly.
"""
import os
import re

ROOT = r'C:\Users\stefa\look\vendor\truck'
PAT = re.compile(r'\.near2?\(|\bso_small2?\(|\bTOLERANCE2?\b')
SQUARED = re.compile(r'\.near2\(|\bso_small2\(|\bTOLERANCE2\b')

rows = {}
for crate in sorted(os.listdir(ROOT)):
    src = os.path.join(ROOT, crate, 'src')
    if not os.path.isdir(src):
        continue
    prod = doc = strat = test = squared = dead = 0
    for dirpath, _, files in os.walk(src):
        for fn in files:
            if not fn.endswith('.rs'):
                continue
            path = os.path.join(dirpath, fn)
            in_test = False
            depth_at_test = None
            depth = 0
            in_block = False
            for line in open(path, encoding='utf-8', errors='replace'):
                s = line.strip()
                # Block comments are dead text and are NOT migration work. This
                # cost BG-TOL-001-SHAPEOPS an amendment: the packet listed
                # fillet/mod.rs:615 as a live site and it sits inside a /* */
                # spanning lines 500-662, so the worker dutifully rewrote a
                # comment. Counting it as a site is how that reached the packet.
                was_in_block = in_block
                if '/*' in s and '*/' not in s.split('/*', 1)[1]:
                    in_block = True
                elif '*/' in s and was_in_block:
                    in_block = False
                    if PAT.search(line):
                        dead += 1
                    continue
                if was_in_block or in_block:
                    if PAT.search(line):
                        dead += 1
                    continue
                # crude but adequate brace tracking for #[cfg(test)] mod blocks
                if re.match(r'#\[cfg\(test\)\]', s) or s.startswith('#[proptest'):
                    in_test = True
                    depth_at_test = depth
                depth += line.count('{') - line.count('}')
                if in_test and depth_at_test is not None and depth <= depth_at_test:
                    in_test = False
                    depth_at_test = None
                if not PAT.search(line):
                    continue
                if SQUARED.search(line):
                    squared += 1
                    continue
                if s.startswith('///') or s.startswith('//!') or s.startswith('//'):
                    doc += 1
                elif s.startswith('#[strategy') or s.startswith('#['):
                    strat += 1
                elif in_test or fn == 'tests.rs':
                    test += 1
                else:
                    prod += 1
    if prod or doc or strat or test or squared or dead:
        rows[crate] = (prod, doc, strat, test, squared, dead)

print(f"{'crate':24} {'prod':>5} {'doc':>5} {'strat':>6} {'test':>5} {'sq':>4} {'dead':>5}")
tot = [0] * 6
for c, v in sorted(rows.items(), key=lambda kv: -kv[1][0]):
    print(f'{c:24} {v[0]:5} {v[1]:5} {v[2]:6} {v[3]:5} {v[4]:4} {v[5]:5}')
    tot = [a + b for a, b in zip(tot, v)]
print(f"{'TOTAL':24} {tot[0]:5} {tot[1]:5} {tot[2]:6} {tot[3]:5} {tot[4]:4} {tot[5]:5}")
print()
print(f'production first-order predicates to migrate: {tot[0]}')
print(f'excluded: {tot[1]} doc examples, {tot[2]} attributes, {tot[3]} in-src tests, '
      f'{tot[4]} squared-order, {tot[5]} inside block comments')
print()
print('NOTE: `dead` counts only /* */ blocks. A module that is declared out --')
print('truck-shapeops/src/fillet/experiment.rs, via a commented `//mod experiment;`')
print('-- still counts as production here. Check the mod declaration before')
print('putting a file in a write set.')
