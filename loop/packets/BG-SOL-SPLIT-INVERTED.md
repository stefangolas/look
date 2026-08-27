# WORK PACKET BG-SOL-SPLIT-INVERTED - the inverted-face division and sew-direction fixes

The RW4 pre-dispatch prototype (`scratch/rw3probe`, preserved) ran the REAL
M2 flagship event set - SIX events (top AND bottom: FF circle, FE
BoundedCurve rim, Region2 cap-coincidence at each of z=2 and z=0) - through
the landed splitter and found TWO latent defects that no landed test can
see: the landed flagship test divides only the TOP face, and the two defects
cancel each other's visibility in every mesh that does not assemble. Every
number in this packet is measured by the probe (which carried a patched copy
of split.rs, `split_fixed`, byte-identical except the two fixes below, and
ran the full six-event pipeline: split -> classify -> decide -> sew ->
`Solid::try_new` -> containment/grid comparison against `Extrude(P-Q)`).
If live code contradicts this packet, report it in `disagreements`.

This packet dispatches AFTER BG-SOL-RW3-CLASSIFY lands (its anchors are
derived against the post-RW3 tree; the six boolean split tests AND the six
classify tests must be green at your fork point).

```json
{"id":"BG-SOL-SPLIT-INVERTED","status":"DONE","contracts":["BG-SOL-SPLIT-INVERTED"],
 "tests_added":2,"sites_migrated":0,"sites_deferred":0,
 "deviations":[],"disagreements":[],"baseline_failures":[],"notes":"free text"}
```

```yaml
id:          BG-SOL-SPLIT-INVERTED
contract:    [BG-SOL-SPLIT-INVERTED]
class:       design
crates:      [truck-shapeops]
write_allow:
  - vendor/truck/truck-shapeops/src/boolean/split.rs
read_allow:
  - vendor/truck/truck-shapeops/src/boolean/split.rs
  - vendor/truck/truck-shapeops/src/boolean/classify.rs
  - docs/SOLVER_FAMILY_PLAN.md
tests_required:
  - split_six_event_flagship_bottom_face_divides_like_the_top
  - split_sewn_rim_directions_preserve_effective_traversals
budget:      {turns: 40, ctx_tokens: 140000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub mod classify' vendor/truck/truck-shapeops/src/boolean/mod.rs"}
  - {id: A2, expect: 3, cmd: "ls vendor/truck/truck-shapeops/src/boolean | wc -l"}
  - {id: A3, expect: 7, cmd: "grep -cF '#[test]' vendor/truck/truck-shapeops/src/boolean/split.rs"}
  - {id: A4, expect: 6, cmd: "grep -cF '#[test]' vendor/truck/truck-shapeops/src/boolean/classify.rs"}
  - {id: A5, expect: 1, cmd: "grep -c 'pub fn split_fragments' vendor/truck/truck-shapeops/src/boolean/split.rs"}
  - {id: A6, expect: 1, cmd: "grep -c 'let is_region' vendor/truck/truck-shapeops/src/boolean/split.rs"}
  - {id: A7, expect: 1, cmd: "grep -cF 'fn build_closed_loop_wire' vendor/truck/truck-shapeops/src/boolean/split.rs"}
  - {id: A8, expect: 1, cmd: "grep -cF 'fn prepare_contained_wire' vendor/truck/truck-shapeops/src/boolean/split.rs"}
  - {id: A9, expect: 1, cmd: "grep -c 'FragmentInsideOther' vendor/truck/truck-base/src/evidence.rs"}
```

A3 becomes 9 (the two new tests); all others stay.

## Problem

The M2 flagship is the plate-with-hole difference: the block extrude minus
the coaxial disk extrude. Its REAL event set touches BOTH horizontal faces
of the block - and the bottom face of every extruded solid is an INVERTED
face (`orientation() == false`) whose stored wires are still CCW. Two
independent defects make the six-event mesh unusable:

