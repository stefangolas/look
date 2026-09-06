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

//! The CTE program battery gates (CTE-008-GATES). TEST SUPPORT ONLY.
//!
//! This module is `#[doc(hidden)] pub`, excluded from the certified API
//! surface exactly like `fixtures` (a one-line mapping-table note, not a row:
//! no new evidence kind). It lets the program battery's integration tests
//! drive the crate's public certified machinery on the F3/F4/F5/F6 fixture
//! systems, which the crate's in-crate test kits build only under
//! `#[cfg(test)]` (invisible to integration tests). The fixture systems are
//! reconstructed here byte-identically through public API only:
//!
//! - **F3** (A₁⁺, definite) and **F4** (A₁⁻, saddle node) are the monomial
//!   square systems `(G₁, G₂, f) = (y1 − 1/2, y2 − 1/2, f)` of
//!   [`cascade`](crate::tangency::cascade)'s A₁ fixtures, with EXACT
//!   chart-minor nets (`M_j = ∂f/∂z_j`, theory T1.1): the A₁ exact-zero
//!   witness needs the deflated rows exact, and the float-composed
//!   `build_chart_minors` would perturb them.
//! - **F5** (A₂) is the parabola system `(u − s, v − t, (s − 1/2)²)` with the
//!   hand-written T1.7 witness `s = a = 1`, `q = s − 1/2`.
//! - **F6** (transversal) is the crossing-planes system
//!   `(u − s, v − t, 1/2 − s)`.
//! - **F1/F2** (identity fixtures) have no classify box; their certified
//!   ground truths are machine-checked by the always-compiled fixture kit's
//!   exact `admit()` and recorded as depth-0 identity rows in the battery.
//!
//! **R1.** No runtime call site evaluates the T1.4 deflation-determinant
//! identity; the classify entry takes only the standard certified inputs and
//! the F1 identity is exercised through the fixture kit's exact admission.
//!
//! **House rules.** H-1 (no unwrap/expect/panic), fixed ordered construction,
//! no hash iteration. Every construction is deterministic: two calls return
//! identical inputs, so two classify runs return identical verdicts.

use crate::formal::exact::CertifiedInterval;
use crate::ssi_types::SquareSystem3;
use crate::tangency::a2::{certify_a2, A2ExactData, ExactPoly};
use crate::tangency::cascade::classify_box;
use crate::tangency::graph::{certify_graph, CertifiedGraph};
use crate::tangency::qpoly::{QCoeff, QPoly};
use crate::tangency::shapes::{
    ChartMinorGrids, ChartMinorNet, ContactVerdict, ExactVanishingWitness, Rank2Chart, Rank2Pivot,
    WitnessSide,
};
use crate::tangency::tsystem::TSystem;
use truck_base::evidence::Budget;

/// A four-axis box in the unit chart.
pub type Box4 = [(f64, f64); 4];

/// The identity chart maps every fixture system stores.
const IDENTITY_MAPS: (f64, f64, f64, f64, f64, f64, f64, f64) =
    (0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0);

/// The per-axis degrees `(y1, y2, z1, z2)` of the A₁ fixture systems.
const A1_SYSTEM_DEGREES: (usize, usize, usize, usize) = (1, 1, 2, 2);

/// The booked subdivision budget of one A₁/transversal classify run.
const BOOKED_SUBDIV: u32 = 256;

/// The booked subdivision budget of one A₂ certify run.
const A2_BOOKED_SUBDIV: u32 = 4096;

/// A certified classify input for one box.
#[derive(Debug, Clone)]
pub struct ClassifyInput {
    /// The stored square system.
    pub system: SquareSystem3,
    /// The certified (H-graph) content over the box.
    pub graph: CertifiedGraph,
    /// The chart-minor grids.
    pub minors: ChartMinorGrids,
    /// The deflated `T = (G₁, G₂, M₁, M₂)` system.
    pub tsys: TSystem,
    /// The box being classified.
    pub b: Box4,
}

