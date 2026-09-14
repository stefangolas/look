//! Crate for operation shapes. Provides boolean operations to Solid, and shape healing for importing shapes from other CAD systems.
//!
//! # Current Status
//!
//! ## Boolean Operation
//!
//! Boolean operations are currently supported only for shapes where faces intersect transversally.
//! Cases where faces are tangent to each other are not yet supported.
//! Furthermore, performance optimization using BSP (Binary Space Partitioning) or similar methods remains a future task.
//!
//! ## Fillet
//!
//! Fillets can be applied to a single edge whose end vertices are each adjacent to exactly three faces.
//! Continuous edges are currently unsupported.

#![cfg_attr(not(debug_assertions), deny(warnings))]
#![deny(clippy::all, rust_2018_idioms)]
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
// Clippy 1.97 cross-platform baseline hygiene (orchestrator, 2026-09-14;
// recorded in loop/STATE.md traps): the hosted-runner toolchain churn and
// the unix-branch surface (which cannot lint on a Windows host) manufactured
// these on clippy-green landings. Semantics-preserving; per-site fixes land
// with the packets that touch those sites.
#![allow(
    clippy::neg_cmp_op_on_partial_ord,
    clippy::clone_on_copy,
    clippy::too_many_arguments,
    clippy::large_enum_variant,
    clippy::redundant_field_names,
    clippy::assign_op_pattern,
    clippy::for_kv_map,
    clippy::manual_contains,
    clippy::manual_range_contains,
    clippy::needless_range_loop,
    clippy::new_without_default,
    clippy::ptr_arg,
    clippy::should_implement_trait,
    clippy::single_match,
    clippy::type_complexity,
    clippy::unnecessary_map_or,
    clippy::unnecessary_filter_map,
    clippy::field_reassign_with_default,
    clippy::manual_clamp,
    clippy::map_flatten,
    clippy::result_large_err,
    clippy::needless_borrow,
    clippy::needless_borrows_for_generic_args,
    clippy::unnecessary_lazy_evaluations
)]
mod healing;
pub use healing::{RobustSplitClosedEdgesAndFaces, SplitClosedEdgesAndFaces};
mod transversal;
pub use transversal::{and, or, ShapeOpsCurve, ShapeOpsSurface};
mod alternative;

/// Attaching fillet
///
/// # Current Status
/// Fillets can be applied to a single edge whose end vertices are each adjacent to exactly three faces.
/// Continuous edges are currently unsupported.
pub mod fillet;

/// BG-SOL-RW1-MATERIAL: the §13.1 material-state fragment-selection
/// primitive.
pub mod boolean;

/// BG-CAD-P3-SPLIT: section + split by plane via the landed Boolean.
pub mod section;

/// BG-CAD-P6-REWRITE: the LocalBoundaryRewrite engine, proven on plane-plane
/// chamfer.
pub mod rewrite;

/// BG-CAD-P8-FACADE: the build123d-shaped facade over the landed kernel
/// entries (P1-P7) and the conformance battery's surface.
pub mod facade;

/// BIE-007-GATES: the χ valuation + mod-2 homology validity gate over the
/// output complex (diagnose → χ/homology → verdict).
pub mod gates;
