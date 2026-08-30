---
id: BG-CAD-P8-FACADE
class: design
crates: [truck-shapeops]
write_allow:
  - vendor/truck/truck-shapeops/src/facade.rs
  - vendor/truck/truck-shapeops/src/lib.rs
  - vendor/truck/truck-shapeops/Cargo.toml
  - vendor/truck/truck-shapeops/Cargo.lock
  - vendor/truck/truck-shapeops/tests/conformance_battery.rs
tests_required:
  - facade_naming_table_covers_every_landed_entry
  - facade_constructive_sequence_plate_with_fillet_and_hole
  - facade_metamorphic_rows_still_hold_through_facade
  - facade_refusal_cases_are_typed
  - facade_boolean_modes_map_to_boolop
  - consumability_tessellation_closed_on_generated_carriers
  - consumability_tessellation_oblique_recorded_boundary
budget: {turns: 45, ctx_tokens: 140000}
---

# BG-CAD-P8-FACADE — the build123d-shaped facade + the conformance battery

Program: `docs/BUILD123D_COVERAGE_PLAN.md` P8 (Tier 0 finale): "the
build123d-shaped facade over P1–P7 plus the conformance battery". The
pre-dispatch num3-scratch probe on the battery's consumability row is DONE
and PASSED (its evidence is QUOTED here). Everything below is pre-decided;
churn, don't design. Contradiction with the tree = `SPEC_GAP`.

## Problem

The landed kernel entries (P1-P12) carry their research-shaped signatures;
the plan books a facade that is a **naming + semantics table over them,
carrying zero geometric content of its own** (plan §1): every operation
composes landed primitives or refuses with the typed `Refusal` — no new
solver mathematics, no restricted alternative names, no silent fallbacks.
P8 also folds every packet's metamorphic rows into ONE integration battery
and adds the downstream-consumability row (Boolean/section/tessellation on
generated surfaces).

## The probe evidence (quoted — the worker's worktree has no scratch/)

**Finding 1 — every generated carrier tessellates through the landed
`MeshableShape::triangulation(tol)` path** (tol 0.01, then
`put_together_same_attrs(TOLERANCE)`):

- box control: 192 positions, 180 faces, condition **Closed**;
- plain cylinder (disk extrude): 130 positions, 124 faces, **Closed**;
- **the P12 torus-fillet solid** (through the landed `fillet_circle`):
  281 positions, 366 faces, **Closed** — the Torus carrier is consumable;
- the mirrored (Placed-cylinder) solid: 130 positions, 124 faces,
  **Closed**;
- the boolean Difference output: 384 positions, 364 faces, **Closed**.

**Finding 2 — the oblique placed-affine wall solid (the P10 emission)
tessellates with condition Regular, not Closed** (130 positions, 124
faces): the sheared placement's mesh does not fully pair after the
position merge. This is the MEASURED boundary the battery books — the row
asserts non-empty + the measured `Regular` condition with a comment
citing this probe; closure for sheared placements is a booked follow-up.

**Finding 3 — tessellation of circle-carrying solids PANICS in debug
builds** ("Two same vertices cannot construct an edge", the recorded
self-loop constructor trap; the box control — line edges only — meshes
fine in debug). The battery's tessellation rows are therefore gated
`#[cfg(not(debug_assertions))]` with a comment citing this finding; the
refusal/metamorphic rows are debug-safe and ungated.

## Anchors (MEASURE AT DISPATCH — the values below were measured at the
pre-P10 root `51e343d`; re-derive every one by command immediately before
dispatch and correct the table; a mismatch at dispatch is a stop, so the
table MUST be the dispatch-time root's)

| id | file | pattern | count |
|----|------|---------|-------|
| A1 | vendor/truck/truck-shapeops/src/lib.rs | `pub mod` | 4 |
| A2 | vendor/truck/truck-shapeops/src/lib.rs | `pub mod facade` | 0 |
| A3 | vendor/truck/truck-shapeops/src/rewrite.rs | `pub fn fillet\(` | 1 |
| A4 | vendor/truck/truck-shapeops/src/rewrite.rs | `pub fn fillet_circle\(` | 1 |
| A5 | vendor/truck/truck-shapeops/src/rewrite.rs | `pub fn chamfer\(` | 1 |
| A6 | vendor/truck/truck-shapeops/src/boolean/assemble.rs | `pub fn boolean\(` | 1 |
| A7 | vendor/truck/truck-shapeops/tests | `conformance_battery.rs` | 0 (new file) |
| A8 | vendor/truck/truck-shapeops/src/facade.rs | `pub fn` | 0 (new file) |
| A9 | vendor/truck/truck-modeling/src/cad.rs | `pub fn rotate_solid\(` | 1 (post-P10; re-measure) |
| A10 | vendor/truck/truck-modeling/src/cad.rs | `pub fn mirror_about_plane\(` | 1 (post-P10; re-measure) |

