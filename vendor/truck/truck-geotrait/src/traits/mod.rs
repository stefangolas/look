use std::ops::Bound;
use truck_base::cgmath64::*;
use truck_base::evidence::{
    Budget, Certificate, Certified, Margin, Method, Modulus, Outcome, PropMap,
};

mod curve;
pub use curve::*;
mod surface;
pub use surface::*;
mod search_parameter;
pub use search_parameter::*;

/// parameter range
pub type ParameterRange = (Bound<f64>, Bound<f64>);
fn bound2opt<T>(x: Bound<T>) -> Option<T> {
    match x {
        Bound::Included(x) => Some(x),
        Bound::Excluded(x) => Some(x),
        Bound::Unbounded => None,
    }
}
const UNBOUNDED_ERROR: &str = "Parameter range is unbounded.";

/// Oriented and reversible
pub trait Invertible: Clone {
    /// Inverts `self`
    fn invert(&mut self);
    /// Returns the inverse.
    #[inline(always)]
    fn inverse(&self) -> Self {
        let mut res = self.clone();
        res.invert();
        res
    }
}

/// Implementation for the test of topological methods.
impl Invertible for (usize, usize) {
    fn invert(&mut self) {
        *self = (self.1, self.0);
    }
    fn inverse(&self) -> Self {
        (self.1, self.0)
    }
}

impl<P: Clone> Invertible for Vec<P> {
    #[inline(always)]
    fn invert(&mut self) {
        self.reverse();
    }
    #[inline(always)]
    fn inverse(&self) -> Self {
        self.iter().rev().cloned().collect()
    }
}

impl<T: Invertible> Invertible for Box<T> {
    #[inline(always)]
    fn invert(&mut self) {
        (**self).invert()
    }
    #[inline(always)]
    fn inverse(&self) -> Self {
        Box::new((**self).inverse())
    }
}

/// Transform geometry
pub trait Transformed<T>: Clone {
    /// transform by `trans`.
    fn transform_by(&mut self, trans: T);
    /// transformed geometry by `trans`.
    #[inline(always)]
    fn transformed(&self, trans: T) -> Self {
        let mut res = self.clone();
        res.transform_by(trans);
        res
    }
}

impl<S: Transformed<T>, T> Transformed<T> for Box<S> {
    #[inline(always)]
    fn transform_by(&mut self, trans: T) {
        (**self).transform_by(trans)
    }
    #[inline(always)]
    fn transformed(&self, trans: T) -> Self {
        Box::new((**self).transformed(trans))
    }
}

macro_rules! impl_transformed {
    ($point: ty, $matrix: ty) => {
        impl Transformed<$matrix> for $point {
            #[inline(always)]
            fn transform_by(&mut self, trans: $matrix) {
                *self = trans.transform_point(*self)
            }
            #[inline(always)]
            fn transformed(&self, trans: $matrix) -> Self {
                trans.transform_point(*self)
            }
        }
    };
}
impl_transformed!(Point2, Matrix3);
impl_transformed!(Point3, Matrix3);
impl_transformed!(Point3, Matrix4);

/// Obtain a curve or surface that gives the same image as a given curve or surface.
pub trait ToSameGeometry<T> {
    /// Obtain a curve or surface that gives the same image as a given curve or surface.
    fn to_same_geometry(&self) -> T;

    /// The fallible form (BG-S0-003, H-2). Implementors whose conversion can
    /// fail on admissible input override this; the default delegates, so every
    /// existing impl keeps working unchanged.
    fn try_to_same_geometry(&self) -> Outcome<T> {
        // BG-S0-003, H-6: the concrete conversions are float arithmetic, so
        // the default certifies `Float`, never `Exact`. A refusal-capable
        // implementor overrides this method and supplies its own certificate.
        Ok(Certified::new(
            self.to_same_geometry(),
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
