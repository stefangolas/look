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

//! The §9.4 trim clip (BG-KV2-401-S3C): certified R9 crossings between an
//! arc's pcurve and the closed trim loops of the SAME chart, arc splitting at
//! those crossings, and inside/outside classification of the sub-arcs by the
//! winding number of the closed trim loop about one certified-off interior
//! sample (the SOUND use of §9 — a closed plane curve about a point certified
//! OFF the loop; not the rejected vector-field cell index). Outside sub-arcs
//! are discarded; the trim boundary endpoints become
//! [`TopoNode::TrimCrossing`] nodes, which increase graph valence and are not
//! [`SegmentBreak`]s.
//!
//! **Steps 3–6 of §9.4.** The clip runs between the certified leaf-product
//! 1-complex (the arcs of the input [`CertifiedGraph`]) and the trimmed faces:
//!
//! 3. Per arc's pcurve and per trim curve IN THE SAME CHART, certified R9
//!    crossings ([`R9System`] + [`krawczyk_c1`], the S1A seam) are isolated by
//!    subdivision to [`crate::kernel::config::DEPTH_MAX`]. Each is a
//!    [`CertifiedCrossing`]: an R9-stamped [`PointCert`] over the certified
//!    `(t_arc, r_trim)` box plus the certified chart point of the crossing.
//!    Every certified root of one (pcurve, trim) residual is unique per §8.2
//!    inside its own certified box and the subdivision boxes are pairwise
//!    disjoint, so two TrimCrossing nodes of one clip never witness the same
//!    root (§4.2 Rule A holds by construction).
//! 4. Arcs are split at the certified crossings.
//! 5. Each resulting sub-arc is classified inside or outside the trim interior
//!    by the winding number of the closed trim loop about one interior sample
//!    point of the sub-arc, where the sample's off-loop property is certified
//!    by R9 distance-positivity data ([`certify_off_loop`]: the cross-multiplied
//!    point-vs-trim residual components of the R9 family, certified to exclude
//!    zero). The winding is an exact integer ray-crossing count in the plane
//!    with certified sign discipline, on the certified Bernstein
//!    representation — polynomial arithmetic only.
//! 6. Outside sub-arcs are discarded; inside ones are retained. A retained
//!    sub-arc that ends at a trim boundary does so at a
//!    [`TopoNode::TrimCrossing`] node.
//!
//! **The named no-special-case case.** An interior loop of the 1-complex that
//! meets no leaf boundary but crosses a trim is handled by steps 3–6 with no
//! special case — the certified crossings split the closed loop of arcs and the
//! winding classifies each piece (the spec's "interior loop" fixture).
//!
//! **Failure.** A crossing isolation that genuinely fails at DEPTH_MAX refuses
//! [`RefusalKind::TrimClipFailed`] (Inconclusive) — the named refusal of §9.4.
//!
//! **H-1.** The crate-level `#![deny(clippy::unwrap_used)]` in `lib.rs` covers
//! this module. The module carries no `unwrap`, no `expect`, and no `panic!`
//! calls, and adds no module-level `allow`. Where a `Result` must carry the
//! frozen `Refusal` (which holds `Option<PartialGraph>`), the large-`Err` lint
//! is allowed item-level only, exactly as the shim files do.
//!
//! **N4 / bit-reproducibility.** This module performs no transcendental call:
//! no `sin`, `cos`, `atan2`, `exp`, `ln`, `log`, `powf`, and no `sqrt`
//! anywhere. Every certified quantity is a deterministic `CertifiedInterval`
//! sequence over the landed hull kernels ([`hull_bernstein_1d`]) plus exact
//! `i64` integer arithmetic — outward-rounded only. The winding sign discipline
//! is polynomial sign evaluation; no angle function appears.
//!
//! **The S3c pcurve seam (honest scope).** The frozen [`CertifiedGraph`] shape
//! (§16) records an arc's certified *trace* but carries no polynomial pcurve
//! leaf. §9.4 certifies each trim-arc event by R9, which consumes two certified
//! polynomial leaves ([`BezierLeaf1`]). Like `tier1::boundary_seeds` receiving
//! its edges as caller-supplied `(curve, chart)` leaf data rather than
//! enumerating B-rep edges (the leaf/atlas wave's contract), this packet does
//! NOT extract polynomial pcurves from topology. An arc of the input graph is
//! clippable in this packet exactly when its certified trace in the trim chart
//! is the STRAIGHT segment between its two certified chart endpoints, which
//! [`trim_clip`] recovers exactly as a degree-1 [`BezierLeaf1`] (the certified
//! planar leaf whose affine image is that segment). The fixtures that exercise
//! this packet — plane/plane and plane/linear traces, polygonal interior loops
//! — certify exactly such straight chart traces, so the recovery is exact for
//! them. An arc that lies in a trim chart but whose trace cannot be recovered
//! as one certified polynomial leaf refuses
//! [`RefusalKind::TrimClipFailed`] (Inconclusive): the module cannot certify
//! the R9 events of a curve it does not hold as a leaf.

use crate::hull::{bernstein_derivative_1d, hull_bernstein_1d};
use crate::kernel::certs::PointCert;
use crate::kernel::config::DEPTH_MAX;
use crate::kernel::engine::{krawczyk_c1, SquareResidualEval};
use crate::kernel::evidence::{ClaimVerdict, Construction, Refusal, RefusalEvidence, RefusalKind};
use crate::kernel::graph::{
    AnyArc, Approx, Arc, ArcEnd, ArcId, CertifiedGraph, ChartId, HermiteSegment, HermiteSpline,
    Node, NodeCert, NodeId, Param, Point4, TopoNode,
};
use crate::kernel::patch::{CertifiedPositive, IBox2};
use crate::kernel::residual::ResidualId;
use crate::kernel::residuals_r89::{BezierLeaf1, R9System};
use crate::kernel::Interval;

