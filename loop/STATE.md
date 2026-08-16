# Autobuild loop — STATE

Rewritten at the end of every session. The **volatile** part — everything from
"Where we are" through "The parallelism picture" — is capped at ~120 lines and
must be rewritten each time. "Traps" and everything below it is **stable and
accumulates**: entries are added when something costs a session and removed only
when they stop being true, never for length. (The old header claimed the cap
covered everything above "Quick reference"; it never did, and pretending
otherwise would eventually cost a trap.) If you are picking this up cold, read
**this file, then
[`loop/ORCHESTRATOR.md`](ORCHESTRATOR.md) for how to run the loop, then
`python loop/slot_status.py`** — nothing else. Do not read `LEDGER.jsonl` whole.

Updated 2026-08-16, end of session 6. Branch: `integration/kernel-bg`. Nothing
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
- `loop/packets/BG-TOL-001-SHAPEOPS.md` — the newest worked example and the
  template the six remaining shards copy. `BG-S0-003.md` is the older one.

The loop is a **build** loop, not a search loop: acceptance is mechanical and
deterministic, so the verifier does the job an objective function would. Nothing
here is scored, tuned, or sampled.

## Where we are

**Seven contracts discharged**: BG-S0-001, BG-S0-002, BG-S0-003, BG-EVD-r3,
BG-TOL-001-TYPE, BG-TOL-001-TYPE-r2, BG-NUM-001-FILLET. 7 of 58 packets.

**The real state of BG-TOL-001 is lower than the packet count suggests.** The
type exists and the scaffold exists; **0 of 184 call sites are migrated.** The
first shard is running now. Two of the seven "discharged" items are partial
against their contract: BG-NUM-001-FILLET budgeted 1 of the 14 unbounded loops
the spec names, and the two BG-TOL-001 items built machinery and migrated
nothing.

Session 6 landed two contracts, fixed two gates, and **amended the spec twice —
both times because writing a packet exposed something the spec had not decided.**
That is the loop working as designed and it is where its value has actually come
from. The worker model has still never been the bottleneck: BG-TOL-001-TYPE-r2
went DONE → ACCEPTED → merged in one attempt, one verify run, no amendment.

## Pick up here

1. `python loop/slot_status.py`, then the last 3 rows of `LEDGER.jsonl`.
2. **Slot 3 holds BG-TOL-001-SHAPEOPS, dispatched and unverified.** Verify with
   `--base 37a0503` or whatever `git -C loop/slots/3/wt merge-base HEAD
   integration/kernel-bg` reports. It is the **first Stage-A migration shard and
   the template for six more**, so read its diff yourself even if it is
   ACCEPTED — V7 and V8 are still stubs and this is the packet whose conventions
   the other six inherit.
3. **When it lands, lower the ceiling.** `scripts/unscaled_legacy_ceiling.txt`
   is at **20** as a budget for that shard. Set it to the actual count from the
   merged tree (`git grep -oh 'unscaled_legacy(' HEAD -- 'vendor/truck/*/src/*'
   ':(exclude)vendor/truck/truck-base/src/tolerance.rs' | wc -l`) in its own
   commit. A ceiling left at a budget is not a ratchet.
4. **Then write the other six shards**, copying BG-TOL-001-SHAPEOPS. Size them
   with `python loop/census_tol_sites.py`, **not** with a raw grep — see the
   census note below. **Pre-make every model/param judgement yourself**; that is
   the whole value of a shard and it needs someone who reads the surrounding
   code. MODELING next: it is 11 sites, not the 34 a grep reports.
5. **`BG-TOL-001-TOPOLOGY` blocks all eight BG-INV checkers** and is 4 sites.
   Earlier sessions planned to split it by module. Do not — look first.
6. **Three crates with production sites have no shard at all**: truck-stepio
   (19), truck-polymesh (7), truck-geotrait (4). stepio is the STEP import and
   export path — the most directly reachable-from-untrusted-geometry surface in
   the tree, and exactly what BG-TOL-001's contract is about. Add the rows.

