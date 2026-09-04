#![cfg_attr(not(debug_assertions), deny(warnings))]
#![deny(clippy::all, rust_2018_idioms)]
#![deny(clippy::unwrap_used)]
#![warn(
    missing_docs,
    missing_debug_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unstable_features,
    unused_import_braces,
    unused_qualifications
)]

//! CC-005-GRAPHDISK (spine seam S6): the P3 graph-disk embedding certificate
//! and its normative projection search.
//!
//! P3 certifies injectivity for a GLUED region — a corner patch or a closed
//! offset star — where no single parameterization exists. The certificate is
//! the DECIDER over caller-supplied certificates: a projection `w` with a
//! strictly positive determinant lower bound on every piece, every seam glued,
//! and a simple projected boundary makes the projection a homeomorphism onto a
//! Jordan domain, hence `P` injective (theory §1 P3).
//!
//! **Section 1 — the decider.** [`DiskPiece`] is the per-piece certificate a
//! consumer builds from its own derivative enclosures (PUB fields, no solver
//! inside CC-005). [`certify_graph_disk`] applies the pre-made decision table
//! in frozen order — no heuristics, no repair, no second chances:
//!
//! 1. any piece whose `det_lower` is NOT strictly positive (sup ≤ 0 or an
//!    enclosure straddling 0; a positive margin requires inf > 0) →
//!    `Err(ConstructRefusal::NoAdmissibleProjection)` (the caller must search
//!    another projection);
//! 2. any piece with `seam_glued == false` → `Err(ConstructRefusal::StarNotEmbedded)`
//!    (theory §1 P3: the seam clause is NOT implied by per-piece determinants);
//! 3. a non-simple projected boundary (the [`BoundaryPlan`] verdict) →
//!    `Err(ConstructRefusal::StarNotEmbedded)`;
//! 4. all pass → `Ok(GraphDiskCert)` (the per-piece witness records).
//!
//! **Section 2 — the projection search.** [`search_projection`] runs the FROZEN
//! normative candidate sequence from theory §1 P3 over the caller's admitted
//! pieces: (1) the area-weighted average patch normal; (2) the principal
//! directions of the control net; (3) the fixed 14-point spherical code
//! [`SPHERICAL_CODE_14`] (the exact, order-stable table). Per candidate `w`,
//! each piece's determinant lower bound is the interval arithmetic evaluation
//! `w · (S_u × S_v)` of the caller-provided per-piece derivative enclosure, and
//! the projected-boundary simplicity verdict comes from the caller's planar
//! discharge (the near-diagonal P2 radius over the plane-projected arcs and
//! planar exclusion of non-adjacent arcs, theory §1 P3 hypothesis (2) —
//! [`projected_boundary_simplicity`] is that discharge over caller-certified
//! projected arcs). Exhaustion → `Err(ConstructRefusal::NoAdmissibleProjection)`.
//! The pairwise patch/patch SSI fallback with an inside/outside witness is a
//! LATER packet (CC-014's composition), not this one.
//!
//! **H-1.** This module carries no `unwrap`, no `expect`, and no `panic!`, and
//! adds no module-level `allow`. All reductions run in fixed order with
//! directed rounding.

use crate::construct::refusal::ConstructRefusal;
use crate::construct::stubs::BoundaryPlan;
use crate::construct::Interval;

/// The inverse-root-of-three diagonal coordinate of the spherical-code table
/// (`1/√3`, the unit octant body diagonal), written out so the table is a
/// self-contained `pub const` with exact `f64` values.
const INV_SQRT_3: f64 = 0.5773502691896258; // H-3: exact table vertex coordinate (1/sqrt(3))

