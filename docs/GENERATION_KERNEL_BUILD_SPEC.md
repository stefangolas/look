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
> - **BG-FID-001a / bridge lemmas (session 19)** — the edge wedge term is now
>   `chi_lower`, a certified bound on [CCSL09]'s χ_K derived from BG-INV-109's
>   sine certificate (branch ambiguity and midpoint scope documented); CCS05
>   T2.1/T2.2 named as the theorem BG-FID-003 instantiates, CONDITIONAL on
>   L-TUBE, L-COVERING, L-SEPARATES (open proof obligations); Federer equality
>   demoted to motivation pending L-FEDERER-PATCH; naming discipline until
>   then: `FaceScaleComponents.conservative_min()` and
>   `WedgeSlopeLowerBound`, never bare `reach`/`lfs`/`TubeWidthLowerBound`/
>   `ChiLowerBound`.
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

### BG-S0-004 — STEP spline import: supplied knots convert or refuse, never synthesize

**Raised 2026-08-20** by the failing `truck-stepio` `input` property tests
(`b_spline_surface_with_knots`, `nurbs_surface_b_spline_surface_with_knots`,
`nurbs_curve_b_spline_curve_with_knots` — all fail deterministically at the
tree this item was written against; a fourth, `b_spline_curve_with_knots`,
fails on the same degenerate-active-domain class seed-dependently — found by
the BG-S0-004 worker at dispatch, covered by the same generator guard). Not a
loop defect: the behaviour arrived
with the vendored tree in `da72cd5`. The curve-path tiny-interval
reparameterization and the unsorted-knot refusal in the same file predate this
loop and are hereby promoted from local decisions to a named contract that
covers every spline import path.

**Anchors** `vendor/truck/truck-stepio/src/in/mod.rs`.
**Locate** `rg -c 'quasi_uniform_knots'` — **expect 7** (definition + 6 calls);
`rg -c 'ValidatedKnotVector::validate\('` — **expect 3** (curve, surface u,
surface v). **If either count differs, stop.**

**Problem.** Every spline-with-explicit-knots import path
(`BSplineCurveWithKnots`, `BSplineSurfaceWithKnots` u and v axes; the rational
forms dispatch through these) validates the supplied knot vector with
`ValidatedKnotVector::validate` and then handles validation failure — other
than `UnsortedRawKnots` — by **substituting a synthesized quasi-uniform knot
vector** for the source's. Two failures follow:

1. *Silent wrong answer.* A source whose knots fail validation imports
   "successfully" as different geometry: the control points are the source's
   but the knot vector is invented, which changes the represented curve
   (observed: a surface's first u-knot importing as `0.0` against a source
   value of `73.7…`). This is precisely the class BG-EVD-001 exists to
   eliminate: no evidence, no refusal, wrong geometry.
2. *Panic.* The synthesis helper computes `num_ctrl - degree` and calls
   `KnotVec::transform(division, 0.0)`; a source declaring `ctrl <= degree`
   reaches the subtraction underflow (debug) or `transform`'s
   `scalar > 0.0` assert with `division == 0` (release) — an H-1 violation
   reachable from untrusted geometry.

The triggering sources are ordinary STEP: any knot/multiplicity structure
whose active domain `[T_degree, T_ctrl]` collapses (both endpoints inside one
multiplicity run) or whose expanded knot count does not cover
`ctrl + degree + 1`.

**Contract BG-S0-004.** For spline entities that carry explicit knots:

1. Validation failure of **any** variant — `UnsortedRawKnots`,
   `DegenerateActiveDomain`, `ControlPointCountMismatch` — is a typed refusal
   carrying the `SplineConstructionError` witness. It is never repaired by
   knot substitution: no path may replace supplied knots with synthesized
   ones.
2. A knot vector whose **total** range is positive but below the parametric
   tolerance may be normalized to `[0, 1]` before construction — an exact,
   shape-preserving affine reparameterization (the curve path's landed rule,
   now stated for every axis of every path, surfaces included). A true-zero or
   structurally degenerate domain is never normalized; it refuses.
3. Quasi-uniform synthesis is legitimate **only** for entity forms whose
   semantics genuinely omit the knot list (`quasi_uniform_curve`,
   `quasi_uniform_surface`, `uniform_*`), and there too `ctrl <= degree` is a
   typed `ControlPointCountMismatch` refusal, never a panic.

