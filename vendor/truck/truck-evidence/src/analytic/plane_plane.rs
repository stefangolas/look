//! BG-ANA-001-PP: plane × plane — a line, or a typed parallel/coincident
//! classification, decided by exact predicates on the carrier parameters.
//!
//! Two planes meet in a line, or they are parallel (no intersection), or they
//! coincide (the intersection is a surface, not a curve). The classification
//! is decided by decisive interval predicates on the f64 carrier parameters —
//! never by sampling the surfaces (BG-ANA-002) — and the transverse line is
//! the closed-form solution of the two plane equations.
//!
//! The shared result type [`crate::analytic::AnalyticIntersection`] (with
//! [`crate::analytic::ExactCurve`]) is defined by the family's module root and
//! is not redefined here.

use super::{AnalyticIntersection, AnalyticOutcome, ExactCurve};
use inari::Interval;
use truck_base::cgmath64::{EuclideanSpace, InnerSpace, Point3, Vector3};
use truck_base::evidence::{
    Budget, Certificate, Certified, Margin, Method, Modulus, Prop, PropMap, Refusal, Truth,
    UnresolvedWitness,
};
use truck_geometry::specifieds::{Line, Plane};

/// Classifies the pair `plane0 × plane1` exactly.
///
/// The classification is **exact**: every predicate is decided by decisive
/// interval enclosures of quantities derived from the f64 carrier parameters,
/// and an arm is returned only when its predicate is decisive. An enclosure
/// that straddles the threshold is `Err(Refusal::NumericallyUnresolved)`,
/// never a confident guess (BG-ANA-002).
///
/// What `Method::Exact` means here, precisely: the *classification* is exact —
/// decided by decisive interval predicates on the f64 carrier parameters —
/// and the emitted curve is the closed-form solution of the two plane
/// equations. The curve coordinates themselves are computed in f64; the
/// obligation the certificate takes on is that the emitted line lies on both
/// carriers to machine precision (the on-both-carriers test asserts this with
/// an H-3 slack), not that the coordinates are dyadic-exact. No `τ_rep` is
/// attached anywhere.
pub fn plane_plane(plane0: &Plane, plane1: &Plane) -> AnalyticOutcome {
    let n0 = plane0.normal();
    let n1 = plane1.normal();
    let o0 = plane0.origin();
    let o1 = plane1.origin();

    let cross = interval_cross(n0, n1);
    if cross.iter().any(|c| excludes_zero(*c)) {
        // Transverse: the normals are decisively non-collinear, so the
        // intersection is one line.
        let d = n0.cross(n1).normalize();
        let p = point_on_both(n0, o0, n1, o1);
        let mut props = PropMap::new();
        props.set(Prop::AnalyticCarrier, Truth::True);
        Ok(Certified::new(
            AnalyticIntersection::Curve(ExactCurve::Line(Line(p, p + d))),
            Certificate {
                props,
                method: Method::Exact,
                budget_left: Budget::new(0, 0, 0),
                margin: Margin::UNBOUNDED,
                modulus: Modulus::Unbounded,
            },
        ))
    } else if cross.iter().all(|c| decisively_zero(*c)) {
        // Parallel: the normals are decisively collinear; the offset between
        // the carriers decides coincidence.
        let h = interval_dot(o1.to_vec() - o0.to_vec(), n0);
        let value = if decisively_zero(h) {
            AnalyticIntersection::Coincident
        } else if excludes_zero(h) {
            AnalyticIntersection::Parallel
        } else {
            return Err(Refusal::NumericallyUnresolved {
                spent: Budget::new(0, 0, 0),
                witness: UnresolvedWitness::RootNotIsolated,
            });
        };
        let mut props = PropMap::new();
        props.set(Prop::AnalyticCarrier, Truth::True);
        Ok(Certified::new(
            value,
            Certificate {
                props,
                method: Method::Exact,
                budget_left: Budget::new(0, 0, 0),
                margin: Margin::UNBOUNDED,
                modulus: Modulus::Unbounded,
            },
        ))
    } else {
        // The normal cross product straddles the zero threshold: the pair is
        // too close to parallel to classify. A stop, not a guess.
        Err(Refusal::NumericallyUnresolved {
            spent: Budget::new(0, 0, 0),
            witness: UnresolvedWitness::RootNotIsolated,
        })
    }
}

