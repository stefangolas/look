//! Certified identification of an embedded conical support surface, with its
//! apex located rather than assumed away.
//!
//! # Scope
//!
//! The rank-1 companion to [`super::cylinder`]. A STEP `conical_surface`
//! reaches the tessellator as the *same* representation a cylinder does —
//! `Processor<RevolutedCurve<Line<Point3>>, Matrix4>` (see `truck-stepio`'s
//! `ConicalSurface`) — and differs from it in exactly one structural fact: the
//! revolved line is **tilted** with respect to the axis instead of parallel to
//! it. [`identify_cone`] reads the inner [`RevolutedCurve<Line<Point3>>`]
//! structurally and either certifies an embedded cone or refuses with a named
//! reason.
//!
//! Because the representation is shared, so is the periodicity: the `2π`
//! angular period the cylinder certifies is certified here by the same
//! evaluation, and the deck generator is the same object on the same developed
//! axis. This module adds no new periodicity mathematics. What it adds is the
//! apex.
//!
//! # Where the authority comes from
//!
//! Stated once, because "certified cone" has to mean something specific and
//! because two of the checks below look like classifiers and are not.
//!
//! ```text
//! that the surface is a cone      the representation. `look`'s adapter matches
//!                                 `ElementarySurface::ConicalSurface`, which is
//!                                 STEP's own `conical_surface` with its
//!                                 `radius` and `semi_angle`. This module never
//!                                 decides cone-ness from geometry.
//!
//! that the placement preserves    the adapter, which admits only a similarity
//! it                              transform. A similarity carries circles to
//!                                 circles and circular cones to circular
//!                                 cones, which is what makes reassembling the
//!                                 revolution from its transformed atomic
//!                                 pieces equal to the `Processor`'s own
//!                                 surface rather than merely close to it.
//!
//! where the apex is               closed form, from the generatrix's own
//!                                 representation: the projected generatrix is
//!                                 a ray in the plane normal to the axis, and
//!                                 the apex is where it meets that plane's
//!                                 origin. One division, no search, no fit.
//!
//! that the chart is regular       the two degeneracy floors, which refuse a
//! enough to use                   declared cone whose half-angle is zero (a
//!                                 cylinder's chart) or a right angle (a planar
//!                                 annulus's).
//!
//! that the arithmetic held        [`CONE_STRUCTURAL_RESIDUAL`], applied to the
//!                                 apex solve, the projected parallelism and
//!                                 the half-angle's reproduction of the
//!                                 declared endpoints. Confirmation of this
//!                                 module's own algebra, at the scale of the
//!                                 quantities involved.
//!
//! that the parameter convention   `u_period()`/`v_period()`, structurally.
//! is the expected one
//! ```
//!
//! **Sampling appears in exactly one place and creates no authority.**
//! [`verify_angular_convention`] evaluates the surface at one interior point to
//! *reject* a representation whose accessors disagree with its own evaluation.
//! Every sample-derived branch there returns `None`; none of them can turn a
//! `None` into a `Some`, and removing all of them would only make the function
//! more permissive. The structural `2π` from `v_period()` carries the
//! authority, exactly as it does in [`super::cylinder`].
//!
//! # The apex is the whole difference
//!
//! A cylinder is regular everywhere: every orbit of its angular deck action is
//! a circle of the cylinder's own positive radius, at every axial coordinate.
//! A cone is not. At one point of the axis the revolved line meets it, the
//! orbit collapses to a point, the deck action stops being free, and every
//! chart built on the quotient degenerates.
//!
//! So a certified cone must **locate** that point, not merely avoid it. This
//! module solves for it in closed form from the generatrix's own
//! representation and then verifies the solution lies on the axis, which is
//! also the test that separates a cone from a one-sheet hyperboloid: a
//! revolved line that is tilted *and skew* to the axis never meets it and
//! sweeps a hyperboloid, whose signature at a face boundary can look exactly
//! like a frustum's. That surface is refused by name
//! ([`ConeIdentificationFailure::GeneratrixSkewToAxis`]) rather than fitted to
//! a cone.
//!
//! # The developed coordinate system
//!
//! The cone develops into a chart with two coordinates, on the same
//! `(aperiodic = First, periodic = Second)` convention the cylinder fixes:
//!
//! - the **generator** coordinate `s = (x - apex) · axis`, signed, aperiodic,
//!   and zero exactly at the apex;
//! - the **angular** coordinate `theta`, defined so that it advances with the
//!   surface's own revolution parameter `v`:
//!   `theta = atan2((x - origin) · radial_y, (x - origin) · radial_x)` with
//!   `radial_y = axis × radial_x`.
//!
//! `s` rather than a raw axial coordinate, and that choice is the substance of
//! the packet's first new obligation. On a cylinder two parallels are ordered
//! by an axial coordinate whose origin is arbitrary, because every level is
//! alike. On a cone the levels are not alike: the radius at `s` is
//! `slope · |s|`, the sign of `s` names the **nappe**, and `s = 0` is the
//! singular orbit. Stating a carrier's position in `s` therefore carries the
//! half-angle and the apex with it, and "these two carriers lie on one nappe
//! with the apex outside the interval between them" becomes a statement about
//! the signs and order of two numbers rather than a claim about proximity.
//!
//! `theta` is a genuine `2π`-periodic coordinate on **both** nappes: the
//! revolution rotates every point about the axis right-handedly, whichever
//! side of the apex it is on, so `theta` advances with `v` on both. The one
//! difference is a constant offset of `π` between the nappes, because the
//! recorded `radial_x` is the radial direction of a point on one particular
//! side. That offset is irrelevant to everything downstream — nothing here
//! compares a `theta` on one nappe against a `theta` on the other, because
//! [`super::cone_band`] refuses a face whose carriers are not on one nappe
//! before any chart arithmetic happens.