Highest-value harness work left: **V7 and V8 are always-pass stubs** — the two
remaining gates where PASS means nothing. V8 is where "this packet broke a
pre-existing test" belongs; V5 only compares against its cached baseline.

## Landed

| commit | what |
|---|---|
| `da72cd5` | vendored truck at `vendor/truck/` (12 crates) + kernel gates + evidence module |
| `fddc62a` | vendored crates are workspace members — without this `cargo test -p <crate>` (V5) cannot run |
| `65450b3` `ca22bc4` `a5660c3` | loop scaffolding, first packets, the 56-packet DAG |
| `b06a535` | three baseline clippy defects fixed (the slot-0 fork point) |
| `ed35879` … `8dca941` | the verifier and dispatcher, made to actually work |
| `c8acab6` | **BG-S0-003** — the first packet through the whole loop |
| `4cc5aca` | harness ported to stdlib-only Python; the four `.ps1` scripts are gone |
| `baa2dfa` `88b93ee` `aa6e31a` `fb697ea` | V5 `--no-fail-fast` + cached baseline, V0 allow-lists `*.obj`, dispatch survives its parent, ORCHESTRATOR counterweights |
| `5b68c78` `27ce4d7` | **BG-S0-002**, **BG-EVD-r3** |
| `ec34aa0` | **session 6:** V3's fmt half was whole-crate — the same defect its clippy half already fixed |
| `ce524fa` | **BG-NUM-001-FILLET**, merged unmodified after six verify runs |
| `331633a` | spec: BG-TOL-001 never said where a call site gets its context |
| `c53e3e6` | **GATE-4**, the `unscaled_legacy` ratchet |
| `871e79f` | **BG-TOL-001-TYPE-r2** — the Stage-A scaffold |
| `11aa0b9` | spec: squared-order sites deferred to BG-TOL-004; the SHAPEOPS packet |

## The two spec amendments session 6 paid for

**BG-TOL-001 never said where a call site gets its `ToleranceCtx`.** It says
migrate 184 sites and §9 says "every signature below takes ctx"; neither says
how a site *obtains* one, and none of the 184 sits in a function that has one.
Threading from the entry points inward changes public signatures in every crate
at once, so it cannot be sharded per crate — which is exactly what the eight
`BG-TOL-001-*` rows assume and what makes their write sets disjoint. The
migration is now **two stages**: Stage A (the shards) classifies every site
`model` or `param` through `ToleranceCtx::unscaled_legacy()`, moving no
threshold and changing no signature; Stage B threads a real `model_scale` from
each entry point and is what actually discharges the contract. **Stage A alone
fixes nothing** — it buys the judgement, which is the half that cannot be done
mechanically later. GATE-4 ratchets the scaffold so Stage A cannot quietly
become the answer.

**`near2`/`so_small2` cannot be migrated at all.** They compare against
`TOLERANCE2` = 1e-12 and `ToleranceCtx` has no squared-order predicate; mapping
them onto `tau_rep` loosens them by six orders of magnitude *while looking like
a migration*. All 23 sites tree-wide are excluded from Stage A and deferred to
**BG-TOL-004** (design, blocks nothing).

## The parallelism picture

58 packets: 37 mechanical, 12 design, 8 wide-mechanical + the two r2 items.
Scheduling is on **write-set disjointness**, not waves — two packets can be
logically independent and still collide on a file, and that collision surfaces
at merge, after both workers have been paid for.

The frontier is 10 eligible / 7 write-disjoint and stays there until the shards
land; it opens to 22 at W4. From here **slots, not dependencies, are the binding
constraint**: a warm slot costs 0.9–4.4 GB and 0.8–1.5 min, and free disk is
~17 GB. **Six of the seven eligible shards have no packet written yet** — the
binding constraint in practice is orchestrator packet-writing, not slots.

## Traps, each one paid for

