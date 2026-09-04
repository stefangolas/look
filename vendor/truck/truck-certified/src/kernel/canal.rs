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

//! The §12 fillet/canal machinery (BG-KV2-402-S7): the R7 ball-center residual,
//! the n=7 tube certificate that serves it, the [`Canal`] representation, the
//! Δ_off offset-regularity diagnostic, and the §12.3 three-face corner.
//!
//! **H-1.** The crate-level `#![deny(clippy::unwrap_used)]` in `lib.rs` covers
//! this module. The module carries no `unwrap`, no `expect`, and no `panic!`
//! calls, and adds no module-level `allow`. Where a `Result` must carry the
//! frozen `Refusal` (which holds `Option<PartialGraph>`), the large-`Err` lint
//! is allowed item-level only, exactly as the shim files do.
//!
//! **R7 (spec §12.1).** The rolling-ball center residual: unknowns
//! `(c, u, v, s, t) ∈ R⁷`, for `i = 1, 2`:
//!
//! ```text
//! (c − Sᵢ)·Sⁱ_u = 0,     (c − Sᵢ)·Sⁱ_v = 0,     ‖c − Sᵢ‖² − r² = 0
//! ```
//!
//! Six polynomial equations in seven unknowns; the zero set is one-
//! dimensional. Side selection is by the certified sign of `Nᵢ·(c − Sᵢ)` — an
//! inequality argument, recorded as data on the solution ([`SideSign`]). R7 is
//! polynomial, so Bernstein/exclusion applies directly; no normalized-normal
//! enclosure is formed (N6 vacuous). The module certifies R7 arcs through the
//! §8.3 tube certificate at n = 7 (Theorem 8.1 is n-generic): [`build_frame7`]
//! carries the 307 two-pass discipline at n = 7 (perpendicular block 6×6), and
//! [`c2_certify_tube7`] emits an [`ArcCert<7>`](crate::kernel::certs::ArcCert).
//!
//! **N5 / no premature division.** The R7 residual is evaluated on the
//! D-HOMOGENEOUS cross-multiplied polynomials of the two rational-carrier
//! leaves. For a leaf `S = P/w`, the row `(c − S)·S_u = 0` is carried as
//! `(c·w − P)·(P_u w − P w_u)` and the norm row as
//! `(c·w − P)·(c·w − P) − r²w²`; both are polynomial in `(c, p, q)` and no
//! weight-bearing interval expression is divided anywhere in this module
//! (the §7.1 weight positivity of a box is the leaf oracle's business, and the
//! zero set agrees with the divided residual while the weight stays positive).
//!
//! **N4 / bit-reproducibility.** This module performs no transcendental call:
//! no `sin`, `cos`, `atan2`, `exp`, `ln`, `log`, `powf` anywhere. The only
//! `sqrt` is the IEEE square root used to normalize the kernel direction in
//! [`build_frame7`] (frame normalization, exactly the engine carve-out). The
//! frame basis is built by deterministic two-pass Gram–Schmidt in fixed index
//! order.
//!
//! **Canal (spec §12.2, §16 verbatim).** [`Canal`] stores `spine: ArcId`,
//! `r: f64`, `sigma: (i8, i8)`, and `contact: (DirField, DirField)`. The
//! orthogonality invariant (Prop 12.3: `dᵢ(τ)·c′(τ) = 0` along an R7 branch)
//! is a THEOREM, not an obligation: the type deliberately carries NO
//! orthogonality certificate field (the named audit pins that). The contact
//! direction fields `dᵢ = (c − Sᵢ)/r` are the certified outputs of R7, not
//! results of an intersection (§10.4 case 2). Δ_off (spec §8.7) survives as a
//! named diagnostic ([`DeltaOff`]) — its content is subsumed by the R7
//! regularity certificate (Theorem 12.2), and the module computes it as a
//! check on fixtures, never as a separate precondition.
//!
//! **Corner (spec §12.3).** The three-face corner is compositional by
//! preference: solve `c₁₂(τ) = O₃(u,v)` as the R8 curve–surface system over
//! the S1A seam ([`corner_compositional`]). The direct fallback (9 unknowns,
//! 9 equations, C1 at n = 9) is refused unless the additive pattern extends
//! cheaply: [`corner_unsolved_refusal`] names the refusal so the caller never
//! invents a blend network. Valence ≤ 3 scope (spec §12.4).

use crate::kernel::certs::{ArcCert, Frame};
use crate::kernel::config::{KAPPA_MAX, RHO_MAX, TOL_JACOBIAN};
use crate::kernel::evidence::{ClaimVerdict, Construction, Refusal, RefusalEvidence, RefusalKind};
use crate::kernel::graph::ArcId;
use crate::kernel::leaf::BezierLeaf;
use crate::kernel::patch::{CertifiedPositive, IBox, IBox2};
use crate::kernel::residual::ResidualId;
use crate::kernel::residuals_r89::{BezierLeaf1, R8System};
use crate::kernel::Interval;

/// The three-variable certified-interval vector of `R³` (a point or box).
type Iv3 = [Interval; 3];

/// A named predicate refusal: the kind names the violated class, the predicate
/// name the precise invariant.
fn refusal(kind: RefusalKind, name: &'static str, detail: String) -> Refusal {
    Refusal::new(kind, RefusalEvidence::Predicate { name, detail })
}

/// An empty hull / out-of-domain marker: the fully unbounded interval the leaf
/// hull kernels cannot enclose (a box axis outside a leaf's unit domain).
fn unbounded() -> Interval {
    Interval {
        lo: f64::NEG_INFINITY,
        hi: f64::INFINITY,
    }
}

/// A certified-interval vector in `R³` from an `IBox3` (the leaf oracle's
/// shape), or `None` on a non-finite enclosure.
fn box3_iv(b: crate::kernel::patch::IBox3) -> Option<Iv3> {
    let mut out = [Interval::point(0.0); 3];
    for (k, cell) in out.iter_mut().enumerate() {
        let lo = b.lo[k];
        let hi = b.hi[k];
        if !lo.is_finite() || !hi.is_finite() {
            return None;
        }
        *cell = Interval { lo, hi };
    }
    Some(out)
}

/// The interval dot product of two `R³` interval vectors.
fn dot3(a: &Iv3, b: &Iv3) -> Interval {
    a[0].mul(&b[0]).add(&a[1].mul(&b[1])).add(&a[2].mul(&b[2]))
}

/// The interval squared norm of an `R³` interval vector.
fn norm2_iv(a: &Iv3) -> Interval {
    a[0].mul(&a[0]).add(&a[1].mul(&a[1])).add(&a[2].mul(&a[2]))
}

// ---------------------------------------------------------------------------
// The R7 ball-center residual over two rational-carrier leaves
// ---------------------------------------------------------------------------

/// The §7 R7 ball-center residual system (BG-KV2-402-S7): six polynomial
/// equations in `(c, u, v, s, t) ∈ R⁷` over two rational-carrier surface
/// leaves `S₁(u,v)` and `S₂(s,t)`, in the D-homogeneous cross-multiplied form
/// of the module doc.
///
/// `r` is the rolling-ball radius. The residual is polynomial in all seven
/// unknowns, so Bernstein/exclusion applies directly over every box. The two
/// leaves are the rational Bézier carriers of the two parent faces of the
/// canal.
#[derive(Debug, Clone)]
pub struct R7System {
    /// The first rational-carrier leaf `S₁`.
    a: BezierLeaf,
    /// The second rational-carrier leaf `S₂`.
    b: BezierLeaf,
    /// The rolling-ball radius.
    r: f64,
}

/// Validate a surface leaf for admission to the R7 residual: re-runs the
/// landed `BezierLeaf::try_new` structural checks so a raw leaf cannot reach
/// the residual.
#[allow(clippy::result_large_err)] // Refusal carries Option<PartialGraph> by frozen §2 shape (BG-KV2-000).
fn validate_leaf(leaf: &BezierLeaf) -> Result<(), Refusal> {
    if leaf.degree_u == 0 || leaf.degree_v == 0 {
        return Err(refusal(
            RefusalKind::ClaimRefuted,
            "bezier_zero_degree",
            format!(
                "leaf degrees ({}, {}) must be positive",
                leaf.degree_u, leaf.degree_v
            ),
        ));
    }
    let expected = (leaf.degree_u + 1) * (leaf.degree_v + 1);
    if leaf.control.len() != expected {
        return Err(refusal(
            RefusalKind::ClaimRefuted,
            "bezier_control_count_mismatch",
            format!(
                "control net has {} points, degrees ({}, {}) require {expected}",
                leaf.control.len(),
                leaf.degree_u,
                leaf.degree_v
            ),
        ));
    }
    for (i, p) in leaf.control.iter().enumerate() {
        for c in p {
            if !c.is_finite() {
                return Err(refusal(
                    RefusalKind::NonFinite,
                    "bezier_coordinate_not_finite",
                    format!("control point {i} has a non-finite coordinate: {p:?}"),
                ));
            }
        }
        if p[3] <= 0.0 {
            return Err(refusal(
                RefusalKind::WeightDegenerate,
                "bezier_control_weight_not_positive",
                format!("control point {i} has weight {} which is not > 0", p[3]),
            ));
        }
    }
    Ok(())
}

