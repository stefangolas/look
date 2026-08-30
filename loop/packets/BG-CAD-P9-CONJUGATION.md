---
id: BG-CAD-P9-CONJUGATION
class: design
crates: [truck-evidence]
write_allow:
  - vendor/truck/truck-evidence/src/contact/mod.rs
  - vendor/truck/truck-evidence/tests/conjugation.rs
tests_required:
  - conjugation_placed_intersecting_axes_two_ellipses
  - conjugation_identity_placement_equals_bare
  - conjugation_parallel_placed_pair_folds
  - conjugation_metamorphic_rigid
  - conjugation_skew_pair_defers
  - conjugation_unequal_radii_defers
  - conjugation_noncylinder_placed_defers
  - conjugation_nonuniform_scale_refuses
budget: {turns: 45, ctx_tokens: 140000}
---

# BG-CAD-P9-CONJUGATION — relative-frame canonicalization in the Contact dispatch

Program: `docs/BUILD123D_COVERAGE_PLAN.md` P9 (Tier 1): the plan's §2 rule —
**frame conjugation is a Contact normalization rule, not a fifth primitive**.
The dispatch admits `Placed` carriers that conjugate (or fold) to supported
relative configurations, and the landed-but-unreachable
`equal_radius_cylinders` cell becomes reachable for the first time. The
pre-dispatch probe on the conjugation precondition's refusal boundaries is
DONE and PASSED (its evidence is QUOTED here; the probe source is untracked
scratch). Everything below is pre-decided; churn, don't design.
Contradiction with the tree = `SPEC_GAP`.

## Problem

Every canonical curved carrier is z-axis-aligned, so the landed dispatch's
curved × curved cells are parallel-axis only, and the `(Placed, _)` arm
(`contact/mod.rs:522`) refuses ALL placements with
`ContactReductionDeferred`. Two placed z-aligned cylinders whose WORLD axes
intersect at an angle are the classic exact cell — two ellipses, rational,
no iteration (`equal_radius_cylinders.rs:60`, landed and tested, imported
NOWHERE — the dispatcher imports only `coaxial` + `parallel_cylinders`).
The normalization: extract each side's world pose from its placement,
classify the relative configuration, and route to the axis-explicit cell
(equal radii) or the fold path (parallel axes), leaving everything else
exactly as deferred as it is today.

## The probe evidence (quoted; every number machine-measured)

**W1 — the state at HEAD.** `contact()` on two Placed-cylinder face strata
(equal radius 1, world axes x̂ and ŷ through the origin, placements
`rotY(90°)` and `rotX(−90°)` of canonical z-axis cylinders) refuses with
exactly `UnsupportedEnvelope(ContactReductionDeferred)` at the Placed arm.
So does `Placed(identity) × bare`.

**W2 — the conjugation certificate.** Extracting the world poses (axis
foot = `M·center`, axis dir = `M·ẑ` normalized) and calling the LANDED
`equal_radius_cylinders(1.0, ((0,0,0), ẑ), &((0,0,0), conjugated_dir))`
yields the two ellipses; transforming sampled ellipse points back through
the left placement `M0`, 26/26 sampled points lie on BOTH original
world-placed cylinders (axis distance = 1 exactly at unit scale, residual
< 1e-9). The analytic cell is frame-free — its axes are arguments — so
world-frame poses route to it DIRECTLY, with no conjugation-back of the
emitted loci.

**W3 — the fold is a C1 PARAMETER-MAP equivalence, not a raw carrier
equality.** A placement that is translation + rotation-about-z + uniform
scale maps a canonical cylinder to a canonical cylinder AS A POINT SET:
`M·cyl.subs(u, v) == recon.subs(u + θ, s·v)` with θ the placement's
z-rotation and s its uniform scale (machine-checked at three sample
points, residual < 1e-9; a RAW subs comparison FAILS — placed
(4.393, −0.565, 7.2) vs naive recon (4.911, −1.409, 6.1)). Therefore the
u/v parameter boxes MUST ride the map when a folded carrier is routed
onward: `u' = (u0 + θ, u1 + θ)`, `v' = (v0·s + t_z, v1·s + t_z)` where
`t_z` is the axis foot's z. A tilted placement's axis is decisively not
z-parallel (z-cross = 1.0 on the witness) — the fold condition is exactly
"axis ∥ ẑ".