- **A gate that fails on the untouched baseline is not a gate.** The vendored
  tree is nowhere near clippy-clean (truck-meshalgo ~93 lints,
  `revolved_curve.rs:694` "items after a test module") and **not rustfmt-clean
  either** (`revolved_curve.rs:690`, a stray blank line, present at base). Its
  test suite is not clean either (`healing::tests::step_import` needs an absent
  STEP file; `tests/fillet.rs::complex_surface` triangulates to `Irregular`).
  V3 is scoped to the **lines the diff added** (clippy) and the **files the diff
  changed** (fmt); V5 to the **test fns the diff added**.
- **The fmt half of V3 was whole-crate for five sessions** and cost
  BG-NUM-001-FILLET its sixth rejection — on a file the packet never opened and
  *could not have fixed*, because it was outside its own `write_allow`. That
  combination is the signature of a gate defect: when the only way to get green
  is to violate another gate, it is not the worker who is wrong. Fixed in
  `ec34aa0`. **fmt is scoped by file, not by line** — rustfmt reports where its
  diff *context* starts, several lines above the text it wants to change, so
  intersecting those numbers with added lines both misses real findings and
  invents absent ones.
- **`git grep` exits 1 when it matches nothing, and under `set -o pipefail`
  that kills the whole script.** GATE-4's first run on a clean tree exited 1
  with **zero output** — every earlier gate unreported, indistinguishable from
  a crash. The `|| true` inside the command substitution is load-bearing.
- **`kernel-gates.sh` reads `HEAD`, not the worktree.** Staging a probe is not
  enough to trip GATE-4; the negative test has to commit. This matches V4's
  timing, which runs after the worker commits.
- **Watch a gate fail before trusting it, and commit it first.** The negative
  test for GATE-4 ended in `git reset --hard`, which reverted the *uncommitted
  gate itself*. Commit the gate, then probe, then reset.
- **rustfmt moves a trailing `// H-3` comment off a brace-opener line.** The
  opt-out only works on the same line as the literal, so rustfmt silently
  defeats it. Extract the literal onto its own statement line. (Worker-reported,
  BG-TOL-001-TYPE-r2.)
- **`rg` is not installed on this host.** H-8 anchors are still `rg` patterns
  and workers check them with whatever grep they have. Verify anchors yourself
  before dispatch; do not write a packet that shells out to `rg`.
- **Dead code looks exactly like live code.** `truck-shapeops/src/fillet/
  experiment.rs` holds 5 tolerance sites and is not compiled — `fillet/mod.rs`
  carries `//mod experiment;`. A fifth of the SHAPEOPS shard would have been
  unverifiable edits to a file nothing builds. Check that a module is actually
  declared before putting it in a write set.
- **Not every use of `TOLERANCE` is a predicate.**
  `polyline_construction/mod.rs:32` uses it as a spatial-hash bucket pitch. It
  compares nothing, so it has no `model`/`param` classification, and a
  mechanical migration would have produced nonsense that still compiled.
- **Legacy `.near()` is componentwise; `ToleranceCtx::near_pt` is Euclidean.**
  Not the same predicate — Euclidean is stricter by up to `sqrt(3)`. Every Stage-A
  shard is therefore a small deliberate tightening. A test that moves because of
  it is a finding to report, never a tolerance to widen.
- **A write set has to cover the ripple, not just the edit.** BG-NUM-001-FILLET
  changed one function's signature and was rejected for touching its only
  caller, in another crate. Grep for callers of anything whose signature a
  packet changes before writing `write_allow`.
- **A ceiling a packet can raise is not a ratchet.**
  `scripts/unscaled_legacy_ceiling.txt` is deliberately outside every shard's
  `write_allow`; the orchestrator raises it before dispatch and lowers it after.
**Already fixed in the harness — do not undo these.** Each cost a session; the
reason is in the commit that made the change, and the code will not tell you.

- **V5 uses `--no-fail-fast`**: without it cargo stops at the first failing test
  binary and never reaches the packet's own `tests/*.rs`, so it reported PASS on
  something it never ran.
