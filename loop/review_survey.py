"""Brief the orchestrator on one or more SURVEY.json files.

Reviewing a survey is judgement and stays with the orchestrator. *Gathering the
evidence for that review* is mechanism, and until now it was done by writing
throwaway scripts at frontier cost -- three of them in the session that reviewed
BG-TOL-001-MESHALGO, one each for the census reconciliation, the low-confidence
rows and the multi-predicate lines. This is those scripts, kept.

The output is deliberately small. The point is not to print the survey; it is to
print only what a human must decide, so the reviewer spends context on
classifications rather than on re-deriving which rows matter.

Two rules are baked in, both paid for:

* **Do not re-check what V10 checked.** V10 already proves every (file, line,
  expression) resolves against the tree, and it does not miss -- re-verifying
  all 26 expressions of the first survey by hand found exactly nothing. This
  tool therefore spot-checks only a small sample as a tripwire against a
  regression in V10 itself, and spends its output budget on judgement instead.

* **Surface the shapes that have actually gone wrong**, rather than everything:
  mixed-class lines (a rewrite that migrates one predicate of two and silently
  deletes a guard), degree-2 quantities classified `model` (an area compared
  with a length margin), non-predicates given a `ctx.` rewrite (a `const` has no
  ctx), and rows the census and the survey disagree about.

    python loop/review_survey.py loop/surveys/*.json
"""
import argparse
import collections
import json
import os
import re
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
PRED = re.compile(r'\.near2?\(|\bso_small2?\(')
# Degree-2 in length: a cross product magnitude is twice a triangle's area, a
# 3x3 determinant of two displacements is a scalar triple product. Neither
# `model` nor `param` fits and `is_small_len` on one is wrong at Stage B.
DEG2 = re.compile(r'\.cross\(|\bdeterminant\(\)|\bmagnitude2\(\)')
# A value, not a comparison. A const initializer has no ctx in scope at all.
NONPRED = re.compile(r'^\s*(pub\s+)?(const|static)\s|^\s*use\s|\.max\(|\.min\(')


def src_line(f, ln):
    p = REPO_ROOT / f
    try:
        lines = p.read_text(encoding='utf-8', errors='replace').splitlines()
    except OSError:
        return None
    return lines[ln - 1] if 0 < ln <= len(lines) else None


def census_prod(fragment):
    """(file, line) of every production predicate the census sees."""
    res = subprocess.run(
        [sys.executable, str(REPO_ROOT / 'loop' / 'census_tol_sites.py'), fragment],
        capture_output=True, text=True, encoding='utf-8', errors='replace')
    return res.stdout


