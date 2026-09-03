# BG-KV2-103-IDENTITY — node identity Rules A/B/C + the dyadic sampling join

Wave-1 implementation packet (build spec §4). Lands v2 spec §4.2 (node
identity, three rules) and §4.3 (Theorem 4.1's deterministic dyadic join) as
NEW code in `kernel/identity.rs`, built entirely on shim types. **No solver
bodies**: the uniqueness premises arrive as landed certificates (the shim's
`PointCert`); the rules are pure box/relation logic. The one landed
D2-contradiction (shapeops' `near_pt` node welding) is NOT touched — its
replacement is a booked seam, outside this write set.

**H-1.** New module `identity.rs` carries the crate's
`#![deny(clippy::unwrap_used)]` discipline (crate-level deny covers it): no
`unwrap`/`expect`/`panic!`, no module-level `allow`. Copy the header style
from `hull.rs`.

```yaml
id:          BG-KV2-103-IDENTITY
contract:    [BG-KV2-103-IDENTITY]
class:       design
crates:      [truck-certified]
depends_on:  [BG-KV2-000-CONTRACT]
write_allow:
  - vendor/truck/truck-certified/src/kernel/identity.rs
  - vendor/truck/truck-certified/src/kernel/mod.rs
  - vendor/truck/truck-certified/tests/kernel_identity.rs
read_allow:
  - docs/CONSTRUCTIVE_GEOMETRY_KERNEL_SPEC_V2.md
  - docs/KERNEL_V2_BUILD_SPEC.md
  - vendor/truck/truck-certified/src/kernel
  - vendor/truck/truck-certified/src/formal/exact.rs
  - vendor/truck/truck-certified/src/formal/deck.rs
budget:      {turns: 30, ctx_tokens: 100000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub struct PointCert' vendor/truck/truck-certified/src/kernel/certs.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub fn implication' vendor/truck/truck-certified/src/kernel/residual.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'pub mod kernel;' vendor/truck/truck-certified/src/lib.rs"}
  - {id: A4, expect: 0, cmd: "grep -rnw 'rule_a_identify' vendor/truck/truck-certified/src | wc -l"}
  - {id: A5, expect: 1, cmd: "grep -c 'fn next_after(' vendor/truck/truck-certified/src/formal/deck.rs"}
tests_required:
  - rule_a_identifies_same_residual_unique_root_in_union
  - rule_a_refuses_different_residuals_and_noncontained_unions
  - rule_b_transports_deck_translations_exactly
  - rule_b_transports_affine_reparams_with_outward_rounding
  - rule_c_identifies_through_implication_only
  - identity_never_uses_distance_or_tolerance
  - dyadic_join_is_associative_commutative_idempotent
  - dyadic_join_is_order_independent_under_randomized_gather
  - nondyadic_shared_request_refuses
```

## Section 1 — Rules A/B/C (`kernel/identity.rs`, NEW)

```rust
pub enum IdentityVerdict {
    /// The two certificate neighborhoods are certified equal by the cited
    /// rule; carries the rule for the evidence trail.
    CertifiedEqual { rule: IdentityRule },
    /// No rule applies — the nodes are NOT certified equal (the caller
    /// refuses rather than snaps, §4.2 closing rule).
    NotCertified,
}
pub enum IdentityRule { RuleA, RuleB, RuleC }
```

- **Rule A** — `pub fn rule_a(a: &PointCert, b: &PointCert, union_cert:
  &PointCert) -> IdentityVerdict`. Requires: `a.residual == b.residual ==
  union_cert.residual` (else NotCertified); `union_cert.box_` contains the
  union hull of `a.box_` and `b.box_` componentwise (else NotCertified —
  the caller owes the C1 certificate on B\* = □hull(B₁∪B₂); this fn checks
  containment, it does not solve). Union hull is exact f64 min/max — no
  tolerance anywhere. NEVER `a.box_ ∩ b.box_` (the spec's named error).
- **Rule B** — `pub fn rule_b_transport(b: &PointCert, deck: (i32, i32),
  affine: Option<[[f64; 2]; 2]>) -> Construction<PointCert>`. Deck
  translation: exact integer shift of the box's u/v bounds offset by
  `deck * period` — the period arrives as a dyadic-representable f64
  argument (`period_u`, `period_v`) and the shift is exact (periods in this
  kernel are dyadic-representable by construction; assert the product is
  exactly representable and refuse `RefusalKind::NonFinite` otherwise).
  Affine leaf reparameterization: outward-rounded interval evaluation of
  the exact map — push each computed bound outward one ULP with std's
  `f64::next_up`/`f64::next_down` (bit-exact, deterministic, no libm);
  the landed deck.rs steppers are private and stay untouched (their
  `next_after` discipline is the prior art, anchor A5). Then Rule A on the
  transported certificate. Outward rounding preserves containment, which
  is all Rule A needs (spec verbatim in the doc).
- **Rule C** — `pub fn rule_c(a: &PointCert, b: &PointCert, union_certs:
  &[(ResidualId, PointCert)]) -> IdentityVerdict`. Find a common weaker
  residual R with `implication(a.residual, R) != None` and
  `implication(b.residual, R) != None` using the shim's typed relation;
  then Rule A against the union cert for R. The admissible set is the
  shim's — this fn adds no implications.

All three refuse-not-snap: anything ambiguous is `NotCertified`. A source
test pins that NO `dist < eps`-style comparison exists in the module (N/D2
audit; same-line `// H-3` opt-outs for the unit-basis comparisons that
remain).

## Section 2 — The dyadic join (§4.3, Theorem 4.1)

```rust
/// A dyadic refinement request on [a, b]: a finite prefix-closed set of
/// binary node addresses at depth d.
pub struct DyadicRequest { a: f64, b: f64, depth: u32, leaves: BTreeSet<u64> }
pub struct EdgeSampleSet { a: f64, b: f64, depth: u32, nodes: BTreeSet<u64> }
pub fn join(base: DyadicRequest, others: &[DyadicRequest]) -> Construction<EdgeSampleSet>
pub fn sample_parameters(s: &EdgeSampleSet) -> Vec<f64>
```

- Addresses: within one edge, depth is common after normalization — a
  request at depth d' < d lifts to depth d by expanding its leaves
  (integer bit operations ONLY: address k at depth d' expands to the
  2^(d-d') children addresses k*2^(d-d') through (k+1)*2^(d-d') - 1, or
  kept as the parent prefix — prefix-closed semantics make expansion exact
  integer work).
- `join` = set union of prefix-closed address sets, associative,
  commutative, idempotent — BTreeSet, never HashMap (N3). The property
  tests prove Theorem 4.1's three laws under randomized gather orders
  (fixed seed, recorded).
- `sample_parameters` generates `a + (b−a)·k/2^d` from addresses by the
  fixed formula in a fixed order (spec: float comparison occurs only
  INSIDE each requester converting tolerance to depth; the join itself is
  integer — assert no float comparison participates in the join).
- `pub fn refuse_custom_on_shared(face_count: usize, policy:
  SamplingFlag) -> Construction<()>` — the §4.3 guard: a
  CustomParameters-style request on an edge incident to more than one face
  refuses `RefusalKind::NonDyadicSharedRequest` (Disproven). `SamplingFlag`
  is a local two-variant enum {Dyadic, Custom} — the landed
  `truck-geometry` `SamplingPolicy` is NOT modified (write-set discipline);
  the C1 wave wires the real policy type to this guard.

## Section 3 — tests

The nine `tests_required` names. Key machine-checked ground truths:
- Rule A: two `PointCert`s on `ResidualId::R1` with boxes [0.4,0.6]² and
  [0.5,0.7]², union cert box [0.35,0.75]² containing the hull →
  CertifiedEqual{RuleA}; different residuals → NotCertified; union cert
  not containing the hull → NotCertified.
- Rule B: deck (1, 0) with period 1.0 transports box [0.2,0.3] → [1.2,1.3]
  EXACTLY (f64 equality); an affine scaling [2,0;0,1] transported box
  contains the scaled points with outward-rounded bounds (assert lo ≤
  exact, hi ≥ exact, and lo is the largest representable ≤ exact via
  `toward_neg`/`toward_pos` — one boundary ULP each, stated numerically).
- Rule C: an R2 residual cert identifies with an R1 cert through the
  implication; an R8 cert identifies with NOTHING (the §4.2 table).
- Dyadic: join of {depth 3, leaves {0b010}} with itself = itself
  (idempotent); join(A,B) == join(B,A) and join(join(A,B),C) ==
  join(A,join(B,C)) over a fixed corpus; randomized gather orders
  (shuffled by a fixed-seed LCG) produce identical `nodes` sets.
- NonDyadic: face_count > 1 + Custom → the named refusal; face_count 1 +
  Custom → Ok.

## Done-when

- `cargo check -p truck-certified --all-targets` green (CARGO_BUILD_JOBS=2-4).
- `cargo test -p truck-certified --lib --tests --no-fail-fast` green.
- fmt clean; clippy (exact verify form, unfiltered, ALL findings) clean on
  the packet's files.
- `cargo check --workspace --all-targets` green.

## Stop conditions

1. The shim's `PointCert`/`implication`/`IBox` shapes differ from the
   quoted contract — stop, record the diff (frozen-shape rule).
2. Rule B's exact-deck-shift premise fails for a needed period (not
   exactly representable) — record the period and the obstruction; do not
   silently round (that is the D2 violation the module exists to prevent).
3. The dyadic address normalization needs float comparisons to stay
   prefix-closed — stop; the design is wrong and the spec (Theorem 4.1)
   says integer-only.

## Finish by writing `RESULT.json` AT THE WORKTREE ROOT

Commit on the current branch (subject: `feat(certified): node identity
Rules A/B/C + dyadic sampling join (BG-KV2-103-IDENTITY)`) BEFORE writing
`RESULT.json`.
