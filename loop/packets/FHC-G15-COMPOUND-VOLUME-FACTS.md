# WORK PACKET FHC-G15-COMPOUND-VOLUME-FACTS — recursive certified volume over nested groups

R4 census finding (2026-09-14, `loop/nightly_ops/r4/`): four Falcon-Heavy
rows — `vehicle`, `cutaway`, `engine_prototype`, `mvac` — **construct and
mesh correctly** (2142 solids / 4.46M triangles / exact bboxes) but their
measured `volume` is `0.0`, so the oracle policy cannot judge them. The
cause is precisely located: the `Facts` path aggregates a group's volume
over its **immediate child parts only** and nested groups contribute
nothing (`bd_bridge.rs`, the `Facts::volume` doc comment, "immediate
child" sites). `vehicle` is a group whose immediate children are groups →
sum = 0.0. The aggregation machinery, the certified brackets, and the
per-child `rows` instrumentation all already exist — this packet widens
the aggregation to the full descendant tree.

```yaml
id:          FHC-G15-COMPOUND-VOLUME-FACTS
contract:    [FHC-G15-COMPOUND-VOLUME-FACTS]
class:       mechanical
crates:      [truck123d]
depends_on:  [FHC-G1-RATIONAL-FLUX]
needs:       [FHC-G1-RATIONAL-FLUX]
write_allow:
  - truck123d/src/bd_bridge.rs
  - truck123d/tests/compound_volume_facts.rs
read_allow:
  - docs/REFUSALS.md
  - docs/TTC_CENSUS_FINAL.md
tests_required: [truck123d/tests/compound_volume_facts.rs]
anchors:
  - {id: A1, expect: 6, cmd: "grep -cE '\\bvolume_bracket\\b' truck123d/src/bd_bridge.rs"}
  - {id: A2, expect: 4,  cmd: "grep -cE '\\bimmediate child\\b' truck123d/src/bd_bridge.rs"}
  - {id: A3, expect: 5,  cmd: "grep -cE '\\bRowFacts\\b' truck123d/src/bd_bridge.rs"}
budget:      {turns: 45, ctx_tokens: 140000}
```

Anchors measured 2026-09-14 at HEAD `7e77c33`. **Re-measure at dispatch** —
the R4 bookkeeping commits touched only `loop/` and `docs/`, but re-run
`gen_packet --check` anyway (H-8).

## Pre-made judgements (the worker churns, it does not design)

1. **Diagnose first, in the open.** Reproduce the 0.0 with a minimal nested
   construction tree (group of groups of boxes) through the same facts entry
   the door uses, and record the pre-packet output in `RESULT.json` notes.
   The fix site is the group-aggregation arm of the `Facts` path in
   `bd_bridge.rs` — do not touch the boolean solver, the flux certification,
   or the transversality gate.
2. **The new semantics: a group's `volume_bracket` is the interval sum over
   ALL descendant parts** (recursive; nested groups expand, parts
   contribute their own certified bracket). Interval addition is the
   composition rule — brackets stay sound, widths add. This WIDENS the
   current behavior (nested groups contributed nothing); the flat-group
   case (immediate children are parts) must remain bit-identical.
3. **Instrumentation is part of the deliverable** (owner directive): the
   `rows` breakdown must expose the nested structure — either one row per
   immediate child where a nested child carries its own recursive aggregate
   plus its own `rows` sub-list, or a flattened per-part list with group
   paths. Choose the minimal-change shape consistent with the existing
   `RowFacts` type and SAY WHICH in `RESULT.json`. Every part's row carries
   its own certified bracket.
4. **No loosening anywhere.** No tolerance change, no `#[allow]` without
   justification, no behavior change for flat groups, boolean nodes, or
   scalar parts. The boolean node's volume stays the PRODUCT's certified
   bracket (never an operand's).
5. **Empty group** (no descendant parts): volume bracket `[0, 0]` — an
   empty sum is a legitimate zero, not a refusal. Record the case in a
   test.

## Done-when

1. `cargo check -p truck123d --tests --locked` green.
2. `cargo test -p truck123d --test compound_volume_facts --locked` green,
   covering at minimum: flat group of two boxes (sum of analytic volumes,
   unchanged from pre-packet behavior), nested group of boxes (the new
   recursive sum — this test FAILS on the pre-packet tree), three-level
   nesting, mixed group (part + nested group), empty group. Float
   comparisons against analytic box volumes use exact-ish assertions with
   the H-3 same-line opt-out marker where needed.
3. `cargo fmt -p truck123d -- --check` clean on the touched files.
4. `cargo clippy -p truck123d --lib --locked` reports no diagnostic
   referencing the touched regions.
5. Write `RESULT.json` AT THE WORKTREE ROOT (nowhere else), including the
   pre-packet 0.0 reproduction and the anchor re-measurement.
