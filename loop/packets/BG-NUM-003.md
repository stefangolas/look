# WORK PACKET BG-NUM-003 — the Krawczyk operator

You are implementing one item from a formal kernel specification. Everything you
need is in this document. **Do not read `docs/GENERATION_KERNEL_BUILD_SPEC.md`
or any other spec file** — they are not on your allowlist and this packet is
self-contained. If something you need is genuinely missing, that is a SPEC_GAP
(see "Stop conditions"): you stop and report, you do not research it.

```json
{"id":"BG-NUM-003","status":"DONE","contracts":["BG-NUM-003"],
 "tests_added":6,"sites_migrated":0,"sites_deferred":0,
 "deviations":[],"disagreements":[],"baseline_failures":[],
 "notes":"free text"}
```

Fill `deviations`, `disagreements` and `baseline_failures` as arrays, empty if
empty. `disagreements` is the highest-value field: every formula below was
compiled and **RUN** in a scratch crate against real witnesses before this
packet was written (transverse quadratic, tangential double root, no-root
quadratic, nonsingular 2x2 linear system; the measured outcomes are quoted
inline), but they are exactly the kind of claim that can be confidently wrong.
**If anything below contradicts what you find in the code, say so in
`disagreements` rather than making the code match the packet.**

```yaml
id:          BG-NUM-003
contract:    [BG-NUM-003]
class:       design
crates:      [truck-evidence]
write_allow:
  - vendor/truck/truck-evidence/src/num/krawczyk.rs
read_allow:
  - vendor/truck/truck-evidence/src/num/mod.rs
  - vendor/truck/truck-evidence/src/num/roots.rs
  - vendor/truck/truck-evidence/src/lib.rs
  - vendor/truck/truck-evidence/Cargo.toml
  - vendor/truck/truck-base/src/evidence.rs
tests_required:
  - transverse_quadratic_certifies_unique_one_shot
  - tangential_double_root_refuses_indeterminate
  - budget_exhaustion_carries_spend
  - linear_system_certifies_one_shot
  - no_root_box_proves_no_root
  - empty_input_refuses_empty
budget:      {turns: 30, ctx_tokens: 60000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub mod krawczyk' vendor/truck/truck-evidence/src/num/mod.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'KrawczykIndeterminate' vendor/truck/truck-base/src/evidence.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'fn spend_subdiv' vendor/truck/truck-base/src/evidence.rs"}
  - {id: A4, expect: 0, cmd: "grep -c 'unscaled_legacy(' vendor/truck/truck-evidence/src/num/krawczyk.rs"}
  - {id: A5, expect: 2, cmd: "grep -c 'BG-NUM-002' vendor/truck/truck-evidence/src/num/mod.rs"}
```

(`grep -c` exits 1 on zero matches — a count of 0 IS the expected answer for
A4, not a command failure.)

## Problem

`vendor/truck/truck-evidence/src/num/krawczyk.rs` is scaffolded with a doc
comment recording the contract (keep that doc comment; extend it — decision
7). You fill in the operator:

```
K(Q) = m − Y·F(m) + (I − Y·J(Q))·(Q − m)
```

Existence AND uniqueness of a system's solution in a box, proven in interval
arithmetic. The two failure directions are both silent-wrong-answer classes:

- accepting `K ⊆ Q` **non-strict** proves existence but NOT uniqueness;
- emitting "no root" for a box whose root could not be resolved (the
  tangential double root) hides a real root behind a clean-looking answer.

Everything you write goes in the ONE file `num/krawczyk.rs`. The module tree
(`num/mod.rs`, `num/roots.rs`, `lib.rs`) is already landed and is READ-ONLY to
you — neither worker of the NUM pair touches it.

## Decisions already made for you

### 0. Public API, exact shapes

