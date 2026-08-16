# WORK PACKET BG-EVD-r3 — the modulus contract, r2 shape to r3 shape

You are implementing one item from a formal kernel specification. Everything you
need is in this document. **Do not read `docs/GENERATION_KERNEL_BUILD_SPEC.md`
or any other spec file** — they are not on your allowlist and this packet is
self-contained. If something you need is genuinely missing, that is a SPEC_GAP
(see "Stop conditions"): you stop and report, you do not research it.

```yaml
id:          BG-EVD-r3
contract:    [BG-EVD-004]
class:       mechanical
crates:      [truck-base]
depends_on:  []
write_allow:
  - vendor/truck/truck-base/src/evidence.rs
read_allow:
  - vendor/truck/truck-modeling/src/geometry.rs   # the reference answer, see "Template"
tests_required:
  - modulus_shape_decides_subadditivity
  - propagate_never_exceeds_split_bound_on_subadditive_chains
  - split_bound_under_reports_through_a_pole
  - compose_refuses_a_non_subadditive_operand
  - pole_modulus_is_finite_inside_its_domain
budget:      {turns: 40, ctx_tokens: 100000}
```

## Problem

`Modulus` is the kernel's modulus of continuity ω: it says how much an output
error can grow from an input error. Today's `Modulus::compose` implements the
**split bound** unconditionally — it composes two moduli and lets callers sum
the per-step tolerances separately.

That is only valid when ω is **subadditive**: ω(a+b) ≤ ω(a) + ω(b). Every
modulus in the tree today happens to be subadditive, so the arithmetic is
currently right — but that is a property of today's cells, not of the theorem.
The first non-subadditive modulus makes the split bound **under-report** the
true error, and an under-reported error bound is worse than no bound: it is a
certificate that claims accuracy the computation does not have.

This item makes subadditivity a property **decided from the shape**, never
declared by a caller, and makes the unconditionally-valid recurrence the default
path.

## Anchors — verified 2026-08-16, counts are exact

Locate by running the `rg` command. **Never locate by line number.**
**If a count differs, STOP** and report `ANCHOR_MISMATCH` with what you saw.
All are in `vendor/truck/truck-base/src/evidence.rs`.

| # | `rg` pattern | expect |
|---|---|---|
| A1 | `pub enum Modulus` | **1** |
| A2 | `pub fn compose` | **1** |
| A3 | `pub enum Refusal` | **1** |
| A4 | `modulus: Modulus` | **3** |

## The design — all of it is decided; implement it, do not re-litigate

### 1. Split the shape out of the modulus

```rust
/// The shape of ω. Subadditivity is read off this and never declared.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ModulusShape {
    /// ω(ε) = k·ε.
    Lipschitz(f64),
    /// ω(ε) = k·ε^p. Tangency is p = 1/2.
    Holder { k: f64, exponent: f64 },
    /// ω(ε) = k·ε / (domain − ε): finite inside the domain, unbounded at its
    /// edge. This is what a near-degenerate cell publishes instead of
    /// `Unbounded` — an honest non-subadditive bound beats no bound at all.
    Pole { k: f64 },
    /// No bound is published.
    Unbounded,
}

/// ω: modulus of continuity, valid on `[0, domain)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Modulus {
    /// The shape, which decides subadditivity.
    pub shape: ModulusShape,
    /// ω is valid only on `[0, domain)`. `f64::INFINITY` for a global bound.
    pub domain: f64,
}
```

### 2. Subadditivity is derived, and the rule is concavity

ω with ω(0) = 0 is subadditive when it is concave. So:

| shape | `is_subadditive()` | why |
|---|---|---|
| `Lipschitz` | `true` | linear |
| `Holder { exponent }` | `exponent <= 1.0` | concave at p ≤ 1, convex above |
| `Pole` | `false` | convex; that is the entire point of the variant |
| `Unbounded` | `false` | nothing to be subadditive about |

Give this to **`Modulus::is_subadditive(&self) -> bool`**. There must be **no way
for a caller to assert subadditivity** — no constructor argument, no setter, no
public field for it.

### 3. `eval` is total, and honest outside its domain

```rust
/// ω(ε). Returns `f64::INFINITY` outside `[0, domain)` and for `Unbounded`:
/// "no bound available here" is a real answer, and a total function keeps this
/// on the right side of H-1.
pub fn eval(&self, eps: f64) -> f64
```

`Pole { k }` evaluates to `k * eps / (domain - eps)`. A negative `eps`, a NaN
`eps`, or `eps >= domain` gives `INFINITY`. Never panic, never index.

### 4. `propagate` is the default path, `compose` becomes the opt-in fast path

The nested recurrence is unconditionally valid, subadditive or not:

> `E₀ = τ₀`, and `Eᵢ = ωᵢ(Eᵢ₋₁) + τᵢ`

```rust
/// One step of the forward-error recurrence: ω(incoming) + tau. Always valid.
pub fn propagate(&self, incoming: f64, tau: f64) -> f64

