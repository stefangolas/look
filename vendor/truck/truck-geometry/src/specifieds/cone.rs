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
        radial.near(&(r.z.abs() * self.half_angle.tan()))
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
            curve.is_const() && ctx.is_small_len(radial - r.z.abs() * self.half_angle().tan()),
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
        if !ctx.is_small_len(radial - r.z.abs() * self.half_angle().tan()) {
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
        let azimuth = if radial == 0.0 {
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
        let denom = 1.0 + slope * slope;
        // The double cone is `radial = |v| * slope` in the (radial, z) plane, so
        // the squared distance is `(r_q - |v| * s)^2 + (z_q - v)^2` and the two
        // one-sided stationary candidates are the v_plus (upper) and v_minus
        // (lower) values below. At least one of them is always valid.
        let v_plus = (slope * radial + r.z) / denom;
        let v_minus = (r.z - slope * radial) / denom;
        let (v, upper) = if v_plus >= 0.0 && v_minus <= 0.0 {
            let d_plus = (radial - v_plus * slope).powi(2) + (r.z - v_plus).powi(2);
            let d_minus = (radial + v_minus * slope).powi(2) + (r.z - v_minus).powi(2);
            if d_plus <= d_minus {
                (v_plus, true)
            } else {
                (v_minus, false)
            }
        } else if v_plus >= 0.0 {
            (v_plus, true)
        } else {
            (v_minus, false)
        };
        // A lower-nappe point at parameter u sits at the azimuth u + PI, so the
        // near-side azimuth is reached by shifting the query azimuth by PI there.
        let u = if upper {
            azimuth
        } else {
            let shifted = azimuth + PI;
            if shifted >= 2.0 * PI {
                shifted - 2.0 * PI
            } else {
                shifted
            }
        };
        Some((u, v))
    }
}

#[test]
fn cone_include_holds_pointwise_on_both_nappes() {
    let cone = match Cone::new(Point3::origin(), PI / 4.0) {
        Ok(certified) => certified.value,
        Err(_) => unreachable!("a finite half angle in the open interval is always accepted"),
    };
    let upper = BSplineCurve::new(KnotVec::bezier_knot(0), vec![cone.subs(0.7, 3.0)]);
    let lower = BSplineCurve::new(KnotVec::bezier_knot(0), vec![cone.subs(0.7, -3.0)]);
    assert!(
        matches!(
            IncludeCurve::include(&cone, &upper),
            Ok(Certified { value: true, .. })
        ),
        "the upper nappe point must include"
    );
    assert!(
        matches!(
            IncludeCurve::include(&cone, &lower),
            Ok(Certified { value: true, .. })
        ),
        "the lower nappe point must include"
    );
    let apex = BSplineCurve::new(KnotVec::bezier_knot(0), vec![cone.subs(0.0, 0.0)]);
    assert!(
        matches!(
            IncludeCurve::include(&cone, &apex),
            Ok(Certified { value: true, .. })
        ),
        "the apex must include"
    );
}

#[test]
fn cone_nearest_parameter_near_side_for_lower_nappe_query() {
    let cone = match Cone::new(Point3::origin(), PI / 4.0) {
        Ok(certified) => certified.value,
        Err(_) => unreachable!("a finite half angle in the open interval is always accepted"),
    };
    let query = Point3::new(0.5, 0.0, -3.0);
    let (u, v) = match cone.search_nearest_parameter(query, None, 0) {
        Some((u, v)) => (u, v),
        None => unreachable!("a nearest parameter always exists for a cone"),
    };
    // The double-cone nearest point on the lower nappe is sqrt(25/8) ~= 1.77 from
    // the query; the single-nappe far side is sqrt(49/8) ~= 2.47 away, so a 2.0
    // bound separates the near side from the far side.
    assert!(
        (cone.subs(u, v) - query).magnitude() < 2.0,
        "the returned surface point must be the near side of the lower nappe"
    );
    let on_cone = cone.subs(0.7, -3.0);
    let (u0, v0) = match cone.search_nearest_parameter(on_cone, None, 0) {
        Some((u0, v0)) => (u0, v0),
        None => unreachable!("a nearest parameter always exists for a cone"),
    };
    assert_near!(
        cone.subs(u0, v0),
        on_cone,
        "an on-cone query must round-trip, not snap to the apex"
    );
}
