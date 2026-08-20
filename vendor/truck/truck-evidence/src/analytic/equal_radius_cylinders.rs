//! BG-ANA-001-EQRCYL: equal-radius cylinders with intersecting axes — the
//! classic exact case: **two ellipses**, rational parameterization, no
//! iteration, no approximation.
//!
//! Two cylinders of equal radius `r` whose axes are coplanar and intersect at
//! a point `q` at a nonzero angle θ meet in two ellipses, both centred at `q`,
//! each lying in one of the two planes bisecting the angle between the axes
//! (BG-ANA-001's "two ellipses" table row):
//!
//! - the ellipse in the **internal** bisector plane, spanned by the internal
//!   bisector `b̂+ = normalize(a0 + a1)` and `û = normalize(a0 × a1)`, has
//!   semi-major `r / sin(θ/2)` along `b̂+` and semi-minor `r` along `û`;
//! - the ellipse in the **external** bisector plane, spanned by
//!   `b̂− = normalize(a0 − a1)` and `û`, has semi-major `r / cos(θ/2)` along
//!   `b̂−` and semi-minor `r` along `û`.
//!
//! The two semi-major axes differ for θ ≠ π/2; the perpendicular Steinmetz
//! case (θ = π/2) is the degeneracy where `sin(θ/2) = cos(θ/2) = 1/√2` and
//! both ellipses are congruent with semi-axes `r` and `r√2`. The packet's
//! decision 5 claimed `r / cos(θ/2)` for *both* planes; the internal-bisector
//! claim is corrected here to `r / sin(θ/2)`, verified against the
//! on-both-carriers distance test in the tests below and recorded as a
//! deviation/disagreement in RESULT.json.
//!
//! **What `Method::Exact` means here:** the classification is exact — every
//! predicate (zero direction, parallel axes, coplanarity) is decided by
//! outward-rounded interval arithmetic on the f64 carrier parameters, and an
//! undecidable enclosure straddle is a `NumericallyUnresolved` refusal, never a
//! guess. The emitted ellipses are the closed-form intersections: coordinates
//! are computed in f64, and the spec's obligation is "lies on both carriers to
//! machine precision", asserted in the tests with an H-3-commented slack.
//! There is no `τ_rep` anywhere in this module.
//!
//! Refused, never approximated: unequal radii belong to the general solver,
//! and parallel or skew (non-coplanar) axes belong to sibling cells.

use std::cmp::Ordering;
use std::f64::consts::TAU;

use inari::Interval;
use truck_base::cgmath64::{InnerSpace, Matrix4, Point3, Vector3, Vector4};
use truck_base::evidence::{
    Budget, Certificate, Certified, EnvelopeCase, Margin, Method, Modulus, Prop, PropMap, Refusal,
    Truth, UnresolvedWitness,
};
use truck_geometry::decorators::{Processor, TrimmedCurve};
use truck_geometry::specifieds::UnitCircle;

use crate::analytic::{AnalyticIntersection, AnalyticOutcome, ExactCurve, PlacedCircle};

