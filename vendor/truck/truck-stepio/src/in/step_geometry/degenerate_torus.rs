//! Carrier for a STEP `degenerate_toroidal_surface`.
//!
//! ISO 10303-42 defines `degenerate_toroidal_surface` as a `toroidal_surface`
//! whose WHERE clause fixes `major_radius < minor_radius` and that adds one
//! boolean, `select_outer`. With `R < r` the torus parametrisation
//!
//! ```text
//! σ(u, v) = ((R + r·cos v)·cos u, (R + r·cos v)·sin u, r·sin v)
//! ```
//!
//! is self-intersecting (a spindle torus): the set of points with
//! `R + r·cos v < 0` is a second surface ("lemon") folded inside the outer
//! one, meeting it only at the pinch ring `R + r·cos v = 0`. The face must
//! therefore name *which* sheet it lies on, and that is what `select_outer`
//! does, as a restricted `v` interval of the carrier parametrisation:
//!
//! ```text
//! cos φ = -R / r
//! select_outer = true  ⇒  u ∈ [0, 2π], v ∈ [-φ, φ]
//! select_outer = false ⇒  u ∈ [0, 2π], v ∈ [φ, 2π - φ]
//! ```
//!
//! On either interval the restricted map is an embedding (the fold is
//! excluded), so the surface is a proper bounded sheet: `u` is `2π`-periodic,
//! `v` is not, and every projection answer must stay on the sheet it declares.
//!
//! The carrier wraps [`Torus`] so the same analytic geometry and its
//! derivatives are reused; only the declared domain and the inverse are sheet
//! aware.

use super::*;
use std::f64::consts::PI;
use std::ops::Bound;

/// Carrier of a `degenerate_toroidal_surface`.
///
/// Constructed only with `0 < R < r` (the EXPRESS WHERE clause), so `φ =
/// acos(-R/r)` always lies in `(π/2, π)`.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, SelfSameGeometry)]
pub struct DegenerateTorus {
    inner: Torus,
    select_outer: bool,
}

impl DegenerateTorus {
    /// Build the sheet carrier, refusing radii that do not satisfy the source
    /// WHERE clause `0 < major_radius < minor_radius` (both finite).
    pub fn new(major_radius: f64, minor_radius: f64, select_outer: bool) -> Option<Self> {
        if !major_radius.is_finite()
            || !minor_radius.is_finite()
            || major_radius <= 0.0
            || minor_radius <= 0.0
            || major_radius >= minor_radius
        {
            return None;
        }
        let inner = Torus::new(Point3::origin(), major_radius, minor_radius);
        Some(Self {
            inner,
            select_outer,
        })
    }

    /// The underlying torus carrier.
    #[inline(always)]
    pub const fn inner(&self) -> &Torus {
        &self.inner
    }

    /// Whether this sheet is the outer one (`v ∈ [-φ, φ]`) or the inner one.
    #[inline(always)]
    pub const fn select_outer(&self) -> bool {
        self.select_outer
    }

    /// The source-defined sheet half-angle `φ = acos(-R/r)`.
    #[inline(always)]
    fn phi(&self) -> f64 {
        f64::acos(-self.inner.large_radius() / self.inner.small_radius())
    }

    /// The restricted `v` interval this sheet occupies.
    ///
    /// Outer sheet: `[-φ, φ]`. Inner sheet: `[φ, 2π - φ]`.
    #[inline(always)]
    pub fn v_range(&self) -> (f64, f64) {
        let phi = self.phi();
        match self.select_outer {
            true => (-phi, phi),
            false => (phi, 2.0 * PI - phi),
        }
    }

    /// Closed-form inverse for a point on the outer sheet, in the carrier's
    /// local frame (the torus centre at the origin, the axis along `z`).
    fn inverse_outer(&self, point: Point3) -> Option<(f64, f64)> {
        let ctx = ToleranceCtx::unscaled_legacy();
        let r = point - self.inner.center();
        let rxy = Vector2::new(r.x, r.y);
        let rho = rxy.magnitude();
        let (v0, v1) = self.v_range();
        let v = f64::atan2(r.z, rho - self.inner.large_radius());
        if !(v >= v0 && v <= v1) {
            return None;
        }
        let u = if ctx.is_small_len(rxy.magnitude()) {
            // BG-TOL-001: model
            0.0
        } else {
            let rxy_n = rxy.normalize();
            let u = f64::acos(f64::clamp(rxy_n.x, -1.0, 1.0));
            match rxy_n.y < 0.0 {
                true => 2.0 * PI - u,
                false => u,
            }
        };
        Some((u, v))
    }

