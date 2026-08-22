# WORK PACKET BG-FID-001 — stratified tube-width and edge χ certificates

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
budget:      {turns: 40, ctx_tokens: 95000}
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

Everything downstream of the formal system compares against a lower bound on
local feature size — but nothing in truck computes one, and the scaffold
(`truck-evidence/src/fid/lfs.rs`) holds only the contract prose. Worse, two of
the quantities the scaffold gestures at are NOT soundly available:

- Federer's closed-manifold reach decomposition does not transfer to trimmed
  patches by citation; until it does (open obligation L-FEDERER-PATCH), the
  face-interior term must ship under a name that does NOT claim reach.
- The scaffold's `ϱ_wedge` coinage has no theorem behind it; what the
  literature certifies ([CCSL09] Def 4.3) is a critical-function quantity χ_K,
  and what BG-INV-109 actually provides is a certified LOWER bound on
  `sin φ` — φ being the angle between adjacent faces' normals at an edge.

This packet implements both terms with their honest semantics and names:
`tube_width_lower` for the face interior (the three-way min), `chi_lower` for
edges. The naming IS the contract: no function in this module may use the
words `reach` or `lfs` in its name or doc signature until the bridge lemmas
recorded in THEOREM-MAP land.

## Decisions already made for you

### Decision 0 — topology-free core

The module depends ONLY on `crate::enclosure` types (`Box3`,
`EnclosureSurface`, `Interval`) plus `elementary` if needed. No truck-topology
dependency: callers pass a surface carrier, its parameter cell, and CERTIFIED
BOXES for the structures to exclude (non-incident sheets, boundary wires).
Wiring real Shells into strata traversal is a later packet. This keeps the
write set inside `fid/` and every claim testable against hand-built carriers.

### Decision 1 — the curvature term (face-interior intrinsic)

Implement EXACTLY this (scratch-validated on the sphere carrier this session;
sound everywhere it answers, refuses where it cannot certify):

```
rho_max_upper(surface, (uu,vv)) -> Option<f64>:
    su  = surface.enclose_der(1,0,uu,vv)     # Box3 interval enclosures
    sv  = surface.enclose_der(0,1,uu,vv)
    s2u = surface.enclose_der(2,0,uu,vv)
    s12 = surface.enclose_der(1,1,uu,vv)
    s2v = surface.enclose_der(0,2,uu,vv)
    E = dot_box(su,su); F = dot_box(su,sv); G = dot_box(sv,sv)
    N_raw = cross_box(su,sv)
    iota = surface.immersion_lower_bound(uu,vv)
    refuse if iota <= 0                       # immersion margin collapsed
    L_up = mag_up(dot_box(s2u,N_raw)) / iota   # mag_up(i) = max(|inf|,|sup|)
    M_up = mag_up(dot_box(s12,N_raw)) / iota
    N_up = mag_up(dot_box(s2v,N_raw)) / iota
    trace_lo = E.inf + G.inf
    disc_up  = sqrt(max(|E.sup - G.inf|, |G.sup - E.inf|)^2 + 4*F.sup^2)
    lam_min_lo = 0.5*(trace_lo - disc_up)
    refuse if lam_min_lo <= 0
    Some((L_up + M_up + N_up) / lam_min_lo)
```

Normalization uses the carrier's own `immersion_lower_bound` (the iota route),
NOT a naive `sqrt(dot_box(N_raw,N_raw).inf)` — measured on the sphere: same
refusals but consistently tighter, and the naive route's denominator can
collapse on cells where the true immersion is fine. The sum-of-coefficients
numerator is deliberately coarse; over-estimation costs only eps budget
downstream (BG-FID-007 inequality form) and MUST NOT be "optimized" in this
packet. Do not add subdivision here: refusal-driven subdivision was measured
to certify wide cells only with terrible constants, and tightness-driven
subdivision is out of scope unless a downstream inequality fails.

### Decision 2 — face-interior tube width (the three-way min)

```
tube_width_lower(surface, cell, non_incident: &[Box3], boundary_wires: &[Box3])
  -> Option<TubeWidthLowerBound>
  rho   = rho_max_upper(surface, cell)?            # certified curvature radius
  d_bnd = min over boundary_wires of box_distance(surface.enclose(cell), w)
  d_inc = min over non_incident     of box_distance(surface.enclose(cell), b)
  Some(min(1/rho, d_bnd, d_inc))
```

