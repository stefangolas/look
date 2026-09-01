# BG-CK-P0-FREEZE — the Phase-0 contract freeze: F1 witness edge, F2 bound policy, F3 continuation coordinates

Certified-kernel plan Phase 0, second packet. The plan's own words: the four
contract-freeze decisions "are irrecoverable later" and are therefore made
HERE, in the packet, pre-made — the way BG-CG-000 froze the §3.5 certificate
mapping. This packet lands the frozen decisions as module docs + typed
signatures + contract-pinning tests in `truck-certified`, exactly the
CG-000 shape. No numerical implementations: every evaluator refuses, the
types and the DECISIONS are the deliverable. The mapping rows this implements
are already published in `docs/CERTIFICATE_MAPPING.md` section C (rows 2–4);
this packet adds no new evidence kinds and edits no mapping row.

```yaml
id:          BG-CK-P0-FREEZE
contract:    [BG-CK-P0-FREEZE]
class:       design
crates:      [truck-certified]
depends_on:  [BG-CK-P0-CRATE]
write_allow:
  - vendor/truck/truck-certified/src/contract.rs
  - vendor/truck/truck-certified/src/lib.rs
  - vendor/truck/truck-certified/tests/contract_freeze.rs
read_allow:
  - CERTIFIED-KERNEL-PLAN.md
  - docs/CERTIFICATE_MAPPING.md
  - vendor/truck/truck-certified/src/lib.rs
  - vendor/truck/truck-certified/src/meshable.rs
  - vendor/truck/truck-certified/src/formal/numeric.rs
  - vendor/truck/truck-certified/src/formal/evidence.rs
  - vendor/truck/truck-certified/src/formal/outcome.rs
budget:      {turns: 30, ctx_tokens: 100000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub mod formal;' vendor/truck/truck-certified/src/lib.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub mod domain;' vendor/truck/truck-certified/src/lib.rs"}
  - {id: A3, expect: 0, cmd: "grep -c 'pub mod contract;' vendor/truck/truck-certified/src/lib.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'pub mod meshable;' vendor/truck/truck-certified/src/lib.rs"}
  - {id: A5, expect: 1, cmd: "grep -rc 'pub struct Expansion' vendor/truck/truck-certified/src/formal/exact.rs"}
  - {id: A6, expect: 1, cmd: "grep -c 'pub enum StageOutcome' vendor/truck/truck-certified/src/formal/outcome.rs"}
  - {id: A7, expect: 1, cmd: "grep -c 'truck-geotrait' vendor/truck/truck-certified/Cargo.toml"}
tests_required:
  - witness_edge_carries_pcurve_pair_and_surfaces_and_enclosures
  - witness_edge_has_no_spline_field
  - bound_policy_table_names_all_five_quantities
  - denominator_well_definedness_uses_root_isolation_not_composition
  - continuation_coordinate_selection_is_deterministic_lowest_index_on_ties
  - coordinate_switch_requires_both_certificates
  - no_coordinate_certified_refuses_with_named_case
  - freeze_types_refuse_construction_outside_their_rules
```

## The three decisions, frozen (quote these verbatim into the module docs)

**F1 — witness edge (plan §1, §3 Phase 0, §4).** The certified Edge carries
the fiber-product witness — the pcurve pair, BOTH surface handles, and the
enclosures — not a fitted spline with error bars. Spline emission happens at
export/meshing only. A downstream consumer that wants a polyline gets it from
the witness at export time; the witness itself is the identity claim "there
was never a second edge", and it is NEVER a spline carrier. (Mapping section
C row 2: the witness stays attached to the edge; only derived facts with
`Method` tags enter row-set A carriers.)

**F2 — per-quantity bound policy (plan D2 scope statement).** The class-3
rational bounds decompose into five named quantities, and EACH gets one
pre-made choice between the two sanctioned mechanisms (named interval
composition vs auxiliary root isolation):

