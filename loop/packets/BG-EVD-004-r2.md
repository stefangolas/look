# WORK PACKET BG-EVD-004-r2 — the Hölder composition constants are wrong

You are implementing one item from a formal kernel specification. Everything you
need is in this document. **Do not read `docs/GENERATION_KERNEL_BUILD_SPEC.md`
or any other spec file** — they are not on your allowlist and this packet is
self-contained. If something you need is genuinely missing, that is a SPEC_GAP
(see "Stop conditions"): you stop and report, you do not research it.

```json
{"id":"BG-EVD-004-r2","status":"DONE","contracts":["BG-EVD-004"],
 "tests_added":2,"sites_migrated":0,"sites_deferred":0,
 "deviations":[],"disagreements":[],"baseline_failures":[],
 "notes":"free text"}
```

Fill `deviations`, `disagreements` and `baseline_failures` as arrays, empty if
empty. `disagreements` is the highest-value field: if you find the table below
is wrong, say so rather than making the code match it.

```yaml
id:          BG-EVD-004-r2
contract:    [BG-EVD-004]
class:       mechanical
crates:      [truck-base]
depends_on:  [BG-EVD-r3]
write_allow:
  - vendor/truck/truck-base/src/evidence.rs
read_allow:
  - vendor/truck/truck-base/src/tolerance.rs
tests_required:
  - composition_constant_is_order_dependent
  - composition_matches_nested_application_on_every_arm
budget:      {turns: 40, ctx_tokens: 90000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub fn compose' vendor/truck/truck-base/src/evidence.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'fn composed_constant' vendor/truck/truck-base/src/evidence.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'fn compose_math' vendor/truck/truck-base/src/evidence.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'fn modulus_composition_matches_numeric_evaluation' vendor/truck/truck-base/src/evidence.rs"}
```

## Problem

`Modulus::compose` publishes a forward error bound that is **too small** on two
of its four arms. A bound that under-reports error is the one failure this
contract exists to prevent: every certificate produced by
`Certificate::accumulate` composes moduli through this function.

For `self.compose(other)`, `self` is applied **outside**:

    ω_self(ω_other(ε)) = a·(b·ε^q)^p = a·b^p·ε^(p·q)

so the composite constant is **a·b^p**, and it depends on which operand is
outside. The exponents in the current code are right. Two of the four constants
are not:

| ω_self (outer) | ω_other (inner) | correct composite | what the code writes |
|---|---|---|---|
| `Lipschitz(a)` | `Lipschitz(b)` | `Lipschitz(a·b)` | same — correct |
| `Lipschitz(a)` | `Holder{k, q}` | `Holder{a·k, q}` | same — correct |
| `Holder{k, p}` | `Lipschitz(a)` | **`Holder{k·a^p, p}`** | `Holder{k·a, p}` — **wrong** |
| `Holder{k₁, p}` | `Holder{k₂, q}` | **`Holder{k₁·k₂^p, p·q}`** | `Holder{k₁·k₂, p·q}` — **wrong** |

**The error is not conservative.** `compose` only accepts subadditive operands,
so p ≤ 1 always. When the inner constant is below 1 — a *contracting* step,
which is what a well-conditioned projection or a normalisation is — then
`a^p > a`, the true bound exceeds the published one, and the published bound
under-reports by a factor of `a^(p−1)`. Measured: `Holder{1, 0.5} ∘
Lipschitz(0.01)` publishes `0.01·√ε` against a true `0.1·√ε` — a **10×**
under-report at every ε. At an inner constant of 1e-6 the factor is 1000.
Composing an outer tangency (p = ½ is exactly the tangency exponent) with an
inner contraction is an ordinary chain, not a contrived one.

## Decisions already made for you

1. **The convention is `self ∘ other`, self outside.** This is not a choice you
   need to make: the `(Lipschitz(a), Holder{k, q})` arm already computes
   `Holder{a·k, q}`, which is only correct under that reading. Do **not** change
   the convention or the argument order. The doc comment above `compose` says
   "ω₂ ∘ ω₁", which is ambiguous about which is which — **fix the comment to say
   `self ∘ other`, i.e. `self` applied outside** while you are there.

2. **`compose_math` in the test module is already correct — do not delete it and
   do not change its arithmetic.** It computes `k * a.powf(exponent)` and
   `k1 * k2.powf(e1)`, which is the true table above. Its doc comment claims
   production "preserves the r2 arithmetic ... which for Hölder is not the true
   function composition"; that claim is what this packet overturns. **Rewrite
   that comment** to say the two now agree, and keep the helper — the property
   test still needs a reference implementation.

