//! Structural identification of the authoritative support-surface schema.
//!
//! # Why this module exists
//!
//! Step 1 measured the corpus and found `0 / 24,199` faces resolving to a
//! certified ambient lattice, with `22,681` exiting as
//! `PeriodAbsenceNotEstablished`. That number is honest but it is not a fact
//! about the faces: `12,122` of them are planes, and a plane is aperiodic on
//! both axes by an analytic rule that has been in [`super::ambient`] since Step
//! 1 landed.
//!
//! Nothing could apply that rule, because the only surface description
//! reaching the tessellator is
//! [`super::super::domain::lattice::CertifiedLattice`], and that type's
//! `AxisPeriodStatus::NonPeriodic` has two producers — `NON_PERIODIC`, an
//! analytic claim about a plane, and `from_unevidenced_accessor(None)`, a bare
//! accessor that returned nothing. Once constructed the two are
//! indistinguishable, so the adapter must map both to `Undetermined` or it
//! would assign a torus a trivial deck group.
//!
//! The fix is not to weaken the adapter. It is to read the *representation*
//! before it is erased, one step earlier in the pipeline, and to hand the
//! formal resolver a witness that a plane was actually seen.
//!
//! # The admitted path
//!
//! ```text
//! authoritative support-surface schema identifies a plane
//!   -> analytic premise SupportSurfaceIsAPlane
//!   -> certify U-axis aperiodicity
//!   -> certify V-axis aperiodicity
//!   -> CertifiedAmbientLattice::Rank0
//! ```
//!
//! and the forbidden one:
//!
//! ```text
//! legacy lattice says NonPeriodic -> guess that the surface is a plane
//! period accessor returned None  -> infer aperiodicity
//! ```
//!
//! # How the premise is made unforgeable
//!
//! [`PlaneSchema`]'s fields are private and it has exactly one constructor:
//! [`identify_plane`], which takes `&truck_geometry::prelude::Plane` by value
//! reference. A caller cannot produce one without holding an actual plane
//! representation. [`super::ambient::certify_plane_aperiodicity`] takes
//! `&PlaneSchema`, so the premise `SupportSurfaceIsAPlane` is discharged by
//! *presenting the plane*, not by choosing to call a particular function.
//!
//! Every other schema is [`SupportSurfaceSchema::NotStructurallyIdentified`],
//! which carries no authority at all and whose constructor is public precisely
//! because it grants nothing. A composition layer that meets a surface it has
//! no structural reader for says so, and the face exits `Unresolved`.
//!
//! # Degenerate bases are not planes for this purpose
//!
//! `AnalyticRule::PlaneHasNoPeriodicDirection` stands on the parameterisation
//! being *injective* on `R^2`. `Plane::subs(u, v) = o + u(p-o) + v(q-o)` is
//! injective exactly when `p-o` and `q-o` are linearly independent. If they
//! are parallel the map is genuinely periodic — with `u_axis == v_axis`,
//! `S(u+1, v-1) = S(u, v)` for every `(u, v)` — so certifying absence from the
//! entity type alone would be *false*, not merely unproved. [`identify_plane`]
//! therefore refuses a basis it cannot separate, and the refusal is
//! [`SchemaIdentificationFailure::PlaneBasisNotSeparated`].

use super::numeric::{FiniteF64, NonNegativeFinite, NumericDomainError};
use truck_geometry::prelude::{InnerSpace, Plane, Point3, Vector3};

/// The dimensionless floor a plane's normalised Gram determinant must clear
/// before its basis counts as separated.
///
/// The criterion is `det(G) / (g00 * g11)`, which is `1 - cos^2(theta)` for the
/// angle between the two axes: scale-free, so it says nothing about the units
/// the model is in, and equal to `0` exactly when the axes are parallel.
///
/// `1e-9` corresponds to an angle of about `1.8e-3` degrees. The quantities
/// entering it are sums of three products of coordinates, so the relative error
/// of the computed value is a small multiple of `f64::EPSILON` (~2.2e-16)
/// against the larger of the two terms; a computed value at or above `1e-9` is
/// six orders of magnitude clear of that, so the sign and the separation are
/// both established rather than assumed.
///
/// This is a *structural* admissibility floor on the representation, not a
/// geometric tolerance, and it deliberately does not discharge Step 3's
/// conditioning obligation — Step 3 must still bound its own inverse error
/// against the tolerance it is asked to meet.
pub const MINIMUM_NORMALISED_GRAM_DETERMINANT: f64 = 1e-9;