**Tests.** Unit tests on named witnesses per path (curve, surface-u,
surface-v, quasi-uniform curve, quasi-uniform surface): degenerate active
domain refuses; knot-count mismatch refuses (this one imports "successfully"
with wrong knots today — the never-silently-replaced witness);
`ctrl <= degree` on a quasi-uniform form refuses without panicking;
tiny-but-nonzero span converts with the axis normalized to `[0, 1]` and the
other axis untouched. Property round-trip tests generate only non-degenerate
domains (a `prop_assume` guard mirroring `validate`'s condition) and keep
asserting **faithful** conversion — the refusal cases are unit-tested, not
folded into the round-trip property.

**Known adjacent defect, out of scope here:** `BezierCurve`/`BezierSurface`
import uses the panicking `new` constructor on unvalidated source counts —
same H-1 class, different mechanism, to be discharged by its own item.

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

**(M5) The composition constants are order-dependent, and the r2 arithmetic was
wrong.** Added 2026-08-19 after the defect below was found by reading the code
against the algebra. For `self.compose(other)` = ω_self ∘ ω_other, i.e. `self`
applied outside:

$$\omega_1(\omega_2(\varepsilon)) = a\,(b\,\varepsilon^{q})^{p} = a\,b^{p}\,\varepsilon^{pq}$$

so the composite constant is **a·b^p**, not a·b. It depends on which operand is
outside. The full table, and each arm is a contract:

| ω_self (outer) | ω_other (inner) | composite |
|---|---|---|
| `Lipschitz(a)` | `Lipschitz(b)` | `Lipschitz(a·b)` |
| `Lipschitz(a)` | `Holder{k, q}` | `Holder{a·k, q}` |
| `Holder{k, p}` | `Lipschitz(a)` | `Holder{k·a^p, p}` |
| `Holder{k₁, p}` | `Holder{k₂, q}` | `Holder{k₁·k₂^p, p·q}` |

**The implementation had rows 3 and 4 as `k·a` and `k₁·k₂`.** The exponents were
right and the constants were not. The error is **not** conservative: for the
subadditive Hölder that `compose` accepts (p ≤ 1) and an inner constant below 1 —
a *contracting* step, which is what a well-conditioned projection or a
normalisation is — the true constant `k·a^p` exceeds the published `k·a` by
`a^(p-1)`, so the published bound **under-reports the forward error**. Measured:
`Holder{1, ½} ∘ Lipschitz(0.01)` publishes `0.01·√ε` against a true `0.1·√ε`, a
10× under-report at every ε; at an inner constant of 1e-6 the factor is 1000.
Composing an outer tangency (p = ½) with an inner contraction is an ordinary
chain, not a contrived one.

**How it survived.** `compose` is live — `Certificate::accumulate` calls it — and
every test that calls it uses `Lipschitz` or `Pole`/`Unbounded` operands only,
which are exactly the arms where the two formulas agree or refuse. The test
named for the property, `modulus_composition_matches_numeric_evaluation`,
exercises `Lipschitz ∘ Lipschitz`. Worse, the test module carries a private
helper `compose_math` that implements the **correct** table, with a comment
saying production "preserves the r2 arithmetic ... which for Hölder is not the
true function composition" — so the correct formula was known, written down, and
used only to compute the tests' expected values. **A test named after a property,
which does not test the property, sitting beside a correct implementation kept
only for tests, is the strongest form of the failure mode H-8 and the "watch it
fail" rule exist to prevent.**

The required property in the Tests block below —
`(ω₂∘ω₁)(ε) == ω₂(ω₁(ε))` over random ε — is therefore a real obligation and was
never discharged for any Hölder operand. It must be run over all four arms, with
the inner constant sampled **below** 1 as well as above, because the direction of
the error changes at 1 and only one side is unsafe.

**Tests.**
- Property: accumulation is associative and commutative in `props` and `margin`.
- Property: `method` is monotone non-increasing under accumulation.
- Unit: `True ⊔ False` ⇒ `Contradictory` propagates to the top.
- Unit: attempting `Proven` with a provisional token fails to compile (trybuild).
- Property: `Modulus` composition matches numeric evaluation —
  `(ω₂∘ω₁)(ε) == ω₂(ω₁(ε))` within float tolerance, over random ε.
- Property (added 2026-08-19): `compose` is **associative** on subadditive
  chains — `(ω₁∘ω₂)∘ω₃` and `ω₁∘(ω₂∘ω₃)` evaluate equal over random ε, and
  both equal the nested application. This pins the order-dependence of (M5)
  to the constants, not to the grouping; an implementation that gets
  associativity wrong is composing the wrong operand into the exponent.
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

### BG-EVD-005 — A published modulus needs a geometric certificate

**Implements** §18 alongside BG-EVD-004. **Raised by external review,
2026-08-19.** Blocks nothing today and should not wait long.

**Problem.** BG-EVD-004 constrains the modulus *algebra* — (M1) ω(0)=0, (M2)
nondecreasing, (M3) validity on `[0, domain)`, (M4) subadditivity decided from
the shape, (M5) the composition constants. Every one of those is a statement
about arithmetic. **Nothing anywhere says a cell must publish the modulus its
geometry actually has.** A tangential intersection may publish
`Lipschitz(1.0)`, every gate stays green, and every bound downstream of it is
wrong. Given that (M5) records the composition arithmetic itself being wrong for
a whole revision without a test noticing, "cells publish honestly" being
entirely unenforced is the larger exposure of the two.

The shapes were chosen for specific geometric configurations — the `Holder`
doc comment says "Tangency is p = 1/2" and `Pole` is documented as what a
near-degenerate cell publishes — but that correspondence lives in prose.

**Contract.**
- **BG-EVD-005** An operation that publishes a `Modulus` publishes it **with a
  witness**, not as a naked enum. The pairing is the contract: a
  `Certified<Modulus>` whose certificate names the configuration it was derived
  from and the quantity that decided it.

**The governing identity (recorded 2026-08-19 after external review).** The
moduli are not a tolerance-propagation utility; they are the kernel's
conditioning framework, and the whole error story is one equation:

$$\varepsilon_{\text{out}} \;=\; \omega_{\text{op}}\big(\varepsilon_{\text{in}}\big) \;+\; \tau_{\text{rep}}$$

— *result uncertainty = conditioning of the operation ∘ input uncertainty +
representation error*. BG-EVD-004/005 make the $\omega_{\text{op}}$ factor
honest (the algebra, then the geometric witness); BG-TOL-005 makes the
$\varepsilon_{\text{in}}$ factor honest (the certified $J_S$ bridge, so input
uncertainty is stated in model space, not in whatever units the chart
happens to use); $\tau_{\text{rep}}$ is BG-TOL-001's budget. Items below that
publish bounds should read against this identity: every term of it is either
certified or the result is a refusal.
- The intersection cells owe, at minimum, this table, each row backed by
  evidence rather than by the author's belief:

  | configuration | admissible modulus | the deciding quantity |
  |---|---|---|
  | transverse | `Lipschitz(k)` | a lower bound on the crossing angle / the transversality λ |
  | near-tangent | `Pole { k }` with a finite domain | how close λ comes to zero, and where |
  | tangency | `Holder { k, p = 1/2 }` | a nonzero Hessian eigenvalue at the tangency (§9.2.2) |
  | coincident / non-identifiable | `Unbounded`, or a refusal | failure to separate the branches |

- A cell that cannot produce the witness publishes `Unbounded` or refuses. It
  does **not** publish a shape it cannot justify. `Unbounded` already means "no
  bound is published" and is the honest fallback.
- The rule this replaces is the one the loop keeps paying for in other forms:
  **a claim that no gate can check is a claim that will eventually be false.**

**Tests.**
- Unit, one per row: build the configuration, assert the published shape is the
  admissible one, and assert the witness is present and non-vacuous.
- **Negative, and this is the point of the item:** a cell hand-built to publish
  `Lipschitz` for a constructed tangency must be **rejected** by the checker.
  A checker that accepts every shape is the failure mode here, exactly as it was
  for the coedge-pairing checker in BG-CE-001.
- Property: for a sampled family deforming transverse → tangent, the published
  domain shrinks and the constant grows monotonically; the shape changes at most
  once and only in the direction transverse → Pole → Hölder.

### BG-TOL-005 — Parameter-space tolerance needs a certified UV↔model bridge

**Implements** §2 (scale invariance) beyond what BG-TOL-001 reaches. **Raised by
external review, 2026-08-19.** Design class; the orchestrator writes it.

**Problem.** BG-TOL-001 splits every predicate into `model` (scales with
`model_scale`) and `param` (does not). That split is real and the Stage-A shards
have now classified most of the tree by it. It is also **not sufficient**, and
the insufficiency is structural rather than a matter of unfinished work:
`ratio_margin()` currently returns `tau_rep` for three different kinds of
dimensionless quantity that do not behave the same way.

| quantity | invariant under | example |
|---|---|---|
| genuinely intrinsic | everything | a sine, a cosine, a unit-vector magnitude, a weight |
| parameter fraction | affine reparameterization only | a normalized knot value in `[0, 1]` |
| **uv length** | **nothing** | `‖δp‖` between two `uv` points |

`ToleranceCtx` today exposes `sin_margin()` and `ratio_margin()` and **both
return `tau_rep`** — two names for one number, which is precisely where the
distinction wants to live and does not yet.

The consequence is that `is_small_ratio(δu) ⟺ |δu| ≤ tau_rep` is a universal
claim that cannot hold. The same physical surface carried on `[0, 1]` and on
`[0, 1000]` gives different answers to the same geometric question. Nothing in
this document currently addresses what happens to any contract under
`(u,v) ↦ (φ(u), ψ(v))`; `Jacobian` appears three times, twice as a *sign*
condition for injectivity (BG-ENC-001) and once as a transversality λ, and never
as a metric bridge. `reparameterisation` appears once, in BG-TEST-001's
invariant list for boolean operations — a test on volumes, not a tolerance
contract.

The Stage-A shards have already produced the empirical shadow of this. Two
sites were classified `param` only because "the frame settles it": a
point-in-polygon test in `PolylineCurve<Point2>` compares a quantity that is
arithmetically `area / length` — a **length** — and is `param` solely because
that `Point2` is a `uv` point. Whether the tolerance on it is *correct* depends
on `‖J_S‖`, which nothing records.

**Contract.**
- **BG-TOL-005** A `param` predicate declares which of the three kinds above it
  compares. Intrinsic quantities keep `tau_rep` unconditionally. A **uv length**
  must go through a bridge, never through `ratio_margin()`.
- The forward bridge is a certified bound over the cell:

  $$\|\delta x\| \le C_{\sup}\,\|J_S\|\,\|\delta p\|, \qquad
    C_{\sup} = \sup_{\text{cell}} \sigma_{\max}(J_S)$$

  so a uv displacement admissible at model tolerance `τ` is one with
  `‖δp‖ ≤ τ / C_sup`. `C_sup` is an **upper** bound and must be certified over
  the whole cell — the same interval-evaluation obligation BG-CE-002 carries,
  and for the same reason: a sampled sup is the classic false pass.
- The inverse direction — "this model-space displacement is at most this much
  uv" — needs a **lower** bound on `σ_min(J_S)`, which is conditioning
  information and is where near-singular parameterizations bite. A cell whose
  `σ_min` lower bound reaches zero is **exactly** the `Pole` case of BG-EVD-004:
  a chart singularity is not a special case to be handled separately, it is the
  same object. Where `σ_min` cannot be bounded below, the inverse bridge refuses.
- `sin_margin()` and `ratio_margin()` stop being aliases. Their divergence is
  the observable signal that this item has landed.

**Tests.**
- Property, the one that fails today: take a surface, reparameterize it
  `u ↦ 1000u`, and assert every `param` predicate's verdict is unchanged. An
  intrinsic quantity passes trivially; a uv length passes only through the
  bridge.
- Property: `C_sup` certified over a cell dominates `σ_max(J_S)` at every
  sampled interior point, with the sampling used only to *falsify*, never to
  establish.
- Unit: a chart singularity (`uder ∥ vder`) drives the `σ_min` lower bound to
  zero and the inverse bridge refuses rather than returning a large number.
- Unit: the `PolylineCurve<Point2>` point-in-polygon sites migrate from
  `is_small_ratio` onto the bridge without changing behaviour at
  `model_scale = 1` and identity parameterization.

**Relationship to BG-TOL-001.** Stage A's `model`/`param` classification is
*not* invalidated by this — every site it marked `param` is still not a
model-space length. What this item adds is that a third of those sites need a
bridge rather than a bare constant, and the classification the shards recorded
is what makes it possible to find them.

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
`FIXME(BG-TOL-001, DEGREE2): <quantity> is an area (length squared); neither
predicate fits`. Deferred to **BG-TOL-004** with the squared-order family, which
must decide whether `ToleranceCtx` grows a degree-aware predicate or whether
these sites should compare a first-order quantity instead.

**The heading says degree ≠ 1 and it means it: degree −1 happens too.** Added
2026-08-19, adjudicating a worker-reported SPEC_GAP at
`truck-geometry/src/decorators/rbf_surface/contact_circle.rs:167`. Everything
above illustrates the exclusion with areas, so a worker reading it looks for a
cross product and finds nothing:

```rust
// next_point -- mat's third column is the UNNORMALIZED normal uder x vder
let del = mat.invert().unwrap() * (q - p);
debug_assert!(del.z.so_small(), "{del:?}");
```

`mat`'s first two columns are surface derivatives (degree 1 in length) and its
third is `uder × vder` (degree 2). `q - p` is a length. So `del.x` and `del.y`
come out dimensionless — they are parameter increments, and the next line uses
them as exactly that — while `del.z` is a length divided by an area and
therefore scales as `1/k`. It is not a length, so `is_small_len` is wrong; it is
not dimensionless, so `is_small_ratio` is wrong; and it moves in the *opposite*
direction from `length_margin()` under a rescale, which is worse than the
degree-2 case rather than better. **The worker was right to stop.**

Such a site is `excluded` and takes `FIXME(BG-TOL-001, DIMENSION): <quantity> is
<dimension>; neither predicate fits`, deferred to **BG-TOL-004** with the rest.
The code is `DIMENSION` rather than `DEGREE2` because `DEGREE2` is already in
the tree on the area sites and a marker that names the wrong dimension is worse
than a generic one. The general rule, which both codes are instances of: **a
`model` site's quantity must be degree ONE in length and a `param` site's must
be degree ZERO. Any other degree is BG-TOL-004's problem, not a shard's.**

That the site is a `debug_assert!` does not change the answer and is worth
saying, because it is the argument someone will reach for. At Stage A
`model_scale = 1.0` and every rewrite here is a no-op, so the cost of migrating
it is zero *today* and the whole cost lands on Stage B, which sees a migrated
site and never looks again. A cheap wrong migration is the failure mode this
exclusion exists to prevent.

This exclusion is written down because the loop has discovered it twice and
paid for it twice. A worker on an earlier shard hit it unprompted at
`truck-modeling/src/geom_impls.rs:91` and left the FIXME on its own judgement;
the spec did not record it, so the `truck-meshalgo` survey a session later
proposed `is_small_len` for six sites of the same shape — and its own stated
reason for one of them called the quantity "a length-squared quantity" while
applying the length predicate anyway. An exclusion that lives only in one
worker's inline comment is an exclusion the next worker will not find.

**BG-TOL-004 adjudication (2026-08-21).** `ToleranceCtx` now carries the
squared companions `length2_margin()` / `is_small_len2(q)` (degree-2-in-length
quantities: `q <= (tau_rep * model_scale)^2`, the sqrt-free form of
`is_small_len`, marginally different from it by one ulp at the boundary *by
construction* — the squared form is the predicate, not an approximation) and
`is_small_ratio2(x)` (the named tight floor `|x| <= tau_rep^2` for
degree-ZERO quantities — knot-normalization and Newton-convergence checks —
deliberately unscaled because degree zero is scale-invariant). The 20
excluded sites adjudicate into four classes:

1. **Squared-distance comparisons** (`distance2 <= TOLERANCE2`): first-order
   predicates written squared to skip a `sqrt`; migrate to
   `is_small_len2` / `length2_margin()`.
2. **Genuine degree-2 quantities** (cross-product magnitudes, areas): same
   predicate, `is_small_len2`.
3. **Dimensionless tight floors** (`knot(i).near2(&1.0)`,
   `next.near2(&param)`): degree zero, scale-invariant as written; migrate to
   `is_small_ratio2` with behaviour preserved exactly (no loosening to
   `is_small_ratio` without a per-site adjudication that accepts the 10^6
   widening).
4. **Dimensionally incoherent** (degree-3 triple products in
   `meshalgo/analyzers/collision.rs`, homogeneous-point comparisons in the
   nurbs `near2_as_curve` family, the `1/k` residual at
   `rbf_surface/contact_circle.rs`): no single predicate fits; these stay
   excluded for **per-site redesign** (normalize to a first-order quantity
   first, or argue their case individually) and are not sharded until that
   argument is made.

Classes 1-3 are the follow-up migration shards' work; class 4 is design work
that stays open. Recording the classes here is what stops a future shard from
"migrating" a class-4 site onto a class-1 predicate — the failure mode this
whole section exists to prevent.

**A `const fn` cannot be migrated at Stage A at all.** Added 2026-08-19, from a
worker's report on `BG-TOL-001-GEOM-NURBS`.
`truck-geometry/src/nurbs/mod.rs:186` is

```rust
#[doc(hidden)]
#[inline(always)]
pub const fn inv_or_zero(delta: f64) -> f64 {
    if delta.abs() <= TOLERANCE { 0.0 } else { 1.0 / delta }
}
```

The predicate is an ordinary `param` site and its rewrite is trivial —
`ctx.is_small_ratio(delta)` — but `ToleranceCtx::unscaled_legacy()` is **not a
`const fn`**, so no context can exist in that body. The only ways through are to
drop `const` from a public signature, to thread a `ctx` parameter, or to make
`unscaled_legacy` const in `truck-base`. All three are signature or cross-crate
changes and all three are Stage B; the first two are explicitly forbidden to a
shard. A `const fn` site is therefore **excluded**, keeps its literal, and takes
`FIXME(BG-TOL-001, CONST_FN): a const fn cannot obtain a ToleranceCtx`.

The worker did drop `const`, reported the contradiction between the packet's
site table and its own Forbidden clause in `disagreements`, and was right that
the two could not both be satisfied. **That is the packet failing, not the
worker.** Note what makes this exclusion different from the others: the site is
correctly classified, the rewrite is correct, and the quantity is the right
degree — it is the *enclosing item* that blocks it. Grep a shard's site list
for `const fn` before writing the packet.

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

**Landed shape (2026-08-21, BG-CE-001 at `6625529`).** The migration turned out
narrower than the mention set suggested — Rust applies defaulted type
parameters in `impl` headers and every type position, so only the struct
definition and the seven struct-literal sites in `edge.rs` changed; V8's
downstream gating over all dependent crates discharged the migration row with
zero edits outside `truck-topology`. Three deviations from this entry's sketch
were forced by the compiler and landed: (1) the inherent impls split in two —
`impl<P, C> Edge<P, C>` for the constructors and PC-free methods,
`impl<P, C, PC> Edge<P, C, PC>` for the per-use payload methods, because a
single generic block leaves `PC` unconstrained at every `Edge::new` call site
(E0282); (2) `with_pcurve<Q>(self, pcurve: Q) -> Edge<P, C, Q>` — attaching a
trace *changes the payload type parameter*, since every constructor returns
`Edge<P, C, ()>` and a `-> Self` signature could never produce a non-`()`
edge; (3) `is_same`, `PartialEq`, `Eq` and `Hash` take the other edge's `PC`
as a separate type parameter, with comparison bodies byte-identical —
identity remains the shared curve pointer plus orientation, and the pcurve
remains per-use payload, never identity. `pre_cut` drops the trace on both
halves (`pcurve: None`); restricting an arbitrary `PC` needs a `Cut` bound
this item does not add, and the packet that wires real pcurves owns trace
splitting.

**What it unlocks immediately.** A seam edge is two handles, one shared curve,
two *different* pcurves — the case §1 says is otherwise impossible.

**Contracts.**
- **BG-CE-001** Coedge pairing (invariant 1): every non-degenerate edge has
  exactly 2 uses of opposite sense, or a declared even number, or a declared 1.
- **BG-CE-002** Same-parameter / same-range (invariant 4), *now statable*:
  `‖Γ_f(pc_u(t)) − c_e(φ_u(t))‖ ≤ τ_e` for **all** t, certified by **interval
  evaluation over the whole span** (BG-ENC-001), not by sampling. Sampling here
  is the classic false pass.

**Design (2026-08-21, the packet).** `certify_deviation(leader, carrier, phi,
tt, tau, budget) -> Outcome<f64>` in `truck-evidence/src/deviation.rs`, where
`phi` is the affine `ParamMap { scale, offset }` (identity, flip, or a range
map). Two routes, both measured against the real carriers before dispatch:

- **Route 1 (the main path): the difference spline.** When both sides expose
  themselves exactly as `BSplineCurve<Point3>` (new `EnclosureCurve::
  exact_spline` default `None`, overridden for plain splines and — via the new
  `EnclosureSurface::as_plane` — for the exactly-affine
  `PCurve<BSplineCurve<Point2>, Plane>` composition) and `phi` is identity or a
  flip, the certificate forms `leader∘phi − carrier` *as a spline* (affine knot
  map, reversal at full-multiplicity endpoints, degree elevation to match,
  exact-count knot merge, coefficientwise subtraction — cgmath point minus
  point is a Vector, so differences are built coordinatewise) and hulls it by
  the convex-hull property over the pre-raised span. This kills the interval
  dependency problem: an exact-agreement pair certifies **one-shot** (measured
  bound 2.5e-14 at tau = 1e-6, zero subdivisions), and a pair offset by 2·tau
  refuses decisively one-shot (lower bound 2·tau). Subdivision by exact-cut
  bisection, budgeted, handles everything between.
- **Route 2 (the fallback): box-minus-box bisection.** For carriers with no
  exact spline (lines, circles, NURBS, curved-surface pcurves), the residual
  box per cell is `carrier.enclose(t) − leader.enclose(phi(t))` with per-axis
  interval subtraction and a norm bound over the box; adaptive bisection under
  `Budget` with a midpoint-representability floor. Honest but
  `O((‖c'‖+‖l'‖)·span/tau)` cells — measured ~130 µs per cell for spline
  carriers, minutes per edge at tau = 1e-6 — which is exactly why route 1
  exists and why the budget is the contract.

Refusals: `Empty` (empty/non-finite span), `ForwardToleranceExceeded { bound,
allowed }` (a certified *lower* bound on some cell's deviation exceeds tau —
the violation proof), `NumericallyUnresolved { spent, witness }` with the new
`UnresolvedWitness::DeviationUncertified` when neither holds within budget.
`tau` needs no validity guard: nonpositive or NaN tau is refused by the loop's
own logic. The certificate carries `Method::Interval` and
`Prop::SoundEnclosure`.

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

**Scope split (2026-08-21, the packet).** The item lands in two rows. The
**design head** is the standalone identity algebra — `EntityId`/`OpId`/`Op`/
`OpKind`/`OpParams`/`Selector`/`End` in `truck-topology/src/entity_id.rs`,
with no truck geometry types, no `Mutex`, no `Arc`, property-tested,
serde-round-tripped, and one stable hasher (FNV-1a over the `Hash` byte
stream finalized by MurmurHash3's `fmix64`, pinned by known-answer constants)
so identity is a pure function of construction content. `OpId` is the stable
content hash of an `Op { kind, params }`; `OpKind` is a closed vocabulary of
the tree's real construction verbs (Primitive, Sweep, Loft, Attach, Boolean,
Fillet, Offset, Transform); `OpParams` is a closed value language
(Unit/Bool/Index/Scalar/Point/Matrix/List) with **bit-wise** float equality
and hashing (`f64` implements neither `Eq` nor `Hash` in std; `-0.0` and
`0.0` are different constructions, NaN is id-stable by bit pattern);
`Selector` is a closed structural vocabulary (BoundaryWire, WireEdge, End,
Seam, Apex, Pole) that carries no coordinates at all — the "never a geometric
query" rule made structural. `truck-topology`'s pre-existing
`compress::SourceEntityId` (STEP-import metadata, a different type for the
same role) is fenced off, not wired. The **wide-mechanical tail** —
`Arc<Mutex<G>>` → `Arc<G>`, the mapped/set_point replacement API, the 12
documented deadlock warnings, and the 8-rayon-thread regression test — is
**BG-CE-003-MIGRATE**, its own row, gated on the algebra landing.
BG-CE-005's regeneration totality stays open design (it needs a regeneration
subsystem, not just the algebra).

