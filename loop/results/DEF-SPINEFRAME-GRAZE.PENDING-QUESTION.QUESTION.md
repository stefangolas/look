# DEF-SPINEFRAME-GRAZE — QUESTION: option (c) v-pad cannot flip the showcase suites; orchestrator amendment required

**Status:** diagnosed stop (packet stop condition #1). Faithful option (c) is
implemented, committed and green on truck-geometry; the two showcase suites
still die at the sanctioned match-unwrap. Full record: `RESULT.json`.

## What was implemented (faithful to the packet's scope decisions)

`PROFILE_V_DOMAIN_PAD = 1.75e-6` (= 1.75× `DirectTolerance::parameter`; H-3
line names the quantity; re-exported through `constructive/mod.rs`) pads the
profile-v domain refusal at all THREE mandated sites — `ProfileLaw::evaluate`
(`constructive/profile.rs`), `profile_derivative_v` and `profile_derivative_s`
(`decorators/spine_frame.rs`) — refusing typed `InvalidInput` beyond the pad.
Constructor `validate_surface_window`, the `evaluate_position` match-unwrap,
the frozen seam, and the shapeops/geotrait funnel are untouched. Three new
regression tests (v = −0.5×parameter → typed Ok and subs/uder/vder answer;
v = −2×parameter → typed `InvalidInput`; same for the derivative fns) pass,
as do all 81 truck-geometry lib tests; fmt/check/clippy −D warnings are clean.
A1 measured 0 (the exact check was replaced by the padded check); A2=5 and
A3=1 unchanged. No landed kernel test flips red.

## The measured stop

The funnel's unclamped Newton iterates over admitted `SpineFrameSweep` faces
are not tolerance-grazing — they range far outside the parameter domain on
BOTH axes. First-refusal witnesses as the pad grew:

| pad (v) | first refusal |
|---|---|
| exact (defect) | (s=2.3875e-4, v=−1.2138e-6) |
| 1.5e-6 | (s=1.9463e-3, v=−1.5301e-6) |
| 1.9e-6 | (s=3.1654e-3, v=−1.9048e-6) |
| 1e-5 | (s=6.7781e-3, v=−1.28796e-5) |
| 1e-4 (s still 1e-6) | (s=−2.3052e-3, v=0.50029) — SPINE axis, s ≈ 2300× parameter |

A probe pad of 0.5 on BOTH the profile-v sites and the sweep-evaluation spine
`position_at`/`derivative_at` sites flips battery_waterslide 9/9 of the
recorded tests green. So the required tolerance is ~5×10⁵× beyond
`DirectTolerance::parameter` — no pad that also refuses at the packet's own
−2×parameter probe can rescue the suites, and the s-axis (scoped to stay at
the 1e-6 pad) is grazed by the same searches.

## Pre-existing, unrelated failures confirmed identical at base b5491db

- `showcases` `concave_u_chute_refuses_on_the_facet_path` (facet_sweep returns
  Ok on the u-chute profile instead of refusing; the "stale chute" sibling
  packet's domain).
- `constructive_spine_enum` `general_spine_becomes_certifiedpatch_not_refused_for_promotion`
  and `spine_enum_dispatches_general_to_the_landed_spine_curve` (hence the
  packet's Done-when scoping that target to a filtered subset).

## Amendment options (funnel stays read-only in both)

1. **Widen the pad orders of magnitude** (order 1e-1) on both the profile-v
   sites and the sweep-evaluation spine-axis sites, relax the −2×parameter
   refusal probe to a beyond-pad probe, and reconcile the landed
   `constructive_recipe.rs` assertion that `ProfileLaw::evaluate(0.5, −0.5)`
   refuses (a pad ≥ 0.5 accepts v = −0.5 — the pad must stay < 0.5 with the
   true class maximum confirmed, or that landed assertion moves with the
   amendment).
2. **Authorize total sweep evaluation on the search path** (extrapolate on both
   axes instead of panicking at the sanctioned match-unwrap), retaining typed
   `Result` refusal on the recipe/ProfileLaw API for construction admission —
   currently blocked by the packet's Forbidden list.

Redispatch recommendation: amend the packet to one of the above and re-dispatch
(keep this faithful v-pad commit; it is a strict subset of either amendment).
