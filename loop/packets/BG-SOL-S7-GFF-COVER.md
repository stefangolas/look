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

### The mathematics, fixed by the plan (AMENDMENT r2)

The contact curve is `C = { p : f1(p) = 0, f2(p) = 0 }`. The certified probe is
a **2×2 z-slab Krawczyk system** (NOT the packet's r1 3×3 augmented system —
see the r2 amendment note below): decompose the search box's z-range into
leaves; for each z-leaf, at its mid-plane `z0`, solve

```text
F(x, y) = [ f1(x, y, z0), f2(x, y, z0) ]   over the (x, y) box
```

A `KrawczykProof::Unique` proves EXACTLY ONE crossing of C through the slab's
mid-plane. The Jacobian is the 2×2 `∂(f1,f2)/∂(x,y)`; for the z-aligned
quadric pairs this stage exists for, its determinant is
`4(y·cx − x·cy)`-type — non-singular exactly away from the singular locus, so
the 2×2 is well-conditioned wherever the curve is regular. Slabs where the
Jacobian determinant enclosure contains zero classify as `Singular`.

**Why r1's 3×3 augmented system was abandoned (SPEC_GAP evidence, first
attempt 836b704):** the r1 probe `F(p) = [f1, f2, g·(p−m)]` with a full 3×3
inverse preconditioner could not reach `KrawczykProof::Unique` on the
transversal sphere/cylinder witness — the coupled rows kept `I − YJ` wide at
every box scale (the worker's numerical report: the residual rows' interval
sums ≥ 1 at certified-crossing scale, `NumericallyUnresolved` after 4096
subdivisions). Diagnosis after reading `k_image` (num/krawczyk.rs:162): the
operator IS the correct full-matrix Krawczyk (`d[r][c] = δ − y[r][c]·j[r][c]`,
row-major, Y system-supplied); it is NOT limited to diagonal systems. The
defect was in the FORMULATION's conditioning, not the operator. The 2×2 slab
system shrinks the coupled part and gives an exactly-invertible, well-scaled
preconditioner (`det = 4(y·cx − x·cy)` for cylinder×sphere at (cx,0)), which is
what Krawczyk needs to contract. This amendment replaces only the probe; the
API, the algorithm shape (exclusion → screen → probe → bisect), the tests'
intent, and the scope guards are unchanged.

### 1. Public API, verbatim:

```rust
/// What the cover proved about one leaf of the decomposition.
#[derive(Clone, Debug, PartialEq)]
pub enum CellVerdict {
    /// The box contains no point of C: some f_i enclosure excludes zero.
    Empty,
    /// The box holds (part of) a singular locus: the slab Jacobian
    /// determinant enclosure contains zero AND neither field excludes zero
    /// on the box. Not further classified here.
    Singular,
    /// Krawczyk proved exactly one crossing of C through the slab mid-plane.
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

Worklist of **z-leaves**: intervals partitioning `domain.z`, initialised with
the full z-range. Pop a z-leaf Z. Let `B` be the full 3-D box
`domain.x × domain.y × Z`, and `z0 = midpoint(Z)`.

a. **Interval exclusion**: if `f1.implicit(B)` or `f2.implicit(B)` does not
   contain zero → `Empty`, emit nothing.
b. **Singularity screen**: let `c = (midpoint(x), midpoint(y), z0)`; evaluate
   the 2×2 slab Jacobian `J = ∂(f1,f2)/∂(x,y)` as INTERVAL enclosures over
   `B` (take the `grad` boxes' x/y components). If the determinant interval
   `det(J)` contains zero → `Singular`, record B.
c. **Probe**: build the 2×2 slab system with `z = z0` and `m = (mx, my)`:
   implement `KrawczykSystem<2>` for a private struct holding the two fields
   and `z0`: `f_point` evaluates both implicits AT the point `(x, y, z0)`
   (degenerate-interval wrap); `jacobian` evaluates both grads over the box
   `domain.x × domain.y × [z0, z0]` and returns the `(x, y)` 2×2 sub-matrix
   (rows f1/f2, cols ∂/∂x ∂/∂y); `preconditioner` returns the EXACT float
   inverse of `mid(J)` (2×2 closed form `1/det · [[d, −b], [−c, a]]`; `None`
   when `|det|` is degenerate — krawczyk then bisects per its contract).
d. `krawczyk(...)` outcome: `Unique` → record the certified point `(x, y,
   z0)` (the recorded point may be refined by a few float Newton steps of the
   2×2 system toward the root — the Krawczyk uniqueness proof justifies the
   contraction; the point must satisfy both implicits to float accuracy).
   `NoRoot` → `Empty`. Refusal (`NumericallyUnresolved`) → bisect Z
   widest-axis... Z is a scalar interval, so bisect Z at its midpoint,
   spending budget; also bisect the (x, y) box inside the probe when krawczyk
   reports unresolved for a slab that survives the exclusion screen (the
   probe worklist is nested: per Z-leaf, an (x, y)-worklist). If neither can
   bisect (width ≤ tau, or f64 resolution) → `unresolved_boxes`.

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
- the 2×2 slab probe cannot certify ANY crossing of the transversal pair
  under a generous budget AND you have re-checked the formulation (including
  re-deriving the determinant) → `SPEC_GAP` naming what you observed
- three consecutive failed `cargo` runs on the same error → `BLOCKED`

## Amendment r2 note

This is the second attempt (first: SPEC_GAP at 836b704, recorded above). The
r1 implementation in `gff.rs` on this branch is the 3×3 formulation; convert
it to the 2×2 z-slab formulation — keep the public API, the exclusion and
singular screens, the bisection structure, the `invert` helper if reusable
for 2×2, and the three passing tests; REPLACE the probe and the failing
transversal test's expectations. Update RESULT.json `notes` to record the
conversion and confirm the transversal witness now certifies.

## Finish by writing `RESULT.json` in the root of your worktree

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit on the current branch with subject
`feat(evidence): certified branch cover for implicit field pairs (BG-SOL-S7-GFF-COVER)`.
