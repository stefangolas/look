# falcon_heavy models

> **Educational, non-functional public-source reconstruction. Not suitable for
> manufacture, propulsion, testing, or operational engineering.**

| Script | Artifact | Description |
|---|---|---|
| `falcon_heavy.py` | `STEP/falcon_heavy.step` | Full stack: three cores, 27 linked Merlin 1D engines, nosecones, grid fins, stowed legs, raceways, attach hardware, MVac second stage, 5.2 m fairing |
| `falcon_heavy_cutaway.py` | `STEP/falcon_heavy_cutaway.step` | Center core + S2 + fairing sectioned 270° (opening +Y): schematic LOX/RP-1 volumes, transfer tube, domes, COPV-like placeholders, octaweb frames, avionics/separation placeholders, payload adapter + payload |
| `falcon_heavy_exploded.py` | `STEP/falcon_heavy_exploded.step` | Boosters outboard, stage/fairing lifted, translucent guide rods |

Build: `python src/<script>` per row; unchanged models are no-ops. All three
build in parallel safely: `ls src/*.py | xargs -n1 -P3 python`.

`src/lib/` holds the shared libraries (plain modules, no `@step`):

- `lib/falcon_common.py` — vehicle library: cores, stages, fairing, cluster
  placement, cutaway sectioning. `build_vehicle(cutaway=False)` is the entry.
- `lib/merlin_common.py` — vendored Merlin 1D engine library, with two
  vehicle-driven adjustments (960 mm cluster-fit exit, `DETAIL_BOLTS` toggle).
  See the module header and `../PROVENANCE.md`.

No imported source files: every artifact is generated, so `STEP/` is
gitignored in full.
