//! BG-ANA-001 — exactly solvable surface pairs.
//!
//! One submodule per analytic pair family. The **shared result type**
//! `AnalyticIntersection` (with `ExactCurve`) lives here, in the module root,
//! so that all eight families speak one vocabulary: no submodule defines a
//! private result enum. The type was designed and landed by the orchestrator
//! before any shard was dispatched, because eight workers given "define the
//! result type as you see fit" would define eight of them.
//!
//! BG-ANA-002 constrains every submodule: position classification is decided
//! by **exact predicates on the carrier parameters**, never by sampling the
//! surfaces, and no analytic pair may return a float-certified result — if it
//! could, the pair belongs in the general solver. The cells built here become
//! the ground-truth oracle for BG-NUM-003.

use truck_base::cgmath64::{Matrix4, Point3};
use truck_base::evidence::Outcome;
use truck_geometry::decorators::{Processor, TrimmedCurve};
use truck_geometry::specifieds::{Line, UnitCircle, UnitHyperbola, UnitParabola};

/// A full circle placed in space: the trimmed unit circle under an affine
/// placement. This is deliberately the same representation channel as
/// truck-geometry's canonical `Curve::Circle` variant, so an exact circle from
/// an analytic cell converts to the kernel's carrier enum without re-encoding.
pub type PlacedCircle = Processor<TrimmedCurve<UnitCircle<Point3>>, Matrix4>;

/// A parabola placed in space: the trimmed unit parabola under an affine
/// placement.
pub type PlacedParabola = Processor<TrimmedCurve<UnitParabola<Point3>>, Matrix4>;

/// One branch of a hyperbola placed in space: the trimmed unit hyperbola under
/// an affine placement.
pub type PlacedHyperbola = Processor<TrimmedCurve<UnitHyperbola<Point3>>, Matrix4>;

/// An exactly parameterized intersection curve (BG-ANA-001).
///
/// The parameterization is derived from the carrier parameters by rational
/// operations; there is no `τ_rep` and no float-certified path here (H-6). An
/// analytic cell that cannot fill one of these arms honestly must refuse, not
/// approximate.
///
/// `Circle` and `Ellipse` share a payload type because an ellipse *is* the
/// unit circle under a non-conformal placement; they remain distinct arms
/// because the classification is the deliverable — the spec's table
/// distinguishes "circle" from "ellipse" and "two ellipses", and BG-ANA-002
/// classification predicates key off the arm.
#[derive(Clone, Debug)]
pub enum ExactCurve {
    /// A straight line, given by two points on it.
    Line(Line<Point3>),
    /// A full circle.
    Circle(PlacedCircle),
    /// An ellipse: an affine image of the unit circle.
    Ellipse(PlacedCircle),
    /// A parabola.
    Parabola(PlacedParabola),
    /// One branch of a hyperbola.
    Hyperbola(PlacedHyperbola),
}

/// The result of an exactly-solved surface pair (BG-ANA-001): an exact curve,
/// or a typed classification of the degenerate position (BG-ANA-002).
///
/// The degenerate arms are not error codes; they are the *answer*. Two
/// parallel planes have no intersection line, and saying so by type is the
/// certified outcome — decided by exact predicates on the carrier parameters,
/// never by sampling. An arm is only returned when its predicate is decided;
/// an undecidable predicate (its interval enclosure straddles the threshold)
/// is a `Refusal::NumericallyUnresolved`, never a confident guess.
#[derive(Clone, Debug)]
pub enum AnalyticIntersection {
    /// One exact curve: the transverse case.
    Curve(ExactCurve),
    /// Two exact curves: the plane × cylinder line pair, the equal-radius
    /// cylinders' two ellipses, the coaxial circle pair.
    TwoCurves([ExactCurve; 2]),
    /// A family of exact curves whose *count* is itself data: the coaxial
    /// cylinder × torus and sphere × torus families can meet in up to four
    /// circles, decided by the same discriminant predicates. Use `Curve` /
    /// `TwoCurves` when the pair family fixes the count; this arm is for when
    /// the count is part of the answer.
    Curves(Vec<ExactCurve>),
    /// Degenerate: the pair is tangent at a single point
    /// (plane × sphere, sphere × sphere, a plane meeting a cone at its apex).
    TangentPoint(Point3),
    /// Degenerate: the pair is tangent along a whole line
    /// (plane × cylinder, tangent parallel-axis cylinders, a plane tangent to
    /// a cone along a generator).
    TangentLine(Line<Point3>),
    /// Degenerate: the pair is tangent along a whole circle — a sphere or
    /// torus tangent to a coaxial cylinder or cone (the counterbore and
    /// fillet families BG-ANA-002 names). The circle is the entire contact
    /// locus; distinguishing it from a transverse single circle is the
    /// tangency classification, decided by the same exact discriminant
    /// predicates that count the transverse circles.
    TangentCircle(PlacedCircle),
    /// Degenerate: a parallel placement with no intersection — parallel
    /// planes, or parallel axes too far apart to touch. The parallelism is
    /// the classification, and it is exact.
    Parallel,
    /// Degenerate: the carriers coincide in a surface, not a curve — equal
    /// planes, identical spheres, same-radius coaxial cylinders. The
    /// intersection is two-dimensional and outside this track's contract.
    Coincident,
    /// No intersection, and no degeneracy in the placement: the carriers are
    /// transversally positioned and simply do not meet.
    Empty,
}

