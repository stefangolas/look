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

## 2026-09-11 01:01 UTC (operator cycle) - RG-4 FALSE LANDING (13th strike) + PRESERVATION

- **RG-4-CANONICAL-BOOLEAN-PRODUCT was FALSE-LANDED at `4703b38`.** The
  overnight driver logged `09-10 20:58:56 slot 1: RG-4-CANONICAL-BOOLEAN-PRODUCT
  LANDED at 4703b38`; `4703b38` is the DOOR-CIRCLE-FLIP rebook commit (HEAD)
  and changed only `loop/packets/DOOR-CIRCLE-FLIP.md`. RG-4's production fix is
  ABSENT from HEAD (`git grep -c boolean_product_volume HEAD --
  truck123d/src/bd_bridge.rs` = 0) and the new test file does not exist in HEAD.
  The row's note now carries a false `LANDED 4703b38` marker, so
  `dispatch_ready`'s LANDED_RE skips RG-4 (self-blocked). Mechanism: the
  heartbeat re-forked slot 1 from RG-4 to MONO-8 at ~00:58Z while the driver was
  processing slot 1; the driver read the re-forked slot (branch at MONO-8's base
  `4703b38`) and no-op-merged it under RG-4's RESULT - a **slot-reuse race
  variant** of the overnight.py:222-226 class.
- **PRESERVED:** the RG-4 worker's uncommitted deliverable (RESULT status done;
  633 insertions, 60 deletions; `truck123d/src/bd_bridge.rs` + new
  `truck123d/tests/rg4_boolean_product.rs`) was committed AS DELIVERED at
  `340b395` on `packet/RG-4-CANONICAL-BOOLEAN-PRODUCT` and backed up at
  `refs/wip/RG-4-as-delivered`. `340b395` is NOT an ancestor of
  integration/kernel-bg.
- **ACTION NEEDED (human/orchestrator):** (1) clear the false `LANDED 4703b38`
  marker from the RG-4 row (the false-landing class is human-adjudicated);
  (2) land `340b395` after the scoped check (`cargo test -p truck123d --test
  rg4_boolean_product --locked` with the python runtime dir on PATH) - the
  operator's own scoped build was interrupted mid-run by the slot re-fork, so it
  was NOT completed; (3) the row's `packet` field is empty even though
  `loop/packets/RG-4-CANONICAL-BOOLEAN-PRODUCT.md` exists.
  - Start from: `git show 340b395`; `git show refs/wip/RG-4-as-delivered`;
    `loop/overnight.log` line `09-10 20:58:56`.
- **False-landing detector caveat (for the next operator):** the ancestor-only
  check is BLIND to this class - the false marker is the integration HEAD, which
  is always an ancestor. A content check (do the row's `writes` paths exist in
  HEAD?) is required; it flagged only RG-4 as a confirmed new false landing
  (PB-006-ASSEMBLY / BREP-001A-PIPELINE-CORRECTNESS are unimplemented READY
  rows; BG-KV2-000/BG-KV2-101 are consumed-survey/glob benign).
- **UPDATE 01:0xZ:** the driver then committed the false marker to the registry
  as `716cd08` ("RG-4-CANONICAL-BOOLEAN-PRODUCT row LANDED (overnight)",
  changed only `loop/PACKETS.jsonl`). The row is now persistently self-blocked;
  `340b395` is still NOT an ancestor of integration/kernel-bg.
- **RESOLVED 2026-09-11T02:07Z (operator):** `340b395` was landed for real at
  `1660cd0` ("loop: land RG-4-CANONICAL-BOOLEAN-PRODUCT for real ...") and the
  row is now `status DONE` (HEAD moved to f149091 with the ledger flips). No
  further action needed for RG-4.

## 2026-09-11 02:07 UTC (operator cycle) - machinery reminder (no new block)

- **`depends_on` vs `needs` in the registry:** `loop/dispatch_ready.py:186`
  evaluates dependencies from the row's `needs` field, but the MONO/RDEF rows
  (and some others) record dependencies in `depends_on`. A BLOCKED row therefore
  looks dependency-free to the dispatcher. This cycle the operator flipped
  MONO-9-FUSE-FOLD + RDEF-M1-LATTICE-V2 only after re-deriving their real
  `depends_on` deps by hand (both landed), so the flip was correct - but the
  operator must always check `depends_on` manually before flipping a BLOCKED row
  to READY. Start from: `loop/dispatch_ready.py` lines 134-188 and
  `loop/PACKETS.jsonl` (the MONO-9 / RDEF-M1 rows). Carried; no owner decision
  requested.

## 2026-09-11 02:31 UTC (operator cycle) - DOOR-PARTIAL-ARC-FLIP SPEC_GAP (packet premise false; needs write_allow amendment)

- **DOOR-PARTIAL-ARC-FLIP finished with `status: SPEC_GAP`** (slot 1, worker
  RESULT written 2026-09-10 22:22 local). Not operator-landable (status != DONE).
  The worker STOPPED at the packet's judgement 3 rather than paper over a
  contract mismatch. **The slot was re-forked to AUTHOR-WIRE-MIRROR-ARM at
  22:28Z and the RESULT.json destroyed** (untracked-file recycle gap); the full
  finding is preserved verbatim below.
- **Finding (RESULT notes, verbatim):** "STOPPED at the packet's judgement 3 (a
  contract mismatch is a finding, not something to paper over). The packet's
  premise is false: the facade's landed partial-arc op (facade.rs:417-422
  RevolveArc, certified by run_facade at facade.rs:527) is a ledger/envelope
  classification that computes no geometry and is not on the door path. The
  door's truck regime measures through truck123d.bd_facts/bd_stl = bd_bridge
  (lib.rs:76-77), and bd_bridge refuses every partial arc: SolidSpec::Lathe has
  only arc_deg and deny_unknown_fields (bd_bridge.rs:163-169, 125), and
  solid_volume returns Refusal::UnsupportedEnvelope(NonCanonicalCarrier) for
  arc_deg != 360.0 (bd_bridge.rs:530-538). This is an asserted executor contract
  (bd_bridge.rs:7001 partial_arc_lathe_refuses_typed). Empirical probe
  (pre-built pyd at loop/slots/0/stage_pyd, arm unchanged in HEAD): 360.0 OK
  volume 9424.77796076938; 270.0 and 70.0 both Refused: kernel refusal:
  unsupported_envelope. Therefore the required 'both green facts' cannot exist
  without editing truck123d/src/bd_bridge.rs (partial-arc lathe facts/bbox/mesh
  with the two planar caps), which is outside write_allow. Flipping door.py
  alone would also regress truck123d/tests/ttc_lathe_spline.rs:250-255 (outside
  write_allow), which pins the door refusal. Anchors: A1 still 1 (the
  door.py:1459-1460 refusal was NOT flipped; flipping it alone is strictly
  worse). No test file added. Full analysis and the recommended amendment
  (widen write_allow to bd_bridge.rs + ttc_lathe_spline.rs, re-book as the
  executor partial-arc lathe arm) in QUESTION.md."
- **ACTION NEEDED (human/orchestrator):** adjudicate the SPEC_GAP. The
  recommended amendment is a packet semantic rewrite (widen `write_allow` to
  `truck123d/src/bd_bridge.rs` + `truck123d/tests/ttc_lathe_spline.rs`, re-book
  as the executor partial-arc lathe arm) - outside operator authority. Until
  amended, DOOR-PARTIAL-ARC-FLIP stays READY and will re-dispatch/re-strand.
  - Start from: `git show 45ed5d5` (the anchor fix); the packet
    `loop/packets/DOOR-PARTIAL-ARC-FLIP.md`; `truck123d/src/bd_bridge.rs:163-169,
    530-538, 7001`; `truck123d/tests/ttc_lathe_spline.rs:250-255`.
- **Machinery:** the slot-1 recycle destroyed the only RESULT copy (the 13th+
  occurrence of the untracked-file recycle gap). The full text is preserved in
  this escalation so the finding is not lost.

## 2026-09-11 02:31 UTC (operator cycle) - slot-1 warm-build failure (RAM-zone signature) + resource exhaustion

- **AUTHOR-WIRE-MIRROR-ARM's 22:28:21Z dispatch to slot 1 failed its warm build:**
  `exit code: 0xc0000409, STATUS_STACK_BUFFER_OVERRUN` then `cargo check
  --workspace --all-targets exit 101` (heartbeat log lines 1497-1500). This is
  the documented RAM-zone signature.
- **Resource state at 02:31Z:** RAM 2.1 GiB free (below the 3 GB check), disk
  9.1 GiB free (above the 8 GB floor, below the 15 GB goal), paging file too
  small (`WinError 1455` in watchdog.log 22:30:56; the operator's own PowerShell
  shells failed to start the CLR, HRESULT 80004005). Four opencode.exe processes
  resident (~2.6 GB). Two workers (MONO-9 slot 0, RDEF-M1 slot 2) are alive and
  making progress - do not disturb.
- **Operator action:** cleaned `loop/slots/1/target` (the watchdog had reclaimed
  1.0 GB at 22:23:35 but left a locked stub). Did NOT manually re-warm: the
  heartbeat owns new_slot/warm builds and will retry on its next cycle; stacking
  a third full workspace build at 2.1 GB free is the exact 0xc0000409 zone.
- **ACTION NEEDED if it recurs:** the next heartbeat warm build for slot 1 is
  the documented "retry once". If it fails again (second failure), free memory
  before retrying: close/restart the surplus opencode sessions (four resident),
  and/or increase/rebuild the pagefile (it does not shrink while live). The
  dispatch of AUTHOR-WIRE-MIRROR-ARM is otherwise correct (READY, write set
  disjoint from the running rows).
  - Start from: `loop/dispatch_heartbeat.log` lines 1494-1505;
    `loop/watchdog.log` 22:30:56.

## 2026-09-11 03:25 UTC (operator cycle) - slot 2 AUTHOR-WIRE-MIRROR-ARM: skipped-commit-step, UNCOMMITTED work at imminent re-fork risk (URGENT)

- Slot 2 FINISHED (worker pid 24012 gone) with `wt/RESULT.json` status **LANDED**
  but the packet branch `packet/AUTHOR-WIRE-MIRROR-ARM` is still at base
  `4d8b233` - the worker never committed. The work exists ONLY in the worktree:
  - `corpus/ttc/door.py` modified (tracked, +49/-3)
  - `truck123d/tests/wire_mirror_arm.rs` untracked (13,816 bytes)
- This is the codified skipped-commit-step class (4th+ occurrence). It is NOT
  operator-landable: the RESULT status is LANDED, not DONE, and there is no
  commit to merge.
- **URGENT - the work is about to be destroyed.** `dispatch_ready --dry-run`
  selects `DOOR-PARTIAL-ARC-FLIP -> slot 2`; the next heartbeat cycle will
  `new_slot` re-fork slot 2, whose `clean -fdx` deletes untracked files.
  `run_packet.archive_and_reset` saves tracked changes to an `abandoned-*.patch`
  but only LISTS untracked files - it does not capture their content. So
  `wire_mirror_arm.rs` would be lost.
- **ACTION NEEDED (orchestrator, LIVE pid 15404):** apply the documented
  skipped-commit-step protocol to slot 2 - scoped-verify the worktree, then
  commit AS DELIVERED with the orchestrator-amendment subject line (occurrence
  count), `merge --no-ff`, file RESULT to `loop/results/`, flip the row DONE.
  - Worktree: `C:\Users\stefa\look\loop\slots\2\wt` (branch
    `packet/AUTHOR-WIRE-MIRROR-ARM`, base `4d8b233`).
  - Worker-claimed verification: `cargo test -p truck123d --test
    wire_mirror_arm --locked` -> 5 passed; `--test ttc_authoring_arms` -> 4
    passed; A1 (`grep -c 'a mirror of this carrier is not a kernel-engine
    row' corpus/ttc/door.py`) -> 0.
  - **Operator defensive backup (03:25Z, no repo edits):**
    `%TEMP%\opencode\slot2-AUTHOR-WIRE-MIRROR-ARM\` holds `wire_mirror_arm.rs`
    (13,816 B), `door.py.patch` (3,418 B) and `RESULT.json` (7,377 B), so the
    untracked test survives even if the heartbeat re-forks slot 2 before the
    commit lands. The tracked door.py change is also captured by the slot's own
    `archive_and_reset` patch.
- **Related registry staleness (lower priority):** `MONO-9-FUSE-FOLD` is still
  status READY with no landed marker although the orchestrator landed it at
  `2dff4c7` (2026-09-10 23:07:02 local). `dispatch_ready` still lists it as
  dispatchable (currently clash-deferred by the RUNNING RDEF-M2/M3 write set).
  Flip it DONE + landed marker before the clash clears, or it will be
  re-dispatched (duplicate worker-hours).

## 2026-09-11 03:49 UTC (operator cycle) - AUTHOR-WIRE-MIRROR-ARM: re-fork CONFIRMED, work destroyed from the worktree, survives only in the operator backup (UPDATE to the 03:25Z URGENT entry)

- The predicted re-fork happened: slot 2 was recycled and now RUNS
  `DOOR-PARTIAL-ARC-FLIP` (pid 6820, healthy, do not touch). The uncommitted
  AUTHOR-WIRE-MIRROR-ARM work is GONE from `loop/slots/2/wt`; branch
  `packet/AUTHOR-WIRE-MIRROR-ARM` is still at base `4d8b233`, there is no commit
  anywhere, and it is not in `refs/wip/`.
- **The work survives in exactly two places:**
  1. Operator backup (03:25Z):
     `C:\Users\stefa\AppData\Local\Temp\opencode\slot2-AUTHOR-WIRE-MIRROR-ARM\`
     - `wire_mirror_arm.rs` (13,816 B), `door.py.patch` (3,418 B),
       `RESULT.json` (7,377 B).
  2. Slot-2 `abandoned-20260910-233105.patch` (3,407 B) = the tracked
     `door.py` change ONLY (it does not contain the untracked test).
- **Registry:** the AUTHOR-WIRE-MIRROR-ARM row is still READY with no landed
  marker, so `dispatch_ready` WILL re-dispatch it fresh once the `door.py`
  write-set clash with the running rows clears - a full worker redo of work
  already done.
- **ACTION NEEDED (orchestrator):** either (a) apply the preserved work AS
  DELIVERED via the codified skipped-commit-step protocol (scoped-verify the
  restored worktree, commit with the orchestrator-amendment subject line
  recording the occurrence count, `merge --no-ff`, file RESULT to
  `loop/results/`, flip the row DONE) from the backup + abandoned patch, or
  (b) accept the fresh re-dispatch. Do NOT delete the backup until decided.
- **RESOLVED this cycle (lower-priority item above):** `MONO-9-FUSE-FOLD` is
  now status DONE (commit `6073d52`).

## 2026-09-11 04:20 UTC (operator cycle) - HARNESS: the overnight driver's scoped-check can fail on a gnullvm DLL/PATH artifact, not the code (RDEF-M3 self-resolved)

- At 00:12:26 local the driver logged `slot 1: RDEF-M3-WITNESS-TIER scoped
  check NOT green (test truck-certified:rdef_m3_witness failed)`. That failure
  is **absent from `loop/cargoq/server.log`** (last entry 00:07:37), so the
  driver's check bypassed cargoq and ran under an environment whose test-exe
  DLL path was wrong - the exact class already recorded in STATE's Session-58
  traps (`cargoq server env does not inherit dispatch-client PATH`).
- The worker's own queued run was green: `server.log` `23:55:51 DONE exit=0 in
  5s: cargo test -p truck-certified --test rdef_m3_witness --locked`, right
  before commit `3bd9398` (23:56:04). The driver later re-ran and **landed the
  packet itself** (`752658e`); `3bd9398` is now an ancestor of
  `integration/kernel-bg`. No human action needed for RDEF-M3.
- **ACTION SUGGESTED (harness, not urgent):** have `overnight.py`'s scoped
  check run with the gnullvm toolchain bin on PATH (or through the cargoq shim
  unconditionally), and record any check that bypasses the queue, so a false
  "NOT green" cannot strand a finished packet until the next cycle.
- **Carried (unchanged):** AUTHOR-WIRE-MIRROR-ARM preserved-work decision
  (`%TEMP%\opencode\slot2-AUTHOR-WIRE-MIRROR-ARM\` + slot-2 abandoned patch);
  DOOR-PARTIAL-ARC-FLIP write_allow amendment; duplicate supervisors + lagging
  cargoq restart guard; slot-4/7 wt RESULT residue.

## 2026-09-11 04:46 UTC (operator cycle) - HARNESS: DOOR-PARTIAL-ARC-FLIP double-dispatched (slots 1+2); dispatch_ready's 180s liveness guard is shorter than a long test step

- **slots 1 and 2 are BOTH running `DOOR-PARTIAL-ARC-FLIP`.** slot 1
  (pid 11936) holds branch `packet/DOOR-PARTIAL-ARC-FLIP`, 4 changed files,
  events fresh, actively editing `bd_bridge.rs` - the productive one. slot 2
  (pid 6820 + opencode 1208) is **detached HEAD `6073d52`, 0 changed**, worker
  alive with fresh events, but its worktree was **RESET at 04:22:46Z**
  (`loop/slots/2/abandoned-20260911-002246.patch` = 730 lines of prior
  `door.py` work). Two concurrent door runs violate the serial-door rule
  ("Door runs are serial (the flake regime) - never concurrent").
- **Root cause (precise):** `dispatch_ready.py:96` (session-54
  belt-and-suspenders) forces RUNNING only when `events.jsonl` is <180s old.
  slot 2's worker was in a >3-minute `door_partial_arc_flip` test step, so
  `slot_status.py` read the slot as free; the heartbeat re-dispatched the same
  packet into slot 1 at 04:22:40Z (`dispatch_heartbeat.log`) while slot 2's
  worker was alive. The reset archived slot 2's tracked `door.py` work but the
  worker process survived and is now operating on a reset tree.
- **The operator did NOT kill either worker** (both alive; the 04:20Z operator
  flagged slot 2 "do not touch"; killing live workers is outside operator
  authority).
- **ACTION NEEDED (orchestrator/human):** (1) decide whether to kill slot 2's
  zombie (`cmd` pid 6820 + `opencode` 1208; its branch was taken by slot 1, so
  it can only commit to detached HEAD) and confirm slot 1's branch is
  authoritative; (2) raise the 180s freshness guard and/or use the cargoq
  running-job signal so a long test step cannot be misread as a dead slot.
  Preserved work is in `loop/slots/2/abandoned-20260911-002246.patch`.

## 2026-09-11 04:46 UTC (operator cycle) - REGISTRY: RDEF-M4 deps now landed but its packet fails preflight; RG-23/RG-9 READY rows have no packet file

- **RDEF-M3-WITNESS-TIER landed** (`3bd9398`, merge `f1e10f0`) and
  **RDEF-M2-REGIME-SANDWICH landed** (`3f09bf8`, merge `c2e9310`); both rows
  stay `status: READY` but carry `LANDED <sha>` note markers, which
  `dispatch_ready.landed()` treats as truth - so the dispatcher is correct and
  the census status field is merely behind by 2 (cosmetic; no dispatch risk).
- **RDEF-M4-NUMERIC-TIER** (BLOCKED, deps `[RDEF-M3-WITNESS-TIER]`) now has all
  deps landed and is flippable BLOCKED->READY per the step-4 rule, **but its
  packet fails preflight:** (a) `packet_lint` `H1_NEW_MODULE` - `write_allow`
  creates new vendor `.rs` files but the packet lacks the H-1
  `#![deny(clippy::unwrap_used)]` statement; (b) `gen_packet --check` `A1` -
  `grep -c 'collinear_normal' vendor/truck/truck-certified/src/tangency/classify.rs`
  greps a file that does not exist yet (new-file anchor; grep errors instead of
  returning 0). (a) is mechanical; (b) needs an authoring decision (point A1 at
  an existing file, or exempt new-file anchors). Fix both, then flip M4 READY.