**Amendment (2026-08-21, session 17: the MIGRATE design packet).** The
replacement API is resolved, and the ripple was re-derived against the live
tree (the row's original list was grep-measured before filing and is wrong in
both directions):

- **Vertex: `Vertex::new` IS the replacement.** No `with_point` — a fresh
  allocation with the new point is the whole operation. The breaking part is
  documentary: the crate's own doctest claimed "the id does not changed even
  if the value of point changes" (`lib.rs`, `VertexID` docs) — that claim
  reverses. Replacement produces a new allocation id; every existing handle
  keeps the old geometry.
- **Edge and Face gain `with_curve` / `with_surface`** — non-trivial
  replacements that preserve vertices, boundaries, orientation and the pcurve
  payload — plus **`shared_curve` / `shared_surface` → `&C` / `&S`**, the
  generic accessors that replace reaching into the field's mutex (the one
  live in-crate consumer is `invariants/same_parameter.rs`).
- **The derived id is algebra-side, not stored.** Vertices carry no
  `EntityId` (that is CE-005's subsystem). The replacement *event* derives:
  `OpKind` gains a `Replace` arm (additive; no exhaustive `match` on `OpKind`
  exists) and `EntityId::replaced(&self, params: &OpParams) -> EntityId`
  constructs `Op { kind: Replace, params }` over the old id. The generic `P`
  cannot produce `OpParams` (a closed f64 value language), so the derivation
  helper takes `OpParams` directly — layering, not laziness.
- **The corrected ripple.** No edits needed (signatures unchanged):
  `truck-modeling/{sweep,multi_sweep,closed_sweep,mapped,builder}.rs`,
  `truck-shapeops/transversal/integrate/mod.rs`, `truck-meshalgo/src/
  tessellation/triangulation.rs`, all of `truck-stepio`. Edits required and
  previously MISSED by the row: `truck-shapeops/src/fillet/mod.rs` (a live
  `set_curve` at the rolling-ball edge, where shared mutation is load-bearing
  — the boundary already holds the edge — and the fix is `with_curve` plus a
  construction-order swap), `truck-meshalgo/tests/tessellation/
  triangulation.rs` (a `set_curve` in a test; the row's `src/` path was the
  wrong file), and `truck-topology/src/invariants/same_parameter.rs` (the
  mutex-reach). `parking_lot` stays a declared dependency (the `nightly`
  feature references it); only the `use` dies.
- **The `mapped`/`try_mapped` family keeps its signatures, loses
  `#[doc(hidden)]` and all 12 deadlock remarks**, and gains the closure
  doctest the deadlock hazard made impossible: a closure that reads the
  entity's own geometry while mapping it. `VertexID<P> = ID<P>` (likewise
  Edge/Face) — every alias use is source-compatible. The 8-rayon-thread
  regression test lands as `truck-topology/tests/parallel_query.rs`; `rayon`
  is already a regular dependency of the crate, so no manifest change.

