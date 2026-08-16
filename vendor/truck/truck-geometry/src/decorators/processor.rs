use super::*;
use algo::surface::SsnpVector;
use truck_base::evidence::{EnvelopeCase, Outcome, Refusal};

impl<E, T: One> Processor<E, T> {
    /// Creates new processor
    #[inline(always)]
    pub fn new(entity: E) -> Self {
        Self {
            entity,
            transform: T::one(),
            orientation: true,
        }
    }

    /// Creates new transformed processor
    #[inline(always)]
    pub const fn with_transform(entity: E, transform: T) -> Self {
        Self {
            entity,
            transform,
            orientation: true,
        }
    }

    /// Returns the reference of entity
    #[inline(always)]
    pub const fn entity(&self) -> &E {
        &self.entity
    }

    /// Returns the mutable reference of entity
    #[inline(always)]
    pub fn entity_mut(&mut self) -> &mut E {
        &mut self.entity
    }

    /// Returns the reference of transform
    #[inline(always)]
    pub const fn transform(&self) -> &T {
        &self.transform
    }

    /// Returns the orientation of surface
    #[inline(always)]
    pub const fn orientation(&self) -> bool {
        self.orientation
    }

    #[inline(always)]
    fn sign(&self) -> f64 {
        match self.orientation {
            true => 1.0,
            false => -1.0,
        }
    }

    /// apply the function to the entity geometry
    #[inline(always)]
    pub fn map<G, F: FnOnce(E) -> G>(self, f: F) -> Processor<G, T> {
        Processor {
            entity: f(self.entity),
            transform: self.transform,
            orientation: self.orientation,
        }
    }

    /// apply the function to the entity geometry
    #[inline(always)]
    pub fn map_ref<G, F: FnOnce(&E) -> G>(&self, f: F) -> Processor<G, T>
    where
        T: Copy,
    {
        Processor {
            entity: f(&self.entity),
            transform: self.transform,
            orientation: self.orientation,
        }
    }

    /// apply the transform and inverse
    pub fn contract(self) -> E
    where
        E: Transformed<T> + Invertible,
    {
        let mut res = self.entity;
        res.transform_by(self.transform);
        if !self.orientation {
            res.invert();
        }
        res
    }
}

impl<E: Clone, T: Clone> Invertible for Processor<E, T> {
    #[inline(always)]
    fn invert(&mut self) {
        self.orientation = !self.orientation;
    }
    #[inline(always)]
    fn inverse(&self) -> Self {
        Processor {
            entity: self.entity.clone(),
            transform: self.transform.clone(),
            orientation: !self.orientation,
        }
    }
}

impl<C: BoundedCurve, T> Processor<C, T> {
    #[inline(always)]
    fn get_curve_parameter(&self, t: f64) -> f64 {
        let (t0, t1) = self.range_tuple();
        match self.orientation {
            true => t,
            false => t0 + t1 - t,
        }
    }
}

impl<C, T> ParametricCurve for Processor<C, T>
where
    C: BoundedCurve,
    C::Point: EuclideanSpace<Diff = C::Vector>,
    C::Vector: VectorSpace<Scalar = f64>,
    T: Transform<C::Point> + Clone,
{
    type Point = C::Point;
    type Vector = C::Vector;
    #[inline(always)]
    fn der_n(&self, n: usize, t: f64) -> Self::Vector {
        if n == 0 {
            self.subs(t).to_vec()
        } else {
            let t = self.get_curve_parameter(t);
            self.transform.transform_vector(self.entity.der_n(n, t))
        }
    }
    #[inline(always)]
    fn subs(&self, t: f64) -> C::Point {
        let t = self.get_curve_parameter(t);
        self.transform.transform_point(self.entity.subs(t))
    }
    #[inline(always)]
    fn der(&self, t: f64) -> Self::Vector {
        let t = self.get_curve_parameter(t);
        self.transform.transform_vector(self.entity.der(t)) * self.sign()
    }
    #[inline(always)]
    fn der2(&self, t: f64) -> Self::Vector {
        let t = self.get_curve_parameter(t);
        self.transform.transform_vector(self.entity.der2(t))
    }
    #[inline(always)]
    fn parameter_range(&self) -> ParameterRange {
        self.entity.parameter_range()
    }
    #[inline(always)]
    fn period(&self) -> Option<f64> {
        self.entity.period()
    }
}

