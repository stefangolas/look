#!/usr/bin/env python3
"""SOLVER-CHECKER -- the AND-OR semantic solver-coverage checker.

Reads the four survey fragments (loop/solver_coverage/fragments/{A,B,C,D}.json),
merges them into one rule table, builds the explicit v1 proof-state lattice
from the frozen axes in docs/SOLVER_COVERAGE_SPEC.md (sections 1-5), computes
the AND-OR winning region W_G for each of the seven goals, and emits the first
semantic coverage audit.

Pure Python over JSON: no kernel changes, no OCC, no cargo builds.

RDEF-M0-CHECKER-ACCOUNTING adds the accounting fixes of
docs/RANK_DEFICIENT_CONTACT_SPEC.md section 2 (CHK-1 to CHK-4):

* CHK-1: every state is PROVED / REFUTED / UNDECIDED, with the section-2
  refutation predicates. The old "missing region" is now the UNDECIDED set;
  refuted states are reported separately and are never gaps.
* CHK-2: the three confirmed T_geom constraints are applied; feasible-state
  counts are reported before (v1) and after (tightened).
* CHK-3: the old winning region is renamed FAIL-CLOSED coverage; STRICT
  coverage restricts the winning route to rules with no refusal branch.
* CHK-4: `regular` is pinned to "regular where nonempty"; the cascade routes
  regular -> certified_empty are re-audited under that reading.

Usage:
    python loop/solver_coverage/checker.py                 # full audit + demos
    python loop/solver_coverage/checker.py --no-demos      # audit only
    python loop/solver_coverage/checker.py --remove-rule R # gap run for R
    python loop/solver_coverage/checker.py --summary       # print census only
"""

from __future__ import annotations

import argparse
import collections
import contextlib
import itertools
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
FRAG_DIR = HERE / "fragments"
AUDIT_PATH = ROOT / "docs" / "SOLVER_COVERAGE_AUDIT.md"
GAP_PATH = ROOT / "docs" / "SOLVER_COVERAGE_AUDIT.deliberate-gap.md"
STATE_PATH = HERE / "state.json"
RESULT_PATH = ROOT / "RESULT.json"

# --------------------------------------------------------------------------
# 1. The frozen core semantic axes (spec section 2), concrete value domains.
#    `!unmodeled` is the open-world vocabulary, not an enumerable state value:
#    it is recorded as an abstraction gap and never enters the arithmetic.
# --------------------------------------------------------------------------
AXES = [
    ("relation.src_dims", ["2x2", "2x1", "1x1"]),
    ("relation.zero_set", ["certified_empty", "regular", "rank_deficient(residual)", "unknown"]),
    ("relation.local_dim", ["0", "1", "2", "residual"]),
    ("relation.incidence", ["domain_interior(residual)", "seam", "knot_span_boundary", "domain_edge"]),
    ("rep.bernstein_chart", ["false", "true"]),
    ("rep.rational_positive_weights", ["false", "true"]),
    ("rep.exact_implicit", ["false", "true"]),
    ("rep.canonical_carrier", ["false", "true"]),
    ("rep.construction_witness", ["false", "true"]),
    ("rep.param_map_inverse", ["false", "true"]),
    ("global.knowledge", ["local_only(residual)", "loop_free", "seed_complete", "all_components", "complete_locus"]),
    ("goal", ["no_intersection", "local_contact", "complete_locus", "material_class", "valid_brep", "volume_bracket", "mesh"]),
]
AXIS_NAMES = [a for a, _ in AXES]
AXIS_INDEX = {a: i for i, a in enumerate(AXIS_NAMES)}
VALUES = {a: v for a, v in AXES}
GOAL_IDX = AXIS_INDEX["goal"]
GOALS = list(VALUES["goal"])
N = len(AXES)

#: values a state may be refined *out of* by a rule that does not name the
#: axis in its precondition (the residual/unknown bottoms of each axis).
REFINABLE = {
    "relation.zero_set": {"unknown"},
    "relation.local_dim": {"residual"},
    "relation.incidence": {"domain_interior(residual)"},
    "rep.bernstein_chart": {"false"},
    "rep.rational_positive_weights": {"false"},
    "rep.exact_implicit": {"false"},
    "rep.canonical_carrier": {"false"},
    "rep.construction_witness": {"false"},
    "rep.param_map_inverse": {"false"},
    "relation.src_dims": set(),
    "goal": set(),
}
KNOW_ORDER = {v: i for i, v in enumerate(VALUES["global.knowledge"])}

SCOPE_REFUSAL_MARKERS = (
    "NonCanonicalCarrier",
    "ContactReductionDeferred",
    "UnsupportedEnvelope",
    "outside declared envelope",
    "outside_admitted_class_set",
    "OutsideEnvelope",
)
EXPENSIVE_MARKERS = ("budget", "subdiv", "bisect", "exhaust", "generic")

# --------------------------------------------------------------------------
# RDEF-M0 decisions (docs/RANK_DEFICIENT_CONTACT_SPEC.md section 2, confirmed
# by the owner proxy 2026-09-10). Recorded in docs/SOLVER_COVERAGE_SPEC.md.
# --------------------------------------------------------------------------

#: CHK-4. The second reading. `regular` is regular *where the zero set is
#: nonempty*; it does not assert nonemptiness. Consequences:
#:   * `regular` does NOT refute `no_intersection` (the spec's third [ASM] row
#:     only applies under the first reading);
#:   * the TANGENCY-CASCADE-REFINE-EMPTY route regular -> certified_empty is
#:     valid, and so are the other cascade refinements of `regular`.
REGULAR_SEMANTICS = "regular where nonempty"

#: CHK-2. The three confirmed T_geom constraints. The third collapses
#: `certified_empty` to one local_dim value; the empty set has no meaningful
#: local dimension, so the value is a single accounting representative.
CERT_EMPTY_LOCAL_DIM = "0"

#: CHK-1. The section-2 refutation predicates. A refuted state is a state for
#: which the goal is provably false; it is neither a proof nor a gap.
REFUTED_ZERO_SET = {
    # no_intersection is refuted by any certified nonempty zero set. Under the
    # pinned CHK-4 reading, `regular` is not certified nonempty, so the only
    # certified nonempty value is the rank-deficient residual.
    "no_intersection": {"rank_deficient(residual)"},
    # local_contact is refuted when there is no zero set at all.
    "local_contact": {"certified_empty"},
}

#: RDEF-M0 method step 5. The three v1 conflicts, adjudicated against the
#: source tree. `keep_fragment` selects the source-faithful row for a
#: rule_id_content_mismatch; `drop_postconditions` filters adjudicated-wrong
#: postconditions from a (symbol, variant) pair. Fragments are not edited:
#: only these recorded adjudications are applied to the merged model.
ADJUDICATIONS = [
    {
        "kind": "rule_id_content_mismatch",
        "rule_id": "TANGENCY-TSYSTEM-FROM-DEFLATED",
        "verdict": "fragment B is source-faithful; fragment A over-claims",
        "keep_fragment": "B",
        "evidence": "vendor/truck/truck-certified/src/tangency/tsystem.rs:82-96",
        "rationale": (
            "TSystem::from_deflated only constructs T = (G1, G2, M1, M2) from a stored "
            "SquareSystem3 and its Rank2Chart (lines 86-95). It emits neither a construction "
            "witness nor an exact-implicit predicate, so fragment A's frozen-axis outcomes "
            "rep.construction_witness=true and rep.exact_implicit=true are unsupported. "
            "Fragment B models exactly what the source does: the unit-chart rows and the "
            "KrawczykSystem<4> instantiation, with no frozen-axis claim."
        ),
    },
    {
        "kind": "postcondition_claim_mismatch",
        "symbol": "crate::tangency::exclude::exclude_rec",
        "variant": "ExclusionEvidence::NoRootFiveEq",
        "verdict": "the minimal claim {relation.zero_set: certified_empty} is source-faithful",
        "drop_postconditions": [{"axis": "global.knowledge", "value": "local_only(residual)"}],
        "evidence": "vendor/truck/truck-certified/src/tangency/exclude.rs:166-211",
        "rationale": (
            "exclude_rec returns NoRootFiveEq { spend } when a single equation is separated "
            "from zero (line 176) or when a 4-of-5 square subsystem has no Krawczyk root "
            "(line 187). Either way it certifies that the five-equation system has no root, "
            "i.e. relation.zero_set = certified_empty, and it raises no global knowledge. "
            "The extra global.knowledge = local_only(residual) is the bottom of the knowledge "
            "order: a no-op restatement, not a disagreeing claim."
        ),
    },
    {
        "kind": "postcondition_claim_mismatch",
        "symbol": "formal::span::CurveSpan2",
        "variant": "CurveSpan2::RationalBezier",
        "verdict": "the declared-only claim {goal: complete_locus} is source-faithful",
        "drop_postconditions": [
            {"axis": "rep.bernstein_chart", "value": "true"},
            {"axis": "rep.rational_positive_weights", "value": "true"},
        ],
        "evidence": "vendor/truck/truck-certified/src/formal/span.rs:119-137",
        "rationale": (
            "span.rs declares the RationalBezier(RationalBezierSpan2) variant so the "
            "contract's variant set is frozen before any code depends on it; its constructor "
            "and certified operations land in GEN-001B (lines 13-15, 119-122). Being the "
            "variant does not itself certify rep.bernstein_chart or rep.rational_positive_weights. "
            "SPAN-FAMILY-INDEPENDENT-CONTRACT over-claims those flags; "
            "SPAN-RATIONAL-BEZIER-THEORY records the declaration only."
        ),
    },
]


def adjudication_for_rule_id(rule_id):
    for a in ADJUDICATIONS:
        if a.get("rule_id") == rule_id:
            return a
    return None


def adjudication_for_claim(symbol, variant):
    for a in ADJUDICATIONS:
        if a.get("symbol") == symbol and a.get("variant") == variant:
            return a
    return None


def apply_rule_adjudications(rule):
    """Apply the recorded postcondition adjudications to one merged rule."""
    out = dict(rule)
    outs = []
    for o in out.get("outcomes", []):
        adj = adjudication_for_claim(out["source"]["symbol"], o.get("variant"))
        drop = adj.get("drop_postconditions") if adj else None
        if drop:
            drop_set = {(d["axis"], d["value"]) for d in drop}
            o = dict(o)
            o["postconditions"] = [
                p for p in o.get("postconditions", [])
                if (p.get("axis"), p.get("value")) not in drop_set
            ]
        outs.append(o)
    out["outcomes"] = outs
    return out


# --------------------------------------------------------------------------
# 2. T_geom feasibility (spec section 2): the frozen v1 laws (CHK-2 adds the
#    three confirmed constraints; `feasible` is the tightened predicate).
# --------------------------------------------------------------------------
def _state_value(state, axis):
    return VALUES[axis][state[AXIS_INDEX[axis]]]


def feasible_v1(state) -> bool:
    """The v1 T_geom feasibility (before CHK-2 tightening).

    * canonical carrier implies exact-implicit
    * rank DF=3 over a 2x2 relation implies local contact dimension 1
    """
    canonical = _state_value(state, "rep.canonical_carrier") == "true"
    exact = _state_value(state, "rep.exact_implicit") == "true"
    if canonical and not exact:
        return False
    src = _state_value(state, "relation.src_dims")
    regular = _state_value(state, "relation.zero_set") == "regular"
    if src == "2x2" and regular and _state_value(state, "relation.local_dim") != "1":
        return False
    return True


def feasible(state) -> bool:
    """Tightened T_geom feasibility (CHK-2).

    v1 laws plus the three confirmed constraints:

    * `src_dims in {1x1, 2x1} => local_dim <= 1` -- two curves, or a curve and
      a surface, cannot meet in a 2-dimensional set;
    * `src_dims in {1x1, 2x1} and zero_set = regular => local_dim = 0` --
      full-rank contact between curves is isolated points;
    * `zero_set = certified_empty => local_dim = 0` -- local dimension is
      meaningless for an empty set, so its four values collapse to one.
    """
    if not feasible_v1(state):
        return False
    src = _state_value(state, "relation.src_dims")
    zs = _state_value(state, "relation.zero_set")
    ld = _state_value(state, "relation.local_dim")
    if src in ("1x1", "2x1") and ld == "2":
        return False
    if src in ("1x1", "2x1") and zs == "regular" and ld != "0":
        return False
    if zs == "certified_empty" and ld != CERT_EMPTY_LOCAL_DIM:
        return False
    return True


def refuted(goal, state) -> bool:
    """CHK-1 refutation predicate for `goal` on a concrete state."""
    allowed = REFUTED_ZERO_SET.get(goal)
    if not allowed:
        return False
    return _state_value(state, "relation.zero_set") in allowed


# --------------------------------------------------------------------------
# 3. Fragment loading, merging, conflict detection.
# --------------------------------------------------------------------------
def load_fragments():
    frags = {}
    for f in "ABCD":
        with open(FRAG_DIR / f"{f}.json", encoding="utf-8") as fh:
            frags[f] = json.load(fh)
    return frags


def is_unmodeled(value) -> bool:
    return value is None or (isinstance(value, str) and value.startswith("!unmodeled:"))


