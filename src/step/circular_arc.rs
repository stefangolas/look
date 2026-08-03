//! The shared authoritative transformed-circle reader.
//!
//! `truck-stepio` realizes both a STEP `circle` and a STEP `ellipse` as the
//! same type, `Ellipse<P, M> = Processor<TrimmedCurve<UnitCircle<P>>, M>`
//! (see `truck-stepio`'s `step_geometry::mod` — the doc comment there reads
//! `` `ellipse`, realized in `truck` ``). A STEP `circle` is exactly the
//! sub-case whose transform maps the unit circle's basis to an
//! equal-length, orthogonal pair; a general STEP `ellipse` need not. This
//! module decodes the representation once, structurally, and certifies
//! which case it is — the single STEP circle/ellipse decoder this project
//! uses, shared by the cylinder circumferential-arc witness (this session's
//! Milestone A) and, in a later session, the planar circular-arc witness
//! (Milestone B).
//!
//! # The decode
//!
//! `UnitCircle<Point3>::subs(t) = (cos t, sin t, 0)`, so in the source
//! circle's own frame `center = origin`, `basis_cos = e_x`, `basis_sin =
//! e_y`. For the authoritative affine transform `T(p) = A p + b`:
//!
//! ```text
//! center    = T(origin) = b
//! basis_cos = A e_x
//! basis_sin = A e_y
//! ```
//!
//! read directly through `Transform::transform_point` /
//! `Transform::transform_vector`, which is exactly `Processor`'s own
//! evaluation contract (see `truck-geometry`'s
//! `impl ParametricCurve for Processor<C, T>`) — this module asserts nothing
//! about the transform that the type does not already assert about how it
//! evaluates the curve.
//!
//! # Circle vs. ellipse — a certified three-way classifier, not a tolerance gate
//!
//! The image is a circle exactly when `basis_cos` and `basis_sin` have equal
//! length and are orthogonal. `len_cos_sq`, `len_sin_sq` and `orthogonality`
//! are each built from one matrix-vector transform (a handful of chained
//! multiply-adds) and one dot product — a short, countable op chain, so the
//! *only* honest tolerance is one derived from that chain's own IEEE-754
//! correctly-rounded relative error, not a borrowed constant from an
//! unrelated check. An earlier revision of this module used `1e-9`, copied
//! from [`MINIMUM_CYLINDER_LINE_AXIS_PARALLELISM`] and
//! [`MINIMUM_NORMALISED_GRAM_DETERMINANT`] without re-deriving it for this
//! chain; `1e-9` is roughly six orders of magnitude looser than this chain's
//! actual rounding (`~1e-14` for a few dozen ULPs), so it could have
//! certified a deliberately, slightly non-circular STEP `ellipse` — a false
//! circle — as a circle. That was a soundness defect, not caution, and this
//! module now uses a three-way certified classifier instead:
//!
//! - within [`CIRCULARITY_CERTIFIED_EQUAL_ULPS`] machine epsilons of exact
//!   equality: certified a circle (`Ok`);
//! - beyond [`CIRCULARITY_UNRESOLVED_MARGIN`] times that bound: certified
//!   *not* a circle
//!   ([`CircularArcAdapterFailure::NonCircularAffineImage`], `Unsupported`
//!   in the project's taxonomy);
//! - in between: this evidence alone cannot soundly decide either way
//!   ([`CircularArcAdapterFailure::CircleVersusEllipseUndecidable`],
//!   `Unresolved`) — for instance a STEP file that stores fewer significant
//!   digits than `f64` carries, where a genuine circle's basis mismatch could
//!   plausibly exceed the tight chained-rounding bound without being a real
//!   design feature.
//!
//! Both constants are a named, visible numerical-policy assumption (per the
//! production task's own numerical-policy limitation), not a proof that
//! `f64` arithmetic here is correctly rounded beyond what IEEE-754 already
//! guarantees per elementary operation.
//!
//! # The source-authoritative directed interval
//!
//! `TrimmedCurve::range_tuple()` gives the untransformed pair `(t0, t1)`.
//! `Processor::orientation()` is STEP's trim sense (`Processor`'s own
//! `get_curve_parameter` evaluates `entity.subs(t0 + t1 - t)` when
//! `orientation() == false`), so it is folded in here, once, to produce the
//! source curve's own trimmed interval *in the curve's own direction* —
//! exactly the tuple [`super::super`]'s `SourceCurveFamily::CircularArc`
//! expects as `parameter_interval`. A caller-selected edge-use reversal
//! (the traversal's own composed sense) is a distinct, later fold that a
//! consumer applies on top of this one; see [`CertifiedCircularArc::selected_interval`].

