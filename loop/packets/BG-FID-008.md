# WORK PACKET BG-FID-008 — the one-sheet condition (iv-a), curve case

You are implementing one item from a formal kernel specification. Everything you
need is in this document. **Do not read `docs/GENERATION_KERNEL_BUILD_SPEC.md`
or any other spec file** — they are not on your allowlist and this packet is
self-contained. If something you need is genuinely missing, that is a SPEC_GAP
(see "Stop conditions"): you stop and report, you do not research it.

```json
{"id":"BG-FID-008","status":"DONE","contracts":["BG-FID-008"],
 "tests_added":6,"sites_migrated":0,"sites_deferred":0,
 "deviations":[],"disagreements":[],"baseline_failures":[],
 "notes":"free text"}
```

Fill `deviations`, `disagreements` and `baseline_failures` as arrays, empty if
empty. `disagreements` is the highest-value field: **if anything below
contradicts what you find in the code as you work it, say so in
`disagreements` rather than making the code match the packet.**

```yaml
id:          BG-FID-008
contract:    [BG-FID-008]
class:       design
crates:      [truck-evidence]
write_allow:
  - vendor/truck/truck-evidence/src/fid/one_sheet.rs
  - vendor/truck/truck-evidence/src/fid/mod.rs
read_allow:
  - vendor/truck/truck-evidence/src/enclosure.rs
  - vendor/truck/truck-evidence/src/elementary.rs
  - vendor/truck/truck-evidence/src/num/krawczyk.rs
  - vendor/truck/truck-evidence/src/num/roots.rs
  - vendor/truck/truck-evidence/src/fid/lfs.rs
  - vendor/truck/truck-topology/src/invariants/wedge.rs
budget:      {turns: 44, ctx_tokens: 120000}
anchors:
  # Measured under Git Bash on integration HEAD at packet-writing time.
  # A count mismatch is a stop condition (ANCHOR_MISMATCH), not a nuisance.
  - {id: F1, expect: 1, cmd: "grep -c '^pub mod' vendor/truck/truck-evidence/src/fid/mod.rs"}
  - {id: F2, expect: 1, cmd: "grep -c 'pub fn krawczyk' vendor/truck/truck-evidence/src/num/krawczyk.rs"}
  - {id: F3, expect: 7, cmd: "grep -c 'KrawczykSystem' vendor/truck/truck-evidence/src/num/krawczyk.rs"}
  - {id: F4, expect: 1, cmd: "grep -c 'pub fn cos' vendor/truck/truck-evidence/src/elementary.rs"}
  - {id: F5, expect: 1, cmd: "grep -c 'pub fn sin' vendor/truck/truck-evidence/src/elementary.rs"}
  - {id: F6, expect: 1, cmd: "grep -c 'fn tangent_cone' vendor/truck/truck-evidence/src/enclosure.rs"}
  - {id: F7, expect: 1, cmd: "grep -c 'pub trait EnclosureCurve' vendor/truck/truck-evidence/src/enclosure.rs"}
  - {id: F8, expect: 0, cmd: "grep -c 'MultiSheetInTube' vendor/truck/truck-evidence/src/fid/lfs.rs"}
  - {id: F9, expect: 1, cmd: "grep -c 'fn box_distance' vendor/truck/truck-evidence/src/fid/lfs.rs"}
  - {id: F10, expect: 4, cmd: "grep -c 'ParametricCurve' vendor/truck/truck-topology/src/invariants/wedge.rs"}
  - {id: F12, expect: 0, cmd: "grep -c 'pub mod one_sheet' vendor/truck/truck-evidence/src/fid/mod.rs"}
```

## Problem

Conditions (i)-(iii) of the isotopy lemma make the normal projection
restricted to an approximant a proper local homeomorphism — a covering of
SOME constant finite degree. They do NOT force degree one, so a checker
implementing only (i)-(iii) passes topologically wrong output. The canonical
witness: X = circle of radius R, X' = `(R + eps*cos(t/2))*e(t)` over
`t ∈ [0, 4π]` — closed, within eps both ways, tangent deviation O(eps/R),
and a 2-to-1 covering. Nothing landed today can distinguish it from a good
approximant.

This packet ships discharge **(iv-a)** for CURVE components: certify that one
witnessed normal disc meets the approximant exactly once, by root isolation
over the whole span with certified exclusion everywhere else. The witness
above is implemented verbatim as the flagship negative test — it must come
back MULTI-SHEET, and any implementation that returns degree-one on it is
wrong no matter what its other tests say.

Scope, decided for you: (iv-a) on curves ONLY. The surface case needs 2D
root certification in the normal bundle and lands with BG-FID-005, where the
emitter's own cell partition makes discharge (iv-b) free ("the partition is
free" is the spec's design point there). Discharge (iv-b) itself also lands
with FID-005 — no emitter partition exists to feed it here. Do not stub
either; the module documents both deferrals.

## Decisions already made for you

### Decision 0 — API and types

