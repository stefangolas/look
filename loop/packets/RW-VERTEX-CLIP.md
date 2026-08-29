---
id: RW-VERTEX-CLIP
class: design
crates: [truck-shapeops]
write_allow:
  - vendor/truck/truck-shapeops/src/boolean/assemble.rs
  - vendor/truck/truck-shapeops/src/boolean/split.rs
  - vendor/truck/truck-shapeops/tests/cut_boundaries.rs
tests_required:
  - diagonal_plane_box_splits
  - diagonal_recombination_is_original
  - no_certified_endpoints_still_refuses
  - overlapping_union_unchanged
budget: {turns: 30, ctx_tokens: 100000}
---

# RW-VERTEX-CLIP — cross-solid EndpointTouch points become certified clipping points

Program: the D2 remainder of RW-DIVIDE-NESTING, whose landing
(`loop/results/RW-DIVIDE-NESTING.json`, deviation `D2_BLOCKED_AT_SWEEP`)
localized the blocker exactly: the sweep's seam-record filter at
`assemble.rs:273` drops EVERY `EndpointTouch` record before the splitter,
so a cut through a solid's edge graph (P3 test 3, the diagonal plane
x + y = 2 through a 2×2×2 box's opposite edges) has no certified clipping
points and the open-arc guard refuses. This packet is small by design:
one filter arm changes, one collection arm changes, the diagonal family
unblocks. Everything below is pre-decided; churn, don't design.
Contradiction with the tree = `SPEC_GAP`.

## Anchors (measured 2026-08-29 at HEAD `819fe9b`; re-derive before writing RESULT.json)

| id | file | pattern | count |
|----|------|---------|-------|
| A1 | vendor/truck/truck-shapeops/src/boolean/assemble.rs | `let seam = matches!\(record.kind, ContactEventKind::EndpointTouch\)` | 1 |
| A2 | vendor/truck/truck-shapeops/src/boolean/assemble.rs | `seam records the splitter cannot act` | 1 |
| A3 | vendor/truck/truck-shapeops/src/boolean/split.rs | `fn event_cross_solid\(&self` | 1 |
| A4 | vendor/truck/truck-shapeops/src/boolean/split.rs | `ContactLocus::Point\(_\), ContactEventKind::EndpointTouch` | 1 |
| A5 | vendor/truck/truck-shapeops/src/boolean/split.rs | `fn collect_events\(&mut self` | 1 |

All anchors are the PRE-packet tree; no new module is declared, so none
diverges.

## Decisions already made for you

**D1 — the filter's arm switches from record-kind to cross-solid-ness.**
The :273 filter's EndpointTouch arm exists to protect the M2 self-pair
from zero-measure seam touches (the comment at A2 is the history). The
RW-RESEW landing already built the correct discriminator:
`event_cross_solid` (A3, ptr-eq shell identity) decides whether an event
is between two DISTINCT inputs. The change: an `EndpointTouch` record is
dropped ONLY when the event is NOT cross-solid (the self-pair case);
cross-solid `EndpointTouch` records flow through to the splitter. The
`Arc1 Coincident` arms of the filter keep their current behavior EXACTLY
(machine-check which inputs reach them; do not touch their condition).

**D2 — the splitter collects the points.** `collect_events` (A5) gains
the cross-solid `EndpointTouch` Point loci as certified clipping points
alongside the Transverse ones; the dispatch arm at A4 currently REFUSES
`(Point, EndpointTouch, Point0)` — re-route it to the Point phase's
`point_cut` (the same treatment Transverse points get) instead of
refusing. `point_cut`'s no-interior-edge case (`Ok(())` when the point is
already a boundary vertex) is the path a box-vertex crossing takes — the
cut is a no-op there, and the OPEN-ARC clipping is where the point does
its work (it lands in `certified_points`, `open_arc_certified` (A2's
neighbour) reads it, `canonical_vertex`'s tolerance dedupe guarantees no
double-mint — verified by the DIVIDE-NESTING worker, split.rs:603-615).

**D3 — what you must NOT break.** The M2 self-pair keeps refusing
(`boolean_m2` unchanged, the ptr-eq guard is the mechanism); the RW-RESEW
zero-measure record filters (degenerate arcs, EE collinear coincidences)
keep their own arms — you change ONLY the EndpointTouch handling; the
`no_certified_endpoints_still_refuses` guard in the landed
`tests/cut_boundaries.rs` must keep passing UNCHANGED in its assertion
(the class is: an open FF locus whose certified-extreme set is empty —
machine-check the fixture still exercises it after your change; if the
fixture's refusal class dissolves, that is a STOP condition finding, not
something to re-purpose the test around).