```rust
use inari::Interval;
use truck_base::evidence::{
    Budget, Certificate, Certified, Margin, Method, Modulus, Outcome, PropMap,
    Refusal, UnresolvedWitness,
};
use std::array;

/// A system the Krawczyk operator can prove things about.
pub trait KrawczykSystem<const N: usize> {
    /// F at a POINT, evaluated exactly (each component wrapped as a
    /// degenerate interval). Never evaluate F over the whole box here.
    fn f_point(&self, x: &[f64; N]) -> [Interval; N];
    /// The interval Jacobian over a box. ROW-MAJOR:
    /// `jacobian(b)[r][c] = lower..upper of dF_r/dx_c over b`.
    /// This convention is yours to rely on — the operator never transposes.
    fn jacobian(&self, b: &[Interval; N]) -> [[Interval; N]; N];
    /// A float approximate inverse of J at a point. `None` means the
    /// system cannot supply one here (singular derivative) — the operator
    /// BISECTS on None, it does not refuse (decision 4).
    fn preconditioner(&self, x: &[f64; N]) -> Option<[[f64; N]; N]>;
}

/// What the operator proved about the box.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KrawczykProof {
    /// Exactly one solution in the box (strict-interior rule, decision 5).
    Unique,
    /// No solution in the searched region.
    NoRoot,
}

/// The Krawczyk existence/uniqueness operator over a worklist.
pub fn krawczyk<const N: usize>(
    system: &impl KrawczykSystem<N>,
    start: &[Interval; N],
    budget: &mut Budget,
) -> Outcome<KrawczykProof>
```

`Outcome<T> = Result<Certified<T>, Refusal>` — re-exported through
`truck_base::evidence`; check the actual import path resolves in-crate and
adapt the `use` lines, not the signatures.

### 1. The operator body

A `Vec<[Interval; N]>` worklist stack, initialised with `*start`. Loop while
it pops a box `q`:

1. Any component empty or non-finite (`is_empty()`, `!inf().is_finite()`,
   `!sup().is_finite()`) → `Refusal::Empty`.
2. `m[i]` = float midpoint, `0.5 * (inf + sup)` per axis. Then VERIFY
   `q.inf() <= m && m <= q.sup()` per axis — naive midpoints can round
   outside their own box at extreme magnitudes. A failed membership check
   takes the SAME branch as a `None` preconditioner: bisect (decision 4),
   refuse only at zero width.
3. `y = system.preconditioner(&m)`.
4. `f = system.f_point(&m)` — the POINT evaluation at the midpoints. NEVER
   the interval F over Q: the interval center decorrelates the linear part
   against the contraction term and no box ever certifies (measured on the
   BG-ENC-004-ISC carrier: K ≥ 5×width(Q) at every scale with the interval
   center, second-order width with the point center — the scaffold doc
   comment already records this; keep it recorded).
5. `j = system.jacobian(&q)` — row-major interval Jacobian.
6. Per output row r: `k[r] = iv(m[r]) − Σ_c y[r][c]·f[c]
   + Σ_c d[r][c]·(q[c] − iv(m[c]))`, where `d = I − Y·J` computed
   row-major (`d[r][c] = δ(r,c) − y[r][c]·j[r][c]`).
7. **Strict** interior containment on ALL axes —
   `k.inf() > q.inf() && k.sup() < q.sup()` componentwise, no empty k —
   → return `Certified::new(KrawczykProof::Unique, certificate(budget))`.
   Strictness IS uniqueness; non-strict containment proves existence only.
   Write the comparison in exactly this form (a packet-mandated rewrite to
   `!(...)` form fails GATE V-clippy; see the H-3 section).
8. Else if ANY axis has `k.intersection(q).is_empty()` → NO root in this
   box: discard it, continue the loop.
9. Else bisect (decision 4).

When the worklist empties without certification:
`Certified::new(KrawczykProof::NoRoot, certificate(budget))`.

`certificate(budget)` builds `Certificate { props: PropMap::new(),
method: Method::Interval, budget_left: *budget, margin: Margin::UNBOUNDED,
modulus: Modulus::Unbounded }`.

### 2. Spent accounting

On any refusal carrying `spent`: capture `initial = *budget` at entry, and
report `spent = Budget { subdiv: initial.subdiv − budget.subdiv, newton:
… , depth: … }` fieldwise. Do not report the REMAINING budget as `spent` —
that is the tempting bug and it makes exhaustion unobservable.

### 3. Bisecting

