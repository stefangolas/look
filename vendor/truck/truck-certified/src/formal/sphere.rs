//! Certified identification of an embedded sphere support surface.
//!
//! # Scope
//!
//! A STEP `spherical_surface` reaches the tessellator as
//! `Processor<Sphere, Matrix4>` (see `truck-stepio`'s `step_geometry::mod`:
//! `SphericalSurface = Processor<Sphere, Matrix4>`). [`identify_sphere`] reads
//! the inner [`Sphere`] structurally and either certifies an embedded sphere
//! witness or refuses with a named reason.
//!
//! The prevalence census (`docs/CERTIFIED_PREVALENCE.md`, section "Sphere")
//! found 1,831 sphere-carried corpus faces (2.56%) with representation-named
//! evidence only, because no certified constructor existed. This module is
//! that constructor; it unblocks sphere PAIRS in BG-CK-P1-DISPATCH
//! (cylinder~sphere 3,249; sphere~spline 1,202; torus~sphere 539;
//! plane~sphere 281; sphere~sphere 126). The 284 degenerate-torus faces are
//! the honest-refusal residual and remain out of scope.
//!
//! # Pre-made decisions
//!
//! **H-1.** The crate-level `#![deny(clippy::unwrap_used)]` covers this module:
//! no `unwrap`/`expect`/`panic!` anywhere in `sphere.rs`, and no module-level
//! `allow`. This is authored certified code — the grandfathered-allow doctrine
//! does not apply.
//!
//! **Representation-derived, never re-derived (booking decision 3).** The
//! witness carries the center and radius EXACTLY as the representation states
//! them (the identify_plane retained-basis doctrine: never orthogonalised,
//! never normalised downstream). The constructor certifies ADMISSIBILITY
//! (finiteness, positivity, similarity); it does not "improve" the numbers. No
//! least-squares fitting, no averaging, no epsilon-tolerant snapping.
//!
//! **World-params is the single introduction rule.** [`identify_sphere_world`]
//! is the one place a [`CertifiedEmbeddedSphere`] can be born; the typed and
//! placement entries delegate to it.
//!
//! **Placement entry, one named similarity rule.** STEP spheres arrive as
//! `Processor<Sphere, Matrix4>`. The placement entry extracts world parameters
//! by this pre-decided rule: the placement matrix's three direction columns
//! have magnitudes `s_x, s_y, s_z` computed in `f64`. If they are not ALL
//! EQUAL as `f64` (exact comparison — no epsilon), the placement deforms the
//! sphere into an ellipsoid and the entry refuses
//! [`SphereIdentificationFailure::NonSimilarityPlacement`]. (A similarity
//! placement's columns are equal by construction; STEP does not carry
//! anisotropic sphere placements.) The common column magnitude IS the radius
//! scale: `radius_world = radius_local * s_x`, one `f64` product — the
//! representation's own claim read out, not a re-derivation. The center maps
//! through the placement in `f64`. A `Processor` with an identity-ish rotation
//! still goes through this rule; there is no fast path that skips the column
//! check.
//!
//! **Refusal vocabulary is sphere-local and named.** [`crate::contract::Refusal`]
//! is FROZEN; the base `truck_base::evidence::Refusal` is untouched (mapping
//! section C row 1).
//!
//! # The parameter convention
//!
//! `Sphere::subs(u, v)` (see `truck-geometry::specifieds::sphere`) is
//! `center + radius * (sin u cos v, sin u sin v, cos u)`, so:
//!
//! - `u` is the **latitude** (colatitude) angle, range `[0, π]`, aperiodic,
//! - `v` is the **longitude** angle, period `2π` (the only periodic axis;
//!   `Sphere::v_period` reports `2π`).
//!
//! The longitude `2π` period is verified by evaluation on a canonical
//! evaluation sphere (periods are placement-independent) against
//! [`super::torus::MINIMUM_TORUS_PERIOD_RESIDUAL`]. The latitude axis is not
//! periodic, so only the longitude period is verified.

use super::numeric::{FiniteF64, NumericDomainError, PositiveFinite};
use super::torus::MINIMUM_TORUS_PERIOD_RESIDUAL;
use std::f64::consts::TAU;
use truck_geometry::prelude::{
    InnerSpace, Matrix4, ParametricSurface, Point3, Processor, Sphere, Transform,
};

/// Why a surface could not be certified as an embedded sphere.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SphereIdentificationFailure {
    /// A coordinate or the radius was not finite.
    NonFiniteCoordinate {
        /// The failing coordinate's domain error.
        cause: NumericDomainError,
    },
    /// The radius was not strictly positive.
    DegenerateRadius,
    /// The placement's direction columns do not share one magnitude, so
    /// the surface is an ellipsoid, not a sphere.
    NonSimilarityPlacement,
    /// The longitude period could not be verified by evaluation.
    UnverifiedPeriod,
}

