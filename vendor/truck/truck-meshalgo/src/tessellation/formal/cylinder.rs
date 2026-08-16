//! Certified identification of an embedded cylinder support surface.
//!
//! # Scope
//!
//! This is the rank-1 analog of [`super::support`]'s plane identification. A
//! STEP cylinder reaches the tessellator as
//! `Processor<RevolutedCurve<Line<Point3>>>` (see `truck-stepio`'s
//! `geom_impls.rs`: a `RevolutedCurve<Line>` whose generatrix is parallel to the
//! axis becomes an `ElementarySurface::CylindricalSurface`). [`identify_cylinder`]
//! reads the inner [`RevolutedCurve<Line<Point3>>`] *structurally* — before the
//! legacy types erase the representation — and either certifies an embedded
//! cylinder or refuses with a named reason.
//!
//! # The parameter-convention hazard
//!
//! `RevolutedCurve<C>::subs(u, v) = origin + R_axis(v) * (curve.subs(u) - origin)`,
//! so `u` parameterizes the generatrix and `v` the revolution angle. For a
//! cylinder the generatrix is a line parallel to the axis, so:
//!
//! - `u` is the **axial** (aperiodic) parameter,
//! - `v` is the **angular** (periodic) parameter, with period `2π`.
//!
//! `RevolutedCurve::invert()` negates the revolution axis (it does **not** swap
//! `u` and `v`), so `v` remains the periodic coordinate after the importer's
//! inversion. This module nevertheless **verifies the convention by evaluation**
//! rather than trusting the type: it checks `v_period()` / `u_period()`, samples
//! `subs(u, v) == subs(u, v + 2π)`, and confirms the `u`-derivative is parallel
//! to the axis. A surface whose convention cannot be verified is refused, never
//! guessed.
//!
//! # The developed coordinate system
//!
//! The cylinder develops into a plane with two coordinates:
//!
//! - the **axial** coordinate `z = (x - origin) · axis`,
//! - the **angular** coordinate `theta`, defined so that `theta == v` in the
//!   recorded radial frame: `radial_x` is the unit radial direction of the
//!   generatrix point at `v = 0`, and `radial_y = axis x radial_x`, giving
//!   `theta = atan2((x - origin) . radial_y, (x - origin) . radial_x)` equal to
//!   `v` on the cylinder.
//!
//! The deck group translates `theta` by the period, so the deck generator lives
//! on the angular (developed-second) axis. See [`CylinderSchema::deck_generator`].

use super::deck::{DeckConstructorFailure, DeckGenerator, DevelopedAxis};
use super::numeric::{FiniteF64, NumericDomainError, PositiveFinite};
use std::f64::consts::TAU;
use truck_geometry::prelude::{
    InnerSpace, Line, ParametricSurface, Point3, RevolutedCurve, Vector3,
};

/// Dimensionless sine floor below which a generatrix line counts as parallel to
/// the axis (cylinder) rather than tilted (cone or general revolute).
///
/// `|d x axis| / (|d| |axis|)` is the sine of the angle between the generatrix
/// direction and the axis; `1e-9` is ~1.8e-3 degrees, six orders of magnitude
/// clear of `f64::EPSILON`-scale summation error in the cross product.
pub const MINIMUM_CYLINDER_LINE_AXIS_PARALLELISM: f64 = 1e-9;

/// Which surface parameter carries the revolution angle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeriodicParameter {
    /// The first parameter `u` is the angle.
    U,
    /// The second parameter `v` is the angle. The verified truck convention.
    V,
}

impl PeriodicParameter {
    /// The aperiodic (axial) parameter.
    pub fn aperiodic(self) -> Self {
        match self {
            Self::U => Self::V,
            Self::V => Self::U,
        }
    }

    /// A short stable tag, for probe records.
    pub fn tag(self) -> &'static str {
        match self {
            Self::U => "periodic_u",
            Self::V => "periodic_v",
        }
    }
}

