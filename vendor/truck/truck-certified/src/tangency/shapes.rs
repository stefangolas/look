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

//! The CTE certificate vocabulary freeze (CTE-000-SPINE).
//!
//! This is a freeze in the P0-FREEZE pattern (`ssi_types.rs` precedent): only
//! the shared shapes land here, with refusing constructors and verbatim
//! accessors. Nothing here evaluates, solves, isolates, or certifies
//! numerically. The seven later packets (CTE-001..005, 007, 008) build against
//! this module and never restate it.
//!
//! **D-shim.** Types and refusing constructors only. Any method that would
//! evaluate, solve, isolate, or certify NUMERICALLY refuses (`InvalidInput`-shaped
//! or a named case from the existing vocabularies). The module doc says
//! verbatim: "This module freezes shapes; the CTE wave packets implement
//! against it and never restate it."
//!
//! **D-reuse.** The crate's landed types are the vocabulary: `CertifiedInterval`
//! (`formal/exact.rs`) for every interval value, `PositiveFinite`
//! (`formal/numeric.rs`) for the R9 modulus, `KrawczykCertificate3`
//! (`ssi_types.rs`) for the A₂ branch seeds, and `contract::Refusal` for every
//! refusing-constructor error (a construction outside a frozen rule is
//! `InvalidInput`). The module wraps/aliases; it never duplicates a landed type
//! under a new name, and it adds no new top-level evidence kind (mapping rows in
//! `docs/CERTIFICATE_MAPPING.md`).
//!
//! **Shape semantics.** Chart-relative certificates (`Rank2Chart`,
//! `GraphEnclosure`, `A2BranchCurve`) are carried relative to the *recorded*
//! pivot of the owning [`Rank2Chart`] — component index `0..3` (which F
//! component is `f`) and an ascending `coord_pair` naming the two `y`
//! coordinates (`A = D_y G`). A curve or box carried beside such a shape uses
//! the same ordering; no consumer re-derives the chart.
//!
//! **Frozen type-level rules** (spine §2): R2 — the reduced-Hessian evaluator's
//! only signature takes `&GraphEnclosure` ([`ReducedHessianEvaluator`]); R1 —
//! no "deflation determinant predicate" type exists anywhere in this module;
//! R9 — `Definite { sign, mu }` refuses `mu <= 0`; R3 — `A1Node` is a first-class
//! verdict arm; R6 — `A2Cert` carries a certified non-empty branch.

use crate::contract::Refusal;
use crate::formal::exact::CertifiedInterval;
use crate::formal::numeric::PositiveFinite;
use crate::ssi_types::KrawczykCertificate3;

/// A finite, ordered axis interval, stored as the shim's raw `(lo, hi)` pair
/// (the `ssi_types` convention). Pure structure; no arithmetic lives here.
fn interval_ok((lo, hi): (f64, f64)) -> bool {
    lo.is_finite() && hi.is_finite() && lo <= hi
}

/// A chart pivot: one of the 18 charts of Lemma T1.0.
///
/// A chart chooses one component of the cross-multiplied difference `F` to be
/// `f` (the other two are `G = (G_1, G_2)`) and two of the four chart
/// coordinates to be `y` (the other two are `z`), so `A = D_y G` is the
/// `2 x 2` pivot block. `component` is the index `0..3` of the `F` component
/// taken as `f`; `coord_pair` is the ascending pair of coordinate indices taken
/// as `y`. Constructed only through [`Rank2Pivot::new`], which refuses a
/// component outside `0..3` or a coordinate pair that is not two distinct
/// ascending indices in `0..4`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rank2Pivot {
    component: usize,
    coord_pair: (usize, usize),
}

impl Rank2Pivot {
    /// Build a chart pivot, refusing a component index outside `0..3` or a
    /// coordinate pair that is not `a < b` with `b < 4` (there are exactly
    /// `3 * C(4, 2) = 18` such pivots).
    pub fn new(component: usize, coord_pair: (usize, usize)) -> Result<Self, Refusal> {
        if component > 2 {
            return Err(Refusal::InvalidInput);
        }
        let (a, b) = coord_pair;
        if a >= b || b > 3 {
            return Err(Refusal::InvalidInput);
        }
        Ok(Self {
            component,
            coord_pair,
        })
    }

    /// The index `0..3` of the `F` component taken as `f`.
    pub fn component(&self) -> usize {
        self.component
    }

    /// The ascending pair of coordinate indices taken as `y` (so that
    /// `A = D_y G`).
    pub fn coord_pair(&self) -> (usize, usize) {
        self.coord_pair
    }
}

/// One chart-minor Bernstein net `M_j`: the coefficient table of the
/// polynomial `det D_(y1,y2,z_j)(G_1, G_2, f)` over the four chart axes.
///
/// `degrees` are the per-axis degrees in the chart's axis order and `coeffs`
/// is the flat tensor-Bernstein coefficient table of length
/// `(degrees[0] + 1) * (degrees[1] + 1) * (degrees[2] + 1) * (degrees[3] + 1)`,
/// laid out axis-major. This is a certificate *carrier* (D-shim): no arithmetic
/// happens here, and the table values arrive from CTE-003's minor-grid
/// construction. Constructed only through [`ChartMinorNet::new`].
#[derive(Debug, Clone, PartialEq)]
pub struct ChartMinorNet {
    degrees: [usize; 4],
    coeffs: Vec<f64>,
}

impl ChartMinorNet {
    /// Build a minor net, refusing a non-finite coefficient or a coefficient
    /// table whose length does not equal the shape the degrees demand.
    pub fn new(degrees: [usize; 4], coeffs: Vec<f64>) -> Result<Self, Refusal> {
        let expected = degrees.iter().map(|d| d + 1).product::<usize>();
        if coeffs.len() != expected || coeffs.iter().any(|c| !c.is_finite()) {
            return Err(Refusal::InvalidInput);
        }
        Ok(Self { degrees, coeffs })
    }

    /// The per-axis degrees, verbatim.
    pub fn degrees(&self) -> [usize; 4] {
        self.degrees
    }

    /// The flat coefficient table, verbatim.
    pub fn coeffs(&self) -> &[f64] {
        &self.coeffs
    }
}

/// The two chart-minor nets `(M_1, M_2)` of a chart (theory T1.1).
///
/// A [`Rank2Chart`] carries exactly one of these, in the chart's recorded axis
/// order. Constructed through [`ChartMinorGrids::new`] from two already-valid
/// nets.
#[derive(Debug, Clone, PartialEq)]
pub struct ChartMinorGrids {
    m1: ChartMinorNet,
    m2: ChartMinorNet,
}

impl ChartMinorGrids {
    /// Package the two chart-minor nets. The nets are already validated by
    /// [`ChartMinorNet::new`]; the packaging refuses nothing further.
    pub fn new(m1: ChartMinorNet, m2: ChartMinorNet) -> Result<Self, Refusal> {
        Ok(Self { m1, m2 })
    }

    /// The first chart minor `M_1`, verbatim.
    pub fn m1(&self) -> &ChartMinorNet {
        &self.m1
    }

    /// The second chart minor `M_2`, verbatim.
    pub fn m2(&self) -> &ChartMinorNet {
        &self.m2
    }
}

/// A certified rank-2 chart of the difference system (theory §2.1, §3).
///
/// Carries the pivot, the `2 x 2` determinant enclosure `det_a` with `0`
/// STRICTLY excluded by construction, and the chart-minor grids. The refusing
/// constructor enforces the frozen certificate precondition
/// `0 ∉ det_a` — an enclosure containing zero (inclusive of a `0` endpoint) is
/// a box no downstream certificate may be relative to. Constructed only through
/// [`Rank2Chart::new`].
#[derive(Debug, Clone, PartialEq)]
pub struct Rank2Chart {
    pivot: Rank2Pivot,
    det_a: CertifiedInterval,
    minors: ChartMinorGrids,
}

