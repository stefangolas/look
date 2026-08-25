# WORK PACKET BG-SOL-S2-ORIENT — normalize outward face orientation of the extruded plate

You are fixing a known defect in the landed S2 extrude (`truck-modeling/src/
extrude.rs`, BG-SOL-S2-EXTRUDE, `extrude_profile`). The solid it builds is
combinatorially Closed (`Solid::try_new` passes) but **two of its seven faces
are geometrically inward**: the bottom cap's effective normal points INTO the
material, and the cylinder wall's effective normal points into the plate rather
than into the hole. This is the recorded M1-completion item (STATE.md session-28
close, item 1a). The fix must be landed BEFORE the Phase-4 material-state
Boolean consumes the solid, or M2's inside/outside classification will be wrong.

Everything you need is in this document. **Do not read any other spec file** —
this packet is self-contained.

```json
{"id":"BG-SOL-S2-ORIENT","status":"DONE","contracts":["BG-SOL-S2-ORIENT"],
 "tests_added":1,"deviations":[],"disagreements":[],
 "baseline_failures":[],"notes":"free text"}
```

Fill `deviations`, `disagreements` and `baseline_failures` as arrays, empty if
empty. **If anything below contradicts what you find in the code, say so in
`disagreements` rather than making the code match the packet.**

```yaml
id:          BG-SOL-S2-ORIENT
class:       design
crates:      [truck-modeling, truck-topology]
write_allow:
  - vendor/truck/truck-modeling/src/extrude.rs
read_allow:
  - vendor/truck/truck-modeling/src/multi_sweep.rs
  - vendor/truck/truck-topology/src/face.rs
  - vendor/truck/truck-topology/src/shell.rs
  - vendor/truck/truck-topology/src/wire.rs
tests_required:
  - extrude_all_face_normals_point_outward
budget:      {turns: 90, ctx_tokens: 220000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c '^pub fn extrude_profile' vendor/truck/truck-modeling/src/extrude.rs"}
  - {id: A2, expect: 1, cmd: "grep -c '^fn select_material' vendor/truck/truck-modeling/src/extrude.rs"}
  - {id: A3, expect: 4, cmd: "grep -c 'Face::try_new' vendor/truck/truck-modeling/src/extrude.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'fn point_in_solid' vendor/truck/truck-modeling/src/extrude.rs"}
  - {id: A5, expect: 1, cmd: "grep -c 'self.inverse()' vendor/truck/truck-modeling/src/multi_sweep.rs"}
  - {id: A6, expect: 1, cmd: "grep -c 'pub fn invert' vendor/truck/truck-topology/src/face.rs"}
```

## Problem

`extrude_profile` turns the M1 plate (a 4×4 rectangle with a radius-1 hole at
(2,2), z ∈ [0, height]) into a closed `Solid<Point3, Curve, Surface>`: one
bottom cap, one top cap, four planar rect side faces, and one cylindrical hole
wall. The shell passes `Solid::try_new` (Closed / connected / no singular
vertices), but the **effective geometric normals of two faces point into the
material**:

1. **Bottom cap.** Its surface is `Plane::new((0,0,0),(1,0,0),(0,1,0))` whose
   natural normal is `+z`. The face's `orientation` flag is `true` (the default
   from `Face::try_new`), so the face's effective normal is `+z`. But the
   material occupies z ∈ [0, height], so the outward normal of the solid at the
   bottom cap is `−z`. The cap's `+z` normal points INTO the material.
2. **Cylinder wall.** The hole wall's surface is
   `Cylinder::new(center=(2,2,0), radius=1)`. Its parametric normal is radial
   (`+r`, away from the axis). The material is the plate OUTSIDE the circle; the
   hole is air. The outward normal of the solid at the hole wall therefore
   points INTO the hole (`−r`, toward the axis). The face's effective `+r`
   normal points into the plate — into the material.

The top cap (normal `+z`, material below — correct), the four rect side faces
(normals pointing away from the plate — correct), and the seam/edge pairing are
all fine. Only the bottom cap and the cylinder wall are wrong.

**Why this matters.** Phase 4 (the material-state Boolean) classifies points as
inside/outside by walking the shell and using each face's effective normal
(`face.orientation() ? surface_normal : -surface_normal`). A face whose normal
points into the material flips the sign of every crossing through it, so any
inside/outside computation that crosses the bottom cap or the hole wall is
wrong. The fix is a construction change in `extrude_profile`, not a post-hoc
flip: the boundary wire directions and the face `orientation` flags must be
re-derived so that every face's effective normal points outward AND the shell
stays Closed.

## Design decisions already made for you

### 1. The convention to follow

The sanctioned precedent is truck's own `multi_sweep` prism
(`truck-modeling/src/multi_sweep.rs:113`):

```rust
let mut shell = Shell::from(vec![self.inverse()]);
```