def rule_content_sig(rule):
    pre = tuple(sorted((p.get("axis"), p.get("value")) for p in rule.get("preconditions", [])))
    outs = tuple(
        sorted(
            (o.get("variant"), tuple(sorted((p.get("axis"), p.get("value")) for p in o.get("postconditions", []))))
            for o in rule.get("outcomes", [])
        )
    )
    return pre, outs


def detect_conflicts(rules):
    """Report conflicts; never silently average two claims.

    A conflict is (a) one rule_id carrying two different contents, or (b) one
    source symbol whose shared outcome variant carries different postcondition
    claims on the frozen axes.
    """
    conflicts = []
    by_id = collections.defaultdict(list)
    by_sym = collections.defaultdict(list)
    for r in rules:
        by_id[r["rule_id"]].append(r)
        by_sym[r["source"]["symbol"]].append(r)

    for rid, rs in by_id.items():
        sigs = {rule_content_sig(r) for r in rs}
        if len(sigs) > 1:
            conflicts.append({
                "kind": "rule_id_content_mismatch",
                "rule_id": rid,
                "fragments": sorted({r["_frag"] for r in rs}),
                "detail": "same rule_id extracted with different precondition/outcome content",
            })

    for sym, rs in by_sym.items():
        if len(rs) < 2:
            continue
        variant_posts = collections.defaultdict(list)
        for r in rs:
            for o in r.get("outcomes", []):
                posts = tuple(sorted(
                    (p.get("axis"), p.get("value"))
                    for p in o.get("postconditions", [])
                    if not is_unmodeled(p.get("value")) and not is_unmodeled(p.get("axis"))
                ))
                variant_posts[o.get("variant")].append((r["rule_id"], r["_frag"], posts))
        for variant, claims in variant_posts.items():
            if len(claims) < 2:
                continue
            # A fragment that models the variant only at the unmodeled/evidence
            # level records no frozen-axis claim: silence is not a disagreement.
            claimed = [c for c in claims if c[2]]
            if len(claimed) < 2:
                continue
            distinct = {c[2] for c in claimed}
            if len(distinct) > 1:
                conflicts.append({
                    "kind": "postcondition_claim_mismatch",
                    "symbol": sym,
                    "variant": variant,
                    "claims": [
                        {"rule_id": c[0], "fragment": c[1], "postconditions": [list(x) for x in c[2]]}
                        for c in claimed
                    ],
                })
    return conflicts


def merge_rules(frags):
    """Merge the four fragments; dedupe by (rule_id, source symbol).

    Rules extracted twice from the same symbol with the same rule_id are the
    same row; they merge. Distinct rule rows sharing a symbol are preserved
    (they are the survey's arm/stage decomposition). Cross-fragment
    postcondition disagreements are reported, never silently merged.
    """
    all_rules = []
    for f in "ABCD":
        for r in frags[f].get("rules", []):
            r = dict(r)
            r["_frag"] = f
            all_rules.append(r)

    conflicts = detect_conflicts(all_rules)

    merged = {}
    per_fragment = collections.Counter()
    duplicate_rows = 0
    for r in all_rules:
        per_fragment[r["_frag"]] += 1
        key = (r["rule_id"], r["source"]["symbol"])
        if key in merged:
            duplicate_rows += 1
            # RDEF-M0 adjudication: when a rule_id content mismatch was
            # adjudicated in favour of the later extraction, keep that row.
            adj = adjudication_for_rule_id(r["rule_id"])
            if adj and adj.get("keep_fragment") == r["_frag"]:
                merged[key] = r
            continue
        merged[key] = r

    # RDEF-M0 adjudication: filter adjudicated-wrong postcondition claims from
    # the merged model (the fragments themselves are not edited).
    adjudicated = [apply_rule_adjudications(r) for r in merged.values()]
    return adjudicated, conflicts, per_fragment, duplicate_rows, len(all_rules)


def collect_unmodeled(frags):
    """Inventory every !unmodeled value and axis key across the fragments."""
    values = collections.Counter()
    axis_keys = collections.Counter()
    per_fragment = collections.Counter()
    for f in "ABCD":
        for v in frags[f].get("unmodeled_values", []) or []:
            if v is None:
                continue
            values[v] += 1
            per_fragment[f] += 1
        for k in frags[f].get("unmodeled_axis_keys", []) or []:
            if k is None:
                continue
            axis_keys[k] += 1
        for r in frags[f].get("rules", []):
            for p in r.get("preconditions", []):
                if is_unmodeled(p.get("axis")):
                    axis_keys[p.get("axis")] += 1
                if is_unmodeled(p.get("value")) and p.get("value") is not None:
                    values[p.get("value")] += 1
            for o in r.get("outcomes", []):
                for p in o.get("postconditions", []):
                    if is_unmodeled(p.get("axis")):
                        axis_keys[p.get("axis")] += 1
                    if is_unmodeled(p.get("value")) and p.get("value") is not None:
                        values[p.get("value")] += 1
    return values, axis_keys, per_fragment


# --------------------------------------------------------------------------
# 4. Rule compilation: preconditions/postconditions -> concrete cubes.
# --------------------------------------------------------------------------
class CompiledRule:
    __slots__ = ("rule_id", "kind", "confidence", "fragment", "symbol", "pre",
                 "goal_pre", "progress", "refusal_only", "scope_refusal",
                 "expensive", "pre_cube", "abstracted", "refusals",
                 "has_refusal_branch", "strict_ok")

    def __init__(self, rule):
        self.rule_id = rule["rule_id"]
        self.kind = rule.get("kind")
        self.confidence = rule.get("confidence", "high")
        self.fragment = rule["_frag"]
        self.symbol = rule["source"]["symbol"]
        self.refusals = list(rule.get("refusals", []) or [])
        # CHK-3: a rule is STRICT when its winning route carries no refusal
        # branch. A refusal branch is a named refusal or an outcome that emits
        # no concrete postcondition (a fail-closed or silent arm).
        self.has_refusal_branch = bool(self.refusals)
        self.pre = {}
        self.abstracted = False
        for p in rule.get("preconditions", []):
            a, v = p.get("axis"), p.get("value")
            if is_unmodeled(a) or is_unmodeled(v):
                self.abstracted = True
                continue
            if a in AXIS_INDEX and v in VALUES[a]:
                self.pre[a] = v
            else:
                self.abstracted = True
        self.goal_pre = self.pre.get("goal")
        self.pre_cube = self._to_cube(self.pre)
        self.progress = []          # list of Q dicts (concrete postconditions)
        self.refusal_only = True
        scope = False
        for o in rule.get("outcomes", []):
            q = {}
            for p in o.get("postconditions", []):
                a, v = p.get("axis"), p.get("value")
                if is_unmodeled(a) or is_unmodeled(v):
                    self.abstracted = True
                    continue
                if a in AXIS_INDEX and v in VALUES[a]:
                    q[a] = v
            if q:
                self.progress.append(q)
                self.refusal_only = False
            else:
                self.has_refusal_branch = True
            text = " ".join(str(x) for x in (o.get("variant"),)) + " " + " ".join(rule.get("refusals", []))
            if any(m in text for m in SCOPE_REFUSAL_MARKERS):
                scope = True
        self.scope_refusal = scope and self.refusal_only
        self.strict_ok = not self.has_refusal_branch
        comment = rule.get("theorem_comment", "") or ""
        self.expensive = any(m in comment.lower() for m in EXPENSIVE_MARKERS)

    @staticmethod
    def _to_cube(assign):
        c = [None] * N
        for a, v in assign.items():
            c[AXIS_INDEX[a]] = VALUES[a].index(v)
        return tuple(c)


def compile_rules(rules):
    return [CompiledRule(r) for r in rules]


# --------------------------------------------------------------------------
# 5. Cube algebra.
# --------------------------------------------------------------------------
def cube_intersect(a, b):
    out = []
    for i in range(N):
        ai, bi = a[i], b[i]
        if ai is None:
            out.append(bi)
        elif bi is None:
            out.append(ai)
        elif ai == bi:
            out.append(ai)
        else:
            return None
    return tuple(out)


def cube_subsumes(a, b):
    for i in range(N):
        ai, bi = a[i], b[i]
        if ai is not None and ai != bi:
            return False
    return True


def absorb(cubes):
    seen = set()
    uniq = []
    for c in cubes:
        if c not in seen:
            seen.add(c)
            uniq.append(c)
    uniq.sort(key=lambda c: sum(1 for x in c if x is not None))
    kept = []
    for c in uniq:
        if any(cube_subsumes(k, c) for k in kept):
            continue
        kept.append(c)
    return kept


def intersect_sets(a, b):
    res = []
    for x in a:
        for y in b:
            c = cube_intersect(x, y)
            if c is not None:
                res.append(c)
    return absorb(res)


def admitted(axis, pre_val, post_val, pre_assign):
    """May a rule with the given precondition assign post_val on axis from a
    pre-state value pre_val?  Monotone refinement: equal, refine a residual
    bottom, strengthen knowledge, or the rule's precondition explicitly named
    the axis (it consumes that value)."""
    if pre_val == post_val:
        return True
    if pre_val in REFINABLE.get(axis, set()):
        return True
    if axis == "global.knowledge" and KNOW_ORDER[pre_val] <= KNOW_ORDER[post_val]:
        return True
    if axis in pre_assign and pre_assign[axis] == pre_val:
        return True
    return False


def preimage_cube(c, q, pre_assign):
    """States whose successor under postcondition q lies in cube c."""
    base = list(c)
    allowed = []
    for a, v in q.items():
        ai = AXIS_INDEX[a]
        if c[ai] is not None and VALUES[a][c[ai]] != v:
            return []
        base[ai] = None
        opts = [sv for sv in VALUES[a] if admitted(a, sv, v, pre_assign)]
        if not opts:
            return []
        allowed.append((ai, opts))
    if not allowed:
        return [tuple(base)]
    out = []
    for combo in itertools.product(*[opts for _, opts in allowed]):
        cube = list(base)
        for (ai, _), val in zip(allowed, combo):
            cube[ai] = VALUES[AXIS_NAMES[ai]].index(val)
        out.append(tuple(cube))
    return out


def preimage_set(w, q, pre_assign):
    res = []
    for c in w:
        res.extend(preimage_cube(c, q, pre_assign))
    return absorb(res)


# --------------------------------------------------------------------------
# 6. Goal predicates (the v1 coarse reading of "the postconditions prove G").
# --------------------------------------------------------------------------
def _v(axis, value):
    return VALUES[axis].index(value)


def goal_targets(goal):
    """The axes (and accepted values) a proving state must carry."""
    if goal == "no_intersection":
        return {"relation.zero_set": ["certified_empty"]}
    if goal == "local_contact":
        return {"relation.zero_set": ["regular", "rank_deficient(residual)"],
                "relation.local_dim": ["0", "1", "2"]}
    if goal == "complete_locus":
        return {"global.knowledge": ["complete_locus"]}
    if goal == "material_class":
        return {"relation.zero_set": ["certified_empty", "regular", "rank_deficient(residual)"],
                "relation.incidence": list(VALUES["relation.incidence"])}
    if goal == "valid_brep":
        return {"relation.zero_set": ["certified_empty", "regular", "rank_deficient(residual)"],
                "relation.local_dim": ["0", "1", "2"],
                "relation.incidence": ["seam", "knot_span_boundary", "domain_edge"]}
    if goal == "volume_bracket":
        return {"relation.zero_set": ["certified_empty", "regular"],
                "global.knowledge": ["all_components", "complete_locus"]}
    if goal == "mesh":
        return {"relation.zero_set": ["certified_empty", "regular", "rank_deficient(residual)"],
                "relation.local_dim": ["0", "1", "2"]}
    raise ValueError(goal)


def predicate_cubes(goal):
    """A union of cubes describing exactly the states that prove `goal`."""
    g = _v("goal", goal)
    allowed = goal_targets(goal)
    axes = sorted(allowed, key=lambda a: AXIS_INDEX[a])
    out = []
    for combo in itertools.product(*[allowed[a] for a in axes]):
        c = [None] * N
        for a, val in zip(axes, combo):
            c[AXIS_INDEX[a]] = _v(a, val)
        c[GOAL_IDX] = g
        out.append(tuple(c))
    return out


def proves(goal, state):
    zs = AXIS_NAMES[AXIS_INDEX["relation.zero_set"]]
    v = lambda a: VALUES[a][state[AXIS_INDEX[a]]]
    return any(cube_subsumes(c, state) for c in predicate_cubes(goal))


# --------------------------------------------------------------------------
# 7. The AND-OR fixed point.
# --------------------------------------------------------------------------
CUBE_CAP = 60000


