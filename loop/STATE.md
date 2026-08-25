# Autobuild loop - STATE

Rewritten at the end of every session. The **volatile** part - everything from
"Where we are" through "The parallelism picture" - is capped at ~120 lines and
must be rewritten each time. "Traps" and everything below it is **stable and
accumulates**: entries are added when something costs a session and removed only
when they stop being true, never for length. (The old header claimed the cap
covered everything above "Quick reference"; it never did, and pretending
otherwise would eventually cost a trap.) If you are picking this up cold, read
**this file, then
[`loop/ORCHESTRATOR.md`](ORCHESTRATOR.md) for how to run the loop, then
`python loop/slot_status.py`** - nothing else. Do not read `LEDGER.jsonl` whole.

Updated 2026-08-25, close of session 30 (M1 finish: PCURVE SPEC_GAP landed + plan amended; Contact Layer funnel STARTED, skeleton landed). Branch: `integration/kernel-bg`, HEAD `1155186`.

## Where we are

**M1's construction is fully green AND its last open item is closed; the
Contact Layer funnel is now running.** Two packets landed this session:

- **BG-SOL-S2-PCURVE** (SPEC_GAP probe, `56184b2`, verified ACCEPTED at
  `36d2e82`, merged `4f204d3`) — proved empirically that pcurves cannot ride on
  the returned `Solid`'s edges: `Wire<P,C>` holds `VecDeque<Edge<P,C>>`
  (`PC = ()` by default) and `with_pcurve<Q>` returns `Edge<P,C,Q>`, so a real
  `PCurve` edge cannot enter a Wire (compile probe E0277). Plan amended (§2,
  §4 Phase 2, §7 M1): pcurves on the solid's edges are deferred to a cross-crate
  topology-PC program (BG-CE-001 anticipated it); M1's "exercises pcurves" is
  recorded as satisfied by the S2 construction + the `PCurve<C,S>` carrier and
  same-parameter invariant existing.
- **BG-SOL-S3-CONTACT** (first Contact Layer packet, `e9ecd2a`, verified
  ACCEPTED at `bc434d5`, merged `9eae9ae`) — the funnel's skeleton:
  `BoundedStratum { Face/Edge/Vertex }` over **canonical carriers** (strata are
  geometry-side: truck-evidence cannot name truck-topology), `ContactComplex`,
  `ContactLocus`, and `contact(&lhs, &rhs, budget)` dispatching C0-C2 identity
  (struct-equal canonical carriers → Coincident/IdenticalCarrier) then the FF
  analytic table (plane_plane/plane_sphere/sphere_sphere/plane_cylinder/
  plane_cone, both orientations). Everything else — FE/EE, cylinder×cylinder,
  Torus/Placed, 2-D overlap — refuses honestly with the new
  `EnvelopeCase::ContactReductionDeferred` arm. Signature deltas from the plan's
  §4 booking are recorded in the plan doc (§4 Phase 3) and in the packet's
  RESULT notes (by-value `BoundedStratum` infeasible; `CanonicalSurface` has no
  `Unrecognized` arm, so spline refusals live at the `face_stratum` lift).

Registry is 96 rows: **95 DONE, 1 BLOCKED** (BG-AUD-FIX-004, owner), **0 READY**.
`schedule.py` reports eligible 0, dispatchable 0. Nothing is running except the
watchdog.

## Pick up here

The Contact Layer funnel is the active program; M1's construction is green and
its pcurve item is closed. The funnel's dispatch stages, in plan order, and
their current state:

1. **C0-C2 identity/overlap + analytic FF** — LANDED (S3-CONTACT).
2. **Strata reductions (FE, EE via curve machinery)** — the next packet
   (FE: `Edge` × `Face`; EE: `Edge` × `Edge`). Write it against the landed
   `contact` API (anchor the CONSUMING signature per the session-28 trap) and
   the §4 Phase 3 amendment. It will consume `BoundedStratum` and extend the
   `ContactLocus`/`ContactRecord` vocabulary with the bounded locus forms.
3. **Cylinder×Cylinder and the remaining analytic-pair families** (parallel/
   equal-radius/coaxial) — still `ContactReductionDeferred`; a follow-up packet
   fills the table.
4. **General validated FF → singular event cells → 2-D overlap (C3/C4, last)** —
   the hard funnel, deliberately delayed.
5. After the funnel: **Boundary Rewrite (Phase 4)** and then the M2 flagship
   differential test `Extrude(P−Q) ≅ Extrude(P)−Extrude(Q)`.

Recommended, NOT dispatched: the follow-up audit of
`truck-evidence/src/fid/rep.rs` (4362 lines; sole sanctioned exact→emitted
path; last touched by BG-FID-005 / BG-FID-005-SRF). Its own program.

## State of the machine, as left

- **Watchdog RUNNING** (lock `loop/watchdog.lock`; heartbeat current,
  `stagnant=3600s`).
- Registry: 96 rows = 76 original + 11 AUD-FIX + 4 SOL-P0 + S1 + S2 + ORIENT +
  PCURVE + S3-CONTACT. **95 DONE, 1 BLOCKED** (BG-AUD-FIX-004 owner), 0 READY.
  `schedule.py` eligible 0, dispatchable 0.
- Slots 0-3 FINISHED on landed branches. Slot 0 holds a stale RESULT.json
  (harmless; re-fork with `new_slot.py`). No leaked
  `%TEMP%/look-verify-baseline-*`.
- Disk ~14 GB free at close. No leaked `%TEMP%/proc-macro-srv*` beyond
  per-session regrowth.
- GATE-4 ceiling 111 (true count). `scripts/kernel-gates.sh HEAD` passes
  111/111. No `unscaled_legacy()` calls added by any solver-family packet
  (PCURVE, S3-CONTACT both ceiling-neutral).
- PCURVE and S3-CONTACT ledgered and filed under `loop/results/` (both verified
  ACCEPTED at their fork points).

## The parallelism picture

Nothing running (watchdog only; no workers, no verifies). Next work is the
Contact Layer funnel (item 2 above: the FE/EE strata-reduction packet), then
the remaining analytic-pair families, then the funnel stages, then Phase 4. The
fid/rep.rs audit stays a recommendation.

## Traps, each one paid for
### Session 30 (M1 finish: pcurve probe landed + plan amended; Contact Layer funnel started) - paid in full

- **A V9 `look` baseline is NOT cached by the earlier V5/V8 baselines at the
  same base, and its absence can kill a verify at the 8.0 GB disk floor even
  when everything before it passed.** The S3-CONTACT verify's first run passed
  V0-V8, then died computing the V9 `look` geometry baseline at `bc434d5`
  (`RuntimeError: refusing to compute a baseline: 7.7 GB free`). The V5/V8
  caches key on the packet's crates, not on `look`; the `look` baseline is a
  separate key that only the V9 gate builds. The earlier repo-root `target/`
  (5.75 GB of idle regenerable junk) plus `%TEMP%/proc-macro-srv*` were the
  reclaim; after freeing to 27 GB the re-verify passed. Recovery is the usual
  disk recipe, but the surprise was *when*: a verify can be almost done and
  still die on disk. Check `Get-PSDrive C` before every verify, not only before
  a run of several.
- **A stale VERDICT.json from a previous packet on the same slot reads as a
  fresh ACCEPTED if you do not check its `commit` field.** The ORIENT
  VERDICT.json (base `f1052ad`, commit `cb96f64`) survived the S3-CONTACT
  verify's early stage and my first poll read `verdict=ACCEPTED` from it while
  the S3 verify was still running. The ORCHESTRATOR.md warning says check `base`
  AND `commit`; the `base` alone (both were different commits) would have
  passed. Always confirm `commit == the slot's HEAD` before trusting a verdict,
  and delete the stale file between packets so the next poll cannot see it.
- **The uncommitted-RESULT landing dance happened again (S3-CONTACT) and the
  recovery is the recorded one, with one extra check.** The S3 worker left
  RESULT.json uncommitted; `land_packet.py` would have died at the filing step.
  Before copying, `git -C loop/slots/N/wt ls-files | Select-String RESULT.json`
  told me it was untracked, so the copy-first dance applied: `Copy-Item
  loop/slots/N/wt/RESULT.json` → repo root, re-run `land_packet.py`, then
  `Remove-Item` the root copy (the script's `git rm` only removes tracked
  files). The extra check (`ls-files`) is cheaper than reading the merge stat
  and answers the same question.