/// Equal-radius cylinders with intersecting axes: the two intersection
/// ellipses, or a typed refusal for an out-of-cell placement.
///
/// `axis.0` is a point on the line and `axis.1` its direction (need not be
/// unit; it is normalized internally). `radius` is the shared cylinder radius.
/// A zero-length direction refuses as `ChartDegenerate`; parallel axes and
/// skew (non-coplanar) axes refuse as `NonCanonicalCarrier`; an undecidable
/// interval enclosure refuses as `NumericallyUnresolved`. Every `Ok` is the
/// exact two-ellipse answer with `Method::Exact` (see the module docs).
pub fn equal_radius_cylinders(
    radius: f64,
    axis0: &(Point3, Vector3),
    axis1: &(Point3, Vector3),
) -> AnalyticOutcome {
    let (p0, d0) = (axis0.0, axis0.1);
    let (p1, d1) = (axis1.0, axis1.1);

    // A zero-length direction is a chart degeneracy (§9.1): the carrier is
    // ill-formed, not a placement of this cell.
    let a0 = normalize_axis(d0)?;
    let a1 = normalize_axis(d1)?;

    // Step 1 — parallel axes. `a0 × a1` per component in interval arithmetic:
    // all three decisively zero means the axes are parallel (this placement
    // belongs to the parallel-axis / coaxial cells), any component decisively
    // nonzero means they are not, and a straddling component is undecidable.
    let [cx, cy, cz] = cross_intervals(a0, a1);
    let all_zero = decisively_zero(cx) && decisively_zero(cy) && decisively_zero(cz);
    if all_zero {
        return Err(Refusal::UnsupportedEnvelope(
            EnvelopeCase::NonCanonicalCarrier,
        ));
    }
    let definitely_not_parallel = excludes_zero(cx) || excludes_zero(cy) || excludes_zero(cz);
    if !definitely_not_parallel {
        return Err(Refusal::NumericallyUnresolved {
            spent: Budget::new(0, 0, 0),
            witness: UnresolvedWitness::RootNotIsolated,
        });
    }

    // Step 2 — coplanar (intersecting) axes. The scalar triple product
    // `τ = (a0 × a1) · (p1 − p0)` in interval arithmetic is decisive zero
    // exactly when the non-parallel lines meet; decisively nonzero means they
    // are skew (non-coplanar), which belongs to another cell.
    let delta = p1 - p0;
    let tau = cx * ival(delta.x) + cy * ival(delta.y) + cz * ival(delta.z);
    match three_way(tau, ival(0.0)) {
        Some(Ordering::Equal) => {}
        Some(Ordering::Less) | Some(Ordering::Greater) => {
            return Err(Refusal::UnsupportedEnvelope(
                EnvelopeCase::NonCanonicalCarrier,
            ));
        }
        None => {
            return Err(Refusal::NumericallyUnresolved {
                spent: Budget::new(0, 0, 0),
                witness: UnresolvedWitness::RootNotIsolated,
            });
        }
    }

    // Step 3 — the intersection point `q`, f64 closed form: the closest-point
    // formula specialized to intersecting lines,
    // `q = p0 + ((p1 − p0) × a1) · (a0 × a1) / |a0 × a1|² · a0`.
    let d = a0.cross(a1);
    let denom = d.dot(d);
    let t = delta.cross(a1).dot(d) / denom;
    let q = p0 + a0 * t;

    // Step 4 — the two ellipses. With `û = normalize(a0 × a1)`,
    // `b̂+ = normalize(a0 + a1)` and `b̂− = normalize(a0 − a1)`:
    //   e0: centre `q`, semi-major `r / sin(θ/2)` along `b̂+`, semi-minor `r`
    //       along `û` (internal bisector plane);
    //   e1: centre `q`, semi-major `r / cos(θ/2)` along `b̂−`, semi-minor `r`
    //       along `û` (external bisector plane).
    let cos_theta = a0.dot(a1);
    let sin_half = ((1.0 - cos_theta) / 2.0).sqrt();
    let cos_half = ((1.0 + cos_theta) / 2.0).sqrt();
    let u_hat = d.normalize();
    let bp = (a0 + a1).normalize();
    let bm = (a0 - a1).normalize();

    let e0 = placed_ellipse(bp, u_hat, q, radius / sin_half, radius);
    let e1 = placed_ellipse(bm, u_hat, q, radius / cos_half, radius);

    let mut props = PropMap::new();
    props.set(Prop::AnalyticCarrier, Truth::True);
    Ok(Certified::new(
        AnalyticIntersection::TwoCurves([ExactCurve::Ellipse(e0), ExactCurve::Ellipse(e1)]),
        Certificate {
            props,
            method: Method::Exact,
            budget_left: Budget::new(0, 0, 0),
            margin: Margin::UNBOUNDED,
            modulus: Modulus::Unbounded,
        },
    ))
}

/// A degenerate interval from an f64 component. A non-finite component is a
/// caller error; `Interval::EMPTY` then propagates through the predicates and
/// refuses downstream rather than panicking.
fn ival(x: f64) -> Interval {
    Interval::try_from((x, x)).unwrap_or(Interval::EMPTY)
}

