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

//! CC-022-STARS (CC program Phase C, spine S6 consumer; theory §4.1 stars,
//! §4.3 broad phase): the closed-star embedding certificate over the CC-021
//! constructed strata and the certified broad phase over constructed strata.
//!
//! Within a single stratum CC-021's P2 injectivity radius supplies the local
//! self-contact bound; a CLOSED STAR spanning several glued strata is the
//! place where P3 (the graph-disk certificate) does the work: a projection
//! with a strictly positive determinant lower bound on every piece, every
//! internal seam glued, and a simple projected boundary makes the projection
//! a homeomorphism onto its image — the constructive form of local
//! embeddedness at edges and corners.
//!
//! **Section 1 — the star.** [`Star`] is a caller-supplied combinatorial
//! object: the ordered CC-021 strata plus a [`GluePlan`] recording which
//! stratum boundaries are identified. The plan is EXACT and identity-based
//! (P6): a seam names the two strata and, on each side, the boundary of that
//! stratum together with the identity of the shared feature; agreement on the
//! shared identity is checked combinatorially and is NEVER matched by
//! proximity. [`certify_star`] reduces the star to CC-005's machinery:
//!
//! 1. a structural validation of the plan ([`ConstructRefusal::InvalidInput`]
//!    on out-of-range or self seams, duplicated sides, empty stars);
//! 2. the PRE-MADE glue gate, run BEFORE any graph-disk / projection search:
//!    the two boundary references of every seam must name the SAME shared
//!    identity — a mismatch is [`ConstructRefusal::StarNotEmbedded`];
//! 3. the graph-disk reduction: every stratum becomes one
//!    [`DiskPiece`](crate::construct::graphdisk::DiskPiece)
//!    whose determinant data is the interval `w · (X_u × X_v)` over the piece
//!    against the winning projection `w` — built from the piece's landed
//!    regularity certificate (the CC-021 face `J_t` margin) times the source
//!    area-normal enclosure of the carrier map the caller supplied inside the
//!    stratum. The frozen CC-005 candidate sequence is searched; the projected
//!    boundary is discharged per candidate through CC-005's
//!    [`projected_boundary_simplicity`] over the glued fan's outer rim; the
//!    winning
//!    [`DiskPiece`](crate::construct::graphdisk::DiskPiece)
//!    records become the returned
//!    [`GraphDiskCert`](crate::construct::graphdisk::GraphDiskCert).
//!
//! **Certified scope (v1).** The graph-disk reduction in this module
//! certifies stars whose strata are all k=1 face strata whose carrier map is
//! a single-patch flat (affine) map: the per-piece projected-determinant data
//! reduces to the landed `J_t` margin times the constant area normal, and the
//! outer-rim discharge reduces to straight offset segments. The CC-021 edge
//! and corner strata CANNOT supply per-piece determinant data from their
//! landed certificates — the canal frame and the corner-cap extent are not
//! carried — so a star containing them refuses
//! [`ConstructRefusal::NoAdmissibleProjection`] here and is recorded as the
//! missing datum for the CC-023 contact funnel. See RESULT notes.
//!
//! **Section 2 — the broad phase.** [`reach_prune`] prunes on certified reach
//! bounds over the constructed strata: with `ρ_A` the certified per-stratum
//! reach (`A3`), the theory §4.3 prune `d(A, B) > ρ_A + ρ_B ⟹ realizations
//! disjoint` holds, where `d` is the CC-004 axis-gap lower bound on the
//! strata's bounding boxes (the landed `Bvh::distance_lower_bound` is the
//! piece-set form of the same fact; at this layer boxes suffice). Retained
//! pairs are returned sorted and deduplicated. **The prune is SOUND but not
//! complete**: retained pairs still go to the CC-023 contact funnel.
//!
//! **H-1.** This module carries no `unwrap`, no `expect`, and no `panic!`,
//! and adds no module-level `allow`. All reductions run in fixed order.

