# Plan

> ## Read [`MATHEMATICAL_FOUNDATION.md`](MATHEMATICAL_FOUNDATION.md) first.
>
> It is the authority for the STEP ingestion layer: the exact mathematical
> semantics, the numbered contract registry (`TOP-`, `GEO-`, `QUO-`, `DOM-`,
> `ARR-`, `CDT-`, `MSH-`, `SHL-`), the Rust enforcement architecture, and the
> pipeline typestate. **This file is subordinate to it.** Where the two
> disagree, that document wins and this one is stale.
>
> The division of labour: `MATHEMATICAL_FOUNDATION.md` says what must be true
> and which type is supposed to make it true. `PLAN.md` records what has
> actually been built, what it measured, and what is still wrong. The roadmap
> below is the same PR sequence as that document's Part VI — read the contracts
> there, the measurements here.
>
> **Every change to the ingestion layer must name the contract IDs it
> discharges**, and every falsified hypothesis must be recorded here per §56 of
> that document. Nothing landed so far cites its contracts; that is a debt, not
> a precedent.
>
> ### Next work is §33a of that document, in order
>
> **Items 1–3 landed 2026-07-29; item 4 landed for surfaces only.** See
> "PR 2b — Retained identity" below for what was built and what it measured.
>
> **Next is item 5**: implement `RES-001`–`RES-006`. Then 6–12: downgrade capped
> subdivision to `ResourceCapped`, add cost fields to every contract, fix QUO-002
> and CDT-005 in code, give `Unknown` its renderer semantics, add the empirical
> acceptance axes, act on the owned-fork decision, and start citing contract IDs.
>
> Item 11 has grown a second reason to happen: it is now what unblocks TOP-001
> for faces, which item 4 could not reach.

## Design intent

`look` is a native command-line utility that turns GLB, STL, and STEP models
into PNG images, optimized for time to a usable image. It exists so a person, a
script, or an agent can inspect a 3D model without a CAD application or a
browser.

The working method is autonomous improvement against measurement: find where
the tool is slow or wrong on real inputs, fix the largest thing, prove the fix
with numbers, and keep the hot path small. A change is only finished when it is
measured, tested, and either published or explicitly recorded as unverified.

**Scope decision, 2026-07-29: the STEP path is becoming a certified geometry
kernel.** `AGENTS.md` previously drew the boundary at "a renderer, not an
embeddable CAD framework". That boundary has been deliberately moved. See
§"Why the architecture changed" and the roadmap below. The renderer product
goal is unchanged; the ingestion layer beneath it is being rebuilt.

## Where the effort goes

STEP. `look` reads it with its own ISO 10303-21 reader (`src/step/part21.rs`),
resolves the entity graph through a fork of `truck-stepio`, tessellates through
`truck-meshalgo`, and renders with `wgpu`.

Two forks are pinned by exact revision: `stefangolas/truck` and
`stefangolas/ruststep`.

> **Fork strategy, DECIDED 2026-07-29: owned architectural fork.**
> `stefangolas/truck` is permanent. The core model changes, guarantees are
> load-bearing across the whole pipeline, and upstream compatibility is not a
> design constraint. Divergence from `ricosjp/truck` is accepted and is never a
> reason to weaken a contract; rebase burden is a cost of the strategy, not an
> argument against it. Self-contained safety fixes may still be offered upstream
> as a courtesy — the resource bounds are the obvious candidate — but that is
> contribution, not obligation, and must not shape a design decision here.
> See `MATHEMATICAL_FOUNDATION.md` §31a.
>
> **Consequence:** `CompressedFace::boundaries` becomes
> `Vec<TopologicallyClosedWire>`, and every remaining `Index::position()`
> escape hatch — which exists only to satisfy a truck signature that is now ours
> to change — is a defect to close rather than a boundary to respect.
>
> **Fork state, still unresolved and blocking release.** `Cargo.toml` carries a
> temporary `[patch."https://github.com/stefangolas/truck"]` block pointing at
> `../truck-fork`; thirteen-plus commits sit in that local clone, unpushed;
> nothing is pinned. With the strategic question now settled this is purely
> mechanical: push the fork, pin the rev, delete the patch block.

---

## Current state of the corpus

