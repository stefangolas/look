# WORK PACKET BG-SOL-S7-SING-CLASSIFY - the singular-event stage

Classify the GFF stage's singular candidate boxes by contact-locus dimension:
certify isolated tangency points, detect carrier-degenerate contacts and
tangential crossings, recover regular crossings hiding inside broad singular
domains, and defer everything this stage cannot prove. If live code
contradicts this packet, report it in `disagreements`.

```json
{"id":"BG-SOL-S7-SING-CLASSIFY","status":"DONE","contracts":["BG-SOL-S7-SING-CLASSIFY"],
 "tests_added":0,"sites_migrated":0,"sites_deferred":0,
 "deviations":[],"disagreements":[],"baseline_failures":[],"notes":"free text"}
```

```yaml
id:          BG-SOL-S7-SING-CLASSIFY
contract:    [BG-SOL-S7-SING-CLASSIFY]
class:       design
crates:      [truck-evidence]
write_allow:
  - vendor/truck/truck-evidence/src/contact/singular.rs
  - vendor/truck/truck-evidence/src/contact/mod.rs
read_allow:
  - vendor/truck/truck-evidence/src/contact/implicit.rs
  - vendor/truck/truck-evidence/src/contact/gff.rs
  - vendor/truck/truck-evidence/src/num/krawczyk.rs
  - vendor/truck/truck-evidence/src/enclosure.rs
  - vendor/truck/truck-base/src/evidence.rs
tests_required:
  - singular_refines_regular_cover_from_broad_singular_domain
  - singular_certifies_isolated_external_tangency
  - singular_classifies_internal_tangency_as_crossing
  - singular_certifies_degenerate_apex_contact
  - singular_events_are_order_insensitive
  - contact_ff_internal_tangency_pair_stays_deferred
budget:      {turns: 45, ctx_tokens: 120000}
anchors:
  - {id: A1, expect: 3, cmd: "grep -c 'pub mod' vendor/truck/truck-evidence/src/contact/mod.rs"}
  - {id: A2, expect: 16, cmd: "grep -c 'singular' vendor/truck/truck-evidence/src/contact/mod.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'ContactLocus::Point' vendor/truck/truck-evidence/src/contact/mod.rs"}
  - {id: A4, expect: 6, cmd: "grep -c 'gff::cover_branch' vendor/truck/truck-evidence/src/contact/mod.rs"}
  - {id: A5, expect: 0, cmd: "ls vendor/truck/truck-evidence/src/contact/singular.rs 2>/dev/null | wc -l"}
```

Anchors describe the PRE-EDIT state. A1 grows to 4 (`pub mod singular;`),
A2 and A3 grow; A4 stays 6; A5 becomes 1. A3's single pre-existing
`ContactLocus::Point` use is in the FE/EE stage - do not touch it.

## Problem

Post-CHART, `BranchCover::singular_boxes` holds UNSUBDIVIDED domains where
all three cross-gradient minors merely contain zero: chart-artifact boxes are
already recovered, but a singular domain may still contain (a) regular
crossings whose tangent directions vary too much for a domain-level chart,
(b) isolated tangency points, (c) gradient-parallel saddle points where the
contact locus crosses itself, (d) carrier-degenerate contact points (cone
apex on the other carrier), or any mix. The dispatcher currently refuses
every such pair with `ContactReductionDeferred`. This packet lands the
classifier: refine, then classify each resolution-floor residue leaf, then
let the dispatcher emit `Point0`/`Tangency` records for PROVEN isolated
tangencies and defer everything else with named reasons.

## Decisions already made

### 1. Module and public shape

New module `contact/singular.rs` (H-1 deny header like `gff.rs`), declared
`pub mod singular;` in `contact/mod.rs`:

