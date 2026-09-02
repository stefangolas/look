# BG-CK-P2-CONTRACT — the SSI wave shim: shared types + synthetic fixture kit

The pre-wave contract packet for the Phase-2 implementation wave (ORCHESTRATOR
"Wave mode"). Its ONLY job is to materialize the already-decided shared shapes
the four wave packets exchange — `SquareSystem3`, `KrawczykCertificate3`,
`TraceStep`/`TraceOutcome`, and the fixture kit — as landed, verified code, so
the four implementation branches can fork from one base and build against
frozen contracts instead of each other. **No solver implementation**: every
evaluator refuses; the types, the refusal vocabulary, and the fixtures are the
deliverable. The BG-CK-P0-FREEZE pattern exactly.

The mathematics is frozen in `docs/CERTIFIED_PHASE2_BOOKING.md` and the
frozen F3 contract (`src/contract.rs`): this packet adds no decisions and
invents no evidence kinds. Reuse over redefinition: `ContinuationCoordinate`,
`CoordinateSwitch`, `SquareSystemInput`, `ConditioningBelowThreshold`,
`BranchGerm`, `BranchIncidence` are LANDED — import, never restate.

```yaml
id:          BG-CK-P2-CONTRACT
contract:    [BG-CK-P2-CONTRACT]
class:       design
crates:      [truck-certified, look]
depends_on:  [BG-CK-P1-DISPATCH, BG-CK-P1-FLOOR]
write_allow:
  - vendor/truck/truck-certified/src/ssi_types.rs
  - vendor/truck/truck-certified/src/ssi_fixtures.rs
  - vendor/truck/truck-certified/src/lib.rs
  - vendor/truck/truck-certified/tests/ssi_contract.rs
  - Cargo.toml
  - Cargo.lock
read_allow:
  - CERTIFIED-KERNEL-PLAN.md
  - docs/CERTIFIED_PHASE2_BOOKING.md
  - docs/CERTIFICATE_MAPPING.md
  - vendor/truck/truck-certified/src/lib.rs
  - vendor/truck/truck-certified/src/contract.rs
  - vendor/truck/truck-certified/src/hull.rs
  - vendor/truck/truck-certified/src/formal/exact.rs
  - vendor/truck/truck-certified/src/formal/numeric.rs
  - vendor/truck/truck-certified/src/formal/contact.rs
  - vendor/truck/truck-certified/src/formal/span.rs
budget:      {turns: 30, ctx_tokens: 100000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub struct SquareSystemInput' vendor/truck/truck-certified/src/contract.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub enum BranchGerm' vendor/truck/truck-certified/src/formal/span.rs"}
  - {id: A3, expect: 0, cmd: "grep -c 'pub mod ssi_types;' vendor/truck/truck-certified/src/lib.rs"}
  - {id: A4, expect: 0, cmd: "grep -rnw 'SquareSystem3' vendor/truck/truck-certified/src | wc -l"}
  - {id: A5, expect: 4, cmd: "grep -c 'ConditioningBelowThreshold' vendor/truck/truck-certified/src/contract.rs"}
  - {id: A6, expect: 1, cmd: "grep -c 'pub struct CoordinateSwitch' vendor/truck/truck-certified/src/contract.rs"}
tests_required:
  - square_system3_refuses_ragged_empty_or_nonfinite_grids
  - krawczyk_certificate3_is_built_only_from_strict_inclusion
  - trace_step_carries_box_germ_and_certificates
  - trace_outcome_refusals_are_named_cases
  - fixture_well_conditioned_root_matches_ground_truth
  - fixture_determinant_spans_zero_is_constructible
  - fixture_germ_ladder_covers_all_branch_germ_variants
  - shim_never_implements_a_solver
```

## Pre-made decisions (do not relitigate; quote the tags into the module doc)

**H-1.** Crate-level `#![deny(clippy::unwrap_used)]` covers both new modules.
No `unwrap`/`expect`/`panic!`, no module-level `allow`.

**D-shim.** Types and refusing constructors only. Any method that would
evaluate, solve, isolate, or certify NUMERICALLY refuses
(`InvalidInput`-shaped or a named case from the existing vocabularies). The
module doc says verbatim: "This module freezes shapes; BG-CK-P2-SYSTEM /
KRAWCZYK3 / TRACE implement against it and never restate it."