- **`land_packet.py` compares the packet path against the verdict's stored
  `packet` field, which is stored with backslashes on Windows.** Passing
  `loop/packets/...` (forward slashes) failed with "the verdict in slot 0 is
  for 'loop\\packets\\...', not 'loop/packets/...'". The fix is to pass the
  packet path with backslashes (`loop\packets\BG-SOL-S3-CONTACT.md`), matching
  how verify.py recorded it. Worth one line in the landing instructions: on
  this host, pass the Windows-form path to `land_packet.py`.

### Session 29 (M1 finish: face orientation landed, pcurve probe written) - paid in full

- **`land_packet.py` can crash AFTER a successful merge, and the fix is one
  `Copy-Item` + one `Remove-Item`, in that order, never a re-run first.**
  The ORIENT worker left `RESULT.json` UNCOMMITTED in the worktree (the
  uncommitted flavor). `land_packet.py` merged `--no-ff` successfully, then died
  at the filing step: it reads `REPO_ROOT / 'RESULT.json'`, which the merge
  never created because the file wasn't tracked. Recovery: copy
  `loop/slots/N/wt/RESULT.json` to the repo root, re-run `land_packet.py` (the
  second merge is a no-op "Already up to date"), then `Remove-Item` the root
  copy — the script's own `git rm -q RESULT.json` only removes TRACKED files, so
  the untracked copy silently survives and would be the "untracked file would be
  overwritten by merge" trap for the NEXT landing. Check whether the worker
  committed RESULT.json (read the merge stat for a `create mode 100644
  RESULT.json` line) before deciding whether to copy.
- **`Set-Content -Encoding UTF8` in PowerShell writes a BOM, and
  `loop/PACKETS.jsonl` breaks with `Unexpected UTF-8 BOM`.** Editing the
  registry via PowerShell to flip a row's `status` field introduced a BOM that
  made `schedule.py`'s `json.loads` die on line 1. Recover with
  `[System.IO.File]::ReadAllText(...).TrimStart([char]0xFEFF)` and write back
  with `UTF8Encoding($false)`. Prefer editing `PACKETS.jsonl` with the Edit tool
  or a python one-liner that round-trips through `json.loads`/`json.dumps`,
  never a PowerShell `Set-Content`.
- **The origin-`z==0` face dispatch in a packet's suggested test is ambiguous in
  a cap-and-prism solid: every side plane's origin is a bottom vertex at z=0,
  and the y==0 side face's origin equals the bottom cap's origin `(0,0,0)`.**
  The ORIENT packet's suggested face identification ("plane whose origin() has
  z==0.0 / 2.0 or x/y==0.0 / 4.0") cannot tell the bottom cap from a side face
  by origin alone. The worker disambiguated caps from sides by boundary-wire
  count (caps carry 2 wires, sides 1) and recorded it in notes — correct, and
  the packet's sample-point table was still usable once the dispatch is fixed.
  When a packet tells a worker how to identify faces, dispatch on wire count or
  surface type, not origin coordinates.

### Session 28 (M1 construction: S1 arrange + S2 extrude) - paid in full

