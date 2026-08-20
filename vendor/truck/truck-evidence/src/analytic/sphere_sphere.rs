//! BG-ANA-001-SS: sphere × sphere — circle, tangent point, empty, or
//! coincident.
//!
//! Everything here is decided by one exact comparison family: the squared
//! centre distance `d² = Σ (c1ᵢ − c0ᵢ)²` against `(r0 ± r1)²`, computed as
//! outward-rounded `inari::Interval` enclosures of the carrier parameters
//! (`inari` rounds outward, so a dyadic-clean witness gives degenerate
//! intervals and the classification is exact). An undecidable enclosure —
//! one that straddles a threshold — is `Refusal::NumericallyUnresolved`, never
//! a confident guess (BG-ANA-002).
//!
//! The shared result type is [`crate::analytic::AnalyticIntersection`]; this
//! module defines no result type of its own. The emitted circle is a
//! [`crate::analytic::PlacedCircle`]: the trimmed unit circle under an affine
//! placement. [`TrimmedCurve`] does **not** remap its parameter — `subs(t)`
//! takes the angle directly.

use std::cmp::Ordering;
use std::f64::consts::TAU;

use inari::Interval;
use truck_base::cgmath64::{InnerSpace, Matrix4, Point3, Vector3, Vector4};
use truck_base::evidence::{
    Budget, Certificate, Certified, Margin, Method, Modulus, Prop, PropMap, Refusal, Truth,
    UnresolvedWitness,
};
use truck_geometry::decorators::{Processor, TrimmedCurve};
use truck_geometry::specifieds::{Sphere, UnitCircle};

use crate::analytic::{AnalyticIntersection, AnalyticOutcome, ExactCurve};

