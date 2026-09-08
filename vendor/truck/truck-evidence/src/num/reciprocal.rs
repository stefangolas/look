// The crate-level deny list includes clippy::indexing_slicing. The dense
// Bernstein/monomial coefficient arithmetic below indexes only fixed-capacity
// vectors whose lengths are recomputed in the same expression (never a value
// derived from untrusted geometry), so the lint is re-allowed for this module
// only, on the parallelotope.rs precedent.
#![allow(clippy::indexing_slicing)]

//! ADM-L5-RECIPROCAL — Theorem D: the certified reciprocal-power primitive.
//!
//! This is a PURE polynomial mathematics packet in the `num/` substrate (the
//! `krawczyk.rs` home). It lands `certified_reciprocal_power(W, p,
//! target_error)`, the exact polynomial `Q_m` approximating `W^{-p}` with a
//! CERTIFIED uniform error bound `eps_m` — the reciprocal-power
//! polynomialization of Theorem D from which ADM-003's volume assembly
//! brackets rational-face integrals.
//!
//! # The lemma (verbatim from the adopted review)
//!
//! Given a Bernstein polynomial `W` on `[0, 1]` with certified hull
//! `0 < w₋ ≤ W ≤ w₊`, set `w₀ = (w₋+w₊)/2`, `δ = (W − w₀)/w₀`,
//! `|δ| ≤ ρ = (w₊−w₋)/(w₊+w₋) < 1`. Then
//!
//! ```text
//! W^{-p} = w₀^{-p} (1+δ)^{-p},   (1+δ)^{-p} = Σ_{k≥0} (−1)^k C(k+p−1, p−1) δ^k
//! ```
//!
//! Truncating after `m`: `Q_m = w₀^{-p} Σ_{k≤m} (−1)^k C(k+p−1, p−1) δ^k` is
//! an ordinary polynomial with the CERTIFIED uniform tail
//!
//! ```text
//! eps_m ≤ w₀^{-p} Σ_{k=m+1}^∞ C(k+p−1, p−1) ρ^k
//! ```
//!
//! (the closed-form geometric-sum bound `w₀^{-p}((1−ρ)^{-p} − Σ_{k≤m} C ρ^k)`,
//! which telescopes to `w₋^{-p} − w₀^{-p} Σ_{k≤m} C(k+p−1,p−1) ρ^k` because
//! `w₀(1−ρ) = w₋`). Any integral of `W^{-p}·(polynomial)` computed with `Q_m`
//! is within `‖polynomial‖∞ · eps_m · measure` of the truth.
//!
//! # What is certified here
//!
//! The returned `q` is a vector of [`Interval`] enclosures of the EXACT
//! coefficients of `Q_m` in the degree-`m·n` Bernstein basis on `[0, 1]`. The
//! coefficients are assembled in EXACT rational arithmetic ([`Rat`], fixed
//! width `i128`): the inputs are exact dyadic `f64` values, `w₀` is an exact
//! rational midpoint, the binomial coefficients `(−1)^k C(k+p−1, p−1)` are
//! exact integers, and every polynomial product is an exact rational
//! convolution. Only the final rational → [`Interval`] projection rounds, and
//! only outward. Wherever a coefficient is exactly representable (a dyadic
//! hull), its enclosure is DEGENERATE — the coefficient is exact.
//!
//! The returned `eps` encloses the closed-form tail above (outward-rounded
//! interval evaluation of the same identity), so
//!
//! ```text
//! for all t ∈ [0,1]:  |W(t)^{-p} − Q_m^⋆(t)| ≤ eps.sup()
//! ```
//!
//! with `Q_m^⋆` the exact truncated polynomial whose coefficients lie inside
//! the `q` enclosures. No coefficient of `Q_m` and no bound in `eps_m` is a
//! naked float: every value is exact or an outward-rounded enclosure. There is
//! no quadrature anywhere and no uncertified truncation: the order `m` is the
//! first order whose CERTIFIED bound `eps_m` meets `target_error`.
//!
//! # Why monomial accumulation is exact
//!
//! All coefficient arithmetic runs in the MONOMIAL basis, where powers of `δ`
//! are plain convolutions (no binomial normalization) and polynomial sums of
//! different degrees align slot-wise. The final polynomial is converted once to
//! the Bernstein basis of degree `m·n` by the exact identity
//! `t^j = Σ_i C(i,j)/C(m·n,j) B_i(t)`. Binomials are exact `i128` integers.
//! Degrees are capped at [`MAX_DEGREE`] so every binomial stays inside `i128`;
//! an input that needs a higher order refuses with
//! [`Refusal::ForwardToleranceExceeded`] — the caller's cue to subdivide the
//! parameter domain (the packet's own convergence rule). A `Rat` that would
//! overflow `i128` (a hull whose odd denominator powers outgrow the fixed
//! width) refuses [`Refusal::Empty`]: the module never silently rounds.
//!
//! # Refusals
//!
//! - empty / non-finite coefficients, `p == 0`, non-finite or non-positive
//!   `target_error` ⇒ [`Refusal::Empty`];
//! - a bracket that is not strictly positive (`w₋ ≤ 0`) ⇒
//!   [`Refusal::UnsupportedEnvelope`] with
//!   [`EnvelopeCase::NonPositiveNurbsWeight`] — the primitive never divides by
//!   an uncertified sign;
//! - `ρ ≥ 1` after outward rounding, an order beyond [`MAX_DEGREE`] needed to
//!   meet `target_error`, or arithmetic outside the fixed-width exact lattice
//!   ⇒ typed refusals.

