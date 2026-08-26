# WORK PACKET BG-SOL-S7-GFF-WIRE - validated mixed-quadric wiring

Implement the first dispatcher consumer of the general validated FF branch
cover. If live code contradicts this packet, report it in `disagreements`.

```json
{"id":"BG-SOL-S7-GFF-WIRE","status":"DONE","contracts":["BG-SOL-S7-GFF-WIRE"],
 "tests_added":0,"sites_migrated":0,"sites_deferred":0,
 "deviations":[],"disagreements":[],"baseline_failures":[],"notes":"free text"}
```

```yaml
id:          BG-SOL-S7-GFF-WIRE
contract:    [BG-SOL-S7-GFF-WIRE]
class:       design
crates:      [truck-evidence]
write_allow:
  - vendor/truck/truck-evidence/src/contact/mod.rs
read_allow:
  - vendor/truck/truck-evidence/src/contact/gff.rs
  - vendor/truck/truck-evidence/src/contact/implicit.rs
  - vendor/truck/truck-evidence/src/enclosure.rs
  - vendor/truck/truck-evidence/src/cylinder.rs
  - vendor/truck/truck-evidence/src/cone.rs
  - vendor/truck/truck-evidence/src/sphere.rs
  - vendor/truck/truck-base/src/evidence.rs
tests_required:
  - contact_ff_offset_mixed_quadric_pairs_return_validated_cover
  - contact_ff_offset_mixed_quadric_cover_is_order_insensitive
  - contact_ff_offset_disjoint_aabbs_return_empty
  - contact_ff_offset_tangent_pair_stays_deferred_for_singular_stage
budget:      {turns: 35, ctx_tokens: 90000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub enum ContactLocus' vendor/truck/truck-evidence/src/contact/mod.rs"}
  - {id: A2, expect: 0, cmd: "grep -c 'ValidatedBranchCover' vendor/truck/truck-evidence/src/contact/mod.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'fn analytic_ff' vendor/truck/truck-evidence/src/contact/mod.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'pub struct BranchCover' vendor/truck/truck-evidence/src/contact/gff.rs"}
  - {id: A5, expect: 1, cmd: "grep -c 'pub fn cover_branch' vendor/truck/truck-evidence/src/contact/gff.rs"}
  - {id: A6, expect: 1, cmd: "grep -c 'impl EnclosureSurface for Cylinder' vendor/truck/truck-evidence/src/cylinder.rs"}
  - {id: A7, expect: 1, cmd: "grep -c 'impl EnclosureSurface for Cone' vendor/truck/truck-evidence/src/cone.rs"}
  - {id: A8, expect: 1, cmd: "grep -c 'impl EnclosureSurface for Sphere' vendor/truck/truck-evidence/src/sphere.rs"}
```

## Problem

`contact::gff::cover_branch` certifies regular crossings in a finite world
box. Wire the four non-coaxial cells it was built for: Cylinder/Cone,
Cylinder/Sphere, Cone/Cone, and Cone/Sphere, both orders. Preserve boundaries:
complete regular cover -> validated locus; empty -> empty `ContactComplex`;
singular -> deferred singular stage; unresolved -> typed numerical refusal;
Torus, Placed, and unrelated cells remain deferred.

## Decisions already made

### 1. Honest intermediate locus

Add this arm:

```rust
/// A complete regular branch cover from the validated FF engine.
/// The singular and unresolved lists are empty when this arm is built.
/// Points are certified cross-sections of one or more Arc1 components;
/// connectivity and component ordering are deliberately not claimed yet.
ValidatedBranchCover(gff::BranchCover),
```

Do not add a point-vector arm, manufacture an `ExactCurve`, or connect a
polyline. Update nearby docs: one cover record may contain multiple
not-yet-separated regular components. Event continuation later produces
`RegularContactArc`s.

### 2. Bounds and world domain

The Face/Face arm must pass both `(u_range, v_range)` pairs to `analytic_ff`;
make its budget mutable. Exact arms ignore bounds and spend nothing. Public
`contact()` does not change.

Use each concrete carrier's existing `EnclosureSurface::enclose`. Match the
enum explicitly; do not implement the trait for `CanonicalSurface`.
Parameter endpoints must be finite and ordered (`lo <= hi`). Otherwise return
`UnsupportedEnvelope(ContactReductionDeferred)`; reversed periodic bounds may
mean seam crossing and are not empty. Use `Interval::try_from`, never unwrap.

Intersect certified AABBs by `lo=max(lhs.inf,rhs.inf)` and
`hi=min(lhs.sup,rhs.sup)`. A separated axis proves empty contact. Non-finite or
empty enclosure data that does not prove separation must refuse numerically,
never become a no-contact answer. The AABB is conservative; as in exact FF,
precise face trimming/component splitting is Phase 4 Boundary Rewrite work.

### 3. Scale-relative resolution

For a finite non-degenerate intersection box use
`tau = domain.width() / 128`. Name the dimensionless divisor. A larger floor
can only cause an honest unresolved refusal because this packet accepts no
unresolved boxes. If width or tau is non-finite or not positive, return
`NumericallyUnresolved` with `KrawczykIndeterminate`.

The caller owns budget. Capture its entry value, propagate every refusal from
`cover_branch`, never create a fixed private budget, and never replenish it.

