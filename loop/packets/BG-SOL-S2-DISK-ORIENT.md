# WORK PACKET BG-SOL-S2-DISK-ORIENT - the disk-profile wall orientation fix

`extrude_profile` inverts every circle wall face unconditionally - the
HOLE convention. When the circle cycle is the material region's OUTER
boundary (the pure disk profile - the solid cylinder), that orients the
wall's effective normal INTO the material. The defect passes
`Solid::try_new` (closure only pairs edges), so it is silent. The M2
flagship needs `Extrude(Q)` (the circle profile extruded alone) as a
correctly-oriented input, and no existing S2 test covers it. If live code
contradicts this packet, report it in `disagreements`.

```json
{"id":"BG-SOL-S2-DISK-ORIENT","status":"DONE","contracts":["BG-SOL-S2-DISK-ORIENT"],
 "tests_added":1,"sites_migrated":0,"sites_deferred":0,
 "deviations":[],"disagreements":[],"baseline_failures":[],"notes":"free text"}
```

```yaml
id:          BG-SOL-S2-DISK-ORIENT
contract:    [BG-SOL-S2-DISK-ORIENT]
class:       mechanical
crates:      [truck-modeling]
write_allow:
  - vendor/truck/truck-modeling/src/extrude.rs
read_allow:
  - vendor/truck/truck-geometry/src/arrange.rs
tests_required:
  - extrude_disk_wall_normal_points_outward
budget:      {turns: 16, ctx_tokens: 60000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'cylinder_face.invert()' vendor/truck/truck-modeling/src/extrude.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub fn extrude_profile' vendor/truck/truck-modeling/src/extrude.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'for (ci, cycle) in material.boundaries.iter().enumerate()' vendor/truck/truck-modeling/src/extrude.rs"}
  - {id: A4, expect: 5, cmd: "grep -cF '#[test]' vendor/truck/truck-modeling/src/extrude.rs"}
  - {id: A5, expect: 6, cmd: "grep -c 'fn extrude_' vendor/truck/truck-modeling/src/extrude.rs"}
```

A1 becomes 0 (the unconditional call is replaced by the role-keyed form -
a `let mut cylinder_face = ...; if ci > 0 { cylinder_face.invert(); }`
shape keeps an `invert()` call but not the `cylinder_face.invert()` exact
string; if your shape keeps that exact string under the hole arm only,
report the observed count in RESULT.json - the intent is "the invert is
conditional", not the string). A4 becomes 6 and A5 becomes 7 (the new
test). A2 and A3 stay.

## Problem

