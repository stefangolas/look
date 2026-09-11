# WORK PACKET FHC-EX-B — spline lofts as boolean operands + hypercar band lofts

Owner roadmap 2026-09-11 (SHORT-TERM ROADMAP, step 2, packet 2). The largest
extraction class: multi-station spline lofts must WORK AS BOOLEAN OPERANDS,
and the hypercar band lofts must extract. Flips toward green: `f1/airbox`,
`f1/details`, `f1/beam_wing`, `f1/drs_flap`, `f1/power_unit`,
`f1/track_rod_left`, `f1/track_rod_right`, `f1/monocoque`, `f1/engine_cover`,
`f1/drivetrain`; `hypercar/aero`, `hypercar/body`, `hypercar/chassis`,
`hypercar/glazing`, `hypercar/interior`; diagnoses `hypercar/brakes`.

```yaml
id:          FHC-EX-B-SPLINE-LOFT-OPERANDS
contract:    [FHC-EX-B-SPLINE-LOFT-OPERANDS]
class:       design
crates:      [truck123d]
depends_on:  [FHC-EX-A-CLOSED-LOOP-SHELL]
write_allow:
  - truck123d/src/bd_bridge.rs
  - corpus/ttc/door.py
  - truck123d/tests/extraction_breadth_b.rs
read_allow:
  - truck123d/src/facade.rs
  - truck123d/src/binding.rs
  - docs/TTC_CENSUS_FINAL.md
  - docs/MONO_CLOSURE_BOOKING.md
tests_required: [truck123d/tests/extraction_breadth_b.rs]
anchors:
  - {id: A1, expect: 4,  cmd: "grep -c 'dispatch_swept_carrier_boolean' truck123d/src/facade.rs"}
  - {id: A2, expect: 12, cmd: "grep -c 'ProfileEdge::Spline' truck123d/src/bd_bridge.rs"}
  - {id: A3, expect: 1,  cmd: "grep -c 'fn profile_loop' truck123d/src/bd_bridge.rs"}
budget:      {turns: 80, ctx_tokens: 260000}
```

Anchors measured 2026-09-11 at HEAD `cb1fabf` — **re-measure at dispatch**;
FHC-EX-A's landing WILL drift A2/A3. Update the packet at dispatch if a
count moved, never dispatch a stale anchor.

## R3 verdicts this packet closes (verbatim)

- multi-station spline loft: `beam_wing`, `drs_flap`, `power_unit`,
  `track_rod_left/right` — `kernel refusal: unsupported_envelope`
- native boolean admission on spline-loft operands: `airbox`, `details`
- native facts at `_fuse`: `drivetrain`
- OCC-probe + cut/fuse: `monocoque`, `engine_cover` (the probe gap is
  door-side; the boolean tail is the same machinery as the rest)
- hypercar band lofts: `body`, `chassis`, `glazing`, `interior` —
  `unsupported_envelope`; `aero` — `kernel refusal: empty`
- `brakes` — `revolve needs a closed profile` (diagnose: name the exact
  missing profile verb, or admit it)

## Pre-made judgements

1. The R3 finding is definitive: the wall is ADMISSIBILITY (extraction /
   envelope), not solver numerics (RDEF-M2 confirmed zero
   NonTransversalContact/BudgetExhausted). So: widen the ADMITTED operand
   class of the landed swept-carrier boolean dispatch to include
   spline-section loft carriers, and widen the extraction that feeds it.
2. The boolean machinery (MONO-6 contact covers + MONO-5 named membership +
   MONO-8 admission wiring + MONO-9 fuse fold) is LANDED and is not edited
   here beyond the dispatch/admission wiring in the allowlisted files. If a
   genuine solver-theory gap appears (a pair the landed theory cannot
   bracket), that is MONO-10/owner territory: refuse typed naming the pair,
   do not improvise a numeric shortcut.
3. Hypercar band lofts (`surfaces._mirrored_face`, `_xz_band`, `chassis._face`)
   are spline section bands with end tangents — same carrier class as the
   landed smooth loft; end-tangent handling is the only new shape.
   `aero`'s `case empty` refusal is a diagnosis item: either the band
   composition produces a degenerate section (fix at the carrier, honestly)
   or it names its open carrier typed.
4. OCC references are diagnostics (annex C). A green row's bracket is its
   own certificate. Honesty line unchanged: no tolerance stretch, no
   approximant replication.
5. door.py is in the write set only to record new carrier rows and keep
   refusals typed. No corpus tree files.

## Method

One row end-to-end (record -> bd_facts bracket + bd_stl emits) before the
next; start with `beam_wing` (pure loft, no boolean) to pin the operand
carrier, then `airbox` (loft-as-operand boolean), then the rest, then the
hypercar band lofts, then `brakes`' diagnosis. Tests: one green round-trip
per claimed row; one typed refusal naming the open carrier for any row left
red; a spline-loft cut/fuse composition test exercising the widened
admission.

## Done when

```
cargo fmt --check -p truck123d
cargo clippy -p truck123d --all-targets -- -D warnings
cargo test -p truck123d --test extraction_breadth_b --locked
```

plus door spot-checks per claimed row (`ok: true` with `volume_lo ==
volume_hi`, or a typed refusal recorded in RESULT notes). All cargo through
the queue (the `cargo` on PATH IS the queue shim); interpreter dir on PATH
if STATUS_DLL_NOT_FOUND.

## Forbidden

Any file outside `write_allow`. Editing `corpus/ttc/trees/**`. Editing the
certified funnel (`vendor/truck/**` — instant violation). Changing the
pinned loft convention. `#[ignore]`, deleted/weakened tests, bare
`cargo test`. Committing to `main`.

## Stop conditions

- a swept x swept (or loft x swept) pair the landed theory cannot certify → typed refusal + `SPEC_GAP` naming the pair class (the loop books the widening — that is success, not failure)
- anchor count differs at dispatch and was not re-measured → `ANCHOR_MISMATCH`
- three consecutive failed cargo runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` AT THE WORKTREE ROOT

```json
{"id":"FHC-EX-B-SPLINE-LOFT-OPERANDS","status":"DONE","contracts":["FHC-EX-B-SPLINE-LOFT-OPERANDS"],
 "anchors_verified":{"A1":4,"A2":12,"A3":1},
 "rows_flipped":[],"rows_still_refused":[],
 "notes":"per-row verdict + bracket; any SPEC_GAP pair classes named"}
```

Commit on the current branch, subject
`truck123d: spline lofts as boolean operands + hypercar band lofts (FHC-EX-B)`.
