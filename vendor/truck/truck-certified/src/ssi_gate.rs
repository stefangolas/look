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

//! The separable tangency-free admission gate (FSSI-001-GATE).
//!
//! At SSI admission, below the contact funnel, a box `B = B1 × B2` (side A's
//! parameter box times side B's) is certified **transverse**: `rank DF = 3` at
//! EVERY point of `Σ ∩ B`, where `Σ = F⁻¹(0)` is the zero set of the pair's
//! difference map. The certificate (Theorem C, corrected r2) is the separable
//! one — TWO per-surface 2-D normal cones, never a 4-D normal-product field:
//!
//! ```text
//! cone(a, θ_X) ⊇ { n_X(p) : p ∈ B1 },   cone(b, θ_Y) ⊇ { n_Y(q) : q ∈ B2 },
//! δ = min{∠(a,b), π − ∠(a,b)} − θ_X − θ_Y > 0
//!     ⇒  ‖n_X × n_Y‖ ≥ sin δ > 0  on  B1 × B2
//!     ⇒  rank DF = 3 at every point of Σ ∩ B.
//! ```
//!
//! This does **not** certify `Σ ∩ B = ∅`; emptiness is the separate Bernstein
//! exclusion of `F`, never this gate's output (r2 correction). The gate is
//! monotone-widening: a box it admits stays admitted under subdivision, and it
//! can only admit or refuse a box — never flip a landed certified verdict (the
//! V5-pair identity is a required conformance test).
//!
//! # The gate vocabulary
//!
//! The gate's per-box verdict vocabulary is {**TangencyFree**, **Subdivide**,
//! **Suspect(...)**}. [`GateAdmission::TangencyFree`] is the certified arm (an
//! `Ok`); [`GateAdmission::Subdivide`] is the soft per-call probe arm (an `Ok`,
//! produced only when the caller asks for a single level via
//! [`GateParams::max_level == 0`](GateParams)); the **Suspect** arm is the typed
//! refusal on the `Err` side ([`SsiRefusal::TangentCurveSuspected`] /
//! [`SsiRefusal::CoincidentPatchSuspected`]), raised when the box stayed
//! undecided through the whole admission budget and the undecided measure's
//! level-to-level behaviour is classified (theory §1.2). The suspicion halt
//! fires ONLY in the refusal decision: it can cause a premature refusal, never
//! a wrong acceptance.
//!
//! # The subdivision and the halt
//!
//! A product box is decided level by level. At level `k` the undecided region
//! is a union of dyadic product cells (each side's box quartered per level, so
//! a cell's parameter volume scales like `2⁻⁴ᵏ`). A cell certifies when the two
//! per-side normal cones separate; the rest are quartered again. When the
//! budget ([`GateParams::max_cells`], [`GateParams::max_level`]) exhausts with a
//! positive undecided measure, the halt compares the undecided measure across
//! levels against the full-dimensional shrink `2⁻⁴ᵏ`: an undecided region that
//! did not shrink at all (area-scale stagnation — coincident patches) raises
//! [`SsiRefusal::CoincidentPatchSuspected`]; a region that shrank but only
//! sub-linearly (a tangency locus of lower dimension — a tangent curve) raises
//! [`SsiRefusal::TangentCurveSuspected`]. Both carry the undecided measure and
//! the observed shrink exponent at halt (evidence, never a tolerance).
//!
//! # The pole composition (theory §4.1)
//!
//! A box whose side carries a parametric pole (a collapsed apex row, a sphere
//! pole) cannot produce a normal cone for the sub-boxes straddling the pole:
//! the normal *enclosure* contains the zero vector there, and [`cone_of_box`]
//! returns `None`. The gate is per-box, so a box over the regular part of the
//! carrier (the F3 selector's *surviving coordinate* region) is unaffected by
//! the pole elsewhere; a box straddling the pole subdivides toward it and the
//! surviving cells certify. This is the regression the naive global-`q_t`
//! design fails: a whole-carrier normal cone would contain the pole's zero
//! normal and poison every box of the carrier.
//!
//! # Substrate and arithmetic
//!
//! Every certified quantity flows through the outward-rounded
//! [`CertifiedInterval`] of `formal::exact`; the per-side normal enclosures are
//! hulls of each side's *composed normal net* (exact Bernstein product of the
//! derivative nets, the FSSI-ELEV elevated pairing discipline — never the naive
//! product of factor hulls). One interval algebra, no second engine. The gate
//! carries no `unwrap`, no `expect`, no `panic!` (H-1: the module denies
//! `clippy::unwrap_used`).

