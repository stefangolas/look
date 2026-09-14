# WORK PACKET FHC-G8 â€” planarity router: exact planar-face flux without generic certification (absorbs FHC-E)

Gap-register 4b: the suspension rows' `_rocker` lightening cut
(`suspension.py:503` â€” a coaxial equal-profile plate through-cut) produces a
planar cap whose normal cone degenerates at the loop centroid; the landed
RDEF-M2 tangential-sandwich admission refuses typed `SingularParametrization`,
and the same geometry drives the certification grind (G6's SPEC_GAP evidence).

**Normative theory:** the owner's **Certified Flux Calculus** Â§8
(Theorems 8.1/8.2) via `docs/CERTIFIED_FLUX_CALCULUS_INTEGRATION_MAP.md`. The
key property: the planarity test is a **coefficient test**
(`Î½Â·A_ij = cÂ·w_ij` for all i,j), parametrization-independent â€” the degenerate
centroid does not exist for this router. Planar faces route to an exact
boundary formula instead of generic 2D certification. This packet **absorbs
FHC-E-FAN-CAP** (its closed-form fan cap is the polynomial-planar special
case; the two suspension rows are this packet's row targets).

```yaml
id:          FHC-G8-PLANARITY-ROUTER
contract:    [FHC-G8-PLANARITY-ROUTER]
class:       mechanical+
crates:      [truck123d]
depends_on:  [FHC-G1-RATIONAL-FLUX]
needs:       [FHC-G1-RATIONAL-FLUX]
write_allow:
  - truck123d/src/bd_bridge.rs
  - truck123d/tests/planarity_router.rs
read_allow:
  - docs/F1_HYPERCAR_GAP_REGISTER.md
  - docs/CERTIFIED_FLUX_CALCULUS_INTEGRATION_MAP.md
tests_required: [truck123d/tests/planarity_router.rs]
anchors:
  - {id: A1, expect: 11, cmd: "grep -c 'spline_loop_area_vector' truck123d/src/bd_bridge.rs"}
  - {id: A2, expect: 7, cmd: "grep -c 'cell_flux_exact' truck123d/src/bd_bridge.rs"}
  - {id: A3, expect: 3, cmd: "grep -c 'binding_volume_facts' truck123d/src/bd_bridge.rs"}
budget:      {turns: 55, ctx_tokens: 200000}
```

Anchors measured 2026-09-13 at HEAD (A3 counts G1's landing). **Re-measure at
dispatch** â€” update stale counts, never dispatch stale.

## Pre-made judgements

1. **Reuse the landed end-cap pattern.** The loft arm already computes exact
   planar cap terms `(1/3)Â·dÂ·A` via `spline_loop_area_vector`
   (`bd_bridge.rs:2436-2457`). This packet generalizes that into a router:
   planarity detection â†’ boundary area-vector route, replacing generic
   per-patch certification for planar faces.
2. **The planarity detector is exact and cheap**: verify `Î½Â·A_ij = cÂ·w_ij`
   over the homogeneous control net (Thm 8.1) â€” a coefficient scan, no
   normal-cone analysis, no parametrization introspection. Refusal if no
   plane fits is typed and routes to the generic path (G1's), never to a
   guess.
3. **Polynomial planar boundary â†’ exact rational flux** (Thm 8.2 boundary
   form `(câˆ’Î½Â·a)/(3(Î½Â·Î½)) Â· Î½Â·Aâƒ—` with `Aâƒ— = (1/2)âˆ®XÃ—dX`); **rational planar
   boundary â†’ 1D `k=2` reciprocal-power certificates** via G1's kernel.
   The exactness distinction matters: a rational parametrization of a
   circular sector has area containing Ï€ â€” never force it through the exact
   integer path.
4. **Routing order**: planarity runs BEFORE generic 2D certification on every
   admitted patch. A planar hit replaces the 2D `k=3` problem with boundary
   1D work; a miss falls through unchanged. Deterministic order preserved.
5. Out of scope: non-planar re-parametrization, trimmed faces (G9),
   approximate trims.

## Method

Coefficient-plane detector â†’ boundary area-vector route (polynomial exact;
rational via the 1D `k=2` hook into G1's kernel) â†’ wire into the per-patch
certification site ahead of the generic path â†’ the two suspension rows'
lightening cuts become the end-to-end door spot-checks (bounded runs; the
through-cut cap is the regression case) â†’ `f1/suspension_front` and
`f1/suspension_rear` verdicts in RESULT.

## Tests required (`truck123d/tests/planarity_router.rs`)

1. `coefficient_plane_detector` â€” planar patches (polynomial and rational)
   detected exactly; tilted/near-planar non-planar patches refused to the
   generic path; the test plane need not be axis-aligned.
2. `planar_polynomial_exact` â€” a planar polynomial face's boundary-form flux
   equals the generic exact path bit-for-bit on a case where both terminate.
3. `planar_rational_k2_route` â€” a rational planar face (quarter-disk class)
   routes to the 1D `k=2` certificate and converges to the known
   Ï€-dependent value (the regression test preventing
   "planar rational â‡’ rational-valued exact").
4. `fan_cap_through_cut` â€” the suspension `_rocker` through-cut cap
   (equal-profile coaxial plate subtraction) certifies exactly via the
   planar route: no `SingularParametrization`, byte-identical STL
   fingerprint before/after for the whole row.
5. `non_planar_falls_through` â€” a genuinely curved face produces results
   identical to the pre-router path (router adds nothing, changes nothing).

## Done when

```
cargo fmt --check -p truck123d
cargo clippy -p truck123d --all-targets -- -D warnings
cargo test -p truck123d --test planarity_router --locked
```

plus bounded door spot-checks: `f1/suspension_front` and `f1/suspension_rear`
complete with the cap routed (green bracket + STL, byte-identical where the
row previously completed). All cargo through the queue.

## Forbidden

Any file outside `write_allow`; editing `corpus/ttc/**` or `vendor/truck/**`;
approximation of the boundary integral; tolerance changes; `#[ignore]`,
deleted or weakened tests, bare `cargo test`; committing to `main`.

## Stop conditions

- the coefficient plane test cannot be made exact within the funnel's
  arithmetic â†’ `SPEC_GAP` naming the gap
- a routed planar face disagrees with the generic path on a
  doubly-computable case â†’ that is a BUG: fix, never widen
- anchor count differs at dispatch and was not re-measured â†’ `ANCHOR_MISMATCH`
- three consecutive failed cargo runs on the same error â†’ `BLOCKED`

## Finish by writing `RESULT.json` AT THE WORKTREE ROOT

```json
{"id":"FHC-G8-PLANARITY-ROUTER","status":"DONE","contracts":["FHC-G8-PLANARITY-ROUTER"],
 "anchors_verified":{"A1":11,"A2":6,"A3":1},
 "rows_flipped":["f1/suspension_front","f1/suspension_rear"],
 "notes":"router verdicts; fan-cap regression; row before/after"}
```

Commit subject: `truck123d: planarity router - exact planar-face flux, fan cap absorbed (FHC-G8)`.
