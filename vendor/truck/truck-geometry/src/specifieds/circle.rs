use super::*;
use std::f64::consts::{PI, TAU};

impl<P> UnitCircle<P> {
    /// constructor
    #[inline]
    pub const fn new() -> Self {
        Self(std::marker::PhantomData)
    }
}

impl ParametricCurve for UnitCircle<Point2> {
    type Point = Point2;
    type Vector = Vector2;
    #[inline]
    fn der_n(&self, n: usize, t: f64) -> Vector2 {
        match n % 4 {
            0 => Vector2::new(f64::cos(t), f64::sin(t)),
            1 => Vector2::new(-f64::sin(t), f64::cos(t)),
            2 => Vector2::new(-f64::cos(t), -f64::sin(t)),
            _ => Vector2::new(f64::sin(t), -f64::cos(t)),
        }
    }
    #[inline]
    fn subs(&self, t: f64) -> Point2 {
        Point2::from_vec(self.der_n(0, t))
    }
    #[inline]
    fn der(&self, t: f64) -> Vector2 {
        self.der_n(1, t)
    }
    #[inline]
    fn der2(&self, t: f64) -> Vector2 {
        self.der_n(2, t)
    }
    #[inline]
    fn parameter_range(&self) -> ParameterRange {
        (Bound::Included(0.0), Bound::Excluded(TAU))
    }
}

impl BoundedCurve for UnitCircle<Point2> {}

impl ParametricCurve for UnitCircle<Point3> {
    type Point = Point3;
    type Vector = Vector3;
    #[inline]
    fn der_n(&self, n: usize, t: f64) -> Vector3 {
        match n % 4 {
            0 => Vector3::new(f64::cos(t), f64::sin(t), 0.0),
            1 => Vector3::new(-f64::sin(t), f64::cos(t), 0.0),
            2 => Vector3::new(-f64::cos(t), -f64::sin(t), 0.0),
            _ => Vector3::new(f64::sin(t), -f64::cos(t), 0.0),
        }
    }
    #[inline]
    fn subs(&self, t: f64) -> Point3 {
        Point3::from_vec(self.der_n(0, t))
    }
    #[inline]
    fn der(&self, t: f64) -> Vector3 {
        self.der_n(1, t)
    }
    #[inline]
    fn der2(&self, t: f64) -> Vector3 {
        self.der_n(2, t)
    }
    #[inline]
    fn period(&self) -> Option<f64> {
        Some(TAU)
    }
    #[inline]
    fn parameter_range(&self) -> ParameterRange {
        (Bound::Included(0.0), Bound::Excluded(TAU))
    }
}

impl BoundedCurve for UnitCircle<Point3> {}

impl<P> ParameterDivision1D for UnitCircle<P>
where
    UnitCircle<P>: ParametricCurve<Point = P>,
{
    type Point = P;
    fn parameter_division(&self, range: (f64, f64), tol: f64) -> (Vec<f64>, Vec<P>) {
        let ctx = ToleranceCtx::unscaled_legacy();
        let tol = tol.max(ctx.ratio_margin()); // BG-TOL-001: param
        nonpositive_tolerance!(tol);
        let tol = f64::min(tol, 0.8);
        let delta = 2.0 * f64::acos(1.0 - tol);
        let n = 1 + ((range.1 - range.0) / delta) as usize;
        let params = (0..=n)
            .map(|i| {
                let t = i as f64 / n as f64;
                range.0 * (1.0 - t) + range.1 * t
            })
            .collect::<Vec<_>>();
        let pts = params.iter().map(|t| self.subs(*t)).collect();
        (params, pts)
    }
}

impl SearchNearestParameter<D1> for UnitCircle<Point2> {
    type Point = Point2;
    fn search_nearest_parameter<H: Into<SPHint1D>>(
        &self,
        pt: Point2,
        hint: H,
        _: usize,
    ) -> Option<f64> {
        let ctx = ToleranceCtx::unscaled_legacy();
        let v = pt.to_vec();
        if ctx.is_small_ratio(v.magnitude()) {
            // BG-TOL-001: param
            return None;
        }
        let v = v.normalize();
        let theta = f64::acos(f64::clamp(v.x, -1.0, 1.0));
        let theta = match v.y > 0.0 {
            true => theta,
            false => TAU - theta,
        };
        Some(round_theta(theta, hint.into()))
    }
}

