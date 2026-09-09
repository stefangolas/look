# DOOR-GAP AUDIT — what it takes to render F1, Hypercar, and Falcon-Heavy fully on the kernel engine

2026-09-09, orchestrator, no code changed. Measures the six additions named in
the post-RECLENSUS demand list against the landed tree
(`integration/kernel-bg` 166498c: AUTHOR-FRAME-CARRIERS landed). Every claim is
verified against the source; the four surfaces involved are:

- `corpus/ttc/door.py` (1,742 lines) — the truck-regime drop-in shim (spec-8
  name-for-name vocabulary; stubs refuse typed)
- `truck123d/src/bd_bridge.rs` (3,014 lines) — the data-row executor: its own
  exact analytic facts arms (lathe/prism/loft-of-lines volume, bbox, mesh)
- `truck123d/src/facade.rs` (753 lines) — `run_facade` + the swept-carrier
  boolean ROUTER (`dispatch_swept_carrier_boolean` — routes/records, executes
  nothing)
- the certified funnel — `truck-certified/src/construct/*` (admission,
  volume_facts, loft) + the evidence entry traits (`truck-evidence/src/contact/
  {mod,solver_entry}.rs`: `SplineSsiEntry`, `RestrictedSolverEntry`)

## THE HEADLINE: the dependency wall (blocks items 1 and 2)

`truck123d/Cargo.toml` depends on **pyo3, serde, truck-assembly, truck-base
only**. The recorded dependency law: the loop-side crate cannot name
truck-certified (STATE session-56, applied in PB-011 — the boolean routing
lands as a facade MIRROR of the solver-entry dispatch precisely because of
this). Consequence: the data-row executor can never call the certified funnel
in-process, and the funnel's impl cannot be linked into the pyd. Today the
corpus facts come entirely from bd_bridge's own analytic arms; the certified
machinery is exercised only by tests that name truck-certified directly.

So booleans (item 1) and kernel-grade spline-loft facts (item 2) — the two
carriers every F1 row ultimately needs — each require choosing ONE of three
sanctioned resolutions:

- **(a) The pyo3 binding program** (already booked, "deferred behind the CG
  core", AGENTS.md:24): translate the stabilized facade into pyo3 exports so
  the door reaches the funnel through the sanctioned surface. This IS the CG
  program's first deliverable. Cleanest, biggest.
- **(b) A new sanctioned manifest edge** truck123d → truck-evidence, with the
  needed exact math ported into evidence (the C2 "one sanctioned edge"
  precedent exists — truck-certified → truck-evidence — and was itself a
  recorded amendment). Smaller, but a spec amendment + owner sign-off, and it
  forks the facts authority (bridge arms vs kernel rows).
- **(c) Per-carrier ports into bd_bridge** (the existing pattern: the bridge
  already carries exact Lagrange/Hermite spline interpolation,
  `spline_edge_volume`, divergence-form lathe volume). Fastest per row, but
  duplicates kernel math and erodes the single-substrate doctrine with every
  carrier.

Recommendation: (a) for booleans (they need the real funnel anyway), (c) for
spline-path transport queries (item 4 — the bridge already reconstructs the
interpolating spline exactly; a tangent arm is the same math).

## Item-by-item

### 1. Booleans on the data-row path — BLOCKER: dependency wall (a)
- Exists: the full certified funnel end-to-end (ADM-001 admission → ADM-002
  certificates → ADM-004 facade consult; `BooleanPairVerdict::Routed(
  CertifiedBooleanRoute)`); the OCC regime's boolean evidence
  (`Shape.cut`/`fuse` observed, references recorded).
- Missing: every gram of execution behind the route. The bridge has zero
  boolean ops; `run_facade` returns routes/refusals, not geometry. The corpus
  rows stop typed at their first boolean (airbox, details, monocoque).
- Take: via (a) — pyo3-export the funnel's boolean entry over the stabilized
  facade, then flip the shim's `cut`/`fuse`/`intersection` stubs to record
  boolean rows and submit both operands. Sizing: the binding translation is
  the CG program's booked opening (its LOC is the program's, not countable
  from this audit); the shim flip is ~100–150 LOC; the bridge needs a
  placed-operand convention (operands transformed to canonical before
  dispatch — exactness preserved) ~100 LOC.
