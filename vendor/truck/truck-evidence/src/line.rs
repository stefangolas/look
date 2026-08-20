//! BG-ENC-002-LINE: `EnclosureCurve` for the `Line<Point3>` carrier.
//!
//! A line is affine:
//!
//! ```text
//! C(t) = p0 + t·(p1 − p0)
//! ```
//!
//! so the enclosure over a box is exact interval arithmetic on the
//! parameterisation (no subdivision needed), the derivative is constant, and
//! every higher derivative vanishes. The domain is not restricted to `[0, 1]`:
//! `ParametricCurve` evaluates the line for any `t`, so the enclosure must be
//! correct for a `tt` that is negative, straddles zero, or lies beyond 1.
//! Every method here is closed-form.

use crate::enclosure::{Box3, DirCone, EnclosureCurve};
use inari::Interval;
use truck_base::cgmath64::{InnerSpace, Point3};
use truck_geometry::specifieds::Line;

/// A degenerate interval from a runtime `f64`. Finite coordinates always
/// construct; a NaN widens to the empty interval rather than panicking (H-1).
/// Duplicated from `plane.rs`, which is outside this packet's write set; the
/// sibling carriers copy it the same way rather than coupling on one shared
/// definition.
fn interval_at(x: f64) -> Interval {
    Interval::try_from((x, x)).unwrap_or(Interval::EMPTY)
}

/// True when every coordinate interval of the box contains `0.0`.
fn contains_zero(b: &Box3) -> bool {
    b.x.contains(0.0) && b.y.contains(0.0) && b.z.contains(0.0)
}

impl EnclosureCurve for Line<Point3> {
    fn enclose(&self, tt: Interval) -> Box3 {
        // C = p0 + t·d with d = p1 − p0. Each coordinate is p0_c + tt·d_c in
        // interval arithmetic: affine, hence exact up to outward rounding for
        // any tt. Mixed-sign multiplication is inari's job; do not hand-roll a
        // sign case analysis.
        let p0 = self.0;
        let d = self.1 - self.0;
        Box3 {
            x: interval_at(p0.x) + tt * interval_at(d.x),
            y: interval_at(p0.y) + tt * interval_at(d.y),
            z: interval_at(p0.z) + tt * interval_at(d.z),
        }
    }

    fn enclose_der(&self, n: usize, tt: Interval) -> Box3 {
        match n {
            // der_n(0, t) = subs(t).to_vec(): a vector whose components equal
            // the point's coordinates, so the n = 0 enclosure is the point box.
            // Match the carrier; do not "fix" it.
            0 => self.enclose(tt),
            // der is the constant d = p1 − p0. Take the inari difference of
            // the endpoint intervals so the subtraction's rounding is captured
            // rather than assumed away.
            1 => {
                let p0 = self.0;
                let p1 = self.1;
                Box3 {
                    x: interval_at(p1.x) - interval_at(p0.x),
                    y: interval_at(p1.y) - interval_at(p0.y),
                    z: interval_at(p1.z) - interval_at(p0.z),
                }
            }
            // Every n >= 2 derivative vanishes identically on an affine curve.
            _ => Box3 {
                x: interval_at(0.0),
                y: interval_at(0.0),
                z: interval_at(0.0),
            },
        }
    }

    fn tangent_cone(&self, tt: Interval) -> Option<DirCone> {
        // The direction is the constant d = p1 − p0, independent of tt. The
        // trait's contract: None when the derivative enclosure contains 0 —
        // exactly a degenerate Line(p, p). Normalize only after the zero check
        // (normalizing a zero vector yields NaN).
        let d = self.1 - self.0;
        let der = self.enclose_der(1, tt);
        if contains_zero(&der) {
            None
        } else {
            Some(DirCone {
                axis: d.normalize(),
                // A constant direction has zero spread; 0.0 is the honest
                // half-angle. Do not pad it to make containment tests easier —
                // any tolerance belongs in a test helper, not in the cone.
                half_angle: 0.0,
            })
        }
    }
}

