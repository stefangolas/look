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

Updated 2026-08-19, end of session 9. Branch: `integration/kernel-bg`. Nothing
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
- `loop/packets/BG-TOL-001-GEOM-NURBS.md` — the newest and largest worked
  example (57 sites, 26 contexts, 12 deferrals, six files), generated from a
  reviewed survey by `gen_packet --skeleton` and landed in one dispatch.
  `BG-TOL-001-MESHALGO.md` is the previous one, `BG-TOL-001-SHAPEOPS.md` the
  older hand-written shard, and `BG-S0-003.md` older still.
- `loop/packets/BG-EVD-004-r2.md` — written, checked and **undispatched**; the
  worked example of a packet that fixes a defect rather than migrating sites.
- `loop/packets/BG-TOL-001-MESHALGO-SURVEY.md` — the `class: survey` template,
  sharpened after its first run. Copy it to survey a new crate.

The loop is a **build** loop, not a search loop: acceptance is mechanical and
deterministic, so the verifier does the job an objective function would. Nothing
here is scored, tuned, or sampled.

## Where we are

**Twenty-three packets DONE of 65 (35%).** Session 11 landed the whole
**BG-CE-006-ENUM stack** — three commits on one branch (`60aee3f` the canonical
`Curve`/`Surface` model, `a08fd8f` the placed-surface variant and
branch-consistent revolution search, `b4a201d` the transversal engine's
periodic-domain unwrap) merged as `1686390`, closing three registry rows at
once via `covers:`. GATE-4 sits at **109/109**.

That merge is the one the frontier was waiting on: `schedule.py` goes from
**3 eligible to 17**, including the six `BG-ENC-002` carrier packets and the
eight `BG-ANA-001` shards.

**BG-ENC-001 has landed** (it is the reference `EnclosureSurface for Plane` in
`truck-evidence`), so the "single most load-bearing unbuilt item" note below
from session 9 is discharged; what is unbuilt now is the *carriers*.

**Session 11 was not a code session.** Almost all of it went on establishing
that three separate "regressions" charged to ENUM-r3 were the harness lying —
see the last four traps, which are the durable output. The kernel change that
landed had already passed V0–V7 before the session started.

## Pick up here

1. **The ENC-002 carrier fan-out is dispatchable now and is the frontier.**
   `BG-ENC-002-CIRCLE`, `-CYLINDER` and `-SPHERE` are written, their anchors
   check, and their registry rows are wired to their files. `-LINE`, `-CONE`
   and `-TORUS` still need writing; `plane.rs` is the template and the three
   existing packets are the model to copy.

   **They are write-disjoint by construction as of `afba979`.** All six
   carrier modules are already created and declared in
   `truck-evidence/src/lib.rs`, so no carrier packet needs to touch `lib.rs` —
   which is the one line they would otherwise all have edited, and the merge
   conflict that would have surfaced at the *second* land. Any packet written
   for a new carrier must inherit that: `lib.rs` in `read_allow`, never
   `write_allow`, and say so in "Decisions already made for you" or the worker
   will follow the `plane.rs` pattern and earn a V1 scope violation.

2. **Disk, not the DAG, is what caps the fan-out.** 17 packets are eligible in
   parallel and this machine holds about **one**. A warm slot target is
   ~7 GB and free space is ~9 GB. Two concurrent workers plus a verify does not
   fit and trying it is what produced session 11's corruption. Check
   `Get-PSDrive C` before adding a slot, and reclaim
   `%TEMP%/look-verify-baseline-*` rather than a warm target.

3. **`BG-TOL-004`** (20 squared-order sites, design class, blocks nothing) and
   **`BG-INV-107`** (design-blocked: no per-entity tau in the tree) are the
   other eligible rows. `BG-TOL-001-SMALL` is in the registry as dispatchable
   but `run_packet.py` refuses it — three of its anchor claims have rotted
   against the tree (`A1: expected 2, tree has 1`). Re-anchor it before
   re-dispatching; the watchdog burned its whole restart budget on retrying a
   packet that could never dispatch.

