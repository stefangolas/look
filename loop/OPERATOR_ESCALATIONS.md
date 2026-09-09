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
