# Packet economics — cost-aware packet sizing from measured loop data

*Status: harness doctrine (2026-09-14, owner-directed). Companion to
`docs/CONTACT_ATLAS_BUILD_SPEC.md`, which applies it. Every claim below is
either measured from `loop/LEDGER.jsonl` (348 rows) or derived and labeled
as a model. The model's purpose: scope packets so the loop's expected cost
per landed obligation is minimized.*

## 1. Measured basis

- 348 ledger rows; 62 carry an attributed fault.
  Mix: `PACKET` 27, `WORKER` 8, `HARNESS` 8, `ORCHESTRATOR` 12,
  `GATE` 5, `SPEC` 1, other 1.
- **The dominant controllable waste is `PACKET`: 27 of 62 (44%)** —
  defects introduced by packet authoring (wrong anchors, wrong scope,
  underspecified judgements), not by workers or gates.
- Small packets (≤ 6 tests added, n = 18 rows where size is recorded):
  **17% round-trip/fault rate**. The large-packet sample in the ledger is
  too sparse for a rate, but the program's recorded large-packet events
  (MONO-6 ~1150 LOC, multi-cycle; session-51's verify sink: 401 ×3 / 405
  ×6 attempts) are the qualitative large-packet baseline.
- Measurement caveat: `tests_added` is recorded sparsely; going forward
  every ledger row records `obligations`, `loc_landed`, `attempts`,
  `worker_minutes` (§6) so this model gets sharper with use.

## 2. The cost model (derived, calibrated by §1)

For one packet, let:

- `k` = number of independent judgements the packet makes (design
  decisions a worker must not relitigate: admission decisions, signature
  choices, scoping calls, anchor claims about structure);
- `L` = landed LOC (implementation + tests);
- `p(k)` = probability the packet lands without a round trip. Model:
  `p(k) = p1^k` with `p1 ≈ 0.95` per judgement — calibrated by the 17%
  small-packet fault rate (≈ 1–2 judgements deep: `0.95² ≈ 0.90`,
  observed 0.83; treat `p1 = 0.92–0.95` as the working range);
- `C_ceremony` = fixed per-packet cost (dispatch, slot warm-up, worker
  cold-start context read, landing, ledger): observed ~20–40 min;
- `C_work(L, k)` = worker productive time, roughly linear in `L` plus a
  per-obligation design cost that is superlinear when obligations share
  a context (the k-th judgement costs more than the first: each must be
  kept consistent with the previous k−1);
- `C_reject(L, k)` = cost of a rejection: redispatch or amendment, plus
  recovery-from-archive on the big-packet path. Measured examples:
  20 min (small amendment) to 2+ h (big redispatch).

Expected cost:

```
E[cost] = C_ceremony + p(k)·C_work(L,k) + (1−p(k))·(C_work(L,k) + C_reject(L,k))
```

The failure term is what makes sizing nonlinear: a packet that doubles
`L` and `k` does not double its expected cost — it doubles the productive
cost AND adds a failure term whose probability grew multiplicatively and
whose recovery cost grows with `L` (more proven-correct work to salvage).
The ledger's `PACKET`-fault mass sits exactly in this term.

Additional scaling terms (all observed, none modeled away):

- **Context saturation**: workers have fixed turn/token budgets. Above a
  packet-specific size, budget goes to re-reading instead of editing, and
  the observed end-of-budget failure mode is guessing (the
  skipped-commit class, 3+ occurrences recorded). Effectively
  `C_work` goes from linear to convex past the saturation size.
- **Drift exposure**: the probability that a sibling landing invalidates
  a packet assumption grows with its wall-time window. Small packets
  shrink the window; floor anchors (gen_packet `min:`, 2026-09-14) shrink
  the damage on the check side.
- **Adjudication cost** is superlinear in `L`: the operator verifies the
  whole surface at merged HEAD, and a big packet's verdict is harder to
  attribute when one test fails among twenty.

## 3. The sizing law

1. **One packet = 1–3 modest proof obligations.** Each obligation is
   stated in the packet as a claim with a machine-checked test.
2. **`L ≤ ~800` LOC landed** per packet (impl + tests). Beyond this,
   split by obligation, not by file.
3. **Tests runnable in < 2 min** per packet (scoped `cargo test -p …
   --test …`); a packet whose own tests need a full battery is too big.
