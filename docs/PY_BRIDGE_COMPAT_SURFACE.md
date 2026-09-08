# PY Bridge — build123d-compat surface the corpus exercises (PB-008-TTC-HARNESS)

**Status:** LANDED by work packet PB-008-TTC-HARNESS.

This document is the **build123d-compat vocabulary table** (spec
`TRUCK123D_PY_BRIDGE_SPEC.md` §8's measured surface) that the text-to-cad
corpus exercises — **distinct from PB-000-CONTRACT's kernel-facing API table**
(`docs/PY_BRIDGE_CONTRACT.md`), which freezes the Rust entry vocabulary the
bridge dispatches onto. This table answers the question PB-008 exists for:
when a vendored corpus script calls a build123d name, which compat row answers
it, and what is that row's landed status at the current program stage? Because
`cadgen.build123d` is a transparent re-export of genuine build123d ("no wrapper
objects, same signatures, never improved"), the corpus exercises the **real
build123d API** and the compat surface must answer to it, name for name.

**Status vocabulary used below** (every row carries one):

| status | meaning |
|---|---|
| `landed` | answered today by a landed facade/CC-port row behind the bridge; the corpus row may run |
| `recorded-client-layer` | zero kernel content — client/frame arithmetic the bridge models as data rows (Plane/Location algebra, `Mode`) |
| `deferred-bie` | canonical-carrier forms land today; the sweep/swept-carrier forms wait on BIE (sweep-pair certification) |
| `boundary-refusal` | answered as a **typed refusal** (never silent): a name inside a documented kernel boundary refuses with the named case |
| `staged-skip` | corpus scripts exercising this row in a not-yet-landed form carry a machine-checked skip reason (`corpus/ttc/SKIPS.json`) |
| `lift-evidence-recorded` | a staged row whose green door run + reference-matched facts are recorded as lift evidence in its SKIPS note (PB-011B's canonical-cutter rows), still staged until the orchestrator's manifest movement |
| `census-recorded` | a staged row whose SKIPS note carries a typed census record (PB-011C's swept x swept rows: typed-refusal, with the row's OCC door evidence), still staged until the certified funnel admits the class |

## Corpus and census

Corpus: `github.com/earthtojake/text-to-cad` @ `c222e5da1ae4e6387fcb7ba09c8e475936ee3260`
(vendored under `corpus/ttc/trees/`; provenance in `corpus/ttc/PROVENANCE.md`).

Usage counts below were **re-derived by command** on 2026-09-06 from the
vendored trees (house rule: every tree claim is re-derived before it is
quoted):

```console
python corpus/ttc/census.py
```

The census is an AST count over the two vendored trees' `src/` files (53
files; comments, docstrings and strings excluded). `algebra_ops` is an
**upper bound** — it counts every binary `+` `-` `&` operator including scalar
arithmetic; shape algebra is the dominant verb of the corpus and is why the
corpus rows are staged (`corpus/ttc/SKIPS.json`, reason
`booleans-on-swept-carriers` → BIE-006). The other families are name/call
counts of the exact compat vocabulary. Row S8 below is **not** a counted §8
census family: `shape.color` and the `#o1.N` occurrence labels are client-layer
payload (data rows + a sidecar table), so its usage figures are the corpus AST
count of `.color` assignment sites and the label count pinned by the vendored
`f1.step.js`, re-derived separately on 2026-09-06.

## The measured surface — spec §8's 7 counted rows, plus S8 (recorded client layer)

| # | surface id | usage (census 2026-09-06) | compat status | answered-by (bridge rows) |
|---|---|---|---|---|
| S1 | `Algebra operators + - &` | 2780 binary `+` `-` `&` (upper bound) | `landed` for canonical×canonical AND for the routed swept-carrier forms (spline/swept/revolved carrier classes dispatch through the certified entry, PB-011); `boundary-refusal` for a swept pair coupling a funnel-refused carrier class (torus); `lift-evidence-recorded` for the 12 canonical-cutter F1 rows PB-011B proved green through the door with facts matching their recorded references; `census-recorded` for the 8 swept x swept F1 rows PB-011C ran census-first (every row filed a typed-refusal census record with its door evidence — the funnel does not admit a boolean between two spline-loft solids end to end, so no C lift); the remaining `staged-skip` corpus rows carry the machine-checked reason | facade `boolean_op` (`Mode` union/subtract/intersect) on canonical carriers; swept-carrier `Mode` rows route through the certified-entry dispatch (`dispatch_swept_carrier_boolean`, PB-011); PB-011B recorded the 2-D path lift evidence on the canonical-cutter F1 rows (door run green + facts vs the recorded reference, `PB-011B LIFT EVIDENCE` marker in the row notes); PB-011C recorded the 4-D path census on the swept x swept rows (typed-refusal, `PB-011C CENSUS` marker in the row notes); BIE-006 stays the resolution for the still-deferred forms |
| S2 | `Plane/Location algebra (plane * shape, plane.offset(d), Pos, Rotation, Location, Axis)` | 183 | `recorded-client-layer` | frame/placement rows of the submitted session table; `.offset(` is 19 of the 183 and every hit is a `Plane.offset` frame move — no kernel solid-offset hides here |
| S3 | `Primitives (Box, Cylinder, Sphere, Torus, Compound)` | 126 | `landed` | facade `box`/`cylinder`/`sphere`/`torus`; `Compound`/grouping is assembly emission (PB-006) |
| S4 | `make_face / topology types (Edge, Face, Wire, Solid, Shape)` | 49 | `landed` | facade `make_face`; topology typing only on the compat side (no kernel geometry in the bridge) |
| S5 | `Spline(*pts, periodic=...) sections -> loft` (+ closed-circle `Circle(radius)` profile authoring, PB-014) | 28 (Spline-call count) | `landed` (authoring = PB-002 amended scope + PB-014 circle carrier; loft = CC-port) | facade `spline`/`circle` carrier authoring + `loft`/`loft_ribs` CC-port forwards |
| S6 | `loft / revolve / extrude / sweep / fillet / chamfer / mirror` | 26 | `landed` | facade `loft`/`revolve`/`extrude`/`sweep`/`fillet`/`chamfer`/`mirror` (constructive parts export STL, never STEP — TR-NRB-001) |
| S7 | `Selectors (.faces(), .edges(), .filter_by(), .take())` | 4 | `landed` | the four-expression selector rows of the facade table (PB-001 machinery, exposed as data rows); the corpus is algebra-mode, so 4 is the whole census |
| S8 | `Color/Shape.color` + occurrence naming | 6 `.color` assignment sites (corpus AST, 2026-09-06; not a counted §8 census family) + 28 occurrence labels pinned by `f1.step.js` | `recorded-client-layer` | GLB emission (PB-009): per-solid sRGB color → linear `baseColorFactor`; `#o1.N` node names reproduced from assembly insertion order |

## Row notes

**S1 — algebra.** 1107 of the corpus's algebra operators are shape-boolean in
the upstream audit. Canonical×canonical boolean composition lands on the
facade's `boolean_op`; swept-carrier boolean algebra (the corpus's dominant
verb — 2,780 binary operators upper bound) lands for the routed forms on the
facade's certified-entry dispatch (PB-011): a `Mode` row over a
spline/swept/revolved carrier dispatches the carrier pair through
`dispatch_swept_carrier_boolean`, the facade mirror of the landed CL-006
solver-entry funnel, and an accepted pair is recorded on the facade report as
a routed swept-carrier boolean row. A pair coupling a swept carrier with a
funnel-refused carrier class (torus — excluded from the implicit-reduction
stage) is answered as the typed, localized refusal, fail-closed. The 2-D path
wave (PB-011B) lifted the 12 canonical-cutter F1 rows whose first refusing op
is a canonical-tool pair (floor/diffuser/suspension x2/steering_rack/track
rods x2/corners x4/drs_actuator): each row's OCC-baseline harness door run is
green and its facts match the recorded `corpus/ttc/reference/*.json` within
tolerance, and each row now carries the `PB-011B LIFT EVIDENCE` marker in its
SKIPS note (`lift-evidence-recorded`). The four corner rows were checked for
the torus caveat: their revolve carriers are spline/line-polyline profiles,
none is a revolved-circle (torus) carrier, so every cut(revolved, canonical)
pair certifies through the certified-entry dispatch and no typed torus
refusal was recorded. The rows still carry the machine-checked skip reason
`booleans-on-swept-carriers` (`corpus/ttc/SKIPS.json`, `resolved_by:
BIE-006`) — the physical manifest movement to the runnable canonical set is
the orchestrator corpus-output step, exactly as PB-011's cutaway lift. The F1
rows whose first refusing op is swept×swept (airbox, beam_wing, details,
drivetrain, drs_flap, engine_cover, power_unit, rear_wing) were PB-011C's
4-D path wave. PB-011C ran all eight census-first: none certifies on this
dispatch — every row's swept-carrier booleans are swept×swept (lofted/swept
base × lofted/swept tool) carrier pairs the certified funnel does not admit
end to end today (the fuse(swept,swept)/cut(swept,swept) pair cells answer
the typed constructive-carrier `NonCanonicalCarrier` refusal at the boolean
boundary; no landed boolean runs between two spline-loft solids; the
restricted 4-D arm admits circular-section sweeps only, so no solver budget
is spent — a typed refusal, not a stagnation). Each row note records a
`PB-011C CENSUS` typed-refusal record with its OCC-baseline door evidence
(green; facts reproducing the recorded reference within tolerance; facts
mismatch: none), the rows carry no lift marker, and they stay staged under
the machine-checked skip reason (`corpus/ttc/SKIPS.json`, `resolved_by:
BIE-006`). The Falcon-Heavy rows that are boolean-free run as the canonical
subset, and the remaining F1 rows that shell+cut lofted geometry are skipped
with that reason.