use super::deck::{DeckConstructorFailure, DeckGenerator, DevelopedAxis};
use super::numeric::{FiniteF64, NumericDomainError, PositiveFinite};
use std::f64::consts::TAU;
use truck_geometry::prelude::{
    InnerSpace, Line, ParametricSurface, Point3, RevolutedCurve, Vector3,
};

/// Dimensionless sine floor below which a generatrix line counts as parallel
/// to the axis, i.e. as a *cylinder* rather than a cone.
///
/// The exact complement of [`super::cylinder::MINIMUM_CYLINDER_LINE_AXIS_PARALLELISM`],
/// deliberately the same number: a revolved line is admitted by exactly one of
/// the two identifiers, and a surface in the gap between them is admitted by
/// neither rather than by both.
///
/// A **degeneracy floor**, not a classifier — see the module docs' authority
/// chain. What a surface *is* comes from the representation the caller matched
/// on; what this refuses is a declared cone whose half-angle is so near zero
/// that the generator coordinate carries no usable radius and the apex is
/// effectively at infinity.
pub const MINIMUM_CONE_GENERATRIX_TILT: f64 = 1e-9;

/// Relative residual bound for confirming a closed-form structural derivation.
///
/// Distinct from the two floors above in *role*, which is why it is a distinct
/// constant despite sharing their value. The floors decide whether a declared
/// cone is degenerate. This one confirms that arithmetic this module performed
/// — the apex solve, the parallelism of the projected generatrix, the
/// half-angle's reproduction of the declared endpoints — reproduces the
/// representation it was derived from, at the scale of the quantities
/// involved.
///
/// It therefore never has to distinguish a cone from a nearby non-cone on a
/// real file: a STEP `conical_surface` is built from a `radius` and a
/// `semi_angle` about a placement axis, so its projected generatrix passes
/// through the axis *exactly*, by construction, in the placement's own frame —
/// and `look`'s adapter admits only similarity placements, which preserve that
/// exactly. What this bound measures on a conforming file is floating-point
/// drift, and what it refuses is a `RevolutedCurve<Line>` handed in from
/// somewhere else that is genuinely not a cone at all.
pub const CONE_STRUCTURAL_RESIDUAL: f64 = 1e-9;

/// Dimensionless cosine floor below which a generatrix line counts as
/// perpendicular to the axis.
///
/// A line perpendicular to the axis sweeps a planar annulus, not a cone: it
/// has no apex on the axis to speak of (every point of it is at the same
/// axial coordinate), the half-angle is `π/2`, and the generator coordinate
/// `s` would be constant and therefore useless as an aperiodic chart
/// coordinate. Refused rather than certified with an infinite slope.
pub const MINIMUM_CONE_AXIAL_COMPONENT: f64 = 1e-9;

/// Why a revolved-line surface was not certified as an embedded cone.
#[derive(Debug, Clone, PartialEq)]
pub enum ConeIdentificationFailure {
    /// A coordinate was `NaN` or infinite.
    NonFiniteCoordinate {
        /// Why the value was refused.
        cause: NumericDomainError,
    },
    /// The revolution axis has (near-)zero magnitude, so it defines no line.
    DegenerateAxis,
    /// The generatrix `Line` has coincident endpoints, so it has no direction.
    DegenerateGeneratrix,
    /// The generatrix is parallel to the axis: the surface is a cylinder, and
    /// [`super::cylinder::identify_cylinder`] is the identifier for it.
    CylindricalRevolution,
    /// The generatrix is perpendicular to the axis: the surface is a planar
    /// annulus, not a cone.
    GeneratrixPerpendicularToAxis,
    /// The generatrix is tilted but does not meet the axis, so the revolution
    /// has no apex and is a one-sheet hyperboloid rather than a cone.
    GeneratrixSkewToAxis,
    /// The generatrix lies along the axis, so the revolution is a line.
    DegenerateRadius,
    /// The apex solved for from the generatrix does not lie on the axis, so
    /// the closed-form solution is not confirmed by the representation it was
    /// derived from.
    ApexNotOnAxis,
    /// The radius the certified half-angle predicts at a generatrix endpoint
    /// is not the radius that endpoint actually has, so the surface is not the
    /// cone the structural read claims.
    SlopeDoesNotReproduceGeneratrix,
    /// The period evidence did not verify the expected `2π` angular period, or
    /// the parameter convention could not be confirmed by evaluation.
    UnverifiedParameterConvention,
}

impl ConeIdentificationFailure {
    /// A short stable tag, for probe records.
    pub fn tag(&self) -> &'static str {
        match self {
            Self::NonFiniteCoordinate { .. } => "cone_non_finite_coordinate",
            Self::DegenerateAxis => "cone_degenerate_axis",
            Self::DegenerateGeneratrix => "cone_degenerate_generatrix",
            Self::CylindricalRevolution => "cone_cylindrical_revolution",
            Self::GeneratrixPerpendicularToAxis => "cone_generatrix_perpendicular_to_axis",
            Self::GeneratrixSkewToAxis => "cone_generatrix_skew_to_axis",
            Self::DegenerateRadius => "cone_degenerate_radius",
            Self::ApexNotOnAxis => "cone_apex_not_on_axis",
            Self::SlopeDoesNotReproduceGeneratrix => "cone_slope_does_not_reproduce_generatrix",
            Self::UnverifiedParameterConvention => "cone_unverified_parameter_convention",
        }
    }
}

