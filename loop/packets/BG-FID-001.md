# WORK PACKET BG-FID-001 — primitive certified evidence: face scale components and wedge slope

You are implementing one item from a formal kernel specification. Everything you
need is in this document. **Do not read `docs/GENERATION_KERNEL_BUILD_SPEC.md`
or any other spec file** — they are not on your allowlist and this packet is
self-contained. If something you need is genuinely missing, that is a SPEC_GAP
(see "Stop conditions"): you stop and report, you do not research it.

```json
{"id":"BG-FID-001","status":"DONE","contracts":["BG-FID-001","BG-FID-001a"],
 "tests_added":7,"sites_migrated":0,"sites_deferred":0,
 "deviations":[],"disagreements":[],"baseline_failures":[],
 "notes":"free text"}
```

Fill `deviations`, `disagreements` and `baseline_failures` as arrays, empty if
empty. `disagreements` is the highest-value field: the mathematics below was
derived, scratch-validated and theorem-checked before this packet was written,
but that is exactly the kind of claim that can be confidently wrong. **If
anything below contradicts what you find in the code or in the mathematics as
you work it, say so in `disagreements` rather than making the code match the
packet.**

```yaml
id:          BG-FID-001
contract:    [BG-FID-001, BG-FID-001a]
class:       design
crates:      [truck-evidence]
write_allow:
  - vendor/truck/truck-evidence/src/fid/mod.rs
  - vendor/truck/truck-evidence/src/fid/lfs.rs
read_allow:
  - vendor/truck/truck-evidence/src/enclosure.rs
  - vendor/truck/truck-evidence/src/sphere.rs
  - vendor/truck/truck-evidence/src/plane.rs
  - vendor/truck/truck-evidence/src/elementary.rs
  - vendor/truck/truck-evidence/src/lib.rs
  - vendor/truck/truck-topology/src/invariants/wedge.rs
  - loop/packets/BG-FID-001-THEOREM-MAP.md
budget:      {turns: 42, ctx_tokens: 100000}
anchors:
  # Measured on integration HEAD at packet-writing time. A count mismatch is a
  # stop condition (ANCHOR_MISMATCH), not a nuisance.
  - {id: A1, expect: 1, cmd: "grep -c 'pub mod lfs' vendor/truck/truck-evidence/src/fid/mod.rs"}
  - {id: A2, expect: 2, cmd: "grep -c 'BG-FID-001' vendor/truck/truck-evidence/src/fid/lfs.rs"}
  - {id: A3, expect: 8, cmd: "grep -c 'sin_margin' vendor/truck/truck-topology/src/invariants/wedge.rs"}
  - {id: A4, expect: 2, cmd: "grep -c 'fn immersion_lower_bound' vendor/truck/truck-evidence/src/enclosure.rs"}
  - {id: A5, expect: 2, cmd: "grep -c 'fn enclose_der' vendor/truck/truck-evidence/src/enclosure.rs"}
  - {id: A6, expect: 1, cmd: "grep -c 'impl EnclosureSurface for Sphere' vendor/truck/truck-evidence/src/sphere.rs"}
  - {id: A7, expect: 1, cmd: "grep -c 'pub struct Box3' vendor/truck/truck-evidence/src/enclosure.rs"}
  - {id: A8, expect: 1, cmd: "grep -c 'pub mod fid' vendor/truck/truck-evidence/src/lib.rs"}
  - {id: A9, expect: 1, cmd: "grep -c 'pub fn sin' vendor/truck/truck-evidence/src/elementary.rs"}
```

## Problem

Everything downstream of the formal system compares against certified local
feature-size evidence — but nothing in truck computes any, and the scaffold
(`truck-evidence/src/fid/lfs.rs`) holds only contract prose. Two scaffold
gestures are NOT soundly available and must not be faked:

- Federer's closed-manifold reach decomposition does not transfer to trimmed
  patches by citation; until it does (open obligation L-FEDERER-PATCH), no
  computed quantity may claim to bound a tube width, reach or lfs — even
  though each COMPONENT direction is certifiable today.
- The scaffold's `ϱ_wedge` coinage has no theorem behind it; what [CCSL09]
  defines (Def 4.3) is χ_K(t), an infimum over an entire distance locus. What
  BG-INV-109 actually witnesses is ONE point per edge (a midpoint normal-pair)
  with a certified lower bound on sin φ. That supports a LOCAL normalized-
  slope lower bound and nothing more.

This packet therefore ships PRIMITIVE EVIDENCE ONLY: `FaceScaleComponents`
(three independently certified directions + an explicit conservative min) and
`WedgeSlopeLowerBound` (local, witnessed-scope). The certificate types that
WOULD overclaim (`TubeWidthLowerBound`, `ChiLowerBound`) are deliberately NOT
created; they appear later, constructed only by proof-bearing code once their
bridge lemmas land. That is the evidence architecture working as designed.

