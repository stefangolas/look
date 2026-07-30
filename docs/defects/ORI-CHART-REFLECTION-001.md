# ORI-CHART-REFLECTION-001 — Signed UV area used as chart-invariant

**Family** `ORI` · **Manifestation** `INVERSION`, `EXCESS`
**Contracts** `DOM-001`, `DOM-002`

## 1. Status

```
Closed
```

## 2. Mathematical objects

A trimmed face $F = (S, \Gamma, o)$ on a chart $S : \Omega \to \mathbb{R}^3$.
The **material region** $M \subseteq \Omega$ is the subset that gets meshed. A
**chart reflection** is a reparameterization $\phi$ with
$\det D\phi < 0$ — e.g. $(u,v) \mapsto (u, -v)$ — which changes no point of the
represented solid.

## 3. Required obligation

$M$ is a property of the face, so it must be **equivariant** under
reparameterization:

$$M(\phi \circ \Gamma) = \phi^{-1}\bigl(M(\Gamma)\bigr).$$

Any predicate used to decide it must therefore be chart-invariant. Signed area
is not:

$$A(\phi \circ \gamma) = -A(\gamma)\quad\text{for } \det D\phi < 0.$$

## 4. What the implementation did

Inferred the material side from the **sign of the signed UV area** of the
boundary loops. Under a reflected chart the sign flips while the face is
unchanged, so *the same solid meshed differently depending on which way the
importer happened to parameterize its surface.*

## 5. Minimal counterexample

Synthetic, in `truck-meshalgo/tests/tessellation/trimming_domain.rs`: one face,
two charts related by a reflection. Pre-fix the two produce complementary
regions; the correct answer is one region and its image.

## 6. Control / oracle

The unreflected chart of the same face. The oracle is **the invariance itself**,
which is why this is a metamorphic defect and needs no external reference.

## 7. Measurements

Part of the sequence that took `00009190` from 70 blob shells to 12
("after containment fix"). Attribution to this change alone is from that
sweep's ordering, not from an isolated A/B.

## 8. First divergent checkpoint

**I — material-domain construction.** Everything upstream is chart-independent
or correctly equivariant.

## 9. Causal derivation

```
material side inferred from sign(signed UV area)
→ a reflected chart flips the sign with the face unchanged
→ the complement is meshed instead of the region
→ an inverted face, or the whole surface where a small patch belonged
→ a smooth plausible blob, with no error raised
```

## 10. Proposed correction

Decide the domain by **containment**, a chart-invariant predicate, rather than
by the sign of an area.

## 11. Experimental correction

None.

## 12. Production correction

`stefangolas/truck` `323c5a87` *"Decide a face's domain by containment, not by
the sign of an area"* — `truck-meshalgo/src/tessellation/triangulation.rs`,
`truck-geotrait/src/algo/curve.rs`.

## 13. Regression tests

`truck-meshalgo/tests/tessellation/trimming_domain.rs`, added by the same
commit — **the two tests named in `PLAN.md` as the template for the whole
metamorphic harness (PR 9).** Not yet ID-named.

> They have not been run in this environment. `truck-meshalgo`'s suite cannot
> build here: it needs `resources/shape/bottle.json`, which the local fork
> clone does not have. Recorded as an environmental gap, not a passing claim.

## 14. Corpus-wide effect

Included in the 70 → 12 blob-shell reduction; not separately isolated.

## 15. Known exclusions

Containment answers *which* side, given a decidable boundary. It does **not**
answer *whether the starting region was material* —
`closed.is_empty()` remains the gate, and parity answers "how many boundaries
were crossed", not "was the base region material". That is `DOM-003`
(explicit base domain), still open, and it is the bill that
[`DOM-ARTIFICIAL-CLOSURE-001`](DOM-ARTIFICIAL-CLOSURE-001.md) is presenting.

## 16. Relationship to other defects

One of the four defects that, arriving in a single session, motivated moving
the ingestion layer to fallible constructors with evidence-carrying output
types. All four share one mode: **an invalid state was representable and the
next stage consumed it happily.**

## 17. Claim status

- **(D)** $A(\phi\circ\gamma) = -A(\gamma)$ — arithmetic, and the regression
  test exercises it.
- **(A)** Its share of the 70 → 12 blob reduction, from sweep ordering rather
  than an isolated A/B.

## 18. Links

- `truck` `323c5a87`
- [`PLAN.md` § Why the architecture changed](../../PLAN.md)
- `../look-trimming-residual` — formal treatment of the invariance fix