/// The authoritative schema of one face's support surface.
///
/// Produced by a composition layer that can name the concrete surface
/// representation, and consumed by the ambient resolver. There is no variant
/// meaning "probably a plane".
#[derive(Debug, Clone, PartialEq)]
pub enum SupportSurfaceSchema {
    /// The representation *is* a plane, with a separated basis.
    Plane(PlaneSchema),
    /// Nothing structural was established. Carries no authority.
    NotStructurallyIdentified(SchemaIdentificationFailure),
}

impl SupportSurfaceSchema {
    /// The negative case. Public because it grants nothing: a caller that
    /// fabricates one only makes its own face exit `Unresolved`.
    pub fn not_structurally_identified(cause: SchemaIdentificationFailure) -> Self {
        Self::NotStructurallyIdentified(cause)
    }

    /// The plane schema, when there is one.
    pub fn plane(&self) -> Option<&PlaneSchema> {
        match self {
            Self::Plane(plane) => Some(plane),
            Self::NotStructurallyIdentified(_) => None,
        }
    }

    /// A short stable tag, for probe records and diagnostics.
    pub fn tag(&self) -> &'static str {
        match self {
            Self::Plane(_) => "plane",
            Self::NotStructurallyIdentified(cause) => cause.tag(),
        }
    }
}

/// Why no structural schema was established.
///
/// Each variant is a statement about *evidence*, never about the surface. A
/// surface that lands in [`Self::NoStructuralReader`] may well be a plane; what
/// is recorded is that nothing looked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemaIdentificationFailure {
    /// The composition layer has no structural reader for this representation.
    /// The expansion path for a surface class starts here.
    NoStructuralReader {
        /// The representation's own name, as the reader knows it.
        representation: &'static str,
    },
    /// A plane representation was read and its two axes could not be
    /// separated, so the parameterisation's injectivity — and with it the
    /// aperiodicity rule — is unavailable. See the module docs.
    PlaneBasisNotSeparated,
    /// A plane representation was read and a coordinate was `NaN` or infinite,
    /// so no predicate about it holds.
    PlaneBasisNotFinite {
        /// Why the value was refused.
        cause: NumericDomainError,
    },
}

impl SchemaIdentificationFailure {
    /// A short stable tag, for probe records.
    pub fn tag(self) -> &'static str {
        match self {
            Self::NoStructuralReader { .. } => "no_structural_reader",
            Self::PlaneBasisNotSeparated => "plane_basis_not_separated",
            Self::PlaneBasisNotFinite { .. } => "plane_basis_not_finite",
        }
    }
}

/// A support surface proved to be a plane with a separated basis.
///
/// The only constructor is [`identify_plane`]. Fields are private so the
/// witness cannot be assembled from numbers a caller happens to have.
///
/// The retained basis is the plane's *native* one — `o`, `p - o`, `q - o`
/// exactly as the representation stores them. It is deliberately neither
/// orthogonalised nor normalised: downstream parameter coordinates must stay in
/// the support surface's own chart, or every 2D coordinate this pipeline
/// produces would refer to a chart the source never declared.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlaneSchema {
    origin: Point3,
    u_axis: Vector3,
    v_axis: Vector3,
    gram: PlaneGram,
}

impl PlaneSchema {
    /// The plane's origin, `S(0, 0)`.
    pub fn origin(&self) -> Point3 {
        self.origin
    }

    /// The `u` direction: `S(1, 0) - S(0, 0)`. Not normalised.
    pub fn u_axis(&self) -> Vector3 {
        self.u_axis
    }

    /// The `v` direction: `S(0, 1) - S(0, 0)`. Not normalised.
    pub fn v_axis(&self) -> Vector3 {
        self.v_axis
    }

    /// The Gram matrix of the retained basis, with its determinant proved
    /// positive and separated.
    pub fn gram(&self) -> PlaneGram {
        self.gram
    }

