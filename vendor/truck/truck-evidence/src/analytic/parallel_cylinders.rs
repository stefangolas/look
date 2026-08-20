//! BG-ANA-001-PARCYL: parallel-axis cylinders — two lines, one tangent line,
//! or empty. This shard owns the **margin sweep** test of BG-ANA-002: two
//! cylinders walked through tangency (`d → r₀+r₁`) must switch cleanly
//! transverse → tangent → disjoint with no band of wrong-but-confident
//! answers near the crossing.
//!
//! The canonical [`Cylinder`] runs along the **z axis** through its `center`,
//! so any two of them are a parallel-axis pair by construction and the whole
//! classification reduces to comparing the axis-to-axis distance `d` against
//! `r0 + r1` and `|r0 − r1|`.
//!
//! Every verdict is decided by exact predicates on the f64 carrier
//! parameters. `d2 − (r0 + r1)²` and `d2 − (r0 − r1)²` are computed as
//! outward-rounded `inari::Interval`s and compared against zero with the
//! three-way comparator, so a dyadic input classifies exactly and an
//! enclosure that merely contains zero proves nothing.
//!
//! `Method::Exact` on the certificate means the **classification** is exact:
//! decisive interval predicates on the f64 carrier parameters chose the arm.
//! The emitted curves are the closed-form intersections, whose coordinates
//! are computed in plain f64; the spec's obligation is "lies on both carriers
//! to machine precision", which the tests assert with an H-3-commented slack.
//! There is no `τ_rep` anywhere on this path.

use super::{AnalyticIntersection, AnalyticOutcome, ExactCurve};
use inari::Interval;
use std::cmp::Ordering;
use truck_base::cgmath64::{Point3, Vector3};
use truck_base::evidence::{
    Budget, Certificate, Certified, Margin, Method, Modulus, Prop, PropMap, Refusal, Truth,
    UnresolvedWitness,
};
use truck_geometry::specifieds::{Cylinder, Line};

/// The common z axis direction of every canonical cylinder.
fn z_axis() -> Vector3 {
    Vector3::new(0.0, 0.0, 1.0)
}

/// Whether the interval is decisively the point zero, `[0, 0]`.
#[allow(clippy::float_cmp)] // deliberate: exact endpoint equality is the definition of decisively-zero
fn decisively_zero(i: Interval) -> bool {
    i.inf() == 0.0 && i.sup() == 0.0
}

/// Whether the interval excludes zero entirely: wholly positive or wholly
/// negative.
fn excludes_zero(i: Interval) -> bool {
    i.inf() > 0.0 || i.sup() < 0.0
}

/// Whether an enclosure straddles the zero threshold: it contains zero
/// without being the point zero, so it is neither decisively zero nor does
/// it exclude zero. Such an enclosure proves nothing (BG-ANA-002).
fn straddles_zero(i: Interval) -> bool {
    !(decisively_zero(i) || excludes_zero(i))
}

/// The three-way comparison of two intervals as exact predicates.
///
/// `Some(Less)` iff `a` lies wholly below `b`; `Some(Greater)` iff `a` lies
/// wholly above `b`; `Some(Equal)` iff both intervals are the same degenerate
/// point interval; `None` when the two enclosures overlap without both being
/// a point — the verdict an exact classification may never guess (BG-ANA-002).
#[allow(clippy::float_cmp)] // deliberate: degenerate-and-identical point intervals compare by exact endpoint equality
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

/// The single refusal shape of this shard: an undecidable predicate is a
/// stop, not a guess (BG-ANA-002).
fn unresolved() -> Refusal {
    Refusal::NumericallyUnresolved {
        spent: Budget::new(0, 0, 0),
        witness: UnresolvedWitness::RootNotIsolated,
    }
}