/// Whether the interval is exactly the single point `0`.
fn decisively_zero(i: Interval) -> bool {
    i.inf() == 0.0 && i.sup() == 0.0
}

/// Whether the interval lies entirely on one side of `0`.
fn excludes_zero(i: Interval) -> bool {
    i.inf() > 0.0 || i.sup() < 0.0
}

/// The three-way ordering of two intervals: `Some` exactly when the relation
/// is forced by the enclosures, `None` when they straddle (undecidable).
fn three_way(a: Interval, b: Interval) -> Option<Ordering> {
    if a.sup() < b.inf() {
        Some(Ordering::Less)
    } else if b.sup() < a.inf() {
        Some(Ordering::Greater)
    } else if a.inf() == a.sup() && b.inf() == b.sup() && a.inf() == b.inf() {
        Some(Ordering::Equal)
    } else {
        None
    }
}

/// Normalizes an axis direction; a decisively zero `|d|²` is a chart
/// degeneracy.
fn normalize_axis(d: Vector3) -> Result<Vector3, Refusal> {
    let x = ival(d.x);
    let y = ival(d.y);
    let z = ival(d.z);
    if decisively_zero(x * x + y * y + z * z) {
        return Err(Refusal::UnsupportedEnvelope(EnvelopeCase::ChartDegenerate));
    }
    Ok(d.normalize())
}

/// The cross product of two (normalized) directions, per component in interval
/// arithmetic so the outward rounding of the products catches the f64 noise.
fn cross_intervals(a0: Vector3, a1: Vector3) -> [Interval; 3] {
    [
        ival(a0.y) * ival(a1.z) - ival(a0.z) * ival(a1.y),
        ival(a0.z) * ival(a1.x) - ival(a0.x) * ival(a1.z),
        ival(a0.x) * ival(a1.y) - ival(a0.y) * ival(a1.x),
    ]
}

/// The affine placement carrying a unit ellipse with semi-axes `ru` along `u`
/// and `rv` along `v`, centred at `o`, with plane normal `n = u × v`.
fn frame(u: Vector3, v: Vector3, n: Vector3, o: Point3, ru: f64, rv: f64) -> Matrix4 {
    Matrix4::from_cols(
        Vector4::new(u.x, u.y, u.z, 0.0),
        Vector4::new(v.x, v.y, v.z, 0.0),
        Vector4::new(n.x, n.y, n.z, 0.0),
        Vector4::new(o.x, o.y, o.z, 1.0),
    ) * Matrix4::from_nonuniform_scale(ru, rv, 1.0)
}

/// A full ellipse with semi-axes `ru` along `u` and `rv` along `v`, centred at
/// `o` in the plane spanned by `(u, v)`.
fn placed_ellipse(u: Vector3, v: Vector3, o: Point3, ru: f64, rv: f64) -> PlacedCircle {
    let n = u.cross(v);
    Processor::with_transform(
        TrimmedCurve::new(UnitCircle::<Point3>::new(), (0.0, TAU)),
        frame(u, v, n, o, ru, rv),
    )
}