impl Rank2Chart {
    /// Build a certified rank-2 chart.
    ///
    /// Refuses ([`Refusal::InvalidInput`]) a non-finite or misordered
    /// (`lo > hi`) `det_a`, or one containing `0` (inclusive of a `0` endpoint —
    /// the exclusion is STRICT, mirroring `KrawczykCertificate3`'s determinant
    /// rule). The pivot and minors arrive as already-validated values.
    pub fn new(
        pivot: Rank2Pivot,
        det_a: CertifiedInterval,
        minors: ChartMinorGrids,
    ) -> Result<Self, Refusal> {
        if !det_a.is_finite() || det_a.lo > det_a.hi {
            return Err(Refusal::InvalidInput);
        }
        if det_a.lo <= 0.0 && 0.0 <= det_a.hi {
            return Err(Refusal::InvalidInput);
        }
        Ok(Self {
            pivot,
            det_a,
            minors,
        })
    }

    /// The recorded pivot, verbatim.
    pub fn pivot(&self) -> Rank2Pivot {
        self.pivot
    }

    /// The `det A` enclosure over the box, with `0` strictly excluded by
    /// construction.
    pub fn det_a(&self) -> CertifiedInterval {
        self.det_a
    }

    /// The chart-minor grids, verbatim.
    pub fn minors(&self) -> &ChartMinorGrids {
        &self.minors
    }
}

/// The certified (H-graph) content a [`GraphEnclosure`] rides on: the working
/// box `B` and the strict-inclusion image the parametric Newton/Krawczyk step
/// certified over it.
///
/// Hypothesis (H-graph) (theory §2.2): on `B = Y x Z`, `0 ∉ det A(B)` and a
/// parametric interval Newton/Krawczyk certifies, for every `z ∈ Z`, a unique
/// `y = φ(z) ∈ Y`. The certificate's essential numeric content is the strict
/// inclusion — the image box is component-wise STRICTLY inside the working box
/// on every one of the four axes, the same strict-inclusion rule as
/// `KrawczykCertificate3`. This is a carrier produced by CTE-002's (H-graph)
/// machinery; nothing here produces or verifies it. Constructed only through
/// [`GraphCert::new`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GraphCert {
    /// The four-axis working box `B = Y x Z` in chart axis order.
    work_box: [(f64, f64); 4],
    /// The certified image box, component-wise strictly inside `work_box`.
    image: [(f64, f64); 4],
}

impl GraphCert {
    /// Build the certified graph content from a strict inclusion.
    ///
    /// Refuses ([`Refusal::InvalidInput`]) a non-finite or misordered interval,
    /// or an image box that is not component-wise STRICTLY inside the working
    /// box on every axis (a boundary-touching or reversed image is a box the
    /// parametric Newton step may not certify).
    pub fn new(work_box: [(f64, f64); 4], image: [(f64, f64); 4]) -> Result<Self, Refusal> {
        for ((w_lo, w_hi), (i_lo, i_hi)) in work_box.iter().zip(image.iter()) {
            let (w_lo, w_hi, i_lo, i_hi) = (*w_lo, *w_hi, *i_lo, *i_hi);
            if !interval_ok((w_lo, w_hi)) || !interval_ok((i_lo, i_hi)) {
                return Err(Refusal::InvalidInput);
            }
            let strictly_inside = w_lo < i_lo && i_hi < w_hi;
            if !strictly_inside {
                return Err(Refusal::InvalidInput);
            }
        }
        Ok(Self { work_box, image })
    }

    /// The four-axis working box `B`, verbatim.
    pub fn work_box(&self) -> [(f64, f64); 4] {
        self.work_box
    }

    /// The certified strict-inclusion image box, verbatim.
    pub fn image(&self) -> [(f64, f64); 4] {
        self.image
    }
}

/// The certified graph enclosure `Ŷ ⊇ φ(Z)` (theory §2.2; spine §2).
///
/// Carries the enclosure `y_hat` of the certified graph over the `z`-domain
/// `z`, together with the (H-graph) certificate content. Produced only by
/// CTE-002's graph machinery and consumed by the reduced-Hessian evaluator
/// (R2: [`ReducedHessianEvaluator::reduced_hessian`] accepts ONLY this type —
/// no raw-box overload exists). Constructed only through
/// [`GraphEnclosure::new`].
#[derive(Debug, Clone, PartialEq)]
pub struct GraphEnclosure {
    /// The certified `Ŷ ⊇ φ(Z)` enclosure, two axis intervals in the `y`
    /// coordinates.
    y_hat: [(f64, f64); 2],
    /// The `z`-domain `Z`, two axis intervals in the `z` coordinates.
    z: [(f64, f64); 2],
    /// The certified (H-graph) content behind the enclosure.
    evidence: GraphCert,
}

impl GraphEnclosure {
    /// Build a graph enclosure from the two interval pairs and the certified
    /// (H-graph) content.
    ///
    /// Refuses ([`Refusal::InvalidInput`]) a non-finite or misordered interval
    /// in `y_hat` or `z`. The enclosure relation `Ŷ ⊇ φ(Z)` is certified by
    /// `evidence`; nothing here evaluates it.
    pub fn new(
        y_hat: [(f64, f64); 2],
        z: [(f64, f64); 2],
        evidence: GraphCert,
    ) -> Result<Self, Refusal> {
        if !y_hat.iter().all(|i| interval_ok(*i)) || !z.iter().all(|i| interval_ok(*i)) {
            return Err(Refusal::InvalidInput);
        }
        Ok(Self { y_hat, z, evidence })
    }

    /// The certified `Ŷ ⊇ φ(Z)` enclosure, verbatim.
    pub fn y_hat(&self) -> [(f64, f64); 2] {
        self.y_hat
    }

    /// The `z`-domain `Z`, verbatim.
    pub fn z(&self) -> [(f64, f64); 2] {
        self.z
    }

    /// The certified (H-graph) content, verbatim.
    pub fn evidence(&self) -> GraphCert {
        self.evidence
    }
}

/// A certified `2 x 2` symmetric interval matrix `[[a, b], [b, c]]` (theory
/// §2.8): the reduced Hessian's interval value the definiteness test consumes.
///
/// This is a certificate *carrier* — no arithmetic lives here (D-shim); the
/// interval values arrive from CTE-004's T1.3 evaluation over `Ŷ x Z`.
/// Constructed only through [`IntervalSym2::new`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IntervalSym2 {
    /// The `(1,1)` entry `a`.
    a: CertifiedInterval,
    /// The shared off-diagonal entry `b`.
    b: CertifiedInterval,
    /// The `(2,2)` entry `c`.
    c: CertifiedInterval,
}

impl IntervalSym2 {
    /// Build a symmetric interval matrix.
    ///
    /// Refuses ([`Refusal::InvalidInput`]) a non-finite or misordered
    /// (`lo > hi`) entry interval.
    pub fn new(
        a: CertifiedInterval,
        b: CertifiedInterval,
        c: CertifiedInterval,
    ) -> Result<Self, Refusal> {
        let valid = [a, b, c].iter().all(|i| i.is_finite() && i.lo <= i.hi);
        if !valid {
            return Err(Refusal::InvalidInput);
        }
        Ok(Self { a, b, c })
    }

    /// The `(1,1)` entry `a`, verbatim.
    pub fn a(&self) -> CertifiedInterval {
        self.a
    }

    /// The shared off-diagonal entry `b`, verbatim.
    pub fn b(&self) -> CertifiedInterval {
        self.b
    }

    /// The `(2,2)` entry `c`, verbatim.
    pub fn c(&self) -> CertifiedInterval {
        self.c
    }
}

/// The sign of a definite reduced Hessian (theory §2.8, R9).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefinitenessSign {
    /// Positive definite (`H_h ⪰ μI`).
    Positive,
    /// Negative definite (`H_h ⪯ −μI`).
    Negative,
}

