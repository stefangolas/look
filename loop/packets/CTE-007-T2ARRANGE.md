# WORK PACKET CTE-007-T2ARRANGE — carrier arrangement, atoms, self-pair gate

You are implementing Part II (T2) of the Certified Tangency and Exact Contact
program. Everything you need is in this document, `docs/CTE_BUILD_SPINE.md`,
and the root theory `docs/CERTIFIED_TANGENCY_EXACT_CONTACT_SPEC.md` (§5 is
normative). Do not read other spec files. If something you need is genuinely
missing, that is a SPEC_GAP: you stop and report, you do not research it.

```yaml
id:          CTE-007-T2ARRANGE
contract:    [CTE-007-T2ARRANGE]
class:       design
crates:      [truck-shapeops, truck-certified]
depends_on:  [CTE-001-QPOLY, CTE-004-HESSIAN, CTE-005-A2, CTE-006-SIDEALG]
write_allow:
  - vendor/truck/truck-certified/src/tangency/arrange.rs
  - vendor/truck/truck-certified/src/tangency/mod.rs
  - vendor/truck/truck-shapeops/src/boolean/split.rs
  - vendor/truck/truck-shapeops/src/boolean/assemble.rs
read_allow:
  - vendor/truck/truck-certified/src/tangency/{shapes,fixtures}.rs
  - vendor/truck/truck-certified/src/tangency/{a2,cascade,witness}.rs
  - vendor/truck/truck-shapeops/src/boolean/{mod,classify}.rs
  - vendor/truck/truck-topology/src/entity_id.rs
  - docs/CERTIFIED_TANGENCY_EXACT_CONTACT_SPEC.md
  - docs/CTE_BUILD_SPINE.md
tests_required:
  - self_pair_rewrites_before_sweep
  - tangential_curve_is_stratum_on_f5_carrier
  - h_atom_probe_points_off_boundary
  - containment_certified_without_case_explosion
  - butt_join_union_drops_shared_wall
  - atom_emission_is_single_canonical_record
budget:      {turns: 120, ctx_tokens: 240000}
```

## Problem

T2 classifies coincident-carrier fragments by the two-bit side algebra over a
certified carrier arrangement whose atoms include tangential contact curves
as strata (H-atom, §5.2) — the structural T1→T2 link: CTE-005's
`A2BranchCurve` output feeds the arrangement. You build: the carrier
arrangement + atoms; the containment predicate (§5.6); the self-pair
entry-gate rewrite (§5.8 — the landed typed refusal at the self-pair becomes
a rewrite before the sweep for certified operand identity); and the atom
emission semantics (§5.5 corollary — one canonical geometry record +
provenance set, never two coincident faces).

## Scope decisions — pre-made, do not relitigate

1. **H-atom is a construction invariant, checked at atom admission**: probe
   points `p ± ε·n_C` off every operand boundary for the atom's certified ε₀
   range — the theory's formalization (R10). An atom that fails admission
   splits the arrangement further; a split that cannot fix it refuses
   (named), never guesses.
2. **The decision is a function of the signature** (T2.4): three-valued
   `{Drop, Keep(canonical), Keep(flipped)}` — the landed
   `fragment_decision` (refactored by CTE-006 into `SideState`) already
   computes this; you DERIVE `σ_A, σ_B` per atom from the classifier's
   parity bits (§5.3 implementation note — no new membership predicate).
3. **Certified carrier coincidence uses the §4 primitive**
   (`CoincidenceWitness`): provenance-first via construction-node identity
   (`EntityId` — same construction node ⇒ rewrite before the sweep, §5.8
   arm 1); geometric equality through the contact calculus otherwise
   (arm 2). The exact-algebraic fallback for arm 2 reuses CTE-001's
   verifier shape; if a case needs a NEW producer, that is §6.1 open —
   emit `Unresolved`, do not improvise.
4. **Containment is cheap by design** (§5.6): interval separation of trim
   boundaries' parameter boxes; dyadic-exact where the chart is dyadic; one
   certified interior seed per connected domain. Full curve arrangement
   only when boundaries touch/overlap. Containment decides WHERE the bit
   ops evaluate, never WHAT they return.
5. **Butt-join union** (§5.9): the anti-oriented shared wall carries
   `10/01` ⇒ union `11` ⇒ certified internal ⇒ discard. The landed
   orientation-consistency fold (`assemble.rs:612-620`) is the arm this
   generalizes; behavior on the landed battery is unchanged.
6. **You are `boolean/{split,assemble}.rs`'s sole writer this program**
   (CTE-006 owns `mod.rs` and is DONE before you dispatch). Landed test
   files are byte-identical constraints (V5 guard).
7. **Scoping correction** (§5.9): the exact-footprint halfspace refusal is
   NOT a T2 case — do not book it into the battery; T2 closes
   coincident/shared-wall classification only.

## Contract — frozen output (spine §2; CTE-008 consumes)

```rust
// tangency/arrange.rs
pub struct CarrierArrangement { pub atoms: Vec<Atom>, /* cells + strata */ }
pub fn arrange_carrier(carrier: &CoincidenceWitness, trims: &[TrimDomain],
                       tangential: &[A2BranchCurve]) -> Result<CarrierArrangement, TangencyRefusal>;
pub fn side_signature(classifier_bits: &ParityBits, atom: &Atom) -> (SideState, SideState);
pub fn atom_decision(sa: SideState, sb: SideState, op: BoolOp) -> FragmentDecision;
// boolean/assemble.rs + split.rs
// self-pair rewrite: certified operand identity → pre-sweep rewrite (§5.8);
// the typed refusal remains ONLY for uncertifiable identity.
```

