---
id: RW-INTERIOR-LOOP
class: design
crates: [truck-shapeops]
write_allow:
  - vendor/truck/truck-shapeops/src/boolean/split.rs
  - vendor/truck/truck-shapeops/src/boolean/classify.rs
  - vendor/truck/truck-shapeops/src/boolean/assemble.rs
  - vendor/truck/truck-shapeops/tests/interior_loop.rs
tests_required:
  - variant_f_through_cylinder_difference_assembles
  - variant_f_through_cylinder_intersection_assembles
  - variant_h_halfspace_difference_assembles
  - variant_h_halfspace_intersection_assembles
  - recombination_f_is_original
  - recombination_h_is_original
  - cutter_wall_divided_at_interior_rims
  - flagship_coplanar_variant_still_ok
budget: {turns: 50, ctx_tokens: 140000}
---

# RW-INTERIOR-LOOP — interior-loop division for the through-cut family

Program: the boolean Boundary Rewrite follow-up booked in
`loop/results/BG-CAD-P3-SPLIT.PROBE.md` (read it first — it carries the bisect
evidence this packet is written from). Everything below is pre-decided; churn,
don't design. Contradiction with the tree = `SPEC_GAP`.

## Problem

The landed `boolean()` refuses EVERY cut whose cutter terminates INTERIOR to
the solid (probe variants [a-h] in the PROBE doc, all
`UnsupportedEnvelope(ContactReductionDeferred)`), while the coplanar M2
flagship (variant [i]) works. The probe proved every stratum pair answers in
the Contact Layer — the deferral is downstream, in the Boundary Rewrite: a
contact locus that arrives only as an FF Transverse record must divide faces
that have NO coincident partner, including the cutter's own wall at its
interior rims. Session 37 recorded exactly this limitation ("the wall is not
divided at interior circles").

The deferral is raised from MANY `unsupported()` / `refused()` sites (the
refusal constructor is shared), and the probe bisected only to pipeline level.
**Your first mandated task is to localize the actual refusing site
empirically** (D1) — the packet's diagnosis hypotheses may be wrong in the
details; they are wrong in your favor if the evidence says so. Machine-check,
record the site with its derivation in RESULT notes; deviations are expected
to be RIGHT.

## Anchors (measured 2026-08-28 at HEAD `8111be9`; re-derive before writing RESULT.json)

| id | file | pattern | count |
|----|------|---------|-------|
| A1 | vendor/truck/truck-shapeops/src/boolean/split.rs | `fn ff_curve\(&mut self` | 1 |
| A2 | vendor/truck/truck-shapeops/src/boolean/split.rs | `fn add_doubled_loop\(&mut self` | 1 |
| A3 | vendor/truck/truck-shapeops/src/boolean/split.rs | `fn classify_curve\(` | 1 |
| A4 | vendor/truck/truck-shapeops/src/boolean/split.rs | `fn insert_open_arc_shared\(` | 1 |
| A5 | vendor/truck/truck-shapeops/src/boolean/classify.rs | `Refusal::Contradictory` | 3 |
| A6 | vendor/truck/truck-shapeops/src/boolean/assemble.rs | `pub fn boolean\(` | 1 |
| A7 | vendor/truck/truck-shapeops/src/lib.rs | `pub mod` | 2 |

All anchors are the PRE-packet tree. If your design needs no new `pub mod`
(and it does not — no new module), A7 never diverges.

## The fixtures (exact, dyadic)

- **S** — the flagship plate: extruded 4×4×2 plate, exactly as
  `tests/boolean_m2.rs` builds its fixtures (`extrude_profile` + `arrange`;
  read that file's fixture section and copy the recipe, do not reinvent it).
- **Cylinder cutter [f]** — disk r=1 centered (2,2): extrude a circle profile
  at z=0 height 4, then `truck_modeling::cad::translate_solid` by
  (0,0,−1) → z ∈ [−1,3], caps NOT coplanar with the plate caps (plate
  z ∈ [0,2]). `translate_solid` is landed at `cad.rs:281` — verify the
  signature before use.
- **Halfspace box [h]** — rect x,y ∈ [1,3], z ∈ [−4,1]: rect profile extruded
  height 5, translated by (0,0,−4). No coplanar pair with the plate anywhere.
- Every parameter above is dyadic; no epsilon enters fixture construction.

## Decisions already made for you

**D1 — reproduce, then localize, then fix.** Write
`tests/interior_loop.rs` first with the four boolean calls (S × cutter,
`Difference` and `Intersection`, for [f] and [h]) asserting ONLY the refusal
shape you actually observe. Then localize the refusing site by instrumenting
your OWN working copy (temporary `eprintln!`/backtrace-style traces are fine;
REMOVE every trace line before the final commit — the verifier diffs your
code). Record in RESULT notes: file, function, the guard that fired, and why
the input reaches it. Then implement the fix per D2–D4 and flip the tests to
the acceptance assertions in the Template section.

**D2 — closed FF Transverse loci divide BOTH faces, seam-blind (the
generalization).** The doubled-independent-loop insertion for a closed locus
strictly inside a face's region exists (`add_doubled_loop`, split.rs:1014, fed
by `ff_curve` at split.rs:804 when `classify_curve` says `Inside`). The
through-cut family exposes the gap: on a PERIODIC carrier (the cutter's
cylindrical wall) a full-period circle's parameter polygon touches the seam,
so the parameter-space classification is seam-confused even though the 3-D
trace is strictly interior to the face's region. The canonical rule:

> A full-period closed locus whose 3-D trace lies strictly inside a face's
> region divides that face by the doubled independent loop, INDEPENDENT of
> seam/branch artifacts of the parameter-space classification.

The session-37 fix pattern is the prescribed tool: re-test the classification
at ±period translates of the query frame wherever the parameter polygons'
frame and the locus's frame can differ. Do NOT weaken the parameter-space
test for open loci — open-arc clipping (`insert_clipped_arc`) and the
no-certified-endpoints refusal for open interior arcs stay exactly as they
are.

**D3 — the cutter's wall divides at its interior rims.** Under D2 the cutter
wall (a periodic carrier) is divided at every interior rim circle as a
doubled independent loop with SHARED edge instances (the splitter's
shared-instance invariant, split.rs header lines 8-10), producing separate
wall fragments. The solid's faces divide at the same circles symmetrically.
No new wire-mutation machinery: `add_independent_loop` /
`add_doubled_loop` / `swap_edge_into_wire` are the only insertion primitives,
as today.

**D4 — the classifier adjudicates; you add no classification machinery.**
The landed seed-and-propagate parity check (`classify.rs`, `Refusal::Contradictory`
= the A5 sites) is the authority on whether the divided mesh is consistent.
If a happy-path variant answers `Contradictory` after a D2/D3 implementation
you believe correct, STOP — that is a stop-condition finding (the parity
graph sees an event class the packet did not anticipate), not something to
route around by hand-seeding.

**D5 — the v1 envelope of this packet.** In scope: closed full-period
Circle/Line FF loci (a full-period Line locus is not a thing — closed means
Circle/Ellipse per the split.rs header; Ellipse stays on the RW-CONIC
refusal), and open loci whose crossing behavior is already landed. Still
out of scope and REFUSING as today: `ValidatedBranchCover`, `Tangency`,
`Parabola`/`Hyperbola`, partial-angle circles, multi-shell, self-pairs. Zero
new `Refusal`/`EnvelopeCase`/`UnresolvedWitness`/`Collapse` arms — a
perceived need is a SPEC_GAP.

**D6 — the acceptance metamorphic.** For [f] and for [h]:
`boolean(S, Difference, C)` ∪ `boolean(S, Intersection, C)` ≅ S — assert via
`boolean(plus, BoolOp::Union, minus)` assembling to a solid with the
ORIGINAL's face count and exact `cad::solid_bounding_box` (landed,
`cad.rs:82`). Additionally `cutter_wall_divided_at_interior_rims` asserts the
[f] Difference's hole wall appears as cylinder face(s) spanning exactly
z ∈ [0,2].

