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

//! The five-way contact-classifier cascade (CTE-004-HESSIAN; theory §2.11).
//!
//! [`classify_box`] runs the theory's stage table (stages 0–7) over one box of
//! a chart-certified square system and returns the frozen five-way
//! [`ContactVerdict`](crate::tangency::shapes::ContactVerdict):
//!
//! | Stage | Certificate | Verdict |
//! |---|---|---|
//! | 0 | `0 ∉ F̂ᵢ(B)` for some `i` | `Empty` |
//! | 2 | `(F, M₁, M₂)` has no root in `B` (T1.5.1) | `Transversal` |
//! | 3 | unique root `x*` of `T`; `0 ∉ f̂(X*)` | `Transversal` (T1.5.2), refined to `Empty` / arc under T1.6a |
//! | 4 | unique `x*` + exact-zero witness + definite `Ĥ_h` | `A1Isolated` (T1.6b) |
//! | 5 | unique `x*` + exact-zero witness + `det Ĥ_h < 0` | `A1Node` (T1.6c) |
//! | 7 | otherwise | `Unresolved { kappa, cell }` |
//!
//! (Stage 1 — chart search and (H-graph) certification — is the caller's work:
//! `classify_box` receives the certified [`CertifiedGraph`]. Stage 6 — the A₂
//! factor identity — is CTE-005's witness-parameterized route and is not
//! reachable from this signature, which carries no identity data; a box that
//! needs it records `Unresolved` with the deepest stage reached.)
//!
//! # The A₁ exact-zero witness (scope decision 4)
//!
//! Interval methods CANNOT certify the exact vanishing of the critical value
//! (theory §4.1, D7), and an [`A1Cert`] without an exact-zero witness is
//! unconstructible. The cascade therefore obtains the A₁ witness as an exact
//! ideal-membership certificate: the stored component and chart-minor grids
//! are lifted exactly over ℚ (dyadic Bernstein coefficients → monomial
//! basis), and `f` is reduced modulo the deflated ideal
//! `⟨G₁, G₂, M₁, M₂⟩` by deterministic multivariate division. When the
//! remainder is exactly zero the recorded quotients ARE the multipliers of a
//! certified A₁ witness (`s = 1`, terms `(wᵢ, Pᵢ)`), which is then handed to
//! the ONE verifier (CTE-001) — producers never self-certify. When no exact
//! membership can be exhibited the A₁ arms are NOT reachable: the box records
//! `Unresolved` (never an `A1Isolated`/`A1Node` without a verified witness).
//!
//! # Determinism
//!
//! Fixed stage order, fixed exclusion subsystem order (ascending skip), fixed
//! reduction order (lexicographic monomial order, generator order
//! `(G₁, G₂, M₁, M₂)`), fixed Gershgorin-then-closed-form modulus rule, and no
//! hash iteration anywhere. Identical input yields an identical verdict
//! field-for-field (the `cascade_verdicts_are_deterministic` gate).
//!
//! **House rules.** No `unwrap`, `expect`, `panic!` or out-of-range indexing
//! reachable from geometry; every fallible operation returns
//! [`TangencyRefusal`]; float-computed values are never recorded as exact.

use crate::contract::Refusal;
use crate::formal::exact::CertifiedInterval;
use crate::hull::HullRefusal;
use crate::tangency::exclude::{exclude_five, ExclusionEvidence};
use crate::tangency::graph::CertifiedGraph;
use crate::tangency::hessian::{definiteness, reduced_hessian};
use crate::tangency::minors::from_system_grid;
use crate::tangency::qpoly::{QCoeff, QPoly};
use crate::tangency::shapes::{
    A1Cert, ChartMinorGrids, ContactVerdict, Definiteness, DefinitenessSign, DeflatedRootCert,
    ExactVanishingWitness, ExactWitnessVerifier, GraphEnclosure, Rank2Chart, TransversalCert,
    WitnessSide,
};
use crate::tangency::tsystem::TSystem;
use crate::tangency::TangencyRefusal;
use crate::SquareSystem3;
use truck_base::evidence::Budget;
use truck_evidence::enclosure::Interval;
use truck_evidence::num::krawczyk::{krawczyk, KrawczykProof, KrawczykSystem};

/// A four-axis box in the unit chart.
type Box4 = [(f64, f64); 4];

/// Whether a box is well formed for the classifier (finite, ordered).
fn box_ok(b: &Box4) -> bool {
    b.iter()
        .all(|(lo, hi)| lo.is_finite() && hi.is_finite() && lo <= hi)
}

/// The two stored components of a pivot other than `f`, ascending.
fn g_rows_of(component: usize) -> [usize; 2] {
    let mut gs = [0usize; 2];
    let mut slot = 0usize;
    for c in 0..3 {
        if c != component {
            gs[slot] = c;
            slot += 1;
        }
    }
    gs
}

/// The certified range enclosure of one stored component over a unit-chart box.
fn component_enclosure(
    system: &SquareSystem3,
    component: usize,
    b: &Box4,
) -> Result<CertifiedInterval, TangencyRefusal> {
    let grid = from_system_grid(&system.grids()[component], system.degrees())?;
    grid.hull_unit(*b)
}

/// Whether a certified interval strictly excludes zero.
fn excludes_zero(enc: &CertifiedInterval) -> bool {
    enc.is_finite() && !(enc.lo <= 0.0 && 0.0 <= enc.hi)
}

/// A unit-chart box as the Krawczyk operator's interval box. `None` on a
/// non-finite, misordered, or non-compact axis.
fn to_interval_box(b: &Box4) -> Option<[Interval; 4]> {
    let mut out = [Interval::EMPTY; 4];
    for (axis, cell) in out.iter_mut().enumerate() {
        let (lo, hi) = b.get(axis)?;
        if !lo.is_finite() || !hi.is_finite() || lo > hi {
            return None;
        }
        *cell = Interval::try_from((*lo, *hi)).ok()?;
    }
    Some(out)
}

