//! BG-SOL-S6-IMPLICIT — certified interval evaluation of the canonical
//! carriers' implicit functions.
//!
//! The Contact Layer's general validated FF stage (offset mixed quadrics,
//! later `Torus` and `Placed`) has no closed-form cell in the §3.3 analytic
//! table, and every certified formulation of that stage — event finding,
//! Krawczyk arc continuation, singular-cell detection — needs the same
//! primitive first: interval evaluation of each canonical carrier's implicit
//! function `f(p)` and its gradient `∇f` on a `Box3`, with a documented sign
//! convention and a regularity predicate. This module builds exactly that
//! primitive and nothing else: no solver logic, no event finding, no
//! certificates, no `Method` — those belong to the consumers.
//!
//! Every evaluation is plain sound interval arithmetic (BG-ENC-001): the true
//! `f` value of EVERY point in the box lies inside the returned interval.
//! Under-estimation is a silent-wrong-answer bug.
//!
//! House rules H-1..H-8 apply.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

use crate::enclosure::interval_at;
use crate::enclosure::{Box3, Interval};
use truck_geometry::specifieds::{Cone, Cylinder, Plane, Sphere, Torus};

/// Certified interval evaluation of a canonical carrier's implicit function.
///
/// The contact set of a carrier is `{ p : f(p) = 0 }`; the sign convention is
/// documented per implementing arm. Evaluations are sound interval enclosures:
/// the true f value of EVERY point in the box lies inside the returned
/// interval. This trait is substrate for the general validated FF stage
/// (event finding, Krawczyk continuation); it decides nothing about contact
/// by itself.
///
/// The impls cover the five bare canonical carriers (`Plane`, `Sphere`,
/// `Cylinder`, `Cone`, `Torus`) only. `CanonicalSurface` and `Placed` carriers
/// are deliberately omitted: the dispatcher refuses `Placed` upstream, and the
/// general-validated-FF stage matches the enum itself.
pub trait ImplicitField {
    /// Sound interval enclosure of f over the box.
    fn implicit(&self, p: &Box3) -> Interval;
    /// Sound interval enclosure of ∇f over the box, component order (x, y, z).
    fn grad(&self, p: &Box3) -> [Interval; 3];
    /// Proves ∇f ≠ 0 somewhere in every direction test: true iff SOME
    /// component's gradient enclosure excludes zero. `false` means "not
    /// PROVEN regular here", never "proven singular".
    fn regular_on(&self, p: &Box3) -> bool;
}

/// Whether the interval lies strictly away from zero.
fn excludes_zero(i: Interval) -> bool {
    i.inf() > 0.0 || i.sup() < 0.0
}

impl ImplicitField for Plane {
    /// `f = n · (p − o)` with `o = origin()`, unit normal `n = normal()`:
    /// positive on the side `n` points to. The gradient is the constant normal.
    fn implicit(&self, p: &Box3) -> Interval {
        let n = self.normal();
        let o = self.origin();
        let dx = p.x - interval_at(o.x);
        let dy = p.y - interval_at(o.y);
        let dz = p.z - interval_at(o.z);
        interval_at(n.x) * dx + interval_at(n.y) * dy + interval_at(n.z) * dz
    }

    fn grad(&self, _p: &Box3) -> [Interval; 3] {
        let n = self.normal();
        [interval_at(n.x), interval_at(n.y), interval_at(n.z)]
    }

    /// The normal is unit, so some component is always nonzero: regular
    /// everywhere.
    fn regular_on(&self, _p: &Box3) -> bool {
        let n = self.normal();
        excludes_zero(interval_at(n.x))
            || excludes_zero(interval_at(n.y))
            || excludes_zero(interval_at(n.z))
    }
}

impl ImplicitField for Sphere {
    /// `f = |p−c|² − r²`: negative inside the ball.
    fn implicit(&self, p: &Box3) -> Interval {
        let c = self.center();
        let dx = p.x - interval_at(c.x);
        let dy = p.y - interval_at(c.y);
        let dz = p.z - interval_at(c.z);
        dx.sqr() + dy.sqr() + dz.sqr() - interval_at(self.radius()).sqr()
    }

    /// `∇f = 2·(p−c)`.
    fn grad(&self, p: &Box3) -> [Interval; 3] {
        let c = self.center();
        let two = interval_at(2.0);
        [
            two * (p.x - interval_at(c.x)),
            two * (p.y - interval_at(c.y)),
            two * (p.z - interval_at(c.z)),
        ]
    }

