# WORK PACKET BG-ENC-004-SHARED-CONE — consolidate the shared enclosure helpers

You are implementing one item from a formal kernel specification. Everything you
need is in this document. **Do not read `docs/GENERATION_KERNEL_BUILD_SPEC.md`
or any other spec file** — they are not on your allowlist and this packet is
self-contained. If something you need is genuinely missing, that is a SPEC_GAP
(see "Stop conditions"): you stop and report, you do not research it.

```json
{"id":"BG-ENC-004-SHARED-CONE","status":"DONE","contracts":["BG-ENC-004"],
 "tests_added":3,"sites_migrated":0,"sites_deferred":0,
 "deviations":[],"disagreements":[],"baseline_failures":[],
 "notes":"free text"}
```

Fill `deviations`, `disagreements` and `baseline_failures` as arrays, empty if
empty. This packet is a MECHANICAL consolidation: the code being moved is
landed, tested, and verified code; your job is that it survives the move
byte-for-byte in behaviour. **If anything below contradicts what you find in
the code, say so in `disagreements` rather than making the code match the
packet.**

```yaml
id:          BG-ENC-004-SHARED-CONE
contract:    [BG-ENC-004]
class:       mechanical
crates:      [truck-evidence]
write_allow:
  - vendor/truck/truck-evidence/src/enclosure.rs
  - vendor/truck/truck-evidence/src/decorators/extruded.rs
  - vendor/truck/truck-evidence/src/decorators/processor.rs
  - vendor/truck/truck-evidence/src/decorators/revolved.rs
  - vendor/truck/truck-evidence/src/decorators/pcurve.rs
  - vendor/truck/truck-evidence/src/decorators/intersection_curve.rs
  - vendor/truck/truck-evidence/src/bspline.rs
  - vendor/truck/truck-evidence/src/nurbs.rs
read_allow:
  - vendor/truck/truck-evidence/src/lib.rs
  - vendor/truck/truck-evidence/src/harness.rs
  - vendor/truck/truck-evidence/Cargo.toml
tests_required:
  - midpoint_ball_cone_contains_off_axis_directions
  - midpoint_ball_cone_refuses_when_the_box_straddles_the_origin
  - cross_box_encloses_the_componentwise_formula
budget:      {turns: 30, ctx_tokens: 60000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'fn cross_box' vendor/truck/truck-evidence/src/decorators/extruded.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'fn cross_box' vendor/truck/truck-evidence/src/decorators/revolved.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'fn cross_box' vendor/truck/truck-evidence/src/decorators/processor.rs"}
  - {id: A4, expect: 2, cmd: "grep -c 'MAX_CONE_HALF_ANGLE' vendor/truck/truck-evidence/src/decorators/revolved.rs"}
  - {id: A5, expect: 1, cmd: "grep -c 'wid() / 2.0' vendor/truck/truck-evidence/src/nurbs.rs"}
  - {id: A6, expect: 0, cmd: "grep -c 'midpoint_ball_cone' vendor/truck/truck-evidence/src/enclosure.rs"}
  - {id: A7, expect: 2, cmd: "grep -c 'MAX_HALF_ANGLE' vendor/truck/truck-evidence/src/decorators/pcurve.rs"}
```

(`grep -c` exits 1 on zero matches — a count of 0 IS the expected answer for
A6, not a command failure.)

## Problem

Seven modules of this crate carry private copies of the same three helpers,
duplicated because their original packets needed disjoint write sets:

- `fn interval_at(x: f64) -> Interval` — identical body in every module;
- `fn cross_box(a: &Box3, b: &Box3) -> Box3` — the componentwise interval
  cross product, identical in `extruded.rs`, `revolved.rs`, and inline in
  `processor.rs`;
- the **midpoint-ball normal cone**: from a derivative-box `n`, take
  `c` = midpoint vector, `h` = half-width vector,
  `rho = ‖h‖` rounded UP (`.sup()`), `cn = ‖c‖` rounded DOWN (`.inf()`),
  refuse (`None`) when `!cn.is_finite() || !rho.is_finite() || cn <= rho`,
  else `axis = c.normalize()`,
  `half_angle = ((rho/cn).asin() * (1.0 + 8.0 * f64::EPSILON)
  + 8.0 * f64::EPSILON).min(MAX_HALF_ANGLE)` — byte-identical logic in all
  seven modules (the ulp nudge keeps the f64 asin/normalize from rounding
  the cone too narrow; the clamp keeps it on the sphere);
- the **mignitude immersion lower bound**:
  `sqrt(mig(n.x)² + mig(n.y)² + mig(n.z²))` computed in inari, returning
  `.inf()` when finite else `0.0` — three near-identical copies
  (`extruded.rs`, `processor.rs`, `revolved.rs`).

Copies drift. One rounding-mode fix applied to six of seven modules is a
soundness bug in the seventh and nobody can see which copies are current.
This packet moves ONE copy of each helper into `enclosure.rs` and points
every carrier at it.

