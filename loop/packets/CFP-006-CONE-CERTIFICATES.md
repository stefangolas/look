# WORK PACKET CFP-006-CONE-CERTIFICATES — Gauss-map cone disjointness, three verdicts

You are adding per-box Gauss-map cone disjointness to the contact funnel:
three certified verdicts that close the gff seed-completeness question BY
PROOF and make the CTE cascade provably unreachable on certified cells.
Everything you need is in this document,
`docs/CONTACT_FAST_PATH_BUILD_SPEC.md` (§3 Gauss-map section — quoted below,
§3a findings, §5 gates), and the anchors. Do not read other spec files. A
genuine gap is a SPEC_GAP: stop and report.

```yaml
id:          CFP-006-CONE-CERTIFICATES
contract:    [CFP-006-CONE-CERTIFICATES]
class:       design
crates:      [truck-evidence]
depends_on:  [CFP-000-SPINE, CFP-001-LIFT-ENCLOSURE, CFP-004-IMPLICIT-REDUCTION]
write_allow:
  - vendor/truck/truck-evidence/src/contact/mod.rs
  - vendor/truck/truck-evidence/src/contact/gff.rs
read_allow:
  - vendor/truck/truck-certified/src/cfp/spine.rs
  - vendor/truck/truck-certified/src/cfp/fixtures.rs
  - vendor/truck/truck-certified/src/ssi.rs
  - vendor/truck/truck-certified/src/tangency/cascade.rs
  - vendor/truck/truck-evidence/src/contact/implicit2d.rs
  - vendor/truck/truck-evidence/src/contact/instrument.rs
  - docs/CONTACT_FAST_PATH_BUILD_SPEC.md
tests_required:
  - cone_disjointness_three_verdicts_canonical
  - gff_branch_seeding_complete_under_loop_freedom
  - transversality_cascade_unreachable_on_disjoint_cells
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub fn' vendor/truck/truck-evidence/src/contact/gff.rs"}
  - {id: A2, expect: 11, cmd: "grep -c SplineSsiEntry vendor/truck/truck-certified/src/cfp/spine.rs"}
  - {id: A3, expect: 19, cmd: "grep -c instrument vendor/truck/truck-evidence/src/contact/mod.rs"}
  - {id: A4, expect: 5, cmd: "grep -cF 'F-C6' vendor/truck/truck-certified/src/cfp/fixtures.rs"}
budget:      {turns: 60, ctx_tokens: 140000}
```

H-1: no unwrap/expect without a justified same-line opt-out. H-3
same-line `// H-3`. H-6: never record Float as Exact.

## Problem

The funnel's degenerate tail (near-tangency pairs) burns subdivision
budget until `Unresolved`, and the gff branch-seeding completeness rests
on investigation, not proof. Per-box Gauss-map cone disjointness
certifies both away on the cells where the cones separate.

## The basis (normative; spec §3 quoted)