/// The definiteness verdict of the reduced Hessian, carrying the certified
/// modulus the Taylor isolation bound consumes (theory §2.8, correction R9).
///
/// - [`Definiteness::Definite`] carries the sign and the certified modulus
///   `μ > 0`; construction refuses `mu <= 0` (a PD verdict without the μ the
///   Taylor bound consumes is not a usable certificate).
/// - [`Definiteness::Indefinite`] carries the certified strictly-negative upper
///   bound of `det H_h` (the A₁⁻ predicate `det H_h < 0` of theory §2.9c);
///   construction refuses a non-negative or non-finite bound.
/// - [`Definiteness::Singular`] is the degenerate case (A₂ or higher — the
///   verdict routes to the A₂ machinery, never to an A₁ arm).
///
/// The `mu` is stored as the landed [`PositiveFinite`], so a non-positive
/// modulus is unrepresentable even by direct variant construction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Definiteness {
    /// The reduced Hessian is certified definite with modulus `mu`.
    Definite {
        /// The sign of definiteness.
        sign: DefinitenessSign,
        /// The certified modulus `μ > 0` (R9).
        mu: PositiveFinite,
    },
    /// The reduced Hessian is certified indefinite: `det H_h < 0` with the
    /// certified upper bound `det_upper < 0`.
    Indefinite {
        /// The certified strictly-negative upper bound of `det H_h`.
        det_upper: f64,
    },
    /// The reduced Hessian is certified singular (degenerate contact).
    Singular,
}

impl Definiteness {
    /// Build a `Definite` verdict from a raw modulus.
    ///
    /// Refuses ([`Refusal::InvalidInput`]) a non-finite or non-positive `mu`
    /// (the R9 rule: `Definite` with `mu <= 0` is a construction outside the
    /// frozen rule).
    pub fn definite(sign: DefinitenessSign, mu: f64) -> Result<Self, Refusal> {
        let mu = PositiveFinite::new(mu).map_err(|_| Refusal::InvalidInput)?;
        Ok(Definiteness::Definite { sign, mu })
    }

    /// Build an `Indefinite` verdict from the certified upper bound of
    /// `det H_h`.
    ///
    /// Refuses ([`Refusal::InvalidInput`]) a non-finite or non-negative
    /// `det_upper` — certified indefiniteness requires the bound to be
    /// STRICTLY negative.
    pub fn indefinite(det_upper: f64) -> Result<Self, Refusal> {
        if !det_upper.is_finite() || det_upper >= 0.0 {
            return Err(Refusal::InvalidInput);
        }
        Ok(Definiteness::Indefinite { det_upper })
    }

    /// The `Singular` verdict.
    pub fn singular() -> Self {
        Definiteness::Singular
    }
}

/// The instantiation tag of an [`ExactVanishingWitness`] (theory §4.2): which
/// of the two uses the identity `s·f = q²a + Σ wᵢPᵢ` is for.
///
/// The tag selects nothing at verification time (ONE code path); it documents
/// the identity's structural shape and guards the refusing constructor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WitnessSide {
    /// The A₁ instantiation: `P = (G₁, G₂, M₁, M₂)` (four terms), no `q²a`
    /// term. Ideal membership of `f` in the saturation of `⟨G₁,G₂,M₁,M₂⟩`.
    A1,
    /// The A₂ instantiation: `P = (G₁, G₂)` (two terms) with the `q²a` term
    /// (T1.7's contact-factor identity).
    A2,
}

/// An exact-vanishing witness: the data of the exact identity
///
/// ```text
/// s·f = q²a + Σᵢ wᵢ·Pᵢ          (theory §4.2, exact over ℚ)
/// ```
///
/// together with its [`WitnessSide`] tag. `P` is the polynomial representation
/// of the ℚ-polynomial substrate (CTE-001's `QPoly`); the witness is generic
/// over it so this freeze does not depend on a substrate that lands in a later
/// packet (the `WitnessEdge<S, C>` precedent).
///
/// The refusing constructor enforces the structural shape of the two
/// instantiations — an A₁ witness must carry exactly the four terms
/// `(G₁, G₂, M₁, M₂)` and no `q²a`; an A₂ witness must carry exactly the two
/// terms `(G₁, G₂)` and the `q²a` factor. A witness whose side and term shapes
/// disagree is an incomplete identity and refuses construction.
///
/// Verification — expand both sides over ℚ and compare coefficients, plus the
/// interval nonvanishing predicates — is the ONE verifier ([`ExactWitnessVerifier`]),
/// frozen here as a trait signature and implemented by CTE-001. Producers never
/// verify; the verifier never produces.
#[derive(Debug, Clone, PartialEq)]
pub struct ExactVanishingWitness<P> {
    side: WitnessSide,
    /// The identity's ℚ-polynomial multiplier `s` (`0 ∉ ŝ(B)` certified
    /// separately by the verifier's caller).
    s: P,
    /// The correction terms `(wᵢ, Pᵢ)`.
    terms: Vec<(P, P)>,
    /// The optional quadratic-factor term `(q, a)`, present iff `side` is A₂.
    q2a: Option<(P, P)>,
}

impl<P> ExactVanishingWitness<P> {
    /// The number of correction terms the given side's instantiation demands:
    /// four for A₁ (`P = (G₁,G₂,M₁,M₂)`), two for A₂ (`P = (G₁,G₂)`).
    fn term_count(side: WitnessSide) -> usize {
        match side {
            WitnessSide::A1 => 4,
            WitnessSide::A2 => 2,
        }
    }

    /// Build a witness, refusing an identity whose side and term shapes
    /// disagree ([`Refusal::InvalidInput`]).
    ///
    /// Refuses: an A₁ witness carrying a `q²a` term; an A₂ witness missing the
    /// `q²a` term; and either side whose term count differs from its
    /// instantiation's (`4` for A₁, `2` for A₂) — such a witness is incomplete.
    pub fn new(
        side: WitnessSide,
        s: P,
        terms: Vec<(P, P)>,
        q2a: Option<(P, P)>,
    ) -> Result<Self, Refusal> {
        let expected = Self::term_count(side);
        let shape_ok = match side {
            WitnessSide::A1 => q2a.is_none(),
            WitnessSide::A2 => q2a.is_some(),
        };
        if !shape_ok || terms.len() != expected {
            return Err(Refusal::InvalidInput);
        }
        Ok(Self {
            side,
            s,
            terms,
            q2a,
        })
    }

    /// The instantiation tag.
    pub fn side(&self) -> WitnessSide {
        self.side
    }

    /// The multiplier polynomial `s`.
    pub fn s(&self) -> &P {
        &self.s
    }

    /// The correction terms `(wᵢ, Pᵢ)`, verbatim.
    pub fn terms(&self) -> &[(P, P)] {
        &self.terms
    }

    /// The optional quadratic-factor term `(q, a)`.
    pub fn q2a(&self) -> Option<&(P, P)> {
        self.q2a.as_ref()
    }
}

/// The frozen R2 signature (theory §2.5, evaluation-rule correction): the
/// reduced-Hessian evaluator.
///
/// The T1.3 reduced Hessian `H_h = Jᵀ(H_f − λ₁H_G₁ − λ₂H_G₂)J` must be
/// evaluated over the certified graph enclosure `Ŷ x Z` — never over a raw
/// box. This trait is the ONLY signature that exists for requesting the
/// reduced Hessian: **no raw-box overload exists**. The bodies are provided by
/// CTE-004 (`tangency/hessian.rs`); this packet ships the frozen signature and
/// no implementor (the `CertifiedPatch` D-shim precedent, `kernel/patch.rs`).
///
/// Any method that would evaluate NUMERICALLY refuses here (D-shim); the
/// trait's implementors carry the certified evaluation.
pub trait ReducedHessianEvaluator {
    /// The T1.3 reduced Hessian over the certified graph enclosure of `chart`.
    fn reduced_hessian(chart: &Rank2Chart, over: &GraphEnclosure) -> Result<IntervalSym2, Refusal>;
}