    /// Closed-form inverse for a point on the inner sheet.
    ///
    /// On the inner sheet the radial distance is `ρ = -(R + r·cos v)` and the
    /// point's azimuth is `u + π`, so `u` is recovered from `atan2(-y, -x)` and
    /// `v` from `atan2(z, -(ρ + R))`.
    fn inverse_inner(&self, point: Point3) -> Option<(f64, f64)> {
        let ctx = ToleranceCtx::unscaled_legacy();
        let r = point - self.inner.center();
        let rxy = Vector2::new(r.x, r.y);
        let rho = rxy.magnitude();
        let large = self.inner.large_radius();
        let small = self.inner.small_radius();
        let cos_v = -(rho + large) / small;
        if !(-1.0..=1.0).contains(&cos_v) {
            return None;
        }
        let mut v = f64::atan2(r.z, -(rho + large));
        if v < 0.0 {
            v += 2.0 * PI;
        }
        let (v0, v1) = self.v_range();
        if !(v >= v0 && v <= v1) {
            return None;
        }
        let u = if ctx.is_small_len(rxy.magnitude()) {
            // BG-TOL-001: model
            0.0
        } else {
            let u = f64::atan2(-r.y, -r.x);
            match u < 0.0 {
                true => u + 2.0 * PI,
                false => u,
            }
        };
        Some((u, v))
    }
}

impl ParametricSurface for DegenerateTorus {
    type Point = Point3;
    type Vector = Vector3;
    #[inline(always)]
    fn der_mn(&self, m: usize, n: usize, u: f64, v: f64) -> Self::Vector {
        self.inner.der_mn(m, n, u, v)
    }
    #[inline(always)]
    fn subs(&self, u: f64, v: f64) -> Self::Point {
        self.inner.subs(u, v)
    }
    #[inline(always)]
    fn uder(&self, u: f64, v: f64) -> Self::Vector {
        self.inner.uder(u, v)
    }
    #[inline(always)]
    fn vder(&self, u: f64, v: f64) -> Self::Vector {
        self.inner.vder(u, v)
    }
    #[inline(always)]
    fn uuder(&self, u: f64, v: f64) -> Self::Vector {
        self.inner.uuder(u, v)
    }
    #[inline(always)]
    fn uvder(&self, u: f64, v: f64) -> Self::Vector {
        self.inner.uvder(u, v)
    }
    #[inline(always)]
    fn vvder(&self, u: f64, v: f64) -> Self::Vector {
        self.inner.vvder(u, v)
    }
    fn parameter_range(&self) -> (ParameterRange, ParameterRange) {
        let (v0, v1) = self.v_range();
        (
            (Bound::Included(0.0), Bound::Excluded(2.0 * PI)),
            (Bound::Included(v0), Bound::Included(v1)),
        )
    }
    #[inline(always)]
    fn u_period(&self) -> Option<f64> {
        Some(2.0 * PI)
    }
    #[inline(always)]
    fn v_period(&self) -> Option<f64> {
        None
    }
}

impl ParametricSurface3D for DegenerateTorus {
    #[inline(always)]
    fn normal(&self, u: f64, v: f64) -> Self::Vector {
        self.inner.normal(u, v)
    }
}

impl BoundedSurface for DegenerateTorus {}

impl ParameterDivision2D for DegenerateTorus {
    #[inline(always)]
    fn parameter_division(
        &self,
        range: ((f64, f64), (f64, f64)),
        tol: f64,
    ) -> (Vec<f64>, Vec<f64>) {
        self.inner.parameter_division(range, tol)
    }
}

impl SearchParameter<D2> for DegenerateTorus {
    type Point = Point3;
    fn search_parameter<H: Into<SPHint2D>>(
        &self,
        point: Point3,
        _: H,
        _trials: usize,
    ) -> Option<(f64, f64)> {
        let ctx = ToleranceCtx::unscaled_legacy();
        let (u, v) = match self.select_outer {
            true => self.inverse_outer(point),
            false => self.inverse_inner(point),
        }?;
        match ctx.near_pt(self.subs(u, v), point) {
            // BG-TOL-001: model
            true => Some((u, v)),
            false => None,
        }
    }
}

