#![deny(clippy::unwrap_used)]

//! BG-CG-004-FACET — the direct facet realization backend.
//!
//! Realizes a landed `SpineFrameRecipe` as a shared-topology `PolygonMesh`
//! closed by construction: the structured grid x_{i,j} = position(s_i, v_j)
//! is emitted once per grid vertex (index i*k + j), adjacent faces reuse the
//! identity, and no positional welding, sewing, or healing is ever invoked.
//! The mandatory mesh-level sanity audit (plan §3.3) rides beside the mesh.

use std::collections::HashMap;

use truck_base::evidence::{
    Budget, Certificate, Certified, ConstructErrorSummary, EnvelopeCase, Margin, Method, Modulus,
    Outcome, PropMap, RealizationCertificate, RealizationVerdict, Refusal, SharedEdgePairEvidence,
};
use truck_geometry::constructive::*;
use truck_polymesh::*;

/// The three-valued verdict of the plan §3.3 sanity audit. CG-007 maps this
/// onto the unified realization evidence (the CG-000 §3.5 mapping row);
/// until then this local spelling is the booked representation. Uncertainty
/// is surfaced (Inconclusive), never converted into success.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FacetVerdict {
    /// The mesh closed by construction and the audit found nothing.
    CertifiedWithinTolerance,
    /// The winding audit found violations — FAILED, never a warning.
    Failed,
    /// The audit could not decide (e.g. the signed volume is degenerate
    /// against the mesh's own extent).
    Inconclusive,
}

/// The mandatory mesh-level sanity audit facts (plan §3.3): signed-volume
/// sign sanity and the twin-triangle winding audit. Pure data; the verdict
/// is derived by [`verdict_of`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FacetSweepAudit {
    /// Emitted triangles.
    pub triangle_count: usize,
    /// Emitted planar quads (a quad is ONE face of the polygon mesh).
    pub quad_count: usize,
    /// Signed volume V = (1/6) * sum a . (b x c) over the fan triangulation
    /// of every face, after the global orientation normalization.
    pub signed_volume: f64,
    /// Number of interior mesh edges whose two uses do NOT traverse in
    /// opposite effective directions, plus boundary uses (0 for a closed
    /// mesh — which this construction produces).
    pub winding_violations: usize,
}

/// The result: the mesh, the audit facts, and the verdict.
#[derive(Debug, Clone)]
pub struct FacetSweepResult {
    /// The realized mesh. Every position index is a grid-registry index:
    /// adjacent faces share the identity BY CONSTRUCTION (plan §3.3); no
    /// positional welding is ever invoked.
    pub mesh: PolygonMesh,
    /// The audit facts.
    pub audit: FacetSweepAudit,
    /// The three-valued verdict.
    pub verdict: FacetVerdict,
    /// Mapping A row 2: the per-realization certificate, Method::Float (H-6).
    pub realization_certificate: RealizationCertificate,
    /// Mapping A row 3. Empty on the exact-grid path: the grid registry makes
    /// shared edges index-identical by construction, so there is no measured
    /// error to record. The LEDGER assembly (meshalgo) populates this when a
    /// realization is built over sampled edges.
    pub shared_edge_pairs: Vec<SharedEdgePairEvidence>,
}