/// Fold the recurrence over a chain of (modulus, tau) steps. Always valid.
pub fn propagate_chain(steps: &[(Modulus, f64)]) -> f64
```

`compose` keeps ω₂∘ω₁ but becomes **fallible**, refusing a non-subadditive
operand rather than silently producing a bound that may under-report:

```rust
/// ω₂ ∘ ω₁, the split-bound fast path. Refuses unless BOTH operands are
/// subadditive (BG-EVD-004 M4).
pub fn compose(&self, other: &Self) -> Outcome<Modulus>
```

Composition arithmetic, unchanged from today where it applies: `Lipschitz(a) ∘
Lipschitz(b) = Lipschitz(ab)`; Hölder exponents multiply and constants multiply;
anything with `Unbounded` is `Unbounded`. The composed `domain` is the **minimum
of the two domains** — a composite is valid only where both parts are.

The refusal is the new variant:

```rust
    /// A forward error bound exceeded what the operation could certify
    /// (BG-EVD-004). Also raised when the split bound is requested for a chain
    /// that has not been shown subadditive.
    ForwardToleranceExceeded {
        /// The bound that was computed.
        bound: f64,
        /// The largest bound that would have been acceptable.
        allowed: f64,
    },
```

Add it to `Refusal`. `allowed` is `f64::INFINITY` when the refusal is about
subadditivity rather than about a numeric tolerance.

### 5. `concave_majorant` is the escape hatch

```rust
/// The tightest subadditive modulus that dominates this one on its domain, so a
/// caller holding a non-subadditive modulus can still reach the fast path by
/// paying for a looser bound.
pub fn concave_majorant(&self) -> Modulus
```

`Lipschitz` and `Holder { exponent <= 1.0 }` are already subadditive and return
themselves. For `Holder { exponent > 1.0 }` and for `Pole`, return the
`Lipschitz` chord over the domain — the line through `(0, 0)` and
`(d, ω(d))` for a finite domain `d`, which dominates a convex ω on `[0, d]`.
Where the domain is infinite and the shape is convex, no finite Lipschitz chord
exists, so return `Unbounded`. Assert in a test that the majorant is
`is_subadditive()` and dominates the original at sampled points.

### 6. The 38 existing call sites must keep compiling

Every use of `Modulus` outside this file — measured 2026-08-16, **38 sites
across 10 files in 5 crates** — is the literal `Modulus::Unbounded` in a struct
field initializer. None is a match pattern. So a compatibility associated
constant keeps every one of them working while the write set stays at this one
file:

```rust
impl Modulus {
    /// Compatibility with the 38 `Modulus::Unbounded` call sites the r2 shape
    /// left behind; BG-EVD-r3b renames them and deletes this.
    #[allow(non_upper_case_globals)] // deliberate: it stands in for a variant path
    pub const Unbounded: Modulus = Modulus {
        shape: ModulusShape::Unbounded,
        domain: f64::INFINITY,
    };
}
```

**`cargo check --workspace --all-targets` must pass**, and that is what proves
this worked. If it does not, do not start editing other crates to fix it —
that is out of scope and out of allowlist. Report `BLOCKED` with the errors.

### 7. `Certificate::accumulate` keeps working, conservatively

Accumulation currently calls `compose`, which is now fallible. Accumulation
itself must not become fallible in this packet — that ripples. Where `compose`
refuses, accumulation records `Modulus::Unbounded`, which is the honest
conservative answer ("no bound published") rather than a bound that might
under-report. Mark the site:

```rust
// TODO(BG-EVD-r3b): thread propagate() through accumulation so a
// non-subadditive chain still publishes its real bound instead of Unbounded.
```

This is the packet's one judgement call and it is already made; **say in your
`RESULT.json` notes that you implemented it**, so a reviewer sees it.

## Template — the reference answer

`vendor/truck/truck-modeling/src/geometry.rs` holds BG-S0-001's landed diff:
`fn include`, `fn include_intersection_curve`, and the
`include_intersection_curve_tests` module. Copy its style — certificate
construction, comment voice, test module layout.

## House rules

- **H-1** No `unwrap`, `expect`, `panic!`, `unimplemented!`, `todo!`, or
  out-of-range indexing reachable from geometry. `eval` returning `INFINITY` is
  how this item honours it.
- **H-2** Fallible operations return `Outcome<T>` — never `Option`, never a
  bare `Result`.
- **H-3** No absolute constants in predicates. `exponent <= 1.0` is a
  dimensionless exponent, not a length, and is therefore fine.
- **H-6** A value computed in floats is never recorded as `Method::Exact`.

## Tests required

Each must be a named `#[test]` fn — the verifier checks the names appear in your
diff, so a test you describe but do not write fails the gate.

