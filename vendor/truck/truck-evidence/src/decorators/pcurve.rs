//! BG-ENC-004-PCURVE: `EnclosureCurve` for the `PCurve` decorator.
//!
//! Scaffolding only. The impl is the body of work packet BG-ENC-004-PCURVE;
//! this file is declared in `mod.rs` up front so that the sibling decorator
//! packets have disjoint write sets and can run in parallel.
//!
//! `PCurve<BSplineCurve<Point2>, S>` is a **curve** (a parameter-space
//! B-spline composed with a surface), so the impl is `EnclosureCurve`, not
//! `EnclosureSurface`: hull the parameter curve in 2D by the landed
//! BSPLINE machinery, then take the inner surface's `enclose` over that
//! parameter box — a pure composition, the same shape as the three landed
//! decorators. `der`/`der2`/`der3` compose by the chain rule over
//! `enclose_der` boxes of the surface.
