# WORK PACKET BREP-001A-PIPELINE-CORRECTNESS — the BRep generation pipeline defects, one combined packet

You are correcting Five violated obligations in the BRep generation pipeline
(STEP ingestion → surface conversion → boundary projection → material domain
→ tessellation, plus the boolean screen layer). Everything you need is in
this document and the six normative defect records in `docs/defects/` named
below — **read each named record before its fix; they are normative**. Do not
read other spec files. If something you need is genuinely missing, that is a
SPEC_GAP (see "Stop conditions"): you stop and report, you do not research it.

```yaml
id:          BREP-001A-PIPELINE-CORRECTNESS
contract:    [BREP-001A-PIPELINE-CORRECTNESS]
class:       design
crates:      [truck-stepio, truck-geometry, truck-meshalgo, truck-evidence]
depends_on:  []
write_allow:
  - vendor/truck/truck-meshalgo/src/tessellation/triangulation.rs
  - vendor/truck/truck-stepio/src/in/mod.rs
  - vendor/truck/truck-stepio/src/in/step_geometry.rs
  - vendor/truck/truck-geometry/src/decorators/revolved_curve.rs
  - vendor/truck/truck-geometry/src/arrange.rs
  - vendor/truck/truck-meshalgo/tests/defect_regressions.rs
  - vendor/truck/truck-geometry/tests/defect_regressions.rs
  - vendor/truck/truck-modeling/tests/revolve_p5.rs
  - vendor/truck/truck-modeling/tests/until_p4.rs
read_allow:
  - docs/defects/PAR-RANGE-INHERITANCE-001.md
  - docs/defects/QUO-EUCLIDEAN-CLOSURE-001.md
  - docs/defects/DOM-ARTIFICIAL-CLOSURE-001.md
  - docs/defects/DOM-ZERO-AREA-001.md
  - docs/defects/DSC-BOUNDARY-SAMPLE-EXTENT-001.md
  - docs/defects/NUM-SUBDIVISION-GROWTH-001.md
  - docs/defects/GEO-INCIDENCE-ACCEPTANCE-001.md
  - docs/defects/SEM-ARRANGE-DYADIC-CONTRACT-001.md
  - docs/defects/DEFECT_INDEX.md
  - vendor/truck/truck-meshalgo/src/tessellation/
  - vendor/truck/truck-geometry/src/decorators/
tests_required:
  - quo_euclidean_closure_001_full_period_is_closed_in_quotient
  - quo_euclidean_closure_001_partial_gap_stays_open
  - dom_artificial_closure_001_lone_open_piece_is_not_closed_against_range_edge
  - dom_artificial_closure_001_outer_bound_role_survives_stepio
  - dom_artificial_closure_001_whole_rectangle_branch_census_is_zero_or_recorded
  - par_range_inheritance_001_face_domain_does_not_use_line_default
  - par_range_inheritance_001_same_cone_two_encodings_same_domain
  - num_subdivision_growth_001_cap_hit_is_observable_as_resource_capped
  - num_subdivision_growth_001_sample_count_constructor_refuses_non_finite
  - sem_arrange_dyadic_contract_001_non_dyadic_vertices_refuse_typed
  - geo_incidence_001_transform_provenance_probe_records_first_failing_stage
budget:      {turns: 160, ctx_tokens: 220000}
```

**New files** (`defect_regressions.rs` × 2): H-1 applies — no `unwrap_used`
without a justified same-line opt-out.

**R2 split note:** this packet was split from BREP-001 so it can dispatch
while CTE-007 runs — the DSC enclosure-screen fix (Contract F) moved to
[`BREP-002-DSC-SCREENS`](BREP-002-DSC-SCREENS.md) because it writes
`boolean/assemble.rs`, which CTE-007 owns until it lands. Nothing else
changed.

## The one-sentence version

Five records, one pipeline: make the domain model derive from source
semantics (A→B→C), make cap-hitting observable (D), make the arrange
refusal honest (E), and settle the incidence question with a probe instead
of a gate (G). Work them in that order; ONE commit at the end.

## Contracts — frozen, per boundary (these cross six module boundaries; each is restated AT the boundary)

### Contract A (QUO) — closure in the quotient
`PolyBoundary`'s open/closed classification of a boundary piece on a
periodic axis must test closure **in the quotient** of the periodic axis: a
lifted gap equal to the axis period (to the exact-predicate discipline, not
a loosened tolerance) IS closure. Signature: the piece classifier gains the
period (already carried as `vperiod`) and compares `gap ≡ 0 (mod period)`.
The Euclidean equality test it replaces is the defect. Partial gaps stay
open. (Record: QUO-EUCLIDEAN-CLOSURE-001; measurement `gap=2π=perimeter,
closed=false` must flip to closed on the reproducer.)

### Contract B (DOM) — every boundary segment has a source antecedent
`∀ s ⊆ ∂M, ∃ a source entity e with s ∈ desc(e)`. Concretely:
1. `truck-stepio` preserves `FACE_OUTER_BOUND` vs `FACE_BOUND` as a `role`
   field on the parsed bound (both currently collapse into one struct — the
   file states the outer/hole role explicitly and the parse discards it).
