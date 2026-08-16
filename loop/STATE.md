# Autobuild loop — STATE

Rewritten at the end of every session; the state above "Quick reference" is
capped at ~120 lines, and the reference below it is stable and does not count
against that. If you are picking this up cold, read **this file, then
[`loop/ORCHESTRATOR.md`](ORCHESTRATOR.md) for how to run the loop, then
`python loop/slot_status.py`** — nothing else. Do not read `LEDGER.jsonl` whole.

Updated 2026-08-16, end of session 3. Branch: `integration/kernel-bg`. Nothing
from the loop has reached `main` and nothing has been pushed.

## What this is, if you have never seen it

`vendor/truck/` is a vendored CAD kernel this repo owns. A formal specification,
`docs/GENERATION_KERNEL_BUILD_SPEC.md`, lists ~56 numbered contract items
(`BG-S0-*`, `BG-EVD-*`, `BG-TOL-*`, …) that harden it — replacing panics with
refusals carrying evidence, giving tolerances a model, certifying enclosures.
This loop discharges those items with LLM workers instead of by hand.

**You are the orchestrator.** You write work packets, schedule them, adjudicate
verification, and amend the spec. **You do not write the kernel code.** A packet
is dispatched to a worker (deepseek v4 flash via opencode) that gets one file —
the packet — and one git worktree, and gets no say in whether its work is
accepted. `loop/verify.py` is the only acceptance authority; a worker's
`RESULT.json` is a claim, never a verdict.

Three documents define the rest, and you should read them in this order when a
specific need arises rather than upfront:

- [`docs/KERNEL_AUTOBUILD_LOOP.md`](../docs/KERNEL_AUTOBUILD_LOOP.md) — the loop
  design: packet schema (§4), the V-gates (§5), context budget (§3), disk (§7).
- [`docs/GENERATION_KERNEL_BUILD_SPEC.md`](../docs/GENERATION_KERNEL_BUILD_SPEC.md)
  — the contract items themselves and **house rules H-1..H-8**, which every
  packet restates and every worker must obey. H-8 is the one that bites: anchors
  are `rg` patterns and symbol names, never line numbers, and a count mismatch
  is a stop condition, not a nuisance.
- `loop/packets/BG-S0-002.md` and `BG-S0-003.md` — two worked examples. Copy
  their shape; BG-S0-003 is the one that has been through the full loop.

The loop is a **build** loop, not a search loop: acceptance is mechanical and
deterministic, so the verifier does the job an objective function would. Nothing
here is scored, tuned, or sampled.

## Where we are

The harness works end to end. **BG-S0-003 was accepted on all nine gates and is
merged** (`c8acab6`), still the only packet through the whole loop. **BG-S0-002
is on attempt 2**, RUNNING on slot 0 (pid 34028) with a sharpened 3-test packet.
Attempt 1 was correct work rejected on two harness defects and one genuine spec
gap — all three found by running a real packet through the loop and all fixed
this session (commit `baa2dfa`). The worker's attempt-1 mechanical conversion
(A1/A2/A3) passed V1–V4; only its test 3 was unachievable, and that is now the
loop's first filed spec follow-up, **BG-S0-002-r2**.

Session 3 did not land a contract. It fixed the two gates that were lying about
BG-S0-002 and resolved the spec gap that stopped the worker. That is the work
that unblocks BG-S0-002 itself.

## Pick up here