4. **Two spec items from session 10's external review are still unstarted:**
   - **`BG-EVD-005`** — a published modulus needs a *geometric* certificate.
     BG-EVD-004's M1-M5 constrain only the arithmetic; nothing obliges a cell
     to publish the modulus its geometry actually has, so a tangency may
     publish `Lipschitz` and every gate stays green. Its negative test is the
     point: a cell hand-built to publish `Lipschitz` for a constructed tangency
     must be **rejected**.
   - **`BG-TOL-005`** — uv tolerance is not a generic dimensionless ratio.
     `sin_margin()` and `ratio_margin()` both return `tau_rep` today: two names
     for one number, covering three quantities that do not behave alike
     (intrinsic / parameter fraction / uv length). Needs a certified
     `‖δx‖ ≤ C_sup‖δp‖` bridge with `C_sup` an upper bound on `σ_max(J_S)` over
     the cell, and a `σ_min` lower bound for the inverse direction. **The
     observation that makes it fit the existing machinery:** a cell whose
     `σ_min` bound reaches zero *is* BG-EVD-004's `Pole`. A chart singularity is
     not a special case, it is the same object.

   Both need the interval carriers item 1 is building.

5. **V8 is ON, has been watched failing twice, and no longer builds a second
   world.** It runs the full downstream suite at HEAD; if that is green the
   base is never built at all; otherwise it asks the base only about the red
   tests, one `-p <crate> --test <target> -- --exact <names>` per target, and
   caches each answer in `loop/baselines/<base>__v8-observations.json`. The
   negative test is recorded in `46accfb`/`5d86783`. **23 downstream tests are
   red at base** in truck-meshalgo and truck-stepio and are correctly ignored;
   two of them (`geometry::b_spline_curve_with_knots`,
   `geometry::nurbs_curve_b_spline_curve_with_knots`) are a real importer defect
   worth its own packet — the proptest draws knot increments down to 1e-3 and
   the conversion dies with "The scalar 0 is not positive".

6. **Then the CE chain.** Unchanged from session 9's scoping, which is filed
   below under "The CE chain, scoped against the tree".

## What session 9 changed about how the loop runs

Five harness defects were found and fixed, **four of them by running a tool
rather than reading it**. They are in Traps with the evidence; the short list:

- Both site counters attributed a line to the last `fn` seen without tracking
  braces. `loop/rustscan.py` is the single implementation now, with
  `loop/selftest_rustscan.py` watching the old scan fail on the same fixture.
- `gen_packet` resolved a table's file heading by bare-basename `endswith`, so
  `curve.rs` matched `polyline_curve.rs` and **every row under an ambiguous
  heading was silently dropped** while printing "all checked claims hold".
- `new_slot` used `rev-parse --is-inside-work-tree` to decide whether a path was
  a live worktree. That is true for *any* directory inside the repo, so a stub
  `wt/` was treated as live and `checkout -B` + `reset --hard` **ran against the
  main worktree**. Nothing was lost, by luck.
- `PACKET.md` and `RESULT.json` are worker scaffolding and were riding into the
  integration branch on merge; `PACKET.md` then conflicts on the *next* merge,
  because every slot's copy is a different packet.
- **`loop/check_metric_bound.py` is the one worth keeping.** The
  `near_points` bound has now bitten three shards. It resolves every `model` row
  to its enclosing impl or free fn and reports MIGRATES / BLOCKED / CHECK.
  Validated against all three shards whose answers were already known by hand —
  DECORATORS 2, NURBS 12, SMALL 2 — **zero false negatives on 47 rows**. Run it
  before writing any TOL packet.

## Did the session-8 tooling bet pay? Yes, with a visible ceiling

**For.** Three shards in one session, the largest of them 3× anything before it,
for $0.18. `gen_packet --skeleton` produced the front block, anchors, both site
tables, the deferral table and the budget from reviewed surveys; almost none of
the three packets was typed by hand.

**Against, and this is the finding.** Every round trip this session was the
orchestrator's, and none was of a kind a gate could have caught:

| packet | fault | what it was |
|---|---|---|
| NURBS | PACKET | demanded a rewrite inside a `pub const fn` while forbidding signature changes; `unscaled_legacy()` is not const |
| DECORATORS | PACKET | never ran the `near_points` bound check over its model rows |
| SMALL | GATE | verify handed a base from before the slot forked, so the orchestrator's own commit entered the packet's diff |

