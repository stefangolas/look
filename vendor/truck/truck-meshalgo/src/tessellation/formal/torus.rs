//! Certified identification of an embedded torus support surface (rank 2).
//!
//! # Scope
//!
//! The rank-2 analog of [`super::cylinder`]'s cylinder identification. A STEP
//! `toroidal_surface` reaches the tessellator as `Processor<Torus, Matrix4>`
//! (see `truck-stepio`'s `step_geometry::mod`: `ToroidalSurface =
//! Processor<Torus, Matrix4>`, constructed by `Torus::new(center, major,
//! minor)`). [`identify_torus`] reads the inner [`Torus`] *structurally* and
//! either certifies a rank-two deck or refuses with a named reason.
//!
//! This is the certified rank-two deck solver that [`super::quotient`] leaves
//! as `DeckPlacementResult::Unsupported` ("until a certified solver exists").
//! It supplies the truck-side [`CertifiedRankTwoDeck`] the torus outcome path
//! will consume; the look-side adapter (`look::step::torus_deck`) applies the
//! STEP placement transform and may re-certify in world space.
//!
//! # The parameter convention
//!
//! `Torus::subs(u, v)` (see `truck-geometry::specifieds::torus`) is
//! `center + (large + small*cos v)*(cos u, sin u, 0) + small*sin v*(0,0,1)`, so:
//!
//! - `u` is the **azimuthal** (major) angle, period `2π`, sweeping the tube
//!   center circle in the `z = center.z` plane,
//! - `v` is the **poloidal** (minor) angle, period `2π`, sweeping the tube
//!   cross-section,
//! - the symmetry axis is the `z` axis in the canonical (untransformed) frame.
//!
//! Both parameters are periodic, so the deck group is rank two: one generator
//! per period, each `2π`. The convention is **verified by evaluation** rather
//! than trusted from the type, exactly as [`super::cylinder::identify_cylinder`]
//! verifies its single angular period.
//!
//! # Regular torus only
//!
//! Only a *regular ring torus* (`large_radius > small_radius > 0`) is
//! certified. A spindle torus (`small > large`) self-intersects and a horn
//! torus (`small == large`) is singular at the centre; neither is a regular
//! surface with a free rank-two deck action, so both are refused.

use super::deck::{DeckConstructorFailure, DeckGenerator, DevelopedAxis};
use super::numeric::{FiniteF64, NumericDomainError, PositiveFinite};
use std::f64::consts::TAU;
use truck_geometry::prelude::{InnerSpace, Matrix4, ParametricSurface, Point3, Torus, Vector3};

/// Dimensionless residual below which the `2π` period is certified by
/// evaluation. `sin`/`cos` of `t + 2π` disagree with `sin`/`cos` of `t` at the
/// `f64::EPSILON` scale because `2π` is irrational; `1e-9` is six orders clear
/// of that and matches [`super::cylinder::MINIMUM_CYLINDER_LINE_AXIS_PARALLELISM`].
pub const MINIMUM_TORUS_PERIOD_RESIDUAL: f64 = 1e-9;

/// Why a revolved surface could not be certified as a regular embedded torus.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TorusIdentificationFailure {
    /// A coordinate or radius was `NaN` or infinite.
    NonFiniteCoordinate {
        /// The failing coordinate's domain error.
        cause: NumericDomainError,
    },
    /// A radius was not strictly positive.
    DegenerateRadius,
    /// The symmetry axis was zero (a degenerate placement).
    DegenerateAxis,
    /// `small_radius >= large_radius`: a spindle or horn torus, not a regular
    /// ring torus.
    SpindleOrHornTorus,
    /// The `2π` period could not be verified by evaluation on either axis.
    UnverifiedPeriod,
}

/// The structural data of a certified embedded torus.
#[derive(Debug, Clone, PartialEq)]
pub struct TorusSchema {
    center: Point3,
    axis: Vector3,
    large_radius: PositiveFinite,
    small_radius: PositiveFinite,
    /// The azimuthal (major, `u`) period generator.
    major_generator: DeckGenerator,
    /// The poloidal (minor, `v`) period generator.
    minor_generator: DeckGenerator,
}

