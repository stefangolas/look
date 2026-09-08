# TOR-A theory statement — toric plane sections: reducibility, exact circles, and the runtime-factorization certificate

**Status:** draft for frontier review (2026-09-08, orchestrator). This is
the mathematical statement the TOR-A packet will implement. The review
ask is at §6: the numbered questions are what we need adjudicated. The
implementation never TRUSTS the classification — it factors the section
quartic exactly, per instance, and the factorization is the certificate.
The theorem's role is to promise the case census is complete.

## 1. Setup and canonical reduction

Ring torus, major radius `R`, minor radius `r`, `R > r > 0`, axis = z-axis,
center = origin (WLOG: the corpus placements are exact rigid isometries;
every predicate below transforms covariantly, so instances are handled in
canonical position by exact conjugation). Implicit form, degree 4:

```
T(x, y, z) = (x² + y² + z² + R² − r²)² − 4R²(x² + y²) = 0
```

A cutting plane `Π` reduces, by exact rotation about the z-axis (a symmetry
of `T`), to normal `n = (sin α, 0, cos α)` with offset `d = n·x₀`,
`0 ≤ α ≤ π/2` (the `z → −z` symmetry halves the range). The section type
depends only on the two exact invariants `(α, d)`.

**Lemma 1 (orthonormal plane coordinates kill the cross terms).** With
in-plane orthonormal frame `e₁ = (cos α, 0, −sin α)`, `e₂ = (0, 1, 0)`, a
plane point is `x = d·n + u·e₁ + v·e₂`. Then
`‖x‖² = d² + u² + v²` (the `du` cross terms cancel exactly by
orthonormality), while `x² + y² = (u cos α + d sin α)² + v²`. Substituting
into `T`:

```
Q(u, v) = (u² + v² + σ²)² − 4R²( v² + (u cos α + d sin α)² ) = 0,
σ² = d² + R² − r².
```

This is the classical **spiric section of Perseus** (plane-parallel torus
sections are its sub-family): after completing the square in `u` (for
`cos α ≠ 0`) it takes the spiric form `(u′² + v² + a)² = b·u′² + c·v² + e`.
Q is a quartic with EXACT rational coefficients whenever `R, r, d, cos α,
sin α` are exact (dyadic/rational corpus data; `sin α, cos α` come from the
exact normal's components).

## 2. The reducibility classification (statement for review)

`Q` factors over the reals into a product of two conics exactly in the
following cases, and each conic factor is a CIRCLE in the `(u,v)`
coordinates:

| Case | Exact predicate | Section |
|---|---|---|
| C1 | `α = π/2`, `d = 0` (plane through the axis) | two profile circles `(u ∓ R)² + v² = r²` — verified by direct substitution: `Q = ((u² + v² + R² − r²)² − 4R²u²)` factors as `((u−R)² + v² − r²)·((u+R)² + v² − r²)` |
| C2 | `0 ≤ α < π/2`, bitangent-oblique: `d² = (R² − r²)·f(α)` — the precise `f(α)` is review question Q2 | two **Villarceau circles**; classical corollary (flagged, not yet verified by us): each has radius `R` |
| C3 | tangent-plane limits (`d² = R² − r²` with `α = 0`, i.e. `σ² = 0`... and the tube-tangent planes) | degenerate conics (point / doubled circle) — typed degenerate records |
| C4 | `|d| > R + r` or the section is empty by the exact exclusion predicate | no real section |

Otherwise `Q` is IRREDUCIBLE over the rationals (a genuine spiric quartic —
Cassini-oval-class curves among them) and the locus is TRACED, never
approximated.

**The claim needing review is completeness**: that C1–C3 are the ONLY
reducible-with-circle-factors cases, i.e. there is no further
`(α, d)` family where `Q` splits into two conics that are circles. (Q may
factor into two general conics in additional cases — that is harmless: the
locus emission requires CIRCLE factors specifically; non-circle conic
factors of a toric section would be a surprise and is review question Q3.)

## 3. The runtime-factorization certificate (why no theorem needs trusting)

Per instance, the implementation:

1. Forms `Q(u, v)` with exact rational coefficients (Lemma 1).
2. Factors `Q` over ℚ exactly (polynomial factorization in the landed
   `Expansion`/exact-arithmetic culture; the factorization may require a
   certified quadratic extension `ℚ(√k)` — the extension degree and `k`
   are recorded on the certificate).
3. Tests each factor's quadratic part for the circle condition: equal `u²`
   and `v²` coefficients, zero `uv` coefficient (in the orthonormal plane
   frame every circle has quadratic part `λ(u² + v²)`).
4. Recovers each circle's center and radius exactly (completing squares;
   radii over the certified extension), and emits
   `AnalyticIntersection` circle loci — the landed vocabulary the splitter
   already consumes.
