# OPERATOR ESCALATIONS

Judgment-required items appended each operator cycle. Newest at the bottom.

## 2026-09-09 00:45 UTC - FH-TIMING-REFRESH landing blocked on a failing scoped check

- What: slot 0 FINISHED, packet FH-TIMING-REFRESH, wt HEAD 6c4ba7c (worker commit
  481ca91 "bench: Falcon-Heavy kernel timing column" + 6c4ba7c "file RESULT").
  RESULT.json status LANDED; row status READY; NOT yet merged into
  integration/kernel-bg.
- Why: the overnight driver's scoped check failed at 20:43 ("check -p
  truck-certified failed") and it left the landing for morning. FH-TIMING-REFRESH
  is a doc-production packet with `crates: [look]`; the driver's
  packet_tests_and_crates() falls back to `truck-certified` when the row writes no
  `vendor/truck/**` path - so the failing check may be the wrong crate (gate-wrong)
  OR disk-exhaustion (only ~5 GB free at the time). Either way a landing decision
  needs judgment I do not have.
- Start here: `git -C C:\Users\stefa\look log --oneline packet/FH-TIMING-REFRESH`;
  decide whether `cargo check -p truck-certified` at 6c4ba7c is meaningful for a
  docs-only packet; if the packet's own done-when (`cargo check -p look` +
  doc-production) passes, merge --no-ff and file `loop/results/FH-TIMING-REFRESH.json`.
  IMPORTANT: do NOT re-fork slot 0 until this lands or is closed - the unlanded
  RESULT.json in the worktree is the only copy.

- RESOLVED SUPERSEDED 20:46 UTC (same cycle): the session-56 handoff landed it
  concurrently - merge 0daf8d6 + row LANDED c94d043 are now in
  integration/kernel-bg (verified: c94d043 is an ancestor of HEAD). No action
  needed; slot 0's leftover worktree RESULT is stale residue and is safe for
  dispatch_ready to recycle.

## 2026-09-09 02:30 UTC - four items

- **F1-AUTHORING-ARMS (slot 4, LANDED-WITH-FINDINGS) needs an orchestrator
  amendment to free its slot.** The worker's F1 finding names an obsolete pin:
  `mvac_row_still_refuses_typed_at_the_extrude_carrier` in
  truck123d/tests/ttc_lathe_spline.rs now FAILS because the extrude carrier
  records (this packet's own scope folded it in) and mvac proceeds to refuse
  typed at the later non-z Location placement. Driver logs it "LEFT FOR MORNING
  (judgment required)" every cycle. Start here: re-point that test at the
  Location refusal per the packet's finding F1, then the merge (3c2109b is the
  worker commit; its integration merge is already done, so this is the pin fix +
  slot 4 residue cleanup).
- **overnight.py stop-condition prose parse is still wrong for the "NOT
  triggered." class.** 594f07b special-cases "none"; a string beginning "NOT
  triggered." (ADM-L4) still trips it as stopped=True, so the driver never lands
  such rows AND holds dispatch in "landing/running phase" forever. I bypassed it
  for ADM-L4 by hand-landing. Fix suggestion: treat str stop_conditions as
  stopped only when a trigger is asserted positively (e.g. regex on
  "trigger(ed|s)?" NOT preceded by "NOT|none|no ").
- **dispatch_ready is dispatching 0 with 8 free slots and prints NO per-candidate
  reasons** (only "slots: ..." + "dispatched 0"). Every READY row seems to be
  silent-filtered (status/landed-note/assigned/dead) yet ADM-001/002/003 now have
  all deps landed and many legacy READY rows exist. Look at the candidate loop
  (loop/dispatch_ready.py ~line 5500) before trusting the machine idle by choice.
- **Duplicate supervisors observed** 2026-09-09 02:1xZ: python 35200
  (C:\Program Files\PyManager\python.exe loop/supervisor.py) AND 24272
  (pythoncore-3.14 loop/supervisor.py). Session-56 paid for the duplicate-driver
  class; confirm which is canonical and kill the stale one.

