# Minimum correctness cut

**Audited artifacts.** `look` @ `e7fb495`; truck-fork @ `39346191`;
`truck-*` pinned @ `79eaaf36`; `spade` 2.15.1.

Recommendations only. Nothing here was implemented. Gap IDs refer to
`CORRECTNESS_GAP_REGISTER.md`; scope is named to the file and function, not to
field-level design.

---

## A. Minimum soundness cut

**Goal:** the system must never claim a correct mesh when a required obligation
is unresolved. This says nothing about *meshing more faces* — several items here
will make the tool report **fewer** successes, which is the point.

| # | Gap | Change in one sentence | Files / functions |
|---|---|---|---|
| A1 | **G8** | Stop converting a typed failure into an empty mesh; propagate `TessellationOutcome` to the caller. | [fork] `triangulation.rs:1885-1907` (`trimming_tessellation`), its two call sites `:351` and `:399`, and [look] `src/step.rs:196-…` where the loss tally already hand-reconstructs the distinction |
| A2 | **G2** | A lift that exhausts `MAX_LIFT_REFINEMENTS` must return a typed unresolved result instead of the ambiguous value. | [fork] `triangulation.rs:608-626` (`PolyBoundaryPiece::try_new`) |
| A3 | **G5** (role half) | Replace the four-call insertion dance with `try_add_constraint`, and record roles on the chain it returns. | [fork] `triangulation.rs:1446-1475`; [spade] `cdt.rs:807-817` |
| A4 | **G5** / **G7** | An unresolvable constraint role must not silently default to toggling material. | [fork] `triangulation.rs:1694-1698` (`toggles_material`) |
| A5 | **G7** | `include()` must not collapse "cannot decide" into "outside". | [fork] `triangulation.rs:1334-1357` |

### Why this ordering

**A1 is first and is a genuine prerequisite, not a convenience.** Every other
item on this list produces a *refusal*, and until a refusal has a typed
destination, adding refusals only converts silent-wrong into silent-empty — the
same failure with less information. A1 is also the cheapest item in the whole
document: one match arm, no semantic decision.

A3 before A4 because A4's correct behaviour depends on how many roles are
actually unresolvable, and A3 is what shrinks that population from "every split
chain" to "genuinely unknown". Doing A4 first would tune a threshold against a
defect A3 removes.

A2, A5 are independent of each other and of A3/A4; order among them is free.

### What A deliberately excludes

- **G9** (compatibility gate). Turning it on is a *policy* change with a measured
  cost of 292 faces and no measured benefit ([fork] `:874-889`). It becomes
  reasonable only after A1, and even then it needs its own sweep. Not soundness-critical
  in the strict sense: it makes a wrong answer *likely*, not *undetected*, once
  the other refusals are typed.
- **G1, G4, G6**. These are correctness-of-result gaps, not
  claims-more-than-it-knows gaps. They belong to spine B.

---

## B. Minimum useful correctness spine

**Goal:** a defensible source-to-mesh refinement path for the main supported
surface classes — planes, cylinders, cones, and non-periodic splines. Assumes A
is done.

| # | Gap | Change | Files / functions |
|---|---|---|---|
| B1 | **G11** | Transpose the hint alongside the result in `Processor::search_parameter`. | [pin] `truck-geometry/src/decorators/processor.rs:507-521` — **requires a fork bump**, this crate is pinned |
| B2 | **G1** | Make the working parameter extent an *output* of lifting rather than a read of the primitive's declared range. `working_range` already exists; the change is to derive it unconditionally and retire the two `try_range_tuple` reads. | [fork] `triangulation.rs:1025-1048` (`working_range`), `:484`, `:1062-1065` |
| B3 | **G3** | Retain `[ku,kv]` on the piece instead of dropping it after translation, and replace per-piece absolute anchoring with a relative rule anchored once per face. | [fork] `triangulation.rs:659-669`, `:1110-1130`; enables [fork] `domain/deck.rs` (`DeckPotentialUnionFind`) to be called at all |
| B4 | **G6** | Carry a per-segment origin discriminant through stitching so synthetic closure enters as `UnresolvedSyntheticClosure`, not `PhysicalBoundary`. | [fork] `triangulation.rs:1222-1295`, `:1208-1211`, `:628-658` (creation); `:1465` (classification) |
| B5 | **G10** | Retain the composed edge-use orientation as a fact on the lifted arc rather than only applying `curve.inverse()`. | [fork] `triangulation.rs:340-343`, `:515` |

### Why this ordering

**B1 first because it is nine lines and upstream of everything.** It degrades the
exact continuity mechanism B3 and A2 depend on, and leaving it in place means
every later measurement on inverted cylinders and cones is taken through a known
defect. It is also the only item requiring a pinned-crate bump, so starting it
early de-risks the dependency work.

