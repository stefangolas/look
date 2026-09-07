# TTC model code-path audit — what actually executes for the corpus models (2026-09-07)

Question audited: for the three text-to-cad models (F1, Falcon-Heavy,
Hypercar), what exact algorithm code paths run, today and after PB-011?

## Headline finding: three execution regimes, and the corpus→kernel
## geometry executor does not exist yet

1. **The OCC baseline regime (runs today, green).** `corpus/ttc/door.py`
   spawns one Python process per manifest row and executes the corpus's
   GENUINE build123d script against real OCC
   (`install_cadgen_alias` answers `cadgen.build123d` with build123d
   itself). Outputs: STL + geometry-facts JSON = the recorded references.
   `truck123d/compat/runner.rs::run_one` drives exactly this door per
   manifest row; `compare_against_reference` adjudicates the facts.
   **The kernel is not in this loop at all** — it is the differential
   oracle baseline.
2. **The facade classifier regime (runs today, green).**
   `truck123d/src/facade.rs::run_facade` consumes a `FacadeTable` (the
   ordered op log a builder session records — the corpus-shaped op
   vocabulary, facade.rs:299-455) and computes a deterministic
   `FacadeReport`. Verified by reading the execution arms
   (facade.rs:495-600): the match tracks carrier classes
   (`apply_solid_class`), the constructive flag, boolean events
   (`Mode` rows arm the next solid op), selection state and exports.
   **It executes no kernel geometry** — there is no
   `truck_modeling`/`truck_shapeops` call anywhere in the arms; the
   dispatch is class algebra + typed refusals
   (`Refusal::Empty` on selection misuse, `NonCanonicalCarrier` on
   `ExportStep` for constructive parts per TR-NRB-001).
   Runtime evidence: `pb_op_matrix` (7/7 green in the 2026-09-07
   battery) drives `run_facade` probes over every landed cell; the
   `ttc_harness` tests (determinism, staged skips) are green.
3. **The kernel geometry regime (showcase-proven, corpus-pending).** Real
   kernel execution today is the showcase path: the waterslide/amphora/
   teapot tables drive `truck_modeling` builders, the boolean funnel
   (`truck_shapeops::boolean`), and GLB/tessellation emission — exercised
   by the showcases suites. The corpus equivalent — replaying a corpus
   script's op sequence through THIS machinery — is precisely
   PB-011's scope (route `Mode`-armed solid ops on non-canonical carriers
   into the certified entry via `sweep_lift`, lift the skips), and the
   Python drop-in (pyo3) is booked behind it.

**Consequence for the "will the models ship" question:** the missing piece
between "skips lifted" and "models run through the kernel" is the executor
binding — the layer that takes a corpus op sequence and calls the kernel
arms instead of the classifier. PB-011 owns the routing inside the facade;
the corpus→facade replay layer is the next booking after it.

## Per-op code-path map (corpus op → current behavior → post-PB-011 kernel chain)

Ops from the corpus census (PB-010, 93 site rows), mapped through the
facade vocabulary:

| Corpus op | Facade arm | Today | Post-PB-011 kernel chain |
|---|---|---|---|
| `Box/Cylinder/Sphere/Torus` | primitives (facade.rs:530-536) | classifier: canonical, solid_ops++ | `truck_modeling` primitive constructors (S3) |
| `Polygon/Polyline/Circle` | sketch carriers (525) | classifier: canonical profile | profile authoring → `arrange` (S4/S5) |
| `Spline` | 518: marks profile Spline → constructive | classifier only | spline authoring entry (PB-002) |
| `make_face` | 552 | classifier only | `make_face` (certified planar face) |
| `extrude(amount)` | 537 | classifier only | `truck_modeling::extrude` (+taper/until) |
| `revolve(angle)` | 538 | classifier only | `truck_modeling::revolve` (z-axis, line/circle profiles) |
| `revolve_arc(arc,start)` | 539 | classifier only (PB-014 landed the op) | trimmed-shell construction over the full revolve |
| `sweep` / `loft` | 504-516: constructive, Swept class | classifier only | `spine_sweep`/`facet_sweep` + loft substrate |
| `Mode{add/sub/intersect}` + next solid op | 565-578: arms the boolean | classifier records a `SweptBooleanEvent`; geometry refuses `NonCanonicalCarrier` | **the G1 flip**: `boolean_op` routes into the certified funnel (`solver_entry` ← `sweep_lift` adapters) |
| `fillet/chamfer` after selectors | 579-584 | classifier only | `truck_shapeops` fillet/chamfer (edge-selector vocabulary) |
| `mirror(axis)` | 559 | classifier only | placement op (kernel-side mirror) |
| `export_stl` | 585 | recorded | kernel tessellation (`truck-meshalgo`) → `truck-polymesh` STL |
| `export_step` | 595 | typed refusal for constructive | unchanged (TR-NRB-001 boundary) |

## Per-model verdict surface (from the manifest + SKIPS + census)

- **Falcon-Heavy: 13 canonical rows + cutaway (staged).** Canonical rows
  run the OCC door green with recorded references; their op mix
  (revolve/sweep/loft/primitives/compound) is the landed S3/S5/S6 surface.
  Cutaway = partial-arc revolves (PB-014 landed the op) — its lift keys on
  the door run atop PB-014.
- **F1: 21 staged rows.** Every one boolean-composes swept carriers
  (`surfaces.cut`/`_fuse`/`fuse_all`) — the G1 class. Post-PB-011 each
  routes into the funnel; the certified envelope (spline×analytic contact,
  sweep-lift adapters) covers the canonical-tool cases first; loft×loft
  pairs are the deep end.
- **Hypercar: wheels canonical; 12 systems staged** — the lofted master
  body shell, canopy/glass shells, structural lofts; deepest envelope
  exposure (loft×loft booleans throughout).

## The gap list this audit adds to the booking record

1. **The corpus→kernel executor binding** (post-PB-011): replay a lifted
   row's op sequence through the kernel arms (the facade classifier
   becomes the pre-flight; a geometry executor consumes the same table).
   Without it, lifted rows still execute only against OCC.
2. **`run_facade` is the pre-flight, not the flight**: PB-011's "expose
   AND route" must land BOTH the classifier's boolean-event routing AND
   the geometry call behind it — the current arms prove the classification
   contract only.
3. **The OCC door stays**: it is the differential oracle; the runtime
   audit's comparison protocol (geometry facts vs recorded references)
   is exactly how lifted rows get adjudicated after PB-011.