**Problem.** Geometry lives in `Arc<Mutex<_>>` with 12 documented deadlock
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
  the direct regression for the 12 warnings above.

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
    RevolutedCurve(RevolutedCurve<Curve>),
    ExtrudedCurve(ExtrudedCurve<Curve, Vector3>),   // reserved; BG-CE-007 emits it
    BSplineSurface(BSplineSurface<Point3>), NurbsSurface(NurbsSurface<Vector4>),
}
```

**Amendment (2026-08-19, session 10): payload-naming, not short names.** The
sketch above originally read `Revolved/Extruded/BSpline/Nurbs`; the dispatched
packet keeps payload-naming (`RevolvedCurve`, `ExtrudedCurve`,
`BSplineSurface`, `NurbsSurface`) — the convention `Curve` already follows —
so the one breaking release does not also churn every construction site.
`RevolvedCurve`'s payload drops the legacy identity `Processor` wrapper (the
sole construction site wrapped with `Processor::new`, i.e. identity;
`RevolutedCurve` carries its own `origin`/`axis`). `ExtrudedCurve` is added
**reserved**: no conversion emits it until BG-CE-007, so this is the last
breaking data-model release. `Curve` gains
`Circle(Processor<TrimmedCurve<UnitCircle<Point3>>, Matrix4>)` — exactly the
type the existing circle conversion consumed and degraded to NURBS; the
conversion now preserves it, and a representable (z-canonical placement,
z-extrusion) circle pair extrudes to `Cylinder` rather than a homotopy
B-spline.

**Amendment (2026-08-19, from the ENUM SPEC_GAP): the placed-surface variant.**
`Surface` also carries

```rust
Processor(Processor<Surface, Matrix4>),   // a placed surface, exact under affine
```

and the `Transformed<Matrix4>` contract for the four z-canonical analytic
carriers is: **translation-only** linear part (exactly identity) moves
center/apex and keeps radius/half-angle; **any other transform** produces
`Surface::Processor(Processor::with_transform(inner, M))` — never a
silently-wrong carrier. (A rotation of a `Cylinder` is a real cylinder, but
not a z-canonical one; moving only its center, as the first attempt did,
publishes wrong geometry with a confident type. A tighter rule for `Sphere` —
rotation about center is carrier-invariant — is allowed later.) This composes
with BG-ENC-004-PROCESSOR, whose enclosure is exact for affine maps.

**Two defects found by the ENUM packet's first dispatch (kept here so the
next packet does not rediscover them):**

1. **The pre-packet circle→NURBS degrade is unsound on full circles.** The
   rational NURBS of a full `(0, 2π)` circle has a weight double-zero at its
   midpoint parameter (`w(s) = (2s−1)²`), so the *old* conversion produced a
   curve that evaluates to **NaN** at that parameter. Preserving the circle
   as `Curve::Circle` is not just analytic hygiene; it removes a live NaN
   from the sweep path.
2. **`RevolutedCurve` parameter search is not branch-consistent for periodic
   profiles.** With a periodic profile (the preserved `Circle`), the search
   can return u values from different branches (observed `−10π` and `11π`
   against hints near the principal branch), flipping boundary orientation
   in downstream tests. The contract: when the entity curve has period T, a
   returned parameter must be normalized by multiples of T to the branch
   nearest the hint (and to the principal branch when no hint is given).
   Impl obligations for new variants (`Invertible`, `Transformed`,
   `ParameterDivision2D`, search traits) live with the enum; a no-op
   `Invertible` on an orientation-free carrier is honest and must be
   documented as such where it is written. `derive_more`'s `From`/`TryInto`
   are hand-written in `truck-geometry` (dep boundary: derive_more is a
   truck-modeling dependency).

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

**Amendment (2026-08-21, scoping BG-INV-108's packet).** The break is
**deferred to the BG-NUM-004 wiring**, and the checker lands first, pure. Two
facts force the split:

1. `nesting_forest`'s inside query is an oracle (the certified winding is
   NUM-004's, still unwritten). The checker therefore takes the oracle as an
   injected parameter — `nesting_forest(n_components, contains: Fn(usize,
   usize) -> Option<bool>)` — and is a pure graph algorithm: build the
   containment order, refuse on a cycle (`Contradictory` with
   `Prop::ShellNesting`), return the forest of roots and their inner-shell
   children, refuse undecided (`NumericallyUnresolved`). Tests exercise it
   entirely on hand-built oracles.
2. Breaking `and`/`or` to `Vec<Solid>` **without** the oracle would fix F-1 by
   introducing its mirror: a boolean result with a cavity (cube minus inner
   cube) has two *nested* components that a naive per-component partition
   would return as two solids, breaking the cavity case that works today.
   A lying break is worse than a deferred one; the signature change lands
   with the oracle, in the NUM-004 wiring packet.

The F-1 regression test itself moves with the wiring.

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

**Amendment (2026-08-21, BG-INV-104 attempt 1's SPEC_GAP).** The same-parameter
checker lives in `truck-topology/src/invariants/` but calls
`truck-evidence`'s certificate — and truck-topology's manifest declared no such
dependency. The edge `truck-topology → truck-evidence` (+ `inari`; and
`truck-geometry` as a dev-dependency for the witnesses) is **acyclic**
(truck-evidence depends on truck-base/geometry/geotrait, not on topology) and
is wired by BG-INV-104's packet as its decision 0. Any future checker in the
invariants tree that speaks interval certificates uses the same edge — it is
the intended layering, not an accident.

---

## 3. Stage 2 — certified evaluation interface

**This is the bottom of the certified stack and it does not exist in truck at
all.** `ParametricCurve::subs(&self, t: f64)` hardcodes the parameter to `f64`
(audit D-1), so nothing can be evaluated over a box. Every certified quantity in
the formal system is an enclosure over a box, so Stages 3+ are *unimplementable*
without this — not merely uncertified.

### BG-ENC-001 — Enclosure traits

**Status (2026-08-19).** The substrate **landed** with P-6 (2026-08-15) as
`vendor/truck/truck-evidence/src/{enclosure,harness,plane}.rs` on `inari`
(no-GMP), with the sampling soundness harness and the `Plane` carrier as the
reference impl. It has no `PACKETS.jsonl` row because it predates the graph —
a session-10 handoff calling BG-ENC-001 "unwritten" was stale. The open ENC
work is the carriers: BG-ENC-002 (analytic), BG-ENC-003 (splines), BG-ENC-004
(decorators), all of which gate on BG-CE-006-ENUM so enclosures are written
against the canonical carrier set.

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

**Amendment (2026-08-21, found while writing BG-CE-002's packet).** The
B-spline-backed carriers violated this in the **terminal strip** of every knot
range: within `tau_rep` (1e-6 at the legacy scale) of a knot, the sub-curve
extraction helper `knot_multiplicity` (one copy each in `bspline.rs`,
`decorators/pcurve.rs`, `nurbs.rs`) counted neighbouring *distinct* knot values
as copies of the cut parameter — `KnotVec::multiplicity` matches by tolerance —
so `raise_to_full_multiplicity` under-inserted, the extracted "sub-curve"
spanned a much larger piece, and the hull plateaued at the wide hull instead of
converging (measured: the enclosure of `[1−w, 1]` stops shrinking for every
`w < 1e-6`). Sound throughout — over-estimation, never under-estimation — but
non-convergent, and nothing caught it because no landed test bisects into the
strip. BG-CE-002's packet fixes all three copies (count exact matches only) and
lands the regression tests. Any future consumer that subdivides enclosures is
a consumer of this fix.

**BG-ENC-003 (Outward rounding).** All interval arithmetic rounds outward. Never
compile enclosure code with fast-math or FMA contraction that could round inward.

### BG-ENC-005 — Certified elementary functions

**Amendment (2026-08-20, from BG-ENC-002-CIRCLE's SPEC_GAP).** BG-ENC-002 below
says to "propagate intervals through the parameterisation, being careful that
`sin`/`cos` over an interval must account for the extrema at kπ/2 inside the
interval". It never said where interval `sin`/`cos` come from, and in this tree
they did not exist: `inari` defines `Interval::sin`/`::cos` in its `elementary`
module behind `#[cfg(feature = "gmp")]`, and `truck-evidence` declares
`inari = { version = "2.0", default-features = false }`. `Plane`, the reference
carrier, is affine and never needed them, so the gap survived until the first
curved carrier was dispatched and stopped on `E0599`.

**The `gmp` route was measured and rejected**, not waved away: it pulls
`gmp-mpfr-sys` and `rug`, which build GMP and MPFR from source through
autotools. The build machine has neither `make` nor `m4`, and its active
toolchain is `x86_64-pc-windows-gnullvm`, which `gmp-mpfr-sys` does not list as
a supported target. Turning it on is a toolchain project, not a feature flag,
and it would put a C build between the kernel and every certified evaluation.

**So the substrate is built in-tree**, in `truck-evidence/src/elementary.rs`, on
the part of `inari` that is not feature-gated — outward-rounded arithmetic,
`sqr`, `floor`, and the correctly-rounded constants `PI` and `FRAC_PI_2`:

    pub fn sin(xx: Interval) -> Interval
    pub fn cos(xx: Interval) -> Interval

Three obligations, each a theorem rather than an estimate, and each the reason a
line of that module looks the way it does:

1. **Enclosure comes from inclusion-monotone interval arithmetic.** Every
   expression is evaluated on intervals, so it encloses its own range.
2. **Truncation is bounded, not estimated.** The Taylor series of sin and cos
   alternate with decreasing terms for |t| ≤ 1, so the error after n terms is at
   most the magnitude of the first omitted term. The partial sum is inflated by
   exactly that.
3. **Argument reduction is an exact identity.** `sin(x) = ±sin(x − k·π/2)` or
   `±cos(x − k·π/2)` for *every* integer k, so the choice of k cannot make the
   answer wrong — only wide. The subtraction runs against the interval `π/2`
   rather than a float, so the reduction's own rounding is carried. When
   cancellation makes the reduced argument leave the series' domain, the
   functions return `[-1, 1]`: loose, never wrong.

**Tests.** Property: containment against a dense sweep of arguments across many
periods, sampled through the quadrant boundaries where the reduction switches
branch — a sign error in the quadrant table dies there and nowhere else. Unit:
an interval spanning π/2 encloses `sin = 1` (the trig bug, at the primitive
level rather than the carrier level). Property: `sin² + cos² ∋ 1`, which is
independent of the reference implementation. Property: a degenerate argument
gives a *narrow* answer — without it, every soundness test above passes on a
function that returns `[-1, 1]` unconditionally.

