# WORK PACKET FHC-G17-TRIM-PRISM-EXTRUDE-ADMISSION — admit the unary trimmed-extrude construction

R4 census refusal payloads (2026-09-14): two rows refuse a **unary**
construction (not a boolean pair): `engine_cover` (`lib.engine_cover`) and
`aero` (`lib.aero`) refuse `E_UNSUPPORTED_ENVELOPE / non_canonical_carrier`
with `verb: extrude`, `carrier: trim_prism`, `phase: admission`. The
trimmed-prism extrude — a profile extrusion whose profile or path carries a
prior trim record — is not an admitted construction envelope today. This
packet admits it through the same certified carrier machinery every other
construction verb uses.

```yaml
id:          FHC-G17-TRIM-PRISM-EXTRUDE-ADMISSION
contract:    [FHC-G17-TRIM-PRISM-EXTRUDE-ADMISSION]
class:       mechanical
crates:      [truck123d]
depends_on:  [FHC-G15-COMPOUND-VOLUME-FACTS]
needs:       [FHC-G15-COMPOUND-VOLUME-FACTS]
write_allow:
  - truck123d/src/bd_bridge.rs
  - truck123d/tests/trim_prism_extrude_admission.rs
read_allow:
  - docs/REFUSALS.md
  - docs/F1_HYPERCAR_GAP_REGISTER.md
tests_required: [truck123d/tests/trim_prism_extrude_admission.rs]
anchors:
  - {id: A1, expect: 48, cmd: "grep -cE '\\bSweptAdmissionRefusal\\b' truck123d/src/bd_bridge.rs"}
  - {id: A2, expect: 3,  cmd: "grep -cE '\\btrim_prism\\b' truck123d/src/bd_bridge.rs"}
  - {id: A3, expect: 5,  cmd: "grep -cE '\\bRowFacts\\b' truck123d/src/bd_bridge.rs"}
budget:      {turns: 35, ctx_tokens: 120000}
```

Anchors measured 2026-09-14 at HEAD `14befdf`. **Re-measure at dispatch** —
FHC-G15 and (if landed first) FHC-G16 will have touched this file; the
dispatcher serializes on the write set.

## Pre-made judgements

1. **Diagnose first.** One door spot-check with `--trace-carriers` on
   `engine_cover` and `aero` to capture the exact construction (which
   profile, which trim record, which extrusion path), recorded in
   `RESULT.json`. The two rows may hit DIFFERENT trim-prism shapes; admit
   the shape actually present, not a guessed generalization.
2. **The prism vocabulary is landed** (`trim_prism` exists as a carrier
   row and in the routing vocabulary — 3 sites in `bd_bridge.rs`); the
   gap is the extrude admission decision for it. Route the admitted case
   through the standard certified carrier construction: the extrusion
   emits its own `VolumeRow`s with certified brackets, same as the
   canonical extrude path. No new solver, no new certificate type.
3. **Where the trim record cannot be certified** (a profile segment the
   facts cannot measure), refuse typed with the specific missing piece —
   never a bare `unsupported_envelope`, never a silent fallback.
4. **V5 net is a hard gate:** rows green at the dispatch base stay green.
   Acceptance: `engine_cover` and `aero` construct end to end through the
   door; record the verdict flips in `RESULT.json`.
5. **Tests with ground truth** in the new test file: a trimmed-profile
   extrusion whose analytic volume is known exactly (profile minus a
   rectangular notch, extruded a known height — the volume is the
   rectilinear difference), plus a typed-refusal test for a trim record
   the machinery cannot certify. H-3 same-line markers where float
   epsilons appear.

## Done-when

1. `cargo check -p truck123d --tests --locked` green.
2. `cargo test -p truck123d --test trim_prism_extrude_admission --locked`
   green.
3. `cargo fmt -p truck123d -- --check` clean on touched files; `cargo
   clippy -p truck123d --lib --locked` clean on the diff.
4. Door spot-checks: `engine_cover` and `aero` construct; record
   before/after in `RESULT.json`.
5. Write `RESULT.json` AT THE WORKTREE ROOT (nowhere else), including the
   anchor re-measurement.
