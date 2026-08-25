//! BG-SOL-S2-EXTRUDE — direct certified B-rep extrude of a planar arrangement.
//!
//! `extrude_profile` turns the material region(s) of an `Arrangement` (S1,
//! `truck-geometry/src/arrange.rs`) into a closed `Solid<Point3, Curve,
//! Surface>` with NO tool-body Boolean: the bottom/top caps (each carrying the
//! hole's wire as an inner boundary), the outer planar side faces, and the
//! single cylindrical hole wall — built combinatorially with SHARED vertex
//! instances and canonical surfaces. The second half of M1 (certified planar
//! construction, docs/SOLVER_FAMILY_PLAN.md §4 Phase 2 + §7): rectangle −
//! circle → arrangement → profile with hole → direct extrude → valid B-rep.
//!
//! Target API (plan §4 Phase 2):
//!
//! ```rust,ignore
//! pub fn extrude_profile(profile: &Arrangement, height: f64)
//!     -> Outcome<Solid<Point3, Curve, Surface>>;
//! ```
//!
//! v1 scope (documented in the packet): exactly one material region (winding
//! == 1 under the profile's declared loop orientations); PC = () (no pcurves).
//! House rules H-1..H-8 apply.
//!
//! Scaffolded empty; packet BG-SOL-S2-EXTRUDE fills it.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]