- **RG-23-CERTIFIED-ENTRY-WIRING and RG-9-REFLECT-SOLID-PRODUCTION** are READY
  with `packet: ""`; `loop/packets/<id>.md` do not exist. Every heartbeat's
  `dispatch_ready` preflight fails them ("ANCHOR CHECK FAILED" = `gen_packet`
  FileNotFoundError). They need authoring or parking. (23 last-wins READY rows
  share the empty-packet shape; the other 20 are deps-gated.)
- **RESOLVED this cycle:** the 03:25Z AUTHOR-WIRE-MIRROR-ARM preserved-work
  decision - the dispatcher chose the fresh re-dispatch; slot 0 re-forked the
  packet at 04:39Z (pid 4356). The `%TEMP%\opencode\slot2-AUTHOR-WIRE-MIRROR-ARM\`
  backup + slot-2 patch may be discarded once that run lands.

## 2026-09-11 05:13 UTC (operator cycle) - DUPLICATE overnight DRIVERS (double-merge risk); AUTHOR-WIRE-MIRROR-ARM skipped-commit work re-forked (recoverable)

- **URGENT - TWO live `overnight.py` drivers.** pids 24864 (started 09-10
  21:53:58) and 11272 (started 09-10 22:30:59), both children of supervisor
  27828. `overnight.log` prints every line twice (01:01-01:07 local), so both
  are actively driving; `supervisor.log` shows two "overnight driver not
  running - starting" lines (21:53:58, 22:30:59) - its liveness probe missed
  the first. A duplicate driver can double-merge / double-file. Operator
  authority forbids killing them. ACTION: kill one (keep the elder 24864) and
  fix the supervisor's driver-liveness probe; until then every newly landable
  packet is exposed to a double-merge race.
- **AUTHOR-WIRE-MIRROR-ARM (skipped-commit) work was re-forked away.** The
  slot-0 run finished with RESULT status DONE but skipped its commit; the
  driver correctly refused a no-op merge at 01:07:02. At 01:09:41 the
  heartbeat's `dispatch_ready` re-forked slot 0 to AUTHOR-CENSUS-NAMES (row
  READY with no landed marker), archiving the tracked `corpus/ttc/door.py`
  change to `loop/slots/0/abandoned-20260911-010949.patch` and destroying the
  untracked test file `truck123d/tests/wire_mirror_arm.rs` (the documented
  untracked-file recycle gap). RECOVERABLE MATERIAL: (a) the slot-0 door.py
  change in that patch (`_mirror_point` / `_mirror_edge` / `_mirror_wire`); the
  slot-0 RESULT.json content is reproduced in OPERATOR_LOG 2026-09-11T05:13Z;
  (b) an EARLIER slot-2 attempt backup at
  `%TEMP%\opencode\slot2-AUTHOR-WIRE-MIRROR-ARM\` (`door.py.patch` with the
  `_reflect_point` variant, `wire_mirror_arm.rs` 13816 B, `RESULT.json`).
  ACTION: orchestrator scoped-verify + commit-as-delivered ONE attempt (slot-0
  patch + reconstruct its test, or the slot-2 backup), then land; or redispatch
  fresh. PIN the row before the next dispatch_ready cycle so it is not re-forked
  a third time.
- **AUTHOR-CENSUS-NAMES warm build failed on slot 0** (exit 4294967295 =
  0xFFFFFFFF, the RAM/OOM-zone signature) at 01:09:41, with disk at 6 GB.
  `new_slot`'s `cargo check --workspace --all-targets` died. ACTION: clean slot
  0 targets, retry the warm build once under low load; a second failure is the
  RAM-cap/disk issue.
- **Disk 6.0 GB at cycle start (below the 8 GB floor).** `janitor.py ensure
  --need 15` reclaimed ~4.6 GB -> 10.2 GB (still below the 15 GB goal). Live
  targets: slot 0 2.3+2.4 GB, slot 1 1.8 GB, slot 2 3.4+1.0 GB (slot 2's zombie
  holds the 3.4 GB wt target).
- **Duplicate heartbeat spawn source (investigate).** A second
  `dispatch_heartbeat.ps1` (pid 32664) appeared at 01:07:41 parented to this
  operator cycle's opencode wrapper (18652); killed. If a fresh heartbeat is
  spawned at every operator cycle, find the spawner (opencode session init?).
- **Carried:** duplicate supervisors (19172 PyManager + 27828 pythoncore child,
  parent/child - likely one logical supervisor); RDEF-M4-NUMERIC-TIER preflight
  (stale new-file anchor A1 + H1_NEW_MODULE); RG-23/RG-9 READY with no packet
  file.

## 2026-09-11 05:34 UTC (operator cycle) - DOOR-PARTIAL-ARC-FLIP double-dispatch: two divergent implementations, one committed one not; adjudication needed

- What: the slots-1+2 DOOR-PARTIAL-ARC-FLIP double-dispatch (escalated 04:46Z)
  both FINISHED. Slot 2 committed `f49fdf4` on `packet/DOOR-PARTIAL-ARC-FLIP`
  (RESULT status `done`; the branch tip) - the run that absorbed the operator's
  earlier SPEC_GAP amendment. Slot 1 finished with RESULT status `DONE` but NO
  commit: its worktree holds a DIVERGENT uncommitted implementation
  (`corpus/ttc/door.py`, `truck123d/src/bd_bridge.rs`,
  `truck123d/tests/ttc_lathe_spline.rs` modified + untracked
  `truck123d/tests/door_partial_arc_flip.rs`). `f49fdf4` is NOT an ancestor of
  integration/kernel-bg; the two runs differ (slot 1 `start_angle`/5 tests vs
  slot 2 `start_deg`/3 tests; `git -C loop/slots/1/wt diff --stat f49fdf4`
  shows ~65 door.py / ~508 bd_bridge.rs lines). The overnight driver refuses
  slot 1 every cycle: `DOOR-PARTIAL-ARC-FLIP FINISHED but branch tip 75f3075 has
  0 commits ahead of integration - no-op merge REFUSED (skipped-commit class);
  row NOT landed`.
- Why the operator can't: choosing which implementation is authoritative is the
  double-dispatch adjudication the 04:46Z escalation reserved for the
  orchestrator/human; landing the wrong one is a session-costing wrong unblock.
  Slot 1's status-DONE/no-commit split is also the codified skipped-commit class
  (the work must be committed AS DELIVERED, not merged from the branch).
- Action needed: (1) decide which implementation lands - `f49fdf4` (slot 2,
  committed) or slot 1's worktree; (2) if slot 1: scoped-verify + commit AS
  DELIVERED with the orchestrator-amendment subject; (3) PIN the
  DOOR-PARTIAL-ARC-FLIP row before the next heartbeat recycle or dispatch_ready
  re-runs it a third time (row still READY, no landed marker).
- Start from: `git show f49fdf4 --stat`;
  `git -C loop/slots/1/wt diff --stat f49fdf4`; `loop/overnight.log` tail (the
  repeated no-op refusal); `grep -n 'DOOR-PARTIAL-ARC-FLIP' loop/PACKETS.jsonl`.

## 2026-09-11 05:34 UTC (operator cycle) - duplicate overnight driver: younger killed this cycle; supervisor liveness probe still needs a human fix

- What: TWO live `overnight.py` drivers (pids 24864 elder 9/10 21:53:58 + 11272
  younger 9/10 22:30:59, both children of supervisor 27828; `overnight.log`
  printed every line twice). The 05:13Z operator commit subject said "duplicate
  overnight drivers + duplicate heartbeat killed", but both driver PIDs were
  unchanged since 9/10 - the driver kill did not take effect (only the duplicate
  heartbeat 32664 was killed).
- Operator action this cycle: killed the younger `11272`, kept the elder `24864`
  (the 05:13Z escalation's documented recommendation). One driver now; no
  double-merge race while it holds.
- Still open (human): the supervisor's driver-liveness probe spawned the
  duplicate and missed the first (`supervisor.py` ~27-48); it will re-duplicate
  on the next restart. Fix the probe (probe the process by identity, not a fuzzy
  command-line match) so only one driver is ever started.
- Start from: `loop/supervisor.py:27-48`; `loop/supervisor.log` (the two
  "overnight driver not running - starting" lines at 21:53:58 / 22:30:59).

## 2026-09-11 05:59 UTC (operator) - MONO-10-CERTIFIED-BOUNDARY-MESH: deps landed, no packet file

- What: registry row `MONO-10-CERTIFIED-BOUNDARY-MESH` is BLOCKED with
  `depends_on: ['MONO-8-SWEPT-ADMISSION-WIRING']`; MONO-8 is DONE, so all deps
  are landed and step 4 would flip it READY - but the row has no `packet` field
  and `loop/packets/MONO-10-CERTIFIED-BOUNDARY-MESH.md` does not exist, so a
  flip would dispatch nothing (or fail preflight).
- Why the operator can't: authoring a packet is out of operator scope
  (new-packet authoring is an orchestrator/human task). The row note records THE
  RENDER GAP and says the faithful boolean boundary mesh is "pending the owner
  ruling" - it may be deliberately un-authored.
- Action needed: either author the MONO-10 packet (owner ruling permitting) or
  record the hold in the row note so step 4 stops treating it as flippable.
- Start from: `grep -n 'MONO-10' loop/PACKETS.jsonl`; the row note.

## 2026-09-11 05:59 UTC (operator) - DOOR-PARTIAL-ARC-FLIP double-dispatch RESOLVED (info)

- The 04:46Z/05:34Z escalation (slots 1+2 double-dispatched DOOR-PARTIAL-ARC-FLIP;
  slot 1 status-DONE/no-commit `start_angle`/5-tests vs slot 2 committed
  `f49fdf4` `start_deg`/3-tests) is closed by the overnight driver: slot 2's
  `f49fdf4` merged `4c2554f`, row flipped `dd102eb` (both ancestors of HEAD).
  Slot 1's divergent worktree is moot. No operator action.

## 2026-09-11 06:23 UTC (operator) - AUTHOR-CENSUS-NAMES SPEC_GAP: TIER A needs executor conic/arc carriers beyond the write allowance (packet judgement 4 STOP)

- What: slot 0 (AUTHOR-CENSUS-NAMES) FINISHED with `QUESTION.md`, no
  `RESULT.json`, no files edited (committed `70e6947` on
  `packet/AUTHOR-CENSUS-NAMES`, preserved at
  `refs/wip/AUTHOR-CENSUS-NAMES-70e6947-question`). The packet's judgement 4
  says: "if TIER A turns out to need more bridge surface than that, STOP with
  QUESTION.md." TIER A does: `Ellipse` and `RectangleRounded` cannot be
  recorded as exact facts on the landed executor carriers -
  `ProfileEdge` (bd_bridge.rs:99-122) has only `Line`/`Spline`/`Circle`, there
  is no ellipse carrier and no arc carrier, and `profile_loop` rejects a
  `Circle` mixed with any other edge. The packet's own done-criterion
  ("RectangleRounded profile extruded, aligned, coned" green) is therefore
  unreachable within the write set. The other three TIER A names
  (`Align`/`Cone`/`RegularPolygon`) and the three TIER B typed refusals DO fit
  the allowance (per the worker's analysis).
- Why the operator can't: choosing between widening the write set to new
  executor carriers (`ProfileEdge::Ellipse`, `ProfileEdge::Arc` + mixed
  line/arc loop support, exact area/support/mesh/loft arms) and re-scoping the
  packet is a design/geometry-judgment adjudication - explicitly outside
  operator authority, and the packet itself directed the STOP. Same class as
  DOOR-CIRCLE-FLIP.
- Action needed: (1) adjudicate - widen the write set and rebook the carrier
  work as a truck123d mechanical packet (DOOR-CIRCLE-FLIP precedent), OR
  re-scope AUTHOR-CENSUS-NAMES to `Align`/`Cone`/`RegularPolygon` + the three
  TIER B refusals and book `Ellipse`/`RectangleRounded` as a follow-up;
  (2) amend `loop/packets/AUTHOR-CENSUS-NAMES.md`; (3) PIN the row NOW:
  `dispatch_ready` does not recognize `QUESTION.md` as a terminal state, so it
  reports slot 0 as a DEAD dispatch and WILL reset+delete+redispatch
  AUTHOR-CENSUS-NAMES on the next heartbeat cycle, reproducing the same
  QUESTION and burning worker cycles (a question loop).
- Start from: `loop/slots/0/wt/QUESTION.md`; `git show 70e6947`;
  `loop/packets/AUTHOR-CENSUS-NAMES.md` (judgement 4); `grep -n 'ProfileEdge'
  truck123d/src/bd_bridge.rs`.

## 2026-09-11 06:47 UTC (operator cycle) - heartbeat-count probe self-matches (the 05:13Z "duplicate heartbeat" was likely the probe's own shell)

- What: the charter step-1 health check "powershell processes matching
  `dispatch_heartbeat`" self-matches - the scanning command's own command line
  contains the literal `dispatch_heartbeat`, so every scan also reports the
  probe shell (and this operator session's wrapper). This cycle the 05:13Z
  "second heartbeat pid 32664, parent = the operator wrapper" shape reappeared
  as short-lived powershell children of this session's opencode (8244, 29804,
  34008 - each gone within seconds); the only durable match is the incumbent
  27872, whose command line is exactly
  `-File C:\Users\stefa\look\loop\dispatch_heartbeat.ps1`. The 05:13Z operator
  killed 32664 believing it a duplicate; if 32664 was the probe shell the kill
  was harmless, but the duplicate-heartbeat diagnosis is unproven.
- Why the operator can't: hardening the probe is a code change (out of scope).
- Action needed: make the heartbeat-count check an exact match on the
  `-File ...dispatch_heartbeat.ps1` invocation (or exclude the current
  process/its command-line ancestry) before any kill, and re-audit whether a
  real second heartbeat ever existed.
- Start from: `loop/OPERATOR_CHARTER.md` step 1; the health-sweep commands in
  this and the prior `loop/OPERATOR_LOG.md` entries.

## 2026-09-11 07:12 UTC (operator) - AUTHOR-WIRE-MIRROR-ARM double-dispatched into slots 0+1 and in a reset/re-dispatch loop; trigger is a hung truck123d full-suite test vs the heartbeat freshness guard

- What: two LIVE workers on the SAME branch `packet/AUTHOR-WIRE-MIRROR-ARM` -
  slot 0 (cmd 18540 -> opencode 31940, forked 06:29:36Z) and slot 1 (cmd 32348 ->
  opencode 26564, forked 06:52:05Z). The heartbeat log's 06:49:41Z cycle read
  "0 running" (180s freshness guard, while the worker sat blocked on a `cargo
  test` step) and dispatched the packet into slot 1 without clearing slot 0 -
  the documented 180s-guard-vs-long-test double-dispatch class (same as DOOR).
  Slot 0's worktree was then RESET 06:49:47Z
  (`loop/slots/0/abandoned-20260911-024947.patch`, 3483 B - the
  `_reflection_frame`/`_mirror_edge`/`_mirror_wire` door.py work); slot 1 was
  re-forked 06:52:05Z after its own 06:49:48Z reset archived DOOR-PARTIAL-ARC-FLIP
  content (`loop/slots/1/abandoned-20260911-024948.patch`, 33969 B).
- Root trigger: cargoq's running job is slot 0's `cargo test --locked -p
  truck123d` (START 06:35:34Z, cwd `slots/0/wt`), HUNG on the pre-existing
  `unanswerable_arc_lathe_refuses_typed` test (test exe
  `truck123d-361b704ce0515825.exe` pid 34104 since 06:36:09Z; the same test hit
  cargoq's 2400s timeout once already at 01:53:25Z). Slot 1's required test is
  QUEUED behind it, so slot 1's events also age past the guard. `dispatch_ready
  --dry-run` (07:10Z) reports AUTHOR-WIRE-MIRROR-ARM as a DEAD dispatch and would
  reset+delete+redispatch - so the heartbeat's next cycle can DESTROY slot 1's
  live work (the only live copy).
- Why the operator can't: the double-dispatch adjudication (which run/impl is
  authoritative) and the freshness-guard fix are harness/semantic changes;
  killing a live worker is outside operator authority. The hung job self-reaps
  ~07:15Z (cargoq 40-min timeout), after which both workers unblock.
- Action needed: (1) PIN or amend the AUTHOR-WIRE-MIRROR-ARM row so
  dispatch_ready stops resetting+redispatching it (its note carries no `landed`
  marker, so it reads READY); (2) adjudicate the two runs and recover work -
  slot 1's live worktree first, then `slots/0/abandoned-20260911-024947.patch`
  (mirror arm) and `slots/1/abandoned-20260911-024948.patch` (DOOR partial-arc);
  (3) fix the freshness guard so a worker blocked on a cargoq step is not read
  as dead (same class as the DOOR double-dispatch); (4) consider scoping the
  packet done-when off the full `cargo test -p truck123d` suite, whose
  pre-existing `unanswerable_arc_lathe_refuses_typed` hang wedges it.
- Start from: `loop/dispatch_heartbeat.log` (the 06:49:41Z cycle); `loop/cargoq/
  server.log` tail (the hung `cargo test --locked -p truck123d`);
  `git -C loop/slots/1/wt status`; `loop/packets/AUTHOR-WIRE-MIRROR-ARM.md`.
- UPDATE 07:16Z: the 07:12:10Z heartbeat cycle confirmed the loop - it reset
  slot 1 and archived its live work to
  `loop/slots/1/abandoned-20260911-031216.patch` (3503 B), then dispatched a
  THIRD run into slot 2 (cmd 21184, forked 07:14:57Z). All three workers (slots
  0/1/2) are still alive on branch `packet/AUTHOR-WIRE-MIRROR-ARM`. Recovery
  archives now: slot 0 `abandoned-20260911-024947.patch`, slot 1
  `abandoned-20260911-031216.patch`, plus the older slot-1 DOOR archive
  `abandoned-20260911-024948.patch`. Pin the row NOW - each 10-min heartbeat
  cycle can reset another worker and spawn another duplicate.

## 2026-09-11 07:42 UTC (operator) - AUTHOR-WIRE-MIRROR-ARM: hung-test root cause RECURRED after the 07:15Z reap; the 8 GB disk floor is now the only guard against a 4th duplicate

- What: the 06:35:34Z hung `cargo test --locked -p truck123d` was reaped by
  cargoq at 07:15:34Z, but the replacement job `cargo test --locked -p
  truck123d --lib` (started 07:15:43Z) hit the SAME pre-existing
  `unanswerable_arc_lathe_refuses_typed` hang - test exe
  `truck123d-361b704ce0515825.exe` PID 9356 still alive at 07:38Z, due to
  re-reap ~07:55Z. cargoq is wedged again (running true, queued 4).
- Consequence: the 07:35Z heartbeat read 0 running (180s freshness guard) and
  tried to dispatch a 4th/5th worker; BOTH `new_slot` calls FAILED on the 8 GB
  disk floor (6.5 GiB free). The floor - not the guard - is the only thing
  preventing another duplicate. DO NOT reclaim disk or run the janitor until
  the row is pinned; freeing disk would spawn a 4th duplicate into a 3.78
  GiB-RAM machine (0xc0000409 zone).
- Why the operator can't: pinning the row / fixing the freshness guard /
  scoping the packet off the hanging test are harness/semantic changes;
  killing a live worker or resetting a duplicate slot is outside authority.
- Action needed: (1) PIN or amend the AUTHOR-WIRE-MIRROR-ARM row NOW (its note
  carries no `landed` marker so it reads READY and the heartbeat keeps
  re-dispatching it); (2) fix `unanswerable_arc_lathe_refuses_typed` or scope
  the packet's done-when off the full `cargo test -p truck123d`/`--lib` suite;
  (3) fix the freshness guard so a worker blocked on a cargoq step is not read
  as dead; (4) then reclaim disk.
- Start from: `loop/cargoq/server.log` (07:15:34Z TIMEOUT, 07:15:43Z restart);
  `loop/dispatch_heartbeat.log` (the 07:35Z cycle, new_slot floor failures);
  `loop/packets/AUTHOR-WIRE-MIRROR-ARM.md`; `loop/slots/1/wt` (the one live
  copy: untracked `truck123d/tests/wire_mirror_arm.rs`).

## 2026-09-11 08:02 UTC (operator) - AUTHOR-WIRE-MIRROR-ARM: hang cleared, but the pin + freshness-guard root causes remain; the 8 GB disk floor is still the only guard

- What: cargoq TIMEOUT-reaped the second `cargo test --locked -p truck123d
  --lib` at 07:55:43Z. All three triple-dispatched workers (slots 0/1/2, branch
  packet/AUTHOR-WIRE-MIRROR-ARM) resumed and are issuing cargo: slot 0
  `test --lib mirror` START 08:00:19Z; slots 1/2 `test wire_mirror_arm` exit
  101. The 07:55:18Z heartbeat still read 0 running (180s freshness guard) and
  tried to dispatch AUTHOR-WIRE-MIRROR-ARM -> slot 3 + AUTHOR-CENSUS-NAMES ->
  slot 4; BOTH new_slot FAILED on the 8 GB floor (5.9 GiB free). So the floor
  is still the only thing preventing a 4th duplicate, exactly as at 07:42Z.
- New precise fact: the AUTHOR-WIRE-MIRROR-ARM registry row is `status=READY`,
  `packet=loop/packets/AUTHOR-WIRE-MIRROR-ARM.md`, and carries NO `landed`
  marker (note ends "... QUEUE-2 (door.py lane: after PARTIAL-ARC) - dispatched
  2026-09-10 shipping wave"). That is why dispatch_ready treats it as
  dispatchable.
- New precise fact: RG-23-CERTIFIED-ENTRY-WIRING and
  RG-9-REFLECT-SOLID-PRODUCTION are `status=READY` with an EMPTY `packet`
  field - there is NO packet file. gen_packet --check crashes on the missing
  path (FileNotFoundError), which the dispatcher renders as "ANCHOR CHECK
  FAILED". These need authoring, not an anchor re-measure.
- Why the operator can't: pinning a registry row, fixing the freshness guard,
  authoring a packet, and scoping the packet off the hanging test are all
  semantic/harness changes outside the three-file operator authority; killing
  a live worker or resetting a duplicate slot is also outside authority.
- Action needed: (1) PIN or amend the AUTHOR-WIRE-MIRROR-ARM row NOW; (2) fix
  `unanswerable_arc_lathe_refuses_typed` or scope the packet's done-when off
  the full `cargo test -p truck123d`/`--lib` suite; (3) fix the freshness guard
  so a worker blocked on a cargoq step is not read as dead; (4) author the
  RG-23/RG-9 packets or park the rows; (5) then reclaim disk (5.9 GiB free is
  below the 8 GB new_slot floor, so the loop is frozen until then).
- Start from: `loop/PACKETS.jsonl` (the AUTHOR-WIRE-MIRROR-ARM, RG-23, RG-9
  rows); `loop/cargoq/server.log` (07:55:43Z TIMEOUT);
  `loop/dispatch_heartbeat.log` (07:55:18Z cycle, floor failures);
  `loop/slots/1/wt` (the one live copy: untracked
  `truck123d/tests/wire_mirror_arm.rs`).

---

[operator 2026-09-11T08:36Z - new + carried]

RESOLVED this cycle: the AUTHOR-WIRE-MIRROR-ARM redispatch root is closed. The
overnight driver merged worker commit 19acb3e as 38d3534 and the operator
completed the bookkeeping (filed loop/results/AUTHOR-WIRE-MIRROR-ARM.json,
flipped the row DONE, commit 0039227). No further pin needed. The disk floor is
no longer binding (11.5 GiB free), and the hung `cargo test --locked -p
truck123d` that wedged cargoq has cleared (queued 0, running false).

NEW - AUTHOR-CENSUS-NAMES SPEC_GAP (geometry-judgment rebooking required):
- What: worker RESULT status SPEC_GAP; no file in write_allow was edited. The
  packet classifies Ellipse and RectangleRounded as TIER A ("record exactly
  now") but both need new executor carriers - `ProfileEdge::Ellipse` and a
  mixed line/arc section vocabulary - that do not exist on the landed executor
  (bd_bridge.rs ProfileEdge has only Line/Spline/Circle; profile_loop rejects a
  Circle mixed with any other edge). The packet's own done criterion
  (RectangleRounded profile extruded, aligned, coned green) is unreachable
  without them. Recording an Ellipse as a sampled Spline is not exact.
- Why the operator can't: widening a packet's write set / re-scoping TIER A is
  a packet-semantic + geometry-judgment decision outside operator authority.
- Action needed: either (a) widen the AUTHOR-CENSUS-NAMES write set to add
  `ProfileEdge::Ellipse` + an arc edge and re-dispatch, or (b) re-scope the
  packet to the names that fit (Align, Cone, RegularPolygon + the three TIER B
  typed refusals) and book Ellipse/RectangleRounded as a follow-up.
- Start from: `loop/results/AUTHOR-CENSUS-NAMES.PENDING-QUESTION.QUESTION.md`
  and `loop/results/AUTHOR-CENSUS-NAMES.PENDING.RESULT.json` (the worker's
  carrier inventory); packet `loop/packets/AUTHOR-CENSUS-NAMES.md`.

CARRIED - RG-23-CERTIFIED-ENTRY-WIRING and RG-9-REFLECT-SOLID-PRODUCTION:
READY rows with EMPTY `packet` fields (no packet file exists). `dispatch_ready`
renders this as "ANCHOR CHECK FAILED" with an empty detail. Authoring, not the
anchor ritual - needs a human/orchestrator to author the packets or park the
rows. Start from `loop/PACKETS.jsonl` (both rows).

CARRIED (unchanged): FRAME-REVOLVE F1 non_z_axis pin amendment
(`truck123d/tests/ttc_lathe_spline.rs:255`); duplicate supervisors
(19172+27828) + the lagging cargoq restart guard; slot-4 + slot-7 wt RESULT
residue parking the driver's dispatch arm; TOR-C flip-or-pin (orchestrator
call); MONO-10 missing packet; RDEF-M4 H-8 stale anchor; duplicate-driver
probe fix.

---

[operator 2026-09-11T09:41Z - new + carried]

NEW - `loop/schedule.py` crashes on the current registry schema:
- What: `python loop/schedule.py` raises `KeyError: 'needs'` at schedule.py:45
  (`if any(n not in done for n in r['needs'])`). ~31 registry rows carry
  `depends_on` and have no `needs` key, so the frontier loop crashes on the
  first such READY row.
- Why the operator can't: schedule.py is not one of the three files the
  operator may edit; the fix is a one-line harness change.
- Action needed: normalize the key, e.g.
  `if any(n not in done for n in r.get('needs', r.get('depends_on', []))):`.
  `dispatch_ready.py` is the dispatch authority and is unaffected - this is a
  query/debug primitive only, so priority is low.
- Start from: `loop/schedule.py:45`.

CARRIED (unchanged): AUTHOR-CENSUS-NAMES SPEC_GAP rebooking; RG-23/RG-9
missing packet files; FRAME-REVOLVE F1 non_z_axis pin amendment
(ttc_lathe_spline.rs:255); duplicate supervisors (19172+27828) + lagging cargoq
restart guard; slot-4 + slot-7 wt RESULT residue; TOR-C flip-or-pin.

---

[operator 2026-09-11T16:59Z - RDEF-M4 now unblocked but not dispatchable]

UPDATED - RDEF-M4-NUMERIC-TIER: its `needs` are now ALL landed
(RDEF-M3-WITNESS-TIER is DONE after the owner's 7a08c0c reconciliation), so per
the mechanical rule it is a BLOCKED->READY candidate. It cannot be flipped
because the packet fails preflight against the current tree:
- What: `python loop/gen_packet.py --check loop/packets/RDEF-M4-NUMERIC-TIER.md`
  -> "MISMATCH A1: grep: vendor/truck/truck-certified/src/tangency/classify.rs:
  No such file or directory" (H-8 stop condition); `python
  loop/packet_lint.py loop/packets/RDEF-M4-NUMERIC-TIER.md` -> "FAIL
  H1_NEW_MODULE" (write_allow creates new vendor .rs files but the packet never
  states the H-1 `#![deny(clippy::unwrap_used)]` requirement).
