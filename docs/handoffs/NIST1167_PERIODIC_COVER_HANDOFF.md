# NIST #1167/#1169 — PERIODIC-COVER PRODUCTION REALIZATION — HANDOFF

**Date:** 2026-08-11
**Status:** STAGES A–B FOUNDATION LANDED IN WORKTREE (NOT COMMITTED); STAGES C–F
**PENDING.** All work is in the local truck fork working tree and look working
tree, building green against the local fork. Nothing committed, nothing pushed,
no re-pin yet.

---

## 0. How to read this document

This is the working handoff for the *production* session that converts the
proven diagnostic contract into landed production code. Read first:

- `NIST1167_QUOTIENT_EVALUATION_FINDINGS.md` — the decisive diagnostic
  counterfactual and the exact causal chain (authoritative for the contract).
- `NIST1167_PERIODICITY_HANDOFF.md` — the earlier source-closure-loss
  investigation (superseded conclusion "production representation unresolved").

This document records **what was done this session**, **exactly where**,
**what is left**, and the **precise call sites** each remaining stage touches.

---

## 1. Starting state (this session)

```text
Look commit : 30f3d44  (Pin truck b4cebf05: R01 source-edge traversal)
Truck pin   : b4cebf05  (Cargo.toml + Cargo.lock)
local fork  : C:\Users\stefa\truck-fork  (checked out at b4cebf05, HEAD clean)
worktree    : look tracked tree clean at session start; only untracked handoffs/probes
NIST baseline: 33 models, 7902 declared, 7901 rendered, 1 lost
               sole loss nist_13 #1167 (ContradictoryDualParity), #1169 wrong-geometry render
ABC baseline : current-tree reference 839179 declared / 811798 rendered / 27381 lost
```

Baseline re-confirmed this session by running the quotient counterfactual:
`clean` reproduces #1167 ContradictoryDualParity and #1169's wrong 89-triangle
mesh; `period` reproduces the 5.17e9-area evaluator garbage; `period+quot`
reproduces the "band correct-ish but not joined" state.

---

## 2. Session result in one paragraph

The two-stage foundation of the production architecture is in the worktrees
and compiling:

- **Stage A (provenance)** — truck-stepio now exposes the source-declared
  `u_closed`/`v_closed` flags on every spline surface entity form, and look has
  a `spline_closure_map(table)` builder that reads them keyed by surface entity
  id.
- **Stage B (lattice witness)** — a new `PeriodWitness::
  SourceDeclaredClosedSplineAxis` exists in truck-meshalgo, and look's
  `lattice_of_with_closure` certifies a source-closed spline axis as
  `Exact { period: b - a, witness }` **gated on an evaluator seam check**
  (`spline_seam_compatible`: `S(·,a)==S(·,b)` and first derivative over the
  active `evaluation_range` interval, 8 samples, `1e-8` relative tolerance).

Nothing downstream consumes the closure yet: `wrap_shell` still drops it,
`PolyBoundary::new` still uses `TwoLoopJoinPolicy::Legacy`, the tessellator
still evaluates cover UV directly, and subdivision is not quotient-aware. The
next session's work is stages C–G on top of this foundation.

---

## 3. Current worktree state (exact)

### Look worktree (`C:\Users\stefa\look`)

```text
 M .cargo/config.toml       <- TEMP-ENABLED local truck-fork path override (see §4)
 M src/step/lattice.rs      <- SplineAxisClosure, spline_closure_map,
                               lattice_of_with_closure, spline_lattice,
                               spline_seam_compatible (+ tests to be added)
```

### Truck fork (`C:\Users\stefa\truck-fork`)

```text
 M truck-stepio/src/in/mod.rs                  <- u_closed()/v_closed() accessors
 M truck-meshalgo/src/tessellation/domain/lattice.rs  <- PeriodWitness variant
 M truck-meshalgo/src/tessellation/formal/ambient.rs  <- Step-1 probe arm for the
                                                         new witness
```

