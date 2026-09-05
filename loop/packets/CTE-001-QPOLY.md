# WORK PACKET CTE-001-QPOLY — the exact witness verifier (one verifier, two uses)

You are implementing the exactness primitive of the Certified Tangency and
Exact Contact (CTE) program. Everything you need is in this document,
`docs/CTE_BUILD_SPINE.md`, and the root theory
`docs/CERTIFIED_TANGENCY_EXACT_CONTACT_SPEC.md` (§4 is normative). Do not
read other spec files. If something you need is genuinely missing, that is a
SPEC_GAP: you stop and report, you do not research it.

```yaml
id:          CTE-001-QPOLY
contract:    [CTE-001-QPOLY]
class:       design
crates:      [truck-certified]
depends_on:  [CTE-000-SPINE]
write_allow:
  - vendor/truck/truck-certified/src/tangency/qpoly.rs
  - vendor/truck/truck-certified/src/tangency/witness.rs
  - vendor/truck/truck-certified/src/tangency/mod.rs
read_allow:
  - vendor/truck/truck-certified/src/tangency/shapes.rs
  - vendor/truck/truck-certified/src/tangency/fixtures.rs
  - vendor/truck/truck-certified/src/formal/exact.rs
  - vendor/truck/truck-certified/src/ssi_types.rs
  - docs/CERTIFIED_TANGENCY_EXACT_CONTACT_SPEC.md
  - docs/CTE_BUILD_SPINE.md
tests_required:
  - qpoly_ring_arithmetic_is_exact
  - verifier_accepts_f5_hand_witness
  - verifier_rejects_perturbed_identity
  - nonvanishing_multiplier_is_interval_certified
  - provenance_producer_refuses_pending
budget:      {turns: 90, ctx_tokens: 180000}
```

## Problem

Exact contact is intrinsically uncertifiable from floating enclosures
(theory §4.1); the certificate is an exact polynomial identity over ℚ with
interval-nonvanishing multipliers (§4.2). You build the ℚ-polynomial
substrate and the ONE verifier both A₁ and A₂ instantiate. The theory is
explicit: verification is "expand both sides in a common basis over ℚ and
compare coefficients, plus one or two interval nonvanishing tests" — small,
total, provenance-independent. Producers stay OUT of scope (§6.1 open): the
provenance fast path is wired as a stub behind a pending refusal.

## Scope decisions — pre-made, do not relitigate

1. **No CAS dependency.** Hand-rolled multivariate polynomial type:
   `QPoly` as a sorted `BTreeMap<Monomial, QCoeff>` (or an equivalent
   deterministic ordered representation) with exact rational coefficients.
   Coefficients are `i128`-numerator/`i128`-denominator pairs with a
   refusing constructor (overflow refuses — Bernstein degrees here make
   coefficient blow-up a named event, not a panic). NO num-rational, NO rug,
   NO external crates (product boundary, `AGENTS.md`).
2. **The verifier is total and side-parametric** (theory §4.2):
   `verify(&self, f: &QPoly, p: &[&QPoly]) -> Result<Verified, WitnessRefusal>`
   expands `s·f − (q²a + Σ wᵢPᵢ)` to zero exactly and checks the interval
   nonvanishing predicates. The A₁ instantiation has `q2a = None`; A₂ has
   `q2a = Some((q, a))`. ONE code path — the `WitnessSide` tag selects
   nothing except documentation.
3. **Interval nonvanishing is the landed discipline**: `ŝ(B)` and `â(B)`
   enclosures come from the caller (hull machinery over the witness's
   grids), 0 STRICTLY excluded — the same strict-exclusion rule as
   `KrawczykCertificate3::new`'s determinant.
4. **Producers never verify; the verifier never produces** (theory §4.2
   corollary). The provenance fast path is a constructor from already-known
   polynomial data whose IDENTITY still goes through the same verifier —
   provenance is a hint about what to TRY, never a trust level.
5. **Normal-form/RUR producers are pending refusals**, named
   `witness_producer_packet_pending` (the `cone_torus_carrier_packet_pending`
   precedent). No stub logic, no fake results.
6. **Deterministic iteration everywhere**: `BTreeMap` order = monomial
   lexicographic order; coefficient comparison is order-independent by
   construction.