/// The frozen v1 spherical code: the 14-point vertices of the refined
/// octahedron.
///
/// The first six points are the octahedron's ±coordinate-axis vertices (the
/// cube's six face-centre directions), in the fixed order `+X, −X, +Y, −Y,
/// +Z, −Z`. The remaining eight points are the cube-octant body diagonals
/// `(±1, ±1, ±1)/√3` (the refined octahedron's face centres), in the fixed
/// sign order `(+++), (++−), (+−+), (+−−), (−++), (−+−), (−−+), (−−−)`. The
/// order is normative and stable: consumers and tests read ONE table.
pub const SPHERICAL_CODE_14: [[f64; 3]; 14] = [
    // The six ±axis octahedron vertices.
    [1.0, 0.0, 0.0],
    [-1.0, 0.0, 0.0],
    [0.0, 1.0, 0.0],
    [0.0, -1.0, 0.0],
    [0.0, 0.0, 1.0],
    [0.0, 0.0, -1.0],
    // The eight octant body diagonals (±1, ±1, ±1)/√3, fixed sign order.
    [INV_SQRT_3, INV_SQRT_3, INV_SQRT_3],
    [INV_SQRT_3, INV_SQRT_3, -INV_SQRT_3],
    [INV_SQRT_3, -INV_SQRT_3, INV_SQRT_3],
    [INV_SQRT_3, -INV_SQRT_3, -INV_SQRT_3],
    [-INV_SQRT_3, INV_SQRT_3, INV_SQRT_3],
    [-INV_SQRT_3, INV_SQRT_3, -INV_SQRT_3],
    [-INV_SQRT_3, -INV_SQRT_3, INV_SQRT_3],
    [-INV_SQRT_3, -INV_SQRT_3, -INV_SQRT_3],
];

/// One disk piece of the glued region: the caller's own per-piece certificate
/// (PUB fields — consumers construct these from their own derivative
/// enclosures; no solver body lives in this module).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DiskPiece {
    /// The certified lower bound of `det D(π_w ∘ P)` on the piece: the
    /// interval arithmetic evaluation of `w · (S_u × S_v)` over the piece
    /// against the winning projection `w`.
    pub det_lower: Interval,
    /// Whether the piece's own boundary arcs are certified simple.
    pub boundary_simple: bool,
    /// Whether the piece's seams to its neighbours are certified glued.
    pub seam_glued: bool,
}

/// The graph-disk certificate: the per-piece witness records that certified.
///
/// The winning projection `w` lives on [`search_projection`]'s
/// `Ok((w, pieces))` return; the decider itself re-derives no direction — it
/// is the gate over caller-supplied certificates, and its `Ok` is the witness
/// record set a consumer attaches to its own projection.
#[derive(Debug, Clone, PartialEq)]
pub struct GraphDiskCert {
    /// The certified per-piece records, in the caller's order.
    pub pieces: Vec<DiskPiece>,
}

/// The P3 graph-disk DECIDER (spine seam S6): a pre-made decision table over
/// caller-supplied per-piece certificates.
///
/// Checked in frozen order:
///
/// 1. any piece whose `det_lower` is NOT strictly positive (require
///    `inf > 0`; `sup ≤ 0` or an enclosure straddling 0 is not admissible) →
///    [`ConstructRefusal::NoAdmissibleProjection`] — the caller must search
///    another projection;
/// 2. any piece with `seam_glued == false` →
///    [`ConstructRefusal::StarNotEmbedded`] (the seam clause is NOT implied by
///    per-piece determinants);
/// 3. a non-simple projected boundary (the [`BoundaryPlan`] verdict) →
///    [`ConstructRefusal::StarNotEmbedded`];
/// 4. otherwise `Ok(GraphDiskCert)` with the witness records.
///
/// No heuristics, no repair, no second chances inside this function.
pub fn certify_graph_disk(
    pieces: &[DiskPiece],
    boundary: &BoundaryPlan,
) -> Result<GraphDiskCert, ConstructRefusal> {
    for piece in pieces {
        if piece.det_lower.lo <= 0.0 || !piece.det_lower.lo.is_finite() {
            return Err(ConstructRefusal::NoAdmissibleProjection);
        }
    }
    for piece in pieces {
        if !piece.seam_glued {
            return Err(ConstructRefusal::StarNotEmbedded);
        }
    }
    if !boundary.boundary_simple {
        return Err(ConstructRefusal::StarNotEmbedded);
    }
    Ok(GraphDiskCert {
        pieces: pieces.to_vec(),
    })
}