/// Why a revolved-line surface was not certified as an embedded cylinder.
#[derive(Debug, Clone, PartialEq)]
pub enum CylinderIdentificationFailure {
    /// A coordinate was `NaN` or infinite.
    NonFiniteCoordinate {
        /// Why the value was refused.
        cause: NumericDomainError,
    },
    /// The revolution axis has (near-)zero magnitude, so it defines no line.
    DegenerateAxis,
    /// The generatrix `Line` has coincident endpoints, so it has no direction.
    DegenerateGeneratrix,
    /// The generatrix is not parallel to the axis: the surface is a cone or a
    /// general revolute, not a cylinder.
    ConeOrNonCylindricalRevolution,
    /// The generatrix lies on the axis (radius zero), so the revolution is not a
    /// regular embedded cylinder.
    DegenerateRadius,
    /// The period evidence did not verify the expected `2π` angular period, or
    /// the parameter convention could not be confirmed by evaluation.
    UnverifiedParameterConvention,
}

/// The result of reading a revolved-line surface structurally.
#[derive(Debug, Clone, PartialEq)]
pub enum CylinderIdentification {
    /// An embedded cylinder was certified.
    Cylinder(CertifiedEmbeddedCylinder),
    /// The surface is not an embedded cylinder. Carries the reason.
    NotACylinder(CylinderIdentificationFailure),
}

/// A certificate that a surface is a regular embedded cylinder, with the deck
/// generator that its quotient defines.
///
/// The only constructor is [`identify_cylinder`]; fields are private, so the
/// obligations — positive radius, nondegenerate axis, generatrix parallel to the
/// axis, a verified `2π` angular period with no second independent period, and a
/// regular parameter map — are discharged by presenting the representation, not
/// by assembling numbers.
#[derive(Debug, Clone, PartialEq)]
pub struct CertifiedEmbeddedCylinder {
    schema: CylinderSchema,
    certificate: CylinderValidityCertificate,
}

impl CertifiedEmbeddedCylinder {
    /// The structural schema.
    pub fn schema(&self) -> &CylinderSchema {
        &self.schema
    }

    /// The validity obligations discharged.
    pub fn certificate(&self) -> &CylinderValidityCertificate {
        &self.certificate
    }

    /// The deck generator: the angular period on the developed-second axis.
    pub fn deck_generator(&self) -> DeckGenerator {
        self.schema.deck_generator
    }

    /// A short stable tag, for probe records.
    pub fn tag(&self) -> &'static str {
        "embedded_cylinder"
    }
}

/// The structural data of a certified embedded cylinder.
#[derive(Debug, Clone, PartialEq)]
pub struct CylinderSchema {
    origin: Point3,
    axis: Vector3,
    radial_x: Vector3,
    radial_y: Vector3,
    radius: PositiveFinite,
    generatrix: Line<Point3>,
    periodic_parameter: PeriodicParameter,
    period: PositiveFinite,
    deck_generator: DeckGenerator,
}

impl CylinderSchema {
    /// A point on the axis (the revolution origin).
    pub fn origin(&self) -> Point3 {
        self.origin
    }

    /// The unit axis direction.
    pub fn axis(&self) -> Vector3 {
        self.axis
    }

    /// The unit radial direction at angular parameter zero (`radial_x`).
    pub fn radial_x(&self) -> Vector3 {
        self.radial_x
    }

    /// The second radial basis vector `axis x radial_x`.
    pub fn radial_y(&self) -> Vector3 {
        self.radial_y
    }

    /// The cylinder radius, proved strictly positive.
    pub fn radius(&self) -> PositiveFinite {
        self.radius
    }

    /// The generatrix line the surface revolves.
    pub fn generatrix(&self) -> Line<Point3> {
        self.generatrix
    }

