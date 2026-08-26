# WORK PACKET BG-SOL-S7-GFF-COVER — certified branch cover for two implicit fields

You are implementing one stage of the solver family's Contact Layer funnel.
Everything you need is in this document. **Do not read
`docs/GENERATION_KERNEL_BUILD_SPEC.md` or any other spec file** — they are not
on your allowlist and this packet is self-contained. If something you need is
genuinely missing, that is a SPEC_GAP (see "Stop conditions"): you stop and
report, you do not research it.

```json
{"id":"BG-SOL-S7-GFF-COVER","status":"DONE","contracts":["BG-SOL-S7-GFF-COVER"],
 "tests_added":0,"sites_migrated":0,"sites_deferred":0,
 "deviations":[],"disagreements":[],"baseline_failures":[],
 "notes":"free text"}
```

Fill `deviations`, `disagreements` and `baseline_failures` as arrays, empty if
empty. **If anything below contradicts what you find in the code, say so in
`disagreements` rather than making the code match the packet.**

```yaml
id:          BG-SOL-S7-GFF-COVER
contract:    [BG-SOL-S7-GFF-COVER]
class:       design
crates:      [truck-evidence]
write_allow:
  - vendor/truck/truck-evidence/src/contact/mod.rs
  - vendor/truck/truck-evidence/src/contact/gff.rs
read_allow:
  - vendor/truck/truck-evidence/src/contact/implicit.rs
  - vendor/truck/truck-evidence/src/contact/mod.rs
  - vendor/truck/truck-evidence/src/enclosure.rs
  - vendor/truck/truck-evidence/src/num/krawczyk.rs
tests_required:
  - transversal_pair_yields_proven_points_on_curve
  - tangent_pair_classifies_singular
  - disjoint_pair_proves_empty
  - empty_boxes_prune_by_interval_exclusion
budget:      {turns: 30, ctx_tokens: 80000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub trait ImplicitField' vendor/truck/truck-evidence/src/contact/implicit.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub fn krawczyk' vendor/truck/truck-evidence/src/num/krawczyk.rs"}
  - {id: A3, expect: 0, cmd: "grep -c 'gff' vendor/truck/truck-evidence/src/contact/mod.rs"}
```

(A3 pins that no gff machinery exists yet in the dispatcher file; `grep -c`
exits 1 on zero matches, which IS the expected count.)

## Problem

The Contact Layer's general validated FF stage needs its engine: given two
carriers' implicit fields (BG-SOL-S6-IMPLICIT landed `ImplicitField` for the
five bare canonical carriers), decide for a 3-D search box whether the shared
zero set `{ f1 = 0, f2 = 0 }` passes through it — and where — using ONLY
certified steps: interval exclusion and the Krawczyk existence/uniqueness
operator (`num/krawczyk.rs`). This packet builds that engine as
**branch-cover enumeration**: a decomposition of the search box into proven
curve points, proven-singular boxes, proven-empty regions, and honestly-typed
unresolved remainder.

This writes NO dispatcher logic and NO new `ContactLocus` arms — wiring the
cover into `contact()` is the NEXT packet's job. Like S6-IMPLICIT, this is an
independently testable substrate layer.

## Decisions already made for you

**New file `vendor/truck/truck-evidence/src/contact/gff.rs`, declared in
`contact/mod.rs` beside `pub mod implicit;` as `pub mod gff;` plus whatever
imports that declaration requires. That is the ONLY edit to mod.rs.**

### The mathematics, fixed by the plan

The contact curve is `C = { p : f1(p) = 0, f2(p) = 0 }`. At any regular point
the curve's tangent direction is `t = ∇f1 × ∇f2`. The certified probe is a
**3×3 augmented Krawczyk system**: pick a direction `g` and a reference point
`m`, and solve

```text
F(p) = [ f1(p), f2(p), g · (p − m) ]   over a box Q
```

A `KrawczykProof::Unique` proves EXACTLY ONE point of C in Q that also lies in
the plane `g·(p−m) = 0` — one certified crossing. Direction choice per call:
`g = ∇f1(m_b) × ∇f2(m_b)` at the search box midpoint `m_b`; when that cross
product is degenerate (∇f1 ∥ ∇f2 across the whole box — the tangency/singularity
case) the box classifies as singular instead of probing. Interval arithmetic
soundness comes from `ImplicitField`; existence/uniqueness from krawczyk; the
composition decides nothing it cannot prove.

### 1. Public API, verbatim:

```rust
/// What the cover proved about one leaf of the decomposition.
#[derive(Clone, Debug, PartialEq)]
pub enum CellVerdict {
    /// The box contains no point of C: some f_i enclosure excludes zero.
    Empty,
    /// The box holds (part of) a singular locus: the gradient cross product
    /// enclosure contains zero at the box midpoint AND neither field excludes
    /// zero on the box. Not further classified here.
    Singular,
    /// Krawczyk proved exactly one crossing of C through the box's mid-plane.
    Point(Point3),
}

/// The certified branch cover of a search box.
#[derive(Clone, Debug, Default)]
pub struct BranchCover {
    /// Certified crossings, in discovery order (deterministic worklist).
    pub points: Vec<Point3>,
    /// Boxes holding provable-or-suspected singular loci.
    pub singular_boxes: Vec<Box3>,
    /// Leaves neither pruned nor certified before budget/resolution ran out.
    pub unresolved_boxes: Vec<Box3>,
}

/// Decompose `domain` into CellVerdict leaves for the shared zero set of two
/// implicit fields. Deterministic: widest-axis bisection, ties toward the
/// lowest axis index. `tau` is the resolution floor — a leaf narrower than
/// `tau` on its widest axis that still cannot be classified goes to
/// `unresolved_boxes` rather than bisecting further. Subdivision spend goes
/// through `budget`.
pub fn cover_branch(
    f1: &impl ImplicitField,
    f2: &impl ImplicitField,
    domain: &Box3,
    tau: f64,
    budget: &mut Budget,
) -> Outcome<BranchCover>;
```