**43% of a packet is templatable and the remaining 29% is exactly where all
three landed.** Assembly is cheap now; judgement is the entire remaining cost.
Two of the three are now mechanised anyway (`check_metric_bound.py`, and
`--base` is a checkable fact), which is the right response — but the general
lesson is that further *generation* tooling has a low ceiling, while tooling
that **checks a claim against the tree** keeps paying.

## The parallelism picture

62 packets: 38 mechanical, 13 design, 11 wide-mechanical. 43 remain, and the
frontier is suddenly narrow: `schedule.py` reports **3 eligible**, and two of
the three are design class (`BG-CE-006-ENUM`, `BG-TOL-004`). The one mechanical
row is `BG-INV-107`. The wide-mechanical TOL seam that has carried the last four
sessions is finished, so the loop's throughput is now bounded by design packets
the orchestrator writes by hand. The distribution is the story: **ENC 14, INV 9, ANA 8, FID 4 are all at zero** —
35 packets, 56% of the graph, none started. That is the certification half of
the kernel. What has landed is the plumbing: a tolerance model with dimension in
its type, an error-propagation algebra, a refusal taxonomy, budget accounting.

Three concurrent workers on write-disjoint packets ran without interference for
the first time this session. The constraint is no longer worker throughput or
orchestrator packet-writing; it is **disk** (see Traps) and **design-class
packets**, of which the CE chain alone is three.

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

- **A crate that denies its own lints makes V3 unpassable, and "could not
  compile" does not mean "was not linted".** `truck-meshalgo/src/lib.rs` carries
  `#![deny(clippy::all, rust_2018_idioms)]`, so its ~93 pre-existing lints are
  hard errors whatever V3 puts on the command line; cargo then reports "could not
  compile ... due to 93 previous errors" and V3's coverage guard fired on that,
  short-circuiting the added-line scoping which is the gate's whole purpose.
  Every finding was in files the diff never opened and V2 passed on the same
  tree. **The distinguishing marker is `error[E####]`** -- rustc gives a real
  compile error a code and never gives one to a lint. Fixed; a crate counts as
  unlinted only when an E-coded diagnostic is present, so the guard keeps its
  property. Watched failing three ways first.
- **GATE-1 requires `#![deny(clippy::unwrap_used)]` on every new module under
  `vendor/truck/`, including `tests/*.rs`, and a packet that does not say so gets
  rejected for the orchestrator's omission.** BG-TOL-001-MESHALGO's packet never
  contained the string `unwrap_used`; both landed shards' test files carry the
  attribute, so the convention existed and only the packet was missing it. Fixed
  by amendment rather than redispatch. **Put this line in every packet that asks
  for a new test file.**
- **Do not read a verdict off a background wrapper's exit code.** A command of
  the form `python loop/verify.py ... > log; echo "EXIT: $?" >> log` exits with
  the *echo's* status, so the task notification says 0 while the log says
  REJECTED. This was reported to the user as an acceptance before the log was
  read. **Read `VERDICT.json` or the `VERDICT:` line, never the harness's exit
  notification.**
- **`kill -0 <pid>` in Git Bash cannot see a Windows PID and reports the process
  dead.** A watch built on it fired instantly with "pid gone" while the worker
  was alive and working. Use `tasklist //FI "PID eq N"`, or better,
  `slot_status.py`'s own state.
- **`slot_status.py`'s 12-minute STALLED threshold is mis-calibrated for this
  worker.** Measured on BG-TOL-001-MESHALGO: **75% of the run's wall clock was
  model latency**, in five gaps of 6-10 minutes, some of them followed by a
  `grep` on a local file that takes milliseconds. A 12.9-minute gap tripped
  STALLED on a worker that then resumed and finished normally. **`--kill-stalled`
  on that label would have destroyed an hour of correct work.** Confirm with
  `tasklist` and check whether cargo/rustc are running before reaping anything.
- **A bash heredoc eats backslashes, and it has now corrupted three separate
  Python patches in one session** -- `'\'` collapsing to `''` (unterminated
  string), and `'\b'` arriving as a literal 0x08 backspace inside a regex that
  then silently matched nothing. **Write Python patch scripts to a file and run
  the file, or use the Edit tool; never pipe a patch containing regex escapes
  through a heredoc.**
