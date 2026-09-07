# Frontier math audit brief — code ↔ theorem correspondence (2026-09-07)

Purpose: a reviewer (human or model) should be able to verify, per item,
(a) **the math is correct as stated**, and (b) **the code implements exactly
that math, with every hypothesis of the theorem enforced at runtime**.
Transitively that gives code correctness on the certified paths.

Each item: the claim (verbatim where load-bearing), the implementing code,
the two verification questions, and the orchestrator's judgment. Severity is
the cost of the item being wrong.

---

## M1 — Krawczyk uniqueness for ARBITRARY preconditioner Y (HIGHEST VALUE)

**Claim (code, verbatim):** `truck-evidence/src/num/krawczyk.rs:13-19` —
"`K(Q) = m − Y·F(m) + (I − Y·J(Q))·(Q − m)` … `K ⊆ strict interior(Q) ->
Proven(unique root in Q)`". And `docs/CONTACT_FAST_PATH_BUILD_SPEC.md:238-240`
(Krawczyk Y-independence, CFP-007 basis): "proves unique root by strict
inclusion for **any** invertible Y; Y's quality affects tightness only."

**Code:** `krawczyk.rs` (the operator), `truck-certified/src/formal/
bezier_isect.rs` (2-D), `truck-certified/src/ssi.rs::krawczyk3_certificate`
(3×3, strict-inclusion-only constructor `KrawczykCertificate3::new`,
`ssi_types.rs:156-169`), and CFP-007's **caller-side stale-Y reuse**
(`construct/bie/ssi4.rs`, commit e7378e3: "Cache the last accepted float
preconditioner Y in trace_branch and reuse it for the immediately following
continuation sample").

**Q1 (is the math correct?):** Existence for arbitrary Y is solid: T(x) =
x − Y·F(x) satisfies T(Q) ⊆ K(Q) ⊆ int(Q) (componentwise interval MVT), T is
continuous, Q convex compact → Brouwer gives a fixed point (root). **Uniqueness
for arbitrary Y is not established by that argument** — the componentwise-MVT
uniqueness sketch needs a norm condition on I − Y·J(Q). The classical
references (Moore, *Interval Analysis*; Neumaier, *Interval Methods for
Systems of Equations*) state K ⊆ int(X) ⟹ unique root — verify WHICH
hypotheses their proofs use: arbitrary regular Y, or Y with
`‖I − Y·J(X)‖ < 1`, or Y an approximate inverse of J at the midpoint.

**Q2 (does the code match the math?):** Even if "any invertible Y" is a true
theorem, CFP-007's reuse is a **stale** Y — computed at the PREVIOUS sample's
midpoint. If the correct theorem requires Y ≈ J(m_current)⁻¹ (or a norm
bound against the CURRENT J(Q)), stale reuse can emit `Proven(unique)` that
no theorem backs. If the theorem is genuinely Y-arbitrary, stale reuse is
sound and the existing "bit-identical on/off" battery is beside the point.

**Orchestrator judgment:** existence is safe; uniqueness-for-arbitrary-Y is
the single most load-bearing unverified claim in the kernel — every
`KrawczykProof::Unique` emitted through the reused-Y path inherits it.
Severity if wrong: **critical** (false `Proven` verdicts = the exact
silent-wrong-answer class the kernel exists to prevent).

---

## M2 — Rank screen: per-minor containment is not simultaneous vanishing

**Claim:** Theorem 6.4 / Prop 2: rank deficiency of DF ⟺ all four maximal
minors vanish **at a point**. The §10.2/§10.4 ladder contract
(`tracer.rs:46-59`) classifies the R2 zero set from a count of sub-boxes
where "all four minors' enclosures contain zero".

**Code:** `truck-certified/src/kernel/tracer.rs:821-831` — per sub-box,
`minor_enclosures(sys, &sub_box).iter().all(contains_zero)`; buckets at
833-867 (`0 → Rebuild`, `all → TangentialCurve` ("the R2 contact zero set
is 1-dimensional"), `≤ ISOLATED_CAP → isolated_contact_is_s5a`, else
`HighOrderJet`).

**Q1 (math):** the minors ⟺ rank equivalence is fine. **Q2 (code):** each
minor's enclosure containing zero **independently** does NOT imply a common
point where all four vanish (no Helly argument) — the screen is a
NECESSARY-condition test used as SUFFICIENT. Consequences observed in the
2026-09-07 battery: a genuinely isolated node classified `HighOrderJet`, and
three isolated nodes classified `TangentialCurve` with the detail asserting
a positive geometric fact ("the zero set IS 1-dimensional") while the
refusal's backing is `Inconclusive`. Over-approximated collapse flips
bucket membership in BOTH directions — this is not conservative, it is
mislabeled.

**Fix direction already booked:** BG-KV2-207B-TRACER-REST (screen bisection
until simultaneity is decidable, or a Miranda-style common-zero test; rung-5
keyed on cluster structure). **Judgment:** this is the real math bug behind
two of the three tracer failures; the bucket labels assert facts the
decision procedure cannot support. Severity: **high** (misrouted escalation
within fail-closed refusals — wrong typed refusals, not wrong successes).

---

## M3 — Gauss-map cone certificate: the loop-freedom inference

**Claim (code, verbatim):** `truck-evidence/src/contact/gff.rs:421-426` —
"when the two carriers' gradient-direction cones over the box are disjoint …
no point of the shared zero set in the box can be tangent — the branch is
transversal by proof **and loop-free (every branch meets the box boundary;
Sinha/Sederberg)**". Verdicts: `LoopFree | TransversalByProof | AxisFixed`
(`truck-certified/src/cfp/spine.rs:346-410`). Loop-freedom underwrites
**seed completeness** ("boundary-crossing seeds are complete, closing the
gff seed-completeness question", spec §3 247-254).

**Q1 (math):** transversality alone does NOT imply branches meet the box
boundary: two spheres intersecting in a closed circle, inside a box barely
containing the circle, have (narrow, disjoint) gradient cones over that box
and a closed transverse branch strictly interior to it. The sound chain must
be: cone half-angle < π/2 on at least one carrier ⇒ h is strictly monotone
along that cone's axis (directional derivative ≥ |∇h|·cos(half-angle) > 0)
⇒ the zero set is a graph over the axis' orthogonal complement ⇒ every
component meets ∂box. **Verify whether the emitted certificate enforces
half-angle < π/2 and derives the monotonicity bound**, or whether
disjointness alone is what the code certifies on.

**Q2 (code):** `carrier_direction_cone` (gff.rs:447-455) returns `None` when
the box may contain a zero gradient — the immersion hypothesis IS enforced
there (good). The separation test (`unoriented_axis_separation`, 463-467)
is unoriented-axes with a conservative pad — but nothing visible bounds
each cone's half-angle below π/2, and `LoopFree` appears emit-able without
an `AxisFixed` graph direction. **Check `LoopFree`'s emission site for the
monotonicity/graph argument; if absent, seed completeness inherits an
unproved claim.**

**Judgment:** the transversality half is sound (Prop 2 + enforced
immersion); the loop-freedom half is, as documented, an uncited-step
assertion. Severity if wrong: **high** — a missed interior loop means
incomplete seeds ⇒ wrong fragment classification downstream (the
inverted-material class).

---

## M4 — Theorem 3 (exclusion = hull separation) for RATIONAL patches

**Claim:** `docs/CONTACT_FAST_PATH_BUILD_SPEC.md:214-219` — Theorem 3
"recovers verbatim for rational patches provided weights are certified
positive on the box (Bernstein positivity of weight coefficients)".

**Code:** `truck-certified/src/bvh.rs` (per-carrier span BVH; Theorem-3 sign
rows over "a node's net bounds"); rational carriers reach the BVH through
the CFP spline-admission layer (`patch_admit.rs`).

**Q1 (math):** the identity `0 ∉ conv{c_αβ} ⇔ conv{P¹} ∩ conv{P²} = ∅` is a
Minkowski-sum fact about POLYNOMIAL Bernstein coefficients. For rational
patches F = P/w, cross differences of F are not Bernstein coefficients of
any polynomial; "recovers verbatim" needs a homogenization argument
(positive weights ⇒ conv of homogeneous cross-differences encloses the
rational image's separation?) — **verify that argument exists and is sound,
including the direction used** (exclusion is the load-bearing direction: a
false prune removes real contact pairs).

**Q2 (code):** do the BVH sign rows for rational carriers run over the
homogeneous form with the positivity precondition enforced, or over the
rational control points directly? A false prune is silent by construction.

**Judgment:** probable gap — the phrase "recovers verbatim" is doing heavy
lifting. Severity: **high** (silent false prunes).

---

## M5 — Certified elementary functions: reduction soundness

**Claim (BG-ENC-005, `docs/GENERATION_KERNEL_BUILD_SPEC.md:1544-1565`):**
"Argument reduction is an exact identity … the choice of k cannot make the
answer wrong — only wide."

**Code:** `truck-evidence/src/elementary.rs:136-156` — k = round(x/(π/2)) in
f64; the subtraction runs in INTERVAL arithmetic: `at(x) − Interval::FRAC_PI_2
* at(k)`; wide results (> SERIES_DOMAIN) return None → caller emits [-1, 1].

**Q1 (math):** the identity holds for every integer k — correct. The
subtraction is sound **iff `Interval::FRAC_PI_2` is an outward-rounded
enclosure of π/2** (true reduced argument ∈ interval). If it is the f64
constant as a DEGENERATE interval, the true reduced argument escapes the
enclosure by k·(π/2 − f64(π/2)) — for k ~ 1e15 that is not "only wide", it
is wrong.

**Q2 (code):** verify `Interval::FRAC_PI_2`'s definition (outward
enclosure vs degenerate constant). Also verify the quadrant arithmetic
`k.rem_euclid(4.0)` is exact on f64 for |k| < 2^53 (it is — 4 is a power of
two — but the check deserves one line in the test suite), and that the
`[-1, 1]` fallback path is actually reachable (a reduction that always
"works" would hide the loss-of-precision case rather than degrade).

**Judgment:** likely sound (the interval-subtraction design is the right
one); the load-bearing check is one definition away. Severity: **medium**
(every trig-using certificate upstream).

---

## M6 — Isotopy lemma: is the one-sheet discharge ORDER-enforced?

**Claim:** `docs/FORMAL_SYSTEM_BREP_GENERATION.md` §6.2 — conditions (i)-(iii)
give "a covering of SOME degree, not a homeomorphism"; degree-one must be
discharged **after** (i)-(iii); the double-covered circle
`(R + ε·cos(t/2))·e(t)`, t ∈ [0,4π], passes (i)-(iii) and voids every
certificate above them.

**Code:** `truck-evidence/src/fid/one_sheet.rs`
(`single_sheet_circle_certifies_degree_one`), `fid/isotopy.rs`
(`single_sheet_circle_conditions_hold`).

**Q (code):** does the certificate CONSTRUCTOR enforce (i)-(iii) as
preconditions of the degree-one discharge (type-state or checked
constructor), or are they sibling tests that merely happen to pass? The
failure mode is a caller applying fibre isolation to a multi-sheet object
whose metric conditions hold. **Judgment:** the test names suggest
condition-checking exists; verify the ORDER is structural, not procedural.

---

## M7 — Margin sweeps: doctrine vs machinery

**Claim (BG-TEST-SWEEP):** "Sweep the item's margin parameter … assert the
outcome degrades monotonically … A gated item without a margin sweep is not
done. This is the directly testable statement of epistemic closure."

**Q (code):** no sweep HARNESS exists (no `margin_sweep`/`MarginSweep`
machinery in the tree) — sweeps are hand-written per packet where someone
remembered. Enumerate which landed gates actually have one vs which assert
only endpoint behavior. **Judgment:** the doctrine's "directly testable"
claim is currently tested only sporadically; either build the harness or
downgrade the doctrine's wording. Severity: **medium** (coverage illusion,
not unsoundness).

---

## M8 — Watertightness-by-index-identity: which emission paths carry the proof?

**Claim:** `constructive/mod.rs` (frozen BG-CG-000): index identity
(`I(A,E) == reverse(I(B,E))` as integers) ⇒ mesh edge-watertight by
construction; welding never invoked; winding audit failure = FAILED.

**Q (code):** the facet backend carries the proof. Enumerate ALL mesh
emission paths (boolean `assemble.rs` output, legacy `truck-meshalgo`
triangulation, stepio round-trips) and identify which run the winding audit
vs which rely on `put_together_same_attrs` (positional welding — observed in
TEST code, e.g. `triangulation.rs` upstream comparisons; confirm it is
test-only). **Judgment:** the theorem's premise ("every mesh boundary vertex
assigned by (EdgeID, ordinal)") is path-specific; any path outside the
constructive backend is only as watertight as its audit. Severity:
**medium**.

---

## M9 — Material-state decision: certified ray admissibility

**Claim:** `docs/FORMAL_SYSTEM_BREP_GENERATION.md` §12 — "Ray admissibility
certified by interval separation, not symbolic-perturbation folklore";
one certified seed per face; flip parity by contact order.

**Code:** `truck-shapeops/src/boolean/classify.rs` / fragment decision path.

**Q (code):** locate the seed-ray construction and verify the admissibility
check is an interval-separation certificate (ray provably misses all
projected boundaries within the cell), not a float direction with a
tolerance. **Judgment:** unverified — this is the inversion-class's
front door; if the seed is float-plus-tolerance, the "certified seed"
language overstates. Severity: **high** if float.

---

## M10 — Theorem 2.1 conditioning: is the code metric-normalized?

**Claim:** σ_min(DF̂) = √(1 − |n_A·n_B|) exactly, ‖DF̂‖ = √2 exactly;
"A kernel conditioned on raw DF escalates on knot placement rather than on
geometry" is named as THE defect the identity prevents.

**Q (code):** verify the escalation/conditioning thresholds in the funnel
(`ssi_trace.rs`, `escalate` rung 1, stagnation route CFP-008) consume the
crossing-angle identity (or the minor-bound Prop 2.1), not raw ‖DF‖.
**Judgment:** unverified; medium.

---

## M11 — Theorem 4's elevation trap: type-enforced or convention?

**Claim:** ∂h/∂u and ∂h/∂v "cannot be paired into 2-vectors until both are
degree-elevated to a common bidegree — only then is ∇h(x) = Σ B_α(x)(a_α,
b_α) a convex combination and the hull test valid" (packet-normative).

**Code:** `truck-evidence/src/contact/implicit2d.rs`.

**Q (code):** is the common-bidegree pairing enforced by a constructor TYPE
(elevate-then-pair, illegal to skip), or by convention inside one function?
If convention, a future caller can rebuild the invalid pairing silently.
**Judgment:** check; small fix (newtype) if conventional. Severity:
**medium**.

---

## M12 — Declared weakenings (not defects — record for the reviewer's prior)

- INV-109 wedge non-degeneracy: sampled at 3 points, "deliberately NOT a
  whole-edge interval certificate" (`truck-topology/src/invariants/wedge.rs`)
  — consumers must not treat it as whole-edge.
- STEP symbolic closure (Theorem 1): proved from finite combinatorics;
  numerical embedding, solver completeness, pcurve compatibility are
  DECLARED assumptions (§XVI) — correctly separated.
- `Pole` modulus publishes "an honest non-subadditive bound beats no bound"
  (`evidence.rs:429-431`) — by design.
- `bracket` golden drift and the two `cone_topology` tests are attributed
  separately (DEF wave, 2026-09-07).

---

## Suggested audit protocol for the reviewer

For each item in order M1 → M4 → M3 → M9 → M2 (highest severity first):
1. Restate the theorem with its hypotheses precisely.
2. Read the cited code; list every hypothesis the theorem needs and mark
   ENFORCED / NOT ENFORCED / STRENGTHENED in the code.
3. For M1: check the classical proofs (Moore ch. 5-7; Neumaier ch. 5-6;
   Rump's verification theory) for the exact hypotheses of
   `K(Q) ⊆ int(Q) ⟹ unique root`, then adjudicate CFP-007's stale-Y reuse.
4. For M3/M4: attempt the missing derivations (monotonicity/graph argument;
   rational homogenization) — if they fail, the verdict vocabulary needs a
   new arm or the emission site needs the extra precondition.
5. Produce per-item verdicts: SOUND / SOUND-WITH-MISSING-PROOF-STEP (write
   the step) / UNSOUND (book the fix packet).
