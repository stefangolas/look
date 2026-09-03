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

//! The kernel-v2 certificate calculus engine (BG-KV2-201-S2A): Lemma 8.0's
//! contraction-rate extraction, the §8.2 square C1 entry, the §8.3
//! one-dimensional tube (Theorem 8.1) over the recorded F3 amendment, and the
//! §8.1 frame construction — all over the landed interval core
//! (`formal::exact::CertifiedInterval`) and the stored `SquareSystem3` grids.
//!
//! **H-1.** The crate-level `#![deny(clippy::unwrap_used)]` in `lib.rs` covers
//! this module. The module carries no `unwrap`, no `expect`, and no `panic!`
//! calls, and adds no module-level `allow`. Where a `Result` must carry the
//! frozen `Refusal` (which holds `Option<PartialGraph>`), the large-`Err` lint
//! is allowed item-level only, exactly as the shim files do.
//!
//! **N4 / bit-reproducibility.** This module performs no transcendental call:
//! no `sin`, `cos`, `atan2`, `exp`, `ln`, `log`, or `powf` appears anywhere.
//! The only `sqrt` is the IEEE square root used to normalize the kernel
//! direction in [`build_frame4`]. Frame bases are built by deterministic
//! Gram–Schmidt in fixed index order — no SVD — because SVD is not
//! cross-platform bit-reproducible (the N4 record this module exists to
//! honor). Point evals, intervals, and the preconditioner arithmetic are
//! deterministic `f64` / `CertifiedInterval` sequences.
//!
//! **N5 / N6.** No division by a weight enclosure anywhere: the stored
//! `SquareSystem3` grids are the D-homogeneous cross-multiplied difference
//! `F_k = W2·P1_k − W1·P2_k`, and the tube certifies on that homogeneous
//! residual directly. The positive weight bounds arrive as the §7.1 VALUE
//! argument `w`; the engine only checks non-emptiness (an empty slice is
//! `WeightDegenerate`, Disproven) and carries the values into the emitted
//! certificate. It never re-derives a weight bound (rule 5).
//!
//! **Frozen seam.** [`SquareResidualEval`] and [`krawczyk_c1`] are the frozen
//! S1a seam verbatim (BG-KV2-202-S1A consumes them): `arity` is the number of
//! variables == number of equations (2 or 3), `eval` is the outward-rounded
//! interval residual over the box, `jac_encl` the row-major interval Jacobian
//! enclosure. Do not rename these shapes.
//!
//! **§2 rule-2 backing (normative).** A `Proven` arm carries the certificate;
//! a `Disproven` arm carries a `Refusal` (the residual's claim is refuted);
//! an `Inconclusive` arm carries a static [`Reason`]. In the square C1 the
//! Krawczyk image `K(B)` is classified exactly: strictly inside `B`
//! componentwise → candidate Proven; disjoint from `B` → Disproven (no root
//! in `B`); overlapping but not strictly inside → Inconclusive. The tube path
//! (Theorem 8.1) refuses non-inclusion as Inconclusive always — failure of the
//! perpendicular image to fit is never evidence of no branch (shrink-and-retry
//! is licensed), so no Disproven arm exists there.
//!
//! **Seam judgement (recorded): the C1 box carrier.** The frozen
//! `PointCert.box_` is an `IBox2`, so the C1 entry's box is the parameter box
//! of that same certificate: [`krawczyk_c1`] lands on `IBox2` (a square 2x2
//! residual — the R9 class, and the general square-plane C1). A 3x3 residual
//! (the R8 class) has no typed Proven carrier yet: the frozen `PointCert`
//! cannot record a 3D box. The Krawczyk algebra is written generically over
//! `arity` in the seam trait so the R8 wave can extend the entry when the
//! certificate shape grows; the engine never claims a certificate it cannot
//! represent.
//!
//! **Seam judgement (recorded): residual identity.** The frozen `krawczyk_c1`
//! carries no `ResidualId`, but the emitted [`crate::kernel::certs::PointCert`]
//! must name one. The engine therefore stamps the emitted certificate with
//! [`ResidualId::R1`] (the ordinary-trace residual, and the family this
//! packet's own certificate work targets). A caller certifying a different §7
//! residual must rebuild the `PointCert` with its own id through
//! `PointCert::try_new` — a documented one-line seam for the R8/R9 wave.
//!
//! **F3 amendment (additive).** The landed "square 3x3 slice, tau frozen to a
//! point" rule is untouched. [`c2_certify_tube4`] evaluates the 3x3
//! perpendicular system over the JOINT box `(I_tau, B_perp)` in frame
//! coordinates: the only extension is that the enclosure argument spans
//! `I_tau` jointly instead of a frozen slice point. Every landed ssi/trace
//! test stays green (V5 identity).
//!
//! **`ArcCert` box convention.** [`crate::kernel::certs::ArcCert<4>`] stores
//! `b_perp: IBox<4>`. The shim's `Frame` keeps the perpendicular basis in
//! `q_perp[0..N-2]` and re-stores `q_tau` as its final column, so the box the
//! tube ran over is recorded in that same `q_perp`-aligned frame-coordinate
//! order: axes `0..=2` are the perpendicular coordinates `y` and axis `3` is
//! the tangent coordinate `tau`. [`c2_certify_tube4`] lifts its `IBox<3>`
//! argument by appending `i_tau` as the final axis; `ArcCert.i_tau` carries
//! the same interval verbatim.

use crate::kernel::certs::{ArcCert, Frame, PointCert};
use crate::kernel::config::{KAPPA_MAX, RHO_MAX, TOL_JACOBIAN};
use crate::kernel::evidence::{ClaimVerdict, Construction, Refusal, RefusalEvidence, RefusalKind};
use crate::kernel::patch::{CertifiedPositive, IBox, IBox2, Reason};
use crate::kernel::residual::ResidualId;
use crate::kernel::Interval;
use crate::SquareSystem3;

/// The frozen S1a seam: an `n`-variable / `n`-equation square residual over an
/// interval box (`n` is 2 or 3).
pub trait SquareResidualEval {
    /// Number of variables == number of equations (2 or 3).
    fn arity(&self) -> usize;
    /// Outward-rounded interval residual over the box (component `i` evaluated
    /// over ALL variables' intervals jointly).
    fn eval(&self, b: &[Interval]) -> Vec<Interval>;
    /// Outward-rounded interval Jacobian enclosure, row-major `n x n`.
    fn jac_encl(&self, b: &[Interval]) -> Vec<Vec<Interval>>;
}