- **Worker cost is not a constraint and orchestrator time is.** The whole
  BG-TOL-001-MESHALGO run cost **$0.057**; all 49 remaining packets extrapolate
  to about **$2.78**. Parallelism therefore buys wall-clock but never credits,
  and it *raises* orchestrator load because every finished worker needs
  adjudicating. **43% of a packet's text is templatable boilerplate and only
  ~29% is real judgement** -- that ratio, not slot count, is where the leverage
  is.

- **`--base` is per-slot, not per-session, and slots forked at different moments
  have different fork points.** Session 9 created slot 0, then committed a
  harness fix, then created slots 2 and 4. Verifying slot 2 with slot 0's base
  put `loop/new_slot.py` -- the orchestrator's own commit -- inside the packet's
  diff, and V1 correctly rejected it for a file no worker touched. The gate was
  right and the invocation was wrong. **Read the fork point off the slot**
  (`git -C loop/slots/N/wt rev-parse HEAD~1` for a single-commit packet) rather
  than reusing the base from the last verify.
- **`PACKET.md` is worker scaffolding and must not reach the integration
  branch.** `run_packet.py` writes it into each worktree root; a `--no-ff` merge
  carries it in, and the *next* packet's merge then hits
  `CONFLICT (content): PACKET.md`, because every slot's copy is a different
  packet. It stopped two landings in a row before it was removed from tracking.
  This is the same rule already recorded for `RESULT.json` and it applies for
  the same reason: the authoritative packet is in `loop/packets/`, the
  authoritative result in `loop/results/`, and neither belongs in the repo root.
- **`git add -A` at the repo root sweeps in deliberately-untracked work.** This
  repo carries a dozen untracked analysis documents, `benchmarks/` outputs and
  all of `scratch/`. One `git add -A` put 196 files into a commit before it was
  caught and the commit rebuilt from `git add loop/ docs/ scripts/`. Stage the
  paths the commit is actually about.
- **A `model` site can be correctly classified and still unmigratable, and it
  has now cost three shards.** `ctx.near_points<P>` needs
  `P: MetricSpace<Metric = f64>`; `P: ControlPoint<f64> + Tolerance` does not
  supply it, and `Homogeneous::Point` is only `EuclideanSpace`. **Run
  `python loop/check_metric_bound.py <survey.json>` before writing any TOL
  packet.** It resolves every `model` row to its enclosing impl or free fn and
  reports MIGRATES / BLOCKED / CHECK; validated against all three of session 9's
  shards, whose answers were known by hand, with zero false negatives on 47
  rows. Two sites with the *same function name and the same shape* can take
  different rewrites -- `truck-geotrait`'s two `search_parameter`s do.
- **`const fn` blocks a migration outright.** `nurbs/mod.rs`'s `inv_or_zero` is
  a `pub const fn` and `ToleranceCtx::unscaled_legacy()` is not const, so no
  context can exist in that body. The packet demanded the rewrite while its
  Forbidden clause banned signature changes; the worker dropped `const`,
  reported the contradiction, and was right that both could not hold. Now a
  named exclusion class, `FIXME(BG-TOL-001, CONST_FN)`. **Grep a shard's site
  list for `const fn` before writing the packet.** This is the first exclusion
  where the classification and the rewrite are both *correct* and the enclosing
  item is what blocks them.
- **GATE-4 counts `unscaled_legacy(` anywhere in the file, comments included.**
  A `FIXME` explaining why a site cannot be migrated will naturally name the
  constructor, inflating the ratchet by one and making a *deferral* read as a
  *migration*. The NURBS amendment's own FIXME did exactly this and held GATE-4
  at 76 when the tree had 75. Write the name without its parentheses in prose.
- **A checker can DROP rows rather than miscount them, and print green either
  way.** `gen_packet.packet_contexts` resolved a site table's file heading with
  a bare-basename `endswith`, so `curve.rs` also matched `polyline_curve.rs`;
  two candidates made the heading ambiguous and **every row beneath it was
  skipped silently** while the tool printed "all checked claims hold". Two
  packets read low by one and two contexts. A skipped row is never resolved
  against the tree at all, which is the one thing that function exists to do.
  A second bug hid the first: the no-table path returned a hardcoded `[]` for
  `unresolved`, so the guard could not have reported it even once it existed.
