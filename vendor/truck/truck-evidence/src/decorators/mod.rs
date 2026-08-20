//! BG-ENC-004: enclosure impls for `truck-geometry`'s decorators.
//!
//! A decorator wraps another carrier and reports a derived geometry, so its
//! enclosure is a *composition*: it calls the inner carrier's `enclose` /
//! `enclose_der` and combines the resulting boxes. Nothing here evaluates a
//! parameterisation directly, which is what separates this module from the
//! analytic carriers in the crate root.
//!
//! Scaffolding only. Each submodule is the body of its own work packet and
//! currently holds a doc comment; the modules are declared here, and this
//! module in `lib.rs`, up front so that the sibling packets have disjoint
//! write sets and can run in parallel without colliding on one `pub mod` line
//! (the same reasoning as `afba979` for the six BG-ENC-002 carriers).

/// BG-ENC-004-EXTRUDED: `EnclosureSurface` for `ExtrudedCurve`.
pub mod extruded;
/// BG-ENC-004-OFFSET: `EnclosureSurface` for `Offset`. Blocked; see the module.
pub mod offset;
/// BG-ENC-004-PROCESSOR: `EnclosureSurface` for `Processor`.
pub mod processor;
/// BG-ENC-004-REVOLVED: `EnclosureSurface` for `RevolutedCurve`.
pub mod revolved;
