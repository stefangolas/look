---
id: RW-DIVIDE-NESTING
class: design
crates: [truck-shapeops]
write_allow:
  - vendor/truck/truck-shapeops/src/boolean/split.rs
  - vendor/truck/truck-shapeops/src/boolean/classify.rs
  - vendor/truck/truck-shapeops/tests/cut_boundaries.rs
tests_required:
  - diagonal_plane_box_splits
  - diagonal_recombination_is_original
  - annulus_section_face
  - annulus_split_assembles
  - two_hole_section_face
  - no_certified_endpoints_still_refuses
  - overlapping_union_unchanged
budget: {turns: 50, ctx_tokens: 140000}
---

# RW-DIVIDE-NESTING — cuts through the edge graph + nested interior loops

Program: the two follow-ups booked by BG-CAD-P3-SPLIT's second stop
(`loop/results/BG-CAD-P3-SPLIT.SPEC_GAP3.json` — read it first; its
derivations are the bisect evidence). Both fixes live in the same division
machinery in `truck-shapeops/src/boolean/split.rs`, hence one packet (the
plan's same-module compression rule). Everything below is pre-decided;
churn, don't design. Contradiction with the tree = `SPEC_GAP`.

## Problem

BG-CAD-P3-SPLIT's implementation is complete (archived at
`packet/BG-CAD-P3-SPLIT@7361b7a`) and its axial flagship family works, but
two booked happy paths refuse:

