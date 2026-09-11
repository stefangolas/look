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

//! The A₂ (Morse–Bott branch) certificate and its rank-3 continuation adapter
//! (CTE-005-A2; theory §2.10, T1.7 / R6).
//!
//! When the contact set is a curve, the deflated system `T = (G₁, G₂, M₁, M₂)`
//! has a critical MANIFOLD of roots and the landed Krawczyk operator can never
//! contract. That non-contraction is the A₂ signal: this module certifies the
//! T1.7 contact-factor identity
//!
//! ```text
//!   s·f = q²a + u₁G₁ + u₂G₂            (exact over ℚ)
//! ```
//!
//! together with the interval certificates `0 ∉ ŝ(B)`, `0 ∉ â(B)`, a rank-3
//! separation of one `3 x 3` minor of `D(G₁, G₂, q)` over the box, a certified
//! NONEMPTY branch piece (the R6 correction: vacuous satisfaction refuses), and
//! rank-3 parallelotope continuation on `C = (G₁, G₂, q)` that fills the
//! [`A2BranchCurve`](crate::tangency::shapes::A2BranchCurve) with certified
//! samples and per-sample frames.
//!
//! # Arnold naming (RDEF-M3-WITNESS-TIER; spec CHK-5 note)
//!
//! In Arnold's classification `A_k` denotes an ISOLATED singularity: `A1` is
//! the Morse point and `A2` the cusp. This module's `A2`/`A2Branch` label is
//! the codebase's historical name for NON-isolated Morse–Bott contact along a
//! branch — the lattice-v2 `tangent_curve` class. The convention is KEPT (a
//! rename would ripple through the frozen CTE-000 vocabulary and every caller
//! of [`ContactVerdict::A2Branch`]); the normative lattice class name is
//! `tangent_curve`, and this note is the documented mapping. The `A1Isolated`
//! and `A1Node` arms are genuine Arnold `A1` points (Morse), so the `A1`
//! spelling is correct for them.
//!
//! **Continuation is an adapter, not new algebra** (scope decision 1): the
//! landed `parallelotope.rs` tracker and the landed
//! [`KrawczykSystem`](truck_evidence::num::krawczyk::KrawczykSystem) operator
//! drive `C = (G₁, G₂, q)` exactly as `construct/bie/ssi4.rs` drives its
//! systems. `num/{parallelotope,krawczyk}.rs` are instantiated, never edited.
//! **`q` arrives, it is not searched** (scope decision 2): this packet
//! VERIFIES a supplied `q` (provenance: construction data); verification is
//! provenance-independent. **Nonemptiness is REQUIRED** (theory R6): without a
//! certified piece on `{G = q = 0} ∩ B` the module emits
//! [`ContactVerdict::Empty`](crate::tangency::shapes::ContactVerdict::Empty),
//! never `A2Branch`. **Non-contraction is the router** (scope decision 4): the
//! cascade's stage-6 `T` non-contraction evidence is a
//! [`NonContractionDiagnostic`] carried into the attempt.
//!
//! # Exact data carrier
//!
//! The one-witness verifier (CTE-001) proves the exact identity over the
//! [`QPoly`] ring, whose term map is private to `qpoly.rs`. The certified
//! interval work of this module needs the identity's factor polynomials
//! (`q`, `s`, `a`) as READABLE monomial data, so the caller supplies them in
//! the module's own exact term carrier [`ExactPoly`] ([`A2ExactData`]), and
//! the module cross-checks the carrier against the authoritative [`QPoly`]
//! witness (the carrier's rebuilt `QPoly` must equal the witness's recorded
//! polynomials EXACTLY) before any certificate is emitted. The `G₁, G₂, f`
//! polynomials are bound to the stored system: the recorded `G₁, G₂` must
//! equal two of the stored component polynomials exactly (an EXACT
//! Bernstein→monomial expansion of the stored grids), and the identity is
//! verified against the stored `f` (API note: the plan's sketch passed a bare
//! `q: &QPoly`; the landed shape needs the readable carrier as well).
//!
//! The certified interval work for the two `G` rows rides the landed
//! `partial_enclosure` / `certified_value` discipline over the stored grids;
//! only the `q` row and the multipliers are carried as exact monomial data.
//!
//! # House rules
//!
//! - **H-1.** No `unwrap`, `expect`, `panic!` or out-of-range indexing is
//!   reachable from geometry. Fixed-size matrix algebra indexes only constant
//!   or iterator-derived positions.
//! - **H-2.** Every fallible operation returns
//!   `Result<_, TangencyRefusal>` wrapping the landed named causes.
//! - **H-6.** Float-computed tangents, predictions, and sample centres are
//!   never recorded as `Method::Exact`: the certified statement is always the
//!   Krawczyk/parallelotope box, and every certified sample carries its box.
//! - **Determinism.** Fixed candidate order in the split search and the seed
//!   scan (axis `0..4`, value order mid/lo/hi), fixed column-triple order in
//!   the rank-3 search, fixed parallelotope cadence, and no hash ordering in
//!   any output.
//!
//! **Frozen shapes.** The certificate vocabulary
//! ([`A2Cert`](crate::tangency::shapes::A2Cert),
//! [`RankThreeMinorCert`](crate::tangency::shapes::RankThreeMinorCert),
//! [`A2NonemptyEvidence`](crate::tangency::shapes::A2NonemptyEvidence),
//! [`A2BranchCurve`](crate::tangency::shapes::A2BranchCurve),
//! [`A2BranchSample`](crate::tangency::shapes::A2BranchSample),
//! [`ContactVerdict`](crate::tangency::shapes::ContactVerdict)) is CTE-000's
//! freeze and is never restated here. The frozen [`A2Cert`] does not carry the
//! produced branch curve, so the produced curve and its per-sample
//! certificates ride the module-level [`A2Attestation`]; [`certify_a2`]
//! returns the frozen [`ContactVerdict`] contract.

use crate::contract::Refusal;
use crate::formal::exact::CertifiedInterval;
use crate::hull::HullRefusal;
use crate::ssi::{partial_enclosure, SsiRefusal};
use crate::ssi_types::{KrawczykCertificate3, SquareSystem3};
use crate::tangency::chart::g_rows_of;
use crate::tangency::exclude::Box4;
use crate::tangency::qpoly::{QCoeff, QPoly};
use crate::tangency::shapes::{
    A2BranchCurve, A2BranchSample, A2Cert, A2NonemptyEvidence, ChartMinorGrids, ContactVerdict,
    ExactVanishingWitness, ExactWitnessVerifier, RankThreeMinorCert, WitnessSide,
};
use crate::tangency::TangencyRefusal;
use truck_base::evidence::{Budget, Certificate, Certified};
use truck_evidence::enclosure::Interval;
use truck_evidence::num::krawczyk::{krawczyk, KrawczykProof, KrawczykSystem};
use truck_evidence::num::parallelotope::{
    box_around, theta_rho_step, ParallelotopeFrame, StepVerdict,
};

/// The subdivision budget of one seed/step certification attempt (an
/// algorithmic bound, never a numeric predicate tolerance).
const ATTEMPT_BUDGET: u32 = 4096;

/// The certified half-width of a seed query box, as a fraction of the free
/// axis width (dimensionless — H-3).
const SEED_RADIUS_FRACTION: f64 = 0.05; // H-3: dimensionless seed box half-width

/// The θ advance of one continuation step along the unit branch tangent
/// (dimensionless parameter-space advance — H-3).
const THETA_STEP: f64 = 0.04; // H-3: θ-step length in parameter units

/// The parallelotope half-width of one certified continuation box as a
/// fraction of the box's travel window (dimensionless — H-3).
const STEP_RADIUS_FRACTION: f64 = 0.02; // H-3: parallelotope half-width in parameter units

/// The maximum number of certified continuation steps on one half-branch.
const MAX_TRACE_STEPS: usize = 48;

/// The number of parallelotope-radius shrink attempts on a failing query.
const RADIUS_SHRINKS: usize = 5;

/// The Newton convergence threshold (H-3: float Newton, dimensionless).
const NEWTON_EPS: f64 = 1.0e-12; // H-3

// ---------------------------------------------------------------------------
// Exact monomial data (the introspectable carrier of the identity factors)
// ---------------------------------------------------------------------------

/// An exact polynomial over the four unit-chart variables as a deterministic
/// monomial term list (ascending lexicographic exponent order, no zero
/// coefficient). This is the READABLE exact carrier of the identity's factor
/// polynomials (`q`, `s`, `a`); the authoritative [`QPoly`] witness is
/// cross-checked against it by exact equality of the rebuilt ring value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExactPoly {
    /// The ordered terms `(exponents, coefficient)`.
    terms: Vec<(Vec<u32>, QCoeff)>,
}

impl ExactPoly {
    /// The zero polynomial over the four chart variables.
    pub fn zero() -> Self {
        ExactPoly { terms: Vec::new() }
    }

    /// Build a polynomial from raw monomial terms. Refuses a term whose
    /// exponent vector is not of length 4.
    pub fn from_terms(terms: Vec<(Vec<u32>, QCoeff)>) -> Result<Self, TangencyRefusal> {
        let mut merged: Vec<(Vec<u32>, QCoeff)> = Vec::with_capacity(terms.len());
        for (exp, coeff) in terms {
            if exp.len() != 4 {
                return Err(TangencyRefusal::Input(Refusal::InvalidInput));
            }
            if coeff.is_zero() {
                continue;
            }
            match merged.iter_mut().find(|(m, _)| *m == exp) {
                Some((_, slot)) => {
                    let num = slot
                        .num()
                        .checked_mul(coeff.den())
                        .and_then(|v| v.checked_add(coeff.num() * slot.den()))
                        .ok_or(TangencyRefusal::Input(Refusal::InvalidInput))?;
                    let den = slot
                        .den()
                        .checked_mul(coeff.den())
                        .ok_or(TangencyRefusal::Input(Refusal::InvalidInput))?;
                    *slot = QCoeff::new(num, den)
                        .map_err(|_| TangencyRefusal::Input(Refusal::InvalidInput))?;
                }
                None => merged.push((exp, coeff)),
            }
        }
        merged.retain(|(_, c)| !c.is_zero());
        merged.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(ExactPoly { terms: merged })
    }

