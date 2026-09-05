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

//! CC-033-SETBACK (CC program Phase D, spine S6/S3 consumers; theory §5.5):
//! the certified **n-valent corner setback patch**.
//!
//! For a GENUINE n-valent corner the rolling-ball system has no degrees of
//! freedom left (theory §3.3): the answer is a setback vertex blend whose
//! corner region is naturally 2n-sided — the boundary alternates `n` PROFILE
//! arcs `P_i` (the setback cuts across the incoming fillet flanks) and `n`
//! SPRING arcs `Q_i` (the surviving primary faces between the fillets). This
//! module builds that region deterministically (the UNTRUSTED construction)
//! and then certifies it on FOUR counts: boundary equality, G¹ ribbons, local
//! regularity, and global embeddedness.
//!
//! # The v1 data class (closed; the boundary data classes are NOT amended)
//!
//! The [`SetbackInput`] is the 2n boundary arcs with their exact tangent-plane
//! data, as delivered by the CC-030/031 blend traces (the `BlendTrace` of seam
//! S12) and the incident offset strata. Each arc is a polynomial cubic Bézier
//! `C(u)` lying in an **affine shell surface** (the CC-020/030 affine-support
//! class: the tangent plane is constant over the arc, unit normal `n̂`), and
//! carries the delivered **cross-tangent field**
//! `cross(u) = P_v(u,0) = Σ_j cross_j · B_j(u)` — the exact tangent-plane data
//! of how the shell leaves the boundary toward the setback region. Over the
//! closed v1 class:
//!
//! 1. the arcs form a closed chain and alternate `Profile`/`Spring`;
//! 2. every curve control point and every cross coefficient lies in the arc's
//!    tangent plane;
//! 3. at every shared corner the cross fields of the two incident arcs agree
//!    EXACTLY: `cross_{i-1}(1) = cross_i(0)`, and the corner positions agree
//!    exactly (the corner-closure identity of the setback split below).
//!
//! Genuine **n = 3** corners do NOT route through this module: a genuine
//! triple node is solved once by CC-020's `solve_triple_node` (seam S11) and
//! the three incident branches consume it there. Setback fills n ≥ 4 corners
//! (and degenerate triples that no triple node serves). [`SetbackInput::try_new`]
//! refuses fewer than `2 · SETBACK_MIN_VALENCE` arcs; the n = 3 routing is
//! asserted in the integration test comment and kept OUT of this module by
//! construction.
//!
//! # The setback split (deterministic; STOP CONDITION 1)
//!
//! Where the fill crosses each incoming fillet is NOT heuristic here. The rule
//! (recorded verbatim so it is an implementation choice, never a spec
//! amendment) is the **corner-advance rule**: each rim corner `V_i` advances
//! into the region along the delivered cross-tangent of the two incident arcs
//! by exactly the delivered cross step, giving the hub vertex
//!
//! ```text
//! H_i = V_i + cross_{i-1}(1)    (= V_i + cross_i(0) by the closure identity).
//! ```
//!
//! The hub polygon `H_0 .. H_{2n-1}` is the inner rail polygon: ribbon `i`
//! spans from the outer arc `P_i`/`Q_i` (at `v = 0`) to the straight rail edge
//! `H_i H_{i+1}` (at `v = 1`) and its two straight seam spokes
//! `V_i H_i`, `V_{i+1} H_{i+1}`; the rail polygon is capped by `n` bilinear
//! hub quads over consecutive rail-edge pairs. Every piece is a tensor bicubic
//! control net determined in closed form from the delivered data:
//!
//! ```text
//! row0 = C,        row1 = C + cross/3,
//! row3 = rail edge, row2 = (row1 + row3) / 2   (the mean-arrival rule:
//!           P_v(u,1) continues the profile toward the hub cap; no free factor).
//! ```
//!
//! With the closure identity the two spokes of a ribbon are straight (each
//! boundary column of the net is collinear at `j/3`), so neighbouring ribbons
//! and the hub quads glue C⁰ along their shared straight seams exactly, and
//! the shell's cross-tangent `P_v(u,0)` is reproduced identically at the
//! coefficient level.
//!
//! # The certified solve (Section 1; the construction never guesses)
//!
//! The delivered cross data is treated as UNTRUSTED until the ribbon's
//! interpolation system certifies it. The **ribbon interpolation system** is
//! the Bernstein-node sampling map `s = M · x` of a degree-3 profile field
//! (the matrix `M_{jk} = B_k³(t_j)` at the Bernstein nodes `t_j = j/3`). It is
//! DENSE (NOT banded-TP), so [`residual_solve_dense`] (seam S3, A2) is the
//! certified solve recovering the certified coefficients and their L2
//! enclosure `ε`; where a system IS banded-TP-shaped (a station-collocation
//! system with compact basis support) the `factor_banded_tp` /
//! `solve_homogeneous` fast path (A1) applies. The dispatch lives in
//! [`certified_profile_solve`]. An unsolvable ribbon system (a `η ≥ 1` residual
//! bound, a singular factor) REFUSES — never a guessed continuation.
//!
//! # The four certification counts (Section 2; all PRE-MADE)
//!
//! 1. **Boundary** — [`certify_boundary`]: every ribbon's outer patch boundary
//!    `P(u,0)` equals its prescribed `P_i`/`Q_i` up to the delivered enclosure
//!    `ε`.
//! 2. **G¹ ribbons** — [`certify_g1_ribbons`]: on each boundary
//!    `P_v(u,0) = λ(u)·d(u)` with the certified `λ` enclosure STRICTLY positive
//!    and the direction `d` in the adjacent tangent plane. Fold-back
//!    prevention is PART of the certificate (a `λ.lo ≤ 0` enclosure refuses),
//!    not a separate check.
//! 3. **Local regularity** — [`certify_regularity`]: the certified lower bound
//!    of `inf ‖P_u × P_v‖` over the whole patch (every piece) via the CC-002
//!    Bernstein hull path on the pieces' Bézier form is `≥ CC_ETA_J`.
//! 4. **Global embeddedness** — [`certify_embeddedness`]:
//!    [`certify_graph_disk`] (A3, seam S6) over the whole 2n-sided region with
//!    the normative projection search ([`search_projection`] over the frozen
//!    seam-S6 candidate family). Exhaustion of the projection search
//!    (`NoAdmissibleProjection`) falls back to the PAIRWISE discharge
//!    ([`pairwise_embeddedness`]: manifest-edge seam coincidence, boundary
//!    intersection exclusion by certified box separation plus the edge-plane
//!    inside/outside and shared-vertex witnesses, per-piece regularity) and
//!    still certifies or refuses `NoAdmissibleProjection`.
//!
//! # Scope guards (stop conditions)
//!
//! (1) The setback split is the corner-advance rule above — deterministic and
//! recorded; a heuristic setback distance would be a spec amendment. (2) n = 3
//! with a genuine triple node routes through CC-020's node; setback serves
//! n ≥ 4 (asserted in the test comment at minimum). (3) The Hermite ribbons
//! ARE polynomial-solvable for the closed v1 boundary-data classes above; the
//! v1 embeddedness count is additionally scoped to CHORD arcs (a non-straight
//! arc refuses [`certify_embeddedness`] with `InvalidInput`), which is
//! recorded here so no `QUESTION.md` is needed.
//!
//! **H-1.** This module carries no `unwrap`, no `expect`, and no `panic!`, and
//! adds no module-level `allow`. Every float reduction runs in a fixed order
//! with directed rounding (C9), and every [`Interval`] below is the C3
//! universe (`construct::Interval`), never a second interval type.

use crate::construct::banded::factor_banded_tp;
use crate::construct::config::CC_ETA_J;
use crate::construct::graphdisk::{
    certify_graph_disk, search_projection, AdmittedPiece, GraphDiskCert,
};
use crate::construct::refusal::ConstructRefusal;
use crate::construct::residual_solve::residual_solve_dense;
use crate::construct::stubs::BoundaryPlan;
use crate::construct::Interval;
use crate::hull::bernstein_derivative_2d;

/// The minimum valence served by the setback construction: `n ≥ 4` arcs-pairs
/// (2n arcs). A genuine n = 3 corner routes through CC-020's triple node, not
/// this module.
pub const SETBACK_MIN_VALENCE: usize = 4;

/// The tolerance of the in-plane clause of the G¹ count: the certified plane
/// gap of `P_v(u,0)` against the delivered tangent-plane normal must lie
/// within this band. The v1 data class is exact up to `f64` noise; the
/// tolerance absorbs the outward rounding of the hull path, never a real
/// tangent-plane error (those refuse).
pub const G1_PLANE_TOL: f64 = 1e-6;

