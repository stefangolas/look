# WORK PACKET BG-SOL-RW4-ASSEMBLE - the assembler and the boolean() entry

The Boundary Rewrite's final topology packet: decide every fragment through
the landed §13.1 primitive, sew the kept fragments, and validate with
`Solid::try_new` — plus the `boolean()` entry that composes the whole
pipeline (lift, AABB-screened sweep over the landed Contact Layer, split,
classify, assemble). Every design decision below was prototyped and
MEASURED by `scratch/rw3probe` (preserved) against the post-
BG-SOL-SPLIT-INVERTED splitter semantics via its `split_fixed` module: the
sweep produces exactly the flagship's six events, the decision table and
the pair-dedup rule assemble all four ops, and the Difference result is
geometrically congruent to `Extrude(P−Q)` (six containment probes plus a
256-point grid, 208/208). If live code contradicts this packet, report it
in `disagreements`.

This packet dispatches AFTER BG-SOL-SPLIT-INVERTED lands (the six-event
mesh your tests depend on is that packet's product; its two tests and the
sixteen pre-existing boolean tests must be green at your fork point).

```json
{"id":"BG-SOL-RW4-ASSEMBLE","status":"DONE","contracts":["BG-SOL-RW4-ASSEMBLE"],
 "tests_added":4,"sites_migrated":0,"sites_deferred":0,
 "deviations":[],"disagreements":[],"baseline_failures":[],"notes":"free text"}
```

```yaml
id:          BG-SOL-RW4-ASSEMBLE
contract:    [BG-SOL-RW4-ASSEMBLE]
class:       design
crates:      [truck-shapeops]
write_allow:
  - vendor/truck/truck-shapeops/src/boolean/assemble.rs
  - vendor/truck/truck-shapeops/src/boolean/mod.rs
read_allow:
  - vendor/truck/truck-shapeops/src/boolean/split.rs
  - vendor/truck/truck-shapeops/src/boolean/classify.rs
  - vendor/truck/truck-evidence/src/contact/mod.rs
  - vendor/truck/truck-geometry/src/recognize.rs
  - docs/SOLVER_FAMILY_PLAN.md
tests_required:
  - boolean_sweep_produces_the_flagship_event_complex
  - boolean_difference_flagship_assembles_the_plate_with_hole
  - boolean_union_intersection_xor_on_the_flagship
  - boolean_refuses_multishell_input
budget:      {turns: 50, ctx_tokens: 160000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub mod classify' vendor/truck/truck-shapeops/src/boolean/mod.rs"}
  - {id: A2, expect: 3, cmd: "ls vendor/truck/truck-shapeops/src/boolean | wc -l"}
  - {id: A3, expect: 9, cmd: "grep -cF '#[test]' vendor/truck/truck-shapeops/src/boolean/split.rs"}
  - {id: A4, expect: 6, cmd: "grep -cF '#[test]' vendor/truck/truck-shapeops/src/boolean/classify.rs"}
  - {id: A5, expect: 1, cmd: "grep -c 'pub fn classify_fragments' vendor/truck/truck-shapeops/src/boolean/classify.rs"}
  - {id: A6, expect: 1, cmd: "grep -c 'pub fn fragment_decision' vendor/truck/truck-shapeops/src/boolean/mod.rs"}
  - {id: A7, expect: 1, cmd: "grep -c 'let is_region = |area: f64| area > 0.0' vendor/truck/truck-shapeops/src/boolean/split.rs"}
  - {id: A8, expect: 1, cmd: "grep -c 'FragmentInsideOther' vendor/truck/truck-base/src/evidence.rs"}
```

A2 becomes 4 (`assemble.rs` joins); all others stay. (All anchors are
re-derived by the orchestrator against the dispatch fork point; A2 measured 3
at bbad435 and was corrected from the stale 4/5 pair.)

## Problem

`boolean()` is the M2 flagship's right-hand side:
`Extrude(P−Q) ≅ boolean(Extrude(P), Difference, Extrude(Q))`. The splitter
(RW2 + fixes), the classifier (RW3), and the decision primitive (RW1) are
landed; what is missing is the composition: producing the contact events
from the two solids (no caller-supplied event list), deciding every
fragment, and sewing the kept fragments into a valid solid.

