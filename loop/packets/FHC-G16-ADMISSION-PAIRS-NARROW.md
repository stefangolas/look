# WORK PACKET FHC-G16-ADMISSION-PAIRS-NARROW — widen the five narrow boolean carrier pairs

R4 census refusal taxonomy (2026-09-14, `loop/nightly_ops/r4/` payloads,
`E_UNSUPPORTED_ENVELOPE / non_canonical_carrier / phase:admission`): five
boolean carrier-pair classes fall outside the admitted envelope, each with
a named production row that hits it. The certification machinery (parse →
broad phase → transversality gate → certified refinement) already exists
and is carrier-agnostic at the math level; this packet routes these pair
classes through it with typed refusals for everything that still cannot be
admitted.

```yaml
id:          FHC-G16-ADMISSION-PAIRS-NARROW
contract:    [FHC-G16-ADMISSION-PAIRS-NARROW]
class:       mechanical
crates:      [truck123d]
depends_on:  [FHC-G15-COMPOUND-VOLUME-FACTS]
needs:       [FHC-G15-COMPOUND-VOLUME-FACTS]
write_allow:
  - truck123d/src/bd_bridge.rs
  - truck123d/tests/admission_pairs_narrow.rs
read_allow:
  - docs/REFUSALS.md
  - docs/F1_HYPERCAR_GAP_REGISTER.md
tests_required: [truck123d/tests/admission_pairs_narrow.rs]
anchors:
  - {id: A1, expect: 48, cmd: "grep -cE '\\bSweptAdmissionRefusal\\b' truck123d/src/bd_bridge.rs"}
  - {id: A2, expect: 1,  cmd: "grep -c 'unsupported_envelope' truck123d/src/bd_bridge.rs"}
  - {id: A3, expect: 3,  cmd: "grep -cE '\\btrim_prism\\b' truck123d/src/bd_bridge.rs"}
budget:      {turns: 60, ctx_tokens: 160000}
```

Anchors measured 2026-09-14 at HEAD `14befdf`. **Re-measure at dispatch** —
FHC-G15 will have landed on this file first (write-set serial; the
dispatcher enforces it).

## The five pair classes (R4 evidence, verbatim)

| pair | verb | production row (client module) |
|---|---|---|
| `lathe * cylinder` | cut | corner_fl/fr/rl/rr (`lib.wheels`) |
| `trim_prism * trim_prism` | cut | f1/suspension_front (`lib.suspension`) |
| `section_prism * section_prism` | fuse | brakes (`lib.brakes`) |
| `spline_loft * box` | cut | details, second site (`lib.details`) |
| `cylinder * cylinder` | fuse | nose (`lib.nose`) |

## Pre-made judgements (the worker churns, it does not design)

1. **Diagnose first, per pair class.** One door spot-check with
   `--trace-carriers` per owning row to capture the exact operand carriers
   and geometry (which lathe, which prism, which cylinder), recorded in
   `RESULT.json`. Do not admit anything from the pair name alone.
2. **Admit through the existing gate only.** Each pair class routes into
   the SAME funnel: parse → control-hull broad phase → transversality
   admission → certified refinement. The widening is the envelope routing
   decision, never a bypass of the gate. Where the surface-class facts the
   gate needs are already landed (cylinders: canonical; lathe surfaces:
   the revolute arm; prisms: the trim/extrude carrier rows), admit.
3. **Where the gate refuses, refuse typed.** A pair class whose
   transversality or membership coverage is not already landed stays
   refused with its typed code (`TransversalityUncertified`,
   `unsupported_envelope` with the SPECIFIC missing surface named) — never
   a bare `unsupported_envelope`. The refusal narrows; it does not vanish.
   Do NOT weaken `TRANSVERSALITY_MIN` or any tolerance.
4. **V5 net is a hard gate:** every row green at the dispatch base stays
   green. The corner rows refuse today; after this packet the corner
   wheels construct (the lathe disc minus the axle cylinder is the
   production cut) — verify each owning row end to end through the door,
   record the verdict flip in `RESULT.json`.
5. **Tests with ground truth, one per pair class**, in the new test file:
   minimal constructions whose analytic volumes are known exactly (a
   lathe disc of known radius/height cut by a known cylinder; prism-prism
   overlap boxes; a spline loft against a box cut). House rules on float
   comparisons: H-3 same-line opt-out markers where epsilons appear.

## Done-when

1. `cargo check -p truck123d --tests --locked` green.
2. `cargo test -p truck123d --test admission_pairs_narrow --locked` green
   (one ground-truth test per pair class, minimum).
3. `cargo fmt -p truck123d -- --check` clean on touched files; `cargo
   clippy -p truck123d --lib --locked` clean on the diff.
4. Door spot-checks: corner (any), nose, f1/suspension_front, brakes,
   details — the previously-refusing dispatch now either certifies or
   refuses with the narrowed typed code; record before/after in
   `RESULT.json`.
5. Write `RESULT.json` AT THE WORKTREE ROOT (nowhere else), including the
   anchor re-measurement and the per-pair admission decision table.