use inari::Interval;
use truck_base::evidence::{
    Budget, Certificate, Certified, EnvelopeCase, Margin, Method, Modulus, Outcome, PropMap,
    Refusal,
};

/// The highest degree `m·n` of `Q_m` this module can carry: every binomial
/// `C(d, j)` with `d ≤ MAX_DEGREE` fits `i128` with margin for the
/// multiplicative intermediate, so the exact monomial→Bernstein conversion
/// stays in [`Rat`]. The caller subdivides (the packet's rule) when the
/// required order would exceed it. H-3: a dimensionless polynomial degree, not
/// a length.
const MAX_DEGREE: usize = 100;

/// An exact rational `num / den` (`den > 0`, reduced). Fixed-width `i128` by
/// design: every operation is checked, and an overflow is a typed refusal,
/// never a silent rounding (H-6).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Rat {
    num: i128,
    den: i128,
}

impl Rat {
    const ZERO: Self = Rat { num: 0, den: 1 };
    const ONE: Self = Rat { num: 1, den: 1 };

    /// `num/den`, reduced; `den` must be nonzero.
    fn new(num: i128, den: i128) -> Option<Self> {
        if den == 0 {
            return None;
        }
        let (num, den) = if den < 0 { (-num, -den) } else { (num, den) };
        let g = gcd(num.unsigned_abs(), den as u128) as i128;
        Some(Rat {
            num: num / g,
            den: den / g,
        })
    }

    fn from_i128(n: i128) -> Self {
        Rat { num: n, den: 1 }
    }

    fn is_zero(self) -> bool {
        self.num == 0
    }

    fn neg(self) -> Self {
        Rat {
            num: -self.num,
            den: self.den,
        }
    }

    /// `self + other`, checked. Aligns to the least common denominator (the
    /// larger power-of-two exponent in the dyadic case) instead of multiplying
    /// denominators, so numerators and denominators stay at their minimal
    /// dyadic scale.
    fn add(self, other: Self) -> Option<Self> {
        let g = gcd(self.den as u128, other.den as u128) as i128;
        let lcm = self.den / g * other.den;
        let na = self.num.checked_mul(lcm / self.den)?;
        let nb = other.num.checked_mul(lcm / other.den)?;
        let num = na.checked_add(nb)?;
        Rat::new(num, lcm)
    }

    /// `self − other`, checked.
    fn sub(self, other: Self) -> Option<Self> {
        self.add(other.neg())
    }

