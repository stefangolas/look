# WORK PACKET BG-SOL-S2-PCURVE — pcurves on the extruded plate solid

You are investigating a known open item in the landed S2 extrude
(`truck-modeling/src/extrude.rs`, BG-SOL-S2-EXTRUDE). The plan
(`docs/SOLVER_FAMILY_PLAN.md`) §4 Phase 2 lists **pcurves** as part of S2's
direct B-rep construction, and §7 M1 says M1 "exercises ... pcurves". The S2
worker explicitly deferred them: S2 v1 sets every edge's pcurve payload to `()`
and recorded "the pcurve layer is a documented later refinement" in its
RESULT.json. This packet's job is to determine whether pcurves can be attached
to the edges of the returned `Solid` under the LANDED topology — and to deliver
the honest answer, which we have reason to believe is a SPEC_GAP.

Everything you need is in this document. **Do not read any other spec file** —
this packet is self-contained.

```json
{"id":"BG-SOL-S2-PCURVE","status":"DONE","contracts":["BG-SOL-S2-PCURVE"],
 "tests_added":0,"deviations":[],"disagreements":[],
 "baseline_failures":[],"notes":"free text"}
```

Fill `deviations`, `disagreements` and `baseline_failures` as arrays, empty if
empty. **If anything below contradicts what you find in the code, say so in
`disagreements` rather than making the code match the packet.**

```yaml
id:          BG-SOL-S2-PCURVE
class:       design
crates:      [truck-modeling, truck-topology, truck-geometry]
write_allow:
  - vendor/truck/truck-modeling/src/extrude.rs
read_allow:
  - vendor/truck/truck-topology/src/lib.rs
  - vendor/truck/truck-topology/src/edge.rs
  - vendor/truck/truck-geometry/src/decorators/mod.rs
budget:      {turns: 60, ctx_tokens: 160000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub struct Edge<P, C, PC = ()>' vendor/truck/truck-topology/src/lib.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub struct Wire<P, C>' vendor/truck/truck-topology/src/lib.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'pcurve: Option<PC>' vendor/truck/truck-topology/src/lib.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'edge_list: VecDeque<Edge<P, C>>' vendor/truck/truck-topology/src/lib.rs"}
  - {id: A5, expect: 1, cmd: "grep -c 'pub fn with_pcurve' vendor/truck/truck-topology/src/edge.rs"}
  - {id: A6, expect: 1, cmd: "grep -c 'pub struct PCurve<C, S>' vendor/truck/truck-geometry/src/decorators/mod.rs"}
```

## Problem

A pcurve is the parametric trace of a boundary edge on the face that owns it: a
2-D curve in the face's surface parameter space whose composition with the
surface reproduces the 3-D edge. The Boundary Rewrite Atlas (plan §2) needs them
so the Phase-4 material-state Boolean can do parameter-space classification.
The question is where they can live in the landed topology.

The plan books them into S2's construction. The landed S2 sets them to `()`.
This packet determines whether the gap between the two is a missing
implementation or a structural impossibility — and if it is structural, proves
it and records the SPEC_GAP.

## Design decision already made for you

### 1. The landed topology facts (re-derive each with grep / a compile probe)

These are the facts the packet is anchored on. Verify each before acting:

- **A1/A3:** `Edge<P, C, PC = ()>` carries `pcurve: Option<PC>` — the pcurve
  payload is a *type parameter* with unit default.
- **A2/A4:** `Wire<P, C>` holds `VecDeque<Edge<P, C>>`, which by A1 is
  `VecDeque<Edge<P, C, ()>>`. **The Wire level erases `PC` to `()`.** A Wire
  cannot hold an edge whose `PC` is anything but `()`.
- **A5:** `with_pcurve<Q>(self, pcurve: Q) -> Edge<P, C, Q>` *changes the type
  parameter*. Attaching a real pcurve produces `Edge<P, C, PCurve<...>>`, which
  does not fit in `Wire<P, C>`. (The truck-topology test module attaches
  `i32`/`PCurve<...>` payloads to *standalone* edges — never to an edge inside a
  Wire — precisely because the moment it enters a Wire the payload is `()`.)
- **A6:** the real pcurve carrier `PCurve<C, S>` exists in truck-geometry, so
  the *value* side is fine; the *container* side is the blocker.

From these four facts the consequence is immediate and it is not a small edit:
for a pcurve to ride on an edge inside the returned `Solid`, `PC` must be
threaded through `Wire<P, C>` → `Face<P, C, S>` → `Shell<P, C, S>` →
`Solid<P, C, S>`. That is a cross-crate topology-wide type change (every `Wire`
mention across meshalgo, shapeops, modeling, stepio), which the spec's own
BG-CE-001 record already anticipated: "the packet that wires real pcurves owns
trace splitting" — i.e. it is a future, larger program, not an S2 follow-up.

