# WORK PACKET BIE-000-CONTRACT — the BIE shim: outcome contract, carrier decision, fixture kit

You are implementing the contract packet of the Certified Interaction Engine
(BIE) program. Everything you need is in this document and the spine doc
`docs/BIE_BUILD_SPINE.md`. Do not read other spec files — this packet is
self-contained. If something you need is genuinely missing, that is a SPEC_GAP
(see "Stop conditions"): you stop and report, you do not research it.

```yaml
id:          BIE-000-CONTRACT
contract:    [BIE-000-CONTRACT]
class:       design
crates:      [truck-certified, truck-base]
depends_on:  []
write_allow:
  - vendor/truck/truck-certified/src/construct/mod.rs
  - vendor/truck/truck-certified/src/construct/bie/mod.rs
  - vendor/truck/truck-certified/src/construct/bie/fixtures.rs
  - docs/CERTIFICATE_MAPPING.md
read_allow:
  - vendor/truck/truck-base/src/evidence.rs
  - vendor/truck/truck-geometry/src/constructive/sweep_surface.rs
  - vendor/truck/truck-evidence/src/contact/mod.rs
  - docs/BIE_BUILD_SPINE.md
  - docs/CERTIFIED_INTERACTION_ENGINE_BUILD_SPEC.md
tests_required:
  - interaction_outcome_maps_onto_landed_refusal
  - fixture_plane_sphere_ground_truth
  - fixture_plane_cylinder_ground_truth
  - fixture_sweep_plane_ground_truth
  - fixture_kit_is_deterministic
budget:      {turns: 50, ctx_tokens: 120000}
```

This is the program's **shim packet**: frozen types + refusing constructors +
a synthetic fixture kit with stated, machine-checked ground truths. NO solver
bodies. Later waves (BIE-001..007) type against what you land here.

## Problem

The BIE program (restricted interaction: `SpineFrameSweep × canonical`, plus
the landed canonical × canonical path) has no contract layer. This packet
lands three things:

1. **`InteractionOutcome`** — the program's typed outcome vocabulary, in
   `construct/bie/mod.rs`, mapping onto the landed evidence taxonomy.
2. **The §8.1 carrier decision, recorded** — doc-comment + mapping rows.
3. **The unit-shape fixture kit** — `construct/bie/fixtures.rs`, with known
   ground truths that later packets' tests are graded against.

## The carrier decision — PRE-DECIDED, do not relitigate

The orchestrator has decided (spine §3; derivation in
`docs/BIE_BUILD_SPINE.md`): the §8.1 procedural carrier is
**`CertifiedImplicitIntersectionCurve`** — a new canonical `Curve` variant in
`truck-geometry/src/canonical.rs` (landed in BIE-003, NOT this packet),
mirroring the landed `IntersectionCurve` pattern, carrying a certified 3-D
polyline with per-sample tangent frames and the unresolved witness slot, with
the PL-at-tessellation policy (`EdgeSampleLedger`-compatible; truck-meshalgo
is read-only). Your job is to **record** this decision: a doc comment in
`construct/bie/mod.rs` naming it, and the mapping rows in
`docs/CERTIFICATE_MAPPING.md`. If reading the tree gives you evidence the
decision cannot work, that is a SPEC_GAP with the evidence — not a redesign.

## Contract types (frozen; signatures below are the reference answer)

```rust
/// The restricted-pair interaction outcome (BIE program).
/// `Unresolved` maps onto the landed `Refusal::NumericallyUnresolved`
/// witness — zero new refusal arms (spine §8; a violation is a SPEC_GAP).
pub enum InteractionOutcome {
    /// A certified answer carrying its certificate value type.
    Certified(CertificateValue),
    /// The three-valued verdict: unresolved with κ / cell / slope witness.
    Unresolved { kappa: f64, cell: WitnessCell, slope: f64 },
    /// A landed typed refusal, passed through unchanged.
    Refused(Refusal),
}
```

`CertificateValue` and `WitnessCell` are the minimal value types the fixtures
need (a certified scalar/point value; a `(u,v)×(s,t)` parameter-cell record).
Check `truck_base::evidence` for the exact `Refusal` shape — **use what is
actually there**; the `NumericallyUnresolved` variant at
`truck-base/src/evidence.rs:57` has a witness slot, and BIE-000 maps
`Unresolved { kappa, cell, slope }` onto it (both `NumericallyUnresolved`
sites, evidence.rs:57 and :347, are the anchor). Refusing constructors only:
no `From` impl may fabricate a `Certified` from floats; H-6 applies.

## Anchors — verified 2026-09-05, counts are exact

Locate by running the pattern (use Select-String / the Grep tool; `rg` is not
on PATH). **Never locate by line number.** If a count differs, STOP and
report `ANCHOR_MISMATCH` with what you saw.