use crate::formal::exact::CertifiedInterval;
use crate::ssi_types::{SideCarrier, SquareSystem3};
use super::{SsiRefusal, unit_box};

/// The certified per-side normal-component enclosure over one side's unit-chart
/// box: `normal[k]` encloses `{ n_k(p) : p ∈ box }` for `k ∈ {x, y, z}`.
pub type NormalBox = [CertifiedInterval; 3];

/// One side's parameter box (two compact subintervals of the side's unit
/// chart `[0, 1]²`).
pub type SideBox = [(f64, f64); 2];

/// The gate's admission budget and halt classification.
///
/// `max_level == 0` selects the single-level probe verdict
/// ([`GateAdmission::Subdivide`]); a positive `max_level` runs the level-driven
/// subdivision to a certified verdict or a typed suspicion refusal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GateParams {
    /// The largest number of undecided product cells the gate may hold at one
    /// level; exhausting it is the admission-budget halt.
    pub max_cells: u64,
    /// The largest subdivision depth (number of quartering steps) the gate may
    /// take before the admission-budget halt.
    pub max_level: usize,
}

/// The share of the initial undecided measure that may remain at the budget
/// halt before the region is classified as area-scale stagnation (a coincident
/// patch). Named on its defining line (H-3).
const STAGNATION_RATIO: f64 = 0.9;

/// The default undecided-cell budget.
const DEFAULT_MAX_CELLS: u64 = 20000;
/// The default subdivision depth budget.
const DEFAULT_MAX_LEVEL: usize = 12;

impl Default for GateParams {
    fn default() -> Self {
        GateParams {
            max_cells: DEFAULT_MAX_CELLS,
            max_level: DEFAULT_MAX_LEVEL,
        }
    }
}

/// The gate's per-box verdict (FSSI-001 scope decision 1).
///
/// `TangencyFree` and `Subdivide` are the `Ok` carriers of
/// [`gate_admit`]; the **Suspect** arm of the vocabulary is the typed suspicion
/// refusal on the `Err` side ([`SsiRefusal::TangentCurveSuspected`] /
/// [`SsiRefusal::CoincidentPatchSuspected`]).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GateAdmission {
    /// The whole box is certified tangency-free: `rank DF = 3` at every point
    /// of `Σ ∩ B`. `margin` is a certified enclosure of
    /// `min_{B1 × B2} ‖n_X × n_Y‖` with a strictly positive lower bound (the
    /// returned transversality evidence — never a tolerance).
    TangencyFree {
        /// The certified enclosure of the minimal `‖n_X × n_Y‖` over the box,
        /// `(lo, hi)` with `lo > 0`.
        margin: (f64, f64),
    },
    /// The box could not be decided in the requested single level (a probe
    /// with [`GateParams::max_level == 0`](GateParams)); the caller may
    /// subdivide and re-gate (monotone: a certified box never returns here).
    Subdivide,
}

impl GateAdmission {
    /// A short stable tag, for diagnostics.
    pub fn tag(&self) -> &'static str {
        match self {
            Self::TangencyFree { .. } => "tangency_free",
            Self::Subdivide => "subdivide",
        }
    }
}

/// A certified per-side normal direction cone over a side box.
///
/// The cone is anchored at the box's midpoint vector `m` with a certified
/// half-angle `θ` given only through `sin θ ≤ s_up < 1`: for every normal `n`
/// in the box, `sin ∠(n, m) ≤ s_up`. The containment `n · m > 0` (equivalently
/// `∠ < π/2`) follows from `s_up < 1`.
#[derive(Debug, Clone, Copy)]
struct SideCone {
    /// The anchor midpoint vector of the normal enclosure box.
    m: [f64; 3],
    /// A certified upper bound of `max_p |n(p) − m| / |m|` (the sine of the
    /// half-angle).
    s_up: f64,
}