```rust
pub struct SingularReport {
    /// Crossings certified inside chartable children of the singular cells,
    /// accumulated in discovery order (the regular cover's own lists).
    pub regular: BranchCover,
    /// Certified isolated tangency points: unique Lagrange root with
    /// definite restricted Hessian; the contact set in a neighborhood of
    /// each point is exactly that point.
    pub tangencies: Vec<Point3>,
    /// Certified gradient-parallel saddle points: unique Lagrange root with
    /// INdefinite restricted Hessian. The contact locus self-crosses there;
    /// NOT isolated; deferred with the point recorded.
    pub tangential_crossings: Vec<Point3>,
    /// Certified carrier-degenerate contact points (e.g. cone apex on the
    /// other carrier). Local branch topology unclassified; deferred.
    pub degenerate: Vec<Point3>,
    /// Resolution-floor leaves that certified nothing. Dimension unknown.
    pub residue: Vec<Box3>,
}

pub fn singular_events(
    f1: &impl ImplicitField,
    f2: &impl ImplicitField,
    cells: &[Box3],
    tau: f64,
    budget: &mut Budget,
) -> Outcome<SingularReport>;
```

`cover_branch` and `implicit.rs` are NOT edited. All doc comments above must
carry the substance of their claims (isolation scoping: the isolation claim
is about a neighborhood of the certified point, not about the whole leaf;
sub-tau structure is the resolution contract's business, exactly as for
regular cover points).

### 2. Refinement

Worklist starts with `cells` in the given order (LIFO stack, gff
convention). Pop a box `b`; call `gff::cover_branch(f1, f2, &b, tau,
budget)`:

- every refusal propagates (budget exhaustion);
- merge its `points` and `unresolved_boxes` into `report.regular`
  (appended after existing entries, discovery order);
- for each `s` in its `singular_boxes` (0 or 1, equal to `b` when no chart
  certified): if `s`'s widest axis width is `<= tau`, push `s` to the
  residue candidates; otherwise bisect `s` on its widest axis (ties toward
  the lowest axis index, convex-combination midpoint exactly as
  `krawczyk::push_children` does), spending one subdivision per bisection,
  and push both children.

`cover_branch` on a chartless box returns immediately after its own
field-exclusion screen, so descent is cheap; a chartable child runs the full
inner cover, which is exactly what we want - it re-selects a chart per child
(the per-child chart use CHART deferred from its own scope; connectivity
stays unclaimed). Capture the entry budget once; spend reporting mirrors
`cover_branch` exactly (exhaustion refusals carry `initial - remaining`).

### 3. Classification of each residue leaf, in this order

**(a) Degenerate pass.** For each `q` in `f1.degenerate_points()` and
`f2.degenerate_points()`: if `q` lies inside the leaf (componentwise
`>= inf` and `<= sup`, exact f64 comparisons) AND the other field's
enclosure over the degenerate point-box `Box3::point(q)` contains 0.0, the
leaf is DONE: record `q` in `degenerate` (dedup per decision 6) and do not
run the Lagrange step on this leaf.

**(b) Lagrange system.** Only if (a) found nothing. Orientation is fixed:
constraint `f1`, objective `f2`. Unknowns `(x, y, z, lam)`:

```text
F = [ f1(x,y,z),
      d2x(f2) + lam*d2x(f1),
      d2y(f2) + lam*d2y(f1),
      d2z(f2) + lam*d2z(f1) ]
```

(the three trailing rows are the components of `grad(f2) + lam*grad(f1)`).
`f_point` evaluates the point exactly (degenerate intervals). The Jacobian
over the 4-box, row-major, columns `(x, y, z, lam)`, from `grad`/`hess`
enclosures:

- row 0: `[df1/dx, df1/dy, df1/dz, 0]` over the leaf's x/y/z;
- row `1+i` (axis `i`): columns `j` in {x,y,z} are
  `hess(f2)[i][j] + lam_interval * hess(f1)[i][j]`; the `lam` column is
  `grad(f1)` component `i`.

`preconditioner(x)` builds the same Jacobian at the f64 point (entries
extracted from the degenerate point-box enclosures, `.inf()` of a degenerate
interval), inverts it with a PRIVATE Gauss-Jordan with partial pivoting
(unrolled or `.get()`-based - the crate denies indexing), and returns `None`
when the best pivot is zero or non-finite. `krawczyk` bisects on `None` by
design.