| Quantity | Choice | Mechanism |
|---|---|---|
| normal admissibility: certified lower bound on `|Sᵤ × Sᵥ|` | interval composition | fixed named composition: hull-bounded first-derivative patches, interval cross product, directed rounding at the leaves |
| curvature (rational in derivatives through order 2) | interval composition + isolation guard | value from the named composition of hull-bounded derivative enclosures; the well-definedness of the division (denominator ≠ 0) is certified by AUXILIARY ROOT ISOLATION on the denominator polynomial via `bezier_isect` — never by interval sign-testing alone |
| rational NURBS numerator/denominator | interval composition | homogeneous control points bounded separately (hulls), division under directed rounding |
| rational NURBS quotient (the divided value) | interval composition | directed-rounded division of the two enclosures above |
| any FUTURE quantity not in this table | unspecified — refuses | a quantity outside the frozen table is a SPEC_GAP: the policy records `Unfrozen` and the constructor refuses `InvalidInput`; widening the table is an orchestrator spec edit, never a worker decision |

**F3 — continuation-coordinate contract (plan §2 class 2 generic).** The
class-2 Krawczyk operator runs on SQUARE 3×3 systems only (the
pseudo-inverse-preconditioned rectangular route is explicitly rejected). Per
box, ONE continuation coordinate is selected by this frozen rule: the
coordinate `i` whose certified ∂H_i/∂t_i enclosure over the box is strictly
away from zero with the LARGEST relative margin (|lower bound| / box extent
in t_i); ties break to the LOWEST index (deterministic — no hash order). If
NO coordinate certifies away-from-zero, the box refuses
`ConditioningBelowThreshold` — it is never retried with a weaker test.
Turning-point SWITCHING is a certified event: at a switch box, BOTH square
systems (the outgoing coordinate's and the incoming coordinate's) are
certified by their own Krawczyk calls, and the traced branch records a
`CoordinateSwitch` carrying both certificates. A heuristic reseed without
both certificates is a contract violation.

## Section 1 — `truck-certified/src/contract.rs` (NEW)

Header: the crate's lint style (match `lib.rs`). Module doc: the three
frozen decisions above, quoted, each tagged with its plan section. The
module is the FROZEN TEXT made typecheckable; Phase-1 packets implement
against it and never relitigate it.

Types (signatures exact; bodies refuse — this is a freeze, not an
implementation):

```rust
/// F1: the fiber-product witness. The certified Edge IS this; a spline
/// view is derived at export only (a future ExportView type, not a field).
pub struct WitnessEdge<S, C> {
    /// The two pcurves, one per support surface, in the support surfaces'
    /// own charts (the identify_plane retained-basis doctrine: never
    /// orthogonalised, never normalised).
    pub pcurve_a: C,
    pub pcurve_b: C,
    /// Both support surfaces. Handles, not copies.
    pub surface_a: S,
    pub surface_b: S,
    /// Enclosures for both pcurves over their domains. `Method::Interval`
    /// per H-6 — the witness is interval work; a float estimate never
    /// enters this struct.
    pub enclosure_a: IntervalEnclosure,
    pub enclosure_b: IntervalEnclosure,
}

/// The F2 table as data. Construction only through `BoundPolicy::frozen()`,
/// which returns the five-row table above; every other construction path
/// refuses.
pub struct BoundPolicy { /* five named rows, exactly the frozen table */ }

/// F3: which coordinate runs the square system, and why.
pub struct ContinuationCoordinate {
    /// 0-based coordinate index.
    pub index: usize,
    /// The certified away-from-zero margin of dH_i/dt_i over the box,
    /// relative to the box's t_i extent. `Method::Interval`.
    pub relative_margin: IntervalEnclosure,
}

/// F3: a turning-point switch. Both fields are REQUIRED certificates —
/// there is no default, no `Option`, no reseed path.
pub struct CoordinateSwitch {
    pub outgoing: ContinuationCoordinate,
    pub incoming: ContinuationCoordinate,
}
```

`IntervalEnclosure` — reuse what `formal/numeric.rs`/`formal/evidence.rs`
already name for certified interval bounds if such a type exists there;
otherwise define the minimal one here (lower/upper as certified interval
values, `Method` tagged). Do NOT invent a second interval algebra — D2's
parsimony rule: one primitive, composed.