/// A named predicate refusal for an engine invariant.
fn engine_refusal(kind: RefusalKind, name: &'static str, detail: String) -> Refusal {
    Refusal::new(kind, RefusalEvidence::Predicate { name, detail })
}

// ---------------------------------------------------------------------------
// The n=2 Krawczyk arm (§8.2, the square-plane C1)
// ---------------------------------------------------------------------------

type M2 = [[Interval; 2]; 2];

/// The float midpoint centre of an `IBox<2>`.
fn centre2(b: &IBox2) -> [f64; 2] {
    [(b.lo[0] + b.hi[0]) / 2.0, (b.lo[1] + b.hi[1]) / 2.0]
}

/// The interval radius vector of an `IBox<2>`, `None` on a non-positive or
/// non-finite radius.
fn radii2(b: &IBox2) -> Option<[f64; 2]> {
    let r = [(b.hi[0] - b.lo[0]) / 2.0, (b.hi[1] - b.lo[1]) / 2.0];
    if r.iter().all(|c| c.is_finite() && *c > 0.0) {
        Some(r)
    } else {
        None
    }
}

/// Determinant of a 2x2 interval matrix under directed rounding.
fn det2_iv(m: &M2) -> Interval {
    m[0][0].mul(&m[1][1]).sub(&m[0][1].mul(&m[1][0]))
}

/// The interval inverse of a 2x2 matrix via adjugate over determinant.
/// `None` when the determinant enclosure contains (or is) zero or the quotient
/// is not finite.
fn inv2_iv(m: &M2) -> Option<M2> {
    let det = det2_iv(m);
    if !det.is_finite() || (det.lo <= 0.0 && det.hi >= 0.0) {
        return None;
    }
    let adj: M2 = [[m[1][1], m[0][1].neg()], [m[1][0].neg(), m[0][0]]];
    let mut out = [[Interval::point(0.0); 2]; 2];
    for r in 0..2 {
        for c in 0..2 {
            out[r][c] = adj[r][c].div(&det)?;
        }
    }
    Some(out)
}

/// Interval 2x2 matrix product.
fn matmul2(a: &M2, b: &M2) -> M2 {
    let mut out = [[Interval::point(0.0); 2]; 2];
    for r in 0..2 {
        for c in 0..2 {
            let mut acc = Interval::point(0.0);
            for k in 0..2 {
                acc = acc.add(&a[r][k].mul(&b[k][c]));
            }
            out[r][c] = acc;
        }
    }
    out
}

/// Interval 2x2 matrix times 2-vector.
fn matvec2(m: &M2, v: &[Interval; 2]) -> [Interval; 2] {
    [
        m[0][0].mul(&v[0]).add(&m[0][1].mul(&v[1])),
        m[1][0].mul(&v[0]).add(&m[1][1].mul(&v[1])),
    ]
}

/// The outward-rounded box `B − z_hat` (centred box), replicating the landed
/// trace reduction's op order.
fn centred_dx2(b: &IBox2, z: &[Interval; 2]) -> [Interval; 2] {
    let mut dx = [Interval::point(0.0); 2];
    for k in 0..2 {
        let d_lo = Interval::point(b.lo[k]).sub(&z[k]);
        let d_hi = Interval::point(b.hi[k]).sub(&z[k]);
        dx[k] = Interval {
            lo: d_lo.lo.min(d_hi.lo),
            hi: d_lo.hi.max(d_hi.hi),
        };
    }
    dx
}

/// The componentwise magnitude `mag(v) = max(|lo|, |hi|)` of an interval.
fn mag(v: &Interval) -> f64 {
    v.lo.abs().max(v.hi.abs())
}

/// Lemma 8.0's contraction rate `max_i (M r)_i / r_i`. `None` when a quotient
/// is not finite.
fn rho2(id_minus: &M2, r: [f64; 2]) -> Option<f64> {
    let mut rho = 0.0f64;
    for i in 0..2 {
        let mr = mag(&id_minus[i][0]) * r[0] + mag(&id_minus[i][1]) * r[1];
        let ratio = mr / r[i];
        if !ratio.is_finite() {
            return None;
        }
        rho = rho.max(ratio);
    }
    Some(rho)
}