    fn regular_on(&self, p: &Box3) -> bool {
        self.grad(p).iter().any(|g| excludes_zero(*g))
    }
}

impl ImplicitField for Cylinder {
    /// `f = (x−cx)² + (y−cy)² − r²`: negative inside the wall. Note `cz` does
    /// NOT enter the form — the cylinder is a z-axis surface, so the signed
    /// axial offset cannot move a point off the wall.
    fn implicit(&self, p: &Box3) -> Interval {
        let c = self.center();
        let dx = p.x - interval_at(c.x);
        let dy = p.y - interval_at(c.y);
        dx.sqr() + dy.sqr() - interval_at(self.radius()).sqr()
    }

    /// `∇f = (2(x−cx), 2(y−cy), 0)`: the axial component is identically zero.
    fn grad(&self, p: &Box3) -> [Interval; 3] {
        let c = self.center();
        let two = interval_at(2.0);
        [
            two * (p.x - interval_at(c.x)),
            two * (p.y - interval_at(c.y)),
            interval_at(0.0),
        ]
    }

    fn regular_on(&self, p: &Box3) -> bool {
        self.grad(p).iter().any(|g| excludes_zero(*g))
    }
}

impl ImplicitField for Cone {
    /// `f = x'² + y'² − (z'·t)²` with `t = half_angle().tan()`: the double
    /// cone about the z axis through the apex. The apex IS on the zero set
    /// (`f(a) = 0`) and ∇f = 0 there, so `regular_on` returns false near it.
    fn implicit(&self, p: &Box3) -> Interval {
        let a = self.apex();
        let t = interval_at(self.half_angle().tan());
        let dx = p.x - interval_at(a.x);
        let dy = p.y - interval_at(a.y);
        let dz = p.z - interval_at(a.z);
        dx.sqr() + dy.sqr() - (dz * t).sqr()
    }

    /// `∇f = (2x', 2y', −2z'·t²)`.
    fn grad(&self, p: &Box3) -> [Interval; 3] {
        let a = self.apex();
        let t = self.half_angle().tan();
        let two = interval_at(2.0);
        let t2 = interval_at(t) * interval_at(t);
        let dx = p.x - interval_at(a.x);
        let dy = p.y - interval_at(a.y);
        let dz = p.z - interval_at(a.z);
        [two * dx, two * dy, -two * t2 * dz]
    }

    fn regular_on(&self, p: &Box3) -> bool {
        self.grad(p).iter().any(|g| excludes_zero(*g))
    }
}

impl ImplicitField for Torus {
    /// The sqrt-free quartic form: with `g = |p−c|² + R² − r²` and
    /// `h = x'² + y'²`, `f = g² − 4R²h` (all of `|p−c|²`, `R`, `r` positive).
    fn implicit(&self, p: &Box3) -> Interval {
        let c = self.center();
        let dx = p.x - interval_at(c.x);
        let dy = p.y - interval_at(c.y);
        let dz = p.z - interval_at(c.z);
        let r2 = interval_at(self.large_radius()).sqr();
        let g = dx.sqr() + dy.sqr() + dz.sqr() + r2 - interval_at(self.small_radius()).sqr();
        let h = dx.sqr() + dy.sqr();
        g.sqr() - interval_at(4.0) * r2 * h
    }

    /// `∇f = 2g·∇g − 4R²·∇h` with `∇g = 2(x', y', z')`, `∇h = (2x', 2y', 0)`.
    fn grad(&self, p: &Box3) -> [Interval; 3] {
        let c = self.center();
        let two = interval_at(2.0);
        let dx = p.x - interval_at(c.x);
        let dy = p.y - interval_at(c.y);
        let dz = p.z - interval_at(c.z);
        let r2 = interval_at(self.large_radius()).sqr();
        let g = dx.sqr() + dy.sqr() + dz.sqr() + r2 - interval_at(self.small_radius()).sqr();
        let scale = interval_at(4.0) * r2;
        [
            two * g * (two * dx) - scale * (two * dx),
            two * g * (two * dy) - scale * (two * dy),
            two * g * (two * dz) - scale * interval_at(0.0),
        ]
    }

    fn regular_on(&self, p: &Box3) -> bool {
        self.grad(p).iter().any(|g| excludes_zero(*g))
    }
}

#[cfg(test)]
mod tests {
    // Test-only: H-1 bans unwrap/expect on paths reachable from untrusted
    // geometry. Every carrier below is constructed by hand (or via the matched
    // `Outcome` constructors) so unwrap is never needed; the packet re-asserts
    // the deny here rather than allowing it.
    #![deny(clippy::unwrap_used)]
    use super::*;
    use std::f64::consts::{FRAC_1_SQRT_2, FRAC_PI_4};
    use truck_base::cgmath64::{EuclideanSpace, Point3};

