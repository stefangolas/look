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

//! CC-023-SHELL-BRIDGE (CC program Phase C, spine S7 consumer; theory §4.2–4.4):
//! the S1 embedding certificate on the stratum quotient and the S1′ solid
//! corollary.
//!
//! The bridge theorem: with (1) the stratum complex a compact
//! 2-manifold-with-corners, (2) exactly consistent realizations on identified
//! strata (P6), and (3) injectivity on the quotient, `F_t` is a topological
//! embedding. Hypothesis (3) is discharged in three regimes — P2 near-diagonal,
//! P3 on stars, the evidence contact funnel elsewhere. S1′ adds the
//! Jordan–Brouwer solid corollary for closed connected orientable complexes.
//!
//! **The certificate (Section 1).** [`certify_shell`] composes the landed
//! certificates in the theory §4.4 order, over a FACE-stratum shell complex:
//!
//! 1. **stars through [`certify_star`]** (anchor A1): every glue seam's two
//!    incident faces form the closed star of the seam, certified embedded by
//!    CC-022's graph-disk reduction. [`ShellCert::stars_certified`] counts the
//!    stars that certified; a seam whose star refuses (a fold, a non-simple
//!    rim) merely lowers the count — the certificate is still produced and its
//!    validity is read off it.
//! 2. **the broad phase via [`reach_prune`]** (anchor A2): the certified
//!    reach-bound prune over the constructed strata (sound, not complete).
//! 3. **retained pairs through the evidence contact funnel** (anchor A3, the
//!    landed `truck_evidence::contact::contact`) across the C2 manifest edge.
//!    The v1 certified scope funnels the pairs whose two carriers are FLAT
//!    single-patch faces: the realized offset plane (the source affine plane
//!    carried out by the signed stratum offset) is presented to the funnel as a
//!    canonical [`Plane`] patch — no interval crosses the edge, so the C3
//!    interval bridge ([`super::convert`]) is untouched. A pair that cannot
//!    present canonical carriers (a curved or multi-patch face) is
//!    [`ShellPairVerdict::Inconclusive`], never certified. The per-pair verdict
//!    is the CC-014 three-valued mapping table, reused verbatim: the funnel
//!    certifies the pair contact-free → [`ShellPairVerdict::Certified`]; the
//!    funnel certifies a contact → [`ShellPairVerdict::Contact`] (the caller
//!    refuses `UnintendedContact`); a funnel refusal of the
//!    `NumericallyUnresolved` family, an `UnsupportedEnvelope` deferral, or any
//!    budget exhaustion → [`ShellPairVerdict::Inconclusive`], never `Certified`.
//!
//! **S1′ (Section 2).** [`SolidOutcome`] runs the three pre-made checks in
//! order over the certified-embedded complex: closed (every face side is
//! identified pairwise by exactly one seam; an unglued sheet is `Open`),
//! connected under the glue plan (`Disconnected`), and orientation — the
//! provenance-first consistency of the induced global nesting, read off the
//! seam parity of the face maps' boundary walks (never an independent
//! re-determination of the material side); an unresolvable or inconsistent
//! nesting is `OrientationUnresolved`. All three pass → `Solid`. A shell that
//! certifies embedding but fails S1′ is a certified SURFACE, not a solid — the
//! type makes that distinction unrepresentable to ignore. The corollary is
//! evaluated when the funnel certifies the complex free of unintended contact
//! — every non-seam pair verdict is `Certified`; otherwise [`ShellCert::solid`]
//! is `None`. The seam-star count is a SEPARATE witness: CC-022's projected-rim
//! discharge is label-sensitive and may refuse a genuinely embedded convex
//! seam, so [`ShellCert::stars_certified`] does not gate the corollary.
//!
//! **Certified scope (v1).** The shell complex is a FACE complex — every
//! stratum is a k=1 [`OffsetStratum::Face`] — because the glue vocabulary is
//! the face-side vocabulary (P6) and CC-022's stars certify face strata only.
//! A request carrying an edge or corner stratum is refused
//! [`ConstructRefusal::InvalidInput`]: the k=2/k=3 realization machinery and
//! its ball-vs-excluded-boundary funnel (the P5 `BallAdmissibility`
//! clearance predicate of `clear.rs`, anchor A4) are a later packet's system,
//! and the seam stars certify flat single-patch faces only.
//!
//! **H-1.** This module carries no `unwrap`, no `expect`, and no `panic!`, and
//! adds no module-level `allow`. All reductions run in fixed order.

