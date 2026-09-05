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

//! CC-026-THICKNESS (CC program Phase C, spine S7 consumer; theory §7.1, with
//! §7.2–§7.3 DEFERRED): the conservative certified shell thickness `t_safe`.
//!
//! `shell(body, t)` needs no critical-parameter theory (theory §4.4): the
//! maximum shellable thickness of a rounded-offset body is the minimum of the
//! **focal** term (the first focal event of the offset faces — the `J_t =
//! 1 − 2Ht + Kt²` margin reaching the regularity floor [`config::CC_ETA_J`])
//! and the **bottleneck** term (half the certified distance between the
//! NON-ADJACENT source strata of the offset contact complex, the theory §7.1
//! `t ≤ d_min/2` collision bound). This module serves
//! `max_shell_thickness` with the conservative certified LOWER bound
//! `t_safe = min(t_focal, d_min/2)`. No root finding, no semialgebraic
//! projection; the exact `valid_shell_interval` (theory §7.2–§7.3, the 5×5
//! systems) is DEFERRED and out of this packet.
//!
//! **Section 1 — the focal term** [`t_focal`]. Per Bézier patch over the
//! region, the two invariant enclosures `[H]` and `[K]` (mean and Gaussian
//! curvature of the admitted map, SIGNED against its oriented normal) are
//! composed DIRECTLY from the per-patch first/second-derivative hull
//! enclosures (the CC-002 discipline, the `hull.rs` kernels — NO second-form
//! module, NO principal-curvature extraction): the interval first-form
//! entries `E, F, G` from the first-partial hulls, the interval normal
//! `n̂ = (Sᵤ × Sᵥ)/‖Sᵤ × Sᵥ‖` from the cross-product hulls against a certified
//! determinant enclosure `W = ‖Sᵤ × Sᵥ‖²` whose positive lower endpoint is
//! the region's certified rank margin squared (`σ²`, from the map's own
//! `rank_margin`) and whose upper endpoint is the certified sup of the
//! squared normal (directed-rounded component sups), and the interval
//! second-form entries `L, M, N` from the second-partial hulls against that
//! normal. The focal quadratic's coefficient intervals are then `A = [K]`,
//! `B = −2·[H]`, `C = 1`; the admissible `t`-set is solved over the
//! coefficient box in closed form. Every fixed `t ≥ 0` focal quadratic is
//! linear in `(A, B)` with non-negative coefficients `(t², t)`, so the lower
//! envelope of the box is attained at the corner `(A_lo, B_lo) =
//! (K.lo, −2·H.hi)` and the patch's bound is that corner's first non-negative
//! threshold crossing of `A·t² + B·t + C − CC_ETA_J` — one closed-form
//! interval-quadratic solve per patch, covering the upward `K.lo > 0` case,
//! the concave `K.lo < 0` case, and the degenerate linear `K.lo = 0`
//! (`0 ∈ [K]`) case (theory §7.1). Intersecting over all patches gives the
//! certified focal bound `t_focal.lo` (rounded DOWN), and the region's
//! certified mean-curvature lower bound `H.lo > 0` gives the certified upper
//! bound `t_focal.hi = 1/H.lo` of the same focal event (rounded UP). A flat or
//! concave face (`H.lo ≤ 0`, no positive focal event along the offset
//! direction) reports `[+∞, +∞]` — the flat `J_t = 1` convention.
//!
//! **Section 2 — the bottleneck term** [`d_min_over_nonadjacent`]. The
//! certified minimum distance between NON-ADJACENT source strata; adjacent
//! pairs are the seams of the CC-022 [`GluePlan`] (they are handled by the
//! local star certificates of theory §4.1 — the exclusion is pinned by the
//! `non_adjacent_exclusion_matches_star_glue_plan` test). Every retained pair
//! is lower-bounded through the landed `Bvh::distance_lower_bound` over the
//! strata's source control boxes; where that query certifies a finite bound
//! (overlapping root boxes) the finite value is used, and where it certifies
//! the pair unboundedly separated (`+∞`, the disjoint-root sentinel) the
//! pair's certified lower bound is recovered as the axis-gap of the two
//! control boxes rounded DOWN (the bvh's own root-box primitive; reach bounds
//! are NOT subtracted — `d_min` is over SOURCE strata, per theory §7.1). The
//! minimum over all retained pairs is returned; no non-adjacent pair (fewer
//! than two strata, or every pair glued) returns `+∞` — no bottleneck.
//!
//! **Section 3 — the bound** [`t_safe`]. The certified lower bound
//! `min(t_focal_lower, d_min/2)` (the minimum rounded DOWN). When the
//! bottleneck sits strictly inside the certified focal enclosure
//! (`t_focal.lo < d_min/2 < t_focal.hi`) the ordering of the two events is
//! not generically decidable on the v1 evidence — an enclosure straddling the
//! minimum — and the construction refuses
//! [`ConstructRefusal::NonGenericThicknessEvent`] (the exact
//! `valid_shell_interval` of theory §7.2–§7.3 is deferred).
//!
//! **House rules (H-1).** This module carries no `unwrap`, no `expect`, and
//! no `panic!`, and adds no module-level `allow`. Every float reduction runs
//! in a fixed order with directed rounding (C9). The focal term performs
//! exactly ONE closed-form interval-quadratic solve per patch (the binding
//! coefficient corner `(K.lo, −2·H.hi)` of the interval quadratic), so it is
//! `O(N)` solves over the region's `N` patches — the per-patch solve count is
//! recorded in RESULT.

