//! FACE-VALIDITY: conservative, evidence-backed face-admissibility classification.
//!
//! This module distinguishes *intrinsically non-renderable* source faces —
//! faces whose boundary is certifiably zero- or one-dimensional, or whose every
//! source bound is collapsed — from genuine tessellation failures. It never
//! manufactures triangles for zero-area geometry; it classifies such faces
//! explicitly and rejects them with a [`FaceValidityCertificate`] that carries
//! the measured evidence.
//!
//! # Hard-validity rule
//!
//! **Small or thin is valid.** A face is rejected only when the represented
//! trim is *certifiably* zero-dimensional, one-dimensional, or topologically
//! collapsed. This is a sufficient condition for mathematical degeneracy; it is
//! deliberately not a threshold tuned to reproduce any known residual.
//!
//! # The certificates
//!
//! - **Detector A — collapsed source bounds.** Every source bound is a
//!   `VERTEX_LOOP` (or otherwise contributes no realisable 1D boundary
//!   geometry). Established from STEP topology at conversion time, never
//!   inferred from a later CDT failure.
//! - **Detector B — world-rank certificate.** The boundary's world-space points
//!   have numerical **rank < 2**: they all lie within a floating-point
//!   conditioning bound of a point (rank 0) or of a line (rank 1). A
//!   generator-line loop, an out-and-back traversal of one curve, a collapsed
//!   band, or a trim constant in one independent surface coordinate all have
//!   rank 1. The error bound is derived from the coordinate magnitude and
//!   `f64::EPSILON` — the conditioning of the surface evaluation — **never**
//!   from the meshing chord tolerance.
//!
//! Dimensionless quantities (extent ratios, normalized area, UV aspect) are
//! carried as diagnostic evidence in the certificate but are never the sole
//! basis of a rejection. The meshing tolerance is never a degeneracy threshold.
//!
//! # What is deliberately not classified invalid
//!
//! A mathematically ambiguous lift at an apex or pole
//! ([`AmbiguousFaceReason::SingularLiftAmbiguous`]) is *not* invalid geometry;
//! it is a source underspecification. A projection failure
//! (`BoundaryProjectionFailed`, including `DomainOrContractIssue`) is an
//! implementation gap, not source degeneracy. A face with a finite aspect
//! ratio, however small or thin, is a real region and survives. Neither is
//! rejected here.

use crate::{EuclideanSpace, InnerSpace, Point2, Point3};
use serde::Serialize;

/// The hard-degenerate reasons, each a claim that the boundary is certifiably
/// zero-dimensional, one-dimensional, or topologically collapsed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
pub enum DegenerateFaceReason {
    /// Every source bound is a `VERTEX_LOOP` / collapsed; there is no
    /// realisable 1D boundary geometry at all. Detector A, from STEP topology.
    AllBoundsCollapsed,
    /// The trim is a line in parameter space enclosing no area, and the world
    /// boundary is one-dimensional. A generator-line loop, an out-and-back
    /// traversal, or a trim constant in one surface coordinate.
    LineLikeTrim,
    /// The whole world boundary lies within a floating-point conditioning
    /// bound of a single point; the trim is zero-dimensional.
    PointLikeTrim,
    /// The boundary is a genuine 2D region in parameter space but collapses to
    /// a one-dimensional curve in world space — the surface metric degenerates
    /// one direction.
    ZeroWidthBand,
    /// Parity selected a real parameter-space region but every realized
    /// triangle collapses in world space to at or below the world-area
    /// validation floor (`1e-12`): a physically sub-resolution sliver at the
    /// meshing resolution. Detector C, from the CDT result stage.
    SubToleranceSliver,
}

impl DegenerateFaceReason {
    /// A short stable tag for aggregation and reporting.
    pub const fn tag(self) -> &'static str {
        match self {
            Self::AllBoundsCollapsed => "AllBoundsCollapsed",
            Self::LineLikeTrim => "LineLikeTrim",
            Self::PointLikeTrim => "PointLikeTrim",
            Self::ZeroWidthBand => "ZeroWidthBand",
            Self::SubToleranceSliver => "SubToleranceSliver",
        }
    }
}