- **The plan's §4 target signatures are infeasible until validated against the
  landed modules, and the SPEC_GAP is the cheap detector.** `extrude_profile(
  &Arrangement, height)` was booked in the plan, but the landed S1 arrangement
  carries no carrier geometry (`ArrHalfEdge.curve` is an index into the profile
  slice, which the arrangement-only signature never receives) and a full circle
  is not determined by its seam vertex plus a 2π window. The S2 worker returned
  SPEC_GAP with the empirical proof (three unknowns, two constraints); the
  amendment added the `&[Curve]` argument and the plan doc now records it.
  Lesson: a Phase-N packet that consumes a Phase-(N−1) module must anchor the
  CONSUMING signature against the landed API (the anchors did catch the types;
  the signature's feasibility is the thing to double-check).
- **S1 normalizes every loop to its CCW representative, so winding is
  orientation-unsigned and cannot distinguish a hole from its plate.** The S2
  packet's `winding == 1` material rule selected BOTH the plate and the hole
  (reversing the circle changes nothing). The fix is a containment/nesting
  rule: material = bounded `winding == 1` regions not strictly inside another
  bounded `winding == 1` region's boundary cycle. Recorded here because any
  future "winding decides materiality" assumption will hit the same wall.
- **A single-wire face boundary containing a closed self-loop cannot close a
  shell.** The packet's cylinder wall `[circle, seam up, top circle, seam down]`
  is not simple (the seam vertex appears twice → `NotSimpleWire`) and its seam
  edges occur once in the shell → `shell_condition() != Closed`. The correct
  construction (worker-verified, `Solid::try_new` Ok with 7 faces): the hole
  wall is an ANNULUS with two boundary wires (bottom + top circle self-loops)
  and NO seam edges; each circle edge is shared by exactly two faces with
  opposite orientations.
- **`Wire::mapped` and `Edge::debug_new` PANIC in debug builds on a SameVertex
  self-loop.** The closed circle edges must be built with
  `Edge::new_unchecked(front, back, curve)` (the BG-TOL-001-MESHALGO precedent),
  and the top cap's wires must be constructed explicitly, never by mapping the
  bottom wires.
- **The bspcurve proptest flake hit a third time, and the recovery is the
  recorded one.** `truck-geometry/tests/bspcurve.rs::parameter_random_tests`
  failed once in S2's first verify (a file the packet never touched); it passed
  4/4 re-runs at the branch commit and the re-verify ACCEPTED. No
  `proptest-regressions` artifact was persisted this time. Before believing a
  V5 failure in a file the packet did not open, re-run at both commits.
- **A packet's §1 "module NOT yet in the tree" prose goes stale the moment the
  orchestrator scaffolds the module.** The S2 packet was written pre-scaffold,
  then the module was scaffolded before dispatch; the worker correctly refused
  to edit `lib.rs` (outside `write_allow`) and filled the existing file. When a
  scaffold exists, the packet's §1 must say so and keep `lib.rs` out of the
  write set.

### Session 27 (solver-family Phase 0) - paid in full

- **Four cold workers on one disk is a uv_spawn massacre, and the symptom is
  not the cause.** Dispatching the 4-wide wave with cold slots (and slot 0
  still holding 9.5 GB of the previous packet's target residue) drove free
  disk to 6.5 GB; three workers died with `EUNKNOWN: unknown error, uv_spawn`
  in `events.jsonl` — a process-spawn failure from resource exhaustion, not an
  API-balance death and not a code error. The recoveries worked (nothing was
  committed, so `new_slot.py` + `run_packet.py` redelivered cleanly), but the
  real fix is prophylactic: **delete a slot's old target residue at fork time,
  forbid workspace-wide `cargo` commands in packets** (a cold `cargo check
  --workspace --all-targets` is ~9 GB), and stagger or serialize the cold
  builds when slots are cold.
- **Verifies are also a disk risk, and different bases do NOT save you.**
  V8 runs the whole downstream workspace into the slot target (truck-base
  verify grew a target to 7.65 GB, truck-geometry to 5.84 GB). Two verifies at
  DIFFERENT bases ran concurrently this session and still drove disk to 3.1 GB,
  and the second (SPAN) died at `compute_baseline`'s 8.0 GB floor — a harness
  refusal, not a verdict. Recovery was freeing the landed PRED slot's target
  and re-running. **Run verifies sequentially always**, and free a landed
  slot's target before launching the next verify.
- **An anchor that counts occurrences of `clippy::unwrap_used` in the packet's
  OWN target file cannot stay constant.** The H-1 deny header is one
  occurrence; the packet-mandated test-module
  `#[allow(clippy::unwrap_used, clippy::expect_used)]` is a second. Both REC
  and BVH workers hit this; REC kept the count at 1 by writing unwrap/expect-
  free tests, BVH let it become 2. The anchor's intent was "the H-1 header is
  present", which is a presence check — count a stable string, or accept that
  the anchor is a pre-dispatch-only scaffold check and say so in the packet.
- **Machine-check witnesses for f64 REPRESENTABILITY, not just exact rational
  arithmetic.** The orient2d escalation witness used `(1e16+1, 1e16+3)` as
  integer literals; the exact-rational check computed det = −2 on the
  MATHEMATICAL integers, but `1e16+1`/`1e16+3` round in f64 to `1e16`/`1e16+4`,
  making the actual float determinant +2e16 and the filter CONCLUSIVE — the
  test could not exercise the escalation path it existed for. The worker
  caught it and substituted coordinates < 2^53 (all exactly representable)
  preserving det = −2 and filter inconclusiveness. `Fraction`-checking the
  integer literals is not enough; check the f64 values the literal parses to.
- **A parallel wave needs its shared vocabulary scaffolded BEFORE the wave, or
  the dependent packet defines its own copy and a post-merge amendment must
  reconcile it.** The SPAN packet's `SpanRecord.derivative_hull` logically
  depended on the BVH packet's `truck_base::bvh::DerivativeBounds`, but at the
  SPAN worker's fork point the BVH types had not merged. The worker correctly
  defined a local identical `DerivativeBounds` and documented it; after BVH
  merged, an orchestrator amendment (`14688e6`) replaced the local copy with
  the shared type (structurally identical, so the swap was a few lines and the
  span tests stayed green). Two same-named public structs in one workspace is
  the trap to avoid; scaffold shared types into the leaf crate before a wave
  that consumes them.
- **The watchdog's disk guard is load-bearing and it works.** At 3.3 GB free
  during the verify phase it reclaimed `loop/slots/1/target` (7.0 GB) and the
  machine kept breathing. The trap from session 25 ("reclaims nothing while a
  verify.pid is alive") protected the live verify; the landed slot was fair
  game.

### Session 25 (BG-AUDIT-001 remediation close) - paid in full

- **The RESULT.json landing dance has a fourth flavor that is the first three
  combined, and it cost one merge refusal.** FIX-008's worker left RESULT.json
  UNCOMMITTED in the worktree while FIX-011's worker COMMITTED it. Landing
  FIX-008 first required copying `loop/slots/1/wt/RESULT.json` to the repo
  root before `land_packet.py` (uncommitted flavor), and that root copy is
  UNTRACKED, so `land_packet.py`'s `git rm -q RESULT.json` (which only removes
  TRACKED files) silently did nothing and left it behind. The very next
  `land_packet.py` for FIX-011 then failed its merge: `The following
  untracked working tree files would be overwritten by merge: RESULT.json`,
  because the committed flavor's merge needs to CREATE that file. Recovery was
  one `Remove-Item RESULT.json`, but the ordering lesson is load-bearing: when
  landing a MIXED pair (one uncommitted, one committed), delete the untracked
  root copy after the first landing, BEFORE the second. **An untracked root
  RESULT.json is invisible to `git rm` and lethal to the next merge that wants
  to bring one in.**
- **Two fresh dispatches can both die of balance after a session boundary and
  the dead slots present as "clean, no work" - the read-before-redispatch rule
  applies to slots that LOOK empty.** This session's handoff recorded FIX-008's
  first worker dying of API Insufficient Balance with no commits; the slot was
  reset and the fix redelivered cleanly this session. The rule is unchanged and
  it saved nothing this time (the reset was already done) - recorded so the
  next session does not re-derive it.
- **This session's whole campaign close was the cheapest possible version of
  the API-balance trap: the API was live again (this session itself is proof),
  so the two pending packets dispatched, ran to DONE in ~30 min each, verified
  ACCEPTED at the shared fork point `61baa58` sequentially, and landed without
  a single rejection.** FIX-008's one deviation (regression test 1 builds the
  degenerate `[e, e.inverse()]` face via `Face::new_unchecked` because fix 1
  makes `Face::new`->`try_new` refuse that wire) was recorded by the worker and
  judged correct: the closedness gate is still exercised, all three assertions
  unchanged. FIX-011 needed zero deviations. Both packets needed NO
  re-ranking, NO amendments, and NO round trips - the write-disjoint split and
  the packets themselves were right on the first dispatch.

### Session 24 (BG-AUDIT-001 remediation) - paid in full

- **API balance deaths are silent and can land POST-COMMIT, and this session
  they were the norm, not the exception.** Both workers that died showed
  `APIError: Insufficient Balance` (402, non-retryable) only in the
  `events.jsonl` tail - no error reaches the slot status. FIX-009's worker
  committed the COMPLETE fix and ran all its Done-when gates (fmt/clippy/
  tests/check/kernel-gates green, verified from the transcript) before dying
  at the RESULT.json step; FIX-008's worker died before committing anything.
  **Read the slot branch for commits BEFORE redispatching** - a committed fix
  is proof the work is done and only the RESULT.json/verify/land remain.
  Reconstruct a missing RESULT.json as an orchestrator record with
  `amended_by: orchestrator` and the verbatim evidence from the transcript;
  never from memory.
- **`verify --base` is each packet's FORK POINT, read off the slot
  (`git -C loop/slots/N/wt merge-base HEAD integration/kernel-bg`).** Two
  mistakes: FIX-002's verify ran in a slot pointing at the WRONG branch
  (V1 SCOPE_VIOLATION on files a different packet had touched) and a FIX-006
  diff was measured against a non-ancestor commit, showing foreign changes
  that were my wrong base. Same-base verifies must be sequential; one
  concurrent run BLOCKED via leaked-baseline interference and relaunched
  clean alone.
- **inari 2.0.0 has no `Interval::asin`** (it is compiled only under the
  `gmp` feature, which is off and not viable on this toolchain) **and no
  `Interval * f64` / `Interval / f64`** (only `Interval * Interval`).
  FIX-005's wedge-slope lower bound landed the series hybrid
  (`s/2 + s^3/16 + 7 s^5/256`, all coefficients positive, scalars wrapped as
  degenerate intervals, `.inf()`). The s^5 coefficient is 7/256 - a packet
  that said 5/512 was machine-checked wrong.
- **A fix that ADDS an `unscaled_legacy()` call must have
  `scripts/unscaled_legacy_ceiling.txt` in the packet's `write_allow`.**
  FIX-010's sphere guard raised the GATE-4 ceiling 110->111; the worker did it
  correctly and documented it, but V1 would have rejected the ceiling file as
  out of scope until the packet was amended. The ceiling at its true measured
  count is a valid ratchet, not a licence.
- **verify.py block-buffers redirected stdout** - the `verify-*.out` log is
  EMPTY until near the end. An empty log is not a hang; poll `verify.pid`
  and rustc processes.
- **Disk: `%TEMP%/proc-macro-srv*` regrows (8.5 GB cleaned once, ~0.17 GB
  each), and the repo-root `target/` was 8 GB of idle regenerable junk** -
  deletable, the workers/verifies use their own targets and baselines. Check
  `Get-PSDrive C` before every verify round; the watchdog holds its disk
  guard during a live verify by design.

- **The watchdog's default 1200s wedged-killer kills healthy workers during a
  model-latency storm.** Session 13's endpoint had 23-60+ minute silence gaps
  from boot on three workers; the shipped default killed two of them mid-think
  (both later passed verify after redispatch — the kills cost ~25 min each).
  Start it with `LOOK_WATCHDOG_STAGNANT=3600`, and note **PowerShell 5.1's
  `Start-Process` has no `-Environment` parameter** — launch through
  `cmd /c "set LOOK_WATCHDOG_STAGNANT=3600&& python loop/watchdog.py"` or the
  variable silently never reaches the child.
- **The watchdog reaps on its own events-growth clock, not yours.** PARCYL's
  first worker looked 67 minutes stale from outside, but the run_packet boot
  touches the events file again after the worker's first write, so the
  watchdog's 3600s clock started ~13 minutes later than the file mtime
  suggested — and the reap fired at exactly `stagnant=3610s` (log,
  14:24:10). A reap that looks hours late usually is not: read the log's
  `stagnant Ns` field before concluding the timer failed. Related: a wedged
  worker keeps its `cmd.exe` shim alive, so a pid check alone says RUNNING —
  the underlying `opencode run` node process is the one that matters.
- **Whether a worker commits `RESULT.json` varies, and it changes the landing
  dance.** Four of session 13's six left it uncommitted in the worktree (copy
  it to the repo root yourself before `land_packet.py`); two committed it, so
  it arrived **tracked** on the merge and `land_packet.py` refused on the
  dirty tree until `git checkout -- RESULT.json`. Read the merge stat: a
  `create mode 100644 RESULT.json` line means the committed flavor.