    /// Evaluate the plane: `S(u, v) = o + u U + v V`.
    pub fn point_at(&self, u: f64, v: f64) -> Point3 {
        self.origin + u * self.u_axis + v * self.v_axis
    }
}

/// The Gram matrix of a plane's retained basis, with a separated determinant.
///
/// ```text
/// g00 = U . U     g01 = U . V     g11 = V . V
/// det = g00 g11 - g01^2
/// ```
///
/// Held as a type rather than recomputed at each use so that the separation
/// established once at identification is the separation every consumer reads.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlaneGram {
    g00: f64,
    g01: f64,
    g11: f64,
    determinant: f64,
    normalised_determinant: NonNegativeFinite,
}

impl PlaneGram {
    /// `U . U`, proved finite and positive.
    pub fn g00(self) -> f64 {
        self.g00
    }

    /// `U . V`, proved finite.
    pub fn g01(self) -> f64 {
        self.g01
    }

    /// `V . V`, proved finite and positive.
    pub fn g11(self) -> f64 {
        self.g11
    }

    /// `g00 g11 - g01^2`, proved finite and strictly positive.
    pub fn determinant(self) -> f64 {
        self.determinant
    }

    /// `det / (g00 g11)`, the dimensionless separation, proved to clear
    /// [`MINIMUM_NORMALISED_GRAM_DETERMINANT`].
    pub fn normalised_determinant(self) -> NonNegativeFinite {
        self.normalised_determinant
    }

    /// Solve `G [u v]^T = [r0 r1]^T` for the plane parameters of a point whose
    /// in-plane offsets against the basis are `r0 = (P-O).U`, `r1 = (P-O).V`.
    ///
    /// This is the whole reason the Gram matrix is retained. The naive
    /// per-axis form `u = ((P-O).U)/(U.U)` is correct only for an *orthogonal*
    /// basis, and STEP planes are under no obligation to supply one.
    pub fn solve(self, r0: f64, r1: f64) -> (f64, f64) {
        let u = (self.g11 * r0 - self.g01 * r1) / self.determinant;
        let v = (-self.g01 * r0 + self.g00 * r1) / self.determinant;
        (u, v)
    }
}