    /// `self · other`, checked. Cancels cross factors before multiplying so
    /// intermediate products stay inside `i128`.
    fn mul(self, other: Self) -> Option<Self> {
        // Cancel self.num with other.den, other.num with self.den first.
        let g1 = gcd(self.num.unsigned_abs(), other.den as u128) as i128;
        let num1 = self.num / g1;
        let den2 = other.den / g1;
        let g2 = gcd(other.num.unsigned_abs(), self.den as u128) as i128;
        let num2 = other.num / g2;
        let den1 = self.den / g2;
        let num = num1.checked_mul(num2)?;
        let den = den1.checked_mul(den2)?;
        Rat::new(num, den)
    }

    /// `self / other`, checked; `other` must be nonzero.
    fn div(self, other: Self) -> Option<Self> {
        if other.num == 0 {
            return None;
        }
        Rat::new(
            self.num.checked_mul(other.den)?,
            self.den.checked_mul(other.num)?,
        )
    }

    /// `self^e` for a nonnegative integer `e`, by square-and-multiply.
    fn pow(self, e: u64) -> Option<Self> {
        let mut result = Rat::ONE;
        let mut base = self;
        let mut exp = e;
        while exp > 0 {
            if exp & 1 == 1 {
                result = result.mul(base)?;
            }
            exp >>= 1;
            if exp > 0 {
                base = base.mul(base)?;
            }
        }
        Some(result)
    }

    /// The reciprocal, checked (`self` must be nonzero).
    fn recip(self) -> Option<Self> {
        Rat::new(self.den, self.num)
    }
}

/// The exact value of a finite `f64` as a [`Rat`]: every `f64` is a dyadic
/// rational. `None` when the dyadic denominator (or a scale-up) leaves the
/// `i128` lattice.
fn f64_to_rat(x: f64) -> Option<Rat> {
    if !x.is_finite() {
        return None;
    }
    if x == 0.0 {
        return Some(Rat::ZERO);
    }
    let bits = x.to_bits();
    let sign: i128 = if bits >> 63 == 1 { -1 } else { 1 };
    let exp_bits = ((bits >> 52) & 0x7ff) as i64;
    let frac = bits & 0x000f_ffff_ffff_ffff;
    let (mant, e) = if exp_bits == 0 {
        // Subnormal: value = frac · 2^{-1074}.
        (frac as i128, -1074i64)
    } else {
        // Normal: value = (2^52 + frac) · 2^{exp_bits − 1023 − 52}.
        ((frac | (1 << 52)) as i128, exp_bits - 1023 - 52)
    };
    // Strip powers of two from the mantissa into the exponent.
    let v2 = mant.trailing_zeros() as i64;
    let mant = mant >> v2;
    let e = e + v2;
    let (num, den) = if e >= 0 {
        if e > 126 {
            return None;
        }
        (mant.checked_mul(1i128 << e)?, 1)
    } else {
        if e < -126 {
            return None;
        }
        (mant, 1i128 << (-e))
    };
    Rat::new(sign * num, den)
}

/// `gcd` of two nonnegative magnitudes (binary Euclid).
fn gcd(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    a
}

/// An enclosure of a (possibly negative) `i128` integer: degenerate when the
/// integer is exactly representable in `f64`, otherwise a two-ulp span that
/// provably contains it.
fn int_iv(v: i128) -> Interval {
    if v == 0 {
        return interval_at(0.0);
    }
    let a = v.unsigned_abs();
    if a < (1i128 << 53).unsigned_abs() {
        return interval_at(v as f64);
    }
    let r = v as f64;
    let lo = f64::from_bits(r.to_bits() - 1);
    let hi = f64::from_bits(r.to_bits() + 1);
    Interval::try_from((lo, hi)).unwrap_or(Interval::EMPTY)
}

/// An outward enclosure of an exact [`Rat`]: the exact ratio lies in the
/// interval; a dyadic ratio that is exactly representable gives a degenerate
/// interval.
fn rat_iv(r: Rat) -> Interval {
    int_iv(r.num) / int_iv(r.den)
}

