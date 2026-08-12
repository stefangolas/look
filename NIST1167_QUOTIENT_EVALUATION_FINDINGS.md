# NIST #1167 — PERIODIC-COVER vs EVALUATOR-DOMAIN SEPARATION — SESSION FINDINGS

**Date:** 2026-08-11
**Status:** THEOREM PROVEN AND VALIDATED END-TO-END; PRODUCTION REPRESENTATION
**BLOCKED** (stop condition 2). No production change landed. Tree at `30f3d44`,
truck pinned `b4cebf05`, no tracked modifications. All probe/diagnostic work is
in untracked example files.

---

## 0. Session result in one paragraph

The central hypothesis of this session is **proven and validated**:

> A periodic chart may use unwrapped/deck-lifted UV coordinates for topology,
> but every evaluation of a periodic surface must be performed through the
> quotient map into an evaluator-valid fundamental domain.

The diagnostic counterfactual (`nist1167_quotient_counterfactual.rs`) shows that
supplying the source-declared V period **and** evaluating through the quotient
`v_eval = v_cover mod 1.0` **and** joining the two genuine full-period deck loops
(`DeckConsistent` two-loop join) **and** rejecting spurious out-of-domain
projections produces a geometrically correct annular band for both #1167 and
#1169:

```text
                    clean (current tree)      period+quotient+deckjoin+guard
#1167   parity       ContradictoryDualParity  ok
        area         -                       8354  (trim-exact ref 8180)
        max edge     -                       24    (was 7.16e6, then 238)
        direct res   -                       8.6e-3
#1169   parity       ok (WRONG mesh)         ok
        area         38265 (4x too big)      8322  (trim-exact ref 8177)
        max edge     241 (false chord)       24
        direct res   1.19e-2                 1.19e-2
```

The quotient map alone fixes the evaluator-domain garbage but not the whole
band: the correct periodic-band realization additionally requires the
`DeckConsistent` two-loop join and a domain guard on the parameter inverse.
Those are separate meshalgo defects that the period supply exposes. Production
implementation is blocked on their scope (multi-crate, truck re-pin, regression
risk), not on any remaining uncertainty about the causal chain.

---

## 1. First cover→evaluator mismatch

Where cover UV is consumed as native evaluator UV (trace from
`PolyBoundaryPiece::try_new` / `get_mindiff`):

| function / stage (truck-meshalgo `triangulation.rs`) | UV role | cover-aware? | wraps before eval? | can receive UV outside native domain? |
|---|---|---|---|---|
| `try_new` line 4524 | raw-projection residual check | no (native projection output) | N/A | no |
| `try_new` line 4848/4865 | degenerate-boundary dense reconstruction | partial (`+ frac·P`) | no | yes (latent, point-degenerate boundaries only) |
| `detect_degenerate_trim` line 5130 | `uder`/`vder` metric scale on boundary samples | no | no | yes (diagnostic-only, Detector B) |
| `PolyBoundary::new` line 6834 | collapsed-pair apex branch | no | no | yes (cone-like only) |
| `PolyBoundary::new` lines 6877–7001 | rectangle-closure corners | — | — | no (native-range corners) |
| `insert_surface` line 8540 | grid vertex *placement* at cover bbox | no | N/A (insertion) | N/A |
| **`triangulation_into_polymesh_outcome` line 8760** | **interior grid vertex physical point** | **no** | **no** | **YES — FIRST BREAK** |
| **`triangulation_into_polymesh_outcome` line 8787** | **interior grid vertex normal** | **no** | **no** | **YES — FIRST BREAK** |
| `polyline_on_surface` line 8982 | seam/closure polyline realization | no | no | yes (latent) |

The **first physical-evaluation call that consumes out-of-domain cover
coordinates** is `surface.subs(p.x, p.y)` at `triangulation.rs:8760` (interior
grid vertex), immediately followed by `surface.normal(p.x, p.y)` at `:8787`.
The grid vertices were placed at cover coordinates by `insert_surface` because
its bounding box is derived from the boundary loops' cover UVs. Boundary
vertices are exempt: `boundary_map` (populated at `insert_to`, lines
7166–7183) carries the *projected* source-curve 3D point, so boundary vertices
are never re-evaluated at realization.

For the #1167/#1169 counterfactual the two non-degenerate full-period loops are
not joined by the legacy two-loop join (it requires both loops to be
area-degenerate), so no seam closures fire; line 8760/8787 is the only
evaluation site. The other rows are latent sites the quotient adapter must also
cover for generality.

## 2. Existing periodic-surface assumption

Cylinder, cone, sphere and torus are **globally periodic evaluators**:

- `RevolutedCurve::subs(u, v) = origin + rotation_matrix(v)·(curve(u) − origin)`
  — every `v`, including `v > 2π` and `v < 0`, evaluates through the rotation
  map. This is why generic periodic-cover UV has always worked for analytic
  surfaces: the evaluator supports every deck copy by construction.
- `Sphere::subs` enters the azimuth only through `(cos v, sin v)` — same.

`BSplineSurface::subs` (truck-geometry `bspsurface.rs:576`) is Cox–de Boor over
the surface's own knot domain `[0,1]` (`knot_vec.rs:166` `bspline_basis_functions`
uses `floor(t)` over the knot vector; outside the domain the basis no longer
partitions unity and extrapolates to garbage).

**Conclusion:** meshalgo's implicit invariant is "periodic evaluator supports
all deck copies". It holds for revolution primitives and fails for the first
surface family — source-declared-closed B-spline — where the topology is
periodic but the evaluator support is bounded. This is the
"periodic topological chart ≠ globally periodic evaluator" distinction, and it
is the precise reason analytic surfaces worked and the B-spline band did not.

## 3. Quotient-map theorem

```text
cover coordinate semantics   The boundary lift (get_mindiff, PolyBoundaryPiece::
                             try_new) and the CDT reason about unwrapped deck
                             coordinates v_cover ∈ ℝ; v_cover and v_cover + P
                             name the same physical surface point.

native evaluator domain      The generic BSplineSurface evaluator supports only
                             v ∈ [v0, v1] = [0, 1] (its active knot span,
                             P = v1 − v0 = 1.0). Outside, Cox–de Boor
                             extrapolates to garbage.

period representative rule   v_eval = v_cover − P·⌊v_cover / P⌋ ∈ [0, P).
                             Every evaluator call (subs, uder, vder, uuder,
                             uvder, vvder, der_mn, normal, normal_uder,
                             normal_vder) on a periodic axis goes through this
                             map; non-periodic axes forward unchanged.

seam behavior                The map is continuous on the physical surface:
                             S(u,0) = S(u,1) to 6.96e-14 and Sv(u,0) = Sv(u,1)
                             to 9.10e-13 (measured), so the representative jump
                             at v_cover ∈ ℤ is invisible in the quotient. The
                             map never identifies seam endpoints as a topology
                             change: it only selects the evaluator
                             representative. Boundary cover coordinates,
                             winding, parity, deck class and CDT topology are
                             untouched.
```

Edge cases: `v = v1` → `v_eval = 0`; `v = v0` → `v_eval = 0`; values within
epsilon of the seam stay continuous; negative deck copies and multiple periods
away are handled by `floor`; derivatives at the seam are continuous to the
measured `Sv` residual.

## 4. Counterfactual results (mechanism table)

Diagnostic `nist1167_quotient_counterfactual.rs` runs the exact production path
(`wrap_shell(...).robust_triangulation_with_torus_outcome`) on the whole shell,
overriding only the lattice for #506/#507. With the temporary local fork
(`TRUCK_DIAG_DECK_JOIN` eager deck-consistent join), a fourth mode is measured.
`TRUCK_DIAG_DECK_JOIN` is diagnostic-only and reverted; the workspace is restored
to the pinned fork.

```text
                        clean        period      period+quot    period+quot
                                                                 +deckjoin+guard
#1167 lattice          NonPeriodic  V=1.0       V=1.0          V=1.0
  boundary #606        out-and-back full-period full-period    full-period
                       [0,1] seam   unwrapped   unwrapped      unwrapped
                       jump         (u=3.63     (u=3.63 spike  (u in [0.57,1])
                                    spike)      rejected)
  parity               ContradictoryDualParity  ok             ok            ok
  material mesh        none         WRONG       band+spike     CORRECT band
  triangles            0            3225        3225           237344 (dense)*
  area                 -            5.17e9      47499          8354
  max edge             -            7.16e6      238            24
  direct residual      -            -           8.6e-3         8.6e-3
  evaluator ood v      -            1342/1657   wrapped        wrapped
  evaluator ood u      -            yes (3.63)  yes (3.63)     none (guard)

#1169 lattice          NonPeriodic  V=1.0       V=1.0          V=1.0
  parity               ok (WRONG)   ok          ok             ok
  area                 38265        2.07e9      33158          8322
  max edge             241          4.7e6       241            24
  direct residual      1.19e-2      -           1.19e-2        1.19e-2
```

*Dense-triangle artifact: `parameter_division` over the cover range never
converges for grid cells that span the seam (the bilinear corner blend is wrong
across the periodic identification), so it subdivides to `MAX_DIVISION_CELLS`.
Geometry is correct; density is a seam-aware-subdivision performance issue to
solve before production.

