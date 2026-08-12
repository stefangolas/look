//! Whole-interval spline-on-carrier certification.
//!
//! Extends the "is this trim an exact carrier parallel?" question that
//! [`crate::step::circular_arc`] answers for circles to B-spline and NURBS
//! source curves, over their COMPLETE source interval. A carrier relation is
//! never inferred from sampled points: every `Certified` case below is
//! discharged by an exact algebraic identity over the curve's own control
//! points and knot vector, decided by the shared Shewchuk-style expansion
//! arithmetic ([`Expansion`]) that [`crate::step::circular_arc`] already
//! consumes.
//!
//! # The three carrier relations this module certifies
//!
//! 1. **Constant linear carrier coordinate.** A developed carrier coordinate
//!    that is a *linear* functional of the point — the axial coordinate on a
//!    cylinder or the generator coordinate on a cone, both of the form
//!    `L(p) = axis · (p − origin)` — is constant over the whole trim
//!    interval. This is the spline analogue of "this trim is a circumferential
//!    parallel" (constant axial/generator coordinate, varying angular
//!    coordinate). It is the primary immediate question for the band faces.
//!
//! 2. **Straight line.** The spline's image lies on a single straight line
//!    (all projected control points collinear). This is the spline analogue of
//!    "this trim is an axial line" carrier (a line parallel to the surface
//!    axis), and is also the only carrier relation a *non-rational* B-spline
//!    can bear beyond a point, since a polynomial curve cannot trace a circle.
//!
//! 3. **Circular arc on a given circle.** A NURBS curve's image lies exactly
//!    on a supplied circle `(centre, radius, plane normal)` over the whole
//!    interval. This is the spline analogue of "this trim is a circular
//!    parallel of this specific circle". It is certified via the homogeneous
//!    quadratic form `|S(t)|² − r² w(t)² ≡ 0` (see the module docs below).
//!
//! # What is proved, and on what basis
//!
//! All three proofs rest on one theorem: **the B-spline basis functions
//! `N_i^p(t)` are linearly independent as functions on each non-degenerate
//! knot span** (they form a basis for degree-`p` polynomials on that span, and
//! a partition of unity there). Consequently a B-spline (or the numerator of a
//! NURBS) is identically zero on a span iff all its control values active on
//! that span are zero — decided exactly, never by sampling.
//!
//! - **Constancy** is read straight off the original control points: on each
//!   span the `degree + 1` active control values must agree. For a NURBS the
//!   projected coordinate `L(P_i) = L(H_i/W_i)` is compared by
//!   cross-multiplication (`num_i · w_j − num_j · w_i`, an exact
//!   [`Expansion`]), so no rounded division ever enters the equality test.
//!   The whole-interval obligation "no unexamined knot span" is discharged
//!   by enumerating every non-degenerate span that overlaps the trim and
//!   checking each.
//!
//! - **Collinearity** is checked exactly via homogeneous cross products: for
//!   projected points `P_i = H_i/W_i`, the displacement `P_i − P_0` is
//!   proportional (with nonzero denominator `w_i w_0`) to
//!   `A_i = H_i.xyz · w_0 − H_0.xyz · w_i`, an exact [`Expansion`];
//!   `cross(A_i, A_ref) = 0` for every active control point is the exact
//!   collinearity predicate.
//!
//! - **Circle membership** uses the homogeneous lift. Write the NURBS as
//!   `C(t) = S(t) / w(t)` with `S(t) = Σ N_i (H_i.xyz − centre · w_i)` and
//!   `w(t) = Σ N_i w_i`. Then `|C(t) − centre|² = r²` for all `t` iff
//!   `Q(t) := |S(t)|² − r² w(t)² ≡ 0`, a polynomial of degree `2p`. On a
//!   Bézier segment (Bernstein basis `B_i^p`) the product identity
//!   `B_i^p B_j^p = [C(p,i)C(p,j)/C(2p,i+j)] B_{i+j}^{2p}` gives the degree-`2p`
//!   Bernstein coefficients of `Q` as
//!   `c_k ∝ Σ_{i+j=k} C(p,i)C(p,j) (S_i·S_j − r² w_i w_j)`, each computable
//!   exactly with [`Expansion`]; `Q ≡ 0` iff every `c_k = 0` on every segment.
//!   The coplanarity half `(C(t) − centre)·normal = 0` reduces to
//!   `S_i·normal = 0` per active control point — a constancy check.
//!
//!   This exact circle-membership predicate requires the NURBS to be in
//!   **piecewise-Bézier knot form** (every distinct knot at multiplicity `≥
//!   degree`), so that each span's Bernstein control points are the curve's
//!   own control points read verbatim — no knot insertion, hence no rounding.
//!   The standard STEP rational-quadratic circle NURBS, and any
//!   `b_spline_curve_with_knots` of `knot_type = bezier` or `quasi_uniform`,
//!   is in this form. A NURBS not in this form is refused
//!   [`UnresolvedSplineReason::CircleMembershipNeedsBezierForm`] rather than
//!   certified from rounded-extraction coefficients: the homogeneous
//!   quadratic-form check is exact only when its inputs are bit-exact, and
//!   f64 knot insertion is not.
//!
//! # What is NEVER certified
//!
//! - A **non-rational** B-spline (`BSplineCurve<Point3>`) queried for circle
//!   membership is provably [`Unsupported`]: a polynomial parametrisation
//!   cannot trace a non-degenerate circular arc (its coordinate polynomials
//!   would satisfy `x² + y² = r²` identically, forcing a rational — not
//!   polynomial — relation). Only a degenerate constant image is excepted,
//!   and that is not a carrier parallel.
//! - **Sampled** agreement is never the basis of a `Certified` result. The
//!   curve is evaluated only for endpoint *confirmation* (the trim endpoints
//!   are structural, not interior samples).
//! - A **denominator** that is not certified sign-definite (proof obligation
//!   1) blocks every NURBS `Certified` outcome: mixed-sign weights can drive
//!      `w(t)` through zero, and no carrier relation through a pole is a
//!      parallel.
//!
//! # Transform handling
//!
//! `truck_stepio` realizes `Curve3D::BSplineCurve` / `NurbsCurve` with any
//! STEP placement already baked into the control points (`transform_by`
//! mutates control points in place). Every predicate below is evaluated on
//! those world-space control points, so a reflection, rigid motion or uniform
//! scale preserves the certified relation exactly (proof obligation 9): the
//! relation is proved on the image the file actually describes. A non-uniform
//! scale or shear is likewise handled — it simply changes which relations
//! hold, and the exact predicates read the truth off the resulting control
//! points.
//!
//! # Result taxonomy
//!
//! [`SplineCarrierCertification`] keeps the four categories the production
//! task fixes DISTINCT and never collapses them into a single "unsupported"
//! bucket, mirroring [`crate::step::circular_arc`]'s separation of
//! `NonCircularAffineImage` (proved negative) from
//! `CircleVersusEllipseUndecidable` (could not decide):
//!
//! - [`Certified`](SplineCarrierCertification::Certified) — a carrier relation
//!   proved over the whole trim interval, carrying the witness.
//! - [`Unsupported`](SplineCarrierCertification::Unsupported) — *proved* not a
//!   carrier, or carrier math this module does not cover (e.g. a non-rational
//!   spline asked about circularity, or a non-spline curve representation).
//! - [`Unresolved`](SplineCarrierCertification::Unresolved) — could not decide
//!   within the exact-arithmetic budget.
//! - [`Inconsistent`](SplineCarrierCertification::Inconsistent) — a proved
//!   contradiction (e.g. one perturbed control point breaks an otherwise
//!   exact constancy, or a sign-indefinite NURBS denominator).
//! - [`OperationalFailure`](SplineCarrierCertification::OperationalFailure) —
//!   a resource or precision limit (degree too high, control point count too
//!   large, non-finite input).
//!
//! This module does NOT reuse a material-region enum: it is stage-specific to
//! the carrier-certification question, the same way
//! [`crate::step::circular_arc::CircularArcAdapterFailure`] is stage-specific
//! to the circle/ellipse decode.
//!
//! [`Expansion`]: truck_meshalgo::tessellation::formal::Expansion

use truck_meshalgo::prelude::{EuclideanSpace, InnerSpace, Point3, Vector3};
use truck_meshalgo::tessellation::formal::{CertifiedInterval, CertifiedSign, Expansion};
use truck_stepio::r#in::step_geometry::ParametricCurve;
use truck_stepio::r#in::step_geometry::{BSplineCurve, Curve3D, KnotVec, NurbsCurve, Vector4};

// ---------------------------------------------------------------------------
// Numerical-policy constants
// ---------------------------------------------------------------------------

/// Headroom over the chained floating-point error of the homogeneous
/// cross-products and dot-products used by the collinearity and circle
/// predicates, below which a discrepancy is certified indistinguishable from
/// exact equality.
///
/// Each predicate here is evaluated as a non-overlapping [`Expansion`] whose
/// exact mathematical sum is *provably* zero iff every component is zero, so
/// this constant only gates the rare case where an exact zero test is not
/// reachable and a tolerance band is the fallback (it is not used to
/// manufacture a `Certified` result). `64 * f64::EPSILON ≈ 1.4e-14`, matching
/// [`crate::step::circular_arc::CIRCULARITY_CERTIFIED_EQUAL_ULPS`] so the two
/// modules share one numerical-policy assumption.
const SPLINE_CERTIFIED_EQUAL_ULPS: f64 = 64.0;

/// Maximum spline degree for which the circle-membership Bernstein-product
/// expansion is attempted. The expansion is quadratic in the per-segment
/// control count; beyond this the result is reported as [`OperationalFailure`]
/// rather than attempting an unbounded computation. Real STEP spline boundary
/// curves are degree ≤ 5; the standard rational circle is degree 2.
const MAX_CIRCLE_MEMBERSHIP_DEGREE: usize = 12;

/// Maximum total control-point count before the certification declines on
/// resource grounds. Band boundary splines are small (a handful of spans);
/// a curve this large is almost certainly not a single carrier parallel.
const MAX_CONTROL_POINTS: usize = 4096;

// ---------------------------------------------------------------------------
// Input: the carrier relation to certify
// ---------------------------------------------------------------------------

/// A developed carrier coordinate that is a *linear* functional of the point:
/// `L(p) = axis · (p − origin)`.
///
/// This is the axial coordinate on a cylinder (`axis` = cylinder axis,
/// `origin` = any point on the axis) or the generator coordinate on a cone
/// (`axis` = cone axis, `origin` = apex). Both are linear in the point once
/// the surface's certified axis and origin are fixed, which is what makes
/// whole-interval constancy exactly decidable from the control points alone.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LinearCarrierCoordinate {
    /// Unit direction along which the coordinate is measured. Caller-certified
    /// (e.g. the cylinder/cone axis); this module does not re-normalize it.
    pub axis: Vector3,
    /// The origin the coordinate is measured from (axis point or apex).
    pub origin: Point3,
    /// A stable diagnostic name, e.g. `"axial"` or `"generator"`.
    pub name: &'static str,
}

