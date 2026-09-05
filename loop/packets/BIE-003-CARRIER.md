# WORK PACKET BIE-003-CARRIER — the certified implicit intersection curve carrier

You are implementing the curve carrier of the Certified Interaction Engine
(BIE) program. Everything you need is in this document and
`docs/BIE_BUILD_SPINE.md`. Do not read other spec files. If something you
need is genuinely missing, that is a SPEC_GAP (see "Stop conditions"): you
stop and report, you do not research it.

```yaml
id:          BIE-003-CARRIER
contract:    [BIE-003-CARRIER]
class:       design
crates:      [truck-geometry, truck-modeling, truck-shapeops, truck-stepio]
depends_on:  [BIE-000-CONTRACT]
write_allow:
  - vendor/truck/truck-geometry/src/canonical.rs
  - vendor/truck/truck-geometry/src/constructive/mod.rs
  - vendor/truck/truck-geometry/src/constructive/intersection_carrier.rs
  - vendor/truck/truck-geometry/src/span.rs
  - vendor/truck/truck-geometry/src/recognize.rs
  - vendor/truck/truck-modeling/src/cad.rs
  - vendor/truck/truck-shapeops/src/section.rs
  - vendor/truck/truck-stepio/src/out/geometry.rs
read_allow:
  - vendor/truck/truck-geometry/src/constructive/sweep_surface.rs
  - vendor/truck/truck-meshalgo/src/tessellation/realization_evidence.rs
  - vendor/truck/truck-base/src/bvh.rs
  - docs/BIE_BUILD_SPINE.md
tests_required:
  - carrier_constructs_from_certified_polyline
  - carrier_refuses_uncertified_input
  - carrier_pl_at_tessellation_only
  - swept_face_span_covers_windowed_domain
  - canonical_ripple_delegates_all_methods
budget:      {turns: 60, ctx_tokens: 150000}
```

**New file** (`constructive/intersection_carrier.rs`): H-1 applies — no
`unwrap_used` without a justified same-line opt-out.

## Problem

Theory §8.1: the procedural intersection-curve carrier is "prerequisite for
everything, requires none of the theory". The landed canonical `Curve` enum
carries `IntersectionCurve` (the decorator precedent) but no *certified*
implicit intersection carrier — one that stores its certified polyline and
per-sample tangent frames, refuses uncertified input, and is PL at
tessellation only. Additionally the broad phase is blind to swept faces: the
span cache returns nothing for `SpineFrameSweep`, so swept faces get no
per-face AABB. This packet lands both.

## Scope decisions — pre-made, do not relitigate

> **AMENDMENT r2 (2026-09-05, orchestrator; resolves the r1 SPEC_GAP).** The
> canonical `Curve` variant's exhaustive-match ripple reaches SEVEN sites
> outside the original write set: `truck-geometry/src/recognize.rs` (2),
> `truck-modeling/src/cad.rs` (2), `truck-shapeops/src/section.rs` (1),
> `truck-stepio/src/out/geometry.rs` (3). Those files are now in
> `write_allow`. For each site add the new variant arm
> (`| Curve::CertifiedImplicitIntersectionCurve(_)`) delegating exactly as
> the landed `IntersectionCurve` arm delegates — no landed arm changes
> semantics. Everything r1 committed (carrier, spans, canonical ripple in
> `canonical.rs`) is proven work: keep it, do not redo it. Scoped checks
> must now also pass: `cargo check -p truck-modeling -p truck-shapeops -p
> truck-stepio` (the signature-ripple rule: a canonical variant is a
> cross-crate fact).

1. **The carrier type lives in a new file**,
   `truck-geometry/src/constructive/intersection_carrier.rs`; the canonical
   `Curve` enum in `canonical.rs` gains one variant delegating to it —
   mirroring the landed `IntersectionCurve` pattern exactly (variant arm +
   the `From` impl + the method-delegation macro arm; see anchor A1 and copy
   how the landed variant threads through every macro arm).
2. **The carrier is the frozen contract** (spine §3): a certified 3-D
   polyline with per-sample tangent frames and the unresolved witness slot
   (`InteractionOutcome`-compatible via the BIE-000 vocabulary; if the type
   is needed here, define the minimal carrier-local mirror and note it —
   truck-geometry does not depend on truck-certified).
3. **PL at tessellation only**: the carrier is procedural (evaluable
   continuously through its stored frames); its polyline form is consumed
   ONLY at tessellation. The ledger integration is read-only against
   `truck-meshalgo`'s landed `EdgeSampleLedger` — the carrier must carry the
   sample data the ledger expects, but truck-meshalgo is NOT edited and NOT
   in your write set.
4. **Swept-face AABB bounds land in `span.rs`** — NOT `truck-base/bvh.rs`
   (spine §2 drift record: the span cache lives in truck-geometry). Replace
   the blind arm `Surface::SpineFrameSurface(_) => Vec::new()` with real
   span records: sampling/enclosure-based bounds over the windowed domain
   `(s0, s1, v0, v1)` fields of the landed `SpineFrameSweep` (:52), outward
   by construction. `bvh.rs` is NOT edited — `candidate_pairs` consumes the
   spans generically.