use crate::certified_map::CertifiedSurfaceMap;
use crate::construct::graphdisk::{
    projected_boundary_simplicity, search_projection, AdmittedPiece, BoundaryArc, GraphDiskCert,
};
use crate::construct::offset_strata::OffsetStratum;
use crate::construct::refusal::ConstructRefusal;
use crate::construct::Interval;
use crate::hull::bernstein_derivative_2d;

/// One boundary side of a face stratum's domain rectangle, in the map's own
/// parameter order. A face carries exactly four: `UMin` (u = 0), `UMax`
/// (u = 1), `VMin` (v = 0), and `VMax` (v = 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FaceSide {
    /// The `u = 0` side of the face domain.
    UMin,
    /// The `u = 1` side of the face domain.
    UMax,
    /// The `v = 0` side of the face domain.
    VMin,
    /// The `v = 1` side of the face domain.
    VMax,
}

impl FaceSide {
    /// The canonical side order index (`UMin = 0, UMax = 1, VMin = 2,
    /// VMax = 3`), used for deterministic bookkeeping only.
    fn index(self) -> usize {
        match self {
            FaceSide::UMin => 0,
            FaceSide::UMax => 1,
            FaceSide::VMin => 2,
            FaceSide::VMax => 3,
        }
    }

    /// The corner `(u_corner, v_corner)` at the side's low-parameter end
    /// (`end = 0`) and high-parameter end (`end = 1`). The low end of a
    /// `u`-side is the `v = 0` corner; the low end of a `v`-side is the
    /// `u = 0` corner.
    fn end_corner(self, end: usize) -> (usize, usize) {
        match self {
            FaceSide::UMin => (0, end),
            FaceSide::UMax => (1, end),
            FaceSide::VMin => (end, 0),
            FaceSide::VMax => (end, 1),
        }
    }

    /// The corner pair `(start, end)` the side is traversed along when the
    /// face boundary is walked counter-clockwise in `(u, v)`.
    fn arc_corners(self) -> ((usize, usize), (usize, usize)) {
        match self {
            FaceSide::VMin => ((0, 0), (1, 0)),
            FaceSide::UMax => ((1, 0), (1, 1)),
            FaceSide::VMax => ((1, 1), (0, 1)),
            FaceSide::UMin => ((0, 1), (0, 0)),
        }
    }
}

/// The identity of a shared boundary feature. Identifications reference
/// boundaries BY IDENTITY (P6) — this token is never compared against any
/// geometry or proximity datum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SharedBoundary(u64);

impl SharedBoundary {
    /// A shared-boundary identity token.
    pub const fn new(id: u64) -> Self {
        SharedBoundary(id)
    }
}

/// One side of one seam: which stratum, which of its boundaries, and the
/// identity of the shared feature that boundary lies along.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoundaryRef {
    /// The index of the stratum in the star's stratum list.
    pub stratum: usize,
    /// Which boundary of that stratum the seam uses.
    pub side: FaceSide,
    /// The identity of the shared boundary feature this side references.
    pub boundary: SharedBoundary,
}

/// One glue seam: an identification of two stratum boundaries.
///
/// The two references carry the intended identification; [`certify_star`]
/// refuses any seam whose two sides do not agree on the shared boundary's
/// identity BEFORE the graph-disk reduction runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Glue {
    /// The first identified boundary.
    pub a: BoundaryRef,
    /// The second identified boundary.
    pub b: BoundaryRef,
}

/// The caller-supplied combinatorial glue plan of a star: which stratum
/// boundaries are identified.
///
/// The plan is exact and identity-based — identified boundaries are
/// referenced by identity, never matched by proximity.
#[derive(Debug, Clone, Default)]
pub struct GluePlan {
    /// The seams, in the caller's order (kept deterministic as given).
    pub seams: Vec<Glue>,
}