/// The structural half-bandwidth at or below which a ribbon interpolation
/// system takes the banded-TP fast path; a wider system takes the dense
/// residual solve.
pub const RIBBON_TP_MAX_BAND: usize = 1;

/// The tolerance of the plane-membership validation of an input arc's data
/// (curve and cross coefficients against the delivered unit normal).
pub const DATA_PLANE_TOL: f64 = 1e-9;

/// The tolerance of the seam witnesses of the pairwise embeddedness fallback
/// (shared-edge endpoints, off-seam control points, coplanarity, shared
/// vertices).
pub const SEAM_TOL: f64 = 1e-9;

/// The minimum signed turn of the projected rim corners certified by the
/// convex-boundary discharge of the graph-disk path. Coordinates are O(1) in
/// the v1 class; a degenerate (edge-on) projection fails the test and the
/// search moves to the next candidate.
pub const CONVEX_TURN_MARGIN: f64 = 1e-6;

/// One arc of the 2n-sided setback loop: a profile cut `P_i` across an
/// incoming fillet flank or a spring curve `Q_i` on a surviving primary face.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArcKind {
    /// A profile cut `P_i` across an incoming fillet flank.
    Profile,
    /// A spring curve `Q_i` on a surviving primary face.
    Spring,
}

/// One boundary arc of the setback loop with its exact tangent-plane data (the
/// v1 data class; delivered by the CC-030/031 traces and the incident strata).
#[derive(Debug, Clone, PartialEq)]
pub struct SetbackArc {
    /// Whether the arc is a profile cut or a spring curve.
    pub kind: ArcKind,
    /// The cubic Bézier control points of the boundary arc `C(u)`, `u ∈ [0,1]`.
    /// `curve[0]` and `curve[3]` are the shared rim corners.
    pub curve: [[f64; 3]; 4],
    /// The unit normal of the adjacent affine shell surface over the arc (its
    /// constant tangent plane).
    pub normal: [f64; 3],
    /// The Bernstein coefficients of the delivered cross-tangent field
    /// `cross(u) = P_v(u,0) = Σ_j cross[j] · B_j(u)` lying in the arc's tangent
    /// plane. The endpoint coefficients are the corner advances of the setback
    /// split: `cross[3]` of an arc equals `cross[0]` of the next arc.
    pub cross: [[f64; 3]; 4],
}

/// The complete 2n-sided setback input: the ordered boundary arcs with their
/// exact tangent-plane data.
#[derive(Debug, Clone, PartialEq)]
pub struct SetbackInput {
    /// The `2n` boundary arcs in cyclic order, alternating `Profile` and
    /// `Spring`.
    pub arcs: Vec<SetbackArc>,
}

impl SetbackInput {
    /// Validate a 2n-sided setback loop against the closed v1 data class and
    /// refuse [`ConstructRefusal::InvalidInput`] on any violation:
    ///
    /// - the arc count is even and `≥ 2 · SETBACK_MIN_VALENCE` (a genuine n = 3
    ///   triple routes through CC-020's node, never here);
    /// - the arc kinds alternate around the loop;
    /// - the chain closes exactly: `arcs[i].curve[3] == arcs[i+1].curve[0]`
    ///   (identical shared rim-corner values);
    /// - the corner-closure identity holds exactly:
    ///   `arcs[i].cross[3] == arcs[i+1].cross[0]`;
    /// - the curve control points and the cross coefficients lie in the
    ///   delivered tangent plane within `DATA_PLANE_TOL`.
    pub fn try_new(arcs: Vec<SetbackArc>) -> Result<Self, ConstructRefusal> {
        let count = arcs.len();
        if count < 2 * SETBACK_MIN_VALENCE || !count.is_multiple_of(2) {
            return Err(ConstructRefusal::InvalidInput);
        }
        for (index, arc) in arcs.iter().enumerate() {
            let next_index = (index + 1) % count;
            let next = &arcs[next_index];
            if arc.kind == next.kind {
                return Err(ConstructRefusal::InvalidInput);
            }
            if arc.curve[3] != next.curve[0] {
                return Err(ConstructRefusal::InvalidInput);
            }
            if arc.cross[3] != next.cross[0] {
                return Err(ConstructRefusal::InvalidInput);
            }
            validate_arc_data(arc)?;
        }
        Ok(SetbackInput { arcs })
    }
}

/// The piece kind of the built setback region.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PieceKind {
    /// A Hermite ribbon piece spanning one boundary arc to its rail edge.
    Ribbon,
    /// A hub quad of the flat cap over the rail polygon.
    Hub,
}

/// One tensor bicubic piece of the built setback region.
#[derive(Debug, Clone, PartialEq)]
pub struct SetbackPiece {
    /// The piece kind.
    pub kind: PieceKind,
    /// The input arc this ribbon carries (a hub quad carries `None`).
    pub arc: Option<usize>,
    /// The bicubic Bézier control net over `[0,1]²`; `net[v][u]`, so `net[0]`
    /// is the `v = 0` (outer boundary) row of a ribbon.
    pub net: [[[f64; 3]; 4]; 4],
}

/// The built setback patch: the deterministic corner-advance fill of the 2n
/// loop, split into 2n Hermite ribbons plus the `n` hub quads of the cap.
#[derive(Debug, Clone)]
pub struct SetbackPatch {
    /// The validated input the patch was built from.
    pub input: SetbackInput,
    /// The pieces in deterministic order: the 2n ribbons in arc order, then
    /// the n hub quads.
    pub pieces: Vec<SetbackPiece>,
    /// The hub vertices `H_0 .. H_{2n-1}` of the corner-advance rule, in rim
    /// corner order.
    pub hub: Vec<[f64; 3]>,
    /// The delivered enclosure `ε`: the maximum certified control width of the
    /// ribbon interpolation solves.
    pub epsilon: f64,
}

impl SetbackPatch {
    /// The ribbon piece carrying input arc `index`.
    pub fn ribbon(&self, index: usize) -> &SetbackPiece {
        &self.pieces[index]
    }

    /// The number of arcs `2n`.
    pub fn arc_count(&self) -> usize {
        self.input.arcs.len()
    }
}

/// The deterministic corner-advance construction of the setback patch.
///
/// The rule is recorded in the module doc (STOP CONDITION 1): hub vertex
/// `H_i = V_i + arcs[(i + 2n - 1) % 2n].cross[3]` (the shared corner advanced
/// by the delivered cross step); ribbon `i` spans arc `i` (v = 0) to the
/// straight rail edge `H_i H_{i+1}` (v = 1); the rail polygon is capped by `n`
/// bilinear hub quads over the consecutive rail-edge pairs. Every row follows
/// the module doc's closed form, so the build is deterministic and refuses
/// only where the data cannot be certified: a cross field that the certified
/// ribbon interpolation solve cannot reproduce propagates the solve's refusal,
/// and a non-finite intermediate refuses [`ConstructRefusal::InvalidInput`].
pub fn build_setback_patch(input: &SetbackInput) -> Result<SetbackPatch, ConstructRefusal> {
    let count = input.arcs.len();
    if count < 2 * SETBACK_MIN_VALENCE || !count.is_multiple_of(2) {
        return Err(ConstructRefusal::InvalidInput);
    }
    let n = count / 2;
    let mut hub = Vec::with_capacity(count);
    for index in 0..count {
        let vertex = input.arcs[index].curve[0];
        let previous = input.arcs[(index + count - 1) % count].cross[3];
        hub.push(add3(vertex, previous));
    }
    let mut pieces = Vec::with_capacity(count + n);
    let mut epsilon = 0.0_f64;
    for (index, arc) in input.arcs.iter().enumerate() {
        let arc_epsilon = certified_profile_solve(&arc.cross)?;
        if arc_epsilon > epsilon {
            epsilon = arc_epsilon;
        }
        let next_index = (index + 1) % count;
        let rail_lo = hub[index];
        let rail_hi = hub[next_index];
        let mut net = [[[0.0_f64; 3]; 4]; 4];
        #[allow(clippy::needless_range_loop)]
        // the fixed 4-column ribbon fill (t = j/3 along the arc) is the determinism contract
        for j in 0..4 {
            let t = (j as f64) / 3.0;
            net[0][j] = arc.curve[j];
            net[1][j] = add3(arc.curve[j], scale3(arc.cross[j], 1.0 / 3.0));
            net[3][j] = lerp3(rail_lo, rail_hi, t);
            net[2][j] = scale3(add3(net[1][j], net[3][j]), 0.5);
        }
        if !net_finite(&net) {
            return Err(ConstructRefusal::InvalidInput);
        }
        pieces.push(SetbackPiece {
            kind: PieceKind::Ribbon,
            arc: Some(index),
            net,
        });
    }
    let centre = hub_centre(&hub)?;
    for k in 0..n {
        let a = hub[2 * k];
        let b = hub[2 * k + 1];
        let c = hub[(2 * k + 2) % count];
        let mut net = [[[0.0_f64; 3]; 4]; 4];
        #[allow(clippy::needless_range_loop)]
        // the fixed row-major hub-grid fill (v rows) is the determinism contract
        for v in 0..4 {
            let s = (v as f64) / 3.0;
            #[allow(clippy::needless_range_loop)]
            // the fixed row-major hub-grid fill (u columns within a v row) is the determinism contract
            for u in 0..4 {
                let t = (u as f64) / 3.0;
                net[v][u] = bilinear_point(centre, a, b, c, t, s);
            }
        }
        pieces.push(SetbackPiece {
            kind: PieceKind::Hub,
            arc: None,
            net,
        });
    }
    Ok(SetbackPatch {
        input: input.clone(),
        pieces,
        hub,
        epsilon,
    })
}

