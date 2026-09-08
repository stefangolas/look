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

//! The Lemma T1.0 bounded 18-chart search (CTE-002-GRAPH, part one).
//!
//! A *rank-2 chart* is a component choice × coordinate-pair choice over the
//! stored four-axis difference system: `f` is one of the three `F` components
//! and `y = (y1, y2)` is a pair of the four chart coordinates, so
//! `A = D_y G` is the `2 x 2` pivot block of the remaining two components
//! `G = (G_1, G_2)`. Lemma T1.0 (theory §2.1): a point where `DF` has rank 2
//! admits at least one of the `3 * C(4,2) = 18` charts with
//! `det A ≠ 0`, so the search is a bounded enumeration.
//!
//! [`find_chart`] enumerates the 18 charts in the FIXED order (component
//! `x, y, z`; coordinate pairs lexicographic) and returns the FIRST chart whose
//! certified `det A(B)` enclosure excludes `0` strictly — determinism gate, no
//! conditioning score, no "best pivot" heuristic (scope decision 1). The
//! determinant enclosure is a `2 x 2` interval determinant of certified first
//! partials ([`partial_enclosure`]) over the pivot's rows and columns; the
//! chart-minor grids (`M_1, M_2`) are CTE-003's polynomial work and are NOT
//! built here — the returned [`Rank2Chart`] carries the frozen shape with a
//! documented zero placeholder in the minors slot until CTE-003 lands the real
//! nets (scope decision 4; RESULT note).
//!
//! This module also hosts the shared certified interval engine over a stored
//! [`SquareSystem3`] (`pub(crate)`, reused by the graph machinery): certified
//! component hulls over chart-coordinate boxes, certified first partials (the
//! landed [`partial_enclosure`]), and plain-`f64` point/partial evaluation used
//! only to form float preconditioner estimates and test probes. Every numeric
//! refusal maps onto the landed [`SsiRefusal`] vocabulary (zero new top-level
//! evidence kinds).

use crate::contract::Refusal;
use crate::formal::exact::CertifiedInterval;
use crate::ssi::SsiRefusal;
use crate::ssi_types::SquareSystem3;
use crate::tangency::shapes::{ChartMinorGrids, ChartMinorNet, Rank2Chart, Rank2Pivot};

/// The coordinate-pair choices of the 18-chart enumeration, lexicographic.
pub(crate) const COORD_PAIRS: [(usize, usize); 6] =
    [(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)];

/// A four-axis box `(lo, hi)` per chart coordinate, in the stored system's
/// chart-coordinate order (`(u, v, s, t)` = axis `0..=3`).
pub type Box4 = [(f64, f64); 4];

/// Why a CTE tangency chart-search or graph-certification operation could not
/// be certified.
///
/// Named cases only — no catch-all, mirroring the refusal shape of the rest of
/// the crate. The two graph-layer cases are [`TangencyRefusal::NoChart`] (all
/// 18 pivots' determinant enclosures contain `0`: the box is not
/// rank-2-admissible) and [`TangencyRefusal::GraphFailure`] (the parametric
/// contraction failed at the budgeted depth). Every other refusal wraps a
/// landed [`SsiRefusal`] verbatim (D-reuse — no new top-level evidence kinds).
///
/// `Eq` is intentionally absent (FSSI-000 decision 3): the wrapped
/// [`SsiRefusal`] now carries `(f64, f64)` suspicion evidence, and nothing in
/// the crate uses this refusal as a map key.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TangencyRefusal {
    /// All 18 chart-pivot determinant enclosures contain zero over the box: the
    /// box is not rank-2-admissible.
    NoChart,
    /// The parametric (H-graph) contraction could not certify the graph at the
    /// budgeted subdivision depth.
    GraphFailure,
    /// A landed SSI refusal (a hull/domain/conditioning cause) wrapped verbatim.
    Ssi(SsiRefusal),
}

impl TangencyRefusal {
    /// A short stable tag, for diagnostics.
    pub fn tag(self) -> &'static str {
        match self {
            Self::NoChart => "tangency_no_chart",
            Self::GraphFailure => "tangency_graph_failure",
            Self::Ssi(cause) => match cause {
                SsiRefusal::PairClass(_) => "tangency_ssi_pair_class",
                SsiRefusal::Conditioning(_) => "tangency_ssi_conditioning",
                SsiRefusal::Hull(_) => "tangency_ssi_hull",
                SsiRefusal::DeterminantSpansZero => "tangency_ssi_determinant_spans_zero",
                SsiRefusal::InclusionNotStrict => "tangency_ssi_inclusion_not_strict",
                SsiRefusal::InvalidInput => "tangency_ssi_invalid_input",
                // FSSI-000 decision 4 (FSSI-EXT): the FSSI-001 gate's
                // suspicion halts fold onto the existing low-information
                // pattern here too — the wrapped value still carries the
                // specificity and the SSI-layer `tag()` carries the naming the
                // producers assert against. The arm cannot fire until
                // FSSI-001/002 wire the producers; it documents that.
                SsiRefusal::TangentCurveSuspected { .. } => "tangency_ssi_invalid_input",
                SsiRefusal::CoincidentPatchSuspected { .. } => "tangency_ssi_invalid_input",
            },
        }
    }
}

