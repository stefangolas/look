//! BG-ANA-001-PS: plane × sphere — circle, tangent point, or empty.
//!
//! A plane cuts a sphere in a circle, touches it in a single tangent point, or
//! misses it. All three outcomes — and the tangency boundary between them — are
//! decided by one exact comparison: the signed distance from the sphere centre
//! to the plane against the radius. In symbols,
//!
//! ```text
//! h  = (c − o) · n̂          signed distance, enclosed in inari
//! h² < r² → circle          radius ρ = √(r² − h²), centre c − h·n̂
//! h² = r² → tangent point   the foot c − h·n̂
//! h² > r² → empty
//! ```
//!
//! Classification is decided by **exact predicates on the f64 carrier
//! parameters**, never by sampling the surfaces (BG-ANA-002), and
//! `Method::Exact` is therefore honest: the arm is chosen only by decisive
//! outward-rounded interval enclosures of the carrier quantities — dyadic-clean
//! inputs give degenerate intervals, so exact classifications stay exact — and
//! the emitted curve is the closed-form intersection. The circle's coordinates
//! are computed in f64; the obligation is "lies on both carriers to machine
//! precision", asserted in the tests with an H-3-commented slack. There is no
//! `τ_rep` and no float-certified path here (H-6): an enclosure that straddles
//! the threshold refuses, it never guesses.
//!
//! A placed circle is `TrimmedCurve::new(UnitCircle, (0.0, TAU))` under the
//! affine `frame(...)` placement, the shared `PlacedCircle` channel. Note the
//! module-wide convention asserted in `analytic/mod.rs`: `TrimmedCurve` does
//! **not** remap its parameter — `subs(t)` takes the angle directly, it is not
//! a 0..1 parameter.

use std::cmp::Ordering;
use std::f64::consts::TAU;

use inari::Interval;
use truck_base::cgmath64::{InnerSpace, Matrix4, Point3, Vector3, Vector4};
use truck_base::evidence::{
    Budget, Certificate, Certified, Margin, Method, Modulus, Prop, PropMap, Refusal, Truth,
    UnresolvedWitness,
};
use truck_geometry::decorators::{Processor, TrimmedCurve};
use truck_geometry::specifieds::{Plane, Sphere, UnitCircle};

use crate::analytic::{AnalyticIntersection, AnalyticOutcome, ExactCurve, PlacedCircle};

/// A degenerate interval from a runtime `f64`. Finite coordinates always
/// construct; a NaN widens to the empty interval rather than panicking (H-1).
fn interval_at(x: f64) -> Interval {
    Interval::try_from((x, x)).unwrap_or(Interval::EMPTY)
}

/// Whether the interval is exactly `[0, 0]`. Only a degenerate interval proves
/// zero: an inari enclosure of a quantity that is zero only through
/// cancellation is a wide `[-ulp, +ulp]`, and claiming it proves zero is
/// exactly the wrong-but-confident answer BG-ANA-002 forbids. Dyadic-clean
/// inputs produce degenerate intervals, so exact classifications stay exact.
fn decisively_zero(i: Interval) -> bool {
    i.inf() == 0.0 && i.sup() == 0.0
}

/// Whether the interval lies strictly away from zero.
fn excludes_zero(i: Interval) -> bool {
    i.inf() > 0.0 || i.sup() < 0.0
}

/// A three-way comparison of two intervals, decided only when the ordering is
/// unambiguous: `Some(Less)` iff `a.sup() < b.inf()`, `Some(Greater)` iff
/// `b.sup() < a.inf()`, `Some(Equal)` iff both intervals are degenerate and
/// identical, and `None` — undecidable — otherwise.
///
/// `Some(Less)`/`Some(Greater)` are read off the outward-rounded difference
/// `a − b`, which is exactly the strict-interval condition; `Some(Equal)`
/// requires degeneracy (see [`decisively_zero`]). Undecidable is a stop, not a
/// guess: the caller refuses rather than returns an `Ok` arm chosen by a
/// predicate that did not decide.
fn three_way(a: Interval, b: Interval) -> Option<Ordering> {
    if excludes_zero(a - b) {
        if (a - b).inf() > 0.0 {
            Some(Ordering::Greater)
        } else {
            Some(Ordering::Less)
        }
    } else if decisively_zero(a - b) {
        Some(Ordering::Equal)
    } else {
        None
    }
}

