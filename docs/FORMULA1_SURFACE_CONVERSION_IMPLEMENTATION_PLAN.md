# FORMULA 1 SURFACE CONVERSION — IMPLEMENTATION PLAN

## A. Provenance

| Item | Value |
| --- | --- |
| Look SHA | `78bd208de5c100242286f71b4b22dd5867e92582` |
| Truck SHA | `40212ece7cb0a7a3eaf9576f29fac8097adaf99c` (local fork `C:\Users\stefa\truck-fork` at same) |
| Truck revision resolved by Look | `40212ece` (pinned in `look/Cargo.toml`, verified in `look/Cargo.lock` source `#40212ece7cb0a7a3eaf9576f29fac8097adaf99c`) |
| ruststep revision | `67f1f7c3875abe05487d1650cc435b4609a3a341` |
| Formula 1 witness | `C:\Users\stefa\look-corpus\formula1\formula1.step` (46,179,762 bytes) |
| Control run | `C:\Users\stefa\look\scratch\unseen_audit\formula1_control.stderr` (142 JSON records) |
| STEP origin | ST-Developer v20 export from Onshape, FILE_SCHEMA `AP242_MANAGED_MODEL_BASED_3D_ENGINEERING_MIM_LF` |

---

## B. Exact 142-face partition

**ONE mechanism explains all 142 losses. There is no second mechanism.**

| Mechanism | Face count | Affected shells | Representative face | Failed entity type |
| --- | --- | --- | --- | --- |
| F1-A | 142 | 4×3: `#81966 #81968 #81976 #81977` (28→25 each) · 4×32: `#81967 #81970 #81971 #81975` (691→659 each) · 1×2: `#81973` (2215→2213) | `#76853` (shell `#81967`) | `DEGENERATE_TOROIDAL_SURFACE` |
| **Total** | **142** | | | |

Arithmetic: `4 × 3 + 4 × 32 + 2 = 12 + 128 + 2 = 142`. ✓

**Census proof (independent of the diagnostic sink):**
- The STEP file contains exactly **142** `DEGENERATE_TOROIDAL_SURFACE` entities.
- Exactly **142** `ADVANCED_FACE` records reference one (verified by regex over the raw file).
- Exactly **142** ImportDiagnosticRecords were emitted, all `SurfaceConversionFailed`.
- The two face sets are the **same** set (0 faces in either set absent from the other).
- `5092` ADVANCED_FACEs reference non-degenerate surfaces and convert; `5,235 − 142 = 5,093` survived. ✓

---

## C. Source chain for the mechanism (F1-A)

Every one of the 142 faces has the identical chain shape. Two representatives:

```text
#76853=ADVANCED_FACE('',(#70698),#14023,.T.);                     // shell #81967
  → #14023=DEGENERATE_TOROIDAL_SURFACE('',#82382,0.063,0.1,.T.);  // FAILED LOOKUP
    → #82382=AXIS2_PLACEMENT_3D('',#106808,#89229,#89230);
      → #106808=CARTESIAN_POINT('',(-0.99008,-1.60563,0.331667)); // geometry is fully present

#76735=ADVANCED_FACE('',(#70511,#70512),#14020,.T.);              // shell #81966
  → #14020=DEGENERATE_TOROIDAL_SURFACE('',#82099,0.1303,0.2,.T.);
    → #82099=AXIS2_PLACEMENT_3D('',#104457,#88631,#88632);
```

The chain is **complete and valid**: `ADVANCED_FACE → DEGENERATE_TOROIDAL_SURFACE → AXIS2_PLACEMENT_3D → CARTESIAN_POINT/DIRECTION`. No hop is missing, no geometry is elsewhere, no explicit STEP relationship must be followed.

---

## D. Exact production failure call graph

