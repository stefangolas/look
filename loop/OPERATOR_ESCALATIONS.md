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