**W4 — the refusal boundaries.** `equal_radius_cylinders` refuses a skew
(non-coplanar) equal-radius pair with
`UnsupportedEnvelope(NonCanonicalCarrier)` (its own landed contract) — the
dispatcher MAPS that to `ContactReductionDeferred` (the §7 mapping:
"deferred pair (…, skew cylinders) → ContactReductionDeferred"); the
dispatcher's own parallel pre-screen means the parallel refusal arm cannot
fire. The bare parallel-offset pair (axis distance 3, r = 1, v ∈ [0, 2])
answers a certified-EMPTY complex — the metamorphic baseline. A
`Placed(identity) × bare` pair refuses deferred at HEAD (the fold path's
input state).

## Anchors (measured 2026-08-29 at HEAD `abb42ef`; re-derive before writing RESULT.json)

| id | file | pattern | count |
|----|------|---------|-------|
| A1 | vendor/truck/truck-evidence/src/contact/mod.rs | `\(CanonicalSurface::Placed\(_\), _\) \| \(_, CanonicalSurface::Placed\(_\)\)` | 1 |
| A2 | vendor/truck/truck-evidence/src/contact/mod.rs | `equal_radius_cylinders` | 0 |
| A3 | vendor/truck/truck-evidence/src/analytic/equal_radius_cylinders.rs | `pub fn equal_radius_cylinders\(` | 1 |
| A4 | vendor/truck/truck-evidence/src/contact/mod.rs | `fn analytic_ff\(` | 1 |
| A5 | vendor/truck/truck-evidence/tests | `conjugation.rs` | 0 (new file) |

A2 becomes ≥ 1 once you import the cell (expected divergence, not a
mismatch). All other anchors are the PRE-packet tree.

## Decisions already made for you

**D1 — the normalization lives IN the `(Placed, _)` arm of `analytic_ff`
(A4), replacing the unconditional refusal.** No new module, no new stage
function in the doc-comment's stage list (the arm's own doc-comment line
5 "Everything else" keeps its shape — the Placed family simply stops being
unconditionally deferred). The curve strata (FE/EE) and the C0-C2 identity
screens are UNCHANGED: v1 books FACE pairs only; a placed CURVE family
keeps the deferred refusal (the landed `CanonicalCurve` has no placement
arm at all — nothing to normalize).

**D2 — the v1 carrier family is CYLINDERS (both sides).** If either
placed side's inner surface is not `CanonicalSurface::Cylinder`, the arm
refuses `ContactReductionDeferred` exactly as today (other families are
booked follow-ups). A bare side is treated as its own canonical pose
(foot = its center, dir = ẑ, radius r, identity parameter map).