The bottom cap of a swept solid is stored as the **inverted** seed face
(orientation flag `false`); the top cap is the non-inverted seed; the side faces
are built to connect the two consistently. So "the bottom cap face is inverted"
is the truck-native way to get an outward `−z` bottom normal, and the whole
shell must be re-derived coherently — you cannot flip one face in isolation (see
section 3).

### 2. The target face-by-face

Effective normal = `face.orientation() ? surface.normal : -surface.normal`.
After the fix every face must satisfy:

| Face | Surface | Surface normal | Desired outward | orientation flag |
|---|---|---|---|---|
| bottom cap | `Plane::new((0,0,0),(1,0,0),(0,1,0))` | `+z` | `−z` | **false** |
| top cap | `Plane::new((0,0,height),(1,0,height),(0,1,height))` | `+z` | `+z` | true |
| 4 rect sides | `Plane::new(a, b, a + height·ẑ)` | outward (away from plate) | outward | true |
| cylinder wall | `Cylinder::new(center, radius)` | `+r` (away from axis) | `−r` (into hole) | **false** |

So the two faces whose `orientation` flag becomes `false` are exactly the two
defective ones. The top cap and the four side faces keep `orientation = true`.

### 3. Why the wire directions must change too (the trap)

`Face::invert()` flips the `orientation` flag, and the shell's `Closed` check
(`Shell::shell_condition`, via `face.edge_iter()`) reads edges through the flag:
an inverted face reports all its boundary edges in the opposite orientation.
Every boundary edge of a closed shell must appear in exactly two faces with
opposite effective orientations. If you flip ONLY the bottom cap's flag, its
bottom rect edges would be reported the same way the side faces report them —
the shell becomes `Regular`, not `Closed`, and `Solid::try_new` refuses.

Therefore the re-orientation is a **coordinated change** to all four
construction sites in `extrude_profile`:

1. **Bottom cap** (currently ~lines 119-128): keep the stored boundary wires
   exactly as they are now (the material region's cycles, as the arrangement
   traced them), but invert the face so `orientation == false`. Concretely:
   `Face::try_new(bottom_wires, bottom_surface)` returns `Face`; call
   `.invert()` on it (mutating) before pushing it. Its effective normal becomes
   `−z` and its effective boundary edges flip, which the other changes pair
   against.
2. **Top cap** (currently ~lines 135-146): **stop reversing the top wires.**
   Today the code builds each top wire then calls `wire.invert()`. Remove that
   call: the top cap's stored wires become the same direction as the bottom
   cap's stored wires (the arrangement's traced direction), and the face keeps
   `orientation == true`. Its effective normal stays `+z` (outward), and its
   effective boundary edges are now the traced direction.
3. **Side faces** (currently ~lines 194-200): re-derive the quad wire so it
   pairs against the flipped bottom cap and the un-reversed top cap. Today the
   quad is `[be.inverse(), seam_o, te.clone(), seam_n.inverse()]`. The corrected
   quad is `[be.clone(), seam_n, te.inverse(), seam_o.inverse()]` (same four
   edge instances, new direction and order), still on the same side-plane
   surface and `orientation == true`. Do not change the edge instances or the
   surface; only the wire.
4. **Cylinder wall** (currently ~lines 207-223): keep the stored boundary wires
   exactly as they are now (`wire_bot = [be.inverse()]`, `wire_top = [te]`), but
   invert the face so `orientation == false`. Its effective normal becomes `−r`
   (into the hole) and its effective boundary edges flip, which the changed cap
   wires pair against.

Re-verify the pairing after your change with the rule: every edge id appears in
exactly two faces with opposite effective orientations (this is what
`ShellCondition::Closed` checks). The `edge_iter()` on a face respects the
`orientation` flag (via `BoundaryIter`), so reasoning about "effective" means
applying the flag.

### 4. What must NOT change

- The M1 profile and `plate_with_hole()` test helper (same rectangle + circle).
- The face count (7), the surface types (6 planes + 1 cylinder), and the cap
  wire counts (bottom/top caps each have 2 boundary wires: a 4-edge outer rect
  wire and a 1-edge circle wire; the cylinder has 2 circle self-loops).
- The `Edge::new_unchecked` construction of the closed circle self-loop edges
  (bottom and top) — do not replace it with `Edge::try_new`.
- The shared `Vertex` instance rule (one bottom + one top vertex per boundary
  vertex, reused across faces). Do not weaken any validation.
- `select_material`, the containment-based material rule, and the
  `winding == 1` logic. Do not touch them.

### 5. The regression test (exact name)

Add `extrude_all_face_normals_point_outward` to the existing `#[cfg(test)] mod
tests` inside `extrude.rs` (the module already carries
`#[allow(clippy::unwrap_used, clippy::expect_used)]`). The test:

- builds the M1 solid with `extrude_profile(&profile, &arrangement, 2.0)`;
- iterates `solid.face_iter()`, and for each face dispatches on `face.surface()`
  to pick an interior sample point `q` **strictly inside the face's domain** and
  the expected outward direction;
