/// Dimension for search nearest parameter
pub trait SPDimension {
    /// dimension
    const DIM: usize;
    /// parameter type, curve => f64, surface => (f64, f64)
    type Parameter;
    /// hint, curve => [`SPHint1D`], surface => [`SPHint2D`]
    type Hint;
}

/// curve geometry
#[derive(Clone, Copy, Debug)]
pub enum D1 {}

impl SPDimension for D1 {
    const DIM: usize = 1;
    type Parameter = f64;
    type Hint = SPHint1D;
}

/// curve geometry
#[derive(Clone, Copy, Debug)]
pub enum D2 {}

impl SPDimension for D2 {
    const DIM: usize = 2;
    type Parameter = (f64, f64);
    type Hint = SPHint2D;
}

/// hint for searching parameter for curve
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SPHint1D {
    /// a parameter near the answer
    Parameter(f64),
    /// the range of parameter including answer
    Range(f64, f64),
    /// There are no hint. In the case of `BoundedCurve`, most of the time the parameter range is applied.
    /// Such as planes, no hinting is needed in the first place.
    None,
}

impl From<f64> for SPHint1D {
    #[inline(always)]
    fn from(x: f64) -> SPHint1D {
        SPHint1D::Parameter(x)
    }
}

impl From<(f64, f64)> for SPHint1D {
    #[inline(always)]
    fn from(range: (f64, f64)) -> SPHint1D {
        SPHint1D::Range(range.0, range.1)
    }
}

impl From<Option<f64>> for SPHint1D {
    #[inline(always)]
    fn from(x: Option<f64>) -> SPHint1D {
        match x {
            Some(x) => x.into(),
            None => SPHint1D::None,
        }
    }
}

/// hint for searching parameter for surface
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SPHint2D {
    /// a parameter near the answer
    Parameter(f64, f64),
    /// the range of parameter including answer
    Range((f64, f64), (f64, f64)),
    /// There are no hint. If the algorithm needed a hint, it always returns None.
    None,
}

impl From<(f64, f64)> for SPHint2D {
    #[inline(always)]
    fn from(x: (f64, f64)) -> Self {
        Self::Parameter(x.0, x.1)
    }
}

impl From<((f64, f64), (f64, f64))> for SPHint2D {
    #[inline(always)]
    fn from(ranges: ((f64, f64), (f64, f64))) -> Self {
        Self::Range(ranges.0, ranges.1)
    }
}

impl From<Option<(f64, f64)>> for SPHint2D {
    #[inline(always)]
    fn from(x: Option<(f64, f64)>) -> Self {
        match x {
            Some(x) => x.into(),
            None => SPHint2D::None,
        }
    }
}

/// Search parameter `t` such that `self.subs(t)` is near point.
pub trait SearchParameter<Dim: SPDimension> {
    /// point
    type Point;
    /// Search parameter `t` such that `self.subs(t)` is near point.  
    /// Returns `None` if could not find such parameter.
    fn search_parameter<H: Into<Dim::Hint>>(
        &self,
        point: Self::Point,
        hint: H,
        trials: usize,
    ) -> Option<Dim::Parameter>;

    /// Starting parameters the geometry's own structure suggests for the
    /// iterative inverse, most promising first.
    ///
    /// [`Self::search_parameter`] is a Newton iteration, and a Newton iteration
    /// from one bad start fails on a surface whose inverse is perfectly
    /// well-posed. The default hint strategy is a uniform presearch grid that
    /// keeps only its single best cell, which is a poor start precisely where
    /// the geometry is not uniform. A piecewise geometry knows better: its
    /// knot vector already partitions its domain, and one start per span
    /// covers every piece.
    ///
    /// An empty result means "no structure to offer" — the default, and the
    /// honest answer for a primitive whose inverse is closed-form anyway. A
    /// caller must treat the seeds as *starts*, never as answers: each one
    /// still has to converge and still has to be checked for incidence.
    fn search_parameter_seeds(&self) -> Vec<Dim::Parameter> {
        Vec::new()
    }
}

impl<Dim: SPDimension, T: SearchParameter<Dim>> SearchParameter<Dim> for &T {
    type Point = T::Point;
    #[inline(always)]
    fn search_parameter<H: Into<Dim::Hint>>(
        &self,
        point: Self::Point,
        hint: H,
        trials: usize,
    ) -> Option<Dim::Parameter> {
        T::search_parameter(*self, point, hint, trials)
    }
    #[inline(always)]
    fn search_parameter_seeds(&self) -> Vec<Dim::Parameter> {
        T::search_parameter_seeds(*self)
    }
}

impl<Dim: SPDimension, T: SearchParameter<Dim>> SearchParameter<Dim> for Box<T> {
    type Point = T::Point;
    #[inline(always)]
    fn search_parameter<H: Into<Dim::Hint>>(
        &self,
        point: Self::Point,
        hint: H,
        trials: usize,
    ) -> Option<Dim::Parameter> {
        T::search_parameter(&**self, point, hint, trials)
    }
    #[inline(always)]
    fn search_parameter_seeds(&self) -> Vec<Dim::Parameter> {
        T::search_parameter_seeds(&**self)
    }
}

/// Search parameter `t` such that `self.subs(t)` is nearest point.
pub trait SearchNearestParameter<Dim: SPDimension> {
    /// point
    type Point;
    /// Search nearest parameter `t` such that `self.subs(t)` is nearest point.  
    /// Returns `None` if could not find such parameter.
    fn search_nearest_parameter<H: Into<Dim::Hint>>(
        &self,
        point: Self::Point,
        hint: H,
        trials: usize,
    ) -> Option<Dim::Parameter>;
}

impl<Dim: SPDimension, T: SearchNearestParameter<Dim>> SearchNearestParameter<Dim> for &T {
    type Point = T::Point;
    #[inline(always)]
    fn search_nearest_parameter<H: Into<Dim::Hint>>(
        &self,
        point: Self::Point,
        hint: H,
        trials: usize,
    ) -> Option<Dim::Parameter> {
        T::search_nearest_parameter(*self, point, hint, trials)
    }
}

impl<Dim: SPDimension, T: SearchNearestParameter<Dim>> SearchNearestParameter<Dim> for Box<T> {
    type Point = T::Point;
    #[inline(always)]
    fn search_nearest_parameter<H: Into<Dim::Hint>>(
        &self,
        point: Self::Point,
        hint: H,
        trials: usize,
    ) -> Option<Dim::Parameter> {
        T::search_nearest_parameter(&**self, point, hint, trials)
    }
}