use crate::construct::offset_strata::OffsetStratum;
use crate::construct::refusal::ConstructRefusal;
use crate::construct::stars::{certify_star, reach_prune, FaceSide, GluePlan, Star};
use crate::hull::bernstein_derivative_2d;
use truck_base::evidence::Budget;
use truck_geometry::prelude::Point3;
use truck_geometry::recognize::CanonicalSurface;
use truck_geometry::specifieds::Plane;

/// The three-valued per-pair verdict of the shell certificate (Section 1).
///
/// One verdict per non-seam unordered pair of strata, in ascending index
/// order. The three-valued shape is the CC-014 CG verdict doctrine: the funnel
/// certifies the pair contact-free, the funnel certifies an (unintended)
/// contact, or the pair cannot be decided.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellPairVerdict {
    /// The pair is certified free of unintended contact (by the reach-bound
    /// broad phase or by the evidence contact funnel).
    Certified,
    /// The pair's realizations certify an unintended contact (the caller
    /// refuses `ConstructRefusal::UnintendedContact`).
    Contact,
    /// The pair cannot be certified either way (a funnel refusal of the
    /// `NumericallyUnresolved` family, an unsupported funnel envelope, or
    /// budget exhaustion). Never a `Certified`.
    Inconclusive,
}

/// The S1′ solid-corollary outcome (Section 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolidOutcome {
    /// The closed connected orientable complex certifies a solid (the
    /// Jordan–Brouwer corollary).
    Solid,
    /// The complex certifies an embedded surface but fails S1′.
    SurfaceOnly {
        /// Why the surface is not certified a solid.
        reason: SurfaceOnlyReason,
    },
}

/// Why a certified-embedded shell is a surface, not a solid (Section 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceOnlyReason {
    /// The complex is open: some face side is not identified pairwise by a
    /// seam.
    Open,
    /// The complex is not connected under the glue plan.
    Disconnected,
    /// The provenance-first orientation nesting is inconsistent or
    /// unresolvable.
    OrientationUnresolved,
}

/// The shell certificate: the per-pair verdicts, the certified-star count, and
/// the S1′ solid outcome.
///
/// `Ok` from [`certify_shell`] always means the certificate was produced;
/// validity is read off it.
#[derive(Debug, Clone)]
pub struct ShellCert {
    /// One verdict per non-seam unordered pair of strata, ascending.
    pub pairs: Vec<ShellPairVerdict>,
    /// The number of glue seams whose closed star certified embedded.
    pub stars_certified: usize,
    /// The S1′ solid corollary, evaluated when every pair is certified free of
    /// unintended contact; `None` otherwise.
    pub solid: Option<SolidOutcome>,
}

