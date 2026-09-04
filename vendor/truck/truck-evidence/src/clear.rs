//! CC-004-CLEAR — the P5 ball-clearance predicate.
//!
//! Theory `docs/CERTIFIED_LOFT_AND_SHELL_THEORY_SPEC.md` §1 P5, per spine S7
//! (amended: `mu` is an explicit parameter because `truck-evidence` cannot
//! read `construct/config.rs`; the ball CENTRE is an explicit box for the same
//! reason the S7 seam text gives — the caller passes the centre region inside
//! the exclusion box's coordinate frame, and the ground-truth cases (ball at
//! the origin vs. ball displaced along `z`) are only expressible through it).
//!
//! `Clear` holds iff the contact ball of radius `r` is farther than `mu` from
//! the excluded boundary region AND the mode's containment side holds:
//! `Round` (negative-inside convention of [`ImplicitField`]) requires
//! `field <= 0` over the ball, `Fillet` requires `field >= 0` over it. Both
//! sides are decided in interval arithmetic over the INPUT boxes — never by
//! widening, never by an internal retry (higher precision is the caller's
//! escalation; the theory §9 retry rule lives above this layer).
//!
//! Both sides return a three-way verdict — certified Clear, certified
//! Rejected, or Undecided:
//!
//! - separation: the ball is enclosed by the box built from the caller's
//!   centre box grown by `r` on every axis (`centre ± r`), and the six-line
//!   axis-gap formula (`box_distance`, ported from `truck-base`'s BVH) lower
//!   bounds the ball-to-exclusion distance at the box level. A lower bound
//!   strictly above `mu` certifies Clear; a `0` gap certifies that the ball's
//!   own box meets the excluded region (Rejected — the box as a whole
//!   intrudes, so the configuration is inadmissible); any gap in `(0, mu]`
//!   straddles the margin and is Undecided.
//!
//! - containment: the carrier's implicit field over the ball box, `Round`
//!   Clear iff its upper endpoint is `<= 0` and `Fillet` Clear iff its lower
//!   endpoint is `>= 0`; the opposite strict side is Rejected; an enclosure
//!   straddling zero is Undecided.
//!
//! True on both sides → `Ok(true)`; Rejected on either → `Ok(false)`;
//! Undecided on either → `Err(Refusal::NumericallyUnresolved)` with witness
//! [`UnresolvedWitness::UncertifiedContainment`]. The refusal carries a zero
//! spent ledger: this layer performs one decisive interval evaluation per side
//! and never subdivides, so nothing is spent here (the zero-spend-with-witness
//! pattern of `num/cluster.rs`).
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

use crate::contact::implicit::ImplicitField;
use crate::enclosure::{Box3, Interval};
use truck_base::evidence::{Budget, Refusal, UnresolvedWitness};

/// Which containment side of the implicit field the contact ball must satisfy.
///
/// The carriers' sign convention is negative-inside (documented per impl in
/// `contact/implicit.rs`): the zero set is the surface and the interior of the
/// canonical solid is where `field < 0`. `Round` places the ball in that
/// interior (`field <= 0`), `Fillet` on the exterior side (`field >= 0`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BallAdmissibility {
    /// The ball must lie where the field is non-negative.
    Fillet,
    /// The ball must lie where the field is non-positive.
    Round,
}

/// One side's three-way verdict.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Side {
    /// The side certifies Clear.
    Clear,
    /// The side certifies a violation: the configuration is inadmissible.
    Rejected,
    /// The interval test straddles the decision threshold.
    Undecided,
}

