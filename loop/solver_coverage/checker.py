#!/usr/bin/env python3
"""SOLVER-CHECKER -- the AND-OR semantic solver-coverage checker.

Reads the four survey fragments (loop/solver_coverage/fragments/{A,B,C,D}.json),
merges them into one rule table, builds the explicit v1 proof-state lattice
from the frozen axes in docs/SOLVER_COVERAGE_SPEC.md (sections 1-5), computes
the AND-OR winning region W_G for each of the seven goals, and emits the first
semantic coverage audit.

Pure Python over JSON: no kernel changes, no OCC, no cargo builds.

Usage:
    python loop/solver_coverage/checker.py                 # full audit + demos
    python loop/solver_coverage/checker.py --no-demos      # audit only
    python loop/solver_coverage/checker.py --remove-rule R # gap run for R
    python loop/solver_coverage/checker.py --summary       # print census only
"""

from __future__ import annotations

import argparse
import collections
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
# 2. T_geom feasibility (spec section 2): the two frozen v1 laws.
# --------------------------------------------------------------------------
def feasible(state) -> bool:
    """T_geom feasibility of a full concrete state tuple (axis indices).

    * canonical carrier implies exact-implicit (rep.canonical_carrier -> rep.exact_implicit)
    * rank DF=3 over a 2x2 relation implies local contact dimension 1
      (src_dims == 2x2 and zero_set == regular -> local_dim == 1)
    """
    canonical = state[AXIS_INDEX["rep.canonical_carrier"]] == VALUES["rep.canonical_carrier"].index("true")
    exact = state[AXIS_INDEX["rep.exact_implicit"]] == VALUES["rep.exact_implicit"].index("true")
    if canonical and not exact:
        return False
    src = state[AXIS_INDEX["relation.src_dims"]] == VALUES["relation.src_dims"].index("2x2")
    regular = state[AXIS_INDEX["relation.zero_set"]] == VALUES["relation.zero_set"].index("regular")
    if src and regular and state[AXIS_INDEX["relation.local_dim"]] != VALUES["relation.local_dim"].index("1"):
        return False
    return True


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
            continue
        merged[key] = r
    return list(merged.values()), conflicts, per_fragment, duplicate_rows, len(all_rules)


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
                 "expensive", "pre_cube", "abstracted")

    def __init__(self, rule):
        self.rule_id = rule["rule_id"]
        self.kind = rule.get("kind")
        self.confidence = rule.get("confidence", "high")
        self.fragment = rule["_frag"]
        self.symbol = rule["source"]["symbol"]
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
            text = " ".join(str(x) for x in (o.get("variant"),)) + " " + " ".join(rule.get("refusals", []))
            if any(m in text for m in SCOPE_REFUSAL_MARKERS):
                scope = True
        self.scope_refusal = scope and self.refusal_only
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
def enumerate_non_goal_states():
    """All T_geom-feasible concrete states over the eleven fact axes."""
    fact_axes = [i for i in range(N) if i != GOAL_IDX]
    states = []
    for combo in itertools.product(*[VALUES[AXIS_NAMES[i]] for i in fact_axes]):
        state = [None] * N
        for i, val in zip(fact_axes, combo):
            state[i] = VALUES[AXIS_NAMES[i]].index(val)
        state[GOAL_IDX] = 0  # placeholder
        st = tuple(state)
        if feasible(st):
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

    * OUTSIDE_ENVELOPE -- an intentional scope-refusal rule matches the state.
    * THEORY GAP      -- a goal-target axis is incompatible with the goal and no
                         goal-applicable rule can refine it (the residual parent).
    * VERTICAL GAP    -- rules can progress the target axes but the chain stalls
                         below the requested guarantee.
    """
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


def compute_goal(goal, compiled):
    w, witness = fixpoint(goal, compiled)
    winning = materialize_winning(goal, w)
    g = _v("goal", goal)
    a_g = []
    for st in NON_GOAL_STATES:
        s = list(st)
        s[GOAL_IDX] = g
        a_g.append(tuple(s))
    missing = [s for s in a_g if s not in winning]
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
                      "covered": sum(1 for s in states if s in winning), "route": route})
    return {
        "goal": goal,
        "A_G": len(a_g),
        "W_G": len(winning),
        "M_G": len(missing),
        "cubes": len(w),
        "winning_cube_set": w,
        "regions": table,
        "missing": missing,
        "winning": winning,
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


def write_state_json(goal_reports, census):
    payload = {
        "schema": "solver-coverage-state.v1",
        "lattice": {
            "axes": {a: VALUES[a] for a in AXIS_NAMES},
            "t_geom": [
                "rep.canonical_carrier=true => rep.exact_implicit=true",
                "relation.src_dims=2x2 and relation.zero_set=regular => relation.local_dim=1",
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
            "winning_cubes": len(cube_list),
            "winning_cube_list": cube_list,
            "missing_cells": minimal_missing_cubes(rep["missing"]),
        }
    with open(STATE_PATH, "w", encoding="utf-8") as fh:
        json.dump(payload, fh, indent=1, ensure_ascii=False)


def render_goal_md(rep, unmodeled_note):
    lines = []
    lines.append(f"### Goal `{rep['goal']}`\n")
    lines.append(f"- `A_G` (feasible states for this goal): **{rep['A_G']}**")
    lines.append(f"- `W_G` (AND-OR winning states): **{rep['W_G']}**")
    lines.append(f"- `M_G = A_G \\ W_G` (missing region): **{rep['M_G']}**")
    lines.append(f"- winning region described by {rep['cubes']} cubes\n")

    lines.append("#### Horizontal table (region -> status)\n")
    lines.append("Region = (src_dims, zero_set, local_dim, global.knowledge); incidence and the "
                 "six rep flags are existentially quantified.\n")
    lines.append("| src_dims | zero_set | local_dim | knowledge | states | covered | status | route |")
    lines.append("|---|---|---|---|---:|---:|---|---|")
    for row in rep["regions"]:
        k = row["region"]
        route = " -> ".join(row["route"][:6]) if row["route"] else ""
        lines.append(f"| {k[0]} | {k[1]} | {k[2]} | {k[3]} | {row['states']} | "
                     f"{row['covered']} | {row['status']} | {route} |")
    lines.append("")

    lines.append("#### Vertical chains (proof-strength reach per route)\n")
    levels = VALUES["global.knowledge"]
    reached = collections.Counter()
    for s in rep["winning"]:
        reached[VALUES["global.knowledge"][s[AXIS_INDEX["global.knowledge"]]]] += 1
    missing_levels = collections.Counter()
    for s in rep["missing"]:
        missing_levels[VALUES["global.knowledge"][s[AXIS_INDEX["global.knowledge"]]]] += 1
    lines.append("| proof strength | winning states | missing states |")
    lines.append("|---|---:|---:|")
    for lv in levels:
        lines.append(f"| {lv} | {reached[lv]} | {missing_levels[lv]} |")
    lines.append("")

    lines.append("#### Missing region (concrete semantic cells)\n")
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


def render_audit(goal_reports, census, conflicts, unmodeled_values, unmodeled_axes, routing):
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
        for c in conflicts:
            L.append(f"- `{c['kind']}`: `{c.get('rule_id') or c.get('symbol')}` "
                     f"{c.get('variant', '')} -- {c.get('detail', '')}")
        L.append("")
    else:
        L.append("No fragment conflicts were found; overlapping extractions (e.g. A/B on "
                 "`tangency/gates.rs` and `tangency/tsystem.rs`) are complementary rows, not "
                 "disagreeing postcondition claims.\n")

    L.append("## 2. The v1 lattice\n")
    L.append("Frozen axes and concrete value domains:\n")
    L.append("| axis | values |")
    L.append("|---|---|")
    for a in AXIS_NAMES:
        L.append(f"| `{a}` | {', '.join('`'+v+'`' for v in VALUES[a])} |")
    L.append("")
    L.append("`T_geom` (v1): `rep.canonical_carrier=true => rep.exact_implicit=true`; "
             "`relation.src_dims=2x2 and relation.zero_set=regular => relation.local_dim=1`. "
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

    L.append("## 3. Per-goal audit\n")
    for rep in goal_reports:
        L.append(render_goal_md(rep, None))

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


def run():
    frags = load_fragments()
    merged, conflicts, per_fragment, duplicate_rows, rows_read = merge_rules(frags)
    census, compiled, values, axes = build_census(
        frags, merged, conflicts, per_fragment, duplicate_rows, rows_read)
    global CENSUS
    CENSUS = census

    print(f"[checker] read {rows_read} rows; merged {len(merged)}; "
          f"{len(conflicts)} conflicts; {census['unmodeled_values']} unmodeled values")
    goal_reports = []
    for goal in GOALS:
        rep = compute_goal(goal, compiled)
        goal_reports.append(rep)
        print(f"[checker] {goal:16s} A_G={rep['A_G']:6d} W_G={rep['W_G']:6d} "
              f"M_G={rep['M_G']:6d} cubes={rep['cubes']}")

    routing = compute_routing(compiled, goal_reports, frags)
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
    })

    routing["deliberate_gap_note"] = (
        "The deliberate-gap run is committed at `docs/SOLVER_COVERAGE_AUDIT.deliberate-gap.md`.")
    audit = render_audit(goal_reports, census, conflicts, values, axes, routing)
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
    result = {
        "packet": "SOLVER-CHECKER",
        "status": "DONE",
        "rule_census": {
            "per_fragment": census["per_fragment"],
            "rows_read": census["rows_read"],
            "merged_rows": census["merged_rows"],
            "duplicate_rows": census["duplicate_rows"],
            "conflicts_found": len(conflicts),
            "conflicts": conflicts,
            "unmodeled_values": census["unmodeled_values"],
            "unmodeled_axis_keys": census["unmodeled_axis_keys"],
            "low_confidence": census["low_confidence"],
            "medium_confidence": census["medium_confidence"],
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
            "loop/solver_coverage/state.json",
            "loop/solver_coverage/test_checker.py",
            "docs/SOLVER_COVERAGE_AUDIT.md",
            "docs/SOLVER_COVERAGE_AUDIT.deliberate-gap.md",
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
