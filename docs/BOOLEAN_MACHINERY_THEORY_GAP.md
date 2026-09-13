# Boolean machinery — formal theory of the missing pieces, and an API-gap filling estimate

2026-09-13, orchestrator session. Companion to `docs/F1_HYPERCAR_GAP_REGISTER.md`
(which tracks *rows*; this file states the *machinery* theory behind its
category 1 and names what G1 does not close). The normative source for the
in-flight theory is `loop/packets/FHC-G1-RATIONAL-FLUX.md` (owner statement
2026-09-12, T1-T6); this document summarizes it, situates it in the landed
funnel, and enumerates formally what remains outside it.

---

## 1. The landed funnel (what exists, precisely)

Certified boolean volume evaluation is a five-stage funnel in
`truck123d/src/bd_bridge.rs`:

```
BooleanNode
  → [admission]        MONO-8 amendment: operand surfaces must be
                       polynomial patches with W(u,v) = const
  → [extraction]       node_box_patches → placed_box_patches: per-cell
                       face restrictions of the operand boundaries
  → [membership]       MONO-5 ray-classify: inside/outside decisions
                       with positive margins per sample
  → [per-cell flux]    cell_flux_exact: ∫∫_face F_a·(S_u × S_v) dudv
                       over each cell face, F_a(x) = (x−a)/3, ∇·F_a = 1,
                       summed over the closed cell gives the volume
                       contribution exactly (polynomial integrand,
                       Bernstein machinery, exact rational/dyadic result)
  → [fold]             MONO-9: same-mode operand chains fold; mixed-mode
                       nesting evaluates pairwise; each certified leaf
                       contributes its extracted patches
```

Structural facts that matter for everything below:

- **Leaf evaluation is pairwise.** `boolean_product_volume_certified` takes a
  binary node. N-ary evaluation exists only as the MONO-9 chained fold over
  same-mode runs (`fold_operands`/`fold_collect`, `is_fold_chain`), which
  sounds because each operand contributes *extracted patch restrictions*, not
  because cells of an N-way intersection are ever formed.
- **No simultaneous multi-operand cell topology exists.** There is no N-way
  intersection cell decomposition, and hence no native representation of
  non-manifold configurations.
- **Every refusal is typed** (v2 refusal records, `docs/REFUSALS.md`), and the
  `__getattr__` catch-all (commit `3e0494e`) makes the name-axis total: the
  surface cannot fail silently. The enumeration in §4 is complete for that
  reason, not by audit diligence alone.
- **Determinism is absolute**: fixed reduction order, no parallelism in the
  certified path. Any machinery below must preserve this.

## 2. Operand admission — the wall, formally

**Current criterion (MONO-8).** A boundary patch is admissible iff it is a
polynomial patch in Bernstein form with constant weight `W(u,v) ≡ w`. The
per-cell flux integrand for the divergence field `F_a(x) = (x−a)/3` over a
patch `S(u,v)` is `F_a(S)·(S_u × S_v)`; for a *rational* patch
`S = P/W` this contains ratios of `W` at mixed powers (`W⁻²`, `W⁻³` terms),
and the landed polynomial exact-integration machinery does not bound it.
Hence every rotational boundary — cylinder, cone, sphere, revolve, any lathe
surface — is a *rational* patch with `W(u,v) ≢ const`, and a boolean whose
operand carries one refuses typed `unsupported_envelope` **at admission,
before any numerics run**. This is the register's category 1: it is an
admissibility wall, not a solver-accuracy wall (RDEF-M2 fired zero
NonTransversalContact/BudgetExhausted refusals corpus-wide — admitted pairs
are numerically sound).

**What G1 lands (normative, in flight).** The rational-flux path
(`cell_flux_certified`, T1-T6 of the G1 packet):

- **T1 (rational flux lemma).** With the *symmetric* field `F_a(x) = (x−a)/3`
  and `Q = P − aW`, the flux integrand collapses to a single-reciprocal form:
  `F_a(S)·(S_u × S_v) = (1/3)·det(Q, Q_u, Q_v)/W³`. The symmetric field is
  load-bearing — an asymmetric field leaves mixed `W` powers with no single-
  reciprocal interface. The anchor `a` must be common to the entire closed
  cell (cell box center).
