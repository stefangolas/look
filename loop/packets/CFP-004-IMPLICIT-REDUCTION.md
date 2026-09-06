# WORK PACKET CFP-004-IMPLICIT-REDUCTION — the analytic×spline funnel stage

You are adding the funnel's implicit-reduction stage: contact between a
recognized analytic surface and a spline patch becomes ONE scalar equation on
the spline chart. Everything you need is in this document,
`docs/CONTACT_FAST_PATH_BUILD_SPEC.md` (§2 spine decisions, §3 Theorem 4, §3a
findings, §5 gates — quoted below where load-bearing), and the anchors. Do
not read other spec files. A genuine gap is a SPEC_GAP: stop and report.

```yaml
id:          CFP-004-IMPLICIT-REDUCTION
contract:    [CFP-004-IMPLICIT-REDUCTION]
class:       design
crates:      [truck-evidence]
depends_on:  [CFP-000-SPINE, CFP-002-INSTRUMENT, CFP-003-SEPARABILITY]
write_allow:
  - vendor/truck/truck-evidence/src/contact/mod.rs
  - vendor/truck/truck-evidence/src/contact/implicit2d.rs
  - docs/BREP_GENERATION_API.md
read_allow:
  - vendor/truck/truck-certified/src/cfp/spine.rs
  - vendor/truck/truck-certified/src/cfp/fixtures.rs
  - vendor/truck/truck-certified/src/formal/bezier_isect.rs
  - vendor/truck/truck-evidence/src/contact/implicit.rs
  - vendor/truck/truck-evidence/src/contact/solver_entry.rs
  - vendor/truck/truck-evidence/src/contact/instrument.rs
  - vendor/truck/truck-certified/src/ssi_types.rs
  - docs/CONTACT_FAST_PATH_BUILD_SPEC.md
  - docs/BREP_GENERATION_API.md
tests_required:
  - implicit_stage_reduces_plane_spline_theorem4
  - elevation_trap_twin_disagrees_fc4
  - unresolved_localizes_to_stratum_pair
anchors:
  - {id: A1, expect: 11, cmd: "grep -c SplineSsiEntry vendor/truck/truck-certified/src/cfp/spine.rs"}
  - {id: A2, expect: 18, cmd: "grep -c instrument vendor/truck/truck-evidence/src/contact/mod.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'pub fn' vendor/truck/truck-certified/src/formal/bezier_isect.rs"}
  - {id: A4, expect: 0, cmd: "ls vendor/truck/truck-evidence/src/contact/implicit2d.rs 2>/dev/null | wc -l"}
  - {id: A5, expect: 8, cmd: "grep -cF 'F-C4' vendor/truck/truck-certified/src/cfp/fixtures.rs"}
budget:      {turns: 70, ctx_tokens: 160000}
```

New file (implicit2d.rs): H-1 applies — no unwrap/expect without a
justified same-line opt-out. H-3 same-line `// H-3`. H-6: never record
Float as Exact — the certificate discipline below is normative.

## Problem

The funnel's stage-5 pair entry tests analytic×spline pairs with the full
4-D bidegree machinery: expensive, and the pair mix data (CFP-002's
counters) says plane/quadric×spline dominates. Theorem 4 collapses that
class: the contact set on the spline chart is the zero set of ONE scalar
function, and its critical points are a 2×2 square system for the LANDED
Krawczyk — no new solver, no new evidence kinds.

## The theorem (normative; spec §3 quoted)

**Theorem 4 (implicit reduction, analytic × spline).** The contact set on
the spline chart is the zero set of `h(u,v) = g(S(u,v))` — one scalar
equation, exact Bernstein form: bidegree `(p,q)` preserved for planes
(coefficients = signed control-point distances), `(2p,2q)` for quadrics
(exact Bernstein product). **The torus is excluded explicitly** — its
implicit form is quartic, giving `(4p,4q)` (169 coefficients at bicubic)
and its own D3 economics; out of scope (A7). Critical points are `∇h = 0`
— a 2×2 square system for the landed Krawczyk directly; F3 reduction,
continuation-axis choice, and `ConditioningBelowThreshold` disappear; A1
classification is literal scalar Morse theory.