/// A box as the float midpoint used for the root certificate's centre.
fn midpoint(b: &Box4) -> Option<[f64; 4]> {
    let mut m = [0.0f64; 4];
    for (axis, cell) in m.iter_mut().enumerate() {
        let (lo, hi) = b.get(axis)?;
        if !lo.is_finite() || !hi.is_finite() || lo > hi {
            return None;
        }
        *cell = 0.5 * (lo + hi);
    }
    Some(m)
}

/// A degenerate interval of a finite float.
fn iv_scalar(x: f64) -> Interval {
    Interval::try_from((x, x)).unwrap_or(Interval::EMPTY)
}

/// The certified `‖·‖` upper bound of one Krawczyk interval entry.
fn interval_abs_sup(iv: Interval) -> Option<f64> {
    if iv.is_empty() || !iv.inf().is_finite() || !iv.sup().is_finite() {
        return None;
    }
    Some(iv.inf().abs().max(iv.sup().abs()))
}

/// The certified conditioning ratio `ρ` and residual norm `η` of the deflated
/// system over the certified root box (theory §2.6, §3's `KrawczykEvidence`).
///
/// `ρ = max_r Σ_c |(I − C·J(B))[r][c]|` and `η = max_r |(C·T(m))[r]|` with
/// `C` the float preconditioner at the box midpoint and `J(B)` the interval
/// Jacobian over the box. `None` when the preconditioner is unavailable, an
/// interval entry is non-finite, or `ρ` is not certified below `1`.
#[allow(clippy::needless_range_loop)] // fixed-size 4x4 row/column index algebra (I − C·J); the index form is the algebra
fn deflated_conditioning(tsys: &TSystem, b: &Box4) -> Option<(f64, f64)> {
    let start = to_interval_box(b)?;
    let m = midpoint(b)?;
    let c = tsys.preconditioner(&m)?;
    let j = tsys.jacobian(&start);
    let f = tsys.f_point(&m);

    let mut rho = 0.0f64;
    for r in 0..4 {
        let mut row = 0.0f64;
        for col in 0..4 {
            let mut acc = iv_scalar(if r == col { 1.0 } else { 0.0 });
            for k in 0..4 {
                acc -= iv_scalar(c[r][k]) * j[k][col];
            }
            let sup = interval_abs_sup(acc)?;
            row += sup;
        }
        rho = rho.max(row);
    }
    if !rho.is_finite() || rho >= 1.0 {
        return None;
    }

    let mut eta = 0.0f64;
    for r in 0..4 {
        let mut acc = iv_scalar(0.0);
        for k in 0..4 {
            acc += iv_scalar(c[r][k]) * f[k];
        }
        let sup = interval_abs_sup(acc)?;
        eta = eta.max(sup);
    }
    if !eta.is_finite() || eta < 0.0 {
        return None;
    }
    Some((rho, eta))
}

// ---------------------------------------------------------------------------
// The exact A₁ witness producer (scope decision 4)
// ---------------------------------------------------------------------------

/// An exact rational polynomial over the four chart variables as a sorted term
/// list `(exponents, coefficient)`, ascending lexicographic exponent order
/// with zero coefficients omitted. The reduction's working ring: it exposes
/// its terms so the multivariate division can read leading monomials (the
/// [`QPoly`] term map is private to `qpoly.rs`).
#[derive(Clone, Debug, PartialEq)]
struct ExactPoly4 {
    terms: Vec<(Vec<u32>, QCoeff)>,
}

impl ExactPoly4 {
    /// The zero polynomial.
    fn zero() -> Self {
        ExactPoly4 { terms: Vec::new() }
    }

    /// Normalize a raw term list: merge equal exponents, drop zero
    /// coefficients, sort ascending lexicographic. `None` on coefficient
    /// overflow.
    fn from_terms(raw: Vec<(Vec<u32>, QCoeff)>) -> Option<Self> {
        let mut merged: Vec<(Vec<u32>, QCoeff)> = Vec::with_capacity(raw.len());
        for (exp, coeff) in raw {
            if exp.len() != 4 || coeff.is_zero() {
                continue;
            }
            match merged.iter_mut().find(|(m, _)| *m == exp) {
                Some((_, slot)) => {
                    let num = slot
                        .num()
                        .checked_mul(coeff.den())
                        .and_then(|v| v.checked_add(coeff.num() * slot.den()))?;
                    let den = slot.den().checked_mul(coeff.den())?;
                    *slot = QCoeff::new(num, den).ok()?;
                }
                None => merged.push((exp, coeff)),
            }
        }
        merged.retain(|(_, c)| !c.is_zero());
        merged.sort_by(|a, b| a.0.cmp(&b.0));
        Some(ExactPoly4 { terms: merged })
    }

    /// The constant polynomial.
    fn constant(c: QCoeff) -> Self {
        let terms = if c.is_zero() {
            Vec::new()
        } else {
            vec![(vec![0u32; 4], c)]
        };
        ExactPoly4 { terms }
    }

    /// Exact addition. `None` on overflow.
    fn add(&self, rhs: &Self) -> Option<Self> {
        let mut raw = self.terms.clone();
        raw.extend(rhs.terms.iter().cloned());
        Self::from_terms(raw)
    }

    /// Exact subtraction. `None` on overflow.
    fn sub(&self, rhs: &Self) -> Option<Self> {
        let mut neg: Vec<(Vec<u32>, QCoeff)> = Vec::with_capacity(rhs.terms.len());
        for (exp, coeff) in &rhs.terms {
            neg.push((exp.clone(), coeff.neg().ok()?));
        }
        let mut raw = self.terms.clone();
        raw.extend(neg);
        Self::from_terms(raw)
    }

