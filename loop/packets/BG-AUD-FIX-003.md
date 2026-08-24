# WORK PACKET BG-AUD-FIX-003 — wedge certificate soundness (AUD-003)

You are repairing one defect found by the formal-kernel correctness audit
`loop/audits/BG-AUDIT-001.md` (finding AUD-003), in
`truck-topology/src/invariants/wedge.rs`. Everything you need is in this
document. **Do not read any other spec file** — this packet is self-contained.

```json
{"id":"BG-AUD-FIX-003","status":"DONE","contracts":["AUD-003"],
 "tests_added":2,"deviations":[],"disagreements":[],
 "baseline_failures":[],"notes":"free text"}
```

Fill `deviations`, `disagreements` and `baseline_failures` as arrays, empty if
empty. **If anything below contradicts what you find in the code, say so in
`disagreements` rather than making the code match the packet.**

```yaml
id:          BG-AUD-FIX-003
contract:    [AUD-003]
class:       design
crates:      [truck-topology]
write_allow:
  - vendor/truck/truck-topology/src/invariants/wedge.rs
read_allow:
  - vendor/truck/truck-topology/src/shell.rs
  - vendor/truck/truck-base/src/evidence.rs
tests_required:
  - wedge_singular_midpoint_normal_refuses
  - wedge_singular_endpoint_with_finite_midpoint_refuses
budget:      {turns: 40, ctx_tokens: 100000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub mod wedge' vendor/truck/truck-topology/src/invariants/mod.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'WedgeNonDegeneracy' vendor/truck/truck-base/src/evidence.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'fn check' vendor/truck/truck-topology/src/invariants/wedge.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'fn surface_normal' vendor/truck/truck-topology/src/invariants/wedge.rs"}
  - {id: A5, expect: 1, cmd: "grep -c 'fn test_edge' vendor/truck/truck-topology/src/invariants/wedge.rs"}
```

## Problem

`wedge::check` certifies `Prop::WedgeNonDegeneracy` by sampling each interior
edge at its parameter **midpoint**, projecting onto both faces and requiring
`|n_A × n_B| >= sin_margin`. Two defects:

1. **NaN silent-pass (the core soundness hole).** `surface_normal` returns
   `surface.normal(u, v).normalize()`. At a singular surface point — a cone
   apex (`Cone::normal = Vector3::zero()` there), a sphere pole, any
   vanishing-partial point — `normalize()` of the zero vector is `NaN`, and
   `NaN < sin_margin` is **false** in IEEE, so the refusal arm never fires and
   the edge is certified non-degenerate. A knife edge / crack at a singular
   point is silently certified. Probe confirmed on this tree: `wedge::check`
   on a cone-apex-midpoint edge returns `OK`.

2. **Single-sample scope.** Even with a finite midpoint normal, a fold/crack
   elsewhere on the edge is invisible to a single sample, yet the certificate
   sets the whole-edge `Prop::WedgeNonDegeneracy`.

## The two required regressions (both must FAIL on the buggy code first)

Construct both with a new test-only `Cone` mirror surface in the test module of
`wedge.rs` (see below), paired with the existing `Plane` mirror.

1. `wedge_singular_midpoint_normal_refuses` — the shared edge's MIDPOINT maps
   to the cone apex. Build a two-face shell: face A = `Cone` mirror (apex at
   the origin, half-angle 45°), face B = `Plane` mirror. The shared edge is the
   `Line` from `(1,0,1)` to `(-1,0,-1)` — a cone generator, so every point lies
   on the cone, and its midpoint `(0,0,0)` is the apex. Face B's plane contains
   that line (e.g. the plane through `(1,0,1)`, `(-1,0,-1)` and `(0,1,0)`).
   The other two edges of each triangular face complete the wires. With
   `sin_margin = 0.5`, `wedge::check` must return `Err(NumericallyUnresolved)`
   — the singular normal at the apex must REFUSE, never certify.

2. `wedge_singular_endpoint_with_finite_midpoint_refuses` — the shared edge
   from `(0,0,0)` (the apex) to `(1,0,1)` (a cone point). Its midpoint
   `(0.5,0,0.5)` is on the cone with a FINITE normal and a well-defined wedge
   against the plane (the old midpoint-only code certifies a hold here), but
   its `t = 0` endpoint is the apex. After the fix, the checker samples the
   endpoints too and must REFUSE (`NumericallyUnresolved`).

