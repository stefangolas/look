# Session handoff — 2026-07-29

State at the end of a session that moved from building the certified-ingestion
architecture to **closing visible defects**. Read this for what is true *now*;
read `PLAN.md` for the full record and `MATHEMATICAL_FOUNDATION.md` for the
contracts. This file is current-state only and will go stale — trust the
measurements in `PLAN.md` over anything here that disagrees.

---

## Where everything is

| | |
|---|---|
| `look` | `main` at `7c139b5`, pushed |
| `stefangolas/truck` | `master` at `7199cc90`, pushed |
| local override | **off** — `.cargo/config.toml` `paths` is commented out |
| build | release binaries present in `target/x86_64-pc-windows-gnullvm/` |
| tests | `truck-stepio` 36 pass, `look` 54 pass |

Both repos also carry the branch `checkpoint/certified-ingestion-2026-07-29`,
now identical to their default branch.

New companion repo: **`stefangolas/look-collapsed-boundary`** — an isolated
one-face reproducer plus `FORMALISM.md`, which labels every claim demonstrated
/ asserted / undemonstrated. Start there for the cone work.

## Baseline — reproduce this before trusting any change

```console
look inspect <abc>/00009190/00009190_..._step_000.step
#  warning: 396 of 24202 STEP faces produced no geometry
#           (3 failed to convert, 227 had no surface, 166 meshed to nothing)
#  triangles: 216335

cargo run --release --example find_blobs -- <same file>
#  10 of 577 shells mesh beyond 1.5x their own extent
#  160144 43.4 | 160784 42.1 | 161274 30.3 | 160374 20.8 | 160039 18.3
```

NIST (33 files): 356 of 7902 lost, 7546 rendered.

---

## What landed this session

1. **Typed identities and arenas, retained provenance.** `SourceId`/`Index`,
   `get_or_try_insert`, `source_id` in every arena item, `get_checked` printing
   the `TOP-001` failure form. Surfaces joined edges and vertices in the arena.
   `ClosedWire` → `TopologicallyClosedWire`.
2. **`FaceProvenance` on `CompressedFace`** — `use_id` / `definition_id` /
   `surface_id` kept apart, because an `ORIENTED_FACE` use is not its
   `FACE_SURFACE` definition. Carried through tessellation and face splitting.
3. **Plane-angle units** (`GEO-001`). Degrees were read as radians; a 2° draft
   cone became a 114° one and rendered as fans bursting out of a box. **Fixed
   NIST `ftc_07`.**
4. **`VERTEX_LOOP` support.** A collapsed bound resolves and contributes no trim
   segment. **604 → 396 lost on ABC**; conversion failures 274 → 3.
5. **Two screens**: `examples/face_fingerprint.rs` (per-face differential vs. an
   equivalent encoding) and `examples/face_census.rs` (typed loss reasons taken
   from the real conversion path, so it cannot drift from the renderer).

---

## The one thing in flight

**`TRUCK_CONE_APEX_RANGE`** — committed, **default off**, in
`truck-stepio/src/in/mod.rs`.

`Line::parameter_range()` returns `[0,1]` unconditionally and `RevolutedCurve`
inherits it, so a cone declares the domain `[0,1] × [0,2π)` — one unit of axial
parameter starting at the STEP reference radius and running *outward*. The apex
is at `u* = −R/tan θ`, outside it. The base circle then lies on the domain edge,
is classified open because it crosses the seam, and is stitched to that edge —
enclosing **zero area**, so nothing meshes.

Setting the flag spans the generatrix apex → 2× reference radius instead:

| | off | on |
|---|---:|---:|
| ABC lost | 396 | **339** |
| NIST lost | 356 | **276** |
| **ABC blob shells** | 10 | **9** |

**+137 faces and blob shell `#161274` (ratio 30.3, 161 faces) disappears.**
Verified that the default path is byte-identical with the flag off.

**Why it is not on.** Two reasons, both real:

- The factor of 2 is arbitrary. A cone face reaching past twice its reference
  radius falls outside the domain again — unmeasured.
- On NIST it moves 52 faces *sideways*: `NoSurfaceProduced/cone` rises 216 → 268
  while `MeshedToNothing/cone` falls 132 → 0. Those faces trade an empty domain
  for a chart that now contains the rank-deficient apex. This is `U2` in
  `FORMALISM.md`, arriving as predicted.