### 4. Completion rules and certificate

Apply these rules in order after `cover_branch`:

1. non-empty `singular_boxes` ->
   `UnsupportedEnvelope(ContactReductionDeferred)`;
2. non-empty `unresolved_boxes` -> `NumericallyUnresolved`, spend = entry
   budget minus remaining budget, witness `KrawczykIndeterminate`;
3. empty `points` -> certified empty `ContactComplex`;
4. otherwise -> one record: `Arc1`, `Transverse`,
   `ValidatedBranchCover(cover)`.

For 3 and 4 preserve the cover certificate (Interval method and actual
`budget_left`). For an early disjoint AABB, make the same interval-method
certificate with untouched budget, empty props, and unbounded margin/modulus.
Do not set `Prop::AnalyticCarrier` on this path.

### 5. Dispatch table

Keep exact/coaxial branches unchanged. Replace only the deferred offset result:

| ordered pair | predicate | offset result |
|---|---|---|
| Cylinder, Cone / Cone, Cylinder | axes unequal | validated FF |
| Cylinder, Sphere / Sphere, Cylinder | axes unequal | validated FF |
| Cone, Cone | apexes unequal | validated FF |
| Cone, Sphere / Sphere, Cone | axes unequal | validated FF |

Each carrier stays paired with its bounds. Cylinder/Cylinder remains
`parallel_cylinders`; coaxial mixed pairs remain `coaxial`; Torus and Placed
remain deferred.

## Tests required

Add a bounds-aware face helper beside the existing unit-box helper. The four
regular witnesses share `p = (1/2, sqrt(3)/2, 1)` and a first-carrier patch
`u in [0.8,1.3]`, `v in [0.8,1.2]`, keeping y decisively positive. Use:

- unit cylinder at the origin;
- cone A: apex `(0,0,0)`, `tan(alpha)=1`;
- cone B: apex `(1,0,0)`, `tan(alpha)=1`;
- sphere: center `(2,0,0)`, radius `2`.

Machine-checked identities at p:

```text
cylinder:                 x^2 + y^2 - 1 = 0
cone A:                   x^2 + y^2 - z^2 = 0
cone B:           (x-1)^2 + y^2 - z^2 = 0
sphere:           (x-2)^2 + y^2 + z^2 - 4 = 0
```

Use a broad second-carrier patch containing p (cone B may use `u in [0,PI]`,
`v in [0.8,1.2]`; sphere may use full polar/azimuth ranges). Its AABB
intersection with the first patch retains positive y.

1. `contact_ff_offset_mixed_quadric_pairs_return_validated_cover`: exercise
   Cylinder/Cone B, Cylinder/Sphere, Cone A/Cone B, and Cone A/Sphere with a
   healthy budget. Assert one `Arc1`/`Transverse` validated cover, non-empty
   points, and empty singular/unresolved lists for each.
2. `contact_ff_offset_mixed_quadric_cover_is_order_insensitive`: run at least
   Cylinder/Cone B in both orders with bounds kept with carriers. Compare point
   sets order-insensitively using a named unit-scale residual; discovery order
   need not match.
3. `contact_ff_offset_disjoint_aabbs_return_empty`: unit cylinder patch versus
   a full sphere centered `(10,0,0)`, radius `1`. Assert no contacts and an
   untouched budget.
4. `contact_ff_offset_tangent_pair_stays_deferred_for_singular_stage`: unit
   cylinder versus sphere center `(2,0,0)`, radius `1`, with patches enclosing
   `(1,0,0)`. Assert exactly
   `UnsupportedEnvelope(ContactReductionDeferred)`.

### Verifier repair r2

V5 correctly rejected the first worker commit because it renamed the
pre-existing regression test
`contact_ff_non_coaxial_curved_pair_refuses_deferred`. Preserve that exact test
function name while updating its assertions to the new dispatch semantics; a
passing test at the base may not disappear from the head test inventory. This
is a one-line test-identity repair only: retain the implementation and the four
new required tests unchanged.

H-3 rejects an added bare `1e-N` unless that line has a same-line `// H-3`.

## Done when

```console
cargo fmt --check -p truck-evidence
cargo clippy -p truck-evidence --all-targets --no-deps
cargo check --locked -p truck-evidence --all-targets
cargo test -p truck-evidence --lib contact --no-fail-fast
bash scripts/kernel-gates.sh <your base commit>
```

Never run bare `cargo test` or a workspace-wide cargo command.

## Forbidden

Editing outside `write_allow`; editing gff, implicit, carrier enclosures,
fe_ee, truck-base, or manifests; changing `cover_branch`; adding dependencies;
constructing polyline/exact curves from points; accepting singular/unresolved
boxes; adding Torus/Placed dispatch; changing cylinder/cylinder or coaxial
behavior; adding `#[ignore]`; loosening a gate; changing the GATE-4 ceiling.

## Stop conditions

- anchor mismatch -> `ANCHOR_MISMATCH` with observed count;
- a regular witness cannot certify under healthy budget after checking bounds
  and order -> `SPEC_GAP` with the measured cover/refusal;
- non-finite AABB data passes without typed refusal -> `SPEC_GAP`;
- three consecutive cargo failures with one cause -> `BLOCKED`.

Finish by writing `RESULT.json` in the worktree root, not `loop/results/`.
