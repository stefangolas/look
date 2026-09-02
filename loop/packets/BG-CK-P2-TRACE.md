# BG-CK-P2-TRACE — certified branch tracing + coordinate switching (wave W2)

Wave member W2 of the Phase-2 implementation wave
(`docs/CERTIFIED_INTERLEAVE_BUILD_SPEC.md`; ORCHESTRATOR "wave mode").
Implements the continuation loop against the shim's frozen types and the
fixture kit. W1 (BG-CK-P2-SYSTEM) implements the constructor and the 3×3
Krawczyk certificate in a DIFFERENT new module; you do not wait for it and
you do not read its code — the contracts you share are frozen in the shim
(`ssi_types.rs`), and production substitution happens at integration.

LOCAL_GREEN is not DONE: your RESULT.json is a claim. No wave worker runs
the global verifier.

```yaml
id:          BG-CK-P2-TRACE
contract:    [BG-CK-P2-TRACE]
class:       design
crates:      [truck-certified]
depends_on:  [BG-CK-P2-CONTRACT]
write_allow:
  - vendor/truck/truck-certified/src/ssi_trace.rs
  - vendor/truck/truck-certified/src/lib.rs
  - vendor/truck/truck-certified/tests/ssi_trace.rs
read_allow:
  - docs/CERTIFIED_PHASE2_BOOKING.md
  - docs/CERTIFIED_INTERLEAVE_BUILD_SPEC.md
  - vendor/truck/truck-certified/src/ssi_types.rs
  - vendor/truck/truck-certified/src/ssi_fixtures.rs
  - vendor/truck/truck-certified/src/contract.rs
  - vendor/truck/truck-certified/src/formal/contact.rs
  - vendor/truck/truck-certified/src/formal/span.rs
  - vendor/truck/truck-certified/src/formal/bezier_isect.rs
budget:      {turns: 34, ctx_tokens: 130000}
anchors:
  - {id: A1, expect: TBD, cmd: "grep -c 'pub enum TraceOutcome' vendor/truck/truck-certified/src/ssi_types.rs"}
  - {id: A2, expect: TBD, cmd: "grep -c 'pub struct TraceStep' vendor/truck/truck-certified/src/ssi_types.rs"}
  - {id: A3, expect: TBD, cmd: "grep -c 'pub enum BranchGerm' vendor/truck/truck-certified/src/formal/span.rs"}
  - {id: A4, expect: 0, cmd: "ls vendor/truck/truck-certified/src/ssi_trace.rs 2>/dev/null | wc -l"}
  - {id: A5, expect: TBD, cmd: "grep -c 'closed_loop_pair' vendor/truck/truck-certified/src/ssi_fixtures.rs"}
  - {id: A6, expect: TBD, cmd: "grep -c 'pub struct CoordinateSwitch' vendor/truck/truck-certified/src/contract.rs"}
tests_required:
  - trace_loop_walks_fixture_closed_loop_to_identity_recurrence
  - trace_loop_terminates_at_domain_boundary
  - trace_switch_requires_both_certificates_and_refuses_otherwise
  - trace_germ_classification_reads_next_nonzero_jet
  - trace_steps_carry_branch_incidence_records
  - trace_refusals_are_named_cases
```

## The seam contract (frozen here so integration composes — read this twice)

You cannot call W1's solver: it does not exist in your tree and you must
not invent its API. The loop therefore drives a SOLVER-PRIVATE step
interface defined in YOUR module:

```rust
/// One per-box Krawczyk step, as the loop consumes it. Solver-private
/// (the HULL precedent): NOT public API, not re-exported. At integration
/// the orchestrator adapters W1's certificate evaluator to this shape.
pub(crate) trait BranchCertifier {
    /// Certify one parameter box along the branch: produce the TraceStep
    /// (box, germ, incidence, chosen coordinate) or refuse the named way.
    fn step(&self, box_hint: &BranchBox) -> Result<TraceStep, TraceRefusal>;
}
```

The exact shape (`BranchBox` naming, argument order, error side) is YOURS
to fix — but it stays `pub(crate)`, and your tests drive the loop with
SYNTHETIC certifiers (hand-written `impl` blocks walking known fixture
geometry), never a real Krawczyk evaluation (that is W1's; the
integration amendment plugs it in). This is the wave-mode
measurement/implementation split: you build the LOOP against fixture
outcomes; the production substitution is one adapter at integration.

## Section 1 — the continuation loop (`src/ssi_trace.rs`, NEW)

- Seed from an isolated Krawczyk certificate (the shim's
  `KrawczykCertificate3` shape), step boxes along the branch, per box
  the frozen coordinate selection (via the step's `TraceStep`, which
  carries the chosen `ContinuationCoordinate`).