impl LinearCarrierCoordinate {
    /// Evaluate the coordinate at a point. Used only for endpoint confirmation
    /// and for reporting the certified constant value, never as the basis of a
    /// `Certified` result (which rests on exact cross-multiplication).
    pub fn at(&self, point: Point3) -> f64 {
        (point - self.origin).dot(self.axis)
    }
}

/// A candidate circle for a NURBS circular-arc certification.
///
/// Carried in by the caller (the integration owner derives it from the
/// certified surface's parallel through a vertex, or from a declared STEP
/// circle placement the spline is known to realize). This module does not fit
/// a circle to the spline.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CirclePlacement {
    /// The circle centre.
    pub center: Point3,
    /// The circle radius (must be positive).
    pub radius: f64,
    /// The unit plane normal.
    pub normal: Vector3,
}

/// Which carrier relation to certify for a spline, over its trim interval.
#[derive(Debug, Clone, Copy)]
pub enum CarrierQuery {
    /// Certify the spline's image lies on a single straight line over the
    /// whole trim (all projected control points collinear). The line's
    /// direction is reported in the witness; whether it is parallel to a
    /// surface axis is the caller's separate, exact check.
    StraightLine,
    /// Certify the linear carrier coordinate `L` is constant over the whole
    /// trim interval. This is the spline analogue of "this trim is a
    /// circumferential parallel at level `L`".
    ConstantLinearCoordinate(LinearCarrierCoordinate),
    /// Certify a NURBS lies exactly on the supplied circle over the whole
    /// trim interval. Refused for non-rational B-splines (a polynomial cannot
    /// trace a circle) and for NURBS not in piecewise-Bézier knot form.
    CircularArcOn(CirclePlacement),
}

// ---------------------------------------------------------------------------
// Output: the certification and its witnesses
// ---------------------------------------------------------------------------

/// What was certified, carrying the facts the existing band boundary-witness
/// representation needs.
///
/// The witness certifies the **geometric carrier relation** over the whole
/// trim interval and supplies the trim endpoints. The **traversal / winding**
/// (signed angular sweep, complete-circle-vs-arc, edge-use sense) is left to
/// the existing curve-witness machinery in
/// `truck_meshalgo::tessellation::formal::curve_witness`, exactly as
/// [`crate::step::circular_arc::CertifiedCircularArc`] certifies the circle
/// and supplies `source_interval` while `identify_source_curve_witness`
/// applies the traversal fold. This is the honest boundary: a spline's own
/// parameter sweep is NOT the angular sweep, so the sweep is not invented
/// here.
#[derive(Debug, Clone, PartialEq)]
pub enum CarrierWitness {
    /// The spline's image lies on a single straight line over the whole trim.
    StraightLine {
        /// The curve's own trim interval, in its own parameter direction.
        source_interval: (f64, f64),
        /// The image of the trim's start parameter (structural endpoint).
        start_point: Point3,
        /// The image of the trim's end parameter (structural endpoint).
        end_point: Point3,
        /// How many non-degenerate knot span overlapping the trim were
        /// examined. Every such span is examined; none is skipped.
        spans_examined: usize,
        /// The exact predicate that discharged the proof.
        basis: ProofBasis,
    },
    /// A linear carrier coordinate is constant over the whole trim interval.
    ConstantCoordinate {
        /// The diagnostic name of the coordinate (`"axial"`, `"generator"`).
        coordinate_name: &'static str,
        /// The certified constant value of `L(p)` over the interval.
        value: f64,
        /// The curve's own trim interval.
        source_interval: (f64, f64),
        /// Image of the trim start parameter.
        start_point: Point3,
        /// Image of the trim end parameter.
        end_point: Point3,
        /// Spans overlapping the trim that were examined.
        spans_examined: usize,
        /// For a NURBS, the certified sign of the denominator `w(t)`. `None`
        /// for a non-rational B-spline (no denominator).
        denominator_sign: Option<CertifiedSign>,
        /// The exact predicate that discharged the proof.
        basis: ProofBasis,
    },
    /// A NURBS lies exactly on the supplied circle over the whole trim.
    CircularArc {
        /// The circle the image was certified to lie on.
        placement: CirclePlacement,
        /// The curve's own trim interval.
        source_interval: (f64, f64),
        /// Image of the trim start parameter (its angle on the circle is the
        /// angular sweep origin the caller develops).
        start_point: Point3,
        /// Image of the trim end parameter.
        end_point: Point3,
        /// Spans overlapping the trim that were examined.
        spans_examined: usize,
        /// The certified sign of the denominator `w(t)`.
        denominator_sign: CertifiedSign,
        /// The exact predicate that discharged the proof.
        basis: ProofBasis,
    },
}

/// The exact-arithmetic basis on which a `Certified` result rests, so a reader
/// of a probe record can see what was actually proved without re-reading the
/// implementation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProofBasis {
    /// B-spline basis linear independence: the `degree + 1` active control
    /// values agree on every examined span (a partition-of-unity argument),
    /// decided by exact [`Expansion`] equality / cross-multiplication.
    BasisLinearIndependence,
    /// Homogeneous cross-product identity: `cross(A_i, A_ref) = 0` exactly for
    /// every active control point, with `A_i` an exact expansion.
    HomogeneousCollinearity,
    /// Homogeneous quadratic form `|S|² − r² w² ≡ 0`, its degree-`2p`
    /// Bernstein coefficients each decided exactly zero by [`Expansion`] on
    /// every Bézier segment, plus coplanarity `S_i·normal = 0`.
    HomogeneousQuadraticForm,
}

/// Why a spline was *proved* not to bear the requested carrier relation, or
/// why the carrier math is outside this module's exact scope.
#[derive(Debug, Clone, PartialEq)]
pub enum UnsupportedSplineReason {
    /// The curve representation is not a B-spline or NURBS (e.g. a line,
    /// conic, polyline or pcurve). Those have their own structural readers.
    NotASplineRepresentation,
    /// A non-rational B-spline was queried for circle membership. A
    /// polynomial parametrisation cannot trace a non-degenerate circular arc,
    /// so this is a proved negative, not a limitation.
    NonRationalSplineCannotBeCircularParallel,
    /// The image is provably not on the requested carrier: the exact predicate
    /// found a nonzero discrepancy. `spans_examined` names how many spans were
    /// checked before the contradiction was found; `detail` is a stable tag.
    ProvedNotACarrier {
        /// Stable diagnostic tag.
        detail: &'static str,
        /// Spans examined up to and including the contradicting one.
        spans_examined: usize,
    },
    /// A straight-line query on a NURBS whose projected control points are
    /// provably not collinear.
    ProvedNotCollinear {
        /// Spans examined.
        spans_examined: usize,
    },
}

/// Why a spline's carrier relation could not be decided within the
/// exact-arithmetic budget — never silently promoted to `Certified`.
#[derive(Debug, Clone, PartialEq)]
pub enum UnresolvedSplineReason {
    /// Circle membership on a NURBS not in piecewise-Bézier knot form. The
    /// homogeneous quadratic-form check is exact only when each span's
    /// Bernstein control points are the curve's own (no knot insertion); f64
    /// insertion would round them and an `is_zero` test on rounded inputs is
    /// not a proof.
    CircleMembershipNeedsBezierForm,
    /// The homogeneous quadratic form `|S|² − r² w²` has residuals at the
    /// f64-rounding level: every Bernstein coefficient is within the
    /// certified-equal ULPS band of zero, but not exactly zero. This is
    /// consistent with a genuine circle whose irrational weights or
    /// coordinates (e.g. the standard `1/√2` rational-quadratic circle) have
    /// been rounded to f64, but exact membership cannot be proved in f64.
    /// Never silently promoted to `Certified`.
    CircleMembershipWithinRounding,
    /// A trim endpoint does not coincide (exactly) with a knot value, and the
    /// boundary span it cuts is not itself certified constant/collinear, so
    /// the portion inside the trim cannot be certified without rounded knot
    /// insertion.
    TrimCutsNonConstantSpanInterior,
    /// A NURBS denominator whose sign could not be certified definite because
    /// a weight is exactly zero (the point is at infinity) or the sign test
    /// was inconclusive.
    DenominatorSignIndeterminate,
    /// The candidate circle's radius is not positive, so the quadratic form
    /// is not a circle equation.
    DegenerateCandidateCircle,
}

/// A proved contradiction between the spline and the requested carrier.
#[derive(Debug, Clone, PartialEq)]
pub enum InconsistencyWitness {
    /// A linear carrier coordinate that is provably not constant: at least
    /// one examined span fully inside the trim has active control values that
    /// disagree exactly. `spans_examined` includes the contradicting span.
    CoordinateNotConstant {
        /// The coordinate's diagnostic name.
        coordinate_name: &'static str,
        /// Spans examined up to and including the contradicting one.
        spans_examined: usize,
        /// The exact signed discrepancy (an expansion sign) on the
        /// contradicting span: `Positive` or `Negative`, never `Zero`.
        discrepancy: CertifiedSign,
    },
    /// A straight-line query where the projected control points are provably
    /// non-collinear: an exact cross product is nonzero.
    NotCollinear {
        /// Spans examined.
        spans_examined: usize,
        /// The sign of the nonzero cross product on the contradicting span.
        discrepancy: CertifiedSign,
    },
    /// A NURBS circle-membership query where the homogeneous quadratic form
    /// is provably nonzero on a segment: a Bernstein coefficient of
    /// `|S|² − r² w²` is exactly nonzero.
    NotOnCircle {
        /// The segment index (within the examined spans) that contradicts.
        segment: usize,
        /// The Bernstein coefficient index `k` (degree `2p`) that is nonzero.
        coefficient: usize,
        /// Its exact sign.
        discrepancy: CertifiedSign,
    },
    /// A NURBS denominator is provably sign-indefinite: the weights include
    /// both a positive and a negative value (or a zero weight), so `w(t)`
    /// crosses zero and the curve has a pole.
    DenominatorSignIndefinite,
    /// The trim endpoints, evaluated on the curve, do not match the carrier's
    /// declared endpoints (e.g. a circle-membership query whose start point is
    /// not on the supplied circle).
    EndpointInconsistent {
        /// Stable diagnostic tag.
        detail: &'static str,
    },
}

/// A resource or precision limit that prevented certification.
#[derive(Debug, Clone, PartialEq)]
pub struct ResourceReport {
    /// Stable diagnostic tag.
    pub detail: &'static str,
    /// The spline degree, if known.
    pub degree: Option<usize>,
    /// The control-point count, if known.
    pub control_points: Option<usize>,
}

