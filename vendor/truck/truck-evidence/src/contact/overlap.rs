//! BG-SOL-S7-OVERLAP — the 2-D overlap screen.
//!
//! The coincident paths of the Contact Layer (the struct-equal identity arms
//! and the analytic `Coincident` cells) used to emit Region2/Arc1 records
//! without screening the parameter boxes, so two disjoint patches of the same
//! canonical carrier reported contact. This module is the screen: parameter-
//! box interior overlap decides Coincident-vs-empty.
//!
//! Every decision here is exact-f64 arithmetic on stored analytic data — the
//! BG-ANA-002 5.1 decision class already used by `parallel_cylinders`' exact
//! radius equality. Sub-ulp boundary configurations may decide either way,
//! and the test witnesses are dyadic.
//!
//! Boundary-only contact (boxes touching at an edge or corner, interiors
//! disjoint) is intentionally EMPTY here: shared-boundary contact is owned by
//! the FE/EE stages over their own strata pairs.
//!
//! The plane × plane screen (in `contact/mod.rs`) solves the parameter
//! correspondence by Cramer and screens only the PARALLEL-frame signature
//! (`M[0][1] == 0.0` and `M[1][0] == 0.0`, exactly zero for construction data
//! whose frames are exact multiples). Rotated-frame coplanar planes are
//! deliberately NOT screened here: today's emission is kept and the decision
//! is deferred to the booked `BG-SOL-S7-OVERLAP-PLANE` follow-up (3-D SAT).
//!
//! House rules H-1..H-8 apply.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

/// Strict interior overlap of two aperiodic intervals.
///
/// `interior_overlap(a, b) = a.0 < b.1 && b.0 < a.1`. Boundary-touching at an
/// endpoint is not overlap. The periodic screen gives degenerate intervals
/// empty interior by construction (`circle_arcs` drops width `<= 0`), and the
/// aperiodic formula answers "strict interior intersection of the two open
/// intervals" exactly as the packet prescribes.
pub(crate) fn interior_overlap(a: (f64, f64), b: (f64, f64)) -> bool {
    a.0 < b.1 && b.0 < a.1
}

/// Strict interior overlap of two intervals on a circle of the given period.
///
/// Each interval wraps into `[0, period)` as at most two arcs, and any pair of
/// arcs with strict interior overlap decides true. An interval whose width is
/// `>= period` covers the whole circle. The wrap mirrors `fe_ee.rs`'s
/// convention of folding circle angles into `[0, TAU)`; the period is
/// `std::f64::consts::TAU` everywhere it appears in the screens.
pub(crate) fn periodic_interior_overlap(a: (f64, f64), b: (f64, f64), period: f64) -> bool {
    let arcs_a = circle_arcs(a, period);
    let arcs_b = circle_arcs(b, period);
    arcs_a
        .iter()
        .any(|x| arcs_b.iter().any(|y| interior_overlap(*x, *y)))
}

/// The arcs of an interval on a circle of the given period.
///
/// An interval of width `>= period` covers the whole circle (one arc spanning
/// `[0, period)`); a degenerate or reversed interval has empty interior and
/// contributes no arcs; otherwise the interval wraps into at most two arcs in
/// `[0, period)`.
fn circle_arcs(i: (f64, f64), period: f64) -> Vec<(f64, f64)> {
    let (lo, hi) = i;
    let width = hi - lo;
    if width >= period {
        return vec![(0.0, period)];
    }
    if width <= 0.0 {
        return Vec::new();
    }
    let lo_wrapped = lo.rem_euclid(period);
    let hi_wrapped = lo_wrapped + width;
    if hi_wrapped <= period {
        vec![(lo_wrapped, hi_wrapped)]
    } else {
        vec![(lo_wrapped, period), (0.0, hi_wrapped - period)]
    }
}

#[cfg(test)]
// Test-only allow: H-1 bans unwrap/expect/panic on paths reachable from
// untrusted geometry. Unit-test assertions on hand-built dyadic witnesses are
// not such a path.
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use std::f64::consts::TAU;

    #[test]
    fn interior_overlap_is_strict_on_both_ends() {
        assert!(interior_overlap((0.0, 1.0), (0.5, 1.5)));
        assert!(interior_overlap((0.5, 1.5), (0.0, 1.0)));
        assert!(
            !interior_overlap((0.0, 1.0), (1.0, 2.0)),
            "endpoint touch is empty"
        );
        assert!(
            !interior_overlap((1.0, 2.0), (0.0, 1.0)),
            "endpoint touch is empty"
        );
        assert!(
            !interior_overlap((0.0, 1.0), (2.0, 3.0)),
            "disjoint is empty"
        );
        assert!(
            !interior_overlap((0.5, 0.5), (0.5, 0.5)),
            "two degenerate intervals never overlap"
        );
    }

    #[test]
    fn periodic_wrap_joins_the_seam_arcs() {
        // (TAU - 0.1, TAU + 0.1) wraps onto (0, 0.1) ∪ (TAU-0.1, TAU); the
        // near-seam interval (0.05, 0.2) overlaps the low arc while (3.0, 3.1)
        // stays disjoint.
        assert!(periodic_interior_overlap(
            (0.05, 0.2),
            (TAU - 0.1, TAU + 0.1),
            TAU
        ));
        assert!(periodic_interior_overlap(
            (TAU - 0.1, TAU + 0.1),
            (0.05, 0.2),
            TAU
        ));
        assert!(!periodic_interior_overlap(
            (3.0, 3.1),
            (TAU - 0.1, TAU + 0.1),
            TAU
        ));
        // A whole-circle interval overlaps everything; a degenerate one nothing.
        assert!(periodic_interior_overlap((0.0, TAU), (0.1, 0.2), TAU));
        assert!(!periodic_interior_overlap((0.5, 0.5), (0.0, TAU), TAU));
    }
}
