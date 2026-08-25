# WORK PACKET BG-SOL-S3-CONTACT — Contact Layer skeleton: stratum vocabulary, C0-C2 identity/overlap, analytic FF dispatch

You are implementing Phase 3 of the solver family: the **Contact Layer** (`contact`
module in `truck-evidence`). This is the first packet of the contact funnel that
the M2 cross-layer gate (the flagship differential test `Extrude(P−Q) ≅
Extrude(P)−Extrude(Q)`) consumes: it establishes the stratum vocabulary and the
dispatcher's two cheapest stages — identity/overlap (C0-C2) and the analytic
FF pairs (which already exist, plan §3.3). Everything you need is in this
document. **Do not read any other spec file** — this packet is self-contained.

```json
{"id":"BG-SOL-S3-CONTACT","status":"DONE","contracts":["BG-SOL-S3-CONTACT"],
 "tests_added":5,"deviations":[],"disagreements":[],
 "baseline_failures":[],"notes":"free text"}
```

Fill `deviations`, `disagreements` and `baseline_failures` as arrays, empty if
empty. **If anything below contradicts what you find in the code, say so in
`disagreements` rather than making the code match the packet.**

```yaml
id:          BG-SOL-S3-CONTACT
class:       design
crates:      [truck-evidence, truck-geometry, truck-base]
write_allow:
  - vendor/truck/truck-evidence/src/contact.rs
  - vendor/truck/truck-base/src/evidence.rs
read_allow:
  - vendor/truck/truck-geometry/src/recognize.rs
  - vendor/truck/truck-evidence/src/analytic/mod.rs
  - vendor/truck/truck-evidence/src/analytic/plane_plane.rs
  - vendor/truck/truck-evidence/src/analytic/plane_sphere.rs
  - vendor/truck/truck-evidence/src/analytic/sphere_sphere.rs
  - vendor/truck/truck-evidence/src/analytic/plane_cylinder.rs
  - vendor/truck/truck-evidence/src/analytic/plane_cone.rs
  - vendor/truck/truck-base/src/contact.rs
tests_required:
  - contact_ff_plane_plane_transverse_returns_analytic_line
  - contact_ff_coincident_planes_returns_coincident
  - contact_ff_plane_cylinder_returns_analytic
  - contact_ff_spline_surface_refuses
  - contact_fe_stratum_refuses_deferred
budget:      {turns: 90, ctx_tokens: 220000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub enum CanonicalSurface' vendor/truck/truck-geometry/src/recognize.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub fn recognize_surface' vendor/truck/truck-geometry/src/recognize.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'pub fn plane_plane' vendor/truck/truck-evidence/src/analytic/plane_plane.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'pub fn plane_cylinder' vendor/truck/truck-evidence/src/analytic/plane_cylinder.rs"}
  - {id: A5, expect: 1, cmd: "grep -c 'pub enum ContactDimension' vendor/truck/truck-base/src/contact.rs"}
  - {id: A6, expect: 1, cmd: "grep -c 'pub struct Budget' vendor/truck/truck-base/src/evidence.rs"}
  - {id: A7, expect: 1, cmd: "grep -c 'pub mod contact' vendor/truck/truck-evidence/src/lib.rs"}
```

## Problem

The flagship differential test `Extrude(P−Q) ≅ Extrude(P)−Extrude(Q)` needs a
3-D Boolean on its RHS (box − cylinder for M1). That Boolean is the Boundary
Rewrite (Phase 4), and the Boundary Rewrite dispatches **stratum contact**:
for every pair of boundary strata (face/face, face/edge, edge/edge) it must
know where they meet and how. `contact(lhs, rhs)` is that oracle. The plan
books it (§4 Phase 3) with a dispatch order: identity/overlap (C0-C2) →
analytic pairs (exists, §3.3) → strata reductions (FE, EE) → general validated
FF → singular event cells → 2-D overlap (last, deliberately delayed).

Your job: the module skeleton, the stratum vocabulary, and the first two
dispatch stages — with an honest, documented refusal for the parts of the
funnel this packet does NOT build.

## Design decisions already made for you

### 1. Module and scaffolding

The module file `vendor/truck/truck-evidence/src/contact.rs` is **already
scaffolded** (empty, with the H-1 deny header and a doc comment). The
`pub mod contact;` declaration is **already in** `vendor/truck/truck-evidence/
src/lib.rs`. Do NOT touch `lib.rs` — it is outside your write set. Fill the
existing `contact.rs`.

`truck-evidence` depends on `truck-base`, `truck-geometry`, `truck-geotrait`.
The types you build on:

- `truck_geometry::recognize::{CanonicalSurface, CanonicalCurve,
  CanonicalCarrier, CanonicalCarrierWitness}` — the structural recognizer
  (P0-REC). `CanonicalSurface` has variants `Plane(Plane)`, `Cylinder(Cylinder)`,
  `Cone(Cone)`, `Sphere(Sphere)`, `Torus(Torus)`, `Placed(Processor<...>)`.