/// The certified normal-enclosure box over a side sub-box, `None` when the box
/// is empty or a component is not finite.
fn cone_of_box(n: NormalBox) -> Option<SideCone> {
    let mut m = [0.0f64; 3];
    for (k, iv) in n.iter().enumerate() {
        if !iv.is_finite() {
            return None;
        }
        m[k] = 0.5 * (iv.lo + iv.hi);
    }
    // Certified radius bound: the largest distance from `m` to any box point,
    // computed outward.
    let mut w2 = CertifiedInterval::point(0.0);
    for (k, iv) in n.iter().enumerate() {
        let dlo = abs_iv(&CertifiedInterval::point(m[k]).sub(&CertifiedInterval::point(iv.lo)));
        let dhi = abs_iv(&CertifiedInterval::point(iv.hi).sub(&CertifiedInterval::point(m[k])));
        let d = CertifiedInterval {
            lo: dlo.lo.min(dhi.lo),
            hi: dlo.hi.max(dhi.hi),
        };
        w2 = w2.add(&d.mul(&d));
    }
    let radius = w2.sqrt()?;
    let mut norm2 = CertifiedInterval::point(0.0);
    for v in m.iter() {
        let iv = CertifiedInterval::point(*v);
        norm2 = norm2.add(&iv.mul(&iv));
    }
    let cn = norm2.sqrt()?.lo;
    if !cn.is_finite() || !radius.hi.is_finite() || radius.hi >= cn {
        return None;
    }
    // Outward-rounded certified upper bound of `radius / |m|`.
    let ratio = CertifiedInterval::point(radius.hi).div(&CertifiedInterval::point(cn))?;
    let s_up = ratio.hi;
    if !s_up.is_finite() || s_up >= 1.0 {
        return None;
    }
    Some(SideCone { m, s_up })
}

/// The absolute value of a [`CertifiedInterval`], outward rounded.
fn abs_iv(iv: &CertifiedInterval) -> CertifiedInterval {
    if iv.lo >= 0.0 {
        *iv
    } else if iv.hi <= 0.0 {
        iv.neg()
    } else {
        CertifiedInterval {
            lo: 0.0,
            hi: iv.lo.abs().max(iv.hi.abs()),
        }
    }
}

/// A certified lower bound of `min_{x ∈ box} |x|` over a normal enclosure box
/// (the box's mignitude), via the component-wise projection of the origin.
fn min_norm_lb(n: &NormalBox) -> f64 {
    let mut acc = CertifiedInterval::point(0.0);
    for iv in n {
        let mig = if iv.lo > 0.0 {
            iv.lo
        } else if iv.hi < 0.0 {
            -iv.hi
        } else {
            0.0
        };
        let mig_iv = CertifiedInterval::point(mig);
        acc = acc.add(&mig_iv.mul(&mig_iv));
    }
    match acc.sqrt() {
        Some(norm) => norm.lo,
        None => 0.0,
    }
}

/// A certified upper bound of `max_{x ∈ box} |x|` over a normal enclosure box.
fn max_norm_ub(n: &NormalBox) -> f64 {
    let mut acc = CertifiedInterval::point(0.0);
    for iv in n {
        let far = iv.lo.abs().max(iv.hi.abs());
        let far_iv = CertifiedInterval::point(far);
        acc = acc.add(&far_iv.mul(&far_iv));
    }
    match acc.sqrt() {
        Some(norm) => norm.hi,
        None => f64::INFINITY,
    }
}

