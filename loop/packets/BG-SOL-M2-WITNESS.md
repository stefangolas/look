# WORK PACKET BG-SOL-M2-WITNESS - the M2 cross-layer flagship and the metamorphic battery

The M2 milestone's differential test: `Extrude(P−Q) ≅ boolean(Extrude(P),
Difference, Extrude(Q))` — the M1 construction (arrangement + direct
extrude, no 3-D Boolean) checked against the 3-D contact path through the
LANDED `boolean()` entry — plus the metamorphic battery (A∩B≅ the
cylinder, A∪B≅B∪A, and the self-pair metamorphics' v1 boundary).

Every number in this packet was MEASURED by the design probe
(`scratch/m2probe_run1.txt`, session 39) against the LANDED entry at
`bd591bb`: the flagship congruence is 7 faces vs 7 faces with 256/256
per-point grid agreement, Intersection is 3 faces vs 3 (48/48 grid),
Union is 8 faces in BOTH orders (256/256 each), and both self-pair runs
REFUSE `UnsupportedEnvelope(ContactReductionDeferred)`. Every measured
number is reproduced in this packet — you need no scratch artifact (they
are untracked and absent from your worktree). If live code contradicts
this packet, report it in `disagreements`.

This packet dispatches AFTER BG-SOL-RW4-ASSEMBLE lands (the `boolean()`
entry and its four tests are that packet's product; all twenty-two
pre-existing boolean tests must be green at your fork point).

```json
{"id":"BG-SOL-M2-WITNESS","status":"DONE","contracts":["BG-SOL-M2-WITNESS"],
 "tests_added":4,"sites_migrated":0,"sites_deferred":0,
 "deviations":[],"disagreements":[],"baseline_failures":[],"notes":"free text"}
```

```yaml
id:          BG-SOL-M2-WITNESS
contract:    [BG-SOL-M2-WITNESS]
class:       design
crates:      [truck-shapeops]
write_allow:
  - vendor/truck/truck-shapeops/tests/boolean_m2.rs
read_allow:
  - vendor/truck/truck-shapeops/src/boolean/assemble.rs
  - vendor/truck/truck-shapeops/src/boolean/mod.rs
  - vendor/truck/truck-shapeops/src/boolean/split.rs
  - docs/SOLVER_FAMILY_PLAN.md
tests_required:
  - m2_flagship_extrude_p_minus_q_congruent_boolean_difference
  - m2_intersection_is_extrude_q_with_outward_wall
  - m2_union_commutative_both_orders_match_extrude_p
  - m2_self_pair_refuses_the_typed_envelope
