# Refinement audit: source face bounds → Spade insertion

**Scope:** the path from `CompressedFace.boundaries` to constrained-triangulation
realization, in `truck-meshalgo/src/tessellation/triangulation.rs`.
**Method:** for each transformation, state the formal input contract, the formal
output contract, the implementation representation, and whether the code
*proves* the transition. Guided by `FORMAL_SYSTEM.md` and the contract registry
in `MATHEMATICAL_FOUNDATION.md`.
**Audited artifact:** truck-fork `628f39e7` (`audit/a1-constraint-roles`),
look `7f315c7`. See `formal_baseline_manifest.json`.

A correctness audit of source that is not the executed source is worth nothing.
This project has already produced one such document: manifest v1.0 recorded a
truck rev the build never had, because a live `paths` override in
`.cargo/config.toml` silently redirected nine crates. **Provenance is a
precondition of this audit, not a footnote.** It is discharged above.

---

## 1. Refinement map

For each formal object, its faithful implementation representation — or the
absence of one. An object with no representation is an architectural gap, not a
missing check.

| Formal object | Source | Current representation | Verdict |
|---|---|---|---|
| Ambient schema $(\Omega,\Lambda,N,\Sigma,S,C)$ | FS Def. 7 | three unrelated accessors: `u_period()`, `v_period()`, `try_range_tuple()`; no certificate binding them | **absent.** `domain/schema.rs::ParametricQuotient` is the faithful type and is dead code |
| Deck lattice $\Lambda = L\mathbb{Z}^r$ | FS Def. 7 | two `Option<f64>` periods | **partial.** Rank is implicit; no generator, no validity certificate (QUO-001 unchecked) |
| Collapsed stratum $\sigma$ + link $\lambda_\sigma$ | FS Def. 7, Def. 24 | ad-hoc `uder().so_small()` probes at point of use | **absent.** `SingularStratum` / `StratumCertificate` dead |
| Admissible normalized arc $a=(\gamma,p,q,\delta,\tau,\ell,\pi)$ | FS Def. 9 | `Vec<SurfacePoint>` — $\gamma$ only | **absent.** No endpoint descriptor, no $\delta$, no traversal semantics, no orientation, no provenance |
| Deck displacement $\delta$ | FS Def. 9 | computed as `BoundaryClosure::PeriodicClosed{displacement}`, then dropped | **computed and discarded** (QUO-002: "a Boolean `closed` is insufficient" — not even that survives) |
| Lift potential $\psi(v)$, cycle consistency | FS §VII | — | **absent.** `domain/deck.rs::DeckPotentialUnionFind` is the exact solver and is never called |
| Role-labeled half-edge, $\text{kind}(h)$ | FS §IX | `ConstraintRole` side table keyed on Spade handles (audit A1) | **partial.** Loses entries wherever Spade realizes a constraint as a chain — 213 measured |
| Normalized arrangement edge | FS Def. 18 | raw consecutive point pair; no splitting stage exists | **absent** |
| Arrangement vertex | FS §IX | Spade vertex welded at UV distance² < 1e-12, linear scan | **partial**, absolute and unit-dependent |
| Quotient identification $\sim_\Lambda$ vs $\sim_\Sigma$ | FS §IX | not distinguished | **absent.** FS explicitly forbids merging them into one proximity weld |
| Material cell variable $\mu_c$ | FS Def. 19 | CDT face + parity bit from BFS flood | **partial**, and parity ≠ the Def. 20 constraint system |
| Constraint witness | MF CDT-002 | `bool` | **absent** |
| Region complex $R=(A,G,\mu)$ | FS Def. 26 | — | **absent.** Collapses directly to `PolygonMesh` |

**Structural conclusion.** Four of FORMAL_SYSTEM §IV's sorts — ambient term,
boundary term, arrangement complex $G$, region term $R$ — are all represented by
one type, `PolyBoundary(Vec<Vec<SurfacePoint>>)`. This is why every finding in
this audit lands in one file and mostly one function: there is nowhere else for
a finding to live. A1's side table was not a design choice; it is what one does
when there is no half-edge type to label.

---

## 2. Transition audit