/// One admitted piece of the surface the projection search runs over: the
/// caller's own per-piece derivative certificate plus the surface-space facts
/// that do not depend on the projection.
#[derive(Debug, Clone)]
pub struct AdmittedPiece {
    /// The certified per-piece enclosure of `S_u × S_v` (the per-piece
    /// derivative enclosure; `w · normal_box` evaluated in interval arithmetic
    /// bounds `det D(π_w ∘ P)` on the piece).
    pub normal_box: [Interval; 3],
    /// A certified lower bound of `‖S_u × S_v‖` over the piece — the area
    /// density used to form the area-weighted average patch normal (candidate
    /// family (1)).
    pub area_lower: f64,
    /// The mean control-net `u`-parameter leg direction (unit hint; candidate
    /// family (2), principal direction 1).
    pub net_u: [f64; 3],
    /// The mean control-net `v`-parameter leg direction (unit hint; candidate
    /// family (2), principal direction 2).
    pub net_v: [f64; 3],
    /// Whether this piece's seams to its neighbours are certified glued.
    pub seam_glued: bool,
}

/// The FROZEN normative projection-candidate sequence (theory §1 P3), in
/// order:
///
/// 1. the area-weighted average patch normal
///    `unit(Σ_k area_lower_k · mid(normal_box_k))`;
/// 2. the two principal directions of the control net — the unit means of the
///    pieces' `net_u` and `net_v` leg directions;
/// 3. the fixed 14-point spherical code [`SPHERICAL_CODE_14`], in table order.
///
/// Degenerate (zero-length) directions are skipped; the code family always
/// contributes its 14 points, so a non-empty piece set never yields an empty
/// sequence.
pub fn projection_candidates(pieces: &[AdmittedPiece]) -> Vec<[f64; 3]> {
    let mut out: Vec<[f64; 3]> = Vec::new();
    if let Some(w) = area_weighted_patch_normal(pieces) {
        out.push(w);
    }
    if let Some(w) = mean_net_direction(pieces, 0) {
        out.push(w);
    }
    if let Some(w) = mean_net_direction(pieces, 1) {
        out.push(w);
    }
    for w in SPHERICAL_CODE_14.iter().copied() {
        out.push(w);
    }
    out
}

/// The projection search over an admitted surface's pieces: returns the FIRST
/// candidate `w` — in the frozen normative sequence — under which every
/// piece's determinant lower bound certifies strictly positive, every seam is
/// glued, and the caller's projected-boundary verdict is simple, together with
/// the per-piece [`DiskPiece`] records evaluated against `w`.
///
/// `boundary_simple(w)` supplies the certified projected-boundary verdict for
/// each candidate projection `w` (the discharge of theory §1 P3 hypothesis (2)
/// over the plane-projected boundary arcs — see
/// [`projected_boundary_simplicity`]); a `false` verdict for a candidate moves
/// the search to the next candidate, as does a refused determinant. Exhaustion
/// → [`ConstructRefusal::NoAdmissibleProjection`]. An empty piece set is an
/// input defect ([`ConstructRefusal::InvalidInput`]); an unglued seam is a
/// surface-space defect ([`ConstructRefusal::StarNotEmbedded`]) that no
/// projection can repair.
///
/// This function does NOT implement pairwise patch/patch SSI — that is
/// CC-014's composition.
pub fn search_projection<B>(
    pieces: &[AdmittedPiece],
    mut boundary_simple: B,
) -> Result<([f64; 3], Vec<DiskPiece>), ConstructRefusal>
where
    B: FnMut([f64; 3]) -> Result<bool, ConstructRefusal>,
{
    if pieces.is_empty() {
        return Err(ConstructRefusal::InvalidInput);
    }
    let seams_glued = pieces.iter().all(|piece| piece.seam_glued);
    if !seams_glued {
        return Err(ConstructRefusal::StarNotEmbedded);
    }
    let candidates = projection_candidates(pieces);
    for w in candidates {
        let simple = boundary_simple(w)?;
        if !simple {
            continue;
        }
        let mut disks = Vec::with_capacity(pieces.len());
        for piece in pieces {
            disks.push(DiskPiece {
                det_lower: det_interval(w, &piece.normal_box),
                boundary_simple: true,
                seam_glued: true,
            });
        }
        let plan = BoundaryPlan {
            boundary_simple: true,
            seams_glued: true,
        };
        if certify_graph_disk(&disks, &plan).is_ok() {
            return Ok((w, disks));
        }
    }
    Err(ConstructRefusal::NoAdmissibleProjection)
}