- **`git rev-parse --is-inside-work-tree` cannot tell a linked worktree from any
  directory inside the repo, and `new_slot` used it to decide.** A stub
  `loop/slots/N/wt/` holding a leftover `vendor/` was classified live, and the
  idempotent path then ran `git -C wt checkout -B packet/<id>` and
  `reset --hard` -- both repo-wide, both resolving to the **main worktree**,
  which was silently moved onto a packet branch. Nothing was lost only because
  every ref sat on the same commit and the tree was clean. The test is now
  `rev-parse --show-toplevel` compared against the path itself, plus a flat
  refusal to treat the repo root as a slot.
- **An interrupted verify leaks its baseline worktree, and that is what actually
  fills the disk.** `compute_baseline`'s cleanup does not run when the process
  is killed. Two leaked baselines plus one live one took session 9 from 9.4 GB
  to 3.1 GB and cost it a completed run. `verify.py` now names them and their
  size both in the 8 GB refusal and as a warning while there is still room to
  act; the deletion is still yours.
- **`loop/slots/*/target` is only half the disk the loop owns.** Workers create
  a *second* `target/` **inside** the worktree -- `loop/slots/N/wt/target` --
  despite `CARGO_TARGET_DIR` pointing elsewhere, and it is the larger of the
  two: 1.9 GB against 0.9 GB in one slot. Every recovery recipe in these
  documents named only the outer one. `slot_status.py --disk` reports both.
- **Most of this machine's missing disk is not the loop's.** Session 9 found
  **19 GB** of regenerable junk in `%TEMP%`: 118 `chrome-lite-*` throwaway
  browser profiles at ~125 MB each (14.7 GB) and 28 `proc-macro-srv*`
  rust-analyzer server copies (4.2 GB), all under three days old. Check those
  before concluding the loop needs a smaller footprint.
- **A dead worker can leave its `cmd.exe` shim alive, so a pid check says
  RUNNING.** Two workers died in an ~8-hour gap with their recorded pids still
  resolving. What separates them is `events.jsonl` mtime *plus* whether any
  `cargo`/`rustc` process exists at all. `slot_status.py` shows `DEAD?` when the
  log has been silent over an hour and no toolchain process exists. **It is a
  prompt to look, never grounds to reap** -- the opposite mis-calibration on
  `STALLED` is recorded above and cost an hour of correct work.
- **Never background a verify through a shell wrapper.** `nohup python
  loop/verify.py ... &` inside a backgrounded compound either never runs or is
  orphaned, the wrapper reports exit 0, and the **stale `VERDICT.json` from the
  previous packet is still in the slot**, ready to be read as this one's.
  Session 9 read an ACCEPTED verdict for MESHALGO and nearly believed it was
  NURBS's. Check `VERDICT.json`'s `base` and `commit` match the run you think
  you are reading.
- **`cd` persists across a compound command and `-F` resolves against it.** A
  `cd` into a slot worktree followed by `git commit -F scratch/msg.txt` looked
  for the message *inside the worktree* and died. The `git -C` rule is not only
  about committing to the wrong branch; use absolute paths for `-F` too.
- **`land_packet.py` must run BEFORE the stray `RESULT.json` is deleted.** It
  reads `RESULT.json` from the repo root -- where it arrives on the merge -- and
  files it. Deleting it first makes `land_packet` die with `FileNotFoundError`.
  Order: merge, `land_packet`, then delete.
