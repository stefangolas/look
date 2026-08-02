# Codebase correctness map

**Audited artifacts.** `look` @ `e7fb495` (branch `audit/pre-a1-snapshot`);
truck-fork @ `39346191` (clean); `truck-*` pinned crates @ git rev `79eaaf36`;
`spade` 2.15.1 from crates.io.

**Provenance is a precondition of this map, not a footnote.** `truck-meshalgo`
is *not* consumed from the pinned rev. `Cargo.toml:36-37` and `:45` patch it to a
local path:

```toml
[patch."https://github.com/stefangolas/truck"]
truck-meshalgo = { path = "../truck-fork/truck-meshalgo" }
```

Every other truck crate resolves to `79eaaf36` and is effectively immutable;
meshalgo is locally editable. That asymmetry is load-bearing — it explains why
period witnesses and schema failures were built in meshalgo rather than in
`truck-geotrait`, and it means line numbers below come from two different trees.
Citations are prefixed `[fork]` or `[pin]` where ambiguous.

**Scope.** Analysis only. No production code, tests, or probes were modified.

---

## 0. Three corrections to the stated starting facts

These are recorded first because they invalidate table rows in
`REFINEMENT_AUDIT.md` §6.2 and change what a future reader should look for.

### 0.1 The concrete surface type is *not* `truck_modeling::Surface`

