# CC-000-CONTRACT — the construct shim: types, refusals, constants, fixture kit

Shim packet for the CC program (certified construction: loft / offset/shell /
blend). The spine is `docs/CERTIFIED_CONSTRUCTION_CONTRACTS.md` (decisions
C1–C9, seams S1–S12, fixture inventory §6); the theory is
`docs/CERTIFIED_LOFT_AND_SHELL_THEORY_SPEC.md` §1. This packet lands ONLY
frozen types, refusing constructors, constants, the inari bridge, and the
fixture kit — no solver bodies. Every later CC packet types against what
lands here. Its landing merge SHA becomes the CC wave base.

```yaml
id:          CC-000-CONTRACT
contract:    [CC-000-CONTRACT]
class:       design
crates:      [truck-certified]
depends_on:  []
write_allow:
  - vendor/truck/truck-certified/Cargo.toml
  - vendor/truck/truck-certified/src/lib.rs
  - vendor/truck/truck-certified/src/construct/mod.rs
  - vendor/truck/truck-certified/src/construct/refusal.rs
  - vendor/truck/truck-certified/src/construct/convert.rs
  - vendor/truck/truck-certified/src/construct/config.rs
  - vendor/truck/truck-certified/src/construct/stubs.rs
  - vendor/truck/truck-certified/src/construct/fixtures.rs
  - vendor/truck/truck-certified/tests/construct_contract.rs
read_allow:
  - docs/CERTIFIED_CONSTRUCTION_CONTRACTS.md
  - vendor/truck/truck-certified/src/kernel
budget:      {turns: 20, ctx_tokens: 90000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub type Interval' vendor/truck/truck-certified/src/kernel/mod.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub type IBox2' vendor/truck/truck-certified/src/kernel/patch.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'pub enum Refusal' vendor/truck/truck-base/src/evidence.rs"}
  - {id: A4, expect: 0, cmd: "grep -c 'truck-evidence' vendor/truck/truck-certified/Cargo.toml"}
  - {id: A5, expect: 0, cmd: "grep -c 'pub mod construct' vendor/truck/truck-certified/src/lib.rs"}
  - {id: A6, expect: 1, cmd: "grep -c 'pub use inari::Interval' vendor/truck/truck-evidence/src/enclosure.rs"}
  - {id: A7, expect: 1, cmd: "grep -c 'pub fn hull_bernstein_2d' vendor/truck/truck-certified/src/hull.rs"}
tests_required:
  - construct_refusal_variants_are_distinct_and_tagged
  - config_constants_match_the_spine_document
  - inari_conversion_is_an_exact_order_preserving_copy
  - box3_to_ibox_preserves_bounds
  - radius_law_stubs_carry_no_default_construction
  - fixture_ground_truths_hold
```

Section 1: `construct/mod.rs` — `pub type Interval =
crate::formal::exact::CertifiedInterval;` (the SAME alias as
`kernel/mod.rs:50` — do not introduce a second interval type), the child
`pub mod` lines, and a module doc stating the doctrine verbatim: one home
(C1), the single manifest edge `truck-certified → truck-evidence` (C2, added
in this packet to `Cargo.toml` as `truck-evidence = { version = "0.1.0",
path = "../truck-evidence" }` — copy the spelling style of the existing truck-*
lines), no inari in this crate (C3), stub posture (C7), determinism house
rules (C9).

Section 2: `construct/refusal.rs` — `pub enum ConstructRefusal` with EXACTLY
these variants, in this order: `NonPositiveWeightField,
SingularInterpolationSystem, AmbiguousCorrespondence, FocalDegeneracy,
CanalSingular, RankDeficientContact, UnintendedContact, StarNotEmbedded,
NoAdmissibleProjection, NonGenericThicknessEvent, AmbiguousEventOrdering,
InvalidInput, ConditioningBelowThreshold, Unfrozen`. Derive `Debug, Clone,
Copy, PartialEq, Eq`. Plus `pub fn tag(&self) -> &'static str` returning the
variant name (the `MapRefusal::tag` precedent in `certified_map.rs`). Do NOT
add variants; growth is a CC-000 amendment. `Unfrozen` is the refusing-stub
marker (C7) and matches the existing `contract::Refusal::Unfrozen` precedent
(2 occurrences at `contract.rs` — A3-class fact, not an anchor obligation).