**Status.** Landed. It is logically prior to BG-ENC-002 and blocks every curved
carrier there; `Line` and `Plane` are affine and do not depend on it.

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

**Amendment (2026-08-20, from writing the BG-ENC-004 fan-out).** Four things
this section did not say, three of which a packet would otherwise have had to
guess and one of which stops an item outright.

**1. The family shares one construction, and it is not per-decorator case
analysis.** Every decorator's normal cone and immersion margin come from the
same three steps: enclose `S_u` and `S_v` as boxes via `enclose_der`, take the
*interval cross product* componentwise, then read both answers off the resulting
box `N`.

    immersion_lower_bound = sqrt(mig(N.x)^2 + mig(N.y)^2 + mig(N.z)^2)
    normal_cone           = Some { axis: mid(N).normalize(),
                                   half_angle: asin(rho / ||mid(N)||) }
                            when rho < ||mid(N)||, where rho = || halfwidths(N) ||
                            None otherwise

The mignitude norm is exactly the box's minimum norm, because each coordinate
attains its mignitude independently; the cone is the ball-around-the-midpoint
bound, `rho < ||c||` being precisely the condition for a ball at distance
`||c||` to subtend less than a half-turn. Round `rho` up and `||c||` down.

This is sound but loose: the cross product of two boxes encloses
`{ p x q : p in a, q in b }`, which decorrelates `S_u` and `S_v` when in truth
they are evaluated at the same point. That is acceptable (BG-ENC-001 permits
over-estimation) and it buys something structural — **every singular locus in
the family becomes the same `None`.** The tangent parallel to the extrusion
vector, the profile curve meeting the axis of revolution, a degenerate
placement: all of them are `rho >= ||c||`, detected numerically, with no
per-carrier apex analysis of the kind BG-ENC-002's `Cone` needed. Use it for
`PCurve` and `IntersectionCurve` too when they are written.

**2. `enclose_der(0, 0, ..)` is the point box, not the zero box.** The trait
documents `enclose_der` as enclosing `der_mn`, and `der_mn(0, 0)` returns
`subs(u, v).to_vec()`. `line.rs`, `cone.rs`, `sphere.rs` and `torus.rs` follow
that; `plane.rs` and `cylinder.rs` return the zero box and are the outliers.
Both are *sound* -- neither under-estimates anything a caller asks for by that
name only if the caller never asks -- but they are not the same function, and a
composition that delegates to an inner carrier's `(0, 0)` inherits whichever
convention that carrier chose. The point box is the contract. The two outliers
are a defect to fix under their own item, not silently in a decorator packet.

**3. `Processor`'s inversion swaps the parameters.** "Affine transform -- map
the box corners" is only half of it, and the omitted half is a soundness bug
waiting to happen. `Processor::subs` with `orientation == false` evaluates
`entity.subs(v, u)`, and `der_mn` swaps both the orders and the arguments:
`entity.der_mn(n, m, v, u)`. An enclosure that reads `orientation` as a normal
flip returns a box that does not contain the surface. Resolve it once, by
swapping `(uu, vv)` and `(m, n)` at the top of each method; the normal reversal
then falls out of the generic cross product above rather than being applied by
hand, and applying both is a double flip.

Two further notes on it. Interval arithmetic on the affine map is *equally
tight* as hulling the eight mapped corners -- each output coordinate is linear
with every input interval appearing once, so there is no dependency loss -- and
it gets outward rounding for free, so prefer it to the corner hull this section
suggests. And `Transform<Point3> for Matrix4` is projective, not affine:
`transform_point` goes through `from_homogeneous`, which divides by `w`, while
`transform_vector` uses the linear part only. For the affine matrices the kernel
constructs, `w` is exactly `1.0` and the divide is exact -- but the type does
not promise it, so the enclosure carries the divide in intervals and returns the
entire box when `w` straddles zero.

**4. `Offset` cannot be written against BG-ENC-001 as it stands, and the reason
is a type error rather than a curvature bound.** `Offset<T, N>` is not "a
surface pushed along its normal by a distance". It is the pointwise sum
`S(u, v) + N(u, v)`, and `truck-geometry`'s only `ParametricSurface` impl for it
is bounded `N: ParametricSurface<Point = C::Vector>` -- the offset field is
**vector**-valued. `EnclosureSurface` is bounded
`ParametricSurface<Point = Point3>`. So for any `S` that is an
`EnclosureSurface`, `N` has `Point = Vector3` and can never be one, and
`impl<S, N> EnclosureSurface for Offset<S, N>` does not typecheck for any
choice of the two.

The classical normal offset is the concrete instance
`Offset<S, NormalField<S, F>>`, and enclosing *that* needs two things this
interface does not have:

- an enclosure for the **unit normal field** `n = (S_u x S_v)/||S_u x S_v||` and
  its partials. This is where curvature genuinely enters -- `n_u` and `n_v` are
  the shape operator -- and it is derivable from what `EnclosureSurface` already
  exposes, since `enclose_der` takes arbitrary `(m, n)` and `n_u` needs only
  second partials of `S` and the existing `immersion_lower_bound` to bound the
  normalising denominator away from zero. Deriving it is not the obstacle;
  having nowhere to put it is.
- an enclosure for `F: ScalarFunctionD2`, the variable offset distance. There is
  no interval scalar-function trait in the crate at all.

Both are **new interface surface in `enclosure.rs`**, which makes
BG-ENC-004-OFFSET a *design* item and not a mechanical one. The registry row is
`BLOCKED`, and `truck-evidence/src/decorators/offset.rs` is scaffolded with the
same explanation so it is found from the code as well as from here. The
resolution to decide when it is picked up: add an `EnclosureVectorField` (and a
scalar sibling) alongside `EnclosureCurve`/`EnclosureSurface`, then `Offset`
composes exactly as the other three decorators do. Note the geometric condition
this buys automatically -- an offset surface is an immersion only while the
offset distance stays under the base's radius of curvature, and past that the
cross-product box straddles zero and `normal_cone` returns `None` on its own.
The self-intersection does not need to be predicted analytically; it is the
family's `None` again.

**Amendment (2026-08-20, session 14: scaffolding PCURVE and ISC).** Two
facts that section did not know, both found by reading the carriers before
writing packets:

**1. `PCurve` and `IntersectionCurve` are curves, not surfaces.** Both are
`ParametricCurve` (a parameter-space B-spline composed with a surface; and a
leader curve projected onto a surface-surface intersection). Their packets
are `EnclosureCurve` impls in `decorators/pcurve.rs` and
`decorators/intersection_curve.rs`. `PCURVE` is a pure composition — hull
the parameter curve in 2D, take the inner surface's `enclose` over the
parameter box, compose `der`/`der2`/`der3` by the chain rule over
`enclose_der` boxes of the surface — and stays mechanical. Its module tree
is scaffolded like the first four decorators'.

**2. `ISC` is not mechanically dispatchable yet, and the reason is the same
shape as OFFSET's.** The item's own rule — "enclose the leader, then
inflate by the certified Newton residual bound" — presupposes a per-curve
leader-vs-truth residual certificate. `IntersectionCurve::subs` is
`search_triple`, an iterative double projection; the leader hull alone
under-estimates by construction (the true point is the projection, not the
leader point), and nothing in the BG-ENC-001 interface certifies the
projection travel: `distance(point, surface)` has no interval-evaluable
closed form for general parametric surfaces. That certificate is exactly
**BG-CE-002's** whole-span deviation bound (or BG-NUM-003's Krawczyk
operator). The registry row now `needs` BG-CE-002; the scaffold's doc
comment records the blocker the offset.rs way. The `der` and `tangent_cone`
halves *are* composable today (the cross-normal formula in intervals off
the surfaces' `normal_cone`s and the leader's derivative hulls), so the
eventual packet is mostly composition plus the one residual bound it waits
for.

**Amendment (2026-08-21, session 17: validating the ISC design in a scratch
crate).** The ISC packet was designed by building and RUNNING the certification
against the real carriers before dispatch. Four findings, one of them a design
correction to this section's own prose:

1. **The residual certificate is the spec's parenthetical alternative, not
   `certify_deviation` itself.** `certify_deviation(leader, carrier, ...)` needs
   both sides as `EnclosureCurve`s, and for the ISC the "truth" side *is* the
   ISC — calling it from inside the impl's own `enclose` is circular (route 2
   bisects by calling `carrier.enclose`). What CE-002 actually unlocked is the
   *interface completeness*: `exact_spline` (for knot-aligned cells) and the
   landed `EnclosureSurface` impls the operator composes over. The impl-side
   residual is a **parametric Krawczyk operator** — this section's sanctioned
   alternative — certifying, per knot-aligned t-cell, that for **every** t in
   the cell the double-projection system has a unique solution inside a
   parameter box Q. The 3D enclosure is then pure composition:
   `midpoint(S0.enclose(Q0), S1.enclose(Q1))`. `certify_deviation` remains the
   *consumer-side* certificate (an ISC carrier can now be the carrier argument,
   INV-104-style); it is not called inside the impl.
2. **The operator's center term is a point evaluation.**
   `K = m − Y·F(m, t_mid) + (I − Y·J(Q, cell))·(Q − m)` — `F` at the float
   point `(m, t_mid)` only. Evaluating the interval `F` over `Q` in the center
   term drags the `p0 − p1` decorrelation (two copies of the solution arc's
   width) into the center and doubles the linear part against the contraction
   term; with it, no box ever certifies (measured: K ≥ 5·width(Q) at every
   scale). With the point center, K's width is second order and certification
   succeeds with wide margin.
3. **Cells must not straddle a leader knot.** A degree-1 chord leader's
   derivative jumps at each knot; the interval `L'` box over a straddling cell
   spans the whole kink fan, decorrelates J's fourth row, and again no box
   certifies. The impl reads the leader's knots through `exact_spline()` and
   pre-splits the span there (a generic leader without an exact spline simply
   pays bisection instead). Related, sound but loose: `bspline.rs`'s
   `enclose_der` hull includes neighbouring-span hodograph control points
   (~6% over-estimate on the witnesses); certification absorbs it.