/// Read a plane representation structurally.
///
/// The single introduction rule for [`PlaneSchema`], and therefore the single
/// route by which the premise `SupportSurfaceIsAPlane` can enter the formal
/// system. It refuses a non-finite or unseparated basis; see the module docs
/// for why the second refusal is a soundness requirement and not caution.
pub fn identify_plane(plane: &Plane) -> SupportSurfaceSchema {
    let origin = plane.origin();
    let u_axis = plane.u_axis();
    let v_axis = plane.v_axis();

    let finite = |value: f64| FiniteF64::new(value).map(FiniteF64::get);
    let coordinates = [
        origin.x, origin.y, origin.z, u_axis.x, u_axis.y, u_axis.z, v_axis.x, v_axis.y, v_axis.z,
    ];
    for coordinate in coordinates {
        if let Err(cause) = finite(coordinate) {
            return SupportSurfaceSchema::NotStructurallyIdentified(
                SchemaIdentificationFailure::PlaneBasisNotFinite { cause },
            );
        }
    }

    let g00 = u_axis.dot(u_axis);
    let g01 = u_axis.dot(v_axis);
    let g11 = v_axis.dot(v_axis);
    let determinant = g00 * g11 - g01 * g01;
    let scale = g00 * g11;

    // A zero-length axis makes `scale` zero and the normalisation undefined.
    // That is `PlaneBasisNotSeparated` and not a numeric failure: a degenerate
    // axis is precisely an unseparated basis.
    if !(scale > 0.0) || !determinant.is_finite() {
        return SupportSurfaceSchema::NotStructurallyIdentified(
            SchemaIdentificationFailure::PlaneBasisNotSeparated,
        );
    }
    let normalised = determinant / scale;
    let Ok(normalised_determinant) = NonNegativeFinite::new(normalised) else {
        return SupportSurfaceSchema::NotStructurallyIdentified(
            SchemaIdentificationFailure::PlaneBasisNotSeparated,
        );
    };
    if normalised_determinant.get() < MINIMUM_NORMALISED_GRAM_DETERMINANT {
        return SupportSurfaceSchema::NotStructurallyIdentified(
            SchemaIdentificationFailure::PlaneBasisNotSeparated,
        );
    }

    SupportSurfaceSchema::Plane(PlaneSchema {
        origin,
        u_axis,
        v_axis,
        gram: PlaneGram {
            g00,
            g01,
            g11,
            determinant,
            normalised_determinant,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit_plane() -> Plane {
        Plane::new(
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        )
    }

    /// A skewed, unequally scaled, non-axis-aligned basis. Legal STEP, and the
    /// case the naive per-axis inverse gets wrong.
    fn skew_plane() -> Plane {
        Plane::new(
            Point3::new(1.0, 2.0, 3.0),
            Point3::new(4.0, 2.0, 3.0),
            Point3::new(2.0, 5.0, 3.0),
        )
    }

    #[test]
    fn an_orthonormal_plane_is_identified() {
        let schema = identify_plane(&unit_plane());
        let plane = schema.plane().expect("a unit plane is a plane");
        assert_eq!(plane.origin(), Point3::new(0.0, 0.0, 0.0));
        assert_eq!(plane.u_axis(), Vector3::new(1.0, 0.0, 0.0));
        assert_eq!(plane.v_axis(), Vector3::new(0.0, 1.0, 0.0));
        assert_eq!(plane.gram().normalised_determinant().get(), 1.0);
    }

    #[test]
    fn the_retained_basis_is_native_and_unnormalised() {
        let plane = identify_plane(&skew_plane());
        let plane = plane.plane().expect("a skewed plane is still a plane");
        // `p - o` = (3, 0, 0), magnitude 3, not 1: nothing normalised it.
        assert_eq!(plane.u_axis(), Vector3::new(3.0, 0.0, 0.0));
        assert_eq!(plane.v_axis(), Vector3::new(1.0, 3.0, 0.0));
        // And nothing orthogonalised it either.
        assert_ne!(plane.gram().g01(), 0.0);
    }

    #[test]
    fn the_gram_solve_inverts_the_skewed_parameterisation() {
        let schema = identify_plane(&skew_plane());
        let plane = schema.plane().expect("plane");
        let gram = plane.gram();
        for (u, v) in [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (-2.5, 7.25)] {
            let point = plane.point_at(u, v);
            let offset = point - plane.origin();
            let (su, sv) = gram.solve(offset.dot(plane.u_axis()), offset.dot(plane.v_axis()));
            assert!((su - u).abs() < 1e-12, "u: {su} vs {u}");
            assert!((sv - v).abs() < 1e-12, "v: {sv} vs {v}");
        }
    }

    #[test]
    fn the_naive_per_axis_inverse_would_have_been_wrong_here() {
        // Guards the reason `PlaneGram::solve` exists: on this basis the
        // per-axis quotient disagrees with the true parameter, so a future
        // simplification to `((P-O).U)/(U.U)` fails this test rather than
        // silently shifting every projected coordinate.
        let schema = identify_plane(&skew_plane());
        let plane = schema.plane().expect("plane");
        let point = plane.point_at(0.0, 1.0);
        let offset = point - plane.origin();
        let naive_u = offset.dot(plane.u_axis()) / plane.gram().g00();
        assert!(
            naive_u.abs() > 0.3,
            "the naive inverse should be visibly wrong, got {naive_u}"
        );
        let (u, v) = plane
            .gram()
            .solve(offset.dot(plane.u_axis()), offset.dot(plane.v_axis()));
        assert!(u.abs() < 1e-12 && (v - 1.0).abs() < 1e-12);
    }

    #[test]
    fn parallel_axes_are_not_a_plane_schema() {
        // u_axis = (1,0,0), v_axis = (2,0,0). `S(u + 2t, v - t) = S(u, v)`, so
        // this parameterisation *is* periodic and certifying absence from it
        // would be false.
        let degenerate = Plane::new(
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(2.0, 0.0, 0.0),
        );
        assert_eq!(
            identify_plane(&degenerate),
            SupportSurfaceSchema::NotStructurallyIdentified(
                SchemaIdentificationFailure::PlaneBasisNotSeparated
            )
        );
    }

    #[test]
    fn a_zero_length_axis_is_not_a_plane_schema() {
        let degenerate = Plane::new(
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        );
        assert_eq!(
            identify_plane(&degenerate),
            SupportSurfaceSchema::NotStructurallyIdentified(
                SchemaIdentificationFailure::PlaneBasisNotSeparated
            )
        );
    }

    #[test]
    fn a_nearly_parallel_basis_is_refused() {
        // 1e-6 radians of separation gives a normalised determinant of ~1e-12,
        // below the floor.
        let nearly = Plane::new(
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(1.0, 1e-6, 0.0),
        );
        assert_eq!(
            identify_plane(&nearly),
            SupportSurfaceSchema::NotStructurallyIdentified(
                SchemaIdentificationFailure::PlaneBasisNotSeparated
            )
        );
    }

    #[test]
    fn a_tiny_but_well_separated_basis_is_accepted() {
        // Separation is dimensionless, so a plane in metres whose basis
        // vectors are micrometres long is identified exactly like a unit one.
        let tiny = Plane::new(
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1e-6, 0.0, 0.0),
            Point3::new(0.0, 1e-6, 0.0),
        );
        assert!(identify_plane(&tiny).plane().is_some());
    }

    #[test]
    fn a_non_finite_coordinate_is_refused_as_such() {
        let broken = Plane::new(
            Point3::new(f64::NAN, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        );
        assert_eq!(
            identify_plane(&broken),
            SupportSurfaceSchema::NotStructurallyIdentified(
                SchemaIdentificationFailure::PlaneBasisNotFinite {
                    cause: NumericDomainError::NotANumber
                }
            )
        );
    }

    #[test]
    fn the_negative_case_carries_no_plane() {
        let schema = SupportSurfaceSchema::not_structurally_identified(
            SchemaIdentificationFailure::NoStructuralReader {
                representation: "toroidal_surface",
            },
        );
        assert!(schema.plane().is_none());
        assert_eq!(schema.tag(), "no_structural_reader");
    }
}

// ---------------------------------------------------------------------------
// Curve representations
// ---------------------------------------------------------------------------

/// The authoritative schema of one edge's 3D curve.
///
/// The same discipline as [`SupportSurfaceSchema`], for the same reason:
/// `Step 3` has to establish a curve-on-surface relation over the *complete*
/// trimmed interval, and which certificate route is available is a fact about
/// the representation, not about a handful of sampled points.
///
/// Only the two families whose planar projection is *exact* are read. Circles,
/// ellipses, splines and p-curves each need their own whole-interval
/// certificate and are P2 coverage work; naming them in the refusal is what
/// lets the corpus rank them.
#[derive(Debug, Clone, PartialEq)]
pub enum CurveSchema {
    /// A straight segment, complete over its trimmed interval.
    LineSegment(PolylineSchema),
    /// A polyline. Every source segment maps exactly to a 2D segment.
    Polyline(PolylineSchema),
    /// A circular arc, structurally identified but *not* exactly polygonal:
    /// [`Self::polygonal`] returns `None` for this variant, exactly as it
    /// does for [`Self::NotStructurallyIdentified`]. Its purpose is to let
    /// [`Self::is_structurally_identified`] tell a Step-2 traversal gate
    /// that a reader *did* read this representation, so a caller whose
    /// downstream stage understands arcs (the rank-1 cylinder witness route;
    /// see `super::curve_witness`) can admit it past that gate without
    /// touching the polygonal-only planar Step 3
    /// (`super::planar_slice::certified_planar_curves`), which still exits
    /// `UnsupportedCurveRepresentation` on it exactly as it does for
    /// `NotStructurallyIdentified` today.
    ///
    /// The payload is the arc's own placement in the curve's own parameter
    /// basis, carried so that a stage which understands arcs can obtain it
    /// from the schema the entry point already threads, rather than from a
    /// second adapter parameter. It is *evidence*, not authority: the reader
    /// that builds it is the only thing that certifies circularity, and every
    /// consumer still discharges its own curve-on-surface obligation against
    /// it. A consumer that does not understand arcs is unaffected — the
    /// variant is as unusable to it as it was when it carried nothing.
    CircularArc(CircularArcPlacement3),
    /// Nothing structural was established. Carries no authority.
    NotStructurallyIdentified(CurveSchemaFailure),
}

/// A source circle's placement, in the curve's own parameter basis.
///
/// ```text
/// point(t) = center + cos_basis * cos(t) + sin_basis * sin(t)
/// ```
///
/// The 3D counterpart of [`super::curve2d::DirectedCircularArc2`], and
/// deliberately the same shape: developing one into a certified planar chart
/// is then an affine map applied to three vectors, with no reparameterization
/// and no resampling.
///
/// `parameter_interval` is the source curve's own trimmed interval in the
/// curve's own direction, with the representation's orientation folded in
/// exactly once by the reader. It is never reduced modulo `TAU`, never
/// shortened to the "near" sweep, and never inferred from coincident
/// endpoints — the same discipline
/// [`super::curve2d::DirectedCircularArc2`]'s `t0`/`t1` carry, stated here
/// because this is where the value crosses the importer seam.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CircularArcPlacement3 {
    /// The circle center.
    pub center: Point3,
    /// The transformed image of the unit circle's `e_x`.
    pub cos_basis: Vector3,
    /// The transformed image of the unit circle's `e_y`.
    pub sin_basis: Vector3,
    /// The source curve's own trimmed interval, in the curve's own direction.
    pub parameter_interval: (f64, f64),
}

impl CurveSchema {
    /// The negative case. Public because it grants nothing.
    pub fn not_structurally_identified(cause: CurveSchemaFailure) -> Self {
        Self::NotStructurallyIdentified(cause)
    }

    /// The vertex chain, when this curve has an exact polygonal
    /// representation.
    pub fn polygonal(&self) -> Option<&PolylineSchema> {
        match self {
            Self::LineSegment(schema) | Self::Polyline(schema) => Some(schema),
            Self::CircularArc(_) | Self::NotStructurallyIdentified(_) => None,
        }
    }

    /// The arc placement, when a reader identified this curve as a circle.
    ///
    /// The arc counterpart of [`Self::polygonal`]: a stage reads whichever
    /// accessor matches the certificate route it can discharge, and gets
    /// `None` from the other.
    pub fn circular_arc(&self) -> Option<&CircularArcPlacement3> {
        match self {
            Self::CircularArc(placement) => Some(placement),
            Self::LineSegment(_) | Self::Polyline(_) | Self::NotStructurallyIdentified(_) => None,
        }
    }

    /// Whether *some* structural reader succeeded for this curve, whether or
    /// not that reader's result is the exact polygonal chain
    /// [`Self::polygonal`] returns. A Step-2 traversal gate that admits any
    /// structurally-identified representation (rather than only the
    /// polygonal ones) reads this instead of [`Self::polygonal`].
    pub fn is_structurally_identified(&self) -> bool {
        !matches!(self, Self::NotStructurallyIdentified(_))
    }

    /// A short stable tag, for probe records.
    pub fn tag(&self) -> &'static str {
        match self {
            Self::LineSegment(_) => "line_segment",
            Self::Polyline(_) => "polyline",
            Self::CircularArc(_) => "circular_arc",
            Self::NotStructurallyIdentified(cause) => cause.tag(),
        }
    }
}

/// Why no structural curve schema was established.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurveSchemaFailure {
    /// The composition layer has no structural reader for this representation.
    NoStructuralReader {
        /// The representation's own name.
        representation: &'static str,
    },
    /// A polygonal representation was read and a coordinate was not finite.
    VertexNotFinite {
        /// Why the value was refused.
        cause: NumericDomainError,
    },
    /// A polygonal representation was read and had fewer than two vertices, so
    /// it traverses nothing and has no direction.
    FewerThanTwoVertices,
}

