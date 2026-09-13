# WORK PACKET FHC-D-SURFACE-RESIDUE — the last uncovered admission surface (hypercar OCP-probe cluster + 4 names)

Admission-surface audit 2026-09-13 (`scratch/admission_surface_audit.json`):
of 38 `bd.*` verbs and 122 distinct method names across the manifest row
modules, exactly **10 call-sites remain uncovered**, all in one hypercar
script section plus four names. This packet closes the axis-1/axis-2
residual. After it, every name and method a row module can express is in
{admitted, typed-refused-with-code}.

```yaml
id:          FHC-D-SURFACE-RESIDUE
contract:    [FHC-D-SURFACE-RESIDUE]
class:       mechanical
crates:      [truck123d]
depends_on:  [FHC-G1-RATIONAL-FLUX]
needs:       [FHC-G1-RATIONAL-FLUX]
write_allow:
  - corpus/ttc/door.py
  - truck123d/src/bd_bridge.rs
  - truck123d/tests/surface_residue.rs
read_allow:
  - docs/F1_HYPERCAR_GAP_REGISTER.md
  - docs/REFUSALS.md
tests_required: [truck123d/tests/surface_residue.rs]
anchors:
  - {id: A1, expect: 2, cmd: "grep -cE '\\bPart\\b' corpus/ttc/door.py"}
  - {id: A2, expect: 2, cmd: "grep -cE '\\bSolid\\b' corpus/ttc/door.py"}
  - {id: A3, expect: 1, cmd: "grep -c 'VolumeProperties_s' corpus/ttc/trees/hypercar/src/lib/chassis.py"}
budget:      {turns: 45, ctx_tokens: 140000}
```

Anchors measured 2026-09-13 at the B-landed HEAD (A1/A2 count the module
*attribute* usage; the door does not answer them as constructors yet).
**Re-measure at dispatch.**

## The 10 call-sites (audit, verbatim)

- **Names (1 call each, hypercar):** `Part`, `Rectangle`, `Solid` —
  `Rectangle` parallels the landed `RectangleRounded`; `Part`/`Shape`/`Solid`
  are build123d class names used as type anchors/probes.
- **OCP-level methods (1 call each, one hypercar section):**
  `IsIdentity`, `Located`, `Mass`, `ShapeType`, `Transformation`,
  `VolumeProperties_s` — an OCC-style probe/transform block.
- **Data-row attribute:** `vertices` (3 calls, hypercar).

## Pre-made judgements

1. **Diagnose first:** one door spot-check with `--trace-carriers` on the
   owning row to capture the exact calling context (which object, which
   arguments) before admitting anything.
2. **Admit only where the kernel fact exists:**
   `Mass`/`VolumeProperties_s` -> the certified volume/bracket facts
   (already computed per part);
   `Located`/`Transformation` -> the landed placement algebra (exact
   isometries, the MONO-3 principle);
   `ShapeType`/`IsIdentity` -> type/identity rows from the recorded
   carrier kind;
   `vertices` -> the recorded section/edge samples (G3-adjacent);
   `Rectangle` -> the landed rectangle/rounded-rect profile vocabulary
   (unrounded: four line edges);
   `Part`/`Solid` -> constructor/anchor rows per their actual call shape.
3. **Anything whose call shape is not derivable from landed facts refuses
   TYPED** with the FHC-G7 metadata — an honest residual is a valid
   outcome; the audit's job is that nothing is untyped.
4. No corpus tree edits; no approximations; OCC references diagnostic.

## Method

Spot-check the owning row; admit the ten sites one at a time (green
round-trip or typed refusal each); tests: one per admitted site (facts or
identity asserted), typed-refusal tests for the residual, and a full-row
door spot-check for the owning row.

## Done when

```
cargo fmt --check -p truck123d
cargo clippy -p truck123d --all-targets -- -D warnings
cargo test -p truck123d --test surface_residue --locked
```

plus the door spot-checks in RESULT. All cargo through the queue (the
`cargo` on PATH IS the queue shim); interpreter dir on PATH if
STATUS_DLL_NOT_FOUND.

## Forbidden

Any file outside `write_allow`; editing `corpus/ttc/trees/**` or
`vendor/truck/**`; `#[ignore]`, deleted or weakened tests, bare
`cargo test`; committing to `main`.

## Stop conditions

- a call site needs machinery beyond the landed facts/placement algebra → typed refusal + `SPEC_GAP` naming it
- anchor count differs at dispatch and was not re-measured → `ANCHOR_MISMATCH`
- three consecutive failed cargo runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` AT THE WORKTREE ROOT

```json
{"id":"FHC-D-SURFACE-RESIDUE","status":"DONE","contracts":["FHC-D-SURFACE-RESIDUE"],
 "anchors_verified":{"A1":2,"A2":2,"A3":1},
 "rows_flipped":[],"notes":"per-site disposition (admitted/typed); owning row verdict"}
```

Commit subject: `truck123d: last uncovered admission surface - hypercar OCP-probe cluster + 4 names (FHC-D)`.