/// A closed trim loop of a face: a certified curve leaf in a lifted chart.
///
/// The caller supplies trim loops from the leaf/B-rep side; this packet does
/// not extract trims from topology (§9.4 step 2). A loop with `closed == true`
/// bounds a face region whose interior is classified by the winding number of
/// its closed trace about a certified-off sample. An arc is only ever compared
/// against a trim loop whose chart equals the arc pcurve's chart ("IN THE SAME
/// CHART", §9.4 step 3).
#[derive(Debug, Clone, PartialEq)]
pub struct TrimLoop {
    /// The lifted chart the trim curve lives in.
    pub chart: ChartId,
    /// The certified trim curve leaf (its parameter `r ∈ [0, 1]`).
    pub curve: BezierLeaf1,
    /// Whether the curve's trace is a closed loop (bounds a face region).
    pub closed: bool,
}

/// A certified §9.4 trim crossing of an arc pcurve against a trim curve.
///
/// The R9 C1 certificate ([`krawczyk_c1`] over the S1A seam) isolates a unique
/// root of `J(t, r) = C₁(t) − C₂(r)` in the certified `(t, r)` box; the box is
/// re-stamped with [`ResidualId::R9`] (the engine's documented one-line seam).
/// `point` is the certified chart point of the crossing: the affine image of
/// the certified `t`-midpoint on the arc pcurve leaf.
#[derive(Debug, Clone, PartialEq)]
pub struct CertifiedCrossing {
    /// The R9-stamped point certificate over the certified `(t_arc, r_trim)` box.
    pub cert: PointCert,
    /// The certified chart point `(u, v)` of the crossing.
    pub point: [f64; 2],
}

/// One certified exclusion box of an off-loop certificate.
///
/// Over the trim-parameter box `r`, the `component`-th cross-multiplied
/// point-vs-trim residual of the R9 family certifiably excludes zero with the
/// recorded image `separation`. Because the trim weights are certified
/// positive, that is a certified positive separation of the sample from the
/// loop in the `component` chart coordinate over the whole box.
#[derive(Debug, Clone, PartialEq)]
pub struct CertifiedExclusion {
    /// The trim-parameter sub-box `[lo, hi] ⊆ [0, 1]` the exclusion runs over.
    pub r: (f64, f64),
    /// The excluded residual component: `0` is the `u`-difference, `1` the
    /// `v`-difference.
    pub component: usize,
    /// The certified image of the excluded component over the box.
    pub separation: Interval,
}

/// The certificate that a chart sample point is OFF a trim curve.
///
/// The R9 distance-positivity data: over a partition of the trim parameter
/// `[0, 1]`, at least one cross-multiplied residual component excludes zero on
/// every box, so no trim parameter maps the curve onto the sample. This is the
/// precondition that makes the §9.4 winding-number classification SOUND (a
/// closed plane curve about a point certified OFF the loop).
#[derive(Debug, Clone, PartialEq)]
pub struct OffLoopCert {
    /// The chart sample point that was certified off the loop.
    pub sample: [f64; 2],
    /// The certified exclusion partition of the trim parameter.
    pub exclusions: Vec<CertifiedExclusion>,
}

/// A named predicate refusal: the kind names the violated class, the predicate
/// name the precise invariant.
fn refusal(kind: RefusalKind, name: &'static str, detail: String) -> Refusal {
    Refusal::new(kind, RefusalEvidence::Predicate { name, detail })
}

/// A caller-data refusal: the input violates a documented precondition.
fn caller_refusal(name: &'static str, detail: String) -> Refusal {
    refusal(RefusalKind::ClaimRefuted, name, detail)
}

/// The trim-clip refusal for a stalled isolation: a certified event could not
/// be isolated at DEPTH_MAX (§9.4's named `Refuse(TrimClipFailed)`,
/// Inconclusive).
fn depth_failure(name: &'static str, detail: String) -> Refusal {
    refusal(RefusalKind::TrimClipFailed, name, detail)
}

// ---------------------------------------------------------------------------
// Certified polynomial substrate over a 1-var leaf (the R9 chart curves)
// ---------------------------------------------------------------------------

/// The `comp`-coordinate Bernstein coefficients of a 1-var leaf (`0..=2` are
/// `x`, `y`, `z`; `3` is the weight `w`).
fn coeffs(leaf: &BezierLeaf1, comp: usize) -> Vec<f64> {
    leaf.control.iter().map(|p| p[comp]).collect()
}

/// The certified range of a Bernstein coefficient list over a `[0, 1]`
/// sub-interval, or the vacuous unbounded enclosure when the hull kernel
/// refuses.
fn hull1(coeffs: &[f64], sub: (f64, f64)) -> Interval {
    match hull_bernstein_1d(coeffs, sub) {
        Ok(hull) => hull,
        Err(_) => Interval {
            lo: f64::NEG_INFINITY,
            hi: f64::INFINITY,
        },
    }
}

/// Float de Casteljau evaluation of a Bernstein coefficient list at `t`.
fn de_casteljau_f64(coeffs: &[f64], t: f64) -> f64 {
    let mut level: Vec<f64> = coeffs.to_vec();
    let mt = 1.0 - t;
    while level.len() > 1 {
        let mut next = Vec::with_capacity(level.len() - 1);
        for pair in level.windows(2) {
            next.push(mt * pair[0] + t * pair[1]);
        }
        level = next;
    }
    level[0]
}

/// The affine chart point `(x, y) = (X(t)/W(t), Y(t)/W(t))` of a leaf, by
/// float de Casteljau. The certified leaf constructors guarantee a
/// strictly-positive weight field, so `W(t) > 0` and the division is
/// well-defined.
fn chart_point(leaf: &BezierLeaf1, t: f64) -> [f64; 2] {
    let x = de_casteljau_f64(&coeffs(leaf, 0), t);
    let y = de_casteljau_f64(&coeffs(leaf, 1), t);
    let w = de_casteljau_f64(&coeffs(leaf, 3), t);
    [x / w, y / w]
}