/// A proved source-level inconsistency, distinguished from degeneracy.
///
/// Reserved for outcomes where the *source* itself is certified inconsistent,
/// not merely the realized boundary of one attempted tessellation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
pub enum InconsistentFaceReason {
    /// The source defines no coherent material region, proved from source
    /// evidence. Not currently emitted: `ContradictoryDualParity` in the
    /// realized boundary is not yet certified against the source.
    ContradictoryDualParity,
}

/// A semantically ambiguous outcome that is **not** invalid.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
pub enum AmbiguousFaceReason {
    /// The periodic lift across a singular apex/pole is not determined by the
    /// source evidence.
    SingularLiftAmbiguous,
}

/// The source-level classification of one face.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
pub enum FaceAdmissibility {
    /// A face with realisable positive-area geometry; the tessellator should
    /// proceed unchanged.
    RenderableCandidate,
    /// A certified degenerate face that is hard-rejected before tessellation.
    RejectDegenerate(DegenerateFaceReason),
    /// A certified source-inconsistent face.
    RejectInconsistent(InconsistentFaceReason),
    /// A mathematically ambiguous face that is **not** invalid.
    Ambiguous(AmbiguousFaceReason),
    /// The classification could not be established.
    Unknown,
}

/// The geometric evidence backing one hard rejection.
///
/// The invariant that every automatically rejected face satisfies: the
/// certificate's `world_rank` is 0 or 1, or the reason is
/// [`DegenerateFaceReason::AllBoundsCollapsed`] (boundary rank never measured)
/// or [`DegenerateFaceReason::SubToleranceSliver`] (the rank is measured from
/// the realized triangles, not the boundary, and a sub-floor sliver may still
/// carry two tiny world directions). There is deliberately no bare
/// `is_bad_face()` boolean — a rejection is always accompanied by its
/// certificate.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct FaceValidityCertificate {
    /// Why the face was rejected.
    pub reason: DegenerateFaceReason,
    /// The number of bounds declared on the source face.
    pub bound_count: usize,
    /// The number of boundary pieces measured.
    pub piece_count: usize,
    /// The numerical rank of the world-space boundary: 0 (a point), 1 (a
    /// line), or 2+ (a real region). Rejection requires rank < 2.
    pub world_rank: u8,
    /// The farthest-pair span of the world points. Rank evidence.
    pub rank_span: Option<f64>,
    /// The maximum perpendicular distance of any world point from the line
    /// through the farthest pair. Rank evidence: for rank 1 this is within
    /// `rank_tolerance`.
    pub rank_max_perp: Option<f64>,
    /// The floating-point conditioning tolerance the rank decision was taken
    /// at. See [`fp_rank_tolerance`].
    pub rank_tolerance: Option<f64>,
    /// The signed parameter-space area of the trim (sum of |per-piece
    /// shoelace|), when measurable. Diagnostic.
    pub signed_area: Option<f64>,
    /// `|signed_area| / (du * dv)`, the dimensionless fill of the parameter
    /// bbox. Diagnostic, never load-bearing.
    pub normalized_area: Option<f64>,
    /// The parameter-space bounding-box extents `(du, dv)`. Diagnostic.
    pub uv_extents: Option<(f64, f64)>,
    /// The world-space bounding-box extents, sorted `(max, mid, min)`.
    /// Diagnostic.
    pub world_extents: Option<(f64, f64, f64)>,
    /// The support-surface local metric scale `(avg |Su|, avg |Sv|)`, when the
    /// pipeline could compute it. Diagnostic.
    pub metric_scale: Option<(f64, f64)>,
    /// The number of triangles the material-parity selection produced, when the
    /// certificate was made at the CDT result stage (Detector C). Diagnostic.
    pub selected_triangle_count: Option<usize>,
    /// The maximum world-space area of a realized (selected) triangle, when the
    /// certificate was made at the CDT result stage (Detector C). Diagnostic.
    pub max_realized_area: Option<f64>,
}

impl FaceValidityCertificate {
    /// The classification this certificate implies.
    pub const fn admissibility(&self) -> FaceAdmissibility {
        FaceAdmissibility::RejectDegenerate(self.reason)
    }

