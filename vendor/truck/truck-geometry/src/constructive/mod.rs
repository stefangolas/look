#![deny(clippy::unwrap_used)]

//! BG-CG-000-CONTRACT — the constructive geometry contract skeleton.
//!
//! Index identity (frozen at BG-CG-000-CONTRACT; two consumers: the direct
//! facet backend's grid registry and the meshalgo edge-sample ledger):
//!
//! A mesh position index is a pure function of (entity identity, sample
//! ordinal) — never of coordinates.
//!
//! - Each unique `EdgeID<Curve>` is sampled once; a reversed edge consumes the
//!   same integer sequence, reversed.
//! - Watertightness invariant: for incident faces A, B sharing edge E,
//!   I(A, E) == reverse(I(B, E)) **as integer sequences**.
//! - If the shell is combinatorially closed and every boundary mesh vertex's
//!   index derives from (EdgeID, ordinal), the emitted mesh is closed by
//!   construction; positional welding (`put_together_same_attrs`) is never
//!   invoked.
//! - The ledger carrier itself — `EdgeSampleLedger { edge_id: EdgeID<Curve>,
//!   parameters: Vec<f64>, position_indices: Vec<usize> }` — lands in
//!   truck-meshalgo (CG-005), not here.
//! - Implementation shape: a NEW parallel entry point
//!   (`triangulation_with_ledger`-style) reusing the existing unique-edge
//!   sampling and per-face CDT internals; the existing entry points remain
//!   bit-identical.
//! - FAC (CG-004): grid vertex (i, j) is created exactly once via a private
//!   grid registry keyed by (entity identity, sample ordinal); adjacent faces
//!   reuse the identity; internal grid edges are created once and traversed
//!   oppositely by their two faces.
//!
//! Certificate mapping (frozen at BG-CG-000-CONTRACT; CG-007 implements it
//! and cannot be dispatched against an unfrozen mapping). New evidence
//! composes with the existing vocabulary — `MeshedShellOutcome`,
//! `FaceValidityCertificate`, `ProvenanceRecord` — never a parallel validation
//! universe.
//!
//! | Evidence kind | Carrier | Where the variant lands |
//! |---|---|---|
//! | Recipe construct refusals — every `ConstructError` variant (spine/frame validity, profile collapse, correspondence mismatch) | `Refusal::UnsupportedEnvelope(EnvelopeCase::ConstructRefused)` at the realization entry; the detailed `ConstructError` rides the realization evidence record | NEW unit variant `EnvelopeCase::ConstructRefused` in `truck-base/src/evidence.rs`; NEW `RealizationEvidence` type in truck-meshalgo (CG-007) |
//! | Jacobian bounds (frame conditioning during realization) | per-face, positionally aligned with `shell.faces` exactly as `MeshedShellOutcome::face_failures` is | NEW `RealizationCertificate` struct + NEW field on the CG-004 realization outcome (CG-007 fills it); deliberately NOT a widening of `FaceValidityCertificate` — different vocabulary, the same separation doctrine as `band_attempts` vs `cone_band_attempts` |
//! | Shared-edge pair errors (`EdgeID` + FaceID A + FaceID B + error_a + error_b) | NEW field `shared_edge_pairs: Vec<SharedEdgePairEvidence>` on the realization outcome | NEW `SharedEdgePairEvidence` struct (CG-007); never a `ProvenanceRecord` variant (that type is `Copy + Eq`; the payload carries f64s) |
//! | Winding audit (twin-triangle) | a three-valued verdict carried beside the emitted `PolygonMesh` | NEW `RealizationVerdict { CertifiedWithinTolerance, Failed, Inconclusive }` (CG-007); winding-audit failure is `FAILED`, never a warning; uncertainty is `INCONCLUSIVE`, never converted into success |
//! | Any other realization-stage per-face evidence | the existing `MeshedShellOutcome` positional-vector doctrine | new vocabulary = a new `Vec<Option<...>>` field aligned with `shell.faces`; never a widening of an existing vector |
//!
//! Standing notes: construct-stage failures predate meshing, so they never
//! enter `MeshedShellOutcome` (there is no shell to annotate). Every value
//! computed in floats certifies `Method::Float` (H-6), never `Method::Exact`.
//! Verdicts are three-valued throughout: `CERTIFIED_WITHIN_TOLERANCE | FAILED
//! | INCONCLUSIVE`.

use truck_base::cgmath64::*;

mod errors;
mod frame_fixed;
mod frame_radial;
mod frame_transport;
mod frame_up;
pub mod intersection_carrier;
mod profile;
mod recipe;
mod sampling;
mod spine_ph;
mod sweep_surface;
pub mod validation;

/// The orthonormal right-handed frame at one spine station.
///
/// Convention (normative): `tangent` is the spine direction, and the triple
/// (tangent, normal, binormal) satisfies `tangent × normal == binormal` and
/// unit lengths — i.e. `n = b × t`, `b = t × n`, matching the plan's
/// `FixedPlane` semantics (`t = C'/‖C'‖`, `b` = the plane normal, `n = b × t`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frame3 {
    /// The unit tangent — the spine direction.
    pub tangent: Vector3,
    /// The unit normal.
    pub normal: Vector3,
    /// The unit binormal.
    pub binormal: Vector3,
}