/// The certificate of [`certified_reciprocal_power`]: the truncated
/// polynomial `Q_m`, the truncation order `m`, and the certified tail.
#[derive(Clone, Debug)]
pub struct ReciprocalPower {
    /// The weight polynomial's degree `n` (`coeffs.len() − 1`).
    pub degree: usize,
    /// The truncation order actually used: the first `m` whose certified
    /// bound `eps_m.sup() ≤ target_error`.
    pub m: usize,
    /// Certified enclosures of the EXACT coefficients of `Q_m` in the
    /// Bernstein basis of degree `m·degree` on `[0, 1]`, length
    /// `m·degree + 1`. Entries are degenerate when the exact coefficient is
    /// exactly representable; every entry contains its exact value.
    pub q: Vec<Interval>,
    /// The certified uniform tail bound `eps_m` (an enclosure of the
    /// closed-form geometric-sum bound above): with `Q_m^⋆` the exact
    /// truncated polynomial whose coefficients lie inside the `q` enclosures,
    /// `|W(t)^{-p} − Q_m^⋆(t)| ≤ eps.sup()` for every `t ∈ [0, 1]`.
    pub eps: Interval,
    /// The certified weight bracket `(w₋, w₊)` from the Bernstein hull:
    /// `0 < w₋ ≤ W(t) ≤ w₊` on `[0, 1]`.
    pub bracket: (f64, f64),
}

impl ReciprocalPower {
    /// The midpoint coefficients of `Q_m` — a concrete polynomial whose
    /// uniform deviation from `W^{-p}` is at most `eps.sup() + half_width`,
    /// where `half_width` is [`ReciprocalPower::half_width`]. Provided for
    /// callers that integrate in plain floats; the certified object is the
    /// `q`/`eps` pair.
    pub fn q_mid(&self) -> Vec<f64> {
        self.q.iter().map(|iv| iv.mid()).collect()
    }

    /// Half the largest coefficient-enclosure width. A caller using the
    /// midpoint coefficients [`ReciprocalPower::q_mid`] must add this to
    /// `eps.sup()` (uniformly, because the Bernstein basis sums to 1).
    pub fn half_width(&self) -> f64 {
        self.q
            .iter()
            .map(|iv| 0.5 * (iv.sup() - iv.inf()))
            .fold(0.0_f64, f64::max)
    }
}