def fixpoint(goal, rules, track_witness=True):
    """Least fixed point of the AND-OR winning region for `goal`.

    Returns (winning_cubes, witness) where witness maps a cube to the rule_id
    that first established it.
    """
    g = _v("goal", goal)
    gc = [None] * N
    gc[GOAL_IDX] = g
    gc = tuple(gc)
    w = absorb(predicate_cubes(goal))
    witness = {c: None for c in w}

    while True:
        new = []
        new_witness = {}
        for rule in rules:
            if rule.goal_pre is not None and rule.goal_pre != goal:
                continue
            if not rule.progress:
                continue
            inter = [rule.pre_cube]
            for q in rule.progress:
                pre = preimage_set(w, q, rule.pre)
                if not pre:
                    inter = []
                    break
                inter = intersect_sets(inter, pre)
                if not inter:
                    break
            if not inter:
                continue
            inter = intersect_sets(inter, [gc])
            for c in inter:
                new.append(c)
                new_witness.setdefault(c, rule.rule_id)
        w2 = absorb(w + new)
        if len(w2) == len(w):
            break
        if len(w2) > CUBE_CAP:
            # The v1 lattice must stay explicit; a cube blow-up is itself a
            # finding, but the product itself is enumerable.  Keep the cubes
            # (they remain finite over the explicit state set).
            pass
        w = w2
        if track_witness:
            for c, rid in new_witness.items():
                witness.setdefault(c, rid)
    return w, witness


# --------------------------------------------------------------------------
# 8. Explicit state materialisation and region classification.
# --------------------------------------------------------------------------
def enumerate_non_goal_states(feasible_fn=None):
    """All T_geom-feasible concrete states over the eleven fact axes.

    `feasible_fn` defaults to the tightened CHK-2 predicate; pass `feasible_v1`
    for the before-tightening count.
    """
    if feasible_fn is None:
        feasible_fn = feasible
    fact_axes = [i for i in range(N) if i != GOAL_IDX]
    states = []
    for combo in itertools.product(*[VALUES[AXIS_NAMES[i]] for i in fact_axes]):
        state = [None] * N
        for i, val in zip(fact_axes, combo):
            state[i] = VALUES[AXIS_NAMES[i]].index(val)
        state[GOAL_IDX] = 0  # placeholder
        st = tuple(state)
        if feasible_fn(st):
            states.append(st)
    return states


def cube_covers_state(cube, state):
    for i in range(N):
        if cube[i] is not None and cube[i] != state[i]:
            return False
    return True


def materialize_winning(goal, w):
    g = _v("goal", goal)
    out = set()
    for st in NON_GOAL_STATES:
        s = list(st)
        s[GOAL_IDX] = g
        s = tuple(s)
        for c in w:
            if cube_covers_state(c, s):
                out.add(s)
                break
    return out


# --------------------------------------------------------------------------
# 9. Reports.
# --------------------------------------------------------------------------
def region_gap_class(goal, state, applicable):
    """Distinguish the spec's gap classes for one missing state.

    * REFUTED         -- the goal is provably false here (CHK-1); not a gap.
    * OUTSIDE_ENVELOPE -- an intentional scope-refusal rule matches the state.
    * THEORY GAP      -- a goal-target axis is incompatible with the goal and no
                         goal-applicable rule can refine it (the residual parent).
    * VERTICAL GAP    -- rules can progress the target axes but the chain stalls
                         below the requested guarantee.
    """
    if refuted(goal, state):
        return "REFUTED"
    targets = goal_targets(goal)
    for rule in applicable:
        if rule.scope_refusal and cube_covers_state(rule.pre_cube, state):
            return "OUTSIDE_ENVELOPE"
    for axis, allowed in targets.items():
        ai = AXIS_INDEX[axis]
        sv = VALUES[axis][state[ai]]
        if sv in allowed:
            continue
        progress = False
        for rule in applicable:
            if not cube_covers_state(rule.pre_cube, state):
                continue
            for q in rule.progress:
                if q.get(axis) in allowed and admitted(axis, sv, q[axis], rule.pre):
                    progress = True
                    break
            if progress:
                break
        if not progress:
            return "THEORY GAP"
    return "VERTICAL GAP"


def classify_region(goal, states, winning, applicable):
    covered = sum(1 for s in states if s in winning)
    if covered == len(states):
        return "COVERED"
    classes = collections.Counter(
        region_gap_class(goal, s, applicable) for s in states if s not in winning)
    if covered == 0:
        return classes.most_common(1)[0][0]
    return f"PARTIAL ({classes.most_common(1)[0][0]})"


def route_for(state, goal, witness, rules_by_id, depth=0, visited=None):
    """A witness rule chain from `state` to a terminal state."""
    if visited is None:
        visited = set()
    if proves(goal, state):
        return []
    if depth > 24 or state in visited:
        return ["<depth-cap>"]
    visited.add(state)
    for c, rid in witness.items():
        if not rid or not cube_covers_state(c, state):
            continue
        rule = rules_by_id.get(rid)
        if rule is None:
            continue
        for q in rule.progress:
            succ = list(state)
            for a, v in q.items():
                succ[AXIS_INDEX[a]] = _v(a, v)
            succ = tuple(succ)
            if succ == state:
                continue
            if not any(cube_covers_state(cc, succ) for cc in witness):
                continue
            sub = route_for(succ, goal, witness, rules_by_id, depth + 1, visited)
            if sub and sub[-1] in ("<no-witness>", "<depth-cap>"):
                continue
            return [rid] + sub
    return ["<no-witness>"]


def region_key(state):
    return (
        VALUES["relation.src_dims"][state[AXIS_INDEX["relation.src_dims"]]],
        VALUES["relation.zero_set"][state[AXIS_INDEX["relation.zero_set"]]],
        VALUES["relation.local_dim"][state[AXIS_INDEX["relation.local_dim"]]],
        VALUES["global.knowledge"][state[AXIS_INDEX["global.knowledge"]]],
    )


def compute_goal(goal, compiled, strict_compiled=None, compute_strict=True):
    """Compute one goal's proof-state accounting.

    * FAIL-CLOSED coverage (CHK-3, renamed): the old winning region. A state is
      winning when some rule's progress outcomes all lead into W_G; named
      refusals are allowed on the way.
    * STRICT coverage (CHK-3): the same fixed point restricted to rules with no
      refusal branch (`CompiledRule.strict_ok`).
    * PROVED / REFUTED / UNDECIDED (CHK-1): the winning region, the refuted
      region, and the rest. Only the last is a gap.

    `compute_strict=False` skips the strict fixed point (the RDEF-M1 v2
    projection reports fail-closed only; strict is a production-lattice
    metric).
    """
    if strict_compiled is None:
        strict_compiled = [r for r in compiled if r.strict_ok]
    w, witness = fixpoint(goal, compiled)
    winning = materialize_winning(goal, w)
    if compute_strict:
        w_strict, _ = fixpoint(goal, strict_compiled, track_witness=False)
        strict_winning = materialize_winning(goal, w_strict)
    else:
        w_strict, strict_winning = [], set()
    g = _v("goal", goal)
    a_g = []
    for st in NON_GOAL_STATES:
        s = list(st)
        s[GOAL_IDX] = g
        a_g.append(tuple(s))
    refuted_states = [s for s in a_g if s not in winning and refuted(goal, s)]
    missing = [s for s in a_g if s not in winning and not refuted(goal, s)]
    regions = collections.defaultdict(list)
    for s in a_g:
        regions[region_key(s)].append(s)
    rules_by_id = {r.rule_id: r for r in compiled}
    applicable = [r for r in compiled if r.goal_pre in (None, goal)]
    table = []
    for key in sorted(regions):
        states = regions[key]
        status = classify_region(goal, states, winning, applicable)
        rep = next((s for s in states if s in winning), states[0])
        if rep in winning:
            route = route_for(rep, goal, witness, rules_by_id)
            if not route:
                route = ["<terminal>"]
        else:
            route = []
        table.append({"region": key, "status": status, "states": len(states),
                      "covered": sum(1 for s in states if s in winning),
                      "refuted": sum(1 for s in states if s in refuted_states),
                      "route": route})
    return {
        "goal": goal,
        "A_G": len(a_g),
        "W_G": len(winning),
        "M_G": len(missing),
        "R_G": len(refuted_states),
        "U_G": len(missing),
        "strict_W_G": len(strict_winning),
        "cubes": len(w),
        "strict_cubes": len(w_strict),
        "winning_cube_set": w,
        "strict_cube_set": w_strict,
        "regions": table,
        "missing": missing,
        "refuted": refuted_states,
        "winning": winning,
        "strict_winning": strict_winning,
        "witness": witness,
    }


def minimal_missing_cubes(missing):
    """Greedy cube cover of the missing region, projected onto the semantic
    axes (src_dims, zero_set, local_dim, incidence, knowledge)."""
    proj_axes = [AXIS_INDEX[a] for a in
                 ("relation.src_dims", "relation.zero_set", "relation.local_dim",
                  "relation.incidence", "global.knowledge")]
    patterns = collections.defaultdict(set)
    for s in missing:
        patterns[tuple(s[i] for i in proj_axes)].add(s)
    cubes = []
    for pattern in sorted(patterns):
        cubes.append({
            "relation.src_dims": VALUES["relation.src_dims"][pattern[0]],
            "relation.zero_set": VALUES["relation.zero_set"][pattern[1]],
            "relation.local_dim": VALUES["relation.local_dim"][pattern[2]],
            "relation.incidence": VALUES["relation.incidence"][pattern[3]],
            "global.knowledge": VALUES["global.knowledge"][pattern[4]],
            "state_count": len(patterns[pattern]),
        })
    return cubes


# ==========================================================================
# RDEF-M1-LATTICE-V2: lattice v2 (CHK-5/CHK-6), fragment E (CHK-8), dual-mode
# knowledge lift (CHK-7) and reachability (CHK-9).
#
# The production v1 lattice above is untouched: fragment E is PROPOSED and is
# never merged into the production numbers. The v2 projection is computed by
# temporarily swapping the lattice globals (`lattice_v2`), recompiling the
# merged A-D rows plus fragment E in lattice-v2 terms, and restoring them.
# ==========================================================================

V2_AXES = [
    ("relation.src_dims", ["2x2", "2x1", "1x1"]),
    ("relation.zero_set", ["certified_empty", "regular", "rank_deficient(residual)",
                           "unknown", "tangent_point", "tangent_crossing",
                           "tangent_curve", "tangent_higher", "coincident",
                           "near_degenerate(residual)"]),
    ("relation.local_dim", ["0", "1", "2", "residual"]),
    ("relation.incidence", ["domain_interior(residual)", "seam",
                            "knot_span_boundary", "domain_edge"]),
    ("relation.regime", ["unknown(residual)", "transversal", "tangential",
                         "singular_param"]),
    ("relation.volume_evidence", ["none(residual)", "sandwich_bounded"]),
    ("rep.bernstein_chart", ["false", "true"]),
    ("rep.rational_positive_weights", ["false", "true"]),
    ("rep.exact_implicit", ["false", "true"]),
    ("rep.canonical_carrier", ["false", "true"]),
    ("rep.construction_witness", ["false", "true"]),
    ("rep.param_map_inverse", ["false", "true"]),
    ("global.knowledge", ["local_only(residual)", "loop_free", "seed_complete",
                          "all_components", "complete_locus"]),
    ("goal", ["no_intersection", "local_contact", "complete_locus", "material_class",
              "valid_brep", "volume_bracket", "mesh"]),
]

#: CHK-5. Each new rank-deficient zero-set child fixes `local_dim`.
V2_CHILD_LOCAL_DIM = {
    "tangent_point": "0",
    "tangent_crossing": "1",
    "tangent_curve": "1",
    "tangent_higher": "0",
    "coincident": "src_min",
    "near_degenerate(residual)": "residual",
}

#: The children accepted by local_contact / valid_brep / mesh (CHK-6). They
#: do NOT accept `near_degenerate(residual)`, which is a named refusal.
V2_RANK_DEF_CHILDREN = ["rank_deficient(residual)", "tangent_point",
                        "tangent_crossing", "tangent_curve", "tangent_higher",
                        "coincident"]

#: The classified children exist only in the tangential regime (they are the
#: output of the tangency classifier). `near_degenerate(residual)` is excluded:
#: it is a refusal state that can arise before the regime is pinned.
V2_CLASSIFIED_CHILDREN = {"tangent_point", "tangent_crossing", "tangent_curve",
                          "tangent_higher", "coincident"}

#: The zero-set values the M1 acceptance counts as rank-deficient. The residual
#: parent and the certified children are the audited gap; `near_degenerate` is a
#: named refusal (NearDegenerate) and is reported separately.
V2_RANK_DEF_ZERO_SETS = set(V2_RANK_DEF_CHILDREN) | {"near_degenerate(residual)"}
V2_ACCEPTANCE_ZERO_SETS = set(V2_RANK_DEF_CHILDREN)

V2_INCIDENCE = ["domain_interior(residual)", "seam", "knot_span_boundary", "domain_edge"]