3. **`composed_constant` has the same bug and must be fixed the same way.** It
   is only used for the `bound` field of a `ForwardToleranceExceeded` refusal,
   so it never made a bound too small — but a refusal that reports the wrong
   number is still wrong, and leaving one of two copies fixed guarantees they
   diverge again. Its `Holder`/`Lipschitz` and `Holder`/`Holder` arms take the
   same `a.powf(p)` and `k2.powf(e1)` corrections.

4. **Change nothing else.** Not `is_subadditive`, not `eval`, not `propagate`,
   not `concave_majorant`, not the `Pole`/`Unbounded` arms, not the refusal
   behaviour, not any signature. This is a four-line arithmetic correction plus
   tests.

5. **`powf` is the right call and H-6 is satisfied.** The composite is float
   arithmetic and `compose` already stamps `Method::Float`; do not change that.

## Tests required

Both go in the existing `#[cfg(test)]` module in
`vendor/truck/truck-base/src/evidence.rs`. That module already exists, so you
are adding to a file, not creating one — **no new-module attribute is needed**.

1. `composition_matches_nested_application_on_every_arm` — the property
   BG-EVD-004 has always required and that has never been discharged for a
   Hölder operand. For each of the four arms, and for several ε, assert

       composed.eval(eps) ≈ outer.eval(inner.eval(eps))

   where `composed = outer.compose(&inner).unwrap().value`. **Sample the inner
   constant both below and above 1** — the direction of the old error flips at
   1, and only the below-1 side is unsafe, so a test that only samples above 1
   passes on the broken code. Use at least `Lipschitz(0.01)` and
   `Holder{0.01, 0.5}` as inner operands.

2. `composition_constant_is_order_dependent` — assert directly that
   `Holder{k, p}.compose(&Lipschitz(a))` and `Lipschitz(a).compose(&Holder{k, p})`
   give **different** constants for some `a ≠ 1`, and that each equals the table
   above. This is the test that states the thing the old code got wrong: the
   constants do not simply multiply.

**H-3 applies to your tests.** Comparing two float evaluations needs an epsilon,
and `scripts/kernel-gates.sh` flags a bare float literal on any added line. The
opt-out is a `// H-3` comment **on the same line as the literal**, not the line
above. Say what the quantity is — here it is a dimensionless ratio of two
evaluations of one modulus, not a length. Note that **rustfmt moves a trailing
comment off a line that opens a brace**; if that happens, extract the literal to
its own statement line and mark that.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-base
cargo clippy -p truck-base --all-targets --no-deps -- -D warnings
cargo test -p truck-base --lib --no-fail-fast
cargo check --workspace --all-targets
```

Never run a bare `cargo test` — it builds 56 examples. Send cargo output to a
file and read the tail.

**`modulus_composition_matches_numeric_evaluation` already exists and must keep
passing.** It only exercises `Lipschitz ∘ Lipschitz`, which this change does not
touch. If it moves, something is wrong with your edit, not with the test.

**A downstream crate may hold a bound that shifts.** `truck-evidence` and others
consume `Certificate::accumulate`. If a pre-existing test elsewhere moves,
**confirm it fails identically at the base commit**, record it in
`baseline_failures`, and report it — do not adjust a bound to make it pass. A
test that moves because a forward bound got *larger* is this packet working.

## Forbidden

Editing any file outside `write_allow`. Changing the composition convention or
any signature. Weakening or deleting `compose_math`. Changing `is_subadditive`,
`eval`, `propagate` or `concave_majorant`. Making a bound smaller anywhere.
Adding `#[ignore]`. Committing to `main`.

## Stop conditions

- an anchor count differs → `ANCHOR_MISMATCH`, naming the file and what you saw
- **you work through the algebra and disagree with the table** → do not silently
  implement your version. Put it in `disagreements` with code
  `RULE_MISSING` and your derivation, and implement the table as written. The
  table was derived twice independently and checked numerically, but it is
  exactly the kind of algebra where being confidently wrong is easy, and a
  contradiction from you is worth more than a green build.
- three consecutive failed `cargo test` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit on the current branch with subject
`fix(evidence): Holder composition constants are order-dependent (BG-EVD-004-r2)`.