/// The affine chart point at the certified `t`-midpoint of a certified
/// `(t, r)` box (axis 0 is the arc pcurve parameter).
fn crossing_chart_point(leaf: &BezierLeaf1, box_: IBox2) -> [f64; 2] {
    let t = 0.5 * (box_.lo[0] + box_.hi[0]);
    chart_point(leaf, t)
}

/// `C(n, k)` as an `f64` (iterative; exact for the small degrees here).
fn binom(n: usize, k: usize) -> f64 {
    let k = k.min(n - k);
    let mut out = 1.0f64;
    for i in 0..k {
        out *= (n - i) as f64 / (i + 1) as f64;
    }
    out
}

/// The Bernstein coefficients of the product of two Bernstein polynomials.
///
/// If `a` has degree `m` and `b` degree `n`, the product has degree `m + n`
/// and coefficient `k` is `Σ_{i+j=k} (C(m,i)C(n,j)/C(m+n,k)) a_i b_j`. The
/// arithmetic is `f64`; the binomials are positive and the degrees are small.
fn bernstein_mul(a: &[f64], b: &[f64]) -> Vec<f64> {
    if a.is_empty() || b.is_empty() {
        return Vec::new();
    }
    let m = a.len() - 1;
    let n = b.len() - 1;
    let mut out = vec![0.0f64; m + n + 1];
    for (i, &ai) in a.iter().enumerate() {
        for (j, &bj) in b.iter().enumerate() {
            let w = binom(m, i) * binom(n, j) / binom(m + n, i + j);
            out[i + j] += w * ai * bj;
        }
    }
    out
}

/// Whether any residual component certifiably excludes zero over the box (a
/// sound no-root-in-box exclusion that prunes a subdivision tree).
fn residual_excludes_zero(sys: &dyn SquareResidualEval, b: &IBox2) -> bool {
    let box_iv: [Interval; 2] = [
        Interval {
            lo: b.lo[0],
            hi: b.hi[0],
        },
        Interval {
            lo: b.lo[1],
            hi: b.hi[1],
        },
    ];
    let h = sys.eval(&box_iv);
    h.iter().any(|component| !component.contains(0.0))
}

/// Bisect a 2-axis box along its widest axis (lowest-index tie-break,
/// deterministic order): returns the two closed half-boxes.
fn bisect2(b: &IBox2) -> Vec<IBox2> {
    let mut axis = 0usize;
    let mut width = b.hi[0] - b.lo[0];
    for i in 1..2 {
        let w = b.hi[i] - b.lo[i];
        if w > width {
            width = w;
            axis = i;
        }
    }
    if !(width.is_finite() && width > 0.0) {
        return Vec::new();
    }
    let mid = 0.5 * (b.lo[axis] + b.hi[axis]);
    let mut lo_child = *b;
    let mut hi_child = *b;
    lo_child.hi[axis] = mid;
    hi_child.lo[axis] = mid;
    vec![lo_child, hi_child]
}

/// The certified positive weight bound of a curve pair over `[0, 1]`: the
/// minimum control weight of either leaf. This is the §7.1 VALUE argument —
/// a certified lower bound because the Bernstein basis is non-negative and the
/// leaf constructors certify strictly-positive control weights.
// Refusal carries Option<PartialGraph> by frozen §2 shape; large-Err is allowed (BG-KV2-000).
#[allow(clippy::result_large_err)]
fn pair_weight_bound(a: &BezierLeaf1, b: &BezierLeaf1) -> Result<CertifiedPositive, Refusal> {
    let min_a = a.control.iter().map(|p| p[3]).fold(f64::INFINITY, f64::min);
    let min_b = b.control.iter().map(|p| p[3]).fold(f64::INFINITY, f64::min);
    CertifiedPositive::try_new(min_a.min(min_b))
}

// ---------------------------------------------------------------------------
// §9.4 step 3: certified R9 crossings
// ---------------------------------------------------------------------------

/// Isolate every certified R9 crossing of two 1-var curve leaves in ONE chart
/// over the full parameter square `[0, 1]²` (§9.4 step 3).
///
/// The two leaves are the arc pcurve (parameter `t`) and the trim curve
/// (parameter `r`). The square C1 ([`krawczyk_c1`], the S1A seam) certifies a
/// unique root per box; subdivision to
/// [`crate::kernel::config::DEPTH_MAX`] enumerates the roots. A box that
/// remains unresolved at DEPTH_MAX refuses [`RefusalKind::TrimClipFailed`]
/// (Inconclusive) — the named refusal of §9.4. The result is in ascending
/// certified arc-parameter order.
// Refusal carries Option<PartialGraph> by frozen §2 shape; large-Err is allowed (BG-KV2-000).
#[allow(clippy::result_large_err)]
pub fn certify_crossings(a: &BezierLeaf1, b: &BezierLeaf1) -> Construction<Vec<CertifiedCrossing>> {
    let sys = R9System::try_new(a, b)?;
    if a.chart != b.chart {
        return Err(caller_refusal(
            "r9_requires_one_chart",
            "certify_crossings requires the two curve leaves in the same lifted chart".to_string(),
        ));
    }
    let w = vec![pair_weight_bound(a, b)?];
    let root = IBox2::try_new([0.0, 0.0], [1.0, 1.0]).map_err(|_| {
        caller_refusal(
            "crossing_unit_box_refused",
            "the unit box refused".to_string(),
        )
    })?;
    let mut found: Vec<CertifiedCrossing> = Vec::new();
    let mut stack: Vec<(IBox2, u32)> = vec![(root, 0u32)];
    while let Some((box_, depth)) = stack.pop() {
        if residual_excludes_zero(&sys, &box_) {
            continue;
        }
        match krawczyk_c1(&sys, box_, &w) {
            ClaimVerdict::Proven(cert) => {
                // The engine stamps R1; rebuild through the documented one-line
                // seam with the R9 residual's own id.
                let cert = PointCert::try_new(ResidualId::R9, cert.box_, cert.rho)?;
                let point = crossing_chart_point(a, cert.box_);
                found.push(CertifiedCrossing { cert, point });
            }
            ClaimVerdict::Disproven(_) => {}
            ClaimVerdict::Inconclusive(_) => {
                if depth < DEPTH_MAX {
                    for child in bisect2(&box_) {
                        stack.push((child, depth + 1));
                    }
                } else {
                    return Err(depth_failure(
                        "trim_crossing_depth_max",
                        format!(
                            "an R9 crossing could not be isolated at DEPTH_MAX {DEPTH_MAX} \
                             (box {box_:?}); refusing TrimClipFailed (Inconclusive)"
                        ),
                    ));
                }
            }
        }
    }
    found.sort_by(|x, y| x.cert.box_.lo[0].total_cmp(&y.cert.box_.lo[0]));
    Ok(found)
}

