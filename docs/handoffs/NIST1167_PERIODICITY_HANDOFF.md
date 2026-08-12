# NIST #1167 — SOURCE-DECLARED B-SPLINE V-CLOSURE LOSS — INVESTIGATION HANDOFF

**Date:** 2026-08-10
**Status:** SOURCE CLOSURE LOSS CONFIRMED; PARITY MECHANISM CONFIRMED; PRODUCTION
REPRESENTATION **UNRESOLVED**. No production change landed. Tree at `30f3d44`,
truck pinned `b4cebf05`, no tracked modifications.

---

## A. Starting provenance

```text
Look commit : 30f3d44  (Pin truck b4cebf05: R01 source-edge traversal under STEP
                       source geometric uncertainty)
Truck pin   : b4cebf05
git status  : clean of tracked modifications; untracked handoff/probe files only
              (P1_SPLINE_PHASE2_HANDOFF.md, R01_HANDOFF.md, opencode.json,
              examples/nist1167_*.rs, examples/nist1169_mesh.rs,
              examples/spline_edge_00007667_*.rs)
NIST baseline (pinned state): 33 models, 7902 declared, 7901 rendered, 1 lost
              (0.01%); sole loss nist_13 #1167, MeshedToNothing/bspline,
              declared_face_index=61, terminal reason ContradictoryDualParity.
ABC baseline provenance: current-tree reference 839179 declared / 811798
              rendered / 27381 lost (NOT comparable to old P3b totals).
```

Confirmed at session start via `git status --short` and `git log --oneline -5`.

---

## B. #1169 verdict

**`#1169` (surface `#507`) is currently a SILENT WRONG-GEOMETRY render — NOT a
correct control.**

Geometric evidence (clean production path, `nist1167_production_mesh`,
`nist1169_mesh` probes):

```text
triangle count      : 89
vertex count        : 90
bbox                : x[-265.0,-249.5] y[79.5,320.3] z[-275.7,-94.5], diag 301.7
area                : 38265.26
max edge            : 241.05   (5 of the 89 triangles have edges > 230)
boundary            : vertex-on-surface residual max=inf, mean=inf; 4/90 vertices
                      have infinite projection residual (indices 9, 11, 23, 25)
```

The 241-unit edge is a **chart-spanning false chord**: it connects
`(-253.27, 320.32, -178.73)` to `(-252.84, 79.51, -189.48)` — two points on
diametrically opposite sides of the ring (y=320 vs y=79), ~241 units apart
across a band whose true width is ~15 units. It is a CDT triangulation artifact
of the broken seam/parity handling, not legitimate boundary geometry.

Reference geometry (direct surface sampling of the intended band, probe
`nist1167_reference`):

```text
surface #507 band u[0.6,1]: area 9680,  bbox diag 302
surface #507 band u[0.7,1]: area 7253,  bbox diag 302
```

The clean `#1169` area (38265) is **~4x the true band area** (7253–10895). The
mesh spans the right bounding box but the material selection is wrong —
overlapping/folded material + spanning chords inflate the area. `rendered=1`
here is a false positive: the same lost V-closure fact that hard-fails `#1167`
silently corrupts `#1169`.

**Classification: `incorrect/suspicious render`.**

---

## C. Evaluator / lattice theorem

**What the source proves:**

`#506` (behind `#1167`) and `#507` (behind `#1169`) are the two
`B_SPLINE_SURFACE_WITH_KNOTS` entities in the corpus that declare
`v_closed = .T.` (u_closed `.F.`), u_degree 1, v_degree 3, 2×34 control net,
v knot vector uniform unclamped over `[-0.0625, 1.0625]`, surface_form
`.UNSPECIFIED.`, self_intersect `.F.`. The STEP text is authoritative and is
NOT inferred from control-point coincidence.