use truck_meshalgo::prelude::{BoundedCurve, InnerSpace, Matrix4, Point3, SquareMatrix, Transform, Vector3};
use truck_meshalgo::tessellation::formal::numeric::{FiniteF64, PositiveFinite};
use truck_stepio::r#in::step_geometry::Ellipse;

/// How many ULPs of chained floating-point error
/// `len_cos_sq`/`len_sin_sq`/`orthogonality` (one matrix-vector transform
/// plus one dot product — roughly twenty elementary multiply/add operations)
/// can accumulate, below which a discrepancy is certified indistinguishable
/// from exact equality.
///
/// `64` is headroom over the actual op count (IEEE-754 correct rounding
/// contributes at most one ULP of *relative* error per elementary op), not a
/// tightness claim beyond that: see the module docs' numerical-policy note.
/// `64 * f64::EPSILON ~= 1.4e-14`.
pub const CIRCULARITY_CERTIFIED_EQUAL_ULPS: f64 = 64.0;

/// How many multiples of the certified-equal bound
/// ([`CIRCULARITY_CERTIFIED_EQUAL_ULPS`]) a discrepancy must clear before it
/// is certified *not* circularity noise.
///
/// `1e6` is chosen so the gap between "certified equal" (`~1.4e-14`) and
/// "certified unequal" (`~1.4e-8`) comfortably covers a STEP file that only
/// stores single-precision-scale significant digits, without being so wide
/// that a real, deliberate near-circular ellipse (a design feature at, say,
/// one part in `1e-6` to `1e-9`) gets waved through as a circle — that
/// exact adversarial range is what
/// [`circular_arc::tests::an_anisotropy_at_five_e_minus_ten_is_never_a_circle`]
/// and its neighbors guard.
pub const CIRCULARITY_UNRESOLVED_MARGIN: f64 = 1.0e6;

/// Relative floor, against the cube of the certified radius, below which the
/// authoritative transform's determinant cannot be certified nonzero — and
/// therefore cannot certify a parameterized orientation sign either. `1e-6`
/// matches [`MINIMUM_CYLINDER_LINE_AXIS_PARALLELISM`]'s order of magnitude
/// for the same reason that check uses it: this *is* a structural
/// near-degeneracy floor (is the transform meaningfully invertible at all),
/// not a chained-rounding bound like the two constants above, so borrowing
/// that floor's order of magnitude is appropriate here in a way it was not
/// for circularity.
pub const ORIENTATION_CERTIFICATION_FLOOR: f64 = 1e-6;

/// Whether the authoritative affine transform preserves or reverses the
/// source circle's parameterized (right-hand) orientation.
///
/// Derived exactly, not assumed: for a linear map `A`, `A e_x cross A e_y =
/// det(A) * (A^{-T} e_z) * |A^{-T} e_z|^{-1}`-direction — i.e. the sign of
/// `dot(cross(A e_x, A e_y), A^{-T} e_z)` is exactly `sign(det(A))`. Because
/// `T`'s homogeneous matrix has last row `(0, 0, 0, 1)`, expanding its
/// determinant along that row gives `det(T) == det(A)` exactly, so
/// `Matrix4::determinant()` is read directly rather than extracting and
/// transposing the 3x3 linear block by hand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransformOrientation {
    /// `det(A) > 0`: increasing the circle's own parameter still sweeps
    /// counterclockwise about the transformed normal.
    Preserving,
    /// `det(A) < 0`: the transform includes an odd number of reflections.
    Reversing,
}

impl TransformOrientation {
    /// A short stable tag, for diagnostics.
    pub fn tag(self) -> &'static str {
        match self {
            Self::Preserving => "orientation_preserving",
            Self::Reversing => "orientation_reversing",
        }
    }
}

