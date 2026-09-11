#!/usr/bin/env python3
"""Regression tests for the SOLVER-CHECKER coverage checker.

The load-bearing test is the RETRODICTION assertion (spec section 4,
demonstration (a)): the checker must independently re-derive the
partial-coincidence/tangency theory gap that the loop historically discovered
reactively. If a future rule table starts claiming coverage of the
rank-deficient residual parent for the swept-operand boolean volume goal
without a real refining theorem, this assertion fails and forces review.

Run with either:

    python loop/solver_coverage/test_checker.py
    python -m pytest loop/solver_coverage/test_checker.py
"""

from __future__ import annotations

import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

import checker  # noqa: E402


def _compiled():
    if not checker.NON_GOAL_STATES:
        checker.NON_GOAL_STATES = checker.enumerate_non_goal_states()
    frags = checker.load_fragments()
    merged, conflicts, per_fragment, duplicate_rows, rows_read = checker.merge_rules(frags)
    return checker.compile_rules(merged), conflicts


def test_retrodiction():
    """Demonstration (a): the swept-operand boolean volume goal leaves the
    rank-deficient residual parent UNCOVERED (a theory gap)."""
    compiled, _ = _compiled()
    rep = checker.compute_goal("volume_bracket", compiled)

    rank_def = checker.VALUES["relation.zero_set"].index("rank_deficient(residual)")
    uncovered = [
        s for s in rep["missing"]
        if s[checker.AXIS_INDEX["relation.zero_set"]] == rank_def
    ]
    # The residual parent is a first-class uncovered region for this goal.
    retrodiction = len(uncovered) > 0
    assert retrodiction, (
        "RETRODICTION FAILED: volume_bracket now covers the rank_deficient(residual) "
        "zero-set parent without a refining theorem -- review the rule table."
    )

    # The specific swept-operand (2x2 surface-relation) witness cell.
    src = checker.VALUES["relation.src_dims"].index("2x2")
    swept_witness = [s for s in uncovered if s[checker.AXIS_INDEX["relation.src_dims"]] == src]
    assert swept_witness, "no 2x2 rank-deficient witness for the swept-operand boolean volume goal"


def test_rank_deficient_is_a_sink_for_volume():
    """No rule whose goal precondition is volume_bracket (or absent) may refine
    the rank-deficient residual into a volume-proving zero set."""
    compiled, _ = _compiled()
    offenders = []
    for rule in compiled:
        if rule.goal_pre not in (None, "volume_bracket"):
            continue
        for q in rule.progress:
            if q.get("relation.zero_set") in ("regular", "certified_empty"):
                # A refining rule must name the residual in its precondition to
                # be allowed to transform it (the admission rule).
                if rule.pre.get("relation.zero_set") == "rank_deficient(residual)":
                    offenders.append(rule.rule_id)
    assert not offenders, f"rules claim to refine the residual for volume_bracket: {offenders}"


def test_deliberate_gap_orphans_cells():
    """Demonstration (b): removing a load-bearing rule orphans concrete cells."""
    compiled, _ = _compiled()
    baseline = checker.compute_goal("complete_locus", compiled)
    reduced = [r for r in compiled if r.rule_id != "C-CLASSIFY-DISCHARGE-COMPLETENESS"]
    w, _ = checker.fixpoint("complete_locus", reduced, track_witness=False)
    win2 = checker.materialize_winning("complete_locus", w)
    orphaned = baseline["winning"] - win2
    assert orphaned, "removing C-CLASSIFY-DISCHARGE-COMPLETENESS orphaned no cell"
    cells = checker.minimal_missing_cubes(orphaned)
    assert cells and all("relation.zero_set" in c for c in cells)


def test_conflicts_ignore_unmodeled_silence():
    """Complementary A/B extractions are not reported as postcondition conflicts."""
    _, conflicts = _compiled()
    for c in conflicts:
        if c["kind"] == "postcondition_claim_mismatch":
            assert all(claim["postconditions"] for claim in c["claims"]), c


if __name__ == "__main__":
    tests = [v for k, v in sorted(globals().items()) if k.startswith("test_") and callable(v)]
    failed = 0
    for t in tests:
        try:
            t()
            print(f"ok   {t.__name__}")
        except AssertionError as exc:
            failed += 1
            print(f"FAIL {t.__name__}: {exc}")
    if failed:
        sys.exit(1)
    print(f"\n{len(tests)} passed")