/// A closed star over the CC-021 constructed strata: the ordered strata and
/// the glue plan naming which of their boundaries are identified.
#[derive(Debug, Clone)]
pub struct Star {
    /// The strata of the star, in a fixed caller order.
    pub strata: Vec<OffsetStratum>,
    /// The intended identifications between the strata's boundaries.
    pub glue_plan: GluePlan,
}

/// The certified per-face geometry the graph-disk reduction reads off a face
/// stratum: the (offset) corner points, the landed `J_t` margin, the constant
/// source area normal, and the control-net legs.
struct FaceGeometry {
    /// The offset-stratum corner points, `corners[u][v]`, in world space.
    corners: [[[f64; 3]; 2]; 2],
    /// The constant source area normal `S_u × S_v` (source units), scaled by
    /// the landed `J_t` margin, as a point interval per coordinate.
    normal_box: [Interval; 3],
    /// A certified lower bound of `‖S_u × S_v‖` over the face.
    area_lower: f64,
    /// The source map's control-net `u`-leg direction.
    net_u: [f64; 3],
    /// The source map's control-net `v`-leg direction.
    net_v: [f64; 3],
}

/// The certify_star graph-disk reduction over a face-stratum star.
///
/// Every stratum must be a k=1 face whose carrier map is a single-patch flat
/// (affine) map; any other stratum kind or a non-affine / multi-patch carrier
/// cannot supply the per-piece projection data at this layer and refuses
/// [`ConstructRefusal::NoAdmissibleProjection`].
fn face_geometry(stratum: &OffsetStratum) -> Result<FaceGeometry, ConstructRefusal> {
    let (map, offset, j_t) = match stratum {
        OffsetStratum::Face {
            map,
            offset,
            j_t_lower,
        } => (map, *offset, *j_t_lower),
        _ => return Err(ConstructRefusal::NoAdmissibleProjection),
    };
    if !offset.is_finite() {
        return Err(ConstructRefusal::InvalidInput);
    }
    let boxes = map.patch_boxes();
    if boxes.len() != 1 {
        return Err(ConstructRefusal::NoAdmissibleProjection);
    }
    let grids = map.patch_grids();
    if grids.len() != 1 {
        return Err(ConstructRefusal::InvalidInput);
    }
    let domain = boxes[0];
    let width_u = domain.0 .1 - domain.0 .0;
    let width_v = domain.1 .1 - domain.1 .0;
    if !width_u.is_finite() || !width_v.is_finite() || width_u <= 0.0 || width_v <= 0.0 {
        return Err(ConstructRefusal::InvalidInput);
    }
    let grid = &grids[0];
    let corners = surface_corners(grid)?;
    if !grid_is_flat(grid) {
        return Err(ConstructRefusal::NoAdmissibleProjection);
    }
    let leg_u = sub3(corners[1][0], corners[0][0]);
    let leg_v = sub3(corners[0][1], corners[0][0]);
    let cross_world = cross3(leg_u, leg_v);
    let norm = norm3(cross_world);
    if !norm.is_finite() || norm <= 0.0 {
        return Err(ConstructRefusal::InvalidInput);
    }
    let normal = scale3(cross_world, 1.0 / norm);
    let offset_corners = offset_corners(&corners, offset, normal);
    // The source area normal in source units: `S_u × S_v = (leg_u × leg_v) /
    // (width_u · width_v)` (the legs span the whole patch widths).
    let inv_scale = 1.0 / (width_u * width_v);
    let cross_source = scale3(cross_world, inv_scale);
    let mut normal_box = [Interval::point(0.0); 3];
    for (k, component) in normal_box.iter_mut().enumerate() {
        *component = j_t.mul(&Interval::point(cross_source[k]));
    }
    let area_lower = (norm * inv_scale).next_down();
    if !area_lower.is_finite() || area_lower <= 0.0 {
        return Err(ConstructRefusal::InvalidInput);
    }
    Ok(FaceGeometry {
        corners: offset_corners,
        normal_box,
        area_lower,
        net_u: leg_u,
        net_v: leg_v,
    })
}