impl From<SsiRefusal> for TangencyRefusal {
    fn from(refusal: SsiRefusal) -> Self {
        Self::Ssi(refusal)
    }
}

impl From<Refusal> for TangencyRefusal {
    fn from(refusal: Refusal) -> Self {
        Self::Ssi(SsiRefusal::from(refusal))
    }
}

// ---------------------------------------------------------------------------
// Chart geometry helpers (component/axis bookkeeping over the four axes).
// ---------------------------------------------------------------------------

/// The two `F` component indices taken as `G = (G_1, G_2)` when `f = F_c`:
/// the remaining two components in ascending order.
pub(crate) fn g_rows_of(component: usize) -> [usize; 2] {
    let mut out = [0usize; 2];
    let mut k = 0;
    for row in 0..3 {
        if row != component {
            out[k] = row;
            k += 1;
        }
    }
    out
}

/// The two chart axes taken as `z` when the coordinate pair `(a, b)` is taken
/// as `y`: the complementary axes in ascending order.
pub(crate) fn z_axes_of(coord_pair: (usize, usize)) -> [usize; 2] {
    let (a, b) = coord_pair;
    let mut out = [0usize; 2];
    let mut k = 0;
    for axis in 0..4 {
        if axis != a && axis != b {
            out[k] = axis;
            k += 1;
        }
    }
    out
}

/// Whether a chart-coordinate box is finite, ordered, and a compact subset of
/// the stored chart rectangle on every axis.
pub(crate) fn box_in_chart(system: &SquareSystem3, b: Box4) -> bool {
    let maps = system.domain_maps();
    let lo = [maps.0, maps.2, maps.4, maps.6];
    let hi = [maps.1, maps.3, maps.5, maps.7];
    for axis in 0..4 {
        let (lo_, hi_) = b[axis];
        if !lo_.is_finite() || !hi_.is_finite() || lo_ > hi_ {
            return false;
        }
        if !(lo[axis] <= lo_ && hi_ <= hi[axis]) {
            return false;
        }
    }
    true
}

// ---------------------------------------------------------------------------
// Certified interval engine over a stored square system.
//
// These are `pub(crate)` because the graph machinery (CTE-002 part two) composes
// the same certified primitives. The interval hull replication below mirrors
// the landed de-Casteljau-over-`CertifiedInterval` discipline of
// `ssi::hull_tensor4` (which is private to `ssi.rs`); the certified first
// partials are the landed [`partial_enclosure`] itself.
// ---------------------------------------------------------------------------

/// One chart axis's interval, mapped onto the unit chart `[0, 1]`, outward
/// rounded and clamped. `None` when the interval is not a compact subset of the
/// axis's chart rectangle.
pub(crate) fn to_unit_interval(lo: f64, hi: f64, d0: f64, d1: f64) -> Option<(f64, f64)> {
    if !lo.is_finite() || !hi.is_finite() || !d0.is_finite() || !d1.is_finite() {
        return None;
    }
    if !(d0 <= lo && lo <= hi && hi <= d1) {
        return None;
    }
    let width = CertifiedInterval::point(d1).sub(&CertifiedInterval::point(d0));
    if width.lo <= 0.0 {
        return None;
    }
    let lo_u = CertifiedInterval::point(lo).sub(&CertifiedInterval::point(d0));
    let hi_u = CertifiedInterval::point(hi).sub(&CertifiedInterval::point(d0));
    let lo_div = lo_u.div(&width)?;
    let hi_div = hi_u.div(&width)?;
    let u_lo = lo_div.lo.min(hi_div.lo).clamp(0.0, 1.0);
    let u_hi = lo_div.hi.max(hi_div.hi).clamp(0.0, 1.0);
    Some((u_lo, u_hi))
}

/// Interval de Casteljau over one axis for a 1-D coefficient list.
pub(crate) fn one_d_interval(
    pts: &[CertifiedInterval],
    u: &CertifiedInterval,
) -> Result<CertifiedInterval, SsiRefusal> {
    if pts.is_empty() {
        return Err(SsiRefusal::Hull(
            crate::hull::HullRefusal::EnclosureUnavailable,
        ));
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
        Err(SsiRefusal::Hull(
            crate::hull::HullRefusal::EnclosureUnavailable,
        ))
    }
}