impl CurveSchemaFailure {
    /// A short stable tag, for probe records.
    pub fn tag(self) -> &'static str {
        match self {
            Self::NoStructuralReader { .. } => "curve_no_structural_reader",
            Self::VertexNotFinite { .. } => "curve_vertex_not_finite",
            Self::FewerThanTwoVertices => "curve_fewer_than_two_vertices",
        }
    }
}

/// A chain of at least two finite points, in the curve's own parameter
/// direction, covering the complete trimmed interval.
///
/// Private field and no public constructor: the claim "these points *are* the
/// complete trimmed curve" is what the type carries, and it is discharged by
/// the readers below reading a representation that has no other content.
#[derive(Debug, Clone, PartialEq)]
pub struct PolylineSchema {
    vertices: Vec<Point3>,
}

impl PolylineSchema {
    /// The vertex chain, in the curve's own parameter direction.
    pub fn vertices(&self) -> &[Point3] {
        &self.vertices
    }

    /// The first point: the curve at its trimmed interval's lower end.
    pub fn start(&self) -> Point3 {
        self.vertices[0]
    }

    /// The last point: the curve at its trimmed interval's upper end.
    pub fn end(&self) -> Point3 {
        self.vertices[self.vertices.len() - 1]
    }
}

fn polyline_schema(vertices: Vec<Point3>) -> Result<PolylineSchema, CurveSchemaFailure> {
    if vertices.len() < 2 {
        return Err(CurveSchemaFailure::FewerThanTwoVertices);
    }
    for vertex in &vertices {
        for coordinate in [vertex.x, vertex.y, vertex.z] {
            if let Err(cause) = FiniteF64::new(coordinate) {
                return Err(CurveSchemaFailure::VertexNotFinite { cause });
            }
        }
    }
    Ok(PolylineSchema { vertices })
}

