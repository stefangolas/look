# WORK PACKET FHC-G11 — adaptive certification controller: order-vs-subdivide under a certified budget

Closes the cost wall (register category 6, G6's SPEC_GAP evidence): per-patch
certification is 99.8% of facts time on heavy rows; heavy fuses do not
terminate because every patch is certified at full degree uniformly. The
proposal's §12 supplies the controller design; **the math is already shipped**
(Thm 5.2's tail `R_{k,r}(δ)`, Cor 5.4's `O(h^{r+1})` spatial order, Hong &
Stahl's inclusion monotonicity [C]). What this packet implements is the
scheduler — and per §12, "the prediction can be crude and empirical; it
cannot affect soundness because every accepted child/order update carries its
own certificate."

**Normative theory:** Certified Flux Calculus §12 + §16 via
`docs/CERTIFIED_FLUX_CALCULUS_INTEGRATION_MAP.md`.

```yaml
id:          FHC-G11-ADAPTIVE-CERTIFICATION
contract:    [FHC-G11-ADAPTIVE-CERTIFICATION]
class:       mechanical+
crates:      [truck123d]
depends_on:  [FHC-G1-RATIONAL-FLUX]
needs:       [FHC-G1-RATIONAL-FLUX]
write_allow:
  - truck123d/src/bd_bridge.rs
  - truck123d/tests/adaptive_certification.rs
read_allow:
  - docs/F1_HYPERCAR_GAP_REGISTER.md
  - docs/CERTIFIED_FLUX_CALCULUS_INTEGRATION_MAP.md
tests_required: [truck123d/tests/adaptive_certification.rs]
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'fn cell_flux_certified' truck123d/src/bd_bridge.rs"}
  - {id: A2, expect: 6, cmd: "grep -c 'cell_flux_exact' truck123d/src/bd_bridge.rs"}
budget:      {turns: 60, ctx_tokens: 220000}
```

Anchors measured post-G1 expectations; **re-measure at dispatch.**

## Pre-made judgements

1. **One controller, two actions** (§12): each active cell carries its
   current certified interval, width channels, current order `r`; the two
   legal improvements are (a) raise the reciprocal order, (b) subdivide the
   cell (Bernstein subdivision; nesting is inclusion-monotone [C]). Initial
   policy: **order-first up to `r_max`, then binary subdivision**, deepest-cell
   first — crude, sound, tuned later by E1–E5 data. The policy is data, not
   semantics; record it in RESULT.
2. **Two stop modes** (§12.3/12.4): decision mode stops on separation from a
   caller threshold (wired to MONO-5's existing separation semantics); value
   mode stops at a caller-supplied width `ε` — an API request, never a hidden
   kernel tolerance. Budget exhaustion returns `rational_flux_inconclusive`
   (G1's vocabulary) — never a guess, never a silent widening.
3. **The door's default**: value mode with a GENEROUS recorded `ε` on
   volume facts (the bracket is the output contract; `volume` reports the
   bracket as `(value, lo, hi)` with `value` the midpoint only when the
   caller mode requests it — existing exact paths keep `(q,q;0,0)` migration,
   byte-identical).
4. **No parallelism** (determinism law absolute). Fixed deterministic
   best-first order.
5. Out of scope: trim clipping (G9 owns trims), policy tuning (E-data lands
   after this packet runs the corpus).

## Tests required (`truck123d/tests/adaptive_certification.rs`)

1. `exact_path_unchanged` — every previously exact result is bit-identical
   under the controller (migration `(q,q;0,0)` semantics).
2. `heavy_fuse_terminates` — a previously non-terminating suspension fuse
   certifies to a recorded width within a recorded budget; the bracket
   contains a high-precision reference.
3. `width_monotone_under_refinement` — controller iterations never widen the
   active interval (nesting).
4. `separation_early_stop` — decision mode stops finite when separated from
   the threshold; equality/near-tangency refuses `rational_flux_inconclusive`,
   never a fabricated sign (doc T12).
5. `budget_refuses_typed` — exhausted budget produces the typed refusal with
   `w_m`/`w_i` attribution in the refusal metadata.
6. `row_spot_checks` — `f1/power_unit` and `f1/suspension_rear` complete
   end-to-end within a GENEROUS recorded bound (completion + identity
   asserted, never a time threshold).

## Done when

```
cargo fmt --check -p truck123d
cargo clippy -p truck123d --all-targets -- -D warnings
cargo test -p truck123d --test adaptive_certification --locked
```

plus bounded door spot-checks on both heavy rows. All cargo through the queue.

## Forbidden

Any file outside `write_allow`; editing `corpus/ttc/**` or `vendor/truck/**`;
parallelism; hidden global tolerances; `#[ignore]`, deleted or weakened
tests, bare `cargo test`; committing to `main`.

## Stop conditions

- a heavy row cannot terminate within a 10x generous budget even adaptively
  → record the decomposition + `SPEC_GAP` naming where the width concentrates
- anchor count differs at dispatch and was not re-measured → `ANCHOR_MISMATCH`
- three consecutive failed cargo runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` AT THE WORKTREE ROOT

```json
{"id":"FHC-G11-ADAPTIVE-CERTIFICATION","status":"DONE","contracts":["FHC-G11-ADAPTIVE-CERTIFICATION"],
 "anchors_verified":{"A1":1,"A2":6},
 "rows_flipped":["f1/power_unit"],
 "notes":"controller policy recorded; suite 1-6; row before/after wall times; w_m/w_i attribution"}
```

Commit subject: `truck123d: adaptive certification controller - order-vs-subdivide under certified budget (FHC-G11)`.
