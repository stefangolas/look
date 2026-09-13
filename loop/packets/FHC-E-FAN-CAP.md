# WORK PACKET FHC-E-FAN-CAP — the degenerate planar fan cap (register 4b)

Gap-register 4b: `f1/suspension_front` and `f1/suspension_rear` block at the
`_rocker` lightening cut (`suspension.py:503`) — a coaxial spline-profile
prism through-cut (`_plate` body minus a thicker `_plate` tool). The
trim-extrude envelope extracts it EXACTLY (tensor-Bernstein 2-cycle), but the
planar cap is a fan whose normal cone **degenerates at the loop centroid**, so
the landed RDEF-M2 tangential-sandwich admission refuses typed
`SingularParametrization`. Both duplicate TRIM workers converged on this
verdict independently.

```yaml
id:          FHC-E-FAN-CAP
contract:    [FHC-E-FAN-CAP]
class:       mechanical+
crates:      [truck123d]
depends_on:  [FHC-D-SURFACE-RESIDUE]
needs:       [FHC-D-SURFACE-RESIDUE]
write_allow:
  - truck123d/src/bd_bridge.rs
  - truck123d/tests/fan_cap.rs
read_allow:
  - docs/F1_HYPERCAR_GAP_REGISTER.md
  - corpus/ttc/trees/f1/src/lib/suspension.py
tests_required: [truck123d/tests/fan_cap.rs]
anchors:
  - {id: A1, expect: 28, cmd: "grep -c 'SingularParametrization' truck123d/src/bd_bridge.rs"}
budget:      {turns: 55, ctx_tokens: 170000}
```

Anchor measured 2026-09-13 at the B-landed HEAD. **Re-measure at dispatch.**

## Pre-made judgements (the fix is known — register 4b)

1. **A planar fan cap is FLAT: its exact flux has a closed planar form.**
   For a planar polygon/fan region with the divergence field `F_a(x) = (x-a)/3`,
   the cap flux reduces to a 2-D computation over the loop (the planar
   Green's-theorem form) — no parametric normal cone is needed, so the
   degenerate centroid never enters the certificate. Implement the planar
   closed form as the cap's flux certificate whenever the cap region is
   certified planar (the prism's caps are planar by construction).
   Alternative (equal rank): re-parametrize the cap away from the centroid
   (e.g. two half-fans split at a non-degenerate apex). Pick ONE, name the
   choice in RESULT.
2. **Soundness invariants:** the chosen cap certificate must agree with the
   landed per-patch bracket on a NON-degenerate cap (before/after
   equality), must keep the `lo <= hi` bracket ordering under the orientation
   sign, and must leave every non-cap patch path untouched (bit-identical).
3. **Scope:** the planar-cap fix only. Non-planar degenerate caps stay
   typed-refused naming the case. The RDEF-M2 admission dichotomy itself is
   NOT edited — the planar cap takes the closed-form path BEFORE the
   tangential-sandwich admission would see it.
4. `suspension.py` is read-only context; the corpus is never tuned.

## Method

Construct the `_rocker` cut shape in a test (body minus over-extending
plate, per the corpus idiom), reproduce the `SingularParametrization`
refusal, implement the planar closed-form cap certificate, verify: the cut
composes through the certified solver, the bracket certifies, STL emits,
and the two rows' door spot-checks go green (or advance to their next
honest carrier). Tests: degenerate fan cap certified via the closed form;
non-degenerate cap before/after equality; orientation-sign bracket
ordering; the two suspension rows' compositions.

## Done when

```
cargo fmt --check -p truck123d
cargo clippy -p truck123d --all-targets -- -D warnings
cargo test -p truck123d --test fan_cap --locked
```

plus door spot-checks for both suspension rows. All cargo through the
queue (the `cargo` on PATH IS the queue shim); interpreter dir on PATH if
STATUS_DLL_NOT_FOUND.

## Forbidden

Any file outside `write_allow`; editing `corpus/ttc/trees/**` or
`vendor/truck/**`; editing the RDEF-M2 admission dichotomy's semantics;
numeric shortcuts; `#[ignore]`, deleted or weakened tests, bare
`cargo test`; committing to `main`.

## Stop conditions

- the planar closed form cannot be made to agree with the landed bracket on non-degenerate caps → `SPEC_GAP` naming the discrepancy (that is theory evidence, not failure)
- the suspension composition reaches a SECOND blocker after the cap fix → land the cap fix, record the next refusal typed, note it for the register
- anchor count differs at dispatch and was not re-measured → `ANCHOR_MISMATCH`
- three consecutive failed cargo runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` AT THE WORKTREE ROOT

```json
{"id":"FHC-E-FAN-CAP","status":"DONE","contracts":["FHC-E-FAN-CAP"],
 "anchors_verified":{"A1":28},
 "rows_flipped":["f1/suspension_front","f1/suspension_rear"],
 "notes":"closed form chosen (or re-parametrization); before/after bracket equality; per-row verdicts"}
```

Commit subject: `truck123d: closed-form planar fan-cap certificate (FHC-E)`.
