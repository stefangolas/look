# WORK PACKET FHC-G4 — named-carrier admission: revolve profile closure, Color name, mirrored band loft

Gap-register category 4: hypercar rows that progressed past their R3 name
gaps (CENSUS-NAMES landed) and now stop at new, NAMED carriers. Flips toward
green: `hypercar/brakes`, `hypercar/hinge`, `hypercar/wheels`,
`hypercar/body`, `hypercar/chassis`, `hypercar/glazing`, `hypercar/interior`.

```yaml
id:          FHC-G4-NAMED-CARRIER-ADMISSION
contract:    [FHC-G4-NAMED-CARRIER-ADMISSION]
class:       mechanical+
crates:      [truck123d]
depends_on:  [FHC-G2-PROBE-QUERIES]
needs:       [FHC-G2-PROBE-QUERIES]
write_allow:
  - corpus/ttc/door.py
  - truck123d/src/bd_bridge.rs
  - truck123d/tests/named_carrier_admission.rs
read_allow:
  - docs/F1_HYPERCAR_GAP_REGISTER.md
  - docs/MONO_CLOSURE_BOOKING.md
tests_required: [truck123d/tests/named_carrier_admission.rs]
anchors:
  - {id: A1, expect: 2, cmd: "grep -c 'revolve needs a closed profile' corpus/ttc/door.py"}
  - {id: A2, expect: 0, cmd: "grep -c 'Color' corpus/ttc/door.py"}
  - {id: A3, expect: 1, cmd: "grep -c 'class Vector' corpus/ttc/door.py"}
budget:      {turns: 70, ctx_tokens: 220000}
```

Anchors measured 2026-09-12 at the EX-B-landed HEAD. **Re-measure at
dispatch** — the docket ahead WILL drift them; update at dispatch, never
dispatch stale.

## Rows and their named carriers (interim census, verbatim)

- `hypercar/brakes`, `hypercar/hinge` — `revolve needs a closed profile`
  (0.11 s, instant): the profile the row composes does not arrive at the
  lathe arm as a closed loop.
- `hypercar/wheels` — `AttributeError: module 'bd' has no attribute 'Color'`
  (a name-surface gap again — the trivial bucket).
- `hypercar/body`, `chassis`, `glazing`, `interior` —
  `unsupported_envelope` (1.5–3.7 s): the mirrored spline band-loft carrier
  (`surfaces._mirrored_face` section bands feeding the 24–26-station lofts).

## Pre-made judgements

1. **Revolve profile closure:** the landed lathe arm is exact; the gap is
   the profile COMPOSITION arriving open. Admit the closed-profile
   composition the rows actually author (read the corpus source via door
   spot-checks; likely a face/loop construction the landed
   `orient_profile_ring` machinery already handles) — or refuse typed
   naming the exact missing verb. No profile approximation.
2. **`bd.Color`:** name-surface answer (a color data row), same class
   CENSUS-NAMES closed. It feeds the GLB emit path's per-part color, which
   is landed.
3. **Mirrored band loft:** EX-A landed `orient_profile_ring` /
   `spline_loop_spans` (closed-loop sections) and MONO-2 the canonical
   N-station convention. The band variant is a section set built from a
   face MIRRORED about a plane — admit it by composing the landed exact
   mirror with the landed loft, feeding the SAME facts path. If the
   mirrored band needs a facts path that does not exist, refuse typed and
   SPEC_GAP it (that would be register category 1/5 reclassification
   evidence, which is a valid outcome).
4. One row end-to-end (door round-trip: facts bracket + STL) before the
   next; easiest first: `wheels` (name), then `brakes`/`hinge` (profile),
   then the band-loft quartet.

## Method

Door spot-check each row to record the exact refusing verb in context;
admit or refuse-typed per the judgements; green round-trip per claimed row.
Tests: one green round-trip per claimed row; the mirrored-band composition
test (mirror a landed loft's section, certify equal-and-opposite facts);
one typed refusal for a profile that genuinely does not close.

## Done when

```
cargo fmt --check -p truck123d
cargo clippy -p truck123d --all-targets -- -D warnings
cargo test -p truck123d --test named_carrier_admission --locked
```

plus door spot-checks per claimed row. All cargo through the queue (the
`cargo` on PATH IS the queue shim); interpreter dir on PATH if
STATUS_DLL_NOT_FOUND.

## Forbidden

Any file outside `write_allow`; editing `corpus/ttc/trees/**` or
`vendor/truck/**`; profile or loft approximations; `#[ignore]`, deleted or
weakened tests, bare `cargo test`; committing to `main`.

## Stop conditions

- the mirrored band loft needs machinery beyond composing the landed mirror + loft → typed refusal + `SPEC_GAP` naming it
- anchor count differs at dispatch and was not re-measured → `ANCHOR_MISMATCH`
- three consecutive failed cargo runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` AT THE WORKTREE ROOT

```json
{"id":"FHC-G4-NAMED-CARRIER-ADMISSION","status":"DONE","contracts":["FHC-G4-NAMED-CARRIER-ADMISSION"],
 "anchors_verified":{"A1":2,"A2":0,"A3":1},
 "rows_flipped":[],"notes":"per-row verdict + bracket"}
```

Commit subject: `truck123d: named-carrier admission for hypercar walls (FHC-G4)`.