**Do this in order:** add the tests, watch them FAIL on the current code
(regression 1 fails because it certifies instead of refusing; regression 2
fails because the midpoint-only sampling certifies a hold), then implement the
repair and watch them PASS. Record the pre-fix failing observation in
`RESULT.json.notes`.

## Repair

1. **NaN guard (mandatory).** In `surface_normal`, compute the normal's
   magnitude yourself and refuse `NumericallyUnresolved` when it is not finite
   or is zero (do NOT rely on `normalize()`). In `test_edge`, additionally
   refuse when `sin_angle` is not finite. The refusal is the existing
   `unresolved()` helper.

2. **Strengthened sampling (decided).** Sample the edge at `t0`, `t_mid` and
   `t1` (the parameter-range endpoints and the midpoint), not only `t_mid`.
   Every sample must project onto BOTH faces and clear the margin with finite
   normals; any failure is a refusal. This is still a float certificate
   (`method: Method::Float`), deliberately NOT an interval certificate.

3. **Explicit scope (owner-approved amendment, recorded here).** The
   certificate's claim is the wedge condition at the SAMPLED parameters, not
   the whole edge. A whole-edge interval certificate is not expressible through
   this checker's generic bounds — `S` is bounded by
   `ParametricSurface + ParametricSurface3D + SearchParameter` (wedge.rs:253-257),
   which has no interval-normal capability, and the whole-span path additionally
   needs the edge's parameter image on each face (the pcurve, BG-CE-001's
   unwired payload). Update the module doc so this scope is stated precisely,
   and record the API-bound limitation in `RESULT.json.notes`. The existing
   prose already documents v1 as midpoint-sampled; extend it to the
   three-sample scope and the "not a whole-edge claim" caveat.

The pre-existing tests — `wedge_right_angle_tent_holds`,
`wedge_folded_coplanar_faces_violate`, `wedge_doubled_back_faces_violate`,
`wedge_boundary_edge_is_skipped`, `wedge_projection_failure_is_unresolved` —
must stay green (the tent still clears the margin at all three samples; the
fold/crack witnesses fold at every point of the shared edge, so all samples
refuse).

### The test-only `Cone` mirror (design given, you transcribe)

In the `mod tests` block of `wedge.rs`, add a `Cone` mirror implementing the
same traits as the existing `Plane` mirror:
`ParametricSurface` (`subs(u,v) = apex + v*tan(half_angle)*(cos u, sin u, 0) +
(0,0,v)`, `uder`/`vder` the partials), `ParametricSurface3D` (`normal(u, v)`:
the zero vector when `v == 0.0`, else the finite cone normal, sign-following
`v`), and `SearchParameter<D2>` (inverse: `v = r.z`, `u` from `atan2` of the
radial direction; at the apex `radial == 0` return `(0.0, 0.0)`). Mirror the
existing `Plane`'s code style. `half_angle = π/4` gives `tan = 1`.

## H-3, the house rule that rejects bare float literals

GATE-2 of `scripts/kernel-gates.sh` fails any **added** line carrying a bare
`1e-N`-shaped literal unless that line ends with an `// H-3` comment. The
witnesses use `0.0`/`1.0`/`0.5` (no match). If you must write a small float,
add the same-line `// H-3` comment. Run `bash scripts/kernel-gates.sh <your
base commit>` yourself before writing `RESULT.json`.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-topology
cargo clippy -p truck-topology --all-targets --no-deps
cargo test -p truck-topology --lib --tests --no-fail-fast
cargo test -p truck-topology --doc
cargo check --workspace --all-targets
bash scripts/kernel-gates.sh <your base commit>
```

Never run a bare `cargo test`.

## Forbidden

Editing any file outside `write_allow`. Emitting a positive `WedgeNonDegeneracy`
certificate from a non-finite normal. Weakening the refusal shape. Adding
`#[ignore]`. Trying to build a whole-edge interval certificate through the
generic `S` bounds (it is not expressible — that is the documented
API-bound limitation, not a task).

## Stop conditions

- an anchor count differs → `ANCHOR_MISMATCH`, naming the file and what you saw
- the `Cone` mirror cannot be expressed against the real trait signatures →
  `SPEC_GAP`, with the exact mismatch
- the strengthened sampling or scope amendment requires an owner-level semantic
  choice beyond what this packet decides → `SPEC_GAP`, with the precise open
  choice
- three consecutive failed `cargo` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit on the current branch with subject
`fix(topology): refuse wedge certificates on non-finite normals; sample edge endpoints (BG-AUD-FIX-003)`.
