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

//! CC-021-OFFSET-STRATA (CC program Phase C, spine S10/S11 consumers; theory
//! §3.3, §3.4, §4.1): the rounded offset of a solid IS the constant-radius
//! rolling-ball contact complex, and its boundary is stratified by the
//! dimension of the nearest source feature:
//!
//! - **k = 1 — face strata**: regions of the offset parallel to an admitted
//!   source face, at signed offset distance `t`. The offset map over the face
//!   is `X_t = S + t·n̂` with Jacobian determinant `J_t = 1 − 2Ht + Kt²` (the
//!   shape-operator identity `J_t = det(I − tW) = (1 − tκ₁)(1 − tκ₂)`), so the
//!   stratum exists only where `J_t` certifies strictly positive — inside the
//!   focal set. [`OffsetStratum::Face`] carries the admitted face map and the
//!   certified enclosure of `J_t` over the face.
//! - **k = 2 — edge strata**: canal surfaces over the source solid's edges —
//!   the constant-radius envelope of the spheres of radius `|t|` centred on an
//!   admitted spine curve. [`OffsetStratum::Edge`] carries the spine map, the
//!   constant radius, and the S10 [`canal_regularity`] criterion value over the
//!   edge arc.
//! - **k = 3 — corner strata**: spherical patches at the P4-isolated centres —
//!   the isolated node where a ball of radius `|t|` touches three incident
//!   face regions (the S11 [`solve_triple_node`] outcome). [`OffsetStratum::Corner`]
//!   packages the [`TripleContactNode`] certificate directly.
//!
//! **Reach bounds.** Every stratum carries a certified reach bound
//! `ρ_A ≥ sup d(x, A)` over the stratum (theory §3.4). For the ball strata
//! (Face and Edge) the bound is exact: `ρ_A = |t|`, because every point of the
//! stratum is at offset distance exactly `|t|` from its source feature. For
//! the Corner the stratum is the certified ball around the isolated centre, so
//! `reach_bound` returns the certified centre-to-source bound carried by the
//! node's radius enclosure. That corner bound is a SOUND reach bound but not
//! the exact supremum: the corner sphere touches the three supports at
//! certified distance `|t|`, and the reach gate asserts the bound dominates the
//! distance from the node-centre enclosure to each support's bounding box — a
//! sound lower-bound sanity check, never an exactness claim. See
//! [`OffsetStratum::reach_bound`].
//!
//! **J_t without principal curvature extraction (k = 1).** The v1 Jacobian
//! certificate composes ONLY the landed map accessors and hull kernels — the
//! CC-002 discipline, never a second-form module (CC-026's booked deliverable).
//! Over the face's Bézier patches we bound the shape operator uniformly by
//! `‖W‖₂ ≤ ‖II‖_F / λ_min(I)` with `‖II‖_F` bounded from the second-derivative
//! hulls (`sup ‖S_uu‖`, `sup ‖S_uv‖`, `sup ‖S_vv‖`, exactly the CC-002 `‖D²S‖`
//! path) and `λ_min(I)` bounded from below by `σ²/(E + G)`, where `σ` is the
//! certified rank margin (`|S_u × S_v|`, the map's first-form immersion
//! certificate) and `E + G = ‖S_u‖² + ‖S_v‖²` is bounded from the first-form
//! derivative hulls. With the certified curvature magnitude bound `c` and
//! `r = |t|·c < 1` every eigenvalue `1 − tκᵢ` of the offset Jacobian lies in
//! `[1 − r, 1 + r]`, so `J_t` is enclosed by `[(1 − r)², (1 + r)²]` — the
//! certified lower endpoint is the stored `j_t_lower`. The bound is
//! deliberately sign-agnostic (no principal-curvature extraction), hence
//! conservative: a face whose certified margin cannot be proven strictly
//! positive — the certified `J_t` lower bound is at or below zero (the margin
//! straddles the focal threshold) — is refused [`ConstructRefusal::FocalDegeneracy`].
//! A flat face (`‖D²S‖ ≡ 0`) has `c = 0` and certifies `J_t = 1` exactly for
//! every offset.
//!
//! **Scope guards (stop conditions).** (1) The offset side signs `ε_i` are
//! caller-supplied per support — this packet never infers convexity from
//! geometry; when an input shape cannot carry per-support side signs the
//! caller-facing convention is recorded in the construction docs below. (2)
//! Sharp/concave completions — extending or intersecting offset faces — are
//! CC-024's, not this module's. (3) Strata are certified INDIVIDUALLY: stars,
//! broad phase, and embedding are CC-022/CC-023. This module never touches
//! those.
//!
//! **H-1.** This module carries no `unwrap`, no `expect`, and no `panic!`, and
//! adds no module-level `allow`. Every float reduction runs in a fixed order
//! with directed rounding (C9).

