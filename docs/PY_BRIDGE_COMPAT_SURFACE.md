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
| S1 | `Algebra operators + - &` | 2780 binary `+` `-` `&` (upper bound) | `landed` for canonical×canonical; `deferred-bie` for sweep×canonical/sweep×sweep; corpus rows exercising the deferred forms are `staged-skip` | facade `boolean_op` (`Mode` union/subtract/intersect); BIE-006 unlocks swept pairs |
| S2 | `Plane/Location algebra (plane * shape, plane.offset(d), Pos, Rotation, Location, Axis)` | 183 | `recorded-client-layer` | frame/placement rows of the submitted session table; `.offset(` is 19 of the 183 and every hit is a `Plane.offset` frame move — no kernel solid-offset hides here |
| S3 | `Primitives (Box, Cylinder, Sphere, Torus, Compound)` | 126 | `landed` | facade `box`/`cylinder`/`sphere`/`torus`; `Compound`/grouping is assembly emission (PB-006) |
| S4 | `make_face / topology types (Edge, Face, Wire, Solid, Shape)` | 49 | `landed` | facade `make_face`; topology typing only on the compat side (no kernel geometry in the bridge) |
| S5 | `Spline(*pts, periodic=...) sections -> loft` | 28 | `landed` (authoring = PB-002 amended scope; loft = CC-port) | facade `spline` carrier authoring + `loft`/`loft_ribs` CC-port forwards |
| S6 | `loft / revolve / extrude / sweep / fillet / chamfer / mirror` | 26 | `landed` | facade `loft`/`revolve`/`extrude`/`sweep`/`fillet`/`chamfer`/`mirror` (constructive parts export STL, never STEP — TR-NRB-001) |
| S7 | `Selectors (.faces(), .edges(), .filter_by(), .take())` | 4 | `landed` | the four-expression selector rows of the facade table (PB-001 machinery, exposed as data rows); the corpus is algebra-mode, so 4 is the whole census |
| S8 | `Color/Shape.color` + occurrence naming | 6 `.color` assignment sites (corpus AST, 2026-09-06; not a counted §8 census family) + 28 occurrence labels pinned by `f1.step.js` | `recorded-client-layer` | GLB emission (PB-009): per-solid sRGB color → linear `baseColorFactor`; `#o1.N` node names reproduced from assembly insertion order |

## Row notes

**S1 — algebra.** 1107 of the corpus's algebra operators are shape-boolean in
the upstream audit. Canonical×canonical boolean composition lands on the
facade's `boolean_op` today; boolean algebra over lofted/swept/revolved
carriers is a spec §7 non-goal until the BIE program lands sweep-pair
certification (BIE-006). Every corpus row that boolean-composes swept carriers
carries the machine-checked skip reason `booleans-on-swept-carriers`
(`corpus/ttc/SKIPS.json`, `resolved_by: BIE-006`) — the Falcon-Heavy rows that
are boolean-free run as the canonical subset, and the F1 rows that shell+cut
lofted geometry are skipped with that reason.

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

**S5 — spline sections.** `Spline(*points, periodic=...)`-shaped section
authoring (PB-002's amended scope) feeding the loft/loft-rib CC-port forwards.
A spline carrier is a non-canonical carrier: parts built over one are
constructive for the STEP boundary and export STL only.

**S6 — constructive verbs.** The loft/sweep/revolve/extrude/fillet/chamfer/
mirror families. Constructive parts stay behind the TR-NRB-001 STEP boundary:
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
`deferred-bie`, `boundary-refusal`, `staged-skip` values). The check is
one-directional: the seven `SURFACE_ROWS` of `compat/surface.rs` are the
spec §8 counted families, and a doc row beyond them (row S8, PB-009) is not
rejected. Adding a corpus surface family that spec §8's table does not name
is a SPEC_GAP, not a doc edit: the doc and the corpus table move together,
and the test pins that.
