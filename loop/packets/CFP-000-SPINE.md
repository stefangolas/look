# WORK PACKET CFP-000-SPINE — contact fast-path shim (shapes + fixture kit + gates)

You are creating the spine of the Contact Fast-Path (CFP) program. Everything
you need is in this document and `docs/CONTACT_FAST_PATH_BUILD_SPEC.md`. Do
not read other spec files. If something you need is genuinely missing, that is
a SPEC_GAP (see "Stop conditions"): you stop and report, you do not research
it.

```yaml
id:          CFP-000-SPINE
contract:    [CFP-000-SPINE]
class:       design
crates:      [truck-certified]
depends_on:  []
write_allow:
  - vendor/truck/truck-certified/src/cfp/mod.rs
  - vendor/truck/truck-certified/src/cfp/spine.rs
  - vendor/truck/truck-certified/src/cfp/fixtures.rs
  - vendor/truck/truck-certified/src/lib.rs
  - docs/CERTIFICATE_MAPPING.md
read_allow:
  - docs/CONTACT_FAST_PATH_BUILD_SPEC.md
  - docs/defects/DSC-BOUNDARY-SAMPLE-EXTENT-001.md
  - docs/defects/NUM-SPLINE-ENCLOSURE-CONVERGENCE-001.md
  - vendor/truck/truck-certified/src/ssi_types.rs
  - vendor/truck/truck-certified/src/formal/exact.rs
  - vendor/truck/truck-certified/src/patch_admit.rs
  - vendor/truck/truck-base/src/evidence.rs
tests_required:
  - localizable_refusal_carries_stratum_pair
  - instrument_schema_round_trip
  - spline_entry_dispatch_method_signature_freezes
  - fc0_defect_counterexample_data_admits
  - fc2_enclosure_battery_data_admits
  - fc3_separation_ground_truths_admit
  - fc4_elevation_trap_twin_data_admits
budget:      {turns: 80, ctx_tokens: 180000}
```

## Problem

The CFP program (booking `docs/CONTACT_FAST_PATH_BUILD_SPEC.md`) corrects two
live enclosure defects (BG-ENC-001 at the lift screen, BG-ENC-002 on spline
carriers) and lands the fast-path program on top. This packet is the spine:
it freezes the shapes, fixture kit, gate definitions, and one-writer map
BEFORE any geometry lands. D-shim discipline verbatim from `ssi_types.rs`:
types and refusing constructors only — nothing here evaluates, solves,
isolates, or certifies numerically. Five later packets build against this
module and never restate it.

## Scope decisions — pre-made, do not relitigate

1. **D-shim discipline**: any method that would evaluate numerically refuses
   (`InvalidInput`-shaped). The module doc states this verbatim.
2. **SFC is a type-level split**: the search/certificate pair is two types —
   `FloatHint<T>` (a float search result carrying NO evidence status, never
   constructible into an evidence position) and the exact certificate types.
   A `FloatHint` cannot implement the evidence traits. This makes
   "search in floats, certify exactly" unviolatable by construction.
3. **Localizable refusals**: the refusal wrapper is
   `LocalizedRefusal { cause, stratum_pair: (StratumSide, usize, StratumSide, usize) }`,
   `StratumSide::{A, B}`. It wraps landed causes verbatim (zero new
   top-level evidence kinds); re-attribution to faces happens in
   `truck-shapeops` (CFP-001), not here.
4. **`SplineSsiEntry` slot resolution decided HERE**: the trait grows ONE
   dispatch method
   `fn dispatch_pair(&self, pair: &PairDescriptor, box_: &Box4) -> Outcome<...>`
   and the single registered entry routes internally by pair descriptor.
   Registration order becomes irrelevant. CFP-004 executes this; the
   signature freezes here.
5. **Instrument schema frozen here as DATA**: counter names
   (`knot_span_count`, `stage5_pair_class`, `composed_bidegree`,
   `cells_visited`, `distinct_side_boxes`, `unresolved_provenance`) and the
   JSON shape. Each crate implements its own local counter struct with these
   field names (F1 forbids a shared implementation); the spine freezes the
   names and the JSON key order so per-crate outputs concatenate.
6. **Gate definitions frozen here as docs + assertion helpers**: V5-pair
   (untouched paths), V5-boolean (monotone-diff adjudication: a diff is
   accepted iff the new event set is a SUPERSET of the old on the same
   inputs), and the BG-ENC-002 convergence assertion helper's signature.
7. **Zero new top-level evidence kinds.** Mapping rows in
   `docs/CERTIFICATE_MAPPING.md` section C (one row per new Method tag:
   `ImplicitReduction`, `ConeCertificate`).

## Shapes to freeze (in `cfp/spine.rs`, each with a refusing constructor)

`FloatHint<T>`, `LocalizedRefusal` + `StratumSide`, `PairDescriptor`
(carrier-class pair + both parameter boxes), `InstrumentCounters` (schema
data), `ConeVerdict` (`LoopFree | TransversalByProof | AxisFixed{axis, margin}`
— consumed by CFP-006, produced behind a pending refusal until then), the
`SplineSsiEntry::dispatch_pair` signature (declared as a trait-method
obligation on the landed trait's extension — declare the target here; the
impl lands in `contact/mod.rs` in CFP-004; a one-line forward-declaration
comment is enough, no cross-crate dependency), and the V5-boolean adjudication
record shape.

## Fixture kit (`cfp/fixtures.rs`; spec §2, normative)

Each fixture freezes DATA + ground-truth records, machine-checked at
admission. Evaluation-asserting tests land in the implementing packets
(CFP-001/003/004/006) against this data — that is the parallelism contract:
wave-B/C workers consume these constants, never sibling production code.

