# B-rep generation kernel — build specification

> Implementation orders for the corpus-invariant foundation of
> [`FORMAL_SYSTEM_BREP_GENERATION.md`](FORMAL_SYSTEM_BREP_GENERATION.md),
> against the vendored truck tree. Sequencing rationale and cost estimates are in
> [`TRUCK_GENERATION_AUDIT.md`](TRUCK_GENERATION_AUDIT.md) §7.
>
> **Scope: Stages 0–3 only** — the ~24k LOC that the audit establishes as
> ~94% corpus-invariant. Stage 4 (intersection/Boolean) is sketched in §5 to fix
> its interfaces, but is deliberately not specified to implementation depth:
> its tangential-cell density is corpus-contingent and speccing it now would be
> guessing. Stage 5 is not specified at all — every `[POLICY]` item in the
> formal system lives there.
>
> **Contract namespace is `BG-`**, disjoint from the ingestion registry
> (`TOP-`, `GEO-`, `QUO-`, `DOM-`, `ARR-`, `CDT-`, `MSH-`, `SHL-`, `RES-`) in
> [`MATHEMATICAL_FOUNDATION.md`](MATHEMATICAL_FOUNDATION.md). Every commit names
> the `BG-` IDs it discharges.
>
> **Synced to formal system revision 3.** Four items changed and one landed
> artefact needs amending:
> - **BG-FID-008 (new)** — §6.2 gained a one-sheet condition (iv). (i)–(iii) give
>   a covering of *some* degree, not a homeomorphism, so a checker implementing
>   only them passes a double-cover input and voids everything above it. Consumed
>   by BG-FID-005, BG-FID-006 and BG-TEST-007.
> - **BG-EVD-004 (new)** — §18's forward bound is now the nested recurrence, with
>   the split form conditional on certified subadditivity. `Modulus` changes shape;
>   `Refusal` gains `ForwardToleranceExceeded`.
> - **BG-FID-007 (new)** — reach quantities are certified *lower bounds*
>   throughout; `lfs` is renamed `lfs_lower`, tests assert `<=` not `==`, and
>   `ReachTooSmall` becomes `ReachLowerBoundTooSmall`.
> - **Stage 4 interfaces** — §16.3 topology events use generalized
>   (Grove–Shiohama/Clarke) critical values, which constrains the nearest-point
>   API Stage 3 ships.
> - **P-6** — `truck-evidence` has already landed against the r2 `Modulus`; see
>   the amendment note in §8.
>
> **Also synced to revision 4**, which is addition rather than repair and lands
> mostly in Stage 4 — but two pieces reach back into this document's scope:
> - **BG-ENC second derivatives are load-bearing.** §9.2.2 traces the tangency
>   locus as a ridge curve of $g$, gated on the nonzero Hessian eigenvalue, so
>   `enclose_der(2, ·)` is on the critical path rather than being completeness
>   polish. Do not defer it in BG-ENC-002/003.
> - **The blow-up in SI-DEF-001 is reused, not re-derived.** §9.2.1's polar
>   blow-up at an isolated tangency is the same deflation move as the diagonal
>   blow-up for self-intersection. One implementation, two call sites.
>
> Chamfer, corners and the tangential cells themselves are Stage 4/5 bodies and
> stay out of scope here; §5's interface sketch is updated so Stage 3 targets
> them correctly.

---

## 0. House rules

These apply to **every** item below and are not repeated in the item bodies. A
reviewer should reject any diff that violates one.

**H-1 — No panics on data.** No `unwrap`, `expect`, `panic!`, `unimplemented!`,
`todo!`, or indexing that can go out of range, on any path reachable from
untrusted geometry. `assert!` is permitted only for internal invariants that no
input can violate, and must carry a comment saying why. `debug_assert!` is
permitted freely.

**H-2 — Every fallible operation returns `Outcome<T>` (BG-EVD-001), never
`Option` or a bare `Result`.** `None` is not a diagnosis.

**H-3 — No absolute constants in predicates.** Every comparison against a length
goes through `ToleranceCtx` (BG-TOL-001). A literal `1.0e-6` in a predicate is a
defect. Dimensionless comparisons (angles, sines, parameter fractions) may use
literals, and must name the quantity in a comment.

**H-4 — No `cfg!(debug_assertions)`-dependent semantics.** `debug_new` and
friends are banned in new code (audit F-7): validity checks either run always or
are an explicit, named, caller-chosen `_unchecked` variant.

**H-5 — Budget or bound, never a bare loop.** Any iteration whose count depends
on geometry takes a `&mut Budget` (BG-NUM-001) and returns
`NumericallyUnresolved` on exhaustion. A hard-coded `for _ in 0..16` is a defect.

**H-6 — Certificates carry their method.** Every `Certificate` records `μ ∈
{Exact, Interval, Float, None}` (§4). A value computed in floats may never be
recorded as `Exact`.

**H-8 — Anchors are symbols, never line numbers.** Every code reference in this
document names a **file + enclosing symbol + a `rg` pattern**, and where the
count matters, an **expected hit count**. Line numbers rot immediately — often
within the very item that cites them, since BG-S0-001 alone edits six sites in
one file and fixing the first moves the other five.

Two rules follow, and the second is the point of the convention:

1. **Locate by pattern, never by line.** Run the given `rg` command.
2. **A count mismatch is a stop condition, not a nuisance.** If the pattern
   yields a different number of hits than stated, the code has moved on since
   this document was written (truck rev `c5f4b6e`). Stop and re-scope the item
   rather than patching whichever sites happen to match — the wrong-five-of-six
   failure is silent and expensive.

Note that **paths** change exactly once, when the tree is vendored (P-1), and
are safe to fix with a global find-and-replace. **Symbols** are stable across
both events. That is why anchors are symbols.

**H-7 — Tests are three-layer.** Every item ships: unit tests on named
witnesses; `proptest` property tests (truck already depends on `proptest`, follow
`truck-geometry/src/decorators/af_surface.rs` for style); and, where the item has
a margin parameter, a **margin sweep** (BG-TEST-SWEEP below).

**BG-TEST-SWEEP — the epistemic test, mandatory for every gated item.**
Sweep the item's margin parameter (δ, σ, ι, …) logarithmically toward zero and
assert the outcome degrades **monotonically**:

```
Proven → CertifiedEquivalent → NumericallyUnresolved → UnsupportedEnvelope
```

and **never** skips to a wrong-but-confident answer. This is the directly
testable statement of epistemic closure (§21), and it is the one test no
differential oracle can serve. A gated item without a margin sweep is not done.

---

## 1. Stage 0 — free wins

Land first; independent of everything else. ~250 LOC total. BG-S0-001 is
closed; BG-S0-002 and BG-S0-003 remain.

### BG-S0-001 — ~~`IncludeCurve` on `IntersectionCurve` must not abort~~ DONE

**Closed 2026-08-16**, landed in `da72cd5`. `Surface::include` returns
`Outcome<bool>`, the sampling path lives in `include_intersection_curve`, the
`ssi-carrier` and `leader-witness` certificates are emitted as specified, and
`boolean_derived_face_consistency_returns` is the regression test.

**The anchor below now yields 0 hits and is retained only as history** — under
H-8 a count mismatch is a stop condition, so a work packet generated from the
original text correctly halts on its first command rather than patching
something else. See BG-S0-003 for the one site that remains.

**Fixed** audit F-9.
**Anchor** `truck-modeling/src/geometry.rs`, inside `impl IncludeCurve<Curve> for Surface`.
**Located by** `rg -n 'Curve::IntersectionCurve\(_\) => unimplemented!\(\)' truck-modeling/src/geometry.rs` — **was 6 hits**, now 0.

**Problem.** `Surface::include(curve)` is `unimplemented!()` for
`Curve::IntersectionCurve` at six sites. `IntersectionCurve` is the variant
Booleans *produce*, and `builder::try_attach_plane` is bounded on
`Plane: IncludeCurve<C>` — so capping a Boolean result aborts the process.

**Algorithm.** No new mathematics. An `IntersectionCurve` carries its own two
surfaces:

```
include(self: &Surface, c: &Curve) -> Outcome<bool>
  match c:
    IntersectionCurve(ic):
      # exact structural answer, the case that actually arises
      if surface_identity(self, ic.surface0()) or surface_identity(self, ic.surface1()):
          return Proven(true, Certificate{ mu: Exact, rule: "ssi-carrier" })
      # otherwise: sample the leader polyline, test each point against self
      # within tau_rep; a negative is conclusive, a positive is not
      for p in ic.leader().points():
          if not self.contains_point(p, ctx.tau_rep):
              return Proven(false, Certificate{ mu: Float, rule: "leader-witness" })
      return NumericallyUnresolved(budget, Witness::UncertifiedContainment)
    _: (existing paths, unchanged)
```

`surface_identity` compares by carrier identity (BG-CE-004), **not** geometric
equality.

**Contract BG-S0-001.** For all `Surface × Curve`, `include` terminates and
returns an `Outcome`. No input reaches a panic.

**Tests.**
- Unit: a plane and an `IntersectionCurve` whose `surface0` *is* that plane →
  `Proven(true)`, `μ = Exact`.
- Unit: a plane and an ISC lying demonstrably off it → `Proven(false)`.
- Unit: a plane and an ISC of two *other* surfaces that happens to lie in it →
  `NumericallyUnresolved`, **not** `Proven(true)`. This is the test that catches
  someone "helpfully" strengthening the sampling path.
- Regression: `try_attach_plane` over a wire of Boolean-derived edges returns an
  `Outcome` (previously aborted).

### BG-S0-002 — Fillet solve failures are refusals, not aborts

**Fixes** audit F-4a.
**Anchors**
- `truck-shapeops/src/fillet/mod.rs`, fn `simple_fillet` — `rg -n 'search_contact_curve[01]_cross_point_with_adjacent_edge' -A3` then the trailing `.unwrap()`; **expect 4**.
- same file, fn `create_pcurve_edge` — `rg -n 'Matrix3::from_cols.*invert\(\)\.unwrap\(\)'`; **expect 2**.
- `truck-geometry/src/decorators/af_surface.rs`, fn `approx_rolling_ball_fillet` — the `contact_circle(v).unwrap()` inside the refinement closure; **expect 1**.

**Algorithm.** Mechanical: `.unwrap()` → `?` propagating `Outcome`. The six
`search_contact_curve*_cross_point_with_adjacent_edge` sites become
`NumericallyUnresolved`; the two `Matrix3::invert().unwrap()` sites become
`UnsupportedEnvelope(ChartDegenerate)` (a singular
`[uder, vder, n]` frame *is* a chart degeneracy, §9.1).

This does **not** give the right diagnosis — `RadiusExceedsCurvature` needs real
gates (Stage 5, out of scope here). It stops the abort. Note the inconsistency
being repaired in `approx_rolling_ball_fillet`: `contact_circle` is `?`-handled
where the initial three circles are built, then `.unwrap()`ed on the identical
call inside the refinement closure.