impl<C, T> BoundedCurve for Processor<C, T>
where
    C: BoundedCurve,
    C::Point: EuclideanSpace<Diff = C::Vector>,
    C::Vector: VectorSpace<Scalar = f64>,
    T: Transform<C::Point> + Clone,
{
}

impl<C, T> Cut for Processor<C, T>
where
    C: BoundedCurve + Cut,
    C::Point: EuclideanSpace<Diff = C::Vector>,
    C::Vector: VectorSpace<Scalar = f64>,
    T: Transform<C::Point> + Clone,
{
    fn cut(&mut self, t: f64) -> Self {
        let t = self.get_curve_parameter(t);
        let mut entity = self.entity.cut(t);
        if !self.orientation {
            std::mem::swap(&mut entity, &mut self.entity);
        }
        Self {
            entity,
            transform: self.transform.clone(),
            orientation: self.orientation,
        }
    }
}

impl<S, T> ParametricSurface for Processor<S, T>
where
    S: ParametricSurface,
    S::Point: EuclideanSpace<Scalar = f64, Diff = S::Vector>,
    T: Transform<S::Point> + SquareMatrix<Scalar = f64> + Clone,
{
    type Point = S::Point;
    type Vector = S::Vector;
    #[inline(always)]
    fn der_mn(&self, m: usize, n: usize, u: f64, v: f64) -> Self::Vector {
        if (m, n) == (0, 0) {
            self.subs(u, v).to_vec()
        } else {
            match self.orientation {
                true => self
                    .transform
                    .transform_vector(self.entity.der_mn(m, n, u, v)),
                false => self
                    .transform
                    .transform_vector(self.entity.der_mn(n, m, v, u)),
            }
        }
    }
    #[inline(always)]
    fn subs(&self, u: f64, v: f64) -> Self::Point {
        match self.orientation {
            true => self.transform.transform_point(self.entity.subs(u, v)),
            false => self.transform.transform_point(self.entity.subs(v, u)),
        }
    }
    #[inline(always)]
    fn uder(&self, u: f64, v: f64) -> Self::Vector {
        match self.orientation {
            true => self.transform.transform_vector(self.entity.uder(u, v)),
            false => self.transform.transform_vector(self.entity.vder(v, u)),
        }
    }
    #[inline(always)]
    fn vder(&self, u: f64, v: f64) -> Self::Vector {
        match self.orientation {
            true => self.transform.transform_vector(self.entity.vder(u, v)),
            false => self.transform.transform_vector(self.entity.uder(v, u)),
        }
    }
    #[inline(always)]
    fn uuder(&self, u: f64, v: f64) -> Self::Vector {
        match self.orientation {
            true => self.transform.transform_vector(self.entity.uuder(u, v)),
            false => self.transform.transform_vector(self.entity.vvder(v, u)),
        }
    }
    #[inline(always)]
    fn uvder(&self, u: f64, v: f64) -> Self::Vector {
        match self.orientation {
            true => self.transform.transform_vector(self.entity.uvder(u, v)),
            false => self.transform.transform_vector(self.entity.uvder(v, u)),
        }
    }
    #[inline(always)]
    fn vvder(&self, u: f64, v: f64) -> Self::Vector {
        match self.orientation {
            true => self.transform.transform_vector(self.entity.vvder(u, v)),
            false => self.transform.transform_vector(self.entity.uuder(v, u)),
        }
    }
    #[inline(always)]
    fn parameter_range(&self) -> (ParameterRange, ParameterRange) {
        let (urange, vrange) = self.entity.parameter_range();
        match self.orientation {
            true => (urange, vrange),
            false => (vrange, urange),
        }
    }
    #[inline(always)]
    fn u_period(&self) -> Option<f64> {
        match self.orientation {
            true => self.entity.u_period(),
            false => self.entity.v_period(),
        }
    }
    #[inline(always)]
    fn v_period(&self) -> Option<f64> {
        match self.orientation {
            true => self.entity.v_period(),
            false => self.entity.u_period(),
        }
    }
}

impl<S, T> ParametricSurface3D for Processor<S, T>
where
    S: ParametricSurface3D,
    T: Transform<Point3> + SquareMatrix<Scalar = f64> + Clone,
{
    #[inline(always)]
    fn normal(&self, u: f64, v: f64) -> Self::Vector {
        let transtrans = self.transform.transpose();
        let n = match self.orientation {
            true => self.entity.normal(u, v),
            false => -self.entity.normal(v, u),
        };
        let n = transtrans
            .inverse_transform_vector(n)
            .expect("invalid transform");
        (n / self.transform.determinant()).normalize()
    }
}