**Elevation trap (packet-normative):** `∂h/∂u` and `∂h/∂v` have bidegrees
`(p−1,q)` and `(p,q−1)`; their arrays cannot be paired into 2-vectors
until both are degree-elevated to a common bidegree — only then is
`∇h(x) = Σ B_I(x)(a_I, b_I)` a convex combination and the hull test valid.
Cheap and exact; F-C4 pins it. Loop detection: `0 ∉ conv{∇h coefficients}`
(Jordan + extreme value theorem; `h ≡ 0` on the disk fails the test
honestly). Coefficient mass at bicubic×plane: 16 vs 192.

## Scope decisions — pre-made, do not relitigate

1. **Slot decision executed, not redesigned.** The spine froze
   `SplineSsiEntry`'s dispatch-method signature (spine.rs, 11 sites). You
   grow the entry by that signature; ONE registrant routes internally to
   the new stage. The registration-order irrelevance is already solved at
   spine — do not re-decide it.
2. **The 2-D scalar exclusion/trace is adapted from the
   `formal/bezier_isect.rs` lineage** — read it, reuse its discipline
   (exact predicates over float search), do not import it wholesale.
3. **Torus stays on the old path** — the stage dispatches planes and
   quadrics only; the torus arm is a no-op route-through (A7 boundary).
   The dispatcher must still route torus pairs correctly today; it just
   gets no reduction.
4. **Consumer rule (spec §3a, DECIDED 2026-09-06, owner sign-off):**
   `Unresolved` propagation into the boundary rewrite is a typed refusal
   to the caller of `boolean()` **failing the WHOLE operation** — one
   resistant cell fails the call, with the stratum-pair identity carried
   so a client-layer partition strategy remains available; no narrowing
   retry, no fallback, no partial-result shape. This matches the landed
   fail-closed refusal arms and OCC's whole-op failure semantics.
5. **SFC discipline:** every float heuristic is (search, exact-verify)
   pairs. The exclusion test's coefficients are exact Bernstein
   coefficients; sign decisions use the landed interval/Expansion
   predicates, never naked float comparisons.
6. **Instrument counters ride the same hook** (CFP-002's schema): count
   stage-entry pair classes so the post-fix `f` datum stays measurable.
   Zero verdict change from counters.
7. **Determinism:** fixed dispatch order (recognized-class enum order);
   no hash iteration in output-affecting paths.

## Tests required

1. `implicit_stage_reduces_plane_spline_theorem4` — F-C4's plane×bicubic
   ground truth: the stage produces the known `h` (coefficients = signed
   control-point distances) and finds the known critical point through
   the landed Krawczyk; result matches the recorded ground truth.
2. `elevation_trap_twin_disagrees_fc4` — the F-C4 elevation-trap twin:
   the non-elevated `∇h` hull pairing DISAGREES with the elevated one on
   the constructed input (this test is why the elevation step exists;
   assert the elevated one is the correct verdict).
3. `unresolved_localizes_to_stratum_pair` — a constructed resistant cell
   yields the typed refusal carrying the stratum-pair identity, per the
   consumer rule; no retry, no fallback, no silent downgrade.

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

Anything outside write_allow — especially `ssi4.rs`/`krawczyk.rs` (frozen;
the Krawczyk is CONSUMED via its landed API), CFP-003's files
(`ssi.rs`, `ssi_types.rs`, `tangency/{minors,exclude,tsystem}.rs`),
`contact/gff.rs` (CFP-006's), corpus fixtures (read-only). Any new
top-level evidence kind (a seeming new arm is a SPEC_GAP against
`CERTIFICATE_MAPPING.md`). Torus reduction. Adding `#[ignore]`.
Unjustified `#[allow]`. Committing to main.

## Stop conditions

- any anchor count differs → ANCHOR_MISMATCH
- the frozen `SplineSsiEntry` signature cannot express the dispatch
  without widening it → SPEC_GAP (the signature is spine-frozen; widening
  is an owner amendment, never a local decision)
- three consecutive failed cargo test runs on the same error → BLOCKED

## Finish by writing RESULT.json at the WORKTREE ROOT (then COMMIT first)

```json
{"id":"CFP-004-IMPLICIT-REDUCTION","status":"DONE","contracts":["CFP-004-IMPLICIT-REDUCTION"],
 "tests_added":3,"anchors_verified":{"A1":11,"A2":19,"A3":1,"A4":1,"A5":13},
 "notes":"the dispatch-method realization vs the frozen signature; the elevation step's implementation; any deviation"}
```

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-DONE status also write `QUESTION.md` beside it.

Commit subject: `feat(contact): implicit-reduction funnel stage — Theorem 4 reduction, slot decision executed (CFP-004-IMPLICIT-REDUCTION)`.