**Contract BG-S0-002.** No fillet input causes a panic. Every failure returns an
outcome naming at least the stage that failed.

**Tests.** Radius > face curvature; contact curve running off the trimmed
domain. Both must return, not abort. Assert the process survives —
`catch_unwind` in the test asserting it was *not* needed.

The third failure mode — fillet at a chart pole returning
`UnsupportedEnvelope(ChartDegenerate)` — is **deferred to BG-S0-002-r2**. The
A2 mechanical conversion (`Matrix3::invert().unwrap()` → `?` propagating
`UnsupportedEnvelope(ChartDegenerate)`) is required here and is verified by V3
(it compiles) and V4 (H-1: no `unwrap` in the added lines), but its runtime
reachability through `simple_fillet` is blocked: the contact-curve crossings
that feed `create_pcurve_edge` are computed in
`truck-geometry/src/decorators/rbf_surface/algo.rs`
(`search_contact_curve{0,1}_cross_point_with_adjacent_edge`), which call
`mat.invert().unwrap()` at lines 815/824/834/847/925/934/944/957 and abort on
the same degenerate geometry that would singularize `create_pcurve_edge`'s
`[uder, vder, n]` frame (det = `|uder × vder|²`, zero iff `uder ∥ vder`). A
worker proved this empirically on the first attempt (QUESTION.md, 2026-08-16);
the runtime test must call `create_pcurve_edge` directly with a constructed
degenerate surface, which does not depend on that hardening and is filed as
BG-S0-002-r2.

### BG-S0-002-r2 — Direct unit test of the chart-pole refusal (deferred from BG-S0-002)

**Splits** the runtime-test half of BG-S0-002's third failure mode out of
BG-S0-002. Filed 2026-08-16 after a worker proved the A2 refusal path
unreachable through `simple_fillet` (see BG-S0-002 "Tests"): degenerate
geometry panics first in `rbf_surface/algo.rs`'s `mat.invert().unwrap()` sites,
which are out of BG-S0-002's `write_allow`.

**Depends on** BG-S0-002 (the `?` / `UnsupportedEnvelope(ChartDegenerate)`
conversion must be in place). Does **not** depend on hardening
`rbf_surface/algo.rs`: the test calls `create_pcurve_edge` directly with a
surface whose `[uder, vder, n]` frame is singular at the search point
(`uder ∥ vder`, a parametric pole), bypassing the contact-curve search.

**Class.** design — the degenerate-surface fixture is a construction, not a
mechanical edit. The orchestrator writes this packet.

**Contract BG-S0-002-r2.** `create_pcurve_edge` returns
`Refusal::UnsupportedEnvelope(EnvelopeCase::ChartDegenerate)` for a singular
frame, without panicking.

### BG-S0-003 — the extrude case BG-S0-001 left behind

**Fixes** the residual of audit F-9. Split out of BG-S0-001 on 2026-08-16: that
item's text said a 7th site "lives in `impl ToSameGeometry<Surface> for
ExtrudedCurve<Curve, Vector3>`; that one is the extrude case, handled separately
below" — and then never handled it below. It is still an abort.

**Anchor** `truck-modeling/src/geometry.rs`, inside
`impl ToSameGeometry<Surface> for ExtrudedCurve<Curve, Vector3>`, fn
`to_same_geometry`.
**Locate** `rg -n 'IntersectionCurve\(_\), Curve::IntersectionCurve\(_\)\) => unimplemented!' truck-modeling/src/geometry.rs` — **expect 1 hit**.
**If the count differs, stop.**

**Problem.** Extruding an `IntersectionCurve` — the variant Booleans produce —
along a vector aborts the process. Extruding a Boolean result's edge is an
ordinary modelling operation, so this is reachable from untrusted geometry and
violates H-1.

**Algorithm.** The pair arm is reached when *both* rail curves are intersection
curves. There is no exact surface for the general case, so this is a refusal,
not a construction:

```
to_same_geometry(self) -> Outcome<Surface>
  (IntersectionCurve(_), IntersectionCurve(_)) ->
      UnsupportedEnvelope(ExtrudedIntersectionCurve)
```

Note the signature change: `to_same_geometry` currently returns `Surface`, so
this item carries the `Outcome` conversion for that trait (H-2) and its call
sites — the same mechanical shape BG-S0-001 applied to `include`. Use that
landed diff as the template; it is the reference answer for this exact move.

**Contract BG-S0-003.** No `ExtrudedCurve` input reaches a panic.

**Tests.**
- Unit: extruding an ISC-railed curve returns `UnsupportedEnvelope`, not a
  panic. `catch_unwind` asserting it was **not** needed.
- Unit: every non-ISC pair still produces the surface it produced before,
  bit-identically — this item must be semantically inert everywhere else.

---

## 2. Stage 1 — data model

Everything downstream depends on these types. Build in the order given: BG-EVD
and BG-TOL first, because every later signature mentions them.

### BG-EVD-001 — Outcome and evidence algebra

**Implements** §4. **New crate/module** `truck-evidence` (or
`truck-base::evidence`).

**Shape decision (P-2, closed 2026-08-15).** `Outcome<T>` is a `Result` so `?`
works natively — a plain enum cannot (the `Try` trait is unstable), and an agent
following H-2 literally would otherwise write nested `match` pyramids or quietly
revert to `Option`. The `Proven` vs `CertifiedEquivalent` distinction is a field
of `Certificate` (BG-EVD-002), not a variant. Totality and mutual exclusivity
(§4) are preserved: the success side is exactly one certified value, the refusal
side is exactly one terminal diagnosis.

```rust
pub type Outcome<T> = Result<Certified<T>, Refusal>;

pub struct Certified<T> { pub value: T, pub cert: Certificate }

/// Every non-success terminal outcome of §4. `?` propagates these directly.
pub enum Refusal {
    Empty,
    UnsupportedEnvelope(EnvelopeCase),
    NumericallyUnresolved { spent: Budget, witness: UnresolvedWitness },
    CompositionMarginExhausted(MarginWitness),
    /// §18: the nested error recurrence exceeded the declared forward tolerance
    /// while every topological margin held. Referenced by §18 but missing from
    /// the §4 list through formal-system r2 — a totality gap under OB-1, since a
    /// chain can fail the metric condition and pass every topological one.
    ForwardToleranceExceeded { bound: f64, declared: f64, step: u32 },
    InputOutsideBackwardBudget(RepairWitness),
    Contradictory(ContradictionWitness),
    Collapsed(Collapse, Certificate),   // certified, but not a realisation
}

/// §4 evidence tuple (π, μ, β, 𝔪, ω).
pub struct Certificate {
    pub props: PropMap,        // π: Prop -> Truth
    pub method: Method,        // μ: Exact | Interval | Float | None
    pub budget_left: Budget,   // β
    pub margin: Margin,        // 𝔪: topological stability margin (§18)
    pub modulus: Modulus,      // ω: modulus of continuity (§18)
}

pub enum Truth { Unknown, True, False, Both }  // ⊥ ≤k {T,F} ≤k ⊤

/// ω: modulus of continuity (§18). Valid only on `[0, domain)` — outside the
/// topological stability cell the operation is not continuous and no modulus
/// exists (M3).
pub struct Modulus {
    pub shape: ModulusShape,
    /// (M3) the bound holds only for ε < domain. Usually the cell's `Margin`.
    pub domain: f64,
    /// (M4) subadditivity — **certified, never assumed**. Gates the split bound.
    pub subadditive: bool,
}

pub enum ModulusShape {
    Lipschitz(f64),                   // ω(ε) = kε                — subadditive
    Holder { k: f64, exponent: f64 }, // ω(ε) = k·ε^p, tangency p = 1/2
                                      //   — subadditive iff p ≤ 1 (concave)
    /// ω(ε) = kε/(m − ε): the natural shape near a degenerate gate. Convex,
    /// so **not** subadditive. Exists so that such a cell can publish an honest
    /// modulus instead of falling back to `Unbounded`.
    Pole { k: f64, m: f64 },
    Unbounded,                        // may not participate in composition (OB-6)
}

impl Modulus {
    pub fn eval(&self, eps: f64) -> Option<f64>;   // None when eps >= domain (M3)
    /// (M4), decided from `shape`, not declared by the caller.
    pub fn is_subadditive(&self) -> bool;
    /// Least concave majorant on `[0, m']`, `m' < domain`. Always subadditive
    /// and vanishing at 0, so the split bound is available at the price of
    /// declared pessimism. Returns `None` if ω is unbounded on `[0, m']`.
    pub fn concave_majorant(&self, m_prime: f64) -> Option<Modulus>;
}
```

**Algorithm — accumulation.** Combining two certificates:
- `props`: join in the knowledge order. `True ⊔ False = Both`.
- **Any `Both` anywhere ⇒ the whole result is `Contradictory`.** Not a warning.
- `method`: the **weakest** of the two (`Exact ⊓ Float = Float`). Method never
  improves by combination — H-6.
- `budget_left`: sum of remaining.
- `margin`: **minimum**.
- `modulus`: **not** a bare composition. The propagated quantity is the error
  itself, stepped through the nested recurrence of §18:

```rust
/// ε_{i+1} = ω_i(ε_i) + τ_rep,i, evaluated stepwise so the step that leaves the
/// stability cell is identifiable. This is the fundamental bound and it needs
/// only (M1)–(M3).
pub fn propagate(eps: f64, m: &Modulus, tau_rep: f64, margin: Margin, step: u32)
    -> Result<f64, Refusal>;