1. `modulus_shape_decides_subadditivity` — the table in §2, every row.
2. `propagate_never_exceeds_split_bound_on_subadditive_chains` — over many
   random subadditive chains and random τ's, `propagate_chain` ≤ the split
   bound. The recurrence is never looser; that is why it is safe as the default.
3. `split_bound_under_reports_through_a_pole` — **the test this whole item
   exists for.** Build a chain containing a `Pole` modulus and exhibit an input
   where the split bound is **strictly smaller** than `propagate_chain`, i.e.
   where using it would under-report the error.
4. `compose_refuses_a_non_subadditive_operand` — `compose` on that same chain
   returns `Refusal::ForwardToleranceExceeded`, rather than returning the
   under-reporting bound.
5. `pole_modulus_is_finite_inside_its_domain` — a `Pole` evaluates finite for
   `eps` inside `[0, domain)`, `INFINITY` at and beyond `domain`, and
   `INFINITY` for negative and NaN input, without panicking.

Use a fixed seed for anything random so a failure is reproducible. No existing
test may be deleted, `#[ignore]`d, or weakened.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-base
cargo clippy -p truck-base --all-targets --no-deps -- -D warnings
cargo test -p truck-base --lib --tests
cargo check --workspace --all-targets
```

`--no-deps` matters: without it clippy lints the vendored path dependencies too,
and truck-meshalgo's 93 pre-existing lints abort the run before it reaches your
crate. Never run a bare `cargo test` — it builds 56 examples. Send cargo output
to a file and read the tail.

## Forbidden

Editing any file outside `write_allow`. Adding any caller-facing way to declare
subadditivity. Making `Certificate::accumulate` fallible. Adding `#[ignore]`.
Adding `#[allow]` without a justification comment on the same line — the one in
§6 is pre-approved and already carries its comment. Committing to `main`.

## Stop conditions

- an anchor count differs → `ANCHOR_MISMATCH`
- `cargo check --workspace` cannot pass without editing another crate →
  `BLOCKED`, with the errors
- a required test cannot be written without inventing a rule this packet does
  not state → `SPEC_GAP`, naming the readings you could not choose between

## Finish by writing `RESULT.json` in the root of your worktree

```json
{"id":"BG-EVD-r3","status":"DONE","contracts":["BG-EVD-004"],
 "tests_added":5,"anchors_verified":{"A1":1,"A2":1,"A3":1,"A4":3},
 "notes":"how you handled accumulate's conservative fallback, and the input where the split bound under-reports through a Pole"}
```

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it: what you attempted, the
exact ambiguity, and the readings you could not choose between.

Commit on the current branch with subject
`feat(base): subadditivity is decided from the shape, not declared (BG-EVD-r3)`.