**Causal progression (confirmed exactly as the refined model predicted):**

```text
clean ──> parity failure (ContradictoryDualParity), no mesh
period ──> parity fixed, evaluator consumes cover UV -> 7.16e6-unit garbage
period+quot ──> evaluator stays in V-domain; garbage chords gone (max 238);
               but the two loops are not joined (Legacy join) and one
               projection escapes u -> band still wrong
period+quot+deckjoin+guard ──> annulus joined, u-guard fixes the spike
               -> correct band (area ~2% of trim-exact reference)
```

## 5. Trim-exact geometry certificate

Reference `nist1167_reference_exact.rs` derives `u_inner(v)` by brute-force
projecting the source inner trim #606 and integrates `u_inner(v) ≤ u ≤ 1` over
one V period on the surface:

```text
#1167 trim-exact band area = 8180   inner u∈[0.50,0.77]  (rect u[0.6,1] = 9625)
#1169 trim-exact band area = 8177   inner u∈[0.50,0.77]  (rect u[0.6,1] = 9680)
```

Recovered mesh vs reference:

```text
                area       ratio   max edge   direct vertex residual
#1167           8354       1.021   24.2       8.6e-3
#1169           8322       1.018   23.9       1.19e-2
```

Boundary follows #606/#607 (boundary vertices are the projected source-curve
points); triangle interiors and edge midpoints are on the surface to the direct
evaluator residual (the larger `search_nearest_parameter`-based numbers are a
separate B-spline inverse-search artifact, not a geometry defect). Material
side agrees (the mesh is the annulus between the two loops). No 100+/million-unit
false chords. Single annular region.

This is supportive evidence, not official NIST ground truth.

## 6. Production implementation (architecture, NOT landed)

Required components, in pipeline order:

1. **Source closure provenance.** The STEP `v_closed = .T.` fact is dropped in
   `TryFrom<&BSplineSurfaceWithKnots> for BSplineSurface` (truck-stepio
   `in/mod.rs:2535`). Preserve it so the composition layer can certify a V
   lattice generator of period `P = 1.0` (gated on evaluator seam compatibility:
   position + derivative, as measured in the handoff §C). Do NOT infer closure
   from knot/control-net patterns.
2. **`CertifiedLattice` V period.** `look::step::lattice_of` gains a
   source-closed-B-spline arm. A new `PeriodWitness` is required
   (e.g. `SourceDeclaredClosedSplineAxis`); the theorem is "the STEP source
   declares V closed and the converted evaluator satisfies S(u,0)=S(u,1) and
   Sv(u,0)=Sv(u,1); therefore P = v1−v0 is a V-lattice generator for this
   surface." Do not use `ExactRevolutionAngle`.
3. **Deck-consistent two-loop join as the default for genuine deck pairs.**
   `PolyBoundary::new` (truck-meshalgo `triangulation.rs:6528`) currently passes
   `TwoLoopJoinPolicy::Legacy`, which joins only area-degenerate pairs; the
   `DeckConsistent` policy (line 6691) already implements the correct
   non-degenerate deck-pair join (solves the deck equation, translates the
   loops, builds seam bridges) but is reachable only after a legacy
   `ContradictoryDualParity` failure. Genuine full-period deck walks must get
   the deck join on the first pass. **INV-W2-1 must be re-swept**: making this
   the default is a behavior change for every two-bound periodic face.
4. **Quotient adapter.** A surface decorator (analogous to `PolicySurface`,
   `look/step/policy_geometry.rs`) or a meshalgo evaluation wrapper that maps
   cover → native domain for every evaluator call on a periodic axis, using the
   surface's own `try_range_tuple` as the fundamental domain.
5. **Domain-bounded projection.** The B-spline inverse (`search_parameter` /
   `search_nearest_parameter`, truck-geotrait `algo/surface.rs`) can converge
   to a spurious stationary point outside the native domain (measured u=3.63
   for a point whose true u is 0.765). A result outside the native domain on a
   non-periodic axis is never a valid representative; reject it so the chain
   falls through to the hintless/nearest/seed links (which stay in-domain).
6. **Seam-aware interior subdivision** (follow-up): `parameter_division` over
   the cover range explodes across the seam (dense-mesh artifact). The interior
   grid should be built in the fundamental domain with seam-adjacent cells
   handled explicitly.

Invariants to hold: boundary cover coordinates / winding / parity / deck class /
CDT topology operate on cover coordinates; every evaluator call goes through the
quotient; non-periodic axes never wrap; the quotient is never a topology
identification.