impl<S, T> BoundedSurface for Processor<S, T>
where
    S: BoundedSurface<Point = Point3, Vector = Vector3>,
    T: Transform<S::Point> + SquareMatrix<Scalar = f64> + Clone,
{
}

impl<E, T> Deref for Processor<E, T> {
    type Target = E;
    #[inline(always)]
    fn deref(&self) -> &E {
        &self.entity
    }
}

impl<E, T> DerefMut for Processor<E, T> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut E {
        &mut self.entity
    }
}

impl<E, T> Transformed<T> for Processor<E, T>
where
    T: Mul<T, Output = T> + Copy,
    E: Clone,
{
    #[inline(always)]
    fn transform_by(&mut self, trans: T) {
        self.transform = trans * self.transform;
    }
    #[inline(always)]
    fn transformed(&self, trans: T) -> Self {
        Self {
            entity: self.entity.clone(),
            transform: trans * self.transform,
            orientation: self.orientation,
        }
    }
}

impl<E, T, C> IncludeCurve<C> for Processor<E, T>
where
    C: ParametricCurve + Transformed<T> + Clone,
    C::Point: EuclideanSpace,
    E: IncludeCurve<C>,
    T: Transform<C::Point>,
{
    fn include(&self, curve: &C) -> Outcome<bool> {
        // BG-S0-001 (H-1): a singular transform has no inverse to test the
        // curve against. That is a chart degeneracy (§9.1), not a reason to
        // panic — the previous `.expect("irregular transform")` aborted on
        // data. Refuse instead.
        let Some(inv) = self.transform.inverse_transform() else {
            return Err(Refusal::UnsupportedEnvelope(EnvelopeCase::ChartDegenerate));
        };
        let curve = curve.clone().transformed(inv);
        self.entity.include(&curve)
    }
}

impl<C> ParameterDivision1D for Processor<C, Matrix3>
where
    C: ParameterDivision1D<Point = Point2> + BoundedCurve<Point = Point2>,
{
    type Point = Point2;
    fn parameter_division(&self, range: (f64, f64), tol: f64) -> (Vec<f64>, Vec<Self::Point>) {
        let a = self.transform;
        let range = match self.orientation {
            true => range,
            false => (
                self.get_curve_parameter(range.1),
                self.get_curve_parameter(range.0),
            ),
        };
        let (_, k, _) = a
            .iwasawa_decomposition()
            .expect("transform matrix must be invertible!");
        let n = f64::abs(k[0][0])
            .max(f64::abs(k[1][1]))
            .max(f64::abs(k[2][2]));
        let (mut params, mut points) = self.entity.parameter_division(range, tol / n);
        points
            .iter_mut()
            .for_each(|pt| *pt = a.transform_point(*pt));
        if !self.orientation {
            params
                .iter_mut()
                .for_each(|t| *t = self.get_curve_parameter(*t));
            points.reverse();
        }
        (params, points)
    }
}

impl<C> ParameterDivision1D for Processor<C, Matrix4>
where
    C: ParameterDivision1D<Point = Point3> + BoundedCurve<Point = Point3>,
{
    type Point = Point3;
    fn parameter_division(&self, range: (f64, f64), tol: f64) -> (Vec<f64>, Vec<Self::Point>) {
        let a = self.transform;
        let range = match self.orientation {
            true => range,
            false => (
                self.get_curve_parameter(range.1),
                self.get_curve_parameter(range.0),
            ),
        };
        let (_, k, _) = a
            .iwasawa_decomposition()
            .expect("transform matrix must be invertible!");
        let n = f64::abs(k[0][0])
            .max(f64::abs(k[1][1]))
            .max(f64::abs(k[2][2]))
            / f64::abs(k[3][3]);
        let (mut params, mut points) = self.entity.parameter_division(range, tol / n);
        points
            .iter_mut()
            .for_each(|pt| *pt = a.transform_point(*pt));
        if !self.orientation {
            params
                .iter_mut()
                .for_each(|t| *t = self.get_curve_parameter(*t));
            points.reverse();
        }
        (params, points)
    }
}

