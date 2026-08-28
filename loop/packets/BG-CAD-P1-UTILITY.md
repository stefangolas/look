---
id: BG-CAD-P1-UTILITY
class: design
crates: [truck-modeling]
write_allow:
  - vendor/truck/truck-modeling/src/cad.rs
  - vendor/truck/truck-modeling/src/lib.rs
  - vendor/truck/truck-modeling/Cargo.toml
  - vendor/truck/truck-modeling/tests/cad_p1.rs
  - Cargo.lock
tests_required:
  - bounding_box_of_flagship_extrude_is_exact
  - translated_solid_is_congruent
  - uniform_scaled_solid_is_congruent
  - mirrored_solid_is_congruent
  - mirrored_flagship_box_is_reflected
  - make_face_rectangle
  - make_face_with_hole
  - make_hull_square
  - make_hull_degenerate_collapses
  - profile_off_plane_refuses
budget: {turns: 40, ctx_tokens: 120000}
---

# BG-CAD-P1-UTILITY — Phase 7 utility surface + planar face construction

Program: `docs/BUILD123D_COVERAGE_PLAN.md` P1 (Tier 0). Everything below is
pre-decided; your job is to churn, not design. If a decision here contradicts
the tree, that is a `SPEC_GAP`, not an edit.

## Problem

The build123d coverage program opens with the operations that unlock the
frontend with zero new solver mathematics: a certified bounding box, the
similarity fold (translate / uniform scale / axis-aligned mirror), planar
face construction from the landed arrangement, and the 2-D convex hull.
Each is a composition of landed machinery; each must return
`Outcome<T>` (certified value or typed `Refusal`), never an uncertified
maybe-answer, and every output must remain downstream-consumable
(canonical carriers recognized by `recognize_surface`/`recognize_curve`).

## Anchors (measured 2026-08-28 at HEAD; re-derive before writing RESULT.json)

| id | file | pattern | count |
|----|------|---------|-------|
| A1 | vendor/truck/truck-modeling/src/lib.rs | `pub mod` | 8 |
| A2 | vendor/truck/truck-modeling/src/extrude.rs | `fn select_material` | 1 |
| A3 | vendor/truck/truck-evidence/src/lib.rs | `pub use enclosure` | 1 |
| A4 | vendor/truck/truck-modeling/src/mapped.rs | `Mapped<T> for Solid` | 1 |
| A5 | vendor/truck/truck-modeling/Cargo.toml | `truck-evidence` | 0 |
| A6 | vendor/truck/truck-base/src/pred.rs | `pub fn orient2d` | 1 |

A5 is the dispatch-time state (no dependency edge yet); after your manifest
edit it reads 1 — that is the expected divergence, not a mismatch. A1 becomes
9 once you declare `pub mod cad;`.

## Decisions already made for you

**D0 — manifest edge.** Add
`truck-evidence = { version = "<match the vendored manifest>", path = "../truck-evidence" }`
to `vendor/truck/truck-modeling/Cargo.toml` `[dependencies]`, copying the
exact style of the existing path entries. Acyclic: truck-evidence depends only
on base/geotrait/geometry. `Cargo.lock` at the repo root updates as part of
the diff and is in your write allow. (INV-104 precedent.)

