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

## 2026-09-10 01:59Z - substrate restarted ~01:55Z; new PIDs; duplicate supervisors persist

- What: the substrate restarted ~01:55Z (21:55 local). All substrate PIDs
  changed: heartbeat 27872, operator runner 27876, watchdog 24472, overnight
  driver 26920, cargoq 28544; the old orchestrator opencode 17740 is gone and 3
  opencode processes are now observed (14776/27196/28356) - a human should
  confirm which is the orchestrator session. The supervisor's restart guard DID
  recover cargoq + the driver on its own ~2 min after the restart (cargoq now
  answers HTTP 200), so the carried "wedged guard" concern is downgraded to a
  ~2-min lag, but the supervisor duplication persists (now 19172 PyManager +
  27828 pythoncore) and each duplicate can independently start a driver/cargoq -
  the double-merge hazard class. Adjudicate/de-duplicate the twin supervisors.
- Start here: `Get-CimInstance Win32_Process | ? { $_.CommandLine -match
  'supervisor.py' }`; `Get-Content loop\supervisor.log -Tail 15`.
- Carried (unchanged): FRAME-REVOLVE F1 non_z_axis pin amendment
  (ttc_lathe_spline.rs:255); slot-4 + slot-7 wt RESULT residue parking the
  driver's dispatch arm; TOR-C flip-or-pin (orchestrator LIVE).

## 2026-09-10 11:33Z - program is DISPATCH-IDLE after CG-BINDING landed; final verify battery outstanding

- What: the overnight driver landed the last booked long pole, CG-BINDING (the
  pyo3 translation) at 58d1e05 (merge 8229c84). This cycle re-derived the board
  by command: 0 RUNNING, `dispatch_ready --max-workers=4` reports 0 dispatched,
  every READY row carries a landed marker, and all 7 BLOCKED rows have all deps
  landed (all parked for owner/orchestrator). There is therefore nothing left
  for the loop to dispatch or land. The remaining program step is the SINGLE
  end-of-program verify battery at the integrated HEAD (the one-verify
  amendment), which is orchestrator/owner work, not operator work.
- Also outstanding (unchanged): F1-AUTHORING-ARMS sits LANDED-WITH-FINDINGS in
  slot 4 and the driver logs it "LEFT FOR MORNING (judgment required)"; and the
  FRAME-REVOLVE F1 non_z_axis pin (ttc_lathe_spline.rs:255) is still unamended,
  so `cargo test -p truck123d --tests` is expected red at HEAD.
- Start here: `git -C C:\Users\stefa\look log --oneline -3 integration/kernel-bg`
  (HEAD 58d1e05); `python loop/dispatch_ready.py --max-workers=4`; then run the
  end-of-program verification campaign against HEAD.
- Carried (unchanged): duplicate supervisors 19172 + 27828; lagging cargoq
  restart guard; slot-4 + slot-7 wt RESULT residue parking the driver's dispatch
  arm; TOR-C flip-or-pin (orchestrator-held).


## 2026-09-10 11:57 UTC - the 11:33Z operator cycle left its loop files UNCOMMITTED (machinery gap, recovered)

- What: the 11:33Z operator cycle wrote its three deliverables (loop/STATE.md,
  loop/OPERATOR_LOG.md, loop/OPERATOR_ESCALATIONS.md) but never committed them
  - HEAD's newest operator commit was 76de2a9 (the 11:09Z cycle); the 11:33Z
  STATE/LOG/ESCALATIONS deltas sat uncommitted in the working tree (`git
  status`: ` M loop/STATE.md`, ` M loop/OPERATOR_LOG.md`, ` M
  loop/OPERATOR_ESCALATIONS.md`). `git log --all --grep=11:33` finds no commit
  anywhere. The 11:57Z operator committed them together with its own refresh,
  so the content is preserved (the files were on disk the whole time, so
  cold-start readers still saw them).
- Why it needs a human: low-severity recurrence of the
  "cycle-ends-before-its-commit" class. If a later cycle had hard-reset or
  checked out the tree, the previous cycle's STATE/LOG/ESCALATIONS edits would
  be lost, and stale STATE is the loop's most expensive failure mode. Worth a
  look at the operator runner's kill-at-15 vs finish-under-12 budget: the
  11:33Z instance may have been killed after its report writes but before its
  commit step.
