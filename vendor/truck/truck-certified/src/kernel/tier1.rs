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

//! The Tier-1 loop-free certificate and the §9.3 R8 boundary-stratum seeds
//! (BG-KV2-301-S03A).
//!
//! **Tier 1 (Theorem 9.1).** [`tier1_loop_free`] implements the two-cone LP
//! of §9.1: a feasible `d` with `d·(n1 × n2) ≠ 0` for all `n1 ∈ Cone1`,
//! `n2 ∈ Cone2` certifies that the pair is loop-free (no tangency, no closed
//! component, every component meets the boundary of the lifted product
//! domain). Feasibility is certified by the CONE-SEPARATION test: the
//! candidate direction `d = (a1 × a2)/|a1 × a2|` has a certified strictly
//! positive lower bound on `d·(n1 × n2)` computed from the axes and
//! half-angles in COS-SPACE — a polynomial inequality on cosines, no angle
//! function anywhere (N4). The bound is derived from the bilinear identity
//! `(a1×a2)·(n1×n2) = c1c2(1−m²) − m·c1·s2·(a1·r2) − m·c2·s1·(a2·r1)
//! − s1·s2·(a1·r2)(a2·r1)` (Lagrange), with `c_i = a_i·n_i`, `s_i·r_i` the
//! perpendicular split of `n_i`, evaluated over certified superset ranges
//! (Sederberg–Meyers/Hohmeyer lineage, spec §23 — informational only, the
//! bound itself is derived here). An infeasible (tangential-adjacent) pair is
//! `Inconclusive` and routes to Tier 2 (S3b's business); the `Disproven` arm
//! is unused by this test.
//!
//! **Boundary seeds (§9.3).** [`boundary_seeds`] solves every caller-supplied
//! boundary edge of one leaf against the other leaf as an R8 problem (3
//! equations in `(t, u, v)`, square C1) via [`krawczyk_c1_n3`] with
//! subdivision to [`crate::kernel::config::DEPTH_MAX`], collecting the
//! `Proven` certificates as R8-stamped [`PointCert3`]s. v1 honest scope: the
//! edge set is CALLER-SUPPLIED as `(BezierLeaf1 curve, chart)` leaf data —
//! this packet does not enumerate B-rep edges from face topology (that is the
//! leaf/atlas wave's contract); it solves R8 per supplied edge.
//!
//! **H-1.** The crate-level `#![deny(clippy::unwrap_used)]` in `lib.rs` covers
//! this module. The module carries no `unwrap`, no `expect`, and no `panic!`
//! calls, and adds no module-level `allow`. Where a `Result` must carry the
//! frozen `Refusal` (which holds `Option<PartialGraph>`), the large-`Err`
//! lint is allowed item-level only, exactly as the shim files do.
//!
//! **N4 / cos-space.** No `sin`, `cos`, `atan2`, `exp`, `ln`, `log`, or
//! `powf` call appears anywhere in this module. The one `sqrt` is the IEEE
//! square root used to normalize the candidate direction `d` (and, with it,
//! the axis-separation magnitude `|a1 × a2|`) — the N4 normalization carve-out.

use crate::kernel::certs::PointCert3;
use crate::kernel::config::{DEPTH_MAX, TOL_JACOBIAN};
use crate::kernel::engine::{krawczyk_c1_n3, SquareResidualEval};
use crate::kernel::evidence::{ClaimVerdict, Construction, Refusal, RefusalEvidence, RefusalKind};
use crate::kernel::leaf::BezierLeaf;
use crate::kernel::patch::{
    CertifiedPatch, CertifiedPositive, Cone, Degeneracy, IBox2, IBox3, Reason,
};
use crate::kernel::residual::ResidualId;
use crate::kernel::residuals_r89::{BezierLeaf1, R8System};
use crate::kernel::Interval;

/// A certified Tier-1 transversal direction (Theorem 9.1): `d` is a unit
/// vector and `min_dot` is a certified strictly-positive lower bound on
/// `d·(n1 × n2)` over all `n1` in the first cone and `n2` in the second.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TierOneCert {
    /// The unit transversal direction.
    pub d: [f64; 3],
    /// The certified lower bound of `d·(n1 × n2)` (strictly positive).
    pub min_dot: f64,
}

/// The infeasible/tangential-adjacent reasons (Inconclusive arms, static).
const AXES_PARALLEL: Reason = "tier1_axes_parallel_no_transversal_direction";
const NON_FINITE: Reason = "tier1_cone_data_not_finite";
const NOT_SEPARABLE: Reason = "tier1_cross_cone_not_separable_from_zero";
const BELOW_FLOOR: Reason = "tier1_min_dot_below_jacobian_floor";

