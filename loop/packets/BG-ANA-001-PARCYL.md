# WORK PACKET BG-ANA-001-PARCYL — exactly solvable pair: parallel-axis cylinders

You are implementing one item from a formal kernel specification. Everything you
need is in this document. **Do not read `docs/GENERATION_KERNEL_BUILD_SPEC.md`
or any other spec file** — they are not on your allowlist and this packet is
self-contained. If something you need is genuinely missing, that is a SPEC_GAP
(see "Stop conditions"): you stop and report, you do not research it.

```json
{"id":"BG-ANA-001-PARCYL","status":"DONE","contracts":["BG-ANA-001","BG-ANA-002"],
 "tests_added":6,"sites_migrated":0,"sites_deferred":0,
 "deviations":[],"disagreements":[],"baseline_failures":[],
 "notes":"free text"}
```

Fill `deviations`, `disagreements` and `baseline_failures` as arrays, empty if
empty. `disagreements` is the highest-value field: if a decision below is wrong,
say so rather than working around it.

```yaml
id:          BG-ANA-001-PARCYL
contract:    [BG-ANA-001, BG-ANA-002]
class:       mechanical
crates:      [truck-evidence]
depends_on:  [BG-CE-006-ENUM]
write_allow:
  - vendor/truck/truck-evidence/src/analytic/parallel_cylinders.rs
read_allow:
  - vendor/truck/truck-evidence/src/lib.rs
  - vendor/truck/truck-evidence/src/analytic/mod.rs
  - vendor/truck/truck-evidence/src/harness.rs
  - vendor/truck/truck-base/src/evidence.rs
  - vendor/truck/truck-geometry/src/specifieds/mod.rs
  - vendor/truck/truck-geometry/src/specifieds/cylinder.rs
tests_required:
  - parcyl_two_lines_transverse
  - parcyl_margin_sweep_switches_cleanly
  - parcyl_internal_tangency_and_containment
  - parcyl_coincident_and_concentric
  - parcyl_undecidable_predicates_refuse
  - parcyl_certificate_is_exact
budget:      {turns: 34, ctx_tokens: 85000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub enum AnalyticIntersection' vendor/truck/truck-evidence/src/analytic/mod.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub enum ExactCurve' vendor/truck/truck-evidence/src/analytic/mod.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'pub mod parallel_cylinders' vendor/truck/truck-evidence/src/analytic/mod.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'pub fn new(center: Point3, radius: f64) -> Outcome<Self>' vendor/truck/truck-geometry/src/specifieds/cylinder.rs"}
  - {id: A5, expect: 1, cmd: "grep -c 'pub const fn center' vendor/truck/truck-geometry/src/specifieds/cylinder.rs"}
  - {id: A6, expect: 1, cmd: "grep -c 'pub const fn radius' vendor/truck/truck-geometry/src/specifieds/cylinder.rs"}
  - {id: A7, expect: 2, cmd: "grep -c 'TangentLine' vendor/truck/truck-evidence/src/analytic/mod.rs"}
```

## Problem

The canonical `Cylinder` of the specifieds runs along the **z axis** through its
`center` — so **any** two of them are a parallel-axis pair by construction, and
the entire classification reduces to comparing the axis-to-axis distance `d`
against `r0 + r1` and `|r0 − r1|`. This shard also carries the family's most
important test, the **margin sweep** of BG-ANA-002: two cylinders walked through
tangency must switch `transverse → tangent → disjoint` **cleanly**, with no band
of wrong-but-confident answers near the crossing. That property is why the
predicates are intervals and not f64 comparisons.

## Decisions already made for you

1. **One existing file**, `vendor/truck/truck-evidence/src/analytic/parallel_cylinders.rs`.
   It is already created and already declared as `pub mod parallel_cylinders;`
   in `analytic/mod.rs`, itself declared in `lib.rs`. **Both `lib.rs` and
   `analytic/mod.rs` are read-only for you** — editing either is a scope
   violation that will get this packet rejected. The declarations and the
   shared result type were landed up front by the orchestrator so the eight
   sibling packets have disjoint write sets and can run in parallel; your file
   currently holds only a scaffolding doc comment, which you replace. The
   crate-level `#![deny(...)]` covers your module; do not add a second header.

2. **The shared result type is `crate::analytic::AnalyticIntersection` with
   `crate::analytic::{AnalyticOutcome, ExactCurve}` — read `analytic/mod.rs`
   first.** You do NOT define any result type of your own. Your public
   function is:

   ```rust
   pub fn parallel_cylinders(cylinder0: &Cylinder, cylinder1: &Cylinder) -> AnalyticOutcome
   ```