budget:      {turns: 40, ctx_tokens: 140000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub mod assemble' vendor/truck/truck-shapeops/src/boolean/mod.rs"}
  - {id: A2, expect: 4, cmd: "ls vendor/truck/truck-shapeops/src/boolean | wc -l"}
  - {id: A3, expect: 9, cmd: "grep -cF '#[test]' vendor/truck/truck-shapeops/src/boolean/split.rs"}
  - {id: A4, expect: 6, cmd: "grep -cF '#[test]' vendor/truck/truck-shapeops/src/boolean/classify.rs"}
  - {id: A5, expect: 4, cmd: "grep -cF '#[test]' vendor/truck/truck-shapeops/src/boolean/assemble.rs"}
  - {id: A6, expect: 1, cmd: "grep -c 'pub fn boolean' vendor/truck/truck-shapeops/src/boolean/assemble.rs"}
  - {id: A7, expect: 1, cmd: "grep -c 'pub fn fragment_decision' vendor/truck/truck-shapeops/src/boolean/mod.rs"}
  - {id: A8, expect: 1, cmd: "grep -c 'material_state_decides_coincident_fragments' vendor/truck/truck-shapeops/src/boolean/mod.rs"}
  - {id: A9, expect: 2, cmd: "ls vendor/truck/truck-shapeops/tests | wc -l"}
```

(A2 becomes 5 and A9 becomes 3 when `tests/boolean_m2.rs` lands; all
anchors were re-derived by command against `bd591bb` — the pre-dispatch
values above are the tree you fork from.)

## Problem

M2's claim is cross-layer: the same canonical answer must fall out of two
independent constructions. M1 (landed) builds the plate-with-hole by the
2-D arrangement and a direct extrude. The Boundary Rewrite (landed:
splitter + fixes, classifier, assembler, entry) builds it by lifting both
solids' strata, sweeping certified contact events, splitting, classifying,
deciding, and sewing. Nothing has yet CHECKED the two against each other,
and nothing has pinned the battery's commutativity or the self-pair
boundary at the entry level.

## Decisions already made for you

### 1. Module shape

`vendor/truck/truck-shapeops/tests/boolean_m2.rs` (new integration test;
public API only: `truck_shapeops::boolean::assemble::boolean`,
`truck_shapeops::boolean::BoolOp`, `truck_modeling::extrude::extrude_profile`
(truck-modeling is already a dev-dependency), `truck_geometry::arrange`,
`truck_geometry::canonical::{Curve, Surface}`, `truck_base::evidence`
(`Budget`, `Refusal`, `EnvelopeCase`), `truck_topology::Solid`). The file
opens with the boolean family's H-1 idiom — the six-lint deny header
followed by the documented test-only allow, exactly as `assemble.rs`'s
`#[cfg(test)]` module carries it:

```rust
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]
// Test-only allow: H-1 bans unwrap/expect/panic on paths reachable from
// untrusted geometry. This file is integration-test assertions on
// hand-built dyadic witnesses - not such a path.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]
```

Every public item the file declares (helpers) carries a doc comment only
if the crate's lints require it for test binaries — they do not; keep
helpers private with `///` comments where they encode a derivation.

### 2. Construction (copy, do not redesign)

Copy `placed_circle`, `block_profile`, `plate_with_hole_profile`,
`disk_profile` VERBATIM from split.rs's test module (they are in-crate
and proven). Build the SOLIDS directly — the entry takes `&Solid`, and
`extrude_profile(&profile, &arr, height)` returns the solid:

- `solid_a` = Extrude(P): the 4×4 block, height 2 (6 faces).
- `solid_b` = Extrude(Q): the disk at (2,2) r=1, height 2 (3 faces).
- `solid_ph` = Extrude(P−Q): the plate-with-hole profile, height 2
  (7 faces — MEASURED).

The witness geometry is the flagship's: a = the 4×4 block (faces: bottom
z=0, top z=2, four sides), b = the disk extrude at (2,2) r=1 (bottom cap,
top cap, wall). The disk's footprint lies strictly inside the square; the
caps' planes COINCIDE with the block's (both z=0 and z=2).

### 3. The congruence criterion (≅): the face-set bijection

Two closed manifold solids with identical face sets — same carriers, same
trimmed regions, same effective orientations — bound the same point set
(the winding number is determined by the face set). So the differential
is asserted as a FACE-SET BIJECTION: for each result face there is exactly
one ground-truth face with (a) the same carrier discriminant (Plane by
its constant coordinate, Cylinder by axis + radius), (b) the same
wire-count signature, (c) the same wire curve kinds in corresponding
wires, and (d) the same effective normal direction. Identify faces by
CARRIER + wire structure, never by raw index without derivation (the
landed house rule). The design-time volumetric confirmation is recorded
in this packet's test table (a 256-point grid, 8×8×4 dyadic points at
0.25 + 0.5k over [0,4]×[0,4]×[0,2]); do NOT import ray-casting oracle
machinery into the test — the bijection plus `Solid::try_new` (which the
entry already ran) is the claim.

Wire curve kinds, for (c): the annulus's outer wire is 4 line segments
(the unit square), its hole wire is ONE full circle (center (2,2), r=1);
a side's wire is 4 line segments; the wall's two wires are each one full
circle at z=0 and z=2; a disk's single wire is one full circle.

