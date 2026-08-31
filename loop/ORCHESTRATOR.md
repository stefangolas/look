# Standing instructions for the orchestrator

You are running the BG- kernel autobuild loop. `loop/STATE.md` tells you where
the work stands; this file tells you how to run it. Read STATE.md first, then
this, then run `python loop/slot_status.py`.

You may not be the agent that wrote these. Nothing here depends on that.

## The current program

The base loop (76/76), BG-AUDIT-001 (17/17), the solver family, and the
build123d coverage program (P1–P12) are FINISHED. The next program is the
**constructive geometry kernel** — `docs/CONSTRUCTIVE_GEOMETRY_PLAN.md` is the
approved design and books the contract (§3), packet list, write sets, and
dependency graph (§4, §6). Dispatch `BG-CG-000-CONTRACT` first; everything in
the CG graph types against it. Concurrency is capped at ≤3 live packets over
the write-set-disjoint set; full-wave orchestration is deliberately NOT
planned (plan §5 — velocity recalibration). The pyo3 binding translation is
booked but DEFERRED behind the CG core. Solver-family and pyo3 packet
conventions (booked signatures, write-set disjointness) carry over unchanged.

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

## The one rule that catches most of it

> **Any claim a packet, a survey, or STATE.md makes about the tree must be
> re-derived by running a command before you act on it.**

Not "when it looks doubtful" — always, because these claims read as
authoritative and are wrong often enough to matter. In session 9 three separate
inputs were confidently wrong and only a command caught them:

- **STATE's own handoff.** It recorded the NURBS generic-bound split as ten
  migrating and ten deferred, naming nine specific lines. Resolving all twenty
  `model` rows to their enclosing `impl` gave **twelve deferred and eight
  migrating** — wrong in both directions. Two of the lines it listed as blocked
  (`bspcurve.rs:1102`, `:1112`) sit in an impl that *does* carry
  `MetricSpace<Metric = f64>`, and it missed four others entirely.
- **A reviewed survey's proposed rewrite.** It offered `ctx.near_points(...)`
  for `truck-geotrait/src/algo/curve.rs:66`, whose generic bound supplies no
  `MetricSpace`. It would not have compiled. Its near-identical sibling in
  `algo/surface.rs` takes that exact rewrite correctly.
- **A packet's own budget**, twice — MESHALGO's 11-against-10, and this
  session's two counters disagreeing with each other again.

The failure mode is not writing bad packets. It is **accepting a plausible
wrong answer**, and plausibility is exactly what a survey, a worker's
`RESULT.json`, and the previous session's summary are all optimised for.

## Recurring failure modes, each one hit more than once

**A green line can be dropping rows, not just stating something false.**
`packet_contexts` resolved a table's file heading with a bare-basename
`endswith`, so `curve.rs` also matched `polyline_curve.rs`; two candidates made
the heading ambiguous and **every row beneath it was silently skipped**, while
the tool printed "all checked claims hold". Two packets read low by one and two
contexts. Worse than a wrong count: rows that are skipped are never checked
against the tree at all, which is the only thing that function is for. When a
check's two halves are computed from different sources, assume they will
eventually disagree — and when a count surprises you, confirm the checker saw
every row before believing the number.

**GATE-4 counts `unscaled_legacy(` anywhere in the file, comments included.**
A `FIXME` that explains *why* a site cannot be migrated will mention the
constructor by name, inflate the ratchet by one, and make a deferral read as a
migration. Write the name without its parentheses in prose.

**`cd` persists across a compound command, and `-F` paths resolve against it.**
A `cd` into a slot worktree followed by `git commit -F scratch/msg.txt` looked
for the message inside the worktree and died. Use `git -C <path>` and absolute
paths for `-F`; the rule below about `cd` is not only about the wrong branch.

**Never background a verify through a shell wrapper.** `nohup python
loop/verify.py ... &` inside a backgrounded compound either never runs or is
orphaned, the wrapper reports exit 0, and the stale `VERDICT.json` from the
*previous* packet is still sitting in the slot ready to be misread as this
one's. Check `VERDICT.json`'s `base` and `commit` fields match the run you
think you are reading.

**A dead worker can leave its shim alive.** Two workers died mid-run with
`cmd.exe` still present under their recorded pid, so a pid check said "running".
What distinguishes them is `events.jsonl` mtime plus whether any `cargo`/`rustc`
process exists at all. `slot_status.py --kill-stalled` is still mis-calibrated
in the other direction (see STATE) — confirm both ways before reaping.

**An interrupted verify leaks its baseline worktree, and that is what actually
fills the disk.** `compute_baseline`'s cleanup does not run when the process is
killed. Two leaked baselines plus one live one took the machine from 9.4 GB to
3.1 GB and forced this session to abandon a run. Before starting a verify,
check `%TEMP%/look-verify-baseline-*` is empty — `verify.py` now warns, but the
deletion is yours.

