# Deliberate-gap demonstration (SOLVER-CHECKER)

Demonstration (b) of `docs/SOLVER_COVERAGE_SPEC.md` section 4: run the checker with the merged rule table minus exactly one rule and name the orphaned semantic cell(s).

The packet's suggested rule `SSI4-KRAWCZYK-UNIQUENESS` was measured first: removing it orphans **0** states under the v1 lattice. It is a solver row whose precondition state already satisfies the `local_contact` goal predicate (a transversal `regular` zero set with `local_dim=1`), so it is not load-bearing for coverage; the checker therefore selects the most load-bearing row below.

**Removed rule:** `C-CLASSIFY-DISCHARGE-COMPLETENESS` (a genuinely load-bearing row).

Total orphaned states across all goals: **27552**.

| goal | orphaned states | exact orphaned cell(s) |
|---|---:|---|
| `complete_locus` | 27552 | (2x2, rank_deficient(residual), 0, domain_interior(residual), local_only(residual)); (2x2, rank_deficient(residual), 0, domain_interior(residual), loop_free); (2x2, rank_deficient(residual), 0, domain_interior(residual), seed_complete); (2x2, rank_deficient(residual), 0, domain_interior(residual), all_components); (2x2, rank_deficient(residual), 0, seam, local_only(residual)); (2x2, rank_deficient(residual), 0, seam, loop_free) |

Restoring the rule clears the gap (the default audit in `docs/SOLVER_COVERAGE_AUDIT.md` is the restored run).