- Why the operator can't: the anchor target file is absent, so this is a
  packet re-scope (the write set references `tangency/classify.rs`, which the
  tree does not have) plus a missing house-rule statement - packet SEMANTICS,
  which the operator may not edit. It is NOT a re-measurable expect-value.
- Action needed: re-scope RDEF-M4-NUMERIC-TIER against the post-RDEF-M3 tree
  (fix the write set / anchors to the real module layout, add the H-1
  statement), then flip BLOCKED->READY. M0's TANGENCY-SYSTEM-FROM-DEFLATED
  conflict adjudication is already DONE (RDEF-M0-CHECKER-ACCOUNTING), so that
  part of the note's requirement is satisfied.
- Start from: `loop/packets/RDEF-M4-NUMERIC-TIER.md` (anchor A1 + Template
  house rules), then `loop/PACKETS.jsonl` row RDEF-M4-NUMERIC-TIER.

CARRIED (unchanged): RG-23/RG-9 packet preflight; FRAME-REVOLVE F1
non_z_axis pin amendment (ttc_lathe_spline.rs:255); duplicate supervisors
(19172+27828) + lagging cargoq restart guard; slot-4 + slot-7 wt RESULT residue;
TOR-C flip-or-pin; `loop/schedule.py` KeyError 'needs' at schedule.py:45.

[operator 2026-09-11T17:26Z - NEW: slot 0 worker blocked on a hung broad lib test]

- What: slot 0 (AUTHOR-CENSUS-NAMES, pid 2268 alive) is blocked on a
  self-initiated `cargo test -p truck123d --lib --locked` (cargoq START
  13:09:22 local). The test binary `truck123d-361b704ce0515825.exe` (pid 2076)
  is HUNG: 0 CPU accumulated over 12.8 min. The packet's Done-when asks only for
  scoped checks, so the broad lib test is the worker's own over-run, not a
  packet requirement.
- Why the operator can't: the worker is ALIVE and its worktree holds 1127+
  uncommitted lines (corpus/ttc/door.py +385, truck123d/src/bd_bridge.rs +840,
  truck123d/src/binding.rs +9, new truck123d/tests/census_names.rs). Resetting
  slot 0 (which `dispatch_ready` would do - it labels the slot a DEAD dispatch
  because events are stale while the worker blocks) discards proven work for a
  hang the worker will self-recover from. The operator must not restart a live
  worker, and must not kill anything but its own timed-out predecessor.
- Self-recovery: cargoq's per-job timeout (CARGOQ_TIMEOUT, default 2400s) kills
  the hung job ~13:49 local; the worker receives exit 3 and continues. If it
  does NOT recover by the next operator cycle, or if the lib test hangs again
  after a reset, the hang is a real defect.
- Action needed (human/orchestrator): identify the hanging test in
  `cargo test -p truck123d --lib` (run it directly outside cargoq with
  `--nocapture --test-threads=1` and watch which test stalls) - a hanging lib
  test will keep wedging every worker that runs the full lib suite. Until then,
  operators must NOT run `dispatch_ready` non-dry-run while slot 0 is live: it
  would reset+delete the worker.
- Start from: `loop/slots/0/events.jsonl` (last events, session
  ses_f6e99c721ffeeugzOmZNHXtGnX), `loop/cargoq/server.log` (the START with no
  DONE), and `loop/slots/0/wt` (the uncommitted work).

CORRECTION (operator 2026-09-11T17:26Z): the 16:59Z entry above states RG-23/
RG-9 now fail "on write-set clash with the RUNNING slot-0 bd_bridge.rs, not on
missing files". Re-derived by command: both registry rows still carry
`packet: ""` and BOTH PACKET FILES ARE ABSENT (`Test-Path
loop/packets/RG-23-CERTIFIED-ENTRY-WIRING.md` = False, `.../RG-9-REFLECT-SOLID-
PRODUCTION.md` = False). `dispatch_ready` reports ANCHOR CHECK FAILED with empty
content, i.e. the missing-file authoring gap. The write-set clash is a SEPARATE
reason they cannot dispatch concurrently with slot 0; it is not the preflight
failure. Both packets must be authored before either can dispatch.

## 2026-09-11 17:52 UTC - NEW: slot 0 AUTHOR-CENSUS-NAMES FINISHED with RESULT status "LANDED" (not DONE) + an out-of-scope finding

- What: slot 0 (AUTHOR-CENSUS-NAMES) is now FINISHED. The worker committed
  ccec2e1 + rustfmt 43e26c9 on branch `packet/AUTHOR-CENSUS-NAMES` (base df81808,
  which IS an ancestor of HEAD `9616df0`); branch diff vs base = 4 files /
  +1511 -107 (corpus/ttc/door.py +385, truck123d/src/bd_bridge.rs +840,
  truck123d/src/binding.rs +9, truck123d/tests/census_names.rs +384). RESULT.json
  status is **"LANDED", not "DONE"**, and it carries findings F1-F4: F1 harness
  reset the slot mid-session (work redone); F2 a single conic edge is now a whole
  extrusion section; F3 TIER-B typed refusals; **F4 `truck123d/src/binding.rs`
  is OUTSIDE the packet's `write_allow` (read_allow only) - a compile-forced
  wildcard-arm edit the worker flagged "for adjudication" (the DOOR-CIRCLE-FLIP
  `ProfileEdge::Circle` collateral precedent)**.
- Why the operator can't: the charter's landing gate requires RESULT status DONE;
  "LANDED" is not DONE, and the RESULT carries findings. Both are explicit
  do-not-land triggers (the FRAME-REVOLVE precedent: status LANDED -> the
  operator did not land; the orchestrator did). The F4 scope question is
  packet-semantics adjudication, which the operator may not decide. The packet's
  done-when is CLAIMED green (cargo check -p truck123d clean; census_names 8
  passed; lib 82 passed; door_circle_flip/ttc_lathe_spline/ttc_authoring_arms
  4/4 each) but the operator did NOT re-run them (not landing; disk below floor).
- Why it matters: the four FHC-EX packets (`needs: [AUTHOR-CENSUS-NAMES]`) are
  serial on this row, so the owner's F1/hypercar send is stalled until it lands
  or is adjudicated. And **RESULT.json is gitignored/untracked in the slot wt
  (NOT on the branch)** - the next heartbeat re-fork destroys it (the known
  recycle-destroys-RESULT class). The code itself is safe on the branch.
- Action needed (orchestrator/human): adjudicate F4 (accept the precedented
  binding.rs collateral, or require a packet re-scope), then land
  `packet/AUTHOR-CENSUS-NAMES` (merge --no-ff; scoped-verify `cargo check -p
  truck123d` + `cargo test -p truck123d --test census_names`; file
  `loop/results/AUTHOR-CENSUS-NAMES.json`; flip the row DONE). Preserve the
  RESULT.json before any slot re-fork.
- Start from: `git -C loop/slots/0/wt log --oneline -3`; `git -C
  loop/slots/0/wt diff --stat df81808..43e26c9`; `loop/slots/0/wt/RESULT.json`;
  `loop/packets/AUTHOR-CENSUS-NAMES.md`.

