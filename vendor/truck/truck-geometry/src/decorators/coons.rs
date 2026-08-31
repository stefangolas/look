#![deny(clippy::unwrap_used)]

use super::*;
use crate::constructive::{ConstructError, DirectTolerance};
use algo::surface::SspVector;
use truck_base::evidence::{
    Budget, Certificate, Certified, Margin, Method, Modulus, Outcome, PropMap,
};

/// The bilinearly blended Coons patch of four boundary curves (plan §3.7).
///
/// r2: ONE curve parameter — all four boundaries have type `C`. (The
/// r1 worker proved the four-parameter form cannot satisfy the trait
/// checklist: `IncludeCurve` impls on independent parameters overlap
/// (E0119) and `Invertible`'s re-parametrization swaps same-role fields
/// (E0308).) Mixed-boundary generality is a promotion case, not a booking.
///
/// Boundary correctness is by EXACT pairwise cancellation against the corner
/// term in exact arithmetic; in floats it holds to
/// `DirectTolerance::default().position` and the tests assert exactly that.
///
/// Regularity is certified, never assumed: `jacobian` exposes
/// J = S_u × S_v; a folded patch is construction-valid but geometry-invalid.
///
/// Convention (normative): `bottom` runs u: 0→1 at v = 0; `top` runs u: 0→1 at
/// v = 1; `left` runs v: 0→1 at u = 0; `right` runs v: 0→1 at u = 1. Corners:
/// P00 = bottom(0) = left(0), P10 = bottom(1) = right(0), P01 = top(0) =
/// left(1), P11 = top(1) = right(1).
#[derive(Clone, Debug, PartialEq)]
pub struct CoonsSurface<C> {
    bottom: C,
    top: C,
    left: C,
    right: C,
    p00: Point3,
    p10: Point3,
    p01: Point3,
    p11: Point3,
}

impl<C> CoonsSurface<C> {
    /// Returns the bottom boundary curve (`u: 0 → 1` at `v = 0`).
    #[inline(always)]
    pub fn bottom(&self) -> &C {
        &self.bottom
    }
    /// Returns the top boundary curve (`u: 0 → 1` at `v = 1`).
    #[inline(always)]
    pub fn top(&self) -> &C {
        &self.top
    }
    /// Returns the left boundary curve (`v: 0 → 1` at `u = 0`).
    #[inline(always)]
    pub fn left(&self) -> &C {
        &self.left
    }
    /// Returns the right boundary curve (`v: 0 → 1` at `u = 1`).
    #[inline(always)]
    pub fn right(&self) -> &C {
        &self.right
    }
}

impl<C: ParametricCurve3D> CoonsSurface<C> {
    /// Validates the four corner equalities pairwise at
    /// `DirectTolerance::default().position` and caches the four corners.
    ///
    /// Any corner mismatch or any non-finite corner is
    /// `ConstructError::InvalidInput`.
    pub fn try_new(
        bottom: C,
        top: C,
        left: C,
        right: C,
    ) -> std::result::Result<Self, ConstructError> {
        let b00 = bottom.subs(0.0);
        let b10 = bottom.subs(1.0);
        let t00 = top.subs(0.0);
        let t11 = top.subs(1.0);
        let l00 = left.subs(0.0);
        let l01 = left.subs(1.0);
        let r00 = right.subs(0.0);
        let r11 = right.subs(1.0);
        let tol = DirectTolerance::default().position;
        let finite = [b00, b10, t00, t11, l00, l01, r00, r11]
            .iter()
            .all(|p| p.x.is_finite() && p.y.is_finite() && p.z.is_finite());
        if !finite
            || (b00 - l00).magnitude() > tol
            || (b10 - r00).magnitude() > tol
            || (t00 - l01).magnitude() > tol
            || (t11 - r11).magnitude() > tol
        {
            return Err(ConstructError::InvalidInput);
        }
        Ok(Self {
            bottom,
            top,
            left,
            right,
            p00: b00,
            p10: b10,
            p01: t00,
            p11: t11,
        })
    }
    /// J = S_u × S_v at `(u, v)` — the certified regularity witness. A folded
    /// patch (construction-valid) has J vanishing somewhere; the caller
    /// certifies, this only reports.
    #[inline(always)]
    pub fn jacobian(&self, u: f64, v: f64) -> Vector3 {
        self.uder(u, v).cross(self.vder(u, v))
    }
}

