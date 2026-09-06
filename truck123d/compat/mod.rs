//! truck123d/compat — the text-to-cad corpus compat layer (PB-008-TTC-HARNESS).
//!
//! This directory is PB-008's `truck123d/compat/**` write set: the build123d-
//! compat harness over the vendored text-to-cad corpus (`corpus/ttc/`, spec
//! `TRUCK123D_PY_BRIDGE_SPEC.md` §8). The corpus rows name real corpus geometry
//! entries; the compat vocabulary they exercise is documented (and machine-
//! checked) by `docs/PY_BRIDGE_COMPAT_SURFACE.md`.
//!
//! Why it lives here and not in `src/`: the compat layer is harness, not
//! bridge surface. `lib.rs` owns the pyo3 bridge core and is not widened by
//! this packet; the module is included by the crate's integration suite
//! (`tests/ttc_harness.rs` declares it with `#[path = "../compat/mod.rs"]`) and
//! compiled into that test binary. Nothing in production imports it and nothing
//! under `corpus/` is imported by production code (scope decision 1).
//!
//! Responsibilities:
//!
//! * [`manifest`] — the corpus row manifest (`MANIFEST.json`): rows name a
//!   vendored tree, a `lib` geometry entry, its arguments and its stage
//!   (`canonical` vs `skipped`).
//! * [`skips`] — the machine-checked skip machinery (`SKIPS.json`): a skip
//!   reason vocabulary keyed by code, each resolving to the packet/program that
//!   unblocks it; every `skipped` manifest row must carry exactly one skip row
//!   whose reason resolves, and a reasonless skip is a harness failure.
//! * [`reference`] — the recorded OCC-derived reference geometry facts
//!   (`reference/*.json`) and the tolerance comparison a canonical run is
//!   graded against.
//! * [`runner`] — the plain process-per-script door (`corpus/ttc/door.py`,
//!   OCC baseline regime): runs each canonical row end to end producing STL +
//!   a report JSON, then asserts geometry facts against the reference.
//! * [`surface`] — the machine-check of `docs/PY_BRIDGE_COMPAT_SURFACE.md`
//!   against spec §8's seven compat-surface rows.
//!
//! Determinism: canonical rows are executed in manifest order; the report
//! schema and the fact comparison are fixed. H-1 discipline is kept here even
//! though this is test-support code: no panics reachable from geometry data.

#![cfg_attr(
    not(test),
    deny(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::todo,
        clippy::unimplemented,
        clippy::indexing_slicing
    )
)]

pub mod manifest;
pub mod reference;
pub mod runner;
pub mod skips;
pub mod surface;

use std::path::PathBuf;

/// The absolute path of the `corpus/ttc` directory in this worktree.
///
/// Compiled into the `truck123d` test binary, so `CARGO_MANIFEST_DIR` is the
/// `truck123d` crate directory and the corpus sits one level up.
pub fn corpus_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../corpus/ttc")
}
