# WORK PACKET BG-NUM-002 — certified univariate root isolation

You are implementing one item from a formal kernel specification. Everything you
need is in this document. **Do not read `docs/GENERATION_KERNEL_BUILD_SPEC.md`
or any other spec file** — they are not on your allowlist and this packet is
self-contained. If something you need is genuinely missing, that is a SPEC_GAP
(see "Stop conditions"): you stop and report, you do not research it.

```json
{"id":"BG-NUM-002","status":"DONE","contracts":["BG-NUM-002"],
 "tests_added":6,"sites_migrated":0,"sites_deferred":0,
 "deviations":[],"disagreements":[],"baseline_failures":[],
 "notes":"free text"}
```

Fill `deviations`, `disagreements` and `baseline_failures` as arrays, empty if
empty. The algorithm below was worked through BY HAND on the four witnesses
before this packet was written (the coefficient sequences quoted in the test
section were computed, not guessed), but hand algebra is exactly the kind of
claim that can be confidently wrong. **If anything below contradicts what you
find by computing the witnesses yourself, say so in `disagreements` rather than
making the code match the packet.**

```yaml
id:          BG-NUM-002
contract:    [BG-NUM-002]
class:       design
crates:      [truck-evidence]
write_allow:
  - vendor/truck/truck-evidence/src/num/roots.rs
read_allow:
  - vendor/truck/truck-evidence/src/num/mod.rs
  - vendor/truck/truck-evidence/src/num/krawczyk.rs
  - vendor/truck/truck-evidence/src/lib.rs
  - vendor/truck/truck-evidence/Cargo.toml
  - vendor/truck/truck-base/src/evidence.rs
tests_required:
  - simple_root_is_isolated_narrow_and_unique
  - double_root_refuses_never_an_empty_list
  - no_root_returns_certified_empty_vec
  - clustered_roots_separate_with_enough_budget
  - clustered_roots_refuse_without_enough_budget
  - empty_domain_refuses_empty
budget:      {turns: 30, ctx_tokens: 60000}
anchors:
  - {id: B1, expect: 1, cmd: "grep -c 'pub mod roots' vendor/truck/truck-evidence/src/num/mod.rs"}
  - {id: B2, expect: 1, cmd: "grep -c 'RootNotIsolated' vendor/truck/truck-base/src/evidence.rs"}
  - {id: B3, expect: 0, cmd: "grep -c 'unscaled_legacy(' vendor/truck/truck-evidence/src/num/roots.rs"}
  - {id: B4, expect: 2, cmd: "grep -c 'BG-NUM-003' vendor/truck/truck-evidence/src/num/mod.rs"}
```

(`grep -c` exits 1 on zero matches — a count of 0 IS the expected answer for
B3, not a command failure.)

## Problem

`vendor/truck/truck-evidence/src/num/roots.rs` is scaffolded with a doc
comment recording the contract (keep it; extend it — decision 6). You fill in
the operator: certified isolation of the real roots of a polynomial given by
its **Bernstein coefficients** over a domain interval.

The failure direction that gives this module its reason to exist: a
**tangential double root** has an all-one-sign Bernstein coefficient sequence
with a zero in it (worked example below), and the naive reading — "no sign
changes, so no root" — returns an EMPTY list for a box containing a root.
Reporting "no root" for a tangential contact is precisely the silent wrong
answer this module exists to prevent; the empty-list outcome must be reserved
for boxes CERTIFIED root-free, and a multiple-root contact must be a typed
refusal.

Everything you write goes in the ONE file `num/roots.rs`. The module tree is
landed and READ-ONLY — neither NUM packet touches it.

## Decisions already made for you

### 0. Public API, exact shape

```rust
use inari::Interval;
use truck_base::evidence::{
    Budget, Certificate, Certified, Margin, Method, Modulus, Outcome, PropMap,
    Refusal, UnresolvedWitness,
};
use std::array;

/// Isolates the real roots of a polynomial in its Bernstein basis over
/// `domain`.
///
/// `coeffs[i]` is the degree-`len−1` Bernstein coefficient at basis index i
/// over `domain`. Returns one isolating interval per distinct simple real
/// root found: every returned interval has width `< tau`, contains exactly
/// one root, and their union contains every simple root in the open domain.
pub fn isolate_roots(
    coeffs: &[f64],
    domain: (f64, f64),
    tau: f64,
    budget: &mut Budget,
) -> Outcome<Vec<Interval>>
```

- `domain.lo >= domain.hi`, a non-finite bound, a non-finite `tau`,
  `tau <= 0.0`, or any non-finite coefficient → `Refusal::Empty`.
- Roots exactly ON the domain endpoints: a zero endpoint coefficient blocks
  pruning like any other zero (decision 2), which drives subdivision toward
  the boundary and eventually exhausts budget → `NumericallyUnresolved`.
  This is deliberate: endpoint multiplicity is ambiguous under floating
  evaluation, and a refusal is sound where a guess would not be. Document it
  in the module docs as a known, typed limitation.
