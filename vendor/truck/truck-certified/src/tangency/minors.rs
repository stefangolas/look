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

//! The chart-minor polynomial substrate (CTE-003-MINORS; theory §2.3).
//!
//! The chart minors `M_j = det D_(y1,y2,z_j)(G1, G2, f)` are POLYNOMIALS in the
//! four chart coordinates, expressible as composed tensor-Bernstein grids. The
//! landed `kernel/minor_algebra.rs` only produces per-box interval minor
//! enclosures; this module composes the coefficient grids themselves.
//!
//! # The polynomial grid algebra
//!
//! [`PolyGrid`] is the CTE substrate's own four-axis tensor-Bernstein
//! coefficient grid (the pattern of the SSI `Tensor4` / kernel `Grid4`,
//! adapted — never imported, the V5 guard). It stores the coefficient rows in
//! the *landed* flat layout — `row = a·(n1+1)+b`, `column = i·(n2+1)+j` over
//! the per-axis degrees `(m1, n1, m2, n2)` — so a chart-minor net carries
//! exactly the table the landed de-Casteljau kernels consume
//! ([`PolyGrid::to_net`] flattens row-major, which is the
//! [`ChartMinorNet`](crate::tangency::shapes::ChartMinorNet) layout
//! convention). A `PolyGrid` is the polynomial represented by its stored
//! float coefficients over the unit chart `[0,1]^4` of the stored
//! [`SquareSystem3`]; every derivative is the Bernstein coefficient rule
//! `d·(c[k+1] − c[k])`, every product the Bernstein convolution, every
//! enclosure the de-Casteljau-over-[`CertifiedInterval`] hull.
//!
//! # Coordinate convention
//!
//! Chart minors and the deflated system built on them (CTE-003-MINORS) are
//! expressed over the UNIT chart of the stored system: the four product axes
//! of the `(u, v, s, t)` cube the stored grids are Bernstein in. All boxes
//! consumed by this substrate ([`PolyGrid::hull_unit`],
//! [`tsystem::TSystem`](crate::tangency::tsystem::TSystem),
//! [`exclude::exclude_five`](crate::tangency::exclude::exclude_five)) are
//! unit-chart boxes. The SSI square systems this program produces all carry
//! the identity chart maps, so this coincides with the chart-coordinate
//! convention of `kernel/engine.rs`.
//!
//! # Refusals
//!
//! Every fallible operation returns [`TangencyRefusal`], wrapping the landed
//! named causes verbatim (scope decision 6).

use crate::contract::Refusal;
use crate::formal::exact::CertifiedInterval;
use crate::hull::HullRefusal;
use crate::tangency::shapes::{ChartMinorGrids, ChartMinorNet, Rank2Chart};
use crate::tangency::TangencyRefusal;
use crate::SquareSystem3;

// ---------------------------------------------------------------------------
// The four-axis polynomial grid
// ---------------------------------------------------------------------------

/// A four-axis tensor-Bernstein coefficient grid in the landed flat layout.
///
/// `degrees = (m1, n1, m2, n2)`; `rows` holds `(m1+1)·(n1+1)` rows of
/// `(m2+1)·(n2+1)` coefficients, coefficient of
/// `B^{m1}_a(u) B^{n1}_b(v) B^{m2}_i(s) B^{n2}_j(t)` at
/// `rows[a·(n1+1)+b][i·(n2+1)+j]`.
#[derive(Clone, Debug)]
pub(crate) struct PolyGrid {
    degrees: (usize, usize, usize, usize),
    rows: Vec<Vec<f64>>,
}

impl PolyGrid {
    /// Row spacing of the flat layout (`n1 + 1`).
    fn row_spacing(&self) -> usize {
        self.degrees.1 + 1
    }

    /// Column spacing of the flat layout (`n2 + 1`).
    fn col_spacing(&self) -> usize {
        self.degrees.3 + 1
    }

    /// Whether a stored table has the shape the degrees demand.
    fn shape_ok(&self) -> bool {
        let (m1, n1, m2, n2) = self.degrees;
        let rows = (m1 + 1) * (n1 + 1);
        let cols = (m2 + 1) * (n2 + 1);
        self.rows.len() == rows
            && self
                .rows
                .iter()
                .all(|row| row.len() == cols && row.iter().all(|c| c.is_finite()))
    }

    /// The first-partial coefficient grid along one chart axis: the Bernstein
    /// derivative `d·(c[k+1] − c[k])` of degree `d − 1`.
    pub(crate) fn partial_axis(&self, axis: usize) -> Result<PolyGrid, TangencyRefusal> {
        let (m1, n1, m2, n2) = self.degrees;
        let base = [m1, n1, m2, n2][axis];
        if base == 0 {
            return Err(TangencyRefusal::Input(Refusal::InvalidInput));
        }
        let scale = base as f64;
        let degrees = match axis {
            0 => (m1 - 1, n1, m2, n2),
            1 => (m1, n1 - 1, m2, n2),
            2 => (m1, n1, m2 - 1, n2),
            _ => (m1, n1, m2, n2 - 1),
        };
        let (nm1, nn1, nm2, nn2) = degrees;
        let nrows = (nm1 + 1) * (nn1 + 1);
        let ncols = (nm2 + 1) * (nn2 + 1);
        let sp1 = self.row_spacing();
        let sp2 = self.col_spacing();
        let mut out = vec![vec![0.0f64; ncols]; nrows];
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
        Ok(PolyGrid { degrees, rows: out })
    }

