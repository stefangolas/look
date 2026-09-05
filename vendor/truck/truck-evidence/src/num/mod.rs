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
/// BG-NUM-003: the Krawczyk existence/uniqueness operator. Scaffolded empty;
/// the packet fills it.
pub mod krawczyk;
/// BIE-002-SSI4: the parallelotope continuation tracker (theory §3.3 θρ step)
/// — the certified tangent-frame continuation the restricted-pair solver uses
/// to track an interaction branch. Additive over the Krawczyk operator; no
/// geometry of its own.
pub mod parallelotope;
/// BG-NUM-002: certified univariate root isolation (Bernstein/Descartes).
/// Scaffolded empty; the packet fills it.
pub mod roots;