- **F-C0**: sphere-cap × plane-slab data: cap boundary circle above the slab,
  apex as the contact region; ground truth: `sampled_aabb ∩ sampled_aabb = ∅`
  AND `true images touch` (both facts stated as data records).
- **F-C1**: the parameter-space twin: a trim whose parameter extent is not
  spanned by its boundary polygon; ground truth: `boundary_uv_hull ⊊ true_uv_extent`.
- **F-C2**: the enclosure battery: three nested box triples `(B₁ ⊃ B₂ ⊃ B₃)`
  on a fixed bicubic net; ground truth: `width(enclose(B₁)) ≥
  width(enclose(B₂)) ≥ width(enclose(B₃))` AND `width(enclose(B₃)) <
  width(enclose(B₁))` (strict decrease somewhere — the BG-ENC-002 guard) AND
  monotonicity `B ⊆ B′ ⇒ enclose(B) ⊆ enclose(B′)`.
- **F-C3**: Theorem 3 ground truths: two control-net pairs with a known
  separating direction `λ` (separated: `min λ·P¹ > max λ·P²`, exact integer
  coordinates) and a known touching pair.
- **F-C4**: Theorem 4 ground truths: plane×bicubic with known `h` coefficients
  (signed control-point distances) and known critical point; quadric×spline
  record; the **elevation-trap twin**: a constructed `h` whose non-elevated
  `∇h` array pairing gives a DIFFERENT hull verdict than the elevated one
  (data + both expected verdicts).
- **F-C5**: span-BVH ground truth: a 20×30-span decomposition record with
  exactly one contact span pair and its expected dyadic prune count.
- **F-C6**: stagnation fixture: a near-tangency pair record whose per-box
  cones stay overlapping at three successive depths; expected routing:
  cascade, verdict `A1Node`.
- **F-C7**: F1 cross-check data: randomized (seeded, fixed seed) net + box
  pairs; the evidence-side and certified-side sub-box hulls must agree on
  these exact inputs (the consuming packets copy these constants; agreement
  tests land with the implementations).

Fixture data copied from landed sources is copied as constants, read-only —
never import a test module.

## Anchors — base-relative, drift-tolerant (the CTE r1 lesson: never anchor a
count on a shared-file total; anchor on the packet-owned delta)

Re-check on your branch:

| id | file | pattern | expect |
|---|---|---|---|
| A1 | `truck-certified/src/patch_admit.rs` | `pub struct SplinePatchStack` | 1 |
| A2 | `truck-certified/src/lib.rs` | `pub mod cfp` | **0 before, 1 after your change** |
| A3 | `truck-evidence/src/enclosure.rs` | `impl EnclosureSurface for BSplineSurface` | 1 (read-only anchor) |
| A4 | `docs/defects/NUM-SPLINE-ENCLOSURE-CONVERGENCE-001.md` | `NUM-SPLINE-ENCLOSURE-CONVERGENCE-001` | ≥1 (read-only anchor) |

A2 is deliberately 0 at base: the `cfp` module does not exist yet — you create
it. If it is already present, another writer landed first: STOP,
`ANCHOR_MISMATCH`.

## House rules

- **H-1** No `unwrap`, `expect`, `panic!`, `unimplemented!`, `todo!`, or
  out-of-range indexing reachable from geometry.
- **H-2** Fallible operations return `Result<T, named>`, never a bare enum
  without a named cause.
- **H-3** No absolute constants in predicates; test epsilons carry `// H-3`
  on the SAME line as the literal.
- **Determinism**: identical ordered input → identical verdicts; no output
  ordering from hash iteration.
- **All cargo invocations go through the queue (the `cargo` on PATH IS the
  queue shim). Do not invoke cargo by absolute path; do not unset the shim.**
- Never run a bare `cargo test` — use the scoped commands below.

## Tests required

Named `#[test]` fns; the verifier checks the names appear in your diff:

1. `localizable_refusal_carries_stratum_pair`
2. `instrument_schema_round_trip` — counters serialize to the frozen JSON key
   order and back.
3. `spline_entry_dispatch_method_signature_freezes` — compile-time shape
   assertion of decision 4.
4. `fc0_defect_counterexample_data_admits`
5. `fc2_enclosure_battery_data_admits`
6. `fc3_separation_ground_truths_admit`
7. `fc4_elevation_trap_twin_data_admits`

## Done when — run these, all must pass

```
cargo fmt --check -p truck-certified
cargo clippy -p truck-certified --all-targets -- -D warnings
cargo test -p truck-certified --lib cfp
cargo check -p truck-evidence
```

Send cargo output to a file and read the tail.

## Forbidden

Editing any file outside `write_allow` — especially `src/ssi.rs`,
`ssi_types.rs`, `tangency/**`, `construct/**`, anything under `truck-evidence/`
or `truck-shapeops/`, `Cargo.lock`. Implementing any numeric evaluator in this
packet (later packets' work). Adding `#[ignore]`. Adding `#[allow]` without a
justification comment on the same line. Committing to `main`.

## Stop conditions

- any anchor count differs → `ANCHOR_MISMATCH`
- a fixture ground truth cannot be stated without fabricating the certificate
  → `SPEC_GAP`
- three consecutive failed `cargo test` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` at the WORKTREE ROOT (then COMMIT first)

**COMMIT BEFORE writing `RESULT.json`.** Then write `RESULT.json` at the root
of your worktree.

```json
{"id":"CFP-000-SPINE","status":"DONE","contracts":["CFP-000-SPINE"],
 "tests_added":7,"anchors_verified":{"A1":1,"A2":1,"A3":1,"A4":1},
 "notes":"cfp module frozen: shapes, fixture kit F-C0..F-C7, gate definitions, instrument schema, slot decision; mapping rows added; any contract deviation stated here"}
```