/// The per-leaf homogeneous factor enclosures over the leaf's parameter box
/// `(p, q)` (the R7 cross-multiply form): for each of the four homogeneous
/// coordinates `(X, Y, Z, w)` and the parameter partials needed by the R7
/// residual and its Jacobian.
///
/// The layout mirrors the 2-var leaf's control net in the hull kernel
/// convention (rows over `p`, columns over `q`).
#[derive(Debug, Clone)]
struct LeafBox {
    /// `X, Y, Z, w` homogeneous coordinate hulls over the box.
    h: [Interval; 4],
    /// The first `p`-partial hulls of `X, Y, Z, w`.
    hp: [Interval; 4],
    /// The first `q`-partial hulls of `X, Y, Z, w`.
    hq: [Interval; 4],
    /// The second `pp`-partial hulls of `X, Y, Z, w`.
    hpp: [Interval; 4],
    /// The second `pq`-partial hulls of `X, Y, Z, w`.
    hpq: [Interval; 4],
    /// The second `qq`-partial hulls of `X, Y, Z, w`.
    hqq: [Interval; 4],
}

/// The homogeneous coordinate grids of a leaf (rows over `u`, columns over
/// `v`), in the layout the 2-var hull kernel consumes.
fn leaf_grid(leaf: &BezierLeaf, comp: usize) -> Vec<Vec<f64>> {
    let width = leaf.degree_v + 1;
    (0..=leaf.degree_u)
        .map(|i| {
            (0..=leaf.degree_v)
                .map(|j| leaf.control[i * width + j][comp])
                .collect()
        })
        .collect()
}

/// The `axis`-partial coordinate grid of a leaf (rows over `u`, columns over
/// `v`).
fn leaf_grid_partial(leaf: &BezierLeaf, comp: usize, axis: usize) -> Vec<Vec<f64>> {
    crate::hull::bernstein_derivative_2d(&leaf_grid(leaf, comp), axis)
}

/// The interval hull of a tensor-Bernstein grid over the `(lo, hi)` sub-box of
/// the unit square, or `None` when the box leaves the unit square or the hull
/// kernel refuses.
fn hull2(grid: &[Vec<f64>], p: &Interval, q: &Interval) -> Option<Interval> {
    crate::hull::hull_bernstein_2d(grid, (p.lo, p.hi), (q.lo, q.hi))
        .ok()
        .filter(|h| h.is_finite())
}

/// Enclose the homogeneous factors of one leaf over its parameter box `(p, q)`.
fn leaf_box_factors(leaf: &BezierLeaf, p: &Interval, q: &Interval) -> LeafBox {
    let mut out = LeafBox {
        h: [Interval::point(0.0); 4],
        hp: [Interval::point(0.0); 4],
        hq: [Interval::point(0.0); 4],
        hpp: [Interval::point(0.0); 4],
        hpq: [Interval::point(0.0); 4],
        hqq: [Interval::point(0.0); 4],
    };
    for comp in 0..4 {
        let grid = leaf_grid(leaf, comp);
        let gu = leaf_grid_partial(leaf, comp, 0);
        let gv = leaf_grid_partial(leaf, comp, 1);
        let guu = crate::hull::bernstein_derivative_2d(&gu, 0);
        let guv = crate::hull::bernstein_derivative_2d(&gu, 1);
        let gvv = crate::hull::bernstein_derivative_2d(&gv, 1);
        out.h[comp] = hull2(&grid, p, q).unwrap_or_else(unbounded);
        out.hp[comp] = hull2(&gu, p, q).unwrap_or_else(unbounded);
        out.hq[comp] = hull2(&gv, p, q).unwrap_or_else(unbounded);
        out.hpp[comp] = hull2(&guu, p, q).unwrap_or_else(unbounded);
        out.hpq[comp] = hull2(&guv, p, q).unwrap_or_else(unbounded);
        out.hqq[comp] = hull2(&gvv, p, q).unwrap_or_else(unbounded);
    }
    out
}

impl R7System {
    /// Build the R7 system over two rational-carrier leaves and a rolling-ball
    /// radius, refusing a leaf that is not a certified rational homogeneous
    /// net (non-finite, zero-degree, or non-positive-weight data) or a
    /// non-finite, non-positive radius.
    #[allow(clippy::result_large_err)] // Refusal carries Option<PartialGraph> by frozen §2 shape (BG-KV2-000).
    pub fn try_new(a: &BezierLeaf, b: &BezierLeaf, r: f64) -> Construction<R7System> {
        validate_leaf(a)?;
        validate_leaf(b)?;
        if !r.is_finite() {
            return Err(refusal(
                RefusalKind::NonFinite,
                "r7_radius_not_finite",
                format!("r7 radius {r} is not finite"),
            ));
        }
        if r <= 0.0 {
            return Err(refusal(
                RefusalKind::ClaimRefuted,
                "r7_radius_not_positive",
                format!("r7 radius {r} must be > 0"),
            ));
        }
        Ok(R7System {
            a: a.clone(),
            b: b.clone(),
            r,
        })
    }

    /// The first rational-carrier leaf `S₁`.
    pub fn a(&self) -> &BezierLeaf {
        &self.a
    }

    /// The second rational-carrier leaf `S₂`.
    pub fn b(&self) -> &BezierLeaf {
        &self.b
    }

    /// The rolling-ball radius `r`.
    pub fn radius(&self) -> f64 {
        self.r
    }

    /// The number of unknown axes: seven `(c, u, v, s, t)`.
    pub fn arity(&self) -> usize {
        7
    }

    /// The number of residual equations: six.
    pub fn nrows(&self) -> usize {
        6
    }

    /// The three residual rows of one leaf's cross-multiplied R7 system over
    /// its parameter box `(p, q)` and the shared centre box `c`: the two
    /// orthogonality rows `(c·w − P)·(P_p w − P w_p)` and `(c·w − P)·(P_q w −
    /// P w_q)` and the norm row `(c·w − P)·(c·w − P) − r²w²`.
    ///
    /// The rows are the D-homogeneous numerators: no division by any weight
    /// enclosure occurs anywhere (N5); the positive weight over the box keeps
    /// the zero set identical to the divided residual's.
    fn leaf_rows(f: &LeafBox, c: &Iv3, r2: &Interval) -> [Interval; 3] {
        // X_k = c_k·w − P_k for k in {X, Y, Z}; D_p,k = P_kp·w − P_k·w_p;
        // D_q,k = P_kq·w − P_k·w_q.
        let w = f.h[3];
        let x = [
            c[0].mul(&w).sub(&f.h[0]),
            c[1].mul(&w).sub(&f.h[1]),
            c[2].mul(&w).sub(&f.h[2]),
        ];
        let d_p = [
            f.hp[0].mul(&w).sub(&f.h[0].mul(&f.hp[3])),
            f.hp[1].mul(&w).sub(&f.h[1].mul(&f.hp[3])),
            f.hp[2].mul(&w).sub(&f.h[2].mul(&f.hp[3])),
        ];
        let d_q = [
            f.hq[0].mul(&w).sub(&f.h[0].mul(&f.hq[3])),
            f.hq[1].mul(&w).sub(&f.h[1].mul(&f.hq[3])),
            f.hq[2].mul(&w).sub(&f.h[2].mul(&f.hq[3])),
        ];
        let row_p = x[0]
            .mul(&d_p[0])
            .add(&x[1].mul(&d_p[1]))
            .add(&x[2].mul(&d_p[2]));
        let row_q = x[0]
            .mul(&d_q[0])
            .add(&x[1].mul(&d_q[1]))
            .add(&x[2].mul(&d_q[2]));
        let row_n = norm2_iv(&x).sub(&r2.mul(&w).mul(&w));
        [row_p, row_q, row_n]
    }

    /// The certified interval residual of the six R7 equations over the joint
    /// seven-var box `b = (c0, c1, c2, u, v, s, t)`.
    ///
    /// Rows `0..=2` are leaf `S₁` over `(u, v)`; rows `3..=5` are leaf `S₂`
    /// over `(s, t)`. A box axis leaving a leaf's unit domain yields the
    /// vacuous unbounded enclosure (never an unsound bound), so the caller
    /// sees a non-finite image and answers Inconclusive.
    pub fn eval(&self, b: &[Interval]) -> Vec<Interval> {
        if b.len() != 7 {
            return vec![unbounded(); 6];
        }
        let c = [b[0], b[1], b[2]];
        let r2 = Interval::point(self.r).mul(&Interval::point(self.r));
        let fa = leaf_box_factors(&self.a, &b[3], &b[4]);
        let fb = leaf_box_factors(&self.b, &b[5], &b[6]);
        let ra = Self::leaf_rows(&fa, &c, &r2);
        let rb = Self::leaf_rows(&fb, &c, &r2);
        vec![ra[0], ra[1], ra[2], rb[0], rb[1], rb[2]]
    }