/// The four source corners of a single-patch surface map, `corners[u][v]`,
/// read off the patch's coefficient-grid corners (row = u, column = v).
fn surface_corners(grid: &[Vec<Vec<f64>>; 3]) -> Result<[[[f64; 3]; 2]; 2], ConstructRefusal> {
    let mut corners = [[[0.0_f64; 3]; 2]; 2];
    for coordinate in grid {
        let rows = coordinate.len();
        let cols = coordinate[0].len();
        if rows == 0 || cols == 0 {
            return Err(ConstructRefusal::InvalidInput);
        }
        let corner = |r: usize, c: usize| coordinate[r][c];
        if !corner(0, 0).is_finite()
            || !corner(rows - 1, 0).is_finite()
            || !corner(0, cols - 1).is_finite()
            || !corner(rows - 1, cols - 1).is_finite()
        {
            return Err(ConstructRefusal::InvalidInput);
        }
    }
    for k in 0..3 {
        let rows = grid[k].len();
        let cols = grid[k][0].len();
        corners[0][0][k] = grid[k][0][0];
        corners[1][0][k] = grid[k][rows - 1][0];
        corners[0][1][k] = grid[k][0][cols - 1];
        corners[1][1][k] = grid[k][rows - 1][cols - 1];
    }
    Ok(corners)
}

/// The world-space offset corners `S(corner) + t·n̂` of a face stratum.
fn offset_corners(
    corners: &[[[f64; 3]; 2]; 2],
    offset: f64,
    normal: [f64; 3],
) -> [[[f64; 3]; 2]; 2] {
    let mut out = [[[0.0_f64; 3]; 2]; 2];
    for u in 0..2 {
        for v in 0..2 {
            out[u][v] = add3(corners[u][v], scale3(normal, offset));
        }
    }
    out
}

/// Whether a surface coefficient grid represents a FLAT (affine) map: the
/// derived second-derivative coefficient grids along `u`, along `v`, and
/// across `u`/`v` are EXACTLY zero (the CC-002 flatness discipline — rounding
/// slivers are never mistaken for curvature).
fn grid_is_flat(grid: &[Vec<Vec<f64>>; 3]) -> bool {
    grid.iter().all(|coordinate| {
        let du = bernstein_derivative_2d(coordinate, 0);
        let dv = bernstein_derivative_2d(coordinate, 1);
        let duu = bernstein_derivative_2d(&du, 0);
        let duv = bernstein_derivative_2d(&du, 1);
        let dvv = bernstein_derivative_2d(&dv, 1);
        coefficients_zero(&duu) && coefficients_zero(&duv) && coefficients_zero(&dvv)
    })
}

/// Whether every coefficient of a derivative grid is exactly zero.
fn coefficients_zero(grid: &[Vec<f64>]) -> bool {
    grid.iter().all(|row| row.iter().all(|c| *c == 0.0))
}

