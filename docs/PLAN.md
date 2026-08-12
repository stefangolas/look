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
>
> ### Superseded 2026-08-02 by the correctness map
>
> That sequence was written before the ingestion layer had been mapped against
> the formal system end to end. It now has been:
> [`CODEBASE_CORRECTNESS_MAP.md`](CODEBASE_CORRECTNESS_MAP.md) locates every
> correctness-relevant concept in production code,
> [`CORRECTNESS_GAP_REGISTER.md`](CORRECTNESS_GAP_REGISTER.md) merges the gaps by
> root cause and orders them by dependency, and
> [`MINIMUM_CORRECTNESS_CUT.md`](MINIMUM_CORRECTNESS_CUT.md) ranks them.
> **Start there, not here.** The item list above is not wrong so much as
> unordered: it names obligations without saying which must be discharged first,
> and several of its items turn out to depend on gaps it does not mention.
>
> **Phases 0–1 landed 2026-08-02** (branch `fix/correctness-phase-0-1`, look
> `edd46d5`, truck-fork `4f4426bb`): G8, G11, G5a, G5b, G2, G7a. See
> "Correctness phases 0–1" in the roadmap below. **Next is Phase 2**, the lift
> and domain spine, whose ordering is the opposite of the obvious one — see
> there before starting.
>
> Item 12, "start citing contract IDs", is now in force: every commit in phases
> 0–1 names the gaps and contracts it discharges.

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

### PR 7 — Constraint provenance and conforming CDT (LARGELY LANDED 2026-08-02)

Superseded by G5a/G5b; see "Correctness phases 0–1" below.

The premise here was slightly wrong. `insert_to` did not silently skip
constraints — it returned a Boolean that the caller *did* check. What it could
not do was say which edges Spade actually created, because
`add_constraint` reports only that a count changed, and a request realized as a
chain leaves `get_edge_from_neighbors(from, to)` empty. `try_add_constraint`
returns the chain, which is the `requested == inserted` verification this item
asked for, obtained from the library rather than reconstructed after the fact.

Still open from this item: CDT-002 in full, which needs an atomic arrangement
to verify against (G4, phase 3).

### Correctness phases 0–1 (LANDED, MEASURED 2026-08-02)

Six gaps, in dependency order. Measured on ABC `00009190`, 24,202 declared
faces. Full detail in `CORRECTNESS_GAP_REGISTER.md`; the short version:

| Gap | What it stopped doing | Evidence |
|---|---|---|
| G8 | erasing typed failures into empty meshes | totals reconcile exactly; `ContradictoryDualParity x143` visible for the first time |
| G11 | forwarding a search hint on the wrong axis | commuting test, verified to fail without the fix |
| G5a | losing constraint roles on split chains | `unresolved_at_flood` 213 → 0 |
| G5b | guessing material semantics for unnameable edges | lands provably non-firing |
| G2 | accepting an unresolved periodic branch | 71 faces; 31 had been rendering geometry from a coin-flip |
| G7a | reporting "could not decide" as "outside" | 117,145 `Boundary`, 0 `Indeterminate` |

**Face count fell, and that is the result, not a regression.** 4,457 → 4,486
faces lost, 1,359,029 → 1,358,543 triangles. The system stopped claiming meshes
it cannot justify. Per §"Acceptance criteria" and `REFINEMENT_AUDIT.md` §4,
rendered-face count is not a correctness oracle and was not optimised for — the
`79eaaf36` line renders more faces precisely because it flood-filled across
boundaries it had failed to represent.

Three findings worth not rediscovering:

- **The A1 defect had returned by another route.** `insert_surface` used the
  same lossy role lookup, so a *sampling grid* edge realized as a chain lost its
  role and fell to the toggling default. Fixing it recovered 21 faces.
- **`ConstraintOverlapUnsupported` and `ConstraintRoleMissing` were declared and
  never constructed.** Both now are, on real faces (9 and 0 respectively).
- **G11 cost 19 faces on its own** before the rest of phase 0 absorbed it. A
  correct upstream fix perturbing an unjustified downstream heuristic is a
  symptom of absent preconditions, not an argument against the fix.

### PR 6 / Phase 2 — the lift and domain spine (NEXT)

PR 6 as written below is a subset of this and is superseded by it.

**The ordering is the opposite of the obvious one, and this was got wrong once.**
The instinct is to derive a correct face domain first and then fix the lift. That
is circular: `PolyBoundaryPiece::try_new` anchors `quot_u` on `u0` taken from
`try_range_tuple()`, and `working_range` derives its extent from the *already
normalised* pieces. An extent derived from a lift still anchored on the
fabricated origin inherits the fabrication.