/// The three-valued inclusion classification of a Krawczyk image axis.
enum Inclusion {
    /// `K(B)` is component-wise strictly inside `B`.
    Strict,
    /// `K(B)` is disjoint from `B` (no common point).
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

/// The §8.2 C1 entry: Lemma 8.0 + §8.2 verbatim over the frozen seam, on the
/// 2D parameter box that [`PointCert`] carries.
///
/// `w` is a §7.1 VALUE argument (never re-derived): an empty slice refuses
/// `WeightDegenerate` (Disproven). `rho` is Lemma 8.0's contraction rate,
/// `rho = max_i (M r)_i / r_i` with `M = mag(I − A·□DR(B))` and `r = rad(B)`,
/// refusing any zero or non-finite radius as `NonFinite` (Disproven).
pub fn krawczyk_c1(
    g: &dyn SquareResidualEval,
    b: IBox2,
    w: &[CertifiedPositive],
) -> ClaimVerdict<PointCert, Refusal, Reason> {
    if g.arity() != 2 {
        return ClaimVerdict::Inconclusive("c1_arity_mismatch_box_dimension");
    }
    if w.is_empty() {
        return ClaimVerdict::Disproven(engine_refusal(
            RefusalKind::WeightDegenerate,
            "c1_weights_empty",
            "krawczyk_c1 requires at least one certified positive weight bound (§7.1 value argument)"
                .to_string(),
        ));
    }
    let r = match radii2(&b) {
        Some(r) => r,
        None => {
            return ClaimVerdict::Disproven(engine_refusal(
                RefusalKind::NonFinite,
                "c1_radius_nonpositive",
                "krawczyk_c1 requires a strictly positive finite radius on every box axis"
                    .to_string(),
            ))
        }
    };
    let z = centre2(&b);
    let ziv: [Interval; 2] = [Interval::point(z[0]), Interval::point(z[1])];
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

    let r0 = g.eval(&ziv);
    if r0.len() != 2 {
        return ClaimVerdict::Inconclusive("c1_eval_arity_mismatch");
    }
    let j0_rows = g.jac_encl(&ziv);
    let jb_rows = g.jac_encl(&box_iv);
    if j0_rows.len() != 2
        || j0_rows.iter().any(|row| row.len() != 2)
        || jb_rows.len() != 2
        || jb_rows.iter().any(|row| row.len() != 2)
    {
        return ClaimVerdict::Inconclusive("c1_jac_arity_mismatch");
    }

    let j0: M2 = [
        [j0_rows[0][0], j0_rows[0][1]],
        [j0_rows[1][0], j0_rows[1][1]],
    ];
    let jb: M2 = [
        [jb_rows[0][0], jb_rows[0][1]],
        [jb_rows[1][0], jb_rows[1][1]],
    ];

    // A = the interval inverse of the midpoint (centre) Jacobian.
    let a = match inv2_iv(&j0) {
        Some(a) => a,
        None => return ClaimVerdict::Inconclusive("c1_midpoint_jacobian_singular"),
    };

    // (I − A·□DR(B)) and the Krawczyk image K(B).
    let cj = matmul2(&a, &jb);
    let id_minus: M2 = [
        [Interval::point(1.0).sub(&cj[0][0]), cj[0][1].neg()],
        [cj[1][0].neg(), Interval::point(1.0).sub(&cj[1][1])],
    ];
    if id_minus.iter().flatten().any(|v| !v.is_finite()) {
        return ClaimVerdict::Inconclusive("c1_enclosure_not_finite");
    }
    let dx = centred_dx2(&b, &ziv);
    let r0v: [Interval; 2] = [r0[0], r0[1]];
    let ch = matvec2(&a, &r0v);
    let md = matvec2(&id_minus, &dx);
    let k: [Interval; 2] = [
        ziv[0].sub(&ch[0]).add(&md[0]),
        ziv[1].sub(&ch[1]).add(&md[1]),
    ];
    if k.iter().any(|v| !v.is_finite()) {
        return ClaimVerdict::Inconclusive("c1_enclosure_not_finite");
    }

    // Classification (rule 2).
    let mut strict = true;
    let mut disjoint = false;
    for ((lo_i, hi_i), k_i) in b.lo.iter().zip(b.hi.iter()).zip(k.iter()) {
        match classify_axis(*lo_i, *hi_i, k_i.lo, k_i.hi) {
            Inclusion::Strict => {}
            Inclusion::Disjoint => {
                disjoint = true;
                strict = false;
            }
            Inclusion::Overlap => strict = false,
        }
    }
    if !strict {
        if disjoint {
            return ClaimVerdict::Disproven(engine_refusal(
                RefusalKind::ClaimRefuted,
                "c1_k_disjoint_no_root_in_box",
                "the Krawczyk image is disjoint from the box: no root of the residual in the box"
                    .to_string(),
            ));
        }
        return ClaimVerdict::Inconclusive("c1_inclusion_not_strict");
    }

    // Lemma 8.0's contraction rate.
    let rho = match rho2(&id_minus, r) {
        Some(rho) => rho,
        None => return ClaimVerdict::Inconclusive("c1_rho_not_finite"),
    };
    if rho > RHO_MAX {
        return ClaimVerdict::Inconclusive("c1_rho_exceeds_rho_max");
    }
    // See the module-doc seam judgement: the engine stamps R1.
    match PointCert::try_new(ResidualId::R1, b, rho) {
        Ok(cert) => ClaimVerdict::Proven(cert),
        Err(refusal) => ClaimVerdict::Disproven(refusal),
    }
}

// ---------------------------------------------------------------------------
// SquareSystem3 tensor evaluation (engine-local, over the landed interval core)
// ---------------------------------------------------------------------------

/// Why a hull/derivative enclosure could not be produced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HullErr {
    /// The box is not a compact subset of the chart rectangle.
    DomainNotCompact,
    /// The enclosure work did not produce a finite interval.
    Unavailable,
}

/// A four-axis tensor-Bernstein grid in the `SquareSystem3` flat layout.
struct Grid4 {
    /// Degrees `(m1, n1, m2, n2)`.
    degrees: (usize, usize, usize, usize),
    /// Flat coefficient rows, each of length `(m2+1)·(n2+1)`.
    rows: Vec<Vec<f64>>,
}

impl Grid4 {
    fn row_spacing(&self) -> usize {
        self.degrees.1 + 1
    }

    fn col_spacing(&self) -> usize {
        self.degrees.3 + 1
    }

    fn len_axis(&self, axis: usize) -> usize {
        let (m1, n1, m2, n2) = self.degrees;
        match axis {
            0 => m1 + 1,
            1 => n1 + 1,
            2 => m2 + 1,
            _ => n2 + 1,
        }
    }

    /// The first-partial coefficient grid along a chart axis (Bernstein
    /// derivative: `d·(c[k+1] − c[k])` of degree `d − 1`).
    fn partial_axis(&self, axis: usize) -> Result<Grid4, HullErr> {
        let (m1, n1, m2, n2) = self.degrees;
        let base = [m1, n1, m2, n2][axis];
        if base == 0 {
            return Err(HullErr::Unavailable);
        }
        let scale = base as f64;
        let degrees = match axis {
            0 => (m1 - 1, n1, m2, n2),
            1 => (m1, n1 - 1, m2, n2),
            2 => (m1, n1, m2 - 1, n2),
            _ => (m1, n1, m2, n2 - 1),
        };
        let (nm1, nn1, nm2, nn2) = degrees;
        let rows = (nm1 + 1) * (nn1 + 1);
        let cols = (nm2 + 1) * (nn2 + 1);
        let mut out = vec![vec![0.0f64; cols]; rows];
        let sp1 = self.row_spacing();
        let sp2 = self.col_spacing();
        for a in 0..=nm1 {
            for b in 0..=nn1 {
                for i in 0..=nm2 {
                    for j in 0..=nn2 {
                        let (a0, b0, i0, j0, a1, b1, i1, j1) = match axis {
                            0 => (a, b, i, j, a + 1, b, i, j),
                            1 => (a, b, i, j, a, b + 1, i, j),
                            2 => (a, b, i, j, a, b, i + 1, j),
                            _ => (a, b, i, j, a, b, i, j + 1),
                        };
                        let lo = self.rows[a0 * sp1 + b0][i0 * sp2 + j0];
                        let hi = self.rows[a1 * sp1 + b1][i1 * sp2 + j1];
                        let dst_row = a * (nn1 + 1) + b;
                        let dst_col = i * (nn2 + 1) + j;
                        out[dst_row][dst_col] = scale * (hi - lo);
                    }
                }
            }
        }
        Ok(Grid4 { degrees, rows: out })
    }
}