/// Certify that the star — the ordered strata plus the glue plan — is an
/// embedded closed star, reducing to CC-005's graph-disk machinery.
///
/// The decision table runs in frozen order:
///
/// 1. **structural validation** — the strata list is non-empty, every seam
///    references two distinct in-range strata, and no boundary side of any
///    stratum is glued twice; a violation is
///    [`ConstructRefusal::InvalidInput`];
/// 2. **the glue gate** — for every seam the two boundary references must
///    agree on the shared boundary's identity; a mismatch is
///    [`ConstructRefusal::StarNotEmbedded`] and fires BEFORE the graph-disk /
///    projection search runs;
/// 3. **the graph-disk reduction** — every stratum becomes one admitted piece
///    whose per-piece determinant data (against the searched projection) is
///    built from the stratum's landed regularity certificate times the
///    carrier's area-normal enclosure; the frozen CC-005 candidate sequence is
///    searched and the projected outer rim is discharged through
///    [`projected_boundary_simplicity`]. A star that no frozen projection
///    certifies (a fold, a non-simple rim) is
///    [`ConstructRefusal::StarNotEmbedded`]. Edge and corner strata, and face
///    strata whose carrier cannot supply the piece data (curved or
///    multi-patch maps), refuse
///    [`ConstructRefusal::NoAdmissibleProjection`] — the v1 certified scope.
///
/// The returned [`GraphDiskCert`] carries the per-piece witness records
/// against the internally winning projection.
pub fn certify_star(star: &Star) -> Result<GraphDiskCert, ConstructRefusal> {
    validate_and_gate(star)?;
    let pieces = admitted_pieces(star)?;
    let seams = seam_pairs(star);
    let state = RimState::new(star, &seams)?;
    let mut query = |w: [f64; 3]| state.discharge(w);
    match search_projection(&pieces, &mut query) {
        Ok((_w, disks)) => Ok(GraphDiskCert { pieces: disks }),
        Err(ConstructRefusal::NoAdmissibleProjection) => Err(ConstructRefusal::StarNotEmbedded),
        Err(refusal) => Err(refusal),
    }
}

/// Structural validation plus the pre-graphdisk glue gate.
fn validate_and_gate(star: &Star) -> Result<(), ConstructRefusal> {
    if star.strata.is_empty() {
        return Err(ConstructRefusal::InvalidInput);
    }
    let n = star.strata.len();
    let mut used_sides = vec![false; n * 4];
    for glue in &star.glue_plan.seams {
        if glue.a.stratum >= n || glue.b.stratum >= n {
            return Err(ConstructRefusal::InvalidInput);
        }
        if glue.a.stratum == glue.b.stratum {
            return Err(ConstructRefusal::InvalidInput);
        }
        // The v1 seam vocabulary is the face domain side vocabulary; a seam
        // that names a boundary of a non-face stratum is not expressible here.
        let face_a = matches!(star.strata[glue.a.stratum], OffsetStratum::Face { .. });
        let face_b = matches!(star.strata[glue.b.stratum], OffsetStratum::Face { .. });
        if !face_a || !face_b {
            return Err(ConstructRefusal::InvalidInput);
        }
        for (stratum, side) in [(glue.a.stratum, glue.a.side), (glue.b.stratum, glue.b.side)] {
            let key = stratum * 4 + side.index();
            if used_sides[key] {
                return Err(ConstructRefusal::InvalidInput);
            }
            used_sides[key] = true;
        }
        if glue.a.boundary != glue.b.boundary {
            return Err(ConstructRefusal::StarNotEmbedded);
        }
    }
    Ok(())
}

/// The CC-005 admitted pieces of the star's face strata, in stratum order.
fn admitted_pieces(star: &Star) -> Result<Vec<AdmittedPiece>, ConstructRefusal> {
    let mut pieces = Vec::with_capacity(star.strata.len());
    for stratum in &star.strata {
        let face = face_geometry(stratum)?;
        pieces.push(AdmittedPiece {
            normal_box: face.normal_box,
            area_lower: face.area_lower,
            net_u: face.net_u,
            net_v: face.net_v,
            seam_glued: true,
        });
    }
    Ok(pieces)
}

/// The validated seams of the star as `(stratum_a, side_a, stratum_b,
/// side_b)` pairs, in plan order.
fn seam_pairs(star: &Star) -> Vec<(usize, FaceSide, usize, FaceSide)> {
    star.glue_plan
        .seams
        .iter()
        .map(|glue| (glue.a.stratum, glue.a.side, glue.b.stratum, glue.b.side))
        .collect()
}

/// The certified outer-rim state of a face-stratum star: the offset corner
/// points of every face and the seam endpoint identifications.
struct RimState {
    /// Per face, the offset corners `[u][v]`, in stratum order.
    face_corners: Vec<[[[f64; 3]; 2]; 2]>,
    /// The seam pairs `(stratum_a, side_a, stratum_b, side_b)`.
    seams: Vec<(usize, FaceSide, usize, FaceSide)>,
}