2. `PolyBoundary::new`'s branch structure is REPLACED by an explicit base
   domain: `BaseDomain { Empty, NaturalRange, PeriodicQuotient }` with
   material region = `base XOR bound-parity` (`BoundRole { Outer, Inner }`),
   classified from the parsed roles — never from loop count.
3. The `\|open\|=1 → close against the range edge` path and the
   `closed empty ∧ finite range → whole rectangle` path are DELETED. The
   `AllBoundsCollapsed` refusal (11 references, landed) is PRESERVED
   byte-identically — it is why INC-VERTEX-LOOP-001 added no blobs.
4. A census probe counts how often the deleted whole-rectangle branch fired
   on the corpus (the record's uncounted U-item) and reports it in RESULT.

### Contract C (PAR) — the working domain is derived, not inherited
`Ω ⊇ π(Γ)` and `Ω ⊥ (construction route to S)`. `Line::parameter_range()`
keeps returning `[0,1]` (landed, other callers depend on it) — instead the
REVOLVED surface's domain is derived from its face's bounds at the
conversion boundary: the conversion context (or the retune pass, worker's
choice — state it in RESULT) computes the generatrix interval that covers
the face's own boundary curves. **A constant window is the defect itself**
(measured: the flag's factor-2 destroys 268 faces). The acceptance gate is
the metamorphic test: the SAME cone in two encodings (ctc_02 AP203/AP242
fixture class) must yield the SAME derived domain.

### Contract D (NUM) — cap-hitting is observable (RES-003)
`MAX_CIRCLE_DIVISION`/`MAX_DIVISION_CELLS` stay as bounds but a capped
request is no longer ordinary success: a checked `SampleCount` constructor
(refuses non-finite/negative) and a `ResourceCapped` signal on the returned
division, so the fourth-kind test — "no model silently hits the cap" —
becomes writable. The cap VALUE is unchanged (4096; chord error ~2.3e-7 ×
radius). No change to any uncapped result (bit-identical guard).

### Contract E (ARRANGE) — the dyadic contract is typed
The profile-arrangement entry refuses non-dyadic profile vertices with a
NAMED typed refusal (e.g. `NonDyadicProfileVertices { at }`) BEFORE the
root-isolation call, instead of letting them surface as the misleading
`RootNotIsolated`. The landed dyadic behavior is unchanged (dyadic twins
arrange exactly as today). Location: the arrange entry
(`truck-geometry/src/arrange.rs`, 3 `pub fn arrange` matches — the
profile-facing one), with the precondition on vertex coordinates the
SEM-ARRANGE-DYADIC-CONTRACT-001 record defines.

### Contract F — moved to BREP-002-DSC-SCREENS
The DSC enclosure-screen fix writes `boolean/assemble.rs`, which CTE-007
owns until it lands — it is BREP-002's whole content. Read its record
(DSC-BOUNDARY-SAMPLE-EXTENT-001) but do NOT implement it here.

### Contract G (GEO) — the incidence question is settled by a probe, not a gate
NO production policy change: `COMPATIBILITY_FACTOR` stays `inf` (the record's
refusal stands — the gate costs 10% of a model and fixes zero blobs, and it
masked a crash). You add ONLY the transform-provenance probe as a test-side
diagnostic: evaluate source and converted geometry in the same coordinates at
each transform stage and record the FIRST stage where
`d(C(t), S) ≤ ε` stops holding on the 00009190 witness population. RESULT
notes the stage; the owner decides policy afterward. Also record the
face→surface pairing answer (checked for edges, never for surfaces).

## Ordering (fail-closed per fix)

A → B → C share `triangulation.rs`/`stepio` and chain causally (each arrow
of PAR's derivation is separately correctable; the closure test feeds the
open-piece classification, which feeds the parity model, which the domain
derivation consumes). D, E are independent — interleave anywhere. G is
test-side, anytime. **After each fix, run that fix's named tests
before starting the next; a red gate stops the packet** (stop condition 3).

## Anchors — measured 2026-09-06 morning, re-check on your branch

| id | file | pattern | expect |
|---|---|---|---|
| A1 | `truck-meshalgo/src/tessellation/triangulation.rs` | `impl PolyBoundary` | 1 |
| A2 | `truck-geometry/src/specifieds/line.rs` | `fn parameter_range` | 1 |
| A3 | `truck-geometry/src/decorators/revolved_curve.rs` | `MAX_CIRCLE_DIVISION` | ≥1 |
| A4 | `truck-stepio/src/` (recursive) | `FACE_OUTER_BOUND` | 10 |
| A5 | `vendor/truck/` (recursive) | `AllBoundsCollapsed` | 11 |

A4/A5 are shared-file totals quoted as CENSUS anchors only — your write set
names the files; do not re-count them as behavior gates.

## Acceptance gates (the records' own measurements — these ARE the verdicts)

1. `repro/apex_only.stp`: 0 → ~46 triangles (the plane_control figure).
2. ctc_02: the 148 cone faces render in BOTH the AP203 and AP242 encodings.
3. Corpus blob shells: 10 baseline — must NOT increase; `#161274` (ratio
   30.3) should disappear via C, not via silent rejection.
4. Every currently-correct render stays byte-identical EXCEPT the measured
   defect populations (V5 with the records' carve-outs).
5. `00000730`: still renders (~425k triangles), now with any cap-hit visible
   as `ResourceCapped` instead of silent.

## House rules

- **H-1** no unwrap/expect/panic reachable from geometry; **H-3** same-line
  `// H-3`; **H-6** never record `Float` as `Exact`.
- **Determinism**: fixed enumeration order everywhere; no hash ordering in
  output.
- **All cargo through the queue shim.** Scoped commands only:
  `cargo check -p truck-stepio -p truck-geometry -p truck-meshalgo -p truck-shapeops`,
  the per-fix `cargo test -p <crate> --lib <filter>` and the two
  `--test defect_regressions` targets. Never a bare `cargo test`, never a
  workspace build.
- **ID-named regressions**: the tests_required names follow the defect
  index's convention — the index becomes load-bearing through this packet.
- This packet changes LANDED tessellation behavior on measured-defect
  populations only. Any behavior diff outside the four acceptance gates is
  a SPEC_GAP, not a tuning opportunity.

## Forbidden (the records' own list, verbatim in spirit)

A synthesized small circle around the apex; using the declared rectangle
when the boundary lies on its edge; loosening the closure tolerance until
2π counts as zero; any arbitrary range multiplier (the experiment's factor
of 2 included); file-specific exceptions; silently guessed parameters;
turning `COMPATIBILITY_FACTOR` on; deleting or weakening
`AllBoundsCollapsed`; touching `truck-evidence/src/num/krawczyk.rs`,
`tangency/**` (CTE's, landed), or `boolean/assemble.rs` (BREP-002's write
set — the DSC screen fix lives there); `Cargo.lock`. Adding `#[ignore]`.
Unjustified `#[allow]`. Committing to `main`.

## Stop conditions

- any anchor count differs → `ANCHOR_MISMATCH`
- an acceptance gate cannot be met without violating a Forbidden item →
  `SPEC_GAP`, naming the blocked gate and the record that forbids the
  available fix
- the QUO/DOM/PAR chain cannot be landed incrementally (a fix gates on a
  later one) → `SPEC_GAP` with the dependency you found
- three consecutive failed `cargo test` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` at the WORKTREE ROOT (then COMMIT first)

**COMMIT BEFORE writing `RESULT.json`.**

```json
{"id":"BREP-001A-PIPELINE-CORRECTNESS","status":"DONE","contracts":["BREP-001A-PIPELINE-CORRECTNESS"],
 "tests_added":11,"anchors_verified":{"A1":1,"A2":1,"A3":1,"A4":10,"A5":11},
 "notes":"per-fix: which acceptance gates met (apex triangles, ctc_02 both-encodings, blob census, cap observability); the whole-rectangle census count; the transform-provenance probe's first failing stage; every API deviation"}
```

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit subject: `fix(brep): pipeline correctness — quotient closure, base-domain parity, bounds-derived domains, observable caps, typed arrange refusal, incidence probe (BREP-001A-PIPELINE-CORRECTNESS)`.

## Amendment r2 (orchestrator, 2026-09-06) - downstream compile ripple of the typed refusal

Your r1 work landed (c7c9b41, merged). One downstream ripple remains: the
E-ARRANGE deliverable changed `arrange()`'s error type from `Refusal` to
`ArrangeError`, and two landed truck-modeling test files call `arrange()`
through helpers typed against the OLD error type. At integrated HEAD:

- `cargo check --workspace --all-targets` fails with 9x E0308 in
  `vendor/truck/truck-modeling/tests/until_p4.rs` (helpers at :38) and
  2x E0308 in `vendor/truck/truck-modeling/tests/revolve_p5.rs`
  (helper `expect_ok` at :35): expected `Result<Certified<_>, Refusal>`,
  found `Result<Certified<Arrangement>, ArrangeError>`.

Required correction (mechanical): widen those test files' helpers to accept
the new typed error (`Result<Certified<Arrangement>, ArrangeError>` or a
generic `Result<Certified<T>, E: Debug>` - your call, keep it minimal), fix
every call site the compiler names, change NOTHING else. The helper
refactors must not weaken any assertion: same expected values, same
refusal cases. The rebase onto integration HEAD is already done for you -
commit on top of it.

Done-when additions:
cargo check -p truck-modeling --tests
cargo test -p truck-modeling --test revolve_p5 --test until_p4
Commit subject: `fix(brep): widen truck-modeling test helpers to the typed ArrangeError ripple (BREP-001A-PIPELINE-CORRECTNESS r2)`.
Write RESULT.json AT THE WORKTREE ROOT (r2 - notes describe the helper widening).
