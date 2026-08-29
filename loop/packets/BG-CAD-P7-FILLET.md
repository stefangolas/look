---
id: BG-CAD-P7-FILLET
class: design
crates: [truck-shapeops]
write_allow:
  - vendor/truck/truck-shapeops/src/rewrite.rs
  - vendor/truck/truck-shapeops/tests/fillet.rs
tests_required:
  - fillet_symmetric_box
  - fillet_two_independent_edges
  - fillet_same_face_pair
  - fillet_three_edge_corner_sphere
  - tangent_distance_certificate
  - fillet_nonplane_refuses
  - fillet_radius_overflow_refuses
  - fillet_result_survives_boolean
budget: {turns: 50, ctx_tokens: 150000}
---

# BG-CAD-P7-FILLET — F1 plane-plane fillet + F4 three-plane corner sphere

Program: `docs/BUILD123D_COVERAGE_PLAN.md` P7 (Tier 0): realization table
6.4 rows (center locus Line → Cylinder; three-plane corner → Sphere) riding
the LANDED P6 rewrite engine (`rewrite.rs`). The pre-dispatch F1 probe is
DONE and PASSED (its recipe is quoted in D3; the probe source is
untracked scratch — everything load-bearing is in this packet).
Everything below is pre-decided; churn, don't design. Contradiction with
the tree = `SPEC_GAP`.

## Problem

build123d's `fillet` (constant radius) on plane-plane edges. The
parsimony identity: `fillet = offset + Contact + realization + rewrite`.
For two planes meeting at a convex edge, the rolling-ball contact loci are
the two tangent lines (the edge's line offset by r IN each face's plane —
the SAME loci the chamfer trim computes), and the realized face is a
canonical z-axis `Cylinder` (table 6.4 row 1). Where THREE filleted edges
meet at a corner, the corner region is a `Sphere` patch (table 6.4 row 2,
the F4 three-plane corner: the triple offset intersection).

## Anchors (measured 2026-08-29 at HEAD `5d8ff06`; re-derive before writing RESULT.json)

| id | file | pattern | count |
|----|------|---------|-------|
| A1 | vendor/truck/truck-shapeops/src/rewrite.rs | `pub fn chamfer\(` | 1 |
| A2 | vendor/truck/truck-shapeops/src/rewrite.rs | `pub struct ChamferSpec` | 1 |
| A3 | vendor/truck/truck-shapeops/src/lib.rs | `pub mod rewrite` | 1 |
| A4 | vendor/truck/truck-geometry/src/specifieds/cylinder.rs | `pub fn new\(center: Point3, radius: f64\)` | 1 |

All anchors are the PRE-packet tree. This packet adds NO new module: the
fillet lives IN `rewrite.rs` beside the chamfer (same engine, same
declared module — A3 never diverges).

## The F1 probe evidence (quoted — the worker's worktree has no scratch/)

The filleted box [0,4]²×[0,2] (radius 1 at the vertical edge (4,4)) built
DIRECTLY and `Solid::try_new` accepted FIRST TRY: 7 faces, 15 unique
edges, 10 unique vertices, bbox exactly [0,4]²×[0,2].

- The fillet face is the quarter cylinder: canonical `Cylinder`
  carrier (`Cylinder::new(center, radius)`, center (3,3,0), r=1;
  `subs(u,v) = center + r·(cos u, sin u, 0) + (0,0,v)` — z-axis-aligned,
  u = angle, v = height), spanning u ∈ [0, π/2], v ∈ [0, 2]. Its wire:
  bottom arc forward (u 0→π/2 at v=0), up at u=π/2, top arc inverse,
  down at u=0 — the cuboid side pattern in the (u,v) frame; the surface
  normal (uder×vder at u=0: +x) is OUTWARD.
- The cap faces (z=0, z=2) carry the quarter-ARC edge (the landed
  revolve arc recipe: `Curve::Circle(Processor::with_transform(
  TrimmedCurve::new(UnitCircle::<Point3>::new(), (0.0, angle)),
  Matrix4{ x: (r,0,0,0), y: (0,r,0,0), z: (0,0,1,0), w: (cx,cy,z,1) }))`)
  — the arc from (4,3,0) to (3,4,0) about (3,3,0), used FORWARD in the
  top face's CCW traversal and INVERSE in the bottom face's
  reverse-CCW traversal.
- The two trimmed side faces (x=4 spanning y∈[0,3]; y=4 spanning
  x∈[0,3]) and the two unchanged faces ride exactly as the chamfer P1
  target state (the tangent lines ARE the chamfer trim lines at d=r).

## Decisions already made for you

**D1 — the entry mirrors the chamfer.**

```rust
/// One filleted straight edge; the radius is the single rolling-ball
/// radius applied to BOTH adjacent faces (fillets are symmetric —
/// distance-distance is the chamfer's business).
#[derive(Clone, Copy, Debug)]
pub struct FilletSpec {
    pub a: Point3,
    pub b: Point3,
    pub radius: f64,
}

pub fn fillet(
    solid: &Solid<Point3, Curve, Surface>,
    specs: &[FilletSpec],
    budget: &mut Budget,
) -> Outcome<Solid<Point3, Curve, Surface>>;
```

