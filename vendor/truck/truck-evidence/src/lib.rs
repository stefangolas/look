//! Certified evaluation substrate for the B-rep generation kernel.
//!
//! This crate is the reference implementation specified by
//! `docs/GENERATION_KERNEL_BUILD_SPEC.md` P-6: it establishes the pattern that
//! every later kernel item copies. It implements
//!
//! - **BG-EVD-001** — the evidence algebra, which now lives in
//!   `truck_base::evidence` and is re-exported here (`outcome`): `Outcome<T>`,
//!   `Certified<T>`, `Refusal`, `Certificate` and the §4 accumulation rules.
//!   The algebra moved to `truck-base` (BG-S0-001) so that `truck-geotrait`'s
//!   `IncludeCurve` can return `Outcome<bool>` without a geotrait→evidence
//!   dependency cycle;
//! - **BG-ENC-001** — the enclosure interface (`enclosure`): `Interval`,
//!   `Box3`, `DirCone` and the `EnclosureCurve`/`EnclosureSurface` traits;
//! - **BG-ENC-002 for `Plane`** (`plane`) — the reference carrier impl;
//! - the shared sampling harness (`harness`) so BG-ENC-001's soundness test is
//!   written once rather than once per carrier.
//!
//! House rules H-1..H-7 (spec §0) apply throughout.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

/// BG-ENC-001: the enclosure interface.
pub mod enclosure;
/// Shared sampling-soundness harness (BG-TEST of BG-ENC-001).
pub mod harness;
/// BG-EVD-001: the outcome/evidence algebra, re-exported from `truck-base`.
pub use truck_base::evidence as outcome;
/// BG-ENC-002 reference: `EnclosureSurface for Plane`.
pub mod plane;

pub use enclosure::{Box3, DirCone, EnclosureCurve, EnclosureSurface, Interval};
pub use truck_base::evidence::{Budget, Certificate, Certified, Outcome, Refusal};