Bisect the WIDEST axis (largest `sup − inf`; break ties toward the LOWEST
axis index deterministically). Split at the axis midpoint into lo/hi halves
hulling back to the original box. Before recursing:

- `budget.spend_subdiv(1)` — on `Err(_)` return
  `Refusal::NumericallyUnresolved { spent, witness:
  UnresolvedWitness::KrawczykIndeterminate }`.
- An axis of ZERO width that is neither strictly contained nor disjoint
  (a degenerate point box) cannot subdivide — same refusal, same witness.
  Do NOT spin, do NOT panic.

### 4. `None` preconditioner BISECTS — this is a calibration, do not "fix" it

Measured finding: a `None` preconditioner at the box midpoint (e.g. x²+1 at
m = 0, vanishing derivative) MUST take the bisection path, not a refusal. A
vanishing midpoint derivative says nothing about the box; refusing there
turns every symmetric no-root instance into a spurious
`NumericallyUnresolved`. Bisecting costs nothing when the answer is NoRoot
(the children prune) and is honest when a genuine structure sits at the
midpoint. The scratch crate first refused here and the no-root witness
failed because of it; the bisect branch is what makes test
`no_root_box_proves_no_root` terminate. The same branch serves the
degenerate-midpoint case of decision 1 step 2.

### 5. The strict-interior rule

`Proven(unique)` is emitted ONLY on strict interior containment
(decision 1 step 7). The common bug — accepting `K ⊆ Q` non-strict —
proves existence but not uniqueness and is the exact defect the spec
amendment calls out. There is no inflation step and no padding scheme in
THIS operator: callers widen their own boxes. Keep it that way.

### 6. Element access without indexing — H-1 is enforced at the crate root

`truck-evidence` denies `clippy::indexing_slicing`,
`unwrap_used`, `expect_used` and `panic` crate-wide (see `lib.rs`). All
matrix/vector element access goes through iterators, `enumerate`, `zip`,
and `std::array::from_fn` — never `x[i]` with a runtime `i`. Float max/by
comparisons use `f64::total_cmp`, never `partial_cmp(...).unwrap()`.
Interval construction uses the house helper
`fn interval_at(x: f64) -> Interval { Interval::try_from((x,
x)).unwrap_or(Interval::EMPTY) }` (test-side unwraps are covered by the
test module's allow block, decision 8).

### 7. Module docs

Keep the scaffold's doc comment (the contract block quote and the
point-center paragraph are both correct and load-bearing) and add: the trait
contract (row-major Jacobian convention; the system supplies its own float
inverse so the operator holds no linear-algebra machinery), the
None-preconditioner-bisects rule with its one-line justification, the
worklist/bisection shape under Budget, and the two proof outcomes. State
that over-estimation never occurs silently: every non-proof exit is a typed
refusal carrying spend.

### 8. Witnesses and tests (all in the module's `#[cfg(test)]`, opening with
the standard `#[allow(clippy::unwrap_used, clippy::expect_used)]` +
H-1 justification comment)

Witnesses, copied from the validated scratch:

```rust
struct Quad(f64, f64, f64); // a·x² + b·x + c, univariate
// f_point: [iv(a·x² + b·x + c)]; jacobian: [[iv(2a)·b[0] + iv(b)]];
// preconditioner: d = 2a·x + b, None iff d == 0.0, else Some([[1.0/d]])

struct Lin2([[f64; 2]; 2], [f64; 2]); // A·x − w
// f_point components: iv(A[r][0]·x[0] + A[r][1]·x[1] − w[r]);
// jacobian: constant A as degenerate intervals; preconditioner: the float
// adjugate inverse, None iff det == 0.0
```

By name (helper `iv(lo, hi)` = `Interval::try_from((lo, hi)).unwrap()`,
test-side):

- `transverse_quadratic_certifies_unique_one_shot` — x²−2 on [1, 2],
  `Budget::new(4, 0, 0)`: `Ok(KrawczykProof::Unique)` AND
  `budget.subdiv == 4` afterwards (untouched — proving the one-shot claim;
  this assertion is what exposed the wrong exhaustion premise recorded
  below, so keep it sharp).