    /// The certified FLOAT Jacobian of the R7 system at a point: the midpoint
    /// of the certified interval Jacobian over the degenerate point box (the
    /// engine's certified-float-partials convention). Refuses `NonFinite` when
    /// any enclosure is not finite over the point box.
    #[allow(clippy::result_large_err)] // Refusal carries Option<PartialGraph> by frozen §2 shape (BG-KV2-000).
    pub fn float_jacobian_at(&self, point: [f64; 7]) -> Construction<[[f64; 7]; 6]> {
        if !point.iter().all(|c| c.is_finite()) {
            return Err(refusal(
                RefusalKind::NonFinite,
                "r7_jacobian_point_not_finite",
                "the R7 jacobian requires a finite chart point".to_string(),
            ));
        }
        match r7_float_partials(self, point) {
            Some(p) => Ok(p),
            None => Err(refusal(
                RefusalKind::NonFinite,
                "r7_jacobian_partials_unavailable",
                "the certified partials of the R7 system could not be enclosed at the point"
                    .to_string(),
            )),
        }
    }
}

// ---------------------------------------------------------------------------
// The n=7 additive frame construction (307 two-pass discipline at n = 7)
// ---------------------------------------------------------------------------

/// A finite float point of the seven-var chart.
type V7 = [f64; 7];
/// A 6×7 float matrix (the R7 Jacobian).
type M67 = [[f64; 7]; 6];

/// The dot product of two 7-vectors.
fn dot7(a: &V7, b: &V7) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2] + a[3] * b[3] + a[4] * b[4] + a[5] * b[5] + a[6] * b[6]
}

/// The float 6×6 inverse of `m` by Gauss–Jordan elimination with partial
/// pivoting in fixed index order (deterministic; no transcendental calls).
/// `None` on a singular or non-finite matrix.
fn inv6_f64(m: [[f64; 6]; 6]) -> Option<[[f64; 6]; 6]> {
    let mut a = m;
    let mut inv = [[0.0f64; 6]; 6];
    for (i, row) in inv.iter_mut().enumerate() {
        row[i] = 1.0;
    }
    for col in 0..6 {
        // Partial pivot: the first row at/after `col` with the largest
        // absolute pivot.
        let mut pivot = col;
        let mut best = a[col][col].abs();
        for row in a.iter().enumerate().skip(col + 1) {
            let cand = row.1[col].abs();
            if cand > best {
                best = cand;
                pivot = row.0;
            }
        }
        if !best.is_finite() || best == 0.0 {
            return None;
        }
        if pivot != col {
            a.swap(pivot, col);
            inv.swap(pivot, col);
        }
        let d = a[col][col];
        if !d.is_finite() {
            return None;
        }
        for j in 0..6 {
            a[col][j] /= d;
            inv[col][j] /= d;
        }
        for row in 0..6 {
            if row == col {
                continue;
            }
            let factor = a[row][col];
            if !factor.is_finite() {
                return None;
            }
            for j in 0..6 {
                a[row][j] -= factor * a[col][j];
                inv[row][j] -= factor * inv[col][j];
            }
        }
    }
    if a.iter().flatten().any(|c| !c.is_finite()) || inv.iter().flatten().any(|c| !c.is_finite()) {
        return None;
    }
    Some(inv)
}

/// The `max` row-absolute-sum norm of a 6×6 float matrix.
fn norm_inf6(m: &[[f64; 6]; 6]) -> f64 {
    let mut best = 0.0f64;
    for row in m {
        let s = row.iter().map(|c| c.abs()).sum::<f64>();
        best = best.max(s);
    }
    best
}

/// The float dot product of the `r`-th Jacobian row against a 7-vector `v`.
fn row_dot(row: &[f64; 7], v: &V7) -> f64 {
    dot7(row, v)
}

/// The certified float partials of the R7 system at a point (evaluated over a
/// degenerate point box and taking the midpoint of each enclosure — the
/// engine's certified-float-partials convention). `None` when any enclosure is
/// not finite.
fn r7_float_partials(system: &R7System, point: V7) -> Option<M67> {
    let box_: Vec<Interval> = point.iter().map(|x| Interval::point(*x)).collect();
    let jac = r7_jac_encl(system, &box_);
    let mut out = [[0.0f64; 7]; 6];
    for (row, out_row) in out.iter_mut().enumerate() {
        for (col, cell) in out_row.iter_mut().enumerate() {
            let enc = jac[row][col];
            if !enc.is_finite() {
                return None;
            }
            *cell = 0.5 * (enc.lo + enc.hi);
        }
    }
    Some(out)
}

/// The certified interval Jacobian of the R7 system (6 rows × 7 columns) over
/// a seven-var interval box, in the D-homogeneous form: the partial
/// derivatives of the cross-multiplied rows.
fn r7_jac_encl(system: &R7System, b: &[Interval]) -> [[Interval; 7]; 6] {
    if b.len() != 7 {
        return [[unbounded(); 7]; 6];
    }
    let c = [b[0], b[1], b[2]];
    let r2 = Interval::point(system.r).mul(&Interval::point(system.r));
    let fa = leaf_box_factors(&system.a, &b[3], &b[4]);
    let fb = leaf_box_factors(&system.b, &b[5], &b[6]);
    let mut out = [[Interval::point(0.0); 7]; 6];
    let ra = leaf_jac_rows(&fa, &c, &r2);
    for row in 0..3 {
        out[row][0] = ra[row][0];
        out[row][1] = ra[row][1];
        out[row][2] = ra[row][2];
        out[row][3] = ra[row][3];
        out[row][4] = ra[row][4];
    }
    let rb = leaf_jac_rows(&fb, &c, &r2);
    for row in 0..3 {
        out[3 + row][0] = rb[row][0];
        out[3 + row][1] = rb[row][1];
        out[3 + row][2] = rb[row][2];
        out[3 + row][5] = rb[row][3];
        out[3 + row][6] = rb[row][4];
    }
    out
}