## Contract — frozen output (spine §2; CTE-005/007 consume)

```rust
pub struct QPoly { /* ordered exact monomial map */ }
impl QPoly { pub fn add/sub/mul/pow/scale/zero/is_zero ... }
pub struct ExactVanishingWitness { /* shapes.rs type; built here */ }
impl ExactVanishingWitness {
    pub fn verify(&self, f: &QPoly, p: &[&QPoly],
                  s_encl: (f64, f64), a_encl: Option<(f64, f64)>)
        -> Result<VerifiedWitness, WitnessRefusal>;
}
pub fn provenance_witness(/* construction data */) -> Result<ExactVanishingWitness, WitnessRefusal>;  // pending-refusal stub until CTE-005/007 wire real provenance
```

## Anchors — measured this session; re-check on your branch before building

| id | file | pattern | expect |
|---|---|---|---|
| A1 | `truck-certified/src/formal/exact.rs` | `pub struct Expansion` | 1 |
| A2 | `truck-certified/src/tangency/shapes.rs` | `pub struct ExactVanishingWitness` | 1 |
| A3 | `truck-certified/src/construct/refusal.rs` | `pub enum ConstructRefusal` | 1 |
| A4 | `truck-certified/src/ssi_types.rs` | `pub struct SquareSystem3` | 1 |

## House rules

- **H-1** No `unwrap`, `expect`, `panic!`, `todo!`, or out-of-range indexing
  reachable from geometry.
- **H-2** Fallible operations return `Result<T, WitnessRefusal>`; arithmetic
  overflow is a named refusal, never a wrap or a panic.
- **H-3** No absolute constants in predicates; test epsilons carry `// H-3`
  on the SAME line as the literal.
- **H-6** Float-computed values are never recorded as `Method::Exact`; the
  witness's exactness claim is about the ℚ identity, the interval predicates
  are certified separately.
- **Determinism**: BTreeMap iteration; no hash ordering in output.
- **All cargo invocations go through the queue (the `cargo` on PATH IS the
  queue shim). Do not invoke cargo by absolute path; do not unset the shim.**
- Never run a bare `cargo test` — use the scoped commands below.

## Tests required

1. `qpoly_ring_arithmetic_is_exact` — randomized (seeded, fixed order)
   identities: `(a+b)² = a²+2ab+b²`, distributivity, `pow`, all exact.
2. `verifier_accepts_f5_hand_witness` — the F5 A₂ witness
   (`s=a=1, q=u1`): `f = u1²` on `G=0` exactly; verifier accepts.
3. `verifier_rejects_perturbed_identity` — perturb any witness polynomial by
   one coefficient; the verifier refuses (this is the falsifiability gate).
4. `nonvanishing_multiplier_is_interval_certified` — `ŝ` containing 0
   refuses; strict exclusion accepts.
5. `provenance_producer_refuses_pending` — the stub refuses with the named
   pending cause.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-certified
cargo clippy -p truck-certified --all-targets -- -D warnings
cargo test -p truck-certified --lib tangency
```

Send cargo output to a file and read the tail.

## Forbidden

Editing any file outside `write_allow` — especially `formal/exact.rs`,
`tangency/shapes.rs`, `Cargo.toml` (NO new dependencies), `Cargo.lock`.
Implementing normal-form or RUR producers (booked open, theory §6.1).
Adding `#[ignore]`. Adding `#[allow]` without a justification comment on the
same line. Committing to `main`.

## Stop conditions

- any anchor count differs → `ANCHOR_MISMATCH`
- exact arithmetic overflows i128 on any fixture → `SPEC_GAP` with the
  coefficients (a degree/coefficient-scale decision the loop must make)
- three consecutive failed `cargo test` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` at the WORKTREE ROOT (then COMMIT first)

**COMMIT BEFORE writing `RESULT.json`.**

```json
{"id":"CTE-001-QPOLY","status":"DONE","contracts":["CTE-001-QPOLY"],
 "tests_added":5,"anchors_verified":{"A1":1,"A2":1,"A3":1,"A4":1},
 "notes":"coefficient representation; monomial order; F5 verification result; any API deviation"}
```

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit on the current branch with subject
`feat(certified): exact ℚ-polynomial substrate + one-witness verifier (CTE-001-QPOLY)`.