impl<C: ParametricCurve3D + Invertible> CoonsSurface<C> {
    /// Tries the 16 finite legal reversals in lexicographic order (bottom's
    /// flag most significant, `false < true`) and returns the first
    /// `try_new` success together with its `(bottom, top, left, right)`
    /// inversion flags.
    ///
    /// A consistent-as-given input returns flips `[false; 4]`. If all 16
    /// refuse, the last `try_new` error is returned.
    pub fn try_new_any_orientation(
        bottom: C,
        top: C,
        left: C,
        right: C,
    ) -> std::result::Result<(Self, [bool; 4]), ConstructError> {
        let mut last_error = None;
        for bits in 0..16 {
            let flags = [
                (bits & 0b1000) != 0,
                (bits & 0b0100) != 0,
                (bits & 0b0010) != 0,
                (bits & 0b0001) != 0,
            ];
            let candidate = (
                if flags[0] {
                    bottom.inverse()
                } else {
                    bottom.clone()
                },
                if flags[1] { top.inverse() } else { top.clone() },
                if flags[2] {
                    left.inverse()
                } else {
                    left.clone()
                },
                if flags[3] {
                    right.inverse()
                } else {
                    right.clone()
                },
            );
            match Self::try_new(candidate.0, candidate.1, candidate.2, candidate.3) {
                Ok(surface) => return Ok((surface, flags)),
                Err(err) => last_error = Some(err),
            }
        }
        match last_error {
            Some(err) => Err(err),
            None => Err(ConstructError::InvalidInput),
        }
    }
}

impl<C: ParametricCurve3D> ParametricSurface for CoonsSurface<C> {
    type Point = Point3;
    type Vector = Vector3;

    fn der_mn(&self, m: usize, n: usize, u: f64, v: f64) -> Self::Vector {
        match (m, n) {
            (0, 0) => {
                let b = self.bottom.subs(u);
                let t = self.top.subs(u);
                let l = self.left.subs(v);
                let r = self.right.subs(v);
                let corner = self.p00.to_vec() * ((1.0 - u) * (1.0 - v))
                    + self.p10.to_vec() * (u * (1.0 - v))
                    + self.p01.to_vec() * ((1.0 - u) * v)
                    + self.p11.to_vec() * (u * v);
                b.to_vec() * (1.0 - v) + t.to_vec() * v + l.to_vec() * (1.0 - u) + r.to_vec() * u
                    - corner
            }
            (1, 0) => {
                let l = self.left.subs(v);
                let r = self.right.subs(v);
                let corner_u = (self.p10 - self.p00) * (1.0 - v) + (self.p11 - self.p01) * v;
                self.bottom.der(u) * (1.0 - v) + self.top.der(u) * v + (r - l) - corner_u
            }
            (0, 1) => {
                let b = self.bottom.subs(u);
                let t = self.top.subs(u);
                let corner_v = (self.p01 - self.p00) * (1.0 - u) + (self.p11 - self.p10) * u;
                (t - b) + self.left.der(v) * (1.0 - u) + self.right.der(v) * u - corner_v
            }
            (2, 0) => self.bottom.der2(u) * (1.0 - v) + self.top.der2(u) * v,
            (1, 1) => {
                (self.top.der(u) - self.bottom.der(u))
                    + (self.right.der(v) - self.left.der(v))
                    + (self.p10 - self.p00)
                    - (self.p11 - self.p01)
            }
            (0, 2) => self.left.der2(v) * (1.0 - u) + self.right.der2(v) * u,
            _ => Self::Vector::zero(),
        }
    }
    #[inline(always)]
    fn subs(&self, u: f64, v: f64) -> Self::Point {
        Point3::from_vec(self.der_mn(0, 0, u, v))
    }
    #[inline(always)]
    fn uder(&self, u: f64, v: f64) -> Self::Vector {
        self.der_mn(1, 0, u, v)
    }
    #[inline(always)]
    fn vder(&self, u: f64, v: f64) -> Self::Vector {
        self.der_mn(0, 1, u, v)
    }
    #[inline(always)]
    fn uuder(&self, u: f64, v: f64) -> Self::Vector {
        self.der_mn(2, 0, u, v)
    }
    #[inline(always)]
    fn uvder(&self, u: f64, v: f64) -> Self::Vector {
        self.der_mn(1, 1, u, v)
    }
    #[inline(always)]
    fn vvder(&self, u: f64, v: f64) -> Self::Vector {
        self.der_mn(0, 2, u, v)
    }
    #[inline(always)]
    fn parameter_range(&self) -> (ParameterRange, ParameterRange) {
        (
            (Bound::Included(0.0), Bound::Included(1.0)),
            (Bound::Included(0.0), Bound::Included(1.0)),
        )
    }
}

