#![cfg_attr(not(debug_assertions), deny(warnings))]
#![deny(clippy::all, rust_2018_idioms)]
#![deny(clippy::unwrap_used)]
#![warn(
    missing_docs,
    missing_debug_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unstable_features,
    unused_import_braces,
    unused_qualifications
)]

//! The T1.5(1) five-equation exclusion driver (CTE-003-MINORS; theory §2.7).
//!
//! The driver decides "the system `(F, M1, M2)` — five polynomial equations —
//! has no root in the box" by subdivision (scope decision 5). Per box it
//! FIRST tries single-equation interval separation (any of the five equations
//! certified away from zero over the box excludes — cheap pruning), THEN a
//! square-subsystem Krawczyk exclusion (any 4-of-5 subsystem with no root in
//! the box also excludes, because a common root of all five would be a root
//! of that subsystem), ELSE it bisects the widest axis, low-before-high,
//! spending one subdivision from the shared [`Budget`](truck_base::evidence::Budget).
//!
//! An exhausted budget at a non-excluded, non-degenerate box is
//! [`ExclusionEvidence::Inconclusive`] carrying the cell — never a guess. The
//! subsystem attempt order is FIXED: skip-one in ascending equation index
//! order, deterministically. The landed Krawczyk operator is instantiated on
//! the subsystem rows (never extended); `num/krawczyk.rs` is not edited.
//!
//! **H-1.** No unwrap, expect, panic or out-of-range indexing is reachable
//! from geometry.
//!
//! **H-6.** The driver's certificate is the box-level exclusion statement;
//! float values never enter an [`ExclusionEvidence`].

use crate::formal::exact::CertifiedInterval;
use crate::hull::HullRefusal;
use crate::tangency::minors::{from_net, from_system_grid, PolyGrid};
use crate::tangency::shapes::{ChartMinorGrids, Rank2Chart};
use crate::tangency::tsystem::TSystem;
use crate::tangency::TangencyRefusal;
use crate::SquareSystem3;
use truck_base::evidence::Budget;
use truck_evidence::enclosure::Interval;
use truck_evidence::num::krawczyk::{krawczyk, KrawczykProof};

/// A four-axis box in the unit chart.
pub type Box4 = [(f64, f64); 4];

/// The T1.5 exclusion driver's only outputs (theory §2.7; spine §2).
///
/// `NoRootFiveEq` certifies that `(F, M1, M2)` has no root in the searched
/// region (T1.5.1: the box is transversal). `CertifiedCriticalPoint` is the
/// reserved witness of the deflated-root route (T1.5.2 — produced by the
/// cascade stage that certifies a unique deflated root). `Inconclusive`
/// carries the cell that resisted certification after the budget was spent —
/// never a guess, never a string.
#[derive(Debug, Clone, PartialEq)]
pub enum ExclusionEvidence {
    /// The five-equation system `(F, M1, M2)` has no root in the searched
    /// region (T1.5.1).
    NoRootFiveEq {
        /// The subdivisions spent since the driver's entry.
        spend: Budget,
    },
    /// A certified unique deflated root candidate in the region (T1.5.2
    /// route; the root enclosure).
    CertifiedCriticalPoint(Box4),
    /// The budget was exhausted on a non-excluded, non-degenerate cell.
    Inconclusive {
        /// The cell that resisted certification.
        cell: Box4,
    },
}

/// The budget spent since entry (initial minus remaining), matching the
/// landed operator's reporting convention.
fn spent(initial: &Budget, budget: &Budget) -> Budget {
    Budget {
        subdiv: initial.subdiv - budget.subdiv,
        newton: initial.newton - budget.newton,
        depth: initial.depth - budget.depth,
    }
}

/// Whether a certified interval excludes zero STRICTLY.
fn excludes_zero(enc: &CertifiedInterval) -> bool {
    enc.is_finite() && !(enc.lo <= 0.0 && 0.0 <= enc.hi)
}

/// The five stored rows `(F0, F1, F2, M1, M2)` as unit-chart polynomial grids.
fn five_rows(
    system: &SquareSystem3,
    minors: &ChartMinorGrids,
) -> Result<[PolyGrid; 5], TangencyRefusal> {
    let f0 = from_system_grid(&system.grids()[0], system.degrees())?;
    let f1 = from_system_grid(&system.grids()[1], system.degrees())?;
    let f2 = from_system_grid(&system.grids()[2], system.degrees())?;
    let m1 = from_net(minors.m1())?;
    let m2 = from_net(minors.m2())?;
    Ok([f0, f1, f2, m1, m2])
}