/// The partial rows of one leaf's three cross-multiplied R7 rows with respect
/// to `(c0, c1, c2, p, q)`, in that order. Each row is a 5-vector; the caller
/// splices them into the 7-var chart columns.
///
/// With `X_k = c_k·w − P_k`, `D_p,k = P_kp·w − P_k·w_p`, `D_q,k = P_kq·w −
/// P_k·w_q`:
///
/// ```text
/// ∂(X·D_p)/∂c_l = w·D_p,l            ∂(X·D_q)/∂c_l = w·D_q,l
/// ∂(X·X − r²w²)/∂c_l = 2·X_l·w
/// ```
///
/// and the parameter partials follow by the product rule; all factors are the
/// homogeneous hulls (see [`leaf_box_factors`]).
fn leaf_jac_rows(f: &LeafBox, c: &Iv3, r2: &Interval) -> [[Interval; 5]; 3] {
    let w = f.h[3];
    let x = [
        c[0].mul(&w).sub(&f.h[0]),
        c[1].mul(&w).sub(&f.h[1]),
        c[2].mul(&w).sub(&f.h[2]),
    ];
    let d_p = [
        f.hp[0].mul(&w).sub(&f.h[0].mul(&f.hp[3])),
        f.hp[1].mul(&w).sub(&f.h[1].mul(&f.hp[3])),
        f.hp[2].mul(&w).sub(&f.h[2].mul(&f.hp[3])),
    ];
    let d_q = [
        f.hq[0].mul(&w).sub(&f.h[0].mul(&f.hq[3])),
        f.hq[1].mul(&w).sub(&f.h[1].mul(&f.hq[3])),
        f.hq[2].mul(&w).sub(&f.h[2].mul(&f.hq[3])),
    ];
    // xp_k = ∂X_k/∂p = c_k·w_p − P_kp; xq_k = ∂X_k/∂q = c_k·w_q − P_kq.
    let xp = [
        c[0].mul(&f.hp[3]).sub(&f.hp[0]),
        c[1].mul(&f.hp[3]).sub(&f.hp[1]),
        c[2].mul(&f.hp[3]).sub(&f.hp[2]),
    ];
    let xq = [
        c[0].mul(&f.hq[3]).sub(&f.hq[0]),
        c[1].mul(&f.hq[3]).sub(&f.hq[1]),
        c[2].mul(&f.hq[3]).sub(&f.hq[2]),
    ];
    // D_p,kp = P_kpp·w − P_k·w_pp   (the p-partial of D_p,k, product rule)
    // D_p,kp = P_kpp·w − P_k·w_pp   (the p-partial of D_p,k, product rule)
    let d_p_p = [
        f.hpp[0].mul(&w).sub(&f.h[0].mul(&f.hpp[3])),
        f.hpp[1].mul(&w).sub(&f.h[1].mul(&f.hpp[3])),
        f.hpp[2].mul(&w).sub(&f.h[2].mul(&f.hpp[3])),
    ];
    // D_q,kp = P_kpq·w + P_kq·w_p − P_kp·w_q − P_k·w_pq (the p-partial of
    // D_q,k = P_kq·w − P_k·w_q).
    let d_q_p = [
        f.hpq[0]
            .mul(&w)
            .add(&f.hq[0].mul(&f.hp[3]))
            .sub(&f.hp[0].mul(&f.hq[3]))
            .sub(&f.h[0].mul(&f.hpq[3])),
        f.hpq[1]
            .mul(&w)
            .add(&f.hq[1].mul(&f.hp[3]))
            .sub(&f.hp[1].mul(&f.hq[3]))
            .sub(&f.h[1].mul(&f.hpq[3])),
        f.hpq[2]
            .mul(&w)
            .add(&f.hq[2].mul(&f.hp[3]))
            .sub(&f.hp[2].mul(&f.hq[3]))
            .sub(&f.h[2].mul(&f.hpq[3])),
    ];
    // D_q,kq = P_kqq·w − P_k·w_qq.
    let d_q_q = [
        f.hqq[0].mul(&w).sub(&f.h[0].mul(&f.hqq[3])),
        f.hqq[1].mul(&w).sub(&f.h[1].mul(&f.hqq[3])),
        f.hqq[2].mul(&w).sub(&f.h[2].mul(&f.hqq[3])),
    ];
    // D_p,kq = P_kpq·w + P_kp·w_q − P_kq·w_p − P_k·w_pq (q-partial of D_p,k).
    let d_p_q = [
        f.hpq[0]
            .mul(&w)
            .add(&f.hp[0].mul(&f.hq[3]))
            .sub(&f.hq[0].mul(&f.hp[3]))
            .sub(&f.h[0].mul(&f.hpq[3])),
        f.hpq[1]
            .mul(&w)
            .add(&f.hp[1].mul(&f.hq[3]))
            .sub(&f.hq[1].mul(&f.hp[3]))
            .sub(&f.h[1].mul(&f.hpq[3])),
        f.hpq[2]
            .mul(&w)
            .add(&f.hp[2].mul(&f.hq[3]))
            .sub(&f.hq[2].mul(&f.hp[3]))
            .sub(&f.h[2].mul(&f.hpq[3])),
    ];
    let mut out = [[Interval::point(0.0); 5]; 3];
    // Row 0 (X·D_p): ∂/∂c_l = w·D_p,l.
    out[0][0] = w.mul(&d_p[0]);
    out[0][1] = w.mul(&d_p[1]);
    out[0][2] = w.mul(&d_p[2]);
    // ∂/∂p = Σ_k (xp_k·D_p,k + X_k·D_p,kp).
    let mut acc_p = xp[0].mul(&d_p[0]).add(&x[0].mul(&d_p_p[0]));
    for k in 1..3 {
        acc_p = acc_p.add(&xp[k].mul(&d_p[k]).add(&x[k].mul(&d_p_p[k])));
    }
    out[0][3] = acc_p;
    // ∂/∂q = Σ_k (xq_k·D_p,k + X_k·D_p,kq).
    let mut acc_q = xq[0].mul(&d_p[0]).add(&x[0].mul(&d_p_q[0]));
    for k in 1..3 {
        acc_q = acc_q.add(&xq[k].mul(&d_p[k]).add(&x[k].mul(&d_p_q[k])));
    }
    out[0][4] = acc_q;

    // Row 1 (X·D_q): ∂/∂c_l = w·D_q,l.
    out[1][0] = w.mul(&d_q[0]);
    out[1][1] = w.mul(&d_q[1]);
    out[1][2] = w.mul(&d_q[2]);
    // ∂/∂p = Σ_k (xp_k·D_q,k + X_k·D_q,kp).
    let mut acc_p = xp[0].mul(&d_q[0]).add(&x[0].mul(&d_q_p[0]));
    for k in 1..3 {
        acc_p = acc_p.add(&xp[k].mul(&d_q[k]).add(&x[k].mul(&d_q_p[k])));
    }
    out[1][3] = acc_p;
    // ∂/∂q = Σ_k (xq_k·D_q,k + X_k·D_q,kq).
    let mut acc_q = xq[0].mul(&d_q[0]).add(&x[0].mul(&d_q_q[0]));
    for k in 1..3 {
        acc_q = acc_q.add(&xq[k].mul(&d_q[k]).add(&x[k].mul(&d_q_q[k])));
    }
    out[1][4] = acc_q;

    // Row 2 (X·X − r²w²): ∂/∂c_l = 2·X_l·w.
    let two = Interval::point(2.0);
    out[2][0] = two.mul(&x[0]).mul(&w);
    out[2][1] = two.mul(&x[1]).mul(&w);
    out[2][2] = two.mul(&x[2]).mul(&w);
    // ∂/∂p = Σ_k 2·X_k·xp_k − 2·r²·w·w_p.
    let acc_p = two
        .mul(&x[0])
        .mul(&xp[0])
        .add(&two.mul(&x[1]).mul(&xp[1]))
        .add(&two.mul(&x[2]).mul(&xp[2]))
        .sub(&two.mul(r2).mul(&w).mul(&f.hp[3]));
    out[2][3] = acc_p;
    // ∂/∂q = Σ_k 2·X_k·xq_k − 2·r²·w·w_q.
    let acc_q = two
        .mul(&x[0])
        .mul(&xq[0])
        .add(&two.mul(&x[1]).mul(&xq[1]))
        .add(&two.mul(&x[2]).mul(&xq[2]))
        .sub(&two.mul(r2).mul(&w).mul(&f.hq[3]));
    out[2][4] = acc_q;

    out
}

// ---------------------------------------------------------------------------
// The frame at n = 7 and the C2 tube at n = 7
// ---------------------------------------------------------------------------

/// The outcome of a §8.1 frame construction at n = 7: the frame and the float
/// kernel direction `m`.
#[derive(Debug, Clone)]
pub struct FrameBuild7 {
    /// The §8.1 frame.
    pub frame: Frame<7>,
    /// The float maximal-minor kernel direction at `z_hat`.
    pub m: V7,
}

/// The determinant of a square float matrix (recursive cofactor expansion along
/// row 0, deterministic op order). `None` when the matrix is not square.
fn det_square(m: &[Vec<f64>]) -> Option<f64> {
    let n = m.len();
    if n == 0 {
        return Some(1.0);
    }
    if m.iter().any(|row| row.len() != n) {
        return None;
    }
    if n == 1 {
        return Some(m[0][0]);
    }
    let mut acc = 0.0f64;
    for c in 0..n {
        if m[0][c] == 0.0 {
            continue;
        }
        let mut minor = vec![Vec::with_capacity(n - 1); n - 1];
        for (r, row) in m.iter().enumerate().skip(1) {
            for (d, &v) in row.iter().enumerate().take(n) {
                if d != c {
                    minor[r - 1].push(v);
                }
            }
        }
        let sign = if c % 2 == 0 { 1.0 } else { -1.0 };
        let sub = det_square(&minor)?;
        acc += sign * m[0][c] * sub;
    }
    Some(acc)
}

/// The determinant of a 6×6 float matrix (see [`det_square`]).
fn det6_f64(m: &[[f64; 6]; 6]) -> Option<f64> {
    let rows: Vec<Vec<f64>> = m.iter().map(|row| row.to_vec()).collect();
    det_square(&rows)
}

/// The maximal-minor (kernel-direction) vector of a 6×7 float matrix with
/// Theorem 6.4's sign pattern at n = 7: `m_j = (−1)^j det(DF with column j
/// deleted)`.
fn kernel_minors7(rows: &M67) -> [f64; 7] {
    let minor = |skip: usize| -> f64 {
        let mut m = [[0.0f64; 6]; 6];
        for (r, row) in rows.iter().enumerate() {
            let mut cc = 0usize;
            for (c, &v) in row.iter().enumerate() {
                if c == skip {
                    continue;
                }
                m[r][cc] = v;
                cc += 1;
            }
        }
        // A 6x6 cofactor is always computable for a finite matrix; a non-square
        // submatrix cannot arise from the skip construction.
        det6_f64(&m).unwrap_or(f64::NAN)
    };
    let mut m = [0.0f64; 7];
    for (j, cell) in m.iter_mut().enumerate() {
        let sign = if j % 2 == 0 { 1.0 } else { -1.0 };
        *cell = sign * minor(j);
    }
    m
}

