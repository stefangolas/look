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
//! over the product chart `(u,v) × (s,t)`, exactly as the shim froze it.
//! CFP-003-SEPARABILITY stores the system PER SIDE ([`SquareSystem3`] keeps
//! the two carriers, never the materialized four-axis coefficient grid);
//! [`construct_square_system`] builds the two per-side carriers from the
//! patches and feeds the shim's refusing `SquareSystem3::from_sides`; ragged /
//! empty / non-finite / degree-0 refusal is the shim's, never restated here.
//! Every certified enclosure is the interval product / difference of per-side
//! 2-D hulls over each carrier's box (spec Corollaries 1.1–1.2, O(deg²) per
//! cell). A materialized view of the component grids stays available through
//! [`SquareSystem3::grids`] for the pre-CFP whole-grid consumers, computed
//! from the per-side carriers on first demand.
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
//! 3D box `X`: the Jacobian minors are certified per-side 2-D hulls composed
//! by Corollaries 1.1–1.2 (the landed `CertifiedInterval` de-Casteljau
//! discipline, per carrier), the inverse is the adjugate over the determinant
//! under directed rounding, and only a STRICT inclusion `K(X) ⊂ int(X)` emits
//! a [`KrawczykCertificate3`] through the shim's strict-inclusion-only
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
use crate::ssi_types::{KrawczykCertificate3, SideCarrier, SquareSystem3};

/// Why an SSI square-system or Krawczyk3 operation could not be certified.
///
/// Named cases only — no catch-all — matching the refusal shape of the rest
/// of the crate. Each variant wraps a landed named cause verbatim (D-reuse):
/// a class-pair refusal carries [`PairUnsupported`], a conditioning refusal
/// carries [`Refusal`], and a hull failure carries [`HullRefusal`]. The two
/// certificate preconditions (`DeterminantSpansZero`, `InclusionNotStrict`)
/// are this module's own named cases, mirroring `bezier_isect`'s
/// typed-unresolved discipline.
///
/// FSSI-000 (FSSI topology contract) adds the two positive-dimension
/// suspicion halts of the FSSI-001 gate as named cases here —
/// [`SsiRefusal::TangentCurveSuspected`] and
/// [`SsiRefusal::CoincidentPatchSuspected`]. They are refusals (the gate
/// could not decide the box), never a wrong acceptance; the SSI-layer `tag()`
/// carries the specificity the FSSI-001/002 producers assert against, and
/// downstream layers fold them onto their existing low-information refusal
/// shapes. `Eq` is intentionally absent: the new payloads carry `(f64, f64)`
/// evidence, and nothing in the crate uses these refusals as map keys.
#[derive(Debug, Clone, Copy, PartialEq)]
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
    /// The FSSI-001 positive-dimension suspicion halt: the separable
    /// tangency-free gate could not decide the box (its undecided measure
    /// failing to shrink like `2^-4k` at budget exhaustion), so a tangent
    /// curve is suspected. Carries the undecided measure and its shrink
    /// exponent at halt. Evidence, never a tolerance. Cannot fire until
    /// FSSI-001 wires the producer; the arm documents that.
    TangentCurveSuspected {
        /// The undecided measure and its shrink exponent at halt (evidence,
        /// never a tolerance).
        margin: (f64, f64),
    },
    /// The FSSI-001 coincident-carrier suspicion halt: the separable
    /// tangency-free gate could not decide the box at area scale, so a
    /// coincident patch is suspected. Carries the undecided measure and its
    /// shrink exponent at halt. Evidence, never a tolerance. Cannot fire
    /// until FSSI-001 wires the producer; the arm documents that.
    CoincidentPatchSuspected {
        /// The undecided measure and its shrink exponent at halt (evidence,
        /// never a tolerance).
        margin: (f64, f64),
    },
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
            Self::TangentCurveSuspected { .. } => "ssi_tangent_curve_suspected",
            Self::CoincidentPatchSuspected { .. } => "ssi_coincident_patch_suspected",
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
// The FSSI fold-verdict carrier (FSSI-000 contract; FSSI-002 populates).
// ---------------------------------------------------------------------------

/// The refusing-constructor carrier of a certified ordinary fold (FSSI-000
/// scope decision 1; the FSSI-002 fold packet populates it as a NEW
/// escalation-lattice tier registered in `docs/CERTIFICATE_MAPPING.md`).
///
/// D-shim: a type and a refusing constructor only — nothing here evaluates,
/// solves, isolates, or certifies numerically. The record carries the chart
/// whose fold system `E_j = (F, q_j)` certified, the recovered fold sign
/// `σ ∈ {−1, +1}`, and the certified determinant enclosure
/// `D(F, q_j)` evidence of the proof. CFP-008 stagnation verdicts stand
/// unchanged; a genuinely degenerate fold stays a landed refusal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FoldCert {
    /// The chart index whose fold system certified the ordinary fold.
    pub chart: usize,
    /// The recovered fold sign `σ`, `+1` or `−1`.
    pub sigma: i8,
    /// The certified determinant enclosure of the fold proof, a finite,
    /// ordered `(lo, hi)` evidence pair (never a tolerance).
    pub det_enclosure: (f64, f64),
}