impl RimState {
    /// Build the rim state over the star's face strata (all strata are faces
    /// once the pieces were admitted).
    fn new(
        star: &Star,
        seams: &[(usize, FaceSide, usize, FaceSide)],
    ) -> Result<Self, ConstructRefusal> {
        let mut face_corners = Vec::with_capacity(star.strata.len());
        for stratum in &star.strata {
            let face = face_geometry(stratum)?;
            face_corners.push(face.corners);
        }
        Ok(RimState {
            face_corners,
            seams: seams.to_vec(),
        })
    }

    /// Certify the projected outer rim is simple against the candidate
    /// projection `w`: every non-glued face domain side is a rim arc; the rim
    /// arcs are straight offset segments (the v1 flat-face scope), so each
    /// arc carries the flat near-diagonal radius `+∞` and a planar box built
    /// from its projected endpoints. The CC-005
    /// [`projected_boundary_simplicity`] discharge decides.
    fn discharge(&self, w: [f64; 3]) -> Result<bool, ConstructRefusal> {
        let frame = projection_frame(w)?;
        let projected = project_corners(&self.face_corners, frame)?;
        let mut dsu = Dsu::new(self.face_corners.len() * 4);
        for &(fa, side_a, fb, side_b) in &self.seams {
            for end in 0..2 {
                let (ua, va) = side_a.end_corner(end);
                let (ub, vb) = side_b.end_corner(end);
                dsu.union(corner_key(fa, ua, va), corner_key(fb, ub, vb));
            }
        }
        let mut glued = vec![false; self.face_corners.len() * 4];
        for &(fa, side_a, fb, side_b) in &self.seams {
            glued[fa * 4 + side_a.index()] = true;
            glued[fb * 4 + side_b.index()] = true;
        }
        let mut arcs: Vec<BoundaryArc> = Vec::new();
        for (face, corners) in projected.iter().enumerate() {
            for side in ALL_SIDES {
                if glued[face * 4 + side.index()] {
                    continue;
                }
                let ((us, vs), (ue, ve)) = side.arc_corners();
                let start_root = dsu.find(corner_key(face, us, vs));
                let end_root = dsu.find(corner_key(face, ue, ve));
                let planar_box = segment_box(corners[us][vs], corners[ue][ve])?;
                arcs.push(BoundaryArc {
                    planar_box,
                    radius: FLAT_RADIUS,
                    start: start_root,
                    end: end_root,
                });
            }
        }
        projected_boundary_simplicity(&arcs)
    }
}

/// The four face domain sides in fixed deterministic order.
const ALL_SIDES: [FaceSide; 4] = [
    FaceSide::UMin,
    FaceSide::UMax,
    FaceSide::VMin,
    FaceSide::VMax,
];

/// The certified near-diagonal radius of a straight rim segment: `+∞` (a flat
/// curve has `L = 0`, the CC-002 `δ = 2σ/L = +∞` convention).
const FLAT_RADIUS: Interval = Interval {
    lo: f64::INFINITY,
    hi: f64::INFINITY,
};

/// An orthonormal frame `(e1, e2)` of the projection plane perpendicular to
/// `w`, built deterministically: the reference axis is the standard basis
/// vector least aligned with `w`.
fn projection_frame(w: [f64; 3]) -> Result<[[f64; 3]; 2], ConstructRefusal> {
    let Some(w_hat) = unit(w) else {
        return Err(ConstructRefusal::InvalidInput);
    };
    let mut least = 0usize;
    for axis in 1..3 {
        if w_hat[axis].abs() < w_hat[least].abs() {
            least = axis;
        }
    }
    let mut reference = [0.0_f64; 3];
    reference[least] = 1.0;
    let dot = reference[0] * w_hat[0] + reference[1] * w_hat[1] + reference[2] * w_hat[2];
    let e1_raw = sub3(reference, scale3(w_hat, dot));
    let Some(e1) = unit(e1_raw) else {
        return Err(ConstructRefusal::InvalidInput);
    };
    let e2 = cross3(w_hat, e1);
    if !norm3(e2).is_finite() || norm3(e2) <= 0.0 {
        return Err(ConstructRefusal::InvalidInput);
    }
    Ok([e1, e2])
}

