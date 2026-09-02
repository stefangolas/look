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
pub mod meshable;
pub mod pair_dispatch;
pub mod source_evidence;
pub mod ssi;
#[doc(hidden)]
pub mod ssi_fixtures;
pub mod ssi_types;

/// The SSI wave shim's shared shapes, re-exported at the crate root for the
/// look test target's reachability (BG-CK-P2-CONTRACT).
pub use ssi_types::{KrawczykCertificate3, SquareSystem3, TraceOutcome, TraceRefusal, TraceStep};