- **V5 diffs a cached baseline** rather than scoping to added lines the way V3
  does: a test failure is not located in the diff, and scoping it there
  discards regression detection instead of noise.
- **V0 allow-lists `*.obj` specifically** rather than ignoring untracked files —
  `cargo test` drops mesh dumps into the worktree the verifier is judging, but
  an uncommitted new `.rs` is exactly what V0 exists to catch.
- **V0 exists because an interrupted run reads as a perfect one**: a worker that
  dies mid-packet leaves its edits uncommitted, an empty diff that passes V1–V6.
- **The dispatch uses `CREATE_NO_WINDOW | CREATE_NEW_PROCESS_GROUP |
  CREATE_BREAKAWAY_FROM_JOB`, never `DETACHED_PROCESS`**, which silences the
  worker it is meant to free, and writes its redirect into a command file
  because `opencode` is a `.cmd` shim with an 8191-char command line. **Run
  `python loop/selftest_dispatch.py` after touching any of this.**
- **V4 hardcodes Git Bash** — a bare `bash` is the WSL stub.
- **`slot_status.py --kill-stalled`** reaps anything silent for 12 min; only
  `events.jsonl` growth distinguishes a hang from a worker thinking.
- **`autotests = false` in truck-polymesh**: a new test file there needs an
  explicit `[[test]]` entry or it silently never runs. V6 flags this.
- **`truck_base::evidence`, not `truck_evidence`** — the module lives in
  truck-base to avoid a geotrait→evidence cycle.

- **A raw grep for tolerance sites is off by 3x, in both directions.**
  `python loop/census_tol_sites.py` splits them: **238 production predicates**,
  plus 66 doc-comment examples, 4 `#[strategy = TOLERANCE..]` test-input bounds,
  2 in-src test assertions and 22 squared-order sites — none of which are
  migration work. truck-modeling reads as 34 sites and has **11**; truck-topology
  reads as 14 and has **4**; truck-meshalgo was recorded here as 375 and has
  **45**, so it never needed the split earlier sessions planned. Size a shard
  with the census, and note the production total is *higher* than the spec's
  stated 184, not lower.
- **The spec goes stale invisibly.** Re-run every anchor when you touch a packet.
- **A spec gap is the loop's most valuable output, not a failure.** Session 6's
  two amendments both came from writing a packet and finding the spec had not
  decided something. Fix the spec *and* the packet; never only the packet.
- **CI gates are still vacuous.** `kernel-gates.sh` is diff-scoped and
  `origin/main` has no `vendor/truck/`, so CI passes on nothing. Packet
  verification is unaffected — its baseline is the branch tip.

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
  the right home for "a packet broke a pre-existing test" -- V5 deliberately
  does not catch that, only added-test failures, so it never false-rejects on
  the baseline.
- V6 matches test names by keyword overlap, not exactly. Tighten when
  `gen_packet.py` fixes a naming convention.
- `gen_packet.py` is still unwritten. It must re-run every anchor's pattern at
  generation time and refuse to emit on a count mismatch. Six shard packets are
  about to be written by hand; this is the session to build it in.
- GATE-4's ceiling is checked, but nothing checks that a Stage-A shard's
  `// BG-TOL-001:` markers match its actual rewrites. The SHAPEOPS packet asks
  the worker to write that test itself (`every_migrated_shapeops_site_is_marked`).
  If that pattern holds up, hoist it into a gate rather than repeating it in
  six packets.
- BG-TOL-004 (what a squared-order tolerance means in a scale-relative system)
  is named in the spec but unwritten. 23 sites wait on it. It blocks nothing.
- BG-S0-002-r2 needs a constructed degenerate surface (a parametric pole where
  `uder` is parallel to `vder`) to exercise `create_pcurve_edge` directly. The
  fixture is design work; the orchestrator writes that packet.
- `opencode/deepseek-v4-flash-free` would run W4's 23 packets at no API cost.
  Untested against concurrent workers.
