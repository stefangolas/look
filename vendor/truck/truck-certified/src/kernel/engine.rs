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
pub(crate) enum HullErr {
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

/// The dot product of two 4-vectors.
fn dot4(a: &[f64; 4], b: &[f64; 4]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2] + a[3] * b[3]
}

/// §8.1/§11 frame construction: `q_tau = m/||m||` (IEEE sqrt, deterministic),
/// the perpendicular basis by TWO-PASS (reorthogonalized) Gram–Schmidt in
/// FIXED index order, and the `a = [DF(ẑ) Q_⊥]⁻¹` preconditioner (embedded as
/// the 4x4 `Frame` field with the tangent axis carried identically).
///
/// Two passes drive the residual perpendicular–tangent dot products to machine
/// precision for every non-degenerate input, so the §8.1 orthonormality gate
/// (in the frozen [`Frame::try_new`]) is not tripped by rounding near
/// degenerate seeds. If a residual dot still exceeds [`TOL_JACOBIAN`] after two
/// passes, the input is genuinely near-rank-collapse and this refuses
/// `Conditioning` (Inconclusive) — the caller subdivides (spec §9.2's `k_a`
/// discipline does not apply here; the seed quality is the caller's).
///
/// Returns the frame and the float kernel direction `m`. If `||m||` is below
/// the normative floor (rank 2 territory) or the frame Jacobian block is
/// singular, refuses `Conditioning` (Inconclusive) — the caller subdivides or
/// switches coordinate; rank 2 is S0/S5a territory, not this packet's.
///
/// The frame's κ estimate (the §0.4 conditioning measure `||DF(ẑ)Q_⊥||·||(DF(ẑ)
/// Q_⊥)⁻¹||`, compared against [`KAPPA_MAX`] by the tube seam) is not carried
/// by the frozen tuple; it is reported through [`frame4_kappa_report`].
// Refusal carries Option<PartialGraph> by frozen §2 shape; large-Err is allowed (BG-KV2-000).
#[allow(clippy::result_large_err)]
pub fn build_frame4(system: &SquareSystem3, z_hat: [f64; 4]) -> Construction<(Frame<4>, [f64; 4])> {
    let built = frame4_impl(system, z_hat)?;
    Ok((built.frame, built.m))
}

/// The outcome of a §8.1 frame construction: the frame, the float kernel
/// direction `m`, and the frame's κ estimate (the conditioning measure the
/// tube seam compares against [`KAPPA_MAX`]).
#[derive(Debug, Clone, Copy)]
struct FrameBuild4 {
    /// The §8.1 frame.
    frame: Frame<4>,
    /// The float maximal-minor kernel direction at `z_hat`.
    m: [f64; 4],
    /// The κ estimate `||DF(ẑ)Q_⊥||_∞ · ||(DF(ẑ)Q_⊥)⁻¹||_∞` of the frame.
    kappa: f64,
}

