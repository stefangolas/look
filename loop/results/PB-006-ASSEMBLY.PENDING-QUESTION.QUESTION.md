# QUESTION — PB-006-ASSEMBLY (SPEC_GAP)

Status: `SPEC_GAP`. The packet cannot be executed as written inside the
`truck123d` crate. Nothing was committed; the tree is left at the dispatch
HEAD with only `PACKET.md`/`CONTEXT.md` untracked (probe reverted).

## Anchors (all verified at dispatch HEAD, before any edit)

| id | expect | found |
|---|---|---|
| A1 `truck123d/src/python.rs` `pub struct PyTruckSolid` | 1 | 1 |
| A2 `vendor/truck/truck-assembly/src/assy.rs` `pub fn ` | 10 | 10 |
| A3 `showcases/src/teapot.rs` `fn main` or `pub fn` | ≥1 | 8 |
| A4 `truck123d/src/assembly_emit.rs` anything | 0 before | 0 (absent) |

## What blocks DONE — the missing pieces, in dependency order

### 1. The bridge crate cannot name or reach any assembly/STEP/solid crate.

PB-006 scope decision 1 places the emission in `truck123d/src/assembly_emit.rs`,
built on the landed `truck-assembly` DAG/assy surface. `truck123d/Cargo.toml`
(pinned by PB-004, `docs/TRUCK123D_PY_BRIDGE_SPEC.md` §5 "zero geometric
content in the bridge crate") declares only `pyo3`, `serde`, `serde_json`,
`truck-base` (+ `pyo3` dev). It has **no** dependency on `truck-assembly`,
`truck-modeling`, `truck-topology`, or `truck-stepio`, so the emission module
cannot `use` any of them. Adding the edges requires editing
`truck123d/Cargo.toml` and ticking `Cargo.lock` — both outside `write_allow`
(`write_allow` = the three files only; `Forbidden` names `Cargo.lock`
explicitly, and `vendor/**` is read-only). PB-004's write_allow included
`Cargo.toml`/`Cargo.lock` for exactly this reason; PB-006's does not.

Probe (scoped `cargo check -p truck123d`, through the PATH queue shim), with a
faithful `assembly_emit.rs` skeleton wired as `mod assembly_emit;`:

```
error[E0433]: cannot find module or crate `truck_assembly` in this scope
  --> truck123d\src\assembly_emit.rs:12:5
   = help: ... use `cargo add truck_assembly` to add it to your `Cargo.toml`

error[E0432]: unresolved import `truck_modeling`
  --> truck123d\src\assembly_emit.rs:13:5
   = help: ... use `cargo add truck_modeling` to add it to your `Cargo.toml`
```

Baseline `cargo check -p truck123d` at dispatch HEAD is green; the probe is the
only failure and it was reverted.

### 2. There is no reachable representation of "a solid" in the write set.

Test 1 ("3 solids → a STEP assembly") and the teapot fixture both need to hand
real kernel `Solid` values to the emitter. The only Solid type in the tree is
the topology/modeling `Solid`; `truck123d` cannot name it (gap 1) and its
crate doctrine forbids kernel geometry crossing the boundary. `PyTruckSolid`
(`python.rs:23`) is a READ-ONLY file in this packet and still carries only a
`kind: String`; `python.rs:19` books "the concrete kernel Solid" to PB-006, but
this packet's `write_allow` does not include `python.rs`, so even the intended
handle extension is out of reach.

### 3. The teapot geometry path is unreachable.

The teapot body/spout/handle builders live in `showcases/src/teapot.rs`
(read-only, read_allow A3), built on `truck-modeling`/`truck-geometry`/
`truck-shapeops` (`body_solid`, `realize_tube`, `spine_sweep`, ...). The
required test `teapot_body_spout_handle_assembles` runs in
`truck123d/tests/pb_assembly.rs`, which can link nothing but `truck123d`'s own
dependencies. No path from the test target to the fixture builders or to the
modeling kernels exists.

### 4. Stop condition 2 fires: truck-assembly's emission cannot represent an
intended-contact row.

The truck-side STEP emission surface (`truck-stepio out/assy.rs`, which is
what turns a `truck_assembly::assy::Assembly` into a STEP file) carries node
attributes only as `PartAttrs { id, name, description }`
(`truck-stepio/src/common.rs:3`) and writes each node's shape as iterable
`DisplayByStep` solids. There is **no record** on that surface that can carry a
structured contact intent — `{ kind: INTENT, part_a, part_b }` — as typed
evidence on a node: it could at most be flattened into the free-text
`description`. The missing record is a *structured intended-contact evidence
row typed INTENT on the assembly node* (recording, never certifying, per scope
decision 2). It exists nowhere in the packet, the spec (§ packet-table row
PB-006 only), or the landed `truck-base` evidence vocabulary (marshal.rs lists
the full landed set: `EnvelopeCase`, `UnresolvedWitness`, `Prop`, `Truth`,
`Method`, `CollapseReason` — none is an intent/evidence-row record). The same
gap blocks "round-trip read-back": no STEP reader that reconstructs nodes plus
structured rows is reachable from `truck123d` (truck-stepio has no assembly
read-back here, and `look`'s reader is a separate crate).

## Readings considered

- **(a) Manifest authoring slip.** PB-004's packet explicitly listed
  `Cargo.toml`/`Cargo.lock` in `write_allow` and shipped the dependency edge
  that way; PB-006 presumes a `truck-assembly` edge in `truck123d` that was
  never landed. The emission then simply cannot compile (probe above). This is
  the leading reading.
- **(b) Purely structural, dependency-free emission.** `assembly_emit.rs`
  would hand-write AP203 product/occurrence text over client-defined part and
  evidence-row types with no truck crate at all. This contradicts the spec row
  ("via `truck-assembly`"), makes a "difficulty 2/10" mechanical packet into a
  full STEP writer + parser reimplementation, still needs the INTENT record
  shape (missing, see 4), and leaves "3 solids" without any solid to pass.

## Resolution the owner can choose

1. Grow the packet's `write_allow` to include `truck123d/Cargo.toml` (+ a
   `Cargo.lock` tick) so `truck123d` can depend on `truck-assembly` (and, if
   the teapot fixture and STEP read-back are to be exercised from the
   `truck123d` test target, the modeling/stepio/topology edges and a decision
   on where the teapot geometry builders and the STEP reader live), **and**
   define the intended-contact evidence-row record (kind INTENT) that the
   emission writes on assembly nodes; or
2. Rebook PB-006's write set to a crate that already owns the
   truck-assembly/stepio surface (the spec row's "`truck-modeling` additive"
   alternative) and keep `truck123d` for the pure client/report side; or
3. Amend the packet to specify the client-side data shapes (solid
   representation, INTENT evidence-row record, report shape, teapot node
   census) under a dependency-free structural emission, explicitly overriding
   the "via truck-assembly" spec wording.

No test names were landed; `tests_added` is 0 because nothing compiles. Per the
packet's stop conditions no further research or implementation was attempted.
