//! BG-ENC-002-LINE: `EnclosureCurve` for the `Line` carrier.
//!
//! Scaffolding only. The carrier impl is the body of work packet BG-ENC-002-LINE;
//! this file is declared in `lib.rs` up front so that the six sibling
//! packets have disjoint write sets and can run in parallel without
//! colliding on one `pub mod` line. `plane.rs` is the reference pattern.