/// A certified enclosure of the scalar product `a·b` of two float vectors
/// (outward-rounded `CertifiedInterval` sequence, fixed index order).
fn dot_iv(a: [f64; 3], b: [f64; 3]) -> Interval {
    let mut acc = Interval::point(0.0);
    for i in 0..3 {
        acc = acc.add(&Interval::point(a[i]).mul(&Interval::point(b[i])));
    }
    acc
}

/// A certified lower bound of `cos θ`: the algebraic bound `cos θ ≥ 1 − θ²/2`
/// (from `cos θ = 1 − 2 sin²(θ/2) ≥ 1 − θ²/2`), outward-rounded downward.
fn cos_lower_bound(theta: f64) -> f64 {
    let t = Interval::point(theta);
    let sq = t.mul(&t);
    let half = sq.mul(&Interval::point(0.5));
    Interval::point(1.0).sub(&half).lo
}

/// A certified upper bound of `sin u` for `u ∈ [0, θ]`: `sin u ≤ min(u, 1) ≤
/// min(θ, 1)`, rounded upward. (For `θ ≥ 1` the unit ceiling is the bound; for
/// `θ < 1` the chord bound `sin u ≤ u` applies on `[0, θ] ⊂ [0, π/2)`.)
fn sin_upper_bound(theta: f64) -> f64 {
    theta.min(1.0).next_up()
}

