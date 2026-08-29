---
id: BG-CAD-P5-REVOLVE
class: design
crates: [truck-modeling, truck-geometry]
write_allow:
  - vendor/truck/truck-modeling/src/revolve.rs
  - vendor/truck/truck-modeling/src/lib.rs
  - vendor/truck/truck-geometry/src/recognize.rs
  - vendor/truck/truck-modeling/tests/revolve_p5.rs
tests_required:
  - revolve_rectangle_full_turn_is_tube
  - revolve_carriers_are_analytic
  - revolve_matches_extruded_annulus
  - revolve_partial_angle_valid
  - revolve_axis_crossing_refuses
  - revolve_axis_touch_refuses_collapsed
  - revolve_angle_bounds_refuse
  - revolve_circle_profile_refuses
  - revolve_result_survives_boolean
budget: {turns: 45, ctx_tokens: 130000}
---

# BG-CAD-P5-REVOLVE — Phase 7 revolve of line-edge profiles

Program: `docs/BUILD123D_COVERAGE_PLAN.md` P5 (Tier 0), realization table 6.2
and the metamorphic gate in §9. Everything below is pre-decided; churn, don't
design. Contradiction with the tree = `SPEC_GAP`.

## Problem

build123d's `revolve` is the next Tier 0 operation. The plan's scoping
decouples it from the torus funnel: **line-edge profiles only** (circle edges
are table 6.3, Tier 2). The realization table (plan §6.2) turns every profile
line into a canonical carrier, so the emitted faces stay inside the landed
FF funnel and the result downstream-consumes through the certified Boolean.

## Anchors (measured 2026-08-28 at HEAD `8111be9`; re-derive before writing RESULT.json)

| id | file | pattern | count |
|----|------|---------|-------|
| A1 | vendor/truck/truck-modeling/src/lib.rs | `pub mod cad` | 1 |
| A2 | vendor/truck/truck-modeling/src/lib.rs | `pub mod revolve` | 0 |
| A3 | vendor/truck/truck-geometry/src/recognize.rs | `Surface::RevolutedCurve` | 1 |
| A4 | vendor/truck/truck-modeling/src/extrude.rs | `pub fn extrude_profile_vector\(` | 1 |
| A5 | vendor/truck/truck-modeling/src/cad.rs | `pub fn solid_bounding_box\(` | 1 |

A2 becomes 1 once you declare `pub mod revolve;` (expected divergence, not a
mismatch). A3 becomes 2 once the recognition arm names the variant
(D7); if you take the D7 fallback it stays 1 — either is a consistent end
state, say which in RESULT notes.

## Decisions already made for you

**D1 — module shape.** New file `vendor/truck/truck-modeling/src/revolve.rs`,
declared `pub mod revolve;` in lib.rs (place so
`cargo fmt --check -p truck-modeling` passes; mirror the extrude/cad block
style around lib.rs:102-115). The module opens with the house deny header
copied from `truck-geometry/src/recognize.rs:22-29`. New test file
`vendor/truck/truck-modeling/tests/revolve_p5.rs` with the same deny header
and a `fn expect_ok<T>(r: Outcome<T>) -> T` match-based helper (the
`tests/cad_p1.rs` pattern).