impl ClassifyInput {
    /// Run the frozen cascade on this input.
    pub fn classify(&self, budget: &mut Budget) -> Result<ContactVerdict<QPoly>, String> {
        classify_box(
            &self.system,
            &self.graph,
            &self.minors,
            &self.tsys,
            &self.b,
            budget,
        )
        .map_err(|e| format!("{e:?}"))
    }
}

/// An A₂ certify input.
#[derive(Debug, Clone)]
pub struct A2Input {
    /// The stored parabola system.
    pub system: SquareSystem3,
    /// The placeholder minors carrier.
    pub minors: ChartMinorGrids,
    /// The T1.7 witness.
    pub witness: ExactVanishingWitness<QPoly>,
    /// The readable exact data.
    pub data: A2ExactData,
    /// The box containing the branch.
    pub b: Box4,
}

impl A2Input {
    /// Run the A₂ certify route on this input.
    pub fn certify(&self, budget: &mut Budget) -> Result<ContactVerdict<QPoly>, String> {
        certify_a2(
            &self.system,
            &self.minors,
            &self.witness,
            &self.data,
            &self.b,
            budget,
        )
        .map_err(|e| format!("{e:?}"))
    }
}

// ---------------------------------------------------------------------------
// monomial → Bernstein grid helpers (dyadic-exact for the fixture degrees)
// ---------------------------------------------------------------------------

/// The binomial coefficient `C(n, k)` as `f64`.
fn binom(n: usize, k: usize) -> f64 {
    if k > n {
        return 0.0;
    }
    let mut out = 1.0f64;
    for t in 0..k {
        out = out * ((n - t) as f64) / ((t + 1) as f64);
    }
    out
}

/// A four-axis tensor-Bernstein grid from monomial terms over the identity
/// chart, in the landed flat layout.
fn monomial_grid4(
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

/// A square system from three monomial component grids.
fn system_from_monomials(
    degrees: (usize, usize, usize, usize),
    comps: [&[([usize; 4], f64)]; 3],
) -> Result<SquareSystem3, String> {
    let grids = [
        monomial_grid4(degrees, comps[0]),
        monomial_grid4(degrees, comps[1]),
        monomial_grid4(degrees, comps[2]),
    ];
    SquareSystem3::new(grids, degrees, IDENTITY_MAPS).map_err(|e| format!("{e:?}"))
}

// ---------------------------------------------------------------------------
// F3 / F4 — the A₁ fixture systems (definite bowl / indefinite saddle node)
// ---------------------------------------------------------------------------

/// The exact Bernstein control value of the monomial `x^exp` at the net index
/// `idx` of a tensor net of per-axis degrees `[1, 1, 2, 2]`: the per-axis
/// factor `C(idx, exp)/C(d, exp)`.
fn monomial_control(exp: &[usize; 4], idx: &[usize; 4]) -> f64 {
    let d = [1usize, 1, 2, 2];
    let mut out = 1.0f64;
    for axis in 0..4 {
        out *= binom(idx[axis], exp[axis]);
        out /= binom(d[axis], exp[axis]);
    }
    out
}

/// A chart-minor net over the A₁ fixture degrees whose exact monomial content
/// is `Σ coeff·x^exp`.
fn exact_minor_net(terms: &[([usize; 4], f64)]) -> Result<ChartMinorNet, String> {
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
    ChartMinorNet::new(degrees, coeffs).map_err(|e| format!("{e:?}"))
}

/// The A₁ fixture `f` as monomial data (F3 bowl or F4 saddle).
fn a1_f_monomials(bowl: bool) -> &'static [([usize; 4], f64)] {
    if bowl {
        &[
            ([0, 0, 2, 0], 1.0),
            ([0, 0, 1, 0], -1.0),
            ([0, 0, 0, 2], 1.0),
            ([0, 0, 0, 1], -1.0),
            ([0, 0, 0, 0], 0.5),
        ]
    } else {
        &[
            ([0, 0, 2, 0], 1.0),
            ([0, 0, 1, 0], -1.0),
            ([0, 0, 0, 2], -1.0),
            ([0, 0, 0, 1], 1.0),
        ]
    }
}