/// The P5 clearance predicate: whether a contact ball of radius `r` whose
/// centre lies anywhere in `centre` is admissible against the excluded region
/// `exclusion` under `mode`.
///
/// `mu` is the clearance margin: `Clear` needs the ball farther than `mu` from
/// `exclusion`, which is the region to stay away from — it does NOT contain
/// the ball. The ball is enclosed by its own axis-aligned box built from the
/// caller's centre box `± r`; every conclusion is certified at that box level
/// over the WHOLE family of placements (all centres in `centre`, all radii in
/// `r`), so `Ok(true)` guarantees every member of the family is clear.
/// `f64::INFINITY`-endpoint intervals are fine for an unbounded half-space
/// exclusion; the axis-gap formula only reads finite comparisons.
pub fn ball_clearance(
    field: &impl ImplicitField,
    centre: &Box3,
    exclusion: &Box3,
    r: Interval,
    mu: f64,
    mode: BallAdmissibility,
) -> Result<bool, Refusal> {
    let ball = ball_box(centre, r);
    let separation = separation_side(&ball, exclusion, mu);
    let containment = containment_side(field, &ball, mode);
    match (separation, containment) {
        (Side::Rejected, _) | (_, Side::Rejected) => Ok(false),
        (Side::Undecided, _) | (_, Side::Undecided) => Err(Refusal::NumericallyUnresolved {
            spent: Budget::new(0, 0, 0),
            witness: UnresolvedWitness::UncertifiedContainment,
        }),
        (Side::Clear, Side::Clear) => Ok(true),
    }
}

/// The axis-aligned box enclosing every ball `{ c ± ρ : c ∈ centre, ρ ∈ r }`:
/// the per-axis hull of `centre ± r`. Computed as the hull of the two interval
/// grows so any radius interval (including a non-degenerate one) is enclosed
/// without extra rounding.
fn ball_box(centre: &Box3, r: Interval) -> Box3 {
    let axis = |v: Interval| -> Interval {
        let grew_high = v + r;
        let grew_low = v - r;
        let lo = grew_low.inf().min(grew_high.inf());
        let hi = grew_low.sup().max(grew_high.sup());
        Interval::try_from((lo, hi)).unwrap_or(Interval::EMPTY)
    };
    Box3 {
        x: axis(centre.x),
        y: axis(centre.y),
        z: axis(centre.z),
    }
}

/// Section (a): the separation side. Ports the six-line axis-gap distance of
/// `truck-base`'s BVH onto the inari `Box3` level: per-axis
/// `max(0, lo_b − hi_a, lo_a − hi_b)`, Euclidean-combined, is a certified
/// lower bound on the distance between the ball's enclosing box and the
/// exclusion box, hence on every real ball's distance to the exclusion.
fn separation_side(ball: &Box3, exclusion: &Box3, mu: f64) -> Side {
    let d = box_distance(ball, exclusion);
    if d > mu {
        Side::Clear
    } else if d == 0.0 {
        Side::Rejected
    } else {
        Side::Undecided
    }
}

/// Section (b): the containment side, against the documented sign convention
/// of the carrier (read from the `ImplicitField` impls, never assumed).
fn containment_side(field: &impl ImplicitField, ball: &Box3, mode: BallAdmissibility) -> Side {
    let f = field.implicit(ball);
    match mode {
        BallAdmissibility::Round => {
            if f.sup() <= 0.0 {
                Side::Clear
            } else if f.inf() > 0.0 {
                Side::Rejected
            } else {
                Side::Undecided
            }
        }
        BallAdmissibility::Fillet => {
            if f.inf() >= 0.0 {
                Side::Clear
            } else if f.sup() < 0.0 {
                Side::Rejected
            } else {
                Side::Undecided
            }
        }
    }
}

/// A lower bound on the point-set distance between two boxes: per-axis
/// `max(lo_b − hi_a, lo_a − hi_b)` clamped at 0, Euclidean-combined.
fn box_distance(a: &Box3, b: &Box3) -> f64 {
    let gap = |lo_a: f64, hi_a: f64, lo_b: f64, hi_b: f64| (lo_b - hi_a).max(lo_a - hi_b).max(0.0);
    let dx = gap(a.x.inf(), a.x.sup(), b.x.inf(), b.x.sup());
    let dy = gap(a.y.inf(), a.y.sup(), b.y.inf(), b.y.sup());
    let dz = gap(a.z.inf(), a.z.sup(), b.z.inf(), b.z.sup());
    (dx * dx + dy * dy + dz * dz).sqrt()
}