4. **`enclose_der` for n ≥ 2 is the unbounded box.** The carrier's `ders`
   recursion differentiates the 4×4 system implicitly per order; composing
   that in intervals is not derived, and an unsound widest box is the honest
   answer (the PCURVE fourth-order precedent). n = 0 is `enclose`; n = 1
   composes: `n_box = (S0_u × S0_v) × (S1_u × S1_v)` off the surfaces'
   `enclose_der` boxes over the certified parameter images, scaled by the
   carrier's own `k = (|L'|² − (c−L)·L'')/(n·L')` in intervals — the
   division's divisor straddling zero widens to the unbounded box, which is
   exactly the leader's tangent lying in the constraint plane, the family's
   `None` condition arriving numerically.

Measured on the witnesses (sphere-sphere unit circle, plane-sphere slice,
8/16-segment chord leaders, dev profile): certification succeeds at 6–12 cells
per span, 0.3–2.6 ms per `enclose` call, 0 containment escapes of `subs` on
100-point grids, 0 float `search_triple` parameter escapes from the certified
boxes on 200-point grids, and the degenerate negative (identical surfaces —
the system rank-deficient everywhere) honestly returns the unbounded box.
`tangent_cone` follows the family ball-around-midpoint construction off the
n = 1 hull.

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

**Singular midpoint ⇒ bisect, never refuse** (measured calibration,
2026-08-22): a `None` float preconditioner at the box midpoint — e.g. x²+1 at
m = 0, a vanishing derivative — must take the BISECTION path. A vanishing
midpoint derivative says nothing about the box; refusing there turns every
symmetric no-root instance (x²+1 on [−2, 2]) into a spurious
`NumericallyUnresolved`, and bisection costs nothing when the children prune.
The same branch serves a float midpoint that rounds outside its own box.

**Budget-exhaustion tests must use a case that actually bisects**: the
transverse witness x²−2 on [1, 2] certifies one-shot even at a zero budget
(certification needs no subdivision), so "zero budget refuses" is false for
it. The tangential double root is the exhaustion case — it subdivides to
budget and refuses carrying its spend.

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

**AMENDMENT (session 19, revised after review).** Two stricter naming rules hold
until their theorem chains are discharged: (1) the face-interior three-way
computation ships as `FaceScaleComponents { curvature_radius_lower,
nonincident_separation_lower, boundary_distance_lower }` with a
`conservative_min()` — NOT as any single bound named "tube width", because
`q = conservative_min() <= true-normal-tube-radius` is exactly what
L-FEDERER-PATCH has not yet established; a type named `TubeWidthLowerBound`
may only be constructed by proof-bearing code once that lemma lands (the
evidence architecture enforces this: the certificate type appears when its
proposition exists). Empty exclusion sets are the identity element:
`d(A, ∅) = +∞`, and `min(empty) = +∞`; extended-real values are permitted
and intentional (a plane's curvature radius component is `+∞`). (2) The edge
row ships as `WedgeSlopeLowerBound { value, scope: EdgeMidpointWitness }` — a
local normalized-slope lower bound at INV-109's witnessed point. It is NOT a
`ChiLowerBound`: χ_K(t) is an infimum over an entire distance locus, not one
witness value; promotion to `ChiLowerBound` happens only via L-COVERAGE
(type-level promotion, not prose). Consumers use the inequality form
`q < c · bound` (BG-FID-007), which any conservative bound satisfies
regardless of name.

```
lfs_lower(x, stratum) = min( intrinsic_lower(stratum), separation_lower(x), wedge_lower(x) )
                      <= lfs(x, stratum)          # true value, never computed
```

| stratum | intrinsic (lower bound on reach) | separation | incident structure |
|---|---|---|---|
| face interior | `min(1/κ_max_upper, ½·σ_self_lower)` | lower bound on dist to non-incident strata | lower bound on dist to own boundary wires |
| edge interior | lower bound on curve reach of `c_e` | as above | `chi_lower` — certified lower bound on the critical-function quantity χ_K; see BG-FID-001a |
| vertex | 0-dimensional | star separation | min incident edge length, min angular separation, min dihedral over star |

**BG-FID-001.** `lfs_lower` is computed **per stratum**, never as a single global
reach of `∂Ω`. The global reach of a mechanical B-rep is **zero** — it collapses
at every sharp edge — so any code path using a global reach is a defect. This is
the specific error §6.1 exists to correct, and it is easy to reintroduce.

**BG-FID-002.** `lfs_lower > 0` requires invariant 9 (BG-INV-109): a knife edge (ψ→0) or
a crack (ψ→2π) drives `ϱ_wedge` to zero. Faces whose bound is 0 route to collapse
(§5), not to a certificate.

**BG-FID-001a (edge χ certificate — session 19 amendment).** The edge row's
incident-structure term is a certified lower bound on the critical function
χ_K of [CCSL09] (*A sampling theory for compact sets in Euclidean space*,
Def 4.3), replacing the previously undefined `ϱ_wedge`. Derivation, to be
carried as structured comments at the implementation site:

- For a wedge whose two face normals at the edge make angle φ, the minimum
  norm over the generalized gradient `conv{n_A, n_B}` is `cos(φ/2)`; this is
  the local normalized-slope value the distance function takes on the
  bisector region, and it dies correctly at BOTH knife degeneracies (folded
  ψ→0 and crack ψ→2π both force antiparallel normals, φ→π).
- BG-INV-109 certifies `|n_A × n_B| >= sin_margin`, i.e. a LOWER bound on
  sin φ — which constrains φ away from BOTH 0 and π and cannot distinguish
  the healthy-flat branch from the degenerate branches. The sound two-sided
  consequence is therefore

  ```
  chi_lower = sqrt((1 - sqrt(1 - sin_margin^2)) / 2)
  ```

  monotone increasing in sin_margin: →0 exactly when no non-degeneracy is
  certified (knife witnesses route to collapse, per BG-FID-002), and strictly
  positive whenever INV-109 passes.
- **Known limitation, documented not hidden:** this bound is weak by
  construction (at sin_margin = 1 it still reports only 1/√2, because a sine
  certificate cannot see branch identity). Distinguishing the healthy
  near-flat branch (φ≈0: `dot(n_A,n_B) >= c`) from the near-knife branch
  (φ≈π: `dot(n_A,n_B) <= -c`) needs SIGNED normal-alignment evidence, which
  BG-INV-109 does not provide; and note the direction of usefulness — to
  improve a LOWER bound on `cos(φ/2)` one wants an UPPER bound on φ, i.e. a
  LOWER bound on the dot product. Extending INV-109 with signed alignment is
  future packet work, not FID-001 scope.
- **Sampling scope:** BG-INV-109 v1 samples each edge's parameter midpoint,
  so the edge χ certificate is scoped `MidpointCell` until BG-CE-001's pcurve
  payloads enable whole-span normal enclosure. Consumers must not read it as
  whole-edge.

**Bridge lemmas for BG-FID-003 (session 19 amendment — proof obligations, not
facts).** [CCS05] (*A condition for isotopic approximation*, Thms 2.1/2.2)
gives isotopy from purely topological hypotheses: containment in a common
topological thickening + side separation (+ homeomorphy for T2.1 alone). Its
METRIC tube section assumes C² CLOSED surfaces; trimmed faces are ours to
justify. BG-FID-003's conditions are DESIGNED TO discharge CCS05's hypotheses
through three named bridge lemmas, each carrying statement + proof sketch or
SPEC_GAP:

```
L-TUBE       eps < reach(X) => the closed eps-tube of a compact C²
             surface-with-boundary is a topological thickening whose sides
             are the offset sheets   (closed case = classical tubular
             neighborhood theorem; the with-boundary restriction is OURS)
L-COVERING   transversality/local-inverse (BG-FID-003 ii) + properness +
             certified fibre multiplicity one (iv-a/b) => the fibre
             projection is a ONE-SHEETED COVERING => homeomorphism.
             ("degree one implies homeomorphism" is NOT the theorem; the
             covering property from (i)-(iii) is what supplies properness.)
L-SEPARATES  a continuous one-sheet SECTION of the product thickening —
             a graph inside S×[0,1] — separates the thickening's sides;
             the section property comes from L-COVERING's homeomorphism
             inverse. Fibre-wise "met exactly once" alone does not close
             this proof.
Chain: (i)-(iii) + (iv) => local homeomorphism => covering => homeomorphism
       => continuous section => side separation => CCS05 Thm 2.1 isotopy.
```

Additionally, L-FEDERER-PATCH is an OPEN obligation: given a cell at
certified distance h from its trimmed boundary, curvature bounded above by K,
and certified exclusion of non-incident sheets within radius r, prove the
normal tube of radius min(1/K, r, h) is single-valued over the cell. Until it
lands, Federer's closed-manifold equality is MOTIVATION for the decomposition,
not a proof of it, and no API may claim reach semantics (see the naming rules
above). Likewise L-COVERAGE: local per-stratum χ certificates do NOT compose
into a global r_mu(K)/wfs(K) without certified coverage of all competing
regions — that composition is an open obligation wherever a global quantity
is claimed.

**BG-FID-007 (bound direction, §6.1).** Every gate has the form `q < c ·
lfs_lower`, so substituting a lower bound is conservative: it can refuse an
instance the true value would admit, and can never admit one the true value would
refuse. Two consequences the code must respect:

- Federer's equality `reach = min(1/κ_max, ½·bottleneck)` holds only for a
  **closed** `C²` submanifold. A trimmed patch has boundary, `κ_max_upper` is a
  computed upper bound, and `σ_self_lower` is a computed lower bound on the
  bottleneck — so no API may return this quantity under a name asserting equality
  with reach, and no test may assert equality against a hand-computed reach.
  **AMENDMENT (session 19):** the equality is demoted to MOTIVATION for the
  decomposition's shape until L-FEDERER-PATCH (see BG-FID-001a's bridge-lemma
  register) proves the trimmed-patch tube statement directly; the executable
  contract is `FaceScaleComponents` with certified component directions and a
  `conservative_min()`, whose semantics are "certified inputs to a tube-width
  argument", never "tube width", "reach" or "lfs" — those names wait for the
  lemma.
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

**AMENDMENT (2026-08-23, scoping BG-FID-003's packet after orchestrator review
killed attempt 1 pre-commit).** Five decisions, each fixing a defect the first
packet carried into dispatch:

1. **`rho_lower` is composed, never curvature-only.** The §6.2 signature takes
   `rho_lower` as a certified INPUT; a curvature radius lower bound alone does
   not bound reach (`reach = min(1/κ_max, ½·bottleneck)`; a hairpin with a
   gentle far-away turnaround has curvature radius ~10 and reach ~gap/2 —
   curvature-only composition ADMITS it, the exact silent wrong answer this
   stratified design exists to prevent). The curve-layer type is
   `CurveScaleComponents { curvature_radius_lower, self_separation_lower }`,
   named under the BG-FID-001 amendment's rules: no field, fn or type may claim
   tube/reach/lfs semantics; promotion to a reach statement is L-FEDERER-PATCH
   (open). `self_separation_lower` is the certified minimum of
   `box_distance(X(I), X(J))` over cell pairs at certified PARAMETER gap ≥ G
   (G an input; +∞ when no pair qualifies, the empty-set identity). The
   Federer-motivation composition ships as
   `tube_scale_lower() = min(curvature_radius_lower, ½·self_separation_lower)`
   — the ½ is the motivation shape, NOT a proved reach equality. Gates use the
   inequality form (BG-FID-007: substituting lower bounds is conservative).
   `ReachLowerBoundTooSmall` fires when `2·eps >= tube_scale_lower()` and
   keeps its epistemic reading (line above).
2. **(ii) is an angle between tangent SPACES — unoriented.** The executable
   form uses the absolute dot product: pass when
   `abs_lower(dot_box(D',D)) / (‖D'‖sup·‖D‖sup) > s`, violate when
   `abs_upper(dot_box(D',D)) / (‖D'‖inf·‖D‖inf) <= s`, with
   `abs_lower(I) = 0 if 0 ∈ I else min(|lo|,|hi|)`,
   `abs_upper(I) = max(|lo|,|hi|)`, `s = eps/tube_scale_lower()`. A signed-dot
   form tests ORIENTED tangents and fails the same pair reversed; a
   reversed-parameterization witness must pass identically (required test).
3. **(iii) takes the boundary kind as explicit input.** `EnclosureCurve`
   carries no topology; `CurveBoundary::{Closed, Open}` is supplied per curve
   by the caller, who vouches for it (the carrier owns topology). Mixed kinds
   are a `BoundaryMismatch` (a closed exact with an open approx at sub-eps
   seam gap is circle-vs-interval — NOT isotopic — and no purely geometric
   endpoint check can catch it). For both-Closed, each curve's own endpoint
   enclosures must be within `2·eps` (a consistency gate on the carrier's
   claim, never a closedness certificate — documented as such).
4. **(iv-a)'s annotation must say what one witness establishes.** The module
   establishes conditions (i)-(iii) plus (iv-a) on ONE witnessed disc; the
   promotion of the single witness to whole-span (iv) is L-COVERING's
   consequence of (i)-(iii) (the dependency the iv-a note at "Two sanctioned
   discharges" already states) and MUST NOT appear in `@establishes`.
5. **Distances and search.** Cell pairing uses BOX-BOX distances:
   per coordinate the sup-distance term is `max(|a_lo−b_hi|, |a_hi−b_lo|)`
   and the inf-distance term is `max(0, a_lo−b_hi, b_lo−a_hi)`; the landed
   `one_sheet::sup_distance` is box-to-POINT and is never to be copied for
   box operands. Partner search and separation minimisation run over a
   pruned structure (BVH or paired worklist with bounding-box pruning);
   O(N·M) whole-array scans are a review reject at the witnessed cell counts
   (the ω=4000 sinusoid refines to ~1.6e4 cells). (ii) consumes exactly the
   pairs (i) certified. Witness selection for the fibre check is
   one_sheet-internal (`fibre_degree_one_auto`): callers never choose `t_x`
   and carry no bisection-edge folklore.

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

**Amendment (2026-08-22, scoping BG-FID-008's packet).** BG-FID-008 v1 ships
(iv-a) for CURVE components only, driven by NUM-003's Krawczyk operator on the
univariate fibre equation `h(t) = <X'(t) − x, u>`, with disc-membership by
certified box distance and geometric dedupe of closed-curve duplicate roots.
The SURFACE negative test ("a double sheet inside one normal tube") moves to
BG-FID-005 together with discharge (iv-b): the surface case needs 2D root
certification in the normal bundle, and FID-005's emitter partition is where
(iv-b) — the form the spec itself calls better for emitters — is free. The
curve double-cover witness remains FID-008's flagship negative test verbatim.

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

**AMENDMENT (2026-08-23, scoping BG-FID-005's packet).** Six decisions from
designing the packet, one of them a scope split:

1. **Scope split: REP-CRV-001 is BG-FID-005; REP-SRF-001, the surface (iv-b)
   discharge and the surface double-sheet negative test move once more, to
   BG-FID-005-SRF.** The 2026-08-22 amendment moved them "to BG-FID-005";
   landing the curve rep first is the schedulable increment — the 2D Krawczyk
   operator is generic-`N` and landed, so what is deferred is the surface
   emitter and the bivariate normal-bundle systems, which is packet-sized on
   its own. The curve double-cover witness stays FID-008's.
2. **The emitted approximant is piecewise cubic Hermite in Bezier form.** Per
   cell `[a,b]` (h = b−a): `p0 = X(a)`, `p3 = X(b)`, `p1 = p0 + (h/3)·T(a)`,
   `p2 = p3 − (h/3)·T(b)`, with positions/tangents taken as the midpoints of
   the exact endpoint enclosures (deterministic). Enclosures are the Bernstein
   hull property: position = hull of the four control points, derivative =
   hull of the three difference points, padded outward by the house
   `64·ε·(1+|coord|)` (the BG-ENC-003 padding precedent). Error is O(h⁴)
   (machine-verified: max radial error on the R=2 circle 0.3365 / 0.4292 /
   0.0304 / 0.00196 at depths 0-3 — note depth 1 is WORSE than depth 0, the
   long-cell tangent overshoot).
3. **(iv-b)'s curve form, derived once for the worker.** The emitter shares
   the exact curve's parameter space, so D_j = I_j and the pairing is the
   identity. Per cell, with `s(t)` the projected exact parameter defined by
   `<φ(t) − X(s), X'(s)> = 0`:
   `s'(t) = <φ'(t), X'(s)> / (<X'(s), X'(s)> − <φ(t)−X(s), X''(s)>)`.
   Given (ii) and the tube gate, the NUMERATOR is sign-definite (|cos| > s > 0
   already excludes zero) and the DENOMINATOR is positive (`m² − eps·K > 0`
   rearranges to `eps < ρ`): (iv-b)'s independent content is the knot-
   projection correspondence (each knot's projected parameter lies in the
   shared closure of its two cells) and the non-adjacent separation
   (`box_distance(H_j, E_k) > eps` for k non-adjacent to j, wrap-inclusive on
   Closed) — assertions on boxes the partition already computed. The refine
   arm exists for both; the radial-tube misreading of (a) (the note above)
   remains the trap.
4. **`ReachTooSmall` maps to certification failure, not to a small bound.**
   A small-but-positive `tube_scale_lower` NEVER refuses: refinement drives
   eps under it (an R=0.08 circle at τ=0.05 EMITS at target 0.04 — recorded so
   nobody re-institutes an immediate `2τ ≥ tube → refuse`, which over-refuses
   refinable geometry). The refusal fires when the components cannot be
   certified at all — the collapsing-geometry route: a corner's tangent
   enclosure contains both branch directions at every refinement,
   `curvature_radius_lower_span` returns its epistemic refusal, and rep
   routes to §5 collapse via `UnsupportedEnvelope(ReachTooSmall)`.
5. **Reuse, made concrete:** the cell/BVH/pairing machinery BG-FID-003 lands
   is exposed `pub(crate)` and rep measures (eps, theta) with it — no
   duplicated pairing code, honouring "no new subdivision structure".
6. **`σ_cl` is not gated in v1**: standalone rep has no arrangement context;
   BG-FID-006's consumer adds `ε < σ_cl/3` where it exists. Errors follow the
   fid/ house pattern (typed local enum with an `into_refusal()` conversion to
   the landed §4 `Refusal`, whose `EnvelopeCase::ReachTooSmall` arm is
   documented for exactly this packet); `Refusal` itself has no invalid-input
   arm and must not be stretched.

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

**AMENDMENT (2026-08-23, writing BG-FID-005-SRF's packet).** Eight decisions
from designing the surface packet, each machine-checked through the mandated
formulas before dispatch (outward-rounded interval arithmetic reproducing the
gates' exact box semantics):

1. **The emitted approximant is a tensor-product bicubic Hermite surface in
   Bezier form.** Per cell `[a,b]×[c,d]` (hu = b−a, hv = d−c) the 4×4 control
   net `Q[i][j]` (i u-index, j v-index) is built from the exact surface's
   corner data — positions via `subs`, tangents and twists as midpoints of
   degenerate enclosures (the curve packet's deterministic convention):
   corners `Q[0][0]=S(a,c)` etc.; edge tangents
   `Q[1][0]=P00+(hu/3)S_u(a,c)`, `Q[2][0]=P30−(hu/3)S_u(b,c)` and the v
   analogues; interiors carry the twist with ALTERNATING signs —
   `Q[1][1]=P00+(hu/3)U00+(hv/3)V00+(hu·hv/9)S_uv(a,c)`,
   `Q[2][1]=P30−(hu/3)U30+(hv/3)V30−(hu·hv/9)S_uv(b,c)`,
   `Q[1][2]=P03+(hu/3)U03−(hv/3)V03−(hu·hv/9)S_uv(a,d)`,
   `Q[2][2]=P33−(hu/3)U33−(hv/3)V33+(hu·hv/9)S_uv(b,d)` (the mixed
   second-difference relation fixes the signs; + at (a,c) and (b,d), − at
   (b,c) and (a,d)). Enclosures are the Bernstein hull of the de
   Casteljau-restricted net over the query box (split per axis, the curve
   module's restrict logic), padded by the house `64ε(1+|coord|)`.
2. **Sliver routing (a new enclosure rule, found by machine-check).** A query
   box whose edge lands within ulps of an emitter grid knot produces
   cell-intersection slivers of width ~1e-16; the restricted-net derivative
   scaling divides by the intersection width and explodes (measured: the
   re-`rep` of an emission collapsed its curvature certificate to ~0).
   Intersections narrower than the house width floor (8 ulps at magnitude)
   route through direct point evaluation (the degenerate-axis construction:
   u-derivative column at the line, then the 1D v-curve machinery); the house
   hull pad absorbs the O(sliver) variation. The curve module's `cellOverlaps`
   rule ports per axis: a cell contributes on interior overlap, or when the
   query is a degenerate point on the cell boundary lying inside the cell.
3. **Surface scale components, with RELATIVE convergence and a level cap.**
   `SurfaceScaleComponents { curvature_radius_lower, self_separation_lower }`
   with `tube_scale_lower() = min(curvature, separation/2)` (the
   Federer-motivation shape, never reach — L-FEDERER-PATCH open). Curvature:
   the min over cells of `lfs::curvature_radius_lower` (landed, pub) under
   uniform quad refinement. Separation: the min over QUALIFYING cell pairs of
   `box_distance`, qualification by the Chebyshev point-gap — a pair qualifies
   when `max(gap_u, gap_v) ≥ G` with per-axis FARTHEST gaps (wrapped per
   closed direction via the closed form
   `d_max` if `d_max ≤ P/2`, `P − d_min` if `d_min ≥ P/2`, else `P/2`); the
   max-gap reading is the BG-FID-003-r2 soundness argument lifted to 2D (the
   minimizer's own cell pair always qualifies). Both helpers stop at
   RELATIVE convergence (level change < 5% of the certificate) OR a level
   cap of 7: uniform quad refinement is 4^level cells and the lfs bound's
   deficit is LINEAR in cell width (absolute-0.01 convergence needs level ~11
   = 4M cells), and an absolute threshold FALSELY CONVERGES on the
   garbage-small coarse-level certificates (measured: the R=0.3 belt's
   level-3 certificate is ~0.001 against a converged ~0.076 — stopping
   there would have driven the loop to an infeasible target). The capped
   value is a certified, more conservative lower bound (BG-FID-007).
4. **The refine loop refines ONE axis per step** (a 2D uniform grid squares
   the cell count): the axis with the larger certified sub-image tangent
   extent (max over sub-cells of sub-width × ‖S_axis‖sup; tie → u). On a
   separation failure the refined axis is the one in which the failing pair's
   index distance is ZERO (the non-separating axis — its extent inflates
   cell_eps without widening that gap; measured: the belt's u-edges need
   w_u ≲ w_v/2 to separate). A stall counter (2 consecutive < 1% eps
   improvements above target) returns `Unresolved`.
5. **The surface (ii) gate is the normal-box form:** the tangent-PLANE angle
   via `angle_pass_form(cross_box(φ_u, φ_v), cross_box(S_u, S_v))` — the same
   unoriented |cos| form as the curve, applied to normal boxes.
6. **The surface (iv-b) discharge.** The emitter shares the exact surface's
   parameter space, so the pairing is the identity grid. (a) own-cell
   containment is the per-cell eps measurement (cell_eps[j] = max over
   sub-cells; the (iv-b)(c) gate is per-cell as in the curve packet). (b) the
   grid-vertex projection correspondence: at every interior grid vertex
   (u*,v*), `φ(u*,v*) = S(u*,v*)` exactly (corner interpolation), so the
   bivariate system `F(s,t) = [<φ−S(s,t), S_u>, <φ−S(s,t), S_v>]` has
   (u*,v*) as a root; certify `KrawczykProof::Unique` via `krawczyk::<2>`
   over the box `(u*±wu)×(v*±wv)` (per-axis adjacent cell widths). **The
   first box must certify: once the operator bisects, the root sits exactly
   on the children's shared edge and strict-interior uniqueness is
   unreachable** (the BG-FID-008-r3 bisection-edge trap, 2D edition — the
   split midpoint IS the vertex). A coarse grid's failure to certify is the
   honest refine signal. (c) non-adjacent separation over whole-cell boxes
   with Chebyshev-1 adjacency (index distance ≤ 1 in the max metric) PLUS
   per-direction wrap adjacency when that direction is Closed; corner-sharing
   cells share a fibre and MUST be exempt. Search runs over a 2D BVH local to
   the module (median split on the widest position axis, union-box pruning —
   the isotopy tree's shape with 2D parameter leaves); O(N²) scans are a
   review reject. The typed outcome `MultiSheet { cells }` fires on a
   certified non-adjacent overlap; `rep_surface`'s loop maps it to refine
   (this spec's loop) — a GENUINE double sheet then exhausts the budget or
   stalls to `Unresolved`, never `Ok`.
7. **`SurfaceBoundary::{Open, ClosedU, ClosedV, ClosedUV}`** is caller-vouched
   input per direction (the BG-FID-003-r2 boundary decision, lifted); it
   drives wrap adjacency in separation and wrapped gaps in self-separation
   ONLY — rep v1 runs no boundary-correspondence gate (the curve rep ran
   none either; that condition belongs to the isotopy checker, which has no
   surface form yet).
8. **The surface double-sheet witness** (the spec's negative test):
   `D(u,v) = (R + a·cos(u/2))·(sin v cos u, sin v sin u, cos v)` over
   `u ∈ [0,4π]` (ClosedU — the azimuth is covered TWICE), `v ∈ [π/4, 3π/4]`,
   `R = 2`, `a = eps/2` (the test-3 trap: the deviation is STRICTLY inside
   eps, max = a, never = eps). Both sheets' tangent planes agree with the
   sphere's (|cos| ≥ 0.999, machine-checked) — the surface case where (iv)
   is least intuitive. The scale components CORRECTLY certify ~0
   self-separation (the sheets coincide), so `rep_surface` refuses
   (Unresolved, never Ok), and the DIRECT discharge call at a fixed
   (du,dv)=(7,5) grid returns `MultiSheet` with the failing pair's u-index
   distance ≈ n_u/2 (the two sheets).

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
| coaxial pairs (cyl/cone/sphere/torus) | circles or empty | see the amendment below for the double-nappe cone rows |
| parallel-axis cylinders | lines or empty | |
| equal-radius cylinders, intersecting axes | **two ellipses**, rational | the classic exact case |

**Amendment (2026-08-20, from landing BG-ENC-003-BSPLINE and the first five
BG-ANA-001 shards).** Two corrections the packets pre-decided wrong and the
workers proved:

**1. BG-ENC-003: `subs` extrapolates outside the knot range; there is no
origin union.** The hull-of-control-points ∪ {origin} reasoning applies to the
mathematical Cox–de Boor basis, not to truck's evaluator: `der_n` clamps its
basis window and extrapolates the boundary polynomial, so `subs(t)` is
unbounded as |t| → ∞. The landed `enclose` returns the entire box for any `tt`
extending beyond the knot range. Over-estimation is always acceptable
(BG-ENC-001); the origin union would have under-estimated. Two further
landed details: the hull endpoints are padded by `64·ε·(1+|coord|)` rather
than one ulp (Boehm insertion and the source evaluation disagree by up to
~10 ulps, measured), and the degree-0 hodograph's right-boundary evaluation
makes `hull(sub-curve) ∪ {subs(lo), subs(hi)}` the sound hull.

**2. BG-ANA-001 coaxial table: the cone rows are double-napped, twice
over.** (a) Two same-slope coaxial cones with different apexes are **not**
empty — they meet in **one circle** at the apex midpoint
(`|z − za0| = |z − za1|` has the solution `z = (za0+za1)/2`). (b) Two
different-slope coaxial cones meet in **two circles**, not "0, 1 or 2": the
between-apexes sign region always contributes one and the outside region
another. Single-nappe intuition is the error; the carrier's `v` is unbounded
both ways. The landed COAX cell implements both corrections.

**Amendment (2026-08-20, session 14: landing PARCYL and EQRCYL, adjudicating
PCONE's SPEC_GAP).** Three more, same provenance — the packets pre-decided
wrong and the workers proved it:

**3. Equal-radius cylinders: the two ellipses are not mirror images.** The
pre-decided "semi-major `r/cos(θ/2)`" is right only for the ellipse in the
**external** bisector plane (spanned by `b̂− = a0 − a1`). The ellipse in the
**internal** bisector plane (spanned by `b̂+ = a0 + a1`) has semi-major
**`r/sin(θ/2)`**. One line of algebra: a point `q + x·b̂+ + y·û` has squared
distance `x²·sin²(θ/2) + y²` to *each* axis, so the section is
`sin²(θ/2)·x² + y² = r²`. The two semi-majors coincide only at `θ = π/2` —
the Steinmetz case, which is exactly why the packet's own perpendicular
verification passed while the formula was wrong. The landed EQRCYL cell
carries the corrected pair, worker-derived and verified against the
distance-to-both-axes oracle.

**4. Plane × cone: the parabola family is not classifiable by a decisive
`Δ2 = B² − 4AC`, and the family test moves to the primary parameters.** The
2D-reduction discriminant is a multi-step polynomial in the plane normal's
components; inari rounds every intermediate outward, so at the boundary the
interval is `[−ε, 0]` or `[0, ε]` and never `[0, 0]` — `decisively_zero`
cannot fire. No witness substitution fixes it inside that rule: no
non-axis-aligned unit vector has dyadic components, so the intermediate
products are never exactly representable (attempt 1's QUESTION.md carries
the full impossibility argument and a direct inari probe:
`Δ2 = [−6.66e-16, 0.0]` for the packet's own witness). The classification
that **is** exactly decidable is the scale-free boundary invariant on the
**raw** (unnormalized) normal `N = (p − o) × (q − o)` and the carrier's own
slope `t = tan(half_angle)`, all in inari:

```text
three_way( [N.z]·[N.z],  ([t]·[t]) · ([N.x]·[N.x] + [N.y]·[N.y]) )
  Some(Greater) → ellipse family    (|n̂.z| > sin α, plane steeper)
  Some(Less)    → hyperbola family
  Some(Equal)   → parabola family    (exactly on the boundary)
  None          → refuse (NumericallyUnresolved)
```

(the equivalence `N.z² > t²(Nx²+Ny²) ⟺ |n̂.z| > sin α` is exact for
`cos α > 0`). Exact decideness needs `N.z²`, `t²` and `N.x² + N.y²`
exactly representable, which is achievable — construct plane points with
**integer** differences so the raw cross is exact, and choose a **dyadic**
slope: `tan α = 3/4` (`α = atan(3/4)`) is dyadic where
`sin α = 3/5` is not; the witness asserts `half_angle().tan() == 0.75`
(the `tan ∘ atan` round-trip holds on this host's libm and fails loudly
elsewhere). The horizontal special case and the apex-degeneracy test
`h = (p − o)·n̂` (computed in inari, component-wise) are unchanged; `Δ2`
no longer classifies anything — the 2D reduction still *emits* the
geometry (centre, axes, vertex), verified by sampling on both carriers.
For the degenerate parabola arm (`TangentLine`) the generator azimuth is a
double root and is computed by the vertex form `−b/2a`, not the
discriminant formula.

**5. Plane × cone: the in-plane horizontal axis is `normalize(n̂ × ẑ)`, not
the triple product.** `ẑ × (n̂ × ẑ)` evaluates to the horizontal *projection
of n̂* — `n̂ − (n̂·ẑ)ẑ` — which is not in the plane for any tilted plane.
The horizontal direction lying **in** the plane is `n̂ × ẑ` (up to sign);
with `v̂ = n̂ × û` the `(û, v̂)` frame spans the plane. Worker-found and
recorded as a disagreement with the derivation; the third time in this
family that a packet's formula survived review only because the worker
re-derived it.

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