**D4 — the landed diagonal tests flip from refusal to acceptance.** The
landed `tests/cut_boundaries.rs` tests 1-2 (`diagonal_plane_box_splits`,
`diagonal_recombination_is_original`) currently assert the recorded
SPEC_GAP3 refusal verbatim; after your change they assert the booked
happy path (two valid 5-face triangular prisms; the recombination a valid
single shell, box-equal exactly, face count asserted as observed with the
10-face pre-decision comment). Same test NAMES, flipped assertions — this
is the booked unblocking, not a gate loosening; say so in the commit
message. The landed tests 3-5 (annulus/2-hole) and 7 (overlapping union)
do not change.

**D5 — zero new arms.** No new `Refusal`/`EnvelopeCase`/
`UnresolvedWitness`/`Collapse` arms. A happy-path fixture answering
`Contradictory` after a D1/D2-faithful implementation is a STOP condition
(report the witness verbatim).

## Template

- `vendor/truck/truck-shapeops/src/boolean/assemble.rs:262-281` — the
  filter (A1/A2) and its history comment.
- `vendor/truck/truck-shapeops/src/boolean/split.rs` — `collect_events`
  (A5), the A4 dispatch arm, `event_cross_solid` (A3), the open-arc
  clipping machinery (`open_arc_certified`, `insert_open_arc_shared`,
  `assemble_pending_loops`) that consumes the points.
- `loop/results/RW-DIVIDE-NESTING.json` deviation `D2_BLOCKED_AT_SWEEP` —
  the localization evidence (tracked; committed before this fork).
- `vendor/truck/truck-shapeops/tests/cut_boundaries.rs` — the landed
  fixtures you flip (D4) and the guard you preserve (D3).

## Tests required (edit the landed `tests/cut_boundaries.rs` in place; no new file)

1. `diagonal_plane_box_splits` — flipped per D4: two valid 5-face
   triangular prisms from the 2×2×2 box and the plane x + y = 2 (built
   from three exact dyadic points, compared by data).
2. `diagonal_recombination_is_original` — flipped per D4: the prisms
   recombine to a valid single shell, box-equal exactly.
3. `no_certified_endpoints_still_refuses` — UNCHANGED assertion (D3); if
   its fixture's class dissolves, STOP and report.
4. `overlapping_union_unchanged` — UNCHANGED (the envelope guard).

## H-3 (house rule; V4 is a text gate on your diff)

No ADDED line carries a bare absolute small literal (`1e-N`) without the
same-line `// H-3` opt-out. Run `& "C:\Program Files\Git\bin\bash.exe"
scripts/kernel-gates.sh HEAD` before writing RESULT.json (bare `bash` is
the WSL stub). CLIPPY YOUR TEST FILE TOO — run
`cargo clippy --locked -p truck-shapeops --tests` and make every changed
file clean BEFORE committing (two prior packets each lost a verify round
to an unused const).

## Done when

Commit on the current branch (subject
`RW-VERTEX-CLIP: cross-solid EndpointTouch points clip open FF loci`)
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

`boolean_m2`, `interior_loop`, and `resew` must pass UNCHANGED. The lib
suite's `healing::tests::step_import` failure is the recorded
environmental one (fails at base too).

## Forbidden

- Do not edit `boolean/mod.rs`, `boolean/classify.rs`, or anything outside
  `write_allow`.
- Do not add `Refusal`/`EnvelopeCase`/`UnresolvedWitness`/`Collapse` arms
  (D5).
- Do not touch the `Arc1 Coincident` arms of the :273 filter or the
  RW-RESEW zero-measure record filters (D3).
- GATE-3/H-4: `Face::debug_new` is banned in added lines.
- Do not write the tolerance-migration constructor name in prose or comments
  (GATE-4 counts raw text, comments included).
- No instrumentation traces may survive in the committed diff.

## Stop conditions

- `ANCHOR_MISMATCH` — report the measured count, change nothing.
- `SPEC_GAP` — a decision contradicts the tree; QUESTION.md with the
  empirical proof.
- A happy-path fixture answers `Contradictory` after a D1/D2-faithful
  implementation — stop, report the witness verbatim.
- The guard fixture (test 3) refuses for a DIFFERENT reason than the
  recorded one after your change — stop and report; the envelope may have
  moved in a way the packet did not decide.

RESULT.json: `{"id":"RW-VERTEX-CLIP","status":"DONE","contracts":[...],
"tests_added":2,"deviations":[...],"notes":"..."}` — tests_required lists
the four flipped/preserved names; every deviation with your derivation;
deviations are expected to be RIGHT.
