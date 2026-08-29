---
id: BG-CAD-P6-REWRITE
class: design
crates: [truck-shapeops]
write_allow:
  - vendor/truck/truck-shapeops/src/rewrite.rs
  - vendor/truck/truck-shapeops/src/lib.rs
  - vendor/truck/truck-shapeops/tests/chamfer.rs
tests_required:
  - chamfer_symmetric_box
  - chamfer_asymmetric_box
  - chamfer_two_independent_edges
  - chamfer_same_face_pair
  - trim_distance_certificate
  - chamfer_nonplane_refuses
  - chamfer_trim_overflow_refuses
  - chamfer_distance_angle
  - chamfer_result_survives_boolean
budget: {turns: 50, ctx_tokens: 150000}
---

# BG-CAD-P6-REWRITE — LocalBoundaryRewrite engine, proven on plane-plane chamfer

Program: `docs/BUILD123D_COVERAGE_PLAN.md` P6 (Tier 0). The pre-dispatch
num3-scratch probe is DONE and PASSED (this packet carries its evidence;
the probe source lives at `scratch/chamfer_probe/src/main.rs` in the
ORCHESTRATOR's checkout — untracked, so its recipe is QUOTED here, see D3).
Everything below is pre-decided; churn, don't design. Contradiction with
the tree = `SPEC_GAP`.

## Problem

build123d's `chamfer` is the next Tier 0 operation, and the plan books the
LocalBoundaryRewrite engine underneath it (P7's fillet rides the same
engine). The decomposition: `chamfer = closed-form trim-loci replacement +
rewrite`. The probe machine-validated the rewrite TARGET STATE: a
chamfered box assembles as a closed valid solid with exact plane data and
dyadic-exact invariants, using only the landed construction primitives.

## Anchors (measured 2026-08-29 at HEAD `1d001e4`; re-derive before writing RESULT.json)

| id | file | pattern | count |
|----|------|---------|-------|
| A1 | vendor/truck/truck-shapeops/src/lib.rs | `pub mod` | 3 |
| A2 | vendor/truck/truck-shapeops/src/lib.rs | `pub mod rewrite` | 0 |
| A3 | vendor/truck/truck-shapeops/src/boolean/assemble.rs | `pub fn boolean\(` | 1 |
| A4 | vendor/truck/truck-modeling/src/primitive.rs | `pub fn cuboid` | 1 |

A1 becomes 4 once you declare `pub mod rewrite;` (expected divergence, not
a mismatch). A2 is the dispatch-time state.

## The probe evidence (quoted — the worker's worktree has no scratch/)

The probe built the chamfered box [0,4]²×[0,2] DIRECTLY as a prism over a
chamfered polygon and `Solid::try_new` accepted EVERY witness first try:

- **P1 symmetric** d=1 at the vertical edge (4,4): 7 faces, 15 unique
  edges, 10 unique vertices; the chamfer face's plane is x+y=7 (outward
  normal (1,1)); bbox exactly [0,4]²×[0,2].
- **P2 asymmetric** d_in=1 (x=4 face), d_out=0.5 (y=4 face): trim points
  (4,3) and (3.5,4); chamfer plane normal ∝ (2,1), offset 2x+y=11.
- **P3 two independent edges** ((4,4) and (0,0), both d=1): 8 faces,
  18 edges, 12 vertices; planes x+y=7 and x+y=1.
- **P4 same-face pair** ((4,0) and (4,4), both d=1): the shared x=4 face
  trims to y∈[1,3]; 8 faces, 18 edges, 12 vertices; planes x−y=3 and
  x+y=7.

The probe's construction recipe (the engine's building blocks):

1. The box boundary walked CCW (viewed from +z); a chamfered corner emits
   TWO trim points: `p1 = corner − arrive_dir·d_in` and
   `p2 = corner + leave_dir·d_out`, where the leaving direction is the
   arriving direction rotated +90° (for the CCW square walk).
