//! BG-NUM: the certified numerical substrate.
//!
//! Two modules, both consuming the evidence algebra (`truck_base::evidence`)
//! and interval arithmetic (`crate::enclosure`):
//!
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

/// BG-NUM-003: the Krawczyk existence/uniqueness operator. Scaffolded empty;
/// the packet fills it.
pub mod krawczyk;
/// BG-NUM-002: certified univariate root isolation (Bernstein/Descartes).
/// Scaffolded empty; the packet fills it.
pub mod roots;
