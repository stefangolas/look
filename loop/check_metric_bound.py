"""Which `model` sites can actually take `ctx.near_points`?

`ToleranceCtx::near_points<P>` is declared `where P: MetricSpace<Metric = f64>`.
Large parts of `truck-geometry` are generic over point types that do NOT supply
it -- `P: ControlPoint<f64> + Tolerance` is the common shape, and
`Homogeneous::Point` is only `EuclideanSpace<Scalar = Self::Scalar>`. A `model`
site inside such an impl is correctly classified and cannot be migrated: neither
`near_points` nor `.distance()` exists there, widening the bound is cross-crate,
and that is Stage B.

**This has now cost three shards.** BG-TOL-001-GEOM-NURBS pre-checked it and
found twelve, correcting a handoff that said ten. BG-TOL-001-SMALL pre-checked
it and found that two `search_parameter` functions with the same name and the
same shape take different rewrites. BG-TOL-001-GEOM-DECORATORS did NOT pre-check
it, asserted a rewrite that could not compile, and its worker returned SPEC_GAP
-- correctly, and at the cost of a round trip. The check is mechanical, so it
should not depend on remembering to do it.

    python loop/check_metric_bound.py loop/surveys/BG-TOL-001-XXX.json
    python loop/check_metric_bound.py --file vendor/truck/.../bspcurve.rs --line 474

Reports, per `model` row, the enclosing `impl` header and a verdict:

    MIGRATES   the impl supplies MetricSpace<Metric = f64>, or the type is concrete
    BLOCKED    generic point type with no MetricSpace -- defer GENERIC_BOUND
    CHECK      could not resolve the impl; read it yourself

`BLOCKED` is advice, not a gate. It reads the impl header textually and does not
run the type checker, so a bound reached through a trait's supertraits will read
as BLOCKED when it is fine. Treat a BLOCKED row as "open this file", which is
exactly what nobody did for DECORATORS.
"""
import argparse
import json
import re
import sys
from pathlib import Path

sys.stdout.reconfigure(encoding='utf-8')
REPO_ROOT = Path(__file__).resolve().parent.parent

# The question is never "is the impl generic" -- it is "is the POINT TYPE at
# this site a type parameter". `impl<C: ParametricCurve3D> ... for
# RevolutedCurve<C>` is generic in the curve and concrete in the point (the
# 3D-specialised traits fix Point = Point3), so it migrates. A first cut that
# flagged every generic impl reported 21 blocked rows in a crate that has 2.
METRIC = re.compile(r'MetricSpace\s*<\s*Metric\s*=\s*f64\s*>')

# Traits and types that pin the point to a concrete cgmath type, which brings
# its own MetricSpace impl.
CONCRETE = re.compile(
    r'\b(ParametricCurve3D|ParametricSurface3D|ParametricCurve2D|ParametricSurface2D'
    r'|Point1|Point2|Point3|Vector2|Vector3|Vector4)\b')

# Shapes that make the point itself a type parameter with only algebraic
# structure: ControlPoint gives arithmetic, Copy, Debug and Index and NOT
# MetricSpace; Homogeneous::Point is only EuclideanSpace<Scalar = Self::Scalar>.
GENERIC_POINT = re.compile(r'\bControlPoint\b|\bHomogeneous\b|\bEuclideanSpace\b')

# `impl<...>` or `fn name<...>`. The name matters: a first cut wrote
# `(?:pub\s+)?fn\s*<`, which cannot match `pub fn search_parameter<C>` because
# the NAME sits between `fn` and `<`. That read truck-geotrait's free function
# as non-generic and so as MIGRATES -- the one site already proven to need a
# different rewrite. A regex that silently fails to match reports the safe
# answer, which is the wrong way round for a check like this.
GENERIC_ITEM = re.compile(
    r'^(impl\s*<|(?:pub(?:\([^)]*\))?\s+)?(?:const\s+)?fn\s+\w+\s*<)')


def enclosing_item(path, line):
    """The nearest enclosing `impl ... {` or column-0 `fn ...` header.

    Free functions matter: truck-geotrait's `algo::search_parameter` is one, and
    its where-clause is the whole answer for that site.
    """
    try:
        src = path.read_text(encoding='utf-8', errors='replace').splitlines()
    except OSError:
        return None
    if line > len(src):
        return None
    for i in range(min(line, len(src)) - 1, -1, -1):
        s = src[i]
        if s.startswith('impl') or re.match(r'^(pub(\([^)]*\))?\s+)?(const\s+)?fn\s', s):
            out = []
            for j in range(i, min(i + 24, len(src))):
                out.append(src[j].rstrip())
                stripped = src[j].rstrip()
                if stripped.endswith('{') or stripped.endswith(';'):
                    break
            return '\n'.join(out)
    return None


def verdict(header):
    if header is None:
        return 'CHECK', 'no enclosing `impl` or `fn` found from this line upward'
    flat = ' '.join(header.split())
    if METRIC.search(flat):
        return 'MIGRATES', 'bounds the point type with MetricSpace<Metric = f64>'
    if not GENERIC_ITEM.search(flat):
        return 'MIGRATES', 'not generic; the point type is concrete'
    if CONCRETE.search(flat):
        return 'MIGRATES', 'generic, but the point type is pinned concrete'
    if GENERIC_POINT.search(flat):
        return 'BLOCKED', 'point type is a parameter with no MetricSpace<Metric = f64>'
    return 'CHECK', 'generic with an unrecognised point bound -- read it'


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('survey', nargs='?')
    ap.add_argument('--file')
    ap.add_argument('--line', type=int)
    args = ap.parse_args()

    if args.file:
        rows = [{'file': args.file, 'line': args.line, 'symbol': '?',
                 'classification': 'model'}]
    elif args.survey:
        doc = json.loads(Path(args.survey).read_text(encoding='utf-8'))
        rows = doc['sites'] if isinstance(doc, dict) else doc
    else:
        sys.exit('pass a survey json, or --file and --line')

    model = [r for r in rows if r.get('classification') == 'model']
    if not model:
        print('no `model` rows; nothing to check')
        return 0

    counts = {'MIGRATES': 0, 'BLOCKED': 0, 'CHECK': 0}
    print(f"{len(model)} `model` row(s)\n")
    for r in model:
        path = REPO_ROOT / r['file']
        head = enclosing_item(path, int(r['line']))
        v, why = verdict(head)
        counts[v] += 1
        name = r['file'].split('/')[-1]
        flag = '' if v == 'MIGRATES' else '  <<<'
        print(f"  {v:8} {name}:{r['line']:<6} {r.get('symbol', '?'):<28} {why}{flag}")
        if v != 'MIGRATES' and head:
            first = ' '.join(head.splitlines()[:6])
            print(f"           {' '.join(first.split())[:150]}")
    print()
    print('  '.join(f'{k} {v}' for k, v in counts.items()))
    if counts['BLOCKED'] or counts['CHECK']:
        print('\nEvery BLOCKED/CHECK row needs a human read before the packet is written.')
        print('A `model` site that cannot take its recipe is deferred')
        print('FIXME(BG-TOL-001, GENERIC_BOUND), not reclassified to make it compile.')
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
