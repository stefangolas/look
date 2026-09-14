# WORK PACKET FHC-G1 â€” rational-weight patch flux through the certified boolean funnel (AMENDED to the Certified Flux Calculus architecture)

Gap-register category 1: the certified boolean funnel consumes unit-weight
patches only; every rotational boundary (cylinder/cone/sphere/revolve) is a
rational patch with `W(u,v) â‰  const`, so canonical CYLINDER+CYLINDER unions
refuse `unsupported_envelope` at admission. This packet lands the
rational-flux path.

**NORMATIVE THEORY (supersedes the inline T1â€“T6 of the 2026-09-12 owner
statement, owner adoption 2026-09-13):**
`docs/CERTIFIED_FLUX_CALCULUS_INTEGRATION_MAP.md` maps the architecture onto
this codebase; the normative statement is the owner's **Certified Flux
Calculus** proposal (Â§5 Theorem 5.2 reciprocal-power kernel, Â§6 exponent-3
reduction `det[B,B_u,B_v]/WÂ³` with `B = A âˆ’ aW`, Â§6.1 sign admission, Â§8
referenced for the router boundary, Â§13 routing order). The worker implements
it; it does not relitigate it. `cell_flux_exact` stays untouched as the
polynomial fast path (its `W = const` special case). The L5/`Modulus::compose`
precondition of the original packet is DROPPED â€” Theorem 5.2 consumes no
composite reciprocals.

```yaml
id:          FHC-G1-RATIONAL-FLUX
contract:    [FHC-G1-RATIONAL-FLUX]
class:       design
crates:      [truck123d]
depends_on:  [FHC-G6-CERT-COST-SCALE]
needs:       [FHC-G6-CERT-COST-SCALE]
write_allow:
  - truck123d/src/bd_bridge.rs
  - truck123d/src/facade.rs
  - truck123d/tests/rational_flux.rs
read_allow:
  - docs/F1_HYPERCAR_GAP_REGISTER.md
  - docs/CERTIFIED_FLUX_CALCULUS_INTEGRATION_MAP.md
  - docs/MONO_CLOSURE_BOOKING.md
tests_required: [truck123d/tests/rational_flux.rs]
anchors:
  - {id: A1, expect: 6, cmd: "grep -c 'cell_flux_exact' truck123d/src/bd_bridge.rs"}
  - {id: A2, expect: 4, cmd: "grep -c 'boolean_product_volume_certified' truck123d/src/bd_bridge.rs"}
  - {id: A3, expect: 2, cmd: "grep -c 'placed_box_patches' truck123d/src/bd_bridge.rs"}
  - {id: A4, expect: 3, cmd: "grep -c 'node_box_patches' truck123d/src/bd_bridge.rs"}
  - {id: A5, expect: 41, cmd: "grep -c 'VolumeRow' truck123d/src/bd_bridge.rs"}
budget:      {turns: 100, ctx_tokens: 350000}
```

Anchors measured 2026-09-13 at HEAD. **Re-measure at dispatch** â€” update
stale counts, never dispatch stale.

## Scope (the five deliverables)

1. **Two-channel certificate** (`facade.rs` + plumbing): `(â„“, h; w_m, w_i)`
   with additive composition (Minkowski) and conservative migration â€” every
   existing exact output embeds as `(q, q; 0, 0)`; no existing output changes.
2. **Reciprocal-power kernel** (`reciprocal_power_integral`, Thm 5.2): 1D and
   2D, any integer `k â‰¥ 1`; optimal center `c = (L+U)/2`, `Î´ = (Uâˆ’L)/(U+L)`;
   certified tail `R_{k,r}(Î´)`; exact/outward-rounded endpoints; `e = Q/c âˆ’ 1`
   keeps the expansion polynomial at `W`'s degree â€” never expand `WÂ³`.
3. **Rational surface arm**: `cell_flux_certified` returning
   `FluxCert::Exact | FluxCert::Enclosure` per the two-channel certificate;
   admission (MONO-8) amended per Â§6.1 â€” admit when `W` is constant OR carries
   a certified positive lower bound (`min w_ij > 0`, homogeneous coefficients);
   budget exhaustion refuses `rational_flux_inconclusive`, never
   `unsupported_envelope`. `VolumeRow.weights` un-pinned; placement and
   subdivision operate homogeneously on `(X,Y,Z,W)` (no dehomogenization,
   no approximation anywhere in the new path).