- **Derive every pair's algebra to the end before fixing shared enum arms.**
  Session 13 added `Curves(Vec<ExactCurve>)` for coaxial torus families it
  believed could meet in four circles, then proved — one line of algebra
  later — that outer and inner contacts are mutually exclusive and share the
  squared equation, capping every family at two. The arm was added and
  dropped in two commits. Arms are cheap before dispatch and impossible
  after; so is the algebra that justifies them.
- **A verify launched through anything with a timeout is a verify you will
  kill, and killing one leaks its baseline worktree.** Session 12 ran three
  verifies inside a backgrounded shell wrapper whose harness cap is 600 s. A
  verify takes longer than that. The wrapper was killed mid-build, all three
  child verifies died with it, and three baseline worktrees (~1.3 GB) and three
  stale `loop/slots/N/verify.pid` files were left behind — the exact leak
  ORCHESTRATOR.md already warns about, walked into anyway. **Launch a verify
  detached** (`Start-Process` on Windows) so no caller's timeout can reach it,
  and poll `loop/slots/N/VERDICT.json` for the result instead of waiting on the
  process. Recovery, if it happens again: `git worktree remove --force` each
  `%TEMP%/look-verify-baseline-*/wt`, `rm -rf` the parents, `git worktree
  prune`, `rm -f loop/slots/*/verify.pid`. Nothing is lost — the packet branches
  are untouched and the verify just re-runs.
- **Never run two verifies at the same base concurrently.** They each build a
  baseline keyed by (base commit, test set) and will race on it, and "a baseline
  cached from a corrupted build" is already one of the three harness lies that
  cost session 11. Session 12 dispatched three packets from two distinct bases
  and ran the verifies **sequentially** for this reason. Related and harmless
  but alarming the first time: concurrent verifies each report the *others'*
  live baselines as "leaked worktrees", because the warning cannot distinguish a
  concurrent baseline from an abandoned one. Under sequential runs it goes away.
- **Read the carrier source; do not enclose the handoff's description of it.**
  Session 12's handoff said `Processor`'s `orientation() == false` "flips the
  normal cone axis". It does not: `Processor::subs` evaluates
  `entity.subs(v, u)` and `der_mn` swaps the orders *and* the arguments, so the
  parameters are transposed. A packet written to the description would have
  produced an enclosure that does not contain its own surface — an
  under-estimation, which BG-ENC-001 calls a silent wrong answer, and one that
  the *sampling* test would have caught only if the test box were asymmetric.
  The same ten minutes in `processor.rs` also turned up the projective divide in
  `transform_point`, and the same habit applied to `offset/mod.rs` is what found
  the `Offset` type error. This is the general rule ("re-derive every claim by
  running a command") applied to prose about types, where it is easiest to skip
  because the prose is confident and specific.
- **A `grep 'pub fn '` misses `pub const fn` and will convince you a type has no
  getters.** Every accessor `BG-ENC-004` needed — `Processor::entity`,
  `transform`, `orientation`, `ExtrudedCurve::entity_curve`,
  `extruding_vector`, `RevolutedCurve::origin`, `axis`, `Offset::entity`,
  `offset` — is `pub const fn`. Grep `pub \(const \)\?fn`.
- **Pseudocode in a packet must satisfy the lints the packet mandates.** Three
  ENC-004 packets specified a guard as `if !(cn > rho)`, and the same packets
  mandate `clippy -D warnings`, under which `neg_cmp_op_on_partial_ord` rejects
  exactly that. Two workers rewrote it to `cn <= rho` and reported it in
  `notes` — correctly, but the two forms **differ on NaN** (`!(x > y)` is true,
  `x <= y` is false), so the rewrite is only safe because an explicit
  `!cn.is_finite()` guard sits beside it. That was checked in the landed code
  rather than taken from the notes, and it held in all three. The lesson is
  cheap to apply: any comparison, cast or arithmetic a packet spells out will be
  typed in verbatim by the worker and then linted, so spell it out in the form
  that passes.
- **`schedule.py` honours `BLOCKED` now, and did not before.** It skipped only
  `RUNNING` and `DONE`, so a packet whose dependencies were all satisfied but
  which had been found undispatchable listed as eligible **forever**.
  `BG-INV-107` was reclassified BLOCKED in session 10 and was reported
  dispatchable in every frontier from then until session 12 — that is where the
  "17 eligible" and "16 eligible" counts came from, both inflated. Filing
  `BG-ENC-004-OFFSET` BLOCKED made it two. A status the scheduler does not know
  is a status that does not exist; putting the reason only in `note` does not
  stop the next session from dispatching the row.

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
- **A scratch crate outside the repo needs the repo's `.cargo/config.toml`
  rustflags** (`[target.'cfg(target_arch = "x86_64")']
  rustflags = ["-Ctarget-feature=+avx,+fma"]`) or inari 2.0.0 fails to
  compile with `cannot find function sub1_ru` — its directed-rounding SIMD
  backend `compile_error!`s without AVX+FMA, and there is no per-crate
  mechanism. Cost twenty minutes of misdiagnosis (the repo builds it
  "somehow") before the config file was remembered.
- **PowerShell has no `grep`, and anchor counts must be verified under the
  same shell the worker uses** — write a small `.sh` and run it through
  `"C:\Program Files\Git\bin\bash.exe"`; Select-String counts can silently
  disagree with grep's on regex-escaping grounds. (PowerShell also mangles
  inline `grep -c '...'` quoting entirely.)
- **A multiline `python -c` through PowerShell mangles embedded quotes**
  — the heredoc trap in its `-c` form, hit twice more this session. Write
  patch scripts to files and run the files. ALWAYS.
- **inari division by a zero-containing divisor returns the ENTIRE
  interval, not EMPTY** (measured: `[-1,1]/[-1,1] = [-inf,inf]`) — safe to
  rely on where a straddling divisor legitimately means "unbounded", but
  an explicit `contains(0.0)` guard reads better and is what the ISC packet
  mandates.
- **`new_slot.py` surviving a tool timeout does NOT mean the warm build
  finished** — session 17's slot-1 fork succeeded while the killed warm
  build left the target at 0.72 GB. Harmless: dispatch anyway and let the
  worker's first cargo pay the warm cost. The thing to check after a
  timeout is `git worktree list` (no duplicate fork), not the target size.
- **A grep-measured ripple list rots in BOTH directions.** The MIGRATE row
  over-included (modeling/integrate/meshalgo-src: no edits needed — the
  `mapped` signatures are unchanged) and under-included (a live `set_curve`
  in `fillet/mod.rs`, the set_curve in meshalgo's TEST file where the row
  named the src file, and the mutex-reach in `invariants/same_parameter.rs`
  which landed after the row was filed). Re-derive every ripple against the
  live tree; a row filed months ago is a hypothesis, not a fact.
- **A dangling `needs` entry is invisible forever.** BG-NUM-002/003 listed
  `BG-NUM-001` but the row's id is `BG-NUM-001-FILLET` — both rows were
  permanently unschedulable and nothing reported it. When a row "should" be
  eligible but `schedule.py` disagrees, print its `needs` against the actual
  id set before assuming a scheduler bug. (A five-line python check now
  exists in spirit — re-run one whenever the frontier looks too small.)

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
- **A carrier packet that does not spell out H-3 will be rejected for H-3.**
  GATE-2 is a text gate on the diff: any *added* line with a bare `1e-N`
  literal fails unless that line ends `// H-3`. It cannot tell an angle from a
  length and it does not exempt tests. `BG-CE-006-ENUM-r3` lost a verify to one
  such line, `BG-ENC-002-LINE` to one, and `BG-ENC-002-CIRCLE` to six — three
  packets, three round trips, one cause. A packet that says "named consts; a
  `// H-3` same-line opt-out if a bare float is ever unavoidable" in its test
  section is **not** enough: it reads as a style note. The four remaining
  carrier packets now carry a dedicated section with the house form copied out
  and the instruction to run `scripts/kernel-gates.sh` before writing
  RESULT.json. Copy that section into every new kernel packet.