impl<S: ParameterDivision2D> ParameterDivision2D for Processor<S, Matrix3> {
    fn parameter_division(
        &self,
        range: ((f64, f64), (f64, f64)),
        tol: f64,
    ) -> (Vec<f64>, Vec<f64>) {
        let a = self.transform;
        let range = match self.orientation {
            true => range,
            false => (range.1, range.0),
        };
        let (_, k, _) = a
            .iwasawa_decomposition()
            .expect("transform matrix must be invertible!");
        let n = f64::abs(k[0][0])
            .max(f64::abs(k[1][1]))
            .max(f64::abs(k[2][2]));
        let (udiv, vdiv) = self.entity.parameter_division(range, tol / n);
        match self.orientation {
            true => (udiv, vdiv),
            false => (vdiv, udiv),
        }
    }
}

impl<S: ParameterDivision2D> ParameterDivision2D for Processor<S, Matrix4> {
    fn parameter_division(
        &self,
        range: ((f64, f64), (f64, f64)),
        tol: f64,
    ) -> (Vec<f64>, Vec<f64>) {
        let a = self.transform;
        let range = match self.orientation {
            true => range,
            false => (range.1, range.0),
        };
        let (_, k, _) = a
            .iwasawa_decomposition()
            .expect("transform matrix must be invertible!");
        let n = f64::abs(k[0][0])
            .max(f64::abs(k[1][1]))
            .max(f64::abs(k[2][2]))
            / f64::abs(k[3][3]);
        let (udiv, vdiv) = self.entity.parameter_division(range, tol / n);
        match self.orientation {
            true => (udiv, vdiv),
            false => (vdiv, udiv),
        }
    }
}

impl<E, T> SearchParameter<D1> for Processor<E, T>
where
    E: SearchParameter<D1> + BoundedCurve,
    <E as SearchParameter<D1>>::Point: EuclideanSpace,
    T: Transform<<E as SearchParameter<D1>>::Point>,
{
    type Point = <E as SearchParameter<D1>>::Point;
    fn search_parameter<H: Into<SPHint1D>>(
        &self,
        point: <E as SearchParameter<D1>>::Point,
        hint: H,
        trials: usize,
    ) -> Option<f64> {
        let inv = self.transform.inverse_transform().unwrap();
        let t = self
            .entity
            .search_parameter(inv.transform_point(point), hint, trials)?;
        Some(self.get_curve_parameter(t))
    }
}

/// Restate a 2D search hint in the *entity's* axis convention.
///
/// An inverted `Processor` evaluates `entity.subs(v, u)`, so every axis-indexed
/// quantity crossing this boundary is transposed — `subs`, the partial
/// derivatives, `parameter_range`, `u_period`/`v_period` and `normal` all do so.
/// A hint travels in the opposite direction to a result: the caller supplies it
/// in the caller's axes and the entity consumes it in the entity's, so it must
/// be transposed on the way *in*, not on the way out.
///
/// Forwarding it untransposed steered the search along the wrong axis. The
/// result was still transposed on return, so the defect stayed invisible
/// wherever the hint was ignored or the surface was injective enough that any
/// starting point converged — while degrading exactly the branch continuity
/// that lifting a boundary across a periodic axis depends on.
#[inline(always)]
fn transpose_hint(hint: SPHint2D, upright: bool) -> SPHint2D {
    match (upright, hint) {
        (true, hint) | (false, hint @ SPHint2D::None) => hint,
        (false, SPHint2D::Parameter(u, v)) => SPHint2D::Parameter(v, u),
        (false, SPHint2D::Range(u, v)) => SPHint2D::Range(v, u),
    }
}

impl<E, T> SearchParameter<D2> for Processor<E, T>
where
    E: SearchParameter<D2>,
    E::Point: EuclideanSpace,
    T: Transform<E::Point>,
{
    type Point = E::Point;
    fn search_parameter<H: Into<SPHint2D>>(
        &self,
        point: E::Point,
        hint: H,
        trials: usize,
    ) -> Option<(f64, f64)> {
        let inv = self.transform.inverse_transform().unwrap();
        let hint = transpose_hint(hint.into(), self.orientation);
        let (u, v) = self
            .entity
            .search_parameter(inv.transform_point(point), hint, trials)?;
        match self.orientation {
            true => Some((u, v)),
            false => Some((v, u)),
        }
    }
}