All 20 ABC models load and render without error. **Rendering correctly is a
different question**, and the failure is silent — nothing errors, the picture is
just wrong.

Blob shells on `00009190` (577 shells, 24,202 faces), by fix:

| state | blob shells | note |
|---|---:|---|
| session start | 70 | renders as undifferentiated lens blobs |
| after containment fix | 12 | |
| after periodic-lift fix | 4 | renders as a recognizable submarine |
| after `eidx_map` fix | 3 | |
| after disabling `surface.invert()` | 2 (diagnostic only, not a fix) | |

Face loss on `00009190`: 394 → 449, with "no surface" flat at 276 and the rest
in "meshed to nothing".

Renders: `../look-trimming-residual/renders/` (iso, front at 1400×1400).

---

## Why the architecture changed

Four distinct defects surfaced in one session. Every one is the same failure
mode: **an invalid state was representable, and the next stage consumed it
happily**, producing a smooth plausible blob instead of an error.

| defect | mechanism |
|---|---|
| Signed-area domain semantics | Material side inferred from a chart-dependent predicate; `A(φ∘γ) = −A(γ)` while the face is unchanged, so the same solid meshed differently under a reflected chart |
| Periodic lift instability | `get_mindiff` breaks a tie arbitrarily at exactly half a period, so a boundary's winding class depended on tessellation tolerance |
| `eidx_map` reserve-before-convert | Index claimed before the conversion that decides whether the edge exists; one failure desynchronised map and vector, and faces silently received a neighbouring edge's curve |
| `surface.invert()` | Breaks curve-on-surface incidence rather than only reversing parameterization |

Four in one session is a design property, not bad luck. The response is to make
each pipeline stage a fallible constructor whose output type carries evidence,
so later stages cannot consume unproven state.

**Sequencing principle: each PR stands alone.** If priorities change, the work
done so far should have shipped value rather than left a half-rewrite.

---

## Roadmap

### PR 1 — Residual gate (VERIFIED, DEFAULT OFF)

A nearest point is not an incidence. `search_nearest_parameter` answers whether
or not the query lies on the surface, so a boundary belonging to another face
still yields a plausible parameter and a smooth uv path.

Implemented in `PolyBoundaryPiece::try_new`: `tol` threaded through both call
sites, boundary points rejected at `residual > tol * COMPATIBILITY_FACTOR`
(currently 5.0), diagnostic behind `TRUCK_PROBE_COMPAT`.

