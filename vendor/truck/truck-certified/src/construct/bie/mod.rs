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

//! The Certified Interaction Engine (BIE) contract shim (BIE-000-CONTRACT).
//!
//! The BIE program restricts the landed interaction pipeline to the pair
//! family `SpineFrameSweep × canonical` (plus the landed canonical × canonical
//! path) and requires a typed outcome vocabulary over that restricted pair.
//! This module is the shim the later waves (BIE-001..007) type against: it
//! freezes
//!
//! 1. **`InteractionOutcome`** — the program's typed outcome vocabulary,
//!    mapping onto the landed evidence taxonomy (`truck-base` §4). It has
//!    exactly three arms: a certified answer carrying its certificate value
//!    type, the three-valued unresolved verdict with the κ / cell / slope
//!    witness, and a landed refusal passed through unchanged.
//! 2. **The §8.1 carrier decision, recorded** (doc comment below).
//! 3. **The unit-shape fixture kit** ([`fixtures`]), whose closed-form ground
//!    truths later packets' tests are graded against.
//!
//! No solver body lives here — the shim ships refusing constructors and data
//! only (spine §3; "fixtures precede solvers").
//!
//! **§8.1 carrier decision — RECORDED, pre-decided, not relitigated.** The
//! procedural carrier that carries a certified restricted-pair intersection
//! from the solver into the topology is **`CertifiedImplicitIntersectionCurve`**:
//! a NEW canonical `Curve` variant in `truck-geometry/src/canonical.rs`,
//! landed by BIE-003 (NOT this packet), mirroring the landed
//! `Curve::IntersectionCurve` pattern (canonical.rs, `IntersectionCurve` at
//! `Curve`'s variant list) and carrying a certified 3-D polyline with
//! per-sample tangent frames plus the unresolved witness slot. The
//! PL-at-tessellation policy (`EdgeSampleLedger`-compatible; truck-meshalgo
//! is read-only for the program) means the polyline is realized only when the
//! certified edge is tessellated, never earlier. Tree evidence that the
//! decision is recordable as stated: the closed `Curve` enum in
//! `truck-geometry/src/canonical.rs` already carries the boxed
//! `IntersectionCurve` procedural variant, so the additive `Curve` variant
//! pattern is landed and extension is a routine enum ripple owned by BIE-003.
//! The mapping rows are booked in `docs/CERTIFICATE_MAPPING.md` section D.
//!
//! **Outcome mapping (spine §8).** `Unresolved { kappa, cell, slope }` maps
//! onto the LANDED `Refusal::NumericallyUnresolved` witness
//! (`truck-base/src/evidence.rs`, both sites) — zero new refusal arms. The
//! κ / cell / slope witness stays in the engine's own outcome vocabulary (the
//! restricted-pair solver's BIE-002 `Unresolved` verdict); the landed
//! projection preserves the refusal class for downstream routing. The witness
//! is `UnresolvedWitness::KrawczykIndeterminate`: the restricted-pair solver
//! raises an unresolved verdict exactly when its slicewise Krawczyk operator
//! proves neither existence nor absence on a box, which is the closest landed
//! arm of the same epistemic shape. A `Refused` arm is a real landed
//! `Refusal`, passed through unchanged.
//!
//! **H-1.** This module carries no `unwrap`, no `expect`, and no `panic!`, and
//! adds no module-level `allow`.
//!
//! **H-6.** A value computed in floats is never recorded as `Method::Exact`.
//! [`CertificateValue`] carries the producing `Method` explicitly; the fixture
//! kit tags every float-derived closed-form constant `Method::Float`.

use truck_base::evidence::{Budget, Method, Refusal, UnresolvedWitness};
use truck_geometry::prelude::Point3;