    /// Which parameter carries the revolution angle.
    pub fn periodic_parameter(&self) -> PeriodicParameter {
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

    /// The axial coordinate of a physical point: `(x - origin) . axis`.
    pub fn axial_coordinate(&self, x: Point3) -> f64 {
        (x - self.origin).dot(self.axis)
    }

    /// The angular coordinate of a physical point on the cylinder, in the
    /// recorded frame, equal to the surface's angular parameter `v`.
    ///
    /// Undefined off the cylinder; callers must have certified the point lies on
    /// the surface (Step 4 does this).
    pub fn angular_coordinate(&self, x: Point3) -> f64 {
        let r = x - self.origin;
        r.dot(self.radial_y).atan2(r.dot(self.radial_x))
    }
}

/// Marker that the embedded-cylinder validity obligations were discharged by
/// [`identify_cylinder`]: positive radius, nondegenerate axis, generatrix
/// parallel to the axis, a regular parameter map, a single `2π` angular period
/// (rank 1), and an embedded quotient. Carried as an opaque token because the
/// only producer is the identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CylinderValidityCertificate {
    #[allow(dead_code)]
    discharged: bool,
}

impl CylinderValidityCertificate {
    /// Constructed only by [`identify_cylinder`].
    fn discharged() -> Self {
        Self { discharged: true }
    }
}

/// Read a revolved-line surface structurally and certify an embedded cylinder.
///
/// The single introduction rule for [`CertifiedEmbeddedCylinder`]. It refuses a
/// cone (generatrix not parallel to the axis), a degenerate radius or axis, a
/// non-finite coordinate, and any surface whose `2π` angular period it cannot
/// verify by evaluation.
pub fn identify_cylinder(revo: &RevolutedCurve<Line<Point3>>) -> CylinderIdentification {
    let origin = revo.origin();
    let axis_raw = revo.axis();
    let line = *revo.entity_curve();
    let Line(p, q) = line;
    let d = q - p;

    // --- finiteness -------------------------------------------------------
    let finite = |v: f64| FiniteF64::new(v).map(FiniteF64::get);
    for coordinate in [
        origin.x, origin.y, origin.z, axis_raw.x, axis_raw.y, axis_raw.z, p.x, p.y, p.z, q.x, q.y,
        q.z,
    ] {
        if let Err(cause) = finite(coordinate) {
            return CylinderIdentification::NotACylinder(
                CylinderIdentificationFailure::NonFiniteCoordinate { cause },
            );
        }
    }

    // --- axis nondegenerate ----------------------------------------------
    let axis_norm = axis_raw.magnitude();
    if !(axis_norm > 0.0) {
        return CylinderIdentification::NotACylinder(CylinderIdentificationFailure::DegenerateAxis);
    }
    let axis = axis_raw / axis_norm;

    // --- generatrix direction nondegenerate ------------------------------
    let d_norm = d.magnitude();
    if !(d_norm > 0.0) {
        return CylinderIdentification::NotACylinder(
            CylinderIdentificationFailure::DegenerateGeneratrix,
        );
    }

    // --- generatrix parallel to axis (cylinder, not cone) ----------------
    let sine = d.cross(axis).magnitude() / (d_norm * axis_norm);
    if !(sine < MINIMUM_CYLINDER_LINE_AXIS_PARALLELISM) {
        return CylinderIdentification::NotACylinder(
            CylinderIdentificationFailure::ConeOrNonCylindricalRevolution,
        );
    }

    // --- radius: perpendicular distance from the generatrix to the axis --
    let rp = p - origin;
    let radial_vec_p = rp - rp.dot(axis) * axis;
    let radius_p = radial_vec_p.magnitude();
    let rq = q - origin;
    let radial_vec_q = rq - rq.dot(axis) * axis;
    let radius_q = radial_vec_q.magnitude();
    // Constant radius along the line is implied by `d parallel axis`; verify it
    // numerically against summation error before trusting it.
    let radius = radius_p.max(radius_q);
    if !(radius > 0.0) {
        return CylinderIdentification::NotACylinder(
            CylinderIdentificationFailure::DegenerateRadius,
        );
    }
    let radius_drift = (radius_p - radius_q).abs() / radius;
    if !(radius_drift < MINIMUM_CYLINDER_LINE_AXIS_PARALLELISM) {
        return CylinderIdentification::NotACylinder(
            CylinderIdentificationFailure::ConeOrNonCylindricalRevolution,
        );
    }
    let radial_x = radial_vec_p / radius_p;
    let radial_y = axis.cross(radial_x);

    // --- parameter convention, verified by evaluation --------------------
    let convention = match verify_angular_convention(revo, axis, radial_x, radial_y, origin, radius)
    {
        Some(periodic) => periodic,
        None => {
            return CylinderIdentification::NotACylinder(
                CylinderIdentificationFailure::UnverifiedParameterConvention,
            );
        }
    };

    let Ok(period_finite) = FiniteF64::new(TAU) else {
        return CylinderIdentification::NotACylinder(
            CylinderIdentificationFailure::UnverifiedParameterConvention,
        );
    };
    let Ok(period) = PositiveFinite::new(period_finite.get()) else {
        return CylinderIdentification::NotACylinder(
            CylinderIdentificationFailure::UnverifiedParameterConvention,
        );
    };

    // The developed plane is (axial = First, angular = Second). The deck
    // generator translates the angular coordinate by the period. The sign is
    // positive in the angular parameter: one revolution is +period in `theta`.
    let deck_generator = match DeckGenerator::new(DevelopedAxis::Second, period_finite) {
        Ok(g) => g,
        Err(DeckConstructorFailure::ZeroPeriod) | Err(DeckConstructorFailure::Numeric(_)) => {
            return CylinderIdentification::NotACylinder(
                CylinderIdentificationFailure::UnverifiedParameterConvention,
            );
        }
        Err(DeckConstructorFailure::BoundsInverted) => {
            // Unreachable: a single FiniteF64 is not an interval.
            return CylinderIdentification::NotACylinder(
                CylinderIdentificationFailure::UnverifiedParameterConvention,
            );
        }
    };

    let Ok(radius_pf) = PositiveFinite::new(radius) else {
        return CylinderIdentification::NotACylinder(
            CylinderIdentificationFailure::DegenerateRadius,
        );
    };

    CylinderIdentification::Cylinder(CertifiedEmbeddedCylinder {
        schema: CylinderSchema {
            origin,
            axis,
            radial_x,
            radial_y,
            radius: radius_pf,
            generatrix: line,
            periodic_parameter: convention,
            period,
            deck_generator,
        },
        certificate: CylinderValidityCertificate::discharged(),
    })
}

