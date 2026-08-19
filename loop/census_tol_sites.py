"""Census of BG-TOL-001 migration sites: production predicates vs noise.

The spec says 184 call sites. A raw grep counts doc-comment examples, proptest
strategy bounds and in-module test assertions alongside real predicates, and
those are not migration work at all -- a doc example is prose, a `#[strategy =
TOLERANCE..]` is a test input range, and a test's epsilon is the test's own
business. This splits them so a shard can be sized honestly.
"""
import os
import re
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import rustscan  # noqa: E402  -- sibling module, loop/ is not a package

ROOT = r'C:\Users\stefa\look\vendor\truck'
PAT = re.compile(r'\.near2?\(|\bso_small2?\(|\bTOLERANCE2?\b')
SQUARED = re.compile(r'\.near2\(|\bso_small2\(|\bTOLERANCE2\b')

rows = {}
fns = {}
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
            test_armed = False
            depth_at_test = None
            text = open(path, encoding='utf-8', errors='replace').read()
            # Line facts -- brace depth, block comments, and above all WHICH
            # FUNCTION a line is in -- come from loop/rustscan.py now. The
            # attribution used to be "the last `fn` seen", which never notices
            # that a function has closed, so a site after a nested helper's
            # closing brace was credited to the helper and two contexts read as
            # one. That is the defect that put a budget of 11 in
            # BG-TOL-001-MESHALGO's packet against a true 10.
            for info in rustscan.scan(text):
                lineno, line, s = info.lineno, info.raw, info.raw.strip()
                cur_fn, cur_fn_line = info.fn_name, info.fn_line
                # Block comments are dead text and are NOT migration work. This
                # cost BG-TOL-001-SHAPEOPS an amendment: the packet listed
                # fillet/mod.rs:615 as a live site and it sits inside a /* */
                # spanning lines 500-662, so the worker dutifully rewrote a
                # comment. Counting it as a site is how that reached the packet.
                # The test is now "the token survives comment and literal
                # stripping", which also catches a token in a trailing `//` and
                # one inside a string -- neither is a predicate either.
                raw_hit = PAT.search(line)
                if raw_hit and not PAT.search(info.code):
                    if s.startswith('//'):
                        doc += 1
                    else:
                        dead += 1
                    continue
                if re.match(r'#\[cfg\(test\)\]', s) or s.startswith('#[proptest'):
                    in_test = True
                    test_armed = False
                    depth_at_test = info.depth_before
                depth = info.depth_after
                # `test_armed` is the whole fix. The attribute and the `mod
                # tests {` it applies to are on different lines, so on the
                # attribute's own line depth is still equal to depth_at_test and
                # a bare `depth <= depth_at_test` closed the region immediately
                # -- every #[cfg(test)] module in the tree read as production.
                # That is how truck-modeling reported 11 production sites when
                # it has 5: six of them are proptest bodies below a
                # #[cfg(test)] at geom_impls.rs:117. Only start looking for the
                # closing brace once one has actually opened.
                if in_test and depth_at_test is not None:
                    if depth > depth_at_test:
                        test_armed = True
                    elif test_armed:
                        in_test = False
                        depth_at_test = None
                if not raw_hit:
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
                    # A Stage-A shard's unscaled_legacy budget is one context
                    # per FUNCTION containing at least one site, not one per
                    # site -- so the number a packet must declare is this set's
                    # size, and nothing printed it. BG-TOL-001-GEOM-SPECIFIEDS
                    # was written with an estimated budget of 12 against a true
                    # 19, the worker did the work correctly, hit GATE-4, and
                    # returned SPEC_GAP. The estimate was the defect; measure it.
                    fns.setdefault(crate, set()).add((path, cur_fn, cur_fn_line))
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
print('functions containing at least one production site -- this is the number a')
print('Stage-A shard must declare as unscaled_legacy_budget, one context each:')
for c in sorted(fns, key=lambda k: -len(fns[k])):
    print(f'  {c:24} {len(fns[c]):5}')
print()
# A shard's write set is a list of files, not a crate, so the crate total is the
# wrong number for a packet whose allowlist covers part of one. Pass any path
# fragment to get the count for just the files that match it.
if len(sys.argv) > 1:
    frag = sys.argv[1].replace('/', os.sep)
    hits = sorted({t for v in fns.values() for t in v if frag in t[0]})
    print(f'functions with a site under {sys.argv[1]!r}: {len(hits)}')
    for path, fn, ln in hits:
        print(f'  {os.path.relpath(path, ROOT).replace(os.sep, "/"):<52} {fn}:{ln}')
    print()
print('NOTE: `dead` counts only /* */ blocks. A module that is declared out --')
print('truck-shapeops/src/fillet/experiment.rs, via a commented `//mod experiment;`')
print('-- still counts as production here. Check the mod declaration before')
print('putting a file in a write set.')