impl TorusSchema {
    /// The torus centre.
    pub fn center(&self) -> Point3 {
        self.center
    }
    /// The symmetry axis (the canonical `z` axis for an untransformed `Torus`).
    pub fn axis(&self) -> Vector3 {
        self.axis
    }
    /// The major (azimuthal) radius, proved strictly positive.
    pub fn large_radius(&self) -> PositiveFinite {
        self.large_radius
    }
    /// The minor (poloidal) radius, proved strictly positive.
    pub fn small_radius(&self) -> PositiveFinite {
        self.small_radius
    }
    /// The azimuthal deck generator (`2π` on the major axis).
    pub fn major_generator(&self) -> DeckGenerator {
        self.major_generator
    }
    /// The poloidal deck generator (`2π` on the minor axis).
    pub fn minor_generator(&self) -> DeckGenerator {
        self.minor_generator
    }
}

/// The validity obligations discharged for a certified torus.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TorusValidityCertificate {
    discharged: bool,
}

impl TorusValidityCertificate {
    /// Constructed only by [`identify_torus`].
    fn discharged() -> Self {
        Self { discharged: true }
    }
}

/// A certified rank-two deck: two independent `2π` period generators on
/// distinct developed axes, the evidence a torus atlas cell needs.
///
/// Deliberately a truck-side type (the tessellator cannot depend on `look`),
/// mirroring [`super::cylinder::CertifiedEmbeddedCylinder`]. The two generators
/// are certified independent by construction: they translate distinct developed
/// coordinates, so the deck group they generate is `2π Z × 2π Z ≅ Z²`.
#[derive(Debug, Clone, PartialEq)]
pub struct CertifiedRankTwoDeck {
    schema: TorusSchema,
    certificate: TorusValidityCertificate,
}

impl CertifiedRankTwoDeck {
    /// The structural schema.
    pub fn schema(&self) -> &TorusSchema {
        &self.schema
    }

    /// The validity obligations discharged.
    pub fn certificate(&self) -> &TorusValidityCertificate {
        &self.certificate
    }

    /// Both deck generators, major then minor.
    pub fn deck_generators(&self) -> [DeckGenerator; 2] {
        [self.schema.major_generator, self.schema.minor_generator]
    }

    /// The azimuthal (major) deck generator.
    pub fn major_generator(&self) -> DeckGenerator {
        self.schema.major_generator
    }

    /// The poloidal (minor) deck generator.
    pub fn minor_generator(&self) -> DeckGenerator {
        self.schema.minor_generator
    }

    /// A short stable tag, for probe records.
    pub fn tag(&self) -> &'static str {
        "embedded_torus_rank2_deck"
    }
}

/// A certified embedded torus carrying the entity and placement needed for
/// cut-open realization.
///
/// The rank-2 analogue of [`super::cylinder::CertifiedEmbeddedCylinder`] and
/// [`super::cone::CertifiedEmbeddedCone`]. The [`CertifiedRankTwoDeck`] is
/// certified in world space (via [`identify_torus_world`]); the `entity` and
/// `transform` are the untransformed `Torus` and its `Matrix4` placement, kept
/// so that [`super::torus_realize::realize_torus_annulus`] can evaluate
/// `transform.transform_point(torus.subs(u, v))` during mesh realization.
#[derive(Debug, Clone, PartialEq)]
pub struct CertifiedEmbeddedTorus {
    deck: CertifiedRankTwoDeck,
    entity: Torus,
    transform: Matrix4,
}

impl CertifiedEmbeddedTorus {
    /// Construct from the certified deck, the untransformed entity, and the
    /// placement transform.
    pub fn new(deck: CertifiedRankTwoDeck, entity: Torus, transform: Matrix4) -> Self {
        Self {
            deck,
            entity,
            transform,
        }
    }

    /// The certified rank-two deck (world space).
    pub fn deck(&self) -> &CertifiedRankTwoDeck {
        &self.deck
    }

