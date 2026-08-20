//! BG-ENC-004-ISC: `EnclosureCurve` for the `IntersectionCurve` decorator.
//!
//! Scaffolding only. **Not dispatchable yet** — the module records why, the
//! same way `offset.rs` does.
//!
//! `IntersectionCurve<C, S0, S1>` is a **curve**: `subs(t)` runs
//! `search_triple`, an iterative double projection of the leader curve's
//! point onto both surfaces. The BG-ENC-004 item names the only sound
//! enclosure shape — "enclose the leader, then inflate by the certified
//! Newton residual bound" — and that residual bound does not exist yet.
//! The leader hull alone is an under-estimation by construction (the true
//! point is the projection, not the leader point), and nothing in the
//! BG-ENC-001 interface certifies the projection travel for general
//! parametric surfaces: distance(point, surface) has no interval-evaluable
//! closed form the way a plane's signed distance does.
//!
//! What unlocks it: the certified whole-span deviation bound of BG-CE-002
//! (or a Krawczyk operator, BG-NUM-003), which is exactly a per-curve
//! "leader vs. truth" certificate. The `der` and `tangent_cone` halves ARE
//! composable today (the cross-normal formula in intervals off the
//! surfaces' `normal_cone`s and the leader's derivative hulls), so the
//! packet that finally lands here will be mostly composition plus the one
//! residual bound it waits for. The registry row carries the dependency.