> **Status: swept 2026-07-29. The population is real; the gate does not fix the
> render.** `COMPATIBILITY_FACTOR` is overridable at runtime by
> `TRUCK_COMPAT_FACTOR` (read once through a `OnceLock`, since it sits in the
> per-boundary-point loop); `inf` disables the gate and is the baseline below.
> Measured on `00009190`, one fresh `LOOK_CACHE_DIR` per run — `look inspect`
> caches statistics keyed on the source file, so a shared cache reports one
> run's numbers five times.
>
> | factor | faces lost | no surface | meshed to nothing | triangles | fires |
> |---|---:|---:|---:|---:|---:|
> | off (`inf`) | 393 | 227 | 166 | 216,379 | 0 |
> | 5 | 685 | 519 | 166 | 195,248 | 315 |
> | 10 | 674 | 508 | 166 | 195,545 | 304 |
> | 25 | 665 | 499 | 166 | 195,775 | 295 |
> | 100 | 624 | 458 | 166 | 198,463 | 253 |
>
> **The factor is not a tuning knob.** Loosening it twenty-fold removes 62 of
> 315 rejections. Of those 315 rejected points the median sits at **191× the
> chord tolerance** and the maximum at **617×**; only 14 fall in the 5–10× band.
> Anything from 5 to 100 selects the same population. A boundary point at 191×
> tolerance is not export slack, so the second reading was the right one:
> ~1.2% of this model genuinely violates curve-on-surface incidence.
>
> **But the gate repairs nothing visible.** With the gate off and at 5,
> `find_blobs` reports the same 10 blob shells with ratios identical to five
> decimals (160144 at 43.4, 160784 at 42.1, 161274 at 30.3). The gate was
> confirmed to fire 315 times inside that same binary, so this is not a plumbing
> artifact. PR 1 costs 292 faces and 21,131 triangles — about 10% of the
> model — and fixes not one blob.
>
> **This splits one assumed defect into two.** The incidence violation the gate
> detects is real and is *not* what produces the blobs. The blob cause remains
> open (see below), and the transform provenance dump is still the next probe.
>
> **The gate was masking a hard crash.** Turning it off exposed an abort on ABC
> `00000730` — a request for 6,638,692,106,004,871,184 bytes. Root cause found
> and fixed (see below); the model now renders with the gate off at 425,328
> triangles, *better* than the 423,170 it produced with the gate masking the
> fault. Recorded because the general lesson is sharper than the bug: a
> validation layer that silently prevents a downstream crash makes the system
> look sounder than it is, and removing it looks like a regression. That is an
> argument for fixing causes rather than adding gates.
>
> **Contracts:** measures `GEO-005`; partially `GEO-006`. It does **not**
> satisfy either — a refusal discards the residual instead of returning a
> `Projection { uv, projected, residual, stationarity_error }` and a
> `WithinTolerance` witness, so the measurement that justified the refusal is
> thrown away at the moment it matters. Detection and policy are also still
> fused: the factor doubles as both the detector threshold and the reject
> switch, where `InvalidGeometryPolicy` (§31) should separate them.
>
> **Disposition: `COMPATIBILITY_FACTOR` is now `f64::INFINITY`** — the gate is
> compiled in and off. Deleting a tenth of the model's triangles to fix nothing
> is the wrong default, and the same measurement is available on demand via
> `TRUCK_COMPAT_FACTOR=5`. Turn it on for real only when something downstream
> can act on the refusal rather than just drop the face.
>
> Baseline correction: this row previously read 449 lost / 276 no surface /
> 216,129 triangles. Re-measured with the gate disabled it is 393 / 227 /
> 216,379. The difference is unattributed and most likely predates one of the
> fixes that landed in the same session.

### PR 2 — Typed identities and arenas (LANDED 2026-07-29)

`truck-stepio/src/in/arena.rs`. Two kinds of number were both `usize` — the
entity id a file writes as `#1234`, and the position a converted value lands at.
Now `SourceId<K>` and `Index<K>`, tagged by kind, so `EdgeCurveId`, `EdgeIndex`,
`VertexPointId` and `VertexIndex` are mutually unsubstitutable. The tag is
`PhantomData<fn() -> K>` over uninhabited markers, so it costs nothing at
runtime and imposes no variance of its own.

`Arena::try_insert(id, convert)` runs the conversion **first** and claims a
position only if it produced a value, so `items.len() == positions.len()` with
every position addressing the value converted from the id that maps to it — by
construction, not by assertion. An `Index` has no public constructor: it comes
from an insert that succeeded or a lookup that found one. `position()` is the
single documented escape hatch, used only where `CompressedEdge` and
`CompressedEdgeIndex` demand a bare `usize`.

Regressions in `arena::tests`: valid A / invalid B / valid C yields C at
position 1 addressing C; a repeated identity converts once and stores once; and
every mapped identity addresses its own value across a run of interleaved
failures.

> **This found a live instance of the defect it was built to prevent.**
> `shell_vertices` still had reserve-before-convert — it inserted the position
> into `vidx_map` and *then* called `get_owned`, which can fail. The point was
> never pushed, the entry stayed, and every later vertex was addressed one slot
> past where it sat. Only the edge path had been fixed; the vertex path had the
> same bug the whole time and nobody had looked.
>
> **Measured: no behavioural change on `00009190`** — 604 of 24202 lost,
> 214,211 triangles, same 10 blob shells, all byte-identical to PR 3 alone. The
> vertex defect is latent on this corpus: no `VERTEX_POINT` conversion fails on
> this model, so the desync never fires. That is the expected shape of a
> soundness change. It removes the possibility, not a current symptom, and the
> right time to remove it is before a file arrives that triggers it.

`TRUCK_PROBE_IDENTITY` is **deleted**. It checked after the fact that every
mapped index addressed the edge it named; that question can no longer have a
bad answer, so the probe became vacuous. First of the diagnostic-to-certificate
promotions the tooling section calls for.