- **The watchdog cannot tell a landed slot from a dead worker.** After
  `land_packet.py` moves RESULT.json out of the worktree, a finished slot has
  no pid, no RESULT.json and no event growth — Rule B exactly. It redispatched
  `BG-ENC-002-LINE` minutes after that packet merged, and the new worker took a
  lock on `events.jsonl` that made the next real dispatch die with
  `PermissionError: [WinError 32]`. Fixed by asking PACKETS.jsonl: a slot whose
  packet row reads DONE is left alone. **Reset a slot with `new_slot.py`
  promptly after landing** rather than leaving it looking abandoned.
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

- **A sampling property test must clamp its grid into the box it quantifies
  over.** `plane_enclose_is_sound` sampled `u0 + (u1-u0)*20/20` at the last
  grid index — a multiply-then-divide round trip that landed one ulp ABOVE
  `u1` (seed `e2369bfc`: 1.6356989675203588 → 1.635698967520359) — and the
  point evaluated there escaped Plane's correctly-rounded affine box by one
  ulp. The enclosure was sound the whole time; the same-tree
  interval-induction argument never failed, and the "escape" was the test
  asserting a false property (soundness outside the box). Two sessions
  misread it as a BG-ENC-001 under-estimation. The shared P-6 harness
  (`harness.rs`) carried the identical latent defect, masked only by every
  other carrier's enclosure slack. Both samplers clamp now and the seed is
  committed, so the case replays in every worktree. **Every packet whose
  tests sample a computed grid against a tight enclosure must clamp (or pin
  exact endpoints) and say why in the test.**

- **`run_packet.py --reset` clears the working tree; it does NOT re-fork the
  branch.** After a SPEC_GAP or a dead run, resetting the slot with
  `--reset` leaves the branch sitting on the old commits, so the next
  worker builds on top of them (tracked QUESTION.md/RESULT.json ride into
  the diff → V1 rejection) and measures against a stale base. The correct
  redispatch sequence is `new_slot.py --slot N --branch packet/<ID>` (which
  re-forks onto integration HEAD) **then** `run_packet.py` (no `--reset`
  needed on a clean fresh fork). Session 14 paid one worker kill for
  learning this — caught it only because `slot_status` showed the old
  commit instead of `(=base, no work)` after the redispatch.

- **A handoff's "loose ends" list rots as fast as any other prose.** Session
  14's handoff said BG-TOL-001-SMALL was "still unadjudicated, not
  mergeable unverified" — the registry and ledger said it was DONE two
  sessions earlier (`8f4f04d`, ACCEPTED, merged as `901f0ac`, fault GATE).
  The leftover branch ref `72e2b89` on the pre-landing base is what fooled
  the handoff. **Read `PACKETS.jsonl` and `LEDGER.jsonl` for status, never
  the handoff's own summary of them** — and never start a rebase/verify of
  an "unadjudicated" packet without that check first.

- **A decisive boundary classification needs dyadic data on the PRIMARY
  parameters, and multi-step interval polynomials never degenerate.**
  inari rounds every intermediate outward, so a polynomial expression of an
  exact-zero quantity evaluates to `[−ε, 0]` or `[0, ε]`, never `[0, 0]` —
  `decisively_zero` can only fire on short exactly-representable chains.
  The PCONE parabola rule died on this (spec amendment entry 4). The
  escape: classify on the primary parameters with a scale-free invariant,
  and choose witnesses with **integer raw normals and a dyadic slope** —
  `tan α = 3/4` is dyadic where `sin α = 3/5` is not, and no nontrivial
  Pythagorean triple has a power-of-two hypotenuse, so no unit vector with
  dyadic components exists at all. The same arithmetic limits apply to any
  future "exactly on the boundary" witness.

- **Verify a packet's own arithmetic with a command before dispatch.** The
  amended PCONE packet stated a witness plane `q = (−3, 0, 5)` whose stated
  cross product `(4, 0, 3)` belongs to `q = (−3, 0, 9)`; the worker caught
  it (the fourth worker correction of the session, and the second one in a
  packet the orchestrator wrote). Ten seconds of Python on the cross
  product at packet-writing time would have caught it. The
  "re-derive every claim" rule applies to claims YOU wrote, not just ones
  you inherited.

- **`Start-Process` with `-RedirectStandardOutput` holds the calling shell
  until the tool's timeout, and the launched process survives the kill.**
  Every verify launch this session "timed out" at the tool cap while the
  verify ran to completion and wrote its verdict. The correct response is
  to poll the artifact (`VERDICT.json`, `verify.out`), never to re-launch —
  a second verify at the same base races the first on its baseline cache.

- **Composing enclosures: never forward an unbounded parameter box into an
  inner carrier's `enclose`.** The landed surface carriers' behavior on
  non-finite input boxes is not uniform — `bspline.rs`'s `hull_of` returns
  the EMPTY box for non-finite `tt` (its "non-finite → empty" rule reads
  NaN and ENTIRE the same way), so a composition that forwards
  `Interval::ENTIRE` inward can under-estimate the whole thing. Decide the
  out-of-range answer yourself, at the composition boundary (PCURVE's
  decision 4 encodes this; the landed `pcurve.rs` returns the unbounded
  box directly). If a future packet hits a similar asymmetry, the
  signature is a composition whose empty/unbounded cases disagree with the
  inner carrier's own.

- **PS 5.1 `Set-Content -Encoding UTF8` writes a BOM, and the watchdog's
  `packet_is_done` dies on it silently.** Session 15's incident: a registry
  edit made through PowerShell left `EF BB BF` at the head of
  `PACKETS.jsonl`; `read_text(encoding="utf-8")` then prefixes line 1 with
  U+FEFF, `json.loads` raises `ValueError`, and the `except` in
  `packet_is_done` returns `False` — so **every packet reads not-DONE and
  Rule B will redispatch a landed one.** At 01:53 it redispatched the
  already-merged `BG-ENC-004-PCURVE` onto the freshly forked
  `packet/BG-TOL-004` branch, where a worker dutifully re-verified the
  landed code and committed a RESULT-only commit. Zero lost work (the code
  was already in the tree), ~7 h of slot time and one worker run wasted.
  The artifact is archived at
  `loop/slots/0/misdispatched-pcurve-20260821-RESULT.json`. **Edit
  `PACKETS.jsonl` only through python** (the loop scripts or a `python -c`
  rewrite), and after any registry edit, run the watchdog's own read path
  and confirm a known-DONE id still reads `True`:
  `python -c "import json; rows={json.loads(l)['id']: json.loads(l)['status'] for l in open('loop/PACKETS.jsonl', encoding='utf-8') if l.strip()}; print(rows['BG-ENC-004-PCURVE'])"`
- **`new_slot.py` alone leaves the slot looking like a dead worker.** It
  resets the *worktree* but not `worker.pid` / `worker.packet` /
  `worker.branch` in the slot root, so a forked-but-not-yet-dispatched slot
  presents the previous packet's dead pid and stagnant events to the
  watchdog — which Rule-B's the *previous* packet onto the *new* branch
  (`run_packet.py` does not re-fork; that half is the older trap). The
  PCURVE misdispatch needed both halves of this: stale slot files AND the
  BOM-blinded DONE check — with the registry intact, `packet_is_done` would
  have held. **Fork and dispatch in one motion; if a forked slot must be
  left idle, delete its `worker.pid` / `worker.packet` / `worker.branch`
  first.**
- **Machine sleep reads as stagnation, and the watchdog reaps on wake.**
  The watchdog's clock is wall time; sleep freezes `events.jsonl` but not
  the clock, so a worker that slept 6.9 h (02:10–08:45, the heartbeat gap
  in `watchdog.log` is the signature) presents exactly like a wedged one
  and is killed and redispatched on the first poll after wake — CE-001
  attempt 1 died this way having done nothing wrong. The workers
  themselves survive sleep and resume fine. **If the machine will sleep
  (lid close, overnight), stop the watchdog first** (pid in
  `loop/watchdog.lock`) and restart it when the session resumes; restart
  budgets absorb one such reap, but each one wastes a worker's progress.