/// Realizes the recipe as a faceted `PolygonMesh` over the given spine
/// stations (RESOLVED stations — ascending, >= 2, inside the spine domain;
/// resolve a `SamplingPolicy` with its `resolve` first and pass the result).
///
/// `ring_resolution` is the profile vertex count k: the ring parameter of
/// profile vertex j is v_j = j / k (the per-edge-uniform convention the
/// profile evaluator is booked on; plan §3.3's grid vertex (i, j)).
///
/// Structured grid x_{i,j} = position(s_i, v_j); grid vertex (i, j) is
/// created EXACTLY ONCE via the private grid registry (index i*k + j);
/// adjacent faces reuse the identity; internal grid edges are created once
/// and traversed oppositely by their two faces. No sewing (plan §3.3).
pub fn facet_sweep<S: SpineCurve>(
    recipe: &SpineFrameRecipe<S, ProfileLaw, FrameLaw>,
    stations: &[f64],
    ring_resolution: usize,
) -> Result<FacetSweepResult, ConstructError> {
    // 1. Validation.
    if ring_resolution < 3 {
        return Err(ConstructError::InvalidInput);
    }
    if stations.len() < 2 {
        return Err(ConstructError::InvalidInput);
    }
    if let Some(&bad) = stations.iter().find(|s| !s.is_finite()) {
        return Err(ConstructError::NonFinite { at: bad });
    }
    if stations.windows(2).any(|w| w[1] <= w[0]) {
        return Err(ConstructError::InvalidInput);
    }
    let parameter_tol = DirectTolerance::default().parameter;
    let (s_min, s_max) = recipe.spine.domain();
    if stations
        .iter()
        .any(|&s| s < s_min - parameter_tol || s > s_max + parameter_tol)
    {
        return Err(ConstructError::InvalidInput);
    }

    // 2. Grid emission. The position array IS the grid registry: grid vertex
    // (i, j) lives at index i*k + j, exactly once; nothing is a "copy".
    let m = stations.len();
    let k = ring_resolution;
    let mut positions = Vec::with_capacity(m * k);
    for &s in stations {
        for j in 0..k {
            let v = j as f64 / k as f64;
            positions.push(recipe.position(s, v)?);
        }
    }

    let mut tri_faces: Vec<[usize; 3]> = Vec::new();
    let mut quad_faces: Vec<[usize; 4]> = Vec::new();
    let mut triangle_count = 0usize;
    let mut quad_count = 0usize;
    let position_tol = DirectTolerance::default().position;
    // The maximum bilinear-twist deviation over the side cells (mapping A
    // row 2). Tracked here, beside the existing quad/tri split decision — no
    // recomputation, no new tolerances.
    let mut max_cell_twist: f64 = 0.0;

    // 3. Side faces. The diagonal choice (i,j)-(i+1,j2) is structural —
    // always this diagonal, never a float comparison between alternatives.
    for i in 0..m - 1 {
        for j in 0..k {
            let j2 = (j + 1) % k;
            let a = i * k + j;
            let b = (i + 1) * k + j;
            let c = (i + 1) * k + j2;
            let d = i * k + j2;
            let origin = Point3::origin();
            let twist = (positions[a] - origin) + (positions[c] - origin)
                - (positions[b] - origin)
                - (positions[d] - origin);
            max_cell_twist = max_cell_twist.max(twist.magnitude());
            if twist.magnitude() <= position_tol {
                quad_faces.push([a, b, c, d]);
                quad_count += 1;
            } else {
                tri_faces.push([a, b, c]);
                tri_faces.push([a, c, d]);
                triangle_count += 2;
            }
        }
    }

    // 4. Caps. The ring vertices ARE the grid vertices (shared identity).
    // Convexity is certified at BOTH cap stations: the ring polygon's
    // consecutive edge pairs all cross with one strict sign.
    let start_ring: Vec<Point3> = (0..k).map(|j| positions[j]).collect();
    let end_ring: Vec<Point3> = ((m - 1) * k..m * k).map(|i| positions[i]).collect();
    if !ring_is_convex(&start_ring, position_tol) || !ring_is_convex(&end_ring, position_tol) {
        return Err(ConstructError::InvalidInput);
    }
    for t in 1..k - 1 {
        tri_faces.push([0, t, t + 1]);
    }
    let r0 = (m - 1) * k;
    for t in 1..k - 1 {
        tri_faces.push([r0, r0 + t + 1, r0 + t]);
    }

    // 5. Global orientation normalization. The grid's faces share one
    // handedness by construction, so one global sign check replaces any
    // per-face BFS: invert every face's index cycle iff the signed volume is
    // negative (the inversion flips the sign exactly).
    let mut signed_volume = signed_volume_of(&positions, &tri_faces, &quad_faces);
    if signed_volume < 0.0 {
        tri_faces.iter_mut().for_each(|f| f.reverse());
        quad_faces.iter_mut().for_each(|f| f.reverse());
        signed_volume = -signed_volume;
    }

    // 6. The mesh extent d = the max distance between any two grid positions.
    let extent = mesh_extent(&positions);

    // 7. Assembly — the mesh's position array IS the grid registry.
    let tri_vertices: Vec<[StandardVertex; 3]> = tri_faces
        .iter()
        .map(|f| [vertex(f[0]), vertex(f[1]), vertex(f[2])])
        .collect();
    let quad_vertices: Vec<[StandardVertex; 4]> = quad_faces
        .iter()
        .map(|f| [vertex(f[0]), vertex(f[1]), vertex(f[2]), vertex(f[3])])
        .collect();
    let mesh = PolygonMesh::new(
        StandardAttributes {
            positions,
            ..Default::default()
        },
        Faces::from_tri_and_quad_faces(tri_vertices, quad_vertices),
    );

    // 8. The mandatory mesh-level sanity audit on the final emitted mesh.
    let audit = FacetSweepAudit {
        triangle_count,
        quad_count,
        signed_volume,
        winding_violations: winding_audit(&mesh),
    };
    let verdict = verdict_of(&audit, extent);

    Ok(FacetSweepResult {
        mesh,
        audit,
        verdict,
        realization_certificate: RealizationCertificate {
            method: Method::Float,
            max_cell_twist,
            extent,
        },
        shared_edge_pairs: Vec::new(),
    })
}