/// The two-pass (reorthogonalized) Gram–Schmidt body shared by
/// [`build_frame4`] and [`frame4_kappa_report`].
// Refusal carries Option<PartialGraph> by frozen §2 shape; large-Err is allowed (BG-KV2-000).
#[allow(clippy::result_large_err)]
fn frame4_impl(system: &SquareSystem3, z_hat: [f64; 4]) -> Result<FrameBuild4, Refusal> {
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

    // Deterministic TWO-PASS Gram-Schmidt over the fixed candidate order
    // e_0..e_3. Each candidate is projected against the accepted basis twice
    // (reorthogonalization): one pass leaves residual dot products at the
    // rounding level of the seed (1e-11..1e-9 near degenerate seeds); the
    // second drives them to machine precision. A residual dot that still
    // exceeds TOL_JACOBIAN after two passes is a genuine near-rank-collapse
    // seed: refuse Conditioning (the caller subdivides).
    let mut perp: Vec<[f64; 4]> = Vec::with_capacity(3);
    let mut basis: Vec<[f64; 4]> = vec![q_tau];
    for k in 0..4 {
        if perp.len() == 3 {
            break;
        }
        let mut e = [0.0f64; 4];
        e[k] = 1.0;
        let mut v = e;
        for _pass in 0..2 {
            for fixed in &basis {
                let dot = dot4(&v, fixed);
                for j in 0..4 {
                    v[j] -= dot * fixed[j];
                }
            }
        }
        let v_norm_sq = v[0] * v[0] + v[1] * v[1] + v[2] * v[2] + v[3] * v[3];
        if !v_norm_sq.is_finite() || v_norm_sq <= TOL_JACOBIAN * TOL_JACOBIAN {
            continue;
        }
        let v_norm = v_norm_sq.sqrt();
        let unit = [v[0] / v_norm, v[1] / v_norm, v[2] / v_norm, v[3] / v_norm];
        if basis
            .iter()
            .any(|fixed| dot4(&unit, fixed).abs() > TOL_JACOBIAN)
        {
            // Two full passes still leave a residual dot above the gate: the
            // candidate is not separable from the accepted basis at the
            // required precision — a genuinely near-rank-collapse seed.
            return Err(engine_refusal(
                RefusalKind::Conditioning,
                "frame_two_pass_residual_above_gate",
                "after two Gram-Schmidt passes a perpendicular candidate still has a residual \
                 dot above TOL_JACOBIAN: the seed is genuinely near-rank-collapse (the caller \
                 subdivides)"
                    .to_string(),
            ));
        }
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

    // The perpendicular Jacobian block B = DF(z_hat)·Q_⊥, its inverse, and the
    // frame's κ estimate (the §0.4 conditioning measure the tube seam compares
    // against KAPPA_MAX — reported, not a build gate).
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
    let kappa = norm_inf3(b) * norm_inf3(a33);
    if !kappa.is_finite() {
        return Err(engine_refusal(
            RefusalKind::Conditioning,
            "frame_kappa_not_finite",
            "the frame's kappa estimate is not finite".to_string(),
        ));
    }

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
    Ok(FrameBuild4 { frame, m, kappa })
}

/// The §8.1 conditioning report of a frame built at `z_hat` (BG-KV2-307):
/// whether the frame's κ estimate crosses the §0.4 [`KAPPA_MAX`] rebuild bound.
///
/// A κ above [`KAPPA_MAX`] marks the frame rebuild-recommended in the
/// certificate evidence (spec §10.1's rebuild rule: re-factor only when
/// `κ(DF Q_⊥) > κ_max`). The build gate itself stays the orthonormality check
/// in [`build_frame4`]; this report never refuses an orthonormal frame.
#[derive(Debug, Clone, Copy)]
pub struct FrameKappaReport {
    /// The frame's κ estimate `||DF(ẑ)Q_⊥||_∞ · ||(DF(ẑ)Q_⊥)⁻¹||_∞`.
    pub kappa: f64,
    /// Whether `kappa > KAPPA_MAX` (frame rebuild-recommended, spec §10.1).
    pub rebuild_recommended: bool,
}

/// Build the §8.1 frame at `z_hat` and report its κ estimate. Refuses exactly
/// as [`build_frame4`] does; the report is additive (the frozen seam tuple is
/// unchanged).
// Refusal carries Option<PartialGraph> by frozen §2 shape; large-Err is allowed (BG-KV2-000).
#[allow(clippy::result_large_err)]
pub fn frame4_kappa_report(
    system: &SquareSystem3,
    z_hat: [f64; 4],
) -> Construction<(Frame<4>, FrameKappaReport)> {
    let built = frame4_impl(system, z_hat)?;
    let report = FrameKappaReport {
        kappa: built.kappa,
        rebuild_recommended: built.kappa > KAPPA_MAX,
    };
    Ok((built.frame, report))
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
///
/// `pub(crate)` since BG-KV2-304-S3B: the Tier-2 `Psi_a` residual (the §7 R3
/// minor form, `kernel/tier2.rs`) composes the value enclosure of `F` with the
/// Theorem 6.4(iii) `a·m` enclosure. Additive exposure only; the machinery is
/// unchanged.
pub(crate) fn system_values(
    system: &SquareSystem3,
    box_: [(f64, f64); 4],
) -> Result<[Interval; 3], HullErr> {
    let mut out = [Interval::point(0.0); 3];
    for (k, out_k) in out.iter_mut().enumerate() {
        *out_k = component_value(system, k, box_)?;
    }
    Ok(out)
}

/// The certified chart-coordinate partial matrix (3 components x 4 axes) over
/// a chart box.
///
/// `pub(crate)` since BG-KV2-304-S3B: the Tier-2 `Psi_a` residual composes the
/// Theorem 6.4 maximal-minor enclosure of `m` from this Jacobian enclosure.
/// Additive exposure only; the machinery is unchanged.
pub(crate) fn system_jacobian(
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

/// A zero coefficient grid of the given degrees (the second-partial-of-a-
/// linear-axis case): every stored coefficient is `0.0`.
fn zero_grid(degrees: (usize, usize, usize, usize)) -> Grid4 {
    let (m1, n1, m2, n2) = degrees;
    let rows = (m1 + 1) * (n1 + 1);
    let cols = (m2 + 1) * (n2 + 1);
    Grid4 {
        degrees,
        rows: vec![vec![0.0f64; cols]; rows],
    }
}

/// The certified second-partial coefficient grid of one stored component along
/// the chart axes `j` then `l` (in the unit chart; the caller scales by the
/// chart widths). The double derivative along a linear axis is the zero
/// polynomial, represented as a zero grid of the reduced degree.
fn grid_second_partial(grid: &Grid4, j: usize, l: usize) -> Result<Grid4, HullErr> {
    if j > 3 || l > 3 {
        return Err(HullErr::Unavailable);
    }
    let derived = grid.partial_axis(j)?;
    if l == j && derived.len_axis(j) == 1 {
        // The axis is linear: the second derivative along it is identically
        // zero, and a further partial would refuse a degree-0 axis.
        return Ok(zero_grid(derived.degrees));
    }
    derived.partial_axis(l)
}

/// The certified chart-coordinate second-partial enclosure of one component
/// along the chart axes `j` and `l` over a chart box (the Hessian entry
/// `∂²F_component/∂x_j ∂x_l`, scaled by the two inverse chart widths).
fn component_second_partial(
    system: &SquareSystem3,
    component: usize,
    j: usize,
    l: usize,
    box_: [(f64, f64); 4],
) -> Result<Interval, HullErr> {
    if component > 2 || j > 3 || l > 3 {
        return Err(HullErr::Unavailable);
    }
    let unit = to_unit_box(system, box_).ok_or(HullErr::DomainNotCompact)?;
    let grid = system_grid(system, component);
    let derived = grid_second_partial(&grid, j, l)?;
    let hull = hull_grid4(&derived, unit)?;
    let rect = chart_rects(system);
    let width = Interval::point(rect[j].1)
        .sub(&Interval::point(rect[j].0))
        .mul(&Interval::point(rect[l].1).sub(&Interval::point(rect[l].0)));
    match hull.div(&width) {
        Some(out) if out.is_finite() => Ok(out),
        _ => Err(HullErr::Unavailable),
    }
}

/// The certified Hessian tensor of the stored system over a chart box:
/// `out[r][j][l]` encloses `∂²F_r/∂x_j ∂x_l` for the three spatial components
/// `r` and the four product-space axes `(j, l)`.
///
/// `pub(crate)` since BG-KV2-304-S3B: the Tier-2 `Psi_a` residual's Jacobian
/// row (the gradient of the `a·m` component) is assembled from this tensor.
/// Additive exposure only; the machinery is unchanged.
pub(crate) fn system_hessian(
    system: &SquareSystem3,
    box_: [(f64, f64); 4],
) -> Result<[[[Interval; 4]; 4]; 3], HullErr> {
    let mut out = [[[Interval::point(0.0); 4]; 4]; 3];
    for (r, orow) in out.iter_mut().enumerate() {
        for (j, ocol) in orow.iter_mut().enumerate() {
            for (l, cell) in ocol.iter_mut().enumerate() {
                *cell = component_second_partial(system, r, j, l, box_)?;
            }
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

// ---------------------------------------------------------------------------
// The tube-reach envelope probe (BG-KV2-307-ENGINEREACH §2): a PUBLISHED
// characteristic, profiled not tuned against (spec §18 discipline)
// ---------------------------------------------------------------------------

/// The probe's starting forward arc width (a proposal constant, H-3): the
/// tracer's §10.1 default `arc_step0`, reused verbatim so the probe and the
/// march probe the same envelope.
const PROBE_ARC_STEP0: f64 = 0.05; // H-3: reach-probe start width (= TracePolicy default arc_step0)

/// The probe's perpendicular half-width ratio (a proposal constant, H-3): the
/// tracer's `PERP_RATIO` — the S4A observation the probe confirms or corrects.
const PROBE_PERP_RATIO: f64 = 3.0; // H-3: reach-probe perpendicular half-width ratio

/// The halving floor of the tau-width search: below this width a seed has no
/// certifiable forward arc (a probe floor, H-3; far below the tracer's dtau
/// floor so curved "hard parts" are still measurable).
const PROBE_MIN_WIDTH: f64 = 1e-6; // H-3: reach-probe width search floor

/// The bisection iterations of each reach search (a probe precision setting,
/// H-3).
const PROBE_BISECT_ITERS: u32 = 60; // H-3: reach-probe bisection iterations

/// The relative tolerance that declares the tau reach chart-limited (a probe
/// precision setting, H-3).
const PROBE_CHART_LIMITED_REL: f64 = 1e-3; // H-3: reach-probe chart-limit relative tolerance

/// The measured single-frame tube-reach envelope of a seed (BG-KV2-307 §2).
///
/// The envelope is a PUBLISHED characteristic of the frozen tube seam: it is
/// profiled, never tuned against. Both searches replicate the tracer's
/// proposal discipline (one Gauss–Newton predictor step reusing `Frame::a`,
/// perpendicular half width [`PROBE_PERP_RATIO`] times the arc width) so the
/// published numbers describe what the march can actually certify from the
/// seed's single frame.
#[derive(Debug, Clone)]
pub struct ProbeReport {
    /// Whether the seed framed at all (false when `build_frame4` refused).
    pub frame_ok: bool,
    /// A machine-readable note (predicate name) when the seed did not frame or
    /// the probe hit a degenerate edge; `None` on a clean measurement.
    pub note: Option<String>,
    /// The frame's κ estimate at the seed (`||DF Q_⊥||_∞ · ||(DF Q_⊥)⁻¹||_∞`).
    pub frame_kappa: f64,
    /// Whether `frame_kappa > KAPPA_MAX` (frame rebuild-recommended, §10.1).
    pub rebuild_recommended: bool,
    /// The largest FORWARD arc width `[0, W]` the frozen tube certifies from
    /// the seed frame at the [`PROBE_PERP_RATIO`] perpendicular half width.
    pub tau_reach: f64,
    /// Whether `tau_reach` saturates the forward chart room (the certified
    /// reach is chart-boundary-limited rather than certificate-limited).
    pub tau_reach_chart_limited: bool,
    /// Whether no positive arc width certified down to [`PROBE_MIN_WIDTH`].
    pub no_reach: bool,
    /// The reference arc width the perpendicular probe ran at (equal to the
    /// measured `tau_reach`, or 0 when there is no certified reach to probe).
    pub perp_probe_width: f64,
    /// The smallest perpendicular half width at which the reference arc still
    /// certifies (the inverse of the S4A "3x width" observation).
    pub perp_half_min: f64,
    /// `perp_half_min / perp_probe_width`: the measured minimum perpendicular
    /// ratio, confirming or correcting the ~3x observation.
    pub perp_ratio_min: f64,
}

/// A single certified unit weight bound (the tracer's §7.1 value argument).
fn probe_weight() -> Option<CertifiedPositive> {
    CertifiedPositive::try_new(1.0).ok()
}

/// The certified float value of the three residual components at a chart point
/// (the midpoint of the certified enclosure over the degenerate point box).
fn float_residual_mid(sys: &SquareSystem3, point: [f64; 4]) -> Option<[f64; 3]> {
    let values = system_values(sys, point_box(point)).ok()?;
    let mut out = [0.0f64; 3];
    for (k, out_k) in out.iter_mut().enumerate() {
        *out_k = 0.5 * (values[k].lo + values[k].hi);
        if !out_k.is_finite() {
            return None;
        }
    }
    Some(out)
}

/// One Gauss–Newton predictor step reusing the seed factorization (`Frame::a`)
/// — the tracer's cheap-predictor rule (§10.1). Float proposal data only.
fn probe_predict_y(frame: &Frame<4>, sys: &SquareSystem3, tau: f64) -> Option<[f64; 3]> {
    let p = chart_point(frame, tau, [0.0, 0.0, 0.0]);
    let f = float_residual_mid(sys, p)?;
    let mut y = [0.0f64; 3];
    for (r, row) in frame.a.iter().take(3).enumerate() {
        let mut acc = 0.0f64;
        for (aa, ff) in row.iter().take(3).zip(f.iter()) {
            acc += aa * ff;
        }
        y[r] = -acc;
    }
    if y.iter().all(|v| v.is_finite()) {
        Some(y)
    } else {
        None
    }
}

/// Whether the frozen tube certifies the forward arc `[0, width]` from the
/// seed frame with the given perpendicular half width (the tracer's proposal
/// shape: the box is centred on the predicted arc-end centre).
fn probe_arc_certifies(
    sys: &SquareSystem3,
    frame: &Frame<4>,
    weight: &CertifiedPositive,
    width: f64,
    half_perp: f64,
) -> bool {
    if !width.is_finite() || width <= 0.0 || !half_perp.is_finite() || half_perp <= 0.0 {
        return false;
    }
    let y_pred = match probe_predict_y(frame, sys, width) {
        Some(y) => y,
        None => return false,
    };
    let lo = [
        y_pred[0] - half_perp,
        y_pred[1] - half_perp,
        y_pred[2] - half_perp,
    ];
    let hi = [
        y_pred[0] + half_perp,
        y_pred[1] + half_perp,
        y_pred[2] + half_perp,
    ];
    let b_perp = match IBox::<3>::try_new(lo, hi) {
        Ok(b) => b,
        Err(_) => return false,
    };
    let i_tau = Interval { lo: 0.0, hi: width };
    matches!(
        c2_certify_tube4(sys, frame, i_tau, b_perp, std::slice::from_ref(weight)),
        ClaimVerdict::Proven(_)
    )
}

/// The chart room ahead of the seed along `+tau` (how far the forward arc may
/// extend before the midpoint leaves the chart rectangle).
fn probe_forward_room(sys: &SquareSystem3, frame: &Frame<4>) -> f64 {
    let rects = chart_rects(sys);
    let mut room = f64::INFINITY;
    for ((&d, (lo, hi)), &z) in frame.q_tau.iter().zip(rects.iter()).zip(frame.z_hat.iter()) {
        let rem = if d > 0.0 {
            (hi - z) / d
        } else if d < 0.0 {
            (lo - z) / d
        } else {
            continue;
        };
        if rem.is_finite() {
            room = room.min(rem);
        }
    }
    room
}

/// Grow-then-halve with a closing bisection: find the largest forward arc
/// width that certifies at the [`PROBE_PERP_RATIO`] perpendicular half width.
fn probe_tau_reach(
    sys: &SquareSystem3,
    frame: &Frame<4>,
    weight: &CertifiedPositive,
) -> (f64, bool, bool) {
    let ceiling = 0.999 * probe_forward_room(sys, frame);
    let start = PROBE_ARC_STEP0.min(ceiling);
    if !start.is_finite() || start <= PROBE_MIN_WIDTH {
        return (0.0, false, true);
    }
    let certifies = |w: f64| probe_arc_certifies(sys, frame, weight, w, PROBE_PERP_RATIO * w);

    let (mut lo, mut hi) = if certifies(start) {
        // Grow on success (halving never runs on the growth side).
        let mut w = start;
        loop {
            let next = (2.0 * w).min(ceiling);
            if next <= w {
                break (w, w);
            }
            if certifies(next) {
                w = next;
            } else {
                break (w, next);
            }
        }
    } else {
        // Halve on failure down to the probe floor.
        let mut w = start;
        loop {
            if certifies(w) {
                break (w, start);
            }
            w *= 0.5;
            if w < PROBE_MIN_WIDTH {
                return (0.0, false, true);
            }
        }
    };
    if hi <= lo {
        hi = ceiling;
    }
    // Close the interval with a deterministic bisection.
    for _ in 0..PROBE_BISECT_ITERS {
        let mid = 0.5 * (lo + hi);
        if mid <= lo || mid >= hi {
            break;
        }
        if certifies(mid) {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let chart_limited = ceiling.is_finite() && (ceiling - lo) <= PROBE_CHART_LIMITED_REL * ceiling;
    (lo, chart_limited, lo <= 0.0)
}

/// The smallest perpendicular half width at which the reference arc still
/// certifies (bisection between a degenerate radius 0 and the certifying
/// [`PROBE_PERP_RATIO`] half width).
fn probe_perp_min(
    sys: &SquareSystem3,
    frame: &Frame<4>,
    weight: &CertifiedPositive,
    width: f64,
) -> Option<f64> {
    if width <= 0.0 {
        return None;
    }
    let hi_start = PROBE_PERP_RATIO * width;
    if !probe_arc_certifies(sys, frame, weight, width, hi_start) {
        return None;
    }
    let mut lo = 0.0f64;
    let mut hi = hi_start;
    for _ in 0..PROBE_BISECT_ITERS {
        let mid = 0.5 * (lo + hi);
        if mid <= lo || mid >= hi {
            break;
        }
        if probe_arc_certifies(sys, frame, weight, width, mid) {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    Some(hi)
}

/// Measure the tube-reach envelope of a seed (BG-KV2-307 §2): the largest
/// forward `I_tau` width the frozen tube certifies from the seed's single
/// frame at the [`PROBE_PERP_RATIO`] perpendicular half width, and the largest
/// perpendicular half width at that tau width.
///
/// The returned [`ProbeReport`] is the PUBLISHED characteristic (spec §18):
/// profile, do not tune against it.
// Refusal carries Option<PartialGraph> by frozen §2 shape; large-Err is allowed (BG-KV2-000).
#[allow(clippy::result_large_err)]
pub fn tube_reach_probe(sys: &SquareSystem3, seed: [f64; 4]) -> ProbeReport {
    let build = match frame4_impl(sys, seed) {
        Ok(built) => built,
        Err(refusal) => {
            let note = match &refusal.evidence {
                RefusalEvidence::Predicate { name, .. } => Some((*name).to_string()),
                _ => Some("seed_frame_refused".to_string()),
            };
            return ProbeReport {
                frame_ok: false,
                note,
                frame_kappa: f64::NAN,
                rebuild_recommended: false,
                tau_reach: 0.0,
                tau_reach_chart_limited: false,
                no_reach: true,
                perp_probe_width: 0.0,
                perp_half_min: f64::NAN,
                perp_ratio_min: f64::NAN,
            };
        }
    };
    let frame = build.frame;
    let weight = match probe_weight() {
        Some(w) => w,
        None => {
            return ProbeReport {
                frame_ok: true,
                note: Some("probe_weight_unavailable".to_string()),
                frame_kappa: build.kappa,
                rebuild_recommended: build.kappa > KAPPA_MAX,
                tau_reach: 0.0,
                tau_reach_chart_limited: false,
                no_reach: true,
                perp_probe_width: 0.0,
                perp_half_min: f64::NAN,
                perp_ratio_min: f64::NAN,
            }
        }
    };
    let (tau_reach, chart_limited, no_reach) = probe_tau_reach(sys, &frame, &weight);
    let perp_probe_width = if no_reach { 0.0 } else { tau_reach };
    let (perp_half_min, perp_ratio_min) =
        match probe_perp_min(sys, &frame, &weight, perp_probe_width) {
            Some(h) => (h, h / perp_probe_width),
            None => (f64::NAN, f64::NAN),
        };
    ProbeReport {
        frame_ok: true,
        note: None,
        frame_kappa: build.kappa,
        rebuild_recommended: build.kappa > KAPPA_MAX,
        tau_reach,
        tau_reach_chart_limited: chart_limited,
        no_reach,
        perp_probe_width,
        perp_half_min,
        perp_ratio_min,
    }
}

// ---------------------------------------------------------------------------
// The n=3 Krawczyk arm (BG-KV2-206-N3CERT): the arity-3 R8 C1 carrier
// ---------------------------------------------------------------------------

use crate::kernel::certs::PointCert3;
use crate::kernel::patch::IBox3;

/// The float midpoint centre of an `IBox<3>`.
fn centre3(b: &IBox3) -> [f64; 3] {
    [
        (b.lo[0] + b.hi[0]) / 2.0,
        (b.lo[1] + b.hi[1]) / 2.0,
        (b.lo[2] + b.hi[2]) / 2.0,
    ]
}

/// The interval radius vector of an `IBox<3>`, `None` on a non-positive or
/// non-finite radius.
fn radii3(b: &IBox3) -> Option<[f64; 3]> {
    let r = [
        (b.hi[0] - b.lo[0]) / 2.0,
        (b.hi[1] - b.lo[1]) / 2.0,
        (b.hi[2] - b.lo[2]) / 2.0,
    ];
    if r.iter().all(|c| c.is_finite() && *c > 0.0) {
        Some(r)
    } else {
        None
    }
}

/// Determinant of a 3x3 interval matrix under directed rounding (the same
/// cofactor expansion as [`det3_f64`], over interval arithmetic).
fn det3_iv(m: &M3) -> Interval {
    let a = m[0][0].mul(&m[1][1].mul(&m[2][2]).sub(&m[1][2].mul(&m[2][1])));
    let b = m[0][1].mul(&m[1][0].mul(&m[2][2]).sub(&m[1][2].mul(&m[2][0])));
    let c = m[0][2].mul(&m[1][0].mul(&m[2][1]).sub(&m[1][1].mul(&m[2][0])));
    a.sub(&b).add(&c)
}

/// The interval inverse of a 3x3 matrix via adjugate over determinant (the
/// same adjugate layout as [`inv3_f64`], over interval arithmetic). `None`
/// when the determinant enclosure contains (or is) zero or a quotient is not
/// finite.
fn inv3_iv(m: &M3) -> Option<M3> {
    let det = det3_iv(m);
    if !det.is_finite() || (det.lo <= 0.0 && det.hi >= 0.0) {
        return None;
    }
    let adj: M3 = [
        [
            m[1][1].mul(&m[2][2]).sub(&m[1][2].mul(&m[2][1])),
            m[0][2].mul(&m[2][1]).sub(&m[0][1].mul(&m[2][2])),
            m[0][1].mul(&m[1][2]).sub(&m[0][2].mul(&m[1][1])),
        ],
        [
            m[1][2].mul(&m[2][0]).sub(&m[1][0].mul(&m[2][2])),
            m[0][0].mul(&m[2][2]).sub(&m[0][2].mul(&m[2][0])),
            m[0][2].mul(&m[1][0]).sub(&m[0][0].mul(&m[1][2])),
        ],
        [
            m[1][0].mul(&m[2][1]).sub(&m[1][1].mul(&m[2][0])),
            m[0][1].mul(&m[2][0]).sub(&m[0][0].mul(&m[2][1])),
            m[0][0].mul(&m[1][1]).sub(&m[0][1].mul(&m[1][0])),
        ],
    ];
    let mut out = [[Interval::point(0.0); 3]; 3];
    for r in 0..3 {
        for c in 0..3 {
            out[r][c] = adj[r][c].div(&det)?;
        }
    }
    Some(out)
}

/// The outward-rounded box `B − z_hat` (centred box), replicating the landed
/// 2D reduction's op order at n = 3.
fn centred_dx3(b: &IBox3, z: &[Interval; 3]) -> [Interval; 3] {
    let mut dx = [Interval::point(0.0); 3];
    for k in 0..3 {
        let d_lo = Interval::point(b.lo[k]).sub(&z[k]);
        let d_hi = Interval::point(b.hi[k]).sub(&z[k]);
        dx[k] = Interval {
            lo: d_lo.lo.min(d_hi.lo),
            hi: d_lo.hi.max(d_hi.hi),
        };
    }
    dx
}

/// Lemma 8.0's contraction rate `max_i (M r)_i / r_i` at n = 3. `None` when a
/// quotient is not finite.
fn rho3(id_minus: &M3, r: [f64; 3]) -> Option<f64> {
    let mut rho = 0.0f64;
    for i in 0..3 {
        let mr =
            mag(&id_minus[i][0]) * r[0] + mag(&id_minus[i][1]) * r[1] + mag(&id_minus[i][2]) * r[2];
        let ratio = mr / r[i];
        if !ratio.is_finite() {
            return None;
        }
        rho = rho.max(ratio);
    }
    Some(rho)
}

/// The arity-3 C1 entry (R8-class): identical operator discipline to
/// [`krawczyk_c1`] (Lemma 8.0 + §8.2), on the n=3 adjugate/det path, emitting
/// a [`PointCert3`]. Weight bounds remain the §7.1 value argument.
///
/// The Disproven/Inconclusive backing table is IDENTICAL to
/// [`krawczyk_c1`]'s: an empty `w` is `WeightDegenerate` (Disproven); a
/// non-positive/non-finite radius is `NonFinite` (Disproven); a disjoint
/// image is `ClaimRefuted` (Disproven); a merely overlapping image, a
/// singular midpoint Jacobian, a non-finite enclosure, or a `rho > RHO_MAX`
/// is Inconclusive. ResidualId stamping keeps the S2A convention: the engine
/// stamps [`ResidualId::R1`] and the caller rebuilds the certificate with its
/// own id through `PointCert3::try_new` (the documented one-line seam).
pub fn krawczyk_c1_n3(
    g: &dyn SquareResidualEval,
    b: IBox3,
    w: &[CertifiedPositive],
) -> ClaimVerdict<PointCert3, Refusal, Reason> {
    if g.arity() != 3 {
        return ClaimVerdict::Inconclusive("c1_n3_arity_mismatch_box_dimension");
    }
    if w.is_empty() {
        return ClaimVerdict::Disproven(engine_refusal(
            RefusalKind::WeightDegenerate,
            "c1_n3_weights_empty",
            "krawczyk_c1_n3 requires at least one certified positive weight bound (§7.1 value argument)"
                .to_string(),
        ));
    }
    let r = match radii3(&b) {
        Some(r) => r,
        None => {
            return ClaimVerdict::Disproven(engine_refusal(
                RefusalKind::NonFinite,
                "c1_n3_radius_nonpositive",
                "krawczyk_c1_n3 requires a strictly positive finite radius on every box axis"
                    .to_string(),
            ))
        }
    };
    let z = centre3(&b);
    let ziv: [Interval; 3] = [
        Interval::point(z[0]),
        Interval::point(z[1]),
        Interval::point(z[2]),
    ];
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

    let r0 = g.eval(&ziv);
    if r0.len() != 3 {
        return ClaimVerdict::Inconclusive("c1_n3_eval_arity_mismatch");
    }
    let j0_rows = g.jac_encl(&ziv);
    let jb_rows = g.jac_encl(&box_iv);
    if j0_rows.len() != 3
        || j0_rows.iter().any(|row| row.len() != 3)
        || jb_rows.len() != 3
        || jb_rows.iter().any(|row| row.len() != 3)
    {
        return ClaimVerdict::Inconclusive("c1_n3_jac_arity_mismatch");
    }

    let j0: M3 = [
        [j0_rows[0][0], j0_rows[0][1], j0_rows[0][2]],
        [j0_rows[1][0], j0_rows[1][1], j0_rows[1][2]],
        [j0_rows[2][0], j0_rows[2][1], j0_rows[2][2]],
    ];
    let jb: M3 = [
        [jb_rows[0][0], jb_rows[0][1], jb_rows[0][2]],
        [jb_rows[1][0], jb_rows[1][1], jb_rows[1][2]],
        [jb_rows[2][0], jb_rows[2][1], jb_rows[2][2]],
    ];

    // A = the interval inverse of the midpoint (centre) Jacobian.
    let a = match inv3_iv(&j0) {
        Some(a) => a,
        None => return ClaimVerdict::Inconclusive("c1_n3_midpoint_jacobian_singular"),
    };

    // (I − A·□DR(B)) and the Krawczyk image K(B).
    let cj = matmul3_iv(&a, &jb);
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
    if id_minus.iter().flatten().any(|v| !v.is_finite()) {
        return ClaimVerdict::Inconclusive("c1_n3_enclosure_not_finite");
    }
    let dx = centred_dx3(&b, &ziv);
    let r0v: Iv3 = [r0[0], r0[1], r0[2]];
    let ch = matvec3_iv(&a, &r0v);
    let md = matvec3_iv(&id_minus, &dx);
    let k: Iv3 = [
        ziv[0].sub(&ch[0]).add(&md[0]),
        ziv[1].sub(&ch[1]).add(&md[1]),
        ziv[2].sub(&ch[2]).add(&md[2]),
    ];
    if k.iter().any(|v| !v.is_finite()) {
        return ClaimVerdict::Inconclusive("c1_n3_enclosure_not_finite");
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
                "c1_n3_k_disjoint_no_root_in_box",
                "the Krawczyk image is disjoint from the box: no root of the residual in the box"
                    .to_string(),
            ));
        }
        return ClaimVerdict::Inconclusive("c1_n3_inclusion_not_strict");
    }

    // Lemma 8.0's contraction rate.
    let rho = match rho3(&id_minus, r) {
        Some(rho) => rho,
        None => return ClaimVerdict::Inconclusive("c1_n3_rho_not_finite"),
    };
    if rho > RHO_MAX {
        return ClaimVerdict::Inconclusive("c1_n3_rho_exceeds_rho_max");
    }
    // See the module-doc seam judgement: the engine stamps R1.
    match PointCert3::try_new(ResidualId::R1, b, rho) {
        Ok(cert) => ClaimVerdict::Proven(cert),
        Err(refusal) => ClaimVerdict::Disproven(refusal),
    }
}

// ---------------------------------------------------------------------------
// The n=4 Krawczyk arm (BG-KV2-304-S3B): the additive arity-4 C1 carrier for
// Tier-2's square `Psi_a = (F, a·m)` system (the §7 R3 minor form, §9.2)
// ---------------------------------------------------------------------------

use crate::kernel::certs::{IBox4, PointCert4};

type Iv4 = [Interval; 4];
type M4 = [[Interval; 4]; 4];

/// The float midpoint centre of an `IBox<4>`.
fn centre4(b: &IBox4) -> [f64; 4] {
    [
        (b.lo[0] + b.hi[0]) / 2.0,
        (b.lo[1] + b.hi[1]) / 2.0,
        (b.lo[2] + b.hi[2]) / 2.0,
        (b.lo[3] + b.hi[3]) / 2.0,
    ]
}

/// The interval radius vector of an `IBox<4>`, `None` on a non-positive or
/// non-finite radius.
fn radii4(b: &IBox4) -> Option<[f64; 4]> {
    let r = [
        (b.hi[0] - b.lo[0]) / 2.0,
        (b.hi[1] - b.lo[1]) / 2.0,
        (b.hi[2] - b.lo[2]) / 2.0,
        (b.hi[3] - b.lo[3]) / 2.0,
    ];
    if r.iter().all(|c| c.is_finite() && *c > 0.0) {
        Some(r)
    } else {
        None
    }
}

/// The 3x3 interval minor of a 4x4 interval matrix after deleting `row` and
/// `col` (the adjugate building block).
fn minor3_iv(m: &M4, row: usize, col: usize) -> M3 {
    let mut out = [[Interval::point(0.0); 3]; 3];
    let mut r_out = 0usize;
        #[allow(clippy::needless_range_loop)] // matrix indices over fixed 4-vectors; the index form is the algebra
    for r in 0..4 {
        if r == row {
            continue;
        }
        let mut c_out = 0usize;
        #[allow(clippy::needless_range_loop)] // matrix indices over fixed 4-vectors; the index form is the algebra
        for c in 0..4 {
            if c == col {
                continue;
            }
            out[r_out][c_out] = m[r][c];
            c_out += 1;
        }
        r_out += 1;
    }
    out
}

/// Determinant of a 4x4 interval matrix under directed rounding: the cofactor
/// expansion along row 0 over the engine's [`det3_iv`] (the exact 3x3 op
/// order), with the `(+ − + −)` sign pattern.
fn det4_iv(m: &M4) -> Interval {
    let mut acc = Interval::point(0.0);
    for c in 0..4 {
        let sign = if c % 2 == 0 { 1.0 } else { -1.0 };
        let term = Interval::point(sign)
            .mul(&m[0][c])
            .mul(&det3_iv(&minor3_iv(m, 0, c)));
        acc = acc.add(&term);
    }
    acc
}

/// The interval inverse of a 4x4 matrix via adjugate over determinant (the
/// `(row, col)` entry is the transposed cofactor `(−1)^{row+col}` of the minor
/// deleting `col`, `row`, divided by the determinant — the same adjugate
/// layout as [`inv3_iv`], at n = 4). `None` when the determinant enclosure
/// contains (or is) zero or a quotient is not finite.
fn inv4_iv(m: &M4) -> Option<M4> {
    let det = det4_iv(m);
    if !det.is_finite() || (det.lo <= 0.0 && det.hi >= 0.0) {
        return None;
    }
    let mut out = [[Interval::point(0.0); 4]; 4];
        #[allow(clippy::needless_range_loop)] // matrix indices over fixed 4-vectors; the index form is the algebra
    for r in 0..4 {
        #[allow(clippy::needless_range_loop)] // matrix indices over fixed 4-vectors; the index form is the algebra
        for c in 0..4 {
            let sign = if (r + c) % 2 == 0 { 1.0 } else { -1.0 };
            let cof = Interval::point(sign).mul(&det3_iv(&minor3_iv(m, c, r)));
            out[r][c] = cof.div(&det)?;
        }
    }
    Some(out)
}

/// Interval 4x4 matrix product.
fn matmul4_iv(a: &M4, b: &M4) -> M4 {
    let mut out = [[Interval::point(0.0); 4]; 4];
    for r in 0..4 {
        for c in 0..4 {
            let mut acc = Interval::point(0.0);
            for k in 0..4 {
                acc = acc.add(&a[r][k].mul(&b[k][c]));
            }
            out[r][c] = acc;
        }
    }
    out
}

/// Interval 4x4 matrix times 4-vector.
fn matvec4_iv(m: &M4, v: &Iv4) -> Iv4 {
    let mut out = [Interval::point(0.0); 4];
    for r in 0..4 {
        let mut acc = Interval::point(0.0);
        for k in 0..4 {
            acc = acc.add(&m[r][k].mul(&v[k]));
        }
        out[r] = acc;
    }
    out
}

/// The outward-rounded box `B − z_hat` (centred box), replicating the landed
/// 2D/3D reductions' op order at n = 4.
fn centred_dx4(b: &IBox4, z: &Iv4) -> Iv4 {
    let mut dx = [Interval::point(0.0); 4];
    for k in 0..4 {
        let d_lo = Interval::point(b.lo[k]).sub(&z[k]);
        let d_hi = Interval::point(b.hi[k]).sub(&z[k]);
        dx[k] = Interval {
            lo: d_lo.lo.min(d_hi.lo),
            hi: d_lo.hi.max(d_hi.hi),
        };
    }
    dx
}

/// Lemma 8.0's contraction rate `max_i (M r)_i / r_i` at n = 4. `None` when a
/// quotient is not finite.
fn rho4(id_minus: &M4, r: [f64; 4]) -> Option<f64> {
    let mut rho = 0.0f64;
    for i in 0..4 {
        let mr = mag(&id_minus[i][0]) * r[0]
            + mag(&id_minus[i][1]) * r[1]
            + mag(&id_minus[i][2]) * r[2]
            + mag(&id_minus[i][3]) * r[3];
        let ratio = mr / r[i];
        if !ratio.is_finite() {
            return None;
        }
        rho = rho.max(ratio);
    }
    Some(rho)
}

/// The arity-4 C1 entry (R3-class): identical operator discipline to
/// [`krawczyk_c1`] / [`krawczyk_c1_n3`] (Lemma 8.0 + §8.2), on the n=4
/// adjugate/det path, emitting a [`PointCert4`]. Weight bounds remain the §7.1
/// value argument.
///
/// The Disproven/Inconclusive backing table is IDENTICAL to
/// [`krawczyk_c1_n3`]'s: an empty `w` is `WeightDegenerate` (Disproven); a
/// non-positive/non-finite radius is `NonFinite` (Disproven); a disjoint image
/// is `ClaimRefuted` (Disproven); a merely overlapping image, a singular
/// midpoint Jacobian, a non-finite enclosure, or a `rho > RHO_MAX` is
/// Inconclusive. ResidualId stamping keeps the S2A convention: the engine
/// stamps [`ResidualId::R1`] and the caller rebuilds the certificate with its
/// own id through `PointCert4::try_new` (the documented one-line seam).
pub fn krawczyk_c1_n4(
    g: &dyn SquareResidualEval,
    b: IBox4,
    w: &[CertifiedPositive],
) -> ClaimVerdict<PointCert4, Refusal, Reason> {
    if g.arity() != 4 {
        return ClaimVerdict::Inconclusive("c1_n4_arity_mismatch_box_dimension");
    }
    if w.is_empty() {
        return ClaimVerdict::Disproven(engine_refusal(
            RefusalKind::WeightDegenerate,
            "c1_n4_weights_empty",
            "krawczyk_c1_n4 requires at least one certified positive weight bound (§7.1 value argument)"
                .to_string(),
        ));
    }
    let r = match radii4(&b) {
        Some(r) => r,
        None => {
            return ClaimVerdict::Disproven(engine_refusal(
                RefusalKind::NonFinite,
                "c1_n4_radius_nonpositive",
                "krawczyk_c1_n4 requires a strictly positive finite radius on every box axis"
                    .to_string(),
            ))
        }
    };
    let z = centre4(&b);
    let ziv: Iv4 = [
        Interval::point(z[0]),
        Interval::point(z[1]),
        Interval::point(z[2]),
        Interval::point(z[3]),
    ];
    let box_iv: Iv4 = [
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
        Interval {
            lo: b.lo[3],
            hi: b.hi[3],
        },
    ];

    let r0 = g.eval(&ziv);
    if r0.len() != 4 {
        return ClaimVerdict::Inconclusive("c1_n4_eval_arity_mismatch");
    }
    let j0_rows = g.jac_encl(&ziv);
    if j0_rows.len() != 4 || j0_rows.iter().any(|row| row.len() != 4) {
        return ClaimVerdict::Inconclusive("c1_n4_jac_arity_mismatch");
    }

    let j0: M4 = [
        [j0_rows[0][0], j0_rows[0][1], j0_rows[0][2], j0_rows[0][3]],
        [j0_rows[1][0], j0_rows[1][1], j0_rows[1][2], j0_rows[1][3]],
        [j0_rows[2][0], j0_rows[2][1], j0_rows[2][2], j0_rows[2][3]],
        [j0_rows[3][0], j0_rows[3][1], j0_rows[3][2], j0_rows[3][3]],
    ];

    // A = the interval inverse of the midpoint (centre) Jacobian. A singular
    // midpoint returns Inconclusive BEFORE the box Jacobian is enclosed —
    // result-identical to enclosing it first, and it skips the expensive box
    // enclosure on every cell whose midpoint Jacobian is singular (the
    // positive-dimensional cells dominate the Tier-2 stall searches).
    let a = match inv4_iv(&j0) {
        Some(a) => a,
        None => return ClaimVerdict::Inconclusive("c1_n4_midpoint_jacobian_singular"),
    };

    let jb_rows = g.jac_encl(&box_iv);
    if jb_rows.len() != 4 || jb_rows.iter().any(|row| row.len() != 4) {
        return ClaimVerdict::Inconclusive("c1_n4_jac_arity_mismatch");
    }
    let jb: M4 = [
        [jb_rows[0][0], jb_rows[0][1], jb_rows[0][2], jb_rows[0][3]],
        [jb_rows[1][0], jb_rows[1][1], jb_rows[1][2], jb_rows[1][3]],
        [jb_rows[2][0], jb_rows[2][1], jb_rows[2][2], jb_rows[2][3]],
        [jb_rows[3][0], jb_rows[3][1], jb_rows[3][2], jb_rows[3][3]],
    ];

    // (I − A·□DR(B)) and the Krawczyk image K(B).
    let cj = matmul4_iv(&a, &jb);
    let id_minus: M4 = [
        [
            Interval::point(1.0).sub(&cj[0][0]),
            cj[0][1].neg(),
            cj[0][2].neg(),
            cj[0][3].neg(),
        ],
        [
            cj[1][0].neg(),
            Interval::point(1.0).sub(&cj[1][1]),
            cj[1][2].neg(),
            cj[1][3].neg(),
        ],
        [
            cj[2][0].neg(),
            cj[2][1].neg(),
            Interval::point(1.0).sub(&cj[2][2]),
            cj[2][3].neg(),
        ],
        [
            cj[3][0].neg(),
            cj[3][1].neg(),
            cj[3][2].neg(),
            Interval::point(1.0).sub(&cj[3][3]),
        ],
    ];
    if id_minus.iter().flatten().any(|v| !v.is_finite()) {
        return ClaimVerdict::Inconclusive("c1_n4_enclosure_not_finite");
    }
    let dx = centred_dx4(&b, &ziv);
    let r0v: Iv4 = [r0[0], r0[1], r0[2], r0[3]];
    let ch = matvec4_iv(&a, &r0v);
    let md = matvec4_iv(&id_minus, &dx);
    let k: Iv4 = [
        ziv[0].sub(&ch[0]).add(&md[0]),
        ziv[1].sub(&ch[1]).add(&md[1]),
        ziv[2].sub(&ch[2]).add(&md[2]),
        ziv[3].sub(&ch[3]).add(&md[3]),
    ];
    if k.iter().any(|v| !v.is_finite()) {
        return ClaimVerdict::Inconclusive("c1_n4_enclosure_not_finite");
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
                "c1_n4_k_disjoint_no_root_in_box",
                "the Krawczyk image is disjoint from the box: no root of the residual in the box"
                    .to_string(),
            ));
        }
        return ClaimVerdict::Inconclusive("c1_n4_inclusion_not_strict");
    }

    // Lemma 8.0's contraction rate.
    let rho = match rho4(&id_minus, r) {
        Some(rho) => rho,
        None => return ClaimVerdict::Inconclusive("c1_n4_rho_not_finite"),
    };
    if rho > RHO_MAX {
        return ClaimVerdict::Inconclusive("c1_n4_rho_exceeds_rho_max");
    }
    // See the module-doc seam judgement: the engine stamps R1.
    match PointCert4::try_new(ResidualId::R1, b, rho) {
        Ok(cert) => ClaimVerdict::Proven(cert),
        Err(refusal) => ClaimVerdict::Disproven(refusal),
    }
}