- **V8 reads a flaky proptest as a regression too — proven this session,
  and the recovery has a path trap.** BG-INV-102's first verify REJECTED
  on V8 via `truck-modeling/src/geom_impls.rs::test_circle_arc_tangent0`
  (a `#[property_test]`). The packet changed one unreferenced leaf module —
  impossible — and replaying the pinned seed at BASE proved it: the seed
  (p0/p1 both on the z-axis, `t = 0.9999789572401523`, so p2 ≈ p1 and
  `circum_center` is ill-conditioned) fails 4/4 at base too. The property
  is missing a precondition on `t` near 0/1 — a latent truck-modeling
  defect outside every write set. Recovery: delete the slot worktree's
  `proptest-regressions/` artifact and re-verify (fresh seeds pass 8/8).
  **THE PATH TRAP: the regression artifact is a DIRECTORY
  (`proptest-regressions/geom_impls.txt`), not a flat file — copying it to
  the flat path makes the replay run silently draw FRESH seeds and
  "pass", invalidating the experiment.** Reproduce at the right path
  before believing any pinned-seed result. Same disease family as the V5
  bspcurve flake; V8 inherits it. The property fix is future work in
  truck-modeling.

- **The RESULT.json landing dance has three flavors, and over-cleaning
  costs recoveries.** (a) Uncommitted: copy worktree → repo root BEFORE
  `land_packet.py` (it reads the root after the worktree check). (b)
  Committed: your untracked root copy BLOCKS the merge — remove it, land
  (the merge brings RESULT.json in tracked), then delete the root copy
  again. (c) If you delete the worktree copy too early: committed ones
  restore with `git -C <slot wt> checkout -- RESULT.json`; UNCOMMITTED
  ones are recoverable VERBATIM from the worker's session transcript —
  `events.jsonl` records the write tool call with the full content
  (`part.state.input.content`). **Never reconstruct a RESULT.json from
  memory or from a truncated read — the worker's reasoning must stay
  verbatim.** A land attempt that fails at the merge step has NOT filed
  anything; re-run it after fixing the collision.

- **API sketches validated OUTSIDE a crate do not type-check INSIDE it.**
  Session 16's INV packets were designed from scratch-crate perspective and
  all three wave-1 workers caught the same class: `Shell` takes THREE type
  parameters (`Shell<P, C, S>`, no default), `Shell` lives at the crate
  root (`use crate::Shell`, not `crate::shell::Shell` — shell.rs only
  privately glob-imports it), `use truck_topology::*` is E0432 from inside
  the crate (use `use crate::*`), and `#[derive(Default)]`-only types trip
  `missing_debug_implementations` in truck-topology but not in a scratch
  crate. The workers' `disagreements` field caught every one — packets that
  name API signatures must invite disagreement explicitly, and the
  orchestrator should write in-crate sketches as if from inside the module.

- **A checker that calls another crate's API needs the manifest edge — and
  nobody had landed it.** BG-INV-104 attempt 1 was a clean SPEC_GAP: the
  worker implemented the whole checker, hit E0432 on `use
  truck_evidence::…`, reverted to baseline and asked. The fix is
  `truck-topology → truck-evidence` (+ `inari`; `truck-geometry` as
  dev-dep) — acyclic, since evidence does not depend on topology — landed
  by the amended packet as decision 0 (manifest + lock in write_allow, the
  BG-CE-003 serde_json precedent). Spec amended: the edge is the intended
  layering. Any future invariants-tree checker speaking interval
  certificates uses the same edge.

- **`run_packet.py` dying with `PermissionError: [WinError 32]` on
  `events.jsonl` right after printing "Running packet" is BENIGN.** The
  worker launched and opened the events log before run_packet's own
  post-launch cleanup could reset it; the crash is orchestrator-side
  bookkeeping only. Check `slot_status.py` — a fresh pid and growing
  events means the dispatch took; do NOT re-run it.

- **`grep -rc PATTERN dir | wc -l` counts FILES, not matches** (GNU grep
  prints one line per file, zero-count files included) — two packet
  anchors were written with it this session and one was caught by
  `run_packet.py`'s own pre-dispatch anchor check (the A3 refusal, fixed
  before any worker was paid), the other by manual verification. The
  match-counting form is `grep -r PATTERN dir | wc -l`.

- **The scratch-crate pre-validation found three errors a compile check
  alone never would, and was worth every minute.** Session 16's CE-002
  scratch RUN (not just compiled) the whole design: it found the carriers'
  terminal-strip convergence defect (a BG-ENC-002 violation, spec
  amended), measured the bisection cost model that forced the two-route
  design (130 µs/cell → minutes per edge at tau=1e-6), and caught f64
  having neither Eq nor Hash. The discipline for design packets: compile
  it, then RUN the flagship witnesses and the cost, then write the packet
  with the measured numbers in it. The one thing the scratch could NOT
  cover was in-crate compilation (the trap above) — the workers covered
  that half, exactly as the disagreements field was designed for.

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

- **Hand-derived numbers in a packet must be machine-checked, not hand-checked.
  Session 18 shipped five wrong witnesses in BG-NUM-002 and a worker caught
  every one.** The Bernstein coefficients of `(2t-1)^2` are `[1, -1, 1]`, not
  `[1, 0, 1]` - I had confused control points with node values (b1 comes from
  `p(1/2) = 0.25*b0 + 0.5*b1 + 0.25*b2`, NOT from evaluating at a node). Same
  fallacy family: the clustered-root middle coefficient was `-s^2 - 0.25`, not
  `-s^2`. Beyond the algebra: a witness root exactly AT a dyadic bisection
  midpoint produces endpoint-contact children that refuse under my own rules,
  so `t - 0.5` on `[0, 1]` can never return Ok - pick domains where the root is
  not a dyadic rational; same for a dyadic cluster half-width `2^-12`; and a
  budget of `log2(1/s)` ignores the ~19 refinement levels tau=1e-6 needs, so
  the "enough budget" test would have refused. Five for five caught by the
  worker's disagreements field - the division of labor held - but each one cost
  worker turns and risked a round trip against a weaker model. A ten-line
  script that PRINTS the sequences (the num3-scratch pattern) catches all five
  before dispatch. Run it on every numeric packet. Always.
- **`run_packet.py --reset` is archive-and-DISPATCH, not reset-only** - there
  is no flag combination that resets without spawning a worker, and dispatching
  a known-flawed packet wastes a launch (cost: two killed dispatches on slot 1
  tonight; harmless money, real noise). To reset only: kill the pid tree
  (`taskkill /PID <pid> /T /F`), then manually `git -C loop/slots/N/wt reset
  --hard HEAD && git -C loop/slots/N/wt clean -fd`, which is all the reset arm
  does after archiving. Fixing the script (a `--reset-only`) is five minutes of
  harness work someone should do.
- **A scaffold commit that touches lib.rs must run `cargo fmt --check -p <crate>`
  BEFORE landing.** e69103e inserted `pub mod num;` after `pub mod nurbs;`,
  violating reorder_modules, and sat in the tree for a full session. Two
  workers then found their done-when gate red at base on a file outside both
  write sets - V3 is file-scoped so no verify failed, but both burned turns and
  one reported SPEC_GAP for what was purely my defect (`726e9b3` fixes it;
  labeled orchestrator amendment per house rule).
- **Commit ledger rows BEFORE calling land_packet.py** - it refuses on a dirty
  tracked tree, and the failure mode compounds: the followup command sequence
  that deletes the stray RESULT.json unconditionally will then make the retry
  die on FileNotFoundError (both halves hit tonight). Order: merge, copy
  RESULT.json to root, commit any pending loop/ edits, land_packet, delete the
  root copy.
- **A migration's information flow includes INSTANCE IDENTITY, not just
  values.** The CE-003 mutation semantics shared one mutable Vertex between two
  loops_stores, and what closed the boolean shell was that both stores kept
  referencing the SAME instance as later replacements re-pointed it - final
  point AND identity together. Immutable `Arc<G>` construction preserves values
  fine and identity not at all; loops byte-identical still produced an open
  shell. Any packet migrating away from interior mutability must ask "who else
  holds this handle, and do they need MY next write to be visible?" before it
  declares the ripple list. This survives in the ledger fault note and the
  r2 row; it stays here because the next migration packet WILL be written by
  someone who has not read QUESTION.md.
