# BG-CG-007-CERT — realization evidence integration (the unified certificate mapping, implemented)

```yaml
id:          BG-CG-007-CERT
contract:    [BG-CG-007-CERT]
class:       design
crates:      [truck-base, truck-modeling, truck-meshalgo]
depends_on:  [BG-CG-000-CONTRACT, BG-CG-004-FACET, BG-CG-005-LEDGER, BG-CG-006-DIAG]
write_allow:
  - vendor/truck/truck-base/src/evidence.rs
  - vendor/truck/truck-modeling/src/facet_sweep.rs
  - vendor/truck/truck-modeling/tests/facet_sweep_conformance.rs
  - vendor/truck/truck-meshalgo/src/tessellation/realization_evidence.rs
  - vendor/truck/truck-meshalgo/src/tessellation/mod.rs
  - vendor/truck/truck-meshalgo/tests/realization_evidence.rs
read_allow:
  - docs/CONSTRUCTIVE_GEOMETRY_PLAN.md
  - docs/CERTIFICATE_MAPPING.md
  - vendor/truck/truck-base/src/evidence.rs
  - vendor/truck/truck-geometry/src/constructive/mod.rs
  - vendor/truck/truck-geometry/src/constructive/recipe.rs
  - vendor/truck/truck-geometry/src/constructive/errors.rs
  - vendor/truck/truck-modeling/src/facet_sweep.rs
  - vendor/truck/truck-meshalgo/src/tessellation/mod.rs
  - vendor/truck/truck-meshalgo/src/tessellation/triangulation_with_ledger.rs
  - vendor/truck/truck-topology/src/manifold.rs
tests_required:
  - construct_refused_variant_exists_and_carries_no_payload
  - realization_verdict_absorbs_facet_verdict
  - facet_sweep_certified_refuses_with_construct_refused
  - facet_sweep_certified_ok_carries_evidence_and_certificate
  - shared_edge_pairs_empty_on_exact_grid_path
  - ledger_assembly_fills_shared_edge_pairs
  - evidence_method_is_float_not_exact
  - inconclusive_never_becomes_certified
budget:      {turns: 45, ctx_tokens: 120000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub enum EnvelopeCase' vendor/truck/truck-base/src/evidence.rs"}
  - {id: A2, expect: 0, cmd: "grep -c 'ConstructRefused' vendor/truck/truck-base/src/evidence.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'pub enum FacetVerdict' vendor/truck/truck-modeling/src/facet_sweep.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'pub fn facet_sweep' vendor/truck/truck-modeling/src/facet_sweep.rs"}
  - {id: A5, expect: 0, cmd: "grep -ci 'realization_evidence' vendor/truck/truck-meshalgo/src/tessellation/mod.rs"}
  - {id: A6, expect: 1, cmd: "grep -c 'pub fn winding_audit' vendor/truck/truck-modeling/src/facet_sweep.rs"}
  - {id: A7, expect: 0, cmd: "grep -c 'shared_edge_pairs' vendor/truck/truck-modeling/src/facet_sweep.rs"}
```

## What this packet is

The certificate-integration packet of the CG program: it implements the
unified certificate mapping (`docs/CERTIFICATE_MAPPING.md`, sections A and B —
the authoritative table; this packet quotes everything load-bearing) onto the
landed facet backend, the edge-sample ledger, and the existing evidence
algebra. No new validation universe: every new type composes with the landed
`Refusal` / `EnvelopeCase` / `Certificate` / `Method` vocabulary.

**Placement pre-decided (do not relitigate):** all new TYPES live in
`truck-base/src/evidence.rs`. The facet outcome that must carry them is in
truck-modeling; a regular modeling→meshalgo dependency is barred (plan §3.1)
and the BG-S0-001 precedent moved the evidence algebra to truck-base for
exactly this reason. Both modeling and meshalgo already depend on base — zero
new manifest edges. The truck-meshalgo side of this packet is the
*assembly module*: functions that build the evidence from a mesh, a ledger,
and a verdict. The mapping table's placement correction (2026-08-31) records
this; the frozen CG-000 module doc in `constructive/mod.rs` predates it and
stays as written — do not edit `vendor/truck/truck-geometry/**`.