- The certificate on success: `Certificate { props: PropMap::new(),
  method: Method::Interval, budget_left: *budget, margin:
  Margin::from_log2(<log2 of the smallest returned interval width>),
  modulus: Modulus::Unbounded }`. If the vec is empty use
  `Margin::UNBOUNDED`. (`Margin::from_log2` exists on truck-base's Margin;
  check its exact constructor name and adapt.)

### 1. The per-box classification

A worklist stack of `(lo: f64, hi: f64, coeffs: Vec<f64>)`, initialised from
the input. Pop a box `b` and classify by its coefficient sequence:

1. Any non-finite coefficient → `Refusal::Empty` (should be caught at entry;
   keep the arm total).
2. Count `v` = strict sign changes over the sequence **after deleting exact
   zeros**, and record `has_zero` = whether any exact `0.0` coefficient
   remains deleted. All coefficients strictly positive or all strictly
   negative (`v == 0 && !has_zero`) → NO root in this box: prune, continue.
3. `v == 0 && has_zero` → the hull touches zero without crossing it: an
   even-multiplicity contact. Do NOT prune, do NOT emit. Bisect (decision 4)
   while the width exceeds the floor; at the floor refuse (decision 5).
   Worked example: `(2t−1)²` over `[0,1]` has Bernstein sequence
   `[1, 0, 1]` — zero variations WITH a zero, and a double root sitting at
   the midpoint that no amount of subdivision will isolate.
4. `v == 1` → exactly one simple root in this box. If `hi − lo < tau`, EMIT
   `Interval::try_from((lo, hi))` (H-1 house helper) and continue.
   Otherwise bisect (decision 4).
5. `v >= 2` → several roots or unresolved structure: bisect (decision 4).

When the worklist empties, return the collected intervals sorted by lower
endpoint (deterministic output order), wrapped in the certificate.

### 2. Why zero coefficients block pruning

The Bernstein convex-hull property makes "all coefficients same STRICT sign"
imply "no root" — but only strictness licenses it. A zero coefficient means
the control polygon touches the axis, and for even-multiplicity roots that
touching IS the root's only signature (step 3's witness never changes sign
anywhere). Dropping zeros before concluding is the tempting bug and it is
exactly how `[1, 0, 1]` becomes a silent empty answer. The rule costs one
extra branch and buys the strongest negative guarantee in Stage 3.

### 3. de Casteljau bisection

Bisecting a Bernstein polynomial = splitting its coefficient sequence at the
midpoint via de Casteljau subdivision: repeated pairwise averaging produces
the LEFT child's coefficients from the front ends and the RIGHT child's from
the back ends. Implement it with iterators/`windows`/rolling accumulators —
**no runtime indexing** (the crate denies `clippy::indexing_slicing`; see
decision 6). Both children inherit the parent's parameter interval halves;
NO reparametrisation of coefficients is needed because the midpoint split of
a Bernstein sequence is basis-covariant. Each bisection consumes ONE
subdivision from the budget.

### 4. Budget discipline

Every bisection calls `budget.spend_subdiv(1)` BEFORE pushing children — on
`Err(_)` return `Refusal::NumericallyUnresolved { spent, witness:
UnresolvedWitness::RootNotIsolated }`. Report SPENT as initial-minus-
remaining fieldwise (never the remaining ledger — that is the tempting bug).
The width floor is `const WIDTH_FLOOR: f64 = 8.0 * f64::EPSILON;` (name it
with an H-3 comment): at or below the floor a box cannot subdivide further,
and step 3's contact case refuses there rather than spinning.

### 5. Exhaustion vs emptiness — keep the distinction sharp

`Ok(vec![])` is a CERTIFIED claim ("no simple roots in the domain") and
`Err(RootNotIsolated)` is a typed failure ("structure I could not resolve").
They must never blur: the tangential witness MUST take the second path. This
distinction is the single most important behaviour of the module; the test
names below encode it.

### 6. Module docs and H-1

Keep the scaffold doc comment and add: the API contract (what `coeffs`
means, what is promised per returned interval, the tau semantics), the
endpoint-root limitation, the zero-coefficient rule with the `[1, 0, 1]`
example, the de Casteljau note, and the exhaustion-vs-emptiness distinction.
`truck-evidence` denies `unwrap_used`/`expect_used`/`panic`/
`indexing_slicing` crate-wide: element access through iterators, `windows`,
and rolling variables; float comparisons through `f64::total_cmp` where an
ordering is needed; `Interval::try_from((lo, hi)).unwrap_or(Interval::EMPTY)`
as the house construction (test-side unwraps sit under the test module's
allow block).

### 7. Witnesses and tests (all in the module's `#[cfg(test)]`, opening with
the standard `#[allow(clippy::unwrap_used, clippy::expect_used)]` + H-1
justification comment)