impl SearchNearestParameter<D2> for DegenerateTorus {
    type Point = Point3;
    fn search_nearest_parameter<H: Into<SPHint2D>>(
        &self,
        point: Point3,
        _hint: H,
        trials: usize,
    ) -> Option<(f64, f64)> {
        // An on-sheet point answers in closed form.
        let ctx = ToleranceCtx::unscaled_legacy();
        if let Some(uv) = self.search_parameter(point, SPHint2D::None, trials) {
            if ctx.near_pt(self.subs(uv.0, uv.1), point) {
                // BG-TOL-001: model
                return Some(uv);
            }
        }
        // Otherwise presearch the declared sheet and refine from the best cell.
        // Newton is unconstrained on the parametrisation, so the answer is
        // clamped back onto the sheet at the end; the caller's incidence check
        // is what actually decides whether an off-sheet point was admissible.
        let (urange, vrange) = self.try_range_tuple();
        let (urange, vrange) = (urange?, vrange?);
        let start = algo::surface::presearch(self, point, (urange, vrange), 40);
        let uv = algo::surface::search_nearest_parameter(self, point, start, trials)?;
        let (v0, v1) = self.v_range();
        Some((uv.0, uv.1.clamp(v0, v1)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The source-defined sheet interval: `v ∈ [-φ, φ]` outer, `v ∈ [φ, 2π-φ]`
    /// inner, with `cos φ = -R/r`.
    #[test]
    fn the_sheet_domain_is_the_source_interval() {
        let phi = f64::acos(-0.5);
        let outer = DegenerateTorus::new(0.5, 1.0, true).expect("valid spindle");
        let (v0, v1) = outer.v_range();
        assert!((v0 + phi).abs() < 1.0e-12, "outer lower bound");
        assert!((v1 - phi).abs() < 1.0e-12, "outer upper bound");
        assert_eq!(outer.u_period(), Some(2.0 * PI));
        assert_eq!(outer.v_period(), None);

        let inner = DegenerateTorus::new(0.5, 1.0, false).expect("valid spindle");
        let (v0, v1) = inner.v_range();
        assert!((v0 - phi).abs() < 1.0e-12, "inner lower bound");
        assert!((v1 - (2.0 * PI - phi)).abs() < 1.0e-12, "inner upper bound");
    }

    /// The closed-form sheet inverse must round-trip every point of the sheet
    /// domain it declares.
    #[test]
    fn the_sheet_inverse_round_trips_on_sheet_points() {
        for select_outer in [true, false] {
            let carrier = DegenerateTorus::new(0.5, 1.0, select_outer).expect("valid spindle");
            let (v0, v1) = carrier.v_range();
            for i in 0..=10 {
                for j in 0..=10 {
                    let u = 2.0 * PI * i as f64 / 10.0;
                    let v = v0 + (v1 - v0) * j as f64 / 10.0;
                    let point = carrier.subs(u, v);
                    let (u2, v2) = carrier
                        .search_parameter(point, SPHint2D::None, 100)
                        .unwrap_or_else(|| panic!("no inverse on {select_outer} at ({u},{v})"));
                    let back = carrier.subs(u2, v2);
                    assert!(
                        point.near(&back),
                        "round trip failed on {select_outer} at ({u},{v}): {point:?} vs {back:?}",
                    );
                }
            }
        }
    }

    /// A point of the other sheet must not project onto this sheet: the two
    /// sheets share only the pinch ring, so a plain torus inverse (which mixes
    /// them) would be a silent semantic error.
    #[test]
    fn an_off_sheet_point_is_refused() {
        let outer = DegenerateTorus::new(0.5, 1.0, true).expect("valid spindle");
        let inner = DegenerateTorus::new(0.5, 1.0, false).expect("valid spindle");
        let p_inner = inner.subs(0.0, PI);
        assert!(
            outer
                .search_parameter(p_inner, SPHint2D::None, 100)
                .is_none(),
            "an inner-sheet point must not project onto the outer sheet",
        );
        let p_outer = outer.subs(0.0, 0.0);
        assert!(
            inner
                .search_parameter(p_outer, SPHint2D::None, 100)
                .is_none(),
            "an outer-sheet point must not project onto the inner sheet",
        );
    }

    /// Radii that violate the EXPRESS WHERE clause must refuse construction.
    #[test]
    fn invalid_radii_are_refused() {
        assert!(
            DegenerateTorus::new(2.0, 1.0, true).is_none(),
            "R >= r refuses"
        );
        assert!(
            DegenerateTorus::new(0.0, 1.0, true).is_none(),
            "R <= 0 refuses"
        );
        assert!(
            DegenerateTorus::new(0.5, 0.0, true).is_none(),
            "r <= 0 refuses"
        );
        assert!(
            DegenerateTorus::new(f64::NAN, 1.0, true).is_none(),
            "nonfinite R refuses",
        );
        assert!(
            DegenerateTorus::new(0.5, f64::INFINITY, true).is_none(),
            "nonfinite r refuses",
        );
    }
}
