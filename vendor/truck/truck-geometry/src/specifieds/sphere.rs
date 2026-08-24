use super::*;
use std::f64::consts::PI;
use truck_base::evidence::{
    Budget, Certificate, Certified, Margin, Method, Modulus, Outcome, PropMap,
};

impl Sphere {
    /// Creates a sphere
    #[inline(always)]
    pub const fn new(center: Point3, radius: f64) -> Sphere {
        Sphere { center, radius }
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
    /// Returns whether the point `pt` is on sphere
    #[inline(always)]
    pub fn include(&self, pt: Point3) -> bool {
        let ctx = ToleranceCtx::unscaled_legacy();
        ctx.is_small_len(self.center.distance(pt) - self.radius) // BG-TOL-001: model
    }
}

impl ParametricSurface for Sphere {
    type Point = Point3;
    type Vector = Vector3;
    #[inline(always)]
    fn der_mn(&self, m: usize, n: usize, u: f64, v: f64) -> Self::Vector {
        let ((su, cu), (sv, cv)) = (u.sin_cos(), v.sin_cos());
        let center = match (m, n) {
            (0, 0) => self.center().to_vec(),
            _ => Vector3::zero(),
        };
        let u_part = match m % 4 {
            0 => Vector3::new(su, su, cu),
            1 => Vector3::new(cu, cu, -su),
            2 => Vector3::new(-su, -su, -cu),
            _ => Vector3::new(-cu, -cu, su),
        };
        let v_z = if n == 0 { 1.0 } else { 0.0 };
        let v_part = match n % 4 {
            0 => Vector3::new(cv, sv, v_z),
            1 => Vector3::new(-sv, cv, 0.0),
            2 => Vector3::new(-cv, -sv, 0.0),
            _ => Vector3::new(sv, -cv, 0.0),
        };
        center + self.radius * u_part.mul_element_wise(v_part)
    }
    #[inline(always)]
    fn subs(&self, u: f64, v: f64) -> Point3 {
        self.center() + self.radius * self.normal(u, v)
    }
    #[inline(always)]
    fn uder(&self, u: f64, v: f64) -> Vector3 {
        self.radius
            * Vector3::new(
                f64::cos(u) * f64::cos(v),
                f64::cos(u) * f64::sin(v),
                -f64::sin(u),
            )
    }
    #[inline(always)]
    fn vder(&self, u: f64, v: f64) -> Vector3 {
        self.radius * f64::sin(u) * Vector3::new(-f64::sin(v), f64::cos(v), 0.0)
    }
    #[inline(always)]
    fn uuder(&self, u: f64, v: f64) -> Vector3 {
        -self.radius * self.normal(u, v)
    }
    #[inline(always)]
    fn uvder(&self, u: f64, v: f64) -> Vector3 {
        self.radius * f64::cos(u) * Vector3::new(-f64::sin(v), f64::cos(v), 0.0)
    }
    #[inline(always)]
    fn vvder(&self, u: f64, v: f64) -> Vector3 {
        -self.radius * f64::sin(u) * Vector3::new(f64::cos(v), f64::sin(v), 0.0)
    }
    #[inline(always)]
    fn parameter_range(&self) -> (ParameterRange, ParameterRange) {
        (
            (Bound::Included(0.0), Bound::Included(PI)),
            (Bound::Included(0.0), Bound::Excluded(2.0 * PI)),
        )
    }
    #[inline(always)]
    fn v_period(&self) -> Option<f64> {
        Some(2.0 * PI)
    }
}

impl ParametricSurface3D for Sphere {
    #[inline(always)]
    fn normal(&self, u: f64, v: f64) -> Vector3 {
        Vector3::new(
            f64::sin(u) * f64::cos(v),
            f64::sin(u) * f64::sin(v),
            f64::cos(u),
        )
    }
    #[inline(always)]
    fn normal_uder(&self, u: f64, v: f64) -> Vector3 {
        Vector3::new(
            f64::cos(u) * f64::cos(v),
            f64::cos(u) * f64::sin(v),
            -f64::sin(u),
        )
    }
    #[inline(always)]
    fn normal_vder(&self, u: f64, v: f64) -> Vector3 {
        Vector3::new(-f64::sin(u) * f64::sin(v), f64::sin(u) * f64::cos(v), 0.0)
    }
}

impl BoundedSurface for Sphere {}

impl IncludeCurve<BSplineCurve<Point3>> for Sphere {
    #[inline(always)]
    fn include(&self, curve: &BSplineCurve<Point3>) -> Outcome<bool> {
        // BG-S0-001: explicit float certificate (see the `Plane` impls for the
        // provenance rationale).
        Ok(Certified::new(
            curve.is_const() && self.include(curve.front()),
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

impl IncludeCurve<NurbsCurve<Vector4>> for Sphere {
    fn include(&self, curve: &NurbsCurve<Vector4>) -> Outcome<bool> {
        let (knots, _) = curve.knot_vec().to_single_multi();
        let degree = curve.degree() * 2;
        let value = knots
            .windows(2)
            .flat_map(move |window| (1..degree).map(move |i| (window, i)))
            .all(move |(window, i)| {
                let t = i as f64 / degree as f64;
                let t = window[0] * (1.0 - t) + window[1] * t;
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

impl ParameterDivision2D for Sphere {
    #[inline(always)]
    fn parameter_division(
        &self,
        (urange, vrange): ((f64, f64), (f64, f64)),
        tol: f64,
    ) -> (Vec<f64>, Vec<f64>) {
        let ctx = ToleranceCtx::unscaled_legacy();
        let tol = tol.max(ctx.length_margin()); // BG-TOL-001: model
        nonpositive_tolerance!(tol);
        // A tolerance coarser than the sphere is a meaningful request rather
        // than a caller error: a tolerance derived from the extent of a whole
        // model is routinely larger than the smallest features in it, and a
        // sphere smaller than the permitted chord deviation simply cannot be
        // subdivided any further. Panicking on that took down the entire
        // tessellation of otherwise valid CAD assemblies.
        //
        // Clamping the ratio also keeps `acos` inside its domain, which is
        // what the assertion was really protecting: past a ratio of two the
        // argument falls below -1 and the subdivision would be NaN. At a ratio
        // of one `delta` is already pi, the coarsest subdivision there is, so
        // nothing above that can mesh any differently.
        let ratio = f64::min(tol / self.radius, 1.0);
        let delta = 2.0 * f64::acos(1.0 - ratio);
        let u_div = 1 + ((urange.1 - urange.0) / delta).floor() as usize;
        let v_div = 1 + ((vrange.1 - vrange.0) / delta).floor() as usize;
        (
            (0..=u_div)
                .map(|i| urange.0 + (urange.1 - urange.0) * i as f64 / u_div as f64)
                .collect(),
            (0..=v_div)
                .map(|j| vrange.0 + (vrange.1 - vrange.0) * j as f64 / v_div as f64)
                .collect(),
        )
    }
}

impl SearchParameter<D2> for Sphere {
    type Point = Point3;
    #[inline(always)]
    fn search_parameter<H: Into<SPHint2D>>(
        &self,
        point: Point3,
        hint: H,
        _: usize,
    ) -> Option<(f64, f64)> {
        let ctx = ToleranceCtx::unscaled_legacy();
        let radius = point - self.center;
        // FIXME(BG-TOL-001): squared order -- both sides are length squared and tau_rep is first order
        if (self.radius * self.radius).near(&radius.magnitude2()) {
            let radius = radius.normalize();
            let u = f64::acos(radius[2]);
            let sinu = f64::sqrt(1.0 - radius[2] * radius[2]);
            let cosv = f64::clamp(radius[0] / sinu, -1.0, 1.0);
            let v = if ctx.is_small_ratio(sinu) {
                // BG-TOL-001: param
                match hint.into() {
                    SPHint2D::Parameter(_, hint) => hint,
                    _ => 0.0,
                }
            } else if radius[1] > 0.0 {
                f64::acos(cosv)
            } else {
                2.0 * PI - f64::acos(cosv)
            };
            Some((u, v))
        } else {
            None
        }
    }
}

impl SearchNearestParameter<D2> for Sphere {
    type Point = Point3;
    #[inline(always)]
    fn search_nearest_parameter<H: Into<SPHint2D>>(
        &self,
        point: Point3,
        _: H,
        _: usize,
    ) -> Option<(f64, f64)> {
        let ctx = ToleranceCtx::unscaled_legacy();
        let radius = point - self.center;
        if ctx.is_small_len(radius.magnitude()) {
            return None;
        }
        let radius = radius.normalize();
        let u = f64::acos(f64::clamp(radius[2], -1.0, 1.0));
        let sinu = f64::sqrt(1.0 - radius[2] * radius[2]);
        let cosv = f64::clamp(radius[0] / sinu, -1.0, 1.0);
        let v = if radius[1] > 0.0 {
            f64::acos(cosv)
        } else {
            2.0 * PI - f64::acos(cosv)
        };
        Some((u, v))
    }
}

#[test]
fn sphere_search_nearest_parameter_center_is_none() {
    let sphere = Sphere::new(Point3::origin(), 1.0);
    assert!(
        sphere
            .search_nearest_parameter(Point3::origin(), None, 0)
            .is_none(),
        "the sphere center has no nearest parameter"
    );
}
