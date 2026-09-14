# WORK PACKET FHC-G12 — admit the floor/diffuser face-stack loft carrier (G5's named residual)

G5 surfaced the swallowed refusals: `f1/floor` and `f1/diffuser` fail in
`floor._loft_stack` (`floor.py:511`) — a **station stack of section faces**
lofted via `surfaces.body_loft`, with coarsening retry trials — refusing
typed `unsupported_envelope` / `non_canonical_carrier` (the open smooth
spline-loft carrier). The landed spline-loft arm
(`certified_spline_loft_volume` → `spline_loft_volume_rows`) admits recorded
closed **loops**; the floor stack lofts **faces** whose boundary structure
differs from the recorded-loop form. This packet names the exact structural
difference, extends admission/extraction to the face-stack form within landed
theory, and takes both rows green.

**Normative theory:** Certified Flux Calculus (§1.4 area vector, §3 exact
polynomial flux, §8 planar cap terms) via
`docs/CERTIFIED_FLUX_CALCULUS_INTEGRATION_MAP.md`. If the face-stack carrier
reduces to recorded loops plus exact planar cap terms, no new integration
machinery is required — the fix is admission/extraction marshalling.

```yaml
id:          FHC-G12-FLOOR-LOFT-CARRIER
contract:    [FHC-G12-FLOOR-LOFT-CARRIER]
class:       mechanical+
crates:      [truck123d]
depends_on:  []
needs:       []
write_allow:
  - truck123d/src/bd_bridge.rs
  - truck123d/tests/floor_loft_carrier.rs
read_allow:
  - corpus/ttc/trees/f1/src/lib/floor.py
  - corpus/ttc/trees/f1/src/lib/surfaces.py
  - docs/F1_HYPERCAR_GAP_REGISTER.md
  - docs/CERTIFIED_FLUX_CALCULUS_INTEGRATION_MAP.md
tests_required: [truck123d/tests/floor_loft_carrier.rs]
anchors:
  - {id: A1, expect: 6, cmd: "grep -c 'cell_flux_exact' truck123d/src/bd_bridge.rs"}
  - {id: A2, expect: 4, cmd: "grep -c 'boolean_product_volume_certified' truck123d/src/bd_bridge.rs"}
  - {id: A3, expect: 11, cmd: "grep -c 'spline_loop_area_vector' truck123d/src/bd_bridge.rs"}
budget:      {turns: 55, ctx_tokens: 200000}
```

Anchors measured 2026-09-13 at HEAD. **Re-measure at dispatch.**

## Pre-made judgements

1. **Diagnose before extending.** The worker's first deliverable is the
   structural statement: what `surfaces.body_loft` produces for
   `_loft_stack`'s face stacks (section count, per-face boundary form, span
   structure, open vs closed) versus what `spline_loft_volume_rows` admits.
   The refusal's carrier metadata (`client_site`, `verb`, `carrier`) names
   the mismatch — read it from a bounded door run, quote it in RESULT.
2. **The coarsening trials are client-side OCC defense** (`trials` loop) —
   the kernel arm must admit the FULL stack (and each trial subset); never
   rely on the client's fallback to mask an admission gap.
3. **If the faces carry interior structure** (plate holes — FHC-B's
   multi-contour representation), route holes through G9's Green machinery
   if landed, else refuse typed `unimplemented_trim` naming G9 — do NOT
   approximate.
4. **No approximation**: the carrier is admitted only if the certified
   bracket is exact-or-enclosed per the landed algebra; byte-identity of
   already-green rows is the regression gate.
5. Out of scope: `hypercar/aero` (`extrude`/`trim_prism` — possibly closed by
   the landed TRIM packet; R4 decides) and `hypercar/powertrain`
   (`spline_loft×spline_loft` — possibly closed by MONO-8 + G1; R4 decides).

## Tests required (`truck123d/tests/floor_loft_carrier.rs`)

1. `carrier_structural_statement` — the diagnosed difference is encoded as a
   test: a minimal face-stack loft of the floor shape admits with a green
   bracket.
2. `stack_coarsening_trials_all_admit` — each trial stack length the builder
   may produce admits through the certified arm (no trial depends on the
   fallback).
3. `floor_row_end_to_end` — `f1/floor` completes: green bracket + solid_count
   + STL emitted; the typed swallow no longer occurs.
4. `diffuser_row_end_to_end` — same for `f1/diffuser`.
5. `regression_byte_identity` — every previously green row's STL fingerprint
   is unchanged (the landed corpus fingerprint rule).

## Done when

```
cargo fmt --check -p truck123d
cargo clippy -p truck123d --all-targets -- -D warnings
cargo test -p truck123d --test floor_loft_carrier --locked
```

plus bounded door spot-checks on both rows. All cargo through the queue.

## Forbidden

Any file outside `write_allow`; editing `corpus/ttc/**` (READ-ONLY here) or
`vendor/truck/**`; approximation; tolerance changes; `#[ignore]`, deleted or
weakened tests, bare `cargo test`; committing to `main`.

## Stop conditions

- the face-stack carrier reduces to a form outside landed theory (not loops +
  planar caps + exact polynomial flux) → `SPEC_GAP` naming the missing
  primitive (that is register evidence, not failure)
- anchor count differs at dispatch and was not re-measured → `ANCHOR_MISMATCH`
- three consecutive failed cargo runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` AT THE WORKTREE ROOT

```json
{"id":"FHC-G12-FLOOR-LOFT-CARRIER","status":"DONE","contracts":["FHC-G12-FLOOR-LOFT-CARRIER"],
 "anchors_verified":{"A1":6,"A2":4,"A3":11},
 "rows_flipped":["f1/floor","f1/diffuser"],
 "notes":"carrier structural statement; suite 1-5; row verdicts"}
```

Commit subject: `truck123d: admit the face-stack loft carrier - floor/diffuser rows (FHC-G12)`.
