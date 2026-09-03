# BG-KV2-102-LEAF — BezierLeaf: knot-span extraction + CertifiedPatch implementation

Wave-1 implementation packet (build spec §4). Implements the §3.2 Bézier
leaf for real: extraction of homogeneous Bézier control nets from landed
B-spline/NURBS spans, and the `CertifiedPatch`/`CertifiedPatchC2`
implementations via interval Bernstein evaluation. **Reuses the shim types
(`kernel::leaf::BezierLeaf`, `kernel::patch::*`) — never restates them.
Reuses the landed hull kernels (`hull.rs` interval de Casteljau, derivative
coefficients) — never reimplements Bernstein arithmetic.**

No solver bodies here: enclosures, regularity, and weight bounds only.
Tracing/certificates over leaves are later waves.

**H-1.** New module `leaf_extract.rs` carries the crate's
`#![deny(clippy::unwrap_used)]` discipline (crate-level deny covers it): no
`unwrap`/`expect`/`panic!`, no module-level `allow`. Copy the header style
from `hull.rs`.

```yaml
id:          BG-KV2-102-LEAF
contract:    [BG-KV2-102-LEAF]
class:       design
crates:      [truck-certified]
depends_on:  [BG-KV2-000-CONTRACT]
write_allow:
  - vendor/truck/truck-certified/src/kernel/leaf_extract.rs
  - vendor/truck/truck-certified/src/kernel/leaf.rs
  - vendor/truck/truck-certified/src/kernel/mod.rs
  - vendor/truck/truck-certified/tests/kernel_leaf.rs
read_allow:
  - docs/CONSTRUCTIVE_GEOMETRY_KERNEL_SPEC_V2.md
  - docs/KERNEL_V2_BUILD_SPEC.md
  - vendor/truck/truck-certified/src/kernel
  - vendor/truck/truck-certified/src/hull.rs
  - vendor/truck/truck-certified/src/formal/exact.rs
  - vendor/truck/truck-geometry/src/nurbs
  - vendor/truck/truck-geometry/src/decorators
budget:      {turns: 34, ctx_tokens: 110000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub struct BezierLeaf' vendor/truck/truck-certified/src/kernel/leaf.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub trait CertifiedPatch' vendor/truck/truck-certified/src/kernel/patch.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'pub mod kernel;' vendor/truck/truck-certified/src/lib.rs"}
  - {id: A4, expect: 0, cmd: "grep -c 'leaf_extract' vendor/truck/truck-certified/src/kernel/mod.rs"}
  - {id: A5, expect: 2, cmd: "grep -c 'fn hull_bernstein_1d\\|fn hull_bernstein_2d' vendor/truck/truck-certified/src/hull.rs"}
  - {id: A6, expect: 1, cmd: "grep -rnw 'fn knot_vec' vendor/truck/truck-geometry/src/nurbs | head -1 | wc -l"}
tests_required:
  - leaf_extraction_round_trips_a_bezier_patch_exactly
  - leaf_extraction_splits_a_bspline_span_into_bezier_leaves
  - enclose_contains_every_sampled_point
  - derivs_enclose_finite_difference_derivatives
  - regularity_proven_on_regular_patch_and_disproven_on_degenerate
  - weight_bound_proven_on_positive_leaf_and_refuses_straddle
  - c2_second_derivs_enclose_second_finite_differences
  - no_transcendental_call_in_leaf_module
```

## Section 1 — `kernel/leaf_extract.rs` (NEW)

- `pub fn extract_bezier_leaves(s: &NurbsSurface or &BSplineSurface) ->
  Construction<Vec<BezierLeaf>>` — per knot-span extraction into
  homogeneous Bézier nets (knot insertion to full multiplicity, the
  standard Bézier-extraction; implement it directly on the control net —
  the landed `nurbs` crate carries knot vectors and control points; do NOT
  add dependencies). Refuses: non-finite data, non-positive weights
  (`RefusalKind::WeightDegenerate`, Disproven), degree 0.
  FROZEN SIGNATURE (consumed by Wave-2 S-packets): takes any
  `ParametricSurface3D + SearchParameter`-compatible landed surface via a
  generic bound OR concrete `&NurbsVector4`-style input — pick ONE and
  record it in RESULT notes; do not expose both.
- `pub fn leaf_from_control(degree_u, degree_v, control: Vec<[f64; 4]>) ->
  Construction<BezierLeaf>` — direct constructor pass-through for clients
  that already hold Bézier nets (the fixture kit uses this).