/// §8.1 frame construction at n = 7 (the additive n = 7 sibling of the
/// 307-hardened `build_frame4`): the R7 Jacobian at `z_hat`, the maximal-minor
/// kernel direction `m` (normalized by IEEE `sqrt` — the frame-normalization
/// carve-out), and the perpendicular basis by TWO-PASS (reorthogonalized)
/// Gram–Schmidt in FIXED index order. The 6×6 preconditioner `a =
/// [DF(ẑ)·Q_⊥]⁻¹` is embedded in the 7×7 frame field with the tangent axis
/// carried identically.
///
/// If `||m||` is below the normative floor (rank < 6 territory) or the frame
/// Jacobian block is singular, refuses `Conditioning` (Inconclusive) — the
/// caller subdivides.
// Refusal carries Option<PartialGraph> by frozen §2 shape; large-Err is allowed (BG-KV2-000).
#[allow(clippy::result_large_err)]
pub fn build_frame7(system: &R7System, z_hat: V7) -> Construction<FrameBuild7> {
    if !z_hat.iter().all(|c| c.is_finite()) {
        return Err(refusal(
            RefusalKind::NonFinite,
            "frame7_z_hat_not_finite",
            "build_frame7 requires a finite chart point".to_string(),
        ));
    }
    let partials = match r7_float_partials(system, z_hat) {
        Some(p) => p,
        None => {
            return Err(refusal(
                RefusalKind::Conditioning,
                "frame7_partials_unavailable",
                "the certified partials of the R7 system could not be enclosed at z_hat"
                    .to_string(),
            ))
        }
    };
    let m = kernel_minors7(&partials);
    let norm_sq = dot7(&m, &m);
    if !norm_sq.is_finite() || norm_sq <= TOL_JACOBIAN * TOL_JACOBIAN {
        return Err(refusal(
            RefusalKind::Conditioning,
            "frame7_kernel_direction_degenerate",
            "the maximal-minor kernel direction of DR7 at z_hat is degenerate (rank < 6)"
                .to_string(),
        ));
    }
    let norm = norm_sq.sqrt();
    let q_tau = [
        m[0] / norm,
        m[1] / norm,
        m[2] / norm,
        m[3] / norm,
        m[4] / norm,
        m[5] / norm,
        m[6] / norm,
    ];

    // Deterministic TWO-PASS Gram–Schmidt over the fixed candidate order
    // e_0..e_6 (the 307 discipline at n = 7). Each candidate is projected
    // against the accepted basis twice.
    let mut perp: Vec<V7> = Vec::with_capacity(6);
    let mut basis: Vec<V7> = vec![q_tau];
    for k in 0..7 {
        if perp.len() == 6 {
            break;
        }
        let mut e = [0.0f64; 7];
        e[k] = 1.0;
        let mut v = e;
        for _pass in 0..2 {
            for fixed in &basis {
                let d = dot7(&v, fixed);
                for j in 0..7 {
                    v[j] -= d * fixed[j];
                }
            }
        }
        let v_norm_sq = dot7(&v, &v);
        if !v_norm_sq.is_finite() || v_norm_sq <= TOL_JACOBIAN * TOL_JACOBIAN {
            continue;
        }
        let v_norm = v_norm_sq.sqrt();
        let unit = [
            v[0] / v_norm,
            v[1] / v_norm,
            v[2] / v_norm,
            v[3] / v_norm,
            v[4] / v_norm,
            v[5] / v_norm,
            v[6] / v_norm,
        ];
        if basis
            .iter()
            .any(|fixed| dot7(&unit, fixed).abs() > TOL_JACOBIAN)
        {
            return Err(refusal(
                RefusalKind::Conditioning,
                "frame7_two_pass_residual_above_gate",
                "after two Gram-Schmidt passes a perpendicular candidate still has a residual \
                 dot above TOL_JACOBIAN: the seed is genuinely near-rank-collapse (the caller \
                 subdivides)"
                    .to_string(),
            ));
        }
        basis.push(unit);
        perp.push(unit);
    }
    if perp.len() != 6 {
        return Err(refusal(
            RefusalKind::Conditioning,
            "frame7_perp_basis_degenerate",
            "Gram-Schmidt could not build a 6-dimensional perpendicular basis".to_string(),
        ));
    }

    // The perpendicular Jacobian block B = DF(z_hat)·Q_⊥ (6×6), its inverse,
    // and the frame's κ estimate.
    let mut b = [[0.0f64; 6]; 6];
    for (r, b_row) in b.iter_mut().enumerate() {
        for (c, cell) in b_row.iter_mut().enumerate() {
            *cell = row_dot(&partials[r], &perp[c]);
        }
    }
    let a66 = match inv6_f64(b) {
        Some(a) => a,
        None => {
            return Err(refusal(
                RefusalKind::Conditioning,
                "frame7_perp_jacobian_singular",
                "the perpendicular Jacobian block [DF(z_hat) Q_perp] is singular".to_string(),
            ))
        }
    };

    // The Frame.a field is 7×7; embed the 6×6 preconditioner with the tangent
    // axis carried identically (row/column 6 = tau).
    let mut a = [[0.0f64; 7]; 7];
    for r in 0..6 {
        for c in 0..6 {
            a[r][c] = a66[r][c];
        }
    }
    a[6][6] = 1.0;

    let mut q = [[0.0f64; 7]; 7];
    q[0].copy_from_slice(&q_tau);
    for (i, p) in perp.iter().enumerate() {
        q[i + 1].copy_from_slice(p);
    }
    // q_perp: columns 0..=5 = perp, final column = q_tau.
    let mut q_perp = [[0.0f64; 7]; 7];
    for (i, p) in perp.iter().enumerate() {
        q_perp[i].copy_from_slice(p);
    }
    q_perp[6].copy_from_slice(&q_tau);

    let frame = Frame::try_new(z_hat, q, q_tau, q_perp, a)?;
    Ok(FrameBuild7 { frame, m })
}

// ---------------------------------------------------------------------------
// The C2 tube at n = 7 (Theorem 8.1, additive sibling of c2_certify_tube4)
// ---------------------------------------------------------------------------

type Iv6 = [Interval; 6];
type M6 = [[Interval; 6]; 6];

/// The componentwise magnitude `mag(v) = max(|lo|, |hi|)` of an interval.
fn mag(v: &Interval) -> f64 {
    v.lo.abs().max(v.hi.abs())
}

/// The interval 6×6 matrix product.
fn matmul6(a: &M6, b: &M6) -> M6 {
    let mut out = [[Interval::point(0.0); 6]; 6];
    for r in 0..6 {
        for c in 0..6 {
            let mut acc = Interval::point(0.0);
            for k in 0..6 {
                acc = acc.add(&a[r][k].mul(&b[k][c]));
            }
            out[r][c] = acc;
        }
    }
    out
}

/// The interval 6×6 matrix times a 6-vector.
fn matvec6(m: &M6, v: &Iv6) -> Iv6 {
    let mut out = [Interval::point(0.0); 6];
    for (r, out_r) in out.iter_mut().enumerate() {
        let mut acc = Interval::point(0.0);
        for k in 0..6 {
            acc = acc.add(&m[r][k].mul(&v[k]));
        }
        *out_r = acc;
    }
    out
}

/// The outward-rounded centred box `B − centre`.
fn centred_dx6(b: &IBox<6>, z: &Iv6) -> Iv6 {
    let mut out = [Interval::point(0.0); 6];
    for (k, cell) in out.iter_mut().enumerate() {
        let d_lo = Interval::point(b.lo[k]).sub(&z[k]);
        let d_hi = Interval::point(b.hi[k]).sub(&z[k]);
        *cell = Interval {
            lo: d_lo.lo.min(d_hi.lo),
            hi: d_lo.hi.max(d_hi.hi),
        };
    }
    out
}

/// The three-valued inclusion classification of a Krawczyk image axis.
enum Inclusion {
    /// `K(B)` is component-wise strictly inside `B`.
    Strict,
    /// `K(B)` is disjoint from `B`.
    Disjoint,
    /// `K(B)` overlaps `B` but is not strictly inside.
    Overlap,
}

fn classify_axis(lo: f64, hi: f64, k_lo: f64, k_hi: f64) -> Inclusion {
    if lo < k_lo && k_hi < hi {
        Inclusion::Strict
    } else if k_hi <= lo || hi <= k_lo {
        Inclusion::Disjoint
    } else {
        Inclusion::Overlap
    }
}

/// The chart-space point `z_hat + q_tau·tau + Σ_c q_perp[c]·y_c` (n = 7).
fn chart_point7(frame: &Frame<7>, tau: f64, y: [f64; 6]) -> V7 {
    let mut out = [0.0f64; 7];
    for (j, out_j) in out.iter_mut().enumerate() {
        let mut v = frame.z_hat[j] + frame.q_tau[j] * tau;
        for (c, y_c) in y.iter().enumerate() {
            v += frame.q_perp[c][j] * y_c;
        }
        *out_j = v;
    }
    out
}

/// The interval chart-space hull of the tube: the seven interval coordinates
/// spanned by `(I_tau, y_1..y_6)` under the frame map.
fn frame_tube_chart_box7(frame: &Frame<7>, i_tau: Interval, y: &Iv6) -> [Interval; 7] {
    let mut acc = [
        Interval::point(frame.z_hat[0]),
        Interval::point(frame.z_hat[1]),
        Interval::point(frame.z_hat[2]),
        Interval::point(frame.z_hat[3]),
        Interval::point(frame.z_hat[4]),
        Interval::point(frame.z_hat[5]),
        Interval::point(frame.z_hat[6]),
    ];
    for (j, acc_j) in acc.iter_mut().enumerate() {
        let tau_term = Interval::point(frame.q_tau[j]).mul(&i_tau);
        *acc_j = acc_j.add(&tau_term);
        for (c, y_c) in y.iter().enumerate() {
            let term = Interval::point(frame.q_perp[c][j]).mul(y_c);
            *acc_j = acc_j.add(&term);
        }
    }
    acc
}

