# WORK PACKET FHC-EX-A — closed-loop loft chains + shelled/composed loft bodies

Owner roadmap 2026-09-11 (SHORT-TERM ROADMAP, step 2, packet 1 of the
extraction-breadth pair). Flips `f1/halo`, `f1/nose`, `f1/cockpit`,
`f1/sidepod_left`, `f1/sidepod_right` toward the green predicate (annex C:
kernel constructs + certified bracket + constructive solid_count +
carrier-derived bbox + mesh emits; OCC refs diagnostic only).

```yaml
id:          FHC-EX-A-CLOSED-LOOP-SHELL
contract:    [FHC-EX-A-CLOSED-LOOP-SHELL]
class:       design
crates:      [truck123d]
depends_on:  [AUTHOR-CENSUS-NAMES]
write_allow:
  - truck123d/src/bd_bridge.rs
  - corpus/ttc/door.py
  - truck123d/tests/extraction_breadth_a.rs
read_allow:
  - truck123d/src/facade.rs
  - truck123d/src/binding.rs
  - docs/TTC_CENSUS_FINAL.md
  - docs/MONO_CLOSURE_BOOKING.md
tests_required: [truck123d/tests/extraction_breadth_a.rs]
anchors:
  - {id: A1, expect: 1,  cmd: "grep -c 'closed-loop' corpus/ttc/door.py"}
  - {id: A2, expect: 1,  cmd: "grep -c 'closed_loop' truck123d/src/bd_bridge.rs"}
  - {id: A3, expect: 3,  cmd: "grep -cE 'shell[^A-Za-z0-9_]|shell$' truck123d/src/bd_bridge.rs"}
budget:      {turns: 70, ctx_tokens: 220000}
```

Anchors measured 2026-09-11 at HEAD `cb1fabf`. **Re-measure at dispatch** —
anchor drift is per-landing (three paid for on 2026-09-10); if a count
differs after re-measure, UPDATE the packet before dispatching, never
dispatch a stale anchor.

## R3 verdicts this packet closes (verbatim, `docs/TTC_CENSUS_FINAL.md`)

- `f1/halo` — typed-refusal: `kernel refusal: unsupported_envelope` (closed-loop loft chain)
- `f1/nose` — typed-refusal: `kernel refusal: unsupported_envelope` (boolean-composed swept carriers)
- `f1/sidepod_left` / `f1/sidepod_right` — typed-refusal: same
- `f1/cockpit` — DNF: `RuntimeError: cockpit: loft failed` (`lib/cockpit.py`
  loft builder; `is_valid_shape` swallows the drop-in refusal — record the
  typed refusal at the door if the carrier is still open, never tune the corpus)

## Pre-made judgements

1. **Closed-loop loft chain** = an N-station loft whose section is a CLOSED
   loop of mixed line/arc/spline edges where the loop closes on its start
   vertex (halo's section band). The landed MONO-2 N-station loft takes
   single-profile sections; the extension is the SECTION representation
   (a closed ring of ProfileEdges), not a new facts path. Reuse the pinned
   canonical loft convention (annex A correction) unchanged — chord-length
   stations, C2 across them, per-patch certificate brackets.
2. **Shelled/composed loft bodies** (nose/sidepods) = a loft (or swept
   carrier) MINUS its inner offset, plus boolean composition. The outer
   surface extraction is this packet's; the boolean tail rides the landed
   MONO-6/MONO-8/MONO-9 swept-boolean machinery via
   `dispatch_swept_carrier_boolean` — submit operands, do not reimplement.
3. **Honesty line:** a carrier that cannot be extracted exactly refuses
   TYPED naming the open carrier. No tolerance stretch, no approximant
   replication (annex C). The `cockpit` DNF must never become green by
   corpus-side loosening.
4. door.py is in the write set ONLY to record new carrier rows in the
   vocabulary table and keep refusals typed. No corpus tree files.

## Method

One row end-to-end (record -> bd_facts bracket + bd_stl emits) before the
next: halo (closed-loop) first — it is the new representation; then nose,
then the two sidepods (same shape mirrored), then cockpit's diagnosis
recorded honestly. Tests: one green round-trip per closed row (facts +
mesh), one typed refusal naming the open carrier if any row stays red.

## Done when

```
cargo fmt --check -p truck123d
cargo clippy -p truck123d --all-targets -- -D warnings
cargo test -p truck123d --test extraction_breadth_a --locked
```

plus the packet's door spot-checks: `python corpus/ttc/door.py --engine truck
corpus/ttc/trees/f1/src <module> <entry> <args> <stl>` for each claimed row,
`ok: true` with `volume_lo == volume_hi`, or a typed refusal recorded in
RESULT notes. All cargo through the queue (the `cargo` on PATH IS the queue
shim); DLL workaround: interpreter dir on PATH if STATUS_DLL_NOT_FOUND.

## Forbidden

Any file outside `write_allow`. Editing `corpus/ttc/trees/**`. Changing the
pinned loft convention. `#[ignore]`, deleted/weakened tests, bare `cargo
test`. Committing to `main`.

## Stop conditions

- anchor count differs at dispatch and you were not told to re-measure → `ANCHOR_MISMATCH`
- a carrier cannot be admitted exactly and the typed refusal cannot be expressed in the current vocabulary → `SPEC_GAP`
- three consecutive failed cargo runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` AT THE WORKTREE ROOT

```json
{"id":"FHC-EX-A-CLOSED-LOOP-SHELL","status":"DONE","contracts":["FHC-EX-A-CLOSED-LOOP-SHELL"],
 "anchors_verified":{"A1":1,"A2":1,"A3":3},
 "rows_flipped":["f1/halo","f1/nose","f1/sidepod_left","f1/sidepod_right"],
 "notes":"per-row verdict + bracket; cockpit disposition"}
```

Commit on the current branch, subject
`truck123d: closed-loop loft chains + shelled loft bodies (FHC-EX-A)`.