    /// Float slack on unit-scale witness coordinates and residuals —
    /// dimensionless in every use, never a model-space length.
    const SLACK: f64 = 1.0e-9; // H-3: float slack on unit-scale witness values, not a length

    /// Central-difference step (a parameter offset, not a length).
    const FD_H: f64 = 1.0e-3; // H-3: central-difference step, a parameter offset not a length

    /// Generous containment slack for the central-difference comparison: the
    /// truncation error of a step-`FD_H` central difference on the quartic
    /// torus form is well under this at the probed scale.
    const FD_SLACK: f64 = 1.0e-4; // H-3: central-difference truncation slack on unit-scale values, not a length

    /// Whether the interval is the degenerate `[0, 0]`.
    fn decisively_zero(i: Interval) -> bool {
        i.inf() == 0.0 && i.sup() == 0.0
    }

    /// Build a test interval, degrading to EMPTY (and failing its assertion)
    /// rather than panicking on a malformed bound.
    fn iv(lo: f64, hi: f64) -> Interval {
        Interval::try_from((lo, hi)).unwrap_or(Interval::EMPTY)
    }

    /// A validated cylinder, matching the `Outcome` constructor's shape.
    fn cylinder(center: Point3, radius: f64) -> Cylinder {
        match Cylinder::new(center, radius) {
            Ok(certified) => certified.value,
            Err(_) => unreachable!("a positive finite radius is always a valid cylinder"),
        }
    }

    /// A validated cone, matching the `Outcome` constructor's shape.
    fn cone(apex: Point3, half_angle: f64) -> Cone {
        match Cone::new(apex, half_angle) {
            Ok(certified) => certified.value,
            Err(_) => unreachable!("a half angle in (0, PI/2) is always a valid cone"),
        }
    }

    /// The scalar enclosure's central difference along `axis` at `p`.
    fn central_diff(f: &impl Fn(Point3) -> Interval, p: Point3, axis: usize, h: f64) -> Interval {
        let mut lo = p;
        let mut hi = p;
        match axis {
            0 => {
                lo.x -= h;
                hi.x += h;
            }
            1 => {
                lo.y -= h;
                hi.y += h;
            }
            _ => {
                lo.z -= h;
                hi.z += h;
            }
        }
        (f(hi) - f(lo)) / interval_at(2.0 * h)
    }

    /// The analytic gradient at a degenerate box must agree with the central
    /// difference of the scalar enclosure, per axis, within `FD_SLACK`.
    fn fd_match<C: ImplicitField>(
        f: &impl Fn(Point3) -> Interval,
        carrier: &C,
        p: Point3,
        label: &str,
    ) {
        let analytic = carrier.grad(&Box3::point(p));
        for (i, a) in analytic.iter().enumerate() {
            let numeric = central_diff(f, p, i, FD_H);
            let diff = *a - numeric;
            assert!(
                diff.inf() >= -FD_SLACK && diff.sup() <= FD_SLACK,
                "{label} axis {i}: analytic {a:?} vs numeric {numeric:?}"
            );
        }
    }