## Template

- `vendor/truck/truck-shapeops/src/boolean/split.rs` — the splitter (A1-A4);
  its header documents the per-arm semantics you are extending.
- `vendor/truck/truck-shapeops/src/boolean/classify.rs` — the parity
  classifier (A5); read its seed rule before D4.
- `vendor/truck/truck-shapeops/tests/boolean_m2.rs` — the fixture recipes
  and the acceptance style (box/face-count equality); your test file mirrors
  its imports (truck-modeling is a dev-dependency).
- `loop/results/BG-CAD-P3-SPLIT.PROBE.md` — the bisect evidence and the
  booking (tracked; in your worktree).

## H-3 (house rule; V4 is a text gate on your diff)

No ADDED line carries a bare absolute small literal (`1e-N`) without the
same-line `// H-3` opt-out. Dyadic fixture constants and geometry-derived
tolerance reuse (the existing `PARAM_SLACK` / insertion-tol consts) only.
Run `& "C:\Program Files\Git\bin\bash.exe" scripts/kernel-gates.sh HEAD`
before writing RESULT.json (bare `bash` is the WSL stub).

## Done when

Commit on the current branch (subject
`RW-INTERIOR-LOOP: interior-loop division for the through-cut family`) BEFORE
writing RESULT.json, then, all green:

```
cargo check --locked -p truck-shapeops
cargo fmt --check -p truck-shapeops
cargo test --locked -p truck-shapeops --lib
cargo test --locked -p truck-shapeops --test interior_loop
cargo test --locked -p truck-shapeops --test boolean_m2
cargo clippy --locked -p truck-shapeops --all-targets
& "C:\Program Files\Git\bin\bash.exe" scripts/kernel-gates.sh HEAD
```

`boolean_m2` must pass UNCHANGED (you do not edit it; it is outside
write_allow — its passing proves the M2 envelope did not regress).

## Forbidden

- Do not edit `boolean/mod.rs`, anything outside `write_allow`, or
  `vendor/truck/**` beyond the three allowed source files.
- Do not add `Refusal`/`EnvelopeCase`/`UnresolvedWitness`/`Collapse` arms
  (D5; zero-new-arms program rule).
- Do not weaken or rescope any landed test; do not edit existing test files.
- GATE-3/H-4: `Face::debug_new` is banned in added lines.
- Do not write the tolerance-migration constructor name in prose or comments
  (GATE-4 counts raw text, comments included).
- No instrumentation traces may survive in the committed diff (D1).

## Stop conditions

- `ANCHOR_MISMATCH` — report the measured count, change nothing.
- `SPEC_GAP` — a decision contradicts the tree; QUESTION.md with the
  empirical proof.
- A happy-path variant refuses `Contradictory` after a D2/D3-faithful
  implementation (D4) — stop, report the witness verbatim.
- The deferral localizes OUTSIDE the three allowed source files — stop and
  report; do not widen your own write set.

RESULT.json: `{"id":"RW-INTERIOR-LOOP","status":"DONE","contracts":[...],
"tests_added":8,"deviations":[...],"notes":"..."}` — the localized refusing
site goes in notes with its derivation; every deviation with your derivation;
deviations are expected to be RIGHT.