#[cfg(test)]
// Test-only allow: H-1 bans unwrap/expect on paths reachable from untrusted
// geometry. These unwraps are on hand-built dyadic witnesses and on the
// `equal_radius_cylinders` outcome; they cannot fire for the inputs below.
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use std::f64::consts::{FRAC_PI_2, FRAC_PI_4, PI, SQRT_2};

    use truck_base::cgmath64::EuclideanSpace;
    use truck_geotrait::ParametricCurve;

    /// The shared radius of both cylinders in every test.
    const UNIT_RADIUS: f64 = 1.0;
    /// H-3: dimensionless float slack for machine-precision witness
    /// comparisons on unit-scale geometry, not a length.
    const MACHINE_PRECISION_SLACK: f64 = 1.0e-12; // H-3: slack literal carried by this line, dimensionless, not a length
    /// Semi-major axis of the Steinmetz ellipses: `r · √2`.
    const STEINMETZ_SEMI_MAJOR: f64 = SQRT_2;
    /// Angle between the axes of the oblique test: 45°.
    const OBLIQUE_AXIS_ANGLE: f64 = FRAC_PI_4;
    /// Sample count per ellipse (≥ 30, per the packet).
    const SAMPLES: usize = 30;

    /// Distance from a point to a line given by a point and a unit direction.
    fn distance_to_line(p: Point3, o: Point3, unit_dir: Vector3) -> f64 {
        (p - o).cross(unit_dir).magnitude()
    }

    /// Extracts the two `Ellipse` arms of an outcome value; the placement must
    /// already be classified as `TwoCurves` of ellipses.
    fn two_ellipses(out: &AnalyticIntersection) -> (&PlacedCircle, &PlacedCircle) {
        let AnalyticIntersection::TwoCurves([ExactCurve::Ellipse(e0), ExactCurve::Ellipse(e1)]) =
            out
        else {
            unreachable!("expected two ellipses, got {out:?}")
        };
        (e0, e1)
    }

    /// Semi-major/semi-minor ratio of a placed ellipse.
    fn ellipse_ratio(e: &PlacedCircle) -> f64 {
        let centre = e.subs(0.0).midpoint(e.subs(PI));
        (e.subs(0.0) - centre).magnitude() / (e.subs(FRAC_PI_2) - centre).magnitude()
    }

    #[test]
    fn eqrcyl_steinmetz_perpendicular_two_ellipses() {
        let axis0 = (Point3::origin(), Vector3::unit_x());
        let axis1 = (Point3::origin(), Vector3::unit_y());
        let out = equal_radius_cylinders(UNIT_RADIUS, &axis0, &axis1).unwrap();
        let (e0, e1) = two_ellipses(&out.value);

        // The Steinmetz conditions: every sampled point lies on both cylinders
        // (y² + z² = 1 and x² + z² = 1), and the two ellipses lie in the
        // planes y = x (e0, internal bisector) and y = −x (e1, external).
        for k in 0..=SAMPLES {
            let t = TAU * k as f64 / SAMPLES as f64;
            for e in [e0, e1] {
                let p = e.subs(t);
                assert!(
                    (p.y * p.y + p.z * p.z - UNIT_RADIUS).abs() < MACHINE_PRECISION_SLACK,
                    "point {p:?} off cylinder y²+z²=1"
                );
                assert!(
                    (p.x * p.x + p.z * p.z - UNIT_RADIUS).abs() < MACHINE_PRECISION_SLACK,
                    "point {p:?} off cylinder x²+z²=1"
                );
            }
            let p0 = e0.subs(t);
            let p1 = e1.subs(t);
            assert!(
                (p0.y - p0.x).abs() < MACHINE_PRECISION_SLACK,
                "e0 point {p0:?} off the plane y = x"
            );
            assert!(
                (p1.y + p1.x).abs() < MACHINE_PRECISION_SLACK,
                "e1 point {p1:?} off the plane y = -x"
            );
        }

        // Each ellipse is centred at the origin and has semi-minor 1 and
        // semi-major √2 (the θ = π/2 degeneracy where the two bisectors agree).
        for e in [e0, e1] {
            let centre = e.subs(0.0).midpoint(e.subs(PI));
            assert!(
                centre.to_vec().magnitude() < MACHINE_PRECISION_SLACK,
                "centre {centre:?} not the origin"
            );
            let major = (e.subs(0.0) - centre).magnitude();
            let minor = (e.subs(FRAC_PI_2) - centre).magnitude();
            assert!(
                (major - STEINMETZ_SEMI_MAJOR).abs() < MACHINE_PRECISION_SLACK,
                "semi-major {major} != sqrt(2)"
            );
            assert!(
                (minor - UNIT_RADIUS).abs() < MACHINE_PRECISION_SLACK,
                "semi-minor {minor} != 1"
            );
        }
    }

    #[test]
    fn eqrcyl_oblique_angle_two_ellipses() {
        let dir1 = Vector3::new(1.0, 0.0, 1.0).normalize();
        let axis0 = (Point3::origin(), Vector3::unit_x());
        let axis1 = (Point3::origin(), dir1);
        let out = equal_radius_cylinders(UNIT_RADIUS, &axis0, &axis1).unwrap();
        let (e0, e1) = two_ellipses(&out.value);

        // Every sampled point of both ellipses is at distance r = 1 from both
        // axes: the on-both-carriers witness (the distance-to-line test).
        for k in 0..=SAMPLES {
            let t = TAU * k as f64 / SAMPLES as f64;
            for e in [e0, e1] {
                let p = e.subs(t);
                let d0 = distance_to_line(p, Point3::origin(), Vector3::unit_x());
                let d1 = distance_to_line(p, Point3::origin(), dir1);
                assert!(
                    (d0 - UNIT_RADIUS).abs() < MACHINE_PRECISION_SLACK,
                    "point {p:?} at distance {d0} from axis0"
                );
                assert!(
                    (d1 - UNIT_RADIUS).abs() < MACHINE_PRECISION_SLACK,
                    "point {p:?} at distance {d1} from axis1"
                );
            }
        }

        // Semi-major/minor ratio: e1 (external bisector plane, b̂−) is
        // 1/cos(θ/2) as the packet's decision 5 states; e0 (internal bisector
        // plane, b̂+) is 1/sin(θ/2) — the correction to decision 5, recorded
        // in RESULT.json (for θ = 45° the two differ: 2.613… vs 1.082…).
        let half_angle = OBLIQUE_AXIS_ANGLE / 2.0;
        let expected_internal = 1.0 / half_angle.sin();
        let expected_external = 1.0 / half_angle.cos();
        let ratio0 = ellipse_ratio(e0);
        let ratio1 = ellipse_ratio(e1);
        assert!(
            (ratio0 - expected_internal).abs() < MACHINE_PRECISION_SLACK,
            "e0 ratio {ratio0} != 1/sin(theta/2) = {expected_internal}"
        );
        assert!(
            (ratio1 - expected_external).abs() < MACHINE_PRECISION_SLACK,
            "e1 ratio {ratio1} != 1/cos(theta/2) = {expected_external}"
        );
    }

    #[test]
    fn eqrcyl_parallel_axes_refused() {
        let axis = (Point3::origin(), Vector3::unit_x());
        assert!(matches!(
            equal_radius_cylinders(UNIT_RADIUS, &axis, &axis),
            Err(Refusal::UnsupportedEnvelope(
                EnvelopeCase::NonCanonicalCarrier
            ))
        ));
        let opposite = (Point3::origin(), -Vector3::unit_x());
        assert!(matches!(
            equal_radius_cylinders(UNIT_RADIUS, &axis, &opposite),
            Err(Refusal::UnsupportedEnvelope(
                EnvelopeCase::NonCanonicalCarrier
            ))
        ));
    }

    #[test]
    fn eqrcyl_skew_axes_refused() {
        // axis0 = x̂ through the origin, axis1 = ŷ through (0, 0, 1): the
        // scalar triple product is decisively 1, so the axes are skew.
        let axis0 = (Point3::origin(), Vector3::unit_x());
        let axis1 = (Point3::new(0.0, 0.0, 1.0), Vector3::unit_y());
        assert!(matches!(
            equal_radius_cylinders(UNIT_RADIUS, &axis0, &axis1),
            Err(Refusal::UnsupportedEnvelope(
                EnvelopeCase::NonCanonicalCarrier
            ))
        ));
    }

    #[test]
    fn eqrcyl_certificate_is_exact() {
        let axis0 = (Point3::origin(), Vector3::unit_x());
        let axis1 = (Point3::origin(), Vector3::unit_y());
        let out = equal_radius_cylinders(UNIT_RADIUS, &axis0, &axis1).unwrap();
        assert_eq!(out.cert.method, Method::Exact);
        assert_eq!(out.cert.props.get(Prop::AnalyticCarrier), Truth::True);
    }
}