5. If `Q` is irreducible (or factors into non-circle conics), the section
   is traced by the continuation machinery — the certificate records the
   factorization result as the reason.

The factorization IS the proof per instance: an emitted circle locus is
backed by an exact identity `Q = C₁·C₂` in the polynomial ring, not by a
believed theorem. The classification theorem's role is ONLY to promise the
case census is complete (nothing falls outside {circle-emitting,
trace-quartic}).

## 4. Sanity anchors (the implementation confirms these)

- C1's explicit factorization (stated above) is machine-checked at test
  time.
- The untrimmed torus volume `2π²Rr²` anchors the facts machinery.
- The Villarceau radius corollary (`radius = R`, classical) — the runtime
  factorization will confirm or refute this for the first time in-repo; if
  the recovered radius differs, the factorization is the evidence and the
  corollary is corrected. (The doc deliberately does not assert it.)

## 5. Scope notes

- Torus×quadric tools (cylinder/sphere/cone cuts, as opposed to planes):
  general position gives degree ≤ 8 algebraic curves — no closed-form
  claim is made; those route through the traced continuation, with the
  coaxial/aligned special positions as exact BG-ANA predicates. The
  (4p,4q) implicit-reduction extension stays a MEASURED decision, not a
  commitment.
- Only the regular ring torus certifies; horn/spindle refuse typed.
- Placement isometry: all predicates above are stated canonically and
  lifted by the exact rigid placement (coefficients transform, the
  factorization structure is invariant).

## 6. Review questions for the frontier agent

- **Q1 (completeness — the load-bearing question).** Is the case census
  C1–C4 + irreducible exhaustive? I.e., is it a theorem that the ONLY
  `(α, d)` configurations where the toric section quartic `Q(u, v)` of §1
  factors into two conics-with-circle-quadratic-parts are C1 (axis plane)
  and C2 (bitangent oblique, `d² = (R² − r²)·f(α)` — derive `f`), plus the
  C3 degenerate limits? A reference or a proof sketch suffices; the
  implementation does not depend on which form the answer takes, but the
  packet's stop conditions depend on the census being complete.
- **Q2 (the bitangent predicate).** Derive the exact bitangent condition
  as a function of `(α, d)` — our probe suggests it involves
  `d² = (R² − r²)`-type expressions modulated by the inclination `α`; the
  packet needs the closed-form predicate to pre-dispatch the factorization
  (though the factorization itself re-decides it per instance).
- **Q3 (non-circle conic factors).** Can `Q` factor into two conics that
  are NOT circles (ellipses/hyperbola pairs) for a torus section? Our
  belief: no (the spiric quartic's invariants forbid it), but this is
  exactly the kind of belief the review should test.
- **Q4 (the Villarceau radius).** Confirm or refute: the bitangent-oblique
  section's two circles each have radius exactly `R`.
- **Q5 (the quadratic extension).** The circle factors may require
  `ℚ(√k)` coefficients (the centers/radii involve `√(R² − r²)`-type
  quantities). Confirm the factorization-over-extension is unavoidable
  (i.e., `Q` can be ℚ-irreducible yet real-factor into circles), so the
  certificate records the extension rather than pretending ℚ-factoring
  suffices.
- **Q6 (edge cases).** Tangent planes (single double-touch), planes
  meeting the torus in isolated points, and the horn/spindle boundary
  `R = r`: confirm each lands in C3/C4-typed handling with no additional
  real-locus cases hiding at the boundary.
