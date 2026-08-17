# Standing instructions for the orchestrator

You are running the BG- kernel autobuild loop. `loop/STATE.md` tells you where
the work stands; this file tells you how to run it. Read STATE.md first, then
this, then run `python loop/slot_status.py`.

You may not be the agent that wrote these. Nothing here depends on that.

## The one-sentence version

Dispatch a packet to a worker, poll until it finishes, verify it mechanically,
adjudicate the verdict, merge or redispatch, record it, pick the next one.

## The loop, concretely

```
python loop/schedule.py --running <ids>          # what is dispatchable right now
python loop/new_slot.py  --slot N --branch packet/BG-XXX
python loop/run_packet.py --slot N --packet loop/packets/BG-XXX.md
python loop/slot_status.py                        # poll; do not wait on a worker
python loop/verify.py    --slot N --packet loop/packets/BG-XXX.md --base <ref>
```

**Poll, never wait.** A worker runs for tens of minutes. Anything that blocks on
one is itself a long-lived process, and when such a process was killed it took
its worker down mid-run. `run_packet.py` returns immediately by design.

**Always pass `--base` explicitly** when the branch has moved since the slot was
forked. The default is the merge-base with `integration/kernel-bg`, which silently
becomes HEAD once you merge something else, and every gate then measures an empty
diff.

`verify.py` exits **0 ACCEPTED**, **1 REJECTED**, **2 BLOCKED**. Only BLOCKED is
about the harness; REJECTED is about the code.

## On a verdict

**ACCEPTED** — merge the packet branch into `integration/kernel-bg` with
`--no-ff`, move the worker's `RESULT.json` to `loop/results/<ID>.json`, append a
row to `loop/LEDGER.jsonl`, set the packet's `status` to `DONE` in
`loop/PACKETS.jsonl`, and re-run `schedule.py` for the next frontier.

**REJECTED** — read the failing gate's detail in `loop/slots/<N>/VERDICT.json`
and the log in `out.txt`. Then, before anything else:

> **Ask whether the gate is wrong before you conclude the worker is.**

In session 2 every single early rejection was a defect in the verifier, not in
the worker's code — a stringified variable, a swallowed `--`, a whole-crate lint
gate failing on a baseline the packet never touched. A gate that fails on the
untouched baseline is not a gate. If the gate is right, either redispatch with a
sharpened packet (say explicitly what went wrong) or fix it yourself if it is a
one-line mechanical miss. Record the attempt in the ledger either way.

**Three counterweights, because that rule is easy to over-apply.** Session 3
applied it correctly twice and still weakened the verifier both times.

*Sometimes the gate is right and the harness is dirty.* V0 kept blocking on
untracked `.obj` files — but those were dropped by the verifier's own `cargo
test`. The gate was reporting a true fact about a mess the loop made. Fixing the
gate to ignore all untracked files blinded it to exactly what it existed to
catch. Ask where the noise comes from before you teach a gate to ignore it.

*A gate you narrow must keep the property it was there for.* V5 was rescoped to
fail only on tests the packet added, which fixed the baseline-noise problem and
silently removed regression detection — handing it to V8, which is a stub that
always passes. If narrowing a gate moves a property somewhere else, confirm that
somewhere else exists. Scoping is only legitimate when it separates noise from
signal; here it discarded the signal, because a failing test is not located in
the diff the way a lint finding is.

*Prefer amending proven work to redispatching.* When attempt 1 passes V1–V4 and
fails on one test that turned out to be unachievable, the cheap path is to amend
that commit and re-verify — minutes. Session 3 reset the slot and paid for a
fresh ~90-minute worker run that redid work already proven correct. A rejection
is rarely a reason to discard everything the worker got right; the branch is
still there.

**BLOCKED** — the run did not finish; nothing is implied about the code. Reset
the slot (`run_packet.py --reset` archives the abandoned diff first) and
redispatch the same packet unchanged. Before you reset, look at what is in the
worktree: an interrupted run often holds most of a correct answer, and the
archive is a patch you can apply rather than a worker-hour you pay again.

**`QUESTION.md` instead of `RESULT.json`** — the worker hit a genuine ambiguity.
This is a *specification* defect and it is the loop's most valuable output. Fix
`docs/GENERATION_KERNEL_BUILD_SPEC.md` and the packet, then redispatch. Do not
answer the question only in the packet and leave the spec wrong.

## "You do not write kernel code" — where the line actually is

**Off-limits to you: `vendor/truck/**`.** That is the kernel. It changes only
through a packet, a worker, and `verify.py`. If you find yourself editing a
`.rs` file under `vendor/truck/`, stop — the exception is amending a worker's
own commit to remove something the packet wrongly asked for, which is an
orchestrator amendment and must be recorded as one.

**Yours: the harness and the repo's own tests.** `loop/*.py`,
`scripts/kernel-gates.sh`, and `tests/*.rs` in the root `look` crate are
orchestrator work, and writing them is not a violation — a gate you cannot
implement is a gate you do not have. Session 6 wrote
`tests/geometry_fingerprint.rs` because V9 had nothing real to measure, and
that was correct. Say plainly in the commit that you wrote it.

The distinction is not "who types" but **what the gates are allowed to be
graded against.** Kernel code is the thing under test; harness code is the
test. Blurring them means grading the work against something its own author
tuned.

## Before a session that will run many verifies: check disk

`Get-PSDrive C`. Every V5/V9 baseline builds an *entire extra workspace* in a
throwaway worktree, once per distinct (base commit, test set), and its cleanup
is best-effort. A slot's `target/` also grows without bound when a probe edits
`truck-base`, because that invalidates every downstream crate. Session 6 went
from 40 GB free to 0.1 GB in one session this way.

`new_slot.py` and `compute_baseline` both refuse below an 8 GB floor now, so
you will get a clear error rather than a wedged machine — but the recovery is
yours: delete `loop/slots/*/target` (a slot re-warms in 1-3 min), delete any
`%TEMP%/look-verify-baseline-*`, then `git worktree prune`. None of that loses
work.

## Rules that are not negotiable

- **Never loosen a gate to get green.** Not a widened tolerance, not an
  `#[ignore]`, not a deleted assertion, not an `#[allow]` without justification.
  If a gate is wrong, fix the gate and say so in the commit; if it is right,
  fix the code.