/// Certify the shell over a FACE-stratum complex: the S1 embedding certificate
/// on the quotient plus the S1′ solid corollary.
///
/// `strata` are the k=1 face strata of the shell boundary complex and `glue`
/// records which of their boundaries are identified (P6, exact and
/// identity-based). The pipeline runs in the theory §4.4 order: the seam
/// stars through [`certify_star`], the broad phase through [`reach_prune`],
/// and each retained non-seam pair through the evidence contact funnel.
///
/// The certificate is refused only on invalid input — an empty complex, a
/// non-face stratum, a structurally corrupt glue plan — or on a seam whose
/// two sides disagree on the shared boundary's identity
/// ([`ConstructRefusal::StarNotEmbedded`], the CC-022 glue gate). Everything
/// else is `Ok`; the per-pair verdicts, the star count, and the solid outcome
/// carry the validity.
pub fn certify_shell(
    strata: Vec<OffsetStratum>,
    glue: &GluePlan,
    budget: &mut Budget,
) -> Result<ShellCert, ConstructRefusal> {
    if strata.is_empty() {
        return Err(ConstructRefusal::InvalidInput);
    }
    for stratum in &strata {
        if !matches!(stratum, OffsetStratum::Face { .. }) {
            return Err(ConstructRefusal::InvalidInput);
        }
    }
    let n = strata.len();
    validate_glue(&strata, glue)?;

    // Step 1: the closed star of every glue seam, through CC-022's certify_star.
    let mut stars_certified = 0usize;
    for seam in &glue.seams {
        let a = seam.a.stratum;
        let b = seam.b.stratum;
        // The star is a two-stratum object: remap the seam's stratum indices
        // onto the star's own list before certify_star reads them.
        let mut star_seam = *seam;
        star_seam.a.stratum = 0;
        star_seam.b.stratum = 1;
        let star = Star {
            strata: vec![strata[a].clone(), strata[b].clone()],
            glue_plan: GluePlan {
                seams: vec![star_seam],
            },
        };
        if certify_star(&star).is_ok() {
            stars_certified += 1;
        }
    }

    // Step 2: the certified reach-bound broad phase (sound, not complete).
    let mut retained = vec![false; n * n];
    for &(i, j) in &reach_prune(&strata) {
        retained[i * n + j] = true;
    }
    let mut seam_pair = vec![false; n * n];
    for seam in &glue.seams {
        let (i, j) = (seam.a.stratum, seam.b.stratum);
        seam_pair[i * n + j] = true;
        seam_pair[j * n + i] = true;
    }

    // Step 3: one verdict per non-seam pair — certified disjoint by the prune,
    // or funnelled through the evidence contact funnel.
    let mut pairs = Vec::new();
    for i in 0..n {
        for j in (i + 1)..n {
            if seam_pair[i * n + j] {
                continue;
            }
            let verdict = if !retained[i * n + j] {
                ShellPairVerdict::Certified
            } else {
                classify_retained_pair(&strata[i], &strata[j], budget)
            };
            pairs.push(verdict);
        }
    }

    // S1′: evaluated when the funnel certifies the complex free of unintended
    // contact — every non-seam pair is Certified. (The seam stars are a
    // separate witness: CC-022's rim discharge is label-sensitive and may
    // refuse a genuinely embedded convex seam, so the star count does not gate
    // the corollary.)
    let embedded = pairs.iter().all(|v| *v == ShellPairVerdict::Certified);
    let solid = if embedded {
        Some(evaluate_solid_outcome(&strata, glue))
    } else {
        None
    };

    Ok(ShellCert {
        pairs,
        stars_certified,
        solid,
    })
}

/// Structural validation of the glue plan over a face complex, plus the CC-022
/// glue gate.
///
/// Refuses [`ConstructRefusal::InvalidInput`] on an out-of-range or self seam,
/// a seam whose side names a non-face stratum (in the v1 vocabulary every
/// stratum is a face and the glue references face-domain sides), or a boundary
/// side glued twice; refuses [`ConstructRefusal::StarNotEmbedded`] when a
/// seam's two sides disagree on the shared boundary's identity.
fn validate_glue(strata: &[OffsetStratum], glue: &GluePlan) -> Result<(), ConstructRefusal> {
    let n = strata.len();
    let mut used = vec![false; n * 4];
    for seam in &glue.seams {
        if seam.a.stratum >= n || seam.b.stratum >= n {
            return Err(ConstructRefusal::InvalidInput);
        }
        if seam.a.stratum == seam.b.stratum {
            return Err(ConstructRefusal::InvalidInput);
        }
        for (stratum, side) in [(seam.a.stratum, seam.a.side), (seam.b.stratum, seam.b.side)] {
            let key = stratum * 4 + side_index(side);
            if used[key] {
                return Err(ConstructRefusal::InvalidInput);
            }
            used[key] = true;
        }
        if seam.a.boundary != seam.b.boundary {
            return Err(ConstructRefusal::StarNotEmbedded);
        }
    }
    Ok(())
}

