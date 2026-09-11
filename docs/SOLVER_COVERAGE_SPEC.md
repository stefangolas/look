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

## 6. RDEF-M0 checker accounting (CHK-1 to CHK-4)

Milestone M0 of `docs/RANK_DEFICIENT_CONTACT_SPEC.md` (packet
`RDEF-M0-CHECKER-ACCOUNTING`, owner proxy 2026-09-10). The checker now reports
the accounting of section 2 of that spec. The three confirmed modelling
assumptions below are recorded here as decided; the rank-deficient spec remains
the theory document of record.

### 6.1 CHK-4 — `regular` means "regular where nonempty" (decided)

`regular` is the second reading: a regular zero set is regular *where it is
nonempty*; it does not assert nonemptiness. Consequences recorded by the
checker:

- `regular` does **not** refute `no_intersection`. The spec's third refutation
  row (`no_intersection` refuted when `regular`, *if* `regular` means nonempty)
  therefore does not apply.
- The `TANGENCY-CASCADE-REFINE-EMPTY` route `regular -> certified_empty` is
  valid, as are the other cascade refinements of `regular`
  (`TANGENCY-CASCADE-STAGE3-CRITICAL-VALUE`, `TANGENCY-CASCADE-CLASSIFY-BOX`).
  Re-audit under the pinned semantics: every cascade route was already accepted
  by the refinement rule (a rule may transform an axis its own precondition
  names), so **no route changes status**; the pin only changes refutation
  accounting.

### 6.2 CHK-1 — three-way status

Every concrete state is exactly one of:

| status | meaning |
|---|---|
| PROVED | in the fail-closed winning region `W_G` |
| REFUTED | the goal is provably false: `relation.zero_set` is in the goal's refutation set |
| UNDECIDED | neither; the only kind of state that is a gap |

