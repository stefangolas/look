# WORK PACKET FHC-G9 â€” Green-reduction trim integration: faces with holes, no trimmed-interior meshing

The funnel's patches are tensor-Bernstein on rectangular domains; a face with
holes (trimmed surface) is inexpressible today. This packet lands the
proposal's Â§9: **never represent the trimmed interior to integrate it** â€”
reduce to the oriented trim boundary, exactly where possible.

**Normative theory:** the owner's **Certified Flux Calculus** Â§9 (Theorems
9.1/9.3/9.5, Lemma 9.2, Corollary 9.4) via
`docs/CERTIFIED_FLUX_CALCULUS_INTEGRATION_MAP.md`. Polynomial surface +
polynomial trims: exact rational. Polynomial surface + rational trims: exact
Green identity, 1D `k â‰¤ r+s+2` reciprocal certificates (G1's kernel).
Rational surface + trims: polynomialize `Pâ‚ƒ/WÂ³` with the certified tail
(Thm 9.5), Green-reduce the retained terms, 1D-certify rational pullbacks.

```yaml
id:          FHC-G9-GREEN-TRIM-INTEGRATION
contract:    [FHC-G9-GREEN-TRIM-INTEGRATION]
class:       mechanical+
crates:      [truck123d]
depends_on:  [FHC-G1-RATIONAL-FLUX]
needs:       [FHC-G1-RATIONAL-FLUX]
write_allow:
  - truck123d/src/bd_bridge.rs
  - truck123d/tests/green_trims.rs
read_allow:
  - docs/F1_HYPERCAR_GAP_REGISTER.md
  - docs/CERTIFIED_FLUX_CALCULUS_INTEGRATION_MAP.md
tests_required: [truck123d/tests/green_trims.rs]
anchors:
  - {id: A1, expect: 3, cmd: "grep -c 'binding_volume_facts' truck123d/src/bd_bridge.rs"}
  - {id: A2, expect: 6, cmd: "grep -c 'cell_flux_exact' truck123d/src/bd_bridge.rs"}
  - {id: A3, expect: 11, cmd: "grep -c 'spline_loop_area_vector' truck123d/src/bd_bridge.rs"}
budget:      {turns: 65, ctx_tokens: 240000}
```

Anchors measured 2026-09-13 at HEAD (A1 counts G1's landing). **Re-measure at
dispatch.**

## Pre-made judgements

1. **Antidifferentiation is a coefficient map** (Lemma 9.2: cumulative sums
   on Bernstein coefficients, degree r â†’ r+1) on the existing Bernstein ops â€”
   no new basis machinery.
2. **Trim loops live in parameter space** as oriented carriers: polynomial
   BÃ©zier segments (exact) and rational BÃ©zier segments (1D-certified).
   Oriented outer loops positive, hole loops negative; a sign-violating loop
   set refuses typed. FHC-B's multi-contour section machinery is the
   representation precedent â€” reuse its loop/orientation discipline, not its
   section code.
3. **Rational surfaces**: polynomialize first (`g_r = (1/cÂ³)Î£(-1)^j C(j+2,2) Pâ‚ƒ e^j`,
   certified tail `E_r`), then Green-reduce each retained polynomial term.
   No rational antiderivative engine, no triangulation of Î©.
4. **Whole-face first** (Design rule 9.6): try increasing reciprocal order
   while the predicted 1D boundary cost is cheaper than trim clipping;
   spatial subdivision of trim loops is an optimization layer, NOT a
   prerequisite for correctness, and is OUT OF SCOPE here.
5. Approximate trims (Â§10's `w_m` tube) are OUT OF SCOPE â€” exact trims only;
   an inexact trim input refuses typed `unrepresented_trim`.

## Tests required (`truck123d/tests/green_trims.rs`)

1. `antiderivative_exact` â€” Lemma 9.2 round-trip: differentiating the
   constructed antiderivative reproduces the integrand coefficients exactly.
2. `green_matches_full_rectangle` â€” for an untrimmed patch, the Green
   boundary route equals the direct exact integral bit-for-bit (the boundary
   of the full rectangle is the degenerate loop set).
3. `polynomial_trims_exact` â€” polynomial surface + polynomial rectangular
   and triangular trims with independently computable values: bit-identical.
4. `quarter_disk_pi` (doc T6/T8) â€” polynomial plane trimmed by a rational
   circular arc: converges to the known Ï€-dependent value via the 1D
   `k=2` route; the exact-rational path is NOT forced.
5. `rational_surface_trimmed` (doc T9) â€” Thm 9.5 end-to-end: certified
   bracket vs high-precision adaptive quadrature over the trimmed domain;
   order increase shrinks width monotonically.
6. `hole_orientation_refuses` â€” mis-oriented hole loops refuse typed; never
   a silently wrong sign.

## Done when

```
cargo fmt --check -p truck123d
cargo clippy -p truck123d --all-targets -- -D warnings
cargo test -p truck123d --test green_trims --locked
```

plus one bounded door spot-check of a corpus row whose surfaces carry holes
(whichever R4 identifies first; `hypercar/details` class). All cargo through
the queue.

## Forbidden

Any file outside `write_allow`; editing `corpus/ttc/**` or `vendor/truck/**`;
meshing a trimmed interior; approximate trims; `#[ignore]`, deleted or
weakened tests, bare `cargo test`; committing to `main`.

## Stop conditions

- a rational trim pullback whose denominator positivity cannot be certified
  â†’ refine or refuse `rational_flux_inconclusive` (G1 semantics)
- Green reduction cannot be made exact for the polynomial case â†’ `SPEC_GAP`
  naming the gap
- anchor count differs at dispatch and was not re-measured â†’ `ANCHOR_MISMATCH`
- three consecutive failed cargo runs on the same error â†’ `BLOCKED`

## Finish by writing `RESULT.json` AT THE WORKTREE ROOT

```json
{"id":"FHC-G9-GREEN-TRIM-INTEGRATION","status":"DONE","contracts":["FHC-G9-GREEN-TRIM-INTEGRATION"],
 "anchors_verified":{"A1":3,"A2":6,"A3":11},
 "rows_flipped":[],"rows_still_refused":[],
 "notes":"suite 1-6 verdicts; trim carrier record; row spot-check"}
```

Commit subject: `truck123d: Green-reduction trim integration - exact hole boundaries, no interior mesh (FHC-G9)`.
