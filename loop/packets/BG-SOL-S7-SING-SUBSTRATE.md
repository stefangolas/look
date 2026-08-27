# WORK PACKET BG-SOL-S7-SING-SUBSTRATE - Hessian and degenerate-point substrate

Extend the `ImplicitField` substrate with the two primitives the
singular-event stage needs: sound Hessian enclosures and exact isolated
degenerate points. If live code contradicts this packet, report it in
`disagreements`.

```json
{"id":"BG-SOL-S7-SING-SUBSTRATE","status":"DONE","contracts":["BG-SOL-S7-SING-SUBSTRATE"],
 "tests_added":0,"sites_migrated":0,"sites_deferred":0,
 "deviations":[],"disagreements":[],"baseline_failures":[],"notes":"free text"}
```

```yaml
id:          BG-SOL-S7-SING-SUBSTRATE
contract:    [BG-SOL-S7-SING-SUBSTRATE]
class:       design
crates:      [truck-evidence]
write_allow:
  - vendor/truck/truck-evidence/src/contact/implicit.rs
read_allow:
  - vendor/truck/truck-evidence/src/contact/gff.rs
  - vendor/truck/truck-evidence/src/contact/mod.rs
  - vendor/truck/truck-evidence/src/enclosure.rs
tests_required:
  - hess_matches_grad_finite_difference
  - hessian_is_constant_where_claimed
  - degenerate_points_report_cone_apex_only
budget:      {turns: 25, ctx_tokens: 80000}
anchors:
  - {id: A1, expect: 0, cmd: "grep -c 'fn hess' vendor/truck/truck-evidence/src/contact/implicit.rs"}
  - {id: A2, expect: 0, cmd: "grep -c 'fn degenerate_points' vendor/truck/truck-evidence/src/contact/implicit.rs"}
  - {id: A3, expect: 5, cmd: "grep -c 'impl ImplicitField for' vendor/truck/truck-evidence/src/contact/implicit.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'pub trait ImplicitField' vendor/truck/truck-evidence/src/contact/implicit.rs"}
  - {id: A5, expect: 7, cmd: "grep -c 'fn regular_on' vendor/truck/truck-evidence/src/contact/implicit.rs"}
```

## Problem

The singular-event stage (next packet) certifies isolated tangency points
with the 4-D Lagrange system `[f1, ∇f2 + λ∇f1]`, whose Jacobian contains
`Hess(f2) + λ·Hess(f1)`. The current trait exposes only `implicit`, `grad`,
and `regular_on`. It also needs each carrier's exact isolated degenerate
points (where ∇f = 0 ON the zero set) to detect carrier-degenerate contact
before running any solver. This packet adds exactly those two primitives and
nothing else: no solver logic, no certificates, no new `Method`.

## Decisions already made

### 1. `hess` - sound Hessian enclosures, row-major

Add to the trait:

```rust
/// Sound interval enclosure of the Hessian of f over the box, row-major:
/// `hess(p)[r][c]` encloses `d2f/dx_r dx_c` over every point of `p`.
fn hess(&self, p: &Box3) -> [[Interval; 3]; 3];
```

All five impls, exact formulas (translation-invariant, so apex/center
offsets do not appear in second derivatives):

- `Plane`: all zeros (`f` is linear).
- `Sphere`: `2I` - the constant matrix with 2 on the diagonal, 0 elsewhere.
- `Cylinder`: `diag(2, 2, 0)` (the axial direction is free).
- `Cone`: `diag(2, 2, -2*t*t)` with `t = half_angle().tan()`, constant.
- `Torus`: from `f = g*g - 4R2*h` with `g = x'^2+y'^2+z'^2+R^2-r^2`,
  `h = x'^2+y'^2`, `nabla g = 2(x',y',z')`:
  `Hess(f) = 2*nabla g * nabla g^T + 4g*I - 8R^2*diag(1,1,0)`,
  i.e. entry `[i][j]` is `8*x'_i*x'_j + 4g*(i==j) - 8R^2*(i==j && i<2)`,
  all computed from interval enclosures of `g` and `x'_i` over the box.

Every entry is plain sound interval arithmetic (BG-ENC-001): the true
second derivative of EVERY point in the box lies inside the returned
interval. Constants wrapped as degenerate intervals are exact.

