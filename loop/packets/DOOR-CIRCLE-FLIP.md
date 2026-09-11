# WORK PACKET DOOR-CIRCLE-FLIP — the circle-profile recording arm

SWARM FINDING 2026-09-10 (highest lever in the corpus): `bd.Circle`
profiles are refused at door.py:823 ("a circle profile is not answered
exactly by a kernel-engine row") while the kernel side is COMPLETE
(PB-014 circle carrier, S6 sweep with closed sections). One flip
unblocks 9 corpus rows: 8 falcon_heavy rows via the Merlin `tube()`
idiom (spline spine + closed Circle section, merlin_common.py:245-249)
plus f1/cockpit's ~30 disc-stack bodies.

```yaml
id:          DOOR-CIRCLE-FLIP
contract:    [DOOR-CIRCLE-FLIP]
class:       mechanical
crates:      [truck123d]
depends_on:  []
write_allow:
  - corpus/ttc/door.py
  - truck123d/src/bd_bridge.rs
  - truck123d/tests/door_circle_flip.rs
  - truck123d/tests/ttc_lathe_spline.rs
read_allow:
  - truck123d/src/facade.rs
  - corpus/ttc/trees/falcon_heavy/src/lib/merlin_common.py
tests_required: [truck123d/tests/door_circle_flip.rs]
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'a circle profile is not answered exactly' corpus/ttc/door.py"}
  - {id: A2, expect: 0, cmd: "grep -c 'def circle' corpus/ttc/door.py"}
budget:      {turns: 65, ctx_tokens: 200000}
```

## QUESTION.md AMENDMENT (2026-09-10, first run STOPPED — adjudicated)

The first run STOPPED at judgement 3 with QUESTION.md, correctly: the
executor's section vocabulary `ProfileEdge` (bd_bridge.rs:85-93)
carries only Line/Spline — there is NO circle carrier behind
`bd_facts`. The swarm's "PB-014 kernel carrier is complete" claim is
true of the facade CLASSIFIER LEDGER only (FacadeOp::Circle,
facade.rs:386 — records, computes no geometry). Landing the door arm
alone would turn a typed refusal into a generic serde error
(`parse_tree` at bd_bridge.rs:5804 maps EVERY serde error to
`Refusal::Empty`). A landed test pins the old refusal
(ttc_lathe_spline.rs:493-522). Amended judgements:

1b. Add `ProfileEdge::Circle { center: [f64;3], radius: f64 }` with
    EXACT handling in `spline_loop_spans` / `profile_loop` /
    `loft_sections` / `loft_volume` / `loft_bbox` / `loft_mesh`
    (analytic extrema, deterministic tessellation) and
    `reflect_profile_edge` if the mirror arm touches section edges.
1c. Degrade-proof `parse_tree` minimally: carry the serde error text
    in the refusal so vocabulary gaps are NAMED, not generic.
1d. AMEND THE PIN at ttc_lathe_spline.rs:493-522: after the carrier
    lands, the mvac sweep case answers or refuses with the NEW
    carrier's vocabulary — move the pin deeper-or-answered, never
    delete the discipline.

```

## Pre-made judgements

1. `bd.Circle(radius)` records as an EXACT analytic section carrier
   (the PB-014 kernel carrier), not a sampled spline — no
   tessellation, no interpolation. Circle options outside the recorded
   exact vocabulary refuse typed naming the option.
2. The refusal at door.py:823 is replaced by the recording arm;
   `tube(section, path)` then records through the EXISTING sweep
   loft-chain arm unchanged — this packet changes the SECTION
   vocabulary only, never the sweep mechanics.
3. The kernel bridge already carries the circle carrier; if a bridge
   gap surfaces (the section JSON needs a `"kind": "circle"` variant),
   STOP with QUESTION.md rather than approximating — that would be a
   bridge finding, not a door problem.

## Method

Replace the :823 refusal with the arm per judgement 1; trace one
`tube()` call end to end through `bd_facts` before writing the rest.
Tests: a tube (spline spine + circle section) records green facts; a
cockpit-style disc stack records; a Circle with an unanswerable option
refuses typed. All cargo through the queue; DLL workaround on PATH for
the test binary if 0xc0000135.

## Done when

Scoped checks green (`cargo check -p truck123d --lib --locked` + the
new test file serially); anchors hold (A1 -> 0, A2 -> 1). Write
RESULT.json AT THE WORKTREE ROOT.