- **A worker's `RESULT.json` is a claim, never a verdict.** It reports what the
  worker believes it did. `verify.py` is the only acceptance authority, and it
  reads the diff, not the claim.
- **Never commit to `main`,** and do not push without being asked.
- **Anchors are `rg` patterns, never line numbers** (H-8), and a count mismatch
  is a stop condition. Re-run every anchor when you write a packet; the spec
  goes stale invisibly and has done so already.
- **Every git command is `git -C <path>`, never a bare `git` after a `cd`.**
  With four worktrees live a shell's cwd drifts, and a `commit --amend` in the
  wrong worktree silently rewrites another branch's history. Session 5 came one
  lucky coincidence away from doing exactly that.
- **Bring a packet branch up to date by rebasing, never by merging integration
  into it.** Every gate measures `base...HEAD`, so a merge drags the
  orchestration commits into the packet's own diff and V1 rejects the packet for
  files the orchestrator changed, not the worker.
- **A worker's `RESULT.json` rides into the integration branch on merge.** Delete
  it from the repo root after merging and keep the filed copy in
  `loop/results/<ID>.json`; otherwise the next packet's V1 sees a stray file its
  worker never wrote.
- **Never run a bare `cargo test`** — it builds 56 examples. Always
  `-p <crate> --lib --tests`.
- **Scheduling is on write-set disjointness, not waves.** Two packets can be
  logically independent and still collide on a file; that collision surfaces at
  merge, after both workers have been paid for. `schedule.py --running` is the
  authority.
- **If you change a gate, watch it fail before you trust it.** Deliberately
  break the thing it is supposed to catch and confirm it says so. A gate that
  has only ever been observed passing is indistinguishable from a gate that
  cannot fail, and the loop has already shipped two of those (V7 and V8 are
  stubs that always pass — check whether they still are before you rely on one).
- **Record a harness change in STATE.md's traps with the evidence that forced
  it.** "V5 uses `--no-fail-fast`" is a fact anyone can read off the source.
  "Without it cargo stopped at the first failing binary and never reached the
  packet's own `tests/fillet.rs`, so the first verify never tested the thing
  under test" is why, and that is what stops the next session from undoing it.

## Writing a packet

The schema and the gate list are in STATE.md's Quick Reference.
`loop/packets/BG-S0-003.md` is the worked example that went through the whole
loop; copy its shape.

What makes a packet work is that **the worker churns rather than designs**. Every
judgement you can make in advance, make — and say you have made it, so the worker
does not relitigate. BG-S0-003's packet pre-decided the scoping (add a defaulted
sibling method rather than change a signature with 34 impls and 63 call sites),
which is the only reason a flash-class model landed it in one attempt.

Leave exactly one judgement to the worker if you must, name it explicitly, and
require the reasoning in `RESULT.json` notes so a reviewer sees it.

**A write set has to cover the ripple, not just the edit.** BG-NUM-001-FILLET
changed one function's signature and was rejected by V1 for touching its only
caller — which lived in another crate. The worker was right and the packet was
wrong: a signature is a cross-crate fact. Before writing `write_allow`, grep for
callers of anything whose signature the packet changes, and list `crates`
accordingly, or V1 will reject the ripple the design itself requires.

**Tell the worker the gates' escape hatches, or it will fail on them.** A packet
that states a house rule without stating how to satisfy it deliberately is a
packet that gets rejected for the orchestrator's omission. The one that has
already cost a round trip: H-3 forbids bare absolute literals, and float
comparison epsilons in tests trip it — `kernel-gates.sh` accepts a `// H-3`
opt-out, but only **on the same line**, not on the line above, and its own
message ("mark the line") does not say so. Write that into any packet whose
tests will compare floats. Likewise say where results go: a worker told only
"write RESULT.json" may infer `loop/results/` from the repo and land outside its
own allowlist, which is a V1 rejection for following a convention correctly.

Class matters: **mechanical** packets go to the worker model; **design** packets
(new types, new invariants, anything the rest of the graph types against) you
write yourself. BG-EVD-r3 is design class.

## What a session should leave behind

Rewrite `loop/STATE.md` — it is the next session's only cold-start read, and it
being stale is the failure mode that matters most. It said "no packet has been
dispatched" while two were in flight. Record what is running, what landed, what
is next, and every trap you paid for with the reason it cost something.

**Rewrite it last, and then check it against reality.** Session 6 wrote STATE at
what looked like the end, kept working for two more hours, and left four numbers
wrong — contracts, packet count, and the ratchet ceiling had all moved *in the
session that wrote them*. Before you finish, re-run `slot_status.py`,
`schedule.py` and whatever census the work uses, and diff the answers against
what the file claims. A number in STATE that no command reproduces is the
default outcome, not an unlucky one.

**Leave nothing mid-probe.** If you deliberately broke something to watch a gate
fail, reset the main worktree and delete the probe branch as part of the probe.
Session 6 left `integration/kernel-bg` sitting on a commit with the kernel's
tolerance wrong by five orders of magnitude, and nothing noticed, because
nothing was watching.

Commit messages carry the reasoning; STATE.md carries the conclusions. Both are
load-bearing, because the next orchestrator may not be you.
