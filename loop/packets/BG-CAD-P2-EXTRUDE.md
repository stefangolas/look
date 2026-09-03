---
id: BG-CAD-P2-EXTRUDE
class: design
crates: [truck-modeling]
write_allow:
  - vendor/truck/truck-modeling/src/extrude.rs
tests_required:
  - vector_z_matches_scalar_extrude
  - oblique_extrude_of_polygon_is_planar_sided
  - both_extrude_is_symmetric_interval
  - oblique_circle_refuses_noncanonical
  - taper_rectangle_top_is_offset
  - taper_circle_side_is_canonical_cone
  - taper_topology_event_refuses_collapsed
  - taper_hole_grows
  - zero_height_vector_refuses_empty
  - negative_taper_expands_material
budget: {turns: 45, ctx_tokens: 130000}
---

# BG-CAD-P2-EXTRUDE — Phase 7 generalized extrusion (vector / both / taper)

Program: `docs/BUILD123D_COVERAGE_PLAN.md` P2 (Tier 0). Everything below is
pre-decided; churn, don't design. Contradiction with the tree = `SPEC_GAP`.

## Problem

The landed `extrude_profile(profile, arrangement, height)` extrudes along +z
only, one-sided, no draft. build123d's `extrude` takes a direction vector,
`both=True`, and `taper=...`. This packet generalizes the landed entry in
place, keeping every emitted carrier inside the canonical set (downstream
Boolean/contact consumable). It stays entirely inside `extrude.rs` — another
packet (BG-CAD-P1-UTILITY) is editing `lib.rs` concurrently; touching lib.rs
is a V1 rejection of a packet you did not write.

## Anchors (measured 2026-08-28; re-derive before writing RESULT.json)

| id | file | pattern | count |
|----|------|---------|-------|
| A1 | vendor/truck/truck-modeling/src/extrude.rs | `#[test]` | 6 |
| A2 | vendor/truck/truck-modeling/src/extrude.rs | `pub fn extrude_profile` | 1 |
| A3 | vendor/truck/truck-geometry/src/arrange.rs | `pub fn arrange` | 1 |

A1 becomes 6 + your added tests; that divergence is expected, not a mismatch.

## Decisions already made for you

**D0 — the landed entry is frozen.** `extrude_profile(profile, arrangement,
height)` keeps its exact signature and behavior (the M2 flagship battery
depends on it; V5 fails on any base-test regression). You may refactor shared
internals beneath it as long as every landed test stays green.

**D1 — internal generalized form.**

```rust
fn extrude_interval(
    profile: &[Curve], arrangement: &Arrangement,
    base: Vector3, tip: Vector3, taper: f64,
) -> Outcome<Solid>
```

`base`/`tip` are translation offsets of the z = 0 profile. Bottom cap on the
plane z = `base.z`, top cap on z = `tip.z` (the cap recipe is the landed one:
closed circles via `Edge::new_unchecked`, explicit cap wires, annulus carries
two boundary wires and no seam edges — all already in this file). Side faces:

- `Line` edge → the plane through the bottom edge and the translated top
  edge — spanned by (b − a) and the sweep vector, hence always a canonical
  `Plane` in ANY direction.