## Section 1 — truck-base: the evidence types (evidence.rs, additive only)

All four additions go beside the existing algebra. Header stays
`#![deny(clippy::unwrap_used)]`-clean; nothing existing moves (V8 identity
guard covers the crate's suites).

```rust
/// The envelope case for constructive-realization refusals (mapping A row 1).
/// CG-007 adds it; every realization entry maps `ConstructError` onto
/// `UnsupportedEnvelope(ConstructRefused)` and rides the details in
/// `RealizationEvidence`.
ConstructRefused,
```

```rust
/// A Copy/Eq-safe projection of a constructive `ConstructError`. base cannot
/// name the error type (geometry depends on base, not vice versa), so the
/// error's identity rides as a tag.
pub struct ConstructErrorSummary {
    pub kind: &'static str,        // "ZeroTangent" | "FrameSingular" | "SpineNotC1"
                                   // | "ProfileCorrespondenceMismatch"
                                   // | "ProfileCollapse" | "NonFinite" | "InvalidInput"
    pub at: Option<f64>,
    pub law: Option<&'static str>, // FrameSingular only
}

/// The three-valued realization verdict (mapping A row 4 / section B).
pub enum RealizationVerdict { CertifiedWithinTolerance, Failed, Inconclusive }

/// Per-realization certificate (mapping A row 2). NOT a widening of
/// FaceValidityCertificate; the same separation doctrine as band_attempts vs
/// cone_band_attempts.
pub struct RealizationCertificate {
    pub method: Method,            // H-6: the facet path computes in floats
    pub max_cell_twist: f64,       // max bilinear-twist deviation over cells
    pub extent: f64,               // the audit's extent (the tolerance scale)
}

/// One shared-edge observation (mapping A row 3). Never a ProvenanceRecord
/// variant (that type is Copy + Eq; this payload carries f64s).
pub struct SharedEdgePairEvidence {
    pub error_a: f64,
    pub error_b: f64,
}

/// The realization evidence record (mapping A row 1). Construct-stage
/// failures predate meshing and never enter MeshedShellOutcome; this is the
/// record that carries them, plus realization-stage facts.
pub struct RealizationEvidence {
    pub construct_error: Option<ConstructErrorSummary>,
    pub certificate: Option<RealizationCertificate>,
    pub shared_edge_pairs: Vec<SharedEdgePairEvidence>,
    pub verdict: RealizationVerdict,
}
```

Derives: `ConstructErrorSummary: Clone, Copy, Debug, PartialEq` (f64 fields
block Eq — that is fine; do NOT derive Eq on f64-carrying types, the CG-002
r1 finding). `RealizationVerdict: Clone, Copy, Debug, PartialEq, Eq`.
`RealizationCertificate: Clone, Debug, PartialEq`. `SharedEdgePairEvidence:
Clone, Debug, PartialEq`. `RealizationEvidence: Clone, Debug, PartialEq`.
Doc-comments on every pub item (crate lints).

## Section 2 — truck-modeling: facet_sweep.rs (additive; one field joins the outcome)

**Do not change the landed `facet_sweep` entry signature or body behavior**
(V8 identity; the conformance suite is graded against the base). Three
additions:

1. `FacetSweepResult` gains two fields at the END of the struct:
   ```rust
   /// Mapping A row 2: the per-realization certificate, Method::Float (H-6).
   pub realization_certificate: RealizationCertificate,
   /// Mapping A row 3. Empty on the exact-grid path: the grid registry makes
   /// shared edges index-identical by construction, so there is no measured
   /// error to record. The LEDGER assembly (meshalgo) populates this when a
   /// realization is built over sampled edges.
   pub shared_edge_pairs: Vec<SharedEdgePairEvidence>,
   ```
   `realization_certificate` is filled by the existing construction from
   numbers the audit already computes: `method: Method::Float`,
   `max_cell_twist` = the audit's existing maximum bilinear-twist deviation,
   `extent` = the existing extent. No recomputation, no new tolerances.
2. The verdict absorption (mapping B — one tri-state doctrine, no third
   vocabulary):
   ```rust
   impl From<FacetVerdict> for RealizationVerdict { ... }
   ```
   CertifiedWithinTolerance→CertifiedWithinTolerance, Failed→Failed,
   Inconclusive→Inconclusive.
3. The certified entry (mapping A row 1) — a NEW function; the landed one is
   untouched:
   ```rust
   /// The realization entry per mapping A row 1: construct refusals surface
   /// as `Refusal::UnsupportedEnvelope(ConstructRefused)` with the detailed
   /// error summarized in the evidence record. `facet_sweep` stays unchanged.
   pub fn facet_sweep_certified<S: Spine>(
       recipe: &SpineFrameRecipe<S, ProfileLaw, FrameLaw>,
       stations: &[f64],
       ring_resolution: usize,
   ) -> Outcome<Certified<FacetSweepResult>>   // prelude Result<_, Refusal>
   ```
   Body: call `facet_sweep`; on `Ok` wrap with a certificate
   (`Certified::new`) whose `Certificate` records `Method::Float` — match the
   landed house pattern (see how landed `Outcome`/`Certified` entries are
   built in this crate; `Certificate::proven()` is unconstructible for float
   work — use the same constructor the landed crate-root entries use for
   `Method::Float` results). On `Err(e: ConstructError)` return
   `Refusal::UnsupportedEnvelope(EnvelopeCase::ConstructRefused)`; the summary
   goes into the evidence record of Section 4's tests (a refusal cannot carry
   a payload — the summary is asserted there by re-deriving the error kind).
   Map every variant of `ConstructError` to its summary tag in ONE place (a
   `From<&ConstructError> for ConstructErrorSummary` impl lives in
   facet_sweep.rs — geometry types are visible here; the impl is
   modeling-local so base stays geometry-blind).