use crate::certified_map::{CertifiedCurveMap, CertifiedSurfaceMap, SurfaceRegion};
use crate::construct::canal::canal_regularity;
use crate::construct::contact3::{solve_triple_node, ReducedSystem, TripleNodeOutcome};
use crate::construct::refusal::ConstructRefusal;
use crate::construct::stubs::{RadiusLaw, TripleContactNode};
use crate::construct::Interval;
use crate::hull::{bernstein_derivative_2d, hull_bernstein_2d};
use crate::kernel::certs::IBox4;
use truck_base::evidence::Budget;

/// The k-th stratum of the rounded offset contact complex (theory §3.3).
///
/// Exactly three strata, distinguished by the dimension of the nearest source
/// feature: a face stratum (k = 1), an edge stratum (k = 2), and a corner
/// stratum (k = 3). Each variant carries its own certificate — the admitted
/// source carrier and the certified stratum witness — and [`OffsetStratum::reach_bound`]
/// reports the certified reach bound `ρ_A` over the stratum.
#[derive(Debug, Clone)]
pub enum OffsetStratum {
    /// The k = 1 face stratum: the parallel offset of an admitted source face.
    ///
    /// The offset map `X_t = S + t·n̂` over the face has Jacobian determinant
    /// `J_t = 1 − 2Ht + Kt²`; the stratum exists where the certified lower
    /// bound of `J_t` is strictly positive (the face is inside the focal set
    /// boundary). `j_t_lower` is the certified enclosure of `J_t` over the
    /// whole face, with `j_t_lower.lo > 0` the certified focal margin.
    Face {
        /// The admitted source face this stratum offsets.
        map: CertifiedSurfaceMap,
        /// The signed offset distance `t` along the map's oriented normal.
        offset: f64,
        /// The certified enclosure of `J_t` over the face; the lower endpoint
        /// is the certified focal margin (strictly positive on success).
        j_t_lower: Interval,
    },

    /// The k = 2 edge stratum: the canal surface over an admitted source edge.
    ///
    /// The spine is an edge curve of the source solid and the constant radius
    /// is the offset magnitude `|t|`; the S10 [`canal_regularity`] criterion
    /// over the whole edge arc certifies the canal's regularity, and its value
    /// is carried in `canal`.
    Edge {
        /// The admitted source edge spine this stratum offsets.
        spine: CertifiedCurveMap,
        /// The constant canal radius, exactly the offset magnitude `|t|`.
        radius: f64,
        /// The S10 certified canal-regularity criterion value over the edge
        /// arc (strictly positive lower endpoint on success).
        canal: Interval,
    },

    /// The k = 3 corner stratum: the spherical patch at the P4-isolated
    /// centre.
    ///
    /// The isolated node where a ball of radius `|t|` touches the three
    /// incident face regions, produced by the S11 [`solve_triple_node`]
    /// solve and packaged directly into the stratum.
    Corner {
        /// The certified triple-contact node (centre, radius, per-support
        /// contact parameters).
        node: TripleContactNode,
    },
}

