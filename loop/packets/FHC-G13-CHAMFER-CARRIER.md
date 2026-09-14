# WORK PACKET FHC-G13 — chamfer carrier: the named-but-unexecuted vocabulary entry

`chamfer` sits in the door's CENSUS_NAMES vocabulary (`bd_bridge.rs:11168`)
but has no executor arm — it refuses typed on use, while its sibling `fillet`
has a landed certified blend path. This packet lands the chamfer carrier so
the name answers with geometry, not a refusal.

**Normative theory:** Certified Flux Calculus via
`docs/CERTIFIED_FLUX_CALCULUS_INTEGRATION_MAP.md`. The mechanistic insight:
**a chamfer face on a straight edge between planar faces is itself planar** —
its flux is exact via the planarity router (G8) or, before G8 lands, the
landed exact planar end-cap pattern. The carrier work is construction:
replace the edge region with the chamfer quad as an admitted carrier,
decomposing to landed arms (booleans + extrude/loft), no new integration.

```yaml
id:          FHC-G13-CHAMFER-CARRIER
contract:    [FHC-G13-CHAMFER-CARRIER]
class:       mechanical+
crates:      [truck123d]
depends_on:  []
needs:       []
write_allow:
  - truck123d/src/bd_bridge.rs
  - truck123d/tests/chamfer_carrier.rs
read_allow:
  - docs/F1_HYPERCAR_GAP_REGISTER.md
  - docs/CERTIFIED_FLUX_CALCULUS_INTEGRATION_MAP.md
tests_required: [truck123d/tests/chamfer_carrier.rs]
anchors:
  - {id: A1, expect: 6, cmd: "grep -c 'cell_flux_exact' truck123d/src/bd_bridge.rs"}
  - {id: A2, expect: 4, cmd: "grep -c 'boolean_product_volume_certified' truck123d/src/bd_bridge.rs"}
budget:      {turns: 45, ctx_tokens: 180000}
```

Anchors measured 2026-09-13 at HEAD. **Re-measure at dispatch.**

## Pre-made judgements

1. **Decompose, don't invent.** A straight-edge chamfer between planar faces
   is the planar quad `(edge, offset distance d1/d2)` — construct the
   chamfered solid through landed arms (the trimmed-plate pattern of
   FHC-TRIM / the boolean funnel), not a new surface class. Its faces are
   planar → the exact flux machinery certifies them unchanged.
2. **Curved-edge chamfers are OUT OF SCOPE** — a chamfer on a spline/rotational
   edge produces a non-planar face class with no admitted carrier: refuse
   typed `unimplemented_carrier` naming it (register evidence), never
   approximate.
3. **No corpus row currently calls chamfer** — the deliverable is API
   completeness for text-to-cad generated scripts (the vocabulary promises
   the name). The done-when is the carrier + tests, not a row flip.
4. Determinism and byte-identity: existing rows' fingerprints unchanged.

## Tests required (`truck123d/tests/chamfer_carrier.rs`)

1. `chamfer_box_edge_exact` — a unit box with one straight-edge chamfer:
   green bracket; the exact volume is independently derivable (box minus
   prism) and matches bit-for-bit.
2. `chamfer_volume_subtraction_identity` — chamfered solid volume equals
   base volume minus the chamfer prism volume, computed via the certified
   path on both sides (cross-validation).
3. `curved_edge_refuses_typed` — a spline-edge chamfer refuses
   `unimplemented_carrier` with full v2 metadata; never silent.
4. `regression_byte_identity` — previously green rows unchanged.
5. `door_name_answers` — `chamfer` through the door drop-in constructs the
   chamfered solid (no refusal) for the straight-edge case.

## Done when

```
cargo fmt --check -p truck123d
cargo clippy -p truck123d --all-targets -- -D warnings
cargo test -p truck123d --test chamfer_carrier --locked
```

All cargo through the queue.

## Forbidden

Any file outside `write_allow`; editing `corpus/ttc/**` or `vendor/truck/**`;
approximation of curved-edge chamfers; `#[ignore]`, deleted or weakened
tests, bare `cargo test`; committing to `main`.

## Stop conditions

- the chamfer quad cannot be constructed through landed arms within the
  envelope → `SPEC_GAP` naming the missing primitive
- anchor count differs at dispatch and was not re-measured → `ANCHOR_MISMATCH`
- three consecutive failed cargo runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` AT THE WORKTREE ROOT

```json
{"id":"FHC-G13-CHAMFER-CARRIER","status":"DONE","contracts":["FHC-G13-CHAMFER-CARRIER"],
 "anchors_verified":{"A1":6,"A2":4},
 "rows_flipped":[],
 "notes":"straight-edge carrier; curved-edge typed refusal; suite 1-5"}
```

Commit subject: `truck123d: chamfer carrier - straight-edge exact, curved-edge typed refusal (FHC-G13)`.
