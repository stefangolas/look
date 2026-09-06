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

//! The Certified Tangency and Exact Contact (CTE) tangency layer
//! (CTE-000-SPINE and the CTE-001..008 wave packets that build on it).
//!
//! This module freezes the certificate vocabulary for singular SSI closure
//! (T1) and coincident-carrier Boolean classification (T2) in the P0-FREEZE
//! pattern (`ssi_types.rs` precedent): **types and refusing constructors only —
//! nothing here evaluates, solves, isolates, or certifies numerically.** Seven
//! later packets (CTE-001..005, 007, 008) build against this module and never
//! restate it. The mathematics is frozen in
//! `docs/CERTIFIED_TANGENCY_EXACT_CONTACT_SPEC.md` (T1.0–T1.8, T2.1–T2.5) and
//! the contract inventory in `docs/CTE_BUILD_SPINE.md` §2.
//!
//! **D-shim.** Types and refusing constructors only. Any method that would
//! evaluate, solve, isolate, or certify NUMERICALLY refuses
//! (`InvalidInput`-shaped or a named case from the existing vocabularies). The
//! module doc says verbatim: "This module freezes shapes; the CTE wave packets
//! implement against it and never restate it."
//!
//! **Frozen rules (spine §2, structural).**
//!
//! - **R2** — `reduced_hessian` consumes the certified [`GraphEnclosure`],
//!   never a raw box (the trait [`ReducedHessianEvaluator`] is the only
//!   signature; no raw-box overload exists).
//! - **R1** — no type here carries a "deflation determinant predicate":
//!   T1.4 appears only in the fixture kit as an exact identity test (F1) and in
//!   doc comments citing the theory.
//! - **R9** — [`Definiteness::Definite`] carries the μ the Taylor isolation
//!   bound consumes; construction with `mu <= 0` refuses.
//! - **R3** — `A1Node` (the indefinite Morse case) is FIRST-CLASS from day one;
//!   it is a [`ContactVerdict`] arm, never a TODO.
//! - **R6** — the `A2Branch` verdict's certificate carries a certified
//!   non-empty branch; vacuous satisfaction is not certifiable.
//!
//! **Zero new top-level evidence kinds.** Refusals map onto the landed
//! vocabularies (`SsiRefusal`, `TraceRefusal`, kernel `Refusal`,
//! `HullRefusal`); the mapping rows are recorded in
//! `docs/CERTIFICATE_MAPPING.md`. A case that seems to need a new arm is a
//! SPEC_GAP.
//!
//! The CTE-003 wave adds the A₁/A₂ deflation substrate behind those shapes
//! (CTE-003-MINORS): the polynomial chart-minor grids
//! ([`minors::build_chart_minors`]), the deflated `T = (G₁, G₂, M₁, M₂`
//! system as a [`tsystem::TSystem`] (`KrawczykSystem<4>`), and the T1.5(1)
//! five-equation exclusion driver [`exclude::exclude_five`]. All of it
//! returns the module-level refusal vocabulary [`TangencyRefusal`], which
//! wraps the landed named causes verbatim.

/// The CTE fixture kit (booking §5, normative). TEST SUPPORT ONLY: this is
/// `#[doc(hidden)] pub` so the wave packets' tests can reach it through the
/// crate's public path without admitting it to the certified API surface
/// (the `ssi_fixtures` precedent).
/// The Lemma T1.0 bounded 18-chart search (CTE-002-GRAPH).
pub mod chart;
#[doc(hidden)]
pub mod fixtures;

/// The CTE program battery gates (CTE-008-GATES): deterministic certified
/// fixture-input reconstruction and gate helpers for the program battery.
/// TEST SUPPORT ONLY: `#[doc(hidden)] pub`, excluded from the certified API
/// surface (the `fixtures` precedent).
#[doc(hidden)]
pub mod gates;

/// The (H-graph) parametric Newton/Krawczyk graph certification
/// (CTE-002-GRAPH).
pub mod graph;
/// The chart-minor polynomial substrate (CTE-003-MINORS): `M₁, M₂` built as
/// composed Bernstein grids from the stored square system and a chart pivot,
/// together with the internal four-axis polynomial-grid algebra the deflated
/// system and the exclusion driver share.
pub mod minors;
pub mod shapes;

