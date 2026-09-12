# WORK PACKET FHC-B-MULTI-CONTOUR-SECTIONS — profiles with holes (annular sections)

Gap-register discovery 2026-09-13 (interim census): `hypercar/brakes` and
`f1/suspension_front` refuse at planar-profile booleans that are, on
inspection, NOT booleans at all — they are **two-contour sections**
(outer ring + inner ring: an annulus). The landed section representation is
single-ring (`orient_profile_ring` / `profile_loop`). This packet adds
multi-contour sections to the landed carriers. **No new theory**: the
divergence-form flux is orientation-driven — an inner ring with reversed
winding contributes negative flux through the same per-patch certified
machinery.

```yaml
id:          FHC-B-MULTI-CONTOUR-SECTIONS
contract:    [FHC-B-MULTI-CONTOUR-SECTIONS]
class:       mechanical+
crates:      [truck123d]
depends_on:  [FHC-G5-SWALLOWED-REFUSAL-DIAGNOSIS]
needs:       [FHC-G5-SWALLOWED-REFUSAL-DIAGNOSIS]
write_allow:
  - corpus/ttc/door.py
  - truck123d/src/bd_bridge.rs
  - truck123d/tests/multi_contour_sections.rs
read_allow:
  - docs/F1_HYPERCAR_GAP_REGISTER.md
tests_required: [truck123d/tests/multi_contour_sections.rs]
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'fn orient_profile_ring' truck123d/src/bd_bridge.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'fn profile_loop' truck123d/src/bd_bridge.rs"}
  - {id: A3, expect: 0, cmd: "grep -c 'multi_contour' truck123d/src/bd_bridge.rs"}
budget:      {turns: 55, ctx_tokens: 170000}
```

Anchors measured 2026-09-13 at the G4-landed HEAD. **Re-measure at
dispatch** — G6/G1 land ahead and WILL drift counts; update at dispatch,
never dispatch stale.

## Pre-made judgements

1. **Section representation:** a section becomes one or more oriented
   rings. Each ring orients independently via the landed
   `orient_profile_ring`; winding is normalized (outer CCW, inner CW) so
   the flux of the hole contributes negatively through the SAME per-patch
   certified path. Contour correspondence and nesting validation (which
   ring encloses which, no crossings) are validation logic — refuse TYPED
   (`E_NESTING_INVALID` via the landed code table) on anything ambiguous.
2. **Caps:** the annular cap is the **quad strip between corresponding
   rings** — no centroid fan, which sidesteps the 4b degenerate-normal-cone
   class entirely. Scope this packet to PLANAR caps (brakes is a revolve);
   non-planar multi-contour caps refuse typed naming the case.
3. **Facts:** no new facts path — more patches through the landed
   per-patch certified machinery. A revolved annulus's bracket must equal
   the analytic `pi*(r1^2-r0^2)*h` (the known-answer validation, exact).
4. **Mesh:** annular grids through the existing registry; index-identity
   convention unchanged.
5. **Faces with holes upstream:** the door's `make_face` over a two-ring
   sketch records both contours; the `face_boolean` refusal for the
   Circle-Circle sketch is REPLACED by the recorded two-contour section.
   No 2D boolean machinery in this packet.

## Method

Multi-ring section representation -> orientation/winding -> lathe path
(brakes' annular revolve) with the analytic known-answer test -> extrude
path -> the planar-profile boolean refusal sites re-aimed at the recorded
two-contour section. Tests: the analytic annulus (revolve, exact bracket ==
closed form); annular extrude; nesting validation refusals (crossing rings,
ring outside ring); mesh annulus grid; the two census spot-checks
(brakes, suspension_front) green or typed-refusing at their NEXT honest
carrier.

## Done when

```
cargo fmt --check -p truck123d
cargo clippy -p truck123d --all-targets -- -D warnings
cargo test -p truck123d --test multi_contour_sections --locked
```

plus door spot-checks for `hypercar/brakes` and `f1/suspension_front` with
records in RESULT. All cargo through the queue (the `cargo` on PATH IS the
queue shim); interpreter dir on PATH if STATUS_DLL_NOT_FOUND.

## Forbidden

Any file outside `write_allow`; editing `corpus/ttc/trees/**` or
`vendor/truck/**`; 2D boolean machinery (out of scope by design); numeric
shortcuts; `#[ignore]`, deleted or weakened tests, bare `cargo test`;
committing to `main`.

## Stop conditions

- a corpus sketch genuinely needs a 2D boolean (not a two-contour section) → typed refusal naming it + `SPEC_GAP` (that books the real 2D-boolean packet)
- bracket mismatch on the analytic annulus → `BLOCKED` (that is a soundness bug, not a gap)
- anchor count differs at dispatch and was not re-measured → `ANCHOR_MISMATCH`
- three consecutive failed cargo runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` AT THE WORKTREE ROOT

```json
{"id":"FHC-B-MULTI-CONTOUR-SECTIONS","status":"DONE","contracts":["FHC-B-MULTI-CONTOUR-SECTIONS"],
 "anchors_verified":{"A1":1,"A2":1,"A3":0},
 "rows_flipped":[],"notes":"per-row verdict + bracket; analytic annulus equality recorded"}
```

Commit subject: `truck123d: multi-contour sections - profiles with holes (FHC-B)`.
