# CC-DEF-FACET-ADMISSION — SEM-FACET-SCALE-ZERO-001 + SEM-FACET-CORRESPONDENCE-TRUNCATION-001

Defect records: `docs/defects/SEM-FACET-SCALE-ZERO-001.md`,
`docs/defects/SEM-FACET-CORRESPONDENCE-TRUNCATION-001.md` (normative). One
obligation: the same recipe means the same geometry in both realization
backends — through-zero scale and mismatched correspondences are TYPED
REFUSALS on every path, never silently folded or truncated.

- **SCALE-ZERO**: `facet_sweep` validates stations only; `ProfileLaw::
  evaluate`'s `ProfileCollapse` fires only at sampled stations, so a scale
  passing through zero BETWEEN stations folds the mesh through the spine and
  the local-winding audit still certifies it (measured: facet Ok +0.0533 vs
  BREP Err).
- **TRUNCATION**: `LinearCorrespondence`'s evaluate arm zips start/end
  vertices — Rust `zip` truncates to the shorter side — so a 4→6
  correspondence silently interpolates to the first 4 (facet Ok +0.0830 vs
  BREP Err).

```yaml
id:          CC-DEF-FACET-ADMISSION
contract:    [CC-DEF-FACET-ADMISSION]
class:       mechanical
crates:      [truck-geometry, truck-modeling]
depends_on:  []
write_allow:
  - vendor/truck/truck-geometry/src/constructive/profile.rs
  - vendor/truck/truck-geometry/src/constructive/validation.rs
  - vendor/truck/truck-geometry/src/constructive/mod.rs
  - vendor/truck/truck-modeling/src/facet_sweep.rs
  - vendor/truck/truck-modeling/src/spine_sweep.rs
  - showcases/tests/battery_construction.rs
read_allow:
  - docs/defects
  - vendor/truck/truck-geometry/src/constructive
budget:      {turns: 18, ctx_tokens: 80000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'scale_touches_zero' vendor/truck/truck-modeling/src/spine_sweep.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'through_zero_scale_facet_path_behavior' showcases/tests/battery_construction.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'correspondence_mismatch_facet_path_behavior' showcases/tests/battery_construction.rs"}
tests_required:
  - facet_path_refuses_through_zero_scale
  - facet_path_refuses_mismatched_correspondence
  - spine_sweep_refusals_unchanged_on_the_twin_fixtures
  - valid_recipes_still_realize_on_both_paths
```

Section 1: the shared validator — NEW file
`constructive/validation.rs`: `pub fn validate_scalar_law_range(law:
&ScalarLaw, domain: (f64, f64)) -> Result<(), ConstructError>` returning
`Err(ProfileCollapse { at })` with the THROUGH-ZERO detection: the signed
scale must not change sign (or touch zero) anywhere in the CLOSED domain —
interval-style endpoint+sign reasoning over the law's declared form, not
station sampling (that is the defect: sampling missed it). And `pub fn
validate_correspondence(start: usize, end: usize) -> Result<(),
ConstructError>` refusing count mismatch with `ProfileCorrespondenceMismatch`
— the check `try_linear_correspondence` enforces at construction, now also
enforced at evaluation for struct-literal laws (the defect path:
`ProfileLaw` built by struct literal bypasses `try_linear_correspondence`;
keep `try_linear_correspondence` unchanged and add the evaluate-time gate).

Section 2: both entries call the shared validator — `spine_sweep` REPLACES
its private `scale_touches_zero` (A1) with the shared fn (behavior
identical — its existing refusals must stay byte-identical on the twin
fixtures); `facet_sweep` ADDS the two validation calls at entry, refusing
`ProfileCollapse`/`ProfileCorrespondenceMismatch` exactly as the BREP path
already does. No other behavior change: the grid registry, winding audit,
and verdicts are untouched.

Section 3: the showcases twins
`through_zero_scale_facet_path_behavior` (A2) and
`correspondence_mismatch_facet_path_behavior` (A3) PIN the defect — INVERT
both facet-path assertions to expect `Err` (mirror the BREP twins' shapes).
The side-session ID-named regressions (`sem_facet_scale_zero_001_*`,
`sem_facet_correspondence_truncation_001_*`) are DESIGNED EXTERNALLY: do
not author or rename them; if present at commit, they must be green.

House rules: **H-1: no `unwrap`/`expect`/`panic!` in shipped code, no
module-level `allow`.** **H-3: float comparisons in tests take the `// H-3`
opt-out ON THE SAME LINE.** **All cargo invocations go through the queue
(the `cargo` on PATH IS the queue shim). Do not invoke cargo by absolute
path; do not unset the shim.** Scoped checks: `cargo check -p
truck-geometry`, `cargo check -p truck-modeling`, `cargo test -p
truck-modeling --lib`, `cargo test -p showcases --test
battery_construction`. The `pub mod validation;` line in
`constructive/mod.rs` is the DESIGNED one-line conflict. COMMIT BEFORE
writing RESULT.json AT THE WORKTREE ROOT.

Stop conditions: (1) the V5 identity guard applies to `spine_sweep`'s
existing refusal behavior — identical inputs must produce identical
refusals, only the code path is shared; (2) `ScalarLaw`'s variant set is
landed — the through-zero reasoning must cover every variant (Constant,
Scale, LinearCorrespondence-carried scales); a variant you cannot reason
about in closed form is a STOP-and-QUESTION, not a sampling fallback (that
is the defect); (3) facet verdicts on VALID recipes must stay
`CertifiedWithinTolerance` — if validation starts refusing valid recipes on
the corpus fixtures, your zero-reasoning is wrong: fix it, do not loosen it.