1. `python loop/slot_status.py`. **Slot 0 is RUNNING attempt 2 of BG-S0-002**
   (pid 34028, dispatched end of session 3 against the sharpened packet, base
   `b06a535`). A fillet run is ~90 min.
   - **RUNNING** — poll. Do not wait on it.
   - **FINISHED** — verify it:
     `python loop/verify.py --slot 0 --packet loop/packets/BG-S0-002.md --base b06a535`.
     **Pass `--base b06a535` explicitly** — the slot is forked there and the
     default merge-base now resolves to HEAD because the branch moved. Then
     follow ORCHESTRATOR.md's verdict handling. The gate is fixed (see "Traps"):
     V5 is diff-scoped to *added* test fns and notes (does not reject on)
     pre-existing baseline failures — `healing::tests::step_import` (missing
     STEP data) and `tests/fillet.rs::complex_surface` (triangulates to
     `Irregular`). If V5 FAILs it names the failing added test; that is the
     worker's defect, not baseline noise. If V5 PASSes with a "pre-existing
     baseline failure(s) ignored" note, that is expected.
   - **STALLED** — `python loop/slot_status.py --kill-stalled`, then
     `python loop/run_packet.py --slot 0 --packet loop/packets/BG-S0-002.md --reset`
     (the packet is already sharpened; redispatch unchanged).
2. On ACCEPTED: merge `packet/BG-S0-002` into `integration/kernel-bg` `--no-ff`,
   move `RESULT.json` to `loop/results/BG-S0-002.json`, append the closing
   ledger row, set `status: DONE` in `loop/PACKETS.jsonl`. That releases
   `truck-base/src/evidence.rs` and unblocks **BG-EVD-r3**.
3. Attempt 1's correct conversion is preserved on branch
   `packet/BG-S0-002-attempt1` (commit `3c24608`) — reference if attempt 2
   diverges; do not redo work the worker already proved.

## Landed

| commit | what |
|---|---|
| `da72cd5` | vendored truck at `vendor/truck/` (12 crates) + kernel gates + evidence module |
| `fddc62a` | vendored crates are workspace members — without this `cargo test -p <crate>` (V5) cannot run |
| `65450b3` `ca22bc4` `a5660c3` | loop scaffolding, first packets, the 56-packet DAG |
| `b06a535` | three baseline clippy defects fixed (the slot-0 fork point) |
| `ed35879` `e927384` `da1b174` `978b902` `d1f9c5b` `8dca941` | the verifier and dispatcher, made to actually work |
| `c8acab6` | **BG-S0-003** — the first packet through the whole loop |
| `4cc5aca` | harness ported to stdlib-only Python; the four `.ps1` scripts are gone |
| `baa2dfa` | **session 3:** V5 diff-scoped to added tests + `--no-fail-fast`; V0 ignores untracked artifacts; BG-S0-002 test 3 deferred to **BG-S0-002-r2** (spec + packet amended to 3 tests) |

Contracts discharged: **BG-S0-001** and **BG-S0-003**. BG-S0-002 attempt 2 in
flight. **BG-S0-002-r2** filed (design class — the chart-pole runtime test;
`create_pcurve_edge` called directly with a constructed degenerate surface, no
dependency on hardening `rbf_surface/algo.rs`).

## The commands

```
python loop/slot_status.py                     # what is every slot doing (poll this)
python loop/slot_status.py --kill-stalled       # reap anything silent for 12 min
python loop/new_slot.py  --slot N --branch packet/BG-XXX
python loop/run_packet.py --slot N --packet loop/packets/BG-XXX.md   # returns at once
python loop/verify.py    --slot N --packet loop/packets/BG-XXX.md [--base <ref>]
python loop/schedule.py --running BG-A,BG-B                          # the frontier
```

Dispatch is fire-and-forget by design: a worker runs for tens of minutes, and
anything that waits on it is a long-lived process that can be killed — when one
was, it took its worker down mid-run. Poll instead. Run `verify.py` with a long
timeout (or in the background); it takes about four minutes on a warm slot, more
with V5's `--no-fail-fast` running every test binary.