```text
look/src/step.rs::parse_step_scene
  → look/src/step.rs::parse_step_table_input (step.rs:251)
    → Table::from_owned_data_section (truck-fork/truck-stepio/src/in/mod.rs:1325)
      → FromIterator::from_iter → Table::push_instance (mod.rs:349)
        → match record.name.as_str() { ... }
          → "DEGENERATE_TOROIDAL_SURFACE" arm: MISSING
          → _ => self.dummy.insert(*id, DummyHolder{..}) (mod.rs:687-695)   ← #14023 lands here
  → look/src/step.rs::parse_step_table (step.rs:281)
    → Table::to_compressed_shells → convert.rs::shell_faces (convert.rs:421)
      → Table::face_surface (convert.rs:296)
        → face.face_geometry.clone().into_owned(self) (convert.rs:305)
          → PlaceHolder<SurfaceAny>::into_owned → EntityTable::<SurfaceAnyHolder>::get_owned
            → (select dispatch, ruststep-derive/src/select.rs:249) tries in order:
              elementary_surface map  → miss
              b_spline_surface map    → miss
              swept_surface map       → miss
              offset_surface map      → miss
            → Err(Error::UnknownEntity(14023))            (ruststep error.rs:17 "Lookup failed for #{0}")
          → .map_err(|e| eprintln!("{e}")).ok()? = None   (convert.rs:306)
        → returns None                                     (convert.rs:307)
      → losses.push(FaceLoss{ reason: FaceLossReason::SurfaceConversionFailed })  (convert.rs:459-463)
  → look/src/step.rs::emit_import_diagnostic (step.rs:142)
    → conversion_stage "surface_conversion", refusal_tag "SurfaceConversionFailed"  (step.rs:119)
    → one JSONL record to stderr
```

**Key defect location:** `truck-fork/truck-stepio/src/in/mod.rs` — `Table::push_instance` record-name dispatch (line 349 onward). It has a `"TOROIDAL_SURFACE"` arm (line 445) but **no** `"DEGENERATE_TOROIDAL_SURFACE"` arm, so the entity is silently filed under `dummy` and is invisible to every typed surface lookup.

---

## E. Root-cause theorem (F1-A)

> The source explicitly represents these 142 faces through `DEGENERATE_TOROIDAL_SURFACE`, a STEP AP242 subtype of `TOROIDAL_SURFACE` carrying one extra boolean (`select_outer`) alongside the identical `(position, major_radius, minor_radius)` geometry. Truck's STEP schema layer (`ruststep` ap203.rs:4639) already models it, but truck-stepio's hand-maintained `Table::push_instance` record dispatch has no arm for `DEGENERATE_TOROIDAL_SURFACE`, so the entity is filed under `dummy` and every `SurfaceAny` lookup of it fails with `UnknownEntity` → `SurfaceConversionFailed`.