```

  `ω₂ ∘ ω₁` is still available as `Modulus::compose`, but composing moduli and
  *then* adding the τ's is the **split bound**, which is a corollary requiring
  (M4) at every step. `compose` therefore refuses when either operand reports
  `is_subadditive() == false`; the caller either uses `propagate` or converts via
  `concave_majorant`. `Lipschitz(a) ∘ Lipschitz(b) = Lipschitz(ab)`; anything
  composed with `Unbounded` is `Unbounded`.

**Contracts.**
- **BG-EVD-001** Totality: every kernel entry point returns exactly one variant.
- **BG-EVD-002** `Proven` requires every gate used to be `[DERIVED]`. A
  `[PROVISIONAL]` gate anywhere caps the result at `CertifiedEquivalent`. Enforce
  in the type system if possible: make `Certificate::proven()` unconstructible
  without a `DerivedGateToken`.
- **BG-EVD-003** Monotonicity: accumulation never raises `method`, never raises
  `margin`, never invents budget.
- **BG-EVD-004 (Modulus contract, §18).** Every published `Modulus` satisfies
  (M1) ω(0)=0 and continuity at 0, (M2) nondecreasing, (M3) validity confined to
  `[0, domain)`. **(M4) subadditivity is decided from the shape, never declared
  by the caller**, and only a (M4)-satisfying chain may use the split bound. The
  default propagation path is `propagate` (the nested recurrence), which is
  unconditionally valid; `compose` is the opt-in fast path and refuses on a
  non-subadditive operand. A cell that computes a forward bound by composing
  moduli and summing τ's without checking (M4) is a defect even when the arithmetic
  is right — the current table happens to be all-subadditive, which is a property
  of today's cells and not of the theorem.

**Tests.**
- Property: accumulation is associative and commutative in `props` and `margin`.
- Property: `method` is monotone non-increasing under accumulation.
- Unit: `True ⊔ False` ⇒ `Contradictory` propagates to the top.
- Unit: attempting `Proven` with a provisional token fails to compile (trybuild).
- Property: `Modulus` composition matches numeric evaluation —
  `(ω₂∘ω₁)(ε) == ω₂(ω₁(ε))` within float tolerance, over random ε.
- **Property, BG-EVD-004 and the one that catches the r2 bug:** for random chains
  of subadditive moduli, `propagate` ≤ the split bound (the recurrence is never
  looser). For a chain containing a `Pole` modulus, assert the split bound can be
  **smaller** than `propagate` on some input — i.e. that using it there would
  under-report the error — and assert `compose` refuses that chain rather than
  producing the smaller number.
- Unit: `Holder { p: 0.5 }` reports `is_subadditive() == true`; `Pole` reports
  `false`; `concave_majorant` of a `Pole` on `[0, m/2]` reports `true` and
  dominates it pointwise.
- Unit: `eval` past `domain` returns `None`, and `propagate` turns that into
  `CompositionMarginExhausted` naming the step index — not a saturated value.

### BG-TOL-001 — Scale-relative tolerance context

**Implements** §0.1 (three budgets), §2 (scale invariance). **Fixes** audit S-2.

```rust
pub struct ToleranceCtx {
    model_scale: f64,   // declared characteristic length; all lengths relative to this
    pub tau_in:  f64,   // backward: perturbation admitted by validation/repair
    pub tau_rep: f64,   // representation error
    pub tau_col: f64,   // collapse quotient
}

impl ToleranceCtx {
    pub fn near_pt(&self, a: Point3, b: Point3) -> bool;      // ||a-b|| <= tau_rep * model_scale
    pub fn is_small_len(&self, l: f64) -> bool;
    pub fn sin_margin(&self) -> f64;   // dimensionless δ floor — NOT scaled

    // Migration scaffold, BG-TOL-001-TYPE-r2. Infallible; model_scale = 1.0 and
    // tau_rep = TOLERANCE, so a Stage-A migrated predicate is numerically
    // identical to the legacy one it replaced. Ratcheted by kernel-gates.sh.
    pub fn unscaled_legacy() -> Self;
}
```

**Algorithm.** Migrate 184 call sites (`TOLERANCE`, `so_small()`, `.near()`,
`.near2()`), 128 of them in `truck-geometry`. **Each site needs a judgement, and
the two cases must not be conflated:**

- **model-space lengths** (distances between points, chord heights, gaps) →
  scale by `model_scale`;
- **parameter-space and dimensionless quantities** (knot values, normalized
  parameters, sines, cosines, weights) → **do not scale**; they are already
  dimensionless and scaling them is a new bug.

Mark every migrated site with `// BG-TOL-001: {model|param}`. A site that is
genuinely ambiguous gets a `FIXME(BG-TOL-001)` and is listed in the PR, not
guessed.

**Migration staging — where a call site gets its context.** Added 2026-08-16,
after `BG-TOL-001-TYPE` landed and the first migration shard could not be
written. The paragraph above says to migrate 184 sites and §9 says "every
signature below takes ctx"; neither says how a site *obtains* a context, and
the answer is not free. None of the 184 sites sits in a function that has one.
Threading `ctx` from the public entry points inward is the end state, but it
changes public signatures in every crate at once, so it cannot be sharded per
crate — which is what the eight `BG-TOL-001-*` shards assume, and what makes
them write-disjoint. Doing it as one packet is a twelve-crate breaking change
with no intermediate state that compiles.

The migration is therefore **two stages, and the contracts below are only
discharged by the second**:

- **Stage A — classify, per crate, behaviour-preserving.** Every site is
  rewritten through a `ToleranceCtx` and marked `model` or `param`. The context
  comes from `ToleranceCtx::unscaled_legacy()`, constructed at the top of each
  function that contains sites. That constructor is infallible and returns
  `model_scale = 1.0`, `tau_rep = TOLERANCE`, so **every migrated predicate
  keeps exactly its present numeric behaviour**. Nothing is fixed at this
  stage; what is bought is that the model/param judgement is made once, in
  writing, by someone reading the code — and that judgement is the expensive
  half. Public signatures do not change, so the shards stay disjoint.
- **Stage B — thread the real scale, per entry point.** Each crate's public
  entry points derive a real `model_scale` from their input and thread the
  context inward, deleting `unscaled_legacy()` calls as they go. This is what
  actually discharges BG-TOL-001 and BG-TOL-002.

**Canonical-space quantities are a third case, and `truck-geometry` is full of
them.** Added 2026-08-16 while sizing the `specifieds` shard. The model/param
dichotomy above assumes every length is a model-space length. It is not:
`UnitCircle`, `UnitHyperbola`, `UnitParabola` and friends are *canonical*
primitives whose geometry is expressed in their own normalized frame, where the
radius is 1 by construction. A distance in that frame is a dimensionless
multiple of the unit radius, so it must **not** scale by `model_scale` — but it
is a distance, and reads exactly like a model-space one:

```rust
// UnitCircle<Point2>::search_parameter -- canonical frame, radius 1
if v.magnitude().so_small() { return None; }
```

The rule is that **the frame the quantity lives in decides, not its type.** A
site inside a canonical primitive is `param` even when it compares a magnitude;
a site that has been transformed back into model space is `model` even when it
compares a ratio-looking number. A shard covering `truck-geometry/src/**` must
state this or it will classify by type and get the whole `specifieds` module
backwards. Sites where the frame is genuinely unclear take `FIXME(BG-TOL-001)`
as usual.

**Squared-order sites are out of Stage A's scope.** `near2` and `so_small2`
compare against `TOLERANCE2 = TOLERANCE²  = 1e-12`, and `ToleranceCtx` has no
squared-order predicate — `tau_rep` is first order and nothing on the type
reproduces `1e-12`. There are **23 such sites** across the vendored tree
(`rg -c '\.near2\(|\.so_small2\(|TOLERANCE2' vendor/truck`, 2026-08-16), and a
shard that mapped them onto `tau_rep` would loosen every one of them by six
orders of magnitude while appearing to migrate them. A shard therefore leaves
them exactly as they are and marks each `FIXME(BG-TOL-001): squared order`.
Deciding what a squared-order tolerance means in a scale-relative system — is
it `(tau_rep · scale)²`, a distinct `tau` , or a squared-distance comparison
that should have been a first-order one on the distance — is design work and is
**BG-TOL-004**, which does not block any shard.

