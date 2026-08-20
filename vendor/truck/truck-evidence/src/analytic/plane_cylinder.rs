//! BG-ANA-001-PCYL: plane × cylinder — two lines, one tangent line, circle,
//! or ellipse, by axis-normal angle and distance.
//!
//! The canonical `Cylinder` of the specifieds runs along the **z axis** through
//! its `center` (BG-CE-006's canonical form). A plane cuts it in **two lines**
//! (plane parallel to the axis, offset inside), **one tangent line** (parallel,
//! offset exactly `r`), **a circle** (plane perpendicular to the axis), **an
//! ellipse** (any other tilt), or **nothing** (parallel, outside). Which one is
//! decided by the axis-normal angle and the offset — both exact predicates on
//! the carrier parameters (BG-ANA-002), never by sampling the surfaces.
//!
//! The tilt predicate is the component `a = n̂.z` of the plane's unit normal,
//! tested directly as an exact f64 value. `|a| = 1` is perpendicular — the
//! (infinite) axis pierces the plane exactly once, so it **always** cuts a
//! circle (there is no empty perpendicular case); `a = 0` is parallel, where
//! the squared axis-to-plane offset `δ²` is compared against `r²` by a
//! three-way comparison of decisive outward-rounded inari enclosures; and
//! `0 < |a| < 1` is an ellipse.
//!
//! A placed circle/ellipse is `TrimmedCurve::new(UnitCircle, (0.0, TAU))` under
//! the affine `frame(...)` placement, the shared `PlacedCircle` channel. Note
//! the module-wide convention asserted in `analytic/mod.rs`: `TrimmedCurve`
//! does **not** remap its parameter — `subs(t)` takes the angle directly.

use std::cmp::Ordering;
use std::f64::consts::TAU;