/// The frozen one-verifier contract (theory §4.2; scope decision 5): the exact
/// coefficient-compare verifier both instantiations share.
///
/// `Self` is the witness data ([`ExactVanishingWitness`]); `P` is the ℚ
/// polynomial representation. Verification expands
/// `s·f − (q²a + Σᵢ wᵢPᵢ)` over ℚ and compares coefficients to zero exactly,
/// then enforces the interval nonvanishing predicates `0 ∉ ŝ(B)` and (when `q`
/// is present) `0 ∉ â(B)` against the caller-supplied enclosures. ONE code
/// path — the [`WitnessSide`] tag selects nothing except documentation.
///
/// Implemented by CTE-001 (`tangency/witness.rs`) over the ℚ-polynomial
/// substrate. Producers never verify; the verifier never produces.
pub trait ExactWitnessVerifier<P> {
    /// Verify the exact identity against the system polynomials `f`, `P`
    /// and the caller-supplied nonvanishing enclosures. `s_encl` is `ŝ(B)`;
    /// `a_encl` is `â(B)`, present iff the witness carries the `q²a` term.
    /// Both, when present, must exclude `0` STRICTLY.
    fn verify(
        &self,
        f: &P,
        p: &[&P],
        s_encl: Option<(f64, f64)>,
        a_encl: Option<(f64, f64)>,
    ) -> Result<(), Refusal>;
}

/// The certificate of a certified unique simple root of the deflated system
/// `T = (G₁, G₂, M₁, M₂)` on a box (theory §2.6, §2.10; theory §3's
/// "KrawczykEvidence").
///
/// Carries the root enclosure `X*` (four chart axes), the certified
/// conditioning ratio `ρ = ‖I − C·DT(B)‖ < 1`, and the certified residual norm
/// `η = ‖C·T(c)‖ ≥ 0` (enclosure radius `r ≤ η/(1−ρ)`). The conditioning
/// numbers arrive from CTE-003's Krawczyk work on `T`; nothing here computes
/// them (D-shim). Constructed only through [`DeflatedRootCert::new`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DeflatedRootCert {
    /// The root enclosure `X*`, four chart axes in chart axis order.
    root_box: [(f64, f64); 4],
    /// The certified contraction ratio `ρ ∈ [0, 1)`.
    rho: f64,
    /// The certified residual norm `η ≥ 0`.
    eta: f64,
}

impl DeflatedRootCert {
    /// Build the deflated-root certificate.
    ///
    /// Refuses ([`Refusal::InvalidInput`]) a non-finite or misordered root box,
    /// a `rho` outside `[0, 1)` (a certified contraction ratio is strictly less
    /// than `1`), or a negative or non-finite `eta`.
    pub fn new(root_box: [(f64, f64); 4], rho: f64, eta: f64) -> Result<Self, Refusal> {
        if !root_box.iter().all(|i| interval_ok(*i)) {
            return Err(Refusal::InvalidInput);
        }
        if !rho.is_finite() || !(0.0..1.0).contains(&rho) {
            return Err(Refusal::InvalidInput);
        }
        if !eta.is_finite() || eta < 0.0 {
            return Err(Refusal::InvalidInput);
        }
        Ok(Self { root_box, rho, eta })
    }

    /// The root enclosure `X*`, verbatim.
    pub fn root_box(&self) -> [(f64, f64); 4] {
        self.root_box
    }

    /// The certified contraction ratio `ρ`.
    pub fn rho(&self) -> f64 {
        self.rho
    }

    /// The certified residual norm `η`.
    pub fn eta(&self) -> f64 {
        self.eta
    }
}

/// The certificate behind a [`ContactVerdict::Transversal`] (theory §2.7,
/// T1.5 / T1.6a).
///
/// Two proof shapes exist, and neither needs a Hessian:
///
/// - `TransversalEvidence::NoCriticalPoint` — the five-equation system
///   `(F, M₁, M₂)` has no root in the box (T1.5.1), strictly sharper than
///   minor separation.
/// - `TransversalEvidence::UniqueDeflatedRootNonzeroCriticalValue` — Krawczyk
///   certifies a unique root `x*` of `T` in the box and `0 ∉ f̂(X*)` (T1.5.2).
///
/// Constructed only through [`TransversalCert::new`] /
/// [`TransversalCert::unique_root`].
#[derive(Debug, Clone, PartialEq)]
pub struct TransversalCert {
    evidence: TransversalEvidence,
}

/// Which proof certified the transversality (theory §2.7).
#[derive(Debug, Clone, PartialEq)]
pub enum TransversalEvidence {
    /// T1.5.1: the five-equation system `(F, M₁, M₂)` has no root in the box.
    NoCriticalPoint,
    /// T1.5.2: a unique deflated root whose critical value is certified
    /// nonzero (`0 ∉ f̂(X*)`).
    UniqueDeflatedRootNonzeroCriticalValue {
        /// The certified unique root of `T`.
        root: DeflatedRootCert,
        /// The enclosure of the critical value `f̂(X*)`, with `0` STRICTLY
        /// excluded.
        critical_value: CertifiedInterval,
    },
}

impl TransversalCert {
    /// The T1.5.1 evidence: five-equation exclusion. Infallible.
    pub fn no_critical_point() -> Self {
        Self {
            evidence: TransversalEvidence::NoCriticalPoint,
        }
    }

    /// The T1.5.2 evidence: a unique deflated root with a certified nonzero
    /// critical value.
    ///
    /// Refuses ([`Refusal::InvalidInput`]) a non-finite/misordered
    /// `critical_value`, or one containing `0` (inclusive of a `0` endpoint) —
    /// the whole point of the route is that the critical value is separated
    /// from zero.
    pub fn unique_root(
        root: DeflatedRootCert,
        critical_value: CertifiedInterval,
    ) -> Result<Self, Refusal> {
        if !critical_value.is_finite() || critical_value.lo > critical_value.hi {
            return Err(Refusal::InvalidInput);
        }
        if critical_value.lo <= 0.0 && 0.0 <= critical_value.hi {
            return Err(Refusal::InvalidInput);
        }
        Ok(Self {
            evidence: TransversalEvidence::UniqueDeflatedRootNonzeroCriticalValue {
                root,
                critical_value,
            },
        })
    }

    /// Which proof certified the transversality, verbatim.
    pub fn evidence(&self) -> &TransversalEvidence {
        &self.evidence
    }
}

/// The certificate behind [`ContactVerdict::A1Isolated`] and
/// [`ContactVerdict::A1Node`] (theory §2.9, §3).
///
/// An A₁ certificate attaches to the SIMPLE root of the deflated system `T`
/// (T1.4 — never to the double root of `F`): the certified chart, the
/// certified deflated root, the certified definiteness/indefiniteness of the
/// reduced Hessian, and the REQUIRED exact-zero witness of §4. The
/// isolated-vs-node distinction is carried by the [`ContactVerdict`] arm, not
/// here. Constructed only through [`A1Cert::new`].
#[derive(Debug, Clone, PartialEq)]
pub struct A1Cert<P> {
    chart: Rank2Chart,
    root: DeflatedRootCert,
    hessian: Definiteness,
    zero_witness: ExactVanishingWitness<P>,
}

impl<P> A1Cert<P> {
    /// Build an A₁ certificate.
    ///
    /// Refuses ([`Refusal::InvalidInput`]) a certificate whose witness is not
    /// an A₁ instantiation (the A₁ exact-zero witness has no `q²a` term), or
    /// whose hessian verdict is [`Definiteness::Singular`] (an A₁ contact point
    /// requires a nondegenerate contact Hessian — a degenerate verdict is A₂ or
    /// higher and routes elsewhere).
    pub fn new(
        chart: Rank2Chart,
        root: DeflatedRootCert,
        hessian: Definiteness,
        zero_witness: ExactVanishingWitness<P>,
    ) -> Result<Self, Refusal> {
        if zero_witness.side() != WitnessSide::A1 {
            return Err(Refusal::InvalidInput);
        }
        if matches!(hessian, Definiteness::Singular) {
            return Err(Refusal::InvalidInput);
        }
        Ok(Self {
            chart,
            root,
            hessian,
            zero_witness,
        })
    }