    /// The constant polynomial.
    pub fn constant(c: QCoeff) -> Self {
        let mut terms = Vec::new();
        if !c.is_zero() {
            terms.push((vec![0, 0, 0, 0], c));
        }
        ExactPoly { terms }
    }

    /// The terms, verbatim.
    pub fn terms(&self) -> &[(Vec<u32>, QCoeff)] {
        &self.terms
    }

    /// The exact first partial along one chart axis.
    pub fn partial(&self, axis: usize) -> Result<ExactPoly, TangencyRefusal> {
        if axis > 3 {
            return Err(TangencyRefusal::Input(Refusal::InvalidInput));
        }
        let mut out = Vec::new();
        for (exp, coeff) in &self.terms {
            let e = exp[axis];
            if e == 0 {
                continue;
            }
            let mut ne = exp.clone();
            ne[axis] = e - 1;
            let scaled = scale_coeff(*coeff, e as i128)
                .map_err(|_| TangencyRefusal::Input(Refusal::InvalidInput))?;
            out.push((ne, scaled));
        }
        ExactPoly::from_terms(out)
    }

    /// The exact ring value over the four-variable ring, for the verifier and
    /// the cross-checks.
    pub fn to_qpoly(&self) -> Result<QPoly, TangencyRefusal> {
        let mut acc = QPoly::zero(4);
        for (exp, coeff) in &self.terms {
            let term = QPoly::term(4, exp, *coeff)
                .map_err(|_| TangencyRefusal::Input(Refusal::InvalidInput))?;
            acc = acc
                .add(&term)
                .map_err(|_| TangencyRefusal::Input(Refusal::InvalidInput))?;
        }
        Ok(acc)
    }

    /// The certified interval enclosure over a unit-chart box.
    pub fn interval(&self, b: &Box4) -> Result<CertifiedInterval, TangencyRefusal> {
        qpoly_exact_interval(&self.terms, b)
    }

    /// The plain-`f64` value at a unit-chart point (bookkeeping only).
    pub fn float(&self, p: [f64; 4]) -> Option<f64> {
        let mut acc = 0.0f64;
        for (exp, coeff) in &self.terms {
            let mut term = qcoeff_float(*coeff);
            for (e, x) in exp.iter().zip(p.iter()) {
                term *= x.powi(*e as i32);
            }
            acc += term;
        }
        if acc.is_finite() {
            Some(acc)
        } else {
            None
        }
    }
}

/// The exact data of the A₂ identity the caller certifies about: the readable
/// multiplier `s`, the factor `q`, and the multiplier `a`, all over the four
/// unit-chart variables.
#[derive(Debug, Clone, PartialEq)]
pub struct A2ExactData {
    /// The multiplier `s` of the identity (`0 ∉ ŝ(B)` required).
    pub s: ExactPoly,
    /// The quadratic factor `q` (`C = (G₁, G₂, q)` drives the branch).
    pub q: ExactPoly,
    /// The multiplier `a` of the quadratic term (`0 ∉ â(B)` required).
    pub a: ExactPoly,
}

impl A2ExactData {
    /// Assemble the exact data. Infallible once each factor is a valid
    /// four-variable polynomial.
    pub fn new(s: ExactPoly, q: ExactPoly, a: ExactPoly) -> Self {
        A2ExactData { s, q, a }
    }
}

/// Exact multiplication of a rational coefficient by an integer, refusing on
/// `i128` overflow.
fn scale_coeff(c: QCoeff, n: i128) -> Result<QCoeff, crate::tangency::witness::WitnessRefusal> {
    QCoeff::new(
        c.num()
            .checked_mul(n)
            .ok_or(crate::tangency::witness::WitnessRefusal::Overflow)?,
        c.den(),
    )
}

/// The exact dyadic rational a finite `f64` represents (every `f64` is
/// `±m·2^p`). `None` for a non-finite value or a magnitude the `i128`
/// coefficient ring cannot carry.
fn float_to_qcoeff(x: f64) -> Option<QCoeff> {
    if x == 0.0 {
        return Some(QCoeff::zero());
    }
    if !x.is_finite() {
        return None;
    }
    let bits = x.to_bits();
    let sign = if bits >> 63 == 1 { -1i128 } else { 1i128 };
    let exponent = ((bits >> 52) & 0x7FF) as i32;
    let mantissa = bits & 0x000F_FFFF_FFFF_FFFF;
    let (mant2, p) = if exponent == 0 {
        // Subnormal: value = mantissa · 2^(1 − 1023 − 52).
        (mantissa as u128, -1074i32)
    } else {
        // Normal: value = (2^52 + mantissa) · 2^(exponent − 1023 − 52).
        ((1u128 << 52) | mantissa as u128, exponent - 1075)
    };
    if p >= 0 {
        let shift = p as u32;
        if shift >= 128 {
            return None;
        }
        let (scaled, overflow) = mant2.overflowing_shl(shift);
        if overflow || scaled > i128::MAX as u128 {
            return None;
        }
        QCoeff::new(sign * scaled as i128, 1).ok()
    } else {
        let shift = (-p) as u32;
        if shift >= 128 {
            return None;
        }
        let den = 1u128.checked_shl(shift)?;
        QCoeff::new(sign * mant2 as i128, den as i128).ok()
    }
}

/// A sound `CertifiedInterval` enclosure of an exact rational coefficient.
///
/// The `i128 → f64` conversions and the division are each correctly rounded
/// (≤ ½ ulp each), so the total relative error of the float quotient is
/// bounded by `2^-51`; widening by 8 ulps (`2^-49`) covers it in the normal
/// range, and in the subnormal range the ulp widening bounds the absolute
/// error. A non-finite quotient degrades to a full enclosure (containing the
/// true value) — a conservative surface, never an unsound claim.
fn qcoeff_iv(c: QCoeff) -> CertifiedInterval {
    if c.is_zero() {
        return CertifiedInterval::point(0.0);
    }
    let n = c.num();
    let d = c.den();
    let r = n as f64 / d as f64;
    if !r.is_finite() {
        return CertifiedInterval {
            lo: f64::NEG_INFINITY,
            hi: f64::INFINITY,
        };
    }
    let mut lo = r;
    let mut hi = r;
    for _ in 0..8 {
        lo = lo.next_down();
        hi = hi.next_up();
    }
    CertifiedInterval { lo, hi }
}

/// The `k`-th certified interval power of a one-axis interval, by repeated
/// outward-rounded multiplication (sound; exact for `k ∈ {0, 1}`).
fn axis_power_iv(axis: CertifiedInterval, k: u32) -> CertifiedInterval {
    let mut acc = CertifiedInterval::point(1.0);
    for _ in 0..k {
        acc = acc.mul(&axis);
    }
    acc
}

/// Certified interval evaluation of an exact monomial term list over a box
/// whose four axes are compact subintervals of the unit chart.
///
/// Evaluation is in the power (monomial) basis by outward-rounded interval
/// arithmetic — a sound enclosure of the polynomial's range over the box
/// (dependency makes the enclosure loose for high degrees, never unsound).
fn qpoly_exact_interval(
    terms: &[(Vec<u32>, QCoeff)],
    b: &Box4,
) -> Result<CertifiedInterval, TangencyRefusal> {
    for &(lo, hi) in b {
        if !lo.is_finite() || !hi.is_finite() || !(lo >= 0.0 && hi <= 1.0 && lo <= hi) {
            return Err(TangencyRefusal::Hull(HullRefusal::DomainNotCompact));
        }
    }
    let axis: [CertifiedInterval; 4] = [
        CertifiedInterval {
            lo: b[0].0,
            hi: b[0].1,
        },
        CertifiedInterval {
            lo: b[1].0,
            hi: b[1].1,
        },
        CertifiedInterval {
            lo: b[2].0,
            hi: b[2].1,
        },
        CertifiedInterval {
            lo: b[3].0,
            hi: b[3].1,
        },
    ];
    let mut acc = CertifiedInterval::point(0.0);
    for (exp, coeff) in terms {
        let mut term = qcoeff_iv(*coeff);
        for (e, a) in exp.iter().zip(axis.iter()) {
            term = term.mul(&axis_power_iv(*a, *e));
        }
        acc = acc.add(&term);
    }
    Ok(acc)
}

/// The nearest-`f64` reading of an exact rational (bookkeeping only).
fn qcoeff_float(c: QCoeff) -> f64 {
    if c.is_zero() {
        return 0.0;
    }
    let n = c.num();
    let d = c.den();
    n as f64 / d as f64
}

/// The exact Bernstein→monomial expansion of the degree-`d` Bernstein basis
/// element `B^d_a(x)`: the integer coefficient of `x^e`, `e ∈ a..=d`.
fn bern_basis_coeff(d: usize, a: usize, e: usize) -> i128 {
    if e < a || e > d {
        return 0;
    }
    let binom = |n: usize, k: usize| -> i128 {
        if k > n {
            return 0;
        }
        let mut out = 1i128;
        for t in 0..k {
            out = out * ((n - t) as i128) / ((t + 1) as i128);
        }
        out
    };
    let sign = if (e - a).is_multiple_of(2) { 1 } else { -1 };
    sign * binom(d, a) * binom(d - a, e - a)
}