/// One arc of the closed, plane-projected region boundary, caller-certified
/// (theory §1 P3 hypothesis (2) discharge input).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoundaryArc {
    /// The arc's planar enclosing box in the projection plane.
    pub planar_box: [Interval; 2],
    /// The certified P2 injectivity radius of the plane-projected arc (the
    /// output of [`crate::construct::injectivity::curve_injectivity_radius`]
    /// over the projected curve) — the near-diagonal radius.
    pub radius: Interval,
    /// The boundary vertex index this arc starts at.
    pub start: usize,
    /// The boundary vertex index this arc ends at.
    pub end: usize,
}

/// The projected-boundary simplicity discharge (theory §1 P3 hypothesis (2)):
/// certify that the closed planar chain of projected boundary arcs is simple.
///
/// Three clauses, all discharged over the caller-certified arc records:
///
/// - **Near-diagonal radius:** every arc's certified P2 radius lower bound is
///   strictly positive, so no arc folds back within its own diagonal
///   neighbourhood (a fold arc carries `radius.lo ≤ 0`);
/// - **Closed single chain:** the arcs connect distinct vertices, every vertex
///   has exactly one incoming and one outgoing arc, no two arcs share both
///   endpoints, and following the chain from the first arc closes a single
///   cycle covering every arc;
/// - **Planar exclusion:** every pair of non-adjacent arcs (sharing no
///   vertex) is strictly separated in the plane — their planar boxes are
///   disjoint on at least one axis, so no non-adjacent projected arc can meet.
///
/// Returns `Ok(true)` when the discharge certifies a simple projected
/// boundary, `Ok(false)` when it refutes or cannot certify simplicity (the
/// search then tries another projection), and
/// [`ConstructRefusal::InvalidInput`] on structurally corrupt input
/// (non-finite or misordered boxes or radii). An empty chain (a closed star
/// with no boundary) is trivially simple.
pub fn projected_boundary_simplicity(arcs: &[BoundaryArc]) -> Result<bool, ConstructRefusal> {
    validate_arc_records(arcs)?;
    if arcs.is_empty() {
        return Ok(true);
    }
    if arcs.len() < 3 {
        return Ok(false);
    }

    // Clause (a): the near-diagonal radius is strictly positive on every arc.
    for arc in arcs {
        if arc.radius.lo <= 0.0 {
            return Ok(false);
        }
    }

    // Clause (b): a closed single chain over distinct vertices.
    if !single_closed_chain(arcs) {
        return Ok(false);
    }

    // Clause (c): planar exclusion between every pair of non-adjacent arcs.
    for i in 0..arcs.len() {
        for j in (i + 1)..arcs.len() {
            if arcs_share_vertex(&arcs[i], &arcs[j]) {
                continue;
            }
            let a = &arcs[i].planar_box;
            let b = &arcs[j].planar_box;
            let separated =
                a[0].hi < b[0].lo || b[0].hi < a[0].lo || a[1].hi < b[1].lo || b[1].hi < a[1].lo;
            if !separated {
                return Ok(false);
            }
        }
    }

    Ok(true)
}

/// Whether two arcs share a boundary vertex.
fn arcs_share_vertex(a: &BoundaryArc, b: &BoundaryArc) -> bool {
    a.start == b.start || a.start == b.end || a.end == b.start || a.end == b.end
}