use inari::Interval;
use truck_base::cgmath64::{InnerSpace, Matrix4, Point3, Vector3, Vector4};
use truck_base::evidence::{
    Budget, Certificate, Certified, Margin, Method, Modulus, Prop, PropMap, Refusal, Truth,
    UnresolvedWitness,
};
use truck_geometry::decorators::{Processor, TrimmedCurve};
use truck_geometry::specifieds::{Cylinder, Line, Plane, UnitCircle};

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
/// Undecidable is a stop, not a guess: the caller refuses rather than returns
/// an `Ok` arm chosen by a predicate that did not decide.
fn three_way(a: Interval, b: Interval) -> Option<Ordering> {
    if excludes_zero(a - b) {
        // `a − b` strictly away from zero decides the ordering; the sign of
        // `(a − b).inf()` is `Less` vs `Greater` (a.sup() < b.inf() resp.
        // b.sup() < a.inf()).
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

/// The affine placement of a unit conic: columns `u`, `v`, `n` and origin `o`,
/// scaled in-plane by `ru`/`rv`. A circle of radius `r` through `o` with
/// in-plane unit axes `u`, `v` (`n = u × v`) is
/// `Processor::with_transform(TrimmedCurve::new(UnitCircle::<Point3>::new(),
/// (0.0, TAU)), frame(u, v, n, o, r, r))`; an ellipse uses `ru ≠ rv`.
fn frame(u: Vector3, v: Vector3, n: Vector3, o: Point3, ru: f64, rv: f64) -> Matrix4 {
    Matrix4::from_cols(
        Vector4::new(u.x, u.y, u.z, 0.0),
        Vector4::new(v.x, v.y, v.z, 0.0),
        Vector4::new(n.x, n.y, n.z, 0.0),
        Vector4::new(o.x, o.y, o.z, 1.0),
    ) * Matrix4::from_nonuniform_scale(ru, rv, 1.0)
}

/// Classifies the plane × cylinder pair: two lines, one tangent line, a
/// circle, an ellipse, or empty.
///
/// `Method::Exact` here means: the arm is decided by exact predicates on the
/// f64 carrier parameters — the tilt component `a = n̂·ẑ` tested directly, and
/// the squared axis-to-plane offset against the squared radius by decisive
/// outward-rounded inari enclosures (dyadic-clean inputs give degenerate
/// intervals, so exact classifications stay exact) — and the emitted curve is
/// the closed-form intersection. An enclosure that merely contains zero proves
/// nothing: an undecidable predicate is a `Refusal::NumericallyUnresolved`,
/// never a confident guess. The curve's coordinates are computed in f64; the
/// obligation is "lies on both carriers to machine precision", asserted with an
/// H-3-commented slack. No `τ_rep` and no float-certified path here (H-6).
pub fn plane_cylinder(plane: &Plane, cylinder: &Cylinder) -> AnalyticOutcome {
    let n = plane.normal();
    let o = plane.origin();
    let c = cylinder.center();
    let r = cylinder.radius();
    let a = n.z;
    let z_hat = Vector3::unit_z();

    if a == 1.0 || a == -1.0 {
        // Perpendicular: the (infinite) axis pierces the plane exactly once, so
        // the cut is always a circle — there is no empty perpendicular case.
        // The axis meets the plane at t = ((o − c) · n̂) / a in f64; the
        // in-plane axes x̂, ŷ are exact.
        let t = (o - c).dot(n) / a;
        let cc = c + t * z_hat;
        let circle: PlacedCircle = Processor::with_transform(
            TrimmedCurve::new(UnitCircle::<Point3>::new(), (0.0, TAU)),
            frame(Vector3::unit_x(), Vector3::unit_y(), n, cc, r, r),
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
    } else if a == 0.0 {
        // Parallel: the offset from the axis to the plane δ = (c − o) · n̂,
        // enclosed in inari, is compared as δ² against r² by three_way.
        let delta = (interval_at(c.x) - interval_at(o.x)) * interval_at(n.x)
            + (interval_at(c.y) - interval_at(o.y)) * interval_at(n.y)
            + (interval_at(c.z) - interval_at(o.z)) * interval_at(n.z);
        let delta_sq = delta * delta;
        let r_sq = interval_at(r) * interval_at(r);
        let delta_f = (c - o).dot(n);
        match three_way(delta_sq, r_sq) {
            Some(Ordering::Less) => {
                // Two lines: the in-plane direction perpendicular to both n̂ and
                // ẑ is û = normalize(n̂ × ẑ); the foot is f = c − δ_f·n̂ and the
                // half-chord is s = √(r² − δ_f²).
                let u_hat = n.cross(z_hat).normalize();
                let f = c - delta_f * n;
                let s = (r * r - delta_f * delta_f).sqrt();
                let p_minus = f - s * u_hat;
                let p_plus = f + s * u_hat;
                let mut props = PropMap::new();
                props.set(Prop::AnalyticCarrier, Truth::True);
                Ok(Certified::new(
                    AnalyticIntersection::TwoCurves([
                        ExactCurve::Line(Line(p_minus, p_minus + z_hat)),
                        ExactCurve::Line(Line(p_plus, p_plus + z_hat)),
                    ]),
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
                // Tangent: a single line along the axis, through the foot
                // f = c − δ_f·n̂.
                let f = c - delta_f * n;
                let mut props = PropMap::new();
                props.set(Prop::AnalyticCarrier, Truth::True);
                Ok(Certified::new(
                    AnalyticIntersection::TangentLine(Line(f, f + z_hat)),
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
    } else {
        // Ellipse: 0 < |a| < 1 (an exact component, so decided without
        // intervals). The centre is where the axis pierces the plane,
        // t = ((o − c) · n̂) / a; the minor semi-axis r lies along
        // û = normalize(n̂ × ẑ) and the major semi-axis r/|a| along
        // v̂ = n̂ × û.
        let t = (o - c).dot(n) / a;
        let cc = c + t * z_hat;
        let u_hat = n.cross(z_hat).normalize();
        let v_hat = n.cross(u_hat);
        let major = r / a.abs();
        let ellipse: PlacedCircle = Processor::with_transform(
            TrimmedCurve::new(UnitCircle::<Point3>::new(), (0.0, TAU)),
            frame(v_hat, u_hat, n, cc, major, r),
        );
        let mut props = PropMap::new();
        props.set(Prop::AnalyticCarrier, Truth::True);
        Ok(Certified::new(
            AnalyticIntersection::Curve(ExactCurve::Ellipse(ellipse)),
            Certificate {
                props,
                method: Method::Exact,
                budget_left: Budget::new(0, 0, 0),
                margin: Margin::UNBOUNDED,
                modulus: Modulus::Unbounded,
            },
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use inari::const_interval;
    use truck_base::cgmath64::EuclideanSpace;
    use truck_geotrait::ParametricCurve;

    /// Number of sample points per emitted curve.
    const N: usize = 32;
    /// Float slack on unit-scale witness residuals, direction cosines and
    /// semi-axis lengths — dimensionless in every use, never a model-space
    /// length.
    const SLACK: f64 = 1.0e-12; // H-3: float slack on unit-scale witness residuals and axis ratios, not a length
    /// x = 3/5: the dyadic plane offset of the two-lines witness (the 3-4-5
    /// triple).
    const THREE_FIFTHS: f64 = 3.0 / 5.0;

    /// Builds a cylinder witness from its center and radius, avoiding `unwrap`
    /// (H-1): every witness radius is finite and positive, so construction
    /// cannot refuse.
    fn witness(center: Point3, radius: f64) -> Cylinder {
        match Cylinder::new(center, radius) {
            Ok(certified) => certified.value,
            Err(refusal) => unreachable!("cylinder construction refused: {refusal:?}"),
        }
    }

    /// Extracts the classified value of a decisive outcome, avoiding `unwrap`
    /// (H-1): the dyadic witnesses below are all decisively classified, so a
    /// refusal is a classification regression.
    fn value_of(out: AnalyticOutcome) -> AnalyticIntersection {
        match out {
            Ok(certified) => certified.value,
            Err(refusal) => unreachable!("expected a decisive classification, got {refusal:?}"),
        }
    }

    /// A plane x = x0 with normal exactly +x̂ — the parallel-offset witnesses.
    fn plane_x(x0: f64) -> Plane {
        Plane::new(
            Point3::new(x0, 0.0, 0.0),
            Point3::new(x0, 1.0, 0.0),
            Point3::new(x0, 0.0, 1.0),
        )
    }

    /// The plane z = 3 with normal exactly ẑ — the perpendicular witness.
    fn plane_z3() -> Plane {
        Plane::new(
            Point3::new(0.0, 0.0, 3.0),
            Point3::new(1.0, 0.0, 3.0),
            Point3::new(0.0, 1.0, 3.0),
        )
    }

    /// The plane through the origin spanned by (0,0,0), (0,1,0), (1,0,1):
    /// normal (1,0,−1)/√2, a decisive 45° tilt — the ellipse witness.
    fn tilted_plane() -> Plane {
        Plane::new(
            Point3::origin(),
            Point3::new(0.0, 1.0, 0.0),
            Point3::new(1.0, 0.0, 1.0),
        )
    }

    /// Asserts an `Ok` certificate is `Method::Exact` with the analytic-carrier
    /// property set (BG-EVD-002: field-by-field, no convenience constructor).
    fn assert_exact(out: AnalyticOutcome) {
        match out {
            Ok(certified) => {
                assert_eq!(certified.cert.method, Method::Exact);
                assert_eq!(certified.cert.props.get(Prop::AnalyticCarrier), Truth::True);
            }
            Err(refusal) => unreachable!("expected a certified outcome, got {refusal:?}"),
        }
    }

    #[test]
    fn pcyl_two_lines_when_the_plane_is_parallel_to_the_axis() {
        // Plane x = 3/5 cutting the cylinder at the origin of radius exactly 1:
        // δ = 3/5 < 1 gives the two lines x = 3/5, y = ±4/5 — the 3-4-5 triple,
        // every coordinate dyadic, so the interval predicates are exact.
        let cylinder = witness(Point3::origin(), 1.0);
        let plane = plane_x(THREE_FIFTHS);
        let value = value_of(plane_cylinder(&plane, &cylinder));
        let curves = match value {
            AnalyticIntersection::TwoCurves(curves) => curves,
            other => unreachable!("expected TwoCurves, got {other:?}"),
        };
        let [ExactCurve::Line(line_a), ExactCurve::Line(line_b)] = curves else {
            unreachable!("expected two Lines")
        };
        for (i, line) in [line_a, line_b].iter().enumerate() {
            for j in 0..N {
                let t = -2.0 + 4.0 * (j as f64) / (N as f64 - 1.0);
                let p = line.subs(t);
                let radial = p.x * p.x + p.y * p.y;
                assert!(
                    (radial - 1.0).abs() < SLACK,
                    "line {i} point {p:?} leaves the cylinder"
                );
                assert!(
                    (p.x - THREE_FIFTHS).abs() < SLACK,
                    "line {i} point {p:?} leaves the plane"
                );
            }
        }
    }

    #[test]
    fn pcyl_tangent_line_and_empty_when_parallel() {
        // Plane x = 1: δ² = r² → the single tangent line x = 1, y = 0. Plane
        // x = 2: δ² > r² → empty. Every coordinate dyadic.
        let cylinder = witness(Point3::origin(), 1.0);

        let value = value_of(plane_cylinder(&plane_x(1.0), &cylinder));
        let line = match value {
            AnalyticIntersection::TangentLine(line) => line,
            other => unreachable!("expected a tangent line, got {other:?}"),
        };
        for j in 0..N {
            let t = -2.0 + 4.0 * (j as f64) / (N as f64 - 1.0);
            let p = line.subs(t);
            let radial = p.x * p.x + p.y * p.y;
            assert!(
                (radial - 1.0).abs() < SLACK,
                "tangent point {p:?} leaves the cylinder"
            );
            assert!(
                (p.x - 1.0).abs() < SLACK,
                "tangent point {p:?} leaves the plane x = 1"
            );
        }

        let value = value_of(plane_cylinder(&plane_x(2.0), &cylinder));
        assert!(matches!(value, AnalyticIntersection::Empty));
    }

    #[test]
    fn pcyl_circle_when_the_plane_is_perpendicular() {
        // Plane z = 3 with normal ẑ: the axis pierces it exactly once, so the
        // cut is always a circle centred (cx, cy, 3) of radius r — there is no
        // empty perpendicular case.
        const CENTER: (f64, f64, f64) = (1.0, 2.0, 0.0);
        let cylinder = witness(Point3::new(CENTER.0, CENTER.1, CENTER.2), 1.0);
        let plane = plane_z3();
        let value = value_of(plane_cylinder(&plane, &cylinder));
        let circle = match value {
            AnalyticIntersection::Curve(ExactCurve::Circle(circle)) => circle,
            other => unreachable!("expected a circle, got {other:?}"),
        };
        for i in 0..N {
            let t = std::f64::consts::TAU * (i as f64) / (N as f64 - 1.0);
            let p = circle.subs(t);
            assert!(
                (p.z - 3.0).abs() < SLACK,
                "point {p:?} leaves the plane z = 3"
            );
            let radial = (p.x - CENTER.0) * (p.x - CENTER.0) + (p.y - CENTER.1) * (p.y - CENTER.1);
            assert!(
                (radial - 1.0).abs() < SLACK,
                "point {p:?} leaves the cylinder"
            );
        }
    }

    #[test]
    fn pcyl_ellipse_when_tilted() {
        // The plane through the origin spanned by (0,0,0), (0,1,0), (1,0,1) has
        // normal ∝ (1,0,−1)/√2, so a = −1/√2 — a decisive 45° tilt strictly
        // between 0 and 1 → ellipse. Sample the emitted ellipse and assert
        // every point lies on both carriers; then read the semi-axes off the
        // parameterization (t = 0 and t = π/2 hit the two axes exactly:
        // cos 0 = 1, sin π/2 = 1) and check the ratio is √2 = 1/cos 45°.
        let cylinder = witness(Point3::origin(), 1.0);
        let plane = tilted_plane();
        let value = value_of(plane_cylinder(&plane, &cylinder));
        let ellipse = match value {
            AnalyticIntersection::Curve(ExactCurve::Ellipse(ellipse)) => ellipse,
            other => unreachable!("expected an ellipse, got {other:?}"),
        };
        const SAMPLES: usize = 64;
        for i in 0..SAMPLES {
            let t = std::f64::consts::TAU * (i as f64) / (SAMPLES as f64 - 1.0);
            let p = ellipse.subs(t);
            let radial = p.x * p.x + p.y * p.y;
            assert!(
                (radial - 1.0).abs() < SLACK,
                "point {p:?} leaves the cylinder"
            );
            assert!(
                (p.x - p.z).abs() < SLACK,
                "point {p:?} leaves the plane x = z"
            );
        }
        let center = Point3::origin();
        let major = (ellipse.subs(0.0) - center).magnitude();
        let minor = (ellipse.subs(std::f64::consts::FRAC_PI_2) - center).magnitude();
        assert!(
            (major - std::f64::consts::SQRT_2).abs() < SLACK,
            "major semi-axis {major} != √2"
        );
        assert!((minor - 1.0).abs() < SLACK, "minor semi-axis {minor} != 1");
        assert!(
            (major / minor - std::f64::consts::SQRT_2).abs() < SLACK,
            "semi-axis ratio {} != √2",
            major / minor
        );
    }

    #[test]
    fn pcyl_undecidable_predicates_refuse() {
        // Hand-built intervals pin the exact predicate semantics: a [-w, w]
        // interval is neither decisively-zero nor excludes-zero.
        let zero = const_interval!(0.0, 0.0);
        assert!(decisively_zero(zero));
        assert!(!excludes_zero(zero));
        let wide = const_interval!(-1.0e-12, 1.0e-12); // H-3: interval bound on a dimensionless squared-distance difference, not a length
        assert!(!decisively_zero(wide));
        assert!(!excludes_zero(wide));
        assert!(excludes_zero(const_interval!(1.0, 2.0)));
        assert!(excludes_zero(const_interval!(-2.0, -1.0)));

        // Overlapping non-degenerate intervals are undecidable, as are equal
        // non-degenerate intervals; strict separation and degeneracy decide.
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

        // A bit-neighbour parallel-offset witness stays decisive: the plane
        // normal is exactly (1, 0, 0), so δ is dyadic and its square is a
        // degenerate interval — no straddle is possible on the dyadic axis.
        let cylinder = witness(Point3::origin(), 1.0);
        for offset in [
            f64::from_bits(1.0_f64.to_bits() + 1),
            f64::from_bits(1.0_f64.to_bits() - 1),
        ] {
            let out = plane_cylinder(&plane_x(offset), &cylinder);
            assert!(
                out.is_ok(),
                "bit-neighbour parallel offset {offset} must stay decisive"
            );
        }

        // A genuine straddle: a parallel plane whose normal
        // normalize(ẑ × (0.6, 0.8, 0)) = (−0.8, 0.6000000000000001, 0) has
        // non-dyadic components, placed tangent to the radius. The offset
        // `o.y = −1/3` (computed in f64) is non-dyadic too, so the offset
        // enclosure rounds outward: δ² = [1, 1 + ε] strictly contains r² = [1, 1],
        // and neither Less nor Equal nor Greater is decidable. Refuse, never
        // guess.
        let o = Point3::new(1.0, (0.8 * 1.0 - 1.0) / 0.6, 0.0);
        let straddle_plane = Plane::new(o, o + Vector3::unit_z(), o + Vector3::new(0.6, 0.8, 0.0));
        let out = plane_cylinder(&straddle_plane, &cylinder);
        assert!(
            matches!(out, Err(Refusal::NumericallyUnresolved { .. })),
            "a tangent placement with a non-dyadic normal must refuse, got {out:?}"
        );
    }

    #[test]
    fn pcyl_certificate_is_exact() {
        // A two-lines, a circle and an ellipse outcome each carry
        // method == Exact and the AnalyticCarrier prop, field-by-field at every
        // return site (BG-EVD-002).
        let cylinder = witness(Point3::origin(), 1.0);
        assert_exact(plane_cylinder(&plane_x(THREE_FIFTHS), &cylinder));
        assert_exact(plane_cylinder(
            &plane_z3(),
            &witness(Point3::new(1.0, 2.0, 0.0), 1.0),
        ));
        assert_exact(plane_cylinder(&tilted_plane(), &cylinder));
    }
}