Refusing evaluators (typed stubs, each returning
`Err(Refusal-shape per formal/outcome.rs)`):

```rust
/// F2: the per-quantity bound, dispatching on the frozen table.
pub fn certified_bound(quantity: Quantity, patch: &BoundedSurfaceInput)
    -> Result<IntervalEnclosure, Refusal>;   // refuses Unfrozen/InvalidInput

/// F3: the per-box coordinate selection. Deterministic; refuses
/// ConditioningBelowThreshold when no coordinate certifies.
pub fn select_continuation_coordinate(system: &SquareSystemInput)
    -> Result<ContinuationCoordinate, Refusal>;
```

`Quantity` is the five-row enum (NormalAdmissibility, Curvature,
RationalNumerator, RationalDenominator, RationalQuotient).

## Section 2 — lib.rs: one line

`pub mod contract;` added to the four existing module declarations. Nothing
else in lib.rs changes (the lint header and the four moved modules are
BG-CK-P0-CRATE's landed surface — V8 identity applies).

## Section 3 — tests (`truck-certified/tests/contract_freeze.rs`, NEW)

Names are contract (`tests_required`). The load-bearing assertions:

1. `witness_edge_carries_pcurve_pair_and_surfaces_and_enclosures` — the
   struct has exactly the six fields; construct one from toy types.
2. `witness_edge_has_no_spline_field` — guard: no field, method, or impl on
   `WitnessEdge` names a spline/Bézier emission; the export view is a
   future type. (Assert by construction: the type has no such accessor —
   a compile-level negative, expressed as a doc test or a comment-pinned
   check, whatever the crate's test style supports.)
3. `bound_policy_table_names_all_five_quantities` — `BoundPolicy::frozen()`
   exposes exactly the five rows, matching the F2 table row for row.
4. `denominator_well_definedness_uses_root_isolation_not_composition` — the
   Curvature row's guard mechanism is recorded as root isolation; a policy
   construction attempting composition-only for that guard refuses.
5. `continuation_coordinate_selection_is_deterministic_lowest_index_on_ties`
   — two coordinates with equal margins select the lower index.
6. `coordinate_switch_requires_both_certificates` — `CoordinateSwitch` has
   no `Option`/default path; both fields are inhabited in every value.
7. `no_coordinate_certified_refuses_with_named_case` — the refusing
   evaluator returns the named `ConditioningBelowThreshold` case.
8. `freeze_types_refuse_construction_outside_their_rules` — the refusing
   evaluators refuse; no evaluator in this module ever panics (the
   crate-level constructor doctrine).

House rules: H-3 float-comparison opt-outs go ON THE SAME LINE. Clippy
zero findings on the new files (`cargo clippy -p truck-certified
--all-targets --message-format=short --no-deps`).

## Done-when

- `cargo fmt --all -- --check` clean.
- `cargo clippy -p truck-certified --all-targets --message-format=short
  --no-deps` — zero findings.
- `cargo test -p truck-certified --lib --tests --no-fail-fast` green —
  moved-tree suites unchanged PLUS the new contract tests.
- `cargo check --workspace --all-targets` green.

## Stop conditions

Stop, commit nothing beyond WIP evidence, write RESULT.json (AT THE WORKTREE
ROOT) with the finding verbatim if:

1. `formal/` already contains a certified-interval type that should be
   reused for `IntervalEnclosure` but cannot be without widening it — name
   the type; reuse-with-widening is an orchestrator decision.
2. The F2/F3 types cannot express the frozen decisions without a mechanism
   the plan bars (expression-tree interval arithmetic, a second root
   engine, a rectangular Krawczyk).
3. A moved module's API changed under you relative to the anchors — the
   tree moved; stop, do not adapt silently.

## Finish by writing `RESULT.json` AT THE WORKTREE ROOT

Commit your work on the current branch (subject: `feat(certified): Phase-0
contract freeze F1/F2/F3 (BG-CK-P0-FREEZE)`) BEFORE writing `RESULT.json`.
