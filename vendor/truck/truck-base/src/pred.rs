//! BG-SOL-P0-PRED — certified predicates with adaptive escalation.
//!
//! Topology-changing predicates (`orient2d`, event ordering, exact tangency,
//! endpoint membership) are never naked f64 comparisons
//! (docs/SOLVER_FAMILY_PLAN.md §2). Escalation discipline:
//! `CertifiedPred = Proven(...) | Unresolved(reason)`; the plan's §4 spelling
//! is `Certified<T> = Proven(T) | Unresolved(reason)`. S1 (arrange) inherits
//! it.
//!
//! Target API (SOLVER_FAMILY_PLAN.md §4), to be filled by packet
//! BG-SOL-P0-PRED:
//!
//! ```rust,ignore
//! pub enum CertifiedPred { Proven, Unresolved(UnresolvedWitness) }
//! pub fn orient2d(a: Point2, b: Point2, c: Point2) -> CertifiedPred;   // exact; adaptive escalation
//! ```
//!
//! House rules H-1..H-8 apply. The packet fills this file; the scaffold
//! declares the module and records the contract.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]
