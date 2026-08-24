//! Scaffold filled by BG-FID-001; bridge lemmas L-TUBE/L-COVERING/L-SEPARATES/L-FEDERER-PATCH/L-COVERAGE remain open.
//! (iv-a) one-sheet on curves lands here (BG-FID-008); the whole-span
//! isotopy conditions (i)-(iv-a) for curve components land here (BG-FID-003).
//! `rep` (BG-FID-005) lands here: the emitted geometry path that approximates
//! one exact CURVE component and discharges (iv-b) on its own certified
//! partition. The SURFACE case — the surface rep (REP-SRF-001), the surface
//! (iv-b) discharge and the surface double-sheet negative test — lands here
//! with BG-FID-005-SRF (`rep_surface` + the per-cell surface (iv-b)
//! discharge).

pub mod isotopy;
pub mod lfs;
pub mod one_sheet;
pub mod rep;
