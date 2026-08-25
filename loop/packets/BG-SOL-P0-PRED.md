# WORK PACKET BG-SOL-P0-PRED — certified predicates (`orient2d` with adaptive escalation) and the `CurveContact` ontology

You are implementing the solver family's predicate substrate: a certified 2-D
orientation predicate whose fast path is float-filtered and whose slow path
escalates to **exact** sign computation, plus the 2-D `CurveContact` ontology
types that S1 (arrange) and the Contact Layer will both speak. Everything you
need is in this document. **Do not read any other spec file** — this packet is
self-contained. It implements the approved design in
`docs/SOLVER_FAMILY_PLAN.md` §2 and §4 (Phase 0, `truck-base` modules `pred`
and `contact`).

```json
{"id":"BG-SOL-P0-PRED","status":"DONE","contracts":["BG-SOL-P0-PRED"],
 "tests_added":5,"deviations":[],"disagreements":[],
 "baseline_failures":[],"notes":"free text"}
```

Fill `deviations`, `disagreements` and `baseline_failures` as arrays, empty if
empty. **If anything below contradicts what you find in the code, say so in
`disagreements` rather than making the code match the packet.**

```yaml
id:          BG-SOL-P0-PRED
class:       design
crates:      [truck-base]
write_allow:
  - vendor/truck/truck-base/src/pred.rs
  - vendor/truck/truck-base/src/contact.rs
read_allow:
  - vendor/truck/truck-base/src/evidence.rs
tests_required:
  - orient2d_clear_cases_are_filtered
  - orient2d_near_degenerate_escalates_to_exact
  - orient2d_collinear_escalates_to_exact
  - orient2d_non_finite_input_is_unresolved
  - curve_contact_types_construct_and_match
budget:      {turns: 60, ctx_tokens: 140000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub mod pred' vendor/truck/truck-base/src/lib.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub mod contact' vendor/truck/truck-base/src/lib.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'clippy::unwrap_used' vendor/truck/truck-base/src/pred.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'clippy::unwrap_used' vendor/truck/truck-base/src/contact.rs"}
```

## Problem

Topology-changing predicates are never naked f64 comparisons
(plan §2): `orient2d`, event ordering, exact tangency and endpoint membership
decide which vertices exist, which edges are adjacent, and which face is on
which side of a seam — a one-ulp error there is not a bad number, it is a
different topology. This packet ships the first one, `orient2d`, with the
discipline every later predicate inherits: a **fast float filter** that returns
`Proven` when the float sign is certain, and an **exact escalation** that
computes the true sign when it is not. `Unresolved` is a result, never a crash.

## Design decisions already made for you

### 1. The result types — `pred.rs`

```rust
/// The trichotomous sign of an orientation predicate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Orientation {
    /// The determinant is negative.
    Clockwise,
    /// The determinant is positive.
    CounterClockwise,
    /// The three points are exactly collinear.
    Collinear,
}

/// Why a predicate could not be decided.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PredUnresolved {
    /// A coordinate is NaN or infinite; no sign exists.
    NonFiniteInput,
    /// The exact escalation cannot represent the sign because the f64
    /// two-product overflows (a coordinate magnitude beyond ~1e150).
    ExactRangeOverflow,
}

/// A certified predicate answer: proven, or honestly unresolved.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CertifiedPred {
    /// The predicate's sign is proven.
    Proven(Orientation),
    /// The predicate could not be decided; `reason` names why.
    Unresolved(PredUnresolved),
}
```

**Deviation from the plan's §4 sketch, recorded here so it is not relitigated:**
the plan writes `pub enum CertifiedPred { Proven, Unresolved(UnresolvedWitness) }`.
A unit `Proven` cannot carry the predicate's trichotomous answer, and the
evidence algebra's `UnresolvedWitness` classifies *refusals of whole certified
operations* (`RootNotIsolated`, `KrawczykIndeterminate`, …), none of which
means "this predicate is undecidable". The packet's `Proven(Orientation)` and
the local two-value `PredUnresolved` are the honest spellings; say so in
`disagreements`.

### 2. `orient2d` — the two-stage algorithm, implement exactly

