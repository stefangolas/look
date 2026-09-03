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

//! The kernel-v2 normative defaults (spec §0.4), verbatim (BG-KV2-000-CONTRACT).
//!
//! **H-1.** The crate-level `#![deny(clippy::unwrap_used)]` in `lib.rs` covers
//! this module. The module carries no `unwrap`, no `expect`, and no `panic!`
//! calls, and adds no module-level `allow`.
//!
//! **D-constants.** This module holds the §0.4 normative defaults as consts.
//! The landed `DirectTolerance` (truck-geometry) is deliberately NOT touched
//! (build-spec decision 2): the two default sources coexist this program and
//! kernel-v2 code consumes only `kernel::config`.
//!
//! The trailing `// H-3` markers are the house-rule fixture opt-out: the
//! literals are the spec's own normative default values, not ad-hoc length
//! comparisons, so they are exempt from the absolute-length-literal gate.

/// Model-space representation gap (spec §0.4): the scale at which two model
/// points are treated as the same represented point.
pub const EPS_REP: f64 = 1e-9; // H-3: normative §0.4 default (representation gap)

/// Krawczyk contraction acceptance ceiling (spec §0.4): a residual-based
/// certificate is only issued when the contraction rate is at most `RHO_MAX`.
pub const RHO_MAX: f64 = 0.5;

/// Conditioning bound (spec §0.4): above `KAPPA_MAX` a frame is rebuilt rather
/// than certified.
pub const KAPPA_MAX: f64 = 1e6;

/// Subdivision cap (spec §0.4): the maximum recursion depth (3 D4
/// carve-out sites).
pub const DEPTH_MAX: u32 = 40;

/// Tier-2 direction retries (spec §0.4): how many alternative continuation
/// directions a chart may attempt.
pub const KA: u32 = 4;

/// Max deck traversals per edge (spec §0.4): the ceiling on how many period
/// crossings one edge may walk.
pub const DECK_MAX: i32 = 8;

/// Model-space agreement tolerance (spec §0.4): two positions agree when they
/// are within `TOL_POSITION`.
pub const TOL_POSITION: f64 = 1e-9; // H-3: normative §0.4 default (position agreement)

/// Parameter agreement tolerance (spec §0.4): parameter values agree, and C1
/// detection is decided, at `TOL_PARAMETER`.
pub const TOL_PARAMETER: f64 = 1e-11; // H-3: normative §0.4 default (parameter agreement / C1 detection)

/// Regularity floor (spec §0.4): the tolerance for `EG - F^2` being treated as
/// zero, i.e. the singular-map floor.
pub const TOL_JACOBIAN: f64 = 1e-12; // H-3: normative §0.4 default (regularity floor EG - F^2)

/// Tangency-claim tag (§10.3): an at-tolerance contact claim may only be
/// issued at `TOL_INTERSECTION`, never unified with an exact certificate.
pub const TOL_INTERSECTION: f64 = EPS_REP;