Coefficient helpers: power-to-Bernstein conversion for the fixed witnesses
is done BY HAND inline (write the literal sequences, don't implement a
converter). For degree n on `[0,1]`: `b_i` comes from the power form by
accumulation — the four sequences you need are already computed here:

- `p(t) = t − 0.5` → `[−0.5, 0.5]` (degree 1)
- `p(t) = (2t−1)²` → `[1.0, 0.0, 1.0]` (degree 2)
- `p(t) = t² + 1` → `[1.0, 1.0, 2.0]` (degree 2)
- `p_s(t) = (t−(0.5−s))(t−(0.5+s))` → `[0.25−s², −s², 0.25−s²]` (degree 2)

By name:

- `simple_root_is_isolated_narrow_and_unique` — witness 1, `tau` a named
  const `1.0e-6` (H-3 comment): `Ok(v)` with `v.len() == 1`, width
  `< 1e-6`, and `0.5 ∈ v`.
- `double_root_refuses_never_an_empty_list` — witness 2, generous budget:
  `Err(NumericallyUnresolved)` with witness `RootNotIsolated` and
  `spent.subdiv > 0` (the ledger actually moved — subdivision happened
  before refusal). THE core negative of Stage 3.
- `no_root_returns_certified_empty_vec` — witness 3: `Ok(v)` with
  `v.is_empty()` — the certified-empty outcome, deliberately distinct from
  the refusal above.
- `clustered_roots_separate_with_enough_budget` — witness 4 with
  `s = 2^-12` (named const, H-3 comment naming it the cluster half-width):
  budget `32` subdivisions ≥ `log2(1/s)+slack`: `Ok(v)`, `v.len() == 2`,
  the intervals disjoint, each containing its own root
  (`0.5 ∓ s`), each narrower than tau.
- `clustered_roots_refuse_without_enough_budget` — the same witness with
  budget `4` (< log2(1/s)): `Err(... RootNotIsolated ...)`.
- `empty_domain_refuses_empty` — `domain = (1.0, 1.0)`: `Err(Refusal::Empty)`
  (degenerate-width domain refuses at entry).

Out of scope, deliberately: the spec's property test against exact rational
arithmetic (no rational tier exists in the tree yet — that cross-validation
lands with the exact-arithmetic packet and must not be stubbed, ignored, or
approximated here); likewise the Krawczyk cross-validation property of
BG-NUM-003 rides with whichever of the two packets lands SECOND — do not
reference `crate::num::krawczyk` from tests unless it already exists in YOUR
worktree's base commit; if it does, adding the cross-check is welcome but
optional, and it goes in `RESULT.json` notes either way.

## H-3, the house rule that rejects bare float literals

GATE-2 of `scripts/kernel-gates.sh` fails any **added** line carrying a bare
`1e-N` literal unless that same line ends with an `// H-3` comment. This
packet's constants — `tau`, the cluster half-width, `WIDTH_FLOOR` — are named
consts whose defining lines carry same-line `// H-3:` comments naming the
dimensionless quantity each is. Witness decimals are
`0.5`/`0.25`/`2.0`-class. Run `bash scripts/kernel-gates.sh` yourself before
writing `RESULT.json`.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-evidence
cargo clippy -p truck-evidence --all-targets --no-deps
cargo test -p truck-evidence --lib --tests --no-fail-fast
cargo check --workspace --all-targets
bash scripts/kernel-gates.sh <your base commit>
```

Never run a bare `cargo test` — it builds 56 examples. Send cargo output to a
file and read the tail.

**The crate is clean at baseline** — measured at the tree this packet was
written against (HEAD 182b2c0): the full lib suite passes, zero clippy
findings. Your bar: everything stays green plus your six new tests. Any
baseline failure you did not cause is a stop condition; any failure you did
cause is yours to fix.

## Forbidden

Editing any file outside `write_allow` — in particular `num/mod.rs`,
`num/krawczyk.rs` and `lib.rs`, `truck-base/**`, and every other crate.
Returning an empty `Vec` for anything except the certified-no-simple-roots
outcome. Widening `tau` semantics or weakening an assertion to get green.
Adding `#[ignore]`. Adding `unwrap()`/`expect()`/`panic!` on fallible paths
in production code (the test module's allow block is the house exception).
Committing to `main`.

## Stop conditions

- an anchor count differs → `ANCHOR_MISMATCH`, naming the file and what you saw
- the double-root witness EVER returns `Ok` (either flavor) with the rules
  verbatim → `SPEC_GAP` — decision 2 would be violated; report your computed
  coefficient sequence for `(2t−1)²` on `[0,1]`
- `Budget`'s fields, `spend_subdiv`'s signature, or
  `UnresolvedWitness::RootNotIsolated` differ from what this packet states →
  `SPEC_GAP`, with the exact signature you found
- three consecutive failed `cargo` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit on the current branch with subject
`feat(evidence): certified univariate root isolation (BG-NUM-002)`.
