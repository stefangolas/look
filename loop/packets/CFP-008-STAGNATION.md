# WORK PACKET CFP-008-STAGNATION — cone-overlap non-shrink routes to the landed CTE cascade

You are converting the funnel's budget-burn `Unresolved` tail into certified
verdicts: when cone overlap does not shrink between subdivision levels,
route the pair to the landed CTE cascade instead of burning more budget.
Everything you need is in this document,
`docs/CONTACT_FAST_PATH_BUILD_SPEC.md` (§3 Gauss-map section, §3a findings,
§5 gates — quoted below), and the anchors. Do not read other spec files. A
genuine gap is a SPEC_GAP: stop and report.

```yaml
id:          CFP-008-STAGNATION
contract:    [CFP-008-STAGNATION]
class:       mechanical
crates:      [truck-certified]
depends_on:  [CFP-003-SEPARABILITY, CFP-006-CONE-CERTIFICATES]
write_allow:
  - vendor/truck/truck-certified/src/ssi.rs
  - vendor/truck/truck-certified/src/tangency/cascade.rs
read_allow:
  - vendor/truck/truck-certified/src/cfp/spine.rs
  - vendor/truck/truck-certified/src/cfp/fixtures.rs
  - vendor/truck/truck-certified/src/ssi_types.rs
  - vendor/truck/truck-certified/src/tangency/mod.rs
  - vendor/truck/truck-evidence/src/contact/gff.rs
  - docs/CONTACT_FAST_PATH_BUILD_SPEC.md
tests_required:
  - stagnation_routes_to_cascade_fc6
  - cascade_verdict_replaces_budget_burn
  - stagnation_detection_deterministic
anchors:
  - {id: A1, expect: 1, cmd: "grep -c unresolved vendor/truck/truck-certified/src/ssi.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub fn' vendor/truck/truck-certified/src/tangency/cascade.rs"}
  - {id: A3, expect: 5, cmd: "grep -cF 'F-C6' vendor/truck/truck-certified/src/cfp/fixtures.rs"}
  - {id: A4, expect: 0, cmd: "grep -c stag-level vendor/truck/truck-certified/src/ssi.rs"}
budget:      {turns: 50, ctx_tokens: 120000}
```

H-1: no unwrap/expect without a justified same-line opt-out. H-3
same-line `// H-3`. H-6: never record Float as Exact.

## Problem

Near-tangency pairs are the funnel's degenerate tail: subdivision repeats,
cones keep overlapping, the loop exhausts its budget and reports
`Unresolved`. The CTE program landed the cascade (reduced Hessian + A1/A2
arms + five-way verdict) that classifies exactly these pairs — the funnel
just never routes to it. This packet adds the route.

## The booked mechanism (normative; spec quoted)

**CFP-008-STAGNATION (wave D, mechanical):** cone-overlap non-shrink
between subdivision levels → route to the landed CTE cascade. Converts
budget-burn `Unresolved` into A1/A2 verdicts. **Conditional in effect on
CFP-001**: today's spline stagnation is the BG-ENC-002 artifact, not
geometry — with CFP-001's certified sub-box enclosures landed, remaining
non-shrink is real tangency, which is what the cascade certifies.

**F-C6 (spine fixture, normative):** a designed near-tangency pair whose
per-box cones stay overlapping at every depth, routing to the cascade with
a certified verdict (not budget burn).

## Scope decisions — pre-made, do not relitigate

1. **Detection is level-overlap comparison, not a heuristic.** Stagnation
   = the cone-overlap measure did not shrink between subdivision level k
   and k+1 (record both; compare exactly). One threshold constant,
   pre-decided: overlap must shrink by ANY strictly-positive certified
   margin; non-shrink at two consecutive levels routes. No tuning knobs.
2. **Routing target is the LANDED cascade entry** (tangency/cascade.rs,
   51 A1/A2 vocabulary sites) — consumed through its published API. The
   cascade's internals are read-only unless the consumer needs a new
   public entry; if it does, that is ONE additive pub fn forwarding to
   the landed internal (declare it in RESULT notes).
3. **The `Unresolved` budget-burn site in ssi.rs (1 site, A1) is replaced
   by the route** — the old arm's behavior on pairs the cascade also
   refuses stays a typed `Unresolved` (fail-closed, stratum-pair
   identity carried). Never a panic, never a guess, never a silent
   downgrade.
4. **Determinism:** the detection sees subdivision levels in fixed order;
   the route is a pure function of the two recorded levels. Same input
   sequence → same verdict.
5. **SFC discipline:** the overlap measure is certified (interval cone
   arithmetic from CFP-001's sub-box cones); the level comparison is
   exact. No naked float comparisons (H-3 on any epsilon).

## Tests required

1. `stagnation_routes_to_cascade_fc6` — the F-C6 near-tangency fixture:
   cones overlap at every depth; the pair routes to the cascade and
   returns the cascade's verdict, not budget exhaustion.
2. `cascade_verdict_replaces_budget_burn` — the A1 site's old behavior
   (budget-burn `Unresolved`) asserted GONE for the F-C6 class, and the
   fail-closed typed `Unresolved` asserted PRESENT for a pair the cascade
   itself refuses.
3. `stagnation_detection_deterministic` — same subdivision sequence
   twice → identical detection decision and verdict (byte-identical
   RESULT payload).

## Done when — run these, all must pass

```
cargo fmt --check -p truck-certified
cargo clippy -p truck-certified --all-targets -- -D warnings
cargo test -p truck-certified --lib
cargo check -p truck-shapeops
```

Scoped commands only, through the queue shim (the `cargo` on PATH IS the
shim). Send output to a file and read the tail.

## Forbidden

Anything outside write_allow — especially `ssi4.rs` (frozen),
`ssi_types.rs`/`ssi_trace.rs`/`ssi_admit.rs` (CFP-003/007's),
`contact/**` (truck-evidence; CFP-004/006's), `tangency/{minors,exclude,
tsystem}.rs` (CFP-003's), the CTE-004 landed tests (byte-identical
constraint — V5 guard), corpus fixtures (read-only). New top-level
evidence kinds. Loosening any landed tolerance. Adding `#[ignore]`.
Unjustified `#[allow]`. Committing to main.

## Stop conditions

- any anchor count differs → ANCHOR_MISMATCH
- the cascade entry cannot classify the F-C6 class without widening its
  public API beyond one additive forward → SPEC_GAP
- three consecutive failed cargo test runs on the same error → BLOCKED

## Finish by writing RESULT.json at the WORKTREE ROOT (then COMMIT first)

```json
{"id":"CFP-008-STAGNATION","status":"DONE","contracts":["CFP-008-STAGNATION"],
 "tests_added":3,"anchors_verified":{"A1":1,"A2":1,"A3":8,"A4":0},
 "notes":"the level-overlap detection record; the cascade consumer call shape; any deviation"}
```

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-DONE status also write `QUESTION.md` beside it.

Commit subject: `feat(certified): stagnation route to the CTE cascade — budget burn becomes certified verdicts (CFP-008-STAGNATION)`.