- Identity recurrence: a step whose box equals the first box's identity
  closes the branch — `TraceOutcome::ClosedLoop` (the
  `closed_loop_pair()` fixture's ground truth).
- Domain exit with no refusal: `TraceOutcome::Terminated`.
- **Turning-point switching is the frozen both-certificate rule**: at a
  certified turning point, a `CoordinateSwitch` event is emitted ONLY
  with BOTH certificates (the frozen contract in `contract.rs`: no
  default, no heuristic reseed). A certifier that returns one
  certificate at a switch box is a REFUSAL, never a reseed — the
  implementation must refuse, never reseed. The loop's own discipline:
  if the step reports a switch request without both certificates,
  return `TraceOutcome::Refused(...)` with the named case.
- Branch records per box: `BranchIncidence`-shaped (mapping row 3 —
  span + certified parameter enclosure + branch germ + deck label,
  `formal/contact.rs`), germ classification via `BranchGerm`
  (`formal/span.rs`): a zero first jet reads the next nonzero jet —
  the span.rs discipline. The germ LADDER is realized by the
  `germ_ladder()` fixture; classification correctness is machine-checked
  against it.
- Every refusal is a named case wrapped through `TraceRefusal`
  (shim vocabulary over landed causes); no catch-all, no stringly
  refusal.

## Section 2 — tests (`truck-certified/tests/ssi_trace.rs`, NEW)

The six `tests_required`, driven by synthetic certifiers over the
fixture kit (through `truck_certified::ssi_fixtures`, the crate's public
path):

1. `trace_loop_walks_fixture_closed_loop_to_identity_recurrence` — a
   synthetic certifier walking `closed_loop_pair()`'s branch returns
   `ClosedLoop` with the recurrence asserted (first box id == closing
   box id, steps non-empty).
2. `trace_loop_terminates_at_domain_boundary` — a synthetic certifier
   whose branch leaves the domain returns `Terminated`.
3. `trace_switch_requires_both_certificates_and_refuses_otherwise` — a
   synthetic certifier reporting a switch WITH both certificates yields
   `Switched` carrying the frozen `CoordinateSwitch`; a certifier
   reporting a switch with ONE certificate yields `Refused` (never a
   reseed, never a default).
4. `trace_germ_classification_reads_next_nonzero_jet` — the
   `germ_ladder()` fixtures classify to `Regular`,
   `StationaryRegular{2}`, `CuspCandidate`, `Singular` per the span.rs
   next-nonzero-jet discipline; each ladder rung asserts its class.
5. `trace_steps_carry_branch_incidence_records` — every emitted
   `TraceStep` carries a `BranchIncidence`-shaped record (span +
   enclosure + germ + label round-trip).
6. `trace_refusals_are_named_cases` — every `TraceRefusal` variant your
   loop can emit wraps a landed named cause; the exhaustive match in
   the test has no catch-all arm.

## Wave-mode rules for this worker (binding)

- LOCAL checks only: `cargo check -p truck-certified`,
  `cargo test -p truck-certified --lib --tests`, scoped clippy on your
  diff, fmt on your files. Do NOT run workspace-wide gates, do NOT run
  verify.py, do NOT touch other crates, do NOT read or anticipate
  W1's module (`src/ssi.rs` must not exist for you; if a branch merge
  delivers it, ignore it and work to the shim).
- The write set is exactly the yaml. lib.rs takes ONE line
  (`pub mod ssi_trace;` — a one-line textual conflict with W1's line is
  expected and resolved at integration).
- H-1: crate-level `deny(clippy::unwrap_used)` covers the new module;
  no `unwrap`/`expect`/`panic!`, no module-level allow.
- F3 is law: the both-certificate switching and no-weaker-retry
  discipline are implemented exactly as frozen; a needed widening is an
  orchestrator spec edit, never a worker decision.
- No spline emission (F1): the trace emits branch records, never shell
  evidence annotations.
- If you discover a real contract ambiguity in the shim's types, STOP
  and SPEC_GAP it (commit nothing beyond WIP evidence) — do not invent
  an interpretation.

## Done-when

- The six `tests_required` functions exist and pass
  (`cargo test -p truck-certified --lib --tests --no-fail-fast` green,
  landed suites unchanged).
- fmt clean on your files; clippy `-p truck-certified` zero findings
  attributable to your diff.
- `cargo check --workspace --all-targets` green (the lib.rs line ripple).
- RESULT.json AT THE WORKTREE ROOT (claim, not verdict), commit on the
  current branch first (subject: `feat(certified): SSI branch tracing +
  coordinate switching (BG-CK-P2-TRACE)`).

## Stop conditions

Stop, commit nothing beyond WIP evidence, write RESULT.json AT THE
WORKTREE ROOT with the finding verbatim if:

1. The shim's landed types differ from this packet's read (a frozen
   shape moved between shim authoring and your dispatch) — stop, do not
   adapt silently.
2. The `BranchGerm` ladder cannot be realized by synthetic certifiers
   without simulating jet evaluation the fixtures do not provide —
   record which rung and what is missing; the fixture kit is frozen.
3. The both-certificate rule cannot be expressed without weakening it
   (e.g. the switch case needs a context the shim type does not
   carry) — record the exact type mismatch; the rule is frozen.
