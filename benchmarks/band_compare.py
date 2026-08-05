"""Reconcile two band sweeps, class by class.

Answers the only question that matters after a curve-admission change: where did
the previously-refused faces *go*? Every face refused by the old revision must
land in exactly one of

    recovered | advanced to a different typed exit | still refused

and this checks that the three add up to the old population exactly, per model
and in total. A change that "gained faces" without balancing here has moved
faces somewhere nobody looked.

The identity, per model, with band eligibility unchanged:

    old_unsupported = (new_recovered - old_recovered)
                    + (new_other_exits - old_other_exits)
                    + new_unsupported

Usage:

    python benchmarks/band_compare.py --baseline report.baseline.json --new report.json
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path


def other_exits(row: dict) -> int:
    """Every refusal that is not the curve exit.

    The deck-join exit belongs here. It is a *later* stage than the traversal
    gate — a face only reaches it once its bound curves are readable — so a
    face moving into it has advanced, exactly like one moving into a witness
    exit. Excluding it left 146 faces unaccounted for corpus-wide and made five
    models fail to balance, which is precisely the arithmetic this script is
    for.
    """
    return (row.get("band_refused") or 0) - (row.get("unsupported_faces") or 0)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--baseline", required=True)
    parser.add_argument("--new", required=True)
    args = parser.parse_args()

    base = json.loads(Path(args.baseline).read_text())
    new = json.loads(Path(args.new).read_text())

    print(
        f"baseline truck={base['revisions']['truck']}  "
        f"new truck={new['revisions']['truck']}"
    )
    header = (
        f"{'model':10} {'circle_only':>11} {'old_unsup':>9} {'recovered':>9} "
        f"{'advanced':>8} {'refused':>8} {'balance':>7} {'rendered':>9}"
    )
    print("\n" + header)

    totals = dict.fromkeys(
        ("circle_only", "old_unsup", "recovered", "advanced", "refused", "rendered"), 0
    )
    unbalanced, regressed = [], []

    for model_id in sorted(new["models"]):
        old_row = base["models"].get(model_id, {})
        new_row = new["models"][model_id]
        if old_row.get("status") != "completed" or new_row.get("status") != "completed":
            continue

        old_unsup = old_row.get("unsupported_faces") or 0
        circle_only = (base.get("near_circle_faces") or {}).get(model_id, 0)
        recovered = (new_row.get("band_recovered") or 0) - (
            old_row.get("band_recovered") or 0
        )
        advanced = other_exits(new_row) - other_exits(old_row)
        refused = new_row.get("unsupported_faces") or 0
        rendered = (new_row.get("band_enabled_rendered") or 0) - (
            old_row.get("band_enabled_rendered") or 0
        )
        balance = old_unsup - (recovered + advanced + refused)

        if balance != 0:
            unbalanced.append(model_id)
        # A face that rendered before and not now is a regression, and the
        # gate-closed control must not move at all.
        if rendered < 0 or (new_row.get("gate_closed_rendered") or 0) != (
            old_row.get("gate_closed_rendered") or 0
        ):
            regressed.append(model_id)

        for key, value in (
            ("circle_only", circle_only),
            ("old_unsup", old_unsup),
            ("recovered", recovered),
            ("advanced", advanced),
            ("refused", refused),
            ("rendered", rendered),
        ):
            totals[key] += value

        if old_unsup or recovered or advanced:
            print(
                f"  {model_id:8} {circle_only:>11} {old_unsup:>9} {recovered:>9} "
                f"{advanced:>8} {refused:>8} {balance:>7} {rendered:>9}"
            )

    print(
        f"  {'TOTAL':8} {totals['circle_only']:>11} {totals['old_unsup']:>9} "
        f"{totals['recovered']:>9} {totals['advanced']:>8} {totals['refused']:>8} "
        f"{totals['old_unsup'] - (totals['recovered'] + totals['advanced'] + totals['refused']):>7} "
        f"{totals['rendered']:>9}"
    )

    print()
    if unbalanced:
        print(f"!! {len(unbalanced)} model(s) do not balance: {unbalanced}")
    else:
        print("every model balances: old_unsupported = recovered + advanced + refused")
    if regressed:
        print(f"!! {len(regressed)} model(s) regressed or moved the gate-closed control: {regressed}")
    else:
        print("no model lost a rendered face; gate-closed control unchanged everywhere")

    print(
        f"\ncircle-only class was {totals['circle_only']}; "
        f"{totals['recovered'] + totals['advanced']} of the old refusals left the "
        f"curve exit ({totals['recovered']} recovered, {totals['advanced']} advanced)"
    )
    return 1 if unbalanced or regressed else 0


if __name__ == "__main__":
    raise SystemExit(main())
