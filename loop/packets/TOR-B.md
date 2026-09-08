# WORK PACKET TOR-B — the ray-torus quartic: seed-ray crossings for torus-bearing solids in classify

Design packet (theory: [`TORUS_CONTACT_THEORY.md`](../../docs/TORUS_CONTACT_THEORY.md)
v2 §2, frontier-reviewed). The classify stage's ray-crossing machinery
gains the torus arm: the ray-torus intersection is a FIXED-DEGREE-4
polynomial in the ray parameter — the landed three-state root isolation
applies unchanged, and the parity/winding and tangency predicates are
algebraic. This unlocks torus-bearing solids in the boolean classify stage
(the `require_canonical_carriers` gate widens by dispatch-table extension
only — FSSI-EXT).

```yaml
id:          TOR-B
contract:    [TOR-B]
class:       design
crates:      [truck-shapeops, truck-certified]
depends_on:  []
write_allow:
  - vendor/truck/truck-shapeops/src/boolean/classify.rs
  - vendor/truck/truck-shapeops/tests/torus_ray_crossings.rs
read_allow:
  - docs/TORUS_CONTACT_THEORY.md
  - docs/TORUS_CONTACT_PROGRAM.md
  - vendor/truck/truck-certified/src/formal/torus.rs
  - vendor/truck/truck-evidence/src/num/roots.rs
tests_required:
  - ray_quartic_coefficients_exact_against_reference
  - parity_classifier_inside_outside_correct
  - tangent_ray_double_root_typed_not_crossing
  - flank_sign_certified_on_higher_contact_fixture
anchors:
  - {id: A1, expect: 4, cmd: "grep -c 'surface_ray_crossings' vendor/truck/truck-shapeops/src/boolean/classify.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub fn' vendor/truck/truck-evidence/src/num/roots.rs"}
  - {id: A3, expect: 19, cmd: "grep -c 'pub fn' vendor/truck/truck-certified/src/formal/torus.rs"}
budget:      {turns: 65, ctx_tokens: 170000}
```

## Scope decisions (pre-decided per theory v2 §2)

1. **The quartic (6).** Ray `q(t) = q₀ + t·v` into the scale-invariant
   T-form: `g₄ = K‖v‖⁴ > 0` (always a genuine quartic), coefficients exact
   rational/algebraic operations on the carrier + ray data. The landed
   `num/roots.rs` discipline (every returned interval contains exactly one
   root; multiple roots refuse `NumericallyUnresolved`) applies unchanged.
2. **Parity classifier (7).** `q₀ ∈ int(T) ⟺ #{positive roots of odd
   multiplicity} ≡ 1 (mod 2)` — the sign argument (`g → +∞`; `g(0) < 0`
   iff inside), stated and TESTED (N4's instantiation obligation).
3. **Oriented BRep statement (8).** `χ_S(q₀) = Σ sgn(Nᵢ·v)` over
   positive-`t` transverse crossings with outward face normals — entering
   −1, exiting +1; mod 2 the ordinary crossing rule. No new classifier
   argument: TOR-B satisfies the EXISTING ray-event contract (sound +
   complete + exact crossing bit + seam deduplication) — FSSI-EXT is
   dispatch-only.
4. **Tangency is algebraic (9).** `∇Φ = 4KSq − 8R²(Kq − (q·a)a)`; the exact
   tangency predicate at an isolated root: `g(t*) = 0 ∧ g'(t*) = 0`. No
   numerical normal anywhere.
5. **Higher contact: flank signs.** "Tangent = noncrossing" is NOT
   assumed — compare the certified signs immediately on the two sides of
   the isolated root: different signs ⇒ odd-multiplicity crossing; equal
   ⇒ even contact (typed record on the crossing bit).
6. **The gate extension.** The `require_canonical_carriers` gate widens to
   torus-bearing solids — dispatch-table extension only; horn/spindle
   refuse typed (the landed `formal/torus.rs` rule); no other carrier
   semantics change.

H-1: `#![deny(clippy::unwrap_used)]` on new code paths. H-3: constants
named. H-6: floats never in evidence. SFC: float roots supply candidates;
exact isolation and flank signs certify.

## Done when

```
cargo fmt --check -p truck-shapeops -p truck-certified
cargo clippy -p truck-shapeops -p truck-certified --all-targets -- -D warnings
cargo test -p truck-shapeops --lib --tests
cargo test -p truck-certified --lib
```

with all four named tests green and the anchors holding.

## Forbidden

Numerical normals in any predicate. Weakening the root-isolation contract
(every interval exactly one root). Widening any verdict class. Editing the
Krawczyk operator, the F3 rule, or `split.rs` (FSSI-LAYER). Certifying
horn/spindle tori.

## Stop conditions

- The quartic's coefficients cannot be made exact on a corpus-derived ray
  (the exactified-ray path fails) → SPEC_GAP naming the step (SFC's
  exactification is the landed discipline; a gap here is a substrate bug).
- A tangency fixture the flank-sign method cannot classify → SPEC_GAP with
  the fixture (do not assume multiplicity semantics).

## Finish by writing RESULT.json at the WORKTREE ROOT (then COMMIT first)

Commit subject: `feat(shapeops): ray-torus quartic crossings — algebraic tangency, parity classifier, gate extension (TOR-B)`.