/// Interval de Casteljau over one axis for a 1-D interval coefficient list.
fn one_d_interval(pts: &[Interval], u: &Interval) -> Result<Interval, HullErr> {
    if pts.is_empty() {
        return Err(HullErr::Unavailable);
    }
    let mut level = pts.to_vec();
    while level.len() > 1 {
        let mut next = Vec::with_capacity(level.len() - 1);
        for pair in level.windows(2) {
            next.push(pair[0].add(&pair[1].sub(&pair[0]).mul(u)));
        }
        level = next;
    }
    if level[0].is_finite() {
        Ok(level[0])
    } else {
        Err(HullErr::Unavailable)
    }
}

/// Certified range enclosure of a four-axis tensor polynomial over the box
/// whose axis intervals are unit-chart `[0,1]` subintervals.
fn hull_grid4(t: &Grid4, box_axis: [(f64, f64); 4]) -> Result<Interval, HullErr> {
    for (lo, hi) in box_axis {
        if !lo.is_finite() || !hi.is_finite() || !(lo >= 0.0 && hi <= 1.0 && lo <= hi) {
            return Err(HullErr::DomainNotCompact);
        }
    }
    if t.rows.is_empty() || t.rows[0].is_empty() {
        return Err(HullErr::Unavailable);
    }
    let sp1 = t.row_spacing();
    let n1p1 = t.len_axis(1);
    let cols = t.rows[0].len();
    let u_iv = Interval {
        lo: box_axis[0].0,
        hi: box_axis[0].1,
    };
    let u_len = t.len_axis(0);
    let mut u_cols = vec![Vec::<Interval>::with_capacity(n1p1); cols];
    for b in 0..n1p1 {
        for (c, slot) in u_cols.iter_mut().enumerate() {
            let mut pts = Vec::with_capacity(u_len);
            for a in 0..u_len {
                pts.push(Interval::point(t.rows[a * sp1 + b][c]));
            }
            slot.push(one_d_interval(&pts, &u_iv)?);
        }
    }
    let v_iv = Interval {
        lo: box_axis[1].0,
        hi: box_axis[1].1,
    };
    let mut v_collapsed = Vec::with_capacity(cols);
    for col in u_cols {
        v_collapsed.push(one_d_interval(&col, &v_iv)?);
    }
    let sp2 = t.col_spacing();
    let mut grid2: Vec<Vec<Interval>> = Vec::with_capacity(v_collapsed.len() / sp2);
    for row_slice in v_collapsed.chunks(sp2) {
        grid2.push(row_slice.to_vec());
    }
    hull_2d_interval(&grid2, box_axis[2], box_axis[3])
}

/// Interval de Casteljau over the `(s, t)` box of an interval-valued bivariate
/// tensor grid.
fn hull_2d_interval(
    grid: &[Vec<Interval>],
    s: (f64, f64),
    t: (f64, f64),
) -> Result<Interval, HullErr> {
    if grid.is_empty() || grid[0].is_empty() {
        return Err(HullErr::Unavailable);
    }
    let width = grid[0].len();
    if grid.iter().any(|row| row.len() != width) {
        return Err(HullErr::Unavailable);
    }
    let s_iv = Interval { lo: s.0, hi: s.1 };
    let t_iv = Interval { lo: t.0, hi: t.1 };
    let mut col_evals = Vec::with_capacity(width);
    for j in 0..width {
        let col: Vec<Interval> = grid.iter().map(|row| row[j]).collect();
        col_evals.push(one_d_interval(&col, &s_iv)?);
    }
    let hull = one_d_interval(&col_evals, &t_iv)?;
    if hull.is_finite() {
        Ok(hull)
    } else {
        Err(HullErr::Unavailable)
    }
}

/// Map a chart-coordinate subinterval of one axis onto the unit chart
/// `[0, 1]`, outward rounded and clamped. `None` when the subinterval is not
/// a compact subset of the axis's chart rectangle.
fn to_unit_interval(lo: f64, hi: f64, d0: f64, d1: f64) -> Option<(f64, f64)> {
    if !lo.is_finite() || !hi.is_finite() || !d0.is_finite() || !d1.is_finite() {
        return None;
    }
    let (a, b) = if d0 <= d1 { (d0, d1) } else { (d1, d0) };
    if !(a <= lo && lo <= hi && hi <= b) {
        return None;
    }
    let width = Interval::point(d1).sub(&Interval::point(d0));
    if width.lo <= 0.0 {
        return None;
    }
    let lo_u = Interval::point(lo).sub(&Interval::point(d0));
    let hi_u = Interval::point(hi).sub(&Interval::point(d0));
    let lo_div = lo_u.div(&width)?;
    let hi_div = hi_u.div(&width)?;
    let u_lo = lo_div.lo.min(hi_div.lo).clamp(0.0, 1.0);
    let u_hi = lo_div.hi.max(hi_div.hi).clamp(0.0, 1.0);
    Some((u_lo, u_hi))
}

/// The chart rectangles of a stored system as per-axis `(lo, hi)` pairs.
fn chart_rects(system: &SquareSystem3) -> [(f64, f64); 4] {
    let maps = system.domain_maps();
    [
        (maps.0, maps.1),
        (maps.2, maps.3),
        (maps.4, maps.5),
        (maps.6, maps.7),
    ]
}

/// The unit-chart image of a chart-coordinate box, `None` when the box is not
/// a compact subset of the chart rectangle.
fn to_unit_box(system: &SquareSystem3, box_: [(f64, f64); 4]) -> Option<[(f64, f64); 4]> {
    let rects = chart_rects(system);
    let mut out = [(0.0f64, 0.0f64); 4];
    for a in 0..4 {
        out[a] = to_unit_interval(box_[a].0, box_[a].1, rects[a].0, rects[a].1)?;
    }
    Some(out)
}