/// Certified range enclosure of one stored component grid over a box whose
/// axis intervals are unit-chart `[0, 1]` subintervals.
///
/// Reduction is axis by axis by interval de Casteljau — the outward-rounded
/// discipline of the landed hull kernels.
pub(crate) fn component_unit_hull(
    grid: &[Vec<f64>],
    degrees: (usize, usize, usize, usize),
    box_axis: [(f64, f64); 4],
) -> Result<CertifiedInterval, SsiRefusal> {
    use crate::hull::HullRefusal;
    for (lo, hi) in box_axis {
        if !lo.is_finite() || !hi.is_finite() || !(lo >= 0.0 && hi <= 1.0 && lo <= hi) {
            return Err(SsiRefusal::Hull(HullRefusal::DomainNotCompact));
        }
    }
    let (m1, n1, m2, n2) = degrees;
    let rows = (m1 + 1) * (n1 + 1);
    let cols = (m2 + 1) * (n2 + 1);
    if grid.len() != rows || grid.iter().any(|row| row.len() != cols) {
        return Err(SsiRefusal::Hull(HullRefusal::EnclosureUnavailable));
    }
    if grid.is_empty() || grid[0].is_empty() {
        return Err(SsiRefusal::Hull(HullRefusal::EnclosureUnavailable));
    }
    let sp1 = n1 + 1;
    let u_iv = CertifiedInterval {
        lo: box_axis[0].0,
        hi: box_axis[0].1,
    };
    let u_len = m1 + 1;
    // Reduce axis 0 (u) over each flat column, grouped by column so the
    // subsequent axis-1 reduction iterates columns (the landed layout).
    let mut u_cols = vec![Vec::<CertifiedInterval>::with_capacity(n1 + 1); cols];
    for b in 0..=n1 {
        for (c, slot) in u_cols.iter_mut().enumerate() {
            let mut pts = Vec::with_capacity(u_len);
            for a in 0..=m1 {
                pts.push(CertifiedInterval::point(grid[a * sp1 + b][c]));
            }
            slot.push(one_d_interval(&pts, &u_iv)?);
        }
    }
    let v_iv = CertifiedInterval {
        lo: box_axis[1].0,
        hi: box_axis[1].1,
    };
    let mut v_collapsed = Vec::with_capacity(cols);
    for col in u_cols {
        v_collapsed.push(one_d_interval(&col, &v_iv)?);
    }
    // Rebuild the (s, t) bivariate interval grid and bound it over (s, t).
    let sp2 = n2 + 1;
    let mut grid2: Vec<Vec<CertifiedInterval>> = Vec::with_capacity(m2 + 1);
    for row_slice in v_collapsed.chunks(sp2) {
        grid2.push(row_slice.to_vec());
    }
    hull_2d_interval(&grid2, box_axis[2], box_axis[3])
}

/// Interval de Casteljau over the `(s, t)` box of an interval-valued bivariate
/// tensor grid (`grid[i][j]` = coefficient of `B^i_m(s) B^j_n(t)`).
pub(crate) fn hull_2d_interval(
    grid: &[Vec<CertifiedInterval>],
    s: (f64, f64),
    t: (f64, f64),
) -> Result<CertifiedInterval, SsiRefusal> {
    use crate::hull::HullRefusal;
    if grid.is_empty() || grid[0].is_empty() {
        return Err(SsiRefusal::Hull(HullRefusal::EnclosureUnavailable));
    }
    let width = grid[0].len();
    if grid.iter().any(|row| row.len() != width) {
        return Err(SsiRefusal::Hull(HullRefusal::EnclosureUnavailable));
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
        Err(SsiRefusal::Hull(HullRefusal::EnclosureUnavailable))
    }
}

/// Certified range enclosure of one component of the stored difference system
/// over a box in chart coordinates. The chart-rectangle requirement is
/// [`SsiRefusal::Hull`]`(`[`crate::hull::HullRefusal::DomainNotCompact`]`)` on a
/// box that leaves the chart.
pub(crate) fn certified_value(
    system: &SquareSystem3,
    component: usize,
    b: Box4,
) -> Result<CertifiedInterval, SsiRefusal> {
    if component > 2 {
        return Err(SsiRefusal::InvalidInput);
    }
    let maps = system.domain_maps();
    let mlo = [maps.0, maps.2, maps.4, maps.6];
    let mhi = [maps.1, maps.3, maps.5, maps.7];
    let mut unit = [(0.0f64, 0.0f64); 4];
    for axis in 0..4 {
        match to_unit_interval(b[axis].0, b[axis].1, mlo[axis], mhi[axis]) {
            Some(u) => unit[axis] = u,
            None => {
                return Err(SsiRefusal::Hull(crate::hull::HullRefusal::DomainNotCompact));
            }
        }
    }
    let grid = system
        .grids()
        .get(component)
        .ok_or(SsiRefusal::InvalidInput)?;
    component_unit_hull(grid, system.degrees(), unit)
}

// ---------------------------------------------------------------------------
// Plain-f64 evaluation (preconditioner estimates and test probes only).
// ---------------------------------------------------------------------------

/// 1-D float de Casteljau evaluation of a coefficient list at `x`.
fn one_d_float(pts: &[f64], x: f64) -> f64 {
    let mut level = pts.to_vec();
    while level.len() > 1 {
        let mut next = Vec::with_capacity(level.len() - 1);
        for w in level.windows(2) {
            next.push(w[0] + x * (w[1] - w[0]));
        }
        level = next;
    }
    level[0]
}