/// Build the A₁ classify input: the square system `(y1−1/2, y2−1/2, f)` over
/// the identity chart with an EXACT chart and the certified (H-graph) content
/// over the A₁ box `[(0.4, 0.6); 4]`.
fn a1_input(bowl: bool) -> Result<ClassifyInput, String> {
    let g1: &[([usize; 4], f64)] = &[([1, 0, 0, 0], 1.0), ([0, 0, 0, 0], -0.5)];
    let g2: &[([usize; 4], f64)] = &[([0, 1, 0, 0], 1.0), ([0, 0, 0, 0], -0.5)];
    let f = a1_f_monomials(bowl);
    let system = system_from_monomials(A1_SYSTEM_DEGREES, [g1, g2, f])?;

    // The EXACT chart: pivot `(f = 2, y = (0, 1))` (det A = 1), minors the
    // exact `∂f/∂z_j`.
    let pivot = Rank2Pivot::new(2, (0, 1)).map_err(|e| format!("{e:?}"))?;
    let mut m1_terms: Vec<([usize; 4], f64)> = Vec::new();
    let mut m2_terms: Vec<([usize; 4], f64)> = Vec::new();
    for (exp, coeff) in a1_f_monomials(bowl) {
        if exp[2] > 0 {
            let mut ne = *exp;
            ne[2] -= 1;
            m1_terms.push((ne, exp[2] as f64 * *coeff));
        }
        if exp[3] > 0 {
            let mut ne = *exp;
            ne[3] -= 1;
            m2_terms.push((ne, exp[3] as f64 * *coeff));
        }
    }
    let m1 = exact_minor_net(&m1_terms)?;
    let m2 = exact_minor_net(&m2_terms)?;
    let minors_grids = ChartMinorGrids::new(m1, m2).map_err(|e| format!("{e:?}"))?;
    let chart = Rank2Chart::new(
        pivot,
        CertifiedInterval { lo: 0.5, hi: 1.5 },
        minors_grids.clone(),
    )
    .map_err(|e| format!("{e:?}"))?;

    let b: Box4 = [(0.4, 0.6), (0.4, 0.6), (0.4, 0.6), (0.4, 0.6)];
    let graph = certify_graph(&system, &chart, b).map_err(|e| format!("{e:?}"))?;
    let tsys = TSystem::from_deflated(&system, &chart).map_err(|e| format!("{e:?}"))?;
    Ok(ClassifyInput {
        system,
        graph,
        minors: minors_grids,
        tsys,
        b,
    })
}

/// The F3 (A₁⁺, definite) classify input.
pub fn f3_input() -> Result<ClassifyInput, String> {
    a1_input(true)
}

/// The F4 (A₁⁻, indefinite saddle node) classify input — the R3 gate
/// fixture.
pub fn f4_input() -> Result<ClassifyInput, String> {
    a1_input(false)
}

// ---------------------------------------------------------------------------
// F6 — the transversal crossing-planes system
// ---------------------------------------------------------------------------