/// The restricted-pair interaction outcome (BIE program).
///
/// `Unresolved` maps onto the landed `Refusal::NumericallyUnresolved`
/// witness — zero new refusal arms (spine §8; a violation is a SPEC_GAP).
#[derive(Clone, Debug)]
pub enum InteractionOutcome {
    /// A certified answer carrying its certificate value type.
    Certified(CertificateValue),
    /// The three-valued verdict: unresolved with κ / cell / slope witness.
    Unresolved {
        /// The conditioning / curvature witness that kept the cell from a
        /// certified answer (the restricted-pair diagnostic).
        kappa: f64,
        /// The `(u, v) × (s, t)` parameter cell that stayed unresolved.
        cell: WitnessCell,
        /// The §5.4 slope diagnostic of the unresolved cell.
        slope: f64,
    },
    /// A landed typed refusal, passed through unchanged.
    Refused(Refusal),
}

impl InteractionOutcome {
    /// Projects the outcome onto the landed §4 refusal taxonomy.
    ///
    /// - `Certified` carries no refusal (`None`).
    /// - `Unresolved { kappa, cell, slope }` maps onto
    ///   `Refusal::NumericallyUnresolved` with a fresh budget and the
    ///   `KrawczykIndeterminate` witness (the module doc: the restricted-pair
    ///   solver raises an unresolved verdict when its slicewise Krawczyk
    ///   operator proves neither existence nor absence on a box). The
    ///   κ / cell / slope witness stays on the engine's own `Unresolved`
    ///   verdict — this projection records the refusal class only, for
    ///   routing through machinery that consumes the landed taxonomy.
    /// - `Refused` returns the landed refusal unchanged.
    pub fn into_landed_refusal(self) -> Option<Refusal> {
        match self {
            InteractionOutcome::Certified(_) => None,
            InteractionOutcome::Unresolved { .. } => Some(Refusal::NumericallyUnresolved {
                spent: Budget::new(0, 0, 0),
                witness: UnresolvedWitness::KrawczykIndeterminate,
            }),
            InteractionOutcome::Refused(refusal) => Some(refusal),
        }
    }
}

/// A landed typed refusal enters the engine outcome vocabulary as a passthrough
/// `Refused` arm (never re-mapped, never widened).
impl From<Refusal> for InteractionOutcome {
    fn from(refusal: Refusal) -> Self {
        InteractionOutcome::Refused(refusal)
    }
}

/// A certified scalar or point value, tagged with the §4 `Method` that
/// produced it (H-6: a float-derived value is `Method::Float`, never
/// `Method::Exact`).
///
/// This is the minimal value type the restricted-pair fixture kit and the
/// `InteractionOutcome::Certified` arm need: a certified scalar magnitude (a
/// section radius, a station, a conditioning value) or a certified 3-D point
/// (a section centre). There is deliberately no `From<f64>` / `From<Point3>`
/// impl — a `Certified` answer is never fabricated from raw floats without an
/// explicit `Method` (refusing-constructors discipline, spine §8).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CertificateValue {
    /// A certified scalar magnitude.
    Scalar {
        /// The scalar value.
        value: f64,
        /// The method that produced it (H-6).
        method: Method,
    },
    /// A certified 3-D point.
    Point {
        /// The point value.
        value: Point3,
        /// The method that produced it (H-6).
        method: Method,
    },
}

impl CertificateValue {
    /// Tags a scalar value with the method that produced it.
    pub const fn scalar(value: f64, method: Method) -> Self {
        CertificateValue::Scalar { value, method }
    }

    /// Tags a 3-D point with the method that produced it.
    pub const fn point(value: Point3, method: Method) -> Self {
        CertificateValue::Point { value, method }
    }

    /// The producing method (H-6).
    pub fn method(self) -> Method {
        match self {
            CertificateValue::Scalar { method, .. } | CertificateValue::Point { method, .. } => {
                method
            }
        }
    }

    /// The scalar payload, when this value is a scalar.
    pub fn scalar_value(self) -> Option<f64> {
        match self {
            CertificateValue::Scalar { value, .. } => Some(value),
            CertificateValue::Point { .. } => None,
        }
    }

    /// The point payload, when this value is a point.
    pub fn point_value(self) -> Option<Point3> {
        match self {
            CertificateValue::Point { value, .. } => Some(value),
            CertificateValue::Scalar { .. } => None,
        }
    }
}

