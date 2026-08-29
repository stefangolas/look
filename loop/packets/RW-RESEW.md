---
id: RW-RESEW
class: design
crates: [truck-shapeops]
write_allow:
  - vendor/truck/truck-shapeops/src/boolean/split.rs
  - vendor/truck/truck-shapeops/src/boolean/classify.rs
  - vendor/truck/truck-shapeops/src/boolean/assemble.rs
  - vendor/truck/truck-shapeops/tests/resew.rs
tests_required:
  - adjacent_boxes_union_assembles
  - p3_recombination_flagship_is_original
  - touching_difference_answers
  - partial_seam_refuses
  - disjoint_union_unchanged
  - overlapping_union_unchanged
  - butt_join_survives_further_boolean
budget: {turns: 45, ctx_tokens: 130000}
---

# RW-RESEW — face-adjacent union: sew-completion across distinct results

Program: the Boundary Rewrite follow-up booked by BG-CAD-P3-SPLIT's session-41
stop (`loop/results/BG-CAD-P3-SPLIT.SPEC_GAP2.json` — read it first) and
foreshadowed by RW-INTERIOR-LOOP's RESULT deviation 5
(`loop/results/RW-INTERIOR-LOOP.json`). Everything below is pre-decided;
churn, don't design. Contradiction with the tree = `SPEC_GAP`.

## Problem

`boolean(A, Union, B)` refuses with
`UnsupportedEnvelope(ContactReductionDeferred)` whenever A and B are
face-adjacent (share a boundary patch with opposite orientations). Isolated
empirically (SPEC_GAP2 derivation (b)): two DIRECTLY-BUILT adjacent boxes
refuse identically to the split-recombination case — the limitation is in the
landed pipeline, not in any construction. Mechanism: the sweep DOES see the
coincident cap pair (the material states discard it correctly), but the two
solids' seam-boundary edges are DISTINCT Edge instances — no coedge pairs
across the seam — so the kept shell is two connected components and the
multi-component fold (`assemble.rs:96-99`) refuses.

The FE sewing oracle already collects exactly the records that witness the
seam (an edge of one solid lying on a face of the other), but it only REUSES
an edge when the face's split produces a boundary along its carrier — and a
butt-join produces no cuts at all. The sew records are collected and then
never consumed.

## Anchors (measured 2026-08-29 at HEAD `07dc0fa`, post-RW-INTERIOR-LOOP; re-derive before writing RESULT.json)

| id | file | pattern | count |
|----|------|---------|-------|
| A1 | vendor/truck/truck-shapeops/src/boolean/split.rs | `fn collect_sew\(&mut self` | 1 |
| A2 | vendor/truck/truck-shapeops/src/boolean/split.rs | `fn run\(&mut self` | 1 |
| A3 | vendor/truck/truck-shapeops/src/boolean/split.rs | `fn split_fragments\(` | 1 |
| A4 | vendor/truck/truck-shapeops/src/boolean/split.rs | `fn migrate_contact_on_cut\(` | 1 |
| A5 | vendor/truck/truck-shapeops/src/boolean/classify.rs | `Refusal::Contradictory` | 3 |
| A6 | vendor/truck/truck-shapeops/src/boolean/assemble.rs | `pub fn boolean\(` | 1 |
| A7 | vendor/truck/truck-shapeops/src/boolean/assemble.rs | `connected_components` | 1 |

All anchors are the PRE-packet tree; no new module is declared, so none
diverges.

## Decisions already made for you