**What the evaluator proves (measured, probe `nist1167_periodicity_probe` and
counterfactual's seam check):**

```text
#506:  S(u,0) == S(u,1)  max positional residual 6.96e-14
       Sv(u,0) == Sv(u,1) max derivative residual 9.10e-13   (active v span 1.0)
       U genuinely open: S(u+1,v) vs S(u,v) max residual 3.62e1
#507:  S(u,0) == S(u,1)  max positional residual 5.68e-14
       Sv(u,0) == Sv(u,1) max derivative residual 1.29e-12
```

So the converted evaluator satisfies seam identification, including the
first derivative, over the source parametric span `[0,1]`. The V-closure is
real and evaluator-compatible.

**What `CertifiedLattice` requires (truck-meshalgo `domain/lattice.rs`):**

- `AxisPeriodStatus::Exact { period, witness }` = a deck generator, with a
  representation-derived `PeriodWitness` (only `ExactRevolutionAngle` /
  `ExactSphereAzimuth` exist). A `generator()` is never numerically sampled;
  sampled agreement at finitely many points establishes nothing.
- `AxisPeriodStatus::Uncertified { declared }` = carries a value usable by the
  legacy path (`declared_period()`), never a generator.
- `NonPeriodic` = no period.

**Why the bridge is valid (for the *lattice*):**

> The STEP source explicitly declares V closed, and the converted evaluator
> satisfies the seam identification `S(u,v0) == S(u,v1)` (position and first
> derivative) over the source parametric span `[v0,v1]`; therefore a V-lattice
> period `P = v1 - v0 = 1.0` is *geometrically justified* as a declared value
> for this surface.

**Where the theorem FAILS (the obstruction this session found):**

Supplying that period to the mesher resolves the parity mechanism but breaks
geometry (section D). The generic tessellator has no correct handling for a
periodic *band* face whose closed boundary loops must be lifted in the periodic
cover: the unwrapped lift places the loops outside the surface's genuine domain
`[0,1]`, and the interior grid evaluates out-of-support parameters to garbage.
The lattice certificate is necessary but **not sufficient**. A correct
production representation additionally needs the meshalgo layer to keep the
lifted boundary UV within a single fundamental domain and to treat seam chords
as periodic identifications rather than physical boundaries. That is a meshalgo
change, not a lattice-provenance change.

---

## D. Counterfactual mechanism

Probe `nist1167_vperiod_counterfactual`: runs the exact production path
(`wrap_shell(...).robust_triangulation_with_torus_outcome`) on the whole shell,
overriding only the lattice for the two target surfaces (identified by converted
surface identity, not face ids), in three modes:
`clean` / `uncert-v1.0` (`Uncertified{1.0}`) / `exact-v1.0` (`Exact{1.0, revolution witness}`).

Boundary UV evidence (probe `nist1167_range`, production sampling range
`evaluation_range()==[0,1]`, self-loop range extension refused because
`basis_is_partition_of_unity` is false at the extended ends):

```text
clean   #606 (inner loop): v 0.526 -> 0.027 (decreasing), seam jump to 0.954,
                             then 0.954 -> 0.526. OUT-AND-BACK, within [0,1].
        #607 (u=1 rim):     v 0.5 -> 1.0 -> (jump) 0.0 -> 0.5. Full period, [0,1].
v-period#606: v 0.526 -> -0.474 continuously (span 1.0). UNWRAPPED, outside [0,1].
        #607: v 0.5 -> 1.5 continuously. UNWRAPPED, outside [0,1].
```

```text
                       clean #1167     v-period #1167   clean #1169    v-period #1169
lattice rank           0                0 / 1            0              0 / 1
periodic axes          none             v (declared)     none           v (declared)
v generator            -                no / yes         -              no / yes
boundary #606 class    closed loop,     full-period      closed loop,   full-period
                       out-and-back     unwrapped        out-and-back   unwrapped
                       in [0,1]         out of [0,1]     in [0,1]       out of [0,1]
boundary #607 class    u=1 rim loop     u=1 rim loop     u=1 rim loop   u=1 rim loop
                       in [0,1]         out of [0,1]     in [0,1]       out of [0,1]
parity outcome         ContradictoryDualParity (ok)       ok (wrong mesh) ok
                       odd_winding=2
repeated_traversals    1                -                24              -
material mesh          none             produced, WRONG  produced, WRONG produced, WRONG
final triangles        0                3225 (7.2e6-unit 89 (241-unit    4668 (4.7e6-unit
                                        chords)          chords)         chords)
mesh UV bbox           -                u[0.57,3.63]     u[0.57,1.0]    u[0.57,2.3]
                                        v[-1.5,1.526]    v[0.011,1.0]   v[-1.5,1.512]
out-of-domain UV       -                1342 / 1657      0              1816 / 2380
```

**Causal confirmation achieved (mechanism as predicted):**

```text
Rank0 / NotPeriodic  ──>  Rank1(V) / periodic cover (declared V period 1.0)
      │                                   │
      ▼                                   ▼
degenerate out-and-back sawtooth     valid full-period closed traversal
(seam jump 0.027->0.954 drawn       (each loop unwrapped across the seam,
 as a physical chord)                span exactly one period)
      │                                   │
      ▼                                   ▼
odd parity (odd_winding=2)          closed mod-2 boundary
      │                                   │
      ▼                                   ▼
ContradictoryDualParity             material mesh produced
(MeshedToNothing)                    (but geometry WRONG)
```

The parity chain is confirmed exactly as hypothesised. But the final stage —
"certified band region" — FAILS: the unwrapped loops sit at `v in [0.5, 1.5]`
and `[-0.5, 0.5]` (the boundary curve parameter origin is offset from the
surface's v origin by 0.5), outside the surface's genuine support `[0,1]`, and
the interior grid evaluates out-of-support parameters to garbage, emitting
7.2e6-unit chords. `uncert` and `exact` produce byte-identical geometry: the
mechanism is driven entirely by `declared_period` (the lift in
`PolyBoundaryPiece::try_new` → `get_mindiff`), not by the certified generator.

---

## E. Production implementation

**Not landed.** Stop condition 2 ("the emitted #1167 geometry is wrong") fires:
the V-period counterfactual's emitted geometry is catastrophically wrong, and
the clean state's `#1169` is also wrong. No production files or fork files were
modified (git status clean of tracked changes).

The preferred design (from the session brief) was evaluated and the pipeline
entry point (`lattice_of` → `meshalgo`) is where it breaks. A correct fix would
require, in addition to the provenance/certificate plumbing:

1. Preserve the STEP `v_closed = .T.` fact into the converted geometry /
   `PolicySurface` (the first loss is in
   `TryFrom<&BSplineSurfaceWithKnots> for BSplineSurface`, truck-stepio
   `in/mod.rs:2535`, which reads degrees/knots/control points but not
   `u_closed`/`v_closed`).
2. Certify a V lattice generator of period `P = v1 - v0 = 1.0` for exactly the
   source-declared-closed B-spline surfaces, gated on evaluator seam
   compatibility (position + derivative, as measured in section C).
3. **meshalgo**: a periodic-band handling so the closed boundary loops are
   lifted in the periodic cover *and* normalized into a single fundamental
   domain, with seam chords treated as periodic identifications (not physical
   toggling boundaries), and the interior grid evaluated only at parameters
   within the surface's genuine domain.

Steps 1–2 are provenance plumbing; step 3 is the actual geometry fix and is a
substantial meshalgo change. Neither was implemented this session.

---

## F. Geometry certificate

No recovered geometry exists to certify. The counterfactual `#1167` mesh fails
every vertex-on-surface / boundary / no-false-chord check:

```text
counterfactual #1167: tris 3225, verts 1657, area 5.17e9, bbox diag 7.16e6,
                      max_edge 7.16e6, on_surface 1129/1657,
                      out-of-domain UV 1342/1657
```

The intended geometry (reference, `nist1167_reference`) is the annular band
`u in [0.6,1] × v in [0,1]`:

```text
#1167 reference band u[0.6,1]: area 9625, bbox diag 302
        x[250.7,264.8] y[79.3,320.7] z[-275.7,-94.3]
```

The clean `#1169` mesh (area 38265 ≈ 4x) and the counterfactual mesh (area
5.17e9) both fail against this reference. No production output for `#1167` /
`#1169` is certified correct.

---

## G. NIST

Unchanged (no production change landed):

```text
33 models, 7902 declared, 7901 rendered, 1 lost (0.01%)
  sole loss: nist_13 #1167 (MeshedToNothing / ContradictoryDualParity)
rendered->lost: 0    lost->rendered: 0
```

---

## H. ABC

No ABC run was performed: no production change exists to measure. The current
tree reference is `839179 declared / 811798 rendered / 27381 lost`. If a
provenance/periodic-band fix is later landed, the before/after comparison must
be per-face (rendered->lost, lost->rendered, rendered->rendered,
lost->lost) grouped by surface kind / u_closed / v_closed / periodic lattice
state / terminal reason — the session's findings do not change that
requirement.

---

## I. Verdict

```text
source-declared closure loss confirmed; production representation still unresolved
```

with the explicitly documented sub-findings:

- The STEP V-closure of `#506`/`#507` is real, lost in the truck-stepio
  conversion, and geometrically evaluator-compatible (position + derivative).
- Supplying the V period resolves the parity mechanism exactly as the causal
  chain predicted (`ContradictoryDualParity` → material mesh produced), which
  confirms the lost-closure causal chain and rules out the alternative "the
  parity contradiction was unrelated to periodicity".
- The naive V-period supply is insufficient: the emitted geometry is
  catastrophically wrong (out-of-domain UV → 7.2e6-unit chords). The geometry
  theorem needs the meshalgo layer to normalize the periodic lift into a
  single fundamental domain and to treat seam chords as periodic
  identifications — a real correctness gap, not a census optimization.
- **`#1169` is a first-class finding**: the same lost source semantic fact
  that hard-fails `#1167` silently corrupts `#1169` into a wrong mesh (4x
  area, 241-unit chart-spanning chords, 4 off-surface vertices). `rendered=1`
  must not be read as correctness for this family.

Do not land the previous B-spline closure heuristic. Do not special-case
`#1167`. Do not weaken `flood_parity`. Do not infer source closure from
knot/control-net patterns.

---

## Re-run

```console
cd C:\Users\stefa\look
cargo build --release --example nist1167_vperiod_counterfactual
target\release\examples\nist1167_vperiod_counterfactual.exe `
  "C:\Users\stefa\Downloads\NIST-PMI-STEP-Files\NIST-PMI-STEP-Files\AP203 with PMI\nist_ctc_02_asme1_ap203.stp"
```

Probes added this session (all untracked, do not commit):
`nist1167_periodicity_probe.rs` (pre-existing), `nist1167_production_mesh.rs`
(pre-existing), `nist1167_vperiod_counterfactual.rs` (new),
`nist1167_range.rs` (new), `nist1167_boundary_uv.rs` (new),
`nist1167_dist.rs` (new), `nist1167_reference.rs` (new), `nist1169_mesh.rs`
(new).