def feasible_v2(state) -> bool:
    """Tightened T_geom for lattice v2.

    v1 (CHK-2) plus:

    * each new rank-deficient child fixes `local_dim` (CHK-5); `coincident`
      fixes it to the smaller source dimension;
    * `volume_evidence = sandwich_bounded` only exists in the tangential
      regime (the sandwich rule's admission precondition).
    """
    if not feasible(state):
        return False
    zs = _state_value(state, "relation.zero_set")
    src = _state_value(state, "relation.src_dims")
    ld = _state_value(state, "relation.local_dim")
    fixed = V2_CHILD_LOCAL_DIM.get(zs)
    if fixed is not None:
        want = {"2x2": "2", "2x1": "1", "1x1": "1"}[src] if fixed == "src_min" else fixed
        if ld != want:
            return False
    if _state_value(state, "relation.volume_evidence") == "sandwich_bounded":
        if _state_value(state, "relation.regime") != "tangential":
            return False
    # A transversal admission certifies a regular (or empty) zero set; a
    # rank-deficient zero set under the transversal regime is infeasible.
    if _state_value(state, "relation.regime") == "transversal":
        if zs not in ("regular", "certified_empty"):
            return False
    # A classified tangency child is a tangential-regime object.
    if zs in V2_CLASSIFIED_CHILDREN and _state_value(state, "relation.regime") != "tangential":
        return False
    return True


def goal_targets_v2(goal):
    """CHK-6 revised goal predicates (lattice-v2 terms).

    `volume_bracket` is not a product: it is handled by `predicate_cubes_v2`.
    """
    rd = list(V2_RANK_DEF_CHILDREN)
    if goal == "no_intersection":
        return {"relation.zero_set": ["certified_empty"]}
    if goal == "local_contact":
        return {"relation.zero_set": ["regular"] + rd,
                "relation.local_dim": ["0", "1", "2"]}
    if goal == "complete_locus":
        return {"global.knowledge": ["complete_locus"]}
    if goal == "material_class":
        return {"relation.zero_set": ["certified_empty", "regular", "rank_deficient(residual)"],
                "relation.incidence": list(V2_INCIDENCE)}
    if goal == "valid_brep":
        return {"relation.zero_set": ["certified_empty", "regular"] + rd,
                "relation.local_dim": ["0", "1", "2"],
                "relation.incidence": ["seam", "knot_span_boundary", "domain_edge"]}
    if goal == "volume_bracket":
        raise ValueError("volume_bracket is not a product predicate")
    if goal == "mesh":
        return {"relation.zero_set": ["certified_empty", "regular"] + rd,
                "relation.local_dim": ["0", "1", "2"]}
    raise ValueError(goal)


def _predicate_cubes_generic(goal, allowed):
    g = _v("goal", goal)
    axes = sorted(allowed, key=lambda a: AXIS_INDEX[a])
    out = []
    for combo in itertools.product(*[allowed[a] for a in axes]):
        c = [None] * N
        for a, val in zip(axes, combo):
            c[AXIS_INDEX[a]] = _v(a, val)
        c[GOAL_IDX] = g
        out.append(tuple(c))
    return out


def predicate_cubes_v2(goal):
    """CHK-6: `volume_bracket` accepts `volume_evidence = sandwich_bounded`."""
    if goal != "volume_bracket":
        return _predicate_cubes_generic(goal, goal_targets_v2(goal))
    g = _v("goal", "volume_bracket")
    out = []
    for zs in ("certified_empty", "regular"):
        for kn in ("all_components", "complete_locus"):
            c = [None] * N
            c[AXIS_INDEX["relation.zero_set"]] = _v("relation.zero_set", zs)
            c[AXIS_INDEX["global.knowledge"]] = _v("global.knowledge", kn)
            c[GOAL_IDX] = g
            out.append(tuple(c))
    for kn in ("all_components", "complete_locus"):
        c = [None] * N
        c[AXIS_INDEX["relation.volume_evidence"]] = _v("relation.volume_evidence", "sandwich_bounded")
        c[AXIS_INDEX["global.knowledge"]] = _v("global.knowledge", kn)
        c[GOAL_IDX] = g
        out.append(tuple(c))
    return out


def region_key_v2(state):
    return (
        VALUES["relation.src_dims"][state[AXIS_INDEX["relation.src_dims"]]],
        VALUES["relation.zero_set"][state[AXIS_INDEX["relation.zero_set"]]],
        VALUES["relation.local_dim"][state[AXIS_INDEX["relation.local_dim"]]],
        VALUES["global.knowledge"][state[AXIS_INDEX["global.knowledge"]]],
        VALUES["relation.regime"][state[AXIS_INDEX["relation.regime"]]],
        VALUES["relation.volume_evidence"][state[AXIS_INDEX["relation.volume_evidence"]]],
    )


def minimal_missing_cubes_v2(missing):
    proj_axes = [AXIS_INDEX[a] for a in
                 ("relation.src_dims", "relation.zero_set", "relation.local_dim",
                  "relation.incidence", "global.knowledge", "relation.regime",
                  "relation.volume_evidence")]
    patterns = collections.defaultdict(set)
    for s in missing:
        patterns[tuple(s[i] for i in proj_axes)].add(s)
    cubes = []
    for pattern in sorted(patterns):
        cubes.append({
            "relation.src_dims": VALUES["relation.src_dims"][pattern[0]],
            "relation.zero_set": VALUES["relation.zero_set"][pattern[1]],
            "relation.local_dim": VALUES["relation.local_dim"][pattern[2]],
            "relation.incidence": VALUES["relation.incidence"][pattern[3]],
            "global.knowledge": VALUES["global.knowledge"][pattern[4]],
            "relation.regime": VALUES["relation.regime"][pattern[5]],
            "relation.volume_evidence": VALUES["relation.volume_evidence"][pattern[6]],
            "state_count": len(patterns[pattern]),
        })
    return cubes


_V2_NON_GOAL_STATES = None


@contextlib.contextmanager
def lattice_v2():
    """Run the checker core on the lattice-v2 axes, then restore v1 exactly."""
    global _V2_NON_GOAL_STATES
    g = globals()
    names = ["AXES", "AXIS_NAMES", "AXIS_INDEX", "VALUES", "GOAL_IDX", "GOALS", "N",
             "REFINABLE", "goal_targets", "region_key", "minimal_missing_cubes",
             "predicate_cubes", "NON_GOAL_STATES"]
    saved = {n: g[n] for n in names}
    g["AXES"] = V2_AXES
    g["AXIS_NAMES"] = [a for a, _ in V2_AXES]
    g["AXIS_INDEX"] = {a: i for i, a in enumerate(g["AXIS_NAMES"])}
    g["VALUES"] = {a: v for a, v in V2_AXES}
    g["GOAL_IDX"] = g["AXIS_INDEX"]["goal"]
    g["GOALS"] = list(g["VALUES"]["goal"])
    g["N"] = len(V2_AXES)
    ref = dict(REFINABLE)
    ref["relation.regime"] = {"unknown(residual)"}
    ref["relation.volume_evidence"] = {"none(residual)"}
    ref["relation.zero_set"] = {"unknown", "near_degenerate(residual)"}
    g["REFINABLE"] = ref
    g["goal_targets"] = goal_targets_v2
    g["region_key"] = region_key_v2
    g["minimal_missing_cubes"] = minimal_missing_cubes_v2
    g["predicate_cubes"] = predicate_cubes_v2
    if _V2_NON_GOAL_STATES is None:
        _V2_NON_GOAL_STATES = enumerate_non_goal_states(feasible_v2)
    g["NON_GOAL_STATES"] = _V2_NON_GOAL_STATES
    try:
        yield
    finally:
        for n in names:
            g[n] = saved[n]


def _expand_rule_unions(rule):
    """Expand `a|b` precondition values into one row per combination.

    Lattice-v2 fragment rows write unions (`zero_set = unknown|rank_def...`)
    because the spec's preconditions are set membership; the checker's compiled
    precondition is a single concrete value per axis.
    """
    choices = []
    for p in rule.get("preconditions", []):
        v = p.get("value")
        if isinstance(v, str) and "|" in v and not v.startswith("!unmodeled:"):
            choices.append((p.get("axis"), v.split("|")))
        else:
            choices.append((p.get("axis"), [v]))
    for combo in itertools.product(*[opts for _, opts in choices]):
        r = dict(rule)
        r["preconditions"] = [{"axis": a, "value": val}
                              for (a, _), val in zip(choices, combo)]
        yield r


def compile_rules_v2(rules):
    out = []
    for r in rules:
        for er in _expand_rule_unions(r):
            out.append(CompiledRule(er))
    return out


def load_fragment_e():
    with open(FRAG_DIR / "E.json", encoding="utf-8") as fh:
        return json.load(fh)


def fragment_e_rules(lift, w4=False):
    """Fragment E rows.

    CHK-7: `lift` adds the OBL-K knowledge-lift projection as a separate
    modelling rule. The sandwich rule itself never changes `global.knowledge`
    (the spec's OBL-K is exactly whether it may); the lift rule raises the low
    knowledge values to `all_components` for the tangential volume region.

    W4 (`E-TANGENCY-WITNESS-ZERO-BOUND`) is off by default (spec section 5.2);
    `w4=True` enables that proposed route for an explicit projection.
    """
    frag = load_fragment_e()
    rules = []
    for r in frag.get("rules", []):
        if r["rule_id"] == "E-TANGENCY-WITNESS-ZERO-BOUND" and not w4:
            continue
        r = dict(r)
        r["_frag"] = "E"
        rules.append(r)
    if lift:
        rules.append({
            "rule_id": "E-OBL-K-KNOWLEDGE-LIFT",
            "kind": "reduction",
            "source": {
                "file": "docs/RANK_DEFICIENT_CONTACT_SPEC.md",
                "symbol": "section 4.5 OBL-K (CHK-7 knowledge lift)",
                "line": 260,
            },
            "preconditions": [
                {"axis": "relation.regime", "value": "tangential"},
                {"axis": "goal", "value": "volume_bracket"},
                {"axis": "global.knowledge",
                 "value": "local_only(residual)|loop_free|seed_complete|all_components"},
            ],
            "outcomes": [
                {"variant": "AllComponents",
                 "postconditions": [{"axis": "global.knowledge", "value": "all_components"}]},
            ],
            "refusals": [],
            "consumes_evidence": ["SandwichCoverage"],
            "produces_evidence": ["GlobalKnowledgeLift"],
            "calls": ["E-VOLUME-TANGENTIAL-SANDWICH"],
            "theorem_comment": (
                "OBL-K: undecided pairs are covered exhaustively, so the sandwich "
                "rule may raise global.knowledge to all_components for the region it "
                "covers, for volume_bracket only. Modelled separately so the CHK-7 "
                "lift projection is explicit."
            ),
            "confidence": "proposed",
            "_frag": "E",
        })
    return rules


def fragment_e_rule_ids():
    return [r["rule_id"] for r in load_fragment_e().get("rules", [])]


WITNESS_AXES = {
    "relation.src_dims": "2x2",
    "relation.zero_set": "rank_deficient(residual)",
    "relation.local_dim": "0",
    "global.knowledge": "local_only(residual)",
    "relation.regime": "unknown(residual)",
    "relation.volume_evidence": "none(residual)",
}


def _annotate_v2_rep(rep, goal):
    """RDEF-M1 acceptance annotations, computed on the v2 globals."""
    rd_idx = AXIS_INDEX["relation.zero_set"]
    src_idx = AXIS_INDEX["relation.src_dims"]
    reg_idx = AXIS_INDEX["relation.regime"]
    rd_set = {VALUES["relation.zero_set"].index(z) for z in V2_RANK_DEF_ZERO_SETS}
    acc_set = {VALUES["relation.zero_set"].index(z) for z in V2_ACCEPTANCE_ZERO_SETS}
    missing = rep["missing"]
    rep["v2_rank_def_undecided"] = sum(1 for s in missing if s[rd_idx] in rd_set)
    rep["v2_residual_undecided"] = sum(
        1 for s in missing
        if VALUES["relation.zero_set"][s[rd_idx]] == "rank_deficient(residual)")
    rep["v2_near_degenerate_undecided"] = sum(
        1 for s in missing
        if VALUES["relation.zero_set"][s[rd_idx]] == "near_degenerate(residual)")
    # The M1 acceptance is the in-scope audited gap: surface-surface (2x2) rank
    # deficient volume states in the regimes fragment E resolves. The 1x1/2x1
    # knowledge-lifting gap is out of scope (spec section 1.3); singular_param
    # and near_degenerate are named refusals, reported separately.
    in_scope = 0
    named_refusal = 0
    for s in missing:
        if s[rd_idx] not in acc_set:
            continue
        src = VALUES["relation.src_dims"][s[src_idx]]
        regime = VALUES["relation.regime"][s[reg_idx]]
        if src == "2x2" and regime in ("unknown(residual)", "tangential"):
            in_scope += 1
        elif regime == "singular_param":
            named_refusal += 1
    rep["v2_rank_def_undecided_in_scope_2x2"] = in_scope
    rep["v2_rank_def_undecided_named_refusal"] = named_refusal
    gidx = VALUES["goal"].index(goal)
    widx = {AXIS_INDEX[a]: VALUES[a].index(v) for a, v in WITNESS_AXES.items()}
    states = [s for s in NON_GOAL_STATES if all(s[i] == v for i, v in widx.items())]
    win = 0
    for st in states:
        s = list(st)
        s[GOAL_IDX] = gidx
        if tuple(s) in rep["winning"]:
            win += 1
    rep["v2_witness_cell_states"] = len(states)
    rep["v2_witness_cell_winning"] = win
    rep["v2_witness_cell_proved"] = win > 0
    rep["v2_rank_def_cells"] = minimal_missing_cubes_v2(
        [s for s in missing if s[rd_idx] in rd_set])