**The principled version derives the range from the face's own boundary**
rather than a constant. That needs the bounds at surface-construction time,
which the `From<&ConicalSurface>` impl does not have — so it wants either a
conversion context or a post-pass that retunes each surface to its faces.

---

## The repair queue

**ABC `00009190`, 396 lost:**

| stage | reason | surface | count |
|---|---|---|---:|
| tessellate | `NoSurfaceProduced` | bspline | 112 |
| tessellate | `NoSurfaceProduced` | nurbs | 70 |
| tessellate | `MeshedToNothing` | cone | 64 |
| tessellate | `MeshedToNothing` | plane | 53 |
| tessellate | `NoSurfaceProduced` | cylinder | 44 |
| tessellate | `MeshedToNothing` | cylinder | 20 |

The 64 cones are the flag's target. B-spline/NURBS surface production is now the
largest untouched category and has never been investigated.

**NIST, 356 lost:** `NoSurfaceProduced/cone` 216, `MeshedToNothing/cone` 132.

---

## Open questions, in the order I would take them

1. **Principled cone range.** Highest yield with the cause already demonstrated:
   137 faces and a blob shell. Needs a design decision (context vs. post-pass).
2. **`U2` — the apex under an extended chart.** 52 NIST faces fail differently
   once the domain contains the singular point. Until this is understood the
   flag cannot be turned on.
3. **The 216 ordinary cone patches.** *Measured to be a separate defect* — they
   have ordinary three-edge bounds with no collapsed vertex, and they fail in
   radian files as well as converted degree files. Never traced.
4. **`ctc_05`'s residual funnel.** Improved under the angle fix (2230 → 2196
   triangles), still wrong. Suspected: angular `PARAMETER_VALUE` trims on
   circles, which are **not** unit-converted. 20 of 33 NIST files contain them.
   The design rule: **never assign a unit to `PARAMETER_VALUE` at parse time** —
   the dimension comes from the consuming entity and parameter slot.
5. **Unit resolution is file-global.** It should be per
   `GEOMETRIC_REPRESENTATION_CONTEXT`. Sufficient today only because every file
   met so far agrees across its contexts; it refuses rather than guesses when
   they disagree.
6. **Nine remaining blob shells**, led by `160144` (43.4) and `160784` (42.1, 20
   faces — the smallest). Use `face_fingerprint`; it has no shell-contribution
   ranking yet, which is the obvious next feature.
7. **Tessellation-stage loss reasons are coarse.** `NoSurfaceProduced` and
   `MeshedToNothing` should split into projection / domain / arrangement / CDT
   terminal reasons. Conversion-stage reasons are already typed.

---

## Traps — these cost real time this session

- **Disk.** It hit 100% and measurements slowed 3×, a background job returned
  empty, and `du` itself timed out. Freed to 9.7 GB by deleting
  `truck-fork/target`, `look/target/debug`, `look/target/competitors`. **Never
  delete `look/target/research`** — it holds the owner's benchmark models.
- **The local override is a loaded gun.** Iterating on the fork means
  uncommenting `paths` in `.cargo/config.toml`; forgetting to re-comment it
  means every subsequent number describes an unpushed tree. It is invisible in
  `Cargo.lock`. The loop is: uncomment → edit → build → measure → commit fork →
  push → bump `rev` in `Cargo.toml` → re-comment → rebuild → **re-verify**.
- **Build the right targets.** `--bins` alone misses examples that construct
  `CompressedFace`; several breakages only appeared under `--examples`.
- **`--output` in `look render` is cwd-relative and overrides `--output-dir`.**
  33 PNGs once landed in the repo root.
- **A detector can be wrong.** `face_fingerprint`'s first scoring summed extent,
  area and edge excess and ranked the box's own walls at the top: absolute size
  is not evidence. Score on *disagreement with an oracle* instead.
- **A tidy correlation can be an encoding artifact.** The apex and ordinary-cone
  populations are perfectly anti-correlated with a clean 2:1 ratio, which looked
  like one cause and is two. Measure before merging categories.
- **An honest refusal catches your own bugs.** The unit resolution refused every
  file it existed to fix, because a degree unit is defined as a multiple of a
  radian unit and the base was counted as a competing declaration. Guessing
  instead would have converted by the wrong factor, silently.

---

## Method that worked

Localize to one face against an oracle, find the first divergent checkpoint,
fix that, re-measure both corpora, and write down what was *not* demonstrated.
Two of this session's four fixes came from looking at pictures the tooling
said were fine — the NIST corpus had only ever been checked computationally,
and rendering all 33 found two silent blobs immediately.