    /// The certified range enclosure over a UNIT-chart box (each axis a
    /// compact subinterval of `[0, 1]`).
    pub(crate) fn hull_unit(
        &self,
        box_: [(f64, f64); 4],
    ) -> Result<CertifiedInterval, TangencyRefusal> {
        for (lo, hi) in box_ {
            if !lo.is_finite() || !hi.is_finite() || !(lo >= 0.0 && hi <= 1.0 && lo <= hi) {
                return Err(TangencyRefusal::Hull(HullRefusal::DomainNotCompact));
            }
        }
        if !self.shape_ok() {
            return Err(TangencyRefusal::Hull(HullRefusal::EnclosureUnavailable));
        }
        hull_tensor(&self.rows, self.degrees, box_)
    }

    /// The plain float evaluation of the polynomial at a unit-chart point
    /// (de Casteljau in `f64`). Used for the deflated system's POINT residual
    /// (the Krawczyk centre term) and its float preconditioner, never for a
    /// certificate.
    pub(crate) fn float_value(&self, u: [f64; 4]) -> Result<f64, TangencyRefusal> {
        if !self.shape_ok() {
            return Err(TangencyRefusal::Hull(HullRefusal::EnclosureUnavailable));
        }
        let (m1, n1, m2, n2) = self.degrees;
        // Reduce the (s, t) columns of every flat row to one value per row.
        let mut per_row = Vec::with_capacity(self.rows.len());
        for row in self.rows.iter() {
            // Rebuild the (m2+1) x (n2+1) coefficient grid of this row.
            let mut grid2 = Vec::with_capacity(m2 + 1);
            for chunk in row.chunks(n2 + 1) {
                grid2.push(chunk.to_vec());
            }
            per_row.push(eval_2d_float(&grid2, u[2], u[3])?);
        }
        // Rebuild the (m1+1) x (n1+1) coefficient grid over the flat rows.
        let mut grid1 = Vec::with_capacity(m1 + 1);
        for a in 0..=m1 {
            let mut inner = Vec::with_capacity(n1 + 1);
            for b in 0..=n1 {
                inner.push(per_row[a * (n1 + 1) + b]);
            }
            grid1.push(inner);
        }
        eval_2d_float(&grid1, u[0], u[1])
    }
}

/// Wrap one stored `SquareSystem3` component grid verbatim (shape validated by
/// the shim's constructor; re-checked defensively).
pub(crate) fn from_system_grid(
    grid: &[Vec<f64>],
    degrees: (usize, usize, usize, usize),
) -> Result<PolyGrid, TangencyRefusal> {
    let poly = PolyGrid {
        degrees,
        rows: grid.to_vec(),
    };
    if poly.shape_ok() {
        Ok(poly)
    } else {
        Err(TangencyRefusal::Input(Refusal::InvalidInput))
    }
}

/// Rebuild a polynomial grid from a frozen chart-minor net (degrees + the flat
/// row-major coefficient table this module emits).
pub(crate) fn from_net(net: &ChartMinorNet) -> Result<PolyGrid, TangencyRefusal> {
    let d = net.degrees();
    let degrees = (d[0], d[1], d[2], d[3]);
    let coeffs = net.coeffs();
    let (m1, n1, m2, n2) = degrees;
    let rows = (m1 + 1) * (n1 + 1);
    let cols = (m2 + 1) * (n2 + 1);
    if coeffs.len() != rows * cols {
        return Err(TangencyRefusal::Input(Refusal::InvalidInput));
    }
    let mut out = Vec::with_capacity(rows);
    for r in 0..rows {
        let lo = r * cols;
        out.push(coeffs[lo..lo + cols].to_vec());
    }
    let poly = PolyGrid { degrees, rows: out };
    if poly.shape_ok() {
        Ok(poly)
    } else {
        Err(TangencyRefusal::Input(Refusal::InvalidInput))
    }
}

impl PolyGrid {
    /// Flatten the stored rows into the frozen [`ChartMinorNet`] carrier
    /// (degrees as `[usize; 4]`, coefficients row-major over rows then
    /// columns).
    pub(crate) fn to_net(&self) -> Result<ChartMinorNet, TangencyRefusal> {
        if !self.shape_ok() {
            return Err(TangencyRefusal::Hull(HullRefusal::EnclosureUnavailable));
        }
        let degrees = [
            self.degrees.0,
            self.degrees.1,
            self.degrees.2,
            self.degrees.3,
        ];
        let mut coeffs = Vec::with_capacity(self.rows.len() * self.rows[0].len());
        for row in self.rows.iter() {
            coeffs.extend_from_slice(row);
        }
        ChartMinorNet::new(degrees, coeffs)
            .map_err(|_| TangencyRefusal::Input(Refusal::InvalidInput))
    }
}

