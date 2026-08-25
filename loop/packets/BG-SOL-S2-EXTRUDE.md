# WORK PACKET BG-SOL-S2-EXTRUDE — direct B-rep extrude of a planar arrangement

You are implementing S2 of the solver family: `extrude_profile`, the certified
direct extrusion of an `Arrangement` (S1) into a closed `Solid` B-rep with NO
tool-body Boolean — the side faces of a plate-with-hole profile become n planar
side faces plus one cylindrical hole wall, and the canonical surfaces are
produced directly. This is the second half of M1 (certified planar
construction, docs/SOLVER_FAMILY_PLAN.md §4 Phase 2 + §7). Everything you need
is in this document. **Do not read any other spec file** — this packet is
self-contained.

```json
{"id":"BG-SOL-S2-EXTRUDE","status":"DONE","contracts":["BG-SOL-S2-EXTRUDE"],
 "tests_added":4,"deviations":[],"disagreements":[],
 "baseline_failures":[],"notes":"free text"}
```

Fill `deviations`, `disagreements` and `baseline_failures` as arrays, empty if
empty. **If anything below contradicts what you find in the code, say so in
`disagreements` rather than making the code match the packet.**

```yaml
id:          BG-SOL-S2-EXTRUDE
class:       design
crates:      [truck-modeling, truck-geometry]
write_allow:
  - vendor/truck/truck-modeling/src/extrude.rs
read_allow:
  - vendor/truck/truck-geometry/src/arrange.rs
  - vendor/truck/truck-geometry/src/recognize.rs
  - vendor/truck/truck-topology/src/face.rs
  - vendor/truck/truck-topology/src/wire.rs
tests_required:
  - extrude_plate_with_hole_is_a_closed_solid
  - extrude_plate_hole_wall_is_a_cylinder
  - extrude_face_and_edge_counts_are_exact
  - extrude_zero_or_negative_height_is_refused
budget:      {turns: 90, ctx_tokens: 220000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c '^pub struct Arrangement' vendor/truck/truck-geometry/src/arrange.rs"}
  - {id: A2, expect: 1, cmd: "grep -c '^pub fn arrange' vendor/truck/truck-geometry/src/arrange.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'pub boundaries' vendor/truck/truck-geometry/src/arrange.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'pub fn recognize_surface' vendor/truck/truck-geometry/src/recognize.rs"}
  - {id: A5, expect: 1, cmd: "grep -c 'pub mod extrude' vendor/truck/truck-modeling/src/lib.rs"}
```

## Problem

M1 constructs a plate with a cylindrical hole by 2-D means: `rectangle − circle`
→ arrangement → profile with hole → **direct extrude** → valid B-rep. S1 (the
`arrange` module, BG-SOL-S1-ARRANGE) produces the certified 2-D subdivision;
S2 turns the material region of that subdivision into a `Solid<Point3, Curve,
Surface>` by building the boundary faces combinatorially: the bottom and top
caps (each with the hole's wire as an inner boundary), the outer planar side
faces, and the single cylindrical side face of the hole. No 3-D Boolean runs
anywhere. The B-rep must be **closed and valid** (`Solid::try_new`), and the
hole's side surface must be the **canonical** `Cylinder` (the recognizer
verifies it), so that a later failed Boolean is provably about contact, not
about assembly (plan §2).

## Design decisions already made for you

### 1. Module and scaffolding

The module is NOT yet in the tree. This packet adds `pub mod extrude;` to
`truck-modeling/src/lib.rs` (place it with the other module declarations; run
`cargo fmt -p truck-modeling` after) AND creates
`vendor/truck/truck-modeling/src/extrude.rs` with the H-1 deny header:

```rust
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]
```

`truck-modeling` depends on `truck-base` (the evidence algebra), `truck-geometry`
(`Curve`/`Surface`/`arrange`/`recognize`) and `truck-topology` (the B-rep). The
`Outcome`/`Refusal` types come from `truck_base::evidence`.

### 2. The signature

```rust
/// Extrudes the material region(s) of a planar arrangement by `height` along
/// +z into a closed solid. v1 scope: exactly ONE material region, and the
/// profile's loops are oriented with material on the left (M1 passes the
/// outer rectangle CCW and the hole circle REVERSED, so the plate's winding is
/// +1 and the hole interior's is 0).
pub fn extrude_profile(profile: &Arrangement, height: f64)
    -> Outcome<Solid<Point3, Curve, Surface>>;
```

`Solid<Point3, Curve, Surface>` is `truck_topology::Solid` with the canonical
`Curve`/`Surface` enums. Re-derive the exact `Arrangement`/`ArrRegion`/
`ArrVertex`/`ArrHalfEdge` field spellings from `arrange.rs` with grep before
coding (the S1 packet's target spellings are authoritative only if they
landed verbatim — record any difference in `disagreements`).

