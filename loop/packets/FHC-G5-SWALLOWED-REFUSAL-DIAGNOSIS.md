# WORK PACKET FHC-G5 — surface the swallowed refusals, then classify (floor, diffuser, aero, powertrain)

Gap-register category 5: four rows whose blockers are UNIDENTIFIED because a
corpus builder swallows the drop-in refusal (`f1/floor`, `f1/diffuser` —
`RuntimeError: floor loft failed: None`) or the refusal is unexamined
(`hypercar/aero` — `case empty`; `hypercar/powertrain` — early envelope past
its landed RectangleRounded name). The deliverable is CLASSIFICATION: typed
refusals surfaced, each row re-bucketed into the register, and any blocker
that maps to landed machinery admitted on the spot.

```yaml
id:          FHC-G5-SWALLOWED-REFUSAL-DIAGNOSIS
contract:    [FHC-G5-SWALLOWED-REFUSAL-DIAGNOSIS]
class:       mechanical+
crates:      [truck123d]
depends_on:  [FHC-G4-NAMED-CARRIER-ADMISSION]
needs:       [FHC-G4-NAMED-CARRIER-ADMISSION]
write_allow:
  - corpus/ttc/door.py
  - truck123d/src/bd_bridge.rs
  - truck123d/tests/swallowed_refusal_diagnosis.rs
  - truck123d/tests/refusal_metadata.rs
  - truck123d/tests/extraction_breadth_a.rs
read_allow:
  - docs/F1_HYPERCAR_GAP_REGISTER.md
  - corpus/ttc/trees/f1/src/lib/floor.py
  - corpus/ttc/trees/f1/src/lib/diffuser.py
tests_required: [truck123d/tests/swallowed_refusal_diagnosis.rs]
anchors:
  - {id: A1, expect: 2, cmd: "grep -c 'is_valid_shape' corpus/ttc/door.py"}
budget:      {turns: 55, ctx_tokens: 170000}
```

Anchor measured 2026-09-12 (A1=0: door.py carried no `is_valid_shape`).
**DRIFTED, re-measured 2026-09-13 (A1=2): the FHC-G2 landing added
`is_valid_shape` answers to door.py — the hygiene premise is partially
resolved by landed work.** The diagnosis step stands; verify what floor/
diffuser now do through the door before assuming the swallow persists.
Re-measure at dispatch.

## Pre-made judgements

1. **Hygiene first, and it is door-side only:** the corpus builder
   (`floor._loft_stack`'s `is_valid_shape`) swallows the drop-in refusal.
   The door cannot edit corpus trees — instead the door's refusal path must
   make the swallowed case VISIBLE: a typed refusal that propagates through
   the builder's probe (e.g. the probe idiom from FHC-G2 answering
   honestly, or a typed marker the builder's check re-raises). If the
   builder still swallows after the honest probe answer, record the
   residual untyped error verbatim in RESULT — never edit the trees, never
   tune the corpus.
2. **Classify, then admit only the already-landed:** once floor/diffuser's
   real refusal is visible, admit it IF it maps to landed machinery (loft
   variants, ring orientation, probe answers); otherwise the typed refusal
   IS the deliverable and the register gets a new named row-blocker
   (category reclassification — a success, not a failure).
3. **`hypercar/aero` (`case empty`):** determine whether the band
   composition produces a genuinely degenerate section (fix at the carrier,
   honestly, if the composition is admissible) or an open carrier (typed
   refusal naming it).
4. **`hypercar/powertrain`:** door spot-check to name the first refusing
   verb past its landed name-gap; admit if it maps to landed machinery,
   else typed refusal naming it.
5. **AMENDED 2026-09-13 (owner session): stale test pins from the G4
   landing.** G4's admissions moved two row verdicts past their pinned
   literals: `truck123d/tests/refusal_metadata.rs` pins
   `hypercar/brakes` at `E_NEEDS_CLOSED_PROFILE` (now
   `E_UNSUPPORTED_ENVELOPE` at the face_boolean carrier) and
   `truck123d/tests/extraction_breadth_a.rs` pins `hypercar/cockpit` red
   (already green at the dispatch HEAD). Update both pins to the current
   honest verdicts (both files are in this packet's write allow), keeping
   every other assertion untouched.
5. One row at a time; every outcome (green or typed) is recorded with the
   door's verbatim record in RESULT.

## Method

floor first (surface the refusal → classify → admit-or-name), then
diffuser (same builder shape), then aero, then powertrain. Tests: the
typed-propagation test (a swallowed probe no longer produces an untyped
RuntimeError through the door path); one green round-trip per row whose
blocker maps to landed machinery; typed refusals recorded for the rest.

## Done when

```
cargo fmt --check -p truck123d
cargo clippy -p truck123d --all-targets -- -D warnings
cargo test -p truck123d --test swallowed_refusal_diagnosis --locked
```

plus door spot-checks per row with verbatim records in RESULT. All cargo
through the queue (the `cargo` on PATH IS the queue shim); interpreter dir
on PATH if STATUS_DLL_NOT_FOUND.

## Forbidden

Any file outside `write_allow`; **editing `corpus/ttc/trees/**`** (the
swallow is theirs; ours is visibility); tuning the corpus; approximations;
`#[ignore]`, deleted or weakened tests, bare `cargo test`; committing to
`main`.

## Stop conditions

- the corpus builder swallows even the honest typed answer → record verbatim; the classification stands on the door-side record; NOT a failure
- an unexaminable refusal (no record, no trace) → `BLOCKED`
- anchor count differs at dispatch and was not re-measured → `ANCHOR_MISMATCH`
- three consecutive failed cargo runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` AT THE WORKTREE ROOT

```json
{"id":"FHC-G5-SWALLOWED-REFUSAL-DIAGNOSIS","status":"DONE","contracts":["FHC-G5-SWALLOWED-REFUSAL-DIAGNOSIS"],
 "anchors_verified":{"A1":0},
 "rows_flipped":[],
 "classifications":{"f1/floor":"","f1/diffuser":"","hypercar/aero":"","hypercar/powertrain":""},
 "notes":"verbatim refusal records; register re-bucketing"}
```

Commit subject: `truck123d: surface swallowed refusals and classify floor/diffuser/aero/powertrain (FHC-G5)`.
