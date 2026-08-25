//! BG-SOL-P0-BVH — the certified solver family's broad-phase substrate.
//!
//! The BVH and the Bézier-span cache (`truck-geometry/src/span.rs`) share one
//! abstraction, `BoundedPiece` (`bbox`, `derivative_bounds`, `subdivide`).
//! Analytic faces, Bézier surface spans, Bézier curve spans, trim segments and
//! intersection candidates all enter the same broad phase:
//! BREP → faces → carrier spans → BVH nodes → candidate span pairs →
//! certified solver (docs/SOLVER_FAMILY_PLAN.md §2, §4).
//!
//! This module is the **broad-phase box**: a plain `f64` axis-aligned box. It
//! deliberately does NOT use `truck-evidence`'s certified `Box3` — `truck-base`
//! has no `inari` dependency, and the broad phase culls candidates; it never
//! certifies. The certified stage later converts conservative enclosures into
//! this type (`.inf()`/`.sup()` endpoints) when it feeds the BVH.
//!
//! Target API (SOLVER_FAMILY_PLAN.md §4), to be filled by packet
//! BG-SOL-P0-BVH:
//!
//! ```rust,ignore
//! pub struct DerivativeBounds { pub first: BoundingBox<Point3>, pub second: BoundingBox<Point3> }
//! pub trait BoundedPiece {
//!     fn bbox(&self) -> BoundingBox<Point3>;
//!     fn derivative_bounds(&self) -> DerivativeBounds;
//!     fn subdivide(&self) -> Vec<Self> where Self: Sized;
//! }
//! pub struct Bvh<P: BoundedPiece> { /* flat node array, contiguous leaves */ }
//! impl<P: BoundedPiece> Bvh<P> {
//!     pub fn build(pieces: &[P]) -> Self;
//!     pub fn candidate_pairs(&self, other: &Self) -> Vec<(usize, usize)>;   // box overlap
//! }
//! ```
//!
//! House rules H-1..H-8 apply. The packet fills this file; the scaffold
//! declares the module and records the contract.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]