**Contracts:** `TOP-002` discharged. `TOP-001` and `TOP-007` were left partial
here and are closed by PR 2b below.

### PR 2b — Retained identity, canonical naming, surfaces (LANDED, MEASURED 2026-07-29)

§33a items 1–4. Corrections to what PR 2 and PR 3 landed, not new capability —
so the expected measurement was *no change*, and that is what it produced.

**1. `ClosedWire` → `TopologicallyClosedWire`** (§24). The unqualified name
asserted three propositions to every reader — vertex identity, metric closure,
quotient closure — where the constructor establishes only the first. The type
doc now names the two it does *not* prove and points at `ARR-001` and `QUO-002`.

**2. Source identity in every arena item** (`TOP-001`, §22). Items are
`Stored { source_id, value }`, one `u64` each. This is not needed to *maintain*
the invariant in a correct arena — it is needed to check it instead of trusting
it, and to print it when it fails. `Arena::get_checked(index, requested)`
returns `IdentityMismatch`, whose `Display` is exactly §61's first example:

```text
TOP-001 failed: requested #714381, but arena index [62] stores #714442
```

It is called on **every** edge reference a face bound resolves — one integer
compare per edge use, structural tier. Zero fired across the six models swept.
That is the correct result and worth stating plainly: the check is not there to
find a bug today, it is there so that the day map and vector do disagree, the
model says which two entities were confused instead of rendering a smooth wrong
region.

**3. `try_insert` → `get_or_try_insert`** (`TOP-007`, §22.1). Behaviour
unchanged; the name and doc now say that a repeated reference *resolves* and
that only a second canonical object for one identity would be a defect.

**4. Surfaces through the same arena** (§51a). A surface is now converted once
per source entity rather than once per face, and `same_sense` inversion is
applied to the copy the face takes — never to the canonical entity, since two
faces may share one `CYLINDRICAL_SURFACE` and disagree about sense.

> **Item 4 is not finished and is not being ticked.** Faces do not use the
> arena, and the reason is not oversight: a face has no compacted identity to
> resolve. There is no face id → index map, because `CompressedFace` is built
> inline and owns its surface by value. Every map/vector pair remaining in the
> converter is now an `Arena`, so the reserve-before-convert class is empty *as
> it currently exists* — but §51a's actual demand, one implementation for every
> entity kind, is unreachable for faces until §33a item 11 gives
> `CompressedFace` a `source_id` and a `SurfaceIndex`. TOP-001 for faces is
> therefore not merely unchecked; it is unaskable. Recorded rather than
> rounded up, because "every arena is generic now" is the same shape of claim as
> "every call site has been repaired", and §51a exists because that claim was
> wrong last time.

> **Measured. No behavioural change, which is the point.**
>
> `00009190`: 604 of 24202 lost (274 failed to convert / 227 no surface / 103
> meshed to nothing), 214,211 triangles — identical to the PR 2 + PR 3 baseline
> to the digit. `find_blobs` reports the same 10 shells with ratios identical to
> five decimals. `00000730`: 425,328 triangles, matching its recorded figure
> exactly.
>
> Swept over six models, all terminating, no aborts, no `TOP-001` fires:
>
> | model | faces lost | of | triangles |
> |---|---:|---:|---:|
> | `00009190` | 604 | 24,202 | 214,211 |
> | `00000730` | 885 | 30,302 | 425,328 |
> | `00000414` | 936 | 19,187 | 164,947 |
> | `00005641` | 122 | 179,656 | 823,994 |
> | `00003172` | 1,087 | 22,971 | 109,319 |
> | `00009272` | — | — | 429,120 |
>
> The last four rows had no recorded prior numbers, so they are a **new
> baseline, not a no-regression result** — the honest claim is that `00009190`
> and `00000730` are unchanged and the rest now have figures to be compared
> against next time. A true before/after on the other four was attempted and
> abandoned: PR 2 and PR 3 are uncommitted in the same file, so stashing this
> change reverts them too and the "baseline" would have been three PRs old.
> **That is an argument for committing the fork**, which is already the
> release blocker below.
>
> Tests: `truck-stepio` 24 lib tests pass, up from 22 — the two new ones cover
> a checked lookup accepting its own identity and refusing another by name.

