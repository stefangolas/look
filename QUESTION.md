# QUESTION — BG-TOL-001-GEOM-DECORATORS (status: SPEC_GAP)

## The blocker

Two rows of the site table cannot be migrated the way the table writes them:

- `decorators/offset/curve.rs` — `search_parameter` (table line 127):
  `ctx.near_points(self.subs(t), point)` does not typecheck. `near_points` is
  bounded `P: MetricSpace<Metric = f64>` and the enclosing
  `impl<C, N, P, V> SearchParameter<D1> for Offset<C, N>` binds only
  `P: ControlPoint<f64, Diff = V> + Copy + Tolerance`.
- `decorators/offset/surface.rs` — `search_parameter` (table line 170): same
  shape, same missing bound on
  `impl<S, N, P, V> SearchParameter<D2> for Offset<S, N>`.

Confirmed by the compiler (E0277, `the trait `MetricSpace` is not implemented
for `P``). Widening the impl's generic bound is forbidden by this packet, and
`near_pt` is `Point3`-only while `P` here is the unconstrained point type of a
2D-or-3D offset geometry. I therefore did not rewrite these two sites and left
them verbatim under `FIXME(BG-TOL-001, GENERIC_BOUND)` — the exact resolution
the GEOM-NURBS shard used for the identical problem
(`vendor/truck/truck-geometry/src/nurbs/{bspcurve,bspsurface,nurbscurve,nurbssurface}.rs`).

## Consequences

- Context budget: 14 `unscaled_legacy()` calls are asserted; the honest truth
  is 12. The two offset `search_parameter` functions hold only their deferred
  site, and a context in a function that has only deferrals is forbidden.
- `sites_migrated` is 26, `sites_deferred` is 3 (2 GENERIC_BOUND + 1 DIMENSION).

## Second finding: packet-internal contradiction in the test spec

Requirement 2 (`the_deferred_dimension_site_carries_a_fixme`) demands that
`decorators/rbf_surface/contact_circle.rs` contain **no** `ToleranceCtx` at
all. But the site table (same packet) migrates `try_new` in that exact file
(contact_circle.rs:61), which introduces one `ToleranceCtx`. Both cannot hold.
I followed the site table and wrote the test to assert exactly one
`FIXME(BG-TOL-001, DIMENSION)` and that the deferred `next_point` function
carries no context.

## What was verified and passes

All six anchors matched exactly. All four "Done when" commands pass, plus the
decorator-exercising integration tests — so the deliberate Euclidean tightening
moved no existing test, and the ratchet stands at 87/111 after this commit.

## What I need from the orchestrator

Whether the two offset `near_points` sites are to be (a) kept deferred with
`FIXME(BG-TOL-001, GENERIC_BOUND)` and the budget re-stated as 12, or (b) the
site table corrected to a form that typechecks without widening a generic bound.
And which side of the contact_circle test contradiction is authoritative.