/// A component grid of the stored system, wrapped.
fn system_grid(system: &SquareSystem3, component: usize) -> Grid4 {
    Grid4 {
        degrees: system.degrees(),
        rows: system.grids()[component].clone(),
    }
}

/// Certified value enclosure of one stored component over a chart box.
fn component_value(
    system: &SquareSystem3,
    component: usize,
    box_: [(f64, f64); 4],
) -> Result<Interval, HullErr> {
    let unit = to_unit_box(system, box_).ok_or(HullErr::DomainNotCompact)?;
    let grid = system_grid(system, component);
    hull_grid4(&grid, unit)
}

/// Certified chart-coordinate partial enclosure of one component along one
/// chart axis over a chart box (the unit-axis derivative scaled by the inverse
/// chart width).
fn component_partial(
    system: &SquareSystem3,
    component: usize,
    axis: usize,
    box_: [(f64, f64); 4],
) -> Result<Interval, HullErr> {
    if component > 2 || axis > 3 {
        return Err(HullErr::Unavailable);
    }
    let unit = to_unit_box(system, box_).ok_or(HullErr::DomainNotCompact)?;
    let grid = system_grid(system, component);
    let derived = grid.partial_axis(axis)?;
    let hull = hull_grid4(&derived, unit)?;
    let rect = chart_rects(system);
    let width = Interval::point(rect[axis].1).sub(&Interval::point(rect[axis].0));
    match hull.div(&width) {
        Some(out) if out.is_finite() => Ok(out),
        _ => Err(HullErr::Unavailable),
    }
}

/// A point as a degenerate chart box.
fn point_box(point: [f64; 4]) -> [(f64, f64); 4] {
    [
        (point[0], point[0]),
        (point[1], point[1]),
        (point[2], point[2]),
        (point[3], point[3]),
    ]
}

/// Certified float partials of the stored system at a chart point: the
/// midpoint of the certified partial enclosure over the degenerate point box.
fn certified_float_partials(system: &SquareSystem3, point: [f64; 4]) -> Option<[[f64; 4]; 3]> {
    let box_ = point_box(point);
    let mut out = [[0.0f64; 4]; 3];
    for (component, row) in out.iter_mut().enumerate() {
        for (axis, cell) in row.iter_mut().enumerate() {
            let enc = component_partial(system, component, axis, box_).ok()?;
            *cell = 0.5 * (enc.lo + enc.hi);
        }
    }
    Some(out)
}

// ---------------------------------------------------------------------------
// Float linear algebra for frame construction and the tube preconditioner
// ---------------------------------------------------------------------------

/// Determinant of a 3x3 float matrix (exact op order as the landed trace).
fn det3_f64(m: [[f64; 3]; 3]) -> f64 {
    m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
}

/// The 3x3 float inverse via adjugate over determinant. `None` on a zero or
/// non-finite determinant or a non-finite result.
fn inv3_f64(m: [[f64; 3]; 3]) -> Option<[[f64; 3]; 3]> {
    let det = det3_f64(m);
    if !det.is_finite() || det == 0.0 {
        return None;
    }
    let adj = [
        [
            m[1][1] * m[2][2] - m[1][2] * m[2][1],
            m[0][2] * m[2][1] - m[0][1] * m[2][2],
            m[0][1] * m[1][2] - m[0][2] * m[1][1],
        ],
        [
            m[1][2] * m[2][0] - m[1][0] * m[2][2],
            m[0][0] * m[2][2] - m[0][2] * m[2][0],
            m[0][2] * m[1][0] - m[0][0] * m[1][2],
        ],
        [
            m[1][0] * m[2][1] - m[1][1] * m[2][0],
            m[0][1] * m[2][0] - m[0][0] * m[2][1],
            m[0][0] * m[1][1] - m[0][1] * m[1][0],
        ],
    ];
    let mut out = [[0.0f64; 3]; 3];
    for r in 0..3 {
        for c in 0..3 {
            let v = adj[r][c] / det;
            if !v.is_finite() {
                return None;
            }
            out[r][c] = v;
        }
    }
    Some(out)
}

/// The `max` row-absolute-sum norm of a 3x3 float matrix.
fn norm_inf3(m: [[f64; 3]; 3]) -> f64 {
    let mut best = 0.0f64;
    for row in m {
        let s = row.iter().map(|c| c.abs()).sum::<f64>();
        best = best.max(s);
    }
    best
}

/// The maximal-minor (kernel-direction) vector of a 3x4 float matrix with
/// EXACTLY Theorem 6.4's sign pattern (as landed in `ssi_trace.rs`).
fn kernel_minors(rows: [[f64; 4]; 3]) -> [f64; 4] {
    let minor = |cols: [usize; 3]| -> f64 {
        let mut m = [[0.0f64; 3]; 3];
        for (r, row) in rows.iter().enumerate() {
            for (k, &c) in cols.iter().enumerate() {
                m[r][k] = row[c];
            }
        }
        det3_f64(m)
    };
    let d0 = minor([1, 2, 3]);
    let d1 = -minor([0, 2, 3]);
    let d2 = minor([0, 1, 3]);
    let d3 = -minor([0, 1, 2]);
    [d0, d1, d2, d3]
}