/// Whether two grids carry the same degrees and shape.
fn same_shape(a: &PolyGrid, b: &PolyGrid) -> bool {
    a.degrees == b.degrees && a.rows.len() == b.rows.len()
}

/// Coefficient-wise sum of two same-degree grids.
fn grid_add(a: &PolyGrid, b: &PolyGrid) -> Result<PolyGrid, TangencyRefusal> {
    if !same_shape(a, b) {
        return Err(TangencyRefusal::Input(Refusal::InvalidInput));
    }
    let mut rows = Vec::with_capacity(a.rows.len());
    for (ra, rb) in a.rows.iter().zip(b.rows.iter()) {
        rows.push(ra.iter().zip(rb.iter()).map(|(x, y)| x + y).collect());
    }
    Ok(PolyGrid {
        degrees: a.degrees,
        rows,
    })
}

/// Coefficient-wise difference of two same-degree grids.
fn grid_sub(a: &PolyGrid, b: &PolyGrid) -> Result<PolyGrid, TangencyRefusal> {
    if !same_shape(a, b) {
        return Err(TangencyRefusal::Input(Refusal::InvalidInput));
    }
    let mut rows = Vec::with_capacity(a.rows.len());
    for (ra, rb) in a.rows.iter().zip(b.rows.iter()) {
        rows.push(ra.iter().zip(rb.iter()).map(|(x, y)| x - y).collect());
    }
    Ok(PolyGrid {
        degrees: a.degrees,
        rows,
    })
}

/// The Bernstein product of two grids over the shared unit chart: per-axis
/// degree sums with the Bernstein convolution weights
/// `C(da,i)·C(db,j)/C(da+db,k)` on each axis.
fn grid_mul(a: &PolyGrid, b: &PolyGrid) -> Result<PolyGrid, TangencyRefusal> {
    let (m1, n1, m2, n2) = a.degrees;
    let (p1, q1, p2, q2) = b.degrees;
    let out = (m1 + p1, n1 + q1, m2 + p2, n2 + q2);
    let (om1, on1, om2, on2) = out;
    let orows = (om1 + 1) * (on1 + 1);
    let ocols = (om2 + 1) * (on2 + 1);
    let sp1 = n1 + 1;
    let sp2 = n2 + 1;
    let sp1b = q1 + 1;
    let sp2b = q2 + 1;
    let mut rows = vec![vec![0.0f64; ocols]; orows];
    for ka in 0..=om1 {
        for kb in 0..=on1 {
            for ki in 0..=om2 {
                for kj in 0..=on2 {
                    let mut acc = 0.0f64;
                    let a0_hi = m1.min(ka);
                    for a0 in 0..=a0_hi {
                        let a1 = ka - a0;
                        if a1 > p1 {
                            continue;
                        }
                        let w0 = axis_weight(m1, p1, a0, a1, ka);
                        let b0_hi = n1.min(kb);
                        for b0 in 0..=b0_hi {
                            let b1 = kb - b0;
                            if b1 > q1 {
                                continue;
                            }
                            let w1 = axis_weight(n1, q1, b0, b1, kb);
                            let i0_hi = m2.min(ki);
                            for i0 in 0..=i0_hi {
                                let i1 = ki - i0;
                                if i1 > p2 {
                                    continue;
                                }
                                let w2 = axis_weight(m2, p2, i0, i1, ki);
                                let j0_hi = n2.min(kj);
                                for j0 in 0..=j0_hi {
                                    let j1 = kj - j0;
                                    if j1 > q2 {
                                        continue;
                                    }
                                    let w3 = axis_weight(n2, q2, j0, j1, kj);
                                    let av = a.rows[a0 * sp1 + b0][i0 * sp2 + j0];
                                    let bv = b.rows[a1 * sp1b + b1][i1 * sp2b + j1];
                                    acc += w0 * w1 * w2 * w3 * av * bv;
                                }
                            }
                        }
                    }
                    let dst_row = ka * (on1 + 1) + kb;
                    let dst_col = ki * (on2 + 1) + kj;
                    rows[dst_row][dst_col] = acc;
                }
            }
        }
    }
    Ok(PolyGrid { degrees: out, rows })
}

/// The Bernstein product weight of one axis: `C(da,i)·C(db,j)/C(da+db,k)`
/// with `i + j = k`.
fn axis_weight(da: usize, db: usize, i: usize, j: usize, k: usize) -> f64 {
    binom(da, i) * binom(db, j) / binom(da + db, k)
}

