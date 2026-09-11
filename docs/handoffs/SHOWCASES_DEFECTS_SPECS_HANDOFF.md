# Session handoff — showcases, defect audits, and program specs

**Session:** 2026-09-05 (interactive; not a loop-packet session).
**Author:** opencode session working the showcase/defect/spec thread.
**Machine state at write time:** kernel HEAD includes overnight landings
`CC-031-BLEND-VARRADIUS` (`9a84a21`), `CC-DEF-BREP-FIXES` (`10a1d13`), and a
post-landing merge-marker repair in `truck-certified/src/construct/mod.rs`
(fixed by the owner mid-session; the tree compiles green as of this
handoff).

---

## 1. What exists now (all new this session)

### 1.1 `showcases/` — workspace member (added to root `Cargo.toml`)

| Path | Role |
|---|---|
| `src/waterslide.rs` (385 lines) | composite spine (drop→helix→runout), 3 frame laws × 2 backends, pool/tower prisms, union attempt |
| `src/teapot.rs` (445) | revolve body (rhombic lattice-clean silhouette), spout/handle sweeps, Pappus oracle, CC probes |
| `src/amphora.rs` (360) | handle twins (FixedPlane, rotated table), octagon foot prism, full CC-port probe battery |
| `src/cc_ports.rs` | anti-corruption trait over in-flight CC packets (`loft`, `gordon`, `blend_var_radius`, `clear`, `canal_*`, `shell_*` + amphora-facing variants); `RadiusLaw` with the portable PCHIP evaluation; `LandedPorts` defers everything typed |
| `src/spine.rs` | composite path math, `spline_from_path` (clamped cubic `try_interpole`, arclength-normalized), `spline_through_points` (uniform params) |
| `src/profile.rs` | `u_chute` (concave — facet-path refusal fixture), `trapezoid_chute` (convex, showcase), `regular_polygon`, CCW normalizer |
| `src/harness.rs` | BREP census + raw parametrization volume (`brep_volume`, N=96, sign is a handedness diagnostic), STL/STEP writers, `write_solid_stl` (BREP→meshalgo→STL display route), report schema |
| `src/bin/{waterslide,teapot,amphora}.rs` | table-driven entry points |
| `tests/battery_waterslide.rs` (10 tests) | certificates, determinism, handedness + lattice witnesses, U-chute refusal fixture |
| `tests/battery_construction.rs` (6 tests) | typed-refusal battery; the three SEM witnesses are **inverted to assert the fixed behavior** |
| `examples/` probes | `frame_probe`, `arrange_probe` (13-case lattice isolation), `interpolation_probe`, `knot_probe`, `mesh_probe`, `construction_probe`, `recipe_probe` |
| `tables/{waterslide,amphora,teapot}.json` | the portable model tables (truck123d payload) |
| `viewer/` | self-contained Three.js r147 viewer (vendored min.js/STLLoader/OrbitControls); served by **detached** `python -m http.server 8137 --directory showcases` — live at `http://127.0.0.1:8137/viewer/index.html`, serves 8 STLs |

**Test state: 16/16 green** (dev profile, `-j 2`; a transient clang-link
failure once — retry/-j 2 fixed it).

**Bins must run in `release`/`quick`-release**: the teapot body's
circle-carrying cone faces hit the booked debug-assertion self-loop trap in
tessellation (trap now empirically confirmed). `cargo build --release -p
showcases --bins` is cheap (~30 s; vendor stack prebuilt).

### 1.2 Model state (honest)

- **Waterslide**: helical chute as a ring-4 trapezoid ribbon — *substituted*;
  the concave U works on the BREP path (facet convexity gate only) — rebuild pending.
- **Teapot body**: rhombic vessel — *substituted*; forced by the lattice
  defect (§2). Spout/handle sweeps certified, floating (no union).
