# WORK PACKET MONO-3-BLADE-MEMBERS-MIRROR — plate-section members and mirror of kernel rows

The corpus's monocoque/nose/rear_wing build blade members — rim bead, roll
hoop, hoop stays, headrest bosses, wishbone fairings — through
`surfaces.swept_plate` / `surfaces.blade_path` / `surfaces.blade_member`
(surfaces.py), and mirror them with `surfaces.mirror_y`. The R2 census names
`mirror_y` a typed refusal on kernel rows ("a mirror of this carrier is not a
kernel-engine row"); the member carriers route through the landed sweep
machinery but their plate-section idiom is unlanded.

```yaml
id:          MONO-3-BLADE-MEMBERS-MIRROR
contract:    [MONO-3-BLADE-MEMBERS-MIRROR]
class:       mechanical
crates:      [truck123d]
depends_on:  [MONO-2-NSTATION-LOFT]
write_allow:
  - truck123d/src/bd_bridge.rs
read_allow:
  - truck123d/src/binding.rs
  - docs/MONO_CLOSURE_BOOKING.md
  - corpus/ttc/trees/f1/src/lib/surfaces.py
  - corpus/ttc/trees/f1/src/lib/mono_tub.py
tests_required: []
anchors:
  - {id: A1, expect: 0,  cmd: "grep -c 'swept_plate\\|blade_member\\|blade_path' truck123d/src/bd_bridge.rs"}
  - {id: A2, expect: 30, cmd: "grep -ci 'mirror' truck123d/src/bd_bridge.rs"}
budget:      {turns: 50, ctx_tokens: 150000}
```

## Method

1. **Trace first (stop condition 1).** Read `surfaces.swept_plate`,
   `surfaces.blade_path`, `surfaces.blade_member` end to end and record the
   exact drop-in op sequence each invokes (profile author -> placement ->
   loft/sweep -> thickening). The carrier mapping must name the landed
   machinery each step rides (SWEEP-PATH's spline-path sweep; MONO-2's loft
   for any stacked-station member). If an idiom has no landed carrier, record
   it as a named gap — do not improvise one.
2. **Plate-section members.** Implement the swept-member carrier for the
   traced idioms: plate profile swept along its recorded path, certified
   volume through the same per-patch `volume_facts` accounting as MONO-2
   (a member is a two-section degenerate case plus wall bands — derive the
   certificate from the sweep carrier's existing facts, do not invent a new
   integrator).
3. **Mirror of kernel rows.** Mirror_y (and mirror about the recorded plane
   generally) of a kernel row is an exact isometry: transform every control
   point by the recorded reflection; the volume's magnitude is invariant and
   orientation flips (a one-line certificate from the change-of-variables
   theorem). The mirrored row reuses the original's patch grid — no
   re-approximation. `mirror(mirror(x)) = x` exactly (test it bit-level).
4. All cargo through the queue; no OCC anywhere.

## Done when

- check/lib tests green including: `mirror_is_exact_isometry`,
  `mirror_twice_is_identity`, `member_volume_certified_bracket`,
  `member_refuses_open_path_typed` (if the traced idiom refuses open paths —
  match whatever the landed sweep carrier refuses).
- fmt/clippy clean on added lines; A1 drifts 0 -> >= 1 (record post-work).
- The traced op-sequence table (step 1) is committed into RESULT.json notes —
  it is the evidence that the mapping is complete.

## Stop conditions

- A member idiom whose trace does not resolve to landed carriers: stop, name
  the gap, typed refusal stays. No ruled-substitution, no tolerance stretch.

Write RESULT.json AT THE WORKTREE ROOT.