**Gauss-map certificates (CFP-006 basis).** Per-box cone disjointness
certifies: (i) **loop-freedom** (Sinha parallel-normal; Sederberg's
collinear-normal strengthening; Hohmeyer's LP formulation) — every branch
meets `∂(B1×B2)`, so boundary-crossing seeds are complete, closing the gff
seed-completeness question; (ii) **transversality** (Prop 2 — the CTE
cascade unreachable by proof on those cells); (iii) **a fixed continuation
axis with certified margin** (Prop 2.1), so `ConditioningBelowThreshold`
cannot fire along the branch.

Supporting landed facts (spec §3, for citation): **Proposition 2** — under
immersion, `M_u = M_v = 0 ⇔ n1 ∥ n2`; all four minors vanish under the
same condition. **Proposition 2.1 (closed-form minor bound)** —
`σ(M_u× + M_v×) ≤ σ_min(G1)·|−n2|−|...|` via `|E−v| ≤ σ_min(E)·|v|` for
`v ∈ span(E)`; dependency-free **only when the normals are enclosed by
their own Bernstein nets** — bidegree `(2p−1, 2q−1)`, ~4pq coefficients
per component vs `p×q×4` for the 4-D grid (36 vs 256 at bicubic; nets
count against D3). Cheaper at equal-or-better tightness than interval
`det3`.

## Scope decisions — pre-made, do not relitigate

1. **Canonical carriers first.** The three verdicts land for recognized
   canonical carriers (plane/quadric normal cones are exact);
   spline-patch arms ride CFP-001's sub-box cones (landed). You wire the
   verdicts; you do not re-derive hull machinery.
2. **gff loop-freedom is a proof-consumption wire, not an investigation.**
   Spec §3a: "gff seed completeness (unverified, not logged): closed by
   proof under CFP-006(i) rather than by investigation." When the cones
   of a cell pair are disjoint, the gff branch-seeding path may assert
   completeness on that cell pair (every branch meets the cell boundary).
   The seeded-path change carries the certificate, not a heuristic.
3. **Transversality = cascade unreachable by proof.** On cone-disjoint
   cells the pair cannot be near-tangential; the routing asserts the
   cascade (CTE-004's machinery, consumed read-only) is never entered
   from these cells. The cascade itself is untouched (CFP-008 owns its
   file).
4. **Three verdicts, one vocabulary.** Disjoint / overlapping /
   undecidable-at-this-depth — mapped to the landed verdict vocabulary;
   mapping rows for any new `Method` tags are the SPINE's
   (`CERTIFICATE_MAPPING.md` — read-only for you; a seeming new arm is a
   SPEC_GAP against it).
5. **SFC discipline:** cone separation search may float (GJK-style hint);
   the certificate is exact-sign. Deterministic traversal; no hash order.
6. **Instrument counters** ride the CFP-002 hook: count verdict
   distribution. Zero verdict change from counters.

## Tests required

1. `cone_disjointness_three_verdicts_canonical` — canonical-carrier
   fixtures exercising all three verdicts (known-disjoint cones,
   known-overlapping cones, and a depth-boundary undecidable case), each
   asserted with its certificate payload.
2. `gff_branch_seeding_complete_under_loop_freedom` — a cell pair with
   certified loop-freedom: every branch asserted to meet the cell
   boundary through the seeded path (the proof made executable).
3. `transversality_cascade_unreachable_on_disjoint_cells` — a constructed
   pair where the flat path would enter the cascade, asserted NOT to
   under cone disjointness (the routing refuses the cascade entry with
   the transversality certificate).

## Done when — run these, all must pass

```
cargo fmt --check -p truck-evidence
cargo clippy -p truck-evidence --all-targets -- -D warnings
cargo test -p truck-evidence --lib contact
cargo check -p truck-shapeops
```

Scoped commands only, through the queue shim (the `cargo` on PATH IS the
shim). Send output to a file and read the tail.

## Forbidden

Anything outside write_allow — especially `tangency/cascade.rs` (CFP-008's),
`ssi.rs`/`ssi_types.rs` (CFP-003's), `implicit2d.rs` (CFP-004's),
`ssi4.rs`/`krawczyk.rs` (frozen), `CERTIFICATE_MAPPING.md` (spine-owned),
corpus fixtures (read-only). New top-level evidence kinds (SPEC_GAP against
the mapping). Adding `#[ignore]`. Unjustified `#[allow]`. Committing to
main.

## Stop conditions

- any anchor count differs → ANCHOR_MISMATCH
- the three verdicts cannot map onto the landed verdict vocabulary
  without a new evidence kind → SPEC_GAP against `CERTIFICATE_MAPPING.md`
- three consecutive failed cargo test runs on the same error → BLOCKED

## Finish by writing RESULT.json at the WORKTREE ROOT (then COMMIT first)

```json
{"id":"CFP-006-CONE-CERTIFICATES","status":"DONE","contracts":["CFP-006-CONE-CERTIFICATES"],
 "tests_added":3,"anchors_verified":{"A1":1,"A2":11,"A3":19,"A4":8},
 "notes":"the verdict-to-vocabulary mapping; the gff completeness wiring; any deviation"}
```

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-DONE status also write `QUESTION.md` beside it.

Commit subject: `feat(contact): Gauss-map cone certificates — loop-freedom, transversality, certified continuation axis (CFP-006-CONE-CERTIFICATES)`.
