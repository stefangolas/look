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
has a complete, V1–V4-clean conversion sitting on a branch** and is closer to
landing than the contract count suggests — see "Pick up here".

Session 3 landed no contract and was still worth it: it found that `cargo test`
was stopping before it reached the packet's own test file, filed the loop's
first spec follow-up (**BG-S0-002-r2**), and proved the dispatch path had been
silently losing its event stream. Session 4 corrected two gates that session 3
had weakened while fixing the first of those.

## Pick up here

**Slot 0 is IDLE.** Attempt 2 of BG-S0-002 was interrupted mid-run; its work is
archived at `loop/slots/0/attempt2-interrupted.patch` (788 lines) and the
worktree still holds it uncommitted at base `b06a535`.

The cheap path is not a third worker run. **Attempt 1 already passed V1-V4** and
is preserved on branch `packet/BG-S0-002-attempt1` (commit `3c24608`, 718
insertions across the six files): a complete, lint-clean, house-rule-clean
mechanical conversion whose only defect was the fourth test, which the spec gap
has since removed from the packet. Amending that commit to drop
`fillet_at_chart_pole_refuses` and re-verifying is minutes of work against ~90
for a fresh dispatch.

1. Decide between amending `3c24608` and redispatching, then verify:
   `python loop/verify.py --slot 0 --packet loop/packets/BG-S0-002.md --base b06a535`.
   **Pass `--base b06a535` explicitly** — the slot is forked there and the
   default merge-base now resolves to HEAD because the branch moved on.
   V5 compares against a cached baseline run at `b06a535`, so pre-existing
   failures (`healing::tests::step_import`, `tests/fillet.rs::complex_surface`)
   are reported as ignored noise; a FAIL names a test that newly fails.
2. On ACCEPTED: merge `packet/BG-S0-002` into `integration/kernel-bg` `--no-ff`,
   move `RESULT.json` to `loop/results/BG-S0-002.json`, append the closing
   ledger row, set `status: DONE` in `loop/PACKETS.jsonl`. That releases
   `truck-base/src/evidence.rs` and unblocks **BG-EVD-r3**.
3. Before dispatching anything, run `python loop/selftest_dispatch.py` (~40s).
   The dispatch path has now broken in three different ways, each presenting as
   silence rather than an error; that selftest is how you find out in forty
   seconds instead of ninety minutes.

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
| `baa2dfa` | **session 3:** V5 gains `--no-fail-fast`; BG-S0-002 test 3 deferred to **BG-S0-002-r2** (spec + packet amended to 3 tests) |
| `88b93ee` | V5 compares against a cached baseline run at the base commit; V0 allow-lists `*.obj` instead of ignoring every untracked file |
| `aa6e31a` | the dispatch actually streams and actually survives; `selftest_dispatch.py` proves it in 40s |
| `fb697ea` | three counterweights to "ask whether the gate is wrong" in ORCHESTRATOR.md |

Contracts discharged: **BG-S0-001** and **BG-S0-003**. BG-S0-002 unlanded. **BG-S0-002-r2** filed (design class — the chart-pole runtime test;
`create_pcurve_edge` called directly with a constructed degenerate surface, no
dependency on hardening `rbf_surface/algo.rs`).

## The commands

`slot_status.py` prints each slot's branch and short HEAD (`git=branch@sha`),
flagged `(=base, no work)` when HEAD is still sitting at the slot's fork
point -- so which branch actually holds a packet's best attempt is read off
the slot, not reconstructed from prose the way BG-S0-002's attempt1 branch
had to be. `run_packet.py` records the branch it dispatched onto in
`loop/slots/<N>/worker.branch`, and `verify.py` records the branch and exact
commit it judged in `VERDICT.json`.

```
python loop/slot_status.py                     # what is every slot doing (poll this)
python loop/slot_status.py --kill-stalled       # reap anything silent for 12 min
python loop/new_slot.py  --slot N --branch packet/BG-XXX
python loop/run_packet.py --slot N --packet loop/packets/BG-XXX.md   # returns at once
python loop/verify.py    --slot N --packet loop/packets/BG-XXX.md [--base <ref>]
python loop/schedule.py --running BG-A,BG-B                          # the frontier
python loop/selftest_dispatch.py                # prove the dispatch works (~40s)
```

