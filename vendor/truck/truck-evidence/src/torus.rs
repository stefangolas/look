//! BG-ENC-002-TORUS: `EnclosureSurface` for the `Torus` carrier.
//!
//! The torus is the first carrier with **two periodic angles** — which is what
//! makes it worth doing separately from the cylinder, and what makes interval
//! trig a hard requirement rather than a nicety. The parameterisation (read
//! off `truck-geometry/src/specifieds/torus.rs`) is
//!
//! ```text
//! S(u, v) = c + ((R + r·cos v)·cos u, (R + r·cos v)·sin u, r·sin v)
//! ```
//!
//! with `u` and `v` both in `[0, 2π)` and periodic, `R = large_radius` and
//! `r = small_radius`. `Torus::new` panics unless both radii are strictly
//! positive, so `R > 0` and `r > 0` are invariants this impl relies on. What
//! it does **not** guarantee is `R > r`: a torus with `r ≥ R` is a *spindle*
//! torus whose inner circle `R + r·cos v = 0` is a genuine singular circle —
//! `S_u` vanishes there and the normal is undefined. The carrier admits
//! spindles, so this impl must too; that is why
//! [`EnclosureSurface::normal_cone`] returns `None` exactly when the cell's
//! tube-radius interval contains zero.
//!
//! Interval `sin`/`cos` come from [`crate::elementary`]: `inari` only ships
//! them behind its `gmp` feature, which this tree does not build. Those are
//! outward-rounded and already account for the interior extrema at `kπ/2`, so
//! this file writes `cos(uu)` / `sin(uu)` and never evaluates a trig function
//! only at interval endpoints — the historic under-estimation bug.
//!
//! Every method is closed form over the two independent angle intervals. The
//! tube radius `rho = R + r·cos v` is computed once and reused everywhere; on
//! a spindle it may straddle zero, which is legal and `inari` handles — no
//! absolute value is taken. The point enclosure is a box (sound, not tight);
//! the derivative enclosures are the exact interval image of the partials; the
//! normal cone and immersion bound arguments are stated where they are
//! defined.

use crate::elementary::{cos, sin};
use crate::enclosure::{Box3, DirCone, EnclosureSurface};
use inari::Interval;
use truck_base::cgmath64::{InnerSpace, Point3, Vector3};
use truck_geometry::specifieds::Torus;

/// A degenerate interval from a runtime `f64`. Finite coordinates always
/// construct; a NaN widens to the empty interval rather than panicking (H-1).
fn interval_at(x: f64) -> Interval {
    Interval::try_from((x, x)).unwrap_or(Interval::EMPTY)
}

/// The tube radius interval `R + r·cos v` over the cell. Every coordinate and
/// every partial is written in terms of it; computing it once is clearer and
/// tighter than inlining the expression three times. On a spindle torus the
/// interval may straddle zero, which is legal and `inari` handles — no
/// absolute value is taken here.
fn tube_radius(t: &Torus, vv: Interval) -> Interval {
    interval_at(t.large_radius()) + interval_at(t.small_radius()) * cos(vv)
}

/// Half-angle at which a cone covers the whole unit sphere: a full sweep in
/// either angle can reach any point of the sphere, so the honest answer is π
/// and a larger bound would be meaningless. This is the clamp applied to
/// `(wu + wv)/2` in [`EnclosureSurface::normal_cone`].
const FULL_SWEEP_HALF_ANGLE: f64 = core::f64::consts::PI;

impl EnclosureSurface for Torus {
    fn enclose(&self, uu: Interval, vv: Interval) -> Box3 {
        // S = c + (rho·cos u, rho·sin u, r·sin v), evaluated in interval
        // arithmetic. This is a box over the two independent angle intervals:
        // not tight (the true patch is curved), but sound, and soundness is
        // the contract.
        let rho = tube_radius(self, vv);
        let c = self.center();
        Box3 {
            x: interval_at(c.x) + rho * cos(uu),
            y: interval_at(c.y) + rho * sin(uu),
            z: interval_at(c.z) + interval_at(self.small_radius()) * sin(vv),
        }
    }

