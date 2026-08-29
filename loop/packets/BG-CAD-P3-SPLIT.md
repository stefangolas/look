---
id: BG-CAD-P3-SPLIT
class: design
crates: [truck-shapeops]
write_allow:
  - vendor/truck/truck-shapeops/src/section.rs
  - vendor/truck/truck-shapeops/src/lib.rs
  - vendor/truck/truck-shapeops/tests/split_plane.rs
tests_required:
  - split_flagship_plate_through_middle
  - split_recombination_is_original
  - split_box_diagonal_plane
  - section_faces_of_plate
  - section_face_with_hole_annulus
  - plane_missing_returns_whole_plus_empty
  - oblique_cylinder_section_refuses
  - sphere_face_refuses_noncanonical
  - split_signs_follow_normal
  - halves_survive_further_boolean
budget: {turns: 45, ctx_tokens: 130000}
---

# BG-CAD-P3-SPLIT — Phase 7 section + split by plane (via the landed Boolean)

Program: `docs/BUILD123D_COVERAGE_PLAN.md` P3 (Tier 0). Everything below is
pre-decided; churn, don't design. Contradiction with the tree = `SPEC_GAP`.

## Problem

build123d's `split(brightness by plane)` and `section()` are the next Tier 0
operations. The parsimony identity of the coverage plan says:
`split(S, Pi) = Contact + classify + caps + rewrite` — but the ENTIRE right
side is already landed as the certified 3-D Boolean. This packet therefore
implements split as **two `boolean()` calls against constructed halfspace
boxes**, and section as **cap-face extraction**. No new cutting, classifying,
or capping machinery is written; everything emitted rides the landed
material-state pipeline and its certificates.

## Anchors (measured 2026-08-28; re-derive before writing RESULT.json)

| id | file | pattern | count |
|----|------|---------|-------|
| A1 | vendor/truck/truck-shapeops/src/lib.rs | `pub mod` | 2 |
| A2 | vendor/truck/truck-shapeops/src/boolean/assemble.rs | `pub fn boolean` | 1 |
| A3 | vendor/truck/truck-shapeops/src/boolean/mod.rs | `pub enum BoolOp` | 1 |
| A4 | vendor/truck/truck-shapeops/src/lib.rs | `pub mod section` | 0 |

A1 becomes 3 once you declare `pub mod section;` (expected divergence, not a
mismatch). A4 is the dispatch-time state.

## Decisions already made for you

**D1 — module shape.** New file
`vendor/truck/truck-shapeops/src/section.rs`, declared `pub mod section;` in
lib.rs (place so `cargo fmt --check -p truck-shapeops` passes). The module
opens with the house deny header copied from
`truck-geometry/src/recognize.rs:22-29`. New test file
`vendor/truck/truck-shapeops/tests/split_plane.rs` with the same deny header
and a `fn expect_ok<T>(r: Outcome<T>) -> T` match-based helper.

**D2 — the solid over-box (local helper, do NOT import from truck-modeling).**

```rust
fn solid_over_box(solid: &Solid<Point3, Curve, Surface>) -> Outcome<((f64, f64), (f64, f64), (f64, f64))>
```

Per-face carrier table (same derivation as P1's D2 — reimplemented here
because truck-shapeops does not depend on truck-modeling, and adding that
edge would invert the layering):

- `Plane` face, `Cylinder` face → hull of the boundary edges' 3-D
  `EnclosureCurve::enclose` boxes over each edge's own range.
- `Sphere` face → the full carrier box `[c−r, c+r]³`.
- `Cone` face → hull of boundary edge boxes plus the apex.
- `Torus`, `CanonicalSurface::Placed`, `Unrecognized` →
  `UnsupportedEnvelope(NonCanonicalCarrier)`.
- Stored wires via `Face::absolute_boundaries()` (session-38 naming trap).

**D3 — halfspace boxes.** From `plane: &Plane` (truck-geometry; signed side
of a point `p` = the sign of `(p − origin) · norm`) and the over-box
`(xs, ys, zs)`:

- `minus_box` = the axis-aligned box covering the over-box EXTENDED by a pad
  on every side, clipped to nothing on the positive side of Pi: its wall
  lies exactly IN Pi (the wall's plane data is constructed from Pi's own
  origin/norm so cap identification later is exact-equality, not tolerance).
  Concretely: the box spans the over-box range on every axis, except the
  axis most aligned with `norm` is extended from the Pi crossing to the far
  negative side, PLUS `pad = 2 * (max over-box dimension)` beyond every
  face so no tangency with S is possible. For a Pi whose norm is not
  axis-aligned, the box is still axis-aligned and simply covers the whole
  over-box on the negative side of the CORNERS: compute the signed side of
  every over-box corner; the box spans the full over-box range but its
  inner wall is the Pi plane itself. Build the box as 8 `Vertex`es, 12
  `Line` edges (`Edge::new_unchecked` is NOT needed — no self-loops — but
  use whatever the landed construction recipes use), 6 `Plane` faces wired
  consistently outward, `Solid::try_new` as the gate.