## Decisions already made for you

### 1. Module shape

`vendor/truck/truck-shapeops/src/boolean/assemble.rs` (new) + one line
`pub mod assemble;` in `boolean/mod.rs` beside `pub mod classify;`. Carry
the H-1 deny header inside the module exactly like `boolean/mod.rs` does
(unwrap/expect/panic/todo/unimplemented/indexing_slicing). Every public
item carries a doc comment (the crate warns on `missing_docs`).

### 2. The booked entry signature (plan §4)

```rust
use truck_base::evidence::{Budget, Outcome};
use truck_geometry::canonical::{Curve, Surface};
use truck_topology::Solid;
use crate::BoolOp;

/// The regularized Boolean of two single-shell solids (plan §4 Phase 4).
pub fn boolean(
    a: &Solid<Point3, Curve, Surface>,
    op: BoolOp,
    b: &Solid<Point3, Curve, Surface>,
    budget: &mut Budget,
) -> Outcome<Solid<Point3, Curve, Surface>>;
```

The insertion tolerance is a named module constant
`INSERTION_TOL: f64 = 1.0e-2; // H-3: the insertion tolerance class (length)`
(the splitter/classifier tolerance class; tightening it is future work,
never a test's lever).

### 3. The pipeline, in order

0. **GUARDS**: refuse `UnsupportedEnvelope(EnvelopeCase::ContactReductionDeferred)`
   unless BOTH inputs are single-shell (`boundaries().len() == 1`) — the
   RW-MULTISHELL fold.
1. **LIFT**: for each face of each solid, `recognize_surface` then
   `face_stratum(witness, u_box, v_box)` where the `(u, v)` box is the
   hull of the face's wire polygons (reuse the `pub(crate)`
   `create_parameter_boundary` from split.rs; min/max over all polygon
   points). For each EDGE (first occurrence by `EdgeID` across
   `face_iter()` order, with `StratumRef::Edge { solid, face, edge }`
   provenance at its flat position in that face's
   `absolute_boundaries()`), `recognize_curve` then
   `BoundedStratum::Edge { curve, t_range }` with `t_range` from
   `edge.curve().range_tuple()`. An `Unrecognized` witness (surface or
   curve) refuses `UnsupportedEnvelope(EnvelopeCase::NonCanonicalCarrier)`
   — the lift boundary, before `contact()` is ever reached.
