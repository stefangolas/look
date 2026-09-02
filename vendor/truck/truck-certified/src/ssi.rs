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

//! The SSI square-system engine (BG-CK-P2-SYSTEM + KRAWCZYK3, collapsed).
//!
//! Wave member W1 implements the two booked packets' shared module
//! (`src/ssi.rs`) against the frozen shim (`ssi_types.rs`, `contract.rs`,
//! `ssi_fixtures.rs`): the square-system constructor (Section 1) and the
//! 3×3 Krawczyk unique-root certificate (Section 2). KRAWCZYK3 is never a
//! separate registry row; its booked content is this module's second half.
//!
//! # The square system (Section 1)
//!
//! From two certified-admitted rational tensor-Bernstein patches (control
//! grids + weights), the surface–surface difference is the cross-multiplied
//! homogeneous system
//!
//! ```text
//! F_k(u,v,s,t) = W2(s,t)·N1_k(u,v) − W1(u,v)·N2_k(s,t)   (k ∈ x, y, z)
//! ```
//!
//! over the product chart `(u,v) × (s,t)`, exactly as the shim froze it. The
//! stored `SquareSystem3` grid of component `k` is the coefficient array of
//! that tensor polynomial over the four chart axes, in the shim's flat
//! layout (`row = a·(n1+1)+b` indexes the `(u,v)` bidegree of patch 1,
//! `col = i·(n2+1)+j` the `(s,t)` bidegree of patch 2). [`construct_square_system`]
//! computes that coefficient grid from two patches and feeds the shim's
//! refusing `SquareSystem3::new`; ragged / empty / non-finite / degree-0
//! refusal is the shim's, never restated here.
//!
//! ## F3 square reduction
//!
//! A trace box is a compact product box in the four-axis chart. For a
//! candidate continuation axis the reduced 3×3 square system's unknowns are
//! the other three chart axes in ascending order and its equations are the F
//! components in order; the coordinate-`i` diagonal derivative `∂H_i/∂t_i`
//! is the partial of component `F_i` along the `i`-th smallest retained axis
//! (the fixture kit's documented identity pairing). [`f3_diagonal_derivatives`]
//! certifies those three enclosures over the box and the retained extents,
//! assembling the FROZEN [`SquareSystemInput`]; [`select_continuation_coordinate`]
//! applies the frozen rule verbatim (largest relative margin, lowest index on
//! ties, `ConditioningBelowThreshold` refuses — never a weaker retry).
//!
//! # The 3×3 Krawczyk certificate (Section 2)
//!
//! The 2D Krawczyk inner loop of `formal/bezier_isect.rs`, dimension-raised.
//! [`krawczyk3_certificate`] certifies a unique root of the reduced square
//! system on the slice `{continuation axis = box centre}` within the retained
//! 3D box `X`: the Jacobian minors are certified Bernstein-patch enclosures
//! (the landed `CertifiedInterval` de-Casteljau discipline, composed over the
//! four axes), the inverse is the adjugate over the determinant under
//! directed rounding, and only a STRICT inclusion `K(X) ⊂ int(X)` emits a
//! [`KrawczykCertificate3`] through the shim's strict-inclusion-only
//! constructor. Every non-result is a named refusal; there is no catch-all.
//!
//! # Refusal vocabulary
//!
//! [`SsiRefusal`] wraps the landed named causes verbatim (D-reuse): class
//! pairs outside spline-admissible shapes carry the DISPATCH widening
//! [`PairUnsupported::UnsupportedPairClass`], conditioning carries
//! [`Refusal::ConditioningBelowThreshold`], hull failures carry the landed
//! [`HullRefusal`] cases, and the certificate's own preconditions are the
//! two named cases `DeterminantSpansZero` and `InclusionNotStrict`. No new
//! top-level evidence kinds are introduced.

use crate::contract::{IntervalEnclosure, Refusal, SquareSystemInput};
use crate::formal::exact::CertifiedInterval;
use crate::formal::intersection::PairUnsupported;
use crate::formal::numeric::PositiveFinite;
use crate::hull::HullRefusal;
use crate::ssi_types::{KrawczykCertificate3, SquareSystem3};