/// A small integer binomial coefficient as `f64`.
fn binom(n: usize, k: usize) -> f64 {
    if k > n {
        return 0.0;
    }
    let mut num = 1.0f64;
    let mut den = 1.0f64;
    for t in 0..k {
        num *= (n - t) as f64;
        den *= (t + 1) as f64;
    }
    num / den
}

// ---------------------------------------------------------------------------
// Certified hulling over the flat layout (the landed de-Casteljau discipline)
// ---------------------------------------------------------------------------

/// Interval de Casteljau over one axis for a 1-D interval coefficient list.
fn one_d_interval(
    pts: &[CertifiedInterval],
    u: &CertifiedInterval,
) -> Result<CertifiedInterval, TangencyRefusal> {
    if pts.is_empty() {
        return Err(TangencyRefusal::Hull(HullRefusal::EnclosureUnavailable));
    }
    let mut level = pts.to_vec();
    while level.len() > 1 {
        let mut next = Vec::with_capacity(level.len() - 1);
        for w in level.windows(2) {
            next.push(w[0].add(&w[1].sub(&w[0]).mul(u)));
        }
        level = next;
    }
    if level[0].is_finite() {
        Ok(level[0])
    } else {
        Err(TangencyRefusal::Hull(HullRefusal::EnclosureUnavailable))
    }
}

/// Certified range enclosure of a stored rows/cols grid over a unit-chart box
/// (axis-by-axis interval de Casteljau, outward-rounded).
fn hull_tensor(
    rows: &[Vec<f64>],
    degrees: (usize, usize, usize, usize),
    box_: [(f64, f64); 4],
) -> Result<CertifiedInterval, TangencyRefusal> {
    let (m1, n1, m2, n2) = degrees;
    let sp1 = n1 + 1;
    let sp2 = n2 + 1;
    let n1p1 = n1 + 1;
    let cols = (m2 + 1) * (n2 + 1);
    let u_iv = CertifiedInterval {
        lo: box_[0].0,
        hi: box_[0].1,
    };
    let u_len = m1 + 1;
    let mut u_cols = vec![Vec::<CertifiedInterval>::with_capacity(n1p1); cols];
    for b in 0..n1p1 {
        for (c, slot) in u_cols.iter_mut().enumerate() {
            let mut pts = Vec::with_capacity(u_len);
            for a in 0..u_len {
                pts.push(CertifiedInterval::point(rows[a * sp1 + b][c]));
            }
            slot.push(one_d_interval(&pts, &u_iv)?);
        }
    }
    let v_iv = CertifiedInterval {
        lo: box_[1].0,
        hi: box_[1].1,
    };
    let mut v_collapsed = Vec::with_capacity(cols);
    for col in u_cols {
        v_collapsed.push(one_d_interval(&col, &v_iv)?);
    }
    let mut grid2: Vec<Vec<CertifiedInterval>> = Vec::with_capacity(v_collapsed.len() / sp2);
    for row_slice in v_collapsed.chunks(sp2) {
        grid2.push(row_slice.to_vec());
    }
    hull_2d_interval(&grid2, box_[2], box_[3])
}

/// Interval de Casteljau over the `(s, t)` box of an interval-valued bivariate
/// tensor grid.
fn hull_2d_interval(
    grid: &[Vec<CertifiedInterval>],
    s: (f64, f64),
    t: (f64, f64),
) -> Result<CertifiedInterval, TangencyRefusal> {
    if grid.is_empty() || grid[0].is_empty() {
        return Err(TangencyRefusal::Hull(HullRefusal::EnclosureUnavailable));
    }
    let width = grid[0].len();
    if grid.iter().any(|row| row.len() != width) {
        return Err(TangencyRefusal::Hull(HullRefusal::EnclosureUnavailable));
    }
    let s_iv = CertifiedInterval { lo: s.0, hi: s.1 };
    let t_iv = CertifiedInterval { lo: t.0, hi: t.1 };
    let mut col_evals = Vec::with_capacity(width);
    for j in 0..width {
        let col: Vec<CertifiedInterval> = grid.iter().map(|row| row[j]).collect();
        col_evals.push(one_d_interval(&col, &s_iv)?);
    }
    let hull = one_d_interval(&col_evals, &t_iv)?;
    if hull.is_finite() {
        Ok(hull)
    } else {
        Err(TangencyRefusal::Hull(HullRefusal::EnclosureUnavailable))
    }
}

/// Float 1-D de Casteljau over a coefficient list.
fn bern_eval_1d(coeffs: &[f64], x: f64) -> f64 {
    let mut level = coeffs.to_vec();
    while level.len() > 1 {
        let mut next = Vec::with_capacity(level.len() - 1);
        for pair in level.windows(2) {
            next.push(pair[0] + x * (pair[1] - pair[0]));
        }
        level = next;
    }
    level[0]
}