/// One count-1 boundary record: the outer boundary of one ribbon piece against
/// its prescribed arc.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoundaryRecord {
    /// The input arc index of the ribbon.
    pub arc: usize,
    /// The maximum control-point deviation of `P(u,0)` from the prescribed
    /// arc, in Euclidean units.
    pub max_deviation: f64,
}

/// COUNT 1 — boundary: every outer patch boundary equals its prescribed
/// `P_i`/`Q_i` arc up to the delivered enclosure `ε`.
///
/// The outer boundary of ribbon `i` is its `v = 0` row, which the construction
/// copies from the arc coefficients; the count measures the deviation
/// control-pointwise and refuses
/// [`ConstructRefusal::ConditioningBelowThreshold`] when any deviation exceeds
/// the delivered `ε`.
pub fn certify_boundary(patch: &SetbackPatch) -> Result<Vec<BoundaryRecord>, ConstructRefusal> {
    let mut records = Vec::with_capacity(patch.input.arcs.len());
    for (index, arc) in patch.input.arcs.iter().enumerate() {
        let net = patch.ribbon(index).net;
        let mut max_deviation = 0.0_f64;
        #[allow(clippy::needless_range_loop)]
        // the fixed 4-row boundary-deviation scan (j) is the determinism contract
        for j in 0..4 {
            #[allow(clippy::needless_range_loop)]
            // the fixed 3-coordinate boundary-deviation scan (c) is the determinism contract
            for c in 0..3 {
                let deviation = (net[0][j][c] - arc.curve[j][c]).abs();
                if deviation > max_deviation {
                    max_deviation = deviation;
                }
            }
        }
        if !max_deviation.is_finite() || max_deviation > patch.epsilon {
            return Err(ConstructRefusal::ConditioningBelowThreshold);
        }
        records.push(BoundaryRecord {
            arc: index,
            max_deviation,
        });
    }
    Ok(records)
}

/// One count-2 G¹ record for one boundary arc.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RibbonG1 {
    /// The input arc index of the ribbon.
    pub arc: usize,
    /// The certified enclosure of the cross-tangent scale `λ(u)` over
    /// `u ∈ [0,1]` against the unit in-plane direction. Its lower endpoint is
    /// strictly positive by the certificate.
    pub lambda: Interval,
    /// The certified enclosure of `P_v(u,0)·n̂` over `u ∈ [0,1]` (the plane
    /// gap of the cross tangent against the delivered tangent plane), bounded
    /// by `G1_PLANE_TOL` by the certificate.
    pub plane_gap: Interval,
}

/// COUNT 2 — G¹ ribbons: on each boundary `P_v(u,0) = λ(u)·d(u)` with the
/// certified `λ` enclosure strictly positive and the direction `d` in the
/// adjacent tangent plane.
///
/// The cross tangent is the ribbon's own derivative data `P_v(u,0)` (degree-3
/// Bernstein coefficients `3 · (net[1] − net[0])`). `d(u)` is the unit
/// in-plane direction field it is measured against (the delivered tangent data
/// lies in the arc's tangent plane by validation). `λ` is the certified norm
/// lower bound of the cross tangent; its strict positivity IS the fold-back
/// prevention, so a `λ.lo ≤ 0` enclosure refuses here
/// ([`ConstructRefusal::ConditioningBelowThreshold`]), never in a separate
/// check. The plane gap is the certified control-hull of `P_v(u,0)·n̂` over the
/// whole arc; an out-of-plane cross tangent refuses the same refusal.
pub fn certify_g1_ribbons(patch: &SetbackPatch) -> Result<Vec<RibbonG1>, ConstructRefusal> {
    let mut records = Vec::with_capacity(patch.input.arcs.len());
    for (index, arc) in patch.input.arcs.iter().enumerate() {
        let net = patch.ribbon(index).net;
        let mut cross_coeffs: [[f64; 3]; 4] = [[0.0_f64; 3]; 4];
        for j in 0..4 {
            for c in 0..3 {
                cross_coeffs[j][c] = 3.0 * (net[1][j][c] - net[0][j][c]);
            }
        }
        let mut gap_coeffs = [0.0_f64; 4];
        for j in 0..4 {
            gap_coeffs[j] = dot3(&cross_coeffs[j], &arc.normal);
        }
        let plane_gap = scalar_range(&gap_coeffs)?;
        if !(plane_gap.lo >= -G1_PLANE_TOL && plane_gap.hi <= G1_PLANE_TOL) {
            return Err(ConstructRefusal::ConditioningBelowThreshold);
        }
        let lambda = norm_enclosure(&cross_coeffs)?;
        if !(lambda.is_finite() && lambda.lo > 0.0) {
            return Err(ConstructRefusal::ConditioningBelowThreshold);
        }
        records.push(RibbonG1 {
            arc: index,
            lambda,
            plane_gap,
        });
    }
    Ok(records)
}

/// The count-3 regularity certificate: the certified lower bound of
/// `inf ‖P_u × P_v‖` over the whole patch.
#[derive(Debug, Clone)]
pub struct RegularityCert {
    /// The certified lower bound of `‖P_u × P_v‖` over the whole patch: the
    /// minimum over the pieces.
    pub margin_lower: f64,
    /// The certified per-piece lower bounds, in piece order.
    pub per_piece: Vec<f64>,
}

/// COUNT 3 — local regularity: `inf ‖P_u × P_v‖ ≥ CC_ETA_J` over the whole
/// patch domain via the CC-002 hull path on each piece's Bézier form.
///
/// Per piece the six first-derivative coefficient grids feed the CC-002 hull
/// discipline, the interval normal is the three fixed cross-product
/// expressions, and the certified norm lower bound is `sqrt(Σ lb_k²)` with the
/// component lower bounds and every square and sum rounded downward — the D2
/// reduction of the certified-map rank margin, re-derived over the raw nets. A
/// piece whose bound is not strictly above `CC_ETA_J` refuses
/// [`ConstructRefusal::ConditioningBelowThreshold`].
pub fn certify_regularity(patch: &SetbackPatch) -> Result<RegularityCert, ConstructRefusal> {
    let mut per_piece = Vec::with_capacity(patch.pieces.len());
    for piece in &patch.pieces {
        let (_, margin) = piece_regularity(&piece.net)?;
        if !margin.is_finite() || margin <= CC_ETA_J {
            return Err(ConstructRefusal::ConditioningBelowThreshold);
        }
        per_piece.push(margin);
    }
    let mut margin_lower = f64::INFINITY;
    for margin in &per_piece {
        if *margin < margin_lower {
            margin_lower = *margin;
        }
    }
    Ok(RegularityCert {
        margin_lower,
        per_piece,
    })
}

/// How the embeddedness count was discharged.
#[derive(Debug, Clone, PartialEq)]
pub enum EmbeddedVerdict {
    /// The graph-disk certificate won with a single projection: the winning
    /// projection and the decider's witness record.
    GraphDisk {
        /// The winning unit projection direction.
        w: [f64; 3],
        /// The graph-disk decider's certificate.
        cert: GraphDiskCert,
    },
    /// The normative projection search exhausted and the PAIRWISE fallback
    /// certified the union.
    Pairwise,
}

/// The count-4 embeddedness certificate.
#[derive(Debug, Clone)]
pub struct Embeddedness {
    /// The discharge that certified (or the path that refused).
    pub verdict: EmbeddedVerdict,
    /// The certified per-piece regularity lower bounds used by the discharge.
    pub margins: Vec<f64>,
}