/// Whether the arc records form one closed chain over distinct vertices:
/// every arc connects distinct vertices, no two arcs share both endpoints,
/// each vertex has exactly one incoming and one outgoing arc, and walking the
/// chain from the first arc closes over every arc.
fn single_closed_chain(arcs: &[BoundaryArc]) -> bool {
    for arc in arcs {
        if arc.start == arc.end {
            return false;
        }
    }
    for i in 0..arcs.len() {
        for j in (i + 1)..arcs.len() {
            let shared_start = arcs[i].start == arcs[j].start && arcs[i].end == arcs[j].end;
            let shared_reversed = arcs[i].start == arcs[j].end && arcs[i].end == arcs[j].start;
            if shared_start || shared_reversed {
                return false;
            }
        }
    }
    let max_vertex = arcs
        .iter()
        .map(|arc| arc.start.max(arc.end))
        .max()
        .unwrap_or(0);
    let mut starts = vec![0usize; max_vertex + 1];
    let mut ends = vec![0usize; max_vertex + 1];
    for arc in arcs {
        starts[arc.start] += 1;
        ends[arc.end] += 1;
    }
    for vertex in 0..=max_vertex {
        if starts[vertex] + ends[vertex] > 0 && (starts[vertex] != 1 || ends[vertex] != 1) {
            return false;
        }
    }
    let mut start_of = vec![usize::MAX; max_vertex + 1];
    for (index, arc) in arcs.iter().enumerate() {
        start_of[arc.start] = index;
    }
    let first_vertex = arcs[0].start;
    let mut visited = 0usize;
    let mut vertex = first_vertex;
    loop {
        let index = start_of[vertex];
        if index == usize::MAX {
            return false;
        }
        visited += 1;
        vertex = arcs[index].end;
        if vertex == first_vertex {
            break;
        }
    }
    visited == arcs.len()
}

/// Structural input validation of the discharge records: finite, ordered
/// planar boxes and a non-NaN, ordered radius.
fn validate_arc_records(arcs: &[BoundaryArc]) -> Result<(), ConstructRefusal> {
    for arc in arcs {
        for axis in [&arc.planar_box[0], &arc.planar_box[1]] {
            if !(axis.lo.is_finite() && axis.hi.is_finite()) {
                return Err(ConstructRefusal::InvalidInput);
            }
            if axis.lo > axis.hi {
                return Err(ConstructRefusal::InvalidInput);
            }
        }
        if arc.radius.lo.is_nan() || arc.radius.hi.is_nan() {
            return Err(ConstructRefusal::InvalidInput);
        }
        if arc.radius.lo > arc.radius.hi {
            return Err(ConstructRefusal::InvalidInput);
        }
    }
    Ok(())
}

/// The area-weighted average patch normal: `unit(Σ area_lower_k ·
/// mid(normal_box_k))`. `None` when the weighted sum is degenerate.
fn area_weighted_patch_normal(pieces: &[AdmittedPiece]) -> Option<[f64; 3]> {
    let mut acc = [0.0_f64; 3];
    for piece in pieces {
        for (slot, coordinate) in acc.iter_mut().zip(piece.normal_box.iter()) {
            *slot += piece.area_lower * interval_mid(coordinate);
        }
    }
    unit(acc)
}

/// The mean control-net leg direction over the pieces: `axis 0` averages the
/// `u` legs, `axis 1` the `v` legs.
fn mean_net_direction(pieces: &[AdmittedPiece], axis: usize) -> Option<[f64; 3]> {
    let mut acc = [0.0_f64; 3];
    for piece in pieces {
        let leg = if axis == 0 { piece.net_u } else { piece.net_v };
        for (slot, component) in acc.iter_mut().zip(leg.iter()) {
            *slot += component;
        }
    }
    unit(acc)
}

/// The interval arithmetic evaluation of `w · (S_u × S_v)` over a piece from
/// its derivative enclosure, in fixed coordinate order.
fn det_interval(w: [f64; 3], normal_box: &[Interval; 3]) -> Interval {
    let mut acc = Interval::point(w[0]).mul(&normal_box[0]);
    acc = acc.add(&Interval::point(w[1]).mul(&normal_box[1]));
    acc.add(&Interval::point(w[2]).mul(&normal_box[2]))
}

/// The midpoint of a certified interval (`(lo + hi)/2`), used only to form the
/// heuristic candidate directions; certification never relies on it.
fn interval_mid(value: &Interval) -> f64 {
    0.5 * (value.lo + value.hi)
}

/// Unit-normalize a direction; `None` for a degenerate (zero, non-finite or
/// NaN) direction.
fn unit(v: [f64; 3]) -> Option<[f64; 3]> {
    let sq = v[0] * v[0] + v[1] * v[1] + v[2] * v[2];
    if !sq.is_finite() || sq <= 0.0 {
        return None;
    }
    let length = sq.sqrt();
    if !length.is_finite() || length <= 0.0 {
        return None;
    }
    Some([v[0] / length, v[1] / length, v[2] / length])
}