- **A reaper that reads `worker.pid` thinks a verifying slot is idle.**
  `worker.pid` disappears the moment the worker writes `RESULT.json`, but
  `verify.py` then spends 10-30 minutes compiling in that same `target/`.
  `watchdog.py`'s `guard_disk` read slot 0 as idle and `rmtree`d
  `loop/slots/0/target` under a live cargo three times on 2026-08-19 (its own
  log, `22:33:04 ACTION disk 3.4 GB free: reclaimed 3.7 GB`, and at 22:43:20 an
  `Access is denied` on a `.dll` it was deleting while cargo held it open).
  The resulting `error[E0786] found invalid metadata files for crate`,
  `error[E0463] can't find crate for truck_stepio` and `failed to write
  ...dep-lib-truck_meshalgo` were all diagnosed as **code regressions in the
  packet under test**, on unchanged source, after repeated clean builds. Two
  hypotheses died on that. `verify.py` now writes `loop/slots/<N>/verify.pid`
  and the watchdog reclaims nothing while one is alive; it also takes leaked
  `%TEMP%/look-verify-baseline-*` worktrees, which are pure garbage from killed
  verifies, before any warm target. **Deleting a warm target is not a disk
  strategy** -- it frees the same bytes and then charges the next verify a full
  cold rebuild, which is how that session made every retry cost more than the
  last.
- **Never cache a baseline whose build did not compile.** Such a file measures
  the disk, not the base commit, and afterwards it is indistinguishable from a
  real one and is trusted by every later verify against that base. A cached
  `a08fd8f` baseline recorded `geometry::b_spline_curve_with_knots = ok` while
  the test failed 3/3 when run at that exact commit; V8 charged r3 for a
  failure it had not caused, twice, and the session's response was to weaken
  the gate (`bfb598b`, since reverted) on a flakiness theory that direct
  measurement had already falsified. **Ask whether the evidence is real before
  you ask whether the gate is wrong.**
- **cargo splits test output across two streams and `stdout + stderr` destroys
  the order.** The `Running <target> (<exe>)` banners go to stderr, the `test
  name ... ok` lines to stdout; captured as two pipes and concatenated, every
  banner lands after every test line and no test can be attributed to the
  target it ran in. `invoke_native` and both baseline runners now merge stderr
  into stdout (`stderr=subprocess.STDOUT`). V8's narrow base query depends on
  that attribution: without it every failing test falls back to rebuilding the
  whole downstream workspace at base, which is the cost the redesign removes.
- **A base build failure is never the packet's fault.** The base commit
  predates the packet; if it will not build, that is disk, toolchain or a
  corrupt target dir. V8 now exits **BLOCKED** rather than REJECTED when it
  cannot get an answer out of the base, and caches nothing from that run.

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

## The CE chain, scoped against the tree (session 9)

Seven of the nine BG-INV checkers gate on `BG-CE-003` through
`BG-CE-006-CYL-CONE -> BG-CE-006-ENUM -> BG-CE-001 -> BG-CE-003`. This is
what the tree says about the last two, as opposed to what the spec says.

- **`BG-CE-001` is mostly assembly and its design is already written.** The
  spec gives the struct verbatim — add `pcurve: Option<PC>` beside the
  existing per-use `orientation`, with `PC = ()` defaulting so `None`
  reproduces today's behaviour. truck's `Edge` is *already* a coedge. The
  work is a wide ripple: **25 files mention `Edge<` across six crates**
  (topology 8, shapeops 6, modeling 6, meshalgo 2, assembly 2, stepio 1).
  That write set is determinable now and getting it wrong is the single most
  common cause of a V1 rejection in this loop.
- **`BG-CE-002`'s certification is real math and is not assembly**:
  `‖Γ_f(pc_u(t)) − c_e(φ_u(t))‖ ≤ τ_e` for **all** t by interval evaluation
  over the whole span, gated on `BG-ENC-001`. Sampling is the classic false
  pass and the spec says so.
- **`BG-CE-003` is half-designed.** `EntityId` is spec'd as an enum, but
  **`Selector`, `OpId` and `Op` are defined nowhere in the tree** — checked —
  so `Sel { base, selector }` is a name, not a design. That algebra can be
  built and property-tested as a standalone module with no truck code
  involved, and that is where the design risk lives.
- **One spec correction, found by running its own command:** BG-CE-003's
  prose says "10 documented deadlock hazards" and its command says expect 12.
  **12 is right** — exactly 2 each in `edge.rs`, `face.rs`, `shell.rs`,
  `solid.rs`, `vertex.rs`, `wire.rs`. Fix the prose; a stale number in the
  item seven checkers gate on will be believed.

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
