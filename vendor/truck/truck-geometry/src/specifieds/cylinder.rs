//! The cylinder carrier (BG-CE-006-CYL-CONE).

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

use super::*;
use std::f64::consts::PI;
use truck_base::evidence::{
    Budget, Certificate, Certified, EnvelopeCase, Margin, Method, Modulus, Outcome, PropMap,
    Refusal,
};

impl Cylinder {
    /// Creates a cylinder, refusing a non-positive or non-finite radius (H-1).
    #[inline(always)]
    pub fn new(center: Point3, radius: f64) -> Outcome<Self> {
        if !radius.is_finite() || radius <= 0.0 {
            return Err(Refusal::UnsupportedEnvelope(EnvelopeCase::ChartDegenerate));
        }
        Ok(Certified::new(
            Self { center, radius },
            Certificate {
                props: PropMap::new(),
                // The cylinder is validated float arithmetic, never exact (H-6).
                method: Method::Float,
                budget_left: Budget::new(0, 0, 0),
                margin: Margin::UNBOUNDED,
                modulus: Modulus::Unbounded,
            },
        ))
    }
    /// Returns the center
    #[inline(always)]
    pub const fn center(&self) -> Point3 {
        self.center
    }
    /// Returns the radius
    #[inline(always)]
    pub const fn radius(&self) -> f64 {
        self.radius
    }
    /// Returns whether the point `pt` is on the cylinder
    #[inline(always)]
    pub fn include(&self, pt: Point3) -> bool {
        let r = pt - self.center;
        Vector2::new(r.x, r.y).magnitude().near(&self.radius)
    }
}

impl ParametricSurface for Cylinder {
    type Point = Point3;
    type Vector = Vector3;
    #[inline(always)]
    fn der_mn(&self, m: usize, n: usize, u: f64, v: f64) -> Self::Vector {
        let (su, cu) = u.sin_cos();
        let center = match (m, n) {
            (0, 0) => self.center().to_vec(),
            _ => Vector3::zero(),
        };
        let u_part = match m % 4 {
            0 => Vector3::new(cu, su, 0.0),
            1 => Vector3::new(-su, cu, 0.0),
            2 => Vector3::new(-cu, -su, 0.0),
            _ => Vector3::new(su, -cu, 0.0),
        };
        let radial = if n == 0 {
            self.radius() * u_part
        } else {
            Vector3::zero()
        };
        let z_part = match n {
            0 => v,
            1 => 1.0,
            _ => 0.0,
        };
        let z = if m == 0 {
            Vector3::new(0.0, 0.0, z_part)
        } else {
            Vector3::zero()
        };
        center + radial + z
    }
    #[inline(always)]
    fn subs(&self, u: f64, v: f64) -> Point3 {
        self.center()
            + self.radius() * Vector3::new(f64::cos(u), f64::sin(u), 0.0)
            + Vector3::new(0.0, 0.0, v)
    }
    #[inline(always)]
    fn uder(&self, u: f64, _v: f64) -> Vector3 {
        self.radius() * Vector3::new(-f64::sin(u), f64::cos(u), 0.0)
    }
    #[inline(always)]
    fn vder(&self, _u: f64, _v: f64) -> Vector3 {
        Vector3::new(0.0, 0.0, 1.0)
    }
    #[inline(always)]
    fn uuder(&self, u: f64, _v: f64) -> Vector3 {
        self.radius() * Vector3::new(-f64::cos(u), -f64::sin(u), 0.0)
    }
    #[inline(always)]
    fn uvder(&self, _u: f64, _v: f64) -> Vector3 {
        Vector3::zero()
    }
    #[inline(always)]
    fn vvder(&self, _u: f64, _v: f64) -> Vector3 {
        Vector3::zero()
    }
    #[inline(always)]
    fn parameter_range(&self) -> (ParameterRange, ParameterRange) {
        const URANGE: (Bound<f64>, Bound<f64>) = (Bound::Included(0.0), Bound::Excluded(2.0 * PI));
        (URANGE, (Bound::Unbounded, Bound::Unbounded))
    }
    #[inline(always)]
    fn u_period(&self) -> Option<f64> {
        Some(2.0 * PI)
    }
}

impl ParametricSurface3D for Cylinder {
    #[inline(always)]
    fn normal(&self, u: f64, _v: f64) -> Vector3 {
        Vector3::new(f64::cos(u), f64::sin(u), 0.0)
    }
}

