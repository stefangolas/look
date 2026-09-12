# WORK PACKET FHC-C-AUTHORING-FIDELITY — make_loft, FilletPolyline, spline interpolation options

Gap-register discovery 2026-09-13 (interim census): three authoring-surface
gaps, each small, each blocking a named row. **No new theory** — every piece
dispatches to landed carriers; the spline options select between the landed
exact Lagrange and Hermite interpolants.

```yaml
id:          FHC-C-AUTHORING-FIDELITY
contract:    [FHC-C-AUTHORING-FIDELITY]
class:       mechanical
crates:      [truck123d]
depends_on:  [FHC-B-MULTI-CONTOUR-SECTIONS]
needs:       [FHC-B-MULTI-CONTOUR-SECTIONS]
write_allow:
  - corpus/ttc/door.py
  - truck123d/src/bd_bridge.rs
  - truck123d/tests/authoring_fidelity.rs
read_allow:
  - docs/F1_HYPERCAR_GAP_REGISTER.md
  - docs/MONO_CLOSURE_BOOKING.md
tests_required: [truck123d/tests/authoring_fidelity.rs]
anchors:
  - {id: A1, expect: 0, cmd: "grep -c 'make_loft' corpus/ttc/door.py"}
  - {id: A2, expect: 4, cmd: "grep -c 'FilletPolyline' corpus/ttc/door.py"}
  - {id: A3, expect: 11, cmd: "grep -c 'interpolation' corpus/ttc/door.py"}
budget:      {turns: 50, ctx_tokens: 160000}
```

Anchors measured 2026-09-13 at the G4-landed HEAD. **Re-measure at
dispatch** — update at dispatch, never dispatch stale.

## The three pieces (each blocks a named row)

1. **`make_loft`** (`hypercar/suspension_rear`, currently untyped
   AttributeError): route to the landed N-station loft — a name-surface
   addition, same class CENSUS-NAMES closed. ~10–30 LOC.
2. **`FilletPolyline`** (`hypercar/wheels`, progressed past `bd.Color`):
   a polyline with arc-filleted corners decomposes EXACTLY into
   line-arc-line chains — every piece is a landed carrier (Line, Arc,
   `profile_loop`). Read the corpus's radius semantics from the call site;
   record the decomposed chain. ~50–150 LOC.
3. **Path spline with interpolation options** (`hypercar/lighting`): the
   corpus passes interpolation options to `bd.Spline` that change the
   curve (tangents/parameterization class). The bridge carries EXACT
   Lagrange AND Hermite interpolation — record the option and dispatch to
   the matching landed interpolant. **Honesty line:** if an option's
   semantics differ from what any landed interpolant produces exactly,
   refuse TYPED naming the option — the canonical-convention precedent
   (MONO annex A correction) applies; do not approximate.

## Pre-made judgements

1. One piece end-to-end (door round-trip: green bracket + STL for its row,
   or typed refusal naming the open carrier) before the next; easiest
   first: `make_loft`, then `FilletPolyline`, then the spline options.
2. All refusals carry the FHC-G7 metadata (refusal_code, verb, carrier,
   phase, client_site) — the vocabulary table in door.py is the
   registration point.
3. No corpus tree edits; no approximations; the OCC references stay
   diagnostics (annex C).

## Method

Door spot-check each target row to capture the exact refusing verb in
context; admit per the judgements; tests: one green round-trip per claimed
row (facts bracket + STL); a FilletPolyline decomposition test asserting
the line-arc-line chain against hand-computed tangent points; a spline
option test per landed interpolant (Lagrange vs Hermite select correctly);
typed refusals for options outside the landed set.

## Done when

```
cargo fmt --check -p truck123d
cargo clippy -p truck123d --all-targets -- -D warnings
cargo test -p truck123d --test authoring_fidelity --locked
```

plus door spot-checks per claimed row. All cargo through the queue (the
`cargo` on PATH IS the queue shim); interpreter dir on PATH if
STATUS_DLL_NOT_FOUND.

## Forbidden

Any file outside `write_allow`; editing `corpus/ttc/trees/**` or
`vendor/truck/**`; spline approximation (options outside the landed
interpolants refuse typed); `#[ignore]`, deleted or weakened tests, bare
`cargo test`; committing to `main`.

## Stop conditions

- a spline option with no exact landed interpolant → typed refusal + `SPEC_GAP` naming the option (books the interpolation-work packet, if one is ever needed)
- anchor count differs at dispatch and was not re-measured → `ANCHOR_MISMATCH`
- three consecutive failed cargo runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` AT THE WORKTREE ROOT

```json
{"id":"FHC-C-AUTHORING-FIDELITY","status":"DONE","contracts":["FHC-C-AUTHORING-FIDELITY"],
 "anchors_verified":{"A1":0,"A2":4,"A3":11},
 "rows_flipped":[],"notes":"per-row verdict + bracket; which spline options mapped to which interpolant"}
```

Commit subject: `truck123d: make_loft, FilletPolyline, spline interpolation options (FHC-C)`.