- **Stale keys in a shared replacement map are POISONOUS under allocator
  address reuse, and the mutation semantics hid this for free.** Session 19:
  the MIGRATE-r2 Inner-arm unify sweep creates fresh half-edges, registers
  them in the shared emap, then DROPS them - their raw Arc addresses return
  to the allocator, the next allocation (the other store's halves) can land
  on the same address, and `emap.entry(id).or_insert_with` then hands that
  store a replacement edge built from the WRONG store's vertices. Baseline
  never died of this because its cut halves directly referenced the shared
  mutable vertex. The worker's fix: after a sweep that consumes-and-drops
  registered edges, EVICT their ids (`emap.remove(&half0_id)`). Any design
  that registers short-lived edges in a map keyed by pointer identity owes an
  eviction argument; see BG-CE-003-MIGRATE-r2 RESULT.json deviations.
- **Narrowing `crates` in an r2/follow-up packet breaks V3, correctly.** The
  r2 verify first came back LINT_UNLINTED because I set
  `crates: [truck-shapeops]` while the diff ddcd706..HEAD still spans
  attempt-1's truck-topology/truck-meshalgo changes - V3 refuses to pass a
  diff containing crates clippy never linted. The gate is right: `crates`
  must name the union of every crate the base..HEAD diff touches, which for
  a branch-carrying follow-up is the ORIGINAL packet's crate list, not the
  r2 edit's own footprint.
- **gen_packet/run_packet anchor checks always run against the MAIN worktree**
  (`run_anchor` uses cwd=REPO_ROOT), so anchors describing a dispatch slot's
  branch tip fail at dispatch with confusing baseline-state counts. For any
  packet dispatched onto a branch carrying prior work, pin each anchor to
  that exact tip: `git show <sha>:<path> | grep -c '<pattern>'`. Measured
  counts stay H-8 obligations either way.
- **inari 2.0 endpoint accessors are `.inf()`/`.sup()`**, not `.lower()/
  .upper()` (the latter cost one scratch compile cycle). Box3/interval code
  outside vendor/ should copy the accessor spellings from
  truck-evidence/src/enclosure.rs rather than guessing.



- **A scratch validates the formula you WROTE, not the formula you MEANT -
  and a witness can be blind to exactly the bug.** Session 19: the curvature
  bracket used `4*F.sup^2` where sup|F^2| needs `4*max(|F.inf|,|F.sup|)^2`;
  for F=[-10,-1] that reads 1 against a true 100, INFLATING lam_min_lo and
  DEFLATING the curvature bound (safety reversal). The sphere scratch passed
  anyway because F ~ 0 analytically on a sphere - the one carrier chosen hid
  the one bug. Caught in review, not by running. Corollary: when a scratch
  witnesses only near-degenerate coefficients for some term, that term is
  untested; add a witness whose coefficient is large and strictly negative.
- **Packet reference values must come from the SAME code path the packet
  mandates.** BG-FID-001's first dispatch quoted the naive-normalization
  scratch numbers while Decision 2 mandated the iota route; the worker's
  disagreements field caught the mismatch (values off by up to 4x). Machine-
  checking is not enough - check WHICH variant produced the numbers.
- **AABB box-distance bounds are translation-invariant but NOT rotation-
  invariant in tightness** (rotated enclosures overlap more; still sound,
  looser). Any invariance test on box-based certificates must be translation
  only, or assert soundness across rotations rather than equality.
- **Refusal provenance is part of certificate semantics - use typed Result,
  never Option**, so callers distinguish ImmersionUnresolved from
  MetricLowerBoundUnresolved from bad input. And annotate evidence with
  `@feeds [Thm, hyp]` / `@establishes` / `@does-not-establish`, NOT
  "Thm instance": evidence that feeds a hypothesis does not instantiate the
  theorem, and a false tag reads as authority later.
- **Box-distance between incident-strata cells sharing a point is trivially
  zero**: star-separation-style terms need a certified excluded ball around
  the shared feature or they are vacuous. Deferred from BG-FID-001 (vertex
  row stays prose) rather than shipping a function that always answers 0.
- **PowerShell `$pid` is a READ-ONLY automatic variable** - assigning the
  watchdog's lock pid to it dies with VariableNotWritable; use any other
  name (`$wdpid`). Cost one failed taskkill line, nothing more, but it is
  the same class as every other PS 5.1 surprise above.


- **Membership needs CONTAINMENT, not intersection.** `box_distance(A, B)
  <= eps` (the infimum/gap distance) proves A *intersects* the eps-ball; a
  certified root lying somewhere in A may be beyond eps. Inclusion needs the
  SUP-distance (farthest corner) <= eps. Session 20 rejected a
  verify-ACCEPTED FID-008 on exactly this: a coverage-violation crossing at
  eps + 2e-5 inside a width-1e-4 decision box certified `ExactlyOne` with a
  true count of zero. The failure mode is silent because every shipped
  witness has margin.
- **`KrawczykProof::Unique` means "at least one root, exactly one in SOME
  sub-box" - never "exactly one in the query box".** The operator returns on
  the first internal proof and does not enumerate the rest of the box.
  Counting one and discarding the box loses a second root (a false
  `ExactlyOne`); the sound structure enumerates to the width floor and
  counts early only where a certified lower bound of >= 2 distinct roots is
  decisive for `NotOne` regardless of the unexamined remainder.
- **An ABSOLUTE width floor starves at large parameters.** `8 * EPS` is 16
  ulps at t ~ 0.7 but 2 ulps at t ~ 7 - too narrow for the interval K
  operator to contract strictly inside. Any resolution floor must scale with
  the box magnitude (`width_floor(&tt)` in one_sheet.rs). One double-cover
  crossing is structurally always at |t| >= 2pi, so no witness choice dodges
  the large-t regime.
- **A machine-check must model the engine's ACTUAL stopping behaviour.** The
  FID-008-r3 check simulated the bisection descent and stopped AT the floor
  box; the engine calls krawczyk THROUGH the floor box and split at exactly
  the root. The check passed while proving a different algorithm than the
  one that runs. When a packet mandates a machine-check of an engine's
  arithmetic, the check must replicate where the engine actually invokes the
  operator, not where the prose imagines it.
- **Witness parameters must be checked against FLOAT BISECTION EDGES, not
  just dyadic rationals.** t_x = 0.7 put the double-cover root at
  t_x + 2pi = 6.983185307179586 exactly ON a box edge produced by
  `0.5*lo + 0.5*hi` rounding; strict-interior Unique is unreachable from any
  box born with that edge. Simulate the descent and require >= 2-ulp margins
  to both edges of the terminal box (the landed engine retries on a widened
  box, but do not pick a witness that needs it).
- **`f64::next_after` does not exist on this toolchain (rustc 1.97); the
  stable `next_up`/`next_down` do.** Spell the stable ones in packets.
- **An orchestrator amendment can land a BLOCKED whose fix the worker already
  proved by experiment.** The r4 BLOCKED's own controlled experiment (move
  the packet's widening retry to every krawczyk Err -> NotOne{count:2} in
  7.82s with zero spend) was the entire design basis for the ~40-line
  amendment; verify ACCEPTED with `amended_by` surfaced in VERDICT.json. The
  worker's instrumentation is admissible evidence - when a BLOCKED arrives
  with a controlled experiment in hand, consider amending before paying for
  another dispatch.

- **A packet's mandated FORMULA must carry the same semantics as its
  machine-checked numbers.** Session 21, BG-FID-005: the scratch script
  measured TRUE radial error by dense sampling (0.3365/0.4292/0.0304) while
  Decision 3 mandated `eps_now = max sup_distance(emitted hull box, exact
  cell box)` - which is the BOX DIAGONAL, ~sqrt(2)x cell width, permanently
  larger than the non-adjacent gap the (iv-b)(c) gate compares it against:
  the literal loop refines forever and the packet's own test 1 is
  unsatisfiable. The numbers were right; the formula was wrong; the
  machine-check passed because it never ran the FORMULA. Generalizes the
  FID-001 trap ("check WHICH variant produced the numbers"): a check must
  evaluate the exact expression the packet mandates, in the exact box
  semantics the gates will use, not a mathematically-equal-on-paper
  surrogate.

- **A witness whose deviation EQUALS the tolerance is uncertifiable by
  design.** Session 21, BG-FID-003-r2 test 3: the FID-008 double-cover
  witness `(R + eps*cos(t/2))e(t)` has max deviation EXACTLY eps; (i)'s
  pairing demands sup_distance <= eps with interval padding, so the seam
  cell can never certify and the check returns ClosenessUnresolved forever -
  not a violation, just undecidable. Same family as the dyadic-root and
  bisection-edge traps: **witnesses must sit strictly inside every
  certification margin, and the machine-check must verify the MARGIN
  (strictly less than the gate), not merely recompute the value.** The
  worker's amplitude-halving fix preserves the property under test.