/// Float evaluation of a bivariate Bernstein grid (`rows` index the first
/// parameter).
fn eval_2d_float(grid: &[Vec<f64>], p: f64, q: f64) -> Result<f64, TangencyRefusal> {
    if grid.is_empty() || grid[0].is_empty() {
        return Err(TangencyRefusal::Hull(HullRefusal::EnclosureUnavailable));
    }
    let mut first = Vec::with_capacity(grid.len());
    for row in grid.iter() {
        first.push(bern_eval_1d(row, q));
    }
    let out = bern_eval_1d(&first, p);
    if out.is_finite() {
        Ok(out)
    } else {
        Err(TangencyRefusal::Hull(HullRefusal::EnclosureUnavailable))
    }
}

// ---------------------------------------------------------------------------
// Chart-minor construction
// ---------------------------------------------------------------------------

/// The 3 x 3 determinant-of-partials of one chart minor as a composed grid.
///
/// `comps` names the three F components in row order `(G1, G2, f)`; `axes`
/// names the three column axes `(y1, y2, z_j)` in order. The determinant is
/// expanded over grids (`E00·(E11·E22 − E12·E21) − E01·(E10·E22 − E12·E20) +
/// E02·(E10·E21 − E11·E20)`), every product a Bernstein convolution and every
/// sum same-degree (each term reduces exactly the three distinct column axes).
fn minor_grid(
    system: &SquareSystem3,
    comps: [usize; 3],
    axes: [usize; 3],
) -> Result<PolyGrid, TangencyRefusal> {
    let mut e = Vec::with_capacity(3);
    for &comp in comps.iter() {
        let mut row = Vec::with_capacity(3);
        for &axis in axes.iter() {
            let base = from_system_grid(&system.grids()[comp], system.degrees())?;
            row.push(base.partial_axis(axis)?);
        }
        e.push(row);
    }
    let e11 = grid_mul(&e[1][1], &e[2][2])?;
    let e12 = grid_mul(&e[1][2], &e[2][1])?;
    let inner0 = grid_sub(&e11, &e12)?;
    let term0 = grid_mul(&e[0][0], &inner0)?;
    let e10 = grid_mul(&e[1][0], &e[2][2])?;
    let e12b = grid_mul(&e[1][2], &e[2][0])?;
    let inner1 = grid_sub(&e10, &e12b)?;
    let term1 = grid_mul(&e[0][1], &inner1)?;
    let e10b = grid_mul(&e[1][0], &e[2][1])?;
    let e11b = grid_mul(&e[1][1], &e[2][0])?;
    let inner2 = grid_sub(&e10b, &e11b)?;
    let term2 = grid_mul(&e[0][2], &inner2)?;
    let s0 = grid_sub(&term0, &term1)?;
    grid_add(&s0, &term2)
}

/// Build the two chart-minor polynomial grids `(M_1, M_2)` of a chart (theory
/// T1.1) from the stored square system and the chart's pivot.
///
/// `M_j = det D_(y1,y2,z_j)(G1, G2, f)` is composed polynomially from the
/// first-partial grids of the two `G` components and the distinguished `f`
/// component along the chart's `y` pair and the `j`-th `z` axis. The returned
/// grids are certificate carriers; nothing here hulls over a box.
pub fn build_chart_minors(
    system: &SquareSystem3,
    chart: &Rank2Chart,
) -> Result<ChartMinorGrids, TangencyRefusal> {
    let pivot = chart.pivot();
    let f = pivot.component();
    let (y0, y1) = pivot.coord_pair();
    let gs: Vec<usize> = (0..3).filter(|&c| c != f).collect();
    let zs: Vec<usize> = (0..4).filter(|&a| a != y0 && a != y1).collect();
    let m1 = minor_grid(system, [gs[0], gs[1], f], [y0, y1, zs[0]])?;
    let m2 = minor_grid(system, [gs[0], gs[1], f], [y0, y1, zs[1]])?;
    let net1 = m1.to_net()?;
    let net2 = m2.to_net()?;
    ChartMinorGrids::new(net1, net2).map_err(|_| TangencyRefusal::Input(Refusal::InvalidInput))
}

#[cfg(test)]
pub(crate) mod support {
    //! Test support for the CTE-003 wave modules (F1/F6 square systems over
    //! the unit chart, monomial grid builders, template charts). TEST ONLY.

    use super::*;
    use crate::formal::exact::CertifiedInterval;
    use crate::ssi::{construct_square_system, RationalBipatch, SsiParticipant};
    use crate::ssi_types::SquareSystem3;
    use crate::tangency::shapes::{ChartMinorGrids, Rank2Chart, Rank2Pivot};

    /// The identity chart map every synthetic system uses.
    pub(crate) const IDENTITY_MAPS: (f64, f64, f64, f64, f64, f64, f64, f64) =
        (0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0);