/// The result of reading a revolved-line surface structurally.
#[derive(Debug, Clone, PartialEq)]
pub enum ConeIdentification {
    /// An embedded cone was certified.
    Cone(CertifiedEmbeddedCone),
    /// The surface is not an embedded cone. Carries the reason.
    NotACone(ConeIdentificationFailure),
}

/// Which nappe of a cone a point lies on: the sign of its generator
/// coordinate.
///
/// A mathematical cone is double-napped, and STEP does not restrict it — the
/// restriction is the trimmed face's job. So "which side of the apex" is a
/// fact that has to be carried explicitly rather than assumed, and this is the
/// type that carries it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Nappe {
    /// `s > 0`: the half-cone in the `+axis` direction from the apex.
    Positive,
    /// `s < 0`: the half-cone in the `-axis` direction from the apex.
    Negative,
}

impl Nappe {
    /// The sign this nappe corresponds to, as a multiplier.
    pub fn sign(self) -> f64 {
        match self {
            Self::Positive => 1.0,
            Self::Negative => -1.0,
        }
    }

    /// A short stable tag, for probe records.
    pub fn tag(self) -> &'static str {
        match self {
            Self::Positive => "nappe_positive",
            Self::Negative => "nappe_negative",
        }
    }
}

/// A certificate that a surface is a regular embedded cone away from a
/// **located** apex, with the deck generator that its quotient defines.
///
/// The only constructor is [`identify_cone`]; fields are private, so the
/// obligations — nondegenerate axis and generatrix, a tilt strictly between
/// parallel and perpendicular, a generatrix that provably meets the axis, an
/// apex confirmed on the axis, a half-angle that reproduces the generatrix,
/// and a verified `2π` angular period — are discharged by presenting the
/// representation, not by assembling numbers.
#[derive(Debug, Clone, PartialEq)]
pub struct CertifiedEmbeddedCone {
    schema: ConeSchema,
    certificate: ConeValidityCertificate,
}

impl CertifiedEmbeddedCone {
    /// The structural schema.
    pub fn schema(&self) -> &ConeSchema {
        &self.schema
    }

    /// The validity obligations discharged.
    pub fn certificate(&self) -> &ConeValidityCertificate {
        &self.certificate
    }

    /// The deck generator: the angular period on the developed-second axis.
    pub fn deck_generator(&self) -> DeckGenerator {
        self.schema.deck_generator
    }

    /// A short stable tag, for probe records.
    pub fn tag(&self) -> &'static str {
        "embedded_cone"
    }
}

/// The structural data of a certified embedded cone.
#[derive(Debug, Clone, PartialEq)]
pub struct ConeSchema {
    origin: Point3,
    apex: Point3,
    axis: Vector3,
    radial_x: Vector3,
    radial_y: Vector3,
    slope: PositiveFinite,
    generatrix: Line<Point3>,
    periodic_parameter: super::cylinder::PeriodicParameter,
    period: PositiveFinite,
    deck_generator: DeckGenerator,
}

impl ConeSchema {
    /// The revolution origin, as the representation declared it.
    ///
    /// Kept for provenance and because [`Self::angular_coordinate`] is defined
    /// in its frame. It is *not* the apex, and no obligation is stated against
    /// it: an exporter may place it anywhere on the axis.
    pub fn origin(&self) -> Point3 {
        self.origin
    }

    /// The apex: the one point where the revolved line meets the axis, and the
    /// one orbit where the deck action is not free.
    pub fn apex(&self) -> Point3 {
        self.apex
    }

    /// The unit axis direction.
    pub fn axis(&self) -> Vector3 {
        self.axis
    }

    /// The unit radial direction at angular parameter zero.
    pub fn radial_x(&self) -> Vector3 {
        self.radial_x
    }

    /// The second radial basis vector `axis × radial_x`.
    pub fn radial_y(&self) -> Vector3 {
        self.radial_y
    }

    /// The tangent of the cone's half-angle, proved strictly positive and
    /// finite: the radius at generator coordinate `s` is `slope · |s|`.
    pub fn slope(&self) -> PositiveFinite {
        self.slope
    }

    /// The generatrix line the surface revolves.
    pub fn generatrix(&self) -> Line<Point3> {
        self.generatrix
    }

    /// Which parameter carries the revolution angle.
    pub fn periodic_parameter(&self) -> super::cylinder::PeriodicParameter {
        self.periodic_parameter
    }

    /// The certified angular period (normally `2π`).
    pub fn period(&self) -> PositiveFinite {
        self.period
    }

    /// The deck generator translating the angular developed coordinate.
    pub fn deck_generator(&self) -> DeckGenerator {
        self.deck_generator
    }

    /// The **generator coordinate** of a physical point: `(x - apex) · axis`.
    ///
    /// The chart's aperiodic coordinate. Signed, zero exactly at the apex, and
    /// its sign is the point's nappe. See the module docs for why the position
    /// of a carrier is stated in this and not in a raw axial coordinate.
    pub fn generator_coordinate(&self, x: Point3) -> f64 {
        (x - self.apex).dot(self.axis)
    }