### 2. Your deliverable: prove it, then return SPEC_GAP

- Empirically confirm the erasure. `grep` the structs (anchors A1-A5) and, if
  you want a compile probe, write a tiny scratch `fn` (in your worktree, not
  committed) that tries to put `Edge::new(&v0, &v1, curve).with_pcurve(PCurve::new(...))`
  into a `Wire<Point3, Curve>` and observe the type error. Record exactly what
  you saw.
- Check whether any in-scope representation exists that you can implement
  entirely inside `vendor/truck/truck-modeling/src/extrude.rs` (the ONLY file in
  `write_allow`): a parallel per-face pcurve structure returned alongside the
  solid, keyed by face, that a Phase-4 consumer could consult. If — and only
  if — you find such a representation that is (a) implementable inside
  `extrude.rs`, (b) typed against the landed API, and (c) genuinely carries the
  parametric trace of each boundary edge on its owning face, you may implement
  it instead. But the expectation is that the honest answer is the SPEC_GAP in
  section 1, because the topology erases the payload at the Wire boundary and no
  `write_allow` file can change that.
- **Return `SPEC_GAP`** (status in `RESULT.json`) with the empirical proof in
  `notes`: the struct definitions you read, the type-parameter erasure, and why
  a `Wire<Point3, Curve>` cannot hold a pcurve-carrying edge. Also write
  `QUESTION.md` beside `RESULT.json` stating the proposed plan-doc amendment:
  the plan's §4 Phase 2 "pcurves" line cannot be delivered on the returned
  `Solid`'s edges without threading `PC` through `Wire`/`Face`/`Shell`/`Solid`
  (a cross-crate topology program, its own packet or family), and M1's
  "exercises pcurves" milestone should be recorded as satisfied by the S2
  construction + the topology's pcurve *carrier* existing, or re-scoped to that
  program.

Do NOT attempt the topology-wide `PC` threading change. Do NOT change any file
outside `write_allow`. Do NOT add pcurves as a half-measure (e.g. a fake
`Some(())` payload) — that is not a pcurve and would be worse than the honest
SPEC_GAP.

### 3. What must NOT change

- `extrude.rs` is in `write_allow` but you should not need to edit it for the
  SPEC_GAP. If you do edit it for the feasible-representation branch, keep every
  existing test green (especially the four S2 tests), keep `Solid::try_new`
  passing, and keep the face count at 7.
- Nothing about the M1 profile, `select_material`, or the material rule.

## Done-when gates

If you implement the feasible-representation branch:

```
cargo fmt --check -p truck-modeling
cargo clippy -p truck-modeling --all-targets --no-deps
cargo test -p truck-modeling --lib --tests --no-fail-fast
cargo check --locked -p truck-modeling --all-targets
bash scripts/kernel-gates.sh <your base commit>
```

If you return SPEC_GAP (the expected outcome), run only:

```
bash scripts/kernel-gates.sh <your base commit>
```

Never run a bare `cargo test`. Never run `cargo check --workspace` — it
exhausts disk on a shared machine.

## H-3 / GATE-4

GATE-2 rejects added lines carrying bare `1e-N` literals unless the line ends
with `// H-3`. You should add no float literals. This packet adds NO
`unscaled_legacy()` calls; do not touch `scripts/unscaled_legacy_ceiling.txt`.

## Forbidden

Editing any file outside `write_allow`. Attempting the topology-wide `PC`
threading change (through `Wire`/`Face`/`Shell`/`Solid`) or any edit to
`truck-topology`, `truck-geometry`'s decorators, or `truck-modeling`'s
`lib.rs`. Inventing a fake `Some(())` pcurve. Changing the GATE-4 ceiling.
Adding `#[ignore]`. Running `cargo check --workspace` / `cargo build --workspace`.

## Stop conditions

- an anchor count differs → `ANCHOR_MISMATCH`, naming the file and what you saw
- three consecutive failed `cargo` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. The
expected status is **`SPEC_GAP`** with `QUESTION.md` beside `RESULT.json`
proposing the plan-doc amendment. In `notes`, record the empirical proof of the
`PC` erasure (the exact struct definitions and, if you ran it, the compile
probe's type error), and your judgment on whether any in-scope representation
exists.

Commit on the current branch with subject
`gap(modeling): pcurves cannot ride on the extruded solid's edges — topology erases PC (BG-SOL-S2-PCURVE)`.
If you implement the feasible-representation branch instead, use subject
`feat(modeling): attach pcurves to the extruded plate solid (BG-SOL-S2-PCURVE)`.
