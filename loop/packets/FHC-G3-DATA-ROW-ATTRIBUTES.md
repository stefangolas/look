# WORK PACKET FHC-G3 — drop-in data-row attributes (Face.faces, Plane.origin, Edge.edge, corner Face bbox)

Gap-register category 3 (`docs/F1_HYPERCAR_GAP_REGISTER.md`): the drop-in's
`Face`/`Plane`/`Edge` data rows do not answer the attributes the corpus's
client-layer algebra reads, so 8 rows refuse typed (or die in untyped
AttributeErrors) at marshalling, not at geometry. All carrier machinery is
landed; this packet is the marshalling.

```yaml
id:          FHC-G3-DATA-ROW-ATTRIBUTES
contract:    [FHC-G3-DATA-ROW-ATTRIBUTES]
class:       mechanical
crates:      [truck123d]
depends_on:  [BD-EMIT-MESH-CACHE]
needs:       [BD-EMIT-MESH-CACHE]
write_allow:
  - corpus/ttc/door.py
  - truck123d/src/bd_bridge.rs
  - truck123d/tests/data_row_attributes.rs
read_allow:
  - docs/F1_HYPERCAR_GAP_REGISTER.md
  - docs/TTC_CENSUS_FINAL.md
tests_required: [truck123d/tests/data_row_attributes.rs]
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'class Face' corpus/ttc/door.py"}
  - {id: A2, expect: 1, cmd: "grep -c 'class Plane' corpus/ttc/door.py"}
  - {id: A3, expect: 1, cmd: "grep -c 'class Edge' corpus/ttc/door.py"}
  - {id: A4, expect: 8, cmd: "grep -c 'wrapped' corpus/ttc/door.py"}
budget:      {turns: 50, ctx_tokens: 150000}
```

Anchors measured 2026-09-12 at the EX-B-landed HEAD. **Re-measure at
dispatch** — TRIM/MIRROR/BD-EMIT-MESH-CACHE land ahead and WILL drift
counts; update the packet at dispatch, never dispatch stale.

## Rows this closes (interim census verdicts, verbatim)

- `hypercar/details`, `hypercar/lighting` — `AttributeError: 'Face' object has no attribute 'faces'`
- `hypercar/suspension_front` — `AttributeError: 'Plane' object has no attribute 'origin'`
- `hypercar/suspension_rear` — `AttributeError: 'Edge' object has no attribute 'edge'`
- `f1/corner_fl`, `corner_fr`, `corner_rl`, `corner_rr` — the corner
  `Face` bounds-probe carrier refuses `unsupported_envelope`

## Pre-made judgements

1. **Attribute answers come from kernel facts already certified.** A
   `Face.faces` iteration maps to the part's face payloads the bridge
   already materializes for facts/mesh; `Plane.origin` (and the frame
   attributes the corpus reads) map to the landed placement/frame algebra;
   `Edge.edge` maps to the recorded kernel edge. Do not invent new
   geometry: if an attribute's answer is not derivable from landed facts,
   refuse TYPED naming it (that is a SPEC_GAP for the register, not a
   numeric shortcut).
2. **Corner rows:** the corners' `surfaces.bbox` reads a drop-in `Face` —
   answer the Face bounds probe from the carrier-derived bbox (the
   documented N>2 control-net ENCLOSURE is the certified answer; OCC
   parity is a diagnostic, not a gate).
3. door.py attribute surface and bd_bridge backing change together in this
   packet; keep every refusal typed. No corpus tree files.

## Method

One attribute class at a time end-to-end (Face.faces first — it unblocks two
hypercar rows), with a green door round-trip per row before the next. Tests:
one green round-trip per claimed row (facts bracket + STL), one typed
refusal naming the missing answer for anything left red.

## Done when

```
cargo fmt --check -p truck123d
cargo clippy -p truck123d --all-targets -- -D warnings
cargo test -p truck123d --test data_row_attributes --locked
```

plus door spot-checks per claimed row. All cargo through the queue (the
`cargo` on PATH IS the queue shim); interpreter dir on PATH if
STATUS_DLL_NOT_FOUND.

## Forbidden

Any file outside `write_allow`; editing `corpus/ttc/trees/**` or
`vendor/truck/**`; inventing geometry not backed by landed facts;
`#[ignore]`, deleted or weakened tests, bare `cargo test`; committing to
`main`.

## Stop conditions

- an attribute is not derivable from landed facts → typed refusal + `SPEC_GAP` naming it
- anchor count differs at dispatch and was not re-measured → `ANCHOR_MISMATCH`
- three consecutive failed cargo runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` AT THE WORKTREE ROOT

```json
{"id":"FHC-G3-DATA-ROW-ATTRIBUTES","status":"DONE","contracts":["FHC-G3-DATA-ROW-ATTRIBUTES"],
 "anchors_verified":{"A1":1,"A2":1,"A3":1,"A4":8},
 "rows_flipped":[],"notes":"per-row verdict + bracket"}
```

Commit subject: `truck123d: drop-in data-row attributes for corpus idioms (FHC-G3)`.