    /// The radius of the cone's parallel at generator coordinate `s`.
    pub fn radius_at(&self, s: f64) -> f64 {
        self.slope.get() * s.abs()
    }

    /// The angular coordinate of a physical point on the cone, in the recorded
    /// frame.
    ///
    /// Advances with the surface's angular parameter `v` on both nappes; see
    /// the module docs for the constant `π` offset between them, and why
    /// nothing downstream can observe it. Undefined at the apex and off the
    /// cone; callers must have certified the point lies on the surface away
    /// from the apex.
    pub fn angular_coordinate(&self, x: Point3) -> f64 {
        let r = x - self.origin;
        r.dot(self.radial_y).atan2(r.dot(self.radial_x))
    }

    /// Which nappe a generator coordinate names, or `None` at the apex.
    ///
    /// Exactly `s.signum()` with zero excluded rather than rounded to a side —
    /// a point at `s = 0` *is* the apex, and naming a nappe for it would be
    /// the one guess this whole module exists to avoid.
    pub fn nappe_of(&self, s: f64) -> Option<Nappe> {
        if s > 0.0 {
            Some(Nappe::Positive)
        } else if s < 0.0 {
            Some(Nappe::Negative)
        } else {
            None
        }
    }

    /// The physical point at chart coordinates `(s, theta)`.
    ///
    /// The developed-to-physical map, and the exact inverse of
    /// [`Self::generator_coordinate`] and [`Self::angular_coordinate`] on
    /// **both** nappes: the apex lies on the axis, so the radial part of
    /// `x - apex` is the radial part of `x - origin` that `theta` was read
    /// from, and `slope · |s|` is its length whichever side of the apex `s`
    /// is on. The `π` offset the module docs describe is between `theta` and
    /// the surface's own parameter `v`, not between `theta` and this map.
    pub fn point_at(&self, s: f64, theta: f64) -> Point3 {
        let radius = self.radius_at(s);
        self.apex
            + s * self.axis
            + radius * theta.cos() * self.radial_x
            + radius * theta.sin() * self.radial_y
    }

    /// The gap between `point` and the cone, as the deviation of its radial
    /// distance from the radius its own generator coordinate predicts.
    pub fn radial_gap(&self, point: Point3) -> f64 {
        let r = point - self.apex;
        let s = r.dot(self.axis);
        let radial = r - s * self.axis;
        (radial.magnitude() - self.radius_at(s)).abs()
    }
}

/// Marker that the embedded-cone validity obligations were discharged by
/// [`identify_cone`]. Carried as an opaque token because the only producer is
/// the identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConeValidityCertificate {
    #[allow(dead_code)]
    discharged: bool,
}

impl ConeValidityCertificate {
    /// Constructed only by [`identify_cone`].
    fn discharged() -> Self {
        Self { discharged: true }
    }
}

