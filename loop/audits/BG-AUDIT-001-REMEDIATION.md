# BG-AUDIT-001 — remediation tracking

Campaign: parallel remediation of every finding in `BG-AUDIT-001.md` / `.json`.
Base: audit snapshot `f919228`; every fix is written against current
`integration/kernel-bg` (HEAD `d34206e` at campaign start).

State vocabulary: `PENDING` `RUNNING` `WORKER_DONE` `VERIFIED` `LANDED`
`ALREADY_FIXED` `OWNER_BLOCKED`.

| finding | fix packet | state | worker commit | verifier | landed commit | regression |
|---|---|---|---|---|---|---|
| AUD-001 | BG-AUD-FIX-001 | RUNNING | | | | |
| AUD-016 | BG-AUD-FIX-001 | RUNNING | | | | |
| AUD-002 | BG-AUD-FIX-002 | RUNNING | | | | |
| AUD-003 | BG-AUD-FIX-003 | RUNNING | | | | |
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