/// COUNT 4 — global embeddedness of the whole 2n-sided region.
///
/// First the graph-disk decider [`certify_graph_disk`] runs over the pieces'
/// derivative certificates under the NORMATIVE projection search
/// ([`search_projection`] over the frozen seam-S6 candidate family). The
/// boundary discharge certifies the projected rim as a strictly convex chain.
/// Exhaustion of the search (`NoAdmissibleProjection`) falls back to
/// [`pairwise_embeddedness`], which still certifies or refuses
/// `NoAdmissibleProjection`. The embeddedness count is scoped to CHORD arcs in
/// v1 (a curved arc refuses `InvalidInput` here — recorded in the module doc).
pub fn certify_embeddedness(patch: &SetbackPatch) -> Result<Embeddedness, ConstructRefusal> {
    for arc in &patch.input.arcs {
        if !arc_is_straight(arc) {
            return Err(ConstructRefusal::InvalidInput);
        }
    }
    let margins = per_piece_margins(patch)?;
    for margin in &margins {
        if !margin.is_finite() || *margin <= CC_ETA_J {
            return Err(ConstructRefusal::NoAdmissibleProjection);
        }
    }
    let admitted = admitted_pieces(patch)?;
    let corners = rim_corners(patch)?;
    let closure_corners = corners;
    let mut closure = move |w: [f64; 3]| -> Result<bool, ConstructRefusal> {
        let frame = orthonormal_frame(w)?;
        convex_chain_projected(&closure_corners, &frame)
    };
    match search_projection(&admitted, &mut closure) {
        Ok((w, disks)) => {
            let plan = BoundaryPlan {
                boundary_simple: true,
                seams_glued: true,
            };
            let cert = certify_graph_disk(&disks, &plan)?;
            Ok(Embeddedness {
                verdict: EmbeddedVerdict::GraphDisk { w, cert },
                margins,
            })
        }
        Err(ConstructRefusal::NoAdmissibleProjection) => {
            pairwise_embeddedness(patch)?;
            Ok(Embeddedness {
                verdict: EmbeddedVerdict::Pairwise,
                margins,
            })
        }
        Err(other) => Err(other),
    }
}

/// The PAIRWISE fallback of COUNT 4: certify or refuse the union without a
/// single projection.
///
/// The discharge over the v1 planar-piece class:
///
/// - **Manifest-edge seam coincidence:** two pieces that share a boundary edge
///   share EXACTLY one, with identical straight seam endpoints (within
///   `SEAM_TOL`). A pair sharing two edges refuses.
/// - **Boundary intersection exclusion:** every pair sharing NO edge is
///   certified disjoint either by strict axis separation of the pieces'
///   certified 3D enclosures or by the shared-VERTEX witness (two flat pieces
///   meeting only at a single common corner). A pair that cannot be certified
///   disjoint refuses.
/// - **Adjacent inside/outside witness:** an adjacent pair meets only along
///   the shared edge. For non-coplanar pieces the two planes meet in the seam
///   line and each piece lies strictly on one side of that line away from the
///   shared edge, so the planar intersection is exactly the shared edge. For
///   coplanar pieces the in-plane half-plane witness separates the two
///   interiors on opposite sides of the seam.
/// - **Regularity:** every piece's certified `‖P_u × P_v‖` lower bound is
///   strictly above `CC_ETA_J`.
///
/// Any failure refuses [`ConstructRefusal::NoAdmissibleProjection`] — the
/// exhaustion verdict of the count.
pub fn pairwise_embeddedness(patch: &SetbackPatch) -> Result<(), ConstructRefusal> {
    let margins = per_piece_margins(patch)?;
    for margin in &margins {
        if !margin.is_finite() || *margin <= CC_ETA_J {
            return Err(ConstructRefusal::NoAdmissibleProjection);
        }
    }
    let boxes = piece_boxes(patch)?;
    for i in 0..patch.pieces.len() {
        for j in (i + 1)..patch.pieces.len() {
            let shared = shared_edge_count(&patch.pieces[i], &patch.pieces[j])?;
            if shared == 0 {
                if !boxes_separated(&boxes[i], &boxes[j])
                    && !vertex_only_contact(&patch.pieces[i], &patch.pieces[j])?
                {
                    return Err(ConstructRefusal::NoAdmissibleProjection);
                }
            } else if shared == 1 {
                if !seam_witness(&patch.pieces[i], &patch.pieces[j])? {
                    return Err(ConstructRefusal::NoAdmissibleProjection);
                }
            } else {
                return Err(ConstructRefusal::NoAdmissibleProjection);
            }
        }
    }
    Ok(())
}

/// The certified profile interpolation solve of one arc's delivered cross
/// field (Section 1): the dense Bernstein-node sample map `s = M·x` (degree-3,
/// nodes `t_j = j/3`) recovered by the certified solve, returning the
/// certified L2 control width `ε` of the recovered coefficients.
///
/// The matrix is structurally inspected: a banded-TP-shaped system (structural
/// half-bandwidth at most [`RIBBON_TP_MAX_BAND`]) takes the
/// `factor_banded_tp` / `solve_homogeneous` fast path; every other system is
/// DENSE and takes [`residual_solve_dense`]. A refusal propagates — the
/// construction never guesses.
pub fn certified_profile_solve(cross: &[[f64; 3]; 4]) -> Result<f64, ConstructRefusal> {
    let matrix = bernstein_sample_matrix();
    let band = structural_half_bandwidth(&matrix);
    let mut width = 0.0_f64;
    for coordinate in 0..3 {
        let samples = sample_field(cross, coordinate);
        let enclosures: [Interval; 4] = samples.map(Interval::point);
        if band <= RIBBON_TP_MAX_BAND {
            let factor = factor_banded_tp(&flat_rows(&matrix))?;
            let mut rhs: Vec<[Interval; 4]> = Vec::with_capacity(4);
            #[allow(clippy::needless_range_loop)]
            // the fixed 0..4 rhs-row assembly order is the determinism contract
            for row in 0..4 {
                let mut channels = [Interval::point(0.0); 4];
                channels[coordinate] = enclosures[row];
                rhs.push(channels);
            }
            let solved = factor.solve_homogeneous(&rhs)?;
            #[allow(clippy::needless_range_loop)]
            // the fixed 0..4 solved-row scan order is the determinism contract
            for row in 0..4 {
                let w = solved[row][coordinate].hi - solved[row][coordinate].lo;
                if w > width {
                    width = w;
                }
            }
        } else {
            let a = interval_rows(&matrix);
            let r_inv = float_inverse(&matrix).ok_or(ConstructRefusal::InvalidInput)?;
            let x_hat = sample_center(cross, coordinate);
            let solved = residual_solve_dense(&a, &r_inv, &x_hat, &enclosures)?;
            for entry in solved {
                let w = entry.hi - entry.lo;
                if w > width {
                    width = w;
                }
            }
        }
    }
    Ok(width)
}

// ---------------------------------------------------------------------------
// The certified-profile solve helpers
// ---------------------------------------------------------------------------

/// The 4×4 Bernstein-node sample matrix `M[j][k] = B_k³(t_j)` at the nodes
/// `t_j = j/3`, in fixed order.
fn bernstein_sample_matrix() -> [[f64; 4]; 4] {
    let mut matrix = [[0.0_f64; 4]; 4];
    #[allow(clippy::needless_range_loop)]
    // the fixed row-major Bernstein sample fill (j rows) is the determinism contract
    for j in 0..4 {
        let t = (j as f64) / 3.0;
        #[allow(clippy::needless_range_loop)]
        // the fixed row-major Bernstein sample fill (k columns within a j row) is the determinism contract
        for k in 0..4 {
            matrix[j][k] = bernstein_basis(3, k, t);
        }
    }
    matrix
}

/// The univariate Bernstein basis value `B_k^n(t)` for `n = 3`.
fn bernstein_basis(n: usize, k: usize, t: f64) -> f64 {
    if n != 3 {
        return 0.0;
    }
    let binom = match k {
        0 | 3 => 1.0,
        1 | 2 => 3.0,
        _ => 0.0,
    };
    let pow = |base: f64, exponent: usize| -> f64 {
        let mut acc = 1.0;
        for _ in 0..exponent {
            acc *= base;
        }
        acc
    };
    binom * pow(t, k) * pow(1.0 - t, n - k)
}