3. **The exact predicates: interval computation, three-way comparison,
   refusal.** Compute predicate quantities as `inari::Interval` (inari rounds
   outward), with named private helpers written exactly this way:

   - `decisively_zero(i) == (i.inf() == 0.0 && i.sup() == 0.0)`
   - `excludes_zero(i) == (i.inf() > 0.0 || i.sup() < 0.0)`
   - `three_way(a, b) -> Option<std::cmp::Ordering>`:
     `Some(Less)` iff `a.sup() < b.inf()`; `Some(Greater)` iff `b.sup() <
     a.inf()`; `Some(Equal)` iff both intervals are degenerate and identical;
     `None` otherwise.

   Dyadic-clean inputs produce degenerate intervals, so exact classifications
   stay exact; an enclosure that merely contains zero proves nothing.

   **Undecidable is a stop, not a guess:** return
   `Err(Refusal::NumericallyUnresolved { spent: Budget::new(0, 0, 0), witness:
   UnresolvedWitness::RootNotIsolated })`. Return `Ok` only when every
   predicate that chose the returned arm was decisive.

4. **Every `Ok` carries the exact certificate, field-by-field at every return
   site** — deliberately no helper (BG-EVD-002):

   ```rust
   let mut props = PropMap::new();
   props.set(Prop::AnalyticCarrier, Truth::True);
   Certified::new(
       value,
       Certificate {
           props,
           method: Method::Exact,
           budget_left: Budget::new(0, 0, 0),
           margin: Margin::UNBOUNDED,
           modulus: Modulus::Unbounded,
       },
   )
   ```

   Doc-comment what `Method::Exact` means here: the classification is exact
   (decisive interval predicates on the f64 carrier parameters), and the
   emitted curves are the closed-form intersections. Coordinates are computed
   in f64; the spec's obligation is "lies on both carriers to machine
   precision", asserted with an H-3-commented slack. No `τ_rep` anywhere.

5. **The classification algorithm, pre-decided.** With `c0, r0, c1, r1` the
   carriers' data, compute **in inari**: `d2 = (x1−x0)² + (y1−y0)²` (interval
   sum of interval squares — the z offsets do not enter; the axes are
   parallel), `zout = d2 − (r0 + r1)²`, `zin = d2 − (r0 − r1)²`. Then:

   1. `three_way(d2, [0,0]) == Some(Equal)` (same axis): `r0 == r1` (exact f64
      equality of parameters) → `Coincident`; else → `Empty` (concentric,
      nested, no contact).
   2. `three_way(zout, 0)`: `Some(Greater)` → `Parallel` (axes parallel, too
      far apart — the parallelism IS the classification). `Some(Equal)` →
      **external tangency** → `TangentLine`. `None` so far → keep going.
   3. `three_way(zin, 0)`: `Some(Equal)` → **internal tangency** →
      `TangentLine`. `Some(Less)` → `Empty` (one inside the other).
      `Some(Greater)` (and zout was `Some(Less)`) → **transverse** → two
      lines, decision 6. `None` → refuse.
   4. Order matters: run zout first; if it is `None`, refuse without
      consulting zin (an undecidable outer predicate must not be resolved by
      a decisive inner one on the other side of the tangency).

6. **The transverse lines, pre-decided.** In f64: `d = √((x1−x0)²+(y1−y0)²)`
   horizontal distance; `d̂ = ((x1−x0)/d, (y1−y0)/d, 0)`; `ℓ = (d² + r0² −
   r1²) / (2d)` (distance from c0 to the chord along d̂); `s = √(r0² − ℓ²)`
   (half-chord); `m = c0 + ℓ·d̂` (chord midpoint, z = 0);
   `w = (−d̂y, d̂x, 0)` (ẑ × d̂). The two lines are `m ± s·w` extruded along
   ẑ: emit `TwoCurves([ExactCurve::Line(Line(m − s·w, m − s·w + ẑ)),
   ExactCurve::Line(Line(m + s·w, m + s·w + ẑ))])`. For tangency (`TangentLine`),
   the line is `Line(m, m + ẑ)` with `m = c0 + ℓ·d̂` and ℓ = r0 (external) or
   the appropriate sign for internal — derive the internal-tangency point from
   the same formula and **verify it numerically in the internal-tangency test
   before committing**; record any correction in `deviations`.

## Tests required

All in the `#[cfg(test)]` module of `parallel_cylinders.rs`: named consts, and
a same-line `// H-3:` comment wherever a bare float slack literal appears.
Construct `Cylinder` through `Cylinder::new(center, radius)` (an `Outcome` —
no unwrap, H-1).

