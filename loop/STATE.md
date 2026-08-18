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

Updated 2026-08-18, end of session 7. Branch: `integration/kernel-bg`. Nothing
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

**Thirteen packets DONE of 62**: BG-S0-001, -002, -003, BG-EVD-r3,
BG-TOL-001-TYPE, -TYPE-r2, -TYPE-r3, BG-NUM-001-FILLET, -SHAPEOPS,
-TOPOLOGY + -MODELING (both closed by `BG-TOL-001-TOPO-MOD`), and session 7's
**-GEOM-SPECIFIEDS** and **BG-CE-006-CYLINDER + -CONE** (closed by the combined
`BG-CE-006-CYL-CONE`).

**BG-TOL-001 burndown: 44 sites migrated, 175 to go** — and note the direction:
it went *up* by 4 when `BG-CE-006-CYL-CONE` landed, because new carriers bring
their own predicates. Stage A is a moving target while the kernel is still
growing surfaces. — `python
loop/census_tol_sites.py` is the authority and it now also prints, per crate and
per path fragment, the number of **functions** containing a site. That second
number is what a shard declares as `unscaled_legacy_budget`, and getting it
wrong is what cost session 7 a round trip. GATE-4 sits at **40/40**: ceiling
equals true count, so the ratchet is tight and the next shard must raise it by
its own measured budget before dispatch.

**Session 7 also changed the delegation architecture.** `class: survey` lets a
worker read a crate and *propose* a `model`/`param` classification for every
site with no write access to `vendor/truck/**`; `gen_packet.py --check` makes a
packet's anchors and budget executable and `run_packet.py` refuses to dispatch
on a mismatch; `land_packet.py` does the whole merge-file-ledger-ratchet
sequence. The first survey ran and passed — reviewing it is item 1.

Session 7's theme: **every defect found was in a packet or a gate, never in the
worker's code.** The worker migrated 22 sites correctly and returned SPEC_GAP
because the packet's budget could not hold its own recipe; it was right. The one
V5 rejection was a flaky proptest. Two gates were fixed and one gate (V9) was
finally proven.

## Pick up here

1. **Review `loop/surveys/BG-TOL-001-MESHALGO.json`, then write
   `BG-TOL-001-MESHALGO` from it.** The first survey ran and was ACCEPTED
   (V0/V1/V10). 104 rows accounting for every grep hit in 8 files: **15 model,
   11 param, 78 excluded**. V10 proved every (file, line, expression) resolves
   against the tree; it says **nothing** about whether the classifications are
   right, and that is your job before any of it reaches a packet.
   - **Read the 4 `confidence: low` rows first** — `triangulation.rs:4517,
     4523, 4529, 4535` in `reconcile_singular_transition`. The worker reports
     each of those source lines carries **two** predicates: a dimensionless
     `!near` guard on uv parameters and a model-space `so_small()` on a surface
     derivative magnitude. It classified the row `model` for the deciding test.
     That is a real judgement call and it is the one to check yourself.
   - **Reconcile the count**: the survey's 26 live sites against the census's
     **30** production predicates for the crate. The worker attributes the gap
     to 3 squared-order and 3 "production non-predicate" uses; confirm that,
     because one of them is likely the spatial-hash bucket pitch and the rest
     need a reason.
   - Then `python loop/gen_packet.py --skeleton loop/surveys/BG-TOL-001-MESHALGO.json
     --id BG-TOL-001-MESHALGO --crate truck-meshalgo` emits the front block,
     write set, measured anchors and site table. **Write the prose yourself** —
     Problem, Decisions-already-made, Stop conditions. The skeleton deliberately
     omits them; they are what makes the worker churn instead of design.
   - If the review holds up, the four other unwritten shards go the same way
     and the orchestrator stops hand-reading call sites. **If it does not, say
     so in STATE** — one survey is not yet evidence.
2. **`BG-CE-006-CYL-CONE` LANDED** (`e9b4be4`, ACCEPTED on all ten gates,
   landed by `land_packet.py`). Cylinder and Cone are first-class carriers now,
   closing both `BG-CE-006-CYLINDER` and `-CONE`. **One thing to follow up:**
   its worker disagreed with the packet and says `Plane`'s `BoundedSurface`
   impl is *not* the panic-installing defect the packet called it, because this
   tree's `Plane::parameter_range` is a bounded `[0,1]^2` so
   `range_tuple().expect` cannot fire. If the worker is right — check it — the
   claim needs correcting in STATE's traps and in any packet that repeats it,
   and the same reasoning should be re-applied to whether Cylinder and Cone
   ought to implement `BoundedSurface` after all.