The working hypothesis is **proven**: 142 losses = 1 repeated entity-reference mechanism (`DEGENERATE_TOROIDAL_SURFACE` absent from truck-stepio's record dispatch), not 142 independently unsupported surfaces.

### Answers to §5 Q1–Q5

- **Q1 (wrong category):** No. `face_geometry` is `PlaceHolder<SurfaceAny>` — the correct category; the type is a surface subtype.
- **Q2 (parsed but missing from dispatch):** **YES.** `DegenerateToroidalSurface` exists in ruststep's full AP242 schema and routes through `ToroidalSurfaceAny → ElementarySurfaceAny → SurfaceAny`. truck-stepio keeps its own reduced hand-written `Table` + `push_instance` and simply omitted the record name.
- **Q3 (geometry elsewhere):** No. `position + major/minor radius` is inline; `AXIS2_PLACEMENT_3D` resolves normally.
- **Q4 (wrong ID):** No. `#14023` is the correct reference and the record exists; the lookup fails only because the record was mis-filed.
- **Q5 (parser/schema unsupported):** No at the schema level (ruststep models it); yes only in truck-stepio's hand-written dispatch.

---

## F. Ownership decision

**Truck (truck-stepio).** Specifically the record-dispatch layer in `truck-stepio/src/in/mod.rs`, not Look and not ruststep.

> If Truck were used by another STEP consumer, would that consumer encounter the same inability to convert this valid source representation?
> **Yes.** The defect is inside truck-stepio's own `Table::push_instance`; any consumer calling `Table::from_owned_data_section` (including the ruststep-agnostic path Look uses) gets the same `UnknownEntity` → `SurfaceConversionFailed`.

- **Not ruststep:** the schema and `EntityTable` routing already exist there and are correct.
- **Not Look:** Look only reports what truck-stepio returns (`emit_import_diagnostic`). A Look-side workaround would mask a Truck gap.
- **Fix layer:** Truck STEP entity table/resolution + geometry conversion (`truck-stepio`).

---

## G. Exact edit map

### Edit 1 — `truck-fork/truck-stepio/src/in/mod.rs`

**Symbol A: `Table` struct (surface block, ~line 72).**

```rust
// current:
pub toroidal_surface: HashMap<u64, ToroidalSurfaceHolder>,
// required: add immediately after
pub degenerate_toroidal_surface: HashMap<u64, DegenerateToroidalSurfaceHolder>,
```

**Symbol B: `Table::push_instance` record dispatch (~line 445, next to `"TOROIDAL_SURFACE"`).**

```rust
// current: no arm for "DEGENERATE_TOROIDAL_SURFACE" → falls to `_ => dummy`
// required: add arm
"DEGENERATE_TOROIDAL_SURFACE" => {
    self.degenerate_toroidal_surface
        .insert(*id, Deserialize::deserialize(record)?);
}
```

**Symbol C: new holder struct + conversion, placed next to `ToroidalSurface` (~line 2517).**

```rust
/// `degenerate_toroidal_surface`
///
/// AP242 subtype of `toroidal_surface` adding `select_outer`; the analytic
/// surface is the same torus, so it converts to `step_geometry::ToroidalSurface`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = degenerate_toroidal_surface)]
#[holder(generate_deserialize)]
pub struct DegenerateToroidalSurface {
    label: String,
    #[holder(use_place_holder)]
    position: Axis2Placement3d,
    major_radius: f64,
    minor_radius: f64,
    select_outer: Logical,   // ISO 10303-42 sheet-selection hint; preserved here
}

impl From<&DegenerateToroidalSurface> for step_geometry::ToroidalSurface {
    #[inline(always)]
    fn from(x: &DegenerateToroidalSurface) -> Self {
        let mat = Matrix4::from(&x.position);
        let torus = Torus::new(Point3::origin(), x.major_radius, x.minor_radius);
        Processor::new(torus).transformed(mat)
    }
}
```

**Symbol D: `ElementarySurfaceAny` enum (~line 2393).**

```rust
// current variants: Plane, SphericalSurface, CylindricalSurface, ToroidalSurface, ConicalSurface
// required: add
#[holder(use_place_holder)]
DegenerateToroidalSurface(Box<DegenerateToroidalSurface>),
```

**Symbol E: `From<&ElementarySurfaceAny> for ElementarySurface` (~line 2406).**

```rust
// current match arm: ToroidalSurface(x) => Self::ToroidalSurface(x.as_ref().into()),
// required: add
DegenerateToroidalSurface(x) => Self::ToroidalSurface(x.as_ref().into()),
```

**Semantic invariant:** `select_outer` is retained on the parsed holder so no source fact is dropped; the geometry maps to the existing `Torus` because the analytic surface is the same torus and the face's bounds + `same_sense` already define the rendered region. This is NOT "treating an unsupported entity as another surface type" — `DEGENERATE_TOROIDAL_SURFACE` *is* a `TOROIDAL_SURFACE` subtype.

**Failure behavior to preserve:** a genuinely missing reference (`get_owned` returning `UnknownEntity`) must still emit `SurfaceConversionFailed`. The new arm only files correctly-typed records; deserialization failure still propagates the ruststep error.

### No other production edits required

- `convert.rs::face_surface` needs no change — once the record is findable, the existing `Surface::try_from(&SurfaceAny)` path converts it.
- No Look changes.
- No tessellation, torus-recovery, tolerance, or assembly changes.

---

## H. Exact tests (design)

### 1. Unit conversion witness — `truck-fork/truck-stepio/tests/input/geometry.rs`

Mirror the existing `exec_toroidal_surface` (line 1082). Build:

```text
DATA;
#1 = DEGENERATE_TOROIDAL_SURFACE('', #2, 2.0, 0.5, .T.);
#2 = AXIS2_PLACEMENT_3D('', #3, #4, #5);
#3 = CARTESIAN_POINT('', (0., 0., 0.));
#4 = DIRECTION('', (0., 0., 1.));
#5 = DIRECTION('', (1., 0., 0.));
ENDSEC;
```

Assert:
- `step_to_entity::<ElementarySurfaceAnyHolder>` succeeds (parses, does not fall to dummy).
- `(&step).try_into()` (or `.into()`) yields `step_geometry::ElementarySurface::ToroidalSurface`.
- Sample 11×11 points and compare against the analytic torus with major 2.0 / minor 0.5 at identity placement (same assertions as `exec_toroidal_surface`).
- Also a `.F.`/`.U.` select_outer variant parses (Logical coverage).

### 2. Negative witness — same file

```text
DATA;
#1 = TOROIDAL_SURFACE('', #2, 2.0, 0.5);
...
#10 = ADVANCED_FACE('', (#11), #9999, .T.);   // #9999 does not exist
```

Assert `Table` keeps the face; conversion of the face reports `FaceLossReason::SurfaceConversionFailed` (via `to_compressed_shells` / `shell_faces`) — i.e., genuinely missing references still refuse.

### 3. Record-dispatch witness — `truck-fork/truck-stepio/tests/input/table.rs` (or mod.rs in-module)

Parse a minimal DATA section containing one `DEGENERATE_TOROIDAL_SURFACE` and assert `table.degenerate_toroidal_surface.contains_key(&1)` and that `EntityTable::<SurfaceAnyHolder>::get_owned(&table, 1)` succeeds and yields `SurfaceAny::ElementarySurface(ElementarySurfaceAny::DegenerateToroidalSurface(_))`.

### 4. Representative Formula 1 faces — Look-side (after the truck fix is merged/pinned)

Faces `#76853` (shell `#81967`) and `#76735` (shell `#81966`):
- current: `SurfaceConversionFailed`, absent from mesh
- after fix: face reaches tessellation; `look inspect`/render shows the face present in the triangle soup.

### 5. Shell-level accounting

| Shell | declared → survived (current) | declared → survived (expected after fix) |
| --- | --- | --- |
| `#81966` | 28 → 25 | 28 → 28 |
| `#81968` | 28 → 25 | 28 → 28 |
| `#81976` | 28 → 25 | 28 → 28 |
| `#81977` | 28 → 25 | 28 → 28 |
| `#81967` | 691 → 659 | 691 → 691 |
| `#81970` | 691 → 659 | 691 → 691 |
| `#81971` | 691 → 659 | 691 → 691 |
| `#81975` | 691 → 659 | 691 → 691 |
| `#81973` | 2215 → 2213 | 2215 → 2215 |
| file | 5235 → 5093 | 5235 → 5235 |

### 6. Full Formula 1 gate (see §I accounting)

### 7. Regression gate

- `core_xy` current witness: 13 conversion losses (`EdgeCurveConversionFailed`×12, `AllBoundsCollapsed`×1) must be unchanged — this edit does not touch edge/bound dispatch.
- truck-stepio unit suite: `cargo test -p truck-stepio` (input tests incl. new ones).
- NIST corpus: include if CI runs it; the edit only adds a new record-name arm, so a broad sweep is not strictly required unless the release process demands it.

---

## I. Expected recovery

- **F1-A alone recovers 142 / 142** — the partition proves one mechanism covers all 142, so there is no F1-B/C.
- `expected conversion recovery = 142 / 142`
- `prospective face coverage = 5,235 / 5,235`

This is the **expected result of the planned implementation**, not a result already achieved.

Validation accounting for the full render gate (before → after):
- `failed-to-convert`: 142 → 0
- `no-surface`: 0 → 0
- `meshed-to-nothing`: 0 → 0
- triangle count: 365,624 → expected to increase by the tessellation of the 142 recovered torus faces (record actual)
- wall time: record actual (this adds 142 faces to one existing shell family; no new pipeline stages)

---

## J. Copy-pastable implementation packet

```text
EDIT 1
    Repo:       truck-fork
    File:       truck-stepio/src/in/mod.rs
    Symbols:    Table.degenerate_toroidal_surface
                Table::push_instance  ("DEGENERATE_TOROIDAL_SURFACE" arm)
                struct DegenerateToroidalSurface + From<&..> for step_geometry::ToroidalSurface
                ElementarySurfaceAny::DegenerateToroidalSurface variant
                From<&ElementarySurfaceAny>::DegenerateToroidalSurface arm
    Ref:        section G, Edit 1 (Symbols A-E)

TEST 1
    Repo:       truck-fork
    Files:      truck-stepio/tests/input/geometry.rs   (unit witness + negative witness)
                truck-stepio/tests/input/table.rs or src/in/mod.rs in-module (record dispatch)
    Ref:        section H, items 1-3

COMMIT 1
    truck-fork: commit the truck-stepio edit + tests. Tag/mark the new SHA.

UPDATE 2
    Repo:       look
    File:       Cargo.toml  (truck-* deps: rev = <new truck SHA>)
    Then:       cargo update -p truck-stepio (and siblings)

TEST 2 (Look face witnesses)
    look render-formula1 --faces 76853,76735  → both reach tessellation (section H item 4)
    shell accounting check (section H item 5)

FULL FORMULA1 GATE
    look formula1.step --json 2> formula1_after.stderr
    Expect: 0 SurfaceConversionFailed records; 5235/5235 faces;
            142/142 recovered; record triangle count + wall time.

REGRESSION GATE
    cargo test -p truck-stepio
    core_xy current witness: 13 conversion losses unchanged
    cargo test --locked --all-targets (truck-fork and look)
    NIST corpus if the release process runs it

STOP
```

No open-ended investigation steps remain. If the DEGENERATE record's `select_outer` ever needs to influence tessellation (e.g., pcurve sheet selection on a self-intersecting torus), that is a separate, later concern and is explicitly out of scope here.