impl<P, E, T> SearchNearestParameter<D1> for Processor<E, T>
where
    E: BoundedCurve<Point = P> + SearchNearestParameter<D1, Point = P>,
    P: EuclideanSpace<Scalar = f64, Diff = E::Vector>,
    E::Vector: InnerSpace<Scalar = f64> + Tolerance,
    T: Transform<P> + Clone,
{
    type Point = P;
    fn search_nearest_parameter<H: Into<SPHint1D>>(
        &self,
        point: Self::Point,
        hint: H,
        trials: usize,
    ) -> Option<f64> {
        let inv = self.transform.inverse_transform().unwrap();
        let hint =
            self.entity
                .search_nearest_parameter(inv.transform_point(point), hint, trials)?;
        let hint = self.get_curve_parameter(hint);
        algo::curve::search_nearest_parameter(self, point, hint, trials)
    }
}

impl<P, E, T> SearchNearestParameter<D2> for Processor<E, T>
where
    E: ParametricSurface<Point = P> + SearchNearestParameter<D2, Point = P>,
    P: EuclideanSpace<Scalar = f64, Diff = E::Vector> + MetricSpace<Metric = f64> + Tolerance,
    E::Vector: SsnpVector<Point = P>,
    T: Transform<P> + SquareMatrix<Scalar = f64> + Clone,
{
    type Point = P;
    fn search_nearest_parameter<H: Into<SPHint2D>>(
        &self,
        point: Self::Point,
        hint: H,
        trials: usize,
    ) -> Option<(f64, f64)> {
        let inv = self.transform.inverse_transform().unwrap();
        // As in `SearchParameter<D2>`: the incoming hint is in the caller's
        // axes and the entity reads it in its own, so transpose on the way in.
        // The result comes back in the entity's axes and is transposed below
        // before it is used as a hint against `self`.
        let hint = hint.into();
        // Whether the caller supplied a hint at all, read before it is
        // transposed and consumed. The fallback below is admitted only on the
        // hintless call, which is the *last* thing the meshalgo projection
        // chain asks of an analytic surface — see there.
        let hintless = matches!(hint, SPHint2D::None);
        let hint = transpose_hint(hint, self.orientation);
        let hint =
            self.entity
                .search_nearest_parameter(inv.transform_point(point), hint, trials)?;
        let hint = match self.orientation {
            true => hint,
            false => (hint.1, hint.0),
        };
        algo::surface::search_nearest_parameter(self, point, hint, trials)
            // `hint` is not a hint. It is the entity's own answer for the
            // inverse-transformed point, mapped back to this processor's
            // parameter axes — closed form for every primitive that reaches
            // here (cylinder and cone are `RevolutedCurve<Line>`, torus is
            // `Torus`). The generic Newton above refines it, and when the
            // Newton fails to converge the refinement is discarded *along with
            // the answer it started from*, so a surface that can invert itself
            // exactly reports that it cannot invert itself at all.
            //
            // Returning the unrefined answer is safe rather than optimistic:
            // this trait promises a *nearest* parameter, never an incidence,
            // so every caller that needs the point to lie on the surface
            // already checks the residual itself. The meshalgo boundary lift
            // does exactly that, against the caller's tolerance, immediately
            // after this returns — so a bad closed-form answer is refused
            // there by the check that already exists, and typed as the
            // off-surface point it is instead of a projection that failed.
            //
            // **Only on the hintless call**, and that restriction is load
            // bearing rather than conservative. The meshalgo chain asks four
            // things in order, of which the third is
            // `search_nearest_parameter(point, hint)` and the fourth is
            // `search_nearest_parameter(point, None)`. Admitted on the third,
            // the entity's answer is taken from whichever branch or period
            // copy the caller's hint led to, and it pre-empts the better
            // answer the hintless call would have found — measured: one cone
            // face on `00009190` went rendered -> lost that way, with the
            // recovery otherwise identical. Restricted to the hintless call
            // the fallback is last, and can replace nothing but a failure.
            //
            // `TRUCK_FORMAL_RECOVERY_ANALYTIC=0` withdraws it, restoring the
            // discard exactly.
            .or_else(|| (hintless && analytic_inverse_fallback_enabled()).then_some(hint))
    }
}