/// The F6 (transversal) classify input: `F = (u − s, v − t, 1/2 − s)`, the
/// cross difference of the planes `z = 0` and `z = x − 1/2`, over the
/// straddling box.
pub fn f6_input() -> Result<ClassifyInput, String> {
    let c0: &[([usize; 4], f64)] = &[([1, 0, 0, 0], 1.0), ([0, 0, 1, 0], -1.0)];
    let c1: &[([usize; 4], f64)] = &[([0, 1, 0, 0], 1.0), ([0, 0, 0, 1], -1.0)];
    let c2: &[([usize; 4], f64)] = &[([0, 0, 0, 0], 0.5), ([0, 0, 1, 0], -1.0)];
    let system = system_from_monomials((1, 1, 1, 1), [c0, c1, c2])?;

    // The chart: pivot `(f = 2, y = (0, 1))`, float-composed minors (a
    // transversal box needs no exact deflated rows).
    let pivot = Rank2Pivot::new(2, (0, 1)).map_err(|e| format!("{e:?}"))?;
    let placeholder =
        ChartMinorNet::new([1, 1, 1, 1], vec![1.0; 16]).map_err(|e| format!("{e:?}"))?;
    let template_minors =
        ChartMinorGrids::new(placeholder.clone(), placeholder).map_err(|e| format!("{e:?}"))?;
    let template = Rank2Chart::new(
        pivot,
        CertifiedInterval { lo: 0.5, hi: 1.5 },
        template_minors,
    )
    .map_err(|e| format!("{e:?}"))?;
    let minors = crate::tangency::minors::build_chart_minors(&system, &template)
        .map_err(|e| format!("{e:?}"))?;
    let chart = Rank2Chart::new(
        pivot,
        CertifiedInterval { lo: 0.5, hi: 1.5 },
        minors.clone(),
    )
    .map_err(|e| format!("{e:?}"))?;

    let b: Box4 = [(0.1, 0.9), (0.1, 0.9), (0.25, 0.75), (0.25, 0.75)];
    let graph = certify_graph(&system, &chart, b).map_err(|e| format!("{e:?}"))?;
    let tsys = TSystem::from_deflated(&system, &chart).map_err(|e| format!("{e:?}"))?;
    Ok(ClassifyInput {
        system,
        graph,
        minors,
        tsys,
        b,
    })
}

// ---------------------------------------------------------------------------
// F5 — the A₂ parabola system with its hand-written T1.7 witness
// ---------------------------------------------------------------------------

/// The F5 (A₂) square system over the unit chart:
/// `F = (u − s, v − t, (s − 1/2)²)`.
fn f5_system() -> Result<SquareSystem3, String> {
    let c0: &[([usize; 4], f64)] = &[([1, 0, 0, 0], 1.0), ([0, 0, 1, 0], -1.0)];
    let c1: &[([usize; 4], f64)] = &[([0, 1, 0, 0], 1.0), ([0, 0, 0, 1], -1.0)];
    let c2: &[([usize; 4], f64)] = &[
        ([0, 0, 2, 0], 1.0),
        ([0, 0, 1, 0], -1.0),
        ([0, 0, 0, 0], 0.25),
    ];
    system_from_monomials((2, 2, 2, 1), [c0, c1, c2])
}

/// A placeholder minors carrier (the A₂ certificate ignores the grids).
fn minor_placeholder() -> Result<ChartMinorGrids, String> {
    let net = ChartMinorNet::new([1, 1, 1, 1], vec![1.0; 16]).map_err(|e| format!("{e:?}"))?;
    ChartMinorGrids::new(net.clone(), net).map_err(|e| format!("{e:?}"))
}

/// The F5 exact data: `s = 1`, `a = 1`, `q = s − 1/2`.
fn f5_data() -> Result<A2ExactData, String> {
    let one = QCoeff::from_int(1);
    let half = QCoeff::new(-1, 2).map_err(|e| format!("{e:?}"))?;
    let s = ExactPoly::constant(one);
    let a = ExactPoly::constant(one);
    let q = ExactPoly::from_terms(vec![(vec![0, 0, 1, 0], one), (vec![0, 0, 0, 0], half)])
        .map_err(|e| format!("{e:?}"))?;
    Ok(A2ExactData::new(s, q, a))
}