- **T2 (L5 closure).** The landed certified reciprocal-power lemma (ADM L5 /
  Theorem D) gives `W⁻³ = R_K + E_K` per leaf `K` with `R_K` polynomial and
  `|E_K| ≤ ε_K`; the flux over `K` is then enclosed by
  `(σ/3)·[A_K − e_K, A_K + e_K]` with `A_K = ∫_K N·R_K` exact (the old
  polynomial path, reused verbatim) and `e_K` an explicit Bernstein
  positivity bound.
- **T3 (completeness).** `W ≥ η > 0` plus L5's `max_K ε_K → 0` under
  refinement gives `Σ_K e_K → 0`: the enclosure converges, every MONO-5
  decision with positive margin certifies after finite refinement, and
  budget exhaustion refuses `rational_flux_inconclusive` — never
  `unsupported_envelope`. Refusal vocabulary shifts from "class not
  admitted" to "certificate inconclusive within budget".
- **T4 (admission amendment).** Admission stops being `W = const` and becomes:
  `W` constant **or** `W` carries a certified positive lower bound (free for
  positive-weight Bernstein patches: `W ≥ min w_ij > 0`) and L5 supports the
  required reciprocal power.
- **T5 (homogeneous operations).** Affine placement `(P,W) ↦ (AP + bW, W)`
  and subdivision of `(X,Y,Z,W)` together are exact; no dehomogenization or
  approximation anywhere in the new path. Gauge invariance `(P,W) ∼ (λP, λW)`
  is flux-invariant.
- **T6 (cylinder-pair corollary).** Contact-cover cells whose faces are
  polynomial or regular rational cylinder restrictions get certified
  enclosures; cones, spheres, and positive-weight rational revolves are
  covered identically. Composite reciprocals are explicitly NOT load-bearing
  (precondition: `Modulus::compose` bounds fix BG-EVD-004-r2 if ever
  consumed).

Acceptance is a 9-test suite ending in the end-to-end CYLINDER+CYLINDER
fastener union through the full funnel.

**What G1 does NOT close** (the honest residual after the in-flight packet):

1. **Trimmed and interior-loop surfaces.** Admission covers positive-weight
   rational *patches*; a boundary with interior trim loops (holes in a face,
   trimmed NURBS) still has no carrier that expresses the restriction as
   patches with the contact-cover hypotheses. This is the same class the
   fan-cap packet (FHC-E) touches at its degenerate boundary.
2. **Approximated / procedural surfaces.** Offset surfaces (genuinely
   rational-plus-trim, and typically non-rational for freeform bases), blend
   (fillet) surfaces as boolean *operands*, and any mesh-derived boundary
   have no admission story at all. A mesh operand is not refused-typed so
   much as out of scope: the certified path has no container for it.
3. **Composite reciprocals.** T2 is per-face single-`W`. If a future cell
   needs `1/(W1·W2)` class integrands (mixed-weight face restrictions), the
   `Modulus::compose` bounds work (BG-EVD-004-r2) must land first; G1
   deliberately stays on the per-face path.
4. **Simultaneous N-ary cells and non-manifold topology.** The MONO-9 fold
   gives *volume* and *membership* certificates for chains; it does not
   construct the intersection complex. Mixed-mode deep nesting evaluates
   pairwise. Consequences: (a) configurations where three or more boundaries
   are mutually tangent or near-tangent at a common point are certified only
   through the pairwise telescoping, with no topology-level witness; (b)
   non-manifold outputs (two solids touching at a face, compound booleans
   producing open shells) are outside the certificate's vocabulary entirely —
   they refuse rather than mis-certify, which is the right failure direction,
   but there is no positive capability.
5. **Query-side boolean surfaces.** MONO-5 membership is internal. There is
   no exposed classify/section/interference/distance query over the certified
   representation; clients that need point-membership answers get them only
   implicitly through volume facts.

## 3. Soundness envelope (what the certificates guarantee)

For every admitted pair (after G1: every admitted pair of polynomial or
positive-weight-rational-patch boundaries): the emitted volume is a bracket
`[lo, hi]` with exact rational/dyadic endpoints, the true volume lies inside,
`solid_count` is constructive, bbox is carrier-derived, and mesh emission
preserves the same carrier. Outside that envelope: typed refusal with v2
metadata (refusal_code/typed/verb/carrier/phase/client_site/known_gap). There
is no configuration that produces an uncertified numeric answer — the failure
mode is refusal, never silent approximation. This invariant is what makes the
API-gap list below safe to fill incrementally: a missing carrier is a visible
typed error, never wrong geometry.

