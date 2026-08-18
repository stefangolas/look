# QUESTION — BG-TOL-001-GEOM-SPECIFIEDS (SPEC_GAP)

## What was attempted

Every site in the packet was migrated exactly per the site table and the
recipes, using one `ToleranceCtx::unscaled_legacy()` obtained at the top of
each function that contains at least one site:

- `circle.rs` (4 sites, all `param`): `parameter_division`, `search_nearest_parameter<P2>`,
  `search_parameter<P2>`, `search_parameter<P3>`.
- `hyperbola.rs` (3 sites, all `param`): `search_nearest_parameter<P2>`,
  `search_parameter<P2>`, `search_parameter<P3>`.
- `parabola.rs` (3 sites, all `param`): `search_nearest_parameter<P2>`,
  `search_parameter<P2>`, `search_parameter<P3>`.
- `line.rs` (1 site, `model`): `search_parameter` (via magnitude, because `Line<P>`'s
  `ControlPoint<f64> + Tolerance` bound does not expose `near_pt`; no bound added).
- `plane.rs` (5 sites, 4 `model` + 1 `param`): `include<BSplineCurve<Point3>>`,
  `include<NurbsCurve<Vector4>>` (sites 226 two-predicate line, 230 `param` weight, 234),
  `search_parameter<D2>`.
- `sphere.rs` (3 sites, 2 `model` + 1 `param`): `include`, `parameter_division`,
  `search_parameter` (site 216 `param` sine; site 211 left byte-for-byte with the
  `FIXME(BG-TOL-001)` comment directly above it).
- `torus.rs` (3 sites, all `model`): `search_parameter`, `search_nearest_parameter`
  (sites 174 and 180).

Added `vendor/truck/truck-geometry/tests/tolerance_specifieds.rs` with the two
required named tests (`canonical_sites_do_not_scale_with_the_model`,
`model_space_sites_do_scale_with_the_model`); truck-geometry has no
`autotests = false`, so the file is picked up automatically.

Verification, all as the packet specifies: the target test command passes with
results identical to baseline; `cargo clippy -p truck-geometry --all-targets
--no-deps -- -D warnings` is clean; `cargo check --workspace --all-targets`
passes. No existing test changed its result.

## The exact gap — the ratchet budget cannot hold the recipe

The recipe says: "Obtain the context once at the top of each function that
contains at least one site, as `let ctx = ToleranceCtx::unscaled_legacy();`.
Do not construct one per site."

There are **19** functions that contain at least one site (enumerated above).
The packet's `unscaled_legacy_budget` is **12**, and `The ratchet` section
states the ceiling is **29** — "17 call sites already in the tree plus at most
12 new ones" — and that "if you need many more you have constructed one per
site."

19 contexts is not "one per site" (22) and not "about 12": it is the true
count of functions with sites. There is no legitimate way to reduce it:

- Reusing a context across functions is impossible without either a signature
  parameter (explicitly forbidden) or a module-level static/helper that hides
  `unscaled_legacy(` behind one or two call sites — which would defeat the
  ratchet's whole purpose of counting migration sites.
- Omitting a context from any of the 19 functions would leave a site with no
  `ctx` to rewrite onto, violating the recipes.

A faithful commit therefore counts **36** call sites at HEAD against a ceiling
of **29**, and GATE-4 fails — V4 rejects the commit. `scripts/unscaled_legacy_ceiling.txt`
is explicitly not on this packet's write_allow, so I cannot raise it.

This looks like the exact "bad dispatch" shape `52d4552` was written to
prevent: the dispatch preflight accepted `17 + 12 = 29 <= 29`, but the recipe's
real requirement is 19, not 12. The budget (and therefore the ceiling) was
underestimated by 7.

## What I believe is needed

Either:
- raise `scripts/unscaled_legacy_ceiling.txt` to **36** (17 + 19) and re-run
  V4 on this commit, or
- amend the packet's `unscaled_legacy_budget` to **19** (and the ceiling to
  match) and re-dispatch.

The migration itself is complete, correct, and verified; no site was
reclassified to make it compile, and every canonical-vs-model judgement in the
table agreed with my reading.

## Pre-existing failures on this machine (confirmed at base f9fa761, not mine)

- `cargo fmt --check -p truck-geometry` fails on
  `truck-geometry/src/decorators/revolved_curve.rs:690` — a stray double blank
  line present at the base commit; the file is outside write_allow so I did not
  touch it (my changed files are rustfmt-clean).
- The packet's test command passes cleanly at the base commit; there are no
  pre-existing failures in the target suites (`--lib`, circle, plane, sphere,
  torus, hyperbola).
