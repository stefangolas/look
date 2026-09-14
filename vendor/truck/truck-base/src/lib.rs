//! Basic structs and traits: importing cgmath, curve and surface traits, tolerance

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
/// Defines bounding box
pub mod bounding_box;
/// BG-SOL-P0-BVH: the broad-phase BVH and its `BoundedPiece` abstraction.
pub mod bvh;
/// Redefines vectors, matrices or points with scalar = f64.
pub mod cgmath64;
/// Additional traits for cgmath
pub mod cgmath_extend_traits;
/// BG-SOL-P0-PRED: the 2-D `CurveContact` ontology (shared by S1 and the
/// Contact Layer).
pub mod contact;
/// Utilities for performing calculations related to differentiation
pub mod ders;
/// Utility
pub mod entry_map;
/// BG-EVD-001: the outcome/evidence algebra (§4 of the B-rep generation formal
/// system). Lives here rather than in `truck-evidence` because `truck-geotrait`
/// is a leaf both geometry and modeling build on, and its `IncludeCurve` trait
/// returns `Outcome<bool>` (BG-S0-001); a geotrait→evidence dependency would
/// cycle. `truck-evidence` re-exports this module.
pub mod evidence;
/// Deterministic hash functions
pub mod hash;
/// ID structure with `Copy`, `Hash` and `Eq` using raw pointers
pub mod id;
pub mod newton;
/// BG-SOL-P0-REC: the certified parameter correspondence φ (moved from
/// `truck-evidence/src/deviation.rs` so `truck-geometry`'s recognizer can
/// name it). `truck-evidence` re-exports it.
pub mod param_map;
/// BG-SOL-P0-PRED: certified predicates with adaptive escalation (`orient2d`).
pub mod pred;
/// Setting Tolerance
pub mod tolerance;