/// The deflated square system `T = (G₁, G₂, M₁, M₂)` (theory §2.4) as a
/// [`tsystem::TSystem`] instantiating the landed generic
/// `KrawczykSystem<4>` (CTE-003-MINORS).
pub mod tsystem;

/// The T1.5(1) five-equation exclusion driver (CTE-003-MINORS): subdivision
/// under [`Budget`](truck_base::evidence::Budget) deciding whether
/// `(F, M₁, M₂)` has a root in a box.
pub mod exclude;

/// The T1.3 reduced-Hessian evaluator and the §2.8 definiteness test
/// (CTE-004-HESSIAN): the R2 graph-enclosure signature
/// [`hessian::reduced_hessian`] and the R9-carrying
/// [`hessian::definiteness`].
pub mod hessian;

/// The five-way contact-classifier cascade (CTE-004-HESSIAN): the theory's
/// stage table (§2.11) over one certified box, driven by
/// [`cascade::classify_box`].
pub mod cascade;

/// The T2 carrier arrangement into atoms (CTE-007-T2ARRANGE): the certified
/// arrangement of a coincidence witness's trim domains and the tangential
/// contact curves as strata, with H-atom probe admission, the §5.6 cheap
/// containment predicate, and the §5.5 single-canonical-record emission.
pub mod arrange;

use crate::contract::Refusal;
use crate::hull::HullRefusal;

/// The CTE deflation substrate's refusal vocabulary (CTE-003-MINORS; scope
/// decision 6 — refusals are named, wrapping landed causes).
///
/// Zero new top-level evidence kinds: every variant wraps a landed named
/// cause verbatim (D-reuse). A construction outside a frozen rule carries
/// [`Refusal::InvalidInput`]; a hull failure carries the landed
/// [`HullRefusal`] cases.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TangencyRefusal {
    /// A construction outside a frozen rule or an invalid input. Carries the
    /// landed [`Refusal::InvalidInput`].
    Input(Refusal),
    /// A certified enclosure could not be produced. Carries the landed
    /// [`HullRefusal`] (`EnclosureUnavailable` / `DomainNotCompact`).
    Hull(HullRefusal),
}

impl From<Refusal> for TangencyRefusal {
    fn from(refusal: Refusal) -> Self {
        TangencyRefusal::Input(refusal)
    }
}

impl From<HullRefusal> for TangencyRefusal {
    fn from(refusal: HullRefusal) -> Self {
        TangencyRefusal::Hull(refusal)
    }
}

impl TangencyRefusal {
    /// A short stable tag, for diagnostics.
    pub fn tag(self) -> &'static str {
        match self {
            Self::Input(Refusal::InvalidInput) => "tangency_invalid_input",
            Self::Input(Refusal::ConditioningBelowThreshold) => "tangency_conditioning",
            Self::Input(Refusal::Unfrozen) => "tangency_unfrozen",
            Self::Hull(HullRefusal::EnclosureUnavailable) => "tangency_hull_enclosure_unavailable",
            Self::Hull(HullRefusal::DomainNotCompact) => "tangency_hull_domain_not_compact",
        }
    }
}
/// The ℚ-polynomial substrate of the exact-vanishing verifier (CTE-001-QPOLY):
/// a hand-rolled exact multivariate polynomial ring with `i128` rational
/// coefficients (no CAS dependency). Consumed by [`witness`] and, later, by
/// CTE-005/007.
pub mod qpoly;

/// The one-witness exact verifier (CTE-001-QPOLY): the frozen
/// [`shapes::ExactWitnessVerifier`] implementation over [`qpoly::QPoly`] plus
/// the pending-refusal provenance stub [`witness::provenance_witness`].
pub mod witness;

/// The A₂ (Morse–Bott branch) certificate and its rank-3 continuation adapter
/// (CTE-005-A2): the T1.7 contact-factor identity with the interval
/// nonvanishing multipliers, the rank-3 separation of `D(G₁, G₂, q)`, the R6
/// certified non-empty branch, and the parallelotope branch-curve production.
pub mod a2;