/// A `(u, v) × (s, t)` parameter-cell record: the four scalar parameter
/// intervals of the product-domain cell the restricted-pair solver bisects and
/// reports when it cannot certify an answer. Each interval is an inclusive
/// `(lo, hi)` pair; the cell label convention is per-side and not
/// semantically load-bearing (the labels name the two carriers' parameter
/// boxes in a fixed order).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WitnessCell {
    /// The `u`-parameter interval.
    pub u: (f64, f64),
    /// The `v`-parameter interval.
    pub v: (f64, f64),
    /// The `s`-parameter interval.
    pub s: (f64, f64),
    /// The `t`-parameter interval.
    pub t: (f64, f64),
}

impl WitnessCell {
    /// Builds the parameter cell from its four intervals.
    ///
    /// The caller supplies finite, ordered `(lo, hi)` intervals; the cell is a
    /// data record (fixtures build it from dyadic, ordered data).
    pub const fn new(u: (f64, f64), v: (f64, f64), s: (f64, f64), t: (f64, f64)) -> Self {
        WitnessCell { u, v, s, t }
    }
}

/// The §3 unit-shape fixture kit of the BIE shim: closed-form carrier pairs
/// (plane × sphere, plane × cylinder, sweep × plane) with stated,
/// machine-checked ground truths, plus the whole-kit determinism record.
///
/// This module is `#[doc(hidden)] pub`: TEST SUPPORT ONLY, excluded from the
/// certified API surface, but reachable by BIE wave packets' integration tests
/// through the crate's public path (the `construct::fixtures` precedent).
#[doc(hidden)]
pub mod fixtures;

#[cfg(test)]
mod tests {
    use super::*;
    use truck_base::evidence::{Budget, Refusal, UnresolvedWitness};

    /// A real landed `Refusal` value used to exercise the passthrough arm.
    fn landed_refusal() -> Refusal {
        Refusal::NumericallyUnresolved {
            spent: Budget::new(1, 2, 3),
            witness: UnresolvedWitness::RootNotIsolated,
        }
    }

    #[test]
    fn interaction_outcome_maps_onto_landed_refusal() {
        // An `Unresolved { kappa, cell, slope }` verdict maps onto the landed
        // `Refusal::NumericallyUnresolved` witness (spine §8): zero new refusal
        // arms, a fresh budget, and the Krawczyk witness of the restricted-pair
        // solver's indeterminate box.
        let cell = WitnessCell::new((0.0, 1.0), (0.0, 1.0), (0.0, 1.0), (0.0, 1.0));
        let unresolved = InteractionOutcome::Unresolved {
            kappa: 1.0e-2,
            cell,
            slope: -3.0e-3,
        };
        let mapped = unresolved.into_landed_refusal();
        assert!(
            matches!(
                &mapped,
                Some(Refusal::NumericallyUnresolved { spent, witness })
                    if *spent == Budget::new(0, 0, 0)
                        && *witness == UnresolvedWitness::KrawczykIndeterminate
            ),
            "an Unresolved verdict must map onto the landed NumericallyUnresolved \
             witness, got {mapped:?}"
        );

        // The κ / cell / slope witness stays on the engine outcome: a
        // Certified answer carries no landed refusal.
        let certified = InteractionOutcome::Certified(CertificateValue::scalar(1.0, Method::Float));
        assert!(
            certified.into_landed_refusal().is_none(),
            "a Certified answer carries no landed refusal"
        );

        // A `Refused` passthrough round-trips a real landed `Refusal` value.
        let landed = landed_refusal();
        let outcome: InteractionOutcome = landed.clone().into();
        assert!(
            matches!(&outcome, InteractionOutcome::Refused(refusal) if matches!(
                refusal,
                Refusal::NumericallyUnresolved { spent, witness }
                    if *spent == Budget::new(1, 2, 3)
                        && *witness == UnresolvedWitness::RootNotIsolated
            )),
            "a landed refusal must enter as an unchanged Refused passthrough"
        );
        let round_trip = outcome.into_landed_refusal();
        assert!(
            matches!(
                &round_trip,
                Some(Refusal::NumericallyUnresolved { spent, witness })
                    if *spent == Budget::new(1, 2, 3)
                        && *witness == UnresolvedWitness::RootNotIsolated
            ),
            "a Refused passthrough must round-trip the exact landed value, got {round_trip:?}"
        );
    }
}
