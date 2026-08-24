use super::*;

impl<P> UnitParabola<P> {
    /// constructor
    #[inline]
    pub const fn new() -> Self {
        Self(std::marker::PhantomData)
    }
}

impl ParametricCurve for UnitParabola<Point2> {
    type Point = Point2;
    type Vector = Vector2;
    #[inline]
    fn der_n(&self, n: usize, t: f64) -> Self::Vector {
        match n {
            0 => Vector2::new(t * t, 2.0 * t),
            1 => Vector2::new(2.0 * t, 2.0),
            2 => Vector2::new(2.0, 0.0),
            _ => Vector2::zero(),
        }
    }
    #[inline]
    fn subs(&self, t: f64) -> Self::Point {
        Self::Point::from_vec(self.der_n(0, t))
    }
    #[inline]
    fn der(&self, t: f64) -> Self::Vector {
        self.der_n(1, t)
    }
    #[inline]
    fn der2(&self, t: f64) -> Self::Vector {
        self.der_n(2, t)
    }
}

impl ParametricCurve for UnitParabola<Point3> {
    type Point = Point3;
    type Vector = Vector3;
    fn der_n(&self, n: usize, t: f64) -> Self::Vector {
        match n {
            0 => Vector3::new(t * t, 2.0 * t, 0.0),
            1 => Vector3::new(2.0 * t, 2.0, 0.0),
            2 => Vector3::new(2.0, 0.0, 0.0),
            _ => Vector3::zero(),
        }
    }
    #[inline]
    fn subs(&self, t: f64) -> Self::Point {
        Self::Point::from_vec(self.der_n(0, t))
    }
    #[inline]
    fn der(&self, t: f64) -> Self::Vector {
        self.der_n(1, t)
    }
    #[inline]
    fn der2(&self, t: f64) -> Self::Vector {
        self.der_n(2, t)
    }
}

impl<P> ParameterDivision1D for UnitParabola<P>
where
    UnitParabola<P>: ParametricCurve<Point = P>,
    P: EuclideanSpace<Scalar = f64> + MetricSpace<Metric = f64> + HashGen<f64>,
{
    type Point = P;
    fn parameter_division(&self, range: (f64, f64), tol: f64) -> (Vec<f64>, Vec<P>) {
        algo::curve::parameter_division(self, range, tol)
    }
}

impl SearchNearestParameter<D1> for UnitParabola<Point2> {
    type Point = Point2;
    #[inline]
    fn search_nearest_parameter<H: Into<SPHint1D>>(
        &self,
        pt: Point2,
        _: H,
        _: usize,
    ) -> Option<f64> {
        let ctx = ToleranceCtx::unscaled_legacy();
        let p = 2.0 - pt.x;
        let q = -pt.y;
        solver::pre_solve_cubic(p, q)
            .into_iter()
            .filter_map(|x| match ctx.is_small_ratio(x.im) {
                // BG-TOL-001: param
                true => Some(x.re),
                false => None,
            })
            .min_by(|s, t| {
                pt.distance2(self.subs(*s))
                    .partial_cmp(&pt.distance2(self.subs(*t)))
                    .unwrap()
            })
    }
}

impl SearchNearestParameter<D1> for UnitParabola<Point3> {
    type Point = Point3;
    #[inline]
    fn search_nearest_parameter<H: Into<SPHint1D>>(
        &self,
        pt: Point3,
        _hint: H,
        _trials: usize,
    ) -> Option<f64> {
        UnitParabola::<Point2>::new().search_nearest_parameter(
            Point2::new(pt.x, pt.y),
            _hint,
            _trials,
        )
    }
}

impl SearchParameter<D1> for UnitParabola<Point2> {
    type Point = Point2;
    #[inline]
    fn search_parameter<H: Into<SPHint1D>>(&self, pt: Point2, _: H, _: usize) -> Option<f64> {
        let ctx = ToleranceCtx::unscaled_legacy();
        let t = pt.y / 2.0;
        let pt0 = self.subs(t);
        match ctx.is_small_len((pt - pt0).magnitude()) {
            // BG-TOL-001: model
            true => Some(t),
            false => None,
        }
    }
}

impl SearchParameter<D1> for UnitParabola<Point3> {
    type Point = Point3;
    #[inline]
    fn search_parameter<H: Into<SPHint1D>>(
        &self,
        pt: Point3,
        _hint: H,
        _trials: usize,
    ) -> Option<f64> {
        let ctx = ToleranceCtx::unscaled_legacy();
        match ctx.is_small_ratio(pt.z) {
            // BG-TOL-001: param
            true => UnitParabola::<Point2>::new().search_parameter(
                Point2::new(pt.x, pt.y),
                _hint,
                _trials,
            ),
            false => None,
        }
    }
}

#[test]
fn snp_test() {
    let curve = UnitParabola::<Point2>::new();

    let p = Point2::new(-3.0, 0.0);
    assert_near!(curve.search_nearest_parameter(p, None, 0).unwrap(), 0.0);
    let p = Point2::new(-3.0, 6.0);
    assert_near!(curve.search_nearest_parameter(p, None, 0).unwrap(), 1.0);
    let p = Point2::new(1.5, 1.5);
    assert_near!(curve.search_nearest_parameter(p, None, 0).unwrap(), 1.0);
}

#[test]
fn sp_test() {
    let curve = UnitParabola::<Point2>::new();

    let p = Point2::new(4.0, -4.0);
    assert_near!(curve.search_parameter(p, None, 0).unwrap(), -2.0);
    let p = Point2::new(-3.0, 6.0);
    assert!(curve.search_parameter(p, None, 0).is_none());
}

#[test]
fn conic_containment_scale_invariant() {
    let parabola = UnitParabola::<Point2>::new();
    let hyperbola = UnitHyperbola::<Point2>::new();
    for scale in [0.5, 1.0, 2.0, 10.0] {
        let ctx = match ToleranceCtx::new(scale, TOLERANCE, TOLERANCE, TOLERANCE) {
            Ok(certified) => certified.value,
            Err(_) => {
                unreachable!("a finite positive scale with finite nonnegative taus is accepted")
            }
        };
        let offset = ctx.length_margin() * 10.0;
        for t in [-2.0, -0.5, 0.0, 0.5, 2.0] {
            let on = parabola.subs(t);
            assert!(
                parabola.search_parameter(on, None, 0).is_some(),
                "parabola on-curve point must contain at scale {scale}"
            );
            let off = on + Vector2::new(offset, 0.0);
            assert!(
                parabola.search_parameter(off, None, 0).is_none(),
                "parabola off-curve point must not contain at scale {scale}"
            );
            let on = hyperbola.subs(t);
            assert!(
                hyperbola.search_parameter(on, None, 0).is_some(),
                "hyperbola on-curve point must contain at scale {scale}"
            );
            let off = on + Vector2::new(offset, 0.0);
            assert!(
                hyperbola.search_parameter(off, None, 0).is_none(),
                "hyperbola off-curve point must not contain at scale {scale}"
            );
        }
    }
}
