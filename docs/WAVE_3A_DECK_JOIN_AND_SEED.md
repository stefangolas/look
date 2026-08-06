# WAVE-3A — the deck-consistent join and the structural-seed projection

Two routes, measured separately, both refinement-only, both default-on. Plus
the DIAG-001 extension that decided the first one's design, and a scoping
finding on the third package that stopped it being built blind.

Baseline: look `2519b6c`, truck `6f8153ea`, corpus-wide 819,769 rendered of
839,179 declared (19,410 lost, 2.31%).

---

## 0. The measurement that came first (DIAG-001 deck evidence)

The handoff required this before any of package 1 was designed, and it was
right to: the hypothesis it tested was read off the source, not measured, and
the previous edition of the handoff had been wrong exactly that way once
already.

DIAG-001 now records, per failed face:

- `boundary_pieces[]` — each piece's `ku`/`kv`, winding sign, signed
  parameter-space area, fundamental-domain representative, and both endpoints;
- `two_loop_join` — what the two-closed-loop branch of `PolyBoundary::new` did:
  both loops' displacements, the lattice translate the mean alignment applied,
  the `Σδ` the chosen traversal realises, and whether the other traversal would
  have closed the deck equation;
- `seam_mechanism` — the mechanism-level subtype derived from those.

Recording is suspended while a recovery route re-tessellates a face, so the
record — and the loss bucket the band routes admit on — keeps describing the
legacy boundary rather than a mixture of two attempts.

### What it says

Over all 19,410 lost faces, the two-loop join ran on **3,088**. Of those:

| | faces |
|---|---:|
| `OppositeWindingReversed` — `Σδ = ±2`, forward traversal closes it | 3,087 |
| `JoinDeckConsistent` | 1 |
| `JoinDeckUnsatisfiable` | 0 |

Every one of the 3,087 lost to `ConstraintInsertionIncomplete`. The observed
displacement pairs are only `((±1,0),(∓1,0))` and `((0,±1),(0,∓1))` — exactly
opposite unit windings, nothing else. The hypothesis was not merely supported;
the population is homogeneous.

It also corrects the handoff's target. The `+1,779 source×seam` faces the
handoff grouped with this class are **not** from this branch: 1,507 are
`SeamWithoutTwoLoopJoin` and 269 carry no seam evidence at all. Package 1's
real population was 3,087, not 4,850.

---

## 1. Deck-consistent two-loop join — **+3,085 faces**

`TRUCK_FORMAL_RECOVERY_DECK_JOIN`, default-on, nested under
`TRUCK_FORMAL_RECOVERY`.

### The defect

`PolyBoundary::new`'s two-closed-loop branch cuts both loops open, **reverses
one unconditionally**, and bridges them with a pair of `Seam` segments. For a
quotient-closed boundary walk `Σδᵢ = Δ_walk`, with `Δ_walk = 0` for a
contractible regular boundary. Reversing loop 1 realises `δ₀ − δ₁`. The two
boundary circles of a band wind *opposite* — as they must, for the face
boundary to be coherently oriented — so that sum is `±2`, the two bridges
become crossing diagonals instead of a rectangle's two vertical cut edges, and
Spade refuses the second one.

### The fix

The equation is decidable, so nothing is guessed:

- reversed closes it and forward does not → keep the legacy traversal;
- forward closes it and reversed does not → traverse forward (the 3,087);
- both close it, or neither → refuse, keep the legacy traversal, and let the
  face keep whatever typed failure it had.

The two bridges the corrected traversal produces are exact lattice translates
of each other by one period — the cut-pair relation — which is why they cannot
cross.

### Why it cannot regress

Structurally, not luckily. The legacy boundary is built and tessellated first;
the corrected join is reached **only** from the arm where that produced no
mesh, and it re-runs the *ordinary* tessellator on the *same* pieces. It
introduces no geometry the legacy path would not have accepted and it can
replace nothing but a failure.

### Measured

Route disabled vs. default-on, alternated per model over all 20 models,
reconciled per `source_face_id`:

```
rendered  819,769 -> 822,854      +3,085
lost       19,410 ->  16,325
rendered -> lost                        0
population drift                        0
```

3,085 of the 3,087 identified faces. The two that stayed lost failed the
retry's own tessellation and kept their typed failure, which is the refusal
path working.

---

## 2. Structural-seed parameter inverse — **+705 faces**

`TRUCK_FORMAL_RECOVERY_SEED`, default-on, nested under the master gate.

### The defect

`BoundaryProjectionFailed` is a numerical failure, not a geometric one.
`search_parameter` is a Newton iteration started from one point: a caller's
hint, or the single best cell of a **uniform** presearch grid. A B-spline is a
different polynomial on each knot span, and a uniform grid can put every one of
its samples inside one span of a knot vector that is dense elsewhere. Splines
are 4,522 of the 6,695 projection failures.

### The fix

