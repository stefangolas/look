# BG-CK-P2-SYSTEM — the square-system constructor + the 3×3 Krawczyk certificate (wave W1)

Wave member W1 of the Phase-2 implementation wave
(`docs/CERTIFIED_INTERLEAVE_BUILD_SPEC.md`; ORCHESTRATOR "wave mode"). This
packet is the booking's BG-CK-P2-SYSTEM and BG-CK-P2-KRAWCZYK3 **collapsed
into one branch**: both booked `src/ssi.rs` as their write set, and wave
mode forbids two workers on the same new file. The collapse was invoked by
the build spec under booking pre-made decision 6's escape hatch. KRAWCZYK3
is never a separate registry row; its content is this packet's Section 2.

LOCAL_GREEN is not DONE: your RESULT.json is a claim. No wave worker runs
the global verifier. The authoritative verify runs once at the integrated
HEAD.

```yaml
id:          BG-CK-P2-SYSTEM
contract:    [BG-CK-P2-SYSTEM]
class:       design
crates:      [truck-certified]
depends_on:  [BG-CK-P2-CONTRACT]
write_allow:
  - vendor/truck/truck-certified/src/ssi.rs
  - vendor/truck/truck-certified/src/lib.rs
  - vendor/truck/truck-certified/tests/ssi_system.rs
read_allow:
  - docs/CERTIFIED_PHASE2_BOOKING.md
  - docs/CERTIFIED_INTERLEAVE_BUILD_SPEC.md
  - vendor/truck/truck-certified/src/ssi_types.rs
  - vendor/truck/truck-certified/src/ssi_fixtures.rs
  - vendor/truck/truck-certified/src/contract.rs
  - vendor/truck/truck-certified/src/hull.rs
  - vendor/truck/truck-certified/src/formal/exact.rs
  - vendor/truck/truck-certified/src/formal/numeric.rs
  - vendor/truck/truck-certified/src/formal/bezier_isect.rs
budget:      {turns: 34, ctx_tokens: 130000}
anchors:
  - {id: A1, expect: TBD, cmd: "grep -c 'pub struct SquareSystem3' vendor/truck/truck-certified/src/ssi_types.rs"}
  - {id: A2, expect: TBD, cmd: "grep -c 'pub struct KrawczykCertificate3' vendor/truck/truck-certified/src/ssi_types.rs"}
  - {id: A3, expect: TBD, cmd: "grep -c 'pub mod ssi_types;' vendor/truck/truck-certified/src/lib.rs"}
  - {id: A4, expect: 0, cmd: "ls vendor/truck/truck-certified/src/ssi.rs 2>/dev/null | wc -l"}
  - {id: A5, expect: TBD, cmd: "grep -c 'ConditioningBelowThreshold' vendor/truck/truck-certified/src/contract.rs"}
  - {id: A6, expect: TBD, cmd: "grep -c 'well_conditioned_root' vendor/truck/truck-certified/src/ssi_fixtures.rs"}
tests_required:
  - system3_constructor_matches_fixture_ground_truth
  - system3_refuses_non_spline_class_pairs
  - coordinate_selection_follows_frozen_rule
  - krawczyk3_certifies_fixture_well_conditioned_root
  - krawczyk3_refuses_determinant_spans_zero_fixture
  - krawczyk3_refuses_conditioning_fixture
  - krawczyk3_negative_orientation_fixture_flips_det_sign
  - krawczyk3_strict_inclusion_only
```

## Section 1 — the square-system constructor (`src/ssi.rs`, NEW)

From two certified-admitted Bézier patches (control grids + weights as
rational tensor-Bernstein), construct the surface-surface difference
system exactly as the shim froze it: the cross-multiplied homogeneous
system `F_k = W2*P1_k − W1*P2_k` (k ∈ x,y,z) over `(u,v) × (s,t)`,
stored through `SquareSystem3::new`'s refusing constructor (ragged /
empty / non-finite / degree-0 refuse — the shim's constructor does the
refusing; you feed it).

- The per-box F3 square reduction: build a `SquareSystemInput`
  (contract.rs, FROZEN — import, never restate) from the system and a
  box, and select the continuation coordinate by the frozen
  `select_continuation_coordinate` rule exactly: largest relative
  margin, lowest index on ties, `ConditioningBelowThreshold` refuses,
  never a weaker retry.
- Jacobian minors as Bernstein-patch enclosures through the landed
  hull kernels (`hull_bernstein_1d/_2d`, derivative patches — the D2
  public API). Refusals map through the hull vocabulary:
  `EnclosureUnavailable` / `DomainNotCompact`.
- Class pairs outside spline-admissible shapes refuse
  `UnsupportedPairClass` (the DISPATCH widening; a named variant, not a
  string).
- The 2D module (`formal/bezier_isect.rs`) is PRIOR ART to copy in
  shape: its identity rules and fail-closed typing carry over; its
  private helpers stay private (the HULL precedent — solver internals
  stay solver-private).

## Section 2 — the 3×3 Krawczyk certificate (same module, continues Section 1)