Correct order:

1. **Remove domain authority from lifting.** The lift's only ambient input is
   the certified lattice.
2. **Retain each arc's endpoint deck displacement** δ, and build relations only
   from established endpoint, source-incidence or seam evidence.
3. **Solve face-level deck potentials.** `domain/deck.rs::DeckPotentialUnionFind`
   is already the correct QUO-004 solver and has never been called, because the
   `[k_u,k_v]` it consumes is computed in `PolyBoundary::new` and dropped.
4. **Return coherent components**, retained winding, **free gauges**, or a typed
   contradiction. A free gauge is a result, not a failure — and not permission
   to pick a placement.
5. **Derive the working cover afterwards**, by finite candidate-translate
   enumeration (FORMAL_SYSTEM Def. 16–17, Lemma 1) for whatever the solve left
   free.
6. **Carry source/synthetic origin and effective orientation on every segment at
   creation** — not retrofitted afterwards.

**Measured 2026-08-02, before attempting any of this.** Two placements were
built and both rejected; see `CORRECTNESS_GAP_REGISTER.md` "Phase 2".

- Deleting the primitive-range anchoring **without replacing it** is worse:
  4,486 → 4,583 faces lost, 1,358,543 → 1,351,456 triangles. The fabricated
  origin is wrong but *load-bearing* — it was holding a face's bounds in one
  deck copy. So step 1 cannot ship alone, and the placement must be replaced
  rather than removed.
- Anchoring every bound to the translate nearest the first bound's centroid
  measures almost exactly neutral and is **still wrong**: it is a whole-component
  gauge choice being passed off as the per-arc δ of FS Def. 9, it asserts a
  relation between bounds that share no vertex, it makes STEP bound order
  semantic, and near half a period it rounds silently — reintroducing the very
  defect class G2 had just removed.

The second point is the reason this phase needs metamorphic acceptance rather
than a face count: no corpus number distinguishes a correct placement from a
lucky one. Required witnesses are listed in the register.

**A derived extent is computational support, not trim authority.** It may
justify clipping, finite search, bounding boxes, and artificial cuts whose two
sides are identified. It does not justify physical closure. The exit condition
is therefore *not* "stitching reaches a derived rectangle" but:

> No segment is assigned physical-boundary semantics merely because it lies on,
> or was constructed from, the working cover extent.

That is why step 4 cannot wait for step 1 to shrink the synthetic population:
any synthetic segment that survives must already carry its origin.

### PR 8 — Certified face and shell meshes

Surface approximation bound, shared-edge conformance, shell incidence.

### PR 9 — Property and metamorphic harness

Chart reflection, wire reversal, seam shift, cyclic rotation, tolerance sweep.
The two existing tests in `truck-meshalgo/tests/tessellation/trimming_domain.rs`
are the template.

---

## `VERTEX_LOOP` support: 604 → 396 on ABC, 0 on NIST (2026-07-29)

STEP lets a face bound reference an `EDGE_LOOP` **or** a `VERTEX_LOOP`, and
`bound_holder` checked only the first — with a comment admitting it: *"For now,
we are going with the policy of accepting nothing but edgeloop."* Every
`VERTEX_LOOP` therefore destroyed its whole face.

A collapsed bound now contributes **no trim segment**. The apex is closed by the
surface's own degeneracy, so nothing is the honest contribution — a synthesised
zero-size loop would trim the face by an empty region and delete it just as
thoroughly. `FaceBoundLoop` keeps the two kinds apart at the type level so that
mistake is not expressible. A face whose bounds *all* collapse is still refused
(`AllBoundsCollapsed`), because trimming by no boundary at all emits the entire
unbounded surface — the blob failure mode.

| | before | after |
|---|---:|---:|
| ABC `00009190` faces lost | 604 | **396** |
| — failed to convert | 274 | **3** |
| — meshed to nothing | 103 | 166 |
| faces rendered | 23,598 | **23,806** |
| triangles | 214,211 | **216,335** |
| blob shells | 10 | 10 (ratios identical to 5 dp) |
| NIST faces lost | 356 | **356** |

**Necessary, and not sufficient.** Resolving the entity is done; the geometry is
not. Of the 404 faces that now convert:

- **ABC: +208 render, not +272.** 64 of the recovered apex faces convert and
  then mesh to nothing.