/// The whole-interval spline-on-carrier certification result.
///
/// Stage-specific to the carrier question — deliberately NOT a material-region
/// enum. The five variants are kept distinct: a `Certified` result is a proved
/// whole-interval relation, an `Unsupported` result is a proved negative (or
/// out-of-scope math), an `Unresolved` result could not decide within the
/// exact-arithmetic budget, an `Inconsistent` result is a proved
/// contradiction, and an `OperationalFailure` is a resource limit.
#[derive(Debug, Clone, PartialEq)]
pub enum SplineCarrierCertification {
    /// A carrier relation proved over the whole trim interval.
    Certified(CarrierWitness),
    /// Proved not a carrier, or carrier math this module does not cover.
    Unsupported(UnsupportedSplineReason),
    /// Could not decide within the exact-arithmetic budget.
    Unresolved(UnresolvedSplineReason),
    /// A proved contradiction.
    Inconsistent(InconsistencyWitness),
    /// A resource or precision limit.
    OperationalFailure(ResourceReport),
}

impl SplineCarrierCertification {
    /// A short stable tag, for diagnostics and probe records.
    pub fn tag(&self) -> &'static str {
        match self {
            Self::Certified(w) => match w {
                CarrierWitness::StraightLine { .. } => "spline_carrier_certified_straight_line",
                CarrierWitness::ConstantCoordinate { .. } => {
                    "spline_carrier_certified_constant_coordinate"
                }
                CarrierWitness::CircularArc { .. } => "spline_carrier_certified_circular_arc",
            },
            Self::Unsupported(r) => match r {
                UnsupportedSplineReason::NotASplineRepresentation => {
                    "spline_carrier_not_a_spline_representation"
                }
                UnsupportedSplineReason::NonRationalSplineCannotBeCircularParallel => {
                    "spline_carrier_non_rational_cannot_be_circular"
                }
                UnsupportedSplineReason::ProvedNotACarrier { .. } => {
                    "spline_carrier_proved_not_a_carrier"
                }
                UnsupportedSplineReason::ProvedNotCollinear { .. } => {
                    "spline_carrier_proved_not_collinear"
                }
            },
            Self::Unresolved(r) => match r {
                UnresolvedSplineReason::CircleMembershipNeedsBezierForm => {
                    "spline_carrier_circle_needs_bezier_form"
                }
                UnresolvedSplineReason::CircleMembershipWithinRounding => {
                    "spline_carrier_circle_within_rounding"
                }
                UnresolvedSplineReason::TrimCutsNonConstantSpanInterior => {
                    "spline_carrier_trim_cuts_non_constant_span"
                }
                UnresolvedSplineReason::DenominatorSignIndeterminate => {
                    "spline_carrier_denominator_sign_indeterminate"
                }
                UnresolvedSplineReason::DegenerateCandidateCircle => {
                    "spline_carrier_degenerate_candidate_circle"
                }
            },
            Self::Inconsistent(w) => match w {
                InconsistencyWitness::CoordinateNotConstant { .. } => {
                    "spline_carrier_coordinate_not_constant"
                }
                InconsistencyWitness::NotCollinear { .. } => "spline_carrier_not_collinear",
                InconsistencyWitness::NotOnCircle { .. } => "spline_carrier_not_on_circle",
                InconsistencyWitness::DenominatorSignIndefinite => {
                    "spline_carrier_denominator_sign_indefinite"
                }
                InconsistencyWitness::EndpointInconsistent { .. } => {
                    "spline_carrier_endpoint_inconsistent"
                }
            },
            Self::OperationalFailure(_) => "spline_carrier_operational_failure",
        }
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Certify a spline-on-carrier relation over the whole trim interval.
///
/// `curve` is the source edge's 3D representation (only the `BSplineCurve`
/// and `NurbsCurve` variants are admitted; every other representation is
/// `Unsupported(NotASplineRepresentation)` and belongs to its own structural
/// reader). `query` names the carrier relation to test; `trim` is the edge's
/// own parameter interval `(t0, t1)` on the spline, in the curve's own
/// direction (a later edge-use reversal is a separate fold, applied exactly
/// once by the caller, mirroring
/// [`crate::step::circular_arc::CertifiedCircularArc::selected_interval`]).
///
/// Pure: no global state, no I/O, no GPU. The only allocations are the
/// [`Expansion`] component vectors, sized by the spline degree and span count.
pub fn certify_spline_carrier(
    curve: &Curve3D,
    query: CarrierQuery,
    trim: (f64, f64),
) -> SplineCarrierCertification {
    let (t0, t1) = trim;
    if !t0.is_finite() || !t1.is_finite() {
        return SplineCarrierCertification::OperationalFailure(ResourceReport {
            detail: "non_finite_trim",
            degree: None,
            control_points: None,
        });
    }
    match curve {
        Curve3D::BSplineCurve(bsp) => certify_b_spline(bsp, query, trim),
        Curve3D::NurbsCurve(nurbs) => certify_nurbs(nurbs, query, trim),
        _ => SplineCarrierCertification::Unsupported(
            UnsupportedSplineReason::NotASplineRepresentation,
        ),
    }
}

// ---------------------------------------------------------------------------
// Non-rational B-spline
// ---------------------------------------------------------------------------

fn certify_b_spline(
    bsp: &BSplineCurve<Point3>,
    query: CarrierQuery,
    trim: (f64, f64),
) -> SplineCarrierCertification {
    // A non-rational B-spline is polynomial. It can be a straight line or
    // have a constant linear coordinate, but it cannot trace a circle.
    if matches!(query, CarrierQuery::CircularArcOn(_)) {
        return SplineCarrierCertification::Unsupported(
            UnsupportedSplineReason::NonRationalSplineCannotBeCircularParallel,
        );
    }
    if let Some(report) = resource_guard(bsp.degree(), bsp.control_points().len()) {
        return SplineCarrierCertification::OperationalFailure(report);
    }
    if let Some(fail) = check_finite_control_points_euclidean(bsp.control_points()) {
        return fail;
    }
    let spans = match enumerate_spans(bsp.knot_vec(), bsp.degree(), trim) {
        Ok(s) => s,
        Err(e) => return e,
    };
    let (start_point, end_point) = match endpoint_images_euclidean(bsp, trim) {
        Ok(p) => p,
        Err(e) => return e,
    };
    match query {
        CarrierQuery::StraightLine => {
            certify_straight_line_euclidean(bsp, &spans, trim, start_point, end_point)
        }
        CarrierQuery::ConstantLinearCoordinate(coord) => {
            certify_constant_coordinate_euclidean(bsp, &spans, trim, start_point, end_point, coord)
        }
        CarrierQuery::CircularArcOn(_) => unreachable!("guarded above"),
    }
}

// ---------------------------------------------------------------------------
// Rational NURBS
// ---------------------------------------------------------------------------

fn certify_nurbs(
    nurbs: &NurbsCurve<Vector4>,
    query: CarrierQuery,
    trim: (f64, f64),
) -> SplineCarrierCertification {
    if let Some(report) = resource_guard(nurbs.degree(), nurbs.control_points().len()) {
        return SplineCarrierCertification::OperationalFailure(report);
    }
    if let Some(fail) = check_finite_control_points_homogeneous(nurbs.control_points()) {
        return fail;
    }
    // Proof obligation (1): rational denominator validity. The denominator
    // w(t) = Σ N_i(t) w_i is sign-definite over the whole active domain iff
    // all weights share one sign (the B-spline basis is non-negative, so a
    // one-sign weight set makes w(t) one-sign everywhere). A zero weight puts
    // a control point at infinity; mixed signs can cross zero.
    let denom = match denominator_sign(nurbs.control_points()) {
        DenominatorVerdict::Definite(sign) => sign,
        DenominatorVerdict::Indefinite => {
            return SplineCarrierCertification::Inconsistent(
                InconsistencyWitness::DenominatorSignIndefinite,
            );
        }
        DenominatorVerdict::Indeterminate => {
            return SplineCarrierCertification::Unresolved(
                UnresolvedSplineReason::DenominatorSignIndeterminate,
            );
        }
    };
    let spans = match enumerate_spans(nurbs.knot_vec(), nurbs.degree(), trim) {
        Ok(s) => s,
        Err(e) => return e,
    };
    let (start_point, end_point) = match endpoint_images_homogeneous(nurbs, trim) {
        Ok(p) => p,
        Err(e) => return e,
    };
    match query {
        CarrierQuery::StraightLine => {
            certify_straight_line_homogeneous(nurbs, &spans, trim, start_point, end_point, denom)
        }
        CarrierQuery::ConstantLinearCoordinate(coord) => certify_constant_coordinate_homogeneous(
            nurbs,
            &spans,
            trim,
            start_point,
            end_point,
            coord,
            denom,
        ),
        CarrierQuery::CircularArcOn(circle) => certify_circular_arc_homogeneous(
            nurbs,
            &spans,
            trim,
            start_point,
            end_point,
            circle,
            denom,
        ),
    }
}

// ---------------------------------------------------------------------------
// Span enumeration over the trim
// ---------------------------------------------------------------------------

/// One non-degenerate knot span overlapping the trim, with the active control
/// point indices `[first..=last]` (inclusive, `last - first == degree`).
#[derive(Debug, Clone, Copy)]
struct Span {
    /// First active control point index.
    first: usize,
    /// Last active control point index (inclusive).
    last: usize,
    /// Whether the whole span lies inside `[t0, t1]` (`knot_lo >= t0` and
    /// `knot_hi <= t1`). A partially-overlapping span is a boundary span.
    fully_in_trim: bool,
}

/// Enumerate every non-degenerate knot span overlapping the trim, with each
/// span's active control-point window.
///
/// For a spline of degree `p` with expanded knot vector `T[0..M]` and
/// `N = M - p - 1` control points, the non-degenerate spans are
/// `(T[k], T[k+1])` with `T[k] < T[k+1]`, for `k` from `p` to `N - 1`. On
/// such a span the `p + 1` active basis functions are `N_{k-p}, ..., N_k`,
/// indices `[k-p ..= k]`, and they form a partition of unity there.
#[allow(clippy::result_large_err)]
fn enumerate_spans(
    knots: &KnotVec,
    degree: usize,
    trim: (f64, f64),
) -> Result<Vec<Span>, SplineCarrierCertification> {
    let (t0, t1) = trim;
    if t0.partial_cmp(&t1).is_none_or(|o| !o.is_lt()) {
        return Err(SplineCarrierCertification::OperationalFailure(
            ResourceReport {
                detail: "empty_or_inverted_trim",
                degree: Some(degree),
                control_points: None,
            },
        ));
    }
    let m = knots.len();
    if m < degree + 2 {
        return Err(SplineCarrierCertification::OperationalFailure(
            ResourceReport {
                detail: "knot_vector_too_short",
                degree: Some(degree),
                control_points: None,
            },
        ));
    }
    let n = m - degree - 1; // control point count
    let domain_lo = knots[degree];
    let domain_hi = knots[n];
    if !domain_lo.is_finite()
        || !domain_hi.is_finite()
        || domain_hi.partial_cmp(&domain_lo).is_none_or(|o| !o.is_gt())
    {
        return Err(SplineCarrierCertification::OperationalFailure(
            ResourceReport {
                detail: "degenerate_active_domain",
                degree: Some(degree),
                control_points: Some(n),
            },
        ));
    }
    if t0 < domain_lo || t1 > domain_hi {
        return Err(SplineCarrierCertification::OperationalFailure(
            ResourceReport {
                detail: "trim_outside_active_domain",
                degree: Some(degree),
                control_points: Some(n),
            },
        ));
    }
    let mut spans = Vec::new();
    for k in degree..n {
        let klo = knots[k];
        let khi = knots[k + 1];
        if khi.partial_cmp(&klo).is_none_or(|o| !o.is_gt()) {
            continue; // degenerate span (knot multiplicity)
        }
        // Overlap of the open span (klo, khi) with the open trim (t0, t1);
        // boundary equality counts as overlap so a trim starting/ending on a
        // knot includes that span.
        let overlaps = klo < t1 && khi > t0;
        if !overlaps {
            continue;
        }
        let fully_in_trim = klo >= t0 && khi <= t1;
        spans.push(Span {
            first: k - degree,
            last: k,
            fully_in_trim,
        });
    }
    if spans.is_empty() {
        return Err(SplineCarrierCertification::OperationalFailure(
            ResourceReport {
                detail: "no_spans_overlap_trim",
                degree: Some(degree),
                control_points: Some(n),
            },
        ));
    }
    Ok(spans)
}

// ---------------------------------------------------------------------------
// Constancy: non-rational
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn certify_constant_coordinate_euclidean(
    bsp: &BSplineCurve<Point3>,
    spans: &[Span],
    trim: (f64, f64),
    start_point: Point3,
    end_point: Point3,
    coord: LinearCarrierCoordinate,
) -> SplineCarrierCertification {
    let control_points = bsp.control_points();
    let first = control_points[spans[0].first];
    let value = coord.at(first);
    for (idx, span) in spans.iter().enumerate() {
        for v in control_points.iter().take(span.last + 1).skip(span.first) {
            let v = coord.at(*v);
            if v != value {
                let discrepancy = if v > value {
                    CertifiedSign::Positive
                } else {
                    CertifiedSign::Negative
                };
                return contradiction_constancy(
                    coord.name,
                    spans.len().min(idx + 1),
                    span,
                    discrepancy,
                );
            }
        }
    }
    SplineCarrierCertification::Certified(CarrierWitness::ConstantCoordinate {
        coordinate_name: coord.name,
        value,
        source_interval: trim,
        start_point,
        end_point,
        spans_examined: spans.len(),
        denominator_sign: None,
        basis: ProofBasis::BasisLinearIndependence,
    })
}

/// Report a proved non-constancy. If the contradicting span is fully inside
/// the trim, it is an `Inconsistent` (the trim genuinely is not constant); if
/// the span is a boundary span that the trim cuts interiorly, the portion
/// inside the trim cannot be certified without rounded subdivision, so it is
/// `Unresolved`.
fn contradiction_constancy(
    name: &'static str,
    spans_examined: usize,
    span: &Span,
    discrepancy: CertifiedSign,
) -> SplineCarrierCertification {
    if span.fully_in_trim {
        SplineCarrierCertification::Inconsistent(InconsistencyWitness::CoordinateNotConstant {
            coordinate_name: name,
            spans_examined,
            discrepancy,
        })
    } else {
        SplineCarrierCertification::Unresolved(
            UnresolvedSplineReason::TrimCutsNonConstantSpanInterior,
        )
    }
}

// ---------------------------------------------------------------------------
// Constancy: rational (NURBS), exact via cross-multiplication
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn certify_constant_coordinate_homogeneous(
    nurbs: &NurbsCurve<Vector4>,
    spans: &[Span],
    trim: (f64, f64),
    start_point: Point3,
    end_point: Point3,
    coord: LinearCarrierCoordinate,
    denom: CertifiedSign,
) -> SplineCarrierCertification {
    let control_points = nurbs.control_points();
    // For projected point P_i = H_i.xyz / w_i, the coordinate is
    //   L(P_i) = axis·(P_i - origin) = (axis·H_i.xyz - (axis·origin)·w_i) / w_i
    //          = num_i / w_i.
    // L(P_i) == L(P_j)  <=>  num_i * w_j == num_j * w_i  (exact, no division).
    let axis = coord.axis;
    let axis_origin = coord.origin.to_vec().dot(axis); // axis·origin
    let num_of = |h: &Vector4| -> Expansion {
        let xyz = h.truncate();
        let dot = Expansion::from_product(axis.x, xyz.x)
            .merge(&Expansion::from_product(axis.y, xyz.y))
            .merge(&Expansion::from_product(axis.z, xyz.z));
        dot.merge(&Expansion::from_product(axis_origin, h.w).negate())
    };
    let h0 = &control_points[spans[0].first];
    let num0 = num_of(h0);
    let w0 = h0.w;
    let w0_exp = Expansion::zero().grow(w0);
    for (idx, span) in spans.iter().enumerate() {
        for hi in control_points.iter().take(span.last + 1).skip(span.first) {
            let wi = hi.w;
            let numi = num_of(hi);
            // num_i * w0 - num0 * w_i  == 0  ?
            let lhs = numi.mul_expansion(&w0_exp);
            let rhs = num0.mul_expansion(&Expansion::zero().grow(wi));
            let diff = lhs.merge(&rhs.negate());
            if !diff.is_zero() {
                let iv = CertifiedInterval::from_expansion(&diff);
                let discrepancy = if iv.hi < 0.0 {
                    CertifiedSign::Negative
                } else if iv.lo > 0.0 {
                    CertifiedSign::Positive
                } else {
                    diff.sign()
                };
                return contradiction_constancy(
                    coord.name,
                    spans.len().min(idx + 1),
                    span,
                    discrepancy,
                );
            }
        }
    }
    // The reported constant value: L at the start point (a directed-rounding
    // evaluation for the report only; the certification used exact
    // cross-multiplication).
    let value = coord.at(start_point);
    SplineCarrierCertification::Certified(CarrierWitness::ConstantCoordinate {
        coordinate_name: coord.name,
        value,
        source_interval: trim,
        start_point,
        end_point,
        spans_examined: spans.len(),
        denominator_sign: Some(denom),
        basis: ProofBasis::BasisLinearIndependence,
    })
}

// ---------------------------------------------------------------------------
// Collinearity: non-rational
// ---------------------------------------------------------------------------

fn certify_straight_line_euclidean(
    bsp: &BSplineCurve<Point3>,
    spans: &[Span],
    trim: (f64, f64),
    start_point: Point3,
    end_point: Point3,
) -> SplineCarrierCertification {
    let cps = bsp.control_points();
    let p0 = cps[spans[0].first];
    // Pick the first nonzero displacement as the reference direction.
    let mut dir: Option<Vector3> = None;
    for span in spans {
        for p in cps.iter().take(span.last + 1).skip(span.first) {
            let d = *p - p0;
            if d.magnitude2() > 0.0 {
                dir = Some(d);
                break;
            }
        }
        if dir.is_some() {
            break;
        }
    }
    let Some(dir) = dir else {
        return SplineCarrierCertification::Unsupported(
            UnsupportedSplineReason::ProvedNotACarrier {
                detail: "degenerate_constant_image",
                spans_examined: spans.len(),
            },
        );
    };
    let dir_arr = [dir.x, dir.y, dir.z];
    for (idx, span) in spans.iter().enumerate() {
        for p in cps.iter().take(span.last + 1).skip(span.first) {
            let d = *p - p0;
            let cross = exact_cross3_f64([d.x, d.y, d.z], dir_arr);
            if !cross.is_zero() {
                return contradiction_collinear(spans.len().min(idx + 1), span, &cross);
            }
        }
    }
    SplineCarrierCertification::Certified(CarrierWitness::StraightLine {
        source_interval: trim,
        start_point,
        end_point,
        spans_examined: spans.len(),
        basis: ProofBasis::HomogeneousCollinearity,
    })
}

/// Exact 3D cross product of two f64 3-vectors, merged into a single
/// expansion whose exact sum is zero iff the cross is the zero vector.
fn exact_cross3_f64(a: [f64; 3], b: [f64; 3]) -> Expansion {
    let c0 =
        Expansion::from_product(a[1], b[2]).merge(&Expansion::from_product(a[2], b[1]).negate());
    let c1 =
        Expansion::from_product(a[2], b[0]).merge(&Expansion::from_product(a[0], b[2]).negate());
    let c2 =
        Expansion::from_product(a[0], b[1]).merge(&Expansion::from_product(a[1], b[0]).negate());
    c0.merge(&c1).merge(&c2)
}

fn contradiction_collinear(
    spans_examined: usize,
    span: &Span,
    cross: &Expansion,
) -> SplineCarrierCertification {
    let iv = CertifiedInterval::from_expansion(cross);
    let discrepancy = if iv.hi < 0.0 {
        CertifiedSign::Negative
    } else if iv.lo > 0.0 {
        CertifiedSign::Positive
    } else {
        cross.sign()
    };
    if span.fully_in_trim {
        SplineCarrierCertification::Inconsistent(InconsistencyWitness::NotCollinear {
            spans_examined,
            discrepancy,
        })
    } else {
        SplineCarrierCertification::Unresolved(
            UnresolvedSplineReason::TrimCutsNonConstantSpanInterior,
        )
    }
}

// ---------------------------------------------------------------------------
// Collinearity: rational (NURBS), exact in homogeneous coordinates
// ---------------------------------------------------------------------------

fn certify_straight_line_homogeneous(
    nurbs: &NurbsCurve<Vector4>,
    spans: &[Span],
    trim: (f64, f64),
    start_point: Point3,
    end_point: Point3,
    denom: CertifiedSign,
) -> SplineCarrierCertification {
    let cps = nurbs.control_points();
    // Projected P_i = H_i.xyz / w_i. The displacement P_i - P_0 is proportional
    // (denominator w_i w_0, nonzero by the sign-definite denominator check) to
    //   A_i = H_i.xyz * w_0 - H_0.xyz * w_i   (exact expansion per coord).
    // Collinearity <=> cross(A_i, A_ref) == 0 for all active points, exact.
    let h0 = cps[spans[0].first];
    let xyz0 = h0.truncate();
    let w0 = h0.w;
    let a_of = |h: &Vector4| -> [Expansion; 3] {
        let xyz = h.truncate();
        let w = h.w;
        [
            Expansion::from_product(xyz.x, w0).merge(&Expansion::from_product(xyz0.x, w).negate()),
            Expansion::from_product(xyz.y, w0).merge(&Expansion::from_product(xyz0.y, w).negate()),
            Expansion::from_product(xyz.z, w0).merge(&Expansion::from_product(xyz0.z, w).negate()),
        ]
    };
    // Pick a reference direction A_ref = first nonzero A_i.
    let mut a_ref: Option<[Expansion; 3]> = None;
    for span in spans {
        for p in cps.iter().take(span.last + 1).skip(span.first) {
            let a = a_of(p);
            let combined = a[0].clone().merge(&a[1].clone()).merge(&a[2].clone());
            if !combined.is_zero() {
                a_ref = Some(a);
                break;
            }
        }
        if a_ref.is_some() {
            break;
        }
    }
    let Some(a_ref) = a_ref else {
        return SplineCarrierCertification::Unsupported(
            UnsupportedSplineReason::ProvedNotACarrier {
                detail: "degenerate_constant_image",
                spans_examined: spans.len(),
            },
        );
    };
    for (idx, span) in spans.iter().enumerate() {
        for p in cps.iter().take(span.last + 1).skip(span.first) {
            let a = a_of(p);
            let cross = cross_expansion3(&a, &a_ref);
            if !cross.is_zero() {
                return contradiction_collinear(spans.len().min(idx + 1), span, &cross);
            }
        }
    }
    let _ = denom; // denominator sign is already certified; recorded for the
    // StraightLine witness is omitted (the line predicate does
    // not depend on it beyond non-vanishing).
    SplineCarrierCertification::Certified(CarrierWitness::StraightLine {
        source_interval: trim,
        start_point,
        end_point,
        spans_examined: spans.len(),
        basis: ProofBasis::HomogeneousCollinearity,
    })
}

/// Exact cross product of two per-component expansion 3-vectors, merged into a
/// single expansion whose exact sum is zero iff the cross is the zero vector.
fn cross_expansion3(a: &[Expansion; 3], b: &[Expansion; 3]) -> Expansion {
    let c0 = a[1]
        .mul_expansion(&b[2])
        .merge(&a[2].mul_expansion(&b[1]).negate());
    let c1 = a[2]
        .mul_expansion(&b[0])
        .merge(&a[0].mul_expansion(&b[2]).negate());
    let c2 = a[0]
        .mul_expansion(&b[1])
        .merge(&a[1].mul_expansion(&b[0]).negate());
    c0.merge(&c1).merge(&c2)
}

// ---------------------------------------------------------------------------
// Circle membership: NURBS, exact via the homogeneous quadratic form
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn certify_circular_arc_homogeneous(
    nurbs: &NurbsCurve<Vector4>,
    spans: &[Span],
    trim: (f64, f64),
    start_point: Point3,
    end_point: Point3,
    circle: CirclePlacement,
    denom: CertifiedSign,
) -> SplineCarrierCertification {
    if !(circle.radius > 0.0 && circle.radius.is_finite()) {
        return SplineCarrierCertification::Unresolved(
            UnresolvedSplineReason::DegenerateCandidateCircle,
        );
    }
    if !vector_unit_finite(&circle.normal) {
        return SplineCarrierCertification::Unresolved(
            UnresolvedSplineReason::DegenerateCandidateCircle,
        );
    }
    // Endpoint confirmation (structural, not interior sampling): the trim
    // endpoints must lie on the circle, else the query contradicts itself.
    let tol = SPLINE_CERTIFIED_EQUAL_ULPS * f64::EPSILON * circle.radius.max(1.0);
    for (p, tag) in [(start_point, "start"), (end_point, "end")] {
        let r = p - circle.center;
        let off_plane = r.dot(circle.normal);
        let radial = r - off_plane * circle.normal;
        let gap = ((radial.magnitude() - circle.radius).abs()).max(off_plane.abs());
        if gap > tol {
            return SplineCarrierCertification::Inconsistent(
                InconsistencyWitness::EndpointInconsistent { detail: tag },
            );
        }
    }

    let degree = nurbs.degree();
    if degree > MAX_CIRCLE_MEMBERSHIP_DEGREE {
        return SplineCarrierCertification::OperationalFailure(ResourceReport {
            detail: "degree_exceeds_circle_membership_budget",
            degree: Some(degree),
            control_points: Some(nurbs.control_points().len()),
        });
    }

    // Exact circle membership requires each examined span to be a single
    // Bézier segment: every distinct knot at multiplicity >= degree. Then the
    // Bernstein control points are the curve's own, read verbatim.
    if !is_piecewise_bezier(nurbs.knot_vec(), degree) {
        return SplineCarrierCertification::Unresolved(
            UnresolvedSplineReason::CircleMembershipNeedsBezierForm,
        );
    }

    let cps = nurbs.control_points();
    let center = circle.center;
    let normal = circle.normal;
    let r2 = Expansion::from_product(circle.radius, circle.radius);

    // Natural scale of the quadratic form's terms: |S_i·S_j| and r²|w_i w_j|
    // are each ~ (radius · max|w|)² for points on the circle. The Bernstein
    // coefficient c_k is a sum of (2p+1) such terms with binomial weights, so
    // the rounding-level residual is bounded by ~ scale · ULPS · EPSILON.
    let max_w = nurbs
        .control_points()
        .iter()
        .map(|h| h.w.abs())
        .fold(0.0_f64, f64::max)
        .max(1.0);
    let term_scale = circle.radius * circle.radius * max_w * max_w * (2 * degree + 1) as f64;
    let rounding_threshold = SPLINE_CERTIFIED_EQUAL_ULPS * f64::EPSILON * term_scale;
    let mut rounding_residual_seen = false;

    for (span_idx, span) in spans.iter().enumerate() {
        let seg_first = span.first;
        let seg_last = span.last;
        debug_assert_eq!(seg_last - seg_first, degree);
        // S_i = H_i.xyz - center * w_i  (exact expansion per coord), and w_i.
        let np1 = degree + 1;
        let mut s_xyz: Vec<[Expansion; 3]> = Vec::with_capacity(np1);
        let mut w_vals: Vec<f64> = Vec::with_capacity(np1);
        for h in cps.iter().take(seg_last + 1).skip(seg_first) {
            let xyz = h.truncate();
            let w = h.w;
            w_vals.push(w);
            s_xyz.push([
                Expansion::from_sum(xyz.x, -center.x * w),
                Expansion::from_sum(xyz.y, -center.y * w),
                Expansion::from_sum(xyz.z, -center.z * w),
            ]);
        }
        // (a) coplanarity: S_i · normal == 0 for every control point.
        for s in &s_xyz {
            let dot = dot_exp_f64(s, &[normal.x, normal.y, normal.z]);
            match decide_zero(&dot, rounding_threshold) {
                ZeroKind::Exact => {}
                ZeroKind::Rounding => rounding_residual_seen = true,
                ZeroKind::Beyond(sign) => {
                    return SplineCarrierCertification::Inconsistent(
                        InconsistencyWitness::NotOnCircle {
                            segment: span_idx,
                            coefficient: 0,
                            discrepancy: sign,
                        },
                    );
                }
            }
        }
        // (b) radius: the degree-2p Bernstein coefficients of Q = |S|^2 - r^2 w^2.
        // c_k ∝ Σ_{i+j=k} C(p,i) C(p,j) (S_i·S_j - r^2 w_i w_j), k = 0..=2p.
        let p = degree;
        let mut s_dot = vec![vec![Expansion::zero(); np1]; np1];
        let mut w_prod = vec![vec![Expansion::zero(); np1]; np1];
        for i in 0..np1 {
            for j in i..np1 {
                let dot = dot_exp_exp(&s_xyz[i], &s_xyz[j]);
                let wp = Expansion::from_product(w_vals[i], w_vals[j]);
                s_dot[i][j] = dot.clone();
                s_dot[j][i] = dot;
                w_prod[i][j] = wp.clone();
                w_prod[j][i] = wp;
            }
        }
        for k in 0..=2 * p {
            let mut ck = Expansion::zero();
            let i_min = k.saturating_sub(p);
            let i_max = k.min(p);
            for i in i_min..=i_max {
                let j = k - i;
                let binom_ij = binom(p, i) * binom(p, j);
                let inner = s_dot[i][j].merge(&r2.mul_expansion(&w_prod[i][j]).negate());
                ck = ck.merge(&scale_expansion_by_int(&inner, binom_ij));
            }
            match decide_zero(&ck, rounding_threshold) {
                ZeroKind::Exact => {}
                ZeroKind::Rounding => rounding_residual_seen = true,
                ZeroKind::Beyond(sign) => {
                    return SplineCarrierCertification::Inconsistent(
                        InconsistencyWitness::NotOnCircle {
                            segment: span_idx,
                            coefficient: k,
                            discrepancy: sign,
                        },
                    );
                }
            }
        }
    }
    if rounding_residual_seen {
        return SplineCarrierCertification::Unresolved(
            UnresolvedSplineReason::CircleMembershipWithinRounding,
        );
    }
    SplineCarrierCertification::Certified(CarrierWitness::CircularArc {
        placement: circle,
        source_interval: trim,
        start_point,
        end_point,
        spans_examined: spans.len(),
        denominator_sign: denom,
        basis: ProofBasis::HomogeneousQuadraticForm,
    })
}

/// Classify an exact expansion against a rounding threshold: exactly zero,
/// within the f64-rounding band of zero (cannot decide), or bounded away from
/// zero (a proved nonzero sign).
///
/// `threshold` is a *rounding bound* derived from the input precision and the
/// term scale — never a tolerance that manufactures a `Certified` result. It
/// only separates a proved contradiction (`Beyond`) from the f64-rounding
/// residual (`Rounding`) that a genuine curve with rounded irrational data
/// (e.g. the standard `1/√2`-weight rational circle) leaves in the exact
/// arithmetic.
enum ZeroKind {
    /// The expansion is exactly zero: `is_zero()` holds.
    Exact,
    /// The expansion is nonzero but within `±threshold` of zero: consistent
    /// with f64-rounded exact data, but exact membership cannot be proved.
    Rounding,
    /// The expansion is bounded away from zero beyond `±threshold`: a proved
    /// nonzero sign, carrying that sign.
    Beyond(CertifiedSign),
}

fn decide_zero(e: &Expansion, threshold: f64) -> ZeroKind {
    if e.is_zero() {
        return ZeroKind::Exact;
    }
    let iv = CertifiedInterval::from_expansion(e);
    if !iv.is_finite() {
        // A non-finite enclosure is an operational precision failure, not a
        // proof of either kind; treat conservatively as rounding (so the
        // caller returns Unresolved rather than a false Inconsistent).
        return ZeroKind::Rounding;
    }
    if iv.lo > threshold {
        ZeroKind::Beyond(CertifiedSign::Positive)
    } else if iv.hi < -threshold {
        ZeroKind::Beyond(CertifiedSign::Negative)
    } else {
        ZeroKind::Rounding
    }
}

/// Exact dot product of an expansion-per-component 3-vector with a plain
/// f64 3-vector.
fn dot_exp_f64(a: &[Expansion; 3], b: &[f64; 3]) -> Expansion {
    let mut acc = Expansion::zero();
    for i in 0..3 {
        acc = acc.merge(&a[i].mul_expansion(&Expansion::zero().grow(b[i])));
    }
    acc
}

/// Exact dot product of two expansion-per-component 3-vectors.
fn dot_exp_exp(a: &[Expansion; 3], b: &[Expansion; 3]) -> Expansion {
    let mut acc = Expansion::zero();
    for i in 0..3 {
        acc = acc.merge(&a[i].mul_expansion(&b[i].clone()));
    }
    acc
}

/// Exact integer binomial coefficient `C(n, k)` as an `f64`. Exact for all
/// results up to `2^53` (well beyond any real spline degree).
fn binom(n: usize, k: usize) -> f64 {
    if k > n {
        return 0.0;
    }
    let k = k.min(n - k);
    let mut r: f64 = 1.0;
    for i in 0..k {
        // r = r * (n - i) / (i + 1), kept integral at every step.
        r = (r * (n - i) as f64) / (i + 1) as f64;
    }
    r
}

/// Scale an expansion by a small integer exactly. The integer is exactly
/// representable in f64, but multiplication by an exactly-representable
/// integer is not generally error-free, so route through the expansion product
/// to preserve exactness.
fn scale_expansion_by_int(e: &Expansion, n: f64) -> Expansion {
    if n == 0.0 {
        return Expansion::zero();
    }
    e.mul_expansion(&Expansion::zero().grow(n))
}

/// Whether a knot vector is in piecewise-Bézier form for a spline of the given
/// degree: every distinct knot has multiplicity `>= degree` (endpoints of a
/// clamped curve have multiplicity `degree + 1`, which satisfies this).
fn is_piecewise_bezier(knots: &KnotVec, degree: usize) -> bool {
    let (_distinct, mults) = knots.to_single_multi();
    for &m in &mults {
        if m < degree {
            return false;
        }
    }
    true
}

/// Whether a vector is finite and (approximately) unit length. Used only as a
/// sanity gate on the caller-supplied circle normal; the exact predicates do
/// not depend on it being bit-exactly unit.
fn vector_unit_finite(v: &Vector3) -> bool {
    let mag2 = v.dot(*v);
    mag2.is_finite() && (mag2 - 1.0).abs() <= SPLINE_CERTIFIED_EQUAL_ULPS * f64::EPSILON
}

// ---------------------------------------------------------------------------
// Denominator sign (proof obligation 1)
// ---------------------------------------------------------------------------

enum DenominatorVerdict {
    Definite(CertifiedSign),
    Indefinite,
    Indeterminate,
}

fn denominator_sign(weights: &[Vector4]) -> DenominatorVerdict {
    let mut pos = false;
    let mut neg = false;
    let mut zero = false;
    for h in weights {
        let w = h.w;
        if !w.is_finite() {
            return DenominatorVerdict::Indeterminate;
        }
        if w > 0.0 {
            pos = true;
        } else if w < 0.0 {
            neg = true;
        } else {
            zero = true;
        }
    }
    if zero {
        // A zero weight makes one control point a point at infinity; the
        // denominator can still be sign-definite away from that knot, but the
        // curve is not a regular carrier parallel through it. Treat as
        // indeterminate rather than certifying through a pole.
        return DenominatorVerdict::Indeterminate;
    }
    if pos && neg {
        DenominatorVerdict::Indefinite
    } else if pos {
        DenominatorVerdict::Definite(CertifiedSign::Positive)
    } else if neg {
        DenominatorVerdict::Definite(CertifiedSign::Negative)
    } else {
        DenominatorVerdict::Indeterminate
    }
}

// ---------------------------------------------------------------------------
// Input validation and endpoint images
// ---------------------------------------------------------------------------

fn resource_guard(degree: usize, n_ctrl: usize) -> Option<ResourceReport> {
    if n_ctrl == 0 {
        return Some(ResourceReport {
            detail: "empty_control_points",
            degree: Some(degree),
            control_points: Some(0),
        });
    }
    if n_ctrl > MAX_CONTROL_POINTS {
        return Some(ResourceReport {
            detail: "control_point_count_exceeds_budget",
            degree: Some(degree),
            control_points: Some(n_ctrl),
        });
    }
    None
}

fn check_finite_control_points_euclidean(cps: &[Point3]) -> Option<SplineCarrierCertification> {
    for p in cps {
        if !p.x.is_finite() || !p.y.is_finite() || !p.z.is_finite() {
            return Some(SplineCarrierCertification::OperationalFailure(
                ResourceReport {
                    detail: "non_finite_control_point",
                    degree: None,
                    control_points: Some(cps.len()),
                },
            ));
        }
    }
    None
}

fn check_finite_control_points_homogeneous(cps: &[Vector4]) -> Option<SplineCarrierCertification> {
    for h in cps {
        if !h.x.is_finite() || !h.y.is_finite() || !h.z.is_finite() || !h.w.is_finite() {
            return Some(SplineCarrierCertification::OperationalFailure(
                ResourceReport {
                    detail: "non_finite_homogeneous_control_point",
                    degree: None,
                    control_points: Some(cps.len()),
                },
            ));
        }
    }
    None
}

#[allow(clippy::result_large_err)]
fn endpoint_images_euclidean(
    bsp: &BSplineCurve<Point3>,
    trim: (f64, f64),
) -> Result<(Point3, Point3), SplineCarrierCertification> {
    let (t0, t1) = trim;
    let start = bsp.subs(t0);
    let end = bsp.subs(t1);
    if !is_finite_point(start) || !is_finite_point(end) {
        return Err(SplineCarrierCertification::OperationalFailure(
            ResourceReport {
                detail: "non_finite_endpoint_image",
                degree: Some(bsp.degree()),
                control_points: Some(bsp.control_points().len()),
            },
        ));
    }
    Ok((start, end))
}

#[allow(clippy::result_large_err)]
fn endpoint_images_homogeneous(
    nurbs: &NurbsCurve<Vector4>,
    trim: (f64, f64),
) -> Result<(Point3, Point3), SplineCarrierCertification> {
    let (t0, t1) = trim;
    let start = nurbs.subs(t0);
    let end = nurbs.subs(t1);
    if !is_finite_point(start) || !is_finite_point(end) {
        return Err(SplineCarrierCertification::OperationalFailure(
            ResourceReport {
                detail: "non_finite_endpoint_image",
                degree: Some(nurbs.degree()),
                control_points: Some(nurbs.control_points().len()),
            },
        ));
    }
    Ok((start, end))
}

fn is_finite_point(p: Point3) -> bool {
    p.x.is_finite() && p.y.is_finite() && p.z.is_finite()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use truck_meshalgo::prelude::Matrix4;
    use truck_stepio::r#in::step_geometry::{
        Conic3D, Line as StepLine, Processor, Transformed, TrimmedCurve, UnitCircle,
    };