/// Whether the interval proves a zero: it must be the degenerate `[0, 0]`.
///
/// An inari enclosure of a dot product that is exactly zero only through
/// cancellation is a wide-ish `[-ulp, +ulp]`; claiming that proves zero is
/// exactly the wrong-but-confident answer BG-ANA-002 forbids. Dyadic-clean
/// inputs produce degenerate intervals, so exact classifications stay exact.
fn decisively_zero(i: Interval) -> bool {
    i.inf() == 0.0 && i.sup() == 0.0
}

/// Whether the interval proves a nonzero value: it lies strictly away from 0.
fn excludes_zero(i: Interval) -> bool {
    i.inf() > 0.0 || i.sup() < 0.0
}

/// Three-way comparison of two interval enclosures.
///
/// `Some(Equal)` only when both intervals are degenerate and identical;
/// `None` when they overlap or one straddles the other — undecidable.
#[cfg(test)]
fn three_way(a: Interval, b: Interval) -> Option<std::cmp::Ordering> {
    if a.sup() < b.inf() {
        Some(std::cmp::Ordering::Less)
    } else if b.sup() < a.inf() {
        Some(std::cmp::Ordering::Greater)
    } else if a.inf() == a.sup() && b.inf() == b.sup() && a.inf() == b.inf() {
        Some(std::cmp::Ordering::Equal)
    } else {
        None
    }
}

/// The closed-form point on both planes, from the unit normals and the plane
/// constants `cᵢ = oᵢ·nᵢ`.
///
/// The packet's formula paired `c0` with the `n1`-bracket and `c1` with the
/// `n0`-bracket, which swaps the plane constants: the returned point then
/// satisfies `x·n0 = c1` and `x·n1 = c0`. The correct pairing — solved from
/// `x·n0 = c0`, `x·n1 = c1` with `d = n0·n1` —
///
/// ```text
/// p = ((c0 − c1·d)·n0 + (c1 − c0·d)·n1) / (1 − d²)
/// ```
///
/// is what is implemented here and verified numerically by the transverse
/// test. See `deviations` in `RESULT.json` (BG-ANA-001-PP).
fn point_on_both(n0: Vector3, o0: Point3, n1: Vector3, o1: Point3) -> Point3 {
    let d = n0.dot(n1);
    let c0 = o0.to_vec().dot(n0);
    let c1 = o1.to_vec().dot(n1);
    let p = ((c0 - c1 * d) * n0 + (c1 - c0 * d) * n1) / (1.0 - d * d);
    Point3::from_vec(p)
}

/// The normal cross product `a × b`, computed per component in interval
/// arithmetic so each component is an outward-rounded enclosure.
fn interval_cross(a: Vector3, b: Vector3) -> [Interval; 3] {
    [
        interval_at(a.y) * interval_at(b.z) - interval_at(a.z) * interval_at(b.y),
        interval_at(a.z) * interval_at(b.x) - interval_at(a.x) * interval_at(b.z),
        interval_at(a.x) * interval_at(b.y) - interval_at(a.y) * interval_at(b.x),
    ]
}

/// The dot product `a · b` in interval arithmetic.
fn interval_dot(a: Vector3, b: Vector3) -> Interval {
    interval_at(a.x) * interval_at(b.x)
        + interval_at(a.y) * interval_at(b.y)
        + interval_at(a.z) * interval_at(b.z)
}

/// A degenerate interval from a runtime `f64`. Finite coordinates always
/// construct; a NaN widens to the empty interval rather than panicking (H-1).
fn interval_at(x: f64) -> Interval {
    Interval::try_from((x, x)).unwrap_or(Interval::EMPTY)
}

