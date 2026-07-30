# IDN-TRANSACTIONAL-INSERT-001 — Identity committed before fallible conversion

**Family** `IDN` · **Manifestation** `MISATTRIBUTION` (asserted from code
reading; no measured witness — see §7) · **Contracts** `TOP-001`, `TOP-002`,
`TOP-007`

**Status** `Closed`. Structural: the class is unrepresentable rather than
absent.

## Obligation

Let $\mathcal{S}$ be source identities, $V$ the converted item vector,
$m$ the map. Required:

$$\forall s \in \operatorname{dom} m:\ V[m(s)] = \operatorname{convert}(s),
\qquad |V| = |\operatorname{dom} m|,$$

with $\operatorname{dom} m$ **exactly** the identities whose conversion
succeeded.

## What the implementation did

```
map.insert(id, items.len());   // commit
let value = convert(id)?;      // may fail — the `?` returns
items.push(value);
```

On failure the entry stayed and nothing was pushed, so every later identity
addressed the item one slot before its own. Both kinds of number were `usize`,
so nothing objected to using an entity id where a position was meant.

## Counterexample / control

Synthetic: *valid A, invalid B, valid C.* C must land at position 1 with
$m(C) = 1$. Pre-fix $m(C) = 2$ addressed nothing. Control: the same sequence
with B valid.

## Measurements

**No behavioural change** — `00009190`: 604 of 24,202 lost, 214,211 triangles,
same 10 blob shells, byte-identical. That is the expected shape of a soundness
change: it removes the possibility, not a current symptom.

`get_checked` — one integer compare on every edge reference a face bound
resolves — fired **zero** times across six models.

## First divergent checkpoint

**C/D.** No earlier stage can see it; no later stage can distinguish it from
correct input.

## Causal derivation

```
position claimed before the conversion that decides whether the item exists
→ one failure desynchronises map and vector
→ every subsequent lookup is off by one
→ a face silently receives a neighbouring edge's curve
→ a smooth, plausible, wrong region, with no error anywhere
```

## Correction

`truck` `52d42086` (edge path), `975fb9f2` (general arena), `714014a4`
(provenance + checked lookup). `truck-stepio/src/in/arena.rs`.

`SourceId<K>` / `Index<K>` tagged by `PhantomData<fn() -> K>`, mutually
unsubstitutable at zero runtime cost. `get_or_try_insert` converts first and
claims a position only on success, so `items.len() == positions.len()` by
construction. `Index` has no public constructor. Items are
`Stored { source_id, value }` so the invariant can be **checked and printed**,
not merely maintained: `get_checked` yields
`TOP-001 failed: requested #714381, but arena index [62] stores #714442`.

> **The type found a live second instance.** `shell_vertices` still had
> reserve-before-convert — it inserted into `vidx_map` then called `get_owned`,
> which can fail. Only the edge path had ever been fixed. It is latent on this
> corpus: no `VERTEX_POINT` conversion fails on `00009190`.

`TRUCK_PROBE_IDENTITY` was **deleted** — the question can no longer have a bad
answer. First diagnostic-to-certificate promotion in the project.

## Tests

`truck-stepio` `arena::tests`, current names: A/B/C ordering; repeated identity
converts and stores once; every mapped identity addresses its own value across
interleaved failures; checked lookup accepts its own identity and refuses
another by name. Not ID-named. The corpus-level assertion is `get_checked`
running in production, which is stronger than a test.

## Known exclusions

**Faces have no compacted identity**, so `TOP-001` for faces is not merely
unchecked — it is *unaskable*. `CompressedFace` is built inline and owns its
surface by value. Unreachable until it gains a `source_id` and a
`SurfaceIndex`. Recorded rather than rounded up: "every arena is generic now" is
the same shape of claim as "every call site has been repaired".

## Claim status

- **(D)** The arena regressions fail on the old ordering, pass on the new; no
  behavioural change on six models; `shell_vertices` had the same defect.
- **(A)** The `MISATTRIBUTION` manifestation. **No file has been observed to
  trigger it.** A latent desync leaves no trace, so the no-change measurement is
  weak evidence about past effect in either direction.

## Links

`truck` `52d42086`, `975fb9f2`, `714014a4` · `truck-stepio/src/in/arena.rs`,
`convert.rs` · [`PLAN.md` § PR 2, PR 2b](../../PLAN.md)
