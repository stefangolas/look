# text-to-cad drop-in: look as the render backend

Status: investigation complete, not implemented. This document records how look
would become an opt-in render backend for the snapshot/render path of
[`earthtojake/text-to-cad`](https://github.com/earthtojake/text-to-cad) and
exactly what look must build to make that a complete drop-in.

## Why this exists

text-to-cad is a library of agent skills for CAD/CAE/CAM. Its `cad` skill
generates STEP parts and assemblies from natural language (via a `build123d`
Python generator over the OCCT kernel), then **must** render a visual snapshot
of every generated artifact so the agent can "see" its own output. That
snapshot is the inner loop of the whole library: generate → validate →
snapshot → review → repair.

The snapshot renderer is `cadgen.snapshot_cli`, which runs a **headless
browser (playwright) over a WebGL page** for every snapshot. look is a native
STEP/STL/GLB → PNG executable with a CPU pipeline measured at ~7.4× faster than
OCCT on typical mechanical faces, deterministic framing, `--json` output, and
no browser dependency. It is a drop-in substitute for the *render* half of the
loop.

## The render surface in text-to-cad

Everything rendering-related lives in the `cadgen` package
(`cadgen==0.4.7`, PyPI), not in the repository itself. The per-skill launcher
is a thin shim:

```
skills/cad/scripts/snapshot/__main__.py   (~50 lines)
    -> cadgen.snapshot_cli.run_snapshot_cli
        -> cadgen.snapshot_core.BatchSnapshotRenderer
            -> headless browser + WebGL (runtime/render.html, snapshot-render.js)
                -> PNG / GIF outputs
```

Facts that define the surface:

- Input kinds: `step`, `stp`, `3mf`, `glb`, `stl`. `.step.py` sources are
  compiled to STEP by `scripts/gen` before snapshotting, so look never sees
  them.
- Render modes: `view`, `orbit`, `section`, `list`, `animate`.
- Outputs: PNG stills; GIF for `orbit` (camera turntable) and `animate`
  (parameter-sweep, geometry regenerated per frame).
- Themes and displays: theme sets background/grid/axis; display sets
  projection, mode, exploded, etc. Size profiles scale from 1200 px (simple)
  to 2000 px (complex assemblies).
- CLI contract: stdout carries the result, stderr carries progress/timing;
  `--json` is compact JSON on stdout.
- The `cad` skill already runs a warm daemon (`cadgen_daemon`) that imports
  cadgen/OCP once and services CLI invocations over a per-worktree AF_UNIX
  socket, opt-in via `CADGEN_WARM=1`, with a version token, idle timeout, and
  cold fallback.

## Where look fits

The seam is `render_resolved_job_packet` / `BatchSnapshotRenderer` — one choke
point shared by six skills. The design is an opt-in backend switch:

- `--renderer look` on the snapshot CLI, or `SNAPSHOT_RENDERER=look` in the
  environment.
- Default behaviour is byte-identical cadgen/playwright; look is never engaged
  unless opted in.
- A `LookSnapshotRenderer` implements the same "resolved job packet → output
  files + JSON" contract by shelling to `look`.

### Mode coverage

| Mode | look coverage | Notes |
|---|---|---|
| `view` | yes (today) | Map preset cameras to look's named views + auto-fit |
| `orbit` | after frames+GIF | Camera turntable over one resident model |
| `animate` | after frames+GIF | Renders N regenerated variants; the rebuild is cadgen's |
| `section` | after clip plane | Shader clip plane (visual cutaway, not B-REP sectioning) |
| `list` / contact-sheet | no | cadgen-specific layout; keep fallback |

## What look must build

### Feature gaps (the render work)

| Gap | What it requires | Effort |
|---|---|---|
| 3MF input | ZIP/XML mesh parse → existing triangle-soup path | 1–2 days |
| GIF / frame sequence | orbit camera path (resident) + animate variants → GIF encode | 2–4 days |
| Section mode | shader clip plane (plane uniform + clip in existing pipeline) | 1–2 days |
| Transparent/studio background | alpha background output for compositable themes | <1 day |
| Theme/display mapping | background colors, camera presets → named views; grid/axis not needed (snapshot strips them) | <1 day |

### Surface needs

- `--json` contract parity: cadgen's snapshot JSON (render results, geometry
  facts, warnings) must be reproducible from look's JSON via a thin
  translator. look already emits scene statistics + outputs; a few fields may
  need adding.
- Stable CLI contract: file + `--views` + `--resolution` + `--background` +
  `--output` + `--json`. Mostly exists today.
