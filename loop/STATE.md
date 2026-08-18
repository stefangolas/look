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

Updated 2026-08-18, end of session 8. Branch: `integration/kernel-bg`. Nothing
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
- `loop/packets/BG-TOL-001-MESHALGO.md` — the newest worked example and the
  template the remaining shards copy; it is the first packet written from a
  reviewed survey rather than by hand. `BG-TOL-001-SHAPEOPS.md` is the older
  hand-written one and `BG-S0-003.md` older still.
- `loop/packets/BG-TOL-001-MESHALGO-SURVEY.md` — the `class: survey` template,
  sharpened after its first run. Copy it to survey a new crate.

The loop is a **build** loop, not a search loop: acceptance is mechanical and
deterministic, so the verifier does the job an objective function would. Nothing
here is scored, tuned, or sampled.

## Where we are

**Thirteen packets DONE of 62**, unchanged from session 7: BG-S0-001, -002,
-003, BG-EVD-r3, BG-TOL-001-TYPE, -TYPE-r2, -TYPE-r3, BG-NUM-001-FILLET,
-SHAPEOPS, -TOPOLOGY + -MODELING (both closed by `BG-TOL-001-TOPO-MOD`),
-GEOM-SPECIFIEDS, and BG-CE-006-CYLINDER + -CONE (closed by
`BG-CE-006-CYL-CONE`). **`BG-TOL-001-MESHALGO` is RUNNING in slot 0** as of the
end of session 8, dispatched at `6ca37b2`; it has not been verified.

**The survey experiment came back positive, and the review was not a
formality.** The first `class: survey` output was ACCEPTED on V0/V1/V10 and V10
did its job perfectly — all 26 live expressions matched the tree byte for byte,
so the half that had gone wrong twice before (invented anchors) did not go wrong
here. The classifications needed **four corrections**, which moved the shard from
26 sites / 16 contexts to **20 sites / 11 contexts**. Both halves of that
sentence matter: a survey that needed no correction would mean the review could
be skipped, and a survey whose sites were invented would be worse than none. This
one was neither. **Keep the survey path; keep reading every row.**

**The survey also found two census defects and got them right**, which is the
clearest evidence that delegating the reading is worth it: the census regex
`\bTOLERANCE\b` cannot match `SOURCE_INCIDENCE_TOLERANCE` because `_` is a word
character (3 hidden hits, one a live predicate), and its SQUARED regex misses a
written-out `TOLERANCE * TOLERANCE`. Neither the census nor the survey covers the
`RELATIVE_TOLERANCE = 1e-9` family in `tessellation/formal/*.rs` — **8 production
predicates in no inventory at all.** The census's 175 is therefore a floor, not a
count.

**BG-TOL-001 burndown: 44 sites migrated, 175 to go** by the census, which is now
known to be wrong in both directions (see above). `python
loop/census_tol_sites.py` is still the sizing tool; it is no longer the authority
on what exists.

**GATE-4 sits at 40/51.** The count is the one `scripts/kernel-gates.sh` takes
(a `git grep -oh 'unscaled_legacy('` over `vendor/truck/*/src/*`, excluding
`truck-base/src/tolerance.rs` where the constructor is defined — a plain grep
reads 41 and is wrong by that one line). The ceiling was raised by MESHALGO's 11 measured
contexts and **must be lowered to the true count in the commit that closes the
packet** — a ceiling left at its dispatch budget is a licence, not a ratchet.

## Pick up here