/// The F5 witness: `s = 1`, `a = 1`, `q = s − 1/2`, `w₁ = w₂ = 0`.
fn f5_witness() -> Result<ExactVanishingWitness<QPoly>, String> {
    let data = f5_data()?;
    let one = QCoeff::from_int(1);
    let g1 = ExactPoly::from_terms(vec![
        (vec![1, 0, 0, 0], one),
        (vec![0, 0, 1, 0], QCoeff::from_int(-1)),
    ])
    .map_err(|e| format!("{e:?}"))?;
    let g2 = ExactPoly::from_terms(vec![
        (vec![0, 1, 0, 0], one),
        (vec![0, 0, 0, 1], QCoeff::from_int(-1)),
    ])
    .map_err(|e| format!("{e:?}"))?;
    let s = data.s.to_qpoly().map_err(|e| format!("{e:?}"))?;
    let a = data.a.to_qpoly().map_err(|e| format!("{e:?}"))?;
    let q = data.q.to_qpoly().map_err(|e| format!("{e:?}"))?;
    let terms = vec![
        (QPoly::zero(4), g1.to_qpoly().map_err(|e| format!("{e:?}"))?),
        (QPoly::zero(4), g2.to_qpoly().map_err(|e| format!("{e:?}"))?),
    ];
    ExactVanishingWitness::new(WitnessSide::A2, s, terms, Some((q, a)))
        .map_err(|e| format!("{e:?}"))
}

/// The F5 (A₂) certify input over the branch-straddling box.
pub fn f5_input() -> Result<A2Input, String> {
    let system = f5_system()?;
    let minors = minor_placeholder()?;
    let witness = f5_witness()?;
    let data = f5_data()?;
    let b: Box4 = [(0.2, 0.8), (0.2, 0.8), (0.2, 0.8), (0.2, 0.8)];
    Ok(A2Input {
        system,
        minors,
        witness,
        data,
        b,
    })
}

// ---------------------------------------------------------------------------
// battery helpers
// ---------------------------------------------------------------------------

/// A booked budget for one A₁/transversal classify run.
pub fn booked_budget() -> Budget {
    Budget::new(BOOKED_SUBDIV, 0, 0)
}

/// A booked budget for one A₂ certify run.
pub fn a2_booked_budget() -> Budget {
    Budget::new(A2_BOOKED_SUBDIV, 0, 0)
}

/// Whether a verdict is one of the four certified verdict kinds (never
/// `Unresolved`; `Empty` is the degenerate case of `Transversal`).
pub fn is_certified(v: &ContactVerdict<QPoly>) -> bool {
    !matches!(v, ContactVerdict::Unresolved { .. })
}

/// The name of the verdict kind.
pub fn verdict_kind(v: &ContactVerdict<QPoly>) -> &'static str {
    match v {
        ContactVerdict::Transversal(_) => "Transversal",
        ContactVerdict::Empty => "Empty",
        ContactVerdict::A1Isolated(_) => "A1Isolated",
        ContactVerdict::A1Node(_) => "A1Node",
        ContactVerdict::A2Branch(_) => "A2Branch",
        ContactVerdict::Unresolved { .. } => "Unresolved",
    }
}

/// The fixture kit's exact admissions (F1–F6 records) each terminate
/// deterministically; this is the depth-0 identity row helper of the
/// termination battery.
pub fn admit_fixture_kit() -> Result<(), String> {
    crate::tangency::fixtures::build_kit()
        .and_then(|kit| kit.admit_all())
        .map_err(|e| format!("{e:?}"))
}

/// The R1 gate: no runtime call site evaluates the T1.4 identity. Asserts the
/// classification entry reaches the certified verdicts with the standard
/// certified inputs (chart/minors/deflated system) and the fixture kit's F1
/// admission is the only sanctioned exact evaluation. Returns the F4 verdict
/// name so the caller can additionally require `A1Node`.
pub fn t14_runtime_classify_ok() -> Result<&'static str, String> {
    admit_fixture_kit()?;
    let input = f4_input()?;
    let mut budget = booked_budget();
    let verdict = input.classify(&mut budget)?;
    Ok(verdict_kind(&verdict))
}