// ---------------------------------------------------------------------------
// R9 distance-positivity: certifying a sample OFF the loop
// ---------------------------------------------------------------------------

/// The cross-multiplied point-vs-trim residual components of the R9 family:
/// the Bernstein coefficients of `c₀(r) = u₀·W(r) − X(r)` and
/// `c₁(r) = v₀·W(r) − Y(r)` for a sample `(u₀, v₀)` and a trim leaf
/// `(X, Y, W)`. With certified-positive weights these are the sign-equivalent,
/// non-divided R9 differences `(u₀ − x(r))·W(r)` and `(v₀ − y(r))·W(r)`.
fn point_trim_coeffs(sample: [f64; 2], leaf: &BezierLeaf1) -> [Vec<f64>; 2] {
    let x = coeffs(leaf, 0);
    let y = coeffs(leaf, 1);
    let w = coeffs(leaf, 3);
    let c0: Vec<f64> = w
        .iter()
        .zip(x.iter())
        .map(|(w, x)| sample[0] * w - x)
        .collect();
    let c1: Vec<f64> = w
        .iter()
        .zip(y.iter())
        .map(|(w, y)| sample[1] * w - y)
        .collect();
    [c0, c1]
}

/// Certify that a chart sample point is OFF a trim curve (§9.4 step 5's
/// precondition) by R9 distance-positivity data.
///
/// The sample's cross-multiplied point-vs-trim residual components are
/// certified over a partition of the trim parameter `[0, 1]`: a box is
/// certified free of the curve when at least one component excludes zero on
/// it (a certified positive separation in that chart coordinate). A partition
/// that cannot exclude the sample by DEPTH_MAX refuses
/// [`RefusalKind::TrimClipFailed`] (Inconclusive) — the sample cannot be
/// certified off the loop (an on-loop or unresolved near-loop sample).
// Refusal carries Option<PartialGraph> by frozen §2 shape; large-Err is allowed (BG-KV2-000).
#[allow(clippy::result_large_err)]
pub fn certify_off_loop(sample: [f64; 2], trim: &TrimLoop) -> Construction<OffLoopCert> {
    let components = point_trim_coeffs(sample, &trim.curve);
    let mut exclusions: Vec<CertifiedExclusion> = Vec::new();
    let mut stack: Vec<((f64, f64), u32)> = vec![((0.0, 1.0), 0u32)];
    while let Some(((lo, hi), depth)) = stack.pop() {
        let h0 = hull1(&components[0], (lo, hi));
        let h1 = hull1(&components[1], (lo, hi));
        let exclude0 = !h0.contains(0.0);
        let exclude1 = !h1.contains(0.0);
        if exclude0 || exclude1 {
            let (component, separation) = if exclude0 { (0usize, h0) } else { (1usize, h1) };
            exclusions.push(CertifiedExclusion {
                r: (lo, hi),
                component,
                separation,
            });
        } else if depth < DEPTH_MAX {
            let mid = 0.5 * (lo + hi);
            stack.push(((mid, hi), depth + 1));
            stack.push(((lo, mid), depth + 1));
        } else {
            return Err(depth_failure(
                "sample_off_loop_depth_max",
                format!(
                    "the sample {sample:?} could not be certified off the trim loop: both R9 \
                     residual components contain zero over the box [{lo}, {hi}] at DEPTH_MAX \
                     {DEPTH_MAX}; refusing TrimClipFailed (Inconclusive)"
                ),
            ));
        }
    }
    exclusions.sort_by(|x, y| x.r.0.total_cmp(&y.r.0));
    Ok(OffLoopCert { sample, exclusions })
}

// ---------------------------------------------------------------------------
// Winding number: the exact integer ray-crossing count (§9.4 step 5)
// ---------------------------------------------------------------------------

/// The certified sign of `dy/dr` over a `[0, 1]` sub-box, or `None` when the
/// numerator `Y′W − YW′` (whose sign is the sign of `dy/dr`, the certified
/// weight being positive) contains zero over the box.
fn y_direction_sign(leaf: &BezierLeaf1, box_: (f64, f64)) -> Option<i64> {
    let y = coeffs(leaf, 1);
    let w = coeffs(leaf, 3);
    let dy = bernstein_derivative_1d(&y);
    let dw = bernstein_derivative_1d(&w);
    let a = bernstein_mul(&dy, &w);
    let b = bernstein_mul(&y, &dw);
    let numerator: Vec<f64> = a.iter().zip(b.iter()).map(|(a, b)| *a - *b).collect();
    let hull = hull1(&numerator, box_);
    if hull.lo > 0.0 {
        Some(1)
    } else if hull.hi < 0.0 {
        Some(-1)
    } else {
        None
    }
}