## 2026-09-09 03:36 UTC - three items carried / re-confirmed

- **Duplicate supervisors STILL alive** (re-confirmed 03:27Z): 35200 (PyManager
  loop/supervisor.py) + 24272 (pythoncore loop/supervisor.py), both since
  09-05 11:28. The overnight driver keeps logging "landing/running phase - no
  dispatch" in the else branch (rows_done_now is False because BG-KV2-207B-
  TRACER-REST is READY), so neither supervisor's dispatch arm is live, but a
  duplicate driver could double-merge the moment a FINISHED DONE slot appears.
  A human should confirm which supervisor is canonical and kill the stale one.
- **F1-AUTHORING-ARMS mvac-pin amendment still open** (slot 4 residue, F1
  finding: re-point mvac_row_still_refuses_typed_at_the_extrude_carrier in
  truck123d/tests/ttc_lathe_spline.rs at the non-z Location refusal; row is DONE,
  merge 27ad2ce landed). Driver logs "LEFT FOR MORNING" every cycle.
- **NEW - overnight driver merged ADM-L1/L2/L3 but never filed their
  RESULT.json.** loop/results had no L1/L2/L3 copy (only the slot-wt copies
  existed). I filed them this cycle (c3f89e4) before the heartbeat recycled
  slots 1-3 for the ADM assemblies. Root cause worth a human look:
  overnight.py try_land's RESULT-filing branch (line ~221) only fires when the
  merge carries RESULT.json into the REPO ROOT working tree; if the root file
  was already consumed/unlinked it silently skips filing. Start from
  loop/results/ADM-L1-EXTRACT.json and the LEDGER.

## 2026-09-09 04:30 UTC - ADM-001-ADAPTER scoped check NOT green; row still READY and re-dispatchable

- What: ADM-001-ADAPTER (slot 0, dispatched ~03:40Z by the heartbeat) finished
  its run; worker commit e076c1f (admission.rs +409/-46, admission_conformance.rs
  +796, parent 428bbff) sits on packet/ADM-001-ADAPTER. The overnight driver's
  scoped check FAILED at 04:17Z and logged "slot 0: ADM-001-ADAPTER scoped check
  NOT green (test truck-certified:admission_conformance failed); left for
  morning" (overnight.log, 00:17:04 local). NOT landed, and I did NOT land it -
  per the charter a failed done-when check blocks landing.
- Why it needs a human: is the admission_conformance failure a GENUINE defect in
  e076c1f or a driver artifact (load/RAM at check time, wrong base)? Disk was
  ~10-12 GB free at 04:17Z (not ENOSPC). The check command is
  `cargo test -p truck-certified --lib --tests` at a worktree at e076c1f (or at
  merged HEAD after `git merge --no-ff e076c1f`); the four named tests are
  polynomialized_F_zeros_equal_direct_difference_on_fixture,
  ruled_section_loft_pair_admits_and_certifies,
  non_admitted_carriers_still_refuse_typed, v5_pair_identity_battery_green.
- Machinery gap (second occurrence of the class): ADM-001's RESULT.json copy was
  LOST - the heartbeat re-forked slot 0 to ADM-002 at 04:15:55Z (dispatch_
  heartbeat.log 00:15:55 local) ~90s BEFORE the driver logged the failed check.
  No ADM-001 RESULT survives anywhere (checked loop/results, slot 0 wt root, repo
  root). The 03:36 escalation (driver never files RESULTs) is the same root
  cause family.
- Re-dispatch risk: the PACKETS row is still READY (no landed marker), and
  dispatch_ready --dry-run confirms it WILL dispatch ADM-001 again the moment
  ADM-002 frees the admission.rs write set - a ~40-min redo that will reproduce
  the same failing test if the defect is genuine. Recommend adjudicating the test
  failure before letting the heartbeat re-fork it, or pinning the row.