The M2 flagship differential test compares `Extrude(P−Q)` (the direct
construction) against `boolean(Extrude(P), Difference, Extrude(Q))`.
`Extrude(Q)` - the circle profile extruded alone - produces a solid whose
cylinder wall carries the hole-wall convention: its effective normal is
−r̂ (pointing into the cylinder's material) instead of +r̂. Every
downstream consumer of face orientation (the Boundary Rewrite's
`MaterialState4` expresses witnesses in the fragment's ABSOLUTE
orientation) would misclassify the wall. A scratch probe
(`scratch/rwdiskprobe/src/main.rs`, preserved in the repo) demonstrated
all of this on the committed tree: 3 faces, `Solid::try_new` passes, the
wall reads `orientation=false` with effective normal `[-1, 0, 0]` at
(3, 2, 1) for a disk at (2, 2) r=1 - into the material.

## Decisions already made

### 1. The discriminator is the cycle index

`ArrRegion::boundaries` is documented (truck-geometry/src/arrange.rs):
index 0 is the outer boundary (CCW), indices ≥ 1 are holes (CW). The
side-face loop already binds `ci` in
`for (ci, cycle) in material.boundaries.iter().enumerate()`. The Circle
arm currently ignores it. The fix keys on it:

- **`ci == 0` (the circle is the region's OUTER boundary - the disk /
  solid cylinder):** `wire_bot = Wire::from(vec![be.clone()])`,
  `wire_top = Wire::from(vec![te.inverse()])`, and the face is stored
  UNINVERTED (`orientation == true`, the cylinder's natural +r̂ normal
  is already outward).
- **`ci > 0` (a hole):** exactly today's form -
  `wire_bot = [be.inverse()]`, `wire_top = [te]`, face inverted. Do not
  touch it; the plate-with-hole tests are the regression gate.

This exact recipe is machine-verified in
`scratch/rwdiskprobe/src/bin/fixprobe.rs`: with the caps built to the
same S2 conventions (bottom cap stored as the traced cycle and inverted;
top cap stored as traced, not inverted), the three-face solid passes
`Solid::try_new` and all three effective normals point outward (bottom
−ẑ, top +ẑ, wall +r̂). Re-run that probe's construction in the test if
you want an independent witness; do not delete the scratch.

### 2. Do not touch anything else

The caps, the line side faces, the vertex/edge identity rules, and the
`Solid::try_new` validation are all correct and tested. The fix is the
Circle arm's two wire constructions plus the conditional invert - nothing
else in the function changes.

## Tests required

1. `extrude_disk_wall_normal_points_outward` (new): build the disk
   profile - one `Curve::Circle` at (2, 2) r=1, exactly the
   `plate_with_hole` helper's circle, but ALONE (copy the helper's
   construction; do not call `plate_with_hole`). `arrange` it, extrude
   height 2.0, and assert:
   - the outcome is `Ok` and the solid has 3 faces: one `Cylinder` face
     with `orientation() == true` and two `Plane` faces (bottom
     `orientation() == false`, top `orientation() == true`);
   - the cylinder wall's effective normal at (3, 2, 1) is +x̂ within
     `TOLERANCE` (the natural radial normal, unflipped) - the outward
     direction, away from the material at r < 1;
   - `point_in_solid` (the existing helper) is true at (2, 2, 1) and
     false at (5, 5, 1);
   - `Solid::try_new(vec![shell])` succeeds on the returned solid's
     boundary (redundant with the internal validation, but it pins the
     contract).
   Preserve the names and assertions of all five existing tests
   unchanged - they are the hole-path regression gate.

## House form (H-3)

This crate is under the kernel's house rules. Any ADDED line with a bare
`1e-N` float literal must end `// H-3`. Prefer named constants or the
`TOLERANCE` already imported in this file. Run
`bash scripts/kernel-gates.sh <your base commit>` before writing
RESULT.json - a failing gate is a finding to report, never one to work
around.

## Done when

```console
cargo fmt --check -p truck-modeling
cargo clippy -p truck-modeling --all-targets --no-deps
cargo check --locked -p truck-modeling --all-targets
cargo test -p truck-modeling --lib extrude --no-fail-fast
bash scripts/kernel-gates.sh <your base commit>
```

Never run bare `cargo test` or a workspace-wide cargo command.

**Commit your work on the current branch** (subject
`modeling: orient the extruded circle wall by cycle role (BG-SOL-S2-DISK-ORIENT)`)
**before** writing `RESULT.json`: the verifier measures the committed
diff, and an uncommitted tree reads as an interrupted run.

## Forbidden

Editing outside `write_allow`; changing the hole (`ci > 0`) construction;
touching the caps, line side faces, vertex identity, or validation;
renaming or deleting a pre-existing test; adding `#[ignore]`; loosening a
gate; changing the GATE-4 ceiling.

## Stop conditions

- anchor mismatch -> `ANCHOR_MISMATCH` with observed count;
- the fixed recipe fails `Solid::try_new` or the outward-normal asserts
  -> `SPEC_GAP` with the failing assertion and your derivation (the
  recipe was verified in scratch; a failure means a convention this
  packet got wrong, and the derivation is the evidence);
- three consecutive cargo failures with one cause -> `BLOCKED`.

Finish by writing `RESULT.json` in the worktree root, not `loop/results/`.
Record in `notes` the wall's `orientation()` before and after your change
and the effective-normal value you asserted.
