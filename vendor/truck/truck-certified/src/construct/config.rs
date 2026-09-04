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

//! The CC normative config constants (spine decision C6, the
//! `kernel/config.rs` pattern).
//!
//! **H-1.** This module carries no `unwrap`, no `expect`, and no `panic!`, and
//! adds no module-level `allow`.
//!
//! The trailing `// H-3` markers are the house-rule fixture opt-out: the
//! literals are the spine's own normative default values (decision C6), not
//! ad-hoc length comparisons, so they are exempt from the absolute-length-
//! literal gate.

/// The P1 exact rational path threshold (spine decision C6): collocation sizes
/// at or below this count take the exact rational path; the interval fast path
/// serves above it.
pub const CC_N_EXACT: usize = 64;

/// The regularity margin floor (spine decision C6): `σ` margins certify only
/// above `CC_ETA_J`.
pub const CC_ETA_J: f64 = 1e-12; // H-3: normative C6 default (regularity margin floor)

/// The projection determinant margin (spine decision C6): P3 projection
/// determinants certify only above `CC_ETA_PI`.
pub const CC_ETA_PI: f64 = 1e-12; // H-3: normative C6 default (projection determinant margin)

/// The clearance margin `μ` (spine decision C6): P5 clearance certifies a gap
/// of at least `CC_MU_CLEAR`.
pub const CC_MU_CLEAR: f64 = 1e-9; // H-3: normative C6 default (clearance margin)

/// The subdivision depth cap (spine decision C6): the maximum recursion depth
/// of the construction subdivision.
pub const CC_DEPTH_MAX: u32 = 40;