/// The stratum kind carried by every [`StratumRefusal`]: which k-stratum was
/// being certified when the construction refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StratumId {
    /// The k = 1 face stratum.
    Face,
    /// The k = 2 edge stratum.
    Edge,
    /// The k = 3 corner stratum.
    Corner,
}

impl StratumId {
    /// The stable diagnostic tag of the stratum kind.
    pub fn tag(self) -> &'static str {
        match self {
            Self::Face => "k=1 face stratum",
            Self::Edge => "k=2 edge stratum",
            Self::Corner => "k=3 corner stratum",
        }
    }
}

/// A certified-construction refusal that names the stratum it arose from.
///
/// The construct-layer refusal vocabulary ([`ConstructRefusal`]) is frozen
/// (C4) and carries no payload, so each refusal here is wrapped together with
/// the [`StratumId`] of the stratum being built — a `FocalDegeneracy` that
/// names the face, a `CanalSingular` that names the edge, and so on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StratumRefusal {
    /// The stratum whose certification refused.
    pub stratum: StratumId,
    /// The construct-layer refusal.
    pub refusal: ConstructRefusal,
}

impl StratumRefusal {
    /// Build a stratum-named refusal.
    pub fn new(stratum: StratumId, refusal: ConstructRefusal) -> Self {
        StratumRefusal { stratum, refusal }
    }

    /// The stratum whose certification refused.
    pub fn stratum(self) -> StratumId {
        self.stratum
    }

    /// The construct-layer refusal.
    pub fn refusal(self) -> ConstructRefusal {
        self.refusal
    }
}

impl OffsetStratum {
    /// The stratum kind of this stratum.
    pub fn kind(&self) -> StratumId {
        match self {
            OffsetStratum::Face { .. } => StratumId::Face,
            OffsetStratum::Edge { .. } => StratumId::Edge,
            OffsetStratum::Corner { .. } => StratumId::Corner,
        }
    }

    /// The certified reach bound `ρ_A ≥ sup d(x, A)` over the stratum (theory
    /// §3.4).
    ///
    /// For the ball strata (Face and Edge) the bound is exact: `ρ_A = |t|`,
    /// the offset magnitude. For the Corner it is the certified centre-to-source
    /// bound carried by the node's radius enclosure (`node.radius.hi`): a sound
    /// reach bound, NOT the exact supremum — the corner sphere touches the
    /// three supports at the certified radius, and the reach gate asserts the
    /// bound dominates the distance from the node-centre enclosure to each
    /// support's bounding box, which is only a sound lower-bound sanity check.
    pub fn reach_bound(&self) -> f64 {
        match self {
            OffsetStratum::Face { offset, .. } => offset.abs(),
            OffsetStratum::Edge { radius, .. } => *radius,
            OffsetStratum::Corner { node } => node.radius.hi,
        }
    }
}

/// Build the k = 1 face stratum over an admitted source face.
///
/// `map` is the admitted source face (a `CertifiedSurfaceMap` over its whole
/// declared domain — the face), and `offset` is the signed offset distance `t`
/// along the map's oriented normal. The stratum certifies the offset Jacobian
/// `J_t = 1 − 2Ht + Kt²` from below over the whole face (the composition of
/// Section 2: first-form derivative hulls + the rank-margin `σ` + the CC-002
/// second-derivative hulls, WITHOUT principal-curvature extraction). When the
/// certified `J_t` lower bound is at or below zero — the face is at or past
/// the focal threshold under the certified curvature bound — the stratum
/// refuses [`ConstructRefusal::FocalDegeneracy`] naming the face stratum.
///
/// The offset side is the caller's: a `CertifiedSurfaceMap` carries no side
/// data, so the sign of `offset` (equivalently the per-face side sign `ε`) is
/// caller-supplied and this packet never infers convexity from geometry.
pub fn face_stratum(
    map: &CertifiedSurfaceMap,
    offset: f64,
) -> Result<OffsetStratum, StratumRefusal> {
    let err = |refusal| StratumRefusal::new(StratumId::Face, refusal);
    if !offset.is_finite() {
        return Err(err(ConstructRefusal::InvalidInput));
    }
    let j_t_lower = certified_jacobian_enclosure(map, offset).map_err(err)?;
    Ok(OffsetStratum::Face {
        map: map.clone(),
        offset,
        j_t_lower,
    })
}

