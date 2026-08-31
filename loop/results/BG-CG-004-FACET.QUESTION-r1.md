# QUESTION.md — BG-CG-004-FACET (status: SPEC_GAP)

## What I attempted

I verified every anchor (A1–A7 all match the expected counts, including the
required A2 = 0 and A5 = 0), read the frozen conventions in
`truck-geometry/src/constructive/mod.rs` (index-identity convention,
certificate mapping, `DirectTolerance`), and read the landed evaluator chain
(`recipe.rs`, `profile.rs`, `errors.rs`, `sampling.rs`, the frame laws,
`truck-polymesh`'s `PolygonMesh`/`Faces` API) plus the program plan. I then
checked whether the packet's required test fixtures can actually be realized
by the landed evaluators, and confirmed the finding empirically with a scratch
build against the landed `truck-geometry` crate.

## The exact conflict

The packet mandates in its own body step 2 that the grid is emitted as

```rust
x_{i,j} = recipe.position(s_i, v_j)?
```

The landed `SpineFrameRecipe::position`
(`truck-geometry/src/constructive/recipe.rs:196`) realizes

```rust
X(s, v) = C(s) + tangent(s) * p.x + normal(s) * p.y
```

i.e. the profile is embedded in the plane spanned by the **spine tangent** and
the **frame normal**. For the packet's booked straight fixture — the
`LineSpine` of length 2 — `tangent` and `normal` are constant, so **every**
grid position lies in one affine plane and the closed mesh's signed volume is
**exactly 0.0** (verified empirically: max |z| over all 20 grid positions = 0,
V = 0.0). The same holds for the planar two-quarter-circle fixture
(verified: V = 0.0; the side cells do split, 16/16, so the fixed-diagonal
split mechanics are unaffected).

Two of the ten required tests are therefore unsatisfiable:

- **Test 1** (`straight_duct_closes_with_exact_shared_indices`) asserts
  `verdict == CertifiedWithinTolerance`. With V = 0.0, `verdict_of` returns
  `Inconclusive` (0.0 <= extent³ / 1_000_000_000.0 always). The
  `CertifiedWithinTolerance` verdict is **unreachable** for every planar or
  straight fixture.
- **Test 8** (`signed_volume_matches_analytic_box`) asserts
  `|V − 2.0|` within a tolerance-scaled bound, expecting the straight square
  duct to reproduce the analytic prism volume `area × length = 1 × 2 = 2.0`.
  V = 0.0; the assertion cannot hold.

Both tests implicitly assume the profile is the **perpendicular cross-section**
of the duct, i.e. embedded in the frame's (normal, binormal) plane. The landed
evaluator embeds the profile in the (tangent, normal) plane instead, so a
straight-spine realization is always coplanar with zero enclosed volume.

## Readings I could not choose between

- **Reading A — the landed evaluator diverges from the design.** The design's
  "profile in the frame plane" means the (normal, binormal) plane (profile =
  cross-section, perpendicular to the spine). Under this reading the volume /
  `CertifiedWithinTolerance` tests are correct as written, and the gap is in
  the landed `recipe.position` axis mapping (`tangent * p.x + normal * p.y`
  instead of `normal * p.x + binormal * p.y`). Resolution requires a change in
  `truck-geometry/src/constructive/**`, which this packet is forbidden to
  touch (`read_allow` only; the "Forbidden" list names that directory
  explicitly).
- **Reading B — the tests are wrong as written.** The (tangent, normal)
  embedding is the intended, landed semantics (CG-001 tested it positively).
  Under this reading the straight/planar realizations are correctly flat, the
  analytic volume of the "duct" is 0.0, and tests 1 and 8's expected values
  (CertifiedWithinTolerance, 2.0) are incorrect and must be re-specified.

Either way the packet as written cannot be satisfied without either editing
the landed constructive evaluator (forbidden here) or re-specifying tests 1
and 8. I stopped here rather than fudge the required tests or silently change
the geometry, per the packet's SPEC_GAP stop condition.

No files were edited; the working tree is unchanged apart from the untracked
`PACKET.md` / `CONTEXT.md` that were present at dispatch.
