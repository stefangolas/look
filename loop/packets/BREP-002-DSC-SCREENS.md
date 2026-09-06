# WORK PACKET BREP-002-DSC-SCREENS — the boolean screen layer becomes certified enclosures

You are correcting the last of the open BRep generation defects: the boolean
entry's rejection screens under-cover curved carriers. Everything you need
is in this document and the normative record
`docs/defects/DSC-BOUNDARY-SAMPLE-EXTENT-001.md` — read it first. Do not
read other spec files. If something you need is genuinely missing, that is a
SPEC_GAP (see "Stop conditions"): you stop and report, you do not research it.

```yaml
id:          BREP-002-DSC-SCREENS
contract:    [BREP-002-DSC-SCREENS]
class:       design
crates:      [truck-shapeops]
depends_on:  [CTE-007-T2ARRANGE]
write_allow:
  - vendor/truck/truck-shapeops/src/boolean/assemble.rs
  - vendor/truck/truck-shapeops/tests/defect_regressions.rs
read_allow:
  - docs/defects/DSC-BOUNDARY-SAMPLE-EXTENT-001.md
  - vendor/truck/truck-evidence/src/enclosure.rs
  - vendor/truck/truck-evidence/src/enclosure_sweep.rs
  - vendor/truck/truck-shapeops/src/boolean/{split,classify,mod}.rs
tests_required:
  - dsc_boundary_sample_extent_001_sphere_cap_screen_admits_the_contact
  - dsc_boundary_sample_extent_001_planar_faces_screen_unchanged
budget:      {turns: 70, ctx_tokens: 150000}
```

## Problem

The boolean entry derives two extents from BOUNDARY-CURVE SAMPLES
(`face_aabb`, `face_uv_box` in `assemble.rs`) and consumes them as
enclosures: the 3-D AABB gates the cross-solid sweep, the parameter box
becomes the stratum's domain. For a curved carrier whose trim interior
bulges away from its boundary hull (spherical cap, loft panel), both
under-cover — the pair never reaches the contact oracle, and the boolean
completes with silently missing contact events. No refusal fires anywhere,
because the oracle was never reached. That is the cardinal failure class
(BG-ENC-001).

## Scope decisions — pre-made, do not relitigate

1. **The screen obligation**: `screen ⊇ S(T)` (world) and `□uv ⊇ T`
   (parameter). Over-estimation is acceptable (extra pairs admitted, refused
   typed downstream); UNDER-estimation is the defect.
2. **Replace the sampled extents with certified enclosures**: world screen =
   `EnclosureSurface::enclose(parameter_box)` per stratum (every carrier
   implements it — CL-000/CL-003 landed the spline and sweep forms); where
   width matters, the union of per-knot-span `enclose` boxes over the D1
   Bézier decomposition the carrier already paid for at admission.
3. **The uv twin derives from the trim's true parameter extent**, not the
   boundary-polygon hull.
4. **The `ee_circle_circle` guard STAYS** — the record's falsified-hypothesis
   section: a documented v1-envelope decision, not a defect.
5. **V5, absolute**: planar-faced booleans (where the sampled extent happens
   to be sound) keep byte-identical results; the only behavior change is
   curved-carrier pairs now reaching the oracle.
6. **Width regression is measured, not assumed**: the false-positive-pair
   rate on the corpus is recorded in RESULT (diagonal loft panels are the
   worst case for a whole-trim enclosure).

## Anchors — measured 2026-09-06 morning, re-check on your branch

| id | file | pattern | expect |
|---|---|---|---|
| A1 | `truck-shapeops/src/boolean/assemble.rs` | `fn face_aabb` | 1 |
| A2 | `truck-shapeops/src/boolean/assemble.rs` | `fn face_uv_box` | 1 |
| A3 | `truck-evidence/src/enclosure_sweep.rs` | `impl EnclosureSurface for SpineFrameSweep` | 1 |

## House rules

- **H-1** no unwrap/expect/panic reachable from geometry; **H-3** same-line
  `// H-3`; **H-6** never record `Float` as `Exact`.
- **Determinism**: fixed pair order; no hash ordering in output.
- **All cargo through the queue shim.** Scoped commands only:
  `cargo check -p truck-shapeops`, `cargo test -p truck-shapeops --lib
  boolean`, `cargo test -p truck-shapeops --test defect_regressions`.

## Tests required

1. `dsc_boundary_sample_extent_001_sphere_cap_screen_admits_the_contact` —
   the record's synthetic counterexample, run red→green: a sphere-cap solid
   minus a plane slab where the cap's apex contacts below its boundary
   circle; the screen admits the pair and the intersection fragment is
   present.
2. `dsc_boundary_sample_extent_001_planar_faces_screen_unchanged` — a
   planar-faced boolean's pair dispatch set is identical pre/post (V5).

No existing test may be deleted, `#[ignore]`d, or weakened.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-shapeops
cargo clippy -p truck-shapeops --all-targets -- -D warnings
cargo test -p truck-shapeops --lib boolean
cargo test -p truck-shapeops --test defect_regressions
```

Send cargo output to a file and read the tail.

## Forbidden

Anything outside `write_allow` — especially `boolean/{split,classify,mod}.rs`,
`tangency/**`, `truck-evidence/**` (read-only: the enclosure implementations
are landed), any landed test file, `Cargo.lock`. Adding `#[ignore]`.
Unjustified `#[allow]`. Committing to `main`.

## Stop conditions

- any anchor count differs → `ANCHOR_MISMATCH`
- the synthetic counterexample cannot be constructed with the landed
  carrier set → `SPEC_GAP`, naming the missing carrier form
- the corpus false-positive rate makes the boolean unusably slow →
  `SPEC_GAP` with the measured rate (the per-knot-span variant is the
  booked fallback — try it before stopping)
- three consecutive failed `cargo test` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` at the WORKTREE ROOT (then COMMIT first)

**COMMIT BEFORE writing `RESULT.json`.**

```json
{"id":"BREP-002-DSC-SCREENS","status":"DONE","contracts":["BREP-002-DSC-SCREENS"],
 "tests_added":2,"anchors_verified":{"A1":1,"A2":1,"A3":1},
 "notes":"screen variant used per carrier class; corpus false-positive-pair rate; planar V5 evidence; any API deviation"}
```

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit subject: `fix(shapeops): boolean screens become certified enclosures (BREP-002-DSC-SCREENS)`.