Section 3: `construct/config.rs` — five consts with doc comments naming the
spine decision (the kernel/config.rs pattern, A1-class style):
`CC_N_EXACT: usize = 64`, `CC_ETA_J: f64 = 1e-12`, `CC_ETA_PI: f64 = 1e-12`,
`CC_MU_CLEAR: f64 = 1e-9`, `CC_DEPTH_MAX: u32 = 40`.

Section 4: `construct/convert.rs` — the ONLY sanctioned inari bridge:
`pub fn from_inari(i: truck_evidence::enclosure::Interval) -> Interval`
(exact lo/hi field copy — both universes are outward-rounded, so the copy is
order-preserving and adds no width; state that in the doc comment) and
`pub fn box3_to_ibox(b: &truck_evidence::enclosure::Box3) -> IBox<3>`. Access
inari types ONLY through the `truck_evidence::enclosure` re-exports (A6); the
`inari` crate name must NOT appear in any manifest or use-statement of this
crate.

Section 5: `construct/stubs.rs` — the shared stub types from seams
S6/S9/S10/S11/S12 that later packets consume: `RadiusLaw` (enum:
`Constant(f64) | Linear { r0: f64, r1: f64 } | CubicHermite { r0: f64, r1:
f64, m0: f64, m1: f64 } | MonotoneCubic(Vec<(f64, f64)>) |
VertexControl(Vec<f64>)` — theory §5.3's admissible v1 laws), `EventKind`
(enum: `Trim | ThirdFace | Focal | Rank | Collision | Trace` — theory §5.2's
vocabulary), `WireComplex`, `ShiftFunctional`, `BoundaryPlan`, `BranchSeed`
(opaque structs with private fields + refusing constructors returning
`ConstructRefusal::Unfrozen`; their production belongs to CC-013/CC-005/
CC-030 respectively), and `TripleContactNode` with the S11 pub fields
(`centre: [Interval; 3]`, `radius: Interval`, `contacts: [[Interval; 2]; 3]`)
plus a refusing constructor. No production logic anywhere in this file.

Section 6: `construct/fixtures.rs` — `#[doc(hidden)]` test support, the
`kernel/fixtures.rs` pattern (builders return `Result<_, ConstructRefusal>`,
machine-checked ground truths, module doc says TEST SUPPORT ONLY). Land the
eight fixtures of spine §6: `banded_cubic_uniform(n)` (cubic collocation
coefficients for uniform stations over `n+1` sections, `det` sign known),
`banded_pivot_spans_zero()` (first pivot interval contains 0),
`argmin_separated()` / `argmin_overlapping()` (interval arrays with strict
sup&lt;inf resp. overlap), `flat_patch()` / `curved_patch()` /
`degenerate_patch()` (σ&gt;0 with L=0; σ&gt;0 with known positive L; σ
enclosure containing 0 — as `(lo, hi)` f64 pairs plus the expected δ), and
`genuine_star()` / `folded_corner()` (DiskPiece-shaped data: per-piece
determinant lower bounds, seam-glued flags, boundary-simplicity flags).
Fixtures are data + builders ONLY — no solvers may be exercised to build
them.

The orchestrator edits `docs/CERTIFICATE_MAPPING.md` at landing; do NOT
write into `docs/` from the worker.

House rules: **H-1: the new files carry the crate's authored-kernel
discipline — no `unwrap`/`expect`/`panic!` in shipped code, no module-level
`allow`** (the crate already denies `clippy::unwrap_used`). **H-3: float
comparisons in tests take the `// H-3` opt-out ON THE SAME LINE.** **All
cargo invocations go through the queue (the `cargo` on PATH IS the queue
shim). Do not invoke cargo by absolute path; do not unset the shim.** Scoped
checks only: `cargo check -p truck-certified` and `cargo test -p
truck-certified --test construct_contract`. No workspace builds. COMMIT
BEFORE writing RESULT.json AT THE WORKTREE ROOT.

Stop conditions: (1) if the A4/A5 anchors fail on the base tree, STOP and
file QUESTION.md — another packet touched the manifest; (2) if any fixture
ground truth cannot be machine-checked without a solver, record the
fixture as data-only in RESULT notes and mark which consumer packet must
deepen it; (3) read `kernel/mod.rs` and `kernel/fixtures.rs` before writing
anything — the alias, doc style, and fixture pattern are copied, not
reinvented.