/// §8.1/§11 frame construction: `q_tau = m/||m||` (IEEE sqrt, deterministic),
/// the perpendicular basis by Gram–Schmidt in FIXED index order, and the
/// `a = [DF(ẑ) Q_⊥]⁻¹` preconditioner (embedded as the 4x4 `Frame` field with
/// the tangent axis carried identically).
///
/// Returns the frame and the float kernel direction `m`. If `||m||` is below
/// the normative floor (rank 2 territory) or the frame Jacobian block is
/// singular, refuses `Conditioning` (Inconclusive) — the caller subdivides or
/// switches coordinate; rank 2 is S0/S5a territory, not this packet's.
// Refusal carries Option<PartialGraph> by frozen §2 shape; large-Err is allowed (BG-KV2-000).
#[allow(clippy::result_large_err)]
pub fn build_frame4(system: &SquareSystem3, z_hat: [f64; 4]) -> Construction<(Frame<4>, [f64; 4])> {
    if !z_hat.iter().all(|c| c.is_finite()) {
        return Err(engine_refusal(
            RefusalKind::NonFinite,
            "frame_z_hat_not_finite",
            "build_frame4 requires a finite chart point".to_string(),
        ));
    }
    let partials = match certified_float_partials(system, z_hat) {
        Some(partials) => partials,
        None => {
            return Err(engine_refusal(
                RefusalKind::Conditioning,
                "frame_partials_unavailable",
                "the certified partials of the system could not be enclosed at z_hat".to_string(),
            ))
        }
    };
    let m = kernel_minors(partials);
    let norm_sq = m[0] * m[0] + m[1] * m[1] + m[2] * m[2] + m[3] * m[3];
    if !norm_sq.is_finite() || norm_sq <= TOL_JACOBIAN * TOL_JACOBIAN {
        return Err(engine_refusal(
            RefusalKind::Conditioning,
            "frame_kernel_direction_degenerate",
            "the maximal-minor kernel direction of DF at z_hat is degenerate (rank 2 / tangency)"
                .to_string(),
        ));
    }
    let norm = norm_sq.sqrt();
    let q_tau = [m[0] / norm, m[1] / norm, m[2] / norm, m[3] / norm];

    // Deterministic Gram-Schmidt over the fixed candidate order e_0..e_3.
    let mut perp: Vec<[f64; 4]> = Vec::with_capacity(3);
    let mut basis: Vec<[f64; 4]> = vec![q_tau];
    for k in 0..4 {
        if perp.len() == 3 {
            break;
        }
        let mut e = [0.0f64; 4];
        e[k] = 1.0;
        let mut v = e;
        for fixed in &basis {
            let dot = v[0] * fixed[0] + v[1] * fixed[1] + v[2] * fixed[2] + v[3] * fixed[3];
            for j in 0..4 {
                v[j] -= dot * fixed[j];
            }
        }
        let v_norm_sq = v[0] * v[0] + v[1] * v[1] + v[2] * v[2] + v[3] * v[3];
        if !v_norm_sq.is_finite() || v_norm_sq <= TOL_JACOBIAN * TOL_JACOBIAN {
            continue;
        }
        let v_norm = v_norm_sq.sqrt();
        let unit = [v[0] / v_norm, v[1] / v_norm, v[2] / v_norm, v[3] / v_norm];
        basis.push(unit);
        perp.push(unit);
    }
    if perp.len() != 3 {
        return Err(engine_refusal(
            RefusalKind::Conditioning,
            "frame_perp_basis_degenerate",
            "Gram-Schmidt could not build a 3-dimensional perpendicular basis".to_string(),
        ));
    }

    // The perpendicular Jacobian block B = DF(z_hat)·Q_⊥ and its inverse.
    let b: [[f64; 3]; 3] = {
        let mut out = [[0.0f64; 3]; 3];
        for r in 0..3 {
            for c in 0..3 {
                let p = perp[c];
                out[r][c] = partials[r][0] * p[0]
                    + partials[r][1] * p[1]
                    + partials[r][2] * p[2]
                    + partials[r][3] * p[3];
            }
        }
        out
    };
    let a33 = match inv3_f64(b) {
        Some(a) => a,
        None => {
            return Err(engine_refusal(
                RefusalKind::Conditioning,
                "frame_perp_jacobian_singular",
                "the perpendicular Jacobian block [DF(z_hat) Q_perp] is singular".to_string(),
            ))
        }
    };
    // The `Frame.a` field is N x N; embed the (N-1)x(N-1) preconditioner with
    // the tangent axis carried identically (row/column 3 = tau).
    let mut a = [[0.0f64; 4]; 4];
    for r in 0..3 {
        for c in 0..3 {
            a[r][c] = a33[r][c];
        }
    }
    a[3][3] = 1.0;

    let q: [[f64; 4]; 4] = [q_tau, perp[0], perp[1], perp[2]];
    let q_perp: [[f64; 4]; 4] = [perp[0], perp[1], perp[2], q_tau];
    let frame = Frame::try_new(z_hat, q, q_tau, q_perp, a)?;
    Ok((frame, m))
}

// ---------------------------------------------------------------------------
// The tube (§8.3 Theorem 8.1, additive F3 amendment)
// ---------------------------------------------------------------------------

type Iv3 = [Interval; 3];
type M3 = [[Interval; 3]; 3];

/// Interval 3x3 matrix product.
fn matmul3_iv(a: &M3, b: &M3) -> M3 {
    let mut out = [[Interval::point(0.0); 3]; 3];
    for r in 0..3 {
        for c in 0..3 {
            let mut acc = Interval::point(0.0);
            for k in 0..3 {
                acc = acc.add(&a[r][k].mul(&b[k][c]));
            }
            out[r][c] = acc;
        }
    }
    out
}

/// Interval 3x3 matrix times 3-vector.
fn matvec3_iv(m: &M3, v: &Iv3) -> Iv3 {
    [
        m[0][0]
            .mul(&v[0])
            .add(&m[0][1].mul(&v[1]))
            .add(&m[0][2].mul(&v[2])),
        m[1][0]
            .mul(&v[0])
            .add(&m[1][1].mul(&v[1]))
            .add(&m[1][2].mul(&v[2])),
        m[2][0]
            .mul(&v[0])
            .add(&m[2][1].mul(&v[1]))
            .add(&m[2][2].mul(&v[2])),
    ]
}

/// The chart-space point `z_hat + q_tau·tau + Q_perp·y`.
fn chart_point(frame: &Frame<4>, tau: f64, y: [f64; 3]) -> [f64; 4] {
    let mut out = [0.0f64; 4];
    for (j, out_j) in out.iter_mut().enumerate() {
        let mut v = frame.z_hat[j] + frame.q_tau[j] * tau;
        for (c, y_c) in y.iter().enumerate() {
            v += frame.q_perp[c][j] * y_c;
        }
        *out_j = v;
    }
    out
}

/// The float 3x3 perpendicular Jacobian block `DF·Q_perp` from float partials.
fn perp_jacobian(partials: &[[f64; 4]; 3], frame: &Frame<4>) -> [[f64; 3]; 3] {
    let mut out = [[0.0f64; 3]; 3];
    for (r, orow) in out.iter_mut().enumerate() {
        for (c, cell) in orow.iter_mut().enumerate() {
            let p = frame.q_perp[c];
            *cell = partials[r][0] * p[0]
                + partials[r][1] * p[1]
                + partials[r][2] * p[2]
                + partials[r][3] * p[3];
        }
    }
    out
}

