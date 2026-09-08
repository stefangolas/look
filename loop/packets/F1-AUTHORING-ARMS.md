# WORK PACKET F1-AUTHORING-ARMS — the door's missing census verbs: loft, sweep-as-loft-chain, mirror, make_face

Design packet. The five excluded/remaining F1 demand verbs refuse in the
drop-in today (`corpus/ttc/door.py`: loft:441, sweep:437, mirror:453;
make_face:424 records but wire-carrier checks refuse spline-trimmed faces).
This packet lands the RECORDING ARMS: each verb records its construction
data as census rows and maps to bridge `SolidSpec` variants with exact
facts — the revolve arm (FH-SPLINE-LATHE) is the template and the
line-profile special case stays bit-identical.

```yaml
id:          F1-AUTHORING-ARMS
contract:    [F1-AUTHORING-ARMS]
class:       design
crates:      [truck123d]
depends_on:  [FH-SPLINE-LATHE]
write_allow:
  - corpus/ttc/door.py
  - truck123d/src/bd_bridge.rs
  - truck123d/tests/ttc_authoring_arms.rs
read_allow:
  - corpus/ttc/trees/f1/src/lib/
  - corpus/ttc/reference/
  - docs/EXCLUDED_SIX_DEMAND_MAP.md
  - docs/SWEPT_PAIR_ADMISSION_SPEC.md
tests_required:
  - loft_facts_match_derived_expectation
  - halo_loft_chain_closes_with_seam_certificate
  - mirror_produces_placed_carrier_rows
  - line_profile_special_cases_bit_identical
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'enum SolidSpec' truck123d/src/bd_bridge.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'loft is not a kernel-engine row' corpus/ttc/door.py"}
  - {id: A3, expect: 1, cmd: "grep -c 'sweep is not a kernel-engine row' corpus/ttc/door.py"}
  - {id: A4, expect: 1, cmd: "grep -c 'mirror is not a kernel-engine row' corpus/ttc/door.py"}
budget:      {turns: 75, ctx_tokens: 190000}
```

## Scope decisions (pre-decided)

1. **Loft arm.** The drop-in records k section profiles + the cross
   interpolation; `SolidSpec::Loft { sections }`. Facts: volume via the
   divergence-form boundary integral over the loft's faces — the landed
   frustum telescoping is the degenerate two-section line-profile case and
   MUST stay bit-identical for it. Derive the segment-moment generalization
   (FH-SPLINE-LATHE's derivation is the revolve precedent).
2. **Sweep-as-loft-chain (halo).** The drop-in's sweep handler records
   DISCRETE stations + per-station frames as a CHAIN of loft rows
   (`mono_halo.py::_stations`/`_frame` structure is already the right
   shape). Per-segment: loft between transformed sections. The bridge
   certifies LOOP CLOSURE with the exact seam identity on the aligned
   meeting edges (`A₀W₁ − A₁W₀ ≡ 0`); a non-closing chain refuses typed
   with the mismatch evidence.
3. **Mirror arm.** `mirror_y` records a placed-carrier transform over the
   mirrored census rows (the landed `Placed`/processor rule) — facts
   transform with the placement; no geometry recomputation.
4. **make_face extension.** Spline-trimmed planar faces record the plane
   surface + the spline boundary as trim data (the face SURFACE is
   elementary; the spline lives in the trims).
5. **Refusals stay typed.** Partial-arc sweeps, non-uniform frames,
   unsupported section carriers: unchanged refusal vocabulary. The
   enclosing F1 rows may still refuse at the booleans (swept×swept) — that
   is the admission program's boundary, NOT this packet's. This packet
   closes the AUTHORING layer only.
6. **FH-CENSUS additions (2026-09-08 adjudication).** (a) The `extrude`
   handler (door.py:490) becomes a recording arm: a closed line-loop planar
   profile extruded along z is an exact prism — facts analytically exact,
   mesh deterministic (thrust_structure's typed refusal). (b) `tube()` (15
   uses in the FH corpus, e.g. feed/gas_generator/turbine_exhaust) currently
   BYPASSES the census vocabulary and dies UNTYPED — it must answer
   name-for-name: a spline-path tube records as the sweep-as-loft-chain
   form (scope 2) where the path is piecewise-linear with recorded frames,
   and refuses TYPED (`unsupported_envelope`) otherwise. An untyped door
   failure on any census verb is a defect this packet closes.

## Done when

```
cargo test -p truck123d --lib
cargo test -p truck123d --test ttc_authoring_arms
```

plus end-to-end door evidence in RESULT.json: per model, the first
authoring carriers answer and the row proceeds past authoring (rows may
then refuse at the booleans — record WHERE, verbatim).

## Forbidden

Flattening splines to polygons. Widening the boolean boundary (admission is
ADM's program). New base Refusal variants. Editing the line-profile revolve
arm's landed behavior.

## Stop conditions

- A corpus construction in the five models does not reduce to
  loft/section/mirror census rows under the recording scheme → SPEC_GAP
  naming the construction (do not approximate).
- The loft facts derivation cannot be machine-checked on a synthetic
  fixture → SPEC_GAP with the failing identity.

## Finish by writing RESULT.json at the WORKTREE ROOT (then COMMIT first)

Commit subject: `feat(bridge): authoring arms — loft, sweep-as-loft-chain with closure certificate, mirror, spline-trimmed faces (F1-AUTHORING-ARMS)`.