/// The verdict of one retained non-seam pair, through the evidence contact
/// funnel (anchor A3).
///
/// Budget exhaustion → [`ShellPairVerdict::Inconclusive`], never `Certified`.
/// Both carriers must present flat canonical planes; otherwise the funnel
/// cannot run and the pair is [`ShellPairVerdict::Inconclusive`]. The funnel's
/// `Ok` with an empty contact complex is a certified no-contact pair; `Ok`
/// with contacts is a certified (unintended) contact; every funnel refusal
/// (the `NumericallyUnresolved` family, an `UnsupportedEnvelope` deferral,
/// anything else) is [`ShellPairVerdict::Inconclusive`].
fn classify_retained_pair(
    a: &OffsetStratum,
    b: &OffsetStratum,
    budget: &mut Budget,
) -> ShellPairVerdict {
    if budget.subdiv == 0 && budget.newton == 0 && budget.depth == 0 {
        return ShellPairVerdict::Inconclusive;
    }
    let (Some(plane_a), Some(plane_b)) = (realized_plane(a), realized_plane(b)) else {
        return ShellPairVerdict::Inconclusive;
    };
    let lhs = bounded_face(&plane_a);
    let rhs = bounded_face(&plane_b);
    match truck_evidence::contact::contact(&lhs, &rhs, budget) {
        Ok(certified) => {
            if certified.value.contacts.is_empty() {
                ShellPairVerdict::Certified
            } else {
                ShellPairVerdict::Contact
            }
        }
        Err(_) => ShellPairVerdict::Inconclusive,
    }
}

/// The realized offset plane of a flat single-patch face stratum, presented in
/// the evidence funnel's canonical vocabulary.
///
/// A face stratum whose carrier is NOT a flat (affine) single-patch map cannot
/// present a canonical plane here — the v1 certified funnel scope — and
/// returns `None` (the pair is then [`ShellPairVerdict::Inconclusive`], never
/// certified).
struct RealizedPlane {
    /// The canonical plane of the realized face: the source affine plane
    /// carried out by the signed stratum offset.
    plane: Plane,
    /// The face's `u`-parameter box on the plane.
    u_range: (f64, f64),
    /// The face's `v`-parameter box on the plane.
    v_range: (f64, f64),
}

/// A bounded face stratum over a realized plane, in the funnel's vocabulary.
fn bounded_face(realized: &RealizedPlane) -> truck_evidence::contact::BoundedStratum {
    truck_evidence::contact::BoundedStratum::Face {
        surface: CanonicalSurface::Plane(realized.plane),
        u_range: realized.u_range,
        v_range: realized.v_range,
    }
}

/// Reconstruct the realized offset plane of a flat single-patch face stratum.
///
/// The source affine plane is read off the single Bézier patch's corner
/// lattice; every corner is then carried along the certified unit area normal
/// by the signed offset. `None` when the stratum is not a face, is not a
/// single-patch flat carrier, or carries non-finite corner data.
fn realized_plane(stratum: &OffsetStratum) -> Option<RealizedPlane> {
    let (map, offset) = match stratum {
        OffsetStratum::Face { map, offset, .. } => (map, *offset),
        _ => return None,
    };
    if !offset.is_finite() {
        return None;
    }
    let boxes = map.patch_boxes();
    let grids = map.patch_grids();
    if boxes.len() != 1 || grids.len() != 1 {
        return None;
    }
    let grid = &grids[0];
    if !grid_is_flat(grid) {
        return None;
    }
    let corners = source_corners(grid)?;
    let leg_u = sub3(corners[1][0], corners[0][0]);
    let leg_v = sub3(corners[0][1], corners[0][0]);
    let normal = unit(cross3(leg_u, leg_v))?;
    let origin = add3(corners[0][0], scale3(normal, offset));
    let one = add3(corners[1][0], scale3(normal, offset));
    let another = add3(corners[0][1], scale3(normal, offset));
    Some(RealizedPlane {
        plane: Plane::new(p3(origin), p3(one), p3(another)),
        u_range: (0.0, 1.0),
        v_range: (0.0, 1.0),
    })
}

