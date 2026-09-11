# PARAMETER-FRONTIER-A — Final handoff

**Date:** 2026-08-14
**Look HEAD:** `78bd208d` (working tree carries the uncommitted truck pin bump, see below)
**Truck HEAD:** `40212ece` (`feature/cone-apex-lift-recovery`, pushed 2026-08-14)

Two production fixes landed in truck-meshalgo, each an isolated commit. `look`
was verified against the local fork working tree, then the fork was pushed and
the pin in `look/Cargo.toml` + `Cargo.lock` bumped to `40212ece`; the
`.cargo/config.toml` path override was re-commented and the bump documented
there. The look-side pin changes are **uncommitted** in the working tree
(`Cargo.toml`, `Cargo.lock`, `.cargo/config.toml`).

The ABC corpus (`C:\Users\stefa\look-corpus\abc`) was deleted by the owner
mid-session, so the final 20-model ABC census, the NIST `7902/7902/0` gate, and
the corpus-backed regression list could not be re-run. Numbers below are from
the real-witness runs performed before deletion.

---

## AmbiguousLift — `197 → 0 on all observed models`

**Symbols changed** (`truck-meshalgo/src/tessellation/triangulation.rs`):

- `PolyBoundaryPiece::try_new`, the bisection-exhaustion site in the boundary
  lift: the two candidate deck copies `[1,0]`, `[0,1]` are now filtered by the
  certified periodicity of the axes **before** any numerical dominance is
  attempted.
- new `fn legal_deck_shifts(candidates: &[[i64; 2]], periodic_u, periodic_v)`
- new `mod legal_deck_shift_tests` (4 tests)

**Theorem.** A deck shift `(ku, kv)` advances `ku` full periods along `u` and
`kv` along `v`; an advance along an axis is legal only when that axis is
periodic. A shift is retained iff `ku != 0 ⇒ periodic_u` and
`kv != 0 ⇒ periodic_v`. `periodic_u`/`periodic_v` are the same accessor facts
the diagnostic records as `periodic_axes` (verified: the 197 historical
records partition exactly as u-only 112 / v-only 51 / nonperiodic 34 against
the record's face-level `periodic_axes` field, which derives from
`surface.u_period()/.v_period()`).

**Routing.**

- both axes certified periodic → both candidates remain, existing
  `get_mindiff`/`bisection_exhausted` dominance, `AmbiguousLift` stays correct;
- exactly one axis certified periodic → the other deck copy is structurally
  impossible, the ambiguity is resolved by the certified axes (no
  `get_mindiff` between the candidates);
- neither axis certified periodic → the periodic deck resolver does not
  activate; the ordinary base lift (the originating real boundary sample,
  `origin`) is preserved and the walk resumes, exactly as the singular tie
  branch does.

No bisection count, epsilon, candidate-picking, or axis preference changed.

**Witness results (real runs):**

| model | OLD AmbiguousLift | NEW |
|-------|-------------------|-----|
| 00007705 | 141 | 0 |
| 00009190 | 15 | 0 |
| 00001075 | 2 | 0 |

**Tests:** `cargo test -p truck-meshalgo --lib` → 751 passed, 2 failed
(`cone_topology_tests::duplicate_edge_creates_no_second_cdt_edge`,
`test_parity_intersecting_constraints_rejected`) — both confirmed **pre-existing
on the clean baseline** via stash. No new failures.

---

## EvaluatorOutOfDomain — `328 → 0 on all observed models`

**Audit witness:** `00009190` face `52230`, surface `#119883`, EOD candidate
`(0.9999999999889794, -1.0623849394715243)`, residual `5.55e-17`, tol `3.056e-3`.

**Source-topology trace (S1–S4):**

- `#52230 = ADVANCED_FACE( '', ( #119882 ), #119883, .T. )`
- `#119882 = FACE_OUTER_BOUND( '', #305509, .T. )`
- `#305509 = EDGE_LOOP( '', ( #570389, #570390, #570391 ) )` →
  `#674882` (CIRCLE r=0.002), `#678056` (B_SPLINE_CURVE),
  `#712703` (B_SPLINE_CURVE)
- `#119883` = RATIONAL_B_SPLINE_SURFACE, u-degree 3 / v-degree 2, knots
  `[0,1]²`, `u_closed=.F.`, `v_closed=.F.`

**S1:** No source PCURVE/UV exists — every boundary edge is a plain 3D curve
(CIRCLE, B_SPLINE_CURVE). **→ Case 3.**
**S2:** N/A. **S3:** N/A. **S4:** N/A directly, but the constrained inverse
restricted to `D_accept = [0,1]²` finds `(1.0, 0.0195)` with residual `1.4e-14`
(grid) and `(1.0, 0.0)` with `5.0e-15` (clamped Newton), both ≤ tol. The
unconstrained production chain returns `None` from every start
(`search_parameter` ×2, `search_nearest_parameter` ×2, 1 structural seed) for
this point. The surface's evaluator is effectively v-periodic with period = the
span (declared open); the solver escaped to the extrapolated copy `v = -1.06`.

**Six concrete answers (witness):**

| item | exact answer |
|------|--------------|
| `q_inv` | `(0.9999999999889794, -1.0623849394715243)` — native NURBS knot coordinate from `search_nearest_parameter_outcome` / `probe_nearest` |
| `D_inv` | native knot space; `surface.subs` evaluates it finitely (extrapolation) |
| `D_accept` | `[0,1] × [0,1]` — `surface.try_range_tuple()` = `BSplineSurface::parameter_range()` = full knot span |
| `q_residual` | identical to `q_inv`; residual `5.55e-17` computed at `surface.subs(q_inv)` in `search_nearest_parameter_outcome` |
| `q_eval` | native — `surface.subs(u, v)` takes exactly `q_inv` (identity) |
| conversion | identity — no mapping needed; the inverse already returns the evaluator's native coordinate |