- Start here: loop/operator_runner.ps1 (the runner's kill timing) and the
  OPERATOR_LOG 11:33Z entry (its report reads as complete, so the kill most
  likely landed after the writes but before the commit). The 11:57Z commit
  carries both the 11:33Z and 11:57Z deltas.

## 2026-09-10 12:43 UTC - BRIDGE-BOOLEANS unlanded, RESULT destroyed by the recycle race; serialization bypassed

- What: BRIDGE-BOOLEANS finished in slot 0 (worker commit c0329e0, RESULT.json
  status "LANDED", 5 named tests green per the worker). The heartbeat's
  08:41:52 local cycle re-forked slot 0 to dispatch BRIDGE-LOFT-FACTS ~9s
  before the overnight driver's 08:42:01 landing cycle, which then logged
  "slot 0: FINISHED without RESULT; left for morning". The unlanded RESULT.json
  was destroyed (5th occurrence of the driver-files-never /
  recycle-destroys-RESULT class). c0329e0 is preserved at
  refs/wip/BRIDGE-BOOLEANS-c0329e0-preserved (and on packet/BRIDGE-BOOLEANS).
- Why it needs a human: the packet is unlanded, its registry row is READY with
  no landed marker, so dispatch_ready WILL re-dispatch it (wasting a worker run)
  once the write set frees. Deciding whether to land c0329e0 as-is or let it
  re-run is judgment (the charter's non-DONE/destroyed-RESULT rule says the
  operator must not land it). Compounding: BRIDGE-LOFT-FACTS was forked from
  1c24aab WITHOUT the BRIDGE-BOOLEANS changes to the same files
  (truck123d/src/bd_bridge.rs, corpus/ttc/door.py), so the second to land will
  conflict - the intended serialization was bypassed because BRIDGE-BOOLEANS
  had finished (not RUNNING) when the heartbeat dispatched.
- Also: the overnight driver's scoped_check derives crates from the row's
  write paths (only vendor/truck/* -> crates) and test pairs from
  vendor/truck/*/tests/*.rs in the packet text. For BRIDGE-BOOLEANS both are
  empty, so it falls back to `cargo check -p truck-certified` and runs NO named
  tests - had the RESULT survived, the driver would have landed a truck123d
  packet without gating its 5 named tests.
- Start here: `git log --oneline -1 c0329e0`; `git show --stat
  refs/wip/BRIDGE-BOOLEANS-c0329e0-preserved`; the RESULT content is
  reconstructible from the packet's done-when + the 5 named tests in
  truck123d/src/bd_bridge.rs. Fix candidates: (a) dispatch_ready must treat a
  FINISHED-but-unlanded slot as holding the write set until the row is
  LANDED/DONE; (b) the driver's packet_tests_and_crates should read the
  packet's `crates:`/`tests_required:` yaml, not the write paths; (c) the
  heartbeat must archive the slot RESULT before any recycle.

## 2026-09-10 13:06 UTC - PARTIAL RESOLVE: BRIDGE-BOOLEANS landed; BRIDGE-LOFT-FACTS merge-conflict risk now live

- Resolved: the 12:43Z "BRIDGE-BOOLEANS unlanded, RESULT destroyed" item.
  BRIDGE-BOOLEANS is now status DONE and c0329e0 is an ancestor of
  integration/kernel-bg (orchestrator commits 4e99196 "row LANDED", 9c4ac9e,
  b82b035). No re-dispatch risk; the land-vs-rerun decision is moot.
- Still open (human): the intended serialization between BRIDGE-BOOLEANS and
  BRIDGE-LOFT-FACTS was bypassed, and BRIDGE-LOFT-FACTS (running slot 0,
  forked from 1c24aab) touches the SAME files (corpus/ttc/door.py,
  truck123d/src/bd_bridge.rs). BRIDGE-BOOLEANS is now on integration, so when
  BRIDGE-LOFT-FACTS lands it will conflict on those files - not the one-line
  `pub mod` kind. The landing owner must resolve bd_bridge.rs / door.py.
  Start from `git -C C:\Users\stefa\look merge-tree integration/kernel-bg
  packet/BRIDGE-LOFT-FACTS`.
- Still open (human, carried from 12:43Z): the heartbeat recycle can destroy a
  finished slot's RESULT.json before landing; the driver's scoped_check derives
  crates/tests from write paths, so a truck123d packet with no vendor/truck
  write paths gates nothing.

## 2026-09-10 13:50 UTC - main worktree left MID-MERGE by the driver (MERGE_HEAD + UU bd_bridge.rs); operator aborted to restore clean integration

- What: on arrival this cycle, integration/kernel-bg (the main worktree,
  C:\Users\stefa\look) was mid-merge: `MERGE_HEAD` = 8b46b64 (BRIDGE-LOFT-FACTS),
  `.git/MERGE_HEAD` mtime 09:33:57 local, `git status` showed `UU
  truck123d/src/bd_bridge.rs` (unresolved conflict markers) plus staged harness
  artifacts `M PACKET.md` / `A RESULT.json`. The driver had logged a
  conflict-ABORT for this packet at 09:26:27 local, so the 09:33:57 merge was a
  LATER attempt whose `git merge --abort` (overnight.py:250) never ran
  (interrupted cycle). This is worse than the logged "aborted, left for morning"
  state: a half-finished merge blocks every subsequent landing and any `git
  commit` (git refuses while unmerged paths exist).
- Operator action (mechanical restoration; no merge, no conflict resolution):
  `git merge --abort` -> exit 0; HEAD back to e700246, no MERGE_HEAD, only the
  pre-existing modified battery/cargoq logs remain. Nothing was lost: 8b46b64
  (and its RESULT.json, status DONE) is intact on packet/BRIDGE-LOFT-FACTS and
  at `git show 8b46b64:RESULT.json`.
- Still open (human) - the conflict itself: BRIDGE-LOFT-FACTS (8b46b64, DONE
  RESULT) conflicts with the landed BRIDGE-BOOLEANS in truck123d/src/bd_bridge.rs
  (and PACKET.md). This is the 12:43Z "bypassed serialization" item materialized.
  The landing owner must rebase/resolve. Start from
  `git merge-tree integration/kernel-bg packet/BRIDGE-LOFT-FACTS` (or
  `git rebase integration/kernel-bg packet/BRIDGE-LOFT-FACTS`).
- Anchor hygiene warning: while the tree was mid-merge, `gen_packet --check
  loop/packets/BRIDGE-LOFT-FACTS.md` reported A1=2/A3=57 (conflict-tree values).
  On the CLEAN HEAD the real counts are A1=0, A2=0, A3=40 vs the packet's
  expected A3=37 - a +3 drift from the BRIDGE-BOOLEANS landing, NOT the conflict.
  Do NOT re-measure anchors from a conflicted tree.
- Also (machinery): overnight.py must guarantee `git merge --abort` on an
  interrupted cycle (or the supervisor should detect a lingering MERGE_HEAD and
  clean it), else a killed landing cycle wedges the integration worktree.
- POST-COMMIT ADDENDUM (2026-09-10T13:52Z): minutes after the operator's
  `git merge --abort` (reflog HEAD@{1}/{2} "reset: moving to HEAD") and the
  operator commit 80087ab, `git status` showed a NEW unstaged, marker-free
  modification: `truck123d/src/bd_bridge.rs` +238 insertions vs HEAD (mtime
  09:46:31 local), NO MERGE_HEAD, nothing staged. It is not the operator's doing
  (the commit touched only the three loop files) and it is NOT a clean revert of
  the conflict. Most likely the overnight driver's next landing cycle re-merged
  BRIDGE-LOFT-FACTS and left the auto-merged worktree change behind without
  committing or aborting. The operator did NOT touch it (destroying in-progress
  work is out of scope). A human should decide: `git diff -- truck123d/src/
  bd_bridge.rs` and either commit it as the BRIDGE-LOFT-FACTS resolution or
  `git checkout -- truck123d/src/bd_bridge.rs` (8b46b64 is safe on its branch).
  This is the same driver-interrupted-landing class as the MERGE_HEAD item.

## 2026-09-10 16:33 UTC - DUPLICATE MONO-2 worker (slot 0 + slot 1) after the heartbeat reset a LIVE worker's worktree

- What: `python loop/slot_status.py` shows TWO RUNNING slots on the same packet,
  MONO-2-NSTATION-LOFT: slot 0 (cmd pid 25604, forked 12:25:06 local, branch
  `packet/MONO-2-NSTATION-LOFT@c6bd3fb`) and slot 1 (cmd pid 17728, the original
  16:10Z worker, opencode session started 12:01, now detached HEAD `e37938a`).
  Both are alive and both are queued through cargoq on the same tests
  (`test -p truck123d --lib line_loft_rows_answer_bit_identically`,
  `check -p truck123d`). This wastes a worker slot and risks two writers to the
  same packet.
- Why (evidence): the heartbeat's slot-liveness check lost the LIVE slot-1
  worker during its long silent cargo build. `loop/dispatch_heartbeat.log`:
  12:04:52 and 12:14:56 both report "1 running, 7 free"; 12:25:00 reports
  "0 running, 7 free" and dispatches MONO-2 to slot 0. In that same 12:25 cycle
  it re-forked slot 1 (git reflog in `loop/slots/1/wt`: "checkout: moving from
  packet/MONO-2-NSTATION-LOFT to e37938a" then "reset: moving to
  integration/kernel-bg"), ARCHIVING the worker's uncommitted +949-line diff to
  `loop/slots/1/abandoned-20260910-122504.patch` (73097 bytes, 12:25:04). The
  run_packet then FAILED with `PermissionError: [WinError 32] ... slots\1\
  events.jsonl` (locked by the still-live worker pid 17728), so the OLD worker
  kept running on the freshly reset tree while a NEW duplicate was spawned in
  slot 0. Net: two independent workers on one packet, and slot 1's in-progress
  work exists only in the abandoned patch.
- Operator action: NONE beyond recording. Both workers are alive and making
  progress; the charter forbids killing/restarting a live worker. Did not touch
  either worker, did not dispatch, did not reset.
- What a human should do (in order):
  1. Decide which worker to keep. If slot 1: recover its work with
     `git -C loop/slots/1/wt apply loop/slots/1/abandoned-20260910-122504.patch`
     (verify against the current amended packet first). If slot 0: kill the
     slot-1 worker (pid 17728) and its cargoq jobs, then let slot 0 finish. Do
     NOT let both commit - slot 0 holds the branch, slot 1 is detached, so a
     slot-1 checkout/commit could move the branch under slot 0.
  2. Fix the heartbeat's slot-liveness detection: a slot with a live
     `worker.pid` must count as running even if `events.jsonl` is momentarily
     stale during a long cargo build.
  3. Guard `run_packet`/`new_slot` re-fork behind a live-pid check so a reset
     cannot run out from under a worker (the events.jsonl PermissionError is the
     only reason this reset did not also destroy the live session).
- Start from: `python loop/slot_status.py`;
  `git -C loop/slots/1/wt reflog`;
  `Get-Content loop/slots/1/abandoned-20260910-122504.patch | Select-Object -First 40`.

## 2026-09-10 16:57 UTC - MONO-row registry schema gap: dispatch_ready cannot see `depends_on`/`write_allow`

- What: the 4 MONO-CLOSURE rows (MONO-1..MONO-4) are the only registry rows
  using the new schema keys `depends_on` and `write_allow`; the other 313 use
  `needs` and `writes`. `dispatch_ready.py` reads only `needs` (line 186) and
  `writes` (line 192), so for the MONO rows the dependency gate AND the
  write-set clash check are both no-ops. Consequence: nothing serializes
  MONO-3/MONO-4 on their shared `truck123d/src/bd_bridge.rs` write set; if both
  are READY at once the heartbeat would dispatch two concurrent writers to the
  same file (the exact same-file-parallelism hazard the 16:35Z orchestrator
  note forbids).
- Operator action: flipped MONO-3-BLADE-MEMBERS-MIRROR BLOCKED->READY (dep
  MONO-2 landed; anchors + lint green). Left MONO-4-TRIM-IDIOMS BLOCKED so
  only one bd_bridge.rs writer can ever be live. `dispatch_ready --dry-run`
  confirms exactly one dispatch (MONO-3 -> slot 0).
- What a human should do: normalize the MONO rows to the dispatcher schema
  (`depends_on` -> `needs`, `write_allow` -> `writes`) or teach dispatch_ready
  to read both key pairs; then MONO-4 can be flipped READY and will serialize
  automatically behind MONO-3. Start from `loop/dispatch_ready.py:186` and
  `loop/dispatch_ready.py:192`, then re-run
  `python loop/dispatch_ready.py --dry-run`.
- Carried (unchanged): FRAME-REVOLVE F1 non_z_axis pin amendment
  (truck123d/tests/ttc_lathe_spline.rs:255); duplicate supervisors + lagging
  cargoq restart guard; slot-4 + slot-7 wt RESULT residue; TOR-C flip-or-pin;
  heartbeat slot-liveness duplicate-dispatch bug (root cause of the 16:33Z
  MONO-2 duplicate; now moot for that packet but structurally unfixed);
  overnight.py guarantee-merge-abort on interrupted cycles.

## 2026-09-10 18:38 UTC - FALSE LANDINGS: the driver merges the BASE when a worker wrote a RESULT but never committed (MONO-4 + SOLVER-SURVEY-C)

- What: `overnight.py`'s landing path (`loop/overnight.py:222-226`) takes
  `head = git rev-parse --short HEAD` **in the slot worktree** and runs
  `git merge --no-ff <head>`. When a worker finishes with a DONE/complete
  RESULT but exits before committing (the skipped-commit-step class already
  recorded 4x, e.g. MONO-2 026b4e9), the branch tip is still the packet BASE,
  so the "merge" is a no-op ("Already up to date", returncode 0) and the
  driver still files the row and appends `LANDED <base>`. The worker's real,
  uncommitted work is never landed and is destroyed when the heartbeat
  re-forks the slot. Two false landings observed this cycle:
  - **MONO-4-TRIM-IDIOMS**: `overnight.log 09-10 14:28:33 slot 0:
    MONO-4-TRIM-IDIOMS LANDED at 641b120`. 641b120 is the 17:23Z **operator**
    commit (the base that flipped MONO-4 READY), not worker work. HEAD cc38b4f
    touched only `loop/PACKETS.jsonl`;
    `git grep spline_profile_prism_facts HEAD -- truck123d/src/bd_bridge.rs`
    is EMPTY (the 405-line idiom implementation is absent). Slot 0 was then
    re-forked to `packet/SOLVER-SURVEY-A` (slot-0 reflog HEAD@{1}) and the
    worktree work + RESULT.json were discarded. **The work is preserved** at
    `loop/slots/0/abandoned-20260910-143257.patch` (30,756 b; contains the
    bd_bridge.rs diff and the untracked RESULT.json content) and in the worker
    session `ses_f73947dc8ffeCJdwH4f17cGmp9` inside
    `loop/slots/0/events.jsonl` (329 refs - but this file will be overwritten
    when the SURVEY-A worker writes; the patch is the durable artifact).
  - **SOLVER-SURVEY-C**: `overnight.log 09-10 14:35:25 slot 3:
    SOLVER-SURVEY-C LANDED at 86d28a3`. 86d28a3 is the operator base; HEAD
    21be490 touched only `loop/PACKETS.jsonl`;
    `git ls-files loop/solver_coverage/fragments` is EMPTY - the 123 KB
    `C.json` deliverable is NOT in HEAD. **SOLVER-SURVEY-B** (RESULT status
    `complete`, a `good` status at overnight.py:182) will be false-landed the
    same way on the next driver cycle; its 403 KB `B.json` is still untracked
    in `loop/slots/2/wt`.
- Operator action: preserved the at-risk survey fragments as refs via
  commit-tree (worktree files left untracked/unstaged):
  `refs/wip/SOLVER-SURVEY-B-fragment` = 0a4b4c7,
  `refs/wip/SOLVER-SURVEY-C-fragment` = 7f11452. Did NOT merge, land, relaunch,
  or edit any packet/registry semantics. Did NOT touch MONO-4's false DONE row
  (PACKETS.jsonl is outside the operator's allowed files).
- What a human should do:
  1. Fix `overnight.py` so a landing REQUIRES the worker's changes to be
     committed - either commit the slot worktree "AS DELIVERED" before merging
     (the MONO-2 026b4e9 precedent) or treat `head == base` / a no-op merge as
     NOT LANDED (leave for morning). The scoped check should also run on the
     committed candidate, not a mid-reset worktree.
  2. Re-land MONO-4 from `loop/slots/0/abandoned-20260910-143257.patch`
     (apply onto integration/kernel-bg, run the packet's scoped check, then
     correct the row's false `LANDED 641b120` note).
  3. Re-land the survey fragments from `refs/wip/SOLVER-SURVEY-{B,C}-fragment`
     (or re-dispatch those survey packets); SOLVER-CHECKER depends on all four
     fragments. Note the driver may already have flipped C (and soon B) DONE.
  4. Do NOT flip MONO-5-RAY-CLASSIFY / MONO-6-SWEPT-BOOLEANS READY until the
     real MONO-4 code is landed - they `depends_on` MONO-4, and dispatch_ready
     does not read `depends_on`, so a manual flip would run them against a HEAD
     missing the trim idiom.
- Start from: `loop/overnight.py:222-226`;
  `loop/slots/0/abandoned-20260910-143257.patch`;
  `git show refs/wip/SOLVER-SURVEY-B-fragment`;
  `git show refs/wip/SOLVER-SURVEY-C-fragment`.

- Carried (unchanged): FRAME-REVOLVE F1 non_z_axis pin amendment
  (truck123d/tests/ttc_lathe_spline.rs:255); duplicate supervisors + lagging
  cargoq restart guard; slot-1/4/7 wt RESULT residue; TOR-C flip-or-pin;
  heartbeat slot-liveness duplicate-dispatch bug; MONO-row registry schema gap.

## 2026-09-10 19:33 UTC - 6th FALSE LANDING: MONO-5-RAY-CLASSIFY (driver landed the base b34ec4e; mechanism absent)

- What: `overnight.log` `09-10 15:24:59 slot 0: MONO-5-RAY-CLASSIFY LANDED at
  b34ec4e`. b34ec4e is the packet BASE (the READY flip), not worker work. The
  landing commit 906dc59 (HEAD) changed only `loop/PACKETS.jsonl` - it appended
  `LANDED b34ec4e (overnight, one-verify amendment)` to the READY row's note.
  The real mechanism is ABSENT from HEAD: in `truck123d/src/bd_bridge.rs`,
  `grep -ci classify` = 0 and `grep -ci bicubic` = 0, while the packet's
  done-when requires A2/A3 to drift 0 -> >=1. Same `overnight.py:222-226` root
  cause as the 18:38Z MONO-4/SOLVER-SURVEY cluster: it merges the slot-wt HEAD
  even when the worker never committed.
- Why it matters now: the appended `LANDED b34ec4e` note makes dispatch_ready's
  `landed()` SKIP the MONO-5 row forever (the LANDED_RE trap), so MONO-5 will
  never re-dispatch while its code is missing. MONO-6-SWEPT-BOOLEANS
  (`needs: [MONO-5, MONO-2]`) reads all-deps-landed and would be the next
  false-frontier if anyone flips it.
- Work preserved: the MONO-5 worker's WIP is in
  `loop/slots/0/abandoned-20260910-152616.patch` (`git apply --stat`: 1161-line
  bd_bridge.rs diff, incl. the `// MONO-5-RAY-CLASSIFY -- certified
  point-vs-spline-solid membership` section, plus CONTEXT.md/PACKET.md harness
  artifacts). The worker session was wiped by the slot-0 re-fork to
  SOLVER-SURVEY-D. `packet/MONO-5-RAY-CLASSIFY` tip is b34ec4e (base) - no
  worker commit exists.
- What a human should do:
  1. Adjudicate MONO-5: apply `abandoned-20260910-152616.patch` onto
     integration/kernel-bg (strip the CONTEXT.md/PACKET.md hunks - harness
     artifacts), run the packet's scoped check + A2/A3 anchors, and if green
     land AS DELIVERED and correct the row's false `LANDED b34ec4e` note.
     Otherwise re-flip the row READY (remove the false landed marker so
     dispatch_ready re-dispatches it).
  2. Fix `overnight.py:222-226` so a landing REQUIRES a committed worker diff
     (treat `head == base` / a no-op merge as NOT LANDED). Same item as the
     18:38Z escalation - now the 6th strike.
  3. Do NOT flip MONO-6-SWEPT-BOOLEANS until MONO-5's real code is landed.
- Start from: `loop/overnight.py:222-226`;
  `loop/slots/0/abandoned-20260910-152616.patch`; `git show 906dc59`;
  `grep -ci classify truck123d/src/bd_bridge.rs`.

## 2026-09-10 19:54 UTC - 7th FALSE LANDING: SOLVER-SURVEY-D (base merged; D.json uncommitted) + SOLVER-SURVEY-A still marker-blocked

- What: `overnight.log` `09-10 15:35:08 slot 0: SOLVER-SURVEY-D LANDED at
  906dc59`. 906dc59 is the packet BASE (packet/SOLVER-SURVEY-D tip == base, no
  worker commit); HEAD 9e19077 touched only `loop/PACKETS.jsonl` (appended
  `LANDED 906dc59` to the READY row's note). The survey work IS real but
  uncommitted: slot 0 wt holds an untracked
  `loop/solver_coverage/fragments/D.json` (170,327 b, 5015 lines) + a
  status-DONE `RESULT.json`; `git ls-tree HEAD loop/solver_coverage/fragments/`
  = B.json, C.json only (D absent). Same `overnight.py:222-226` root cause as
  the 18:38Z cluster and the 19:33Z MONO-5 strike.
- Why it matters now: the appended `LANDED 906dc59` note makes
  `dispatch_ready.landed()` (status==done OR note matches `landed [0-9a-f]{7,}`)
  SKIP SOLVER-SURVEY-D forever while its fragment is missing from HEAD.
- SECOND stranded row, same class: **SOLVER-SURVEY-A is status=READY but its
  note still carries `LANDED cc38b4f`** (cc38b4f is an ancestor = the base), so
  `landed()` returns True and the dispatcher skips it too. Commit 877efa2
  ("row restored READY for re-dispatch") did NOT clear the note marker, so the
  restore was ineffective - SURVEY-A has no fragment in HEAD and no RESULT.
- Preserved (operator): the at-risk untracked D.json - it would be wiped by any
  slot-0 re-fork (the SURVEY-A archive-gap class) - is committed to
  `refs/wip/SOLVER-SURVEY-D-fragment` = 1470e72, worktree file left
  untracked/unstaged. Recover with
  `git show 1470e72:loop/solver_coverage/fragments/D.json`.
- What a human should do:
  1. Fix `loop/overnight.py:222-226` so a landing REQUIRES a committed worker
     diff (`head != base`); same item as the 18:38Z + 19:33Z escalations - now
     the 7th strike. Until fixed, every survey / skipped-commit worker
     false-lands its row and self-blocks via the LANDED_RE trap.
  2. Land SOLVER-SURVEY-D's D.json (from `refs/wip/SOLVER-SURVEY-D-fragment`
     or the slot-0 wt) + its RESULT, then correct the false `LANDED 906dc59`
     note.
  3. Clear the false `LANDED cc38b4f` marker from SOLVER-SURVEY-A's note so
     `dispatch_ready` re-dispatches it (no fragment exists anywhere).
  4. Do NOT flip MONO-6-SWEPT-BOOLEANS / SOLVER-CHECKER until their real deps
     are landed.
- Start from: `loop/overnight.py:222-226`;
  `git show refs/wip/SOLVER-SURVEY-D-fragment`;
  `loop/slots/0/wt/loop/solver_coverage/fragments/D.json`;
  `grep -n 'SOLVER-SURVEY-A' loop/PACKETS.jsonl`.

## 2026-09-10 21:57 UTC - LOOP-WIDE DISPATCH STALL (blocking): integration HEAD tracks a root RESULT.json; new_slot stages its deletion, run_packet refuses the dirty tree

- What: `git ls-tree HEAD` at integration/kernel-bg 436e734 shows root harness
  artifacts tracked: `RESULT.json` (blob bcbd652 = the SOLVER-SURVEY-D result,
  committed by d0f708a "survey(SOLVER-SURVEY-D): ... orchestrator commit
  (skipped-commit-step)"), plus `CONTEXT.md` and `PACKET.md`. Every `new_slot`
  forks from this HEAD, so every new slot inherits the tracked `RESULT.json`.
  `new_slot.py:201-204`'s stale-root-artifact guard then runs `git rm -fq
  RESULT.json`, which leaves the worktree with a STAGED DELETION. `run_packet.py`
  (dirty filter at run_packet.py:306) excludes only PACKET.md/CONTEXT.md/
  worker.*, NOT RESULT.json, so it refuses: "slot N has 1 uncommitted change(s)
  from an earlier run." dispatch_ready's sequence (reset-only -> new_slot ->
  run_packet, dispatch_ready.py:223/225/231) can therefore never complete a
  dispatch.
- Evidence: heartbeat log 2026-09-10 17:46:56 local - `MONO-6-SWEPT-BOOLEANS:
  run_packet FAILED` (slot 0 dirty) and `SOLVER-SURVEY-A: new_slot FAILED`
  (warm build). Reproduced by hand this cycle: after `python loop/new_slot.py
  --slot 1 --branch packet/SOLVER-SURVEY-A --no-warm` (which printed "removed
  stale RESULT.json inherited from the fork base"), `python loop/run_packet.py
  --slot 1 --packet loop/packets/SOLVER-SURVEY-A.md` refused with the same
  message. `git status` in slots 0 and 1 was `D  RESULT.json`.
- Effect: BOTH now-dispatchable rows (MONO-6-SWEPT-BOOLEANS, SOLVER-SURVEY-A)
  are undispatchable and every future fork from HEAD is poisoned. The loop is
  fully stalled; this is why dispatch_ready reports 2 candidates that never
  land.
- Fix (orchestrator/owner - a repo-file edit, outside the operator's 3-file
  limit): remove the root artifacts from integration/kernel-bg and commit, e.g.
  `git -C C:\Users\stefa\look rm RESULT.json CONTEXT.md PACKET.md` then commit.
  The filed copy survives at `loop/results/SOLVER-SURVEY-D.json` (verified
  present). Then the heartbeat's next cycle dispatches MONO-6 + SURVEY-A.
- Machinery alternative: make `run_packet.py`'s dirty filter ignore a staged
  deletion of a root harness artifact, or have `new_slot` remove the stale
  artifact in a way that leaves the index clean.
- Start from: `git -C C:\Users\stefa\look ls-tree HEAD --name-only`;
  `loop/dispatch_heartbeat.log` (tail); `loop/new_slot.py:193-204`;
  `loop/run_packet.py:306`.

## 2026-09-10 21:57 UTC - dispatch_ready.py warms class:survey slots (should pass --no-warm)

- What: `new_slot.py` documents `--no-warm` for `class: survey` ("a survey slot
  needs no target/: the worker never runs cargo ... costs ~5.6 min and ~1-2 GB").
  `dispatch_ready.py:225` calls `new_slot.py --slot N --branch X` unconditionally,
  so survey slots are warmed. SOLVER-SURVEY-A's warm build crashed
  `cargo check --workspace --all-targets exit 101 (0xc0000409
  STATUS_STACK_BUFFER_OVERRUN)` under 2.4 GB free RAM.
- Fix: `dispatch_ready.py` should read the row's `class` and pass `--no-warm`
  when it is `survey`.
- Start from: `loop/dispatch_ready.py:225`; `loop/new_slot.py:80-83,216-226`.

## 2026-09-10 21:57 UTC - low RAM (2.4 GB free) is the 0xc0000409 zone

- 15.7 GB total, 2.4 GB free at 21:5xZ with NO cargo/rustc running. Baseline
  consumers: 2x opencode, chrome x2, Discord, Dropbox, Code, claude, MsMpEng,
  msedgewebview2. `janitor.py ram` frees nothing (no opencode-parented
  rust-analyzer exists). Any cold `cargo check --workspace` is a 4-8 GB spike.
  Recommend the owner free RAM (close chrome/Discord/Code) before re-dispatching
  MONO-6, or lower `--max-workers`.
- Start from: `python loop/janitor.py status`; `python loop/janitor.py ram`.

- Carried (unchanged): FRAME-REVOLVE F1 non_z_axis pin amendment
  (truck123d/tests/ttc_lathe_spline.rs:255); duplicate supervisors + lagging
  cargoq restart guard; slot-4/7 wt RESULT residue; TOR-C flip-or-pin;
  heartbeat slot-liveness duplicate-dispatch bug; MONO-row registry schema gap;
  overnight.py:222-226 false-landing root cause.

## 2026-09-10 22:23 UTC - tracked root RESULT.json STILL blocks the heartbeat dispatch path (21:57Z fix NOT applied)

- What: HEAD 059c588 still tracks root `RESULT.json`, `CONTEXT.md`, `PACKET.md`
  (`git ls-tree HEAD`). The session-58 handoff (item 5) says "the operator fixed
  a loop-wide dispatch stall mid-session (7591ed2: tracked root RESULT.json
  poisoned new_slot forks)" - but only the slot worktrees were cleaned; the
  tracked artifact remains, so `new_slot` re-stages its deletion on every fork
  and `run_packet` refuses. The 18:14:53 heartbeat log still shows
  `SOLVER-SURVEY-A: run_packet FAILED - slot 1 has 1 uncommitted change(s)`.
- Operator workaround applied this cycle (documented step-3c, NOT a repo edit):
  `run_packet --slot 1 --reset-only` then `run_packet --slot 1 --packet
  loop/packets/SOLVER-SURVEY-A.md`, skipping new_slot - SURVEY-A now RUNNING
  (pid 24416). This does NOT fix the dispatcher path; the NEXT heartbeat
  dispatch (e.g. SOLVER-CHECKER after SURVEY-A lands, MONO-7) will fail the same
  way.
- Fix (orchestrator/owner - outside the operator's 3-file limit):
  `git -C C:\Users\stefa\look rm RESULT.json CONTEXT.md PACKET.md` then commit.
  Filed copy survives at loop/results/SOLVER-SURVEY-D.json. Alternatively patch
  `new_slot.py:201-207` to not `git rm` tracked artifacts, or add RESULT.json to
  `run_packet.py:306`'s dirty-filter ignore list.
- Start from: `git -C C:\Users\stefa\look ls-tree HEAD --name-only`;
  `loop/dispatch_heartbeat.log` (tail); `loop/new_slot.py:193-207`;
  `loop/run_packet.py:301-312`.

## 2026-09-10 ~23:00 UTC - RESOLVED: tracked root artifacts removed (orchestrator)

- The 21:57Z/22:23Z dispatch-stall escalation is FIXED by orchestrator commit
  b60c168: root RESULT.json/CONTEXT.md/PACKET.md untracked (git rm --cached,
  local copies kept) + gitignore entries for all three so the class cannot
  recur. Filed copy loop/results/SOLVER-SURVEY-D.json confirmed present
  before removal.
- Verified after the commit: dispatch_ready --dry-run --max-workers=4 reports
  'dispatched 0; workers now ~2/4' with no dirty-artifact refusal - the
  heartbeat's next_slot path should fork cleanly. The new_slot.py guard
  (lines 201-207) remains in place as belt-and-braces for QUESTION.md.
- Workers unaffected: slot 0 MONO-6 and slot 1 SURVEY-A were running on slot
  branches at the time; no rebase required (fork bases unchanged).

## 2026-09-10 22:47 UTC - 8th FALSE LANDING: SOLVER-SURVEY-A (base merged; A.json uncommitted; row marker self-blocks)

- What: HEAD 6233aff "loop: SOLVER-SURVEY-A row LANDED (overnight)" changed
  ONLY `loop/PACKETS.jsonl` (1 insertion) - it flipped the row over no content.
  The row (still status READY) now carries a false `LANDED 7591ed2` note;
  7591ed2 is the operator's 21:57Z dispatch-stall commit, not worker work. The
  real deliverable `loop/solver_coverage/fragments/A.json` (579,713 b, RESULT
  status DONE, 279 rules) exists ONLY as an untracked file in
  `loop/slots/1/wt`; `git ls-files loop/solver_coverage/fragments` =
  B/C/D.json only and `git log --all -- .../A.json` is EMPTY. Same
  `overnight.py:222-226` root cause as the 18:38Z/19:33Z/19:54Z cluster - now
  the 8th strike.
- Why it matters now: `dispatch_ready`'s LANDED_RE skips the row (note matches
  `landed [0-9a-f]{7,}`), so SOLVER-SURVEY-A never re-dispatches while its
  fragment is missing. **SOLVER-CHECKER (`depends_on` SURVEY-A/B/C/D) is
  therefore NOT dispatchable**: A is the one unlanded dep. `gen_packet --check`
  + `packet_lint` on SOLVER-CHECKER are BOTH green - the dependency gate is the
  only blocker. Do NOT flip SOLVER-CHECKER.
- Preserved (operator): A.json committed to
  `refs/wip/SOLVER-SURVEY-A-fragment` = 9ed8d16 (17,723 lines) via a
  temp-index commit-tree; the slot-1 worktree file left untracked/unstaged.
  Recover with `git show 9ed8d16:loop/solver_coverage/fragments/A.json`.
- What a human should do:
  1. Land A.json from `refs/wip/SOLVER-SURVEY-A-fragment`
     (`python loop/scripts/validate_survey.py` it, then commit AS DELIVERED per
     the skipped-commit-step protocol) and clear the false `LANDED 7591ed2`
     marker from the row note so the row reads DONE truthfully.
  2. Fix `loop/overnight.py:222-226` so a landing REQUIRES a committed worker
     diff (`head != base` / a no-op merge = NOT LANDED). Same item as the
     18:38Z + 19:33Z + 19:54Z escalations - now the 8th strike; every
     skipped-commit survey/worker self-blocks via the LANDED_RE trap until it
     is fixed.
  3. Only after A.json is landed: flip SOLVER-CHECKER READY (all four fragments
     then truly present in HEAD).
- Start from: `loop/overnight.py:222-226`;
  `git show refs/wip/SOLVER-SURVEY-A-fragment`; `loop/slots/1/wt/RESULT.json`;
  `grep -n 'SOLVER-SURVEY-A' loop/PACKETS.jsonl`.

## 2026-09-10 23:17 UTC - UPDATE: SOLVER-SURVEY-A RESOLVED; 9th false landing (MONO-6, self-corrected); SOLVER-CHECKER now unblocked

- **RESOLVED (by the overnight driver, not the operator): SOLVER-SURVEY-A.**
  A.json (579,713 b, 279 rules) is now tracked in HEAD; landed via merge
  bfa63f7, ledger row + DONE flip 7af2bad. The 8th-strike blocker is gone and
  the 22:47Z escalation's items 1 (land A.json) is done. The operator did NOT
  have to act; it observed the driver do it mid-cycle.
- **9th strike of the overnight.py:222-226 no-op-merge class: MONO-6-SWEPT-BOOLEANS.**
  At 19:14:25 local the driver committed cb2e3e1 "MONO-6 row LANDED" changing
  ONLY loop/PACKETS.jsonl, with the note `LANDED 436e734` - 436e734 is the
  packet branch BASE, not worker work (the worker's +1155-line
  truck123d/src/bd_bridge.rs change was uncommitted in slot 0). Unlike the
  survey strikes, the driver then SELF-CORRECTED: at 19:16:44 it committed the
  deliverable as aa18e32 ("as delivered, skipped-commit-step") and at ~19:17 it
  landed it for real (HEAD bb15fa1; `git grep -c contact_cover HEAD --
  truck123d/src/bd_bridge.rs` = 3). So this strike cost no work this time, but
  the root cause is unchanged and the next strike may not self-correct.
- **Root cause still open: `loop/overnight.py:222-226`** - a landing must
  REQUIRE a committed worker diff (`head != base`; a no-op merge = NOT LANDED).
  Now NINE strikes. This is the single highest-value human fix in the loop.
- **SOLVER-CHECKER is now unblocked-by-deps** (SOLVER-SURVEY-A/B/C/D.json all
  tracked in HEAD). The 22:47Z escalation's item 3 condition is met: it is now
  safe to flip SOLVER-CHECKER READY. The operator deliberately did NOT flip it
  (that flip was reserved for a human) and notes one dispatch caveat: its
  `crates: []` will fail the CRATES_NONEMPTY lint, so give it a crate (or
  exempt the checker class) when flipping.
- Start from: `loop/overnight.py:222-226`; `git show cb2e3e1`; `git show
  bb15fa1`; `grep -n 'SOLVER-CHECKER' loop/PACKETS.jsonl`.

## 2026-09-11 00:05 UTC - MONO-7 landed with TWO unaddressed findings; the driver's scoped check is the wrong crate; no-op-merge class now 11 strikes

- **MONO-7-ROW-ASSEMBLY is LANDED (merge `adc6151`, row `8ffdf72`) but is NOT
  green - do not treat it as done.** The operator preserved the worker's
  skipped-commit work (`20b808f`, ref `refs/wip/MONO-7-as-delivered`) and the
  driver merged it. Two amendments are required before the merged HEAD can be
  trusted:
  1. **D1 - write_allow ripple.** `truck123d/src/binding.rs:2011` constructs a
     `PartSpec { ... }` literal (in `#[cfg(test)] mod tests`) and is in
     read_allow, not write_allow. Adding the `label`/`color` fields breaks it;
     the worker added `label: None, color: None` (2 lines). V1 will report
     SCOPE_VIOLATION. Fix: widen MONO-7's write_allow to include
     `truck123d/src/binding.rs` (the BG-NUM-001-FILLET ripple precedent), or
     accept the as-delivered amendment.
  2. **D2 - determinism test regression.** The new `timing` columns
     (`construct_ms`/`facts_ms`/`mesh_ms`) make
     `truck123d/tests/ttc_lathe_spline.rs:392` (`assert first == second`) and
     `:464` (`assert_eq!(again["facts"], *facts, ...)`) fail, because both
     compare the whole `bd_facts` JSON string. The packet's judgement 4 states
     timing is gated only `>= 0.0`, so the correct amendment is to pop `timing`
     before comparing (or compare the unchanged keys). NOTE: the operator did
     NOT run the test (budget) - confirm the failure, then amend the test (a
     test file, not kernel code).
  - Start from: `git show 20b808f`; `git -C loop/slots/0/wt show` is gone after
    recycle, use `refs/wip/MONO-7-as-delivered`;
    `grep -n 'first == second\|again\["facts"\]' truck123d/tests/ttc_lathe_spline.rs`;
    `grep -n 'timing' truck123d/src/bd_bridge.rs`.
- **The driver's scoped check is the wrong crate - a NEW root cause for
  false greens.** `overnight.py:packet_tests_and_crates` (lines 84-101)
  derives test pairs ONLY from `vendor/truck/<crate>/tests/<stem>.rs` and
  crates ONLY from write paths starting `vendor/truck/`. Every `truck123d` /
  loop / corpus packet therefore falls back to `crates=["truck-certified"]`
  and `pairs=[]` - `scoped_check` runs `cargo check -p truck-certified` and
  ZERO tests, so it is vacuously green and the packet lands with any
  regression. This is how MONO-7's D2 reached integration. Fix: derive crates
  from the row's write paths generally (root-crate `truck123d`), and add the
  packet's `tests_required` / `write_allow` test files as test pairs.
  - Start from: `loop/overnight.py:84-118`; `grep -n 'packet_tests_and_crates'
    loop/overnight.py`.
- **overnight.py:222-226 no-op-merge class is now 11 strikes (10 + 11 this
  cycle).** MONO-7 and SOLVER-CHECKER both finished DONE with uncommitted work;
  the driver would have no-op-merged the base and marked both LANDED while
  losing the work. The operator pre-empted it by committing both as delivered;
  the driver then merged the real commits. The fix remains: a landing must
  REQUIRE `head != base` (a no-op merge = NOT LANDED). Same item as the
  18:38Z / 19:33Z / 19:54Z / 23:17Z escalations.
  - Start from: `loop/overnight.py:222-226`; `git show adc6151`; `git show
    01fc99c`; `git show 20b808f`; `git show 46ba171`.
- **SOLVER-CHECKER landed** (merge `01fc99c`, row `e57f36a`; preserved from
  skipped-commit at `46ba171`, ref `refs/wip/SOLVER-CHECKER-as-delivered`) -
  no amendment needed; informational.
- **FYI - operator flipped MONO-8-SWEPT-ADMISSION-WIRING and
  AUTHOR-EXT-FILLET-HALO BLOCKED->READY this cycle** (`45ed5d5`) under the
  documented step-4 rule: dep MONO-7-ROW-ASSEMBLY is landed, both packets are
  authored (preflight green, lint clean). The prior 00:05Z operator had
  declined while the packets were unauthored; they were authored at `2a1b164`
  / `a113b39`. Not a semantic rewrite; revertible with a one-line status flip
  if the orchestrator intended these pinned. MONO-8 dispatched to slot 1;
  AUTHOR-EXT is deferred on the door.py write set.
- **FYI - DOOR-PARTIAL-ARC-FLIP anchor quoting fixed** (`45ed5d5`): the
  escaped-double-quote cmd form is not unescaped by `gen_packet.parse_anchors`
  and fails under `bash -lc` when the pattern contains an apostrophe. Watch
  for the same pattern in future authored packets (only this one had it).
