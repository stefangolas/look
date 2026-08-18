//! The cone carrier (BG-CE-006-CYL-CONE).

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

impl Cone {
    /// Creates a cone, refusing a half angle that is not finite or that lies
    /// outside the open interval `(0, PI/2)` (H-1).
    #[inline(always)]
    pub fn new(apex: Point3, half_angle: f64) -> Outcome<Self> {
        if !half_angle.is_finite() || half_angle <= 0.0 || half_angle >= PI / 2.0 {
            return Err(Refusal::UnsupportedEnvelope(EnvelopeCase::ChartDegenerate));
        }
        Ok(Certified::new(
            Self { apex, half_angle },
            Certificate {
                props: PropMap::new(),
                // The cone is validated float arithmetic, never exact (H-6).
                method: Method::Float,
                budget_left: Budget::new(0, 0, 0),
                margin: Margin::UNBOUNDED,
                modulus: Modulus::Unbounded,
            },
        ))
    }
    /// Returns the apex
    #[inline(always)]
    pub const fn apex(&self) -> Point3 {
        self.apex
    }
    /// Returns the half angle
    #[inline(always)]
    pub const fn half_angle(&self) -> f64 {
        self.half_angle
    }
    /// Returns whether the point `pt` is on the cone
    #[inline(always)]
    pub fn include(&self, pt: Point3) -> bool {
        let r = pt - self.apex;
        let radial = Vector2::new(r.x, r.y).magnitude();
        radial.near(&(r.z * self.half_angle.tan()))
    }
}

impl ParametricSurface for Cone {
    type Point = Point3;
    type Vector = Vector3;
    #[inline(always)]
    fn der_mn(&self, m: usize, n: usize, u: f64, v: f64) -> Self::Vector {
        let (su, cu) = u.sin_cos();
        let apex = match (m, n) {
            (0, 0) => self.apex().to_vec(),
            _ => Vector3::zero(),
        };
        let u_part = match m % 4 {
            0 => Vector3::new(cu, su, 0.0),
            1 => Vector3::new(-su, cu, 0.0),
            2 => Vector3::new(-cu, -su, 0.0),
            _ => Vector3::new(su, -cu, 0.0),
        };
        let slope = self.half_angle().tan();
        let radial_amp = match n {
            0 => v * slope,
            1 => slope,
            _ => 0.0,
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
        apex + radial_amp * u_part + z
    }
    #[inline(always)]
    fn subs(&self, u: f64, v: f64) -> Point3 {
        let slope = self.half_angle().tan();
        self.apex()
            + v * slope * Vector3::new(f64::cos(u), f64::sin(u), 0.0)
            + Vector3::new(0.0, 0.0, v)
    }
    #[inline(always)]
    fn uder(&self, u: f64, v: f64) -> Vector3 {
        self.half_angle().tan() * v * Vector3::new(-f64::sin(u), f64::cos(u), 0.0)
    }
    #[inline(always)]
    fn vder(&self, u: f64, _v: f64) -> Vector3 {
        self.half_angle().tan() * Vector3::new(f64::cos(u), f64::sin(u), 0.0)
            + Vector3::new(0.0, 0.0, 1.0)
    }
    #[inline(always)]
    fn uuder(&self, u: f64, v: f64) -> Vector3 {
        self.half_angle().tan() * v * Vector3::new(-f64::cos(u), -f64::sin(u), 0.0)
    }
    #[inline(always)]
    fn uvder(&self, u: f64, _v: f64) -> Vector3 {
        self.half_angle().tan() * Vector3::new(-f64::sin(u), f64::cos(u), 0.0)
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

impl ParametricSurface3D for Cone {
    #[inline(always)]
    fn normal(&self, u: f64, v: f64) -> Vector3 {
        if v == 0.0 {
            return Vector3::zero();
        }
        let slope = self.half_angle().tan();
        let unit = Vector3::new(f64::cos(u), f64::sin(u), -slope) / (1.0 + slope * slope).sqrt();
        if v > 0.0 {
            unit
        } else {
            -unit
        }
    }
}

impl IncludeCurve<BSplineCurve<Point3>> for Cone {
    #[inline(always)]
    fn include(&self, curve: &BSplineCurve<Point3>) -> Outcome<bool> {
        // BG-TOL-001: model-space radial deviation, compared at the model scale.
        let ctx = ToleranceCtx::unscaled_legacy();
        let r = curve.front() - self.apex();
        let radial = Vector2::new(r.x, r.y).magnitude();
        Ok(Certified::new(
            curve.is_const() && ctx.is_small_len(radial - r.z * self.half_angle().tan()),
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

impl IncludeCurve<NurbsCurve<Vector4>> for Cone {
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

impl ParameterDivision2D for Cone {
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
        // cone smaller than the permitted chord deviation simply cannot be
        // subdivided any further. Panicking on that took down the entire
        // tessellation of otherwise valid CAD assemblies.
        //
        // Clamping the ratio also keeps `acos` inside its domain, which is
        // what the assertion was really protecting: past a ratio of two the
        // argument falls below -1 and the subdivision would be NaN. At a ratio
        // of one `delta` is already pi, the coarsest subdivision there is, so
        // nothing above that can mesh any differently.
        //
        // The cone's cross-section radius varies with `v`, so the ratio is
        // taken at the widest cross-section in the requested range — the
        // conservative choice, since the coarsest end drives the chord error.
        let radial = f64::max(vrange.0.abs(), vrange.1.abs()) * self.half_angle().tan();
        let ratio = f64::min(tol / radial, 1.0);
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

impl SearchParameter<D2> for Cone {
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
        let r = point - self.apex();
        let v = r.z;
        let rxy = Vector2::new(r.x, r.y);
        let radial = rxy.magnitude();
        if !ctx.is_small_len(radial - v * self.half_angle().tan()) {
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
        Some((u, v))
    }
}

impl SearchNearestParameter<D2> for Cone {
    type Point = Point3;
    #[inline(always)]
    fn search_nearest_parameter<H: Into<SPHint2D>>(
        &self,
        point: Point3,
        hint: H,
        _: usize,
    ) -> Option<(f64, f64)> {
        let r = point - self.apex();
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
        let slope = self.half_angle().tan();
        let v = (slope * radial + r.z) / (1.0 + slope * slope);
        Some((u, v))
    }
}