4. **Budget ≤ 45 turns / 150k ctx.** A packet whose worker needs more is
   two packets.
5. **Bundle only homogeneous churn**: many instances of ONE judgement
   (FHC-D's 10 call-sites, zero round trips — the measured exception).
   Novel theory never bundles: each obligation is its own packet.
6. **Success target ≥ 80% per packet** (`p(k) ≥ 0.8` → `k ≤ 2–3` at
   `p1 = 0.92–0.95`). A packet the spine cannot get under 80% is a
   signal the obligation is not yet well-understood — write the theory
   doc or a survey first, not a bigger packet.
7. **Rejection-economics test**: prefer the packet shape whose rejection
   is cheapest. "Prefer amending proven work over redispatching" only
   stays cheap when the proven work is small.

## 4. Program-level consequence

Total program time for a wave:

```
T ≈ Σ_packets [ C_ceremony + p·C_work + (1−p)·(C_work + C_reject) ] / parallelism
```

Small packets raise `parallelism` (more independent write sets), raise
`p`, and lower `C_reject` — three of the four terms move the right way.
The only term that fights back is `C_ceremony × packet_count`, which is
why rule 5 (bundle homogeneous churn) and the shared contract shim exist:
the ceremony floor is paid once per *contract*, then packets ride it.

## 5. Application — Contact Atlas program re-scope

The `docs/CONTACT_ATLAS_BUILD_SPEC.md` waves were drafted before this
law; they comply for Waves 0–2 and violate it in Wave 4. Re-scope:

- **Wave 4 split** (was: CT-400 margin, CT-401 charts, CT-402 topology
  T1–T5, CT-403 tubes, CT-404 arrangement, CT-405 resolved flux, CT-406
  events — three of those are multi-obligation):
  - `CT-400` unchanged (margin: one obligation).
  - `CT-401a` chart certificate C1 (Krawczyk inclusion); `CT-401b`
    exclusion predicates C0. (was CT-401, one obligation each)
  - `CT-402a` T1 chart cover; `CT-402b` T2+T3 continuation adjacency and
    component closure; `CT-402c` T4 projection coverage + T5 junction
    isolation. (was one 3-obligation packet)
  - `CT-403a` derivative enclosures through order k+1 with seam
    splitting (P9); `CT-403b` approximant + certified remainder (P10) at
    k=1; `CT-403c` tube flux bound (P11); `CT-403d` k=3 upgrade (a
    parameter change once the k=1 machinery is proven). (was one packet)
  - `CT-404`, `CT-406` unchanged (single obligations).
  - `CT-405a` polynomial Green-contour resolved flux (P13 exact path);
    `CT-405b` rational RI1 via the landed FHC-G1 reciprocal-power
    machinery. (was one packet with the hardest obligation buried in it)
- Every split packet inherits the same write-set lane (`atlas/` submodule
  per packet + its test file), so parallelism is preserved or improved.
- Expected program shape after re-scope: ~24 packets, 10k–17k LOC, but
  with per-packet `p(k) ≥ 0.8` instead of three packets at an estimated
  `p ≈ 0.4–0.6` — the expected-cost model says the split program is
  cheaper even before counting the parallelism gain.

## 6. Telemetry — refining the model

Every ledger row from this program on records: `obligations` (count from
the packet text), `loc_landed`, `attempts`, `worker_minutes` (from slot
events first-seen to RESULT), `fault` as today. The sizing law's
constants (`p1`, saturation size, ceremony floor) are re-fit from this
data at each program close — the law is a hypothesis under test, not
doctrine.

## 7. Codified rules for the authoring session

1. Write the obligation list FIRST; if it exceeds 3, split the packet in
   the same pass (each split must still compile-test green against the
   frozen contracts — that is what the shim is for).
2. Anchors: floor anchors (`min:`) by default; absolute `expect` only on
   files no sibling writes; `delta:` reserved for exact structural
   promises.
3. Judgements are pre-made and numbered; anything the worker must decide
   is either frozen into the packet or split out as its own packet.
4. Every packet's tests are scoped (`-p <crate> --test <file>`), and its
   RESULT lands at the worktree root (packet_lint enforces the phrasing).
5. When a packet rejects twice, the obligation — not the worker — is
   suspect: re-derive the claim by command, split it further, or write
   the theory doc first (that is how `docs/THEORY_GAPS_ADMISSION_CONTACT.md`
   came to exist).
