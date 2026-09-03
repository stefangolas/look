# BG-KV2-101-K0AUDIT — survey: numerics-contract audit over the certificate paths

Survey-class packet (ORCHESTRATOR "The survey class"): **no write access to
vendor/truck at all**. Deliverable: `SURVEY.json` at the worktree root, one
row per site, V10-checked for anchor reality. The judgements proposed here
feed the rational-reparameterization migration packets (build spec §4, Wave
2+); nothing is implemented by this packet.

Normative basis: v2 spec §1 (N1–N7), §0.4, §3.2. Build spec §1 census row K0
is the starting hypothesis — MEASURE every claim before echoing it.

```yaml
id:          BG-KV2-101-K0AUDIT
contract:    [BG-KV2-101-K0AUDIT]
class:       survey
crates:      []
depends_on:  [BG-KV2-000-CONTRACT]
write_allow:
  - SURVEY.json
read_allow:
  - docs/CONSTRUCTIVE_GEOMETRY_KERNEL_SPEC_V2.md
  - docs/KERNEL_V2_BUILD_SPEC.md
  - vendor/truck/truck-evidence/src/elementary.rs
  - vendor/truck/truck-evidence/src/enclosure.rs
  - vendor/truck/truck-evidence/src/decorators/offset.rs
  - vendor/truck/truck-evidence/src
  - vendor/truck/truck-certified/src
  - vendor/truck/truck-geometry/src
  - scripts/kernel-gates.sh
budget:      {turns: 24, ctx_tokens: 90000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub mod elementary' vendor/truck/truck-evidence/src/lib.rs"}
  - {id: A2, expect: 0, cmd: "grep -rn 'par.sum\\|par_sum' vendor/truck | wc -l"}
  - {id: A3, expect: 1, cmd: "grep -c 'pub use inari::Interval' vendor/truck/truck-evidence/src/enclosure.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'deny(clippy::unwrap_used)' vendor/truck/truck-certified/src/lib.rs"}
tests_required: []
```

## The audit (SURVEY.json row classes)

One row per site: `{file, line, symbol, expression, proposed_class, reason,
confidence}`. Classes (this program's vocabulary, not the old model/param):

- `quarantine-rational` — a transcendental evaluation feeding a CERTIFICATE
  or enclosure (N4 violation). Proposed migration: rational reparameterized
  carrier or polynomial-form replacement, with the concrete replacement
  named. Expected seed population: `elementary.rs` interval sin/cos consumed
  by torus/sphere/cone/circle enclosure impls (census hypothesis:
  `torus.rs:22`, `cone.rs:63,117` — re-derive all consumers by grep, both
  crates).
- `predictor-only` — transcendental evaluation on a float path whose result
  is disposed of by intervals (D4-legal). Requires the caller chain as
  evidence, not the call site alone.
- `compliant` — checked and clean (order-pinned reductions, index-stable
  parallelism, directed rounding present).
- `config-gap` — a §0.4 constant the tree lacks or a default that differs
  (DirectTolerance 1e-6 vs normative 1e-9/1e-11/1e-12 — recorded deviation,
  build-spec decision 2; rows here just re-measure the consumers of each
  field so the later migration packet knows the blast radius).
- `n7-candidate` — a predicate with both an interval form and a Bernstein
  form where the two-stage rule (§1 N7) should be asserted.

Minimum row coverage (each verified by command, zero from memory):
1. EVERY consumer of `elementary.rs`'s interval sin/cos/exp/log/atan2 across
   vendor/truck (grep the pub fn names; include tests as excluded rows).
2. Every `par`-iterator reduction that produces a float sum (expect none —
   A2; record the index-stable collect sites as compliant).
3. Every `normalize()` call inside `truck-certified/src` and
   `truck-evidence/src` without a preceding certified-norm gate (N6) —
   `enclosure.rs:109-125` `midpoint_ball_cone` is the compliant exemplar.
4. The directed-rounding primitives inventory (one row each: `next_after`,
   `toward_neg`, `toward_pos`, `DeckInterval`, `CertifiedInterval`,
   `two_sum`, `two_product`) with their consumers.
5. `SamplingPolicy::CustomParameters` call sites (the §4.3 shared-edge guard
   lands in a later wave; the rows here are its blast-radius map).

RESULT.json notes carry: total rows per class, the three sites with
confidence low, and any N4 violation the census hypothesis MISSED.

## Done-when

- `SURVEY.json` at the worktree root, well-formed, every row's (file, line,
  symbol) resolvable against the tree (V10).
- RESULT.json AT THE WORKTREE ROOT with the class totals.

## Stop conditions

1. A census-hypothesis row does not reproduce by command — record the
   actual measurement; never echo the build spec's numbers.
2. A `quarantine-rational` candidate has no obvious rational replacement —
   still file the row with confidence low and name the obstruction; do not
   invent a replacement.
