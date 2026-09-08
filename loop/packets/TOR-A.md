# WORK PACKET TOR-A — torus×plane exact circle loci: the Villarceau factorization certificate

Design packet (theory: [`TORUS_CONTACT_THEORY.md`](../../docs/TORUS_CONTACT_THEORY.md)
v2, frontier-reviewed; the section statement:
[`TORUS_SECTION_THEORY.md`](../../docs/TORUS_SECTION_THEORY.md)). Emits
EXACT analytic circle loci for torus×plane sections via the
runtime-factorization certificate: the plane-section quartic `Q(u,v)` is
factored exactly per instance; circle factors emit
`AnalyticIntersection` circle loci (the landed transverse path), irreducible
`Q` traces. The completeness theorem (N1) is PROVEN in the theory doc — the
implementation never trusts it, only the factorization.

```yaml
id:          TOR-A
contract:    [TOR-A]
class:       design
crates:      [truck-evidence, truck-certified]
depends_on:  []
write_allow:
  - vendor/truck/truck-evidence/src/analytic/torus_section.rs
  - vendor/truck/truck-evidence/src/analytic/mod.rs
  - vendor/truck/truck-certified/src/pair_dispatch.rs
  - vendor/truck/truck-evidence/tests/torus_section_conformance.rs
read_allow:
  - docs/TORUS_CONTACT_THEORY.md
  - docs/TORUS_SECTION_THEORY.md
  - docs/TORUS_CONTACT_PROGRAM.md
  - vendor/truck/truck-evidence/src/
  - vendor/truck/truck-certified/src/formal/torus.rs
tests_required:
  - axial_plane_emits_two_profile_circles
  - axis_perpendicular_plane_emits_coaxial_circles
  - villarceau_bitangent_plane_emits_two_circles_radius_R
  - general_plane_quartic_traces_not_approximated
  - empty_and_tangent_planes_typed
anchors:
  - {id: A1, expect: 6, cmd: "grep -c 'pub fn' vendor/truck/truck-certified/src/pair_dispatch.rs"}
  - {id: A2, expect: 15, cmd: "grep -c 'pub' vendor/truck/truck-evidence/src/analytic/mod.rs"}
  - {id: A3, expect: 19, cmd: "grep -c 'pub fn' vendor/truck/truck-certified/src/formal/torus.rs"}
budget:      {turns: 80, ctx_tokens: 200000}
```

## Scope decisions (pre-decided per theory v2 §1)

1. **The certificate.** New `analytic/torus_section.rs`: given the torus
   `(R, r, placement)` and the plane `(n, d)` in exact carrier data, form
   the section quartic `Q(u, v)` in orthonormal plane coordinates (Lemma 1:
   the cross terms cancel — `‖x‖² = d² + u² + v²`), factor `Q` exactly
   (over ℚ or a certified quadratic extension `ℚ(√k)`, recorded on the
   certificate), test each factor's quadratic part for the circle
   condition (equal `u²`/`v²` coefficients, zero `uv`), recover
   center/radius exactly, and emit circle loci. Irreducible ⇒ the traced
   path. The emitted identity `Q = C₁·C₂` IS the proof per instance.
2. **The scale-invariant substrate.** The torus implicit in the T-form
   (`K = a·a`, no axis normalization — theory v2 §0): placement axes need
   not be unit; the polynomial is sign-invariant under axis scaling.
3. **The classification predicates** (theory v2 §1 table) PRE-DISPATCH the
   factorization: axis-perpendicular (`H = 0`), axial (`d = 0, s = 0`),
   Villarceau (`d = 0, H > 0, s ≠ 0, (R²−r²)H = r²s²`), general. The
   factorization re-decides every instance — the predicates are a fast
   path, never an authority.
4. **Emission.** Axial: two profile circles (center `O ± Rw`, radius `r`).
   Axis-perpendicular: `ρ± = R ± √(r² − h²)` coaxial circles / Empty /
   double-typed. Villarceau: two circles center `O ± rw`, radius `R`,
   meeting at the bitangency points (emitted as exact branch-node events).
   All via the landed `ContactLocus::Analytic` vocabulary — NO splitter
   changes (FSSI-LAYER doctrine; a splitter edit is a stop-and-file).
5. **Aligned-degeneracy predicates** (BG-ANA class): for axis-parallel
   planes, `d² ⋛ (R∓r)²(n·n)` decides the
   two-loop/inner-tangent/one-loop/outer-tangent/empty regimes exactly —
   recorded as topology facts on the certificate.
6. **Residual quartics** route to the tracer; the (4p,4q) implicit-reduction
   extension is a MEASURED decision booked separately (never assumed).
7. **Horn/spindle refuse typed** (the landed `formal/torus.rs` rule,
   carried forward). Placement isometry: instances are exact conjugations
   of the canonical case.

## Done when

```
cargo fmt --check -p truck-evidence -p truck-certified
cargo clippy -p truck-evidence -p truck-certified --all-targets -- -D warnings
cargo test -p truck-evidence --lib --tests
cargo test -p truck-certified --lib
```

with all five named tests green (the Villarceau test asserts the classical
radius-`R` corollary FROM the factorization — if the recovered radius
differs, that is evidence, and the corollary is corrected, not the data).

## Forbidden

Float predicates in any emitted locus. Approximate circle fitting (the
factorization is exact or the locus is traced). Splitter/classifier edits.
New base Refusal variants. Horn/spindle certification.

## Stop conditions

- A plane section's `Q` factors into two NON-circle conics → this
  contradicts the review's classification — SPEC_GAP with the exact
  `(α, d)` instance (frontier review re-engaged).
- The exact factorization needs an extension beyond `ℚ(√k)` → SPEC_GAP
  naming the degree.
- The landed `torus_pairs.rs` contact decisions disagree with the new
  locus emission on any fixture → defect record, stop-and-file.

H-1: `#![deny(clippy::unwrap_used)]` on the new module. H-3: constants
named on their defining line. H-6: floats never in evidence.

## Finish by writing RESULT.json at the WORKTREE ROOT (then COMMIT first)

Commit subject: `feat(evidence): torus x plane exact circle loci — the Villarceau factorization certificate (TOR-A)`.