A1 becomes 5 once you declare `pub mod facade;` (expected divergence).

## Decisions already made for you

**D1 — the facade lives at `truck-shapeops/src/facade.rs`** (A2): the
ONLY landed crate that reaches everything. `pub mod facade;` in
lib.rs (A1 4→5). The module header carries the house deny header (copy
from `rewrite.rs`) and the plan's §1 contract as its doc comment.

**D1a — AMENDMENT (session 42 r2, orchestrator, after the worker's
SPEC_GAP): promote `truck-modeling` to a normal dependency.** The r1
packet asserted "truck-shapeops depends on truck-modeling" without
checking the dependency KIND: `truck-modeling` is listed only under
`[dev-dependencies]`, so non-test `src/facade.rs` cannot reach it (the
worker's empirical proof: E0433 under the packet's own gate). The
amendment: in `truck-shapeops/Cargo.toml`, move the `truck-modeling`
line from `[dev-dependencies]` to `[dependencies]` (same version/path —
no cycle: truck-modeling does not depend on truck-shapeops). The
dependency-kind edge changes the lockfile's record, so run `cargo check
-p truck-shapeops` once WITHOUT `--locked` to refresh `Cargo.lock`,
commit BOTH the manifest and the updated lock, and confirm `cargo check
--locked -p truck-shapeops` is green afterwards. Record the manifest
change in RESULT deviations.

**D2 — the naming table (pre-decided; every entry is a one-line
composition or a typed refusal).**

```rust
pub enum Mode { Add, Subtract, Intersect }              // build123d workplane modes
pub enum BlendSpec { Straight(FilletSpec), Circular(CircleFilletSpec) }