    /// The exact product of one term `(exponents, coeff)` into the polynomial
    /// (shift every exponent by `exp`, scale every coefficient by `coeff`).
    /// `None` on overflow.
    fn mul_term(&self, exp: &[u32], coeff: QCoeff) -> Option<Self> {
        let mut raw = Vec::with_capacity(self.terms.len());
        for (e, c) in &self.terms {
            let mut ne = Vec::with_capacity(4);
            for k in 0..4 {
                ne.push(e[k].checked_add(exp[k])?);
            }
            raw.push((ne, c.mul(&coeff).ok()?));
        }
        Self::from_terms(raw)
    }

    /// Whether `this` term list is exactly zero.
    fn is_zero(&self) -> bool {
        self.terms.is_empty()
    }

    /// The monomial total degree of a term.
    fn total(e: &[u32]) -> u32 {
        e.iter().sum()
    }

    /// The chosen monomial order: graded lexicographic with `x0` most
    /// significant, DESCENDING (`a` strictly greater than `b`).
    fn greater(a: &[u32], b: &[u32]) -> bool {
        let ta = Self::total(a);
        let tb = Self::total(b);
        if ta != tb {
            return ta > tb;
        }
        for k in 0..4 {
            if a[k] != b[k] {
                return a[k] > b[k];
            }
        }
        false
    }

    /// The largest term `(index into terms, monomial, coefficient)` under the
    /// chosen order. The list is kept ascending lex, so the scan is over the
    /// graded order only.
    fn largest(&self) -> Option<(usize, Vec<u32>, QCoeff)> {
        let mut best: Option<(usize, &Vec<u32>, QCoeff)> = None;
        for (idx, (exp, coeff)) in self.terms.iter().enumerate() {
            let replace = match best {
                None => true,
                Some((_, bexp, _)) => Self::greater(exp, bexp),
            };
            if replace {
                best = Some((idx, exp, *coeff));
            }
        }
        best.map(|(idx, exp, coeff)| (idx, exp.clone(), coeff))
    }

    /// The leading term (largest monomial) of a polynomial.
    fn leading(&self) -> Option<(Vec<u32>, QCoeff)> {
        self.largest().map(|(_, exp, coeff)| (exp, coeff))
    }

    /// Whether the monomial `d` divides `m` componentwise.
    fn divides(d: &[u32], m: &[u32]) -> bool {
        d.iter().zip(m.iter()).all(|(a, b)| a <= b)
    }

    /// The exact ring value over the four-variable ring (for the ONE
    /// verifier). `None` on overflow.
    fn to_qpoly(&self) -> Option<QPoly> {
        let mut acc = QPoly::zero(4);
        for (exp, coeff) in &self.terms {
            let term = QPoly::term(4, exp, *coeff).ok()?;
            acc = acc.add(&term).ok()?;
        }
        Some(acc)
    }
}