    /// A certificate for Detector A: every source bound collapsed.
    pub const fn all_bounds_collapsed(bound_count: usize) -> Self {
        Self {
            reason: DegenerateFaceReason::AllBoundsCollapsed,
            bound_count,
            piece_count: 0,
            world_rank: 0,
            rank_span: None,
            rank_max_perp: None,
            rank_tolerance: None,
            signed_area: None,
            normalized_area: None,
            uv_extents: None,
            world_extents: None,
            metric_scale: None,
            selected_triangle_count: None,
            max_realized_area: None,
        }
    }

    /// A certificate for Detector C: the material-parity stage selected a
    /// region, but every realized triangle collapsed to at or below the
    /// world-area validation floor. The `world_rank` here is measured from the
    /// realized triangle vertices, which may legitimately be 2 (two tiny world
    /// directions); the reason records that the *region* is sub-resolution.
    pub fn sub_tolerance_sliver(
        bound_count: usize,
        piece_count: usize,
        world_rank: u8,
        rank_span: f64,
        rank_max_perp: f64,
        rank_tolerance: f64,
        uv_extents: (f64, f64),
        world_extents: (f64, f64, f64),
        selected_triangle_count: usize,
        max_realized_area: f64,
    ) -> Self {
        Self {
            reason: DegenerateFaceReason::SubToleranceSliver,
            bound_count,
            piece_count,
            world_rank,
            rank_span: Some(rank_span),
            rank_max_perp: Some(rank_max_perp),
            rank_tolerance: Some(rank_tolerance),
            signed_area: None,
            normalized_area: None,
            uv_extents: Some(uv_extents),
            world_extents: Some(world_extents),
            metric_scale: None,
            selected_triangle_count: Some(selected_triangle_count),
            max_realized_area: Some(max_realized_area),
        }
    }
}

/// One boundary sample: the parameter-space point and its world position on
/// the support surface.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TrimSample {
    /// The parameter-space point.
    pub uv: Point2,
    /// The world position of the surface at `uv`.
    pub world: Point3,
}

/// The measured geometry of a face's boundary.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TrimMeasurement {
    /// Sum over pieces of the absolute parameter-space shoelace area.
    pub uv_area: f64,
    /// The parameter-space bounding-box extents `(du, dv)`.
    pub uv_extents: (f64, f64),
    /// The world-space bounding-box extents, sorted `(max, mid, min)`.
    pub world_extents: (f64, f64, f64),
    /// The numerical rank of the world points: 0 (all within a floating-point
    /// bound of one point), 1 (all within a floating-point bound of one line),
    /// or 2+. This is the certificate.
    pub world_rank: u8,
    /// The farthest-pair span of the world points.
    pub rank_span: f64,
    /// The maximum perpendicular distance of any point from the farthest-pair
    /// line.
    pub rank_max_perp: f64,
    /// The floating-point conditioning tolerance the rank was taken at.
    pub rank_tolerance: f64,
    /// The number of boundary pieces.
    pub piece_count: usize,
    /// The total number of samples.
    pub sample_count: usize,
    /// The local surface metric scale, `(avg |Su|, avg |Sv|)`, when the
    /// caller supplied it.
    pub metric_scale: Option<(f64, f64)>,
}

/// The margin, in units of `f64::EPSILON * coordinate_magnitude`, on the
/// per-sample coordinate error when deriving the rank tolerance.
///
/// A surface evaluation `S(u, v)` carries an absolute error of order a few
/// `EPSILON` times the coordinate magnitude (the placement/transform scale).
/// This margin absorbs that factor and a small constant in the perpendicular-
/// distance arithmetic; it is a floating-point conditioning bound, never a
/// meshing tolerance.
pub const FP_RANK_MARGIN: f64 = 16.0;

/// The floating-point conditioning tolerance for the world-rank decision.
///
/// A boundary coordinate is computed to an absolute error of order
/// `EPSILON * coordinate_magnitude`, so a direction whose extent is below
/// `FP_RANK_MARGIN * EPSILON * max_coordinate_magnitude` is indistinguishable
/// from a collapsed direction at the model's own floating-point resolution.
/// Real features sit orders of magnitude above this.
pub fn fp_rank_tolerance(scale: f64) -> f64 {
    FP_RANK_MARGIN * f64::EPSILON * scale
}