0. **`BG-TOL-001-MESHALGO` is committed at `8cf0e92` and was verified REJECTED
   on V3 ALONE — and the rejection is probably the gate's, not the worker's.**
   V3's detail is *"clippy could not build truck-meshalgo, so findings in it were
   never produced"* — that is a **build failure inside clippy, not a lint
   finding**, and **V2 (`cargo check --locked`) PASSED on the same tree**. The
   difference is `--all-targets`. **First thing to do: run
   `cargo clippy -p truck-meshalgo --all-targets --no-deps` at base `6ca37b2`.**
   If it fails there too, a gate that fails on the untouched baseline is not a
   gate and the packet must not be redispatched for it. Everything else about
   this packet checks out (see below), so do not discard the branch.
   - **Evidence already gathered, do not re-measure:** the two failing
     `cone_topology_tests` (`duplicate_edge_creates_no_second_cdt_edge`,
     `test_parity_intersecting_constraints_rejected`) fail **identically at base**
     — same assertions, `left: 5 / right: 3`, line numbers shifted by exactly 31.
     Measured independently in slot 1 detached at `6ca37b2`, and the worker
     reached the same answer by stashing its diff. **They have been broken since
     `da72cd5` and no gate ever noticed**, because no packet before this one
     listed truck-meshalgo in `crates:` and V8 is a stub.
   - **The packet's budget of 11 contexts is wrong; it should be 10.**
     `end_pts` is a two-line nested `fn` that closes at 8276; the sites at 8278
     and 8283 sit *after* it, back in `new_with_join`'s `2 =>` arm.
     **`census_tol_sites.py` and `gen_packet.py`'s `packet_contexts` share the
     bug** — both scan upward for `fn <name>` without tracking braces, so both
     attribute a site to a nested helper that has already closed. The worker
     honoured the wrong 11 by introducing a context that **shadows
     `new_with_join`'s** inside the match arm, and said so plainly. That shadow
     is an orchestrator amendment to remove, not a worker defect.
   - Worker cost for the whole packet: **$0.057**. All 49 remaining packets
     extrapolate to **~$2.78**. Worker spend is not a constraint; orchestrator
     time is. 75% of the worker's wall-clock was model latency, not cargo.

1. ~~Verify `BG-TOL-001-MESHALGO`~~ — done, see item 0. **Original guidance:** `python
   loop/verify.py --slot 0 --packet loop/packets/BG-TOL-001-MESHALGO.md --base
   6ca37b2`. Then merge `--no-ff`, file `RESULT.json`, ledger row, `status: DONE`
   in `PACKETS.jsonl`, and **lower the ceiling to the true count**.
   - The packet asks for something no earlier shard did: **six `FIXME` markers
     and no rewrite** in four files, with an explicit test
     (`deferred_area_sites_carry_a_fixme`) asserting those files contain **no**
     `ToleranceCtx` at all. If the worker migrated one of the six area sites
     anyway, that is a real rejection, not a gate defect.
   - Watch for the one thing the packet leaves genuinely open: the four
     `reconcile_singular_transition` lines each need **two** predicates migrated
     and one context. If the worker introduces more than 11 `unscaled_legacy()`
     calls it built one per site.

2. **The other four TOL shards go the survey route: GEOM-NURBS,
   GEOM-DECORATORS, POLYMESH, GEOTRAIT.** The survey packet
   (`loop/packets/BG-TOL-001-MESHALGO-SURVEY.md`) has been **sharpened with the
   four defects this review found** and is now the template — copy it, change
   the crate and the site inventory. The additions are: the degree-2 exclusion,
   the "a value is not a comparison" exclusion, `predicates_on_line` plus
   `mixed_classification` so a two-predicate line can express itself, and the
   instruction to grep for `_TOLERANCE\b` constants the inventory cannot see.
   Size each with `python loop/census_tol_sites.py <path-fragment>` and remember
   it is a floor.
   - **`BG-TOL-001-STEPIO` is still written, checked and undispatched** (budget
     15, now verified by `gen_packet --check` at 15 == 15). Raise the ceiling by
     15 before dispatching it. It does not need a survey; it already has the
     judgement.

