# WORK PACKET CTE-000-SPINE — the tangency shim (shapes + fixture kit)

You are creating the spine of the Certified Tangency and Exact Contact (CTE)
program. Everything you need is in this document, `docs/CTE_BUILD_SPINE.md`,
and the root theory `docs/CERTIFIED_TANGENCY_EXACT_CONTACT_SPEC.md`. Do not
read other spec files. If something you need is genuinely missing, that is a
SPEC_GAP (see "Stop conditions"): you stop and report, you do not research it.

```yaml
id:          CTE-000-SPINE
contract:    [CTE-000-SPINE]
class:       design
crates:      [truck-certified]
depends_on:  []
write_allow:
  - vendor/truck/truck-certified/src/tangency/mod.rs
  - vendor/truck/truck-certified/src/tangency/shapes.rs
  - vendor/truck/truck-certified/src/tangency/fixtures.rs
  - vendor/truck/truck-certified/src/lib.rs
  - docs/CERTIFICATE_MAPPING.md
read_allow:
  - docs/CERTIFIED_TANGENCY_EXACT_CONTACT_SPEC.md
  - docs/CTE_BUILD_SPINE.md
  - docs/CERTIFIED_TANGENCY_BUILD_SPEC.md
  - vendor/truck/truck-certified/src/ssi_types.rs
  - vendor/truck/truck-certified/src/formal/exact.rs
  - vendor/truck/truck-base/src/evidence.rs
tests_required:
  - det_a_constructor_refuses_zero_containing_enclosure
  - graph_enclosure_only_input_of_hessian_signature
  - definiteness_carries_positive_mu
  - witness_data_refuses_incomplete_identity
  - f1_t14_identity_instance_exact
  - f4_saddle_fixture_admits
budget:      {turns: 80, ctx_tokens: 180000}
```

## Problem

The CTE program (booking §3) freezes the certificate vocabulary for singular
SSI closure (T1) and coincident-carrier Boolean classification (T2) BEFORE any
implementation wave. This is the P0-FREEZE pattern (`ssi_types.rs` precedent):
types and refusing constructors only — nothing here evaluates, solves,
isolates, or certifies numerically. Seven later packets build against this
module and never restate it.

## Scope decisions — pre-made, do not relitigate

1. **D-shim discipline verbatim from `ssi_types.rs`**: any method that would
   evaluate numerically refuses (`InvalidInput`-shaped or a named landed
   cause). The module doc states this verbatim.
2. **The R2 rule is a signature**: `reduced_hessian` takes
   `&GraphEnclosure` — no raw-box overload exists. Theory §2.5 evaluation
   rule, made typecheckable.
3. **The R1 rule is structural**: no type in this module carries a
   "deflation determinant predicate". T1.4 appears only in the fixture kit
   as an exact identity test (F1) and in doc comments citing the theory.
4. **`Definiteness` carries μ** (theory R9): `Definite { sign, mu }` refuses
   construction with `mu <= 0`.
5. **One verifier, two instantiations** (theory §4.2): `ExactVanishingWitness`
   is a data type with `side: WitnessSide::{A1, A2}`; the verify fn is frozen
   as a trait signature here, implemented by CTE-001.
6. **`A2BranchCurve` ships behind a pending refusal** — the
   `cone_torus_carrier_packet_pending` precedent
   (`kernel/rational.rs`): the producing packet is CTE-005; the consuming
   packet CTE-007 builds against the shape now.
7. **`ContactVerdict` is the five-way cascade verdict** (theory §2.11):
   `Transversal(TransversalCert) | Empty | A1Isolated(A1Cert) |
   A1Node(A1Cert) | A2Branch(A2Cert) | Unresolved { kappa, cell }`.
   `A1Node` is FIRST-CLASS from day one (theory R3) — it is not a TODO arm.
8. **Zero new top-level evidence kinds.** Refusals map onto the landed
   vocabularies (`SsiRefusal`, `TraceRefusal`, kernel `Refusal`,
   `HullRefusal`); record the mapping rows in `docs/CERTIFICATE_MAPPING.md`.
   A case that seems to need a new arm is a SPEC_GAP.

## Shapes to freeze (theory §3 certificate data; spine §2 contract inventory)

In `tangency/shapes.rs`, each with a refusing constructor and verbatim
accessors: `Rank2Chart` (pivot `(component, coord_pair)`; `det_a` enclosure
with 0 STRICTLY excluded by construction), `GraphEnclosure` (`y_hat`, `z`,
graph evidence), `ChartMinorGrids`, `IntervalSym2`, `Definiteness`,
`ExactVanishingWitness` (+ `WitnessSide`), `ContactVerdict` + `A1Cert` /
`A2Cert` / `TransversalCert`, `A2BranchCurve`, `SideState(u8)` with
`From<&MaterialState4>` declared as a CTE-006 obligation (declare the target
type here; the impl lands in truck-shapeops in CTE-006 — a one-line forward
declaration comment is enough, no cross-crate dependency), `CoincidenceWitness`,
carrier/trim-domain/stratum shapes for T2 (consuming side only).

