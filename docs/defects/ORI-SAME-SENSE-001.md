# ORI-SAME-SENSE-001 — Face-local sense applied to shared surface state

**Family** `ORI` · **Manifestation** `INVERSION` *(asserted — no witness)*
**Contracts** `TOP-001`, `TOP-002` extended to surfaces

**Status** `Closed`. Designed out, not observed. Short record because there is
little to record: no corpus file has been shown to exercise it.

## Obligation

`same_sense` is a property of the **use**, not the entity — two faces may
reference one `CYLINDRICAL_SURFACE` and disagree. A canonical converted entity
is shared and immutable; a use-specific transform applies to the copy the use
takes:

$$\operatorname{geom}(f_i) = \sigma_i\bigl(\operatorname{canon}(s)\bigr),
\qquad \operatorname{canon}(s)\ \text{unaffected by any } \sigma_i .$$

## What the implementation did

Converted a surface **once per face**, so the question could not arise. When
surfaces moved into the arena — one conversion per source entity — an in-place
`invert()` would have mutated the shared canonical entity, making the second
face's orientation depend on the first face's flag and on visitation order.

## Counterexample / control

Two `ADVANCED_FACE`s referencing one surface with opposite `same_sense`.
**Not extracted as a file, and not found in either corpus** — the count has
never been run. Control: two faces referencing two distinct surface entities of
the same geometry, which is what most exporters emit and why this stayed
invisible.

## Measurement

**No behavioural change.** `00009190`: 604 of 24,202 lost, 214,211 triangles,
same 10 blob shells, identical to the digit. `00000730`: 425,328 triangles.
Six models, no `TOP-001` fires.

## Causal derivation (asserted)

```
one surface entity converted once and shared
→ same_sense inversion applied in place to the canonical entity
→ the second face inherits the first face's inversion
→ its normals point into the solid
```

## Correction

`truck` `975fb9f2`, `714014a4` — inversion applies to the copy the face takes.
`FaceProvenance` keeps `use_id` / `definition_id` / `surface_id` apart, because
an `ORIENTED_FACE` use is not its `FACE_SURFACE` definition.

## Tests

**None specific to this defect.** The arena tests cover shared-entity
conversion but not disagreeing sense.
`ori_same_sense_001_two_faces_disagreeing_on_one_surface` against a synthetic
two-face shell is the missing test and is cheap.

## Known exclusions

Distinct from `surface.invert()` breaking curve-on-surface **incidence**, which
is a different obligation and remains a diagnostic behind `TRUCK_NO_INVERT`.
Disabling `invert()` accounted for blob shell `160039` but **not** `160144` or
`160014` — a diagnostic finding, not a fix, and it has no defect ID because the
obligation it violates has not been pinned down.

## Claim status

- **(D)** Surfaces are converted once per source entity; no corpus behaviour
  changed.
- **(A)** The `INVERSION` manifestation — from `same_sense` semantics and code
  reading only.
- **(U)** Whether any file in either corpus shares a surface across faces with
  disagreeing sense. **Never counted; it is a one-line census query.**

## Links

`truck` `975fb9f2`, `714014a4` · [`PLAN.md` § PR 2b item 4](../../PLAN.md) ·
`MATHEMATICAL_FOUNDATION.md` §51a