`box_distance(a, b)` = lower bound on the distance between any point of box a
and any point of box b: per-axis `max(lo_b - hi_a, lo_a - hi_b)` clamped at 0,
then Euclidean-combined. Both distance terms are certified because
`surface.enclose(cell)` contains the whole image and box-to-box distance is a
lower bound on point-set distance. Semantics of the min: "certified
single-valuedness radius of the normal tube over this cell" — the doc comment
must say exactly that, must cite L-FEDERER-PATCH as the open obligation that
would justify reading it as local reach, and must NOT use the words reach or
lfs (see Decision 5).

### Decision 3 — the edge χ certificate (BG-FID-001a)

Implement the closed-form conversion with its derivation as structured
comments:

```
chi_lower_from_sin_margin(sin_margin: f64) -> Option<ChiLowerBound>
    refuse if sin_margin <= 0 or > 1          # INV-109 passing means >= margin > 0
    value = sqrt((1 - sqrt(1 - sin_margin^2)) / 2)
```

Derivation to encode in comments (this IS the deliverable, not decoration):
- For a wedge whose adjacent face normals make angle φ, the minimum norm over
  the generalized gradient conv{n_A, n_B} equals cos(φ/2); cite that this dies
  correctly at BOTH knife degeneracies (folded ψ→0 and crack ψ→2π both force
  antiparallel normals, φ→π) while staying 1 when the surface is flat across
  the edge (φ→0).
