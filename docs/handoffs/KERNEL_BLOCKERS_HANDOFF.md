# Handoff: kernel blockers landed, BG-S0-001 in flight

**Status date:** 2026-08-15

## What the spec is

`docs/GENERATION_KERNEL_BUILD_SPEC.md` is the authoritative source of truth for
the B-rep generation kernel build (blockers P-1..P-7, Stages 0-3). It is
complete and self-contained: every item names its anchors, its contracts, and
its tests. Read it, then read only the code it points at. It references
`docs/FORMAL_SYSTEM_BREP_GENERATION.md` and `docs/TRUCK_GENERATION_AUDIT.md`
for theory and the audit that motivated each item.

## What is DONE (all recorded in the spec, marked `DONE`)

- **P-1 — vendored truck.** The tree lives at `vendor/truck/` (rev
  `c5f4b6e9778e0721a1d446f10568eb5e5594e8ed`), 11 crates as path deps in
  `Cargo.toml`; the `.cargo/config.toml` `paths` override block is **deleted**.
  A clean clone builds (`cargo check --locked --all-targets`), `cargo tree`
  shows no git source for any truck crate. Edit truck code in-tree.
- **P-2 — `Outcome<T>` shape.** `Result<Certified<T>, Refusal>`; written into
  BG-EVD-001.
- **P-3 — CI gates.** `scripts/kernel-gates.sh` wired into
  `.github/workflows/cross-platform.yml`; diff-scoped to `vendor/truck/**`,
  validated in isolation, no-ops until vendoring lands on the baseline branch.
- **P-5 — interval crate.** `inari 2.0.0`, `default-features = false`. x86_64
  AVX+FMA flag set in `.cargo/config.toml` (target-scoped; float results stay
  bit-identical). **Benchmark claims must be re-validated after this.**
- **P-6 — reference implementation.** `vendor/truck/truck-evidence/` (wired as
  a look dev-dep): BG-EVD-001 evidence algebra, BG-ENC-001 enclosure
  interface, BG-ENC-002 for `Plane`, shared harness. 16 tests green,
  clippy-clean under the deny lints. This is the template every later item
  copies.

Verification that passed as of handoff: `cargo fmt --all -- --check`,
`cargo clippy --all-targets`, `cargo test --all-targets` (exit 0). Note: a
parallel `cargo test` right after a `cargo clean` raced on incremental state
and failed spuriously; `-j 1` was clean. Use `-j 1` after any `cargo clean`.

## In flight: BG-S0-001 (the next agent's job)

The user approved the **full `Outcome<bool>` migration** of the `IncludeCurve`
trait (not the minimal no-abort fix). Current state:

- The trait is `truck-geotrait/src/traits/...`, `IncludeCurve<C>` returning
  `bool`. The spec's algorithm and tests need `Outcome<bool>`.
- `truck-evidence` currently owns `Outcome`, but it depends on
  `truck-geometry`, and `truck-geotrait` is a leaf both geometry and modeling
  build on — so a direct dep creates a cycle. **Decision to implement: move
  the evidence algebra to `truck-base::evidence`** (the spec explicitly allows
  `truck-base::evidence` as the home) and have `truck-evidence` re-export it.
- Then migrate the ~22 `include` impls (listed in the spec / findable via
  `impl.*IncludeCurve` under `vendor/truck/`) and the callers, and implement
  the six `IntersectionCurve` arms in
  `vendor/truck/truck-modeling/src/geometry.rs` per the spec's algorithm
  (surface-identity short-circuit, leader-polyline sampling, then
  `NumericallyUnresolved`).

## Ground rules

- House rules H-1..H-7 (spec §0) apply to every new item; the P-3 gates
  enforce them on new kernel code.
- Every commit names the `BG-` IDs it discharges.
- Run the verification in `AGENTS.md` before committing; use `-j 1` after a
  clean.
- Do not move golden images / performance claims to make tests pass.
