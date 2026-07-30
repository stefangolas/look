# INC-EDGE-DROP-001 — Failed edge conversion silently shortened a bound

**Family** `INC` · **Manifestation** `EXCESS` (measured: 2,168 triangles from
211 faces trimmed by a region their file never described), `DISTORTION`
(asserted) · **Contracts** `TOP-003`, `TOP-004` discharged; `TOP-005` **not**
addressed

**Status** `Closed`.

## Obligation

A bound is a **loop**, not a sequence. Either every `ORIENTED_EDGE` the source
names resolves, or the bound does not exist:

$$|\{\text{resolved uses}\}| = |\{\text{source uses}\}|,$$

and the chain closes on **vertex identity**, not position:

$$\operatorname{end}(e_i) = \operatorname{start}(e_{i+1}),\quad
\operatorname{end}(e_n) = \operatorname{start}(e_1).$$

## What the implementation did

`face_bound_to_edges` used `filter_map`, so every `?` **dropped that edge** and
returned a shorter wire — a perfectly valid loop-ish object the next stage
consumed happily. Closure, where tested at all, compared coincident *positions*,
which trusts the exporter to have written two coincident `VERTEX_POINT`s as one
entity.

## Counterexample / control

Synthetic, in `wire.rs`: an open chain; a non-joining edge that nonetheless
resolves; a wrongly oriented edge; a single circle (correct — a lone edge bounds
a face only if it is a full loop); the empty wire. The corpus is its own
control: after the fix, **every fully-resolved wire also closes**, on six models.

## Measurements

`00009190`:

| | before | after |
|---|---:|---:|
| faces lost | 393 | **604** |
| — failed to convert | 0 | **274** |
| — meshed to nothing | 166 | **103** |
| triangles | 216,379 | 214,211 |
| blob shells | 10 | 10 (ratios identical to 5 dp) |

274 faces were assembled from incomplete wires. 63 already meshed to nothing;
the other **211 were producing 2,168 triangles** trimmed by a region their file
never described. Adding `TopologicallyClosedWire` on top produced **no
additional rejections anywhere**.

> **The denominator had to be fixed first.** Dropping a face at conversion also
> dropped it from `total`, so the first reading was "330 of 23928" — *better*
> than the 393 baseline while 274 more faces were missing. `FaceTally` now
> carries `declared`, read before conversion. A loss ratio whose denominator
> moves is not a measurement.

272 of the 274 were `VERTEX_LOOP`. This defect made that population **visible**
by refusing it instead of silently truncating it.

## First divergent checkpoint

**D — topology and oriented uses.**

## Causal derivation

```
one ORIENTED_EDGE fails to convert
→ filter_map drops it and yields a shorter wire
→ the wire still looks like a bound to every later stage
→ the face is trimmed by a region its file never described
→ geometry that should not exist is meshed, or a hole fills in,
   or a lost outer bound promotes the holes to the outline
```

## Correction

`truck` `975fb9f2` (`convert.rs`, `wire.rs`); `look` `ac8d7a2` (declared
denominator). Both levels all-or-nothing; empty wires refused; resolved-use
count discharged **by construction** — the collect yields exactly
`edge_list.len()` indices or nothing.

`TopologicallyClosedWire` walks the chain on **vertex identity**, available only
because `IDN-TRANSACTIONAL-INSERT-001` supplies typed edge endpoints.

> **Named for what it proves.** `MATHEMATICAL_FOUNDATION.md` §24 forbids an
> unqualified `ClosedWire`: three distinct closure propositions exist — vertex
> identity (`TOP-004`), metric endpoint agreement (`ARR-001`), closure modulo
> the period lattice (`QUO-002`) — and none implies another. This type
> establishes the first only, and its doc names the two it does not.

## Tests

Nine in `truck-stepio/src/in/wire.rs`, covering the cases in §Counterexample.
Not ID-named.

## Known exclusions

`TOP-005` not addressed: effective orientation is composed from face, bound,
oriented-edge and edge-curve flags, but never checked against source incidence.

The newtype **stops one step short**: `CompressedFace::boundaries` is
`Vec<Vec<CompressedEdgeIndex>>`, so the proof is discharged at face
construction and a second construction site could still supply a non-closing
wire. Under the owned-fork decision this is a defect to close.

**Fixes no blob.** With `GEO-INCIDENCE-ACCEPTANCE-001` it refuses 566 faces on
individually sound grounds and moves the blob count by zero — a measured
exclusion, and what split one assumed defect into two.

## Claim status

- **(D)** 274 faces built from incomplete wires; 211 producing 2,168 triangles;
  no blob change to five decimals.
- **(A)** That those triangles were *wrong* rather than merely unjustified. The
  file does not describe the region they were trimmed by, but no per-triangle
  comparison against an oracle was run — so `DISTORTION` is asserted, not
  demonstrated.

## Links

`truck` `975fb9f2` · `look` `ac8d7a2` · [`PLAN.md` § PR 3](../../PLAN.md) ·
feeds [`INC-VERTEX-LOOP-001`](INC-VERTEX-LOOP-001.md)
