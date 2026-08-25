//! BG-SOL-P0-PRED — the `CurveContact` ontology, defined once in 2-D and
//! reused by 3-D (docs/SOLVER_FAMILY_PLAN.md §2).
//!
//! Contact has `dimension` (0D point / 1D arc / 2D region) and event kinds
//! (transverse, tangency, endpoint touch, coincident interval, identical
//! carrier). Getting it right in S1 makes S5 conceptually familiar rather
//! than a second paradigm. The types live in `truck-base` so both S1
//! (`arrange` in truck-geometry) and the Contact Layer (`contact` in
//! truck-evidence) can name them without either depending on the other.
//!
//! The types are vocabulary only: construction through the pub fields, no
//! added methods; S1 refines the semantics.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

use serde::{Deserialize, Serialize};

/// The dimension of a curve-curve contact locus.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContactDimension {
    /// A single parameter pair (an isolated point contact).
    Point0,
    /// A one-dimensional contact (an arc of coincident curves).
    Arc1,
    /// A two-dimensional contact (a region; reserved for the 2-D overlap
    /// case in S1/S5.3).
    Region2,
}

/// The event kind of a curve-curve contact.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContactEventKind {
    /// Two curves cross at a point with distinct tangents.
    Transverse,
    /// The tangents agree at the contact point.
    Tangency,
    /// The contact is at the endpoint of one or both curves.
    EndpointTouch,
    /// The curves coincide over an interval (their images overlap).
    CoincidentInterval,
    /// The two curves share a carrier (provenance-identical).
    IdenticalCarrier,
}

/// A contact between two curves, defined once in 2-D and reused by 3-D
/// (plan §2). The parameter lists carry the contact locus on each curve in
/// its own parameterization: `Point0` has one entry per side; `Arc1` has the
/// interval endpoints per side; `Region2` is Phase-1 defined. The values here
/// are the solver's best certified parameters; the refined `Certified<...>`
/// forms land in S1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CurveContact {
    /// Contact locus dimension.
    pub dimension: ContactDimension,
    /// Contact event kind.
    pub kind: ContactEventKind,
    /// Contact locus parameters on the lhs curve (per `dimension`).
    pub params_lhs: Vec<f64>,
    /// Contact locus parameters on the rhs curve (per `dimension`).
    pub params_rhs: Vec<f64>,
}

#[cfg(test)]
// Test-only allow: H-1 bans unwrap/expect on paths reachable from untrusted
// geometry. Unit-test assertions on hand-built witnesses are not such a path;
// these unwraps cannot fire for the values constructed below. (`unwrap_used`
// stays denied here; no test below uses unwrap.)
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn curve_contact_types_construct_and_match() {
        let dimensions = [
            ContactDimension::Point0,
            ContactDimension::Arc1,
            ContactDimension::Region2,
        ];
        let kinds = [
            ContactEventKind::Transverse,
            ContactEventKind::Tangency,
            ContactEventKind::EndpointTouch,
            ContactEventKind::CoincidentInterval,
            ContactEventKind::IdenticalCarrier,
        ];
        for dimension in dimensions {
            for kind in kinds {
                let contact = CurveContact {
                    dimension,
                    kind,
                    params_lhs: vec![0.25],
                    params_rhs: vec![1.5],
                };
                assert_eq!(contact.clone(), contact);
                assert_eq!(contact.dimension, dimension);
                assert_eq!(contact.kind, kind);
                assert_eq!(contact.params_lhs, vec![0.25]);
                assert_eq!(contact.params_rhs, vec![1.5]);
            }
        }
    }
}