The `constructive/mod.rs` frozen doc stays untouched. Zero new manifest
edges: `truck-base` is already a dependency of truck-modeling.

## Section 3 — truck-meshalgo: the assembly module (NEW realization_evidence.rs)

```rust
#![deny(clippy::unwrap_used)]
//! BG-CG-007-CERT — the realization-evidence assembly (mapping A).
//! Builds `RealizationEvidence` over a realized mesh, an optional edge-sample
//! ledger, and the verdict. Types live in truck-base (mapping placement
//! correction, 2026-08-31); this module is the integration, not the type home.
```

Public surface (exact):

```rust
pub fn assemble(
    winding_violations: usize,
    verdict: RealizationVerdict,
    certificate: Option<RealizationCertificate>,
    shared_edge_pairs: Vec<SharedEdgePairEvidence>,
    construct_error: Option<ConstructErrorSummary>,
) -> RealizationEvidence

pub fn ledger_shared_edge_pairs(
    ledger_a: &EdgeSampleLedger,   // the CG-005 type, read triangulation_with_ledger.rs
    ledger_b: &EdgeSampleLedger,
) -> Vec<SharedEdgePairEvidence>
```

`ledger_shared_edge_pairs` semantics: for the shared edge recorded in both
ledgers (same `EdgeID`), `error_a`/`error_b` are the position deviations of
the two faces' sampled positions from the shared canonical sequence — for the
landed ledger the identity is exact (`I(A,E) == reverse(I(B,E))` as integers,
CG-005), so against the landed types the comparison is the integer-identity
check: equal sequences ⇒ two zero-error entries are NOT emitted (empty vec);
any mismatch ⇒ one `SharedEdgePairEvidence` row per mismatched edge with the
measured positional deviation in `error_a`/`error_b` and the verdict doctrine
applies downstream. Do not weld, do not average, do not round: exactness is
expressed by absence of rows, error by a row. If the landed ledger type makes
cross-face comparison impossible without new plumbing, STOP — that is a
SPEC_GAP (see stop conditions).

Wiring: `tessellation/mod.rs` gains exactly `pub mod realization_evidence;`
plus any `pub use` the crate's existing re-export style uses for sibling
modules. Nothing else moves.

## Section 4 — tests