- **NIST: +0.** All 132 turned from `LoopReferenceUnresolved` into
  `MeshedToNothing`. Spot-checked triangle counts are identical, so nothing
  regressed — nothing improved either.

Trimming a cone by its outer circle alone, with the apex closed by degeneracy,
yields an **empty material region** for every NIST apex and a quarter of the ABC
ones. That is domain semantics, not parsing: the classifier cannot infer which
side is material from the one remaining loop, which is precisely what `DOM-003`
(explicit base domain) and `QUO-005` (singular charts) exist for. `PLAN`'s PR 5
note already says `closed.is_empty()` cannot be the final rule; this is that bill
arriving.

Faces carrying a collapsed bound are counted under `TRUCK_PROBE_SINGULAR`, since
their domain now has a singular point nothing downstream is told about.

## The missing-face repair queue (2026-07-29)

`examples/face_census.rs` attributes every lost face to a reason, taken from the
conversion that lost it — `to_compressed_shell` now delegates to
`to_compressed_shell_with_losses`, so there is one conversion path and the census
cannot drift from what the renderer does. It reproduces the renderer's own
numbers exactly (272 + 2 = 274 conversion, 227 no-surface, 103 empty).

**`00009190`, 604 of 24202 lost:**

| stage | reason | surface | count | share |
|---|---|---|---:|---:|
| convert | `LoopReferenceUnresolved` | — | **272** | **45.0%** |
| tessellate | `NoSurfaceProduced` | bspline | 112 | 18.5% |
| tessellate | `NoSurfaceProduced` | nurbs | 70 | 11.6% |
| tessellate | `MeshedToNothing` | plane | 53 | 8.8% |
| tessellate | `NoSurfaceProduced` | cylinder | 44 | 7.3% |
| tessellate | `MeshedToNothing` | cylinder | 20 | 3.3% |
| convert | `EdgeCurveConversionFailed` | — | 2 | 0.3% |

**All 33 NIST, 356 of 7902 lost** — two categories are 98% of it:

| stage | reason | surface | count | share |
|---|---|---|---:|---:|
| tessellate | `NoSurfaceProduced` | cone | **216** | **60.7%** |
| convert | `LoopReferenceUnresolved` | — | **132** | **37.1%** |

Cross-checked against the pre-unit-fix sweep: `ap242 ftc_07` 16, `ftc_10` 16,
`stc_07` 16, `stc_10` 8, `ctc_05` 10 — all unchanged, so both populations
pre-date the angle fix.

### Top item, root-caused: `VERTEX_LOOP` is unsupported

`00009190` contains exactly **272 `VERTEX_LOOP` entities and exactly 272
`LoopReferenceUnresolved` failures** — a 1:1 match. `FaceBoundHolder::bound_holder`
resolves a bound only against `table.edge_loop`, so every `VERTEX_LOOP` kills its
face:

```text
#45222  = ADVANCED_FACE( 'PARTBODY', ( #108199, #108200 ), #108201, .F. )
#108199 = FACE_BOUND( '', #263343, .T. )        <- #263343 is a VERTEX_LOOP
#108201 = CONICAL_SURFACE( '', #263345, 0.00166, 1.0297 )
```

A `VERTEX_LOOP` is a degenerate single-vertex bound — a cone apex or sphere pole,
and the surface above is conical, so it is the apex. This is the **most
consistent category across both corpora**: 45% of loss on `00009190`, 37% on
NIST.

**It is not a parsing gap to paper over.** A collapsed boundary is a singular
chart (`QUO-005`): the bound has no edges, contributes no trim curve, and the
parameter domain closes to a point there. Supporting it means deciding what the
trimming domain of such a face is, not just resolving one more entity type.

### The 216 cone failures are a *separate* defect, measured 2026-07-29

It was worth asking whether they shared the apex/singularity cause, because that
would have made one fix recover both. **They do not.** Three measurements:

**Perfectly anti-correlated.** Across all 33 NIST models, every model with cone
tessellation failures has **zero** `VERTEX_LOOP` entities, and every model with
`VERTEX_LOOP` failures has **zero** cone failures. Not one model has both.

| part | encoding | cone no-surface | `VERTEX_LOOP` in file | loop failures |
|---|---|---:|---:|---:|
| `ctc_02` | ap203geom | 148 | 0 | 0 |
| `ctc_02` | ap203pmi | 0 | 74 | 74 |
| `ctc_05` | ap203geom | 20 | 0 | 0 |
| `ctc_05` | ap203pmi | 0 | 10 | 10 |
| `ftc_07` | ap242 | 16 | 0 | 0 |