    fn enclose_der(&self, m: usize, n: usize, uu: Interval, vv: Interval) -> Box3 {
        // truck's `der_mn` is the ground truth this mirrors. The m-th
        // u-derivative of (cos u, sin u) is 4-periodic (its `u_part`), and the
        // n-th v-derivative of (R + r·cos v, r·sin v) is likewise (its
        // `v_part`); only the 0-th v-derivative carries R. The five cases the
        // packet names — (1,0), (0,1), (2,0), (1,1), (0,2) — are the low
        // quadrants of this table.
        let r = interval_at(self.small_radius());
        let (tx, ty) = match m % 4 {
            0 => (cos(uu), sin(uu)),
            1 => (-sin(uu), cos(uu)),
            2 => (-cos(uu), -sin(uu)),
            _ => (sin(uu), -cos(uu)),
        };
        let r0 = if n == 0 {
            interval_at(self.large_radius())
        } else {
            interval_at(0.0)
        };
        let (vx, vy) = match n % 4 {
            0 => (r0 + r * cos(vv), r * sin(vv)),
            1 => (-r * sin(vv), r * cos(vv)),
            2 => (-r * cos(vv), -r * sin(vv)),
            _ => (r * sin(vv), -r * cos(vv)),
        };
        // der_mn = c·δ₀₀ + (tx·vx, ty·vx, uz·vy) with uz = 1 iff m == 0; for
        // m ≥ 1 the z-partial is identically zero.
        let c = if m == 0 && n == 0 {
            self.center()
        } else {
            Point3::new(0.0, 0.0, 0.0)
        };
        Box3 {
            x: interval_at(c.x) + tx * vx,
            y: interval_at(c.y) + ty * vx,
            z: interval_at(c.z) + if m == 0 { vy } else { interval_at(0.0) },
        }
    }

    fn normal_cone(&self, uu: Interval, vv: Interval) -> Option<DirCone> {
        // The unit normal n(u, v) = (cos v·cos u, cos v·sin u, sin v) is the
        // point of the unit sphere at longitude u, latitude v; it does not
        // depend on R or r at all (truck's `normal` is already unit). It is
        // undefined on the singular circle rho = 0, so a cell that can touch
        // it has no cone.
        let rho = tube_radius(self, vv);
        if rho.contains(0.0) {
            return None;
        }
        // Angular distance from the midpoint normal is at most (wu + wv)/2: a
        // latitude step of Δv moves at most Δv of angle, and a longitude step
        // of Δu moves at most Δu (the factor is |cos v| ≤ 1), so every point
        // of the cell lies within (wu + wv)/2 of the axis. Sound, not tight —
        // tightness is a later packet's problem. A full sweep clamps at π,
        // which is the whole sphere.
        let wu = uu.sup() - uu.inf();
        let wv = vv.sup() - vv.inf();
        let half_angle = ((wu + wv) / 2.0).min(FULL_SWEEP_HALF_ANGLE);
        let (um, vm) = (uu.mid(), vv.mid());
        let axis = Vector3::new(vm.cos() * um.cos(), vm.cos() * um.sin(), vm.sin()).normalize();
        Some(DirCone { axis, half_angle })
    }

    fn immersion_lower_bound(&self, _uu: Interval, vv: Interval) -> f64 {
        // S_u ⊥ S_v with ‖S_u‖ = |R + r·cos v| and ‖S_v‖ = r, so
        // ‖S_u × S_v‖ = r·|rho| — a product, not a numerical minimisation.
        // `mig` is the smallest |rho| over the cell: 0 when rho contains zero,
        // else min(|rho.inf()|, |rho.sup()|). Multiplying in interval
        // arithmetic and taking `.inf()` rounds the product down, keeping the
        // returned value a true lower bound.
        let rho = tube_radius(self, vv);
        (interval_at(self.small_radius()) * interval_at(rho.mig())).inf()
    }
}

