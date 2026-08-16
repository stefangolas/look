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

**BLOCKED** — the run did not finish; nothing is implied about the code. Reset
the slot (`run_packet.py --reset` archives the abandoned diff first) and
redispatch the same packet unchanged.

**`QUESTION.md` instead of `RESULT.json`** — the worker hit a genuine ambiguity.
This is a *specification* defect and it is the loop's most valuable output. Fix
`docs/GENERATION_KERNEL_BUILD_SPEC.md` and the packet, then redispatch. Do not
answer the question only in the packet and leave the spec wrong.

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
- **Never run a bare `cargo test`** — it builds 56 examples. Always
  `-p <crate> --lib --tests`.
- **Scheduling is on write-set disjointness, not waves.** Two packets can be
  logically independent and still collide on a file; that collision surfaces at
  merge, after both workers have been paid for. `schedule.py --running` is the
  authority.

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

Class matters: **mechanical** packets go to the worker model; **design** packets
(new types, new invariants, anything the rest of the graph types against) you
write yourself. BG-EVD-r3 is design class.

## What a session should leave behind

Rewrite `loop/STATE.md` — it is the next session's only cold-start read, and it
being stale is the failure mode that matters most. It said "no packet has been
dispatched" while two were in flight. Record what is running, what landed, what
is next, and every trap you paid for with the reason it cost something.

Commit messages carry the reasoning; STATE.md carries the conclusions. Both are
load-bearing, because the next orchestrator may not be you.