/// Classifies the plane × sphere pair: a circle, a single tangent point, or
/// empty.
///
/// `Method::Exact` here means: the arm is decided by decisive interval
/// predicates on the f64 carrier parameters (the signed distance from the
/// sphere centre to the plane against the radius, both enclosed in `inari`
/// which rounds outward), and the emitted curve is the closed-form
/// intersection. The classification is exact; the emitted circle's coordinates
/// are computed in f64 and lie on both carriers to machine precision, asserted
/// with an H-3-commented slack — no `τ_rep` anywhere. An undecidable predicate
/// is a `Refusal::NumericallyUnresolved`, never a confident guess.
pub fn plane_sphere(plane: &Plane, sphere: &Sphere) -> AnalyticOutcome {
    let c = sphere.center();
    let o = plane.origin();
    let n = plane.normal();
    let h = (interval_at(c.x) - interval_at(o.x)) * interval_at(n.x)
        + (interval_at(c.y) - interval_at(o.y)) * interval_at(n.y)
        + (interval_at(c.z) - interval_at(o.z)) * interval_at(n.z);
    let h_sq = h * h;
    let r = sphere.radius();
    let r_sq = interval_at(r) * interval_at(r);
    let h_f = (c - o).dot(n);
    match three_way(h_sq, r_sq) {
        Some(Ordering::Less) => {
            let cc = c - h_f * n;
            let rho = (r * r - h_f * h_f).sqrt();
            let (u, v) = in_plane_axes(n);
            let circle: PlacedCircle = Processor::with_transform(
                TrimmedCurve::new(UnitCircle::<Point3>::new(), (0.0, TAU)),
                frame(u, v, n, cc, rho, rho),
            );
            let mut props = PropMap::new();
            props.set(Prop::AnalyticCarrier, Truth::True);
            Ok(Certified::new(
                AnalyticIntersection::Curve(ExactCurve::Circle(circle)),
                Certificate {
                    props,
                    method: Method::Exact,
                    budget_left: Budget::new(0, 0, 0),
                    margin: Margin::UNBOUNDED,
                    modulus: Modulus::Unbounded,
                },
            ))
        }
        Some(Ordering::Equal) => {
            let p = c - h_f * n;
            let mut props = PropMap::new();
            props.set(Prop::AnalyticCarrier, Truth::True);
            Ok(Certified::new(
                AnalyticIntersection::TangentPoint(p),
                Certificate {
                    props,
                    method: Method::Exact,
                    budget_left: Budget::new(0, 0, 0),
                    margin: Margin::UNBOUNDED,
                    modulus: Modulus::Unbounded,
                },
            ))
        }
        Some(Ordering::Greater) => {
            let mut props = PropMap::new();
            props.set(Prop::AnalyticCarrier, Truth::True);
            Ok(Certified::new(
                AnalyticIntersection::Empty,
                Certificate {
                    props,
                    method: Method::Exact,
                    budget_left: Budget::new(0, 0, 0),
                    margin: Margin::UNBOUNDED,
                    modulus: Modulus::Unbounded,
                },
            ))
        }
        None => Err(Refusal::NumericallyUnresolved {
            spent: Budget::new(0, 0, 0),
            witness: UnresolvedWitness::RootNotIsolated,
        }),
    }
}

/// The two in-plane unit axes of the plane perpendicular to `n̂`: `u` crosses
/// `n̂` with the least-aligned coordinate axis (picked by comparing
/// `|n̂.x|, |n̂.y|, |n̂.z|`), `v = n̂ × u`. The least-aligned axis makes
/// `n̂ × axis` at least `√(2/3)` long, so the normalize is never degenerate.
fn in_plane_axes(n: Vector3) -> (Vector3, Vector3) {
    let (nx, ny, nz) = (n.x.abs(), n.y.abs(), n.z.abs());
    let axis = if nx <= ny && nx <= nz {
        Vector3::unit_x()
    } else if ny <= nz {
        Vector3::unit_y()
    } else {
        Vector3::unit_z()
    };
    let u = n.cross(axis).normalize();
    let v = n.cross(u);
    (u, v)
}