3. **Then the CE chain, which is the actual critical path to generation.**
   Seven of the nine BG-INV invariant checkers — the things that let the kernel
   say whether its own output is a valid solid — are gated on **one** packet,
   `BG-CE-003`, through `BG-CE-006-CYL-CONE -> BG-CE-006-ENUM -> BG-CE-001 ->
   BG-CE-003`. Three of those four are **design** class, so the orchestrator
   writes them; that is the bottleneck, not worker throughput. Note
   `schedule.py` reads `needs`, and the rows also carry a stale, different
   `depends_on` — `needs` is the real graph.
4. **`BG-TOL-001-STEPIO` is written, anchors verified, budget measured (15),
   not dispatched.** All 19 predicates classified with the judgement pre-made,
   including the one pair worth arguing about (`geom_impls.rs` `include` is
   `param`, `to_same_geometry` is `model`, and they look identical). Raise the
   ceiling by 15 before dispatching.
5. **The other four TOL shards are unwritten**: GEOM-NURBS, GEOM-DECORATORS,
   MESHALGO, POLYMESH, GEOTRAIT. Size each with
   `python loop/census_tol_sites.py <path-fragment>` — it prints both the site
   count and the function count. **This is breadth, not depth**: Stage A moves
   no threshold, and Stage B (threading a real `model_scale` from the entry
   points) is still not in the graph at all. Do not grind all of these before
   item 3.

Highest-value harness work left: **V7 and V8 are always-pass stubs** — the two
remaining gates where PASS means nothing. V8 is where "this packet broke a
pre-existing test" belongs. V5 now has a demonstrated blind spot of its own: it
compares one run of a randomized proptest suite against another and reads a
flake as a regression.

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
| `d26cefb` | **BG-TOL-001-SHAPEOPS** — 16 sites classified, the shard template |
| `6e07e49` | **BG-TOL-001-TYPE-r3** — one-sided margins + generic point predicate |
| `58de977` | **BG-TOL-001-TOPO-MOD** — closes both TOPOLOGY and MODELING |
| `ba2b7be` | **V9** + `tests/geometry_fingerprint.rs` — see the V9 warning above |
| `09fb2bf` | census fix: every `#[cfg(test)]` module was counted as production |
| `bcc9139` | **session 7:** V9 watched failing, twice, on two failure modes — the gate is proven |
| `52d4552` | dispatch preflight: a packet's GATE-4 claim is checked before a worker is paid |
| `f9fa761` | GEOM-SPECIFIEDS' anchor counts were wrong when written; 3 of 7 |
| `e90e9dc` | a shard's `unscaled_legacy` budget is a measurement now, not an estimate; ceiling 29 → 36 |
| `aa2dadd` | V0 missed the proptest seed file under its fallback name |
| `0a7c6fe` | **BG-TOL-001-GEOM-SPECIFIEDS** — 22 sites, 19 contexts, ACCEPTED on all ten gates |
| `808d472` | the `BG-CE-006-CYL-CONE` packet |

## The spec and packet amendments the loop has paid for

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

62 packets: 38 mechanical, 13 design, 11 wide-mechanical. 51 remain. The
frontier reads **10 eligible, 10 write-disjoint** with nothing running, and
session 7 demonstrated that two workers on write-disjoint packets run
concurrently without interfering — slots 0 and 1 both edited `truck-geometry`,
on disjoint files, at the same time.

Scheduling is on **write-set disjointness**, not waves. Two of the declared
write sets were wrong in the same way and it is worth checking for more:
`BG-CE-006-CYLINDER` and `-CONE` each named only their own new file, when both
must also declare their struct in `specifieds/mod.rs`, where every specified
struct lives. As two packets they collide there; they were merged into one,
`BG-CE-006-CYL-CONE`, the way TOPOLOGY and MODELING were.

**The binding constraint is orchestrator packet-writing, not slots or
dependencies** — 12 of 62 packets have a file. Design-class packets are the
sharpest form of it: three of the four on the critical path to the invariant
checkers are design, and the orchestrator writes those.

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

- **This loop can eat 40 GB of disk in one session, and did.** Free space went
  40 GB → **0.1 GB**. Two causes, both in the verifier. Every V9/V5 baseline
  builds a *whole extra workspace* in a throwaway worktree under the system
  temp dir, and `compute_baseline`'s cleanup is best-effort with a comment
  calling a leftover "harmless" — it is not, each one is ~1.3 GB and they
  accumulate per distinct (base, test-set) key. And a probe that edits
  `truck-base` invalidates every downstream crate, so a slot's `target/` grew
  4.4 → 12.9 GB across three negative-test runs. Recovery is easy and total:
  delete `loop/slots/*/target` (a slot re-warms in 1-3 min) and any
  `%TEMP%/look-verify-baseline-*`, then `git worktree prune`. **Check
  `Get-PSDrive C` before a run of repeated verifies, not after.**