/// The value samples of one coordinate of the delivered field at the Bernstein
/// nodes `t_j = j/3`, in fixed order.
fn sample_field(cross: &[[f64; 3]; 4], coordinate: usize) -> [f64; 4] {
    let matrix = bernstein_sample_matrix();
    let mut samples = [0.0_f64; 4];
    for j in 0..4 {
        let mut acc = 0.0_f64;
        for k in 0..4 {
            acc += matrix[j][k] * cross[k][coordinate];
        }
        samples[j] = acc;
    }
    samples
}

/// The coefficient seed of the dense solve (the delivered coefficients are the
/// natural `x̂` of the recovery).
fn sample_center(cross: &[[f64; 3]; 4], coordinate: usize) -> [f64; 4] {
    let mut centre = [0.0_f64; 4];
    for j in 0..4 {
        centre[j] = cross[j][coordinate];
    }
    centre
}

/// The interval matrix of a float matrix (row-major point intervals).
fn interval_rows(matrix: &[[f64; 4]; 4]) -> [[Interval; 4]; 4] {
    let mut rows = [[Interval::point(0.0); 4]; 4];
    for i in 0..4 {
        for j in 0..4 {
            rows[i][j] = Interval::point(matrix[i][j]);
        }
    }
    rows
}

/// The row-major flat interval storage of a matrix, as the banded factor
/// consumes it.
fn flat_rows(matrix: &[[f64; 4]; 4]) -> Vec<Interval> {
    interval_rows(matrix)
        .iter()
        .flat_map(|row| row.iter().copied())
        .collect()
}

/// The structural half-bandwidth of a matrix: the largest `|i − j|` over
/// entries that are not exactly zero.
fn structural_half_bandwidth(matrix: &[[f64; 4]; 4]) -> usize {
    let mut band = 0usize;
    #[allow(clippy::needless_range_loop)]
    // the fixed row-major |i − j| band scan (i rows) is the determinism contract
    for i in 0..4 {
        #[allow(clippy::needless_range_loop)]
        // the fixed row-major |i − j| band scan (j columns within an i row) is the determinism contract
        for j in 0..4 {
            if matrix[i][j] != 0.0 {
                let distance = i.abs_diff(j);
                if distance > band {
                    band = distance;
                }
            }
        }
    }
    band
}

/// A deterministic `f64` Gauss–Jordan inverse with partial pivoting of a 4×4
/// matrix; `None` on a singular matrix. This is the FLOAT preconditioner only —
/// the certificate comes from the certified solve.
fn float_inverse(matrix: &[[f64; 4]; 4]) -> Option<[[f64; 4]; 4]> {
    let mut aug = [[0.0_f64; 8]; 4];
    for i in 0..4 {
        for j in 0..4 {
            aug[i][j] = matrix[i][j];
        }
        aug[i][4 + i] = 1.0;
    }
    for pivot in 0..4 {
        let mut best = pivot;
        let mut best_value = aug[pivot][pivot].abs();
        #[allow(clippy::needless_range_loop)]
        // the partial-row pivot scan over the fixed aug rows is the determinism contract
        for row in (pivot + 1)..4 {
            let value = aug[row][pivot].abs();
            if value > best_value {
                best_value = value;
                best = row;
            }
        }
        if best_value == 0.0 || !best_value.is_finite() {
            return None;
        }
        if best != pivot {
            aug.swap(pivot, best);
        }
        let diagonal = aug[pivot][pivot];
        #[allow(clippy::needless_range_loop)]
        // the fixed 0..8 column normalisation of the pivot row is the determinism contract
        for col in 0..8 {
            aug[pivot][col] /= diagonal;
        }
        for row in 0..4 {
            if row == pivot {
                continue;
            }
            let factor = aug[row][pivot];
            #[allow(clippy::needless_range_loop)]
            // the fixed 0..8 column elimination of a non-pivot row is the determinism contract
            for col in 0..8 {
                aug[row][col] -= factor * aug[pivot][col];
            }
        }
    }
    let mut inverse = [[0.0_f64; 4]; 4];
    for i in 0..4 {
        for j in 0..4 {
            let value = aug[i][4 + j];
            if !value.is_finite() {
                return None;
            }
            inverse[i][j] = value;
        }
    }
    Some(inverse)
}

// ---------------------------------------------------------------------------
// Construction helpers
// ---------------------------------------------------------------------------

/// Validate one arc's data against the closed v1 class.
fn validate_arc_data(arc: &SetbackArc) -> Result<(), ConstructRefusal> {
    let normal_norm = norm3(&arc.normal);
    if !normal_norm.is_finite() || (normal_norm - 1.0).abs() > DATA_PLANE_TOL {
        return Err(ConstructRefusal::InvalidInput);
    }
    let base = arc.curve[0];
    for control in arc.curve {
        if !control.iter().all(|value| value.is_finite()) {
            return Err(ConstructRefusal::InvalidInput);
        }
        if dot3(&sub3(control, base), &arc.normal).abs() > DATA_PLANE_TOL {
            return Err(ConstructRefusal::InvalidInput);
        }
    }
    for coefficient in arc.cross {
        if !coefficient.iter().all(|value| value.is_finite()) {
            return Err(ConstructRefusal::InvalidInput);
        }
        if dot3(&coefficient, &arc.normal).abs() > DATA_PLANE_TOL {
            return Err(ConstructRefusal::InvalidInput);
        }
    }
    Ok(())
}

/// Whether every control point of a net is finite.
fn net_finite(net: &[[[f64; 3]; 4]; 4]) -> bool {
    net.iter()
        .flatten()
        .flatten()
        .all(|value| value.is_finite())
}

/// The hub centre: the fixed-order mean of the hub vertices.
fn hub_centre(hub: &[[f64; 3]]) -> Result<[f64; 3], ConstructRefusal> {
    if hub.is_empty() {
        return Err(ConstructRefusal::InvalidInput);
    }
    let mut acc = [0.0_f64; 3];
    for vertex in hub {
        for (slot, component) in acc.iter_mut().zip(vertex.iter()) {
            *slot += component;
        }
    }
    let scale = 1.0 / (hub.len() as f64);
    let centre = [acc[0] * scale, acc[1] * scale, acc[2] * scale];
    if centre.iter().all(|value| value.is_finite()) {
        Ok(centre)
    } else {
        Err(ConstructRefusal::InvalidInput)
    }
}

/// Evaluate the bilinear hub quad with corners `(p00, p10, p11, p01)` (the
/// `u`/`v` cyclic corner order of the quad map) at `(u, v)`.
fn bilinear_point(
    p00: [f64; 3],
    p10: [f64; 3],
    p11: [f64; 3],
    p01: [f64; 3],
    u: f64,
    v: f64,
) -> [f64; 3] {
    let bottom = lerp3(p00, p10, u);
    let top = lerp3(p01, p11, u);
    lerp3(bottom, top, v)
}

// ---------------------------------------------------------------------------
// The certification helpers
// ---------------------------------------------------------------------------

/// The certified enclosure of the Euclidean norm of a degree-3 vector field
/// over `[0,1]` from the coefficient convex hulls of its coordinate
/// polynomials: the lower bound `sqrt(Σ lb_k²)` and the upper bound
/// `sqrt(Σ sup_k²)`, rounded outward. A polynomial's range over `[0,1]` lies
/// within the coordinate range of its Bernstein coefficients, so the
/// coefficient hull is a valid enclosure without any interval-evaluation
/// dependency widening.
fn norm_enclosure(field: &[[f64; 3]; 4]) -> Result<Interval, ConstructRefusal> {
    let mut components = [Interval::point(0.0); 3];
    for coordinate in 0..3 {
        let mut lo = f64::INFINITY;
        let mut hi = f64::NEG_INFINITY;
        for point in field {
            let value = point[coordinate];
            if !value.is_finite() {
                return Err(ConstructRefusal::InvalidInput);
            }
            lo = lo.min(value);
            hi = hi.max(value);
        }
        components[coordinate] = Interval { lo, hi };
    }
    let mut sq_lo = 0.0_f64;
    let mut sq_hi = 0.0_f64;
    for component in &components {
        let lb = component_lower_bound(component);
        let sup = component_upper_bound(component);
        sq_lo = (sq_lo + (lb * lb).next_down()).next_down();
        sq_hi = (sq_hi + (sup * sup).next_up()).next_up();
    }
    let lower = sq_lo.max(0.0).sqrt().next_down();
    let upper = sq_hi.sqrt().next_up();
    if lower.is_finite() && upper.is_finite() {
        Ok(Interval {
            lo: lower,
            hi: upper,
        })
    } else {
        Err(ConstructRefusal::InvalidInput)
    }
}

