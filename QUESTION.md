# QUESTION — BG-SOL-S7-GFF-COVER (SPEC_GAP)

## The gap

The packet's certified probe — the 3×3 augmented Krawczyk system
`F(p) = [f1(p), f2(p), g·(p−m)]` with the full 3×3 Gaussian-elimination
inverse preconditioner — cannot certify **any** box of the shared zero set of
two implicit fields, because the vendored Krawczyk operator cannot certify a
coupled system at all.

## What I observed

- `num/krawczyk.rs` `k_image` computes its K image with an **entrywise**
  contraction `d[r][c] = δ(r,c) − y[r][c]·j[r][c]` (same column index on Y and
  J), i.e. the *diagonal-preconditioner* Krawczyk operator, not the matrix
  product `(I − Y·J(Q))`.
- That operator only contracts (and is only sound) for systems whose interval
  Jacobian is effectively diagonal. The packet's augmented system has a
  genuinely coupled Jacobian (rows `∇f1`, `∇f2`, `g` are not orthogonal:
  `∇f1·∇f2 ≈ −2.4` at the sphere-cylinder crossing used in the tests).
- Result: for the sphere-cylinder witness on a box centered on the certified
  crossing (width 8e-3, `f(mid) ≈ 1e-3`), the entrywise K image spans nearly
  the whole box (`K ≈ [0.533, 0.541] × …`); row 1 of `(I − Y∘J)` has
  `|d|`-row-sum ≈ 3.9 > 1, so `K ⊂ strict interior(Q)` is structurally
  impossible on any balanced box. `krawczyk` returns
  `NumericallyUnresolved` (never `Unique`) under every budget I tried
  (64 … 4096 subdivisions), and the transversal test fails with budget
  exhaustion.
- The full 3×3 inverse preconditioner the packet requires is exactly the
  configuration that breaks: I verified `invert3x3` is correct (identity,
  diagonal, and a general 3×3 known-inverse check all pass); the failure is in
  the operator's contraction, not the inverse.

For contrast, krawczyk's own 2×2 production impl (`SurfaceKnotProjection` in
`fid/rep.rs`) certifies only because a regular surface parameterization has
near-diagonal Jacobian.

## The question

How should the Contact Layer's general validated FF stage proceed?

1. **Amend the vendored krawczyk operator** (out of this packet's write set)
   to compute the matrix product `(I − Y·J(Q))` instead of the entrywise
   `(I − Y∘J)` — the documented contract ("the system supplies its own float
   inverse") and the 2×2 test (`linear_system_certifies_one_shot`) only pass
   today because of a huge start box, which hides the defect. Then this packet
   is re-runnable as written.
2. **Book the general FF stage on a different certified primitive** (e.g. a
   bisection/exclusion-only curve-pointing scheme, or a Newton-continuation
   arc following `t = ∇f1 × ∇f2`) and drop the 3×3 augmented-Krawczyk
   formulation from the plan.
3. Keep `cover_branch` as the decomposition skeleton and re-point the probe at
   whatever certified primitive option 1 or 2 lands.

The engine itself (`cover_branch`, `AugmentedFF`, `invert3x3`, singular
screen, interval-exclusion pruning, widest-axis bisection, unresolved
remainder) is implemented and the non-probe paths are verified (3 of the 4
required tests pass).
