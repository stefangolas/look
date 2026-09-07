# SEM-SPINEFRAME-WINDOW-GRAZE-001 — spine-frame evaluation refuses (then panics) at window-grazing search iterates

**Family** `SEM` — **Manifestation** `REACHABILITY-HOLE` (H-1 panic reachable
from untrusted geometry through the landed boolean funnel)
**Contracts** `ProfileLaw::evaluate` domain check (constructive/profile.rs:21);
the `evaluate_position` sanctioned match-unwrap (decorators/spine_frame.rs:430-445);
the `validate_surface_window` contract (spine_frame.rs:284-317); BG-TOL-001.

## 1. Status

```
Open (DEF-SPINEFRAME-GRAZE booked 2026-09-07)
```

## 2. The defect

`ProfileLaw::evaluate` refuses `ConstructError::InvalidInput` on the EXACT
check `!(0.0..=1.0).contains(&v)` (profile.rs:21). Every s-axis check in the
same evaluator stack is padded with `DirectTolerance::parameter`
(spine_frame.rs:53/64/268, recipe.rs:246/257) — the v axis is the anomaly.

The panicking evaluation (witness, deterministic across all occurrences):

```
spine-frame evaluation refused at (0.00023875499775604468, -0.000001213823157535602): invalid input
```

fires at `evaluate_position`'s sanctioned match-unwrap
(spine_frame.rs:443), whose doc premise — "the constructor validated the
window, so a refusal inside it is unreachable" — holds only inside the
window. The caller is the shapeops boolean funnel (BIE-006 admitted
SpineFrameSweep faces at `417cdce`; CTE-007 reshaped the entry gates):
unclamped Newton iterates in truck-geotrait
`algo::surface::search_parameter` (surface.rs:278-299) evaluate
`subs/uder/vder` at raw iterates. Call sites: sweep_lift.rs:122,
classify.rs:412/443/505/528, split.rs:1221/2014/2036/2108/2119,
divide_face:36/45, loops_store:509-511, healing:55-109.

The witness overshoots the v0 = 0.0 boundary by 1.21x
`DirectTolerance::parameter` — it is an unclamped numerical-search iterate,
not f64 roundoff of a stored boundary point.

## 3. Regression window

The panic path predates `fde664d` (landed in `c8bd3ef`, BG-CG-009-BREP,
Aug 31). `fde664d` (BG-KV2-501-C6) is semantics-preserving for evaluation
(character-for-character validation extraction; visibility/share for the
new SpineFrameSweep carrier). REACHABILITY came later: BIE-006 (`417cdce`,
Sep 5) let sweep faces through the boolean funnel — before that the
chute∪pool union refused typed at the lift gate and no search ever touched
a sweep face. The domain-check asymmetry (exact v vs padded s) predates
both.

## 4. Blast radius (measured, CFP one-verify battery 2026-09-07)

Ten tests die at this panic: showcases/tests/battery_waterslide.rs (9 —
three_frame_laws_realize_with_matching_volumes,
frame_laws_are_right_handed_and_agree,
frame_laws_enclose_congruent_or_sane_volumes,
facet_volume_near_analytic_estimate, determinism_same_table_same_report,
exports_record_outcomes, cc_ports_defer_with_typed_refusals,
brep_volume_matches_facet_on_small_case, facet_mesh_stays_within_path_bounds)
and showcases/tests/pb_contract.rs (1 —
pb_report_determinism_same_table_same_rev).

Fixing position alone moves the panic to the sibling derivative sites
(spine_frame.rs:473/495 via profile_derivative_v/s at :156/:176 — the same
Newton call evaluates both derivatives): all THREE v-check sites must move
together.

## 5. Sanctioned fix direction

Pad the v-domain refusal at all three sites with ONE named H-3 constant
sized against the witness class (a pad of exactly 1e-6 does NOT rescue the
1.21x witness), typed `InvalidInput` beyond it. Doctrine: `ring_point` is
linear in v (linear extrapolation at the graze — no discontinuity, no
fabricated geometry); the search's own residual gate (`ctx.near_points`)
still decides acceptance; search values are never certificate-quoted (the
same doctrine that sanctions `central_difference_*` "for the SEARCH path
only", spine_frame.rs:499-503). Do NOT pad the constructor's window
validation; do NOT clamp inside evaluate_position; do NOT touch the boolean
funnel or the frozen seam.

Owner packet: DEF-SPINEFRAME-GRAZE.