### 3. Material selection

The material regions are the `ArrRegion`s with **`winding == 1`** (positive
winding = material on the left, the standard convention). S1's landed winding
is computed over each region's OWN boundary cycles (see the S1 RESULT notes):
for the M1 profile with the hole circle REVERSED (CW), the plate winds +1
(inside the rect, outside the reversed circle) and the hole interior winds −1
(inside a CW loop) — so `winding == 1` selects exactly the plate and excludes
the hole. v1 accepts exactly one material region (`Err(Refusal::Empty)`
otherwise; a multi-region profile is v2). If a bounded region's winding is
0 or −1 it is NOT material.

Note the S1 v1 convention: open (non-closed) profile walks are modeled as
separate unbounded regions, which is geometrically a single connected exterior
for a finite crossing — M1 profiles are closed loops, so this does not affect
M1; the S2 packet operates only on closed-loop arrangements.

### 4. Vertex identity — the load-bearing rule

The solid's topology is built from SHARED `Vertex` instances. Build a map from
arrangement vertex → `Vertex<Point3>` ONCE (the bottom layer), and reuse those
instances in every face: the bottom face's rect corner vertex IS the same
instance the adjacent side face's bottom corner uses, IS the same instance the
top face's translated... (no — the top layer is a NEW set of vertices
translated by `height`). Two vertices that coincide geometrically but are
distinct instances produce an OPEN shell (the CE-003-MIGRATE instance-identity
trap). The construction:

- Bottom vertices: one `Vertex::new(point)` per arrangement vertex of the
  material region's boundary cycles (z = 0).
