# WORK PACKET CT-101-SELECTOR-SEMANTICS — P1 conformance: the selector table against the landed fold

Discharges proof obligation P1 as machine-checked conformance: the
selector semantics of equation (1) of `docs/CONTACT_ATLAS_SPEC.md` —
which source boundaries appear with which sign in each regularized
operation — validated against the landed certified fold on ground-truth
fixtures. **Tests-only packet**: no kernel file is touched.

```yaml
id:          CT-101-SELECTOR-SEMANTICS
contract:    [CT-101-SELECTOR-SEMANTICS]
class:       mechanical
crates:      [truck123d]
depends_on:  [CT-000-ATLAS-CONTRACT-SHIM]
needs:       [CT-000-ATLAS-CONTRACT-SHIM]
write_allow:
  - truck123d/tests/atlas_selector_semantics.rs
read_allow:
  - docs/CONTACT_ATLAS_SPEC.md
  - docs/CONTACT_ATLAS_BUILD_SPEC.md
tests_required: [truck123d/tests/atlas_selector_semantics.rs]
anchors:
  - {id: A1, min: 2,  cmd: "grep -cE '\\bcompound_indicator\\b' truck123d/src/bd_bridge.rs"}
  - {id: A2, min: 40, cmd: "grep -cE '\\bVolumeRow\\b' truck123d/src/bd_bridge.rs"}
budget:      {turns: 35, ctx_tokens: 120000}
```

Anchors at authoring time `e271ba9`. **Re-measure at dispatch.**

## The conformance matrix (the whole packet in one table)

For each operation in {union, intersect, difference} and each support
configuration in {disjoint, abutting-planar (antiparallel normals),
abutting-planar (parallel normals — via a duplicated-face construction),
overlapping}, assert on the CT-000 fixtures:

1. the fold's returned bracket contains the analytic volume;
2. the bracket WIDTH is consistent with the geometry class (disjoint
   union = exact sum, zero-width to machine precision; abutting
   antiparallel union = exact sum of parts; overlapping = the analytic
   overlap);
3. the selector-derived sign structure matches Theorem 6's table — for
   the abutting-antiparallel union case the shared interface contributes
   nothing to the bracket (its true contribution is zero).

## Pre-made judgements

1. The fold under test is the LANDED `certify_boolean_volume` path via
   `bd_facts` — no atlas code is under test yet (the scheduler wrap is
   CT-100's; these tests must hold BOTH before and after CT-100, making
   them the V5 net for the scheduler change).
2. Parallel-normals abutting cases are constructed by explicit duplicated
   geometry (two coincident planar faces), which the current admission
   may refuse typed — a typed refusal IS a valid recorded outcome for
   that cell of the matrix; assert the refusal CODE, not greenness.
3. Analytic volumes are exact formulas (box products, cylinder
   intersections with known axes) — H-3 same-line markers where float
   epsilons appear.

## Done-when

1. `cargo test -p truck123d --test atlas_selector_semantics --locked`
   green, with one test per matrix cell (12 minimum) and the
   refusal-cell assertions naming the exact code.
2. `cargo fmt -p truck123d -- --check` clean on the test file.
3. Write `RESULT.json` AT THE WORKTREE ROOT (nowhere else), including the
   full matrix outcome table (which cells are certified-green, which
   typed-refused, with codes).
