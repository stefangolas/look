//! BG-ENC-003-BSPLINE: `EnclosureCurve for BSplineCurve<Point3>`.
//!
//! Scaffolded empty by the orchestrator so that this packet, `BG-ENC-003-NURBS`
//! and the ANA fan-out have disjoint write sets; the packet replaces this
//! comment. The technique is fixed by the spec and is the whole item: the
//! **convex-hull property** — over a knot span the curve lies in the convex
//! hull of its control points — used via knot insertion on the sub-curve over
//! `tt`, never naive interval arithmetic. The tangent cone comes off the
//! **hodograph** (`BSplineCurve::derivation`), `None` when the hull of its
//! control points contains 0.