```rust
/// What the witnessed disc certified.
pub enum FibreMultiplicity {
    /// Exactly one approximant point on the closed normal disc at x.
    ExactlyOne,
    /// Certified cardinality != 1 on that disc. `count` is the CERTIFIED
    /// lower bound on distinct geometric intersections; `count == 0` means
    /// the fibre missed entirely (a coverage violation, equally fatal).
    NotOne { count: usize },
}

/// Typed failures. SheetCountUnresolved is EPISTEMIC: the root count could
/// not be certified within budget — it is a claim about the run, never
/// about geometry in either direction.
pub enum OneSheetError {
    /// The witness parameter's tangent is undefined or zero-magnitude.
    InvalidWitness,
    /// Root isolation did not resolve within budget / width floor.
    SheetCountUnresolved,
}

pub fn fibre_degree_one(
    exact: &impl EnclosureCurve,
    approx: &impl EnclosureCurve,
    t_x: f64,
    eps: f64,
    budget: &mut Budget,
) -> Result<FibreMultiplicity, OneSheetError>
```

InvalidInput-class checks (`eps <= 0`, non-finite eps, `t_x` outside the
exact curve's parameter range) return `InvalidWitness`. Naming discipline:
NOTHING in this module may be named isotopy, homeomorphism, certificate-of-
isotopy, or OneSheetCertificate — what a positive answer establishes is
degree-one ON ONE DISC, and the annotation block says exactly that.

### Decision 1 — the fibre equation and its engine

Witness point `x = exact.subs(t_x)`; unit tangent `u` from
`exact.enclose_der(1, degenerate(t_x))` midpoint, magnitude-checked (refuse
`InvalidWitness` when the enclosure contains zero). The normal disc at x is
`{ p : <p − x, u> == 0, |p − x| <= eps }`. Its intersection with X' is the
roots of the UNIVARIATE equation

```text
h(t) = <approx.subs(t) − x, u> == 0   with   |approx.subs(t) − x| <= eps
```

Engine: bisection worklist over the approximant's parameter range, per box:

1. Interval h via dot_box(shift(approx.enclose(tt), −x), u_box); prune the
   box when the interval excludes 0.
2. Prune when box_distance(approx.enclose(tt), x) > eps (no in-disc
   intersection possible).
3. Otherwise attempt the landed Krawczyk operator (NUM-003, N=1) on the
   system `f(t) = h(t)`:
   - `f_point`: degenerate intervals at `approx.subs(midpoint)`,
   - `jacobian`: interval `<approx.enclose_der(1, box), u>` (h'(t) =
     <X''(t), u> — chain rule against the CONSTANT u),
   - `preconditioner`: `1/h'(midpoint)` when finite and nonzero, else None
     (the operator bisects on None by design).
   - `KrawczykProof::Unique` → one certified root in this box;
   - `NoRoot` → discard;
   - indeterminate → bisect; at the width floor or budget exhaustion →
     `Err(SheetCountUnresolved)`.
4. Each certified root then gets a DISC-MEMBERSHIP decision: subdivide its
   box until `box_distance(enclose(box), x) <= eps` (whole box inside the
   closed ball ⇒ the root lies in-disc, since h == 0 already puts it on the
   normal plane) or `> eps` (root outside the disc ⇒ does not count).
   Ambiguous at the floor/budget → `SheetCountUnresolved`.

Dedupe rule, decided: two certified roots whose point-boxes OVERLAP are the
same geometric point and count ONCE — a closed curve hits the same point at
`t*` and `t* + period`, and naive counting would double every healthy
certificate. Count > 1 after merging → early exit `NotOne{count}` with the
witness parameter of the second distinct root. Worklist drained with total
count != 1 → `NotOne{count}` (0 included); exactly 1 → `ExactlyOne`.

Tangential contact (an even-multiplicity touch of the plane inside the ball)
never yields Unique → drains to `SheetCountUnresolved`. That is correct and
tested; reporting "one sheet" for it would be the classic false pass.

### Decision 2 — ordering dependency, encoded as docs

(iv-a)'s reduction to a single fibre is licensed ONLY by conditions (i)-(iii)
already holding (their consequence — constant fibre cardinality per
component — is what makes one fibre decisive). The function takes no (i)-(iii)
data; instead its doc comment carries, verbatim: "@precondition BG-FID-003
(i)-(iii) hold on this component; calling this without them proves nothing."
Do not weaken, hide, or "implement" that precondition.

### Decision 3 — annotations (@feeds form, mandatory)

Every public item carries immediately above it:

```rust
/// @feeds-open-lemma FID-L-COVERING      # degree-one fibre evidence, per component
/// @establishes certified fibre cardinality on ONE witnessed normal disc
/// @does-not-establish
///   isotopy | homeomorphism | side separation | whole-span one-sheet
```

A definition citation is not a theorem instance; evidence feeding a hypothesis
does not instantiate the theorem. Wrong tags are `disagreements` findings.
The bridge lemmas L-TUBE / L-COVERING / L-SEPARATES remain OPEN obligations —
this module cites them as fed, never as proved.

### Decision 4 — module layout

`fid/mod.rs` gains exactly one line: `pub mod one_sheet;` plus its doc line
extended to note (iv-b) and the surface case wait on BG-FID-005. one_sheet.rs
carries `#![deny(clippy::unwrap_used)]` INCLUDING the test module (GATE-1);
private helpers duplicated locally (`dot_box`, `box_distance`, interval
shift-by-point) exactly as lfs.rs did — enclosure.rs visibility stays
untouched. Test-only curve structs live IN the test module following
wedge.rs's local-curve pattern (read it first): implement the ParametricCurve
surface plus EnclosureCurve with hand-written interval enclosures built on
crate::elementary's outward-rounded cos/sin. Soundness before tightness — a
loose but sound enclosure just costs subdivisions.

### Decision 5 — tests (all in one_sheet.rs's test module)

All floats named consts with same-line `// H-3:` comments. Exact circle
radius R = 2, eps = 0.05, witness parameter away from dyadic bisection
midpoints AND away from the domain endpoints (t_x ≈ 0.7 rad).

1. `single_sheet_circle_certifies_degree_one` — approximant
   `(R+eps)*e(t)` over `[0, 2π]`: `Ok(ExactlyOne)`. This exercises the dedupe
   rule too: the plane crossings at `t*` and `t*+2π` are the SAME point and
   merge to one.
2. `double_cover_witness_refuses` — `(R + eps*cos(t/2))*e(t)` over
   `[0, 4π]`, VERBATIM from the spec: `NotOne { count: 2 }`. The plane
   crossings near `t*` and `t*+2π` are genuinely distinct points
   (`(R±eps)e(t*)`); the crossings at `t = π, 3π` sit ~2R outside the ball
   and MUST be excluded by the disc test, not counted — assert count is
   exactly 2, which fails both an under-counting and an over-counting bug.
3. `offset_sheet_outside_disc_ignored` — approximant offset by 3*eps
   (> disc radius): `NotOne { count: 0 }` — the coverage-violation arm.
4. `tangential_contact_is_unresolved_not_degree_one` — an approximant whose
   signed plane coordinate has a double-touch extremum INSIDE the ball
   (e.g. rho(t) = R + eps − c*(t−t*)² with c small enough to stay within
   eps of the plane... derive the constant, machine-check it, and name it):
   `Err(SheetCountUnresolved)`, NEVER Ok.
5. `zero_budget_refuses_unresolved` — empty budget →
   `SheetCountUnresolved`.
6. `invalid_witness_refuses` — eps <= 0 and a pole-straddling witness
   parameter each return `InvalidWitness`.

Machine-check every hand-derived number in these witnesses with a script
before writing RESULT.json (the session-18 lesson: five wrong witnesses cost
a round trip). Reference values must come from THIS module's code path, not
from a scratch variant.

### Decision 6 — spec amendment already made (context only)

The surface-case negative test ("run it for a surface too") moves to
BG-FID-005's packet together with (iv-b); the spec carries an amendment
saying so. Your RESULT.json does not owe that test, and its absence is not a
SPEC_GAP.

## H-3, the house rule that rejects bare float literals

GATE-2 of `scripts/kernel-gates.sh` fails any **added** line carrying a bare
`1e-N` literal unless that same line ends with an `// H-3` comment. EVERY
epsilon, radius and slack above is a named const whose defining line carries
a same-line `// H-3:` comment naming the dimensionless quantity. Run
`bash scripts/kernel-gates.sh <your base>` before writing RESULT.json.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-evidence
cargo clippy -p truck-evidence --all-targets --no-deps
cargo test -p truck-evidence --lib --no-fail-fast
cargo check --workspace --all-targets
bash scripts/kernel-gates.sh <base>        # base = merge-base with integration tip
```

truck-evidence is green at baseline (measured this session). Any baseline
failure you did not cause is a stop condition. Send cargo output to a file
and read the tail. Never run a bare `cargo test`.

## Forbidden

Editing files outside `write_allow`. Implementing the surface case or (iv-b)
here. Naming anything certificate/isotopy/homeomorphism-flavored beyond the
types in Decision 0. Claiming any bridge lemma as proved, or tagging anything
"Thm instance". Bare float literals without `// H-3`. Adding subdivision-free
shortcuts that bypass the Krawczyk uniqueness step (sampling-based counting
is the classic false pass). `unwrap()`/`expect()` on fallible production
paths. Committing to `main`.

## Stop conditions

- an anchor count differs → `ANCHOR_MISMATCH`, naming the anchor
- the Krawczyk N=1 system cannot express the fibre equation as specified
  (trait shape mismatch you cannot resolve locally) → `SPEC_GAP` naming the
  mismatch
- three consecutive failed `cargo` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. Commit
on the current branch with subject

```
feat(evidence,fid): one-sheet condition (iv-a) for curves via Krawczyk fibre isolation (BG-FID-008)
```