**Contracts:** `TOP-001` discharged for edges, vertices, and surfaces —
retained, checkable, and reported. `TOP-007` discharged and named. `TOP-002`
extended to surfaces. `TOP-004`'s type is correctly named. **First change in
this project to cite contract IDs in the code itself** (§33a item 12), in
`arena.rs`, `wire.rs`, and `convert.rs`.

### PR 3 — Fail-whole-bound conversion (LANDED, MEASURED 2026-07-29)

`face_bound_to_edges` used `filter_map`, so every `?` **dropped that edge from
the wire** rather than failing the face. A bound missing an edge is a broken
bound, not a shorter one.

Both levels are now all-or-nothing: edges collect into `Option<Vec<_>>` so a
bound resolves every `ORIENTED_EDGE` its source names or does not exist, and the
bounds of a face collect the same way, because losing an inner bound fills in a
hole and losing the outer bound promotes the holes to the outline. Empty wires
are refused. The source-use versus resolved-use count is discharged by
construction — the collect yields exactly `edge_list.len()` indices or nothing —
with a `debug_assert` recording the intent.

**`ClosedWire` landed 2026-07-29** (`truck-stepio/src/in/wire.rs`), so the
paragraph below is now history. Because PR 2 supplies edge endpoints, the type
checks something much stronger than "every edge resolved": it walks the chain
and requires each edge to end where the next begins and the last to close back
to the first, on **vertex identity, not position** — two `VERTEX_POINT`
entities at the same coordinates are two vertices, and a wire relying on them
coinciding is relying on the exporter. A single edge is checked against itself,
which is correct: a lone edge bounds a face only if it is a full loop. Nine
regressions cover open chains, a non-joining edge that nonetheless resolves, a
wrongly oriented edge, the single-circle case, and the empty wire.

Measured: **no additional rejections anywhere.** Identical output on
`00009190`, and on five more corpus models every fully-resolved wire also
closes. The invariant now holds by type rather than by one function being
written correctly.

> **Renamed to `TopologicallyClosedWire` 2026-07-29** (PR 2b, §33a item 1).
> `MATHEMATICAL_FOUNDATION.md` §24 forbids an unqualified `ClosedWire`, because
> three different closure propositions exist — vertex identity (`TOP-004`),
> metric endpoint agreement (`ARR-001`), and closure modulo the period lattice
> (`QUO-002`) — and no one of them implies another. This type establishes the
> first only, and now says so.

**Contracts:** `TOP-003` and `TOP-004` discharged. `TOP-005` is **not**
addressed: effective orientation is composed from face, bound, oriented-edge,
and edge-curve flags, but never checked against source incidence.

The newtype exists, but it stops travelling one step short.
`CompressedFace::boundaries` is `Vec<Vec<CompressedEdgeIndex>>` from
`truck-topology`, so `TopologicallyClosedWire::into_edges` is called at the
moment a face is built and the proof is discharged there. A second construction
site could still hand `CompressedFace` a wire that does not close.
**This is the outstanding gap**, and under the owned-fork decision it is no
longer a boundary to respect: §33a item 11 changes `boundaries` to
`Vec<TopologicallyClosedWire>`.

> **Measured on `00009190`. It costs geometry and fixes no blob.**
>
> | | before | after |
> |---|---:|---:|
> | faces lost | 393 | **604** |
> | — failed to convert | 0 | **274** |
> | — no surface | 227 | 227 |
> | — meshed to nothing | 166 | **103** |
> | triangles | 216,379 | 214,211 |
> | blob shells | 10 | 10 |
>
> 274 faces were being assembled from incomplete wires. 63 of them already
> meshed to nothing, so they cost nothing to refuse; the other 211 were
> producing 2,168 triangles trimmed by a region their file never described.
> `find_blobs` is unchanged to five decimals, same 10 shells, same ratios.
>
> **The denominator had to be fixed first.** Dropping a face at conversion also
> drops it from `total`, so the first measurement read "330 of 23928" — better
> than the 393 baseline while 274 more faces were missing. `FaceTally` now
> carries `declared`, read from `ShellHolder::cfs_faces` before conversion, and
> loss is reported against it. That denominator cannot move.