## Anchors — measured this session; re-check on your branch before building

| id | file | pattern | expect |
|---|---|---|---|
| A1 | `truck-shapeops/src/boolean/split.rs` | `pub struct CoincidentPair` | 1 |
| A2 | `truck-shapeops/src/boolean/split.rs` | `pub enum CoincidentOrientation` | 1 |
| A3 | `truck-shapeops/src/boolean/mod.rs` | `pub fn fragment_decision` | 1 |
| A4 | `truck-shapeops/src/boolean/assemble.rs` | `fn decide_and_assemble` | 1 |
| A5 | `truck-certified/src/tangency/shapes.rs` | `pub struct CoincidenceWitness` | 1 |
| A6 | `truck-certified/src/tangency/shapes.rs` | `pub struct A2BranchCurve` | 1 |
| A7 | `truck-certified/src/tangency/shapes.rs` | `pub struct SideState` | 1 |
| A8 | `truck-topology/src/entity_id.rs` | `pub struct EntityId` | 1 |

## House rules

- **H-1** No `unwrap`, `expect`, `panic!`, `todo!`, or out-of-range indexing
  reachable from geometry.
- **H-2** Fallible operations return `Result<T, named refusal>`.
- **H-3** No absolute constants in predicates; test epsilons carry `// H-3`
  on the SAME line as the literal.
- **H-6** Float-computed values are never recorded as `Method::Exact`.
- **Determinism**: fixed stratum order; fixed enumeration; no hash ordering
  in output.
- **All cargo invocations go through the queue (the `cargo` on PATH IS the
  queue shim). Do not invoke cargo by absolute path; do not unset the shim.**
- Never run a bare `cargo test` — use the scoped commands below.

## Tests required

1. `self_pair_rewrites_before_sweep` — certified-identical operands
   (`A∪A=A`, `A∩A=A`, `A−A=∅`) rewrite without entering the sweep; the
   landed typed refusal remains for uncertifiable identity (§5.8).
2. `tangential_curve_is_stratum_on_f5_carrier` — the F5 A₂ branch curve is
   an arrangement stratum; atoms are constant across it (H-atom).
3. `h_atom_probe_points_off_boundary` — probe admission holds on every atom
   of the F7-style coplanar fixtures at the certified ε₀.
4. `containment_certified_without_case_explosion` — `D_A ⊆ D_B` reduces to
   two atoms; separation certifies without the full arrangement.
5. `butt_join_union_drops_shared_wall` — the 10/01 union row drops the wall
   (§5.9), congruent with the landed orientation fold. **R2 addition
   (adjudicated from CL-005's SPEC_GAP, commit `4d59d5b`)**: the x/y-axis
   full-face butt joins currently refuse inside `split.rs::finish`
   (Region2 `CoincidentInterval` between vertical side faces) while their
   z-axis twin certifies — your splitter work makes the vertical seam
   split identically, and the worker's recorded experiment (Region2
   `Coincident` events are load-bearing for strictly-interior
   containments, harmful only for the boundary-touching coplanar class)
   is the evidence to build against. The exact-footprint halfspace class
   stays OUT (theory §5.9: interior-loop rewrite machinery, booked open).
6. `atom_emission_is_single_canonical_record` — a kept coincident atom
   emits ONE canonical geometry record + provenance set, never two faces.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-shapeops -p truck-certified
cargo clippy -p truck-shapeops -p truck-certified --all-targets -- -D warnings
cargo test -p truck-shapeops --lib boolean
cargo test -p truck-certified --lib tangency
cargo check -p truck-shapeops
```

Send cargo output to a file and read the tail.

## Forbidden

Editing any file outside `write_allow` — especially `boolean/mod.rs`
(CTE-006's, already landed), `boolean/classify.rs`, any landed test file
(`boolean_m2.rs`, `conformance_battery.rs` are byte-identical constraints),
`truck-certified/src/tangency/{shapes,a2,cascade}.rs`, `Cargo.lock`.
Adding `#[ignore]`. Adding `#[allow]` without a justification comment on the
same line. Committing to `main`.

## Stop conditions

- any anchor count differs → `ANCHOR_MISMATCH`
- an H-atom admission that no subdivision can certify on the fixture family
  → `SPEC_GAP` with the atoms and probe geometry
- three consecutive failed `cargo test` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` at the WORKTREE ROOT (then COMMIT first)

**COMMIT BEFORE writing `RESULT.json`.**

```json
{"id":"CTE-007-T2ARRANGE","status":"DONE","contracts":["CTE-007-T2ARRANGE"],
 "tests_added":6,"anchors_verified":{"A1":1,"A2":1,"A3":1,"A4":1,"A5":1,"A6":1,"A7":1,"A8":1},
 "notes":"arrangement strata inventory; self-pair rewrite path taken per fixture; any API deviation"}
```

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit on the current branch with subject
`feat(shapeops): carrier arrangement + atoms + self-pair entry-gate rewrite (CTE-007-T2ARRANGE)`.