/// The S1′ solid corollary over the certified-embedded face complex.
///
/// The three pre-made checks run in order: closed (`Open`), connected under
/// the glue plan (`Disconnected`), and the provenance-first orientation
/// consistency (`OrientationUnresolved`). All three pass → `Solid`.
fn evaluate_solid_outcome(strata: &[OffsetStratum], glue: &GluePlan) -> SolidOutcome {
    if !closed_complex(strata, glue) {
        return SolidOutcome::SurfaceOnly {
            reason: SurfaceOnlyReason::Open,
        };
    }
    if !connected_complex(strata, glue) {
        return SolidOutcome::SurfaceOnly {
            reason: SurfaceOnlyReason::Disconnected,
        };
    }
    if !orientation_consistent(strata, glue) {
        return SolidOutcome::SurfaceOnly {
            reason: SurfaceOnlyReason::OrientationUnresolved,
        };
    }
    SolidOutcome::Solid
}

/// Whether every face side of the complex is identified by exactly one seam
/// (a closed complex; an unglued sheet is open).
fn closed_complex(strata: &[OffsetStratum], glue: &GluePlan) -> bool {
    let mut used = vec![false; strata.len() * 4];
    for seam in &glue.seams {
        used[seam.a.stratum * 4 + side_index(seam.a.side)] = true;
        used[seam.b.stratum * 4 + side_index(seam.b.side)] = true;
    }
    used.iter().all(|is_used| *is_used)
}

/// Whether the strata are connected under the glue plan.
fn connected_complex(strata: &[OffsetStratum], glue: &GluePlan) -> bool {
    let n = strata.len();
    let mut parent: Vec<usize> = (0..n).collect();
    for seam in &glue.seams {
        union(&mut parent, seam.a.stratum, seam.b.stratum);
    }
    let root = find(&mut parent, 0);
    (0..n).all(|i| find(&mut parent, i) == root)
}

/// The provenance-first orientation-consistency check: the declared face-map
/// orientations (the strata's own parametrizations, hence their offset sides)
/// induce a consistent global nesting exactly when every seam is traversed in
/// OPPOSITE directions by the two incident faces' boundary walks.
///
/// The check never re-determines a material side — it reads the consistency of
/// the caller's declared orientations off the seam parity. Any seam whose two
/// boundary walks run the same physical direction (a fold, a non-orientable
/// nesting), or whose identified corners do not coincide pairwise (a P6
/// inconsistency), is an unresolvable nesting.
fn orientation_consistent(strata: &[OffsetStratum], glue: &GluePlan) -> bool {
    for seam in &glue.seams {
        let Some(corners_a) = source_corners_of(&strata[seam.a.stratum]) else {
            return false;
        };
        let Some(corners_b) = source_corners_of(&strata[seam.b.stratum]) else {
            return false;
        };
        let ((ua0, va0), (ua1, va1)) = side_arc(seam.a.side);
        let ((ub0, vb0), (ub1, vb1)) = side_arc(seam.b.side);
        let start_a = corners_a[ua0][va0];
        let end_a = corners_a[ua1][va1];
        let start_b = corners_b[ub0][vb0];
        let end_b = corners_b[ub1][vb1];
        let opposite = point_eq(start_a, end_b) && point_eq(end_a, start_b);
        let same = point_eq(start_a, start_b) && point_eq(end_a, end_b);
        if !opposite || same {
            return false;
        }
    }
    true
}