    /// The certified chart, verbatim.
    pub fn chart(&self) -> &Rank2Chart {
        &self.chart
    }

    /// The certified deflated root of `T`, verbatim.
    pub fn root(&self) -> DeflatedRootCert {
        self.root
    }

    /// The certified definiteness/indefiniteness verdict of `H_h`, verbatim.
    pub fn hessian(&self) -> Definiteness {
        self.hessian
    }

    /// The REQUIRED exact-zero witness (§4), verbatim.
    pub fn zero_witness(&self) -> &ExactVanishingWitness<P> {
        &self.zero_witness
    }
}

/// The certificate behind [`ContactVerdict::A2Branch`] (theory §2.10, T1.7 /
/// R6).
///
/// Carries the verified contact-factor identity (an A₂-instantiated
/// [`ExactVanishingWitness`]), the interval nonvanishing multipliers `ŝ`, `â`
/// (each excluding `0` STRICTLY), the rank-3 separation of
/// `D(G₁, G₂, q)`, and the REQUIRED certified non-empty branch evidence — the
/// R6 correction: T1.7 is also satisfied vacuously when `{G = q = 0} ∩ B = ∅`,
/// so an A₂ verdict must certify a non-empty piece. Constructed only through
/// [`A2Cert::new`].
#[derive(Debug, Clone, PartialEq)]
pub struct A2Cert<P> {
    identity: ExactVanishingWitness<P>,
    s_encl: CertifiedInterval,
    a_encl: CertifiedInterval,
    rank3: RankThreeMinorCert,
    nonempty: A2NonemptyEvidence,
}

/// The rank-3 separation certificate for `D(G₁, G₂, q)` on the box (theory
/// T1.7 hypothesis): one `3 x 3` minor of the `3 x 4` Jacobian separated from
/// zero.
///
/// `coordinate_triple` names the three of the four columns whose minor is
/// certified away from zero; `minor_enclosure` is its determinant enclosure.
/// Constructed only through [`RankThreeMinorCert::new`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RankThreeMinorCert {
    /// The ascending triple of columns whose `3 x 3` minor is separated.
    coordinate_triple: (usize, usize, usize),
    /// The minor's determinant enclosure, with `0` STRICTLY excluded.
    minor_enclosure: CertifiedInterval,
}

impl RankThreeMinorCert {
    /// Build the rank-3 separation certificate.
    ///
    /// Refuses ([`Refusal::InvalidInput`]) a column triple that is not three
    /// distinct ascending indices in `0..4`, a non-finite/misordered minor
    /// enclosure, or one containing `0` (the separation must be STRICT).
    pub fn new(
        coordinate_triple: (usize, usize, usize),
        minor_enclosure: CertifiedInterval,
    ) -> Result<Self, Refusal> {
        let (a, b, c) = coordinate_triple;
        if a >= b || b >= c || c > 3 {
            return Err(Refusal::InvalidInput);
        }
        if !minor_enclosure.is_finite() || minor_enclosure.lo > minor_enclosure.hi {
            return Err(Refusal::InvalidInput);
        }
        if minor_enclosure.lo <= 0.0 && 0.0 <= minor_enclosure.hi {
            return Err(Refusal::InvalidInput);
        }
        Ok(Self {
            coordinate_triple,
            minor_enclosure,
        })
    }

    /// The ascending column triple, verbatim.
    pub fn coordinate_triple(&self) -> (usize, usize, usize) {
        self.coordinate_triple
    }

    /// The certified minor determinant enclosure, verbatim.
    pub fn minor_enclosure(&self) -> CertifiedInterval {
        self.minor_enclosure
    }
}

/// The certified non-empty piece an A₂ verdict must carry (theory §2.10,
/// R6): a Krawczyk-verified point on `{G₁ = G₂ = q = 0}` inside the box, or a
/// certified crossing of the branch through `∂B`.
///
/// Both carriers reuse the landed three-axis [`KrawczykCertificate3`] (the
/// seed format of the traced-branch precedent) for the certified slice solve.
#[derive(Debug, Clone, PartialEq)]
pub enum A2NonemptyEvidence {
    /// A Krawczyk-verified point of `{G₁ = G₂ = q = 0}` inside the box.
    BranchSeed(KrawczykCertificate3),
    /// A certified crossing of the branch through the box boundary `∂B`.
    BoundaryCrossing(KrawczykCertificate3),
}

impl<P> A2Cert<P> {
    /// Build an A₂ certificate.
    ///
    /// Refuses ([`Refusal::InvalidInput`]):
    /// - an identity that is not an A₂ instantiation (must carry the `q²a`
    ///   term);
    /// - a `s_encl`/`a_encl` that is non-finite/misordered or contains `0`
    ///   (the multipliers must be STRICTLY nonvanishing on the box).
    ///
    /// `rank3` and `nonempty` arrive as already-validated certificates.
    pub fn new(
        identity: ExactVanishingWitness<P>,
        s_encl: CertifiedInterval,
        a_encl: CertifiedInterval,
        rank3: RankThreeMinorCert,
        nonempty: A2NonemptyEvidence,
    ) -> Result<Self, Refusal> {
        if identity.side() != WitnessSide::A2 {
            return Err(Refusal::InvalidInput);
        }
        for interval in [s_encl, a_encl] {
            if !interval.is_finite() || interval.lo > interval.hi {
                return Err(Refusal::InvalidInput);
            }
            if interval.lo <= 0.0 && 0.0 <= interval.hi {
                return Err(Refusal::InvalidInput);
            }
        }
        Ok(Self {
            identity,
            s_encl,
            a_encl,
            rank3,
            nonempty,
        })
    }

    /// The verified contact-factor identity, verbatim.
    pub fn identity(&self) -> &ExactVanishingWitness<P> {
        &self.identity
    }

    /// The certified `ŝ(B)` enclosure (0 excluded).
    pub fn s_encl(&self) -> CertifiedInterval {
        self.s_encl
    }

    /// The certified `â(B)` enclosure (0 excluded).
    pub fn a_encl(&self) -> CertifiedInterval {
        self.a_encl
    }

    /// The rank-3 separation certificate, verbatim.
    pub fn rank3(&self) -> RankThreeMinorCert {
        self.rank3
    }

    /// The certified non-empty branch evidence (R6), verbatim.
    pub fn nonempty(&self) -> &A2NonemptyEvidence {
        &self.nonempty
    }
}

/// The five-way contact-classifier cascade verdict (theory §2.11; scope
/// decision 7).
///
/// `Transversal` and `Empty` are the regular outcomes; `A1Isolated` (A₁⁺,
/// definite — isolated contact) and `A1Node` (A₁⁻, indefinite — tangential
/// node) carry [`A1Cert`]; `A2Branch` (Morse–Bott contact along a branch)
/// carries [`A2Cert`]; `Unresolved` is the typed first-class residual carrying
/// the conditioning witness `kappa` and the 4-D cell that resisted
/// certification. [`ContactVerdict::A1Node`] is FIRST-CLASS from day one
/// (theory R3) — it is not a TODO arm. Deterministic by construction: identical
/// ordered input yields identical verdicts.
#[derive(Debug, Clone, PartialEq)]
pub enum ContactVerdict<P> {
    /// The box is transversal (rank 3 at every point of `F⁻¹(0) ∩ B`).
    Transversal(TransversalCert),
    /// The box contains no contact (`F⁻¹(0) ∩ B = ∅`, certified).
    Empty,
    /// A₁⁺: a certified isolated tangential contact point (T1.6b).
    A1Isolated(A1Cert<P>),
    /// A₁⁻: a certified tangential node — two branches crossing at the point
    /// (T1.6c, theory R3).
    A1Node(A1Cert<P>),
    /// A₂: certified Morse–Bott contact along a regular branch (T1.7).
    A2Branch(A2Cert<P>),
    /// The typed residual: certification failed on this cell with conditioning
    /// witness `kappa` (never a guess, never a panic).
    Unresolved {
        /// The conditioning witness (e.g. the frame/operator conditioning at
        /// refusal, mirroring the BIE engine's `kappa` witness).
        kappa: f64,
        /// The 4-D chart cell that resisted certification.
        cell: [(f64, f64); 4],
    },
}