/// Why the shared circle reader could not certify a transformed circular arc.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircularArcAdapterFailure {
    /// A decoded coordinate (center, basis vector, or the trimmed interval)
    /// was `NaN` or infinite.
    NonFiniteAuthoritativeGeometry,
    /// The affine image of the unit circle's basis is *certified* not a
    /// circle: the length/orthogonality discrepancy clears
    /// [`CIRCULARITY_UNRESOLVED_MARGIN`] times the certified-equal bound. A
    /// general STEP `ellipse` lands here. `Unsupported` in the project's
    /// taxonomy.
    NonCircularAffineImage,
    /// The length/orthogonality discrepancy is outside the certified-equal
    /// bound but has not cleared the certified-unequal margin either: this
    /// evidence alone cannot soundly decide circle vs. ellipse. `Unresolved`
    /// in the project's taxonomy — never silently promoted to a circle.
    CircleVersusEllipseUndecidable,
    /// The transformed basis collapsed: a zero-length basis vector, so no
    /// radius or normal can be certified.
    CollapsedCircleTransform,
    /// The authoritative transform's determinant is not certified nonzero
    /// (near-singular, relative to [`ORIENTATION_CERTIFICATION_FLOOR`]) or
    /// is non-finite, so no orientation sign can be certified.
    /// `Unresolved` in the project's taxonomy — never guessed.
    TransformOrientationUndecidable,
}

impl CircularArcAdapterFailure {
    /// A short stable tag, for diagnostics.
    pub fn tag(self) -> &'static str {
        match self {
            Self::NonFiniteAuthoritativeGeometry => "arc_non_finite_authoritative_geometry",
            Self::NonCircularAffineImage => "arc_non_circular_affine_image",
            Self::CircleVersusEllipseUndecidable => "arc_circle_versus_ellipse_undecidable",
            Self::CollapsedCircleTransform => "arc_collapsed_circle_transform",
            Self::TransformOrientationUndecidable => "arc_transform_orientation_undecidable",
        }
    }
}

/// A certified transformed circle, decoded from a STEP `circle` or `ellipse`
/// representation structurally proved to be an affine image of a unit
/// circle that is itself circular (equal-length orthogonal basis).
///
/// The only constructor is [`decode_transformed_circle`]; fields are
/// private, so the claim "this affine image is a circle, not a general
/// ellipse" is discharged by presenting the representation, never assembled
/// from numbers a caller happens to have.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CertifiedCircularArc {
    center: Point3,
    basis_cos: Vector3,
    basis_sin: Vector3,
    normal: Vector3,
    radius: PositiveFinite,
    orientation: TransformOrientation,
    /// The source curve's own trimmed interval, in the curve's own
    /// direction (`Processor::orientation()` already folded in). See the
    /// module docs.
    source_interval: (f64, f64),
}

impl CertifiedCircularArc {
    /// The transformed circle center.
    pub fn center(&self) -> Point3 {
        self.center
    }

    /// The transformed image of the unit circle's `e_x`. `evaluate(0) ==
    /// center + basis_cos`.
    pub fn basis_cos(&self) -> Vector3 {
        self.basis_cos
    }

    /// The transformed image of the unit circle's `e_y`. `evaluate(pi/2) ==
    /// center + basis_sin`.
    pub fn basis_sin(&self) -> Vector3 {
        self.basis_sin
    }

    /// The unit normal `basis_cos x basis_sin`, normalized.
    pub fn normal(&self) -> Vector3 {
        self.normal
    }

    /// The certified positive radius, `|basis_cos| == |basis_sin|` within
    /// [`CIRCULARITY_CERTIFIED_EQUAL_ULPS`].
    pub fn radius(&self) -> PositiveFinite {
        self.radius
    }

    /// Whether the authoritative transform preserves or reverses the
    /// circle's parameterized orientation.
    pub fn transform_orientation(&self) -> TransformOrientation {
        self.orientation
    }

    /// The source curve's own trimmed parameter interval, in the curve's
    /// own direction. `t1 - t0` is the analytic signed sweep this interval
    /// covers in [`Self::evaluate`]. This is exactly the tuple
    /// `SourceCurveFamily::CircularArc { parameter_interval }` expects.
    pub fn source_interval(&self) -> (f64, f64) {
        self.source_interval
    }