- `vendor/truck/truck-modeling/tests/facet_sweep_conformance.rs`: AMEND IN
  PLACE ONLY. Every landed test keeps its exact name (V5 identity guard). The
  two struct-literal construction sites in landed tests gain the two new
  fields (or a `..Default`-style filler if the landed style prefers); add the
  new tests below at the end.
- `vendor/truck/truck-meshalgo/tests/realization_evidence.rs`: NEW file (do
  not reuse any landed test file's path).

New tests (names are contract — `tests_required`):

1. `construct_refused_variant_exists_and_carries_no_payload` — matches on
   `EnvelopeCase::ConstructRefused`, unit variant.
2. `realization_verdict_absorbs_facet_verdict` — all three `From` arms.
3. `facet_sweep_certified_refuses_with_construct_refused` — a collapsed
   profile recipe (the landed conformance suite's refusal fixture); the
   refusal is `UnsupportedEnvelope(ConstructRefused)`, and the summary
   re-derived from the same input tags kind `"ProfileCollapse"`.
4. `facet_sweep_certified_ok_carries_evidence_and_certificate` — the landed
   straight-duct fixture; `certificate.method == Method::Float` (H-6: never
   `Exact`), `verdict == CertifiedWithinTolerance`.
5. `shared_edge_pairs_empty_on_exact_grid_path` — the exact-grid facet
   outcome carries `shared_edge_pairs.is_empty()`.
6. `ledger_assembly_fills_shared_edge_pairs` — build two ledgers over a
   shared edge; mismatched sequences ⇒ exactly one row with both errors
   non-negative; identical sequences ⇒ empty. Use the landed CG-005 ledger
   constructors; if a fixture cannot be built without new plumbing, that is a
   SPEC_GAP, not a skipped test.
7. `evidence_method_is_float_not_exact` — guard: assembling evidence whose
   certificate came from the float path never records `Method::Exact`.
8. `inconclusive_never_becomes_certified` — an `Inconclusive` verdict
   assembled in stays `Inconclusive` out; no conversion anywhere in the
   module.

House rules: H-3 forbids bare absolute float literals in comparisons — the
`// H-3` opt-out goes ON THE SAME LINE as the comparison, never the line
above. All crates keep `#![deny(clippy::unwrap_used)]`-style lints clean;
match-based unwraps only.

## Done-when

- `cargo fmt --all -- --check` clean.
- `cargo clippy -p truck-base -p truck-modeling -p truck-meshalgo
  --all-targets --message-format=short --no-deps` — zero findings.
- `cargo test -p truck-base --lib` and `cargo test -p truck-modeling --lib
  --tests` and `cargo test -p truck-meshalgo --lib --tests` all green.
- `cargo check --workspace --all-targets` green (the new base types ripple
  into every downstream crate's match sites if they exhaustively match
  `EnvelopeCase` — if a downstream match breaks, ADDING the arm in that
  downstream crate is NOT in your write set: STOP and file the deviation in
  RESULT.json instead. Measure first: `cargo check --workspace
  --all-targets` at base already passes; only arms the workspace's own code
  holds can break, and non-exhaustive matches with `_` do not).

## Stop conditions

Stop, commit nothing beyond WIP evidence, write RESULT.json (AT THE WORKTREE
ROOT) with the finding verbatim if:

1. A landed ledger API cannot express the cross-face comparison without
   plumbing outside `write_allow` (Section 3).
2. `cargo check --workspace --all-targets` shows downstream exhaustive-match
   breaks on `EnvelopeCase` (Done-when 4) — name the crates and sites.
3. Any landed test cannot keep its name and pass with the two new fields
   (Section 2) — the V5 identity guard fires on renames; if a literal needs
   more than field additions, stop.

Deviations are expected to be small and mechanical; record every one in
RESULT.json notes with the derivation. A deviation that changes the mapping's
CARRIER (not placement) is a SPEC_GAP: the mapping table is the contract.

## Finish by writing `RESULT.json` AT THE WORKTREE ROOT

Commit your work on the current branch (subject: `feat(evidence): realization
evidence integration (BG-CG-007-CERT)`) BEFORE writing `RESULT.json`. All
tests above are contract; `tests_required` names must exist verbatim.