/// The certified reciprocal-power polynomialization of Theorem D.
///
/// `coeffs` are the Bernstein coefficients of `W` over `[0, 1]` (degree
/// `coeffs.len() − 1`), `p ≥ 1` is the reciprocal power (`W^{-p}`), and
/// `target_error > 0` is the caller's absolute uniform error budget. Returns
/// `(Q_m, eps_m)` in the [`ReciprocalPower`] carrier: `m` is the first
/// truncation order whose certified geometric tail bound is within
/// `target_error`.
pub fn certified_reciprocal_power(
    coeffs: &[f64],
    p: u32,
    target_error: f64,
) -> Outcome<ReciprocalPower> {
    // Decision 0: degenerate inputs — nothing to certify.
    if coeffs.is_empty()
        || p == 0
        || !target_error.is_finite()
        || target_error <= 0.0
        || coeffs.iter().any(|c| !c.is_finite())
    {
        return Err(Refusal::Empty);
    }
    let n = coeffs.len() - 1;

    // The certified bracket is the Bernstein hull: W is a convex combination
    // of its coefficients, so its range lies between the coefficient extrema.
    let mut w_lo = f64::INFINITY;
    let mut w_hi = f64::NEG_INFINITY;
    for c in coeffs {
        w_lo = w_lo.min(*c);
        w_hi = w_hi.max(*c);
    }
    if w_lo <= 0.0 || !w_hi.is_finite() {
        return Err(Refusal::UnsupportedEnvelope(
            EnvelopeCase::NonPositiveNurbsWeight,
        ));
    }

    let w0 = interval_at(0.5) * (interval_at(w_lo) + interval_at(w_hi));
    let rho = (interval_at(w_hi) - interval_at(w_lo)) / (interval_at(w_hi) + interval_at(w_lo));
    if rho.is_empty() || !rho.inf().is_finite() || rho.sup() >= 1.0 {
        // ρ ≥ 1 (after outward rounding) makes the geometric tail diverge:
        // no certified finite bound exists.
        return Err(Refusal::Empty);
    }

    // The closed-form geometric bound telescopes to w₋^{-p} − w₀^{-p}·S_m,
    // with S_m = Σ_{k≤m} C(k+p−1, p−1) ρ^k, because w₀(1−ρ) = w₋ exactly.
    // Both arms are evaluated outward in intervals so the difference encloses
    // the exact tail value for the true ρ.
    let full = pos_pow(interval_at(w_lo).recip(), p as u64);
    let w0_inv = pos_pow(w0.recip(), p as u64);
    if !finite(&full) || !finite(&w0_inv) {
        return Err(Refusal::Empty);
    }

    // Largest buildable order: the final degree m·n must stay ≤ MAX_DEGREE so
    // the binomials of the Bernstein conversion remain exact in i128.
    let m_build_max = MAX_DEGREE.checked_div(n).unwrap_or(0);

    // Select the first m whose certified tail meets the budget. S_m and its
    // running term are advanced incrementally (a_k = a_{k−1}·ρ·(k+p−1)/k), so
    // the search is O(m) interval operations and the bound is monotone in m.
    let mut m: usize = 0;
    let mut term = interval_at(1.0); // a_0 = C(p−1, p−1) ρ^0 = 1
    let mut s_m = interval_at(1.0); // S_0
    let eps = loop {
        let eps_m = full - w0_inv * s_m;
        if eps_m.is_empty() || !eps_m.inf().is_finite() || !eps_m.sup().is_finite() {
            return Err(Refusal::Empty);
        }
        if eps_m.sup() <= target_error {
            break eps_m;
        }
        if m >= m_build_max {
            // The certified bound cannot meet the budget within the buildable
            // orders: refuse typed — the caller subdivides in ρ (the packet's
            // convergence rule).
            return Err(Refusal::ForwardToleranceExceeded {
                bound: eps_m.sup(),
                allowed: target_error,
            });
        }
        m += 1;
        // a_m = a_{m−1} · ρ · (m+p−1) / m; exact as integers, outward in
        // intervals.
        let ratio = interval_at((m as u64 + p as u64 - 1) as f64) / interval_at(m as f64);
        term = term * rho * ratio;
        s_m += term;
    };

    // Assemble Q_m's coefficients exactly (rational monomial accumulation,
    // then a single Bernstein conversion at the end). Order zero needs no
    // weight polynomial at all: Q_0 ≡ w₀^{-p}, exact and degenerate.
    let q = if m == 0 {
        vec![w0_inv]
    } else {
        build_q(coeffs, n, m, p, w_lo, w_hi)?
    };

    Ok(Certified::new(
        ReciprocalPower {
            degree: n,
            m,
            q,
            eps,
            bracket: (w_lo, w_hi),
        },
        certificate(),
    ))
}

/// Builds the degree-`m·n` Bernstein coefficient enclosures of
/// `Q_m = w₀^{-p} Σ_{k≤m} (−1)^k C(k+p−1, p−1) δ^k`, in exact rational
/// arithmetic, projected outward to intervals at the end.
fn build_q(
    coeffs: &[f64],
    n: usize,
    m: usize,
    p: u32,
    w_lo: f64,
    w_hi: f64,
) -> Result<Vec<Interval>, Refusal> {
    let degree = m * n;

    // Exact rational inputs.
    let mut c_rat: Vec<Rat> = Vec::with_capacity(coeffs.len());
    for c in coeffs {
        let r = f64_to_rat(*c).ok_or(Refusal::Empty)?;
        c_rat.push(r);
    }
    let w_lo_rat = f64_to_rat(w_lo).ok_or(Refusal::Empty)?;
    let w_hi_rat = f64_to_rat(w_hi).ok_or(Refusal::Empty)?;
    let w0_rat = w_lo_rat
        .add(w_hi_rat)
        .and_then(|s| s.div(Rat::from_i128(2)));
    let w0_rat = w0_rat.ok_or(Refusal::Empty)?;

    let w_mono = bern_to_mono_rat(&c_rat, n)?;
    // δ = (W − w₀)/w₀ in the monomial basis (the constant slot shifts).
    let mut delta: Vec<Rat> = Vec::with_capacity(n + 1);
    for (j, c) in w_mono.iter().enumerate() {
        let shifted = if j == 0 { c.sub(w0_rat) } else { Some(*c) };
        let d = shifted.and_then(|x| x.div(w0_rat)).ok_or(Refusal::Empty)?;
        delta.push(d);
    }

    let mut acc: Vec<Rat> = vec![Rat::ZERO; degree + 1];
    // powk = δ^k in the monomial basis; acc accumulates the exact series
    // Σ_k (−1)^k C(k+p−1,p−1) δ^k slot-wise (monomial slots are nested).
    let mut powk: Vec<Rat> = vec![Rat::ONE];
    for k in 0..=m {
        let c_k = binom(k as u64 + p as u64 - 1, p as u64 - 1);
        let sign: i128 = if k % 2 == 0 { 1 } else { -1 };
        let factor = match c_k {
            Some(c) => Rat::from_i128(c * sign),
            None => return Err(Refusal::Empty),
        };
        for (slot, pv) in acc.iter_mut().zip(powk.iter()) {
            let term = factor.mul(*pv).ok_or(Refusal::Empty)?;
            *slot = slot.add(term).ok_or(Refusal::Empty)?;
        }
        if k < m {
            powk = mono_mul_rat(&powk, &delta)?;
        }
    }

    let mut q_rat = mono_to_bern_rat(&acc, degree)?;
    // w₀^{-p}, exact.
    let scale = w0_rat.recip().ok_or(Refusal::Empty)?.pow(p as u64);
    let scale = scale.ok_or(Refusal::Empty)?;
    for slot in q_rat.iter_mut() {
        *slot = slot.mul(scale).ok_or(Refusal::Empty)?;
    }

    Ok(q_rat.iter().map(|r| rat_iv(*r)).collect())
}

