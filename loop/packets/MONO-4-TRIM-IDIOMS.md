# WORK PACKET MONO-4-TRIM-IDIOMS — the trim constructor's corpus envelope

TRIM-EXTRUDE-CTOR landed the spline-trimmed extrude constructor but refuses
the corpus's actual trim idioms: the rear-wing louvre cutters
(`rear_wing.py:390` `_louvre_cutters`) and the suspension `_plate` profiles
(`suspension.py:162`) — the R2 census names the constructor as the refusing
carrier for rear_wing, steering_rack, suspension_front, suspension_rear. The
exact idiom mismatch is unidentified (the static op sweep found zero `trim(`
calls — the corpus reaches the constructor through composition, not a trim
call). This packet identifies the idiom from source, then extends the
constructor's envelope to it.

```yaml
id:          MONO-4-TRIM-IDIOMS
contract:    [MONO-4-TRIM-IDIOMS]
class:       mechanical
crates:      [truck123d]
depends_on:  [MONO-2-NSTATION-LOFT]
write_allow:
  - truck123d/src/bd_bridge.rs
read_allow:
  - truck123d/src/binding.rs
  - docs/MONO_CLOSURE_BOOKING.md
  - corpus/ttc/trees/f1/src/lib/rear_wing.py
  - corpus/ttc/trees/f1/src/lib/suspension.py
tests_required: []
anchors:
  - {id: A1, expect: 4, cmd: "grep -c 'trim_extrude\\|TrimExtrude' truck123d/src/bd_bridge.rs"}
budget:      {turns: 50, ctx_tokens: 150000}
```

## Method

1. **Identify (stop condition 1).** Read `rear_wing.py` `_louvre_cutters` and
   `suspension.py` `_plate` end to end; record the exact op sequence from
   profile authoring to the point the constructor refuses, and the exact
   `UnsupportedEnvelope` case the constructor returns. The identification is
   committed in RESULT.json notes before the implementation is adjudicated.
2. **Extend the envelope.** The constructor's stages (local-frame extrude,
   pullback polynomial, trim-clip composition) are landed and certified;
   extend the refusing stage to the traced idiom with the same certificate
   discipline (per-stage certified facts, composition order preserved,
   nothing reimplemented).
3. **Typed refusal stays** for anything outside the traced idioms. No
   approximation, no tolerance stretch.
4. All cargo through the queue; no OCC anywhere.

## Done when

- check/lib tests green including: `louver_idiom_composes_certified`,
  `suspension_plate_idiom_composes_certified` (or the honest typed refusals
  if the idiom decomposes into named gaps that belong to other packets),
  `constructor_envelope_still_refuses_unknown_idioms_typed`.
- The landed trim tests remain green unmodified; fmt/clippy clean on added
  lines.

## Stop conditions

- If the traced idiom requires certified intersection between spline-carried
  solids (the wave-3 frontier), STOP and record exactly which composition
  step needs it — that step is out of this packet's scope by design.

Write RESULT.json AT THE WORKTREE ROOT.