### 4. The battery, with the measured numbers

All four runs used `Budget::new(1000, 1000, 1000)`. Measured at `bd591bb`:

| run | outcome | census |
|---|---|---|
| `boolean(&solid_a, Difference, &solid_b)` | Ok, 1 shell, **7 faces** | 2× Plane [4,2] (z=0, z=2 annuli), 4× Plane [4] (sides), 1× Cylinder [2,2] (wall, FLIPPED — effective normal toward the axis) |
| `boolean(&solid_a, Intersection, &solid_b)` | Ok, 1 shell, **3 faces** | 2× Plane [2] (disks z=0/z=2), 1× Cylinder [2,2] (wall, UNFLIPPED — outward) |
| `boolean(&solid_a, Union, &solid_b)` | Ok, 1 shell, **8 faces** | 2× Plane [4,2] (annuli), 2× Plane [2] (deduped disks), 4× Plane [4] (sides) |
| `boolean(&solid_b, Union, &solid_a)` | Ok, 1 shell, **8 faces** | same census as above |
| `boolean(&solid_a, Union, &solid_a)` | **Err** `UnsupportedEnvelope(ContactReductionDeferred)` | — |
| `boolean(&solid_a, Difference, &solid_a)` | **Err** `UnsupportedEnvelope(ContactReductionDeferred)` | — |

Ground-truth censuses (also measured): Extrude(P−Q) 7 faces (the same
classes as the Difference result), Extrude(Q) 3 faces (2× Plane [2] +
Cylinder [2,2]), Extrude(P) 6 faces (2× Plane [4] caps + 4× Plane [4]
sides).