/// Read a revolved-line surface structurally and certify an embedded cone.
///
/// The single introduction rule for [`CertifiedEmbeddedCone`]. It refuses a
/// cylinder (generatrix parallel to the axis), a planar annulus (generatrix
/// perpendicular to it), a one-sheet hyperboloid (generatrix tilted but skew
/// to it), a degenerate radius or axis, a non-finite coordinate, and any
/// surface whose `2π` angular period it cannot verify by evaluation.
pub fn identify_cone(revo: &RevolutedCurve<Line<Point3>>) -> ConeIdentification {
    use ConeIdentificationFailure as Failure;
    let refuse = ConeIdentification::NotACone;

    let origin = revo.origin();
    let axis_raw = revo.axis();
    let line = *revo.entity_curve();
    let Line(p, q) = line;
    let d = q - p;

    // --- finiteness -------------------------------------------------------
    for coordinate in [
        origin.x, origin.y, origin.z, axis_raw.x, axis_raw.y, axis_raw.z, p.x, p.y, p.z, q.x, q.y,
        q.z,
    ] {
        if let Err(cause) = FiniteF64::new(coordinate) {
            return refuse(Failure::NonFiniteCoordinate { cause });
        }
    }

    // --- axis nondegenerate ----------------------------------------------
    let axis_norm = axis_raw.magnitude();
    if !(axis_norm > 0.0) {
        return refuse(Failure::DegenerateAxis);
    }
    let axis = axis_raw / axis_norm;

    // --- generatrix direction nondegenerate ------------------------------
    let d_norm = d.magnitude();
    if !(d_norm > 0.0) {
        return refuse(Failure::DegenerateGeneratrix);
    }

    // --- tilt strictly between parallel and perpendicular -----------------
    // The generatrix splits into a component along the axis and one across
    // it. Both must be nonzero: a zero cross-component is a cylinder, a zero
    // along-component is a planar annulus. Each is refused by its own name so
    // a caller can tell which surface it actually handed over.
    let d_axial = d.dot(axis);
    let d_perp = d - d_axial * axis;
    let d_perp_norm = d_perp.magnitude();
    if !(d_perp_norm / d_norm >= MINIMUM_CONE_GENERATRIX_TILT) {
        return refuse(Failure::CylindricalRevolution);
    }
    if !(d_axial.abs() / d_norm >= MINIMUM_CONE_AXIAL_COMPONENT) {
        return refuse(Failure::GeneratrixPerpendicularToAxis);
    }

    // --- the generatrix meets the axis ------------------------------------
    // Everything below lives in the 2-plane perpendicular to the axis, where
    // the generatrix projects to the ray `rp_perp + t d_perp`. The revolved
    // surface is a cone exactly when that ray passes through the origin of
    // that plane — i.e. when the two vectors are parallel — and a one-sheet
    // hyperboloid otherwise. The test is the cross product, scaled by the two
    // magnitudes it is a product of, so it is a statement about the angle
    // between them and not about the units the file is written in.
    let rp = p - origin;
    let rp_perp = rp - rp.dot(axis) * axis;
    let rp_perp_norm = rp_perp.magnitude();
    let rq = q - origin;
    let rq_perp = rq - rq.dot(axis) * axis;
    if rp_perp_norm == 0.0 && rq_perp.magnitude() == 0.0 {
        // The whole generatrix lies on the axis: the revolution is a line.
        return refuse(Failure::DegenerateRadius);
    }
    let skew = rp_perp.cross(d_perp).magnitude();
    if !(skew <= CONE_STRUCTURAL_RESIDUAL * rp_perp_norm * d_perp_norm) {
        return refuse(Failure::GeneratrixSkewToAxis);
    }

    // --- the apex, in closed form and then confirmed -----------------------
    // The generatrix parameter at which the projected ray reaches the axis:
    // `rp_perp + t d_perp = 0`, solved in least squares, which is exact here
    // because the two vectors were just certified parallel.
    let t_apex = -rp_perp.dot(d_perp) / d_perp.dot(d_perp);
    let apex = p + t_apex * d;
    // The solution is confirmed against the representation it came from,
    // rather than trusted: the apex must lie on the axis. The scale is the
    // generatrix's own length, which is the only length this surface declares.
    let apex_offset = apex - origin;
    let apex_radial = apex_offset - apex_offset.dot(axis) * axis;
    if !(apex_radial.magnitude() <= CONE_STRUCTURAL_RESIDUAL * d_norm.max(1.0)) {
        return refuse(Failure::ApexNotOnAxis);
    }

    // --- the half-angle, and the check that it reproduces the generatrix ---
    let slope = d_perp_norm / d_axial.abs();
    let Ok(slope) = PositiveFinite::new(slope) else {
        return refuse(Failure::DegenerateRadius);
    };
    // `radius = slope · |s|` at both declared endpoints. This is what makes
    // the certificate a statement about the surface rather than about the two
    // numbers it was assembled from: a representation whose endpoints do not
    // sit at the radii its own tilt predicts is not a cone, however the pieces
    // were computed.
    let scale = d_norm.max(1.0);
    for point in [p, q] {
        let r = point - apex;
        let s = r.dot(axis);
        let radial = r - s * axis;
        let predicted = slope.get() * s.abs();
        if !((radial.magnitude() - predicted).abs() <= CONE_STRUCTURAL_RESIDUAL * scale) {
            return refuse(Failure::SlopeDoesNotReproduceGeneratrix);
        }
    }

    // --- the radial frame --------------------------------------------------
    // Taken from whichever declared endpoint sits further from the axis, which
    // is the better-conditioned of the two and is never the apex. Which nappe
    // it belongs to is immaterial: see the module docs on the constant `π`
    // offset.
    let frame_source = match rp_perp_norm >= rq_perp.magnitude() {
        true => rp_perp,
        false => rq_perp,
    };
    let frame_norm = frame_source.magnitude();
    if !(frame_norm > 0.0) {
        return refuse(Failure::DegenerateRadius);
    }
    let radial_x = frame_source / frame_norm;
    let radial_y = axis.cross(radial_x);

    // --- parameter convention, verified by evaluation ----------------------
    let convention = match verify_angular_convention(revo, axis, d, apex, slope.get(), scale) {
        Some(periodic) => periodic,
        None => return refuse(Failure::UnverifiedParameterConvention),
    };

    let Ok(period_finite) = FiniteF64::new(TAU) else {
        return refuse(Failure::UnverifiedParameterConvention);
    };
    let Ok(period) = PositiveFinite::new(period_finite.get()) else {
        return refuse(Failure::UnverifiedParameterConvention);
    };

    // The developed plane is (generator = First, angular = Second), the same
    // convention the cylinder fixes, so the deck generator is the same object
    // on the same axis. That identity is the reason this module adds no new
    // periodicity mathematics.
    let deck_generator = match DeckGenerator::new(DevelopedAxis::Second, period_finite) {
        Ok(generator) => generator,
        Err(DeckConstructorFailure::ZeroPeriod)
        | Err(DeckConstructorFailure::Numeric(_))
        | Err(DeckConstructorFailure::BoundsInverted) => {
            return refuse(Failure::UnverifiedParameterConvention);
        }
    };

    ConeIdentification::Cone(CertifiedEmbeddedCone {
        schema: ConeSchema {
            origin,
            apex,
            axis,
            radial_x,
            radial_y,
            slope,
            generatrix: line,
            periodic_parameter: convention,
            period,
            deck_generator,
        },
        certificate: ConeValidityCertificate::discharged(),
    })
}