use crate::certified_map::{CertifiedCurveMap, CertifiedSurfaceMap, SurfaceRegion};
use crate::construct::config::CC_ETA_J;
use crate::construct::offset_strata::OffsetStratum;
use crate::construct::refusal::ConstructRefusal;
use crate::construct::stars::GluePlan;
use crate::construct::Interval;
use crate::hull::{bernstein_derivative_2d, hull_bernstein_2d};
use truck_base::bounding_box::BoundingBox;
use truck_base::bvh::{BoundedPiece, Bvh, DerivativeBounds};
use truck_base::cgmath64::Point3;

/// The certified lower bound of the focal term: the admissible offset `t` from
/// the focal-quadratic margin `1 − 2Ht + Kt² ≥ CC_ETA_J`.
///
/// Per Bézier patch of the map overlapping `sub`, the invariant enclosures
/// `[H]` and `[K]` are composed directly from the per-patch derivative hulls
/// (Section 1) and the focal quadratic's admissible `t`-set is solved over its
/// binding coefficient corner in closed form (one interval-quadratic solve per
/// patch); the certified lower bound of the region's first focal event is the
/// minimum over the patches of those solves, rounded DOWN. The returned
/// interval's lower endpoint is
/// the certified lower bound `t_focal_lower` of the Section 3 minimum; its
/// upper endpoint is the certified upper bound `1/H.lo` (rounded UP) of the
/// SAME focal event whenever the region's certified mean-curvature lower bound
/// `H.lo` is strictly positive, and `+∞` otherwise. A region with no positive
/// focal event along the offset direction (flat or concave, `H.lo ≤ 0`)
/// returns `[+∞, +∞]` — no focal constraint.
///
/// The certified composition requires the surface's parameter immersion to
/// certify strictly positive over the region (the certified rank margin `σ`
/// must be positive); a region whose margin cannot be proven positive refuses
/// [`ConstructRefusal::InvalidInput`]. A non-compact, misordered, or
/// outside-domain `sub` is an invalid request.
pub fn t_focal(
    map: &CertifiedSurfaceMap,
    sub: SurfaceRegion,
) -> Result<Interval, ConstructRefusal> {
    let domain = surface_domain(map)?;
    check_compact_region(domain, sub)?;
    let margin = map
        .rank_margin(sub)
        .map_err(|_| ConstructRefusal::InvalidInput)?;
    let sigma = margin.lo;
    if !(sigma.is_finite() && sigma > 0.0) {
        return Err(ConstructRefusal::InvalidInput);
    }
    let boxes = map.patch_boxes();
    let grids = map.patch_grids();
    if boxes.is_empty() || grids.is_empty() {
        return Err(ConstructRefusal::InvalidInput);
    }
    let mut focal_lo = f64::INFINITY;
    let mut h_lo = f64::INFINITY;
    let mut any_patch = false;
    for (patch_box, patch_grids) in boxes.iter().zip(grids.iter()) {
        let Some(overlap) = rectangle_overlap(*patch_box, sub) else {
            continue;
        };
        any_patch = true;
        let (u0, u1) = patch_box.0;
        let (v0, v1) = patch_box.1;
        let width_u = u1 - u0;
        let width_v = v1 - v0;
        if !width_u.is_finite() || !width_v.is_finite() || width_u <= 0.0 || width_v <= 0.0 {
            return Err(ConstructRefusal::InvalidInput);
        }
        let inv_u = 1.0 / width_u;
        let inv_v = 1.0 / width_v;
        let (s_lo, s_hi) = unit_image(patch_box.0, overlap.0)?;
        let (t_lo, t_hi) = unit_image(patch_box.1, overlap.1)?;
        let (h_iv, k_iv) =
            patch_invariants(patch_grids, (s_lo, s_hi), (t_lo, t_hi), inv_u, inv_v, sigma)?;
        h_lo = h_lo.min(h_iv.lo);
        let patch_lo = patch_focal_lo(k_iv, h_iv);
        focal_lo = focal_lo.min(patch_lo);
    }
    if !any_patch {
        return Err(ConstructRefusal::InvalidInput);
    }
    if !focal_lo.is_finite() {
        return Ok(Interval {
            lo: f64::INFINITY,
            hi: f64::INFINITY,
        });
    }
    let lo = focal_lo.max(0.0).next_down();
    let hi = if h_lo.is_finite() && h_lo > 0.0 {
        (1.0 / h_lo).next_up()
    } else {
        f64::INFINITY
    };
    let hi = if hi < lo { lo } else { hi };
    Ok(Interval { lo, hi })
}