`verify.py` exits **0 ACCEPTED**, **1 REJECTED** (the work is wrong), or
**2 BLOCKED** (the run never finished — reset the worktree and redispatch;
nothing is implied about the worker's code). Environment: Windows, `cargo`,
and Git Bash at `C:\Program Files\Git\bin\bash.exe`. The harness itself is
Python 3 stdlib-only (`loop/*.py`).

## Next actions, in order

1. Land BG-S0-002 from slot 0 — see "Pick up here" above.
2. **BG-EVD-r3** — design class, so the orchestrator writes it, not the worker
   model. `Modulus` becomes a struct with `domain` + shape-derived
   `is_subadditive` and a `propagate` recurrence; `Refusal` gains
   `ForwardToleranceExceeded`; `ModulusShape` gains `Pole`. It is the neck of
   the whole graph: everything in W2 onward types against it. It cannot start
   until BG-S0-002 releases `truck-base/src/evidence.rs`.
3. Split the `truck-topology/src/**` shard of BG-TOL-001 by module. As one
   packet it single-handedly blocks all eight BG-INV checkers.
4. Write `gen_packet.py`, which must re-run every anchor's `rg` at generation
   time and refuse to emit on a count mismatch.
5. Write the **BG-S0-002-r2** packet (design) — the deferred chart-pole runtime
   test. It can be written any time after BG-S0-002 lands; it does not block
   the graph.

## The parallelism picture

56 packets: 35 mechanical, 13 design, 8 wide-mechanical. Scheduling is on
**write-set disjointness**, not waves — two packets can be logically independent
and still collide on a file, and that collision surfaces at merge, after both
workers have been paid for.

The frontier is **1 packet wide until BG-EVD-r3 lands**, then opens to 22
mutually disjoint packets at the W4 frontier. More slots buy nothing before
that. A warm slot costs 0.90 GB and 1.2 min, so from W4 on, slots — not
dependencies — are the binding constraint.

## Traps, each one paid for

- **A gate that fails on the untouched baseline is not a gate.** The vendored
  tree is nowhere near clippy-clean (truck-meshalgo ~93 lints,
  `revolved_curve.rs:694` "items after a test module", `geometry.rs:294`
  `borrowed_box`), and its test suite is not clean either
  (`healing::tests::step_import` needs a STEP data file absent on this machine;
  `tests/fillet.rs::complex_surface` triangulates to `Irregular`). V3 is scoped
  to the **lines the diff added**; V5 is now scoped to the **test fns the diff
  added** (with `--no-fail-fast` so every binary runs). A whole-crate pass/fail
  rejects every packet for the baseline's defects — the same as no gate.
- **V5 must use `--no-fail-fast`.** Without it, cargo stops at the first failing
  test binary and never reaches the packet's own `tests/*.rs` — BG-S0-002's
  first verify ran every crate *except* `fillet.rs`, so it could not have
  caught the packet's own tests failing.
- **verify.py dirties its own worktree.** `cargo test` (V5) drops `.obj` mesh
  dumps and logs into the worktree as untracked files. V0 used to count those
  as "uncommitted changes" and BLOCK the next verify run. V0 now ignores
  untracked files; an uncommitted new *source* file is still caught by V1/V6,
  which read the committed diff.
- **A bare `bash` is the WSL stub**, which fails with `execvpe(/bin/bash)` —
  an exit 1 that reads as a house-rule violation. V4 hardcodes Git Bash.
- **`opencode` on PATH is a `.ps1`/`.cmd` shim** whose command line caps at
  8191 chars, under a 9 KB packet. The packet is copied into the worktree as
  `PACKET.md` and the prompt points at it. Both failures presented as an empty
  event stream and exit 0.
- **Workers hang.** One sat 45 minutes mid-step on an API call that never
  returned, holding a slot and its write set, producing nothing. CPU time
  cannot tell that apart from a worker waiting on the model; only the growth of
  `events.jsonl` can. `slot_status.py` reaps anything silent for 12 min.
- **An interrupted run reads as a perfect one.** Every gate measures the diff
  between base and HEAD, and a worker that dies mid-packet leaves its edits
  *uncommitted* — an empty diff, which passes V1–V6 on nothing and reports
  ACCEPTED. V0 preflight exists for exactly this. Workers survive a brief
  network drop on their own; they do not survive their parent process killed.
- **The spec goes stale invisibly.** BG-S0-001 was landed while the spec still
  listed it open with an anchor count of 6 that is now 0. Re-run every anchor
  when you touch a packet.
- **A spec gap is the loop's most valuable output, not a failure.** BG-S0-002's
  worker proved test 3 unreachable (`create_pcurve_edge`'s
  `UnsupportedEnvelope(ChartDegenerate)` path is blocked by out-of-scope
  `rbf_surface/algo.rs` `mat.invert().unwrap()` sites at lines 815/824/834/847/
  925/934/944/957) and stopped with `QUESTION.md`. The fix was to amend the
  spec (defer the runtime test to BG-S0-002-r2) and the packet (3 tests), not
  to weaken the gate or fake the test. The A2 mechanical conversion stays
  required and is verified by V3/V4.
- **`autotests = false` in truck-polymesh.** A new test file there needs an
  explicit `[[test]]` entry or it silently never runs. V6 flags this.
- **`truck_base::evidence`, not `truck_evidence`.** The module lives in
  truck-base to avoid a geotrait→evidence cycle; truck-evidence re-exports it.
- **CI gates are still vacuous.** `kernel-gates.sh` is diff-scoped and
  `origin/main` has no `vendor/truck/`, so CI passes on nothing. Packet
  verification is unaffected — its baseline is the branch tip.

## Quick reference — enough to write and judge a packet without another file

A packet is one markdown file whose YAML front block the verifier parses. Only
these fields are read mechanically; everything else in the file is prose aimed
at the worker.

```yaml
id:          BG-XX-000                     # contract item, one per packet
class:       mechanical | design | wide-mechanical
crates:      [truck-geometry, truck-base]  # cargo package names, not paths
write_allow:                               # repo-relative; V1 fails on anything else
  - vendor/truck/truck-geometry/src/specifieds/cone.rs
read_allow:  [...]                         # advisory; not enforced
tests_required:                            # V6 matches these against the diff
  - cone_apex_refuses
budget:      {turns: 40, ctx_tokens: 100000}
```

The prose sections that make a packet work, in the order they earn their keep:
**Problem** (one paragraph, why this is reachable from untrusted geometry);
**Anchors** (a table of `rg` patterns with exact expected counts, verified the
day the packet is written — H-8); **decisions already made for you** (every
judgement you can pre-make, so the worker churns rather than designs);
**Template** (the nearest landed diff to copy — BG-S0-001's work in
`truck-modeling/src/geometry.rs` is the house pattern); **Tests required**;
**Done when** (the exact commands — run touched `--test <stem>` targets, not
`--lib --tests`, on this tree); **Forbidden**; **Stop conditions**
(`ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`) and the `RESULT.json` shape.

The gates, in the order `verify.py` runs them:

| gate | asks |
|---|---|
| V0 preflight | did the run finish — commit past base, clean *tracked* tree, RESULT.json or QUESTION.md |
| V1 scope | is every changed file in `write_allow` |
| V2 build | `cargo check --locked -p <crates>` |
| V3 lint | `cargo fmt --check`, then clippy findings **on the added lines only** |
| V4 house rules | `scripts/kernel-gates.sh <base>` — H-1/H-3/H-4, diff-scoped |
| V5 tests | `cargo test -p <crates> --lib --tests --no-fail-fast`, FAIL only on **added** test fns |
| V6 test-reality | does every `tests_required` name appear as a real test fn in the diff |
| V7 mutation | stub — always passes |
| V8 no-regression | stub — always passes |

## Open questions

- V7 (mutation spot-check) and V8 (no-regression) are always-pass stubs. V7
  needs a packet field naming the negative test; V8 needs ledger state. V8 is
  the right home for "a packet broke a pre-existing test" — V5 deliberately
  does not catch that, only added-test failures, so it never false-rejects on
  the baseline.
- V6 matches test names by keyword overlap, not exactly. Tighten when
  `gen_packet.py` fixes a naming convention.
- `opencode/deepseek-v4-flash-free` would run W4's 23 packets at no API cost.
  Untested against concurrent workers.
- BG-S0-002-r2 needs a constructed degenerate surface (a parametric pole where
  `uder ∥ vder`) to exercise `create_pcurve_edge` directly. The fixture is
  design work; the orchestrator writes that packet.
