//! BG-ANA-001 — exactly solvable surface pairs.
//!
//! One submodule per analytic pair family. The **shared result type**
//! `AnalyticIntersection` (with `ExactCurve`) lives here, in the module root,
//! so that all eight families speak one vocabulary: no submodule defines a
//! private result enum. The type was designed and landed by the orchestrator
//! before any shard was dispatched, because eight workers given "define the
//! result type as you see fit" would define eight of them.
//!
//! BG-ANA-002 constrains every submodule: position classification is decided
//! by **exact predicates on the carrier parameters**, never by sampling the
//! surfaces, and no analytic pair may return a float-certified result — if it
//! could, the pair belongs in the general solver. The cells built here become
//! the ground-truth oracle for BG-NUM-003.

/// BG-ANA-001-COAX: coaxial pairs. Scaffolded empty; the packet fills it.
pub mod coaxial;
/// BG-ANA-001-EQRCYL: equal-radius cylinders with intersecting axes.
/// Scaffolded empty; the packet fills it.
pub mod equal_radius_cylinders;
/// BG-ANA-001-PARCYL: parallel-axis cylinders. Scaffolded empty; the packet
/// fills it.
pub mod parallel_cylinders;
/// BG-ANA-001-PCONE: plane × cone. Scaffolded empty; the packet fills it.
pub mod plane_cone;
/// BG-ANA-001-PCYL: plane × cylinder. Scaffolded empty; the packet fills it.
pub mod plane_cylinder;
/// BG-ANA-001-PP: plane × plane. Scaffolded empty; the packet fills it.
pub mod plane_plane;
/// BG-ANA-001-PS: plane × sphere. Scaffolded empty; the packet fills it.
pub mod plane_sphere;
/// BG-ANA-001-SS: sphere × sphere. Scaffolded empty; the packet fills it.
pub mod sphere_sphere;
