# Code navigation index

**Audited artifacts.** `look` @ `e7fb495`; truck-fork @ `39346191`;
`truck-*` pinned @ `79eaaf36`; `spade` 2.15.1.

Path prefixes:

- `[fork]` = `C:\Users\stefa\truck-fork\truck-meshalgo\src\tessellation\`
  — **the locally patched crate** (`Cargo.toml:36-37`), not the pinned rev.
- `[pin]` = `~/.cargo/git/checkouts/truck-885f4dcc04f583da/79eaaf3/`
- `[look]` = `C:\Users\stefa\look\`
- `[spade]` = `~/.cargo/registry/src/*/spade-2.15.1/src/`

---

## Entry points and composition

| Concept | File | Type / function | Notes |
|---|---|---|---|
| STEP file → mesh | `[look] src/step.rs` | `:87` `Table::from_owned_data_section`, `:183` call site | the whole production path starts here |
| **Composition layer** | `[look] src/step/lattice.rs` | `lattice_of` `:34-51` | the only place the concrete surface enum is matched |
| Descriptor entry point | `[fork] mod.rs` | `LatticeMeshableShape::robust_triangulation_with_lattice` `:357-392` | audit §6.4 "option C", landed in `39346191` |
| Legacy entry points | `[fork] mod.rs` | `:260-354` | all pass `unevidenced_lattice` `:431-433` |
| Tolerance policy | `[look] src/step.rs` | `:26-50`, `model_tolerance` | relative to model diameter |
| Loss accounting | `[look] src/step.rs` | `:196-…` `FaceTally` | hand-splits `None` from `Some(empty)` — see G8 |

---

## Surface representation

| Concept | File | Type / function | Notes |
|---|---|---|---|
| **The production `Surface` enum** | `[pin] truck-stepio/src/in/step_geometry/mod.rs` | `Surface` `:399-405` | **5 variants**, not the 4-variant modeling enum |
| Elementary surfaces | same | `ElementarySurface` `:171-177` | Plane, Sphere, Cylindrical, Toroidal, Conical |
| Cylinder / cone type | same | `:25`, `:29` | both `Processor<RevolutedCurve<Line<Point3>>, Matrix4>` |
| Sphere type | same | `:23` | `Processor<Sphere, Matrix4>` — **not** a `RevolutedCurve` |
| Torus type | same | `:27` | `Processor<Torus, Matrix4>` — **not** a `RevolutedCurve` |
| Modeling `Surface` (not on look's path) | `[pin] truck-modeling/src/geometry.rs` | `:132-142` | 4 variants; truck's own builder path |
| Shell conversion | `[pin] truck-stepio/src/in/convert.rs` | `to_compressed_shell` `:564-568` | fixes `S = stepio Surface` |

---

## Parameter semantics (period, range, axes)

| Concept | File | Type / function | Notes |
|---|---|---|---|
| Trait contracts | `[pin] truck-geotrait/src/traits/surface.rs` | `parameter_range` `:33-36`, `try_range_tuple` `:40-46`, `u_period` `:49-51`, `v_period` `:53-56` | defaults: unbounded range, `None` period. **No injectivity or fundamentality promised** |
| **Exact 2π** | `[pin] truck-geometry/src/decorators/revolved_curve.rs` | `v_period` `:143-145`; `subs` `:84-87` | period is a property of `rotation_matrix(v)` |
| Generatrix period | same | `u_period` `:139-141` | forwards `curve.period()` — accessor only; for a `Line` generatrix, `None` |
| **Apex-extended u range** | same | `parameter_range` `:116-134`, esp. `:130-132` | `u_max = t0 + 10×(t1−t0)` — a heuristic window, not a declared range |
| Fabricated plane range | `[pin] truck-geometry/src/specifieds/plane.rs` | `parameter_range` `:136-139` | `[0,1]²` unconditionally |
| Spline knot-span range | `[pin] truck-geometry/src/nurbs/bspsurface.rs` | `parameter_range` `:304-313` | genuinely exact |
| **Processor axis map** | `[pin] truck-geometry/src/decorators/processor.rs` | `subs` `:217-222`, `uder`/`vder` `:224-235`, `uuder`/`uvder`/`vvder` `:237-256`, `parameter_range` `:259-265`, `u_period`/`v_period` `:267-282`, `normal` `:288-296` | all swap consistently on `orientation == false` |
| **Axis-map defect** | same | `search_parameter` `:507-521` | result transposed, **hint not** — gap G11 |
| Certified lattice | `[fork] domain/lattice.rs` | `CertifiedLattice` `:102-183`, `AxisPeriodStatus` `:44-61`, `PeriodWitness` `:35-40` | `declared_period` `:66-72` vs `generator` `:75-80` |
| Axis restatement | `[fork] domain/lattice.rs` | `swapped` `:151-156`; `[look] src/step/lattice.rs:95-100` | applies the `Processor` inversion to the caller's convention |

---

## Topology and provenance

| Concept | File | Type / function | Notes |
|---|---|---|---|
| Compressed face | `[pin] truck-topology/src/compress.rs` | `CompressedFace` `:154-160` | `boundaries: Vec<Vec<CompressedEdgeIndex>>` — **no outer/inner distinction** |
| Edge use + orientation | same | `CompressedEdgeIndex` `:35-40` | `{index, orientation}` |
| Face provenance | same | `:161-168` | source identity; the only fact surviving to the caller |
| Bound → edge uses | `[pin] truck-stepio/src/in/convert.rs` | `face_bound_to_edges` `:185-256` | orientation composed at `:244`; **TOP-003 discharged** at `:253-255` |
| Collapsed bound handling | same | `:200-211` | a `VERTEX_LOOP` contributes no trim segment |
| Face sense | same | `:468` | `same_sense` folded into the geometry |
| p-curves | — | **absent** | no p-curve handling anywhere in `truck-stepio/src/in/convert.rs`; FS §VI precedence level 1 unavailable |

---

## The executed tessellation path

| Concept | File | Type / function | Notes |
|---|---|---|---|
| Shell driver | `[fork] triangulation.rs` | `cshell_tessellation` `:172-380` | |
| Edge sampling | same | `tessellate_edge` `:184-256` | period-extension `:235-241`; 16-step fallback `:243-251` |
| Face driver | same | `tessellate_face` `:329-365` | `filter_map` hazard `:345` |
| Projection functions | same | `by_search_parameter` `:43-54`, `by_search_nearest_parameter` `:56-69` | the `sp` argument |
| **The lift** | same | `PolyBoundaryPiece::try_new` `:471-863` | |
| Branch rule | same | `get_mindiff` `:866-872` | |
| Ambiguity detection | same | `AMBIGUOUS_STEP_FRACTION` `:919`, `MAX_LIFT_REFINEMENTS` `:922`, logic `:608-622` | **silent acceptance at `:623`** |
| Compatibility gate | same | `:559-577`, `COMPATIBILITY_FACTOR` `:890`, `compatibility_factor()` `:899-908` | off by default |
| Deck normalisation | same | `:659-669` | per-piece centroid; `quot_u`/`quot_v` discarded |
| **Closure and stitching** | same | `PolyBoundary::new` `:1051-1331` | |
| Closure classification | same | `BoundaryClosure` `:926-936`, `periodic_displacement` `:939-950`, use `:1073-1089` | |
| Winding discard | same | `:1110-1130` | |
| Two-loop merge | same | `:1134-1166` | |
| Collapsed-pair seam | same | `CollapsedPeriodicBoundaryPair::try_classify` `:2234`, called `:1168` | **live production** |
| Synthetic closure | same | `:1222-1295` (1 or 2 open pieces), `:1315-1329` (no loops) | |
| Range normalisation helper | same | `normalize_range` `:955-977` | |
| Signed area | same | `signed_area` `:988-993` | diagnostic only; sign is not orientation-invariant (DOM-004) |
| Derived face extent | same | `working_range` `:1025-1048` | `TRUCK_FACE_DOMAIN`, off by default |
| Point-in-domain test | same | `include` `:1334-1357` | random-ray odd/even; degenerate → `false` |
| **Constraint insertion** | same | `insert_to` `:1360-1599` | |
| Vertex welding | same | `:1409-1412` | linear scan, `distance_2 < 1e-12` |
| Role recording | same | `:1452-1472` | lookup fails on split chains |
| Sampling grid | same | `insert_surface` `:1910-1963` | role assigned `:1924` |
| **Material classification** | same | `triangulation_into_polymesh_outcome` `:1966-2181` | |
| Parity base state | same | `:1980-1982` | outer face = 0 |
| Role-gated toggle | same | `:2000-2001` | audit A1 |
| Seam vertex role | same | `:2053-2054` | assumes domain origin 0; diagnostic only |
| Material selection | same | `:2098-2121` | parity 1; degenerate triangles dropped |
| **Failure erasure** | same | `trimming_tessellation` `:1885-1907`, esp. `:1900-1905` | gap G8 |
| Polyline on surface | same | `polyline_on_surface` `:2205-2222` | used by every stitching site |

---

## Roles, outcomes, failures

| Concept | File | Type / function | Notes |
|---|---|---|---|
| Edge role enum | `[fork] triangulation.rs` | `ConstraintRole` `:1627-1644` | 5 variants; only `PhysicalBoundary` and `SurfaceSampling` are ever constructed |
| Role side table | same | `ConstraintRoles` `:1654-1701` | `record` `:1665`, `role_of` `:1675`, `toggles_material` `:1687-1700` |
| Unresolved-role counter | same | `unresolved_at_flood` `:1659` | the honest size of the CDT-001 gap |
| Typed failure reasons | same | `TessellationFailureReason` `:1703-1716` | 11 variants; 2 never constructed |
| Failure payload | same | `TessellationFailure` `:1718-1725` | carries bound, edge use, constraint ids, uv |
| Outcome sum type | same | `TessellationOutcome` `:1819-1823` | correct shape, discarded at `:1900` |
| Sidecar diagnostics | same | `VertexMetadata` `:1797`, `VertexRoles` `:1748`, `SeamPair` `:1768`, `SingularGroup` `:1789` | `seam_pairs`/`singular_groups` always empty (`:2177-2178`) |

---

## Prototype / disconnected architecture

| Concept | File | Type / function | Status |
|---|---|---|---|
| Ambient schema | `[fork] domain/schema.rs` | `ParametricQuotient` `:83-93` | prototype, unused |
| Deck lattice (proto) | same | `DeckLattice` `:9-25` | prototype; superseded by `CertifiedLattice` |
| Singular stratum | same | `SingularStratum` `:73-80`, `StratumCertificate` `:41-70` | prototype; "certificate" holds residuals, proves nothing |
| Schema failure | same | `SchemaFailure` `:95-101` | prototype |
| Schema trait | same | `ParametricQuotientSurface` `:107-112` | implemented only by `adapters/revolution.rs:30` |
| **Deck potential solver** | `[fork] domain/deck.rs` | `DeckPotentialUnionFind` `:26-77` | prototype; **correct QUO-004 solver, never called** — waiting on inputs |
| Ambient prototype | `[fork] domain/ambient.rs` | whole file, 524 lines | frozen at `5c659209`; puts face domain on the wrong side of the lift |
| Projection prototype | `[fork] domain/projection.rs` | `TraversalSemantics`, `project_boundary_curve` | referenced only from tests `:2546-2566` |
| Other prototypes | `[fork] domain/{quotient,canonical,evidence,plan}.rs`, `adapters/revolution.rs` | — | closed cluster, zero production references |
| Quotient lift | `[fork] triangulation.rs` | `solve_quotient_lift` `:2345-2409` | prototype; sole caller is the test at `:2423` |
| Singular transition | same | `reconcile_singular_transition` `:429-465` | prototype; tests only — production activation reverted 2026-08-02 |

---

## Foreign realization layer (Spade 2.15.1)

| Concept | File | Function | Notes |
|---|---|---|---|
| Crossing predicate | `[spade] cdt.rs` | `can_add_constraint` `:445-448` | `false` ⟺ proper crossing of a constraint edge |
| Crossing helper | same | `contains_any_constraint_edge` `:463-471` | matches only `is_constraint_edge()` |
| Insertion (current) | same | `add_constraint` `:558-563` | returns "count changed"; **splitting documented at `:541-544`** |
| **Insertion (chain-returning)** | same | `try_add_constraint` `:807-817` | returns `Vec<FixedDirectedEdgeHandle>`; atomic on conflict — **the G5 remedy** |
| Insertion with splitting | same | `add_constraint_and_split` `:1227-1246` | also returns the chain, plus a vertex constructor |
| Core resolver | same | `resolve_splitting_constraint_request` `:866-877` | returns the chain; `add_constraint` discards it |
| Conflict enumeration | same | `get_conflict_resolutions` `:819-…`, `get_conflicting_edges_between_vertices` `:707` | the latter is used by the probe path at `[fork] :1481` |

---

## Formal documents

| Concept | File | Section |
|---|---|---|
| Ambient schema (Ω,Λ,N,Σ,S,C) | `[look] FORMAL_SYSTEM_STEP_INGESTION.md` | Def. 7, §III |
| Admissible normalized arc | same | Def. 9 |
| Orientation normalization, material-left axiom | same | §V, Def. 12 |
| Curve-on-surface evidence precedence | same | §VI, Def. 13 |
| Lift, deck displacement, potential ψ | same | §VII, Def. 14 |
| Finite cover, Lemmas 1-2 | same | §VIII, Def. 15-17 |
| Arrangement normalization, `~_Λ` vs `~_Σ` | same | §IX, Def. 18 |
| Material constraint system, trichotomy | same | §X, Def. 19-21 |
| Region validity, singular links | same | §XI, Def. 22-25 |
| Mandated architecture / module list | same | §XVIII |
| Contract registry (TOP/GEO/QUO/DOM/ARR/CDT/MSH/SHL/RES) | `[look] MATHEMATICAL_FOUNDATION.md` | Part III, §13-20a |
| `Unknown` semantics, policy vs detection | same | §27, §31 |
| Prior audit, transition table | `[look] REFINEMENT_AUDIT.md` | §1-2 |
| Prior Spade read | same | §4 |
| Production ambient audit, layering options | same | §6 |
| Corrections to the above | `[look] CODEBASE_CORRECTNESS_MAP.md` | §0 |