/// The four rows of a 4-of-5 subsystem: every row except `skip` (ascending).
fn subsystem_rows(rows: &[PolyGrid; 5], skip: usize) -> [PolyGrid; 4] {
    let mut selected: Vec<&PolyGrid> = Vec::with_capacity(4);
    for (i, row) in rows.iter().enumerate() {
        if i != skip {
            selected.push(row);
        }
    }
    [
        selected[0].clone(),
        selected[1].clone(),
        selected[2].clone(),
        selected[3].clone(),
    ]
}

/// A unit-chart box as an interval box, `None` on a non-finite or misordered
/// axis.
fn to_interval_box(b: &Box4) -> Option<[Interval; 4]> {
    let mut out = [Interval::EMPTY; 4];
    for (axis, cell) in out.iter_mut().enumerate() {
        let (lo, hi) = b.get(axis)?;
        if !lo.is_finite() || !hi.is_finite() || lo > hi {
            return None;
        }
        *cell = Interval::try_from((*lo, *hi)).ok()?;
    }
    Some(out)
}

/// The widest axis of a box (ties toward the lowest index) and its midpoint
/// as a convex combination. `None` when the widest axis cannot be bisected in
/// `f64` (a degenerate or at-resolution box).
fn bisect_axis(b: &Box4) -> Option<(usize, f64)> {
    let mut axis = 0usize;
    let mut width = f64::NEG_INFINITY;
    let mut a = 0.0;
    let mut b_hi = 0.0;
    for (i, cell) in b.iter().enumerate() {
        let (lo, hi) = *cell;
        let w = hi - lo;
        if w.total_cmp(&width).is_gt() || (w.total_cmp(&width).is_eq() && i < axis) {
            axis = i;
            width = w;
            a = lo;
            b_hi = hi;
        }
    }
    if width == 0.0 {
        return None;
    }
    let mid = 0.5 * a + 0.5 * b_hi;
    if mid == a || mid == b_hi {
        return None;
    }
    Some((axis, mid))
}

/// The core recursion of [`exclude_five`]: decide no-root on `b`, subdividing
/// under the shared budget.
fn exclude_rec(
    rows: &[PolyGrid; 5],
    b: &Box4,
    initial: &Budget,
    budget: &mut Budget,
) -> Result<ExclusionEvidence, TangencyRefusal> {
    // 1. Cheap pruning: any single equation separated from zero excludes.
    for row in rows.iter() {
        let enc = row.hull_unit(*b)?;
        if excludes_zero(&enc) {
            return Ok(ExclusionEvidence::NoRootFiveEq {
                spend: spent(initial, budget),
            });
        }
    }
    // 2. Square-subsystem Krawczyk exclusion, skip-one ascending.
    let start = to_interval_box(b).ok_or(TangencyRefusal::Hull(HullRefusal::DomainNotCompact))?;
    for skip in 0..5 {
        let sub = TSystem::from_rows(subsystem_rows(rows, skip));
        if let Ok(certified) = krawczyk(&sub, &start, budget) {
            if certified.value == KrawczykProof::NoRoot {
                return Ok(ExclusionEvidence::NoRootFiveEq {
                    spend: spent(initial, budget),
                });
            }
        }
    }
    // 3. No exclusion on this box: bisect the widest axis, low-before-high.
    let (axis, mid) = match bisect_axis(b) {
        Some(parts) => parts,
        None => return Ok(ExclusionEvidence::Inconclusive { cell: *b }),
    };
    if budget.spend_subdiv(1).is_err() {
        return Ok(ExclusionEvidence::Inconclusive { cell: *b });
    }
    let mut low = *b;
    let mut high = *b;
    low[axis] = (low[axis].0, mid);
    high[axis] = (mid, high[axis].1);
    let low_out = exclude_rec(rows, &low, initial, budget)?;
    match low_out {
        ExclusionEvidence::NoRootFiveEq { .. } => {}
        other => return Ok(other),
    }
    exclude_rec(rows, &high, initial, budget)
}

