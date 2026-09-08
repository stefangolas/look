//! BG-NUM: the certified numerical substrate.
//!
//! Three modules, all consuming the evidence algebra (`truck_base::evidence`)
//! and interval arithmetic (`crate::enclosure`):
//!
//! - **`cluster`** (BG-NUM-004) — certified ball-overlap clustering: connected
//!   components of certified ball overlap, each carrying a certified enclosing
//!   ball. Position-independent: never grid quantisation, never transitive
//!   closure of a nearness predicate.
//! - **`roots`** (BG-NUM-002) — certified univariate root isolation by
//!   Bernstein/Descartes subdivision. Every returned interval contains
//!   exactly one root; multiple roots (an even sign-change count that never
//!   resolves to one) are refused as `NumericallyUnresolved`, never reported
//!   as "no root".
//! - **`krawczyk`** (BG-NUM-003) — the Krawczyk operator: existence and
//!   uniqueness of a system's solution in a box, proven only on **strict**
//!   interior containment of the K image.
//!
//! Scaffolded empty (BG-ENC-004's offset.rs pattern): each module records its
//! contract and waits for its packet. House rules H-1..H-8 apply.

/// BG-NUM-004: certified ball-overlap clustering (topology-free core).
pub mod cluster;
/// CL-003-SWEEP-ENCLOSURE: the composite `EnclosureSurface` for the closed
/// whole-sweep value (`truck_geometry::constructive::SpineFrameSweep`). The
/// implementation lives in its own file at the crate root (this `#[path]`
/// declaration) so this packet and CL-000-SPLINE-ADMIT stay write-disjoint on
/// `enclosure.rs`; the impl is crate-wide once compiled here.
#[path = "../enclosure_sweep.rs"]
pub mod enclosure_sweep;
/// BG-NUM-003: the Krawczyk existence/uniqueness operator. Scaffolded empty;
/// the packet fills it.
pub mod krawczyk;
/// BIE-002-SSI4: the parallelotope continuation tracker (theory §3.3 θρ step)
/// — the certified tangent-frame continuation the restricted-pair solver uses
/// to track an interaction branch. Additive over the Krawczyk operator; no
/// geometry of its own.
pub mod parallelotope;
/// ADM-L5-RECIPROCAL (Theorem D): the certified reciprocal-power
/// polynomialization — `certified_reciprocal_power(W, p, target_error)`
/// returns the exact truncated polynomial `Q_m` and its certified geometric
/// tail bound. Pure polynomial mathematics; the F1 substrate ADM-003's volume
/// assembly brackets rational-face integrals with.
pub mod reciprocal;
/// BG-NUM-002: certified univariate root isolation (Bernstein/Descartes).
/// Scaffolded empty; the packet fills it.
pub mod roots;
/// CL-003-SWEEP-ENCLOSURE: the certified sweep-side σ_G helper — interval
/// bounds on the first fundamental form's four entries over the sweep's
/// windowed domain, composed from the sweep derivative enclosures.
pub mod sweep_sigma;