/// The certified sign of the residual coefficient polynomial `c` at the point
/// `p`: `Some(true)` when `c(p)` is certifiably positive, `Some(false)` when
/// certifiably negative, `None` when the degenerate interval contains zero (the
/// polynomial touches its level set exactly at `p`).
fn certified_point_sign(coeffs: &[f64], p: f64) -> Option<bool> {
    let v = hull1(coeffs, (p, p));
    if v.lo > 0.0 {
        Some(true)
    } else if v.hi < 0.0 {
        Some(false)
    } else {
        None
    }
}

/// The certified winding number of a closed trim loop about a chart sample.
///
/// The ray-crossing count in the plane with certified sign discipline: along
/// the trim parameter, every certified transverse crossing of the horizontal
/// line `y = v₀` whose chart point lies to the right of the sample
/// (`x > u₀`) contributes the certified sign of the crossing direction
/// (`+1` upward, `-1` downward); the exact `i64` sum is the winding number of
/// the closed curve about the sample. Only polynomial arithmetic (Bernstein
/// coefficient hulls and degenerate point evaluations) and integer sums
/// participate.
///
/// A box is processed only on certified sign evidence: the residual
/// `c(r) = Y(r) − v₀·W(r)` is hull-excluded when it carries no zero, a box
/// whose certified endpoint signs differ is refined (toward the sign-changing
/// child) until the `x`-side of the sample and the crossing direction are both
/// certifiably determined, and a box whose endpoint sign is undecidable at
/// DEPTH_MAX refuses [`RefusalKind::TrimClipFailed`] (Inconclusive) — a
/// tangential graze or an exactly-aligned sample cannot be counted honestly.
///
/// The caller owes the sample's certified-off-loop property
/// ([`certify_off_loop`]) — that is what makes this index SOUND (§9.4).
// Refusal carries Option<PartialGraph> by frozen §2 shape; large-Err is allowed (BG-KV2-000).
#[allow(clippy::result_large_err)]
pub fn winding_number(trim: &TrimLoop, sample: [f64; 2]) -> Construction<i64> {
    if !trim.closed {
        return Err(caller_refusal(
            "trim_loop_open_no_winding",
            "winding_number requires a CLOSED trim loop (TrimLoop.closed = true)".to_string(),
        ));
    }
    let leaf = &trim.curve;
    let y0 = coeffs(leaf, 1);
    let w0 = coeffs(leaf, 3);
    let x0 = coeffs(leaf, 0);
    // c(r) = Y(r) − v₀·W(r): zero exactly where the loop crosses y = v₀.
    let cy: Vec<f64> = y0
        .iter()
        .zip(w0.iter())
        .map(|(y, w)| *y - sample[1] * *w)
        .collect();
    // g(r) = X(r) − u₀·W(r): positive exactly where the loop point is right of
    // the sample.
    let cx: Vec<f64> = x0
        .iter()
        .zip(w0.iter())
        .map(|(x, w)| *x - sample[0] * *w)
        .collect();
    let mut winding: i64 = 0;
    let mut stack: Vec<((f64, f64), u32)> = vec![((0.0, 1.0), 0u32)];
    while let Some(((lo, hi), depth)) = stack.pop() {
        if !hull1(&cy, (lo, hi)).contains(0.0) {
            // No loop point on this box reaches y = v₀: no ray crossing.
            continue;
        }
        let s_lo = certified_point_sign(&cy, lo);
        let s_hi = certified_point_sign(&cy, hi);
        let (Some(s_lo), Some(s_hi)) = (s_lo, s_hi) else {
            // A crossing sits exactly on a box boundary (or the box is so wide
            // its endpoint value is undecidable). Subdivide toward an interior
            // decision; at DEPTH_MAX the decision genuinely failed.
            if depth < DEPTH_MAX {
                let mid = 0.5 * (lo + hi);
                stack.push(((mid, hi), depth + 1));
                stack.push(((lo, mid), depth + 1));
            } else {
                return Err(depth_failure(
                    "winding_endpoint_sign_depth_max",
                    format!(
                        "the y-crossing of the trim loop about {sample:?} has an undecidable \
                         endpoint sign at DEPTH_MAX {DEPTH_MAX} (box [{lo}, {hi}]); refusing \
                         TrimClipFailed (Inconclusive)"
                    ),
                ));
            }
            continue;
        };
        if s_lo == s_hi {
            // Same certified endpoint sign: no root inside a monotone box; a
            // non-monotone box may still carry an even number of crossings, so
            // it is subdivided (its hull contains zero).
            if y_direction_sign(leaf, (lo, hi)).is_none() {
                if depth < DEPTH_MAX {
                    let mid = 0.5 * (lo + hi);
                    stack.push(((mid, hi), depth + 1));
                    stack.push(((lo, mid), depth + 1));
                } else {
                    return Err(depth_failure(
                        "winding_same_sign_depth_max",
                        format!(
                            "the y-crossing of the trim loop about {sample:?} could not be \
                             resolved at DEPTH_MAX {DEPTH_MAX} (box [{lo}, {hi}]); refusing \
                             TrimClipFailed (Inconclusive)"
                        ),
                    ));
                }
            }
            continue;
        }
        // Certified endpoint sign change: at least one transverse crossing lies
        // in this box. Refine toward the sign-changing child until the side of
        // the sample is certified and the box is y-monotone (exactly one
        // crossing; the crossing direction is the monotonicity sign).
        let mut refined = (lo, hi);
        let mut refined_depth = depth;
        loop {
            let xh = hull1(&cx, refined);
            let direction = y_direction_sign(leaf, refined);
            if !xh.contains(0.0) {
                if let Some(direction) = direction {
                    if xh.lo > 0.0 {
                        winding += direction;
                    }
                    break;
                }
            }
            if refined_depth >= DEPTH_MAX {
                return Err(depth_failure(
                    "winding_refine_depth_max",
                    format!(
                        "the ray-crossing of the trim loop about {sample:?} could not be \
                         certified at DEPTH_MAX {DEPTH_MAX} (box {refined:?}); refusing \
                         TrimClipFailed (Inconclusive)"
                    ),
                ));
            }
            let mid = 0.5 * (refined.0 + refined.1);
            let mid_sign = certified_point_sign(&cy, mid);
            match mid_sign {
                Some(mid_pos) => {
                    let s_left = certified_point_sign(&cy, refined.0);
                    let (keep_left, keep_right) = match s_left {
                        Some(s_left) if s_left != mid_pos => (refined.0, mid),
                        _ => (mid, refined.1),
                    };
                    refined = (keep_left, keep_right);
                }
                None => {
                    // The crossing sits exactly at the refinement midpoint. It
                    // is counted on the right-hand child only (never twice), by
                    // starting the next refinement just to its right is not
                    // representable; push the right half and skip the left.
                    // The left half's right endpoint contains the crossing and
                    // is handled by the sign test above on the next iteration.
                    refined = (mid, refined.1);
                }
            }
            refined_depth += 1;
        }
    }
    Ok(winding)
}

