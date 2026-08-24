# BG-AUDIT-001 — remediation tracking

Campaign: parallel remediation of every finding in `BG-AUDIT-001.md` / `.json`.
Base: audit snapshot `f919228`; every fix is written against current
`integration/kernel-bg` (HEAD `d34206e` at campaign start).

State vocabulary: `PENDING` `RUNNING` `WORKER_DONE` `VERIFIED` `LANDED`
`ALREADY_FIXED` `OWNER_BLOCKED`.

| finding | fix packet | state | worker commit | verifier | landed commit | regression |
|---|---|---|---|---|---|---|
| AUD-001 | BG-AUD-FIX-001 | LANDED | d1355c7 | ACCEPTED | 1844cfe | sphere_normal_cone_wide_azimuth_contains_all_normals |
| AUD-016 | BG-AUD-FIX-001 | LANDED | d1355c7 | ACCEPTED | 1844cfe | sphere_immersion_lower_bound_is_directed |
| AUD-002 | BG-AUD-FIX-002 | LANDED | 794b4e1 | ACCEPTED | d734d2a | route1_degree0_half_span_endpoint_deviation_refuses |
| AUD-003 | BG-AUD-FIX-003 | LANDED | 8bdf204 | ACCEPTED | 70d98bb | wedge_singular_midpoint_normal_refuses |
| AUD-004 | BG-AUD-FIX-004 | OWNER_BLOCKED | b48e2b7 | Phase A: no witness | — | — |
| AUD-007 | BG-AUD-FIX-005 | LANDED | 8f6f71e | ACCEPTED | f98d441 | wedge_slope_lower_bound_is_conservative_at_small_margins |
| AUD-005 | BG-AUD-FIX-006 | LANDED | 622a8ac | ACCEPTED | 2bc77ee | revoluted_curve_nonconformal_transform_is_placed |
| AUD-009 | BG-AUD-FIX-006 | LANDED | 622a8ac | ACCEPTED | 2bc77ee | full_circle_conversion_antipode_is_finite |
| AUD-006 | BG-AUD-FIX-007 | LANDED | 44abfed | ACCEPTED | b452237 | sectional_curve_vcut_u_half_box_does_not_panic |
| AUD-010 | BG-AUD-FIX-010 | LANDED | f893d9c | ACCEPTED | f1ed436 | cone_include_holds_pointwise_on_both_nappes |
| AUD-013 | BG-AUD-FIX-010 | LANDED | f893d9c | ACCEPTED | f1ed436 | sphere_search_nearest_parameter_center_is_none |
| AUD-017 | BG-AUD-FIX-010 | LANDED | f893d9c | ACCEPTED | f1ed436 | conic_containment_scale_invariant |
| AUD-011 | BG-AUD-FIX-009 | LANDED | c0d9c7d | ACCEPTED | 4b071ac | torus_normal_uder_matches_finite_difference |
| AUD-012 | BG-AUD-FIX-009 | LANDED | c0d9c7d | ACCEPTED | 4b071ac | contact_points_singular_frame_refuses |
| AUD-008 | BG-AUD-FIX-008 | RUNNING | | | | |
| AUD-015 | BG-AUD-FIX-008 | RUNNING | | | | |
| AUD-014 | BG-AUD-FIX-011 | RUNNING | | | | |

## Owner decisions made during this campaign (recorded in the packets)

- **AUD-003**: the whole-edge interval wedge certificate is not expressible
  through the generic `S: ParametricSurface + ParametricSurface3D +
  SearchParameter` bounds. NaN guard lands (mandatory); sampling strengthened
  to endpoints + midpoint; the claim scope is explicitly the sampled
  parameters. The whole-span piece is an API-bound owner item, recorded in the
  packet.
- **AUD-004**: Phase A (real failing witness) first; if no witness exists, the
  outcome is `OWNER_BLOCKED` with an invariant note — never a forced
  center-term rewrite.
- **AUD-010**: the cone is DOUBLE nappe (spec pins `v` unbounded both ways);
  predicates are fixed for `v < 0`, not restricted to a single nappe.
- **AUD-014**: absence of a pcurve is not-applicable, not a hold —
  `SameParameter` stays `Unknown` when no trace exists. The `pre_cut` drop of
  the pcurve is a decided spec contract and is untouched.
- **AUD-016**: C1 hardening only (directed rounding); no false certificate was
  demonstrated.

## Campaign facts (append as they land)

- All 17 findings confirmed present at HEAD `d34206e` before packet writing.
- Machine-checked witnesses: AUD-001 (corner half-angle 33.37° vs interior
  42.31°), AUD-007 (s=1e-8 over-reports +49%), AUD-016 (rtn product
  0.4994291492576638 vs directed 0.4994291492576637).
- 11 repair packets written and validated (`gen_packet.check` green) on
  2026-08-24.
- **BG-AUD-FIX-001 LANDED** at `1844cfe` (worker `d1355c7`, verify ACCEPTED at
  base `e21cbeb`). Everything-cone for azimuth span `>= π`; directed interval
  immersion bound. AUD-001 + AUD-016 closed.
- **BG-AUD-FIX-002** worker done (`794b4e1`): union of the original diff
  spline's `subs(a)`/`subs(b)` into every piece hull PLUS a conservative
  endpoint-magnitude refusal (the union alone leaves the piece hull spanning
  both values, pinning its norm infimum at 0 → NumericallyUnresolved, so the
  ForwardToleranceExceeded verdict needs the explicit endpoint check). Sound,
  refuses-only. **LANDED** at `d734d2a` (verify ACCEPTED at `e21cbeb`);
  truck-topology same_parameter re-run 5/5 green post-merge. AUD-002 closed.
- **BG-AUD-FIX-003** worker done (`8bdf204`): NaN guard (hand-computed normal
  magnitude refuses non-finite/zero), sampling at `t0`/`t_mid`/`t1`, scoped
  sampled-claim doc. Test-only delegating `Surface` enum to combine Cone+Plane
  in one shell. Verify queued.
- **BG-AUD-FIX-004** = **OWNER_BLOCKED** after Phase A (`b48e2b7`, no code).
  No real failing witness exists: `certify_cell` builds Q as the widened hull
  of the endpoint seeds, so a cell certifies only when the parameter path is
  monotone in every surface parameter; non-monotone paths always refuse.
  Evidence: 764+ cells with interior parameter escapes across three real
  carrier families (tilted-plane/sphere, cylinder/sphere, full-circle tilted
  leader) all refused; every certified cell enclosed its sampled points. The
  packet's explicit Phase-A-fails branch was followed: no forced center-term
  rewrite. Owner accepts the invariant argument; AUD-004 is closed as
  OWNER-DESIGN-BLOCKED.
- **BG-AUD-FIX-005** attempts 1-2 = SPEC_GAP (workers right): `inari::Interval::asin`
  is gmp-gated (off in this tree, gmp not viable on this toolchain), and inari
  has no `Interval * f64` / `Interval / f64`. Packet amended (orchestrator,
  twice) to the series hybrid `s/2 + s³/16 + 7s⁵/256` (interval arithmetic,
  `.inf()`) for `s < 1e-6` and the interval closed form for `s >= 1e-6`, with
  scalars wrapped as degenerate intervals (s⁵ coefficient machine-checked to
  7/256, not the packet's original 5/512). Attempt 3 = DONE (`8f6f71e`, fix +
  1 test). Verify queued at base `03e1d0a`.
