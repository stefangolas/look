#![cfg_attr(not(debug_assertions), deny(warnings))]
#![deny(clippy::all, rust_2018_idioms)]
#![deny(clippy::unwrap_used)]
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

//! The only sanctioned inari bridge (CC-000-CONTRACT, spine decision C3).
//!
//! The construct layer's interval universe is [`super::Interval`]
//! (`CertifiedInterval`); the `truck-evidence` universe is
//! `truck_evidence::enclosure::Interval` (an `inari` re-export). Two interval
//! universes exist and are bridged explicitly here — never silently, and only
//! here. The `inari` crate name appears nowhere in this crate's manifest or in
//! any `use` statement; the boundary types are reached only through the
//! `truck_evidence::enclosure` re-exports.
//!
//! **Soundness note.** Both universes are outward-rounded, so copying the
//! endpoints verbatim is an exact, order-preserving map that adds no width:
//! `lo = inf`, `hi = sup`. No rounding is performed by the bridge itself.
//!
//! **H-1.** This module carries no `unwrap`, no `expect`, and no `panic!`, and
//! adds no module-level `allow`.

use crate::construct::Interval;
use crate::kernel::patch::IBox;

/// Bridge an inari-world interval into the construct universe as an exact
/// lo/hi field copy (C3).
///
/// Both universes are outward-rounded, so the copy is order-preserving and
/// adds no width: the returned interval has `lo = i.inf()` and `hi = i.sup()`.
pub fn from_inari(i: truck_evidence::enclosure::Interval) -> Interval {
    Interval {
        lo: i.inf(),
        hi: i.sup(),
    }
}

/// Bridge a `truck-evidence` world-space [`truck_evidence::enclosure::Box3`]
/// into the kernel's const-N box type as an exact per-axis lo/hi field copy
/// (C3).
///
/// Each axis is copied verbatim from the box's outward-rounded endpoints, so
/// the mapping is order-preserving and adds no width.
pub fn box3_to_ibox(b: &truck_evidence::enclosure::Box3) -> IBox<3> {
    IBox {
        lo: [b.x.inf(), b.y.inf(), b.z.inf()],
        hi: [b.x.sup(), b.y.sup(), b.z.sup()],
    }
}
