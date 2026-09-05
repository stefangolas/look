//! BIE-007-GATES — the output-complex validity gate layer.
//!
//! The final gate layer of the Certified Interaction Engine program: an
//! Euler-characteristic valuation plus a mod-2 (Z₂) homology check over the
//! finite output complex, layered BESIDES the landed manifold diagnostics
//! (the pipeline is `diagnose → χ/homology → verdict`). The landed
//! `manifold::diagnose` stays the first gate stage; this module runs beside
//! it and refuses a mismatch as a typed `Outcome` error, never a warning.
//!
//! The implementation is ~100 lines of dense Z₂ linear algebra (bitmask rows,
//! Gaussian elimination mod 2); no homology dependency is added.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

pub mod homology;

pub use homology::{
    chi_homology_gate, mod2_homology, BettiNumbers, GateReport, GateVerdict, HomologyData,
};
