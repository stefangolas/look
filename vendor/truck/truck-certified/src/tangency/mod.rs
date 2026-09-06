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

/// The CTE fixture kit (booking §5, normative). TEST SUPPORT ONLY: this is
/// `#[doc(hidden)] pub` so the wave packets' tests can reach it through the
/// crate's public path without admitting it to the certified API surface
/// (the `ssi_fixtures` precedent).
/// The Lemma T1.0 bounded 18-chart search (CTE-002-GRAPH).
pub mod chart;
#[doc(hidden)]
pub mod fixtures;
/// The (H-graph) parametric Newton/Krawczyk graph certification
/// (CTE-002-GRAPH).
pub mod graph;
pub mod shapes;
