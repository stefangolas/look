# WORK PACKET FH-SPLINE-LATHE — admit spline-profile revolves in the kernel executor's lathe arm

The two Falcon-Heavy timing rows (nozzle_assembly, mvac) refuse typed on
their FIRST carrier: `bd.Edge.make_spline(...)` contours revolved 360°.
Refusal site: `corpus/ttc/door.py` revolve arm — `if edge.kind != "line":
_refuse("a spline-profile revolve is not a kernel-engine row")`. The line
profile arm (`SolidSpec::Lathe`, frustum telescoping) is landed and proven
(turbopump end-to-end, volume ~3e-15 vs OCC). This packet extends the lathe
to spline profiles so both rows pass the facts gate — the honest way: the
kernel integrates the TRUE spline, never a sample-polygon flattening (the
polygon differs from the interpolated spline between samples; the OCC
reference revolved the true spline, so flattening would fail the facts gate
by design).

```yaml
id:          FH-SPLINE-LATHE
contract:    [FH-SPLINE-LATHE]
class:       design
crates:      [truck123d]
depends_on:  [TTC-EXECUTOR-BINDING]
write_allow:
  - corpus/ttc/door.py
  - truck123d/src/bd_bridge.rs
  - truck123d/tests/ttc_lathe_spline.rs
read_allow:
  - corpus/ttc/reference/nozzle_assembly.json
  - corpus/ttc/reference/mvac.json
  - docs/TT_TIMING_RESULTS.md
  - vendor/truck/truck-certified/src/patch_admit.rs
  - vendor/truck/truck-certified/src/ssi_admit.rs
tests_required:
  - spline lathe facts match the recorded OCC references within tolerance
  - refusal cases stay typed (partial arc, non-y=0 plane, open profile)
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'a spline-profile revolve is not a kernel-engine row' corpus/ttc/door.py"}
  - {id: A2, expect: 1, cmd: "grep -c 'fn lathe_volume' truck123d/src/bd_bridge.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'revolved_shell' corpus/ttc/trees/falcon_heavy/src/lib/merlin_common.py"}
budget:      {turns: 70, ctx_tokens: 190000}
```

## Scope decisions

1. **Where the arm lives.** The drop-in (`door.py` revolve) classifies the
   profile; the kernel computes. A spline edge must carry its DEFINING data
   to the kernel — derive from build123d's `make_spline` what the Edge
   object exposes (poles, or sample points + interpolation convention) and
   record that in the census row. If the interpolation convention is not
   recoverable from the recorded data, STOP: SPEC_GAP naming exactly what
   is missing. Never guess an interpolation.
2. **Facts math is the design center.** Volume of the solid of revolution
   over a spline profile: per spline segment, `r` and `z` are polynomials
   in the segment parameter; `∫ π r(z)² dz` over the segment reduces to
   exact polynomial moment arithmetic — derive it, machine-check the
   derivation against the landed interval/Bernstein hull kernels
   (`truck-certified` patch_admit/ssi_admit culture; the frustum
   telescoping in `lathe_volume` is the line-profile special case and must
   remain bit-identical for line profiles). bbox from the landed spline
   hull. Deterministic STL: evaluate the spline at fixed segment counts.
3. **No silent flattening.** The arm integrates the spline the corpus
   authored. A test must fail if a sample-polygon volume is substituted
   (assert the facts differ from the polygon approximation by more than
   the facts tolerance on a curved fixture — the deviation is the test).
4. **Refusals stay typed.** Partial arc, non-y=0 profile plane, open
   profile, non-z rotation: unchanged refusal paths, unchanged messages.
5. **Facts gate evidence.** The in-repo tests assert the SolidSpec facts
   against analytically derived expectations for a synthetic spline
   contour; the END-TO-END proof (door `--engine truck` on
   `make_nozzle_assembly` and `make_mvac` matching
   `corpus/ttc/reference/*.json` within tolerance) is executed and
   recorded in RESULT.json with the verbatim facts — the binding's pilot
   precedent (release cdylib staged beside the interpreter runtimes).

## Done when

```
cargo test -p truck123d --lib
cargo test -p truck123d --test ttc_lathe_spline
```

plus the end-to-end door evidence recorded in RESULT.json (both rows green
vs the recorded references).

## Forbidden

Flattening splines to sample polygons. Widening the census vocabulary
beyond the lathe arm. Touching the line-profile arm's landed behavior
(V5-pair: line-profile facts bit-identical). New base Refusal variants.

## Stop conditions

- build123d's spline interpolation convention is not recoverable from the
  data the drop-in can record → SPEC_GAP naming the exact gap.
- The segment-moment volume derivation cannot be machine-checked against
  the landed hull kernels → SPEC_GAP with the failing identity.
- An OCC reference fact for either row is unreachable (reference file
  lacks volume/bbox/triangle count) → record which field and stop.

## Finish by writing RESULT.json at the WORKTREE ROOT (then COMMIT first)

Commit subject: `feat(bridge): spline-profile lathe arm — exact segment-moment facts, typed refusals preserved (FH-SPLINE-LATHE)`.
