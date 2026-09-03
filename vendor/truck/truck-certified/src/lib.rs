//! Certified constructive geometry substrate: formal pipeline, quotient domain, evidence.

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

pub mod certified_map;
pub mod contract;
pub mod domain;
pub mod formal;
pub mod hull;
pub mod kernel;
pub mod meshable;
pub mod pair_dispatch;
pub mod source_evidence;
pub mod ssi;
#[doc(hidden)]
pub mod ssi_fixtures;
pub mod ssi_trace;
pub mod ssi_types;

/// The SSI wave shim's shared shapes, re-exported at the crate root for the
/// look test target's reachability (BG-CK-P2-CONTRACT).
pub use ssi_types::{KrawczykCertificate3, SquareSystem3, TraceOutcome, TraceRefusal, TraceStep};

/// The kernel-v2 wave workers' import surface, re-exported at the crate root
/// (BG-KV2-000-CONTRACT). `kernel::evidence::Refusal` is re-exported only as
/// [`KernelRefusal`], avoiding the `contract::Refusal` / base `Refusal`
/// ambiguity.
pub use kernel::certs::PointCert;
/// The kernel-v2 refusal vocabulary's [`Refusal`](kernel::evidence::Refusal)
/// under its non-colliding crate-root spelling.
pub use kernel::evidence::Refusal as KernelRefusal;
pub use kernel::evidence::{ClaimVerdict, Construction};
pub use kernel::patch::{CertifiedPatch, IBox};
pub use kernel::residual::ResidualId;
