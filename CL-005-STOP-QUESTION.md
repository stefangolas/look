# CL-005-EXACT-CONTACT — STOP: SPEC_GAP (decision not derivable at the write-set boundary)

Status: STOPPED at the packet's stop condition #2 — *"a case's decision cannot be
derived from the certified event records alone → SPEC_GAP, naming the missing
certificate."* No RESULT.json is filed because no DONE path exists in this tree
within the packet's write set. Everything below was re-derived by command at
HEAD `7a7ff96` in this worktree; the raw probe transcripts are preserved at
`C:\Users\stefa\AppData\Local\Temp\opencode\cl_run3_out.txt` (full event +
mesh + bits traces) and `cl_run6_out.txt` (same under the experiment).

## Anchors (verified, all 1)

A1 `pub fn fragment_decision` boolean/mod.rs = 1; A2 `pub fn boolean(` boolean/
assemble.rs = 1; A3 `pub fn classify_fragments` boolean/classify.rs = 1;
A4 `ContactReductionDeferred` truck-base/evidence.rs = 1. No anchor mismatch.

## What was measured at HEAD (this tree)

Fixture kit (dyadic, `extrude_profile` + `translate_solid`, matching resew /
interior_loop / split_plane conventions). S = plate `[0,4]^2 x [0,2]`.

Case-(a) family, `boolean(_, Union, _)`:
- z-stacked full-face (recorded 10-split-face, resew test-1 class): **OK,
  10 faces** already at HEAD — no refusal exists to replace.
- P3 recombination `(S−pad) ∪ (S∩pad)`: **OK, 10 faces** already at HEAD.
- x-/y-adjacent full-face boxes (`xfull`/`yfull`, two 2-cubes sharing an exact
  face): **refuse inside the SPLITTER** (`split.rs::finish`, never reaching
  DECIDE). The seam is a Region2 `CoincidentInterval` between two vertical
  side faces; the z-adjacent equivalent is a Region2 `IdenticalCarrier`
  between two caps and splits fine.
- Partial seams (L-shape, offset): reach DECIDE (12-13 kept) then refuse at
  the multi-component fold — correctly out of scope (not a common face set).

Case-(b) family, `boolean(S, Difference|Intersection|Union, C)` with
C = `[0,4]^2 x [0,1]` (exact-footprint halfspace box, the recorded
`Contradictory(FragmentInsideOther)` trap):
- All three ops **refuse `Contradictory(FragmentInsideOther, left: False,
  right: True)`** in the classifier. The splitter succeeds and emits a
  20-fragment mesh with 5 coincident pairs, but the mesh is **not a proper
  tiling** of the coplanar container faces:
  - Per coplanar wall (A side z∈[0,2] containing B wall z∈[0,1]) the container
    face is left whole (fragment `a=2`, box z∈[0,2]) AND carries two further
    lower-band fragments (indices 3,4; 6,7; 9,10; 12,13) whose sampled
    geometry both cover z∈[0,1] and which share four `Flip` adjacency entries
    with each other (the mixed-instance twins the resew result documents).
  - The coincident pair couples the whole container fragment (a=2) to the
    contained B wall (b=16) — regions unequal, so `decide_and_assemble`'s
    pair rule would discard the whole container wall and lose its z∈[1,2]
    remainder. The container face was never split into "covered sub-region +
    complement".
- Padded control (P3 D3 recipe `[-1,5]^2 x [-1,1]`): Difference/Intersection
  **OK, 6 faces** at HEAD; the interior-loop machinery certifies because no
  wall is coplanar.
- M2 flagship (disk cutter with coplanar caps): OK at HEAD (7/8 faces).

## Why the two decisions cannot enter at the DECIDE/ASSEMBLE boundary

Scope decision 5 places the certified decisions at the DECIDE/ASSEMBLE
boundary, and the write set is `boolean/mod.rs` + `boolean/assemble.rs` +
`tests/cl_exact_contact.rs` (classify.rs and split.rs are explicitly frozen).

- Case (a): the refusing full-face butt joins (x/y axis) never produce a mesh;
  the refusal is inside `split.rs`. There is no fragment stream to decide, so
  no certified answer "whose content matches the recorded 10-split-face
  behavior" can be emitted from the DECIDE boundary for that class. The only
  full-face butt joins that DO reach DECIDE (z-axis) already certify at HEAD —
  for them no typed refusal exists to replace.
- Case (b): the refusal is a classifier `Contradictory`, but the mesh the
  classifier receives is not a proper face tiling of the coplanar containers
  (whole-container fragment + mixed-instance twins, unequal-region coincident
  pairs). No choice of classification bits and no decision rule over the
  existing coincident-pair records can make `decide_and_assemble` emit a
  closed valid shell from that mesh: the duplicate twin faces would both be
  emitted and the whole-container fragment either discarded wrongly or left
  covering the removed material. A certified answer is therefore **not
  derivable from the certified event records alone at this boundary**; it
  requires the containing face to be divided into covered-subregion +
  complement (and the twins consolidated), which is splitter/assembler-mesh
  work in files the packet forbids editing.

Experiment confirming the locus: deleting every Region2 `Coincident` event
before the split (`CL_FILTER=r2`) makes xfull and the exact-footprint
Difference/Union and exact-top-half Difference certify, but it also changes
the M2 flagship outputs (Difference 9 faces instead of 7, Union 6 instead of
8, Intersection of the exact box 2 faces instead of 6) and `corner_box_diff`
still refuses. The Region2 containment splits are load-bearing for the
strictly-interior containments (M2 disk-in-cap) and are only harmful for the
boundary-touching coplanar containment class — i.e. the distinction the fix
needs lives inside the splitter/classifier containment handling.

## Missing certificate named

A certified split/consolidation step (in `split.rs` or a mesh pass it must
own) that divides a containing face cleanly when the contained coplanar face's
region shares a positive-length boundary with the container (the
boundary-touching containment class) — producing one fragment per covered
sub-region plus the complement with shared canonical edge instances — and a
classifier rule that then stays consistent on that tiling. For case (a), the
same step must make the vertical (x/y) full-face coincident Region2 seam split
like its horizontal (z) twin. Both are outside `write_allow`
(`classify.rs`, `split.rs` are frozen; the only permitted boolena module files
cannot reach the choke points).

## Recommended amendment to unblock

Either (1) widen the CL-005 write set to `boolean/split.rs` (and possibly the
classifier's coincident-fragment handling), re-booking the two decisions as
containment-split + classification work; or (2) re-scope the packet to the
z-axis recorded unions and the padded controls (already certified at HEAD) as
pure battery rows plus the certified decision predicates, and leave the
x/y-full-face and exact-footprint classes booked behind the splitter work.