- **Amphora**: handles + octagon foot puck; body waits on CC-010 loft.
- Tessellation density raised 8× (stations 96, rings 16–24). Bins run from
  `target/release`; viewer picks up STLs without restart.

## 2. Defect ledger (the session's main product)

Filed in `docs/defects/` + registered in `DEFECT_INDEX.md`, each with a
permanent witness:

| ID | One-line | Status |
|---|---|---|
| ORI-FRAME-HANDEDNESS-001 | `ArchitecturalUp` frames left-handed (`t×n = −b`) | **Closed** by `10a1d13` (witness inverted: same-sign volumes) |
| ORI-FRAME-ORTHONORMALITY-GATE-001 | frame laws bypass `Frame3::try_new` | **Closed** (laws route through the gate; `FixedPlane` refuses non-planar spines; `RadialAboutAxis` refuses radial-component tangents — waterslide radial arm now a typed-refusal exhibit) |
| SEM-FACET-SCALE-ZERO-001 | facet path accepted through-zero `Scale` | **Closed** (shared `validation.rs` validators; witness asserts `ProfileCollapse`) |
| SEM-FACET-CORRESPONDENCE-TRUNCATION-001 | facet path zip-truncated mismatched correspondence | **Closed** (witness asserts `ProfileCorrespondenceMismatch`) |
| NUM-INTERPOLE-OVERSHOOT-001 | `try_interpole` oscillates catastrophically (30 m → 8e10 m, n=51→257) | **Closed for the standing API** (SW admission + BOUND_FACTOR; certified solve remains CC-001); showcases pinned to n ≤ 48 spine samples |
| **SEM-ARRANGE-DYADIC-CONTRACT-001** | arrange requires a **dyadic extended-line intersection lattice** (stronger than the documented "dyadic vertices"); violations surface as misleading `RootNotIsolated` | **OPEN** — 13-case isolation in `arrange_probe.rs`; near-parallel and exact-parallel exonerated; unowned, needs a loop packet |

The 5-closures/1-open cycle ran end-to-end: showcases surfaced → defects
filed with witnesses → the loop landed all five in one packet → witnesses
broke → inverted to pin the fixes. **The witness-inversion discipline works;
keep it.**

## 3. Program specs written (booking documents)

| Doc | Content | State |
|---|---|---|
| `docs/CERTIFIED_INTERACTION_ENGINE_SPEC.md` | the Boolean theory (adopted draft, §0–15) + §16 substrate reconciliation audit | root theory |
| `docs/CERTIFIED_INTERACTION_ENGINE_BUILD_SPEC.md` | BIE program: stage-by-stage tie-in map into the landed boolean pipeline, 8 packets (BIE-000..007), **~13.7k LOC (band 11–17k)**, ~4 genuinely new mechanisms, classifier/decision reuse discovered (carrier-agnostic parity propagation), not-built list | booked-not-dispatched; sequence AFTER CC solver chain (reuses CC-001/003/020/030) |
| `docs/TRUCK123D_PY_BRIDGE_SPEC.md` | PB program: build123d-shaped Python, scoped to the showcases; 8 packets (PB-000..007), ~6.5k LOC, zero geometric content; booleans are a non-goal (BIE owns them) | can dispatch immediately; parallelizes well |

Key reuse facts the specs established (do not re-derive): `KrawczykSystem<const N>`
landed; Bernstein hull landed 1-D/2-D (n=4 boxes absent); the classifier is
seed-and-propagate parity (carrier-agnostic); `SpineFrameSweep` stores
windowed domains so trimmed output faces need no new types; `EntityId`/`OpKind`/`Selector`
landed in `truck-topology/src/entity_id.rs`.

## 4. Gap accounting for the showcases

- **~60% fixed by CC** (running): amphora + teapot bodies via loft, junction
  blends (landed, unwired), var-radius, thickness, canal, Gordon.
- **~15% by BIE**: whole-assembly unions, STEP-solid interchange — claims, not looks.
- **~20% is landed-but-unused by me**: concave U-chute on the BREP path,
  pool-as-sweep-terminus, tower, better foot within the lattice, density (done).