/// The float cross product of two 3-vectors.
fn cross3(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// The unit direction along `v`, when `v` is not degenerate.
fn unit_direction(v: [f64; 3]) -> Option<[f64; 3]> {
    let norm = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if norm.is_finite() && norm > 0.0 {
        Some([v[0] / norm, v[1] / norm, v[2] / norm])
    } else {
        None
    }
}

/// Theorem 9.1's two-cone test as the CONE-SEPARATION check: a certified
/// strictly-positive lower bound on `d·(n1 × n2)` for the candidate direction
/// `d = (a1 × a2)/|a1 × a2|`, evaluated in cos-space.
///
/// Returns the certified lower bound of `d·(n1 × n2)` and the certified upper
/// bound `sg_hi` of `|a1 × a2|`, or the first [`Reason`] that blocks
/// separation (axes parallel, non-finite data, or a non-positive margin).
fn cross_cone_margin(c1: &Cone, c2: &Cone) -> Result<(f64, f64), Reason> {
    let a1 = c1.axis;
    let a2 = c2.axis;
    let ca1 = cos_lower_bound(c1.half_angle);
    let ca2 = cos_lower_bound(c2.half_angle);
    let sa1 = sin_upper_bound(c1.half_angle);
    let sa2 = sin_upper_bound(c2.half_angle);

    // cos γ (γ = angle between the axes), outward-rounded.
    let m_iv = dot_iv(a1, a2);
    // sγ² = 1 − m², clamped to a non-negative certified range.
    let sq = Interval::point(1.0).sub(&m_iv.mul(&m_iv));
    let sq_lo = sq.lo.max(0.0);
    let sq_hi = sq.hi.max(0.0);
    if !sq_hi.is_finite() || sq_hi == 0.0 {
        // The axes are (anti)parallel to the certified arithmetic: every pair
        // of cone vectors can be parallel, so no transversal d exists.
        return Err(AXES_PARALLEL);
    }
    let sg = match (Interval {
        lo: sq_lo,
        hi: sq_hi,
    })
    .sqrt()
    {
        Some(sg) => sg,
        None => return Err(NON_FINITE),
    };
    let sg_hi = sg.hi;
    if !sg_hi.is_finite() || sg_hi <= 0.0 {
        return Err(AXES_PARALLEL);
    }

    // Certified superset ranges of the cone-split quantities:
    //   n_i = c_i·a_i + s_i·r_i,  c_i = a_i·n_i ∈ [cos θ_i, 1],
    //   s_i ∈ [−sin θ_i, sin θ_i],  and (a1·r2), (a2·r1) ∈ [−sγ, sγ].
    let c1i = Interval { lo: ca1, hi: 1.0 };
    let c2i = Interval { lo: ca2, hi: 1.0 };
    let s1i = Interval { lo: -sa1, hi: sa1 };
    let s2i = Interval { lo: -sa2, hi: sa2 };
    let sqi = Interval {
        lo: sq_lo,
        hi: sq_hi,
    };
    let psii = Interval {
        lo: -sg_hi,
        hi: sg_hi,
    };

    // g = (a1×a2)·(n1×n2) expands by the Lagrange identity to
    //   c1c2(1−m²) − m·c1·s2·(a1·r2) − m·c2·s1·(a2·r1) − s1·s2·(a1·r2)(a2·r1).
    // Evaluating the four terms over the independent certified superset ranges
    // and subtracting outward yields a certified lower bound of g.
    let t1 = c1i.mul(&c2i).mul(&sqi);
    let t2 = m_iv.mul(&c1i).mul(&s2i).mul(&psii);
    let t3 = m_iv.mul(&c2i).mul(&s1i).mul(&psii);
    let t4 = s1i.mul(&s2i).mul(&psii).mul(&psii);
    let g = t1.sub(&t2).sub(&t3).sub(&t4);
    let g_lb = g.lo;
    if !g_lb.is_finite() {
        return Err(NON_FINITE);
    }
    if g_lb <= 0.0 {
        return Err(NOT_SEPARABLE);
    }
    // d·(n1×n2) = g/sγ ≥ g_lb/sg_hi (sγ = |a1×a2| is constant over the cones).
    let min_dot = (g_lb / sg_hi).next_down();
    if !min_dot.is_finite() || min_dot <= TOL_JACOBIAN {
        return Err(BELOW_FLOOR);
    }
    Ok((min_dot, sg_hi))
}

/// Theorem 9.1's Tier-1 loop-free certificate as an LP over two cached normal
/// cones: `Proven(TierOneCert)` carries a feasible unit transversal direction
/// `d` and its certified positive lower bound `min_dot` when the cos-space
/// separation test certifies `d·(n1 × n2) > 0` over the whole cone product
/// (checked at the [`TOL_JACOBIAN`] floor). An infeasible
/// (tangential-adjacent) pair — the axes parallel, or the cross-cone margin
/// not separable from zero — is `Inconclusive`, routing to Tier 2 (S3b).
pub fn tier1_loop_free(c1: &Cone, c2: &Cone) -> ClaimVerdict<TierOneCert, Degeneracy, Reason> {
    let axes_finite = c1.axis.iter().chain(c2.axis.iter()).all(|x| x.is_finite())
        && c1.half_angle.is_finite()
        && c2.half_angle.is_finite();
    if !axes_finite {
        return ClaimVerdict::Inconclusive(NON_FINITE);
    }
    let (min_dot, _sg_hi) = match cross_cone_margin(c1, c2) {
        Ok(margin) => margin,
        Err(reason) => return ClaimVerdict::Inconclusive(reason),
    };
    let d = match unit_direction(cross3(c1.axis, c2.axis)) {
        Some(d) => d,
        None => return ClaimVerdict::Inconclusive(AXES_PARALLEL),
    };
    ClaimVerdict::Proven(TierOneCert { d, min_dot })
}

/// A named predicate refusal: the kind names the violated class, the predicate
/// name the precise invariant.
fn refusal(kind: RefusalKind, name: &'static str, detail: String) -> Refusal {
    Refusal::new(kind, RefusalEvidence::Predicate { name, detail })
}

// ---------------------------------------------------------------------------
// §9.3 boundary-stratum seeds: every edge of P against Q (and of Q against P)
// is an R8 problem, solved by krawczyk_c1_n3 with subdivision to DEPTH_MAX.
// ---------------------------------------------------------------------------

/// Whether a certified residual component over the box provably excludes zero
/// (a sound no-root-in-box exclusion used to prune the subdivision tree before
/// the Krawczyk step).
fn residual_excludes_zero(sys: &dyn SquareResidualEval, b: &IBox3) -> bool {
    let box_iv: [Interval; 3] = [
        Interval {
            lo: b.lo[0],
            hi: b.hi[0],
        },
        Interval {
            lo: b.lo[1],
            hi: b.hi[1],
        },
        Interval {
            lo: b.lo[2],
            hi: b.hi[2],
        },
    ];
    let h = sys.eval(&box_iv);
    h.iter().any(|component| !component.contains(0.0))
}

/// Bisect the box along its widest axis (lowest-index tie-break, deterministic
/// order): returns the two closed half-boxes.
fn bisect(b: &IBox3) -> Vec<IBox3> {
    let mut axis = 0usize;
    let mut width = b.hi[0] - b.lo[0];
    for i in 1..3 {
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

/// Solve one R8 boundary problem with subdivision to
/// [`crate::kernel::config::DEPTH_MAX`], collecting every `Proven` certificate
/// (rebuilt with [`ResidualId::R8`]) into `out`. Boxes that provably contain
/// no root (a residual component excluding zero) or whose Krawczyk image is
/// disjoint are dropped; inconclusive boxes are bisected deterministically.
///
/// The subdivision boxes are pairwise disjoint and the C1 certificate is a
/// uniqueness certificate per box, so two collected seeds never witness the
/// same root.
fn collect_r8_seeds(sys: &R8System, w: &[CertifiedPositive], out: &mut Vec<PointCert3>) {
    let root = match IBox3::try_new([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]) {
        Ok(root) => root,
        Err(_) => return,
    };
    let mut stack: Vec<(IBox3, u32)> = vec![(root, 0u32)];
    while let Some((b, depth)) = stack.pop() {
        if residual_excludes_zero(sys, &b) {
            continue;
        }
        match krawczyk_c1_n3(sys, b, w) {
            ClaimVerdict::Proven(cert) => {
                // The engine stamps ResidualId::R1; rebuild through the
                // documented one-line seam with the R8 residual's own id.
                if let Ok(cert) = PointCert3::try_new(ResidualId::R8, cert.box_, cert.rho) {
                    out.push(cert);
                }
            }
            ClaimVerdict::Disproven(_) => {}
            ClaimVerdict::Inconclusive(_) => {
                if depth < DEPTH_MAX {
                    for child in bisect(&b) {
                        stack.push((child, depth + 1));
                    }
                }
            }
        }
    }
}

/// The certified positive weight bound of the surface leaf over its full unit
/// domain, or a refusal when the §7.1 value argument cannot be produced.
// Refusal carries Option<PartialGraph> by frozen §2 shape; large-Err is allowed (BG-KV2-000).
#[allow(clippy::result_large_err)]
fn surface_weight(surface: &BezierLeaf) -> Construction<CertifiedPositive> {
    let uv = match IBox2::try_new([0.0, 0.0], [1.0, 1.0]) {
        Ok(uv) => uv,
        Err(_) => {
            return Err(refusal(
                RefusalKind::NonFinite,
                "boundary_seeds_domain_box_refused",
                "the leaf's unit domain box could not be constructed".to_string(),
            ))
        }
    };
    match CertifiedPatch::weight_bound(surface, uv) {
        Some(ClaimVerdict::Proven(positive)) => Ok(positive),
        Some(other) => Err(refusal(
            RefusalKind::WeightDegenerate,
            "boundary_seeds_weight_not_proven",
            format!("the R8 surface weight bound is not Proven over the leaf: {other:?}"),
        )),
        None => Err(refusal(
            RefusalKind::WeightDegenerate,
            "boundary_seeds_no_weight_field",
            "the R8 surface leaf exposes no weight bound (weight_bound is None)".to_string(),
        )),
    }
}

/// The §9.3 R8 boundary-stratum seeds of a leaf pair: every edge of `p`
/// (`p_edges`) against the `q` leaf and every edge of `q` (`q_edges`) against
/// the `p` leaf is solved as an R8 problem over `(t, u, v) ∈ [0, 1]³` with
/// subdivision to [`crate::kernel::config::DEPTH_MAX`], collecting the
/// `Proven` certificates (R8-stamped [`PointCert3`]s).
///
/// v1 honest scope (recorded): the edge set arrives as CALLER-SUPPLIED
/// `(curve, chart)` leaf data — this packet does not enumerate B-rep edges
/// from face topology (that is the leaf/atlas wave's contract); it solves R8
/// per supplied edge. The surfaces are passed as [`BezierLeaf`]s (the R8
/// residual consumes a concrete homogeneous net, which a `&dyn
/// CertifiedPatch` cannot expose); the certificate carrier is [`PointCert3`],
/// the frozen arity-3 spelling of the spec's `PointCert` (whose `box_` is an
/// `IBox2` and cannot record an R8 box).
// Refusal carries Option<PartialGraph> by frozen §2 shape; large-Err is allowed (BG-KV2-000).
#[allow(clippy::result_large_err)]
pub fn boundary_seeds(
    p: &BezierLeaf,
    p_edges: &[BezierLeaf1],
    q: &BezierLeaf,
    q_edges: &[BezierLeaf1],
) -> Construction<Vec<PointCert3>> {
    let mut seeds: Vec<PointCert3> = Vec::new();
    // Edges of P pierce Q; edges of Q pierce P.
    let (w_q, w_p) = match (surface_weight(q), surface_weight(p)) {
        (Ok(w_q), Ok(w_p)) => (w_q, w_p),
        (Err(refusal), _) | (_, Err(refusal)) => return Err(refusal),
    };
    for edge in p_edges {
        let sys = R8System::try_new(edge, q)?;
        collect_r8_seeds(&sys, &[w_q], &mut seeds);
    }
    for edge in q_edges {
        let sys = R8System::try_new(edge, p)?;
        collect_r8_seeds(&sys, &[w_p], &mut seeds);
    }
    Ok(seeds)
}