**B2 before B3** because the deck-normalisation origin at `:662-668` is read from
the same fabricated rectangle B2 removes. Anchoring relatively (B3) is what makes
B2's removal of the absolute origin safe — audit §6.3 established that the
circularity is not essential, but the two must move together, B2 leading.

**B4 after B2** because B2 *shrinks* the synthetic-segment population rather than
reclassifying it. Doing B4 first means building provenance plumbing for segments
B2 will delete.

**B5 last** because it is the only item whose benefit is not realised until the
material solve stops using parity — which is set C. It is in B because the
evidence is available now and cheap to retain, and retaining it later costs a
re-derivation.

### What B is expected to achieve, and what it is not

B makes the lift's obligations **statable and checkable** for the main classes.
It does **not** discharge them, and it must not be described as recovering faces.
The prior negative result stands: the measured self-crossings are *intra-bound*,
so no lattice fix separates them. Whether a certified lift removes them or leaves
genuine transverse intersections is **Unknown** and answerable only as a
constructive witness after B3 — not by another population study.

---

## C. Full formal coverage remainder

Everything below is real work that A and B do not touch. It is listed for
completeness and ordering, not recommended for immediate action.

| # | Gap | Scope |
|---|---|---|
| C1 | **G4** | A `NormalizedArrangement` owning certified intersection classification, atomic subdivision, and incidence reconstruction. Absorbs ARR-002/003. Prerequisite for CDT-002 being discharged rather than approximated. |
| C2 | **G7** | Replace parity with the FS Def. 20 constraint system, yielding the Def. 21 trichotomy `Unique / Ambiguous / Inconsistent`. Requires B5's retained orientation to express `μ_L=1, μ_R=0`. |
| C3 | **G4** | Distinguish `~_Λ` from `~_Σ` at vertex identification instead of one proximity weld ([fork] `:1409-1412`), as FS §IX requires. Also retires the O(n²) linear scan. |
| C4 | — | Certified singular strata and local links: sphere poles, cone apexes, collapsed parameter boundaries. Currently ad-hoc `uder().so_small()` probes ([fork] `:441-464`, `:858`) plus one live classifier ([fork] `:2234`). `SingularStratum` / `StratumCertificate` ([fork] `domain/schema.rs:41-80`) are the intended types and prove nothing today. |
| C5 | — | Structural periodicity for the surfaces `lattice_of` currently carries as uncertified: `Sphere`, `ToroidalSurface`, `SweptCurve`, `OffsetSurface` ([look] `src/step/lattice.rs:49`, `:82`, `:90`). Each needs its own representation-derived witness. |
| C6 | — | Move consumers from `declared_period()` to `generator()` ([fork] `domain/lattice.rs:66-80`). This is the point at which the certified lattice starts *changing behaviour*; §0.2 of the map notes it currently does not. Each call site is a separate measured change. |
| C7 | — | Curve-on-surface evidence precedence (FS §VI): source p-curve first, then analytic inverse, then tracked numerical inverse. **No p-curve survives conversion today** — `truck-stepio/src/in/convert.rs` contains no p-curve handling at all, so level 1 of the precedence rule is unavailable and level 3 is what production uses. |
| C8 | — | The remaining FS §XVIII modules with no counterpart: `cover.rs` (finite translated-copy enumeration, Lemmas 1-2), `canonical.rs`, `normal_forms.rs`. |

---

## Summary of the ordering argument

Three properties drove every ordering decision above.

1. **A refusal needs a destination before it is worth producing.** This puts G8
   ahead of everything, including gaps that are individually more serious.

2. **Do not certify segments the next stage will replace.** This is the audit's
   own §5 argument and it survives re-derivation: G1 before G6, G2/G3 before G4,
   G4 before G5's completeness half. Building in any other order certifies the
   wrong objects.

3. **Separate the cheap independent corrections from the architectural ones.**
   G8, G11, and G5's role half are each a handful of lines, each independently
   valid, and each removes a distinct defect. They are worth doing before any of
   the type-boundary work, and they do not commit to a design.

The one place this map improves on the prior audit's sequencing is **G5**.
`REFINEMENT_AUDIT.md` §4 placed the realization bijection behind stage 3
(`NormalizedArrangement`) on the grounds that only an atomic arrangement makes the
bijection retainable. That is right for *completeness* — CDT-002 needs atomic
input. But the **role-loss** half is separable and needs no new architecture:
`try_add_constraint` ([spade] `cdt.rs:807-817`) already returns the realized
chain. That is why A3 sits in the soundness cut rather than in set C.