The 2×2 Krawczyk inner loop dimension-raised, in the same file:

- Jacobian inverse via the adjugate/determinant over `CertifiedInterval`
  under directed rounding (the landed `formal/exact.rs` /
  `formal/numeric.rs` interval machinery; the determinant's enclosure
  strictly away from zero is the certificate's precondition, mirroring
  the 2D code's own structure).
- `K(X)` enclosure over directed rounding; the inclusion test
  `K(X) ⊆ int(X)` component-wise STRICT; only a valid inclusion emits a
  `KrawczykCertificate3` through the shim's strict-inclusion-only
  constructor (non-strict/boundary refuses — the frozen emission rule
  made typecheckable; do not bypass it).
- The 2D module's typed-unresolved discipline carries over verbatim:
  every non-result is a named refusal, no catch-all.

## Section 3 — tests (`truck-certified/tests/ssi_system.rs`, NEW)

Consume the shim's fixture kit through the crate's public path
(`truck_certified::ssi_fixtures`; it is `#[doc(hidden)] pub` test
support on purpose). The eight `tests_required` functions:

1. `system3_constructor_matches_fixture_ground_truth` — construct the
   system for `well_conditioned_root()`'s patches; the stored grids
   round-trip and the cross-multiplied F_k evaluate to the stated
   ground truth at the root and its neighbors.
2. `system3_refuses_non_spline_class_pairs` — a non-spline-admissible
   class pair refuses `UnsupportedPairClass` (named variant).
3. `coordinate_selection_follows_frozen_rule` — on the fixture box the
   selection returns the frozen rule's answer; a box whose every
   coordinate margin fails refuses `ConditioningBelowThreshold`
   (the `conditioning_below_threshold()` fixture).
4. `krawczyk3_certifies_fixture_well_conditioned_root` — the certificate
   constructs at the fixture root box; the box, K(X), and determinant
   enclosure are component-wise consistent with the fixture's stated
   ground truth.
5. `krawczyk3_refuses_determinant_spans_zero_fixture` —
   `determinant_spans_zero()` refuses (the det-enclosure-excludes-zero
   precondition is part of the certificate's construction, not a later
   check).
6. `krawczyk3_refuses_conditioning_fixture` — the conditioning fixture
   refuses through the frozen coordinate rule before any Krawczyk work
   (fail-closed ordering: refuse early, refuse named).
7. `krawczyk3_negative_orientation_fixture_flips_det_sign` —
   `negative_orientation_root()` certifies with the determinant sign
   flipped (the orientation certificate's other branch).
8. `krawczyk3_strict_inclusion_only` — a hand-built non-strict
   inclusion (K(X) touching the boundary) refuses through the shim
   constructor; the boundary case is constructed in the test, not
   found by search.

## Wave-mode rules for this worker (binding)

- LOCAL checks only: `cargo check -p truck-certified`,
  `cargo test -p truck-certified --lib --tests`, scoped clippy on your
  diff, fmt on your files. Do NOT run workspace-wide gates, do NOT run
  verify.py, do NOT touch other crates.
- The write set is exactly the yaml. lib.rs takes ONE line
  (`pub mod ssi;` beside `pub mod ssi_types;` — a one-line textual
  conflict with W2's line is expected and resolved at integration).
- H-1: crate-level `deny(clippy::unwrap_used)` covers the new module;
  no `unwrap`/`expect`/`panic!`, no module-level allow.
- F3 is law: the selection rule, the both-certificate switching
  discipline (a TRACE concern, not yours), and no-weaker-retry are
  implemented exactly as frozen; a needed widening is an orchestrator
  spec edit, never a worker decision.
- If you discover a real contract ambiguity in the shim's types, STOP
  and SPEC_GAP it (commit nothing beyond WIP evidence) — do not invent
  an interpretation; the answer gets frozen in the shim and amended.

## Done-when

- The eight `tests_required` functions exist and pass
  (`cargo test -p truck-certified --lib --tests --no-fail-fast` green,
  landed suites unchanged).
- fmt clean on your files; clippy `-p truck-certified` zero findings
  attributable to your diff.
- `cargo check --workspace --all-targets` green (the lib.rs line ripple).
- RESULT.json AT THE WORKTREE ROOT (claim, not verdict), commit on the
  current branch first (subject: `feat(certified): SSI square-system
  constructor + 3x3 Krawczyk certificate (BG-CK-P2-SYSTEM)`).

## Stop conditions

Stop, commit nothing beyond WIP evidence, write RESULT.json AT THE
WORKTREE ROOT with the finding verbatim if:

1. The shim's landed types differ from this packet's read (a frozen
   shape moved between shim authoring and your dispatch) — stop, do not
   adapt silently.
2. A fixture's stated ground truth cannot be reproduced by the
   constructor + certificate (a derivation mismatch) — record the
   numbers; the fixture kit is frozen, the mismatch is the finding.
3. The 3×3 adjugate/determinant path needs an interval primitive the
   landed `formal/` does not have — record exactly which operation and
   where the 2D code solved the same need; do NOT add numeric
   primitives ad hoc.