UPDATED (operator 2026-09-11T17:52Z): DISK re-confirmed BELOW the 8 GB floor -
entered 2.68 GiB, `janitor ensure --need 15` reclaimed ~4.2 -> 6.4 GiB (STILL
SHORT; the slot-0 target is now 0.0 GB, nothing else reclaimable). RG-23/RG-9
still missing packet files; RDEF-M4 still preflight-fails. Carried unchanged.

RESOLVED (operator 2026-09-11T18:11Z): the slot-0 AUTHOR-CENSUS-NAMES item is
CLOSED - the overnight driver landed it (merge `54d0713`, row landed-note
`b860b0a`; tip 43e26c9 = ancestor of HEAD `2706af4`; RESULT preserved at
loop/results/AUTHOR-CENSUS-NAMES.PENDING.RESULT.json). Status-only residue: the
PACKETS row note carries `LANDED 43e26c9` but its `status` is still `READY`
(functionally landed; landed() skips it) and the loop/LEDGER.jsonl append is
uncommitted - left for the orchestrator, no dispatch impact.

## 2026-09-11 18:11 UTC - CARRIED/UPDATED: DISK below the 8 GB floor now stalls the FHC-EX frontier

- What: DISK is 5.8 GiB free (floor 8 GB, goal 15 GB). `janitor ensure --need
  15` reclaimed ~0.0 (pool exhausted: no root/slot `target/` dirs, no TEMP
  look-verify-baseline-* leaks). The ~500 GB C: usage is outside the loop; the
  repo itself is ~2.8 GB (scratch/ 1.73 GB untracked human WIP + loop/ 0.91 GB,
  mostly untracked loop/baselines/*.json).
- Why it matters NOW: the frontier moved. `dispatch_ready --dry-run
  --max-workers=4` would dispatch **FHC-EX-A-CLOSED-LOOP-SHELL -> slot 0**
  (FHC-EX-B / FHC-TRIM-EXTRUDE-ENVELOPE / FHC-MIRROR-FORM serial behind it) -
  the first dispatchable work since AUTHOR-CENSUS-NAMES landed. But `new_slot`
  refuses below the 8 GB floor, so the whole FHC chain is stalled on disk.
  RAM is also 1.6-2.7 GiB free (below the 3 GB stack threshold).
- Why the operator can't: no in-loop reclaimable space remains; widening the
  floor or deleting the human session's untracked scratch/baselines is a
  harness/owner decision, not a mechanical operator unblock.
- Action needed (human/owner): free C: space outside the loop (or relocate the
  pagefile / clear a non-loop consumer) to reach the 8 GB floor; then the
  heartbeat's next cycle dispatches FHC-EX-A. Carried unchanged: RG-23/RG-9
  missing packet files; RDEF-M4 re-scope; MONO-10 owner R3-mesh decision;
  FRAME-REVOLVE F1 non_z_axis pin; duplicate supervisors + lagging cargoq
  restart guard; TOR-C flip-or-pin; schedule.py 'needs' crash.
- Start from: `Get-PSDrive C`; `python loop/janitor.py status`;
  `python loop/dispatch_ready.py --dry-run --max-workers=4`.

## 2026-09-11 18:35 UTC - CARRIED (no new item): DISK 5.7 GiB still below the 8 GB floor

- Re-derived this cycle: `janitor ensure --need 15` -> reclaimed ~0.0 -> 5.7 GB
  free (STILL SHORT). No root/slot `target/` dirs exist; no TEMP
  look-verify-baseline-* leaks. Frontier unchanged: FHC-EX-A-CLOSED-LOOP-SHELL
  READY but `new_slot` refuses below the floor, so the FHC chain stays stalled.
- Action needed (human/owner): free C: space outside the loop to reach 8 GB.
  Carried unchanged: RG-23/RG-9 missing packet files (`packet: ''`); RDEF-M4
  re-scope; MONO-10 owner R3-mesh decision; FRAME-REVOLVE F1 non_z_axis pin;
  duplicate supervisors + lagging cargoq restart guard; TOR-C flip-or-pin;
  schedule.py 'needs' crash.

## 2026-09-11 18:57 UTC - CARRIED (no new item): DISK 5.6 GiB still below the 8 GB floor

- Re-derived this cycle: `janitor ensure --need 15` -> reclaimed ~0.0 -> 5.6 GB
  free (STILL SHORT). No root `target/`; no TEMP look-verify-baseline-* leaks;
  slot dirs ~0.9 GB total. The bulk is untracked human `scratch/` 1.73 GB.
  Frontier unchanged: FHC-EX-A-CLOSED-LOOP-SHELL READY but the real dispatcher's
  `new_slot` refuses below the floor, so the FHC chain stays stalled.
- Action needed (human/owner): free C: space outside the loop to reach 8 GB.
  Carried unchanged: RG-23/RG-9 missing packet files (`packet: ''`); RDEF-M4
  re-scope; MONO-10 owner R3-mesh decision; FRAME-REVOLVE F1 non_z_axis pin;
  duplicate supervisors + lagging cargoq restart guard; TOR-C flip-or-pin;
  schedule.py 'needs' crash.

## 2026-09-11 19:21 UTC - CARRIED (no new item): DISK 5.5 GiB still below the 8 GB floor

- Re-derived this cycle: `janitor ensure --need 15` -> reclaimed ~0.0 -> 5.5 GB
  free (STILL SHORT). No root `target/`; no TEMP look-verify-baseline-* leaks;
  `loop/slots/*` outer+inner targets are all 0 bytes; slot dirs ~0.9 GB total.
  The bulk is untracked human `scratch/` 1.73 GB (not loop-owned - do not delete).
  Frontier unchanged: FHC-EX-A-CLOSED-LOOP-SHELL READY but the real dispatcher's
  `new_slot` refuses below the floor, so the FHC chain stays stalled.
- Action needed (human/owner): free C: space outside the loop to reach 8 GB.
  Carried unchanged: RG-23/RG-9 missing packet files (`packet: ''`); RDEF-M4
  re-scope; MONO-10 owner R3-mesh decision; FRAME-REVOLVE F1 non_z_axis pin;
  duplicate supervisors + lagging cargoq restart guard; TOR-C flip-or-pin;
  schedule.py 'needs' crash.

## 2026-09-11 21:00 UTC - NEW: duplicate dispatch of FHC-EX-B-SPLINE-LOFT-OPERANDS (slots 0 AND 1)

- The heartbeat dispatched FHC-EX-B to slot 0 at 16:25:50 local (cmd pid 10504,
  opencode 2384, session ses_f6dda1e31ffePZufbaRlRmdTja) and then AGAIN to slot 1
  at 16:45:55 local (cmd pid 20780, opencode 31040, session
  ses_f6dc55f58ffep4TZZSPRw0gQCf). `loop/dispatch_heartbeat.log` shows the
  16:45:55 cycle reporting "slots: 8 (0 running, 7 free)" - i.e. `dispatch_ready`'s
  dead-dispatch check false-positived on slot 0 while its worker was alive and
  mid-work (its branch tip had 0 commits ahead of base, which the check reads as a
  dead dispatch).
- Both workers are live and progressing (events 0.9/3.5 min fresh at sweep) and
  both edit the same write-set `truck123d/src/bd_bridge.rs`. The operator did NOT
  kill or reset either (the charter forbids disturbing a live worker).
- Why it matters: two workers are being paid for the same packet, and their
  branches will collide on merge. At most one is needed; the second is wasted
  worker-hours and doubles the RAM/disk pressure while the machine is already
  below the disk floor.
- Action needed (human/orchestrator): decide which branch to keep. When the first
  finishes with a DONE RESULT, land it and discard (or compare) the other. Also
  fix `dispatch_ready`'s dead-dispatch detection so a live worker whose branch tip
  has 0 commits ahead is not classified dead (ground truth is the process scan in
  `slot_status.py`, not the branch tip). Start from `loop/dispatch_ready.py` and
  the 2026-09-11 16:45 `loop/dispatch_heartbeat.log` entry.

## 2026-09-11 21:00 UTC - CARRIED (no new item): DISK 4.8 GiB still below the 8 GB floor

- Re-derived this cycle: `janitor.py status` -> 4.8 GB free; the only reclaimable
  targets are the TWO LIVE slots' `target/` + `wt/target/` (slot 0: 1.0 + 0.7 GB;
  slot 1: 1.0 + 0.6 GB), which must not be touched while their workers run. No
  root `target/`, no TEMP look-verify-baseline-* leaks. Nothing reclaimable.
- Action needed (human/owner): free C: space outside the loop to reach 8 GB.
  Carried unchanged: RG-23/RG-9 missing packet files (`packet: ''`); RDEF-M4
  re-scope; MONO-10 owner R3-mesh decision; FRAME-REVOLVE F1 non_z_axis pin;
  duplicate supervisors + lagging cargoq restart guard; TOR-C flip-or-pin;
  schedule.py 'needs' crash.

## 2026-09-11 21:22 UTC - CARRIED (no new item): duplicate FHC-EX-B dispatch persists, now 25+ min in

- Re-derived this cycle: the duplicate is UNCHANGED. Slot 0 (cmd pid 10504,
  opencode 2384, session ses_f6dda1e31ffePZufbaRlRmdTja) has no event for 24.1 min
  - a bare `step_start` with no following tool event - because it is blocked in
  the cargoq-run `cargo test --profile quick -p truck123d --lib swept_admission
  --locked` (server.log START 16:58:15 local; cargoq's 40-min timeout frees it
  ~17:38 local). Slot 1 (cmd pid 20780, opencode 31040) is progressing (events
  12.8 min old) and its cargo job is QUEUED behind slot 0's. Both edit the same
  `truck123d/src/bd_bridge.rs`. Still NOT killed/reset (charter: do not disturb a
  live worker).
- NEW supporting evidence: `dispatch_ready --dry-run --max-workers=4` still
  FALSE-POSITIVES FHC-EX-B as "DEAD dispatch (slot 1 holds no matching RESULT) -
  would reset + delete + redispatch". Running the real dispatcher would destroy
  the live slot-1 worker. This is the same dead-dispatch defect the 21:00Z
  escalation named (branch tip 0 commits ahead of base read as dead); it is now
  confirmed to fire on slot 1 as well.
- Action needed (human/orchestrator): (1) decide which FHC-EX-B branch to keep
  once one returns a DONE RESULT; (2) fix `dispatch_ready`'s dead-dispatch
  detection to use the process scan in `slot_status.py` as ground truth rather
  than the branch tip. Start from `loop/dispatch_ready.py` and the 2026-09-11
  16:45 + 21:22 `loop/dispatch_heartbeat.log` / dry-run output.
- Also carried: disk 4.7 GiB free (below the 8 GB floor; only the two live slots'
  targets exist, nothing reclaimable).

## 2026-09-11 21:45 UTC - CARRIED + new detail: duplicate FHC-EX-B now serializing on a timed-out lib test

- Re-derived this cycle: the duplicate is UNCHANGED (slots 0+1, both live). NEW
  detail: slot 0's worker-initiated `cargo test --profile quick -p truck123d
  --lib swept_admission --locked` TIMED OUT at 17:38:15 local after the 2400s
  cargoq limit (server.log), and the queue immediately started slot 1's IDENTICAL
  `--lib swept_admission` test (START 17:38:15). So the duplicate pair now
  serializes on a 40-min test that already timed out once; a second timeout is
  likely (~18:18 local). Both workers are live (slot 0 watching a fresh rustc
  build; slot 1 waiting on the queued job) and were NOT killed/reset.
- Why it matters: two worker-hours are being spent on one packet, and the
  `swept_admission` lib test is an unbounded-time sink (a worker self-initiated
  broad `--lib` run, not the packet's required test
  `truck123d/tests/extraction_breadth_b.rs`). If both workers keep retrying it,
  the cargoq queue stays wedged.
- Action needed (human/orchestrator): (1) decide which FHC-EX-B branch to keep
  once one returns a DONE RESULT; (2) fix `dispatch_ready`'s dead-dispatch
  detection to use `slot_status.py`'s process scan, not the branch tip (it
  false-positived at 21:22Z; it did NOT fire this pass at 21:45Z, so it is
  intermittent); (3) consider whether the `swept_admission` lib test needs a
  timeout/targeted scope so it cannot wedge the queue. Start from
  `loop/dispatch_ready.py`, `loop/cargoq/server.log` (17:38:15), and
  `loop/dispatch_heartbeat.log`.
- Also carried: disk 3.95 GiB (below the 8 GB floor; janitor pool = only the two
  live slots' targets); RAM 2.58 GiB (below the 3 GB threshold).

## 2026-09-11 22:10 UTC - CARRIED (no new item): duplicate FHC-EX-B now wedged in cargoq on the swept_admission lib test

- Re-derived this cycle: the duplicate is UNCHANGED (slots 0+1, both live, both
  STALLED-blocked on cargoq). The queue is running slot 1's IDENTICAL
  `cargo test --profile quick -p truck123d --lib swept_admission --locked`
  (slot 0's run timed out 17:38:15 after the 2400s limit; slot 1's started
  17:38:15, times out ~18:18), with slot 0's `qbuild -p truck123d` queued behind.
  Both workers' events 13.5/12.5 min old at 18:09 (bare waits, not dead); both
  processes alive. Neither killed/reset (charter: do not disturb a live worker).
- Why it matters: two worker-hours spent on one packet; the `swept_admission`
  lib test is an unbounded-time sink (a worker self-initiated broad `--lib` run,
  not the packet's required `truck123d/tests/extraction_breadth_b.rs`) and it has
  now wedged the whole cargoq queue for ~40 min twice in a row.
- Action needed (human/orchestrator): (1) decide which FHC-EX-B branch to keep
  once one returns a DONE RESULT; (2) fix `dispatch_ready`'s dead-dispatch
  detection to use `slot_status.py`'s process scan, not the branch tip; (3)
  scope/timeout the worker's self-initiated broad `--lib` runs so one cannot wedge
  the queue. Start from `loop/dispatch_ready.py`, `loop/cargoq/server.log`
  (17:38:15), `loop/dispatch_heartbeat.log`.
- Also carried: disk 3.48 GiB (below the 8 GB floor; janitor pool = only the two
  live slots' targets); RAM 3.07 GiB (at the 3 GB threshold). RG-23/RG-9 missing
  packet files; RDEF-M4 re-scope; MONO-10 owner R3-mesh decision; FRAME-REVOLVE F1
  non_z_axis pin; duplicate supervisors + lagging cargoq restart guard; TOR-C
  flip-or-pin; schedule.py 'needs' crash; slot-4/7 wt RESULT residue; CL-005/CL-006
  READY-but-landed bookkeeping.

## 2026-09-11 22:34 UTC - CARRIED + UPDATED: duplicate FHC-EX-B (slots 0+1) - slot 0 progressing, slot 1 now HUNG on an API step; disk blocker CLEARED

- Re-derived this cycle: slot 0 is LIVE and progressing (wrote
  `truck123d/tests/extraction_breadth_b.rs`; cargoq running that test since
  18:30:13 local; events 2.6 min fresh) - it is the copy to keep. Slot 1 is the
  redundant duplicate and is now hung on a model-API step, NOT the cargoq queue:
  its last event is a bare `step_start` at 18:12:11 local with no output for
  ~22 min, worktree clean, no cargo job queued (cargoq ping queued 0). Both
  slot-1 processes are alive (cmd 20780 / opencode 31040).
- Action needed (human/orchestrator): (1) once slot 0 returns a DONE RESULT,
  land it and reap slot 1's hung processes (20780 cmd, 31040 opencode) to free
  ~1 GB RAM and the slot - reaping a live worker is outside the operator charter;
  (2) fix `dispatch_ready`'s dead-dispatch detection to use `slot_status.py`'s
  process scan, not the branch tip (intermittent false positive); (3) scope/timeout
  worker self-initiated broad `--lib` runs so one cannot wedge cargoq.
- CLEARED this cycle: the disk blocker. Disk recovered to 32.4 GB free (was 3.48
  GiB) - above the 8 GB floor and the 15 GB goal; the FHC chain is no longer
  disk-stalled. RAM remains LOW at ~1.7-2.2 GiB (two workers resident - do not
  stack a third).
- Also carried: RG-23/RG-9 missing packet files; RDEF-M4 re-scope; MONO-10 owner
  R3-mesh decision; FRAME-REVOLVE F1 non_z_axis pin; duplicate supervisors +
  lagging cargoq restart guard; TOR-C flip-or-pin; schedule.py 'needs' crash;
  slot-4/7 wt RESULT residue; CL-005/CL-006 READY-but-landed bookkeeping.

## 2026-09-11 22:57 UTC - CARRIED + UPDATED: duplicate FHC-EX-B RESOLVED by the owner session (slot 1 reaped); no operator action

- Re-derived this cycle: slot 1 is IDLE (pid=-, changed=0, git=packet@7830086
  =base, no work) and the owner session commit `3d78cee` records the resolution
  ("slot 1 hung+empty -> killed, reset-only; slot 0 keeper"). The duplicate is
  CLOSED. Operator did NOT re-dispatch slot 1's copy: slot 0 owns the packet and
  the shared `bd_bridge.rs`/`door.py`/`extraction_breadth_b.rs` write set, so a
  fresh dispatch would recreate the duplicate.
- Slot 0 remains the keeper and is progressing (worker cmd 10504, events 4.7 min
  fresh, changed=2).
- Registry re-derived by command: 10 BLOCKED rows, none cleanly flippable. The
  only near-miss is `RDEF-M4-NUMERIC-TIER` (dep `RDEF-M3-WITNESS-TIER` is DONE)
  whose note requires the M0 tangency-system adjudication, but no M0 registry row
  exists. Action needed (orchestrator): confirm whether M0 is subsumed/landed; if
  so flip M4 READY, else record the hold explicitly in the row.
- Carried unchanged: RG-23/RG-9 missing packet files (registry `"packet": ""`);
  MONO-10 owner R3-mesh predicate decision; FRAME-REVOLVE F1 non_z_axis pin
  (ttc_lathe_spline.rs); duplicate supervisors + lagging cargoq restart guard;
  TOR-C flip-or-pin; slot-4/7 wt RESULT residue; CL-005/CL-006 READY-but-landed
  bookkeeping; schedule.py 'needs' crash.
- Health: heartbeat 1 (27872), operator_runner 1 (27876), watchdog 1 (29264),
  cargoq UP (queued 0, running false); disk 30.9 GB free (above floor and goal);
  RAM 3.21 GB free (just above threshold; one worker resident).

## 2026-09-12 00:13 UTC - NEW: DUPLICATE DISPATCH of FHC-TRIM-EXTRUDE-ENVELOPE (slots 0 + 1, both live) - needs owner/orchestrator resolution

- What: `slot_status` shows BOTH slot 0 and slot 1 RUNNING
  `FHC-TRIM-EXTRUDE-ENVELOPE.md`. Slot 1 is on the correct branch
  `packet/FHC-TRIM-EXTRUDE-ENVELOPE` @ `3d1d769` (= HEAD), worker pid 13864,
  session `ses_f6d126887ffeHsqfQfurAjdDLj`, dispatched by the heartbeat at
  2026-09-12 00:03:58Z (`dispatch_heartbeat.log` "FHC-TRIM-EXTRUDE-ENVELOPE ->
  slot 1"). Slot 0 is a second copy on a DETACHED HEAD @ `c139afb` (older base;
  the branch is checked out in slot 1 so slot 0 could not hold it), worker pid
  35628, session `ses_f6d388643ffed1HjglUgd0U78F`, worker files stamped
  2026-09-11 23:22:20Z (with an `abandoned-20260911-200244.patch` archive at
  00:02:44Z). Both `events.jsonl` are fresh (0.1 / 1.2 min) and `changed=0` -
  both alive, neither has committed work yet.
- Why the operator did NOT resolve it: the charter's hard limits forbid killing or
  restarting a worker that is alive and making progress ("kill anything except
  your own timed-out predecessor's leftovers"). Slot 0 is an active worker, not a
  hung/empty leftover (the 3d78cee precedent reaped a *hung+empty* slot). The
  heartbeat count is exactly 1, so this is not the two-heartbeat race; it is a
  heartbeat-vs-(prior-cycle) double-dispatch on the same packet.
- Risk: two live workers write the same files (`truck123d/src/bd_bridge.rs`,
  `corpus/ttc/door.py`, `truck123d/tests/trim_extrude_envelope.rs`); one will be
  redundant at merge, and RAM is 2.5 GB free (below the 3 GB threshold) with two
  workers resident - do not dispatch a third.
- Start here (owner/orchestrator): pick the keeper and reap the other. Slot 1 is
  the better keeper (on the packet branch at HEAD, created by the documented
  dispatcher); slot 0 is detached at an older base. To reap slot 0 without losing
  a possible partial diff: `python loop/run_packet.py --reset --slot 0` (archives
  the abandoned diff first), or kill pid 35628 then reset-only. Do NOT re-dispatch
  FHC-TRIM to another slot.
- Health this cycle: heartbeat 1 (27872), operator_runner 1 (27876), watchdog 1
  (29264), overnight driver 1 (24864), cargoq UP (ping ok, queued 0, running
  true); disk 30.8 GB free; RAM 2.5 GB free.

## 2026-09-12 00:36 UTC - UPDATED: duplicate FHC-TRIM - slot 1 is now STALLED (dead-shim), slot 0 alive; reap slot 1

- What changed: the 00:13Z escalation named slot 1 the "better keeper" (on the
  packet branch @ HEAD) and slot 0 the mis-forked copy. That premise is now
  dead. `slot_status` shows slot 1 **STALLED**: `events.jsonl` frozen at
  2026-09-11 20:18:10 local (~18 min stale), no cargo/rustc attributed to it,
  `changed=1` (M `truck123d/src/bd_bridge.rs`); its cmd.exe pid 13864 is alive
  but the shim is idle (the documented dead-shim signature). Slot 0 is ALIVE and
  progressing: events 20:27:51 local, currently the cargoq job
  `test -p truck123d --lib debug_trim_extract_stage` (7 prior exit-101
  iterations = active debugging). The heartbeat's 20:34:11 cycle counts "1
  running", i.e. it no longer sees slot 1.
- Why the operator did NOT act: the charter forbids killing a worker with a live
  pid, and `--reset-only` under a live pid could clobber a woken worker. The
  duplicate is already an owner/orchestrator item.
- Recommendation (matches the 3d78cee precedent, which reaped a hung slot 1 and
  kept slot 0): keep slot 0; reap slot 1. Start here: kill pid 13864 (and its
  opencode child started 20:03:58 local), then
  `python loop/run_packet.py --reset-only --slot 1` (archives the partial
  bd_bridge.rs diff first). Do NOT re-dispatch FHC-TRIM. Note slot 0 is on a
  DETACHED HEAD @ c139afb (the packet branch is checked out in slot 1); the
  packet branch will need the landing commit moved onto it at merge time.
- Health this cycle: heartbeat 1 (27872), operator_runner 1 (27876), watchdog 1
  (29264), cargoq UP (ping ok, queued 0, running true); disk 32.9 GB free; RAM
  1.74 GB free (below the 3 GB threshold).

## 2026-09-12 04:00 UTC - UPDATED: duplicate FHC-TRIM - BOTH slots alive (the 00:36Z "slot 1 STALLED" premise is falsified); keep slot 1, reap slot 0

- What changed: the 00:36Z escalation declared slot 1 STALLED (dead-shim) and
  recommended reaping it. Re-derived this cycle: BOTH workers are alive and both
  edit `truck123d/src/bd_bridge.rs`.
  - slot 1 (branch `packet/FHC-TRIM-EXTRUDE-ENVELOPE` @ `3d1d769`, cmd pid 13864,
    opencode 35128): events 03:57:26Z (~2.2 min old), changed=1 (+92 lines), last
    cargoq job `test --profile quick -p truck123d --lib debug_trim_prism_pair`
    exit 0 at 23:56:45 local (one `exit=3221225781` = 0xC0000409 RAM-zone crash
    at 23:56:27 before it). The more active of the two.
  - slot 0 (DETACHED HEAD @ `c139afb`, cmd pid 35628, opencode 11492): events
    03:54:06Z (~5.5 min old), changed=1 (+92/-5), last cargoq job `build
    --profile quick -p truck123d` exit 0 at 23:54:02 local.
- Why the operator did NOT act: both pids are live and progressing; the charter
  forbids killing/restarting a live worker. Reaping is an owner/orchestrator item.
- Recommendation (REVERSES the 00:36Z call): keep slot 1 (it is on the packet
  branch, so its commit lands normally); reap slot 0 (detached HEAD @ c139afb,
  no branch to land on). Start here: kill pid 35628 + opencode 11492, then
  `python loop/run_packet.py --reset-only --slot 0` (archives the partial diff
  first). Do NOT re-dispatch FHC-TRIM - the registry row is READY/assigned None,
  so once a duplicate slot frees the dispatcher can start a THIRD worker.
- Why it matters now: the two diffs are near-identical (+92 lines) = redundant
  work; RAM is 1.3 GB free (below the 3 GB threshold) and slot 1 already took a
  0xC0000409 crash - the duplicate is actively over the RAM cap.
- Health this cycle: heartbeat 1 (27872), operator_runner 1 (27876), watchdog 1
  (29264), overnight driver 1 (24864); cargoq UP (ping ok, queued 0, running
  false); disk 26.88 GB free; RAM 1.3 GB free (below threshold).

## 2026-09-12 04:24 UTC - UPDATED: duplicate FHC-TRIM - active/stalled roles FLIPPED; the "keeper" call is unstable

- What changed: the 04:00Z escalation (keep slot 1, reap slot 0) is now STALE.
  Re-derived this cycle: slot 0 is the ACTIVE one (events ~2 min old, last
  cargoq `build --profile quick -p truck123d` exit 0 at 00:20:37 local, diff
  `bd_bridge.rs` +89/-4, DETACHED HEAD c139afb) and slot 1 is STALLED (events
  ~13 min old, last cargoq build exit 0 at 00:09:50 local, diff `bd_bridge.rs`
  +114/-1, packet branch 3d1d769). Both cmd+opencode pids remain alive; no
  cargo/rustc process exists.
- Why the operator did NOT act: both workers still hold live pids, and the
  charter forbids killing/restarting a live worker. The last three cycles gave
  OPPOSITE recommendations (00:13Z/00:36Z kept slot 0; 04:00Z kept slot 1)
  because the roles oscillate; a third flip is possible. Reaping is an
  owner/orchestrator item.
- Recommendation: do NOT act on any single cycle's keeper call. Decide from the
  two diffs and prefer the more complete one, ideally the one on the packet
  branch. Start here: `git -C loop/slots/0/wt diff` and
  `git -C loop/slots/1/wt diff`; then kill the loser's cmd pid + opencode child
  (slot 0: 35628/11492; slot 1: 13864/35128) and
  `python loop/run_packet.py --reset-only --slot <n>` (archives the diff first).
  Do NOT re-dispatch FHC-TRIM - the registry row is READY/assigned None, so once
  a duplicate slot frees the dispatcher can start a THIRD worker.
- Why it matters now: RAM 2.18 GB free (below the 3 GB threshold); the duplicate
  is over the RAM cap and slot 1 already took a 0xC0000409 crash earlier.
- Health this cycle: heartbeat 1 (27872), operator_runner 1 (27876), watchdog 1
  (29264), overnight driver 1 (24864); cargoq UP (ping ok, queued 0, running
  false); disk 26.7 GB free; RAM 2.18 GB free (below threshold).

## 2026-09-12 04:48 UTC - NEW: slot 0 FHC-TRIM-EXTRUDE-ENVELOPE finished with RESULT status SPEC_GAP (not landable) + UPDATED the duplicate is now slot-1-only

- What: slot 0 FINISHED the duplicate FHC-TRIM run with RESULT status
  **SPEC_GAP** (worker commit 33f6269 "truck123d: trim-extrude envelope
  extension", preserved on branch `packet/FHC-TRIM-EXTRUDE-ENVELOPE-slot0`; NOT
  an ancestor of HEAD; the overnight driver archived
  `loop/results/FHC-TRIM-EXTRUDE-ENVELOPE.PENDING.RESULT.json` and logged "slot 0:
  FHC-TRIM-EXTRUDE-ENVELOPE status='spec_gap' ... LEFT FOR MORNING (judgment
  required)" at 00:39 local). The RESULT's spec_gap: the envelope extension admits
  the shape exactly, but the exact planar fan cap carries a degenerate normal cone
  at the loop centroid, so the landed RDEF-M2 tangential sandwich admission
  refuses TYPED `SingularParametrization` (the booked FHC-EX-B degenerate-cap
  pair class). f1/rear_wing + f1/steering_rack GREEN; suspension_front/rear
  BLOCKED at composition.
- Why the operator did NOT land it: charter step 2 lands only RESULT status DONE
  with green scoped checks; SPEC_GAP is explicitly a do-not-land/escalate class.
  The worker's own note also records `fmt --check`/`clippy -D warnings` failures,
  but only on PRE-EXISTING drift outside the write set.
- Action needed (human/orchestrator): adjudicate the SPEC_GAP - decide whether the
  degenerate-fan-cap composition is in scope for FHC-TRIM or is a rebooking. The
  work is preserved on `packet/FHC-TRIM-EXTRUDE-ENVELOPE-slot0` and the RESULT at
  `loop/results/FHC-TRIM-EXTRUDE-ENVELOPE.PENDING.RESULT.json`.
- UPDATED duplicate: the FHC-TRIM duplicate is now SLOT-1-ONLY. Slot 1 (branch
  `packet/FHC-TRIM-EXTRUDE-ENVELOPE` @ 3d1d769=base, cmd pid 13864, opencode
  35128) is the sole live worker, ACTIVE (events ~4 min fresh, `M
  truck123d/src/bd_bridge.rs`, no commit). The 04:24Z "reap slot 0" recommendation
  is moot (slot 0 finished); do NOT apply it to live slot 1. If slot 1 also
  returns SPEC_GAP, the row stays READY and needs an authoring/rebooking decision.
- Health this cycle: heartbeat 1 (27872), operator_runner 1 (27876), watchdog 1
  (29264), overnight driver 1 (24864); cargoq UP (ping ok, queued 0, running
  false); disk 28.15 GB free; RAM 3.77 GB free (recovered above threshold).

## 2026-09-12 06:43 UTC - UPDATED (resolves the 04:48Z "if slot 1 also returns SPEC_GAP" branch): slot 1 FHC-TRIM FINISHED SPEC_GAP - BOTH duplicate runs SPEC_GAP; row READY needs a rebooking decision

- What: slot 1 FINISHED the duplicate FHC-TRIM-EXTRUDE-ENVELOPE run with RESULT
  status **SPEC_GAP** (worker commit `4a1dd49` on branch
  `packet/FHC-TRIM-EXTRUDE-ENVELOPE`, NOT an ancestor of HEAD; RESULT at
  `loop/slots/1/wt/RESULT.json`). Its own done-when is green
  (`cargo test -p truck123d --test trim_extrude_envelope --locked`, 4 passed,
  388 s), and the RESULT records f1/rear_wing + f1/steering_rack GREEN with
  suspension_front/rear BLOCKED at the composition. The spec_gap is the same
  booked FHC-EX-B degenerate-fan-cap class slot 0 hit: the exact planar fan cap
  carries a degenerate normal cone at the loop centroid, so the composition
  refuses TYPED (`unsupported_envelope`/`non_canonical_carrier`, or
  `ExtremeSlabContaminated` when the tool over-extends). No numeric shortcut, no
  trim approximation.
- Why the operator did NOT land it: charter step 2 lands only RESULT status DONE
  with green scoped checks; SPEC_GAP is explicitly a do-not-land/escalate class.
  Both duplicate runs now independently reach the same verdict, which is evidence
  the gap is real and not a worker error.
- Action needed (human/orchestrator): adjudicate the FHC-TRIM SPEC_GAP pair -
  decide whether the degenerate-fan-cap composition is in scope for FHC-TRIM or
  is a rebooking (the two duplicate runs agree). The registry row
  FHC-TRIM-EXTRUDE-ENVELOPE remains READY with NO landed marker and its slot
  assignment cleared (both slots FINISHED), so `dispatch_ready` can start a THIRD
  worker once it considers the row dispatchable - pin/rebook before that
  happens. Preserved work: slot 1 `4a1dd49` on
  `packet/FHC-TRIM-EXTRUDE-ENVELOPE`; slot 0 `33f6269` on
  `packet/FHC-TRIM-EXTRUDE-ENVELOPE-slot0` + archived
  `loop/results/FHC-TRIM-EXTRUDE-ENVELOPE.PENDING.RESULT.json`.
- Carried unchanged: duplicate supervisors (19172 + 27828); RG-23/RG-9 missing
  packet files; F1-AUTHORING-ARMS LANDED-WITH-FINDINGS; FRAME-REVOLVE F1
  non_z_axis pin; TOR-C flip-or-pin; slot-4/slot-7 wt RESULT residue.
- Health this cycle: heartbeat 1 (27872), operator_runner 1 (27876), watchdog 1
  (29264), overnight driver 1 (24864); cargoq UP (ping ok, queued 0, running
  false); disk 27.7 GB free; RAM 3.84 GB free.

## [operator 2026-09-12T13:33Z] LOW RAM: 1.3 GiB free with a worker warm build launching

- What: FreePhysicalMemory 1.3 GiB of 15.71 GB total, BELOW the 3 GB floor.
  Resident consumers are the owner's desktop (chrome + Teams msedgewebview2 +
  Dropbox) plus two opencode processes; the loop's own footprint is small. The
  heartbeat dispatched FHC-MIRROR-FORM to slot 0 at ~09:29 local and its warm
  build is starting into this pressure.
- Why it matters: janitor.ensure only kills opencode-parented language servers
  when free_ram_gb() < 4.0; it does NOT refuse dispatch on RAM. The documented
  RAM-zone signature (rustc exit 101 / 0xc0000409) already appears in the recent
  FHC-TRIM cargoq history (server.log), and a cold warm build is the 4-8 GB
  spike class.
- Action needed (human): close chrome/Teams/Dropbox if the FHC-MIRROR-FORM build
  crashes, or confirm the dispatcher should refuse below a RAM floor. Watch
  loop/cargoq/server.log for exit 101 / 3221225781 (0xc0000409) on the
  FHC-MIRROR-FORM build; if it fails, clean slot 0 target/ dirs and re-warm once.
- Do NOT raise the worker cap. This is the only new escalation; all carried items
  are unchanged (RG-23/RG-9 missing packet files; duplicate supervisors +
  duplicate cargoq/server.py + NEW duplicate http.server:8780 (3260 + 14452);
  F1-AUTHORING-ARMS LANDED-WITH-FINDINGS; FRAME-REVOLVE F1 non_z_axis pin;
  TOR-C flip-or-pin; slot-4/slot-7 wt RESULT residue).

## 2026-09-12 18:42 UTC (operator): FHC-FACTS-CACHE now running in BOTH slot 0 and slot 1 (the 18:09Z hazard fired)

- What: the 18:09Z escalation predicted that dispatch_ready's STALLED-as-free
  accounting would duplicate a live packet. It fired. The heartbeat
  (dispatch_heartbeat.ps1, pid 27872) recovered from its wedge and ran
  `dispatch_ready.py` LIVE at 14:40:09 local, seeing "slots: 8 (0 running, 7
  free)" and dispatching `FHC-FACTS-CACHE -> slot 1` while slot 0's
  FHC-FACTS-CACHE worker (cmd 16952, opencode 12864, session
  ses_f69c975a...) was ALIVE mid serial door.py pass (live child powershell
  25576 running facts_call_count.py on power_unit since 13:47:45 local;
  events stale 54.1 min by design). Slot 1 now runs a fresh
  FHC-FACTS-CACHE worker (cmd 29372, opencode 29476, session
  ses_f69141b95ffe...). Both write truck123d/src/bd_bridge.rs and
  truck123d/tests/facts_cache.rs.
- Why it matters: (1) two concurrent door.py measurement runs violate the
  serial-door rule and can flake; (2) the two branches collide on bd_bridge.rs
  at merge; (3) the heartbeat log at 14:19:30 had correctly deferred
  FHC-FACTS-CACHE on a bd_bridge.rs write-set clash, then at 14:40 saw
  "0 running" - the liveness detector lost slot 0 when its events aged past
  the <180s RUNNING override.
- Action needed (human/orchestrator): decide which run to keep. Recommended:
  keep slot 0 (55 min of door.py measurement already invested) and stop slot 1's
  fresh worker; but the charter forbids the operator from killing a live worker,
  so this is escalated, not done. If slot 0's child is genuinely hung (not just
  slow), the reverse. Start from `loop/slots/0/events.jsonl` mtime,
  `loop/slots/1/events.jsonl`, and `loop/dispatch_heartbeat.log`. Fix the
  underlying gap: STALLED (events stale) with a live opencode child must NOT be
  counted as free for dispatch, and the heartbeat should not run dispatch_ready
  live while any slot holds a live opencode child with stale events.
- Minor: slot 1's `worker.session` file records ses_f8b738b95ffe... but the
  actual worker session in its events is ses_f69141b95ffe... (run_packet wrote a
  session id the worker did not adopt). Harness bookkeeping only.
- Carried unchanged: RG-23/RG-9 missing packet files; duplicate supervisors
  (19172+27828) + duplicate cargoq/server.py (28544+34564); F1-AUTHORING-ARMS
  LANDED-WITH-FINDINGS; FRAME-REVOLVE F1 non_z_axis pin; TOR-C flip-or-pin;
  slot-2/slot-4/slot-7 wt RESULT residue; RAM below the 3 GB floor.

## 2026-09-12 14:40 UTC - LOW RAM persists and is now crashing the running worker's tests (carried escalation, worsened)

- What: FreePhysicalMemory 0.57 GiB of 15.71 GB total (was 1.21-1.3 GiB earlier
  today), still far below the 3 GB floor. The slot-0 BD-EMIT-MESH-CACHE worker is
  the only loop consumer; the rest is the owner's desktop (chrome + Teams + Dropbox
  + resident opencode).
- Why it matters: the RAM-zone signature is now material in the cargoq stats -
  `test -p truck123d --lib --locked` exited 3221225781 (0xC0000409) twice this
  window, and repeated `clippy -p truck123d --all-targets` exit 101s. The worker
  is still alive and progressing (do not touch), but a cold warm-build spike is
  the 4-8 GB class and could kill it mid-run.
- Action needed (human): free RAM (close chrome/Teams/Dropbox) before the next
  dispatch, or confirm the dispatcher should refuse below a RAM floor. If the
  slot-0 worker dies on 0xC0000409, clean `loop/slots/0/target` +
  `loop/slots/0/wt/target`, then let the heartbeat re-dispatch.
- Do NOT raise the worker cap. All other carried items unchanged (RG-23/RG-9
  missing packet files; duplicate supervisors 19172+27828 + duplicate
  cargoq/server.py 28544+34564; F1-AUTHORING-ARMS LANDED-WITH-FINDINGS;
  FRAME-REVOLVE F1 non_z_axis pin; TOR-C flip-or-pin; slot-4/slot-7 wt RESULT
  residue).

## 2026-09-12 15:50 UTC (operator): slot-0 FHC-FACTS-CACHE alive-but-stalled + bd_bridge.rs collision with newly-dispatched FHC-G2

- What: slot 0's FHC-FACTS-CACHE worker (cmd pid 34776) is ALIVE with a live
  opencode child (pid 3520, packet text loaded) but its events.jsonl has been
  stale ~13.7 min with changed=0, no commit, and no cargo/rustc process - i.e.
  idle, not the dead-shim signature (a dead shim has no opencode child). This
  cycle's `dispatch_ready --max-workers=4` counted slot 0 as not-running and
  dispatched FHC-G2-PROBE-QUERIES to slot 1. Both packets write
  `truck123d/src/bd_bridge.rs`; if the stalled FACTS-CACHE worker later commits
  and G2 also commits, the two branches collide at merge.
- Why it matters: (1) reaping a genuinely-live worker is forbidden by the
  charter; (2) if it is actually wedged, its slot stays unavailable and its
  bd_bridge.rs work may be lost; (3) the collision is the write-set-disjointness
  rule being overridden by the dispatcher's STALLED-as-free accounting.
- Action needed (human/orchestrator): decide whether the FACTS-CACHE worker is
  wedged. If its events stay stale > 20 min with no cargo, reset slot 0
  (`run_packet.py --reset-only --slot 0`, archiving first) and let FHC-FACTS-CACHE
  re-dispatch AFTER FHC-G2 lands, so the bd_bridge.rs write sets serialize. Start
  from `loop/slots/0/events.jsonl` mtime and `loop/slots/0/wt` git status.
  All other carried items unchanged.

## 2026-09-12 18:09 UTC (operator): dispatch_ready dead-dispatch reset would clobber the two live workers; heartbeat wedged

- What: `dispatch_ready --dry-run` now classifies BOTH live slots as dead:
  slot 0 FHC-FACTS-CACHE (cmd 16952, opencode 12864, live door.py children) and
  slot 1 FHC-G2-PROBE-QUERIES (cmd 20612, opencode 27780). Both have no
  RESULT.json and events >3 min stale (19.3 / 13.3 min), so the session-54
  <180s RUNNING override in dispatch_ready.slot_states() does not fire and they
  fall into the `dead` set.
- Why it matters: in LIVE mode dispatch_ready's dead branch calls
  `run_packet.py --reset-only` (archive-and-hard-reset the worktree) and then
  re-dispatches the packet into the same slot - spawning a SECOND worker while
  the first is still running, and double-writing truck123d/src/bd_bridge.rs
  (both packets write it). The in-progress door.py facts measurement would be
  archived and the live worker left writing into a reset tree.
- The heartbeat (dispatch_heartbeat.ps1) runs `dispatch_ready.py
  --max-workers=3` LIVE every 10 min. It is currently WEDGED (last cycle
  13:48:37 local, no python child at 14:09), which is the only reason the reset
  has not fired. DO NOT restart the heartbeat while slots 0/1 are alive.
- Action needed (human/orchestrator): (1) do not run dispatch_ready live until
  both workers commit or finish; (2) decide whether slot_status's STALLED
  (events >3 min) should suppress the dead-dispatch path when a live opencode
  child exists (the dead-shim signature is the absence of that child); (3)
  investigate why heartbeat 27872 stopped cycling at 13:48 local. Start from
  loop/slots/0/events.jsonl, loop/slots/1/events.jsonl, and
  loop/dispatch_heartbeat.log.
- Also: RAM 1.79 GB free (below the 3 GB floor). Do not raise the worker cap.
  All other carried items unchanged (RG-23/RG-9 missing packet files; duplicate
  supervisors 19172+27828 + duplicate cargoq/server.py 28544+34564;
  F1-AUTHORING-ARMS LANDED-WITH-FINDINGS; FRAME-REVOLVE F1 non_z_axis pin;
  TOR-C flip-or-pin; slot-4/slot-7 wt RESULT residue).

## 2026-09-12 19:07 UTC (operator): the 18:09Z dead-dispatch reset hazard is now ARMED (heartbeat live + 0-running classification); slot-1 live worker at risk

- What: `dispatch_ready --dry-run --max-workers=4` NOW reports "slots: 8 (0 running,
  7 free)" and for `FHC-FACTS-CACHE` prints: "DEAD dispatch (slot 1 holds no matching
  RESULT) - would reset + delete + redispatch", then "FHC-G7-REFUSAL-METADATA -> slot
  0; dispatched 1". Slot 1's worker is ALIVE and mid-work: cmd 29372 +
  opencode.exe 29476 (session ses_f69141b95ffe...) + live powershell/python
  facts_call_count.py children (2076) running a serial door pass over
  slots/1/wt/corpus/ttc; wt dirty (M truck123d/src/bd_bridge.rs, ?? tests/facts_cache.rs).
- Why it matters: this is the exact 18:09Z hazard, but the two conditions that kept
  it dormant are both gone. (1) The heartbeat 27872 is no longer wedged - it is LIVE
  and cycling (log entries 14:50:35 and 15:00:39 local, `--max-workers=3` LIVE). (2)
  slot_status no longer sees any RUNNING slot (slot 0's worker died; slot 1 reads
  STALLED because the door pass writes no events), so the <180s RUNNING override at
  dispatch_ready.py:96 does NOT fire. In LIVE mode dispatch_ready.py:176-180 runs
  `run_packet.py --slot 1 --reset-only`, hard-resetting slot 1's worktree under the
  live worker, then line 185 frees the slot and the packet is re-dispatched -> a
  SECOND FHC-FACTS-CACHE worker writing a reset tree. The heartbeat's next cycle
  (~15:10 local) fires this. NOTE: FHC-G7's write set
  (corpus/ttc/door.py, truck123d/src/binding.rs, marshal.rs, tests/refusal_metadata.rs,
  docs/REFUSALS.md, README.md) is disjoint from FHC-FACTS-CACHE, so that dispatch is
  legitimate on its own - the reset of slot 1 is the harm.
- Action needed (human/orchestrator): (1) STOP the live reset path before the next
  heartbeat cycle - either pause/repoint the heartbeat (do NOT kill the worker), or
  patch dispatch_ready.py so a slot with a live opencode child under its worker pid
  is never classified dead (the dead-shim signature is the ABSENCE of that child, not
  stale events); (2) the FHC-FACTS-CACHE row itself: slot 0 died leaving worker commit
  2a0d581 (bd_bridge.rs +132, facts_cache.rs +483; NOT an ancestor, NO RESULT.json) -
  PRESERVED at refs/wip/FHC-FACTS-CACHE-2a0d581; slot 1 will produce the same packet.
  Decide: let slot 1 finish and land, or recover 2a0d581 (the FHC-G2/G3 post-recycle
  recovery pattern). (3) RAM 2.36 GB free, below the 3 GB floor - do not raise the cap.
- Start here: loop/dispatch_ready.py:70-119 (slot_states dead set) and :165-185 (dead
  branch); loop/dispatch_heartbeat.ps1; loop/slots/1/events.jsonl; git show 2a0d581.
- Carried unchanged: RG-23/RG-9 missing packet files (anchor check fails = missing
  authoring, not drift); duplicate supervisors 19172+27828 + duplicate cargoq/server.py
  28544+34564; F1-AUTHORING-ARMS LANDED-WITH-FINDINGS; FRAME-REVOLVE F1 non_z_axis
  pin; TOR-C flip-or-pin; slot-2/4/7 wt RESULT residue.

## 2026-09-12 20:05 UTC (operator) - nothing new; carried + one caution

- RESOLVED this cycle (recorded, no human action): FHC-G7-REFUSAL-METADATA was
  half-landed by the live orchestrator (merge f999c7d, but no RESULT filing / ledger /
  row flip). Operator completed the bookkeeping (6985805) after a green scoped check.
- RESOLVED this cycle: FHC-G4 A1 anchor drift 1->2 (FHC-G7 door.py landing); re-measured
  and committed (647b756); G4 is now dispatchable.
- CAUTION: RAM 2.54 GB free, below the 3 GB floor, with no worker resident. The FHC-G4
  warm build may hit 0xc0000409. If the heartbeat's next cycle fails the warm build,
  the documented clean-target-and-retry-once applies; a second failure needs a human.
- Carried unchanged: RG-23/RG-9 packet .md files ABSENT (missing authoring, not drift);
  duplicate supervisors + duplicate cargoq/server.py; F1-AUTHORING-ARMS
  LANDED-WITH-FINDINGS; FRAME-REVOLVE F1 non_z_axis pin; TOR-C flip-or-pin; slot-2/4/7
  wt RESULT residue.

## 2026-09-12 19:35 UTC (operator): CLOSED - the 18:09Z/19:07Z dead-dispatch reset hazard is resolved; FHC-FACTS-CACHE landed

- Resolution: no live worker remained. The slot-1 worker (cmd 29372) exited and the
  slot-0 fresh re-implementation worker (cmd 24000 / opencode 22596) died ~15:31; the
  ORCHESTRATOR landed the operator-preserved orphan 2a0d581 (merge b05a753, row DONE
  a8118b5) and cleared the redundant slot-0 worker. So the "reset a live slot mid-write"
  harm never fired. No operator action was required or taken.
- Residual (low priority, no action needed): the dead slot-0 worker's WIP archive
  loop/slots/0/abandoned-20260912-153155.patch is INCOMPLETE - it lacks the untracked
  truck123d/tests/facts_cache.rs (the untracked-files-not-archived recycle trap). It is
  moot because the landed orphan 2a0d581 carries that file, but the archive gap is a
  real machinery defect worth fixing in run_packet's reset path.
- Dispatch state: FHC-G7-REFUSAL-METADATA is the next READY packet. The heartbeat's
  15:24 cycle failed its slot-2 warm build (0xc0000409 / exit 101, RAM-zone); RAM is
  now 3.34 GB free (just above the 3 GB floor) so the heartbeat should succeed next
  cycle. Do NOT run dispatch_ready live (heartbeat owns dispatch).
- Carried unchanged: RG-23/RG-9 missing packet files; duplicate supervisors + duplicate
  cargoq/server.py; F1-AUTHORING-ARMS LANDED-WITH-FINDINGS; FRAME-REVOLVE F1 non_z_axis
  pin; TOR-C flip-or-pin; slot-2/4/7 wt RESULT residue.

## 2026-09-12 21:54 UTC (operator): NEW - registry has 3 byte-identical duplicate rows

- What: `loop/PACKETS.jsonl` is 359 lines but 356 unique IDs. Three IDs appear twice,
  byte-identical and both status DONE: MONO-9-FUSE-FOLD, RDEF-M1-LATTICE-V2,
  FHC-G7-REFUSAL-METADATA. Unique count (356) is unchanged, so dispatch semantics are
  unaffected (dispatch_ready reads unique IDs; --dry-run is correct), but the file is
  now inconsistent with the "356 unique" claim in STATE and the duplicate lines will
  keep inflating any line-count census.
- Why I did not fix it: dedup is a registry edit outside the operator's three-file
  limit (STATE/LOG/ESCALATIONS) and is not one of the documented mechanical flips
  (BLOCKED->READY, anchor re-measure, yaml lint fix). Cost asymmetry says escalate.
- Exact command a human should start from:
  `python -c "import json,collections; rows=[json.loads(l) for l in open('loop/PACKETS.jsonl',encoding='utf-8') if l.strip()]; seen=set(); out=[]; [out.append(r) for r in rows if not (r['id'] in seen or seen.add(r['id']))]; open('loop/PACKETS.jsonl','w',encoding='utf-8').write('\n'.join(json.dumps(r,ensure_ascii=False) for r in out)+'\n')"`
  (drops later duplicates, keeps first occurrence; verify the 3 IDs first).
- Priority: low. No dispatch impact observed.

## 2026-09-12 22:23 UTC (operator): NEW - overnight driver wedged mid-scoped-check; FHC-G5 committed-unlanded

- What: slot 0 holds FHC-G5-SWALLOWED-REFUSAL-DIAGNOSIS with RESULT status DONE but no
  worker commit (skipped-commit class). The overnight driver (pid 24864) began landing
  it and is now WEDGED inside `scoped_check`: `cargo.exe` pid 37656
  (`cargo test --locked -p truck123d --test extraction_breadth_a --manifest-path
  C:\Users\stefa\look\loop\slots\0\wt\Cargo.toml`) has 0 CPU, no `rustc` child, and has
  been idle >8 min; `loop/overnight.log` has logged nothing since 18:10:39 (its last
  poll). `overnight.py`'s `sh()` default timeout is 3600 s, so the driver may stay
  wedged up to an hour, not landing G5 and not polling.
- Operator action already taken: to prevent loss to a re-fork (untracked files are not
  archived), the five write_allow files were committed AS DELIVERED at `e69f404` (1 ahead
  of integration, NOT merged). RESULT.json remains at `loop/slots/0/wt/RESULT.json` for
  filing. The branch is `packet/FHC-G5-SWALLOWED-REFUSAL-DIAGNOSIS`.
- Why I did not land it: the operator scoped check crashed environmentally
  (`cargo check -p truck123d --tests --locked` exit 101 / 0xC0000409, RAM-zone, at
  2.x GB free and competing with the driver's build); no green scoped check exists yet.
  The packet's declared done-when also includes `cargo fmt --check -p truck123d`, which
  the cargoq server log records as exit 1 (the worker asserts pre-existing toolchain
  drift outside write_allow - that adjudication is not an operator call).
- Exact commands a human should start from:
  1. Inspect the wedge: `Get-Process -Id 37656 | Select Id,CPU` and
     `Get-Content loop/overnight.log -Tail 20`.
  2. If wedged, kill 37656 and the driver's stuck wait, or let the 3600 s timeout fire.
  3. Land from the committed branch once RAM permits (close chrome/Dropbox/Discord -
     baseline is heavy at 15.7 GB):
     `git -C C:\Users\stefa\look merge --no-ff --no-edit -m "merge: FHC-G5 ... (scoped check green)" e69f404`,
     then file `loop/results/FHC-G5-SWALLOWED-REFUSAL-DIAGNOSIS.json`, remove the
     worktree-root RESULT.json, append the ledger row, flip the registry row DONE.
- Priority: HIGH - G6, G1, FHC-B, FHC-C are all chained behind G5; the frontier is
  stalled until it lands or is adjudicated. `overnight.py scoped_check` should also get a
  bounded per-command timeout smaller than 3600 s (machinery defect).

## 2026-09-12 22:46 UTC (operator): RESOLVED - the 22:23Z G5-unlanded/driver-wedge item

- Resolution: the overnight driver (24864) recovered on its own and landed G5. `loop/
  overnight.log` records "09-12 18:38:22 slot 0: FHC-G5-SWALLOWED-REFUSAL-DIAGNOSIS
  LANDED at e69f404"; the merge is `18e44ef`, the registry row is flipped (`44e769f`),
  and `e69f404` is an ancestor of HEAD. Slot 0 was re-forked to the frontier FHC-G6 at
  18:41:55 local, which is RUNNING healthy. No human action required for the landing.
- Still worth a look (machinery, not blocking): `overnight.py scoped_check`'s 3600 s
  unbounded default let the driver appear wedged for ~28 min; a bounded per-command
  timeout would make a future wedge visible sooner.
- Priority: closed (landing); the timeout note is low.

## 2026-09-12 23:18 UTC (operator): NEW HIGH - frontier FHC-G6 cannot re-dispatch; warm builds fail below-floor RAM

- What: slot 0's FHC-G6-CERT-COST-SCALE worker died (events 20+ min stale, no
  cargo/rustc, CPU idle) and was already reset by the dispatcher at 19:02:08, but
  its orphaned `cmd.exe`+opencode tree kept slot 0's 13 GB target marked LIVE.
  Operator killed the orphan (taskkill /PID 15368 /T /F) and ran
  `python loop/janitor.py ensure --need 8` (reclaimed ~15.5 GB -> 20.7 GB; all slot
  targets cleared). The heartbeat's auto re-dispatch STILL failed: G6 -> slot 1
  warm build exit 101 (`target-lexicon` 180 errors), FHC-B -> slot 2 warm build
  0xc0000409 STATUS_STACK_BUFFER_OVERRUN. Both are the stale-target + RAM-zone
  signature. The heartbeat (27872) started its 19:14:41 cycle and was warming the
  clean targets at handoff with RAM spiked to 1.05 GB free - outcome not yet known.
- Why it needs a human: RAM is below the 3 GB floor (heavy baseline: chrome +
  Dropbox + Discord + MsMpEng + 2 opencode) and the workspace warm-build spike
  alone is 4-8 GB, so warm builds crash (0xc0000409) before the frontier can
  dispatch. The operator cannot close human apps.
- Exact commands a human should start from:
  1. `Get-Process | Sort-Object WorkingSet64 -Descending | Select -First 12 Name,Id,@{n='MB';e={[math]::Round($_.WorkingSet64/1MB)}}`
  2. Close chrome/Dropbox/Discord (frees ~1.2 GB), then
     `python loop/dispatch_ready.py --dry-run --max-workers=4` to confirm G6 is
     READY and unclashed.
  3. Let the heartbeat (27872) dispatch G6; if the warm build fails again with
     0xc0000409, raise `CARGO_BUILD_JOBS`/lower concurrency or prewarm one slot
     at a time.
  4. If the `target-lexicon` 180-error build recurs on CLEAN targets, that is a
     toolchain/registry issue, not RAM - investigate `cargo check -p target-lexicon`
     in a slot.
- Priority: HIGH - G6 is the frontier; G1/FHC-B/FHC-C are chained behind it.

## 2026-09-12 23:18 UTC (operator): NEW LOW - RG-23 / RG-9 READY rows have empty `packet` fields (unauthored)

- What: `loop/PACKETS.jsonl` rows RG-23-CERTIFIED-ENTRY-WIRING and
  RG-9-REFLECT-SOLID-PRODUCTION are status READY but carry `"packet": ""`. The
  heartbeat's dispatcher prints "ANCHOR CHECK FAILED" for them every cycle; the
  real cause is `gen_packet.py --check <empty>` raising FileNotFoundError, not
  anchor drift. No packet file exists under loop/packets/ for either.
- Why operator did not fix: new-packet authoring is not an operator call.
- Exact commands a human should start from:
  `Select-String -Path loop/PACKETS.jsonl -Pattern 'RG-23-CERTIFIED-ENTRY-WIRING|RG-9-REFLECT-SOLID-PRODUCTION'`
  then author the packets (or mark the rows BLOCKED/parked if the RG shipping
  wave is no longer intended). Both write sets clash with G6's
  `truck123d/src/bd_bridge.rs` in any case.
- Priority: low. No dispatch impact until G6 lands and frees the write set.

## 2026-09-13 00:38 UTC (operator): LOW/MED - FHC-B scoped-check verdict disagrees between driver and operator; driver landing it now

- What: slot 1 FHC-B-MULTI-CONTOUR-SECTIONS finished with RESULT status DONE and
  commit a43f7fc (not yet an ancestor of integration/kernel-bg). The operator's
  independent re-run of the packet's named test is GREEN (cargoq server.log
  2026-09-13 20:38:16 DONE exit=0 in 651s; worker's own 20:06:02 exit=0). But the
  overnight driver's 20:21:48 cycle logged "slot 1: scoped check NOT green (check
  -p truck123d failed); left for morning". The packet's other done-when items
  (cargo fmt --check -p truck123d, cargo clippy -p truck123d --all-targets -- -D
  warnings) are KNOWN baseline-broken per RESULT notes (fmt: pre-existing
  out-of-scope truck123d/tests/ttc_hazard_battery.rs; clippy: 383 pre-existing
  errors in bd_bridge.rs, 0 new). Also seen: cargo test -p truck123d --lib exited
  3221225781 (0xC0000409) twice at 20:15 under RAM pressure.
- Why operator did not land: the driver (pid 24864) is ACTIVELY running the same
  named test as its landing check (cargo pid 18012), so a merge now would risk a
  double-merge. Left to the driver.
- Exact command a human should start from if the driver parks it again:
  git -C loop/slots/1/wt log --oneline -1 (expect a43f7fc) then run the named test
  and, if green, merge --no-ff; or confirm which of the driver's scoped checks
  failed (loop/overnight.log 20:21:48; loop/cargoq/server.log around 19:55-20:21).
- Priority: low/medium. If a43f7fc is genuinely green, one merge unblocks FHC-G6.

## 2026-09-13 01:01 UTC (operator): MED - FHC-G6 warm build dies 0xc0000409 (RAM); FHC frontier held until RAM frees

- What: the heartbeat's 20:52:55 local cycle attempted FHC-G6-CERT-COST-SCALE -> slot 0
  and new_slot's warm build (`cargo check --workspace --all-targets`) failed exit 101 with
  `0xc0000409 STATUS_STACK_BUFFER_OVERRUN` building lzma-sys (loop/dispatch_heartbeat.log
  20:52:55). FHC-G6 is now the frontier (FHC-B landed at 94ba60b) and FHC-G1 waits on it.
- Why operator did not clean+re-warm: the charter's step-5 recipe is clean targets + re-warm
  once, but the failure signature is the RAM-zone one (ORCHESTRATOR: "a rustc 0xc0000409
  anywhere is the sign the inequality is violated - shrink, do not retry blindly"). RAM is
  2.28 GB free (BELOW the 3 GB floor) with FHC-C resident in slot 1; a cold workspace re-warm
  is a 4-8 GB spike and would likely crash again and/or disturb the live worker. Cleaning
  targets would only make the next attempt colder, not safer.
- Exact commands a human should start from: free RAM (close chrome/Dropbox/Discord or the
  second opencode), then `python loop/dispatch_ready.py --max-workers=4` (or let heartbeat
  27872 retry) and watch for `0xc0000409` in loop/dispatch_heartbeat.log; if it persists on a
  quiet machine, reduce max-workers and/or prewarm slot 0 once by hand.
- Priority: medium. The FHC chain is stalled at G6 until a warm build succeeds; nothing else
  on the board is dispatchable.

## 2026-09-13T01:24Z - operator carry (cycle 2): FHC-G6 frontier doubly held
- Same G6 warm-build 0xc0000409 / RAM-below-floor item, re-observed and CARRIED; no new
  action. New detail this cycle: `dispatch_ready --dry-run` also reports FHC-G6 write-set
  CLASHING with the RUNNING FHC-C-AUTHORING-FIDELITY on `truck123d/src/bd_bridge.rs`, so even
  with RAM restored G6 cannot dispatch until FHC-C frees its slot. The heartbeat (27872) owns
  live dispatch and will retry G6 when the clash clears. Start from: let FHC-C finish; then
  `python loop/dispatch_ready.py --max-workers=3` (or the heartbeat's next cycle) and watch
  loop/dispatch_heartbeat.log for `0xc0000409`. Priority: medium (carried).

## 2026-09-13T03:33Z - operator carry (cycle 3): FHC-G6 frontier - slot-0 target corruption cleaned, RAM still below floor

- What: FHC-C landed (eb66295 -> 39f65ba), so the G6 write-set clash on truck123d/src/bd_bridge.rs is CLEARED. The remaining blocker is the warm build, which now fails with a CORRUPTION signature rather than the earlier 0xc0000409: heartbeat cycles 21:34/23:12/23:22 local show E0463 "can't find crate for clap" (look lib test), 211 errors in test certified_phase2_floor ("map is implemented but not in scope"), 11/41 errors in examples stageb_probe/nist1167_dist, and 1 error in mesh_correctness_census - a different spurious error each run. Slot-0 target was incomplete (0.98 GB vs slot-1's 4.2 GB) after the 20:31-21:45 0xc0000409 crashes; cargo treats the partial artifacts as fresh, so the corruption persists across warm-build retries.
- What the operator did: removed loop/slots/0/target (documented step-5 clean). Did NOT re-warm: RAM 1.5 GB free (BELOW the 3 GB floor; janitor `ram` killed 1 opencode-parented rust-analyzer and RAM stayed ~1.5 GB); a cold `cargo check --workspace --all-targets` is a 4-8 GB spike and would 0xc0000409 again.
- Why escalate: the re-warm is a RAM-bound action the operator cannot make safe; per the charter's cost asymmetry, shrink rather than retry blindly. The live heartbeat (27872, --max-workers=3) will retry G6 each 10-min cycle and may re-corrupt the target on another crash.
- Exact command a human should start from: free RAM (close chrome/Dropbox/Discord and/or the second opencode, ~2-3 GB), then let heartbeat 27872's next cycle retry, or run `python loop/dispatch_ready.py --max-workers=3`; watch loop/dispatch_heartbeat.log for `0xc0000409`. Slot 0's target is now clean, so the first retry after RAM is freed should be a genuine cold build. Priority: medium (frontier; FHC-G1/FHC-D/FHC-E all queue behind G6).

## 2026-09-13T03:51Z - operator carry (cycle 4): FHC-G6 frontier RESOLVED

- The 03:33Z item is RESOLVED; no human action needed. The heartbeat (27872) dispatched
  FHC-G6-CERT-COST-SCALE to slot 0 at ~03:47Z after last cycle's `loop/slots/0/target` clean;
  the cold warm build succeeded (`dispatch_heartbeat.log`: "FHC-G6-CERT-COST-SCALE -> slot 0
  ... dispatched 1; workers now ~1/3") and the worker is live and progressing (cmd pid 2324,
  session `ses_f671f0eb4ffer7zIT6yKdcOe6R`, events 822 KB / 0.0 min fresh). RAM recovered to
  3.81 GB (above the 3 GB floor); disk 16.1 GB free. No operator action taken.
- Carried non-G6 items remain: RG-23/RG-9 unauthored (packet:''); duplicate supervisors
  (19172 + 27828) + duplicate cargoq/server.py (28544 + 34564); F1-AUTHORING-ARMS
  LANDED-WITH-FINDINGS; FRAME-REVOLVE F1 non_z_axis pin; TOR-C flip-or-pin; slot-4/7 wt
  RESULT residue. Priority: none (informational closure).

## 2026-09-13T14:49Z - operator cycle 8: DUPLICATE FHC-G6 dispatch (slot 0 active + slot 1 stalled-alive)

- What: the heartbeat (27872) re-forked FHC-G6-CERT-COST-SCALE into slot 0 at ~10:30 local while
  the original G6 worker in slot 1 was still live. NOW TWO live workers on the same packet:
  - slot 0 RUNNING/ACTIVE: cmd pid 35812 -> opencode 14252 (session
    `ses_f64d2e588ffeYAPndGa9CJ43Gw`), worktree on `packet/FHC-G6-CERT-COST-SCALE`@45575f6;
    cargoq is running `cargo test --profile quick -p truck123d --lib tmp_microbench_patch_cert`
    with cwd=`loop/slots/0/wt` (server.log 10:44:11), events mtime 10:44:10 local - progressing.
  - slot 1 STALLED-but-ALIVE: cmd pid 25232 -> opencode 26668 (session
    `ses_f650755e6ffeXNsMpdBvyxH7IM`), detached at `fd40760`; NO RESULT/commit/QUESTION, last
    cargo job DONE 10:32:14, events frozen 10:32:22 local (~17 min quiet at scan).
- What the operator did: NOTHING destructive. Did NOT kill or reset either worker: both opencode
  processes are alive, and the charter forbids killing a live worker / resetting the shared
  packet branch. slot 0 covers the frontier, so no unblock is missed.
- Why escalate: two live workers on one packet can each write a RESULT/commit and diverge; the
  duplicate is a dispatch/heartbeat judgment call, not a control-flow action. Also the heartbeat
  has now re-forked G6 repeatedly (09:38Z, 14:03Z, this cycle) without cleaning the prior slot.
- Exact command a human/orchestrator should start from: decide which worker is authoritative
  (slot 0 is active; slot 1 is hung). To reclaim slot 1 safely, kill opencode pid 26668 + cmd
  pid 25232, then `python loop/run_packet.py --slot 1 --reset-only`; or leave it and let it be
  reaped. Watch `loop/slots/0/events.jsonl` for slot 0's RESULT/commit. Priority: medium-high
  (frontier; G1/D/E queue behind G6).

## 2026-09-13T15:11Z - operator cycle 9: duplicate FHC-G6 CARRY (still open, no change)

- The 14:49Z duplicate-FHC-G6 item remains OPEN and unchanged; no new decision needed beyond it.
  Re-derived this cycle: slot 0 still ACTIVE (cmd 35812 -> opencode 14252, events 10.9 min old,
  uncommitted `truck123d/src/bd_bridge.rs`, no RESULT/commit/QUESTION, cargoq running=false) and
  slot 1 still STALLED-but-ALIVE (cmd 25232 -> opencode 26668, events 14.3 min old, uncommitted
  `bd_bridge.rs`, no RESULT/commit/QUESTION). Both opencode processes alive; operator took no
  destructive action (slot 0 covers the frontier). Slot 1 is approaching the 15-min dead-worker
  threshold with zero cargo/rustc activity, so it is the natural reclaim candidate.
- Start from the 14:49Z item above (same kill+`--reset-only` recipe). Priority: medium-high
  (carried; G1/D/E queue behind G6).

## 2026-09-13T15:37Z - operator cycle 10: duplicate FHC-G6 now DEAD (slot 1) + frontier deadlocked on slot 0's held branch

- What: the 14:49Z/15:11Z duplicate item is now SHARPENED. slot 0 is CONFIRMED WORKING - its
  events are 33.9 min old, but a live child chain runs the packet's expensive measurement:
  cmd 35812 -> opencode 14252 (session `ses_f64d2e588ffeYAPndGa9CJ43Gw`) -> powershell 16492 ->
  `python 24408` executing `Measure-Command { python measure_row.py ... build_suspension_rear }`
  at ~1 core (CPU 1634.9 -> 1640.8 s over 6 s, WS 237 MB). Do NOT kill or reset slot 0.
- slot 1 is now a DEAD duplicate: opencode 26668 (session `ses_f650755e6ffeXNsMpdBvyxH7IM`) /
  cmd 25232 are alive but have NO child process, no cargo/rustc, no RESULT/commit/QUESTION, and
  events frozen 37.3 min. `dispatch_ready --dry-run` flags it: "FHC-G6-CERT-COST-SCALE: DEAD
  dispatch (slot 1 holds no matching RESULT) - would reset + delete + redispatch".
- Why the operator did not just do it: (a) the hard limit forbids killing a worker (only own
  timed-out predecessor's leftovers); (b) resetting slot 1's worktree while its opencode is alive
  is the MONO-2 2026-09-10 live-reset hazard. And the redispatch would fail anyway: branch
  `packet/FHC-G6-CERT-COST-SCALE` is held by slot 0's dirty worktree, so the heartbeat's own
  cycle logs `new_slot FAILED: branch ... held by loop/slots/0/wt (dirty=True, at_tip=True);
  release it manually` (dispatch_heartbeat.log 11:30:58 local). The frontier is deadlocked on
  slot 0's legitimate in-progress work.
- Exact command a human/orchestrator should start from: kill opencode pid 26668 + cmd pid 25232,
  then `python loop/run_packet.py --slot 1 --reset-only` (archive-and-reset, no spawn). Leave
  slot 0 alone; it is the frontier and is actively measuring. When slot 0 finishes/commits, the
  branch frees and the heartbeat can dispatch the next FHC link. Priority: medium-high (frontier;
  FHC-G1/D/E queue behind G6).
- Carried human items unchanged: RG-23/RG-9 unauthored (empty `packet`, only
  RG-4-CANONICAL-BOOLEAN-PRODUCT.md exists in loop/packets); duplicate supervisors (19172 + 27828)
  + duplicate cargoq/server.py (28544 + 34564 + 31804); slot-4/7 wt RESULT residue; FRAME-REVOLVE
  F1 non_z_axis pin (ttc_lathe_spline.rs:255); TOR-C flip-or-pin (orchestrator-held).

## 2026-09-13T16:02Z - operator cycle 11: duplicate FHC-G6 CARRY (slot 1 now clean; blocker unchanged)

- Update to the 15:37Z item: slot 1's processes are now GONE. opencode 26668 and cmd 25232 no
  longer exist; slot 1's worktree is clean (changed=0, detached `fd40760`), no
  RESULT/commit/QUESTION. So the 15:37Z kill step is moot and `python loop/run_packet.py
  --slot 1 --reset-only` is now a NO-OP ("slot 1 is clean; nothing to reset") - do not bother.
- The remaining blocker is UNCHANGED and is the only thing holding the frontier: branch
  `packet/FHC-G6-CERT-COST-SCALE` is held by slot 0's dirty worktree (slot 0 is the legitimate
  in-progress G6 worker; do NOT touch it). The heartbeat's redispatch of the dead slot-1 G6
  cannot complete until slot 0 commits/finishes and frees the branch. Nothing for the operator to
  do beyond carrying this.
- Exact command a human/orchestrator should start from: none needed now; wait for slot 0 to
  commit, then the branch frees and the heartbeat dispatches FHC-G1. Priority: medium (frontier;
  FHC-G1/D/E queue behind G6). All other carried items unchanged (RG-23/RG-9 unauthored;
  duplicate supervisors + cargoq/server.py; slot-4/7 wt RESULT residue; FRAME-REVOLVE F1 pin;
  TOR-C).

## 2026-09-13T17:11Z - operator cycle 14: slot 0 FHC-G6 STALLED - measurement processes are ORPHANS (kill/reset-or-wait decision)

- NEW EVIDENCE (corrects the 16:48Z "live measurement children" and every carried "actively
  measuring" read): slot 0's opencode 14252 is alive but CPU-FLAT for 79+ min (events.jsonl
  frozen at 11:52:06 local; +0.03 s CPU over a 6 s sample). The four python processes it launched
  are ORPHANS - every parent shell is DEAD (34764/24476/1872). They spin ~1 core each and have
  produced NO output: measure_row.py's `susp_rear_rel.stl` and probe.py's `power_unit.stl` are
  ABSENT from `%TEMP%/opencode` (newest `*.stl` there is beam_wing_rel.stl @ 11:51:05).
- Why the operator took no action: the opencode worker process is alive, and the charter forbids
  killing a live worker; the branch `packet/FHC-G6-CERT-COST-SCALE` is held by slot 0's dirty
  worktree, so the frontier (FHC-G6 -> FHC-G1 -> FHC-D -> FHC-E) cannot advance. A wrong reset
  discards the only in-progress G6 work; a right one frees the branch. Judgment call -> escalate.
- Two options for a human/orchestrator:
  (a) WAIT (prior posture): if the four measurements are legitimately long, they eventually finish
      and the worker commits; the frontier unblocks on its own.
  (b) UNBLOCK NOW: kill the four orphaned python pids, then `run_packet --reset-only` on slot 0
      (archives the dirty `bd_bridge.rs`) to free the branch; the heartbeat can then redispatch
      FHC-G6 fresh or FHC-G1. Confirm opencode 14252 is truly idle (CPU flat, no new events) first.
- Exact command to start from: `Get-Process -Id 29552,31800,38936,35748 | Stop-Process` then
  `python loop/run_packet.py --slot 0 --reset-only`. Priority: high (frontier; G1/D/E queue behind
  G6 and the branch has been held ~2.5 h).
- Carried human items unchanged: RG-23/RG-9 unauthored (packet files missing); duplicate
  supervisors (19172 + 27828) + duplicate cargoq/server.py (28544 + 34564 + 31804); slot-4/7 wt
  RESULT residue; FRAME-REVOLVE F1 non_z_axis pin; TOR-C flip-or-pin.

## 2026-09-13T17:35Z - operator cycle 15: FHC-G6 recovered + progressing (17:11Z stall superseded); orphan probe + RG-23/RG-9 authoring carried

- UPDATE to the 17:11Z item: the orchestrator recovered slot 0 (WIP commit `0429294`) and
  re-forked at 13:17:55 local. The fresh worker is HEALTHY: opencode 26336 with a live
  measurement child 25388 (`measure_row.py ... build_suspension_rear`; ancestry 25388->4272->26336
  confirmed live) and it produced `track_rod_post.stl` + `beam_wing_post.stl` at 13:21 before the
  measurement. So the 17:11Z "STALLED/orphans" kill/reset list is SUPERSEDED: 29552/31800/38936
  are gone and slot 0 must NOT be reset (it holds the frontier's only in-progress work and is
  progressing).
- Remaining leak: orphan `35748` (`probe.py f1 lib.power_unit`, parent 1872 dead) still spins ~1
  core (12.6 ks CPU / ~3.5 h, output `power_unit.stl` ABSENT). Leftover of attempt 3, not
  blocking. The charter limits operator kills to its own predecessor's leftovers, so NOT killed -
  escalate.
- Exact command a human/orchestrator should start from: `Stop-Process -Id 35748` to reclaim the
  core (safe: parent dead, output never produced). Do NOT reset slot 0. Priority: low.
- Carried authoring item (dispatch blocker): RG-23-CERTIFIED-ENTRY-WIRING and
  RG-9-REFLECT-SOLID-PRODUCTION are registered READY with EMPTY `packet` and MISSING files
  (`loop/packets/RG-23-CERTIFIED-ENTRY-WIRING.md` / `RG-9-REFLECT-SOLID-PRODUCTION.md` absent);
  dispatch_ready's anchor check fails with empty detail (gen_packet FileNotFoundError). Needs
  authoring (orchestrator), not operator. Priority: medium (frontier filler while FHC-G6 runs).
- All other carried human items unchanged: duplicate supervisors (19172 + 27828) + duplicate
  cargoq/server.py (28544 + 34564 + 31804); slot-4/7 wt RESULT residue; FRAME-REVOLVE F1
  non_z_axis pin; TOR-C flip-or-pin.

## 2026-09-13T17:58Z - operator cycle 16: carried - FHC-G6 progressing (do not reset); orphan 35748; RG-23/RG-9 unauthored

- No NEW escalations this cycle. Carried items re-verified by command:
- Slot 0 FHC-G6 HEALTHY (opencode 26336, live child 25388 measuring build_suspension_rear at ~1
  core). Do NOT reset/kill. Frontier FHC-G1/D/E stay chained behind it; the dead slot-1 duplicate
  cannot redispatch until slot 0 commits and frees `packet/FHC-G6-CERT-COST-SCALE`.
- Orphan `35748` (`probe.py f1 lib.power_unit`, parent 1872 dead) still spins ~1 core, output
  `power_unit.stl` ABSENT. Safe to reclaim: `Stop-Process -Id 35748`. Priority: low.
- RG-23-CERTIFIED-ENTRY-WIRING / RG-9-REFLECT-SOLID-PRODUCTION remain READY with empty `packet`
  and missing files; dispatch_ready prints "ANCHOR CHECK FAILED" (gen_packet FileNotFoundError).
  Needs authoring (orchestrator). Priority: medium (frontier filler while FHC-G6 runs).
- All other carried human items unchanged: duplicate supervisors + duplicate cargoq/server.py;
  slot-4/7 wt RESULT residue; FRAME-REVOLVE F1 non_z_axis pin; TOR-C flip-or-pin.

## 2026-09-13T18:28Z - operator cycle 17: NEW - supervisor + watchdog + overnight driver all DEAD (substrate); do NOT blindly restart

- What: the whole substrate stack is down. Only two python processes exist: cargoq server `31804`
  (parent `27828`, now dead) and slot-0's measure child `10028`. `watchdog` = 0 processes,
  `watchdog.lock` holds stale pid `29264` (gone), `watchdog.log` silent since 2026-09-10T22:30Z.
  `supervisor.py` = 0 processes; supervisor.log's last action was 09-13 09:38:57 ("cargoq server
  not running - restarting", which started 31804), then it died. `overnight.py` = 0 processes;
  overnight.log last 09-13 14:16:31 (it was leaving slot 4 F1-AUTHORING-ARMS for morning). The
  17:35Z/17:58Z operator STATE entries that read "watchdog 1 (29264)" were STALE.
- Why it matters: no disk guard, no wedge/hard-death auto-recovery, and no overnight driver to
  land/adjudicate when humans are away. The heartbeat (27872) and this operator still run, so the
  loop is not stopped, but nothing self-heals.
- Why the operator did NOT restart: `supervisor.py` would restart the watchdog AND the overnight
  driver. The watchdog restart is now safe (`packet_is_done()` skips slot 2's already-DONE
  TTC-RECENSUS-F1-R3; slot 0's pid is live so it is untouched; slot 1 would only fail its
  branch-held redispatch), but restarting the overnight driver is a behavioral/orchestration
  decision, and the stack may have been stopped intentionally because an orchestrator session is
  active today (orchestrator commit `2fdf11a`, 09-13). Owner/orchestrator call.
- Exact command a human/orchestrator should start from: confirm the stop was not intentional, then
  `Start-Process python -ArgumentList 'loop\supervisor.py' -WorkingDirectory C:\Users\stefa\look`
  (supervisor then restores watchdog + overnight + cargoq). If you want the watchdog only:
  `Start-Process python -ArgumentList 'loop\watchdog.py' -WorkingDirectory C:\Users\stefa\look`.
  Delete the stale `loop\watchdog.lock` first only if watchdog.py refuses to start (it self-checks
  the lock pid, so it should be fine). Priority: medium.
- Carried unchanged: RG-23/RG-9 authoring; slot-4/7 wt RESULT residue; FRAME-REVOLVE F1
  non_z_axis pin; TOR-C flip-or-pin. The earlier "duplicate supervisors + duplicate cargoq" item
  is now moot (the duplicates are gone). Orphan 35748 is gone.

## 2026-09-13T18:46Z - operator cycle 18: substrate-down item CARRIED unchanged (no new evidence)

- Re-verified this cycle: watchdog = 0 processes, `watchdog.lock` still holds stale pid `29264`,
  `watchdog.log` silent since 2026-09-10T22:30Z; supervisor = 0 processes, `supervisor.log` last
  action 09-13 09:38:57; overnight = 0 processes, `overnight.log` last 09-13 14:16:31. Operator did
  NOT restart (same reasoning as the 18:28Z item: the supervisor would also restart the overnight
  driver, which is an orchestration decision; the stop may be intentional). See the 18:28Z item for
  the exact restart commands. Priority: medium. No other new judgment items this cycle.

## 2026-09-13T19:30Z - operator cycle 20: substrate-down item CARRIED unchanged; disk pressure resolved by janitor

- Re-verified this cycle: watchdog = 0 processes, `watchdog.lock` still holds stale pid `29264`,
  `watchdog.log` silent since 2026-09-10T22:30Z; supervisor = 0 processes, `supervisor.log` last
  action 09-13 09:38:57; overnight = 0 processes, `overnight.log` last 09-13 14:16:31. Operator did
  NOT restart (same reasoning as the 18:28Z item: the supervisor would also restart the overnight
  driver, which is an orchestration decision; the stop may be intentional). See the 18:28Z item for
  the exact restart commands. Priority: medium.
- Disk was 13.98 GB free (below the 15 GB goal); the operator ran the documented
  `python loop/janitor.py ensure --need 15`, which reclaimed ~4.5 GB of idle slot-1 dead-duplicate
  targets -> 17.99 GB free. Not an escalation (routine janitor action; slot 0 live/protected).
- No other new judgment items this cycle. Carried unchanged: RG-23/RG-9 authoring; slot-4/7 wt
  RESULT residue; FRAME-REVOLVE F1 non_z_axis pin; TOR-C flip-or-pin.

## 2026-09-13T19:57Z - operator cycle 21: NEW - slot 0 FHC-G6 worker ALIVE but its bash tool call is hung on a 0-CPU cargoq test (19 min); operator did NOT kill

- What: slot 0 (frontier packet FHC-G6-CERT-COST-SCALE) worker opencode `18632` (cmd `15276`) is
  alive but its in-flight tool call has not returned for ~19 min. Evidence it is HUNG, not the
  usual expensive measurement: the cargoq job
  `cargo test --profile quick -p truck123d --lib tmp_microbench_patch_cert_scaling` (cwd
  `slots/0/wt`, START 15:37:30 local) has **no DONE line in `loop/cargoq/server.log`**; the spawned
  test exe `33784` (`truck123d-01311bf50ba025c6.exe tmp_microbench_patch_cert_scaling --nocapture`,
  created 15:38:16 local) has burned **0.00 s CPU / 3 threads / 7 MB for ~19 min**; cargo 31524 /
  29484 and cargoq client python 27580 are all at 0 CPU; no rustc. events.jsonl frozen at 19:38Z.
- Why the operator did NOT kill/reset: the worker process is ALIVE (charter: never restart a worker
  that is alive; and the frontier branch `packet/FHC-G6-CERT-COST-SCALE` is held by slot 0's dirty
  worktree so a reset/redispatch is not available). Killing `33784`/`27580` might free the tool
  call, but it is a judgment call on a live frontier worker -> escalated rather than acted.
- Exact command a human/orchestrator should start from: inspect
  `C:\Users\stefa\look\loop\cargoq\server.log` tail and the process tree under pid `15276`; if the
  job is confirmed wedged, `Stop-Process -Id 33784` (the hung test exe) is the minimal unblock; do
  NOT kill opencode `18632` unless you also intend to re-fork slot 0. Priority: high (frontier).
- Carried unchanged: substrate stack down (supervisor/watchdog/overnight); RG-23/RG-9 authoring;
  slot-4/7 wt RESULT residue; FRAME-REVOLVE F1 non_z_axis pin; TOR-C flip-or-pin.

## 2026-09-13T20:19Z - operator cycle 22: slot 0 FHC-G6 startup hang is REPRODUCIBLE (timeout -> self-kill -> retry -> hang again); operator still did NOT kill

- Update to the 19:57Z item (same packet, same symptom, now confirmed reproducible). The 19:57Z hung
  job `cargo test --profile quick -p truck123d --lib tmp_microbench_patch_cert_scaling` (cargoq
  START 15:37:30 local) TIMED OUT at 16:07:37 local (exit 4294967295 = -1, 1807 s) with no test
  output. The worker then self-diagnosed (20:07:34Z `Get-CimInstance`), killed the hung
  `33784`/`29484`/`31524` itself (20:07:40Z), edited, and retried. The retry (cargoq START 16:07:47
  local) spawned exe `31336` (`truck123d-01311bf50ba025c6.exe tmp_microbench_patch_cert_scaling
  --nocapture`, created 16:07:50 local) that has now burned **0.00 s CPU / 3 threads / 4 MB for ~12
  min** - the identical startup-hang signature. opencode `18632` (cmd `15276`) is idle (CPU delta
  0.000 s / 4 s); last event 20:07:45Z (step_start). So a plain retry does NOT clear it: the test
  binary wedges at startup deterministically on this slot.
- Why the operator did NOT kill/reset: worker process ALIVE, frontier packet, branch
  `packet/FHC-G6-CERT-COST-SCALE` held by slot 0's dirty worktree (reset/redispatch unavailable),
  and killing a live worker's child is a judgment call. The worker already killed its own hung exe
  once, so it is aware; a human/orchestrator should decide whether to keep killing the exe (the
  worker will keep retrying) or investigate why the binary wedges at 0 CPU on this slot.
- Exact command a human/orchestrator should start from: inspect
  `C:\Users\stefa\look\loop\cargoq\server.log` tail (last line: `16:07:47 START ... -p truck123d
  ...`) and the process tree under pid `15276`; minimal unblock is `Stop-Process -Id 31336` (the
  hung test exe) and `Stop-Process -Id 38104` (its cargoq client). If the wedge recurs, the slot's
  `target/quick` for truck123d may need clearing (a corrupt/locked test binary is the prime
  suspect), or the packet should be re-forked. Do NOT kill opencode `18632` unless you intend to
  re-fork slot 0. Priority: high (frontier; FHC-G1/D/E all chained behind it).
- Carried unchanged: substrate stack down (supervisor/watchdog/overnight); RG-23/RG-9 authoring;
  slot-4/7 wt RESULT residue; FRAME-REVOLVE F1 non_z_axis pin; TOR-C flip-or-pin.

## 2026-09-13T20:44Z - operator cycle 23: slot 0 FHC-G6 startup hang is now CROSS-TEST -> slot/environment-level; operator still did NOT kill

- Update to the 20:19Z item. The worker abandoned `tmp_microbench_patch_cert_scaling` and switched
  to a new integration test `truck123d/tests/cert_cost_scale.rs`. `cargo check -p truck123d --test
  cert_cost_scale` now exits 0 (compile fixed). `cargo test -p truck123d --test cert_cost_scale
  --locked -- --nocapture --test-threads=1` (cargoq START 16:32:03 local) ran 331 s and exited 101,
  and the retry (cargoq START 16:37:45 local) spawned `cert_cost_scale-c3919afee52a961b.exe`
  (pid 14584, created 16:37:49 local) that has burned **0.00 s CPU / 4 threads / 12.2 MB for ~6
  min** (8 s CPU delta = 0.000 s). This is the IDENTICAL startup-hang signature seen on
  `tmp_microbench` at 19:57Z/20:19Z, now on a DIFFERENT test binary.
- Why this matters: the wedge is NOT specific to one test. Two distinct test binaries have now hung
  at ~0 CPU / ~12 MB on slot 0. Prime suspects are slot-level: a corrupt or locked `target/quick`
  test binary for truck123d, or a loader/AV wedge on freshly-linked exes in that target dir. A plain
  retry does not clear it (the worker already tried twice).
- Why the operator did NOT kill/reset: worker process ALIVE, frontier packet, branch
  `packet/FHC-G6-CERT-COST-SCALE` held by slot 0's dirty worktree (reset/redispatch unavailable),
  and killing a live worker's child is a judgment call. The worker is aware and self-kills its hung
  exe, but keeps landing on the same wall.
- Exact command a human/orchestrator should start from: inspect the process tree under pid `15276`
  and `C:\Users\stefa\look\loop\cargoq\server.log` tail (last line: `16:37:45 START ... --test
  cert_cost_scale`). Minimal unblock is `Stop-Process -Id 14584` (the hung test exe) and its cargoq
  client. Given the cross-test recurrence, the stronger fix is to clear the slot's truck123d
  `target/quick` (and/or re-fork slot 0) rather than retry. Do NOT kill opencode `18632` unless you
  intend to re-fork slot 0. Priority: high (frontier; FHC-G1/D/E all chained behind it).
- Carried unchanged: substrate stack down (supervisor/watchdog/overnight); RG-23/RG-9 authoring;
  slot-4/7 wt RESULT residue; FRAME-REVOLVE F1 non_z_axis pin; TOR-C flip-or-pin.

## 2026-09-13T21:06Z - operator cycle 24: slot 0 FHC-G6 FINISHED with a SPEC_GAP RESULT (status DONE); NOT landed; orchestrator adjudication needed

- What: slot 0 FHC-G6-CERT-COST-SCALE is FINISHED. Worker commit `19cfd68` on
  `packet/FHC-G6-CERT-COST-SCALE` (2 files: `truck123d/src/bd_bridge.rs` +78/-5, new
  `truck123d/tests/cert_cost_scale.rs` +584). wt-root `RESULT.json` status **DONE**, tests 3/3 green,
  byte-identity verified. BUT it carries a top-level `spec_gap` field and its notes say "Per the
  packet stop condition this is a SPEC_GAP".
- Why not landed: the packet's stop condition fired - the dominant phase is the per-patch interval
  certification (99.4-99.8% of facts_ms) and no whitelisted bd_bridge-only fix applies; the bracket
  decomposition needs a smarter form (theory-adjacent, back to the gap register). Additionally the
  packet's done-when (`cargo fmt --check -p truck123d`, `cargo clippy -p truck123d --all-targets --
  -D warnings`) does not pass: RESULT's `pre_existing_drift` reports both fail on files/lints
  OUTSIDE `write_allow` (ttc_hazard_battery.rs; vendor/truck/truck-certified clone_on_copy; lib/test
  lints). Charter step 2: RESULT status is DONE but the outcome is a SPEC_GAP and a done-when check
  fails -> do NOT land; escalate.
- Decision needed (orchestrator): (a) land the byte-identical, instrumentation-only commit +
  route the `spec_gap` to `docs/F1_HYPERCAR_GAP_REGISTER.md` (treat fmt/clippy as pre-existing
  baseline noise), or (b) hold and re-fork with a sharpened packet / theory fix. Note the FHC
  frontier (FHC-G1 -> FHC-D -> FHC-E) is chained behind G6's disposition.
- Exact command to start from: `git -C C:\Users\stefa\look show 19cfd68 --stat` and
  `git -C C:\Users\stefa\look show packet/FHC-G6-CERT-COST-SCALE:...`; the RESULT is the untracked
  `C:\Users\stefa\look\loop\slots\0\wt\RESULT.json` (only copy - preserve before any re-fork).
- Carried unchanged: substrate stack down (supervisor/watchdog/overnight); RG-23/RG-9 authoring
  (packet files absent from `loop/packets/`); slot-4/7 wt RESULT residue; FRAME-REVOLVE F1
  non_z_axis pin; TOR-C flip-or-pin.
