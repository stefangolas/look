# WORK PACKET FHC-MIRROR — the mirror carrier form

Owner roadmap 2026-09-11 (SHORT-TERM ROADMAP, step 4). The last red F1 row
outside the extraction class. AUTHOR-WIRE-MIRROR landed wire/mirror in the
door; `f1/drs_actuator` still refuses at the mirror carrier — the row
mirrors a kernel SOLID/member row, not a wire. Flips toward green:
`f1/drs_actuator`.

```yaml
id:          FHC-MIRROR-FORM
contract:    [FHC-MIRROR-FORM]
class:       mechanical
crates:      [truck123d]
depends_on:  [FHC-TRIM-EXTRUDE-ENVELOPE]
write_allow:
  - truck123d/src/bd_bridge.rs
  - corpus/ttc/door.py
  - truck123d/tests/mirror_form.rs
read_allow:
  - docs/TTC_CENSUS_FINAL.md
tests_required: [truck123d/tests/mirror_form.rs]
anchors:
  - {id: A1, expect: 1,  cmd: "grep -c 'fn mirrored_part' truck123d/src/bd_bridge.rs"}
  - {id: A2, expect: 25, cmd: "grep -cE 'mirror[^A-Za-z0-9_]|mirror$' corpus/ttc/door.py"}
budget:      {turns: 40, ctx_tokens: 120000}
```

Anchors measured 2026-09-11 at HEAD `cb1fabf` — **re-measure at dispatch**;
prior landings WILL have drifted A1/A2. Update at dispatch, never dispatch
stale.

## R3 verdict this packet closes (verbatim)

- `f1/drs_actuator` — `kernel refusal: unsupported_envelope` (mirror carrier)

## Pre-made judgements

1. Mirror of a kernel row is an EXACT ISOMETRY: facts transform exactly
   (volume invariant, bbox mirrors, mesh mirrors by vertex reflection with
   reversed winding). This is the landed mirror principle (MONO-3
   `mirror_y` of kernel rows) extended to the form drs_actuator actually
   uses — mirror across a plane other than y=0, or mirror of a composed
   member. Read the row's corpus source via door spot-checks; admit the
   form it uses; refuse typed anything beyond it.
2. No new facts path: the mirror composes the landed placement algebra
   (translate ∘ R ∘ mirror, exact). If the row's mirror is not an exact
   isometry in the landed algebra, that is a `SPEC_GAP`, not a numeric
   shortcut.
3. door.py is in the write set only for the vocabulary table. No corpus
   tree files.

## Method

Diagnose the exact mirror form (one door run on `drs_actuator` recording
the refusal), admit that form in bd_bridge with a green round-trip
(record -> bd_facts bracket + bd_stl), then the composition paths around
it. Tests: one green round-trip for the admitted form; one typed refusal
for a form outside it; volume-invariance and bbox-mirror assertions on the
mirrored facts.

## Done when

```
cargo fmt --check -p truck123d
cargo clippy -p truck123d --all-targets -- -D warnings
cargo test -p truck123d --test mirror_form --locked
```

plus the door spot-check on `f1/drs_actuator`. All cargo through the queue
(the `cargo` on PATH IS the queue shim); interpreter dir on PATH if
STATUS_DLL_NOT_FOUND.

## Forbidden

Any file outside `write_allow`. Editing `corpus/ttc/trees/**` or
`vendor/truck/**`. `#[ignore]`, deleted or weakened tests, bare
`cargo test`. Committing to `main`.

## Stop conditions

- the row's mirror is not expressible in the landed exact-isometry algebra → `SPEC_GAP`
- anchor count differs at dispatch and was not re-measured → `ANCHOR_MISMATCH`
- three consecutive failed cargo runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` AT THE WORKTREE ROOT

```json
{"id":"FHC-MIRROR-FORM","status":"DONE","contracts":["FHC-MIRROR-FORM"],
 "anchors_verified":{"A1":1,"A2":25},
 "rows_flipped":["f1/drs_actuator"],
 "notes":"the mirror form admitted; facts invariance recorded"}
```

Commit on the current branch, subject
`truck123d: mirror carrier form for drs_actuator (FHC-MIRROR)`.
