"""Emit a `class: survey` packet for a crate (or a subtree of one).

The survey packet is ~90% invariant across crates: the classification rules, the
four exclusion classes, the SURVEY.json schema, the stop conditions and the
forbidden list are contract-wide and were sharpened by the review of the first
survey. Only three things change per shard -- the id/crate/paths, the inventory
of where the sites are, and a size note. Writing those three by hand while
copying the other 90% is exactly the assembly work `class: survey` exists to
remove from the orchestrator, so it is removed here too.

The inventory is MEASURED, never typed: it comes from census_tol_sites.py's own
function list at generation time. A packet whose inventory is a claim about the
repo is a packet whose inventory rots -- that has already cost this loop two
round trips (GEOM-SPECIFIEDS' anchor counts were wrong when written).

    python loop/gen_survey.py --id BG-TOL-001-GEOM-NURBS --crates truck-geometry \
        --fragment nurbs --subtree src/nurbs

Multiple --fragment/--crates values are allowed for a combined shard.
"""
import argparse
import collections
import os
import re
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
TEMPLATE = REPO_ROOT / 'loop' / 'packets' / 'BG-TOL-001-MESHALGO-SURVEY.md'


def census(fragment):
    """(sites, functions, {file: [fn, ...]}) for one path fragment, measured."""
    res = subprocess.run(
        [sys.executable, str(REPO_ROOT / 'loop' / 'census_tol_sites.py'), fragment],
        capture_output=True, text=True, encoding='utf-8', errors='replace')
    out = res.stdout
    m = re.search(r"functions with a site under .*?: (\d+)", out)
    nfun = int(m.group(1)) if m else 0
    files = collections.OrderedDict()
    for line in out.splitlines():
        mm = re.match(r'\s{2}(\S+\.rs)\s+(\S+):(\d+)\s*$', line)
        if mm:
            files.setdefault(mm.group(1), []).append(mm.group(2))
    # site totals come from the per-crate table
    sites = 0
    for line in out.splitlines():
        row = re.match(r'^(truck-\S+)\s+(\d+)\s', line)
        if row and any(row.group(1) in f for f in [fragment]) :
            sites = int(row.group(2))
    return sites, nfun, files


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--id', required=True)
    ap.add_argument('--crates', required=True, help='comma-separated cargo package names')
    ap.add_argument('--fragment', required=True, help='comma-separated census path fragments')
    ap.add_argument('--subtree', default='', help='comma-separated read_allow subtrees, e.g. src/nurbs')
    ap.add_argument('--out', default='')
    args = ap.parse_args()

    try:
        sys.stdout.reconfigure(encoding='utf-8')
    except (AttributeError, OSError):
        pass

    crates = [c.strip() for c in args.crates.split(',') if c.strip()]
    frags = [f.strip() for f in args.fragment.split(',') if f.strip()]
    subs = [s.strip() for s in args.subtree.split(',') if s.strip()]

    total_fun = 0
    inventory = collections.OrderedDict()
    for f in frags:
        _, nfun, files = census(f)
        total_fun += nfun
        for k, v in files.items():
            inventory.setdefault(k, []).extend(v)
    total_sites = sum(len(v) for v in inventory.values())

    tpl = TEMPLATE.read_text(encoding='utf-8')

    # read_allow paths
    if subs:
        reads = ['  - vendor/truck/%s/%s/**' % (crates[0], s) for s in subs]
    else:
        reads = ['  - vendor/truck/%s/src/**' % c for c in crates]
    reads.append('  - vendor/truck/truck-base/src/tolerance.rs')

    front = """```yaml
id:          {id}
contract:    [BG-TOL-001]
class:       survey
crates:      [{crates}]
depends_on:  [BG-TOL-001-TYPE]
write_allow:
  - SURVEY.json
read_allow:
{reads}
tests_required: []
budget:      {{turns: 60, ctx_tokens: 150000}}
```""".format(id=args.id, crates=', '.join(crates), reads='\n'.join(reads))

    rows = '\n'.join(
        '| `%s` | %s |' % (f, ', '.join('`%s`' % x for x in dict.fromkeys(v)))
        for f, v in inventory.items())

    where = """## Where the sites are

**At least %d production predicates across %d functions.** This inventory is
generated from `loop/census_tol_sites.py` at the moment this packet was written.
It is your **starting point, not your answer** -- and it is known to be a floor,
not a count. Find every site yourself with

```
grep -nE '\\.near2?\\(|so_small2?\\(|TOLERANCE2?' <file>
```

| file | functions with a site |
|---|---|
%s

**The inventory has two known blind spots and finding what it missed is a
result, not an error.** Its pattern requires a word boundary before
`TOLERANCE`, so any constant named like `SOURCE_INCIDENCE_TOLERANCE` or
`RELATIVE_TOLERANCE` is invisible to it -- grep for `_TOLERANCE\\b` and
`\\bTOLERANCE_` yourself. And it matches the `TOLERANCE2` token but not a
written-out `TOLERANCE * TOLERANCE`. Report anything outside the table in
`not_in_inventory`.

A site at file scope sits outside any function -- a `const` or a `static`.
Report it with `"symbol": "<file scope>"` and say in `reason` what the constant
is used for, because that decides whether it is a predicate at all (see
exclusion 2).

Work through large files by grep hit, not front to back, and do not load a whole
large file into context at once.
""" % (total_sites, total_fun, rows)

    # splice: new front block, new "Where the sites are", everything else kept
    out = re.sub(r'```yaml\n.*?\n```', lambda _: front, tpl, count=1, flags=re.S)
    out = re.sub(r'## Where the sites are\n.*?(?=\n## Your output)', where, out,
                 count=1, flags=re.S)
    out = out.replace('BG-TOL-001-MESHALGO-SURVEY', args.id)
    out = out.replace('`truck-meshalgo` holds 30 tolerance predicates across 20 functions',
                      '`%s` holds at least %d tolerance predicates across %d functions'
                      % (' + '.join(crates), total_sites, total_fun))
    out = out.replace('truck-meshalgo', crates[0])

    dest = Path(args.out) if args.out else REPO_ROOT / 'loop' / 'packets' / (args.id + '.md')
    dest.write_text(out, encoding='utf-8', newline='\n')
    print('wrote %s  (%d sites, %d functions, %d files)'
          % (dest.relative_to(REPO_ROOT), total_sites, total_fun, len(inventory)))


if __name__ == '__main__':
    main()
