# CC-DEF-INTERPOLE — NUM-INTERPOLE-OVERSHOOT-001 (typed admission for try_interpole)

Defect record: `docs/defects/NUM-INTERPOLE-OVERSHOOT-001.md` (normative).
`try_interpole` (truck-geometry `nurbs/bspcurve.rs:271`) solves the
collocation system with plain-f64 no-pivot `gaussian_elimination`; on
moderate data counts (n = 51→257) the control points blow up to 10⁹–10¹⁰ ×
data extent BETWEEN samples — exact at the data, catastrophic between — and
`facet_sweep` ships the distorted mesh under a clean verdict. The STRUCTURAL
fix is landed (CC-001's certified banded solve + CC-010's de Boor averaging
in truck-certified); this packet fixes the STANDING API: a caller cannot
distinguish a good interpolation from a wildly oscillating one.

```yaml
id:          CC-DEF-INTERPOLE
contract:    [CC-DEF-INTERPOLE]
class:       mechanical
crates:      [truck-geometry]
depends_on:  []
write_allow:
  - vendor/truck/truck-geometry/src/nurbs/bspcurve.rs
  - vendor/truck/truck-geometry/src/nurbs/mod.rs
  - vendor/truck/truck-geometry/src/nurbs/knot_vec.rs
  - vendor/truck/truck-geometry/tests/constructive_interpole_bounds.rs
read_allow:
  - docs/defects/NUM-INTERPOLE-OVERSHOOT-001.md
  - vendor/truck/truck-geometry/src/nurbs
budget:      {turns: 16, ctx_tokens: 70000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub fn try_interpole' vendor/truck/truck-geometry/src/nurbs/bspcurve.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'GaussianEliminationFailure' vendor/truck/truck-geometry/src/errors.rs"}
tests_required:
  - sw_violating_knot_vector_refused_typed
  - averaged_knots_helper_matches_de_boor_definition
  - interpolation_stays_within_data_bounds_on_the_probe_fixture
  - existing_interpole_callers_behavior_unchanged_on_valid_inputs
```

Section 1: the SW gate — after solving, VALIDATE before returning: the
delivered control points are checked against a BOUNDEDNESS criterion, and
the knot vector against the Schoenberg–Whitney condition
(`M_{j,q}(v_k) != 0` for every station). Pre-made decisions: (a) a knot
vector violating SW → `Err(Error::InterpolationNotSwVerified { at })` (NEW
typed variant in `errors.rs`, alongside `GaussianEliminationFailure` (A2) —
never a panic, never a silent accept); (b) a solve whose control-point
extent exceeds `BOUND_FACTOR ×` the data extent (new named const, value
1e3, justified in the module doc from the record's scaling table: honest
interpolants sit at O(1)× data extent, the defect at 10⁹×) → the same typed
refusal — an interpolant that wildly overshoots is not delivered as
success; (c) NO pivoting, NO change to the solve itself: valid inputs get
bit-identical results (the landed-behavior guard).

Section 2: the helper — `pub fn averaged_interpolation_knots(stations:
&[f64], degree: usize) -> KnotVec` in `knot_vec.rs`: de Boor averaging
`ξ_{j+q} = (1/q) Σ_{r=j}^{j+q−1} v_r` with clamped ends repeated q+1 — the
pure-math port of the landed certified version (truck-geometry cannot
depend on truck-certified; the two implementations must agree, and a test
asserts agreement on a shared fixture against the certified one is the
side-session's business — here the DEFINITION is the ground truth). Doc
comment points interpolant users at it as the default knot choice.

Section 3: the probe fixture — test 3 reproduces the record's scaling table
in miniature: bounded data points (unit-scale), the BAD knot choice that
triggered the defect, assert the typed refusal; then the same data with
`averaged_interpolation_knots`, assert between-sample evaluation stays
within `BOUND_FACTOR`-style bounds (H-3 opt-outs). The showcases probes
(`knot_probe.rs`, `mesh_probe.rs`) and
`facet_mesh_stays_within_path_bounds` are the external oracles — the
side-session owns their ID-named successors
(`num_interpole_overshoot_001_*`); do not author or rename them.

House rules: **H-1: no `unwrap`/`expect`/`panic!` in shipped code, no
module-level `allow`.** **H-3: float comparisons in tests take the `// H-3`
opt-out ON THE SAME LINE.** **All cargo invocations go through the queue
(the `cargo` on PATH IS the queue shim). Do not invoke cargo by absolute
path; do not unset the shim.** Scoped checks: `cargo check -p
truck-geometry`, `cargo test -p truck-geometry --lib`,
`cargo test -p truck-geometry --test constructive_interpole_bounds`. COMMIT
BEFORE writing RESULT.json AT THE WORKTREE ROOT.

Stop conditions: (1) existing valid-input behavior must be bit-identical —
if the SW gate refuses an input the current callers pass today (e.g.
`facet_sweep`'s interpole use), that is the DEFECT's own admission surface:
record the input and the refusal in RESULT notes rather than weakening the
gate; (2) `KnotVec`'s landed ops are reused (`try_bspline_basis_functions`,
`validate`) — do not reimplement basis evaluation; (3) the certified path
(truck-certified CC-001/CC-010) is the booked structural fix — this packet
does NOT try to make `try_interpole` certified, only honest.