impl<P> ContactVerdict<P> {
    /// Build an `Unresolved` verdict from a conditioning witness and a cell.
    ///
    /// Refuses ([`Refusal::InvalidInput`]) a non-finite `kappa` or a
    /// non-finite/misordered `cell` interval.
    pub fn unresolved(kappa: f64, cell: [(f64, f64); 4]) -> Result<Self, Refusal> {
        if !kappa.is_finite() || !cell.iter().all(|i| interval_ok(*i)) {
            return Err(Refusal::InvalidInput);
        }
        Ok(ContactVerdict::Unresolved { kappa, cell })
    }
}

/// A sample on an [`A2BranchCurve`]: the certified 4-D chart point of the
/// branch together with its certified continuation frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct A2BranchSample {
    /// The chart point in the four-axis chart ordering of the owning record.
    point: [f64; 4],
    /// The certified unit tangent direction of the branch at the point.
    tangent: [f64; 4],
}

impl A2BranchSample {
    /// Build a branch sample.
    ///
    /// Refuses ([`Refusal::InvalidInput`]) a non-finite `point` or a
    /// non-finite `tangent` (the frame's unit length and certification are the
    /// producer CTE-005's contract, not this freeze's).
    pub fn new(point: [f64; 4], tangent: [f64; 4]) -> Result<Self, Refusal> {
        if !point.iter().all(|c| c.is_finite()) || !tangent.iter().all(|c| c.is_finite()) {
            return Err(Refusal::InvalidInput);
        }
        Ok(Self { point, tangent })
    }

    /// The chart point, verbatim.
    pub fn point(&self) -> [f64; 4] {
        self.point
    }

    /// The certified tangent direction, verbatim.
    pub fn tangent(&self) -> [f64; 4] {
        self.tangent
    }
}

/// The named pending refusal under which [`A2BranchCurve`] ships
/// (CTE-000-SPINE scope decision 6): producing an A₂ branch curve is CTE-005's
/// work, so the producing stub refuses until CTE-005 lands (the
/// `cone_torus_carrier_packet_pending` precedent, `kernel/rational.rs`).
pub const A2_BRANCH_PACKET_PENDING: &str = "a2_branch_packet_pending";

/// An A₂ (Morse–Bott) branch curve: the T1 output and the T2 strata input
/// (theory §2.10, §5.2; spine §2).
///
/// Carries the carrier chart (the two-axis chart the branch lives in — its
/// `carrier_chart`), and the certified samples along the branch, each a
/// chart point plus its certified continuation frame. This is the curve the
/// carrier arrangement of T2 consumes as a tangential-contact stratum
/// (H-atom): omitting it would break side-bit constancy on the atoms.
///
/// The shape ships behind the pending refusal
/// [`A2_BRANCH_PACKET_PENDING`]: CTE-000 freezes the shape so CTE-007 can type
/// against it now, but PRODUCING a branch curve is CTE-005's work and refuses
/// until then. Constructed through [`A2BranchCurve::new`]; the producing stub
/// [`A2BranchCurve::produce`] refuses with the pending cause.
#[derive(Debug, Clone, PartialEq)]
pub struct A2BranchCurve {
    /// The two-axis carrier chart the branch is recorded in.
    carrier_chart: [(f64, f64); 2],
    /// The certified samples along the branch.
    samples: Vec<A2BranchSample>,
}

impl A2BranchCurve {
    /// Assemble a branch curve from its carrier chart and samples.
    ///
    /// Refuses ([`Refusal::InvalidInput`]) a non-finite/misordered carrier
    /// chart interval or an empty sample list (a branch curve with no sample is
    /// not a certified non-empty branch).
    pub fn new(
        carrier_chart: [(f64, f64); 2],
        samples: Vec<A2BranchSample>,
    ) -> Result<Self, Refusal> {
        if !carrier_chart.iter().all(|i| interval_ok(*i)) || samples.is_empty() {
            return Err(Refusal::InvalidInput);
        }
        Ok(Self {
            carrier_chart,
            samples,
        })
    }

    /// The producing stub: PRODUCING a branch curve from a real solver run is
    /// CTE-005's work and is not certified by this packet.
    ///
    /// Always refuses with the pending refusal name
    /// [`A2_BRANCH_PACKET_PENDING`] (mapped onto [`Refusal::InvalidInput`] for
    /// the shape layer; the named cause is recorded in the mapping table and
    /// asserted by CTE-008's gates).
    pub fn produce(
        _carrier_chart: [(f64, f64); 2],
        _samples: Vec<A2BranchSample>,
    ) -> Result<A2BranchCurve, Refusal> {
        Err(Refusal::InvalidInput)
    }

    /// The carrier chart, verbatim.
    pub fn carrier_chart(&self) -> [(f64, f64); 2] {
        self.carrier_chart
    }

    /// The certified samples, verbatim.
    pub fn samples(&self) -> &[A2BranchSample] {
        &self.samples
    }
}

/// The two-bit side signature `σ_S(α) = (m⁻, m⁺)` of a solid `S` on a carrier
/// atom `α` (theory §5.3): state `00` = exterior on both sides, `11` =
/// interior on both sides (the carrier is internal to `S`), `10`/`01` =
/// oriented boundary.
///
/// Stored as the raw `u8` with the `m⁻` bit in the high place and the `m⁺` bit
/// in the low place, so the state reads as the two-bit number `(m⁻)(m⁺)`
/// (`10` = `0b10` = canonical boundary, `01` = `0b01` = flipped boundary).
/// Constructed only through [`SideState::new`], which refuses a value outside
/// `0..=3`.
///
/// The §5.4 coordinatewise algebra and the [`From`] conversion
/// `From<&MaterialState4>` are CTE-006's obligation: the target type is
/// declared here, and the `impl` lands in `truck-shapeops` (CTE-006) — a
/// cross-crate `From` whose local type is the shapeops `MaterialState4`, so it
/// is implementable there. No cross-crate dependency exists today.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SideState(u8);

impl SideState {
    /// Build a side state.
    ///
    /// Refuses ([`Refusal::InvalidInput`]) a value outside the two-bit range
    /// `0..=3`.
    pub fn new(state: u8) -> Result<Self, Refusal> {
        if state > 3 {
            return Err(Refusal::InvalidInput);
        }
        Ok(Self(state))
    }

    /// The two-bit state `00..11` with `m⁻` in the high bit, verbatim.
    pub fn state(&self) -> u8 {
        self.0
    }

    /// The `m⁻` side witness (high bit).
    pub fn minus(&self) -> bool {
        self.0 & 0b10 != 0
    }

    /// The `m⁺` side witness (low bit).
    pub fn plus(&self) -> bool {
        self.0 & 0b01 != 0
    }

    /// Whether the carrier is internal to the operand — both side witnesses
    /// interior (state `11`).
    pub fn mid(&self) -> bool {
        self.0 == 0b11
    }

    /// The coordinatewise complement `¬σ` of the §5.4 algebra: `00 ↔ 11` and
    /// `01 ↔ 10`.
    pub fn flip(&self) -> Self {
        Self(self.0 ^ 0b11)
    }
}

/// The chosen certified normal of a T2 carrier (theory §5.2).
///
/// Consuming side only: CTE-007 fills this record from the classified
/// operand-face data; nothing here produces it. Constructed only through
/// [`CarrierNormal::new`], which refuses a non-finite or zero-length direction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CarrierNormal {
    /// The chosen carrier normal direction `n_C` (certified unit by the
    /// producing layer; the freeze only checks finiteness and non-vanishing).
    direction: [f64; 3],
}