/// Build the k = 2 edge stratum over an admitted source edge spine.
///
/// `spine` is the source solid edge as an admitted `CertifiedCurveMap` over
/// its whole domain (the edge arc), and `offset` supplies the offset
/// magnitude `|offset|`, which is the constant canal radius. The stratum
/// routes through the S10 [`canal_regularity`] criterion with the
/// [`RadiusLaw::Constant`] radius over the whole edge arc; a spine whose canal
/// is singular on the arc refuses [`ConstructRefusal::CanalSingular`] naming
/// the edge stratum. A zero or non-finite offset magnitude is an invalid
/// request (a zero-radius canal degenerates to the spine itself).
pub fn edge_stratum(
    spine: &CertifiedCurveMap,
    offset: f64,
) -> Result<OffsetStratum, StratumRefusal> {
    let err = |refusal| StratumRefusal::new(StratumId::Edge, refusal);
    if !offset.is_finite() {
        return Err(err(ConstructRefusal::InvalidInput));
    }
    let radius = offset.abs();
    if radius == 0.0 {
        return Err(err(ConstructRefusal::InvalidInput));
    }
    let intervals = spine.piece_intervals();
    if intervals.is_empty() {
        return Err(err(ConstructRefusal::InvalidInput));
    }
    let arc = (intervals[0].0, intervals[intervals.len() - 1].1);
    let law = RadiusLaw::Constant(radius);
    let canal = canal_regularity(spine, &law, arc).map_err(err)?;
    Ok(OffsetStratum::Edge {
        spine: spine.clone(),
        radius,
        canal,
    })
}

/// Build the k = 3 corner stratum over three incident face regions.
///
/// `maps`/`regions` are the three admitted face maps and their certified
/// regions (the faces incident at the corner, in one index order), `eps` the
/// caller-supplied signed offset sides (`+1` = centre on the `+n̂` side of the
/// support, `−1` otherwise), `offset` the offset magnitude `|offset|` (the
/// rolling-ball radius), `seed` the four-variable solve box `(c_x, c_y, c_z, r)`,
/// and `budget` the shared solve budget. The stratum routes through the S11
/// [`solve_triple_node`] solve (the [`ReducedSystem`] over the three flat
/// supports with the [`RadiusLaw::Constant`] closure): a certified
/// [`TripleNodeOutcome::Node`] packages directly into the Corner stratum; a
/// certified [`TripleNodeOutcome::Empty`] is a caller error — a corner was
/// expected — and refuses [`ConstructRefusal::InvalidInput`]. Solve refusals
/// (e.g. a structural `RankDeficientContact`, or a non-affine support
/// `InvalidInput`) propagate unchanged. The corner solve requires the flat
/// (affine-over-region) support class CC-020 scopes; curved-support corners
/// are a later packet's system.
pub fn corner_stratum(
    maps: [&CertifiedSurfaceMap; 3],
    regions: [SurfaceRegion; 3],
    eps: [f64; 3],
    offset: f64,
    seed: IBox4,
    budget: &mut Budget,
) -> Result<OffsetStratum, StratumRefusal> {
    let err = |refusal| StratumRefusal::new(StratumId::Corner, refusal);
    if !offset.is_finite() {
        return Err(err(ConstructRefusal::InvalidInput));
    }
    let radius = offset.abs();
    if radius == 0.0 {
        return Err(err(ConstructRefusal::InvalidInput));
    }
    let law = RadiusLaw::Constant(radius);
    let system = ReducedSystem::try_new(maps, regions, eps, &law, seed).map_err(err)?;
    let outcome = solve_triple_node(&system, budget).map_err(err)?;
    match outcome {
        TripleNodeOutcome::Node(node) => Ok(OffsetStratum::Corner { node }),
        TripleNodeOutcome::Empty => Err(err(ConstructRefusal::InvalidInput)),
    }
}