Refutation predicates (the spec's section-2 table): `no_intersection` is
refuted by `zero_set = rank_deficient(residual)` (the certified nonempty zero
set under the pinned `regular`); `local_contact` is refuted by
`zero_set = certified_empty`. `REFUTED` states are removed from the missing
region and are reported separately.

### 6.3 CHK-2 — confirmed `T_geom` constraints (decided)

The three constraints are confirmed as standard dimension facts for the
carrier vocabulary the kernel admits:

| constraint | reason |
|---|---|
| `src_dims in {1x1,2x1} => local_dim <= 1` | two curves, or a curve and a surface, cannot meet in a 2-dimensional set |
| `src_dims in {1x1,2x1} and zero_set = regular => local_dim = 0` | full-rank contact between curves is isolated points |
| `zero_set = certified_empty => local_dim = 0` | local dimension is meaningless for an empty set, so its four values collapse to one accounting representative (`0`) |

The checker reports feasible-state and per-goal counts before (v1 `T_geom`,
43200 feasible fact states) and after tightening (24960).

### 6.4 CHK-3 — two coverage metrics

- **Fail-closed coverage** (renamed from the old "coverage"): the winning
  region `W_G`; every outcome either proves the goal or refuses with a named
  tag. Safety closure is still never coverage.
- **Strict coverage**: the winning region computed from rules with no refusal
  branch (`CompiledRule.strict_ok`: no named refusal and every outcome emits a
  concrete postcondition). A strict winning route contains no refusal.

### 6.5 Adjudication of the three v1 conflicts (method step 5)

The conflicts are reported by `detect_conflicts` and adjudicated against the
source tree; the verdict is applied to the merged model by `ADJUDICATIONS`
(fragments are not edited).

| conflict | verdict | evidence |
|---|---|---|
| `rule_id_content_mismatch` `TANGENCY-TSYSTEM-FROM-DEFLATED` | fragment B is source-faithful; A's `rep.construction_witness=true` / `rep.exact_implicit=true` outcomes are unsupported | `vendor/truck/truck-certified/src/tangency/tsystem.rs:82-96` |
| `postcondition_claim_mismatch` `exclude_rec` `ExclusionEvidence::NoRootFiveEq` | the minimal claim `{relation.zero_set: certified_empty}` is source-faithful; the `global.knowledge=local_only(residual)` restatement is the bottom of the knowledge order and is dropped | `vendor/truck/truck-certified/src/tangency/exclude.rs:166-211` |
| `postcondition_claim_mismatch` `CurveSpan2::RationalBezier` | the declared-only claim `{goal: complete_locus}` is source-faithful; the variant's rep flags are not certified here | `vendor/truck/truck-certified/src/formal/span.rs:119-137` |

The `TANGENCY-SYSTEM-FROM-DEFLATED` adjudication is the precondition that
`docs/RANK_DEFICIENT_CONTACT_SPEC.md` section 5.3 places on M4's deflation
reuse.

## 7. RDEF-M1 lattice v2, fragment E, dual-mode projection (CHK-5 to CHK-9)

Milestone M1 of `docs/RANK_DEFICIENT_CONTACT_SPEC.md` (packet
`RDEF-M1-LATTICE-V2`, owner proxy 2026-09-10). The production v1 lattice of
sections 1-6 is unchanged. The v2 lattice and fragment E are a **separate
projection**: `checker.lattice_v2` swaps the lattice globals, recompiles the
merged A-D rows plus fragment E in lattice-v2 terms, computes, and restores
v1. Fragment E is `confidence: proposed` and is never merged into the
production numbers; `state.json` carries both under `lattice` (v1) and
`lattice_v2` / `fragment_e` (M1).

### 7.1 CHK-5 - lattice v2 axes and children

Two axes are added:

| axis | values |
|---|---|
| `relation.regime` | `unknown(residual)`, `transversal`, `tangential`, `singular_param` |
| `relation.volume_evidence` | `none(residual)`, `sandwich_bounded` |

`relation.zero_set` gains six children under the `rank_deficient` parent. Each
fixes `local_dim` (enforced by `feasible_v2`):

| child | `local_dim` |
|---|---|
| `tangent_point` | `0` |
| `tangent_crossing` | `1` |
| `tangent_curve` | `1` |
| `tangent_higher` | `0` |
| `coincident` | smaller source dimension (`2x2 -> 2`, `2x1/1x1 -> 1`) |
| `near_degenerate(residual)` | `residual` |

T_geom v2 adds, beyond the M0 constraints: the fixed child dimensions above;
`volume_evidence = sandwich_bounded => regime = tangential`; a transversal
regime implies a regular or empty zero set; and a classified child
(`tangent_*`, `coincident`) exists only in the tangential regime. Feasible
fact states on the v2 lattice: **145920** (vs 24960 tightened v1).

### 7.2 CHK-6 - revised goal predicates

`volume_bracket` is proved when `knowledge in {all_components,
complete_locus}` **and** (`zero_set in {certified_empty, regular}` **or**
`volume_evidence = sandwich_bounded`). The union is not a product, so
`predicate_cubes_v2` emits two cube families. `local_contact`, `valid_brep`
and `mesh` accept the new children with a determined `local_dim` and do **not**
accept `near_degenerate`; `material_class` is unchanged.

### 7.3 CHK-7 - dual mode (OBL-K knowledge lift)

OBL-K (rank-deficient spec section 4.5) is not discharged. The checker runs
both modes and reports both:

- **lift off:** the sandwich rule sets `volume_evidence=sandwich_bounded` only.
- **lift on:** an explicit proposed modelling rule
  `E-OBL-K-KNOWLEDGE-LIFT` (not one of the nine section-7 rules) raises
  `global.knowledge` to `all_components` for the tangential `volume_bracket`
  region.

### 7.4 CHK-8 - fragment E (nine proposed rules)

The nine rules of `docs/RANK_DEFICIENT_CONTACT_SPEC.md` section 7 are encoded
in `loop/solver_coverage/fragments/E.json` with `confidence: proposed`. The
schema (`loop/solver_coverage/schema.json`) gained `"E"` in the fragment enum
and `"proposed"` in the confidence enum. Union precondition values
(`a|b`) are expanded into one compiled row per value. A proposed rule can
refine `rank_deficient(residual)` only by naming it (the v1 sink property is
preserved for the production lattice).

### 7.5 CHK-9 - reachability

`compute_reachability_v2` compares every fragment-E rule against fragment D's
door/bridge `reaches` set. Because fragment E is proposed and unwired, **all
nine rules are flagged as routing gaps** (none is named in `reaches`). The
report also records `first_witness_in_model`. The `E-TANGENCY-WITNESS-ZERO-BOUND`
(W4) rule is excluded from the default projection because W4 is off by default
(rank-deficient spec section 5.2); enabling it is an explicit `w4=True`
projection.

### 7.6 Open questions (rank-deficient spec section 11, Q1-Q4)

1. **Fragment schema / `!unmodeled`.** The schema is
   `loop/solver_coverage/schema.json`. `!unmodeled` preconditions go in the
   `preconditions` array as axis/value strings prefixed `!unmodeled:`
   (fragment E has four such axis keys). `CompiledRule` abstracts them; they
   never enter the arithmetic.
2. **OBL-S1 composition.** Unresolved. The checker models the sandwich as an
   additive `relation.volume_evidence` axis and does not claim that the
   contact-cover flux bracket composes with the sandwich term.
3. **1x1/2x1 volume measure.** Not determined from the model. Lattice v2
   applies the same `volume_evidence` axis to all `src_dims`; the actual
   curve-relation measure remains an [ASM] to confirm in M2.
4. **Does `regular` mean nonempty?** Decided in M0 (section 6.1): `regular`
   means "regular where nonempty"; it does not assert nonemptiness.

### 7.7 M1 acceptance

The M1 acceptance is the in-scope audited gap: surface-surface (`2x2`)
rank-deficient `volume_bracket`, regimes `{unknown(residual), tangential}`.
`1x1/2x1` is the out-of-scope section 1.3 knowledge-lifting gap; `singular_param`
is a named refusal (`SingularParametrization`).

| projection | in-scope rank-deficient UNDECIDED | witness cell proved |
|---|---:|---|
| production (A-D) | 17664 | no |
| proposed, lift off | 5760 | yes |
| proposed, lift on | **0** | **yes** |

The retrodiction witness cell
`2x2 / rank_deficient(residual) / local_dim 0 / local_only(residual) /
regime unknown / volume_evidence none / volume_bracket` (192 concrete states)
flips to PROVED under fragment E. The section 1.3 residual gaps are reported,
not hidden: `volume_bracket` 5760 (`2x1`) + 5760 (`1x1`),
`no_intersection` 5760 (`unknown`, 1x1/2x1), `valid_brep` 1920 (`2x1`), which
match the spec's declared section 1.3 counts.

