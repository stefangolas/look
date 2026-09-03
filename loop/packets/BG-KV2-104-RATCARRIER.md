# BG-KV2-104-RATCARRIER — rational half-angle carriers: Plane/Sphere/Cylinder as CertifiedPatch

Wave-1 implementation packet (build spec §4). Lands the v2 spec §3.2
carrier requirement for the first three carriers: rational (half-angle)
parameterizations with interval enclosures — **N4 by construction: no
transcendental function may appear anywhere in this module** (that is the
point of the rational reparameterization; the landed `EnclosureSurface`
impls use interval sin/cos from `elementary.rs` and are the audit's
quarantine population, NOT this module's template).

Implements `CertifiedPatch` (shim trait) for the shim's `RationalCarrier`.
No C2/C3 here (second/third jets are the contact/fillet path, Wave 3+).

**H-1.** New module `rational.rs` carries the crate's
`#![deny(clippy::unwrap_used)]` discipline (crate-level deny covers it): no
`unwrap`/`expect`/`panic!`, no module-level `allow`. Copy the header style
from `hull.rs`.

```yaml
id:          BG-KV2-104-RATCARRIER
contract:    [BG-KV2-104-RATCARRIER]
class:       design
crates:      [truck-certified]
depends_on:  [BG-KV2-000-CONTRACT]
write_allow:
  - vendor/truck/truck-certified/src/kernel/rational.rs
  - vendor/truck/truck-certified/src/kernel/mod.rs
  - vendor/truck/truck-certified/tests/kernel_rational.rs
read_allow:
  - docs/CONSTRUCTIVE_GEOMETRY_KERNEL_SPEC_V2.md
  - docs/KERNEL_V2_BUILD_SPEC.md
  - vendor/truck/truck-certified/src/kernel
  - vendor/truck/truck-certified/src/formal/exact.rs
budget:      {turns: 30, ctx_tokens: 100000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub enum RationalCarrierKind' vendor/truck/truck-certified/src/kernel/leaf.rs"}
  # 3 = CertifiedPatch + prefix-matching CertifiedPatchC2/C3 subtrait declarations (measured post-shim, deliberate)
  - {id: A2, expect: 3, cmd: "grep -c 'pub trait CertifiedPatch' vendor/truck/truck-certified/src/kernel/patch.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'pub mod kernel;' vendor/truck/truck-certified/src/lib.rs"}
  - {id: A4, expect: 0, cmd: "grep -c 'rational.rs' vendor/truck/truck-certified/src/kernel/mod.rs"}
  - {id: A5, expect: 1, cmd: "grep -c 'pub struct CertifiedInterval' vendor/truck/truck-certified/src/formal/exact.rs"}
tests_required:
  - sphere_rational_param_matches_implicit_form_on_grid
  - cylinder_rational_param_matches_implicit_form_on_grid
  - enclosures_contain_sampled_points_all_three_carriers
  - regularity_proven_away_from_the_rational_degeneration
  - weight_bound_proven_denominator_separated_from_zero
  - no_transcendental_call_in_rational_module
  - cone_variant_refuses_pending_its_packet
```

## Section 1 — parameterizations (N4-safe, all arithmetic rational)

Homogeneous evaluation per N5: carry (P, w) and never divide inside an
enclosure until `weight_bound` certifies w > 0 on the box (then divide
once, at the end).

- **Plane** — polynomial (rational of degree 0 in the denominator):
  `X(u,v) = origin + u·u_dir + v·v_dir`. `weight_bound` → `None` is
  FORBIDDEN by the shim trait contract? No — the shim trait says
  `Some(_)` only for rational carriers per §3.1's doc comment ("§7.1 makes
  this a precondition, not an option") — for the plane the denominator is
  the constant 1: return `Some(Proven(1))` with a doc note, keeping the
  §7.1 value-argument plumbing uniform. SPELLING FROZEN: plane returns
  `Some(ClaimVerdict::Proven(CertifiedPositive of 1.0))`.
- **Sphere** — rational quarter/half-angle form over a chart box
  (u, v ∈ R² lifted, no poles on the chart): the standard rational sphere
  parameterization x = 2u/(1+u²+v²), y = 2v/(1+u²+v²), z = (1−u²−v²)/(1+u²
  +v²) (radius 1, then scaled/translated by the carrier data) —
  denominator enclosure: 1 + u² + v² > 0 ALWAYS, certified by the interval
  form (lower bound ≥ 1 > 0; record the exact interval arithmetic). The
  charts of the sphere are the stereographic atlas pieces; the carrier's
  `domain: IBox2` names the chart box. Chart switching across the
  point-at-infinity is §3.4's business (later wave) — this module refuses
  boxes reaching the degeneration point with
  `RefusalKind::CarrierSingularity` (Disproven).
- **Cylinder** — rational in the angular direction: over a lifted angular
  chart, tan-half-angle substitution gives rational X(u, v) with
  denominator 1+u² > 0 (same discipline); height linear. The seam/wrap is
  a deck translation (shim `Param.deck`), not an event — the module's
  domain boxes are per-chart; wrapping is the consumer's lift.
- **Cone** — the shim carries the `Cone` kind; its rational half-angle
  parameterization is THIS PACKET'S REFUSAL POINT: implement the type
  plumbing but `CertifiedPatch for RationalCarrier` refuses the Cone and
  Torus variants with `RefusalKind::CarrierSingularity`-adjacent named
  evidence `RefusalEvidence::Predicate { name: "cone_torus_carrier_packet_pending" }`
  (the cone's apex straddling and the torus's rational form are Wave-4
  work; the `cone_variant_refuses_pending_its_packet` test pins the
  refusal so nothing silently half-implements them).

## Section 2 — `CertifiedPatch for RationalCarrier`

- `enclose` — interval evaluation of the rational forms via
  `CertifiedInterval` (outward-rounded), homogeneous until the certified
  denominator split; refuses `WeightDegenerate` (Disproven) if a
  denominator enclosure ever contains 0 (sphere/cylinder denominators
  cannot by construction — the check is still made, N6 discipline).
- `derivs` — interval derivatives of the rational forms (differentiate the
  closed form ONCE by hand in the implementation; no autodiff, no
  transcendental).
- `normal_cone` — cone over the cross-product enclosure of the derivative
  enclosure (local constructor from the shim `Cone`, the
  `BG-KV2-102` discipline).
- `regularity` — EG − F² enclosure from `derivs`; Proven iff lower > 0
  (the rational sphere's EG − F² is a positive rational function on the
  chart — the enclosure must prove it away from the degeneration point);
  Disproven(Inconclusive per §3.4 routing) at the degeneration.
- `weight_bound` — the denominator enclosure per carrier (see §1): Proven
  with the lower bound (uniform `Some` plumbing).

## Section 3 — tests

The seven `tests_required` names. Machine-checked ground truths:
1. Sphere: sampled (u,v) grid points satisfy x²+y²+z² = r² within
   1e-12 (named constant, `// H-3` same-line where needed) AND the
   implicit-form agreement — the rational parameterization reproduces the
   sphere POINTWISE (exact identity checks on the closed form, not
   tolerances, where the algebra is exact).
2. Cylinder: same for x²+y² = r² on the chart.
3. Every sampled point ∈ `enclose(box)` for all three carriers.
4. Regularity Proven on the full chart box of a sphere chart that avoids
   the degeneration; the degeneration-point box refuses.
5. `weight_bound` = Proven with a positive lower bound ≥ 1 on every chart
   box (the denominators are bounded below by construction; assert the
   actual numeric lower bound is ≥ 1 − 0 for the sphere form at u=v=0 and
   stays > 0 on the test boxes).
6. Source scan: no `sin|cos|atan2|exp|ln|log|powf|sqrt` outside comments
   in `rational.rs`.
7. Cone/Torus variant → the named pending refusal.

## Done-when

- `cargo check -p truck-certified --all-targets` green (CARGO_BUILD_JOBS=2-4).
- `cargo test -p truck-certified --lib --tests --no-fail-fast` green.
- fmt clean; clippy (exact verify form, unfiltered, ALL findings) clean on
  the packet's files.
- `cargo check --workspace --all-targets` green.

## Stop conditions

1. The shim's `RationalCarrier`/`CertifiedPatch` shapes differ from the
   quoted contract — stop, record the diff.
2. An enclosure for a needed derivative is not computable without a
   transcendental call — stop; that carrier's form is wrong for N4 and
   the spec's §3.2 must be consulted, not bypassed.
3. The rational sphere form's regularity cannot be certified on a
   legitimate chart box — record the enclosure numbers; do not loosen the
   comparison to force Proven.

## Finish by writing `RESULT.json` AT THE WORKTREE ROOT

Commit on the current branch (subject: `feat(certified): rational
half-angle carriers implement CertifiedPatch (BG-KV2-104-RATCARRIER)`)
BEFORE writing `RESULT.json`.
