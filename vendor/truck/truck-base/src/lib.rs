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