All three truck crates compile; look `cargo check --locked --all-targets` is
green against the local fork. `cargo test --locked` NOT yet run this session
(foundation only).

---

## 4. The local-fork iteration workflow (MANDATORY)

The `.cargo/config.toml` in look has a `paths` override enabled pointing at
`C:\Users\stefa\truck-fork`. This is how the truck changes are iterated on.

**Rules (from the file's own comment history, learned at cost):**

1. A measurement taken through the path override is a measurement of the local
   fork tree, **not** of anything a clean clone builds. Do not report numbers
   as production numbers until the fork is pushed and the pin bumped.
2. Before committing anything in the look repo:
   - `git -C C:\Users\stefa\truck-fork push` (commit + push the truck changes),
   - bump the `rev = "..."` in look's `Cargo.toml` to the new fork HEAD,
   - **re-comment** the `paths` block in `.cargo/config.toml`,
   - update `Cargo.lock`.
3. The fork's own `--all-targets` tests fail to build via the path override
   (proptest/truck_modeling dev-dependencies don't resolve) — that is a
   pre-existing fork-test limitation, NOT a regression. Look's workspace
   `--all-targets` is the gate that matters.

---

## 5. Stage A — closure provenance (FOUNDATION LANDED)

### 5.1 truck-stepio (`truck-stepio/src/in/mod.rs`)

`TryFrom<&BSplineSurfaceWithKnots> for BSplineSurface` at ~line 2535 dropped
`u_closed`/`v_closed`. Added `pub fn u_closed(&self) -> bool` /
`v_closed(&self) -> bool` to:

- `BSplineSurfaceWithKnots` (~line 2518)
- `UniformSurface` (~line 2616)
- `QuasiUniformSurface` (~line 2664)
- `BezierSurface` (~line 2713)
- `NonRationalBSplineSurface` (enum, forwards to the concrete form)
- `RationalBSplineSurface` (forwards to the wrapped non-rational)

Semantics: `matches!(self.u_closed, Logical::True)` — only an explicit `.T.`
is closure; `.U.` and `.F.` are both "not closed". The STEP declaration is the
authority; nothing downstream infers closure.

### 5.2 look — `spline_closure_map` (`look/src/step/lattice.rs:61`)

```rust
pub fn spline_closure_map(table: &truck_stepio::r#in::Table)
    -> HashMap<u64, SplineAxisClosure>
```

Iterates the table's spline surface holder maps (`b_spline_surface_with_knots`,
`uniform_surface`, `quasi_uniform_surface`, `bezier_surface`,
`rational_b_spline_surface`), resolves each holder to its owned entity via
`EntityTable::<..Holder>::get_owned(table, id)`, and reads the flags. Keyed by
**surface entity id** — the same id `FaceProvenance.surface_id` carries per
face.

### 5.3 Remaining for Stage A (next session)

Thread the map into the tessellation. The lattice callback is:
`|s: &policy_geometry::PolicySurface| lattice::lattice_of(s.inner())` at
`look/src/step.rs:215`. Plan:

1. Add `source_closure: Option<SplineAxisClosure>` field to `PolicySurface`
   (`look/src/step/policy_geometry.rs:198`).
2. Change `wrap_shell` (`policy_geometry.rs:445`) to accept the closure map
   (or add `wrap_shell_with_closure`) and attach per face using
   `face.provenance.surface_id` (available on `CompressedFace`).
3. Change the callback in `step.rs:215` to
   `lattice::lattice_of_with_closure(s.inner(), s.source_closure())`.
4. Mirror the closure threading in `examples/face_census.rs` (the census must
   exercise the same path as production).
5. Update `lib.rs` re-exports if a new public symbol is needed.

## 6. Stage B — spline period witness (FOUNDATION LANDED)

### 6.1 truck-meshalgo `domain/lattice.rs`

`PeriodWitness::SourceDeclaredClosedSplineAxis` added (module doc updated).
Deliberately distinct from `ExactRevolutionAngle`: a spline is not a rotation;
the period is the native span `b - a`.

### 6.2 truck-meshalgo `formal/ambient.rs` (Step-1 diagnostic probe)

`ambient_axis_evidence_from_legacy` (~line 1741) now matches the new witness
and maps it to `PeriodAxisEvidence::DeclaredButUncertified` with
`NoCertifyingRuleForSchema` — the analytic Step-1 model has no rule that can
re-derive a spline generator, and it must not fabricate one. This is a
diagnostic-probe-only path (`TRUCK_PROBE_AMBIENT`); it does not affect
production geometry.

### 6.3 look — `lattice_of_with_closure` / `spline_lattice` / `spline_seam_compatible`

```rust
pub fn lattice_of(surface: &Surface) -> CertifiedLattice            // = with_closure(.., None)
pub fn lattice_of_with_closure(surface: &Surface, closure: Option<SplineAxisClosure>) -> CertifiedLattice
fn spline_lattice(surface: &Surface, closure: Option<SplineAxisClosure>) -> CertifiedLattice
fn spline_seam_compatible(surface: &Surface, axis: Axis, (a, b): (f64, f64)) -> Option<f64>
```

Certification theorem (one axis `A`, active interval `[a,b]`):

> STEP declares `A` closed AND the converted evaluator satisfies the seam
> identification `S(·,a)==S(·,b)` (position + first derivative) over `[a,b]`
> → `P = b - a` is a valid deck generator on `A`.

- `evaluation_range()` (BoundedSurface) gives `[a,b]` — the basis-valid
  interior knot rectangle, which is narrower than `try_range_tuple` for
  unclamped end knots (this is why the quotient base must be the
  *evaluation range*, not the declared range).
- `SEAM_SAMPLES = 8`, `SEAM_RELATIVE_TOLERANCE = 1e-8` (measured seam
  residuals are ~7e-14 position / ~9e-13 derivative on a 300-unit model; 1e-8
  sits five orders above that and five below render tolerance).
- The seam check is a *rejection gate only*; it never establishes closure.

### 6.4 Remaining for Stage B (next session)

Tests. Required (work packet §30 / §15 A1–A5):

```text
A1 source v_closed=.T.  -> Exact{V, period=b-a, SourceDeclaredClosedSplineAxis}
A2 source v_closed=.F.  -> NonPeriodic
A3 coincident endpoints but source open  -> stays open
A4 repeated control net but source open  -> stays open
A5 unclamped knots but source open       -> stays open
```

Construct these with truck-geometry `BSplineSurface::new_unchecked` wrapped in
`Surface::BSplineSurface(...)`, or load the NIST model and read its actual
surfaces. Do NOT derive closure from geometry in the tests.

---

## 7. Stage C — quotient-aware evaluation (PENDING — next session's core)

### The invariant to hold

```text
cover UV (topology)  := what PolyBoundaryPiece::try_new produces via get_mindiff
                        (line 5136) + periodic_displacement — unwrapped deck coords
native UV (evaluator):= a + mod(x_cover - a, P) on certified periodic axes;
                        non-periodic axes pass through unchanged
topology must NEVER be rewritten by the quotient
```

### Where the break happens (measured, from findings doc §1)

First physical-evaluation call consuming out-of-domain cover UV:
`triangulation.rs:8760` `surface.subs(p.x, p.y)` (interior grid vertex),
immediately followed by `:8787` `surface.normal(p.x, p.y)`. Grid vertices were
placed at cover coords by `insert_surface` (`:8470`) because its bbox comes
from the boundary loops' cover UVs. Boundary vertices are exempt (carry
projected 3D point via `boundary_map`, populated at `insert_to` ~`:7172`).