- `tangential_double_root_refuses_indeterminate` — x² on [−1, 1],
  `Budget::new(64, 0, 0)`: `Err(NumericallyUnresolved)` with witness
  `KrawczykIndeterminate` and `spent.subdiv == 64` (every subdivision
  actually consumed — measured outcome of the scratch run).
- `budget_exhaustion_carries_spend` — the same tangential witness with
  `Budget::new(3, 0, 0)`: refuses with `spent.subdiv == 3`. NOTE: the
  originally-planned exhaustion negative (zero budget on the TRANSVERSE
  case) is WRONG — measured: x²−2 on [1, 2] certifies one-shot even at
  `Budget::new(0, 0, 0)` because certification needs no subdivision. Only
  a case that actually bisects can exhaust; the tangential one does.
- `linear_system_certifies_one_shot` — Lin2([[2.0, 1.0], [1.0, 3.0]],
  [5.0, 10.0]) on ([−10, 10], [−10, 10]), `Budget::new(16, 0, 0)`:
  `Ok(Unique)`, subdiv untouched (measured: one-shot at any budget).
- `no_root_box_proves_no_root` — x²+1 on [−2, 2], `Budget::new(1024, 0,
  0)`: `Ok(KrawczykProof::NoRoot)`. This test FAILS if decision 4's
  bisect-on-None regresses to a refusal (measured: the refusal variant
  dies at m = 0 with a `None` preconditioner) — it is the regression guard
  for the calibration, not a decorative extra.
- `empty_input_refuses_empty` — x²−2 with an EMPTY first component
  (`iv(1.0, 1.0).intersection(iv(2.0, 3.0))` or `Interval::EMPTY`
  directly): `Err(Refusal::Empty)`.

Out of scope, deliberately: the spec's property test against BG-NUM-002
("two independent methods must agree") — `num/roots.rs` is still a scaffold
with no implementation to compare against; that cross-validation lands with
BG-NUM-002 and must not be stubbed, ignored, or approximated here. Say so
in `RESULT.json` notes.

## H-3, the house rule that rejects bare float literals

GATE-2 of `scripts/kernel-gates.sh` fails any **added** line carrying a bare
`1e-N` literal (the regex catches `1e-6`, `1.0e-6`, ...) unless that same
line ends with an `// H-3` comment. This packet's constants are all
small-integer budgets and `0.5`/`2.0`-class witness decimals — if YOU
introduce any absolute float constant (a floor width, a pad), name it as a
`const` whose defining line carries a same-line `// H-3:` comment naming the
dimensionless quantity it is. Run `bash scripts/kernel-gates.sh` yourself
before writing `RESULT.json`; it is the same script V4 runs.

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
written against (HEAD ddcd706): the full lib suite passes, zero clippy
findings (the crate denies `clippy::all` at its root and is clean). Your bar:
everything above stays green plus your six new tests. Any baseline failure you
did not cause is a stop condition; any failure you did cause is yours to fix.

## Forbidden

Editing any file outside `write_allow` — in particular `num/mod.rs`,
`num/roots.rs` and `lib.rs` (already landed; neither NUM packet touches them),
`truck-base/**` (the evidence algebra is a read-only dependency), and every
other crate. Adding `#[ignore]`. Stubs for the NUM-002 cross-property.
Adding `unwrap()`/`expect()`/`panic!` on fallible paths in production code
(the test module's allow block is the house exception). Committing to `main`.

## Stop conditions

- an anchor count differs → `ANCHOR_MISMATCH`, naming the file and what you saw
- `Budget`'s field names, `spend_subdiv`'s signature (it is
  `spend_subdiv(&mut self, n: u32) -> Result<(), Exhausted>`), or
  `UnresolvedWitness::KrawczykIndeterminate` differ from what this packet
  states → `SPEC_GAP`, with the exact signature you found
- the tangential witness EVER returns `Ok(Unique)` with the formulas verbatim
  → `SPEC_GAP` — the strict-interior rule would be violated; report what K
  looks like relative to Q near the double root
- three consecutive failed `cargo` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit on the current branch with subject
`feat(evidence): the Krawczyk existence/uniqueness operator (BG-NUM-003)`.
