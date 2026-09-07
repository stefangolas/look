# WORK PACKET DEF-VENDOR-FIXTURES â€” vendor the resources/shape JSON fixtures the upstream suites require

Two upstream suites read `CARGO_MANIFEST_DIR/../resources/shape/*.json` and
have NEVER run in this tree: the directory was never vendored (verified:
zero `--diff-filter=A` hits in all git history). ~17 tests fail with
`Os { code: 3, NotFound }`.

```yaml
id:          DEF-VENDOR-FIXTURES
contract:    [DEF-VENDOR-FIXTURES]
class:       mechanical
crates:      [truck-meshalgo, truck-stepio]
depends_on:  []
write_allow:
  - vendor/truck/resources/
  - vendor/truck/truck-stepio/tests/
  - vendor/truck/truck-meshalgo/tests/
read_allow:
  - vendor/truck/truck-stepio/tests/
  - vendor/truck/truck-meshalgo/tests/
  - vendor/truck/truck-topology/src/compress.rs
tests_required:
  - meshalgo tessellation suite reads and tessellates the fixtures
  - stepio output/io suites parse the fixtures
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'resources/shape' vendor/truck/truck-meshalgo/tests/tessellation/triangulation.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'resources/shape' vendor/truck/truck-stepio/tests/output/topology.rs"}
  - {id: A3, expect: 0, cmd: "ls vendor/truck/resources/shape 2>/dev/null | wc -l"}
budget:      {turns: 45, ctx_tokens: 160000}
```

## Problem

Required fixtures (union of both suites' lists):
meshalgo `SHAPE_JSONS`: bottle, punched-cube, torus-punched-cube, sphere,
torus (triangulation.rs:6-12). stepio `SOLID_JSONS`: bottle, punched-cube,
torus-punched-cube, cube-in-cube (tests/output/topology.rs:6-11). Union =
6 files. The tests deserialize `truck_topology::compress::CompressedSolid<
Point3, Curve, Surface>` (serde) then triangulate/parse/display.

## Scope decisions

1. GENERATE with the landed kernel, do not fetch the network: write a small
   generator (a normal unit test behind `#[ignore]` plus a documented
   command, or a tiny rust bin under `vendor/truck/resources/gen/` with its
   own README) that CONSTRUCTS each solid through the certified facade /
   truck-modeling primitives and serializes `CompressedSolid` with
   serde_json. Each fixture must satisfy the reading test's own asserts
   (closed shell, manifold, the shape its name names: bottle = revolved
   profile; punched-cube = cube minus through-cylinder; torus-punched-cube =
   torus-punched cube; sphere; torus; cube-in-cube = nested shells).
2. If a test requires OCCT-side reference data absent from the tree (e.g.
   `triangulation::compare_occt_mesh` compares against an OCCT-produced
   mesh), do NOT fake it: leave that test's status unchanged and record it
   in RESULT as still-environmental with the reason. Do not weaken any
   assertion.
3. Committed fixtures are DATA: deterministic serialization, one commit,
   provenance documented in `vendor/truck/resources/shape/README.md`
   (generator name, commit, construction parameters).
4. `healing::tests::step_import` (truck-shapeops) needs an absent STEP
   file â€” out of scope unless trivially satisfied the same way; record
   either way.

## Done when

```
cargo test -p truck-meshalgo --test tessellation
cargo test -p truck-stepio --lib
cargo test -p truck-stepio --test output
```

green EXCEPT tests recorded still-environmental in RESULT with reasons.

## Forbidden

Weakening assertions. Network fetches. Touching production code. Changing
the reading tests' expectations.

## Stop conditions

- A fixture cannot be constructed through the certified kernel (the shape
  needs a capability outside the landed envelope) â†’ SPEC_GAP naming it.
- More than 2 of the reading tests remain unsatisfiable â†’ BLOCKED with the
  inventory.

## Finish by writing RESULT.json at the WORKTREE ROOT (then COMMIT first)

Commit subject: `test(vendor): generate the resources/shape fixture set through the landed kernel (DEF-VENDOR-FIXTURES)`.