Starting facts 1 and 3, and `REFINEMENT_AUDIT.md` §6.2 ("`S` is instantiated
exactly once, at `truck_modeling::Surface`, a four-variant enum"), do not hold
for look's executed path.

`look` does not depend on `truck-modeling` at all. `Cargo.toml:30-34` lists
`ruststep`, `truck-meshalgo`, `truck-polymesh`, `truck-stepio`, `truck-topology`
— no modeling. The `[patch.crates-io]` entry at `:46` only rewrites a transitive
resolution; it does not create a dependency edge.

The type actually instantiating `S` is fixed by the conversion entry point:

```rust
// [pin] truck-stepio/src/in/convert.rs:564-568
pub fn to_compressed_shell(
    &self,
    shell: &impl StepShell,
) -> Result<CompressedShell<Point3, Curve3D, Surface>, StepConvertingError>
```

where `Surface` is `truck_stepio::r#in::step_geometry::Surface`
([pin] `truck-stepio/src/in/step_geometry/mod.rs:399-405`), a **five**-variant
enum over a different variant tree:

| `truck_stepio::…::Surface` | Underlying type | [pin] location |
|---|---|---|
| `ElementarySurface(ElementarySurface)` | see below | `step_geometry/mod.rs:400` |
| `SweptCurve(SweptCurve)` | `step_geometry/mod.rs:198` | `:401` |
| `BSplineSurface(BSplineSurface<Point3>)` | — | `:402` |
| `NurbsSurface(NurbsSurface<Vector4>)` | — | `:403` |
| `OffsetSurface(StepOffsetSurface)` | `step_geometry/mod.rs:214` | `:404` |

| `ElementarySurface` variant | Resolved type | [pin] alias |
|---|---|---|
| `Plane(Plane)` | `truck_geometry::Plane` | `mod.rs:172` |
| `Sphere(SphericalSurface)` | `Processor<Sphere, Matrix4>` | `mod.rs:23` |
| `CylindricalSurface(..)` | `Processor<RevolutedCurve<Line<Point3>>, Matrix4>` | `mod.rs:25` |
| `ToroidalSurface(..)` | `Processor<Torus, Matrix4>` | `mod.rs:27` |
| `ConicalSurface(..)` | `Processor<RevolutedCurve<Line<Point3>>, Matrix4>` | `mod.rs:29` |

Consequences a future reader must carry:

- A **sphere is not a `RevolutedCurve`.** It is `Processor<Sphere, Matrix4>`, so
  the exact-`2π`-by-rotation argument does not apply to it. `look`'s composition
  layer already gets this right ([look] `src/step/lattice.rs:82`).
- A **torus is `Processor<Torus, Matrix4>`**, doubly periodic, and likewise not a
  `RevolutedCurve`.
- Only **cylinder and cone** are `RevolutedCurve`, and both over a `Line`
  generatrix — so the generatrix axis carries no period at all, which is
  stronger than "an accessor result we cannot certify".
- `truck_modeling::Surface` ([pin] `truck-modeling/src/geometry.rs:132-142`) is a
  real four-variant enum matching the stated facts, but it is on truck's own
  `builder`/example path, not look's STEP path.

### 0.2 The certified-lattice boundary *did* land in production

`REFINEMENT_AUDIT.md` §6 was written at truck-fork `628f39e7` and states that
production reads raw accessors and that the architectural transition "has **not**
occurred". Two commits later that is no longer accurate:

```
5c659209  Stage 1: CertifiedParametricAmbient with representation-derived witnesses
39346191  Stage 1 vertical: route production periodicity through a certified lattice
```

`39346191` touched `tessellation/mod.rs` and `triangulation.rs`. The audit's
recommended **option C** ("descriptor built before entering meshalgo",
§6.4) is implemented and executed:

- [fork] `tessellation/mod.rs:357-373` — `LatticeMeshableShape::robust_triangulation_with_lattice`
  takes `lattice_of: impl Fn(&S) -> CertifiedLattice`.
- [look] `src/step.rs:183` — production calls it with a real resolver:
  `shell.robust_triangulation_with_lattice(tolerance, lattice::lattice_of)`.
- [look] `src/step/lattice.rs:34-51` — the resolver, matching the concrete enum.

**But the behaviour is unchanged**, deliberately. Every consumer inside
triangulation.rs reads `declared_period()`, not `generator()`:

| Site | [fork] `triangulation.rs` | Reads |
|---|---|---|
| lift branch selection | `:483` | `declared_u_period` / `declared_v_period` |
| degenerate re-synthesis | `:631`, `:644` | `declared_*_period` |
| closure classification | `:1067-1068` | `declared_*_period` |
| two-loop merge | `:1135`, `:1144`, `:1154` | `declared_*_period` |
| collapsed-pair seam | `:1174-1179` | `declared_*_period` |
| seam vertex role | `:2030-2031` | `declared_*_period` |

`declared_period()` returns `Some` for both `Exact` and `Uncertified`
([fork] `domain/lattice.rs:66-72`), so the type boundary currently **records**
evidence quality without **acting** on it. This is stated honestly in the source
([fork] `domain/lattice.rs:20-27`) and is the correct sequencing — but it means
no gap in §4 below is discharged by the lattice's existence alone.

### 0.3 `working_range` exists but is not executed

Starting fact 8 is confirmed, with a caveat worth recording: the derived
face-extent function is written and env-gated off.

```rust
// [fork] triangulation.rs:1062-1065
let range = match std::env::var_os("TRUCK_FACE_DOMAIN").is_some() {
    true  => working_range(&pieces, surface),   // derived from the face's bounds
    false => surface.try_range_tuple(),          // the primitive's declared range
};
```

Default runs take the primitive's range. `working_range`
([fork] `:1025-1048`) is *itself* only partially derived: for any axis with a
period it returns the declared range unchanged (`:1031-1034`).

---

## 1. Crate and visibility map

### 1.1 Dependency directions (normal deps only)

```
                       spade 2.15.1 ──────┐
                                          ▼
truck-geotrait ◄── truck-geometry ◄── truck-meshalgo [LOCAL PATH]
      ▲                  ▲                    ▲
      │                  │                    │
      └──── truck-modeling ◄── truck-stepio   │
                 (not a look dep)      ▲      │
                                       │      │
                                    look ─────┘
                                       │
                                  truck-topology
```

- `truck-meshalgo` ↔ `truck-modeling` are **mutually dev-only**. This is the
  dependency restriction that prevents meshalgo from naming any concrete surface.
- `truck-stepio` depends on `truck-modeling` normally; its meshalgo dep is dev-only.
- `look` is the **only crate that sees both** the concrete `Surface` enum and
  meshalgo's types. It is therefore the composition layer, and it is now used as
  one ([look] `src/step/lattice.rs`).

### 1.2 Per-crate ownership

| Crate | Owns (correctness-relevant) | Downstream can name | Present but opaque | Concrete dispatch |
|---|---|---|---|---|
| `truck-geotrait` | trait *contracts*: `ParametricSurface`, `ParametricSurface3D`, `SearchParameterD2`, `ParameterDivision2D`; the default `parameter_range`/`u_period`/`v_period` | all | — | none (defaults only) |
| `truck-geometry` | the actual parameterisations: `RevolutedCurve`, `Processor`, `Plane`, `BSplineSurface`, `NurbsSurface`, `Sphere`, `Torus` | the types, if depended on | — | inside each `impl` |
| `truck-modeling` | its own 4-variant `Surface` | — (not on look's path) | — | derive macro |
| `truck-stepio` | **the production `Surface` enum**; STEP→geometry conversion; face/bound/edge-use orientation composition; `FaceProvenance` | `look` | — | `#[derive(ParametricSurface3D)]`, `step_geometry/mod.rs:389-405` |
| `truck-meshalgo` | the entire tessellation semantics: lift, closure, arrangement, parity, realization; `CertifiedLattice`; `ConstraintRole`; `TessellationFailureReason` | `look` | **`S` in full** | none — cannot match on `S` |
| `look` | composition: `lattice_of`; tolerance policy; loss accounting | — | — | `src/step/lattice.rs:34-51` |
| `spade` | CDT realization | all | its DCEL internals | — |

### 1.3 The four distinctions, applied

The task asks these be kept apart. In this codebase they land as follows.

- **Semantic erasure** — information destroyed, unrecoverable downstream.
  *Real instances:* deck displacement `[ku,kv]` computed then dropped
  ([fork] `:1083-1130`); the typed `TessellationFailureReason` collapsed to an
  empty mesh ([fork] `:1900-1905`); STEP outer/inner bound syntax, absent from
  `CompressedFace.boundaries: Vec<Vec<CompressedEdgeIndex>>`
  ([pin] `truck-topology/src/compress.rs:154-160`).

- **Visibility restriction** — nameable in principle, not from here.
  *Not a real instance in the current design.* `S` is generic, not private; the
  obstacle is a dependency restriction, next item.

- **Dependency restriction** — the actual obstacle. meshalgo cannot depend on
  `truck-modeling`/`truck-stepio` normally (dev-only / would invert the mesher
  onto the kernel), so it cannot write `match surface { … }`.

- **Present but inaccessible to a consumer** — the accurate description of `S`.
  The concrete surface **is intact** for the whole of meshalgo, carried as the
  generic parameter. Nothing is erased; meshalgo simply cannot *name* it.
  Confirmed at every stage: `S: PreMeshableSurface` on `cshell_tessellation`
  ([fork] `:172-181`), `PolyBoundaryPiece::try_new` ([fork] `:471-477`),
  `PolyBoundary::new` ([fork] `:1051-1056`), and
  `triangulation_into_polymesh_outcome` ([fork] `:1966-1973`).

  **This is why the descriptor parameter works.** `lattice_of` is evaluated in
  `look`, where the enum *is* nameable, and the result travels as data. It is the
  general remedy shape for this class.

---

## 2. Executed call graph: STEP face → `PolygonMesh`

Legend for fact tracking: **I**ntroduced, **T**ransformed, **R**etained,
**D**iscarded.

| # | File | Function / type | Input | Output | Facts |
|---|---|---|---|---|---|
| 1 | [look] `src/step.rs:87` | `Table::from_owned_data_section` | Part 21 text | entity table | source identity **I** |
| 2 | [pin] `truck-stepio/…/convert.rs:564` | `to_compressed_shell` | STEP shell | `CompressedShell<Point3, Curve3D, Surface>` | representation **I**; face orientation **I** |
| 3 | [pin] `convert.rs:185-256` | `face_bound_to_edges` | `FaceBoundHolder` | `Vec<CompressedEdgeIndex>` | edge-use orientation **I** (`:244`, `oriented_edge.orientation == ori`); TOP-003 **discharged** by `collect::<Result<_>>` (`:253-255`); **outer/inner bound syntax D** |
| 4 | [pin] `convert.rs:468` | face `same_sense` handling | face | inverted surface | face sense **T** (folded into geometry) |
| 5 | [look] `src/step.rs:183` | `robust_triangulation_with_lattice` | shell, tol, `lattice_of` | meshed shell | — |
| 6 | [look] `src/step/lattice.rs:34-51` | `lattice_of` | `&Surface` | `CertifiedLattice` | **period evidence I** — the only place representation is read |
| 7 | [fork] `mod.rs:379-391` | `LatticeMeshableShape::robust_triangulation_with_lattice` | ↑ | ↑ | — |
| 8 | [fork] `triangulation.rs:172-380` | `cshell_tessellation` | shell | `MeshedCShell` | — |
| 9 | [fork] `:184-256` | `tessellate_edge` | `CompressedEdge<C>` | polyline | curve sampling **I**; zero-length edge extended by `curve.period()` (`:235-241`); `len()<=2` → 16-step resample, **no residual** (`:243-251`) |
| 10 | [fork] `:330` | provenance capture | `face.provenance` | `source_face_id` | source identity **R** |
| 11 | [fork] `:340-343` | `create_edge` | `CompressedEdgeIndex` | oriented polyline | edge-use orientation **T→D** — applied as `curve.inverse()`, never retained |
| 12 | [fork] `:344-347` | `create_boundary` | wire | `PolyBoundaryPiece` | `filter_map` **silent-drop hazard** (see §4) |
| 13 | [fork] `:471-863` | `PolyBoundaryPiece::try_new` | wire + surface + lattice | `Vec<SurfacePoint>` | **the lift** — see §2.1 |
| 14 | [fork] `:1051-1331` | `PolyBoundary::new` | pieces | `Vec<Vec<SurfacePoint>>` | closure, merge, synthetic stitching — see §2.2 |
| 15 | [fork] `:1360-1599` | `PolyBoundary::insert_to` | pieces | `bool` + CDT + roles | welding, constraint insertion — see §2.3 |
| 16 | [fork] `:1910-1963` | `insert_surface` | CDT, surface | CDT + grid constraints | sampling role **I** (`:1924`) |
| 17 | [fork] `:1966-2181` | `triangulation_into_polymesh_outcome` | CDT, roles, lattice | `TessellationOutcome` | material classification — see §2.4 |
| 18 | [fork] `:1885-1907` | `trimming_tessellation` | ↑ | `PolygonMesh` | **typed failure D** (`:1900-1905`) |
| 19 | [look] `src/step.rs:207-…` | loss accounting | meshed faces | tally + ids | source identity **R** (only survivor) |

### 2.1 The lift — `PolyBoundaryPiece::try_new` [fork] `:471-863`

| Line | Act |
|---|---|
| `:483` | reads `declared_*_period` from the lattice |
| `:484` | reads `surface.try_range_tuple()` — the conflated range |
| `:492-505` | a 2-point polyline is replaced by **8 fabricated points on the straight 3D chord**; other edges drop their last point (`take(n)`) for concatenation |
| `:512-514` | empty wire → `None` → whole face lost |
| `:515` | `bdry3d.push(bdry3d[0])` — the walk is **closed by construction**; TOP-004 is never checked |
| `:532` | `sp(surface, pt, previous)` — per-sample projection, hint = previous sample |
| `:538-542` | projection failure → whole face `None` (synthetic midpoints are skipped instead) |
| `:559-577` | curve–surface compatibility gate — **disabled by default**, `COMPATIBILITY_FACTOR = f64::INFINITY` (`:890`) |
| `:579-584` | `get_mindiff` — the branch choice (`:866-872`) |
| `:608-622` | ambiguity refinement, ≤ 8 halvings (`:919-922`) |
| `:623` | **on exhaustion the ambiguous step is accepted with no record** |
| `:628-658` | a degenerate bound is re-synthesised as a full declared period |
| `:659-669` | **per-piece centroid deck normalisation** against the declared range origin; `quot_u`/`quot_v` **D** (used only in the probe print at `:843`) |
| `:855-861` | closure appended only when a derivative is small |

### 2.2 Closure and stitching — `PolyBoundary::new` [fork] `:1051-1331`

| Line | Act |
|---|---|
| `:1062-1065` | domain source (see §0.3) |
| `:1073` | closure by **raw UV distance < 1.0e-3** — no first fundamental form (QUO-002 requires the metric) |
| `:1076-1085` | winding `[ku,kv]` computed into `BoundaryClosure::PeriodicClosed` |
| `:1110-1130` | winding **consumed to shift points, then D** — never stored |
| `:1134-1166` | two zero-area loops merged via centroid means and reversal |
| `:1167-1213` | `CollapsedPeriodicBoundaryPair` seam construction (**live production**) |
| `:1222-1295` | open pieces stitched against `range` corners — synthetic segments appended into the **same `Vec<SurfacePoint>`** as source segments, becoming indistinguishable |
| `:1315-1329` | no-loop face takes the whole declared rectangle |

### 2.3 Realization — `insert_to` [fork] `:1360-1599`

| Line | Act |
|---|---|
| `:1409-1412` | vertex weld: **linear scan over all vertices**, `distance_2 < 1e-12`, absolute and unit-dependent |
| `:1437-1439` | `(k+1) % len` — every piece treated as a closed cycle |
| `:1442-1445` | `vi == vj` zero-length segment skipped silently |
| `:1450` | `can_add_constraint` — refuses on proper crossing |
| `:1452-1465` | `add_constraint` → `get_edge_from_neighbors` → role recorded; **the lookup fails whenever Spade split the request into a chain**, and the role is lost |
| `:1465` | every `PolyBoundary` segment tagged `PhysicalBoundary`, **including synthetic closure** (acknowledged in-source, `:1455-1464`) |
| `:1535` | refusal → `success = false` → `ConstraintInsertionIncomplete` |

### 2.4 Material and mesh — [fork] `:1966-2181`

| Line | Act |
|---|---|
| `:1980-1982` | parity seeded at the **outer face = 0** — the implicit `Empty` base (DOM-003) |
| `:2000-2001` | role-gated toggle (A1) |
| `:2009-2012` | inconsistent cycle → `ContradictoryDualParity` |
| `:2039-2042` | non-boundary vertices positioned by `surface.subs` |
| `:2053-2054` | seam vertex role compares `p.x` against `0` and `u_period` — **assumes the domain origin is 0**; diagnostic-only, but false for any shifted chart |
| `:2098-2100` | material = parity 1 |
| `:2108-2118` | degenerate/zero-area triangles dropped silently |
| `:2123-2125` | empty selection → `NoOddParityRegion` |

---

## 3. Formal object map

| Formal object | Formal definition | Production representation | Constructor | Consumers | Status |
|---|---|---|---|---|---|
| Ambient schema (Ω,Λ,N,Σ,S,C) | FS Def. 7 | none as a whole; `CertifiedLattice` covers Λ only | [fork] `domain/lattice.rs:109-183` | lift, closure | **partially represented** |
| Parameter lattice Λ | FS Def. 7 | `CertifiedLattice` with per-axis witness | [look] `src/step/lattice.rs:34` | read as `declared_period`, never as `generator` | **faithfully represented on a restricted class** (cylinder/cone only) |
| Native boundary N | FS Def. 7 | `ConstraintRole::NativeBoundary` exists; never constructed | [fork] `:1636` | — | **represented but not established** |
| Singular stratum Σ + link λ | FS Def. 7, 24 | ad-hoc `uder().so_small()` probes; `SingularStratum` prototype | [fork] `domain/schema.rs:73-80`; probes at `:441-464`, `:858` | `CollapsedPeriodicBoundaryPair` (live) | **partially represented** |
| Oriented source edge use | FS Def. 12 | `CompressedEdgeIndex{index, orientation}` | [pin] `compress.rs:35-40` | [fork] `:340-343`, then lost | **erased before use** |
| Lifted boundary arc *a*=(γ,p,q,δ,τ,ℓ,π) | FS Def. 9 | `Vec<SurfacePoint>` — γ only | [fork] `:468` | everything downstream | **partially represented** (no p,q,δ,τ,ℓ,π) |
| Lifted closed walk | FS Def. 14 | `Vec<SurfacePoint>` closed by construction | [fork] `:515` | `PolyBoundary::new` | **represented but not established** |
| Deck displacement δ | FS Def. 9, 14 | `BoundaryClosure::PeriodicClosed{displacement}` | [fork] `:926-936`, `:1083` | shifts points, then dropped | **erased before use** |
| Winding | FS §VII, QUO-002 | same as above | [fork] `:1076-1085` | — | **erased before use** |
| Boundary role kind(h) | FS §IX | `ConstraintRole` | [fork] `:1627-1644` | `toggles_material` `:1687` | **partially represented** (side table with holes) |
| Normalized arrangement vertex | FS §IX | Spade vertex welded at 1e-12 | [fork] `:1409-1412` | CDT | **reconstructed heuristically** |
| Normalized arrangement half-edge | FS Def. 18 | raw consecutive point pair; no splitting stage | [fork] `:1437-1441` | CDT | **absent** |
| Cell / region | FS §IX, Def. 22 | CDT inner face + parity bit | [fork] `:1977-2018` | triangle selection | **partially represented** |
| Material assignment μ | FS Def. 19-21 | `u32` parity from BFS flood | [fork] `:2098-2100` | mesh | **faithfully represented on a restricted class** (see §6) |
| CDT realization | MF CDT-002 | `bool` from `insert_to` | [fork] `:1360-1599` | pass/fail only | **represented but not established** |
| Mesh realization | MF MSH-002/003 | `PolygonMesh` | [fork] `:2164-2171` | caller | **partially represented** |
| Certificate | FS Def. 26, Π | `PeriodWitness::ExactRevolutionAngle` only | [fork] `domain/lattice.rs:35-40` | none — recorded, never branched on | **represented but not established** |
| Typed unknown | MF §27 | `TessellationFailureReason` (11 variants) | [fork] `:1703-1716` | **discarded at `:1900-1905`** | **erased before use** |
| Contradiction | FS §VII, Def. 21 | `ContradictoryDualParity`, `ConstraintInsertionIncomplete` | [fork] `:1705`, `:1708` | same — erased | **erased before use** |
| Unsupported case | FS Def. 3 | `ConstraintIntersectionUnsupported`, `ConstraintOverlapUnsupported` | [fork] `:1709-1710` | **never constructed anywhere** | **absent** |

---

## 4. Formal obligation map

The question for every row: *what code establishes this before another function
relies on it?*

| Obligation | Formal source | Producer | Consumer | Evidence actually used | Status |
|---|---|---|---|---|---|
| Period validity `S(u+P,v)=S(u,v)` | QUO-001 | `RevolutedCurve::v_period` [pin] `revolved_curve.rs:143-145` | lift `:483` | **exact by construction** for the revolution angle (`subs` applies `rotation_matrix(v)`, `:84-87`); **accessor result** for the generatrix (`:139-141`) | established for `v` on cylinder/cone; **unestablished elsewhere** |
| Parameter-axis identity | new (fact 7) | `Processor` swap [pin] `processor.rs:259-282` | all axis-indexed reads | consistent across `subs`/`uder`/`vder`/`uuder`/`uvder`/`vvder`/`range`/`period`/`normal`; restated in caller convention by [look] `lattice.rs:95-100` | **established** — with one exception, next row |
| Parameter-search axis identity | GEO-006 | `Processor::search_parameter` [pin] `processor.rs:507-521` | lift `:532` | **result is swapped, the hint is not** — see §5.1 | **violated** |
| Parameter-domain authority | DOM-001 | `try_range_tuple` [pin] `geotrait/…/surface.rs:40-46` | `:484`, `:1064` | four different epistemic classes collapsed — see §5.2 | **absent** |
| Effective edge-use orientation | TOP-005 | [pin] `convert.rs:244` composes bound × oriented-edge | [fork] `:340-343` | applied geometrically as `curve.inverse()`, never composed with face sense nor retained | **represented but not established** |
| Wire continuity | TOP-004 | — | `:515` | **none** — the walk is closed by appending point 0 | **absent** |
| Curve-sampling fidelity | GEO-003 | `PolylineCurve::from_curve` | `:242` | trusted; `len()<=2` triggers a 16-step resample with **no residual check** (`:243-251`) | **represented but not established** |
| Curve–surface compatibility | GEO-005 | residual gate `:559-577` | — | **disabled by default** (`COMPATIBILITY_FACTOR = INFINITY`, `:890`) | **represented but not established** |
| Lift branch continuity | FS Def. 14 | `get_mindiff` `:866-872` | `:579-584` | nearest-copy rule; correct only while the true step < ½ period; ambiguity refined ≤8× then **accepted silently** (`:614-623`) | **violated** |
| Deck-potential consistency | QUO-004 | `DeckPotentialUnionFind` [fork] `domain/deck.rs:26-77` | **never called** | none; each piece is normalised independently at `:659-669` | **represented only in disconnected code** |
| Closed-walk winding | QUO-002 | `periodic_displacement` `:939-950` | `:1076-1085` | computed, used to shift, then dropped | **erased before use** |
| Closure validity | ARR-001, QUO-002 | `:1073` | `:1105` | raw UV distance vs hard-coded `1.0e-3`; **no metric G** | **violated** |
| Source vs synthetic edge distinction | FS §IX | `ConstraintRole::UnresolvedSyntheticClosure` `:1643` | `:1465` | **never constructed** — stitched segments enter as `PhysicalBoundary` | **absent** |
| Vertex-welding topology preservation | ARR-002 | `:1409-1412` | CDT | proximity `< 1e-12` in UV units; no deck/singular distinction (FS §IX forbids merging `~_Λ` with `~_Σ`) | **reconstructed heuristically** |
| Intersection atomization | ARR-002 | — | — | **no splitting stage exists**; raw pairs handed to Spade | **absent** |
| Overlap normalization | ARR-003 | — | — | **none** | **absent** |
| Complete physical-constraint realization | CDT-002 | `insert_to` → `bool` | `:1838` | `bool`; a `false` correctly means "not representable as stated" (§7) | **represented but not established** |
| Semantic-edge → CDT-edge correspondence | CDT-001 | `ConstraintRoles` side table `:1654-1662` | `:2001` | `get_edge_from_neighbors` after the fact; **fails on every chain Spade splits** | **partially represented** |
| Material base-state evidence | DOM-003, DOM-005 | outer face seeded 0 `:1980-1982` | parity flood | **assumed**, not established | **represented but not established** |
| Role-sensitive material transition | FS Def. 20 | `toggles_material` `:1687-1700` | `:2001` | **discharged for `SurfaceSampling`** (A1); unresolved roles default to toggling (`:1694-1698`) | **faithfully represented on a restricted class** |
| Ambiguity / contradiction handling | FS Def. 21, MF §27 | `TessellationFailureReason` `:1703-1716` | `:1900-1905` | detected, typed, then **converted to an empty mesh** | **erased before use** |
| Exact region-to-mesh realization | CDT-005, MSH-002 | `:2098-2121` | mesh | selection by parity; degenerate triangles dropped silently | **partially represented** |

---

## 5. Epistemic-value map

Values whose runtime type does not express their epistemic status.

### 5.1 `Option<(f64,f64)>` — the projection result

| Collapsed meanings | Where interpreted | Consequence |
|---|---|---|
| exact analytic inverse | `:532` | — |
| iterative inverse converged within tolerance | `:532` | — |
| **nearest point on a surface the point does not lie on** | `:532` via `by_search_nearest_parameter` `:56-69` | a boundary from another face yields a plausible UV and triangulates into a large wrong region; the guard exists at `:559-577` but is **off by default** |
| correct point, **wrong branch** | `:579-584` | period fold |
| no solution | `:538-542` | whole face lost |

**Compounding defect.** For an inverted `Processor`, `search_parameter` swaps its
*result* but forwards the *hint* unswapped:

```rust
// [pin] truck-geometry/src/decorators/processor.rs:507-521
let (u, v) = self.entity.search_parameter(inv.transform_point(point), hint, trials)?;
match self.orientation {
    true  => Some((u, v)),
    false => Some((v, u)),      // result transposed …
}                               // … but `hint` went in untransposed
```

The lift passes the previous lifted UV as the hint (`:532`), so on any inverted
`Processor` the continuity hint lands on the wrong axes. Classification:
**caller-asserted, silently invalid**. Marked `Unknown` for blast radius — how
many STEP cylinders/cones arrive inverted is not measured here, and measuring it
is out of scope for this map.

### 5.2 `(Option<Tuple>, Option<Tuple>)` — `try_range_tuple`

Four epistemic classes arrive indistinguishable at `:484` and `:1064`:

| Origin | Value | Class | [pin] cite |
|---|---|---|---|
| `RevolutedCurve` angular axis | `[0, 2π)` | **exact by construction** | `revolved_curve.rs:135` |
| `BSplineSurface`/`NurbsSurface` | knot span | **exact** — genuinely undefined outside | `nurbs/bspsurface.rs:304-313` |
| `Plane` | `[0,1]×[0,1]` unconditionally | **fabricated** — a plane is unbounded | `specifieds/plane.rs:136-139` |
| `RevolutedCurve` generatrix axis | apex-extended heuristic window | **numerically inferred** | `revolved_curve.rs:116-134` |

The generatrix row is stronger than previously recorded. `RevolutedCurve::parameter_range`
does not merely inherit `[0,1]`: when the two end radii differ it computes an
apex parameter and returns

```rust
// [pin] revolved_curve.rs:130-132
let u_min = f64::min(t0, t_apex);
let u_max = f64::max(t1, t0 + 10.0 * (t1 - t0).abs().max(1.0));
```

— a **10× extrapolated window** that is neither the primitive's declared range
nor the face's extent. This is the value that reaches the synthetic-closure
corners at `:1227-1269`.

### 5.3 `Option<f64>` — period

Now *partly* resolved by `AxisPeriodStatus` ([fork] `domain/lattice.rs:44-61`),
which separates `Exact{witness}` / `Uncertified{declared}` / `NonPeriodic`.
Still collapsed **at the point of use**, because every consumer calls
`declared_period()` (§0.2), which re-merges `Exact` and `Uncertified`.

### 5.4 `bool` — `insert_to`

| Collapsed meanings | Where | Consequence |
|---|---|---|
| all constraints realized | `:1598` | proceed |
| a segment properly crossed an existing constraint | `:1450`→`:1535` | `ConstraintInsertionIncomplete` |
| a vertex insertion failed | `:1428-1431` | same code |
| already fully represented (`add_constraint` false) | `:1473-1475` | counted only under probe; **correctly ignored** |

Four causes, one bit, one failure reason.

### 5.5 `FixedUndirectedEdgeHandle` — the role key

Identifies an edge **only if Spade realized the request as that single edge**.
Spade's own documentation says otherwise (`cdt.rs:541-544`): *"the given
constraint might be split into smaller edges… Thus `cdt.exists_constraint(from,
to)` is not necessarily `true` after a call."* Every split chain loses its role
and falls to the `None` arm at `:1694-1698`, which **toggles material anyway**.

### 5.6 Empty `PolygonMesh`

The terminal collapse. `:1900-1905` maps **all eleven**
`TessellationFailureReason` variants — including `ContradictoryDualParity`,
which is a *proved inconsistency* — onto `PolygonMesh::default()`.
[look] `src/step.rs:208-…` then cannot distinguish "surface could not be
produced" from "produced and meshed to nothing", which is why loss accounting
had to split `None` from `Some(empty)` by hand.

### 5.7 `Vec<Vec<SurfacePoint>>` — `PolyBoundary`

One type denoting four distinct FS sorts (ambient term, boundary term,
arrangement complex *G*, region term *R*). Source segments, synthetic closure
segments, seam segments and re-synthesised full-period circles are all
`SurfacePoint` pairs with no discriminant.

---

## 6. Restricted-algorithm map

Not globally incorrect — correct on a characterizable class.

| Algorithm | Preconditions | Where established | Current scope of correctness |
|---|---|---|---|
| Odd/even parity classification [fork] `:1977-2018` | base occupancy known; every material transition is a role-toggling constraint edge; arrangement conforming | base **assumed** `Empty` at `:1980-1982`; roles partial | correct when the outer face is genuinely empty **and** every constraint role resolved; A1 fixed the sampling-grid case |
| `get_mindiff` nearest-copy lift `:866-872` | true parameter step < ½ period between consecutive samples | **nowhere**; `:614-622` detects and refines, then gives up silently | correct on boundaries sampled finely relative to the period |
| Centroid deck normalisation `:659-669` | one bound per face, or all bounds in the same deck copy | **nowhere** — each piece normalised independently | correct for single-bound faces; places multi-bound faces in unrelated copies |
| UV-distance closure `:1073` | chart is locally isometric, i.e. G ≈ I | **nowhere** — no metric evaluated | correct on near-conformal charts at ordinary scale; wrong on anisotropic or unit-scaled charts |
| Fixed-tolerance vertex welding `:1409-1412` | UV coordinates O(1); genuine distinct vertices > 1e-6 apart | **nowhere** | correct for normalised angular charts; unit-dependent, and O(n²) |
| Direct sequential constraint insertion `:1437-1537` | segments pairwise non-crossing and non-overlapping **as presented** | **nowhere** — no atomization stage | correct when the lifted boundary is already simple; this is precisely what fails on 84% of measured loss |
| Spade CDT realization | vertex set contains every intersection; no requested constraint crosses another | not established | see §7 — the library is faithful; the precondition is not met |
| Projection-based curve-on-surface `:532` | the point lies on *this* surface | gate exists `:559-577`, **off by default** | correct when the face/surface pairing is right |
| Synthetic rectangle closure `:1222-1295`, `:1315-1329` | the declared range is the face's material extent | **false by construction** for `Plane` and for revolved generatrix axes (§5.2) | correct only when the primitive range coincides with the face extent |
| Mesh extraction from selected cells `:2098-2121` | parity labelling correct; degenerate triangles are genuinely degenerate | inherits parity's preconditions | correct given the above |

---

## 7. Spade: what the foreign layer actually guarantees

Read from `spade-2.15.1` source, not assumed. Confirms the prior 2.15.0 read.

- **`can_add_constraint`** (`cdt.rs:445-448`) — `true` iff the segment properly
  crosses no existing constraint edge. `contains_any_constraint_edge`
  (`:463-471`) matches only `Intersection::EdgeIntersection(e)` with
  `e.is_constraint_edge()`. So `false` ⟺ **proper crossing**. Faithful.
- **`add_constraint`** (`cdt.rs:558-563`) — calls
  `resolve_splitting_constraint_request` and returns whether `num_constraints`
  changed. `false` ⟺ *already fully represented*; ignoring it is correct.
- **Splitting** — documented at `:541-544`: the request may be realized as a
  chain, after which `get_edge_from_neighbors(from, to)` returns `None`. This is
  the documented root cause of the role-table holes.

### The chain *is* publicly available — no vendoring required

`REFINEMENT_AUDIT.md` §4 qualified this as "unrecoverable under the *current*
design". It is better than that: **two public APIs already return the realized
chain.**

```rust
// [spade] cdt.rs:807-817
/// Returns all constraint edges that connect `from` and `to`. This includes any
/// constraint edge that was already present.
/// Returns an empty list if the new constraint would intersect any existing
/// constraint or if `from == to`.
pub fn try_add_constraint(&mut self, from: FixedVertexHandle, to: FixedVertexHandle)
    -> Vec<FixedDirectedEdgeHandle>

// [spade] cdt.rs:1227-1234
pub fn add_constraint_and_split<C>(&mut self, from, to, vertex_constructor: C)
    -> Vec<FixedDirectedEdgeHandle>
```

`try_add_constraint` is the precise shape the current code needs:

- it returns the **complete realized chain**, discharging CDT-002 directly
  instead of inferring it from a Boolean;
- it is **atomic** — on conflict it leaves the triangulation unchanged and
  returns an empty vector, so refusal and mutation cannot interleave;
- an empty return distinguishes *refused* from a non-empty return of
  pre-existing edges (*already represented*).

The current call sequence at `:1446-1475` — `get_edge_from_neighbors` +
`can_add_constraint` + `add_constraint` + `get_edge_from_neighbors` — performs
three redundant traversals and still loses the chain. **A realization bijection
is achievable against the public API of the pinned version.** Nothing measured
supports vendoring or modifying Spade.

---

## 8. Active, dead, and prototype classification

| Component | [fork] location | Class | Notes |
|---|---|---|---|
| `cshell_tessellation` and the whole `PolyBoundaryPiece`→`PolyBoundary`→`insert_to`→parity chain | `triangulation.rs:172-2181` | **executed production** | the path of §2 |
| `CertifiedLattice`, `AxisPeriodStatus`, `PeriodWitness` | `domain/lattice.rs` | **executed production** | reached via `declared_period` only |
| [look] `lattice_of` | `src/step/lattice.rs` | **executed production** | the composition layer |
| `CollapsedPeriodicBoundaryPair::try_classify` | `:2234` ← called `:1168` | **executed production** | the one live singular-stratum path |
| `ConstraintRole` + `ConstraintRoles` | `:1627-1701` | **executed production** | 3 of 5 variants never constructed |
| `working_range` | `:1025-1048` | **executed restricted fallback** | `TRUCK_FACE_DOMAIN`, off by default |
| compatibility gate | `:559-577`, `:890-908` | **executed restricted fallback** | `TRUCK_COMPAT_FACTOR`, `INFINITY` by default |
| `TessellationOutcome` / `FaceTessellation` / `TessellationDiagnostics` | `:1806-1823` | **diagnostic-only** | constructed, then discarded at `:1900-1905` |
| `VertexMetadata`, `VertexRoles`, `SeamPair`, `SingularGroup` | `:1740-1811` | **diagnostic-only** | `seam_pairs`/`singular_groups` always empty (`:2177-2178`) |
| `trimming_tessellation_with_outcome` | `:1872` | **dead code** | `#[allow(dead_code)]`, no caller |
| `triangulation_into_polymesh` | `:2184` | **dead code** | superseded by `_outcome` |
| `solve_quotient_lift`, `QuotientLiftCertificate`, `QuotientResolvedFace`, `WireAssembledFace` | `:2323-2409` | **prototype not consumed** | sole caller is the test at `:2423` |
| `reconcile_singular_transition` | `:429-465` | **prototype not consumed** | callers are tests only (`:2841`+); production activation reverted 2026-08-02 |
| `domain/schema.rs` — `ParametricQuotient`, `DeckLattice`, `SingularStratum`, `StratumCertificate`, `SchemaFailure`, `ParametricQuotientSurface` | whole file | **prototype not consumed** | only `adapters/revolution.rs`, `ambient.rs`, `quotient.rs` reference it — all themselves dead |
| `domain/deck.rs` — `DeckPotentialUnionFind` | whole file | **prototype not consumed** | the exact QUO-004 solver; **never called** |
| `domain/ambient.rs` | 524 lines | **prototype not consumed** | frozen at `5c659209`; puts the face domain on the input side of the lift, which §6.3 of the audit showed is the wrong side |
| `domain/projection.rs` | 279 lines | **prototype not consumed** | referenced only from tests `:2546-2566` |
| `domain/{quotient,canonical,evidence,plan}.rs`, `adapters/revolution.rs` | — | **prototype not consumed** | closed cluster; zero production references |
| `PROBE_FACE_CONTEXT` and all `TRUCK_PROBE_*` blocks | `:21-26` and passim | **diagnostic-only** | |
| FS §XVIII required modules (`atlas/…`) | — | **formal specification only** | no counterpart exists |

### For each disconnected component

**`ParametricQuotient` / `SchemaFailure`** — intends FS Def. 7 (ambient schema).
Blocked because it bundles the parameter domain with the lattice, and the domain
is an *output* of lifting (audit §6.3). Its inputs are not themselves justified:
`u_period`/`v_period` fields duplicate `lattice`, and `StratumCertificate`
variants are residual-bearing structs, not proofs. Partially duplicated by the
live `CertifiedLattice`, which is the corrected subset.

**`DeckPotentialUnionFind`** — intends QUO-004 / FS §VII exactly, and is a
correct weighted union-find. Blocked because nothing produces the `δ` inputs it
consumes: the only δ computed in production is dropped at `:1110-1130`. Does not
duplicate any production logic. **This is the cleanest prototype in the tree** —
it is waiting on its inputs, not on its own correctness.

**`ambient.rs`** — intends the full FS Def. 7 tuple. Blocked by the circular
dependency named in audit §6.3 (`FaceContext` demands a face domain that only the
lift can produce). Superseded in part by `lattice.rs`.

**`ConstraintRole` side table** — intends FS §IX `kind(h)`. Not blocked; it is
*live but lossy*, for the Spade reason in §7. Its remedy is now known to be a
public-API change rather than an architectural one.

---

## 9. Cross-references

- Merged root gaps, dependency ordering, remedy categories →
  `CORRECTNESS_GAP_REGISTER.md`
- Ranked recommendation sets A/B/C → `MINIMUM_CORRECTNESS_CUT.md`
- Concept → file/symbol jump table → `CODE_NAVIGATION_INDEX.md`
