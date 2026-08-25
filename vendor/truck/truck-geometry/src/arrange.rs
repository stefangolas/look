//! BG-SOL-S1-ARRANGE — the 2-D planar arrangement over analytic profiles.
//!
//! Turns a closed analytic profile (`Curve::Line`/`Curve::Circle` in the
//! plane) into a certified 2-D subdivision: vertices, half-edges and regions
//! with winding numbers. The critical path to M1 (certified planar
//! construction, docs/SOLVER_FAMILY_PLAN.md §7): rectangle − circle →
//! arrangement → profile with hole → direct extrude.
//!
//! Builds on the LANDED Phase-0 API: `truck_base::pred::orient2d` (the exact
//! crossing/winding predicate), `truck_base::contact::CurveContact` (the event
//! vocabulary S1 and the Contact Layer share), `truck_base::bounding_box::`
//! `BoundingBox<Point2>` (the domain box), and `truck_geometry::recognize` (to
//! read the analytic carriers off `Curve`).
//!
//! Target API (plan §4 Phase 1):
//!
//! ```rust,ignore
//! pub struct Arrangement {
//!     pub vertices: Vec<ArrVertex>,
//!     pub half_edges: Vec<ArrHalfEdge>,
//!     pub regions: Vec<ArrRegion>,
//! }
//! pub fn arrange(profile: &[Curve], domain: Option<BoundingBox<Point2>>) -> Outcome<Arrangement>;
//! ```
//!
//! v1 scope (documented in the packet): analytic Line/Circle profiles only;
//! exactly-representable (dyadic) vertices; the algebraic intersection-vertex
//! case is a documented refusal. House rules H-1..H-8 apply.
//!
//! Scaffolded empty; packet BG-SOL-S1-ARRANGE fills it.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]