| Stage | Formal input | Formal output | Implementation constructor | Obligation | Status |
|---|---|---|---|---|---|
| **Ambient construction** | surface syntax and geometry | certified ambient schema $(\Omega,\Lambda,N,\Sigma,S,C)$ | none — scattered `u_period()` / `v_period()` / `try_range_tuple()` at points of use | lattice / domain / strata consistency; period validity (QUO-001) | **absent** |
| Edge sampling | `CompressedEdge.curve` | chord-bounded polyline with provenance | `tessellate_edge` | GEO-003 sampling fidelity | absent — `from_curve` trusted; a `len()<=2` result is rewritten by a 16-step fallback with no residual |
| Wire assembly | ambient + source edge uses | oriented 3D walk | `try_new` (flat_map) | TOP-005 effective orientation vs source incidence | absent — orientation applied geometrically (`curve.inverse()`), never composed or checked |
| **Boundary lifting** | ambient + source walks | deck-coherent lifted walks | `PolyBoundaryPiece::try_new` | global $\psi$ potential, cycle consistency, arc simplicity (FS Def. 7 embedding, Def. 9) | **violated** — see §3 |
| Loop closure | lifted arcs | closed quotient loops, winding retained | `PolyBoundary::new` | QUO-002 closure under the surface metric | violated — raw UV distance vs hard-coded 1e-3, no first fundamental form, winding discarded |
| Synthetic closure | open arcs + ambient | closures distinguished from source evidence | `open.len()∈{1,2}`, empty-domain rectangle | DOM-001, FS §IX edge kinds | violated — stitched against the *primitive's* declared range (`PAR-RANGE-INHERITANCE-001`); enters as `PhysicalBoundary` |
| **Arrangement** | lifted walks | atomic role-labeled complex | none | intersection / overlap normalization (ARR-002, ARR-003) | **absent** |
| **Material solve** | ambient + arrangement | Unique / Ambiguous / Inconsistent region | dual-parity BFS flood | FS Def. 20 constraints, Def. 21 trichotomy | restricted / incorrect — odd-even with implicit `Empty` base (DOM-003); `Ambiguous` inexpressible |
| **CDT realization** | certified arrangement | realization bijection | direct Spade mutation via `insert_to` | preserve vertices, edges, roles (CDT-001, CDT-002) | **absent** — returns `bool`; see §4 |
| Mesh realization | material region + CDT | surface mesh | `triangulation_into_polymesh_outcome` | selected cells exactly realized; MSH-002, MSH-003 | partial |

Sampling-grid insertion is the one transition that now discharges its
obligation: since A1, `ConstraintRole::SurfaceSampling` establishes that grid
edges carry no material meaning (FS Def. 20).

**Correction to an earlier draft of this document.** It named the lift as the
earliest unproved transition. That is wrong by one stage: the lift consumes
period, domain and stratum facts that no constructor establishes, so **ambient
construction is earlier, and its status is worse — not "unproved" but
"absent."** `domain/schema.rs::ParametricQuotient` and
`domain/deck.rs::DeckPotentialUnionFind` are therefore not stray unused
abstractions; they are partial implementations of exactly the first two missing
semantic layers.

---

## 3. The first *violated* transition: the lift

**Contract required.** FS Def. 7 requires the induced map on the regular set to
be an **embedding** over the certified face neighborhood, with every injectivity
failure accounted for by a deck translation or a declared collapse. FS Def. 9
requires each arc to be a continuous lifted curve carrying its deck displacement
$\delta$ and traversal semantics $\tau$.

**What the code does.** `sp(surface, pt, previous)` returns a parameter with no
residual obligation (the gate is `f64::INFINITY` by default).
`get_mindiff(u, u0, up) = u + round((u0-u)/up)*up` selects the period copy
nearest the previous sample — correct only while the true step is under half a
period. `AMBIGUOUS_STEP_FRACTION = 0.45` guards the tie by bisecting the 3D
chord, up to `MAX_LIFT_REFINEMENTS = 8`; **on exhaustion the ambiguous step is
accepted silently**, with no record that it was ambiguous. The code's own
comment describes the failure: "measured advancing `-0.5` of a period where the
curve went `+0.5`, which folds a full turn onto itself."

Nothing checks that the resulting lifted arc is simple.

**Measured consequence** (ABC `00009190`, audited build, existing probe output
only, no new instrumentation):

- `ConstraintInsertionIncomplete` = **4,048 faces**, 84% of all loss.
- Of 3,764 first-conflicts reported on boundary piece 0, **3,763 conflict with an
  edge from piece 0 itself.** The loop crosses *itself* — this is not two bounds
  landing in different deck copies.
- **96%** of refused segments cross exactly **one** constraint edge; **91%** of
  failing faces have exactly **one** refusal. The typical failure is a single
  isolated self-crossing, not tangled geometry.
- Periodic rank of failing faces: 3,158 rank-1, 554 rank-2, 331 rank-0. **92%
  have a periodic axis**, which is where `get_mindiff` can fold a loop.
- Conflict provenance resolves directly in 4,038 of 4,043 cases.

**What this proves, and what it does not.** It is a *negative* result about one
mechanism, and negative results here are weaker than positive ones. It
establishes that **independent whole-bound centroid normalization does not
explain these witnesses** — the conflicting segments belong to the same bound,
so they were shifted by the same $(k_u,k_v)$ and no lattice translation between
bounds can separate them.

It does **not** establish that these are genuine transverse intersections of
well-formed boundaries. Every one of the following remains consistent with the
evidence, and they are not distinguished:

- an incorrect lift *within* one bound (`get_mindiff` folding a step);
- discarded winding, so a period-wrapping loop reads as closed;
- a projection branch or sheet jump inside `sp`;
- synthetic closure segments crossing source segments in the same piece;
- vertex welding at 1e-12 manufacturing a T-junction;
- singular-coordinate collapse at a pole or apex.