/// The certified range of `min_{B1 × B2} ‖n_X × n_Y‖` for one product cell
/// whose two normal enclosures separate, `None` when either cone is unavailable
/// or the two cones overlap (the cell is undecided).
///
/// The certificate: with `s_X`, `s_Y` the certified sine bounds of the two
/// cones about their anchors `m_X`, `m_Y`, and `sin α = ‖m̂_X × m̂_Y‖` the sine
/// of the (projective) angle between the anchors, the strict certified
/// inequality `sin α > s_X + s_Y` implies the two direction sets are disjoint
/// in the projective metric (`d_proj > θ_X + θ_Y`, the superadditivity of
/// `asin`), so for every `n_X`, `n_Y`, `‖n_X × n_Y‖ ≥ |n_X||n_Y| sin δ` with a
/// strictly positive `δ`. The returned lower bound is
/// `min|n_X| · min|n_Y| · (sin α − s_X − s_Y)` (each factor certified).
fn cell_margin(nx: &NormalBox, ny: &NormalBox) -> Option<(f64, f64)> {
    let cx = cone_of_box(*nx)?;
    let cy = cone_of_box(*ny)?;
    let (mx, my) = (cx.m, cy.m);
    let norm_x = norm_of(&mx)?;
    let norm_y = norm_of(&my)?;
    // sin α = ‖m_X × m_Y‖ / (|m_X| |m_Y|), certified interval.
    let cross = [
        CertifiedInterval::point(mx[1])
            .mul(&CertifiedInterval::point(my[2]))
            .sub(
                &CertifiedInterval::point(mx[2]).mul(&CertifiedInterval::point(my[1])),
            ),
        CertifiedInterval::point(mx[2])
            .mul(&CertifiedInterval::point(my[0]))
            .sub(
                &CertifiedInterval::point(mx[0]).mul(&CertifiedInterval::point(my[2])),
            ),
        CertifiedInterval::point(mx[0])
            .mul(&CertifiedInterval::point(my[1]))
            .sub(
                &CertifiedInterval::point(mx[1]).mul(&CertifiedInterval::point(my[0])),
            ),
    ];
    let cross2 = cross[0]
        .mul(&cross[0])
        .add(&cross[1].mul(&cross[1]))
        .add(&cross[2].mul(&cross[2]));
    let cross_norm = cross2.sqrt()?;
    let denom = norm_x.mul(&norm_y);
    let ratio = cross_norm.div(&denom)?;
    let sin_alpha_lb = ratio.lo;
    let s_sum = cx.s_up + cy.s_up;
    if !sin_alpha_lb.is_finite() || !(sin_alpha_lb > s_sum) {
        return None;
    }
    // min |n_X| · min |n_Y| · (sin α − s_X − s_Y): certified lower bound of
    // the minimal ‖n_X × n_Y‖ over the product cell.
    let nx_lb = CertifiedInterval::point(min_norm_lb(nx));
    let ny_lb = CertifiedInterval::point(min_norm_lb(ny));
    let gap = CertifiedInterval::point(sin_alpha_lb)
        .sub(&CertifiedInterval::point(cx.s_up))
        .sub(&CertifiedInterval::point(cy.s_up));
    let lo_iv = nx_lb.mul(&ny_lb).mul(&gap);
    let hi_iv = CertifiedInterval::point(max_norm_ub(nx)).mul(&CertifiedInterval::point(
        max_norm_ub(ny),
    ));
    if !lo_iv.is_finite() || !hi_iv.is_finite() || !(lo_iv.lo > 0.0) {
        return None;
    }
    Some((lo_iv.lo, hi_iv.hi))
}

/// The certified norm enclosure `[|m|_lo, |m|_hi]` of a float vector `m`.
fn norm_of(m: &[f64; 3]) -> Option<CertifiedInterval> {
    let mut acc = CertifiedInterval::point(0.0);
    for v in m {
        let iv = CertifiedInterval::point(*v);
        acc = acc.add(&iv.mul(&iv));
    }
    acc.sqrt()
}

/// The total parameter volume of a list of undecided product cells.
fn undecided_volume(cells: &[(SideBox, SideBox)]) -> f64 {
    let mut vol = 0.0f64;
    for (b1, b2) in cells {
        vol += (b1[0].1 - b1[0].0)
            * (b1[1].1 - b1[1].0)
            * (b2[0].1 - b2[0].0)
            * (b2[1].1 - b2[1].0);
    }
    vol
}

/// The two closed halves of a unit interval.
fn bisect(interval: (f64, f64)) -> [(f64, f64); 2] {
    let mid = 0.5 * (interval.0 + interval.1);
    [(interval.0, mid), (mid, interval.1)]
}

/// The four quadrants of one side's box.
fn split_side(b: SideBox) -> [SideBox; 4] {
    let (u, v) = (b[0], b[1]);
    let us = bisect(u);
    let vs = bisect(v);
    [
        [us[0], vs[0]],
        [us[1], vs[0]],
        [us[0], vs[1]],
        [us[1], vs[1]],
    ]
}