1. **The diagonal cut through box vertices** (P3 test 3, plane x + y = 2
   through a 2×2×2 box's opposite edges): every FF line's certified crossing
   is a box vertex; the sweep emits NO Transverse Point events there (the
   touching edges are `EndpointTouch`, filtered as zero-measure seam records
   at the split.rs:966 arm; the lying edges are `CoincidentInterval`), so
   the open-arc guard refuses — an open locus with no certified endpoints.
2. **Two nested interior loops on one face** (P3 test 5, the annulus
   section): the halfspace wall carries the footprint rect loop AND the
   hole circle; `divide_one_face`'s negative-wire attachment is
   FIRST-containing, so the circle hole attaches to the wrong region, the
   nesting malforms, and the assembled shell fails `NotClosedShell`.

## Anchors (measured 2026-08-29 at HEAD `19a3408`; re-derive before writing RESULT.json)

| id | file | pattern | count |
|----|------|---------|-------|
| A1 | vendor/truck/truck-shapeops/src/boolean/split.rs | `fn sew_completion\(&mut self` | 1 |
| A2 | vendor/truck/truck-shapeops/src/boolean/split.rs | `fn open_arc_certified\(&self` | 1 |
| A3 | vendor/truck/truck-shapeops/src/boolean/split.rs | `fn insert_open_arc_shared\(` | 1 |
| A4 | vendor/truck/truck-shapeops/src/boolean/split.rs | `fn assemble_pending_loops\(&mut self` | 1 |
| A5 | vendor/truck/truck-shapeops/src/boolean/split.rs | `fn divide_one_face\(` | 1 |
| A6 | vendor/truck/truck-shapeops/src/boolean/split.rs | `ContactLocus::Point\(_\), ContactEventKind::EndpointTouch` | 1 |

All anchors are the PRE-packet tree; no new module is declared, so none
diverges.

## Decisions already made for you

**D1 — reproduce first.** Write `tests/cut_boundaries.rs` with the two
refusing fixtures (the diagonal-plane box split via the P3 two-call recipe
with the PADDED over-box, and the plate-with-hole section at z = 1),
asserting the observed refusals. Then implement D2-D3 and flip the tests.

**D2 — cuts through the edge graph (finding 1).** The certified clipping
points EXIST — as `EndpointTouch` Point records the pipeline currently
filters. The canonical rule:

> A Transverse OR EndpointTouch `Point` locus lying on an open FF locus's
> curve is a certified clipping point for that locus.

Concretely: collect EndpointTouch Point loci alongside Transverse ones in
the event-collection pass (machine-check what the current filter at A6
actually drops — SPEC_GAP3 says the sweep filters some EndpointTouch
records BEFORE the splitter; if the points are gone by collection time,
the fix is at the sweep's seam-record filter, keeping the M2 self-pair
guards of RW-RESEW's deviation 1 intact — the ptr-eq cross-solid guard
decides, not the record kind alone). The open-arc path then clips at the
certified endpoints: an arc whose two extreme crossings are existing
vertices is inserted between the canonical vertices (the arc edge is a NEW
shared instance; its endpoints are the EXISTING canonical vertices — no
new vertices are minted for them). The `open_arc_certified` gate (A2)
keeps refusing open interior arcs with NO certified endpoints on ANY face
(test 6 pins this). A cut whose crossing is a box vertex must not
duplicate that vertex: `canonical_vertex` already dedupes by tolerance —
verify it, and if the seam-filter removal makes a Point record reachable
that canonical_vertex would double-mint, that is a machine-check finding,
record it.

**D3 — nested interior loops (finding 2).** `divide_one_face`'s
negative-wire attachment becomes MINIMAL-CONTAINING (the session-28
containment/nesting rule transplanted): each negative (interior) wire
attaches as the hole of the SMALLEST-AREA pre-face region whose outer
polygon contains it, instead of the first. Ties (equal area) resolve to
lowest region index; record the rule in a comment. The invariant: a wire
strictly inside another wire's region never attaches to an ancestor
region. Machine-check the annulus case by hand first (the footprint rect
chord loop attaches to the outer border region; the hole circle — strictly
inside the rect region — attaches to the RECT region as its hole), and
require the two-hole generalization to fall out (test 5: three nested
wires, no hand-pairing).

**D4 — the classifier adjudicates; you add no classification machinery.**
If a happy-path fixture answers `Refusal::Contradictory` after a
D2/D3-faithful implementation, STOP and report the witness verbatim. If
classify.rs genuinely needs no change, change nothing (the write_allow
covers it defensively only).

**D5 — the v1 envelope.** In scope: open FF loci with certified endpoints
on the face boundary INCLUDING existing vertices; interior-loop nesting to
any depth on one face. Still refusing as today: arcs with no certified
endpoints, `ValidatedBranchCover`, `Tangency`, `Parabola`/`Hyperbola`,
partial-angle circles, multi-shell, self-pairs (boolean_m2 unchanged).
Zero new `Refusal`/`EnvelopeCase`/`UnresolvedWitness`/`Collapse` arms — a
perceived need is a SPEC_GAP.

**D6 — acceptance.** Tests 1-5 are P3's booked happy paths 3 and 5 plus
their generalizations; tests 1-2 pin the diagonal prism family (two valid
5-face prisms; the recombination is box-equal — the union keeps 10-ish
cosmetically-split faces per the RW-RESEW pre-decision, so assert the
observed face count and the exact box, never the 6). Tests 6-7 are the
envelope guards.

## Template

- `vendor/truck/truck-shapeops/src/boolean/split.rs` — the whole surface:
  the A6 filter, `open_arc_certified` (A2), `insert_open_arc_shared` (A3),
  `assemble_pending_loops` (A4), `divide_one_face` (A5), and
  `sew_completion` (A1, whose cross-solid guards you must not weaken).
- `vendor/truck/truck-shapeops/src/boolean/mod.rs:286` area — the landed
  material-state tests (do not edit; read for the touching-pair behavior).
- `loop/results/BG-CAD-P3-SPLIT.SPEC_GAP3.json` — the derivations (tracked;
  committed before this packet's fork).
- `vendor/truck/truck-shapeops/tests/resew.rs` — the fixture conventions
  (padded over-box recipe, `expect_ok` helper style); do not edit it.

## Tests required (new file `tests/cut_boundaries.rs`, dyadic witnesses only)

1. `diagonal_plane_box_splits` — 2×2×2 box, plane x + y = 2 (norm
   (1,1,0)/√2, built from three exact dyadic points; compare planes by
   data): two triangular prisms, each valid, 5 faces each.
2. `diagonal_recombination_is_original` — the two prisms recombine:
   `boolean(plus, Union, minus)` is a valid single shell, box-equal to the
   original exactly; face count asserted AS OBSERVED with the 10-face
   pre-decision in a comment.
3. `annulus_section_face` — the plate-with-hole (4×4×2, hole r=1 at (2,2))
   sectioned at z = 1 via the P3 two-call recipe: exactly 1 section face
   with 2 boundary wires, exact plane data.
4. `annulus_split_assembles` — the same split's both halves are valid
   solids; box equality against the hand-derived halves.
5. `two_hole_section_face` — plate with holes r=0.5 at (1,1) and (3,3),
   sectioned at z = 1: exactly 1 section face with 3 boundary wires (the
   D3 generalization; no hand-pairing of wires anywhere in the test).
6. `no_certified_endpoints_still_refuses` — an open FF locus with NO
   certified endpoints on a face's boundary still refuses (the D2 guard;
   machine-check which fixture actually produces this — the RW-INTERIOR-
   LOOP notes describe the class).
7. `overlapping_union_unchanged` — a genuinely overlapping pair still
   assembles with its landed face counts (the envelope guard, the
   `tests/resew.rs` convention).

## H-3 (house rule; V4 is a text gate on your diff)

No ADDED line carries a bare absolute small literal (`1e-N`) without the
same-line `// H-3` opt-out. Dyadic constants and the existing named consts
only. Run `& "C:\Program Files\Git\bin\bash.exe" scripts/kernel-gates.sh
HEAD` before writing RESULT.json (bare `bash` is the WSL stub). CLIPPY
YOUR OWN TEST FILE TOO — the last two packets each lost a verify round to
an unused const in the new test file; run
`cargo clippy --locked -p truck-shapeops --tests` and make the new file
clean BEFORE committing.

## Done when

Commit on the current branch (subject
`RW-DIVIDE-NESTING: vertex-touch clipping + minimal-containing loop nesting`)
BEFORE writing RESULT.json AT THE WORKTREE ROOT (that exact path — not
`loop/results/`), then, all green:

```
cargo check --locked -p truck-shapeops
cargo fmt --check -p truck-shapeops
cargo test --locked -p truck-shapeops --lib
cargo test --locked -p truck-shapeops --test cut_boundaries
cargo test --locked -p truck-shapeops --test boolean_m2
cargo test --locked -p truck-shapeops --test interior_loop
cargo test --locked -p truck-shapeops --test resew
cargo clippy --locked -p truck-shapeops --all-targets
& "C:\Program Files\Git\bin\bash.exe" scripts/kernel-gates.sh HEAD
```

`boolean_m2`, `interior_loop`, and `resew` must pass UNCHANGED (outside
write_allow). The lib suite's `healing::tests::step_import` failure is the
recorded environmental one (STEP fixtures absent; fails at base too).

## Forbidden

- Do not edit `boolean/mod.rs`, `boolean/assemble.rs`, or anything outside
  `write_allow`.
- Do not add `Refusal`/`EnvelopeCase`/`UnresolvedWitness`/`Collapse` arms
  (D5).
- Do not weaken or rescope any landed test; do not edit existing test files.
- Do not weaken the `sew_completion` cross-solid guards (A1) — the M2
  self-pair must keep refusing.
- GATE-3/H-4: `Face::debug_new` is banned in added lines.
- Do not write the tolerance-migration constructor name in prose or comments
  (GATE-4 counts raw text, comments included).
- No instrumentation traces may survive in the committed diff.

## Stop conditions

- `ANCHOR_MISMATCH` — report the measured count, change nothing.
- `SPEC_GAP` — a decision contradicts the tree; QUESTION.md with the
  empirical proof.
- A happy-path fixture (tests 1-5) answers `Contradictory` after a
  D2/D3-faithful implementation (D4) — stop, report the witness verbatim.
- The EndpointTouch records are unreachable at the splitter without
  touching `assemble.rs` (the sweep filter lives there, outside your
  write set) — stop and report; do not widen your own write set.

RESULT.json: `{"id":"RW-DIVIDE-NESTING","status":"DONE","contracts":[...],
"tests_added":7,"deviations":[...],"notes":"..."}` — every deviation with
your derivation; deviations are expected to be RIGHT.