```rust
/// The exact orientation predicate: the sign of the determinant
/// `(b - a) x (c - a)` in 2-D. Positive is counterclockwise.
/// Filtered, then exact; never a naked f64 comparison.
pub fn orient2d(a: Point2, b: Point2, c: Point2) -> CertifiedPred;
```

`Point2` is `cgmath::Point2<f64>` (`crate::cgmath64::Point2`). Stage 1 — the
float filter:

```rust
let acx = a.x - c.x;
let bcx = b.x - c.x;
let acy = a.y - c.y;
let bcy = b.y - c.y;
let detleft = acx * bcy;
let detright = acy * bcx;
let det = detleft - detright;
let detsum = detleft.abs() + detright.abs();
let errbound = CCDETERRBOUND * detsum;
// CCDETERRBOUND is a named const:
const CCDETERRBOUND: f64 = (3.0 + 16.0 * f64::EPSILON) * f64::EPSILON;
```

If `det > errbound` → `Proven(CounterClockwise)`; if `det < -errbound` →
`Proven(Clockwise)`; otherwise the sign is not yet certain and you **escalate**
to stage 2 — you never guess. First check finiteness: if any of `a.x`, `a.y`,
`b.x`, `b.y`, `c.x`, `c.y` is not finite → `Unresolved(NonFiniteInput)`.

Stage 2 — the exact sign by expansion arithmetic (the classic adaptive-precision
orientation predicate; the packet names the primitives so you implement them,
not design them):