#[cfg(test)]
// Test-only allow: H-1 bans unwrap/expect on paths reachable from untrusted
// geometry. Unit-test assertions on hand-built witnesses are not such a path;
// these unwraps cannot fire for the values constructed below.
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    /// Plane residuals of unit-scale witnesses are dimensionless, so this is a
    /// dimensionless slack, not a length.
    const SLACK: f64 = 1.0e-9; // H-3: dimensionless plane-residual slack of a unit-scale witness

    const SAMPLES: usize = 64;

    /// The line carried by a `Curve(Line(..))` arm, if that is the arm.
    fn as_line(value: &AnalyticIntersection) -> Option<Line<Point3>> {
        match value {
            AnalyticIntersection::Curve(ExactCurve::Line(line)) => Some(*line),
            _ => None,
        }
    }

    #[test]
    fn pp_transverse_line_lies_on_both_planes() {
        // z = 0 through the origin and y = 0 through the origin → the x-axis.
        let z0 = Plane::new(
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        );
        let y0 = Plane::new(
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 0.0, 1.0),
        );
        // The packet's generic pair through the origin: spanned by
        // (0,0,0),(1,0,1),(0,1,1) and (0,0,0),(1,1,0),(1,0,0).
        let g0 = Plane::new(
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 1.0),
            Point3::new(0.0, 1.0, 1.0),
        );
        let g1 = Plane::new(
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 1.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
        );
        // A generic pair with nonzero offsets (z = 3, y = 5), so the closed
        // form's point term is exercised rather than vanishing at the origin.
        let z3 = Plane::new(
            Point3::new(0.0, 0.0, 3.0),
            Point3::new(1.0, 0.0, 3.0),
            Point3::new(0.0, 1.0, 3.0),
        );
        let y5 = Plane::new(
            Point3::new(0.0, 5.0, 0.0),
            Point3::new(1.0, 5.0, 0.0),
            Point3::new(0.0, 5.0, 1.0),
        );
        for (a, b) in [(z0, y0), (g0, g1), (z3, y5)] {
            let out = plane_plane(&a, &b).expect("dyadic transverse witness is decidable");
            assert_eq!(out.cert.method, Method::Exact);
            let line = as_line(&out.value).expect("transverse pair emits a Curve(Line(..))");
            let p = line.0;
            let d = line.1 - p;
            let n0 = a.normal();
            let n1 = b.normal();
            let o0 = a.origin();
            let o1 = b.origin();
            // The direction is perpendicular to both normals.
            assert!(
                d.dot(n0).abs() < SLACK,
                "direction {d:?} not perpendicular to n0 = {n0:?}"
            ); // H-3: float slack between a unit direction vector and a unit normal (direction cosines), not a length
            assert!(
                d.dot(n1).abs() < SLACK,
                "direction {d:?} not perpendicular to n1 = {n1:?}"
            ); // H-3: float slack between a unit direction vector and a unit normal (direction cosines), not a length
               // `normalize()` returns a unit vector to machine precision.
            assert!(
                (d.magnitude() - 1.0).abs() < SLACK,
                "direction {d:?} is not unit"
            ); // H-3: float slack between the norm of a unit vector and 1, not a length
            for i in 0..SAMPLES {
                let t = (i as f64) / (SAMPLES as f64 - 1.0);
                let x = p + t * d;
                assert!(
                    (x - o0).dot(n0).abs() < SLACK,
                    "sampled point {x:?} off plane 0 (residual {:?})",
                    (x - o0).dot(n0)
                ); // H-3: dimensionless plane residual of a unit-scale witness
                assert!(
                    (x - o1).dot(n1).abs() < SLACK,
                    "sampled point {x:?} off plane 1 (residual {:?})",
                    (x - o1).dot(n1)
                ); // H-3: dimensionless plane residual of a unit-scale witness
            }
        }
    }

    #[test]
    fn pp_parallel_and_coincident_classify_exactly() {
        let z0 = Plane::new(
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        );
        let z2 = Plane::new(
            Point3::new(0.0, 0.0, 2.0),
            Point3::new(1.0, 0.0, 2.0),
            Point3::new(0.0, 1.0, 2.0),
        );
        // z = 0 vs z = 2: parallel, never touching.
        let out = plane_plane(&z0, &z2).expect("dyadic parallel witness is decidable");
        assert!(matches!(out.value, AnalyticIntersection::Parallel));
        // A plane vs itself (identical three points): coincident.
        let out = plane_plane(&z0, &z0).expect("dyadic coincident witness is decidable");
        assert!(matches!(out.value, AnalyticIntersection::Coincident));
    }

    #[test]
    fn pp_coincident_through_different_point_triples() {
        // The same z = 0 plane, built from two different point triples: the
        // second triple's origin and axes are shifted along in-plane
        // directions, so only the offset between the carriers is zero.
        let a = Plane::new(
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        );
        let b = Plane::new(
            Point3::new(2.0, 3.0, 0.0),
            Point3::new(2.0, 4.0, 0.0),
            Point3::new(3.0, 3.0, 0.0),
        );
        for (p0, p1) in [(&a, &b), (&b, &a)] {
            let out = plane_plane(p0, p1).expect("dyadic coincident witness is decidable");
            assert!(
                matches!(out.value, AnalyticIntersection::Coincident),
                "the offset predicate keys off the carriers, not the representation points"
            );
        }
    }

    #[test]
    fn pp_undecidable_predicates_refuse() {
        // A bit-level straddle witness is not constructible for plane normals:
        // the normal is `normalize(u × v)`, and the dyadic witnesses used here
        // give dyadic cross products that never straddle zero. The refusal
        // path is therefore covered directly on the comparator instead.
        //
        // A non-degenerate interval around zero is neither decisively zero nor
        // does it exclude zero.
        let straddle = Interval::try_from((-1.0e-9, 1.0e-9)).expect("valid interval"); // H-3: interval half-width, dimensionless
        assert!(!decisively_zero(straddle));
        assert!(!excludes_zero(straddle));
        // Overlapping non-degenerate intervals give `three_way == None`.
        let a = Interval::try_from((0.0, 2.0)).expect("valid interval");
        let b = Interval::try_from((1.0, 3.0)).expect("valid interval");
        assert_eq!(three_way(a, b), None);
        assert_eq!(three_way(b, a), None);
        // Decisive cases for contrast: disjoint, then degenerate-identical.
        let lo = Interval::try_from((0.0, 1.0)).expect("valid interval");
        let hi = Interval::try_from((2.0, 3.0)).expect("valid interval");
        assert_eq!(three_way(lo, hi), Some(std::cmp::Ordering::Less));
        assert_eq!(three_way(hi, lo), Some(std::cmp::Ordering::Greater));
        let deg = Interval::try_from((0.0, 0.0)).expect("valid interval");
        assert_eq!(three_way(deg, deg), Some(std::cmp::Ordering::Equal));
    }

    #[test]
    fn pp_certificate_is_exact() {
        let z0 = Plane::new(
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        );
        let z2 = Plane::new(
            Point3::new(0.0, 0.0, 2.0),
            Point3::new(1.0, 0.0, 2.0),
            Point3::new(0.0, 1.0, 2.0),
        );
        let y0 = Plane::new(
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 0.0, 1.0),
        );
        // A transverse, a parallel, and a coincident pair.
        for (a, b) in [(&z0, &y0), (&z0, &z2), (&z0, &z0)] {
            let out = plane_plane(a, b).expect("dyadic witness is decidable");
            assert_eq!(out.cert.method, Method::Exact);
            // The AnalyticCarrier prop is set true and no other prop is set.
            assert_eq!(out.cert.props.get(Prop::AnalyticCarrier), Truth::True);
            assert_eq!(out.cert.props.get(Prop::SoundEnclosure), Truth::Unknown);
            assert_eq!(out.cert.props.get(Prop::Provisional), Truth::Unknown);
            assert_eq!(out.cert.props.get(Prop::AnalyticPreserved), Truth::Unknown);
        }
    }
}