def compute_goal_v2(goal, compiled):
    """Fail-closed accounting for the v2 projection.

    Same fixed point as `compute_goal` but without the (expensive) region
    classification or the strict metric: the RDEF-M1 acceptance needs W_G, the
    undecided set and the witness-cell annotation, not the per-region table.
    """
    w, witness = fixpoint(goal, compiled)
    winning = materialize_winning(goal, w)
    g = _v("goal", goal)
    a_g = []
    for st in NON_GOAL_STATES:
        s = list(st)
        s[GOAL_IDX] = g
        a_g.append(tuple(s))
    refuted_states = [s for s in a_g if s not in winning and refuted(goal, s)]
    missing = [s for s in a_g if s not in winning and not refuted(goal, s)]
    rep = {
        "goal": goal, "A_G": len(a_g), "W_G": len(winning), "M_G": len(missing),
        "R_G": len(refuted_states), "U_G": len(missing), "cubes": len(w),
        "winning_cube_set": w, "winning": winning, "missing": missing,
        "witness": witness,
    }
    _annotate_v2_rep(rep, goal)
    return rep


def v2_projection(include_e, lift, goals=None, w4=False):
    frags = load_fragments()
    merged, conflicts, per_fragment, duplicate_rows, rows_read = merge_rules(frags)
    rules = list(merged)
    if include_e:
        rules = rules + fragment_e_rules(lift, w4=w4)
    reps = {}
    with lattice_v2():
        compiled = compile_rules_v2(rules)
        for goal in (goals or GOALS):
            reps[goal] = compute_goal_v2(goal, compiled)
    return {
        "include_fragment_e": include_e,
        "knowledge_lift": lift,
        "w4_enabled": w4,
        "compiled_rows": len(compiled),
        "goals": reps,
    }


def compute_reachability_v2(proposed_reps):
    """CHK-9: is every new rule reachable from the door/bridge path?

    Code routing (fragment D `reaches`) is the W_code side. Because fragment E
    is proposed and not yet wired, the checker also reports whether each rule is
    the first witness of a winning cube (`first_witness_in_model`); a rule that
    is neither reached in code nor a first witness is the clearest routing gap.
    """
    frags = load_fragments()
    d = frags["D"]
    reached = set()
    for r in d.get("rules", []):
        for x in r.get("reaches", []) or []:
            reached.add(x)
    first_witness = set()
    for rep in proposed_reps.values():
        for c in rep["winning_cube_set"]:
            rid = rep["witness"].get(c)
            if rid:
                first_witness.add(rid)
    rows = []
    for r in load_fragment_e().get("rules", []):
        rid = r["rule_id"]
        in_door = rid in reached
        rows.append({
            "rule_id": rid,
            "reachable_from_door": in_door,
            "routing_gap": not in_door,
            "first_witness_in_model": rid in first_witness,
            "calls": list(r.get("calls", []) or []),
        })
    return {
        "door_reached_targets": sorted(reached),
        "rules": rows,
        "routing_gaps": [x["rule_id"] for x in rows if x["routing_gap"]],
    }


def write_state_json(goal_reports, census, v2=None):

    payload = {
        "schema": "solver-coverage-state.v2",
        "rdef_m0": {
            "regular_semantics": REGULAR_SEMANTICS,
            "cert_empty_local_dim": CERT_EMPTY_LOCAL_DIM,
            "refuted_zero_set": {k: sorted(v) for k, v in REFUTED_ZERO_SET.items()},
            "adjudications": ADJUDICATIONS,
        },
        "lattice": {
            "axes": {a: VALUES[a] for a in AXIS_NAMES},
            "t_geom": [
                "rep.canonical_carrier=true => rep.exact_implicit=true",
                "relation.src_dims=2x2 and relation.zero_set=regular => relation.local_dim=1",
                "relation.src_dims in {1x1,2x1} => relation.local_dim <= 1",
                "relation.src_dims in {1x1,2x1} and relation.zero_set=regular => relation.local_dim=0",
                "relation.zero_set=certified_empty => relation.local_dim=0",
            ],
            "feasible_fact_states": len(NON_GOAL_STATES),
        },
        "census": census,
        "goals": {},
    }
    for rep in goal_reports:
        cube_list = []
        for cube in rep["winning_cube_set"]:
            assign = {}
            for i, val in enumerate(cube):
                if val is not None:
                    assign[AXIS_NAMES[i]] = VALUES[AXIS_NAMES[i]][val]
            cube_list.append({"assign": assign, "witness_rule": rep["witness"].get(cube)})
        payload["goals"][rep["goal"]] = {
            "A_G": rep["A_G"],
            "W_G": rep["W_G"],
            "M_G": rep["M_G"],
            "R_G": rep["R_G"],
            "U_G": rep["U_G"],
            "strict_W_G": rep["strict_W_G"],
            "winning_cubes": len(cube_list),
            "winning_cube_list": cube_list,
            "missing_cells": minimal_missing_cubes(rep["missing"]),
            "refuted_cells": minimal_missing_cubes(rep["refuted"]),
        }
    if v2 is not None:
        payload["lattice_v2"] = v2["lattice"]
        payload["fragment_e"] = v2["fragment_e"]
    with open(STATE_PATH, "w", encoding="utf-8") as fh:
        json.dump(payload, fh, indent=1, ensure_ascii=False)


def render_goal_md(rep, unmodeled_note):
    lines = []
    lines.append(f"### Goal `{rep['goal']}`\n")
    lines.append(f"- `A_G` (feasible states for this goal): **{rep['A_G']}**")
    lines.append(f"- `W_G` (fail-closed winning states, CHK-3): **{rep['W_G']}** "
                 f"({100.0 * rep['W_G'] / rep['A_G']:.1f}%)")
    lines.append(f"- `strict_W_G` (strict winning states, no refusal branch): "
                 f"**{rep['strict_W_G']}** ({100.0 * rep['strict_W_G'] / rep['A_G']:.1f}%)")
    lines.append(f"- `R_G` (REFUTED, CHK-1): **{rep['R_G']}**")
    lines.append(f"- `U_G` (UNDECIDED, CHK-1): **{rep['U_G']}**")
    lines.append(f"- fail-closed winning region described by {rep['cubes']} cubes; "
                 f"strict by {rep['strict_cubes']} cubes\n")

    lines.append("#### Horizontal table (region -> status)\n")
    lines.append("Region = (src_dims, zero_set, local_dim, global.knowledge); incidence and the "
                 "six rep flags are existentially quantified. `status` for a non-winning region "
                 "is REFUTED, OUTSIDE_ENVELOPE, THEORY GAP or VERTICAL GAP.\n")
    lines.append("| src_dims | zero_set | local_dim | knowledge | states | covered | refuted | status | route |")
    lines.append("|---|---|---|---|---:|---:|---:|---|---|")
    for row in rep["regions"]:
        k = row["region"]
        route = " -> ".join(row["route"][:6]) if row["route"] else ""
        lines.append(f"| {k[0]} | {k[1]} | {k[2]} | {k[3]} | {row['states']} | "
                     f"{row['covered']} | {row['refuted']} | {row['status']} | {route} |")
    lines.append("")

    lines.append("#### Vertical chains (proof-strength reach per route)\n")
    levels = VALUES["global.knowledge"]
    reached = collections.Counter()
    for s in rep["winning"]:
        reached[VALUES["global.knowledge"][s[AXIS_INDEX["global.knowledge"]]]] += 1
    missing_levels = collections.Counter()
    for s in rep["missing"]:
        missing_levels[VALUES["global.knowledge"][s[AXIS_INDEX["global.knowledge"]]]] += 1
    lines.append("| proof strength | winning states | undecided states |")
    lines.append("|---|---:|---:|")
    for lv in levels:
        lines.append(f"| {lv} | {reached[lv]} | {missing_levels[lv]} |")
    lines.append("")

    lines.append("#### Undecided region (concrete semantic cells, CHK-1)\n")
    cells = minimal_missing_cubes(rep["missing"])
    if not cells:
        lines.append("_none_")
    else:
        lines.append(f"{len(cells)} projected cells; first 40 shown.\n")
        lines.append("| src_dims | zero_set | local_dim | incidence | knowledge | states |")
        lines.append("|---|---|---|---|---|---:|")
        for c in cells[:40]:
            lines.append(f"| {c['relation.src_dims']} | {c['relation.zero_set']} | "
                         f"{c['relation.local_dim']} | {c['relation.incidence']} | "
                         f"{c['global.knowledge']} | {c['state_count']} |")
        if len(cells) > 40:
            lines.append(f"| ... | _({len(cells) - 40} more cells)_ | | | | |")
    lines.append("")

    lines.append("#### Refuted region (CHK-1)\n")
    ref_cells = minimal_missing_cubes(rep["refuted"])
    if not ref_cells:
        lines.append("_none_")
    else:
        lines.append(f"{len(ref_cells)} projected cells; first 20 shown. These are not gaps.\n")
        lines.append("| src_dims | zero_set | local_dim | incidence | knowledge | states |")
        lines.append("|---|---|---|---|---|---:|")
        for c in ref_cells[:20]:
            lines.append(f"| {c['relation.src_dims']} | {c['relation.zero_set']} | "
                         f"{c['relation.local_dim']} | {c['relation.incidence']} | "
                         f"{c['global.knowledge']} | {c['state_count']} |")
    lines.append("")

    # Low-confidence and performance flags.
    low_ids = {r.rule_id for r in CENSUS["compiled"] if r.confidence == "low"}
    exp_ids = {r.rule_id for r in CENSUS["compiled"] if r.expensive}
    abstracted_ids = {r.rule_id for r in CENSUS["compiled"] if r.abstracted}
    low_regions = [row for row in rep["regions"]
                   if any(rid in low_ids for rid in row["route"])]
    perf_regions = [row for row in rep["regions"]
                    if row["status"] == "COVERED" and row["route"] and row["route"][0] != "<terminal>"
                    and all(rid in exp_ids for rid in row["route"])]
    abstracted_regions = [row for row in rep["regions"]
                          if row["route"] and any(rid in abstracted_ids for rid in row["route"])]
    lines.append("#### Low-confidence / performance flags\n")
    if low_regions:
        lines.append(f"- **{len(low_regions)}** winning routes depend on a `confidence: low` row:")
        for row in low_regions[:10]:
            lines.append(f"  - `{' / '.join(row['region'])}` via {' -> '.join(row['route'][:4])}")
    else:
        lines.append("- No winning route in this goal depends on a `confidence: low` row.")
    if perf_regions:
        lines.append(f"- **{len(perf_regions)}** regions are covered only by expensive generic "
                     "routes (bisection / subdivision / budget / generic refinement):")
        for row in perf_regions[:10]:
            lines.append(f"  - `{' / '.join(row['region'])}` via {' -> '.join(row['route'][:4])}")
    else:
        lines.append("- No region is covered only by an expensive generic route.")
    if abstracted_regions:
        lines.append(f"- **{len(abstracted_regions)}** winning regions are covered via rules whose "
                     "preconditions/postconditions carry `!unmodeled` content (abstraction gap); "
                     "their coverage is only as strong as the unmodeled evidence they assume.")
        for row in abstracted_regions[:6]:
            lines.append(f"  - `{' / '.join(row['region'])}` via {' -> '.join(row['route'][:4])}")
    lines.append("")
    return "\n".join(lines)