- **A negative test leaves the repo on the broken commit.** Probing V9 meant
  committing `TOLERANCE = 1.0e-1` on `integration/kernel-bg` in the *main*
  worktree and pointing a slot at it. When the run was interrupted, the repo
  root sat on that commit with the kernel's tolerance five orders of magnitude
  wrong. Nothing downstream noticed, because nothing was watching. Reset the
  main worktree and delete the probe branch as part of the probe, not after it.
- **Dead text looks exactly like live code, and it reached a packet.** The
  SHAPEOPS site table listed `fillet/mod.rs:615`, which sits inside a `/* */`
  block spanning lines 500–662. The worker migrated a comment, as instructed,
  and said so plainly — it was right and the packet was wrong. `git grep` and
  the census both counted it, and GATE-4 counted the resulting phantom scaffold
  call. The census now skips `/* */`; **a module declared out with a commented
  `mod` statement still slips through** (`experiment.rs`, 5 sites), because
  detecting that needs the declaration, not the file. Check the `mod` statement
  before putting a file in a write set.
- **A ceiling left at its dispatch budget is a licence, not a ratchet.**
  GATE-4's ceiling was raised to 20 to dispatch SHAPEOPS and lowered to the
  true 11 in the same session. Lower it in the commit that closes the packet.
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

- **A packet's budget is a claim about the repo, and claims rot.** Session 7
  found two in one packet: the anchor table was **wrong when written** (3 of 7
  counts, on files unchanged since before the packet existed — so not drift),
  and `unscaled_legacy_budget` was an estimate ("about 12 here") against a true
  19. Both would have been caught by running a command instead of reading a
  file. `run_packet.py` now refuses to dispatch when GATE-4's count plus the
  declared budget exceeds the ceiling **committed on the slot's own branch** —
  raising it on `integration/kernel-bg` after the slot forked does nothing.
- **The budget is one context per FUNCTION with a site, not per site or per
  file.** `census_tol_sites.py <path-fragment>` prints it. Keying those
  functions by (file, name) undercounts: `truck-stepio/src/out/geometry.rs` has
  five distinct `fmt` impls each holding a site, which collapsed to one and made
  the crate read 11 instead of 15.
- **V5 reads a flaky proptest as a regression.** It compares one run of a
  randomized suite at base against one run at HEAD.
  `truck-geometry/tests/bspcurve.rs::parameter_random_tests` fails
  occasionally by ~3e-6 against a 1e-6 tolerance; it did so once during
  GEOM-SPECIFIEDS' verify, in a file the packet never opened. 12 subsequent
  runs — 6 at base, 6 on the branch, with the failing seed present — all
  passed. **Before believing a V5 failure in a file the packet did not touch,
  re-run it at both commits.**
- **A gate that blocks its own retry turns a flake into a permanent
  rejection.** That same proptest failure wrote
  `tests/bspcurve.proptest-regressions`, and V0's ignore rule only matched a
  *directory* of that name, so every later verify on the slot BLOCKED on the
  artifact left by the run being re-measured. Fixed in `aa2dadd`.
- **Backticks in a `git commit -m` message are command substitution.** A commit
  message quoting a command name in backticks hung the shell for two minutes and
  committed nothing. Write the message to a file and use `-F`.
- **Two workers on write-disjoint packets run concurrently without trouble** —
  first demonstrated in session 7 (slots 0 and 1, both editing `truck-geometry`
  but disjoint files). The open question about concurrency was about the *free*
  model tier and is still open.

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

- **V9 was proven in session 7 — see "Pick up here" item 2** for both
  negative tests and what each one does and does not establish. It was added
  because nothing in this loop had ever been measured against real geometry.
  Its first version ran `tests/step.rs`, `torus_deck.rs` and
  `spline_carrier.rs`, **passed with `TOLERANCE` loosened 1e-6 → 1e-1**, and
  the reason is worth keeping: those tests assert *structure*, not geometry —
  one geometry, one instance, indices a multiple of 3, a colour present — and
  `torus_deck` asserts the torus's *declared* parameters read back from the
  source rather than anything tessellated. All of it holds for an arbitrarily
  wrong mesh. `tests/geometry_fingerprint.rs` is the fix (triangle count,
  vertex count, bounds) and passes clean, but has not been seen failing.
- **The corpus tooling is still unused.** `benchmarks/` and the face-census
  scripts have existed the whole time and the loop has never invoked them. V9
  covers two fixtures; that is "these two parts still tessellate the same
  way", not "the kernel is correct."
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