2. EVERY boundary segment (normal and chamfer alike) yields a side face
   by the cuboid side pattern: wire = [bottom forward, up at the segment's
   end, top inverse, down at the segment's start]; plane =
   `Plane::new(bottom_start, bottom_end, top_start)`. For a CCW segment
   (dx,dy) with v=+z, that plane's normal (dx,dy,0)×(0,0,1) = (dy,−dx,0)
   points OUTWARD — the chamfer plane data falls out of the construction
   exactly.
3. Top face: CCW traversal, plane u=+x v=+y → normal +z. Bottom face:
   reverse-CCW traversal via edge INVERSES, plane u=+y v=+x → normal −z.
4. All edges built ONCE as shared instances; `Solid::try_new` is the gate.
5. Counting semantics: `Shell::edge_iter`/`vertex_iter` yield PER-USE
   references (P1 counts 30 edge uses for 15 unique edges) — count unique
   ids when asserting.

## Decisions already made for you

**D1 — module shape.** New file
`vendor/truck/truck-shapeops/src/rewrite.rs`, declared `pub mod rewrite;`
in lib.rs (A1 3→4). House deny header copied from
`truck-geometry/src/recognize.rs:22-29`. New test file
`vendor/truck/truck-shapeops/tests/chamfer.rs` with the same deny header
and the `tests/interior_loop.rs`-style `expect_ok` helper.

**D2 — signatures and the face-ordering contract.**

```rust
/// One chamfered vertical... one chamfered straight edge: the edge is
/// named by its two endpoint positions; d_first applies to the face
/// whose outward normal is lexicographically SMALLER (x, then y, then z),
/// d_second to the other.
#[derive(Clone, Copy, Debug)]
pub struct ChamferSpec {
    pub a: Point3,
    pub b: Point3,
    pub d_first: f64,
    pub d_second: f64,
}

pub fn chamfer(
    solid: &Solid<Point3, Curve, Surface>,
    specs: &[ChamferSpec],
    budget: &mut Budget,
) -> Outcome<Solid<Point3, Curve, Surface>>;
```

The engine resolves each spec's edge from the solid's topology: the unique
edge whose endpoints match `a` and `b` (within the insertion tolerance, by
point proximity — `a`/`b` may be given in either order). Zero or multiple
matches refuse `Refusal::Empty` / `UnsupportedEnvelope(NonCanonicalCarrier)`
respectively (machine-check which arm fits the landed conventions; record
the choice). P2's asymmetric fixture is the ordering contract's witness:
the y=4 face's normal (0,1,0) is lexicographically smaller than (1,0,0),
so `d_first` = the y=4-face trim (0.5) and `d_second` = the x=4-face trim
(1.0) reproduces the probe's P2 mesh.

**D3 — the engine algorithm (pre-decided; the probe recipe generalized).**

1. **Lift.** Every face of the solid must be a canonical `Plane` carrier
   whose boundary is a single wire of `Line` edges forming a CONVEX
   polygon (machine-check convexity + orientation from the stored wire,
   which the landed invariant keeps CCW-positive in the surface frame).
   Multi-wire faces, non-Line edges, non-Plane carriers, non-convex
   polygons → `UnsupportedEnvelope(NonCanonicalCarrier)` at the lift,
   before any construction.
2. **Per-spec neighborhood.** The spec's edge has exactly two adjacent
   faces (machine-check from the shell; anything else refuses). On each
   face, the trim line = the edge's line offset IN the face's plane by
   the face's distance toward the polygon interior (the offset direction
   is the one whose trim points land strictly inside the face's polygon —
   machine-check the sign rule from the face's outward normal and the
   edge direction). Trim points = the trim line's intersections with the
   face's two boundary edges ADJACENT to the spec edge in that face's
   wire (2-D line∩line in the face's plane frame; closed-form).
3. **Collision check.** Per face, the kept region must be non-empty and
   the trims must not overlap: the trim points must appear along the
   face's boundary in wire order without crossing (P4 exercises two
   trims on one face). A trim point that exits a boundary edge's extent
   (d ≥ the adjacent edge's length), an empty/inverted kept region, or
   overlapping trims refuse `Refusal::Empty`.
4. **Rebuild (the probe recipe, generalized).** The new shell = every
   original face's polygon with its chamfered corner(s) replaced by trim
   point(s) (unchanged faces ride with their wires and edge instances
   UNTOUCHED) + one chamfer side face per spec, built by the cuboid side
   pattern (bottom forward, up, top inverse, down; `Plane::new(bottom_
   start, bottom_end, top_start)` — the chamfer plane data falls out
   exactly, the probe's check). Edges surviving from the original solid
   are REUSED as instances; every new edge is minted once and shared by
   its two faces. `Solid::try_new` is the acceptance gate.
5. **Orientation.** The stored-wire-CCW-in-surface-frame invariant (the
   landed rule; the probe's side pattern satisfies it) decides every
   wire direction — no flag flipping.

**D4 — the distance-angle form.** v1 books the right-dihedral closed form
(box-like solids): given `d` on the first face and the chamfer half-angle
`α` measured from that face's plane, the second trim is
`d_second = d · tan(α)` (cross-section: trim (d,0), chamfer line
y = −tan(α)(x−d), hits the second face at d·tan(α)) — machine-checked by
test 8, where α=45° makes d_second = d (the P1 mesh). The API carries it
as a convenience constructor or a second entry — YOUR choice, recorded in
RESULT notes (machine-check which shape fits the module cleanest); the
GENERAL-dihedral formula is a booked follow-up, do not attempt it.

**D5 — refusals (zero new arms).** `NonCanonicalCarrier` at the lift and
for ambiguous edge resolution; `Empty` for degenerate requests (trim
overflow, empty kept region, no matching edge); `NumericallyUnresolved`
remains available for failed projections. No new `Refusal`/`EnvelopeCase`
/`UnresolvedWitness`/`Collapse` arms — a perceived need is a SPEC_GAP.

**D6 — certificates.** `Solid::try_new` is the acceptance gate. Tests
additionally certify: each trim point is at EXACT distance d from the
original edge's line (|cross(p−a, b−a)|/|b−a| = d — machine-check), each
chamfer plane passes through both its trim points (the probe's offset
check), and the bbox is EXACT for convex-corner chamfers (the probe
invariant).

## Template

- `vendor/truck/truck-modeling/src/primitive.rs:140-196` — the cuboid
  construction recipe your engine's rebuild mirrors (read; do NOT edit).
- `vendor/truck/truck-shapeops/src/section.rs` — the P3 module shape
  (deny header, helper layout, budget plumbing).
- `vendor/truck/truck-shapeops/tests/cut_boundaries.rs` — the test-file
  conventions; do not edit.
- The probe recipe is QUOTED above (D3/probe evidence section) — it is
  machine-validated; do not redesign it.

## Tests required (new file `tests/chamfer.rs`, dyadic witnesses only)

Fixtures build boxes via `truck_modeling::primitive::cuboid` (A4). The
box [0,4]²×[0,2]; the probe's vertical edge at (4,4) is the primary
witness.

1. `chamfer_symmetric_box` — the P1 witness through the ENGINE: 7 faces,
   15 unique edges, 10 unique vertices, the chamfer plane x+y=7 (normal
   sign-agnostic, offset exact), bbox exactly [0,4]²×[0,2].
2. `chamfer_asymmetric_box` — the P2 witness: the D2 ordering contract
   pins d_first=0.5 (y=4 face) / d_second=1.0 (x=4 face); chamfer plane
   through (4,3) and (3.5,4) with normal ∝ (2,1), offset 11.
3. `chamfer_two_independent_edges` — the P3 witness: 8 faces, 18 edges,
   12 vertices, planes x+y=7 and x+y=1.
4. `chamfer_same_face_pair` — the P4 witness: the shared face trims to
   y∈[1,3]; planes x−y=3 and x+y=7.
5. `trim_distance_certificate` — for the P1 result, every trim point's
   distance to the original edge's line equals 1 exactly (D6).
6. `chamfer_nonplane_refuses` — a cylinder-carrying solid (extruded
   circle profile, the boolean tests' fixture style) →
   `UnsupportedEnvelope(NonCanonicalCarrier)` at the lift with the budget
   untouched.
7. `chamfer_trim_overflow_refuses` — d=4 on the probe fixture (the trim
   reaches the opposite boundary edge) → `Refusal::Empty` (machine-check
   the arm you actually got; record it).
8. `chamfer_distance_angle` — D4's form: d=1, α=45° on the box = the P1
   mesh (face count + the x+y=7 plane data).
9. `chamfer_result_survives_boolean` — the P1 result downstream-consumes:
   `boolean(chamfered, BoolOp::Difference, small_box)` assembles a valid
   solid (the `tests/resew.rs` convention: the small box must CROSS the
   boundary — e.g. [1.5,2.5]²×[0.5,2.5] through the top).

## H-3 (house rule; V4 is a text gate on your diff)

No ADDED line carries a bare absolute small literal (`1e-N`) without the
same-line `// H-3` opt-out. Dyadic constants and geometry-derived values
only. Run `& "C:\Program Files\Git\bin\bash.exe" scripts/kernel-gates.sh
HEAD` before writing RESULT.json (bare `bash` is the WSL stub). CLIPPY
EVERY CHANGED FILE — run `cargo clippy --locked -p truck-shapeops
--all-targets` UNFILTERED and fix all findings BEFORE committing (three
prior packets each lost verify rounds to partial clippy runs).

## Done when

Commit on the current branch (subject
`BG-CAD-P6-REWRITE: LocalBoundaryRewrite engine, proven on PP chamfer`)
BEFORE writing RESULT.json AT THE WORKTREE ROOT (that exact path — not
`loop/results/`), then, all green:

```
cargo check --locked -p truck-shapeops
cargo fmt --check -p truck-shapeops
cargo test --locked -p truck-shapeops --lib
cargo test --locked -p truck-shapeops --test chamfer
cargo test --locked -p truck-shapeops --test boolean_m2
cargo test --locked -p truck-shapeops --test interior_loop
cargo test --locked -p truck-shapeops --test resew
cargo test --locked -p truck-shapeops --test cut_boundaries
cargo test --locked -p truck-shapeops --test split_plane
cargo clippy --locked -p truck-shapeops --all-targets
& "C:\Program Files\Git\bin\bash.exe" scripts/kernel-gates.sh HEAD
```

All landed suites (`boolean_m2`, `interior_loop`, `resew`,
`cut_boundaries`, `split_plane`) must pass UNCHANGED. The lib suite's
`healing::tests::step_import` failure is the recorded environmental one
(fails at base too).

## Forbidden

- Do not edit `boolean/**`, `section.rs`, `Cargo.toml`, or anything
  outside `write_allow`.
- Do not add `Refusal`/`EnvelopeCase`/`UnresolvedWitness`/`Collapse` arms
  (D5).
- Do not attempt the general-dihedral distance-angle form, non-convex
  faces, curved edges, or multi-wire faces (D3/D4 boundaries; booked
  follow-ups).
- GATE-3/H-4: `Face::debug_new` is banned in added lines.
- Do not write the tolerance-migration constructor name in prose or comments
  (GATE-4 counts raw text, comments included).
- No instrumentation traces may survive in the committed diff.

## Stop conditions

- `ANCHOR_MISMATCH` — report the measured count, change nothing.
- `SPEC_GAP` — a decision contradicts the tree; QUESTION.md with the
  empirical proof.
- A booked happy-path case (tests 1-5, 8-9) fails `Solid::try_new` after
  a D3-faithful construction — stop and report the closure witness
  verbatim.
- The topology edge lookup (D2) cannot resolve a spec edge uniquely on
  the landed Solid structure — stop and report what the walk actually
  yields.

RESULT.json: `{"id":"BG-CAD-P6-REWRITE","status":"DONE","contracts":[...],
"tests_added":9,"deviations":[...],"notes":"..."}` — every deviation with
your derivation; deviations are expected to be RIGHT.
