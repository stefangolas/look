# Autobuild loop — STATE

Rewritten at the end of every session. Capped at ~120 lines. If you are picking
this up cold, read **this file, then run `loop\slot-status.ps1`** — nothing
else. Do not read `LEDGER.jsonl` whole.

Updated 2026-08-16, end of session 2. Branch: `integration/kernel-bg`. Nothing
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
accepted. `loop/verify.ps1` is the only acceptance authority; a worker's
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

The harness works end to end. Two packets have been dispatched to deepseek v4
flash through opencode; **BG-S0-003 came back DONE and is in verification**,
**BG-S0-002 is running in slot 0**. Session 2 was spent almost entirely on
harness defects, every one of which was found by running a real packet through
it rather than by reading the scripts.

## Landed

| commit | what |
|---|---|
| `da72cd5` | vendored truck at `vendor/truck/` (12 crates) + kernel gates + evidence module |
| `fddc62a` | vendored crates are workspace members — without this `cargo test -p <crate>` (V5) cannot run at all |
| `65450b3` `ca22bc4` `a5660c3` | loop scaffolding, first packets, the 56-packet DAG |
| `b06a535` | three baseline clippy defects fixed (see "the baseline is not clean" below) |
| `ed35879` `e927384` `da1b174` `978b902` `d1f9c5b` | the verifier and dispatcher, made to actually work |

Contracts discharged: **BG-S0-001** only. It remains the reference answer every
mechanical packet copies.

## The commands

```powershell
.\loop\slot-status.ps1                 # what is every slot doing (poll this)
.\loop\slot-status.ps1 -KillStalled    # reap anything silent for 12 min
.\loop\new-slot.ps1  -Slot N -Branch packet/BG-XXX
.\loop\run-packet.ps1 -Slot N -Packet loop/packets/BG-XXX.md   # returns at once
.\loop\verify.ps1    -Slot N -Packet loop/packets/BG-XXX.md    # the only authority
python loop\schedule.py --running BG-A,BG-B                    # the frontier
```

Dispatch is fire-and-forget by design: a worker runs for tens of minutes, and
anything that waits on it is a long-lived process that can be killed — when one
was, it took its worker down mid-run. Poll instead. Run `verify.ps1` in the
background; it takes about four minutes on a warm slot.

`verify.ps1` exits **0 ACCEPTED**, **1 REJECTED** (the work is wrong), or
**2 BLOCKED** (the run never finished — reset the worktree and redispatch;
nothing is implied about the worker's code). Environment: Windows, PowerShell
5.1, `cargo`, and Git Bash at `C:\Program Files\Git\bin\bash.exe`.

## Next actions, in order

1. Finish verifying BG-S0-003 in slot 1, then merge `packet/BG-S0-003` into
   `integration/kernel-bg` and write the first `LEDGER.jsonl` row.
2. Same for BG-S0-002 in slot 0 when it lands.
3. **BG-EVD-r3** — design class, so Claude writes it, not deepseek. `Modulus`
   becomes a struct with `domain` + shape-derived `is_subadditive` and a
   `propagate` recurrence; `Refusal` gains `ForwardToleranceExceeded`;
   `ModulusShape` gains `Pole`. It is the neck of the whole graph: everything
   in W2 onward types against it. It cannot start until BG-S0-002 releases
   `truck-base/src/evidence.rs`.
4. Split the `truck-topology/src/**` shard of BG-TOL-001 by module. As one
   packet it single-handedly blocks all eight BG-INV checkers.
5. Write `gen-packet.ps1`, which must re-run every anchor's `rg` at generation
   time and refuse to emit on a count mismatch.

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
  tree is nowhere near clippy-clean: truck-meshalgo carries ~93 lints,
  `revolved_curve.rs` fails before a run reaches truck-shapeops, and
  `geometry.rs:294` trips `borrowed_box` on a line BG-S0-001 wrote. V3 is
  therefore scoped to the **lines the diff added** — file-level scoping is not
  enough, it rejects a packet for its predecessor's lint in a file it edits.
- **PowerShell 5.1 silently breaks the verifier three ways.** A local named
  `$packet` inherits the `[string]$Packet` parameter's type constraint and
  stringifies the object assigned to it. `*>>` on a native command aborts the
  run on cargo's first *progress* line under `ErrorActionPreference = 'Stop'`.
  A literal `--` is eaten before cargo sees it.
- **A bare `bash` is the WSL stub**, which fails with `execvpe(/bin/bash)` —
  an exit 1 that reads as a house-rule violation. V4 hardcodes Git Bash.
- **`opencode` on PATH is a `.ps1` shim** that `Start-Process` cannot execute,
  and the `.cmd` shim caps at 8191 characters, well under a 9 KB packet. The
  packet is copied into the worktree as `PACKET.md` and the prompt points at
  it. Both failures presented as an empty event stream and exit 0.
- **Workers hang.** One sat 45 minutes mid-step on an API call that never
  returned, holding a slot and its write set, producing nothing. CPU time
  cannot tell that apart from a worker waiting on the model; only the growth of
  `events.jsonl` can.
- **An interrupted run reads as a perfect one.** Every gate measures the diff
  between the base and HEAD, and a worker that dies mid-packet leaves its edits
  *uncommitted* — an empty diff, which passes V1 through V6 on nothing and
  reports ACCEPTED. V0 preflight exists for exactly this and returns BLOCKED.
  Workers survive a brief network drop on their own; they do not survive their
  parent process being killed.
- **The spec goes stale invisibly.** BG-S0-001 was already landed while the
  spec still listed it open with an anchor count of 6 that is now 0.
- **`autotests = false` in truck-polymesh.** A new test file there needs an
  explicit `[[test]]` entry or it silently never runs.
- **`truck_base::evidence`, not `truck_evidence`.** The module lives in
  truck-base to avoid a geotrait→evidence cycle; truck-evidence re-exports it.
- **CI gates are still vacuous.** `kernel-gates.sh` is diff-scoped and
  `origin/main` has no `vendor/truck/`, so CI passes on nothing. Packet
  verification is unaffected — its baseline is the branch tip.

## Open questions

- V7 (mutation spot-check) and V8 (no-regression) are always-pass stubs. V7
  needs a packet field naming the negative test; V8 needs ledger state.
- V6 matches test names by keyword overlap, not exactly. Tighten when
  `gen-packet.ps1` fixes a naming convention.
- `opencode/deepseek-v4-flash-free` would run W4's 23 packets at no API cost.
  Untested against concurrent workers.