1. `parcyl_two_lines_transverse` — r0 = r1 = 1, centres (0,0,0) and (1,0,0):
   d = 1 < 2 → `TwoCurves` of two lines at x = 1/2, y = ±√3/2. Sample both
   lines (≥ 20 points each); every point satisfies both cylinders' radial
   equations to machine precision (H-3-commented slack).
2. `parcyl_margin_sweep_switches_cleanly` — **the test this packet exists
   for.** r0 = r1 = 1, centre0 (0,0,0), centre1 (d, 0, 0), walking
   `d ∈ { 2 − 1/16, 2 − 1/256, 2 − 2⁻²⁰, 2, 2 + 2⁻²⁰, 2 + 1/256, 2 + 1/16 }`
   (all dyadic — write them as arithmetic on named consts, not decimal
   literals). Expected: `TwoCurves, TwoCurves, TwoCurves, TangentLine,
   Parallel, Parallel, Parallel`, **and no `Err` anywhere in the walk**. The
   dyadic values keep the interval predicates decisive at every step; a
   refusal appearing in the walk is a failure of the design, not a flake.
3. `parcyl_internal_tangency_and_containment` — r0 = 2, r1 = 1, d = 1 →
   internal `TangentLine` (verify the emitted line lies on both cylinders);
   d = 1/2 → `Empty` (contained).
4. `parcyl_coincident_and_concentric` — same centre and radius → `Coincident`;
   same centre, different radii → `Empty`.
5. `parcyl_undecidable_predicates_refuse` — unit-test the private comparator
   on hand-built inari intervals (a `[-w, w]` interval is neither
   decisively-zero nor excludes-zero; overlapping non-degenerate intervals
   give `three_way == None`); try one bit-neighbour tangency witness
   (centre1.x = `f64::from_bits(2.0f64.to_bits() ± 1)`) and report in
   `notes` whether a genuine straddle refusal was constructible.
6. `parcyl_certificate_is_exact` — for a transverse, a tangent and a parallel
   outcome: every `Ok` carries `method == Method::Exact` and the
   `AnalyticCarrier` prop set to `Truth::True`.

## H-3, which is what rejected three packets in this family

GATE-2 fails any **added** line carrying a bare `1e-N` literal unless that same
line ends with an `// H-3` comment. It is a text gate on the diff: it does not
know your literal is an angle, and it does not care that the line is in a test.
`BG-ENC-002-LINE` was rejected for one such line and `BG-ENC-002-CIRCLE` for
six, both times on assertion epsilons in tests, both times costing a verify.

So: **every comparison epsilon you write gets a same-line `// H-3:` comment
naming the dimensionless quantity being compared.** The house form:

    assert!((a - b).magnitude() < 1.0e-12, ...); // H-3: float slack between two unit direction vectors, not a length
    assert!((h - expected).abs() < 1.0e-12, ...); // H-3: float slack between two half-angles in radians, not a length
    assert!(cos_angle >= limit - 1.0e-12, ...);   // H-3: float slack between two direction cosines, not a length

Directions, angles, direction cosines, parameter values, residuals of
unit-scale witnesses and interval bounds are all dimensionless and all
legitimate — the comment is what says so. A literal that really is a
model-space *length* does not get an opt-out; it goes through `ToleranceCtx`
instead. Run `bash scripts/kernel-gates.sh` yourself before you write
`RESULT.json`; it is the same script V4 runs.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-evidence
cargo clippy -p truck-evidence --all-targets --no-deps -- -D warnings
cargo test -p truck-evidence --lib --tests --no-fail-fast
cargo check --workspace --all-targets
```

Never run a bare `cargo test` — it builds 56 examples. Send cargo output to a
file and read the tail. The existing 74 lib tests + 3 integration tests must
keep passing unchanged.

## Forbidden

Editing any file outside `write_allow` — `lib.rs` and `analytic/mod.rs`
especially. Defining a private result enum. Changing the shared types, the
harness, or any carrier. Deciding a predicate by sampling the surfaces.
Returning an `Ok` arm chosen by an undecidable predicate. Adding `#[ignore]`.
Adding `unscaled_legacy(` call sites. Committing to `main`.

## Stop conditions

- an anchor count differs → `ANCHOR_MISMATCH`, naming the file and what you saw
- `Cylinder` accessors do not supply what decision 5 needs → `SPEC_GAP`,
  naming exactly what is missing
- `inari` lacks a primitive this design needs → `SPEC_GAP`, naming it
- the margin sweep cannot be made refusal-free on the dyadic walk → `SPEC_GAP`,
  with the exact d value that refuses and the interval widths involved
- three consecutive failed `cargo test` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit on the current branch with subject
`feat(evidence): exact parallel-axis cylinders (BG-ANA-001-PARCYL)`.
