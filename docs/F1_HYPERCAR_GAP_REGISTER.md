# F1 + HYPERCAR GAP REGISTER — theory vs mechanical, with the missing machinery named

2026-09-12, owner session. Source of truth: the interim R4 census
(`scratch/fhc_census_r4_interim/summary.json`, release `truck123d` at HEAD
with FHC-EX-A + FHC-EX-B landed, FHC-TRIM in flight **not merged**), EX-A's
RESULT per-row notes, and RDEF-M2's non-firing. Rows counted: 24 of 27 F1;
13 hypercar + 3 F1 pending — presumptive classifications are marked as such.

**The classification axiom.** RDEF-M2 fired zero
NonTransversalContact/BudgetExhausted refusals across the corpus: the certified
solver, *when a pair is admitted to it*, is numerically sound. Therefore no
open blocker is a solver-numerics problem. The gaps divide into:

- **THEORY-ADJACENT** — a proof obligation over landed lemmas (design-class;
  the math is probably already landed and needs a soundness proof + solver
  integration, but that is unproven until demonstrated).
- **MECHANICAL** — the theory and machinery are landed; a named admission,
  query surface, or attribute is missing between the corpus and them.
  Packet-able with no new math.
- **DIAGNOSIS** — the blocker is not yet identified (a swallowed or unrun
  refusal); may become either class once surfaced.

---

## 1. THEORY-ADJACENT — rational-weight patch flux in the certified boolean solver