- `truck_evidence::analytic::*` — `AnalyticIntersection`, `ExactCurve`,
  `AnalyticOutcome`, and the pair functions `plane_plane`, `plane_sphere`,
  `sphere_sphere`, `plane_cylinder`, `plane_cone` (all `pub fn (…) ->
  AnalyticOutcome`).
- `truck_base::contact::{ContactDimension, ContactEventKind}` — the shared
  2-D ontology (P0-PRED), reused by the 3-D Contact Layer per plan §2.
- `truck_base::evidence::{Outcome, Certified, Refusal, Certificate, Method,
  Budget, EnvelopeCase, Prop}` (re-exported by `truck_evidence` as `outcome`).

### 2. The signature (anchored against the landed API)

```rust
/// One boundary stratum of a solid, lifted to the canonical-carrier level.
/// The "bounded" is a parameter-space box/interval on the canonical carrier;
/// trimming to the actual face boundary (wires) is a later strata-reduction
/// refinement, not this packet.
pub enum BoundedStratum {
    Face { surface: CanonicalSurface, u_range: (f64, f64), v_range: (f64, f64) },
    Edge { curve: CanonicalCurve, t_range: (f64, f64) },
    Vertex { point: Point3 },
}

/// The certified contact between one stratum pair.
pub struct ContactComplex {
    pub contacts: Vec<ContactRecord>,
}

pub struct ContactRecord {
    pub dimension: ContactDimension,
    pub kind: ContactEventKind,
    pub locus: ContactLocus,
}

pub enum ContactLocus {
    /// C1/C2 identity/overlap: the two strata share a canonical carrier.
    Coincident,
    /// An exactly-solved analytic FF pair.
    Analytic(AnalyticIntersection),
}

pub fn contact(lhs: &BoundedStratum, rhs: &BoundedStratum, budget: &mut Budget)
    -> Outcome<ContactComplex>;
```

**Why the plan's §4 Phase 3 signature is amended (record it):** the plan books
`contact(lhs: BoundedStratum, rhs: BoundedStratum, budget: &mut Budget)` by
value. Take `&BoundedStratum` instead: the strata carry `CanonicalSurface`
(which contains `Processor<Box<...>>` and is not `Copy`), and the dispatcher
must inspect both strata before constructing anything; references avoid a
meaningless clone and keep the call cheap when the Boundary Rewrite iterates
every pair. `ContactComplex` is returned by value as booked. Note this
amendment in your RESULT.json notes.

### 3. The dispatcher — exactly this order, nothing more

`contact()` must dispatch in this order and **stop at the first decided
stage**:

1. **C0-C2 identity/overlap.** If both strata are `Face` and
   `lhs.surface == rhs.surface` (CanonicalSurface derives `PartialEq`), the
   contact is `ContactLocus::Coincident`, `dimension = Region2`,
   `kind = IdenticalCarrier`. Same for `Edge` (equal `CanonicalCurve`) →
   `Arc1` / `IdenticalCarrier`. (C0 provenance identity is topology-side and
   cannot be expressed at the canonical-carrier level — note this.)

2. **FF analytic.** Both strata are `Face` and both surfaces are canonical
   analytic carriers. Match the ordered pair against this table and call the
   existing function:
   - `Plane × Plane` → `plane_plane`
   - `Plane × Sphere` / `Sphere × Plane` → `plane_sphere`
   - `Sphere × Sphere` → `sphere_sphere`
   - `Plane × Cylinder` / `Cylinder × Plane` → `plane_cylinder`
   - `Plane × Cone` / `Cone × Plane` → `plane_cone`
   Wrap the returned `AnalyticIntersection` as
   `ContactLocus::Analytic(...)`. Map the analytic arm to the ontology:
   `Curve`/`TwoCurves` → `Arc1` / `Transverse`; `Tangent*` → `Arc1` /
   `Tangency`; `Parallel`/`Empty` → the contact is decided as no contact —
   return an empty `ContactComplex`; `Coincident` → `Region2` /
   `CoincidentInterval`.

3. **Everything else → an honest, documented refusal.** This is the deferred
   funnel. Add ONE new arm to `truck_base::evidence::EnvelopeCase` (in
   `vendor/truck/truck-base/src/evidence.rs`, your write set):
   `ContactReductionDeferred` with a doc comment: "a stratum pair whose contact
   reduction (FE, EE, general validated FF, singular event cells, or 2-D
   overlap) is not yet implemented in the Contact Layer (plan §4 Phase 3)". Use
   it for: any pair with a `Face` and an `Edge`/`Vertex`, any `Edge`/`Vertex`
   pair, any face whose canonical surface is `Torus` or `Placed`, and any
   `CanonicalCarrierWitness::Unrecognized` carrier. Return
   `Err(Refusal::UnsupportedEnvelope(EnvelopeCase::ContactReductionDeferred))`.