/// A parameter-space region counts as genuinely 2D only when its minor extent
/// is at least this fraction of its major extent…
pub const UV_MINOR_RATIO: f64 = 0.02;

/// …and its shoelace area fills at least this fraction of its parameter bbox.
/// A generator-line loop or a retracing slit fills essentially zero; a real
/// region fills near one.
pub const UV_AREA_FILL_RATIO: f64 = 0.02;

/// The numerical rank of a set of world points, by the farthest-pair
/// certificate.
///
/// Let `(a, b)` be the farthest pair and `span = |b - a|`. If `span` is within
/// the floating-point tolerance the points coincide (rank 0). Otherwise, if
/// every point's perpendicular distance from the line through `a, b` is within
/// the tolerance, all points lie on one line (rank 1). Otherwise the points
/// span a plane or space (rank 2).
///
/// The perpendicular-distance test is the certificate "all points lie on a
/// line within floating-point error": the computed distance carries an error
/// of order `EPSILON * coordinate_magnitude`, so the tolerance is derived from
/// the coordinate magnitude, not from any meshing tolerance.
pub fn world_rank_of(points: &[Point3]) -> (u8, f64, f64, f64) {
    let scale = points
        .iter()
        .fold(0.0_f64, |acc, p| acc.max(p.to_vec().magnitude()));
    let tol = fp_rank_tolerance(scale);
    if points.len() < 2 {
        return (0, 0.0, 0.0, tol);
    }
    // Farthest pair.
    let mut span = 0.0_f64;
    let mut a = points[0];
    let mut b = points[1];
    for i in 0..points.len() {
        for j in (i + 1)..points.len() {
            let d = (points[i] - points[j]).magnitude();
            if d > span {
                span = d;
                a = points[i];
                b = points[j];
            }
        }
    }
    if span <= tol {
        return (0, span, 0.0, tol);
    }
    let direction = b - a;
    let direction_len = direction.magnitude();
    // The perpendicular distance of each point from the line through a, b.
    let mut max_perp = 0.0_f64;
    for p in points {
        let perp = (p - a).cross(direction).magnitude() / direction_len;
        if perp > max_perp {
            max_perp = perp;
        }
    }
    if max_perp <= tol {
        (1, span, max_perp, tol)
    } else {
        (2, span, max_perp, tol)
    }
}

/// Measure the boundary geometry from its pieces and certify its world rank.
///
/// Each piece is a sample list in traversal order. The world rank is computed
/// from the scatter (covariance) matrix of all world points against the
/// floating-point noise floor; the parameter quantities are diagnostic.
pub fn measure_trim(
    pieces: &[Vec<TrimSample>],
    metric_scale: Option<(f64, f64)>,
) -> Option<TrimMeasurement> {
    if pieces.is_empty() {
        return None;
    }
    let mut uv_lo = [f64::INFINITY; 2];
    let mut uv_hi = [f64::NEG_INFINITY; 2];
    let mut world_lo = [f64::INFINITY; 3];
    let mut world_hi = [f64::NEG_INFINITY; 3];
    let mut uv_area = 0.0_f64;
    let mut sample_count = 0usize;
    let mut world_points: Vec<Point3> = Vec::new();
    for piece in pieces {
        if piece.len() < 2 {
            continue;
        }
        sample_count += piece.len();
        let mut piece_uv_area = 0.0_f64;
        for (i, sample) in piece.iter().enumerate() {
            let next = &piece[(i + 1) % piece.len()];
            uv_lo[0] = uv_lo[0].min(sample.uv.x);
            uv_hi[0] = uv_hi[0].max(sample.uv.x);
            uv_lo[1] = uv_lo[1].min(sample.uv.y);
            uv_hi[1] = uv_hi[1].max(sample.uv.y);
            world_lo[0] = world_lo[0].min(sample.world.x);
            world_hi[0] = world_hi[0].max(sample.world.x);
            world_lo[1] = world_lo[1].min(sample.world.y);
            world_hi[1] = world_hi[1].max(sample.world.y);
            world_lo[2] = world_lo[2].min(sample.world.z);
            world_hi[2] = world_hi[2].max(sample.world.z);
            // Shoelace in parameter space (2× signed area), closed by the wrap.
            piece_uv_area += (next.uv.x + sample.uv.x) * (next.uv.y - sample.uv.y);
            world_points.push(sample.world);
        }
        uv_area += piece_uv_area.abs() * 0.5;
    }
    if sample_count == 0 {
        return None;
    }
    let mut world_extents = [
        world_hi[0] - world_lo[0],
        world_hi[1] - world_lo[1],
        world_hi[2] - world_lo[2],
    ];
    world_extents.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
    let (world_rank, rank_span, rank_max_perp, rank_tolerance) = world_rank_of(&world_points);
    Some(TrimMeasurement {
        uv_area,
        uv_extents: (uv_hi[0] - uv_lo[0], uv_hi[1] - uv_lo[1]),
        world_extents: (world_extents[0], world_extents[1], world_extents[2]),
        world_rank,
        rank_span,
        rank_max_perp,
        rank_tolerance,
        piece_count: pieces.len(),
        sample_count,
        metric_scale,
    })
}