The `lam` interval is the sound envelope, derived and fixed: every tangency
`t` in the leaf with `grad(f1)(t) != 0` satisfies
`|lam(t)| = |grad(f2)(t)| / |grad(f1)(t)| <= sup|grad(f2)| / inf|grad(f1)|`.
With `delta = max_k inf_leaf |df1/dx_k|` (positive only when some gradient
component enclosure excludes zero) and
`B2 = sqrt(sum_k sup_leaf |df2/dx_k| squared)`, set `lam in [-B2/delta,
B2/delta]`. If `delta == 0` the envelope does not exist: the leaf goes to
`residue` (honest - the leaf may contain an f1-degenerate locus that (a) did
not enumerate exactly). `B2/delta` covers `lam(t) = 0` trivially (0 is
inside), so tangencies AT f2-degenerate points are still found when the
envelope exists.

Run `krawczyk::<4>` on `[leaf.x, leaf.y, leaf.z, lam_box]`:

- `Unique`: extract the root - 2-3 Newton steps from the 4-box midpoint
  using the same f64 Jacobian inverse (mirror `gff::refine_point`'s
  pattern); the point is the refined `(x, y, z)`, the multiplier the
  refined `lam`. Go to (c).
- `NoRoot`: the leaf provably contains NO tangency (sound envelope + unique
  criticality argument above) and (a) found no degenerate contact: whatever
  kept it from charting is a resolution issue, not a singular point. Move
  the leaf to `report.regular.unresolved_boxes`.
- refusal: budget exhaustion propagates.

**(c) Restricted-Hessian inertia at the certified root.** `H = hess(f2) +
lam*·hess(f1)` over the root's point-box (`lam*` the refined multiplier).
Normal `n = grad(f1)` at the same point-box, extracted to f64 (degenerate
enclosure; `delta > 0` guaranteed it is nonzero). Deterministic tangent
basis: `a` = index of the largest `|n_i|` (ties toward lowest index);
`u = normalize(n cross e_a)`, `v = normalize(n cross u)`, f64 arithmetic.
Restricted 2x2 `R[i][j] = basis_i^T * H * basis_j` with interval dot
products. Classify:

- `det(R).inf() > 0.0 && R[0][0].inf() > 0.0` -> definite positive;
- `det(R).inf() > 0.0 && R[1][1].sup() < 0.0` -> definite negative;
- `det(R).sup() < 0.0` -> indefinite;
- otherwise -> the leaf goes to `residue`.

Definite (either sign): the point is an isolated strict local extremum of
`f2` restricted to `f1`'s surface at value 0, so the contact set in a
neighborhood is exactly that point - record in `tangencies`. Indefinite:
`f2|surface` has a saddle at value 0, so its zero set crosses itself there -
record in `tangential_crossings`. A final sanity check: the root's point-box
enclosure of `f2` must contain 0.0 (it is a contact point); if it does not,
the certified critical point is not contact - record the leaf in `residue`.

### 4. Dispatcher wiring (contact/mod.rs)

In `validated_ff`, replace the `!cover.singular_boxes.is_empty()` early
return with: call `singular::singular_events(l, r, &cover.singular_boxes,
tau, budget)?`; append `report.regular.points` to `cover.points` and
`report.regular.unresolved_boxes` to `cover.unresolved_boxes`; then, in
order:

1. `!report.residue.is_empty() || !report.tangential_crossings.is_empty()
   || !report.degenerate.is_empty()` -> `ContactReductionDeferred` (same
   refusal as today; the certified points stay in the report for the
   Boundary Rewrite stage that will consume them);
2. the existing `!cover.unresolved_boxes.is_empty()` check -> unchanged;
3. contacts = one `ContactRecord { dimension: Point0, kind: Tangency,
   locus: ContactLocus::Point(p) }` per `report.tangencies` entry (discovery
   order), followed by the existing `Arc1`/`Transverse`/
   `ValidatedBranchCover` record when `cover.points` is non-empty - reuse
   the existing construction, now with the merged cover;
4. the returned certificate is the `singular_events` cert (actual
   `budget_left`); when the singular path did not run, the existing
   `cover_branch` cert is returned unchanged.