**D3 — the pose extraction (the probe's W2 recipe).** For a
`Placed(inner, m)` cylinder: axis foot = `m.transform_point(inner.center())`;
axis dir = the normalized `m·ẑ`; scaled radius `r' = r·|m·ẑ|`. The
placement must be a PROPER SIMILARITY: `|m·x̂| = |m·ŷ| = |m·ẑ|` (decisive;
a violation is a non-uniform scale — an elliptical cross-section, a
non-canonical carrier → `UnsupportedEnvelope(NonCanonicalCarrier)`) and
`det(m) > 0` (an improper/mirror placement defers
`ContactReductionDeferred` — mirrors are P10's business, booked
follow-up). Predicate discipline: interval arithmetic on the extracted
components (the `equal_radius_cylinders.rs` pattern: per-component
interval products, decisive-zero / excludes-zero three-way), never naked
f64 comparisons on geometric predicates; an undecidable straddle refuses
`NumericallyUnresolved` with `RootNotIsolated` (the landed convention).

**D4 — the classification and routing (pre-decided).**

1. **Parallel axes** (the interval cross product of the two world dirs
   decisively zero): FOLD each placed side to its canonical form —
   `Cylinder::new((foot.x, foot.y, foot.z), r')` — and map its u/v boxes
   by the W3 parameter map (`u' = u + θ` with θ the placement's
   z-rotation of x̂, `v' = v·s + t_z`); then route the reconstructed
   (bare, bare) pair through the EXISTING cylinder arms (the coaxial
   screen, then `parallel_cylinders`) UNCHANGED. The reconstructed
   carriers' subs point sets equal the placed carriers' images exactly
   (W3); the parameter maps make the boxes bound the same world patches.
   A folded pair's records are WORLD geometry — no conjugation-back.
2. **Non-parallel, EQUAL radii** (exact f64 radius equality on the scaled
   radii — the `coaxial_axes` exactness convention): call the landed
   `equal_radius_cylinders(r0, &(foot0, dir0), &(foot1, dir1))` with the
   WORLD poses. Its outcome maps: `TwoCurves` ellipses → the existing
   `analytic_records` path (the ellipses are already world-frame placed
   circles — W2); its `NonCanonicalCarrier` refusal (skew; parallel
   cannot reach here) MAPS to `ContactReductionDeferred` (§7); its
   `NumericallyUnresolved` propagates as-is (a stop, not a guess).
3. **Non-parallel, unequal radii** → `ContactReductionDeferred` (the
   general solver's cell, unchanged).

**D5 — zero new arms.** No new `Refusal`/`EnvelopeCase`/
`UnresolvedWitness`/`Collapse` arms. The locus vocabulary is the landed
one (`TwoCurves` of `ExactCurve::Ellipse` rides the existing
`AnalyticIntersection` mapping). A perceived need is a SPEC_GAP.

**D6 — the metamorphic contract (the plan's §9 gate).**
`contact(A, B) ≅ contact(g·A, g·B)` for a rigid g: same record count, same
dimension/kind per record, and the g-image of the first answer's locus
points lies on the second answer's loci (point-level machine-check on the
ellipse sampling). The identity-placement case is the g = id special
case: `Placed(identity) ≡ bare` exactly (the fold path with θ = 0,
s = 1).

**D7 — certificates.** The analytic path's `Method::Exact` and
`AnalyticCarrier` prop ride as today (the folded parallel route and the
eqrcyl route both carry them). The eqrcyl cell's own certificate contract
(its module doc) is unchanged — the dispatcher maps outcomes, it does not
weaken them.

## Template

- `vendor/truck/truck-evidence/src/contact/mod.rs:390-555` — the landed
  `analytic_ff` you are amending (A4); the arm at :522 is the one you
  replace (A1).
- `vendor/truck/truck-evidence/src/analytic/equal_radius_cylinders.rs` —
  the axis-explicit cell (A3): read its module doc and its interval
  predicate pattern (`ival`, `cross_intervals`, `three_way`) before D3.
- `vendor/truck/truck-evidence/src/contact/mod.rs:433-438` — the landed
  cylinder-family arms the fold path re-enters (the coaxial screen +
  `parallel_cylinders`).
- `vendor/truck/truck-evidence/tests/torus_pairs.rs` — the test-file
  conventions (stratum construction, record extraction); read, do not
  edit.

## Tests required (new file `tests/conjugation.rs`, dyadic witnesses)

Fixture placements: `rotY(90°)` maps a canonical z-axis cylinder to world
axis x̂; `rotX(−90°)` maps to world axis ŷ (the probe's W1/W2 witnesses).

1. `conjugation_placed_intersecting_axes_two_ellipses` — the W2 witness
   THROUGH THE DISPATCHER: two placed cylinders (radius 1, world axes x̂
   and ŷ through the origin, u ∈ [0, TAU], v ∈ [0, 2]) answer records
   whose loci are the two ellipses; every sampled ellipse point lies on
   BOTH world-placed carriers (axis distance = 1, the probe's 26-point
   check at your achieved precision, recorded).
2. `conjugation_identity_placement_equals_bare` — `Placed(identity) ×
   bare` answers EXACTLY what `bare × bare` answers for the same pair
   (use the parallel-offset pair and a coaxial pair; assert record-level
   equality — the D6 g = id case).
3. `conjugation_parallel_placed_pair_folds` — the W3 witness: a
   translation + rotation-about-z + uniform-scale placed pair answers
   exactly what the corresponding bare pair answers (the fold path with
   mapped boxes; the certified-empty baseline and a coaxial pair both
   work — machine-check what you assert, record it).
4. `conjugation_metamorphic_rigid` — the D6 gate on the intersecting-axes
   pair under a rigid g (a rotation + translation): record count and
   kinds equal; the g-image of every first-answer ellipse sample lies on
   the second answer's ellipses.
5. `conjugation_skew_pair_defers` — the W4 mapping: skew equal-radius
   placed cylinders → `ContactReductionDeferred` (assert the typed arm).
6. `conjugation_unequal_radii_defers` — non-parallel placed cylinders
   with radii 1 and 2 → `ContactReductionDeferred`.
7. `conjugation_noncylinder_placed_defers` — a `Placed` sphere × bare
   cylinder (and a placed cylinder × bare plane) →
   `ContactReductionDeferred` (D2's family boundary; assert what you
   observe, record it).
8. `conjugation_nonuniform_scale_refuses` — a placed cylinder with
   `|m·x̂| ≠ |m·ẑ|` (e.g. scale (2,2,3)) →
   `UnsupportedEnvelope(NonCanonicalCarrier)` (D3's similarity screen).

## H-3 (house rule; V4 is a text gate on your diff)

No ADDED line carries a bare absolute small literal (`1e-N`) without the
same-line `// H-3` opt-out. Dyadic constants and geometry-derived values
only. Run `& "C:\Program Files\Git\bin\bash.exe" scripts/kernel-gates.sh
HEAD` before writing RESULT.json (bare `bash` is the WSL stub). CLIPPY
EVERY CHANGED FILE — run `cargo clippy --locked -p truck-evidence
--all-targets` UNFILTERED and fix all findings BEFORE committing (five
prior packets each lost verify rounds to partial clippy runs).

## Done when

Commit on the current branch (subject
`BG-CAD-P9-CONJUGATION: relative-frame canonicalization, equal_radius_cylinders reachable`)
BEFORE writing RESULT.json AT THE WORKTREE ROOT (that exact path — not
`loop/results/`), then, all green:

```
cargo check --locked -p truck-evidence
cargo fmt --check -p truck-evidence
cargo test --locked -p truck-evidence --lib
cargo test --locked -p truck-evidence --test conjugation
cargo test --locked -p truck-evidence --test torus_pairs
cargo test --locked -p truck-evidence --test plane_properties
cargo clippy --locked -p truck-evidence --all-targets
& "C:\Program Files\Git\bin\bash.exe" scripts/kernel-gates.sh HEAD
```

All landed `truck-evidence` suites (`torus_pairs`, `plane_properties`, the
lib suite) must pass UNCHANGED.

## Forbidden

- Do not edit `gff.rs`, `singular.rs`, `analytic/**` (the cells are
  certified machinery — the dispatcher maps, it does not amend), or
  anything outside `write_allow`.
- Do not add `Refusal`/`EnvelopeCase`/`UnresolvedWitness`/`Collapse` arms
  (D5).
- Do not attempt non-cylinder families, improper/mirror placements,
  placed CURVE strata, or general oblique-cylinder cells (D2/D7
  boundaries; booked follow-ups).
- GATE-3/H-4: `Face::debug_new` is banned in added lines.
- Do not write the tolerance-migration constructor name in prose or
  comments (GATE-4 counts raw text, comments included).
- No instrumentation traces may survive in the committed diff.

## Stop conditions

- `ANCHOR_MISMATCH` — report the measured count, change nothing.
- `SPEC_GAP` — a decision contradicts the tree; QUESTION.md with the
  empirical proof.
- The fold path's reconstructed pair does not answer what the
  corresponding bare pair answers (D4.1 / tests 2-3) — stop and report
  the divergence with the placed and bare outcomes verbatim.
- The eqrcyl route's ellipse records fail the on-both-carriers
  machine-check (the probe's W2 is the reproducibility witness) — stop
  and report your extraction's divergence.

RESULT.json: `{"id":"BG-CAD-P9-CONJUGATION","status":"DONE",
"contracts":[...],"tests_added":8,"deviations":[...],"notes":"..."}`
— the D4 routing-choice notes and the θ/s/t derivation go in notes; every
deviation with your derivation; deviations are expected to be RIGHT.