/// The frame-transformed chart box of the tube. `axis_iv` holds the three
/// perpendicular coordinates (axes `0..=2` of the `q_perp`-aligned frame);
/// the tangent interval is `i_tau`. Returns the axis-aligned hull in chart
/// coordinates, `None` when the hull is not a compact subset of the chart
/// rectangle.
fn frame_tube_chart_box(
    system: &SquareSystem3,
    frame: &Frame<4>,
    i_tau: Interval,
    axis_iv: &Iv3,
) -> Option<[(f64, f64); 4]> {
    let mut acc: [Interval; 4] = [
        Interval::point(frame.z_hat[0]),
        Interval::point(frame.z_hat[1]),
        Interval::point(frame.z_hat[2]),
        Interval::point(frame.z_hat[3]),
    ];
    for (j, acc_j) in acc.iter_mut().enumerate() {
        let tau_term = Interval::point(frame.q_tau[j]).mul(&i_tau);
        *acc_j = acc_j.add(&tau_term);
        for (c, axis_c) in axis_iv.iter().enumerate() {
            let term = Interval::point(frame.q_perp[c][j]).mul(axis_c);
            *acc_j = acc_j.add(&term);
        }
    }
    let rects = chart_rects(system);
    let mut out = [(0.0f64, 0.0f64); 4];
    for (j, out_j) in out.iter_mut().enumerate() {
        if !acc[j].is_finite() {
            return None;
        }
        if acc[j].lo < rects[j].0 || acc[j].hi > rects[j].1 {
            return None;
        }
        *out_j = (acc[j].lo, acc[j].hi);
    }
    Some(out)
}

/// The certified value enclosure of the three residual components over a
/// chart box.
fn system_values(system: &SquareSystem3, box_: [(f64, f64); 4]) -> Result<[Interval; 3], HullErr> {
    let mut out = [Interval::point(0.0); 3];
    for (k, out_k) in out.iter_mut().enumerate() {
        *out_k = component_value(system, k, box_)?;
    }
    Ok(out)
}

/// The certified chart-coordinate partial matrix (3 components x 4 axes) over
/// a chart box.
fn system_jacobian(
    system: &SquareSystem3,
    box_: [(f64, f64); 4],
) -> Result<[[Interval; 4]; 3], HullErr> {
    let mut out = [[Interval::point(0.0); 4]; 3];
    for (r, orow) in out.iter_mut().enumerate() {
        for (c, cell) in orow.iter_mut().enumerate() {
            *cell = component_partial(system, r, c, box_)?;
        }
    }
    Ok(out)
}

/// The outward-rounded centred box `B − centre` of an interval box against an
/// interval centre.
fn centred_dx3_axis(b: &Iv3, centre: &Iv3) -> Iv3 {
    [
        b[0].sub(&centre[0]),
        b[1].sub(&centre[1]),
        b[2].sub(&centre[2]),
    ]
}

