---
id: BG-CAD-P4-UNTIL
class: design
crates: [truck-modeling]
write_allow:
  - vendor/truck/truck-modeling/src/until.rs
  - vendor/truck/truck-modeling/src/lib.rs
  - vendor/truck/truck-modeling/tests/until_p4.rs
tests_required:
  - until_parallel_target_metamorphic
  - until_oblique_rectangle_prism
  - until_oblique_cap_in_target_plane
  - until_misses_refuses
  - until_parallel_sweep_refuses
  - until_nonconvex_refuses
  - until_circle_profile_oblique_refuses
  - project_parallel_is_translation
  - project_oblique_lines
budget: {turns: 45, ctx_tokens: 130000}
---

# BG-CAD-P4-UNTIL — Phase 7 sweep reduction: until + project

Program: `docs/BUILD123D_COVERAGE_PLAN.md` P4 (Tier 0), curtain table 6.1
and the metamorphic gate in §9. Everything below is pre-decided; churn, don't
design. Contradiction with the tree = `SPEC_GAP`.

## Problem

build123d's `extrude(until=...)` and `project` are the next Tier 0
operations. The plan's decomposition: `until/project = swept Contact +
certified-t ordering + rewrite`. The parsimony identity says no new solving
machinery is needed: the swept curtain of a line/circle profile is canonical
(table 6.1), the landed exact FF arms answer curtain × target-plane pairs
(lines for plane walls), and the termination is a closed-form rewrite.

## Anchors (measured 2026-08-28 at HEAD `8111be9`; re-derive before writing RESULT.json)

| id | file | pattern | count |
|----|------|---------|-------|
| A1 | vendor/truck/truck-modeling/src/lib.rs | `pub mod cad` | 1 |
| A2 | vendor/truck/truck-modeling/src/lib.rs | `pub mod until` | 0 |
| A3 | vendor/truck/truck-modeling/src/extrude.rs | `pub fn extrude_profile_vector\(` | 1 |
| A4 | vendor/truck/truck-modeling/src/cad.rs | `pub fn solid_bounding_box\(` | 1 |
| A5 | vendor/truck/truck-modeling/Cargo.toml | `truck-evidence` | 1 |

A2 becomes 1 once you declare `pub mod until;` (expected divergence, not a
mismatch). A5 is the manifest edge that lets you call the landed
`contact()`; it is already present — do not touch Cargo.toml.

## Decisions already made for you

**D1 — module shape.** New file `vendor/truck/truck-modeling/src/until.rs`,
declared `pub mod until;` in lib.rs (place so
`cargo fmt --check -p truck-modeling` passes). House deny header copied from
`truck-geometry/src/recognize.rs:22-29`. New test file
`vendor/truck/truck-modeling/tests/until_p4.rs`, same header, the
`tests/cad_p1.rs` helper pattern.

**D2 — signatures.**

```rust
/// The certified sweep target. v1: planes only.
pub enum Until {
    Plane(Plane),
}

pub fn extrude_until(
    profile: &[Curve],
    arrangement: &Arrangement,
    dir: Vector3,
    target: &Until,
) -> Outcome<Solid>;

pub fn project_profile(
    profile: &[Curve],
    arrangement: &Arrangement,
    dir: Vector3,
    target: &Until,
) -> Outcome<Vec<Curve>>;
```

`Until` is a NEW enum in YOUR module — the zero-new-arms rule bans new arms
on the landed `Refusal`/`EnvelopeCase`/`UnresolvedWitness`/`Collapse` types,
not new module-local types. One variant only; more are booked follow-ups.

**D3 — the certified-t ordering.** Frame conventions are the extrude
family's: profile material region(s) in the z = 0 plane, one material region
(`Empty` otherwise, mirroring `extrude_interval`'s guard). Let Π be the
target plane (origin o, unit norm n — derive the exact accessor convention
from `truck-geometry`'s Plane).

- `n · dir == 0` (or non-finite inputs, `dir.z == 0` per the landed vector
  convention) → `Refusal::Empty`: the sweep never terminates on Π.
- The signed crossing parameter of a point p is
  `t(p) = (n·o − n·p) / (n·dir)`. The sweep terminates where the ENTIRE
  leading front has crossed: the truncated solid is the sweep over
  t ∈ [0, t(p)] pointwise — equivalently the prism cut by the halfspace
  {x : n·x ≤ n·o} when `n·dir > 0` (mirror the sense when negative; machine-
  check the two cases and record the derivation).
- If NO boundary point of the region crosses along dir in the positive
  direction (Π behind the profile: every t(p) < 0), refuse `Refusal::Empty`
  — there is no termination.
- Machine-check every witness t against the closed-form formula; BG-NUM-002
  applies to the t values exactly as to geometry.

**D4 — the termination construction (no boolean, no new solver math).**

- **Parallel target (n ∥ dir, the §9 metamorphic case):** t* is uniform;
  the solid is exactly the landed `extrude_profile_vector(profile,
  arrangement, t*·dir, false)` construction. Construct it THAT way (call
  the landed entry with the certified height component) — the metamorphic
  identity is then structural, and test 1 certifies it against a direct
  call.
- **Oblique target, line-edge convex region:** every curtain wall is a
  Plane (table 6.1 row 1), so every wall × Π termination locus is a Line
  (the landed `plane_plane` exact arm — call the landed `contact()` from
  truck-evidence to certify each termination line; A5). The oblique cap is
  the planar polygon in Π bounded by the termination lines of consecutive
  walls; its vertices are the pairwise intersections of consecutive
  termination lines (machine-check each vertex lies on BOTH lines and in
  Π). Build the cap face in Π's own frame: map Π to a local z = 0 frame
  (rigid transform), construct the face with the house recipes, map back —
  the bottom cap is the profile region's face (extrude recipe), walls are
  the curtain Planes each trimmed at its termination line, `Solid::try_new`
  is the gate.
- **Refusals (typed, zero new arms):**
  - non-convex region boundary (machine-check convexity from the
    arrangement's polygon — a reflex vertex anywhere) refuses
    `UnsupportedEnvelope(NonCanonicalCarrier)`: the cap polygon's
    region-structure is not v1.
  - a Circle edge in the region boundary with an OBLIQUE target refuses
    `UnsupportedEnvelope(NonCanonicalCarrier)` at the lift: the curtain is
    a Cylinder and the termination an Ellipse — the RW-CONIC boundary (the
    booked follow-up is a Curve-enum ellipse arm). With a PARALLEL target a
    circle profile is FINE (D4 parallel case rides the landed extrude,
    which already handles circle walls).
- **`project_profile`** returns the projected boundary of the region onto Π
  along dir: the same termination loci, as curves. Parallel target →
  profile translated by t*·dir (lines and circles alike — translation
  preserves the Curve type). Oblique → each Line edge maps to the Line
  between its endpoints' images (closed form); a Circle edge refuses as
  above. The returned curves' carriers must be canonical (`Line`/`Circle`
  only) — that IS the refusal rule, not a post-check.

**D5 — certificates.** `Solid::try_new` is the acceptance gate. The
metamorphic gates: test 1 (`extrudeUntil(P, Π) ≅ extrude(P, h_Π)` for the
parallel target — box + face equality against a direct
`extrude_profile_vector` call) and test 8 (parallel projection ≅
translation — curve-type and endpoint equality). Oblique certificates: the
cap face's Plane data equals Π's EXACTLY (construct both from the same
`Plane` value; compare by data, not tolerance), and every wall's termination
line came from the landed `contact()` (certified records, not raw
intersections).

## Template

- `vendor/truck/truck-modeling/src/extrude.rs` — the entry signature shape
  (A3), the arrangement region extraction, the wall/cap construction
  recipes, refusal conventions. Read before D4; do NOT edit.
- `vendor/truck/truck-modeling/src/cad.rs` — module shape, `make_face`
  (P1) for the planar-face construction discipline,
  `solid_bounding_box` (A4) for tests.
- `vendor/truck/truck-evidence` `contact()` — the landed exact FF arms your
  termination lines ride (`analytic/plane_plane.rs`); call it, do not
  reimplement the locus math.
- `vendor/truck/truck-modeling/tests/cad_p1.rs` — the test-file pattern.

## Tests required (new file `tests/until_p4.rs`, dyadic witnesses only)

1. `until_parallel_target_metamorphic` — square [1,3]², dir (0,0,2),
   target z = 2: the solid is face-count- and box-equal to a direct
   `extrude_profile_vector` call with the same arguments.
2. `until_oblique_rectangle_prism` — square [1,3]², dir (0,0,2), target
   plane through (0,0,2) with norm (1,0,1)/√2 (build from three exact
   dyadic points; compare planes by data): valid solid; every face's
   carrier is `Plane`; face count 5 (bottom, 4 walls) plus the oblique cap
   = 6 total, machine-check the count you construct.
3. `until_oblique_cap_in_target_plane` — same fixture: exactly one face
   whose Plane data equals the target's exactly; its box is the cap
   polygon's (machine-check the expected polygon vertices from D4's
   formulas).
4. `until_misses_refuses` — target z = −1 (behind the profile along +z) →
   `Refusal::Empty`.
5. `until_parallel_sweep_refuses` — target plane x = 5 (n ⊥ dir) →
   `Refusal::Empty`.
6. `until_nonconvex_refuses` — an L-shaped region (6-edge polygon) →
   `UnsupportedEnvelope(NonCanonicalCarrier)`, machine-check the convexity
   predicate actually fires on YOUR region extraction.
7. `until_circle_profile_oblique_refuses` — circle profile r=2, oblique
   target → `UnsupportedEnvelope(NonCanonicalCarrier)`; the same circle
   profile with a PARALLEL target assembles (assert both in one test).
8. `project_parallel_is_translation` — square profile, parallel target:
   returned curves are 4 `Line`s whose endpoints are the profile's endpoints
   translated by (0,0,2) exactly.
9. `project_oblique_lines` — square profile, oblique target as in test 2:
   returned curves are 4 `Line`s, each endpoint certified on Π (the plane
   equation holds exactly at dyadic points).

## H-3 (house rule; V4 is a text gate on your diff)

No ADDED line carries a bare absolute small literal (`1e-N`) without the
same-line `// H-3` opt-out, and there should be none: dyadic constants,
geometry-derived t values, and named consts only. The √2-style
normalization lives in the carrier (compare planes by data, never by unit
length). Run `& "C:\Program Files\Git\bin\bash.exe" scripts/kernel-gates.sh
HEAD` before writing RESULT.json (bare `bash` is the WSL stub).

## Done when

Commit on the current branch (subject
`BG-CAD-P4-UNTIL: extrude-until-plane + project via the curtain table`)
BEFORE writing RESULT.json, then, all green:

```
cargo check --locked -p truck-modeling
cargo fmt --check -p truck-modeling
cargo test --locked -p truck-modeling --lib
cargo test --locked -p truck-modeling --test until_p4
cargo clippy --locked -p truck-modeling --all-targets
& "C:\Program Files\Git\bin\bash.exe" scripts/kernel-gates.sh HEAD
```

## Forbidden

- Do not edit `extrude.rs`, `cad.rs`, `arrange.rs`, `Cargo.toml`, or
  anything outside `write_allow`.
- Do not add `Refusal`/`EnvelopeCase`/`UnresolvedWitness`/`Collapse` arms
  (zero-new-arms program rule; a perceived need is a SPEC_GAP). `Until`
  itself is a new module-local enum — allowed (D2).
- Do not attempt circle-profile oblique termination, non-convex caps, or
  non-plane targets (D4 boundaries; all are booked follow-ups).
- GATE-3/H-4: `Face::debug_new` is banned in added lines.
- Do not write the tolerance-migration constructor name in prose or comments
  (GATE-4 counts raw text).

## Stop conditions

- `ANCHOR_MISMATCH` — report the measured count, change nothing.
- `SPEC_GAP` — a decision contradicts the tree; QUESTION.md with the
  empirical proof. (Likely candidates: the Plane accessor/data convention,
  the landed `contact()` call signature from outside truck-evidence, the
  cap-face construction in a non-z frame — all derive-from-the-tree tasks.)
- A booked happy-path case (tests 1-3) fails `Solid::try_new` after a
  D4-faithful construction — stop and report the closure witness verbatim.

RESULT.json: `{"id":"BG-CAD-P4-UNTIL","status":"DONE","contracts":[...],
"tests_added":9,"deviations":[...],"notes":"..."}` — every deviation with
your derivation; deviations are expected to be RIGHT.