Tests: `look` 54 pass. `truck-stepio` 47 pass, 2 fail — `assy::occt_assy` and
`tessellate_shape`, both `NotFound` opening files under `resources/`, which the
local fork clone does not have. `truck-meshalgo`'s suite cannot build for the
same reason (`resources/shape/bottle.json`). Environmental, not this change, but
it means **the meshalgo tests named as PR 9's template have not run here.**

### PR 4 — Source identity through `CompressedEdge`

Carry the `EDGE_CURVE` entity ID to the point of use and assert the edge a face
receives is the one its `ORIENTED_EDGE` named. Turns a whole class of geometric
inference into a one-line equality check. (A probe form of this already exists
behind `TRUCK_PROBE_IDENTITY`; it reports clean on all three reproducers.)

### PR 5 — Explicit domain semantics

Preserve `FACE_OUTER_BOUND` vs `FACE_BOUND` through `truck-stepio` — currently
both parse into the same `FaceBound` struct, discarding the outer/hole role the
file states explicitly. Add `BaseDomain { Empty, NaturalRange, PeriodicQuotient }`
and `BoundRole { Outer, Inner }` as `Known`/`Unknown`, and classify by
`base XOR parity` rather than inferring from loop count.

Note `closed.is_empty()` (the current gate) cannot be the final rule: parity
answers "how many boundaries crossed", not "was the starting region material".

### PR 6 — Quotient topology and deck consistency

`QuotientLoop` carrying `(k_u, k_v)`; a relative deck-offset solver across the
bounds of one face. Each bound is currently lifted from `sp(surface, pt, None)`
— an arbitrary principal value — so relative offsets between bounds were never
controlled. Measured: two bounds of one face at `quot_v = −1` and `+1`.

### PR 7 — Constraint provenance and conforming CDT

`insert_to` silently skips constraints it cannot add. Flood-fill labelling is
`O(T+E)` versus the current `O(nm)` per-point ray cast, but a *missing*
constraint leaks a label across a whole region, so it requires
`requested == inserted` verification with per-face fallback.

### PR 8 — Certified face and shell meshes

Surface approximation bound, shared-edge conformance, shell incidence.

### PR 9 — Property and metamorphic harness

Chart reflection, wire reversal, seam shift, cyclic rotation, tolerance sweep.
The two existing tests in `truck-meshalgo/tests/tessellation/trimming_domain.rs`
are the template.

---

## The open defect: 2 remaining blob shells

Reproducers: `../look-trimming-residual/repro/blobs/` — `shell_160144.step`
(76 faces), `shell_160014.step` (53). Each reproduces standalone at the parent
model's tolerance and is correct at its own:

```console
cargo run --release --example find_blobs -- shell_160144.step 0.003056   # blob
cargo run --release --example find_blobs -- shell_160144.step            # clean
```

### What is established by measurement

- The **source STEP is correct**. All 15 circle/cylinder pairs in 160039 are
  exactly incident: `e_angle = 0`, `e_axis ≤ 1e-14`, `e_radius = 0`.
- **Surface conversion is correct.** Fitted cross-section radii match source
  cylinder radii to five decimals.
- **Curve conversion is correct.** Converted edge radii reproduce the source
  inventory {0.003, 0.00337, 0.0035, 0.004, 0.0045, 0.006}.
- **Edge association is correct.** `TRUCK_PROBE_IDENTITY` reports
  `mismatched=0`, `edges == mapped`, on all three.
- Yet boundary points sit **0.027 from their own surface** — nine times the
  chord tolerance — and `d_min` by brute force over the whole parameter domain
  equals the projection's residual, so **the projection is right and the input
  is wrong**.
- `surface.invert()` accounts for 160039 but **not** 160144 or 160014.

### Ruled out as the cause (2026-07-29)

Neither the residual gate (PR 1) nor fail-whole-bound conversion (PR 3) moves
the blob count: 10 shells before and after each, ratios identical to five
decimals. Between them they refuse 566 faces on grounds that are individually
sound. Whatever builds the blobs survives both checks, so it is not a boundary
point off its surface and not a wire missing an edge.

### Remaining candidates