Edge resolution identical to the chamfer's (A2's contract: unique edge
by endpoint proximity; 0 matches → `Empty`, >1 → `NonCanonicalCarrier`).

**D2 — F1 per-edge construction (the probe recipe generalized).** The
lift, trim-line computation, collision checks, shared-instance pools, and
orientation invariants are the CHAMFER engine's (D3 of BG-CAD-P6-REWRITE)
— refactor/reuse within rewrite.rs as the module layout wants (the
chamfer's trim machinery and the fillet's are the same computation; do
not duplicate it — factor the shared trim walk once, machine-check the
refactor against the landed chamfer tests which MUST stay green
unchanged). The delta: the consumed corner strip is replaced by the
CYLINDER face, not a plane strip:

- The cylinder carrier: axis parallel to the spec edge (for the v1
  right-dihedral box fixtures, z-aligned — the canonical form), center =
  the edge's start point inset by r along BOTH faces' outward... machine-
  check: the center is the point at distance r from BOTH face planes ON
  the material side, projected onto the edge's normal plane at the edge
  start; radius r (A4's constructor; the carrier refuses r ≤ 0).
- The cylinder face's boundary: bottom arc (tangent point to tangent
  point, the quarter arc about the axis, the revolve arc recipe rotated
  into the edge's normal plane), top arc, and the two straight edges at
  the tangent points (shared instances with the trimmed faces).
- The trimmed faces' new boundary edges at the tangent lines are the
  chamfer trim edges (same computation, same instances).

**D3 — F4: the three-plane corner sphere.** When the spec set covers all
three edges meeting at a solid corner (machine-check: the three edges
pairwise share endpoints at the corner, all convex), the corner region is
the sphere patch: center = the corner inset by r along all three face
normals (for the box corner (4,4,2) with r=1: center (3,3,1), radius r);
each of the three cylinders is trimmed where the sphere takes over (the
junction is the sphere∩cylinder circle — a plane circle of radius r at
the junction height, e.g. the vertical cylinder's top arc at z=1);
the three PLANAR faces trim at their tangent lines to the corner-adjacent
tangent points (the top face of the tri-filleted box is the hexagon with
the corner at the sphere's pole (3,3,2) — the sphere touches the top
plane at exactly one point, the parameter-frame pole, which is a REGULAR
wire vertex); the sphere patch face = `Surface::Sphere` with the three
junction quarter-arcs as its single closed wire.

MACHINE-CHECK MANDATE (the one genuinely open derivation): the Sphere
carrier's parameter frame and the patch wire's orientation are NOT
pre-decided — read `truck-geometry/src/specifieds/sphere.rs`'s
`subs`/ders, derive the octant patch's (u,v) rectangle (its v=0 boundary
degenerates to the pole vertex — a regular wire vertex, not a self-loop),
determine the wire direction that makes `Solid::try_new` accept, and
record the derived convention in RESULT notes. The three junction arcs
are plane-circle arcs constructible by the revolve arc recipe rotated
into each junction plane (the machine-check: each arc lies ON the sphere
(subs-distance = r) and ON its cylinder). If the Sphere carrier's frame
cannot host the patch without a new arm, STOP — SPEC_GAP with the
carrier's subs derivation as evidence.

**D4 — refusals (zero new arms).** As the chamfer: `NonCanonicalCarrier`
at the lift (non-Plane faces, non-Line edges, non-convex polygons,
ambiguous resolution) and `Empty` for degenerate requests (radius ≥ the
adjacent extent — the tangent point exits the boundary edge; duplicate
specs; empty spec list). A radius that forces non-cylindrical
realizations (curved edges, non-right dihedrals with by_angle) is out of
envelope and refuses at the lift.

**D5 — the distance-angle form is NOT booked here** (the chamfer carries
it; fillets are symmetric). A `FilletSpec` with differing per-face radii
is a chamfer-and-fillet blend — booked follow-up, refuse `Empty` cannot
arise (the spec has one radius) — nothing to do.

**D6 — certificates.** `Solid::try_new` is the gate. Tests additionally
certify: each tangent point at EXACT distance r from the spec edge's
line; the cylinder face's radius/axis exact (the probe check); the bbox
exact; the corner sphere's center at the exact triple-offset point and
radius r.

## Template

- `vendor/truck/truck-shapeops/src/rewrite.rs` — the chamfer engine
  (A1/A2): the lift, trim walk, pools, orientation; the fillet reuses it.
- `vendor/truck/truck-geometry/src/specifieds/cylinder.rs` and
  `sphere.rs` — the carrier conventions (A4; the D3 machine-check).
- `vendor/truck/truck-modeling/src/revolve.rs:695-731` — the landed arc
  recipe (read; do NOT edit).
- `vendor/truck/truck-shapeops/tests/chamfer.rs` — the fixture style; do
  not edit.

## Tests required (new file `tests/fillet.rs`, dyadic witnesses only)

Fixtures via `truck_modeling::primitive::cuboid`; the box [0,4]²×[0,2];
the primary witness edge at (4,4), r=1.

1. `fillet_symmetric_box` — the F1 probe witness through the ENGINE:
   7 faces, 15 unique edges, 10 unique vertices, the cylinder face
   (radius 1, axis at (3,3)), the cap arcs, bbox exact.
2. `fillet_two_independent_edges` — the verticals (4,4) and (0,0), r=1:
   8 faces, 18 edges, 12 vertices, cylinders at (3,3) and (1,1).
3. `fillet_same_face_pair` — the verticals (4,0) and (4,4), r=1: the
   shared x=4 face trims to y∈[1,3]; two cylinder faces.
4. `fillet_three_edge_corner_sphere` — F4: the three edges at the corner
   (4,4,2) filleted r=1: the solid assembles; 10 faces (6 planes + 3
   cylinders + 1 sphere); the sphere face exists with center (3,3,1),
   radius 1; the top face is the hexagon with the corner at (3,3) (the
   pole); the machine-checked sphere-frame convention is recorded.
5. `tangent_distance_certificate` — for test 1, every tangent point at
   exactly distance 1 from the original edge's line.
6. `fillet_nonplane_refuses` — a cylinder-carrying solid →
   `UnsupportedEnvelope(NonCanonicalCarrier)` at the lift, budget
   untouched.
7. `fillet_radius_overflow_refuses` — r=4 on the probe fixture →
   `Refusal::Empty` (machine-check the arm; record it).
8. `fillet_result_survives_boolean` — the test-1 result through
   `boolean(Difference, small_box)` with a boundary-crossing box (the
   `tests/resew.rs` convention).

## H-3 (house rule; V4 is a text gate on your diff)

No ADDED line carries a bare absolute small literal (`1e-N`) without the
same-line `// H-3` opt-out. Run `& "C:\Program Files\Git\bin\bash.exe"
scripts/kernel-gates.sh HEAD` before writing RESULT.json (bare `bash` is
the WSL stub). CLIPPY EVERY CHANGED FILE — full unfiltered
`cargo clippy --locked -p truck-shapeops --all-targets` before committing.

## Done when

Commit on the current branch (subject
`BG-CAD-P7-FILLET: F1 PP fillet + F4 corner sphere on the rewrite engine`)
BEFORE writing RESULT.json AT THE WORKTREE ROOT (that exact path — not
`loop/results/`), then, all green:

```
cargo check --locked -p truck-shapeops
cargo fmt --check -p truck-shapeops
cargo test --locked -p truck-shapeops --lib
cargo test --locked -p truck-shapeops --test fillet
cargo test --locked -p truck-shapeops --test chamfer
cargo test --locked -p truck-shapeops --test boolean_m2
cargo test --locked -p truck-shapeops --test interior_loop
cargo test --locked -p truck-shapeops --test resew
cargo test --locked -p truck-shapeops --test cut_boundaries
cargo test --locked -p truck-shapeops --test split_plane
cargo clippy --locked -p truck-shapeops --all-targets
& "C:\Program Files\Git\bin\bash.exe" scripts/kernel-gates.sh HEAD
```

All landed suites must pass UNCHANGED. The lib suite's
`healing::tests::step_import` failure is the recorded environmental one.

## Forbidden

- Do not edit `boolean/**`, `section.rs`, `Cargo.toml`, or anything
  outside `write_allow`.
- Do not add `Refusal`/`EnvelopeCase`/`UnresolvedWitness`/`Collapse` arms
  (D4).
- Do not attempt general-dihedral fillets, curved-edge fillets,
  variable-radius fillets, or non-sphere corner realizations (D3/D4
  boundaries; booked follow-ups).
- GATE-3/H-4: `Face::debug_new` is banned in added lines.
- Do not write the tolerance-migration constructor name in prose or comments
  (GATE-4 counts raw text, comments included).
- No instrumentation traces may survive in the committed diff.

## Stop conditions

- `ANCHOR_MISMATCH` — report the measured count, change nothing.
- `SPEC_GAP` — a decision contradicts the tree; QUESTION.md with the
  empirical proof.
- The Sphere carrier's frame cannot host the F4 patch without a new arm
  (D3) — stop with the subs derivation as evidence.
- A booked happy-path case fails `Solid::try_new` after a D2/D3-faithful
  construction — stop and report the closure witness verbatim.

RESULT.json: `{"id":"BG-CAD-P7-FILLET","status":"DONE","contracts":[...],
"tests_added":8,"deviations":[...],"notes":"..."}` — the D3 sphere-frame
derivation goes in notes; every deviation with your derivation; deviations
are expected to be RIGHT.