3. ~~`BG-CE-006-CYL-CONE`'s worker disagreed with its packet~~ — **checked and
   closed in session 8; the worker was right.** `Plane::parameter_range` returns
   `(Bound::Included(0.0), Bound::Included(1.0))` on both axes, so
   `try_range_tuple` yields two `Some`s and `range_tuple`'s
   `.expect(UNBOUNDED_ERROR)` cannot fire — `impl BoundedSurface for Plane {}` is
   sound and the packet's "pre-existing defect" sentence was false. The packet is
   annotated with the correction rather than silently edited. **The rest of that
   decision got stronger, not weaker:** `Cylinder` and `Cone` both return
   `(Bound::Unbounded, Bound::Unbounded)` for `v`, so `range_tuple` genuinely
   would panic on them, and the answer to "should they implement `BoundedSurface`
   after all" is a firmer **no**. Nothing to do here.

4. **Then the CE chain, which is the actual critical path to generation.** Seven
   of the nine BG-INV invariant checkers are gated on **one** packet,
   `BG-CE-003`, through `BG-CE-006-CYL-CONE -> BG-CE-006-ENUM -> BG-CE-001 ->
   BG-CE-003`. Three of those four are **design** class, so the orchestrator
   writes them; that is the bottleneck, not worker throughput. `schedule.py`
   reads `needs`; the rows also carry a stale, different `depends_on`.

5. **Highest-value harness work left: V7 and V8 are still always-pass stubs** —
   the two remaining gates where PASS means nothing. V8 is where "this packet
   broke a pre-existing test" belongs. Note `gen_packet --check-all` now reports
   two landed packets (`BG-CE-006-CYL-CONE`, `BG-TOL-001-GEOM-SPECIFIEDS`) as
   having unverifiable budgets, because their site tables are not in a parseable
   form. That is true and blocks nothing — `run_packet` checks only the packet
   being dispatched — but it means neither of those budgets was ever checked
   against anything.

6. **Disk: 13 GB free** at the end of session 8, above the loop's 8 GB floor and
   falling while slot 0 builds.
   Slot 0 is warm; slots 1-3 had their `target/` deleted at the end of session 7
   and re-warm cold in ~5.6 min.

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
| `6ca37b2` | **session 8:** the first survey reviewed and corrected; `gen_packet` made usable end to end; the degree-2 exclusion written into the spec; the `BG-TOL-001-MESHALGO` packet |

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

### Queued and half-built (session 8 ran out of budget here)

- **Three survey packets are generated but NOT dispatched**:
  `BG-TOL-001-GEOM-NURBS-SURVEY` (35 fns), `-GEOM-DECORATORS-SURVEY` (16),
  `-SMALL-SURVEY` (polymesh+geotrait combined, 11). They came from the new
  `loop/gen_survey.py`. **Fix its site-count bug before dispatching**: it reports
  `sum(len(functions per file))`, i.e. the *function* count, and labels it
  "at least N production predicates". The phrasing stays true but the number is
  far too low, and a worker told "at least 35" when the real count is higher may
  stop early. Surveys are read-only and cannot collide, so all three can run
  concurrently on slots 1-3.
- **Not yet built, ranked by value:** (a) **V8** — the base-vs-HEAD comparison
  was done by hand twice this session and is the gate that just waved through
  two broken invariant tests; (b) `gen_packet --skeleton` should emit the
  boilerplate prose — **43% of a packet is templatable** and only ~29% is real
  judgement; (c) the brace-tracking fix for both context counters.

## The parallelism picture

62 packets: 38 mechanical, 13 design, 11 wide-mechanical. 49 remain. With
MESHALGO running the frontier reads **9 eligible, 8 dispatchable in parallel**
(`schedule.py --running BG-TOL-001-MESHALGO`). Session 7
demonstrated two workers on write-disjoint packets running concurrently without
interfering.

Scheduling is on **write-set disjointness**, not waves. Two of the declared write
sets were wrong in the same way and it is worth checking for more:
`BG-CE-006-CYLINDER` and `-CONE` each named only their own new file when both
must also declare their struct in `specifieds/mod.rs`.