impl CarrierNormal {
    /// Build a carrier normal.
    ///
    /// Refuses ([`Refusal::InvalidInput`]) a non-finite or exactly-zero
    /// direction. Unit length is the producing layer's certified contract.
    pub fn new(direction: [f64; 3]) -> Result<Self, Refusal> {
        if !direction.iter().all(|c| c.is_finite()) {
            return Err(Refusal::InvalidInput);
        }
        let len_sq =
            direction[0] * direction[0] + direction[1] * direction[1] + direction[2] * direction[2];
        if len_sq == 0.0 {
            return Err(Refusal::InvalidInput);
        }
        Ok(Self { direction })
    }

    /// The direction, verbatim.
    pub fn direction(&self) -> [f64; 3] {
        self.direction
    }
}

/// The parameter chart of a T2 carrier (theory §5.2).
///
/// Consuming side only: the two-axis chart domain a carrier's trim domains and
/// contact curves are expressed in. Constructed only through
/// [`CarrierChart::new`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CarrierChart {
    /// The two-axis chart domain.
    domain: [(f64, f64); 2],
}

impl CarrierChart {
    /// Build a carrier chart.
    ///
    /// Refuses ([`Refusal::InvalidInput`]) a non-finite or misordered domain
    /// interval.
    pub fn new(domain: [(f64, f64); 2]) -> Result<Self, Refusal> {
        if !domain.iter().all(|i| interval_ok(*i)) {
            return Err(Refusal::InvalidInput);
        }
        Ok(Self { domain })
    }

    /// The two-axis chart domain, verbatim.
    pub fn domain(&self) -> [(f64, f64); 2] {
        self.domain
    }
}

/// The certified common carrier of two or more operand faces (theory §5.2):
/// the carrier normal and chart.
///
/// Consuming side only — production lands in CTE-007 from the classified
/// operand data. Constructed only through [`CoincidenceCarrier::new`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CoincidenceCarrier {
    normal: CarrierNormal,
    chart: CarrierChart,
}

impl CoincidenceCarrier {
    /// Build a carrier record from its normal and chart. Infallible once both
    /// are validated.
    pub fn new(normal: CarrierNormal, chart: CarrierChart) -> Result<Self, Refusal> {
        Ok(Self { normal, chart })
    }

    /// The chosen carrier normal, verbatim.
    pub fn normal(&self) -> CarrierNormal {
        self.normal
    }

    /// The carrier chart, verbatim.
    pub fn chart(&self) -> CarrierChart {
        self.chart
    }
}

/// One operand face's trim domain on a carrier, in the carrier chart (theory
/// §5.2).
///
/// Consuming side only. Constructed only through [`CarrierTrimDomain::new`].
#[derive(Debug, Clone, PartialEq)]
pub struct CarrierTrimDomain {
    /// The operand slot (`0` or `1` of the Boolean pair).
    operand: u8,
    /// The trim domain in the carrier chart.
    domain: [(f64, f64); 2],
    /// The operand's side signature on the domain.
    side: SideState,
}

impl CarrierTrimDomain {
    /// Build a trim-domain record.
    ///
    /// Refuses ([`Refusal::InvalidInput`]) an operand slot outside `0..=1` or a
    /// non-finite/misordered domain interval.
    pub fn new(operand: u8, domain: [(f64, f64); 2], side: SideState) -> Result<Self, Refusal> {
        if operand > 1 || !domain.iter().all(|i| interval_ok(*i)) {
            return Err(Refusal::InvalidInput);
        }
        Ok(Self {
            operand,
            domain,
            side,
        })
    }

    /// The operand slot.
    pub fn operand(&self) -> u8 {
        self.operand
    }

    /// The trim domain in the carrier chart.
    pub fn domain(&self) -> [(f64, f64); 2] {
        self.domain
    }

    /// The operand's side signature on the domain.
    pub fn side(&self) -> SideState {
        self.side
    }
}

/// One stratum of the carrier arrangement (theory §5.2): a trim-domain
/// boundary curve or a tangential contact curve incident to the carrier.
///
/// Consuming side only — the arrangement that refines the atoms is CTE-007's;
/// this freeze types the strata the atoms' side bits are constant on (H-atom).
#[derive(Debug, Clone, PartialEq)]
pub enum CarrierStratum {
    /// The boundary of one operand's trim domain in the carrier chart.
    TrimBoundary {
        /// The operand whose trim-domain boundary this is.
        operand: u8,
    },
    /// A tangential contact curve of the carrier (a T1 A₂ branch on `C`).
    TangentialContact(A2BranchCurve),
}

/// The T2 coincidence witness: certified evidence that two faces lie on a
/// common carrier (theory §4.4, §5.8).
///
/// Certified common-carrier coincidence is the same exactness problem as A₁'s
/// critical value — no enclosure can establish it — so the witness is
/// provenance-first with the §4 exact primitive as the fallback:
///
/// - `CoincidenceIdentity::ProvenanceIdentical` — the operands are the same
///   construction node; the self-pair rewrites before the sweep (§5.8).
/// - `CoincidenceIdentity::ExactCoincident` — an A₂-instantiated exact
///   vanishing identity certifies the common carrier (§4.4).
///
/// `P` is the ℚ-polynomial representation, as in [`ExactVanishingWitness`].
/// Consuming side only. Constructed through [`CoincidenceWitness::new`].
#[derive(Debug, Clone, PartialEq)]
pub struct CoincidenceWitness<P> {
    carrier: CoincidenceCarrier,
    a: CarrierTrimDomain,
    b: CarrierTrimDomain,
    identity: CoincidenceIdentity<P>,
}

/// The identity route of a [`CoincidenceWitness`].
#[derive(Debug, Clone, PartialEq)]
pub enum CoincidenceIdentity<P> {
    /// The operands are the same construction node — rewrite before the sweep.
    ProvenanceIdentical,
    /// An exact common-carrier identity (the §4.4 A₂ instantiation).
    ExactCoincident(ExactVanishingWitness<P>),
}

impl<P> CoincidenceWitness<P> {
    /// Build a coincidence witness.
    ///
    /// Refuses ([`Refusal::InvalidInput`]) when both operands claim the same
    /// operand slot, or when the exact-identity route carries a witness that is
    /// not an A₂ instantiation.
    pub fn new(
        carrier: CoincidenceCarrier,
        a: CarrierTrimDomain,
        b: CarrierTrimDomain,
        identity: CoincidenceIdentity<P>,
    ) -> Result<Self, Refusal> {
        if a.operand() == b.operand() {
            return Err(Refusal::InvalidInput);
        }
        if let CoincidenceIdentity::ExactCoincident(witness) = &identity {
            if witness.side() != WitnessSide::A2 {
                return Err(Refusal::InvalidInput);
            }
        }
        Ok(Self {
            carrier,
            a,
            b,
            identity,
        })
    }

    /// The common carrier, verbatim.
    pub fn carrier(&self) -> CoincidenceCarrier {
        self.carrier
    }

    /// The first operand's trim domain, verbatim.
    pub fn a(&self) -> &CarrierTrimDomain {
        &self.a
    }

    /// The second operand's trim domain, verbatim.
    pub fn b(&self) -> &CarrierTrimDomain {
        &self.b
    }