/// Read a `Line` structurally.
///
/// `Line(a, b)` is `subs(t) = a + t(b - a)` on the parameter range `0..=1` —
/// the representation *is* the segment, with no trimming to reconcile — so the
/// complete trimmed occurrence is the two endpoints and nothing is
/// approximated. That exactness is why line segments are the first admitted
/// curve family.
pub fn identify_line_segment(line: &truck_geometry::prelude::Line<Point3>) -> CurveSchema {
    match polyline_schema(vec![line.0, line.1]) {
        Ok(schema) => CurveSchema::LineSegment(schema),
        Err(cause) => CurveSchema::NotStructurallyIdentified(cause),
    }
}

/// Read a `PolylineCurve` structurally.
///
/// `subs` interpolates linearly between consecutive vertices on `0..=n-1`, so
/// each source segment maps to exactly one 2D segment under an affine
/// projection and the approximation error is again zero.
pub fn identify_polyline(curve: &[Point3]) -> CurveSchema {
    match polyline_schema(curve.to_vec()) {
        Ok(schema) => CurveSchema::Polyline(schema),
        Err(cause) => CurveSchema::NotStructurallyIdentified(cause),
    }
}

#[cfg(test)]
mod curve_tests {
    use super::*;
    use truck_geometry::prelude::Line;

