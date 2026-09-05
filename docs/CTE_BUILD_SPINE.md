# CTE build spine — Certified Tangency and Exact Contact program

**Status:** spine authored this session (spine workflow, `loop/ORCHESTRATOR.md`
step 1). The packet plan is
[`CERTIFIED_TANGENCY_BUILD_SPEC.md`](CERTIFIED_TANGENCY_BUILD_SPEC.md)
(CTE-000..008, ~12.9k LOC); this document is the execution decision layer the
booking deliberately does not make: contract inventory, write-set pre-matrix,
concurrency answer, integration order, wave manifest. Anchors quoted here were
re-derived by command this session against the tree.

## 1. Preconditions, re-derived

- **The general-pair SSI engine is LANDED**: `SquareSystem3` +
  `construct_square_system` (`ssi_types.rs:75`, `ssi.rs:917`), the 3×3 slice
  Krawczyk (`ssi.rs:691`), and the generic const-N operator
  (`truck-evidence/src/num/krawczyk.rs:62/86`) with the N=4 precedent
  (`construct/bie/ssi4.rs`). The theory's "existing Krawczyk applies
  unchanged" is true **only via the generic trait**; packets say so verbatim.
- **No parametric Newton exists anywhere in the tree** (re-searched this
  session). CTE-002 is genuinely new code and the program's critical path.
- **No rational polynomial algebra exists**; `Expansion` (`formal/exact.rs`)
  is float-exact evaluation only. CTE-001 is a hand-rolled module, no CAS
  dependency (product boundary, `AGENTS.md`).
- **BIE-006 ordering constraint**: BIE-006-CLASSIFY is the in-flight owner of
  `truck-shapeops/src/boolean/*` and `truck-evidence/src/contact/mod.rs`.
  CTE-006 and CTE-007 dispatch **only after the BIE-006 merge lands**. This
  is the one hard cross-program edge.
- One-verify amendment applies to CTE-* exactly as to BG-KV2-*/BIE-*:
  per-packet `verify.py` runs are SUSPENDED; the full verifier runs ONCE at
  the final integrated HEAD. Between merges: orchestrator scoped checks
  (`cargo check -p <crate>` + the packet's test file).

## 2. Contract inventory — what is frozen, where, and by whom

Contracts precede concurrency. Every cross-packet seam is frozen in CTE-000
and restated with its exact signature in the owning packet. Two theory rules
are frozen **in the type system** (booking §2): R2 (the Hessian evaluator
accepts only `GraphEnclosure`) and R1 (T1.4 never evaluates as a runtime
interval predicate).

| Contract | Frozen in | Signature (pre-decided; worker machine-checks, never reinvents) |
|---|---|---|
| `Rank2Chart` (theory §3) | CTE-000 | `{ pivot: (component, coord_pair) /* one of 18 */, det_a: CertifiedInterval /* 0 ∉ det_a by construction */, minors: ChartMinorGrids }` — refusing constructor refuses any `det_a` containing 0 |
| `GraphEnclosure` `Ŷ ⊇ φ(Z)` (§2.2) | CTE-000 | `{ y_hat: [(f64,f64);2], z: [(f64,f64);2], evidence: GraphCert }` — produced only by CTE-002's (H-graph) machinery; consumed by the Hessian evaluator. **The R2 rule: `reduced_hessian(chart: &Rank2Chart, over: GraphEnclosure)` — no raw-box overload exists** |
| `TSystem` (§2.4) | CTE-000 shape, CTE-003 impl | `impl KrawczykSystem<4>` over `(G₁,G₂,M₁,M₂)`; admission test = the T1.1 identity residual exactly 0 on the F2 fixture kit |
| `ExclusionEvidence` (T1.5) | CTE-000 | `NoRootFiveEq { spend }` \| `CertifiedCriticalPoint(KrawczykCertificate)` \| `Inconclusive { cell }` — the five-equation driver's only outputs |
| `IntervalSym2` / `Definiteness` (§2.8) | CTE-000 | `Definite { sign, mu: PositiveFinite }` \| `Indefinite { det_upper: NegativeFinite }` \| `Singular` — the constructor refuses `Definite` with `mu ≤ 0` (the R9 correction: a PD verdict must carry the μ the Taylor bound consumes) |
| `ExactVanishingWitness` (§4.2) | CTE-000 data, CTE-001 verifier | `{ s, terms: Vec<(w_i, P_i)>, q2a: Option<(q,a)>, side: A1 \| A2 }`; `fn verify(&self, sys) -> Result<Verified, WitnessRefusal>` is the **one** verifier (two instantiations, one code path). Producers never verify; the verifier never produces |
| `ContactVerdict` five-way (§2.11) | CTE-000 | `Transversal(TransversalCert)` \| `Empty` \| `A1Isolated(A1Cert)` \| `A1Node(A1Cert)` \| `A2Branch(A2Cert)` \| `Unresolved { kappa, cell }` — maps onto landed refusal vocab per `CERTIFICATE_MAPPING.md`; zero new top-level evidence kinds expected (SPEC_GAP if violated) |
| `A2BranchCurve` (T1 output / T2 strata input) | CTE-000 | `{ carrier_chart, samples: Vec<(sample, frame_cert)> }` — producing stub behind pending refusal `a2_branch_packet_pending` (the `cone_torus_carrier_packet_pending` precedent) until CTE-005 lands |
| `SideState(u8)` ∈ {00,01,10,11} (§5.3) | CTE-000 shape, CTE-006 impl | `From<&MaterialState4>`, `mid() -> bool`, `flip() -> Self`; the §5.4 algebra is coordinatewise bitwise ops — exhaustively tested, never re-derived |
| `CoincidenceWitness` + carrier/trim-domain/stratum shapes (§5.2) | CTE-000 (consuming side) | Carrier `{ normal, chart }`, trim domains in the carrier chart; production lands in CTE-007 |

Fixtures precede solvers: every wave worker builds against the CTE-000
fixture kit (F1–F7, booking §5) and its own packet's synthetic fixtures,
never against an upstream sibling's production code.

## 3. Waves and interleave

```text
W0 (shim, normal loop, full verify once):  CTE-000
        │ landing merge SHA = wave base
W1 (parallel, contracts frozen):           CTE-001 ∥ CTE-002 ∥ CTE-003 ∥ CTE-006*
W2 (parallel):                             CTE-004 ∥ CTE-005
W3 (single):                               CTE-007   (after BIE-006 merge)
W4 (single):                               CTE-008 + ONE full battery at integrated HEAD
```

\* CTE-006 writes only `boolean/mod.rs`, depends on nothing in CTE, and has
no BIE file conflict — but per the hard cross-program edge it still waits for
the BIE-006 merge. If BIE-006 has not landed by W1, CTE-006 slips to W2; it
is 0.4k and never the critical path.

- **CTE-004 does not parallel CTE-002/003 as a file matter** (its files are
  disjoint) but has a real contract dependency: it consumes `GraphEnclosure`
  production (002) and `TSystem` admission (003). It dispatches when both
  land, not when its wave siblings finish (rolling-dispatch rule).
- **CTE-007 is the program's convergence point** (consumes 001, 004, 005,
  006) and the sole writer of `boolean/{split,assemble}.rs` — one worker by
  design.