- `Circle` edge → canonical `Cylinder` when sweep ∥ z and `taper == 0`
  (the landed recipe); canonical z-aligned `Cone` when sweep ∥ z and
  `taper != 0` (apex on the axis, derived from the bottom circle (r at
  z₀) and top circle (r′ at z₁)); sweep NOT ∥ z with any circle edge →
  `UnsupportedEnvelope(NonCanonicalCarrier)` (an oblique cylinder is a
  `Placed` carrier — Tier 1's unlock, not ours).

**D2 — public vector entry.**

```rust
pub fn extrude_profile_vector(
    profile: &[Curve], arrangement: &Arrangement, dir: Vector3, both: bool,
) -> Outcome<Solid>
```

- `dir.z == 0` → `Refusal::Empty` (a z = 0 profile swept within its own plane
  has zero volume; the extrude.rs non-positive-height convention).
- `both == false` → interval `[0, dir]`; `both == true` → `[−dir, +dir]`
  (build123d's `both` semantics: the same amount each way).
- Circle edges + non-z-parallel `dir` → `NonCanonicalCarrier` (D1).

**D3 — public taper entry.**

```rust
pub fn extrude_profile_taper(
    profile: &[Curve], arrangement: &Arrangement, height: f64, taper: f64,
) -> Outcome<Solid>
```

- `height <= 0` or non-finite → `Refusal::Empty` (landed convention).
- `|taper| >= pi/2` or non-finite → `Refusal::Empty` (tan undefined /
  sign-flipped; do NOT write a bare `1e-N`-adjacent literal — see H-3).
- Sweep is ∥ +z by definition here; `taper != 0` combined with an oblique
  direction is Tier 1 — this entry takes no direction parameter at all.
- Signed offset `d = height * tan(taper)`: `taper > 0` shrinks the material
  (build123d's positive-taper draft), `taper < 0` grows it.

**D4 — the top profile is the 2-D offset, re-arranged (the parsimony move).**

Do NOT hand-construct the top polygon. Offset the profile curves and re-run
the landed `arrange`:

- Line edge on an OUTER material cycle → translate by `d` toward the material
  (left normal of the CCW cycle), then EXTEND both endpoints by `2|d|` along
  the segment direction (guarantees corner coverage at any convex angle).
  Line edge on a HOLE cycle → translate by `d` away from the hole's interior
  (the hole grows; the material still shrinks), same extension.
- Circle edge, outer → radius `r − d`; hole → radius `r + d`. `r − d <= 0`
  → `Refusal::Collapsed(..)` (hole/outer collapse is a topology event).
- Cycle role (outer vs hole) comes from the landed containment rule
  (`select_material`'s region logic in this file), never from winding sign
  (session-28 trap: S1 normalizes every loop to CCW, so winding cannot
  distinguish a hole from its plate).
- Re-arrange the offset curves. The top cap's material region structure
  (region count AND nesting) must equal the bottom's; any difference is a
  topology event → `Refusal::Collapsed(..)`. A zero-area / inverted top
  region → `Collapsed` likewise.
- Side-face correspondence: a top boundary edge lying on the offset carrier
  of bottom edge i pairs with bottom edge i (carrier identity, not index
  luck). Line → plane through both segments; circle → z-aligned `Cone` (D1).

**D5 — certificates.** After construction: `Solid::try_new` is the acceptance
gate (a refusal there is a typed refusal, never a panic); every emitted
carrier must be recognized by `recognize_surface`/`recognize_curve` —
anything `Unrecognized` refuses `UnsupportedEnvelope(NonCanonicalCarrier)`
(defensive; the D1 table cannot produce it).

Out of scope, recorded: `mode` (ADD/SUBTRACT/...) is facade-level composition
over the landed `BoolOp` — Phase 7 P8's business, not this packet's. `clean`
is a no-op here (this construction never leaves degenerate edges).

## Template

This same file IS the template: `extrude_profile` (lines 52+), the material
selection and cap recipe, and the in-src test module (6 tests, the house
`#[test]` + dyadic-witness style). Read the whole file before writing.
`truck-geometry/src/recognize.rs` is the recognizer you assert against.

## Tests required (in-src, extending the existing module — A1 grows)

All witnesses dyadic; no bare `1e-N` literals (H-3); `f64::atan(0.5)` and
friends are fine (computed, not literal).

1. `vector_z_matches_scalar_extrude` — `extrude_profile_vector` with
   `dir = (0, 0, h)` produces a solid congruent to the landed scalar entry:
   same face count, same bounding corners, `Solid::try_new` ok both.
2. `oblique_extrude_of_polygon_is_planar_sided` — line-only rectangle,
   `dir = (1, 0, 1)`: 6 faces, every side carrier recognizes to `Plane`,
   try_new ok, top cap on z = 1 translated by the sweep.
3. `both_extrude_is_symmetric_interval` — `both = true`, rect: box
   z ∈ `[−h, +h]` exactly, 6 faces.
4. `oblique_circle_refuses_noncanonical` — circle profile,
   `dir = (1, 0, 1)` → `UnsupportedEnvelope` (machine-check the arm is
   `NonCanonicalCarrier`).
5. `taper_rectangle_top_is_offset` — rect 4×4, h = 1, taper with
   tan = 0.5: top cap is the 0.5-inset rectangle (exact corners), 4 side
   planes, try_new ok.
6. `taper_circle_side_is_canonical_cone` — circle r = 1, h = 2, taper with
   tan = 0.25: the side face's surface recognizes to `Cone`, top radius
   0.5, try_new ok.
7. `taper_topology_event_refuses_collapsed` — taper so deep the inset
   collapses (d >= 2 on the 4-wide rect) → `Collapsed`.
8. `taper_hole_grows` — rectangle-minus-circle profile, positive taper:
   top cap has 2 wires and the hole's top radius is r + d; try_new ok.
9. `zero_height_vector_refuses_empty` — `dir = (3, 0, 0)` → `Empty`.
10. `negative_taper_expands_material` — negative taper on the rect: top cap
    is the 0.5-outset rectangle, try_new ok.

## H-3 (house rule; V4 is a text gate on your diff)

No ADDED line may carry a bare absolute small literal (`1e-N`) without the
same-line opt-out `// H-3`, and there should be none: dyadic constants and
computed values (`f64::atan(0.5)`, `height * taper.tan()`) only. Run
`& "C:\Program Files\Git\bin\bash.exe" scripts/kernel-gates.sh HEAD` before
writing RESULT.json (bare `bash` is the WSL stub).

## Done when

Commit on the current branch (subject
`BG-CAD-P2-EXTRUDE: generalized extrusion — vector, both, taper`) BEFORE
writing RESULT.json, then, all green:

```
cargo check --locked -p truck-modeling
cargo fmt --check -p truck-modeling
cargo test --locked -p truck-modeling --lib
cargo clippy --locked -p truck-modeling --all-targets
& "C:\Program Files\Git\bin\bash.exe" scripts/kernel-gates.sh HEAD
```

## Forbidden

- Do not touch `lib.rs`, `cad.rs`, or any file outside `write_allow` —
  BG-CAD-P1-UTILITY owns lib.rs concurrently (V1 would reject its merge, not
  yours, and then yours).
- Do not change `extrude_profile`'s signature or its landed tests' assertions.
- Do not add `Refusal`/`EnvelopeCase`/`UnresolvedWitness` arms (zero-new-arms
  program rule; a perceived need is a SPEC_GAP).
- Do not emit `Placed` carriers (oblique circles refuse, per D1).
- GATE-3/H-4: `Face::debug_new` is banned in added lines.
- Do not write the tolerance-migration constructor name in prose or comments
  (GATE-4 counts raw text).

## Stop conditions

- `ANCHOR_MISMATCH` — report the measured count, change nothing.
- `SPEC_GAP` — a decision contradicts the tree; QUESTION.md with the
  empirical proof.
- The offset-and-rearrange construction (D4) cannot express a build123d taper
  case you can prove is in-scope — stop and report rather than hand-building
  the top polygon.

RESULT.json: `{"id":"BG-CAD-P2-EXTRUDE","status":"DONE",
"contracts":[...],"tests_added":10,"deviations":[...],"notes":"..."}` —
every deviation recorded with your derivation; deviations are expected to be
RIGHT.
