# BG-CAD-P3-SPLIT probe finding — the interior-loop division gap

Date: 2026-08-28, session 40. Probe: `scratch/contact_probe` (num3-scratch
discipline; deleted with its target after this filing — the bisect table below
is the record).

## The bisect (all through the landed `boolean()` entry, plate 4x4x2)

| variant | cutter | cutter caps vs plate caps | result |
|---|---|---|---|
| [i] | disk r=1 at (2,2), z in [0,2] (the exact M2 flagship) | COPLANAR | **OK** (1 shell) |
| [f] | same disk, z in [-1,3] (through, caps past) | not coplanar | REFUSES `ContactReductionDeferred` |
| [e] | same disk, z in [-2,1] (partway) | not coplanar | REFUSES |
| [g] | rect pocket x,y in [1,3], z in [-1,3] (through) | n/a | REFUSES |
| [h] | rect x,y in [1,3], z in [-4,1] (halfspace, no coplanar pair anywhere) | n/a | REFUSES |
| [a-d] | every box variant in the first probe run | various | REFUSES |

Plus: a pairwise `contact()` probe over every plate-face x box-face FF pair,
the FE vertical-edge x wall pair, and an EE pair — **every stratum pair
answers** (lines/points/empty, all landed cells). The Contact Layer is NOT the
deferrer.

## Diagnosis

The M2 milestone's verified Boolean envelope is exactly the case where the
cutter's termination faces are **coplanar with the solid's faces**: the caps
then split through the Region2 coincident-containment path, and the cutter
wall's rims ride the solid's cap planes as FE `BoundedCurve` sewing arcs. No
face is ever divided by an interior loop that arrives only as an FF Transverse
record.

The moment a cut wall terminates INTERIOR to the solid — i.e. every real
split, pocket, or partial Boolean — the solid's faces must be divided at
**interior contact loops** (closed loops inserted as free loops on a face that
has no coincident partner), and the cutter's wall must be divided at its
interior rim circles. That machinery is the booked-but-unlanded follow-up
family: session 37's recorded limitation ("a mesh whose fragments STRADDLE the
other solid's boundary — the through-hole family — a contact arc interior to
the other solid's unsplit carrier region ... the wall is not divided at
interior circles") and the plan's named follow-ups (RW-COPLANAR family,
plan §4 Phase 4).

`split_by_plane` (P3) sits squarely in this family: the halfspace box's wall
always terminates interior to the solid. **P3 is blocked on the interior-loop
division, not on any Contact Layer gap.**

## Next packet (booked)

**RW-INTERIOR-LOOP** (truck-shapeops `boolean/split.rs` + classify): divide
every face of BOTH solids at interior closed FF loops (the doubled-independent-
loop insertion exists for the coincident path — generalize its trigger to FF
Transverse closed loci), divide the cutter's wall at interior rim circles, and
let the classifier's parity verification (landed) adjudicate the resulting
mesh. Acceptance: variants [f] and [h] above assemble, and the split₊ ∪ split₋
≅ S metamorphic holds. Then BG-CAD-P3-SPLIT dispatches unchanged (its WIP is
archived at `loop/slots/0/abandoned-20260828-180102.patch`).