    /// A bivariate Bernstein grid of bidegree `(m, n)` built from monomial
    /// terms `(pu, pv, coeff)`.
    fn monomial_grid2(m: usize, n: usize, terms: &[(usize, usize, f64)]) -> Vec<Vec<f64>> {
        let mut grid = vec![vec![0.0f64; n + 1]; m + 1];
        for &(pu, pv, coeff) in terms {
            let row_factors: Vec<f64> = (pu..=m).map(|a| binom(a, pu) / binom(m, pu)).collect();
            let col_factors: Vec<f64> = (pv..=n).map(|b| binom(b, pv) / binom(n, pv)).collect();
            for (a, fa) in (pu..=m).zip(row_factors.iter()) {
                for (b, fb) in (pv..=n).zip(col_factors.iter()) {
                    grid[a][b] += coeff * fa * fb;
                }
            }
        }
        grid
    }

    /// A graph-patch coordinate grid (the parameter coordinate elevated to the
    /// bidegree `(m, n)`).
    fn coord_grid(m: usize, n: usize, which: usize) -> Vec<Vec<f64>> {
        let mut grid = Vec::with_capacity(m + 1);
        for a in 0..=m {
            let mut row = Vec::with_capacity(n + 1);
            for b in 0..=n {
                row.push(if which == 0 {
                    a as f64 / m as f64
                } else {
                    b as f64 / n as f64
                });
            }
            grid.push(row);
        }
        grid
    }

    /// The graph patch `(x, y, z) = (u, v, h(u, v))` with unit weights.
    fn graph_patch(
        m: usize,
        n: usize,
        h: Vec<Vec<f64>>,
    ) -> Result<RationalBipatch, TangencyRefusal> {
        let ones = vec![vec![1.0f64; n + 1]; m + 1];
        RationalBipatch::new(m, n, [coord_grid(m, n, 0), coord_grid(m, n, 1), h], ones)
            .map_err(|_| TangencyRefusal::Input(Refusal::InvalidInput))
    }

    /// A four-axis Bernstein grid from monomial terms over the identity chart:
    /// `(exponents [usize;4], coeff)` with exponent `e` meaning the `e`-th
    /// coordinate raised to that power. Returns the stored rows layout for the
    /// given degrees.
    pub(crate) fn monomial_grid4(
        degrees: (usize, usize, usize, usize),
        terms: &[([usize; 4], f64)],
    ) -> Vec<Vec<f64>> {
        let (m1, n1, m2, n2) = degrees;
        let rows = (m1 + 1) * (n1 + 1);
        let cols = (m2 + 1) * (n2 + 1);
        let mut grid = vec![vec![0.0f64; cols]; rows];
        for &(exp, coeff) in terms {
            let ax0: Vec<f64> = (exp[0]..=m1)
                .map(|a| binom(a, exp[0]) / binom(m1, exp[0]))
                .collect();
            let ax1: Vec<f64> = (exp[1]..=n1)
                .map(|b| binom(b, exp[1]) / binom(n1, exp[1]))
                .collect();
            let ax2: Vec<f64> = (exp[2]..=m2)
                .map(|i| binom(i, exp[2]) / binom(m2, exp[2]))
                .collect();
            let ax3: Vec<f64> = (exp[3]..=n2)
                .map(|j| binom(j, exp[3]) / binom(n2, exp[3]))
                .collect();
            for a in exp[0]..=m1 {
                for b in exp[1]..=n1 {
                    for i in exp[2]..=m2 {
                        for j in exp[3]..=n2 {
                            grid[a * (n1 + 1) + b][i * (n2 + 1) + j] += coeff
                                * ax0[a - exp[0]]
                                * ax1[b - exp[1]]
                                * ax2[i - exp[2]]
                                * ax3[j - exp[3]];
                        }
                    }
                }
            }
        }
        grid
    }

    /// A `SquareSystem3` from three four-axis monomial component grids.
    pub(crate) fn system_from_monomials(
        degrees: (usize, usize, usize, usize),
        comps: [&[([usize; 4], f64)]; 3],
    ) -> Result<SquareSystem3, TangencyRefusal> {
        let grids = [
            monomial_grid4(degrees, comps[0]),
            monomial_grid4(degrees, comps[1]),
            monomial_grid4(degrees, comps[2]),
        ];
        SquareSystem3::new(grids, degrees, IDENTITY_MAPS)
            .map_err(|_| TangencyRefusal::Input(Refusal::InvalidInput))
    }

    /// The F1 (T1.4 identity) instance as a square system over the unit chart:
    /// components `(G1, G2, f) = (y1, y2, 3·z1² − 5·z2²)` over the chart axes
    /// `(0, 1, 2, 3) = (y1, y2, z1, z2)`, degrees `(1, 1, 2, 2)`.
    pub(crate) fn f1_system() -> Result<SquareSystem3, TangencyRefusal> {
        system_from_monomials(
            (1, 1, 2, 2),
            [
                &[([1, 0, 0, 0], 1.0)],
                &[([0, 1, 0, 0], 1.0)],
                &[([0, 0, 2, 0], 3.0), ([0, 0, 0, 2], -5.0)],
            ],
        )
    }