The anti-correlation is an **encoding artifact, not a shared cause**: the two
encodings of one part model the same features differently, so each file exhibits
only one of the two defects. The tidy 2:1 ratio is a property of how each
exporter splits those features and nothing more.

**The failing cone faces are not collapsed boundaries.** Sampled face `#4932` of
`ap203geom/ctc_05`:

```text
#4932 = ADVANCED_FACE('',(#4931),#4924,.F.)
#4931 = FACE_OUTER_BOUND('',#4930,.F.)
#4930 = EDGE_LOOP('',(#4926,#4928,#4929))      <- three real edges
        two LINEs and a CIRCLE, three distinct vertices, none coincident
```

An ordinary trimmed cone patch. No degenerate edge, no apex vertex, nothing
singular. `VERTEX_LOOP` support cannot touch it.

**Nor is it the angle-unit bug.** It occurs in files that declare degrees *after*
conversion (`ap203geom/ctc_02` at 59°) **and** in files that were always in
radians (`ap242/ftc_07` at `1.0297442575` = the same 59°). A defect present on
both sides of the unit fix is not a unit defect.

So this is a **third independent family**: trimmed cone patches that convert
correctly and then produce no triangles. Unexplained, untouched, and it will
**not** be recovered by the `VERTEX_LOOP` work.

**Consequence for the estimate.** Fixing `VERTEX_LOOP` should recover ~272 faces
on `00009190` and ~132 across NIST — **404 faces, not 404 + 216.**

## Plane-angle units: `ftc_07` FIXED (2026-07-29)

**The first blob to be fixed, and it was not a geometry defect.**

`nist_ftc_07_asme1_rd` declares plane angles in **degrees**, and the importer had
**no unit handling of any kind**:

```text
#42  = (GEOMETRIC_REPRESENTATION_CONTEXT(3)
        GLOBAL_UNIT_ASSIGNED_CONTEXT((#24,#28,#38)) ...)
#24  = (CONVERSION_BASED_UNIT('DEGREE',#20) NAMED_UNIT(#19) PLANE_ANGLE_UNIT())
#20  = PLANE_ANGLE_MEASURE_WITH_UNIT(PLANE_ANGLE_MEASURE(0.0174532925),#18)
#686 = CONICAL_SURFACE('',#685,0.282184119986423,1.999999999999705)
```

That cone is 2°, read as 2 radians, and the value goes straight into `tan`:
`tan(2°) = 0.0349` against `tan(2 rad) = −2.185`. **Wrong by 63× and inverted in
sign**, so each corner draft face opens backwards at enormous angle. Those were
the fans.

**Why angles and not lengths.** The same file is in *inches* and always was, with
no ill effect: a length unit is a uniform scale, the tolerance is relative, and
nothing downstream cares. An angle is not scale-covariant. An angle in degrees
beside lengths in inches is dimensionally inconsistent, and the error is a
different *shape* rather than a different size. That asymmetry is why a total
absence of unit handling stayed invisible for the whole life of the project and
then produced a blob.

| | before | after |
|---|---:|---:|
| `ftc_07` triangles | 2,501 | **2,140** |
| escaping cone faces in the differential | 9 | **0** |
| render vs. its AP242 twin | fans | **agrees** |

**Corpus regression sweep, all 33 NIST models.** Seven change; every one of the
seven declares degrees; five change imperceptibly (1–92 triangles) and none
regress visually. ABC `00009190` is byte-identical — 604 of 24202, 214,211
triangles, the same 10 blob shells — because it declares no degree units, which
also means **this fixes none of the ABC blobs**.

**The honest-refusal rule caught a bug in itself.** Resolution applies a factor
only when every *independently assigned* declaration agrees, and warns rather
than guessing otherwise. The first version refused every file it existed to fix,
printing `plane angle units disagree (1 vs 0.0174532925)`: a degree unit is
*defined* as a multiple of a radian unit, so every degree file necessarily also
contains a radian `SI_UNIT` — referenced, not assigned. Conversion bases are now
excluded rather than compared, and the regression test is named for it. Worth
recording because the refusal is what made the mistake visible instead of
silently converting by the wrong factor.

**Still open on this front:** `PARAMETER_VALUE` trims on circles are angle-valued
and are **not** converted. `ftc_07` contains none; **20 of 33 NIST files do**, and
`ctc_05` is one — which is why `ctc_05` improved (2,230 → 2,196 triangles) but
still renders its shaft as a funnel. That is the next concrete item.