## Decisions already made for you

### Decision 0 — topology-free core

The module depends ONLY on `crate::enclosure` types (`Box3`,
`EnclosureSurface`, `Interval`). No truck-topology dependency: callers pass a
surface carrier, its parameter cell, and CERTIFIED BOXES for structures to
exclude. Wiring real Shells into strata traversal is a later packet.

### Decision 1 — typed refusals everywhere, never Option

Refusal provenance is part of this kernel's semantics; do not throw it away:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FidRefusal {
    /// The immersion margin collapsed on this cell: the normal direction is
    /// not certifiably well-defined there.
    ImmersionUnresolved,
    /// The first-form eigenvalue bracket could not certify a positive lower
    /// bound on this cell (cell too wide for the interval arithmetic).
    MetricLowerBoundUnresolved,
    /// An input margin was outside its mathematical domain.
    InvalidMargin,
    /// Fewer than two witness boxes were supplied where two are required.
    InsufficientWitnesses,
}
```

Every public function returns `Result<T, FidRefusal>`. Map each refusal cause
exactly as named — a caller must be able to distinguish "geometry too curved
to certify here" from "you gave me garbage".

### Decision 2 — the curvature term (face-interior intrinsic radius)

Implement EXACTLY this (scratch-validated on the sphere carrier; sound
everywhere it answers). **Note the F-magnitude term — using `F.sup^2` there
is a SOUNDNESS REVERSAL** (for F = [-10,-1], sup^2 reads 1 while sup|F^2| =
100, which inflates lam_min_lo and DEFLATES the curvature bound):

```
curvature_radius_lower(surface, (uu,vv)) -> Result<f64, FidRefusal>
    su  = surface.enclose_der(1,0,uu,vv)     # Box3 interval enclosures
    sv  = surface.enclose_der(0,1,uu,vv)
    s2u = surface.enclose_der(2,0,uu,vv)
    s12 = surface.enclose_der(1,1,uu,vv)
    s2v = surface.enclose_der(0,2,uu,vv)
    E = dot_box(su,su); F = dot_box(su,sv); G = dot_box(sv,sv)
    N_raw = cross_box(su,sv)
    iota = surface.immersion_lower_bound(uu,vv)
    if iota <= 0 { return Err(ImmersionUnresolved) }
    L_up = mag_up(dot_box(s2u,N_raw)) / iota   # mag_up(i)=max(|i.inf|,|i.sup|)
    M_up = mag_up(dot_box(s12,N_raw)) / iota
    N_up = mag_up(dot_box(s2v,N_raw)) / iota
    delta_mag = max(|E.sup - G.inf|, |G.sup - E.inf|)
    f_mag     = max(|F.inf|, |F.sup|)          # sup|F|^2 <= f_mag^2 ALWAYS
    disc_up   = sqrt(delta_mag*delta_mag + 4*f_mag*f_mag)
    lam_min_lo = 0.5*(E.inf + G.inf - disc_up)
    if lam_min_lo <= 0 { return Err(MetricLowerBoundUnresolved) }
    k_up = (L_up + M_up + N_up) / lam_min_lo
    if k_up == 0 { return Ok(f64::INFINITY) }  # flat within enclosure: intentional
    Ok(1.0 / k_up)
```

Justification to encode as comments: every pointwise λ_min of [[E,F],[F,G]]
is ≥ ((E+G) − sqrt((E−G)²+4F²))/2 evaluated at interval worst-cases, PROVIDED
sup((E−G)²) ≤ delta_mag² (true by construction) and sup(F²) ≤ f_mag² (the
fix above — verify you understand why before writing it). Normalization uses
the carrier's own `immersion_lower_bound` (the iota route): measured on the
sphere it matches refusals but beats the naive cross-box norm bracket. The
sum-of-coefficients numerator is deliberately coarse; over-estimation costs
only eps budget downstream. Do NOT add subdivision or tightness improvements.

### Decision 3 — FaceScaleComponents (NOT TubeWidthLowerBound)

```rust
/// Three independently certified component directions for one face cell.
/// This type makes NO claim about tubes, reach or feature size: composing
/// these into a tube-width statement requires L-FEDERER-PATCH (open).
pub struct FaceScaleComponents {
    pub curvature_radius_lower: f64,        // from Decision 2; +inf permitted
    pub nonincident_separation_lower: f64,  // d(cell image, exclusion boxes); +inf if none
    pub boundary_distance_lower: f64,       // d(cell image, boundary boxes); +inf if none
}
impl FaceScaleComponents {
    pub fn conservative_min(&self) -> f64 { /* min of the three */ }
}
pub fn face_scale_components(
    surface: &impl EnclosureSurface,
    cell: (Interval, Interval),
    nonincident_boxes: &[Box3],
    boundary_boxes: &[Box3],
) -> Result<FaceScaleComponents, FidRefusal>
```

- `box_distance(a,b)` = lower bound on point-set distance: per-axis
  `max(lo_b - hi_a, lo_a - hi_b)` clamped at 0, Euclidean-combined. Certified
  because `surface.enclose(cell)` contains the whole image.
- **Empty-set semantics, explicit:** `d(A, ∅) = +∞`; both distance components
  are `+∞` when their slice is empty, and `conservative_min()` of components
  including `+∞` ignores them exactly as extended reals. Infinity is
  intentional and permitted (plane ⇒ flat ⇒ curvature radius `+∞`).
- Doc comment states what this does NOT establish: topological thickening,
  local reach, isotopy. Cite L-FEDERER-PATCH as open.

### Decision 4 — WedgeSlopeLowerBound (NOT ChiLowerBound)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WedgeScope { EdgeMidpointWitness }   # INV-109 v1 samples a POINT
pub struct WedgeSlopeLowerBound { pub value: f64, pub scope: WedgeScope }

pub fn wedge_slope_lower_from_sin_margin(sin_margin: f64)
    -> Result<WedgeSlopeLowerBound, FidRefusal>
    # InvalidMargin unless 0 < sin_margin <= 1
    value = sqrt((1 - sqrt(1 - sin_margin*sin_margin)) / 2)
```

