# WORK PACKET FSSI-001-GATE — the separable tangency-free admissibility certificate

FSSI-0 of the theory spec ([`FSSI_BUILD_SPEC.md`](../../docs/FSSI_BUILD_SPEC.md)
§3, packet 2): discharge `dim Σ = 0` BEFORE the event system, with typed
refusals. This is the piece that engages the known failure surface —
near-tangency and coincidence — at SSI admission, below the contact funnel
(FSSI-LAYER: `truck-shapeops/boolean/classify.rs` is off-limits for the
whole program; DEF-SEEDRAY-B owns it).

```yaml
id:          FSSI-001-GATE
contract:    [FSSI-001-GATE]
class:       design
crates:      [truck-certified]
depends_on:  [FSSI-000-CONTRACT]
write_allow:
  - vendor/truck/truck-certified/src/ssi_gate.rs
  - vendor/truck/truck-certified/src/ssi.rs
  - vendor/truck/truck-certified/src/ssi_types.rs
  - vendor/truck/truck-certified/src/construct/bie/ssi4.rs
  - vendor/truck/truck-certified/tests/ssi_gate_conformance.rs
read_allow:
  - docs/FSSI_BUILD_SPEC.md
  - vendor/truck/truck-certified/src/
  - vendor/truck/truck-evidence/src/
tests_required:
  - gate_admit_certifies_tangency_free_box
  - tangent_curve_fixture_refuses_named_case
  - coincident_patch_fixture_refuses_named_case
  - near_tangent_rank3_pair_passes_gate
  - loft_apex_pole_fixture_passes_gate
  - v5_pair_identity_on_green_spline_pairs
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'deny(clippy::unwrap_used)' vendor/truck/truck-certified/src/ssi_gate.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub fn gate_admit' vendor/truck/truck-certified/src/ssi_gate.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'TangentCurveSuspected' vendor/truck/truck-certified/tests/ssi_gate_conformance.rs"}
budget:      {turns: 80, ctx_tokens: 200000}
```

## Pre-digested context

- **The certificate (theory §1.1).** On box `B` with `B ∩ P = ∅`: if the
  interval enclosure of `‖n_X × n_Y‖(B)` has a strictly positive lower
  bound, then `Σ ∩ B = ∅` and `rank DF = 3` throughout `B ∩ M`. The test is
  **separable**: `n_X` depends only on `(u,v)`, `n_Y` only on `(s,t)` —
  compose through the CFP-003 separability discipline (hull the `(u,v)`
  slice per `(s,t)` control index, then hull over `(s,t)`); NEVER
  materialize the O(deg⁴) product grid.
- **The substrate**: `hull_bernstein_2d` / `TensorGrid4` / `bernstein_box4`
  (`hull.rs:95-117`, `interval/bounds.rs:111-208`); normal nets are already
  carried per side by `SquareSystem3`. One interval algebra:
  `formal::exact::CertifiedInterval`.
- **FSSI-ELEV**: any product-of-enclosed-factors hull (and
  `n_X × n_Y` is exactly that) uses the elevated pairing discipline — the
  naive pairing can emit a false loop-free certificate
  (`truck-evidence/src/contact/implicit2d.rs:21-30`). The elevation must be
  findable in the code by a reviewer.
- The suspicion halt (theory §1.2): budget-exhausted undecided measure is
  compared against shrinkage like `2⁻⁴ᵏ` across levels. Area-scale
  stagnation ⇒ `CoincidentPatchSuspected`; slower ⇒
  `TangentCurveSuspected`. The criterion fires ONLY in the refusal
  decision — it can cause a premature refusal, never a wrong acceptance.

## Scope decisions

1. **Entry point** `pub fn gate_admit(...) -> Result<GateAdmission, SsiRefusal>`
   in the new `ssi_gate.rs`: per-box verdict {TangencyFree, Subdivide,
   Suspect(...)}; wired at `SquareSystem3` admission and the `ssi4` pair
   entry. Boxes that certify TangencyFree proceed exactly as today; the
   gate is **monotone-widening**: it can only admit or refuse, never flip a
   landed certified verdict (V5-pair identity is a required test).
2. **Parametric poles (theory §4.1)** are handled by composition with the
   frozen F3 rule: the `B ∩ P = ∅` precondition is discharged by the
   selector's surviving coordinate. REQUIRED fixture: a loft with a
   collapsed apex row must PASS the gate (`loft_apex_pole_fixture_passes_gate`)
   — this is the regression that kills the naive global-`q_t` design.
3. **Near-tangency with rank 3 PASSES** (theory §1.3): conditioning cost,
   not a correctness hazard. The conformance battery pins a small-`‖n_X×n_Y‖`
   fixture certifying, with the short tube/deep-subdivision cost visible in
   the returned margin, not refused.
4. **No boolean-output change**: no `truck-shapeops` file is writable, and
   the gate sits below the funnel. If any landed green test changes verdict
   class, that is a DEFECT RECORD (stop-and-file), never an accepted
   regression.

H-1: no panics; `#![deny(clippy::unwrap_used)]` on the new module. H-3:
every constant named on its defining line. H-6: floats never enter evidence.
SFC: float subdivision ordering may search; the certificate is the interval
enclosure computed independently.

## Done when

```
cargo fmt --check -p truck-certified
cargo clippy -p truck-certified --all-targets -- -D warnings
cargo test -p truck-certified --lib --tests
```

with all six named tests green, the V5-pair identity battery green, and the
anchors holding.

## Forbidden

`truck-shapeops` writes (any layer). Weakening the F3 rule or "retrying with
a weaker test" on a refused box. Catch-all refusal arms. Editing the Krawczyk
operator or `num/parallelotope.rs`. Materializing the product grid.

## Stop conditions

- The separable composition cannot be certified without a NEW hull kernel
  → stop-and-file with the kernel named (FSSI-002's cost center must not
  leak into this packet).
- The suspicion halt cannot distinguish area-scale from sub-linear
  shrinkage on the fixtures → SPEC_GAP naming the measure actually
  observable (do not tune silently).

## Finish by writing RESULT.json at the WORKTREE ROOT (then COMMIT first)

Commit subject: `feat(certified): separable tangency-free gate at SSI admission — typed suspicion refusals, pole composition (FSSI-001)`.
