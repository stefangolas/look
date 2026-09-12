# WORK PACKET FHC-G6 — certified-construction cost at scale: profile, then the mechanical fix

Gap-register category 6: `f1/power_unit` and `f1/suspension_rear` exceeded
the census's 120 s cap — they are COST cases, not refusals (the machinery
admits them; the certified multi-station spline loft + bracket evaluation is
expensive at big section counts; beam_wing, a smaller loft of the same
class, builds in 5.4 s). The deliverable is a per-phase cost decomposition,
then the mechanical optimization the profile dictates — within landed
theory. If the profile shows the bracket structure itself needs a smarter
decomposition, that is a SPEC_GAP back to the register (theory-adjacent
reclassification), not a shortcut here.

```yaml
id:          FHC-G6-CERT-COST-SCALE
contract:    [FHC-G6-CERT-COST-SCALE]
class:       mechanical+
crates:      [truck123d]
depends_on:  [FHC-G5-SWALLOWED-REFUSAL-DIAGNOSIS]
needs:       [FHC-G5-SWALLOWED-REFUSAL-DIAGNOSIS]
write_allow:
  - truck123d/src/bd_bridge.rs
  - truck123d/tests/cert_cost_scale.rs
read_allow:
  - corpus/ttc/door.py
  - docs/F1_HYPERCAR_GAP_REGISTER.md
  - docs/TT_TIMING_RESULTS.md
tests_required: [truck123d/tests/cert_cost_scale.rs]
anchors:
  - {id: A1, expect: 4, cmd: "grep -c 'certified_spline_loft_volume' truck123d/src/bd_bridge.rs"}
  - {id: A2, expect: 16, cmd: "grep -cE 'volume_bracket[^A-Za-z0-9_]|volume_bracket$' truck123d/src/bd_bridge.rs"}
budget:      {turns: 70, ctx_tokens: 220000}
```

Anchors measured 2026-09-12 at the EX-B-landed HEAD. **Re-measure at
dispatch** — the docket ahead WILL drift them; update at dispatch, never
dispatch stale.

## Pre-made judgements

1. **Measure before touching anything.** Deliverable one is the
   decomposition: per-phase (section extraction / loft surface evaluation /
   per-patch interval certification / bbox / mesh+STL write) timings for
   `power_unit` and `f1/suspension_rear`, recorded in RESULT with the raw
   samples. Instrument bd_bridge's loft/facts path (phase timers into the
   `bd_facts`/`stl` record — additive fields, existing fields unchanged).
2. **Optimization whitelist (mechanical, within landed theory):** angle
   tables instead of per-vertex `sin_cos` recomputation; evaluation-order
   and allocation work in the enclosure walk (fixed reduction order — the
   determinism law is absolute); reuse of per-span data between the surface
   and bracket passes. All must keep outputs BYTE-IDENTICAL: the
   before/after fingerprint equality test is the gate.
3. **Out of scope (would be SPEC_GAP):** parallelism (determinism law),
   approximating the enclosure, widening tolerances, changing the pinned
   loft convention, restructuring the bracket decomposition.
4. The measured cost numbers are outputs, never thresholds to tune against;
   the done-when is the byte-identity + the decomposition being recorded,
   plus whatever whitelist fix the profile dictated landing green.

## Method

Instrument → measure both rows (fresh process, repeated 3x, note machine
load) → identify the dominant phase → apply the whitelisted fix if the
dominant phase is on the list → re-measure → verify byte-identical
artifacts and green brackets. If the dominant phase is the per-patch
certification and no whitelist fix applies, STOP: record the decomposition
and SPEC_GAP with the numbers (that is the register's theory-adjacent
evidence).

## Tests required

`truck123d/tests/cert_cost_scale.rs`: phase-timer fields present and
additive; beam_wing-class loft before/after byte-identity (STL fingerprint
unchanged); bracket equality before/after; both heavy rows complete within
a GENEROUS bound recorded as data (not asserted as a threshold — assert
only completion + identity).

## Done when

```
cargo fmt --check -p truck123d
cargo clippy -p truck123d --all-targets -- -D warnings
cargo test -p truck123d --test cert_cost_scale --locked
```

plus door spot-checks: `power_unit` and `f1/suspension_rear` complete
(green bracket + STL) with the decomposition recorded. All cargo through
the queue (the `cargo` on PATH IS the queue shim); interpreter dir on PATH
if STATUS_DLL_NOT_FOUND.

## Forbidden

Any file outside `write_allow`; editing `corpus/ttc/**` or
`vendor/truck/**`; parallelism; approximation of enclosures; tolerance
changes; `#[ignore]`, deleted or weakened tests, bare `cargo test`;
committing to `main`.

## Stop conditions

- dominant phase is the per-patch certification and no whitelist fix applies → record + `SPEC_GAP` with the decomposition
- byte-identity cannot be preserved with a whitelist fix → revert the fix, record, proceed with decomposition only
- anchor count differs at dispatch and was not re-measured → `ANCHOR_MISMATCH`
- three consecutive failed cargo runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` AT THE WORKTREE ROOT

```json
{"id":"FHC-G6-CERT-COST-SCALE","status":"DONE","contracts":["FHC-G6-CERT-COST-SCALE"],
 "anchors_verified":{"A1":4,"A2":16},
 "notes":"per-phase decomposition both rows; fix applied (or SPEC_GAP); before/after ms; byte-identity verified"}
```

Commit subject: `truck123d: certified-loft cost decomposition + whitelisted fixes (FHC-G6)`.
