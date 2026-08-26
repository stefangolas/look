# QUESTION — BG-SOL-S7-GFF-COVER (SPEC_GAP, second attempt)

## The gap

The packet's certified probe — the 2×2 z-slab Krawczyk system
`F(x,y) = [f1(x,y,z0), f2(x,y,z0)]` with the exact 2×2 inverse preconditioner —
cannot certify **any** crossing of the transversal sphere/cylinder witness,
even on a tiny box centered on the exact crossing. The r2 amendment's premise
that the vendored Krawczyk operator is a full-matrix operator that will
contract the 2×2 slab is factually wrong.

(Note: anchor A3 is also stale — it pins zero `gff` matches in `contact/mod.rs`,
but the committed r1 module this amendment instructs converting added
`pub mod gff;`, so A3 returns 1 and cannot pass while the conversion exists.)

## What I observed

**The operator is entrywise, not full-matrix.** `num/krawczyk.rs` `k_image`
(162) computes `d[r][c] = δ(r,c) − y[r][c]·j[r][c]` — the *same* column index
`c` on both the preconditioner row and the Jacobian row. That is the
diagonal-preconditioner Krawczyk: it certifies exactly when the entrywise
`(I − Y∘J)` is small, i.e. only for effectively diagonal Jacobians. It is NOT
`(I − Y·J)` (the matrix product `δ − Σ_k y[r][k]·j[k][c]`).

**Control that isolates the operator:** the krawczyk module's own coupled 2×2
linear witness (Lin2, full-matrix inverse) certifies one-shot on a small box
around its root — but only because its entrywise `(I − Y∘J)` row sums are
`0.4 < 1`. I verified this directly.

**The 2×2 slab system cannot certify the witness.** Re-derived determinant:
`J = [[2x, 2y], [2(x−cx), 2(y−cy)]]`, `det = 4(y·cx − x·cy) = 12y` for the
witness — nonsingular off the singular locus, exactly as the packet states. But
at any crossing `(x,y)` of the curve, the entrywise `(I − Y∘J)` row-1 |d|-sum is
`(x−3)²/(3y) + |1 − x/3| ≥ 4/3 + 2/3 > 1` (since `x ∈ [1/6,1]` and `|y| ≤ 1` on
the cylinder), so `K ⊂ strict interior(Q)` is unreachable on any axis-aligned
box, regardless of position, shape, or budget. Measured: a 0.02-wide box
centered on the exact crossing (f residuals ~1e-16) still returns
`NumericallyUnresolved` after 223 subdivisions; the full cover exhausts a 4096
budget.

## The question

How should the Contact Layer's general validated FF stage proceed?

1. **Fix the vendored krawczyk operator** (out of this packet's write set) to
   compute the matrix product `(I − Y·J(Q))` instead of the entrywise
   `(I − Y∘J)`. The current operator certifies the krawczyk module's own
   `linear_system_certifies_one_shot` only because its start box is huge
   ([−10,10]); a coupled 2×2 linear system on a small box would fail today.
   With the matrix-product operator, the 2×2 z-slab probe here certifies, and
   this packet is re-runnable as written.
2. **Book the general FF stage on a different certified primitive** — e.g. a
   coordinate-wise / Gauss–Seidel interval operator over the *diagonalized*
   2×2 slab, or a pure bisection-plus-exclusion scheme, or a Newton-continuation
   arc following `t = ∇f1 × ∇f2`.
3. Keep `cover_branch` as the decomposition skeleton and re-point the probe at
   whatever certified primitive option 1 or 2 lands.

The engine (`cover_branch`, `SlabFF` 2×2 system, exact 2×2 inverse
preconditioner, determinant singular screen, interval-exclusion pruning,
nested (x,y)/z bisection, unresolved remainder) is implemented and the
non-probe paths are verified (3 of the 4 required tests pass). Both the r1
(3×3 augmented) and r2 (2×2 z-slab) formulations fail for the same root cause:
the vendored krawczyk operator cannot contract genuinely coupled systems.