    /// The untransformed `Torus` entity, for `subs(u, v)` evaluation.
    pub fn entity(&self) -> &Torus {
        &self.entity
    }

    /// The placement transform, for `transform_point(torus.subs(u, v))`.
    pub fn transform(&self) -> &Matrix4 {
        &self.transform
    }

    /// A short stable tag, for probe records.
    pub fn tag(&self) -> &'static str {
        "certified_embedded_torus"
    }
}

/// The result of reading a `Torus` structurally.
#[derive(Debug, Clone, PartialEq)]
pub enum TorusIdentification {
    /// A certified regular embedded torus with a rank-two deck.
    Torus(CertifiedRankTwoDeck),
    /// Refused, with a named reason.
    NotATorus(TorusIdentificationFailure),
}

/// Read a `Torus` structurally and certify a regular embedded torus with a
/// rank-two deck.
///
/// The single introduction rule for [`CertifiedRankTwoDeck`]. Refuses a
/// spindle or horn torus (`small >= large`), a degenerate radius, a non-finite
/// coordinate, and any surface whose `2π` period on either axis it cannot
/// verify by evaluation.
pub fn identify_torus(torus: &Torus) -> TorusIdentification {
    identify_torus_world(
        torus.center(),
        Vector3::new(0.0, 0.0, 1.0),
        torus.large_radius(),
        torus.small_radius(),
    )
}