Face→surface pairing (checked for edges, **not** for surfaces), or a transform
applied inconsistently between curve and surface. The next probe is the
transform provenance dump: for one bad face, evaluate source and converted
geometry in the same coordinates at each transform stage, and find the first
stage where `d(C(t), S) ≤ ε` stops holding.

---

## Unbounded sample counts (FIXED 2026-07-29)

`RevolutedCurve::parameter_division` computed its angular sample count as
`1 + ((vrange.1 - vrange.0) / acos(1.0 - tol / max)).floor() as usize` with no
ceiling, and it is reached with untrusted numbers from both directions: `acos`
collapses toward zero as the revolved radius grows against the tolerance, and
`vrange` is the bounding box of a lifted boundary, which a bad lift can make
span many periods. Degenerate radii land here too — zero radius makes the
argument infinite and `acos` NaN, which the old cast turned into a division by
zero.

`sub_parameter_division` already had `MAX_DIVISION_CELLS` for exactly this
reason; this specialised path bypassed it. Now capped by `MAX_CIRCLE_DIVISION`
(4096, a chord error of about 2.3e-7 of the radius — finer than any tolerance
reaching this code) with non-finite requests falling back to a usable division.
Four regressions in `parameter_division_bounds`; each one aborted the process
before the cap existed, so the assertions matter less than the tests returning.

**Contracts:** discharges `RES-001` and `RES-004`. **Violates `RES-003`.**

**The cap is safe but it lies.** A face capped at `MAX_CIRCLE_DIVISION` returns
its approximation as ordinary success, so a mesh that could not reach the
requested tolerance is indistinguishable from one that did. Per `RES-003` a
resource-capped result must be reported as `ResourceCapped { requested, used,
achieved_error }` and must not claim the tolerance it was asked for.
`MAX_DIVISION_CELLS` has the same defect and predates this session. **This is an
open defect, logged rather than fixed**, and it is exactly the failure mode the
architecture exists to prevent: a plausible answer where an honest refusal
belongs.

**Look for the same shape elsewhere.** Any count derived from imported geometry
and used as an allocation size is this bug. The two caps above are the known
instances; there is no reason to assume they are the last. `RES-001`'s checked
`SampleCount` constructor retires the class.

## Acceptance criteria

Completion is measured on **four axes**, not one — see
`MATHEMATICAL_FOUNDATION.md` §60–§60c. "The types exist" is Axis 1 only.

| Axis | What it asks | Status |
|---|---|---|
| Structural | no bare identity/index ambiguity, no silent topology loss, no unbounded derived allocation, no forgeable proof-bearing state | partial |
| Corpus correctness | every model terminates, no unexplained aborts, blobs either fixed or failing at a named contract, no regressions, every missing face categorised | 6 of 20 models measured |
| Performance | time, peak memory, persistent certificate memory, time to first usable image, against a pinned baseline | **no baseline pinned** |
| Diagnostic quality | for each known reproducer, the first reported failed contract localises the defect to the right stage | not started |

The performance axis has no baseline at all, which means the certification work
currently has no cost ceiling. Pin one before the certificate-carrying types of
PR 4 land, not after — retrofitting a budget to a design that ignored it is how
this ends up slower than the CAD applications it exists to avoid.

## Traps — read before trusting any measurement here

- **Detectors have had the bug they were hunting, twice.** `find_untrimmed`
  measured shells from vertices only, and a circle has one vertex, so every
  circle-bounded shell measured near zero and its faces looked enormous. The
  widely-quoted "710 of 24,201 faces untrimmed" came from that and **is not a
  real number**. Two shells named as worst offenders render correctly.
- **A validation layer can be load-bearing for something it was never meant to
  do.** The residual gate silently prevented the `00000730` abort by rejecting
  the offending faces before tessellation. Turning it off — correct on its own
  merits — looked like introducing a crash. Before removing any gate, establish
  what it is actually holding up.
- **A crash is a measurement result.** The corpus sweep is run over six models,
  not one, because `00009190` was clean while `00000730` aborted outright on the
  same build. A change verified on a single model is verified on a single model.
- **A loss ratio whose denominator moves is not a measurement.** Making bound
  conversion all-or-nothing dropped 274 faces *and* removed them from the face
  total, so the warning improved from "393 of 24202" to "330 of 23928" while the
  render lost geometry. Any stage that can refuse a face must be measured
  against what the file declared, never against what survived the stage before.