    #[test]
    fn implicit_zero_on_surface_witnesses() {
        // Sphere r=1 at the origin: (0,0,1) is on the surface.
        let sphere = Sphere::new(Point3::origin(), 1.0);
        let enc = sphere.implicit(&Box3::point(Point3::new(0.0, 0.0, 1.0)));
        assert!(decisively_zero(enc), "sphere at (0,0,1): {enc:?}");

        // Cylinder r=1 about the z-axis through the origin: (1,0,5) is on the
        // wall, with the axial coordinate arbitrary.
        let cyl = cylinder(Point3::origin(), 1.0);
        let enc = cyl.implicit(&Box3::point(Point3::new(1.0, 0.0, 5.0)));
        assert!(decisively_zero(enc), "cylinder at (1,0,5): {enc:?}");

        // Cone apex at the origin, half angle PI/4. The named witnesses
        // (1/sqrt2, 1/sqrt2, 1) and (0, 1, 1) sit on the *mathematical* cone,
        // but tan(PI/4) rounds below 1.0 in f64, so the exact f64 carrier's
        // zero set is slightly narrower and those points evaluate to a tiny
        // positive enclosure rather than one containing 0 (see RESULT.json
        // disagreements). Assert they land within rounding slack of zero.
        let cone = cone(Point3::origin(), FRAC_PI_4);
        let enc = cone.implicit(&Box3::point(Point3::new(FRAC_1_SQRT_2, FRAC_1_SQRT_2, 1.0)));
        assert!(
            enc.inf() >= -SLACK && enc.sup() <= SLACK,
            "cone at (1/sqrt2, 1/sqrt2, 1) within rounding of zero: {enc:?}"
        );
        // The apex is the exact rational zero.
        let enc = cone.implicit(&Box3::point(Point3::origin()));
        assert!(decisively_zero(enc), "cone apex: {enc:?}");
        // The rational witness (0,1,1) is likewise within rounding of zero.
        let enc = cone.implicit(&Box3::point(Point3::new(0.0, 1.0, 1.0)));
        assert!(
            enc.inf() >= -SLACK && enc.sup() <= SLACK,
            "cone at (0,1,1) within rounding of zero: {enc:?}"
        );
        // An exact on-surface witness (x', y', z') = (t, 0, 1) with
        // t = tan(PI/4) satisfies x'² + y'² = (z'·t)² in f64 arithmetic, so
        // its enclosure genuinely contains 0.
        let t = FRAC_PI_4.tan();
        let enc = cone.implicit(&Box3::point(Point3::new(t, 0.0, 1.0)));
        assert!(enc.contains(0.0), "cone at (tan(PI/4), 0, 1): {enc:?}");

        // Torus R=2 r=0.5: all three witnesses are exact zeros.
        let torus = Torus::new(Point3::origin(), 2.0, 0.5);
        for p in [
            Point3::new(2.5, 0.0, 0.0),
            Point3::new(2.0, 0.0, 0.5),
            Point3::new(1.5, 0.0, 0.0),
        ] {
            let enc = torus.implicit(&Box3::point(p));
            assert!(decisively_zero(enc), "torus at {p:?}: {enc:?}");
        }

        // Plane through the origin with normal +z: any (x,y,0) point is on it.
        let plane = Plane::xy();
        let enc = plane.implicit(&Box3::point(Point3::new(1.25, -0.5, 0.0)));
        assert!(decisively_zero(enc), "plane at (1.25,-0.5,0): {enc:?}");
    }

    #[test]
    fn implicit_sign_away_from_surface() {
        // Sphere: the origin is strictly inside, (2,0,0) is outside with f = 3.
        let sphere = Sphere::new(Point3::origin(), 1.0);
        let enc = sphere.implicit(&Box3::point(Point3::origin()));
        assert!(enc.sup() < 0.0, "sphere interior at origin: {enc:?}");
        let enc = sphere.implicit(&Box3::point(Point3::new(2.0, 0.0, 0.0)));
        assert!(enc.contains(3.0), "sphere at (2,0,0): {enc:?}");

        // Cylinder: the axis region is strictly inside, (2,0,0) has f = 3.
        let cyl = cylinder(Point3::origin(), 1.0);
        let enc = cyl.implicit(&Box3::point(Point3::origin()));
        assert!(enc.sup() < 0.0, "cylinder interior at origin: {enc:?}");
        let enc = cyl.implicit(&Box3::point(Point3::new(2.0, 0.0, 0.0)));
        assert!(enc.contains(3.0), "cylinder at (2,0,0): {enc:?}");

        // Cone: (0,0,1) is inside the solid angle (f = −t² < 0); the apex box
        // contains 0.
        let cone = cone(Point3::origin(), FRAC_PI_4);
        let enc = cone.implicit(&Box3::point(Point3::new(0.0, 0.0, 1.0)));
        assert!(
            enc.sup() < 0.0,
            "cone inside the solid angle at (0,0,1): {enc:?}"
        );
        let enc = cone.implicit(&Box3::point(Point3::origin()));
        assert!(enc.contains(0.0), "cone apex box: {enc:?}");

        // Torus R=2 r=0.5: the center is outside the wall (f = 14.0625 > 0).
        let torus = Torus::new(Point3::origin(), 2.0, 0.5);
        let enc = torus.implicit(&Box3::point(Point3::origin()));
        assert!(enc.contains(14.0625), "torus at the center: {enc:?}");
    }