### Latent sites the quotient adapter must cover (findings §1 table)

| site (triangulation.rs) | role |
|---|---|
| `try_new` ~4524 | raw-projection residual check (native output, no wrap needed) |
| `try_new` ~4848/4865 | degenerate-boundary dense reconstruction (`+ frac·P`) |
| `detect_degenerate_trim` 5130 | `uder`/`vder` metric scale on boundary samples |
| `PolyBoundary::new` 6834 | collapsed-pair apex branch (`subs`) |
| `insert_surface` 8540 | grid placement at cover bbox (insertion, N/A for eval) |
| **`triangulation_into_polymesh_outcome` 8760/8787** | **interior vertex subs/normal — FIRST BREAK** |
| `polyline_on_surface` 8982 | seam/closure polyline realization |

### Production placement options (decision needed by next session)

The handoff §17 suggests a surface decorator analogous to
`look/step/policy_geometry.rs::PolicySurface`, or a meshalgo evaluation
wrapper. Because `wrap_shell` already routes every face through `PolicySurface`
and the tessellator calls `surface.subs/uder/...` on the wrapped `S`, folding
the quotient into `PolicySurface`'s `ParametricSurface` /
`ParametricSurface3D` impls is the smallest safe placement — every evaluator
call in the tessellator then goes through the quotient automatically. Analytic
surfaces (cylinder/cone/sphere/torus) must remain byte-identical: their
evaluators are globally periodic and must NOT be routed through a mod map
that changes nothing but costs a wrap (findings §2). Gate the quotient on the
`SourceDeclaredClosedSplineAxis` witness presence.