/// The certified minimum distance between NON-ADJACENT source strata.
///
/// Every unordered pair of strata `(i, j)` with `i < j` whose two members are
/// NOT identified by a seam of the CC-022 `glue` plan is a retained pair; the
/// returned value is a certified LOWER bound of the minimum over those pairs
/// of the distance between the source features. Per pair, two single-piece
/// BVHs over the strata's source control boxes are compared with the landed
/// `Bvh::distance_lower_bound`; the finite value of that query is the pair's
/// certified lower bound, and a `+∞` disjoint-root answer (the source boxes
/// certifiably separated) falls back to the certified axis-gap lower bound of
/// the two control boxes (rounded DOWN) — the bvh's own root-box primitive,
/// so the pair bound stays a certified lower bound of the true source
/// distance. Reach bounds are NOT subtracted: `d_min` is over SOURCE strata,
/// per theory §7.1.
///
/// No non-adjacent pair exists (fewer than two strata, or every pair glued)
/// returns `+∞` — no certified bottleneck. A seam referencing an out-of-range
/// stratum, or a stratum whose source carrier cannot certify its control box,
/// is an invalid request.
pub fn d_min_over_nonadjacent(
    strata: &[OffsetStratum],
    glue: &GluePlan,
) -> Result<f64, ConstructRefusal> {
    if strata.len() < 2 {
        return Ok(f64::INFINITY);
    }
    let boxes: Vec<[Interval; 3]> = strata
        .iter()
        .map(stratum_source_box)
        .collect::<Result<Vec<_>, _>>()?;
    let count = strata.len();
    let mut adjacent = vec![false; count * count];
    for seam in &glue.seams {
        let a = seam.a.stratum;
        let b = seam.b.stratum;
        if a >= count || b >= count {
            return Err(ConstructRefusal::InvalidInput);
        }
        if a != b {
            adjacent[a * count + b] = true;
            adjacent[b * count + a] = true;
        }
    }
    let mut min = f64::INFINITY;
    for i in 0..count {
        for j in (i + 1)..count {
            if adjacent[i * count + j] {
                continue;
            }
            let pair = pair_distance_lower_bound(&boxes[i], &boxes[j]);
            min = min.min(pair);
        }
    }
    if !min.is_finite() {
        return Ok(f64::INFINITY);
    }
    Ok(min.max(0.0).next_down())
}