- Top vertices: one `Vertex::new(point + height·ẑ)` per bottom vertex.
- The vertical seam of a closed boundary edge (the circle's seam) has ONE
  bottom vertex and ONE top vertex; the closed edge runs bottom→bottom and its
  top copy top→top.

### 5. Faces

**Bottom cap** (surface `Surface::Plane(Plane::new(o, u, v))` with o = (0,0,0),
u = (1,0,0), v = (0,1,0)): one face whose `boundaries` wires are the material
region's boundary cycles in order — the outer cycle (rectangle) as the first
wire, the hole cycle (circle) as the second. Wire orientation: the cycles as
the arrangement traced them (the S1 DCEL yields the correct CCW/CW pairing for
a bounded face with material on the left).

**Top cap** (surface `Surface::Plane` translated to z = height): the SAME
cycles translated, with each cycle's edge directions REVERSED (a wire is
oriented by the face it bounds; the top cap's outward normal is +z, so its
wires run opposite the bottom cap's).

**Side faces**: one per boundary edge of the material region. For an edge
whose curve is a `Curve::Line` (the rectangle's edges), the side face is the
quad `[bottom edge, up, top edge reversed, down]` on the surface
`Surface::Plane(Plane::new(a, b, a + height·ẑ))` where `a→b` is the bottom
edge — this is EXACTLY the recognizer's `ExtrudedCurve(Line)→Plane` mapping,
constructed directly. For the `Curve::Circle` edge (the hole), ONE side face
with boundary `[circle edge, vertical seam up, top circle edge reversed,
vertical seam down]` on the surface `Surface::Cylinder(Cylinder::new(center,
radius))` — the canonical cylinder, constructed directly (the circle satisfies
the z-cylinder conditions because the profile is in z = 0; if `Cylinder::new`
refuses, the input was not a valid M1 profile → `Err(Refusal::Empty)`).

**The closed hole edge**: the circle edge's front and back vertices are the
SAME bottom vertex. `Edge::try_new` refuses `SameVertex`; use
`Edge::new_unchecked(front, back, curve)` for the circle edge (and its top
copy), with a comment recording that the closed edge is the seam and the
`new_unchecked` is the sanctioned construction for it (the BG-TOL-001-MESHALGO
precedent). The vertical seam edges are ordinary `Edge::new`/`try_new` lines.

Every edge's curve is a `Curve` built from the arrangement edge's geometry:
`Curve::Line(Line(a, b))` for line pieces, the `Curve::Circle` processor for
the circle piece, `Curve::Line` for the vertical seams. The side faces share
their boundary edges with the caps (instance identity, rule 4).

### 6. Assembly and validation

Collect the 7 faces (1 bottom + 1 top + n_outer + 1 hole) into a
`Shell::new` and the shell into `Solid::try_new(boundaries)`. **The solid MUST
pass `Solid::try_new`** — closed, connected, no singular vertices. If it
refuses, the topology is wrong (a missing shared vertex, a reversed wire, a
missing face) — debug the construction, never weaken the validation. v1 sets
`PC = ()` (no pcurves); the pcurve layer is a documented later refinement (the
plan lists pcurves; the M1 gate is the closed valid solid with the canonical
cylinder).

### 7. Canonicalization — verified, not constructed twice

The side surfaces are built directly as `Plane`/`Cylinder` (section 5), so no
post-hoc surface swap is needed. The tests verify the relationship by calling
`recognize_surface` on each side surface and asserting it returns the same
canonical carrier (the recognizer's `ExtrudedCurve(Circle)→Cylinder` rule is
exactly the construction used). This is the plan's "canonicalization: recognize
(circle × straight path) => Cylinder" exercised as a test, not a second code
path.

## H-3, the house rule that rejects bare float literals

GATE-2 of `scripts/kernel-gates.sh` fails any **added** line carrying a bare
`1e-N`-shaped literal unless that line ends with an `// H-3` comment. This
packet's code compares closure and positions; `height` is an input, vertices
are dyadic — no small literals. If a comparison needs a slack, use the
same-line form:

```rust
const EPS: f64 = 1.0e-9; // H-3: <why this slack, dimensionally>
```

Run `bash scripts/kernel-gates.sh <your base commit>` yourself before writing
`RESULT.json`.

## GATE-4 / `unscaled_legacy` (the ratchet)

This packet adds NO `unscaled_legacy()` calls. Do not touch
`scripts/unscaled_legacy_ceiling.txt` — the orchestrator owns the ratchet.

## Regression tests (exact names)

Put the tests in a `#[cfg(test)] mod tests` inside `extrude.rs` with
`#[allow(clippy::unwrap_used, clippy::expect_used)]`. The M1 profile helper:

```rust
fn plate_with_hole() -> Arrangement {
    // rectangle 4x4 CCW, circle r=1 at (2,2) REVERSED (hole)
    // build via `arrange` (S1) on the reversed profile
}
```

Reversing the circle: `Curve::Circle` processors are `Invertible` — build the
profile with the circle `invert()`ed so its material side is the outside.

1. `extrude_plate_with_hole_is_a_closed_solid` — `extrude_profile` on the M1
   plate (height 2.0) → `Ok`; the solid passes `Solid::try_new` (it was built
   through it); the shell is closed; a point in the plate material `(1,1,1)`
   is inside the solid and a point in the hole's air column `(2,2,1)` (the hole
   runs through the whole height) is NOT inside the solid — assert both, using
   `Solid::contains_point` or a point-in-solid test if one exists, else a
   closest-point-on-boundary distance comparison against a named tolerance.
2. `extrude_plate_hole_wall_is_a_cylinder` — find the face whose surface is a
   `Surface::Cylinder`; assert its center is `(2,2,0)` and radius 1.0, and
   `recognize_surface(&that_surface)` returns the same `Cylinder` carrier.
3. `extrude_face_and_edge_counts_are_exact` — assert 7 faces (1 bottom + 1 top
   + 4 rect sides + 1 cylinder), and that the bottom/top faces each have 2
   boundary wires (the outer rectangle wire with 4 edges and the inner circle
   wire with 1 edge).
4. `extrude_zero_or_negative_height_is_refused` — `height = 0.0` and
   `height = -1.0` → `Err`, never a panic.

Every other existing truck-modeling test must stay green.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-modeling
cargo clippy -p truck-modeling --all-targets --no-deps
cargo test -p truck-modeling --lib --tests --no-fail-fast
cargo check --locked -p truck-modeling --all-targets
bash scripts/kernel-gates.sh <your base commit>
```

Never run a bare `cargo test`. Never run `cargo check --workspace` — it
exhausts disk on a shared machine with concurrent workers.

## Forbidden

Editing any file outside `write_allow`. Building the solid with distinct
`Vertex` instances for coincident geometric points (the shell will be open —
the identity rule is the contract). Weakening `Solid::try_new` or any closure
validation to make a broken shell pass. Running a 3-D Boolean anywhere in this
packet. Running `cargo check --workspace` / `cargo build --workspace` (disk).
Adding `#[ignore]`. Changing the GATE-4 ceiling.

## Stop conditions

- an anchor count differs → `ANCHOR_MISMATCH`, naming the file and what you saw
- a pre-existing test you did not expect to touch is broken → do NOT weaken the
  gate; report it in `disagreements` with the failing test name and the exact
  reason
- three consecutive failed `cargo` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it. In `notes`, record the
face count you observed (must be 7), whether `Edge::new_unchecked` was required
for the closed circle edge, and the exact `Arrangement` field spellings you
read off the landed S1 module (any deviation from this packet's target
spellings).

Commit on the current branch with subject
`feat(modeling): direct certified extrude of a planar arrangement (BG-SOL-S2-EXTRUDE)`.