/// The sixteen product children of an undecided cell (each side quartered).
fn split_cell(cell: &(SideBox, SideBox)) -> Vec<(SideBox, SideBox)> {
    let (b1, b2) = *cell;
    let mut out = Vec::with_capacity(16);
    for s1 in split_side(b1) {
        for s2 in split_side(b2) {
            out.push((s1, s2));
        }
    }
    out
}

/// Classify the suspicion halt from the undecided-measure history.
///
/// `history[k]` is the undecided parameter volume after the level-`k`
/// certification pass. Area-scale stagnation (`u_L / u_0` above
/// [`STAGNATION_RATIO`]) means the undecided region kept full dimension (a
/// coincident pair): [`SsiRefusal::CoincidentPatchSuspected`]. A positive
/// region that shrank (but never resolved within budget) is a lower-dimensional
/// tangency locus: [`SsiRefusal::TangentCurveSuspected`]. The payload is
/// `(u_L, exponent)` with `exponent = log2(u_0/u_L)/L` the observed per-level
/// shrink exponent (evidence, never a tolerance).
fn suspicion(history: &[f64]) -> SsiRefusal {
    let u0 = history.first().copied().unwrap_or(0.0);
    let ul = history.last().copied().unwrap_or(0.0);
    let levels = history.len().saturating_sub(1).max(1) as f64;
    let fraction = if u0.is_finite() && ul.is_finite() && u0 > 0.0 {
        (ul / u0).min(1.0)
    } else {
        1.0
    };
    let exponent = if ul > 0.0 && u0 > 0.0 && ul < u0 {
        (u0 / ul).log2() / levels
    } else {
        0.0
    };
    if fraction > STAGNATION_RATIO {
        SsiRefusal::CoincidentPatchSuspected {
            margin: (ul, exponent),
        }
    } else {
        SsiRefusal::TangentCurveSuspected {
            margin: (ul, exponent),
        }
    }
}

/// Run the level-driven subdivision of a product box to a verdict.
///
/// The two providers give the certified normal-component enclosures of each
/// side over any unit-chart sub-box; `b1`, `b2` are the initial unit-chart
/// side boxes. The gate certifies [`GateAdmission::TangencyFree`] once every
/// cell separates, refuses a typed suspicion once the admission budget
/// exhausts with undecided measure, and returns
/// [`GateAdmission::Subdivide`] only for a single-level probe
/// (`params.max_level == 0`).
fn drive<F, G>(
    side_a: &F,
    side_b: &G,
    b1: SideBox,
    b2: SideBox,
    params: &GateParams,
) -> Result<GateAdmission, SsiRefusal>
where
    F: Fn(SideBox) -> Result<NormalBox, SsiRefusal>,
    G: Fn(SideBox) -> Result<NormalBox, SsiRefusal>,
{
    let mut cells = vec![(b1, b2)];
    let mut history: Vec<f64> = Vec::new();
    // Aggregate certified margin over the box: the certified enclosure of the
    // minimal ‖n_X × n_Y‖ (smallest cell lower bound, largest cell upper).
    let mut margin_lo = f64::INFINITY;
    let mut margin_hi = f64::NEG_INFINITY;
    let mut level = 0usize;
    loop {
        let mut undecided: Vec<(SideBox, SideBox)> = Vec::new();
        for cell in &cells {
            let nx = side_a(cell.0)?;
            let ny = side_b(cell.1)?;
            match cell_margin(&nx, &ny) {
                Some((lo, hi)) => {
                    margin_lo = margin_lo.min(lo);
                    margin_hi = margin_hi.max(hi);
                }
                None => undecided.push(*cell),
            }
        }
        if undecided.is_empty() {
            if !margin_lo.is_finite() || !margin_hi.is_finite() {
                return Err(SsiRefusal::InvalidInput);
            }
            return Ok(GateAdmission::TangencyFree {
                margin: (margin_lo, margin_hi),
            });
        }
        if params.max_level == 0 {
            return Ok(GateAdmission::Subdivide);
        }
        history.push(undecided_volume(&undecided));
        if level + 1 >= params.max_level {
            return Err(suspicion(&history));
        }
        let mut children = Vec::new();
        for cell in &undecided {
            children.extend(split_cell(cell));
        }
        if children.len() as u64 > params.max_cells {
            return Err(suspicion(&history));
        }
        cells = children;
        level += 1;
    }
}