**D2 — signature and frame (the extrude family's house conventions).**

```rust
pub fn revolve_profile(
    profile: &[Curve],
    arrangement: &Arrangement,
    angle: f64,
) -> Outcome<Solid>;
```

- The profile's material region(s) live in the **xz-plane (y = 0)**; the
  revolve axis is the **z-axis**; the sweep turns from the +x direction
  toward +y (right-handed about +z). This is the same "canonical frame,
  general forms later" posture as the landed scalar `extrude_profile`
  (`extrude.rs:70`, FROZEN) — P9/P10 conjugation is the booked unlock for
  arbitrary axes, do not attempt it here.
- `angle <= 0.0`, non-finite, or `angle > TAU` refuses `Refusal::Empty`
  (the landed extrude non-positive-height convention).
- The profile region must lie strictly at x > 0. Any vertex with x < 0
  refuses `UnsupportedEnvelope(NonCanonicalCarrier)` — the revolve map
  double-covers there (the profile crosses the axis; REV-AXIS-CROSS is the
  booked formal follow-up). An edge ENDPOINT exactly at x = 0 (axis touch)
  refuses `Refusal::Collapsed` with a certificate naming the edge — the
  table 6.2 "collapsed edge becomes vertex" row is the booked follow-up, not
  v1 (a singular apex/disk-center vertex is a topology event this packet
  does not build).
- A Circle (or any non-Line) edge in the profile region's boundary refuses
  `UnsupportedEnvelope(NonCanonicalCarrier)` at the lift, before any
  construction is paid for — table 6.3 is Tier 2.
- Exactly ONE material region (the landed extrude v1 rule); the arrangement
  machinery upstream decides region-hood, so refuse `Empty` if the
  arrangement yields a different count, mirroring `extrude_interval`'s
  single-region guard.

**D3 — the carrier table (plan §6.2, exact derivations).** For each Line
edge of the region boundary, with the profile in the xz-plane and the axis
the z-line:

- **Vertical edge (x = c const, c > 0)** → `Cylinder` carrier, center on the
  z-axis, radius c, spanning the edge's z-range.
- **Horizontal edge (z = c const)** → `Plane` carrier, the z = c plane; the
  revolved face is the annulus (or disk — refused here, D2 axis touch)
  between the edge's x-endpoints.
- **Slanted edge (neither)** → `Cone` carrier: extend the edge's line to its
  z-axis crossing (x = 0 at some z*); the carrier cone has apex (0,0,z*)
  and half-angle atan(|dx/dz|) (derive the exact convention from
  `truck-geometry`'s Cone carrier — machine-check which parameterization the
  carrier stores before constructing). The revolved face is the frustum
  between the edge's endpoints. A slanted edge whose extension is parallel
  to the z-axis is the vertical row; a slanted edge whose extension never
  reaches x = 0 (horizontal) is the horizontal row.
- The carrier for every wall is constructed as the CANONICAL analytic type
  directly — you never emit `Surface::RevolutedCurve` (the same
  "canonical without emitting the decorator" posture as the plan's curtain
  table 6.1).

**D4 — full vs partial angle.**

- `angle == TAU`: each wall face is a closed annulus-style face with
  self-loop circle boundary wires — copy the landed construction recipe from
  `extrude.rs` (its cylinder walls already build circle self-loops that pass
  debug-build tests; do NOT edit extrude.rs, copy the recipe). The two planar
  caps coincide as the profile region's face at y = 0 (one cap face, built
  from the arrangement exactly as the extrude builds its bottom).