/// The source corner lattice of a face stratum's map, `corners[u][v]` in world
/// coordinates; `None` when the map has no single patch lattice to read.
fn source_corners_of(stratum: &OffsetStratum) -> Option<[[[f64; 3]; 2]; 2]> {
    match stratum {
        OffsetStratum::Face { map, .. } => {
            let grids = map.patch_grids();
            if grids.len() != 1 {
                return None;
            }
            source_corners(&grids[0])
        }
        _ => None,
    }
}

/// The four source corners of a single-patch surface map, `corners[u][v]`,
/// read off the coefficient-grid corners (row = u, column = v).
fn source_corners(grid: &[Vec<Vec<f64>>; 3]) -> Option<[[[f64; 3]; 2]; 2]> {
    let mut corners = [[[0.0_f64; 3]; 2]; 2];
    for coordinate in grid {
        let rows = coordinate.len();
        if rows == 0 {
            return None;
        }
        let cols = coordinate[0].len();
        if cols == 0 {
            return None;
        }
        if !coordinate[0][0].is_finite()
            || !coordinate[rows - 1][0].is_finite()
            || !coordinate[0][cols - 1].is_finite()
            || !coordinate[rows - 1][cols - 1].is_finite()
        {
            return None;
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
    Some(corners)
}

/// Whether a surface coefficient grid represents a FLAT (affine) map: the
/// derived second-derivative coefficient grids are EXACTLY zero (the CC-002
/// flatness discipline — rounding slivers are never mistaken for curvature).
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

/// The canonical index of a face-domain side (`UMin = 0, UMax = 1, VMin = 2,
/// VMax = 3`), used for deterministic bookkeeping only.
fn side_index(side: FaceSide) -> usize {
    match side {
        FaceSide::UMin => 0,
        FaceSide::UMax => 1,
        FaceSide::VMin => 2,
        FaceSide::VMax => 3,
    }
}

/// The `(start, end)` corner pair a side is traversed along when the face
/// boundary is walked counter-clockwise in `(u, v)`.
fn side_arc(side: FaceSide) -> ((usize, usize), (usize, usize)) {
    match side {
        FaceSide::VMin => ((0, 0), (1, 0)),
        FaceSide::UMax => ((1, 0), (1, 1)),
        FaceSide::VMax => ((1, 1), (0, 1)),
        FaceSide::UMin => ((0, 1), (0, 0)),
    }
}

/// Whether two world points are the SAME point exactly: the identified corners
/// of a shared boundary feature carry the SAME coordinates (P6 identity — this
/// is an exact equality on shared data, never a tolerance).
fn point_eq(a: [f64; 3], b: [f64; 3]) -> bool {
    a[0] == b[0] && a[1] == b[1] && a[2] == b[2]
}

/// The canonical root of `x` (path halving).
fn find(parent: &mut [usize], x: usize) -> usize {
    let mut root = x;
    while parent[root] != root {
        root = parent[root];
    }
    let mut cursor = x;
    while parent[cursor] != cursor {
        let next = parent[cursor];
        parent[cursor] = root;
        cursor = next;
    }
    root
}

/// Unify two stratum indices under the canonical root of the first.
fn union(parent: &mut [usize], a: usize, b: usize) {
    let ra = find(parent, a);
    let rb = find(parent, b);
    if ra != rb {
        parent[rb] = ra;
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

/// Wrap an `[f64; 3]` as a geometry `Point3`.
fn p3(v: [f64; 3]) -> Point3 {
    Point3::new(v[0], v[1], v[2])
}

/// Unit-normalize a direction; `None` for a degenerate direction.
fn unit(v: [f64; 3]) -> Option<[f64; 3]> {
    let sq = v[0] * v[0] + v[1] * v[1] + v[2] * v[2];
    if !sq.is_finite() || sq <= 0.0 {
        return None;
    }
    let length = sq.sqrt();
    if !length.is_finite() || length <= 0.0 {
        return None;
    }
    Some(scale3(v, 1.0 / length))
}
