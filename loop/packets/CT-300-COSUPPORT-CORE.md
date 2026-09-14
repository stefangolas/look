# WORK PACKET CT-300-COSUPPORT-CORE — the planar coincidence certificate, Theorem 6 table, and the three-way oracle (P4)

Build-spec Wave 3. The production unlock: abutting spline-loft interfaces
share planar support, which the transversality gate can never admit
(σ_min = 0). This packet implements the co-support oracle's core: a
certified planar coincidence test from control data (interval evaluation
of the supporting-plane equation over each control hull, plus the
parameter-rectangle overlap decomposition), the Theorem 6 orientation
table, and the three-way answer contract. Partial-curve contact and
reparameterized coincidence remain `Unknown` — refused typed until Wave 4
gives them a geometric treatment. Fold/admission integration is CT-310.

```yaml
id:          CT-300-COSUPPORT-CORE
contract:    [CT-300-COSUPPORT-CORE]
class:       design
crates:      [truck123d]
depends_on:  [CT-000-ATLAS-CONTRACT-SHIM]
needs:       [CT-000-ATLAS-CONTRACT-SHIM]
write_allow:
  - truck123d/src/atlas/cosupport.rs
  - truck123d/tests/atlas_cosupport.rs
read_allow:
  - docs/CONTACT_ATLAS_SPEC.md
  - docs/THEORY_GAPS_ADMISSION_CONTACT.md
tests_required: [truck123d/tests/atlas_cosupport.rs]
anchors:
  - {id: A1, min: 1,  cmd: "grep -cE '\\bmod atlas\\b' truck123d/src/lib.rs"}
  - {id: A2, min: 5,  cmd: "grep -cE '\\bclassify_point\\b' truck123d/src/bd_bridge.rs"}
budget:      {turns: 45, ctx_tokens: 140000}
```

Anchors at authoring time `e271ba9`. **Re-measure at dispatch.**

## Frozen contracts

1. **Planar coincidence certificate (the sufficient condition)**: two
   patch regions are certified co-supported on a shared planar interface
   when (a) interval evaluation of each region's supporting-plane
   equation over the region's control hull proves planarity with a
   common plane (all control points on one plane, outward-rounded); (b)
   the parameter-rectangle overlap decomposes into finitely many
   rectangles; (c) orientations are read from the certified normals
   (parallel vs antiparallel by interval dot product of plane normals).
2. **Theorem 6 table applied on Resolved regions**: the shared region's
   flux contribution is exactly zero (union/intersection, antiparallel;
   difference, parallel) or exactly one copy with common orientation —
   in the zero cases the region EXITS the refinement queue; in the
   one-copy cases the shared region contributes the single patch's plain
   flux, counted once.
3. **The three-way answer is total and honest**:
   `NoPositiveAreaOverlap | Resolved | Unknown`. Uncertifiable
   coincidence (non-planar, curved-curve contact, reparameterized
   overlap) ⇒ `Unknown` ⇒ `CoincidentSupportUncertified` refusal at
   integration. **Never guess**: an incorrect Resolved is the one way
   this certificate could break soundness — the planarity test's interval
   margins are part of the certificate record.
4. **Refinement fallback unaffected**: `Unknown` regions keep the
   existing path (subdivision under the scheduler); the oracle only
   REMOVES resolved regions from it.

## Done-when

1. `cargo test -p truck123d --test atlas_cosupport --locked` green: two
   coplanar abutting boxes resolve with antiparallel orientation
   (union contributes zero on the interface); two coplanar
   parallel-oriented faces resolve per the table; a rotated non-planar
   pair answers `Unknown`; disjoint pairs answer
   `NoPositiveAreaOverlap`; determinism of the overlap decomposition.
2. fmt clean; `clippy -p truck123d --lib` clean on the diff.
3. Write `RESULT.json` AT THE WORKTREE ROOT (nowhere else), including
   which production refusal classes (from the R4 taxonomy) this
   certificate covers vs leaves Unknown.