/// Wraps an f64 carrier parameter as the degenerate interval `[x, x]`. Inari
/// rejects only non-finite or inverted endpoints; validated carriers (H-1)
/// cannot supply them, so a failure means the predicate is uncomputable and
/// the caller refuses.
fn singleton(x: f64) -> Result<Interval, Refusal> {
    Interval::try_from((x, x)).map_err(|_| unresolved())
}

/// Classifies two parallel-axis cylinders by exact predicates on their
/// carrier parameters (BG-ANA-002 decision 5).
///
/// `Method::Exact` here means the classification is exact: every arm is
/// chosen by a decisive interval predicate on the f64 carrier parameters,
/// and the emitted curves are the closed-form intersections. The curve
/// coordinates are computed in f64; the spec's obligation is "lies on both
/// carriers to machine precision", asserted with an H-3-commented slack in
/// the tests. There is no `τ_rep` on this path.
#[allow(clippy::float_cmp)] // deliberate: exact f64 equality of the radius parameters decides coincident vs concentric (BG-ANA-002 decision 5.1)
pub fn parallel_cylinders(cylinder0: &Cylinder, cylinder1: &Cylinder) -> AnalyticOutcome {
    let c0 = cylinder0.center();
    let r0 = cylinder0.radius();
    let c1 = cylinder1.center();
    let r1 = cylinder1.radius();

    // Predicate quantities, computed in inari (outward rounding): the
    // horizontal axis-to-axis distance squared, and the two tangency
    // discriminants. The z offsets do not enter; the axes are parallel.
    let x0 = singleton(c0.x)?;
    let y0 = singleton(c0.y)?;
    let x1 = singleton(c1.x)?;
    let y1 = singleton(c1.y)?;
    let dx = x1 - x0;
    let dy = y1 - y0;
    let d2 = dx * dx + dy * dy;
    let r0i = singleton(r0)?;
    let r1i = singleton(r1)?;
    let sum_r = r0i + r1i;
    let diff_r = r0i - r1i;
    let zout = d2 - sum_r * sum_r;
    let zin = d2 - diff_r * diff_r;
    let zero = singleton(0.0)?;

    // Step 1: same axis? `three_way(d2, [0, 0]) == Some(Equal)`.
    match three_way(d2, zero) {
        Some(Ordering::Equal) => {
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
            // Concentric, nested, no contact.
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
        // Distinct axes: proceed to the external predicate.
        Some(Ordering::Greater) => {}
        // A decisively negative squared distance is contradictory for a sum
        // of squares; refuse rather than guess.
        Some(Ordering::Less) => return Err(unresolved()),
        // The axis separation straddles zero: the same-axis verdict is
        // undecidable.
        None => {
            assert!(straddles_zero(d2));
            return Err(unresolved());
        }
    }

    // Step 2: the outer predicate runs first; an undecidable outer predicate
    // refuses without consulting the inner one (it must not be resolved by a
    // decisive inner predicate on the other side of the tangency).
    match three_way(zout, zero) {
        // Parallel axes too far apart to touch: the parallelism IS the
        // classification.
        Some(Ordering::Greater) => {
            let mut props = PropMap::new();
            props.set(Prop::AnalyticCarrier, Truth::True);
            return Ok(Certified::new(
                AnalyticIntersection::Parallel,
                Certificate {
                    props,
                    method: Method::Exact,
                    budget_left: Budget::new(0, 0, 0),
                    margin: Margin::UNBOUNDED,
                    modulus: Modulus::Unbounded,
                },
            ));
        }
        // External tangency: d = r0 + r1, the contact line at ℓ = r0.
        Some(Ordering::Equal) => {
            let line = tangent_line(c0, c1, r0);
            let mut props = PropMap::new();
            props.set(Prop::AnalyticCarrier, Truth::True);
            return Ok(Certified::new(
                AnalyticIntersection::TangentLine(line),
                Certificate {
                    props,
                    method: Method::Exact,
                    budget_left: Budget::new(0, 0, 0),
                    margin: Margin::UNBOUNDED,
                    modulus: Modulus::Unbounded,
                },
            ));
        }
        // zout decisively negative: the axes are closer than r0 + r1.
        Some(Ordering::Less) => {}
        // zout straddles zero: the outer predicate is undecidable, and it
        // must not be resolved by a decisive inner predicate on the other
        // side of the tangency — refuse without consulting zin.
        None => {
            assert!(straddles_zero(zout));
            return Err(unresolved());
        }
    }

    // Step 3: the inner (internal) predicate.
    match three_way(zin, zero) {
        // Internal tangency: d = |r0 − r1|. The contact line sits at ℓ = r0
        // when r0 ≥ r1 and at ℓ = −r0 when the larger cylinder is c1.
        Some(Ordering::Equal) => {
            let ell = if r0 >= r1 { r0 } else { -r0 };
            let line = tangent_line(c0, c1, ell);
            let mut props = PropMap::new();
            props.set(Prop::AnalyticCarrier, Truth::True);
            Ok(Certified::new(
                AnalyticIntersection::TangentLine(line),
                Certificate {
                    props,
                    method: Method::Exact,
                    budget_left: Budget::new(0, 0, 0),
                    margin: Margin::UNBOUNDED,
                    modulus: Modulus::Unbounded,
                },
            ))
        }
        // One cylinder strictly inside the other, no contact.
        Some(Ordering::Less) => {
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
        // Transverse: two lines (zout was decisively negative, so the two
        // axes do cross the r0 + r1 threshold).
        Some(Ordering::Greater) => {
            let lines = transverse_lines(c0, r0, c1, r1);
            let mut props = PropMap::new();
            props.set(Prop::AnalyticCarrier, Truth::True);
            Ok(Certified::new(
                AnalyticIntersection::TwoCurves(lines),
                Certificate {
                    props,
                    method: Method::Exact,
                    budget_left: Budget::new(0, 0, 0),
                    margin: Margin::UNBOUNDED,
                    modulus: Modulus::Unbounded,
                },
            ))
        }
        // zin straddles zero: the internal tangency verdict is undecidable.
        None => {
            assert!(straddles_zero(zin));
            Err(unresolved())
        }
    }
}

/// The single tangency line `c0 + ℓ·d̂` extruded along ẑ, where `ℓ` is the
/// signed distance of the contact line from `c0` along the in-plane axis
/// direction `d̂` (BG-ANA-002 decision 6): `r0` for external tangency and for
/// internal tangency when `r0 ≥ r1`, `−r0` when the larger cylinder is `c1`.
fn tangent_line(c0: Point3, c1: Point3, ell: f64) -> Line<Point3> {
    let dx = c1.x - c0.x;
    let dy = c1.y - c0.y;
    let d = (dx * dx + dy * dy).sqrt();
    let dhat = Vector3::new(dx / d, dy / d, 0.0);
    let m = c0 + ell * dhat;
    Line(m, m + z_axis())
}

/// The two transverse intersection lines of two parallel-axis cylinders
/// (BG-ANA-002 decision 6): chord midpoint `m`, half-chord `s` along the
/// in-plane perpendicular `w = ẑ × d̂`, both lines extruded along ẑ.
fn transverse_lines(c0: Point3, r0: f64, c1: Point3, r1: f64) -> [ExactCurve; 2] {
    let dx = c1.x - c0.x;
    let dy = c1.y - c0.y;
    let d = (dx * dx + dy * dy).sqrt();
    let dhat = Vector3::new(dx / d, dy / d, 0.0);
    let ell = (d * d + r0 * r0 - r1 * r1) / (2.0 * d);
    let s = (r0 * r0 - ell * ell).sqrt();
    let m = c0 + ell * dhat;
    let w = Vector3::new(-dhat.y, dhat.x, 0.0);
    let z = z_axis();
    [
        ExactCurve::Line(Line(m - s * w, m - s * w + z)),
        ExactCurve::Line(Line(m + s * w, m + s * w + z)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use truck_base::cgmath64::{InnerSpace, Vector2};
    use truck_geotrait::ParametricCurve;

    /// Builds a canonical z-axis cylinder at the given xy centre. The test
    /// radii are positive and the centres finite, so `Cylinder::new` cannot
    /// refuse (H-1: the unreachable carries why).
    fn cyl(cx: f64, cy: f64, radius: f64) -> Cylinder {
        let Ok(cert) = Cylinder::new(Point3::new(cx, cy, 0.0), radius) else {
            unreachable!(
                "radius {radius} is positive and the centre finite, so construction succeeds"
            )
        };
        cert.value
    }

    /// Names an arm for the margin sweep: "two", "tangent", "parallel".
    fn arm_name(value: &AnalyticIntersection) -> &'static str {
        match value {
            AnalyticIntersection::TwoCurves(_) => "two",
            AnalyticIntersection::TangentLine(_) => "tangent",
            AnalyticIntersection::Parallel => "parallel",
            _ => "other",
        }
    }

    #[test]
    fn parcyl_two_lines_transverse() {
        let c0 = cyl(0.0, 0.0, 1.0);
        let c1 = cyl(1.0, 0.0, 1.0);
        let Ok(cert) = parallel_cylinders(&c0, &c1) else {
            unreachable!("the dyadic transverse input classifies, never refuses")
        };
        let AnalyticIntersection::TwoCurves([line0, line1]) = &cert.value else {
            unreachable!("d = 1 < r0 + r1 = 2 is transverse")
        };
        let ExactCurve::Line(line0) = line0 else {
            unreachable!("transverse parallel cylinders meet in two lines")
        };
        let ExactCurve::Line(line1) = line1 else {
            unreachable!("transverse parallel cylinders meet in two lines")
        };

        // Both lines at x = 1/2, y = ±√3/2; every sampled point lies on both
        // cylinders' radial equations to machine precision.
        for line in [line0, line1] {
            for i in 0..21 {
                let p = line.subs(i as f64 / 20.0);
                let radial0 = Vector2::new(p.x - c0.center().x, p.y - c0.center().y).magnitude();
                let radial1 = Vector2::new(p.x - c1.center().x, p.y - c1.center().y).magnitude();
                assert!(
                    (radial0 - 1.0).abs() < 1.0e-12, // H-3: dimensionless radial residual on a unit-radius witness, not a length
                    "point {p:?} escapes cylinder0: radial {radial0}"
                );
                assert!(
                    (radial1 - 1.0).abs() < 1.0e-12, // H-3: dimensionless radial residual on a unit-radius witness, not a length
                    "point {p:?} escapes cylinder1: radial {radial1}"
                );
            }
        }

        let base0 = line0.subs(0.0);
        let base1 = line1.subs(0.0);
        assert!(
            (base0.x - 0.5).abs() < 1.0e-12, // H-3: dimensionless coordinate slack on a unit-scale witness, not a length
            "line0 not at x = 1/2: {base0:?}"
        );
        assert!(
            (base1.x - 0.5).abs() < 1.0e-12, // H-3: dimensionless coordinate slack on a unit-scale witness, not a length
            "line1 not at x = 1/2: {base1:?}"
        );
        assert!(
            (base0.y + base1.y).abs() < 1.0e-12, // H-3: dimensionless coordinate slack on a unit-scale witness, not a length
            "lines not symmetric about y = 0: {base0:?}, {base1:?}"
        );
        assert!(
            (base0.y.abs() - (3.0f64.sqrt() / 2.0)).abs() < 1.0e-12, // H-3: dimensionless slack between a unit-scale coordinate and its closed form √3/2, not a length
            "line0 |y| != √3/2: {base0:?}"
        );
        assert!(
            (base1.y.abs() - (3.0f64.sqrt() / 2.0)).abs() < 1.0e-12, // H-3: dimensionless slack between a unit-scale coordinate and its closed form √3/2, not a length
            "line1 |y| != √3/2: {base1:?}"
        );
    }

    #[test]
    fn parcyl_margin_sweep_switches_cleanly() {
        // r0 = r1 = 1, centre1 = (d, 0, 0), walking d across the external
        // tangency at d = 2. Every d is dyadic, written as arithmetic on
        // named consts so the interval predicates stay decisive at every step.
        const R: f64 = 1.0;
        const TWO: f64 = 2.0;
        const ONE_SIXTEENTH: f64 = 1.0 / 16.0;
        const ONE_TWO_FIVE_SIXTH: f64 = 1.0 / 256.0;
        let two_to_the_minus_20 = 2.0f64.powi(-20);

        let cases = [
            (TWO - ONE_SIXTEENTH, "two"),
            (TWO - ONE_TWO_FIVE_SIXTH, "two"),
            (TWO - two_to_the_minus_20, "two"),
            (TWO, "tangent"),
            (TWO + two_to_the_minus_20, "parallel"),
            (TWO + ONE_TWO_FIVE_SIXTH, "parallel"),
            (TWO + ONE_SIXTEENTH, "parallel"),
        ];

        let c0 = cyl(0.0, 0.0, R);
        for (d, expected) in cases {
            let c1 = cyl(d, 0.0, R);
            match parallel_cylinders(&c0, &c1) {
                Ok(cert) => {
                    assert_eq!(
                        arm_name(&cert.value),
                        expected,
                        "margin sweep arm at d = {d}"
                    );
                }
                // A refusal anywhere in the dyadic walk is a failure of the
                // design, not a flake (BG-ANA-002).
                Err(_) => unreachable!(
                    "margin sweep refused at d = {d}: the dyadic walk must stay decisive"
                ),
            }
        }
    }

    #[test]
    fn parcyl_internal_tangency_and_containment() {
        // r0 = 2, r1 = 1, d = 1: internal tangency. The contact line sits at
        // ℓ = r0 = 2 along d̂, i.e. at (2, 0) — verify the emitted line lies
        // on both cylinders before relying on the formula.
        let c0 = cyl(0.0, 0.0, 2.0);
        let c1 = cyl(1.0, 0.0, 1.0);
        let Ok(cert) = parallel_cylinders(&c0, &c1) else {
            unreachable!("the dyadic internal-tangency input classifies, never refuses")
        };
        let AnalyticIntersection::TangentLine(line) = &cert.value else {
            unreachable!("d = 1 = r0 − r1 is internal tangency")
        };
        for i in 0..21 {
            let p = line.subs(i as f64 / 20.0);
            let radial0 = Vector2::new(p.x - c0.center().x, p.y - c0.center().y).magnitude();
            let radial1 = Vector2::new(p.x - c1.center().x, p.y - c1.center().y).magnitude();
            assert!(
                (radial0 - 2.0).abs() < 1.0e-12, // H-3: dimensionless radial residual on a unit-scale witness, not a length
                "point {p:?} escapes cylinder0: radial {radial0}"
            );
            assert!(
                (radial1 - 1.0).abs() < 1.0e-12, // H-3: dimensionless radial residual on a unit-scale witness, not a length
                "point {p:?} escapes cylinder1: radial {radial1}"
            );
        }

        // r0 = 2, r1 = 1, d = 1/2: the smaller cylinder is strictly inside.
        let c2 = cyl(0.5, 0.0, 1.0);
        let Ok(cert) = parallel_cylinders(&c0, &c2) else {
            unreachable!("the dyadic containment input classifies, never refuses")
        };
        assert!(
            matches!(cert.value, AnalyticIntersection::Empty),
            "a contained cylinder meets the outer one nowhere"
        );
    }

    #[test]
    fn parcyl_coincident_and_concentric() {
        let a = cyl(0.0, 0.0, 1.0);

        // Same centre and radius: the cylinders coincide.
        let same = cyl(0.0, 0.0, 1.0);
        let Ok(cert) = parallel_cylinders(&a, &same) else {
            unreachable!("the coincident input classifies, never refuses")
        };
        assert!(matches!(cert.value, AnalyticIntersection::Coincident));

        // Same centre, different radii: concentric, no contact.
        let nested = cyl(0.0, 0.0, 2.0);
        let Ok(cert) = parallel_cylinders(&a, &nested) else {
            unreachable!("the concentric input classifies, never refuses")
        };
        assert!(matches!(cert.value, AnalyticIntersection::Empty));
    }

    #[test]
    fn parcyl_undecidable_predicates_refuse() {
        // A symmetric interval about zero is neither decisively zero nor
        // excludes zero: the enclosure merely contains zero and proves
        // nothing.
        let w = 1.0e-12; // H-3: dimensionless interval half-width of a hand-built predicate witness, not a length
        let straddle = Interval::try_from((-w, w)).unwrap_or(Interval::EMPTY);
        assert!(!decisively_zero(straddle));
        assert!(!excludes_zero(straddle));
        assert!(decisively_zero(
            Interval::try_from((0.0, 0.0)).unwrap_or(Interval::EMPTY)
        ));
        assert!(excludes_zero(
            Interval::try_from((1.0, 2.0)).unwrap_or(Interval::EMPTY)
        ));

        // Overlapping non-degenerate intervals give three_way == None.
        let a = Interval::try_from((-1.0, 1.0)).unwrap_or(Interval::EMPTY);
        let b = Interval::try_from((0.0, 2.0)).unwrap_or(Interval::EMPTY);
        assert_eq!(three_way(a, b), None);
        assert_eq!(three_way(b, a), None);

        // Degenerate, disjoint intervals stay decisive in both directions.
        let lo = Interval::try_from((1.0, 1.0)).unwrap_or(Interval::EMPTY);
        let hi = Interval::try_from((2.0, 2.0)).unwrap_or(Interval::EMPTY);
        assert_eq!(three_way(lo, hi), Some(Ordering::Less));
        assert_eq!(three_way(hi, lo), Some(Ordering::Greater));
        assert_eq!(three_way(lo, lo), Some(Ordering::Equal));

        // One-ulp tangency neighbours, d = 2 ± 1 ulp: the squared-distance
        // enclosure stays degenerate, so the predicates stay decisive and
        // neither witness refuses. A genuine straddle refusal is not
        // constructible from f64 carrier parameters.
        let bits = 2.0f64.to_bits();
        for witness in [f64::from_bits(bits - 1), f64::from_bits(bits + 1)] {
            let c1 = cyl(witness, 0.0, 1.0);
            let out = parallel_cylinders(&cyl(0.0, 0.0, 1.0), &c1);
            assert!(
                out.is_ok(),
                "bit-neighbour tangency witness d = {witness} refused"
            );
        }
    }

    #[test]
    fn parcyl_certificate_is_exact() {
        let cases = [
            (
                parallel_cylinders(&cyl(0.0, 0.0, 1.0), &cyl(1.0, 0.0, 1.0)),
                "transverse",
            ),
            (
                parallel_cylinders(&cyl(0.0, 0.0, 1.0), &cyl(2.0, 0.0, 1.0)),
                "external tangency",
            ),
            (
                parallel_cylinders(&cyl(0.0, 0.0, 1.0), &cyl(3.0, 0.0, 1.0)),
                "parallel",
            ),
        ];
        for (out, name) in cases {
            let Ok(cert) = out else {
                unreachable!("{name} classifies on dyadic input, never refuses")
            };
            assert_eq!(
                cert.cert.method,
                Method::Exact,
                "{name} must be certified Method::Exact"
            );
            assert_eq!(
                cert.cert.props.get(Prop::AnalyticCarrier),
                Truth::True,
                "{name} must carry AnalyticCarrier = True"
            );
        }
    }
}