| id | file | pattern | expect |
|---|---|---|---|
| A1 | `vendor/truck/truck-base/src/evidence.rs` | `pub enum Refusal` | 1 |
| A2 | `vendor/truck/truck-base/src/evidence.rs` | `NumericallyUnresolved` | 2 |
| A3 | `vendor/truck/truck-evidence/src/contact/mod.rs` | `pub enum BoundedStratum` | 1 |
| A4 | `vendor/truck/truck-geometry/src/constructive/sweep_surface.rs` | `impl ParametricSurface for SpineFrameSweep` | 1 |
| A5 | `vendor/truck/truck-certified/src/construct/mod.rs` | `^pub mod` | 26 |

A5 becomes 27 when you add `pub mod bie;`. Measured 2026-09-05 after the
CC-014 landing added `loft_validity`; the CC program was still closing when
this packet was authored, so **re-derive A5 at dispatch time** (spine §10) —
a count mismatch here is expected if CC lands again, not a worker error.

## Fixture kit — ground truths are contract

Each fixture is a pair with a closed-form interaction whose ground truth is
stated and machine-checked in the test. `truck-certified` depends on
`truck-geometry`, so the fixtures name real carriers:

1. **plane × sphere** — a plane through a sphere: intersection circle, center
   and radius from the plane's signed distance to the sphere center (analytic;
   state the formula in the fixture doc).
2. **plane × cylinder** — a transverse plane through a canonical cylinder:
   ellipse with known semi-axes (`r` and `r/|sin θ|` for incident angle θ).
3. **sweep × plane** — a `SpineFrameSweep` with a straight spine and
   `ProfileLaw::Scale` of a circle crossing a transverse plane: the conic
   section with known station parameter `s*` and ring parameter from the
   windowed domain. Derive `s*` from the plane equation and the spine
   parameterization IN the packet test (show the algebra in a comment).
4. **Determinism**: building the kit twice yields equal values (no hash
   iteration, no unordered collection in construction).

The fixture kit must not call any solver. Ground truths are closed-form
constants derived in comments, asserted with `// H-3` tolerance discipline.

## House rules

- **H-1** No `unwrap`, `expect`, `panic!`, `unimplemented!`, `todo!`, or
  out-of-range indexing reachable from geometry.
- **H-2** Fallible operations return `Outcome<T>` — never `Option`, never a
  bare `Result`.
- **H-3** No absolute constants in predicates. Float comparison epsilons in
  tests carry `// H-3` on the SAME line as the literal (the gate accepts the
  opt-out only same-line, not on the line above).
- **H-6** A value computed in floats is never recorded as `Method::Exact`.
- **Determinism** (spine §8): identical ordered input → identical values; no
  output ordering from hash iteration.
- **All cargo invocations go through the queue (the `cargo` on PATH IS the
  queue shim). Do not invoke cargo by absolute path; do not unset the shim.**
- Never run a bare `cargo test` — it builds 56 examples. Use the scoped
  commands below.

## Tests required

Each must be a named `#[test]` fn (in-module test module in
`construct/bie/fixtures.rs` / `mod.rs`) — the verifier checks the names
appear in your diff.

1. `interaction_outcome_maps_onto_landed_refusal` — an `Unresolved` outcome
   maps onto the landed `Refusal::NumericallyUnresolved` witness; a
   `Refused` passthrough round-trips a real landed `Refusal` value.
2. `fixture_plane_sphere_ground_truth` — circle radius/center match the
   closed form.
3. `fixture_plane_cylinder_ground_truth` — ellipse semi-axes match the
   closed form.
4. `fixture_sweep_plane_ground_truth` — `s*` and the conic parameters match
   the derivation.
5. `fixture_kit_is_deterministic` — two constructions compare equal.

No existing test may be deleted, `#[ignore]`d, or weakened.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-certified
cargo clippy -p truck-certified --all-targets -- -D warnings
cargo test -p truck-certified --lib
cargo check --workspace --all-targets
```

The last one matters most: this is the program shim — every later wave
types against it. Send cargo output to a file and read the tail.

## Forbidden

Editing any file outside `write_allow` — especially anything under
`vendor/truck/truck-geometry/`, `truck-evidence/`, `truck-base/src/evidence.rs`,
`scripts/kernel-gates.sh`, `Cargo.lock`. Writing solver bodies (the fixture
kit must not solve). Adding `#[ignore]`. Adding `#[allow]` without a
justification comment on the same line. Committing to `main`.

## Stop conditions

- any anchor count differs → `ANCHOR_MISMATCH`
- the carrier decision cannot be recorded as stated with tree evidence →
  `SPEC_GAP`, naming the evidence
- three consecutive failed `cargo test` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` at the WORKTREE ROOT (then COMMIT first)

**COMMIT BEFORE writing `RESULT.json`.** Then write `RESULT.json` at the root
of your worktree (not `loop/results/` — the orchestrator files it there).

```json
{"id":"BIE-000-CONTRACT","status":"DONE","contracts":["BIE-000-CONTRACT"],
 "tests_added":5,"anchors_verified":{"A1":1,"A2":2,"A3":1,"A4":1,"A5":26},
 "notes":"anything the fixture derivations pinned down that the packet did not state"}
```

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it: what you attempted, the
exact ambiguity, and the readings you could not choose between.

Commit on the current branch with subject
`feat(certified): BIE contract shim — outcome mapping, carrier decision, fixture kit (BIE-000-CONTRACT)`.
