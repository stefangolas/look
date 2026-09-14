# WORK PACKET CT-102-FALLBACK-LAWS — certify the fallback width and cover laws (P14/P15)

Discharges obligations P14/P15 as measured certificate fields: the
subdivision fallback's local width law `w(cell) <= K h^2` (spec eq. 22)
and cover law `M(h) <= A_d h^-d` (eq. 21) are certified for the existing
refinement path on the fixture kit, and the measured `(K, A_d, d)` become
recorded fields of the run evidence. **Tests + a small measurement
module**: no change to the refinement's certification behavior.

```yaml
id:          CT-102-FALLBACK-LAWS
contract:    [CT-102-FALLBACK-LAWS]
class:       mechanical
crates:      [truck123d]
depends_on:  [CT-000-ATLAS-CONTRACT-SHIM]
needs:       [CT-000-ATLAS-CONTRACT-SHIM]
write_allow:
  - truck123d/src/atlas/laws.rs
  - truck123d/tests/atlas_fallback_laws.rs
read_allow:
  - docs/CONTACT_ATLAS_SPEC.md
  - docs/CONTACT_ATLAS_BUILD_SPEC.md
tests_required: [truck123d/tests/atlas_fallback_laws.rs]
anchors:
  - {id: A1, min: 5, cmd: "grep -cE '\\bPhaseStats\\b' truck123d/src/bd_bridge.rs"}
  - {id: A2, min: 6, cmd: "grep -cE '\\bBooleanVolumeCertificate\\b' truck123d/src/bd_bridge.rs"}
budget:      {turns: 35, ctx_tokens: 120000}
```

Anchors at authoring time `e271ba9`. **Re-measure at dispatch.**

## Pre-made judgements

1. **The laws are measured, not assumed.** For each fixture with a
   transversal contact curve, run the landed refinement at successively
   halved global width budgets and record (cell side h from `max_depth`,
   unresolved-cell count, residual width) — the certified `K` is the
   supremum of `w(cell)/h^2` over the run, `d` is the fitted decay
   exponent of the unresolved count. The test asserts the MEASURED
   constants satisfy the spec inequalities on that fixture, and that
   `d < 2` (the regular-curve regime, eq. 24's convergent branch).
2. `laws.rs` holds the measurement harness + the recorded-constant types
   — it reads certificate phase stats; it does not modify the solver.
3. **A violated law is a finding, not a failure to hide**: if a fixture
   measures `d >= 2` (area-like ambiguity — the abutting-interface
   class), the test records the measurement and asserts the refusal/ceiling
   behavior instead. That measurement is exactly the evidence Wave 3's
   co-support algebra needs.

## Done-when

1. `cargo test -p truck123d --test atlas_fallback_laws --locked` green:
   per-fixture (K, A_d, d) measured and asserted against the spec
   inequalities; the abutting-interface fixture (if it measures
   area-like) recorded with its refusal/ceiling behavior.
2. `cargo fmt -p truck123d -- --check` clean; `clippy -p truck123d
   --lib` clean on the diff.
3. Write `RESULT.json` AT THE WORKTREE ROOT (nowhere else), including the
   measured constants per fixture — they feed the Wave 4 tolerance
   budget split.