/// Why an SSI square-system or Krawczyk3 operation could not be certified.
///
/// Named cases only — no catch-all — matching the refusal shape of the rest
/// of the crate. Each variant wraps a landed named cause verbatim (D-reuse):
/// a class-pair refusal carries [`PairUnsupported`], a conditioning refusal
/// carries [`Refusal`], and a hull failure carries [`HullRefusal`]. The two
/// certificate preconditions (`DeterminantSpansZero`, `InclusionNotStrict`)
/// are this module's own named cases, mirroring `bezier_isect`'s
/// typed-unresolved discipline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SsiRefusal {
    /// A pair whose class is outside the spline-admissible shapes. Carries
    /// the DISPATCH widening [`PairUnsupported::UnsupportedPairClass`].
    PairClass(PairUnsupported),
    /// The frozen F3 coordinate rule refused the box (no coordinate certifies
    /// away-from-zero). Carries [`Refusal::ConditioningBelowThreshold`].
    Conditioning(Refusal),
    /// A certified enclosure could not be produced by the hull layer. Carries
    /// the landed [`HullRefusal`] (`EnclosureUnavailable` / `DomainNotCompact`).
    Hull(HullRefusal),
    /// The reduced Jacobian determinant's enclosure over the box contains
    /// zero (the certificate's construction precondition).
    DeterminantSpansZero,
    /// The Krawczyk image is not component-wise STRICTLY inside the box (the
    /// certificate's emission precondition).
    InclusionNotStrict,
    /// A construction outside a frozen rule (the shim's refusing
    /// constructors refused).
    InvalidInput,
}

impl SsiRefusal {
    /// A short stable tag, for diagnostics.
    pub fn tag(self) -> &'static str {
        match self {
            Self::PairClass(cause) => cause.tag(),
            Self::Conditioning(Refusal::ConditioningBelowThreshold) => "ssi_conditioning",
            Self::Conditioning(Refusal::InvalidInput) => "ssi_invalid_input",
            Self::Conditioning(Refusal::Unfrozen) => "ssi_unfrozen",
            Self::Hull(HullRefusal::EnclosureUnavailable) => "ssi_hull_enclosure_unavailable",
            Self::Hull(HullRefusal::DomainNotCompact) => "ssi_hull_domain_not_compact",
            Self::DeterminantSpansZero => "ssi_determinant_spans_zero",
            Self::InclusionNotStrict => "ssi_inclusion_not_strict",
            Self::InvalidInput => "ssi_invalid_input",
        }
    }
}

impl From<Refusal> for SsiRefusal {
    fn from(refusal: Refusal) -> Self {
        match refusal {
            Refusal::ConditioningBelowThreshold => {
                Self::Conditioning(Refusal::ConditioningBelowThreshold)
            }
            Refusal::InvalidInput => Self::InvalidInput,
            Refusal::Unfrozen => Self::InvalidInput,
        }
    }
}

impl From<HullRefusal> for SsiRefusal {
    fn from(refusal: HullRefusal) -> Self {
        Self::Hull(refusal)
    }
}

// ---------------------------------------------------------------------------
// Four-axis tensor helpers over the shim's flat grid layout.
// ---------------------------------------------------------------------------

/// A four-axis coefficient grid `c[a][b][i][j]` stored in the shim's flat
/// layout: `rows = (m1+1)·(n1+1)` with row `a·(n1+1)+b`, `cols =
/// (m2+1)·(n2+1)` with col `i·(n2+1)+j`. Axis order is `(u, v, s, t)`.
#[derive(Debug, Clone)]
struct Tensor4 {
    /// Degrees `(m1, n1, m2, n2)`.
    degrees: (usize, usize, usize, usize),
    /// Flat coefficient rows (each of length `cols`).
    rows: Vec<Vec<f64>>,
}

impl Tensor4 {
    /// Wrap one stored component grid verbatim (shape already validated by
    /// the shim's `SquareSystem3::new`).
    fn from_grid(grid: &[Vec<f64>], degrees: (usize, usize, usize, usize)) -> Self {
        Tensor4 {
            degrees,
            rows: grid.to_vec(),
        }
    }

    /// Row spacing in the flat layout (`n1 + 1`).
    fn row_spacing(&self) -> usize {
        self.degrees.1 + 1
    }

    /// Column spacing in the flat layout (`n2 + 1`).
    fn col_spacing(&self) -> usize {
        self.degrees.3 + 1
    }

    /// Coefficient count along one axis (degree + 1).
    fn len_axis(&self, axis: usize) -> usize {
        let (m1, n1, m2, n2) = self.degrees;
        match axis {
            0 => m1 + 1,
            1 => n1 + 1,
            2 => m2 + 1,
            _ => n2 + 1,
        }
    }