Do NOT implement FE/EE strata reductions, general validated FF, singular event
cells, or 2-D overlap. Do NOT touch the analytic pair modules. Do NOT attempt
the topology-wide PC threading. The refusal in stage 3 IS the deliverable for
those cases — it types the funnel's boundary so the next packets fill in the
stages they own.

### 4. Certificate construction

Follow the analytic-pair precedent exactly: an explicit field-by-field
`Certificate { props, method, budget_left, margin, modulus }`, never a
convenience constructor. For the analytic stage, reuse the pattern from
`plane_plane.rs` (method `Method::Exact` when the analytic function returned
`Ok`, since the analytic pairs are exact; `Method::Interval` never occurs
here). For the identity stage, `Method::Exact`. Include `Prop::AnalyticCarrier`
in `props` when both carriers are canonical analytic. Spend nothing from
`budget` in this packet — the analytic pairs do not take one, and no
subdivision happens here; pass the untouched budget into the certificate's
`budget_left`.

### 5. Tests (all in `contact.rs`, in a `#[cfg(test)] mod tests`)

House rule: GATE-1 requires `#![deny(clippy::unwrap_used)]` on every new test
file — the module header already carries it. Write `#[allow(clippy::unwrap_used,
clippy::expect_used)]` on the test module exactly as the analytic modules do.

Build canonical carriers by calling `recognize_surface(&Surface::Plane(plane))`
etc. (the `ExactCanonical` arm), or construct `CanonicalSurface::Plane(plane)`
directly. Build analytic witnesses the way `plane_plane.rs` tests do (dyadic
points). The five required tests:

1. `contact_ff_plane_plane_transverse_returns_analytic_line` — two planes
   crossing at a dyadic line (e.g. `z = 0` and a plane through the origin with
   a non-parallel normal) → one record, `locus = Analytic(Curve(Line))`,
   `dimension = Arc1`, `kind = Transverse`.
2. `contact_ff_coincident_planes_returns_coincident` — `Plane::new` twice from
   different dyadic triples lying on the same plane → stage 1 fires,
   `locus = Coincident`, `dimension = Region2`, `kind = IdenticalCarrier`.
3. `contact_ff_plane_cylinder_returns_analytic` — a `Plane` cutting a
   `Cylinder` transversally (axis along z, plane `z = 0` through the cylinder's
   center, dyadic) → `locus = Analytic(...)`, record `dimension = Arc1`.
4. `contact_ff_spline_surface_refuses` — a `CanonicalCarrierWitness::Unrecognized`
   carrier (construct a `BSplineSurface` `Surface` and pass its
   `recognize_surface` result, or build the `Unrecognized` arm directly) →
   `Err(Refusal::UnsupportedEnvelope(EnvelopeCase::ContactReductionDeferred))`.
5. `contact_fe_stratum_refuses_deferred` — a `Face` stratum paired with an
   `Edge` stratum (the FE case) → the same `ContactReductionDeferred` refusal.

Also assert `BoundedStratum` is `Clone + Debug + PartialEq` where the carriers
allow it (they do), so future packets can store strata.

## Done-when gates

```
cargo fmt --check -p truck-evidence
cargo clippy -p truck-evidence --all-targets --no-deps
cargo test -p truck-evidence --lib --tests --no-fail-fast
cargo check --locked -p truck-evidence --all-targets
cargo check --locked -p truck-base --all-targets
bash scripts/kernel-gates.sh <your base commit>
```

Never run a bare `cargo test`. Never run `cargo check --workspace` — it
exhausts disk on a shared machine.

## H-3 / GATE-4

GATE-2 rejects added lines carrying bare `1e-N` literals unless the line ends
with `// H-3`. Float tolerances in test assertions must carry `// H-3` on the
same line. This packet adds NO `unscaled_legacy()` calls; do not touch
`scripts/unscaled_legacy_ceiling.txt`.

## Forbidden

Editing any file outside `write_allow`. Editing `truck-evidence/src/lib.rs`,
any analytic pair module, or any topology/modelling file. Adding pcurves.
Implementing the deferred funnel stages (FE/EE/general-FF/singular/2-D
overlap). Adding `#[ignore]`. Changing the GATE-4 ceiling. Running
`cargo check --workspace` / `cargo build --workspace`.

## Stop conditions

- an anchor count differs → `ANCHOR_MISMATCH`, naming the file and what you saw
- three consecutive failed `cargo` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. Record in
`notes`: the two signature amendments (`&BoundedStratum`, the new
`EnvelopeCase` arm), the stage-3 refusal coverage (which stratum pairs hit it),
and your read of whether any in-scope representation was infeasible.

Commit on the current branch with subject
`feat(evidence): Contact Layer skeleton — stratum vocabulary + C0-C2 + analytic FF dispatch (BG-SOL-S3-CONTACT)`.