- Affine reparameterization of a leaf's domain (`reparam(affine: [[f64;2];
  2]) -> BezierLeaf`) — leaf restrictions per §4.2 Rule B are affine with
  float coefficients; this is the primitive Rule B transports.

## Section 2 — `kernel/leaf.rs` (extends the shim file: impl blocks ONLY)

`impl CertifiedPatch for BezierLeaf` and `impl CertifiedPatchC2 for
BezierLeaf`:
- `enclose` — interval Bernstein range over the box via the landed hull
  kernels (homogeneous; divide by the certified-positive w-enclosure ONCE
  at the end, N5/N6 — the denominator bound is `weight_bound`'s job and is
  computed first internally; if it refuses, `enclose` refuses
  `WeightDegenerate` with backing Disproven when the w-enclosure provably
  contains 0, Inconclusive otherwise — §7.1 verbatim).
- `derivs` — interval evaluation of the derivative control nets (§3.2: the
  leaf caches derivative nets; cache in the impl via interior
  `OnceCell`-free plain fields if needed — or recompute per call and record
  the choice; NO interior mutability without a recorded reason).
- `normal_cone` — cone over the cross-product enclosure of `derivs`
  (cone construction from a vector enclosure; the landed
  `enclosure.rs:103-123` cone constructor is the shape reference, but this
  module may not depend on truck-evidence — implement the small local
  constructor from the shim `Cone` type).
- `regularity` — EG − F² enclosure from `derivs`;
  `ClaimVerdict::Proven(CertifiedPositive)` iff the lower bound > 0;
  `Disproven(Degeneracy)` iff upper < 0; else `Inconclusive(Degeneracy)`.
- `weight_bound` — homogeneous w-control-net interval range over the box:
  lower > 0 → `Proven`; upper < 0 → `Disproven(Pole)` (§7.1 backing rule);
  straddles → `Inconclusive(Pole)`. `None` is NEVER returned by BezierLeaf
  (it is a rational leaf by construction).
- `second_derivs` — second-derivative nets, same discipline.

## Section 3 — tests (`tests/kernel_leaf.rs`, NEW)

Fixture ground truths (machine-checked, reuse `kernel::fixtures` where it
fits — the kit's `leaf_from_control` path):
1. Round-trip: a rational Bézier patch extracted from itself is the
   identity (control nets equal within exact equality of the inputs).
2. A degree-2 B-spline in u with interior knot splits into exactly 2
   leaves whose union reproduces sampled surface points (sample grid,
   containment in `enclose` per leaf).
3. Every sampled surface point lies in its leaf's `enclose` (random grid,
   fixed seed, recorded).
4. `derivs` enclosures contain finite-difference derivative estimates.
5. Regularity: Proven on a plane-ish regular leaf; Disproven on a leaf
   with a collapsed edge (two identical control rows → the cross product
   vanishes; construct it directly).
6. Weight bound: Proven on all-positive weights; the kit's
   weight-straddles-zero data (1,−1,1) refuses/Inconclusive per §7.1.
7. C2: second finite differences inside `second_derivs` enclosures.
8. `no_transcendental_call_in_leaf_module` — source scan of
   `leaf.rs`+`leaf_extract.rs` for `sin|cos|atan2|exp|ln|log|powf|sqrt`
   outside comments (N4; kernel-gates will enforce this — the test is the
   worker-time guard).

## Done-when

- `cargo check -p truck-certified --all-targets` green (CARGO_BUILD_JOBS=2-4;
  sccache wrapper; unset locally if it rejects incremental — record in
  RESULT notes).
- `cargo test -p truck-certified --lib --tests --no-fail-fast` green
  (landed + shim suites unchanged, plus these).
- `cargo fmt` clean; clippy `-p truck-certified --all-targets
  --message-format=short --no-deps` zero findings attributable to the
  packet's files (run unfiltered, fix ALL).
- `cargo check --workspace --all-targets` green (the crate has downstream
  consumers; the module is additive but verify the ripple once).

## Stop conditions

1. The shim's `BezierLeaf`/`CertifiedPatch` shapes differ from the quoted
   contract (a frozen shape moved post-merge) — stop, record the diff.
2. Knot-span extraction on the landed NURBS type requires a signature
   change OUTSIDE the write set (e.g. missing accessors) — stop, name the
   accessor; that is an amendment, not an improvisation.
3. An enclosure is not tight enough to prove regularity on any regular
   test patch after following the landed hull-kernel discipline — record
   the numbers; do not loosen a tolerance to force Proven.

## Finish by writing `RESULT.json` AT THE WORKTREE ROOT

Commit on the current branch (subject: `feat(certified): BezierLeaf
extraction + CertifiedPatch impl (BG-KV2-102-LEAF)`) BEFORE writing
`RESULT.json`.
