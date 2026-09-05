# WORK PACKET CL-006-SOLVER-ENTRY — invert the restricted solver entry so the funnel can close sweep booleans

You are implementing the dependency-inversion layer of the Carrier Lift (CL)
program. Everything you need is in this document and
`docs/CARRIER_LIFT_BUILD_SPEC.md`. If something you need is genuinely
missing, that is a SPEC_GAP (see "Stop conditions"): you stop and report,
you do not research it.

```yaml
id:          CL-006-SOLVER-ENTRY
contract:    [CL-006-SOLVER-ENTRY]
class:       design
crates:      [truck-evidence, truck-certified]
depends_on:  [BIE-006-CLASSIFY]
write_allow:
  - vendor/truck/truck-evidence/src/contact/mod.rs
  - vendor/truck/truck-evidence/src/contact/solver_entry.rs
  - vendor/truck/truck-certified/src/construct/bie/ssi4.rs
read_allow:
  - vendor/truck/truck-certified/src/construct/bie/mod.rs
  - vendor/truck/truck-evidence/src/contact/classify.rs
  - vendor/truck/truck-shapeops/src/boolean/sweep_lift.rs
  - docs/CARRIER_LIFT_BUILD_SPEC.md
tests_required:
  - entry_trait_implemented_by_certified_engine
  - funnel_closes_sweep_pair_end_to_end
  - registry_flips_when_engine_absent
budget:      {turns: 65, ctx_tokens: 160000}
```

**New file** (`contact/solver_entry.rs`): H-1 applies.

## Problem

BIE-006's landed RESULT records the architectural blocker verbatim: the
restricted certified engine (`BIE-002`'s `Ssi4System`/`krawczyk`
machinery) is a `truck-certified` construction, and **no boolean crate can
name it** — `truck-evidence` and `truck-shapeops` sit upstream of
`truck-certified` in the dependency direction (truck-certified depends on
them, not the reverse). The funnel therefore lifts sweep pairs and answers
typed `NumericallyUnresolved` without ever calling the certified engine.
This packet inverts the entry so the funnel CLOSES sweep×canonical booleans
end to end.

## Scope decisions — pre-made, do not relitigate

1. **The inversion shape is a trait in the downstream crate**: define
   `RestrictedSolverEntry` in `truck-evidence/src/contact/solver_entry.rs`
   — the pure interface the funnel needs (given a certified sweep window +
   canonical carrier pair, return the certified outcome vocabulary BIE-000
   froze: certified chart samples / typed `Unresolved{κ, cell, slope}` /
   landed refusal). The funnel dispatches through the trait.
2. **The impl lives in `truck-certified`** (`ssi4.rs`, whose write set this
   packet owns): `impl RestrictedSolverEntry for` the certified engine,
   adapting its landed `CertifiedChartCurve` output onto the trait's
   types. `truck-certified` already depends on `truck-evidence` — the
   impl compiles there, the funnel never names the engine.
3. **Registration is runtime-explicit, never a cargo feature**: a
   set-once registry slot in `solver_entry.rs`
   (`set_restricted_solver(impl ...)` / `take()`); the certified crate
   registers at its init (a `#[ctor]`-free explicit call — the kernel's
   entry point wires it; find the landed init site and note it). When no
   engine is registered, the funnel answers the SAME typed
   `NumericallyUnresolved` as today (the registry-flip test pins both
   arms).
4. **V5, absolute**: canonical×canonical answers are bit-identical; the
   trait dispatch sits exactly where BIE-006's lift currently returns the
   typed-unresolved shortcut — a canonical control fixture set proves no
   drift.
5. **Zero new dependency edges**: no Cargo.toml changes (the whole point).

## Anchors — measured 2026-09-05, counts are exact

| id | file | pattern | expect |
|---|---|---|---|
| A1 | `vendor/truck/truck-certified/src/construct/bie/ssi4.rs` | `pub struct CertifiedChartCurve` | 1 |
| A2 | `vendor/truck/truck-evidence/src/contact/mod.rs` | `NumericallyUnresolved` | 2 |
| A3 | `vendor/truck/truck-certified/src/Cargo.toml` | `truck-evidence` | 1 |
| A4 | `vendor/truck/truck-shapeops/src/boolean/sweep_lift.rs` | `pub struct FragmentProvenance` | 1 |

A3 is the dependency-direction proof: truck-certified already depends on
truck-evidence — the impl side can name the trait.

## House rules

- **H-1** no unwrap/expect/panic reachable from geometry; **H-2** the
  trait returns the landed outcome vocabulary; **H-3** same-line `// H-3`.
- **Determinism**: registry state is set-once; double registration refuses.
- **All cargo through the queue shim.** Scoped commands only.

## Tests required

1. `entry_trait_implemented_by_certified_engine` — the trait object
   resolves to the certified impl and returns a certified chart for the
   BIE-000 fixture kit's transversal pair.
2. `funnel_closes_sweep_pair_end_to_end` — through the FUNNEL entry (the
   dispatch site BIE-006 landed), a sweep×canonical pair that today answers
   `NumericallyUnresolved`-by-absence now returns the certified engine's
   outcome (certified or engine-typed-unresolved — assert it CAME from the
   engine, distinguishable from the absence default).
3. `registry_flips_when_engine_absent` — with the registry cleared, the
   funnel answers exactly today's typed-unresolved (the V5 baseline arm).

No existing test may be deleted, `#[ignore]`d, or weakened.

## Done when

```
cargo fmt --check -p truck-evidence -p truck-certified
cargo clippy -p truck-evidence -p truck-certified --all-targets -- -D warnings
cargo test -p truck-evidence --lib contact
cargo test -p truck-certified --lib
cargo check -p truck-shapeops
```

## Forbidden

Anything outside `write_allow` — especially `Cargo.toml` (zero new edges),
`classify.rs`, `sweep_lift.rs` (read-only), landed test files,
`scripts/kernel-gates.sh`. Adding a cargo feature or cfg switch for the
registry. Adding `#[ignore]`. Unjustified `#[allow]`. Committing to
`main`.

## Stop conditions

- any anchor count differs → `ANCHOR_MISMATCH`
- the trait cannot express the engine's certified output without new
  evidence types → `SPEC_GAP`, naming the type mismatch (zero new
  `Refusal` arms is the standing rule)
- three consecutive failed `cargo test` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` at the WORKTREE ROOT (then COMMIT first)

```json
{"id":"CL-006-SOLVER-ENTRY","status":"DONE","contracts":["CL-006-SOLVER-ENTRY"],
 "tests_added":3,"anchors_verified":{"A1":1,"A2":2,"A3":1,"A4":1},
 "notes":"the init site where the engine registers, and the fixture pairs that flipped from absent-unresolved to engine-certified"}
```

Commit subject: `feat(evidence): RestrictedSolverEntry trait + certified registration (CL-006-SOLVER-ENTRY)`.