/// The exact sphere × sphere classification (BG-ANA-001-SS).
///
/// `Method::Exact` here means: the classification is decided by **decisive
/// interval predicates** on the f64 carrier parameters — the squared centre
/// distance against `(r0 ± r1)²` — and the emitted curve is the closed-form
/// intersection. The point and curve coordinates are computed in f64; the
/// certificate's obligation is "lies on both carriers to machine precision",
/// asserted in the tests with an H-3-commented slack. There is no `τ_rep`
/// anywhere.
///
/// `Ok` is returned only when every predicate that chose the returned arm was
/// decisive; an undecidable predicate is `Refusal::NumericallyUnresolved`.
pub fn sphere_sphere(sphere0: &Sphere, sphere1: &Sphere) -> AnalyticOutcome {
    let c0 = sphere0.center();
    let c1 = sphere1.center();
    let r0 = sphere0.radius();
    let r1 = sphere1.radius();

    // d² = Σ (c1ᵢ − c0ᵢ)²: an interval sum of interval squares, each term the
    // outward-rounded enclosure of the coordinate difference.
    let d2 = {
        let dx = itv(c1.x) - itv(c0.x);
        let dy = itv(c1.y) - itv(c0.y);
        let dz = itv(c1.z) - itv(c0.z);
        dx * dx + dy * dy + dz * dz
    };
    // (r0 ± r1)² as intervals of the exact f64 sums.
    let rp = itv(r0 + r1);
    let rm = itv(r0 - r1);
    let rp2 = rp * rp;
    let rm2 = rm * rm;

    // Step 1: same centre? `decisively_zero(d2)` is exactly `three_way(d2,
    // [0,0])` taking the `Equal` arm: the enclosure must be the degenerate
    // [0, 0]. An enclosure that is zero only through cancellation is a
    // wide-ish [-ulp, +ulp] and is *not* decisive (BG-ANA-002) — it refuses
    // below, never asserts coincidence.
    if decisively_zero(d2) {
        // The radii ARE the carrier parameters: exact f64 equality decides.
        if r0 == r1 {
            let mut props = PropMap::new();
            props.set(Prop::AnalyticCarrier, Truth::True);
            return Ok(Certified::new(
                AnalyticIntersection::Coincident,
                Certificate {
                    props,
                    method: Method::Exact,
                    budget_left: Budget::new(0, 0, 0),
                    margin: Margin::UNBOUNDED,
                    modulus: Modulus::Unbounded,
                },
            ));
        }
        // Concentric, different radii: one strictly inside the other.
        let mut props = PropMap::new();
        props.set(Prop::AnalyticCarrier, Truth::True);
        return Ok(Certified::new(
            AnalyticIntersection::Empty,
            Certificate {
                props,
                method: Method::Exact,
                budget_left: Budget::new(0, 0, 0),
                margin: Margin::UNBOUNDED,
                modulus: Modulus::Unbounded,
            },
        ));
    }
    // Centres distinct. `excludes_zero(d2)` is three_way's `Greater` arm; if
    // the enclosure straddles zero the coincidence question is genuinely
    // undecidable and we refuse.
    if !excludes_zero(d2) {
        return Err(Refusal::NumericallyUnresolved {
            spent: Budget::new(0, 0, 0),
            witness: UnresolvedWitness::RootNotIsolated,
        });
    }

    let delta = c1 - c0;

    // Steps 2–3: d² against (r0 + r1)².
    match three_way(d2, rp2) {
        Some(Ordering::Greater) => {
            // d > r0 + r1: disjoint, apart.
            let mut props = PropMap::new();
            props.set(Prop::AnalyticCarrier, Truth::True);
            return Ok(Certified::new(
                AnalyticIntersection::Empty,
                Certificate {
                    props,
                    method: Method::Exact,
                    budget_left: Budget::new(0, 0, 0),
                    margin: Margin::UNBOUNDED,
                    modulus: Modulus::Unbounded,
                },
            ));
        }
        Some(Ordering::Equal) => {
            // d = r0 + r1: external tangency, at the f64 closed-form point
            // c0 + (r0 / (r0 + r1)) (c1 − c0).
            let p = c0 + delta * (r0 / (r0 + r1));
            let mut props = PropMap::new();
            props.set(Prop::AnalyticCarrier, Truth::True);
            return Ok(Certified::new(
                AnalyticIntersection::TangentPoint(p),
                Certificate {
                    props,
                    method: Method::Exact,
                    budget_left: Budget::new(0, 0, 0),
                    margin: Margin::UNBOUNDED,
                    modulus: Modulus::Unbounded,
                },
            ));
        }
        Some(Ordering::Less) => {}
        None => {
            return Err(Refusal::NumericallyUnresolved {
                spent: Budget::new(0, 0, 0),
                witness: UnresolvedWitness::RootNotIsolated,
            });
        }
    }

    // Steps 4–6: d < r0 + r1, so d² against (r0 − r1)².
    match three_way(d2, rm2) {
        Some(Ordering::Equal) => {
            // d = |r0 − r1|: internal tangency. The signed denominator r0 − r1
            // handles r0 < r1: the point then lies on the far side of c0 from
            // c1, and the formula is the f64 closed form
            // c0 + (r0 / (r0 − r1)) (c1 − c0).
            let p = c0 + delta * (r0 / (r0 - r1));
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
        Some(Ordering::Less) => {
            // d < |r0 − r1|: one sphere strictly inside the other.
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
        Some(Ordering::Greater) => {
            // |r0 − r1| < d < r0 + r1: the transverse circle, in f64 closed
            // form. d = |c1 − c0|, x = (d² + r0² − r1²) / (2 d) measured from
            // c0 toward c1; centre cc = c0 + x (c1 − c0)/d, radius
            // ρ = sqrt(r0² − x²), plane normal n̂ = (c1 − c0)/d.
            let d = delta.magnitude();
            let x = (d * d + r0 * r0 - r1 * r1) / (2.0 * d);
            let cc = c0 + delta * (x / d);
            let rho = (r0 * r0 - x * x).sqrt();
            let n_hat = delta / d;
            // In-plane axes: u unit and ⊥ n̂ via the least-aligned-axis cross
            // trick, v = n̂ × u.
            let u = least_aligned_axis(n_hat).cross(n_hat).normalize();
            let v = n_hat.cross(u);
            let circle = ExactCurve::Circle(Processor::with_transform(
                TrimmedCurve::new(UnitCircle::<Point3>::new(), (0.0, TAU)),
                frame(u, v, n_hat, cc, rho, rho),
            ));
            let mut props = PropMap::new();
            props.set(Prop::AnalyticCarrier, Truth::True);
            Ok(Certified::new(
                AnalyticIntersection::Curve(circle),
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

/// A degenerate interval carrying exactly the f64 `x`.
fn itv(x: f64) -> Interval {
    Interval::try_from((x, x)).unwrap_or(Interval::EMPTY)
}

/// Whether the enclosure is exactly the degenerate `[0, 0]`.
///
/// A quantity that is zero only through cancellation encloses zero with a
/// wide-ish interval and is deliberately *not* decisive here.
fn decisively_zero(i: Interval) -> bool {
    i.inf() == 0.0 && i.sup() == 0.0
}

/// Whether the enclosure lies entirely away from zero.
fn excludes_zero(i: Interval) -> bool {
    i.inf() > 0.0 || i.sup() < 0.0
}

/// The three-way interval comparison: `Less`/`Greater` when the enclosures are
/// disjoint, `Equal` when both are degenerate and identical, `None` when the
/// two enclosures overlap but are not provably equal.
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

/// The coordinate axis least aligned with `n` (smallest |component|), so
/// `axis.cross(n)` is a unit vector ⊥ `n` for any unit `n`.
fn least_aligned_axis(n: Vector3) -> Vector3 {
    if n.x.abs() <= n.y.abs() && n.x.abs() <= n.z.abs() {
        Vector3::unit_x()
    } else if n.y.abs() <= n.z.abs() {
        Vector3::unit_y()
    } else {
        Vector3::unit_z()
    }
}

/// The affine placement of a circle of in-plane unit axes `u`, `v`
/// (`n = u × v`) through `o`, scaled to radius `ru = rv`.
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
// geometry. These unwraps are on hand-built dyadic witnesses and on outcomes
// just asserted to be `Ok`; they cannot fire for the values constructed below.
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use truck_geotrait::ParametricCurve;

    #[test]
    fn ss_circle_lies_on_both_spheres() {
        // Dyadic witness: r0 = r1 = 5/2, centres (0,0,0) and (3,0,0). The
        // intersection circle sits at x = 3/2 with radius 2, all dyadic.
        let s0 = Sphere::new(Point3::new(0.0, 0.0, 0.0), 2.5);
        let s1 = Sphere::new(Point3::new(3.0, 0.0, 0.0), 2.5);
        let out = sphere_sphere(&s0, &s1).unwrap();
        let AnalyticIntersection::Curve(ExactCurve::Circle(circle)) = &out.value else {
            unreachable!("expected a circle, got {:?}", out.value);
        };
        // TrimmedCurve does not remap: subs(t) takes the angle directly.
        // Sample >= 30 angles and assert every point lies on both spheres to
        // machine precision.
        const N: usize = 30;
        for i in 0..N {
            let t = TAU * (i as f64) / (N as f64);
            let p = circle.subs(t);
            let residual0 = (p - s0.center()).magnitude() - s0.radius();
            let residual1 = (p - s1.center()).magnitude() - s1.radius();
            assert!(
                residual0.abs() < 1.0e-12, // H-3: dimensionless unit-scale residual, not a length
                "p={p:?} off s0"
            );
            assert!(
                residual1.abs() < 1.0e-12, // H-3: dimensionless unit-scale residual, not a length
                "p={p:?} off s1"
            );
        }
    }

    #[test]
    fn ss_tangent_points_classify_exactly() {
        // External: r0 = 2, r1 = 1, centres (0,0,0) and (3,0,0). d = 3 = r0+r1.
        // The point is the f64 closed form c0 + (r0/(r0+r1)) (c1 − c0); 2/3
        // rounds, so compare within an H-3 slack.
        let out = sphere_sphere(
            &Sphere::new(Point3::new(0.0, 0.0, 0.0), 2.0),
            &Sphere::new(Point3::new(3.0, 0.0, 0.0), 1.0),
        )
        .unwrap();
        let AnalyticIntersection::TangentPoint(p) = out.value else {
            unreachable!("expected TangentPoint, got {:?}", out.value);
        };
        assert!(
            (p - Point3::new(2.0, 0.0, 0.0)).magnitude() < 1.0e-12, // H-3: dimensionless unit-scale residual, not a length
            "external tangent {p:?}"
        );

        // Internal: same radii, centres (0,0,0) and (1,0,0). d = 1 = r0 − r1.
        // The point is on the far side of c0 from c1, at (2,0,0).
        let out = sphere_sphere(
            &Sphere::new(Point3::new(0.0, 0.0, 0.0), 2.0),
            &Sphere::new(Point3::new(1.0, 0.0, 0.0), 1.0),
        )
        .unwrap();
        let AnalyticIntersection::TangentPoint(p) = out.value else {
            unreachable!("expected TangentPoint, got {:?}", out.value);
        };
        assert!(
            (p - Point3::new(2.0, 0.0, 0.0)).magnitude() < 1.0e-12, // H-3: dimensionless unit-scale residual, not a length
            "internal tangent {p:?}"
        );

        // Internal with r0 < r1 (the sign case): centres (0,0,0) and (1,0,0),
        // r0 = 1, r1 = 2. The signed denominator r0 − r1 flips the direction,
        // placing the point on the far side of c0 from c1 at (−1,0,0).
        let out = sphere_sphere(
            &Sphere::new(Point3::new(0.0, 0.0, 0.0), 1.0),
            &Sphere::new(Point3::new(1.0, 0.0, 0.0), 2.0),
        )
        .unwrap();
        let AnalyticIntersection::TangentPoint(p) = out.value else {
            unreachable!("expected TangentPoint, got {:?}", out.value);
        };
        assert!(
            (p - Point3::new(-1.0, 0.0, 0.0)).magnitude() < 1.0e-12, // H-3: dimensionless unit-scale residual, not a length
            "inverted tangent {p:?}"
        );
    }

    #[test]
    fn ss_disjoint_contained_and_coincident_classify_exactly() {
        // d > r0 + r1: disjoint, apart.
        let out = sphere_sphere(
            &Sphere::new(Point3::new(0.0, 0.0, 0.0), 1.0),
            &Sphere::new(Point3::new(3.0, 0.0, 0.0), 1.0),
        )
        .unwrap();
        assert!(matches!(out.value, AnalyticIntersection::Empty));

        // d < |r0 − r1|: one sphere strictly inside the other.
        let out = sphere_sphere(
            &Sphere::new(Point3::new(0.0, 0.0, 0.0), 3.0),
            &Sphere::new(Point3::new(1.0, 0.0, 0.0), 1.0),
        )
        .unwrap();
        assert!(matches!(out.value, AnalyticIntersection::Empty));

        // Identical spheres: coincident.
        let out = sphere_sphere(
            &Sphere::new(Point3::new(0.0, 0.0, 0.0), 2.0),
            &Sphere::new(Point3::new(0.0, 0.0, 0.0), 2.0),
        )
        .unwrap();
        assert!(matches!(out.value, AnalyticIntersection::Coincident));

        // Concentric, different radii: empty (one strictly inside the other).
        let out = sphere_sphere(
            &Sphere::new(Point3::new(0.0, 0.0, 0.0), 2.0),
            &Sphere::new(Point3::new(0.0, 0.0, 0.0), 1.0),
        )
        .unwrap();
        assert!(matches!(out.value, AnalyticIntersection::Empty));
    }

    #[test]
    fn ss_undecidable_predicates_refuse() {
        // A [-w, w] enclosure is neither decisively-zero nor excludes-zero: it
        // is the straddle the classification must refuse, never report as a
        // guess.
        let straddle = Interval::try_from((-1.0e-9, 1.0e-9)).expect("valid interval"); // H-3: interval half-width, dimensionless
        assert!(!decisively_zero(straddle));
        assert!(!excludes_zero(straddle));

        // Overlapping non-degenerate intervals give three_way == None.
        let a = Interval::try_from((0.0, 2.0)).expect("valid interval");
        let b = Interval::try_from((1.0, 3.0)).expect("valid interval");
        assert_eq!(three_way(a, b), None);
        assert_eq!(three_way(b, a), None);

        // Decided orderings still fire: disjoint, reversed, degenerate-equal.
        let lo = Interval::try_from((0.0, 1.0)).expect("valid interval");
        let hi = Interval::try_from((2.0, 3.0)).expect("valid interval");
        let deg = Interval::try_from((0.0, 0.0)).expect("valid interval");
        assert_eq!(three_way(lo, hi), Some(Ordering::Less));
        assert_eq!(three_way(hi, lo), Some(Ordering::Greater));
        assert_eq!(three_way(deg, deg), Some(Ordering::Equal));

        // A genuine straddle refusal: r0 = r1 = 0.05 at centre distance 0.1 is
        // an exact tangency in real arithmetic, but d² and (r0+r1)² are both
        // the same non-degenerate enclosure of 0.01 (not dyadic), so the
        // tangency predicate is undecidable and the call must refuse.
        let tangent = sphere_sphere(
            &Sphere::new(Point3::new(0.0, 0.0, 0.0), 0.05),
            &Sphere::new(Point3::new(0.1, 0.0, 0.0), 0.05),
        );
        assert!(matches!(
            tangent,
            Err(Refusal::NumericallyUnresolved { .. })
        ));

        // One bit-neighbour tangency witness: nudging r1 one ulp above 2 makes
        // the exact f64 sum r0 + r1' = 3 + 2^-51 exceed the centre distance 3,
        // so the pair genuinely meets in a small circle and stays decided
        // rather than refusing.
        let r1 = 2.0f64 + 2.0 * f64::EPSILON;
        let near = sphere_sphere(
            &Sphere::new(Point3::new(0.0, 0.0, 0.0), 1.0),
            &Sphere::new(Point3::new(3.0, 0.0, 0.0), r1),
        );
        assert!(matches!(
            near,
            Ok(c) if matches!(c.value, AnalyticIntersection::Curve(_))
        ));
    }

    #[test]
    fn ss_certificate_is_exact() {
        // A circle outcome.
        let out = sphere_sphere(
            &Sphere::new(Point3::new(0.0, 0.0, 0.0), 2.5),
            &Sphere::new(Point3::new(3.0, 0.0, 0.0), 2.5),
        )
        .unwrap();
        assert_eq!(out.cert.method, Method::Exact);
        assert_eq!(out.cert.props.get(Prop::AnalyticCarrier), Truth::True);

        // A tangent-point outcome.
        let out = sphere_sphere(
            &Sphere::new(Point3::new(0.0, 0.0, 0.0), 2.0),
            &Sphere::new(Point3::new(3.0, 0.0, 0.0), 1.0),
        )
        .unwrap();
        assert_eq!(out.cert.method, Method::Exact);
        assert_eq!(out.cert.props.get(Prop::AnalyticCarrier), Truth::True);

        // An empty outcome.
        let out = sphere_sphere(
            &Sphere::new(Point3::new(0.0, 0.0, 0.0), 1.0),
            &Sphere::new(Point3::new(3.0, 0.0, 0.0), 1.0),
        )
        .unwrap();
        assert_eq!(out.cert.method, Method::Exact);
        assert_eq!(out.cert.props.get(Prop::AnalyticCarrier), Truth::True);
    }
}