    #[test]
    fn grad_matches_finite_difference() {
        // Exact component checks where the gradient is trivial.
        let sphere = Sphere::new(Point3::origin(), 1.0);
        let [gx, gy, gz] = sphere.grad(&Box3::point(Point3::new(1.0, 0.0, 0.0)));
        assert!(gx.contains(2.0), "sphere grad x at (1,0,0): {gx:?}");
        assert!(
            decisively_zero(gy) && decisively_zero(gz),
            "sphere grad yz at (1,0,0): {gy:?},{gz:?}"
        );
        let cyl = cylinder(Point3::origin(), 1.0);
        let [gx, gy, gz] = cyl.grad(&Box3::point(Point3::new(2.0, 0.0, 3.0)));
        assert!(gx.contains(4.0), "cylinder grad x at (2,0,3): {gx:?}");
        assert!(decisively_zero(gy), "cylinder grad y at (2,0,3): {gy:?}");
        assert!(decisively_zero(gz), "cylinder grad z at (2,0,3): {gz:?}");

        // Central differences of the scalar enclosure at nondegenerate points.
        let sphere = Sphere::new(Point3::origin(), 1.0);
        fd_match(
            &|q| sphere.implicit(&Box3::point(q)),
            &sphere,
            Point3::new(1.0, 0.5, 0.25),
            "sphere",
        );
        let cyl = cylinder(Point3::origin(), 1.0);
        fd_match(
            &|q| cyl.implicit(&Box3::point(q)),
            &cyl,
            Point3::new(1.0, 0.5, 3.0),
            "cylinder",
        );
        let cone = cone(Point3::origin(), FRAC_PI_4);
        fd_match(
            &|q| cone.implicit(&Box3::point(q)),
            &cone,
            Point3::new(0.5, 0.25, 1.0),
            "cone",
        );
        let torus = Torus::new(Point3::origin(), 2.0, 0.5);
        fd_match(
            &|q| torus.implicit(&Box3::point(q)),
            &torus,
            Point3::new(2.0, 1.0, 0.0),
            "torus",
        );
        let plane = Plane::xy();
        fd_match(
            &|q| plane.implicit(&Box3::point(q)),
            &plane,
            Point3::new(1.0, 2.0, 3.0),
            "plane",
        );
    }

    #[test]
    fn regular_on_detects_cone_apex() {
        let cone = cone(Point3::origin(), FRAC_PI_4);
        assert!(
            !cone.regular_on(&Box3::point(Point3::origin())),
            "the cone apex is singular (∇f = 0 there)"
        );
        assert!(
            cone.regular_on(&Box3::point(Point3::new(1.0, 0.0, 1.0))),
            "off the apex the cone gradient excludes zero"
        );

        let cyl = cylinder(Point3::origin(), 1.0);
        assert!(
            cyl.regular_on(&Box3::point(Point3::new(1.0, 0.0, 3.0))),
            "off-axis the cylinder x-component excludes zero"
        );
        assert!(
            !cyl.regular_on(&Box3::point(Point3::new(0.0, 0.0, 3.0))),
            "on the axis both gradient components straddle zero"
        );
    }

    #[test]
    fn implicit_soundness_on_boxes() {
        // A non-degenerate box: every sampled point's exact f must lie inside
        // the enclosure (soundness over the whole box, not just collapsed
        // points).
        let box3 = Box3 {
            x: iv(0.9, 1.1),
            y: iv(-0.05, 0.05),
            z: iv(0.95, 1.05),
        };
        let points = [
            Point3::new(1.0, 0.0, 1.0),
            Point3::new(0.9, -0.05, 0.95),
            Point3::new(0.9, 0.05, 0.95),
            Point3::new(1.1, -0.05, 0.95),
            Point3::new(1.1, 0.05, 0.95),
            Point3::new(0.9, -0.05, 1.05),
            Point3::new(0.9, 0.05, 1.05),
            Point3::new(1.1, -0.05, 1.05),
            Point3::new(1.1, 0.05, 1.05),
        ];

        // Sphere unit at the origin over the box.
        let sphere = Sphere::new(Point3::origin(), 1.0);
        let enc = sphere.implicit(&box3);
        for p in points {
            let f = p.x * p.x + p.y * p.y + p.z * p.z - 1.0;
            assert!(
                enc.contains(f),
                "sphere soundness at {p:?}: f={f} not in {enc:?}"
            );
        }

        // Cylinder r=1 about the z-axis, off-axis over the same box.
        let cyl = cylinder(Point3::origin(), 1.0);
        let enc = cyl.implicit(&box3);
        for p in points {
            let f = p.x * p.x + p.y * p.y - 1.0;
            assert!(
                enc.contains(f),
                "cylinder soundness at {p:?}: f={f} not in {enc:?}"
            );
        }
    }
}