    /// The F1 chart pivot: `f` is component 2, `y = (0, 1)` (so `A = D_y G` is
    /// the identity and `det A = 1`).
    pub(crate) fn f1_pivot() -> Result<Rank2Pivot, TangencyRefusal> {
        Rank2Pivot::new(2, (0, 1)).map_err(|_| TangencyRefusal::Input(Refusal::InvalidInput))
    }

    /// A template chart carrying a pivot and a `det_a` that excludes zero; the
    /// minors are placeholder nets (builders ignore `chart.minors()` and read
    /// only the pivot).
    pub(crate) fn template_chart(
        pivot: Rank2Pivot,
        det_lo: f64,
        det_hi: f64,
    ) -> Result<Rank2Chart, TangencyRefusal> {
        let placeholder = ChartMinorNet::new([1, 1, 1, 1], vec![1.0; 16])
            .map_err(|_| TangencyRefusal::Input(Refusal::InvalidInput))?;
        let minors = ChartMinorGrids::new(placeholder.clone(), placeholder)
            .map_err(|_| TangencyRefusal::Input(Refusal::InvalidInput))?;
        Rank2Chart::new(
            pivot,
            CertifiedInterval {
                lo: det_lo,
                hi: det_hi,
            },
            minors,
        )
        .map_err(|_| TangencyRefusal::Input(Refusal::InvalidInput))
    }

    /// The F6 (transversal crossing-planes) instance as a real square system:
    /// patch 1 is the plane `z = 0` and patch 2 the plane `z = x − 1/2`, so the
    /// intersection line sits strictly inside the unit charts. The F components
    /// are `F = (u − s, v − t, 1/2 − s)`.
    pub(crate) fn f6_system() -> Result<SquareSystem3, TangencyRefusal> {
        let plane = graph_patch(1, 1, vec![vec![0.0f64; 2]; 2])?;
        let shifted = {
            let h = monomial_grid2(1, 1, &[(0, 0, -0.5), (1, 0, 1.0)]);
            graph_patch(1, 1, h)?
        };
        construct_square_system(
            &SsiParticipant::RationalBipatch(plane),
            &SsiParticipant::RationalBipatch(shifted),
        )
        .map_err(|_| TangencyRefusal::Input(Refusal::InvalidInput))
    }
}

#[cfg(test)]
mod tests {
    /// A sparse exact polynomial over the four chart variables with `i64`
    /// coefficients (test-side rational arithmetic).
    #[derive(Clone, Debug, PartialEq, Eq)]
    struct QPoly(Vec<(i64, [u8; 4])>);

    impl QPoly {
        fn constant(c: i64) -> QPoly {
            if c == 0 {
                QPoly(Vec::new())
            } else {
                QPoly(vec![(c, [0, 0, 0, 0])])
            }
        }

        fn var(axis: usize) -> QPoly {
            let mut e = [0u8; 4];
            e[axis] = 1;
            QPoly(vec![(1, e)])
        }

        fn pow2(axis: usize) -> QPoly {
            let mut e = [0u8; 4];
            e[axis] = 2;
            QPoly(vec![(1, e)])
        }

        fn scale(&self, s: i64) -> QPoly {
            QPoly(
                self.0
                    .iter()
                    .map(|&(c, e)| (c * s, e))
                    .filter(|&(c, _)| c != 0)
                    .collect(),
            )
        }

        fn add(&self, o: &QPoly) -> QPoly {
            let mut out = self.0.clone();
            for &(c, e) in o.0.iter() {
                if let Some(slot) = out.iter_mut().find(|(_, ex)| *ex == e) {
                    slot.0 += c;
                } else if c != 0 {
                    out.push((c, e));
                }
            }
            out.retain(|&(c, _)| c != 0);
            QPoly(out)
        }

        fn neg(&self) -> QPoly {
            self.scale(-1)
        }

        fn sub(&self, o: &QPoly) -> QPoly {
            self.add(&o.neg())
        }

        fn mul(&self, o: &QPoly) -> QPoly {
            let mut acc: Vec<(i64, [u8; 4])> = Vec::new();
            for &(ca, ea) in self.0.iter() {
                for &(cb, eb) in o.0.iter() {
                    let mut e = [0u8; 4];
                    for k in 0..4 {
                        e[k] = ea[k] + eb[k];
                    }
                    acc.push((ca * cb, e));
                }
            }
            let mut out = QPoly(Vec::new());
            for (c, e) in acc {
                out = out.add(&QPoly(vec![(c, e)]));
            }
            out
        }

        fn partial(&self, axis: usize) -> QPoly {
            let mut q = QPoly(Vec::new());
            for &(c, e) in self.0.iter() {
                if e[axis] == 0 {
                    continue;
                }
                let p = e[axis] as i64;
                let mut ne = e;
                ne[axis] -= 1;
                q = q.add(&QPoly(vec![(c * p, ne)]));
            }
            q
        }