/// Read a torus from its world-space parameters and certify a regular embedded
/// torus with a rank-two deck.
///
/// The look-side adapter extracts the centre, the (similarity-rotated) axis and
/// the (similarity-scaled) radii from the STEP `ToroidalSurface` placement and
/// calls this. The periods are placement-independent and are verified on a
/// canonical evaluation torus. The axis is normalized defensively.
pub fn identify_torus_world(
    center: Point3,
    axis: Vector3,
    large: f64,
    small: f64,
) -> TorusIdentification {
    // --- finiteness -------------------------------------------------------
    let finite = |v: f64| FiniteF64::new(v);
    for coordinate in [
        center.x, center.y, center.z, large, small, axis.x, axis.y, axis.z,
    ] {
        if let Err(cause) = finite(coordinate) {
            return TorusIdentification::NotATorus(
                TorusIdentificationFailure::NonFiniteCoordinate { cause },
            );
        }
    }

    // --- axis nondegenerate (normalize) ----------------------------------
    let axis_norm = axis.magnitude();
    if !(axis_norm > 0.0) {
        return TorusIdentification::NotATorus(TorusIdentificationFailure::DegenerateAxis);
    }
    let axis = axis / axis_norm;

    // --- strictly positive radii ----------------------------------------
    let Ok(large_pf) = PositiveFinite::new(large) else {
        return TorusIdentification::NotATorus(TorusIdentificationFailure::DegenerateRadius);
    };
    let Ok(small_pf) = PositiveFinite::new(small) else {
        return TorusIdentification::NotATorus(TorusIdentificationFailure::DegenerateRadius);
    };

    // --- regular ring torus: large > small -------------------------------
    if !(large > small) {
        return TorusIdentification::NotATorus(TorusIdentificationFailure::SpindleOrHornTorus);
    }

    // --- both 2π periods, verified on a canonical evaluation torus ------
    // Periods are placement-independent, so evaluate on a canonical torus.
    let canon = Torus::new(Point3::new(0.0, 0.0, 0.0), large, small);
    let scale = large + small;
    let tol = MINIMUM_TORUS_PERIOD_RESIDUAL * scale;
    let (u0, v0) = (0.3, 0.7);
    let p = canon.subs(u0, v0);
    let pu = canon.subs(u0 + TAU, v0);
    let pv = canon.subs(u0, v0 + TAU);
    if (p - pu).magnitude() > tol || (p - pv).magnitude() > tol {
        return TorusIdentification::NotATorus(TorusIdentificationFailure::UnverifiedPeriod);
    }

    // --- rank-two deck: one 2π generator per developed axis --------------
    let Ok(tau_finite) = FiniteF64::new(TAU) else {
        return TorusIdentification::NotATorus(TorusIdentificationFailure::UnverifiedPeriod);
    };
    // Major (azimuthal, u) on the developed-second axis (matching cylinder's
    // angular convention); minor (poloidal, v) on the developed-first axis.
    let major_generator = match DeckGenerator::new(DevelopedAxis::Second, tau_finite) {
        Ok(g) => g,
        Err(DeckConstructorFailure::ZeroPeriod)
        | Err(DeckConstructorFailure::Numeric(_))
        | Err(DeckConstructorFailure::BoundsInverted) => {
            return TorusIdentification::NotATorus(TorusIdentificationFailure::UnverifiedPeriod);
        }
    };
    let minor_generator = match DeckGenerator::new(DevelopedAxis::First, tau_finite) {
        Ok(g) => g,
        Err(DeckConstructorFailure::ZeroPeriod)
        | Err(DeckConstructorFailure::Numeric(_))
        | Err(DeckConstructorFailure::BoundsInverted) => {
            return TorusIdentification::NotATorus(TorusIdentificationFailure::UnverifiedPeriod);
        }
    };

    TorusIdentification::Torus(CertifiedRankTwoDeck {
        schema: TorusSchema {
            center,
            axis,
            large_radius: large_pf,
            small_radius: small_pf,
            major_generator,
            minor_generator,
        },
        certificate: TorusValidityCertificate::discharged(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ring_torus(large: f64, small: f64) -> Torus {
        Torus::new(Point3::new(1.0, 2.0, 3.0), large, small)
    }

    #[test]
    fn a_regular_ring_torus_certifies_rank_two() {
        let id = identify_torus(&ring_torus(5.0, 1.0));
        let CertifiedRankTwoDeck { schema, .. } = id.expect_torus();
        assert!((schema.large_radius().get() - 5.0).abs() < 1e-12);
        assert!((schema.small_radius().get() - 1.0).abs() < 1e-12);
        let [major, minor] = [schema.major_generator(), schema.minor_generator()];
        // Both generators are 2π, on distinct developed axes.
        assert!((major.signed_period().get().abs() - TAU) < 1e-12);
        assert!((minor.signed_period().get().abs() - TAU) < 1e-12);
        assert_ne!(major.periodic_axis(), minor.periodic_axis());
    }

    #[test]
    fn a_spindle_torus_is_refused() {
        // small > large: self-intersecting spindle torus.
        let id = identify_torus(&ring_torus(1.0, 5.0));
        assert_eq!(
            id,
            TorusIdentification::NotATorus(TorusIdentificationFailure::SpindleOrHornTorus)
        );
    }

    #[test]
    fn a_horn_torus_is_refused() {
        // small == large: singular horn torus.
        let id = identify_torus(&ring_torus(3.0, 3.0));
        assert_eq!(
            id,
            TorusIdentification::NotATorus(TorusIdentificationFailure::SpindleOrHornTorus)
        );
    }

    #[test]
    fn both_periods_are_two_pi() {
        // The evaluation check itself certifies 2π periodicity on both axes;
        // a regular torus passes it.
        let id = identify_torus(&ring_torus(10.0, 2.0));
        assert!(matches!(id, TorusIdentification::Torus(_)));
    }

    #[test]
    fn the_deck_is_rank_two_with_independent_generators() {
        let id = identify_torus(&ring_torus(8.0, 2.0));
        let deck = id.expect_torus();
        let [g0, g1] = deck.deck_generators();
        // Independence is structural: distinct developed axes.
        assert_ne!(g0.periodic_axis(), g1.periodic_axis());
        assert!(!g0.signed_period().is_zero());
        assert!(!g1.signed_period().is_zero());
        assert_eq!(deck.tag(), "embedded_torus_rank2_deck");
    }
}

impl TorusIdentification {
    /// Unwrap a `Torus` verdict, panicking otherwise (test helper).
    fn expect_torus(self) -> CertifiedRankTwoDeck {
        match self {
            TorusIdentification::Torus(d) => d,
            other => panic!("expected Torus, got {other:?}"),
        }
    }
}
