---
id: BG-CAD-P11-TORUS
class: design
crates: [truck-evidence]
write_allow:
  - vendor/truck/truck-evidence/src/contact/implicit.rs
  - vendor/truck/truck-evidence/src/contact/mod.rs
  - vendor/truck/truck-evidence/tests/torus_pairs.rs
tests_required:
  - torus_plane_axial_two_circles
  - torus_plane_oblique_loop
  - torus_plane_miss_proves_empty
  - torus_degenerate_family_lift_refuses
  - torus_equator_tangency_refuses
  - torus_torus_identical_carrier_screen
  - torus_quadric_offset_pair_still_green
budget: {turns: 45, ctx_tokens: 140000}
---

# BG-CAD-P11-TORUS — torus FF pairs through the landed validated-FF stage

Program: `docs/BUILD123D_COVERAGE_PLAN.md` P11 (Tier 2). The pre-dispatch
num3-scratch probe is DONE (this packet carries its complete evidence chain;
the probe source is untracked scratch — everything load-bearing is QUOTED
here). The plan rated this packet 7-8/10 expecting new solver math; the
probe found the solver chain ALREADY LANDED — the packet is a re-enclosure
fix, a dispatch-arm flip, and closed-form tests. Everything below is
pre-decided; churn, don't design. Contradiction with the tree = `SPEC_GAP`.

## Problem

`contact()` refuses every `(Torus, X)` pair at the dispatch arm
(`contact/mod.rs:484-491`, `ContactReductionDeferred`). The probe found the
entire downstream machinery landed and generic: `Torus: ImplicitField`
(implicit.rs:239), `Torus: EnclosureSurface` (torus.rs:63), and
`validated_ff` (mod.rs:684) — which composes `gff::cover_branch` +
`singular::singular_events` and emits the landed `ValidatedBranchCover`
locus — already certifies the offset quadric pairs. The probe then found
the ONE genuine blocker and its fix:

## The probe evidence (quoted; every number machine-measured)

**Finding 1 — the landed `Torus::grad` has fatal interval dependency.** The
sqrt-free quartic form computes `grad = 4g·x' − 8R²·x'` as TWO SEPARATE
interval products (`two·g·(two·dx) − scale·(two·dx)`): on a box cradling
the closed-form contact circle (x,y ∈ [1.5,2]², the outer circle r̂=2.433),
the subtraction spans zero ([49.7, 94.7] − [48, 64] = [−14.3, 46.7]) even
though the mathematical value is one-signed ([1.74, 30.7]). Every chart's
2×2 minor contains zero → `select_chart` returns None → the whole domain
lands in `singular_boxes` → the singular stage refuses
(`NumericallyUnresolved(KrawczykIndeterminate)` after 9857 subdivisions).
**The dependency is fatal at every scale for boxes straddling the torus's
own gradient sign structure.**

**Finding 2 — the sqrt-form re-enclosure fixes it completely.** The SAME
implicit function re-enclosed with `r̂ = sqrt(x'²+y'²)` computed ONCE:
`grad = (2(r̂−R)·x'/r̂, 2(r̂−R)·y'/r̂, 2z')`. The probe's probe-local field
certified 8/8 boxes (one Krawczyk-Unique crossing per slab mid-plane, 0
singular, 0 unresolved), every point machine-checked against the closed
form (plane residual < 1e-9, torus residual < 1e-9, radii 2 ±
sqrt(0.1875) exact).

**Finding 3 — the equator band `r̂ = R` is a chart-degenerate locus.** The
torus grad's x,y components vanish ON the equator ring, so ANY box whose
r̂-range straddles R fails every chart, at every subdivision depth
(`cover_branch`'s chart is fixed per input cell — decision 2 — and the
slab worklist never re-charts). The axial witness's z-slab [0.15,0.35]
prunes band cells by the torus enclosure alone (f = z²−r² ≠ 0 there); the
oblique witness x+z=1.5 GRAZES the band tangentially at (2,0,−0.5) — a
genuine singular contact, correctly NOT certified.

