# DOM-ARTIFICIAL-CLOSURE-001 — Lone open trim closed against a parameter-range edge

**Family** `DOM` · **Manifestation** `OMISSION` (measured on the reproducer)
**Contracts** `DOM-003`, `ARR-002`, `ARR-003`

> Full record:
> [`../look-collapsed-boundary/FORMALISM.md`](../../../look-collapsed-boundary/FORMALISM.md)
> §3.3, §4 step 3, §7.

```
Status: Mechanism established
```

## Obligation

The trimming boundary of a face is **what its source describes**. A domain may
be completed only from entities the file states — bounds, declared periodicity,
an explicit base domain — never from an implementation artifact. Formally, every
segment of $\partial M$ must have a source antecedent:

$$\forall s \subseteq \partial M,\ \exists\ \text{a source entity } e \text{ with } s \in \operatorname{desc}(e).$$

## What the implementation did

`PolyBoundary::new` branches on the count of pieces classified open:

- $|\text{open}| = 1$: the piece is closed against **one edge of the declared
  parameter rectangle**, requiring both ranges finite; otherwise the piece is
  *silently dropped*;
- $|\text{open}| = 2$: the two are stitched to each other;
- if `closed` is then empty and the range is finite, **the whole rectangle
  becomes the domain**.

The appended path $q \to (u_0, v_1) \to (u_0, v_0) \to p$ has no source
antecedent. Its position is decided entirely by
[`PAR-RANGE-INHERITANCE-001`](PAR-RANGE-INHERITANCE-001.md)'s inherited range.

## Counterexample / control

`repro/apex_only.stp` (0 triangles). Control: any **cylindrical** face, which
has two circular bounds, takes the $|\text{open}| = 2$ branch, and is stitched
to real source geometry — so the same defective range produces no failure.
**(A)** — code path, not separately measured.

## Measurement

```
PROBE in_closed=0 in_open=1 loops=1 areas=[+0.0000e0] range=true rect=false
```

One open piece, one constructed loop, from a face whose source describes exactly
one real boundary and one collapsed one.

## First divergent checkpoint

**I — material-domain construction.**

## Causal derivation

Arrows 4–5 of the chain in `PAR-RANGE-INHERITANCE-001`. The dangerous property
is **generality**: this branch fabricates boundary for *any* surface with one
open piece and a finite declared range. The cone is where it was caught, not
where it is confined.

The third branch is worse in kind — an empty `closed` set plus a finite range
emits the **entire rectangle**, which is the untrimmed-surface blob signature.

## Correction

**Proposed**: `DOM-003`'s explicit base domain. Classify material region by
`base XOR parity` with `BaseDomain { Empty, NaturalRange, PeriodicQuotient }`
and `BoundRole { Outer, Inner }` as `Known`/`Unknown`, rather than inferring
from loop count. Preserve `FACE_OUTER_BOUND` vs `FACE_BOUND` through
`truck-stepio`, which currently parses both into the same `FaceBound` struct and
**discards the outer/hole role the file states explicitly**.

`PLAN.md`'s PR 5 note already recorded that `closed.is_empty()` cannot be the
final rule — parity answers "how many boundaries were crossed", not "was the
starting region material". This defect is that bill arriving.

**Forbidden**: stitching to the declared rectangle when the boundary lies on its
edge.

**Experimental / production**: none.

## Tests

None. The `AllBoundsCollapsed` refusal — a face whose bounds *all* collapse is
rejected rather than emitting the whole unbounded surface — is the nearest
existing safeguard and **must be preserved**: it is why
[`INC-VERTEX-LOOP-001`](INC-VERTEX-LOOP-001.md) added no blobs.

## Known exclusions

Not the same obligation as [`DOM-ZERO-AREA-001`](DOM-ZERO-AREA-001.md).
Artificial closure is wrong even when the resulting area is nonzero — it is a
provenance violation. The zero area is the particular consequence *here*,
because the range edge happens to coincide with the circle.

## Claim status

- **(D)** One open piece, one constructed loop, on the reproducer.
- **(A)** The branch structure and the silent-drop path — read from
  `triangulation.rs`, not measured.
- **(U)** Whether the whole-rectangle branch fires anywhere in either corpus.
  **Never counted, and it is a probe worth adding**, because that branch is the
  untrimmed-surface signature and would be a blob source independent of
  everything else in this chain.

## Links

[`FORMALISM.md`](../../../look-collapsed-boundary/FORMALISM.md) §3.3, §4, §7 ·
`truck-meshalgo/src/tessellation/triangulation.rs` ·
[`PLAN.md` § PR 5](../../PLAN.md)