**The binding constraint is still orchestrator packet-writing** — 12 of 62
packets have a file (12 files close 14 rows; CYL-CONE and TOPO-MOD each closed
two). But the survey class is measurably eating into it: writing
`BG-TOL-001-MESHALGO` from a reviewed survey took a fraction of what
hand-reading `BG-TOL-001-STEPIO`'s 19 call sites cost, and the expensive part
that remains is judgement, which is the part that should stay here.

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

- **A survey's `proposed_rewrite` can be right about the class and wrong about
  the code, and the schema is what lets it happen.** One row, one rewrite. When
  a source line carries two predicates of different classes —
  `if !previous_uv.x.near(&current_uv.x) && surface.uder(u, v).so_small()` — the
  survey classified for the deciding test and proposed
  `ctx.is_small_len(surface.uder(u, v).magnitude())`, which is the correct
  migration of that predicate and **silently deletes the guard**. The worker knew
  and said so in its own `reason`; it had nowhere else to put it. Four of the 26
  rows were like this, and they are exactly the four it marked `confidence: low`
  — the confidence field worked. The survey template now carries
  `predicates_on_line` and `mixed_classification`, and requires
  `proposed_rewrite` to replace the **whole condition**. **When reviewing a
  survey, grep the source lines for a second predicate token before trusting any
  rewrite.**
- **`model` means degree ONE in length, and nothing enforces that.** A
  cross-product magnitude is twice a triangle's area; a `Matrix3::determinant()`
  of two displacements and a unit direction is a scalar triple product. Both
  scale as `k²` while `ctx.length_margin()` scales as `k`, so
  `ctx.is_small_len(area)` is exactly correct at Stage A — where `model_scale =
  1.0` makes the two identical — and silently wrong the moment Stage B threads a
  real scale. **That is worse than not migrating**, because Stage B sees a
  migrated site and never looks again. Six such sites in `truck-meshalgo` were
  proposed as `model`; a worker on an earlier shard had already hit the identical
  problem unprompted and left `FIXME(BG-TOL-001)` at
  `truck-modeling/src/geom_impls.rs:91`. The spec now records this as a third
  exclusion class alongside squared-order. **The tell is in the reason text: if a
  classification's own justification contains "area" or "length-squared", it is
  not `model`.**
- **Recognise a squared-order site by its CONSTANT, not by its shape.**
  `d.distance2(c) <= TOLERANCE * TOLERANCE` is *not* the `near2` family — it is
  algebraically `distance <= TOLERANCE`, an ordinary first-order predicate
  written squared to skip a `sqrt`, and it migrates. What cannot migrate is a
  comparison against `TOLERANCE2` = 1e-12, because nothing on `ToleranceCtx`
  reproduces that number. The survey excluded a live site by getting this
  backwards, and the census miscounts the same line in the other direction —
  its SQUARED regex matches the `TOLERANCE2` *token* and not a written-out
  `TOLERANCE * TOLERANCE`.
- **A `const` item is never a migration site.** `pub const FOO: f64 = TOLERANCE;`
  has no `ctx` in scope, so any `ctx.` rewrite proposed for one cannot compile.
  Same for a `use` import, a `.max(TOLERANCE)` floor and a `+ TOLERANCE` offset:
  they compare nothing, so they have no class. Their *consumers* are the sites.
  This is the same family as the spatial-hash bucket pitch already recorded, and
  it is much wider than that one instance — four of `truck-meshalgo`'s 30 census
  "production predicates" are value computations, not predicates.
- **`census_tol_sites.py` cannot see a constant whose name ends in
  `_TOLERANCE`.** Its pattern needs a word boundary before `TOLERANCE` and `_` is
  a word character, so `SOURCE_INCIDENCE_TOLERANCE` and `RELATIVE_TOLERANCE` are
  invisible. The survey found three hits this way, one of them a live predicate
  (`source_edge.rs:311`), and `tessellation/formal/*.rs` holds **8 more
  production predicates on a hardcoded `RELATIVE_TOLERANCE = 1e-9` that are in no
  inventory at all**. Treat the census's totals as a floor. (Not fixed: widening
  the regex changes every crate's number under the shards already sized against
  it, so do it deliberately, in its own commit, and re-size.)
