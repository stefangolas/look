//! Geometrical structs: knot vector, B-spline and NURBS

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
use serde::{Deserialize, Serialize};
use std::{fmt::Debug, ops::Bound};
use truck_base::bounding_box::Bounded;

const INCLUDE_CURVE_TRIALS: usize = 100;
const PRESEARCH_DIVISION: usize = 50;

/// re-export `truck_base`
pub mod base {
    pub use truck_base::{
        assert_near, assert_near2, bounding_box::BoundingBox, cgmath64::*, hash, hash::HashGen,
        prop_assert_near, prop_assert_near2, tolerance::*,
    };
    pub use truck_geotrait::*;
}
/// Declares the nurbs
pub mod nurbs;

/// Enumerats `Error`.
pub mod errors;

/// Declares the specified gememetric items: Plane, Sphere, and so on.
pub mod specifieds;

/// Declares some decorators
pub mod decorators;

/// The canonical curve and surface model (BG-CE-006): `Curve` and `Surface`
/// with first-class analytic carriers, owned by this crate.
pub mod canonical;

/// BG-SOL-P0-REC: the structural recognizer (a witness, not a type).
/// Scaffolded empty; the packet fills it.
pub mod recognize;

/// BG-SOL-P0-SPAN: the lazy rational-Bézier span cache. Scaffolded empty; the
/// packet fills it.
pub mod span;

/// BG-SOL-S1-ARRANGE: the certified planar arrangement over analytic profiles.
/// Scaffolded empty; the packet fills it.
pub mod arrange;

/// BG-CG-000-CONTRACT: the constructive geometry contract skeleton
/// (`SpineFrameRecipe`, frame/profile laws, sampling policy, errors).
/// Scaffolded with stub bodies; later CG packets fill them.
pub mod constructive;

/// re-export all modules.
pub mod prelude {
    use crate::*;
    pub use base::*;
    pub use canonical::*;
    pub use decorators::*;
    pub use errors::*;
    pub use nurbs::*;
    pub use rbf_surface::*;
    pub use specifieds::*;
}
