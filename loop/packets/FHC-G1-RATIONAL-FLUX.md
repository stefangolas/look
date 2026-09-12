# WORK PACKET FHC-G1-RATIONAL-FLUX — rational-weight patch flux through the certified boolean funnel

Gap-register category 1 (`docs/F1_HYPERCAR_GAP_REGISTER.md`): the certified
boolean funnel consumes unit-weight patches only; every rotational boundary
(cylinder/cone/sphere/revolve) is a rational patch with `w(u,v) ≠ const`, so
canonical CYLINDER+CYLINDER unions (the F1 fasteners, the suspension bores)
refuse `unsupported_envelope` at admission. This packet lands the
rational-flux path. The theory below is the owner-reviewed statement
(2026-09-12); it is NORMATIVE — the worker implements it, it does not
relitigate it. `cell_flux_exact` stays untouched as the polynomial fast path.

```yaml
id:          FHC-G1-RATIONAL-FLUX
contract:    [FHC-G1-RATIONAL-FLUX]
class:       design
crates:      [truck123d]
depends_on:  [FHC-G6-CERT-COST-SCALE]
needs:       [FHC-G6-CERT-COST-SCALE]
write_allow:
  - truck123d/src/bd_bridge.rs
  - truck123d/src/facade.rs
  - truck123d/tests/rational_flux.rs
read_allow:
  - docs/F1_HYPERCAR_GAP_REGISTER.md
  - docs/MONO_CLOSURE_BOOKING.md
tests_required: [truck123d/tests/rational_flux.rs]
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'fn cell_flux_exact' truck123d/src/bd_bridge.rs"}
  - {id: A2, expect: 4, cmd: "grep -c 'boolean_product_volume_certified' truck123d/src/bd_bridge.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'fn placed_box_patches' truck123d/src/bd_bridge.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'fn node_box_patches' truck123d/src/bd_bridge.rs"}
budget:      {turns: 90, ctx_tokens: 300000}
```

Anchors measured 2026-09-12 at the EX-B-landed HEAD. **Re-measure at
dispatch** — the docket ahead WILL drift them; update at dispatch, never
dispatch stale.

## Normative theory (owner statement, 2026-09-12)

**T1 (rational flux lemma).** For an oriented rational patch
`S(u,v) = P(u,v)/W(u,v)` on a cell D with certified `W > 0`, and the
symmetric divergence field `F_a(x) = (x−a)/3` with `∇·F_a = 1`:

```
F_a(S)·(S_u × S_v) = (1/3)·det(Q, Q_u, Q_v)/W³,     Q = P − aW
```

Proof: `S−a = Q/W`, `S_u = (WQ_u − QW_u)/W²`, `S_v = (WQ_v − QW_v)/W²`;
the determinant is multilinear, every term with Q twice vanishes, leaving
`W²·det(Q,Q_u,Q_v)/W⁵`. The symmetric field is LOAD-BEARING: an `(x,0,0)`-style
field leaves mixed `W⁻²/W⁻³` terms and no single-reciprocal interface.
Per-face integrands for different anchors differ; the flux identity holds
only summed over a CLOSED cell — **the anchor `a` must be common to the
entire closed cell** (use the cell box center; it shrinks Q and tightens the
Bernstein L¹ bound).

**T2 (L5 closure).** If the certified reciprocal (L5/Theorem D, already
landed) returns on a leaf K: `W⁻³ = R_K + E_K`, `R_K` polynomial,
`|E_K| ≤ ε_K`, then with `N = det(Q,Q_u,Q_v)` in Bernstein form:

```
flux over K  ∈  (σ/3)·[A_K − e_K, A_K + e_K]
A_K = ∫_K N·R_K   (exact, polynomial machinery — the cell_flux_exact path)
e_K = ε_K · (|K|/((m+1)(n+1))) · Σ|n_ij|   (Bernstein positivity bound)
```

**T3 (completeness).** With `W ≥ η > 0` on D and L5's `max_K ε_K → 0` under
refinement, `Σ_K e_K → 0` (the leaves partition D), so the accumulated
enclosure converges to the true flux; every MONO-5 decision with positive
margin certifies after finite refinement. Exhausted budget ⇒ refuse
`rational_flux_inconclusive`, never `unsupported_envelope`.

**T4 (admission, MONO-8 amendment).** The unit-weight test stops being the
admission criterion. Admit when `W` is constant OR `W` carries a certified
positive lower bound (for positive-weight Bernstein patches:
`W ≥ min w_ij > 0`, free) and L5 supports the required reciprocal power.
De Casteljau subdivision preserves positivity (convex combinations).

**T5 (homogeneous operations, exact).** Affine placement:
`(P,W) ↦ (AP + bW, W)`. Parameter subdivision: subdivide all four components
`(X,Y,Z,W)` together. Neither dehomogenizes nor approximates. Gauge
normalization `(P,W) ∼ (λP, λW)` is flux-invariant (det scales λ³, W³ scales
λ³); exact power-of-two normalization is permitted for conditioning.
Subdivision for cylinder patches prefers the angular parameter (W varies
only there).

