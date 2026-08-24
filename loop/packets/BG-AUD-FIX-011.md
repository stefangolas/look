# WORK PACKET BG-AUD-FIX-011 — SameParameter semantics after cuts (AUD-014)

You are repairing one defect found by the formal-kernel correctness audit
`loop/audits/BG-AUDIT-001.md` (finding AUD-014), in
`truck-topology/src/invariants/same_parameter.rs`. Everything you need is in
this document. **Do not read any other spec file** — this packet is
self-contained.

```json
{"id":"BG-AUD-FIX-011","status":"DONE","contracts":["AUD-014"],
 "tests_added":2,"deviations":[],"disagreements":[],
 "baseline_failures":[],"notes":"free text"}
```

Fill `deviations`, `disagreements` and `baseline_failures` as arrays, empty if
empty. **If anything below contradicts what you find in the code, say so in
`disagreements` rather than making the code match the packet.**

```yaml
id:          BG-AUD-FIX-011
contract:    [AUD-014]
class:       design
crates:      [truck-topology]
write_allow:
  - vendor/truck/truck-topology/src/invariants/same_parameter.rs
read_allow:
  - vendor/truck/truck-topology/src/edge.rs
  - vendor/truck/truck-base/src/evidence.rs
tests_required:
  - same_parameter_none_pcurve_does_not_certify
  - same_parameter_pre_cut_half_does_not_certify
budget:      {turns: 30, ctx_tokens: 80000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'fn check_edge' vendor/truck/truck-topology/src/invariants/same_parameter.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'fn vacuous_holds' vendor/truck/truck-topology/src/invariants/same_parameter.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'fn pre_cut' vendor/truck/truck-topology/src/edge.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'SameParameter' vendor/truck/truck-base/src/evidence.rs"}
```

## Problem

`Prop::SameParameter` means "every edge use's parametric trace agrees with the
edge's leader curve over the whole span". `check_edge` currently returns
`SameParameter = True` VACUOUSLY whenever `edge.pcurve()` is `None`
(same_parameter.rs:84-110, `vacuous_holds`). `Edge::pre_cut` drops the pcurve
on BOTH halves (edge.rs:626-651) — a decided, documented behavior (the spec
records that "pre_cut drops the trace on both halves... the packet that wires
real pcurves owns trace splitting"). So after `Shell::cut_edge` every cut edge
re-certifies `SameParameter = True` with NO trace to check: the absence of a
trace is silently promoted to positive evidence that the parametric trace
agrees with the leader. The module doc calls this "vacuously satisfied", but
the emitted prop is the full invariant's prop, and a consumer reading
`SameParameter = True` as "the trace agrees" is told so about edges whose
trace was discarded.

**Decided semantic (owner amendment, recorded here):** absence of a trace is
**not applicable**, not a hold. `check_edge` on an edge with no pcurve returns
`Ok` (nothing was violated — there is no trace to disagree), but its
certificate must NOT set `Prop::SameParameter = True`; the prop stays `Unknown`.
This is the house pattern already used elsewhere in this crate family: an
operation that inspects but does not certify the invariant returns `Ok` with an
empty `PropMap` and `method: Method::None`.

**Your first obligation — observe the regression fail on the buggy code:** add
the two tests below; both must FAIL on the current code (the vacuous arm sets
`SameParameter = True`). Record the pre-fix observations in
`RESULT.json.notes`.

## Repair

In `invariants/same_parameter.rs`, change `vacuous_holds` so it does NOT set
`Prop::SameParameter`:

```rust
/// The no-trace certificate: `method: Method::None` (nothing was computed),
/// and `SameParameter` stays `Unknown` because the absence of a trace is NOT
/// evidence that the parametric trace agrees with the leader.
fn vacuous_holds() -> Outcome<()> {
    Ok(Certified::new(
        (),
        Certificate {
            props: PropMap::new(),
            method: Method::None,
            budget_left: Budget::new(0, 0, 0),
            margin: Margin::UNBOUNDED,
            modulus: Modulus::Unbounded,
        },
    ))
}
```

Update the module doc and the `check_edge` doc: the no-trace case is
not-applicable (prop `Unknown`), NOT vacuously-true. Update the existing test
`same_parameter_none_pcurve_is_vacuously_ok` to also assert
`certified.cert.props.get(Prop::SameParameter) == Truth::Unknown`. The doctest
(asserts `.is_ok()`) stays green. The attached-pcurve path
(`certify_trace`, which sets `SameParameter = True` after a real
`certify_deviation`) is unchanged.

## Regression tests (exact names)

1. `same_parameter_none_pcurve_does_not_certify` — an edge
   `Edge::<usize, BSplineCurve<Point3>, ()>` with the `()` pcurve default and
   the `leader_witness()` curve (the existing test module's witness), checked
   with `ParamMap::IDENTITY` and a real tau. Assert `check_edge(...).is_ok()`,
   `method == Method::None`, and
   `props.get(Prop::SameParameter) == Truth::Unknown` — the absence of a trace
   must NOT certify the invariant.

2. `same_parameter_pre_cut_half_does_not_certify` — build the same edge and
   cut it through the public API so both halves drop the pcurve (e.g.
   `edge.cut_with_parameter(&vertex, 0.5)` or the `pre_cut` path the public
   cut uses, with a `BSplineCurve` leader so `Cut` is available). Take one
   half, `check_edge` it, and assert the same: `Ok`, `Method::None`, prop
   `Unknown`. This is the audit's exact provenance (a cut edge re-certifying).

All other tests in `same_parameter.rs` (the exact-pcurve edge, the offset
violation, the route-2 sphere pair, the zero-budget refusal) must stay green —
they certify real traces and are unaffected.

## H-3, the house rule that rejects bare float literals

GATE-2 of `scripts/kernel-gates.sh` fails any **added** line carrying a bare
`1e-N`-shaped literal unless that line ends with an `// H-3` comment. This
packet adds no float literals (the tau comes from the existing
`legacy_tau()`). Run `bash scripts/kernel-gates.sh <your base commit>` yourself
before writing `RESULT.json`.

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

Editing any file outside `write_allow` (edge.rs's `pre_cut` is a read-only
dependency — the drop-trace behavior is a decided spec contract). Setting
`SameParameter = True` for a missing trace anywhere. Returning a refusal for
the no-trace case (the decided semantic is `Ok` with the prop `Unknown`).
Adding `#[ignore]`.

## Stop conditions

- an anchor count differs → `ANCHOR_MISMATCH`, naming the file and what you saw
- the cut through the public API cannot be expressed against the real `Edge`
  / `Cut` signatures in the test module → `SPEC_GAP`, with the exact mismatch
  and the closest constructible cut witness
- three consecutive failed `cargo` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit on the current branch with subject
`fix(topology): no-trace same-parameter is not-applicable, not certified (BG-AUD-FIX-011)`.