/// The §8.3 one-dimensional (tube) certificate at n = 7 over the R7 family:
/// the additive n = 7 sibling of the 307-hardened `c2_certify_tube4`. The six
/// perpendicular equations are evaluated over the JOINT box `(I_tau, B_perp)`
/// in frame coordinates; a strictly-including perpendicular image with
/// `rho ≤ RHO_MAX` emits an [`ArcCert<7>`](crate::kernel::certs::ArcCert) with
/// residual `R7`.
///
/// Refusal backing: a perpendicular image that is not strictly inside `B_perp`
/// is Inconclusive (shrink-and-retry is licensed); a near-singular
/// preconditioner beyond [`KAPPA_MAX`] is Inconclusive (`Conditioning`).
// Refusal carries Option<PartialGraph> by frozen §2 shape; large-Err is allowed (BG-KV2-000).
#[allow(clippy::result_large_err)]
pub fn c2_certify_tube7(
    system: &R7System,
    frame: &Frame<7>,
    i_tau: Interval,
    b_perp: IBox<6>,
) -> ClaimVerdict<ArcCert<7>, Refusal, &'static str> {
    if !i_tau.is_finite() || i_tau.lo > i_tau.hi {
        return ClaimVerdict::Disproven(refusal(
            RefusalKind::ClaimRefuted,
            "tube7_i_tau_invalid",
            "i_tau must be a finite, ordered interval".to_string(),
        ));
    }

    // Perpendicular radii and the frame-coordinate centre.
    let mut r = [0.0f64; 6];
    for (k, cell) in r.iter_mut().enumerate() {
        *cell = (b_perp.hi[k] - b_perp.lo[k]) / 2.0;
    }
    if r.iter().any(|c| !c.is_finite() || *c <= 0.0) {
        return ClaimVerdict::Disproven(refusal(
            RefusalKind::NonFinite,
            "tube7_radius_nonpositive",
            "c2_certify_tube7 requires a strictly positive finite radius on every perpendicular axis"
                .to_string(),
        ));
    }
    let mut y_hat = [0.0f64; 6];
    for (k, cell) in y_hat.iter_mut().enumerate() {
        *cell = (b_perp.lo[k] + b_perp.hi[k]) / 2.0;
    }
    let tau_mid = (i_tau.lo + i_tau.hi) / 2.0;
    let z_mid = chart_point7(frame, tau_mid, y_hat);

    // The float perpendicular Jacobian at the midpoint and its inverse A.
    let partials = match r7_float_partials(system, z_mid) {
        Some(p) => p,
        None => return ClaimVerdict::Inconclusive("tube7_partials_unavailable"),
    };
    let mut b = [[0.0f64; 6]; 6];
    for (r_idx, b_row) in b.iter_mut().enumerate() {
        for (c, cell) in b_row.iter_mut().enumerate() {
            *cell = row_dot(&partials[r_idx], &perp_of(frame, c));
        }
    }
    let a = match inv6_f64(b) {
        Some(a) => a,
        None => return ClaimVerdict::Inconclusive("tube7_midpoint_jacobian_singular"),
    };
    let cond = norm_inf6(&b) * norm_inf6(&a);
    if !cond.is_finite() || cond > KAPPA_MAX {
        return ClaimVerdict::Inconclusive("tube7_midpoint_conditioning");
    }

    // The interval perpendicular boxes (joint box and centre slice).
    let mut y_iv = [Interval::point(0.0); 6];
    let mut yc_iv = [Interval::point(0.0); 6];
    for k in 0..6 {
        y_iv[k] = Interval {
            lo: b_perp.lo[k],
            hi: b_perp.hi[k],
        };
        yc_iv[k] = Interval::point(y_hat[k]);
    }
    let joint_box = frame_tube_chart_box7(frame, i_tau, &y_iv);
    let slice_box = frame_tube_chart_box7(frame, i_tau, &yc_iv);

    // F over the centre slice and D_yF over the joint box (interval).
    let f_slice = system.eval(&slice_box);
    if f_slice.len() != 6 || f_slice.iter().any(|v| !v.is_finite()) {
        return ClaimVerdict::Inconclusive("tube7_value_enclosure_failed");
    }
    let df_joint = r7_jac_encl(system, &joint_box);
    if df_joint.iter().flatten().any(|v| !v.is_finite()) {
        return ClaimVerdict::Inconclusive("tube7_jacobian_enclosure_failed");
    }
    // dy[r][c] = Σ_j df_joint[r][j]·frame.q_perp[c][j].
    let mut dy = [[Interval::point(0.0); 6]; 6];
    for (r, dy_row) in dy.iter_mut().enumerate() {
        for (c, cell) in dy_row.iter_mut().enumerate() {
            let mut acc = Interval::point(0.0);
            for (j, df_rj) in df_joint[r].iter().enumerate() {
                acc = acc.add(&df_rj.mul(&Interval::point(frame.q_perp[c][j])));
            }
            *cell = acc;
        }
    }
    if dy.iter().flatten().any(|v| !v.is_finite()) {
        return ClaimVerdict::Inconclusive("tube7_enclosure_not_finite");
    }

    // K = ŷ − A·F(□I_tau, ŷ) + (I − A·□D_yF)(B_perp − ŷ).
    let mut a_iv = [[Interval::point(0.0); 6]; 6];
    for r in 0..6 {
        for c in 0..6 {
            a_iv[r][c] = Interval::point(a[r][c]);
        }
    }
    let af = matvec6(&a_iv, &slice_vec(&f_slice));
    let cj = matmul6(&a_iv, &dy);
    let mut id_minus = [[Interval::point(0.0); 6]; 6];
    for r in 0..6 {
        for c in 0..6 {
            if r == c {
                id_minus[r][c] = Interval::point(1.0).sub(&cj[r][c]);
            } else {
                id_minus[r][c] = cj[r][c].neg();
            }
        }
    }
    if id_minus.iter().flatten().any(|v| !v.is_finite()) {
        return ClaimVerdict::Inconclusive("tube7_enclosure_not_finite");
    }
    let dx = centred_dx6(&b_perp, &yc_iv);
    let md = matvec6(&id_minus, &dx);
    let mut k = [Interval::point(0.0); 6];
    for i in 0..6 {
        k[i] = yc_iv[i].sub(&af[i]).add(&md[i]);
    }
    if k.iter().any(|v| !v.is_finite()) {
        return ClaimVerdict::Inconclusive("tube7_enclosure_not_finite");
    }

    // Strict inclusion of the perpendicular image in B_perp for ALL tau.
    for ((lo_i, hi_i), k_i) in b_perp.lo.iter().zip(b_perp.hi.iter()).zip(k.iter()) {
        match classify_axis(*lo_i, *hi_i, k_i.lo, k_i.hi) {
            Inclusion::Strict => {}
            _ => return ClaimVerdict::Inconclusive("tube7_perpendicular_image_not_strict"),
        }
    }

    // Lemma 8.0's contraction rate over B_perp's radii.
    let mut rho = 0.0f64;
    for (i, row) in id_minus.iter().enumerate() {
        let mut mr = 0.0f64;
        for c in 0..6 {
            mr += mag(&row[c]) * r[c];
        }
        let ratio = mr / r[i];
        if !ratio.is_finite() {
            return ClaimVerdict::Inconclusive("tube7_rho_not_finite");
        }
        rho = rho.max(ratio);
    }
    if rho > RHO_MAX {
        return ClaimVerdict::Inconclusive("tube7_rho_exceeds_rho_max");
    }

    // Per-column Jacobian enclosures of D_yF over the joint box.
    let mut jac_encl = Vec::with_capacity(6);
    for col in 0..6 {
        let mut lo = f64::INFINITY;
        let mut hi = f64::NEG_INFINITY;
        for row in dy.iter() {
            lo = lo.min(row[col].lo);
            hi = hi.max(row[col].hi);
        }
        jac_encl.push([lo, hi]);
    }

    // Lift the box into the q_perp-aligned IBox<7> convention: axes 0..=5 are
    // the perpendicular coordinates, axis 6 is the tangent interval.
    let mut lo7 = [0.0f64; 7];
    let mut hi7 = [0.0f64; 7];
    lo7[..6].copy_from_slice(&b_perp.lo);
    hi7[..6].copy_from_slice(&b_perp.hi);
    lo7[6] = i_tau.lo;
    hi7[6] = i_tau.hi;
    let b_perp7 = match IBox::<7>::try_new(lo7, hi7) {
        Ok(b) => b,
        Err(_) => return ClaimVerdict::Inconclusive("tube7_box_lift_failed"),
    };

    match ArcCert::try_new(ResidualId::R7, *frame, i_tau, b_perp7, rho, jac_encl, None) {
        Ok(cert) => ClaimVerdict::Proven(cert),
        Err(r) => ClaimVerdict::Disproven(r),
    }
}

