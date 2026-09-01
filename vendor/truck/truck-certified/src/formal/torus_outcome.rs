//! The typed exit of the torus annulus recovery route.
//!
//! The torus route combines three formal stages — circle-on-torus certification
//! ([`super::torus_circle::certify_circle_on_torus`]), annular cell certification
//! ([`super::torus_cell::certify_torus_annular_cell`]), and cut-open realization
//! ([`super::torus_realize::realize_torus_annulus`]) — and its exit vocabulary
//! names the *outcome* of that combined pipeline, not the exit of any one stage.
//! Each variant maps to exactly one category of the corrected torus census, so
//! an observer can reproduce the census from the outcome alone.
//!
//! # The typed distinctions
//!
//! The distinctions are kept apart because they have different remedies and
//! different implications for the face's geometry:
//!
//! - [`Self::NotEligible`]: the face is not a two-complete-circle torus annulus.
//!   This is not a defect in the face; it is a statement about the route's scope.
//! - [`Self::IncompatibleLoopWindings`]: the two loops are not homologous
//!   primitive parallels or meridians. The face may still be a valid torus
//!   region, just not one this theorem covers.
//! - [`Self::InconsistentBoundaryHomology`]: same effective orientations — the
//!   boundary chain is `2h ≠ 0` and cannot bound either annulus.
//! - [`Self::IntersectingBoundaries`]: the two loops coincide on the torus.
//! - [`Self::CircleNotOnTorus`]: a loop was proved not to lie on the torus.
//! - [`Self::CertificationFailure`]: a required proposition could not be
//!   established (e.g. winding unresolved, material authority undecidable).
//! - [`Self::RealizationFailure`]: the cell was certified but the mesh
//!   realization failed (Euler characteristic, boundary count, cut-pair mismatch).
//! - [`Self::OperationalFailure`]: a machine failure (degenerate parameters,
//!   non-finite values), not a geometric judgment.

/// Why a torus annulus was not recovered on one eligible face.
///
/// Each variant is a terminal category in the recovery pipeline. The mapping
/// from the formal stage exits to these variants is performed by
/// `run_torus_annulus_for_face` in `triangulation.rs`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TorusAnnulusExit {
    /// The face is not a two-complete-circle torus annulus: not a toroidal
    /// surface, not exactly two boundary components, an extra source edge,
    /// or a boundary loop that is not a complete source circle.
    NotEligible,
    /// The two loops are not homologous primitive parallels or meridians:
    /// different unsigned primitive classes, a nonprimitive winding, or a
    /// diagonal/other primitive class outside the two-complete-circle
    /// parallel/meridian annulus theorem.
    IncompatibleLoopWindings,
    /// Same effective orientations: the boundary chain is `2h ≠ 0`, not
    /// null-homologous; it cannot bound either complementary annulus.
    InconsistentBoundaryHomology,
    /// The two loops coincide on the torus (same developed coordinate mod `2π`).
    IntersectingBoundaries,
    /// A loop was proved not to lie on the torus by the whole-interval Fourier
    /// test.
    CircleNotOnTorus,
    /// A required proposition could not be established: the on-torus test was
    /// unresolved, the winding could not be certified, or the material
    /// authority was undecidable.
    CertificationFailure,
    /// The cell was certified but the mesh realization failed: wrong Euler
    /// characteristic, wrong boundary component count, mismatched cut pair,
    /// or a disconnected mesh.
    RealizationFailure,
    /// A machine failure: degenerate circle parameters, non-finite values.
    /// Not a geometric judgment.
    OperationalFailure,
}

impl TorusAnnulusExit {
    /// A short stable tag, for probe records and census ledgers.
    pub fn tag(self) -> &'static str {
        match self {
            Self::NotEligible => "torus_not_eligible",
            Self::IncompatibleLoopWindings => "torus_incompatible_loop_windings",
            Self::InconsistentBoundaryHomology => "torus_inconsistent_boundary_homology",
            Self::IntersectingBoundaries => "torus_intersecting_boundaries",
            Self::CircleNotOnTorus => "torus_circle_not_on_torus",
            Self::CertificationFailure => "torus_certification_failure",
            Self::RealizationFailure => "torus_realization_failure",
            Self::OperationalFailure => "torus_operational_failure",
        }
    }
}