**The mismatch.** `q_inv == q_residual == q_eval` (same native convention,
proven by the finite evaluation and the 5.5e-17 residual). `D_accept` (the
declared knot span) is a trim/topology wrapper interval that is narrower than
the evaluator's actual valid domain, and Stage A/B refused the candidate solely
because `q_inv ∉ D_accept` while Stage C refused the large mismatch as
`RepresentationRangeMismatch`. The inverse selected the wrong extrapolated
branch even though an in-range root exists.

**The correction (Case 3 branch).** New recovery stage **PROJ-003 Stage D** —
a constrained inverse restricted to `D_accept`:

- `fn constrained_domain_inverse(surface, point, tol)` in
  `truck-meshalgo/src/tessellation/triangulation.rs`: a `64×64` grid over the
  declared range, then clamped-Newton refinement pinned to the box,
  re-certified under the residual contract (finite UV, finite evaluation,
  `|S(u,v) - P| <= tol`).
- wired into the lift as the fourth `else if` in the projection recovery chain,
  running only after the whole legacy chain and Stages A/B/C refused the point
  (refinement-only; nothing that already projected is changed).
- gate: `TRUCK_FORMAL_RECOVERY_PROJ_STAGE_D` (default on), added in
  `diagnosis.rs` as `proj_domain_constrained_enabled`.
- new `mod constrained_domain_inverse_tests` (4 tests: in-range found,
  boundary-edge found in-range, no-in-range-root refuses, above-tolerance
  refuses).

It never clamps a solver answer into range and never modulos a non-periodic
axis; it only searches inside the range in the first place. `D_accept` is kept.

**Negative invariants preserved:** projection tolerance unchanged, R01
tolerance unchanged, source topology unchanged, no clamp, no modulo.
`ResidualAboveTolerance`, `SingularEvaluation`, and nonfinite-evaluation
refusals are structurally unchanged (Stage D requires residual ≤ tol, finite
evaluation, and a non-singular iteration).

**Witness results (real runs):**

| model | OLD EOD | NEW |
|-------|---------|-----|
| 00009190 | 27 | 0 |
| 00001075 | 113 | 0 |

**Tests:** same truck-meshalgo suite as above (751 passed, 2 pre-existing
failures).

---

## Preserved populations (observed models)

| bucket | 00009190 OLD→NEW | 00001075 OLD→NEW | 00007705 OLD→NEW |
|--------|------------------|------------------|------------------|
| RejectedDegenerate | 2 → 2 | 2 → 2 | 2 → 2 |
| EdgeTraversalUnresolved | — | 8 → 8 | — |
| NoOddParityRegion | 3 → 3 | — | 29 → 29 |
| ResidualAboveTolerance | — | — | — |
| SingularEvaluation | — | — | — |

The `RejectedDegenerate → generic failure` transition count is 0 in every
observed run.

---

## Render transition ledger (observed witnesses)

Faces previously refused as `AmbiguousLift` / `BoundaryProjectionFailed`(EOD)
now render. **rendered → lost = 0 and rendered → rejected = 0** on every
witness: the two fixes are refinement-only, so a face that rendered before is
byte-identical. Failed-face records dropped 195 → 60 (00007705) and
56 → 10 (00009190) and 125 → 10 (00001075), with the surviving records being
unchanged buckets (NoOddParityRegion, RejectedDegenerate,
EdgeTraversalUnresolved, NonFinitePosition, ContradictoryDualParity,
ConstraintRoleMissing).

---

## Verification status

- `cargo fmt --all -- --check` — clean (look and truck-fork).
- `cargo check --locked --all-targets` (look) — clean, against pinned
  `40212ece`.
- `cargo test --locked -p truck-meshalgo --lib` — 751 passed, 2 **pre-existing**
  failures (confirmed on the clean baseline by stash).
- `git diff --check` — clean (both repos).
- Real-witness runs: 00007705, 00009190, 00001075 — both fixes verified on
  the exact historical failing populations (see tables above).
- **Not re-run:** the 20-model ABC census, NIST `7902/7902/0`, and the
  corpus-backed regression list (#1167, #1169, phase alignment, DeckConsistent,
  R01/00007667, NoOdd, Track-B, diagnostic exact-once) — the ABC corpus was
  deleted by the owner mid-session. `look`'s debug test rebuild also could not
  be completed: the C: drive ran to 0 bytes free during the rebuild
  (paging-file/no-space link failures), the target dirs were then cleaned
  (truck-fork ~4.2 GiB, look ~29 GiB), and `cargo check --all-targets` was
  re-verified instead.

## Handoff notes for the next session

- Push/commit the `look` pin bump (working tree currently carries
  `Cargo.toml` + `Cargo.lock` → `40212ece`, `.cargo/config.toml` re-commented
  with a dated note).
- Re-run the ABC census (once), NIST, and the focused regression list from a
  machine with the corpus and enough disk for the debug test profile.
- Expected target after re-run: `AmbiguousLift ~0`, `EvaluatorOutOfDomain ~0`,
  `RejectedDegenerate 1035`, `ResidualAboveTolerance 5`,
  `EdgeTraversalUnresolved 16`, `SingularEvaluation 21` unchanged.