/// The §8.3 one-dimensional (tube) certificate over the R1 family at n = 4
/// (the recorded F3 amendment: the 3x3 perpendicular system is evaluated over
/// the JOINT box `(I_tau, B_perp)` in frame coordinates).
///
/// Refusal backing: an empty `w` is `WeightDegenerate` (Disproven, §7.1); a
/// perpendicular image that is not strictly inside `B_perp` is Inconclusive
/// (shrink-and-retry is licensed); a near-singular preconditioner beyond
/// [`KAPPA_MAX`] is Inconclusive (`Conditioning` — the caller rebuilds the
/// frame, §10.2).
// Refusal carries Option<PartialGraph> by frozen §2 shape; large-Err is allowed (BG-KV2-000).
#[allow(clippy::result_large_err)]
pub fn c2_certify_tube4(
    system: &SquareSystem3,
    frame: &Frame<4>,
    i_tau: Interval,
    b_perp: IBox<3>,
    w: &[CertifiedPositive],
) -> ClaimVerdict<ArcCert<4>, Refusal, Reason> {
    if w.is_empty() {
        return ClaimVerdict::Disproven(engine_refusal(
            RefusalKind::WeightDegenerate,
            "tube_weights_empty",
            "c2_certify_tube4 requires at least one certified positive weight bound (§7.1 value argument)"
                .to_string(),
        ));
    }
    if !i_tau.is_finite() || i_tau.lo > i_tau.hi {
        return ClaimVerdict::Disproven(engine_refusal(
            RefusalKind::ClaimRefuted,
            "tube_i_tau_invalid",
            "i_tau must be a finite, ordered interval".to_string(),
        ));
    }

    // Perpendicular radii and the frame-coordinate centre.
    let r: [f64; 3] = [
        (b_perp.hi[0] - b_perp.lo[0]) / 2.0,
        (b_perp.hi[1] - b_perp.lo[1]) / 2.0,
        (b_perp.hi[2] - b_perp.lo[2]) / 2.0,
    ];
    if r.iter().any(|c| !c.is_finite() || *c <= 0.0) {
        return ClaimVerdict::Disproven(engine_refusal(
            RefusalKind::NonFinite,
            "tube_radius_nonpositive",
            "c2_certify_tube4 requires a strictly positive finite radius on every perpendicular axis"
                .to_string(),
        ));
    }
    let y_hat: [f64; 3] = [
        (b_perp.lo[0] + b_perp.hi[0]) / 2.0,
        (b_perp.lo[1] + b_perp.hi[1]) / 2.0,
        (b_perp.lo[2] + b_perp.hi[2]) / 2.0,
    ];
    let tau_mid = (i_tau.lo + i_tau.hi) / 2.0;

    // The chart-space midpoint of the tube.
    let z_mid = chart_point(frame, tau_mid, y_hat);

    // The float perpendicular Jacobian at the midpoint and its inverse `A`.
    let partials = match certified_float_partials(system, z_mid) {
        Some(partials) => partials,
        None => return ClaimVerdict::Inconclusive("tube_partials_unavailable"),
    };
    let b: [[f64; 3]; 3] = perp_jacobian(&partials, frame);
    let a = match inv3_f64(b) {
        Some(a) => a,
        None => return ClaimVerdict::Inconclusive("tube_midpoint_jacobian_singular"),
    };
    let cond = norm_inf3(b) * norm_inf3(a);
    if !cond.is_finite() || cond > KAPPA_MAX {
        return ClaimVerdict::Inconclusive("tube_midpoint_conditioning");
    }

    // The interval perpendicular boxes (joint box and centre slice).
    let y_iv: Iv3 = [
        Interval {
            lo: b_perp.lo[0],
            hi: b_perp.hi[0],
        },
        Interval {
            lo: b_perp.lo[1],
            hi: b_perp.hi[1],
        },
        Interval {
            lo: b_perp.lo[2],
            hi: b_perp.hi[2],
        },
    ];
    let yc_iv: Iv3 = [
        Interval::point(y_hat[0]),
        Interval::point(y_hat[1]),
        Interval::point(y_hat[2]),
    ];
    let joint_box = match frame_tube_chart_box(system, frame, i_tau, &y_iv) {
        Some(box_) => box_,
        None => return ClaimVerdict::Inconclusive("tube_joint_box_outside_chart_domain"),
    };
    let slice_box = match frame_tube_chart_box(system, frame, i_tau, &yc_iv) {
        Some(box_) => box_,
        None => return ClaimVerdict::Inconclusive("tube_slice_box_outside_chart_domain"),
    };

    // F over the centre slice and D_yF over the joint box (interval).
    let f_slice = match system_values(system, slice_box) {
        Ok(v) => v,
        Err(_) => return ClaimVerdict::Inconclusive("tube_value_enclosure_failed"),
    };
    let df_chart = match system_jacobian(system, joint_box) {
        Ok(v) => v,
        Err(_) => return ClaimVerdict::Inconclusive("tube_jacobian_enclosure_failed"),
    };
    let mut dy: M3 = [[Interval::point(0.0); 3]; 3];
    for (r, dyrow) in dy.iter_mut().enumerate() {
        for (c, cell) in dyrow.iter_mut().enumerate() {
            let mut acc = Interval::point(0.0);
            for (j, df_rj) in df_chart[r].iter().enumerate() {
                acc = acc.add(&df_rj.mul(&Interval::point(frame.q_perp[c][j])));
            }
            *cell = acc;
        }
    }
    if dy.iter().flatten().any(|v| !v.is_finite()) {
        return ClaimVerdict::Inconclusive("tube_enclosure_not_finite");
    }

    // K = ŷ − A·F(□I_tau, ŷ) + (I − A·□D_yF)(B_perp − ŷ).
    let a_iv: M3 = {
        let mut out = [[Interval::point(0.0); 3]; 3];
        for r in 0..3 {
            for c in 0..3 {
                out[r][c] = Interval::point(a[r][c]);
            }
        }
        out
    };
    let af = matvec3_iv(&a_iv, &f_slice);
    let cj = matmul3_iv(&a_iv, &dy);
    let id_minus: M3 = [
        [
            Interval::point(1.0).sub(&cj[0][0]),
            cj[0][1].neg(),
            cj[0][2].neg(),
        ],
        [
            cj[1][0].neg(),
            Interval::point(1.0).sub(&cj[1][1]),
            cj[1][2].neg(),
        ],
        [
            cj[2][0].neg(),
            cj[2][1].neg(),
            Interval::point(1.0).sub(&cj[2][2]),
        ],
    ];
    let dx = centred_dx3_axis(&y_iv, &yc_iv);
    let md = matvec3_iv(&id_minus, &dx);
    let k: Iv3 = [
        yc_iv[0].sub(&af[0]).add(&md[0]),
        yc_iv[1].sub(&af[1]).add(&md[1]),
        yc_iv[2].sub(&af[2]).add(&md[2]),
    ];
    if k.iter().any(|v| !v.is_finite()) {
        return ClaimVerdict::Inconclusive("tube_enclosure_not_finite");
    }

    // Strict inclusion of the perpendicular image in B_perp for ALL tau.
    for ((lo_i, hi_i), k_i) in b_perp.lo.iter().zip(b_perp.hi.iter()).zip(k.iter()) {
        match classify_axis(*lo_i, *hi_i, k_i.lo, k_i.hi) {
            Inclusion::Strict => {}
            _ => return ClaimVerdict::Inconclusive("tube_perpendicular_image_not_strict"),
        }
    }

    // Lemma 8.0's contraction rate over B_perp's radii.
    let rho = {
        let mut rho = 0.0f64;
        for (i, row) in id_minus.iter().enumerate() {
            let mr = mag(&row[0]) * r[0] + mag(&row[1]) * r[1] + mag(&row[2]) * r[2];
            let ratio = mr / r[i];
            if !ratio.is_finite() {
                return ClaimVerdict::Inconclusive("tube_rho_not_finite");
            }
            rho = rho.max(ratio);
        }
        rho
    };
    if rho > RHO_MAX {
        return ClaimVerdict::Inconclusive("tube_rho_exceeds_rho_max");
    }

    // Per-column Jacobian enclosures of D_yF over the joint box.
    let mut jac_encl = Vec::with_capacity(3);
    for col in 0..3 {
        let mut lo = f64::INFINITY;
        let mut hi = f64::NEG_INFINITY;
        for row in dy.iter() {
            lo = lo.min(row[col].lo);
            hi = hi.max(row[col].hi);
        }
        jac_encl.push([lo, hi]);
    }

    // Lift the box into the q_perp-aligned IBox<4> convention: axes 0..=2 are
    // the perpendicular coordinates, axis 3 is the tangent interval.
    let lo4 = [b_perp.lo[0], b_perp.lo[1], b_perp.lo[2], i_tau.lo];
    let hi4 = [b_perp.hi[0], b_perp.hi[1], b_perp.hi[2], i_tau.hi];
    let b_perp4 = match IBox::<4>::try_new(lo4, hi4) {
        Ok(b) => b,
        Err(_) => return ClaimVerdict::Inconclusive("tube_box_lift_failed"),
    };

    let weights = Some(w.to_vec());
    match ArcCert::try_new(
        ResidualId::R1,
        *frame,
        i_tau,
        b_perp4,
        rho,
        jac_encl,
        weights,
    ) {
        Ok(cert) => ClaimVerdict::Proven(cert),
        Err(refusal) => ClaimVerdict::Disproven(refusal),
    }
}
