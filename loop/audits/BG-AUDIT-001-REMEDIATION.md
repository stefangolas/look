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
| AUD-002 | BG-AUD-FIX-002 | WORKER_DONE | | | | |
| AUD-003 | BG-AUD-FIX-003 | WORKER_DONE | | | | |
| AUD-004 | BG-AUD-FIX-004 | RUNNING | | | | |
| AUD-007 | BG-AUD-FIX-005 | RUNNING | | | | |
| AUD-005 | BG-AUD-FIX-006 | PENDING | | | | |
| AUD-009 | BG-AUD-FIX-006 | PENDING | | | | |
| AUD-006 | BG-AUD-FIX-007 | PENDING | | | | |
| AUD-008 | BG-AUD-FIX-008 | PENDING | | | | |
| AUD-015 | BG-AUD-FIX-008 | PENDING | | | | |
| AUD-011 | BG-AUD-FIX-009 | PENDING | | | | |
| AUD-012 | BG-AUD-FIX-009 | PENDING | | | | |
| AUD-010 | BG-AUD-FIX-010 | PENDING | | | | |
| AUD-013 | BG-AUD-FIX-010 | PENDING | | | | |
| AUD-017 | BG-AUD-FIX-010 | PENDING | | | | |
| AUD-014 | BG-AUD-FIX-011 | PENDING | | | | |

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
  refuses-only. Verify queued.
- **BG-AUD-FIX-003** worker done (`8bdf204`): NaN guard (hand-computed normal
  magnitude refuses non-finite/zero), sampling at `t0`/`t_mid`/`t1`, scoped
  sampled-claim doc. Test-only delegating `Surface` enum to combine Cone+Plane
  in one shell. Verify queued.
- **BG-AUD-FIX-005** attempt 1 = SPEC_GAP (worker right): `inari::Interval::asin`
  is gmp-gated and the tree pins `default-features = false`; gmp not viable on
  this toolchain. Packet amended (orchestrator) to the series hybrid:
  `s/2 + s³/16 + 7s⁵/256` in interval arithmetic `.inf()` for `s < 1e-6`, the
  interval closed form `.inf()` for `s >= 1e-6` (the s⁵ coefficient is 7/256,
  machine-checked, not the packet's original 5/512). Re-dispatched with
  `--resume`.