1. **The bottom face divides into garbage.** Measured at HEAD: the bottom
   face's two fragments have wire structures `[2, 2]` (a doubled-loop disk
   with two wires) and `[4]` (the square with NO hole wire); the mesh has
   only 2 Flip adjacencies (the top's); the bottom coincident pair's `a`
   side is the `[4]` fragment; the classifier then finds no region
   representative for the doubled-loop fragment and refuses.
2. **The sewn rim directions are rotated.** Measured at HEAD (this is
   visible in the LANDED top-only three-event mesh too): b's top-cap
   fragment's effective boundary traverses its circle CW - the fragment is
   born geometrically invalid (its effective normal points at +z but the
   region is on the right of travel) - and the wall's effective top wire
   traverses CCW. Relative cap/wall orientation survives (both rotated), so
   no landed assertion fails; but the coincident disk fragment and the wall
   end up traversing the shared rim halves in the SAME effective direction,
   so no Boolean result that keeps disk+wall (Intersection) or
   annulus+flipped-wall (Difference) can ever close. The probe measured:
   Union assembles (wall discarded) and Intersection/Difference both refuse
   `NotClosedShell` at HEAD.

## Decisions already made for you

### 1. Fix one - `divide_one_face` classifies by the STORED-frame sign

Replace

```rust
let is_region = |area: f64| {
    if face.orientation() {
        area > 0.0
    } else {
        area < 0.0
    }
};
```

with

```rust
// The stored-frame outer-positive invariant: for a valid face the STORED
// outer wire is always CCW-positive in the surface's (u, v) frame,
// independent of the orientation flag. Derivation: the effective outer
// wire is CCW around the face's effective normal; the (u, v) frame is
// right-handed around the surface's own normal; inverting a face flips
// the effective-normal side AND inverts every effective wire, and the
// loops hold the STORED wires, so the flag-dependent test below
// double-counted the flag and inverted region/hole for every divided
// inverted face (the extruded bottom cap: stored CCW, flag false).
let is_region = |area: f64| area > 0.0;
```

The polygon handed to `is_region` is computed by `create_parameter_boundary`
from the loops, which `SplitEngine::new` fills from
`face.absolute_boundaries()` - the STORED wires (`absolute_boundaries`
returns `&self.boundaries` unchanged; `Face::boundaries()` is the
orientation-adjusted one). Zero-area wires (the band-form wall) are
`area > 0.0 == false` under both the old and the new rule, so the wall's
path is unchanged.

### 2. Fix two - the sew cut normalizes to the edge's FORWARD traversal

In `build_closed_loop_wire`, the sew stratum names the edge AS USED in one
face - `edge_from_ref` returns the `Edge` object found in the named face's
`absolute_boundaries()`, which for the flagship's top rim is the WALL's
INVERSE use. `cut_edge_to_arc` on that object yields halves in that use's
direction, and `swap_edge_into_wire` then hands the FORWARD uses
(the cap's) the inverse-direction wire - rotating every use's effective
traversal. Normalize before the swap:

```rust
if let Some((edge, range)) = self.sew_edge_for(solid, face_idx, exact) {
    let halves = self.cut_edge_to_arc(&edge, range).ok_or_else(unsupported)?;
    let wire = Wire::from(halves);
    // The sew stratum names the edge AS USED in one face (possibly the
    // inverse use); cutting that object yields halves in that use's
    // direction. Normalize to the edge's FORWARD traversal so
    // `swap_edge_into_wire` (forward use -> wire, inverse use ->
    // wire.inverse()) preserves EVERY use's original effective traversal.
    let wire = if edge.orientation() { wire } else { wire.inverse() };
    self.swap_edge_into_wire(edge.id(), &wire);
    return Ok(wire);
}
```

Apply the SAME normalization in `prepare_contained_wire` after the
`cut_with_parameter` (the contained face's front edge may equally be an
inverse use; for the flagship's bottom rim it is already forward, so this
half is a no-op there - it is the defensive twin of the fix):

```rust
let (e0, e1) = edge
    .cut_with_parameter(&vertex, t)
    .ok_or_else(numerically_unresolved)?;
let mut halves = Wire::from(vec![e0, e1]);
// Same normalization as build_closed_loop_wire: the halves wire carries
// the edge's FORWARD traversal so every use keeps its own.
if !edge.orientation() {
    halves = halves.inverse();
}
self.swap_edge_into_wire(edge.id(), &halves);
return Ok(halves);
```

Nothing else changes. The doubled loop still enters as the (wire,
wire.inverse()) pair; the fragments still keep the parent's stored frame
and flag (`Face::new_unchecked` + the flag-preserving invert); the
a-side disk/annulus wires are UNCHANGED by fix two (the probe measured the
same disk/annulus wires before and after - only the cap and the wall
rotate back to their original directions), which is why the landed
top-only flagship test and the classifier's flagship bits are unaffected.

### 3. The measured post-fix six-event mesh (your test 1 asserts this)

11 fragments; a's bottom face divides EXACTLY like the top: the annulus
`[4, 2]` (square + hole of the two rim half-edges) and the disk `[2]`;
20 adjacency = 4 Flip (two per rim, disk<->annulus) + 16 Same (a's 12:
each annulus<->4 sides + sides<->sides 4; b's 4: wall<->each cap 2); 2
coincident pairs, both `Identical`, `{a: bottom disk, b: b's bottom cap}`
and `{a: top disk, b: b's top cap}`. The classifier's bits over this mesh
measure `[false, true, false, true, false, false, false, false, true, true, true]`
(annuli F, disks T, sides F, b's three T). The probe then assembled all
four ops: Union 8 faces, Intersection 3, Difference 7, Xor 7 - all four
`Solid::try_new` Ok - and the Difference result matched `Extrude(P-Q)` on
six containment probes and a 256-point grid (208/208). You do NOT assemble
anything in this packet; those numbers are the evidence that the two fixes
are the complete repair, quoted so you can sanity-check the mesh you
produce.

## Tests required

Dyadic witnesses throughout (H-3); copy the construction helpers from the
test module (`placed_circle`, `block_profile`, `disk_profile`,
`extrude_shell`, `plane_face_at_z`, `cylinder_face`, `flat_edge_at_z`,
`ev`, `ff_curve_record`); `TOL = 1.0e-2`. Identify fragments by ORIGIN +
wire structure, never by raw index without derivation, and write the
expected-value derivation as inline comments (the BG-NUM-002 rule).
Machine-check every number before asserting; where your derivation and
this packet disagree, follow your derivation and record the difference in
`deviations`.

1. `split_six_event_flagship_bottom_face_divides_like_the_top`: the
   flagship inputs and the SIX events - for EACH of z=2 and z=0: the FF
   circle (a's face at z x b's wall), the FE `BoundedCurve` (a's face at z
   x b's wall rim edge at z), and the Region2 `Coincident` (a's face at z
   x b's cap at z), built exactly like the landed top-only test's three
   events. Assert the decision-3 mesh: 11 fragments; the bottom face's two
   fragments carry wire structures `[4, 2]` and `[2]` (derivable by
   symmetry with the landed top-face assertions); 20 adjacency with 4 Flip
   (each Flip entry is a disk<->annulus pair) and 16 Same; adjacency is
   same-solid only; 2 coincident pairs, both `Identical`, pairing each
   DISK fragment (the `[2]` one) with the cap at the same z; the two rim
   half-edge instances at EACH rim are shared (EdgeID identity, compared
   as unordered pairs like the landed test) across the disk fragment, the
   annulus's hole wire, the wall's wire at that z, and the cap's wire.
2. `split_sewn_rim_directions_preserve_effective_traversals`: the LANDED
   top-only three-event mesh (copy the landed flagship test's events
   verbatim). Let `disk` be a's top-face `[2]` fragment, `annulus` the
   `[4, 2]` one, `cap` b's top-cap fragment, `wall` b's wall fragment.
   Using each face's EFFECTIVE wires (`face.boundaries()`), assert:
   (i) for every shared rim half-edge id, the disk's use orientation
   equals the cap's (the coincident pair, same effective normal, same
   traversal); (ii) the annulus's hole-wire use orientation equals the
   wall's top-wire use (they were opposite in b's original closed shell
   and the wall is what Difference flips); (iii) the disk's use
   orientation differs from the annulus's hole-wire use (the doubled
   loop). Derive all three from the sewing-oracle contract (every use
   keeps its original effective traversal) in the comments before
   asserting.

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

Never run bare `cargo test` or a workspace-wide cargo command. All sixteen
pre-existing boolean tests (ten split + six classify) must stay green.

**Commit your work on the current branch** (subject
`shapeops: inverted-face division and sew-direction fixes in the splitter (BG-SOL-SPLIT-INVERTED)`)
**before** writing `RESULT.json`: the verifier measures the committed
diff, and an uncommitted tree reads as an interrupted run.

## Forbidden

Editing anything outside `write_allow`; changing any split.rs behavior
beyond the two decisions (the exports RW3 added stay untouched); renaming
or deleting a pre-existing test; a `_` wildcard arm in any `Surface` or
`ContactLocus` match; `#[ignore]`; loosening a gate; changing the GATE-4
ceiling; adding classification, assembly, or entry logic (RW4's scope, NOT
yours); calling `truck_evidence::contact::contact()`.

## Stop conditions

- anchor mismatch -> `ANCHOR_MISMATCH` with observed count;
- a booked fix cannot be realized as specified -> `SPEC_GAP` with the
  compile error and your proposed shape;
- the six-event mesh your run produces differs from decision 3's measured
  numbers in a way the fixes do not explain -> `SPEC_GAP` with both
  derivations;
- three consecutive cargo failures with one cause -> `BLOCKED`.

Finish by writing `RESULT.json` in the worktree root, not
`loop/results/`. Record in `notes`: the measured six-event mesh numbers,
which pre-existing tests you ran green, and whether the probe's
predictions (`scratch/rw3probe`, preserved, `split_fixed` module) matched
your implementation.
