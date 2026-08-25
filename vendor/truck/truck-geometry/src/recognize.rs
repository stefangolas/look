//! BG-SOL-P0-REC — the structural recognizer: produce a witness, not a type.
//!
//! `CanonicalCarrierWitness { ExactCanonical { carrier, map } |
//! Derived { carrier, provenance, map } | Unrecognized }`, where
//! `S_stored = S_canonical ∘ φ` is a certified relationship
//! (docs/SOLVER_FAMILY_PLAN.md §2). Coincidence (S5.0) is then a lookup on the
//! witness, not a re-solve. `CanonicalCarrier` reuses the `Surface`/`Curve`
//! enums' analytic arms (`truck-geometry/src/canonical.rs`).
//!
//! Target API (SOLVER_FAMILY_PLAN.md §4), to be filled by packet
//! BG-SOL-P0-REC:
//!
//! ```rust,ignore
//! pub enum CanonicalCarrierWitness {
//!     ExactCanonical { carrier: CanonicalCarrier, map: ParamMap },
//!     Derived { carrier: CanonicalCarrier, provenance: ConstructionWitness, map: ParamMap },
//!     Unrecognized,
//! }
//! pub fn recognize_curve(c: &Curve) -> CanonicalCarrierWitness;
//! pub fn recognize_surface(s: &Surface) -> CanonicalCarrierWitness;
//! ```
//!
//! The `map: ParamMap` comes from `truck_base::param_map` (moved by this
//! scaffold). House rules H-1..H-8 apply. The packet fills this file; the
//! scaffold declares the module and records the contract.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]