/// The certified enclosure of the offset Jacobian `J_t = 1 − 2Ht + Kt²` over
/// the whole face, with `lo > 0` on success.
///
/// Section 2 composition (no principal-curvature extraction, no second-form
/// module): per Bézier patch over the face,
///
/// - the first partials in source units are hulled over the patch subbox and
///   give a certified upper bound `T` of `‖S_u‖² + ‖S_v‖²` (first-form trace),
/// - the certified rank margin `σ` (the map's first-form immersion
///   certificate) lower-bounds `|S_u × S_v|` = `√det(I)`, so the first-form
///   smallest eigenvalue satisfies `λ_min(I) ≥ σ²/(E + G)`,
/// - the three second partials `S_uu`, `S_vv`, `S_uv` are hulled in source
///   units (the CC-002 `‖D²S‖` path) and give a certified upper bound
///   `Q = √(l_uu² + 2·l_uv² + l_vv²)` of `‖II‖_F`,
///
/// and the certified curvature-magnitude bound is `c = Q·T/σ²` (from
/// `‖W‖₂ ≤ ‖II‖_F/λ_min(I)`). With `r = |t|·c < 1` every offset-Jacobian
/// eigenvalue lies in `[1 − r, 1 + r]`, so `J_t` is enclosed by
/// `[(1 − r)², (1 + r)²]`. A certified `r ≥ 1` (or a non-positive lower
/// endpoint) means the certified margin straddles the focal threshold and the
/// face refuses [`ConstructRefusal::FocalDegeneracy`].
fn certified_jacobian_enclosure(
    map: &CertifiedSurfaceMap,
    offset: f64,
) -> Result<Interval, ConstructRefusal> {
    let region = surface_domain(map)?;
    let margin = map
        .rank_margin(region)
        .map_err(|_| ConstructRefusal::InvalidInput)?;
    let sigma = margin.lo;
    if !(sigma.is_finite() && sigma > 0.0) {
        return Err(ConstructRefusal::InvalidInput);
    }
    let (trace_sup, form_sup) = region_first_form_and_second_partials(map, region)?;
    let curvature = curvature_magnitude_bound(form_sup, trace_sup, sigma)?;
    jacobian_enclosure(offset, curvature)
}

/// The `(T, Q²)` region data for the Jacobian bound: `T` a certified upper
/// bound of `sup (‖S_u‖² + ‖S_v‖²)` and `Q²` a certified upper bound of
/// `sup (l_uu² + 2·l_uv² + l_vv²)` with the per-partial norm sups `l_uu`, `l_uv`,
/// `l_vv`. Reductions run in fixed order (patch order, then partial order
/// `uu`, `vv`, `uv`, then coordinate order).
fn region_first_form_and_second_partials(
    map: &CertifiedSurfaceMap,
    region: SurfaceRegion,
) -> Result<(f64, f64), ConstructRefusal> {
    let boxes = map.patch_boxes();
    let grids = map.patch_grids();
    let mut trace_sup = 0.0_f64;
    let mut form_sup = 0.0_f64;
    for (patch_box, patch_grids) in boxes.iter().zip(grids.iter()) {
        let Some(overlap) = rectangle_overlap(*patch_box, region) else {
            continue;
        };
        let (s_lo, s_hi) = unit_image(patch_box.0, overlap.0)?;
        let (t_lo, t_hi) = unit_image(patch_box.1, overlap.1)?;
        let width_u = patch_box.0 .1 - patch_box.0 .0;
        let width_v = patch_box.1 .1 - patch_box.1 .0;
        if !width_u.is_finite() || !width_v.is_finite() || width_u <= 0.0 || width_v <= 0.0 {
            return Err(ConstructRefusal::InvalidInput);
        }
        let inv_u = 1.0 / width_u;
        let inv_v = 1.0 / width_v;
        let (trace, q2) = patch_first_form_and_second_partials(
            patch_grids,
            (s_lo, s_hi),
            (t_lo, t_hi),
            inv_u,
            inv_v,
        )?;
        trace_sup = trace_sup.max(trace);
        form_sup = form_sup.max(q2);
    }
    if !trace_sup.is_finite() || !form_sup.is_finite() {
        return Err(ConstructRefusal::InvalidInput);
    }
    Ok((trace_sup, form_sup))
}