`tau` stays `width / TAU_DIVISOR`. `cover_branch`'s own early-exit behavior
is untouched.

### 5. Determinism and dedup

LIFO worklist, widest-axis bisection ties toward the lowest axis index,
cells processed in the order given, orientation fixed as (f1, f2). Dedup
within `tangencies`, within `tangential_crossings`, and within `degenerate`:
two points whose componentwise max-norm distance is `<= EVENT_RESIDUAL` are
one event, first in discovery order wins; `EVENT_RESIDUAL = 1.0e-6` with a
same-line `// H-3` comment (unit-scale event-identity residual, not a
length).

### 6. Budget and certificates

Caller-owned, captured once at `singular_events` entry: refinement
bisections and every internal `cover_branch`/`krawczyk` spend it; exhaustion
propagates as `NumericallyUnresolved` with `spent = initial - remaining`
and `UnresolvedWitness::KrawczykIndeterminate` (the house pattern). No
private budget, no replenishment. The success certificate is `Method::
Interval`, empty props, actual `budget_left`, unbounded margin/modulus -
exactly `gff`'s `certificate(budget)` shape.

### 7. Machine-checked witnesses (all verified exactly in f64)

1. **External tangency** (isolated): unit cylinder at the origin vs sphere
   center `(2,0,0)` radius 1. At `p = (1,0,0)`: `f1 = f2 = 0` exactly;
   minors `(4yz, -4xz, 8y)` all contain zero on the box
   `x in [0.9,1.1], y in [-0.1,0.1], z in [-0.1,0.1]`; `lam* = 1` exactly;
   the envelope is `B2/delta = 2.2181/1.8 = 1.2323` (contains 1); the
   Lagrange Jacobian determinant at the root is 32 (nonsingular); the
   restricted Hessian is `diag(4, 2)` - definite positive.
2. **Internal tangency** (saddle): unit cylinder at the origin vs sphere
   center `(1,0,0)` radius 2. At `p = (-1,0,0)`: `f1 = f2 = 0` exactly;
   minors `(4yz, -4xz, 4y)` all contain zero on the box
   `x in [-1.1,-0.9], y in [-0.1,0.1], z in [-0.1,0.1]`; `lam* = -2`
   exactly; the restricted Hessian is `diag(-2, 2)`, `det = -4` -
   indefinite: the contact locus self-crosses (the exit curve pinches
   through itself at the internal tangency).
3. **Degenerate apex**: cone apex `(1,0,0)`, `half_angle = (3/4).atan()`,
   vs unit cylinder at the origin: the apex is exactly on the wall
   (`f_cyl(apex) = 0`), and two contact branches cross through it.
4. **Broad regular domain**: unit cylinder at the origin vs sphere center
   `(0.5,0,0)` radius 2, box `x in [-1.5,1.5], y in [-1.5,1.5], z in
   [-2,2]`: minors `(4yz, -4xz, 2y)` = `[-12,12]`, `[-12,12]`, `[-3,3]` -
   all contain zero, NO domain chart; the box contains no tangency
   (candidates `(+-1,0,0)` give `f2 = -3.75` and `-1.75`), and the crossing
   `(1, 0, sqrt(3.75))` lies inside it (`f1 = f2 = 0` exactly).

Before editing, machine-check these identities with the exact formulas
(python or a scratch Rust test) and record the values in RESULT notes.

## Tests required

All in `singular.rs`'s test module unless stated; carriers constructed via
the matched-`Outcome` helper pattern used in `gff.rs` tests. Budgets
generous (mirror `gff.rs` test budgets); `tau` scale-relative per test box
(widest axis / 128).

1. `singular_refines_regular_cover_from_broad_singular_domain`: witness 4's
   box directly against `singular_events`. Assert `regular.points` non-empty
   and all four other lists empty.
2. `singular_certifies_isolated_external_tangency`: witness 1's box.
   Assert `tangencies` has exactly one entry within a named unit-scale
   residual of `(1,0,0)`, and all other lists empty except `regular` (its
   `points` may be empty - the tangency is the only contact there).