/// The conservative certified shell thickness bound `t_safe = min(t_focal,
/// d_min/2)`, as a certified LOWER bound (the minimum rounded DOWN).
///
/// `map` is the boundary face whose focal term bounds the offset, and
/// `strata`/`glue` carry the offset contact-complex strata and their CC-022
/// glue plan for the bottleneck term. The certified focal lower bound
/// `t_focal_lower = t_focal(map, ·).lo` and the certified half-gap
/// `d_min_over_nonadjacent(strata, glue) / 2` decide the Section 3 minimum:
///
/// - the bottleneck sits at or below the certified focal lower bound
///   (`d_min/2 ≤ t_focal.lo`): the bottleneck term binds and the bound is
///   `d_min/2`;
/// - the bottleneck sits at or above the certified focal upper bound
///   (`d_min/2 ≥ t_focal.hi`): the focal term binds and the bound is
///   `t_focal.lo`;
/// - the bottleneck lies strictly INSIDE the certified focal enclosure
///   (`t_focal.lo < d_min/2 < t_focal.hi`): the minimum straddles the
///   evidence and the ordering of the two events is not generically decidable
///   on the v1 evidence — the construction refuses
///   [`ConstructRefusal::NonGenericThicknessEvent`];
/// - an unbounded focal term (`t_focal.lo = +∞`, a flat or concave face)
///   leaves the bottleneck alone, and an unbounded bottleneck (`d_min = +∞`,
///   no non-adjacent pair) leaves the focal term alone.
///
/// The returned interval is the certified lower bound with directed rounding
/// DOWN on its lower endpoint (the H-3 float gate applies in tests); its upper
/// endpoint is the same certified bound, rounded UP.
pub fn t_safe(
    map: &CertifiedSurfaceMap,
    strata: &[OffsetStratum],
    glue: &GluePlan,
) -> Result<Interval, ConstructRefusal> {
    let domain = surface_domain(map)?;
    let focal = t_focal(map, domain)?;
    let d_min = d_min_over_nonadjacent(strata, glue)?;
    let gap = if d_min.is_infinite() {
        f64::INFINITY
    } else {
        d_min / 2.0
    };
    let scalar = t_safe_scalar(focal, gap)?;
    Ok(certified_scalar_interval(scalar))
}

/// The certified scalar lower bound of the Section 3 minimum.
fn t_safe_scalar(focal: Interval, gap: f64) -> Result<f64, ConstructRefusal> {
    let (lo, hi) = (focal.lo, focal.hi);
    if !lo.is_finite() {
        return Ok(gap);
    }
    if !gap.is_finite() {
        return Ok(lo);
    }
    if gap <= lo {
        return Ok(gap);
    }
    if gap >= hi {
        return Ok(lo);
    }
    Err(ConstructRefusal::NonGenericThicknessEvent)
}

/// Package a certified scalar lower bound as a directed-rounding interval.
fn certified_scalar_interval(scalar: f64) -> Interval {
    if scalar.is_finite() {
        let lo = if scalar > 0.0 {
            scalar.next_down()
        } else {
            scalar
        };
        Interval {
            lo,
            hi: scalar.next_up(),
        }
    } else {
        Interval {
            lo: f64::INFINITY,
            hi: f64::INFINITY,
        }
    }
}