Also unresolved by design: resolution is file-global, where the correct rule is
per-`GEOMETRIC_REPRESENTATION_CONTEXT`. Both of `ftc_07`'s two contexts declare
degrees, so this is sufficient here; a file mixing radians for geometry with
degrees for annotation gets a warning and no conversion.

**Contracts:** `GEO-001`. The declared transform now includes the unit
conversion, so converted geometry equals the transform of the source rather than
of a reinterpretation of it.

## The NIST corpus renders two blobs, and neither is reported (2026-07-29)

The NIST PMI set had only ever been checked computationally. It was rendered
and **looked at** for the first time on 2026-07-29, all 33 files, iso view at
512×512, contact-sheeted and inspected. Two are visually wrong.

**This corpus is a metamorphic test that was sitting unused.** Most parts ship
in three encodings — AP203 geometry-only, AP203 with PMI, AP242 — and the same
part in three formats must render as the same shape. That makes disagreement
self-evidencing: no reference renderer is needed, because the corpus disagrees
with itself.

| model | symptom | reported? |
|---|---|---|
| `ap203geom / nist_ftc_07_asme1_rd` | four corner fillets render as full revolved fans bursting out of the box, plus flared sheets along the top rim | **no warning at all** |
| `ap203pmi / nist_ctc_05_asme1_ap203` | the cylindrical output shaft renders as a cone with a disc cap on the end | 10 of 156 faces, but not this |

Both parts render correctly from their AP242 encoding, so the defect is in what
`look` does with the AP203 file, not in the part.

**`ftc_07` is the more valuable reproducer**: 2,501 triangles, no face loss, no
error, and a picture that is obviously wrong. It is the smallest fully silent
blob known, an order of magnitude smaller than `shell_160784`'s 20 faces in
context, and it comes with a correct rendering of the same part to diff against.
The corner-fan shape is the untrimmed-surface signature — a fillet whose trim
was lost renders as the entire surface of revolution.

### What the audit says about the detectors

- **A cross-format extent check catches `ftc_07` and misses `ctc_05`.** Comparing
  the largest bounding-box extent between encodings of one part flags `ftc_07` at
  1.20×, entirely automatically. It says nothing about `ctc_05`, whose cone sits
  *inside* the correct extent. A blob only inflates the bounding box when it
  escapes the part.
- **Beware 25.40.** Three parts show a 25.40× extent disagreement between AP203
  and AP242. That is inches against millimetres, not a defect. A ratio detector
  that does not special-case unit conversion will report four false positives
  out of five findings.
- **Face loss does not predict visual soundness in either direction.**
  `ftc_07` loses zero faces and is wrong; `ap203geom/ctc_02` loses 148 of 664
  and renders the right shape.

### Consequences for the roadmap

`00009190`'s 10 blob shells were the only visual evidence driving the
architecture, and they are all in one file from one exporter. There are now
independent reproducers from a different source, one of which is tiny and one of
which has a correct twin. **PR 9's metamorphic harness should take the NIST
multi-encoding property as a test family** — same part, three encodings, same
bounding box up to a declared unit factor, and ideally same volume.

Reproduce: extract `~/Downloads/NIST-PMI-STEP-Files.zip`, then

```console
look render nist_ftc_07_asme1_rd.stp   --views iso,front,top --atlas 3   # blobbed
look render nist_ftc_07_asme1_ap242-e2.stp --views iso,front,top --atlas 3   # correct
```

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
| Corpus correctness | every model terminates, no unexplained aborts, blobs either fixed or failing at a named contract, no regressions, every missing face categorised | 6 of 20 ABC models measured; all 33 NIST rendered and inspected, 2 blob silently |
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
- `../look-collapsed-boundary` — **the cone-apex domain defect**, isolated to a
  one-face reproducer that renders zero triangles beside a control face from the
  same file that renders 46. `FORMALISM.md` gives the geometry, labels every
  claim demonstrated / asserted / undemonstrated, and lists the seven open
  questions. Pushed to `stefangolas/look-collapsed-boundary`. The measured
  mechanism: `Line::parameter_range()` is `[0,1]` unconditionally, so a revolved
  cone declares `[0,1] × [0,2π)` while its apex sits at `u* = −R/tanθ = −6.01`,
  outside it — the base circle lands on the domain edge and stitches to a
  **zero-area** loop.
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