**S2 — Plane/Location algebra.** Client-layer arithmetic: `Plane * shape`,
`Pos`/`Rotation`/`Location`/`Axis` frames, `.moved(...)`, `plane.offset(...)`.
Zero kernel content (the 19 `.offset(` hits are all `Plane.offset` frame
moves, never a solid-offset). The bridge records these as data rows; nothing
geometric computes in Python (spec §5).

**S3 — Primitives.** The landed facade solid primitives. The corpus composes
them through `Compound`/instance placement rather than booleans wherever a row
is canonical (Falcon-Heavy engine/vehicle assemblies).

**S4 — topology vocabulary.** `make_face` over closed profiles and the
topology type names. The compat side types against them; the kernel content is
the landed facade entries.

**S5 — spline and circle sections.** `Spline(*points, periodic=...)`-shaped
section authoring (PB-002's amended scope) and closed-circle `Circle(radius)`
profile authoring (PB-014, audit G3) feed the loft/loft-rib CC-port forwards
and the landed revolve. A spline carrier is a non-canonical carrier: parts
built over one are constructive for the STEP boundary and export STL only. A
circle profile is a canonical closed profile (the exact conic of the S3
cylinder/torus family): a part built over circle sections is constructive only
when the verb is (`loft`/`sweep`, or a spline-carrier revolve), never by the
circle carrier alone.

**S6 — constructive verbs.** The loft/sweep/revolve/extrude/fillet/chamfer/
mirror families. The partial-arc revolve form (facade `revolve_arc`: arc_deg +
start_deg, the audit G5 cutaway construction — a trimmed shell over
[start, start+arc] closed by two planar caps) rides the S6 revolve family
(PB-014). Constructive parts stay behind the TR-NRB-001 STEP boundary:
STL, never STEP (a corpus row whose export target is STEP-only behind that
boundary would carry the typed skip reason `step-out`).

**S7 — selectors.** The four-expression selector vocabulary
(`faces`/`edges`/`filter_by`/`take`). The corpus is algebra-mode, so the
selectors census is 4 total; PB-001's fluent selectors are for OUR Python
layer, not for corpus reach.

**S8 — color records and occurrence naming (PB-009).** Every corpus solid
carries `shape.color` (a client-layer record, S2-class: zero kernel content)
and every top-level part of the F1 assembly is addressed by a frozen
occurrence label (`#o1.N` — the vendored `f1.step.js` OCCURRENCES block pins
28 labels under the root assembly, ordered by `src/f1.py`'s `assemble()`).
OCCT's `export_step` carried both into the corpus's STEP render payload;
swept/lofted F1 carriers cannot export STEP (TR-NRB-001), so this row is
answered by the bridge's **GLB emission** (PB-009): each emitted GLB node
carries its occurrence label as the node name, its local transform (matrix),
and its solid's sRGB color as a linear `baseColorFactor`. The row is
`recorded-client-layer`, not `landed`: `shape.color` is data, never kernel
geometry, and the occurrence labels are a sidecar table this side reproduces
from assembly insertion order — not a Python surface the facade answers.

## Envelope and boundary notes (normative for the harness)

- **STL, never STEP, for swept parts** (TR-NRB-001). The harness door
  (`corpus/ttc/door.py`) exports STL for every canonical row.
- **cadgen's daemon/store is never reimplemented.** The runner is a plain
  process-per-script door; `cadgen.build123d` is answered as the transparent
  alias the corpus expects and `compound_from_instances` as the equivalent
  plain build123d placement composition.
- **Typed refusals, never silent fallbacks** (spec §5): a compat name the
  current stage cannot answer refuses typed with the booked case; the corpus
  side of that bookkeeping is `corpus/ttc/SKIPS.json`.
- **Timing** in three regimes (OCC baseline, drop-in, native facade) is
  recorded per script as local evidence only and never published from this
  machine (BENCHMARKS doctrine; fresh-process vs resident kept separate).

## Machine-check contract

`truck123d/tests/ttc_harness.rs` (`compat_surface_table_is_complete`) scans
this file for each of the seven surface ids (`S1`..`S7`) above and for a
**status token** on each row (one of the `landed`, `recorded-client-layer`,
`deferred-bie`, `boundary-refusal`, `staged-skip`, `lift-evidence-recorded`,
`census-recorded` values). The check is
one-directional: the seven `SURFACE_ROWS` of `compat/surface.rs` are the
spec §8 counted families, and a doc row beyond them (row S8, PB-009) is not
rejected. Adding a corpus surface family that spec §8's table does not name
is a SPEC_GAP, not a doc edit: the doc and the corpus table move together,
and the test pins that.