/// The certified per-patch invariant pair `(H, K)` — the interval enclosures
/// of the mean and Gaussian curvature over the unit subbox `(s, t)`, SIGNED
/// against the map's oriented normal.
///
/// Direct composition from the per-patch derivative hulls (Section 1): the
/// first partials hull to the interval first-form entries `E = Sᵤ·Sᵤ`,
/// `F = Sᵤ·Sᵥ`, `G = Sᵥ·Sᵥ`; the interval normal is the cross-product hull
/// divided by `√W`, where the certified determinant enclosure
/// `W = ‖Sᵤ × Sᵥ‖²` carries the region's certified rank-margin lower bound
/// `σ²` (the map's own `rank_margin`, so the determinant never needs an
/// interval square of a sign-straddling cross component) and the certified sup
/// of the squared normal as its upper endpoint; the second partials hull to
/// the interval second-form entries `L = Sᵤᵤ·n̂`, `M = Sᵤᵥ·n̂`,
/// `N = Sᵥᵥ·n̂`. Then `H = (E·N − 2·F·M + G·L) / (2·W)` and
/// `K = (L·N − M·M) / W`, every step through the landed `Interval` arithmetic
/// (outward-directed). A patch whose second-partial coefficient grids are
/// EXACTLY zero contributes `L = M = N = 0` exactly (the CC-002 flatness
/// discipline — rounding slivers are never curvature), so a flat face returns
/// `H = K = 0` exactly.
fn patch_invariants(
    grids: &[Vec<Vec<f64>>; 3],
    s: (f64, f64),
    t: (f64, f64),
    inv_u: f64,
    inv_v: f64,
    sigma: f64,
) -> Result<(Interval, Interval), ConstructRefusal> {
    let mut su = [Interval::point(0.0); 3];
    let mut sv = [Interval::point(0.0); 3];
    for (k, grid) in grids.iter().enumerate() {
        let du = scaled_derivative_2d(grid, 0, inv_u);
        let dv = scaled_derivative_2d(grid, 1, inv_v);
        su[k] = hull_bernstein_2d(&du, s, t).map_err(|_| ConstructRefusal::InvalidInput)?;
        sv[k] = hull_bernstein_2d(&dv, s, t).map_err(|_| ConstructRefusal::InvalidInput)?;
    }
    let e = dot3(&su, &su);
    let f = dot3(&su, &sv);
    let g = dot3(&sv, &sv);
    let cross = [
        su[1].mul(&sv[2]).sub(&su[2].mul(&sv[1])),
        su[2].mul(&sv[0]).sub(&su[0].mul(&sv[2])),
        su[0].mul(&sv[1]).sub(&su[1].mul(&sv[0])),
    ];
    let w = determinant_enclosure(&cross, sigma)?;
    let sqrt_w = w.sqrt().ok_or(ConstructRefusal::InvalidInput)?;
    let normal = [
        cross[0]
            .div(&sqrt_w)
            .ok_or(ConstructRefusal::InvalidInput)?,
        cross[1]
            .div(&sqrt_w)
            .ok_or(ConstructRefusal::InvalidInput)?,
        cross[2]
            .div(&sqrt_w)
            .ok_or(ConstructRefusal::InvalidInput)?,
    ];
    let suu = second_partial_vector(grids, 0, 0, inv_u * inv_u, s, t)?;
    let suv = second_partial_vector(grids, 0, 1, inv_u * inv_v, s, t)?;
    let svv = second_partial_vector(grids, 1, 1, inv_v * inv_v, s, t)?;
    let l = dot3(&suu, &normal);
    let m = dot3(&suv, &normal);
    let n = dot3(&svv, &normal);
    let two = Interval::point(2.0);
    let h_num = e.mul(&n).add(&g.mul(&l)).sub(&two.mul(&f).mul(&m));
    let h = h_num
        .div(&two.mul(&w))
        .ok_or(ConstructRefusal::InvalidInput)?;
    let k_num = l.mul(&n).sub(&m.mul(&m));
    let k = k_num.div(&w).ok_or(ConstructRefusal::InvalidInput)?;
    if !h.is_finite() || !k.is_finite() {
        return Err(ConstructRefusal::InvalidInput);
    }
    Ok((h, k))
}

