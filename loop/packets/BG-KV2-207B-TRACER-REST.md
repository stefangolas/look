# WORK PACKET BG-KV2-207B-TRACER-REST â€” the escalation-lattice remainder (the bounced S4A/307 follow-up)

Three `kernel_tracer` tests have been red since landing (BG-KV2-207-S4A
landed 5/8; BG-KV2-307 bounced the remainder because tracer.rs was outside
its write_allow). This packet OWNS them: `write_allow` includes tracer.rs.
The prior partials are the design record. Full audit in
`loop/results/BG-KV2-207B-AUDIT.md` (READ IT FIRST â€” per-test mechanisms,
constants, verified algebraic identities).

```yaml
id:          BG-KV2-207B-TRACER-REST
contract:    [BG-KV2-207B-TRACER-REST]
class:       design
crates:      [truck-certified]
depends_on:  []
write_allow:
  - vendor/truck/truck-certified/src/kernel/tracer.rs
  - vendor/truck/truck-certified/tests/kernel_tracer.rs
read_allow:
  - vendor/truck/truck-certified/src/kernel/
  - vendor/truck/truck-certified/src/formal/exact.rs
  - loop/results/BG-KV2-207-S4A.json
  - loop/results/BG-KV2-307-ENGINEREACH.json
tests_required:
  - all 8 kernel_tracer tests green
  - kernel_engine still 15/15
  - tracer_output_never_claims_certification stays green (frozen seam)
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'RANK_SUBBOXES: usize = 16' vendor/truck/truck-certified/src/kernel/tracer.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'ISOLATED_CAP: usize = RANK_SUBBOXES / 4' vendor/truck/truck-certified/src/kernel/tracer.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'at most two' vendor/truck/truck-certified/src/kernel/tracer.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'c2_certify_tube4(' vendor/truck/truck-certified/src/kernel/tracer.rs"}
budget:      {turns: 90, ctx_tokens: 220000}
```

## Problem â€” three distinct mechanisms (audit-verified)

1. `escalation_routes_isolated_r2_to_the_contact_future`: one isolated node
   statically predicts collapse = 2 (<= ISOLATED_CAP = 4) but observes
   5..=15 (the HighOrderJet bucket). The gv leg identically straddles
   (screen centered on the branch), so the all-four-minors test degenerates
   to the gu leg; the isolated/HighOrderJet boundary is a razor-thin
   smear-vs-advance race with NO designed margin: worst case
   `1 + 2*SCREEN_PERP*RANK_SUBBOXES*sqrt(3) ~= 3.77` vs cap 4.
2. `high_order_singularity_refuses`: three nodes, but `gu = P(v)` is
   degree-3 with between-root dips ~1.8e-9 while the single-pass interval
   hull width is ~7e-5 â€” every sub-box reads `gu âˆ‹ 0` => collapse = 16 =>
   TangentialCurve. Screen RESOLUTION, not constants.
3. `dtau_grows_on_success_and_halves_on_failure`: `gv = 1` on the parabola
   => collapse = 0 everywhere => the ladder can only Rebuild =>
   Conditioning; there is NO honest terminal class for "the frozen seam
   certifies nothing here" (the 307-named gap). The trace exits
   Completed with 0 certified steps.

## Scope decisions â€” ordered work plan

1. **DIAGNOSTIC FIRST (cheap, mandatory):** instrumented run through the
   `#[doc(hidden)] float_trace_impl` (tracer.rs:196-227) dumping for each
   failing fixture: live q_tau/q_perp at the seed, tau_local/dtau at the
   escalate call, per-sub-box gu/gv enclosure widths, collapse count.
   Numbers go in RESULT. This settles mechanism 1's discrepancy
   (predicted 2, observed 5..=15) BEFORE the redesign.
2. **Screen resolution (mechanisms 1+2):** certified bisection inside the
   screen â€” subdivide a sub-box until each minor's hull width is below a
   designed fraction of the box's geometric scale; a sub-box collapses only
   if EVERY child still has all four minors containing zero. Pure enclosure
   refinement (D4 doctrine: certified enclosures only). THEN revisit rung-5
   semantics: three distinct isolated nodes are genuinely NOT one isolated
   contact NOR a 1-D curve â€” decide whether rung 5 keys on footprint
   STRUCTURE (multiplicity of separated collapse clusters) rather than raw
   count, and update the module doc ladder contract (lines 46-59,
   INCLUDING the stale "at most two sub-boxes" drift at lines 57-58 â€”
   d6282a9 raised the cap to 4).
3. **Designed margin (mechanism 1):** derive SCREEN_PERP from the cap
   (`SCREEN_PERP <= (ISOLATED_CAP-1)/(2*RANK_SUBBOXES*sqrt(3))` with
   margin) or derive the cap from the smear bound â€” the derivation goes in
   the doc. Keep the `// H-3` marker on the defining line (tracer.rs:90).
4. **Honest terminal class (mechanism 3):** add the
   rebuild-exhausted-with-zero-collapse -> Budget/Residual-backed refusal
   path in trace_march (tracer.rs:984-1016) â€” the rung BG-KV2-307 named as
   the gap. The parabola test then asserts the halving/growth mechanism via
   the policy machinery OR the fixture is re-derived so the seam's measured
   reach admits a halved step. Prefer the terminal-class fix (fixtures
   stay frozen).
5. **Frozen constraints (mechanically enforced by
   tracer_output_never_claims_certification):** exactly ONE
   `c2_certify_tube4(` call site; never construct ArcCert; the literals
   `certified: Option<ArcCert<4>>` / `certified: Some(` stay;
   `TracePolicy::default` values are load-bearing for the five passing
   tests. H-1: no unwrap/expect/panic in new code. H-3 markers stay on
   defining lines (PERP_RATIO 74, CLOSE_TOL 80, SCREEN_PERP 90).

## Done when

```
cargo test -p truck-certified --test kernel_tracer   (8/8)
cargo test -p truck-certified --test kernel_engine   (15/15)
cargo test -p truck-certified --lib
```

## Forbidden

Touching the frozen seam. Fixture edits unless step 4 forces them (record
the derivation). Constant changes without a derived margin argument.
Weakening any passing test.

## Stop conditions

- The diagnostic run contradicts the audit's algebraic model (live frame
  wildly different from the GS prediction) -> record the numbers, STOP,
  SPEC_GAP with the dump.
- Subdivision refinement cannot separate the three nodes at any depth
  within budget -> the honest answer may be that rung-5 must key on
  structure; propose it in RESULT, do not improvise past the doctrine.

## Finish by writing RESULT.json at the WORKTREE ROOT (then COMMIT first)

Commit subject: `feat(certified): escalation-lattice remainder â€” screen bisection, derived margins, honest Budget terminal class (BG-KV2-207B-TRACER-REST)`.
