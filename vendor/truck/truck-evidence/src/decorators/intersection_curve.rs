//! BG-ENC-004-ISC: `EnclosureSurface` for the `IntersectionCurve` decorator.
//!
//! Scaffolding only. The impl is the body of work packet BG-ENC-004-ISC; this
//! file is declared in `mod.rs` up front so that the sibling decorator packets
//! have disjoint write sets and can run in parallel.
//!
//! The one rule that distinguishes this decorator from its siblings, from the
//! BG-ENC-004 item itself: **the leader polyline is an approximation, never an
//! exact curve.** Enclose the leader, then inflate by the certified Newton
//! residual bound; an enclosure that treats the leader as exact is a
//! BG-ENC-001 under-estimation by construction.
