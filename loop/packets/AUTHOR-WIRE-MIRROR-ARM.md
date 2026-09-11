# WORK PACKET AUTHOR-WIRE-MIRROR-ARM — wire/edge-level mirror recording

SWARM FINDING 2026-09-10: `bd.mirror(wire, Plane.YZ)` is refused at
door.py ("a mirror of this carrier is not a kernel-engine row") — the
mirror arm handles _Part/Compound only. This gates EVERY hypercar
master shell (aero, body, glazing, chassis, interior, powertrain —
surfaces.py:386 mirrors half-section wires before lofting).

```yaml
id:          AUTHOR-WIRE-MIRROR-ARM
contract:    [AUTHOR-WIRE-MIRROR-ARM]
class:       mechanical
crates:      [truck123d]
depends_on:  []
write_allow:
  - corpus/ttc/door.py
  - truck123d/tests/wire_mirror_arm.rs
read_allow:
  - truck123d/src/bd_bridge.rs
tests_required: [truck123d/tests/wire_mirror_arm.rs]
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'a mirror of this carrier is not a kernel-engine row' corpus/ttc/door.py"}
budget:      {turns: 35, ctx_tokens: 120000}
```

## Pre-made judgements

1. A mirrored wire/edge records as REFLECTED CONTROL DATA: for the
   recorded edge vocabulary (line: endpoints reflected; spline: control
   points reflected), the reflection is exact arithmetic — no
   re-interpolation, no resampling. The mirror is exact iff the
   reflection maps the recorded vocabulary to itself; anything else
   refuses typed naming the open carrier.
2. Plane.YZ / Plane.XZ / Plane.XY only (the placed-carrier mirror
   discipline; a free-plane mirror refuses typed, same as the part arm).
3. The mirrored wire feeds the EXISTING loft/make_face section paths —
   this packet adds the carrier, never touches downstream mechanics.

## Method

Extend the mirror arm's carrier dispatch per judgement 1. Tests: a
mirrored spline wire's control data is the exact reflection; a mirrored
line likewise; a mirrored free-plane wire refuses typed; the mirrored
wire then lofts green (composition test). Cargo through the queue; DLL
workaround on PATH if 0xc0000135.

## Done when

Scoped checks green; anchors hold (A1 -> 0). Write RESULT.json AT THE
WORKTREE ROOT.
