# WORK PACKET PB-011B-TTC-PARITY-CHURN — lift the canonical-cutter F1 rows (the 2-D path wave)

PB-011 r1/r2 landed the routing and the monocoque lift. This packet lifts
the remaining F1 rows whose FIRST refusing op is a canonical-tool pair —
the 2-D implicit-reduction path (Theorem 4), the highest-probability wave.
Swept×swept rows are PB-011C's and MUST NOT be lifted here.

```yaml
id:          PB-011B-TTC-PARITY-CHURN
contract:    [PB-011B-TTC-PARITY-CHURN]
class:      design
crates:      [truck123d, truck-evidence]
depends_on:  [PB-011-TTC-PARITY-CHURN]
write_allow:
  - truck123d/src/facade.rs
  - truck123d/compat/surface.rs
  - truck123d/tests/pb_parity.rs
  - docs/PY_BRIDGE_COMPAT_SURFACE.md
  - corpus/ttc/SKIPS.json
read_allow:
  - corpus/ttc/
  - loop/results/PB-011-TTC-PARITY-CHURN.json
  - docs/PY_BRIDGE_COMPAT_SURFACE.md
tests_required:
  - lifted_skip_rows_run_green (the B lifted set, iterated; empty set fails)
anchors:
  - {id: A1, expect: 35, cmd: "grep -c booleans-on-swept-carriers corpus/ttc/SKIPS.json"}
budget:      {turns: 70, ctx_tokens: 190000}
```

## Pre-digested context

- The routing is LANDED (PB-011 2a12623): `run_facade` dispatches
  Mode-armed swept-carrier pairs through the certified entry; the lift
  mechanics + facts gate are exactly r1's (door run green + facts vs the
  recorded reference; references for ALL staged rows already recorded).
- **B's rows (canonical-cutter class):** f1/floor, f1/diffuser,
  f1/suspension_front, f1/suspension_rear, f1/steering_rack,
  f1/track_rod_left, f1/track_rod_right, f1/corner_fl, f1/corner_fr,
  f1/corner_rl, f1/corner_rr, f1/drs_actuator (12 rows).
- **NOT B's (PB-011C):** f1/airbox, f1/beam_wing, f1/details,
  f1/engine_cover, f1/power_unit, f1/rear_wing, f1/drs_flap,
  f1/drivetrain — their first refusing op is swept×swept.
- **Caveat on the corner rows (cut(revolved,canonical)):** a revolved
  LINE carrier is a cone/cylinder (funnel-admitted); a revolved CIRCLE is
  a TORUS (funnel-refused, `ContactReductionDeferred`). If a corner
  row's kernel run refuses on a torus carrier, that is a CORRECT typed
  refusal: record it in the row's SKIPS note (reason upgraded from
  "booleans-on-swept-carriers" to the typed torus refusal), do NOT lift,
  move to the next row. The torus-contact program owns those rows.
- **Lift order: cheapest first** (use `scratch/reference_batch_log.json`
  wall_s as the complexity key) — bank lifts before any stop-and-file.
- Each lift is ATOMIC: one commit per row (door evidence + facts compare
  + SKIPS flip + matrix-cell flip if a new cell flips), RESULT updated
  per row — the orchestrator watches progress live, no opaque batch.

## Scope decisions

1. Identical to PB-011 r1's decisions 1-5 (routing is landed; lifts
   evidence-gated; S1 status text updates; V5 absolute; determinism
   rules).
2. A row whose kernel run refuses typed → do not lift, record the typed
   refusal in its SKIPS note (the refusal census grows — that is the
   instrument), continue to the next row. Only a facts MISMATCH
   (builds but wrong geometry) is a stop-and-file.
3. The six excluded parts (front_wing, cockpit, nose, sidepods, cooling,
   halo) are not manifest rows — out of scope permanently for this
   packet.

## Done when

```
cargo fmt --check -p truck123d
cargo clippy -p truck123d -p truck-evidence --all-targets -- -D warnings
cargo test -p truck123d
python corpus/ttc/census.py
```

## Forbidden

PB-011C's rows. vendor/truck (read-only). Weakening assertions. Lifting
without a green facts-matched run.

## Stop conditions

- A facts mismatch on any row → stop-and-file (defect record), continue
  only via orchestrator amendment.
- Every row ends refused-typed → that is a valid DONE-with-census
  (record the census; the torus/4-D programs own the outcomes).

## Finish by writing RESULT.json at the WORKTREE ROOT (then COMMIT first)

Commit subject: `feat(bridge): canonical-cutter F1 lifts — the 2-D path wave (PB-011B)`.
