# resources/shape

Serialized `CompressedSolid<Point3, Curve, Surface>` JSON fixtures read by the
upstream-derived reading suites:

- `truck-meshalgo/tests/tessellation/triangulation.rs` (`SHAPE_JSONS` +
  `large-torus.json`)
- `truck-stepio/tests/output/topology.rs` (`SOLID_JSONS`)
- `truck-stepio/tests/output/screenshot.rs` (`cube.json`)

The tests deserialize each file into both `truck_topology::compress::
CompressedSolid<Point3, Curve, Surface>` and (through the same JSON) a
truck-modeling `Solid`, then triangulate / STEP-parse / compare. Because
`Solid` serializes as its compressed form (`truck-topology/src/compress.rs`,
`Serialize for Solid` -> `self.compress().serialize(..)`), one JSON serves both
types.

## Provenance

Work packet **DEF-VENDOR-FIXTURES** (wave commit `b5491db2`). The fixture
directory was never vendored in this tree before this packet (zero
`--diff-filter=A` hits in all history), so the two suites had never run here.

**Generator:** the committed truck-modeling example programs under
`vendor/truck/truck-modeling/examples/` (last touched at commit `da72cd5`,
"build: vendor truck as the BG- generation kernel"), run from this directory.
Each example constructs its solid through the certified kernel / truck-modeling
`builder` primitives and serializes it with `serde_json::to_vec_pretty`, which
emits the compressed solid form.

**Documented (re)generation command** (one line per fixture, from this
directory, workspace root `Cargo.toml`):

```console
cargo run -p truck-modeling --example bottle
cargo run -p truck-modeling --example cube
cargo run -p truck-modeling --example cube-in-cube
cargo run -p truck-modeling --example punched-cube
cargo run -p truck-modeling --example torus-punched-cube
cargo run -p truck-modeling --example sphere
cargo run -p truck-modeling --example torus
cargo run -p truck-modeling --example torus 100.0 10.0 large-torus.json
```

Each example writes `<name>.json` into the current working directory; `torus`
additionally accepts `[radius0 radius1 [filename]]` so `large-torus.json` is the
same generator with construction parameters (major radius 100.0, minor radius
10.0). A rebuild of the produced files is byte-identical (verified by SHA-256
before/after regeneration); serialization is deterministic because the
`CompressDirector` sorts vertex/edge tables by insertion index and the builders
iterate faces in stored order.

## Files, generators, construction parameters

| fixture | generator example | construction |
| --- | --- | --- |
| `bottle.json` | `bottle.rs` | bottle(height 1.4, width 1.0, thickness 0.6): outer and inner pill bodies from homotopy of circle arcs + linear sweep, neck cylinder (revolved), grue-joined into one closed boundary shell |
| `cube.json` | `cube.rs` | unit cube in [-0.5, 0.5]^3 by three sweeps |
| `cube-in-cube.json` | `cube-in-cube.rs` | outer unit cube [0,1]^3 plus inner cube [0.25, 0.75]^3 -> two closed boundary shells |
| `punched-cube.json` | `punched-cube.rs` | unit cube in [-0.5, 0.5]^3 with a through-hole along the z axis whose rim is a rounded square/diamond of four quarter-circle arcs (radius 0.2, centre on the axis) |
| `torus-punched-cube.json` | `torus-punched-cube.rs` | cube [0,1]^3 with a quarter-torus (minor radius 0.25) whose two open rims are glued into the -y and -x cube faces (see the committed generator for exact placement) |
| `sphere.json` | `sphere.rs` | sphere radius 0.5 centred at the origin (revolved half-circle + closing sweep) |
| `torus.json` | `torus.rs` (defaults) | smooth torus, major radius 0.75, minor radius 0.25 |
| `large-torus.json` | `torus.rs` (args) | smooth torus, major radius 100.0, minor radius 10.0, for `triangulation::large_number_meshing` |

Every fixture deserializes into a truck-modeling `Solid` (validated closed,
connected, closed-manifold boundary shells by `Solid::try_new`) and into
`CompressedSolid`, and round-trips through truck-stepio STEP emission and the
ruststep parser (`cargo test -p truck-stepio --test output` is green, including
the `screenshot::cube` golden string).

## Out of scope / not vendored here

- `../obj/by_occt.obj` (OCCT-produced reference mesh): the reading tests
  `triangulation::compare_occt_mesh` / `compare_occt_mesh_csolid` need this
  OCCT-side reference data, which is absent from the tree. Per scope decision 2
  of DEF-VENDOR-FIXTURES it is not faked; those tests are recorded
  still-environmental in the packet RESULT.
- `truck-shapeops healing::tests::step_import` needs an absent STEP file and is
  out of scope; recorded in the packet RESULT.