**Finding 4 — the landed singular stage returns SPURIOUS points on
equator-band boxes.** Fed the band's singular boxes, `singular_events`
recovered points violating the plane constraint by ±3.67 (printed verbatim
in the probe: 8 points at r̂ ≈ 2.13, torus residual ~1e-3, plane residual
−3.5 to −3.7). These are unreachable through the v1 entry IF the band
never reaches `singular_events` (D3's pre-split guarantees it).

**W3** — a plane missing the torus (z=0.65 over the torus r=0.5) proved
empty cleanly (0 points, 0 unresolved).

## Anchors (measured 2026-08-29 at HEAD `f45f2be`; re-derive before writing RESULT.json)

| id | file | pattern | count |
|----|------|---------|-------|
| A1 | vendor/truck/truck-evidence/src/contact/mod.rs | `\(CanonicalSurface::Torus\(_\), _\)` | 1 |
| A2 | vendor/truck/truck-evidence/src/contact/implicit.rs | `sqrt-free quartic form` | 1 |
| A3 | vendor/truck/truck-evidence/src/contact/implicit.rs | `fn grad\(&self, p: &Box3\)` | 5 |
| A4 | vendor/truck/truck-evidence/src/contact/mod.rs | `fn validated_ff\(` | 1 |
| A5 | vendor/truck/truck-evidence/tests | `torus_pairs.rs` | 0 (new file) |

A3 counts the five landed carrier grad impls; it becomes 6 IF you add the
sqrt-form as a second impl block on Torus (D1's choice — see the note).

## Decisions already made for you

**D1 — the sqrt-form re-enclosure of `Torus::grad` and `Torus::hess`.**
Replace the landed quartic-gradient evaluation with the sqrt-form:

```text
h = x'² + y'² (interval);  r̂ = sqrt(h)  (inari's interval sqrt);
grad = (2(r̂−R)·x'/r̂, 2(r̂−R)·y'/r̂, 2z');
hess: H_xy = 2·v vᵀ/r̂² + 2(r̂−R)·(I₂/r̂ − v vᵀ/r̂³) with v = (x', y'),
      H_zz = 2, H_xz = H_yz = 0.
```

The sqrt-form is the SAME function (the quartic and sqrt forms coincide
for the z-axis torus), so the enclosure stays SOUND and the landed
`implicit` (the quartic — its enclosure contains zero correctly) may stay
as-is. `f = 4g·x' − 8R²·x'`'s comment must be updated to record WHY the
sqrt form is used (the probe's Finding 1: the dependency blowup kills
`select_chart`). The landed tests at implicit.rs:476-483 (the enclosure
witnesses) must stay green UNCHANGED. The probe validated this exact form
end-to-end (Finding 2). If the landed `implicit` also needs the sqrt form
for enclosure tightness (machine-check W3's pruning through your arm),
make the minimal change and record it.

**D2 — the dispatch arm.** Replace the `(Torus(a), _) | (_, Torus(a))`
deferred arm with routing to the validated-FF stage. The arm's shape:
mirror the offset-quadric arms — non-coaxial/degenerate-family checks
first (D3), then the validated-FF composition. The `(Placed, _)` arms stay
deferred EXACTLY as they are.

**D3 — the torus-aware domain pre-split + the degenerate lift.** The
equator band (Finding 3) means `validated_ff`'s one-shot domain refuses
for any torus pair whose AABB straddles r̂ = R (i.e. essentially always).
The arm therefore composes the certified stages over a PRE-SPLIT domain:

1. Compute the world box exactly as `validated_ff` does (the two certified
   AABBs intersected axiswise; separation proves empty).
2. Recursively bisect the world box (widest axis, ties lowest-index) until
   every leaf's torus xy-enclosure EXCLUDES the equator ring — i.e. the
   leaf's `sqrt(x'²+y'²)` enclosure's distance from R is positive on both
   sides or the leaf is proven empty (the torus or plane enclosure excludes
   zero). The bisection floor is the same scale-relative `width/128` class
   `validated_ff` uses; a leaf that hits the floor still straddling the
   band refuses `ContactReductionDeferred` (the honest typed outcome —
   Finding 3's tangency family: a contact curve GRAZING the band is a
   singular-class contact the v1 envelope excludes; test 5 pins it).
3. Per clean leaf: the certified composition — `gff::cover_branch` over
   the leaf; singular boxes through `singular::singular_events`; merge the
   records into the `ValidatedBranchCover` locus (the landed arm's shape)
   and the returned `ContactComplex`.

FACTRING NOTE: `validated_ff` (A4) computes its domain internally — either
factor its post-domain tail (cover → singular → records) into a helper
both call, or mirror the composition in the torus path; YOUR choice,
recorded in RESULT notes. The landed offset-quadric behavior must not
change (test 7 guards it).

**D4 — the degenerate lift.** The landed doc (implicit.rs:303) books the
`r = R/2` inner-equator degeneracy as a positive-dimensional locus
`degenerate_points()` does not enumerate. The lift refuses the degenerate
families BEFORE any certified work: machine-check the exact condition set
from the landed form (the quartic's critical set {g = 2R², z = 0} vs the
surface, the doc's r = R/2 family, horn r ≥ R) and refuse
`UnsupportedEnvelope(NonCanonicalCarrier)` (or the landed-fitting arm —
record which) with the derivation in RESULT notes. Also handle the
IDENTICAL-carrier screen: `(Torus, Torus)` on the same carrier rides the
landed same-carrier screen machinery (mod.rs:599 area) — machine-check
what it answers and keep that answer.

**D5 — zero new arms.** No new `Refusal`/`EnvelopeCase`/
`UnresolvedWitness`/`Collapse` arms. The locus is the landed
`ValidatedBranchCover(gff::BranchCover)`; the refusals are the landed
vocabulary. A perceived need is a SPEC_GAP.

**D6 — certificates.** Every emitted crossing point carries the gff/singular
stages' Krawczyk certification (landed); the tests machine-check the
closed form (W1's two circles) and the surface residuals (W2) at
certification precision (1e-9 achieved by the probe — keep the tests at
the precision YOU achieve, recorded).

## Template

- `vendor/truck/truck-evidence/src/contact/gff.rs` — the cover engine
  (chart selection :238-266, the slab worklist); read `select_chart` and
  `minor_distance` before D1.
- `vendor/truck/truck-evidence/src/contact/singular.rs:117-123` — the
  singular recovery signature and report shape.
- `vendor/truck/truck-evidence/src/contact/mod.rs:684-790` — the landed
  `validated_ff` composition you mirror (A4).
- `vendor/truck/truck-evidence/src/contact/implicit.rs:239-330` — the
  landed Torus field (A2) and its enclosure witnesses (:476-483).

## Tests required (new file `tests/torus_pairs.rs`, dyadic witnesses)

Fixtures: `Torus::new(Point3::origin(), 2.0, 0.5)`; planes constructed
from three exact points (the probe's recipes).

1. `torus_plane_axial_two_circles` — plane z=0.25 × the torus through the
   DISPATCHER: the answer carries certified points; every point at z=0.25
   exactly, on the torus exactly, at the closed-form radii
   2 ± sqrt(0.1875) (machine-check); both families present.
2. `torus_plane_oblique_loop` — plane x+z=1.35 (clears the band; the
   loop's max r̂ ≈ 1.82): certified points satisfy BOTH surface equations
   at certification precision; non-empty.
3. `torus_plane_miss_proves_empty` — plane z=0.65: empty, no unresolved
   remainder.
4. `torus_degenerate_family_lift_refuses` — the D4 degenerate condition
   (r = R/2 and/or the machine-checked family) → the typed refusal at the
   lift, budget untouched.
5. `torus_equator_tangency_refuses` — plane x+z=1.5 (grazes the band at
   (2,0,−0.5)): the typed refusal (machine-check the arm: the
   pre-split floor's `ContactReductionDeferred` or the singular stage's
   honest outcome; assert what you observe, record it).
6. `torus_torus_identical_carrier_screen` — two identical-torus faces:
   machine-check what the landed same-carrier screen answers post-change
   and assert it (the D4 note).
7. `torus_quadric_offset_pair_still_green` — a landed offset-quadric pair
   (cylinder × cone, the mod.rs:435 shape) through `contact()` still
   answers its landed record (guards the D1/D2 changes against the
   quadric regression).

## H-3 (house rule; V4 is a text gate on your diff)

No ADDED line carries a bare absolute small literal (`1e-N`) without the
same-line `// H-3` opt-out. Run `& "C:\Program Files\Git\bin\bash.exe"
scripts/kernel-gates.sh HEAD` before writing RESULT.json (bare `bash` is
the WSL stub). CLIPPY EVERY CHANGED FILE — full unfiltered
`cargo clippy --locked -p truck-evidence --all-targets` before committing.

## Done when

Commit on the current branch (subject
`BG-CAD-P11-TORUS: torus FF pairs via the sqrt-form re-enclosure and the validated-FF stage`)
BEFORE writing RESULT.json AT THE WORKTREE ROOT (that exact path — not
`loop/results/`), then, all green:

```
cargo check --locked -p truck-evidence
cargo fmt --check -p truck-evidence
cargo test --locked -p truck-evidence --lib
cargo test --locked -p truck-evidence --test torus_pairs
cargo clippy --locked -p truck-evidence --all-targets
& "C:\Program Files\Git\bin\bash.exe" scripts/kernel-gates.sh HEAD
```

The landed `truck-evidence` suites must pass UNCHANGED (the lib suite's
known environmental failures excepted, if any — machine-check the base).

## Forbidden

- Do not edit `gff.rs`, `singular.rs`, `krawczyk.rs`, or anything outside
  `write_allow` (the cover/singular engines are certified machinery —
  D3's pre-split lives in the dispatch layer).
- Do not add `Refusal`/`EnvelopeCase`/`UnresolvedWitness`/`Collapse` arms
  (D5).
- Do not attempt torus×torus offset pairs, band-grazing cuts, or the
  `Placed` torus family (D2/D3 boundaries; booked follow-ups).
- GATE-3/H-4: `Face::debug_new` is banned in added lines.
- Do not write the tolerance-migration constructor name in prose or comments
  (GATE-4 counts raw text, comments included).
- No instrumentation traces may survive in the committed diff.

## Stop conditions

- `ANCHOR_MISMATCH` — report the measured count, change nothing.
- `SPEC_GAP` — a decision contradicts the tree; QUESTION.md with the
  empirical proof.
- The sqrt-form grad cannot certify where the probe certified (the probe's
  W1c boxes are the reproducibility witness: x,y ∈ [1.5,2]², z ∈
  [0.2,0.3] against plane z=0.25, torus (0,0,0) R=2 r=0.5) — stop and
  report your enclosure's divergence from the probe's.
- The D4 degenerate-condition derivation contradicts the landed doc's
  r = R/2 family — stop with the derivation.

RESULT.json: `{"id":"BG-CAD-P11-TORUS","status":"DONE","contracts":[...],
"tests_added":7,"deviations":[...],"notes":"..."}` — the D4 degenerate
derivation and the D3 factoring choice go in notes; every deviation with
your derivation; deviations are expected to be RIGHT.