/// The separable tangency-free admission gate (FSSI-001-GATE entry).
///
/// `side_a` / `side_b` certify the normal-component enclosure of each surface
/// over any unit-chart sub-box of its own parameter square; the gate decides
/// the product box `b1 × b2`. Returns [`GateAdmission::TangencyFree`] (rank
/// `DF = 3` certified on `Σ ∩ B`), [`GateAdmission::Subdivide`] for a
/// single-level probe, or a typed refusal — the two suspicion halts
/// ([`SsiRefusal::TangentCurveSuspected`],
/// [`SsiRefusal::CoincidentPatchSuspected`]) at the admission budget, or a
/// landed named cause for a construction failure. Deterministic: fixed level
/// order, fixed cell-split order, no randomness.
pub fn gate_admit<F, G>(
    side_a: F,
    side_b: G,
    b1: SideBox,
    b2: SideBox,
    params: &GateParams,
) -> Result<GateAdmission, SsiRefusal>
where
    F: Fn(SideBox) -> Result<NormalBox, SsiRefusal>,
    G: Fn(SideBox) -> Result<NormalBox, SsiRefusal>,
{
    drive(&side_a, &side_b, b1, b2, params)
}

/// The composed normal net of one `SquareSystem3` side (FSSI-ELEV: exact
/// Bernstein product of the derivative nets — never the naive pairing), with
/// its bidegree.
struct SystemSideNets {
    /// First-axis degree of the composed normal net (`2m − 1`).
    dm: usize,
    /// Second-axis degree of the composed normal net (`2n − 1`).
    dn: usize,
    /// The three normal-component coefficient grids.
    net: [Vec<Vec<f64>>; 3],
}

/// Build the composed normal net of one side, refusing a non-D2 (non-unit
/// weight) carrier: the gate never dehomogenizes, and the spline-admissible
/// admission family is the unit-weight D2 family.
fn system_side_nets(carrier: &SideCarrier) -> Result<SystemSideNets, SsiRefusal> {
    if carrier
        .weights()
        .iter()
        .any(|row| row.iter().any(|w| *w != 1.0))
    {
        return Err(SsiRefusal::InvalidInput);
    }
    let (m, n) = (carrier.m(), carrier.n());
    if m == 0 || n == 0 {
        return Err(SsiRefusal::InvalidInput);
    }
    let net = super::side_normal_nets(carrier)?;
    Ok(SystemSideNets {
        dm: 2 * m - 1,
        dn: 2 * n - 1,
        net,
    })
}

/// The certified normal enclosure of a prebuilt composed net over a unit-chart
/// sub-box.
fn hull_system_nets(nets: &SystemSideNets, box2: SideBox) -> Result<NormalBox, SsiRefusal> {
    let mut out = [CertifiedInterval::point(0.0); 3];
    for (k, cell) in out.iter_mut().enumerate() {
        let hull = super::hull_bivariate(&nets.net[k], nets.dm, nets.dn, box2[0], box2[1])?;
        *cell = hull;
    }
    Ok(out)
}

/// Gate one `SquareSystem3` pair over a trace box (the wiring at SquareSystem3
/// admission).
///
/// `box_` is a four-interval box in the stored chart coordinates (each axis a
/// compact subset of that axis's chart rectangle); it is mapped to the two
/// sides' unit charts and decided by [`gate_admit`]. Boxes that certify
/// TangencyFree proceed exactly as today; the gate is monotone-widening — it
/// can only admit or refuse, never flip a landed certified verdict.
pub fn gate_square_system(
    system: &SquareSystem3,
    box_: [(f64, f64); 4],
    params: &GateParams,
) -> Result<GateAdmission, SsiRefusal> {
    if params.max_cells == 0 {
        return Err(SsiRefusal::InvalidInput);
    }
    let nets_a = system_side_nets(system.side_a())?;
    let nets_b = system_side_nets(system.side_b())?;
    let unit = unit_box(system, box_)?;
    let side_a = |b2: SideBox| hull_system_nets(&nets_a, b2);
    let side_b = |b2: SideBox| hull_system_nets(&nets_b, b2);
    gate_admit(
        side_a,
        side_b,
        [unit[0], unit[1]],
        [unit[2], unit[3]],
        params,
    )
}