- `plus_box` = the mirror construction on the positive side.
- Exact-arity caution: Pi's wall plane must be EXACTLY the plane you will
  match caps against (D5) — construct both from the same `Plane` value.

**D4 — the two operations.**

```rust
pub fn split_by_plane(
    solid: &Solid<Point3, Curve, Surface>, plane: &Plane, budget: &mut Budget,
) -> Outcome<(Solid, Solid)>;   // (plus, minus) per the plane's normal

pub fn section_faces(
    solid: &Solid<Point3, Curve, Surface>, plane: &Plane, budget: &mut Budget,
) -> Outcome<Vec<Face>>;
```

- `plus = boolean(solid, BoolOp::Difference, minus_box, budget)?`
- `minus = boolean(solid, BoolOp::Intersection, minus_box, budget)?`
  (`minus_box` covers the whole negative side, so Intersection = S ∩ minus
  side exactly.) Budget flows into both calls; report the composed spend.
- `section_faces` = run the split (or just the Difference half), then
  extract the faces whose surface `Plane` data equals the minus_box wall's
  plane data EXACTLY (C0 identity with your own construction — no tolerance,
  no recognize needed beyond the exact match). Return them as the section.
- The plane NOT touching the solid is a NORMAL result: plus ≅ S, minus is
  the empty solid (zero shells) — the landed assembler's all-discarded rule.
  `section_faces` on a missed plane refuses `Refusal::Empty` (no section
  exists).

**D5 — v1 envelope (typed refusals, no new arms).**

- Non-canonical S faces → `NonCanonicalCarrier` (D2, at the over-box lift —
  before any Boolean is paid for).
- An oblique plane × z-aligned cylinder wall yields an `Ellipse` FF locus —
  the landed splitter's RW-CONIC refusal fires INSIDE `boolean()`. Let it:
  do not pre-screen, do not catch. The typed refusal the caller sees is the
  landed one; document that this is the RW-CONIC boundary and that a
  `Curve` enum ellipse arm is the booked follow-up.
- Coplanar S face on Pi: NOT special-cased — the landed material-state
  machinery owns coincident cells (the M2 battery proves the butt-join
  cells). Whatever the landed entry answers is the answer; do not work
  around it.
- Unrecognized profile carriers inside boolean (spline faces) refuse
  upstream at its lift — again, let the landed refusal surface.

**D6 — certificates.** No new certificate machinery: the landed boolean's
`Solid::try_new` acceptance gate IS the certificate for both operations.
`section_faces` additionally certifies the cap extraction by exact plane
identity (a lookup, not a solve).

## Template

- `vendor/truck/truck-shapeops/src/boolean/assemble.rs` — the `boolean()`
  entry you compose (A2) and the single-shell guards it enforces.
- `vendor/truck/truck-shapeops/src/boolean/mod.rs` — `BoolOp` (A3) and the
  material-state vocabulary.
- `vendor/truck/truck-modeling/src/extrude.rs` — the house pattern for
  building a Solid from vertices/edges/faces with `Solid::try_new` as the
  gate (read for the construction recipe; do NOT edit it, and NOTE it is a
  different crate — your box construction is your own code in section.rs).

## Tests required (new file `tests/split_plane.rs`, dyadic witnesses only)

1. `split_flagship_plate_through_middle` — the extruded 4×4×2 plate,
   plane z = 1 (norm +z): both halves valid (`Solid::try_new` via the
   returned values being constructed solids), boxes `[0,4]²×[0,1]` and
   `[0,4]²×[1,2]` exactly.
2. `split_recombination_is_original` — `boolean(plus, Union, minus)` is
   face-count- and box-equal to the original S (the booked metamorphic
   split₊ ∪ split₋ ≅ S).
3. `split_box_diagonal_plane` — 2×2×2 box, plane through opposite edges
   (x + y = 2, norm (1,1,0)/√2), built from three exact dyadic points
   (compare planes by data, not unit length): **asserts the typed refusal
   the landed pipeline answers** (machine-check the arm you actually got;
   the recorded class across three instrumented stops is
   `UnsupportedEnvelope(ContactReductionDeferred)` /
   `NumericallyUnresolved(UncertifiedContainment)` depending on how far
   the chain runs). This is the booked **vertex-touch cut boundary** — the
   same envelope class as test 7's RW-CONIC refusal (docs/BUILD123D_
   COVERAGE_PLAN.md deferred list, session 41): a cut through the solid's
   edge graph requires four kernel decisions (canonical-vertex splicing,
   seam-edge replacement, per-face arc certification, Region2
   coplanar-adjacent) booked as the follow-up family. Assert the refusal
   verbatim with the boundary comment; do not stop on it (this amendment
   r2 re-books the test from happy-path to boundary).