/// The perpendicular column vector `c` of the frame (columns `0..=5` of
/// `q_perp`, which are the perp basis; column 6 is `q_tau`).
fn perp_of(frame: &Frame<7>, c: usize) -> V7 {
    frame.q_perp[c]
}

/// Convert a residual value vector into the fixed-size slice vector.
fn slice_vec(v: &[Interval]) -> Iv6 {
    let mut out = [Interval::point(0.0); 6];
    for (i, cell) in out.iter_mut().enumerate() {
        *cell = v[i];
    }
    out
}

// ---------------------------------------------------------------------------
// Side sign, DirField, Canal, and the Delta_off diagnostic
// ---------------------------------------------------------------------------

/// The certified side sign of an R7 ball-center solution (spec §12.1): the
/// certified sign of `Nᵢ·(c − Sᵢ)` for each parent face. Side selection is an
/// INEQUALITY argument and is recorded as DATA on the solution — never as an
/// equation of the residual.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SideSign {
    /// The certified sign for face `S₁`.
    pub s1: i8,
    /// The certified sign for face `S₂`.
    pub s2: i8,
}

impl SideSign {
    /// The canonical side sign of a two-parent pair from the two signed
    /// distances, refusing a non-finite or zero value.
    #[allow(clippy::result_large_err)] // Refusal carries Option<PartialGraph> by frozen §2 shape (BG-KV2-000).
    pub fn try_new(n1_dot: f64, n2_dot: f64) -> Construction<SideSign> {
        let s1 = sign_i8(n1_dot, "side1")?;
        let s2 = sign_i8(n2_dot, "side2")?;
        Ok(SideSign { s1, s2 })
    }

    /// The `(σ₁, σ₂)` pair this side sign records.
    pub fn pair(&self) -> (i8, i8) {
        (self.s1, self.s2)
    }
}

// Refusal carries Option<PartialGraph> by frozen §2 shape; large-Err is allowed (BG-KV2-000).
#[allow(clippy::result_large_err)]
fn sign_i8(v: f64, what: &'static str) -> Result<i8, Refusal> {
    if !v.is_finite() {
        return Err(refusal(
            RefusalKind::NonFinite,
            "r7_side_dot_not_finite",
            format!("the certified {what} dot product {v} is not finite"),
        ));
    }
    if v > 0.0 {
        Ok(1)
    } else if v < 0.0 {
        Ok(-1)
    } else {
        Err(refusal(
            RefusalKind::ClaimRefuted,
            "r7_side_dot_zero",
            format!("the certified {what} dot product {v} is exactly zero: the ball center lies ON the tangent plane"),
        ))
    }
}

/// The certified contact direction field of one parent face of a canal
/// (spec §12.2): `dᵢ = (c − Sᵢ)/r`, the certified output of R7. The field is
/// a unit vector field along the spine arc; the orthogonality of `dᵢ` to the
/// spine tangent is Prop 12.3's THEOREM and is deliberately NOT carried as a
/// certificate field.
///
/// `DirField` stores the certified reference data of one contact direction: the
/// parent-face index (1 or 2), the side sign `σᵢ`, and the certified unit
/// direction `d` at a spine reference point. Along-arc evaluations are the R7
/// outputs themselves (§10.4 case 2) and need no separate certificate.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DirField {
    /// The parent face index: `1` for `S₁`, `2` for `S₂`.
    pub face: u8,
    /// The side sign `σᵢ` of this contact direction.
    pub sigma: i8,
    /// The certified unit contact direction `d = (c − Sᵢ)/r` at the reference
    /// point.
    pub d: [f64; 3],
}

impl DirField {
    /// Build a certified contact direction field datum, refusing a non-unit
    /// `d` (unit slack [`TOL_JACOBIAN`]), a non-finite `d`, a `face` outside
    /// `{1, 2}`, or a `sigma` outside `{−1, +1}`.
    #[allow(clippy::result_large_err)] // Refusal carries Option<PartialGraph> by frozen §2 shape (BG-KV2-000).
    pub fn try_new(face: u8, sigma: i8, d: [f64; 3]) -> Construction<DirField> {
        if face != 1 && face != 2 {
            return Err(refusal(
                RefusalKind::ClaimRefuted,
                "dir_field_face_out_of_range",
                format!("a contact direction field parent face is 1 or 2, received {face}"),
            ));
        }
        if sigma != -1 && sigma != 1 {
            return Err(refusal(
                RefusalKind::ClaimRefuted,
                "dir_field_sigma_out_of_range",
                format!("a contact direction field side sign is ±1, received {sigma}"),
            ));
        }
        if !d.iter().all(|c| c.is_finite()) {
            return Err(refusal(
                RefusalKind::NonFinite,
                "dir_field_direction_not_finite",
                format!("the contact direction {d:?} is not finite"),
            ));
        }
        let norm_sq = d[0] * d[0] + d[1] * d[1] + d[2] * d[2];
        // The unit check is done on the squared norm with the engine's frame
        // slack (no sqrt: the norm of a certified unit direction is 1, so
        // |norm − 1| ≤ TOL_JACOBIAN is equivalent to |norm² − 1| ≤
        // TOL_JACOBIAN·(2 + TOL_JACOBIAN) up to the rounding of d itself).
        let slack = TOL_JACOBIAN * (2.0 + TOL_JACOBIAN);
        if (norm_sq - 1.0).abs() > slack {
            return Err(refusal(
                RefusalKind::ClaimRefuted,
                "dir_field_direction_not_unit",
                format!("the contact direction {d:?} has squared norm {norm_sq}, not unit"),
            ));
        }
        Ok(DirField { face, sigma, d })
    }
}

/// A fillet/canal (spec §12.2/§16 VERBATIM): the R7 spine arc id, the constant
/// radius, the side pair `(σ₁, σ₂)`, and the two certified contact direction
/// fields.
///
/// The normal-plane invariant `dᵢ(τ)·c′(τ) = 0` (Prop 12.3) is a THEOREM of
/// the R7 residual, NOT an obligation: this type deliberately carries NO
/// orthogonality certificate field (the named audit pins that). Δ_off is not a
/// field either: it is a named diagnostic ([`DeltaOff`]) computed on fixtures.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Canal {
    /// The R7 spine arc id.
    pub spine: ArcId,
    /// The constant rolling-ball radius `r`.
    pub r: f64,
    /// The side pair `(σ₁, σ₂)`.
    pub sigma: (i8, i8),
    /// The two certified contact direction fields, in parent-face order.
    pub contact: (DirField, DirField),
}

impl Canal {
    /// Build a canal representation, refusing a non-finite or non-positive
    /// radius, a side pair whose members are outside `{−1, +1}`, or a contact
    /// pair whose parent faces / side signs disagree with the side pair.
    #[allow(clippy::result_large_err)] // Refusal carries Option<PartialGraph> by frozen §2 shape (BG-KV2-000).
    pub fn try_new(
        spine: ArcId,
        r: f64,
        sigma: (i8, i8),
        contact: (DirField, DirField),
    ) -> Construction<Canal> {
        if !r.is_finite() {
            return Err(refusal(
                RefusalKind::NonFinite,
                "canal_radius_not_finite",
                format!("canal radius {r} is not finite"),
            ));
        }
        if r <= 0.0 {
            return Err(refusal(
                RefusalKind::ClaimRefuted,
                "canal_radius_not_positive",
                format!("canal radius {r} must be > 0"),
            ));
        }
        if sigma.0 != -1 && sigma.0 != 1 {
            return Err(refusal(
                RefusalKind::ClaimRefuted,
                "canal_sigma1_out_of_range",
                format!("canal side σ₁ is ±1, received {}", sigma.0),
            ));
        }
        if sigma.1 != -1 && sigma.1 != 1 {
            return Err(refusal(
                RefusalKind::ClaimRefuted,
                "canal_sigma2_out_of_range",
                format!("canal side σ₂ is ±1, received {}", sigma.1),
            ));
        }
        if contact.0.face != 1 || contact.1.face != 2 {
            return Err(refusal(
                RefusalKind::ClaimRefuted,
                "canal_contact_face_order",
                "the canal contact fields must be in parent-face order (S₁, S₂)".to_string(),
            ));
        }
        if contact.0.sigma != sigma.0 || contact.1.sigma != sigma.1 {
            return Err(refusal(
                RefusalKind::ClaimRefuted,
                "canal_contact_sigma_mismatch",
                "the contact direction fields' side signs must match the canal side pair"
                    .to_string(),
            ));
        }
        Ok(Canal {
            spine,
            r,
            sigma,
            contact,
        })
    }
}

/// The Δ_off offset-regularity diagnostic (spec §8.7): `Δ_off = (EG − F²) −
/// σr(EN − 2FM + GL) + (σr)²(LN − M²)`, computed as a check on fixtures. Its
/// content is subsumed by the R7 regularity certificate (Theorem 12.2); it is
/// never a separate precondition.
#[derive(Debug, Clone)]
pub struct DeltaOff {
    /// The parameter box the diagnostic ran over.
    pub box_: IBox2,
    /// `σr` (the signed radius).
    pub sigma_r: f64,
    /// The `EG − F²` enclosure over the box.
    pub egf2: (f64, f64),
    /// The `Δ_off` enclosure over the box.
    pub delta: (f64, f64),
    /// Whether `0 ∉ Δ_off` (the offset is regular / immersed).
    pub excludes_zero: bool,
}