/// The affine placement of a unit circle: columns `u`, `v`, `n` and origin
/// `o`, scaled in-plane by `ru`/`rv`. A circle of radius `r` through `o` with
/// in-plane unit axes `u`, `v` (`n = u × v`) is
/// `Processor::with_transform(TrimmedCurve::new(UnitCircle::<Point3>::new(),
/// (0.0, TAU)), frame(u, v, n, o, r, r))`.
fn frame(u: Vector3, v: Vector3, n: Vector3, o: Point3, ru: f64, rv: f64) -> Matrix4 {
    Matrix4::from_cols(
        Vector4::new(u.x, u.y, u.z, 0.0),
        Vector4::new(v.x, v.y, v.z, 0.0),
        Vector4::new(n.x, n.y, n.z, 0.0),
        Vector4::new(o.x, o.y, o.z, 1.0),
    ) * Matrix4::from_nonuniform_scale(ru, rv, 1.0)
}

#[cfg(test)]
// Test-only allow: H-1 bans unwrap/expect on paths reachable from untrusted
// geometry. Unit-test assertions on hand-built witnesses are not such a path.
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use inari::const_interval;
    use truck_base::cgmath64::Point3;
    use truck_geotrait::ParametricCurve;

    const N: usize = 32;

    fn z_plane() -> Plane {
        Plane::new(
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        )
    }

    fn plane_z_at(z: f64) -> Plane {
        Plane::new(
            Point3::new(0.0, 0.0, z),
            Point3::new(1.0, 0.0, z),
            Point3::new(0.0, 1.0, z),
        )
    }

    fn circle(out: Certified<AnalyticIntersection>) -> PlacedCircle {
        match out.value {
            AnalyticIntersection::Curve(ExactCurve::Circle(circle)) => circle,
            other => {
                // The dyadic witnesses below are provably circles; any other
                // arm is a classification regression.
                unreachable!("expected a circle, got {other:?}")
            }
        }
    }

    #[test]
    fn ps_circle_lies_on_both_carriers() {
        // Dyadic witness: plane z = 0, sphere centre (0, 0, 1), radius 5/4.
        // h = 1 < 5/4 → circle of radius 3/4 at the origin; every number
        // dyadic, so the interval predicates are exact.
        let plane = z_plane();
        let center = Point3::new(0.0, 0.0, 1.0);
        let sphere = Sphere::new(center, 1.25);
        let circle = circle(plane_sphere(&plane, &sphere).unwrap());
        let radius = 1.25;
        for i in 0..N {
            let t = std::f64::consts::TAU * i as f64 / (N as f64 - 1.0);
            let p = circle.subs(t);
            assert!(p.z.abs() < 1.0e-12, "point {p:?} leaves the z = 0 plane"); // H-3: float slack between p·ẑ and 0, a dimensionless residual of a unit-scale witness, not a length
            let radial = (p - center).magnitude();
            assert!(
                (radial - radius).abs() < 1.0e-12, // H-3: float slack between the sampled radius and the sphere's radius, a dimensionless residual of a unit-scale witness, not a length
                "point {p:?} leaves the sphere"
            );
        }
    }

    #[test]
    fn ps_great_circle_when_the_plane_passes_through_the_center() {
        // Plane z = 0 through the sphere centre → a great circle of radius r
        // in that plane; the emitted radius equals r to machine precision.
        let plane = z_plane();
        let center = Point3::new(0.0, 0.0, 0.0);
        let sphere = Sphere::new(center, 2.0);
        let circle = circle(plane_sphere(&plane, &sphere).unwrap());
        let radius = 2.0;
        for i in 0..N {
            let t = std::f64::consts::TAU * i as f64 / (N as f64 - 1.0);
            let p = circle.subs(t);
            assert!(p.z.abs() < 1.0e-12, "point {p:?} leaves the z = 0 plane"); // H-3: float slack between p·ẑ and 0, a dimensionless residual of a unit-scale witness, not a length
            let radial = (p - center).magnitude();
            assert!(
                (radial - radius).abs() < 1.0e-12, // H-3: float slack between the emitted radius and r, a dimensionless residual of a unit-scale witness, not a length
                "emitted radius {radial} differs from {radius}"
            );
        }
    }

    #[test]
    fn ps_tangent_point_and_empty_classify_exactly() {
        let plane = z_plane();
        let sphere = Sphere::new(Point3::new(0.0, 0.0, 1.0), 1.0);
        let out = plane_sphere(&plane, &sphere).unwrap();
        match out.value {
            AnalyticIntersection::TangentPoint(p) => assert_eq!(p, Point3::new(0.0, 0.0, 0.0)),
            other => unreachable!("expected a tangent point, got {other:?}"),
        }

        let raised = plane_z_at(2.0);
        let out = plane_sphere(&raised, &sphere).unwrap();
        match out.value {
            AnalyticIntersection::TangentPoint(p) => assert_eq!(p, Point3::new(0.0, 0.0, 2.0)),
            other => unreachable!("expected a tangent point, got {other:?}"),
        }

        let high = Sphere::new(Point3::new(0.0, 0.0, 2.0), 1.0);
        let out = plane_sphere(&plane, &high).unwrap();
        assert!(matches!(out.value, AnalyticIntersection::Empty));
    }

    #[test]
    fn ps_undecidable_predicates_refuse() {
        // Hand-built intervals pin the exact predicate semantics.
        let zero = const_interval!(0.0, 0.0);
        assert!(decisively_zero(zero));
        assert!(!excludes_zero(zero));
        let wide = const_interval!(-1.0e-12, 1.0e-12); // H-3: interval bound on a dimensionless signed-distance difference, not a length
        assert!(!decisively_zero(wide));
        assert!(!excludes_zero(wide));
        assert!(excludes_zero(const_interval!(1.0, 2.0)));
        assert!(excludes_zero(const_interval!(-2.0, -1.0)));

        // Overlapping non-degenerate intervals are undecidable, as are equal
        // non-degenerate intervals; strict separation decides.
        assert_eq!(
            three_way(const_interval!(1.0, 3.0), const_interval!(2.0, 4.0)),
            None
        );
        assert_eq!(
            three_way(const_interval!(1.0, 2.0), const_interval!(1.0, 2.0)),
            None
        );
        assert_eq!(
            three_way(const_interval!(1.0, 1.0), const_interval!(2.0, 2.0)),
            Some(Ordering::Less)
        );
        assert_eq!(
            three_way(const_interval!(3.0, 3.0), const_interval!(2.0, 2.0)),
            Some(Ordering::Greater)
        );
        assert_eq!(three_way(zero, zero), Some(Ordering::Equal));

        // A bit-neighbour radius next to the tangency value on the dyadic
        // witness stays decisive: degenerate intervals, no straddle.
        let plane = z_plane();
        let center = Point3::new(0.0, 0.0, 1.0);
        let r_up = f64::from_bits(1.0_f64.to_bits() + 1);
        assert!(plane_sphere(&plane, &Sphere::new(center, r_up)).is_ok());
        let r_down = f64::from_bits(1.0_f64.to_bits() - 1);
        assert!(plane_sphere(&plane, &Sphere::new(center, r_down)).is_ok());

        // A genuine straddle: the tilted plane's normal is non-dyadic, so the
        // signed-distance interval is non-degenerate, and a radius whose
        // squared interval falls strictly inside it must refuse rather than
        // guess either arm.
        let tilted = Plane::new(
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.1),
        );
        let center = Point3::new(1.0, 1.0, 1.0);
        let n = tilted.normal();
        let h = (interval_at(center.x) - interval_at(tilted.origin().x)) * interval_at(n.x)
            + (interval_at(center.y) - interval_at(tilted.origin().y)) * interval_at(n.y)
            + (interval_at(center.z) - interval_at(tilted.origin().z)) * interval_at(n.z);
        let h_sq = h * h;
        let mid = (h_sq.inf() + h_sq.sup()) / 2.0;
        let sphere = Sphere::new(center, mid.sqrt());
        assert!(matches!(
            plane_sphere(&tilted, &sphere),
            Err(Refusal::NumericallyUnresolved { .. })
        ));
    }

    #[test]
    fn ps_certificate_is_exact() {
        let plane = z_plane();
        let circle_out =
            plane_sphere(&plane, &Sphere::new(Point3::new(0.0, 0.0, 1.0), 1.25)).unwrap();
        assert_eq!(circle_out.cert.method, Method::Exact);
        assert_eq!(
            circle_out.cert.props.get(Prop::AnalyticCarrier),
            Truth::True
        );

        let tangent_out =
            plane_sphere(&plane, &Sphere::new(Point3::new(0.0, 0.0, 1.0), 1.0)).unwrap();
        assert_eq!(tangent_out.cert.method, Method::Exact);
        assert_eq!(
            tangent_out.cert.props.get(Prop::AnalyticCarrier),
            Truth::True
        );

        let empty_out =
            plane_sphere(&plane, &Sphere::new(Point3::new(0.0, 0.0, 2.0), 1.0)).unwrap();
        assert_eq!(empty_out.cert.method, Method::Exact);
        assert_eq!(empty_out.cert.props.get(Prop::AnalyticCarrier), Truth::True);
    }
}