Generalized quotient: `x_native = a + mod(x_cover - a, P)`; `[a, a+P]` is the
axis's `evaluation_range`. Seam behavior: `x=a` -> `a`, `x=a+P` -> `a`,
negative/multiple deck copies handled by mod; the map is not a topology merge.

---

## 8. Stage D — deck-consistent routing (PENDING)

### Current structure (triangulation.rs)

- `PolyBoundary::new` (`:6522`) calls
  `new_with_join(pieces, surface, tol, lattice, TwoLoopJoinPolicy::Legacy).0`.
- `TwoLoopJoinPolicy` enum at `:6032`; `TwoLoopJoinOutcome` at `:6048`.
- The `DeckConsistent` path is reachable only via the recovery arm at
  `:1788`–`:1860` (`deck_join_candidate` + `PolyBoundary::new_with_join` with
  `DeckConsistent`), and only after the legacy path failed with
  `ContradictoryDualParity`.
- `deck_pair` predicate inside `new_with_join` (`:6691`): both closed loops
  have nonzero lattice displacement, evaluated only under `DeckConsistent`.
- Recovery gates: `diagnosis::deck_join_recovery_enabled()` =
  `formal_recovery_enabled() && TRUCK_FORMAL_RECOVERY_DECK_JOIN` (both default
  on).

### Why direct routing is needed

With the V period certified (Stage B), #1167's legacy path fails with
`ContradictoryDualParity`, so the recovery *would* fire. But #1169's legacy
path **renders a wrong mesh** (89 triangles) — it never fails, so the
failure-gated recovery never runs. Eager first-pass `DeckConsistent` for the
proven class fixes #1169.

### Production task (do NOT make DeckConsistent globally default)

Find the structural class "two genuine full-period deck walks bounding one
band" and route only it to `DeckConsistent` on the first pass. Evidence the
handoff §10 lists: two closed loops, certified periodic axis, full-period ±1
winding/deck displacement, compatible two-bound material topology, valid deck
equation. Do not gate on `surface == BSpline` / `face == #1167`. The already
landed orientation and phase-correspondence invariants must continue to
compose. Run the INV-W2-1 population report (§19 of work packet) before/after.

Note the earlier `:1848` `TwoLoopJoinPolicy::DeckConsistent` in the recovery
arm — that path stays as-is; the new work is *first-pass* routing.

---

## 9. Stage E — bounded inverse-projection acceptance (PENDING)

### The escape