/// Confirm by evaluation that one parameter is the `2π` angular parameter and
/// the other runs along the generatrix, returning which is angular.
///
/// Checks the surface's reported `u_period`/`v_period`, then verifies
/// `subs(u, v) == subs(u, v + 2π)` (angular closure), that the aperiodic
/// derivative runs along a generator, and that the sample point lies on the
/// certified cone.
///
/// **Every sample-derived branch here returns `None`.** The structural `2π`
/// from the accessors is the only thing that can produce a `Some`, so sampling
/// can narrow this function's acceptance but never widen it, and deleting all
/// of it would make the identifier strictly more permissive rather than less
/// justified. That is the same discipline [`super::cylinder`] applies, and it
/// is why a sample landing somewhere unhelpful — at the apex, say, where the
/// on-cone test is trivially satisfied — costs nothing: an uninformative sample
/// declines to reject, and rejection is all a sample can do.
fn verify_angular_convention(
    revo: &RevolutedCurve<Line<Point3>>,
    axis: Vector3,
    generatrix_direction: Vector3,
    apex: Point3,
    slope: f64,
    scale: f64,
) -> Option<super::cylinder::PeriodicParameter> {
    use super::cylinder::PeriodicParameter;

    let u_period = revo.u_period();
    let v_period = revo.v_period();
    // The truck contract: v carries the 2π revolution, u carries the line
    // (aperiodic). Accept the convention only when the accessors agree.
    let candidate = match (u_period, v_period) {
        (None, Some(vp)) if (vp - TAU).abs() < 1e-9 * TAU => PeriodicParameter::V,
        (Some(up), None) if (up - TAU).abs() < 1e-9 * TAU => PeriodicParameter::U,
        _ => return None,
    };

    let on_cone = |x: Point3| {
        let r = x - apex;
        let s = r.dot(axis);
        let radial = r - s * axis;
        (radial.magnitude() - slope * s.abs()).abs() <= 1e-9 * scale
    };
    // The aperiodic derivative runs along the generatrix — but along the
    // *revolved* copy of it, since `RevolutedCurve` rotates the whole curve by
    // the angular parameter before differentiating. Comparing the two
    // directions outright would therefore fail on every cone with a nonzero
    // sample angle, and pass on a cylinder only because a direction parallel
    // to the axis is fixed by the rotation. What survives the rotation is the
    // angle to the axis, so that is what is compared: the aperiodic derivative
    // must make the cone's own half-angle with the axis, and must have a
    // component across it, which together say it is a generator direction and
    // not an angular one.
    let generatrix_cosine = generatrix_direction.dot(axis).abs() / generatrix_direction.magnitude();
    let runs_along_a_generator = |v: Vector3| {
        let magnitude = v.magnitude();
        magnitude > f64::EPSILON
            && (v.dot(axis).abs() / magnitude - generatrix_cosine).abs() <= 1e-9
    };

    // One interior sample. Nothing depends on where it lands — a sample at the
    // apex, or on a domain that does not reach the face, simply declines to
    // reject — because rejection is the only thing a sample can do here.
    let (u0, v0) = (0.5, 0.3);
    let base = revo.subs(u0, v0);
    if !on_cone(base) {
        return None;
    }

    match candidate {
        PeriodicParameter::V => {
            if (revo.subs(u0, v0 + TAU) - base).magnitude() > 1e-9 * scale {
                return None;
            }
            if !runs_along_a_generator(revo.uder(u0, v0)) {
                return None;
            }
            Some(PeriodicParameter::V)
        }
        PeriodicParameter::U => {
            if (revo.subs(u0 + TAU, v0) - base).magnitude() > 1e-9 * scale {
                return None;
            }
            if !runs_along_a_generator(revo.vder(u0, v0)) {
                return None;
            }
            Some(PeriodicParameter::U)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::cylinder::PeriodicParameter;
    use super::*;

    /// A cone about the z-axis with its apex at the origin, half-angle
    /// `atan(slope)`, whose declared generatrix runs from `z = z0` to
    /// `z = z1` on the nappe those coordinates name.
    fn z_cone(slope: f64, z0: f64, z1: f64) -> RevolutedCurve<Line<Point3>> {
        RevolutedCurve::by_revolution(
            Line(
                Point3::new(slope * z0, 0.0, z0),
                Point3::new(slope * z1, 0.0, z1),
            ),
            Point3::new(0.0, 0.0, 0.0),
            Vector3::new(0.0, 0.0, 1.0),
        )
    }

    fn expect_cone(id: ConeIdentification) -> CertifiedEmbeddedCone {
        match id {
            ConeIdentification::Cone(cone) => cone,
            other => panic!("expected a cone, got {other:?}"),
        }
    }

    fn expect_refusal(id: ConeIdentification) -> ConeIdentificationFailure {
        match id {
            ConeIdentification::NotACone(failure) => failure,
            other => panic!("expected a refusal, got {other:?}"),
        }
    }

    #[test]
    fn an_ordinary_cone_is_certified_with_its_apex_located() {
        let cone = expect_cone(identify_cone(&z_cone(0.5, 1.0, 4.0)));
        let schema = cone.schema();
        assert!((schema.slope().get() - 0.5).abs() < 1e-12);
        assert!((schema.apex() - Point3::new(0.0, 0.0, 0.0)).magnitude() < 1e-9);
        assert!((schema.axis() - Vector3::new(0.0, 0.0, 1.0)).magnitude() < 1e-12);
        assert_eq!(schema.periodic_parameter(), PeriodicParameter::V);
        assert_eq!(cone.deck_generator().periodic_axis(), DevelopedAxis::Second);
        assert!((cone.deck_generator().period_magnitude().get() - TAU).abs() < 1e-12);
    }

    /// The generator coordinate is signed, is zero at the apex, and its sign
    /// is the nappe. That is the whole reason it exists.
    #[test]
    fn the_generator_coordinate_signs_the_nappe_and_vanishes_at_the_apex() {
        let cone = expect_cone(identify_cone(&z_cone(0.5, 1.0, 4.0)));
        let schema = cone.schema();
        let above = schema.point_at(2.0, 0.7);
        let below = schema.point_at(-2.0, 0.7);
        assert!((schema.generator_coordinate(above) - 2.0).abs() < 1e-9);
        assert!((schema.generator_coordinate(below) + 2.0).abs() < 1e-9);
        assert_eq!(
            schema.nappe_of(schema.generator_coordinate(above)),
            Some(Nappe::Positive)
        );
        assert_eq!(
            schema.nappe_of(schema.generator_coordinate(below)),
            Some(Nappe::Negative)
        );
        assert_eq!(schema.generator_coordinate(schema.apex()), 0.0);
        assert_eq!(schema.nappe_of(0.0), None);
    }

    /// The radius is not a property of the surface, it is a property of the
    /// level — which is the fact a cylinder does not have and the reason
    /// carrier order has to be stated in `s`.
    #[test]
    fn the_radius_grows_with_the_distance_from_the_apex() {
        let cone = expect_cone(identify_cone(&z_cone(0.5, 1.0, 4.0)));
        let schema = cone.schema();
        assert!((schema.radius_at(2.0) - 1.0).abs() < 1e-12);
        assert!((schema.radius_at(6.0) - 3.0).abs() < 1e-12);
        // Both nappes, same radius at the same distance.
        assert!((schema.radius_at(-2.0) - schema.radius_at(2.0)).abs() < 1e-12);
        assert_eq!(schema.radius_at(0.0), 0.0);
    }

    /// The chart round-trips: a point built from `(s, theta)` reports the same
    /// `s`, and the same `theta` on the frame's own nappe.
    #[test]
    fn the_chart_round_trips_on_the_frames_own_nappe() {
        let cone = expect_cone(identify_cone(&z_cone(0.7, 2.0, 5.0)));
        let schema = cone.schema();
        for s in [1.0_f64, 3.5, 9.0] {
            for theta in [0.0_f64, 0.9, 2.4, -1.6] {
                let point = schema.point_at(s, theta);
                assert!((schema.generator_coordinate(point) - s).abs() < 1e-9);
                let recovered = schema.angular_coordinate(point);
                let gap = (recovered - theta).abs();
                assert!(gap.min(TAU - gap) < 1e-9, "s={s} theta={theta}");
                assert!(schema.radial_gap(point) < 1e-9);
            }
        }
    }

    /// The angular coordinate advances with the surface's own revolution
    /// parameter — the fact the deck generator is built on.
    #[test]
    fn the_angular_coordinate_tracks_the_surface_parameter() {
        let revo = z_cone(0.5, 1.0, 4.0);
        let cone = expect_cone(identify_cone(&revo));
        let schema = cone.schema();
        let mut previous = schema.angular_coordinate(revo.subs(0.3, 0.0));
        for v in [0.4_f64, 0.8, 1.2, 1.6] {
            let theta = schema.angular_coordinate(revo.subs(0.3, v));
            assert!(
                theta > previous,
                "theta must advance with v: {theta} <= {previous}"
            );
            previous = theta;
        }
    }

    /// A translated, rotated cone certifies, and its apex is found where the
    /// placement actually puts it.
    #[test]
    fn a_placed_cone_is_certified_and_its_apex_follows_the_placement() {
        let axis = Vector3::new(1.0, -2.0, 2.0).normalize();
        let apex = Point3::new(-4.0, 7.0, 1.5);
        let perp = axis.cross(Vector3::new(0.0, 0.0, 1.0)).normalize();
        let slope = 0.75;
        // Two points on one generating ray, at generator coordinates 2 and 6.
        let on_ray = |s: f64| apex + s * axis + slope * s * perp;
        let revo = RevolutedCurve::by_revolution(
            Line(on_ray(2.0), on_ray(6.0)),
            // The revolution origin is deliberately *not* the apex: an
            // exporter may put it anywhere on the axis, and the certificate
            // must locate the apex rather than inherit it.
            apex + 11.0 * axis,
            axis,
        );
        let cone = expect_cone(identify_cone(&revo));
        assert!((cone.schema().apex() - apex).magnitude() < 1e-8);
        assert!((cone.schema().slope().get() - slope).abs() < 1e-9);
    }

    /// A generatrix spanning the apex is still one cone; the apex lands
    /// strictly inside the declared generatrix, and the two declared endpoints
    /// are on opposite nappes.
    #[test]
    fn a_generatrix_spanning_the_apex_still_locates_it() {
        let cone = expect_cone(identify_cone(&z_cone(0.5, -3.0, 4.0)));
        let schema = cone.schema();
        assert!((schema.apex() - Point3::new(0.0, 0.0, 0.0)).magnitude() < 1e-9);
        let Line(p, q) = schema.generatrix();
        assert_eq!(
            schema.nappe_of(schema.generator_coordinate(p)),
            Some(Nappe::Negative)
        );
        assert_eq!(
            schema.nappe_of(schema.generator_coordinate(q)),
            Some(Nappe::Positive)
        );
    }

    #[test]
    fn a_cylinder_is_refused_by_name() {
        let revo = RevolutedCurve::by_revolution(
            Line(Point3::new(2.0, 0.0, 0.0), Point3::new(2.0, 0.0, 5.0)),
            Point3::new(0.0, 0.0, 0.0),
            Vector3::new(0.0, 0.0, 1.0),
        );
        assert_eq!(
            expect_refusal(identify_cone(&revo)),
            ConeIdentificationFailure::CylindricalRevolution
        );
    }

    #[test]
    fn a_generatrix_perpendicular_to_the_axis_is_refused_by_name() {
        // A radial segment: revolving it sweeps a planar annulus.
        let revo = RevolutedCurve::by_revolution(
            Line(Point3::new(1.0, 0.0, 3.0), Point3::new(4.0, 0.0, 3.0)),
            Point3::new(0.0, 0.0, 0.0),
            Vector3::new(0.0, 0.0, 1.0),
        );
        assert_eq!(
            expect_refusal(identify_cone(&revo)),
            ConeIdentificationFailure::GeneratrixPerpendicularToAxis
        );
    }

    /// The hyperboloid. Tilted, so not a cylinder; skew, so it never meets the
    /// axis and has no apex. Its trimmed faces can present the same boundary
    /// signature as a frustum's, so refusing it by name rather than fitting a
    /// cone to it is the point of the test.
    #[test]
    fn a_skew_generatrix_is_refused_as_a_hyperboloid_not_fitted_to_a_cone() {
        let revo = RevolutedCurve::by_revolution(
            // Offset in y, so the projected ray misses the axis by 2.
            Line(Point3::new(1.0, 2.0, 0.0), Point3::new(4.0, 2.0, 5.0)),
            Point3::new(0.0, 0.0, 0.0),
            Vector3::new(0.0, 0.0, 1.0),
        );
        assert_eq!(
            expect_refusal(identify_cone(&revo)),
            ConeIdentificationFailure::GeneratrixSkewToAxis
        );
    }

    #[test]
    fn a_generatrix_on_the_axis_is_refused() {
        let revo = RevolutedCurve::by_revolution(
            Line(Point3::new(0.0, 0.0, 1.0), Point3::new(0.0, 0.0, 5.0)),
            Point3::new(0.0, 0.0, 0.0),
            Vector3::new(0.0, 0.0, 1.0),
        );
        // Parallel to the axis and on it: the cylinder arm catches it first,
        // which is correct — it is not a cone for that reason too.
        assert_eq!(
            expect_refusal(identify_cone(&revo)),
            ConeIdentificationFailure::CylindricalRevolution
        );
    }

    #[test]
    fn a_degenerate_generatrix_is_refused() {
        let revo = RevolutedCurve::by_revolution(
            Line(Point3::new(2.0, 0.0, 1.0), Point3::new(2.0, 0.0, 1.0)),
            Point3::new(0.0, 0.0, 0.0),
            Vector3::new(0.0, 0.0, 1.0),
        );
        assert_eq!(
            expect_refusal(identify_cone(&revo)),
            ConeIdentificationFailure::DegenerateGeneratrix
        );
    }

    /// A zero revolution axis is refused, and the *name* it is refused under
    /// records where the degeneracy became observable.
    /// `RevolutedCurve::by_revolution` normalizes the axis it is handed, so a
    /// zero vector has already become `NaN` by the time the identifier reads
    /// it back — which is a true statement about the representation and is
    /// reported as one. [`ConeIdentificationFailure::DegenerateAxis`] remains
    /// the verdict for a representation that reaches this function with a
    /// genuinely zero-magnitude axis intact.
    #[test]
    fn a_degenerate_axis_is_refused() {
        let revo = RevolutedCurve::by_revolution(
            Line(Point3::new(1.0, 0.0, 1.0), Point3::new(2.0, 0.0, 3.0)),
            Point3::new(0.0, 0.0, 0.0),
            Vector3::new(0.0, 0.0, 0.0),
        );
        assert!(matches!(
            expect_refusal(identify_cone(&revo)),
            ConeIdentificationFailure::NonFiniteCoordinate { .. }
                | ConeIdentificationFailure::DegenerateAxis
        ));
    }

    /// An inverted cone — the representation `truck-stepio` actually produces,
    /// since its `ConicalSurface` conversion calls `Processor::invert()` — is
    /// the same cone with the axis negated. The apex is unchanged; the nappe
    /// labels swap, which is exactly what negating the axis means.
    #[test]
    fn an_inverted_cone_is_the_same_cone_with_the_nappes_relabelled() {
        let forward = expect_cone(identify_cone(&z_cone(0.5, 1.0, 4.0)));
        let inverted = expect_cone(identify_cone(&RevolutedCurve::by_revolution(
            Line(Point3::new(0.5, 0.0, 1.0), Point3::new(2.0, 0.0, 4.0)),
            Point3::new(0.0, 0.0, 0.0),
            Vector3::new(0.0, 0.0, -1.0),
        )));
        assert!((forward.schema().apex() - inverted.schema().apex()).magnitude() < 1e-9);
        assert!((forward.schema().slope().get() - inverted.schema().slope().get()).abs() < 1e-9);
        assert!((forward.schema().axis() + inverted.schema().axis()).magnitude() < 1e-12);
        let sample = forward.schema().point_at(2.0, 0.4);
        assert_eq!(
            forward
                .schema()
                .nappe_of(forward.schema().generator_coordinate(sample)),
            Some(Nappe::Positive)
        );
        assert_eq!(
            inverted
                .schema()
                .nappe_of(inverted.schema().generator_coordinate(sample)),
            Some(Nappe::Negative)
        );
    }
}