/// The exact unit-chart polynomial a stored component grid represents, as a
/// readable [`ExactPoly`].
///
/// The grid's Bernstein coefficient floats are dyadic rationals (exact under
/// [`float_to_qcoeff`]) and the Bernstein basis expands into monomials with
/// integer coefficients, so the conversion is exact over ℚ. Requires the
/// stored system to carry the identity domain maps (unit chart = chart
/// coordinates).
fn poly_from_grid(system: &SquareSystem3, component: usize) -> Result<ExactPoly, TangencyRefusal> {
    let maps = system.domain_maps();
    let identity = (0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0);
    if maps != identity {
        return Err(TangencyRefusal::Input(Refusal::InvalidInput));
    }
    let grid = system
        .grids()
        .get(component)
        .ok_or(TangencyRefusal::Input(Refusal::InvalidInput))?;
    let (m1, n1, m2, n2) = system.degrees();
    let sp1 = n1 + 1;
    let sp2 = n2 + 1;
    let mut raw: Vec<(Vec<u32>, QCoeff)> = Vec::new();
    for a in 0..=m1 {
        for b in 0..=n1 {
            for i in 0..=m2 {
                for j in 0..=n2 {
                    let value = grid[a * sp1 + b][i * sp2 + j];
                    let coeff = float_to_qcoeff(value)
                        .ok_or(TangencyRefusal::Hull(HullRefusal::EnclosureUnavailable))?;
                    if coeff.is_zero() {
                        continue;
                    }
                    for e0 in 0..=m1 {
                        let c0 = bern_basis_coeff(m1, a, e0);
                        if c0 == 0 {
                            continue;
                        }
                        for e1 in 0..=n1 {
                            let c1 = bern_basis_coeff(n1, b, e1);
                            if c1 == 0 {
                                continue;
                            }
                            for e2 in 0..=m2 {
                                let c2 = bern_basis_coeff(m2, i, e2);
                                if c2 == 0 {
                                    continue;
                                }
                                for e3 in 0..=n2 {
                                    let c3 = bern_basis_coeff(n2, j, e3);
                                    if c3 == 0 {
                                        continue;
                                    }
                                    let scale = c0 * c1 * c2 * c3;
                                    if scale == 0 {
                                        continue;
                                    }
                                    let scaled = scale_coeff(coeff, scale).map_err(|_| {
                                        TangencyRefusal::Input(Refusal::InvalidInput)
                                    })?;
                                    if scaled.is_zero() {
                                        continue;
                                    }
                                    raw.push((
                                        vec![e0 as u32, e1 as u32, e2 as u32, e3 as u32],
                                        scaled,
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    ExactPoly::from_terms(raw)
}

// ---------------------------------------------------------------------------
// Row evaluation substrate (stored component grids and exact polynomials)
// ---------------------------------------------------------------------------

/// One row of a certified system: a stored component grid or an exact
/// monomial polynomial.
#[derive(Clone)]
enum Row {
    /// A stored component of the square system.
    Grid(usize),
    /// An exact monomial polynomial (the `q` row).
    Exact(ExactPoly),
}

/// Map a landed `SsiRefusal` onto the module-level refusal vocabulary.
fn map_ssi(r: SsiRefusal) -> TangencyRefusal {
    match r {
        SsiRefusal::Hull(h) => TangencyRefusal::Hull(h),
        SsiRefusal::InvalidInput
        | SsiRefusal::DeterminantSpansZero
        | SsiRefusal::InclusionNotStrict
        | SsiRefusal::Conditioning(_)
        | SsiRefusal::PairClass(_)
        // FSSI-000 decision 4 (FSSI-EXT): the FSSI-001 gate's two suspicion
        // halts fold onto the existing low-information
        // `Input(Refusal::InvalidInput)` pattern — NO new `TangencyRefusal`
        // variant. The SSI-layer `tag()` carries the specificity the
        // FSSI-001/002 producers assert against; the arms cannot fire until
        // those producers wire them.
        | SsiRefusal::TangentCurveSuspected { .. }
        | SsiRefusal::CoincidentPatchSuspected { .. } => TangencyRefusal::Input(Refusal::InvalidInput),
    }
}

/// The certified enclosure of a row over a chart-coordinate box.
fn row_cert_value(
    system: &SquareSystem3,
    row: &Row,
    b: &Box4,
) -> Result<CertifiedInterval, TangencyRefusal> {
    match row {
        Row::Grid(c) => crate::tangency::chart::certified_value(system, *c, *b).map_err(map_ssi),
        Row::Exact(p) => p.interval(b),
    }
}

/// The certified enclosure of a row's first partial along a chart axis over a
/// box.
fn row_cert_partial(
    system: &SquareSystem3,
    row: &Row,
    axis: usize,
    b: &Box4,
) -> Result<CertifiedInterval, TangencyRefusal> {
    match row {
        Row::Grid(c) => partial_enclosure(system, *c, axis, *b).map_err(map_ssi),
        Row::Exact(p) => p.partial(axis)?.interval(b),
    }
}

/// The plain-`f64` value of a row at a chart point (bookkeeping and float
/// preconditioners only — never a certified value).
fn row_float_value(system: &SquareSystem3, row: &Row, p: [f64; 4]) -> Option<f64> {
    match row {
        Row::Grid(c) => {
            let uvst = (p[0], p[1], p[2], p[3]);
            crate::ssi_fixtures::eval_grid4(&system.grids()[*c], system.degrees(), uvst)
        }
        Row::Exact(poly) => poly.float(p),
    }
}

/// The plain-`f64` first partial of a row along a chart axis at a point.
fn row_float_partial(system: &SquareSystem3, row: &Row, axis: usize, p: [f64; 4]) -> Option<f64> {
    match row {
        Row::Grid(c) => crate::tangency::chart::float_partial(system, *c, axis, p),
        Row::Exact(poly) => poly.partial(axis).ok()?.float(p),
    }
}

// ---------------------------------------------------------------------------
// The frozen certificate carriers and the module-level attestation
// ---------------------------------------------------------------------------

/// The §2.10 stage-6 diagnostic: a cell where the deflated system `T` kept
/// shrinking without excluding or contracting across the cascade's stage-6
/// attempt. Routing this evidence into the A₂ attempt is the §2.10 diagnostic
/// (scope decision 4); the cell is where the A₂ identity is attempted.
#[derive(Debug, Clone, PartialEq)]
pub struct NonContractionDiagnostic {
    /// The cell that resisted `T`'s contraction.
    cell: Box4,
    /// The subdivisions the stage-6 attempt spent before routing.
    spend: u32,
}

impl NonContractionDiagnostic {
    /// Build the diagnostic, refusing a non-finite/misordered cell.
    pub fn new(cell: Box4, spend: u32) -> Result<Self, TangencyRefusal> {
        for (lo, hi) in cell {
            if !lo.is_finite() || !hi.is_finite() || lo > hi {
                return Err(TangencyRefusal::Input(Refusal::InvalidInput));
            }
        }
        Ok(NonContractionDiagnostic { cell, spend })
    }

    /// The cell that resisted `T`'s contraction.
    pub fn cell(&self) -> Box4 {
        self.cell
    }

    /// The subdivision spend of the routing attempt.
    pub fn spend(&self) -> u32 {
        self.spend
    }
}

/// One certified sample of an A₂ branch: the frozen sample record plus the
/// certified box that carries it (H-6: the certified statement is the box; the
/// float point and tangent are bookkeeping).
#[derive(Debug, Clone)]
pub struct A2CertifiedSample {
    /// The frozen sample record (point + unit tangent).
    pub sample: A2BranchSample,
    /// The certified 4-D box: exactly one branch point of `C = (G₁, G₂, q)`
    /// lies in it.
    pub cell: Box4,
    /// The Krawczyk certificate of the box.
    pub cert: Certificate,
}

/// The full A₂ certificate outcome: the frozen [`ContactVerdict`] plus the
/// produced branch curve and its per-sample certificates.
///
/// The frozen [`A2Cert`] shape does not carry the produced [`A2BranchCurve`]
/// (CTE-000 froze the curve as a separate T1→T2 record), so the producing
/// packet returns the curve and its certificates on this attestation; the
/// frozen verdict contract is exposed through [`certify_a2`].
#[derive(Debug, Clone)]
pub struct A2Attestation {
    /// The five-way contact verdict (`A2Branch` or `Empty` on the certified
    /// paths).
    pub verdict: ContactVerdict<QPoly>,
    /// The produced branch curve, present exactly on the `A2Branch` path.
    pub curve: Option<A2BranchCurve>,
    /// The certified samples along the produced curve, in curve order.
    pub samples: Vec<A2CertifiedSample>,
    /// The routing evidence that produced this attempt, when routed from a
    /// stage-6 `T` non-contraction.
    pub diagnostic: Option<NonContractionDiagnostic>,
}

// ---------------------------------------------------------------------------
// The certified split search: which stored component is `f`
// ---------------------------------------------------------------------------

/// Resolve which stored component is the identity's distinguished `f`.
///
/// The witness records `G₁, G₂` exactly (its two correction terms); the stored
/// components whose exact monomial forms equal those two ring values are
/// `G₁, G₂`, and the remaining component is `f`. Deterministic: the candidate
/// `f` index ascends, and within a candidate the two `G` rows are the
/// complement in ascending component order.
fn resolve_split(
    system: &SquareSystem3,
    witness: &ExactVanishingWitness<QPoly>,
) -> Result<(usize, [usize; 2]), TangencyRefusal> {
    let comps = [
        poly_from_grid(system, 0)?.to_qpoly()?,
        poly_from_grid(system, 1)?.to_qpoly()?,
        poly_from_grid(system, 2)?.to_qpoly()?,
    ];
    let terms = witness.terms();
    if terms.len() != 2 {
        return Err(TangencyRefusal::Input(Refusal::InvalidInput));
    }
    let r0 = &terms[0].1;
    let r1 = &terms[1].1;
    for f_comp in 0..3usize {
        let gs = g_rows_of(f_comp);
        let a = &comps[gs[0]];
        let b = &comps[gs[1]];
        let matches = (a == r0 && b == r1) || (a == r1 && b == r0);
        if matches {
            return Ok((f_comp, gs));
        }
    }
    Err(TangencyRefusal::Input(Refusal::InvalidInput))
}

// ---------------------------------------------------------------------------
// Certified interval helpers over a four-axis box
// ---------------------------------------------------------------------------

/// The certified `3 x 3` determinant of an interval matrix (the S2A cofactor
/// expansion, matching `kernel/minor_algebra.rs`'s `det3_iv` op order).
fn det3_iv(m: &[[CertifiedInterval; 3]; 3]) -> CertifiedInterval {
    let a = m[0][0].mul(&m[1][1].mul(&m[2][2]).sub(&m[1][2].mul(&m[2][1])));
    let b = m[0][1].mul(&m[1][0].mul(&m[2][2]).sub(&m[1][2].mul(&m[2][0])));
    let c = m[0][2].mul(&m[1][0].mul(&m[2][1]).sub(&m[1][1].mul(&m[2][0])));
    a.sub(&b).add(&c)
}

/// The four ascending column triples of a `3 x 4` matrix.
const COLUMN_TRIPLES: [(usize, usize, usize); 4] = [(0, 1, 2), (0, 1, 3), (0, 2, 3), (1, 2, 3)];

/// Whether a certified interval strictly excludes zero.
fn strictly_excludes_zero(enc: &CertifiedInterval) -> bool {
    enc.is_finite() && !(enc.lo <= 0.0 && 0.0 <= enc.hi)
}

/// The certified `3 x 3` minor of `D(G₁, G₂, q)` over a box on the three
/// given columns. Rows are the two stored `G` components and the exact `q`.
fn minor_enclosure(
    system: &SquareSystem3,
    g_rows: [usize; 2],
    q: &ExactPoly,
    cols: (usize, usize, usize),
    b: &Box4,
) -> Result<CertifiedInterval, TangencyRefusal> {
    let rows: [Row; 3] = [
        Row::Grid(g_rows[0]),
        Row::Grid(g_rows[1]),
        Row::Exact(q.clone()),
    ];
    let mut m = [[CertifiedInterval::point(0.0); 3]; 3];
    for (r, row) in rows.iter().enumerate() {
        let axis_cols = [cols.0, cols.1, cols.2];
        for (c, &axis) in axis_cols.iter().enumerate() {
            let partial = row_cert_partial(system, row, axis, b)?;
            if !partial.is_finite() {
                return Err(TangencyRefusal::Hull(HullRefusal::EnclosureUnavailable));
            }
            m[r][c] = partial;
        }
    }
    Ok(det3_iv(&m))
}

/// Certify the T1.7 rank-3 hypothesis: the first (ascending) `3 x 3` minor of
/// `D(G₁, G₂, q)` strictly separated from zero over the box.
fn certify_rank3(
    system: &SquareSystem3,
    g_rows: [usize; 2],
    q: &ExactPoly,
    b: &Box4,
) -> Result<RankThreeMinorCert, TangencyRefusal> {
    for triple in COLUMN_TRIPLES {
        let enc = minor_enclosure(system, g_rows, q, triple, b)?;
        if strictly_excludes_zero(&enc) {
            return RankThreeMinorCert::new(triple, enc)
                .map_err(|_| TangencyRefusal::Input(Refusal::InvalidInput));
        }
    }
    Err(TangencyRefusal::Input(Refusal::ConditioningBelowThreshold))
}

// ---------------------------------------------------------------------------
// The slice Krawczyk system (seeding on a 3-of-4 slice of {G = q = 0})
// ---------------------------------------------------------------------------

/// The 3-of-4 slice system of `C = (G₁, G₂, q)` with one chart axis fixed.
#[derive(Clone)]
struct CSliceSystem {
    /// The stored square system (for the two `G` component rows).
    system: SquareSystem3,
    /// The two stored `G` component indices.
    g_rows: [usize; 2],
    /// The exact `q` row.
    q: ExactPoly,
    /// The fixed axis (`0..4`).
    fixed_axis: usize,
    /// The fixed coordinate value.
    fixed_value: f64,
}

impl CSliceSystem {
    /// The three free axes in ascending order.
    fn free_axes(&self) -> [usize; 3] {
        let mut out = [0usize; 3];
        let mut k = 0usize;
        for j in 0..4 {
            if j != self.fixed_axis {
                out[k] = j;
                k += 1;
            }
        }
        out
    }

    /// Embed a 3-D free-coordinate point into the 4-D chart.
    fn embed(&self, x: &[f64; 3]) -> [f64; 4] {
        let free = self.free_axes();
        let mut out = [self.fixed_value; 4];
        for (k, &axis) in free.iter().enumerate() {
            out[axis] = x[k];
        }
        out
    }

    /// The rows as row descriptors.
    fn rows(&self) -> [Row; 3] {
        [
            Row::Grid(self.g_rows[0]),
            Row::Grid(self.g_rows[1]),
            Row::Exact(self.q.clone()),
        ]
    }

    /// The three row values at an embedded point.
    fn rows_at(&self, x4: [f64; 4]) -> Option<[f64; 3]> {
        let rows = self.rows();
        let mut out = [0.0f64; 3];
        for (r, row) in rows.iter().enumerate() {
            out[r] = row_float_value(&self.system, row, x4)?;
        }
        Some(out)
    }
}

impl KrawczykSystem<3> for CSliceSystem {
    fn f_point(&self, x: &[f64; 3]) -> [Interval; 3] {
        let x4 = self.embed(x);
        let degenerate: Box4 = [
            (x4[0], x4[0]),
            (x4[1], x4[1]),
            (x4[2], x4[2]),
            (x4[3], x4[3]),
        ];
        let rows = self.rows();
        let mut out = [Interval::EMPTY; 3];
        for (r, row) in rows.iter().enumerate() {
            match row_cert_value(&self.system, row, &degenerate) {
                Ok(enc) => out[r] = certified_to_interval(&enc),
                Err(_) => out[r] = Interval::EMPTY,
            }
        }
        out
    }

    fn jacobian(&self, b: &[Interval; 3]) -> [[Interval; 3]; 3] {
        let free = self.free_axes();
        let box4 = slice_box_to_4(self.fixed_axis, self.fixed_value, b);
        let rows = self.rows();
        let mut out = [[Interval::EMPTY; 3]; 3];
        for (r, row) in rows.iter().enumerate() {
            for (c, &axis) in free.iter().enumerate() {
                match row_cert_partial(&self.system, row, axis, &box4) {
                    Ok(enc) => out[r][c] = certified_to_interval(&enc),
                    Err(_) => out[r][c] = Interval::EMPTY,
                }
            }
        }
        out
    }

    fn preconditioner(&self, x: &[f64; 3]) -> Option<[[f64; 3]; 3]> {
        let free = self.free_axes();
        let x4 = self.embed(x);
        let rows = self.rows();
        let mut m = [[0.0f64; 3]; 3];
        for (r, row) in rows.iter().enumerate() {
            for (c, &axis) in free.iter().enumerate() {
                m[r][c] = row_float_partial(&self.system, row, axis, x4)?;
            }
        }
        invert3(&m)
    }
}

/// A free-axis box embedded into the 4-D chart (the fixed axis degenerate).
#[allow(clippy::needless_range_loop)] // fixed-size 4-axis embedding; the index form is the algebra (ssi4/tsystem precedent)
fn slice_box_to_4(fixed_axis: usize, fixed_value: f64, b: &[Interval; 3]) -> Box4 {
    let mut out = [(fixed_value, fixed_value); 4];
    let mut k = 0usize;
    for j in 0..4 {
        if j == fixed_axis {
            continue;
        }
        out[j] = (b[k].inf(), b[k].sup());
        k += 1;
    }
    out
}

/// A certified interval as an `inari` interval.
fn certified_to_interval(enc: &CertifiedInterval) -> Interval {
    Interval::try_from((enc.lo, enc.hi)).unwrap_or(Interval::EMPTY)
}

/// The float inverse of a 3 x 3 matrix by cofactors over the determinant.
/// `None` on a (near-)singular matrix.
fn invert3(m: &[[f64; 3]; 3]) -> Option<[[f64; 3]; 3]> {
    let det = det3_f(m);
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
            out[r][c] = adj[r][c] / det;
        }
    }
    Some(out)
}

/// The float determinant of a 3 x 3 matrix.
fn det3_f(m: &[[f64; 3]; 3]) -> f64 {
    m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
}

/// The float inverse of a 4 x 4 matrix by Gauss–Jordan with partial pivoting.
/// `None` on a singular or non-finite matrix.
#[allow(clippy::needless_range_loop)] // fixed-size 4x4 Gauss-Jordan index algebra (ssi4/tsystem precedent)
fn invert4(m: &[[f64; 4]; 4]) -> Option<[[f64; 4]; 4]> {
    let mut a = *m;
    let mut inv = [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ];
    for col in 0..4 {
        let mut pivot = col;
        let mut best = a[col][col].abs();
        for r in (col + 1)..4 {
            let cand = a[r][col].abs();
            if cand > best {
                best = cand;
                pivot = r;
            }
        }
        if !best.is_finite() || best == 0.0 {
            return None;
        }
        if pivot != col {
            for c in 0..4 {
                let t = a[col][c];
                a[col][c] = a[pivot][c];
                a[pivot][c] = t;
                let t = inv[col][c];
                inv[col][c] = inv[pivot][c];
                inv[pivot][c] = t;
            }
        }
        let d = a[col][col];
        for c in 0..4 {
            a[col][c] /= d;
            inv[col][c] /= d;
        }
        for r in 0..4 {
            if r == col {
                continue;
            }
            let factor = a[r][col];
            if factor == 0.0 {
                continue;
            }
            for c in 0..4 {
                a[r][c] -= factor * a[col][c];
                inv[r][c] -= factor * inv[col][c];
            }
        }
    }
    if inv.iter().all(|row| row.iter().all(|c| c.is_finite())) {
        Some(inv)
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// The Krawczyk operator's own quantities, re-derived for the certificate
// carriers (the certification itself comes from the landed operator)
// ---------------------------------------------------------------------------

/// The Krawczyk image `K(Q) = m − Y·F(m) + (I − Y·J(Q))·(Q − m)` for N = 3,
/// in the operator's row-major formula (re-derived here only to build the
/// [`KrawczykCertificate3`](crate::ssi_types::KrawczykCertificate3)
/// strict-inclusion carrier).
fn k_image_3(
    q: &[Interval; 3],
    m: &[f64; 3],
    y: &[[f64; 3]; 3],
    f: &[Interval; 3],
    j: &[[Interval; 3]; 3],
) -> [Interval; 3] {
    let iv_at = |x: f64| Interval::try_from((x, x)).unwrap_or(Interval::EMPTY);
    let mut out = [Interval::EMPTY; 3];
    for r in 0..3 {
        let center = iv_at(m[r]);
        let mut yf = iv_at(0.0);
        for c in 0..3 {
            yf += iv_at(y[r][c]) * f[c];
        }
        let mut correction = iv_at(0.0);
        for c in 0..3 {
            let mut delta = iv_at(if r == c { 1.0 } else { 0.0 });
            for k in 0..3 {
                delta -= iv_at(y[r][k]) * j[k][c];
            }
            correction += delta * (q[c] - iv_at(m[c]));
        }
        out[r] = center - yf + correction;
    }
    out
}

/// The strict-inclusion carrier content of a certified slice query: the box
/// `X`, the Krawczyk image `K(X)` (component-wise strictly inside `X`), and
/// the Jacobian determinant enclosure (strictly excluding zero).
struct SliceCarrier {
    /// The certified box `X` (three free axes).
    box_x: [(f64, f64); 3],
    /// The Krawczyk image `K(X)`.
    k_x: [(f64, f64); 3],
    /// The Jacobian determinant enclosure over `X`.
    det: (f64, f64),
}

/// Compute the operator's own quantities over a slice query box.
fn slice_quantities(
    slice: &CSliceSystem,
    query: &[Interval; 3],
) -> Result<SliceCarrier, TangencyRefusal> {
    let mut m = [0.0f64; 3];
    for (mi, axis) in m.iter_mut().zip(query.iter()) {
        *mi = 0.5 * (axis.inf() + axis.sup());
    }
    let y = slice
        .preconditioner(&m)
        .ok_or(TangencyRefusal::Input(Refusal::ConditioningBelowThreshold))?;
    let f = slice.f_point(&m);
    let j = slice.jacobian(query);
    let k = k_image_3(query, &m, &y, &f, &j);
    let mut box_x = [(0.0f64, 0.0f64); 3];
    let mut k_x = [(0.0f64, 0.0f64); 3];
    for axis in 0..3 {
        box_x[axis] = (query[axis].inf(), query[axis].sup());
        let kv = k[axis];
        if kv.is_empty() || !kv.inf().is_finite() || !kv.sup().is_finite() {
            return Err(TangencyRefusal::Hull(HullRefusal::EnclosureUnavailable));
        }
        k_x[axis] = (kv.inf(), kv.sup());
    }
    let mut jm = [[CertifiedInterval::point(0.0); 3]; 3];
    for (r, row) in j.iter().enumerate() {
        for (c, cell) in row.iter().enumerate() {
            jm[r][c] = CertifiedInterval {
                lo: cell.inf(),
                hi: cell.sup(),
            };
        }
    }
    let det = det3_iv(&jm);
    if !det.is_finite() {
        return Err(TangencyRefusal::Hull(HullRefusal::EnclosureUnavailable));
    }
    Ok(SliceCarrier {
        box_x,
        k_x,
        det: (det.lo, det.hi),
    })
}

/// The frozen three-axis certificate of a certified slice carrier.
fn slice_certificate(carrier: &SliceCarrier) -> Result<KrawczykCertificate3, TangencyRefusal> {
    KrawczykCertificate3::new(carrier.box_x, carrier.k_x, carrier.det)
        .map_err(|_| TangencyRefusal::Input(Refusal::InvalidInput))
}

/// A plain-float Newton solve of the slice system from a start, for the seed
/// search's float root localization (never a certificate).
fn slice_newton(system: &CSliceSystem, start: [f64; 3]) -> Option<[f64; 3]> {
    let mut x = start;
    for _ in 0..24 {
        let x4 = system.embed(&x);
        let res = system.rows_at(x4)?;
        if res.iter().all(|v| v.abs() <= NEWTON_EPS) {
            return Some(x);
        }
        let free = system.free_axes();
        let rows = system.rows();
        let mut m = [[0.0f64; 3]; 3];
        for (r, row) in rows.iter().enumerate() {
            for (c, &axis) in free.iter().enumerate() {
                m[r][c] = row_float_partial(&system.system, row, axis, x4)?;
            }
        }
        let inv = invert3(&m)?;
        for r in 0..3 {
            let mut step = 0.0f64;
            for c in 0..3 {
                step += inv[r][c] * res[c];
            }
            x[r] -= step;
        }
        if x.iter().any(|v| !v.is_finite()) {
            return None;
        }
    }
    None
}

/// Try to certify a point of `{G = q = 0}` on the slice that fixes
/// `fixed_axis` at `fixed_value`, localizing the float root inside the box
/// and searching it with decreasing certified query radii.
///
/// Returns the certified carrier and the float free-axis root when the landed
/// operator certifies a unique slice root over the query box.
fn certify_slice_seed(
    system: &SquareSystem3,
    g_rows: [usize; 2],
    q: &ExactPoly,
    b: &Box4,
    fixed_axis: usize,
    fixed_value: f64,
    budget: &mut Budget,
) -> Option<(SliceCarrier, [f64; 3])> {
    let free: [usize; 3] = {
        let mut out = [0usize; 3];
        let mut k = 0usize;
        for j in 0..4 {
            if j != fixed_axis {
                out[k] = j;
                k += 1;
            }
        }
        out
    };
    let mut free_lo = [0.0f64; 3];
    let mut free_hi = [0.0f64; 3];
    for (slot, &axis) in free.iter().enumerate() {
        free_lo[slot] = b[axis].0;
        free_hi[slot] = b[axis].1;
    }
    let slice = CSliceSystem {
        system: system.clone(),
        g_rows,
        q: q.clone(),
        fixed_axis,
        fixed_value,
    };
    let mid = [
        0.5 * (free_lo[0] + free_hi[0]),
        0.5 * (free_lo[1] + free_hi[1]),
        0.5 * (free_lo[2] + free_hi[2]),
    ];
    let root = slice_newton(&slice, mid)?;
    if root.iter().any(|v| !v.is_finite()) {
        return None;
    }
    // The root must lie strictly inside the free box: a certified seed must be
    // a point of the branch inside the box (R6), not a root outside it.
    for k in 0..3 {
        if !(free_lo[k] < root[k] && root[k] < free_hi[k]) {
            return None;
        }
    }
    let mut fraction = SEED_RADIUS_FRACTION;
    for _ in 0..RADIUS_SHRINKS {
        let mut radii = [0.0f64; 3];
        let mut ok = true;
        for k in 0..3 {
            let width = free_hi[k] - free_lo[k];
            let gap = (root[k] - free_lo[k]).min(free_hi[k] - root[k]);
            if !width.is_finite() || width <= 0.0 || !gap.is_finite() || gap <= 0.0 {
                ok = false;
                break;
            }
            let want = width * fraction;
            radii[k] = want.min(gap);
            if !radii[k].is_finite() || radii[k] <= 0.0 {
                ok = false;
                break;
            }
        }
        if !ok {
            return None;
        }
        let query = box_around(root, radii)?;
        let carrier = match slice_quantities(&slice, &query) {
            Ok(carrier) => carrier,
            Err(_) => {
                fraction *= 0.1;
                continue;
            }
        };
        if !(carrier.det.0 > 0.0 || carrier.det.1 < 0.0) {
            fraction *= 0.1;
            continue;
        }
        let strict = carrier
            .box_x
            .iter()
            .zip(carrier.k_x.iter())
            .all(|(x, k)| x.0 < k.0 && k.1 < x.1);
        if !strict {
            fraction *= 0.1;
            continue;
        }
        match krawczyk::<3>(&slice, &query, budget) {
            Ok(Certified {
                value: KrawczykProof::Unique,
                ..
            }) => return Some((carrier, root)),
            Ok(Certified {
                value: KrawczykProof::NoRoot,
                ..
            }) => {
                fraction *= 0.1;
            }
            Err(_) => {
                fraction *= 0.1;
            }
        }
    }
    None
}

// ---------------------------------------------------------------------------
// The θρ continuation adapter on C = (G₁, G₂, q)
// ---------------------------------------------------------------------------

/// The augmented N = 4 system of the θρ corrector: the rows `(G₁, G₂, q)`
/// plus the hyperplane `τ·x = rhs` (the Ssi4System pattern, adapted to the
/// A₂ rows).
#[derive(Clone)]
struct CAugmentedSystem {
    /// The stored square system (for the two `G` component rows).
    system: SquareSystem3,
    /// The two stored `G` component indices.
    g_rows: [usize; 2],
    /// The exact `q` row.
    q: ExactPoly,
    /// The hyperplane normal (the unit tangent of the continuation).
    normal: [f64; 4],
    /// The hyperplane right-hand side.
    rhs: f64,
}

impl CAugmentedSystem {
    fn rows(&self) -> [Row; 3] {
        [
            Row::Grid(self.g_rows[0]),
            Row::Grid(self.g_rows[1]),
            Row::Exact(self.q.clone()),
        ]
    }
}

impl KrawczykSystem<4> for CAugmentedSystem {
    #[allow(clippy::needless_range_loop)] // fixed-size 4-axis index algebra (ssi4/tsystem precedent)
    fn f_point(&self, x: &[f64; 4]) -> [Interval; 4] {
        let degenerate: Box4 = [(x[0], x[0]), (x[1], x[1]), (x[2], x[2]), (x[3], x[3])];
        let rows = self.rows();
        let mut out = [Interval::EMPTY; 4];
        for (r, row) in rows.iter().enumerate() {
            match row_cert_value(&self.system, row, &degenerate) {
                Ok(enc) => out[r] = certified_to_interval(&enc),
                Err(_) => out[r] = Interval::EMPTY,
            }
        }
        let mut aug = 0.0f64;
        for j in 0..4 {
            aug += self.normal[j] * x[j];
        }
        aug -= self.rhs;
        out[3] = Interval::try_from((aug, aug)).unwrap_or(Interval::EMPTY);
        out
    }

    #[allow(clippy::needless_range_loop)] // fixed-size 4-axis index algebra (ssi4/tsystem precedent)
    fn jacobian(&self, b: &[Interval; 4]) -> [[Interval; 4]; 4] {
        let mut box4 = [(0.0f64, 0.0f64); 4];
        for axis in 0..4 {
            box4[axis] = (b[axis].inf(), b[axis].sup());
        }
        let rows = self.rows();
        let mut out = [[Interval::EMPTY; 4]; 4];
        for (r, row) in rows.iter().enumerate() {
            for axis in 0..4 {
                match row_cert_partial(&self.system, row, axis, &box4) {
                    Ok(enc) => out[r][axis] = certified_to_interval(&enc),
                    Err(_) => out[r][axis] = Interval::EMPTY,
                }
            }
        }
        for axis in 0..4 {
            out[3][axis] = Interval::try_from((self.normal[axis], self.normal[axis]))
                .unwrap_or(Interval::EMPTY);
        }
        out
    }

    #[allow(clippy::needless_range_loop)] // fixed-size 4-axis index algebra (ssi4/tsystem precedent)
    fn preconditioner(&self, x: &[f64; 4]) -> Option<[[f64; 4]; 4]> {
        let rows = self.rows();
        let mut m = [[0.0f64; 4]; 4];
        for (r, row) in rows.iter().enumerate() {
            for axis in 0..4 {
                m[r][axis] = row_float_partial(&self.system, row, axis, *x)?;
            }
        }
        for axis in 0..4 {
            m[3][axis] = self.normal[axis];
        }
        invert4(&m)
    }
}

/// The 3 x 4 float Jacobian of the rows `(G₁, G₂, q)` at a chart point.
#[allow(clippy::needless_range_loop)] // fixed-size 3x4 index algebra (ssi4/tsystem precedent)
fn row_jacobian_float(
    system: &SquareSystem3,
    g_rows: [usize; 2],
    q: &ExactPoly,
    x: [f64; 4],
) -> Option<[[f64; 4]; 3]> {
    let rows: [Row; 3] = [
        Row::Grid(g_rows[0]),
        Row::Grid(g_rows[1]),
        Row::Exact(q.clone()),
    ];
    let mut out = [[0.0f64; 4]; 3];
    for (r, row) in rows.iter().enumerate() {
        for axis in 0..4 {
            out[r][axis] = row_float_partial(system, row, axis, x)?;
        }
    }
    Some(out)
}

/// Embed a 3-D free-coordinate point at a fixed axis/value into the 4-D chart.
#[allow(clippy::needless_range_loop)] // fixed-size 4-axis embedding; the index form is the algebra (ssi4/tsystem precedent)
fn embed_point(fixed_axis: usize, fixed_value: f64, x: &[f64; 3]) -> [f64; 4] {
    let mut out = [fixed_value; 4];
    let mut k = 0usize;
    for j in 0..4 {
        if j == fixed_axis {
            continue;
        }
        out[j] = x[k];
        k += 1;
    }
    out
}

/// Whether a 4-D point lies on the box's boundary (any coordinate at a face).
fn point_on_boundary(b: &Box4, p: &[f64; 4]) -> bool {
    for axis in 0..4 {
        if p[axis] <= b[axis].0 || p[axis] >= b[axis].1 {
            return true;
        }
    }
    false
}

/// Whether a point lies (strictly, up to an ulp-scale slack) inside the box.
fn inside_box(x: &[f64; 4], box4: &Box4) -> bool {
    for j in 0..4 {
        let slack = (box4[j].1 - box4[j].0).abs() * 1.0e-9;
        if x[j] < box4[j].0 - slack || x[j] > box4[j].1 + slack {
            return false;
        }
    }
    true
}

/// Negate a 4-vector.
fn negate4(v: &[f64; 4]) -> [f64; 4] {
    [-v[0], -v[1], -v[2], -v[3]]
}

/// The dominant travel axes of the branch at a seed: the two axes with the
/// largest |tangent| component, ties toward the lower index. These define the
/// produced curve's two-axis carrier chart.
fn carrier_axes(tangent: &[f64; 4]) -> (usize, usize) {
    let mut order = [0usize, 1, 2, 3];
    order.sort_by(|&a, &b| {
        let ta = tangent[a].abs();
        let tb = tangent[b].abs();
        tb.partial_cmp(&ta)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.cmp(&b))
    });
    (order[0], order[1])
}

/// Trace one half-branch from a certified seed in the signed tangent
/// direction, collecting certified samples while the prediction stays inside
/// the box.
#[allow(clippy::needless_range_loop)] // fixed-size 4-axis march algebra (ssi4/tsystem precedent)
fn trace_half_branch(
    system: &SquareSystem3,
    g_rows: [usize; 2],
    q: &ExactPoly,
    b: &Box4,
    seed: [f64; 4],
    requested_tangent: [f64; 4],
) -> Vec<A2CertifiedSample> {
    let mut out: Vec<A2CertifiedSample> = Vec::new();
    let mut centre = seed;
    let mut tangent = requested_tangent;
    let mut step = THETA_STEP;
    for _ in 0..MAX_TRACE_STEPS {
        let jac = match row_jacobian_float(system, g_rows, q, centre) {
            Some(jac) => jac,
            None => break,
        };
        let Some(frame) = ParallelotopeFrame::from_jacobian(centre, &jac) else {
            break;
        };
        // Orient the frame along the requested marching direction.
        let mut dot = 0.0f64;
        for j in 0..4 {
            dot += frame.tangent[j] * tangent[j];
        }
        let frame = if dot < 0.0 {
            ParallelotopeFrame {
                center: centre,
                tangent: negate4(&frame.tangent),
                transversal: frame.transversal,
            }
        } else {
            frame
        };
        let predicted = frame.predict(step);
        if !inside_box(&predicted, b) {
            break;
        }
        let mut radii = [0.0f64; 4];
        let mut ok = true;
        for j in 0..4 {
            let width = b[j].1 - b[j].0;
            if !width.is_finite() || width <= 0.0 {
                ok = false;
                break;
            }
            radii[j] = width * STEP_RADIUS_FRACTION;
        }
        if !ok {
            break;
        }
        let mut rhs = 0.0f64;
        for j in 0..4 {
            rhs += frame.tangent[j] * predicted[j];
        }
        let system4 = CAugmentedSystem {
            system: system.clone(),
            g_rows,
            q: q.clone(),
            normal: frame.tangent,
            rhs,
        };
        let mut attempt_budget = Budget::new(ATTEMPT_BUDGET, 0, 0);
        match theta_rho_step(&system4, predicted, radii, &mut attempt_budget) {
            StepVerdict::Certified { cell, center, cert } => {
                let mut cell_box = [(0.0f64, 0.0f64); 4];
                for j in 0..4 {
                    cell_box[j] = (cell[j].inf(), cell[j].sup());
                }
                let jac_new = match row_jacobian_float(system, g_rows, q, center) {
                    Some(jac) => jac,
                    None => break,
                };
                let Some(new_frame) = ParallelotopeFrame::from_jacobian(center, &jac_new) else {
                    break;
                };
                let mut tangent_new = new_frame.tangent;
                let mut d = 0.0f64;
                for j in 0..4 {
                    d += tangent_new[j] * frame.tangent[j];
                }
                if d < 0.0 {
                    tangent_new = negate4(&tangent_new);
                }
                let sample = match A2BranchSample::new(center, tangent_new) {
                    Ok(sample) => sample,
                    Err(_) => break,
                };
                out.push(A2CertifiedSample {
                    sample,
                    cell: cell_box,
                    cert,
                });
                centre = center;
                tangent = tangent_new;
                step = THETA_STEP;
            }
            StepVerdict::NoRoot => {
                step *= 0.5;
                if step < 1.0e-12 {
                    break;
                }
            }
            StepVerdict::Refused(_) => break,
        }
    }
    out
}

// ---------------------------------------------------------------------------
// The A₂ certify body
// ---------------------------------------------------------------------------

/// The A₂ attempt over one box: verify the exact identity, certify the
/// multipliers and the rank-3 separation, certify a non-empty piece, and fill
/// the branch curve by rank-3 parallelotope continuation.
///
/// Returns the frozen [`ContactVerdict`]: `A2Branch` when a non-empty piece is
/// certified, `Empty` when the identity holds but no piece exists on the box.
///
/// `data` supplies the identity's factor polynomials in the module's readable
/// exact carrier; the authoritative [`QPoly`] witness must carry exactly those
/// polynomials (checked by exact ring equality) and the recorded `G₁, G₂` must
/// equal two stored components of `system`.
pub fn certify_a2(
    system: &SquareSystem3,
    minors: &ChartMinorGrids,
    witness: &ExactVanishingWitness<QPoly>,
    data: &A2ExactData,
    b: &Box4,
    budget: &mut Budget,
) -> Result<ContactVerdict<QPoly>, TangencyRefusal> {
    certify_a2_attested(system, minors, witness, data, b, budget, None).map(|a| a.verdict)
}

/// Certify an A₂ attempt, returning the full attestation (verdict, produced
/// branch curve, per-sample certificates, and the routing diagnostic).
pub fn certify_a2_attested(
    system: &SquareSystem3,
    _minors: &ChartMinorGrids,
    witness: &ExactVanishingWitness<QPoly>,
    data: &A2ExactData,
    b: &Box4,
    budget: &mut Budget,
    diagnostic: Option<NonContractionDiagnostic>,
) -> Result<A2Attestation, TangencyRefusal> {
    if witness.side() != WitnessSide::A2 {
        return Err(TangencyRefusal::Input(Refusal::InvalidInput));
    }
    for (lo, hi) in b {
        if !lo.is_finite() || !hi.is_finite() || lo > hi {
            return Err(TangencyRefusal::Input(Refusal::InvalidInput));
        }
    }
    let maps = system.domain_maps();
    let identity = (0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0);
    if maps != identity {
        return Err(TangencyRefusal::Input(Refusal::InvalidInput));
    }

    // The readable exact data must equal the authoritative witness's recorded
    // polynomials exactly (the witness is what the one verifier checks).
    let (q_recorded, a_recorded) = match witness.q2a() {
        Some(q2a) => q2a,
        None => return Err(TangencyRefusal::Input(Refusal::InvalidInput)),
    };
    if data.s.to_qpoly()? != *witness.s()
        || data.q.to_qpoly()? != *q_recorded
        || data.a.to_qpoly()? != *a_recorded
    {
        return Err(TangencyRefusal::Input(Refusal::InvalidInput));
    }

    // Resolve which stored component is f, and recover the stored G rows.
    let (f_comp, g_rows) = resolve_split(system, witness)?;
    let f = poly_from_grid(system, f_comp)?.to_qpoly()?;

    // The multipliers' certified nonvanishing enclosures over the box.
    let s_encl = data.s.interval(b)?;
    let a_encl = data.a.interval(b)?;
    if !strictly_excludes_zero(&s_encl) || !strictly_excludes_zero(&a_encl) {
        return Err(TangencyRefusal::Input(Refusal::InvalidInput));
    }

    // The ONE verifier: the exact identity over ℚ plus the interval
    // nonvanishing predicates (CTE-001).
    let terms = witness.terms();
    let g1 = &terms[0].1;
    let g2 = &terms[1].1;
    ExactWitnessVerifier::verify(
        witness,
        &f,
        &[g1, g2],
        Some((s_encl.lo, s_encl.hi)),
        Some((a_encl.lo, a_encl.hi)),
    )
    .map_err(|_| TangencyRefusal::Input(Refusal::InvalidInput))?;

    // Rank-3 separation of one minor of D(G₁, G₂, q).
    let q = &data.q;
    let rank3 = certify_rank3(system, g_rows, q, b)?;

    // R6: certify a non-empty piece on {G = q = 0}; without it the identity
    // is satisfied vacuously and the verdict is Empty.
    let Some(nonempty) = seed_on_box(system, g_rows, q, b, budget)? else {
        return Ok(A2Attestation {
            verdict: ContactVerdict::Empty,
            curve: None,
            samples: Vec::new(),
            diagnostic,
        });
    };
    let (evidence, seed_point) = nonempty;

    // Fill the branch curve by rank-3 parallelotope continuation.
    let (curve, samples) = trace_branch(system, g_rows, q, b, seed_point)?;

    let cert = A2Cert::new(witness.clone(), s_encl, a_encl, rank3, evidence)
        .map_err(|_| TangencyRefusal::Input(Refusal::InvalidInput))?;
    Ok(A2Attestation {
        verdict: ContactVerdict::A2Branch(cert),
        curve: Some(curve),
        samples,
        diagnostic,
    })
}

/// Certify a non-empty piece of `{G = q = 0}` inside the box, scanning the
/// deterministic (axis, value) grid over the box's centre and faces.
///
/// Returns the non-emptiness evidence and the certified 4-D branch point.
fn seed_on_box(
    system: &SquareSystem3,
    g_rows: [usize; 2],
    q: &ExactPoly,
    b: &Box4,
    budget: &mut Budget,
) -> Result<Option<(A2NonemptyEvidence, [f64; 4])>, TangencyRefusal> {
    // Fixed axis order; within an axis the value order is mid, lo, hi
    // (interior-first, deterministic).
    for axis in 0..4 {
        let lo = b[axis].0;
        let hi = b[axis].1;
        let mid = 0.5 * (lo + hi);
        let values = [mid, lo, hi];
        for value in values {
            if let Some((carrier, root)) =
                certify_slice_seed(system, g_rows, q, b, axis, value, budget)
            {
                let kcert = slice_certificate(&carrier)?;
                let point = embed_point(axis, value, &root);
                let evidence = if point_on_boundary(b, &point) {
                    A2NonemptyEvidence::BoundaryCrossing(kcert)
                } else {
                    A2NonemptyEvidence::BranchSeed(kcert)
                };
                return Ok(Some((evidence, point)));
            }
        }
    }
    Ok(None)
}

/// Trace the branch from a certified seed in both directions, returning the
/// ordered certified curve and its per-sample certificates.
///
/// Ordering: the negative-tangent half-branch reversed, then the seed, then
/// the positive-tangent half-branch — the samples advance along the branch in
/// a fixed, deterministic order.
fn trace_branch(
    system: &SquareSystem3,
    g_rows: [usize; 2],
    q: &ExactPoly,
    b: &Box4,
    seed_point: [f64; 4],
) -> Result<(A2BranchCurve, Vec<A2CertifiedSample>), TangencyRefusal> {
    // The seed's certified tangent direction (the unit kernel direction of
    // D(G₁, G₂, q) at the seed, by the landed cofactor/minor formula).
    let jac = row_jacobian_float(system, g_rows, q, seed_point)
        .ok_or(TangencyRefusal::Input(Refusal::InvalidInput))?;
    let seed_frame = ParallelotopeFrame::from_jacobian(seed_point, &jac)
        .ok_or(TangencyRefusal::Input(Refusal::InvalidInput))?;
    // Canonical orientation: the leading travel axis (the tangent's largest
    // |component|, ties toward the lower index) points positive. The produced
    // curve order then advances monotonically along that axis.
    let (leading, _) = carrier_axes(&seed_frame.tangent);
    let seed_tangent = if seed_frame.tangent[leading] < 0.0 {
        negate4(&seed_frame.tangent)
    } else {
        seed_frame.tangent
    };

    let forward = trace_half_branch(system, g_rows, q, b, seed_point, seed_tangent);
    let reverse_raw = trace_half_branch(system, g_rows, q, b, seed_point, negate4(&seed_tangent));

    let seed_sample = A2CertifiedSample {
        sample: A2BranchSample::new(seed_point, seed_tangent)
            .map_err(|_| TangencyRefusal::Input(Refusal::InvalidInput))?,
        cell: point_cell(b, seed_point),
        cert: interval_certificate(),
    };

    let mut samples: Vec<A2CertifiedSample> = Vec::new();
    for s in reverse_raw.into_iter().rev() {
        samples.push(s);
    }
    samples.push(seed_sample);
    for s in forward {
        samples.push(s);
    }

    if samples.is_empty() {
        return Err(TangencyRefusal::Input(Refusal::InvalidInput));
    }

    let (c0, c1) = carrier_axes(&seed_tangent);
    let carrier_chart = [b[c0], b[c1]];
    let curve_samples = samples
        .iter()
        .map(|s| s.sample)
        .collect::<Vec<A2BranchSample>>();
    let curve = A2BranchCurve::new(carrier_chart, curve_samples)
        .map_err(|_| TangencyRefusal::Input(Refusal::InvalidInput))?;
    Ok((curve, samples))
}

/// A small certified cell about the seed point used for the seed sample's
/// curve record (the seed's own certified statement is its slice certificate,
/// carried in the [`A2Cert`]'s non-emptiness evidence).
fn point_cell(b: &Box4, p: [f64; 4]) -> Box4 {
    let mut out = [(0.0f64, 0.0f64); 4];
    for axis in 0..4 {
        let width = b[axis].1 - b[axis].0;
        let half = (width * 1.0e-3).max(f64::EPSILON);
        let lo = (p[axis] - half).max(b[axis].0);
        let hi = (p[axis] + half).min(b[axis].1);
        out[axis] = (lo, hi);
    }
    out
}

/// The interval-method certificate of a certified box (H-6: never float).
fn interval_certificate() -> Certificate {
    Certificate {
        props: truck_base::evidence::PropMap::new(),
        method: truck_base::evidence::Method::Interval,
        budget_left: Budget::new(0, 0, 0),
        margin: truck_base::evidence::Margin::UNBOUNDED,
        modulus: truck_base::evidence::Modulus::Unbounded,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tangency::fixtures::F5ParabolaFixture;

    /// The F5 (A₂) square system over the unit chart: the plane `z = 0`
    /// (graph over `(u, v)`) and the downward-centered parabola graph
    /// `z = −(s − 1/2)²` over `(s, t)`, so the Morse–Bott contact line
    /// `(u, v, s, t) = (1/2, w, 1/2, w)` lies STRICTLY inside the chart — the
    /// F5 A₂ class (plane × extruded parabola, contact along a line) with an
    /// interior branch, so a `BranchSeed` (the R6 gate) is certifiable.
    ///
    /// The cross difference is `F = (u − s, v − t, (s − 1/2)²)`.
    fn f5_system() -> SquareSystem3 {
        let degrees = (2usize, 2usize, 2usize, 1usize);
        let f0 = monomial_grid4(degrees, &[([1, 0, 0, 0], 1.0), ([0, 0, 1, 0], -1.0)]);
        let f1 = monomial_grid4(degrees, &[([0, 1, 0, 0], 1.0), ([0, 0, 0, 1], -1.0)]);
        let f2 = monomial_grid4(
            degrees,
            &[
                ([0, 0, 2, 0], 1.0),
                ([0, 0, 1, 0], -1.0),
                ([0, 0, 0, 0], 0.25),
            ],
        );
        SquareSystem3::new(
            [f0, f1, f2],
            degrees,
            (0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0),
        )
        .expect("the F5 square system admits")
    }

    /// A four-axis monomial Bernstein grid over the identity chart (test
    /// support; the small integer/dyadic degrees stay exact in `f64`).
    fn monomial_grid4(
        degrees: (usize, usize, usize, usize),
        terms: &[([usize; 4], f64)],
    ) -> Vec<Vec<f64>> {
        let (m1, n1, m2, n2) = degrees;
        let rows = (m1 + 1) * (n1 + 1);
        let cols = (m2 + 1) * (n2 + 1);
        let binom = |n: usize, k: usize| -> f64 {
            let mut out = 1.0f64;
            for t in 0..k {
                out = out * ((n - t) as f64) / ((t + 1) as f64);
            }
            out
        };
        let mut grid = vec![vec![0.0f64; cols]; rows];
        for &(exp, coeff) in terms {
            for a in exp[0]..=m1 {
                for b in exp[1]..=n1 {
                    for i in exp[2]..=m2 {
                        for j in exp[3]..=n2 {
                            let factor = binom(a, exp[0]) / binom(m1, exp[0]) * binom(b, exp[1])
                                / binom(n1, exp[1])
                                * binom(i, exp[2])
                                / binom(m2, exp[2])
                                * binom(j, exp[3])
                                / binom(n2, exp[3]);
                            grid[a * (n1 + 1) + b][i * (n2 + 1) + j] += coeff * factor;
                        }
                    }
                }
            }
        }
        grid
    }

    /// The one-term exact polynomial `coeff·x^exp` over the four variables.
    fn exact_term(exp: [u32; 4], coeff: QCoeff) -> ExactPoly {
        ExactPoly::from_terms(vec![(exp.to_vec(), coeff)]).expect("a valid four-variable term")
    }

    /// The F5 exact data: `s = 1`, `a = 1`, `q = s − 1/2` (the readable
    /// carrier of the identity's factors).
    fn f5_data() -> A2ExactData {
        let one = QCoeff::from_int(1);
        let half = QCoeff::new(-1, 2).expect("a half is a valid rational");
        let s = exact_term([0, 0, 0, 0], one);
        let a = exact_term([0, 0, 0, 0], one);
        let q = ExactPoly::from_terms(vec![
            ([0, 0, 1, 0].to_vec(), one),
            ([0, 0, 0, 0].to_vec(), half),
        ])
        .expect("q = s - 1/2 is a valid polynomial");
        A2ExactData::new(s, q, a)
    }

    /// The F5 witness: `s = 1`, `a = 1`, `q = s − 1/2`, `u₁ = u₂ = 0`, so
    /// `1·f = q²·1 + 0·G₁ + 0·G₂` holds exactly with `G₁ = u − s`,
    /// `G₂ = v − t`, `f = (s − 1/2)²`.
    fn f5_witness() -> ExactVanishingWitness<QPoly> {
        let data = f5_data();
        let one = QCoeff::from_int(1);
        let g1 = ExactPoly::from_terms(vec![
            ([1, 0, 0, 0].to_vec(), one),
            ([0, 0, 1, 0].to_vec(), QCoeff::from_int(-1)),
        ])
        .expect("G1 = u - s");
        let g2 = ExactPoly::from_terms(vec![
            ([0, 1, 0, 0].to_vec(), one),
            ([0, 0, 0, 1].to_vec(), QCoeff::from_int(-1)),
        ])
        .expect("G2 = v - t");
        let s = data.s.to_qpoly().expect("s converts");
        let a = data.a.to_qpoly().expect("a converts");
        let q = data.q.to_qpoly().expect("q converts");
        let terms = vec![
            (QPoly::zero(4), g1.to_qpoly().expect("G1 converts")),
            (QPoly::zero(4), g2.to_qpoly().expect("G2 converts")),
        ];
        ExactVanishingWitness::new(WitnessSide::A2, s, terms, Some((q, a)))
            .expect("the F5 A2 witness has the A2 term shape")
    }

    /// The F5 box containing the branch strictly in its interior.
    fn f5_box() -> Box4 {
        [(0.2, 0.8), (0.2, 0.8), (0.2, 0.8), (0.2, 0.8)]
    }

    /// The box disjoint from the F5 branch (its `u` and `s` windows exclude
    /// `1/2`), over which the identity still holds but no piece exists.
    fn f5_disjoint_box() -> Box4 {
        [(0.1, 0.4), (0.2, 0.8), (0.1, 0.4), (0.2, 0.8)]
    }

    /// A placeholder minors carrier (the A₂ certificate does not consume the
    /// minor grids; the frozen signature carries them for the cascade).
    fn minor_placeholder() -> ChartMinorGrids {
        let net = crate::tangency::shapes::ChartMinorNet::new([1, 1, 1, 1], vec![1.0; 16])
            .expect("a valid placeholder net");
        ChartMinorGrids::new(net.clone(), net).expect("a valid placeholder grid")
    }

    #[test]
    fn f5_witness_verifies_and_rank3_separates() {
        F5ParabolaFixture::new()
            .admit()
            .expect("the F5 ground truth admits");
        let system = f5_system();
        let witness = f5_witness();
        let data = f5_data();
        let b = f5_box();
        let mut budget = Budget::new(ATTEMPT_BUDGET, 0, 0);
        let verdict = certify_a2(
            &system,
            &minor_placeholder(),
            &witness,
            &data,
            &b,
            &mut budget,
        )
        .expect("the F5 A2 certificate verifies and certifies");
        match verdict {
            ContactVerdict::A2Branch(cert) => {
                assert_eq!(cert.identity().side(), WitnessSide::A2);
                assert!(
                    cert.s_encl().lo > 0.0 || cert.s_encl().hi < 0.0,
                    "ŝ(B) excludes zero"
                );
                assert!(
                    cert.a_encl().lo > 0.0 || cert.a_encl().hi < 0.0,
                    "â(B) excludes zero"
                );
                // One 3 x 3 minor of D(G, q) is strictly separated on the box.
                let rank3 = cert.rank3();
                let (a, b2, c) = rank3.coordinate_triple();
                assert!(a < b2 && b2 < c && c <= 3);
                assert!(
                    rank3.minor_enclosure().lo > 0.0 || rank3.minor_enclosure().hi < 0.0,
                    "the rank-3 minor is separated from zero"
                );
            }
            other => panic!("the F5 box must certify an A2 branch, got {other:?}"),
        }
    }

    #[test]
    fn nonemptiness_seed_certifies_on_f5() {
        F5ParabolaFixture::new()
            .admit()
            .expect("the F5 ground truth admits");
        let system = f5_system();
        let witness = f5_witness();
        let data = f5_data();
        let b = f5_box();
        let mut budget = Budget::new(ATTEMPT_BUDGET, 0, 0);
        let verdict = certify_a2(
            &system,
            &minor_placeholder(),
            &witness,
            &data,
            &b,
            &mut budget,
        )
        .expect("the F5 certificate certifies");
        match verdict {
            ContactVerdict::A2Branch(cert) => match cert.nonempty() {
                A2NonemptyEvidence::BranchSeed(seed) => {
                    // The R6 gate: the seed is a Krawczyk certificate with a
                    // strict inclusion and a determinant excluding zero.
                    let box_x = seed.box_x();
                    let k_x = seed.k_x();
                    for (bx, kx) in box_x.iter().zip(k_x.iter()) {
                        assert!(bx.0 < kx.0 && kx.1 < bx.1, "K(X) ⊂ int(X) strictly");
                    }
                    let det = seed.det();
                    assert!(
                        det.0 > 0.0 || det.1 < 0.0,
                        "the slice determinant excludes zero"
                    );
                }
                other => panic!("the interior F5 branch must certify a BranchSeed, got {other:?}"),
            },
            other => panic!("the F5 box must certify an A2 branch, got {other:?}"),
        }
    }

    #[test]
    #[allow(clippy::needless_range_loop)] // fixed-size 4-axis sample-record assertions (index form reads the axis)
    fn a2_branch_curve_carries_certified_samples() {
        F5ParabolaFixture::new()
            .admit()
            .expect("the F5 ground truth admits");
        let system = f5_system();
        let witness = f5_witness();
        let data = f5_data();
        let b = f5_box();
        let mut budget = Budget::new(ATTEMPT_BUDGET, 0, 0);
        let attestation = certify_a2_attested(
            &system,
            &minor_placeholder(),
            &witness,
            &data,
            &b,
            &mut budget,
            None,
        )
        .expect("the F5 attestation certifies");
        let curve = attestation
            .curve
            .expect("the A2 branch carries a produced curve");
        let samples = curve.samples();
        assert!(!samples.is_empty(), "the branch curve must be nonempty");
        assert!(
            samples.len() >= 3,
            "the branch curve must carry certified steps in both directions from the seed, got {}",
            samples.len()
        );
        assert_eq!(samples.len(), attestation.samples.len());
        // Ordered along the branch; every sample carries its certified box.
        let mut previous_axis1 = f64::NEG_INFINITY;
        for (i, record) in attestation.samples.iter().enumerate() {
            assert_eq!(&record.sample, &samples[i]);
            for axis in 0..4 {
                assert!(
                    record.cell[axis].0 <= record.sample.point()[axis]
                        && record.sample.point()[axis] <= record.cell[axis].1,
                    "sample {} point inside its certified cell on axis {}",
                    i,
                    axis
                );
                assert!(
                    record.cell[axis].0 >= b[axis].0 && record.cell[axis].1 <= b[axis].1,
                    "sample {} certified cell inside the box on axis {}",
                    i,
                    axis
                );
            }
            // The branch advances monotonically along axis 1 (the shared
            // v = t direction): certified order is deterministic.
            assert!(
                record.sample.point()[1] >= previous_axis1,
                "samples must be ordered along the branch"
            );
            previous_axis1 = record.sample.point()[1];
        }
    }

    #[test]
    fn vacuous_a2_refuses() {
        F5ParabolaFixture::new()
            .admit()
            .expect("the F5 ground truth admits");
        let system = f5_system();
        let witness = f5_witness();
        let data = f5_data();
        let b = f5_disjoint_box();
        let mut budget = Budget::new(ATTEMPT_BUDGET, 0, 0);
        let verdict = certify_a2(
            &system,
            &minor_placeholder(),
            &witness,
            &data,
            &b,
            &mut budget,
        )
        .expect("the identity verifies on the disjoint box");
        assert!(
            matches!(verdict, ContactVerdict::Empty),
            "vacuous satisfaction must emit Empty, never A2Branch, got {verdict:?}"
        );
    }

    #[test]
    fn noncontraction_diagnostic_routes_to_a2() {
        F5ParabolaFixture::new()
            .admit()
            .expect("the F5 ground truth admits");
        let system = f5_system();
        let witness = f5_witness();
        let data = f5_data();
        let b = f5_box();
        let diagnostic =
            NonContractionDiagnostic::new(b, 3).expect("the routing diagnostic admits");
        let mut budget = Budget::new(ATTEMPT_BUDGET, 0, 0);
        let attestation = certify_a2_attested(
            &system,
            &minor_placeholder(),
            &witness,
            &data,
            &b,
            &mut budget,
            Some(diagnostic.clone()),
        )
        .expect("the routed attempt certifies");
        match &attestation.verdict {
            ContactVerdict::A2Branch(_) => {}
            other => panic!("the routed F5 attempt must certify A2Branch, got {other:?}"),
        }
        let routed = attestation
            .diagnostic
            .expect("the routing evidence is recorded on the attestation");
        assert_eq!(routed.cell(), diagnostic.cell());
        assert_eq!(routed.spend(), diagnostic.spend());
    }

    /// FSSI-000 decision 4: the module's downstream `SsiRefusal` match
    /// (`map_ssi`) compiles exhaustively over every variant — including the
    /// two FSSI-001 suspicion halts — and folds each onto exactly one named
    /// [`TangencyRefusal`]. The two new cases use the pre-decided
    /// low-information `Input(Refusal::InvalidInput)` pattern (FSSI-EXT: no
    /// new `TangencyRefusal` variant); the SSI-layer `tag()` carries the
    /// specificity. This test is the compile probe: a new `SsiRefusal`
    /// variant without an arm breaks the exhaustive match below.
    #[test]
    fn downstream_matches_exhaustive_compile() {
        let cases: [(SsiRefusal, &'static str); 8] = [
            (
                SsiRefusal::Hull(HullRefusal::EnclosureUnavailable),
                "tangency_hull_enclosure_unavailable",
            ),
            (
                SsiRefusal::PairClass(
                    crate::formal::intersection::PairUnsupported::UnsupportedPairClass,
                ),
                "tangency_invalid_input",
            ),
            (
                SsiRefusal::Conditioning(Refusal::ConditioningBelowThreshold),
                "tangency_invalid_input",
            ),
            (SsiRefusal::DeterminantSpansZero, "tangency_invalid_input"),
            (SsiRefusal::InclusionNotStrict, "tangency_invalid_input"),
            (SsiRefusal::InvalidInput, "tangency_invalid_input"),
            (
                SsiRefusal::TangentCurveSuspected {
                    margin: (0.25, 4.0),
                },
                "tangency_invalid_input",
            ),
            (
                SsiRefusal::CoincidentPatchSuspected {
                    margin: (0.25, 4.0),
                },
                "tangency_invalid_input",
            ),
        ];
        for (refusal, expected) in cases {
            assert_eq!(
                map_ssi(refusal).tag(),
                expected,
                "map_ssi must fold {refusal:?} onto the named {expected} refusal"
            );
        }
    }
}