**Quantities of degree ≠ 1 in length are out of Stage A's scope too, and
this is a different exclusion from the squared-order one.** A squared-order
site is recognised by its *constant* (`TOLERANCE2`); a degree-2 site is
recognised by its *quantity*. `(b - a).cross(c - a).so_small()` compares twice a
triangle's area against `TOLERANCE`; `Matrix3::from_cols(a, b, dir).determinant()
.so_small()` compares a scalar triple product. Both are degree 2 in length, so
under a model rescale by `k` the quantity scales as `k²` while
`ctx.length_margin()` scales as `k`. Classifying such a site `model` and
rewriting it `ctx.is_small_len(...)` is exactly correct at Stage A — where
`model_scale = 1.0` makes the two identical — and silently wrong the moment
Stage B threads a real scale. That is worse than not migrating it, because Stage
B will then see a migrated site and never look again.

A shard therefore leaves these exactly as they are and marks each
`FIXME(BG-TOL-001): <quantity> is an area (length squared); neither predicate
fits`. Deferred to **BG-TOL-004** with the squared-order family, which must
decide whether `ToleranceCtx` grows a degree-aware predicate or whether these
sites should compare a first-order quantity instead.

This exclusion is written down because the loop has discovered it twice and
paid for it twice. A worker on an earlier shard hit it unprompted at
`truck-modeling/src/geom_impls.rs:91` and left the FIXME on its own judgement;
the spec did not record it, so the `truck-meshalgo` survey a session later
proposed `is_small_len` for six sites of the same shape — and its own stated
reason for one of them called the quantity "a length-squared quantity" while
applying the length predicate anyway. An exclusion that lives only in one
worker's inline comment is an exclusion the next worker will not find.

`unscaled_legacy()` is a scaffold and is the obvious way to leave the job half
done, so it is **ratcheted, not trusted**: `scripts/kernel-gates.sh` counts its
occurrences against a recorded ceiling and fails when the count rises. The
ceiling only ever moves down, one Stage-B packet at a time, and BG-TOL-001 is
not closed until it reaches zero. A gate that merely forbade the constructor
would forbid Stage A; a ceiling permits Stage A exactly once per site and
permits nothing after.

**Contracts.**
- **BG-TOL-001** No predicate reachable from a public entry point compares a
  model-space length against a literal.
- **BG-TOL-002** Scale invariance: for any operation `op` and uniform scale
  `s > 0`, `op(scale(x, s), ctx.scaled(s))` is isotopy-equivalent to
  `scale(op(x, ctx), s)`.
- **BG-TOL-003** Monotonicity (invariant 7): entity τ ≥ boundary τ.

**Tests.**
- **Property, the important one:** BG-TOL-002 by construction — build a solid,
  scale it by `s ∈ [1e-4, 1e4]`, run the operation, assert same combinatorics
  and proportional geometry. This single test would have caught the whole S-2
  class.
- Unit: `and(a, b)` at `tau = 1e-8` on a millimetre part **returns** (previously
  `nonpositive_tolerance!` panicked — macro defined in `truck-geotrait/src/lib.rs`,
  `rg -n 'macro_rules! nonpositive_tolerance'`; it is an `assert!`).
- Grep test in CI: no `1.0e-6` literal in any `src/**/*.rs` predicate outside
  `ToleranceCtx`'s own defaults.

### BG-CE-001 — Coedge: per-use payload on the edge handle

**Implements** §1 (coedges first-class, pcurve on the use). **Fixes** audit S-1.

**Key fact — this is not a restructure.** truck's `Edge` is *already* a coedge:
`curve` is shared through the `Arc`, `orientation` is per-handle. Two handles to
one edge are already two coedges over one curve. It has exactly one per-use
field. Add a second.

```rust
pub struct Edge<P, C, PC = ()> {
    vertices: (Vertex<P>, Vertex<P>),
    orientation: bool,      // existing per-use field
    pcurve: Option<PC>,     // NEW per-use field — the parametric trace on the owning face
    curve: Arc<Mutex<C>>,   // shared entity geometry (immutability: BG-CE-003)
}
```

**Migration.** `PC = ()` defaults so `pcurve: None` reproduces today's behaviour
exactly. Consumers opt in. This is broad (every `Edge<P, C>` mention across
meshalgo, shapeops, modeling, stepio) but semantically inert.

**What it unlocks immediately.** A seam edge is two handles, one shared curve,
two *different* pcurves — the case §1 says is otherwise impossible.

**Contracts.**
- **BG-CE-001** Coedge pairing (invariant 1): every non-degenerate edge has
  exactly 2 uses of opposite sense, or a declared even number, or a declared 1.
- **BG-CE-002** Same-parameter / same-range (invariant 4), *now statable*:
  `‖Γ_f(pc_u(t)) − c_e(φ_u(t))‖ ≤ τ_e` for **all** t, certified by **interval
  evaluation over the whole span** (BG-ENC-001), not by sampling. Sampling here
  is the classic false pass.

**Tests.**
- Unit: a full cylinder built by `rsweep` carries a seam edge with two uses whose
  pcurves differ by exactly the period.
- Property: for every edge use in a generated solid, BG-CE-002 holds under
  interval certification.
- **Negative test:** an edge whose pcurve is deliberately offset by `2·τ_e` must
  **fail** BG-CE-002. Assert the checker is not vacuous — a checker that passes
  everything is the failure mode here.
- Migration test: with `PC = ()`, every existing truck test still passes bit-identically.

### BG-CE-003 — Immutable geometry, construction-derived identity

**Implements** §20. **Fixes** audit D-2.

**Problem.** Geometry lives in `Arc<Mutex<_>>` with 10 documented deadlock
hazards — `rg -n 'will result in a deadlock' truck-topology/src`, **expect 12**
across `vertex.rs`, `edge.rs`, `wire.rs`, `face.rs`, `shell.rs`, `solid.rs` (the
`mapped` / `try_mapped` family, 2 per file). `VertexID = ID<Mutex<P>>` is *allocation*
identity over a *mutable* cell, and truck documents that mutation preserves the
ID. §20 needs the opposite.

```rust
pub enum EntityId {
    Src(u64),                              // imported entity
    Op { op: OpId, inputs: Box<[EntityId]>, slot: u32 },
    Sel { base: Box<EntityId>, selector: Selector },  // construction-derived, NEVER a geometric query
}
```

Replace `Arc<Mutex<G>>` with `Arc<G>`. Mutation becomes replacement: an edit
produces a new value and a *derived* id.

**Contracts.**
- **BG-CE-003** Identity is a pure function of the construction DAG. Serialising
  and reloading preserves all ids.
- **BG-CE-004** Carrier identity: two carriers are "the same surface" iff their
  `EntityId`s match. Never by geometric comparison (used by BG-S0-001).
- **BG-CE-005** Regeneration totality (OB-5): the old→new id map is **total**
  into `{Preserved, Split(n), Merged(m), Vanished, Ambiguous}`. Correctness of
  any heuristic is explicitly not claimed; totality is.

**Tests.**
- Property: id is invariant under serialise/deserialise round-trip.
- Property: id is invariant under rigid motion and uniform scale (it is
  construction-derived, so geometry cannot affect it).
- Unit: regenerating with a changed parameter maps every old id to exactly one
  variant; assert exhaustiveness by counting.
- Thread test: build and query a shell from 8 rayon threads. Must not deadlock —
  the direct regression for the 10 warnings above.

### BG-CE-006 — Canonical carrier set

**Implements** §2 ($\mathcal{G}$). **Fixes** audit D-3, D-4.

**Problem.** `specifieds/` has Line, UnitCircle, UnitHyperbola, UnitParabola,
Plane, Sphere, Torus — **no Cylinder, no Cone**, the two commonest mechanical
surfaces after the plane. And `truck_modeling::Surface` is a *third, smaller*
set (`Plane | BSpline | NURBS | RevolutedCurve`) that silently drops Sphere and
Torus.

**Algorithm.**
1. Add `Cylinder` and `Cone` to `truck-geometry/src/specifieds/`, following
   `sphere.rs`/`torus.rs`. Cone carries its apex explicitly (the apex must be a
   first-class point — §16.1's apex-vanishing is a *topology event*).
2. Define **one** canonical `Curve`/`Surface` model. Delete `truck-modeling`'s
   competing enum.
3. Conversions preserve analytic identity: extruding a `Circle` yields a
   `Cylinder`, **not** a NURBS. Today `impl ToSameGeometry<Surface> for
   ExtrudedCurve<Curve, Vector3>` in `truck-modeling/src/geometry.rs` degrades it
   via `BSplineSurface::homotopy`.

```rust
pub enum Surface {
    Plane(Plane), Cylinder(Cylinder), Cone(Cone), Sphere(Sphere), Torus(Torus),
    Revolved(RevolutedCurve<Curve>), Extruded(ExtrudedCurve<Curve>),
    BSpline(BSplineSurface<Point3>), Nurbs(NurbsSurface<Vector4>),
}
```

**Why this is Stage 1 and not later.** §16.1 needs cylinder→cylinder ($r \pm d$)
and cone→cone (shifted apex) in closed form. Once code depends on a NURBS
cylinder, every one of those call sites has to be found and changed.

**Note (corpus-contingent tail).** The *core* set above is invariant. `Hyperbola`,
`Parabola`, the NURBS degree cap and span cap (the spec's placeholder "≤3, ≤32")
should be set from corpus measurement, not chosen now. Leave them
`UnsupportedEnvelope` until measured.

**Contracts.**
- **BG-CE-006** Every carrier in $\mathcal{G}$ has: exact evaluation, an
  enclosure impl (BG-ENC-001), a declared periodicity Λ, and a STEP round-trip.
- **BG-CE-007** Analytic preservation: an operation whose exact result is an
  analytic carrier emits that carrier, never an approximation of it.

**Tests.**
- Property: `tsweep(circle, v)` yields `Surface::Cylinder`, and its point set
  agrees with the NURBS construction to `tau_rep`.
- Property: STEP round-trip preserves the variant for all 5 analytic surfaces.
- Unit: cone offset by `d` > apex distance is detected as a topology event, not
  silently emitted with a flipped apex.

### BG-INV-001 — Invariant checkers

**Implements** §1.1 invariants 1–9. One checker per invariant, each returning
`Outcome<()>` with a **localising witness** on failure.

| ID | Invariant | Status in truck | Action |
|---|---|---|---|
| BG-INV-101 | Coedge pairing | ✅ `ShellCondition::Closed` | keep, wrap in Outcome |
| BG-INV-102 | Vertex link = single cycle | ✅ **correct** — `singular_vertices` tests link connectivity, which given `Closed` implies a single cycle since every link node then has degree 2 | keep; **document the dependency on `Closed` so nobody "simplifies" it** |
| BG-INV-103 | Euler–Poincaré | ✅ available | keep; never accept as a substitute for INV-2 (a pinch point satisfies it) |
| BG-INV-104 | Same-parameter/range | ❌ | BG-CE-002 |
| BG-INV-105 | Domain–boundary correspondence | ❌ | new |
| BG-INV-106 | Representation in $\mathcal{G}$ within τ_rep | ❌ | new, needs BG-FID |
| BG-INV-107 | Tolerance monotonicity | ❌ | BG-TOL-003 |
| BG-INV-108 | Shell nesting | ❌ | **new — this is audit F-1** |
| BG-INV-109 | Wedge non-degeneracy (dihedral bounded off 0 and 2π) | ❌ | new; required for BG-FID-001 to be positive |

**BG-INV-108 algorithm** (fixes F-1: today `Solid::new(connected_components())`
declares disjoint lumps to be a solid-with-cavities):

```
nesting_forest(components) -> Outcome<Vec<Solid>>
  for each component C: pick a certified point p_C on C  (needs BG-NUM-004)
  for each ordered pair (C, D): inside(p_C, D)  by certified winding, not `count >= 1`
  build containment partial order; verify antisymmetry — a cycle is Contradictory
  each maximal element is one Solid; its immediate children are its inner shells
  verify inner shells are mutually disjoint
  return one Solid per root  # NOT one Solid containing everything
```

Note the signature consequence: `and`/`or` must return `Vec<Solid>`, not
`Option<Solid>`. Make that break now, in Stage 1, while the call sites are few.

**Tests.**
- Unit (F-1 regression): union of two **disjoint** cubes ⇒ **two** solids, each
  with one boundary shell. Today this returns one solid with a phantom cavity.
- Unit: a cube with a concentric cubical void ⇒ one solid, two shells, correct
  inner/outer assignment.
- Unit: three-level nesting (solid ⊃ void ⊃ solid) ⇒ two solids.
- Property: for every generated solid, all nine invariants hold.
- **Negative tests for every checker.** Hand-build a violator of each invariant
  and assert the checker *fails*. A checker never exercised against a violator is
  assumed broken.

---

## 3. Stage 2 — certified evaluation interface

**This is the bottom of the certified stack and it does not exist in truck at
all.** `ParametricCurve::subs(&self, t: f64)` hardcodes the parameter to `f64`
(audit D-1), so nothing can be evaluated over a box. Every certified quantity in
the formal system is an enclosure over a box, so Stages 3+ are *unimplementable*
without this — not merely uncertified.

### BG-ENC-001 — Enclosure traits

**Design decision: a parallel interface, not a rewrite.** The existing `f64`
traits survive untouched as the fast path. This is what production kernels do.

```rust
pub struct Interval { pub lo: f64, pub hi: f64 }   // outward-rounded throughout
pub struct Box3 { pub x: Interval, pub y: Interval, pub z: Interval }
/// Enclosure of a set of unit directions: axis + half-angle.
pub struct DirCone { pub axis: Vector3, pub half_angle: f64 }

pub trait EnclosureCurve: ParametricCurve<Point = Point3> {
    /// MUST contain { self.subs(t) : t ∈ tt }. Soundness before tightness.
    fn enclose(&self, tt: Interval) -> Box3;
    fn enclose_der(&self, n: usize, tt: Interval) -> Box3;
    /// None when the derivative enclosure contains 0 (direction undefined).
    fn tangent_cone(&self, tt: Interval) -> Option<DirCone>;
}

