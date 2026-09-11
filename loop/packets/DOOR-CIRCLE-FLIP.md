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
  - truck123d/tests/door_circle_flip.rs
read_allow:
  - truck123d/src/bd_bridge.rs
  - truck123d/src/facade.rs
  - corpus/ttc/trees/falcon_heavy/src/lib/merlin_common.py
tests_required: [truck123d/tests/door_circle_flip.rs]
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'a circle profile is not answered exactly' corpus/ttc/door.py"}
  - {id: A2, expect: 0, cmd: "grep -c 'def circle' corpus/ttc/door.py"}
budget:      {turns: 40, ctx_tokens: 120000}
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
