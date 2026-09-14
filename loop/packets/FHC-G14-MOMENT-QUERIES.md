# WORK PACKET FHC-G14 — moment queries: area, center of mass, inertia via divergence-field moments

The certified flux machinery computes volume via one divergence field
(`F = (x−a)/3`, `∇·F = 1`). The same exact polynomial integrator yields the
full moment family by swapping fields — the register's "mechanical,
nearly-free" class:

- volume: `∇·F = 1`, `F = (x−a)/3` (landed)
- first moments `∫x_i dV`: `∇·(x_i·x/4) = 4x_i/4 = x_i` → `F_i = x_i·x/4`
- second moments `∫x_i·x_j dV` (inertia tensor inputs): `∇·(x_i·x_j·x/5) = 5·x_i·x_j/5`
- surface area is NOT a flux (no divergence field) — out of scope here.

**Normative theory:** Certified Flux Calculus §1.3/§3 via
`docs/CERTIFIED_FLUX_CALCULUS_INTEGRATION_MAP.md` (the moment contraction is
the same Bernstein mean-value machinery — Lemma 3.1 — at higher degree).

```yaml
id:          FHC-G14-MOMENT-QUERIES
contract:    [FHC-G14-MOMENT-QUERIES]
class:       mechanical
crates:      [truck123d]
depends_on:  []
needs:       []
write_allow:
  - truck123d/src/bd_bridge.rs
  - truck123d/tests/moment_queries.rs
read_allow:
  - docs/F1_HYPERCAR_GAP_REGISTER.md
  - docs/CERTIFIED_FLUX_CALCULUS_INTEGRATION_MAP.md
tests_required: [truck123d/tests/moment_queries.rs]
anchors:
  - {id: A1, expect: 6, cmd: "grep -c 'cell_flux_exact' truck123d/src/bd_bridge.rs"}
  - {id: A2, expect: 11, cmd: "grep -c 'spline_loop_area_vector' truck123d/src/bd_bridge.rs"}
budget:      {turns: 45, ctx_tokens: 180000}
```

Anchors measured 2026-09-13 at HEAD. **Re-measure at dispatch.**

## Pre-made judgements

1. **Scope: polynomial patches** (the current admitted set). Rational-patch
   moments ride G1's reciprocal kernel later — additive, not a rewrite.
2. **Additive fields only**: the volume path and its outputs are untouched;
   the moment integrands are the same determinant machinery at
   `x`-degree+1 (first) and +2 (second). Determinism: fixed reduction order,
   one field at a time.
3. **Output surface**: new additive fields on the `bd_facts` record
   (`centroid`, `second_moments` — bracket-valued exactly like volume;
   rational exact). No existing field changes; byte-identity of volume/STL
   is the regression gate.
4. Client-side wiring (door probe attributes reading these fields) belongs
   to FHC-D's surface, not this packet.

## Tests required (`truck123d/tests/moment_queries.rs`)

1. `box_analytic` — unit box: centroid, ∫x², ∫xy match closed forms exactly.
2. `sphere_analytic` — polynomial-faceted vs analytic sphere bounds: bracket
   contains the analytic value, width consistent with facetization.
3. `composite_additivity` — a compound's moments equal the sum of parts'
   (the certificate composition law).
4. `placement_law_consistency` — first moments under a known placement
   transform match direct recomputation (bridges to G10's law).
5. `regression_byte_identity` — volume/STL outputs of previously green rows
   unchanged.

## Done when

```
cargo fmt --check -p truck123d
cargo clippy -p truck123d --all-targets -- -D warnings
cargo test -p truck123d --test moment_queries --locked
```

All cargo through the queue.

## Forbidden

Any file outside `write_allow`; editing `corpus/ttc/**` or `vendor/truck/**`;
surface-area claims (no divergence field); `#[ignore]`, deleted or weakened
tests, bare `cargo test`; committing to `main`.

## Stop conditions

- a moment integrand cannot be certified within the exact polynomial
  algebra → `SPEC_GAP` naming the gap
- anchor count differs at dispatch and was not re-measured → `ANCHOR_MISMATCH`
- three consecutive failed cargo runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` AT THE WORKTREE ROOT

```json
{"id":"FHC-G14-MOMENT-QUERIES","status":"DONE","contracts":["FHC-G14-MOMENT-QUERIES"],
 "anchors_verified":{"A1":6,"A2":11},
 "rows_flipped":[],
 "notes":"suite 1-5; additive field record"}
```

Commit subject: `truck123d: moment queries via divergence fields - centroid + second moments (FHC-G14)`.