impl IncludeCurve<BSplineCurve<Point3>> for Cylinder {
    #[inline(always)]
    fn include(&self, curve: &BSplineCurve<Point3>) -> Outcome<bool> {
        // BG-TOL-001: model-space radial deviation, compared at the model scale.
        let ctx = ToleranceCtx::unscaled_legacy();
        let radial = {
            let r = curve.front() - self.center();
            Vector2::new(r.x, r.y).magnitude()
        };
        Ok(Certified::new(
            curve.is_const() && ctx.is_small_len(radial - self.radius()),
            Certificate {
                props: PropMap::new(),
                method: Method::Float,
                budget_left: Budget::new(0, 0, 0),
                margin: Margin::UNBOUNDED,
                modulus: Modulus::Unbounded,
            },
        ))
    }
}

impl IncludeCurve<NurbsCurve<Vector4>> for Cylinder {
    fn include(&self, curve: &NurbsCurve<Vector4>) -> Outcome<bool> {
        let (knots, _) = curve.knot_vec().to_single_multi();
        let degree = curve.degree() * 2;
        let value = knots
            .windows(2)
            .flat_map(move |window| (1..degree).map(move |i| (window, i)))
            .all(move |(window, i)| {
                let t = i as f64 / degree as f64;
                let t = match window {
                    [t0, t1] => t0 * (1.0 - t) + t1 * t,
                    _ => unreachable!("windows(2) yields two-element slices"),
                };
                self.include(curve.subs(t))
            });
        Ok(Certified::new(
            value,
            Certificate {
                props: PropMap::new(),
                method: Method::Float,
                budget_left: Budget::new(0, 0, 0),
                margin: Margin::UNBOUNDED,
                modulus: Modulus::Unbounded,
            },
        ))
    }
}

impl ParameterDivision2D for Cylinder {
    #[inline(always)]
    fn parameter_division(
        &self,
        (urange, vrange): ((f64, f64), (f64, f64)),
        tol: f64,
    ) -> (Vec<f64>, Vec<f64>) {
        let tol = tol.max(TOLERANCE);
        nonpositive_tolerance!(tol);
        // A tolerance coarser than the surface is a meaningful request rather
        // than a caller error: a tolerance derived from the extent of a whole
        // model is routinely larger than the smallest features in it, and a
        // cylinder smaller than the permitted chord deviation simply cannot be
        // subdivided any further. Panicking on that took down the entire
        // tessellation of otherwise valid CAD assemblies.
        //
        // Clamping the ratio also keeps `acos` inside its domain, which is
        // what the assertion was really protecting: past a ratio of two the
        // argument falls below -1 and the subdivision would be NaN. At a ratio
        // of one `delta` is already pi, the coarsest subdivision there is, so
        // nothing above that can mesh any differently.
        let ratio = f64::min(tol / self.radius(), 1.0);
        let delta = 2.0 * f64::acos(1.0 - ratio);
        let u_div = 1 + ((urange.1 - urange.0) / delta).floor() as usize;
        (
            (0..=u_div)
                .map(|i| urange.0 + (urange.1 - urange.0) * i as f64 / u_div as f64)
                .collect(),
            vec![vrange.0, vrange.1],
        )
    }
}

impl SearchParameter<D2> for Cylinder {
    type Point = Point3;
    #[inline(always)]
    fn search_parameter<H: Into<SPHint2D>>(
        &self,
        point: Point3,
        hint: H,
        _: usize,
    ) -> Option<(f64, f64)> {
        // BG-TOL-001: model-space radial deviation, compared at the model scale.
        let ctx = ToleranceCtx::unscaled_legacy();
        let r = point - self.center();
        let rxy = Vector2::new(r.x, r.y);
        let radial = rxy.magnitude();
        if !ctx.is_small_len(radial - self.radius()) {
            return None;
        }
        let u = if ctx.is_small_len(radial) {
            match hint.into() {
                SPHint2D::Parameter(u, _) => u,
                _ => 0.0,
            }
        } else {
            let rxy_n = rxy / radial;
            let u0 = f64::acos(f64::clamp(rxy_n.x, -1.0, 1.0));
            if rxy_n.y < 0.0 {
                2.0 * PI - u0
            } else {
                u0
            }
        };
        Some((u, r.z))
    }
}

impl SearchNearestParameter<D2> for Cylinder {
    type Point = Point3;
    #[inline(always)]
    fn search_nearest_parameter<H: Into<SPHint2D>>(
        &self,
        point: Point3,
        hint: H,
        _: usize,
    ) -> Option<(f64, f64)> {
        let r = point - self.center();
        let rxy = Vector2::new(r.x, r.y);
        let radial = rxy.magnitude();
        let u = if radial == 0.0 {
            match hint.into() {
                SPHint2D::Parameter(u, _) => u,
                _ => 0.0,
            }
        } else {
            let rxy_n = rxy / radial;
            let u0 = f64::acos(f64::clamp(rxy_n.x, -1.0, 1.0));
            if rxy_n.y < 0.0 {
                2.0 * PI - u0
            } else {
                u0
            }
        };
        Some((u, r.z))
    }
}