    #[test]
    fn a_line_is_its_own_complete_trimmed_occurrence() {
        let line = Line(Point3::new(0.0, 0.0, 0.0), Point3::new(3.0, 4.0, 0.0));
        let schema = identify_line_segment(&line);
        let polygonal = schema.polygonal().expect("a line is polygonal");
        assert_eq!(polygonal.vertices().len(), 2);
        assert_eq!(polygonal.start(), Point3::new(0.0, 0.0, 0.0));
        assert_eq!(polygonal.end(), Point3::new(3.0, 4.0, 0.0));
    }

    #[test]
    fn a_degenerate_polyline_is_refused() {
        assert_eq!(
            identify_polyline(&[Point3::new(0.0, 0.0, 0.0)]),
            CurveSchema::NotStructurallyIdentified(CurveSchemaFailure::FewerThanTwoVertices)
        );
        assert_eq!(
            identify_polyline(&[]).tag(),
            "curve_fewer_than_two_vertices"
        );
    }

    #[test]
    fn a_non_finite_vertex_is_refused() {
        let broken = [
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(f64::INFINITY, 0.0, 0.0),
        ];
        assert_eq!(
            identify_polyline(&broken),
            CurveSchema::NotStructurallyIdentified(CurveSchemaFailure::VertexNotFinite {
                cause: NumericDomainError::Infinite
            })
        );
    }
}