`search_parameter`/`search_nearest_parameter`
(`truck-geotrait/src/algo/surface.rs:278` and `:213`) can converge to a
spurious root outside the native domain on a non-periodic axis (measured
`u=3.63` for a point whose true u is `0.765`). The diagnostic fixed it by
rejecting out-of-native-domain results in the surface wrapper's search
methods (`in_native_domain`), letting the existing fallback chain
(hintless / structural seeds) find the in-domain root.

### Semantic rule (handoff §20)

> A final candidate parameter outside the native bounded domain of a
> non-periodic axis cannot be accepted as a semantic inverse representative.
> Periodic axes use deck equivalence instead. Do not clamp the candidate to a
> boundary and claim success.

### Ownership decision (handoff §11 — next session picks)

Options: truck-geotrait search algorithm, bounded surface adapter, or
policy/provenance layer. Prefer the narrowest generic semantic location. The
`PolicySurface` wrapper already overrides `SearchParameter`/`SearchNearestParameter`
forwarding, so folding the acceptance rule there is consistent with Stage C's
placement. Tests: true in-domain root exists, spurious exterior stationary root
exists, exterior result rejected, correct root recovered; endpoint/tolerance
cases.

---

## 10. Stage F — quotient-aware subdivision (PENDING)

### Failure

`parameter_division` (`truck-geotrait/src/algo/surface.rs:329`, recursion in
`sub_parameter_division` `:358`, cap `MAX_DIVISION_CELLS = 1 << 16` at `:356`)
never converges for grid cells that span the periodic seam: the bilinear corner
blend compares physically-adjacent corners as if far apart, so it subdivides to
the cap — measured ~237k triangles for a simple annulus.

### Invariant (handoff §21–22)

> All corners/interior samples participating in a physical approximation must
> be represented in one locally coherent physical chart.

Cover extent ≠ physical quotient extent (`v_cover ∈ [0.5, 1.5]` is one
physical period). Do not raise tolerance, cap triangles, or special-case
#1167. A useful invariant test: translate an entire loop's cover
representation by `+P`, `+2P`, `-P`; physical tessellation density must stay
essentially unchanged.

`insert_surface` (`triangulation.rs:8470`) calls
`surface.parameter_division(range, tol)` with the cover bbox range; the grid
vertices are then placed at cover coords and evaluated at `:8760`. If the
Stage C quotient lives in `PolicySurface::subs`, the subdivision error
estimator (which calls `surface.subs` on the wrapped surface) becomes
quotient-aware automatically — but the seam-crossing cell blend must also be
handled (the `insert_surface` range spans several periods). Decide whether the
fix lives in `PolicySurface::parameter_division` (fold range to native, divide,
map back) or in `sub_parameter_division`.

---

## 11. Stage G — integrated validation (PENDING)

### Focused witnesses (work packet §23)

Always keep #1167/#506 and #1169/#507 together. For each stage report: render
status, parity result, triangle count, area, bbox, max edge, vertex surface
residual, edge-midpoint residual, triangle-interior residual, boundary
residual, accepted UV domain, material topology. Never accept only
`rendered = true`.

### Positive/negative controls (work packet §24)

- Analytic periodic surfaces (cylinder, torus, sphere, cone): no regression.
- Source-open spline that numerically closes: must NOT gain lattice semantics.
- Source-closed spline without the exact #1167 trim topology (if available).

### NIST gate (work packet §26)

Fresh full build: `7902 declared, 7902 rendered, 0 lost`, plus
`0 previously-rendered → lost`, and **#1169 geometry repaired** (it already
rendered at baseline, so `rendered→rendered but geometry changed` must be
reported for the affected class).

### ABC gate (work packet §27–28)

Per-face A/B: rendered→lost, lost→rendered, rendered→rendered mesh changed,
lost→lost, grouped by surface kind / source u_closed / v_closed / periodic
lattice state / terminal reason. Source-closed spline faces must be inspected
for geometry, not just status. Historical reference `839179 / 811798 / 27381`.
Do not chase census drift; do not aim for a specific +N.

### Performance gate (work packet §29)