impl FoldCert {
    /// Construct a fold-verdict record, refusing a malformed one: a `sigma`
    /// outside `{−1, +1}` or a non-finite / misordered `det_enclosure`.
    pub fn new(chart: usize, sigma: i8, det_enclosure: (f64, f64)) -> Result<Self, SsiRefusal> {
        let (lo, hi) = det_enclosure;
        if !lo.is_finite() || !hi.is_finite() || lo > hi {
            return Err(SsiRefusal::InvalidInput);
        }
        if !matches!(sigma, -1 | 1) {
            return Err(SsiRefusal::InvalidInput);
        }
        Ok(FoldCert {
            chart,
            sigma,
            det_enclosure,
        })
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
    /// the shim's `SquareSystem3`).
    #[allow(dead_code)] // retained as the materialized-grid test substrate (per-side vs grid-hull set-identity tests, Cor 1.1)
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

    /// A zero coefficient grid of the given degrees.
    #[allow(dead_code)] // CTE-003 substrate, exercised by the ssi second-partial tests
    fn zero_tensor(degrees: (usize, usize, usize, usize)) -> Self {
        let (m1, n1, m2, n2) = degrees;
        let rows = (m1 + 1) * (n1 + 1);
        let cols = (m2 + 1) * (n2 + 1);
        Tensor4 {
            degrees,
            rows: vec![vec![0.0f64; cols]; rows],
        }
    }

    /// The second-partial coefficient grid along the chart axes `j` then `l`.
    ///
    /// The Bernstein second-derivative rule `d·(d−1)·(c[k+2] − 2c[k+1] + c[k])`
    /// per axis pair, computed exactly as the kernel engine's private
    /// `grid_second_partial` (engine.rs:1128) — first partial along `j`, then
    /// along `l`. The pattern is adapted, not imported (V5 guard). A linear
    /// axis (the double derivative along it would refuse a degree-0 axis) is
    /// the identically-zero polynomial, represented as a zero grid of the
    /// first partial's degrees.
    #[allow(dead_code)] // CTE-003 substrate, exercised by the ssi second-partial tests
    fn partial2_axis(&self, j: usize, l: usize) -> Result<Tensor4, SsiRefusal> {
        if j > 3 || l > 3 {
            return Err(SsiRefusal::InvalidInput);
        }
        let first = self.partial_axis(j)?;
        if l == j && first.len_axis(j) == 1 {
            // The axis is linear after one partial: the second derivative
            // along it is identically zero, and a further partial would
            // refuse the degree-0 axis.
            return Ok(Self::zero_tensor(first.degrees));
        }
        first.partial_axis(l)
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
#[allow(dead_code)] // retained as the materialized-grid hull reference for the per-side set-identity test (Cor 1.1)
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
#[allow(dead_code)] // retained as part of the materialized-grid hull reference (hull_tensor4), exercised by the Cor 1.1 set-identity test
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
// Per-side 2-D hull kernels (CFP-003-SEPARABILITY, spec Corollaries 1.1-1.2)
// ---------------------------------------------------------------------------

/// The certified range enclosure of one side's bivariate Bernstein grid over a
/// UNIT-chart sub-box (each axis a compact subinterval of `[0, 1]`).
///
/// Reduction is axis-by-axis interval de Casteljau (the outward-rounded
/// discipline of the landed hull kernels). `grid` has `dm + 1` rows (first
/// parameter) and `dn + 1` columns (second parameter).
fn hull_bivariate(
    grid: &[Vec<f64>],
    dm: usize,
    dn: usize,
    u: (f64, f64),
    v: (f64, f64),
) -> Result<CertifiedInterval, SsiRefusal> {
    if grid.len() != dm + 1 || grid.iter().any(|row| row.len() != dn + 1) {
        return Err(SsiRefusal::Hull(HullRefusal::EnclosureUnavailable));
    }
    for (lo, hi) in [u, v] {
        if !lo.is_finite() || !hi.is_finite() || !(lo >= 0.0 && hi <= 1.0 && lo <= hi) {
            return Err(SsiRefusal::Hull(HullRefusal::DomainNotCompact));
        }
    }
    let u_iv = CertifiedInterval { lo: u.0, hi: u.1 };
    let v_iv = CertifiedInterval { lo: v.0, hi: v.1 };
    // Transpose to column-major: one interval per second-parameter slot after
    // reducing the first parameter over each column of coefficients.
    let mut columns = vec![Vec::<CertifiedInterval>::with_capacity(dm + 1); dn + 1];
    for row in grid.iter().take(dm + 1) {
        for (c, value) in row.iter().enumerate() {
            columns[c].push(CertifiedInterval::point(*value));
        }
    }
    let mut per_column = Vec::with_capacity(dn + 1);
    for column in columns {
        per_column.push(one_d_interval(&column, &u_iv)?);
    }
    // Reduce the second parameter.
    let hull = one_d_interval(&per_column, &v_iv)?;
    if hull.is_finite() {
        Ok(hull)
    } else {
        Err(SsiRefusal::Hull(HullRefusal::EnclosureUnavailable))
    }
}

/// The Bernstein first-derivative coefficient grid of one side's bivariate
/// grid along its first (`axis == 0`) or second (`axis == 1`) parameter.
fn bivariate_derivative(
    grid: &[Vec<f64>],
    dm: usize,
    dn: usize,
    axis: usize,
) -> Result<Vec<Vec<f64>>, SsiRefusal> {
    if grid.len() != dm + 1 || grid.iter().any(|row| row.len() != dn + 1) {
        return Err(SsiRefusal::Hull(HullRefusal::EnclosureUnavailable));
    }
    match axis {
        0 => {
            if dm == 0 {
                return Err(SsiRefusal::InvalidInput);
            }
            let scale = dm as f64;
            let mut out = vec![vec![0.0f64; dn + 1]; dm];
            for a in 0..dm {
                for b in 0..=dn {
                    out[a][b] = scale * (grid[a + 1][b] - grid[a][b]);
                }
            }
            Ok(out)
        }
        1 => {
            if dn == 0 {
                return Err(SsiRefusal::InvalidInput);
            }
            let scale = dn as f64;
            let mut out = vec![vec![0.0f64; dn]; dm + 1];
            for a in 0..=dm {
                for b in 0..dn {
                    out[a][b] = scale * (grid[a][b + 1] - grid[a][b]);
                }
            }
            Ok(out)
        }
        _ => Err(SsiRefusal::InvalidInput),
    }
}

/// The certified range enclosure of one component of the stored difference
/// system over a UNIT-chart product box, computed PER SIDE.
///
/// `F_k = W2·N1_k − W1·N2_k` with the two terms' functions on disjoint
/// parameter sets: the range of `W2(s,t)·N1_k(u,v)` over `B1 x B2` is exactly
/// the product of the per-side ranges, and the range of the difference is the
/// interval difference (spec Corollary 1.1; set-identical to the materialized
/// four-axis hull, computed at O(deg²)).
fn component_value_unit(
    system: &SquareSystem3,
    component: usize,
    unit: [(f64, f64); 4],
) -> Result<CertifiedInterval, SsiRefusal> {
    if component > 2 {
        return Err(SsiRefusal::InvalidInput);
    }
    let side_a = system.side_a();
    let side_b = system.side_b();
    let (ma, na) = (side_a.m(), side_a.n());
    let (mb, nb) = (side_b.m(), side_b.n());
    let ha_num = hull_bivariate(&side_a.numerator()[component], ma, na, unit[0], unit[1])?;
    let ha_w = hull_bivariate(side_a.weights(), ma, na, unit[0], unit[1])?;
    let hb_num = hull_bivariate(&side_b.numerator()[component], mb, nb, unit[2], unit[3])?;
    let hb_w = hull_bivariate(side_b.weights(), mb, nb, unit[2], unit[3])?;
    // (W2 over B2)·(N1_k over B1) − (W1 over B1)·(N2_k over B2).
    let term1 = hb_w.mul(&ha_num);
    let term2 = ha_w.mul(&hb_num);
    let hull = term1.sub(&term2);
    if hull.is_finite() {
        Ok(hull)
    } else {
        Err(SsiRefusal::Hull(HullRefusal::EnclosureUnavailable))
    }
}

/// The certified partial-derivative enclosure of one component along one chart
/// axis over a UNIT-chart product box, computed PER SIDE.
///
/// `∂_u F_k = W2·∂_u N1_k − ∂_u W1·N2_k`, `∂_v` likewise on side A; `∂_s
/// F_k = ∂_s W2·N1_k − W1·∂_s N2_k`, `∂_t` likewise on side B. Each product
/// term hulls on disjoint parameter sets (spec Corollary 1.2 — every partial
/// belongs to one carrier, no mixed-derivative grid).
fn component_partial_unit(
    system: &SquareSystem3,
    component: usize,
    axis: usize,
    unit: [(f64, f64); 4],
) -> Result<CertifiedInterval, SsiRefusal> {
    if component > 2 || axis > 3 {
        return Err(SsiRefusal::InvalidInput);
    }
    let side_a = system.side_a();
    let side_b = system.side_b();
    let (ma, na) = (side_a.m(), side_a.n());
    let (mb, nb) = (side_b.m(), side_b.n());
    let ha_num = hull_bivariate(&side_a.numerator()[component], ma, na, unit[0], unit[1])?;
    let ha_w = hull_bivariate(side_a.weights(), ma, na, unit[0], unit[1])?;
    let hb_num = hull_bivariate(&side_b.numerator()[component], mb, nb, unit[2], unit[3])?;
    let hb_w = hull_bivariate(side_b.weights(), mb, nb, unit[2], unit[3])?;
    let hull = match axis {
        0 | 1 => {
            // Side A: ∂ F_k = W2·∂ N1_k − ∂ W1·N2_k.
            let du = bivariate_derivative(&side_a.numerator()[component], ma, na, axis)?;
            let dw = bivariate_derivative(side_a.weights(), ma, na, axis)?;
            let (d_ma, d_na) = if axis == 0 {
                (ma - 1, na)
            } else {
                (ma, na - 1)
            };
            let h_du = hull_bivariate(&du, d_ma, d_na, unit[0], unit[1])?;
            let h_dw = hull_bivariate(&dw, d_ma, d_na, unit[0], unit[1])?;
            hb_w.mul(&h_du).sub(&h_dw.mul(&hb_num))
        }
        _ => {
            // Side B: ∂ F_k = ∂ W2·N1_k − W1·∂ N2_k.
            let side_axis = axis - 2;
            let du = bivariate_derivative(&side_b.numerator()[component], mb, nb, side_axis)?;
            let dw = bivariate_derivative(side_b.weights(), mb, nb, side_axis)?;
            let (d_mb, d_nb) = if side_axis == 0 {
                (mb - 1, nb)
            } else {
                (mb, nb - 1)
            };
            let h_du = hull_bivariate(&du, d_mb, d_nb, unit[2], unit[3])?;
            let h_dw = hull_bivariate(&dw, d_mb, d_nb, unit[2], unit[3])?;
            h_dw.mul(&ha_num).sub(&ha_w.mul(&h_du))
        }
    };
    if hull.is_finite() {
        Ok(hull)
    } else {
        Err(SsiRefusal::Hull(HullRefusal::EnclosureUnavailable))
    }
}

// ---------------------------------------------------------------------------
// Section 1 — F3 square reduction (certified diagonal derivatives + extents)
// ---------------------------------------------------------------------------

/// The certified partial-derivative enclosure of one stored component grid
/// along a chart axis over a trace box.
///
/// The box is given in the chart coordinates of the stored system (each axis
/// must be a compact subset of that axis's chart rectangle). The partial is
/// hulled per side in the unit chart (spec Corollary 1.2 — the derivative
/// belongs to one carrier), then scaled by the inverse chart width so the
/// result is the partial along the CHART coordinate. `component` selects `x`,
/// `y` or `z`; `axis` is 0..=3 in the `(u,v,s,t)` order.
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
    let hull = component_partial_unit(system, component, axis, unit)?;
    let inv_width = CertifiedInterval::point(1.0).div(&CertifiedInterval::point(widths[axis]));
    match inv_width {
        Some(scale) => Ok(hull.mul(&scale)),
        None => Err(SsiRefusal::Hull(HullRefusal::EnclosureUnavailable)),
    }
}

// ---------------------------------------------------------------------------
// Per-side normal nets and the Corollary 1.4 hull memo (CFP-003)
// ---------------------------------------------------------------------------

/// A small integer binomial coefficient as `f64`.
#[allow(dead_code)] // CFP-003 per-side substrate: Bernstein product weights (normal-net composition, Prop 2.1), exercised by the normal-net/memo tests
fn binom_f(n: usize, k: usize) -> f64 {
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

/// The Bernstein product weight of one axis: `C(da,i)·C(db,j)/C(da+db,k)`
/// with `i + j = k`.
#[allow(dead_code)] // CFP-003 per-side substrate: exercised by the normal-net composition tests
fn product_weight(da: usize, db: usize, i: usize, j: usize, k: usize) -> f64 {
    binom_f(da, i) * binom_f(db, j) / binom_f(da + db, k)
}

/// The Bernstein product of two bivariate grids over the shared unit square:
/// per-axis degree sums with the Bernstein convolution weights
/// `C(da,i)·C(db,j)/C(da+db,k)` on each axis (the exact coefficient algebra
/// behind the per-side normal-net composition, spec Prop. 2.1 scope decision
/// 2). `a` has bidegree `(ma, na)`, `b` `(mb, nb)`.
#[allow(dead_code)]
// CFP-003 per-side substrate: normal-net composition + Cor 1.3 tests; consumed by CFP-005 (bvh) per the build-spec packet row
#[allow(clippy::needless_range_loop)] // the Bernstein-convolution index algebra over the (ka,kb,a0,a1,b0,b1) index lattice is clearest in the explicit range form
fn bernstein_mul_2d(
    a: &[Vec<f64>],
    (ma, na): (usize, usize),
    b: &[Vec<f64>],
    (mb, nb): (usize, usize),
) -> Result<Vec<Vec<f64>>, SsiRefusal> {
    if a.len() != ma + 1 || b.len() != mb + 1 {
        return Err(SsiRefusal::Hull(HullRefusal::EnclosureUnavailable));
    }
    if a.iter().any(|r| r.len() != na + 1) || b.iter().any(|r| r.len() != nb + 1) {
        return Err(SsiRefusal::Hull(HullRefusal::EnclosureUnavailable));
    }
    let (om, on) = (ma + mb, na + nb);
    let mut out = vec![vec![0.0f64; on + 1]; om + 1];
    for ka in 0..=om {
        for kb in 0..=on {
            let mut acc = 0.0f64;
            for a0 in 0..=ma.min(ka) {
                let a1 = ka - a0;
                if a1 > mb {
                    continue;
                }
                let w0 = product_weight(ma, mb, a0, a1, ka);
                for b0 in 0..=na.min(kb) {
                    let b1 = kb - b0;
                    if b1 > nb {
                        continue;
                    }
                    let w1 = product_weight(na, nb, b0, b1, kb);
                    acc += w0 * w1 * a[a0][b0] * b[a1][b1];
                }
            }
            out[ka][kb] = acc;
        }
    }
    Ok(out)
}

/// The per-side unit normal net of one carrier under D2 (unit weights): the
/// coefficient grids of `n = ∂_u S × ∂_v S`, composed per patch by exact
/// Bernstein coefficient products (spec Prop. 2.1, scope decision 2 — the
/// normal is enclosed by its OWN net of bidegree `(2m−1, 2n−1)`, `4mn`
/// coefficients per component, never as interval cross products of derivative
/// hulls). Returns the three component grids in `(x, y, z)` order.
#[allow(dead_code)] // CFP-003 per-side substrate: D3-counted normal nets (Prop 2.1), exercised by the normal-net tests; the count feeds the D3 budget note
fn side_normal_nets(side: &SideCarrier) -> Result<[Vec<Vec<f64>>; 3], SsiRefusal> {
    let (m, n) = (side.m(), side.n());
    // ∂_u has bidegree (m−1, n), ∂_v bidegree (m, n−1); the cross product has
    // bidegree (2m−1, 2n−1).
    let mut du = [Vec::new(), Vec::new(), Vec::new()];
    let mut dv = [Vec::new(), Vec::new(), Vec::new()];
    for k in 0..3 {
        du[k] = bivariate_derivative(&side.numerator()[k], m, n, 0)?;
        dv[k] = bivariate_derivative(&side.numerator()[k], m, n, 1)?;
    }
    let du_deg = (m - 1, n);
    let dv_deg = (m, n - 1);
    // n = ∂_u × ∂_v: n_x = du_y·dv_z − du_z·dv_y, etc.
    let x = bernstein_mul_2d(&du[1], du_deg, &dv[2], dv_deg)?;
    let x_sub = bernstein_mul_2d(&du[2], du_deg, &dv[1], dv_deg)?;
    let y = bernstein_mul_2d(&du[2], du_deg, &dv[0], dv_deg)?;
    let y_sub = bernstein_mul_2d(&du[0], du_deg, &dv[2], dv_deg)?;
    let z = bernstein_mul_2d(&du[0], du_deg, &dv[1], dv_deg)?;
    let z_sub = bernstein_mul_2d(&du[1], du_deg, &dv[0], dv_deg)?;
    let sub = |a: Vec<Vec<f64>>, b: Vec<Vec<f64>>| -> Result<Vec<Vec<f64>>, SsiRefusal> {
        if a.len() != b.len() {
            return Err(SsiRefusal::Hull(HullRefusal::EnclosureUnavailable));
        }
        let mut out = Vec::with_capacity(a.len());
        for (ra, rb) in a.iter().zip(b.iter()) {
            if ra.len() != rb.len() {
                return Err(SsiRefusal::Hull(HullRefusal::EnclosureUnavailable));
            }
            out.push(ra.iter().zip(rb.iter()).map(|(u, v)| u - v).collect());
        }
        Ok(out)
    };
    Ok([sub(x, x_sub)?, sub(y, y_sub)?, sub(z, z_sub)?])
}

/// A deterministic memoization key: one side's unit box (spec Corollary 1.4).
///
/// A side box is the two unit-chart intervals of one carrier plus the function
/// being hulled (one of the three numerator components, or the weight grid).
/// The interval endpoints are stored as exact `f64` bit patterns so that two
/// float-identical boxes collide and the key order is total (unit coordinates
/// in `[0, 1]` are positive, so `to_bits` is monotone in value).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[allow(dead_code)] // CFP-003 per-side substrate: memo key exercised by the memoization test (Cor 1.4)
pub(crate) struct SideBoxKey {
    /// Which carrier (0 = side A, 1 = side B).
    side: u8,
    /// Which grid on that side is hulled (0..2 numerator component, 3 weight).
    kind: u8,
    /// First-axis unit interval, `(lo, hi)` bit patterns.
    axis0: (u64, u64),
    /// Second-axis unit interval, `(lo, hi)` bit patterns.
    axis1: (u64, u64),
}

/// The deterministic per-side hull memo (spec Corollary 1.4): hulls keyed on
/// (side, box, function), with no eviction — the subdivision visit order is
/// fixed, so a subdivision visiting `M` cells drawn from `k` distinct per-side
/// boxes pays `O(k)` hulls per side plus `O(M)` interval arithmetic on cached
/// values.
#[derive(Debug, Clone, Default)]
#[allow(dead_code)] // CFP-003 per-side substrate: memoized per-side hulls (Cor 1.4), exercised by the memoization test
pub(crate) struct PerSideHullMemo {
    cache: std::collections::BTreeMap<SideBoxKey, CertifiedInterval>,
}

impl PerSideHullMemo {
    /// The cached certified hull of one function on one side over a unit box,
    /// computing (and counting) it only on a first visit.
    fn hull_side_box(
        &mut self,
        system: &SquareSystem3,
        side: usize,
        kind: usize,
        box2: [(f64, f64); 2],
    ) -> Result<CertifiedInterval, SsiRefusal> {
        if side > 1 || kind > 3 {
            return Err(SsiRefusal::InvalidInput);
        }
        for (lo, hi) in box2 {
            if !lo.is_finite() || !hi.is_finite() || !(lo >= 0.0 && hi <= 1.0 && lo <= hi) {
                return Err(SsiRefusal::Hull(HullRefusal::DomainNotCompact));
            }
        }
        let key = SideBoxKey {
            side: side as u8,
            kind: kind as u8,
            axis0: (box2[0].0.to_bits(), box2[0].1.to_bits()),
            axis1: (box2[1].0.to_bits(), box2[1].1.to_bits()),
        };
        if let Some(hit) = self.cache.get(&key) {
            return Ok(*hit);
        }
        let carrier = if side == 0 {
            system.side_a()
        } else {
            system.side_b()
        };
        let (m, n) = (carrier.m(), carrier.n());
        let grid = if kind == 3 {
            carrier.weights()
        } else {
            &carrier.numerator()[kind]
        };
        let hull = hull_bivariate(grid, m, n, box2[0], box2[1])?;
        self.cache.insert(key, hull);
        Ok(hull)
    }

    /// The memoized per-side value enclosure of one component over a unit
    /// product box (the D2 cross-multiplied form of spec Corollary 1.1).
    #[allow(dead_code)] // CFP-003 per-side substrate: exercised by the memoization + set-identity tests (Cor 1.1/1.4)
    pub(crate) fn component_value_unit(
        &mut self,
        system: &SquareSystem3,
        component: usize,
        unit: [(f64, f64); 4],
    ) -> Result<CertifiedInterval, SsiRefusal> {
        if component > 2 {
            return Err(SsiRefusal::InvalidInput);
        }
        let ha_num = self.hull_side_box(system, 0, component, [unit[0], unit[1]])?;
        let ha_w = self.hull_side_box(system, 0, 3, [unit[0], unit[1]])?;
        let hb_num = self.hull_side_box(system, 1, component, [unit[2], unit[3]])?;
        let hb_w = self.hull_side_box(system, 1, 3, [unit[2], unit[3]])?;
        let hull = hb_w.mul(&ha_num).sub(&ha_w.mul(&hb_num));
        if hull.is_finite() {
            Ok(hull)
        } else {
            Err(SsiRefusal::Hull(HullRefusal::EnclosureUnavailable))
        }
    }

    /// The number of distinct (side, box, function) hull entries held.
    #[allow(dead_code)] // CFP-003 per-side substrate: exercised by the memoization test (Cor 1.4)
    pub(crate) fn distinct_entries(&self) -> usize {
        self.cache.len()
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

/// Certified value of one component at a chart point: per-side hull over a
/// degenerate box (no differentiation; spec Corollary 1.1).
fn value_at_point(
    system: &SquareSystem3,
    component: usize,
    point: [f64; 4],
) -> Result<CertifiedInterval, SsiRefusal> {
    if component > 2 {
        return Err(SsiRefusal::InvalidInput);
    }
    let box_: [(f64, f64); 4] = [
        (point[0], point[0]),
        (point[1], point[1]),
        (point[2], point[2]),
        (point[3], point[3]),
    ];
    let unit = unit_box(system, box_)?;
    let hull = component_value_unit(system, component, unit)?;
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
/// string). For two rational patches the cross-multiplied D-homogeneous
/// component is `F_k = W2(s,t)·N1_k(u,v) − W1(u,v)·N2_k(s,t)`. Under
/// CFP-003-SEPARABILITY the stored form is PER-SIDE ([`SquareSystem3`] keeps
/// the two carriers, no materialized four-axis grid): every certified
/// enclosure combines per-side 2-D hulls (spec Corollaries 1.1–1.2), so
/// construction performs no cross-multiplied materialization. The two patches
/// share the unit chart, so the stored domain maps are the identity rectangle
/// `(0,1,0,1,0,1,0,1)`.
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
    let side_a = SideCarrier::new(
        p1.m(),
        p1.n(),
        p1.numerator().clone(),
        p1.weights().to_vec(),
    )
    .map_err(|_| SsiRefusal::InvalidInput)?;
    let side_b = SideCarrier::new(
        p2.m(),
        p2.n(),
        p2.numerator().clone(),
        p2.weights().to_vec(),
    )
    .map_err(|_| SsiRefusal::InvalidInput)?;
    let identity = (0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0);
    SquareSystem3::from_sides(side_a, side_b, identity).map_err(SsiRefusal::from)
}

// ---------------------------------------------------------------------------
// CFP-008-STAGNATION: cone-overlap non-shrink detection and the CTE route
// ---------------------------------------------------------------------------
//
// The funnel's degenerate tail (spec §3, decision 6; §2 F-C6) is a near-tangent
// pair whose per-box Gauss-map cones keep overlapping as the box subdivides:
// the old loop spun until its budget reported `Unresolved`. This section is the
// booked CFP-008 route: the certified cone-overlap outcome of two consecutive
// subdivision levels is compared EXACTLY, and a non-shrink (the cones overlap
// at both recorded depths, so no strictly-positive shrink was certified) routes
// the pair to the landed CTE cascade ([`crate::tangency::cascade::classify_box`])
// through its published entry instead of burning more subdivision budget. The
// cascade's verdict is returned verbatim; a box the cascade cannot certify keeps
// the typed fail-closed `Unresolved` residual or the typed refusal. Never a
// panic, never a guess, never a silent downgrade.

use crate::cfp::fixtures::ConeOverlapRecord;
use crate::tangency::cascade::classify_box;
use crate::tangency::graph::CertifiedGraph;
use crate::tangency::qpoly::QPoly;
use crate::tangency::shapes::{ChartMinorGrids, ContactVerdict};
use crate::tangency::tsystem::TSystem;
use crate::tangency::TangencyRefusal;
use truck_base::evidence::Budget;

/// Whether the certified cone-overlap outcome recorded at `later` failed to
/// shrink from the outcome recorded at `earlier` (two consecutive subdivision
/// levels, in the fixed subdivision order).
///
/// The overlap measure is the certified interval-cone outcome of CFP-001's
/// sub-box cones (spec §3, Gauss-map certificates); a pair whose cones still
/// overlap at the later depth has NOT shrunk by any strictly-positive certified
/// margin — the recorded overlap persisted. The level comparison is exact:
/// never a naked float comparison, never an epsilon (H-3).
pub fn cone_overlap_did_not_shrink(earlier: &ConeOverlapRecord, later: &ConeOverlapRecord) -> bool {
    earlier.cones_overlap && later.cones_overlap
}

/// The certified cascade inputs of one routed box: the `system`/`graph`/
/// `minors`/`tsys`/`box` inputs of the landed [`classify_box`] entry bundled
/// verbatim so the CFP-008 route stays a small pure consumer.
#[derive(Debug, Clone)]
pub struct CascadeClassifyInput<'a> {
    /// The stored square system of the routed pair.
    pub system: &'a SquareSystem3,
    /// The certified (H-graph) content over the box.
    pub graph: &'a CertifiedGraph,
    /// The chart-minor grids of the chart.
    pub minors: &'a ChartMinorGrids,
    /// The deflated `T = (G1, G2, M1, M2)` system of the chart.
    pub tsys: &'a TSystem,
    /// The box being classified.
    pub b: &'a [(f64, f64); 4],
}

/// The outcome of the CFP-008 stagnation route on one recorded subdivision
/// sequence.
///
/// Deterministic by construction: the recorded levels are seen in fixed order,
/// the route is a pure function of the recorded levels and the certified
/// cascade inputs, and identical ordered input yields an identical outcome
/// field-for-field.
#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::large_enum_variant)] // the cascade verdict is carried verbatim, boxed nowhere (CertifiedPairResult precedent)
pub enum StagnationOutcome {
    /// The recorded cone-overlap measure shrank (or the pair was never
    /// overlapping): no stagnation is certified and the funnel keeps its normal
    /// subdivision course. No cascade entry, no subdivision budget spent here.
    Progress,
    /// Cone overlap failed to shrink between the recorded levels: the pair is
    /// routed to the landed CTE cascade, whose verdict is returned verbatim — a
    /// certified `A1Isolated`/`A1Node`/`A2Branch`/`Transversal`/`Empty`
    /// verdict, or the cascade's typed fail-closed `Unresolved` residual with
    /// its cell when the cascade cannot certify the routed box.
    RoutedToCascade(ContactVerdict<QPoly>),
    /// The routed box refused classification inside the cascade (an invalid
    /// box or an unavailable hull enclosure): the typed refusal is carried
    /// verbatim — fail-closed, never a panic, never a guess.
    CascadeRefused(TangencyRefusal),
}

/// The CFP-008 route: when cone overlap does not shrink between subdivision
/// levels, classify the pair through the landed CTE cascade instead of burning
/// the funnel's subdivision budget.
///
/// `records` is the subdivision sequence in fixed order; the route compares the
/// two consecutive recorded levels exactly ([`cone_overlap_did_not_shrink`]).
/// When no stagnation is certified the outcome is [`StagnationOutcome::Progress`]
/// and the funnel keeps subdividing; when the recorded overlap persisted the
/// pair is handed to [`classify_box`] through its published entry (the
/// "A1/A2 vocabulary" of `tangency/cascade.rs`), returning the cascade's
/// verdict verbatim. A box the cascade refuses is carried as the typed
/// [`StagnationOutcome::CascadeRefused`] refusal — the old budget-burn arm is
/// never taken for the F-C6 class.
pub fn route_stagnation(
    records: &[ConeOverlapRecord],
    input: &CascadeClassifyInput<'_>,
    budget: &mut Budget,
) -> StagnationOutcome {
    // The two consecutive recorded levels in fixed order; fewer than two
    // recorded levels means nothing to compare yet.
    let [.., earlier, later] = records else {
        return StagnationOutcome::Progress;
    };
    if !cone_overlap_did_not_shrink(earlier, later) {
        return StagnationOutcome::Progress;
    }
    match classify_box(
        input.system,
        input.graph,
        input.minors,
        input.tsys,
        input.b,
        budget,
    ) {
        Ok(verdict) => StagnationOutcome::RoutedToCascade(verdict),
        Err(refusal) => StagnationOutcome::CascadeRefused(refusal),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formal::exact::{CertifiedInterval, CertifiedSign, Expansion};

    /// A tolerance for the coefficientwise finite-difference comparison.
    /// H-3: a dimensionless roundoff allowance over the small integer
    /// coefficient grids of the F5 fixture pair.
    const SECOND_PARTIAL_TOL: f64 = 1e-9; // H-3

    /// The binomial coefficient `C(n, k)` as `f64` (small exact integers).
    fn binom(n: usize, k: usize) -> f64 {
        let mut num = 1.0;
        let mut den = 1.0;
        for i in 0..k {
            num *= (n - i) as f64;
            den *= (i + 1) as f64;
        }
        num / den
    }

    /// A graph-patch coordinate grid: the first/second parameter coordinate
    /// elevated to the bidegree `(m, n)`.
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

    /// A zero grid of bidegree `(m, n)`.
    fn zero_grid(m: usize, n: usize) -> Vec<Vec<f64>> {
        vec![vec![0.0; n + 1]; m + 1]
    }

    /// A Bernstein grid of bidegree `(m, n)` from monomial terms
    /// `(pu, pv, coeff)` in `u^pu v^pv`.
    fn monomial_grid(m: usize, n: usize, terms: &[(usize, usize, f64)]) -> Vec<Vec<f64>> {
        let mut grid = zero_grid(m, n);
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

    /// The F5 fixture pair: the plane `z = 0` (graph over `(u, v)`, bidegree
    /// `(2, 2)`) and the extruded-parabola bipatch `z = s^2` over `(s, t)`,
    /// bidegree `(2, 1)`, both with unit weights. The cross-multiplied square
    /// system's grids are `F = P1 - P2`.
    fn parabola_system() -> SquareSystem3 {
        let m1 = 2usize;
        let n1 = 2usize;
        let m2 = 2usize;
        let n2 = 1usize;
        let ones1 = vec![vec![1.0f64; n1 + 1]; m1 + 1];
        let ones2 = vec![vec![1.0f64; n2 + 1]; m2 + 1];
        let plane = RationalBipatch::new(
            m1,
            n1,
            [
                coord_grid(m1, n1, 0),
                coord_grid(m1, n1, 1),
                zero_grid(m1, n1),
            ],
            ones1,
        )
        .expect("the F5 plane patch admits");
        let parabola = RationalBipatch::new(
            m2,
            n2,
            [
                coord_grid(m2, n2, 0),
                coord_grid(m2, n2, 1),
                monomial_grid(m2, n2, &[(2, 0, 1.0)]),
            ],
            ones2,
        )
        .expect("the F5 parabola patch admits");
        construct_square_system(
            &SsiParticipant::RationalBipatch(plane),
            &SsiParticipant::RationalBipatch(parabola),
        )
        .expect("the F5 pair constructs a square system")
    }

    /// A dense axis-major 4-D view of a stored `rows x cols` grid.
    struct Dense4 {
        dims: [usize; 4],
        data: Vec<f64>,
    }

    impl Dense4 {
        fn from_rows(grid: &[Vec<f64>], degrees: (usize, usize, usize, usize)) -> Self {
            let (m1, n1, m2, n2) = degrees;
            let dims = [m1 + 1, n1 + 1, m2 + 1, n2 + 1];
            let mut data = vec![0.0f64; dims[0] * dims[1] * dims[2] * dims[3]];
            for a in 0..dims[0] {
                for b in 0..dims[1] {
                    for i in 0..dims[2] {
                        for j in 0..dims[3] {
                            let idx = ((a * dims[1] + b) * dims[2] + i) * dims[3] + j;
                            data[idx] = grid[a * (n1 + 1) + b][i * (n2 + 1) + j];
                        }
                    }
                }
            }
            Dense4 { dims, data }
        }

        fn into_rows(self, degrees: (usize, usize, usize, usize)) -> Vec<Vec<f64>> {
            let (m1, n1, m2, n2) = degrees;
            let rows = (m1 + 1) * (n1 + 1);
            let cols = (m2 + 1) * (n2 + 1);
            let mut out = vec![vec![0.0f64; cols]; rows];
            let [d0, d1, d2, d3] = self.dims;
            for a in 0..d0 {
                for b in 0..d1 {
                    for i in 0..d2 {
                        for j in 0..d3 {
                            let idx = ((a * d1 + b) * d2 + i) * d3 + j;
                            out[a * (n1 + 1) + b][i * (n2 + 1) + j] = self.data[idx];
                        }
                    }
                }
            }
            out
        }

        fn at(&self, x: [usize; 4]) -> f64 {
            self.data[((x[0] * self.dims[1] + x[1]) * self.dims[2] + x[2]) * self.dims[3] + x[3]]
        }
    }

    /// The closed-form second-difference reference on a stored grid along the
    /// chart axes `j` then `l`: `d_j·d_l` applied to the two-axis centered
    /// difference of the coefficient sequence (for `j == l`, the one-axis
    /// rule `d·(d−1)·(c[k+2] − 2c[k+1] + c[k])`). A linear double-derivative
    /// axis is the zero grid. Returns the reference in the rows layout.
    fn second_difference_reference(
        grid: &[Vec<f64>],
        degrees: (usize, usize, usize, usize),
        j: usize,
        l: usize,
    ) -> (Vec<Vec<f64>>, (usize, usize, usize, usize)) {
        let deg = [degrees.0, degrees.1, degrees.2, degrees.3];
        let out_deg: [usize; 4] = deg.map(|d| d + 1);
        let mut out_deg = out_deg;
        for a in [j, l] {
            out_deg[a] -= 1;
        }
        if j == l && deg[j] == 1 {
            // The axis is linear: the second derivative is the zero grid at
            // the first partial's degree (axis degree 0).
            let mut reduced = deg;
            reduced[j] = 0;
            let (m1, n1, m2, n2) = (reduced[0], reduced[1], reduced[2], reduced[3]);
            let rows = (m1 + 1) * (n1 + 1);
            let cols = (m2 + 1) * (n2 + 1);
            return (vec![vec![0.0; cols]; rows], (m1, n1, m2, n2));
        }
        let dense = Dense4::from_rows(grid, degrees);
        let mut data = vec![0.0f64; out_deg[0] * out_deg[1] * out_deg[2] * out_deg[3]];
        for x0 in 0..out_deg[0] {
            for x1 in 0..out_deg[1] {
                for x2 in 0..out_deg[2] {
                    for x3 in 0..out_deg[3] {
                        let x = [x0, x1, x2, x3];
                        let value = if j == l {
                            let dj = deg[j] as f64;
                            let up2 = {
                                let mut y = x;
                                y[j] += 2;
                                dense.at(y)
                            };
                            let up1 = {
                                let mut y = x;
                                y[j] += 1;
                                dense.at(y)
                            };
                            let here = dense.at(x);
                            dj * (dj - 1.0) * (up2 - 2.0 * up1 + here)
                        } else {
                            let dj = deg[j] as f64;
                            let dl = deg[l] as f64;
                            let jp = {
                                let mut y = x;
                                y[j] += 1;
                                y
                            };
                            let lp = {
                                let mut y = x;
                                y[l] += 1;
                                y
                            };
                            let jplp = {
                                let mut y = x;
                                y[j] += 1;
                                y[l] += 1;
                                y
                            };
                            dj * dl * (dense.at(jplp) - dense.at(jp) - dense.at(lp) + dense.at(x))
                        };
                        let idx = ((x0 * out_deg[1] + x1) * out_deg[2] + x2) * out_deg[3] + x3;
                        data[idx] = value;
                    }
                }
            }
        }
        let od = Dense4 {
            dims: out_deg,
            data,
        };
        let out_degrees = (
            out_deg[0] - 1,
            out_deg[1] - 1,
            out_deg[2] - 1,
            out_deg[3] - 1,
        );
        (od.into_rows(out_degrees), out_degrees)
    }

    #[test]
    fn second_partials_match_finite_difference_on_fixture() {
        let system = parabola_system();
        let degrees = system.degrees();
        let grids = system.grids();
        for (component, grid) in grids.iter().enumerate() {
            let grid = grid.clone();
            let tensor = Tensor4::from_grid(&grid, degrees);
            for j in 0..4 {
                for l in 0..4 {
                    let got = tensor
                        .partial2_axis(j, l)
                        .expect("every F5 grid second-partials on a degree-1+ axis");
                    let (want_rows, want_degrees) =
                        second_difference_reference(&grid, degrees, j, l);
                    assert_eq!(got.degrees, want_degrees);
                    assert_eq!(got.rows.len(), want_rows.len());
                    for (grow, wrow) in got.rows.iter().zip(want_rows.iter()) {
                        assert_eq!(grow.len(), wrow.len());
                        for (g, w) in grow.iter().zip(wrow.iter()) {
                            let diff = (g - w).abs();
                            assert!(
                                diff <= SECOND_PARTIAL_TOL,
                                "axis pair ({}, {}) component {}: partial2 {} vs finite difference {}",
                                j,
                                l,
                                component,
                                g,
                                w
                            );
                        }
                    }
                }
            }
        }

        // The fixture's semantic pin: the z-component of the F5 pair is
        // `-s^2`, whose second s-partial is the constant `-2` over the whole
        // chart.
        let z_grid = &grids[2];
        let z_tensor = Tensor4::from_grid(z_grid, degrees);
        let second = z_tensor
            .partial2_axis(2, 2)
            .expect("the parabola axis second-partials");
        for row in second.rows.iter() {
            for c in row.iter() {
                assert!(
                    (c + 2.0).abs() <= SECOND_PARTIAL_TOL,
                    "d2/ds2 of -s^2 must be the constant -2, got {}",
                    c
                );
            }
        }
    }

    // -----------------------------------------------------------------------
    // CFP-003-SEPARABILITY — per-side machinery gates (fixture data copied
    // from `cfp/fixtures.rs` as read-only constants).
    // -----------------------------------------------------------------------

    /// The exact `λ·p` dot product of integer data as an [`Expansion`].
    fn dot_exp(lambda: &[i64; 3], p: &[i64; 3]) -> Expansion {
        let mut acc = Expansion::zero();
        for k in 0..3 {
            acc = acc.merge(&Expansion::from_product(lambda[k] as f64, p[k] as f64));
        }
        acc
    }

    /// Whether `a > b` exactly (the exact `Expansion` dot-sign predicate).
    fn expansion_strictly_gt(a: &Expansion, b: &Expansion) -> bool {
        matches!(a.merge(&b.negate()).sign(), CertifiedSign::Positive)
    }

    /// The exact Theorem-3 separation certificate
    /// `min_α λ·P¹_α > max_β λ·P²_β` (F-C3 data, copied read-only).
    fn separated_by_exact_dot(
        lambda: &[i64; 3],
        net_a: &[[i64; 3]; 4],
        net_b: &[[i64; 3]; 4],
    ) -> bool {
        let dots_a: Vec<Expansion> = net_a.iter().map(|p| dot_exp(lambda, p)).collect();
        let dots_b: Vec<Expansion> = net_b.iter().map(|p| dot_exp(lambda, p)).collect();
        dots_a
            .iter()
            .all(|a| dots_b.iter().all(|b| expansion_strictly_gt(a, b)))
    }

    #[test]
    fn fc3_separation_exact_sign_certificate() {
        // F-C3 data copied from cfp/fixtures.rs (read-only).
        // The separated pair: λ = (1, 0, 0), all of A at x = 2, all of B at
        // x ∈ {0, 1}.
        let separated_lambda = [1i64, 0, 0];
        let separated_a = [[2, 0, 0], [2, 1, 0], [2, 0, 1], [2, 1, 1]];
        let separated_b = [[0, 0, 0], [1, 1, 0], [1, 0, 1], [0, 1, 1]];
        // The touching pair shares the point (2, 1, 0).
        let touching_lambda = [1i64, 0, 0];
        let touching_a = [[0, 0, 0], [1, 1, 1], [2, 1, 0], [1, 0, 0]];
        let touching_b = [[2, 1, 0], [3, 0, 0], [3, 2, 1], [2, 2, 0]];

        assert!(
            separated_by_exact_dot(&separated_lambda, &separated_a, &separated_b),
            "the separated pair certifies via the exact Expansion dot-sign row"
        );
        assert!(
            !separated_by_exact_dot(&touching_lambda, &touching_a, &touching_b),
            "the touching pair must NOT separate along the exact dot-sign row"
        );
        assert!(
            touching_a.contains(&[2, 1, 0]) && touching_b.contains(&[2, 1, 0]),
            "F-C3's touching pair shares the recorded control point"
        );
    }

    /// Whether two certified enclosures agree within a directed-rounding slack
    /// scaled to the enclosure magnitude (H-3).
    fn intervals_agree(a: &CertifiedInterval, b: &CertifiedInterval) -> bool {
        let scale = 1.0 + a.lo.abs().max(a.hi.abs()).max(b.lo.abs().max(b.hi.abs()));
        const SLACK: f64 = 1e-12; // H-3: outward-rounding slack for set-identity (Cor 1.1), fixture scale O(1)
        let tol = SLACK * scale;
        (a.lo - b.lo).abs() <= tol && (a.hi - b.hi).abs() <= tol
    }

    /// The certified coefficient hull of a grid: `[min c, max c]` over all
    /// coefficients, outward rounded (the exact control hull over the full
    /// chart — the tight enclosure the per-side/grid set identity is about).
    fn coefficient_hull(grid: &[Vec<f64>]) -> CertifiedInterval {
        let mut lo = f64::INFINITY;
        let mut hi = f64::NEG_INFINITY;
        for row in grid {
            for &c in row {
                lo = lo.min(c);
                hi = hi.max(c);
            }
        }
        CertifiedInterval {
            lo: lo.next_down(),
            hi: hi.next_up(),
        }
    }

    #[test]
    fn per_side_hull_set_identical_to_grid_hull_on_fixtures() {
        // The landed fixture battery of per-side systems (all D2 unit-weight
        // cross differences). For every fixture pair and every component the
        // per-side composition and the old (materialized four-axis) grid hull
        // are SET-identical (Corollary 1.1): `min c_αβ = min P1 − max P2` and
        // `max c_αβ = max P1 − min P2` over the coefficient lattice, so the
        // tight control-hull ranges agree exactly up to outward rounding; and
        // the per-side interval enclosure stays a sound BG-ENC-001 enclosure
        // of the same grid polynomial on sub-boxes.
        let systems: Vec<SquareSystem3> = {
            let mut out = Vec::new();
            out.push(parabola_system());
            if let Ok(record) = crate::ssi_fixtures::well_conditioned_root() {
                out.push(record.system);
            }
            if let Ok(record) = crate::ssi_fixtures::negative_orientation_root() {
                out.push(record.system);
            }
            if let Ok(record) = crate::ssi_fixtures::determinant_spans_zero() {
                out.push(record.system);
            }
            if let Ok(record) = crate::ssi_fixtures::conditioning_below_threshold() {
                out.push(record.system);
            }
            if let Ok(pair) = crate::ssi_fixtures::closed_loop_pair() {
                out.push(pair.system);
            }
            if let Ok(ladder) = crate::ssi_fixtures::germ_ladder() {
                for germ in ladder {
                    out.push(germ.system);
                }
            }
            out
        };
        assert!(
            systems.len() >= 8,
            "the battery must span at least the F5 + F-C fixture systems"
        );
        let mut checked = 0usize;
        for system in &systems {
            let degrees = system.degrees();
            let side_a = system.side_a();
            let side_b = system.side_b();
            for component in 0..3 {
                let grid = system.grids()[component].clone();
                let grid_hull = coefficient_hull(&grid);
                let a_hull = coefficient_hull(&side_a.numerator()[component]);
                let b_hull = coefficient_hull(&side_b.numerator()[component]);
                let per_side = CertifiedInterval {
                    lo: (a_hull.lo - b_hull.hi).next_down(),
                    hi: (a_hull.hi - b_hull.lo).next_up(),
                };
                assert!(
                    intervals_agree(&per_side, &grid_hull),
                    "system degrees {degrees:?} comp {component}: per-side {per_side:?} vs \
                     grid hull {grid_hull:?}"
                );
                // Structural identity: every stored coefficient is exactly the
                // per-side difference of the two side grids' coefficients
                // (Theorem 1's `c_αβ = P¹_α − P²_β`, no cross terms).
                let sp1 = degrees.1 + 1;
                let sp2 = degrees.3 + 1;
                for (r, row) in grid.iter().enumerate() {
                    for (c, value) in row.iter().enumerate() {
                        let a_val = side_a.numerator()[component][r / sp1][r % sp1];
                        let b_val = side_b.numerator()[component][c / sp2][c % sp2];
                        assert!(
                            (*value - (a_val - b_val)).abs() <= 1e-12,
                            "degrees {degrees:?} comp {component}: grid[{r}][{c}] {} != {a_val} − {b_val}",
                            value
                        );
                    }
                }
                // Sub-box soundness: the per-side interval enclosure is a
                // BG-ENC-001 enclosure of the same grid polynomial: it
                // contains the direct evaluations at every cell corner of a
                // depth-2 dyadic subdivision.
                let div = 4usize;
                for ui in 0..div {
                    for vi in 0..div {
                        for si in 0..div {
                            for ti in 0..div {
                                let cell = [
                                    (ui as f64 / div as f64, (ui + 1) as f64 / div as f64),
                                    (vi as f64 / div as f64, (vi + 1) as f64 / div as f64),
                                    (si as f64 / div as f64, (si + 1) as f64 / div as f64),
                                    (ti as f64 / div as f64, (ti + 1) as f64 / div as f64),
                                ];
                                let per_side = component_value_unit(system, component, cell)
                                    .expect("the per-side composition certifies on the cell");
                                for du in [cell[0].0, cell[0].1] {
                                    for dv in [cell[1].0, cell[1].1] {
                                        for ds in [cell[2].0, cell[2].1] {
                                            for dt in [cell[3].0, cell[3].1] {
                                                let sample = crate::ssi_fixtures::eval_grid4(
                                                    &grid,
                                                    degrees,
                                                    (du, dv, ds, dt),
                                                )
                                                .expect("a corner evaluation of the fixture grid");
                                                assert!(
                                                    per_side.lo <= sample && sample <= per_side.hi,
                                                    "per-side enclosure {per_side:?} misses the \
                                                     sampled value {sample}"
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                checked += 1;
            }
        }
        assert!(checked >= systems.len() * 3);
    }

    /// Float evaluation of a bivariate Bernstein coefficient grid at a unit
    /// point (test support).
    fn eval_2d_float_test(grid: &[Vec<f64>], u: f64, v: f64) -> f64 {
        let bern = |mut level: Vec<f64>, x: f64| -> f64 {
            while level.len() > 1 {
                let mut next = Vec::with_capacity(level.len() - 1);
                for w in level.windows(2) {
                    next.push(w[0] + x * (w[1] - w[0]));
                }
                level = next;
            }
            level[0]
        };
        let mut rows = Vec::with_capacity(grid.len());
        for row in grid {
            rows.push(bern(row.to_vec(), v));
        }
        bern(rows, u)
    }

    #[test]
    #[allow(clippy::needless_range_loop)] // the 3x4 block-Jacobian row/column index algebra is clearest in the explicit range form
    fn minor_factorization_matches_direct_expansion() {
        // Corollary 1.3 vs the direct det3 expansion of the block Jacobian on
        // fixture Jacobians, equal within rounding.
        let system = parabola_system();
        let a = system.side_a();
        let b = system.side_b();
        let (ma, na) = (a.m(), a.n());
        let (mb, nb) = (b.m(), b.n());
        let points: [(f64, f64, f64, f64); 4] = [
            (0.5, 0.5, 0.5, 0.5),
            (0.25, 0.5, 0.75, 0.5),
            (0.5, 0.25, 0.5, 0.75),
            (0.75, 0.25, 0.25, 0.75),
        ];
        const MINOR_TOL: f64 = 1e-9; // H-3
        for (u, v, s, t) in points {
            // The 3 x 4 block Jacobian J = [S1_u | S1_v | -S2_s | -S2_t].
            let mut j = [[0.0f64; 4]; 3];
            for k in 0..3 {
                let du_a = bivariate_derivative(&a.numerator()[k], ma, na, 0)
                    .expect("side A u-derivative grid");
                let dv_a = bivariate_derivative(&a.numerator()[k], ma, na, 1)
                    .expect("side A v-derivative grid");
                let ds_b = bivariate_derivative(&b.numerator()[k], mb, nb, 0)
                    .expect("side B s-derivative grid");
                let dt_b = bivariate_derivative(&b.numerator()[k], mb, nb, 1)
                    .expect("side B t-derivative grid");
                j[k][0] = eval_2d_float_test(&du_a, u, v);
                j[k][1] = eval_2d_float_test(&dv_a, u, v);
                j[k][2] = -eval_2d_float_test(&ds_b, s, t);
                j[k][3] = -eval_2d_float_test(&dt_b, s, t);
            }
            let det3 = |cols: [usize; 3]| -> f64 {
                let m = |r: usize, c: usize| j[r][cols[c]];
                m(0, 0) * (m(1, 1) * m(2, 2) - m(1, 2) * m(2, 1))
                    - m(0, 1) * (m(1, 0) * m(2, 2) - m(1, 2) * m(2, 0))
                    + m(0, 2) * (m(1, 0) * m(2, 1) - m(1, 1) * m(2, 0))
            };
            let dot = |x: [f64; 3], y: [f64; 3]| x[0] * y[0] + x[1] * y[1] + x[2] * y[2];
            let cross = |x: [f64; 3], y: [f64; 3]| -> [f64; 3] {
                [
                    x[1] * y[2] - x[2] * y[1],
                    x[2] * y[0] - x[0] * y[2],
                    x[0] * y[1] - x[1] * y[0],
                ]
            };
            let col = |c: usize| [j[0][c], j[1][c], j[2][c]];
            let s1_u = col(0);
            let s1_v = col(1);
            let n1 = cross(s1_u, s1_v);
            let n2 = cross(col(2), col(3));
            // Corollary 1.3 factorizations per dropped chart axis.
            let factored = [
                dot(s1_v, n2),   // drop u: M_u = S1_v·n2
                dot(s1_u, n2),   // drop v: M_v = S1_u·n2
                dot(n1, col(3)), // drop s: M_s = −n1·S2_t = n1·J_t
                dot(n1, col(2)), // drop t: M_t = −n1·S2_s = n1·J_s
            ];
            let direct = [
                det3([1, 2, 3]),
                det3([0, 2, 3]),
                det3([0, 1, 3]),
                det3([0, 1, 2]),
            ];
            for axis in 0..4 {
                let scale = 1.0 + direct[axis].abs();
                assert!(
                    (factored[axis] - direct[axis]).abs() <= MINOR_TOL * scale,
                    "point ({u},{v},{s},{t}) axis {axis}: Cor 1.3 {:.12} vs det3 {:.12}",
                    factored[axis],
                    direct[axis]
                );
            }
        }
    }

    #[test]
    fn memoization_distinct_boxes_bounded() {
        // Corollary 1.4: a dyadic subdivision visiting M cells drawn from k
        // distinct per-side boxes pays O(k) hulls. F-C5-style product-grid
        // subdivision of the unit chart at depth 2 (4 intervals per axis):
        // M = 4^4 cells, k = 4^2 + 4^2 = 32 distinct side boxes.
        use crate::cfp::spine::InstrumentCounters;
        let system = parabola_system();
        let mut memo = PerSideHullMemo::default();
        let div = 4usize;
        let mut cells_visited = 0u64;
        let mut distinct_side_boxes = std::collections::BTreeSet::<(u8, u64, u64, u64, u64)>::new();
        for ui in 0..div {
            for vi in 0..div {
                for si in 0..div {
                    for ti in 0..div {
                        let cell = [
                            (ui as f64 / div as f64, (ui + 1) as f64 / div as f64),
                            (vi as f64 / div as f64, (vi + 1) as f64 / div as f64),
                            (si as f64 / div as f64, (si + 1) as f64 / div as f64),
                            (ti as f64 / div as f64, (ti + 1) as f64 / div as f64),
                        ];
                        let side_a_box = (
                            0u8,
                            cell[0].0.to_bits(),
                            cell[0].1.to_bits(),
                            cell[1].0.to_bits(),
                            cell[1].1.to_bits(),
                        );
                        let side_b_box = (
                            1u8,
                            cell[2].0.to_bits(),
                            cell[2].1.to_bits(),
                            cell[3].0.to_bits(),
                            cell[3].1.to_bits(),
                        );
                        distinct_side_boxes.insert(side_a_box);
                        distinct_side_boxes.insert(side_b_box);
                        let hull = memo
                            .component_value_unit(&system, 0, cell)
                            .expect("the memoized per-side enclosure certifies on every cell");
                        assert!(hull.is_finite());
                        cells_visited += 1;
                    }
                }
            }
        }
        let k = distinct_side_boxes.len() as u64;
        assert_eq!(cells_visited, 256, "M = 4^4 product cells visited");
        assert_eq!(k, 32, "k = 4^2 + 4^2 distinct per-side boxes");
        // The memo holds one (side, box, function) entry per distinct hull;
        // component 0 needs the numerator and weight hull on each side box.
        let entries = memo.distinct_entries() as u64;
        assert!(
            entries <= 2 * k,
            "O(k) hulls: {entries} entries for {k} distinct side boxes"
        );
        assert!(
            k * 4 < cells_visited,
            "memoization is only meaningful when k is far below M"
        );
        // Recorded through the landed instrument fields (schema positions 3/4).
        let counters = InstrumentCounters::new(0, 0, 0, cells_visited, k, 0);
        assert_eq!(counters.values()[3], cells_visited);
        assert_eq!(counters.values()[4], k);
    }

    #[test]
    #[allow(clippy::needless_range_loop)] // the three normal-net component grids are indexed by component in the degree/count checks
    fn normal_net_degree_and_d3_count() {
        // Prop 2.1 / scope decision 2: the composed per-side normal net of a
        // bidegree-(p, q) carrier has bidegree (2p−1, 2q−1) and 4pq
        // coefficients per component (never interval cross products of
        // derivative hulls).
        let system = parabola_system();
        // Side A of the F5 fixture is the bidegree-(2, 2) plane z = 0; side B
        // the bidegree-(2, 1) extruded parabola.
        let (pa, qa) = (system.side_a().m(), system.side_a().n());
        let (pb, qb) = (system.side_b().m(), system.side_b().n());
        let net_a = side_normal_nets(system.side_a()).expect("side A normal net composes");
        let net_b = side_normal_nets(system.side_b()).expect("side B normal net composes");
        for (grid, (p, q)) in [(&net_a, (pa, qa)), (&net_b, (pb, qb))] {
            let want_rows = 2 * p;
            let want_cols = 2 * q;
            assert_eq!(want_rows * want_cols, 4 * p * q);
            for component in 0..3 {
                let g = &grid[component];
                assert_eq!(g.len(), want_rows, "normal net rows = 2p for p={p}");
                assert_eq!(g[0].len(), want_cols, "normal net cols = 2q for q={q}");
                assert_eq!(
                    g.iter()
                        .map(Vec::len)
                        .collect::<std::collections::HashSet<_>>()
                        .len(),
                    1,
                    "normal net grids are rectangular"
                );
                assert_eq!(g.iter().flatten().count(), 4 * p * q);
            }
        }
        assert_eq!(4 * pa * qa, 16, "4pq = 16 for the (2,2) side");
        assert_eq!(4 * pb * qb, 8, "4pq = 8 for the (2,1) side");
    }

    /// One battery verdict tag of the landed SSI fixture battery: the outcome
    /// of certifying one continuation axis on one fixture box.
    fn ssi_fixture_tag(system: &SquareSystem3, axis: usize, box_: [(f64, f64); 4]) -> String {
        match krawczyk3_certificate(system, axis, box_) {
            Ok(cert) => {
                let (d_lo, d_hi) = cert.det();
                if d_lo > 0.0 {
                    "certified_pos".to_string()
                } else if d_hi < 0.0 {
                    "certified_neg".to_string()
                } else {
                    "certified_zero_span".to_string()
                }
            }
            Err(e) => e.tag().to_string(),
        }
    }

    #[test]
    fn landed_ssi_fixture_verdicts_unchanged() {
        // The landed ssi_fixtures battery's VERDICT SET is identical
        // pre/post the per-side refactor: same Certified/Unresolved pattern,
        // same roots. Enclosures may differ in ulps (Corollary 1.1 is
        // SET-identical, not bit-identical); the verdict tags below are the
        // battery recorded on the pre-refactor (materialized-grid) engine and
        // re-measured unchanged on the per-side engine (the SSI system /
        // contract / trace integration suites pin the same verdicts on the
        // same data).
        let well = crate::ssi_fixtures::well_conditioned_root().expect("F-C fixture admits");
        let det0 = crate::ssi_fixtures::determinant_spans_zero().expect("F-C fixture admits");
        let cond = crate::ssi_fixtures::conditioning_below_threshold().expect("F-C fixture admits");
        let root_box: [(f64, f64); 4] = [(0.4, 0.6), (0.4, 0.6), (0.4, 0.6), (0.4, 0.6)];
        let mut battery = Vec::new();
        for axis in 0..4 {
            battery.push(ssi_fixture_tag(&well.system, axis, root_box));
        }
        for axis in 0..4 {
            battery.push(ssi_fixture_tag(&det0.system, axis, det0.box_));
        }
        for axis in 0..4 {
            battery.push(ssi_fixture_tag(&cond.system, axis, cond.box_));
        }
        // Recorded pre-refactor verdict set (identical post-refactor): the
        // well-conditioned root certifies positive on the s continuation axis
        // and negative on the v/t axes (axis 0 conditioning-refuses); the
        // determinant-spans-zero fixture refuses DeterminantSpansZero on the
        // v/s/t axes (axis 0 conditions); the conditioning-below-threshold
        // fixture Conditioning-refuses on every axis.
        assert_eq!(
            battery,
            vec![
                "ssi_conditioning",
                "certified_neg",
                "certified_pos",
                "certified_neg",
                "ssi_conditioning",
                "ssi_determinant_spans_zero",
                "ssi_determinant_spans_zero",
                "ssi_determinant_spans_zero",
                "ssi_conditioning",
                "ssi_conditioning",
                "ssi_conditioning",
                "ssi_conditioning",
            ]
            .into_iter()
            .map(String::from)
            .collect::<Vec<String>>()
        );
        // The roots themselves are unchanged: F evaluates to zero at the
        // fixture roots (the set of roots is part of the verdict set).
        let values = crate::ssi_fixtures::eval_system(&well.system, well.root)
            .expect("the fixture root evaluates");
        for v in values {
            assert!(v.abs() < 1e-9); // H-3
        }
    }

    /// FSSI-000 decision 2: the refusal tag strings are stable, asserted by
    /// name. The two FSSI-001 producer suspicion halts carry the pre-decided
    /// tags `ssi_tangent_curve_suspected` / `ssi_coincident_patch_suspected`;
    /// the six landed families keep their recorded tags (V5 identity).
    #[test]
    fn refusal_tags_stable() {
        let cases: [(SsiRefusal, &'static str); 8] = [
            (
                SsiRefusal::PairClass(PairUnsupported::UnsupportedPairClass),
                "pair_unsupported_class",
            ),
            (
                SsiRefusal::Conditioning(Refusal::ConditioningBelowThreshold),
                "ssi_conditioning",
            ),
            (
                SsiRefusal::Hull(HullRefusal::EnclosureUnavailable),
                "ssi_hull_enclosure_unavailable",
            ),
            (
                SsiRefusal::DeterminantSpansZero,
                "ssi_determinant_spans_zero",
            ),
            (SsiRefusal::InclusionNotStrict, "ssi_inclusion_not_strict"),
            (SsiRefusal::InvalidInput, "ssi_invalid_input"),
            (
                SsiRefusal::TangentCurveSuspected {
                    margin: (0.25, 4.0),
                },
                "ssi_tangent_curve_suspected",
            ),
            (
                SsiRefusal::CoincidentPatchSuspected {
                    margin: (0.25, 4.0),
                },
                "ssi_coincident_patch_suspected",
            ),
        ];
        for (refusal, expected) in cases {
            assert_eq!(refusal.tag(), expected, "stable tag of {refusal:?}");
        }
        // The suspicion halts refuse with their pre-decided tag regardless of
        // the recorded margin (evidence, never a tolerance).
        assert_eq!(
            SsiRefusal::TangentCurveSuspected { margin: (0.0, 0.0) }.tag(),
            "ssi_tangent_curve_suspected"
        );
        assert_eq!(
            SsiRefusal::CoincidentPatchSuspected { margin: (0.0, 0.0) }.tag(),
            "ssi_coincident_patch_suspected"
        );
    }

    /// FSSI-000 scope decision 1: [`FoldCert`] is a refusing-constructor
    /// carrier — a malformed record (non-finite or misordered
    /// `det_enclosure`, a `sigma` outside `{−1, +1}`) refuses
    /// `SsiRefusal::InvalidInput`, never a guess. FSSI-002 populates the
    /// well-formed records; this only freezes the shape.
    #[test]
    fn fold_cert_refusing_constructor() {
        let ok = FoldCert::new(3, 1, (0.5, 2.0)).expect("a well-formed record admits");
        assert_eq!(ok.chart, 3);
        assert_eq!(ok.sigma, 1);
        assert_eq!(ok.det_enclosure, (0.5, 2.0));
        assert_eq!(
            FoldCert::new(3, -1, (0.5, 2.0))
                .expect("sigma = -1 admits")
                .sigma,
            -1
        );
        for (chart, sigma, det_enclosure) in [
            (0usize, 0i8, (0.5, 2.0)),    // sigma outside {−1, +1}
            (0, 2, (0.5, 2.0)),           // sigma outside {−1, +1}
            (0, 1, (f64::NAN, 2.0)),      // non-finite det_enclosure
            (0, 1, (0.5, f64::INFINITY)), // non-finite det_enclosure
            (0, 1, (2.0, 0.5)),           // misordered det_enclosure
        ] {
            assert!(
                matches!(
                    FoldCert::new(chart, sigma, det_enclosure),
                    Err(SsiRefusal::InvalidInput)
                ),
                "a malformed fold record must refuse InvalidInput, got sigma={sigma} \
                 det_enclosure={det_enclosure:?}"
            );
        }
    }
}