        fn is_zero(&self) -> bool {
            self.0.is_empty()
        }
    }

    /// Build the `G1`, `G2`, `f` polynomials of one F2 chart from the recorded
    /// integer coefficient data (the documented monomial interpretation:
    /// linear entries over `(y1, y2, z1, z2)`; `f` slots `0` = `y1²` and `7` =
    /// `z2²`, the rest zero).
    fn f2_polys(
        g1: &[i64],
        g2: &[i64],
        f: &[i64],
        y_coords: (usize, usize),
    ) -> (QPoly, QPoly, QPoly) {
        let var = QPoly::var;
        let mut p1 = QPoly::constant(0);
        for (i, &c) in g1.iter().enumerate() {
            if c != 0 && i < 4 {
                p1 = p1.add(&var(i).scale(c));
            }
        }
        let mut p2 = QPoly::constant(0);
        for (i, &c) in g2.iter().enumerate() {
            if c != 0 && i < 4 {
                p2 = p2.add(&var(i).scale(c));
            }
        }
        // The z axes are the two coordinates outside the y pair (ascending).
        let zs: Vec<usize> = (0..4)
            .filter(|&a| a != y_coords.0 && a != y_coords.1)
            .collect();
        let mut pf = QPoly::constant(0);
        if !f.is_empty() && f[0] != 0 {
            pf = pf.add(&QPoly::pow2(y_coords.0).scale(f[0]));
        }
        if f.len() > 7 && f[7] != 0 {
            pf = pf.add(&QPoly::pow2(zs[1]).scale(f[7]));
        }
        (p1, p2, pf)
    }

    /// A 3x3 exact determinant of QPolys.
    fn det3p(m: &[[QPoly; 3]; 3]) -> QPoly {
        let d00 = m[1][1].mul(&m[2][2]).sub(&m[1][2].mul(&m[2][1]));
        let d01 = m[1][0].mul(&m[2][2]).sub(&m[1][2].mul(&m[2][0]));
        let d02 = m[1][0].mul(&m[2][1]).sub(&m[1][1].mul(&m[2][0]));
        let t0 = m[0][0].mul(&d00);
        let t1 = m[0][1].mul(&d01);
        let t2 = m[0][2].mul(&d02);
        t0.sub(&t1).add(&t2)
    }

    /// The T1.1 right-hand side `det A · (f_{z_j} − f_y · A⁻¹ · G_{z_j})`,
    /// evaluated without division via the adjugate:
    /// `det A·f_{z_j} − f_y·adj(A)·G_{z_j}` — an exact integer polynomial.
    fn t11_rhs(g1: &QPoly, g2: &QPoly, f: &QPoly, y0: usize, y1: usize, z: usize) -> QPoly {
        let a00 = g1.partial(y0);
        let a01 = g1.partial(y1);
        let a10 = g2.partial(y0);
        let a11 = g2.partial(y1);
        let det_a = a00.mul(&a11).sub(&a01.mul(&a10));
        // adj(A) = [[a11, -a01], [-a10, a00]].
        let fy0 = f.partial(y0);
        let fy1 = f.partial(y1);
        let gz = [g1.partial(z), g2.partial(z)];
        // f_y · adj(A) · G_z = fy0·(a11·gz0 + (−a01)·gz1) + fy1·((−a10)·gz0 + a00·gz1).
        let t0 = a11.mul(&gz[0]).sub(&a01.mul(&gz[1]));
        let t1 = a10.neg().mul(&gz[0]).add(&a00.mul(&gz[1]));
        let lin = fy0.mul(&t0).add(&fy1.mul(&t1));
        det_a.mul(&f.partial(z)).sub(&lin)
    }

    #[test]
    fn chart_minors_match_t11_identity_on_f2() {
        let kit = crate::tangency::fixtures::build_kit().expect("the F2 kit admits");
        for chart_data in [&kit.f2.first, &kit.f2.second] {
            let (y0, y1) = chart_data.y_coords;
            let zs: Vec<usize> = (0..4).filter(|&a| a != y0 && a != y1).collect();
            let (g1, g2, f) = f2_polys(
                &chart_data.g1,
                &chart_data.g2,
                &chart_data.f,
                chart_data.y_coords,
            );
            for &z in zs.iter() {
                // M_j directly as the 3x3 determinant over columns (y0, y1, z).
                let m = [
                    [g1.partial(y0), g1.partial(y1), g1.partial(z)],
                    [g2.partial(y0), g2.partial(y1), g2.partial(z)],
                    [f.partial(y0), f.partial(y1), f.partial(z)],
                ];
                let lhs = det3p(&m);
                let rhs = t11_rhs(&g1, &g2, &f, y0, y1, z);
                let residual = lhs.sub(&rhs);
                assert!(
                    residual.is_zero(),
                    "T1.1 residual must vanish exactly on the F2 chart, got {:?}",
                    residual
                );
            }
        }
    }
}