pub trait EnclosureSurface: ParametricSurface<Point = Point3> {
    fn enclose(&self, uu: Interval, vv: Interval) -> Box3;
    fn enclose_der(&self, m: usize, n: usize, uu: Interval, vv: Interval) -> Box3;
    /// Drives §9.1's transversality predicate, §6.2(ii)'s angle condition, and
    /// §6.2(iv)'s fibre blocks — the normal cone over a cell is what defines the
    /// block that BG-FID-008(iv-b) requires the approximant to stay inside.
    fn normal_cone(&self, uu: Interval, vv: Interval) -> Option<DirCone>;
    /// Lower bound on ‖S_u × S_v‖ over the box — §10's immersion margin ι.
    fn immersion_lower_bound(&self, uu: Interval, vv: Interval) -> f64;
}
```

**THE contract, and the only one that matters:**

> **BG-ENC-001 (Soundness).** For every carrier and every box,
> `enclose(box) ⊇ { f(p) : p ∈ box }`. Over-estimation is always acceptable;
> **under-estimation is a silent-wrong-answer bug** and invalidates every
> certificate built on top of it.

**BG-ENC-002 (Convergence).** `width(enclose(box)) → 0` as `width(box) → 0`, at
worst linearly for analytic carriers and quadratically under subdivision for
B-splines. Required for BG-NUM termination.

**BG-ENC-003 (Outward rounding).** All interval arithmetic rounds outward. Never
compile enclosure code with fast-math or FMA contraction that could round inward.

### BG-ENC-002 — Analytic carriers

Closed-form. For `Plane`, `Line`, `Circle`, `Cylinder`, `Cone`, `Sphere`,
`Torus`: propagate intervals through the parameterisation, being careful that
`sin`/`cos` over an interval must account for the extrema at $k\pi/2$ **inside**
the interval, not only at the endpoints. This is the classic interval
trigonometry bug and it under-estimates, so it violates BG-ENC-001.

**Tests.** Property, for each carrier: sample 10⁴ points in a random box, assert
every one is inside `enclose(box)`. This is the direct BG-ENC-001 test and it
must run for every impl. Property: BG-ENC-002 convergence under bisection.
Unit: an interval spanning $\pi/2$ encloses `sin = 1` (the trig bug).

### BG-ENC-003 — B-spline and NURBS carriers

**Do not use naive interval arithmetic here.** The convex-hull property gives a
*tighter* and *cheaper* enclosure: over a knot span, the curve lies in the convex
hull of its control points. Truck already has the pieces —
`BSplineCurve::control_points`, subdivision, and `roughly_bounding_box`
(`BSplineCurve::roughly_bounding_box` in `truck-geometry/src/nurbs/bspcurve.rs`)
which is already a control-hull enclosure.

```
enclose(bsp, tt):
    extract the sub-curve over tt by knot insertion (exact, rational)
    return bbox(control_points)          # convex hull property
tangent_cone(bsp, tt):
    hodograph = derivative curve (control points = scaled forward differences)
    return cone over bbox(hodograph control points), None if it contains 0
```

For NURBS, the hull property holds in **homogeneous** coordinates; project
carefully — the projection of a hull is not the hull of the projection unless all
weights are positive. Assert positive weights (BG-CE-006) or refuse.

**Tests.** Property: BG-ENC-001 by sampling, over random knot vectors and degrees.
Property: enclosure is *tighter* than naive interval arithmetic (the reason for
this design). **Unit: a NURBS with a negative weight is refused, not silently
mis-enclosed** — this is the one place the hull property fails.

### BG-ENC-004 — Decorators

`Processor` (affine transform — map the box corners, exact for affine),
`RevolutedCurve`, `ExtrudedCurve`, `PCurve`, `Offset`, `IntersectionCurve`.
Compositional and the real work of this stage.

`IntersectionCurve` is the interesting one: its enclosure must account for the
fact that the leader polyline is an *approximation*. Enclose the leader, then
inflate by the certified Newton residual bound. Never treat the leader as exact.

**Tests.** Property: BG-ENC-001 for each decorator. Composition property:
`enclose(Processor(s, M), box) ⊆ M · enclose(s, box)` inflated by rounding.

---

## 4. Stage 3 — fidelity and solvers

The formal system's declared **root** (§23 node 0), plus its numerical substrate.
Pure mathematics; ~100% corpus-invariant.

### BG-NUM-001 — Budget ledger

**Implements** §7, OB-2. Kills H-5 violations like `approx_rolling_ball_fillet`'s
`for _i in 0..16`.

```rust
pub struct Budget { pub subdiv: u32, pub newton: u32, pub depth: u32 }
impl Budget {
    /// Err ⇒ caller returns Outcome::NumericallyUnresolved with the witness.
    pub fn spend_subdiv(&mut self, n: u32) -> Result<(), Exhausted>;
}
```

**BG-NUM-001.** Every geometry-dependent loop takes `&mut Budget`. Exhaustion is
a **typed terminal state carrying what was spent and where**, never a silent
fallthrough to `None` and never an approximate answer.

**Tests.** Property: for any input, the operation terminates within the declared
budget. Unit: a deliberately tiny budget yields `NumericallyUnresolved`, never a
wrong answer — the Stage-3 instance of BG-TEST-SWEEP.

### BG-NUM-002 — Certified univariate root isolation

**Implements** §7. Bernstein/Descartes subdivision.

```
isolate_roots(f: BernsteinPoly, domain, budget) -> Outcome<Vec<Interval>>
  loop:
    sign_changes = descartes_count(f.coeffs)     # Descartes' rule on Bernstein basis
    0 -> no root in this box, prune
    1 -> unique root; refine to width < tau, emit isolating interval
    _ -> bisect; budget.spend_subdiv(1)?; recurse
