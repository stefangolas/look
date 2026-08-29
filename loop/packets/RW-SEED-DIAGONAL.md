---
id: RW-SEED-DIAGONAL
class: design
crates: [truck-shapeops]
write_allow:
  - vendor/truck/truck-shapeops/src/boolean/assemble.rs
  - vendor/truck/truck-shapeops/src/boolean/split.rs
  - vendor/truck/truck-shapeops/src/boolean/classify.rs
  - vendor/truck/truck-shapeops/tests/cut_boundaries.rs
tests_required:
  - diagonal_plane_box_splits
  - diagonal_recombination_is_original
  - no_certified_endpoints_still_refuses
  - overlapping_union_unchanged
budget: {turns: 50, ctx_tokens: 150000}
---

# RW-SEED-DIAGONAL — the vertex-touch chain end to end: filter, dedupe, seeding

Program: the vertex-touch cut family (P3 test 3, diagonal plane x + y = 2
through a 2×2×2 box's opposite edges), after two instrumented stops that
peeled the chain to its last layer. Read
`loop/results/RW-VERTEX-CLIP.STOP-r1.json` and `.STOP-r2.json` first —
they are the bisect evidence, and both runs' mechanism work was PROVEN by
instrumentation before being reverted (the tree is green at base
`7463b68`; nothing of D1/D2/D6/D7 is landed — you re-land the whole
chain). Everything below is pre-decided; churn, don't design. Contradiction
with the tree = `SPEC_GAP`.

## The chain (all four links, in landing order)

1. **Filter (assemble.rs, the r1/r2 D1).** The seam-record filter at
   `assemble.rs:273` drops an `EndpointTouch` record ONLY when the event
   is NOT cross-solid (the `event_cross_solid` ptr-eq discriminator,
   split.rs:1108); cross-solid EndpointTouch records flow through. The
   `Arc1 Coincident` arms and the RW-RESEW zero-measure filters are
   byte-identical to base.
2. **Collection (split.rs, the r1/r2 D2).** `collect_events` collects
   cross-solid EndpointTouch Point0 loci as certified clipping points
   alongside Transverse ones; the `(Point, EndpointTouch, Point0)`
   dispatch arm re-routes to `point_cut` in the Point phase instead of
   refusing (the same-solid arm still refuses — the landed lib test
   `split_deferred_locus_family_refuses` must stay green).
3. **Two-point certification + dedupe (split.rs, the r2 D6/D7).**
   `insert_open_arc_shared` builds an arc only when the certified-extreme
   set holds at least TWO DISTINCT (near_pt-separated) points — a single
   certified point is a corner touch riding the seam_skip path as at base
   (this is what keeps resew 7/7 byte-for-byte). An open arc witnessed by
   multiple FF records dedupes at insertion time per face: skip if an arc
   with the same carrier AND the same certified extent (endpoints
   near_pt-equal, same order or reversed) is already pending (first
   instance is the shared instance).
4. **Seeding (classify.rs, the r2 third class — THE NEW WORK).** After
   links 1-3 the diagonal SPLIT succeeds (r2-verified) and the refusal
   moves to the classifier: `find_seed` (classify.rs:277 at the r2 tree)
   finds no seedable fragment in a mesh component and returns
   `NumericallyUnresolved { witness: UncertifiedContainment }` (the
   "no fragment in this component yields a region representative" arm).

## Anchors (measured 2026-08-29 at HEAD `7463b68`; re-derive before writing RESULT.json)

| id | file | pattern | count |
|----|------|---------|-------|
| A1 | vendor/truck/truck-shapeops/src/boolean/assemble.rs | `let seam = matches!\(record.kind, ContactEventKind::EndpointTouch\)` | 1 |
| A2 | vendor/truck/truck-shapeops/src/boolean/split.rs | `fn event_cross_solid\(&self` | 1 |
| A3 | vendor/truck/truck-shapeops/src/boolean/split.rs | `fn insert_open_arc_shared\(` | 1 |
| A4 | vendor/truck/truck-shapeops/src/boolean/split.rs | `fn assemble_pending_loops\(&mut self` | 1 |
| A5 | vendor/truck/truck-shapeops/src/boolean/classify.rs | `fn find_seed\(` | 1 |
| A6 | vendor/truck/truck-shapeops/src/boolean/split.rs | `ContactLocus::Point\(_\), ContactEventKind::EndpointTouch` | 1 |

All anchors are the PRE-packet tree; no new module is declared, so none
diverges.

## Decisions already made for you

**D1 — links 1-3 are re-landed verbatim from the r1/r2 derivations.** Both
stops recorded machine-verified implementations (the r2 RESULT's
contracts section is the specification; the STOP-r1/r2 deviations carry
the derivations). Re-land them exactly; the machine-check obligations
carry over (resew 7/7 byte-for-byte, boolean_m2 4/4, interior_loop 8/8,
`split_deferred_locus_family_refuses` green).

**D2 — diagnosis-first on the seeding layer.** Your first mandated task:
instrument `find_seed` on the diagonal mesh and record WHICH component
fails, WHICH fragment candidates exist, and WHY each representative fails
to resolve (degenerate polygon? no region representative? the
seed-fallback rule not firing?). Remove the instrumentation before
commit. The r2 backtrace is your starting point:
`numerically_unresolved classify.rs:852 <- find_seed classify.rs:277 <-
classify_fragments classify.rs:98`.

**D3 — the seeding invariant (the fix rule).** Every fragment-mesh
component must be seedable by exactly one of the landed rules (arc-side
seed for components touching contact arcs; ray-parity seed for
contact-free components). Two pre-decided sub-rules, in order:

- **(a) The seed-fallback rule must fire across components.** Session 37
  landed a fallback: when a fragment's region representative does not
  resolve (degenerate polygon), the seed comes from the LOWEST-INDEX
  fragment whose representative resolves. Machine-check whether that
  fallback is per-component or global: if per-component and the diagonal
  mesh has a component where EVERY fragment is degenerate-representative,
  the fallback cannot fire — that is the bug class to look for first.
- **(b) A component whose every fragment is genuinely unrepresentable**
  (no polygon resolves at any index) refuses
  `NumericallyUnresolved(UncertifiedContainment)` — the typed refusal
  stays, and `no_certified_endpoints_still_refuses` (D5) asserts it
  against a fixture that ACTUALLY has that property (machine-check which
  fixture does; the diagonal is no longer one after your fix).

No new seed RULES (no new classification machinery): you fix the
application of the landed rules to this mesh shape. If the mesh genuinely
needs a new rule (a seed source that is neither arc-side nor ray-parity),
STOP — that is a SPEC_GAP with the component's fragment census as
evidence.

**D4 — the diagonal acceptance.** Tests 1-2 flip to the booked happy
path: two valid 5-face triangular prisms from the 2×2×2 box and plane
x + y = 2 (three exact dyadic points, planes compared by data); the
recombination `boolean(plus, Union, minus)` is a valid single shell,
box-equal exactly, face count asserted AS OBSERVED with the 10-face
pre-decision comment (the RW-RESEW pre-decision — no coplanar merging).

**D5 — the guard test re-pointed honestly.** The landed
`no_certified_endpoints_still_refuses` fixture IS the diagonal Difference,
whose recorded refusal dissolves once the family works (the r2 stop's
finding). Keep the test NAME (the session-34 identity rule), re-point it
at a fixture that genuinely exercises the refusal TODAY, and assert the
mechanism you observe. Machine-check FIRST which fixture has the property;
candidates to try: an oblique-plane cut of the cylinder wall (the
RW-CONIC family — but machine-check its actual refusal arm), or whatever
your D2 diagnosis surfaces. If NO fixture in the envelope still refuses
with that class, record the finding and assert the closest surviving
class, documenting the substitution — the test's job is to pin a REAL
refusal, not a remembered one.

**D6 — zero new arms; the landed suites are the regression wall.** No new
`Refusal`/`EnvelopeCase`/`UnresolvedWitness`/`Collapse` arms. boolean_m2,
interior_loop, resew pass UNCHANGED. A happy-path fixture answering
`Contradictory` after a faithful implementation is a STOP condition.

## Template

- `vendor/truck/truck-shapeops/src/boolean/classify.rs` — the classifier:
  `find_seed`, the seed-fallback rule, `region_representative`, and the
  numerically_unresolved arm (A5).
- `vendor/truck/truck-shapeops/src/boolean/assemble.rs:262-281` and
  `vendor/truck/truck-shapeops/src/boolean/split.rs` — links 1-3's
  landing sites.
- `loop/results/RW-VERTEX-CLIP.STOP-r1.json` and `.STOP-r2.json` — the
  instrumented bisect (tracked; committed before this fork).
- `vendor/truck/truck-shapeops/tests/cut_boundaries.rs` — the landed
  fixtures; the diagonal tests flip (D4), the guard re-points (D5).

## Tests required (edit the landed `tests/cut_boundaries.rs` in place; no new file)

1. `diagonal_plane_box_splits` — flipped per D4: two valid 5-face
   triangular prisms.
2. `diagonal_recombination_is_original` — flipped per D4: valid single
   shell, box-equal exactly, observed face count.
3. `no_certified_endpoints_still_refuses` — re-pointed per D5: same name,
   a fixture that genuinely refuses today, observed mechanism asserted
   with the derivation recorded.
4. `overlapping_union_unchanged` — UNCHANGED (the envelope guard).

## H-3 (house rule; V4 is a text gate on your diff)

No ADDED line carries a bare absolute small literal (`1e-N`) without the
same-line `// H-3` opt-out. Run `& "C:\Program Files\Git\bin\bash.exe"
scripts/kernel-gates.sh HEAD` before writing RESULT.json (bare `bash` is
the WSL stub). CLIPPY YOUR TEST FILE TOO — run
`cargo clippy --locked -p truck-shapeops --tests` and make every changed
file clean BEFORE committing.

## Done when

Commit on the current branch (subject
`RW-SEED-DIAGONAL: the vertex-touch chain - filter, dedupe, seeding`)
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

- Do not edit `boolean/mod.rs` or anything outside `write_allow`.
- Do not add `Refusal`/`EnvelopeCase`/`UnresolvedWitness`/`Collapse` arms
  (D6).
- Do not weaken or rescope any landed test; do not edit other test files.
- Do not touch the `Arc1 Coincident` arms of the :273 filter or the
  RW-RESEW zero-measure record filters.
- GATE-3/H-4: `Face::debug_new` is banned in added lines.
- Do not write the tolerance-migration constructor name in prose or comments
  (GATE-4 counts raw text, comments included).
- No instrumentation traces may survive in the committed diff.

## Stop conditions

- `ANCHOR_MISMATCH` — report the measured count, change nothing.
- `SPEC_GAP` — a decision contradicts the tree; QUESTION.md with the
  empirical proof.
- The diagonal mesh needs a seed source that is neither arc-side nor
  ray-parity (D3) — stop with the component's fragment census.
- A happy-path fixture answers `Contradictory` after a faithful
  implementation — stop, report the witness verbatim.

RESULT.json: `{"id":"RW-SEED-DIAGONAL","status":"DONE","contracts":[...],
"tests_added":4,"deviations":[...],"notes":"..."}` — the D2 diagnosis
(find_seed's failure mode on the diagonal mesh) goes in notes; every
deviation with your derivation; deviations are expected to be RIGHT.