/// The exact monomial coefficients of a degree-`n` Bernstein polynomial with
/// coefficients `coeffs`: expanding `Σ_i c_i C(n,i) t^i (1−t)^{n−i}`.
fn bern_to_mono_rat(coeffs: &[Rat], n: usize) -> Result<Vec<Rat>, Refusal> {
    let mut out: Vec<Rat> = vec![Rat::ZERO; n + 1];
    for (i, ci) in coeffs.iter().enumerate() {
        let base = match binom(n as u64, i as u64) {
            Some(b) => ci.mul(Rat::from_i128(b)),
            None => return Err(Refusal::Empty),
        };
        let base = base.ok_or(Refusal::Empty)?;
        for l in 0..=(n - i) {
            let j = i + l;
            let coef = match binom((n - i) as u64, l as u64) {
                Some(b) => b,
                None => return Err(Refusal::Empty),
            };
            let term = if l % 2 == 0 {
                base.mul(Rat::from_i128(coef))
            } else {
                base.mul(Rat::from_i128(-coef))
            };
            let term = term.ok_or(Refusal::Empty)?;
            out[j] = out[j].add(term).ok_or(Refusal::Empty)?;
        }
    }
    Ok(out)
}

/// Exact monomial convolution: the coefficients of `a(t)·b(t)`.
fn mono_mul_rat(a: &[Rat], b: &[Rat]) -> Result<Vec<Rat>, Refusal> {
    let mut out: Vec<Rat> = vec![Rat::ZERO; a.len() + b.len() - 1];
    for (i, ai) in a.iter().enumerate() {
        for (j, bj) in b.iter().enumerate() {
            let prod = ai.mul(*bj).ok_or(Refusal::Empty)?;
            out[i + j] = out[i + j].add(prod).ok_or(Refusal::Empty)?;
        }
    }
    Ok(out)
}

/// Converts degree-`degree` monomial coefficients to the Bernstein basis of
/// the same degree, by the exact identity `t^j = Σ_i C(i,j)/C(degree,j) B_i`.
/// `mono` must have length `degree + 1`.
fn mono_to_bern_rat(mono: &[Rat], degree: usize) -> Result<Vec<Rat>, Refusal> {
    let mut out: Vec<Rat> = Vec::with_capacity(degree + 1);
    for i in 0..=degree {
        let mut slot = Rat::ZERO;
        for j in 0..=i {
            let a = mono.get(j).copied().unwrap_or(Rat::ZERO);
            if a.is_zero() {
                continue;
            }
            let num = binom(i as u64, j as u64).ok_or(Refusal::Empty)?;
            let den = binom(degree as u64, j as u64).ok_or(Refusal::Empty)?;
            let ratio = Rat::new(num, den).ok_or(Refusal::Empty)?;
            slot = slot
                .add(a.mul(ratio).ok_or(Refusal::Empty)?)
                .ok_or(Refusal::Empty)?;
        }
        out.push(slot);
    }
    Ok(out)
}

