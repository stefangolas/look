//! BG-SOL-P0-PRED — the `CurveContact` ontology, defined once in 2-D and
//! reused by 3-D (docs/SOLVER_FAMILY_PLAN.md §2).
//!
//! Contact has `dimension` (0D point / 1D arc / 2D region) and event kinds
//! (transverse, tangency, endpoint touch, coincident interval, identical
//! carrier). Getting it right in S1 makes S5 conceptually familiar rather than
//! a second paradigm. The types live in `truck-base` so both S1 (`arrange` in
//! truck-geometry) and the Contact Layer (`contact` in truck-evidence) can
//! name them without either depending on the other.
//!
//! Target API (SOLVER_FAMILY_PLAN.md §4), to be filled by packet
//! BG-SOL-P0-PRED:
//!
//! ```rust,ignore
//! pub enum ContactDimension { Point0, Arc1, Region2 }
//! pub enum ContactEventKind { Transverse, Tangency, EndpointTouch, CoincidentInterval, IdenticalCarrier }
//! pub struct CurveContact { pub dimension: ContactDimension, pub kind: ContactEventKind, /* params */ }
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
