---
id: BG-CAD-P10-FRAMED
class: design
crates: [truck-modeling, truck-shapeops]
write_allow:
  - vendor/truck/truck-modeling/src/cad.rs
  - vendor/truck/truck-modeling/src/extrude.rs
  - vendor/truck/truck-modeling/src/geom_impls.rs
  - vendor/truck/truck-modeling/proptest-regressions/geom_impls.txt
  - vendor/truck/truck-modeling/tests/transforms.rs
  - vendor/truck/truck-shapeops/tests/transform_metamorphic.rs
tests_required:
  - rotate_about_z_extrude_metamorphic
  - mirror_general_plane_assembles
  - mirror_axis_aligned_still_green
  - oblique_extrude_circle_assembles
  - oblique_extrude_refuses_dz0
  - transform_union_metamorphic
  - transform_difference_metamorphic
  - transform_scale_metamorphic
budget: {turns: 45, ctx_tokens: 140000}
---

# BG-CAD-P10-FRAMED — framed emission + general transforms, the T(A op B) = T(A) op T(B) metamorphic

Program: `docs/BUILD123D_COVERAGE_PLAN.md` P10 (Tier 1), riding the LANDED
P9 conjugation normalization (placed cylinders are now consumable in the
Contact dispatch) and the LANDED P1 fold family (`translate_solid`,
`uniform_scale_solid`, `mirror_solid` axis-aligned, all through the
`fold_solid` similarity machinery). The pre-dispatch num3-scratch probe on
the oblique wall's target state is DONE and PASSED (its evidence is QUOTED
here; the probe source is untracked scratch). Everything below is
pre-decided; churn, don't design. Contradiction with the tree = `SPEC_GAP`.

## Problem