/// `C(n, k)` as an exact `i128`, `None` on overflow. `k = 0` and `k = n` give
/// `1`.
fn binom(n: u64, k: u64) -> Option<i128> {
    if k > n {
        return Some(0);
    }
    let k = k.min(n - k);
    let mut c: i128 = 1;
    for i in 0..k {
        c = c.checked_mul((n - i) as i128)?;
        c /= (i + 1) as i128;
    }
    Some(c)
}

/// `iv^e` for a nonnegative integer `e` by square-and-multiply.
fn pos_pow(iv: Interval, e: u64) -> Interval {
    let mut result = interval_at(1.0);
    let mut base = iv;
    let mut exp = e;
    while exp > 0 {
        if exp & 1 == 1 {
            result *= base;
        }
        exp >>= 1;
        if exp > 0 {
            base *= base;
        }
    }
    result
}

/// A finite interval whose endpoints are finite (`inari` intervals may be
/// unbounded; a certified bound must not be).
fn finite(iv: &Interval) -> bool {
    !iv.is_empty() && iv.inf().is_finite() && iv.sup().is_finite()
}

/// A degenerate interval at a runtime `f64`. A non-finite `x` degrades to the
/// empty interval rather than panicking (H-1); callers validate finiteness
/// before arithmetic.
fn interval_at(x: f64) -> Interval {
    Interval::try_from((x, x)).unwrap_or(Interval::EMPTY)
}

/// The primitive's certificate: outward-rounded interval method, no budget
/// spent (the module is stateless), unbounded margin and modulus.
fn certificate() -> Certificate {
    Certificate {
        props: PropMap::new(),
        method: Method::Interval,
        budget_left: Budget::new(0, 0, 0),
        margin: Margin::UNBOUNDED,
        modulus: Modulus::Unbounded,
    }
}

#[cfg(test)]
// Test-only allow: H-1 bans unwrap/expect on paths reachable from untrusted
// geometry. Unit-test assertions on hand-built witnesses are not such a path.
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn binomial_small_values_are_exact() {
        assert_eq!(binom(5, 0), Some(1));
        assert_eq!(binom(5, 2), Some(10));
        assert_eq!(binom(4, 9), Some(0));
    }

    #[test]
    fn f64_converts_to_its_exact_dyadic_ratio() {
        // 1/256 = 2^{-8}, 8 = 2^3, 3/256 = 3·2^{-8}.
        assert_eq!(f64_to_rat(1.0 / 256.0), Rat::new(1, 256));
        assert_eq!(f64_to_rat(8.0), Some(Rat::from_i128(8)));
        assert_eq!(f64_to_rat(3.0 / 256.0), Rat::new(3, 256));
        assert_eq!(f64_to_rat(-0.5), Rat::new(-1, 2));
        assert_eq!(f64_to_rat(f64::NAN), None);
    }

    #[test]
    fn bern_mono_round_trip_reproduces_values() {
        // W = [1, 3] linear: monomial 1 + 2t.
        let mono = bern_to_mono_rat(&[Rat::ONE, Rat::from_i128(3)], 1).unwrap();
        assert_eq!(mono[0], Rat::ONE);
        assert_eq!(mono[1], Rat::from_i128(2));
        // (1−t)^3 has Bernstein coefficients [1, 0, 0, 0]; its monomial form
        // 1 − 3t + 3t² − t³ converts back exactly.
        let mono4 = vec![
            Rat::ONE,
            Rat::from_i128(-3),
            Rat::from_i128(3),
            Rat::from_i128(-1),
        ];
        let back = mono_to_bern_rat(&mono4, 3).unwrap();
        assert_eq!(back[0], Rat::ONE);
        assert_eq!(back[1], Rat::ZERO);
        assert_eq!(back[3], Rat::ZERO);
    }
}