/// The certified upper bound of `|x|` over an interval enclosure: the larger
/// endpoint magnitude.
fn component_upper_bound(value: &Interval) -> f64 {
    value.lo.abs().max(value.hi.abs())
}

/// The certified lower bound of `|x|` over an interval enclosure: `0` when the
/// enclosure contains zero, else the nearer endpoint's magnitude.
fn component_lower_bound(value: &Interval) -> f64 {
    if value.lo <= 0.0 && value.hi >= 0.0 {
        0.0
    } else {
        value.lo.abs().min(value.hi.abs())
    }
}

/// The certified per-piece regularity certificate of one piece (the CC-002
/// hull path over its Bézier form). The coordinate range of a derivative
/// field's Bernstein coefficients is a valid control-hull enclosure of the
/// field over the whole unit domain (convex-hull property, no interval
/// dependency), so the interval normal components and the certified lower
/// bound of `inf ‖P_u × P_v‖` follow directly. Returns the certified normal
/// enclosure `S_u × S_v` over the whole piece domain and the certified lower
/// bound of `inf ‖P_u × P_v‖`.
fn piece_regularity(net: &[[[f64; 3]; 4]; 4]) -> Result<([Interval; 3], f64), ConstructRefusal> {
    let grids = coord_grids(net);
    let mut su = [Interval::point(0.0); 3];
    let mut sv = [Interval::point(0.0); 3];
    for coordinate in 0..3 {
        let du = bernstein_derivative_2d(&grids[coordinate], 0);
        let dv = bernstein_derivative_2d(&grids[coordinate], 1);
        su[coordinate] = grid_coefficient_range(&du)?;
        sv[coordinate] = grid_coefficient_range(&dv)?;
    }
    let n0 = su[1].mul(&sv[2]).sub(&su[2].mul(&sv[1]));
    let n1 = su[2].mul(&sv[0]).sub(&su[0].mul(&sv[2]));
    let n2 = su[0].mul(&sv[1]).sub(&su[1].mul(&sv[0]));
    let components = [n0, n1, n2];
    for component in &components {
        if !component.is_finite() {
            return Err(ConstructRefusal::InvalidInput);
        }
    }
    let mut sq_sum = 0.0_f64;
    for component in &components {
        let lb = component_lower_bound(component);
        sq_sum = (sq_sum + (lb * lb).next_down()).next_down();
    }
    let margin = sq_sum.max(0.0).sqrt().next_down();
    if !margin.is_finite() {
        return Err(ConstructRefusal::InvalidInput);
    }
    Ok((components, margin))
}

/// The coordinate range of a Bernstein coefficient grid: the interval hull of
/// every entry, which contains the field's range over the whole unit domain by
/// the convex-hull property.
fn grid_coefficient_range(grid: &[Vec<f64>]) -> Result<Interval, ConstructRefusal> {
    let mut lo = f64::INFINITY;
    let mut hi = f64::NEG_INFINITY;
    for row in grid {
        for value in row {
            if !value.is_finite() {
                return Err(ConstructRefusal::InvalidInput);
            }
            lo = lo.min(*value);
            hi = hi.max(*value);
        }
    }
    Ok(Interval { lo, hi })
}

/// The coefficient range of a fixed-length scalar coefficient array (the
/// control hull enclosure of the polynomial over the unit domain).
fn scalar_range(coeffs: &[f64; 4]) -> Result<Interval, ConstructRefusal> {
    let mut lo = f64::INFINITY;
    let mut hi = f64::NEG_INFINITY;
    for value in coeffs {
        if !value.is_finite() {
            return Err(ConstructRefusal::InvalidInput);
        }
        lo = lo.min(*value);
        hi = hi.max(*value);
    }
    Ok(Interval { lo, hi })
}

/// The per-coordinate tensor grids of a bicubic net.
fn coord_grids(net: &[[[f64; 3]; 4]; 4]) -> [Vec<Vec<f64>>; 3] {
    let mut grids: [Vec<Vec<f64>>; 3] = [Vec::new(), Vec::new(), Vec::new()];
    for coordinate in 0..3 {
        #[allow(clippy::needless_range_loop)]
        // the fixed coordinate-major grid extraction (v rows) is the determinism contract
        for v in 0..4 {
            let mut row = Vec::with_capacity(4);
            #[allow(clippy::needless_range_loop)]
            // the fixed coordinate-major grid extraction (u columns within a v row) is the determinism contract
            for u in 0..4 {
                row.push(net[v][u][coordinate]);
            }
            grids[coordinate].push(row);
        }
    }
    grids
}

/// The certified per-piece regularity lower bounds of a patch.
fn per_piece_margins(patch: &SetbackPatch) -> Result<Vec<f64>, ConstructRefusal> {
    let mut margins = Vec::with_capacity(patch.pieces.len());
    for piece in &patch.pieces {
        let (_, margin) = piece_regularity(&piece.net)?;
        margins.push(margin);
    }
    Ok(margins)
}

/// The certified 3D value enclosure boxes of the pieces: the coordinate range
/// of each piece's control net, which contains the whole surface by the
/// convex-hull property of its Bézier form.
fn piece_boxes(patch: &SetbackPatch) -> Result<Vec<[Interval; 3]>, ConstructRefusal> {
    let mut boxes = Vec::with_capacity(patch.pieces.len());
    for piece in &patch.pieces {
        let mut boxed = [Interval::point(0.0); 3];
        #[allow(clippy::needless_range_loop)]
        // the fixed 0..3 coordinate-box scan is the determinism contract
        for coordinate in 0..3 {
            let mut lo = f64::INFINITY;
            let mut hi = f64::NEG_INFINITY;
            #[allow(clippy::needless_range_loop)]
            // the fixed row-major net box scan (v rows) is the determinism contract
            for v in 0..4 {
                #[allow(clippy::needless_range_loop)]
                // the fixed row-major net box scan (u columns within a v row) is the determinism contract
                for u in 0..4 {
                    let value = piece.net[v][u][coordinate];
                    if !value.is_finite() {
                        return Err(ConstructRefusal::InvalidInput);
                    }
                    lo = lo.min(value);
                    hi = hi.max(value);
                }
            }
            boxed[coordinate] = Interval { lo, hi };
        }
        boxes.push(boxed);
    }
    Ok(boxes)
}

/// Whether two certified boxes are strictly separated on an axis.
fn boxes_separated(a: &[Interval; 3], b: &[Interval; 3]) -> bool {
    for axis in 0..3 {
        if a[axis].hi < b[axis].lo || b[axis].hi < a[axis].lo {
            return true;
        }
    }
    false
}

/// The boundary edges of a piece, each as its two endpoints. Edge 0 is the
/// `v = 0` row, edge 1 the `v = 1` row, edge 2 the `u = 0` column and edge 3
/// the `u = 1` column.
fn piece_edges(net: &[[[f64; 3]; 4]; 4]) -> [[[f64; 3]; 2]; 4] {
    [
        [net[0][0], net[0][3]],
        [net[3][0], net[3][3]],
        [net[0][0], net[3][0]],
        [net[0][3], net[3][3]],
    ]
}

/// The number of shared boundary edges of two pieces (endpoints coincide
/// within `SEAM_TOL` in either order). A count above one is a structural
/// defect.
fn shared_edge_count(a: &SetbackPiece, b: &SetbackPiece) -> Result<usize, ConstructRefusal> {
    let edges_a = piece_edges(&a.net);
    let edges_b = piece_edges(&b.net);
    let mut shared = 0usize;
    for edge_a in edges_a {
        for edge_b in edges_b {
            if segments_coincide(edge_a, edge_b) {
                shared += 1;
            }
        }
    }
    Ok(shared)
}

/// Whether two straight boundary segments coincide (endpoints equal within
/// `SEAM_TOL` in either order).
fn segments_coincide(a: [[f64; 3]; 2], b: [[f64; 3]; 2]) -> bool {
    points_close(a[0], b[0]) && points_close(a[1], b[1])
        || points_close(a[0], b[1]) && points_close(a[1], b[0])
}

/// Whether two points coincide within `SEAM_TOL`.
fn points_close(a: [f64; 3], b: [f64; 3]) -> bool {
    distance3(a, b) <= SEAM_TOL
}