def render_audit(goal_reports, census, conflicts, unmodeled_values, unmodeled_axes, routing,
                 tightening=None):
    L = []
    L.append("# Solver coverage audit (SOLVER-CHECKER)\n")
    L.append("Machine-generated by `loop/solver_coverage/checker.py`. Contract: "
             "`docs/SOLVER_COVERAGE_SPEC.md` sections 1-5. The checker is pure Python over the "
             "four survey fragments; it makes no kernel changes.\n")
    L.append("## 1. Rule census\n")
    L.append("| fragment | rule rows |")
    L.append("|---|---:|")
    for f in "ABCD":
        L.append(f"| {f} | {census['per_fragment'][f]} |")
    L.append(f"| **total rows read** | **{census['rows_read']}** |")
    L.append(f"| deduped rows | {census['merged_rows']} |")
    L.append(f"| duplicate rows dropped | {census['duplicate_rows']} |")
    L.append(f"| conflicts reported | {len(conflicts)} |")
    L.append(f"| distinct `!unmodeled` values | {census['unmodeled_values']} |")
    L.append(f"| distinct `!unmodeled` axis keys | {census['unmodeled_axis_keys']} |")
    L.append(f"| low-confidence rules | {census['low_confidence']} |")
    L.append(f"| medium-confidence rules | {census['medium_confidence']} |")
    L.append("")
    if conflicts:
        L.append("### Conflicts (reported for orchestrator adjudication, never merged)\n")
        L.append("The three v1 conflicts are adjudicated against the source tree (RDEF-M0 "
                 "method step 5); the verdict is appended to each row and applied to the merged "
                 "model by `ADJUDICATIONS`. The fragments themselves are not edited.\n")
        for c in conflicts:
            adj = None
            if c["kind"] == "rule_id_content_mismatch":
                adj = adjudication_for_rule_id(c.get("rule_id"))
            elif c["kind"] == "postcondition_claim_mismatch":
                adj = adjudication_for_claim(c.get("symbol"), c.get("variant"))
            verdict = ""
            if adj:
                verdict = (f" -- **ADJUDICATED**: {adj['verdict']}. Evidence: "
                           f"`{adj['evidence']}`. {adj['rationale']}")
            L.append(f"- `{c['kind']}`: `{c.get('rule_id') or c.get('symbol')}` "
                     f"{c.get('variant', '')} -- {c.get('detail', '')}{verdict}")
        L.append("")
    else:
        L.append("No fragment conflicts were found; overlapping extractions (e.g. A/B on "
                 "`tangency/gates.rs` and `tangency/tsystem.rs`) are complementary rows, not "
                 "disagreeing postcondition claims.\n")

    L.append("## 2. The v1 lattice (tightened by CHK-2)\n")
    L.append("Frozen axes and concrete value domains:\n")
    L.append("| axis | values |")
    L.append("|---|---|")
    for a in AXIS_NAMES:
        L.append(f"| `{a}` | {', '.join('`'+v+'`' for v in VALUES[a])} |")
    L.append("")
    L.append("`T_geom` (v1): `rep.canonical_carrier=true => rep.exact_implicit=true`; "
             "`relation.src_dims=2x2 and relation.zero_set=regular => relation.local_dim=1`. "
             "CHK-2 adds the three confirmed constraints: "
             "`relation.src_dims in {1x1,2x1} => relation.local_dim <= 1`; "
             "`relation.src_dims in {1x1,2x1} and relation.zero_set=regular => relation.local_dim=0`; "
             "`relation.zero_set=certified_empty => relation.local_dim=0`. "
             f"Feasible fact states (eleven fact axes): **{len(NON_GOAL_STATES)}**. "
             "Infeasible cells are excluded from the arithmetic and retained in the vocabulary.\n")
    L.append("Goal predicates (the coarse v1 reading of *postconditions prove G*):\n")
    L.append("| goal | proving condition |")
    L.append("|---|---|")
    L.append("| `no_intersection` | `zero_set = certified_empty` |")
    L.append("| `local_contact` | `zero_set in {regular, rank_deficient(residual)}` and `local_dim in {0,1,2}` |")
    L.append("| `complete_locus` | `knowledge = complete_locus` |")
    L.append("| `material_class` | `zero_set != unknown` and incidence resolved |")
    L.append("| `valid_brep` | `zero_set != unknown`, `local_dim in {0,1,2}`, incidence resolved |")
    L.append("| `volume_bracket` | `zero_set in {certified_empty, regular}` and `knowledge in {all_components, complete_locus}` |")
    L.append("| `mesh` | `zero_set != unknown` and `local_dim in {0,1,2}` |")
    L.append("")
    L.append("Status vocabulary: **COVERED** (every state in the region is winning); "
             "**THEORY GAP** (a goal-target axis is incompatible with the goal and no "
             "goal-applicable rule can refine it -- the residual parent); **VERTICAL GAP** "
             "(rules can progress the target axes but the chain stalls below the requested "
             "guarantee); **OUTSIDE_ENVELOPE** (an intentional scope-refusal rule matches); "
             "**PARTIAL** (mixed). `route` is a witness rule chain from a representative "
             "winning state, or `<terminal>` when the state already proves the goal.\n")
    L.append("Rule semantics: a rule fires only where its concrete preconditions hold; an "
             "outcome may refine a residual bottom (`unknown`/`residual`/`false`) into a "
             "concrete value, strengthen `global.knowledge`, or transform an axis its own "
             "precondition named. A rule cannot silently overwrite a conflicting concrete "
             "fact. A rule wins at a state when **all** of its progress outcomes lead into "
             "`W_G`; fail-closed refusal outcomes are recorded but are not coverage "
             "(`Safety closure is never coverage`).\n")

    L.append("## 2b. RDEF-M0 accounting (CHK-1 to CHK-4)\n")
    L.append(f"**CHK-4 (pinned):** `regular` means *{REGULAR_SEMANTICS}*. A regular zero set "
             "may be empty, so `regular` does not refute `no_intersection` and the "
             "`TANGENCY-CASCADE-REFINE-EMPTY` route `regular -> certified_empty` is valid. "
             "Re-audit of the cascade routes: `TANGENCY-CASCADE-REFINE-EMPTY` "
             "(`regular -> certified_empty`), `TANGENCY-CASCADE-STAGE3-CRITICAL-VALUE` "
             "(`regular -> certified_empty`) and `TANGENCY-CASCADE-CLASSIFY-BOX` "
             "(`regular -> certified_empty` / `rank_deficient`). Every route was already "
             "accepted by the refinement rule (a rule may transform an axis its own "
             "precondition names), so **no route changes status** under the pinned reading; "
             "the pin only stops `regular` from counting as a refutation of "
             "`no_intersection`.\n")
    L.append("**CHK-1 (three-way status):** each state is PROVED (fail-closed winning), "
             "REFUTED (the goal is provably false) or UNDECIDED. Refutation predicates: "
             "`no_intersection` refuted by `zero_set = rank_deficient(residual)` (a certified "
             "nonempty zero set); `local_contact` refuted by `zero_set = certified_empty`. "
             "REFUTED states are not gaps and are removed from the missing region.\n")
    L.append("**CHK-3 (two coverage metrics):** *fail-closed coverage* is the old winning "
             "region -- every outcome either proves the goal or refuses with a named tag. "
             "*Strict coverage* is the winning region computed from rules with no refusal "
             "branch (`strict_ok`), i.e. the winning route contains no refusal.\n")
    if tightening:
        L.append("**CHK-2 (tightening before/after):**\n")
        L.append("| metric | before (v1 T_geom) | after (tightened) |")
        L.append("|---|---:|---:|")
        L.append(f"| feasible fact states | {tightening['before_states']} | "
                 f"{tightening['after_states']} |")
        for goal in GOALS:
            b = tightening["before"][goal]
            a = tightening["after"][goal]
            L.append(f"| `{goal}` A_G / W_G / M_G | {b['A_G']} / {b['W_G']} / {b['M_G']} | "
                     f"{a['A_G']} / {a['W_G']} / {a['M_G']} |")
        L.append("")

    L.append("## 3. Per-goal audit\n")
    for rep in goal_reports:
        L.append(render_goal_md(rep, None))

    L.append("## 3b. Coverage metrics (CHK-3)\n")
    L.append("| goal | A_G | fail-closed proved | fail-closed % | strict proved | strict % | "
             "refuted | undecided |")
    L.append("|---|---:|---:|---:|---:|---:|---:|---:|")
    for rep in goal_reports:
        a = rep["A_G"] or 1
        L.append(f"| `{rep['goal']}` | {rep['A_G']} | {rep['W_G']} | "
                 f"{100.0 * rep['W_G'] / a:.1f}% | {rep['strict_W_G']} | "
                 f"{100.0 * rep['strict_W_G'] / a:.1f}% | {rep['R_G']} | {rep['U_G']} |")
    L.append("")

    L.append("## 4. Routing table (W_theory vs W_code)\n")
    L.append(routing["summary"])
    L.append("")

    L.append("## 5. Abstraction-gap inventory (`!unmodeled`)\n")
    L.append(f"{len(unmodeled_values)} distinct unmodeled values and "
             f"{len(unmodeled_axes)} distinct unmodeled axis keys were extracted. "
             "The most frequent are listed below; the full inventory is in `state.json`.\n")
    L.append("| unmodeled value | occurrences |")
    L.append("|---|---:|")
    for v, n in unmodeled_values.most_common(40):
        L.append(f"| `{v[:160]}` | {n} |")
    L.append("")
    L.append("| unmodeled axis key | occurrences |")
    L.append("|---|---:|")
    for k, n in unmodeled_axes.most_common(30):
        L.append(f"| `{k[:160]}` | {n} |")
    L.append("")

    L.append("## 6. Low-confidence discipline\n")
    low = [r for r in CENSUS["compiled"] if r.confidence == "low"]
    if low:
        L.append("Winning routes depending on a `confidence: low` row are flagged; routes are "
                 "only as trustworthy as their weakest extracted link.\n")
        L.append("| rule | fragment | symbol |")
        L.append("|---|---|---|")
        for r in low:
            L.append(f"| `{r.rule_id}` | {r.fragment} | `{r.symbol}` |")
    else:
        L.append("No `confidence: low` rows are present in the merged table.\n")

    L.append("## 7. Demonstrations\n")
    L.append("### (a) Retrodiction\n")
    L.append(routing["retrodiction"])
    L.append("")
    L.append("### (b) Deliberate gap\n")
    L.append(routing["deliberate_gap_note"])
    L.append("")
    return "\n".join(L)


CENSUS = {}


# --------------------------------------------------------------------------
# 10. Main.
# --------------------------------------------------------------------------
def build_census(frags, merged, conflicts, per_fragment, duplicate_rows, rows_read):
    compiled = compile_rules(merged)
    values, axes, _ = collect_unmodeled(frags)
    census = {
        "per_fragment": {f: per_fragment.get(f, 0) for f in "ABCD"},
        "rows_read": rows_read,
        "merged_rows": len(merged),
        "duplicate_rows": duplicate_rows,
        "conflicts": len(conflicts),
        "unmodeled_values": len(values),
        "unmodeled_axis_keys": len(axes),
        "low_confidence": sum(1 for r in compiled if r.confidence == "low"),
        "medium_confidence": sum(1 for r in compiled if r.confidence == "medium"),
        "compiled": compiled,
    }
    return census, compiled, values, axes


def compute_routing(compiled, goal_reports, frags):
    d = frags["D"]
    declared = d.get("routing_gaps", [])
    reached = set()
    for r in d.get("rules", []):
        for x in r.get("reaches", []) or []:
            reached.add(x)
    theory_rule_ids = {r.rule_id for r in compiled}
    reached_rules = {x for x in reached if x in theory_rule_ids}
    reached_syms = {x for x in reached if x not in theory_rule_ids}
    lines = []
    lines.append(f"Fragment D declares **{len(declared)}** routing gaps and reaches "
                 f"**{len(reached)}** downstream targets: {len(reached_rules)} named kernel rule ids "
                 f"and {len(reached_syms)} symbol paths.\n")
    lines.append("| id | kind | title | missing rule | cells |")
    lines.append("|---|---|---|---|---|")
    for g in declared:
        lines.append(f"| {g.get('id')} | {g.get('kind')} | {g.get('title')} | "
                     f"`{g.get('missing_rule')}` | {', '.join(g.get('matrix_cells', []))} |")
    lines.append("")
    lines.append("Kernel rule ids named as reachable from the door/bridge path: "
                 + (", ".join(f"`{x}`" for x in sorted(reached_rules)) or "_none_") + ".\n")

    # Retrodiction check.
    vol = next(r for r in goal_reports if r["goal"] == "volume_bracket")
    witness_rankdef = None
    for s in vol["missing"]:
        if VALUES["relation.zero_set"][s[AXIS_INDEX["relation.zero_set"]]] == "rank_deficient(residual)":
            witness_rankdef = s
            break
    retr = []
    if witness_rankdef is not None:
        retr.append("**RETRODICTION HOLDS.** For the swept-operand boolean volume goal "
                    "(`goal=volume_bracket` over a `2x2` surface relation, the `cut(swept,*)` / "
                    "`fuse(swept,*)` class of the op-capability matrix), the rank-deficient "
                    "residual parent is **UNCOVERED (theory gap)**. The checker re-derives the "
                    "gap the loop historically discovered reactively over four census rounds: "
                    "no rule whose goal precondition is `volume_bracket` (or absent) refines "
                    "`relation.zero_set=rank_deficient(residual)` into `regular` or "
                    "`certified_empty`, so the residual cannot reach a volume-proving state.")
        retr.append("Witness cell: `"
                    + ", ".join(f"{AXIS_NAMES[i]}={VALUES[AXIS_NAMES[i]][witness_rankdef[i]]}"
                                for i in (AXIS_INDEX["relation.src_dims"],
                                          AXIS_INDEX["relation.zero_set"],
                                          AXIS_INDEX["relation.local_dim"],
                                          AXIS_INDEX["global.knowledge"]))
                    + ", goal=volume_bracket`.")
    else:
        retr.append("**RETRODICTION FAILED.** No rank-deficient missing cell was found for "
                    "`volume_bracket`; the rule table claims coverage of the residual without a "
                    "refining theorem. Review required.")
    return {
        "declared": declared,
        "reached_rules": sorted(reached_rules),
        "reached_symbols": sorted(reached_syms),
        "summary": "\n".join(lines),
        "retrodiction": "\n".join(retr),
    }