`SearchParameter::search_parameter_seeds` — a new defaulted trait method
returning the starts the geometry's own structure suggests. `BSplineSurface`
and `NurbsSurface` return one per knot-span **cell**; repeated knots span
nothing and are dropped. Everything else keeps the default, which is empty:
"no structure to offer" is the honest answer for a primitive whose inverse is
closed-form.

The retry is the **last** link of the projection chain, so it is reached only
where every existing attempt returned `None`. A face that projects today
projects through the identical chain and receives the identical parameter.
Among converged seeds it takes the smallest 3D residual, breaking ties on
proximity to the hint — a spline can carry the same point in more than one
span, and taking whichever converged first would step the traversal across the
domain. The returned parameter is still subject to the caller's existing
incidence check; nothing is admitted here that another start would not have
been.

### Measured

```
rendered  822,854 -> 823,559        +705
rendered -> lost                        0
population drift                        0
```

Every recovered face is a NURBS or a B-spline. Not one face of any other family
changed state, which is what a route whose seeds only exist for splines should
look like.

**+705, not +4,522, and the gap is structural rather than disappointing.** A
face is lost if *any* one of its boundary points fails to project, so recovering
it requires the seeds to fix *every* failing point on it. Partial success on a
face recovers nothing. The right way to read 705 is as whole faces cleared, not
as a hit rate on points, and the next question for this route is the
distribution of failing points per face — which the current record does not
carry.

**The derive macros forward it.** The trait method is defaulted, so
`#[derive(SearchParameterD2)]` on the production `Surface` enum would otherwise
have compiled, run, and answered "no seeds" for every surface in every model —
a retry that can never fire, indistinguishable from a retry that does not help.
That failure mode has cost this project a measurement before (`face_census`
calling the cone entry point). `truck-stepio` carries a test that the
production enum forwards seeds, for exactly that reason.

---

## 3. ARR-003 — scoped, not built

The handoff names `GEN-001` (the chord-side audit item, A7) as the
prerequisite. **A7 is closed**, by GEN-001E's certified `2π`-copy enumeration
in `intersection.rs`; `GEN-001.md`'s §A7 paragraph still read "Now:
implementation" long after its own status header recorded the closure, and has
been corrected. So ARR-003 is not blocked on its prerequisite.

It is, however, not a splice into the planar slice. The measured population is
real and large — 3,591 rank-0 source/source crossings, 2,947 of them planar —
and `SliceExit` already names it exactly (`NonadjacentCrossing`,
`BoundaryComponentsCross`). But everything downstream of Step 7 is built on the
boundary being a **simple** Jordan curve: Step 8A's polygonal region, ear
clipping "inserts no Steiner points and emits only interior triangles", and
Step 8B's battery including "the mesh boundary equals the expected polygon
cycle". A normalized arrangement is not a simple polygon. Splitting the arcs at
their certified intersections gives a planar *arrangement*, whose faces have to
be extracted and whose material region has to be selected by parity (§X) —
a new Step 7′ and a new material selection, not a repair of the existing ones.

That is the next work packet, and it is a build rather than a fix. What exists
to build it on: `formal/bezier_isect::intersect_bezier_pair` (certified roots,
germs, transverse orientation, canonical pair-local identities),
`formal/exact`, and the typed exits that already isolate the population.
`ParameterEnclosure2` on the DIAG-001 witnesses is still `None` everywhere and
is the natural first observable.

---

## 4. Combined result

| | rendered | lost | loss |
|---|---:|---:|---:|
| WAVE-2C baseline | 819,769 | 19,410 | 2.31% |
| + deck-consistent join | 822,854 | 16,325 | 1.95% |
| + structural seeds | **823,559** | **15,620** | **1.86%** |

**+3,790 faces, zero `rendered -> lost` across 839,179 faces, zero population
drift.** Each route's own contribution stays one subtraction: set its variable
to `0` and diff against the default.

The residual's shape after this wave, against the handoff's table:

- `ConstraintInsertionIncomplete` 8,917 → ~5,830, and what remains of it is now
  overwhelmingly the rank-0 arrangement (§3), not the periodic seam;
- `BoundaryProjectionFailed` 6,695 → ~5,990, still the largest single reason,
  and its cylinder (1,072) and torus (939) rows are the ones the handoff said to
  **re-measure after package 1** — that is now due, and has not been done.

---

## Reproducing

Artifacts outside git, in `C:\Users\stefa\look-corpus\p1-out\`:

- `diag-deck/<id>.jsonl` — DIAG-001 with deck evidence, default-on, per model
- `deck_tab.py`, `DECK_TAB.txt` — the cross-tab above
- `diag-deck-sweep.sh`, `deck-join-sweep.sh`, `seed-sweep.sh`
- `deckjoin/{off,on}_<id>.{census,ledger}.txt`, `seed/{off,on}_…` — per-route
  subtractions, alternated per model
- `reconcile.py` — the per-`source_face_id` regression gate

Both sweeps were taken through the `.cargo/config.toml` path override, then the
override was re-commented and the pin bumped before these numbers were written
down.