Derivation to encode as comments (this IS the deliverable):
- For a wedge whose adjacent face normals make angle φ ∈ [0,π]:
  d(0, conv{n_A,n_B}) = cos(φ/2) — the local normalized-slope value on the
  bisector region. It dies correctly at BOTH knife degeneracies (folded ψ→0
  and crack ψ→2π force antiparallel normals, φ→π) and equals 1 when flat
  (φ→0).
- BG-INV-109 certifies sin φ >= sin_margin, i.e. φ ∈ [arcsin s, π−arcsin s];
  cos(φ/2) is decreasing there, so the sound WORST case sits at the right
  endpoint: cos((π−arcsin s)/2) = sin(arcsin s/2) = the formula above.
  Monotone increasing in s; →0 when no non-degeneracy is certified.
- KNOWN LIMITATION, documented not hidden: at s = 1 the bound still reports
  1/√2 because a sine certificate cannot see branch identity. Distinguishing
  healthy near-flat (dot(n_A,n_B) >= c) from near-knife (dot <= −c) needs
  SIGNED alignment evidence INV-109 lacks; note the useful direction — to
  improve THIS lower bound one wants an upper bound on φ, i.e. a lower bound
  on the dot product. Extending INV-109 is future work, not this packet.
- Naming discipline: this is NOT ChiLowerBound. χ_K(t) infers over an entire
  distance locus; promoting local wedge evidence to it requires L-COVERAGE —
  future type-level promotion, not prose.