// ---------------------------------------------------------------------------
// The full §9.4 clip
// ---------------------------------------------------------------------------

/// The certified chart ends of an arc on one consistent `p1`/`p2` slot: both
/// endpoints resolve to nodes and both place the same slot on the same chart.
struct ArcChartEnds {
    /// The chart the arc's trimmed pcurve lives in.
    chart: ChartId,
    /// The certified chart point of the first end.
    first: [f64; 2],
    /// The certified chart point of the second end.
    second: [f64; 2],
    /// The deck of the first end on the trimmed side.
    deck: i32,
}

/// The certified chart ends of an arc on one consistent slot, `None` for
/// carrier arcs, self-intersection/spine arcs, or arcs whose ends do not both
/// resolve to nodes on one common chart.
fn arc_chart_ends(nodes: &[Node], arc: &AnyArc) -> Option<ArcChartEnds> {
    let (first, second) = match arc {
        AnyArc::Ordinary(Arc { ends, .. }) | AnyArc::Difference(Arc { ends, .. }) => *ends,
        AnyArc::SelfInt(_) | AnyArc::Spine(_) | AnyArc::Carrier(_) => return None,
    };
    let a = node_at(nodes, first)?;
    let b = node_at(nodes, second)?;
    if a.p1.chart == b.p1.chart {
        Some(ArcChartEnds {
            chart: a.p1.chart,
            first: [a.p1.u, a.p1.v],
            second: [b.p1.u, b.p1.v],
            deck: a.p1.deck,
        })
    } else if a.p2.chart == b.p2.chart {
        Some(ArcChartEnds {
            chart: a.p2.chart,
            first: [a.p2.u, a.p2.v],
            second: [b.p2.u, b.p2.v],
            deck: a.p2.deck,
        })
    } else {
        None
    }
}

/// The [`Point4`] of a node-resolved [`ArcEnd`], `None` for a break end (a
/// break is a SegmentBreak; the module splits only at certified topology
/// nodes).
fn node_at(nodes: &[Node], end: ArcEnd) -> Option<&Point4> {
    match end {
        ArcEnd::Topo(id) => nodes.iter().find(|node| node.id == id).map(|node| &node.at),
        ArcEnd::Seg(_) => None,
    }
}

/// The straight pcurve leaf of an arc in its chart: the degree-1 leaf whose
/// affine image is the certified straight segment between the two certified
/// chart ends (the S3c pcurve seam documented in the module doc).
// Refusal carries Option<PartialGraph> by frozen §2 shape; large-Err is allowed (BG-KV2-000).
#[allow(clippy::result_large_err)]
fn arc_pcurve_leaf(ends: &ArcChartEnds) -> Result<BezierLeaf1, Refusal> {
    BezierLeaf1::try_new(
        1,
        vec![
            [ends.first[0], ends.first[1], 0.0, 1.0],
            [ends.second[0], ends.second[1], 0.0, 1.0],
        ],
        ends.chart,
    )
}

/// A certified crossing of one arc, with its split parameters on the arc and
/// the (lazily allocated) TrimCrossing node.
struct SplitEvent {
    /// The certified crossing.
    crossing: CertifiedCrossing,
    /// The certified arc-parameter interval (axis 0 of the cert box).
    t_lo: f64,
    /// The certified arc-parameter interval (axis 0 of the cert box).
    t_hi: f64,
    /// The TrimCrossing node, allocated on first use by a retained sub-arc.
    node: Option<Node>,
}

/// The per-arc allocation state of the clip.
struct ClipState {
    /// The next fresh node id.
    next_node: usize,
    /// The next fresh arc id.
    next_arc: usize,
}

/// The outcome of clipping one input arc.
enum ClipOutcome {
    /// The arc is not on any trim chart: pass it through untouched.
    Untouched,
    /// The arc was clipped away (every sub-arc outside the trim region).
    ClippedAway,
    /// The arc is retained (whole or as its retained sub-arcs).
    Retained(Vec<AnyArc>),
}