/// The outcome of sphere identification: a certified witness or a named
/// refusal. A refusal is the classifier saying "not this class" — the
/// dispatch order the Phase-1 fast path runs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SphereIdentification {
    /// A certified embedded sphere.
    Sphere(CertifiedEmbeddedSphere),
    /// Not a sphere, with a named reason.
    NotASphere(SphereIdentificationFailure),
}

/// A certified embedded sphere: representation-derived center and radius,
/// admissibility certified by exact predicates at construction.
///
/// Constructed only through [`identify_sphere_world`] (the single
/// introduction rule). Fields are private; accessors return the
/// representation-derived values verbatim.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CertifiedEmbeddedSphere {
    center: Point3,
    radius: PositiveFinite,
}

impl CertifiedEmbeddedSphere {
    /// The representation-derived center, verbatim.
    pub fn center(&self) -> Point3 {
        self.center
    }

    /// The certified strictly-positive radius.
    pub fn radius(&self) -> PositiveFinite {
        self.radius
    }

    /// A short stable tag, for diagnostics.
    pub fn tag(&self) -> &'static str {
        "certified_embedded_sphere"
    }
}

/// Read a constructed `Sphere` and certify an embedded sphere.
///
/// The typed entry: reads the center and radius straight off the entity and
/// delegates to [`identify_sphere_world`].
pub fn identify_sphere(sphere: &Sphere) -> SphereIdentification {
    identify_sphere_world(sphere.center(), sphere.radius())
}

/// Read a STEP placement (`SphericalSurface` shape) and certify an embedded
/// sphere under the similarity rule above.
///
/// Extracts the world-space parameters from the `Processor<Sphere, Matrix4>`
/// placement (column-magnitude similarity check, one-product radius scale,
/// placement-mapped center) and delegates to [`identify_sphere_world`].
pub fn identify_sphere_placement(sphere: &Processor<Sphere, Matrix4>) -> SphereIdentification {
    let matrix = *sphere.transform();
    let entity = *sphere.entity();
    let local_center = entity.center();
    let local_radius = entity.radius();

    // --- similarity placement --------------------------------------------
    // The three direction columns (the placement's linear part). Their
    // magnitudes are the per-axis scales `s_x, s_y, s_z`, computed in `f64`.
    let sx = matrix.x.truncate().magnitude();
    let sy = matrix.y.truncate().magnitude();
    let sz = matrix.z.truncate().magnitude();
    // Exact comparison, no epsilon: a similarity placement's columns are equal
    // by construction, and STEP does not carry anisotropic sphere placements.
    if sx != sy || sy != sz {
        return SphereIdentification::NotASphere(
            SphereIdentificationFailure::NonSimilarityPlacement,
        );
    }

    // --- world parameters, read out (never re-derived) --------------------
    // The common column magnitude IS the radius scale: the representation's
    // own claim read out as one `f64` product.
    let radius_world = local_radius * sx;
    let center_world = matrix.transform_point(local_center);
    identify_sphere_world(center_world, radius_world)
}

/// Read world-space parameters and certify an embedded sphere.
///
/// The single introduction rule for [`CertifiedEmbeddedSphere`]; the other two
/// entries delegate here. Order of refusals (each an early return): finiteness
/// of all coordinates → [`SphereIdentificationFailure::DegenerateRadius`]
/// (`PositiveFinite::new`) → longitude period verification
/// ([`SphereIdentificationFailure::UnverifiedPeriod`]). No other checks: a
/// sphere has no axis to degenerate and no radius ratio to bound.
pub fn identify_sphere_world(center: Point3, radius: f64) -> SphereIdentification {
    // --- finiteness -------------------------------------------------------
    let finite = |v: f64| FiniteF64::new(v);
    for coordinate in [center.x, center.y, center.z, radius] {
        if let Err(cause) = finite(coordinate) {
            return SphereIdentification::NotASphere(
                SphereIdentificationFailure::NonFiniteCoordinate { cause },
            );
        }
    }

    // --- strictly positive radius ----------------------------------------
    let Ok(radius_pf) = PositiveFinite::new(radius) else {
        return SphereIdentification::NotASphere(SphereIdentificationFailure::DegenerateRadius);
    };

    // --- longitude 2π period, verified on a canonical evaluation sphere ----
    // Periods are placement-independent, so evaluate on a canonical sphere.
    // `v` is the longitude axis (truck convention; `Sphere::v_period` is 2π),
    // verified here by evaluation. The latitude axis is not periodic.
    let canon = Sphere::new(Point3::new(0.0, 0.0, 0.0), radius);
    let tol = MINIMUM_TORUS_PERIOD_RESIDUAL * radius;
    let (u0, v0) = (0.3, 0.7);
    let p = canon.subs(u0, v0);
    let pv = canon.subs(u0, v0 + TAU);
    if (p - pv).magnitude() > tol {
        return SphereIdentification::NotASphere(SphereIdentificationFailure::UnverifiedPeriod);
    }

    SphereIdentification::Sphere(CertifiedEmbeddedSphere {
        center,
        radius: radius_pf,
    })
}
