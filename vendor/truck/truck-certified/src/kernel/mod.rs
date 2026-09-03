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

//! The kernel-v2 shim: the shared shapes, the refusing constructors, and the
//! machine-checked fixture kit (BG-KV2-000-CONTRACT).
//!
//! **H-1.** The crate-level `#![deny(clippy::unwrap_used)]` in `lib.rs` covers
//! this module and every submodule. The new files carry no `unwrap`, no
//! `expect`, and no `panic!` calls, and add no module-level `allow`.
//!
//! **D-shim.** Types and refusing constructors only. Any method that would
//! evaluate, solve, isolate, or certify NUMERICALLY refuses with a named
//! `RefusalKind` (or returns `RefusalKind`-carrying data for later use). This
//! module freezes the kernel-v2 shapes; the wave packets (BG-KV2-1xx/2xx/3xx/4xx)
//! implement against it and never restate it.
//!
//! **D-reuse.** [`Interval`] and [`SignCert`] alias the landed
//! `formal/exact.rs` primitives — zero new manifest edges (no inari). The
//! landed refusal vocabularies (`truck_base::evidence::Refusal`,
//! `contract::Refusal`) are NOT widened and NOT re-exported through this
//! module.
//!
//! **D-spelling.** The spec's §16 spellings are used INSIDE this module
//! (`Refusal`, `Arc`, `Sheet`, `Node`, ...). At the crate root only
//! `kernel::evidence::Refusal` is re-exported, under the name
//! [`crate::KernelRefusal`] (avoiding `contract::Refusal` / base `Refusal`
//! ambiguity); `ClaimVerdict`, `Construction`, `ResidualId`,
//! `CertifiedPatch`, `IBox`, and `PointCert` are also crate-root re-exports
//! (none collide). `kernel::graph::Arc<const N>` shadows `std::sync::Arc`
//! module-locally — acceptable, noted in `graph.rs`. `Frame<const N>` does not
//! collide (`Frame3` lives in truck-geometry, a different crate).
//!
//! **D-fixtures-public.** [`fixtures`] is `#[doc(hidden)] pub`: test support
//! only, excluded from the certified API surface, but reachable by wave
//! workers' integration tests through the crate's public path.

/// The certified-interval primitive of the kernel (D-reuse): aliases the
/// landed `CertifiedInterval`.
pub type Interval = crate::formal::exact::CertifiedInterval;
/// The certified-sign primitive of the kernel (D-reuse): aliases the landed
/// `CertifiedSign`.
pub type SignCert = crate::formal::exact::CertifiedSign;

pub mod certs;
pub mod config;
pub mod evidence;
/// The machine-checked fixture kit — test support only.
#[doc(hidden)]
pub mod fixtures;
pub mod graph;
pub mod identity;
pub mod leaf;
pub mod patch;
pub mod residual;
