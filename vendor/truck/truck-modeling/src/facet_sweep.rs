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
use truck_geometry::constructive::validation::{
    validate_correspondence, validate_scalar_law_range,
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

    // 1b. V1 profile-law domain validation, shared with the BREP entry
    // (SEM-FACET-SCALE-ZERO-001, SEM-FACET-CORRESPONDENCE-TRUNCATION-001):
    // a `Scale` law whose scalar touches zero anywhere on the CLOSED station
    // window, and a struct-literal `LinearCorrespondence` whose vertex counts
    // differ, refuse here — the same admissions the BREP path already enforces.
    // The per-station evaluator catches a zero AT a station; the window gate
    // catches the between-station zero sampling misses.
    let s_first = stations[0];
    let s_last = stations[stations.len() - 1];
    match &recipe.profile_law {
        ProfileLaw::Scale { scale, .. } => {
            validate_scalar_law_range(scale, (s_first, s_last))?;
        }
        ProfileLaw::LinearCorrespondence { start, end } => {
            validate_correspondence(start.vertices.len(), end.vertices.len())?;
        }
        ProfileLaw::Constant(_) => {}
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
    // consecutive edge pairs all cross with one strict sign. A convex ring
    // keeps the landed apex fan verbatim — the V5 byte-identity guard (the
    // cap triangulation of a convex ring is bit-identical to the pre-packet
    // build). A non-convex ring routes to the deterministic ear-clip
    // triangulation ([`cap_triangulation::triangulate`]) instead of refusing:
    // that path only ever replaces the historical refusal.
    let start_ring: Vec<Point3> = (0..k).map(|j| positions[j]).collect();
    let end_ring: Vec<Point3> = ((m - 1) * k..m * k).map(|i| positions[i]).collect();
    let start_convex = ring_is_convex(&start_ring, position_tol);
    let end_convex = ring_is_convex(&end_ring, position_tol);
    if start_convex && end_convex {
        for t in 1..k - 1 {
            tri_faces.push([0, t, t + 1]);
        }
        let r0 = (m - 1) * k;
        for t in 1..k - 1 {
            tri_faces.push([r0, r0 + t + 1, r0 + t]);
        }
    } else {
        // H-6: cap triangulation is facet output computed in floats — the same
        // evidence doctrine as the fast path, never `Exact`. Each emitted
        // triangle is a fan from the ring's own boundary orientation at the
        // start cap and reversed at the end cap (the side band's ring edges
        // are shared in the opposite direction — closure, plan §3.3).
        let start_cap = if start_convex {
            (1..k - 1).map(|t| [0usize, t, t + 1]).collect::<Vec<_>>()
        } else {
            cap_triangulation::triangulate(&start_ring, position_tol)?
        };
        let r0 = (m - 1) * k;
        let end_cap = if end_convex {
            (1..k - 1).map(|t| [0usize, t, t + 1]).collect::<Vec<_>>()
        } else {
            cap_triangulation::triangulate(&end_ring, position_tol)?
        };
        for tri in start_cap {
            tri_faces.push(tri);
        }
        for tri in end_cap {
            tri_faces.push([tri[2] + r0, tri[1] + r0, tri[0] + r0]);
        }
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

/// The concave-cap layer (PB-003-CONCAVE-CAPS): the deterministic ear-clip
/// triangulation of ONE planar cap ring.
///
/// Caps are planar, hole-free simple polygons. A convex ring never reaches
/// this module — `ring_is_convex` keeps the apex-fan fast path bit-identical
/// (the V5 guard). Only a ring that failed the convex fan is triangulated
/// here, and only when it is a simple polygon: a self-intersecting ring
/// refuses typed (`ConstructError::InvalidInput`), never repaired. The ring's
/// simplicity check precedes any triangulation.
///
/// Determinism (H-1, plan §7): the ring is projected onto its carrier plane
/// and clipped leftmost-most-convex-ear first — at every step the clipped ear
/// is the convex ear whose apex projects LEFTMOST (smallest carrier-plane x;
/// an exact x tie is broken by the smallest carrier-plane y). An exact tie
/// cannot occur for a simple ring, which never revisits a boundary point; the
/// rule makes the emitted triangle order a pure function of the ring. Every
/// emitted triangle keeps the ring's own boundary orientation (the caller
/// reverses the end cap so both caps share the side band's ring edges in the
/// opposite direction — closure, plan §3.3).
///
/// Ear clipping is O(n²) ears over an O(n) scan (n = the ring resolution).
/// Refusals are typed, never panics (H-1): a degenerate ring that offers no
/// strictly convex ear refuses instead of looping. H-6: cap triangulation is
/// facet output computed in floats — never `Exact`.
mod cap_triangulation {
    use super::*;

    /// Triangulates one planar cap ring, returning the triangles over the
    /// ring's LOCAL vertex ordinals (0..k-1), each oriented with the ring's
    /// own boundary. See the module docs for the determinism rule.
    pub(super) fn triangulate(
        ring: &[Point3],
        tolerance: f64,
    ) -> Result<Vec<[usize; 3]>, ConstructError> {
        let k = ring.len();
        if k < 3 {
            return Err(ConstructError::InvalidInput);
        }
        let coords = project(ring, tolerance)?;
        if !ring_is_simple(&coords) {
            return Err(ConstructError::InvalidInput);
        }
        let twice_area = signed_twice_area(&coords);
        if twice_area.abs() <= tolerance {
            return Err(ConstructError::InvalidInput);
        }
        let orientation = if twice_area > 0.0 { 1.0 } else { -1.0 };
        ear_clip(&coords, orientation)
    }

    /// Projects the ring onto its carrier plane as 2-D coordinates. The
    /// carrier normal is the Newell normal; when the algebraic area vanishes
    /// (a self-intersecting ring whose lobes cancel) the first non-collinear
    /// consecutive triple supplies the plane instead. A degenerate (collinear)
    /// ring refuses typed.
    fn project(ring: &[Point3], tolerance: f64) -> Result<Vec<[f64; 2]>, ConstructError> {
        let origin = Point3::origin();
        let mut normal = Vector3::zero();
        for (j, point) in ring.iter().enumerate() {
            let next = ring[(j + 1) % ring.len()];
            normal += (*point - origin).cross(next - origin);
        }
        if normal.magnitude() <= tolerance {
            normal = Vector3::zero();
            let mut found = false;
            for j in 0..ring.len() {
                let a = ring[j];
                let b = ring[(j + 1) % ring.len()];
                let c = ring[(j + 2) % ring.len()];
                let cross = (b - a).cross(c - b);
                if cross.magnitude() > tolerance {
                    normal = cross;
                    found = true;
                    break;
                }
            }
            if !found {
                return Err(ConstructError::InvalidInput);
            }
        }
        let n_hat = normal.normalize();
        let axis = if n_hat.x.abs() <= n_hat.y.abs() && n_hat.x.abs() <= n_hat.z.abs() {
            Vector3::unit_x()
        } else if n_hat.y.abs() <= n_hat.z.abs() {
            Vector3::unit_y()
        } else {
            Vector3::unit_z()
        };
        let u = n_hat.cross(axis).normalize();
        let v = n_hat.cross(u);
        let base = ring[0];
        Ok(ring
            .iter()
            .map(|point| {
                let d = *point - base;
                [d.dot(u), d.dot(v)]
            })
            .collect())
    }

    /// The ring's simplicity gate, preceding any triangulation: no zero-length
    /// edge and no two non-adjacent edges intersect. An endpoint touching a
    /// non-adjacent edge counts as an intersection — a simple ring never
    /// revisits a boundary point. Exact f64 predicates; the planar fixtures
    /// decide exactly (no repair, no tolerance games).
    fn ring_is_simple(coords: &[[f64; 2]]) -> bool {
        let n = coords.len();
        for j in 0..n {
            if coords[j] == coords[(j + 1) % n] {
                return false;
            }
        }
        for j in 0..n {
            let a = coords[j];
            let b = coords[(j + 1) % n];
            for l in (j + 1)..n {
                if l == j + 1 || (j == 0 && l == n - 1) {
                    continue;
                }
                let c = coords[l];
                let d = coords[(l + 1) % n];
                if segments_intersect(a, b, c, d) {
                    return false;
                }
            }
        }
        true
    }

    /// The signed twice-area of the ring (shoelace), giving its orientation.
    fn signed_twice_area(coords: &[[f64; 2]]) -> f64 {
        let mut sum = 0.0;
        for (j, a) in coords.iter().enumerate() {
            let b = coords[(j + 1) % coords.len()];
            sum += a[0] * b[1] - a[1] * b[0];
        }
        sum
    }

    /// The ear-clip loop over the ring's carrier-plane coordinates. Clips
    /// leftmost-most-convex-ear first (see the module docs); refuses typed if
    /// a step finds no strictly convex ear.
    fn ear_clip(coords: &[[f64; 2]], orientation: f64) -> Result<Vec<[usize; 3]>, ConstructError> {
        let mut active: Vec<usize> = (0..coords.len()).collect();
        let mut triangles: Vec<[usize; 3]> = Vec::with_capacity(coords.len() - 2);
        while active.len() > 3 {
            let len = active.len();
            let mut chosen: Option<usize> = None;
            for i in 0..len {
                let prev = active[(i + len - 1) % len];
                let cur = active[i];
                let next = active[(i + 1) % len];
                if turn2(coords, prev, cur, next) * orientation <= 0.0 {
                    continue;
                }
                if contains_other_vertex(coords, &active, prev, cur, next) {
                    continue;
                }
                chosen = match chosen {
                    None => Some(i),
                    Some(best) => {
                        let best_apex = active[best];
                        if coords[cur][0] < coords[best_apex][0]
                            || (coords[cur][0] == coords[best_apex][0]
                                && coords[cur][1] < coords[best_apex][1])
                        {
                            Some(i)
                        } else {
                            Some(best)
                        }
                    }
                };
            }
            let i = match chosen {
                Some(i) => i,
                None => return Err(ConstructError::InvalidInput),
            };
            let len = active.len();
            let prev = active[(i + len - 1) % len];
            let cur = active[i];
            let next = active[(i + 1) % len];
            triangles.push([prev, cur, next]);
            active.remove(i);
        }
        triangles.push([active[0], active[1], active[2]]);
        Ok(triangles)
    }

    /// Whether any OTHER active ring vertex lies inside or on the candidate
    /// ear triangle `(prev, cur, next)`. A convex vertex whose ear triangle
    /// holds another vertex is not an ear — its diagonal would cut the ring.
    fn contains_other_vertex(
        coords: &[[f64; 2]],
        active: &[usize],
        prev: usize,
        cur: usize,
        next: usize,
    ) -> bool {
        for &m in active {
            if m == prev || m == cur || m == next {
                continue;
            }
            if in_triangle(coords, m, prev, cur, next) {
                return true;
            }
        }
        false
    }

    /// Whether vertex `m` lies inside or on the (non-degenerate) triangle
    /// `(a, b, c)`, in either orientation.
    fn in_triangle(coords: &[[f64; 2]], m: usize, a: usize, b: usize, c: usize) -> bool {
        let o1 = orient2(coords, a, b, m);
        let o2 = orient2(coords, b, c, m);
        let o3 = orient2(coords, c, a, m);
        (o1 >= 0.0 && o2 >= 0.0 && o3 >= 0.0) || (o1 <= 0.0 && o2 <= 0.0 && o3 <= 0.0)
    }

    /// The signed 2-D turn at `b` along the path `a -> b -> c` (the z of
    /// `(b - a) x (c - b)`).
    fn turn2(coords: &[[f64; 2]], a: usize, b: usize, c: usize) -> f64 {
        let (ax, ay) = (coords[a][0], coords[a][1]);
        let (bx, by) = (coords[b][0], coords[b][1]);
        let (cx, cy) = (coords[c][0], coords[c][1]);
        let (dx1, dy1) = (bx - ax, by - ay);
        let (dx2, dy2) = (cx - bx, cy - by);
        dx1 * dy2 - dy1 * dx2
    }

    /// The signed 2-D orient of vertex `m` against the directed line
    /// `a -> b` (the z of `(b - a) x (m - a)`).
    fn orient2(coords: &[[f64; 2]], a: usize, b: usize, m: usize) -> f64 {
        let (ax, ay) = (coords[a][0], coords[a][1]);
        let (bx, by) = (coords[b][0], coords[b][1]);
        let (mx, my) = (coords[m][0], coords[m][1]);
        (bx - ax) * (my - ay) - (by - ay) * (mx - ax)
    }

    /// Whether segments `ab` and `cd` intersect (proper crossing, collinear
    /// overlap, or an endpoint touching the other segment).
    fn segments_intersect(a: [f64; 2], b: [f64; 2], c: [f64; 2], d: [f64; 2]) -> bool {
        let o1 = orient(a, b, c);
        let o2 = orient(a, b, d);
        let o3 = orient(c, d, a);
        let o4 = orient(c, d, b);
        if o1 * o2 < 0.0 && o3 * o4 < 0.0 {
            return true;
        }
        if o1 == 0.0 && on_segment(a, b, c) {
            return true;
        }
        if o2 == 0.0 && on_segment(a, b, d) {
            return true;
        }
        if o3 == 0.0 && on_segment(c, d, a) {
            return true;
        }
        if o4 == 0.0 && on_segment(c, d, b) {
            return true;
        }
        false
    }

    /// The signed 2-D orient of `c` against the directed line `a -> b`.
    fn orient(a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> f64 {
        (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
    }

    /// Whether `p` lies within the bounding box of segment `ab` (the caller
    /// has already certified `p` collinear with `ab`).
    fn on_segment(a: [f64; 2], b: [f64; 2], p: [f64; 2]) -> bool {
        p[0] >= a[0].min(b[0])
            && p[0] <= a[0].max(b[0])
            && p[1] >= a[1].min(b[1])
            && p[1] <= a[1].max(b[1])
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spine_sweep::spine_sweep;
    use truck_base::cgmath64::{Point2, Point3, Vector3};

    fn unit_square() -> Profile2D {
        Profile2D {
            vertices: vec![
                Point2::new(0.0, 0.0),
                Point2::new(1.0, 0.0),
                Point2::new(1.0, 1.0),
                Point2::new(0.0, 1.0),
            ],
        }
    }

    fn regular_polygon(n: usize, radius: f64) -> Profile2D {
        let vertices = (0..n)
            .map(|i| {
                let t = 2.0 * std::f64::consts::PI * (i as f64) / (n as f64);
                Point2::new(radius * t.cos(), radius * t.sin())
            })
            .collect();
        match Profile2D::try_closed(vertices) {
            Ok(profile) => profile,
            Err(error) => panic!("regular polygon must be a valid closed profile: {error:?}"),
        }
    }

    fn vertical_line() -> LineSpine {
        LineSpine {
            start: Point3::new(0.0, 0.0, 0.0),
            end: Point3::new(0.0, 0.0, 2.0),
        }
    }

    fn stations(spine: usize) -> Vec<f64> {
        match (SamplingPolicy::UniformCount { spine }).resolve(0.0, 1.0) {
            Ok(stations) => stations,
            Err(error) => panic!("the station fixture must resolve: {error:?}"),
        }
    }

    fn through_zero_recipe() -> SpineFrameRecipe<LineSpine, ProfileLaw, FrameLaw> {
        SpineFrameRecipe::new(
            vertical_line(),
            ProfileLaw::Scale {
                profile: unit_square(),
                scale: ScalarLaw::Linear {
                    start: 1.0,
                    end: -1.0,
                },
            },
            FrameLaw::FixedPlane {
                normal: Vector3::unit_x(),
            },
        )
    }

    fn mismatched_correspondence_recipe() -> SpineFrameRecipe<LineSpine, ProfileLaw, FrameLaw> {
        // Built as a STRUCT LITERAL, deliberately bypassing
        // `try_linear_correspondence`: the defect path.
        SpineFrameRecipe::new(
            vertical_line(),
            ProfileLaw::LinearCorrespondence {
                start: regular_polygon(4, 0.2),
                end: regular_polygon(6, 0.1),
            },
            FrameLaw::FixedPlane {
                normal: Vector3::unit_x(),
            },
        )
    }

    fn valid_recipe() -> SpineFrameRecipe<LineSpine, ProfileLaw, FrameLaw> {
        SpineFrameRecipe::new(
            LineSpine {
                start: Point3::origin(),
                end: Point3::new(0.0, 0.0, 1.0),
            },
            ProfileLaw::Constant(unit_square()),
            FrameLaw::FixedPlane {
                normal: Vector3::unit_x(),
            },
        )
    }

    #[test]
    fn facet_path_refuses_through_zero_scale() {
        // SEM-FACET-SCALE-ZERO-001: the facet backend must refuse a through-zero
        // `Scale` law the moment the shared window validator sees the sign
        // change — the same typed refusal the BREP entry produces.
        let recipe = through_zero_recipe();
        let result = facet_sweep(&recipe, &stations(4), 4);
        match result {
            Err(ConstructError::ProfileCollapse { at }) => {
                let expected = 0.5;
                assert!(
                    (at - expected).abs() <= 1.0e-12, // H-3: the linear root is exactly 0.5
                    "the collapse must be reported at the closed-form zero {expected}, got {at}"
                );
            }
            other => panic!(
                "through-zero scale must refuse ProfileCollapse on the facet path, got {other:?}"
            ),
        }
    }

    #[test]
    fn facet_path_refuses_mismatched_correspondence() {
        // SEM-FACET-CORRESPONDENCE-TRUNCATION-001: a struct-literal
        // `LinearCorrespondence` with differing vertex counts must refuse at the
        // facet entry — never zip-truncate to the shorter profile.
        let recipe = mismatched_correspondence_recipe();
        let result = facet_sweep(&recipe, &stations(4), 4);
        match result {
            Err(ConstructError::ProfileCorrespondenceMismatch) => {}
            other => panic!(
                "mismatched correspondence must refuse ProfileCorrespondenceMismatch \
                 on the facet path, got {other:?}"
            ),
        }
    }

    #[test]
    fn spine_sweep_refusals_unchanged_on_the_twin_fixtures() {
        // The BREP entry's refusals on the twin fixtures are unchanged by the
        // shared-validator move: both SEM fixtures still refuse at the entry
        // with the same constructive-refusal envelope.
        let through_zero = spine_sweep(&through_zero_recipe(), &stations(4));
        match through_zero {
            Err(Refusal::UnsupportedEnvelope(EnvelopeCase::ConstructRefused)) => {}
            other => panic!("the BREP twin must still refuse through-zero scale, got {other:?}"),
        }
        let mismatched = spine_sweep(&mismatched_correspondence_recipe(), &stations(4));
        match mismatched {
            Err(Refusal::UnsupportedEnvelope(EnvelopeCase::ConstructRefused)) => {}
            other => {
                panic!("the BREP twin must still refuse mismatched correspondence, got {other:?}")
            }
        }
    }

    #[test]
    fn valid_recipes_still_realize_on_both_paths() {
        // Backend symmetry, positive direction: a valid recipe realizes on the
        // facet path AND the authored-BREP path, with a clean winding audit on
        // the facet mesh.
        let recipe = valid_recipe();
        let result = facet_sweep(&recipe, &[0.0, 0.5, 1.0], 4);
        match result {
            Ok(realized) => {
                assert_eq!(realized.audit.winding_violations, 0);
            }
            Err(error) => panic!("the valid recipe must realize on the facet path: {error:?}"),
        }
        match spine_sweep(&recipe, &[0.0, 1.0]) {
            Ok(_) => {}
            Err(refusal) => panic!("the valid recipe must realize on the BREP path: {refusal:?}"),
        }
    }
}