- Warm/resident mode: reuse the device across the agent loop. Mirror
  `cadgen_daemon` (AF_UNIX socket, version token, idle timeout, cold fallback)
  against look's `persist` / `render --session`. This is what turns per-
  iteration snapshots from ~500 ms into ~ms.

## Integration plumbing (their side)

1. `skills/cad/scripts/snapshot/__main__.py`: route to the look backend when
   `SNAPSHOT_RENDERER=look` (default path unchanged).
2. A `look-snapshot` translator mapping resolved jobs → look invocation + JSON
   remap, or a `LookSnapshotRenderer` inside cadgen.
3. SKILL.md documentation for look install (the release binaries +
   `install.sh`/`install.ps1`) and the opt-in.
4. Optional: `cadgen_daemon` spawns the look session server alongside the OCP
   daemon.

## Why this wins (Amdahl, honestly bounded)

Two regimes, two answers.

**Small model, big reasoning change** (LLM-dominant):

| stage | cost |
|---|---|
| LLM reason | 2–10 s |
| `gen` (OCCT rebuild) | 0.5–5 s |
| `inspect` | 0.1–1 s |
| `snapshot` (browser) | 1–3 s |
| LLM review | 1–5 s |

look's ceiling is the snapshot slice: ~10–20% per-iteration wall win, plus
compounding (more iterations per budget) and a qualitative shift (deterministic
~ms snapshots make visual-regression loops affordable).

**Big model, small incremental change** (tooling-dominant — the regime that
dominates real agentic CAD):

| stage | cost |
|---|---|
| LLM reason | 1–3 s (cheap edit) |
| `gen` rebuild | 10–60 s (full O(model) re-derivation) |
| `inspect` | 1–5 s |
| `snapshot` (browser at 2000 px) | 3–10 s |
| LLM review | 1–3 s |

Here tooling is 80–95% of the iteration. look removes the snapshot slice
(3–10 s → ~0.05–0.3 s resident), which is real but bounded; the elephant is
`gen`'s full OCCT rebuild, which no renderer can touch. Diff-based modeling
upstream is the only lever there.

### What look cannot do

- Speed up `gen`'s B-REP construction (booleans/fillets on OCCT). truck's
  modeling is research-grade, not OCCT-equivalent; look's truck strengths are
  import + tessellation, the opposite direction.
- Replace the interactive CAD Viewer (parameter sidecars, animation controls,
  exploded views, robot files, file browser). look's `--gui` HTML viewer is not
  a substitute and should not try to be.

### Where look's truck does help beyond rendering

- Downstream meshing (STL/3MF/GLB export, slicing meshes): look/truck
  tessellates at ~2–8 µs/tri vs OCP's ~18–20 µs/tri — a 3–7× win on gen's
  *outputs*.
- Fast STEP import as an inspect pre-filter: `look inspect` gives triangles,
  bounds, counts, hash without GPU, in milliseconds.
- Diff-based tessellation builds on truck's per-face structure, so look's
  snapshot/export slice can scale with the *change* instead of the model.

## Phased plan

1. **P1 — thin opt-in** (~half a week of look work). `view`-mode look backend
   for PNGs, `SNAPSHOT_RENDERER=look`, translator, SKILL.md docs. Covers the
   dominant single-view still with cadgen fallback for everything else.
2. **P2 — warm reuse** (medium). Look session server managed by the snapshot
   path, mirroring `cadgen_daemon`. The latency payoff.
3. **P3 — full mode coverage** (~1 week total look work). Add transparent
   background, frames+GIF, 3MF, shader section. Retire the fallback except
   `list`/contact-sheet.

## Open questions

- Whether `section` can be a shader clip plane for text-to-cad's purposes, or
  whether true B-REP sectioning is ever required (the latter is OCCT-level
  work and out of scope).
- Whether the `animate` parameter-sweep regeneration should be diffed (per-face
  tessellation cache keyed on stable entity IDs) — possible in theory, saves
  the meshing slice, not the OCCT rebuild.
- Whether look wants to own a modeling kernel at all. The gen-side "similar
  push" (certified feature-tree kernel over the analytic/prismatic subset,
  validated by look's own import pipeline) is a product-scope decision, not a
  feasibility one.

## Verification

- The drop-in must be measured, not assumed: snapshot one corpus of
  text-to-cad-generated parts with cadgen/playwright and with look, comparing
  per-iteration latency, output framing, and JSON field parity.
- Follow the repo's benchmark rules (BENCHMARKS.md): physical GPU, recorded
  adapter, release build, raw samples, power mode. Note that look's GPU init is
  state-dependent (~500 ms warm, ~1 s after an OpenGL app, ~1.5 s cold), which
  is precisely why the warm-resident path is the real integration target.