## 4. API-level gap inventory and work estimate

Calibration from the FHC chain: a mechanical packet (one carrier class +
tests + door wiring) costs ~0.5-1 loop-day (worker 1-3 h + adjudication); a
mechanical+ packet with a closed form to derive costs 1-2; a design packet
with normative theory (G1-class) costs 2-4. Estimates assume the packet loop
at the current RAM cap (one worker, occasionally two) and are **provisional
until the item's refusal is surfaced** — the register's classification rule
applies: nothing is "mechanical" until its refusal says so.

### Tier 1 — booked, in flight (the docket)

| item | class | est. remaining |
|---|---|---|
| G6 certified-cost decomposition + whitelisted fixes | mechanical+ | running |
| G1 rational flux (§2, T1-T6) | design | 2-4 loop-days |
| FHC-D surface residue (4 names + 6 probes + vertices) | mechanical | 0.5-1 |
| FHC-E planar fan cap | mechanical+ | 1-2 |
| Census R4 (40 rows, table + register update) | harness | 0.5-1 |

Subtotal: **~5-9 loop-days** (mostly already booked).

### Tier 2 — named but carrier-less

| item | class | est. |
|---|---|---|
| `chamfer` executor arm (closed-form chamfer flux through the certified path; the fillet-arm precedent) | mechanical+ | 1-2 |

### Tier 3 — absent from the surface entirely

*Constructive:*

| item | class | est. |
|---|---|---|
| General fillet: variable radius, edge chains, blend-surface generation (not just the corpus's blend facts path) | design | 3-6 |
| Multi-section sweep / sweep-with-voids / pipe | mechanical+ | 2-4 |
| Standalone shell / thicken / offset (offset is theory-adjacent — rational + trim; rides the G1 substrate) | design | 2-5 |
| Draft | mechanical+ | 1-2 |
| Text / emboss (font-to-curve chain + extrude; render-side alternative is cheaper) | mechanical+ | 2-3 |
| Sketch-level 2D booleans on faces/wires | mechanical | 1-2 |
| Polar / linear pattern ops (over the landed `compound_from_instances` placement) | mechanical | 0.5-1 |
| Helix-as-solid (threads) | mechanical+ | 2-3 |
| N-ary/mixed-mode boolean generalization beyond the fold (see §2.4) | design | 2-5 |

*Query:*

| item | class | est. |
|---|---|---|
| Area / center-of-mass / inertia via the flux machinery — nearly free once G1 lands: `∇·(x_i·x/4) = 4x_i` gives first moments, `∇·(x_i·x_j·x/5) = 5·x_i·x_j` gives second moments; same polynomial/rational integrator, three and six more divergence fields | mechanical | 1-2 |
| Exposed classify / section / interference / distance over the certified representation | mechanical+ | 2-4 |
| Edge/face/vertex iteration completion over the G3 data-row attributes | mechanical | 1-2 |

*I/O:*

| item | class | est. |
|---|---|---|
| STEP write (read exists in `truck-stepio`; a writer is a vendor-packet lift, AP203/AP214 serialization) | vendor | 5-10 |
| SVG / 3MF / OBJ write | mechanical | 1-2 |
| Import symmetry on the door side (mesh/brep in) | mechanical+ | 2-3 |

### Totals

- **Corpus-shaped tail** (everything the current 40-row corpus can plausibly
  call, excluding STEP write and general fillet): **~15-22 loop-days**
  beyond the current docket.
- **Full CAD-parity surface** including STEP write, general fillet, and the
  boolean generalizations: **~25-40 loop-days**.
- **Excluded as open-ended** (not bookable under the current certificate
  vocabulary): non-manifold positive capability, blend-surface surgery,
  arbitrary trimmed-NURBS boolean operands. These refuse typed today and
  should keep doing so until a theory packet names their certificate.

The re-scoping gate is census R4: it re-counts all 40 rows under the landed
machinery and converts "plausibly callable" into "actually called", which is
the only honest input to which Tier 3 rows get booked. The register's
one-sentence reading stays in force: no open blocker is a solver-numerics
problem — everything above is admission, wiring, or a named proof obligation,
and each lands through the packet loop with the refusal surface proving the
enumeration stayed total.