impl Frame3 {
    /// Validates and builds a frame: every component finite, every vector unit
    /// length, all three pairwise orthogonal, and the triple right-handed
    /// (`tangent × normal` equals `binormal`) — all compared at
    /// `DirectTolerance::default()`'s `position` bound. Constructor validation
    /// has no spine parameter, so every failure is `ConstructError::InvalidInput`.
    pub fn try_new(
        tangent: Vector3,
        normal: Vector3,
        binormal: Vector3,
    ) -> Result<Frame3, ConstructError> {
        let tolerance = DirectTolerance::default().position;
        let finite = tangent.x.is_finite()
            && tangent.y.is_finite()
            && tangent.z.is_finite()
            && normal.x.is_finite()
            && normal.y.is_finite()
            && normal.z.is_finite()
            && binormal.x.is_finite()
            && binormal.y.is_finite()
            && binormal.z.is_finite();
        if !finite {
            return Err(ConstructError::InvalidInput);
        }
        let unit_length = (tangent.magnitude() - 1.0).abs() <= tolerance
            && (normal.magnitude() - 1.0).abs() <= tolerance
            && (binormal.magnitude() - 1.0).abs() <= tolerance;
        if !unit_length {
            return Err(ConstructError::InvalidInput);
        }
        let orthogonal = tangent.dot(normal).abs() <= tolerance
            && tangent.dot(binormal).abs() <= tolerance
            && normal.dot(binormal).abs() <= tolerance;
        if !orthogonal {
            return Err(ConstructError::InvalidInput);
        }
        let right_handed = (tangent.cross(normal) - binormal).magnitude() <= tolerance;
        if !right_handed {
            return Err(ConstructError::InvalidInput);
        }
        Ok(Frame3 {
            tangent,
            normal,
            binormal,
        })
    }
}

/// Which frame law a recipe carries, and its normative semantics.
///
/// Every law's constructor routes its result through [`Frame3::try_new`]
/// (ORI-FRAME-ORTHONORMALITY-GATE-001); a triple that fails the gate is
/// refused as `ConstructError::FrameSingular`, never emitted.
///
/// - `FixedPlane`: `t = C'/‖C'‖`, `b = normal`, `n = b × t`; refuse
///   `‖C'‖ < tolerance`. Orthonormal only while `t ⊥ b`, so a non-planar
///   spine refuses in v1 (projection is a later amendment). Preferred for
///   planar spines.
/// - `ArchitecturalUp`: `b = normalize(up × t)`, `n = b × t` (so `t × n = b`,
///   the right-handed completion); refuse `up ∥ t`. No silent frame rotation.
/// - `ParallelTransport`: Bishop rotation-minimizing frame via the
///   double-reflection method; stable at zero curvature and inflections;
///   deterministic from `initial_normal`. Frenet framing is never the default.
/// - `RadialAboutAxis`: analytic from the axis; rotated copies equivariant
///   modulo floating-point. Orthonormal only while `t ⊥ n`, so a tangent with
///   a radial component refuses.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FrameLaw {
    /// Pin the binormal to a fixed plane normal (planar spines).
    FixedPlane {
        /// The plane's unit normal; becomes the binormal.
        normal: Vector3,
    },
    /// The architectural up vector `up` (e.g. world +Z); refuses when
    /// `up ∥ tangent`.
    ArchitecturalUp {
        /// The preferred up direction.
        up: Vector3,
    },
    /// Rotation-minimizing (Bishop) frame, double-reflection method,
    /// deterministic from the initial normal.
    ParallelTransport {
        /// The normal at the spine's start station.
        initial_normal: Vector3,
    },
    /// Frames derived analytically from a fixed axis (revolved shapes).
    RadialAboutAxis {
        /// A point on the axis.
        origin: Point3,
        /// The axis direction.
        axis: Vector3,
    },
}

impl FrameLaw {
    /// The stable law name carried by `ConstructError::FrameSingular`'s `law`
    /// field: exactly `"FixedPlane"`, `"ArchitecturalUp"`,
    /// `"ParallelTransport"`, or `"RadialAboutAxis"`.
    pub fn law_name(&self) -> &'static str {
        match *self {
            FrameLaw::FixedPlane { .. } => "FixedPlane",
            FrameLaw::ArchitecturalUp { .. } => "ArchitecturalUp",
            FrameLaw::ParallelTransport { .. } => "ParallelTransport",
            FrameLaw::RadialAboutAxis { .. } => "RadialAboutAxis",
        }
    }
}

/// A closed polygonal profile in the frame plane.
///
/// Semantics (normative for CG-001): vertices are ordered CCW about the
/// profile normal; edge `i` connects vertex `i` to vertex `(i + 1) mod k`;
/// the closing edge is implicit and never stored; no self-intersection.
#[derive(Debug, Clone, PartialEq)]
pub struct Profile2D {
    /// The distinct vertices, in CCW order.
    pub vertices: Vec<Point2>,
}