/// Project every face corner into the projection plane, returning per face the
/// `corners[u][v]` plane coordinates.
fn project_corners(
    face_corners: &[[[[f64; 3]; 2]; 2]],
    basis: [[f64; 3]; 2],
) -> Result<Vec<[[[f64; 2]; 2]; 2]>, ConstructRefusal> {
    let mut out = Vec::with_capacity(face_corners.len());
    for face in face_corners {
        let mut projected = [[[0.0_f64; 2]; 2]; 2];
        for u in 0..2 {
            for v in 0..2 {
                let point = face[u][v];
                let a = dot3(point, basis[0]);
                let b = dot3(point, basis[1]);
                if !a.is_finite() || !b.is_finite() {
                    return Err(ConstructRefusal::InvalidInput);
                }
                projected[u][v] = [a, b];
            }
        }
        out.push(projected);
    }
    Ok(out)
}

/// The certified planar box of a straight segment between two projected
/// endpoints: the componentwise endpoint hull, rounded outward.
fn segment_box(a: [f64; 2], b: [f64; 2]) -> Result<[Interval; 2], ConstructRefusal> {
    let mut box2 = [Interval::point(0.0); 2];
    for axis in 0..2 {
        let lo = a[axis].min(b[axis]).next_down();
        let hi = a[axis].max(b[axis]).next_up();
        if !lo.is_finite() || !hi.is_finite() || lo > hi {
            return Err(ConstructRefusal::InvalidInput);
        }
        box2[axis] = Interval { lo, hi };
    }
    Ok(box2)
}

/// The corner key of `(face, u_corner, v_corner)` in the rim union-find.
fn corner_key(face: usize, u: usize, v: usize) -> usize {
    face * 4 + u * 2 + v
}

/// A tiny deterministic union-find over corner keys.
struct Dsu {
    parent: Vec<usize>,
}

impl Dsu {
    /// A fresh union-find over `n` singleton keys.
    fn new(n: usize) -> Self {
        let parent: Vec<usize> = (0..n).collect();
        Dsu { parent }
    }

    /// The canonical root of `x` (path halving).
    fn find(&mut self, x: usize) -> usize {
        let mut root = x;
        while self.parent[root] != root {
            root = self.parent[root];
        }
        let mut cursor = x;
        while self.parent[cursor] != cursor {
            let next = self.parent[cursor];
            self.parent[cursor] = root;
            cursor = next;
        }
        root
    }

    /// Unify two keys under the canonical root of the first.
    fn union(&mut self, a: usize, b: usize) {
        let ra = self.find(a);
        let rb = self.find(b);
        if ra != rb {
            self.parent[rb] = ra;
        }
    }
}

/// The certified broad phase over the constructed strata (theory §4.3).
///
/// For every unordered pair `(i, j)`, `i < j`, the pair is retained as a
/// candidate unless the CC-004 axis-gap lower bound `d` between the strata's
/// bounding boxes certifies the realizations disjoint: `d > ρ_i + ρ_j` with
/// `ρ` the certified per-stratum reach bound
/// ([`OffsetStratum::reach_bound`](crate::construct::offset_strata::OffsetStratum::reach_bound),
/// anchor A3). The bounding box of a stratum is the certified enclosure of
/// its source carrier (the face's map over its domain, the edge spine over
/// its arc, the corner's centre enclosure); every stratum realization point
/// lies within the certified reach of that box, so the axis-gap disjointness
/// discharges.
///
/// The returned candidate pairs are sorted ascending and deduplicated. **This
/// prune is SOUND but not complete**: a retained pair is not certified to
/// contact — retained pairs go to the CC-023 contact funnel.
pub fn reach_prune(strata: &[OffsetStratum]) -> Vec<(usize, usize)> {
    let data: Vec<(f64, Option<[Interval; 3]>)> = strata
        .iter()
        .map(|stratum| (stratum.reach_bound(), stratum_source_box(stratum)))
        .collect();
    let mut out = Vec::new();
    for i in 0..data.len() {
        for j in (i + 1)..data.len() {
            let (rho_i, box_i) = data[i];
            let (rho_j, box_j) = data[j];
            let (Some(bi), Some(bj)) = (box_i, box_j) else {
                out.push((i, j));
                continue;
            };
            let distance = box_distance_lower_bound(&bi, &bj);
            let certifiably_disjoint = distance.is_finite() && distance > rho_i + rho_j;
            if !certifiably_disjoint {
                out.push((i, j));
            }
        }
    }
    out
}

