# QUO-LIFT-TIEBREAK-001 — Winding class decided by an arbitrary tie-break

**Family** `QUO` · **Manifestation** `INSTABILITY`, `EXCESS`
**Contracts** `QUO-001`, `QUO-003`

> Added to the index beyond the seeded set: `PLAN.md` names the periodic-lift
> instability as one of the four defects that motivated the architecture, and
> an index omitting it would misrepresent the record.

## 1. Status

```
Closed
```

## 2. Mathematical objects

A boundary sampled in a periodic chart. Successive sample points $q_i$ must be
**lifted** from the quotient $Q = \Omega/\Lambda$ into $\Omega$ by choosing, for
each step, the period copy nearest the previous point:

$$\tilde q_{i+1} = q_{i+1} + k_i P,\qquad
k_i = \arg\min_k \bigl\lVert q_{i+1} + kP - \tilde q_i \bigr\rVert.$$

The lift determines the boundary's **winding class** $(k_u, k_v)$.

## 3. Required obligation

The winding class of a boundary is a **topological invariant of the face**. It
must not depend on the tessellation tolerance, the sampling density, or the
starting vertex of the wire:

$$h(\gamma)\ \text{independent of}\ \tau.$$

## 4. What the implementation did

`get_mindiff` broke the tie **arbitrarily at exactly half a period**. When a
step lands at $\lVert \Delta \rVert = P/2$ the two candidate copies are
equidistant and the choice is whichever the comparison happens to favour — so a
sampling density that placed a point near the half-period boundary flipped the
lift, and **a boundary's winding class depended on the tessellation tolerance.**

## 5. Minimal counterexample

Not extracted as a standalone file. The witness is behavioural: the same face at
two tolerances produced two different lifts, and the blob count moved.

## 6. Control / oracle

The **same face at a different tolerance** — a metamorphic control, and the
strongest available, because a tolerance change must not alter topology.

## 7. Measurements

Part of the sequence on `00009190`: **12 → 4 blob shells** ("after periodic-lift
fix"), the point at which the model first rendered as *a recognizable
submarine*. The largest single visual improvement recorded in the project.

## 8. First divergent checkpoint

**H — periodic lifting and winding.**

## 9. Causal derivation

```
a sample step lands at exactly half a period
→ the tie is broken by comparison order, not by geometry
→ the lift jumps a period copy mid-boundary
→ the boundary's winding class changes with tolerance
→ the lifted loop spans a period it should not
→ the trim region grows to most of the surface
→ an undifferentiated lens blob
```

## 10. Proposed correction

Make the tie-break deterministic and geometry-derived rather than
comparison-ordered.

## 11. Experimental correction

None. Preceded by probes `2a206f4b`, `b5ce8aa0`, `b8703ead` — the
closed/open decision, the period copy each bound normalises into, and net
winding versus total variation per bound — which is how the tie-break was
localised.

## 12. Production correction

`stefangolas/truck` `502a5510` *"Keep the periodic lift stable under
tessellation tolerance"* —
`truck-meshalgo/src/tessellation/triangulation.rs`.

## 13. Regression tests

**None.** The tolerance-invariance property that *is* the obligation is exactly
what `PR 9`'s metamorphic harness exists to assert, and it is not written. This
defect is closed on a measured behavioural improvement and code reading, with no
regression protecting it — a real gap, and the cheapest of the missing tests to
write, since it needs only one face meshed at two tolerances.

## 14. Corpus-wide effect

12 → 4 blob shells on `00009190`. Not separately swept.

## 15. Known exclusions

Stabilising the lift does **not** give the pipeline a winding vector.
`QUO-002` / `QUO-005` remain unaddressed: no $(k_u, k_v)$ is retained anywhere,
and the closure test remains Euclidean in $uv$ with no metric —
see [`QUO-EUCLIDEAN-CLOSURE-001`](QUO-EUCLIDEAN-CLOSURE-001.md).

Each bound is still lifted from `sp(surface, pt, None)`, an **arbitrary
principal value**, so relative offsets *between* the bounds of one face were
never controlled. Measured: two bounds of one face at `quot_v = -1` and `+1`.
That is `PR 6`'s deck-offset solver and it is open.

## 16. Relationship to other defects

The second of the four defects that motivated the architecture. Same mode: an
invalid state (an inconsistently lifted boundary) was representable, and the
next stage consumed it happily.

## 17. Claim status

- **(D)** 12 → 4 blob shells, and the render becoming recognizable.
- **(A)** The half-period tie-break as the mechanism — from code reading and the
  probe series, not from an isolated synthetic reproduction.
- **(U)** Tolerance invariance of the winding class **is not asserted anywhere**
  post-fix. The property is believed, not tested.

## 18. Links

- `truck` `502a5510`; probes `2a206f4b`, `b5ce8aa0`, `b8703ead`
- [`PLAN.md` § Why the architecture changed, § PR 6](../../PLAN.md)