/// The exact dyadic rational a finite `f64` represents. `None` for a
/// non-finite value or a magnitude the `i128` coefficient ring cannot carry.
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
        (mantissa as u128, -1074i32)
    } else {
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

/// The exact integer coefficient of `x^e` in the degree-`d` Bernstein basis
/// element `B^d_a(x)` (`(−1)^{e−a}·C(d,a)·C(d−a, e−a)` for `a ≤ e ≤ d`, else 0).
fn bern_basis_coeff(d: usize, a: usize, e: usize) -> Option<i128> {
    if e < a || e > d {
        return Some(0);
    }
    let binom = |n: usize, k: usize| -> Option<i128> {
        if k > n {
            return Some(0);
        }
        let mut out = 1i128;
        for t in 0..k {
            out = out
                .checked_mul((n - t) as i128)?
                .checked_div((t + 1) as i128)?;
        }
        Some(out)
    };
    let ca = binom(d, a)?;
    let cb = binom(d - a, e - a)?;
    let sign = if (e - a).is_multiple_of(2) { 1 } else { -1 };
    ca.checked_mul(cb).map(|v| sign * v)
}

/// The exact lift of a flat `(degrees, coefficient table)` tensor-Bernstein
/// grid over the unit chart into the monomial basis over ℚ.
///
/// The grid's Bernstein control coefficients are dyadic rationals (exact under
/// [`float_to_qcoeff`]) and the Bernstein basis expands into monomials with
/// integer coefficients, so the conversion is exact over ℚ whenever the stored
/// floats represent the true control values (the CTE fixture data is
/// dyadic-exact). `None` on coefficient/exponent overflow.
fn lift_grid(degrees: [usize; 4], coeffs: &[f64]) -> Option<ExactPoly4> {
    let [d0, d1, d2, d3] = degrees;
    let sp1 = d1 + 1;
    let sp2 = d3 + 1;
    let rows = (d0 + 1) * (d1 + 1);
    let cols = (d2 + 1) * (d3 + 1);
    if coeffs.len() != rows * cols {
        return None;
    }
    let mut raw: Vec<(Vec<u32>, QCoeff)> = Vec::new();
    for a0 in 0..=d0 {
        for b0 in 0..=d1 {
            for a2 in 0..=d2 {
                for a3 in 0..=d3 {
                    let value = coeffs[(a0 * sp1 + b0) * cols + a2 * sp2 + a3];
                    let coeff = float_to_qcoeff(value)?;
                    if coeff.is_zero() {
                        continue;
                    }
                    for e0 in 0..=d0 {
                        let c0 = bern_basis_coeff(d0, a0, e0)?;
                        if c0 == 0 {
                            continue;
                        }
                        for e1 in 0..=d1 {
                            let c1 = bern_basis_coeff(d1, b0, e1)?;
                            if c1 == 0 {
                                continue;
                            }
                            for e2 in 0..=d2 {
                                let c2 = bern_basis_coeff(d2, a2, e2)?;
                                if c2 == 0 {
                                    continue;
                                }
                                for e3 in 0..=d3 {
                                    let c3 = bern_basis_coeff(d3, a3, e3)?;
                                    if c3 == 0 {
                                        continue;
                                    }
                                    let scale =
                                        c0.checked_mul(c1)?.checked_mul(c2)?.checked_mul(c3)?;
                                    if scale == 0 {
                                        continue;
                                    }
                                    let scaled =
                                        QCoeff::new(coeff.num().checked_mul(scale)?, coeff.den())
                                            .ok()?;
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
    ExactPoly4::from_terms(raw)
}

/// The exact lift of one stored system component grid.
fn lift_component(system: &SquareSystem3, component: usize) -> Option<ExactPoly4> {
    let grid = system.grids().get(component)?;
    let (d0, d1, d2, d3) = system.degrees();
    let rows = (d0 + 1) * (d1 + 1);
    let cols = (d2 + 1) * (d3 + 1);
    if grid.len() != rows || grid.iter().any(|row| row.len() != cols) {
        return None;
    }
    let mut flat = Vec::with_capacity(rows * cols);
    for row in grid {
        flat.extend_from_slice(row);
    }
    lift_grid([d0, d1, d2, d3], &flat)
}

/// The exact lift of one frozen chart-minor net.
fn lift_net(net: &crate::tangency::shapes::ChartMinorNet) -> Option<ExactPoly4> {
    let d = net.degrees();
    lift_grid([d[0], d[1], d[2], d[3]], net.coeffs())
}

/// The exact rational quotient `a / b` of two coefficients. `None` on a zero
/// denominator or overflow.
fn coeff_div(a: QCoeff, b: QCoeff) -> Option<QCoeff> {
    if b.is_zero() {
        return None;
    }
    let recip = QCoeff::new(b.den(), b.num()).ok()?;
    a.mul(&recip).ok()
}

/// Divide `num` by the ordered generator list, returning the quotients.
///
/// The multivariate division reduces the current LARGEST term (graded-lex
/// order) by the FIRST generator whose leading monomial divides it; a term no
/// generator divides moves to the remainder. The quotients are exact when the
/// remainder is zero. Deterministic: generator order is fixed as given.
/// `None` on coefficient/exponent overflow (never a wrong quotient).
fn divide_exact(num: &ExactPoly4, gens: &[&ExactPoly4]) -> Option<(Vec<ExactPoly4>, ExactPoly4)> {
    let lead: Vec<Option<(Vec<u32>, QCoeff)>> = gens.iter().map(|g| g.leading()).collect();
    let mut work = num.clone();
    let mut quotients: Vec<ExactPoly4> = gens.iter().map(|_| ExactPoly4::zero()).collect();
    let mut remainder_terms: Vec<(Vec<u32>, QCoeff)> = Vec::new();

    while let Some((_, exp, coeff)) = work.largest() {
        let mut reduced = false;
        for (gi, gen) in gens.iter().enumerate() {
            let Some((lm_exp, lc)) = lead.get(gi).and_then(|l| l.clone()) else {
                continue;
            };
            if !ExactPoly4::divides(&lm_exp, &exp) {
                continue;
            }
            let mut qexp = Vec::with_capacity(4);
            for k in 0..4 {
                qexp.push(exp[k].checked_sub(lm_exp[k])?);
            }
            let qcoeff = coeff_div(coeff, lc)?;
            // work -= (qcoeff·monomial(qexp)) · gen
            let qpoly = ExactPoly4::constant(qcoeff).mul_term(&qexp, QCoeff::one())?;
            let sub = gen.mul_term(&qexp, qcoeff)?;
            quotients[gi] = quotients[gi].add(&qpoly)?;
            work = work.sub(&sub)?;
            reduced = true;
            break;
        }
        if reduced {
            continue;
        }
        let term = ExactPoly4::from_terms(vec![(exp.clone(), coeff)])?;
        remainder_terms.push((exp, coeff));
        work = work.sub(&term)?;
    }
    let remainder = ExactPoly4::from_terms(remainder_terms)?;
    Some((quotients, remainder))
}

/// The exact A₁ witness data of a certified box: `f` reduced modulo
/// `⟨G₁, G₂, M₁, M₂⟩` with a zero remainder yields the multipliers `wᵢ` and
/// hence the exact identity `f = Σᵢ wᵢ·Pᵢ` (`s = 1`).
fn a1_witness_data(
    system: &SquareSystem3,
    chart: &Rank2Chart,
) -> Option<(QPoly, Vec<(QPoly, QPoly)>)> {
    let pivot = chart.pivot();
    let f_comp = pivot.component();
    let gs = g_rows_of(f_comp);
    let f = lift_component(system, f_comp)?;
    let g1 = lift_component(system, gs[0])?;
    let g2 = lift_component(system, gs[1])?;
    let m1 = lift_net(chart.minors().m1())?;
    let m2 = lift_net(chart.minors().m2())?;

    let gens = [&g1, &g2, &m1, &m2];
    let (quotients, remainder) = divide_exact(&f, &gens)?;
    if !remainder.is_zero() {
        return None;
    }
    let f_q = f.to_qpoly()?;
    let g1_q = g1.to_qpoly()?;
    let g2_q = g2.to_qpoly()?;
    let m1_q = m1.to_qpoly()?;
    let m2_q = m2.to_qpoly()?;
    let w = [
        quotients[0].to_qpoly()?,
        quotients[1].to_qpoly()?,
        quotients[2].to_qpoly()?,
        quotients[3].to_qpoly()?,
    ];
    let terms = vec![
        (w[0].clone(), g1_q),
        (w[1].clone(), g2_q),
        (w[2].clone(), m1_q),
        (w[3].clone(), m2_q),
    ];
    Some((f_q, terms))
}

/// Produce and VERIFY an A₁ exact-zero witness for the certified box, or
/// `None` when the exact membership cannot be exhibited (the A₁ arms are then
/// unreachable and the box records `Unresolved`).
fn certified_a1_witness(
    system: &SquareSystem3,
    chart: &Rank2Chart,
) -> Option<ExactVanishingWitness<QPoly>> {
    let (f_q, terms) = a1_witness_data(system, chart)?;
    let one = QPoly::one(4);
    let witness = ExactVanishingWitness::new(WitnessSide::A1, one, terms.clone(), None).ok()?;
    let refs: Vec<&QPoly> = terms.iter().map(|(_, p)| p).collect();
    // The ONE verifier: the exact identity plus the strict nonvanishing of
    // ŝ. The multiplier is the constant `s = 1`, so its exact value over the
    // whole unit chart is the degenerate `[1, 1]`, which strictly excludes
    // zero. Producers never self-certify.
    let verified = ExactWitnessVerifier::verify(&witness, &f_q, &refs, Some((1.0, 1.0)), None);
    if verified.is_ok() {
        Some(witness)
    } else {
        None
    }
}

/// The A₁ route (stages 4/5): at a certified unique deflated root whose
/// critical value is not separated from zero, the exact-zero witness decides
/// whether the contact value is EXACTLY zero (A₁) or is left unresolved.
fn a1_arms(
    system: &SquareSystem3,
    chart: &Rank2Chart,
    over: &GraphEnclosure,
    root: DeflatedRootCert,
    b: &Box4,
) -> Result<ContactVerdict<QPoly>, TangencyRefusal> {
    let h = reduced_hessian(system, chart, over)?;
    let hessian = definiteness(&h)?;
    if matches!(hessian, Definiteness::Singular) {
        // Degenerate contact Hessian: A₂ or higher. Not reachable from this
        // signature (no identity data); record the deepest stage.
        return Ok(ContactVerdict::unresolved(1.0, *b)?);
    }
    let is_node = matches!(hessian, Definiteness::Indefinite { .. });
    let Some(witness) = certified_a1_witness(system, chart) else {
        // The exact zero of the critical value cannot be certified: interval
        // methods alone CANNOT decide A₁ (D7). Record the deepest stage.
        return Ok(ContactVerdict::unresolved(1.0, *b)?);
    };
    let cert = A1Cert::new(chart.clone(), root, hessian, witness)
        .map_err(|_| TangencyRefusal::Input(Refusal::InvalidInput))?;
    if is_node {
        Ok(ContactVerdict::A1Node(cert))
    } else {
        Ok(ContactVerdict::A1Isolated(cert))
    }
}

/// The T1.6a refinement of the stage-3 transversal verdict: when the critical
/// value is certified on one side of zero and `H_h` is certified definite with
/// the matching convexity, the whole box is certified empty.
fn refine_empty(
    system: &SquareSystem3,
    chart: &Rank2Chart,
    over: &GraphEnclosure,
    critical_value: CertifiedInterval,
) -> Result<bool, TangencyRefusal> {
    let h = reduced_hessian(system, chart, over)?;
    let hessian = definiteness(&h)?;
    let empty = match hessian {
        Definiteness::Definite {
            sign: DefinitenessSign::Positive,
            ..
        } => critical_value.lo > 0.0,
        Definiteness::Definite {
            sign: DefinitenessSign::Negative,
            ..
        } => critical_value.hi < 0.0,
        _ => false,
    };
    Ok(empty)
}

/// The frozen cascade entry: classify one box of a chart-certified square
/// system by the theory's stage table (theory §2.11).
///
/// `graph` carries the certified chart and its graph enclosure `Ŷ ⊇ φ(Z)`;
/// `minors` is the chart's minor grids; `tsys` is the deflated
/// `T = (G1, G2, M1, M2)` Krawczyk system of the same chart; `b` is the box,
/// and `budget` is the shared subdivision budget each stage spends
/// deterministically.
pub fn classify_box(
    system: &SquareSystem3,
    graph: &CertifiedGraph,
    minors: &ChartMinorGrids,
    tsys: &TSystem,
    b: &Box4,
    budget: &mut Budget,
) -> Result<ContactVerdict<QPoly>, TangencyRefusal> {
    if !box_ok(b) {
        return Err(TangencyRefusal::Input(Refusal::InvalidInput));
    }
    let chart = &graph.chart;
    let over = &graph.graph;

    // Stage 0: any component separated from zero certifies Empty.
    for component in 0..3 {
        let enc = component_enclosure(system, component, b)?;
        if excludes_zero(&enc) {
            return Ok(ContactVerdict::Empty);
        }
    }

    // Stage 2: (F, M1, M2) has no root in B (T1.5.1). The chart minors of ANY
    // chart vanish at a rank-2 point, so this needs no (H-graph) content.
    match exclude_five(system, minors, chart, b, budget)? {
        ExclusionEvidence::NoRootFiveEq { .. } => {
            return Ok(ContactVerdict::Transversal(
                TransversalCert::no_critical_point(),
            ));
        }
        ExclusionEvidence::CertifiedCriticalPoint(_) | ExclusionEvidence::Inconclusive { .. } => {}
    }

    // Stage 3: Krawczyk certifies a unique root x* of T in the box.
    let start = to_interval_box(b).ok_or(TangencyRefusal::Hull(HullRefusal::DomainNotCompact))?;
    let proof = match krawczyk(tsys, &start, budget) {
        Ok(certified) if certified.value == KrawczykProof::Unique => certified,
        Ok(_) | Err(_) => {
            // No unique deflated root certified on this box.
            return Ok(ContactVerdict::unresolved(1.0, *b)?);
        }
    };
    let _ = proof;

    // The certified root certificate over the box (rho, eta).
    let Some((rho, eta)) = deflated_conditioning(tsys, b) else {
        return Ok(ContactVerdict::unresolved(1.0, *b)?);
    };
    let root = DeflatedRootCert::new(*b, rho, eta)
        .map_err(|_| TangencyRefusal::Input(Refusal::InvalidInput))?;

    // The critical value f̂(X*) (X* ⊇ the unique root).
    let f_comp = chart.pivot().component();
    let critical_value = component_enclosure(system, f_comp, b)?;

    if excludes_zero(&critical_value) {
        // T1.5.2: the critical value is certified nonzero, so the box is
        // transversal; refine to Empty when the T1.6a convexity applies.
        if refine_empty(system, chart, over, critical_value)? {
            return Ok(ContactVerdict::Empty);
        }
        let transversal = TransversalCert::unique_root(root, critical_value)
            .map_err(|_| TangencyRefusal::Input(Refusal::InvalidInput))?;
        return Ok(ContactVerdict::Transversal(transversal));
    }

    // Stages 4/5: the critical value is not separated from zero. The A₁ arms
    // require the exact-zero witness; the definiteness of H_h over the graph
    // decides A₁⁺ (isolated) from A₁⁻ (node).
    a1_arms(system, chart, over, root, b)
}

#[cfg(test)]
pub(crate) mod test_kit {
    //! Fixture systems for the CTE-004 tests (F3 definite contact, F4 saddle
    //! node, F6 crossing planes) built over the unit chart from monomial data.
    //! TEST ONLY.

    use super::*;
    use crate::tangency::minors::build_chart_minors;
    use crate::tangency::shapes::{ChartMinorNet, Rank2Pivot};

    /// A four-axis monomial Bernstein grid over the identity chart.
    pub(crate) fn monomial_grid4(
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

    /// A square system from three monomial component grids over the identity
    /// chart.
    fn system_from_monomials(
        degrees: (usize, usize, usize, usize),
        comps: [&[([usize; 4], f64)]; 3],
    ) -> SquareSystem3 {
        let grids = [
            monomial_grid4(degrees, comps[0]),
            monomial_grid4(degrees, comps[1]),
            monomial_grid4(degrees, comps[2]),
        ];
        SquareSystem3::new(grids, degrees, (0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0))
            .expect("the fixture square system admits")
    }

    /// The F3 (A₁⁺, definite) square system over the unit chart: components
    /// `(G1, G2, f) = (y1 − 1/2, y2 − 1/2, (z1−1/2)² + (z2−1/2)²)` in the chart
    /// axes `(y1, y2, z1, z2) = (0, 1, 2, 3)`. The reduced contact function
    /// `h = (z1−1/2)² + (z2−1/2)²` is positive definite with an interior
    /// minimum at the chart centre, matching the F3 kit's A₁⁺ ground truth.
    pub(crate) fn f3_system() -> SquareSystem3 {
        system_from_monomials(
            (1, 1, 2, 2),
            [
                &[([1, 0, 0, 0], 1.0), ([0, 0, 0, 0], -0.5)],
                &[([0, 1, 0, 0], 1.0), ([0, 0, 0, 0], -0.5)],
                &[
                    ([0, 0, 2, 0], 1.0),
                    ([0, 0, 1, 0], -1.0),
                    ([0, 0, 0, 2], 1.0),
                    ([0, 0, 0, 1], -1.0),
                    ([0, 0, 0, 0], 0.5),
                ],
            ],
        )
    }

    /// The F4 (A₁⁻, saddle node) square system over the unit chart: components
    /// `(G1, G2, f) = (y1 − 1/2, y2 − 1/2, (z1−1/2)² − (z2−1/2)²)`. The reduced
    /// contact function is the saddle of the F4 kit with a node at the chart
    /// centre (`det H_h = −4` on the fixture's integer data).
    pub(crate) fn f4_system() -> SquareSystem3 {
        system_from_monomials(
            (1, 1, 2, 2),
            [
                &[([1, 0, 0, 0], 1.0), ([0, 0, 0, 0], -0.5)],
                &[([0, 1, 0, 0], 1.0), ([0, 0, 0, 0], -0.5)],
                &[
                    ([0, 0, 2, 0], 1.0),
                    ([0, 0, 1, 0], -1.0),
                    ([0, 0, 0, 2], -1.0),
                    ([0, 0, 0, 1], 1.0),
                ],
            ],
        )
    }

    /// The pivot of the F3/F4 chart systems: `f` is component 2, `y = (0, 1)`,
    /// so `A = D_yG` is the identity and `det A = 1`.
    pub(crate) fn a1_pivot() -> Rank2Pivot {
        Rank2Pivot::new(2, (0, 1)).expect("the A1 pivot admits")
    }

    /// The A₁ box: the certified root `(1/2, 1/2, 1/2, 1/2)` lies strictly
    /// inside every axis.
    pub(crate) fn a1_box() -> Box4 {
        [(0.4, 0.6), (0.4, 0.6), (0.4, 0.6), (0.4, 0.6)]
    }

    /// The exact Bernstein control value of the monomial `x^exp` at the net
    /// index `idx` of a tensor net of per-axis degrees `d`: the per-axis
    /// factor `C(idx, exp)/C(d, exp)` (zero when `exp > idx`), exact for the
    /// small dyadic degrees of the fixture nets.
    fn monomial_control(exp: &[usize; 4], idx: &[usize; 4]) -> f64 {
        let binom = |n: usize, k: usize| -> f64 {
            if k > n {
                return 0.0;
            }
            let mut out = 1.0f64;
            for t in 0..k {
                out = out * ((n - t) as f64) / ((t + 1) as f64);
            }
            out
        };
        let mut out = 1.0f64;
        for axis in 0..4 {
            out *= binom(idx[axis], exp[axis]);
            out /= binom([1, 1, 2, 2][axis], exp[axis]);
        }
        out
    }

    /// A frozen chart-minor net over the fixture degrees `(1, 1, 2, 2)` whose
    /// exact monomial content is `Σ coeff·x^exp`.
    fn exact_minor_net(terms: &[([usize; 4], f64)]) -> ChartMinorNet {
        let degrees = [1usize, 1, 2, 2];
        let dims = [2usize, 2, 3, 3];
        let mut coeffs = vec![0.0f64; dims.iter().product()];
        for (exp, coeff) in terms {
            for a0 in 0..dims[0] {
                for a1 in 0..dims[1] {
                    for a2 in 0..dims[2] {
                        for a3 in 0..dims[3] {
                            let control = monomial_control(exp, &[a0, a1, a2, a3]);
                            if control == 0.0 {
                                continue;
                            }
                            let row = a0 * 2 + a1;
                            let col = a2 * 3 + a3;
                            coeffs[row * 9 + col] += coeff * control;
                        }
                    }
                }
            }
        }
        ChartMinorNet::new(degrees, coeffs).expect("the exact minor net admits")
    }

    /// A fully built A₁ chart whose chart-minor nets carry the EXACT chart
    /// minors of the fixture.
    ///
    /// The A₁ exact-zero witness (theory §4.1 / D7) is an exact rational
    /// statement, so the fixture's deflated rows must be the EXACT chart
    /// minors, not the float-composed nets of [`build_chart_minors`] (which
    /// perturb the deflated system by rounding and make exact membership
    /// unreachable). For these chart systems `G1 = y1 − 1/2`, `G2 = y2 − 1/2`,
    /// `det A = 1` and `f` is independent of `y`, so the chart minors are
    /// exactly `M_j = ∂f/∂z_j` (theory T1.1), which the fixture derives
    /// monomially.
    pub(crate) fn a1_chart(system: &SquareSystem3) -> Rank2Chart {
        let pivot = a1_pivot();
        let f = lift_component(system, 2).expect("the A1 f component lifts exactly");
        let mut m1_terms: Vec<([usize; 4], f64)> = Vec::new();
        let mut m2_terms: Vec<([usize; 4], f64)> = Vec::new();
        for (exp, coeff) in &f.terms {
            let e = [
                exp[0] as usize,
                exp[1] as usize,
                exp[2] as usize,
                exp[3] as usize,
            ];
            if e[2] > 0 {
                let mut ne = e;
                ne[2] -= 1;
                m1_terms.push((ne, e[2] as f64 * qcoeff_float(*coeff)));
            }
            if e[3] > 0 {
                let mut ne = e;
                ne[3] -= 1;
                m2_terms.push((ne, e[3] as f64 * qcoeff_float(*coeff)));
            }
        }
        let m1 = exact_minor_net(&m1_terms);
        let m2 = exact_minor_net(&m2_terms);
        let minors = ChartMinorGrids::new(m1, m2).expect("the exact minor grids admit");
        Rank2Chart::new(pivot, CertifiedInterval { lo: 0.5, hi: 1.5 }, minors)
            .expect("the A1 chart admits")
    }

    /// The exact `f64` reading of a rational coefficient (dyadic values are
    /// exact).
    fn qcoeff_float(c: QCoeff) -> f64 {
        if c.is_zero() {
            return 0.0;
        }
        c.num() as f64 / c.den() as f64
    }

    /// The certified (H-graph) content of an A₁ system over the box.
    pub(crate) fn a1_certified_graph(
        system: &SquareSystem3,
        chart: &Rank2Chart,
        b: &Box4,
    ) -> CertifiedGraph {
        crate::tangency::graph::certify_graph(system, chart, *b)
            .expect("the A1 graph certifies on the box")
    }

    /// The deflated system of an A₁ chart.
    pub(crate) fn a1_tsystem(system: &SquareSystem3, chart: &Rank2Chart) -> TSystem {
        TSystem::from_deflated(system, chart).expect("the A1 deflated system builds")
    }

    /// The F6 (transversal crossing planes) square system: the two plane
    /// patches of the exclude driver's support (intersection line strictly
    /// inside the chart).
    pub(crate) fn f6_system() -> SquareSystem3 {
        crate::tangency::minors::support::f6_system().expect("the F6 system admits")
    }

    /// A box straddling the F6 intersection line with the (H-graph) content
    /// strictly certifiable (`Y ⊃ φ(Z)`).
    pub(crate) fn f6_box() -> Box4 {
        [(0.1, 0.9), (0.1, 0.9), (0.25, 0.75), (0.25, 0.75)]
    }

    /// The F6 chart over the box: pivot `f = 2, y = (0, 1)` (the same pivot the
    /// exclusion driver's admission uses).
    pub(crate) fn f6_chart(system: &SquareSystem3) -> Rank2Chart {
        let pivot = Rank2Pivot::new(2, (0, 1)).expect("the F6 pivot admits");
        let placeholder =
            ChartMinorNet::new([1, 1, 1, 1], vec![1.0; 16]).expect("a valid placeholder net");
        let minors = ChartMinorGrids::new(placeholder.clone(), placeholder)
            .expect("a valid placeholder grid");
        let template = Rank2Chart::new(pivot, CertifiedInterval { lo: 0.5, hi: 1.5 }, minors)
            .expect("the F6 template chart admits");
        let minors = build_chart_minors(system, &template).expect("the F6 minors build");
        Rank2Chart::new(pivot, CertifiedInterval { lo: 0.5, hi: 1.5 }, minors)
            .expect("the F6 chart admits")
    }

    /// The certified (H-graph) content of the F6 system over the straddling
    /// box.
    pub(crate) fn f6_certified_graph(
        system: &SquareSystem3,
        chart: &Rank2Chart,
        b: &Box4,
    ) -> CertifiedGraph {
        crate::tangency::graph::certify_graph(system, chart, *b)
            .expect("the F6 graph certifies on the box")
    }

    /// The deflated system of the F6 chart.
    pub(crate) fn f6_tsystem(system: &SquareSystem3, chart: &Rank2Chart) -> TSystem {
        TSystem::from_deflated(system, chart).expect("the F6 deflated system builds")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tangency::fixtures::{
        F3PlaneSphereFixture, F4SaddleFixture, F6CrossingPlanesFixture,
    };

    /// A fully built F3/F4 classification input: system, chart, certified
    /// graph, deflated system, and the box.
    fn a1_input(system: &SquareSystem3) -> (Rank2Chart, CertifiedGraph, TSystem, Box4) {
        let chart = test_kit::a1_chart(system);
        let b = test_kit::a1_box();
        let graph = test_kit::a1_certified_graph(system, &chart, &b);
        let tsys = test_kit::a1_tsystem(system, &chart);
        (chart, graph, tsys, b)
    }

    /// Re-verify the exact-zero witness of an A₁ certificate through the ONE
    /// verifier against the lifted system polynomials.
    fn assert_witness_verifies(
        system: &SquareSystem3,
        chart: &Rank2Chart,
        witness: &ExactVanishingWitness<QPoly>,
    ) {
        let (f_q, terms) = a1_witness_data(system, chart).expect("the exact witness data lifts");
        let refs: Vec<&QPoly> = terms.iter().map(|(_, p)| p).collect();
        ExactWitnessVerifier::verify(witness, &f_q, &refs, Some((1.0, 1.0)), None)
            .expect("the exact-zero witness must verify against the system polynomials");
    }

    #[test]
    fn f3_isolated_contact_verdict() {
        // The F3 kit's ground truth (plane × sphere, isolated A₁⁺ contact).
        F3PlaneSphereFixture::new()
            .admit()
            .expect("the F3 ground truth admits");
        let system = test_kit::f3_system();
        let (chart, graph, tsys, b) = a1_input(&system);
        let minors = chart.minors().clone();
        let mut budget = Budget::new(256, 0, 0);
        let verdict = classify_box(&system, &graph, &minors, &tsys, &b, &mut budget)
            .expect("the F3 classification certifies");
        match verdict {
            ContactVerdict::A1Isolated(cert) => {
                // T1.6b: definite H_h with a certified modulus μ > 0.
                match cert.hessian() {
                    Definiteness::Definite { sign, mu } => {
                        assert_eq!(sign, DefinitenessSign::Positive);
                        assert!(mu.get() > 0.0, "the A1⁺ modulus must be positive");
                    }
                    other => panic!("F3 must certify Definite, got {other:?}"),
                }
                // The certified root of T is a contraction (rho < 1).
                assert!(
                    cert.root().rho() >= 0.0 && cert.root().rho() < 1.0,
                    "the deflated root certificate carries rho < 1"
                );
                // The A₁ certificate REQUIRES the exact-zero witness (D7).
                assert_eq!(cert.zero_witness().side(), WitnessSide::A1);
                assert_witness_verifies(&system, &chart, cert.zero_witness());
            }
            other => panic!("F3 must classify A1Isolated, got {other:?}"),
        }
    }

    #[test]
    fn f4_node_verdict_is_not_unresolved() {
        // The R3 gate fixture: the F4 saddle's node must classify A1Node —
        // never Unresolved.
        F4SaddleFixture::new()
            .admit()
            .expect("the F4 ground truth admits");
        let system = test_kit::f4_system();
        let (chart, graph, tsys, b) = a1_input(&system);
        let minors = chart.minors().clone();
        let mut budget = Budget::new(256, 0, 0);
        let verdict = classify_box(&system, &graph, &minors, &tsys, &b, &mut budget)
            .expect("the F4 classification certifies");
        match verdict {
            ContactVerdict::A1Node(cert) => {
                // T1.6c: certified indefiniteness via det Ĥ_h < 0.
                match cert.hessian() {
                    Definiteness::Indefinite { det_upper } => {
                        assert!(det_upper < 0.0, "the node's det bound must be negative");
                    }
                    other => panic!("F4 must certify Indefinite, got {other:?}"),
                }
                assert_eq!(cert.zero_witness().side(), WitnessSide::A1);
                assert_witness_verifies(&system, &chart, cert.zero_witness());
            }
            other => panic!("F4 must classify A1Node, got {other:?}"),
        }
    }

    #[test]
    fn f6_transversal_by_exclusion() {
        // The T1.5(1) sharpening example: two crossing planes whose box
        // straddles the intersection line — the five-equation exclusion
        // certifies transversality where per-minor separation alone cannot.
        F6CrossingPlanesFixture::new()
            .admit()
            .expect("the F6 ground truth admits");
        let system = test_kit::f6_system();
        let chart = test_kit::f6_chart(&system);
        let b = test_kit::f6_box();
        let graph = test_kit::f6_certified_graph(&system, &chart, &b);
        let tsys = test_kit::f6_tsystem(&system, &chart);
        let minors = chart.minors().clone();
        let mut budget = Budget::new(256, 0, 0);
        let verdict = classify_box(&system, &graph, &minors, &tsys, &b, &mut budget)
            .expect("the F6 classification certifies");
        match verdict {
            ContactVerdict::Transversal(cert) => {
                assert!(
                    matches!(
                        cert.evidence(),
                        crate::tangency::shapes::TransversalEvidence::NoCriticalPoint
                    ),
                    "F6 must be certified transversal by the T1.5.1 no-root exclusion"
                );
            }
            other => panic!("F6 must classify Transversal, got {other:?}"),
        }
    }

    #[test]
    fn cascade_verdicts_are_deterministic() {
        // Same input twice (fresh budgets) yields the identical verdict
        // field-for-field: fixed stage order, fixed subdivision and reduction
        // rules, no hash iteration.
        F4SaddleFixture::new()
            .admit()
            .expect("the F4 ground truth admits");
        let system = test_kit::f4_system();
        let (chart, graph, tsys, b) = a1_input(&system);
        let minors = chart.minors().clone();

        let run = |budget: &mut Budget| -> ContactVerdict<QPoly> {
            classify_box(&system, &graph, &minors, &tsys, &b, budget)
                .expect("the deterministic classification certifies")
        };
        let mut first_budget = Budget::new(256, 0, 0);
        let first = run(&mut first_budget);
        let mut second_budget = Budget::new(256, 0, 0);
        let second = run(&mut second_budget);
        assert_eq!(
            first, second,
            "classify_box must be field-for-field deterministic"
        );
    }
}