**Missing machinery, precisely:** the landed funnel
(`boolean_product_volume_certified` → `node_box_patches` → `placed_box_patches`,
admission via MONO-8, membership via MONO-5, fold via MONO-9) consumes
**unit-weight** patches — the contact-cover solver's `cell_flux_exact`
integrates a polynomial flux over cell faces whose patch restrictions are
valid for `w(u,v) = const`. Every **rotational** boundary — cylinder, cone,
sphere, revolve — is a *rational* patch with `w(u,v) ≠ const`, so any boolean
whose operand boundary is rotational refuses
`unsupported_envelope` at admission. The landed ADM lemma wave (L1 extraction,
L2 product, L3 normal-cone, L4 deflate-seam, **L5 certified reciprocal-power —
Theorem D**) is exactly the substrate for rational integrands; what is missing
is the demonstration that `cell_flux_exact` (or a sibling) integrates the
rational flux soundly over cylinder-pair cells with the L5 composite bounds —
a proof obligation plus solver wiring, not a new theory from scratch. EX-A's
own note records this boundary honestly ("not the swept-loft operand
extraction this packet covers").

**Rows blocked (confirmed):** `f1/airbox`, `f1/details`, `f1/drivetrain`,
`f1/nose` (20 canonical CYLINDER+CYLINDER fastener unions),
`f1/sidepod_left`, `f1/sidepod_right` (same fastener class per EX-A) — **6**.
**Presumptive:** `f1/suspension_front` (refused after 67 s — deep into the
row, likely a boolean tail), hypercar rows with boolean tails over
revolve-built bodies (`brakes`, `powertrain`) — pending census.

## 2. MECHANICAL — kernel answers for OCC probe idioms

**Missing machinery:** the corpus builders call `is_valid_shape` /
bounds probes on kernel-engine rows; the door refuses these typed ("an OCC
probe of a kernel-engine row is not a kernel-engine row"). The kernel facts
they need **exist** — validity is the certificate bracket itself, bounds are
the carrier-derived enclosure — so the gap is a door/binding mapping from the
probe idiom to a kernel facts row (or a documented pass-through that answers
the probe from already-certified facts). No new math.

**Rows blocked (confirmed):** `f1/cockpit`, `f1/engine_cover`,
`f1/monocoque` — **3**. Presumptive: any hypercar row whose lib probes a
kernel row (pending census).

## 3. MECHANICAL — drop-in data-row attribute surface

**Missing machinery:** the drop-in's `Face`/`Edge`/`Plane` data rows do not
answer the attributes the corpus's client-layer algebra reads
(`Face.faces`, `Plane.origin`, `Edge.edge`, and the corner `Face`
bounds-probe carrier). EX-A already moved the class forward (`.wrapped` now
refuses typed instead of returning `None`), so every site now refuses typed
at the door — but the attributes themselves need kernel-backed answers
(carrier-derived bbox for a Face; placement extraction for Plane; the
underlying kernel Edge for Edge.edge). All carrier machinery exists; this is
marshalling.

**Rows blocked (confirmed):** `f1/corner_fl`, `corner_fr`, `corner_rl`,
`corner_rr` — **4**. **Presumptive (hypercar, pending census):**
`hypercar/details`, `hypercar/lighting` (`Face.faces`),
`hypercar/suspension_front` (`Plane.origin`), `hypercar/suspension_rear`
(`Edge.edge`) — **4**.

## 4b. MECHANICAL+ — degenerate planar fan cap (named 2026-09-12 by the TRIM landing)

**Missing machinery, precisely:** `f1/suspension_front` and
`f1/suspension_rear` block at the `_rocker` lightening cut
(`suspension.py:503`): a coaxial spline-profile prism through-cut (`_plate`
body minus a thicker `_plate` tool). The trim-extrude envelope extension
(FHC-TRIM, landed) extracts it EXACTLY — the tensor-Bernstein 2-cycle
between the recorded loop and its normal offset — but the exact planar cap
is a fan whose normal cone **degenerates at the loop centroid**, so the
landed RDEF-M2 tangential-sandwich admission refuses typed
`SingularParametrization` (marshaled as `unsupported_envelope` /
`non_canonical_carrier`). No numeric shortcut taken.

**The fix is mechanical+ within landed theory:** a planar fan cap is FLAT —
its exact flux has a closed planar form (or the cap is re-parametrized away
from the degenerate centroid), after which the composition reaches the
certified solver like any other pair. **Rows: 2** (both suspension rows;
their census "timeout" was this refusal taking ~200 s). Both duplicate
TRIM runs converged on identical verdicts.

## 4. DIAGNOSIS — swallowed or unexamined refusals

**Missing machinery: unknown until the typed refusal is surfaced.**

- `f1/floor`, `f1/diffuser` — the corpus builder's `is_valid_shape` swallows
  the drop-in refusal and raises untyped `RuntimeError: floor loft failed:
  None`. The underlying carrier (a stacked-plate loft variant?) is
  unidentified. First step is door-side typed propagation (hygiene), then
  classify. **2 rows.**
- `hypercar/aero` — `kernel refusal: empty` (`case empty`): either a
  degenerate band composition to fix at the carrier or a named open carrier.
  **1 row, pending.**
- `hypercar/wheels`, `hypercar/hinge`, `hypercar/powertrain` — the
  CENSUS-NAMES carriers (RegularPolygon, Align, RectangleRounded, Ellipse,
  Cone) are landed but unexercised by the census until it reaches hypercar;
  their next blocker is unknown. **3+ rows, pending.**

## 5. PERFORMANCE — certified construction cost at scale (not a refusal)

- `f1/power_unit` exceeded a 120 s cap (beam_wing, a smaller multi-station
  spline loft, builds in 5.4 s). If profiling shows the per-patch interval
  certification dominates, the fix is enclosure-scheme engineering within
  landed theory (mechanical+); if the bracket structure itself needs a
  smarter decomposition, it graduates to theory-adjacent. **1 row blocked by
  cost, not admission.**
- Emit-side cost is already docketed (`BD-EMIT-MESH-CACHE`: memoized
  tessellation + shared GLB accessors) — blocks no row; pipeline-only.

---

## Tally (FINAL — all 40 rows counted; census run 2026-09-12)

**GREEN 9/40 — all F1:** beam_wing, drs_actuator, drs_flap, front_wing, halo,
rear_wing, steering_rack, track_rod_left, track_rod_right.
**Hypercar 0/13** — but wheels/hinge/powertrain progressed PAST their R3 name
gaps (CENSUS-NAMES landed) and now stop at new, named carriers.

| category | class | rows |
|---|---|---:|
| 1 rational-patch flux (`airbox`, `details`, `drivetrain`, `nose`, `sidepod`×2; `suspension_front` presumptive — refused after 67 s) | theory-adjacent | 6 (+1) |
| 2 probe query surface (`cockpit`, `engine_cover`, `monocoque`) | mechanical | 3 |
| 3 data-row attributes (F1 `corner`×4; hypercar `details`, `lighting` (`Face.faces`), `suspension_front` (`Plane.origin`), `suspension_rear` (`Edge.edge`)) | mechanical | 8 |
| 4 named-carrier admission (hypercar `brakes`, `hinge` — `revolve needs a closed profile`; `wheels` — `bd.Color` name gap; `body`, `chassis`, `glazing`, `interior` — mirrored band-loft envelope still refusing after EX-B) | mechanical | 7 |
| 5 diagnosis (`floor`, `diffuser` — builder-swallowed; `aero` — `case empty`; `powertrain` — early envelope past its landed name) | TBD | 4 |
| 6 certified-cost scaling (`power_unit`, `f1/suspension_rear` — both exceeded the census's 120 s cap, not refusals) | perf, pending profile | 2 |
| **green** | — | **9** |

**One-sentence reading:** of 31 non-green rows, 18 are mechanical (wiring the
corpus to machinery that already certifies), 4 are unclassified until their
swallowed refusals are surfaced, 2 are cost-not-admission, and at most 7
carry a genuine proof obligation — the rational-weight flux extension of
`cell_flux_exact` over landed ADM lemmas (L5 reciprocal-power is the likely
substrate; the soundness demonstration is the work).