/// The plane data of a piece: its base point and unit normal, certified flat
/// (every control point within `SEAM_TOL` of the plane). Refuses when the
/// piece is not flat.
fn piece_plane(net: &[[[f64; 3]; 4]; 4]) -> Result<([f64; 3], [f64; 3]), ConstructRefusal> {
    let base = net[0][0];
    let e1 = sub3(net[0][3], base);
    let e2 = sub3(net[3][0], base);
    let normal = unit3(cross3(&e1, &e2)).ok_or(ConstructRefusal::NoAdmissibleProjection)?;
    #[allow(clippy::needless_range_loop)]
    // the fixed row-major plane-containment scan (v rows) is the determinism contract
    for v in 0..4 {
        #[allow(clippy::needless_range_loop)]
        // the fixed row-major plane-containment scan (u columns within a v row) is the determinism contract
        for u in 0..4 {
            let offset = sub3(net[v][u], base);
            if dot3(&offset, &normal).abs() > SEAM_TOL {
                return Err(ConstructRefusal::NoAdmissibleProjection);
            }
        }
    }
    Ok((base, normal))
}

/// The inside/outside seam witness of one adjacent pair: certify that the two
/// pieces meet ONLY along their shared straight edge.
fn seam_witness(a: &SetbackPiece, b: &SetbackPiece) -> Result<bool, ConstructRefusal> {
    let (_, normal_a) = piece_plane(&a.net)?;
    let (_, normal_b) = piece_plane(&b.net)?;
    let shared = shared_edge(&a.net, &b.net).ok_or(ConstructRefusal::NoAdmissibleProjection)?;
    let p = shared[0];
    let q = shared[1];
    let coplanar = {
        let dot = dot3(&normal_a, &normal_b).abs();
        1.0 - dot <= SEAM_TOL
    };
    let on_a = if coplanar {
        let in_plane = unit3(cross3(&direction(p, q), &normal_a))
            .ok_or(ConstructRefusal::NoAdmissibleProjection)?;
        off_seam_side(&a.net, p, q, &in_plane)?
    } else {
        off_seam_side_any(&a.net, p, q)?
    };
    let on_b = if coplanar {
        let in_plane = unit3(cross3(&direction(p, q), &normal_a))
            .ok_or(ConstructRefusal::NoAdmissibleProjection)?;
        off_seam_side(&b.net, p, q, &in_plane)?
    } else {
        off_seam_side_any(&b.net, p, q)?
    };
    if !on_a || !on_b {
        return Ok(false);
    }
    if coplanar {
        Ok(true)
    } else {
        let cross_magnitude = norm3(&cross3(&normal_a, &normal_b));
        Ok(cross_magnitude.is_finite() && cross_magnitude > SEAM_TOL)
    }
}

/// The shared straight edge of two pieces as its two endpoints.
fn shared_edge(a: &[[[f64; 3]; 4]; 4], b: &[[[f64; 3]; 4]; 4]) -> Option<[[f64; 3]; 2]> {
    for edge_a in piece_edges(a) {
        for edge_b in piece_edges(b) {
            if segments_coincide(edge_a, edge_b) {
                return Some(edge_a);
            }
        }
    }
    None
}

/// The unit direction of a straight segment.
fn direction(p: [f64; 3], q: [f64; 3]) -> [f64; 3] {
    let unit = unit3(sub3(q, p));
    unit.unwrap_or([1.0, 0.0, 0.0])
}

/// Certify that a piece's off-seam corners lie strictly on ONE side of the
/// shared straight seam `p q` at signed distance at least `SEAM_TOL` along
/// `side` — the piece does not cross the seam line away from the shared edge.
fn off_seam_side(
    net: &[[[f64; 3]; 4]; 4],
    p: [f64; 3],
    q: [f64; 3],
    side: &[f64; 3],
) -> Result<bool, ConstructRefusal> {
    let mut found: Option<bool> = None;
    for corner in piece_corners(net) {
        if points_close(corner, p) || points_close(corner, q) {
            continue;
        }
        let signed = dot3(&sub3(corner, p), side);
        if signed.abs() <= SEAM_TOL {
            return Ok(false);
        }
        let positive = signed > 0.0;
        match found {
            Some(same) if same != positive => return Ok(false),
            _ => found = Some(positive),
        }
    }
    Ok(found.is_some())
}

/// Certify that a piece's off-seam corners lie strictly off the seam line of
/// the shared edge `p q`, at distance at least `SEAM_TOL`.
fn off_seam_side_any(
    net: &[[[f64; 3]; 4]; 4],
    p: [f64; 3],
    q: [f64; 3],
) -> Result<bool, ConstructRefusal> {
    let line_dir = direction(p, q);
    let mut count = 0usize;
    for corner in piece_corners(net) {
        if points_close(corner, p) || points_close(corner, q) {
            continue;
        }
        if point_line_distance(corner, p, &line_dir) <= SEAM_TOL {
            return Ok(false);
        }
        count += 1;
    }
    Ok(count >= 2)
}