5. **V5 identity guard**: the canonical × canonical path must not change
   behavior. The new variant is additive; every landed method must delegate
   (or refuse typed) for it — no existing arm changes semantics.

## Anchors — measured 2026-09-05, counts are exact

Locate by pattern, never by line number. If a count differs, STOP and report
`ANCHOR_MISMATCH` with what you saw.

| id | file | pattern | expect |
|---|---|---|---|
| A1 | `vendor/truck/truck-geometry/src/canonical.rs` | `IntersectionCurve\(IntersectionCurve<Box<Curve>, Box<Surface>, Box<Surface>>\)` | 1 |
| A2 | `vendor/truck/truck-geometry/src/span.rs` | `Surface::SpineFrameSurface\(_\) => Vec::new\(\)` | 1 |
| A3 | `vendor/truck/truck-geometry/src/constructive/sweep_surface.rs` | `pub struct SpineFrameSweep` | 1 |
| A4 | `vendor/truck/truck-geometry/src/constructive/mod.rs` | `^pub mod` | 1 |
| A5 | `vendor/truck/truck-base/src/bvh.rs` | `pub fn candidate_pairs\(` | 1 |

A4 becomes 2 when you add `pub mod intersection_carrier;`.

## House rules

- **H-1** No `unwrap`, `expect`, `panic!`, `unimplemented!`, `todo!`, or
  out-of-range indexing reachable from geometry.
- **H-2** Fallible operations return `Outcome<T>`.
- **H-3** No absolute constants in predicates; test epsilons carry `// H-3`
  on the SAME line as the literal.
- **H-6** Float-computed sample positions are certified (the certificate
  rides on the polyline, not claimed exact).
- **All cargo invocations go through the queue (the `cargo` on PATH IS the
  queue shim). Do not invoke cargo by absolute path; do not unset the shim.**
- Never run a bare `cargo test` — use the scoped commands below.

## Tests required

Named `#[test]` fns (in-module test sections) — the verifier checks the
names appear in your diff.

1. `carrier_constructs_from_certified_polyline` — a certified polyline +
   frames constructs; `subs`/`der`-style evaluation through the stored
   frames interpolates within `// H-3` tolerance.
2. `carrier_refuses_uncertified_input` — constructing from bare floats
   without a certificate refuses (H-2), never panics.
3. `carrier_pl_at_tessellation_only` — the polyline accessor is the
   tessellation-facing one; the continuous evaluation does not round-trip
   through the polyline.
4. `swept_face_span_covers_windowed_domain` — for a straight-spine
   `SpineFrameSweep`, the new span records cover the windowed domain's
   world-space extent (compare against hand-derived bounds).
5. `canonical_ripple_delegates_all_methods` — every method/macro arm of the
   canonical `Curve` enum handles the new variant (compile-enforced +
   explicit assertions where the landed suite asserts behavior).

No existing test may be deleted, `#[ignore]`d, or weakened. The landed
suite that exercises `Surface::SpineFrameSurface(_) => Vec::new()` (if any
test asserts emptiness) must keep its name and gain the new expectation
only as ADDITIONAL assertions, never by weakening existing ones — if that
is impossible, `SPEC_GAP`.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-geometry
cargo clippy -p truck-geometry --all-targets -- -D warnings
cargo test -p truck-geometry --lib --tests
cargo check -p truck-certified -p truck-shapeops -p truck-meshalgo
```

The last one proves the enum ripple did not break downstream consumers
(the signature-ripple rule: a canonical variant is a cross-crate fact).
Send cargo output to a file and read the tail.

## Forbidden

Editing any file outside `write_allow` — especially `truck-base/src/bvh.rs`,
anything under `truck-meshalgo/` (read-only ledger), `truck-shapeops/`,
`truck-certified/`, `scripts/kernel-gates.sh`, `Cargo.lock`. Changing any
landed variant's semantics. Adding `#[ignore]`. Adding `#[allow]` without a
justification comment on the same line. Committing to `main`.

## Stop conditions

- any anchor count differs → `ANCHOR_MISMATCH`
- the canonical ripple reaches a macro arm that cannot delegate → stop and
  report the arm verbatim, `SPEC_GAP`
- three consecutive failed `cargo test` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` at the WORKTREE ROOT (then COMMIT first)

**COMMIT BEFORE writing `RESULT.json`.** Then write `RESULT.json` at the root
of your worktree (not `loop/results/` — the orchestrator files it there).

```json
{"id":"BIE-003-CARRIER","status":"DONE","contracts":["BIE-003-CARRIER"],
 "tests_added":5,"anchors_verified":{"A1":1,"A2":1,"A3":1,"A4":1,"A5":1},
 "notes":"how the span bounds are derived, and any canonical macro arm that needed a deviation"}
```

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit on the current branch with subject
`feat(geometry): certified implicit intersection curve carrier + swept-face spans (BIE-003-CARRIER)`.