    /// The first-partial coefficient grid along a chart axis.
    ///
    /// Bernstein derivative: a degree-`d` coefficient list differentiates to
    /// `d·(c[k+1] − c[k])` of degree `d − 1`. The result keeps the flat
    /// layout invariant with the reduced degree on that axis; a degree-0 axis
    /// refuses (a `SquareSystem3` never stores one, so this is defensive).
    fn partial_axis(&self, axis: usize) -> Result<Tensor4, SsiRefusal> {
        let (m1, n1, m2, n2) = self.degrees;
        let base = [m1, n1, m2, n2][axis];
        if base == 0 {
            return Err(SsiRefusal::InvalidInput);
        }
        let scale = base as f64;
        let degrees = match axis {
            0 => (m1 - 1, n1, m2, n2),
            1 => (m1, n1 - 1, m2, n2),
            2 => (m1, n1, m2 - 1, n2),
            _ => (m1, n1, m2, n2 - 1),
        };
        let (nm1, nn1, nm2, nn2) = degrees;
        // Layout after reduction: rows are a·(nn1+1)+b, cols i·(nn2+1)+j.
        let rows = (nm1 + 1) * (nn1 + 1);
        let cols = (nm2 + 1) * (nn2 + 1);
        let mut out = vec![vec![0.0f64; cols]; rows];
        let sp1 = self.row_spacing();
        let sp2 = self.col_spacing();
        for a in 0..=nm1 {
            for b in 0..=nn1 {
                for i in 0..=nm2 {
                    for j in 0..=nn2 {
                        // Source indices: advance one step on the axis being
                        // differentiated.
                        let (a0, b0) = match axis {
                            0 => (a, b), // diff between a and a+1
                            1 => (a, b), // diff between b and b+1
                            _ => (a, b),
                        };
                        let (i0, j0) = match axis {
                            2 => (i, j),
                            3 => (i, j),
                            _ => (i, j),
                        };
                        let (a1, b1) = match axis {
                            0 => (a + 1, b),
                            1 => (a, b + 1),
                            _ => (a0, b0),
                        };
                        let (i1, j1) = match axis {
                            2 => (i + 1, j),
                            3 => (i, j + 1),
                            _ => (i0, j0),
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
        Ok(Tensor4 { degrees, rows: out })
    }
}

// ---------------------------------------------------------------------------
// Certified hull over a box (landed de-Casteljau-over-CertifiedInterval).
// ---------------------------------------------------------------------------

/// Interval de Casteljau over one axis for a 1-D coefficient list.
fn one_d_interval(
    pts: &[CertifiedInterval],
    u: &CertifiedInterval,
) -> Result<CertifiedInterval, SsiRefusal> {
    if pts.is_empty() {
        return Err(SsiRefusal::Hull(HullRefusal::EnclosureUnavailable));
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
        Err(SsiRefusal::Hull(HullRefusal::EnclosureUnavailable))
    }
}

/// Certified range enclosure of a four-axis tensor polynomial over the box
/// whose axis intervals are unit-chart `[0,1]` subintervals.
///
/// Reduction is axis by axis by interval de Casteljau, exactly the outward-
/// rounded discipline of the landed `hull_bernstein_1d`/`_2d` kernels (each
/// coefficient widened to a point interval, every node step outward-rounded).
fn hull_tensor4(t: &Tensor4, box_axis: [(f64, f64); 4]) -> Result<CertifiedInterval, SsiRefusal> {
    for (lo, hi) in box_axis {
        if !lo.is_finite() || !hi.is_finite() || !(lo >= 0.0 && hi <= 1.0 && lo <= hi) {
            return Err(SsiRefusal::Hull(HullRefusal::DomainNotCompact));
        }
    }
    if t.rows.is_empty() || t.rows[0].is_empty() {
        return Err(SsiRefusal::Hull(HullRefusal::EnclosureUnavailable));
    }
    // Reduce axis 0 (u) over each flat column, yielding one interval per
    // (v, s, t) coefficient slot.
    let sp1 = t.row_spacing();
    let n1p1 = t.len_axis(1);
    let cols = t.rows[0].len();
    let u_iv = CertifiedInterval {
        lo: box_axis[0].0,
        hi: box_axis[0].1,
    };
    let u_len = t.len_axis(0);
    // u_cols[c][b]: axis-0 reduced value for each flat column, grouped by
    // column so the subsequent axis-1 reduction iterates columns.
    let mut u_cols = vec![Vec::<CertifiedInterval>::with_capacity(n1p1); cols];
    for b in 0..n1p1 {
        for (c, slot) in u_cols.iter_mut().enumerate() {
            let mut pts = Vec::with_capacity(u_len);
            for a in 0..u_len {
                pts.push(CertifiedInterval::point(t.rows[a * sp1 + b][c]));
            }
            slot.push(one_d_interval(&pts, &u_iv)?);
        }
    }
    // Reduce axis 1 (v) over b, yielding one interval per flat (s, t) column.
    let v_iv = CertifiedInterval {
        lo: box_axis[1].0,
        hi: box_axis[1].1,
    };
    let mut v_collapsed = Vec::with_capacity(cols);
    for col in u_cols {
        v_collapsed.push(one_d_interval(&col, &v_iv)?);
    }
    // Rebuild the (s, t) bivariate interval grid and bound it with the same
    // interval de Casteljau the landed 2D kernel uses for its second pass.
    let sp2 = t.col_spacing();
    let mut grid2: Vec<Vec<CertifiedInterval>> = Vec::with_capacity(v_collapsed.len() / sp2);
    for row_slice in v_collapsed.chunks(sp2) {
        grid2.push(row_slice.to_vec());
    }
    hull_2d_interval(&grid2, box_axis[2], box_axis[3])
}

/// Interval de Casteljau over the `(s, t)` box of an interval-valued
/// bivariate tensor grid (`grid[i][j]` = coefficient of `B^i_m(s) B^j_n(t)`).
fn hull_2d_interval(
    grid: &[Vec<CertifiedInterval>],
    s: (f64, f64),
    t: (f64, f64),
) -> Result<CertifiedInterval, SsiRefusal> {
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

// ---------------------------------------------------------------------------
// Chart ↔ unit mapping
// ---------------------------------------------------------------------------

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

/// The chart rectangles of a stored system, as eight bounds
/// `(u0,u1,v0,v1,s0,s1,t0,t1)`.
type ChartMap = (f64, f64, f64, f64, f64, f64, f64, f64);

/// Chart widths of the four axes from a domain map.
fn chart_widths(maps: ChartMap) -> [f64; 4] {
    [
        (maps.1 - maps.0).abs(),
        (maps.3 - maps.2).abs(),
        (maps.5 - maps.4).abs(),
        (maps.7 - maps.6).abs(),
    ]
}

/// The unit-chart image of a full trace box (chart coordinates).
fn unit_box(system: &SquareSystem3, box_: [(f64, f64); 4]) -> Result<[(f64, f64); 4], SsiRefusal> {
    let maps = system.domain_maps();
    let lo = [box_[0].0, box_[1].0, box_[2].0, box_[3].0];
    let hi = [box_[0].1, box_[1].1, box_[2].1, box_[3].1];
    let mlo = [maps.0, maps.2, maps.4, maps.6];
    let mhi = [maps.1, maps.3, maps.5, maps.7];
    let mut out = [(0.0f64, 0.0f64); 4];
    for a in 0..4 {
        match to_unit_interval(lo[a], hi[a], mlo[a], mhi[a]) {
            Some(unit) => out[a] = unit,
            None => return Err(SsiRefusal::Hull(HullRefusal::DomainNotCompact)),
        }
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Section 1 — F3 square reduction (certified diagonal derivatives + extents)
// ---------------------------------------------------------------------------

/// The certified partial-derivative enclosure of one stored component grid
/// along a chart axis over a trace box.
///
/// The box is given in the chart coordinates of the stored system (each axis
/// must be a compact subset of that axis's chart rectangle). The partial is a
/// Bernstein coefficient derivative in the unit chart, then scaled by the
/// inverse chart width so the result is the partial along the CHART
/// coordinate. `component` selects `x`, `y` or `z`; `axis` is 0..=3 in the
/// `(u,v,s,t)` order.
pub fn partial_enclosure(
    system: &SquareSystem3,
    component: usize,
    axis: usize,
    box_: [(f64, f64); 4],
) -> Result<CertifiedInterval, SsiRefusal> {
    if component > 2 || axis > 3 {
        return Err(SsiRefusal::InvalidInput);
    }
    let widths = chart_widths(system.domain_maps());
    let unit = unit_box(system, box_)?;
    let grid = &system.grids()[component];
    let tensor = Tensor4::from_grid(grid, system.degrees());
    let derived = tensor.partial_axis(axis)?;
    let hull = hull_tensor4(&derived, unit)?;
    let inv_width = CertifiedInterval::point(1.0).div(&CertifiedInterval::point(widths[axis]));
    match inv_width {
        Some(scale) => Ok(hull.mul(&scale)),
        None => Err(SsiRefusal::Hull(HullRefusal::EnclosureUnavailable)),
    }
}

/// Build the FROZEN [`SquareSystemInput`] of the reduced square system for a
/// candidate continuation axis over a trace box.
///
/// The reduced system's unknowns are the three chart axes other than
/// `continuation_axis`, in ascending order; its equations are the `F`
/// components in order. The `i`-th diagonal derivative is the certified
/// partial of component `i` along the `i`-th smallest retained axis (identity
/// pairing, exactly the fixture kit's documented convention); the `i`-th
/// extent is the box's extent along that retained axis.
pub fn f3_diagonal_derivatives(
    system: &SquareSystem3,
    continuation_axis: usize,
    box_: [(f64, f64); 4],
) -> Result<SquareSystemInput, SsiRefusal> {
    if continuation_axis > 3 {
        return Err(SsiRefusal::InvalidInput);
    }
    let retained: [usize; 3] = retained_axes(continuation_axis);
    let mut diagonal = Vec::with_capacity(3);
    for (i, axis) in retained.iter().enumerate() {
        let enc = partial_enclosure(system, i, *axis, box_)?;
        if !enc.is_finite() {
            return Err(SsiRefusal::Hull(HullRefusal::EnclosureUnavailable));
        }
        diagonal
            .push(IntervalEnclosure::new(enc.lo, enc.hi).map_err(|_| SsiRefusal::InvalidInput)?);
    }
    let mut extents = Vec::with_capacity(3);
    for axis in retained.iter() {
        let width = box_[*axis].1 - box_[*axis].0;
        extents.push(PositiveFinite::new(width).map_err(|_| SsiRefusal::InvalidInput)?);
    }
    Ok(SquareSystemInput {
        diagonal_derivatives: [diagonal[0], diagonal[1], diagonal[2]],
        extents: [extents[0], extents[1], extents[2]],
    })
}

/// Select the continuation coordinate by the FROZEN rule, verbatim.
///
/// Builds the certified [`SquareSystemInput`] from the system and box for the
/// given candidate continuation axis, then applies
/// `contract::select_continuation_coordinate` exactly: largest relative
/// margin, lowest index on ties, `ConditioningBelowThreshold` refuses, never
/// a weaker retry.
pub fn select_continuation_coordinate(
    system: &SquareSystem3,
    continuation_axis: usize,
    box_: [(f64, f64); 4],
) -> Result<crate::contract::ContinuationCoordinate, SsiRefusal> {
    let input = f3_diagonal_derivatives(system, continuation_axis, box_)?;
    crate::contract::select_continuation_coordinate(&input).map_err(SsiRefusal::from)
}

/// The retained axes (ascending) for a candidate continuation axis.
fn retained_axes(continuation_axis: usize) -> [usize; 3] {
    let mut out = [0usize; 3];
    let mut k = 0;
    for a in 0..4 {
        if a != continuation_axis {
            out[k] = a;
            k += 1;
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Section 2 — the 3×3 Krawczyk certificate
// ---------------------------------------------------------------------------

/// A 3×3 interval matrix.
type Matrix3 = [[CertifiedInterval; 3]; 3];

/// Determinant of a 3×3 interval matrix under directed rounding.
///
/// Co-factor expansion along the first row:
/// `a00·(a11a22 − a12a21) − a01·(a10a22 − a12a20) + a02·(a10a21 − a11a20)`.
fn det3(m: &Matrix3) -> CertifiedInterval {
    let a00 = &m[0][0];
    let a01 = &m[0][1];
    let a02 = &m[0][2];
    let a10 = &m[1][0];
    let a11 = &m[1][1];
    let a12 = &m[1][2];
    let a20 = &m[2][0];
    let a21 = &m[2][1];
    let a22 = &m[2][2];
    let t0 = a00.mul(&a11.mul(a22).sub(&a12.mul(a21)));
    let t1 = a01.mul(&a10.mul(a22).sub(&a12.mul(a20)));
    let t2 = a02.mul(&a10.mul(a21).sub(&a11.mul(a20)));
    t0.sub(&t1).add(&t2)
}

/// Adjugate of a 3×3 interval matrix under directed rounding.
fn adjugate3(m: &Matrix3) -> Matrix3 {
    [
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
    ]
}

/// Multiply a 3×3 interval matrix by a 3-vector of intervals.
fn matvec3(m: &Matrix3, v: &[CertifiedInterval; 3]) -> [CertifiedInterval; 3] {
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

/// 3×3 interval matrix product.
fn matmul3(a: &Matrix3, b: &Matrix3) -> Matrix3 {
    let mut out = [[CertifiedInterval::point(0.0); 3]; 3];
    for r in 0..3 {
        for c in 0..3 {
            let mut acc = CertifiedInterval::point(0.0);
            for k in 0..3 {
                acc = acc.add(&a[r][k].mul(&b[k][c]));
            }
            out[r][c] = acc;
        }
    }
    out
}

/// Certified value of one component at a chart point: hull over a degenerate
/// box (no differentiation).
fn value_at_point(
    system: &SquareSystem3,
    component: usize,
    point: [f64; 4],
) -> Result<CertifiedInterval, SsiRefusal> {
    let box_: [(f64, f64); 4] = [
        (point[0], point[0]),
        (point[1], point[1]),
        (point[2], point[2]),
        (point[3], point[3]),
    ];
    let unit = unit_box(system, box_)?;
    let tensor = Tensor4::from_grid(&system.grids()[component], system.degrees());
    let hull = hull_tensor4(&tensor, unit)?;
    if hull.is_finite() {
        Ok(hull)
    } else {
        Err(SsiRefusal::Hull(HullRefusal::EnclosureUnavailable))
    }
}

/// The reduced Jacobian of the slice system over the retained box.
///
/// The reduced square system `H(t)` is `F` with the continuation axis pinned
/// to `slice_value`; its Jacobian entries are the certified partials of the
/// three components along the three retained axes over the retained box `X`.
fn reduced_jacobian(
    system: &SquareSystem3,
    continuation_axis: usize,
    box_: [(f64, f64); 4],
) -> Result<(Matrix3, CertifiedInterval), SsiRefusal> {
    let retained = retained_axes(continuation_axis);
    let slice_value = (box_[continuation_axis].0 + box_[continuation_axis].1) / 2.0;
    let mut box4 = box_;
    box4[continuation_axis] = (slice_value, slice_value);
    let mut j = [[CertifiedInterval::point(0.0); 3]; 3];
    for (row, jrow) in j.iter_mut().enumerate() {
        for (col, cell) in jrow.iter_mut().enumerate() {
            let enc = partial_enclosure(system, row, retained[col], box4)?;
            if !enc.is_finite() {
                return Err(SsiRefusal::Hull(HullRefusal::EnclosureUnavailable));
            }
            *cell = enc;
        }
    }
    let det = det3(&j);
    if !det.is_finite() {
        return Err(SsiRefusal::Hull(HullRefusal::EnclosureUnavailable));
    }
    Ok((j, det))
}

/// Certify a unique root of the reduced square system on the slice
/// `{continuation axis = box centre}` within the retained box `X`.
///
/// Steps, in fail-closed order:
///
/// 1. The frozen F3 coordinate rule runs on the box; a conditioning refusal
///    is returned before any Krawczyk work.
/// 2. The reduced Jacobian minors are certified Bernstein-patch enclosures
///    over `X` ([`partial_enclosure`]); the determinant enclosure is their
///    directed-rounding composition. A determinant enclosure containing zero
///    is [`SsiRefusal::DeterminantSpansZero`] — the precondition is part of
///    the certificate's construction, never a later check.
/// 3. The inverse is the adjugate over the determinant under directed
///    rounding; the Krawczyk image is `x0 − C·H(x0) + (I − C·J)(X − x0)`.
/// 4. Only a component-wise STRICT inclusion emits a
///    [`KrawczykCertificate3`], through the shim's strict-inclusion-only
///    constructor. A boundary or reversed image is
///    [`SsiRefusal::InclusionNotStrict`].
///
/// The returned certificate's `box_x` is the retained box `X` (three axis
/// intervals in the chart coordinates of the input box), `k_x` the Krawczyk
/// image, and `det` the determinant enclosure (0 excluded).
pub fn krawczyk3_certificate(
    system: &SquareSystem3,
    continuation_axis: usize,
    box_: [(f64, f64); 4],
) -> Result<KrawczykCertificate3, SsiRefusal> {
    if continuation_axis > 3 {
        return Err(SsiRefusal::InvalidInput);
    }
    // Fail-closed ordering: coordinate rule first.
    select_continuation_coordinate(system, continuation_axis, box_)?;

    let retained = retained_axes(continuation_axis);
    let mut x_box = [(0.0f64, 0.0f64); 3];
    for (k, axis) in retained.iter().enumerate() {
        x_box[k] = box_[*axis];
    }

    let (j, det) = reduced_jacobian(system, continuation_axis, box_)?;
    // Precondition of construction: the determinant enclosure must exclude 0.
    if det.lo <= 0.0 && det.hi >= 0.0 {
        return Err(SsiRefusal::DeterminantSpansZero);
    }

    // Centre of the retained box (chart coordinates).
    let mut x0 = [0.0f64; 3];
    for (k, (lo, hi)) in x_box.iter().enumerate() {
        x0[k] = (lo + hi) / 2.0;
    }
    let slice_value = (box_[continuation_axis].0 + box_[continuation_axis].1) / 2.0;

    // H(x0): the slice-system value at the retained centre.
    let mut point = [0.0f64; 4];
    for (k, axis) in retained.iter().enumerate() {
        point[*axis] = x0[k];
    }
    point[continuation_axis] = slice_value;
    let mut h0 = [CertifiedInterval::point(0.0); 3];
    for (c, cell) in h0.iter_mut().enumerate() {
        *cell = value_at_point(system, c, point)?;
    }

    // Interval Jacobian at the centre (degenerate box) for the preconditioner.
    let mut center_box = box_;
    center_box[continuation_axis] = (slice_value, slice_value);
    for (k, axis) in retained.iter().enumerate() {
        center_box[*axis] = (x0[k], x0[k]);
    }
    let mut j0 = [[CertifiedInterval::point(0.0); 3]; 3];
    for (row, jrow) in j0.iter_mut().enumerate() {
        for (col, cell) in jrow.iter_mut().enumerate() {
            *cell = partial_enclosure(system, row, retained[col], center_box)?;
        }
    }

    // Inverse via adjugate over determinant (directed rounding).
    let adj = adjugate3(&j0);
    let det0 = det3(&j0);
    if !det0.is_finite() || (det0.lo <= 0.0 && det0.hi >= 0.0) {
        return Err(SsiRefusal::DeterminantSpansZero);
    }
    let mut c = [[CertifiedInterval::point(0.0); 3]; 3];
    for (r, crow) in c.iter_mut().enumerate() {
        for (ccol, cell) in crow.iter_mut().enumerate() {
            match adj[r][ccol].div(&det0) {
                Some(v) => *cell = v,
                None => return Err(SsiRefusal::Hull(HullRefusal::EnclosureUnavailable)),
            }
        }
    }

    // I − C·J over the retained box.
    let cj = matmul3(&c, &j);
    let id_minus_cj = [
        [
            CertifiedInterval::point(1.0).sub(&cj[0][0]),
            cj[0][1].neg(),
            cj[0][2].neg(),
        ],
        [
            cj[1][0].neg(),
            CertifiedInterval::point(1.0).sub(&cj[1][1]),
            cj[1][2].neg(),
        ],
        [
            cj[2][0].neg(),
            cj[2][1].neg(),
            CertifiedInterval::point(1.0).sub(&cj[2][2]),
        ],
    ];

    // x0 as point intervals; dx = X − x0 outward rounded.
    let x0iv = [
        CertifiedInterval::point(x0[0]),
        CertifiedInterval::point(x0[1]),
        CertifiedInterval::point(x0[2]),
    ];
    let mut dx = [CertifiedInterval::point(0.0); 3];
    for (k, (lo, hi)) in x_box.iter().enumerate() {
        let d_lo = CertifiedInterval::point(*lo).sub(&x0iv[k]);
        let d_hi = CertifiedInterval::point(*hi).sub(&x0iv[k]);
        dx[k] = CertifiedInterval {
            lo: d_lo.lo.min(d_hi.lo),
            hi: d_lo.hi.max(d_hi.hi),
        };
    }

    let ch = matvec3(&c, &h0);
    let md = matvec3(&id_minus_cj, &dx);
    let k = [
        x0iv[0].sub(&ch[0]).add(&md[0]),
        x0iv[1].sub(&ch[1]).add(&md[1]),
        x0iv[2].sub(&ch[2]).add(&md[2]),
    ];

    let mut k_pairs = [(0.0f64, 0.0f64); 3];
    for (axis, kv) in k.iter().enumerate() {
        if !kv.is_finite() {
            return Err(SsiRefusal::Hull(HullRefusal::EnclosureUnavailable));
        }
        k_pairs[axis] = (kv.lo, kv.hi);
    }

    // Emission through the shim's strict-inclusion-only constructor.
    KrawczykCertificate3::new(x_box, k_pairs, (det.lo, det.hi)).map_err(|_| {
        // The shim refuses a non-strict / boundary / reversed inclusion or a
        // misordered enclosure. Finiteness and det were pre-checked, so a
        // refusal here is the strict-inclusion precondition.
        SsiRefusal::InclusionNotStrict
    })
}

// ---------------------------------------------------------------------------
// Section 1 — square-system construction from two certified-admitted patches
// ---------------------------------------------------------------------------

/// A certified-admitted rational tensor-Bernstein patch (spline-admissible).
///
/// Bidegree `(m, n)` over the unit square; the homogeneous numerator
/// `num[k][a][b]` and the weight coefficient grid `w[a][b]` are both
/// `(m+1) × (n+1)` control grids (rows index the first parameter). The
/// positive weight certificate is an input (carried here as the strictly
/// positive finite weight grid); it is never re-derived.
#[derive(Debug, Clone)]
pub struct RationalBipatch {
    m: usize,
    n: usize,
    /// Homogeneous numerator control grids, `(x, y, z)` order.
    num: [Vec<Vec<f64>>; 3],
    /// Weight coefficient grid, strictly positive and finite.
    w: Vec<Vec<f64>>,
}

impl RationalBipatch {
    /// Construct a patch, refusing a degree-0 bidegree, empty or ragged
    /// grids, non-finite coefficients, or a non-positive weight.
    pub fn new(
        m: usize,
        n: usize,
        num: [Vec<Vec<f64>>; 3],
        w: Vec<Vec<f64>>,
    ) -> Result<Self, SsiRefusal> {
        if m == 0 || n == 0 {
            return Err(SsiRefusal::InvalidInput);
        }
        let shape_ok = |g: &[Vec<f64>]| {
            g.len() == m + 1
                && g.iter()
                    .all(|row| row.len() == n + 1 && row.iter().all(|c| c.is_finite()))
        };
        if !shape_ok(&num[0]) || !shape_ok(&num[1]) || !shape_ok(&num[2]) || !shape_ok(&w) {
            return Err(SsiRefusal::InvalidInput);
        }
        if w.iter().any(|row| row.iter().any(|c| *c <= 0.0)) {
            return Err(SsiRefusal::InvalidInput);
        }
        Ok(RationalBipatch { m, n, num, w })
    }

    /// Bidegree in the first parameter.
    pub fn m(&self) -> usize {
        self.m
    }

    /// Bidegree in the second parameter.
    pub fn n(&self) -> usize {
        self.n
    }

    /// The homogeneous numerator grids, `(x, y, z)` order.
    pub fn numerator(&self) -> &[Vec<Vec<f64>>; 3] {
        &self.num
    }

    /// The strictly positive weight grid.
    pub fn weights(&self) -> &[Vec<f64>] {
        &self.w
    }
}

/// One side of a square-system construction.
#[derive(Debug, Clone)]
pub enum SsiParticipant {
    /// A certified-admitted rational tensor-Bernstein patch (spline-admissible).
    RationalBipatch(RationalBipatch),
    /// Any non-spline surface shape (a DISPATCH-routed analytic class). The
    /// generic SSI engine refuses such a pair.
    NonSpline,
}

/// Construct the square surface–surface difference system from two
/// certified-admitted patches.
///
/// Class pairs outside the spline-admissible shapes refuse
/// [`SsiRefusal::PairClass`] with the DISPATCH widening
/// [`PairUnsupported::UnsupportedPairClass`] (a named variant, never a
/// string). For two rational patches the cross-multiplied coefficient grid of
/// component `k` is, at flat index `(a,b,i,j)`,
///
/// ```text
/// W2[i][j]·N1_k[a][b] − W1[a][b]·N2_k[i][j]
/// ```
///
/// stored through the shim's refusing `SquareSystem3::new` (ragged / empty /
/// non-finite / degree-0 refusal is the shim's; this function feeds it). The
/// two patches share the unit chart, so the stored domain maps are the
/// identity rectangle `(0,1,0,1,0,1,0,1)`.
pub fn construct_square_system(
    lhs: &SsiParticipant,
    rhs: &SsiParticipant,
) -> Result<SquareSystem3, SsiRefusal> {
    let p1 = match lhs {
        SsiParticipant::RationalBipatch(p) => p,
        SsiParticipant::NonSpline => {
            return Err(SsiRefusal::PairClass(PairUnsupported::UnsupportedPairClass));
        }
    };
    let p2 = match rhs {
        SsiParticipant::RationalBipatch(p) => p,
        SsiParticipant::NonSpline => {
            return Err(SsiRefusal::PairClass(PairUnsupported::UnsupportedPairClass));
        }
    };
    let (m1, n1) = (p1.m(), p1.n());
    let (m2, n2) = (p2.m(), p2.n());
    let rows = (m1 + 1) * (n1 + 1);
    let cols = (m2 + 1) * (n2 + 1);
    let mut grids = [
        vec![vec![0.0f64; cols]; rows],
        vec![vec![0.0f64; cols]; rows],
        vec![vec![0.0f64; cols]; rows],
    ];
    for (k, grid_k) in grids.iter_mut().enumerate() {
        for a in 0..=m1 {
            for b in 0..=n1 {
                let row = a * (n1 + 1) + b;
                let w1 = p1.weights()[a][b];
                let n1k = p1.numerator()[k][a][b];
                for i in 0..=m2 {
                    for j in 0..=n2 {
                        let col = i * (n2 + 1) + j;
                        let w2 = p2.weights()[i][j];
                        let n2k = p2.numerator()[k][i][j];
                        grid_k[row][col] = w2 * n1k - w1 * n2k;
                    }
                }
            }
        }
    }
    let identity = (0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0);
    SquareSystem3::new(grids, (m1, n1, m2, n2), identity).map_err(SsiRefusal::from)
}