4. **MONO-9 interval fold**: same-mode chains fold by Minkowski addition;
   `Exact(x) = [x,x]`; mixed-mode nesting still evaluates pairwise.
5. **Gauge invariance + exact normalization**: `(P,W) âˆ¼ (Î»P, Î»W)` flux-invariant;
   admission-time exact degree reduction (homogeneous, `O(r)` test) before
   flux construction.

## Acceptance suite (each a named `#[test]` in `rational_flux.rs`)

The original 9 tests stand, plus the proposal's T1â€“T4 subset:

1. `unit_weight_cell_equivalence` â€” rational path with `W = 1` equals
   `cell_flux_exact` summed over a closed cell (per-face integrands differ by
   the divergence-field choice; per-face equality is NOT asserted).
2. `constant_weight_gauge` â€” `(P,W)` and `(Î»P, Î»W)` produce identical
   certified flux.
3. `polynomial_oracle` â€” polynomial patches as homogeneous patches collapse
   to the old exact result.
4. `placement_restriction_homogeneous` â€” homogeneous transform + subdivision
   agrees with direct evaluation.
5. `positive_cylinder_admitted` â€” a standard rational cylinder patch no longer
   returns `unsupported_envelope`.
6. `weight_zero_refines_or_refuses` â€” positivity that cannot be certified
   refines or refuses `rational_flux_inconclusive` â€” never enters the
   reciprocal kernel.
7. `fold_partition_contains_whole` â€” partitioning a face and summing child
   certificates contains the unpartitioned face's integral.
8. `anchor_uniformity_enforced` â€” mixed anchors within one closed cell are
   prevented by construction (typed refusal).
9. `end_to_end_cylinder_union` â€” at least one canonical CYLINDER+CYLINDER
   fastener union passes the entire `boolean_product_volume_certified` route
   (MONO-8 â†’ flux â†’ MONO-5 â†’ MONO-9 connected end to end).
10. `tail_monotone_shrinks` (doc T4) â€” reciprocal order increase strictly
    shrinks the returned width on a known integral.
11. `subdivision_order` (doc T5) â€” fixed order `r`, uniform subdivision:
    fitted total-width slope â‰ˆ `r+1`.

## Method

Two-channel certificate first (migration keeps every existing test green) â†’
reciprocal kernel with its own unit tests â†’ rational arm + admission â†’ fold â†’
the suite in order. Row verdicts (`f1/nose`, `f1/sidepod_*`, `f1/airbox`,
`f1/details`, `f1/drivetrain` door spot-checks, bounded runs) in RESULT.

## Done when

```
cargo fmt --check -p truck123d
cargo clippy -p truck123d --all-targets -- -D warnings
cargo test -p truck123d --test rational_flux --locked
```

plus the end-to-end door spot-check (test 9's row through
`corpus/ttc/door.py --engine truck`). All cargo through the queue (the
`cargo` on PATH IS the queue shim); interpreter dir on PATH if
STATUS_DLL_NOT_FOUND.

## Forbidden

Mutating `cell_flux_exact`'s contract; dehomogenization or approximation
anywhere in the new path; editing `corpus/ttc/**` or `vendor/truck/**`;
any file outside `write_allow`; `#[ignore]`, deleted or weakened tests, bare
`cargo test`; committing to `main`.

## Stop conditions

- Theorem 5.2's enclosure cannot be made sound over the landed certificate
  algebra â†’ `SPEC_GAP` naming the exact gap
- an admission widening exposes a pair the contact-cover hypotheses do not
  cover â†’ typed refusal naming the pair class + `SPEC_GAP`
- anchor count differs at dispatch and was not re-measured â†’ `ANCHOR_MISMATCH`
- three consecutive failed cargo runs on the same error â†’ `BLOCKED`

## Finish by writing `RESULT.json` AT THE WORKTREE ROOT

```json
{"id":"FHC-G1-RATIONAL-FLUX","status":"DONE","contracts":["FHC-G1-RATIONAL-FLUX"],
 "anchors_verified":{"A1":6,"A2":4,"A3":2,"A4":3,"A5":41},
 "rows_flipped":[],"rows_still_refused":[],
 "notes":"suite 1-11 verdicts; end-to-end union record; before/after row verdicts"}
```

Commit subject: `truck123d: rational-weight patch flux through the certified funnel (FHC-G1)`.