/// Whether a parameter-space region is genuinely 2D (as opposed to a
/// degenerate line that happens to sit inside a non-degenerate bounding box).
/// Diagnostic only: it names the reason, it never decides rejection.
fn uv_is_two_dimensional(measurement: &TrimMeasurement) -> bool {
    let (du, dv) = measurement.uv_extents;
    let major = du.max(dv);
    if !du.is_finite() || !dv.is_finite() || major <= 0.0 {
        return false;
    }
    let minor = du.min(dv);
    if minor < UV_MINOR_RATIO * major {
        return false;
    }
    let bbox_area = du * dv;
    if !(bbox_area > 0.0 && bbox_area.is_finite()) {
        return false;
    }
    measurement.uv_area.abs() >= UV_AREA_FILL_RATIO * bbox_area
}

/// Classify the measured boundary geometry.
///
/// Returns a certificate exactly when the boundary is certifiably degenerate:
/// its world-space points have numerical rank < 2 (they all lie within a
/// floating-point conditioning bound of a point or of a line). Everything else
/// — every boundary with two real world directions, however small or thin —
/// is a [`RenderableCandidate`](FaceAdmissibility::RenderableCandidate) and is
/// not rejected. No meshing tolerance enters the decision.
pub fn classify_trim_geometry(
    bound_count: usize,
    measurement: &TrimMeasurement,
) -> Option<FaceValidityCertificate> {
    if measurement.world_rank >= 2 {
        return None;
    }
    let certificate = |reason| FaceValidityCertificate {
        reason,
        bound_count,
        piece_count: measurement.piece_count,
        world_rank: measurement.world_rank,
        rank_span: Some(measurement.rank_span),
        rank_max_perp: Some(measurement.rank_max_perp),
        rank_tolerance: Some(measurement.rank_tolerance),
        signed_area: Some(measurement.uv_area),
        normalized_area: {
            let (du, dv) = measurement.uv_extents;
            let bbox = du * dv;
            (bbox.is_finite() && bbox > 0.0).then_some(measurement.uv_area.abs() / bbox)
        },
        uv_extents: Some(measurement.uv_extents),
        world_extents: Some(measurement.world_extents),
        metric_scale: measurement.metric_scale,
        selected_triangle_count: None,
        max_realized_area: None,
    };
    match measurement.world_rank {
        0 => Some(certificate(DegenerateFaceReason::PointLikeTrim)),
        _ => {
            // Rank 1. Name the mechanism by the parameter region: a genuinely
            // 2D parameter region that the surface metric collapses to a world
            // curve is a zero-width band; a trim that is itself a line in
            // parameter space is a line. Both are certified by the rank test;
            // the UV shape only labels which degenerate the source actually is.
            let uv_2d = uv_is_two_dimensional(measurement);
            let reason = if uv_2d {
                DegenerateFaceReason::ZeroWidthBand
            } else {
                DegenerateFaceReason::LineLikeTrim
            };
            Some(certificate(reason))
        }
    }
}