/// Whether [`Processor`] may return the entity's own closed-form inverse when
/// the generic Newton refinement fails to converge.
///
/// Default-on, disabled by an explicit `0`/`off`/`false`/`no`, matching the
/// `TRUCK_FORMAL_RECOVERY_*` convention in `truck-meshalgo`. Read once: this
/// sits on the per-boundary-point path.
fn analytic_inverse_fallback_enabled() -> bool {
    static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ENABLED.get_or_init(|| {
        !std::env::var("TRUCK_FORMAL_RECOVERY_ANALYTIC").is_ok_and(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "0" | "off" | "false" | "no"
            )
        })
    })
}

impl<E, T, U> ToSameGeometry<U> for Processor<E, T>
where
    E: ToSameGeometry<U>,
    T: Copy,
    U: Transformed<T> + Invertible,
{
    fn to_same_geometry(&self) -> U {
        let Self {
            entity,
            transform,
            orientation,
        } = self;
        let mut u = entity.to_same_geometry();
        u.transform_by(*transform);
        if !orientation {
            u.invert();
        }
        u
    }
}

#[cfg(test)]
mod hint_axis_tests {
    use super::*;
    use std::f64::consts::PI;

    /// A cylinder: `u` runs along the generatrix, `v` is the exact `2π`
    /// revolution angle. Periodic in `v`, so a hint genuinely selects a branch.
    fn cylinder() -> RevolutedCurve<Line<Point3>> {
        RevolutedCurve::by_revolution(
            Line(Point3::new(1.0, 0.0, 0.0), Point3::new(1.0, 0.0, 1.0)),
            Point3::origin(),
            Vector3::unit_z(),
        )
    }

    fn swap((u, v): (f64, f64)) -> (f64, f64) {
        (v, u)
    }

    /// The property that must hold, stated as a commuting square rather than as
    /// hint-invariance.
    ///
    /// Invariance would be the *wrong* acceptance condition: on a periodic
    /// surface a hint legitimately changes which local inverse is found, so a
    /// test asserting that the answer is unaffected by the hint would pass only
    /// where the hint is ignored — precisely the cases that cannot detect this
    /// defect. What must hold instead is that going through the processor
    /// agrees with transposing into the entity's axes, asking the entity, and
    /// transposing back.
    #[test]
    fn inverted_processor_commutes_with_axis_transposition() {
        let entity = cylinder();
        let mut processor = Processor::<_, Matrix4>::new(cylinder());
        processor.invert();
        assert!(!processor.orientation(), "the processor must be inverted");

        // A point on the cylinder, named in the entity's own axes, and a hint
        // deliberately near a *different* period copy so the branch matters.
        let point = entity.subs(0.4, 2.0);
        for hint in [(0.35, 2.1), (0.35, 2.1 + 2.0 * PI), (0.35, 2.1 - 2.0 * PI)] {
            // The caller states the hint in the caller's axes.
            let caller_hint = swap(hint);
            let through_processor = processor.search_parameter(point, caller_hint, 100);
            let through_entity = entity.search_parameter(point, hint, 100).map(swap);
            assert_eq!(
                through_processor, through_entity,
                "processor and transposed-entity disagree for hint {caller_hint:?}",
            );
        }
    }

    /// The other half: an upright processor must forward the hint untouched.
    #[test]
    fn upright_processor_forwards_the_hint_unchanged() {
        let entity = cylinder();
        let processor = Processor::<_, Matrix4>::new(cylinder());
        assert!(processor.orientation());

        let point = entity.subs(0.4, 2.0);
        for hint in [(0.35, 2.1), (0.35, 2.1 + 2.0 * PI)] {
            assert_eq!(
                processor.search_parameter(point, hint, 100),
                entity.search_parameter(point, hint, 100),
                "an upright processor must not transpose",
            );
        }
    }

    /// `None` transposes to `None`, and a range transposes as a pair.
    #[test]
    fn hint_transposition_covers_every_variant() {
        assert_eq!(
            transpose_hint(SPHint2D::Parameter(1.0, 2.0), false),
            SPHint2D::Parameter(2.0, 1.0),
        );
        assert_eq!(
            transpose_hint(SPHint2D::Range((0.0, 1.0), (2.0, 3.0)), false),
            SPHint2D::Range((2.0, 3.0), (0.0, 1.0)),
        );
        assert_eq!(transpose_hint(SPHint2D::None, false), SPHint2D::None);
        assert_eq!(
            transpose_hint(SPHint2D::Parameter(1.0, 2.0), true),
            SPHint2D::Parameter(1.0, 2.0),
        );
    }
}