- computes the effective normal `n_eff = if face.orientation() { surface_normal_at(&surface, q) } else { -surface_normal_at(&surface, q) }`;
- asserts `n_eff` points outward, and — the load-bearing check — that stepping a
  small `EPS` along `-n_eff` from `q` is INSIDE the solid and along `+n_eff` is
  OUTSIDE, using the existing `point_in_solid` helper in the test module.

Concrete sample points and directions for the M1 plate (height = 2.0, center of
hole = (2,2)):

- bottom cap (plane, `z == 0`): q = (1, 1, 0). Expect `n_eff ≈ −z`; `q − EPS·n_eff`
  = (1,1,+) inside the plate, `q + EPS·n_eff` = (1,1,−) outside.
- top cap (plane, `z == 2.0`): q = (1, 1, 2.0). Expect `n_eff ≈ +z`; `q − EPS·n_eff`
  inside, `q + EPS·n_eff` outside.
- side face at x == 0: q = (0, 1, 1.0). Expect `n_eff ≈ −x`; `q − EPS·n_eff` =
  (+,1,1) inside the plate, `q + EPS·n_eff` = (−,1,1) outside.
- side face at x == 4: q = (4, 1, 1.0). Expect `n_eff ≈ +x`.
- side face at y == 0: q = (1, 0, 1.0). Expect `n_eff ≈ −y`.
- side face at y == 4: q = (1, 4, 1.0). Expect `n_eff ≈ +y`.
- cylinder wall: q = (3, 2, 1.0) (on the hole wall, +x side of the axis).
  Expect `n_eff` pointing toward the axis, i.e. `n_eff ≈ −x`; `q − EPS·n_eff` =
  (3+EPS, 2, 1) is inside the plate, `q + EPS·n_eff` = (3−EPS, 2, 1) is in the
  hole → outside the solid.

Identify faces by their surface and origin (e.g. a plane whose `origin()` has
`z == 0.0`, `z == 2.0`, or `x == 0.0 / 4.0`, `y == 0.0 / 4.0`). Use the existing
test helpers `point_in_solid`, `surface_normal_at` and `sample_params` — they
already handle Plane and Cylinder.

Define the step constant with the H-3 same-line opt-out (GATE-2 rejects bare
`1e-N` literals on added lines):

```rust
const EPS: f64 = 1.0e-3; // H-3: step from each face into/out of the material in the regression test
```

This test MUST fail on the pre-fix construction (bottom cap `+z`, cylinder `+r`)
and pass after the fix. Do not weaken it: both the normal direction assertion
and the in/out step assertion are required.

### 6. Done-when gates

```
cargo fmt --check -p truck-modeling
cargo clippy -p truck-modeling --all-targets --no-deps
cargo test -p truck-modeling --lib --tests --no-fail-fast
cargo check --locked -p truck-modeling --all-targets
bash scripts/kernel-gates.sh <your base commit>
```

Every existing truck-modeling test must stay green, in particular the four S2
tests `extrude_plate_with_hole_is_a_closed_solid`,
`extrude_plate_hole_wall_is_a_cylinder`, `extrude_face_and_edge_counts_are_exact`
and `extrude_zero_or_negative_height_is_refused`. Never run a bare `cargo test`.
Never run `cargo check --workspace` — it exhausts disk on a shared machine.

## H-3, the house rule that rejects bare float literals

GATE-2 of `scripts/kernel-gates.sh` fails any **added** line carrying a bare
`1e-N`-shaped literal unless that line ends with an `// H-3` comment. The only
added float literal in this packet is the test's `EPS`, spelled with the
same-line `// H-3` comment as shown above. If any other comparison needs a slack,
use the same same-line form. Run `bash scripts/kernel-gates.sh <your base
commit>` yourself before writing `RESULT.json`.

## GATE-4 / `unscaled_legacy` (the ratchet)

This packet adds NO `unscaled_legacy()` calls. Do not touch
`scripts/unscaled_legacy_ceiling.txt` — the orchestrator owns the ratchet.

## Forbidden

Editing any file outside `write_allow`. Weakening `Solid::try_new`, the shell
condition, or any closure validation to make a broken shell pass. Changing the
M1 profile, the face count, the surface types, `select_material`, the material
rule, or the `Edge::new_unchecked` self-loop construction. Running a 3-D Boolean
anywhere in this packet. Running `cargo check --workspace` / `cargo build
--workspace` (disk). Adding `#[ignore]`. Changing the GATE-4 ceiling.

## Stop conditions

- an anchor count differs → `ANCHOR_MISMATCH`, naming the file and what you saw
- a pre-existing test you did not expect to touch is broken → do NOT weaken the
  gate; report it in `disagreements` with the failing test name and the exact
  reason
- three consecutive failed `cargo` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it. In `notes`, record the
final face-by-face `orientation` flags you landed, confirm the shell is still
`Closed` under `Solid::try_new`, and note whether `Edge::new_unchecked` was still
required for the circle self-loops.

Commit on the current branch with subject
`fix(modeling): normalize outward face orientation of the extruded plate (BG-SOL-S2-ORIENT)`.
