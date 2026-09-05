# Certified Carrier Lift (CL) build spec

**Status:** authored 2026-09-05 from the text-to-cad corpus audit (owner
directive: mechanical/exact-fix items grouped and provisioned into the
running loop). Evidence base: the audit greps recorded in
`TRUCK123D_PY_BRIDGE_SPEC.md` §8 and the loop session record. The
theory-class gaps (tangency closure, measure-zero fragment calculus) are
NOT here — they are stated to the owner separately.

Every tree claim below was verified by command on 2026-09-05; re-derive
before quoting in a packet.

## 1. The substrate facts (measured)

- `EnclosureSurface` impls: Cone, Cylinder, Plane, Sphere, Torus only
  (+ Graph/HermiteSurface/DoubleCover/SpherePatch decorators).
  **No BSplineSurface/NurbsSurface. No SpineFrameSweep.**
  (`truck-evidence/src/enclosure.rs` + carrier files.)
- The contact layer's own test pins the spline refusal:
  `contact_ff_spline_surface_refuses` (`contact/mod.rs:1713`) — spline
  carriers are structurally `Unrecognized`. Lifting it is a deliberate
  envelope change, owner-noted, with the test updated to assert the NEW
  certified path (never deleted).
- The general-pair engine is landed and stranded:
  `truck-certified/src/ssi.rs` (rational tensor-Bernstein patches,
  `SquareSystem3`, Krawczyk unique-root certificate, `ssi_trace`
  continuation) has zero funnel callers.
- The certified loft already emits the corpus's carrier:
  `loft_sections(&[BSplineCurve<Vector4>], ...)` → `BSplineSurface`.
- Zero `SpineFrameSurface` arms anywhere in `truck-evidence` — sweep×sweep
  has no dispatch path.
- Blend/offset are carrier-agnostic (they walk certified branch charts);
  the carrier restriction enters only through the solver's recognized
  list — so every item below is a lift at the admission/dispatch layer.

## 2. Packets

| Packet | Class | Content | Write set | Depends |
|---|---|---|---|---|
| `CL-000-SPLINE-ADMIT` | mechanical | (a) Bézier patch decomposition of `BSplineSurface`/`NurbsSurface` by exact knot insertion, non-rational → weight-1; (b) admission gates (finite, bidegree bounds, ragged refusal) feeding `ssi.rs`'s patch form; (c) `impl EnclosureSurface for BSplineSurface` — derivative is a degree-(k−1) B-spline with exact control points; bounds = per-patch Bernstein hull (landed `hull_bernstein_2d` composition) | `truck-evidence/src/enclosure.rs` (additive impls), `truck-certified/src/patch_admit.rs` (new), `truck-certified/src/lib.rs` (one mod line) | — |
| `CL-001-SPLINE-LIFT` | mechanical | The contact dispatch arm: spline×analytic pairs through `SquareSystem3`/Krawczyk (spline side admitted by CL-000, analytic side by landed enclosures); the pinned refusal test updated to assert the certified path; typed `Unresolved` with κ/cell/slope witness where the engine cannot certify | `truck-evidence/src/contact/mod.rs` (dispatch arm + the pinned test's expectation), `truck-certified/src/ssi_admit.rs` (new) | CL-000, BIE-006-CLASSIFY |
| `CL-002-SPLINE-ASSEMBLY` | mechanical | Cross-patch branch stitching: per-patch intersection certificates share edge parameterizations; the stitch certificate asserts the shared-edge samples agree (interval bookkeeping); the stitched branch rides BIE-003's `CertifiedImplicitIntersectionCurve` carrier | `truck-certified/src/ssi_trace.rs`, `truck-geometry/src/constructive/intersection_carrier.rs` (additive constructor) | CL-001 |
| `CL-003-SWEEP-ENCLOSURE` | mechanical | `impl EnclosureSurface for SpineFrameSweep`: certified grad bounds over the windowed domain composed from the landed spine-curve + profile-law enclosures (the frame laws are explicit and landed); sweep-side σ_G helper (first-fundamental-form bounds) | `truck-evidence/src/enclosure.rs` (additive), `truck-evidence/src/num/` (σ_G helper if not in enclosure.rs) | — |
| `CL-004-SWEEP-SWEEP` | mechanical | Sweep×sweep dispatch: both sides sweep through the restricted solver (BIE-002's machinery) with CL-003 enclosures on both sides; σ_G composed on both sides | `truck-evidence/src/contact/mod.rs`, `truck-certified/src/construct/bie/ssi4.rs` (additive carrier list) | CL-003, BIE-006-CLASSIFY |
| `CL-005-EXACT-CONTACT` | mechanical | Certified decisions for the MEASURED closed set of deferrals: butt-join coplanar union (the 10-split-face record), exact-footprint halfspace (`Contradictory(FragmentInsideOther)`), each a pre-decided case certificate asserted by battery rows; NOT the general calculus (that is theory-class, stated separately) | `truck-shapeops/src/boolean/mod.rs`, `truck-shapeops/src/boolean/assemble.rs` | BIE-006-CLASSIFY |

## 3. Dependency graph and urgency

```text
CL-000 ─→ CL-001 ─→ CL-002
CL-003 ─→ CL-004            (CL-003 is URGENT: the BIE battery certifies
BIE-006 ─┴→ CL-001/004/005   real sweep pairs; no sweep-side enclosures
                             exist today — BIE-002's landed solver assumed
                             them. Pre-empt the wall.)
```

CL-003 and CL-000 dispatch immediately (no deps, disjoint write sets).
CL-001/004/005 serialize behind BIE-006 because `contact/mod.rs` is the
program's hot file (one writer at a time, the booked rule).

## 4. Gates

- V5 identity guard: every landed canonical×canonical answer stays
  bit-identical; the only behavior change is where the pinned refusal
  test explicitly books the new certified path.
- Typed outcomes only: `Unresolved` carries κ/cell/slope; zero new
  `Refusal` arms.
- One-verify amendment: scoped checks between merges, one battery at the
  combined BIE+CL integrated HEAD.
- All cargo through the cargoq queue; scoped commands only.

## 5. What this buys (tiered)

- CL-000+CL-003: the admission layer exists; the BIE battery can certify
  real sweep pairs.
- CL-001+CL-002: spline-carrier booleans certify → the corpus's primary
  carrier lifts.
- CL-004: sweep×sweep (monocoque×sidepod class) certifies.
- CL-005: exactly-seated assemblies stop refusing.
- Still open after all of it: tangency closure and the general
  measure-zero calculus (theory-class) bound the Unresolved RATE on
  pathological parts — the typed refusals remain the honest answer there.
