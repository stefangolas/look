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
/// Redefines vectors, matrices or points with scalar = f64.
pub mod cgmath64;
/// Additional traits for cgmath
pub mod cgmath_extend_traits;
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
/// Setting Tolerance
pub mod tolerance;
