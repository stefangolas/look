//! BG-ENC-004-OFFSET: `EnclosureSurface` for the `Offset` decorator.
//!
//! Scaffolding only, and — unlike its three siblings — **not currently
//! dispatchable**. `Offset<T, N>` is not "a surface pushed along its normal by
//! a distance": it is the pointwise sum `S(u, v) + N(u, v)` of two parametric
//! surfaces, and `truck-geometry`'s only `ParametricSurface` impl for it
//! requires `N: ParametricSurface<Point = C::Vector>` — the offset field is
//! *vector*-valued. `EnclosureSurface` is bounded `ParametricSurface<Point =
//! Point3>`, so `N` can never be an `EnclosureSurface` and the composition
//! this module wants cannot be spelled with the BG-ENC-001 interface as it
//! stands. Enclosing the canonical field `NormalField<S, F>` instead needs an
//! enclosure for the *unit normal* field (the shape operator, hence curvature)
//! and one for `F: ScalarFunctionD2`. Both are new interface surface in
//! `enclosure.rs`, which makes BG-ENC-004-OFFSET a design item, not a
//! mechanical one. See the BG-ENC-004 spec section.