    /// The identity route, verbatim.
    pub fn identity(&self) -> &CoincidenceIdentity<P> {
        &self.identity
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::Refusal;

    /// A valid two-axis interval pair helper for tests.
    fn interval(lo: f64, hi: f64) -> CertifiedInterval {
        CertifiedInterval { lo, hi }
    }

    /// A minimal valid chart for the shape tests.
    fn valid_chart() -> Rank2Chart {
        let pivot = Rank2Pivot::new(0, (0, 1)).expect("a valid pivot");
        let net = ChartMinorNet::new([1, 1, 1, 1], vec![1.0; 16]).expect("a valid minor net");
        let minors = ChartMinorGrids::new(net.clone(), net).expect("valid minors");
        Rank2Chart::new(pivot, interval(2.0, 3.0), minors).expect("a valid chart")
    }

    /// A minimal valid graph certificate for the shape tests.
    fn valid_graph_cert() -> GraphCert {
        GraphCert::new(
            [(0.0, 1.0), (0.0, 1.0), (0.0, 1.0), (0.0, 1.0)],
            [(0.25, 0.75), (0.25, 0.75), (0.25, 0.75), (0.25, 0.75)],
        )
        .expect("a valid strict inclusion")
    }

    #[test]
    fn det_a_constructor_refuses_zero_containing_enclosure() {
        let pivot = Rank2Pivot::new(1, (2, 3)).expect("a valid pivot");
        let net = ChartMinorNet::new([1, 1, 1, 1], vec![1.0; 16]).expect("a valid minor net");
        let minors = ChartMinorGrids::new(net.clone(), net).expect("valid minors");

        // An enclosure straddling zero refuses.
        let straddles = Rank2Chart::new(pivot, interval(-1.0, 2.0), minors.clone());
        assert!(matches!(straddles, Err(Refusal::InvalidInput)));

        // An enclosure with a 0 endpoint refuses (the exclusion is STRICT).
        let endpoint = Rank2Chart::new(pivot, interval(-1.0, 0.0), minors.clone());
        assert!(matches!(endpoint, Err(Refusal::InvalidInput)));

        // A single-point enclosure of exactly 0 refuses.
        let point_zero = Rank2Chart::new(pivot, interval(0.0, 0.0), minors.clone());
        assert!(matches!(point_zero, Err(Refusal::InvalidInput)));

        // A misordered enclosure refuses.
        let misordered = Rank2Chart::new(pivot, interval(3.0, 1.0), minors.clone());
        assert!(matches!(misordered, Err(Refusal::InvalidInput)));

        // A strictly positive enclosure admits.
        let admits = Rank2Chart::new(pivot, interval(0.5, 2.0), minors);
        assert!(admits.is_ok());
    }

    #[test]
    fn graph_enclosure_only_input_of_hessian_signature() {
        // The R2 rule (theory §2.5): the reduced-Hessian evaluator consumes
        // ONLY the certified graph enclosure. This test compiles-uses the
        // frozen trait signature — a raw-box overload does not exist and
        // cannot be passed here.
        struct R2Probe;

        impl ReducedHessianEvaluator for R2Probe {
            fn reduced_hessian(
                _chart: &Rank2Chart,
                _over: &GraphEnclosure,
            ) -> Result<IntervalSym2, Refusal> {
                // CTE-004 provides the real bodies; this probe only pins the
                // signature shape and never evaluates numerically.
                Err(Refusal::InvalidInput)
            }
        }

        // The signature as a function item type: `(&Rank2Chart,
        // &GraphEnclosure) -> Result<IntervalSym2, Refusal>`. Binding it to
        // the trait's associated function is the compile-time shape assertion:
        // the evaluator's ONLY input is the GraphEnclosure.
        let _signature: fn(&Rank2Chart, &GraphEnclosure) -> Result<IntervalSym2, Refusal> =
            <R2Probe as ReducedHessianEvaluator>::reduced_hessian;

        let chart = valid_chart();
        let graph = GraphEnclosure::new(
            [(0.0, 0.5), (0.0, 0.5)],
            [(0.0, 1.0), (0.0, 1.0)],
            valid_graph_cert(),
        )
        .expect("a valid graph enclosure");

        // The probe refuses (no numeric evaluation in this freeze), but the
        // call site type-checks against the frozen R2 signature.
        let result = <R2Probe as ReducedHessianEvaluator>::reduced_hessian(&chart, &graph);
        assert!(matches!(result, Err(Refusal::InvalidInput)));
    }

    #[test]
    fn definiteness_carries_positive_mu() {
        // R9: a Definite verdict must carry the mu the Taylor bound consumes;
        // mu <= 0 refuses.
        let zero = Definiteness::definite(DefinitenessSign::Positive, 0.0);
        assert!(matches!(zero, Err(Refusal::InvalidInput)));

        let negative = Definiteness::definite(DefinitenessSign::Negative, -1.0);
        assert!(matches!(negative, Err(Refusal::InvalidInput)));

        let nan = Definiteness::definite(DefinitenessSign::Positive, f64::NAN);
        assert!(matches!(nan, Err(Refusal::InvalidInput)));

        // A positive mu admits and is carried verbatim.
        let admitted = Definiteness::definite(DefinitenessSign::Positive, 1.0e-6)
            .expect("a positive mu admits");
        match admitted {
            Definiteness::Definite { sign, mu } => {
                assert_eq!(sign, DefinitenessSign::Positive);
                assert!(mu.get() > 0.0);
            }
            _ => panic!("a constructed definite verdict must be the Definite arm"),
        }

        // The indefinite arm requires a strictly negative det upper bound.
        assert!(matches!(
            Definiteness::indefinite(0.0),
            Err(Refusal::InvalidInput)
        ));
        assert!(matches!(
            Definiteness::indefinite(1.5),
            Err(Refusal::InvalidInput)
        ));
        let indefinite = Definiteness::indefinite(-1.0).expect("a strictly negative bound admits");
        match indefinite {
            Definiteness::Indefinite { det_upper } => assert!(det_upper < 0.0),
            _ => panic!("an indefinite verdict must be the Indefinite arm"),
        }
    }

    #[test]
    fn witness_data_refuses_incomplete_identity() {
        // P is any polynomial stand-in; the shape checks are structural, so
        // the integer literals below infer `i32` as the polynomial type.

        // A1 requires exactly the four terms (G1,G2,M1,M2) and no q2a term.
        let a1_terms = vec![(1, 1), (1, 1), (1, 1), (1, 1)];
        let valid_a1 = ExactVanishingWitness::new(WitnessSide::A1, 1, a1_terms.clone(), None);
        assert!(valid_a1.is_ok());

        // A1 with a q2a term is a mismatched side shape.
        let a1_with_q2a =
            ExactVanishingWitness::new(WitnessSide::A1, 1, a1_terms.clone(), Some((2, 3)));
        assert!(matches!(a1_with_q2a, Err(Refusal::InvalidInput)));

        // A1 with an incomplete term family (missing the minors) refuses.
        let a1_incomplete =
            ExactVanishingWitness::new(WitnessSide::A1, 1, vec![(1, 1), (1, 1)], None);
        assert!(matches!(a1_incomplete, Err(Refusal::InvalidInput)));

        // A2 requires exactly the two terms (G1,G2) AND the q2a factor.
        let a2_terms = vec![(1, 1), (1, 1)];
        let valid_a2 =
            ExactVanishingWitness::new(WitnessSide::A2, 1, a2_terms.clone(), Some((2, 3)));
        assert!(valid_a2.is_ok());

        // A2 without the q2a factor is an incomplete identity.
        let a2_missing_q2a = ExactVanishingWitness::new(WitnessSide::A2, 1, a2_terms.clone(), None);
        assert!(matches!(a2_missing_q2a, Err(Refusal::InvalidInput)));

        // A2 with the wrong term count refuses.
        let a2_extra_term = ExactVanishingWitness::new(
            WitnessSide::A2,
            1,
            vec![(1, 1), (1, 1), (1, 1)],
            Some((2, 3)),
        );
        assert!(matches!(a2_extra_term, Err(Refusal::InvalidInput)));

        // Accessors are verbatim.
        if let Ok(witness) = valid_a1 {
            assert_eq!(witness.side(), WitnessSide::A1);
            assert_eq!(witness.s(), &1);
            assert_eq!(witness.terms().len(), 4);
            assert!(witness.q2a().is_none());
        }
    }
}