/// Confirm by evaluation that one parameter is the `2π` angular parameter and
/// the other is axial, returning which is angular.
///
/// Checks the surface's reported `u_period`/`v_period`, then verifies
/// `subs(u, v) == subs(u, v + 2π)` (angular closure) and that the axial
/// derivative is parallel to the axis. Sampling diagnoses or rejects; it never
/// certifies on its own — the structural `2π` period from `v_period()` carries
/// the authority, and these evaluations confirm the convention is the expected one.
fn verify_angular_convention(
    revo: &RevolutedCurve<Line<Point3>>,
    axis: Vector3,
    radial_x: Vector3,
    radial_y: Vector3,
    origin: Point3,
    radius: f64,
) -> Option<PeriodicParameter> {
    let u_period = revo.u_period();
    let v_period = revo.v_period();
    // The truck contract: v carries the 2π revolution, u carries the line
    // (aperiodic). Accept the convention only when the accessors agree.
    let candidate = match (u_period, v_period) {
        (None, Some(vp)) if (vp - TAU).abs() < 1e-9 * TAU => PeriodicParameter::V,
        (Some(up), None) if (up - TAU).abs() < 1e-9 * TAU => PeriodicParameter::U,
        _ => return None,
    };

    // Confirm by evaluation at an interior sample point.
    let (u0, v0) = (0.5, 0.3);
    let p_base = revo.subs(u0, v0);

    match candidate {
        PeriodicParameter::V => {
            // Angular closure in v: subs(u, v) == subs(u, v + 2π).
            let p_shift = revo.subs(u0, v0 + TAU);
            if (p_shift - p_base).magnitude() > 1e-9 * radius.max(1.0) {
                return None;
            }
            // Axial derivative in u is parallel to the axis.
            let du = revo.uder(u0, v0);
            if du.magnitude() < f64::EPSILON {
                return None;
            }
            let du_sine = du.cross(axis).magnitude() / (du.magnitude() * axis.magnitude());
            if du_sine > 1e-9 {
                return None;
            }
            // The sample point lies on the cylinder at the expected angle.
            if !point_on_cylinder(p_base, origin, axis, radial_x, radial_y, radius) {
                return None;
            }
            Some(PeriodicParameter::V)
        }
        PeriodicParameter::U => {
            let p_shift = revo.subs(u0 + TAU, v0);
            if (p_shift - p_base).magnitude() > 1e-9 * radius.max(1.0) {
                return None;
            }
            let dv = revo.vder(u0, v0);
            if dv.magnitude() < f64::EPSILON {
                return None;
            }
            let dv_sine = dv.cross(axis).magnitude() / (dv.magnitude() * axis.magnitude());
            if dv_sine > 1e-9 {
                return None;
            }
            if !point_on_cylinder(p_base, origin, axis, radial_x, radial_y, radius) {
                return None;
            }
            Some(PeriodicParameter::U)
        }
    }
}

