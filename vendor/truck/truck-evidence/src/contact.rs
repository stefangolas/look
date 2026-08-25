//! BG-SOL-S3-CONTACT — the Contact Layer skeleton.
//!
//! `contact(lhs, rhs)` answers "how do these two boundary strata meet?" for
//! the solver family's Phase 3 funnel (docs/SOLVER_FAMILY_PLAN.md §4 Phase 3 +
//! §5). The flagship differential test `Extrude(P−Q) ≅ Extrude(P)−Extrude(Q)`
//! is the M2 cross-layer gate and needs the 3-D Boolean on its RHS, which the
//! Boundary Rewrite (Phase 4) drives from this oracle: every pair of boundary
//! strata (FF, FE, EE) is dispatched here.
//!
//! This packet establishes the stratum vocabulary (`BoundedStratum`,
//! `ContactComplex`, `ContactLocus`) and the dispatcher's two cheapest stages:
//! identity/overlap (C0-C2, coincident canonical carriers) and the analytic FF
//! pairs (plan §3.3, which already exist in `truck_evidence::analytic`).
//! Everything else — FE/EE strata reductions, general validated FF, singular
//! event cells, 2-D overlap — returns an honest
//! `Refusal::UnsupportedEnvelope(EnvelopeCase::ContactReductionDeferred)`, the
//! typed boundary of the funnel the later packets fill in.
//!
//! Strata are geometry-side on purpose: `truck-evidence` cannot name
//! `truck-topology` (the dependency direction is the reverse), so a stratum
//! carries the canonical carrier (from the structural recognizer) plus a
//! parameter-space box, not a topology handle. Trimming to the actual face
//! boundary (wires) is a later strata-reduction refinement.
//!
//! House rules H-1..H-8 apply.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]