### 2. `degenerate_points` - exact isolated on-surface critical points

Add to the trait:

```rust
/// Exact isolated points of the carrier's zero set where grad f = 0.
/// Positive-dimensional degenerate loci are NOT enumerated: the torus with
/// small_radius == large_radius/2 is degenerate along its whole inner
/// equator circle, and this method returns empty for the torus; callers
/// must not conclude "no degenerate locus" from an empty result.
fn degenerate_points(&self) -> Vec<Point3>;
```

- `Cone`: exactly `[apex()]` - the apex is on the zero set with `∇f = 0`.
- `Plane`, `Sphere`, `Cylinder`: empty (their `∇f = 0` sets - the plane
  none, sphere/cylinder centers/axes - are strictly off the zero set).
- `Torus`: empty, with the documented caveat above (the r = R/2 inner
  equator circle is a real degenerate locus the method does not report).

The doc comment in code must carry the torus caveat verbatim in substance.

### 3. No behavior change anywhere else

`implicit`, `grad`, and `regular_on` bodies stay byte-identical. No new
imports beyond what the formulas need. The module doc gains one sentence
naming the two new primitives. Nothing outside `implicit.rs` changes; the
only `ImplicitField` impls in the tree are the five in this file (A3).

## Tests required

1. `hess_matches_grad_finite_difference`: for sphere, cylinder, cone, torus,
   and plane at nondegenerate probe points, each Hessian entry agrees with a
   central difference of the corresponding `grad` component,
   `(grad_j(p + h*e_i) - grad_j(p - h*e_i)) / 2h`, within a named
   truncation slack. Reuse the existing `central_diff`/`fd_match` test
   helpers' pattern (H-3 comments on every float literal; `FD_H`-style named
   constants already exist - reuse or add siblings, do not inline).
   The torus probe must use a point where `g != 0`.
2. `hessian_is_constant_where_claimed`: sphere Hessian equals the `2I`
   enclosure at two different point-boxes (identical degenerate intervals);
   cylinder is `diag(2,2,0)`; cone with `half_angle() = (3/4).atan()` is
   `diag(2, 2, -9/8)` exactly (`-2*t*t = -2*9/16`); plane is all zeros.
3. `degenerate_points_report_cone_apex_only`: the cone returns exactly its
   apex (coordinate equality); sphere, cylinder, plane, and torus return
   empty.

Preserve every pre-existing test function name. H-3 rejects an added bare
`1e-N` unless the same line has a `// H-3` comment.

## Done when

```console
cargo fmt --check -p truck-evidence
cargo clippy -p truck-evidence --all-targets --no-deps
cargo check --locked -p truck-evidence --all-targets
cargo test -p truck-evidence --lib contact::implicit --no-fail-fast
bash scripts/kernel-gates.sh <your base commit>
```

Never run bare `cargo test` or a workspace-wide cargo command.

**Commit your work on the current branch** (subject
`contact: Hessian and degenerate-point substrate for ImplicitField
(BG-SOL-S7-SING-SUBSTRATE)`) **before** writing `RESULT.json`: the verifier
measures the committed diff, and an uncommitted tree reads as an interrupted
run.

## Forbidden

Editing outside `write_allow`; changing `implicit`, `grad`, or `regular_on`
bodies or signatures; adding solver logic, certificates, events, or locus
types; editing `gff.rs` or the dispatcher; adding dependencies; claiming the
torus has no degenerate locus; adding `#[ignore]`; loosening a gate; changing
the GATE-4 ceiling; renaming or deleting a pre-existing test.

## Stop conditions

- anchor mismatch -> `ANCHOR_MISMATCH` with observed count;
- a Hessian formula disagrees with the finite difference of `grad` beyond
  truncation slack -> `SPEC_GAP` with the measured entry;
- three consecutive cargo failures with one cause -> `BLOCKED`.

Finish by writing `RESULT.json` in the worktree root, not `loop/results/`.
Record in `notes` the machine-checked Hessian witness value for the torus at
`center (0,0,0), R = 2, r = 0.5, probe (2, 1, 0)`: the expected enclosure
contains `[[35, 16, 0], [16, 11, 0], [0, 0, 35]]` (computed from the exact
formula `8*x'_i*x'_j + 4g*delta - 8R^2*diag(1,1,0)` with `g = 8.75`).