## Decisions already made for you

### 0. The new shared items, in `enclosure.rs`

All four are `pub(crate)` (they are crate plumbing, not public API). Place
them after the `DirCone` definition, each carrying its doc comment MOVED FROM
the most complete existing copy (extruded.rs's cross_box doc, processor's
interval_at doc, extruded's normal_cone reasoning block):

```rust
/// A degenerate interval from a runtime `f64`. Finite coordinates always
/// construct; a NaN widens to the empty interval rather than panicking (H-1).
pub(crate) fn interval_at(x: f64) -> Interval {
    Interval::try_from((x, x)).unwrap_or(Interval::EMPTY)
}

/// The interval cross product of two boxes, written out componentwise.
///
/// Sound but loose: it encloses `{ p x q : p in a, q in b }`, a superset of
/// `{ S_u(x) x S_v(x) : x in box }` because it lets `p` and `q` vary
/// independently where in truth they are evaluated at the same parameter
/// point. Over-estimation is always acceptable (BG-ENC-001).
pub(crate) fn cross_box(a: &Box3, b: &Box3) -> Box3 {
    Box3 {
        x: a.y * b.z - a.z * b.y,
        y: a.z * b.x - a.x * b.z,
        z: a.x * b.y - a.y * b.x,
    }
}

/// The midpoint-ball direction cone of a derivative box: `Some(cone)` iff
/// every element of the box lies within a half-angle `asin(rho/cn)` of the
/// box's midpoint direction, with `rho = ‖h‖` rounded up and `cn = ‖c‖`
/// rounded down so the f64 arithmetic cannot make the cone too narrow.
/// `None` when the box may contain the zero vector or straddle enough
/// directions that no cone bounds it — including any singular locus. That
/// arm is the contract, not a convenience.
pub(crate) fn midpoint_ball_cone(n: &Box3) -> Option<DirCone> {
    // the construction exactly as stated above, with the shared
    // MAX_HALF_ANGLE const below
}

/// The smallest `‖·‖` over a derivative box:
/// `sqrt(mig_x² + mig_y² + mig_z²)` — each coordinate attains its mignitude
/// independently, so this is exactly the box's minimum norm, and since the
/// box contains the true set it is a valid lower bound on the true minimum.
/// Computed in inari and read from the LOWER endpoint (a bound one rounding
/// unit too large is a soundness bug, not a tightness one). An empty or
/// overflowing box contributes nothing: `0.0`.
pub(crate) fn immersion_lower_bound_box(n: &Box3) -> f64 { /* as stated */ }

/// The whole-sphere clamp for computed half-angles; keeps an ulp-nudged
/// value from exceeding PI.
const MAX_HALF_ANGLE: f64 = core::f64::consts::PI;
```

Write the guard comparison as `cn <= rho` — NOT `!(cn > rho)`: clippy
denies `neg_cmp_op_on_partial_ord` at the crate root, and the explicit
finiteness tests beside it make the two forms equivalent (they differ only
on NaN, which the finiteness arms already refuse).

### 1. The callers

Every touched module DELETES its private copy and calls the shared item.
Per module, the change is mechanical:

- **`decorators/extruded.rs`** — delete its `interval_at`, `cross_box`,
  the norm closure inside `normal_cone`; `normal_cone` becomes
  `midpoint_ball_cone(&normal_box(self, uu, vv))`;
  `immersion_lower_bound` becomes
  `immersion_lower_bound_box(&normal_box(self, uu, vv))`; keep
  `normal_box` (it is extrusion-specific). Delete its local
  `MAX_HALF_ANGLE` const.
- **`decorators/processor.rs`** — delete `interval_at`, `interval_norm`,
  the inline `cross_box<S>` (rebuild its two derivative boxes then call the
  shared `cross_box(&a, &b)`); `normal_cone` /
  `immersion_lower_bound` delegate through the processor's own
  orientation-resolved box acquisition exactly as they do now. Delete its
  local `MAX_HALF_ANGLE`.
- **`decorators/revolved.rs`** — delete `interval_at`, `cross_box`, its
  inline constructions; delegate as above. Delete `MAX_CONE_HALF_ANGLE`
  (the shared `MAX_HALF_ANGLE` replaces it).
- **`decorators/pcurve.rs`** — delete `interval_at` and the inline
  midpoint-ball block in `normal_cone`; the box comes from
  `self.enclose_der(1, tt)` as now. Delete its local `MAX_HALF_ANGLE`.
- **`decorators/intersection_curve.rs`** — delete `interval_at` and the
  inline midpoint-ball block; KEEP `sub3`, `dot3` and the other
  intersection-specific helpers (only this module uses them). Delete its
  local `MAX_HALF_ANGLE`.
- **`bspline.rs`** — delete `interval_at` and the inline block (box from
  `hull_of(&self.derivation(), tt)`); delete `MAX_HALF_ANGLE`.
- **`nurbs.rs`** — delete `interval_at` and the inline block (box from
  `self.enclose_der(1, tt)` AFTER the `positive_weights` check — keep that
  check first, it is nurbs-specific); delete `MAX_HALF_ANGLE`.

Each module imports the shared items from `crate::enclosure`. Behaviour
must be IDENTICAL: same refusal conditions, same rounding directions, same
ulp nudge, same clamp. Where two existing copies differ in comment prose
only, keep the better prose in the moved doc comment and move on.

### 2. Do NOT touch the analytic carriers

`plane.rs`, `line.rs`, `circle.rs`, `sphere.rs`, `cylinder.rs`, `cone.rs`,
`torus.rs`, `elementary.rs`, `deviation.rs`, `analytic/**` each carry a
private `interval_at`. LEAVE THEM ALONE — their packets are DONE and landed;
this consolidation deliberately does not reopen nine files for a one-line
helper. (They adopt the shared helper opportunistically whenever their next
packet touches them.) This restriction is enforced by V1.

### 3. Tests (in `enclosure.rs`'s existing `#[cfg(test)] mod tests`)

- `midpoint_ball_cone_contains_off_axis_directions` — build a small box
  around a point off the origin (e.g. hull of `(2±0.1, 1±0.1, 0.5±0.1)`
  via `Interval::try_from`), get the cone; assert it is `Some`, and sample
  directions from the box's corners normalized — each must satisfy
  `angle(d, axis) <= half_angle` (compare via dot products against
  cos(half_angle), with a small slack const carrying an H-3 comment).
- `midpoint_ball_cone_refuses_when_the_box_straddles_the_origin` — a box
  symmetric about the origin (or containing it): `None`. Also `None` for
  `Box3::empty()`.
- `cross_box_encloses_the_componentwise_formula` — two known boxes whose
  exact cross product at corner combinations you can enumerate by hand
  (say a=(x:[1,2], y:[0,1], z:[-1,1]) and b=(x:[0,1], y:[2,2],
  z:[1,1])): assert the resulting intervals contain every enumerated
  corner-pair product, and that a degenerate-input case reproduces the
  schoolbook result exactly.

All pre-existing tests in the eight touched files MUST pass unchanged. If
one fails, the move changed behaviour — fix the move, never the test.

## H-3, the house rule that rejects bare float literals

GATE-2 of `scripts/kernel-gates.sh` fails any **added** line carrying a bare
`1e-N` literal unless that same line ends with an `// H-3` comment. The only
float constants in this packet are `2.0`/`0.5`-class decimals and the
existing `f64::EPSILON` nudges (not literals). If YOU introduce any absolute
constant (a test slack), name it as a `const` with a same-line `// H-3:`
comment naming the dimensionless quantity. Run
`bash scripts/kernel-gates.sh` yourself before writing `RESULT.json`.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-evidence
cargo clippy -p truck-evidence --all-targets --no-deps
cargo test -p truck-evidence --lib --tests --no-fail-fast
cargo test -p truck-evidence --doc
cargo check --workspace --all-targets
bash scripts/kernel-gates.sh <your base commit>
```

Never run a bare `cargo test` — it builds 56 examples. Send cargo output to a
file and read the tail.

**The crate is clean at baseline** — measured at the tree this packet was
written against (HEAD 182b2c0): the full lib suite passes, zero clippy
findings. Your bar: everything stays green plus your three new tests. Any
baseline failure you did not cause is a stop condition; any failure you did
cause is yours to fix.

## Forbidden

Editing any file outside `write_allow` — in particular everything under
`src/analytic/**`, `src/{plane,line,circle,sphere,cylinder,cone,torus,
elementary,deviation,harness}.rs` (decision 2), `truck-base/**`, and every
other crate. Changing any refusal condition, rounding direction, or the ulp
nudge (behaviour-identical means identical). Deleting or weakening any
existing test. Adding `#[ignore]`. Adding `unwrap()`/`expect()`/`panic!` on
fallible paths in production code. Committing to `main`.

## Stop conditions

- an anchor count differs → `ANCHOR_MISMATCH`, naming the file and what you saw
- two copies you were told are identical differ in BEHAVIOUR (different
  guard order, different rounding endpoint, different clamp) → `SPEC_GAP`
  naming both files and the differing lines — do not silently pick one;
  that choice is an orchestrator decision, and it may be a latent soundness
  difference one of the landed verifiers depends on
- any pre-existing test fails after the move → treat as your regression
  first; only report `SPEC_GAP` if the failing assertion encodes the old
  copy's divergent behaviour
- three consecutive failed `cargo` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit on the current branch with subject
`refactor(evidence): consolidate the shared enclosure helpers (BG-ENC-004-SHARED-CONE)`.