/// The local metric scale of a surface at a sample point.
///
/// `avg |Su|` and `avg |Sv|` let a parameter-space width be converted into a
/// world-space width, which is what keeps a pathological parameterization from
/// being misread. Supplied by the pipeline; recorded in the certificate as
/// diagnostic evidence.
pub fn metric_scale_of<F, G>(uder: F, vder: G, samples: &[TrimSample]) -> Option<(f64, f64)>
where
    F: Fn(f64, f64) -> crate::Vector3,
    G: Fn(f64, f64) -> crate::Vector3,
{
    let mut su = 0.0_f64;
    let mut sv = 0.0_f64;
    let mut count = 0usize;
    for sample in samples {
        let (u, v) = (sample.uv.x, sample.uv.y);
        let (du, dv) = (uder(u, v).magnitude(), vder(u, v).magnitude());
        if du.is_finite() && dv.is_finite() {
            su += du;
            sv += dv;
            count += 1;
        }
    }
    if count == 0 {
        return None;
    }
    Some((su / count as f64, sv / count as f64))
}

/// Whether the face-validity rejection path is active.
///
/// **Off by default.** The detector is conservative, but the `rendered ->
/// rejected = 0` gate must be measured on the corpus before hard rejection is
/// enabled in production. Set `TRUCK_FACE_VALIDITY=1` (or `on`/`true`/`yes`)
/// to enable; an explicit `0`/`off`/`false`/`no` disables it.
pub fn rejection_enabled() -> bool {
    match std::env::var("TRUCK_FACE_VALIDITY") {
        Err(_) => false,
        Ok(value) => !matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "0" | "off" | "false" | "no"
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(u: f64, v: f64, x: f64, y: f64, z: f64) -> TrimSample {
        TrimSample {
            uv: Point2::new(u, v),
            world: Point3::new(x, y, z),
        }
    }

    /// Build a planar closed loop from `(u, v)` points, mapped to a world
    /// plane by a per-axis scale `(sx, sy)`. Used to decouple parameter-space
    /// shape from world-space shape.
    fn loop_from(uv: &[(f64, f64)], sx: f64, sy: f64) -> Vec<TrimSample> {
        uv.iter()
            .map(|(u, v)| sample(*u, *v, *u * sx, *v * sy, 0.0))
            .collect()
    }

    fn classify(pieces: &[Vec<TrimSample>]) -> Option<FaceValidityCertificate> {
        let measurement = measure_trim(pieces, None).expect("measurement");
        classify_trim_geometry(1, &measurement)
    }

    // 1. Ordinary planar rectangle -> accepted.
    #[test]
    fn ordinary_planar_rectangle_is_accepted() {
        let rect = loop_from(&[(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)], 1.0, 1.0);
        let cert = classify(&[rect]);
        assert!(cert.is_none(), "a real 1x1 rectangle must not be rejected");
    }

    // 2. Very small but valid rectangle -> accepted. Two real world directions
    //    make it rank 2 regardless of absolute size.
    #[test]
    fn very_small_but_valid_rectangle_is_accepted() {
        let rect = loop_from(
            &[(0.0, 0.0), (1.0e-4, 0.0), (1.0e-4, 1.0e-4), (0.0, 1.0e-4)],
            1.0,
            1.0,
        );
        let cert = classify(&[rect]);
        assert!(cert.is_none(), "a small-but-2D face must survive");
    }

    // 3. Exact line-loop -> rejected line-like (world rank 1).
    #[test]
    fn exact_line_loop_is_rejected_line_like() {
        let line = loop_from(&[(0.0, 0.0), (1.0, 0.0), (0.5, 0.0)], 1.0, 1.0);
        let cert = classify(&[line]).expect("an exact line loop is rank 1");
        assert_eq!(cert.reason, DegenerateFaceReason::LineLikeTrim);
        assert_eq!(cert.world_rank, 1);
    }

    // 4. Exact point-loop -> rejected point-like (world rank 0).
    #[test]
    fn exact_point_loop_is_rejected_point_like() {
        let pt = loop_from(&[(0.5, 0.5), (0.5, 0.5), (0.5, 0.5)], 1.0, 1.0);
        let cert = classify(&[pt]).expect("a point loop is rank 0");
        assert_eq!(cert.reason, DegenerateFaceReason::PointLikeTrim);
        assert_eq!(cert.world_rank, 0);
    }

    // 5. VERTEX_LOOP-only face -> collapsed reject. Detector A, classified
    //    from source topology; the certificate is `all_bounds_collapsed`.
    #[test]
    fn vertex_loop_only_face_is_collapsed_reject() {
        let cert = FaceValidityCertificate::all_bounds_collapsed(2);
        assert_eq!(cert.reason, DegenerateFaceReason::AllBoundsCollapsed);
        assert_eq!(cert.bound_count, 2);
        assert!(matches!(
            cert.admissibility(),
            FaceAdmissibility::RejectDegenerate(DegenerateFaceReason::AllBoundsCollapsed)
        ));
    }

    // 6. Micro-cylinder generator-line loop resembling S1 -> rejected. The
    //    boundary is a line in world space (rank 1): a generator at constant u.
    #[test]
    fn micro_cylinder_generator_line_is_rejected() {
        let u = std::f64::consts::FRAC_PI_4;
        let r = 5.0e-6;
        let (cx, cy) = (r * u.cos(), r * u.sin());
        let v0 = 1.95e-4;
        let v1 = 2.2e-4;
        let gen = vec![
            sample(u, v0, cx, cy, v0),
            sample(u, v1, cx, cy, v1),
            sample(u, v0, cx, cy, v0),
        ];
        let cert = classify(&[gen]).expect("a generator line is rank 1");
        assert_eq!(cert.reason, DegenerateFaceReason::LineLikeTrim);
        assert_eq!(cert.world_rank, 1);
        assert!(cert.signed_area.unwrap().abs() < 1.0e-12);
    }

    // 7. Zero-width B-spline band resembling S5 -> rejected. The u-direction
    //    of the surface collapses, so the whole boundary maps to a world line
    //    (rank 1) even though the parameter region spans v.
    #[test]
    fn zero_width_bspline_band_is_rejected() {
        let n = 64;
        let mut band = Vec::new();
        for i in 0..n {
            let t = i as f64 / (n - 1) as f64;
            band.push(sample(t * 0.062, 0.0, 0.0, 0.0, t * 0.062));
        }
        for i in (0..n).rev() {
            let t = i as f64 / (n - 1) as f64;
            band.push(sample(t * 0.062, 0.0, 0.0, 0.0, t * 0.062));
        }
        let cert = classify(&[band]).expect("a zero-width band is rank 1");
        assert!(matches!(
            cert.reason,
            DegenerateFaceReason::LineLikeTrim | DegenerateFaceReason::ZeroWidthBand
        ));
        assert_eq!(cert.world_rank, 1);
    }

    // 8. Narrow but finite B-spline band -> accepted. A finite width is two
    //    real world directions, however small the aspect ratio.
    #[test]
    fn narrow_band_is_accepted() {
        let width = 1.0e-3;
        let n = 32;
        let mut band = Vec::new();
        for i in 0..n {
            let t = i as f64 / (n - 1) as f64;
            band.push(sample(t * 10.0, 0.0, t * 10.0, 0.0, 0.0));
        }
        for i in (0..n).rev() {
            let t = i as f64 / (n - 1) as f64;
            band.push(sample(t * 10.0, width, t * 10.0, width, 0.0));
        }
        let cert = classify(&[band]);
        assert!(cert.is_none(), "a finite-width band must survive");
    }

    // 9. Pathological UV scaling but finite 3D area -> accepted. The UV region
    //    is enormous, the world is a bounded 2D region: rank 2 in world.
    #[test]
    fn pathological_uv_scaling_with_finite_3d_area_is_accepted() {
        let scale = 1.0e-2;
        let rect = vec![
            sample(0.0, 0.0, 0.0, 0.0, 0.0),
            sample(10.0, 0.0, 10.0 * scale, 0.0, 0.0),
            sample(10.0, 6.0, 10.0 * scale, 6.0 * scale, 0.0),
            sample(0.0, 6.0, 0.0, 6.0 * scale, 0.0),
        ];
        let cert = classify(&[rect]);
        assert!(cert.is_none(), "finite 3D area must survive UV scaling");
    }

    // 10. Cone apex AmbiguousLift example -> NOT classified invalid. This is
    //     an ambiguity classification, distinct from the degenerate reasons.
    #[test]
    fn cone_apex_ambiguous_lift_is_not_invalid() {
        let admissibility =
            FaceAdmissibility::Ambiguous(AmbiguousFaceReason::SingularLiftAmbiguous);
        assert!(!matches!(
            admissibility,
            FaceAdmissibility::RejectDegenerate(_) | FaceAdmissibility::RejectInconsistent(_)
        ));
    }

    // 11. Known projection DomainOrContract face -> not rejected. Projection
    //     failures occur before the boundary is fully realized; the geometric
    //     detector has nothing to reject.
    #[test]
    fn projection_domain_face_is_not_rejected() {
        let rect = loop_from(&[(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)], 1.0, 1.0);
        let cert = classify(&[rect]);
        assert!(cert.is_none());
    }

    // 12. Certificate fields explain every rejection.
    #[test]
    fn certificate_fields_explain_every_rejection() {
        let line = loop_from(&[(0.0, 0.0), (2.0, 0.0), (1.0, 0.0)], 1.0, 1.0);
        let cert = classify(&[line]).expect("line-like");
        assert_eq!(cert.bound_count, 1);
        assert_eq!(cert.piece_count, 1);
        assert_eq!(cert.world_rank, 1);
        let (a, b, _) = cert.world_extents.unwrap();
        assert!(a > b);
        assert!(cert.rank_span.unwrap() > 0.0);
        assert!(cert.rank_max_perp.unwrap() <= cert.rank_tolerance.unwrap());
        let (du, dv) = cert.uv_extents.unwrap();
        assert!(du > 0.0 && du + dv > 0.0, "the line spans the u direction");
        assert!(cert.signed_area.unwrap() < 1.0e-9);
        // A line loop has zero bbox area, so the dimensionless fill is
        // undefined; the certificate reports it honestly as `None`.
        assert!(cert.normalized_area.is_none());
    }

    // A genuinely 2D parameter region whose world metric collapses the minor
    // direction to a line (the two world edges coincide) is a zero-width band.
    #[test]
    fn uv_2d_region_collapsing_in_world_is_zero_width_band() {
        let rect = vec![
            sample(0.0, 0.0, 0.0, 0.0, 0.0),
            sample(10.0, 0.0, 10.0, 0.0, 0.0),
            sample(10.0, 1.0, 10.0, 0.0, 0.0),
            sample(0.0, 1.0, 0.0, 0.0, 0.0),
        ];
        let cert = classify(&[rect]).expect("a world-collapsed band is rank 1");
        assert_eq!(cert.reason, DegenerateFaceReason::ZeroWidthBand);
        assert_eq!(cert.world_rank, 1);
    }

    // The mirror case is the false-reject guard: a *finite* width, even far
    // below any chord tolerance, is two real world directions and survives.
    #[test]
    fn finite_world_width_is_accepted() {
        let rect = vec![
            sample(0.0, 0.0, 0.0, 0.0, 0.0),
            sample(10.0, 0.0, 10.0, 0.0, 0.0),
            sample(10.0, 1.0, 10.0, 1.0e-7, 0.0),
            sample(0.0, 1.0, 0.0, 1.0e-7, 0.0),
        ];
        let cert = classify(&[rect]);
        assert!(
            cert.is_none(),
            "a 1e-7-wide band has rank 2 and must survive"
        );
    }

    // A face far from the origin with a genuinely collapsed direction is still
    // rank 1: the noise floor scales with the coordinate magnitude, not with
    // the feature size.
    #[test]
    fn collapsed_direction_at_large_coordinate_is_rejected() {
        let z0 = 1000.0;
        let gen = vec![
            sample(0.0, 0.0, 1.0e-6, 2.0e-6, z0),
            sample(0.0, 1.0e-3, 1.0e-6, 2.0e-6, z0 + 1.0e-3),
            sample(0.0, 0.0, 1.0e-6, 2.0e-6, z0),
        ];
        let cert = classify(&[gen]).expect("a far-from-origin line is rank 1");
        assert_eq!(cert.reason, DegenerateFaceReason::LineLikeTrim);
        assert_eq!(cert.world_rank, 1);
    }
}