The correlation with periodic rank (92%) and the single-crossing profile are
*suggestive* of a localized lift fold, which is why the lift is named as the
first violated transition. They are not a proof of it, and this document does
not claim one. A constructive witness — one affected face where applying the
coherent lift removes the crossing — would settle it, and only stage 1 below
can produce that witness.

---

## 4. What `insert_to() -> bool` actually proves

Read from Spade 2.15.0 source, not assumed:

```rust
pub fn can_add_constraint(&self, from, to) -> bool {
    let it = LineIntersectionIterator::new_from_handles(self, from, to);
    !self.contains_any_constraint_edge(it)   // matches only edge.is_constraint_edge()
}
```

`can_add_constraint == false` ⟺ **the segment properly crosses an existing
constraint edge.** `insert_to` runs before `insert_surface`, so the only
constraint edges present are earlier segments of the same face's own boundary.

```rust
pub fn add_constraint(&mut self, from, to) -> bool {
    let initial = self.num_constraints();
    self.resolve_splitting_constraint_request(from, to, None);
    self.num_constraints != initial
}
```

It **splits**, and returns whether the count changed — so `false` means *already
fully represented*, and ignoring it is correct. After a split,
`get_edge_from_neighbors(vi, vj)` returns `None`, which is precisely where A1's
role table loses its 213 entries.

**Qualification.** An earlier draft called this unrecoverable from outside the
library. That is too strong. It is unrecoverable under the *current* design —
request a long segment, then rediscover what Spade created. It becomes
avoidable once `NormalizedArrangement` owns the atomic subdivision: if every
arrangement edge already terminates at proper intersections, T-junctions,
overlap endpoints and existing collinear vertices, then atomic edges are
inserted individually and the bijection is retainable. If Spade still
subdivides an allegedly atomic edge, that reveals either that the arrangement
was not atomic relative to the CDT vertex set, or that Spade's public model
cannot preserve the realization bijection. **Only the second would be an
argument for vendoring or modifying Spade**, and nothing measured so far
supports it.

**Verdict.** The Boolean is a *faithful* certificate that some requested
constraint is unrepresentable as stated — it is **not** overstrict in the
"a valid chain already exists" sense (only 5 faces). H1 is refuted; H2 holds,
with the mechanism localized to T3 rather than to inter-bound deck placement.

Two consequences. First, the historical `79eaaf36` behaviour — discarding the
Boolean and proceeding — was *unsound*, not merely lenient: it flood-filled
across a boundary it had failed to represent. Its 23,806 rendered faces include
an unknown number of silently wrong ones. Second, the delta's detector is right
and its **policy** is wrong: `RejectFace` with no repair path is the
detection/policy conflation of MF §31.

---

## 5. What follows

The repair is not "continue despite failure" and not "special-case the
insertion." It is the missing certified stage, and it must be built in this
order, because building it in any other order certifies the wrong segments:

1. **`CertifiedParametricAmbient`** — lattice, periods, admissible domain,
   native boundaries, collapsed strata, singular links, surface evaluation,
   each with a certificate. Even the deck solve depends on these facts, which
   is why this is first. `domain/schema.rs::ParametricQuotient` is the partial
   implementation. Typed failure: `Unsupported(SchemaFailure)`.
2. **`LiftedBoundaryComplex`** — lift certified as a simple arc with $\delta$
   and $\tau$ retained; global $\psi$ potential solved
   (`domain/deck.rs::DeckPotentialUnionFind` already exists). Typed failures:
   `Inconsistent(DeckPotentialContradiction)`, `Unresolved(AmbiguousLift)`.
   Absorbs A5, A7, A8.
3. **`NormalizedArrangement`** — intersection and overlap classification,
   atomic subdivision, incidence reconstruction, role and provenance
   aggregation. Absorbs A1, A2, A4, A6, ARR-002/003.
4. **`MaterialRegion`** — cell labeling by the FS Def. 20 constraint system,
   yielding Unique / Ambiguous / Inconsistent. Absorbs A3, A9, A10.
5. **`CdtRealization`** — the foreign `Cdt` never becomes the semantic object;
   it stays contained inside a proof-carrying realization owned by this code:

```rust
struct CdtRealization {
    cdt: Cdt,
    vertex_map: HashMap<ArrangementVertexId, FixedVertexHandle>,
    edge_map: HashMap<ArrangementEdgeId, FixedUndirectedEdgeHandle>,
}
```

Its constructor succeeds only if every arrangement vertex and every atomic edge
has an exact realization. "Insert, then look up what happened" is what produced
a side table with holes in it.

**What this audit does not settle.** Whether a certified lift removes the
self-crossings, or whether some survive as genuine transverse intersections.
Stage 3 is required either way; the answer only changes how much of the 4,048
stages 1–2 recover on their own. It is answerable only *after* stage 2 exists,
as a constructive witness rather than another population study.

**Which line to build from.** Not whichever revision renders the most faces —
`79eaaf36`'s 23,806 is not a correctness oracle, since §4 shows it flood-fills
across boundaries it failed to represent. Build from the reviewed semantic line
containing A1 and excluding known-invalid experiments.
