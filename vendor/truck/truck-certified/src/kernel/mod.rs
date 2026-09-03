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
pub mod coons_patch;
/// The certificate-calculus engine (BG-KV2-201-S2A): Lemma 8.0's rho, the
/// generic square C1, the C2 tube, and frame construction. This is the wave-2
/// real engine over the landed interval core; the shim shapes it emits are
/// frozen in [`certs`].
pub mod engine;
pub mod evidence;
/// The machine-checked fixture kit — test support only.
#[doc(hidden)]
pub mod fixtures;
pub mod graph;
pub mod identity;
pub mod leaf;
pub mod leaf_extract;
/// The §6.3 maximal-minor algebra (BG-KV2-301-S03A): Theorem 6.4's `m` vector
/// (`m_j = (−1)^j det(DF with column j deleted)`) as a certified enclosure
/// over a per-box 3x4 Jacobian, with the `DF·m = 0` and `a·m` checkables.
pub mod minor_algebra;
pub mod patch;
pub mod rational;
pub mod residual;
/// The §7 R8/R9 square residuals (BG-KV2-202-S1A): the curve–surface system
/// (arity 3) and the one-chart curve–curve system (arity 2) over the S2A C1
/// seam, plus the 1-var homogeneous curve leaf they consume.
pub mod residuals_r89;
/// The Tier-1 loop-free certificate and the §9.3 R8 boundary-stratum seeds
/// (BG-KV2-301-S03A): the two-cone LP of Theorem 9.1 (cos-space cone
/// separation) and the R8 subdivision seeds over caller-supplied boundary
/// edges.
pub mod tier1;