Dispatch is fire-and-forget by design: a worker runs for tens of minutes, and
anything that waits on it is a long-lived process that can be killed — when one
was, it took its worker down mid-run. Poll instead. Run `verify.py` with a long
timeout (or in the background); it takes about four minutes on a warm slot, more
with V5's `--no-fail-fast` running every test binary.

`verify.py` exits **0 ACCEPTED**, **1 REJECTED** (the work is wrong), **2
BLOCKED** (the run never finished — reset the worktree and redispatch;
nothing is implied about the worker's code), or **3 PARTIAL** (`--only` was
used — see below). Environment: Windows, `cargo`, and Git Bash at
`C:\Program Files\Git\bin\bash.exe`. The harness itself is Python 3
stdlib-only (`loop/*.py`).

`verify.py --slot N --packet ... --base <ref> --only V3,V5` runs just those
gates and reports the rest `SKIP`; V0 preflight always runs regardless,
since every other gate reads the diff between base and HEAD and that's
meaningless if the run didn't finish. This exists for the amend-and-verify
path: editing one test file to re-check V5 used to pay for a full 4-6 minute
cycle (V2, V3, V4, and the whole suite) even though nothing else changed. A
partial run can never report ACCEPTED — its verdict is always `PARTIAL`
(exit 3) no matter what the requested gates found, because acceptance is a
claim about the whole packet and nothing about re-checking one gate tells
you the others still hold.

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
- **`DETACHED_PROCESS` silences the very worker it is meant to free.** Measured
  across eight flag combinations: every one containing it produced zero bytes of
  output, every one without it streamed. A batch file with no console cannot get
  its own child's output onto an inherited handle, and `opencode` is a `.cmd`
  shim, so the process doing the work is always a grandchild. The dispatch uses
  `CREATE_NO_WINDOW | CREATE_NEW_PROCESS_GROUP | CREATE_BREAKAWAY_FROM_JOB` and
  writes its redirect into a command file, where `>` binds the whole chain.
  Breakaway is what actually makes a worker outlive its parent — a harness kills
  the tool call's *job*, which detachment never addressed. **Run
  `python loop/selftest_dispatch.py` after touching any of this.**
- **A test failure is not located in the diff.** V3 scopes clippy to added lines
  because a lint finding sits on a line; applying the same move to V5 (fail only
  on tests the packet added) does not filter the noise, it discards the signal,
  and it hands regressions to V8, which is a stub. V5 instead runs the suite once
  at the base commit, caches the result under `loop/baselines/`, and fails on
  anything that newly fails, disappears, or becomes `#[ignore]`d.
- **The verifier dirties the worktree it is judging.** `cargo test` drops `.obj`
  mesh dumps in the worktree, which V0 then read as an unfinished run. The fix is
  to allow-list that specific artifact, not to make V0 blind to untracked files —
  an uncommitted new `.rs` is exactly what V0 exists to catch, and it is *not*
  caught by V1/V6, which read the committed diff.
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

`RESULT.json` is the worker's terminal claim, e.g.
`{"id":"BG-S0-002","status":"DONE","contracts":[...],"tests_added":3,...}`.
The orchestrator may amend it after the fact -- see `loop/results/BG-S0-002.json`
for the real case: the worker returned `SPEC_GAP` correctly (one required test
was unreachable through anything in write_allow), the spec and packet were
sharpened to drop that test, and the orchestrator amended the worker's commit
rather than paying for a fresh ~90-minute dispatch. Any such amendment **must**
carry `"amended_by": "orchestrator"` and **must** keep the worker's original
reasoning verbatim under `notes` rather than overwrite it -- that reasoning is
often the only record of *why* a test or a behaviour is absent, and losing it
to a tidied-up summary destroys the one thing a future reader needs.
`verify.py` reads this field: when the worktree's `RESULT.json` carries
`amended_by`, it appears in the V0 preflight detail line and at the top level
of `VERDICT.json`, so a verdict can never silently present amended work as
untouched worker output.

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
