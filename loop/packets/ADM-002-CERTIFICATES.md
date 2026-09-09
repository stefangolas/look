# WORK PACKET ADM-002-CERTIFICATES — assemble the regularity/transversality certificates into the admission path (r2: the lemma wave's integration)

r2 REWRITE (spine restructure): the certificate lemmas (normal-cone L3,
deflation/seam L4) are landed and machine-tested. This packet ASSEMBLES
them into the four-outcome admission dispatch — regular | regular+seam |
regular-interior+collapsed-boundary | genuine singularity → typed refusal —
and feeds the normal-cone subsystem to FSSI-001's transversality gate.

```yaml
id:          ADM-002-CERTIFICATES
contract:    [ADM-002-CERTIFICATES]
class:       design
crates:      [truck-certified]
depends_on:  [ADM-SHIM, ADM-L1-EXTRACT, ADM-L3-NORMALCONE, ADM-L4-DEFLATE-SEAM]
write_allow:
  - vendor/truck/truck-certified/src/construct/admission.rs
  - vendor/truck/truck-certified/tests/admission_certificates.rs
read_allow:
  - docs/SWEPT_PAIR_ADMISSION_SPEC.md
  - vendor/truck/truck-certified/src/construct/
tests_required:
  - four_outcome_dispatch_exhaustive_on_fixtures
  - certificates_compose_with_fssi001_gate
  - subdivision_stall_refuses_typed_not_silent
anchors:
  - {id: A1, expect: 15, cmd: "grep -c 'TensorBernsteinPatch' vendor/truck/truck-certified/src/construct/patches.rs"}
  - {id: A2, expect: 3, cmd: "grep -c 'extract_patches' vendor/truck/truck-certified/src/construct/extract.rs"}
  - {id: A3, expect: 3, cmd: "grep -c 'certified_reciprocal_power' vendor/truck/truck-evidence/src/num/reciprocal.rs"}
budget:      {turns: 60, ctx_tokens: 170000}
```

## Scope decisions

1. **Assembly only.** The cone (L3), deflation, and seam (L4) kernels are
   landed — this packet composes them into the four-outcome dispatch per
   extracted patch: hemisphere pass ⇒ regular; collapse detected ⇒ deflate
   (L4) then re-certify the interior; seam identity ⇒ paired edges; else ⇒
   typed refusal with the recorded evidence. The dispatch is EXHAUSTIVE on
   the fixture kit — a fifth outcome is a SPEC_GAP.
2. **FSSI-001 substrate handoff**: the normal-cone data (L3) feeds the
   gate's transversality test — verify the composition on a fixture (the
   gate consumes cones; this packet supplies them — no gate edits, that is
   FSSI-001's landed code).
3. **Subdivision-stall discipline**: certificate failures subdivide under
   budget; budget exhaustion ⇒ typed refusal with the stall record. Never
   silent.

H-1/H-3/H-6, SFC, single-interval-algebra rule: inherited.

## Done when

```
cargo fmt --check -p truck-certified
cargo clippy -p truck-certified --all-targets -- -D warnings
cargo test -p truck-certified --lib --tests
```

with all three named tests green and the anchors holding.

## Forbidden

Editing the lemma kernels or the shim's landed types. A "tangent sections"
taxonomy. Sampled normals. Weakening landed tests.

## Stop conditions

- A fixture produces a fifth outcome → the four-outcome space is
  incomplete → SPEC_GAP (this falsifies the theory doc's outcome claim —
  highest-priority adjudication).

## Finish by writing RESULT.json at the WORKTREE ROOT (then COMMIT first)

Commit subject: `feat(certified): certificate assembly — the four-outcome dispatch over the proven lemmas (ADM-002)`.