The landed fold machinery already emits `Placed` carriers exactly
(BG-CE-006-r2: under a non-identity linear part, analytic carriers place,
planes carry bare) — but the entry surface stops at axis-aligned mirrors,
and the vector extrusion REFUSES the circle-profile oblique case it should
emit (`extrude.rs:94-96`: "the wall would be an oblique cylinder, a Placed
carrier outside the canonical set"). P10 opens both: general rigid rotation
and general-plane mirror as fold entries, and the oblique circle extrusion
emitted as the affine-placed right cylinder — with the plan's §9
metamorphic gate `T(A op B) = T(A) op T(B)` as the test algebra.

## The probe evidence (quoted — the worker's worktree has no scratch/)

**W1 — the landed `Cylinder` subs convention.**
`Cylinder::new((0,0,0), 2).subs(u, v)`: `(0,0) = (2,0,0)`, `(0,1) =
(2,0,1)`, `(1,0) = (1.081, 1.683, 0)` — u is the angle (u=0 at +x), v is
the RAW z-offset. A sweep along `dir = (dx, dy, dz)` with parameter
`t ∈ [0, 1]` is therefore the SHEAR `S` with columns `(x̂, ŷ, dir)` applied
to the right cylinder: `S·right.subs(u, t) = (r·cos u + t·dx, r·sin u +
t·dy, t·dz)` — machine-checked exact at sampled `(u, t)`.

**W2 — the oblique target state ASSEMBLES FIRST TRY.** Circle r=2 at z=0,
`dir = (1,0,1)` (`both=false`, span `[0, dir]`): 3 faces, 2 unique edges,
2 unique vertices —

- bottom cap: `Plane` z=0, wire = the self-loop circle r=2 at (0,0,0)
  INVERSE (the landed extrude census convention);
- top cap: `Plane` z=1 (the profile plane TRANSLATED by dir — the caps
  stay parallel to the profile plane, they are NOT perpendicular to dir),
  wire = the self-loop circle r=2 at (1,0,1) FORWARD;
- wall: `Surface::Processor(Processor::with_transform(
  Box::new(Surface::Cylinder(right)), S))` with `right =
  Cylinder::new((0,0,0), 2)` and `S` the shear columns `(x̂, ŷ, dir)`,
  boundary wires [bottom junction FORWARD, top junction INVERSE] — the
  same pairing shape as the landed z-parallel wall (bottom forward, top
  inverse). `Solid::try_new` accepts.

**W3 — the junction circles lie ON the placed wall exactly.** Bottom
circle point at angle u = `S·right.subs(u, 0)`; top circle point =
`S·right.subs(u, 1)` — both machine-checked at 9 samples against the
translated-profile circles.

## Anchors (measured 2026-08-29 at HEAD `da5b6f9`; re-derive before writing RESULT.json)

| id | file | pattern | count |
|----|------|---------|-------|
| A1 | vendor/truck/truck-modeling/src/cad.rs | `pub fn translate_solid\(` | 1 |
| A2 | vendor/truck/truck-modeling/src/cad.rs | `pub fn mirror_solid\(` | 1 |
| A3 | vendor/truck/truck-modeling/src/cad.rs | `pub fn uniform_scale_solid\(` | 1 |
| A4 | vendor/truck/truck-modeling/src/extrude.rs | `non-z-parallel` | 1 |
| A5 | vendor/truck/truck-modeling/src/extrude.rs | `pub fn extrude_profile_vector\(` | 1 |
| A6 | vendor/truck/truck-modeling/tests | `transforms.rs` | 0 (new file) |
| A7 | vendor/truck/truck-shapeops/tests | `transform_metamorphic.rs` | 0 (new file) |

All anchors are the PRE-packet tree. No new module anywhere: `rotate_solid`
and the general mirror live IN `cad.rs` beside the landed fold family; the
oblique emission lives IN `extrude.rs` at the refusal site.

## Decisions already made for you

**D1 — `rotate_solid` (cad.rs).**

```rust
/// Rotates `solid` about the axis through `axis_point` with direction
/// `axis_dir` (need not be unit; normalized internally) by `angle`
/// radians. The similarity fold with the rigid rotation matrix.
pub fn rotate_solid(
    solid: &Solid,
    axis_point: Point3,
    axis_dir: Vector3,
    angle: f64,
) -> Outcome<Solid>;
```

Build the rigid rotation `R` (Rodrigues form about the normalized axis),
translate the axis point to the origin, rotate, translate back — ONE
combined matrix — and route through the LANDED `fold_solid` +
`certify_carriers` machinery unchanged (A1-A3's siblings are the shape).
A zero-length `axis_dir` or non-finite angle refuses `Refusal::Empty`.
The emission rule is the LANDED one, not new code: planes carry bare
(a `Plane` is closed under affine maps), curved analytic carriers place
under the non-identity linear part. Machine-check both halves in test 1.

**D2 — general-plane mirror (cad.rs).**

```rust
/// Mirrors `solid` about the plane through `plane_point` with normal
/// `plane_normal` (need not be unit; normalized internally). The fold
/// with the Householder reflection `I - 2nn^T` composed with the
/// translation; det < 0 exactly like the landed axis-aligned mirror.
pub fn mirror_about_plane(
    solid: &Solid,
    plane_point: Point3,
    plane_normal: Vector3,
) -> Outcome<Solid>;
```

The landed axis-aligned `mirror_solid` (A2) stays UNTOUCHED (its tests are
the identity guard); the new entry is the general form through the same
`fold_solid` path. A zero-length normal refuses `Empty`.

**D3 — the oblique circle extrusion (extrude.rs).** The circle-boundary
refusal at A4 flips to emission for `dir` with `dz != 0` and finite
components: the wall is EXACTLY the probe's W2 construction —
`Surface::Processor(Processor::with_transform(Box::new(
Surface::Cylinder(right_cyl)), S))`, `right_cyl = Cylinder::new((cx, cy,
z0), r)` (the profile circle's center foot and radius), `S` the shear
columns `(x̂, ŷ, dir)` with translation `(cx, cy, z0)` in `w`; the wall
face's wires are [bottom junction circle, top junction circle] with the
landed bottom-forward/top-inverse pairing; the caps are the profile planes
translated by `dir` (bottom inverse, top forward — the W2 census). The
landed non-circle-curve behavior is UNCHANGED (a curved profile edge that
is not a circle with oblique `dir` keeps refusing
`NonCanonicalCarrier`), and `dz == 0` keeps refusing `Empty`. A
`both == true` span is the same wall with `v ∈ [−1, 1]` (the shear
parameter): machine-check the wire construction you actually need
(record it). REPRESENTATION NOTE (recorded, not a check): the sheared
placement is NOT a P9 similarity — the funnel's P9 screen refuses sheared
placed cylinders, so an oblique-extruded solid's wall defers in Contact
(funnel admission for sheared placements is a booked follow-up). The
emission is representational; test 10's metamorphic is carrier-level.

**D4 — the metamorphic battery (truck-shapeops tests).** The plan's §9
gate `T(A op B) = T(A) op T(B)` for a similarity T, realized end-to-end
through the LANDED `boolean()` entry (truck-shapeops depends on
truck-modeling; truck-modeling does NOT depend on truck-shapeops — which
is why this battery lives in `truck-shapeops/tests/`). T is restricted to
transformations that keep every consumed carrier in its landed cell:
rotation about z + translation (planes and lines stay BARE — the boolean
pairs never leave the landed cells) and uniform dyadic scale. The equality
assertion is structural: same face/edge/vertex counts, same carrier kinds,
and every vertex point of `T(A op B)` equals the corresponding point of
`T(A) op T(B)` exactly (dyadic fixtures; the transforms are exact on
dyadics). The oblique metamorphic (test 10) is CARRIER-level: both solids
assemble, face counts equal, and the placed wall's subs points agree under
T at sampled parameters (the shear composition is exact).

**D5 — refusals (zero new arms).** `Refusal::Empty` (degenerate axes,
normals, angles, `dz == 0`) and the landed
`UnsupportedEnvelope(NonCanonicalCarrier)` (non-circle curved profiles
with oblique dir, unchanged). No new `Refusal`/`EnvelopeCase`/
`UnresolvedWitness`/`Collapse` arms — a perceived need is a SPEC_GAP.

**D6 — certificates.** `Solid::try_new` is the acceptance gate for every
constructed solid. The folds inherit the landed `certify_carriers` gate.
Tests machine-check: rotated/mirrored carrier subs points against the
hand-computed images (the fold is exact — assert at f64 equality for
dyadic-friendly angles like π/2 about a coordinate axis, or at your
achieved precision for general angles, recorded); the oblique wall's subs
identity (W3's check, through the landed entry).

**D7 — boundaries (booked follow-ups; refuse, do not attempt).** Revolve
about an arbitrary axis is OUT of scope (its seam logic is keyed on the
z-axis frames — its own probe-sized derivation). Sheared placements in
the Contact funnel (D3's note) are OUT. General similarity folds BEYOND
rotate/mirror/scale/translate are OUT. Do not touch the landed
`fold_solid`/`certify_carriers` interiors — the entries compose matrices
and route.

**D8 — AMENDMENT (session 42, orchestrator): stabilize the landed
`geom_impls::test_circle_arc_tangent0` property.** The verify's V5 gate
exposed a flaky landed property: proptest found the persisting failing
seed `p0=(-9.381, 0, 0), p1=(0, 0, 8.388), tangent0=(0, -5.520,
5.854), t=0.9998653721537082` (now persisted in
`proptest-regressions/geom_impls.txt`, which this packet commits). The
seed's signature: `t` is ARBITRARILY CLOSE to 1 — the tangency parameter
approaches the arc's endpoint. Mandate, in order:

1. Reproduce with the persisted seed (it re-runs before any novel case).
2. DIAGNOSE with a derivation: as `t → 1`, which quantity in the
   construction degenerates (the three-point circle's conditioning as
   the tangency point approaches the endpoint? a division by a vanishing
   chord?). Machine-check the blow-up numerically along the seed's path.
3. EITHER the underlying solve in `geom_impls.rs` is genuinely wrong for
   this family — fix it — OR the property samples a degenerate
   configuration family it must exclude: add the documented precondition
   (e.g. `t` bounded away from 0 and 1 by a justified interval — justify
   the bound from the conditioning derivation, not convenience).
4. NEVER loosen the `near` tolerance without a conditioning derivation
   that shows the achieved precision is the construction's ceiling (the
   no-gate-loosening rule applies in full).
5. Commit the persisted regressions file so the fix is verified against
   the exact failing seed deterministically, forever.

This file's test module is test-only territory for everything EXCEPT the
property in question; do not touch other `geom_impls` code paths unless
your diagnosis lands there (record it).

## Template

- `vendor/truck/truck-modeling/src/cad.rs:249-360` — the landed fold
  family (A1-A2-A3): `fold_solid`, `certify_carriers`, and the three
  entries your new entries mirror.
- `vendor/truck/truck-modeling/src/extrude.rs:87-200` — the vector
  extrusion (A4-A5): the refusal site, the z-parallel wall construction
  your oblique wall mirrors, the cap conventions.
- `vendor/truck/truck-modeling/tests/cad_p1.rs` — the P1 test-file
  conventions (fixture style, H-3 comments); read, do not edit.
- `vendor/truck/truck-shapeops/tests/boolean_m2.rs` — the boolean battery
  conventions (fixture profiles, the `resew`-style boundary-crossing
  rule); read, do not edit.

## Tests required

AMENDMENT (session 42, after the worker's SPEC_GAP, `amended_by:
orchestrator`): the original tests 1 (`rotate_solid_rigid_carriers`) and
10 (`transform_oblique_extrude_metamorphic`) are MOVED OUT of this packet —
they require folding a full-circle disk-extrude solid, and the LANDED fold
machinery aborts on self-loop edges in debug builds (the worker's
QUESTION.md proves the landed `translate_solid` shares the limitation;
locus chain `wire.rs:628 -> edge.rs:55-60`, debug-only `front == back`
check). No self-loop-free cylinder solid is constructible (`arrange`
refuses arc-seamed circle profiles). The two tests are re-booked as rows
of the BG-CAD-P8-FACADE battery, to land after the truck-topology
self-loop-safe fold fix packet. D1/D2/D3 themselves are UNCHANGED and
delivered; `rotate_solid`'s placed-carrier emission is covered by test 2
(planes stay bare) and by the P8 rows to come.

Fixtures: the boolean_m2 profile recipe; T = Rz(pi/2) + translation on a
**2x2** rect profile (the worker's machine-checked deviation: a 4x4
rotated rect refuses `arrange` — 4c rounds so the rotated rect's opposite
edges are not exactly anti-parallel; 2c is exactly representable and keeps
the metamorphic at f64 equality).

New file `vendor/truck/truck-modeling/tests/transforms.rs` (dyadic
fixtures; `truck_modeling` is the crate under test):

1. `rotate_solid_rigid_carriers` — a disk-extrude cylinder (r=2, h=2)
   rotated about the x-axis through the origin by π/2: assembles; census
   identical to the input; every `Plane` face still carries a BARE plane
   (the landed rule) and the `Cylinder` face is now `Placed` — machine-check
   subs points of the placed carrier against the hand-rotated images.
2. `rotate_about_z_extrude_metamorphic` — T = Rz(π/2) + translation on a
   rect profile: `T(extrude_profile(P, h))` vs `extrude_profile(T(P), h)`
   (rotate the PROFILE curves and re-arrange): census, carrier kinds, and
   vertex points (transformed) all equal.
3. `mirror_general_plane_assembles` — a box mirrored about the plane
   through (1,1,0) with normal (1,1,0): assembles; the mirrored vertices
   equal the hand-computed reflection images exactly.
4. `mirror_axis_aligned_still_green` — the landed `mirror_solid` on its
   own fixture answers exactly what it answered before (the identity
   guard; mirror the box, compare census + vertex points with the
   pre-packet behavior you machine-check at base).
5. `oblique_extrude_circle_assembles` — the probe's W2 THROUGH THE LANDED
   `extrude_profile_vector`: disk profile r=2 at z=0, dir (1,0,1): 3
   faces, 2 unique edges, 2 unique vertices; the wall is the placed
   affine cylinder (W3's subs machine-check); caps at z=0 and z=1.
6. `oblique_extrude_refuses_dz0` — dir (1,0,0) → `Refusal::Empty`
   (machine-check the arm; record it).

New file `vendor/truck/truck-shapeops/tests/transform_metamorphic.rs`
(the D4 battery; fixtures via `truck_modeling` + the boolean_m2 profile
recipe):

7. `transform_union_metamorphic` — T = Rz(π/2) + translation; two boxes
   (one small box straddling the other's boundary, the resew convention):
   `boolean(T(A), T(B), Union)` vs `T(boolean(A, B, Union))` — census +
   transformed vertex equality.
8. `transform_difference_metamorphic` — the same T, Difference.
9. `transform_scale_metamorphic` — uniform scale 2: `T(A − B)` vs
   `T(A) − T(B)` (dyadic scale keeps everything dyadic).
10. `transform_oblique_extrude_metamorphic` — T = Rz(π/2): assemble
    `T(extrude(disk, (1,0,1)))` and `extrude(T(disk-profile), (0,1,1))`
    (rotate the profile curves about z, re-arrange, extrude along the
    rotated dir): both assemble, face counts equal, the placed walls'
    subs points agree under T at sampled (u, v).

## H-3 (house rule; V4 is a text gate on your diff)

No ADDED line carries a bare absolute small literal (`1e-N`) without the
same-line `// H-3` opt-out. Dyadic constants and geometry-derived values
only (a `1/sqrt(2)` normalization factor is geometry-derived). Run
`& "C:\Program Files\Git\bin\bash.exe" scripts/kernel-gates.sh HEAD`
before writing RESULT.json (bare `bash` is the WSL stub). CLIPPY EVERY
CHANGED FILE — run `cargo clippy --locked -p truck-modeling
--all-targets` and `cargo clippy --locked -p truck-shapeops
--all-targets` UNFILTERED, then re-check with `--no-deps` (the dependency
crates' pre-existing findings are the recorded environmental ones); fix
all findings in YOUR files BEFORE committing.

## Done when

Commit on the current branch (subject
`BG-CAD-P10-FRAMED: general transforms + oblique circle extrusion + the T(A op B) metamorphic`)
BEFORE writing RESULT.json AT THE WORKTREE ROOT (that exact path — not
`loop/results/`), then, all green:

```
cargo check --locked -p truck-modeling
cargo check --locked -p truck-shapeops
cargo fmt --check -p truck-modeling
cargo fmt --check -p truck-shapeops
cargo test --locked -p truck-modeling --lib
cargo test --locked -p truck-modeling --test transforms
cargo test --locked -p truck-modeling --test cad_p1
cargo test --locked -p truck-modeling --test revolve_p5
cargo test --locked -p truck-modeling --test until_p4
cargo test --locked -p truck-shapeops --test transform_metamorphic
cargo test --locked -p truck-shapeops --test fillet_circle
cargo clippy --locked -p truck-modeling --all-targets --no-deps
cargo clippy --locked -p truck-shapeops --all-targets --no-deps
& "C:\Program Files\Git\bin\bash.exe" scripts/kernel-gates.sh HEAD
```

All landed suites must pass UNCHANGED (`cad_p1`, `revolve_p5`, `until_p4`,
`fillet_circle`; the truck-modeling lib suite's `healing::tests::
step_import`-class environmental failures excepted if they fire —
machine-check the base and record).

## Forbidden

- Do not edit `revolve.rs`, `until.rs`, `primitive.rs`, `builder.rs`, or
  anything outside `write_allow`.
- Do not add `Refusal`/`EnvelopeCase`/`UnresolvedWitness`/`Collapse` arms
  (D5).
- Do not attempt D7's booked follow-ups (arbitrary-axis revolve, sheared
  placements in the funnel).
- GATE-3/H-4: `Face::debug_new` is banned in added lines.
- Do not write the tolerance-migration constructor name in prose or
  comments (GATE-4 counts raw text, comments included).
- No instrumentation traces may survive in the committed diff.

## Stop conditions

- `ANCHOR_MISMATCH` — report the measured count, change nothing.
- `SPEC_GAP` — a decision contradicts the tree; QUESTION.md with the
  empirical proof.
- A booked happy-path case fails `Solid::try_new` after a D1-D3-faithful
  construction — stop and report the closure witness verbatim (the
  probe's W2 structure is the oblique reproducibility witness).
- The landed `fold_solid`/`certify_carriers` machinery refuses a D1/D2
  matrix it should accept — stop and report the refusal verbatim (do not
  patch the fold interiors; that is a SPEC_GAP).

RESULT.json: `{"id":"BG-CAD-P10-FRAMED","status":"DONE","contracts":[...],
"tests_added":10,"deviations":[...],"notes":"..."}` — every deviation with
your derivation; deviations are expected to be RIGHT.