Vertex star separation is DEFERRED out of this packet entirely: incident-star
boxes share the vertex, so pairwise box distances are trivially zero — sound
but vacuous without a certified excluded ball, which nothing yet supplies.
(The scaffold's vertex row stays prose.)

### Decision 5 — structured annotations (@feeds form, mandatory)

Every public item carries immediately above it an annotation block. USE THIS
FORM — "Thm instance" would be false: FID-001 does not instantiate CCS05's
theorem, it produces evidence intended eventually to discharge one hypothesis.

```
/// @feeds [CCS05, Thm 2.1:H2]            # would supply thickening containment
/// @via-open-lemma FID-L-TUBE
/// @establishes
///   component-wise certified directions (this struct)
/// @does-not-establish
///   topological thickening | local reach | isotopy
```

and for the edge term:

```
/// @definition [CCSL09, Def 4.3]          # chi_K - a definition, not an instance
/// @uses-lemma FID-L-WEDGE-SLOPE
/// @establishes local normalized-slope lower bound at the witnessed point
/// @does-not-establish global chi_K
/// @feeds-open-lemma FID-L-COVERAGE
```

A definition citation is not a theorem instance either — keep the tags honest.
These annotations are reviewable claims; a wrong tag is a `disagreements`
finding.

### Decision 6 — module layout

`fid/lfs.rs`: the refusal enum, the two evidence types (+ scopes), functions
from decisions 2-4, private helpers `dot_box`, `cross_box`, `mag_up`,
`box_distance` (duplicate locally — enclosure.rs visibility stays untouched;
read_allow covers reading, write_allow does not cover touching).
`fid/mod.rs`: one doc line ("scaffold filled by BG-FID-001; bridge lemmas
L-TUBE/L-COVERING/L-SEPARATES/L-FEDERER-PATCH/L-COVERAGE remain open").
Everything under `#![deny(clippy::unwrap_used)]` INCLUDING the test module —
GATE-1 gates new modules on it and truck-evidence currently has zero deny
attributes (measured); test floats use named consts with `// H-3:` comments.

### Decision 7 — tests (all in lfs.rs's test module)

Use `Plane` (read plane.rs for its impl and parameter range) and
`Interval::try_from((lo,hi)).unwrap_or(Interval::EMPTY)` as iv() helper.

1. `cube_face_components_upper_bound` — three Plane cells of a unit cube;
   assert each `conservative_min() <=` hand-computed distances (nearest edge,
   neighbouring sheet). `<=`, never `==`.
2. `global_scale_zero_stratified_positive` — the cube's GLOBAL feature size
   is 0; assert all three components are strictly positive on interior cells
   away from edges. Anti-regression against a future global-reach shortcut.
3. `translation_invariance` — translate the whole configuration by t ≠ 0:
   every component equal within a tiny named const slack (AABB box-distance
   IS translation invariant).
4. `rotated_configuration_stays_sound` — rotate the cube configuration:
   do NOT assert equality (AABB separation bounds are not rotation-tight);
   assert each rotated-case conservative_min() is positive AND <= the true
   hand-computed distance (soundness across orientation change).
5. `wedge_slope_monotone_and_knife_limit` — monotone increasing in
   sin_margin over (0,1]; → 0 as s → 0; refuses at s = 0 and s > 1
   (InvalidMargin). At exact antiparallel normals sin φ = 0 the underlying
   INV-109 check fails that solid — assert the refusal propagates.
6. `sphere_curvature_term_soundness` — Sphere r=2: curvature_radius_lower
   <= 2.0 (its TRUE radius of curvature — soundness direction!) on cells
   around (u,v)=(1.1,0.7) at widths {0.125, 0.0625, 0.03125} (named consts);
   reference k_up values from the validated scratch: ≈10.2356, 2.7971,
   1.7339 → radii ≈0.0977, 0.3575, 0.5768; assert <= 2.0 + slack, strict
   increase under refinement, pole-straddling cell u ∈ [-0.1, 0.1] returns
   ImmersionUnresolved or MetricLowerBoundUnresolved.
7. `wedge_formula_matches_geometry` — plane pairs with known normal angles
   φ ∈ {30°, 90°, 150°}: compute sin φ from the planes' actual normals, feed
   the measured value, assert (a) result == the closed-form expression to
   final bits, (b) the GEOMETRIC claim numerically: dist(0, segment[n_A,n_B])
   = cos(φ/2) computed directly satisfies `result <= cos(φ/2) + slack` for
   every tested φ.

All float comparisons through named consts with `// H-3:` same-line comments.
No bare 1e-N anywhere.

## H-3, the house rule that rejects bare float literals

GATE-2 of `scripts/kernel-gates.sh` fails any **added** line carrying a bare
`1e-N` literal unless that same line ends with an `// H-3` comment. This
packet is full of floats; EVERY epsilon, tolerance and slack is a named const
whose defining line carries a same-line `// H-3:` comment naming the
dimensionless quantity. Run `bash scripts/kernel-gates.sh <your base>` before
writing RESULT.json — it is the same script V4 runs.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-evidence
cargo clippy -p truck-evidence --all-targets --no-deps
cargo test -p truck-evidence --lib --no-fail-fast
cargo check --workspace --all-targets
bash scripts/kernel-gates.sh <base>        # base = merge-base (integration tip)
```

truck-evidence is green at baseline (measured this session). Any baseline
failure you did not cause is a stop condition. Send cargo output to a file
and read the tail. Never run a bare `cargo test`.

## Forbidden

Naming anything `reach`, `lfs`, `TubeWidthLowerBound` or `ChiLowerBound`
(Decision 3/4 discipline; the scaffold's `LfsLowerBound` name survives only in
preserved doc prose). Editing files outside `write_allow` — enclosure.rs
visibility stays as-is, wedge.rs read-only, no Cargo.toml changes. Returning
Option where Decision 1 demands typed refusals. Adding subdivision loops or
tightness improvements to the curvature term. Claiming any bridge lemma as
proved, or writing a "Thm instance" tag for something that merely @feeds it.
Bare float literals without `// H-3`. Adding `unwrap()`/`expect()`/`panic!`
on fallible production paths. Committing to `main`.

## Stop conditions

- an anchor count differs → `ANCHOR_MISMATCH`, naming the anchor
- any formula provably contradicts its cited source (e.g. you can show the
  worst-case interval argument for the wedge bound is wrong) → `SPEC_GAP`
  naming the step
- Plane's EnclosureSurface impl turns out missing or insufficient for the
  test constructions → `SPEC_GAP` naming the gap (do NOT add an impl —
  different packet)
- three consecutive failed `cargo` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. Commit
on the current branch with subject

```
feat(evidence): fid face-scale components and wedge-slope evidence (BG-FID-001)
```
