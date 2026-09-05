# BIE build spine — Certified Interaction Engine program

**Status:** spine authored 2026-09-05 (spine session, per the build-spec spine
workflow in `loop/ORCHESTRATOR.md` step 1). The packet plan is
[`CERTIFIED_INTERACTION_ENGINE_BUILD_SPEC.md`](CERTIFIED_INTERACTION_ENGINE_BUILD_SPEC.md)
(BIE-000..007, ~13.7k LOC); this document is the execution decision layer the
booking deliberately does not make: interleave, write-set pre-matrix, contract
inventory, concurrency answer, integration order, wave manifest. Every anchor
quoted here and in the packets was re-derived by command on 2026-09-05 against
the tree; where the booking's substrate audit has drifted, the drift is
recorded in §2.

## 1. Preconditions, re-derived

- **The CC solver chain is LANDED.** `loop/PACKETS.jsonl` notes: CC-001
  BANDED (interval no-pivot banded GE, outward-rounded), CC-003 ARGMIN
  (argmin-with-margin), CC-020 CONTACT-K3 (k=3 reduction over
  `krawczyk_c1_n4`, landed c2857ba), CC-030 BLEND-SPINE (two-support
  reduction over `krawczyk_c1_n3`, landed a077887). The booking's "sequence
  AFTER the CC solver chain" precondition is satisfied. Landed modules:
  `truck-certified/src/construct/{banded,argmin,contact3,blend}.rs`.
- **The kernel v2 swarm is CLOSED** (26/26, battery green at
  `fd65c24..45d9ad6+`). No hot-file contention from that program.