#[cfg(test)]
// Test-only allow: H-1 bans unwrap/expect on paths reachable from untrusted
// geometry. Unit-test assertions on hand-built witnesses are not such a path.
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::harness::assert_encloses_curve;
    use inari::const_interval;
    use truck_geotrait::ParametricCurve;

    fn axis_aligned() -> Line<Point3> {
        Line(Point3::new(1.0, 2.0, 3.0), Point3::new(4.0, 2.0, 3.0))
    }

    fn oblique() -> Line<Point3> {
        Line(Point3::new(-1.0, 0.5, 2.0), Point3::new(2.0, -3.0, 1.0))
    }

    /// The interval parameters every sampling test walks: the whole parameter
    /// range, a sub-interval, an entirely negative one, one straddling zero,
    /// one just beyond `t = 1`, and one far beyond it.
    fn sample_tts() -> [Interval; 6] {
        [
            const_interval!(0.0, 1.0),
            const_interval!(0.2, 0.7),
            const_interval!(-3.0, -1.0),
            const_interval!(-2.0, 0.5),
            const_interval!(1.0, 4.0),
            const_interval!(5.0, 8.0),
        ]
    }

    fn widths(b: &Box3) -> [f64; 3] {
        [
            b.x.sup() - b.x.inf(),
            b.y.sup() - b.y.inf(),
            b.z.sup() - b.z.inf(),
        ]
    }

    /// Asserts that an interval's width matches the exact f64 span of its two
    /// endpoints up to one outward rounding step, and that the bounds sit on
    /// the sound side of `[min, max]`. Asserted as a relation on widths, not
    /// bit-equality: the affine image is the tightest box inari can express.
    fn assert_endpoint_exact(a: f64, b: f64, iv: Interval) {
        let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
        assert!(iv.inf() <= lo, "inf {0} escaped below {lo}", iv.inf());
        assert!(iv.sup() >= hi, "sup {0} escaped above {hi}", iv.sup());
        let w_enc = iv.sup() - iv.inf();
        let w_exact = (a - b).abs();
        let slack = 8.0 * f64::EPSILON * (1.0 + w_exact);
        assert!(
            (w_enc - w_exact).abs() <= slack,
            "enclosure width {w_enc} differs from exact span {w_exact} by more than one rounding step"
        );
    }

    #[test]
    fn line_encloses_sampled_points() {
        // BG-ENC-001 soundness: every sampled point lies inside the enclosure,
        // for an axis-aligned line and an oblique one, over tt's that are
        // negative, straddle zero, or exceed 1.
        let a = axis_aligned();
        let o = oblique();
        for tt in sample_tts() {
            assert_encloses_curve(&a, tt, 20);
            assert_encloses_curve(&o, tt, 20);
        }
    }

    #[test]
    fn line_enclosure_is_exact_at_the_endpoints() {
        // For tt = [0, 1] an affine carrier encloses exactly [min, max] per
        // coordinate. This is the property that distinguishes an affine carrier
        // from a subdivided one.
        let line = oblique();
        let b = line.enclose(const_interval!(0.0, 1.0));
        assert_endpoint_exact(line.0.x, line.1.x, b.x);
        assert_endpoint_exact(line.0.y, line.1.y, b.y);
        assert_endpoint_exact(line.0.z, line.1.z, b.z);
    }

    #[test]
    fn line_enclosure_converges_under_bisection() {
        // The harness's assert_converges is written against EnclosureSurface,
        // so write the curve loop locally: halving tt must at least halve each
        // component width (up to rounding), down to depth ~20.
        let line = oblique();
        let initial = line.enclose(const_interval!(-2.0, 3.0));
        let mut tt = const_interval!(-2.0, 3.0);
        for _ in 0..20 {
            let mid = tt.inf() + (tt.sup() - tt.inf()) / 2.0;
            let left = Interval::try_from((tt.inf(), mid)).unwrap();
            let right = Interval::try_from((mid, tt.sup())).unwrap();
            let parent = widths(&line.enclose(tt));
            let lw = widths(&line.enclose(left));
            let rw = widths(&line.enclose(right));
            for (&wp, (&wl, &wr)) in parent.iter().zip(lw.iter().zip(rw.iter())) {
                let half = wp * 0.5;
                let slack = 16.0 * f64::EPSILON * (1.0 + wp);
                assert!(
                    wl <= half + slack,
                    "left half width {wl} not <= half of {wp}"
                );
                assert!(
                    wr <= half + slack,
                    "right half width {wr} not <= half of {wp}"
                );
            }
            tt = left;
        }
        assert!(
            line.enclose(tt).width() < initial.width(),
            "enclosure did not shrink below the initial width"
        );
    }

    #[test]
    fn line_tangent_cone_is_the_single_direction() {
        // An ordinary line: a cone along (p1 − p0).normalize() with zero spread
        // for any tt. A degenerate Line(p, p) has a derivative enclosure
        // containing 0, so no cone.
        let line = oblique();
        let d = line.1 - line.0;
        for tt in [
            const_interval!(0.0, 1.0),
            const_interval!(-2.0, 0.0),
            const_interval!(1.0, 3.0),
            const_interval!(-1.0, 2.0),
        ] {
            let cone = line.tangent_cone(tt).expect("ordinary line has a cone");
            assert_eq!(cone.half_angle, 0.0);
            let expected = d.normalize();
            assert!(
                (cone.axis - expected).magnitude() < 1.0e-12, // H-3: float slack between two unit direction vectors, not a length
                "axis {:?} != expected {:?}",
                cone.axis,
                expected
            );
        }
        let degenerate = Line(Point3::new(1.0, 2.0, 3.0), Point3::new(1.0, 2.0, 3.0));
        assert!(degenerate.tangent_cone(const_interval!(0.0, 1.0)).is_none());
        assert!(degenerate
            .tangent_cone(const_interval!(-5.0, 100.0))
            .is_none());
    }

    #[test]
    fn line_der_enclosures_are_constant_then_zero() {
        let line = oblique();
        let mut first_der: Option<Box3> = None;
        for tt in [
            const_interval!(0.0, 1.0),
            const_interval!(-10.0, 5.0),
            const_interval!(2.0, 7.0),
        ] {
            // n = 1: the same box for every tt, and it contains the sampled
            // der(t) = p1 − p0 (constant).
            let b1 = line.enclose_der(1, tt);
            if let Some(prev) = first_der {
                assert_eq!(prev, b1, "n=1 enclosure depends on tt");
            } else {
                first_der = Some(b1);
            }
            for t in [tt.inf(), tt.sup(), tt.inf() + (tt.sup() - tt.inf()) / 2.0] {
                let v = line.der(t);
                assert!(b1.contains(Point3::new(v.x, v.y, v.z)));
            }
            // n = 0 agrees with enclose (der_n(0, t) = subs(t).to_vec()).
            assert_eq!(line.enclose_der(0, tt), line.enclose(tt));
            // n >= 2: the zero box.
            for n in [2usize, 5usize] {
                let z = line.enclose_der(n, tt);
                assert!(z.contains(Point3::new(0.0, 0.0, 0.0)));
                assert_eq!(z.width(), 0.0);
            }
        }
    }
}
