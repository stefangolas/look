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

## Tally (interim — 24 of 40 rows counted, 16 pending)

| category | class | rows confirmed | presumptive |
|---|---|---:|---:|
| 1 rational-patch flux | theory-adjacent | 6 | 3+ |
| 2 probe query surface | mechanical | 3 | ? |
| 3 data-row attributes | mechanical | 4 | 4 |
| 4 diagnosis | TBD | 2 | 4+ |
| 5 certified-cost scaling | mechanical+ pending profile | 1 | — |
| **green so far** | — | **7** | — |

**One-sentence reading:** of the 17 F1 rows not yet green with a known
blocker, at most one category (rational-patch flux, 6 rows) carries a genuine
proof obligation over landed lemmas — everything else is wiring the corpus to
machinery that already certifies, plus a short diagnosis list whose members
are unclassified only because their refusals are still swallowed or unrun.