- Start here: `git show e076c1f --stat`; check out e076c1f in a throwaway
  worktree and run `cargo test -p truck-certified --test admission_conformance`
  (quiet machine, after the two live workers' cargo settles). If green -> land
  mechanically; if red -> decide fix (worker amendment) vs SPEC_GAP.

## 2026-09-09 04:56 UTC - ADM-003 landed by operator; ADM-001 adjudication still OPEN (re-fork risk live)

- **ADM-003-VOLUME is LANDED** (merge 9e86346, row DONE f45da69) - it was stranded by the SAME prose-stop_conditions parse bug (worker wrote "NOT triggered. ..." -> overnight.py:165 treated it as a trigger -> "LEFT FOR MORNING"). This is the SECOND ADM packet stranded by this bug (ADM-L4 was the first; I hand-landed both). The 02:30 fix suggestion (treat str stop_conditions as stopped only when a trigger is asserted positively, regex on "trigger(ed|s)?" not preceded by "NOT|none|no ") is now validated twice over - please take it.
- **ADM-001-ADAPTER adjudication remains OPEN and the re-dispatch risk is now LIVE**: ADM-003's slot freed, so only ADM-002 (slot 0) is running; the moment ADM-002 finishes, dispatch_ready WILL re-fork ADM-001 (row READY, no landed marker, write set disjoint from everything running). Its scoped check failed at 04:17Z (admission_conformance). Start here (unchanged): throwaway worktree at e076c1f, `cargo test -p truck-certified --test admission_conformance`, quiet machine, disk was ~10 GB free at the failure (not ENOSPC). If green -> land mechanically; if red -> worker amendment vs SPEC_GAP. Do not let the heartbeat re-fork it un-adjudicated.
- Carried (unchanged): F1 mvac-pin amendment (slot 4 residue); duplicate supervisors 35200 + 24272.

## 2026-09-09 05:20 UTC - ADM-002-CERTIFICATES finished but driver scoped-check raced the slot recycle (same signature as ADM-001's); ADM-001 re-forked un-adjudicated as warned

- **NEW - ADM-002-CERTIFICATES (worker commit 93a3001) is NOT landed and its scoped check verdict is UNTRUSTWORTHY.** Reconstruction from dispatch_heartbeat.log + overnight.log: ADM-002 (pid 30612) ran slot 0 from 04:15:55Z; the heartbeat re-forked slot 0 to ADM-001 at 01:08:33 local (05:08:33Z); overnight.log then logged at 01:08:57 local "slot 0: ADM-002-CERTIFICATES scoped check NOT green (test truck-certified:admission_certificates failed); left for morning" - 24 SECONDS AFTER the slot was re-forked. The check almost certainly ran against a worktree mid-reset to the ADM-001 fork (the driver checks the slot worktree; the reset was already underway). This is EXACTLY the ADM-001 artifact signature (its 04:17Z failure was logged ~90s after the 04:15:55Z re-fork). Both failures may be recycle-race artifacts rather than genuine defects.
- ADM-002's RESULT.json copy is LOST (recycle destroyed the slot-wt copy; none in loop/results or repo root) - third occurrence of the driver-never-files-RESULT / recycle-destroys-it machinery gap.
- **The re-runs are now the natural adjudicator and I did NOT interfere**: ADM-001 is RUNNING in slot 0 (pid 11104) re-forked from clean base 6fcd8aa (=integration HEAD, now INCLUDING the ADM-003 merge). Its events show it re-pulling e076c1f's admission.rs + admission_conformance.rs content and proceeding. If the earlier failure was load/recycle, the fresh run passes and lands; if genuine, its own done-when checks fail and it stops with evidence. dispatch_ready --dry-run shows ADM-002 deferred ONLY by the write-set clash with the RUNNING ADM-001 (admission.rs) - so ADM-002 WILL be re-forked fresh (redo ~40-50 min) the moment ADM-001 frees, its 93a3001 commit then orphaned on the packet branch. I judged the redo acceptable (self-adjudicating) and did NOT land 93a3001 out-of-order while ADM-001 runs (double-merge of admission.rs risk).
- Human ask: (1) confirm whether the driver's scoped check operates on the live slot worktree (if so, gate re-fork on the check, or make the check read the commit, not the worktree) - both ADM-001 and ADM-002 verdicts this night are consistent with check-vs-recycle races; (2) whether to pin ADM-001/ADM-002 rows vs let the natural re-runs decide. Start here: overnight.py scoped-check code path; the two re-runs' own outcomes this cycle are the cleanest evidence - watch slot 0 (ADM-001, pid 11104) and whichever slot takes ADM-002 next.
- Carried (unchanged): F1 mvac-pin amendment (slot 4 residue); duplicate supervisors 35200 + 24272; ADM-001/ADM-002 RESULT-copy loss class.

## 2026-09-09 05:47 UTC - ADM-001 re-run is GREEN but RESULT-less; landing needs a human; re-fork ping-pong is structural

- **ADJUDICATION ANSWERED: the 04:17Z admission_conformance failure was a recycle artifact, NOT a code defect.** The ADM-001 re-run (pid 11104, base 6fcd8aa) finished ~05:43Z having committed 8f6a549 (packet/ADM-001-ADAPTER, parent 6fcd8aa, admission.rs +461 -71... 1186 insertions, 71 deletions). cargoq server.log shows its OWN final done-when checks green at 01:39:35-01:39:37 local (cwd slots/0/wt): 'check --locked -p truck-certified' exit 0 + 'test --locked -p truck-certified --test admission_conformance' exit 0. The re-run adjudicates the earlier failure as the check-vs-recycle race the 05:20 escalation suspected.
- **BUT the RESULT.json is lost again (4th occurrence)**: the heartbeat re-forked slot 0 to ADM-002 at 01:43:41 local, ~1 min after the worker committed 8f6a549, before the overnight driver's next 5-min cycle (its 01:44:39 cycle logged only F1 + no-dispatch - the wt RESULT was already gone). No RESULT copy survives (checked loop/results, slot wt, repo root). The operator charter forbids merging without a RESULT.json status DONE, so this needs the ORCHESTRATOR's documented never-filed-RESULT protocol: scoped-verify 8f6a549 at merged HEAD, merge --no-ff, file loop/results/ADM-001-ADAPTER.json (status done), ledger row, flip row DONE. The code itself is green - do not redispatch for the missing file.
- **Structural ping-pong warning**: ADM-001 row is READY; ADM-002 (running slot 0, pid 19500) shares admission.rs, so dispatch_ready defers ADM-001 until ADM-002 frees it - then it WILL re-fork ADM-001 (3rd run, ~40 min redo of proven work) and the same 1-min heartbeat-vs-5-min-driver recycle race will strand ADM-002's commit in turn. Every run in this regime loses the landing race. Break it by (a) landing 8f6a549 now, and (b) fixing the race: gate the heartbeat's slot re-fork on the driver's scoped check having run (or land from the worker commit, not the live worktree).
- Start here: git -C C:\Users\stefa\look show 8f6a549 --stat; merge --no-ff 8f6a549 into integration/kernel-bg; cargo check --locked -p truck-certified + cargo test --locked -p truck-certified --test admission_conformance at merged HEAD.
- Carried (unchanged): F1 mvac-pin amendment (slot 4 residue, row DONE - needs re-pointing mvac_row_still_refuses_typed_at_the_extrude_carrier in truck123d/tests/ttc_lathe_spline.rs per the F1 finding); duplicate supervisors 35200 + 24272 (still only ONE overnight.py child 37284 under 24272 - no double-merge risk this cycle); driver-never-files-RESULT machinery gap; overnight.py 'NOT triggered.' prose-stop bug (now stranded ADM-L4 + ADM-003, both hand-landed).

## 2026-09-09 06:1x UTC - driver parked for morning (lands nothing); ADM-002 RESULT now PRESERVED; landing order fixed as ADM-001 then ADM-002

- **The overnight driver (37284) is in PERMANENT LEFT-FOR-MORNING and will NOT land ADM-001 or ADM-002.** Every 5-min cycle from 01:19 to 02:10 local logs only 'slot 4: F1-AUTHORING-ARMS landed-with-findings -> LEFT FOR MORNING (judgment required)' then 'landing/running phase - no dispatch'. The F1 slot-4 judgment residue parks the WHOLE driver. Consequence: the heartbeat ping-pong on admission.rs will keep re-forking ADM-001 (currently on its THIRD re-run, pid 19604) and ADM-002, and nothing will ever land until a human either lands the finished commits or clears the F1 residue / fixes the driver's prose-stop misparse (which separately parks every done ADM packet: ADM-L4, ADM-003, and now ADM-002 all logged stopped=True on 'NOT triggered.' prose).
- **NEW: ADM-002's RESULT SURVIVES after all** - loop/results/ADM-002-CERTIFICATES.PENDING.RESULT.json (status done, commit b3c1346, branch packet/ADM-002-CERTIFICATES) was archived by the driver's LEFT-FOR-MORNING path at 01:59:57 local. This supersedes the 05:20 escalation's 'ADM-002 RESULT lost'. The 05:47/04:30/04:56 'ADM-002 not landable, RESULT copy lost' claims should be corrected: b3c1346 IS landable under the normal operator protocol once the live ADM-001 write-set on admission.rs frees.
- **ADM-001's adjudicated-green 8f6a549 was orphaned by the 06:04:37Z re-fork** (packet branch reset to base 4d979e8). It is preserved at refs/wip/ADM-001-8f6a549-green-adjudicated (I created the ref). Its RESULT.json remains genuinely lost (no PENDING archive - the driver never processed that slot pre-recycle). Needs the orchestrator never-filed-RESULT landing protocol: scoped-verify 8f6a549 at a worktree (cargo check --locked -p truck-certified + cargo test --locked -p truck-certified --test admission_conformance - its own final checks were green in server.log 01:39 local), merge --no-ff, file loop/results/ADM-001-ADAPTER.json, ledger row, flip DONE.
- **Landing order matters and I did NOT land ADM-002 first**: b3c1346's parent 531b370 has no ADM-001 content; the live ADM-001 re-run is based on 4d979e8 which has no ADM-002 content. Landing ADM-002 while ADM-001 is mid-file on admission.rs forces a semantic double-merge of admission.rs on ADM-001's eventual commit (the two assemblies both extend the Ssi4System admission entry). Land ADM-001 8f6a549 FIRST, then ADM-002 b3c1346 (RESULT preserved) - OR land 8f6a549 and let ADM-002's next auto-run (forked from the post-ADM-001 integration) absorb it and re-derive.
- **F1 slot-4 residue is now load-bearing for the WHOLE landing pipeline**: clearing it (the mvac-pin amendment to truck123d/tests/ttc_lathe_spline.rs re-pointing mvac_row_still_refuses_typed_at_the_extrude_carrier) un-parks the driver. Without it, even the driver-side fix for the prose-stop bug would not resume landings.
- Start here: land ADM-001 8f6a549 (git merge --no-ff 8f6a549 into integration/kernel-bg after scoped-verifying; file the RESULT per the protocol), then ADM-002 b3c1346 (RESULT at loop/results/ADM-002-CERTIFICATES.PENDING.RESULT.json). Then fix overnight.py:165 (treat str stop_conditions as stopped only on a positive trigger assertion, NOT 'NOT|none|no ' preceding 'trigger') and adjudicate the F1 mvac-pin amendment to un-park the driver.
- Carried (unchanged): duplicate supervisors 35200 + 24272; two cargoq/server.py observed (8132 + 12504, ping/queue fine - flagged, not adjudicated); driver-never-files-RESULT machinery gap; check-vs-recycle race.

## 2026-09-09 15:56 UTC - RESOLVED the ADM saga; NEW: cargoq guard wedged + slot-4 residue + TOR-C flip decision

- **RESOLVED / close the 04:30-06:1x ADM escalations**: the ADM-001/ADM-002
  never-filed-RESULT saga is CLOSED by the orchestrator - ADM-001 landed as
  b805ddd (worker 565ba8b, 3rd re-run), ADM-002 landed as 345e635 (worker
  5a44609), rows LANDED in 3ef4dab (verified ancestors of integration/kernel-bg
  HEAD). ADM-004, TTC-RECENSUS-F1, AUTHOR-FRAME-CARRIERS and the F1 mvac-pin
  amendment (5f1396d/da76ec0) are all landed. The overnight.py prose-stop bug is
  FIXED (d8025d8). Nothing to land from the ADM era.
- **NEW - cargoq was DOWN and the supervisor restart guard is wedged.** cargoq
  server.log died 02:29:54 (exit 1073807364) and never returned; the supervisor
  (restarted 15:48 by the launcher, then 15:51:17Z by the orchestrator session)
  logged only its start lines - no 'cargoq server not running - restarting' line
  ever, even though cargoq was down for 13 hours. supervisor.py:45's
  alive('cargoq') probe appears to false-positive on launcher/orchestrator
  powershell command lines that contain the literal text 'cargoq' (the 15:48
  bootstrap's instance-count probe does), OR the supervisor wedges inside its
  WMI alive() subprocess. The OPERATOR restarted the cargoq server directly at
  15:54Z (ping OK, queued 0) - but the guard itself needs a human look (alive()
  should probe the server port, not process command lines). Start here:
  supervisor.py:27-48 and the 15:48/15:51 bootstrap command lines.
- **Slot-4 F1 residue still parks the overnight driver's dispatch arm** even
  though the F1 packet is fully resolved (row DONE, ledger LANDED, mvac-pin
  amendment landed, RESULT archived in loop/results/F1-AUTHORING-ARMS*.json).
  The stale 'landed-with-findings' RESULT.json in the slot-4 worktree root makes
  overnight.py log 'LEFT FOR MORNING' every 5-min cycle (dispatch arm parked;
  per-slot landing attempts at other slots still run, so slots 0/7 are NOT
  blocked). run_packet --reset-only reports the slot bookkeeping-clean, so the
  residue is the wt-root file itself. Recommend the orchestrator clear it
  (heartbeat recycle on the next slot-4 assignment, or a wt-root RESULT.json
  delete commit) to silence the park.
- **TOR-C is BLOCKED with all deps now landed** (needs ADM-001-ADAPTER +
  ADM-002-CERTIFICATES; both LANDED per 3ef4dab). Per the registry rule it
  qualifies to flip BLOCKED->READY, and the owner ruling ('dispatch after
  ADM-001+ADM-002 land') is now satisfied. NOT flipped by the operator: the
  orchestrator session is LIVE and sequencing the door-gap program (last commit
  3c65c6a = SWEEP-PATH registered), and a flip would dispatch a heavy third
  worker. Flip+dispatch or pin is the orchestrator's call.
- Carried (now current pids): duplicate supervisors 27392 (PyManager) + 15100
  (pythoncore, child of 27392 - likely the PyManager-shim spawn pattern); only
  ONE overnight.py child (28824) = no double-merge risk this cycle.

## 2026-09-09 22:06 UTC - FRAME-REVOLVE finished but NOT landable (status LANDED, mvac-pin finding); driver merge-conflict-aborts every cycle

- What: FRAME-REVOLVE (slot 7) went FINISHED ~22:00Z, worker commit b667a85
  (parent 57aa9ad, which IS an ancestor of integration/kernel-bg HEAD). RESULT.json
  in the slot wt has status "LANDED" (not DONE) and carries finding F1: the
  mvac-pin class. The packet's own done-when is NOT green at the commit - the
  RESULT's own verification records `revolve_refusals_stay_typed_after_spline_
  admission` FAILED on its non_z_axis sub-case in truck123d/tests/ttc_lathe_
  spline.rs (file OUTSIDE this packet's write scope) because an axis-aligned (x)
  ring revolve is now a recorded carrier. The three named kernel tests in
  bd_bridge.rs are green (RESULT: lib 25 passed), but a full
  `cargo test -p truck123d --lib --tests` exits nonzero. Two independent
  do-not-land triggers per the charter (status != DONE; scoped check not green).
- Driver behavior: overnight.py treats status "landed" as good and attempted a
  landing at 18:05:00 local, but the merge CONFLICTED on CONTEXT.md/PACKET.md
  (harness artifacts the worker committed into b667a85) and it aborted cleanly
  (`git merge --abort`; no MERGE_HEAD, b667a85 NOT merged, wt RESULT intact). It
  will retry and abort identically every 5-min cycle - noise, not harm.
- Why it needs a human: this is the mvac-pin adjudication class. The F1 finding
  argues the non_z_axis typed-refusal pin should move (x-axis ring revolve is now
  expressible), mirroring the mvac pin the orchestrator adjudicated by amendment
  (5f1396d/da76ec0). Likely resolution: land b667a85 excluding the worker's
  CONTEXT.md/PACKET.md edits (merge --no-ff, then `git checkout HEAD -- CONTEXT.md
  PACKET.md` before committing, or amend the worker commit to drop them), file
  loop/results/FRAME-REVOLVE.json, then re-point the ttc_lathe_spline.rs pin in a
  small amendment (out-of-write-scope, orchestrator-owned, same as mvac). SWEEP-
  PATH (READY) is gated on FRAME-REVOLVE landing.
- Start here: `git -C C:\Users\stefa\look show b667a85 --stat`; adjudicate the F1
  pin move; merge per the harness-artifact-safe recipe above. Do NOT re-fork slot
  7 until adjudicated - the RESULT.json in the slot wt is the only copy (the
  driver's merge-conflict path does not archive a PENDING copy).
- Carried (unchanged): duplicate supervisors 27392 + 15100; wedged cargoq
  supervisor restart guard; slot-4 F1 wt RESULT residue parking the driver's
  dispatch arm; TOR-C flip-or-pin (orchestrator LIVE - its call).

## 2026-09-10 01:2xZ - FRAME-REVOLVE landed with the F1 non_z_axis pin UNAMENDED; slot-7 redundant re-fork residue

- What: FRAME-REVOLVE was landed by the orchestrator (merge 39e9550 of worker
  b667a85; row DONE ca4a498) but the F1 finding's recommended follow-up did NOT
  land: `truck123d/tests/ttc_lathe_spline.rs` still pins `non_z_axis` at line
  255 (`revolve_refusals_stay_typed_after_spline_admission`), while the x-axis
  ring revolve is now a recorded carrier. `cargo test -p truck123d --tests` is
  therefore expected to fail at HEAD (the packet's own RESULT recorded it red).
  Either amend the pin (mirroring the mvac-extrude pin move 5f1396d) or record
  the acceptance explicitly. Start here: `git -C C:\Users\stefa\look show
  b667a85 --stat`; ttc_lathe_spline.rs:200-260.
- Also: a redundant heartbeat re-fork ran FRAME-REVOLVE again in slot 7 (the row
  was READY with no landed marker at re-fork time), finishing with a RESULT
  status LANDED and NO commit. The slot-7 wt RESULT is now residue of the same
  shape as the slot-4 F1 park; both park the overnight driver's dispatch arm
  each 5-min cycle. Clear both wt-root RESULT files (or let a recycle do it).
- Carried (unchanged): duplicate supervisors 27392 + 15100; wedged cargoq
  supervisor restart guard; TOR-C flip-or-pin (orchestrator LIVE - its call).