impl<C: ParametricCurve3D> ParametricSurface3D for CoonsSurface<C> {}

impl<C: BoundedCurve> BoundedSurface for CoonsSurface<C> where Self: ParametricSurface {}

impl<C: ParameterDivision1D> ParameterDivision2D for CoonsSurface<C> {
    fn parameter_division(
        &self,
        (urange, vrange): ((f64, f64), (f64, f64)),
        tol: f64,
    ) -> (Vec<f64>, Vec<f64>) {
        let (mut udiv, _) = self.bottom.parameter_division(urange, tol);
        let (top_div, _) = self.top.parameter_division(urange, tol);
        udiv.extend(top_div);
        udiv.sort_by(|x, y| x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal));
        udiv.dedup();
        let (mut vdiv, _) = self.left.parameter_division(vrange, tol);
        let (right_div, _) = self.right.parameter_division(vrange, tol);
        vdiv.extend(right_div);
        vdiv.sort_by(|x, y| x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal));
        vdiv.dedup();
        (udiv, vdiv)
    }
}

impl<C> SearchParameter<D2> for CoonsSurface<C>
where
    C: ParametricCurve3D + BoundedCurve,
    Vector3: SspVector<Point = Point3>,
{
    type Point = Point3;
    fn search_parameter<H: Into<SPHint2D>>(
        &self,
        point: Point3,
        hint: H,
        trials: usize,
    ) -> Option<(f64, f64)> {
        let hint = match hint.into() {
            SPHint2D::Parameter(x, y) => (x, y),
            SPHint2D::Range(range0, range1) => {
                algo::surface::presearch(self, point, (range0, range1), PRESEARCH_DIVISION)
            }
            SPHint2D::None => {
                algo::surface::presearch(self, point, self.range_tuple(), PRESEARCH_DIVISION)
            }
        };
        algo::surface::search_parameter(self, point, hint, trials)
    }
}

impl<C> Invertible for CoonsSurface<C>
where
    C: ParametricCurve3D + Invertible,
{
    fn invert(&mut self) {
        self.bottom = self.bottom.inverse();
        self.top = self.top.inverse();
        std::mem::swap(&mut self.left, &mut self.right);
        std::mem::swap(&mut self.p00, &mut self.p10);
        std::mem::swap(&mut self.p01, &mut self.p11);
    }
    fn inverse(&self) -> Self {
        Self {
            bottom: self.bottom.inverse(),
            top: self.top.inverse(),
            left: self.right.clone(),
            right: self.left.clone(),
            p00: self.p10,
            p10: self.p00,
            p01: self.p11,
            p11: self.p01,
        }
    }
}

impl<C> Transformed<Matrix4> for CoonsSurface<C>
where
    C: ParametricCurve3D + Transformed<Matrix4>,
{
    fn transform_by(&mut self, trans: Matrix4) {
        self.bottom.transform_by(trans);
        self.top.transform_by(trans);
        self.left.transform_by(trans);
        self.right.transform_by(trans);
        self.p00.transform_by(trans);
        self.p10.transform_by(trans);
        self.p01.transform_by(trans);
        self.p11.transform_by(trans);
    }
    fn transformed(&self, trans: Matrix4) -> Self {
        Self {
            bottom: self.bottom.transformed(trans),
            top: self.top.transformed(trans),
            left: self.left.transformed(trans),
            right: self.right.transformed(trans),
            p00: self.p00.transformed(trans),
            p10: self.p10.transformed(trans),
            p01: self.p01.transformed(trans),
            p11: self.p11.transformed(trans),
        }
    }
}

impl<C> IncludeCurve<C> for CoonsSurface<C>
where
    C: ParametricCurve3D + PartialEq,
{
    fn include(&self, curve: &C) -> Outcome<bool> {
        // BG-S0-001: structural equality against the stored boundary curves; the
        // predicate is a float computation (H-6), claims no properties, and
        // consumes no caller-provided budget.
        Ok(Certified::new(
            curve == &self.bottom
                || curve == &self.top
                || curve == &self.left
                || curve == &self.right,
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