def review(path, sample):
    doc = json.loads(Path(path).read_text(encoding='utf-8'))
    sites = doc.get('sites', [])
    live = [s for s in sites if s.get('classification') in ('model', 'param')]
    excl = [s for s in sites if s.get('classification') == 'excluded']
    low = [s for s in sites if s.get('confidence') == 'low']

    print('=' * 78)
    print('%s  --  %s' % (doc.get('id', Path(path).stem), doc.get('crate', '?')))
    print('=' * 78)
    print('rows %d   live %d   excluded %d   confidence:low %d'
          % (len(sites), len(live), len(excl), len(low)))

    # contexts: distinct enclosing functions among live rows (what a shard budgets)
    fns = {(s['file'], s.get('symbol')) for s in live}
    print('distinct symbols among live rows: %d  (budget is contexts, verify with gen_packet --check)'
          % len(fns))

    flagged = collections.OrderedDict()

    def flag(key, s, note):
        flagged.setdefault(key, []).append((s, note))

    for s in sites:
        f, ln = s.get('file'), s.get('line')
        cls = s.get('classification')
        rw = s.get('proposed_rewrite') or ''
        raw = src_line(f, ln) if f and ln else None
        reason = (s.get('reason') or '').lower()

        if s.get('confidence') == 'low':
            flag('LOW CONFIDENCE -- read these first', s, s.get('reason', ''))

        if raw and cls in ('model', 'param'):
            n = len(PRED.findall(raw))
            if n > 1:
                migrated = len(re.findall(r'ctx\.', rw))
                if migrated < n:
                    flag('MIXED/MULTI PREDICATE -- rewrite may drop a guard', s,
                         '%d predicates on the line, rewrite migrates %d' % (n, migrated))

        # Deliberately noisy in one direction only. `v.cross(axis)` is degree 2
        # when both operands are displacements and degree 1 -- |v| sin t -- when
        # one is a UNIT vector, and nothing in the text says which; RevolutedCurve
        # normalizes its axis in the constructor, so `(p - origin).cross(axis)`
        # is a genuine length and `model` is right there. Static detection is not
        # possible, so this flags the shape and tells the reviewer what to check.
        # A false positive costs one glance; a false negative ships an area
        # compared against a length margin, which is correct at Stage A and
        # wrong at Stage B.
        if cls == 'model' and (DEG2.search(s.get('expression', '') or '')
                               or 'area' in reason or 'triple product' in reason
                               or 'length-squared' in reason):
            flag('DEGREE-2? -- check the operands', s,
                 'if BOTH cross operands are displacements this is an area (degree 2) and '
                 'belongs in BG-TOL-004; if one is a unit vector it is |v|sin(t), degree 1, '
                 'and `model` is correct')

        if raw and NONPRED.search(raw) and cls in ('model', 'param'):
            flag('NOT A PREDICATE? -- a value, not a comparison', s,
                 'looks like a const/use/max; a const initializer has no ctx')

        if cls in ('model', 'param') and not rw:
            flag('LIVE ROW WITH NO REWRITE', s, '')

    for key, rows in flagged.items():
        print('\n-- %s  (%d)' % (key, len(rows)))
        for s, note in rows:
            print('   %s:%s  %s  [%s]' % (s.get('file', '?').split('/src/')[-1],
                                          s.get('line'), s.get('symbol'), s.get('classification')))
            print('      expr: %s' % str(s.get('expression'))[:110])
            if s.get('proposed_rewrite'):
                print('      ->    %s' % str(s.get('proposed_rewrite'))[:110])
            if note:
                print('      note: %s' % note[:150])

    if not flagged:
        print('\nno rows tripped a heuristic. Still read every live classification --'
              '\nthe heuristics catch shapes that have gone wrong before, not new ones.')

    # V10 tripwire only. See module docstring: V10 already proves this for every
    # row, so a full re-check is wasted context; a small sample catches a
    # regression in V10 itself without paying for the rest.
    bad = []
    for s in live[:sample]:
        raw = src_line(s.get('file'), s.get('line'))
        if raw is None or (s.get('expression') or '').strip() not in raw:
            bad.append('%s:%s' % (s.get('file'), s.get('line')))
    checked = min(sample, len(live))
    print('\nV10 tripwire: %d/%d sampled live rows resolve%s'
          % (checked - len(bad), checked,
             '' if not bad else '  MISMATCH: ' + ', '.join(bad)))

    if doc.get('not_in_inventory'):
        print('\nnot_in_inventory (worker found what the census cannot see) -- %d:'
              % len(doc['not_in_inventory']))
        for n in doc['not_in_inventory']:
            print('   %s' % str(n)[:150])

    print('\nreason-text duplication: %d distinct reasons across %d rows'
          % (len({s.get('reason') for s in sites}), len(sites)))


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('surveys', nargs='+')
    ap.add_argument('--sample', type=int, default=5,
                    help='live rows to spot-check against the tree (V10 tripwire only)')
    args = ap.parse_args()
    try:
        sys.stdout.reconfigure(encoding='utf-8')
    except (AttributeError, OSError):
        pass
    for p in args.surveys:
        review(p, args.sample)
        print()


if __name__ == '__main__':
    main()