    // -- fixtures -----------------------------------------------------------

    /// A non-rational B-spline straight line along +x from (0,0,0) to (1,0,0).
    fn line_bspline() -> BSplineCurve<Point3> {
        let knots = KnotVec::bezier_knot(1);
        let cps = vec![Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 0.0, 0.0)];
        BSplineCurve::new(knots, cps)
    }

    /// A non-rational B-spline that is NOT a line: a parabola.
    fn parabola_bspline() -> BSplineCurve<Point3> {
        let knots = KnotVec::bezier_knot(2);
        let cps = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(0.5, 0.0, 0.0),
            Point3::new(1.0, 1.0, 0.0),
        ];
        BSplineCurve::new(knots, cps)
    }

    /// A rational quadratic NURBS exact quarter circle of radius 1 about the
    /// origin in the xy-plane, from (1,0) to (0,1): control points (1,0),
    /// (1,1), (0,1) with weights (1, 1/sqrt2, 1), degree 2, single Bézier span.
    /// A single rational quadratic Bézier represents an arc of angle < pi; the
    /// weight 1/sqrt2 = cos(pi/4) is the quarter-circle weight.
    fn quarter_circle_nurbs() -> NurbsCurve<Vector4> {
        let knots = KnotVec::bezier_knot(2);
        let s2 = std::f64::consts::FRAC_1_SQRT_2;
        let cps = vec![
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(1.0, 1.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        ];
        NurbsCurve::try_from_bspline_and_weights(BSplineCurve::new(knots, cps), vec![1.0, s2, 1.0])
            .expect("weights match control points")
    }

    /// A two-segment piecewise-Bézier NURBS: a half circle from (1,0) to
    /// (-1,0), as two quadratic Bézier quarter-circle spans joined at (0,1).
    /// Demonstrates multiple knot spans with an interior knot at full
    /// multiplicity.
    fn two_span_circle_nurbs() -> NurbsCurve<Vector4> {
        let s2 = std::f64::consts::FRAC_1_SQRT_2;
        // Interior knot at 1 has multiplicity 2 = degree -> piecewise Bézier.
        let knots = KnotVec::from(vec![0.0, 0.0, 0.0, 1.0, 1.0, 2.0, 2.0, 2.0]);
        let cps = vec![
            Point3::new(1.0, 0.0, 0.0),  // seg0 start
            Point3::new(1.0, 1.0, 0.0),  // seg0 mid (weight s2)
            Point3::new(0.0, 1.0, 0.0),  // seg0 end = seg1 start (shared)
            Point3::new(-1.0, 1.0, 0.0), // seg1 mid (weight s2)
            Point3::new(-1.0, 0.0, 0.0), // seg1 end
        ];
        NurbsCurve::try_from_bspline_and_weights(
            BSplineCurve::new(knots, cps),
            vec![1.0, s2, 1.0, s2, 1.0],
        )
        .expect("weights match")
    }

    /// A rational NURBS that is NOT on the unit circle: the middle control
    /// point perturbed off the tangent line.
    fn perturbed_circle_nurbs() -> NurbsCurve<Vector4> {
        let knots = KnotVec::bezier_knot(2);
        let s2 = std::f64::consts::FRAC_1_SQRT_2;
        let cps = vec![
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(1.0, 1.2, 0.0), // perturbed: was (1,1)
            Point3::new(0.0, 1.0, 0.0),
        ];
        NurbsCurve::try_from_bspline_and_weights(BSplineCurve::new(knots, cps), vec![1.0, s2, 1.0])
            .expect("weights match")
    }

    /// A NURBS with mixed-sign weights: denominator sign-indefinite.
    fn mixed_sign_weights_nurbs() -> NurbsCurve<Vector4> {
        let knots = KnotVec::bezier_knot(2);
        let cps = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 1.0, 0.0),
            Point3::new(2.0, 0.0, 0.0),
        ];
        NurbsCurve::try_from_bspline_and_weights(
            BSplineCurve::new(knots, cps),
            vec![1.0, -1.0, 1.0],
        )
        .expect("weights match")
    }

    fn unit_circle_placement() -> CirclePlacement {
        CirclePlacement {
            center: Point3::new(0.0, 0.0, 0.0),
            radius: 1.0,
            normal: Vector3::new(0.0, 0.0, 1.0),
        }
    }

    fn axial_coord() -> LinearCarrierCoordinate {
        LinearCarrierCoordinate {
            axis: Vector3::new(0.0, 0.0, 1.0),
            origin: Point3::new(0.0, 0.0, 0.0),
            name: "axial",
        }
    }

    // -- non-rational: straight line ---------------------------------------

    #[test]
    fn non_rational_line_is_certified_straight_line() {
        let curve = Curve3D::BSplineCurve(line_bspline());
        let r = certify_spline_carrier(&curve, CarrierQuery::StraightLine, (0.0, 1.0));
        assert!(matches!(
            r,
            SplineCarrierCertification::Certified(CarrierWitness::StraightLine { .. })
        ));
    }

    #[test]
    fn non_rational_parabola_is_refused_as_not_collinear() {
        let curve = Curve3D::BSplineCurve(parabola_bspline());
        let r = certify_spline_carrier(&curve, CarrierQuery::StraightLine, (0.0, 1.0));
        match r {
            SplineCarrierCertification::Inconsistent(InconsistencyWitness::NotCollinear {
                ..
            }) => {}
            other => panic!("expected Inconsistent NotCollinear, got {other:?}"),
        }
    }

    #[test]
    fn non_rational_cannot_be_circular_parallel() {
        let curve = Curve3D::BSplineCurve(line_bspline());
        let r = certify_spline_carrier(
            &curve,
            CarrierQuery::CircularArcOn(unit_circle_placement()),
            (0.0, 1.0),
        );
        assert_eq!(
            r,
            SplineCarrierCertification::Unsupported(
                UnsupportedSplineReason::NonRationalSplineCannotBeCircularParallel
            )
        );
    }

    #[test]
    fn non_rational_line_has_constant_axial_coordinate() {
        let knots = KnotVec::bezier_knot(1);
        let cps = vec![Point3::new(0.0, 0.0, 2.0), Point3::new(1.0, 0.0, 2.0)];
        let curve = Curve3D::BSplineCurve(BSplineCurve::new(knots, cps));
        let r = certify_spline_carrier(
            &curve,
            CarrierQuery::ConstantLinearCoordinate(axial_coord()),
            (0.0, 1.0),
        );
        match r {
            SplineCarrierCertification::Certified(CarrierWitness::ConstantCoordinate {
                value,
                ..
            }) => assert!((value - 2.0).abs() < 1e-12),
            other => panic!("expected Certified ConstantCoordinate, got {other:?}"),
        }
    }

    #[test]
    fn non_rational_varying_z_is_not_constant_axial() {
        let knots = KnotVec::bezier_knot(1);
        let cps = vec![Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 0.0, 1.0)];
        let curve = Curve3D::BSplineCurve(BSplineCurve::new(knots, cps));
        let r = certify_spline_carrier(
            &curve,
            CarrierQuery::ConstantLinearCoordinate(axial_coord()),
            (0.0, 1.0),
        );
        match r {
            SplineCarrierCertification::Inconsistent(
                InconsistencyWitness::CoordinateNotConstant {
                    coordinate_name: "axial",
                    ..
                },
            ) => {}
            other => panic!("expected Inconsistent CoordinateNotConstant, got {other:?}"),
        }
    }

    // -- rational: circle membership ---------------------------------------

    #[test]
    fn rational_quarter_circle_is_unresolved_within_rounding() {
        // The standard rational-quadratic quarter circle uses weight 1/√2,
        // which is irrational and rounds in f64. The exact homogeneous
        // quadratic form therefore has a residual at the f64-rounding level
        // (~1e-17), not exactly zero: f64 cannot prove exact circle
        // membership for a curve whose weights are rounded. Honest outcome:
        // Unresolved, never a false Certified and never a false Inconsistent.
        let curve = Curve3D::NurbsCurve(quarter_circle_nurbs());
        let r = certify_spline_carrier(
            &curve,
            CarrierQuery::CircularArcOn(unit_circle_placement()),
            (0.0, 1.0),
        );
        match r {
            SplineCarrierCertification::Unresolved(
                UnresolvedSplineReason::CircleMembershipWithinRounding,
            ) => {}
            other => panic!("expected Unresolved CircleMembershipWithinRounding, got {other:?}"),
        }
    }

    #[test]
    fn two_span_circle_is_unresolved_within_rounding() {
        // Two quarter-circle spans, each with the irrational 1/√2 weight.
        let curve = Curve3D::NurbsCurve(two_span_circle_nurbs());
        let r = certify_spline_carrier(
            &curve,
            CarrierQuery::CircularArcOn(unit_circle_placement()),
            (0.0, 2.0),
        );
        match r {
            SplineCarrierCertification::Unresolved(
                UnresolvedSplineReason::CircleMembershipWithinRounding,
            ) => {}
            other => panic!("expected Unresolved CircleMembershipWithinRounding, got {other:?}"),
        }
    }

    /// A degenerate NURBS whose image is a single point on the circle: every
    /// control point coincides and every weight is exactly representable, so
    /// the homogeneous quadratic form is *exactly* zero. This is the only
    /// non-trivial-to-construct case where `Certified(CircularArc)` is
    /// reachable in f64 — a real circle with irrational weights lands in
    /// `Unresolved(CircleMembershipWithinRounding)` instead (see above).
    #[test]
    fn degenerate_point_on_circle_is_certified_circular_arc() {
        let knots = KnotVec::bezier_knot(2);
        let cps = vec![
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
        ];
        let nurbs = NurbsCurve::try_from_bspline_and_weights(
            BSplineCurve::new(knots, cps),
            vec![1.0, 1.0, 1.0],
        )
        .expect("weights match");
        let curve = Curve3D::NurbsCurve(nurbs);
        let r = certify_spline_carrier(
            &curve,
            CarrierQuery::CircularArcOn(unit_circle_placement()),
            (0.0, 1.0),
        );
        assert!(matches!(
            r,
            SplineCarrierCertification::Certified(CarrierWitness::CircularArc { .. })
        ));
    }

    #[test]
    fn perturbed_circle_is_inconsistent_not_on_circle() {
        let curve = Curve3D::NurbsCurve(perturbed_circle_nurbs());
        let r = certify_spline_carrier(
            &curve,
            CarrierQuery::CircularArcOn(unit_circle_placement()),
            (0.0, 1.0),
        );
        match r {
            SplineCarrierCertification::Inconsistent(InconsistencyWitness::NotOnCircle {
                ..
            }) => {}
            other => panic!("expected Inconsistent NotOnCircle, got {other:?}"),
        }
    }

    #[test]
    fn rational_circle_endpoint_off_circle_is_inconsistent() {
        let curve = Curve3D::NurbsCurve(quarter_circle_nurbs());
        let bad = CirclePlacement {
            center: Point3::new(0.0, 0.0, 0.0),
            radius: 2.0,
            normal: Vector3::new(0.0, 0.0, 1.0),
        };
        let r = certify_spline_carrier(&curve, CarrierQuery::CircularArcOn(bad), (0.0, 1.0));
        match r {
            SplineCarrierCertification::Inconsistent(
                InconsistencyWitness::EndpointInconsistent { .. },
            ) => {}
            other => panic!("expected EndpointInconsistent, got {other:?}"),
        }
    }

    // -- rational: denominator validity ------------------------------------

    #[test]
    fn mixed_sign_weights_are_inconsistent_denominator() {
        let curve = Curve3D::NurbsCurve(mixed_sign_weights_nurbs());
        let r = certify_spline_carrier(
            &curve,
            CarrierQuery::CircularArcOn(unit_circle_placement()),
            (0.0, 1.0),
        );
        assert_eq!(
            r,
            SplineCarrierCertification::Inconsistent(
                InconsistencyWitness::DenominatorSignIndefinite
            )
        );
    }

    // -- rational: constancy and collinearity ------------------------------

    #[test]
    fn rational_circle_has_constant_axial_coordinate() {
        // The unit semicircle lies in z=0, so its axial (z) coordinate is 0.
        let curve = Curve3D::NurbsCurve(quarter_circle_nurbs());
        let r = certify_spline_carrier(
            &curve,
            CarrierQuery::ConstantLinearCoordinate(axial_coord()),
            (0.0, 1.0),
        );
        match r {
            SplineCarrierCertification::Certified(CarrierWitness::ConstantCoordinate {
                value,
                denominator_sign: Some(CertifiedSign::Positive),
                ..
            }) => assert!(value.abs() < 1e-9),
            other => panic!("expected Certified ConstantCoordinate, got {other:?}"),
        }
    }

    #[test]
    fn rational_varying_z_is_not_constant_axial() {
        let knots = KnotVec::bezier_knot(1);
        let cps = vec![Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 0.0, 1.0)];
        let nurbs =
            NurbsCurve::try_from_bspline_and_weights(BSplineCurve::new(knots, cps), vec![1.0, 1.0])
                .expect("weights match");
        let curve = Curve3D::NurbsCurve(nurbs);
        let r = certify_spline_carrier(
            &curve,
            CarrierQuery::ConstantLinearCoordinate(axial_coord()),
            (0.0, 1.0),
        );
        match r {
            SplineCarrierCertification::Inconsistent(
                InconsistencyWitness::CoordinateNotConstant { .. },
            ) => {}
            other => panic!("expected Inconsistent CoordinateNotConstant, got {other:?}"),
        }
    }

    #[test]
    fn rational_line_is_certified_straight_line() {
        let knots = KnotVec::bezier_knot(1);
        let cps = vec![Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 0.0, 0.0)];
        let nurbs =
            NurbsCurve::try_from_bspline_and_weights(BSplineCurve::new(knots, cps), vec![1.0, 2.0])
                .expect("weights match");
        let curve = Curve3D::NurbsCurve(nurbs);
        let r = certify_spline_carrier(&curve, CarrierQuery::StraightLine, (0.0, 1.0));
        assert!(matches!(
            r,
            SplineCarrierCertification::Certified(CarrierWitness::StraightLine { .. })
        ));
    }

    #[test]
    fn rational_non_collinear_is_inconsistent() {
        let knots = KnotVec::bezier_knot(2);
        let cps = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 1.0, 0.0),
            Point3::new(2.0, 0.0, 0.0),
        ];
        let nurbs = NurbsCurve::try_from_bspline_and_weights(
            BSplineCurve::new(knots, cps),
            vec![1.0, 1.0, 1.0],
        )
        .expect("weights match");
        let curve = Curve3D::NurbsCurve(nurbs);
        let r = certify_spline_carrier(&curve, CarrierQuery::StraightLine, (0.0, 1.0));
        match r {
            SplineCarrierCertification::Inconsistent(InconsistencyWitness::NotCollinear {
                ..
            }) => {}
            other => panic!("expected Inconsistent NotCollinear, got {other:?}"),
        }
    }

    // -- trim / orientation / nonuniform -----------------------------------

    #[test]
    fn inverted_trim_is_refused() {
        let knots = KnotVec::bezier_knot(1);
        let cps = vec![Point3::new(0.0, 0.0, 3.0), Point3::new(1.0, 0.0, 3.0)];
        let curve = Curve3D::BSplineCurve(BSplineCurve::new(knots, cps));
        let r = certify_spline_carrier(
            &curve,
            CarrierQuery::ConstantLinearCoordinate(axial_coord()),
            (1.0, 0.0),
        );
        assert!(matches!(
            r,
            SplineCarrierCertification::OperationalFailure(_)
        ));
    }

    #[test]
    fn nonuniform_knots_constancy_holds() {
        // Clamped B-spline with a nonuniform interior knot, constant z = 5.
        let knots = KnotVec::from(vec![0.0, 0.0, 0.0, 0.3, 1.0, 1.0, 1.0]);
        let cps = vec![
            Point3::new(0.0, 0.0, 5.0),
            Point3::new(0.5, 0.0, 5.0),
            Point3::new(0.7, 0.0, 5.0),
            Point3::new(1.0, 0.0, 5.0),
        ];
        let curve = Curve3D::BSplineCurve(BSplineCurve::new(knots, cps));
        let r = certify_spline_carrier(
            &curve,
            CarrierQuery::ConstantLinearCoordinate(axial_coord()),
            (0.0, 1.0),
        );
        match r {
            SplineCarrierCertification::Certified(CarrierWitness::ConstantCoordinate {
                value,
                spans_examined,
                ..
            }) => {
                assert!((value - 5.0).abs() < 1e-12);
                assert!(
                    spans_examined >= 2,
                    "nonuniform interior knot -> >= 2 spans"
                );
            }
            other => panic!("expected Certified, got {other:?}"),
        }
    }

    #[test]
    fn trim_subinterval_aligning_with_knot_is_certified() {
        let knots = KnotVec::from(vec![0.0, 0.0, 0.0, 0.3, 1.0, 1.0, 1.0]);
        let cps = vec![
            Point3::new(0.0, 0.0, 5.0),
            Point3::new(0.5, 0.0, 5.0),
            Point3::new(0.7, 0.0, 5.0),
            Point3::new(1.0, 0.0, 5.0),
        ];
        let curve = Curve3D::BSplineCurve(BSplineCurve::new(knots, cps));
        let r = certify_spline_carrier(
            &curve,
            CarrierQuery::ConstantLinearCoordinate(axial_coord()),
            (0.0, 0.3),
        );
        assert!(matches!(r, SplineCarrierCertification::Certified(_)));
    }

    #[test]
    fn trim_cutting_non_constant_span_is_unresolved() {
        // A curve that is NOT constant in z on its only span, trimmed to a
        // sub-interval interior to that span.
        let knots = KnotVec::bezier_knot(2); // single span [0,1], degree 2
        let cps = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(0.5, 0.0, 1.0),
            Point3::new(1.0, 0.0, 0.0),
        ];
        let curve = Curve3D::BSplineCurve(BSplineCurve::new(knots, cps));
        let r = certify_spline_carrier(
            &curve,
            CarrierQuery::ConstantLinearCoordinate(axial_coord()),
            (0.2, 0.8),
        );
        match r {
            SplineCarrierCertification::Unresolved(
                UnresolvedSplineReason::TrimCutsNonConstantSpanInterior,
            ) => {}
            other => panic!("expected Unresolved TrimCutsNonConstantSpanInterior, got {other:?}"),
        }
    }

    // -- transform handling -------------------------------------------------

    #[test]
    fn reflected_nurbs_circle_is_unresolved_within_rounding() {
        // Reflect the quarter circle across the x-axis. The image is still a
        // unit circle (with rounded 1/√2 weights), so the exact predicate
        // lands in Unresolved(CircleMembershipWithinRounding) exactly as the
        // unreflected one does — the transform is baked into the control
        // points and the predicate reads them directly.
        let mut nurbs = quarter_circle_nurbs();
        nurbs.transform_by(Matrix4::from_nonuniform_scale(1.0, -1.0, 1.0));
        let curve = Curve3D::NurbsCurve(nurbs);
        let r = certify_spline_carrier(
            &curve,
            CarrierQuery::CircularArcOn(unit_circle_placement()),
            (0.0, 1.0),
        );
        match r {
            SplineCarrierCertification::Unresolved(
                UnresolvedSplineReason::CircleMembershipWithinRounding,
            ) => {}
            other => panic!("expected Unresolved CircleMembershipWithinRounding, got {other:?}"),
        }
    }

    #[test]
    fn translated_nurbs_circle_is_unresolved_within_rounding() {
        let mut nurbs = quarter_circle_nurbs();
        let translation = Vector3::new(5.0, -2.0, 3.0);
        nurbs.transform_by(Matrix4::from_translation(translation));
        let curve = Curve3D::NurbsCurve(nurbs);
        let placement = CirclePlacement {
            center: Point3::new(0.0, 0.0, 0.0) + translation,
            radius: 1.0,
            normal: Vector3::new(0.0, 0.0, 1.0),
        };
        let r = certify_spline_carrier(&curve, CarrierQuery::CircularArcOn(placement), (0.0, 1.0));
        match r {
            SplineCarrierCertification::Unresolved(
                UnresolvedSplineReason::CircleMembershipWithinRounding,
            ) => {}
            other => panic!("expected Unresolved CircleMembershipWithinRounding, got {other:?}"),
        }
    }

    // -- resource / operational --------------------------------------------

    #[test]
    fn non_finite_trim_is_operational_failure() {
        let curve = Curve3D::BSplineCurve(line_bspline());
        let r = certify_spline_carrier(&curve, CarrierQuery::StraightLine, (f64::NAN, 1.0));
        assert!(matches!(
            r,
            SplineCarrierCertification::OperationalFailure(_)
        ));
    }

    #[test]
    fn empty_trim_is_operational_failure() {
        let curve = Curve3D::BSplineCurve(line_bspline());
        let r = certify_spline_carrier(&curve, CarrierQuery::StraightLine, (0.5, 0.5));
        assert!(matches!(
            r,
            SplineCarrierCertification::OperationalFailure(_)
        ));
    }

    #[test]
    fn trim_outside_domain_is_operational_failure() {
        let curve = Curve3D::BSplineCurve(line_bspline());
        let r = certify_spline_carrier(&curve, CarrierQuery::StraightLine, (-1.0, 2.0));
        assert!(matches!(
            r,
            SplineCarrierCertification::OperationalFailure(_)
        ));
    }

    #[test]
    fn non_spline_representation_is_unsupported() {
        let circle = Processor::new(TrimmedCurve::new(UnitCircle::new(), (0.0, 1.0)));
        let curve = Curve3D::Conic(Conic3D::Ellipse(circle));
        let r = certify_spline_carrier(&curve, CarrierQuery::StraightLine, (0.0, 1.0));
        assert_eq!(
            r,
            SplineCarrierCertification::Unsupported(
                UnsupportedSplineReason::NotASplineRepresentation
            )
        );
    }

    // -- non-bezier-form circle membership is unresolved -------------------

    #[test]
    fn non_bezier_form_nurbs_circle_is_unresolved() {
        // Interior knot at multiplicity 1 < degree 2 -> not piecewise Bézier.
        let knots = KnotVec::from(vec![0.0, 0.0, 0.0, 0.5, 1.0, 1.0, 1.0]);
        let cps = vec![
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(1.0, 1.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
            Point3::new(-1.0, 0.0, 0.0),
        ];
        let s2 = std::f64::consts::FRAC_1_SQRT_2;
        let nurbs = NurbsCurve::try_from_bspline_and_weights(
            BSplineCurve::new(knots, cps),
            vec![1.0, s2, s2, 1.0],
        )
        .expect("weights match");
        let curve = Curve3D::NurbsCurve(nurbs);
        let r = certify_spline_carrier(
            &curve,
            CarrierQuery::CircularArcOn(unit_circle_placement()),
            (0.0, 1.0),
        );
        match r {
            SplineCarrierCertification::Unresolved(
                UnresolvedSplineReason::CircleMembershipNeedsBezierForm,
            ) => {}
            other => panic!("expected Unresolved CircleMembershipNeedsBezierForm, got {other:?}"),
        }
    }

    // -- degenerate candidate circle ---------------------------------------

    #[test]
    fn zero_radius_candidate_circle_is_unresolved() {
        let curve = Curve3D::NurbsCurve(quarter_circle_nurbs());
        let bad = CirclePlacement {
            center: Point3::new(0.0, 0.0, 0.0),
            radius: 0.0,
            normal: Vector3::new(0.0, 0.0, 1.0),
        };
        let r = certify_spline_carrier(&curve, CarrierQuery::CircularArcOn(bad), (0.0, 1.0));
        assert_eq!(
            r,
            SplineCarrierCertification::Unresolved(
                UnresolvedSplineReason::DegenerateCandidateCircle
            )
        );
    }

    // -- tag stability ------------------------------------------------------

    #[test]
    fn tags_are_stable_and_distinct() {
        let curve = Curve3D::BSplineCurve(line_bspline());
        let cert = certify_spline_carrier(&curve, CarrierQuery::StraightLine, (0.0, 1.0));
        assert_eq!(cert.tag(), "spline_carrier_certified_straight_line");
        let nonspline = certify_spline_carrier(
            &Curve3D::Line(StepLine(
                Point3::new(0.0, 0.0, 0.0),
                Point3::new(1.0, 0.0, 0.0),
            )),
            CarrierQuery::StraightLine,
            (0.0, 1.0),
        );
        assert_eq!(
            nonspline.tag(),
            "spline_carrier_not_a_spline_representation"
        );
    }

    // -- helper sanity ------------------------------------------------------

    #[test]
    fn exact_dot_helpers_decide_zero() {
        let z = Expansion::zero();
        let a = [z.clone(), z.clone(), z.clone()];
        assert!(dot_exp_f64(&a, &[0.0, 0.0, 1.0]).is_zero());
        let one = Expansion::zero().grow(1.0);
        let nz = [one, Expansion::zero(), Expansion::zero()];
        assert!(!dot_exp_f64(&nz, &[1.0, 0.0, 0.0]).is_zero());
        assert!(dot_exp_exp(&nz, &a).is_zero());
        assert!(!dot_exp_exp(&nz, &nz).is_zero());
    }

    #[test]
    fn binom_is_exact_for_small_n() {
        assert_eq!(binom(0, 0), 1.0);
        assert_eq!(binom(2, 1), 2.0);
        assert_eq!(binom(4, 2), 6.0);
        assert_eq!(binom(5, 0), 1.0);
        assert_eq!(binom(5, 5), 1.0);
        assert_eq!(binom(6, 3), 20.0);
    }

    /// A NURBS that is visually near the unit circle but not on it (one
    /// control point moved by a small but real amount) must be certified
    /// `Inconsistent`, never waved through.
    #[test]
    fn visually_near_but_not_on_circle_is_inconsistent() {
        let knots = KnotVec::bezier_knot(2);
        let s2 = std::f64::consts::FRAC_1_SQRT_2;
        // Quarter circle with the middle control point moved by 1e-6 in y —
        // visually a circle, exactly not one. The endpoints (1,0) and (0,1)
        // are still on the unit circle, so the endpoint check passes and the
        // Bernstein coefficient of |S|^2 - r^2 w^2 fires.
        let cps = vec![
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(1.0, 1.0 + 1e-6, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        ];
        let nurbs = NurbsCurve::try_from_bspline_and_weights(
            BSplineCurve::new(knots, cps),
            vec![1.0, s2, 1.0],
        )
        .expect("weights match");
        let curve = Curve3D::NurbsCurve(nurbs);
        let r = certify_spline_carrier(
            &curve,
            CarrierQuery::CircularArcOn(unit_circle_placement()),
            (0.0, 1.0),
        );
        match r {
            SplineCarrierCertification::Inconsistent(InconsistencyWitness::NotOnCircle {
                ..
            }) => {}
            other => panic!("expected Inconsistent NotOnCircle, got {other:?}"),
        }
    }
}