```

**BG-NUM-002.** Every returned interval contains exactly one root; their union
contains all roots in the domain. **Multiple roots** (even multiplicity, where
the sign-change count never reaches 1) must return
`NumericallyUnresolved`, **not** an empty list — reporting "no root" for a
tangential double root is precisely the §9.2 failure this whole system exists to
prevent.

**Tests.** Property against exact rational arithmetic for random low-degree
polynomials. Unit: a double root at a known location ⇒ `NumericallyUnresolved`,
not `[]`. **This is the single most important negative test in Stage 3.**
Property: clustered roots at separation `s` are isolated when budget
$\ge \log_2(1/s)$, and refused below it — the BG-TEST-SWEEP for this item.

### BG-NUM-003 — Krawczyk operator

**Implements** §7. Existence and uniqueness of solutions in a box.

```
krawczyk(F, F', box, budget) -> Outcome<Root>
  Y = inv(midpoint(F'(box)))          # approximate inverse, float is fine here
  K = m - Y·F(m) + (I - Y·F'(box))·(box - m)
  K ⊆ interior(box)  -> Proven(unique root in box)   # existence AND uniqueness
  K ∩ box = ∅        -> Proven(no root in box)
  otherwise          -> bisect, budget.spend_subdiv(1)?, recurse
```

**BG-NUM-003.** `Proven(unique)` is emitted **only** on strict interior
containment. Anything else is not a proof — the common bug is accepting `K ⊆ box`
(non-strict), which proves existence but not uniqueness.

**Tests.** Unit: transverse intersection ⇒ unique root proven. Unit: tangential
contact ⇒ never `Proven(unique)`, always refuse or bisect to budget. Property
against BG-NUM-002 on univariate problems — two independent methods must agree.

### BG-NUM-004 — Certified clustering

**Implements** §5. **Fixes** audit F-2 (hash-grid snapping at 2e-6 absolute,
`impl From<Point3> for PointIndex` in
`truck-shapeops/src/transversal/polyline_construction/mod.rs`, which both splits at 1e-9 and welds at
3e-6).

```
cluster(points_with_radii, ctx, budget) -> Outcome<Vec<Cluster>>
  # §5: p ~τ q is NOT transitive and is NOT used.
  connect i~j iff ball(X_i, r_i + eps) ∩ ball(X_j, r_j + eps) != ∅
  components = connected components of that graph
  for each: compute certified enclosing ball (c_C, R_C)
  admissibility: R_C <= min(tau_col, theta * lfs_lower(C)), theta < 1/2  # needs BG-FID-001
  violated -> refine (re-solve at higher precision, re-cluster) before any refusal
```

**BG-NUM-004.** Clusters are determined by *certified ball overlap*, never by
grid quantisation or a transitive-closure of pairwise nearness. Radii come from
actual solve residuals, not a constant.

**Tests.**
- **F-2 regression (both directions):** two points 1e-9 apart must cluster
  *regardless of grid alignment*; two points 3e-6 apart with radii 1e-9 must
  **not** cluster. The current hash grid fails both, and it fails them
  *depending on absolute position*, so run each at several translations.
- Property: clustering is equivariant under rigid motion and uniform scale. The
  hash grid is not, which is the root of F-2.
- Property: refinement loop terminates or refuses within budget.

### BG-FID-001 — Stratified reach and local feature size

**Implements** §6.1. The formal system's **root** — every other certificate is
downstream. Nothing resembling this exists in truck.

**Every term is a certified lower bound.** Name the function `lfs_lower` and the
type `LfsLowerBound`, not `lfs`; the naming is the enforcement, because a bare
`lfs` invites a future call site to read it as an equality and compare in the
wrong direction.

```
lfs_lower(x, stratum) = min( intrinsic_lower(stratum), separation_lower(x), wedge_lower(x) )
                      <= lfs(x, stratum)          # true value, never computed
```

| stratum | intrinsic (lower bound on reach) | separation | incident structure |
|---|---|---|---|
| face interior | `min(1/κ_max_upper, ½·σ_self_lower)` | lower bound on dist to non-incident strata | lower bound on dist to own boundary wires |
| edge interior | lower bound on curve reach of `c_e` | as above | `ϱ_wedge(ψ)`, → 0 as ψ→0 or 2π |
| vertex | 0-dimensional | star separation | min incident edge length, min angular separation, min dihedral over star |

**BG-FID-001.** `lfs_lower` is computed **per stratum**, never as a single global
reach of `∂Ω`. The global reach of a mechanical B-rep is **zero** — it collapses
at every sharp edge — so any code path using a global reach is a defect. This is
the specific error §6.1 exists to correct, and it is easy to reintroduce.

**BG-FID-002.** `lfs_lower > 0` requires invariant 9 (BG-INV-109): a knife edge (ψ→0) or
a crack (ψ→2π) drives `ϱ_wedge` to zero. Faces whose bound is 0 route to collapse
(§5), not to a certificate.

**BG-FID-007 (bound direction, §6.1).** Every gate has the form `q < c ·
lfs_lower`, so substituting a lower bound is conservative: it can refuse an
instance the true value would admit, and can never admit one the true value would
refuse. Two consequences the code must respect:

- Federer's equality `reach = min(1/κ_max, ½·bottleneck)` holds only for a
  **closed** `C²` submanifold. A trimmed patch has boundary, `κ_max_upper` is a
  computed upper bound, and `σ_self_lower` is a computed lower bound on the
  bottleneck — so no API may return this quantity under a name asserting equality
  with reach, and no test may assert equality against a hand-computed reach.
- **Refusals are epistemic.** `ReachLowerBoundTooSmall` asserts that the bound
  could not be certified large enough, **not** that the feature size is small.
  A diagnostic that says "feature too small" when the bound merely failed to
  converge is a wrong answer with a confident label — exactly the failure mode
  BG-TEST-SWEEP exists to detect.

**Tests.**
- Unit: a cube. `lfs_lower` at a face interior point ≤ distance to the nearest
  edge; at an edge ≤ the wedge bound; at a vertex ≤ the star separation.
  Hand-computed values are **upper** bounds on what the function may return, and
  the assertions are `<=`, not `==`. A tightness assertion here is the bug this
  item exists to prevent from being written.
- **Unit, the anti-regression:** assert the *global* reach of a cube is 0 while
  the *stratified* `lfs_lower` is positive everywhere. This is the test that
  catches a future "simplification" back to a global reach.
- Property: `lfs_lower` is 1-homogeneous under uniform scale, invariant under
  rigid motion.
- Property: `lfs_lower` is positive on any solid passing BG-INV-109 and zero on a
  deliberately constructed knife edge.
- Property (soundness, the analogue of BG-ENC-001): sample the true local feature
  size by brute force on small hand-built cases and assert
  `lfs_lower <= lfs_sampled` always. Over-refusal is acceptable;
  **over-estimation is a silent-wrong-answer bug**, since every downstream gate
  compares against it.

### BG-FID-003 — Isotopy conditions

**Implements** §6.2. The check that licenses transferring exact-object topology
to an emitted approximant (OB-4).

```
isotopy_ok(X_exact, X_approx, rho_lower, budget) -> Outcome<()>
  (i)   two-sided Hausdorff: d_H(X, X') <= eps < rho_lower/2        via BG-ENC-001
  (ii)  angle: ∠(T_x X', T_π(x) X) <= theta < π/2 - asin(eps/rho_lower)  via normal_cone
  (iii) boundary correspondence under π, or both closed
  (iv)  ONE SHEET: every normal fibre meets X' exactly once          # BG-FID-008
```

**BG-FID-003.** Condition **(ii) is mandatory**. It is the one universally
omitted, and Hausdorff closeness alone does **not** imply isotopy — an
approximant can oscillate inside the tube and be topologically wrong. Truck's
current fillet check (the `is_far` closure in `approx_rolling_ball_fillet`)
tests (i) at three sample points
and never tests (ii) at all; that is exactly the shape of the bug.

**BG-FID-004.** The bound is over the **whole span** by interval evaluation,
never by point sampling. Sampling passes on precisely the inputs that matter.

**BG-FID-008 (one-sheet condition — mandatory, and not implied by (i)–(iii)).**
Conditions (i)–(iii) make `π|X'` a *proper local homeomorphism*, hence a covering
map of some constant finite degree. They do not force degree one. The witness:
`X` the circle of radius `R`, `X' = (R + eps·cos(t/2))·e(t)` over `t ∈ [0, 4π]` —
closed, within `eps` both ways, tangent deviation `O(eps/R)`, and a 2-to-1
covering. A checker implementing only (i)–(iii) **passes this input**, and every
certificate above it is then void. Two sanctioned discharges:

```
# (iv-a) degree one per component — cheapest, valid only AFTER (i)-(iii) hold,
#        since it is their consequence (constant fibre cardinality) that makes a
#        single fibre decisive.
for each connected component C of X_exact:
    pick x in C; isolate roots of the fibre equation in the normal disc at x
      by BG-NUM-003, plus an exclusion certificate over the rest of the disc
    count != 1 -> MultiSheetInTube(witness: x, count)

# (iv-b) fibrewise uniqueness on a certified partition — stronger, per-cell
#        witness, and the form BG-FID-005 gets for free (see there).
for each cell D_j of the emitter's partition:
    (a) fibre-block containment: phi(D_j) ⊆ π⁻¹(X_j)     # NOT radial tube containment
    (b) π∘phi|D_j injective: sign-definite Jacobian over D_j (BG-ENC-001)
        + boundary correspondence on ∂D_j
    (c) adjacent cells share only fibres; non-adjacent fibre blocks disjoint
```

Refusals: `MultiSheetInTube { count, witness }` — degree certified > 1, meaning
either a partition too coarse for (iv-b) (refinement fixes it) or a genuine
self-overlap of the emitted geometry (route to the self-intersection engine);
`SheetCountUnresolved` — the fibre root count was not certified within budget,
which is `NumericallyUnresolved` and **not** a fidelity claim in either direction.

Note (a): *fibre-block* containment is strictly stronger than the radial tube
containment a naive implementation checks, and the difference is the entire
content of (iv). A checker that reads (a) as "‖phi(D_j) − X_j‖ ≤ eps" has
re-implemented condition (i) and certified nothing new.

**Tests.**
- **Unit, the motivating case for (ii):** a high-frequency sinusoidal approximant
  with `d_H` well within tolerance but tangent angle > π/2. Condition (i) passes,
  (ii) must **fail**. If this test passes the checker, the checker is wrong.
- **Unit, the motivating case for (iv) — the highest-value negative test in
  Stage 3:** the double-cover witness above, verbatim. (i), (ii) and (iii) all
  pass; the checker must return `MultiSheetInTube`, not `Ok`. Run it for a
  surface too (a double sheet inside one normal tube, with correct tangent planes
  on **both** sheets — the surface case is where (iv) is least intuitive).
- Unit: (iv-a) and (iv-b) agree on a corpus of approximants where both are
  computable; (iv-b) localises, (iv-a) only counts.
- Unit: `rho_lower → 0` (approaching a sharp edge) ⇒ refuse, route to collapse.
- Property: a genuine refinement sequence eventually satisfies all four
  conditions and the outcome is monotone (BG-TEST-SWEEP). Degree-one
  certification must be **monotone under refinement** — once certified at one
  partition depth it stays certified deeper; a test that finds otherwise has
  found an unsound fibre-block bound.

### BG-FID-005 — The `rep` operator

**Implements** §6.3, REP-CRV-001, REP-SRF-001.

```
rep_curve(exact, ctx, budget) -> Outcome<(Curve, Certificate)>
  target eps < min(rho_lower/2, sigma_cl/3, tau_rep)    # §6.3
  loop:
    approximate over a partition {D_j}; measure (eps, theta) by enclosure
      over the whole span
    BG-FID-003 (i)-(ii) satisfied?
      no  -> refine (eps halves, theta = O(h²)); budget.spend_subdiv(1)?; continue
    discharge (iv) by BG-FID-008(iv-b) on {D_j}         # the SAME partition
      MultiSheetInTube -> refine and continue           # coarse partition, usually
      SheetCountUnresolved -> propagate                 # budget, not geometry
    -> Proven with the achieved (eps, theta) and the degree-one certificate
  exhausted -> NumericallyUnresolved
  rho_lower too small -> UnsupportedEnvelope(ReachLowerBoundTooSmall), route to §5 collapse
```

**The partition is free and that is the design point.** `rep` already subdivides
to hit `(eps, theta)`, so its cell decomposition *is* the partition BG-FID-008(iv-b)
requires — fibre-block containment and per-cell Jacobian sign cost no new
subdivision structure, only new assertions on structure that already exists.
Implementing (iv) as a separate post-pass over an opaque emitted curve is the
expensive way to get the same certificate and should be rejected in review.

**BG-FID-005.** No cell emits topology certified for an object whose geometry it
does not emit (OB-4). `rep` is the *only* sanctioned path from an exact result
into $\mathcal{G}$, and it always returns its achieved `(ε, θ)` **and its
degree-one certificate** in the `Certificate` — never a bare curve, and never
`(ε, θ)` alone, since `(ε, θ)` without (iv) is precisely the unsound pairing.

**BG-FID-006 (arrangement label preservation, §6.3).** When `rep` output feeds an
arrangement, additionally require `ε < σ_cl/3` and that no arc's tube meets a
cluster ball it is not incident to. Only then is the DCEL built on approximants
combinatorially isomorphic to the exact one. **(iv) is load-bearing here, not
decorative:** a degree-2 arc contributes two combinatorial arcs, two half-edge
pairs and a spurious face to the DCEL while satisfying every metric bound in
sight — so the isomorphism fails *silently* and the transferred labels are wrong
on a cell that looks certified.

**Tests.**
- Property: for random analytic curves, `rep` output satisfies BG-FID-003 —
  all four conditions — at the certificate's declared `(ε, θ)`.
- Unit: `ReachLowerBoundTooSmall` routes to collapse rather than emitting.
- **Unit:** a `rep` invocation forced to a deliberately coarse initial partition
  on a tightly-curved closed curve must refine on `MultiSheetInTube` and
  eventually succeed, **not** emit at the coarse depth. Assert the refinement
  actually happened (count subdivisions), so a checker that never fires (iv)
  cannot pass by accident.
- **Metamorphic:** `rep` then re-`rep` at the same tolerance is idempotent up to
  `τ_rep`.
- Unit: budget exhaustion returns `NumericallyUnresolved` carrying spend, never
  a best-effort curve. Returning the best effort here is the tempting bug.
- Unit (BG-FID-006): hand-build a degree-2 approximant of an arc, feed it to the
  arrangement path, and assert the label transfer is **refused** rather than
  producing a DCEL with the extra face. This is the test that proves (iv) is
  wired through to the consumer and not merely computed and discarded.

---

## 5. Stage 4 — interfaces only

**Deliberately not specified to implementation depth.** Stage 4's tangential-cell
density (§9.2) is corpus-contingent, and speccing it before measurement would be
guessing. Fix the signatures now so Stage 3 targets them; fill in the bodies
after the corpus exists.

```rust
// §9.1 — scale-invariant transversality predicate. sin θ = ‖n₁×n₂‖/(‖n₁‖‖n₂‖).
fn transversality(s0: &Surface, s1: &Surface, uv0: Box2, uv1: Box2) -> Outcome<Margin>;

// §9.4 SS-TR-001 — depth bound d ≤ log₂(CDL/δσ), modulus ω(ε) = ε/sin θ.
fn ssi_transverse(..., budget: &mut Budget) -> Outcome<Vec<Curve>>;

// §9.2 — the FIRST split is the dimension of {g = 0, ∇g = 0}, not the contact
// order. Getting this wrong is not a wrong answer, it is a budget exhaustion on
// an exactly decidable input. CORPUS-GATED only in its cell density.
enum ContactLocus { Isolated(Vec<Point2>), Curve(TangencyLocus), Region(CoincidentPatch) }
fn classify_contact_locus(..., budget: &mut Budget) -> Outcome<ContactLocus>;

// §9.2.2 SS-TAN-CRV-001 — trace the ridge {∂_e g = 0}, transverse Jacobian λ.
// NOT a singular solve. Gate |λ| ≥ δ_lambda; sign of g on the trace decides
// touch / clear / cross.
fn trace_tangency_locus(..., delta_lambda: f64, budget: &mut Budget)
    -> Outcome<TangencyLocus>;

// §9.2.1 SS-TAN-BLOW-001 — polar blow-up at an isolated contact. Same deflation
// shape as SI-DEF-001; reuse that code path, do not write a second one.
fn blow_up_contact(..., budget: &mut Budget) -> Outcome<BranchSet>;

// §13.1 — material state, the primitive. The orientation table is a derived case.
fn select_coincident(m: MaterialState4, op: BoolOp) -> Option<Orientation>;
```

Two design commitments worth making **now**, because they are corpus-invariant
and constrain Stage 3's interfaces:

- **§13.1 material-state is the primitive**, not the orientation table. Encode
  `(m_A⁻, m_A⁺, m_B⁻, m_B⁺) ∈ {0,1}⁴`; keep the fragment iff `m_R⁻ ≠ m_R⁺`;
  orient toward the `m_R = 0` side. No case enumeration. Verify it reproduces
  the orientation table for two regularized solids — that is the soundness check.
- **§12 propagation, not per-face ray casting.** One certified seed per face,
  spanning-tree propagation, **verify every non-tree edge**, and make cycle
  disagreement a `Contradictory` witness that localises the offending arc.
  `FacesClassification::integrate_by_component`
  (`FacesClassification::integrate_by_component` in
  `truck-shapeops/src/transversal/faces_classification/mod.rs`) is this idea in embryo — its logic
  transfers, its call site does not.
- **§9.2's first split is the locus dimension**, and this constrains Stage 3 rather
  than Stage 4. A solver whose only tangency story is "isolated contact, classify
  by Hessian" will subdivide to budget on every box along a one-dimensional
  tangency locus — which is what a fillet's contact with its supports *is*. Two
  consequences for interfaces built now: `classify_contact` must return a locus
  **dimension** before anything else, and the ridge trace needs a certified
  Hessian enclosure (`enclose_der(2, 0, ..)` and friends), so BG-ENC-001's
  second-derivative path is load-bearing and not optional polish.
- **TAN-SNAP-001 sits at the entry to the atlas, not in the fallback path.**
  Near-degeneracy costs more than degeneracy: the exact case is a rational
  predicate in BG-ANA-002, the near case runs subdivision to exhaustion. So the
  dispatch order is snap-then-decide, and the snap decision is **one certified
  clustering over the whole constraint set** (BG-NUM-004), never a sequence of
  pairwise snaps — a chain of individually admissible snaps can move a feature by
  an inadmissible amount. This is why BG-NUM-004's admissibility bound is reused
  here rather than a new pairwise tolerance being invented.
- **§16.3 topology events are decided by *generalized* critical values**, never by
  smooth Morse points of `dist(·, ∂Ω)`. The distance function of a B-rep is only
  Lipschitz, and the medial sheets generated by edges and vertices — the ones every
  mechanical part has — carry no smooth critical points at all, so a smooth
  critical-point system detects nothing precisely where detection matters. The
  interface must therefore be shaped around the critical function
  `χ(x) = ‖x − θ(x)‖ / d(x)`, with `Γ(x)` the nearest-point set and `θ(x)` the
  centre of its smallest enclosing ball:

```rust
// §16.3 — μ-criticality over a distance band. CORPUS-GATED for the band width,
// but the SHAPE is corpus-invariant: it consumes a nearest-point SET, not a
// nearest point, and the offset gate is `chi >= mu` over the band.
fn critical_function(omega: &Solid, x: Box3, budget: &mut Budget) -> Outcome<Interval>;
```

  This is corpus-invariant and constrains Stage 3: `Γ(x)` spanning two faces
  across an edge is the **normal case**, not a degeneracy, so any nearest-point
  API that returns a single point (or resolves ties arbitrarily) is unusable here
  and must not be the one Stage 3 ships. Note also that only the direction "no
  critical value in the band ⇒ no topology change" is available — the converse is
  false, so a result differing at a critical value is not by itself a defect.

---

## 6. Analytic solver track — runs in parallel, not downstream

**The most useful scheduling fact in this document: certified intersections for
the analytic subset do not wait for Stages 2–3.**

Bernstein subdivision and Krawczyk (BG-NUM-002/003) exist for the *general*
NURBS case. Plane-against-everything, and several special positions, are
**closed form** — solvable exactly, in rational arithmetic, with `μ = Exact`
certificates and no enclosure machinery at all. They need only BG-EVD-001,
BG-TOL-001 and BG-CE-006.

### BG-ANA-001 — Exactly solvable pairs

| pair | result | notes |
|---|---|---|
| plane × plane | line, or parallel / coincident | classified exactly from normals |
| plane × sphere | circle, tangent point, or empty | |
| plane × cylinder | 2 lines / 1 tangent line / circle / ellipse | by axis-normal angle and distance |
| plane × cone | conic section | this is the *definition* of a conic |
| sphere × sphere | circle, tangent point, or empty | |
| coaxial pairs (cyl/cone/sphere/torus) | circles or empty | |
| parallel-axis cylinders | lines or empty | |
| equal-radius cylinders, intersecting axes | **two ellipses**, rational | the classic exact case |

**Not** closed form, and correctly deferred to the general solver: cylinder ×
cylinder in general position, sphere × cylinder in general position, torus ×
plane in general position (all quartic space curves), and anything × NURBS.

### BG-ANA-002 — Analytic tangency is exactly decidable

This is the second reason to run this track early. §9.2's tangential atlas is
the audit's highest-prevalence gap and needs Hessian classification and Milnor
numbers **in general**. For the analytic subset it needs none of that:

- two cylinders are tangent iff `|d_axes| = r₀ ± r₁` — an exact predicate;
- a plane is tangent to a sphere iff `dist(centre, plane) = r`;
- coaxial and parallel-axis degeneracies are exact conditions on the axes.

So the analytic track delivers **certified tangency handling** for the cases
that dominate mechanical parts long before the general track can. Every fillet
is tangent to its supports and every counterbore is coaxial — and in the
analytic subset those are decidable, not approximated.

**BG-ANA-001.** Every analytic pair returns `Proven` with `μ = Exact`, or a
typed classification of the degenerate position. No analytic pair may return a
float-certified result — if it does, it belongs in the general solver.

**BG-ANA-002.** Position classification (transverse / tangent / coaxial /
parallel / coincident) is decided by **exact predicates on the carrier
parameters**, never by sampling the surfaces.

**Tests.**
- Property: for every pair, the emitted curve lies on **both** carriers to
  machine precision — an exact result has no `τ_rep`.
- **Margin sweep, and the important one:** walk two cylinders through tangency
  (`|d| → r₀+r₁`). The outcome must switch cleanly transverse → tangent →
  disjoint, with **no band of wrong-but-confident answers** near the crossing.
  This is where the general solver will struggle and the analytic one must not.
- Cross-validation: once BG-NUM-003 exists, run Krawczyk on every analytic pair
  and assert agreement with the closed form. **The analytic cells are the ground
  truth for testing the general solver** — build them first and get a test oracle
  for free.

### Why this changes the schedule

The audit's §7 warns that Stages 1–2 produce no user-visible capability. This
track is the mitigation: after roughly **5–7k LOC** — BG-EVD-001, BG-TOL-001,
BG-CE-006 plus these cells — the kernel does something real and certified, on
the surface classes that dominate real mechanical parts. It is also the first
point at which the evidence algebra gets exercised against genuine geometry
rather than unit tests, which is when its design flaws will surface.

Sequence it immediately after item 4 in §9, in parallel with BG-ENC.

---

## 7. Global test obligations

Beyond per-item tests. These run in CI over every generated solid.

**BG-TEST-001 — Metamorphic invariants (§21).** No reference implementation
needed: $A\cup^*A=A$; $A\setminus^*A=\emptyset$; associativity and commutativity
up to isotopy; De Morgan within a bounding box; $(A\cup^*B)\setminus^*B\subseteq A$;
$\mathrm{vol}(A)+\mathrm{vol}(B)=\mathrm{vol}(A\cup^*B)+\mathrm{vol}(A\cap^*B)$
within accumulated ε; invariance under rigid motion, uniform scale, knot
insertion, degree elevation and reparameterisation.

**BG-TEST-002 — All nine invariants on every result** (BG-INV-001).

**BG-TEST-003 — Margin sweeps per gated cell** (BG-TEST-SWEEP). The direct test
of epistemic closure.

**BG-TEST-004 — No-panic fuzz.** Fuzz every public entry point with degenerate
input (zero-length edges, coincident vertices, inverted solids, NaN coordinates,
empty shells). Assert an `Outcome` is always returned. `catch_unwind` asserting
it was **not** needed.

**BG-TEST-005 — Scale sweep.** Every metamorphic test re-run at model scales
$10^{-4}$ to $10^{4}$. Catches the entire S-2 class, which is invisible at
scale 1.

**BG-TEST-006 — OCCT as a differential oracle, never a truth oracle (§21).**
Three-way classification: agreement is *corroborating evidence*, disagreement is
a *discrepancy requiring adjudication* (`ours-wrong | occt-wrong |
convention-difference | both-valid-within-tolerance`), and only certified/formal/
metamorphic passes are *correctness evidence*. Compare up to isotopy and
tolerance, never by literal entity match — seam placement, fragment ordering and
face splitting are conventions. Run OCCT offline to build a cached comparand set;
test against the cache at speed.

**BG-TEST-007 — Degree-one regression suite (BG-FID-008).** A standing corpus of
approximants that satisfy §6.2(i)–(iii) and fail (iv): the double-cover circle at
several radii and `eps`, its surface analogue, and one case per emitter that has
ever produced a multi-sheet result. Every one must be **refused**, and the suite
runs against `isotopy_ok`, against `rep`, and against the arrangement consumer —
three levels, because the failure mode is that the condition is computed at one
level and dropped before the next. A checker that passes (i)–(iii) tests and has
no (iv) tests is indistinguishable from an unsound one.

**BG-TEST-008 — Forward-bound soundness.** For random operation chains, compare
the certified forward bound against measured displacement on sampled
perturbations: `measured <= certified` always. Include at least one chain with a
non-subadditive modulus, where the split bound would under-report — the test
exists to prove the implementation took the `propagate` path and not the cheaper
wrong one.

---

## 8. Before handoff — blockers that must be cleared first

None of these is geometry work. All of them will cost more if discovered
mid-build than if fixed now.

### P-1 — ~~Actually vendor. This is the hard blocker~~ DONE

**Closed 2026-08-15.** The truck tree (rev
`c5f4b6e9778e0721a1d446f10568eb5e5594e8ed`) is vendored at `vendor/truck/`
as real path dependencies in `Cargo.toml` (11 crates; the `[patch.crates-io]`
entries point at the same paths so no crates.io truck crate can leak in). The
`.cargo/config.toml` `paths` override block was **deleted**, not commented, and
the sibling `../truck-fork` directory is no longer referenced by the build;
`scripts/ensure-build-space.ps1` was updated to match. Verified:
`cargo check --locked --all-targets` and `cargo qcheck` pass, and
`cargo tree` shows **no git source** for any truck crate.

Truck was **not** vendored before: it was a git dependency pinned by rev in
`Cargo.toml`, with the working copy at the *sibling path* `../truck-fork`. A
coding agent cannot edit `~/.cargo/git/checkouts/`, and it cannot be trusted to
manage a sibling directory that is invisible to this repo's history.

The naming trap (verified 2026-08-15) remains relevant for any future *re-vendor*
or upstream sync: the local directory was `truck-fork`; the GitHub repo is
**`github.com/stefangolas/truck`**. There is no `stefangolas/truck-fork` repo —
`git ls-remote` returns *Repository not found*. Say "the truck repo" and mean
`stefangolas/truck`.

The vendoring move was clean because all four agreed on `c5f4b6e9778e0721a1d446f10568eb5e5594e8ed`:

- `../truck-fork` working-tree HEAD (branch `feature/cone-apex-lift-recovery`);
- the `Cargo.toml` / `Cargo.lock` pin;
- the remote branch tip;
- the tree this specification and the audit were written against.

So there was **no divergence to reconcile** — vendoring was a move, not a merge.

Note for future syncs: the coupling mechanism's failure record is the reason the
override was deleted rather than disabled. When re-vendoring, do not reintroduce
a `paths` override; update `vendor/truck/` in-tree instead.

### P-2 — ~~Make `Outcome<T>` work with `?`~~ DONE

**Closed 2026-08-15.** Decision is recorded in BG-EVD-001: `Outcome<T>` is
`Result<Certified<T>, Refusal>`. `?` works natively, `Proven` vs
`CertifiedEquivalent` is a field of `Certificate` guarded by the `DerivedGateToken`
of BG-EVD-002, and totality/mutual-exclusivity of §4 are preserved. This was a
gap in this document:

H-2 mandates `Outcome<T>` everywhere, but as declared in BG-EVD-001 it is a
plain enum, so `?` does not apply (the `Try` trait is unstable). An agent
following H-2 literally will either write nested `match` pyramids or quietly
revert to `Option` — and the second is the exact failure this whole rewrite
exists to prevent.

### P-3 — ~~CI gates before item 1, not after~~ DONE

**Closed 2026-08-15.** `scripts/kernel-gates.sh` runs in the `cross-platform.yml`
`core` job (all three OSes) against the diff from `origin/main`. The gates are
diff-scoped to the kernel tree `vendor/truck/**`, so pre-existing violations in
the baseline are never flagged and only added kernel code is policed. Validated
in an isolated repo: Gate-1 fires on a new module lacking the deny attributes,
Gate-2 on a bare `1.0e-6` predicate (honouring a `// H-3` opt-out marker), and
Gate-3 on `debug_new!` / `cfg!(debug_assertions)`; a fully compliant change
passes. While the vendored tree is absent from the baseline the gates no-op
(self-removing once P-1 lands on the baseline branch).

H-1…H-7 are prose until something enforces them, and prose rots on contact with
a fast agent. The gates that now exist:

- `#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic,
  clippy::todo, clippy::unimplemented, clippy::indexing_slicing)]` on every new
  module, with `#[allow]` requiring an inline justification comment;
- a CI grep asserting no bare `1.0e-6`-class literal in any predicate outside
  `ToleranceCtx` (H-3);
- a CI grep banning `debug_new` and `cfg!(debug_assertions)` in new code (H-4).

### P-4 — ~~Convert file:line citations to symbol anchors~~ DONE

Closed 2026-08-15. Every citation in this document is now a **symbol anchor**
per the convention in §0 (H-8), not a line number. No action required.

### P-5 — ~~Choose the interval crate; do not hand-roll~~ DONE

**Closed 2026-08-15.** Decision: **`inari` 2.0.0 with `default-features = false`**
(no `gmp`). Verified in an isolated scratch build on this machine: compiles and
produces outward-rounded results (`const_interval!(1.0, 2.0) +
const_interval!(3.0, 4.0)` = `[4, 6]`, exact). It conforms to IEEE 1788.1-2017.
The `gmp` feature is rejected: it pulls `gmp-mpfr-sys`, which has **no MSVC
support** and drags in a C toolchain; inari's README confirms the no-gmp path
retains "all operations required by certain kinds of tasks, such as making fast
robust predicates for computational geometry" — exactly BG-ENC's needs.

**Build requirement (must be written into the BG-ENC-001 wiring, not
invented later):** on x86_64 inari requires FMA — without
`-Ctarget-cpu=haswell` (or later) it `compile_error!`s. The enabled-FMA
instruction set is safe here for two reasons: rustc does **not** auto-contract
`a*b+c` into FMA even when the target feature is on, and inari's FMA is
explicit directed-rounding (`add_ru`/`mul_ru` via `_mm_fmadd_pd` with rounding
mode), which is the *correct* way to achieve outward rounding, not compiler
contraction. The `aarch64` backend needs no FMA. Record the haswell requirement
in the crate's build wiring and never combine it with fast-math.

BG-ENC-001's soundness rests entirely on correct outward rounding, and a
hand-rolled interval type is a classic source of silent unsoundness — the kind
that produces no local test failure while invalidating every certificate above
it. We depend on `inari` instead.

### P-6 — ~~Implement one item end-to-end as the reference~~ DONE

**Closed 2026-08-15.** The reference is `vendor/truck/truck-evidence/`, a new
crate wired into look as a dev-dependency. It establishes the pattern every
later kernel item copies:

- **BG-EVD-001** (`src/outcome.rs`) — `Outcome<T> = Result<Certified<T>,
  Refusal>` (the P-2 shape), `Certificate`, `PropMap`/`Truth` join algebra,
  `Budget` ledger, `Margin`, `Modulus` composition, and `accumulate` with the
  §4 rules. Method is ordered weakest→strongest in the enum declaration so the
  weakest of two is the `max` (H-6).
- **BG-ENC-001** (`src/enclosure.rs`) — `Box3`, `DirCone`, `EnclosureCurve`,
  `EnclosureSurface` on `inari::Interval`.
- **BG-ENC-002 for `Plane`** (`src/plane.rs`) — the affine carrier: exact
  interval arithmetic on the parameterisation, constant normal cone,
  constant immersion bound.
- **Harness** (`src/harness.rs`) — `assert_encloses_curve`,
  `assert_encloses_surface`, `assert_converges`: the BG-ENC-001 soundness test
  written once, reused by every carrier.
- **Tests** — 13 unit witnesses + 3 proptest properties (soundness sampling,
  monotone bisection convergence, totality). The margin sweep is a no-op for a
  plane because the affine enclosure has no margin parameter.

`inari` is a crates.io dep with `default-features = false` (no GMP) and the
x86_64 AVX+FMA requirement is handled by a target-scoped `rustflags` entry in
`.cargo/config.toml` (see P-5). Everything passes `cargo fmt --check`,
`cargo clippy --all-targets` (deny lints hold; test-only `#[allow]`s carry
justification), and `cargo test --all-targets` for the crate and the repo.

Point the agent at it as the template.

**Amendment required (formal system r3, not yet landed).** `truck-evidence`
shipped against the r2 shape of BG-EVD-001 and needs three changes before
anything is built on it — they are cheap now and expensive once carriers depend
on the types:

1. `Modulus` becomes a struct carrying `domain` and a shape-derived
   `is_subadditive`, and gains `propagate` (the nested recurrence) as the default
   error-propagation path; `compose` becomes the opt-in fast path that refuses a
   non-subadditive operand. The landed `Modulus` composition implements the split
   bound unconditionally, which is unsound for any modulus that is not
   subadditive — see BG-EVD-004.
2. `Refusal` gains `ForwardToleranceExceeded`.
3. The `ModulusShape::Pole` variant is added, so a near-degenerate cell can
   publish an honest non-subadditive modulus instead of `Unbounded`. Without it
   the type system pushes such cells toward the wrong declaration.

The margin-sweep note in P-6 stands: it is still a no-op for a plane.

### P-7 — Corpus requirements (side workflow)

A corpus is being generated separately. To unblock the deferred work it must
yield **measurements**, not just files. Target these specifically:

| Needed for | Measurement |
|---|---|
| §2 carrier tail | frequency of hyperbola/parabola carriers; NURBS degree and span-count distributions (the spec's "≤3, ≤32" are placeholders) |
| §8 envelope | distribution of $N_{\text{copies}}$ and $N_{\text{crossings}}$ per periodic face |
| §7 budgets | subdivision depth actually needed at the 99th percentile |
| §9.2 locus dimension | **the first thing to measure, ahead of contact order:** the split of tangential contacts into isolated / curve / region. r4 asserts the curve case dominates; if the corpus disagrees the whole cell ordering changes |
| §9.2 tangential density | prevalence of tangential contact by contact order — **is $A_2$ enough, or is $A_3$ real?** |
| §9.2.3 snapping | distribution of $\eta$ (distance to exact tangency / coaxiality / coincidence) — this sizes TAN-SNAP-001's yield, and it is a distribution of *file* artefacts, so measure it on round-tripped STEP rather than on native geometry |
| §17 blend mix | fillet vs chamfer share; edges per blend feature; **$k$ at blended vertices** — the $k=3$ / $k\ge4$ split decides how much of BLD-CNR-SETBACK-001 is actually needed |
| §9 analytic share | fraction of face pairs falling in BG-ANA-001's exactly-solvable set — this sizes the analytic track's payoff |
| §15 operation mix | which generative operations actually occur |
| §16.3 topology events | distribution of weak feature size against typical shell thickness and offset distance — sizes how often the generalized-critical-value path is exercised at all |
| §21 weights | per-cell prevalence for verification weighting |

**Topological coverage to aim for**, in rough priority: seams and periodic faces
(closed cylinders, spheres, tori); poles and degenerate edges (cones, spheres of
revolution); solids with genuine voids (inner shells — the F-1 and BG-INV-108
case); disjoint multi-lump results; non-manifold and knife-edge inputs (to
exercise BG-INV-109 and the collapse path); tangential contacts, especially
fillet-to-support and coaxial counterbores; and coincident faces (for §13.1's
material-state selection).

Note that a corpus of *valid* parts only exercises half the system. Roughly a
third should be **defective** — bad pcurves, gaps, inverted shells, knife edges
— because §19's repair atlas and the refusal paths are where epistemic closure
is actually tested, and they are invisible to a clean corpus.

---

## 9. Order of work

Strict dependency order. Items on the same line are parallelisable.

```
0.  P-1 .. P-6                               DONE (see §8)
1.  BG-S0-002, BG-S0-003                     Stage 0; BG-S0-001 landed 2026-08-16
2.  BG-EVD-001                               everything below returns Outcome
3.  BG-TOL-001                               every signature below takes ctx
4.  BG-CE-001, BG-CE-003, BG-CE-006          one breaking data-model release
5.  BG-INV-001                               needs 4; BG-INV-108 also fixes F-1
5b. BG-ANA-001, BG-ANA-002    ── parallel ── needs only 2,3,4; FIRST REAL CAPABILITY
6.  BG-ENC-001 → 002, 003, 004               the certified substrate
7.  BG-NUM-001 → 002, 003                    needs 6
8.  BG-FID-001                               needs 6; the spec's root
9.  BG-NUM-004                               needs 8 (admissibility uses lfs)
10. BG-FID-003 → BG-FID-005                  needs 8, and 7 (BG-FID-008(iv-a)
                                             isolates fibre roots by Krawczyk)
11. Stage 4 bodies                           after corpus measurement (P-7)
```

**Do not reorder 6 before 4** (enclosure impls would be written against the old
carrier set) **or 8 before 6** (`lfs` needs curvature enclosures).

**The item most likely to be got wrong is BG-ENC-001**, because an
under-estimating enclosure produces no test failure locally — it silently
invalidates every certificate built on it. Its sampling property test is the
highest-value test in the entire specification.