#1167/#1169 triangle counts + meshing runtime; deck-translation invariance.
The ~237k-triangle pathology must be gone.

---

## 12. Key numbers (trim-exact references, from findings doc §5)

```text
#1167 trim-exact band area = 8180   (recovered 8354, ratio 1.021, max edge 24)
#1169 trim-exact band area = 8177   (recovered 8322, ratio 1.018, max edge 24)
direct vertex residual ~8.6e-3 / ~1.19e-2
```

## 13. Test suite to add (work packet §30)

Provenance (A1–A5 above); quotient mapping (native interval not starting at
zero, negative cover coords, positive multiple periods, seam endpoint,
non-periodic pass-through); evaluation separation (topology UV unwrapped while
physical eval uses quotient); deck joining (certified full-period deck pair
reaches DeckConsistent primary join; multi-source seams unchanged);
projection (exterior root rejected, legal fallback recovered); subdivision
(deck translation does not change physical subdivision complexity);
structural reproducer (#1167-like band); silent-corruption reproducer
(#1169-like face cannot produce old spanning-chord mesh).

---

## 14. Commit strategy (when landing)

Prefer logically separable commits (work packet §34), each leaving tests
coherent:

```text
1. truck-stepio: preserve STEP spline-axis closure provenance
2. truck-meshalgo: SourceDeclaredClosedSplineAxis witness (+ ambient probe arm)
3. look: expose source-certified spline periodic lattice semantics (Stage A+B)
4. look/truck: separate periodic-cover coordinates from bounded evaluator
   coordinates (quotient adapter, Stage C)
5. truck-meshalgo: route certified full-period deck pairs through
   deck-consistent joining (Stage D)
6. bounded inverse-projection acceptance (Stage E)
7. quotient-aware subdivision (Stage F)
```

Do not produce one opaque `fix #1167` commit.

---

## 15. Stop conditions (work packet §33)

Stop without landing if: source semantics require heuristic reconstruction;
spline period witness cannot be justified representation-theoretically;
quotient evaluation changes topology; genuine deck pairs cannot be
structurally distinguished from source-established seams; bounded projection
rejects legitimate solutions; seam-aware subdivision still scales with deck
copies; an existing analytic periodic population regresses; #1169 remains
geometrically wrong; ABC/NIST changes cannot be causally attributed.

---

## 16. Do NOT reopen / non-goals (work packet §31–32)

- Do not modify R01 source-edge semantics, Track-B DeckConsistent orientation
  invariant, single-source phase correspondence, or `flood_parity` semantics.
- Do not mix with ctc_01 #617/#619/#621 or ftc_08 #6049 (next accuracy
  frontier).
- Do not special-case face IDs; do not infer STEP closure numerically; do not
  globally wrap stored UV coordinates; do not make all two-loop joins
  DeckConsistent without a structural classifier; do not use
  `ExactRevolutionAngle` as a fake spline witness; do not clamp Newton blindly;
  do not loosen parity; do not cap subdivision; do not chase unrelated ABC
  census drift; do not optimize exact triangle counts before correctness.

---

## 17. Re-run (baseline diagnostics)

```console
cd C:\Users\stefa\look
cargo build --release --example nist1167_quotient_counterfactual
target\release\examples\nist1167_quotient_counterfactual.exe `
  "C:\Users\stefa\Downloads\NIST-PMI-STEP-Files\NIST-PMI-STEP-Files\AP203 with PMI\nist_ctc_02_asme1_ap203.stp"
```

The `period+quot+deckjoin+guard` mode requires the diagnostic fork gate
`TRUCK_DIAG_DECK_JOIN=1` with the local truck checkout at
`%LOCALAPPDATA%\Temp\opencode\truck-diag` (diagnostic-only, outside workspace,
restored).

NIST model path: `C:\Users\stefa\Downloads\NIST-PMI-STEP-Files\NIST-PMI-STEP-Files\AP203 with PMI\nist_ctc_02_asme1_ap203.stp`