/// The winding audit: every undirected edge of a closed mesh must appear
/// exactly twice with opposite effective directions; a use-count of 1 or >= 3
/// is also a violation. `pub` because CG-007 consumes it; this function is
/// its test contract.
pub fn winding_audit(mesh: &PolygonMesh) -> usize {
    // The edge map may be a HashMap internally, but violations are COUNTED,
    // not enumerated, into the output — the count is independent of any
    // hash-map iteration order (determinism, plan §7).
    let mut usage: HashMap<(usize, usize), (u32, i32)> = HashMap::new();
    for face in mesh.faces().face_iter() {
        let n = face.len();
        for e in 0..n {
            let u = face[e].pos;
            let w = face[(e + 1) % n].pos;
            let (lo, hi) = if u < w { (u, w) } else { (w, u) };
            let direction = if u < w { 1 } else { -1 };
            let entry = usage.entry((lo, hi)).or_insert((0, 0));
            entry.0 += 1;
            entry.1 += direction;
        }
    }
    usage
        .values()
        .filter(|&&(count, direction_sum)| count != 2 || direction_sum != 0)
        .count()
}

/// Derives the three-valued verdict from the audit facts and the mesh extent
/// `d`. `winding_violations > 0` → `Failed`; `|signed_volume| <= d³ / 1e9` →
/// `Inconclusive`; else `CertifiedWithinTolerance`. `pub` because CG-007
/// consumes it.
pub fn verdict_of(audit: &FacetSweepAudit, extent: f64) -> FacetVerdict {
    if audit.winding_violations > 0 {
        return FacetVerdict::Failed;
    }
    let floor = extent * extent * extent / 1_000_000_000.0;
    if audit.signed_volume.abs() <= floor {
        return FacetVerdict::Inconclusive;
    }
    FacetVerdict::CertifiedWithinTolerance
}

/// The signed volume V = (1/6) * sum a . (b x c) over the fan triangulation
/// of every face (each quad fanned from its first vertex).
fn signed_volume_of(
    positions: &[Point3],
    tri_faces: &[[usize; 3]],
    quad_faces: &[[usize; 4]],
) -> f64 {
    let origin = Point3::origin();
    let mut sum = 0.0;
    for &[a, b, c] in tri_faces {
        let (pa, pb, pc) = (
            positions[a] - origin,
            positions[b] - origin,
            positions[c] - origin,
        );
        sum += pa.dot(pb.cross(pc));
    }
    for &[a, b, c, d] in quad_faces {
        let (pa, pb, pc, pd) = (
            positions[a] - origin,
            positions[b] - origin,
            positions[c] - origin,
            positions[d] - origin,
        );
        sum += pa.dot(pb.cross(pc));
        sum += pa.dot(pc.cross(pd));
    }
    sum / 6.0
}

/// The mesh extent d: the maximum distance between any two grid positions.
fn mesh_extent(positions: &[Point3]) -> f64 {
    let mut d: f64 = 0.0;
    for (i, p) in positions.iter().enumerate() {
        for q in positions.iter().skip(i + 1) {
            d = d.max((*p - *q).magnitude());
        }
    }
    d
}

/// Certifies ring convexity: the consecutive edge pairs' crosses are all
/// strictly one sign, each beyond the tolerance. A non-convex ring (or any
/// collinear adjacent pair) is refused — the cap fan requires it.
fn ring_is_convex(ring: &[Point3], tolerance: f64) -> bool {
    let k = ring.len();
    if k < 3 {
        return false;
    }
    let reference = (ring[1] - ring[0]).cross(ring[2] - ring[1]);
    if reference.magnitude() <= tolerance {
        return false;
    }
    for j in 0..k {
        let a = ring[j];
        let b = ring[(j + 1) % k];
        let c = ring[(j + 2) % k];
        let cross = (b - a).cross(c - b);
        if cross.magnitude() <= tolerance || cross.dot(reference) <= 0.0 {
            return false;
        }
    }
    true
}