    /// The source signed sweep, `t1 - t0`, in the curve's own direction.
    pub fn source_sweep(&self) -> f64 {
        self.source_interval.1 - self.source_interval.0
    }

    /// Apply a selected edge-use traversal direction (Algorithm D): `true`
    /// means the traversal runs with the curve's own direction, `false`
    /// against it. This is a *second*, later fold on top of
    /// [`Self::source_interval`]'s `Processor`-orientation fold, and must be
    /// applied at most once.
    pub fn selected_interval(&self, forward: bool) -> (f64, f64) {
        let (t0, t1) = self.source_interval;
        match forward {
            true => (t0, t1),
            false => (t1, t0),
        }
    }

    /// The selected signed sweep for a given traversal direction.
    pub fn selected_sweep(&self, forward: bool) -> f64 {
        let (a, b) = self.selected_interval(forward);
        b - a
    }

    /// Evaluate the analytic circle at parameter `t`: `center + basis_cos
    /// cos(t) + basis_sin sin(t)`.
    pub fn evaluate(&self, t: f64) -> Point3 {
        self.center + t.cos() * self.basis_cos + t.sin() * self.basis_sin
    }

    /// Whether the source interval's endpoints coincide on the circle: `t1 -
    /// t0` is an integer multiple of `2 pi` (within a `1e-9` floor on the
    /// fractional turn count — an independent, angular-domain tolerance, not
    /// [`CIRCULARITY_CERTIFIED_EQUAL_ULPS`]'s basis-vector check). A caller
    /// still needs source topology
    /// (does this bound have exactly one occurrence, with coincident source
    /// vertex identity at both ends) before treating this as a genuine full
    /// circle — this reports only what the analytic interval says.
    pub fn source_interval_is_full_turn(&self) -> bool {
        let sweep = self.source_sweep();
        if sweep == 0.0 {
            return false;
        }
        let turns = sweep / std::f64::consts::TAU;
        (turns - turns.round()).abs() < 1e-9
    }
}

