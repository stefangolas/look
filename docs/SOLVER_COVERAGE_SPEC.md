# SOLVER COVERAGE SPEC — the semantic proof-state audit (spine, 2026-09-10)

Owner-approved program (frontier proposal, orchestrator-cut): build a
symbolic proof-state map of the kernel's solver coverage, so capability work
is driven by known semantic gaps instead of by whichever corpus model fails
next. This document is the wave spine: the frozen contracts, the wave
manifest, and the integration order. The central claim to demonstrate:

    we can identify a realizable mathematical solver gap from the symbolic
    proof-state space BEFORE a normal corpus model happens to expose it.

## 1. The frozen contract — RULES schema (the shim)

Every extraction packet emits RULE fragments conforming to EXACTLY this
shape (loop/solver_coverage/schema.json is normative; packets cite it):

```json
{
  "rule_id": "SSI4-KRAWCZYK-UNIQUENESS",
  "kind": "solver|reduction|classifier|gate|assembler",
  "source": {"file": "vendor/truck/truck-certified/src/construct/bie/ssi4.rs",
             "symbol": "<exact path>::<fn>", "line": 1},
  "preconditions": [{"axis": "<axis-id>", "value": "<axis-value-or-residual>"}],
  "outcomes": [{"variant": "<exact enum/type variant>", "postconditions": [{"axis": "...", "value": "..."}]}],
  "refusals": ["<exact Refusal variant or named fail-closed case>"],
  "consumes_evidence": ["<exact type names>"],
  "produces_evidence": ["<exact type names>"],
  "calls": ["<rule_id or exact symbol paths>"],
  "theorem_comment": "<verbatim excerpt of the theorem/proof-obligation comment>",
  "confidence": "high|medium|low"
}
```

Rules are extracted from CODE FACTS (signatures, enums, match arms, refusal
constructions, call edges) + theorem comments. `confidence: low` rows are
expected and reviewed, never silently dropped. **The schema is open-world at
the VALUE level**: a precondition/outcome value that does not fit a known
axis value is recorded as `"value": "!unmodeled:<verbatim>"` — that is a
first-class finding (the abstraction-gap signal), not a schema violation.

## 2. The frozen core semantic axes (v1, deliberately coarse)

Axes are independent capability/knowledge facts, NOT a closed enum of cases.
Residual parents stay unnamed until a refinement theorem justifies a split.

| axis | values (v1) | notes |
|---|---|---|
| `relation.src_dims` | `2x2`, `2x1`, `1x1`, `!unmodeled` | surface-surface, surface-curve, curve-curve |
| `relation.zero_set` | `certified_empty`, `regular`, `rank_deficient(residual)`, `unknown` | rank-deficient stays a RESIDUAL parent (tangency, coincidence, higher-order contact all live inside it until a theorem splits them) |
| `relation.local_dim` | `0`, `1`, `2`, `residual` | certified only where a theorem proves it |
| `relation.incidence` | `domain_interior(residual)`, `seam`, `knot_span_boundary`, `domain_edge` | only where theorems distinguish |
| `rep.bernstein_chart` | flag | exact Bernstein representation available |
| `rep.rational_positive_weights` | flag | rational + positive-weight evidence |
| `rep.exact_implicit` | flag | exact implicit predicate available (pullback route) |
| `rep.canonical_carrier` | flag | canonical analytic carrier witness |
| `rep.construction_witness` | flag | constructive provenance |
| `rep.param_map_inverse` | flag | canonical parameter map |
| `global.knowledge` | `local_only(residual)`, `loop_free`, `seed_complete`, `all_components`, `complete_locus` | proof-STRENGTH ordered, open-world; failure to prove is not disproof |
| `goal` | `no_intersection`, `local_contact`, `complete_locus`, `material_class`, `valid_brep`, `volume_bracket`, `mesh` | the caller's request is part of the state |

Background theory `T_geom` (v1, minimal): rank DF=3 over a `2x2` relation
implies local contact dimension 1; canonical carrier implies exact-implicit;
facts not derivable stay in residual parents. Feasibility of symbolic states
is checked against `T_geom` only — an infeasible cell is excluded from
coverage arithmetic, never deleted from the vocabulary.

