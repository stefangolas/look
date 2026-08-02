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

| # | Transition | Required output contract | Proved? |
|---|---|---|---|
| T1 | `CompressedEdge.curve` → `PolylineCurve` (`tessellate_edge`) | chord deviation ≤ tol; samples lie on the source curve (GEO-003) | **No.** `from_curve` is trusted; a `len()<=2` result is rewritten by a 16-step fallback with no residual |
| T2 | wire of `PolylineCurve` → `Vec<Point3>` (`try_new`) | traversal order and edge-use orientation agree with source incidence (TOP-005) | **No.** Orientation is applied geometrically (`curve.inverse()`); never composed or checked |
| T3 | `Vec<Point3>` → lifted UV arc (`try_new`: `sp` + `get_mindiff` + refinement) | (a) each point lies on *this* surface (GEO-005); (b) the lift is continuous and a **simple** arc in the cover (FS Def. 7 embedding, Def. 9) | **No — and this is the earliest unproved transition.** See §3 |
| T4 | arcs → closed loops (`PolyBoundary::new`) | closure modulo $\Lambda$ under the surface metric, winding retained (QUO-002) | **No.** Raw UV distance vs hard-coded 1e-3; no first fundamental form; winding discarded |
| T5 | open arcs → stitched loops (`open.len()∈{1,2}`, empty-domain rectangle) | synthesized segments distinguished from source segments (DOM-001, FS §IX) | **No.** Stitched against the *primitive's* declared range (`PAR-RANGE-INHERITANCE-001`), enters as `PhysicalBoundary` |
| T6 | loops → CDT constraints (`insert_to`) | every requested segment represented by a complete constrained chain (CDT-002) | **No.** Returns `bool`; no chain certificate. See §4 for what the Boolean *does* prove |
| T7 | + sampling grid (`insert_surface`) | grid edges carry no material meaning (FS Def. 20) | **Yes, since A1** — `ConstraintRole::SurfaceSampling` |
| T8 | CDT → material cells (parity flood) | $\mu$ satisfies the Def. 20 constraint system; Unique/Ambiguous/Inconsistent trichotomy (Def. 21) | **No.** Odd-even parity with implicit `Empty` base (DOM-003 unimplemented); `Ambiguous` is inexpressible |
| T9 | cells → `PolygonMesh` | approximation level declared (MSH-002); orientation agrees (MSH-003) | **No** |

---

## 3. The earliest unproved transition: T3, the lift

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

A single localized bad step in one loop, deleting the whole face.

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
fully represented*, and ignoring it is correct. But after a split,
`get_edge_from_neighbors(vi, vj)` returns `None`, which is precisely where A1's
role table loses its 213 entries and where chain provenance is unrecoverable
from outside the library.

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

1. **`LiftedBoundaryComplex`** — ambient schema as a first-class object; lift
   certified as a simple arc with $\delta$ and $\tau$ retained; global
   $\psi$ potential solved (`DeckPotentialUnionFind` already exists).
   Typed failures: `Inconsistent(DeckPotentialContradiction)`,
   `Unresolved(AmbiguousLift)`. Absorbs A5, A7, A8.
2. **`NormalizedArrangement`** — intersection classification, atomic
   subdivision, incidence reconstruction, role and provenance aggregation.
   Absorbs A1, A2, A4, A6, ARR-002/003.
3. **`MaterialRegion`** — cell labeling by the FS Def. 20 constraint system,
   yielding Unique/Ambiguous/Inconsistent. Absorbs A3, A9, A10.

**The certified arrangement must own its CDT realization.** Constructing the
triangulation from an already-atomized, crossing-free edge set with a retained
bijection is the only way the proof survives the Spade boundary; "insert, then
look up what happened" is what produced a side table with holes in it.

An open question this audit does not settle: whether a correct T3 removes the
self-crossings, or whether some are genuine transverse intersections of
well-formed boundaries that stage 2 must handle regardless. Stage 2 is required
either way; the answer only changes how much of the 4,048 stage 1 recovers on
its own.