- `0 < angle < TAU`: the two end caps are the profile region's face at
  y = 0 and its rotated image at `angle` (both plain `Plane` carriers — a
  rotated plane is a canonical Plane, no Placed). Wall faces carry 4-edge
  boundary wires: two meridian Line edges (shared instances between
  adjacent wall faces where the profile is continuous — the shell closes
  through shared edges, the boolean splitter's invariant) and two arc edges
  (arcs of the carrier's circle at the face's z/x-range; construct arcs with
  whatever the landed recipes use for circle-arcs — `Edge::new_unchecked`
  is NOT needed if no self-loop is degenerate, follow the landed recipes).

**D5 — certificates.** `Solid::try_new` is the acceptance gate for every
constructed solid (the house pattern). The metamorphic gate (§9,
`revolve(line polygon) ≅ analytic primitive`) is test 3: the revolve of the
rectangle [r1,0]-[r2,0]-[r2,h]-[r1,h] is asserted carrier- and box-equal to
the landed `extrude_profile` of the annulus arrangement r1..r2 (build the
annulus via `arrange` exactly as the extrude tests do). Box equality via
`cad::solid_bounding_box` (A5).

**D6 — bounded secondary deliverable: RevolutedCurve recognition (D7
fallback allowed).** The plan books the `recognize.rs:151-155` comment's
follow-up. Implement: `Surface::RevolutedCurve(_)` gains an arm in
`recognize_surface` that — when the wrapped profile curve is a `Line` and
the revolve is full-period — derives the exact canonical carrier per the D3
table and returns the `ExactCanonical`/`Derived` witness form the landed
arms use. Machine-check FIRST what the witness vocabulary and parameter-map
convention require (read the landed `Derived`/`Placed` arms at
recognize.rs:156-179 and the `CanonicalParamMap` type): if deriving the
exact witness (including the correct u/v parameter-map roles for the
revolved frame) cannot be certified against the landed vocabulary WITHOUT
inventing new types, TAKE THE FALLBACK — leave recognize.rs untouched
(A3 stays 1) and record the blocker precisely in RESULT notes as a booked
follow-up. The fallback is a pre-made judgement, NOT a SPEC_GAP and NOT a
deviation; the primary contract stands alone.

## Template

- `vendor/truck/truck-modeling/src/extrude.rs` — the house pattern for the
  whole packet: entry signature shape (A4), region extraction from the
  arrangement, face construction recipes (circle self-loops, wall wiring),
  `Solid::try_new` as the gate, refusal conventions (D2). Read the landed
  cylinder-wall construction before D4.
- `vendor/truck/truck-modeling/src/cad.rs` — the P1 module shape you mirror
  (deny header, helper layout), `solid_bounding_box` (A5) for tests.
- `vendor/truck/truck-geometry/src/recognize.rs` — the witness vocabulary
  (D6) and the deny header.
- `vendor/truck/truck-modeling/tests/cad_p1.rs` — the test-file pattern
  (helper, H-3 discipline, box assertions).

## Tests required (new file `tests/revolve_p5.rs`, dyadic witnesses only)

1. `revolve_rectangle_full_turn_is_tube` — rectangle x ∈ [1,3], z ∈ [0,2]
   (r1=1, r2=3, h=2), full turn: valid solid, 4 faces, box exactly
   [−3,3]² × [0,2].
2. `revolve_carriers_are_analytic` — trapezoid (r1=1 at z=0, r2=3 at z=0,
   top edge x ∈ [1,2] at z=2): carriers exactly {Plane ×2, Cylinder ×1,
   Cone ×1}; each Cone/Cylinder's derived radius/angle matches the edge it
   came from (assert the carrier data you derived, not just the type).
3. `revolve_matches_extruded_annulus` — the D5 metamorphic: revolve of the
   rectangle vs landed `extrude_profile` of the annulus r1=1, r2=3, height 2:
   same face count, same carrier multiset, same exact box.
4. `revolve_partial_angle_valid` — the same rectangle, angle π/2
   (`FRAC_PI_2`, no bare literal): valid solid, 6 faces, box
   [0,3] × [0,3] × [0,2] exactly.
5. `revolve_axis_crossing_refuses` — a profile with one vertex at x = −1 →
   `UnsupportedEnvelope(NonCanonicalCarrier)`, with NO construction spent
   (assert the budget/refusal arm you actually got).
6. `revolve_axis_touch_refuses_collapsed` — an edge endpoint exactly at
   x = 0 → `Refusal::Collapsed`.
7. `revolve_angle_bounds_refuse` — angle 0, angle −1, angle TAU + π/8 →
   `Refusal::Empty` each.
8. `revolve_circle_profile_refuses` — a profile region whose boundary
   carries a Circle edge → `UnsupportedEnvelope(NonCanonicalCarrier)`.
9. `revolve_result_survives_boolean` — the tube downstream-consumes:
   `boolean(tube, BoolOp::Difference, small_box)` through the landed entry
   assembles a valid solid (truck-shapeops is NOT a truck-modeling
   dependency — add the boolean call in a way that respects layering: either
   the test file skips it and this test moves to the fallback list, or use
   the `Solid<Point3, Curve, Surface>` value directly — machine-check the
   crate layering BEFORE booking this test's mechanism, and if the layering
   forbids it, record the deviation: the test asserts only
   `Solid::try_new` re-validation plus tessellation-free face/wire census
   instead. The downstream-consumability invariant is re-asserted at the P8
   battery either way.)

## H-3 (house rule; V4 is a text gate on your diff)

No ADDED line carries a bare absolute small literal (`1e-N`) without the
same-line `// H-3` opt-out, and there should be none: dyadic constants,
`FRAC_PI_2`/`TAU`, and geometry-derived values only. Run
`& "C:\Program Files\Git\bin\bash.exe" scripts/kernel-gates.sh HEAD` before
writing RESULT.json (bare `bash` is the WSL stub).

## Done when

Commit on the current branch (subject
`BG-CAD-P5-REVOLVE: revolve of line-edge profiles via the carrier table`)
BEFORE writing RESULT.json, then, all green:

```
cargo check --locked -p truck-modeling
cargo fmt --check -p truck-modeling
cargo test --locked -p truck-modeling --lib
cargo test --locked -p truck-modeling --test revolve_p5
cargo clippy --locked -p truck-modeling --all-targets
& "C:\Program Files\Git\bin\bash.exe" scripts/kernel-gates.sh HEAD
```

## Forbidden

- Do not edit `extrude.rs`, `cad.rs`, `arrange.rs`, `recognize.rs` beyond the
  single D6 arm, or anything outside `write_allow`.
- Do not add `Refusal`/`EnvelopeCase`/`UnresolvedWitness`/`Collapse` arms
  (zero-new-arms program rule; a perceived need is a SPEC_GAP).
- Do not attempt arbitrary-axis revolve, axis-touch construction, or circle
  profiles (D2 boundaries; all are booked follow-ups).
- GATE-3/H-4: `Face::debug_new` is banned in added lines.
- Do not write the tolerance-migration constructor name in prose or comments
  (GATE-4 counts raw text).

## Stop conditions

- `ANCHOR_MISMATCH` — report the measured count, change nothing.
- `SPEC_GAP` — a decision contradicts the tree; QUESTION.md with the
  empirical proof. (Likely candidates: the Cone carrier's parameterization
  convention, the partial-angle arc construction recipe, the D6 witness
  vocabulary — the first two are derive-from-the-tree tasks, the third has
  the pre-made fallback.)
- `Solid::try_new` refuses a booked happy-path case (tests 1-4) after a
  D3/D4-faithful construction — stop and report the closure witness
  verbatim; that is a finding about the landed recipes, not something to
  route around.

RESULT.json: `{"id":"BG-CAD-P5-REVOLVE","status":"DONE","contracts":[...],
"tests_added":9,"deviations":[...],"notes":"..."}`
— every deviation with your derivation; deviations are expected to be RIGHT.