/// The per-patch `(trace, q2)` data: `trace` a certified upper bound of
/// `sup (‖S_u‖² + ‖S_v‖²)` over the unit subbox and `q2` the certified upper
/// bound of `l_uu² + 2·l_uv² + l_vv²` from the per-partial norm sups.
fn patch_first_form_and_second_partials(
    grids: &[Vec<Vec<f64>>; 3],
    s: (f64, f64),
    t: (f64, f64),
    inv_u: f64,
    inv_v: f64,
) -> Result<(f64, f64), ConstructRefusal> {
    let mut su = [Interval::point(0.0); 3];
    let mut sv = [Interval::point(0.0); 3];
    for (k, grid) in grids.iter().enumerate() {
        let du = scaled_derivative_2d(grid, 0, inv_u);
        let dv = scaled_derivative_2d(grid, 1, inv_v);
        su[k] = hull_bernstein_2d(&du, s, t).map_err(|_| ConstructRefusal::InvalidInput)?;
        sv[k] = hull_bernstein_2d(&dv, s, t).map_err(|_| ConstructRefusal::InvalidInput)?;
    }
    let mut trace = 0.0_f64;
    for k in 0..3 {
        trace = (trace + sup_square(&su[k])).next_up();
        trace = (trace + sup_square(&sv[k])).next_up();
    }
    let l_uu = second_partial_norm_sup(grids, 0, 0, inv_u * inv_u, s, t)?;
    let l_uv = second_partial_norm_sup(grids, 0, 1, inv_u * inv_v, s, t)?;
    let l_vv = second_partial_norm_sup(grids, 1, 1, inv_v * inv_v, s, t)?;
    // A flat partial reports `l = 0.0` EXACTLY; the accumulation must preserve
    // that zero so a fully flat patch reports `q2 = 0.0` exactly (the rounding
    // slivers of a squared zero must never become curvature).
    let mut q2 = 0.0_f64;
    if l_uu != 0.0 {
        q2 = (q2 + l_uu * l_uu).next_up();
    }
    if l_uv != 0.0 {
        q2 = (q2 + 2.0 * l_uv * l_uv).next_up();
    }
    if l_vv != 0.0 {
        q2 = (q2 + l_vv * l_vv).next_up();
    }
    if !trace.is_finite() || !q2.is_finite() {
        return Err(ConstructRefusal::InvalidInput);
    }
    Ok((trace, q2))
}

/// The certified sup of the square of a coordinate enclosure: the largest
/// endpoint magnitude squared, rounded upward.
fn sup_square(interval: &Interval) -> f64 {
    let max_abs = interval.lo.abs().max(interval.hi.abs());
    (max_abs * max_abs).next_up()
}