- **`look inspect` caches statistics keyed on the source file**, so a sweep that
  varies behaviour by env var and not by input reports the first run's numbers
  five times. Give every run a fresh `LOOK_CACHE_DIR`.
- **`ASSOC` probe resolution.** Falls back to a `(−1, 1)` window on an axis with
  neither range nor period; at `GRID=60` that is coarser than the residuals it
  reports. Its absolute numbers are not trustworthy.
- **`normal_align ≈ 1` proves nothing.** It is the first-order optimality
  condition of any nearest-point projection. An earlier reading of it as
  evidence of a tilted circle was wrong.
- **A three-point circle fit on a doubly-winding bound is meaningless** as a
  radius.
- Never pass a POSIX-style path to a Windows executable. It fails silently and
  has invalidated measurement runs.
- A measurement taken while the machine is short of memory or disk is not a
  measurement.

## Corpus

**`C:\Users\stefa\look-corpus\abc`** — 20 ABC models, 4.2 GB, moved out of a
temp scratchpad 2026-07-28 because that was the only copy.

**Not in the repo and cannot all be**: 12 of 20 exceed GitHub's 100 MB per-file
limit (up to 540 MB), and 4.2 GB busts the free LFS tier while every CI clone
would pull it. The five smallest are 67–86 MB; committing them was discussed and
not done.

## Companion repositories

- `../look-untrimmed-bug` — the original complement defect: plain-English
  statement, both code paths (truck and OCCT) with real code, extraction tool.
- `../look-trimming-residual` — formal treatment of the invariance fix and the
  residual scale defect, plus the isolated blob reproducers and renders.

## Tools

- `examples/find_blobs.rs` — per-shell mesh extent versus own extent. Optional
  second argument overrides the tolerance, which is how an extracted shell is
  meshed at its parent model's tolerance.
- `examples/find_untrimmed.rs` — per-face variant. Slow (24k separate
  triangulations) and superseded by `find_blobs` for most purposes.
- `examples/face_profile.rs` — per-face timing.
- `../look-untrimmed-bug/tools/extract_shell.py` — transitive closure of one
  `CLOSED_SHELL` into a valid standalone Part 21 file.

Fork probes, all env-gated: `TRUCK_PROBE_BOUNDARY`, `TRUCK_PROBE_LIFT`,
`TRUCK_PROBE_EDGE`, `TRUCK_PROBE_ASSOC`, `TRUCK_PROBE_COMPAT`, and
`TRUCK_NO_INVERT` (disables `surface.invert()`). `TRUCK_COMPAT_FACTOR` sets the
residual gate's strictness; it is `inf` by default, which is off.
`TRUCK_PROBE_IDENTITY` is gone — promoted to a structural guarantee by PR 2.
**These are diagnostic scaffolding and should be culled or promoted to typed
certificates before the fork rev is bumped.**

## Deferred performance work

Measured and real, but subordinate to correctness:

- STEP index buffer is the identity permutation — 4 bytes/vertex, 22 MB on a
  1.9 M-triangle model. Needs a non-indexed draw path.
- Vertices are unwelded; welding on position *and* normal keeps creases.
- `source_attributes` stores one identical 32-byte value per vertex.
- Entity table builds ~60 typed maps regardless of need.
- `upload_scene` makes a second full CPU copy before the GPU buffer exists.
- Non-converging NURBS faces: 15 of 24,202 held 98.3% of tessellation time.
  Needs an early exit in `sub_parameter_division`, not caching.
- `docs/patches/truck-synthetic-boundary-decimation.patch` — unlanded, carries
  profiling `eprintln!`s. May now be moot; re-measure before landing.

## How results are reported

- Benchmark release builds; retain raw samples and the hardware fingerprint.
- Fresh-process and resident-session numbers answer different questions and are
  never combined.
- F3D comparisons use `--force-reader=STEP`, explicit camera, resolution and
  background, and alternating launch order.
- Say when a number is attribution rather than a benchmark.

See `docs/BENCHMARKS.md`, `docs/ARCHITECTURE.md`, and `AGENTS.md`.
