# PROVENANCE — vendored text-to-cad model trees (corpus/ttc/trees)

**Repo:** `https://github.com/earthtojake/text-to-cad`

**Commit:** `c222e5da1ae4e6387fcb7ba09c8e475936ee3260`
(2026-09-04, HEAD of a shallow `--depth 1` clone fetched 2026-09-06; the
clone commit was the tree the harness was audited against.)

**License:** MIT — `LICENSE` (root): "Copyright (c) 2026 Thompson Labs LLC".
The full license text is preserved in every vendored source file's header
where the upstream file carries one; the upstream model READMEs and the
`DIMENSIONS.md`/`PROVENANCE.md`/`HIERARCHY.md`/`ENGINE_INSTANCES.md`/
`RESEARCH.md` notes of the Falcon-Heavy tree are vendored verbatim.

## What is vendored

The three harness corpus trees, verbatim from the upstream repo:

| vendored tree | upstream path | contents |
|---|---|---|
| `trees/f1/` | `models/f1/` | F1 concept-car model tree: `src/*.py` per-part model files, `src/lib/*.py` shared geometry library, `src/README.md`, `f1_stage.appearance.json` |
| `trees/falcon_heavy/` | `models/falcon_heavy/` | Falcon-Heavy educational model tree: `src/*.py` model files, `src/lib/*.py` shared geometry library, `README.md`, `PROVENANCE.md`, `RESEARCH.md`, `DIMENSIONS.md`, `HIERARCHY.md`, `ENGINE_INSTANCES.md` |
| `trees/hypercar/` | `models/hypercar/` | Hypercar mid-engine model tree (PB-012-HYPERCAR-VENDOR): `src/*.py` per-system model files, `src/lib/*.py` per-system geometry builders plus the shared master-surface/`palette`/`context` libraries, `src/README.md`, `STEP/hypercar.step.js`, `render/presentation_display.json`, `render/presentation_theme.json` |

Upstream headers and license notices are retained byte-for-byte in every file.
`trees/f1/STEP/f1.step.js` and `trees/hypercar/STEP/hypercar.step.js` (generated
preview artifacts) are vendored with their trees so the trees stay whole.

Hypercar tree note: every system in the tree is a `src/lib/<system>.py`
`build()` that returns the system's labelled group, wrapped by a sibling
`src/<system>.py` cadgen `@step` file (the wrapper's `@step`/kinematics surface
is the cadgen-store boundary the harness does not reimplement). Body panels are
cut from one lofted master surface (`src/lib/surfaces.py`), so twelve systems
boolean-compose lofted/revolved/swept carriers and stage `skipped` (reason
`booleans-on-swept-carriers` in `SKIPS.json`); the boolean-free `wheels` system
(constructive tyre/rim/hub revolves, spoke and arch-flare lofts) stages
`canonical` with its OCC-door reference in `reference/wheels.json`. The
whole-car `src/hypercar.py` composition (thirteen sibling models + cadgen
`cylindrical`/`fastened`/`couple` door kinematics) is not enrolled.

## What this corpus is NOT

These are **test fixtures, not kernel code**. Nothing under `corpus/` is
imported by production code, the `vendor/truck/**` write-gate doctrine does not
apply to it, and its Python sources are never executed by the bridge — they are
executed by the harness door (`corpus/ttc/door.py`) under the genuine OCC
`build123d` engine to record/verify reference geometry facts and STL, with
`cadgen.build123d` answered as the transparent alias the upstream tree expects
(`cadgen`'s daemon/store is never reimplemented — see the boundary notes in
`docs/PY_BRIDGE_COMPAT_SURFACE.md`).

## Corpus manifest

`MANIFEST.json` enumerates the corpus rows the harness tracks (model tree →
geometry entry, staged `canonical` vs `skipped`); `SKIPS.json` carries the
machine-checked skip reasons; `reference/*.json` carry the recorded OCC-derived
reference geometry facts each canonical row must reproduce. The row manifest is
an output of program progress (spec §8 "Staged skips"): rows are enrolled as
the bridge's compat surface reaches them, and are extended by PB-007-CONFORMANCE
and later BIE/CG packets, never tuned to keep tests green.

F1 tree note: the `src/*.py` part files are `cadgen` `@step` wrappers over the
`lib/` build entries. The harness rows name the `lib` geometry entry each part
file dispatches to (the wrapper's cadgen `@step`/daemon surface is the
`cadgen`-store boundary the harness deliberately does not reimplement). The
`src/f1.py` whole-car composition additionally composes part occurrences
through `cadgen.assembly.AssemblyHelper`; it is not enrolled until the bridge's
own assembly emitter (PB-006) consumes corpus rows.