/// The certified determinant enclosure `W = ‖Sᵤ × Sᵥ‖²` over the patch: the
/// interval from the region's certified rank-margin square `σ²` (lower) to the
/// certified sup of the squared normal, `Σ sup_k²` with every square and sum
/// directed-rounded UP (upper). The lower endpoint is positive exactly when
/// `σ > 0`, so `√W` and every division by `W` stay certified.
fn determinant_enclosure(cross: &[Interval; 3], sigma: f64) -> Result<Interval, ConstructRefusal> {
    if !(sigma.is_finite() && sigma > 0.0) {
        return Err(ConstructRefusal::InvalidInput);
    }
    let lower = Interval::point(sigma).mul(&Interval::point(sigma));
    let mut upper = 0.0_f64;
    for component in cross {
        if !component.is_finite() {
            return Err(ConstructRefusal::InvalidInput);
        }
        let max_abs = component.lo.abs().max(component.hi.abs());
        let square = (max_abs * max_abs).next_up();
        if !square.is_finite() {
            return Err(ConstructRefusal::InvalidInput);
        }
        upper = (upper + square).next_up();
    }
    if !upper.is_finite() {
        return Err(ConstructRefusal::InvalidInput);
    }
    Ok(Interval {
        lo: lower.lo,
        hi: upper.max(lower.lo),
    })
}

/// The certified lower bound of the patch's first focal event: the closed-form
/// solve of the focal quadratic's binding coefficient corner.
///
/// The coefficient intervals are `A = [K]`, `B = −2·[H]`, `C = 1 − CC_ETA_J`.
/// Every fixed `t ≥ 0` focal quadratic is linear in `(A, B)` with non-negative
/// coefficients `(t², t)`, so the minimum over the coefficient box — the
/// certified lower envelope of the patch's focal quadratics — is attained at
/// the corner `(A_lo, B_lo) = (K.lo, −2·H.hi)`, and the patch's admissible
/// `t`-set is the interval up to that corner's first non-negative threshold
/// crossing of `g(t) = A_lo·t² + B_lo·t + C` (the remaining corners are
/// dominated by the envelope, so this single interval-quadratic solve is the
/// patch's whole closed-form solve — the `O(N)` per-patch count). The closed
/// form covers the degenerate `0 ∈ [K]` case (`A_lo = 0` takes the linear
/// root), the concave `A_lo < 0` case (its single positive root), and the
/// upward `A_lo > 0` case (its smaller positive root); `+∞` when the envelope
/// never drops below the floor (a flat or concave face, no focal event along
/// the offset direction).
fn patch_focal_lo(k_iv: Interval, h_iv: Interval) -> f64 {
    let c = 1.0 - CC_ETA_J;
    first_threshold_crossing(k_iv.lo, -2.0 * h_iv.hi, c)
}

/// The first non-negative crossing of `g(t) = a·t² + b·t + c` (with
/// `g(0) = c > 0`), or `+∞` when `g` never drops below zero for `t ≥ 0`.
///
/// Solved in the numerically stable form `2c/(−b + √(b² − 4ac))`, which avoids
/// the cancellation of the textbook root formula when the quadratic is
/// near-parabolic (the degenerate `0 ∈ [K]` corner, where `a` is a signed
/// rounding sliver around zero).
fn first_threshold_crossing(a: f64, b: f64, c: f64) -> f64 {
    if a == 0.0 {
        if b >= 0.0 {
            return f64::INFINITY;
        }
        return -c / b;
    }
    let disc = b * b - 4.0 * a * c;
    if disc <= 0.0 {
        return f64::INFINITY;
    }
    if a > 0.0 && b >= 0.0 {
        return f64::INFINITY;
    }
    let root = 2.0 * c / (-b + disc.sqrt());
    if root.is_finite() && root > 0.0 {
        root
    } else {
        f64::INFINITY
    }
}