**D1 — module shape.** New file `vendor/truck/truck-modeling/src/cad.rs`,
declared `pub mod cad;` in lib.rs (place it so
`cargo fmt --check -p truck-modeling` passes; the current pub-mod order is not
alphabetical, so keep the file's doc comment and let fmt decide).
The module opens with the house deny header, copied from
`truck-geometry/src/recognize.rs:22-29`:
`#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::todo, clippy::unimplemented, clippy::indexing_slicing)]`.

**D2 — `solid_bounding_box`.**

```rust
pub fn solid_bounding_box(
    solid: &Solid<Point3, Curve, Surface>,
    budget: &mut Budget,
) -> Outcome<BoundingBox<Point3>>
```

`BoundingBox<Point3>` is landed (`truck_base::bounding_box::BoundingBox`,
already re-exported by truck-modeling's `base` module) and is the accumulator;
the new work is the per-face certified derivation:

- Lift each face with `recognize_surface` (`truck_geometry::recognize`).
  `Unrecognized`, `RevolutedCurve`, and `CanonicalSurface::Placed` all refuse
  `UnsupportedEnvelope(EnvelopeCase::NonCanonicalCarrier)` — P1 emits and
  consumes bare canonical carriers only.
- Per boundary edge, the 3-D box is `EnclosureCurve::enclose` over the edge's
  own bounded parameter range (landed per-carrier impls; use the edge's
  `range()`).
- Face box rule, by face carrier:
  - `Plane` face → hull of its boundary edges' boxes. Sound because a compact
    planar region's extreme points lie on its boundary.
  - `Cylinder` face → hull of its boundary edges' boxes. The wall's extreme
    xy is achieved on the rims (radius is constant in v) and its z-extent is
    bracketed by the rim circles. Machine-check this claim with a sampled
    witness in test 1's family; if your machine-check falsifies it, STOP and
    report `SPEC_GAP` — do not silently widen the rule.
  - `Sphere` face → the full carrier box `[c−r, c+r]³` (a cap's pole is off
    its boundary; the hull rule is unsound here — this is why the sphere arm
    exists).
  - `Cone` face → hull of its boundary edges' boxes plus the apex point.
  - `Torus` → `UnsupportedEnvelope(NonCanonicalCarrier)` (Tier 2).
- Traverse stored wires: `Face::absolute_boundaries()` returns the STORED
  wires; `boundaries()` returns orientation-adjusted ones. For box purposes
  use the stored wires (session-38 naming trap).
- `budget` is taken for API stability and spent NOT AT ALL — this operation
  performs no subdivision or Newton work; document that in the doc comment.

**D3 — the similarity fold.** One internal struct implementing the three
`GeometricMapping` traits (landed: `truck-modeling/src/topo_traits`,
applied by the landed `Mapped` impls over the whole Vertex→Solid chain,
`mapped.rs:4-64`):

```rust
struct SimilarityFold { mat: Matrix4 }   // cgmath, re-exported by the crate
```

point closure: affine map of `Point3`; curve/surface closures:
`Transformed::transform(&self.mat)` (landed for the `Curve`/`Surface` enums,
`truck-geometry/src/canonical.rs` macro dispatch).

Three public operations, each returning `Outcome<Solid>`:

```rust
pub fn translate_solid(solid: &Solid<Point3, Curve, Surface>, t: Vector3) -> Outcome<Solid>;
pub fn uniform_scale_solid(solid: &Solid<Point3, Curve, Surface>, s: f64) -> Outcome<Solid>;
pub fn mirror_solid(solid: &Solid<Point3, Curve, Surface>, plane: &Plane) -> Outcome<Solid>;
```

- Non-finite or non-positive `s` → `Refusal::Empty` (the extrude.rs
  non-positive-height convention).
- `mirror_solid` accepts ONLY axis-aligned mirror planes (normal in
  {±x, ±y, ±z}; plane through any point `c`: x ↦ 2cᵢ − x). Anything else
  would emit `Placed` carriers — refuse
  `UnsupportedEnvelope(NonCanonicalCarrier)` with a doc comment saying
  exactly that. No other refusal arm is invented for this.
- **Mirror parity rule**: `det(mat) < 0` ⇒ after mapping, invert every face
  of every shell (deterministic; an improper affine map reverses each
  surface's normal, so the orientation flags must flip for the shell to stay
  outward-consistent). `Solid::try_new` is the acceptance gate either way —
  a refusal there is a typed refusal about the transform, never a panic.
- Defensive certificate: after mapping, `recognize_curve`/`recognize_surface`
  must recognize every transformed carrier; anything `Unrecognized` refuses
  `UnsupportedEnvelope(NonCanonicalCarrier)`. (For these three ops it cannot
  fire; it is the fold's certificate that the carrier set was preserved.)
- Congruence: the topology STRUCTURE is identical (same face/edge/wire
  counts, same shared-edge identity pattern `Mapped` already preserves).

**D4 — `make_face`.**

```rust
pub fn make_face(profile: &[Curve]) -> Outcome<Vec<Face>>;
```

- v1 frame: the profile lies in the z = 0 plane. Every `Line` endpoint with
  z ≠ 0, or every `Circle` whose axis is not ±z or whose center has z ≠ 0,
  refuses `UnsupportedEnvelope(NonCanonicalCarrier)`.
- `arrange(profile, None)` (landed S1) → material regions by the session-28
  containment rule: bounded `winding == 1` regions not strictly inside
  another bounded `winding == 1` region's boundary cycle. Implement this
  iteration locally in cad.rs (~20 lines); **do not edit extrude.rs** — its
  `select_material` stays private and untouched (A2 = 1 before and after).
- One face per material region (build123d semantics: multiple disjoint loops
  produce multiple faces). Face/wire/edge construction copies the landed
  cap recipe in extrude.rs — closed circle edges are built with
  `Edge::new_unchecked` (the session-28 `NotSimpleWire` trap), caps get
  explicitly constructed wires, and the annulus-with-holes shape carries two
  boundary wires with NO seam edges.
- The face's surface is `Plane` through z = 0 (the landed extrude cap
  convention).

**D5 — `make_hull`.**

```rust
pub fn make_hull(points: &[Point3]) -> Outcome<Face>;
```

- All points must have z = 0 (else `NonCanonicalCarrier`).
- Monotone-chain convex hull using the landed exact predicate
  `truck_base::pred::orient2d` (A6). Any `CertifiedPred::Unresolved` result
  refuses `NumericallyUnresolved { .. }` with
  `UnresolvedWitness::UncertifiedContainment` (it cannot fire on dyadic test
  witnesses; it is the escalation contract).
- Fewer than 3 distinct points, or a hull of zero area (all collinear), refuses
  `Refusal::Collapsed(..)` — pick the `Collapse` arm that fits and name your
  choice in RESULT notes.
- Build the CCW closed wire of `Line` edges and finish through the same
  single-region face builder D4 uses.

## Template

- `vendor/truck/truck-modeling/src/extrude.rs` — the certified-construction
  house pattern: `Outcome`, material selection, cap/wire/edge construction,
  the `Solid` type alias in scope. Read it end to end before writing cad.rs.
- `vendor/truck/truck-geometry/src/recognize.rs` — the deny-header form and
  the recognizer you consume.
- `vendor/truck/truck-modeling/src/mapped.rs` — the `Mapped` chain you ride.

## Tests required (new file `vendor/truck/truck-modeling/tests/cad_p1.rs`)

Test-file header: the same deny list as cad.rs; write a tiny
`fn expect_ok<T>(r: Outcome<T>) -> T` helper that unwraps via `match` +
panic so the deny lints stay satisfied (recognize.rs's test module is the
precedent). No bare `1e-N` literals anywhere (H-3, below); dyadic constants
(0.5, 4.0) are fine.

1. `bounding_box_of_flagship_extrude_is_exact` — build a rectangle profile
   (the extrude.rs test pattern), `extrude_profile` it, assert
   `solid_bounding_box` equals the exact dyadic box `[0,4]×[0,4]×[0,h]`
   (min/max corners exact — the box derivation is closed-form on planes).
2. `translated_solid_is_congruent` — translate by (1, 2, 3): same
   face/edge/wire counts, every vertex point shifted exactly, box shifted
   exactly, `Solid::try_new` ok, all carriers still recognized.
3. `uniform_scaled_solid_is_congruent` — scale 2.0 about the origin: same
   structure, box doubled, carriers canonical.
4. `mirrored_solid_is_congruent` — mirror across x = 0 (plane normal −x
   through origin): same structure, try_new ok, carriers canonical.
5. `mirrored_flagship_box_is_reflected` — the mirrored flagship's box is the
   exact reflection of test 1's box.
6. `make_face_rectangle` — rectangle profile → `Vec` of exactly 1 face,
   1 boundary wire, on the z = 0 plane, orientation such that the face
   normal is +z.
7. `make_face_with_hole` — rectangle-minus-circle profile (the flagship
   profile) → 1 face with 2 boundary wires, NO seam edges.
8. `make_hull_square` — a point set whose hull is the unit square → 1 face,
   4 edges, +z normal.
9. `make_hull_degenerate_collapses` — three collinear points → the
   `Collapsed` refusal.
10. `profile_off_plane_refuses` — `make_face` on a profile with one vertex at
    z = 1 → `UnsupportedEnvelope` (machine-check the arm you got is
    `NonCanonicalCarrier`).

## H-3 (house rule; V4 is a text gate on your diff)

Every ADDED line containing a bare absolute small literal (`1e-N` form) fails
V4 unless the line ends with the same-line opt-out `// H-3`. There should be
NONE in this packet: use dyadic literals and named consts; import
`truck_base::tolerance::*` if a tolerance comparison is needed. Run
`& "C:\Program Files\Git\bin\bash.exe" scripts/kernel-gates.sh HEAD` before
writing RESULT.json (V4 hardcodes Git Bash; a bare `bash` is the WSL stub).

## Done when

Commit your work on the current branch (subject
`BG-CAD-P1-UTILITY: certified utility surface + planar face construction`)
BEFORE writing RESULT.json, then, all green:

```
cargo check --locked -p truck-modeling
cargo fmt --check -p truck-modeling
cargo test --locked -p truck-modeling --lib
cargo test --locked -p truck-modeling --test cad_p1
cargo clippy --locked -p truck-modeling --all-targets
& "C:\Program Files\Git\bin\bash.exe" scripts/kernel-gates.sh HEAD
```

## Forbidden

- Do not edit `extrude.rs`, `arrange.rs`, `recognize.rs`, or anything outside
  `write_allow` (V1 rejects the ripple; the write set already covers the
  manifest edge and lock).
- Do not add `Refusal`/`EnvelopeCase`/`UnresolvedWitness` arms (program rule:
  zero new arms — a perceived need is a SPEC_GAP).
- Do not emit `Placed` carriers (Tier 1's business, P9/P10).
- GATE-3/H-4: `Face::debug_new` is banned in added lines; use
  `Face::new_unchecked`/construction recipes that pass `Solid::try_new`.
- Do not write the tolerance-migration constructor name in any prose or
  comment — GATE-4 counts raw text and a deferral would read as a migration.

## Stop conditions

- `ANCHOR_MISMATCH` — an anchor disagrees with the tree; stop, report the
  measured count, change nothing.
- `SPEC_GAP` — a decision above contradicts the tree or is unachievable; ask
  in QUESTION.md with the empirical proof. That is the loop's most valuable
  output.
- Machine-check falsifies the cylinder hull rule (D2) — stop and report.

RESULT.json: `{"id":"BG-CAD-P1-UTILITY","status":"DONE","contracts":[...],
"tests_added":10,"deviations":[...],"notes":"..."}` — every deviation
recorded with your derivation; deviations are expected to be RIGHT.
