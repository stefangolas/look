# WORK PACKET DOOR-PARTIAL-ARC-FLIP — pass through the landed partial-arc revolve

SWARM FINDING 2026-09-10: the partial-arc revolve refuses at
door.py:1406 ("a partial-arc revolve is outside the executor's lathe
arm") while the operation is LANDED in the facade (PB-014, DONE). Two
corpus rows block on it: turbine_exhaust (70-degree shield) and
cutaway (270-degree sections). This is a pure pass-through flip.

```yaml
id:          DOOR-PARTIAL-ARC-FLIP
contract:    [DOOR-PARTIAL-ARC-FLIP]
class:       mechanical
crates:      [truck123d]
depends_on:  []
write_allow:
  - corpus/ttc/door.py
  - truck123d/src/bd_bridge.rs
  - truck123d/tests/door_partial_arc_flip.rs
  - truck123d/tests/ttc_lathe_spline.rs
read_allow:
  - truck123d/src/facade.rs
tests_required: [truck123d/tests/door_partial_arc_flip.rs]
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'a partial-arc revolve is outside the executor' corpus/ttc/door.py"}
budget:      {turns: 55, ctx_tokens: 180000}
```

## QUESTION.md AMENDMENT (2026-09-11, first run SPEC_GAP — adjudicated)

The first run STOPPED with SPEC_GAP: the "PB-014 partial-arc op is
landed" claim is true of the facade CLASSIFIER LEDGER only — the
executor has NO partial-arc lathe arm (the same facade-ledger-vs-
executor conflation as the circle flip). The operator's recommended
amendment is adopted; write set widened accordingly:

1b. Add the executor partial-arc lathe arm in `bd_bridge.rs` (the
    lathe carrier records/realizes `revolution_arc` + `start_angle`
    exactly; sites named in the escalation: bd_bridge.rs:163-169,
    530-538, 7001).
1c. `truck123d/tests/ttc_lathe_spline.rs:250-255` pins the old
    partial-arc refusal — move the pin deeper-or-answered.
1d. The door arm (original judgement 1) stays, now backed by the
    executor arm; full-360 rows serialize unchanged.

## Pre-made judgements

1. The door arm records `revolution_arc` and `start_angle` verbatim in
   the lathe row (the kernel PB-014 op consumes them); profile
   discipline is UNCHANGED (the existing lathe profile gates apply).
2. `revolution_arc: 360.0` rows serialize exactly as today — no
   behavior change on the already-green path.
3. If the facade's landed partial-arc op rejects a field the door
   sends, STOP with QUESTION.md — a field-contract mismatch is a
   finding, not something to paper over.

## Method

Flip the :1406 refusal per judgement 1; test with a 270-degree lathe
and a 70-degree shield-shaped lathe, both green facts, plus one
unanswerable angle option refusing typed. Cargo through the queue; DLL
workaround on PATH if 0xc0000135.

## Done when

Scoped checks green; anchors hold (A1 -> 0). Write RESULT.json AT THE
WORKTREE ROOT.