/// The §9.4 trim clip: clip every clippable arc of the certified leaf-product
/// 1-complex against the closed trim loops of its chart.
///
/// `trim_clip(graph, trims)` implements §9.4 steps 3–6 over the input
/// [`CertifiedGraph`]: certified R9 crossings of each arc's pcurve against
/// each closed trim curve of the SAME chart become [`TopoNode::TrimCrossing`]
/// nodes; the arcs are split at those crossings; sub-arcs are classified by
/// the winding number of the closed trim loop about a certified-off interior
/// sample; outside sub-arcs are discarded. Arcs that are not on any trim chart
/// pass through untouched. A closed trim whose crossings cannot be isolated at
/// DEPTH_MAX refuses [`RefusalKind::TrimClipFailed`] (Inconclusive). An open
/// trim loop (`closed == false`) is refused as a caller error: it cannot bound
/// a face region for classification. The returned graph carries the original
/// nodes, breaks, and sheets plus the fresh TrimCrossing nodes and retained
/// (sub-)arcs.
// Refusal carries Option<PartialGraph> by frozen §2 shape; large-Err is allowed (BG-KV2-000).
#[allow(clippy::result_large_err)]
pub fn trim_clip(graph: &CertifiedGraph, trims: &[TrimLoop]) -> Construction<CertifiedGraph> {
    if trims.iter().any(|trim| !trim.closed) {
        return Err(caller_refusal(
            "trim_loop_open_cannot_bound",
            "trim_clip requires every trim loop to be closed (TrimLoop.closed = true); an open \
             trim cannot bound a face region for winding classification"
                .to_string(),
        ));
    }
    if trims.is_empty() {
        return Ok(graph.clone());
    }
    let max_node = graph.nodes.iter().map(|node| node.id.0).max().unwrap_or(0);
    let max_arc = graph
        .arcs
        .iter()
        .map(|arc| any_arc_id(arc).map(|id| id.0).unwrap_or(0))
        .max()
        .unwrap_or(0);
    let mut state = ClipState {
        next_node: max_node + 1,
        next_arc: max_arc + 1,
    };
    let mut fresh_nodes: Vec<Node> = Vec::new();
    let mut out_arcs: Vec<AnyArc> = Vec::with_capacity(graph.arcs.len());
    for arc in &graph.arcs {
        match clip_arc(arc, &graph.nodes, trims, &mut fresh_nodes, &mut state)? {
            ClipOutcome::Untouched => out_arcs.push(arc.clone()),
            ClipOutcome::ClippedAway => {}
            ClipOutcome::Retained(mut kept) => out_arcs.append(&mut kept),
        }
    }
    let mut nodes = graph.nodes.clone();
    nodes.append(&mut fresh_nodes);
    Ok(CertifiedGraph {
        nodes,
        breaks: graph.breaks.clone(),
        arcs: out_arcs,
        sheets: graph.sheets.clone(),
        exhaustive: graph.exhaustive,
    })
}

/// The id of a non-carrier [`AnyArc`].
fn any_arc_id(arc: &AnyArc) -> Option<ArcId> {
    match arc {
        AnyArc::Ordinary(Arc { id, .. })
        | AnyArc::Difference(Arc { id, .. })
        | AnyArc::SelfInt(Arc { id, .. })
        | AnyArc::Spine(Arc { id, .. }) => Some(*id),
        AnyArc::Carrier(_) => None,
    }
}

/// Clip one arc against the closed trim loops of its chart (§9.4 steps 3–6).
#[allow(clippy::result_large_err)]
fn clip_arc(
    arc: &AnyArc,
    nodes: &[Node],
    trims: &[TrimLoop],
    fresh_nodes: &mut Vec<Node>,
    state: &mut ClipState,
) -> Construction<ClipOutcome> {
    let ends = match arc_chart_ends(nodes, arc) {
        Some(ends) => ends,
        None => return Ok(ClipOutcome::Untouched),
    };
    let loops: Vec<&TrimLoop> = trims.iter().filter(|t| t.chart == ends.chart).collect();
    if loops.is_empty() {
        return Ok(ClipOutcome::Untouched);
    }
    let pcurve = arc_pcurve_leaf(&ends)?;
    // Every certified crossing of the arc's pcurve against every closed trim
    // loop of the same chart (the union over loops of §9.4 step 3), in
    // ascending arc-parameter order.
    let mut events: Vec<SplitEvent> = Vec::new();
    for trim in &loops {
        for crossing in certify_crossings(&pcurve, &trim.curve)? {
            let t_lo = crossing.cert.box_.lo[0];
            let t_hi = crossing.cert.box_.hi[0];
            events.push(SplitEvent {
                crossing,
                t_lo,
                t_hi,
                node: None,
            });
        }
    }
    events.sort_by(|x, y| x.t_lo.total_cmp(&y.t_lo));
    if events.is_empty() {
        // No trim boundary is met: the whole arc is one sub-arc. Classify its
        // interior sample; retain the arc as-is, or drop it.
        let sample = chart_point(&pcurve, 0.5);
        for trim in &loops {
            if !sample_inside(trim, sample)? {
                return Ok(ClipOutcome::ClippedAway);
            }
        }
        return Ok(ClipOutcome::Retained(vec![arc.clone()]));
    }

    let mut retained: Vec<AnyArc> = Vec::new();
    // The retained sub-arcs are the certified open runs between consecutive
    // crossing zones (and between a crossing zone and the original arc end).
    // A crossing zone itself is the split marker, never a sub-arc.
    let mut cursor = 0.0f64;
    for index in 0..events.len() {
        if events[index].t_lo > cursor {
            let start_end = if cursor == 0.0 {
                None
            } else {
                let node = event_node(&mut events[index - 1], &ends, fresh_nodes, state)?;
                Some(ArcEnd::Topo(node.id))
            };
            let end_end = {
                let node = event_node(&mut events[index], &ends, fresh_nodes, state)?;
                Some(ArcEnd::Topo(node.id))
            };
            let piece = build_piece(
                arc,
                &pcurve,
                cursor,
                events[index].t_lo,
                start_end,
                end_end,
                &loops,
                state,
            )?;
            if let Some(sub) = piece {
                retained.push(sub);
            }
        }
        cursor = events[index].t_hi;
    }
    if cursor < 1.0 {
        let last = events.len() - 1;
        let start_end = {
            let node = event_node(&mut events[last], &ends, fresh_nodes, state)?;
            Some(ArcEnd::Topo(node.id))
        };
        let piece = build_piece(arc, &pcurve, cursor, 1.0, start_end, None, &loops, state)?;
        if let Some(sub) = piece {
            retained.push(sub);
        }
    }
    if retained.is_empty() {
        Ok(ClipOutcome::ClippedAway)
    } else {
        Ok(ClipOutcome::Retained(retained))
    }
}