3. `singular_classifies_internal_tangency_as_crossing`: witness 2's box.
   Assert `tangential_crossings` has exactly one entry within a named
   residual of `(-1,0,0)`, `tangencies` and `degenerate` empty, and
   `regular.points` NON-empty (the crossing branches chart-certify around
   the pinch).
4. `singular_certifies_degenerate_apex_contact`: witness 3's box (e.g.
   `x in [0.9,1.1], y in [-0.1,0.1], z in [-0.1,0.1]`). Assert `degenerate`
   contains exactly `(1,0,0)`, `tangencies` and `tangential_crossings`
   empty, `regular.points` non-empty.
5. `singular_events_are_order_insensitive`: witnesses 1 and 2 with the two
   field orders swapped: same classified lists (points matched
   order-insensitively within a named residual; same list KINDS non-empty).
6. `contact_ff_internal_tangency_pair_stays_deferred` (in `mod.rs`):
   dispatcher-level witness 2 - unit cylinder `u in (PI-0.4, PI+0.4),
   v in (-0.5,0.5)` vs sphere center `(1,0,0)` radius 2 with the same patch
   bounds construction as the existing tangent-sphere test (same direction
   `(1,0,0) - (2,0,0)` from the center, so the same u/v windows). Assert
   `ContactReductionDeferred`.

**Update in place, preserving the name** (session-34 rule: never rename a
pre-existing test):
`contact_ff_offset_tangent_pair_stays_deferred_for_singular_stage` now
asserts the NEW contract: `contact()` returns Ok with exactly one record,
`dimension == Point0`, `kind == Tangency`, locus `Point` within a named
residual of `(1,0,0)`, certificate `Method::Interval`. The name is
historical; the assertions are the contract.

`contact_ff_non_coaxial_curved_pair_refuses_deferred` must keep passing
unchanged (the apex pair still defers - the degenerate arm defers).

Preserve every other pre-existing test function name. H-3 rejects an added
bare `1e-N` unless the same line has a `// H-3` comment.

## Done when

```console
cargo fmt --check -p truck-evidence
cargo clippy -p truck-evidence --all-targets --no-deps
cargo check --locked -p truck-evidence --all-targets
cargo test -p truck-evidence --lib contact --no-fail-fast
bash scripts/kernel-gates.sh <your base commit>
```

Never run bare `cargo test` or a workspace-wide cargo command.

**Commit your work on the current branch** (subject
`contact: singular-event classification stage (BG-SOL-S7-SING-CLASSIFY)`)
**before** writing `RESULT.json`: the verifier measures the committed diff,
and an uncommitted tree reads as an interrupted run.

## Forbidden

Editing outside `write_allow` (in particular `implicit.rs`, `gff.rs`, and
`krawczyk.rs` are read-only for this packet); changing `cover_branch`,
`ImplicitField`, Krawczyk, enclosure, or dispatcher dispatch order; adding
dependencies; changing `BranchCover` public fields; claiming a saddle or
degenerate point is isolated; claiming residue leaves are empty; connecting
points into curves or ordering components; adding Arc1/Region2 singular
locus claims; accepting unresolved boxes silently; adding `#[ignore]`;
loosening a gate; changing the GATE-4 ceiling; renaming or deleting a
pre-existing test.

## Stop conditions

- anchor mismatch -> `ANCHOR_MISMATCH` with observed count;
- witness 1 cannot certify isolated tangency under a healthy budget after
  checking the four-box, envelope, and Jacobian mapping -> `SPEC_GAP` with
  the measured krawczyk outcome;
- witness 2 classifies as isolated (definite) or witness 3 as tangency ->
  `SPEC_GAP` with the measured inertia;
- swapping field orders changes the classified list kinds -> `SPEC_GAP`;
- three consecutive cargo failures with one cause -> `BLOCKED`.

Finish by writing `RESULT.json` in the worktree root, not `loop/results/`.
Record in `notes` the machine-checked witness values (decision 7) and the
deterministic basis `u`, `v` you derived at each classified root.