/// Decide "the five-equation system `(F, M1, M2)` has no root in `b`" by
/// subdivision (theory T1.5.1, scope decision 5).
///
/// Per box: single-equation interval separation first, then the 4-of-5
/// square-subsystem Krawczyk exclusion in ascending skip order, else bisect
/// (widest axis, low-before-high) under `budget`. An exhausted budget at a
/// non-excluded, non-degenerate box is [`ExclusionEvidence::Inconclusive`].
///
/// `b` is a unit-chart box of the stored system. The chart contextualizes the
/// minors (the driver's rows are the chart-expressed polynomials of `system`
/// and `minors`).
pub fn exclude_five(
    system: &SquareSystem3,
    minors: &ChartMinorGrids,
    _chart: &Rank2Chart,
    b: &Box4,
    budget: &mut Budget,
) -> Result<ExclusionEvidence, TangencyRefusal> {
    let initial = *budget;
    let rows = five_rows(system, minors)?;
    exclude_rec(&rows, b, &initial, budget)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tangency::minors::build_chart_minors;
    use crate::tangency::minors::support::{f1_pivot, f1_system, f6_system, template_chart};
    use crate::tangency::shapes::{Rank2Chart, Rank2Pivot};

    /// A fully built chart for a system and pivot (template chart → real
    /// minors → real chart). `det_lo/det_hi` is the recorded `det A`
    /// enclosure of the chart's pivot over the chart's box (0 excluded).
    fn build_chart(
        system: &SquareSystem3,
        pivot: Rank2Pivot,
        det_lo: f64,
        det_hi: f64,
    ) -> Rank2Chart {
        let template = template_chart(pivot, det_lo, det_hi).expect("the template chart admits");
        let minors = build_chart_minors(system, &template).expect("the chart minors build");
        Rank2Chart::new(
            pivot,
            CertifiedInterval {
                lo: det_lo,
                hi: det_hi,
            },
            minors,
        )
        .expect("the chart admits")
    }

    #[test]
    fn t15_excludes_transversal_fixture_f6() {
        let system = f6_system().expect("the F6 system admits");
        let pivot = Rank2Pivot::new(2, (0, 1)).expect("the F6 pivot admits");
        let chart = build_chart(&system, pivot, 0.5, 1.5);
        let minors = chart.minors().clone();
        // A box straddling the planes' intersection line (which sits strictly
        // inside the unit chart at s = u = 1/2).
        let b: Box4 = [(0.0, 1.0); 4];
        let mut budget = Budget::new(64, 0, 0);
        let out = exclude_five(&system, &minors, &chart, &b, &mut budget)
            .expect("the F6 box excludes cleanly");
        assert!(
            matches!(out, ExclusionEvidence::NoRootFiveEq { .. }),
            "a transversal crossing box must be certified root-free, got {:?}",
            out
        );
    }

    #[test]
    fn t15_refusal_is_named() {
        let system = f1_system().expect("the F1 system admits");
        let pivot = f1_pivot().expect("the F1 pivot admits");
        let chart = build_chart(&system, pivot, 0.5, 1.5);
        let minors = chart.minors().clone();
        // The F1 critical point (0, 0, 0, 0) IS a root of (F, M1, M2): the box
        // around it can never be excluded, so the subdivision must exhaust the
        // budget and refuse with the named Inconclusive cell — never a string,
        // never a guess.
        let b: Box4 = [(0.0, 0.5); 4];
        let mut budget = Budget::new(12, 0, 0);
        let out = exclude_five(&system, &minors, &chart, &b, &mut budget)
            .expect("budget exhaustion yields the named refusal");
        match out {
            ExclusionEvidence::Inconclusive { cell } => {
                assert!(cell
                    .iter()
                    .all(|(lo, hi)| lo.is_finite() && hi.is_finite() && lo <= hi));
                // The exhausted cell still contains the critical point.
                assert!(cell[0].0 <= 0.0 && 0.0 <= cell[0].1);
                assert!(cell[1].0 <= 0.0 && 0.0 <= cell[1].1);
                assert!(cell[2].0 <= 0.0 && 0.0 <= cell[2].1);
                assert!(cell[3].0 <= 0.0 && 0.0 <= cell[3].1);
            }
            other => panic!(
                "a box containing the F1 critical point must end Inconclusive, got {:?}",
                other
            ),
        }
    }
}