2. **SWEEP** (`pub(crate) fn sweep_contact_events(a, b, tol) ->
   Outcome<Vec<ContactEvent>>`, unit-testable): compute each stratum's 3-D
   AABB (min/max over the boundary curves' `parameter_division` sample
   points). For every CROSS-SOLID pair — FF (a-face × b-face), FE
   (a-face × b-edge AND b-face × a-edge), EE (a-edge × b-edge) — skip
   unless the AABBs touch (INCLUSIVE overlap: `a.lo <= b.hi && b.lo <=
   a.hi` on all three axes; boundary touch counts — the real FF circle
   sits exactly on the wall's box boundary). Survivors run
   `contact(lhs, rhs, budget)`; every record of every `Ok` complex
   becomes `ContactEvent { record, lhs, rhs }` with the pair's strata
   (FE events as `(Face, Edge)` in either order — the splitter's
   `collect_sew` normalizes). A `contact()` refusal propagates as-is.
   The screen is the sound candidate filter that drops the exact arms'
   bounds-blind false positives (measured: without it, plane×plane emits
   Line records for the side-plane × cap pairs whose loci miss both
   trimmed faces; with it, the flagship sweep yields exactly the six
   real events).
3. **SPLIT**: `split_fragments(shell_a, shell_b, &events, INSERTION_TOL)`
   (the shells are the inputs' single boundaries).
4. **CLASSIFY**: `classify_fragments(shell_a, shell_b, &mesh,
   INSERTION_TOL)` (the landed RW3 function).
5. **DECIDE + ASSEMBLE** (decision 4 below).

### 4. The decision table and the sewing

Per fragment `i`: `MaterialState4` with own pair `(1,0)` (the fragment's
own solid is on the minus side of its own effective normal) and the other
pair `(s, s)` with `s = classification.inside_other[i]` — EXCEPT that a
coincident pair takes PRECEDENCE: a fragment in `mesh.coincident[p]` gets
the other pair `(1,0)` if the pair is `Identical`, `(0,1)` if `Anti`.
(Both fragments of a pair read the same orientation-derived other-pair;
the own/other slots follow the fragment's `FragmentOrigin`.)

`fragment_decision(op, m)` decides. A coincident pair is resolved ONCE:

- The two fragments' verdicts must AGREE (both Keep or both Discard);
  disagreement refuses (decision 6's refusal).
- When both are kept with flips `fa`, `fb`: they are geometric duplicates
  with the same resulting effective normal — `Identical` requires
  `fa == fb`, `Anti` requires `fa != fb`; anything else refuses. Emit
  EXACTLY the pair's `a` fragment once (with `fa` applied).

Non-pair fragments: keep iff `Keep { flip }`; `flip` ⇒ `face.invert()` on
a clone. The kept faces form the result shell; if the shell is empty the
result is the empty solid (`Solid::try_new(Vec::new())` — zero shells;
all-discarded means the op's result is empty). Otherwise
`Shell::connected_components()` must yield exactly ONE component (more
refuses — the multi-component fold); `Solid::try_new(vec![shell])`
validates. Measured on the flagship (the 11-fragment six-event mesh,
bits `[F,T,F,T,F,F,F,F,T,T,T]`): Difference keeps a's two annuli + 4
sides unflipped and b's wall FLIPPED (7 faces — the plate with hole);
Union keeps the annuli, the two deduped disks, and the sides (8 faces —
the block, cosmetically split); Intersection keeps the two deduped disks
and the wall unflipped (3 faces — the cylinder); Xor keeps the Difference
set (7 faces). All four `Solid::try_new` Ok.

### 5. What the sweep measures on the flagship (your test 1 asserts this)

a = the 4×4 block extrude (faces: 0 = bottom z=0 inverted, 1 = top z=2,
2..5 = sides), b = the disk extrude at (2,2) r=1 (faces: 0 = bottom cap
inverted, 1 = top cap, 2 = the wall). Exactly SIX events: Region2
`Coincident` for (a0 × b0) and (a1 × b1); FF `Transverse
Analytic(Curve(Circle))` for (a0 × b2) and (a1 × b2) — circles at
(2,2,0) and (2,2,2), r=1; FE `CoincidentInterval BoundedCurve` full-period
for (a0 × b's bottom rim edge) and (a1 × b's top rim edge). The rim-edge
provenance names whichever face carries the edge first in
`face_iter()` order (measured: b's caps) — the splitter resolves the
instance either way. The six events reproduce the hand-built mesh exactly
(11 fragments, 20 adjacency, 2 coincident pairs — the numbers BG-SOL-
SPLIT-INVERTED's test pins).

### 6. Refusals (typed, never panics)

- Multi-shell input, unrecognized carrier at the lift: decision 3.
- A fragment in two coincident pairs; a pair whose verdicts disagree; a
  pair whose flips contradict its orientation; a multi-component kept
  shell; any `Solid::try_new` error: refuse
  `UnsupportedEnvelope(EnvelopeCase::ContactReductionDeferred)` — the v1
  envelope's boundary (each site documented with a comment naming the
  fold). A `Solid::try_new` failure is a typed refusal about the
  split/classify, never a panic.
- `contact()`, `split_fragments`, and `classify_fragments` refusals
  propagate as themselves.

## Tests required

Dyadic witnesses throughout (H-3); copy the construction helpers from
split.rs's test module (`placed_circle`, `block_profile`, `disk_profile`,
`extrude_shell`) — `truck-modeling` is already a dev-dependency, so the
tests build the inputs with `extrude_profile` exactly like split.rs's
tests do. Identify result faces by CARRIER + wire structure, never by raw
index without derivation; write every expected value's derivation as an
inline comment (the BG-NUM-002 rule). Machine-check every number before
asserting; where your derivation and this packet disagree, follow your
derivation and record the difference in `deviations`.

1. `boolean_sweep_produces_the_flagship_event_complex`: build the
   flagship inputs; run `sweep_contact_events`; assert exactly six
   events; classify each by (dimension, kind, locus arm, provenance)
   against decision 5's table; assert the FF circles' centers and radius
   and the FE `t_range == (0.0, TAU)`.
2. `boolean_difference_flagship_assembles_the_plate_with_hole`:
   `boolean(&block, Difference, &disk, &mut budget)` is `Ok`; the result
   has ONE boundary shell and 7 faces: two `[4, 2]`-wire Plane faces (the
   annuli, at z=0 and z=2), four `[4]`-wire Plane faces (the sides), one
   `[2, 2]`-wire Cylinder face (the hole wall). Assert the wall's
   EFFECTIVE normal points TOWARD the axis: sample the effective normal
   (`surface.normal` negated iff `!face.orientation()`) at a dyadic point
   of the wall and assert its dot with the outward radial direction
   there is negative. (The entry already ran `Solid::try_new`.)
3. `boolean_union_intersection_xor_on_the_flagship`: the other three ops
   on the same inputs; face counts 8 / 3 / 7, one shell each; for
   Intersection identify the 3 faces (two `[2]`-wire Planes at z=0/z=2
   and the `[2, 2]` Cylinder, unflipped — its effective normal points
   OUTWARD from the axis).
4. `boolean_refuses_multishell_input`: a two-shell solid (two disjoint
   2×2 block extrudes far apart as `Solid::try_new(vec![s1, s2])`) against
   the block: the entry refuses
   `UnsupportedEnvelope(EnvelopeCase::ContactReductionDeferred)` at the
   guard, before any sweep work.

## House form (H-3)

This crate is under the kernel's house rules. Any ADDED line with a bare
`1e-N` float literal must end `// H-3`; prefer dyadic values,
`TAU`/`std::f64::consts`, or named constants. GATE-2 scans the diff.
Run `bash scripts/kernel-gates.sh <your base commit>` before writing
RESULT.json - a failing gate is a finding to report, never one to work
around.

## Done when

```console
cargo fmt --check -p truck-shapeops
cargo clippy -p truck-shapeops --all-targets --no-deps
cargo check --locked -p truck-shapeops --all-targets
cargo test -p truck-shapeops --lib boolean --no-fail-fast
bash scripts/kernel-gates.sh <your base commit>
```

Never run bare `cargo test` or a workspace-wide cargo command. All
eighteen pre-existing boolean tests (nine split + six classify + ...count
them at your fork point) must stay green.

**Commit your work on the current branch** (subject
`shapeops: the assembler and the boolean() entry (BG-SOL-RW4-ASSEMBLE)`)
**before** writing `RESULT.json`: the verifier measures the committed
diff, and an uncommitted tree reads as an interrupted run.

## Forbidden

Editing anything outside `write_allow` (split.rs and classify.rs are
READ-ONLY for you — their pub(crate) exports are consumed, never
changed); changing any split/classify behavior; renaming or deleting a
pre-existing test; a `_` wildcard arm in any `Surface`, `ContactLocus`,
or `CanonicalCarrierWitness` match; `#[ignore]`; loosening a gate;
changing the GATE-4 ceiling; widening `INSERTION_TOL` to make a test
pass; implementing M2-WITNESS's congruence battery (the cross-layer
flagship and the metamorphics are the NEXT packet, not yours); editing
`truck-evidence` (the sweep CONSUMES `contact`/`face_stratum`).

## Stop conditions

- anchor mismatch -> `ANCHOR_MISMATCH` with observed count;
- a booked type or signature cannot be realized as specified ->
  `SPEC_GAP` with the compile error and your proposed shape - do NOT
  silently change the booked field names, they are the inter-packet
  contract;
- the sweep or an assembly cannot reproduce decision 5's / decision 4's
  measured numbers -> `SPEC_GAP` with both derivations;
- three consecutive cargo failures with one cause -> `BLOCKED`.

Finish by writing `RESULT.json` in the worktree root, not
`loop/results/`. Record in `notes`: the measured event complex (six
events with provenance), the four ops' face counts, any tolerance-class
decision that surprised you, and whether the prototype's predictions
(`scratch/rw3probe`, preserved) matched your implementation.