impl SearchParameter<D1> for UnitCircle<Point2> {
    type Point = Point2;
    fn search_parameter<H: Into<SPHint1D>>(&self, pt: Point2, hint: H, _: usize) -> Option<f64> {
        let ctx = ToleranceCtx::unscaled_legacy();
        let v = pt.to_vec();
        if !ctx.is_small_ratio(v.magnitude() - 1.0) {
            // BG-TOL-001: param
            return None;
        }
        let v = v.normalize();
        let theta = f64::acos(f64::clamp(v.x, -1.0, 1.0));
        let theta = match v.y > 0.0 {
            true => theta,
            false => TAU - theta,
        };
        Some(round_theta(theta, hint.into()))
    }
}

fn round_theta(theta: f64, hint: SPHint1D) -> f64 {
    match hint {
        SPHint1D::None => theta,
        SPHint1D::Parameter(hint) => {
            let floor = (hint / TAU).floor() * TAU;
            [theta + floor - TAU, theta + floor, theta + floor + TAU]
                .into_iter()
                .fold(theta, |theta0, theta| {
                    match (theta - hint).abs() < (theta0 - hint).abs() {
                        true => theta,
                        false => theta0,
                    }
                })
        }
        SPHint1D::Range(hint0, hint1) => {
            let floor = (hint0 / TAU).floor() * TAU;
            let theta = match theta + floor > hint0 {
                true => theta + floor,
                false => theta + floor + TAU,
            };
            if theta < hint1 {
                return theta;
            }
            let theta0 = theta - TAU;
            match hint0 - theta0 < theta - hint1 {
                true => theta0,
                false => theta,
            }
        }
    }
}

impl SearchNearestParameter<D1> for UnitCircle<Point3> {
    type Point = Point3;
    fn search_nearest_parameter<H: Into<SPHint1D>>(
        &self,
        pt: Point3,
        _: H,
        _: usize,
    ) -> Option<f64> {
        UnitCircle::<Point2>::new().search_nearest_parameter(Point2::new(pt.x, pt.y), None, 0)
    }
}

impl SearchParameter<D1> for UnitCircle<Point3> {
    type Point = Point3;
    fn search_parameter<H: Into<SPHint1D>>(&self, pt: Point3, _: H, _: usize) -> Option<f64> {
        let ctx = ToleranceCtx::unscaled_legacy();
        if !ctx.is_small_ratio(pt.z) {
            // BG-TOL-001: param
            return None;
        }
        UnitCircle::<Point2>::new().search_parameter(Point2::new(pt.x, pt.y), None, 0)
    }
}

impl ToSameGeometry<NurbsCurve<Vector3>> for TrimmedCurve<UnitCircle<Point2>> {
    fn to_same_geometry(&self) -> NurbsCurve<Vector3> {
        let (t0, t1) = self.range_tuple();
        let angle = t1 - t0;
        let (cos2, sin2) = (f64::cos(angle / 2.0), f64::sin(angle / 2.0));
        let rot = Matrix3::from(Matrix2::from_angle(Rad(t0)));
        NurbsCurve::new(BSplineCurve::new_unchecked(
            KnotVec::bezier_knot(2),
            vec![
                rot * Vector3::new(1.0, 0.0, 1.0),
                rot * Vector3::new(cos2, sin2, cos2),
                rot * Vector3::new(f64::cos(angle), f64::sin(angle), 1.0),
            ],
        ))
    }
}