### 2. The algorithm, exactly:

Worklist of boxes, initialised with `domain`. Pop a box B:

a. **Interval exclusion**: if `f1.implicit(B)` or `f2.implicit(B)` does not
   contain zero → `Empty`, emit nothing.
b. **Singularity screen**: let `c = midpoint(B)` (float point); compute
   `cross = ∇f1(c) × ∇f2(c)` as INTERVAL enclosures from the gradient boxes
   over B (evaluate `grad` ON THE BOX, take the componentwise products'
   intervals). If every component of `cross` contains zero → `Singular`,
   record B.
c. **Probe**: build the augmented system above with `m = c`,
   `g = cross.midpoint()` (the float midpoints of the three component
   intervals, renormalized if nonzero — if the midpoint degenerates to zero
   treat as singular). Implement `KrawczykSystem<3>` for a private struct
   holding the two fields, `g`, `m`: `f_point` evaluates both implicits AT
   the point (degenerate-interval wrap) plus `g·(p−m)`; `jacobian` evaluates
   both grads over the box (row per field, last row = g as constants);
   `preconditioner` returns a float inverse of the 3×3 Jacobian midpoint —
   write a small explicit 3×3 Gaussian-elimination invert returning
   `Option`; None lets krawczyk bisect (its contract).
d. `krawczyk(...)` outcome: `Unique` → record the certified point (use
   the box midpoint projected onto the plane — the POINT recorded is the
   box midpoint `c`; it lies within tau of the true crossing, and the PROOF
   is the certificate, so store `c`). `NoRoot` → `Empty`. Refusal
   (`NumericallyUnresolved`) → bisect B widest-axis-first (ties lowest
   index) and push children, spending budget; if B cannot bisect (width ≤
   tau on all axes, or f64 resolution) → `unresolved_boxes`.

`Outcome<BranchCover>` errors only on budget exhaustion per the house
`Refusal::BudgetExhausted`-style conventions already used by krawczyk — read
how krawczyk reports spend and mirror it; never panic.

### 3. Scope guards

- No `ContactLocus` changes, no `contact()` dispatch changes, no new locus
  arms. The next packet wires this in.
- No connectivity/ordering claims between the proven points — the cover is a
  SET, enumerated deterministically.
- Do not touch implicit.rs. If you find a defect IN IT, report in
  `disagreements` and work around it locally.

## Tests (witnesses machine-checked at packet-writing time)

All three configurations use the UNIT z-cylinder at the origin (r=1,
center origin):

- `transversal_pair_yields_proven_points_on_curve` — sphere center (3,0,0),
  radius 3. Derivation (verified numerically): subtracting the cylinder
  equation gives `z² = 6x − 1`, a smooth space curve wherever `x > 1/6`;
  e.g. the point (1/2, −√3/2, √2) satisfies both equations to f64 rounding
  (checked: f_cyl ≈ −1.1e-16, f_sph = 0). Search box around the overlap
  region (say x∈[0,1], y∈[−1,1], z∈[−3,3]), tau generous (1e-2). Assert:
  `points` is non-empty, EVERY returned point satisfies |f1| ≤ 1e-9 and
  |f2| ≤ 1e-9 evaluated with the scalar forms, and `unresolved_boxes` does
  not grow without bound under a healthy budget (just assert it stayed
  finite and points exist — exact counts are NOT pinned, they depend on
  bisection order).
- `tangent_pair_classifies_singular` — sphere center (2,0,0), radius 1.
  Tangent to the cylinder at exactly (1,0,0) (both equations vanish there;
  gradients (2,0,0) and (−2,0,0) are antiparallel). Assert: some box in
  `singular_boxes` contains (1,0,0).
- `disjoint_pair_proves_empty` — sphere center (10,0,0), radius 1: every
  cylinder-wall point is ≥ 8 from the sphere center. Assert: `points` empty,
  `singular_boxes` empty, `unresolved_boxes` empty.
- `empty_boxes_prune_by_interval_exclusion` — same transversal pair but a
  domain box entirely off the cylinder wall (e.g. x∈[3,4], y∈[3,4],
  z∈[0,1]): assert everything prunes (`points`, `singular_boxes`,
  `unresolved_boxes` all empty) — this path must exit on rule (a) alone.

Float literals: H-3 forbids added bare `1e-N` literals without a same-line
`// H-3` opt-out; use rational decimals or mark lines.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-evidence
cargo clippy -p truck-evidence --all-targets --no-deps
cargo check --workspace --all-targets
cargo test -p truck-evidence --lib contact::gff --no-fail-fast
cargo test -p truck-evidence --lib --no-fail-fast
bash scripts/kernel-gates.sh <your base commit>
```

Never run a bare `cargo test`.

## Forbidden

Editing implicit.rs, the dispatcher logic, `analytic_ff`, `fe_ee.rs`, or any
file outside the write set. Adding `ContactLocus` arms or dispatch wiring.
Adding dependencies. Panicking on budget exhaustion (return the typed
Refusal).

## Stop conditions

- an anchor count differs → `ANCHOR_MISMATCH`, naming the file and what you saw
- the 3×3 preconditioner contract cannot be met as specified → `SPEC_GAP`
- the transversal test cannot certify ANY point under a generous budget and
  you have re-checked the formulation → `SPEC_GAP` naming what you observed
- three consecutive failed `cargo` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit on the current branch with subject
`feat(evidence): certified branch cover for implicit field pairs (BG-SOL-S7-GFF-COVER)`.
