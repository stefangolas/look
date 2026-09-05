# Truck123d Python Bridge — Build Spec (PB program)

**Status:** proposed packet program, written this session. Scope: a
build123d-shaped Python API over the landed kernel, **scoped to the three
showcase models** (waterslide, teapot, amphora). This spec covers
everything needed for a drop-in-flavored competitor *except booleans* —
booleans root in
[`CERTIFIED_INTERACTION_ENGINE_SPEC.md`](CERTIFIED_INTERACTION_ENGINE_SPEC.md)
(the BIE program) and are a deliberate non-goal here.

Companion docs: `CONSTRUCTIVE_GEOMETRY_PLAN.md` (sweep core),
`CERTIFIED_CONSTRUCTION_BUILD_SPEC.md` (loft/blend/shell — consumed, not
duplicated), `BREP_GENERATION_API.md` (landed facade). House rule: every
tree claim below was verified by command this session; re-derive before
quoting in a packet.

## 1. Framing

The bridge is a **naming + semantics table with zero geometric content**,
exactly the doctrine the landed Rust facade already follows
(`truck-shapeops/src/facade.rs` header: "Python selectors are NOT part of
the facade (booked with the pyo3 program)"). This program is that booking.
Three layers, each independently testable:

```
Python (build123d-shaped: BuildPart/BuildSketch sugar, selectors, primitives)
   ↕  pyo3 (Outcome → typed exception; tables → dataclasses)
Rust client layer (selectors-lite, sketch arcs, concave caps, assembly emitter)
   ↕  (existing)
kernel (facade + constructive + certified, landed)
```

**Portability contract:** every model is a serde data table
(`showcases/tables/*.json`) interpreted by a thin builder. The Python side
consumes the same tables. A model built from Python must produce a
byte-identical report JSON to the same table built from Rust — this is the
drop-in claim made testable.

## 2. Substrate audit (verified by command this session)

| Need | Status | Anchor |
|---|---|---|
| Fluent selectors over live topology | **ABSENT** — the facade doc explicitly books selectors to this program | `truck-shapeops/src/facade.rs:10`; iteration substrate EXISTS: `face_iter`, `absolute_boundaries`, edge census pattern (`spine_sweep_conformance.rs:52`), `solid_bounding_box` (`cad.rs`) |
| Edge identity machinery | **LANDED** — reuse, do not duplicate | `truck-topology/src/entity_id.rs`: `EntityId`, `Op`, `OpKind`, and a `Selector` primitive (`sel(base, selector)` :176) — PB-001 must consume this vocabulary |
| Sketch arcs in profiles | **PARTIAL** — `arrange.rs` `Carrier2D` is `Line | CircleCarrier` (:682), and `CircleCarrier` carries a parameter range `(c.t0, c.t1)` :690 — trimmed circles (arcs) are representable; what's missing is the authoring level (3-point/tangent arc constructors) and arc×line/arc×arc Region2 cells | `truck-geometry/src/arrange.rs:682-708` |
| Concave caps on the facet path | **ABSENT** — `ring_is_convex` gate refuses | `truck-modeling/src/facet_sweep.rs:164` |
| Per-face CDT for caps | **LANDED** — the ledger entry point reuses per-face CDT internals | `docs/CONSTRUCTIVE_GEOMETRY_PLAN.md` §3.4 (CG-005 landed) |
| pyo3 / any Python binding | **ABSENT** — greenfield | no `pyo3`/`pyclass` occurrence in the workspace |
| Build123d-named facade entries | **LANDED** (Rust) | `truck-shapeops/src/facade.rs`: extrude/extrude_vector/revolve/fillet/chamfer/mirror/rotate/scale/translate/section/split/bounding_box/boolean_op/make_face/make_hull |
| Constructive sweeps | **LANDED** | `truck-modeling/src/{spine_sweep,facet_sweep}.rs` |
| Loft/Gordon/shell/blend ports | **BOOKED, RUNNING** (CC program) | consumed via `showcases/src/cc_ports.rs` anti-corruption layer |
| Assembly emission (multi-solid, no booleans) | **PARTIAL** | `truck-assembly` (STEP assembly graph resolver, used by `look`) exists; a Rust-side assembly-emitter entry over multiple `Solid`s is small |
| STEP out of constructive surfaces | **ABSENT, booked elsewhere** | TR-NRB-001; `truck-stepio/out` typed-refuses `SpineFrameSurface` (`out/geometry.rs`, test `step_out_refuses_spine_frame_variants_typed`). Non-goal here; STL/OBJ renders |
| Defects pinned by the showcase battery | **LANDED witnesses** | `docs/defects/DEFECT_INDEX.md` rows `ORI-FRAME-*`, `SEM-FACET-*`, `NUM-INTERPOLE-OVERSHOOT-001` — the bridge inherits the stable-regime constraints (spine interpolation n ≲ 48) and must surface the typed refusals, not hide them |

## 3. Packet plan

Packet = unit of work sized to ~50% of a worker context. Write-set
disjointness per the scheduler's law; merge only along same-module chains.

### Phase A — Rust client layer (no Python)

| Packet | Class | Content | Write set | Depends |
|---|---|---|---|---|
| `PB-000-CONTRACT` | design | Freeze the Python-facing API table (build123d name → Rust entry), the refusal→exception mapping table, the table-schema version (`tables/*.json` v1), and the byte-determinism contract (same table + same kernel rev → identical report JSON). Book mapping rows into `docs/CERTIFICATE_MAPPING.md` if any new evidence variant appears (target: none) | `docs/TRUCK123D_PY_BRIDGE_SPEC.md` amendments, `showcases/src/cc_ports.rs` doc freeze | — |
| `PB-001-SELECTORS` | mechanical | Scoped selector layer in a new `truck-modeling::selectors` module: `FaceRef`/`EdgeRef` iteration over a `Solid`, per-face centroid + AABB (fan-sampled, reusing the harness method), `sort_by_axis`, `group_by_axis`, `filter_by_plane`, `take`/`last`, and resolution into `BlendSpec`-compatible edge names (endpoint pairs for straight edges, canonical rim for circles). Consumes `entity_id.rs`'s `Selector` for identity. Difficulty 2/10 | new module + tests | 000 |
| `PB-002-SKETCH-ARCS` | design | Arc authoring + arrangement: `arc_three_point`, `arc_radius` constructors producing trimmed `CircleCarrier`s; Region2 cells for arc×line and arc×arc reusing the landed analytic intersections; profile assembly accepting mixed line/arc loops. The teapot silhouette switches to arcs. Difficulty 3–4/10 (the Region2 cells are wiring; the loop assembly needs the endpoint-pairing care of P3) | `truck-geometry/src/arrange.rs` additive + new `sketch.rs` | 000 |
| `PB-003-CONCAVE-CAPS` | mechanical | Facet backend: non-convex cap rings triangulate through the per-face CDT path instead of fan+convexity-gate; the convexity fast path stays for convex rings (bit-identical behavior there — V5 identity guard). The U-chute negative test inverts to a positive test. Difficulty 3/10 | `truck-modeling/src/facet_sweep.rs` cap section + tests | 000 |

### Phase B — Python bridge

| Packet | Class | Content | Write set | Depends |
|---|---|---|---|---|
| `PB-004-PYO3-CORE` | mechanical | New crate `truck123d/` (pyo3): module init, `Outcome` → typed exception hierarchy (`Refused`, `Unresolved`, with `EnvelopeCase`/witness payload), `Budget`/verdict marshaling, GIL policy (all kernel calls release GIL; no kernel type crosses the boundary except via opaque handles), serde round-trip of tables. Difficulty 3/10 | new crate + Cargo workspace member | 000 |
| `PB-005-PYTHON-FACADE` | design | build123d-shaped Python: `BuildPart`/`BuildSketch` context managers (thin sugar over data tables — the Python side EDITS TABLES, then submits; statefulness is Python-side only), `Mode` algebra sugar, primitives (`Box`, `Cylinder`, `Polygon`, `Polyline`), `extrude`/`revolve`/`sweep`/`loft`/`fillet`/`chamfer` entry points, scoped selectors exposed fluently, `export_stl`/`export_step`. Difficulty 3/10 — the design constraint is that nothing computes in Python | `truck123d/src/*.py` + pyo3 surface | 001, 004 |
| `PB-006-ASSEMBLY` | mechanical | Multi-solid assembly emission: N solids + intended-contact/evidence list → STEP assembly via `truck-assembly`; the teapot ships as body+spout+handle with recorded contact intents until BIE lands. Difficulty 2/10 | `truck-modeling` additive or `truck123d` client | 004 |
| `PB-007-CONFORMANCE` | design | The three showcase scripts **in Python** as the conformance battery: each produces report JSON byte-equal to the Rust run of the same table; refusal battery (the typed-refusal tests of `battery_construction.rs`) mirrored as pytest `raises`; determinism test. Gate for the whole program | `truck123d/tests/` | 005, 006 |

## 4. Dependency graph

```text
PB-000 ─┬→ PB-001 ─┐
        ├→ PB-002  ├→ PB-005 ─┐
        ├→ PB-003  │           ├→ PB-007
        └→ PB-004 ─┴→ PB-006 ──┘
```

PB-001/002/003 are mutually write-disjoint and parallel-eligible after 000.
PB-004 is independent of A-phase packets.

## 5. Gates and invariants

- **Zero geometric content in Python and in the bridge crate**: every
  operation dispatches to a landed kernel entry or refuses; a reviewer can
  diff the Rust facade against the Python surface and find no math.
- **Byte-determinism**: same table, same kernel rev → identical report JSON
  from Rust and Python (PB-007 is the executable form).
- **Refusal fidelity**: every typed kernel refusal surfaces as a typed
  Python exception carrying witness payloads; nothing degrades to a bare
  `Exception` or a silent default.
- **Stable-regime inheritance**: the spine-interpolation bound
  (NUM-INTERPOLE-OVERSHOOT-001) and the facet convexity/caps behavior are
  visible in the Python docs as kernel-refusal semantics, not papered over.
- **Naming conformance**: no restricted alternative names (`sweep_frame`,
  `loft_ribs`) at the Python layer — build123d names (`sweep`, `loft`,
  `fillet`, `offset`) with typed refusals where geometry is outside the
  envelope, per the facade doctrine.
- Existing entry points stay bit-identical (PB-003's convex fast path is
  the sensitive one — V5 identity guard applies).

## 6. Estimates

| Phase | Packets | LOC (code+tests) | Sessions |
|---|---|---|---|
| A (Rust client) | 4 | ~3k | 2–3 |
| B (Python bridge) | 4 | ~3.5k | 2–3 |
| **Total** | 8 | **~6.5k** | **4–6** |

The program is deliberately shallow: its only design-class packets are
PB-000, PB-002, PB-005, PB-007. Everything else is mechanical and
parallel-eligible. It can start immediately — nothing in it waits on CC,
and its outputs (selectors, arcs, concave caps) are consumed by the
showcases whether or not the Python layer ships first.

## 7. Non-goals (explicit)

- **Booleans on non-canonical carriers** — root theory is
  `CERTIFIED_INTERACTION_ENGINE_SPEC.md` (BIE program). Until then the
  teapot ships as an assembly (PB-006) and the pool as a sweep terminus.
- **NURBS conversion / STEP out of constructive surfaces** (TR-NRB-001) —
  separate follow-on; STL/OBJ covers render and print.
- **Shell/offset as Python operations** — CC-021..026; the amphora wall is
  authored into the silhouette.
- **General workplanes / arbitrary-axis revolve** — P9/P10 conjugation
  substrate; not exercised by the three models (all are z-canonical).
- **mesh Booleans, fuzzy intent recovery, ShapeFix analogues** — out of
  scope indefinitely; the typed-refusal doctrine is the product.