- BG-INV-109 certifies `|n_A × n_B| >= sin_margin`: a lower bound on sin φ.
  sin φ bounds φ away from BOTH 0 and π, so the sound worst case over the
  certified interval `[arcsin(sin_margin), π − arcsin(sin_margin)]` is taken
  at φ = π − arcsin(sin_margin), giving cos(φ/2) = sin(arcsin(sin_margin)/2)
  = the formula above. Monotone increasing in sin_margin; →0 when no
  non-degeneracy is certified (knife witnesses then route to collapse per
  BG-FID-002's routing rule).
- KNOWN LIMITATION, documented not hidden: at sin_margin = 1 the bound still
  reports only 1/√2, because a sine certificate cannot see branch identity
  (healthy-flat vs near-knife). A branch-specific bound needs a SIGNED
  normal-alignment certificate (`dot(n_A,n_B) <= -c`), which BG-INV-109 does
  not provide; extending INV-109 is future packet work, NOT this packet.
- SCOPE: BG-INV-109 v1 samples each edge midpoint only, so the certificate
  this function consumes carries scope MidpointCell. Type:
  `ChiLowerBound { value: f64, scope: EdgeScope }`, `enum EdgeScope {
  MidpointCell }`. Consumers must not read it as whole-edge.

### Decision 4 — vertex star separation

```
vertex_star_separation(star_boxes: &[Box3]) -> Option<f64>
    min over distinct pairs of box_distance(a,b); None if fewer than 2 boxes
```

Doc comment cites the strata table's star-separation row and states that this
feeds χ_K composition (open obligation L-COVERAGE: local certificates do NOT
compose into global r_mu/wfs without certified coverage — say so).

### Decision 5 — structured theorem comments (mandatory form)

Every public function carries, immediately above it, a citation block naming
the theorem instance it instantiates and which hypothesis each input
discharges. Form:

```
// Thm instance [CCS05 Thm 2.1]: isotopy from containment-in-thickening +
// side separation (+ homeomorphy). This fn certifies the TUBE whose sides
// the approximant must separate. Bridge lemmas L-TUBE/L-COVERING/
// L-SEPARATES are OPEN obligations — see THEOREM-MAP; none is claimed here.
```

and for the edge term:

```
// Thm instance [CCSL09 Def 4.3] chi_K via normalized slope; derivation per
// BG-FID-001a. Scope MidpointCell inherits BG-INV-109 v1 sampling.
```

Cite [CCSL09-DCG] for χ_K definition and THEOREM-MAP for the bridge-lemma
register. These comments are reviewable claims: a wrong citation is a
`disagreements` finding.

### Decision 6 — module layout

`fid/lfs.rs` gains: the two output types (`TubeWidthLowerBound(f64)`,
`ChiLowerBound{value,scope}`), `EdgeScope`, the four functions of decisions
1-4, private helpers `dot_box`, `cross_box`, `mag_up`, `box_distance`
(duplicate these locally — do NOT widen enclosure.rs visibility; read_allow
covers reading it, write_allow does not cover touching it). `fid/mod.rs`
gains one doc line pointing at the implemented state ("scaffold filled by
BG-FID-001; bridge lemmas remain open"). Everything carries
`#![deny(clippy::unwrap_used)]` at the top of lfs.rs INCLUDING its test
module — GATE-1 gates new modules on it and the crate currently has zero
deny attributes (measured), so there is no house pattern to copy inside this
crate; test-module float comparisons use named consts with `// H-3:` comments
instead of unwrap-style escapes.

### Decision 7 — tests (all in lfs.rs's test module)

Use `Plane` (read plane.rs for its EnclosureSurface impl and parameter range)
for cube faces and `Interval::try_from((lo,hi)).unwrap_or(Interval::EMPTY)`
as the iv() helper pattern (see enclosure.rs tests).

1. `cube_face_interior_upper_bound` — three Plane cells of a unit cube
   meeting at a corner; assert `tube_width_lower <=` the hand-computed
   distances (distance to nearest edge, to the neighbouring sheet). `<=`
   never `==`.
2. `global_reach_zero_stratified_positive` — the cube's GLOBAL feature size
   is 0 (its edges collapse); assert the stratified terms are strictly
   positive on interior cells away from edges. This is the anti-regression
   test: a future "simplification" back to a global quantity fails here.
3. `scale_homogeneity` — scale the cube by k>0 (rebuild planes through scaled
   points): every bound multiplies by k (within a tiny named const slack).
   1-homogeneity under uniform scale.
4. `rigid_motion_invariance` — rotate+translate the cube; all bounds equal
   within the named const slack.
5. `knife_edge_routes_to_zero` — two Plane cells sharing an edge with
   included angle ψ between normals' antiparallel limit: compute the actual
   sin φ of the witness pair, feed chi_lower_from_sin_margin; assert
   monotonicity (larger certified sin_margin => larger-or-equal chi) and that
   chi -> 0 as the witness's sin φ -> 0. At exact antiparallel (sin φ = 0)
   the function refuses (INV-109 would fail that solid) — assert the refusal.
6. `sphere_curvature_term_soundness` — Sphere r=2: `rho_max_upper >= 0.5`
   on cells around (u,v)=(1.1,0.7) at widths {0.125, 0.0625, 0.03125}
   (named consts); pole-straddling cell u ∈ [-0.1, 0.1] REFUSES; ratios
   shrink under refinement (assert k_up(0.0625) < k_up(0.125)). Reference
   values from the validated scratch: k_up ≈ 10.2356, 2.7971, 1.7339 at the
   three widths — assert soundness (>= 0.5) and strict decrease, NOT
   equality (interval arithmetic is allowed to differ in final bits).
7. `wedge_formula_matches_geometry` — pick wedge witnesses with known normal
   angles (φ = 30°, 90°, 150°: build Plane pairs accordingly), compute
   |n_A × n_B| = sin φ numerically from the planes' own normals, feed
   chi_lower_from_sin_margin(sqrt(1-s²)... i.e. feed the MEASURED sin), and
   compare against cos(φ_measured/2): the closed-form worst case must satisfy
   `chi_lower <= cos(φ/2) + tiny_slack` AND equality must hold when φ ≥ π/2
   branch... precisely: assert `chi_lower == sin(asin(s)/2)` to final bits
   (it is the same expression), and separately assert the GEOMETRIC claim
   `min-norm-over-conv{n_A,n_B} = cos(φ/2)` numerically: compute
   dist(0, segment[n_A,n_B]) directly and check `chi_lower <= that + slack`.

All floats compared through named consts with `// H-3:` same-line comments.
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
bash scripts/kernel-gates.sh <base>        # base = c7f6ae3 (merge-base)
```

truck-evidence is green at baseline (measured this session). Any baseline
failure you did not cause is a stop condition. Send cargo output to a file
and read the tail. Never run a bare `cargo test`.

## Forbidden

Naming anything `reach` or `lfs` (Decision 5's discipline — the scaffold's
`LfsLowerBound` name may appear ONLY in the preserved scaffold doc prose).
Editing files outside `write_allow` — in particular enclosure.rs visibility
stays as-is, wedge.rs is read-only, and no Cargo.toml changes (everything
needed is already declared). Adding subdivision loops or "tightness
improvements" to the curvature term. Claiming any bridge lemma as proved.
Bare float literals without `// H-3`. Adding `unwrap()`/`expect()`/`panic!`
on fallible paths in production code (test modules follow Decision 6's
attribute rule). Committing to `main`.

## Stop conditions

- an anchor count differs → `ANCHOR_MISMATCH`, naming the anchor
- any formula here provably contradicts the cited source (e.g. you can show
  the worst-case interval argument for chi_lower is wrong) → `SPEC_GAP`
  naming the step
- Plane's EnclosureSurface impl turns out not to exist or not to support the
  cell construction the tests need → `SPEC_GAP` naming the gap (do NOT add
  an impl — that is a different packet)
- three consecutive failed `cargo` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. Commit
on the current branch with subject

```
feat(evidence): fid tube_width_lower and edge chi_lower certificates (BG-FID-001)
```