fn point_on_cylinder(
    x: Point3,
    origin: Point3,
    axis: Vector3,
    radial_x: Vector3,
    radial_y: Vector3,
    radius: f64,
) -> bool {
    let r = x - origin;
    let axial = r.dot(axis);
    let radial = r - axial * axis;
    let rx = radial.dot(radial_x);
    let ry = radial.dot(radial_y);
    let fit = (rx.hypot(ry) - radius).abs();
    fit < 1e-9 * radius.max(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A cylinder of `radius` and height `h` along the z-axis, generatrix from
    /// `(radius, 0, 0)` to `(radius, 0, h)`.
    fn z_cylinder(radius: f64, h: f64) -> RevolutedCurve<Line<Point3>> {
        RevolutedCurve::by_revolution(
            Line(Point3::new(radius, 0.0, 0.0), Point3::new(radius, 0.0, h)),
            Point3::new(0.0, 0.0, 0.0),
            Vector3::new(0.0, 0.0, 1.0),
        )
    }

    fn expect_cylinder(id: CylinderIdentification) -> CertifiedEmbeddedCylinder {
        match id {
            CylinderIdentification::Cylinder(c) => c,
            other => panic!("expected a cylinder, got {other:?}"),
        }
    }

    #[test]
    fn ordinary_cylinder_is_certified() {
        let c = expect_cylinder(identify_cylinder(&z_cylinder(2.0, 5.0)));
        let s = c.schema();
        assert!((s.radius().get() - 2.0).abs() < 1e-12);
        assert!((s.axis() - Vector3::new(0.0, 0.0, 1.0)).magnitude() < 1e-12);
        assert_eq!(s.periodic_parameter(), PeriodicParameter::V);
        assert!((s.period().get() - TAU).abs() < 1e-12);
        assert_eq!(c.deck_generator().periodic_axis(), DevelopedAxis::Second);
    }

    #[test]
    fn angular_coordinate_equals_the_surface_parameter() {
        // In the recorded frame, atan2 recovers v exactly on the cylinder.
        let c = expect_cylinder(identify_cylinder(&z_cylinder(3.0, 4.0)));
        let s = c.schema();
        let revo = z_cylinder(3.0, 4.0);
        for v in [0.0_f64, 0.5, 1.7, 2.8, TAU - 0.1] {
            let x = revo.subs(0.3, v);
            let theta = s.angular_coordinate(x);
            // Normalize the comparison into [0, 2π).
            let mut d = (theta - v).abs();
            if d > TAU {
                d -= TAU;
            }
            let diff = (theta.rem_euclid(TAU) - v.rem_euclid(TAU)).abs();
            assert!(diff < 1e-9 || d < 1e-9, "v={v}: theta={theta}");
        }
    }

    #[test]
    fn translated_and_rotated_cylinder_is_certified() {
        // Axis along (1,1,1)/sqrt3, origin shifted, generatrix parallel to axis.
        let axis = Vector3::new(1.0, 1.0, 1.0).normalize();
        let origin = Point3::new(10.0, -3.0, 4.0);
        // A point at radius 1.5 from the axis: offset perpendicular to axis.
        let perp = Vector3::new(1.0, -1.0, 0.0).normalize();
        let p = origin + 1.5 * perp;
        let q = p + 7.0 * axis;
        let revo = RevolutedCurve::by_revolution(Line(p, q), origin, axis);
        let c = expect_cylinder(identify_cylinder(&revo));
        assert!(
            (c.schema().radius().get() - 1.5).abs() < 1e-9,
            "{:?}",
            c.schema().radius()
        );
        assert_eq!(c.schema().periodic_parameter(), PeriodicParameter::V);
    }

    #[test]
    fn inverted_cylinder_is_still_a_cylinder() {
        // An inverted cylinder has its axis negated; the surface is the same
        // cylinder. `RevolutedCurve::invert()` negates the revolution axis, so we
        // build that representation directly (avoids the geotrait `Invertible`
        // dependency, which truck-meshalgo does not pull in).
        let revo = RevolutedCurve::by_revolution(
            Line(Point3::new(2.0, 0.0, 0.0), Point3::new(2.0, 0.0, 5.0)),
            Point3::new(0.0, 0.0, 0.0),
            Vector3::new(0.0, 0.0, -1.0),
        );
        let c = expect_cylinder(identify_cylinder(&revo));
        assert!((c.schema().radius().get() - 2.0).abs() < 1e-12);
        assert_eq!(c.schema().periodic_parameter(), PeriodicParameter::V);
        // The axis is the negated z-axis.
        assert!((c.schema().axis() + Vector3::new(0.0, 0.0, 1.0)).magnitude() < 1e-12);
    }

    #[test]
    fn cone_is_refused() {
        // Generatrix from (2,0,0) to (0,0,5): tilted, meeting the axis -> a cone.
        let revo = RevolutedCurve::by_revolution(
            Line(Point3::new(2.0, 0.0, 0.0), Point3::new(0.0, 0.0, 5.0)),
            Point3::new(0.0, 0.0, 0.0),
            Vector3::new(0.0, 0.0, 1.0),
        );
        assert!(matches!(
            identify_cylinder(&revo),
            CylinderIdentification::NotACylinder(
                CylinderIdentificationFailure::ConeOrNonCylindricalRevolution
            )
        ));
    }

    #[test]
    fn degenerate_radius_is_refused() {
        // Generatrix on the axis: radius zero, revolution is a line not a surface.
        let revo = RevolutedCurve::by_revolution(
            Line(Point3::new(0.0, 0.0, 0.0), Point3::new(0.0, 0.0, 5.0)),
            Point3::new(0.0, 0.0, 0.0),
            Vector3::new(0.0, 0.0, 1.0),
        );
        assert!(matches!(
            identify_cylinder(&revo),
            CylinderIdentification::NotACylinder(CylinderIdentificationFailure::DegenerateRadius)
        ));
    }

    #[test]
    fn degenerate_generatrix_is_refused() {
        // Coincident endpoints: no direction.
        let revo = RevolutedCurve::by_revolution(
            Line(Point3::new(2.0, 0.0, 0.0), Point3::new(2.0, 0.0, 0.0)),
            Point3::new(0.0, 0.0, 0.0),
            Vector3::new(0.0, 0.0, 1.0),
        );
        assert!(matches!(
            identify_cylinder(&revo),
            CylinderIdentification::NotACylinder(
                CylinderIdentificationFailure::DegenerateGeneratrix
            )
        ));
    }

    #[test]
    fn deck_generator_period_matches_two_pi() {
        let c = expect_cylinder(identify_cylinder(&z_cylinder(1.0, 1.0)));
        let g = c.deck_generator();
        assert_eq!(g.periodic_axis(), DevelopedAxis::Second);
        assert!((g.period_magnitude().get() - TAU).abs() < 1e-12);
    }
}