/// What every analytic pair family returns (BG-ANA-001).
///
/// Constructing the `Certificate` inside the `Ok` remains **explicit
/// field-by-field at every site** (`method: Method::Exact` included), exactly
/// as `truck_base::evidence` demands: this module deliberately provides no
/// convenience constructor, so "exact" cannot be manufactured casually
/// (BG-EVD-002).
pub type AnalyticOutcome = Outcome<AnalyticIntersection>;

/// BG-ANA-001-COAX: coaxial pairs. Scaffolded empty; the packet fills it.
pub mod coaxial;
/// BG-ANA-001-EQRCYL: equal-radius cylinders with intersecting axes.
/// Scaffolded empty; the packet fills it.
pub mod equal_radius_cylinders;
/// BG-ANA-001-PARCYL: parallel-axis cylinders. Scaffolded empty; the packet
/// fills it.
pub mod parallel_cylinders;
/// BG-ANA-001-PCONE: plane × cone. Scaffolded empty; the packet fills it.
pub mod plane_cone;
/// BG-ANA-001-PCYL: plane × cylinder. Scaffolded empty; the packet fills it.
pub mod plane_cylinder;
/// BG-ANA-001-PP: plane × plane. Scaffolded empty; the packet fills it.
pub mod plane_plane;
/// BG-ANA-001-PS: plane × sphere. Scaffolded empty; the packet fills it.
pub mod plane_sphere;
/// BG-ANA-001-SS: sphere × sphere. Scaffolded empty; the packet fills it.
pub mod sphere_sphere;

#[cfg(test)]
mod tests {
    use super::*;
    use truck_base::cgmath64::EuclideanSpace;
    use truck_geotrait::ParametricCurve;

    /// Scales the trimmed unit conic by `(a, b, 1)` and translates it to
    /// `center`, in the `z = 0` plane. Affine, `w = 1` exactly.
    fn placed(a: f64, b: f64, center: Point3) -> PlacedCircle {
        let trimmed = TrimmedCurve::new(UnitCircle::<Point3>::new(), (0.0, std::f64::consts::TAU));
        let matrix =
            Matrix4::from_translation(center.to_vec()) * Matrix4::from_nonuniform_scale(a, b, 1.0);
        Processor::with_transform(trimmed, matrix)
    }

    #[test]
    fn circle_arm_evaluates_on_the_expected_circle() {
        // Dyadic values: the t = 0 point is computed exactly.
        let center = Point3::new(1.0, 2.0, 4.0);
        let circle = ExactCurve::Circle(placed(3.0, 3.0, center));
        let ExactCurve::Circle(curve) = &circle else {
            unreachable!("just constructed this arm");
        };
        assert_eq!(curve.subs(0.0), Point3::new(4.0, 2.0, 4.0));
        // TrimmedCurve does not remap: subs takes the angle itself.
        let opposite = curve.subs(std::f64::consts::PI);
        // H-3: dimensionless slack on unit-scale coordinates, from cos(PI) not being exactly -1.0
        assert!((opposite.x + 2.0).abs() < 1.0e-12, "{opposite:?}");
        // H-3: dimensionless slack on unit-scale coordinates, from sin(PI) not being exactly 0.0
        assert!((opposite.y - 2.0).abs() < 1.0e-12, "{opposite:?}");
    }

    #[test]
    fn ellipse_arm_evaluates_with_distinct_semi_axes() {
        let center = Point3::origin();
        let ellipse = ExactCurve::Ellipse(placed(4.0, 2.0, center));
        let ExactCurve::Ellipse(curve) = &ellipse else {
            unreachable!("just constructed this arm");
        };
        assert_eq!(curve.subs(0.0), Point3::new(4.0, 0.0, 0.0));
        let top = curve.subs(std::f64::consts::FRAC_PI_2);
        // H-3: dimensionless slack on unit-scale coordinates, from cos(PI/2) not being exactly 0.0
        assert!(top.x.abs() < 1.0e-12, "{top:?}");
        // H-3: dimensionless slack on unit-scale coordinates, sin(PI/2) is exactly 1.0
        assert!((top.y - 2.0).abs() < 1.0e-12, "{top:?}");
    }

    #[test]
    fn degenerate_arms_are_the_classification() {
        // Every arm is constructible and distinguishable by `matches!`; the
        // classification is the deliverable, so the arms must stay distinct.
        let line = Line(Point3::origin(), Point3::new(0.0, 0.0, 1.0));
        let arms = [
            AnalyticIntersection::Curve(ExactCurve::Line(line)),
            AnalyticIntersection::TwoCurves([
                ExactCurve::Line(line),
                ExactCurve::Line(Line(Point3::origin(), Point3::new(1.0, 0.0, 0.0))),
            ]),
            AnalyticIntersection::Curves(vec![
                ExactCurve::Line(line),
                ExactCurve::Line(Line(Point3::origin(), Point3::new(1.0, 0.0, 0.0))),
                ExactCurve::Line(Line(Point3::origin(), Point3::new(0.0, 1.0, 0.0))),
            ]),
            AnalyticIntersection::TangentPoint(Point3::origin()),
            AnalyticIntersection::TangentLine(line),
            AnalyticIntersection::TangentCircle(placed(1.0, 1.0, Point3::origin())),
            AnalyticIntersection::Parallel,
            AnalyticIntersection::Coincident,
            AnalyticIntersection::Empty,
        ];
        for (i, arm) in arms.iter().enumerate() {
            for (j, other) in arms.iter().enumerate() {
                // Debug formatting is discriminative: same arm pairs agree,
                // distinct arms differ.
                if i == j {
                    assert_eq!(format!("{arm:?}"), format!("{other:?}"));
                } else {
                    assert_ne!(format!("{arm:?}"), format!("{other:?}"), "arms {i} and {j}");
                }
            }
        }
    }
}