/// The certified source bounding box of a stratum, or `None` when the carrier
/// cannot certify an enclosure (the pair is then conservatively retained).
fn stratum_source_box(stratum: &OffsetStratum) -> Option<[Interval; 3]> {
    match stratum {
        OffsetStratum::Face { map, .. } => {
            let domain = surface_domain(map)?;
            map.enclosure(domain).ok()
        }
        OffsetStratum::Edge { spine, .. } => {
            let intervals = spine.piece_intervals();
            let first = *intervals.first()?;
            let last = *intervals.last()?;
            spine.enclosure((first.0, last.1)).ok()
        }
        OffsetStratum::Corner { node } => Some(node.centre),
    }
}

/// The declared domain rectangle of a surface map, from its patch table;
/// `None` when the map carries no patch.
fn surface_domain(map: &CertifiedSurfaceMap) -> Option<crate::certified_map::SurfaceRegion> {
    let boxes = map.patch_boxes();
    let first = *boxes.first()?;
    let last = *boxes.last()?;
    Some(((first.0 .0, last.0 .1), (first.1 .0, last.1 .1)))
}

/// A certified LOWER bound of the minimum distance between two boxes: the
/// Euclidean norm of the per-axis gap vector, every square, sum and root
/// rounded downward.
fn box_distance_lower_bound(a: &[Interval; 3], b: &[Interval; 3]) -> f64 {
    let mut sq = 0.0_f64;
    for k in 0..3 {
        let gap = axis_gap(&a[k], &b[k]);
        if !gap.is_finite() {
            return f64::NEG_INFINITY;
        }
        let term = (gap * gap).next_down();
        sq = (sq + term).next_down();
    }
    sq.sqrt().next_down()
}

/// The exact gap between two closed intervals on one axis (`0` when they
/// touch or overlap).
fn axis_gap(a: &Interval, b: &Interval) -> f64 {
    if a.hi < b.lo {
        b.lo - a.hi
    } else if b.hi < a.lo {
        a.lo - b.hi
    } else {
        0.0
    }
}

/// Subtract two 3-vectors.
fn sub3(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

/// Add two 3-vectors.
fn add3(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

/// Scale a 3-vector.
fn scale3(a: [f64; 3], s: f64) -> [f64; 3] {
    [a[0] * s, a[1] * s, a[2] * s]
}

/// The cross product of two 3-vectors.
fn cross3(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// The dot product of two 3-vectors.
fn dot3(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// The Euclidean norm of a 3-vector.
fn norm3(a: [f64; 3]) -> f64 {
    dot3(a, a).sqrt()
}

/// Unit-normalize a direction; `None` for a degenerate (zero, non-finite or
/// NaN) direction.
fn unit(v: [f64; 3]) -> Option<[f64; 3]> {
    let sq = dot3(v, v);
    if !sq.is_finite() || sq <= 0.0 {
        return None;
    }
    let length = sq.sqrt();
    if !length.is_finite() || length <= 0.0 {
        return None;
    }
    Some(scale3(v, 1.0 / length))
}