/// The certified lower bound of the distance between two source control boxes,
/// through the landed piece-set distance query.
///
/// The two boxes enter as single-piece BVHs; the landed
/// `Bvh::distance_lower_bound` certifies the finite minimum leaf-box gap when
/// the roots overlap and the `+∞` disjoint-root sentinel when they do not. A
/// finite answer is the pair's certified lower bound; a disjoint answer falls
/// back to the certified axis-gap lower bound of the two boxes (rounded DOWN)
/// — the bvh's own root-box primitive — so the returned value never exceeds
/// the true minimum distance between the two source features.
fn pair_distance_lower_bound(a: &[Interval; 3], b: &[Interval; 3]) -> f64 {
    let piece_a = StratumBoxPiece::new(box_bbox(a));
    let piece_b = StratumBoxPiece::new(box_bbox(b));
    let bvh_a = Bvh::build(&[piece_a]);
    let bvh_b = Bvh::build(&[piece_b]);
    let queried = bvh_a.distance_lower_bound(&bvh_b);
    if queried.is_finite() {
        queried
    } else {
        axis_gap_lower_bound(a, b)
    }
}

/// The certified source control box of a stratum: the certified enclosure of
/// its source carrier over its whole declared domain.
///
/// For the k=1 face stratum the carrier is the source face map over its full
/// domain; for the k=2 edge stratum it is the source spine over its full arc;
/// for the k=3 corner stratum it is the node centre enclosure. A stratum
/// whose carrier cannot certify an enclosure is an invalid request.
fn stratum_source_box(stratum: &OffsetStratum) -> Result<[Interval; 3], ConstructRefusal> {
    match stratum {
        OffsetStratum::Face { map, .. } => {
            let domain = surface_domain(map)?;
            map.enclosure(domain)
                .map_err(|_| ConstructRefusal::InvalidInput)
        }
        OffsetStratum::Edge { spine, .. } => {
            let domain = curve_domain(spine)?;
            spine
                .enclosure(domain)
                .map_err(|_| ConstructRefusal::InvalidInput)
        }
        OffsetStratum::Corner { node } => Ok(node.centre),
    }
}

/// One box piece feeding the landed BVH distance query.
#[derive(Debug, Clone, Copy)]
struct StratumBoxPiece {
    /// The piece's box.
    bbox: BoundingBox<Point3>,
}

impl StratumBoxPiece {
    /// Wrap a source control box (endpoints already certified outward).
    fn new(bbox: BoundingBox<Point3>) -> Self {
        StratumBoxPiece { bbox }
    }
}

impl BoundedPiece for StratumBoxPiece {
    fn bbox(&self) -> BoundingBox<Point3> {
        self.bbox
    }

    fn derivative_bounds(&self) -> DerivativeBounds {
        DerivativeBounds::new()
    }

    fn subdivide(&self) -> Vec<Self> {
        Vec::new()
    }
}

/// The axis-aligned bounding box of a certified source control box.
fn box_bbox(box3: &[Interval; 3]) -> BoundingBox<Point3> {
    let mut bbox = BoundingBox::new();
    let mut lo = [0.0_f64; 3];
    let mut hi = [0.0_f64; 3];
    for k in 0..3 {
        lo[k] = box3[k].lo;
        hi[k] = box3[k].hi;
    }
    bbox.push(Point3::new(lo[0], lo[1], lo[2]));
    bbox.push(Point3::new(hi[0], hi[1], hi[2]));
    bbox
}