/// The certified sup of `‖partial‖` over the unit subbox for one second
/// partial `∂²/∂a∂b` in SOURCE units (`scale` = the inverse patch-width
/// product). A partial whose derived coefficient grids are EXACTLY zero
/// contributes zero (the CC-002 flatness discipline — rounding slivers are
/// never curvature).
fn second_partial_norm_sup(
    grids: &[Vec<Vec<f64>>; 3],
    axis_a: usize,
    axis_b: usize,
    scale: f64,
    s: (f64, f64),
    t: (f64, f64),
) -> Result<f64, ConstructRefusal> {
    let mut components = [Interval::point(0.0); 3];
    let mut flat = true;
    for (k, grid) in grids.iter().enumerate() {
        let first = bernstein_derivative_2d(grid, axis_a);
        let second = bernstein_derivative_2d(&first, axis_b);
        let coeffs: Vec<Vec<f64>> = second
            .iter()
            .map(|row| row.iter().map(|c| c * scale).collect())
            .collect();
        if coeffs.iter().any(|row| row.iter().any(|c| *c != 0.0)) {
            flat = false;
        }
        components[k] =
            hull_bernstein_2d(&coeffs, s, t).map_err(|_| ConstructRefusal::InvalidInput)?;
    }
    if flat {
        Ok(0.0)
    } else {
        norm_sup(&components)
    }
}

/// A first-derivative Bernstein coefficient grid in SOURCE units along `axis`
/// (the CC-002 `derived_grid_2d` scaling discipline, re-derived here because
/// the helper is private to `injectivity.rs`).
fn scaled_derivative_2d(grid: &[Vec<f64>], axis: usize, scale: f64) -> Vec<Vec<f64>> {
    bernstein_derivative_2d(grid, axis)
        .iter()
        .map(|row| row.iter().map(|c| c * scale).collect())
        .collect()
}

/// The certified sup of the Euclidean norm over the coordinate enclosure of a
/// vector-valued field: `√(Σ_k (max(|lo_k|, |hi_k|))²)`, every square, sum and
/// root rounded upward so the result certifies the norm's supremum.
fn norm_sup(components: &[Interval; 3]) -> Result<f64, ConstructRefusal> {
    let mut sum = 0.0_f64;
    for component in components {
        if !component.is_finite() {
            return Err(ConstructRefusal::InvalidInput);
        }
        let max_abs = component.lo.abs().max(component.hi.abs());
        let square = (max_abs * max_abs).next_up();
        sum = (sum + square).next_up();
    }
    if !sum.is_finite() {
        return Err(ConstructRefusal::InvalidInput);
    }
    Ok(sum.sqrt().next_up())
}

/// The certified curvature-magnitude bound `c = Q·T/σ²` over the region, with
/// `Q = √Q²` and `T`/`Q²` the region sups, every step directed-rounded so `c`
/// certifies `sup ‖W‖₂ ≤ sup ‖II‖_F / λ_min(I) ≤ Q·T/σ²`.
fn curvature_magnitude_bound(q2: f64, trace_sup: f64, sigma: f64) -> Result<f64, ConstructRefusal> {
    if q2 == 0.0 {
        return Ok(0.0);
    }
    let q = q2.sqrt().next_up();
    let sigma2 = Interval::point(sigma).mul(&Interval::point(sigma));
    if !sigma2.is_finite() || !(sigma2.lo > 0.0) {
        return Err(ConstructRefusal::InvalidInput);
    }
    let numer = Interval::point(q).mul(&Interval::point(trace_sup));
    let quotient = numer.div(&sigma2).ok_or(ConstructRefusal::InvalidInput)?;
    if !quotient.hi.is_finite() {
        return Err(ConstructRefusal::InvalidInput);
    }
    Ok(quotient.hi)
}