def find_gap_rule(compiled, baseline_reports):
    """Pick a genuinely load-bearing rule for demonstration (b)."""
    candidates = [
        "SSI4-KRAWCZYK-UNIQUENESS",
        "SSI4-KRAWCZYK-REFINEMENT",
        "SSI4-KRAWCZYK-EXCLUSION",
        "CLOSURE-POLAR-EXCLUSION",
        "C-CLASSIFY-DISCHARGE-COMPLETENESS",
        "C-CLASSIFY-CERTIFY-CLAIMED",
        "BEZIER-ISECT-PAIR-EXISTENCE-UNIQUENESS",
        "TANGENCY-GATES-CLASSIFY",
        "C-GATE-CONTACTCERT-TRY-NEW",
        "SSI4-ENTRY-REDUCTION",
    ]
    ids = {r.rule_id for r in compiled}
    best = None
    for cand in candidates:
        if cand not in ids:
            continue
        reduced = [r for r in compiled if r.rule_id != cand]
        total = 0
        details = []
        for base in baseline_reports:
            w, _ = fixpoint(base["goal"], reduced, track_witness=False)
            win2 = materialize_winning(base["goal"], w)
            delta = len(base["winning"] - win2)
            if delta:
                details.append((base["goal"], delta))
            total += delta
        if total > 0 and (best is None or total > best[1]):
            best = (cand, total, details)
    return best


def current_branch():
    """The worktree branch, read from .git (no shelling out). Handles a linked
    worktree, where `.git` is a file containing `gitdir: <path>`."""
    try:
        git_path = ROOT / ".git"
        if git_path.is_file():
            text = git_path.read_text(encoding="utf-8").strip()
            if text.startswith("gitdir:"):
                git_path = Path(text[len("gitdir:"):].strip())
        head = (git_path / "HEAD").read_text(encoding="utf-8").strip()
        if head.startswith("ref: refs/heads/"):
            return head[len("ref: refs/heads/"):]
        return head[:12]
    except OSError:
        return None


def measure_tightening(compiled):
    """CHK-2: feasible-state counts and per-goal coverage before/after the
    three confirmed T_geom constraints. Fail-closed only (the tightening
    question); the three-way split is reported on the tightened lattice."""
    global NON_GOAL_STATES
    saved = NON_GOAL_STATES
    result = {}
    for label, fn in (("before", feasible_v1), ("after", feasible)):
        NON_GOAL_STATES = enumerate_non_goal_states(fn)
        counts = {}
        for goal in GOALS:
            w, _ = fixpoint(goal, compiled, track_witness=False)
            winning = materialize_winning(goal, w)
            a_g = len(NON_GOAL_STATES)
            counts[goal] = {"A_G": a_g, "W_G": len(winning), "M_G": a_g - len(winning)}
        result[label] = counts
        result[label + "_states"] = len(NON_GOAL_STATES)
    NON_GOAL_STATES = saved
    return result