**T6 (cylinder-pair corollary — the landing theorem).** A contact-cover cell
whose faces are polynomial patches and restrictions of regular rational
cylinder patches, each with a certified `W > 0` witness and the landed
MONO/contact hypotheses, gets a certified flux enclosure from the new path.
Nonconstant rational cylinder weights are NOT grounds for
`unsupported_envelope`. Cones, spheres, and positive-weight rational
revolves are covered identically. Per-face flux needs only that face's own
`W⁻³`; composite reciprocals are NOT load-bearing for this corollary.

**Precondition check at dispatch:** if any path consumes COMPOSITE
reciprocals, the `Modulus::compose` bounds fix (BG-EVD-004-r2) must be
landed first — verify, or stay on the per-face single-`W` path.

## Architecture judgements

1. **Do not mutate `cell_flux_exact`.** Add `cell_flux_certified` returning
   `FluxCert::Exact(Exact) | FluxCert::Enclosure { lo, hi }` (exact
   rational/dyadic endpoints matching the certificate algebra). The old
   polynomial solver becomes the `W = const` special case of the new
   certificate algebra.
2. **MONO-5 consumes enclosures by separation** (`lo > threshold`,
   `hi < threshold`; overlap ⇒ refine; budget ⇒ `rational_flux_inconclusive`).
3. **MONO-9 folds intervals** — Minkowski addition `[l1,h1]+[l2,h2]`; exact
   endpoints keep it associative and monotone; `Exact(x)` is `[x,x]`.
4. **Placement/subdivision stay homogeneous** — `placed_*` must preserve
   `(P,W)` per T5 (the current box-only path gains the homogeneous patch
   arm).

## Acceptance suite (each a named `#[test]` in `rational_flux.rs`)

1. `unit_weight_cell_equivalence` — rational path with `W = 1` equals
   `cell_flux_exact` **summed over a closed cell** (per-face integrands
   differ by the divergence-field choice; per-face equality is NOT
   asserted).
2. `constant_weight_gauge` — `(P,W)` and `(λP, λW)` produce identical
   certified flux.
3. `polynomial_oracle` — polynomial patches as homogeneous patches with
   constant `W` collapse to the old exact result.
4. `placement_restriction_homogeneous` — homogeneous transform +
   subdivision agrees with direct evaluation.
5. `positive_cylinder_admitted` — a standard rational cylinder patch no
   longer returns `unsupported_envelope`.
6. `weight_zero_refines_or_refuses` — a cell whose positivity cannot be
   certified refines or refuses `rational_flux_inconclusive` — never enters
   the reciprocal kernel.
7. `fold_partition_contains_whole` — partitioning a face and summing child
   certificates contains the unpartitioned face's actual integral.
8. `anchor_uniformity_enforced` — mixed anchors within one closed cell are
   prevented by construction (compile-time or typed refusal).
9. `end_to_end_cylinder_union` — at least one canonical CYLINDER+CYLINDER
   fastener union passes the entire `boolean_product_volume_certified`
   route (MONO-8 → flux → MONO-5 → MONO-9 connected end to end).

## Method

T1/T2 as `rational_patch_flux(patch, domain, anchor, budget)` over the
homogeneous restriction; wire T4 admission; T3's refinement loop with
budget; then the suite in order 1–8 before 9. Record before/after row
verdicts (`f1/nose`, `f1/sidepod_*`, `f1/airbox`, `f1/details`,
`f1/drivetrain` spot-checks) in RESULT.

## Done when

```
cargo fmt --check -p truck123d
cargo clippy -p truck123d --all-targets -- -D warnings
cargo test -p truck123d --test rational_flux --locked
```

plus the end-to-end door spot-check (test 9's row through
`corpus/ttc/door.py --engine truck`). All cargo through the queue (the
`cargo` on PATH IS the queue shim); interpreter dir on PATH if
STATUS_DLL_NOT_FOUND.

## Forbidden

Mutating `cell_flux_exact`'s contract; dehomogenization or approximation
anywhere in the new path; editing `corpus/ttc/trees/**` or `vendor/truck/**`
without an orchestrator-approved write-set amendment; any file outside
`write_allow`; `#[ignore]`, deleted or weakened tests, bare `cargo test`;
committing to `main`.

## Stop conditions

- T2's enclosure cannot be made sound over the landed certificate algebra → `SPEC_GAP` naming the exact gap (that is the register's evidence, not a failure)
- an admission widening exposes a pair the contact-cover hypotheses do not cover → typed refusal naming the pair class + `SPEC_GAP`
- anchor count differs at dispatch and was not re-measured → `ANCHOR_MISMATCH`
- three consecutive failed cargo runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` AT THE WORKTREE ROOT

```json
{"id":"FHC-G1-RATIONAL-FLUX","status":"DONE","contracts":["FHC-G1-RATIONAL-FLUX"],
 "anchors_verified":{"A1":1,"A2":4,"A3":1,"A4":1},
 "rows_flipped":[],"rows_still_refused":[],
 "notes":"suite 1-9 verdicts; end-to-end union record; before/after row verdicts"}
```

Commit subject: `truck123d: rational-weight patch flux through the certified funnel (FHC-G1)`.