## Fixture kit (tangency/fixtures.rs; booking §5, normative)

Each fixture is a hand-computable ground truth machine-checked at admission:

- **F1 (T1.4 identity)**: `h = 3z1² − 5z2²`, `det A = 1` ⇒
  `det DT = det A³·det H_h = −60`. Exact integer arithmetic in the test.
- **F2 (T1.1 identity)**: two random rational charts; the chart-minor
  identity residual is exactly 0 for both j (rational arithmetic in-test).
- **F3 (A₁⁺)**: plane × rational sphere (stereographic chart) tangential,
  known `z*`.
- **F4 (A₁⁻)**: plane `z=0` × rational saddle bipatch `z = x² − y²`; node at
  the origin. This fixture is the R3 gate.
- **F5 (A₂)**: plane `z=0` × extruded parabola bipatch `z = u1²` (degree
  (2,1)); witness `s=a=1, q=u1` hand-written.
- **F6 (transversal)**: plane × plane crossing with a minor-sign-straddling
  box (the T1.5(1) sharpening case).
- **F7 (T2)**: reuse the landed `boolean_m2` fixture *values* (copied as
  constants, read-only — do not import the test module).

## Anchors — measured this session; re-check on your branch before building

Locate by pattern, never by line number. If a count differs, STOP and report
`ANCHOR_MISMATCH` with what you saw.

| id | file | pattern | expect |
|---|---|---|---|
| A1 | `truck-certified/src/ssi_types.rs` | `pub struct SquareSystem3` | 1 |
| A2 | `truck-certified/src/formal/exact.rs` | `pub struct CertifiedInterval` | 1 |
| A3 | `truck-certified/src/formal/exact.rs` | `pub struct Expansion` | 1 |
| A4 | `truck-certified/src/lib.rs` | `^pub mod` | 13 |
| A5 | `truck-certified/src/kernel/rational.rs` | `cone_torus_carrier_packet_pending` | 1 |

A4 becomes 14 when you add `pub mod tangency;`.

## House rules

- **H-1** No `unwrap`, `expect`, `panic!`, `unimplemented!`, `todo!`, or
  out-of-range indexing reachable from geometry.
- **H-2** Fallible operations return `Result<T, named>` (the crate's refusal
  vocabulary), never a bare enum without a named cause.
- **H-3** No absolute constants in predicates; test epsilons carry `// H-3`
  on the SAME line as the literal.
- **Determinism**: identical ordered input → identical verdicts; no output
  ordering from hash iteration.
- **All cargo invocations go through the queue (the `cargo` on PATH IS the
  queue shim). Do not invoke cargo by absolute path; do not unset the shim.**
- Never run a bare `cargo test` — use the scoped commands below.

## Tests required

Named `#[test]` fns; the verifier checks the names appear in your diff:

1. `det_a_constructor_refuses_zero_containing_enclosure`
2. `graph_enclosure_only_input_of_hessian_signature` — compiles-uses the
   R2 signature (a compile-time shape assertion in the test module).
3. `definiteness_carries_positive_mu` — `Definite` with `mu <= 0` refuses.
4. `witness_data_refuses_incomplete_identity` — a witness with mismatched
   `side`/term shapes refuses construction.
5. `f1_t14_identity_instance_exact` — the F1 ground truth, exact integers.
6. `f4_saddle_fixture_admits` — the A₁⁻ fixture constructs and admits.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-certified
cargo clippy -p truck-certified --all-targets -- -D warnings
cargo test -p truck-certified --lib tangency
cargo check -p truck-shapeops
```

Send cargo output to a file and read the tail.

## Forbidden

Editing any file outside `write_allow` — especially `src/ssi.rs`,
`src/ssi_types.rs`, `kernel/**`, `formal/**`, anything under
`truck-shapeops/`, `Cargo.lock`. Implementing any numeric evaluator in this
packet (that is later packets' work). Adding `#[ignore]`. Adding `#[allow]`
without a justification comment on the same line. Committing to `main`.

## Stop conditions

- any anchor count differs → `ANCHOR_MISMATCH`
- a fixture ground truth cannot be stated without fabricating the
  certificate → `SPEC_GAP`
- three consecutive failed `cargo test` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` at the WORKTREE ROOT (then COMMIT first)

**COMMIT BEFORE writing `RESULT.json`.** Then write `RESULT.json` at the root
of your worktree.

```json
{"id":"CTE-000-SPINE","status":"DONE","contracts":["CTE-000-SPINE"],
 "tests_added":6,"anchors_verified":{"A1":1,"A2":1,"A3":1,"A4":13,"A5":1},
 "notes":"shape inventory frozen; mapping rows added; any contract deviation from spine §2 stated here"}
```

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit on the current branch with subject
`feat(certified): tangency shim — certificate vocabulary + fixture kit (CTE-000-SPINE)`.