- **CTE-008** runs the differential/termination/node batteries and the
  program's single full verify. Nothing integrates after it.
- **Rolling dispatch** (session-50 posture): waves are bookkeeping and
  integration boundaries, never dispatch barriers. Author the next packet
  while workers run; never idle a slot behind authoring.

## 4. Write-set pre-matrix

Same new file = collapse; same landed file = serial or shim; disjoint = parallel.

| pair | verdict | basis |
|---|---|---|
| 000 / any | disjoint post-W0 | 000 owns `tangency/{mod,shapes,fixtures}.rs` + the `lib.rs` mod line and is LANDED at the wave base |
| 001 / 002 | disjoint | `qpoly.rs`+`witness.rs` vs `chart.rs`+`graph.rs` |
| 001 / 003 | disjoint | different files; 003 reads `formal/exact.rs` only |
| 002 / 003 | disjoint | `chart.rs`+`graph.rs` vs `minors.rs`+`tsystem.rs`+`exclude.rs`; both read `ssi.rs` |
| 003 / ssi.rs | **003 is ssi.rs's sole writer** | additive `Tensor4::partial2_axis`; `kernel/engine.rs` untouched (V5 guard — the pattern is adapted, not imported) |
| 003 / 004 | serial by dep, disjoint files | 004 consumes 003's `TSystem` + `ChartMinorGrids` |
| 004 / 005 | disjoint | `hessian.rs`+`cascade.rs` vs `a2.rs` |
| 005 / 006 | disjoint, different crates | `tangency/a2.rs` vs `boolean/mod.rs` |
| 006 / 007 | disjoint files, same dir | 006: `mod.rs`; 007: `split.rs`+`assemble.rs` — additive rule; 007 consumes 006's `SideState` (contract, W1) |
| 007 / 008 | serial by dep | 008's batteries run over 007's output |
| any / BIE packets | disjoint **except the BIE-006 edge** | the one hard ordering (§1) |
| any / landed tests | **V5 identity guard** | `boolean_m2.rs`, `conformance_battery.rs`, and every landed test name are byte-identical constraints; CTE-008 adds NEW test files only |

Expected textual conflicts, resolved at integration, exempt from the clash
rule: `pub mod` lines in `tangency/mod.rs` (001, 002, 003, 004, 005, 007,
008 each add one) and one line in `truck-certified/src/lib.rs` (000 only).

## 5. Concurrency answer (measured machine facts)

- **Cap: 4 workers** — the session-51 re-derived arithmetic stands (no
  per-worker rust-analyzer, `lsp:false` lean profile): waves here are ≤4
  wide, so the cap binds exactly once (W1).
- cargoq is MANDATORY and must be running before any dispatch; the bypass
  rules in ORCHESTRATOR apply verbatim.
- `CARGO_BUILD_JOBS=2`, `CARGO_INCREMENTAL=1`, `RUSTC_WRAPPER=sccache` if
  available (check before W1; if absent, run 2 workers).
- Prewarm each slot's target once at dispatch; workers run scoped checks only
  (`cargo check -p truck-certified` / `-p truck-shapeops`, the packet's own
  tests) — never a workspace build, never baselines or corpus suites.
- **Critical path** = spine → CTE-002 (parametric Newton) → CTE-004 →
  CTE-007 → CTE-008. Two workers cover W2/W3 fully; the third and fourth
  slots in W1 buy CTE-003 and CTE-006 in parallel with the critical path.
- Disk: `janitor.py ensure --need` is wired into `new_slot`; still check free
  disk before W1 and before the final battery.

## 6. What this spine deliberately does not decide

- **Witness production scheduling** (theory §6.1): the verifier contract is
  frozen; producers are built only when a measured unresolved rate on real
  mating geometry demands them. No packet in this program owns production.
- **The A₃⁺ / ≥3-operand / positive-codimension tails** (§6.2–6.4): verdicts
  route to `Unresolved`; strata booking lands with CTE-007, production does
  not.
- **Whether CTE overtakes or trails the BIE program**: only the BIE-006 edge
  is hard; everything else is slot-arithmetic for the orchestrator.