## 7. Tests

No production code landed, so no production tests were added (T1–T7 are
specified for the landing session). Diagnostic evidence for each:
T2 quotient (`v=-0.25/1.25/2.25 → correct representative`) — exercised by the
quotient wrapper's `wrap`; T3 seam continuity — measured S/Sv seam residuals;
T4 source-open B-spline unaffected — the wrapper forwards non-target surfaces
identically (clean-mode #1169 mesh is byte-identical to the production path);
T7 analytic surfaces — the wrapper never applies to non-target surfaces.

## 8. NIST

Unchanged (no production change landed): `33 models, 7902 declared, 7901
rendered, 1 lost` — sole loss `nist_13 #1167`
(`MeshedToNothing` / `ContradictoryDualParity`). A successful census after the
architecture lands must additionally check #1169's *geometry* (it already
rendered before; `rendered=1` was a false positive).

## 9. ABC

No ABC run: no production change exists to measure. Current-tree reference
`839179 declared / 811798 rendered / 27381 lost`. Before/after comparison must
be per-face (rendered→lost, lost→rendered, rendered→rendered, lost→lost) grouped
by surface kind / u_closed / v_closed / periodic-lattice state / terminal
reason, and the two source-closed B-spline faces checked for geometry, not just
status.

## 10. Verdict

```text
periodic-cover theorem proven; production representation blocked
```

Supporting statements:

- The cover/evaluator separation (quotient map) is proven necessary and correct:
  it eliminates the evaluator-domain garbage (7.16e6-unit chords) with zero
  change to cover topology, and the direct evaluator residual of the recovered
  band is 1e-2 units on a 300-unit model.
- The quotient alone is **not sufficient** for the full band. The period supply
  exposes two further meshalgo defects that must also be fixed: (a) the legacy
  two-loop join never joins two non-degenerate full-period deck loops (the
  `DeckConsistent` join exists but is gated behind a legacy
  `ContradictoryDualParity` failure); (b) the B-spline parameter inverse can
  escape to a spurious root on the non-periodic u axis (u=3.63), which corrupts
  the boundary and hence the grid.
- With (V period + deck join + quotient + projection guard) the correct band IS
  produced for both faces (area within ~2% of the trim-exact reference, no
  false chords, vertices on-surface). The causal chain is therefore fully
  resolved diagnostically.
- Production is blocked by scope and risk, not uncertainty: the fix spans
  truck-stepio (closure provenance), truck-meshalgo (deck-join default, quotient
  adapter, projection guard, seam-aware subdivision), truck-geometry (inverse
  domain guard), and look (wiring), requires a truck re-pin, and flips a default
  (`DeckConsistent` join) whose INV-W2-1 sweep is a prerequisite.
- Stop condition 2 fires ("topology requires fundamentally different
  periodic-band construction"): the deck-consistent join must become the default
  handling for genuine deck pairs, which is a behavior change to be swept, not a
  landing.

Do not land the previous B-spline closure heuristic. Do not special-case
`#1167`. Do not weaken `flood_parity`. Do not infer source closure from
knot/control-net patterns. Do not globally wrap UV coordinates into a base
rectangle.

---

## Probes added this session (all untracked, do not commit)

- `nist1167_quotient_counterfactual.rs` — the decisive quotient/deckjoin/guard
  counterfactual (clean / period / period+quot / period+quot+deckjoin+guard).
- `nist1167_reference_exact.rs` — trim-exact band area reference (u_inner(v)
  derived from source trim #606).
- Existing probes preserved untouched.

## Re-run

```console
cd C:\Users\stefa\look
cargo build --release --example nist1167_quotient_counterfactual
target\release\examples\nist1167_quotient_counterfactual.exe `
  "C:\Users\stefa\Downloads\NIST-PMI-STEP-Files\NIST-PMI-STEP-Files\AP203 with PMI\nist_ctc_02_asme1_ap203.stp"
# the deck-join mode requires the diagnostic fork gate TRUCK_DIAG_DECK_JOIN=1
# with a local truck checkout at b4cebf05 (temporary Cargo.toml path patch)
```

The local truck diagnostic checkout lives outside the workspace
(`%LOCALAPPDATA%\Temp\opencode\truck-diag`) with the single diagnostic change
`PolyBoundary::new` → eager `TwoLoopJoinPolicy::DeckConsistent` under
`TRUCK_DIAG_DECK_JOIN`. The look `Cargo.toml`/`Cargo.lock` are restored to the
pinned fork (verified: `cargo check --locked --all-targets` and
`cargo test --locked --lib` green after a full `cargo clean`).