**D1 — reproduce first, on the exact SPEC_GAP2 fixtures.** Write
`tests/resew.rs` first with the two refusing calls (adjacent boxes
`[0,4]²×[0,1]` + `[0,4]²×[1,2]` under `Union`; and the split-recombination
pair from SPEC_GAP2's derivation (a)), asserting the observed refusal. Then
implement D2-D4 and flip them to the acceptance assertions.

**D2 — the sew-completion pass (the fix).** After the three phase passes
(`run`, A2) and before the mesh is returned, run a completion pass over the
collected sew records: for every FE `BoundedCurve` record whose edge was NOT
consumed by a face split or a point cut, certify that the arc's exact curve
is carrier-and-range IDENTICAL to a boundary edge of the face-side solid
(exact `Line`/`Circle`/`Ellipse` identity — the collect_sew carrier gate
already restricts to these), and UNIFY the instance: replace every use of
the edge-side's edge with the face-side's edge object (or vice versa — pick
ONE direction, say which), normalizing orientation first (session-38 rule:
the edge as named in one face may be the inverse use; normalize to the
forward traversal before the swap, `if !edge.orientation() { wire =
wire.inverse(); }` discipline). After the pass, every seam edge pair is ONE
shared instance used by both solids' fragment wires with OPPOSITE effective
orientations (the proper-manifold butt-join). Reuse the landed
wire-mutation/instance-substitution primitives (`swap_edge_into_wire`,
`migrate_contact_on_cut`, A4) — no new topology machinery.

- A record whose arc matches NO boundary edge of the face-side solid: skip
  (the pre-existing reuse path owns consumed records; an unconsumed record
  with no exact partner is a genuine T-shape/partial contact — see D3).
- A record whose arc matches MORE THAN ONE boundary edge: ambiguous seam →
  refuse `unsupported()` (the deferred envelope).
- Multi-edge seams (the adjacent boxes share 4 rim edges) unify edge by
  edge; the pass is idempotent per record.

**D3 — the v1 envelope.** In scope: seams where each shared locus is the
FULL range of a boundary edge on BOTH sides (the butt-join). Out of scope
and refusing as today: T-junctions and partial-range seams (one side's edge
meets the middle of the other's — test 4 pins the refusal), vertex-only
contacts, coincident-VOLUME overlaps of faces that are not boundary patches
(the Region2 material-state machinery already owns those — do not touch
it). The multi-component fold (A7) stays EXACTLY as it is: after D2 a
butt-join closes as one component; anything still multi-component is
genuinely out of envelope and must keep refusing.

**D4 — the classifier adjudicates.** With unified seam edges the parity
graph sees the seam as a Flip-parity adjacency (opposite material sides).
The landed seed-and-propagate rules decide the kept set; a
`Refusal::Contradictory` on a D2-faithful implementation is a STOP
condition (report the witness verbatim), not something to hand-patch. If
the ray-parity seeding needs no change, change nothing in classify.rs.

**D5 — zero new arms.** No new `Refusal`/`EnvelopeCase`/`UnresolvedWitness`/
`Collapse` arms; `unsupported()` (ContactReductionDeferred) remains the
envelope boundary. A perceived need is a SPEC_GAP.

**D6 — the acceptance metamorphic (the P3 unblock).** Test 2 is P3's booked
test verbatim: split the flagship 4×4×2 plate at z = 1 by the two-call
recipe (`boolean(S, Difference, minus_box)` / `boolean(S, Intersection,
minus_box)` per the P3 packet's D3/D4, built in the test), then
`boolean(plus, Union, minus)` is face-count- and box-equal to S. The
adjacent-boxes case (test 1) additionally asserts 6 faces and the exact
merged box.

## Template

- `vendor/truck/truck-shapeops/src/boolean/split.rs` — the splitter (A1-A4):
  the sew oracle's collection (`collect_sew`), the phase runner, the
  instance-substitution primitives, and the session-38 orientation rule.
- `vendor/truck/truck-shapeops/src/boolean/assemble.rs` — the entry (A6)
  and the multi-component fold (A7) you must NOT need to change.
- `vendor/truck/truck-shapeops/src/boolean/mod.rs:286` — the landed
  material-state coincident-fragment test (what already works for the cap
  pair; read before D2).
- `loop/results/BG-CAD-P3-SPLIT.SPEC_GAP2.json` and
  `loop/results/RW-INTERIOR-LOOP.json` (deviation 5) — the bisect evidence
  (tracked; in your worktree).

## Tests required (new file `tests/resew.rs`, dyadic witnesses only)

1. `adjacent_boxes_union_assembles` — the SPEC_GAP2 derivation (b) fixture:
   two directly-built adjacent boxes union to ONE valid solid, 6 faces, box
   `[0,4]²×[0,2]` exactly.
2. `p3_recombination_flagship_is_original` — the D6 metamorphic on the
   flagship plate (both halves assemble per SPEC_GAP2's derivation (a);
   their union is face-count- and box-equal to S).
3. `touching_difference_answers` — `boolean(box0, Difference, box1)` and the
   Intersection analogue on the touching pair: machine-check FIRST what the
   post-D2 pipeline answers (the touching face has zero-measure overlap;
   the material states should keep A whole) and assert exactly that answer,
   recording the observed arm in notes. Do not pre-decide the arm in the
   assertion before you have observed it.
4. `partial_seam_refuses` — an L-shaped pair sharing only PART of a face
   (e.g. `[0,2]²×[0,2]` + `[2,4]×[0,2]×[0,1]`): a typed refusal (machine-
   check which arm; the expected family is the deferred envelope or a
   Contradictory — assert what you observe and record it).
5. `disjoint_union_unchanged` — two far-apart boxes under Union still refuse
   at the multi-component fold (the A7 guard is untouched).
6. `overlapping_union_unchanged` — a genuinely overlapping pair (the M2
   flagship family) still assembles with its landed face counts (guards the
   envelope).
7. `butt_join_survives_further_boolean` — the test-1 result
   downstream-consumes: `boolean(merged, Difference, small_box)` assembles a
   valid solid.

## H-3 (house rule; V4 is a text gate on your diff)

No ADDED line carries a bare absolute small literal (`1e-N`) without the
same-line `// H-3` opt-out. Dyadic constants and the existing named consts
only. Run `& "C:\Program Files\Git\bin\bash.exe" scripts/kernel-gates.sh
HEAD` before writing RESULT.json (bare `bash` is the WSL stub).

## Done when

Commit on the current branch (subject
`RW-RESEW: sew-completion unifies seam edge instances across distinct results`)
BEFORE writing RESULT.json, then, all green:

```
cargo check --locked -p truck-shapeops
cargo fmt --check -p truck-shapeops
cargo test --locked -p truck-shapeops --lib
cargo test --locked -p truck-shapeops --test resew
cargo test --locked -p truck-shapeops --test boolean_m2
cargo test --locked -p truck-shapeops --test interior_loop
cargo clippy --locked -p truck-shapeops --all-targets
& "C:\Program Files\Git\bin\bash.exe" scripts/kernel-gates.sh HEAD
```

`boolean_m2` and `interior_loop` must pass UNCHANGED (both outside
write_allow; their passing proves the landed envelope did not regress).
NOTE: the lib suite's `healing::tests::step_import` fails at BASE and HEAD
(missing STEP fixtures in this checkout — recorded, environmental); it is
not yours and V5's baseline comparison knows it.

## Forbidden

- Do not edit `boolean/mod.rs` or anything outside `write_allow`.
- Do not add `Refusal`/`EnvelopeCase`/`UnresolvedWitness`/`Collapse` arms
  (D5).
- Do not weaken or rescope any landed test; do not edit existing test files.
- Do not touch the multi-component fold (D3) or the Region2 material-state
  machinery.
- GATE-3/H-4: `Face::debug_new` is banned in added lines.
- Do not write the tolerance-migration constructor name in prose or comments
  (GATE-4 counts raw text, comments included).
- No instrumentation traces may survive in the committed diff.

## Stop conditions

- `ANCHOR_MISMATCH` — report the measured count, change nothing.
- `SPEC_GAP` — a decision contradicts the tree; QUESTION.md with the
  empirical proof.
- A happy-path variant (tests 1-2) answers `Contradictory` after a
  D2-faithful implementation (D4) — stop, report the witness verbatim.
- The unification requires semantics beyond exact carrier-and-range
  identity (approximate matching, tolerance-scale decisions) — stop and
  report; that is a numerics decision the packet did not make.

RESULT.json: `{"id":"RW-RESEW","status":"DONE","contracts":[...],
"tests_added":7,"deviations":[...],"notes":"..."}` — every deviation with
your derivation; deviations are expected to be RIGHT.