- One-verify amendment applies to BIE-* packets exactly as it did to
  BG-KV2-*: per-packet `verify.py` runs are SUSPENDED; the full verifier
  runs ONCE at the final integrated HEAD. Between merges: orchestrator
  scoped checks (`cargo check -p <crate>` + the packet's test file).

## 2. Substrate re-derivation — where the booking's audit has drifted

The booking is normative for scope and LOC bands. These are the measured
corrections; each is quoted in the owning packet's anchors.

| Booking says | Tree says (measured 2026-09-05) | Disposition |
|---|---|---|
| span cache in `truck-base/src/bvh.rs`; `span.rs:20` | `span.rs` lives in **`truck-geometry/src/`**; the swept-face gap is the arm `Surface::SpineFrameSurface(_) => Vec::new()` at `span.rs:104` | BIE-003's swept-face bounds land in `span.rs` (truck-geometry), not truck-base; `bvh.rs` is NOT in BIE-003's write set |
| `Region2` machinery reusable in the (s,v) chart | No `Region2` type exists anywhere in the tree. The planar arrangement machinery is `Arrangement`/`ArrRegion`/`ArrHalfEdge` (`truck-geometry/src/arrange.rs:64/90/106`) behind `pub fn arrange` (:134) | BIE-005's containment equivalent targets `ArrRegion` containment, not "Region2"; packet quotes the real names |
| `Carrier2D` is `Line | CircleCarrier` only at `arrange.rs:682` | Still exactly that shape, now at `arrange.rs:1049` | Pattern anchor, no drift in substance |
| `KrawczykSystem<const N: usize>` (read as a struct) | It is a **trait**: `pub trait KrawczykSystem<const N: usize>` at `krawczyk.rs:62`; the entry is `pub fn krawczyk<const N: usize>(system: &impl KrawczykSystem<N>, start: &[Interval; N], budget)` at :86 | BIE-002 instantiates the trait over its F-form system; no struct to extend |
| interval arithmetic ABSENT (`IntervalEnclosure` is a value type only) | **Partially stale**: `CertifiedInterval` (outward-rounded scalar interval: add/sub/neg/mul/div/sqrt/width/contains) is landed in `truck-certified/src/formal/exact.rs:227`, and the CC modules compose it with outward rounding throughout | BIE-001 is a **scope reduction**: a 4-D box type + mean-value/Taylor range bounds + Bernstein n=4 evaluation reusing `CertifiedInterval` — no new outward-rounded scalar interval library. The 1.5k band stands as an upper bound |
| `SpineFrameSweep` has `subs`/`der` (direct F evaluation) | Confirmed: `impl ParametricSurface for SpineFrameSweep` at `sweep_surface.rs:124`; windowed domain fields on the closed value at :52 | BIE-002 evaluates F directly; no polynomialization |
| swept faces lack `EnclosureSurface` | Confirmed: no `impl EnclosureSurface for SpineFrameSweep` exists; `pub trait EnclosureSurface` at `truck-evidence/src/enclosure.rs:175` with decorator impls only | BIE-003's swept-face AABB bounds are sampling/enclosure-based new code in `span.rs` |
| `BoundedStratum::Face/Edge` | Confirmed: `pub enum BoundedStratum` at `contact/mod.rs:87`; **no Sweep variant** | BIE-006 adds `BoundedStratum::Sweep` (contact/mod.rs is hot — BIE-006 is the program's only writer there) |
| Bernstein hull landed 1-D/2-D, n=4 ABSENT | Confirmed: `hull_bernstein_1d`/`hull_bernstein_2d` at `hull.rs:95/126`; no box form | BIE-001 adds the 4-D box Bernstein evaluation as new code in its own module; `hull.rs` is untouched (V5 guard) |
| SSI square-system engine | `truck-certified/src/ssi.rs` (cross-multiplied homogeneous system over rational tensor-Bernstein patches) is LANDED | That is the **general-pair tail**, not the BIE normal path. BIE never cross-multiplies; the spine keeps the two modules separate and the packets say so |
| `truck-certified/src/construct/bie/` stub | ABSENT — BIE-000 creates it | BIE-000 is the shim |
| `EdgeSampleLedger` | Landed in `truck-meshalgo/src/tessellation/` | BIE-003 integrates read-only against it; truck-meshalgo is not in any BIE write set |

## 3. Contract inventory — what is frozen, where, and by whom

Contracts precede concurrency. Every cross-packet seam is frozen here and
restated with its exact signature in the owning packet. A worker is
serialized only where a genuinely missing shared type forces it (§6).

| Contract | Frozen in | Signature (pre-decided; worker machine-checks, never reinvents) |
|---|---|---|
| `InteractionOutcome` + `Unresolved{κ, cell, slope}` witness mapping | BIE-000 (shim) | Maps onto landed `Refusal::NumericallyUnresolved` (`truck-base/src/evidence.rs:57`) — zero new refusal arms; SPEC_GAP if a new arm seems needed |
| Unit-shape fixture kit with stated ground truths | BIE-000 (shim), `construct/bie/fixtures.rs` | plane×sphere (circle, radius known), plane×cylinder (ellipse, axes known), transverse sweep×plane (conic, station parameters known), each machine-checked against its closed form |
| Outward-rounded 4-D box + range bounds | BIE-001 packet §Contract | `IntervalBox4::new([(lo,hi);4])` (refusing on lo>hi, non-finite), `IntervalBox4::mean_value_bound(f, grad_fn, box) -> (f64, f64)`, Bernstein box eval `bernstein_box4(grid, box) -> (f64, f64)`; all reusing `CertifiedInterval` |
| Solver system + output types | BIE-002 packet §Contract | `Ssi4System` (impl of landed `KrawczykSystem<4>` over a restricted-pair F-form), output `CertifiedChartCurve { samples: Vec<(u,v,s,t)>, tangent_frames, witness }` — the type BIE-004 escalates and BIE-005 consumes |
| `CertifiedImplicitIntersectionCurve` edge carrier | BIE-003 packet §Contract | New canonical curve variant/decorator carrying a certified 3-D polyline + per-sample frames; PL at tessellation only (ledger-compatible, read-only vs truck-meshalgo) |
| Validity gate entry | BIE-007 packet §Contract | `pub fn chi_homology_gate(complex) -> Outcome<GateReport>` over the output complex; FAILED not warned on mismatch |
| `BoundedStratum::Sweep { recipe, window }` | BIE-006 packet §Contract | Variant on the landed enum at `contact/mod.rs:87`; LIFT recognizes `SpineFrameSweep` faces instead of refusing `NonCanonicalCarrier` |

Fixtures precede solvers: every wave worker builds against the BIE-000
fixture kit and its own packet's synthetic fixtures, never against an
upstream sibling's production code.

## 4. Waves and interleave

```text
W0 (shim, normal loop, full verify once):  BIE-000
        │ landing merge SHA = wave base
W1 (parallel, contracts frozen):           BIE-001 ∥ BIE-002 ∥ BIE-003
                                           ∥ SEM-PCURVE-MASTER-001-FIX
W2 (parallel):                             BIE-004 ∥ BIE-005
W3 (parallel):                             BIE-006 ∥ BIE-007
Final:                                     ONE full battery at integrated HEAD
```

The wave-1 fourth member, `SEM-PCURVE-MASTER-001-FIX`, is the recorded STEP
defect (`docs/defects/SEM-PCURVE-MASTER-001.md` → correction 1: honor the
declared 3D curve over pcurve mastery). Its write set is
`truck-stepio/src/in/mod.rs` + one new test file — fully disjoint from every
BIE packet — and it has no BIE dependency; it rides wave 1 for slot
efficiency only. Its outcome does not gate any BIE packet.

- **W0 is the only mid-program full verify** (the shim's own normal-loop
  verify; its landing merge is the wave base). Everything after is
  LOCAL_GREEN + orchestrator scoped checks, one-verify amendment.
  **Overnight deviation (owner-directed, recorded):** when the program is
  driven by `loop/overnight.py` (whose `try_land` adjudicates on scoped
  checks + merge and never runs `verify.py`), there is NO mid-program
  verify — the shim is certified by the single end-of-program battery
  together with everything else. One verifier, once, total.
- **BIE-004 does not parallel with BIE-002** (real code dependency, not
  posture): 004 escalates the landed `construct/bie/ssi4.rs` module 002
  owns — scheduler changes inside a file 002 wrote. That is a genuinely
  missing shared file, which is the legitimate serialization reason.
- **BIE-006 is the program's only writer of `truck-evidence/src/contact/mod.rs`**
  and `truck-shapeops/src/boolean/*` — the booking's "hot file — serial"
  rule. BIE-007 touches a new `truck-shapeops/src/gates/` module and a new
  test file only, so it runs parallel with 006; the differential battery
  vs `boolean_m2` consumes landed fixtures read-only.
- **Rolling dispatch** (session-50 posture): waves are bookkeeping and
  integration boundaries, never dispatch barriers. A slot that frees
  refills with the next READY packet whose real contract deps landed.
  Author the next packet while workers run; never idle a slot behind
  authoring.

## 5. Write-set pre-matrix

Same new file = collapse; same landed file = serial or shim; disjoint = parallel.

| pair | verdict | basis |
|---|---|---|
| 000 / 001 | disjoint (post-W0: 000 LANDED at wave base) | 000 owns `construct/bie/{mod,fixtures}.rs` + `docs/CERTIFICATE_MAPPING.md`; 001 owns `src/interval/*` + `lib.rs` |
| 001 / 002 | disjoint | 001 owns `src/interval/*`; 002 owns `construct/bie/ssi4.rs` + `truck-evidence/src/num/parallelotope.rs` + their two `pub mod` lines |
| 001 / 003 | disjoint | different crates (`truck-certified` vs `truck-geometry/src/{canonical.rs,span.rs,intersection_carrier.rs}`) |
| 002 / 003 | disjoint | `truck-certified`+`truck-evidence/num` vs `truck-geometry` |
| 002 / 004 | **serial** | same file `construct/bie/ssi4.rs` — 004 escalates 002's landed scheduler |
| 003 / 005 | disjoint, same crate | 003 owns `canonical.rs`+`span.rs`; 005 owns `arrange.rs` additive — separate-module rule |
| 004 / 005 | disjoint | `construct/bie/{ssi4,closure}.rs` vs `arrange.rs` |
| 005 / 006 | serial by dep, disjoint files | 006 consumes 005's arrangement cells (contract: cell seed list); files disjoint |
| 006 / 007 | disjoint | 006: `contact/mod.rs` + `boolean/*`; 007: new `gates/` module + new test file |
| FIX / any BIE packet | disjoint | the defect fix owns `truck-stepio/src/in/mod.rs` + a new test file — no BIE packet names truck-stepio |
| any / landed tests | **V5 identity guard** | `truck-shapeops/tests/boolean_m2.rs`, `conformance_battery.rs`, and every landed test name are byte-identical constraints; no BIE packet edits a landed test file |

Expected textual conflicts, resolved at integration, exempt from the
clash rule: one `pub mod` line each in `truck-certified/src/lib.rs`
(001), `construct/mod.rs` (000), `truck-evidence/src/num/mod.rs` (002).

## 6. Concurrency answer (measured machine facts)

- **Cap: 4 workers** — the session-51 re-derived arithmetic (no per-worker
  rust-analyzer, `lsp:false` lean profile, chrome closed): N × (worker host
  + helpers) + OS baseline + ONE queued spike ≤ 15.7 GB; 6 re-enters the
  `0xc0000409` zone on a cold warm-build cycle. Waves here are ≤3 wide, so
  the cap binds only when program-tail packets overlap another program.
- cargoq is MANDATORY and must be running before any dispatch (PATH shim;
  the bypass rules in ORCHESTRATOR apply verbatim — the packets state the
  house rule in their own text).
- `CARGO_BUILD_JOBS=2`, `CARGO_INCREMENTAL=1`, `RUSTC_WRAPPER=sccache` if
  available (check before W1; if absent, install or run 2 workers).
- Prewarm each slot's target once at dispatch; workers run scoped checks
  only (`cargo check -p <crate>`, the packet's own tests) — never a
  workspace build, never baselines, corpus suites, or global gates.
- Disk: `janitor.py ensure --need` is wired into `new_slot`; still check
  free disk before W1 and before the final battery (~30 GB as of authoring).

## 7. Integration order and the final battery

Merge order (dependency order, `cargo check -p <affected-crate>` between
merges; `--no-ff` into `integration/kernel-bg`):

```text
BIE-000 → (BIE-001 → BIE-002 → BIE-004) → (BIE-003 → BIE-005 → BIE-006) → BIE-007
```

`SEM-PCURVE-MASTER-001-FIX` merges at any point after W0 opens — it is
dependency-free and disjoint; slot it wherever a merge pass is cheap.

001 precedes 002 (002 consumes the frozen `IntervalBox4` API in its own
tests), 003 precedes 005/006 (carrier type), 006 precedes 007 (the gate
runs on 006's output complex). The 002 and 003 chains interleave freely
between merges.

The final verification is the ordinary verifier ONCE at the composed HEAD:
workspace tests + workspace clippy (baseline-aware, evidence-carrying per
session-51 rules) + kernel-gates + the BIE batteries (completeness
batteries, differential vs landed canonical booleans, unresolved-rate
recording per the booking's §8 completion definition). Registry rows flip
DONE only when it passes; the wave manifest (§9) is filled from the
ledger + git.

## 8. Scoped gates every BIE packet carries (stated per-packet too)

- **V5 identity guard**: the canonical × canonical boolean path is never
  regressed — no landed test file is edited, no landed test renamed,
  `boolean_m2` results bit-identical.
- **Typed outcomes only**: no uncertified shape; `Unresolved` is
  first-class and carries κ/cell/slope; zero new `Refusal` arms
  (a violation is a SPEC_GAP booked in `docs/CERTIFICATE_MAPPING.md`).
- **H-3 discipline**: float comparison epsilons in tests carry `// H-3`
  on the SAME line (the autoformatter-off profile keeps it there).
- **Determinism**: identical ordered input → identical verdicts; no
  output ordering from hash iteration.
- All cargo invocations through the cargoq queue (the `cargo` on PATH IS
  the queue shim); scoped commands only, never a bare `cargo test`.

## 9. Wave manifest (empty — filled at integration)

```json
{
  "program": "BIE",
  "spine_doc": "docs/BIE_BUILD_SPINE.md",
  "wave_base_shim": null,
  "base_sha": null,
  "members": [],
  "integration_amendments": [],
  "verifier_version": null,
  "final_integrated_sha": null
}
```

## 10. Dispatch checklist (per packet, no exceptions)

1. `gen_packet --check` on the packet AFTER the shim lands (anchor-drift
   trap: anchors authored pre-shim drift by exactly the shim's changes).
2. `packet_lint` preflight.
3. `dispatch_ready.py` is the dispatch authority (write-set disjointness
   against LIVE slots, not registry rows).
4. cargoq ping (`curl 127.0.0.1:8231/ping`) + free disk before the first
   dispatch of a wave.
5. On FINISHED: adjudicate (RESULT is a claim), scoped checks at merged
   HEAD, merge `--no-ff`, file RESULT to `loop/results/<ID>.json`, ledger
   row, delete repo-root RESULT.json (committed), registry note
   `LANDED <sha> (one-verify amendment)` — status stays RUNNING until the
   final battery.