- **~10% orphaned**: the lattice defect packet (small loop job);
  TR-NRB-001 STEP-out of constructive surfaces; "certified assembly" evidence type.

## 5. Queued next steps, in order

1. **Wire `cc_ports.blend_var_radius` to the landed
   `truck_certified::construct::blend*`** (CC-030/031 landed overnight) — the
   teapot junction blend becomes real geometry; the port stops reporting deferred.
2. **Waterslide honesty pass**: concave U-chute via `spine_sweep` + `write_solid_stl`;
   splash-pool terminus via `LinearCorrespondence`; tower re-add.
3. **Amphora foot**: tapered lathe-style profile within the lattice rule (slope
   pairs with power-of-two differences, or {0, ∞} stepped), 32-gon rings.
4. **Teapot body**: best silhouette the lattice allows (stepped-shoulder pot
   using slopes {0, ±1, ∞} or {0, ±0.5, ±1} same-sign rules) — or wait for
   CC-010 and loft it like the amphora.
5. **Write `tests/battery_teapot.rs` + `battery_amphora.rs`**: Pappus-exact
   body volume, handle equivariance (`rotate_solid` metamorphic), CC-port
   deferral census, U-chute positive test (inverts the concave fixture).
6. **Book the lattice packet** (small loop job; `arrange_probe.rs` is the
   regression test) and correct `BREP_GENERATION_API.md` §6's contract wording.
7. Optional: tessellated-body renders via rebuilt `look.exe` (janitor
   reclaimed `target/quick/look.exe`; viewer covers display meanwhile).

## 6. Traps and environment notes (read before touching anything)

- **LSP diagnostics in this workspace are chronically stale** — trust
  `cargo check`, not the editor.
- `NUM-INTERPOLE-OVERSHOOT-001`: keep `spine_samples ≤ 48` (the
  `try_interpole` SW admission now refuses some larger uniform-knot cases;
  knot_probe has the scaling table). Stations ≠ spine samples: stations can
  be dense freely.
- BREP volumes in `harness::brep_volume` are **raw parametrization-signed**
  by design (handedness diagnostic); facet volumes are normalized-positive.
  Don't "fix" the sign.
- The `booleans`/`cc_ports` report sections are evidence, not failures:
  `UnsupportedEnvelope(NonCanonicalCarrier)` for unions of swept parts is
  the booked boundary until BIE lands.
- The viewer's python server is detached (`Start-Process`); it survives the
  shell. Kill via port or reuse it.
- Tables (`showcases/tables/*.json`) are the truck123d payload — geometry
  changes go in tables, not code; the Rust builders are thin interpreters.

## 7. File inventory (new, this session)

```
showcases/                       (crate; workspace member)
docs/defects/ORI-FRAME-HANDEDNESS-001.md          [Closed]
docs/defects/ORI-FRAME-ORTHONORMALITY-GATE-001.md [Closed]
docs/defects/SEM-FACET-SCALE-ZERO-001.md          [Closed]
docs/defects/SEM-FACET-CORRESPONDENCE-TRUNCATION-001.md [Closed]
docs/defects/NUM-INTERPOLE-OVERSHOOT-001.md       [Closed, standing API]
docs/defects/SEM-ARRANGE-DYADIC-CONTRACT-001.md   [OPEN — needs owner]
docs/defects/DEFECT_INDEX.md                      (6 new rows)
docs/CERTIFIED_INTERACTION_ENGINE_SPEC.md         (theory + reconciliation)
docs/CERTIFIED_INTERACTION_ENGINE_BUILD_SPEC.md   (BIE program)
docs/TRUCK123D_PY_BRIDGE_SPEC.md                  (PB program)
```

Nothing in `vendor/truck` was edited by this session. The only kernel-side
changes came from the loop's own overnight packets.
