# WORK PACKET BG-INV-104 — invariant checker 4: same-parameter / same-range

You are implementing one item from a formal kernel specification. Everything you
need is in this document. **Do not read `docs/GENERATION_KERNEL_BUILD_SPEC.md`
or any other spec file** — they are not on your allowlist and this packet is
self-contained. If something you need is genuinely missing, that is a SPEC_GAP
(see "Stop conditions"): you stop and report, you do not research it.

```json
{"id":"BG-INV-104","status":"DONE","contracts":["BG-INV-104"],
 "tests_added":5,"sites_migrated":0,"sites_deferred":0,
 "deviations":[],"disagreements":[],"baseline_failures":[],
 "notes":"free text"}
```

Fill `deviations`, `disagreements` and `baseline_failures` as arrays, empty if
empty. **If anything below contradicts what you find in the code, say so in
`disagreements` rather than making the code match the packet.**

```yaml
id:          BG-INV-104
contract:    [BG-INV-001]
class:       mechanical
crates:      [truck-topology]
write_allow:
  - vendor/truck/truck-topology/src/invariants/same_parameter.rs
  - vendor/truck/truck-topology/Cargo.toml
  - Cargo.lock
read_allow:
  - vendor/truck/truck-topology/src/invariants/mod.rs
  - vendor/truck/truck-topology/src/edge.rs
  - vendor/truck/truck-evidence/src/deviation.rs
  - vendor/truck/truck-evidence/src/lib.rs
  - vendor/truck/truck-base/src/evidence.rs
  - vendor/truck/truck-base/src/tolerance.rs
tests_required:
  - same_parameter_exact_pcurve_edge_holds
  - same_parameter_offset_pcurve_edge_violates
  - same_parameter_none_pcurve_is_vacuously_ok
  - same_parameter_route2_offset_violates
  - same_parameter_zero_budget_is_unresolved
budget:      {turns: 35, ctx_tokens: 90000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub mod same_parameter' vendor/truck/truck-topology/src/invariants/mod.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub mod invariants' vendor/truck/truck-topology/src/lib.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'pub fn pcurve' vendor/truck/truck-topology/src/edge.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'pub fn curve' vendor/truck/truck-topology/src/edge.rs"}
  - {id: A5, expect: 1, cmd: "grep -c 'pub fn certify_deviation' vendor/truck/truck-evidence/src/deviation.rs"}
  - {id: A6, expect: 1, cmd: "grep -c 'pub struct ParamMap' vendor/truck/truck-evidence/src/deviation.rs"}
  - {id: A7, expect: 1, cmd: "grep -c 'SameParameter' vendor/truck/truck-base/src/evidence.rs"}
  - {id: A8, expect: 0, cmd: "grep -c 'pub fn' vendor/truck/truck-topology/src/invariants/same_parameter.rs"}
  - {id: A9, expect: 0, cmd: "grep -c 'truck-evidence' vendor/truck/truck-topology/Cargo.toml"}
```

(A8 pins the scaffold as EMPTY and A9 pins the manifest BEFORE the wiring;
`grep -c` exits 1 on zero matches, which IS the expected count. Attempt 1
verified all eight original anchors against this same tree.)

## Problem

§1.1 invariant 4: every edge use's parametric trace agrees with the edge's
leader curve — `||Γ_f(pc_u(t)) − c_e(φ_u(t))|| ≤ τ_e` over the WHOLE span.
BG-CE-002 (landed, `truck_evidence::certify_deviation`) is the certificate;
BG-INV-104 is the checker that applies it to an `Edge`'s pcurve payload
(the `PC` field BG-CE-001 landed) and speaks the invariants contract.

The correspondence `φ_u` is part of the ATTACHMENT CONTRACT — it is not
recorded in the tree (the pcurve field carries only the curve), so the
checker takes it as a parameter. Callers deriving it: an
orientation-matched attachment over the full ranges is
`ParamMap::from_ranges(pc_range, curve_range)`; a flipped use is
`ParamMap::flip(t0, t1)` over the shared range.

The checkers module tree is already scaffolded and declared — read
`invariants/mod.rs` first. **Only `same_parameter.rs` is yours.** Read
`truck-evidence/src/deviation.rs`'s module docs before writing: the
certificate has two routes (the difference spline for exact-spline pairs,
budgeted bisection otherwise) and your checker is a thin adapter over it.

## Decisions already made for you

### 0. The dependency wiring — your FIRST change

**This packet is the first invariant checker that needs crates truck-topology
does not yet depend on** (found by attempt 1, whose worker implemented the
whole checker, hit E0432 on `use truck_evidence::…` / `use inari::…`, and
correctly stopped). The dependency edge is legal — truck-evidence does NOT
depend on truck-topology, so no cycle — and it is yours to land, exactly this
way:

In `vendor/truck/truck-topology/Cargo.toml`, under `[dependencies]` (the
versions and path style of the existing entries; keep alphabetical order,
which is the file's convention):

```toml
inari = { version = "2.0", default-features = false }
truck-evidence = { version = "0.1.0", path = "../truck-evidence" }
```

and under `[dev-dependencies]` (which currently holds the `serde_json`
dev-dependency BG-CE-003 landed):

```toml
truck-geometry = { version = "0.5.0", path = "../truck-geometry" }
```

(Read the manifest FIRST and use each dependency's exact declared form —
copy the version/path style from how truck-base declares its siblings in the
same file; the versions above are what the sibling manifests use, verify by
reading `vendor/truck/truck-evidence/Cargo.toml` and
`vendor/truck/truck-geometry/Cargo.toml`.) Then run
`cargo check -p truck-topology` ONCE WITHOUT `--locked` to update the root
`Cargo.lock`, and commit the manifest and lock together — a `--locked` run
before the lock is updated will refuse. The lock gains dependency edges for
truck-topology; that is the expected diff.

### 1. The public API, verbatim:

   ```rust
   use crate::Edge;
   use truck_base::evidence::{Budget, Certificate, Certified, Method, Outcome, Prop, PropMap, Truth};
   use truck_evidence::{certify_deviation, ParamMap};
   use truck_geotrait::ParametricCurve;

   /// BG-INV-104: same-parameter / same-range (§1.1 invariant 4) for ONE
   /// edge use.
   ///
   /// Certifies `||pc(t) − curve(phi(t))|| ≤ tau` for ALL t in the pcurve's
   /// parameter span, by BG-CE-002's whole-span certificate. The parameter
   /// correspondence `phi` is the attachment contract, supplied by the
   /// caller — the tree does not record it. An edge whose `pcurve()` is
   /// `None` (the `PC = ()` default, today's every edge) is vacuously
   /// satisfied: there is no trace to disagree with the leader.
   ///
   /// Refusals: `ForwardToleranceExceeded { bound, allowed }` is the
   /// VIOLATION (a certified lower bound on the deviation exceeds `tau` —
   /// this checker keeps the quantitative refusal rather than collapsing it
   /// to `Contradictory`, because the bound localises by magnitude);
   /// `NumericallyUnresolved` means neither could be established within
   /// budget; `Empty` means the pcurve's span is empty or unbounded — trim
   /// before certifying.
   pub fn check_edge<P, C, PC>(
       edge: &Edge<P, C, PC>,
       phi: ParamMap,
       tau: f64,
       budget: &mut Budget,
   ) -> Outcome<()>
   where
       C: ParametricCurve + Clone,
       PC: ParametricCurve,
   {
   ```

   Wait — the leader and carrier must satisfy `EnclosureCurve`, not just
   `ParametricCurve`. The bound you actually need, and the only one:

   ```rust
   where
       C: truck_evidence::EnclosureCurve + Clone,
       PC: truck_evidence::EnclosureCurve,
   ```

   (`EnclosureCurve` already implies `ParametricCurve<Point = Point3>`, so
   no separate `ParametricCurve` import or bound is needed.)

2. **The body, in order:**

   - `edge.pcurve()` is `None` → the holds certificate of decision 3 (the
     vacuous case; its doc comment in decision 1 says so).
   - Else extract the span from the PCURVE's `parameter_range()`
     (`PC: ParametricCurve` brings it): both bounds must be
     `Bound::Included(t)` or `Bound::Excluded(t)` with `t0 < t1` finite —
     any `Bound::Unbounded` or inverted span → `Err(Refusal::Empty)`
     (nothing certifiable; the doc comment of decision 1 covers it). Build
     the `inari::Interval` with `Interval::try_from((t0, t1))`, mapping a
     construction failure to `Err(Refusal::Empty)` too (H-1: no unwrap).
   - Call `certify_deviation(&edge.curve(), pc, phi, tt, tau, budget)` —
     note the order: `certify_deviation(leader, carrier, …)` and the LEADER
     is the edge's 3D curve, the CARRIER is the pcurve. Getting this
     backwards inverts the correspondence and the offset tests will catch
     it — they are designed to.
   - Map `Ok(certified_bound)` → `Ok(Certified::new((), …))` carrying the
     bound's certificate with `Prop::SameParameter` set `True` — the
     underlying certificate already has `Method::Interval`,
     `budget_left`, `SoundEnclosure`; SET the new prop on a CLONE of its
     `PropMap` (or rebuild the certificate with the joined property map;
     `PropMap::set` is the join), keeping the other fields. `Err(e)` →
     `Err(e)` UNCHANGED (the passthrough of decision 1's doc).

3. **The vacuous-holds certificate** — the house structural pattern:
   `props.set(Prop::SameParameter, Truth::True)`, `method: Method::None`
   (nothing was computed), `budget_left: Budget::new(0, 0, 0)`,
   `margin: Margin::UNBOUNDED`, `modulus: Modulus::Unbounded`. The
   interval-certified path (decision 2's last bullet) keeps
   `Method::Interval` and the real `budget_left` — two different
   certificates for two different kinds of holding, both correct.

4. **Tests** — one `#[cfg(test)]` module opening with
   `#![deny(clippy::unwrap_used, clippy::expect_used)]` (H-1 justification
   comment), `use super::*;` plus the witness builders. Reuse BG-CE-002's
   landed witnesses (read `truck-evidence/src/decorators/pcurve.rs`'s test
   module and `deviation.rs`'s — copy the builders):

   - the oblique plane `Plane::new(o, (1,0,1), (0,1,1))`;
   - the 2D parabola `BSplineCurve<Point2>` (cps (0,0), (1/2,0), (1,1) on
     `bezier_knot(2)`);
   - the composed leader `BSplineCurve<Point3>` (cps (0,0,0), (1/2,0,1/2),
     (1,1,2));
   - a `Sphere` pcurve witness for the route-2 case (the meridional arc
     from pcurve.rs's tests).

   The required tests:

   - `same_parameter_exact_pcurve_edge_holds` — an
     `Edge<usize, BSplineCurve<Point3>, PCurve<BSplineCurve<Point2>,
     Plane>>` (vertices `Vertex::news([0usize, 1usize])`, leader as the
     curve, `with_pcurve(PCurve::new(parabola2, plane))`),
     `phi = ParamMap::IDENTITY`, `tau` from
     `ToleranceCtx::new(1.0, TOLERANCE, TOLERANCE, TOLERANCE)`'s
     `entity_tau(TOLERANCE)` (**NOT** `unscaled_legacy()` — GATE-4's ratchet
     is at its ceiling and counts constructor call sites; build the
     numerically identical context through `ToleranceCtx::new` exactly as
     BG-CE-002's landed tests do): `Ok`, and
     `props.get(Prop::SameParameter) == Truth::True`.
   - `same_parameter_offset_pcurve_edge_violates` — the same edge with the
     leader translated by `2.0 * tau` in z (add it to every control point):
     `Err(Refusal::ForwardToleranceExceeded { bound, allowed })` with
     `bound > tau` and `allowed == tau`. **The spec's named negative test:
     an edge whose pcurve is deliberately offset by 2·τ must fail.**
   - `same_parameter_none_pcurve_is_vacuously_ok` — an
     `Edge<usize, BSplineCurve<Point3>, ()>` (no pcurve attached): `Ok`
     with `method == Method::None`.
   - `same_parameter_route2_offset_violates` — a ROUTE-2 pair (the carrier
     is a `PCurve<BSplineCurve<Point2>, Sphere>` — a curved surface, not
     flattenable) against a leader offset by a named constant well above
     tau (say `4.0 * tau`), span `[0.1, 0.9]` of the parameter curve, a
     generous budget (`Budget::new(1 << 20, 0, 0)`), tau at
     `1.0e-3`-class (named const, `// H-3:` naming it as the route-2 test
     tolerance): `Err(ForwardToleranceExceeded)` — the bisection route
     proves the violation too.
   - `same_parameter_zero_budget_is_unresolved` — the same route-2 pair
     with `Budget::new(0, 0, 0)`: `Err(Refusal::NumericallyUnresolved …)`
     with `witness == UnresolvedWitness::DeviationUncertified` (the variant
     BG-CE-002 landed).

   Keep every float literal out of `1e-N` form or behind a named const with
   a same-line `// H-3:` comment (H-3 section below).

5. One doctest on `check_edge`: the vacuous case (`Edge<_, _, ()>`) —
   `check_edge(&edge, ParamMap::IDENTITY, tau, &mut budget).is_ok()`.

## H-3, the house rule that rejects bare float literals

GATE-2 of `scripts/kernel-gates.sh` fails any **added** line carrying a bare
`1e-N` literal unless that line ends with an `// H-3` comment naming the
dimensionless quantity. Witness coordinates (`0.5`, `1.0`, `2.0`) are safe;
the route-2 tau and any slack constants go through named consts with the
same-line `// H-3:` comment, exactly as BG-CE-002's landed tests do. Run
`bash scripts/kernel-gates.sh` yourself before writing `RESULT.json`.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-topology
cargo clippy -p truck-topology --all-targets --no-deps
cargo test -p truck-topology --lib --tests --no-fail-fast
cargo test -p truck-topology --doc
cargo check --workspace --all-targets
bash scripts/kernel-gates.sh <your base commit>
```

Never run a bare `cargo test`. The crate is clean at baseline (all tests,
116 doctests, zero clippy findings, measured at HEAD 49997d3); your bar is
everything above stays green plus your five tests and one doctest.

## Forbidden

Editing any file outside `write_allow`. Calling `unscaled_legacy()` anywhere
(GATE-4 ratchet). Swapping the leader/carrier order in `certify_deviation`.
Collapsing `ForwardToleranceExceeded` into `Contradictory`. Adding
`#[ignore]`. Adding `unwrap()`/`expect()` outside the test module.

## Stop conditions

- an anchor count differs → `ANCHOR_MISMATCH`, naming the file and what you saw
- `certify_deviation`'s signature or `ParamMap`'s constructors differ from
  what this packet describes → adapt to the LANDED code and note it in
  `deviations`; only stop if the adapter cannot be written
- three consecutive failed `cargo` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit on the current branch with subject
`feat(topology): same-parameter invariant checker (BG-INV-104)`.