/// Plain-`f64` evaluation of one stored component grid at a UNIT chart point.
fn component_unit_eval(
    grid: &[Vec<f64>],
    degrees: (usize, usize, usize, usize),
    unit: [f64; 4],
) -> Option<f64> {
    let (m1, n1, m2, n2) = degrees;
    let rows = (m1 + 1) * (n1 + 1);
    let cols = (m2 + 1) * (n2 + 1);
    if grid.len() != rows || grid.iter().any(|row| row.len() != cols) {
        return None;
    }
    let (u, v, s, t) = (unit[0], unit[1], unit[2], unit[3]);
    let sp1 = n1 + 1;
    // Reduce axis 0 (u) over each flat column, grouped by column.
    let mut u_cols = vec![Vec::<f64>::with_capacity(n1 + 1); cols];
    for b in 0..=n1 {
        for (c, slot) in u_cols.iter_mut().enumerate() {
            let mut pts = Vec::with_capacity(m1 + 1);
            for a in 0..=m1 {
                pts.push(grid[a * sp1 + b][c]);
            }
            slot.push(one_d_float(&pts, u));
        }
    }
    let mut v_collapsed = Vec::with_capacity(cols);
    for col in u_cols {
        v_collapsed.push(one_d_float(&col, v));
    }
    let sp2 = n2 + 1;
    let mut grid2: Vec<Vec<f64>> = Vec::with_capacity(m2 + 1);
    for row_slice in v_collapsed.chunks(sp2) {
        grid2.push(row_slice.to_vec());
    }
    let mut col_evals = Vec::with_capacity(grid2[0].len());
    for j in 0..grid2[0].len() {
        let col: Vec<f64> = grid2.iter().map(|row| row[j]).collect();
        col_evals.push(one_d_float(&col, s));
    }
    Some(one_d_float(&col_evals, t))
}

/// Map a chart-coordinate point onto the unit chart.
pub(crate) fn unit_point(system: &SquareSystem3, p: [f64; 4]) -> Option<[f64; 4]> {
    let maps = system.domain_maps();
    let mlo = [maps.0, maps.2, maps.4, maps.6];
    let mhi = [maps.1, maps.3, maps.5, maps.7];
    let mut out = [0.0f64; 4];
    for axis in 0..4 {
        let d0 = mlo[axis];
        let d1 = mhi[axis];
        if !p[axis].is_finite() || !(d0 <= p[axis] && p[axis] <= d1) || d1 <= d0 {
            return None;
        }
        out[axis] = (p[axis] - d0) / (d1 - d0);
    }
    Some(out)
}

/// Plain-`f64` first partial of one component along a chart axis at a point.
///
/// The partial is evaluated in the unit chart (Bernstein coefficient
/// derivative) and scaled by the inverse chart width, exactly mirroring the
/// certified [`partial_enclosure`] scaling. This is a preconditioner estimate
/// only — never a certified value.
pub(crate) fn float_partial(
    system: &SquareSystem3,
    component: usize,
    axis: usize,
    p: [f64; 4],
) -> Option<f64> {
    if component > 2 || axis > 3 {
        return None;
    }
    let (derived, degrees) = differentiate_axis(system, component, axis)?;
    let unit = unit_point(system, p)?;
    let unit_partial = component_unit_eval(&derived, degrees, unit)?;
    let maps = system.domain_maps();
    let widths = [
        maps.1 - maps.0,
        maps.3 - maps.2,
        maps.5 - maps.4,
        maps.7 - maps.6,
    ];
    let width = widths[axis];
    if width <= 0.0 {
        return None;
    }
    Some(unit_partial / width)
}

/// A derived coefficient grid and its reduced degrees (plain-`f64`).
type FloatDerivative = (Vec<Vec<f64>>, (usize, usize, usize, usize));