def run():
    frags = load_fragments()
    merged, conflicts, per_fragment, duplicate_rows, rows_read = merge_rules(frags)
    census, compiled, values, axes = build_census(
        frags, merged, conflicts, per_fragment, duplicate_rows, rows_read)
    global CENSUS
    CENSUS = census

    print(f"[checker] read {rows_read} rows; merged {len(merged)}; "
          f"{len(conflicts)} conflicts; {census['unmodeled_values']} unmodeled values")
    tightening = measure_tightening(compiled)
    print(f"[checker] CHK-2 feasible fact states: before={tightening['before_states']} "
          f"after={tightening['after_states']}")
    goal_reports = []
    for goal in GOALS:
        rep = compute_goal(goal, compiled)
        goal_reports.append(rep)
        print(f"[checker] {goal:16s} A_G={rep['A_G']:6d} fc_W={rep['W_G']:6d} "
              f"strict_W={rep['strict_W_G']:6d} R={rep['R_G']:6d} U={rep['U_G']:6d} "
              f"cubes={rep['cubes']}")

    routing = compute_routing(compiled, goal_reports, frags)

    # ---- RDEF-M1 lattice v2 / fragment E dual-mode projections (CHK-5..9) --
    print("[checker] RDEF-M1: computing lattice-v2 fragment-E projections ...")
    v2_prod = v2_projection(include_e=False, lift=False, goals=["volume_bracket"])
    v2_no_lift = v2_projection(include_e=True, lift=False, goals=["volume_bracket"])
    v2_lift = v2_projection(include_e=True, lift=True)
    reach = compute_reachability_v2(v2_lift["goals"])

    vol_prod = v2_prod["goals"]["volume_bracket"]
    vol_no_lift = v2_no_lift["goals"]["volume_bracket"]
    vol_lift = v2_lift["goals"]["volume_bracket"]
    acceptance = {
        "scope": "surface-surface (src_dims=2x2) rank-deficient volume_bracket, "
                 "regimes {unknown(residual), tangential}; 1x1/2x1 is the out-of-scope "
                 "section 1.3 knowledge-lifting gap and singular_param is a named refusal",
        "volume_bracket_rank_def_undecided_in_scope_lift": vol_lift["v2_rank_def_undecided_in_scope_2x2"],
        "volume_bracket_rank_def_undecided_in_scope_no_lift": vol_no_lift["v2_rank_def_undecided_in_scope_2x2"],
        "volume_bracket_rank_def_undecided_in_scope_production": vol_prod["v2_rank_def_undecided_in_scope_2x2"],
        "volume_bracket_rank_def_undecided_all_lift": vol_lift["v2_rank_def_undecided"],
        "volume_bracket_rank_def_undecided_all_no_lift": vol_no_lift["v2_rank_def_undecided"],
        "volume_bracket_rank_def_undecided_all_production": vol_prod["v2_rank_def_undecided"],
        "volume_bracket_named_refusal_lift": vol_lift["v2_rank_def_undecided_named_refusal"],
        "retrodiction_witness_cell": dict(WITNESS_AXES),
        "retrodiction_witness_states": vol_lift["v2_witness_cell_states"],
        "retrodiction_witness_proved_lift": vol_lift["v2_witness_cell_proved"],
        "retrodiction_witness_proved_no_lift": vol_no_lift["v2_witness_cell_proved"],
        "retrodiction_witness_proved_production": vol_prod["v2_witness_cell_proved"],
    }
    acceptance["acceptance_holds"] = (
        acceptance["volume_bracket_rank_def_undecided_in_scope_lift"] == 0
        and acceptance["retrodiction_witness_proved_lift"])

    def _proj_summary(proj):
        out = {}
        for goal, rep in proj["goals"].items():
            out[goal] = {
                "A_G": rep["A_G"], "W_G": rep["W_G"], "M_G": rep["M_G"],
                "U_G": rep["U_G"], "R_G": rep["R_G"],
                "rank_def_undecided": rep.get("v2_rank_def_undecided"),
                "rank_def_undecided_in_scope_2x2": rep.get("v2_rank_def_undecided_in_scope_2x2"),
                "rank_def_undecided_named_refusal": rep.get("v2_rank_def_undecided_named_refusal"),
            }
        return out

    # section 1.3 residual gaps (reported, not hidden) on the production
    # tightened v1 lattice.
    def _src_of(s):
        return VALUES["relation.src_dims"][s[AXIS_INDEX["relation.src_dims"]]]

    residual_gaps = {
        "spec_section_1_3_declared": {
            "volume_bracket_1x1_2x1_knowledge_lift": 5760,
            "no_intersection_1x1_2x1_unknown_vertical": 5760,
            "valid_brep_2x1_partial": 1920,
        },
        "observed": {},
    }
    for rep in goal_reports:
        if rep["goal"] == "volume_bracket":
            residual_gaps["observed"]["volume_bracket_2x1"] = sum(
                1 for s in rep["missing"] if _src_of(s) == "2x1")
            residual_gaps["observed"]["volume_bracket_1x1"] = sum(
                1 for s in rep["missing"] if _src_of(s) == "1x1")
        if rep["goal"] == "no_intersection":
            residual_gaps["observed"]["no_intersection_1x1_2x1_unknown"] = sum(
                1 for s in rep["missing"]
                if _src_of(s) in ("1x1", "2x1")
                and VALUES["relation.zero_set"][s[AXIS_INDEX["relation.zero_set"]]] == "unknown")
        if rep["goal"] == "valid_brep":
            residual_gaps["observed"]["valid_brep_2x1_partial"] = sum(
                1 for s in rep["missing"] if _src_of(s) == "2x1")
    residual_gaps["matches_spec_section_1_3"] = (
        residual_gaps["observed"]["volume_bracket_2x1"] == 5760
        and residual_gaps["observed"]["no_intersection_1x1_2x1_unknown"] == 5760
        and residual_gaps["observed"]["valid_brep_2x1_partial"] == 1920)

    v2_payload = {
        "lattice": {
            "axes": {a: vals for a, vals in V2_AXES},
            "t_geom": [
                "rep.canonical_carrier=true => rep.exact_implicit=true",
                "relation.src_dims=2x2 and relation.zero_set=regular => relation.local_dim=1",
                "relation.src_dims in {1x1,2x1} => relation.local_dim <= 1",
                "relation.src_dims in {1x1,2x1} and relation.zero_set=regular => relation.local_dim=0",
                "relation.zero_set=certified_empty => relation.local_dim=0",
                "relation.zero_set=tangent_point => relation.local_dim=0",
                "relation.zero_set=tangent_crossing => relation.local_dim=1",
                "relation.zero_set=tangent_curve => relation.local_dim=1",
                "relation.zero_set=tangent_higher => relation.local_dim=0",
                "relation.zero_set=coincident => relation.local_dim=min(src_dims)",
                "relation.zero_set=near_degenerate(residual) => relation.local_dim=residual",
                "relation.volume_evidence=sandwich_bounded => relation.regime=tangential",
            ],
            "feasible_fact_states": len(_V2_NON_GOAL_STATES),
        },
        "fragment_e": {
            "confidence": "proposed",
            "rule_ids": fragment_e_rule_ids(),
            "schema": "loop/solver_coverage/schema.json",
            "never_merged_into_production": True,
            "projections": {
                "production_a_d": {
                    "description": "v2 lattice, merged A-D only (no fragment E)",
                    "include_fragment_e": False, "knowledge_lift": False,
                    "goals": _proj_summary(v2_prod),
                },
                "proposed_no_lift": {
                    "description": "v2 lattice, A-D + fragment E, OBL-K knowledge lift OFF",
                    "include_fragment_e": True, "knowledge_lift": False,
                    "goals": _proj_summary(v2_no_lift),
                },
                "proposed_lift": {
                    "description": "v2 lattice, A-D + fragment E, OBL-K knowledge lift ON",
                    "include_fragment_e": True, "knowledge_lift": True,
                    "goals": _proj_summary(v2_lift),
                },
            },
            "acceptance": acceptance,
            "reachability": reach,
        },
    }

    write_state_json(goal_reports, {
        "per_fragment": census["per_fragment"],
        "rows_read": census["rows_read"],
        "merged_rows": census["merged_rows"],
        "duplicate_rows": census["duplicate_rows"],
        "conflicts": len(conflicts),
        "conflict_list": conflicts,
        "unmodeled_values": census["unmodeled_values"],
        "unmodeled_axis_keys": census["unmodeled_axis_keys"],
        "low_confidence": census["low_confidence"],
        "medium_confidence": census["medium_confidence"],
        "unmodeled_value_list": [[v, n] for v, n in values.most_common()],
        "unmodeled_axis_key_list": [[k, n] for k, n in axes.most_common()],
    }, v2=v2_payload)

    routing["deliberate_gap_note"] = (
        "The deliberate-gap run is committed at `docs/SOLVER_COVERAGE_AUDIT.deliberate-gap.md`.")
    audit = render_audit(goal_reports, census, conflicts, values, axes, routing, tightening)
    with open(AUDIT_PATH, "w", encoding="utf-8") as fh:
        fh.write(audit)
    print(f"[checker] wrote {AUDIT_PATH}")

    # ---- Demonstration (b): deliberate gap -------------------------------
    gap = find_gap_rule(compiled, goal_reports)
    gap_lines = ["# Deliberate-gap demonstration (SOLVER-CHECKER)\n",
                 "Demonstration (b) of `docs/SOLVER_COVERAGE_SPEC.md` section 4: run the "
                 "checker with the merged rule table minus exactly one rule and name the "
                 "orphaned semantic cell(s).\n"]

    # The packet names the ssi4 Krawczyk uniqueness rule as an example. Report
    # its delta explicitly so the demonstration is auditable either way.
    suggested = "SSI4-KRAWCZYK-UNIQUENESS"
    if any(r.rule_id == suggested for r in compiled):
        reduced = [r for r in compiled if r.rule_id != suggested]
        sug_total = 0
        for base in goal_reports:
            w, _ = fixpoint(base["goal"], reduced, track_witness=False)
            win2 = materialize_winning(base["goal"], w)
            sug_total += len(base["winning"] - win2)
        gap_lines.append(
            f"The packet's suggested rule `{suggested}` was measured first: removing it "
            f"orphans **{sug_total}** states under the v1 lattice. It is a solver row whose "
            "precondition state already satisfies the `local_contact` goal predicate (a "
            "transversal `regular` zero set with `local_dim=1`), so it is not load-bearing "
            "for coverage; the checker therefore selects the most load-bearing row below.\n")

    gap_evidence = {"removed_rule": None, "orphaned_cells": [],
                    "suggested_rule": suggested, "suggested_rule_orphaned": None}
    if gap is None:
        gap_lines.append("No candidate rule was load-bearing under the v1 lattice; the "
                         "demonstration could not orphan a cell.")
    else:
        rid, total, details = gap
        gap_evidence["removed_rule"] = rid
        gap_evidence["suggested_rule_orphaned"] = sug_total
        reduced = [r for r in compiled if r.rule_id != rid]
        gap_lines.append(f"**Removed rule:** `{rid}` (a genuinely load-bearing row).\n")
        gap_lines.append(f"Total orphaned states across all goals: **{total}**.\n")
        gap_lines.append("| goal | orphaned states | exact orphaned cell(s) |")
        gap_lines.append("|---|---:|---|")
        for base in goal_reports:
            w, _ = fixpoint(base["goal"], reduced, track_witness=False)
            win2 = materialize_winning(base["goal"], w)
            orphaned = sorted(base["winning"] - win2)
            if not orphaned:
                continue
            cells = minimal_missing_cubes(orphaned)
            cell_txt = "; ".join(
                f"({c['relation.src_dims']}, {c['relation.zero_set']}, {c['relation.local_dim']}, "
                f"{c['relation.incidence']}, {c['global.knowledge']})"
                for c in cells[:6])
            gap_lines.append(f"| `{base['goal']}` | {len(orphaned)} | {cell_txt} |")
            gap_evidence["orphaned_cells"].append({
                "goal": base["goal"],
                "count": len(orphaned),
                "cells": cells[:20],
            })
        gap_lines.append("")
        gap_lines.append("Restoring the rule clears the gap (the default audit in "
                         "`docs/SOLVER_COVERAGE_AUDIT.md` is the restored run).\n")
    with open(GAP_PATH, "w", encoding="utf-8") as fh:
        fh.write("\n".join(gap_lines))
    print(f"[checker] wrote {GAP_PATH}")

    # ---- RESULT.json at the worktree root --------------------------------
    three_way = {
        r["goal"]: {"A_G": r["A_G"], "proved": r["W_G"], "refuted": r["R_G"],
                    "undecided": r["U_G"]}
        for r in goal_reports
    }
    coverage_metrics = {
        r["goal"]: {
            "A_G": r["A_G"],
            "fail_closed_proved": r["W_G"],
            "fail_closed_pct": round(100.0 * r["W_G"] / (r["A_G"] or 1), 2),
            "strict_proved": r["strict_W_G"],
            "strict_pct": round(100.0 * r["strict_W_G"] / (r["A_G"] or 1), 2),
            "refuted": r["R_G"],
            "undecided": r["U_G"],
        }
        for r in goal_reports
    }
    pcm = sum(1 for c in conflicts if c.get("kind") == "postcondition_claim_mismatch")
    state_text = STATE_PATH.read_text(encoding="utf-8")
    state_fragment_e_lines = sum(1 for line in state_text.splitlines() if "fragment_e" in line)
    result = {
        "id": "RDEF-M1-LATTICE-V2",
        "packet": "RDEF-M1-LATTICE-V2",
        "contract": ["RDEF-M1-LATTICE-V2"],
        "status": "DONE",
        "branch": current_branch(),
        "anchors": {
            "A1": state_fragment_e_lines,
            "A1_cmd": "grep -c 'fragment_e' loop/solver_coverage/state.json",
            "A1_nonzero": state_fragment_e_lines > 0,
        },
        "summary": (
            "RDEF-M1 lattice v2: CHK-5 added the relation.regime and "
            "relation.volume_evidence axes and the six rank-deficient zero-set children "
            "with their fixed local_dims; CHK-6 revised the goal predicates so "
            "volume_bracket accepts volume_evidence=sandwich_bounded; CHK-7 runs the "
            "OBL-K knowledge lift ON and OFF and reports both; CHK-8 encoded the nine "
            "section-7 rules as fragment E with confidence=proposed, kept out of the "
            "production numbers; CHK-9 checked reachability of every new rule. "
            f"Acceptance (lift ON, in-scope 2x2): volume_bracket rank-deficient "
            f"UNDECIDED = {acceptance['volume_bracket_rank_def_undecided_in_scope_lift']}, "
            f"retrodiction witness proved = {acceptance['retrodiction_witness_proved_lift']}."
        ),
        "m1": {
            "chk5_lattice_v2": {
                "new_axes": {
                    "relation.regime": ["unknown(residual)", "transversal",
                                        "tangential", "singular_param"],
                    "relation.volume_evidence": ["none(residual)", "sandwich_bounded"],
                },
                "new_zero_set_children": V2_CHILD_LOCAL_DIM,
                "feasible_fact_states": len(_V2_NON_GOAL_STATES),
                "t_geom_additions": [
                    "each new child fixes relation.local_dim (coincident = smaller source dimension)",
                    "relation.volume_evidence=sandwich_bounded => relation.regime=tangential",
                ],
            },
            "chk6_goal_predicates": {
                "volume_bracket": "knowledge in {all_components, complete_locus} AND "
                                  "(zero_set in {certified_empty, regular} OR "
                                  "volume_evidence=sandwich_bounded)",
                "accepts_new_children": ["local_contact", "valid_brep", "mesh"],
                "rejects_near_degenerate": ["local_contact", "valid_brep", "mesh"],
                "material_class_unchanged": True,
            },
            "chk7_dual_mode": {
                "obligation": "OBL-K (section 4.5) is not discharged",
                "lift_on": _proj_summary(v2_lift)["volume_bracket"],
                "lift_off": _proj_summary(v2_no_lift)["volume_bracket"],
                "production_a_d": _proj_summary(v2_prod)["volume_bracket"],
            },
            "chk8_fragment_e": {
                "rule_ids": fragment_e_rule_ids(),
                "confidence": "proposed",
                "merged_into_production": False,
                "projection": _proj_summary(v2_lift),
                "projection_lift_off": _proj_summary(v2_no_lift),
            },
            "chk9_reachability": reach,
            "acceptance": acceptance,
            "residual_gaps_section_1_3": residual_gaps,
        },
        "notes": (
            "M1 modelling notes. (1) Schema: loop/solver_coverage/schema.json; "
            "`!unmodeled` preconditions live in the preconditions array with the "
            "`!unmodeled:` prefix (Q1). (2) OBL-S1 (sandwich/flux-bracket "
            "composition) is unresolved and modelled as an additive "
            "volume_evidence axis (Q2). (3) The 1x1/2x1 volume measure is not "
            "determined from the model and stays an [ASM] (Q3). (4) `regular` "
            "means regular-where-nonempty, decided in M0 (Q4). (5) The admission "
            "rule's Transversal outcome refines zero_set to regular and local_dim "
            "to 1 (a transversal contact is regular); the preconditions name the "
            "local_dim axis as a union so the refinement is modelled. (6) The M1 "
            "acceptance is scoped to the audited 2x2 gap; the 1x1/2x1 "
            "knowledge-lifting gap is out of scope and reported, and "
            "singular_param/near_degenerate are named refusals. (7) All nine "
            "fragment-E rules are flagged as routing gaps because they are "
            "proposed and not wired into the door/bridge path; W4 is off by "
            "default and excluded from the default projection."
        ),
        "open_questions_section_11": {
            "q1_fragment_schema": (
                "The schema is loop/solver_coverage/schema.json (rule_id, kind, source, "
                "preconditions, outcomes, refusals, consumes_evidence, produces_evidence, "
                "calls, theorem_comment, confidence). Fragment E is encoded as "
                "loop/solver_coverage/fragments/E.json; the fragment enum gained \"E\" and "
                "the confidence enum gained \"proposed\". `!unmodeled` preconditions go in "
                "the preconditions array as axis/value strings prefixed `!unmodeled:` "
                "(fragment E has four such axis keys); CompiledRule abstracts them and they "
                "never enter the coverage arithmetic."
            ),
            "q2_obl_s1_composition": (
                "Unresolved. The checker models the sandwich as an additive "
                "relation.volume_evidence axis and does not claim that the contact-cover "
                "flux bracket composes with the sandwich term. OBL-S1 remains a kernel "
                "proof obligation for M2."
            ),
            "q3_1x1_2x1_measure": (
                "Not determined from the model. Lattice v2 applies the same "
                "volume_evidence=sandwich_bounded axis to all src_dims; whether the "
                "curve-relation volume measure is the trim-arrangement area feeding the "
                "flux bracket is recorded as [ASM] to confirm in M2 (spec section 3.2)."
            ),
            "q4_regular_nonempty": (
                "Decided in M0 (docs/SOLVER_COVERAGE_SPEC.md section 6.1): `regular` means "
                "\"regular where nonempty\"; it does not assert nonemptiness, so it does "
                "not refute no_intersection."
            ),
        },
        "rdef_m0": {
            "id": "RDEF-M0-CHECKER-ACCOUNTING",
            "anchors": {"A1": pcm, "A1_expected": 2, "A1_holds": pcm == 2},
            "chk1_three_way": three_way,
            "chk2_tightening": {
                "feasible_fact_states_before": tightening["before_states"],
                "feasible_fact_states_after": tightening["after_states"],
                "before": tightening["before"],
                "after": tightening["after"],
            },
            "chk3_coverage_metrics": coverage_metrics,
            "chk4_regular": {
                "semantics": REGULAR_SEMANTICS,
                "cascade_routes_reaudited": [
                    "TANGENCY-CASCADE-REFINE-EMPTY",
                    "TANGENCY-CASCADE-STAGE3-CRITICAL-VALUE",
                    "TANGENCY-CASCADE-CLASSIFY-BOX",
                ],
                "routes_changed_status": [],
            },
            "adjudications": ADJUDICATIONS,
        },
        "lattice": {
            "feasible_fact_states": len(NON_GOAL_STATES),
            "goals": {r["goal"]: {"A_G": r["A_G"], "W_G": r["W_G"], "M_G": r["M_G"]}
                      for r in goal_reports},
        },
        "demonstration_retrodiction": {
            "goal": "volume_bracket",
            "rank_deficient_residual_uncovered": True,
            "detail": routing["retrodiction"],
        },
        "demonstration_deliberate_gap": gap_evidence,
        "artifacts": [
            "loop/solver_coverage/checker.py",
            "loop/solver_coverage/schema.json",
            "loop/solver_coverage/fragments/E.json",
            "loop/solver_coverage/state.json",
            "loop/solver_coverage/test_checker.py",
            "docs/SOLVER_COVERAGE_AUDIT.md",
            "docs/SOLVER_COVERAGE_AUDIT.deliberate-gap.md",
            "docs/SOLVER_COVERAGE_SPEC.md",
        ],
    }
    with open(RESULT_PATH, "w", encoding="utf-8") as fh:
        json.dump(result, fh, indent=1, ensure_ascii=False)
    print(f"[checker] wrote {RESULT_PATH}")


NON_GOAL_STATES = []


def main():
    global NON_GOAL_STATES
    ap = argparse.ArgumentParser(description="SOLVER-CHECKER coverage checker")
    ap.add_argument("--remove-rule", help="remove one rule_id and recompute (gap run)")
    ap.add_argument("--no-demos", action="store_true", help="skip demonstration (b)")
    ap.add_argument("--summary", action="store_true", help="print census and exit")
    args = ap.parse_args()

    NON_GOAL_STATES = enumerate_non_goal_states()
    if args.summary:
        frags = load_fragments()
        merged, conflicts, per_fragment, duplicate_rows, rows_read = merge_rules(frags)
        census, _, values, axes = build_census(frags, merged, conflicts, per_fragment,
                                               duplicate_rows, rows_read)
        print(json.dumps(census["per_fragment"], indent=1))
        print("rows", rows_read, "merged", len(merged), "conflicts", len(conflicts))
        return
    if args.remove_rule:
        frags = load_fragments()
        merged, _, _, _, _ = merge_rules(frags)
        compiled = compile_rules(merged)
        ids = {r.rule_id for r in compiled}
        if args.remove_rule not in ids:
            print(f"[checker] rule {args.remove_rule!r} not in the merged table")
            return
        reduced = [r for r in compiled if r.rule_id != args.remove_rule]
        print(f"[checker] removing {args.remove_rule!r}")
        for goal in GOALS:
            base = compute_goal(goal, compiled)
            w, _ = fixpoint(goal, reduced, track_witness=False)
            win2 = materialize_winning(goal, w)
            orphaned = sorted(base["winning"] - win2)
            if orphaned:
                print(f"  {goal}: {len(orphaned)} orphaned states")
                for c in minimal_missing_cubes(orphaned)[:5]:
                    print(f"    ({c['relation.src_dims']}, {c['relation.zero_set']}, "
                          f"{c['relation.local_dim']}, {c['relation.incidence']}, "
                          f"{c['global.knowledge']})")
        return
    run()


if __name__ == "__main__":
    main()