## Amendment r2 (session 41, after three instrumented stops)

Tests 1, 2, 4-10 are PROVEN working by two prior P3 runs (axial family:
split, section, annulus section, missed plane, sign flip, downstream
booleans — all green; test 2's recombination keeps the pre-decided
10-face/box-equal assertion per the RW-RESEW evidence). Test 3 is
re-booked as a boundary assertion per the deferred-list entry (see its
text above). The packet's stop condition 3 ("the landed boolean refuses
one of the booked happy-path cases") now applies ONLY to tests 1, 2 and
4-6: a refusal there is a stop; test 3's and test 7's refusals are the
booked boundaries and must be asserted, not stopped on.
4. `section_faces_of_plate` — plate cut at z = 1: exactly 1 section face,
   its box `[0,4]²` exactly, its surface plane data exactly the wall's.
5. `section_face_with_hole_annulus` — the flagship plate-WITH-HOLE cut at
   z = 1: 1 section face with 2 boundary wires (the annulus).
6. `plane_missing_returns_whole_plus_empty` — plane beyond the solid:
   plus ≅ S (face/box equality), minus empty; and `section_faces` refuses
   `Empty`.
7. `oblique_cylinder_section_refuses` — extruded-circle solid, oblique
   plane → a typed refusal (machine-check which arm you actually got and
   record it; the expected family is the RW-CONIC boundary via the landed
   splitter, but whatever the landed pipeline answers is the assertion).
8. `sphere_face_refuses_noncanonical` — a solid carrying a sphere face
   (construct minimally) → `UnsupportedEnvelope(NonCanonicalCarrier)` at
   the over-box lift, with NO boolean call spent (assert the budget is
   untouched).
9. `split_signs_follow_normal` — same plate, norm −z: plus/minus swap
   relative to test 1 (box equality decides).
10. `halves_survive_further_boolean` — each half is downstream-consumable:
    `boolean(plus, Difference, small_box)` and the minus-side analogue both
    succeed and are valid solids.

## H-3 (house rule; V4 is a text gate on your diff)

No ADDED line carries a bare absolute small literal (`1e-N`) without the
same-line `// H-3` opt-out, and there should be none: dyadic constants,
geometry-derived pads (D3), and named consts only. Run
`& "C:\Program Files\Git\bin\bash.exe" scripts/kernel-gates.sh HEAD` before
writing RESULT.json (bare `bash` is the WSL stub).

## Done when

Commit on the current branch (subject
`BG-CAD-P3-SPLIT: section + split by plane via the landed boolean`) BEFORE
writing RESULT.json, then, all green:

```
cargo check --locked -p truck-shapeops
cargo fmt --check -p truck-shapeops
cargo test --locked -p truck-shapeops --lib
cargo test --locked -p truck-shapeops --test split_plane
cargo clippy --locked -p truck-shapeops --all-targets
& "C:\Program Files\Git\bin\bash.exe" scripts/kernel-gates.sh HEAD
```

## Forbidden

- Do not edit `boolean/**`, `extrude.rs`, `arrange.rs`, `recognize.rs`, or
  anything outside `write_allow`.
- Do not add `Refusal`/`EnvelopeCase`/`UnresolvedWitness`/`Collapse` arms
  (zero-new-arms program rule; a perceived need is a SPEC_GAP).
- Do not pre-screen the oblique-cylinder case (D5) — the landed RW-CONIC
  refusal is the booked boundary.
- GATE-3/H-4: `Face::debug_new` is banned in added lines.
- Do not write the tolerance-migration constructor name in prose or comments
  (GATE-4 counts raw text).

## Stop conditions

- `ANCHOR_MISMATCH` — report the measured count, change nothing.
- `SPEC_GAP` — a decision contradicts the tree; QUESTION.md with the
  empirical proof. (Likely candidates: the contact-free boolean input, the
  coplanar-face behavior, the box construction recipe under the debug-build
  self-loop trap — none of those apply to a 12-edge box, but verify.)
- The landed `boolean()` refuses one of the booked happy-path cases
  (tests 1–5) — stop and report the refusal verbatim; that is a finding
  about the landed pipeline, not something to route around.

RESULT.json: `{"id":"BG-CAD-P3-SPLIT","status":"DONE","contracts":[...],
"tests_added":10,"deviations":[...],"notes":"..."}` — every deviation with
your derivation; deviations are expected to be RIGHT.