#[cfg(test)]
// Test-only allow: H-1 bans unwrap/expect on paths reachable from untrusted
// geometry. Unit-test assertions on hand-built witnesses are not such a path.
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::harness::{assert_converges, assert_encloses_surface};
    use core::f64::consts::PI;
    use inari::const_interval;
    use truck_geotrait::ParametricSurface;

    /// A degenerate interval from a runtime `f64` pair. The crate denies
    /// `unwrap`/`expect` even in tests, so a malformed interval degrades to
    /// EMPTY and fails the assertion that uses it rather than panicking.
    fn iv(lo: f64, hi: f64) -> Interval {
        Interval::try_from((lo, hi)).unwrap_or(Interval::EMPTY)
    }

    /// `DirCone` containment by angle: a unit direction `d` lies in a cone of
    /// half-angle `θ` iff `cos(angle(axis, d)) = axis·d ≥ cos θ` (both unit).
    /// The float slack is load-bearing at the `π` clamp, where `cos π = −1`
    /// and a rounding unit on `axis·d` must not read as escaping the whole
    /// sphere.
    fn cone_contains(cone: &DirCone, d: Vector3) -> bool {
        cone.axis.dot(d) >= cone.half_angle.cos() - 1.0e-12 // H-3: direction-cosine slack, dimensionless
    }

    #[test]
    fn torus_encloses_sampled_points() {
        let t = Torus::new(Point3::new(0.0, 0.0, 0.0), 3.0, 1.0);
        const SAMPLES: usize = 24;
        let cells = [
            (const_interval!(0.1, 0.7), const_interval!(0.2, 0.9)),
            (
                const_interval!(0.4, 1.2),
                const_interval!(0.9 * PI, 1.1 * PI),
            ),
            (const_interval!(0.0, 2.0 * PI), const_interval!(0.3, 0.8)),
            (
                const_interval!(0.0, 2.0 * PI),
                const_interval!(0.0, 2.0 * PI),
            ),
        ];
        for (uu, vv) in cells {
            assert_encloses_surface(&t, uu, vv, SAMPLES);
        }
    }

    #[test]
    fn torus_trig_extrema_inside_interval() {
        let t = Torus::new(Point3::new(0.0, 0.0, 0.0), 3.0, 1.0);
        let uu = const_interval!(0.0, 0.0);
        let vv = const_interval!(0.9 * PI, 1.1 * PI);
        let enclosure = t.enclose(uu, vv);
        // cos over [0.9π, 1.1π] attains −1 at the interior v = π, so the tube
        // radius interval dips to R − r = 2. Endpoint-only evaluation would
        // stop at 3 + cos(0.9π) ≈ 2.049 and miss the singular approach.
        let endpoint_only = 3.0 + (0.9 * PI).cos();
        assert!(
            enclosure.x.inf() <= 2.0 + 1.0e-12, // H-3: slack on an interval bound (a radius in R units), not a length
            "radial enclosure must reach R − r = 2, got {}",
            enclosure.x.inf()
        );
        assert!(
            enclosure.x.inf() >= 2.0 - 1.0e-12, // H-3: slack on an interval bound (a radius in R units), not a length
            "radial enclosure must not under-run R − r = 2, got {}",
            enclosure.x.inf()
        );
        assert!(
            enclosure.x.inf() < endpoint_only,
            "interior cos minimum must tighten the enclosure beyond endpoint-only evaluation"
        );
    }

    #[test]
    fn torus_enclosure_converges_under_bisection() {
        let t = Torus::new(Point3::new(0.0, 0.0, 0.0), 3.0, 1.0);
        let uu = const_interval!(0.2, 1.4);
        let vv = const_interval!(0.5, 2.2);
        let initial = t.enclose(uu, vv).width();
        assert_converges(&t, uu, vv, initial, 20);
    }

    #[test]
    fn torus_normal_cone_over_patch_and_full_sweep() {
        let t = Torus::new(Point3::new(0.0, 0.0, 0.0), 3.0, 1.0);

        // Small patch: axis is the midpoint normal and the half-angle is
        // exactly (wu + wv)/2 (no clamp).
        let uu = const_interval!(0.4, 1.0);
        let vv = const_interval!(0.6, 1.4);
        let cone = t
            .normal_cone(uu, vv)
            .expect("ordinary torus, no singular circle");
        let (um, vm) = (uu.mid(), vv.mid());
        let expected = Vector3::new(vm.cos() * um.cos(), vm.cos() * um.sin(), vm.sin()).normalize();
        assert!(
            (cone.axis - expected).magnitude() < 1.0e-12, // H-3: slack between two unit direction vectors, not a length
            "axis departed from the midpoint normal"
        );
        let half = (uu.sup() - uu.inf() + vv.sup() - vv.inf()) / 2.0;
        assert!(
            (cone.half_angle - half).abs() < 1.0e-12, // H-3: slack between two half-angles in radians, not a length
            "half-angle {} != (wu + wv)/2 = {}",
            cone.half_angle,
            half
        );

        // Full sweep in both angles: the half-angle clamps at π.
        let full = t
            .normal_cone(
                const_interval!(0.0, 2.0 * PI),
                const_interval!(0.0, 2.0 * PI),
            )
            .expect("ordinary torus, no singular circle");
        assert!(
            (full.half_angle - PI).abs() < 1.0e-12, // H-3: slack between two half-angles in radians, not a length
            "full sweep must clamp the half-angle at π, got {}",
            full.half_angle
        );

        // Containment by angle over a sampled grid of unit normals for both
        // cells. A half-angle of max(wu, wv)/2 would let a corner normal out;
        // the corner is sampled here.
        const N: usize = 30;
        let cells = [
            (uu, vv),
            (
                const_interval!(0.0, 2.0 * PI),
                const_interval!(0.0, 2.0 * PI),
            ),
        ];
        for (box_u, box_v) in cells {
            let cone = t.normal_cone(box_u, box_v).expect("ordinary torus");
            for i in 0..N {
                for j in 0..N {
                    let u = box_u.inf() + box_u.wid() * (i as f64) / (N as f64 - 1.0);
                    let v = box_v.inf() + box_v.wid() * (j as f64) / (N as f64 - 1.0);
                    let d = Vector3::new(v.cos() * u.cos(), v.cos() * u.sin(), v.sin());
                    assert!(
                        cone_contains(&cone, d),
                        "normal at ({u},{v}) escaped the cone (half-angle {})",
                        cone.half_angle
                    );
                }
            }
        }
    }

    #[test]
    fn torus_immersion_lower_bound_vanishes_on_a_spindle() {
        let t = Torus::new(Point3::new(0.0, 0.0, 0.0), 3.0, 1.0);

        // On an ordinary torus the bound is strictly positive, and it is a
        // genuine lower bound: every sampled ‖S_u × S_v‖ sits at or above it.
        const N: usize = 24;
        let cells = [
            (const_interval!(0.1, 1.3), const_interval!(0.3, 2.0)),
            (
                const_interval!(0.0, 2.0 * PI),
                const_interval!(0.0, 2.0 * PI),
            ),
        ];
        for (uu, vv) in cells {
            let lb = t.immersion_lower_bound(uu, vv);
            assert!(lb > 0.0, "ordinary torus bound must be positive, got {lb}");
            for i in 0..N {
                for j in 0..N {
                    let u = uu.inf() + uu.wid() * (i as f64) / (N as f64 - 1.0);
                    let v = vv.inf() + vv.wid() * (j as f64) / (N as f64 - 1.0);
                    let cross = t.uder(u, v).cross(t.vder(u, v));
                    assert!(
                        lb <= cross.magnitude() + 1.0e-12, // H-3: slack between two cross-product magnitudes, not a length
                        "lower bound {lb} exceeds ‖S_u × S_v‖ = {} at ({u},{v})",
                        cross.magnitude()
                    );
                }
            }
        }

        // Spindle torus (R = 1, r = 2): at the singular latitude
        // v* = acos(−R/r) the tube radius vanishes, so a cell containing v*
        // has immersion bound exactly 0 and no normal cone.
        let spindle = Torus::new(Point3::new(0.0, 0.0, 0.0), 1.0, 2.0);
        let vstar = (-0.5_f64).acos();
        let vv = iv(vstar - 0.1, vstar + 0.1);
        let lb = spindle.immersion_lower_bound(const_interval!(0.0, 1.0), vv);
        assert!(
            lb <= 1.0e-12, // H-3: slack on the immersion lower bound (units of r), not a length
            "spindle immersion bound must vanish, got {lb}"
        );
        assert!(lb >= 0.0, "immersion lower bound is never negative");
        assert!(spindle.normal_cone(const_interval!(0.0, 1.0), vv).is_none());
    }

    #[test]
    fn torus_der_enclosures_match_partials() {
        let t = Torus::new(Point3::new(0.0, 0.0, 0.0), 3.0, 1.0);
        let uu = const_interval!(0.2, 1.1);
        let vv = const_interval!(0.4, 1.6);
        const N: usize = 24;
        for (m, n) in [(1, 0), (0, 1), (2, 0), (1, 1), (0, 2)] {
            let box3 = t.enclose_der(m, n, uu, vv);
            for i in 0..N {
                for j in 0..N {
                    let u = uu.inf() + uu.wid() * (i as f64) / (N as f64 - 1.0);
                    let v = vv.inf() + vv.wid() * (j as f64) / (N as f64 - 1.0);
                    let der: Vector3 = t.der_mn(m, n, u, v);
                    assert!(
                        box3.contains(Point3::new(der.x, der.y, der.z)),
                        "der_mn({m},{n}) at ({u},{v}) = {der:?} escaped {box3:?}"
                    );
                }
            }
        }
    }
}