/// Compute the Δ_off offset-regularity diagnostic of a single rational-carrier
/// leaf over a box (spec §8.7) as an interval-style check on the fixture:
/// `EG − F²` from the first fundamental form and the `EN − 2FM + GL` and
/// `LN − M²` terms from the second fundamental form, with `σr` the signed
/// radius of the offset.
///
/// The second-fundamental-form terms are projected onto the certified unit
/// normal direction of the leaf's [`Cone`](crate::kernel::patch::Cone) axis
/// over the box. The diagnostic is a fixture check (never a precondition): on
/// the planar carriers it is exercised over, the second-derivative enclosures
/// vanish identically, the curvature terms are exactly zero, and the
/// diagnostic reduces to `EG − F²` — independent of the projection direction.
#[allow(clippy::result_large_err)] // Refusal carries Option<PartialGraph> by frozen §2 shape (BG-KV2-000).
pub fn delta_off(leaf: &BezierLeaf, sigma_r: f64, d: IBox2) -> Construction<DeltaOff> {
    if !sigma_r.is_finite() {
        return Err(refusal(
            RefusalKind::NonFinite,
            "delta_off_sigma_r_not_finite",
            format!("delta_off signed radius {sigma_r} is not finite"),
        ));
    }
    use crate::kernel::patch::CertifiedPatch;
    use crate::kernel::patch::CertifiedPatchC2;
    let derivs = leaf.derivs(d);
    let su = match box3_iv(derivs.su) {
        Some(v) => v,
        None => return Err(offset_refusal("the leaf derivs enclosure is not finite")),
    };
    let sv = match box3_iv(derivs.sv) {
        Some(v) => v,
        None => return Err(offset_refusal("the leaf derivs enclosure is not finite")),
    };
    let e = dot3(&su, &su);
    let f = dot3(&su, &sv);
    let g = dot3(&sv, &sv);
    let egf2 = e.mul(&g).sub(&f.mul(&f));
    let sec = leaf.second_derivs(d);
    let suu = match box3_iv(sec.suu) {
        Some(v) => v,
        None => {
            return Err(offset_refusal(
                "the leaf second-derivs enclosure is not finite",
            ))
        }
    };
    let suv = match box3_iv(sec.suv) {
        Some(v) => v,
        None => {
            return Err(offset_refusal(
                "the leaf second-derivs enclosure is not finite",
            ))
        }
    };
    let svv = match box3_iv(sec.svv) {
        Some(v) => v,
        None => {
            return Err(offset_refusal(
                "the leaf second-derivs enclosure is not finite",
            ))
        }
    };
    // The unit normal direction of the leaf over the box: the certified axis of
    // its normal cone (sign-free). The second fundamental form coefficients
    // with respect to that direction are the certified projections of the
    // second-derivative enclosures.
    let n0 = leaf.normal_cone(d).axis;
    let n = [
        Interval::point(n0[0]),
        Interval::point(n0[1]),
        Interval::point(n0[2]),
    ];
    let l = dot3(&suu, &n);
    let m = dot3(&suv, &n);
    let nn = dot3(&svv, &n);
    let s = Interval::point(sigma_r);
    let term1 = e
        .mul(&nn)
        .sub(&Interval::point(2.0).mul(&f).mul(&m))
        .add(&g.mul(&l));
    let term2 = l.mul(&nn).sub(&m.mul(&m));
    let delta = egf2.sub(&s.mul(&term1)).add(&s.mul(&s).mul(&term2));
    if !egf2.is_finite() || !delta.is_finite() {
        return Err(offset_refusal("the delta_off enclosure is not finite"));
    }
    let excludes_zero = !(delta.lo <= 0.0 && delta.hi >= 0.0);
    Ok(DeltaOff {
        box_: d,
        sigma_r,
        egf2: (egf2.lo, egf2.hi),
        delta: (delta.lo, delta.hi),
        excludes_zero,
    })
}

/// A refusal for an unavailable offset diagnostic.
fn offset_refusal(detail: &str) -> Refusal {
    refusal(
        RefusalKind::OffsetDegenerate,
        "delta_off_enclosure_unavailable",
        detail.to_string(),
    )
}

// ---------------------------------------------------------------------------
// The §12.3 three-face corner
// ---------------------------------------------------------------------------

/// The outcome of a certified three-face corner solve: the corner center `c`
/// and the certified R8 point certificate of the compositional solve
/// `c₁₂(τ) = O₃(u,v)`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CornerPoint {
    /// The corner center `c = c₁₂(τ*) = O₃(u*, v*)`.
    pub center: [f64; 3],
    /// The R8 square C1 certificate of the corner root (residual `R8`).
    pub cert: crate::kernel::certs::PointCert3,
}

/// Solve the §12.3 three-face corner COMPOSITIONALLY over the S1A seam: the
/// R8 curve–surface system `c₁₂(τ) = O₃(u,v)` between the two-face canal spine
/// (as a 1-var curve leaf [`BezierLeaf1`]) and the third face's offset surface
/// (as a rational surface leaf [`BezierLeaf`]). Square, C1 — via the landed
/// `krawczyk_c1_n3` entry.
///
/// A non-Proven outcome (the spine does not reach the third face's offset in
/// the box, or the intersection is not transverse there) refuses
/// `CornerUnsolved` (Inconclusive) — the caller never invents a blend network.
#[allow(clippy::result_large_err)] // Refusal carries Option<PartialGraph> by frozen §2 shape (BG-KV2-000).
pub fn corner_compositional(
    spine: &BezierLeaf1,
    o3: &BezierLeaf,
    root_box: crate::kernel::patch::IBox3,
    w: &[CertifiedPositive],
) -> Construction<CornerPoint> {
    use crate::kernel::certs::PointCert3;
    use crate::kernel::engine::krawczyk_c1_n3;
    let system = R8System::try_new(spine, o3)?;
    if w.is_empty() {
        return Err(refusal(
            RefusalKind::WeightDegenerate,
            "corner_weights_empty",
            "corner_compositional requires at least one certified positive weight bound (§7.1 value argument)"
                .to_string(),
        ));
    }
    match krawczyk_c1_n3(&system, root_box, w) {
        ClaimVerdict::Proven(cert) => {
            let cert = PointCert3::try_new(ResidualId::R8, cert.box_, cert.rho)?;
            // The certified corner center is the midpoint of the certified box
            // in the chart `(τ, u, v)`; its model center is the spine's value.
            let tau = 0.5 * (cert.box_.lo[0] + cert.box_.hi[0]);
            let u = 0.5 * (cert.box_.lo[1] + cert.box_.hi[1]);
            let v = 0.5 * (cert.box_.lo[2] + cert.box_.hi[2]);
            let center = spine_point(spine, tau, u, v);
            Ok(CornerPoint { center, cert })
        }
        ClaimVerdict::Disproven(_) | ClaimVerdict::Inconclusive(_) => {
            Err(corner_unsolved_refusal(root_box))
        }
    }
}

/// The typed §12.3 refusal of an unsolved corner: `CornerUnsolved`
/// (Inconclusive) with named evidence. The direct fallback (9 unknowns, 9
/// equations, C1 at n = 9) is refused through this same refusal unless the
/// additive pattern extends cheaply; a caller must never invent a blend
/// network.
pub fn corner_unsolved_refusal(box_: crate::kernel::patch::IBox3) -> Refusal {
    Refusal::new(
        RefusalKind::CornerUnsolved,
        RefusalEvidence::Residual {
            residual: ResidualId::R8,
            box_: IBox2 {
                lo: [box_.lo[0], box_.lo[1]],
                hi: [box_.hi[0], box_.hi[1]],
            },
            note: "the compositional R8 corner solve did not certify over the box; the corner is unsolved (do not invent a blend network)",
        },
    )
}

/// Evaluate a 1-var curve leaf's model point (the spine value) at the chart
/// point `(τ, u, v)`: the spine's value at `τ` is the model center, and the
/// corner's `(u, v)` are the offset-surface parameters (reported for the
/// record). The model point is the homogeneous evaluation of the curve leaf at
/// `τ`.
fn spine_point(spine: &BezierLeaf1, tau: f64, _u: f64, _v: f64) -> [f64; 3] {
    // De Casteljau on the homogeneous control polygon; divide by the weight
    // once (plain float: the certificate already guarantees the root).
    let mut level: Vec<[f64; 4]> = spine.control.clone();
    while level.len() > 1 {
        let mut next = Vec::with_capacity(level.len() - 1);
        for w in level.windows(2) {
            let mut p = [0.0f64; 4];
            for k in 0..4 {
                p[k] = w[0][k] + tau * (w[1][k] - w[0][k]);
            }
            next.push(p);
        }
        level = next;
    }
    let p = level[0];
    let w = p[3];
    [p[0] / w, p[1] / w, p[2] / w]
}