/// Certify that two flat pieces that share no edge and whose 3D boxes overlap
/// meet only at a single shared VERTEX.
///
/// - If their planes are NOT parallel the planes meet in the seam line `L`
///   through the common vertex, and it suffices that ONE piece meets `L` only
///   at that vertex (its other corners lie strictly off `L`, all on one side):
///   then the two pieces' common points are contained in `{p}`.
/// - If the pieces are COPLANAR they meet only at `p` when the angular sectors
///   their other corners span around `p` are disjoint (both convex,
///   star-shaped from `p`).
///
/// Any other shared-vertex configuration returns `Ok(false)`; the caller
/// refuses `NoAdmissibleProjection` on it.
fn vertex_only_contact(a: &SetbackPiece, b: &SetbackPiece) -> Result<bool, ConstructRefusal> {
    let (_, normal_a) = piece_plane(&a.net)?;
    let (base_a, normal_b) = piece_plane(&b.net)?;
    let mut common: Vec<[f64; 3]> = Vec::new();
    for corner_a in piece_corners(&a.net) {
        for corner_b in piece_corners(&b.net) {
            if points_close(corner_a, corner_b) {
                common.push(corner_a);
            }
        }
    }
    if common.len() != 1 {
        return Ok(false);
    }
    let p = common[0];
    let parallel = 1.0 - dot3(&normal_a, &normal_b).abs() <= SEAM_TOL;
    if parallel {
        // Parallel planes that share a point are the same plane.
        if point_plane_distance(base_a, p, &normal_b) <= SEAM_TOL {
            return angular_sectors_disjoint(&a.net, &b.net, p, &normal_a);
        }
        return Ok(false);
    }
    let line_dir =
        unit3(cross3(&normal_a, &normal_b)).ok_or(ConstructRefusal::NoAdmissibleProjection)?;
    if quad_meets_line_only_at(&a.net, p, &line_dir, &normal_a)
        || quad_meets_line_only_at(&b.net, p, &line_dir, &normal_b)
    {
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Whether a flat convex quad meets a line `L` (through `p`, unit direction
/// `line_dir`, inside the quad's plane of normal `plane_normal`) only at its
/// vertex `p`: every other corner lies strictly off `L` and strictly on one
/// side of it.
fn quad_meets_line_only_at(
    net: &[[[f64; 3]; 4]; 4],
    p: [f64; 3],
    line_dir: &[f64; 3],
    plane_normal: &[f64; 3],
) -> bool {
    let side = match unit3(cross3(line_dir, plane_normal)) {
        Some(side) => side,
        None => return false,
    };
    let mut found: Option<bool> = None;
    for corner in piece_corners(net) {
        if points_close(corner, p) {
            continue;
        }
        if point_line_distance(corner, p, line_dir) <= SEAM_TOL {
            return false;
        }
        let signed = dot3(&sub3(corner, p), &side);
        let positive = signed > 0.0;
        match found {
            Some(same) if same != positive => return false,
            _ => found = Some(positive),
        }
    }
    found.is_some()
}

/// Whether two coplanar convex quads sharing only the vertex `p` meet nowhere
/// else: the angular sectors their other corners span around `p` (within the
/// shared plane) are disjoint.
fn angular_sectors_disjoint(
    a: &[[[f64; 3]; 4]; 4],
    b: &[[[f64; 3]; 4]; 4],
    p: [f64; 3],
    normal: &[f64; 3],
) -> Result<bool, ConstructRefusal> {
    let frame = orthonormal_frame(*normal)?;
    let angles_a = sector_angles(&piece_corners(a), p, &frame);
    let angles_b = sector_angles(&piece_corners(b), p, &frame);
    let arc_a = minimal_circular_arc(&angles_a);
    let arc_b = minimal_circular_arc(&angles_b);
    match (arc_a, arc_b) {
        (Some(arc_a), Some(arc_b)) => Ok(!circular_arcs_overlap(&arc_a, &arc_b)),
        _ => Ok(false),
    }
}

/// The angular positions (in `[0, 2π)`) of a quad's corners around a vertex
/// `p`, projected into the frame of its plane.
fn sector_angles(corners: &[[f64; 3]; 4], p: [f64; 3], frame: &[[f64; 3]; 2]) -> Vec<f64> {
    let mut angles = Vec::with_capacity(4);
    for corner in corners {
        if points_close(*corner, p) {
            continue;
        }
        let offset = sub3(*corner, p);
        let x = dot3(&offset, &frame[0]);
        let y = dot3(&offset, &frame[1]);
        angles.push(y.atan2(x));
    }
    angles
}

/// The minimal circular arc (start in `[0, 2π)` and width) containing a set of
/// angles, when that width is strictly below `π` (a convex quad star-shaped
/// from its vertex). `None` otherwise.
fn minimal_circular_arc(angles: &[f64]) -> Option<(f64, f64)> {
    let two_pi = 2.0 * std::f64::consts::PI;
    let count = angles.len();
    if count == 0 {
        return None;
    }
    let mut sorted: Vec<f64> = angles
        .iter()
        .map(|angle| angle.rem_euclid(two_pi))
        .collect();
    sorted.sort_by(|x, y| x.total_cmp(y));
    let mut largest_gap = sorted[0] + two_pi - sorted[count - 1];
    let mut gap_start = sorted[count - 1];
    for window in sorted.windows(2) {
        let gap = window[1] - window[0];
        if gap > largest_gap {
            largest_gap = gap;
            gap_start = window[0];
        }
    }
    let width = two_pi - largest_gap;
    if width >= std::f64::consts::PI {
        return None;
    }
    Some((gap_start.rem_euclid(two_pi), width))
}

/// Whether two minimal circular arcs (each of width `< π`) overlap.
fn circular_arcs_overlap(a: &(f64, f64), b: &(f64, f64)) -> bool {
    point_in_arc(a.0, b)
        || point_in_arc(a.0 + a.1, b)
        || point_in_arc(b.0, a)
        || point_in_arc(b.0 + b.1, a)
}

/// Whether an angle lies inside a minimal circular arc.
fn point_in_arc(angle: f64, arc: &(f64, f64)) -> bool {
    let two_pi = 2.0 * std::f64::consts::PI;
    let (start, width) = *arc;
    let delta = (angle - start).rem_euclid(two_pi);
    delta <= width
}

/// The distance of a point from a plane (base point `base`, unit normal).
fn point_plane_distance(base: [f64; 3], point: [f64; 3], normal: &[f64; 3]) -> f64 {
    dot3(&sub3(point, base), normal).abs()
}

/// The four net corners of a piece.
fn piece_corners(net: &[[[f64; 3]; 4]; 4]) -> [[f64; 3]; 4] {
    [net[0][0], net[0][3], net[3][0], net[3][3]]
}

/// The distance of a point to a line through `p` with unit direction
/// `direction`.
fn point_line_distance(point: [f64; 3], p: [f64; 3], direction: &[f64; 3]) -> f64 {
    let offset = sub3(point, p);
    let projection = dot3(&offset, direction);
    let perpendicular = sub3(offset, scale3(*direction, projection));
    norm3(&perpendicular)
}

/// The per-piece derivative certificates of the graph-disk search.
fn admitted_pieces(patch: &SetbackPatch) -> Result<Vec<AdmittedPiece>, ConstructRefusal> {
    let mut pieces = Vec::with_capacity(patch.pieces.len());
    for piece in &patch.pieces {
        let (normal_box, area_lower) = piece_regularity(&piece.net)?;
        let net_u = unit3(sub3(piece.net[0][3], piece.net[0][0])).unwrap_or([1.0, 0.0, 0.0]);
        let net_v = unit3(sub3(piece.net[3][0], piece.net[0][0])).unwrap_or([0.0, 1.0, 0.0]);
        pieces.push(AdmittedPiece {
            normal_box,
            area_lower,
            net_u,
            net_v,
            seam_glued: true,
        });
    }
    Ok(pieces)
}

/// The rim corners of the outer loop, in order.
fn rim_corners(patch: &SetbackPatch) -> Result<Vec<[f64; 3]>, ConstructRefusal> {
    let mut corners = Vec::with_capacity(patch.input.arcs.len());
    for arc in &patch.input.arcs {
        for point in arc.curve {
            if !point.iter().all(|value| value.is_finite()) {
                return Err(ConstructRefusal::InvalidInput);
            }
        }
        corners.push(arc.curve[0]);
    }
    Ok(corners)
}

/// Whether an arc is a straight chord: its interior control points sit on the
/// chord at the `1/3` and `2/3` positions within `SEAM_TOL`.
fn arc_is_straight(arc: &SetbackArc) -> bool {
    let p = arc.curve[0];
    let q = arc.curve[3];
    points_close(arc.curve[1], lerp3(p, q, 1.0 / 3.0))
        && points_close(arc.curve[2], lerp3(p, q, 2.0 / 3.0))
}

/// The convex-chain test over the projection of the rim corners: all signed
/// turns strictly positive above `CONVEX_TURN_MARGIN` — a certified-simple
/// projected boundary for the graph-disk search.
fn convex_chain_projected(
    corners: &[[f64; 3]],
    frame: &[[f64; 3]; 2],
) -> Result<bool, ConstructRefusal> {
    let count = corners.len();
    if count < 3 {
        return Ok(false);
    }
    let mut first_sign = 0.0_f64;
    for index in 0..count {
        let a = corners[index];
        let b = corners[(index + 1) % count];
        let c = corners[(index + 2) % count];
        let ua = planar_projection(a, frame);
        let ub = planar_projection(b, frame);
        let uc = planar_projection(c, frame);
        let ab = [ub[0] - ua[0], ub[1] - ua[1]];
        let bc = [uc[0] - ub[0], uc[1] - ub[1]];
        let turn = ab[0] * bc[1] - ab[1] * bc[0];
        if !turn.is_finite() {
            return Ok(false);
        }
        if turn.abs() <= CONVEX_TURN_MARGIN {
            return Ok(false);
        }
        let sign = turn.signum();
        if index == 0 {
            first_sign = sign;
        } else if sign != first_sign {
            return Ok(false);
        }
    }
    Ok(true)
}

/// The orthonormal projection frame of a unit direction: two unit vectors
/// spanning the plane perpendicular to `w`, in a fixed deterministic order.
fn orthonormal_frame(w: [f64; 3]) -> Result<[[f64; 3]; 2], ConstructRefusal> {
    let unit = unit3(w).ok_or(ConstructRefusal::InvalidInput)?;
    let seed = if unit[2].abs() < 0.9 {
        [0.0_f64, 0.0, 1.0]
    } else {
        [1.0_f64, 0.0, 0.0]
    };
    let e1 = unit3(cross3(&unit, &seed)).ok_or(ConstructRefusal::InvalidInput)?;
    let e2 = cross3(&unit, &e1);
    Ok([e1, e2])
}

/// Project one point into the frame, returning its planar coordinates.
fn planar_projection(point: [f64; 3], frame: &[[f64; 3]; 2]) -> [f64; 2] {
    [dot3(&point, &frame[0]), dot3(&point, &frame[1])]
}

// ---------------------------------------------------------------------------
// Geometric scalar helpers (fixed order, plain `f64` construction math)
// ---------------------------------------------------------------------------

/// Vector addition.
fn add3(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

/// Vector subtraction.
fn sub3(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

/// Scalar-vector product.
fn scale3(a: [f64; 3], s: f64) -> [f64; 3] {
    [a[0] * s, a[1] * s, a[2] * s]
}

/// Linear interpolation `a + t·(b − a)`.
fn lerp3(a: [f64; 3], b: [f64; 3], t: f64) -> [f64; 3] {
    add3(a, scale3(sub3(b, a), t))
}

/// The dot product.
fn dot3(a: &[f64; 3], b: &[f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// The cross product.
fn cross3(a: &[f64; 3], b: &[f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// The Euclidean norm.
fn norm3(a: &[f64; 3]) -> f64 {
    (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt()
}

/// The Euclidean distance.
fn distance3(a: [f64; 3], b: [f64; 3]) -> f64 {
    norm3(&sub3(a, b))
}

/// The unit direction of a nonzero finite vector.
fn unit3(a: [f64; 3]) -> Option<[f64; 3]> {
    let norm = norm3(&a);
    if !norm.is_finite() || norm == 0.0 {
        None
    } else {
        let inv = 1.0 / norm;
        Some([a[0] * inv, a[1] * inv, a[2] * inv])
    }
}