- Risk: admission classes. The funnel admits ruled-section pairs; the corpus's
  spline-section lofts admit only after the certificates cover them (that IS
  ADM-002's dispatch) — expect a first pass where several rows refuse typed
  at admission and each refusal books the widening.

### 2. Spline-section loft facts — BLOCKER: same wall; (c)-capable
- Exists: `ProfileEdge::Spline` in `SolidSpec::Loft` (type plumbing landed);
  kernel-side L1 extraction + ADM-003 divergence-form volume over extracted
  patches (truck-certified); bridge-side exact spline interpolation machinery
  (Lagrange/Hermite per-span, `spline_edge_volume`).
- Missing: a volume/area arm for the SMOOTH lofted surface through spline
  sections (ruled between stations is a different surface than build123d's
  smooth loft — the shim's own doc records this honesty line). Two honest
  routes: consume the kernel volume via (a)/(b), or pin the corpus loft
  semantics to `ruled=True` sections where build123d's default smooth loft
  would differ — NO: the references are OCC smooth-loft facts, so the facts
  arm must match OCC's surface. This makes (c) risky here and pushes item 2
  to (a)/(b) as well. Interim: keep the typed refusal naming the open
  carrier (current behavior, correct).
- Take: (a)/(b) + a facts arm that submits the section carrier set to the
  kernel volume row and marshals the certified bracket. ~200–400 LOC beyond
  the wall resolution.

### 3. Non-z revolve — CONTAINED, no wall
- Exists: the lathe arm (profile + arc about z), the landed general-frame
  placement (translate ∘ R ∘ mirror, exact), the shim's `revolve(axis=...)`
  signature (door.py:1280).
- Missing: record the revolve axis as a frame and compose (revolve in local z
  → apply the frame taking z to the axle). Volume is rotation-invariant; bbox
  and mesh transform under the landed placement path.
- Take: ~100–200 LOC across bd_bridge.rs + door.py. Flips the 4 wheel-corner
  rows. One design packet or a direct task; lowest risk on the board.

### 4. Spline-path sweep — CONTAINED, no wall (with one honesty decision)
- Exists: sweep-as-loft-chain kernel row (F1-AUTHORING-ARMS); the shim's
  `sweep` currently refuses EVERYTHING (`"sweep is not a kernel-engine row"`,
  door.py:1407); `Edge.position_at/tangent_at` refuse beyond recorded
  endpoints (door.py:733–752) with the doc explicitly pointing at "the
  kernel's exact arithmetic" as the missing answer.
- Missing: (i) exact tangent/position on the interpolating spline — the
  bridge already reconstructs the same interpolant exactly for volumes, so a
  Rust-side `edge_query` pyo3 export answering position/tangent (exact
  Hermite derivative) is the same math, ~100–150 LOC; (ii) the sweep
  recording arm as a loft chain over stations along the path — the kernel
  row exists, the shim arm + station walk are ~200–300 LOC.
- Decision to pre-make: sweep along a spline needs per-station frames — the
  frame transport law (fixed-order, exact interpolation of the frame) must be
  pinned before coding; that is a small CG-frame-law slice, not the whole
  program. Flips mvac.

### 5. Spline-trimmed extrudes — DEEP END (CG realization proper)
- Exists: the splitter is deliberately frozen (FSSI-LAYER); ADM-003's
  algebraic-trim cell brackets exist for volume facts over pullback
  polynomials (truck-certified); the census rows refuse typed naming this
  carrier (rear_wing, suspension_front/rear, steering_rack).
- Missing: the direct facet realization backend (shared-topology PolygonMesh,
  no sewing) — the core of the CG program's design doc, not a carrier patch.
  Do not port a shortcut: trimming approximations would poison the facts
  gates.
- Take: uncountable from this audit — this IS the CG program's realization
  milestone. The rows stay honestly typed until it lands.

### 6. Reference recording — MECHANICAL, no kernel work, do first
- (i) Hypercar unstaged rows (details, suspension_front, suspension_rear):
  fix the reference-file short-name collision (rename to
  `hypercar_<row>.json` or namespace the manifest), then one OCC door
  recording run per row (~2–10 min each, serial). ~30 LOC of manifest +
  script. Unblocks 3 rows to even be measured.
- (ii) Converged OCC re-records for the biased spline-row references
  (nozzle_assembly et al.): the recorded ~1.16e-4 BRepGProp volume bias —
  re-record with the converged call per the booked direct task; unblocks
  nozzle to DNF-FACTS→runnable. Zero kernel LOC; pure oracle hygiene.

## Rollup

| Item | Blocker class | LOC (beyond the wall) | Flips |
|---|---|---|---|
| 1 booleans | dependency wall → resolution (a) | ~200–250 shim/bridge + the binding program | airbox, details, monocoque (admission-widening permitting) |
| 2 spline-loft facts | same wall → (a)/(b) | ~200–400 | the dominant F1 carrier class |
| 3 non-z revolve | none | ~100–200 | 4 wheel corners + hypercar wheels/brakes |
| 4 spline-path sweep | none (one frame-law pin) | ~300–450 | mvac + tube rows |
| 5 trimmed extrudes | CG realization milestone | the program | rear_wing, suspensions, steering_rack |
| 6 references | none | ~30 + recording runs | 3 hypercar rows + nozzle |

**The honest sequencing:** item 6 now (hours); items 3+4 as one bridge packet
(days); item 1 is the pyo3 binding program's first slice — i.e. the CG program
starts here whether we like it or not; item 2 rides the same resolution; item
5 is the program's midpoint, not its opening. Rendering F1 fully = items 1+2+3
(+4). Hypercar adds nothing new beyond item 6 and the band-loft end-tangent
variant of item 2. Falcon-Heavy = items 4+6 (+1 for its assembly rows).

The single decision that unblocks the most: **authorizing the pyo3 binding
program (a) as the CG program's opening packet** — it is already booked, it is
the only lawful route to booleans and kernel-grade loft facts, and every row
that matters routes through it.