**D-reuse.** `contract.rs`'s frozen F3 types are the vocabulary:
`ContinuationCoordinate`, `CoordinateSwitch`, `SquareSystemInput`,
`Refusal::ConditioningBelowThreshold`. `formal/span.rs`'s `BranchGerm`,
`formal/contact.rs`'s `BranchIncidence`. The shim wraps/aliases; it never
duplicates a landed type under a new name.

**D-homogeneous.** `SquareSystem3` carries the cross-multiplied homogeneous
system `F_k = W2*P1_k − W1*P2_k` (k ∈ x,y,z) as tensor-Bernstein coefficient
grids over `(u,v) x (s,t)` — the KRAWCZYK3 packet's `K(X)` contract operates
on this. The weight certificates `W1, W2 > 0` are INPUTS (carried as the
patches' own landed certificates), never re-derived here.

**D-fixtures-public.** `ssi_fixtures.rs` is `#[doc(hidden)] pub` — test
support only, explicitly excluded from the certified API surface in the
module doc (a one-line mapping-table note, not a row: no new evidence kind).
Wave workers' integration tests consume it through the crate's public path;
`#[cfg(test)]`-only items would be invisible to them.

## Section 1 — `truck-certified/src/ssi_types.rs` (NEW)

Header: crate lint style. Module doc: D-shim/D-reuse/D-homogeneous above,
each tagged.

```rust
/// The stored square-system representation (SYSTEM's output contract).
/// F_k(u,v,s,t) = W2*P1_k - W1*P2_k as tensor-Bernstein grids; the 3x4
/// Jacobian is DERIVED by consumers via the landed hull kernels, not
/// stored. Constructed only through `SquareSystem3::new`, which refuses
/// ragged/empty grids, non-finite coefficients, and degree-0 inputs.
#[derive(Debug, Clone)]
pub struct SquareSystem3 { /* f_k grids: [Vec<Vec<f64>>; 3], degrees (m1,n1,m2,n2), domain maps (a1,b1,c1,d1,a2,b2,c2,d2) */ }

/// The Krawczyk unique-root certificate (KRAWCZYK3's output contract).
/// Constructed ONLY from a strict inclusion: `new` refuses a non-strict
/// or boundary inclusion (K(X) must be component-wise strictly inside
/// X) — the frozen emission rule made typecheckable. Carries the box,
/// the K(X) enclosure, and the determinant enclosure (0 excluded).
#[derive(Debug, Clone)]
pub struct KrawczykCertificate3 { /* box_x: [(f64,f64);3], k_x: [(f64,f64);3], det: (f64,f64) with 0 excluded */ }

/// One traced branch box (TRACE's per-step output): the parameter box in
/// the 4D chart, the germ class, the branch incidence record, and BOTH
/// certificates the frozen F3 rule requires at any switch box.
#[derive(Debug, Clone)]
pub struct TraceStep { /* box: 4 interval bounds, germ: BranchGerm, incidence: BranchIncidence, coordinate: ContinuationCoordinate */ }

/// The outcome of tracing one branch from one seed. Shape mirrors the
/// landed pair-contact results: named cases, no catch-all.
#[derive(Debug, Clone)]
pub enum TraceOutcome {
    /// The branch closed on itself (identity recurrence) — the loop's
    /// first box id equals the closing box id.
    ClosedLoop { steps: Vec<TraceStep> },
    /// The branch terminated at a certified boundary/refusal-free end.
    Terminated { steps: Vec<TraceStep> },
    /// A certified turning-point switch occurred mid-branch.
    Switched { steps: Vec<TraceStep>, switch: CoordinateSwitch },
    /// Named refusal cases (reuse the landed vocabularies verbatim:
    /// ConditioningBelowThreshold-shaped conditioning refusal, hull
    /// EnclosureUnavailable, GenericUnresolved causes).
    Refused(TraceRefusal),
}

/// The trace refusal vocabulary: aliases/wraps of LANDED named cases.
/// No new top-level evidence kinds (mapping section C).
#[derive(Debug, Clone)]
pub enum TraceRefusal { /* wrapping variants over the landed named causes */ }
```

Signatures exact in body-shape; every numeric method refuses (this is a
freeze). Accessors only; no public fields except where the contract says
the consumer reads them.

## Section 2 — `truck-certified/src/ssi_fixtures.rs` (NEW, `#[doc(hidden)] pub`)

The synthetic fixture kit: mathematically valid states with known ground
truth, each fixture carrying a doc-stated ground truth that the contract
tests verify by direct evaluation. Required fixtures (names are contract):

- `well_conditioned_root()` — a `SquareSystem3` built from two small
  rational Bézier patches whose cross-multiplied system has exactly one
  known transverse root in the domain interior (ground truth stated and
  machine-checked by direct evaluation at the root and its neighbors).
- `negative_orientation_root()` — same with the parameter order flipped so
  the determinant sign flips (the orientation certificate's other branch).
- `determinant_spans_zero()` — a system whose Jacobian determinant
  enclosure contains zero over the fixture box (constructible; the
  certificate MUST refuse it).
- `conditioning_below_threshold()` — a system whose every coordinate
  margin fails the frozen relative-margin rule at the fixture box.
- `germ_ladder()` — fixtures realizing `Regular`, `StationaryRegular{2}`,
  `CuspCandidate`, `Singular` as branch-germ classifications over
  documented boxes (classification is the consumer's job; the fixture
  guarantees the geometry class by construction and states it).
- `closed_loop_pair()` — two seeds on the same closed branch (identity
  recurrence ground truth).

Construction only; no solving. Each fixture's doc states its ground truth
in numbers.

## Section 3 — lib.rs + re-exports

Two lines: `pub mod ssi_types;`, `pub mod ssi_fixtures;` beside
`pub mod pair_dispatch;`, plus a `pub use ssi_types::{...}` re-export line.
Additionally, the root `Cargo.toml` gains NOTHING this packet (the
FLOOR r2 dev-dependency edge already reaches the crate; confirm the
re-exports resolve from the look test target and say so in RESULT notes —
RESIDUAL depends on that reachability).

## Section 4 — tests (`truck-certified/tests/ssi_contract.rs`, NEW)

The eight `tests_required` names. Load-bearing shapes:

1. `square_system3_refuses_ragged_empty_or_nonfinite_grids` — each
   malformed construction refuses; a valid one constructs with accessors
   returning the inputs verbatim (representation-derived).
2. `krawczyk_certificate3_is_built_only_from_strict_inclusion` — a strict
   inclusion constructs; a non-strict/boundary one refuses the named way;
   a determinant-enclosure-containing-zero refuses (the orientation
   precondition is part of construction, not a later check).
3. `trace_step_carries_box_germ_and_certificates` — construction from the
   landed types (`BranchGerm`, `ContinuationCoordinate`,
   `BranchIncidence`) round-trips.
4. `trace_outcome_refusals_are_named_cases` — every `TraceRefusal` variant
   wraps a landed named cause; the exhaustive match has no catch-all.
5-7. The fixtures' ground truths, machine-checked by direct evaluation
   (`fixture_well_conditioned_root_matches_ground_truth`,
   `fixture_determinant_spans_zero_is_constructible` +
   certificate-refuses-it, `fixture_germ_ladder_covers_all_branch_germ_
   variants` — one fixture per `BranchGerm` variant).
8. `shim_never_implements_a_solver` — source scan: no `CertifiedInterval`
   arithmetic chains in `ssi_types.rs` beyond accessor pass-through; both
   modules contain no `hull_bernstein` calls (the solvers own those).

House rules: H-3 opt-outs same-line; clippy zero findings on the new
files; no manifest change.

## Done-when

- `cargo fmt` clean on the new files; clippy `-p truck-certified`
  zero findings attributable to them.
- `cargo test -p truck-certified --lib --tests --no-fail-fast` green —
  landed suites unchanged plus the contract tests.
- `cargo check --workspace --all-targets` green.
- From the look test target: `use truck_certified::ssi_types::SquareSystem3;`
  resolves (the RESIDUAL reachability fact, verified by a throwaway probe
  or by citing FLOOR's landed dev-dependency edge).

## Stop conditions

Stop, commit nothing beyond WIP evidence, write RESULT.json (AT THE
WORKTREE ROOT) with the finding verbatim if:

1. The landed F3/span/contact types differ from the read (a frozen shape
   moved) — stop, do not adapt silently.
2. A required fixture cannot be constructed with known ground truth
   without solving — the fixture list is frozen; say which fixture and
   what the obstruction is.
3. The re-exports do not resolve from the look test target despite the
   landed dev-dependency edge — record the exact compiler error; the
   reachability assumption is load-bearing for RESIDUAL.

## Finish by writing `RESULT.json` AT THE WORKTREE ROOT

Commit your work on the current branch (subject: `feat(certified): SSI
wave shim — shared types + fixture kit (BG-CK-P2-CONTRACT)`) BEFORE writing
`RESULT.json`.