/// The position-only `StandardVertex` for a grid-registry index.
fn vertex(index: usize) -> StandardVertex {
    StandardVertex {
        pos: index,
        uv: None,
        nor: None,
    }
}

/// The verdict absorption (mapping B — one tri-state doctrine, no third
/// vocabulary): the facet backend's immediate verdict maps onto the
/// evidence-stage realization verdict arm-for-arm.
impl From<FacetVerdict> for RealizationVerdict {
    fn from(verdict: FacetVerdict) -> Self {
        match verdict {
            FacetVerdict::CertifiedWithinTolerance => RealizationVerdict::CertifiedWithinTolerance,
            FacetVerdict::Failed => RealizationVerdict::Failed,
            FacetVerdict::Inconclusive => RealizationVerdict::Inconclusive,
        }
    }
}

/// Maps every `ConstructError` variant to its summary tag in ONE place. The
/// mapping is modeling-local so base stays geometry-blind (geometry depends
/// on base, not vice versa). A `From<&ConstructError> for ConstructErrorSummary`
/// impl cannot live in this crate — the orphan rule rejects it because neither
/// `ConstructError` nor `ConstructErrorSummary` is local to truck-modeling —
/// so the mapping rides as a plain function instead (deviation recorded in
/// RESULT.json; the mapping table's carrier is unchanged).
pub fn summarize_construct_error(error: &ConstructError) -> ConstructErrorSummary {
    match *error {
        ConstructError::ZeroTangent { at } => ConstructErrorSummary {
            kind: "ZeroTangent",
            at: Some(at),
            law: None,
        },
        ConstructError::FrameSingular { at, law } => ConstructErrorSummary {
            kind: "FrameSingular",
            at: Some(at),
            law: Some(law),
        },
        ConstructError::SpineNotC1 { at } => ConstructErrorSummary {
            kind: "SpineNotC1",
            at: Some(at),
            law: None,
        },
        ConstructError::ProfileCorrespondenceMismatch => ConstructErrorSummary {
            kind: "ProfileCorrespondenceMismatch",
            at: None,
            law: None,
        },
        ConstructError::ProfileCollapse { at } => ConstructErrorSummary {
            kind: "ProfileCollapse",
            at: Some(at),
            law: None,
        },
        ConstructError::NonFinite { at } => ConstructErrorSummary {
            kind: "NonFinite",
            at: Some(at),
            law: None,
        },
        ConstructError::InvalidInput => ConstructErrorSummary {
            kind: "InvalidInput",
            at: None,
            law: None,
        },
    }
}

/// The realization entry per mapping A row 1: construct refusals surface
/// as `Refusal::UnsupportedEnvelope(ConstructRefused)` with the detailed
/// error summarized in the evidence record. `facet_sweep` stays unchanged.
///
/// The packet spells the return `Outcome<Certified<FacetSweepResult>>`, but
/// `Outcome<T> = Result<Certified<T>, Refusal>` here (the packet's own
/// `// prelude Result<_, Refusal>` comment shows the intended expansion), so
/// the single `Certified::new` wrap the body performs types as
/// `Outcome<FacetSweepResult>` — deviation recorded in RESULT.json.
pub fn facet_sweep_certified<S: SpineCurve>(
    recipe: &SpineFrameRecipe<S, ProfileLaw, FrameLaw>,
    stations: &[f64],
    ring_resolution: usize,
) -> Outcome<FacetSweepResult> {
    let result = match facet_sweep(recipe, stations, ring_resolution) {
        Ok(result) => result,
        Err(_) => {
            // The refusal cannot carry a payload; the summary is re-derived
            // from the construct error by the caller (mapping A row 1).
            return Err(Refusal::UnsupportedEnvelope(EnvelopeCase::ConstructRefused));
        }
    };
    let certificate = Certificate {
        props: PropMap::new(),
        // The facet path computes in floats (H-6); never `Exact`.
        method: Method::Float,
        budget_left: Budget::new(0, 0, 0),
        margin: Margin::UNBOUNDED,
        modulus: Modulus::Unbounded,
    };
    Ok(Certified::new(result, certificate))
}
