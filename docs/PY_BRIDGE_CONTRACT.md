# Truck123d Python Bridge — Contract (PB-000-CONTRACT)

**Status:** FROZEN by work packet PB-000-CONTRACT. Anchors measured 2026-09-05
at `90672a7` (A1 facade `^pub fn` = 16, A2 `Selector` enum = 1, A3 `sel()` = 1,
A4 `showcases` in workspace `Cargo.toml` = 1, A5 table JSONs = 3). The
workspace manifest has **zero pyo3 dependencies** at freeze time (the two prose
mentions are doc comments); PB-004 introduces the only binding.

This document freezes the four contracts later PB packets (PB-001..007) type
against. It is written from the landed surface only; where a contract row
cannot be stated without computing geometry, that would be a SPEC_GAP — there
are none here. The build spec for the program is
`docs/TRUCK123D_PY_BRIDGE_SPEC.md`; this document is the normative contract it
points at.

Contents:

1. [API mapping table](#1-api-mapping-table)
2. [Refusal → exception mapping](#2-refusal--exception-mapping)
3. [Table schema v1](#3-table-schema-v1)
4. [Byte-determinism contract](#4-byte-determinism-contract)

---

## 1. API mapping table

**Doctrine** (spec §1, §5): the bridge is a naming + semantics table with zero
geometric content. Every build123d-facing name dispatches to a landed Rust
entry or refuses typed. There are **25 rows** total: the 16 landed
`truck_shapeops::facade` entries (verbatim) plus **9** CC-port forwards the
showcases consume through `showcases::cc_ports` (the trait method set of the
anti-corruption layer; see the module freeze below). A name with no landed
entry is a row that says "refuses typed" with the refusal case named — the 9
CC-port rows are exactly that today (every one refuses
`UnsupportedEnvelope(ContactReductionDeferred)` from the `LandedPorts` stub).

**Data-shape encoding legend** (used by the "argument table-shape" column; the
Python layer edits data tables, never geometry):

| Landed type | Table encoding |
|---|---|
| `f64` | JSON number |
| `bool` | JSON boolean |
| `usize` | JSON integer |
| `Point3` / `Vector3` | JSON `[x, y, z]` of finite numbers |
| `Plane` | JSON object `{"origin": [x,y,z], "normal": [x,y,z]}` |
| `Curve` | a canonical carrier row (Line / canonical Curve datum), authored on the Rust client side from table data |
| `Arrangement` | produced by the landed `arrange` step over the profile's `Curve[]`; never authored directly |
| `Solid` | the result of an earlier operation in the session (opaque handle) |
| `Mode` | JSON string `"union"` \| `"subtract"` \| `"intersect"` (`Add`/`Subtract`/`Intersect`) |
| `BlendSpec[]` / `ChamferSpec[]` | JSON array of spec rows (named-edge data, resolved by PB-001 selectors) |
| `Budget` | not a table input: per-call kernel budget, owned by the bridge |
| `Wire[]` / `RibWire[]` / `(f64,f64,f64)[]` / `RadiusLaw` | table rows exactly as the CC ports take them |

### 1.1 The landed facade surface (16 rows, verbatim)

Every row's Rust entry path is `truck_shapeops::facade::<entry>`; the "landed
composition" (the kernel entry the facade forwards to) is documented in that
module's naming table at `vendor/truck/truck-shapeops/src/facade.rs`. All 16
return `Outcome` (spec §4), so every one can answer any `Refusal` variant of
§2 that its forwarded entry produces; the facade performs **no** refusal
coercion and no silent fallback (D4/D5). "Refusal cases" below lists only the
cases observable at the facade layer itself; everything else is "the forwarded
entry's typed refusals" (full taxonomy in §2).

| # | build123d name | Rust entry path | argument table-shape | refusal cases | stable-regime notes |
|---|---|---|---|---|---|
| 1 | `extrude` | `truck_shapeops::facade::extrude` → `truck_modeling::extrude::extrude_profile` | `(profile: Curve[], arrangement: Arrangement, height: f64)` | forwarded entry's typed refusals; budget-free | profile must be a planar arrangement; no z-canonical restriction beyond the landed entry's envelope |
| 2 | `extrude_vector` | `truck_shapeops::facade::extrude_vector` → `truck_modeling::extrude::extrude_profile_vector` | `(profile: Curve[], arrangement: Arrangement, dir: [x,y,z], both: bool)` | forwarded entry's typed refusals; budget-free | `both: true` spans `[-dir, +dir]` |
| 3 | `revolve` | `truck_shapeops::facade::revolve` → `truck_modeling::revolve::revolve_profile` | `(profile: Curve[], arrangement: Arrangement, angle: f64)` | forwarded entry's typed refusals; budget-free | revolved about the z-axis; profile region must be at `x > 0` in the working plane per the landed `revolve_p5` pattern |
| 4 | `fillet` | `truck_shapeops::facade::fillet` → `rewrite::fillet` + `rewrite::fillet_circle` (grouped `BlendSpec` batches) | `(solid: Solid, specs: BlendSpec[], budget)` | `Refusal::Empty` when `specs` is empty (facade-level, documented); otherwise the forwarded entries' typed refusals; budget spend can terminate `NumericallyUnresolved` | dispatch is SEQUENTIAL (P12 D4): `Straight` group first, then `Circular` group on the result |
| 5 | `chamfer` | `truck_shapeops::facade::chamfer` → `rewrite::chamfer` | `(solid: Solid, specs: ChamferSpec[], budget)` | forwarded entry's typed refusals; budget spend can terminate `NumericallyUnresolved` | straight-edge chamfers only (the `ChamferSpec` domain) |
| 6 | `mirror` | `truck_shapeops::facade::mirror` → `truck_modeling::cad::mirror_solid` | `(solid: Solid, plane: Plane)` | forwarded entry's typed refusals; budget-free | axis-aligned mirror plane |
| 7 | `mirror_about_plane` | `truck_shapeops::facade::mirror_about_plane` → `truck_modeling::cad::mirror_about_plane` | `(solid: Solid, plane_point: [x,y,z], plane_normal: [x,y,z])` | forwarded entry's typed refusals; budget-free | plane through `plane_point` with `plane_normal` |
| 8 | `rotate` | `truck_shapeops::facade::rotate` → `truck_modeling::cad::rotate_solid` | `(solid: Solid, axis_point: [x,y,z], axis_dir: [x,y,z], angle: f64)` | forwarded entry's typed refusals; budget-free | `angle` in radians |
| 9 | `scale` | `truck_shapeops::facade::scale` → `truck_modeling::cad::uniform_scale_solid` | `(solid: Solid, factor: f64)` | forwarded entry's typed refusals; budget-free | uniform scale about the origin |
| 10 | `translate` | `truck_shapeops::facade::translate` → `truck_modeling::cad::translate_solid` | `(solid: Solid, t: [x,y,z])` | forwarded entry's typed refusals; budget-free | rigid translation |
| 11 | `section` | `truck_shapeops::facade::section` → `truck_shapeops::section::section_faces` | `(solid: Solid, plane: Plane, budget)` | forwarded entry's typed refusals; budget spend can terminate `NumericallyUnresolved` | returns the section `Face[]` of the cut |
| 12 | `split` | `truck_shapeops::facade::split` → `truck_shapeops::section::split_by_plane` | `(solid: Solid, plane: Plane, budget)` | forwarded entry's typed refusals; budget spend can terminate `NumericallyUnresolved` | returns `SplitHalves` = the `(plus, minus)` solids |
| 13 | `bounding_box` | `truck_shapeops::facade::bounding_box` → `truck_modeling::cad::solid_bounding_box` | `(solid: Solid, budget)` | forwarded entry's typed refusals; budget spend can terminate `NumericallyUnresolved` | certified AABB |
| 14 | `boolean_op` | `truck_shapeops::facade::boolean_op` → `truck_shapeops::boolean::assemble::boolean` via `Mode` → `BoolOp` | `(a: Solid, mode: Mode, b: Solid, budget)` | forwarded entry's typed refusals; budget spend can terminate `NumericallyUnresolved` | `Add`→`Union`, `Subtract`→`Difference`, `Intersect`→`Intersection`; booleans on non-canonical carriers are a spec §7 non-goal and refuse typed |
| 15 | `make_face` | `truck_shapeops::facade::make_face` → `truck_modeling::cad::make_face` | `(profile: Curve[])` | forwarded entry's typed refusals; budget-free | planar faces on the z = 0 plane |
| 16 | `make_hull` | `truck_shapeops::facade::make_hull` → `truck_modeling::cad::make_hull` | `(points: [x,y,z][])` | forwarded entry's typed refusals; budget-free | 2-D convex hull of z = 0 points as one planar face |

### 1.2 CC-port forwards the showcases consume (9 rows)

Rust entry path for every row: the named trait method of
`showcases::cc_ports::CcPorts`, as implemented today by
`showcases::cc_ports::LandedPorts`. These are the sweep/loft-shaped, certified
entries of the CC program that the three showcase builders already call; the
bridge inherits identical semantics through the same anti-corruption layer.

The build123d-facing Python spellings of these entries are **not** fixed by
this packet: spec §5 naming conformance says the Python layer uses build123d
names (`sweep`, `loft`, `fillet`, `offset`) with typed refusals and never the
restricted internal names (`loft_ribs`, `gordon_ribs`, …). PB-005 assigns the
exact spelling. What PB-000 freezes is the **Rust-entry vocabulary**: a
build123d-facing `loft`-family name must dispatch onto one of the 9 rows below
(or refuse typed), never onto a name that is absent from this table.

"Refusal cases" is precise for all 9 rows **today**: the `LandedPorts` stub
refuses every call with the typed `UnsupportedEnvelope(ContactReductionDeferred)`
(a deferred capability is surfaced, never silently skipped and never
approximated — `showcases/src/cc_ports.rs`). When a CC packet lands a real
entry, that row's refusal set becomes the certified entry's; the row below
names the CC contract it waits on.

| # | CC-port forward (Rust entry) | showcase consumer(s) | argument table-shape | refusal cases (today) | CC contract / notes |
|---|---|---|---|---|---|
| 17 | `CcPorts::loft` | teapot (`loft_spout_variant` probe) | `(stations: Wire[], tol: DirectTolerance)` | `UnsupportedEnvelope(ContactReductionDeferred)` | CC-010..014 loft with declared positional correspondence |
| 18 | `CcPorts::loft_ribs` | amphora (`loft_body`) | `(ribs: RibWire[])` — each rib `{z: f64, ring: Profile2D}`, height-ordered | `UnsupportedEnvelope(ContactReductionDeferred)` | CC-010..014, amphora-facing: loft the rib set into the closed vessel body |
| 19 | `CcPorts::gordon_ribs` | amphora (`gordon_body`) | `(ribs: RibWire[])` | `UnsupportedEnvelope(ContactReductionDeferred)` | CC-015 Gordon boolean-sum blend over the same rib set (A/B against `loft`) |
| 20 | `CcPorts::blend_var_radius` | teapot (`junction_blend_var_radius`) | `(solid: Solid, edge: [[x,y,z],[x,y,z]], law: RadiusLaw)` | `UnsupportedEnvelope(ContactReductionDeferred)` | CC-030/031 variable-radius blend along one named edge |
| 21 | `CcPorts::blend_handle_root` | amphora (`blend_handle_root_var_radius`) | `(ribs: RibWire[], law: RadiusLaw)` | `UnsupportedEnvelope(ContactReductionDeferred)` | CC-030/031, amphora-facing: handle-root blend, radius growing with height per the law |
| 22 | `CcPorts::clear` | waterslide (`clear_chute_tower`) | `(a: Solid, b: Solid, required: f64)` | `UnsupportedEnvelope(ContactReductionDeferred)` | CC-004 certified minimum distance (`ClearCert`: distance/required/margin) |
| 23 | `CcPorts::canal_regularity` | teapot (`canal_regularity_spout_spine`), waterslide (`canal_regularity_chute_spine`) | `(spine: Curve, tube_radius: f64)` | `UnsupportedEnvelope(ContactReductionDeferred)` | CC-025 canal regularity certificate for one spine (`CanalCert`) |
| 24 | `CcPorts::canal_cert` | amphora (`canal_regularity_handle_spine`) | `(handle_points: [x,y,z][], azimuth_deg: f64, tube_radius: f64)` | `UnsupportedEnvelope(ContactReductionDeferred)` | CC-025, amphora-facing: canal regularity rebuilt kernel-side from the same table points |
| 25 | `CcPorts::shell_thickness` | amphora (`certified_shell_wall`) | `(ribs: RibWire[])` | `UnsupportedEnvelope(ContactReductionDeferred)` | CC-023/026 thinnest certified wall (`ThicknessCert`: t_safe/t_focal/d_min_half) |

The trait's two remaining methods — `CcPorts::gordon` (wire-station Gordon,
CC-015) and `CcPorts::certify_shell` (CC-023/026 generic over a live `Solid`) —
are **not** consumed by any current showcase builder and therefore carry no row
here; their method set is frozen by the `cc_ports.rs` module freeze but they
are not bridge-facing until a consumer exercises them.

**Recorded, not consumed here:** the `Selector` vocabulary
(`vendor/truck/truck-topology/src/entity_id.rs`, anchors A2/A3 — the
`Selector` enum and `EntityId::sel`) is the identity substrate PB-001 consumes
to resolve edge/face names into `BlendSpec`/`ChamferSpec` rows; this packet
only records its existence and does not extend the table with it.

**Stable-regime inheritance (applies to the table as a whole, spec §5):** the
spine-interpolation bound of `NUM-INTERPOLE-OVERSHOOT-001` (n ≲ 48 stations) is
a **documented kernel refusal**, not papered over: any bridge surface that
feeds a spline interpolation keeps the bound visible and surfaces the typed
refusal when the input exits the stable regime. The 9 CC-port rows above are
the sweep/loft-shaped surface where that note bites first; the facade rows that
sweep spines (`revolve`, `extrude_vector`, and their `profile`/`arrangement`
inputs) inherit the same doctrine.

---

## 2. Refusal → exception mapping

The landed `Refusal` taxonomy (`vendor/truck/truck-base/src/evidence.rs`) maps
onto a **two-class** Python exception hierarchy. Nothing degrades to a bare
`Exception`; every kernel error is one of the two classes below, and each class
carries the typed payload the kernel produced. (PB-004 implements the classes;
this packet freezes the mapping.)

```text
BaseError                      # bridge base; not raised directly
├── Refused                    # a definitive kernel refusal
└── Unresolved                 # the kernel could not certify within budget
```

**`Refused`** — a kernel typed refusal: the operation was asked something that
is out of the supported envelope, empty, contradictory, collapsed, or past a
margin/backward/forward bound. Carries the `EnvelopeCase`/witness payload
named below as attributes.

**`Unresolved`** — the kernel's three-valued verdict came back "cannot decide":
the operation exhausted its budget without a certified answer. Carries the
spend (the kernel's `κ`, the remaining budget ledger — every counter read as
"spent = starting − remaining") and the witness that says why. The
cell/slope data of the uncertified-deviation witness (`DeviationUncertified`)
rides the same payload. This class is also the raising class for the certified
realization verdict `RealizationVerdict::Inconclusive` when the constructive
realization path surfaces it as terminal (payload per evidence.rs mapping A
rows 1–4: construct error summary, per-realization certificate with
`max_cell_twist`/`extent`, shared-edge pair errors); `Inconclusive` never
converts into success.

| `Refusal` variant (landed) | Python class | payload carried |
|---|---|---|
| `Empty` | `Refused` | none |
| `UnsupportedEnvelope(EnvelopeCase)` | `Refused` | the `EnvelopeCase` (see the list below) |
| `NumericallyUnresolved { spent: Budget, witness: UnresolvedWitness }` | `Unresolved` | `κ` (the spent budget ledger) + the `UnresolvedWitness` (see below) |
| `CompositionMarginExhausted(MarginWitness)` | `Refused` | `stage: str` (the stage that exhausted the margin) |
| `InputOutsideBackwardBudget(RepairWitness)` | `Refused` | `stage: str` (the stage that gave up) |
| `Contradictory(ContradictionWitness)` | `Refused` | `prop`, `left`, `right` (the property whose truth values conflicted) |
| `Collapsed(Collapse, Certificate)` | `Refused` | `reason` (`KnifeEdge` \| `ApexVanishing`) + the certificate |
| `ForwardToleranceExceeded { bound: f64, allowed: f64 }` | `Refused` | `bound`, `allowed` |

The variant list is pinned by `pb_refusal_mapping_covers_landed_refusal_variants`
(a match-exhaustive helper in `showcases/tests/pb_contract.rs`; adding a
variant later breaks that test on purpose).

**`EnvelopeCase` payloads** (carried by `Refused` when the refusal is
`UnsupportedEnvelope`): `ChartDegenerate` · `ReachTooSmall` ·
`NonCanonicalCarrier` · `NonPositiveNurbsWeight` ·
`ContactReductionDeferred` · `ConstructRefused`.

**`UnresolvedWitness` payloads** (carried by `Unresolved`): `UncertifiedContainment`
· `RootNotIsolated` · `KrawczykIndeterminate` · `ContactCurveNotFound` ·
`DeviationUncertified`.

---

## 3. Table schema v1

The three showcase tables (`showcases/tables/{waterslide,teapot,amphora}.json`)
are the portable model tables the bridge consumes (spec §1 portability
contract). Schema v1 is written here normatively.

### 3.1 Version field decision: **v1-by-omission**

The three tables carry **no** `schema_version` field. Pre-decided and pinned:
**v1 tables omit the version field.** A future non-backward-compatible change
must add an explicit top-level `"schema_version"` integer ≥ 2; the absence of
the field means v1, so `{...}` (v1) and `{"schema_version": 2, ...}` are always
distinguishable. `pb_table_schema_v1_parses_all_three_tables` pins both the
omission and the required key sets below.

### 3.2 Envelope

- One UTF-8 JSON document; top-level value is a single JSON object.
- The top-level key set is **exact**: the required keys of the model's row
  below, no missing keys and no unknown keys. (The landed serde builders ignore
  unknown fields; the schema does not — unknown keys are a schema violation so
  table drift is caught at the boundary, mirroring the facade's naming
  discipline.)
- Value domain vocabulary (used by the per-model key tables below):

| Kind | Domain |
|---|---|
| `LEN` | finite JSON number, `0 ≤ x` (a length/radius/thickness; zero allowed) |
| `LEN+` | finite JSON number, `0 < x` |
| `ANGLE_DEG` | finite JSON number, degrees |
| `COUNT` | JSON integer, `1 ≤ x` (a sampling/station count) |
| `RING` | JSON integer, `3 ≤ x` (a closed-polygon vertex count) |
| `FRAC` | finite JSON number, `0 ≤ x ≤ 1` |
| `SCALE` | finite JSON number, `1 ≤ x` (a widening factor) |
| `PAIR` | JSON array `[z, r]` of two finite numbers |
| `TRIPLE` | JSON array of three finite numbers (a coordinate or point) |

### 3.3 Per-model key tables

**`waterslide.json`** — 21 keys.

| key | kind | | key | kind |
|---|---|---|---|---|
| `drop_length` | `LEN+` | | `chute_wall_height` | `LEN+` |
| `drop_angle_deg` | `ANGLE_DEG` | | `chute_top_fraction` | `FRAC` |
| `transition_radius` | `LEN+` | | `chute_wall_thickness` | `LEN+` |
| `helix_radius` | `LEN+` | | `chute_floor_thickness` | `LEN+` |
| `helix_turns` | `LEN` (fractional turns allowed) | | `runout_widening` | `SCALE` |
| `helix_slope_deg` | `ANGLE_DEG` | | `stations` | `COUNT` |
| `runout_length` | `LEN+` | | `pool_radius` | `LEN+` |
| `spine_samples` | `COUNT` (the NUM-INTERPOLE-OVERSHOOT-001 regime input; the table keeps it at the n ≲ 48 bound, see §1.2) | | `pool_depth` | `LEN+` |
| `chute_width` | `LEN+` | | `pool_rim_height` | `LEN` |
| | | | `pool_center_fraction` | `FRAC` |
| | | | `tower_radius` | `LEN+` |
| | | | `tower_clearance` | `LEN` |

**`teapot.json`** — 13 keys. `body_stations`: array of `PAIR` (`[z, r]`),
ascending `z`, `r` finite > 0. `spout_points` / `handle_points`: arrays of
`TRIPLE`. `spout_plane_normal` / `handle_plane_normal`: `TRIPLE` (a plane
normal; the schema does not enforce unit length). The remaining keys:

| key | kind | | key | kind |
|---|---|---|---|---|
| `wall_thickness` | `LEN+` | | `spout_r1` | `LEN+` |
| `foot_height` | `LEN` | | `spout_ring` | `RING` |
| `spout_r0` | `LEN+` | | `handle_radius` | `LEN+` |
| | | | `handle_ring` | `RING` |
| | | | `stations` | `COUNT` |

**`amphora.json`** — 9 keys. `body_stations`: array of `PAIR` (`[z, r]`),
ascending `z`, `r` finite > 0. `handle_points`: array of `TRIPLE`. `foot`:
`TRIPLE` (`[radius, z0, z1]`, radius finite > 0, `z0`/`z1` finite). The
remaining keys:

| key | kind | | key | kind |
|---|---|---|---|---|
| `y_squash` | `LEN+` | | `handle_radius` | `LEN+` |
| `rib_ring` | `RING` | | `handle_ring` | `RING` |
| `handle_azimuth_deg` | `ANGLE_DEG` | | `stations` | `COUNT` |

---

## 4. Byte-determinism contract

**Statement:** the same table + the same kernel revision → byte-identical
report JSON, whether the builder ran from Rust or Python (spec §1, §5).

Mechanism this contract relies on, frozen here:

- The report is `showcases::harness::ShowcaseReport`, serialized by
  `write_report` through `serde_json::to_string_pretty` (a stable field order
  and a stable pretty format). The whole-run report carries the facets, breps,
  exports (including export file paths), boolean outcomes, and CC-port probes.
- Determinism is defined over the **same invocation inputs**, which include the
  output directory: export paths are part of the report, so a byte comparison
  between two runs is only meaningful when both runs target the same output
  directory (or the export rows are stripped). `pb_report_determinism_same_table_same_rev`
  builds the waterslide table twice **in-process** into the same directory and
  compares report bytes.
- The kernel revision part is the pinned vendor tree (`vendor/truck`, rev
  `c5f4b6e9` per the workspace manifest comment) plus the showcase crate; the
  contract is that no builder-side code path (Rust battery or Python surface)
  introduces ordering, hashing, or floating nondeterminism on top of it.
- Nothing in the builder may shell out; the Python side (PB-007) consumes the
  same table files and the same kernel through the bridge and must reproduce
  these bytes.
