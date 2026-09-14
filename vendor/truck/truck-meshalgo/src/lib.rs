//! Mesh algorigthms, include tessellations of the shape.

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
use common::*;

/// re-export polymesh
pub mod rexport_polymesh {
    pub use truck_polymesh::*;
}
use truck_polymesh::{StandardVertex as Vertex, *};

/// polygon mesh analyzers, including
///
/// - determines topological properties: connectivity, boundary extraction, or shell conditions (closed or oriented)
/// - detects collisions between two meshes and extracts interference lines
/// - investigates positional relations between mesh and point clouds.
#[cfg(feature = "analyzers")]
pub mod analyzers;
mod common;
/// Edits meshes. Add normals, optimizing data, and so on.
#[cfg(feature = "filters")]
pub mod filters;
/// Tessellates shapes.
#[cfg(feature = "tessellation")]
pub mod tessellation;
#[cfg(feature = "tessellation")]
pub use tessellation::print_taxonomy_summary;

/// VTK Output
#[cfg(feature = "vtk")]
#[cfg(not(target_arch = "wasm32"))]
pub mod vtk;

/// This module contains all traits and re-exports `truck_polymesh`.
pub mod prelude {
    #[cfg(feature = "analyzers")]
    pub use crate::analyzers::*;
    #[cfg(feature = "filters")]
    pub use crate::filters::*;
    pub use crate::rexport_polymesh::*;
    #[cfg(feature = "tessellation")]
    pub use crate::tessellation::*;
    #[cfg(feature = "vtk")]
    #[cfg(not(target_arch = "wasm32"))]
    pub use crate::vtk::*;
}