## 3. Coverage semantics (the checker's contract)

- Solver rules are theorem transformers: `R: P_R -> {Q_1..Q_k}` where every
  outcome must carry its postcondition. Soundness = emitted Q follows from
  established evidence; outcome completeness = every admitted input ends in
  a declared outcome or a modeled fail-closed result; progress = strengthen
  knowledge, reduce the problem, or decrease a well-founded measure.
- `W_G` per goal: AND-OR fixed point. Start = states whose postconditions
  already prove G + intentional scope refusals (marked `OUTSIDE_ENVELOPE`,
  which is coverage-negative but honest). Add states from which some rule
  has ALL semantic successors in W_G. `M_G = A_G \ W_G` is the missing
  region, reported by satisfiable symbolic description.
- **Gap classes, explicitly distinguished**: THEORY GAP (no rule),
  VERTICAL GAP (chain stalls below the requested guarantee — e.g.
  `local_contact` proved, `complete_locus` unreachable),
  ROUTING GAP (rule exists, dispatch never reaches it: W_theory vs W_code
  from the bridge/door survey), ABSTRACTION GAP (`!unmodeled` values
  encountered in code), PERFORMANCE (covered only by an expensive generic
  route). Safety closure (`NumericallyUnresolved`/budget exhaustion) is
  NEVER coverage.
- Report shape: the frontier proposal's section 18 example, with the
  repository's vocabulary.

## 4. Wave manifest

| packet | scope (read) | writes | depends |
|---|---|---|---|
| SOLVER-SURVEY-A | kernel surface-relation solvers: `tangency/*`, `ssi*.rs`, `ssi_trace`, `selfint.rs`, `pair_dispatch.rs`, `kernel/contact.rs`, `kernel/canal.rs` | loop/solver_coverage/fragments/A.json | - |
| SOLVER-SURVEY-B | certified construction + admission: `construct/admission.rs`, `construct/bie/*`, `construct/contact3.rs`, `construct/blend*.rs`, `tangency/gates.rs` + `tangency/tsystem.rs` | loop/solver_coverage/fragments/B.json | - |
| SOLVER-SURVEY-C | trim/assemble/fragment stage: `kernel/trimclip.rs`, `kernel/assemble.rs`, `kernel/atlas.rs`, `kernel/sheet.rs`, `kernel/engine.rs`, `kernel/residuals_r89.rs`, `kernel/promote.rs` | loop/solver_coverage/fragments/C.json | - |
| SOLVER-SURVEY-D | routing layer (W_code): `truck123d/src/bd_bridge.rs`, `truck123d/src/binding.rs`, `truck123d/src/facade.rs`, `corpus/ttc/door.py` dispatch, `docs/OP_CAPABILITY_MATRIX.md` cross-ref | loop/solver_coverage/fragments/D.json | - |
| SOLVER-CHECKER | merge fragments, dedupe vocabulary, compute W_G for all 7 goals, emit the audit report | `loop/solver_coverage/` + `docs/SOLVER_COVERAGE_AUDIT.md` | A,B,C,D |

All survey packets are `class: survey` (read-only; no kernel writes;
SURVEY-shaped deliverable validated at filing). The checker is `mechanical`
and includes the two demonstrations: (a) RETRODICTION — the audit must
report the partial-coincidence/tangency theory gap under
`cut(swept,swept)`-class goals that the loop historically discovered
reactively; (b) DELIBERATE GAP — checker run with one rule table entry
removed must name the exact orphaned cell, restore clears it.

## 5. Integration notes

- Vocabulary collisions across fragments resolve at the checker: same rule
  extracted twice merges by source symbol; conflicting postcondition claims
  are reported as conflicts for orchestrator adjudication (never silently
  merged).
- The capability matrix (docs/OP_CAPABILITY_MATRIX.md) remains the coarse
  client layer; the checker's lattice refines it and cross-references it —
  no second ontology at the client level.
- Agentic interpretation is confined to theorem-comment translation within
  survey packets; every interpreted claim carries a verbatim excerpt + a
  confidence, and the checker flags low-confidence rules feeding a winning
  route.