The design-time grid (measured, machine-checked; cite in comments, do
not re-derive mechanically): the plate-with-hole contains 208/256 grid
points, the cylinder column 48/256 (the twelve (x,y) cells with
(|dx|,|dy|) ∈ {(0.25,0.25),(0.25,0.75),(0.75,0.25)} × 4 z-levels), the
block 256/256. Six named probes (all measured OK on every congruent
pair): (2,2,1) outside (the hole's center), (2,3.5,1)/(3.9,3.9,1)/
(0.5,0.5,0.5) inside, (2,2,3)/(2,2,−1) outside.

### 5. The self-pair decision (the design note, decided)

The A−A case through the entry runs the sweep over a solid paired with
ITSELF: six identity-arm Region2 events (one per face) PLUS intra-solid
adjacency events (perpendicular side×cap Line records on shared edges,
FE coincidences of rim edges in cap planes, EE vertex sharings) — an
event class no well-posed cross-solid input produces. The MEASURED
outcome: the entry folds the complex to the typed refusal
`UnsupportedEnvelope(ContactReductionDeferred)` — never a panic, never a
wrong Ok. The battery therefore ASSERTS the refusal as the recorded v1
boundary: the idempotence ALGEBRA is already pinned by
`material_state_decides_coincident_fragments` (A8 — A∪A=A, A∩A=A, A−A=∅,
A△A=∅ at the decision-table level), and the self-pair composition is the
RW-COPLANAR family's concern. Do NOT add a self-pair guard or a ptr-eq
fast path to the entry, and do NOT hand-build a self-pair event list —
both would misrepresent what v1 claims.

## Tests required

Dyadic witnesses throughout (H-3); write every expected value's
derivation as an inline comment (the BG-NUM-002 rule); machine-check
every number before asserting; where your derivation and this packet
disagree, follow your derivation and record the difference in
`deviations`.

1. `m2_flagship_extrude_p_minus_q_congruent_boolean_difference`:
   build all three solids; `boolean(&solid_a, Difference, &solid_b, ...)`
   is Ok with one shell and 7 faces; Extrude(P−Q) has one shell and 7
   faces; the face-set bijection holds class by class (decision 3);
   the Difference wall's effective normal at a dyadic wall point has
   NEGATIVE dot with the outward radial direction there (toward the
   axis — the landed `boolean_difference_flagship_assembles_the_plate_
   with_hole` is the in-crate template for this check); cite the
   measured grid (208/208, per-point 256/256) in a comment.
2. `m2_intersection_is_extrude_q_with_outward_wall`: Ok, 3 faces; the
   bijection against Extrude(Q); the wall's effective normal has
   POSITIVE dot with the outward radial direction (unflipped).
3. `m2_union_commutative_both_orders_match_extrude_p`: both orders Ok,
   8 faces each, the measured census; each order's four sides biject
   with Extrude(P)'s sides, and each order's two annuli have outer
   wires equal to the block's cap wires (the annulus+disk pair tiles the
   square cap: the annulus's hole wire and the disk's wire are the same
   unit circle); the two orders' face sets biject with each other; the
   pair-dedup provenance difference (which side's fragment was emitted)
   is explicitly NOT asserted.
4. `m2_self_pair_refuses_the_typed_envelope`:
   `boolean(&solid_a, Union, &solid_a, ...)` and
   `boolean(&solid_a, Difference, &solid_a, ...)` both return
   `Err(Refusal::UnsupportedEnvelope(EnvelopeCase::ContactReductionDeferred))`
   (the `matches!` form `boolean_refuses_multishell_input` uses); a
   comment documents the fold per decision 5.

## House form (H-3)

This crate is under the kernel's house rules. Any ADDED line with a bare
`1e-N` float literal must end `// H-3`; prefer dyadic values,
`TAU`/`std::f64::consts`, or named constants (a named `TOL` for face
identification is the split.rs test module's form). GATE-2 scans the
diff. Run `bash scripts/kernel-gates.sh <your base commit>` before
writing RESULT.json - a failing gate is a finding to report, never one to
work around.

## Done when

```console
cargo fmt --check -p truck-shapeops
cargo clippy -p truck-shapeops --all-targets --no-deps
cargo check --locked -p truck-shapeops --all-targets
cargo test -p truck-shapeops --test boolean_m2 --no-fail-fast
cargo test -p truck-shapeops --lib boolean --no-fail-fast
bash scripts/kernel-gates.sh <your base commit>
```

Never run bare `cargo test` or a workspace-wide cargo command. All
twenty-two pre-existing boolean tests (nine split + six classify + three
material-state + four assemble) must stay green.

**Commit your work on the current branch** (subject
`shapeops: the M2 cross-layer flagship and metamorphic battery (BG-SOL-M2-WITNESS)`)
**before** writing `RESULT.json`: the verifier measures the committed
diff, and an uncommitted tree reads as an interrupted run.

## Forbidden

Editing anything outside `write_allow` (assemble.rs, split.rs,
classify.rs and mod.rs are READ-ONLY for you — the entry and stages are
consumed, never changed); renaming or deleting a pre-existing test;
adding a self-pair guard, fast path, or hand-built self-pair event list
(decision 5); importing ray-casting/containment oracle machinery (the
bijection is the criterion); `#[ignore]`; loosening a gate; changing the
GATE-4 ceiling; widening any tolerance to make a test pass; editing
`truck-evidence` or `truck-modeling`.

## Stop conditions

- anchor mismatch -> `ANCHOR_MISMATCH` with observed count;
- a booked congruence cannot be realized as specified (a bijection class
  finds no match, a face count disagrees) -> `SPEC_GAP` with both
  derivations;
- a self-pair run STOPS refusing (an entry change landed after this
  packet was written) -> `SPEC_GAP`: report the new outcome, do not
  silently re-assert the refusal;
- three consecutive cargo failures with one cause -> `BLOCKED`.

Finish by writing `RESULT.json` in the worktree root, not
`loop/results/`. Record in `notes`: the four runs' outcomes, whether the
bijection found any surprise (a face class with an extra or missing
member), and any tolerance-class decision that surprised you.
