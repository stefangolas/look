# WORK PACKET FHC-TRIM — trim-extrude envelope extension

Owner roadmap 2026-09-11 (SHORT-TERM ROADMAP, step 3). MONO-4 landed the
trim idioms' wiring; the four trim-extrude rows still refuse at the
constructor envelope. This packet widens the envelope to the corpus's
actual trim shapes. Flips toward green: `f1/rear_wing`, `f1/steering_rack`,
`f1/suspension_front`, `f1/suspension_rear`.

```yaml
id:          FHC-TRIM-EXTRUDE-ENVELOPE
contract:    [FHC-TRIM-EXTRUDE-ENVELOPE]
class:       mechanical+
crates:      [truck123d]
depends_on:  [FHC-EX-B-SPLINE-LOFT-OPERANDS]
write_allow:
  - truck123d/src/bd_bridge.rs
  - corpus/ttc/door.py
  - truck123d/tests/trim_extrude_envelope.rs
read_allow:
  - truck123d/src/facade.rs
  - docs/TTC_CENSUS_FINAL.md
tests_required: [truck123d/tests/trim_extrude_envelope.rs]
anchors:
  - {id: A1, expect: 9, cmd: "grep -c 'TrimPrism' truck123d/src/bd_bridge.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'fn profile_loop' truck123d/src/bd_bridge.rs"}
budget:      {turns: 55, ctx_tokens: 160000}
```

Anchors measured 2026-09-11 at HEAD `cb1fabf` — **re-measure at dispatch**
(EX-A/EX-B landings WILL drift A2). Update at dispatch, never dispatch stale.

## R3 verdicts this packet closes (verbatim)

- `f1/rear_wing`, `f1/steering_rack`, `f1/suspension_front`,
  `f1/suspension_rear` — `kernel refusal: unsupported_envelope`
  (trim-extrude constructor)

## Pre-made judgements

1. MONO-4's note says the corpus reaches the trim constructor through
   COMPOSITION (zero `trim(` calls in the F1 lib). The refusal site is the
   constructor's envelope check, not a missing verb. Enumerate the actual
   trim shapes the four rows author (rear_wing louvres, suspension
   `_plate`, steering_rack) and admit exactly those; anything beyond them
   refuses typed naming the shape.
2. Facts stay on the landed per-patch machinery: a trim-extrude row is
   green when its construction is admitted AND its bracket certifies AND
   the mesh emits. A trim approximation is FORBIDDEN — if exactness cannot
   be reached for a shape, refuse typed naming it.
3. door.py is in the write set only for the vocabulary table. No corpus
   tree files.

## Method

Enumerate the four rows' trim shapes from the corpus source (read-only via
door spot-checks and the tree; do not edit the trees), admit them one at a
time with a green round-trip each (record -> bd_facts bracket + bd_stl),
then the composition paths. Tests: one green round-trip per row; one typed
refusal for a shape outside the admitted envelope.

## Done when

```
cargo fmt --check -p truck123d
cargo clippy -p truck123d --all-targets -- -D warnings
cargo test -p truck123d --test trim_extrude_envelope --locked
```

plus door spot-checks per claimed row. All cargo through the queue (the
`cargo` on PATH IS the queue shim); interpreter dir on PATH if
STATUS_DLL_NOT_FOUND.

## Forbidden

Any file outside `write_allow`. Editing `corpus/ttc/trees/**` or
`vendor/truck/**`. Numeric trim approximations. `#[ignore]`, deleted or
weakened tests, bare `cargo test`. Committing to `main`.

## Stop conditions

- a trim shape that cannot be admitted exactly → typed refusal + `SPEC_GAP` naming the shape
- anchor count differs at dispatch and was not re-measured → `ANCHOR_MISMATCH`
- three consecutive failed cargo runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` AT THE WORKTREE ROOT

```json
{"id":"FHC-TRIM-EXTRUDE-ENVELOPE","status":"DONE","contracts":["FHC-TRIM-EXTRUDE-ENVELOPE"],
 "anchors_verified":{"A1":9,"A2":1},
 "rows_flipped":["f1/rear_wing","f1/steering_rack","f1/suspension_front","f1/suspension_rear"],
 "notes":"per-row verdict + bracket"}
```

Commit on the current branch, subject
`truck123d: trim-extrude envelope extension (FHC-TRIM)`.