- **`--skeleton` and `--check` disagreed with each other, and `--check` won.**
  `gen_packet --skeleton` computed `unscaled_legacy_budget` from the survey's live
  rows (16) while `--check` validated it against the census function count (20),
  and `run_packet.py` refuses to dispatch on the mismatch — so **no
  survey-derived packet could be dispatched at all**. The census counts every
  function holding a grep hit, including ones whose only hits a packet correctly
  excludes, so the two numbers diverge by construction, not by error. The budget
  is now checked against the packet's own site table, with the census kept as a
  ceiling. **A gate whose two halves are computed from different sources is a
  gate that will eventually reject correct work.**
- **Resolve a packet's site table against the tree, not against its own text.**
  The first version of that budget parser keyed on the function name and read
  `BG-TOL-001-STEPIO` as 10 contexts against a true 15 — reproducing the exact
  undercount already in the traps (five distinct `fmt` impls in
  `truck-stepio/src/out/geometry.rs` collapsing to one). It was also brittle
  across packet styles: MESHALGO's generated table states the class in the row
  and STEPIO's states it in the section heading, so a regex tuned to one counts
  zero rows in the other and reports a mismatch that is the parser's. Resolving
  `(file, line)` to the enclosing `fn`'s definition line fixes both and checks
  one more thing for free — a table line that lands outside any function does not
  resolve, so an invented line number cannot pad a budget.
- **A green line can state something false.** Watching the new budget gate fail
  on "budget above the census ceiling" showed it printing
  `budget 25 <= census ceiling 20  ok` — the comparison silently used the counted
  value instead of the declared one. The real error was caught by the other half
  of the same check, so nothing leaked; the message was wrong anyway, and a
  wrong-but-green line is exactly what a later session reads and believes. **Read
  what a gate prints on the negative test, not just its exit code.**
- **`gen_packet --skeleton` writes UTF-8 through a cp1252 console.** Its em
  dashes came back as `?` on screen and `--check` died with `UnicodeDecodeError`
  reading back the file `--skeleton` had just produced. Fixed producer-side with
  `sys.stdout.reconfigure(encoding='utf-8')`. Any new `loop/*.py` that prints
  prose needs the same line.
- **`new_slot.py` can exceed a two-minute tool timeout and still have
  succeeded.** It warms a cold slot, which takes ~5.6 min. A timeout is not a
  failure — check `slot_status.py` and `git worktree list` before re-running it,
  or you will fork a second worktree onto the same branch.

- **"`Plane` implements `BoundedSurface` despite being unbounded" is false, and
  it reached a packet as a stated fact.** `Plane::parameter_range` is
  `(Bound::Included(0.0), Bound::Included(1.0))` on both axes; `range_tuple`'s
  `.expect(UNBOUNDED_ERROR)` cannot fire on it. `Cylinder` and `Cone` are the
  ones that are unbounded — `(Bound::Unbounded, Bound::Unbounded)` in `v` — so
  the panic the claim described is real and lives on the other types. The worker
  that was told this contradicted it in its `RESULT.json` notes and was right.
  **The packet asked for that judgement explicitly** ("report whether Plane's
  impl looked like a defect to you on reading it"), which is the only reason it
  came back; a packet that states a fact without inviting disagreement gets
  compliance instead of a correction.

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
- `gen_packet.py` exists and works end to end as of session 8. What it still
  does **not** do: `--skeleton` emits no prose, by design, and it has no notion
  of a site that needs a `FIXME` rather than a rewrite — `BG-TOL-001-MESHALGO`'s
  six deferred area sites had to be written by hand into a section the budget
  parser deliberately stops at. If a third shard needs deferrals, teach the
  survey schema an `action: fixme_marker` value and let the skeleton emit them.
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