**`loop/slots/*/target` is only half the disk the loop owns.** Workers create a
*second* `target/` **inside** the worktree (`loop/slots/N/wt/target`) despite
`CARGO_TARGET_DIR` pointing elsewhere, and it is the larger of the two — 1.9 GB
in one slot against 0.9 GB outside. The recovery recipe everywhere in these
docs names only the outer one and reclaims less than half of what is there.

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

## The survey class — delegating judgement without delegating decisions

The orchestrator was the bottleneck long before the workers were. Two jobs were
being done here and only one of them is orchestrator-grade:

- **judgement** — what class is this site, what should the refusal be, is this
  gate wrong;
- **assembly** — running greps to verify anchors, counting functions for a
  budget, boilerplate packet prose, ceiling bookkeeping, merge and ledger
  mechanics.

Assembly is most of the wall-clock and none of it needs a person. `class:
survey` moves the largest single piece of it — reading every call site in a
crate and proposing a classification — onto a worker, and `gen_packet.py`
(anchor/budget pre-flight, refuses the dispatch) and `packet_lint.py`
(crates-vs-write-set, test-path ownership, dependency KIND, forecast numbers,
RESULT placement — the packet-fault classes the anchor check cannot see) take
the rest. Run the lint on every packet before dispatch; both checks are cheap
and each prevented round trip is 15-90 minutes.

**A survey worker gets no write access to `vendor/truck/**` at all.** Its whole
deliverable is `SURVEY.json`: one row per site with file, line, symbol, the
expression, a proposed `model`/`param`/`excluded` classification, a one-sentence
reason, and a confidence. `verify.py` runs V0, V1 and **V10 survey shape**, and
skips every cargo gate, because a survey commits no Rust and running the others
would manufacture a PASS that means nothing — the V7/V8 mistake.

**V10 checks anchors, never judgements.** Every (file, line, expression) must
resolve against the tree; an invented line number fails the packet. That split
is the point: a classification cannot be graded mechanically, but the half that
has actually gone wrong twice — GEOM-SPECIFIEDS shipped three of seven anchor
counts wrong, SHAPEOPS listed a line inside a block comment — is exactly the
half a gate can check. A survey whose sites are all real is cheap to review; one
whose sites are invented is *worse* than no survey, because it reads as
authoritative.

**So the review is not optional.** A survey you skim and paste into a packet has
converted a worker's guess into the orchestrator's decision without anyone
deciding anything. Read every `confidence: low` row, spot-check a sample of the
high-confidence ones against the source yourself, and treat a `SPEC_GAP` from a
survey as the most valuable thing it can return.

This does not contradict "pre-make every judgement so the worker churns rather
than designs". That rule exists because a worker *designing unsupervised inside
a write set* went badly. A worker *proposing under review with no write access*
is a different risk profile, and until now the packet schema could not express
the difference.

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

## Amendment dispatches and the worker inner loop (session 20)

Four speed levers, all measured against the FID-008 chain (three amendment
round trips, each paying a fresh-context re-entry for code the prior worker
had already read, built and instrumented):

- **`run_packet.py --resume` / `--session-id`** continues the prior worker's
  opencode session (`opencode run -s`) for an AMENDMENT dispatch instead of
  starting cold. The session id is recovered from the slot's `events.jsonl`
  (every event line carries `sessionID`). New packets stay fresh-context by
  design; only amendments resume. If the session is gone, dispatch fresh.
- **`gen_context.py`** writes `CONTEXT.md` beside `PACKET.md` at dispatch:
  a deterministic bundle of the allow-listed files' signatures, doc
  first-lines, caller sites and test names, plus an amendment diffstat when
  `--context-diff <range>` is given. It is regenerated from the tree every
  dispatch (cannot go stale), is never committed, and `verify.py` ignores it
  by name like `PACKET.md`. The worker is told to use it to skip the initial
  search but read anything it edits.
- **Worker checks are fast; the verifier is authoritative.** Packets scope
  their done-when to the affected crate (`cargo check -p`, `cargo test -p
  <crate> --lib <module>`) instead of workspace-wide runs; V2/V5/V8/V9
  re-establish build, tests, downstream and geometry authoritatively. A
  worker running `cargo check --workspace --all-targets` on every iteration
  is impersonating the verifier at its own expense. Exception: packets whose
  write set changes a signature other crates consume keep the workspace
  check -- a ripple is cheaper to catch at worker time than a verify round
  trip.
- **`CARGO_INCREMENTAL=1`** for workers (reverses the old `= 0`). A packet
  performs 5-15 edit-rebuild cycles; incremental is the difference between a
  one-second and a minute-scale inner loop. The verifier's builds are its
  own, and slot targets are reclaimed on re-fork.

`selftest_dispatch.py` covers the spawn path including the new flags'
plumbing; run it after touching any of this.