- **"The tangent box contains both branch directions" is NOT "contains the
  zero vector".** Session 21, BG-FID-005 test 4: two segments at a 60-degree
  TURN have a corner tangent box that never straddles the origin, so
  `curvature_radius_lower_span` happily certifies +inf (straight legs!) and
  the collapse route never fires. The refusal needs the box hull to CONTAIN
  THE ZERO VECTOR (antiparallel-ish branches) AND an off-dyadic corner (a
  dyadic corner splits exactly at the kink and every cell is straight).
  When a packet prose describes an enclosure property, spell the property
  the code actually tests - "contains 0", not a geometric paraphrase.

- **A mid-run packet review is cheapest acted on immediately.** Session 21:
  the review of the dispatched BG-FID-003 found four design defects; the
  worker was killed at 23 min with zero commits, the packet was amended to
  r2, and the r2 landed clean. The FID-008 alternative (let it run, verify
  ACCEPTS, orchestrator rejects on semantics, four-attempt chain) cost a
  full session. Verify gates are mechanical: certificate-semantics defects
  pass them. Killing a worker on a known-defective packet wastes minutes;
  landing one wastes the chain.

- **API balance deaths are silent, and can land POST-COMPLETION.** Session
  21: the --resume dispatch died at 11 events with worker.err EMPTY and the
  slot reading STALLED; the error exists only in
  `~/.local/share/opencode/log/opencode.log` ("AI_APICallError: Insufficient
  Balance"). The next death hit AFTER the worker committed its code and
  wrote RESULT.json, during the final `git add` - the slot looked dead but
  the work was complete. **Read the opencode log before diagnosing any
  silent worker death, and read the slot branch for commits before
  redispatching - a redispatch archives the worktree and a finished-but-
  unlanded attempt can be lost to a redundant restart.**

- **`Set-Content -Encoding ascii` mangles every non-ASCII character to '?',
  three per em dash.** Session 21 turned 89 em dashes into '???' in one
  packet file by re-encoding it through PowerShell (the known trap was the
  UTF8 BOM; the ascii side is its sibling). Never rewrite a file through
  PowerShell content cmdlets - use python (`io.open(..., 'w',
  encoding='utf-8', newline='')`) or the Write tool, and after ANY script
  passes over a packet file, re-check that its non-ASCII survived.

- **Machine sleep killed a healthy worker a SECOND time, and this time it
  cost four hours of work — the recovery recipe is proven, use it.**
  Session 22: the worker died mid-run at 23:47 when the machine slept; the
  watchdog woke at 02:51, read 12195s of stagnation, killed it and
  redispatched FRESH (the redispatch archives the uncommitted worktree as
  `loop/slots/N/abandoned-<timestamp>.patch` — that archive is the
  lifeline). Recovery that worked, first try: (1) stop the watchdog; (2)
  kill the fresh worker it started; (3) `git -C wt reset --hard && clean
  -fd`, then `git apply` the abandoned patch; (4) COMMIT the restored state
  on the packet branch — run_packet REFUSES a dirty tree without `--reset`,
  and `--reset` archives-and-discards, destroying the recovery;
  (5) `run_packet.py --resume --session-id <old id>` (the old session id is
  in the opencode provider log's stream lines, or worker.session if not yet
  overwritten). The resumed session woke with its full debugging context
  and finished in ~80 min. Also: the pre-sleep events.jsonl transcript is
  LOST to the redispatch's reset — copy it aside before any recovery if the
  transcript matters.

- **A killed or timed-out test binary keeps its cargo file lock, and that
  looks like a stalled worker.** Session 22: a zombie `truck_evidence-*`
  test process from before the machine sleep burned 1463s CPU on the
  pre-fix code and held the output binary; the resumed worker's cargo
  commands sat at "permission denied" for 27+ min while its own kill
  attempt missed. After ANY hard kill or timeout of a worker, `Get-Process
  *truck_evidence*` and taskkill the stragglers before blaming the worker.

- **The refine stall guard does not guard an unreachable target.** Session
  22, worker-right disagreement: with `target_eps ~ 0` (a zero-scale exact
  input, e.g. the double cover whose separation certificate is genuinely
  0), eps improves ~30% per level FOREVER, so the "<1% relative improvement
  twice" stall rule never fires and the loop refines into grids whose
  measurement cost is the hang. The packet's own termination note was
  wrong; the worker's fix (hand the loop the measured scale spend + 2
  subdivisions so exhaustion fires on reachable grids) is the pattern for
  any never-Ok test. If a future packet feeds zero-scale geometry to a rep
  loop, an explicit target-reachability guard is a design decision to make
  in the packet, not to discover in the test suite.

- **A NaN/`[inf,-inf]` enclosure is the signature of a DEGENERATE-OVERLAP
  MISS, not of bad arithmetic.** Session 22, worker bug found by the re-rep
  test: its `cellOverlaps` port dropped the degenerate-inclusion clause, so
  a degenerate query landing exactly ON a grid knot matched NO cell, the
  hull of an empty point list is the `[inf,-inf]` box, every downstream
  midpoint is NaN, and the loop hangs on NaN comparisons. When an enclosure
  comes back empty/infinite, print the OVERLAP SET first. (Same family as
  the curve's cellOverlaps rule — the per-axis port must carry the
  degenerate clause.)

- **The pre-dispatch machine-check caught three design traps and the worker
  caught two more implementation traps — both halves are load-bearing.**
  Session 22: the orchestrator-side interval scratch (scratch/fid005srf-
  check.py, outward-rounded arithmetic through the exact mandated formulas)
  caught the level-cap/linear-convergence explosion, the false convergence
  on garbage-small coarse certificates, and the ulp-sliver derivative
  explosion BEFORE dispatch; the worker's disagreements/tests caught the
  derivative-net transpose and the degenerate-overlap miss AFTER. Neither
  half alone would have landed this packet.

- **A `HashMap` keyed by a type that serializes as a map cannot round-trip
  through serde_json — and the failure is deferred and data-dependent.**
  Session 23, INV-107 pre-dispatch: a derived `HashMap<EntityId, _>`
  Serialize compiles fine and serializes an EMPTY map as `{}`; the first
  non-empty map fails at RUNTIME with "key must be a string" (serde_json
  requires string keys, `EntityId` is externally tagged — a map). A
  doctest or test that only round-trips an empty store would have passed
  and shipped the defect. The fix is to serialize as a sequence of pairs
  (`serializer.collect_seq(map.iter())`, deserialize a `Vec<(K, V)>` and
  rebuild), which is also duplicate-tolerant (last wins). Caught by the
  scratch-crate run against the real type (`scratch/inv107serde`), not by
  compilation — the scratch discipline again. Any new keyed-by-EntityId
  container owes the same check.

- **The scratch must model the packet's EXACT signatures, not a
  simplification of them.** Session 23: the INV-107 serde scratch validated
  the store design with a `raise` returning `()` — but the packet's
  `raise` returned `Outcome<()>` (= `Result<Certified<()>, Refusal>`), and
  the packet's `Ok(())` success line does not compile against that alias.
  The scratch's simplification hid exactly the bug the worker caught (and
  fixed correctly, with an empty PropMap — a mutation does not certify the
  invariant). Same family as "reference values must come from the SAME code
  path the packet mandates": every fn a packet spells verbatim goes into
  the scratch verbatim, return types included, or the scratch proves
  something adjacent to what ships.

- **The bash tool's own timeout cap is one of the "wrappers" that kill a
  verify.** Session 23: a foreground `verify.py` with a 15-minute tool cap
  was killed mid-baseline exactly as the session-12 trap predicts. The
  recovery is already recorded (relaunch detached, poll `VERDICT.json`,
  check its `packet`/`base`/`commit` fields before believing it) and worked
  first try — the addition is only that the tool timeout is a live instance
  of the wrapper, so verify NEVER runs in the foreground of a tool call
  with a cap; launch it via `Start-Process cmd /c "... > out 2>&1"` and
  poll.