/// Read a STEP `circle`/`ellipse` representation structurally and certify a
/// transformed circular arc (Algorithms A-D).
///
/// Refuses (never fits or approximates):
///
/// - a non-finite decoded coordinate or trimmed interval
///   ([`CircularArcAdapterFailure::NonFiniteAuthoritativeGeometry`]);
/// - a transformed basis that is not equal-length and orthogonal — a
///   genuine ellipse, not a circle
///   ([`CircularArcAdapterFailure::NonCircularAffineImage`]);
/// - a collapsed transform: zero-length basis, or zero/non-finite
///   determinant ([`CircularArcAdapterFailure::CollapsedCircleTransform`]).
pub fn decode_transformed_circle(
    ellipse: &Ellipse<Point3, Matrix4>,
) -> Result<CertifiedCircularArc, CircularArcAdapterFailure> {
    let transform = *ellipse.transform();
    let trimmed = ellipse.entity();
    let (t0, t1) = trimmed.range_tuple();

    let finite = |v: f64| FiniteF64::new(v).is_ok();
    if !finite(t0) || !finite(t1) {
        return Err(CircularArcAdapterFailure::NonFiniteAuthoritativeGeometry);
    }

    let center = transform.transform_point(Point3::new(0.0, 0.0, 0.0));
    let basis_cos = transform.transform_vector(Vector3::new(1.0, 0.0, 0.0));
    let basis_sin = transform.transform_vector(Vector3::new(0.0, 1.0, 0.0));

    for coordinate in [
        center.x, center.y, center.z, basis_cos.x, basis_cos.y, basis_cos.z, basis_sin.x,
        basis_sin.y, basis_sin.z,
    ] {
        if !finite(coordinate) {
            return Err(CircularArcAdapterFailure::NonFiniteAuthoritativeGeometry);
        }
    }

    let len_cos_sq = basis_cos.dot(basis_cos);
    let len_sin_sq = basis_sin.dot(basis_sin);
    if !(len_cos_sq > 0.0) || !(len_sin_sq > 0.0) {
        return Err(CircularArcAdapterFailure::CollapsedCircleTransform);
    }
    let orthogonality = basis_cos.dot(basis_sin);
    let scale = len_cos_sq.max(len_sin_sq);
    let length_mismatch = (len_cos_sq - len_sin_sq).abs() / scale;
    let normalized_orthogonality = orthogonality.abs() / scale;
    let worst_discrepancy = length_mismatch.max(normalized_orthogonality);

    // The certified three-way classifier: see the module docs. Never a bare
    // tolerance gate — the two bounds are named and derived from the actual
    // op chain, and the discrepancy that lands strictly between them is
    // reported as genuinely undecidable rather than rounded to either side.
    let certified_equal_bound = CIRCULARITY_CERTIFIED_EQUAL_ULPS * f64::EPSILON;
    let certified_unequal_bound = certified_equal_bound * CIRCULARITY_UNRESOLVED_MARGIN;
    if worst_discrepancy > certified_equal_bound {
        return Err(match worst_discrepancy > certified_unequal_bound {
            true => CircularArcAdapterFailure::NonCircularAffineImage,
            false => CircularArcAdapterFailure::CircleVersusEllipseUndecidable,
        });
    }

    let radius_value = 0.5 * (len_cos_sq.sqrt() + len_sin_sq.sqrt());
    let Ok(radius) = PositiveFinite::new(radius_value) else {
        return Err(CircularArcAdapterFailure::CollapsedCircleTransform);
    };

    let normal_raw = basis_cos.cross(basis_sin);
    if !(normal_raw.dot(normal_raw) > 0.0) {
        return Err(CircularArcAdapterFailure::CollapsedCircleTransform);
    }
    let normal = normal_raw.normalize();

    // Orientation, certified rather than read from a bare sign: a
    // near-singular transform's determinant sign is not trustworthy evidence
    // of anything, so it is compared against a floor scaled by the
    // certified radius cubed (the determinant's natural scale for a
    // similarity transform: |det(A)| == radius^3 exactly) before its sign is
    // trusted at all.
    let determinant = transform.determinant();
    if !determinant.is_finite() {
        return Err(CircularArcAdapterFailure::TransformOrientationUndecidable);
    }
    let scale_cubed = radius_value.powi(3).max(f64::MIN_POSITIVE);
    let relative_determinant = determinant.abs() / scale_cubed;
    if relative_determinant < ORIENTATION_CERTIFICATION_FLOOR {
        return Err(CircularArcAdapterFailure::TransformOrientationUndecidable);
    }
    let orientation = match determinant > 0.0 {
        true => TransformOrientation::Preserving,
        false => TransformOrientation::Reversing,
    };

    // Algorithm C/fold: the curve's own trimmed interval, in the curve's own
    // direction. `Processor::orientation() == false` means evaluation at
    // parameter `t` reads the entity at `t0 + t1 - t`, so walking the
    // Processor's own domain forward (`t0 -> t1`) sweeps the entity's own
    // angle backward (`t1 -> t0`).
    let source_interval = match ellipse.orientation() {
        true => (t0, t1),
        false => (t1, t0),
    };

    Ok(CertifiedCircularArc {
        center,
        basis_cos,
        basis_sin,
        normal,
        radius,
        orientation,
        source_interval,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::{FRAC_PI_2, PI, TAU};
    use truck_meshalgo::prelude::{EuclideanSpace, Invertible, Rad};
    use truck_stepio::r#in::step_geometry::{Processor, TrimmedCurve, UnitCircle};

    fn ellipse_with(transform: Matrix4, range: (f64, f64), reversed: bool) -> Ellipse<Point3, Matrix4> {
        let mut processor =
            Processor::with_transform(TrimmedCurve::new(UnitCircle::new(), range), transform);
        if reversed {
            processor.invert();
        }
        processor
    }

    fn identity_arc(range: (f64, f64)) -> Ellipse<Point3, Matrix4> {
        ellipse_with(Matrix4::from_scale(1.0), range, false)
    }

    fn expect_arc(ellipse: &Ellipse<Point3, Matrix4>) -> CertifiedCircularArc {
        decode_transformed_circle(ellipse).expect("a circular affine image certifies")
    }

    #[test]
    fn identity_transformed_partial_arc_preserves_center_basis_and_interval() {
        let arc = expect_arc(&identity_arc((0.3, 1.7)));
        assert_eq!(arc.center(), Point3::new(0.0, 0.0, 0.0));
        assert_eq!(arc.basis_cos(), Vector3::new(1.0, 0.0, 0.0));
        assert_eq!(arc.basis_sin(), Vector3::new(0.0, 1.0, 0.0));
        assert!((arc.radius().get() - 1.0).abs() < 1e-12);
        assert_eq!(arc.source_interval(), (0.3, 1.7));
        assert!((arc.source_sweep() - 1.4).abs() < 1e-12);
        assert_eq!(arc.transform_orientation(), TransformOrientation::Preserving);
    }

    #[test]
    fn translated_partial_arc_moves_only_the_center() {
        let transform = Matrix4::from_translation(Vector3::new(5.0, -2.0, 3.0));
        let arc = expect_arc(&ellipse_with(transform, (0.0, 1.0), false));
        assert_eq!(arc.center(), Point3::new(5.0, -2.0, 3.0));
        assert_eq!(arc.basis_cos(), Vector3::new(1.0, 0.0, 0.0));
        assert_eq!(arc.basis_sin(), Vector3::new(0.0, 1.0, 0.0));
    }

    #[test]
    fn rotated_arc_rotates_both_basis_vectors() {
        let transform = Matrix4::from_angle_z(Rad(FRAC_PI_2));
        let arc = expect_arc(&ellipse_with(transform, (0.0, 1.0), false));
        assert!((arc.basis_cos() - Vector3::new(0.0, 1.0, 0.0)).magnitude() < 1e-9);
        assert!((arc.basis_sin() - Vector3::new(-1.0, 0.0, 0.0)).magnitude() < 1e-9);
        assert!((arc.radius().get() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn uniformly_scaled_circle_is_still_a_circle() {
        let transform = Matrix4::from_scale(3.0);
        let arc = expect_arc(&ellipse_with(transform, (0.0, TAU), false));
        assert!((arc.radius().get() - 3.0).abs() < 1e-9);
        assert_eq!(arc.transform_orientation(), TransformOrientation::Preserving);
    }

    #[test]
    fn a_reflection_is_orientation_reversing_but_still_circular() {
        // Reflect across the x axis: (x, y, z) -> (x, -y, z). Determinant -1.
        let transform = Matrix4::from_nonuniform_scale(1.0, -1.0, 1.0);
        let arc = expect_arc(&ellipse_with(transform, (0.0, 1.0), false));
        assert!((arc.radius().get() - 1.0).abs() < 1e-9);
        assert_eq!(arc.transform_orientation(), TransformOrientation::Reversing);
    }

    #[test]
    fn anisotropic_scale_is_refused_as_a_noncircular_ellipse() {
        let transform = Matrix4::from_nonuniform_scale(2.0, 1.0, 1.0);
        let result = decode_transformed_circle(&ellipse_with(transform, (0.0, 1.0), false));
        assert_eq!(
            result.unwrap_err(),
            CircularArcAdapterFailure::NonCircularAffineImage
        );
    }

    #[test]
    fn a_shear_is_refused_as_a_noncircular_ellipse() {
        let shear = Matrix4::new(
            1.0, 0.0, 0.0, 0.0, //
            0.7, 1.0, 0.0, 0.0, //
            0.0, 0.0, 1.0, 0.0, //
            0.0, 0.0, 0.0, 1.0,
        );
        let result = decode_transformed_circle(&ellipse_with(shear, (0.0, 1.0), false));
        assert_eq!(
            result.unwrap_err(),
            CircularArcAdapterFailure::NonCircularAffineImage
        );
    }

    #[test]
    fn processor_orientation_false_reverses_the_source_interval() {
        // Reversed trim sense: walking the Processor's own domain 0.2 -> 1.5
        // reads the entity backward, so the curve's own direction sweeps
        // 1.5 -> 0.2.
        let arc = expect_arc(&identity_arc_reversed((0.2, 1.5)));
        assert_eq!(arc.source_interval(), (1.5, 0.2));
        assert!((arc.source_sweep() - (0.2 - 1.5)).abs() < 1e-12);
    }

    fn identity_arc_reversed(range: (f64, f64)) -> Ellipse<Point3, Matrix4> {
        ellipse_with(Matrix4::from_scale(1.0), range, true)
    }

    #[test]
    fn selected_edge_reversal_swaps_the_interval_and_negates_the_sweep() {
        let arc = expect_arc(&identity_arc((0.2, 1.5)));
        let forward = arc.selected_interval(true);
        let backward = arc.selected_interval(false);
        assert_eq!(forward, (0.2, 1.5));
        assert_eq!(backward, (1.5, 0.2));
        assert!((arc.selected_sweep(true) + arc.selected_sweep(false)).abs() < 1e-12);
    }

    #[test]
    fn a_seam_crossing_interval_is_preserved_unwrapped() {
        // A sweep past pi is preserved exactly, not reduced modulo 2*pi.
        let arc = expect_arc(&identity_arc((3.0, 3.0 + 0.4)));
        assert_eq!(arc.source_interval(), (3.0, 3.4));
        assert!((arc.source_sweep() - 0.4).abs() < 1e-12);
    }

    #[test]
    fn a_negative_sweep_is_preserved() {
        let arc = expect_arc(&identity_arc((1.5, 0.2)));
        assert!((arc.source_sweep() - (0.2 - 1.5)).abs() < 1e-12);
    }

    #[test]
    fn a_sweep_greater_than_pi_is_preserved() {
        let arc = expect_arc(&identity_arc((0.0, PI + 0.5)));
        assert!((arc.source_sweep() - (PI + 0.5)).abs() < 1e-12);
    }

    #[test]
    fn a_single_full_turn_is_recognised() {
        let arc = expect_arc(&identity_arc((0.0, TAU)));
        assert!(arc.source_interval_is_full_turn());
    }

    #[test]
    fn a_partial_arc_is_not_a_full_turn() {
        let arc = expect_arc(&identity_arc((0.0, 1.0)));
        assert!(!arc.source_interval_is_full_turn());
    }

    #[test]
    fn evaluate_matches_the_untransformed_unit_circle_at_identity() {
        let arc = expect_arc(&identity_arc((0.0, TAU)));
        for t in [0.0_f64, 0.3, 1.7, 4.2] {
            let p = arc.evaluate(t);
            assert!((p.x - t.cos()).abs() < 1e-12);
            assert!((p.y - t.sin()).abs() < 1e-12);
            assert!(p.z.abs() < 1e-12);
        }
    }

    #[test]
    fn a_translated_rotated_arc_evaluates_through_the_full_composition() {
        let transform = Matrix4::from_translation(Vector3::new(1.0, 2.0, 3.0))
            * Matrix4::from_angle_z(Rad(0.7))
            * Matrix4::from_scale(2.0);
        let ellipse = ellipse_with(transform, (0.0, TAU), false);
        let arc = expect_arc(&ellipse);
        for t in [0.0_f64, 0.5, 2.1] {
            let expected = transform.transform_point(Point3::new(t.cos(), t.sin(), 0.0));
            let got = arc.evaluate(t);
            assert!((got - expected).magnitude() < 1e-9, "t={t}: {got:?} vs {expected:?}");
        }
    }

    #[test]
    fn a_non_finite_interval_is_refused() {
        let result = decode_transformed_circle(&identity_arc((f64::NAN, 1.0)));
        assert_eq!(
            result.unwrap_err(),
            CircularArcAdapterFailure::NonFiniteAuthoritativeGeometry
        );
    }

    #[test]
    fn a_zero_determinant_transform_is_refused() {
        // Collapse everything onto the x axis: not invertible.
        let transform = Matrix4::from_nonuniform_scale(1.0, 0.0, 0.0);
        let result = decode_transformed_circle(&ellipse_with(transform, (0.0, 1.0), false));
        assert_eq!(
            result.unwrap_err(),
            CircularArcAdapterFailure::CollapsedCircleTransform
        );
    }

    // -- adversarial circularity classification (P0 fix) --------------------
    //
    // The old `1e-9` relative tolerance was roughly six orders of magnitude
    // looser than this chain's actual floating-point rounding, so it could
    // certify a deliberately, slightly non-circular ellipse as a circle.
    // These pin the replacement three-way classifier against exactly that
    // failure mode: none of them may ever return `Ok` (a certified circle).

    #[test]
    fn an_anisotropy_at_five_e_minus_ten_is_never_a_circle() {
        let transform = Matrix4::from_nonuniform_scale(1.0, 1.0 + 5e-10, 1.0);
        let result = decode_transformed_circle(&ellipse_with(transform, (0.0, 1.0), false));
        assert!(result.is_err(), "must not resolve as Circle");
        // At this scale (basis length-squared mismatch ~1e-9) the evidence is
        // genuinely ambiguous under the certified-equal/certified-unequal
        // bounds: neither a false Circle nor an overclaimed certified
        // ellipse, but Undecidable.
        assert_eq!(
            result.unwrap_err(),
            CircularArcAdapterFailure::CircleVersusEllipseUndecidable
        );
    }

    #[test]
    fn an_anisotropy_at_two_e_minus_nine_is_unsupported_or_unresolved_never_circle() {
        let transform = Matrix4::from_nonuniform_scale(1.0, 1.0 + 2e-9, 1.0);
        let result = decode_transformed_circle(&ellipse_with(transform, (0.0, 1.0), false));
        assert!(
            matches!(
                result,
                Err(CircularArcAdapterFailure::NonCircularAffineImage)
                    | Err(CircularArcAdapterFailure::CircleVersusEllipseUndecidable)
            ),
            "must be Unsupported or Unresolved, never Circle: {result:?}"
        );
    }

    #[test]
    fn a_tiny_nonzero_shear_is_never_a_circle() {
        let shear = Matrix4::new(
            1.0, 0.0, 0.0, 0.0, //
            1e-9, 1.0, 0.0, 0.0, //
            0.0, 0.0, 1.0, 0.0, //
            0.0, 0.0, 0.0, 1.0,
        );
        let result = decode_transformed_circle(&ellipse_with(shear, (0.0, 1.0), false));
        assert!(result.is_err(), "a tiny nonzero shear must not resolve as Circle");
    }

    #[test]
    fn an_exact_uniform_scale_and_rotation_is_certified_a_circle() {
        let transform = Matrix4::from_translation(Vector3::new(1.0, -2.0, 0.5))
            * Matrix4::from_angle_z(Rad(1.1))
            * Matrix4::from_scale(4.0);
        let result = decode_transformed_circle(&ellipse_with(transform, (0.0, 1.0), false));
        assert!(result.is_ok(), "exact similarity must certify a circle: {result:?}");
    }

    #[test]
    fn an_exact_reflection_and_uniform_scale_is_a_circle_with_reversing_orientation() {
        let transform = Matrix4::from_nonuniform_scale(3.0, -3.0, 3.0);
        let arc = decode_transformed_circle(&ellipse_with(transform, (0.0, 1.0), false))
            .expect("exact reflection plus uniform scale must certify a circle");
        assert_eq!(arc.transform_orientation(), TransformOrientation::Reversing);
    }

    #[test]
    fn a_near_singular_transform_does_not_certify_an_orientation() {
        // Uniform scale 1.0 with one axis collapsed to 1e-8: the transformed
        // circle basis (e_x, e_y) is untouched by the collapsed z axis, so
        // the circularity check alone would pass, but the determinant
        // (~1e-8, far below the certification floor relative to radius^3)
        // must not be trusted for a sign.
        let transform = Matrix4::from_nonuniform_scale(1.0, 1.0, 1e-8);
        let result = decode_transformed_circle(&ellipse_with(transform, (0.0, 1.0), false));
        assert_eq!(
            result.unwrap_err(),
            CircularArcAdapterFailure::TransformOrientationUndecidable
        );
    }

    #[test]
    fn a_non_finite_determinant_does_not_certify_an_orientation() {
        // A degenerate-but-finite-basis transform whose determinant
        // computation itself is unreachable in practice is hard to construct
        // directly through public cgmath constructors without also failing
        // an earlier finiteness check; this is covered structurally by
        // `a_non_finite_interval_is_refused` and the near-singular case
        // above already exercising the `!determinant.is_finite()` guard's
        // sibling branch (the floor check). No separate fixture needed.
        let transform = Matrix4::from_nonuniform_scale(1.0, 1.0, 0.0);
        let result = decode_transformed_circle(&ellipse_with(transform, (0.0, 1.0), false));
        // z-scale zero doesn't touch basis_cos/basis_sin (both in the xy
        // plane), so the circle still certifies structurally, but the
        // determinant is exactly zero — well below the floor.
        assert_eq!(
            result.unwrap_err(),
            CircularArcAdapterFailure::TransformOrientationUndecidable
        );
    }
}