impl Profile2D {
    /// Structural validation: at least three vertices, every coordinate
    /// finite. (Per-station collapse is an evaluation-time
    /// `ConstructError::ProfileCollapse`, CG-001's business.)
    pub fn try_closed(vertices: Vec<Point2>) -> Result<Profile2D, ConstructError> {
        if vertices.len() < 3 {
            return Err(ConstructError::InvalidInput);
        }
        let finite = vertices.iter().all(|v| v.x.is_finite() && v.y.is_finite());
        if !finite {
            return Err(ConstructError::InvalidInput);
        }
        Ok(Profile2D { vertices })
    }
}

/// A scalar function of the normalized spine parameter `s ∈ [0, 1]`.
///
/// Pre-decided here (the plan's `Scale` variant names this type without
/// defining it; CG-001 may add variants additively). `Linear` interpolates
/// `start + (end - start) * s` — total, no clamping.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScalarLaw {
    /// A constant scalar.
    Constant(f64),
    /// Linear interpolation from `start` at s=0 to `end` at s=1 (linear
    /// extrapolation outside). A `Scale` profile law whose scalar reaches
    /// zero collapses the profile — refused as `ProfileCollapse` at
    /// evaluation time (CG-001), never silently.
    Linear {
        /// The value at s = 0.
        start: f64,
        /// The value at s = 1.
        end: f64,
    },
}

impl ScalarLaw {
    /// The scalar at `s`. Total arithmetic; non-finite inputs propagate
    /// (detection is the evaluator's job, CG-001).
    pub fn at(&self, s: f64) -> f64 {
        match *self {
            ScalarLaw::Constant(c) => c,
            ScalarLaw::Linear { start, end } => start + (end - start) * s,
        }
    }
}

/// How the profile evolves along the spine.
///
/// `LinearCorrespondence` requires an EXPLICIT declared vertex/edge
/// correspondence between start and end; correspondence is never inferred.
/// Here the declaration is positional: vertex `i` of `start` corresponds to
/// vertex `i` of `end`. Arbitrary split/merge profile topology is out of
/// scope.
#[derive(Debug, Clone, PartialEq)]
pub enum ProfileLaw {
    /// The same profile at every station.
    Constant(Profile2D),
    /// One profile, uniformly scaled by a scalar law.
    Scale {
        /// The profile being scaled.
        profile: Profile2D,
        /// The scalar law over normalized s.
        scale: ScalarLaw,
    },
    /// Start and end profiles with declared positional correspondence;
    /// intermediate stations interpolate vertex-wise.
    LinearCorrespondence {
        /// The profile at s = 0.
        start: Profile2D,
        /// The profile at s = 1 (same vertex count as `start`).
        end: Profile2D,
    },
}

impl ProfileLaw {
    /// The validated `LinearCorrespondence` constructor: equal vertex counts
    /// or `ConstructError::ProfileCorrespondenceMismatch`; finite fixture
    /// data or `ConstructError::InvalidInput`.
    pub fn try_linear_correspondence(
        start: Profile2D,
        end: Profile2D,
    ) -> Result<ProfileLaw, ConstructError> {
        if start.vertices.len() != end.vertices.len() {
            return Err(ConstructError::ProfileCorrespondenceMismatch);
        }
        let finite = start
            .vertices
            .iter()
            .chain(end.vertices.iter())
            .all(|v| v.x.is_finite() && v.y.is_finite());
        if !finite {
            return Err(ConstructError::InvalidInput);
        }
        Ok(ProfileLaw::LinearCorrespondence { start, end })
    }
}

/// The tolerance bundle of the direct realization path.
///
/// Placement decision (booked): lives here in truck-geometry, not truck-base,
/// so CG-000 stays additive over the existing tree.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DirectTolerance {
    /// World-space distance comparisons (realization output).
    pub position: f64,
    /// Spine/profile parameter-space comparisons, including the C¹
    /// tangent-discontinuity detection threshold.
    pub parameter: f64,
    /// The bound on frame-Jacobian conditioning deviation.
    pub jacobian: f64,
    /// Shared-edge pair error comparison bounds.
    pub intersection: f64,
}

impl Default for DirectTolerance {
    /// Every field defaults to `truck_base::tolerance::TOLERANCE` (the plan:
    /// "defaults derive from truck_base::tolerance").
    fn default() -> Self {
        let t = truck_base::tolerance::TOLERANCE;
        Self {
            position: t,
            parameter: t,
            jacobian: t,
            intersection: t,
        }
    }
}

pub use errors::ConstructError;
pub use intersection_carrier::CertifiedImplicitIntersectionCurve;
pub use recipe::SpineFrameRecipe;
pub use recipe::{FrameData, LineSpine, PolylineSpine, Spine, SpineCurve};
pub use sampling::SamplingPolicy;
pub use spine_ph::{PendingMembership, PhSpine, RmErfSeptic, RrmfQuintic, SepticMembership};
pub use sweep_surface::SpineFrameSweep;
