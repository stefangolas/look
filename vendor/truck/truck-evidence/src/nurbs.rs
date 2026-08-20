//! BG-ENC-003-NURBS: enclosure for `NurbsCurve<Vector4>`.
//!
//! Scaffolded empty by the orchestrator so that `BG-ENC-003-BSPLINE` can run
//! concurrently with the ANA fan-out without either needing `lib.rs`; this
//! module is **not dispatchable yet** — it is blocked on BG-ENC-003-BSPLINE.
//!
//! The hull property holds in **homogeneous** coordinates only; the
//! projection of a hull is not the hull of the projection unless every weight
//! is positive. A non-positive weight is refused
//! (`EnvelopeCase::NonPositiveNurbsWeight`), never silently mis-enclosed.
