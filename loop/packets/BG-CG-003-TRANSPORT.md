# WORK PACKET BG-CG-003-TRANSPORT — the parallel-transport frame (Bishop, double reflection)

You are landing the fourth frame law of the constructive geometry program
(plan §4, CG-003): `ParallelTransport` — a rotation-minimizing (Bishop) frame
via the double-reflection method. This packet is booked SEPARATELY from
CG-002 and is **never split** (plan §4). The design decisions below are
already made — transcribe them; where the packet books a CONTRACT rather
than a formula, the contract is what is normative and the tests pin it. Do
not read other spec files and do not redesign anything named here. If
something you need is genuinely missing, that is a SPEC_GAP (see "Stop
conditions"): you stop and report, you do not research it.

> **Dispatch gating (orchestrator note).** This packet is written against
> the tree as it will be AFTER BG-CG-002-FRAMES-ANALYTIC lands (the
> dispatcher, the frame modules, and `tests/constructive_frames.rs` are
> CG-002's). Anchors A2 and A5 therefore read 0/absent until CG-002 merges;
> the anchor check at DISPATCH time is the authority, and dispatching this
> packet concurrently with CG-002 is FORBIDDEN (both write `recipe.rs`).

```yaml
id:          BG-CG-003-TRANSPORT
contract:    [BG-CG-003-TRANSPORT]
class:       design
crates:      [truck-geometry]
depends_on:  [BG-CG-001-RECIPE, BG-CG-002-FRAMES-ANALYTIC]
write_allow:
  - vendor/truck/truck-geometry/src/constructive/frame_transport.rs
  - vendor/truck/truck-geometry/src/constructive/recipe.rs
  - vendor/truck/truck-geometry/src/constructive/mod.rs
  - vendor/truck/truck-geometry/tests/constructive_transport.rs
  - vendor/truck/truck-geometry/tests/constructive_frames.rs
read_allow:
  - docs/CONSTRUCTIVE_GEOMETRY_PLAN.md
  - vendor/truck/truck-geometry/src/constructive/mod.rs
  - vendor/truck/truck-geometry/src/constructive/recipe.rs
  - vendor/truck/truck-geometry/src/constructive/frame_fixed.rs
  - vendor/truck/truck-geometry/src/constructive/errors.rs
  - vendor/truck/truck-base/src/tolerance.rs
tests_required:
  - transport_starts_from_orthonormalized_initial_normal
  - straight_spine_has_constant_frame
  - circular_loop_has_trivial_holonomy
  - frame_is_evaluation_order_independent
  - s_spine_survives_inflection
  - parallel_initial_normal_is_singular
  - helix_stays_orthonormal_along_transport
budget:      {turns: 45, ctx_tokens: 110000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub fn frame' vendor/truck/truck-geometry/src/constructive/recipe.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'ParallelTransport { .. } => Err(ConstructError::InvalidInput)' vendor/truck/truck-geometry/src/constructive/recipe.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'pub trait Spine' vendor/truck/truck-geometry/src/constructive/recipe.rs"}
  - {id: A4, expect: 0, cmd: "grep -c 'frame_transport' vendor/truck/truck-geometry/src/constructive/mod.rs"}
  - {id: A5, expect: 1, cmd: "grep -c 'parallel_transport_still_refuses_in_cg002' vendor/truck/truck-geometry/tests/constructive_frames.rs"}
  - {id: A6, expect: 1, cmd: "grep -c 'pub enum FrameLaw' vendor/truck/truck-geometry/src/constructive/mod.rs"}
```

## The existing files you may touch (exactly these changes)

**`constructive/mod.rs`** gains exactly one declaration line next to the
other `mod frame_*;` lines:

```rust
mod frame_transport;
```

Nothing else in mod.rs moves.

**`constructive/recipe.rs`** changes in EXACTLY two places, nothing else:
1. The `ParallelTransport` arm of the `frame()` dispatcher: the
   `Err(ConstructError::InvalidInput)` placeholder is replaced by the call
   into the transport (below). Every other arm of the dispatcher, and every
   other function in recipe.rs, stays byte-identical.
2. The dispatcher's doc comment, if (and only if) it names
   `ParallelTransport` as unimplemented — update that sentence to "Filled
   (BG-CG-003-TRANSPORT)". No other doc changes.

**`tests/constructive_frames.rs`** changes in EXACTLY one place: the landed
test `parallel_transport_still_refuses_in_cg002` is amended IN PLACE (name
kept — session-34 identity rule; the name is historical, it pinned the
CG-002 envelope line that this packet retires): the body becomes the
positive form test 7 below requires, with a one-line comment naming
BG-CG-003-TRANSPORT. Every other landed test stays byte-identical.

## The method (normative shape)

Hanson–Ma **double reflection** over a station polyline: transport the normal
station-to-station by reflecting it across two planes per transition (first
across the plane perpendicular to the incoming chord, then across the plane
bisecting the incoming and outgoing chord directions), which produces the
rotation-minimizing (twist-free to O(h²)) frame that is stable at zero
curvature and through inflections. The exact per-transition formulas are the
canonical ones from that method — implement them faithfully; where two
equivalent spellings exist, pick the one that never divides by a quantity
that can be zero (guard the degenerate transition: collinear consecutive
chords transport the normal UNCHANGED). The packet deliberately does NOT
freeze a formula transcription; it freezes the CONTRACT below, and the
tests pin the contract. If you find the canonical method ambiguous in a way
that changes observable behavior, that is a SPEC_GAP.

## The evaluation contract (frozen; every clause pinned by a test)

1. **Station discretization is deterministic from the spine alone.** The
   transport grid is `TRANSPORT_STATIONS = 64` uniform stations over the
   spine's FULL `domain()`, plus the queried `s` as the final station when
   `s` is not exactly on the grid. A private const in frame_transport.rs;
   never derived from caller state, never cached across calls with different
   spines. Consequence: `frame(s)` costs O(64) closed-form steps — no
   Newton, no fitting, no iteration (the §3.3 fast-path contract holds).
2. **Evaluation-order independence.** `frame(s)` depends only on (spine,
   `initial_normal`, s) — never on which frames were computed before, and
   never on mutable state. Two recipes with identical fields give identical
   frames; interleaved queries give the same answers as fresh queries.
3. **Initial frame.** `t̂₀` = unit tangent at the domain start
   (`spine.derivative_at(s_min)`, normalized); `n₀` = `initial_normal`
   orthonormalized against `t̂₀` and normalized. Refuse
   `ConstructError::FrameSingular { at: s_min (reported as the QUERIED s),
   law: "ParallelTransport" }` when `initial_normal` is non-finite, zero, or
   parallel to `t̂₀` within `DirectTolerance::default().position` (i.e. the
   orthonormalized residual magnitude is ≤ that bound).
4. **Orthonormality is an invariant, not a hope.** After every transition,
   re-orthonormalize (`n ← n − (n·t̂)t̂`, renormalize) before emitting. The
   frame satisfies the `Frame3` convention everywhere:
   `t × n == b`, unit lengths, within `DirectTolerance::default().position`.
5. **Propagated refusals.** `spine.derivative_at`/`position_at` refusals
   (including `PolylineSpine`'s `SpineNotC1` at corners and out-of-domain
   `InvalidInput`) propagate unchanged; the transport never clamps, never
   smooths, never silently skips a refused station. The zero-tangent refusal
   (`ZeroTangent { at }`) fires exactly like the other laws (CG-002's
   dispatcher already guarantees this — do NOT duplicate the check inside
   frame_transport.rs; the dispatcher hands you a UNIT tangent only for the
   START tangent, which you derive yourself from the spine).
6. **Reported `at`.** FrameSingular's `at` is the QUERIED `s` (the frame is
   refused at the caller's parameter), matching the landed laws' convention.

```rust
//! frame_transport.rs — public(super) entry:
pub(super) fn parallel_transport(
    initial_normal: Vector3,
    spine: &dyn Spine,
    s: f64,
) -> Result<Frame3, ConstructError>
```

(`Spine` is object-safe — `domain`/`position_at`/`derivative_at` only; the
dispatcher calls this as `frame_transport::parallel_transport(initial_normal,
&self.spine, s)` after matching the law payload. If object-safety demands a
different spelling (e.g. a generic parameter instead of `&dyn`), take the
generic form — the dispatcher arm is in your write set; record the choice in
RESULT notes.)

## Tests required — `tests/constructive_transport.rs` (new file)

Header `#![deny(clippy::unwrap_used)]`. Fixtures are test-local `Spine`
impls (the sanctioned extension point, same as CG-002's circle fixture): a
unit circle arc in a plane (closed when the full loop is taken), an
S-shaped spine (two arcs of opposite curvature joined C¹ — e.g. two
semicircles), and a helix (`C(s) = (cos θ, sin θ, c·θ)`). Every fixture
premise is machine-checked before the assertion that depends on it. No
`1e-…` literals (H-3); every bound goes through
`DirectTolerance::default()` or `TOLERANCE` (residue-scale bounds may scale
TOLERANCE by small decimal factors).

1. `transport_starts_from_orthonormalized_initial_normal` — an
   `initial_normal` tilted off-perpendicular from `t̂₀`: `frame(s_min)` is
   `Ok` and its normal is exactly the orthonormalized vector (unit,
   ⊥ tangent, within the position bound); `t × n == b`.
2. `straight_spine_has_constant_frame` — a straight LineSpine (zero
   curvature): `frame` is the SAME `Frame3` (within the position bound) at
   every queried s — zero-curvature stability, the property Frenet framing
   lacks and the plan demands.
3. `circular_loop_has_trivial_holonomy` — the full closed planar circle:
   `frame(1.0)`'s (tangent, normal, binormal) equals `frame(0.0)`'s within
   `64.0 * TOLERANCE` (planar closed curves carry no twist — the RMF's
   holonomy is trivial in the plane).
4. `frame_is_evaluation_order_independent` — for the helix: evaluate
   `frame(0.9)` then `frame(0.3)` on one recipe; evaluate `frame(0.3)` then
   `frame(0.9)` on a clone; and evaluate `frame(0.3)` after fifty
   intermediate queries. All answers agree exactly (bit-identical f64s —
   same inputs, same deterministic station list, same arithmetic).
5. `s_spine_survives_inflection` — the S-spine: `frame` is `Ok` and
   orthonormal at stations on both arcs AND at the joining station; the
   normal does not flip: `n(s)` varies by < 0.5 rad between adjacent query
   stations spaced 1/64 apart across the inflection (continuity, no sign
   catastrophe).
6. `parallel_initial_normal_is_singular` — `initial_normal` = +tangent
   direction: `Err(FrameSingular { law: "ParallelTransport", .. })`.
7. `helix_stays_orthonormal_along_transport` — (this is the AMENDED BODY of
   the landed `parallel_transport_still_refuses_in_cg002` in
   tests/constructive_frames.rs, name kept) — a helix recipe with a valid
   `initial_normal`: `frame(s)` is `Ok` and orthonormal (`|t|`, `|n|`, `|b|`
   within the position bound of 1; pairwise dots within the bound of 0) at
   17 stations along the transport, and `t × n == b` at each.

No existing test may be deleted, `#[ignore]`d, or weakened — except the ONE
booked in-place amendment above (name-preserving).

## House rules

- **H-1** No `unwrap`, `expect`, `panic!`, `todo!`, `unimplemented!` — in
  code and tests.
- **H-3** No `1e-…` literals anywhere; tolerances from
  `DirectTolerance::default()` / `TOLERANCE` (with small decimal scale
  factors where a residue-scale bound is needed).
- The crate warns `missing_docs, missing_debug_implementations` and denies
  warnings in release: doc-comment everything.
- No `unscaled_legacy(` calls (GATE-4); no `debug_new`, no
  `cfg!(debug_assertions)` (GATE-3). New files carry
  `#![deny(clippy::unwrap_used)]` (GATE-1).
- The transport must not consume the profile law, the sampling policy, or
  any mutable/cached state — the frame is a pure function of (spine,
  initial_normal, s).
- Determinism (plan §7): identical ordered input → bit-identical output; no
  parallelism inside the transport (it is inherently sequential along the
  station list); no hash-order dependence anywhere.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-geometry
cargo clippy -p truck-geometry --all-targets -- -D warnings
cargo test -p truck-geometry --lib --tests
```

Never run a bare `cargo test`. Send cargo output to a file and read the tail.

## Forbidden

Editing any file outside `write_allow` — especially
`constructive/errors.rs` (no new variants), the landed analytic frame files
(`frame_fixed.rs`, `frame_up.rs`, `frame_radial.rs` — read-only
references), the landed `Spine` trait and spine types, `constructive/
profile.rs`, `constructive/sampling.rs`, the crate `prelude`, `Cargo.toml`,
`Cargo.lock`, `scripts/kernel-gates.sh`. Adding caching/memoization of
frames (the deterministic pure-function contract forbids it; performance is
CG-004's concern). Adding a twist/exposure policy input (not booked).
Adding `#[ignore]`. Adding `#[allow]` without a same-line justification.
Committing to `main`.

## Stop conditions

- any anchor count differs → `ANCHOR_MISMATCH` (A4 must read 0 — the module
  does not exist yet; A2 must read 1 — the dispatcher arm you are replacing
  exists; A5 must read 1 — the amendable test exists)
- the contract as written cannot compile as specified → `SPEC_GAP`, naming
  the exact conflict
- three consecutive failed `cargo test` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` AT THE WORKTREE ROOT

```json
{"id":"BG-CG-003-TRANSPORT","status":"DONE","contracts":["BG-CG-003-TRANSPORT"],
 "tests_added":7,"anchors_verified":{"A1":1,"A2":1,"A3":1,"A4":0,"A5":1,"A6":1},
 "notes":"any deviation from the quoted contract, with the reason"}
```

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it: what you attempted, the
exact ambiguity, and the readings you could not choose between.

Commit on the current branch with subject
`feat(geometry): parallel-transport frame via double reflection (BG-CG-003-TRANSPORT)`
BEFORE writing `RESULT.json`.
