//! BG-SOL-P0-SPAN — the lazy rational-Bézier span cache.
//!
//! Extracts, per carrier, the per-knot-span records that the broad phase and
//! the certified solvers both consume: each `SpanRecord` carries the span's
//! conservative bounding box and derivative hull plus its parameter window.
//! Shares the `BoundedPiece` abstraction with the BVH
//! (`truck-base/src/bvh.rs`, docs/SOLVER_FAMILY_PLAN.md §2).
//!
//! Target API (SOLVER_FAMILY_PLAN.md §4), to be filled by packet
//! BG-SOL-P0-SPAN:
//!
//! ```rust,ignore
//! pub struct SpanCache { /* per-carrier lazy extraction */ }
//! impl SpanCache {
//!     pub fn spans(&self, s: &Surface) -> Vec<SpanRecord>;
//! }
//! pub struct SpanRecord { pub bbox: Box3, pub derivative_hull: DerivativeBounds, /* knot range */ }
//! ```
//!
//! The plan's `Box3` is the broad-phase box, here `truck_base::bounding_box::`
//! `BoundingBox<Point3>` (see `bvh.rs`'s module doc). House rules H-1..H-8
//! apply. The packet fills this file; the scaffold declares the module and
//! records the contract.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]