- `two_product(x, y) -> (hi, lo)` — the exact error-free split product
  (`hi = x*y` rounded, `lo` the exact residual; implement via the standard
  `SPLITTER = 134217729.0` splitting or Dekker's algorithm).
- `two_diff(x, y) -> (hi, lo)` — exact difference via two-sum.
- `fast_expansion_sum_zeroelim(h, f) -> Vec<f64>` — the zero-eliminating
  expansion sum.
- The exact determinant expansion: compute `acx, bcx, acy, bcy` as exact
  `two_diff` pairs, then the four products `two_product(acx, bcy)` and
  `two_product(acy, bcx)`, then
  `det = fast_expansion_sum_zeroelim(± products)` so the expansion's sign IS
  the exact sign of the determinant. If any `two_product` returns an infinite
  component (the inputs' product overflowed f64) → `Unresolved(ExactRangeOverflow)`.
- The sign of a nonzero expansion is read off its largest-magnitude component;
  if the expansion is all zero → `Proven(Collinear)`.

The escalation must return the **exact** trichotomy — there is no
"approximately collinear" in this predicate. The witness tests below pin that.

### 3. The `CurveContact` ontology — `contact.rs`

```rust
/// The dimension of a curve-curve contact locus.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContactDimension {
    /// A single parameter pair (an isolated point contact).
    Point0,
    /// A one-dimensional contact (an arc of coincident curves).
    Arc1,
    /// A two-dimensional contact (a region; reserved for the 2-D overlap
    /// case in S1/S5.3).
    Region2,
}

/// The event kind of a curve-curve contact.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContactEventKind {
    /// Two curves cross at a point with distinct tangents.
    Transverse,
    /// The tangents agree at the contact point.
    Tangency,
    /// The contact is at the endpoint of one or both curves.
    EndpointTouch,
    /// The curves coincide over an interval (their images overlap).
    CoincidentInterval,
    /// The two curves share a carrier (provenance-identical).
    IdenticalCarrier,
}

/// A contact between two curves, defined once in 2-D and reused by 3-D
/// (plan §2). The parameter lists carry the contact locus on each curve in
/// its own parameterization: `Point0` has one entry per side; `Arc1` has the
/// interval endpoints per side; `Region2` is Phase-1 defined. The values here
/// are the solver's best certified parameters; the refined `Certified<...>`
/// forms land in S1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CurveContact {
    pub dimension: ContactDimension,
    pub kind: ContactEventKind,
    /// Contact locus parameters on the lhs curve (per `dimension`).
    pub params_lhs: Vec<f64>,
    /// Contact locus parameters on the rhs curve (per `dimension`).
    pub params_rhs: Vec<f64>,
}
```

`serde` derive is available in `truck-base` (its Cargo.toml has
`serde` with the `derive` feature). The types are vocabulary only — no methods
beyond construction through the pub fields; S1 refines the semantics.

## H-3, the house rule that rejects bare float literals

GATE-2 of `scripts/kernel-gates.sh` fails any **added** line carrying a bare
`1e-N`-shaped literal unless that line ends with an `// H-3` comment. The only
small literal this packet's code needs is the filter bound, which is spelled
`CCDETERRBOUND` from `f64::EPSILON` (a name, not a bare literal — fine). The
test witnesses use **integer and dyadic coordinates** (below), so no test
tolerance is needed at all. Run `bash scripts/kernel-gates.sh <your base
commit>` yourself before writing `RESULT.json`.

## GATE-4 / `unscaled_legacy` (the ratchet)

This packet adds NO `unscaled_legacy()` calls. Do not touch
`scripts/unscaled_legacy_ceiling.txt` — the orchestrator owns the ratchet.

## Regression tests (exact names)

The witnesses are machine-checked (orchestrator-verified with exact rational
arithmetic before dispatch). `Point2::new(x, y)` from `crate::cgmath64`.

1. `orient2d_clear_cases_are_filtered`
   - `orient2d((0,0), (1,0), (0,1))` → `Proven(CounterClockwise)` (det +1).
   - `orient2d((0,0), (0,1), (1,0))` → `Proven(Clockwise)` (det −1).
   - `orient2d((0,0), (2,1), (4,2))` → `Proven(Collinear)` (det exactly 0).
2. `orient2d_near_degenerate_escalates_to_exact`
   - `orient2d((0,0), (10000000000000000, 10000000000000001), (10000000000000002, 10000000000000003))`
     → `Proven(Clockwise)`. Exact determinant is **−2**; the float filter is
     INCONCLUSIVE (errbound ≈ 26.6 ≫ |det|), so this test only passes if stage 2
     actually runs and returns the exact sign.
3. `orient2d_collinear_escalates_to_exact`
   - `orient2d((0,0), (1000000001, 1000000001), (1000000002, 1000000002))`
     → `Proven(Collinear)`. Exact determinant is **0**; the filter is
     INCONCLUSIVE (errbound ≈ 1.3e-6 > 0), so this test only passes if stage 2
     computes the exact zero.
4. `orient2d_non_finite_input_is_unresolved`
   - `orient2d(Point2::new(f64::NAN, 0.0), (0,1), (1,0))` → `Unresolved(NonFiniteInput)`.
   - `orient2d((0,0), (f64::INFINITY, 1.0), (1,0))` → `Unresolved(NonFiniteInput)`.
5. `curve_contact_types_construct_and_match`
   - construct a `CurveContact` of each `ContactDimension` and each
     `ContactEventKind`; assert `Clone`, `PartialEq`, and that the pub fields
     round-trip through construction. (No serde_json round-trip: `truck-base`
     has no `serde_json` dependency — the derive compiling is the serialization
     contract.)

Every other existing truck-base test must stay green.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-base
cargo clippy -p truck-base --all-targets --no-deps
cargo test -p truck-base --lib --tests --no-fail-fast
cargo test -p truck-base --doc
cargo check --locked -p truck-base --all-targets
bash scripts/kernel-gates.sh <your base commit>
```

Never run a bare `cargo test`.

## Forbidden

Editing any file outside `write_allow`. Adding a *finite* answer where the
predicate is genuinely undecidable — `Unresolved` is the honest terminal state
and there is no fallback guess. Guessing the sign inside the filter's
uncertain band. Adding `#[ignore]`. Changing the GATE-4 ceiling. Adding
methods to the `CurveContact` types beyond the pub fields (S1 refines).

## Stop conditions

- an anchor count differs → `ANCHOR_MISMATCH`, naming the file and what you saw
- a pre-existing test you did not expect to touch is broken → do NOT weaken the
  gate; report it in `disagreements` with the failing test name and the exact
  reason
- three consecutive failed `cargo` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it. In `notes`, record the
two deviations you made explicit (the `Proven(Orientation)` and
`PredUnresolved` spellings) and the measured escalation path on the witnesses
(i.e. that tests 2 and 3 pass through stage 2, not the filter).

Commit on the current branch with subject
`feat(base): certified orient2d with exact escalation and CurveContact ontology (BG-SOL-P0-PRED)`.