/// Allocate (or reuse) the TrimCrossing node of a split event.
#[allow(clippy::result_large_err)]
fn event_node(
    event: &mut SplitEvent,
    ends: &ArcChartEnds,
    fresh_nodes: &mut Vec<Node>,
    state: &mut ClipState,
) -> Construction<Node> {
    if let Some(node) = event.node {
        return Ok(node);
    }
    let crossing = &event.crossing;
    let id = NodeId(state.next_node);
    state.next_node += 1;
    let at = Point4 {
        p1: Param::try_new(ends.chart, ends.deck, crossing.point[0], crossing.point[1])?,
        p2: Param::try_new(ends.chart, ends.deck, crossing.point[0], crossing.point[1])?,
    };
    let node = Node {
        id,
        at,
        kind: TopoNode::TrimCrossing,
        cert: NodeCert::Exact(crossing.cert),
    };
    fresh_nodes.push(node);
    event.node = Some(node);
    Ok(node)
}

/// Build one retained sub-arc for the arc-parameter run `[piece_lo, piece_hi]`
/// if (and only if) its interior sample classifies inside every closed trim
/// loop of the chart.
///
/// `start_end` is `None` exactly when the run begins at the original arc's own
/// first end; `end_end` likewise for the original second end.
#[allow(clippy::too_many_arguments)]
#[allow(clippy::result_large_err)]
fn build_piece(
    arc: &AnyArc,
    pcurve: &BezierLeaf1,
    piece_lo: f64,
    piece_hi: f64,
    start_end: Option<ArcEnd>,
    end_end: Option<ArcEnd>,
    loops: &[&TrimLoop],
    state: &mut ClipState,
) -> Construction<Option<AnyArc>> {
    if piece_lo >= piece_hi {
        return Ok(None);
    }
    // The sample is certified off EVERY closed loop before any winding is
    // evaluated (the SOUND use), then classified.
    let sample_t = 0.5 * (piece_lo + piece_hi);
    let sample = chart_point(pcurve, sample_t);
    for trim in loops {
        if !sample_inside(trim, sample)? {
            return Ok(None);
        }
    }
    let original = match arc {
        AnyArc::Ordinary(Arc { ends, .. }) | AnyArc::Difference(Arc { ends, .. }) => *ends,
        _ => {
            return Err(caller_refusal(
                "trimclip_unclippable_arc_family",
                "only Ordinary and Difference arcs are clippable".to_string(),
            ))
        }
    };
    let first_end = start_end.unwrap_or(original.0);
    let second_end = end_end.unwrap_or(original.1);
    let sub = build_sub_arc(
        arc, pcurve, piece_lo, piece_hi, first_end, second_end, state,
    )?;
    Ok(Some(sub))
}

/// Build the retained sub-arc over the certified pcurve parameter range
/// `[piece_lo, piece_hi]`: a fresh linear Hermite witness over the sub-segment
/// and a clone of the input arc's tube certificate narrowed to the sub-range.
#[allow(clippy::result_large_err)]
fn build_sub_arc(
    arc: &AnyArc,
    pcurve: &BezierLeaf1,
    piece_lo: f64,
    piece_hi: f64,
    first_end: ArcEnd,
    second_end: ArcEnd,
    state: &mut ClipState,
) -> Construction<AnyArc> {
    let pa = chart_point(pcurve, piece_lo);
    let pb = chart_point(pcurve, piece_hi);
    let d = [pb[0] - pa[0], pb[1] - pa[1], 0.0];
    let segment = HermiteSegment {
        p0: [pa[0], pa[1], 0.0],
        p1: [pb[0], pb[1], 0.0],
        t0: d,
        t1: d,
    };
    let spline = HermiteSpline::try_new(vec![segment])?;
    let approx = Approx { gamma: spline };
    let i_tau = Interval {
        lo: piece_lo,
        hi: piece_hi,
    };
    let id = ArcId(state.next_arc);
    state.next_arc += 1;
    match arc {
        AnyArc::Ordinary(Arc { cert, .. }) => {
            let mut cert = cert.clone();
            cert.i_tau = i_tau;
            Ok(AnyArc::Ordinary(Arc {
                id,
                approx,
                cert,
                ends: (first_end, second_end),
            }))
        }
        AnyArc::Difference(Arc { cert, .. }) => {
            let mut cert = cert.clone();
            cert.i_tau = i_tau;
            Ok(AnyArc::Difference(Arc {
                id,
                approx,
                cert,
                ends: (first_end, second_end),
            }))
        }
        _ => Err(caller_refusal(
            "trimclip_unclippable_arc_family",
            "only Ordinary and Difference arcs are clippable".to_string(),
        )),
    }
}

/// Whether a chart sample is inside a closed trim loop: the certified winding
/// number of the closed loop about the certified-off sample is non-zero.
///
/// This is the SOUND composition of §9.4 step 5: the sample is first certified
/// OFF the loop by R9 distance-positivity data, and only then is the winding
/// number (a classical, exactly-computable index with no cancellation
/// ambiguity) evaluated.
#[allow(clippy::result_large_err)]
fn sample_inside(trim: &TrimLoop, sample: [f64; 2]) -> Construction<bool> {
    certify_off_loop(sample, trim)?;
    let winding = winding_number(trim, sample)?;
    Ok(winding != 0)
}