impl ToSameGeometry<NurbsCurve<Vector4>> for TrimmedCurve<UnitCircle<Point3>> {
    fn to_same_geometry(&self) -> NurbsCurve<Vector4> {
        let (t0, t1) = self.range_tuple();
        let angle = t1 - t0;
        if angle >= TAU {
            // AUD-009: a single quadratic Bezier arc cannot represent a full
            // circle: its middle weight `cos(angle / 2)` is `cos(π) = -1` for
            // `angle = 2π`, and the evaluated weight hits exactly 0 at the
            // antipode, so `subs` there is NaN and every include that routes
            // the circle through this conversion answers false. Split the arc
            // into `ceil(angle / π)` half-circle pieces (exactly two for a
            // full circle), convert each piece through the half-circle path —
            // weight-0-middle but never degenerate — and concatenate them into
            // ONE curve on a shared knot vector (two quadratic Bezier spans
            // for a full circle). The join keeps the endpoint/antipode
            // geometry: every evaluated weight is strictly positive.
            let n = (angle / PI).ceil() as usize;
            let mut curve = arc_to_vector4(t0, t0 + PI);
            let mut offset = 1.0;
            for i in 1..n {
                let start = t0 + i as f64 * PI;
                let end = f64::min(start + PI, t1);
                let mut piece = arc_to_vector4(start, end);
                piece.knot_translate(offset);
                curve = curve.concat(&piece);
                offset += 1.0;
            }
            curve.knot_normalize();
            curve
        } else {
            let mut curve = arc_to_vector4(t0, t1);
            curve.add_knot(0.25);
            curve.add_knot(0.5);
            curve.add_knot(0.75);
            curve
        }
    }
}

/// A circle arc (angle ≤ π) as a rational NURBS in `Vector4` form, lifted from
/// the 2D homogeneous arc the `UnitCircle<Point2>` conversion produces.
fn arc_to_vector4(start: f64, end: f64) -> NurbsCurve<Vector4> {
    let bsp: NurbsCurve<Vector3> =
        TrimmedCurve::new(UnitCircle::<Point2>::new(), (start, end)).to_same_geometry();
    let (knot_vec, pts) = bsp.into_non_rationalized().destruct();
    NurbsCurve::new(BSplineCurve::new_unchecked(
        knot_vec,
        pts.into_iter()
            .map(|pt| Vector4::new(pt.x, pt.y, 0.0, pt.z))
            .collect(),
    ))
}

#[cfg(test)]
mod full_circle_conversion_tests {
    use super::*;
    use std::f64::consts::TAU;

    #[test]
    fn full_circle_conversion_antipode_is_finite() {
        // AUD-009: a full circle must convert to a NURBS whose every evaluated
        // point is finite (the single-arc conversion hit a zero weight at the
        // antipode) and whose evaluated weight is never zero.
        let circle = TrimmedCurve::new(UnitCircle::<Point3>::new(), (0.0, TAU));
        let nurbs = ToSameGeometry::<NurbsCurve<Vector4>>::to_same_geometry(&circle);
        const SAMPLES: usize = 128;
        let mut closest_to_antipode = f64::INFINITY;
        for i in 0..=SAMPLES {
            let t = i as f64 / SAMPLES as f64;
            let p = nurbs.subs(t);
            assert!(
                p.x.is_finite() && p.y.is_finite() && p.z.is_finite(),
                "subs({t}) is not finite: {p:?}"
            );
            let weight = nurbs.non_rationalized().subs(t).w;
            assert!(
                weight > 0.0,
                "evaluated weight at {t} is not positive: {weight}"
            );
            closest_to_antipode =
                closest_to_antipode.min((p - Point3::new(-1.0, 0.0, 0.0)).magnitude());
        }
        // The antipode of a circle starting at angle 0 is the point at angle
        // π, found by evaluating the sweep; the piecewise knot vector puts the
        // half-circle join (and hence the antipode) at parameter 0.5.
        assert_near!(nurbs.subs(0.5), Point3::new(-1.0, 0.0, 0.0));
        assert!(
            closest_to_antipode < 1.0e-9, // H-3
            "no sweep sample reached the antipode, closest distance {closest_to_antipode}"
        );
    }
}