pub fn extrude(profile: &[Curve], arrangement: &Certified<Arrangement>, height: f64) -> Outcome<Solid>;
pub fn extrude_vector(profile: &[Curve], arrangement: &Certified<Arrangement>, dir: Vector3, both: bool) -> Outcome<Solid>;
pub fn revolve(profile: &[Curve], arrangement: &Certified<Arrangement>, angle: f64) -> Outcome<Solid>;
pub fn fillet(solid: &Solid, specs: &[BlendSpec], budget: &mut Budget) -> Outcome<Solid>;
pub fn chamfer(solid: &Solid, specs: &[ChamferSpec], budget: &mut Budget) -> Outcome<Solid>;
pub fn mirror(solid: &Solid, plane: &Plane) -> Outcome<Solid>;
pub fn mirror_about_plane(solid: &Solid, plane_point: Point3, plane_normal: Vector3) -> Outcome<Solid>;
pub fn rotate(solid: &Solid, axis_point: Point3, axis_dir: Vector3, angle: f64) -> Outcome<Solid>;
pub fn scale(solid: &Solid, factor: f64) -> Outcome<Solid>;
pub fn translate(solid: &Solid, t: Vector3) -> Outcome<Solid>;
pub fn section(solid: &Solid, plane: &Plane, budget: &mut Budget) -> Outcome<...>;   // section_faces
pub fn split(solid: &Solid, plane: &Plane, budget: &mut Budget) -> Outcome<...>;     // split_by_plane
pub fn bounding_box(solid: &Solid) -> Outcome<BoundingBox<Point3>>;
pub fn boolean_op(a: &Solid, mode: Mode, b: &Solid, budget: &mut Budget) -> Outcome<Solid>;
pub fn make_face(profile: &[Curve]) -> Outcome<Vec<Face<Point3, Curve, Surface>>>;
pub fn make_hull(points: &[Point3]) -> Outcome<Face<Point3, Curve, Surface>>;
```

Machine-check each landed signature against the tree and copy the
SPELLING exactly (the §3 rule); where a landed signature differs from the
sketch above (argument order, wrapper types, the section/split return
shapes), the LANDED signature wins — the facade adapts to the tree, never
the reverse, and records the adaptation in RESULT notes. `Mode` maps
Add→`BoolOp::Union`, Subtract→`BoolOp::Difference`, Intersect→
`BoolOp::Intersection` (machine-check the landed variant spellings).
`fillet` dispatches the `BlendSpec` enum: every Straight through
`rewrite::fillet`, every Circular through `rewrite::fillet_circle`,
SEQUENTIAL per the P12 D4 rule (mixed spec lists process in order; an
empty list refuses `Empty` exactly as the landed entries do). The enum
wrapper is naming, not geometry (D-zero-content). Python selectors are
NOT part of the facade (booked with the pyo3 program — plan §1).

**D3 — the conformance battery (`tests/conformance_battery.rs`, new
file A7).** The plan's §9 metamorphic algebra re-asserted THROUGH the
facade names, the constructive sequences, the refusal cases, and the
consumability row:

1. **Constructive sequence** (the flagship): rectangle profile →
   `extrude` → `fillet` (two vertical edges, the grouped Straight batch
   per D2) → `boolean_op(Subtract, small boundary-crossing box)` —
   EVERY intermediate assembles (extrude 6 faces; fillet 8 faces;
   Subtract 13 faces — the worker's r2 measured census). **AMENDMENT
   (session 42 r3, orchestrator, after the worker's STOP 3): the
   sequence ENDS at the boolean.** The r1 split step is re-booked as two
   separate rows, both machine-measured by the r2 worker:
   - `split` of a PLAIN box by z=1 assembles (1+1 shells) — a positive
     split row;
   - `split` of a FILLET-CARRYING solid refuses
     `UnsupportedEnvelope(ContactReductionDeferred)` for every plane —
     the measured **split-of-arc-carrying-faces v1 boundary** (the
     RW-CONIC class; the landed boolean splitter's fragment machinery on
     Circle edges — the worker proved `contact()` itself succeeds on
     every face×plane and edge×plane pair, so the boundary sits in the
     splitter/classifier, not the contact sweep). Assert the typed arm;
     book the fix as a follow-up.
2. **Metamorphic rows through the facade**: `A ∪ B ≅ B ∪ A`,
   `A − A = ∅` refusal-or-empty (machine-check the landed behavior),
   `fillet round trip` (the P6/P7 row: offset the two adjacent faces
   back by r reconstructs the original neighborhood — re-assert via the
   landed suite's own witnesses at the facade level), the P9
   `contact(A,B) ≅ contact(g·A, g·B)` (re-assert one instance through
   the facade-rotated pair), the P10 `T(A op B) = T(A) op T(B)`
   (re-assert one instance through the facade names).
3. **Refusal cases** (one typed refusal per feature family, asserted
   through the facade): non-plane fillet lift, trim overflow, circular
   overflow, multi-wire cap, oblique `dz=0` extrude, non-z-parallel
   oblique on non-circle profiles, boolean multi-shell guard, split by a
   plane that grazes (the vertex-touch typed refusal — the booked v1
   boundary).
4. **Consumability** (Finding 1/2/3 + the r2 worker's exhaustive
   re-measurement): tessellation on the box, the plain cylinder, the P12
   torus-fillet solid, the mirrored placed cylinder, and the boolean
   output — each asserts the MEASURED condition on the dispatch tree
   (**Closed** for all of these; the r2 worker reproduced every one) plus
   non-emptiness; do NOT assert the probe's exact position/face counts
   (they are fixture- and tree-dependent — the r2 worker measured the
   boolean row at 290/280 vs the probe's quoted 384/364). GATED
   `#[cfg(not(debug_assertions))]` (Finding 3).
   **AMENDMENT (r3, after the worker's STOP 4): the oblique row asserts
   the dispatch-tree measurement — condition Closed** (the r2 worker
   exhaustively measured Closed for the landed `extrude_vector`
   emission: dir (1,0,1)/(1,1,1)/(0,1,1), both=true, rotated, mirrored,
   translated; raw triangulation Oriented, closing at every
   put_together tolerance tried). The probe's quoted Regular was
   measured on a HAND-BUILT construction on the PRE-P10 tree; the landed
   P10 emission assembles a cleaner shell. Cite both measurements in the
   test comment; the closure of sheared placements is simply recorded as
   measured, no boundary claimed.

**D4 — the facade is NOT allowed geometric content.** Every facade line
either calls a landed entry or refuses; ANY branch that computes geometry
(a tolerance decision, a frame construction, a sign flip) is a SPEC_GAP.
The D2 signature adaptations are naming; record each one.

**D5 — refusals (zero new arms).** The facade forwards the landed
refusals verbatim. No new `Refusal`/`EnvelopeCase`/`UnresolvedWitness`/
`Collapse` arms — a perceived need is a SPEC_GAP.

**D6 — certificates.** The landed entries' certificates forward
untouched; the battery asserts `Solid::try_new`-class validity and the
probe's measured mesh conditions, never new certificates.

## Template

- `vendor/truck/truck-shapeops/src/lib.rs` — the module list (A1/A2).
- `vendor/truck/truck-shapeops/src/rewrite.rs` — the fillet/chamfer/
  fillet_circle signatures (A3-A5) and spec types.
- `vendor/truck/truck-shapeops/src/boolean/assemble.rs` — the boolean
  entry (A6) and `BoolOp` variants.
- `vendor/truck/truck-modeling/src/cad.rs` — the fold family
  (translate/uniform_scale/mirror/mirror_about_plane/rotate — A9/A10)
  and `solid_bounding_box`/`make_face`/`make_hull`.
- `vendor/truck/truck-modeling/src/section.rs` (in truck-shapeops) —
  `split_by_plane`/`section_faces` signatures.
- `vendor/truck/truck-shapeops/tests/boolean_m2.rs` — the battery's
  fixture conventions; read, do not edit.

## Tests required (new file `tests/conformance_battery.rs`)

The D3 list, named exactly:

1. `facade_naming_table_covers_every_landed_entry` — every D2 entry
   resolves (compiles + dispatches) and the module doc's table matches
   the D2 list (a compile-time presence battery; assert one happy-path
   call per entry where cheap).
2. `facade_constructive_sequence_plate_with_fillet_and_hole` — the D3.1
   flagship sequence end-to-end (extrude → fillet → Subtract), plus the
   two split rows (plain-box positive; filleted-solid typed refusal).
3. `facade_metamorphic_rows_still_hold_through_facade` — the D3.2 rows.
4. `facade_refusal_cases_are_typed` — the D3.3 cases (assert each
   refusal arm; machine-check the arms you actually got).
5. `facade_boolean_modes_map_to_boolop` — the D2 Mode mapping: the three
   modes produce the same results as the corresponding `BoolOp` calls.
6. `consumability_tessellation_closed_on_generated_carriers` — the D3.4
   Closed rows (release-gated per Finding 3; the gate itself is part of
   the test's job — in debug the test asserts the gate compiles and
   returns early with the citation comment).
7. `consumability_tessellation_oblique_recorded_boundary` — the D3.4
   oblique row (Regular, the Finding 2 citation, release-gated).

## H-3 (house rule; V4 is a text gate on your diff)

No ADDED line carries a bare absolute small literal (`1e-N`) without the
same-line `// H-3` opt-out. Dyadic constants and geometry-derived values
only; the mesh tolerance 0.01 is a length constant and carries the H-3
comment. Run `& "C:\Program Files\Git\bin\bash.exe"
scripts/kernel-gates.sh HEAD` before writing RESULT.json (bare `bash` is
the WSL stub). CLIPPY EVERY CHANGED FILE — `cargo clippy --locked -p
truck-shapeops --all-targets --no-deps` UNFILTERED before committing.

## Done when

Commit on the current branch (subject
`BG-CAD-P8-FACADE: build123d-shaped facade + conformance battery`)
BEFORE writing RESULT.json AT THE WORKTREE ROOT (that exact path — not
`loop/results/`), then, all green:

```
cargo check --locked -p truck-shapeops
cargo fmt --check -p truck-shapeops
cargo test --locked -p truck-shapeops --lib
cargo test --locked -p truck-shapeops --test conformance_battery
cargo test --locked -p truck-shapeops --test fillet_circle
cargo test --locked -p truck-shapeops --test chamfer
cargo test --locked -p truck-shapeops --test fillet_pp
cargo test --locked -p truck-shapeops --test boolean_m2
cargo test --locked -p truck-shapeops --test interior_loop
cargo test --locked -p truck-shapeops --test resew
cargo test --locked -p truck-shapeops --test cut_boundaries
cargo test --locked -p truck-shapeops --test split_plane
cargo test --locked -p truck-shapeops --test transform_metamorphic
cargo clippy --locked -p truck-shapeops --all-targets --no-deps
& "C:\Program Files\Git\bin\bash.exe" scripts/kernel-gates.sh HEAD
```

All landed suites must pass UNCHANGED. The lib suite's
`healing::tests::step_import` and the upstream `fillet::complex_surface`
failures are the recorded environmental ones (fail at base, V5 knows).

## Forbidden

- Do not edit anything outside `write_allow` (in particular NOT the
  landed entries the facade composes — a needed signature change is a
  SPEC_GAP).
- Do not add geometric content (D4) or new refusal arms (D5).
- Do not attempt Python bindings, selectors, or any new kernel math
  (plan §1; booked programs).
- GATE-3/H-4: `Face::debug_new` is banned in added lines.
- Do not write the tolerance-migration constructor name in prose or
  comments (GATE-4 counts raw text, comments included).
- No instrumentation traces may survive in the committed diff.

## Stop conditions

- `ANCHOR_MISMATCH` — report the measured count, change nothing.
- `SPEC_GAP` — a decision contradicts the tree (including any D2
  signature that cannot adapt without geometric content); QUESTION.md
  with the empirical proof.
- A D3.1 sequence step fails after a D2-faithful composition — stop and
  report the failing step and its landed-entry refusal verbatim.
- The consumability rows cannot reproduce the probe's measured
  conditions — stop and report your measured conditions.

RESULT.json: `{"id":"BG-CAD-P8-FACADE","status":"DONE","contracts":[...],
"tests_added":7,"deviations":[...],"notes":"..."}` — the D2 signature
adaptations go in notes; every deviation with your derivation; deviations
are expected to be RIGHT.