/// A certified LOWER bound of the Euclidean distance between two boxes: the
/// per-axis gap (`0` when the axes touch or overlap), each square, sum and
/// root rounded DOWN (the `stars.rs` / bvh root-gap discipline).
fn axis_gap_lower_bound(a: &[Interval; 3], b: &[Interval; 3]) -> f64 {
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

/// The exact gap between two closed intervals on one axis (`0` when they touch
/// or overlap).
fn axis_gap(a: &Interval, b: &Interval) -> f64 {
    if a.hi < b.lo {
        b.lo - a.hi
    } else if b.hi < a.lo {
        a.lo - b.hi
    } else {
        0.0
    }
}

/// The interval dot product of two interval 3-vectors.
fn dot3(a: &[Interval; 3], b: &[Interval; 3]) -> Interval {
    a[0].mul(&b[0]).add(&a[1].mul(&b[1])).add(&a[2].mul(&b[2]))
}

/// A first-derivative Bernstein coefficient grid in SOURCE units along `axis`
/// (the CC-002 derived-grid discipline, re-derived here because the helper is
/// private to `injectivity.rs` / `offset_strata.rs`).
fn scaled_derivative_2d(grid: &[Vec<f64>], axis: usize, scale: f64) -> Vec<Vec<f64>> {
    bernstein_derivative_2d(grid, axis)
        .iter()
        .map(|row| row.iter().map(|c| c * scale).collect())
        .collect()
}

/// The per-coordinate interval enclosures of one second partial
/// `∂²/∂a∂b` in SOURCE units over the unit subbox. A partial whose derived
/// coefficient grids are EXACTLY zero contributes exact zeros (the CC-002
/// flatness discipline — rounding slivers are never curvature).
fn second_partial_vector(
    grids: &[Vec<Vec<f64>>; 3],
    axis_a: usize,
    axis_b: usize,
    scale: f64,
    s: (f64, f64),
    t: (f64, f64),
) -> Result<[Interval; 3], ConstructRefusal> {
    let mut coeffs: Vec<Vec<Vec<f64>>> = Vec::with_capacity(3);
    let mut flat = true;
    for grid in grids.iter() {
        let first = bernstein_derivative_2d(grid, axis_a);
        let second = bernstein_derivative_2d(&first, axis_b);
        let scaled: Vec<Vec<f64>> = second
            .iter()
            .map(|row| row.iter().map(|c| c * scale).collect())
            .collect();
        if scaled.iter().any(|row| row.iter().any(|c| *c != 0.0)) {
            flat = false;
        }
        coeffs.push(scaled);
    }
    let mut out = [Interval::point(0.0); 3];
    if flat {
        return Ok(out);
    }
    for (cell, grid) in out.iter_mut().zip(coeffs.iter()) {
        *cell = hull_bernstein_2d(grid, s, t).map_err(|_| ConstructRefusal::InvalidInput)?;
    }
    Ok(out)
}

/// The declared domain rectangle of a surface map, derived from its patch
/// table; `None`-free (an empty patch table is an invalid request).
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

/// The declared domain interval of a curve map, derived from its piece table.
fn curve_domain(map: &CertifiedCurveMap) -> Result<(f64, f64), ConstructRefusal> {
    let intervals = map.piece_intervals();
    let first = match intervals.first() {
        Some(piece) => *piece,
        None => return Err(ConstructRefusal::InvalidInput),
    };
    let last = match intervals.last() {
        Some(piece) => *piece,
        None => return Err(ConstructRefusal::InvalidInput),
    };
    Ok((first.0, last.1))
}

/// Whether `sub` is a compact, ordered subset of `domain` (inclusive edges).
fn check_compact_region(domain: SurfaceRegion, sub: SurfaceRegion) -> Result<(), ConstructRefusal> {
    check_compact_axis(domain.0, sub.0)?;
    check_compact_axis(domain.1, sub.1)
}

/// Whether `sub` is a compact, ordered subset of one axis of `domain`.
fn check_compact_axis(domain: (f64, f64), sub: (f64, f64)) -> Result<(), ConstructRefusal> {
    if sub.0.is_finite()
        && sub.1.is_finite()
        && domain.0 <= sub.0
        && sub.0 <= sub.1
        && sub.1 <= domain.1
    {
        Ok(())
    } else {
        Err(ConstructRefusal::InvalidInput)
    }
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