/// The certified `J_t` enclosure from the certified curvature-magnitude bound
/// `c` and the signed offset `t`: with `r = |t|·c < 1` every offset-Jacobian
/// eigenvalue lies in `[1 − r, 1 + r]`, so `J_t ∈ [(1 − r)², (1 + r)²]`. A
/// certified `r ≥ 1` — the certified focal margin `1/c` is at or below `|t|`,
/// so the `J_t` lower bound is at or below zero / straddles the focal
/// threshold — refuses [`ConstructRefusal::FocalDegeneracy`].
fn jacobian_enclosure(offset: f64, curvature: f64) -> Result<Interval, ConstructRefusal> {
    let magnitude = offset.abs();
    if !magnitude.is_finite() {
        return Err(ConstructRefusal::InvalidInput);
    }
    if curvature == 0.0 {
        return Ok(Interval { lo: 1.0, hi: 1.0 });
    }
    let r_iv = Interval::point(magnitude).mul(&Interval::point(curvature));
    if !r_iv.is_finite() {
        return Err(ConstructRefusal::InvalidInput);
    }
    if !(r_iv.hi < 1.0) {
        return Err(ConstructRefusal::FocalDegeneracy);
    }
    let one = Interval::point(1.0);
    let low = one.sub(&r_iv);
    let high = one.add(&r_iv);
    if !(low.lo > 0.0) {
        return Err(ConstructRefusal::FocalDegeneracy);
    }
    let lo = low.mul(&low).lo;
    let hi = high.mul(&high).hi;
    if !lo.is_finite() || !hi.is_finite() || !(lo > 0.0) || lo > hi {
        return Err(ConstructRefusal::InvalidInput);
    }
    Ok(Interval { lo, hi })
}

/// The declared domain rectangle of a surface map, derived from its patch
/// table (the first patch's lower corner to the last patch's upper corner).
fn surface_domain(map: &CertifiedSurfaceMap) -> Result<SurfaceRegion, ConstructRefusal> {
    let boxes = map.patch_boxes();
    let first = match boxes.first() {
        Some(box0) => *box0,
        None => return Err(ConstructRefusal::InvalidInput),
    };
    let last = match boxes.last() {
        Some(box1) => *box1,
        None => return Err(ConstructRefusal::InvalidInput),
    };
    Ok(((first.0 .0, last.0 .1), (first.1 .0, last.1 .1)))
}

/// The source-parameter overlap of `sub` with `patch_box`, per axis; `None`
/// when the rectangle is untouched.
fn rectangle_overlap(patch_box: SurfaceRegion, sub: SurfaceRegion) -> Option<SurfaceRegion> {
    let overlap_u = axis_overlap(patch_box.0, sub.0)?;
    let overlap_v = axis_overlap(patch_box.1, sub.1)?;
    Some((overlap_u, overlap_v))
}

/// The overlap of two closed intervals on one axis; `None` when disjoint.
fn axis_overlap(box_axis: (f64, f64), sub_axis: (f64, f64)) -> Option<(f64, f64)> {
    let (a0, a1) = box_axis;
    let (lo, hi) = sub_axis;
    if hi < a0 || lo > a1 {
        return None;
    }
    Some((lo.max(a0), hi.min(a1)))
}

/// The exact unit-parameter image of an overlap under the span's own
/// source-to-unit affine map, enclosed in `Interval` arithmetic and clamped to
/// `[0, 1]` (the `certified_map.rs` / `injectivity.rs` discipline, re-derived
/// because the helper is private to those modules).
fn unit_image(span: (f64, f64), overlap: (f64, f64)) -> Result<(f64, f64), ConstructRefusal> {
    let (a, b) = span;
    let (lo, hi) = overlap;
    let a_iv = Interval::point(a);
    let span_iv = Interval::point(b).sub(&a_iv);
    let lo_u = Interval::point(lo)
        .sub(&a_iv)
        .div(&span_iv)
        .ok_or(ConstructRefusal::InvalidInput)?;
    let hi_u = Interval::point(hi)
        .sub(&a_iv)
        .div(&span_iv)
        .ok_or(ConstructRefusal::InvalidInput)?;
    let u_lo = lo_u.lo.min(hi_u.lo).clamp(0.0, 1.0);
    let u_hi = lo_u.hi.max(hi_u.hi).clamp(0.0, 1.0);
    if u_lo.is_finite() && u_hi.is_finite() && u_lo <= u_hi {
        Ok((u_lo, u_hi))
    } else {
        Err(ConstructRefusal::InvalidInput)
    }
}