/// The Bernstein first-derivative coefficient grid of one component along one
/// chart axis, in the flat layout with the reduced degree (the plain-`f64`
/// counterpart of the landed `Tensor4::partial_axis`).
fn differentiate_axis(
    system: &SquareSystem3,
    component: usize,
    axis: usize,
) -> Option<FloatDerivative> {
    if component > 2 || axis > 3 {
        return None;
    }
    let (m1, n1, m2, n2) = system.degrees();
    let base = [m1, n1, m2, n2][axis];
    if base == 0 {
        return None;
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
    let grid = system.grids().get(component)?;
    let sp1 = n1 + 1;
    let sp2 = n2 + 1;
    let mut out = vec![vec![0.0f64; cols]; rows];
    for a in 0..=nm1 {
        for b in 0..=nn1 {
            for i in 0..=nm2 {
                for j in 0..=nn2 {
                    let (a1, b1) = match axis {
                        0 => (a + 1, b),
                        1 => (a, b + 1),
                        _ => (a, b),
                    };
                    let (i1, j1) = match axis {
                        2 => (i + 1, j),
                        3 => (i, j + 1),
                        _ => (i, j),
                    };
                    let lo = grid.get(a * sp1 + b)?.get(i * sp2 + j).copied()?;
                    let hi = grid.get(a1 * sp1 + b1)?.get(i1 * sp2 + j1).copied()?;
                    let dst_row = a * (nn1 + 1) + b;
                    let dst_col = i * (nn2 + 1) + j;
                    out.get_mut(dst_row)?.get_mut(dst_col)?;
                    out[dst_row][dst_col] = scale * (hi - lo);
                }
            }
        }
    }
    Some((out, degrees))
}

// ---------------------------------------------------------------------------
// The certified 2 x 2 pivot determinant and the 18-chart search.
// ---------------------------------------------------------------------------

/// The certified determinant enclosure `det A(B)` of one chart pivot over the
/// box: the `2 x 2` interval determinant of the certified first partials of the
/// pivot's two `G` rows along the pivot's two `y` columns.
pub(crate) fn pivot_det(
    system: &SquareSystem3,
    b: Box4,
    pivot: Rank2Pivot,
) -> Result<CertifiedInterval, TangencyRefusal> {
    let rows = g_rows_of(pivot.component());
    let (ya, yb) = pivot.coord_pair();
    let a00 = crate::ssi::partial_enclosure(system, rows[0], ya, b)?;
    let a01 = crate::ssi::partial_enclosure(system, rows[0], yb, b)?;
    let a10 = crate::ssi::partial_enclosure(system, rows[1], ya, b)?;
    let a11 = crate::ssi::partial_enclosure(system, rows[1], yb, b)?;
    Ok(a00.mul(&a11).sub(&a01.mul(&a10)))
}

/// The documented zero placeholder in the chart-minor slot of the frozen
/// [`Rank2Chart`].
///
/// Chart minors (`M_1, M_2`) are CTE-003's polynomial work (scope decision 4).
/// CTE-002 certifies only the pivot determinant, so the `Rank2Chart` it returns
/// carries an all-zero degree-0 minor net as a clearly-unset placeholder; CTE-003
/// rebuilds the chart with the real minor nets from the recorded pivot and
/// `det_a`.
fn placeholder_minors() -> Result<ChartMinorGrids, Refusal> {
    let net = ChartMinorNet::new([0, 0, 0, 0], vec![0.0])?;
    ChartMinorGrids::new(net.clone(), net)
}

/// Lemma T1.0 as a bounded search (theory §2.1; scope decision 1).
///
/// Enumerates the 18 charts in FIXED order — component `x, y, z`; coordinate
/// pairs lexicographic — and returns the FIRST chart whose certified
/// `det A(B)` enclosure strictly excludes `0`. The success predicate is exactly
/// `0 ∉ det A(B)`; there is no conditioning score and no "best pivot"
/// heuristic. When all 18 pivots' enclosures contain `0` the box is not
/// rank-2-admissible and the search refuses [`TangencyRefusal::NoChart`] — a
/// transversal box is such a box, and that is a correct outcome, not an error.
///
/// `b` is a box in the stored system's chart coordinates; every axis must lie
/// inside that axis's chart rectangle.
pub fn find_chart(system: &SquareSystem3, b: Box4) -> Result<Rank2Chart, TangencyRefusal> {
    if !box_in_chart(system, b) {
        return Err(TangencyRefusal::Ssi(SsiRefusal::Hull(
            crate::hull::HullRefusal::DomainNotCompact,
        )));
    }
    for component in 0..3 {
        for &coord_pair in COORD_PAIRS.iter() {
            let pivot = Rank2Pivot::new(component, coord_pair).map_err(TangencyRefusal::from)?;
            let det = pivot_det(system, b, pivot)?;
            if !det.is_finite() {
                // A non-finite enclosure certifies nothing: this pivot does not
                // admit the chart; the search moves on.
                continue;
            }
            if det.lo > 0.0 || det.hi < 0.0 {
                let minors = placeholder_minors().map_err(TangencyRefusal::from)?;
                return Rank2Chart::new(pivot, det, minors)
                    .map_err(|_| TangencyRefusal::Ssi(SsiRefusal::InvalidInput));
            }
        }
    }
    Err(TangencyRefusal::NoChart)
}

// ---------------------------------------------------------------------------
// The shared CTE fixture realization kit (test support only).
//
// The CTE-000 fixture kit (`tangency/fixtures.rs`) freezes the normative
// ground truths of F3/F4/F6 as exact geometry data but does not realize them
// as `SquareSystem3` instances. The wave packets realize each fixture over the
// identity unit chart from unit-weight rational graph patches, then machine
// check the realization's ground truths against the fixture's exact data
// before any certification runs (the `ssi_fixtures` precedent). This kit is
// `#[cfg(test)] pub(crate)` so the CTE-002 chart and graph tests share it.
// ---------------------------------------------------------------------------

#[cfg(test)]
pub(crate) mod test_kit {
    use crate::ssi_types::SquareSystem3;
    use crate::tangency::chart::{pivot_det, Box4, COORD_PAIRS};
    use crate::tangency::shapes::Rank2Pivot;

    /// A small exact binomial coefficient.
    fn binom(n: usize, k: usize) -> f64 {
        let mut numerator = 1u64;
        let mut denominator = 1u64;
        for i in 0..k {
            numerator *= (n - i) as u64;
            denominator *= (i + 1) as u64;
        }
        numerator as f64 / denominator as f64
    }

    /// Add `coeff * u^pu * v^pv` (monomial basis) onto a Bernstein grid.
    fn add_monomial(grid: &mut [Vec<f64>], m: usize, n: usize, pu: usize, pv: usize, coeff: f64) {
        let row_factors: Vec<f64> = (pu..=m).map(|a| binom(a, pu) / binom(m, pu)).collect();
        let col_factors: Vec<f64> = (pv..=n).map(|b| binom(b, pv) / binom(n, pv)).collect();
        for (a, fa) in (pu..=m).zip(row_factors.iter()) {
            for (b, fb) in (pv..=n).zip(col_factors.iter()) {
                grid[a][b] += coeff * fa * fb;
            }
        }
    }

    /// A zero Bernstein grid of bidegree `(m, n)`.
    pub(crate) fn zero_grid(m: usize, n: usize) -> Vec<Vec<f64>> {
        vec![vec![0.0; n + 1]; m + 1]
    }

    /// A constant Bernstein grid of bidegree `(m, n)`.
    pub(crate) fn constant_grid(m: usize, n: usize, value: f64) -> Vec<Vec<f64>> {
        vec![vec![value; n + 1]; m + 1]
    }

    /// The `x`-coordinate grid `u` (which == 0) or `y`-coordinate grid `v`
    /// (which == 1) of the identity chart at bidegree `(m, n)`.
    pub(crate) fn coordinate_grid(m: usize, n: usize, which: usize) -> Vec<Vec<f64>> {
        let mut grid = Vec::with_capacity(m + 1);
        for a in 0..=m {
            let mut row = Vec::with_capacity(n + 1);
            for b in 0..=n {
                let v = if which == 0 {
                    a as f64 / m as f64
                } else {
                    b as f64 / n as f64
                };
                row.push(v);
            }
            grid.push(row);
        }
        grid
    }

    /// A Bernstein grid of bidegree `(m, n)` from monomial terms
    /// `(pu, pv, coeff)`.
    pub(crate) fn monomial_grid(
        m: usize,
        n: usize,
        terms: &[(usize, usize, f64)],
    ) -> Vec<Vec<f64>> {
        let mut grid = zero_grid(m, n);
        for &(pu, pv, coeff) in terms {
            add_monomial(&mut grid, m, n, pu, pv, coeff);
        }
        grid
    }

    /// The unit-weight rational graph patch `(x, y, z) = (u, v, h(u, v))` at
    /// bidegree `(m, n)` over the identity unit chart.
    fn graph_patch(m: usize, n: usize, h: Vec<Vec<f64>>) -> crate::ssi::RationalBipatch {
        crate::ssi::RationalBipatch::new(
            m,
            n,
            [coordinate_grid(m, n, 0), coordinate_grid(m, n, 1), h],
            constant_grid(m, n, 1.0),
        )
        .expect("a valid graph patch grid")
    }

    /// The cross-multiplied difference square system of two unit-weight graph
    /// patches over the identity unit chart.
    fn graph_cross(
        a: crate::ssi::RationalBipatch,
        b: crate::ssi::RationalBipatch,
    ) -> SquareSystem3 {
        crate::ssi::construct_square_system(
            &crate::ssi::SsiParticipant::RationalBipatch(a),
            &crate::ssi::SsiParticipant::RationalBipatch(b),
        )
        .expect("a valid cross system")
    }

    /// The realized F3 (A₁⁺) pair: the plane `z = 0` tangent to the convex bowl
    /// `z = (u − 1/2)² + (v − 1/2)²` at `(1/2, 1/2, 0)` — an isolated tangential
    /// contact (definite reduced Hessian), realizing the fixture kit's plane
    /// × sphere A₁⁺ tangency over the identity chart.
    ///
    /// `box_` is the certification box: `Y = [0.2, 0.8]²` on the `y = (u, v)`
    /// axes strictly contains the `z = (s, t)` domain `[0.4, 0.6]²`, so the
    /// trivial graph `φ(z) = z` lies strictly inside `Y`.
    pub(crate) fn f3_plane_bowl() -> (SquareSystem3, Box4) {
        let h2 = monomial_grid(
            2,
            2,
            &[
                (2, 0, 1.0),
                (1, 0, -1.0),
                (0, 2, 1.0),
                (0, 1, -1.0),
                (0, 0, 0.5),
            ],
        );
        let bowl = graph_patch(2, 2, h2);
        let plane = graph_patch(1, 1, constant_grid(1, 1, 0.0));
        let system = graph_cross(bowl, plane);
        let box_: Box4 = [(0.2, 0.8), (0.2, 0.8), (0.4, 0.6), (0.4, 0.6)];
        (system, box_)
    }

    /// The realized F4 (A₁⁻) pair: the plane `z = 0` × the rational saddle
    /// `z = (s − 1/2)² − (t − 1/2)²`, a tangential node at `(1/2, 1/2, 0)` —
    /// the fixture kit's A₁⁻ saddle over the identity chart.
    pub(crate) fn f4_plane_saddle() -> (SquareSystem3, Box4) {
        let h2 = monomial_grid(
            2,
            2,
            &[
                (2, 0, 1.0),
                (1, 0, -1.0),
                (0, 2, -1.0),
                (0, 1, 1.0),
                (0, 0, 0.0),
            ],
        );
        let saddle = graph_patch(2, 2, h2);
        let plane = graph_patch(1, 1, constant_grid(1, 1, 0.0));
        let system = graph_cross(saddle, plane);
        let box_: Box4 = [(0.2, 0.8), (0.2, 0.8), (0.4, 0.6), (0.4, 0.6)];
        (system, box_)
    }

    /// The certified `det A(B)` status of every one of the 18 pivots over the
    /// box: how many exclude zero, and the component/pair of the first.
    pub(crate) fn chart_census(system: &SquareSystem3, b: Box4) -> (usize, Option<Rank2Pivot>) {
        let mut count = 0usize;
        let mut first = None;
        for component in 0..3 {
            for &coord_pair in COORD_PAIRS.iter() {
                let pivot =
                    Rank2Pivot::new(component, coord_pair).expect("an enumerated pivot is valid");
                if let Ok(det) = pivot_det(system, b, pivot) {
                    if det.is_finite() && (det.lo > 0.0 || det.hi < 0.0) {
                        if count == 0 {
                            first = Some(pivot);
                        }
                        count += 1;
                    }
                }
            }
        }
        (count, first)
    }

    /// A four-axis monomial term `(pu, pv, ps, pt)` with coefficient, in the
    /// unit chart variables `(u, v, s, t)`, accumulated into a flat tensor
    /// grid of the given degrees (each monomial converted to the tensor
    /// Bernstein basis exactly for the small integer degrees used here).
    fn tensor_monomial_grid(
        degrees: (usize, usize, usize, usize),
        terms: &[(usize, usize, usize, usize, f64)],
    ) -> Vec<Vec<f64>> {
        let (m1, n1, m2, n2) = degrees;
        let rows = (m1 + 1) * (n1 + 1);
        let cols = (m2 + 1) * (n2 + 1);
        let mut grid = vec![vec![0.0f64; cols]; rows];
        for &(pu, pv, ps, pt, coeff) in terms {
            for a in pu..=m1 {
                for b in pv..=n1 {
                    for i in ps..=m2 {
                        for j in pt..=n2 {
                            let factor = binom(a, pu) / binom(m1, pu) * binom(b, pv)
                                / binom(n1, pv)
                                * binom(i, ps)
                                / binom(m2, ps)
                                * binom(j, pt)
                                / binom(n2, pt);
                            grid[a * (n1 + 1) + b][i * (n2 + 1) + j] += coeff * factor;
                        }
                    }
                }
            }
        }
        grid
    }

    /// A directly-constructed transversal `SquareSystem3` realization of the F6
    /// crossing fixture, parameterized by the multiplier slopes.
    ///
    /// The system's zero set is exactly the straight segment
    /// `{u = v = s = t}` inside the box (each `F_k = m_k·(axis difference)`
    /// with `m_k` strictly positive on the box), and the Jacobian has rank 3 at
    /// every point of it — a transversal crossing. The three difference forms
    /// `u − v`, `v − s`, `s − t` are coupled through sign-varying multipliers,
    /// so no constant identity pivot survives: over the straddling box every
    /// one of the 18 pivot determinant enclosures contains zero (RESULT note:
    /// an affine plane-pair difference always carries a constant nonzero pivot,
    /// so the F6 no-chart realization is this curved coupled transversal
    /// system rather than two affine planes).
    fn f6_direct(slopes: [f64; 3]) -> SquareSystem3 {
        let degrees = (2usize, 2usize, 2usize, 2usize);
        let a = 6.0f64;
        let [b0, b1, b2] = slopes;
        // F0 = (A + b0·(u − 1/2))·(u − v); F1 = ...·(v − s); F2 = ...·(s − t).
        let f0 = tensor_monomial_grid(
            degrees,
            &[
                (1, 0, 0, 0, a - 0.5 * b0),
                (0, 1, 0, 0, -a + 0.5 * b0),
                (2, 0, 0, 0, b0),
                (1, 1, 0, 0, -b0),
            ],
        );
        let f1 = tensor_monomial_grid(
            degrees,
            &[
                (0, 1, 0, 0, a - 0.5 * b1),
                (0, 0, 1, 0, -a + 0.5 * b1),
                (0, 2, 0, 0, b1),
                (0, 1, 1, 0, -b1),
            ],
        );
        let f2 = tensor_monomial_grid(
            degrees,
            &[
                (0, 0, 1, 0, a - 0.5 * b2),
                (0, 0, 0, 1, -a + 0.5 * b2),
                (0, 0, 2, 0, b2),
                (0, 0, 1, 1, -b2),
            ],
        );
        SquareSystem3::new(
            [f0, f1, f2],
            degrees,
            (0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0),
        )
        .expect("a valid direct transversal system")
    }

    /// The realized F6 (transversal) instance and its straddling box.
    ///
    /// The multiplier slopes are the fixed tuple whose certified 18-pivot
    /// census is empty over the box; see [`f6_direct`].
    pub(crate) fn f6_transversal_crossing() -> (SquareSystem3, Box4) {
        let system = f6_direct([4.0, 8.0, 4.0]);
        let box_: Box4 = [(0.05, 0.95), (0.05, 0.95), (0.05, 0.95), (0.05, 0.95)];
        (system, box_)
    }
}

#[cfg(test)]
mod tests {
    use super::test_kit;
    use super::*;
    use crate::ssi_types::SquareSystem3;
    use crate::tangency::fixtures::{
        F3PlaneSphereFixture, F4SaddleFixture, F6CrossingPlanesFixture,
    };

    /// Admit the fixture kit's exact ground truths this test drives (the kit's
    /// own machine checks), then certify the A₁⁺ chart over the F3 box.
    #[test]
    fn chart_search_finds_pivot_on_rank2_fixture() {
        F3PlaneSphereFixture::new()
            .admit()
            .expect("the F3 ground truth admits");
        F4SaddleFixture::new()
            .admit()
            .expect("the F4 ground truth admits");

        for (label, (system, box_)) in [
            ("f3", test_kit::f3_plane_bowl()),
            ("f4", test_kit::f4_plane_saddle()),
        ] {
            let chart = find_chart(&system, box_).unwrap_or_else(|r| {
                panic!("the {label} rank-2 box must admit a chart, refused with {r:?}")
            });
            let det = chart.det_a();
            assert!(
                det.lo > 0.0 || det.hi < 0.0,
                "the {label} chart's det_a must exclude zero"
            );
            let (census, _) = test_kit::chart_census(&system, box_);
            assert!(census >= 1, "the {label} box must admit at least one pivot");
        }

        // The machine-checked ground truth: F3's tangency is at the plane-sphere
        // contact point (fixture data), F4's at the saddle node.
        let f3 = F3PlaneSphereFixture::new();
        assert_eq!(f3.p, [0, 2, 0]);
        assert_eq!(f3.w, 2);
        let f4 = F4SaddleFixture::new();
        assert!(f4.det_h_upper < 0);
    }

    /// F6 (transversal): the certified 18-pivot census over the straddling box
    /// is empty, so the search refuses `NoChart` — a transversal box admits no
    /// rank-2 chart; that is CORRECT, not an error.
    ///
    /// The realization's zero set is the straight segment `{u = v = s = t}`
    /// through the box; the float samples below machine-check the ground truth
    /// (`F = 0` on the segment with `rank DF = 3` there — transverse — and
    /// `F ≠ 0` off it) before the census is asserted.
    #[test]
    fn chart_search_refuses_on_transversal_fixture() {
        F6CrossingPlanesFixture::new()
            .admit()
            .expect("the F6 ground truth admits");
        let (system, box_) = test_kit::f6_transversal_crossing();
        for w in [0.2f64, 0.5, 0.8] {
            let f = crate::ssi_fixtures::eval_system(&system, (w, w, w, w))
                .expect("the system evaluates on the branch");
            for value in f {
                assert!(
                    value.abs() < 1e-9, // H-3: float ground-truth residual
                    "F must vanish on the branch segment"
                );
            }
            assert_eq!(
                float_rank(&system, (w, w, w, w)),
                3,
                "the branch is transverse (rank 3) at every sample"
            );
        }
        let off = crate::ssi_fixtures::eval_system(&system, (0.25, 0.5, 0.75, 0.5))
            .expect("the system evaluates off the branch");
        assert!(
            off.iter().any(|v| v.abs() > 1e-6), // H-3: off-branch separation
            "F must not vanish off the branch segment"
        );
        let (census, _) = test_kit::chart_census(&system, box_);
        assert_eq!(census, 0, "a transversal box admits no rank-2 chart");
        let refusal = find_chart(&system, box_);
        assert!(matches!(refusal, Err(TangencyRefusal::NoChart)));
    }

    /// The chart-search refusal is a named typed cause with a stable tag —
    /// never a string.
    #[test]
    fn no_chart_refusal_is_named() {
        let (system, box_) = test_kit::f6_transversal_crossing();
        match find_chart(&system, box_) {
            Err(TangencyRefusal::NoChart) => {
                assert_eq!(TangencyRefusal::NoChart.tag(), "tangency_no_chart");
            }
            Err(other) => panic!("expected the named NoChart refusal, got {other:?}"),
            Ok(_) => panic!("a transversal box must not admit a rank-2 chart"),
        }
    }

    /// Plain-float rank of the stored system's Jacobian at a chart point, from
    /// the fixture kit's direct partial evaluation. Test support only.
    fn float_rank(system: &SquareSystem3, p: (f64, f64, f64, f64)) -> usize {
        let mut m = [[0.0f64; 4]; 3];
        for (row, mrow) in m.iter_mut().enumerate() {
            for (axis, cell) in mrow.iter_mut().enumerate() {
                *cell = crate::ssi_fixtures::partial_grid4_axis(
                    &system.grids()[row],
                    system.degrees(),
                    axis,
                    p,
                )
                .expect("a float partial at an interior point");
            }
        }
        let det3 = |cols: [usize; 3]| -> f64 {
            let a = |r: usize, c: usize| m[r][cols[c]];
            a(0, 0) * (a(1, 1) * a(2, 2) - a(1, 2) * a(2, 1))
                - a(0, 1) * (a(1, 0) * a(2, 2) - a(1, 2) * a(2, 0))
                + a(0, 2) * (a(1, 0) * a(2, 1) - a(1, 1) * a(2, 0))
        };
        let rank_eps = 1e-9; // H-3: float rank test threshold
        for cols in [[0, 1, 2], [0, 1, 3], [0, 2, 3], [1, 2, 3]] {
            if det3(cols).abs() > rank_eps {
                return 3;
            }
        }
        let det2 = |rows: [usize; 2], cols: [usize; 2]| -> f64 {
            m[rows[0]][cols[0]] * m[rows[1]][cols[1]] - m[rows[0]][cols[1]] * m[rows[1]][cols[0]]
        };
        for rows in [[0, 1], [0, 2], [1, 2]] {
            for cols in [[0, 1], [0, 2], [0, 3], [1, 2], [1, 3], [2, 3]] {
                if det2(rows, cols).abs() > rank_eps {
                    return 2;
                }
            }
        }
        0
    }
}
