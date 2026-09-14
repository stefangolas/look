# WORK PACKET LOOP-MACH-1 — loop machinery redesign: atomic claims, attempt identities, supervisor-side write-set enforcement, transactional landing, two-state outcomes

Harness redesign packet (loop-owned; `vendor/truck/**` untouched). Every item
below fixes an incident paid for in the 2026-09-13/14 sessions — the review
they implement is grounded, not speculative.

```yaml
id:          LOOP-MACH-1
contract:    [LOOP-MACH-1]
class:       design
crates:      []
needs:       []
write_allow:
  - loop/dispatch_ready.py
  - loop/run_packet.py
  - loop/slot_status.py
  - loop/check_writeset.py
  - loop/ci_gate.py
  - loop/state.sqlite
  - loop/packets/LOOP-MACH-1.md
read_allow:
  - loop/ORCHESTRATOR.md
  - loop/STATE.md
  - docs/CERTIFIED_FLUX_CALCULUS_INTEGRATION_MAP.md
tests_required: [loop/tests_loop_mach1.py]
budget:      {turns: 70, ctx_tokens: 260000}
```

Anchors: none (harness packet; the acceptance suite below is the gate).
The five items, in build order:

## 1. Atomic claim/lease (SQLite via stdlib `sqlite3`)

`loop/state.sqlite` (WAL mode) is the live scheduling state; `PACKETS.jsonl`
remains the human-readable packet SPEC (regenerated/annotated on landing, never
the concurrency primitive). Schema:

```sql
CREATE TABLE attempts (
  attempt_id  TEXT PRIMARY KEY,      -- 'FHC-G1-RATIONAL-FLUX/0002'
  packet_id   TEXT NOT NULL,
  attempt_no  INTEGER NOT NULL,
  base_sha    TEXT NOT NULL,
  branch_sha  TEXT,
  slot        INTEGER,
  state       TEXT NOT NULL CHECK (state IN
              ('READY','CLAIMED','RUNNING','FINISHED','ADJUDICATING',
               'LANDED','SPEC_GAP','SUPERSEDED')),
  lease_owner TEXT, lease_expires INTEGER,
  outcome     TEXT,                  -- LANDED | SPEC_GAP | REFUSAL | NO_CHANGE | SUPERSEDED | INVALID
  findings    TEXT                   -- JSON array
);
CREATE TABLE write_locks (path TEXT PRIMARY KEY, attempt_id TEXT NOT NULL);
```

Claim = `BEGIN IMMEDIATE; UPDATE attempts SET state='CLAIMED', slot=?, lease=?,
lease_expires=? WHERE attempt_id=? AND state='READY'; COMMIT` — no snapshot-
then-mutate. `FINISHED → ADJUDICATING` is claimed by the landing side
atomically; no scheduler touches a packet in ADJUDICATING. Lease expiry is
timestamp-based (crash-consistent; the next dispatch reaps expired claims).
Shadow mode first: `LOOK_STATE_DB=1` writes SQLite alongside the JSONL flow;
after one equivalence-checked cycle the dispatch path reads SQLite.

## 2. Immutable attempt identities

Every dispatch mints `attempt_id = <packet_id>/<NNNN>` with `base_sha` frozen
at fork. Results land at `loop/results/<packet_id>/<NNNN>.json` (attempt
dirs; old attempts' evidence is never overwritten). Ledger events carry
`attempt_id`. Registry annotation on landing names the accepted attempt.

## 3. Supervisor-side write-set enforcement (kills `git add -A`)

Workers edit and publish `slots/<N>/outbox/RESULT.json`; the supervisor then:
(a) runs `diff_paths(base, tip) ⊆ packet.write_allow` (via the new
`loop/check_writeset.py`), (b) commits ONLY the declared paths as-delivered,
(c) rejects with `WRITESET_VIOLATION` otherwise. This dissolves the
skipped-commit-step class (the supervisor always commits) and the .obj sweep
class (undeclared paths are excluded, not swept).

## 4. Transactional landing

Landing runs in a detached temp worktree: merge `--no-ff` there, run the
merged-HEAD scoped gates there, and only then advance
`integration/kernel-bg` via a single atomic `git update-ref`. The metadata
commit (results, registry, ledger) follows the successful landing. A crash
mid-landing leaves the integration ref untouched.

## 5. Two-state outcomes

`execution_status` (PENDING/RUNNING/COMPLETE/FAILED) is separated from
`outcome` (LANDED/SPEC_GAP/REFUSAL/NO_CHANGE/SUPERSEDED/INVALID) with a
`findings` JSON array — G10 becomes COMPLETE + SPEC_GAP; queries stop being
ambiguous. SPEC_GAP results may carry `blocked_by_capability: [names]` (the
G10 example: `exact_unplaced_patch_recovery`) for future re-booking.

## 6. Cheap adds shipped inside this packet

- `loop/ci_gate.py`: fmt check + executable-bit invariant
  (`git ls-files` scripts == mode 100755) + the H-gates — the exact CI entry
  sequence, runnable locally before push.
- `loop/check_writeset.py` (used by item 3, runnable standalone).
- Anchor outputs record `checked_at: <sha>`.

## Acceptance suite (`loop/tests_loop_mach1.py`, plain pytest/unittest)

1. `claim_is_atomic` — two concurrent claim transactions on one READY attempt:
   exactly one wins.
2. `adjudicating_blocks_dispatch` — FINISHED→ADJUDICATING blocks a competing
   dispatch; ADJUDICATING→LANDED resumes.
3. `attempt_evidence_immutable` — landing attempt 0002 never touches
   0001's result file.
4. `writeset_violation_rejected` — a diff touching an undeclared path fails
   the gate and the landing aborts before any ref moves.
5. `transactional_landing_crash_safe` — a gate failure at step 5 leaves
   `integration/kernel-bg` unmoved and the worktree clean.
6. `outcome_separation` — a SPEC_GAP result records COMPLETE + SPEC_GAP +
   findings; `LANDED` records COMPLETE + LANDED.
7. `ci_gate_exec_bit` — a 100644 script file fails `ci_gate.py` locally.

## Done when

```
python -m pytest loop/tests_loop_mach1.py -q
python loop\ci_gate.py          # green locally on Windows
```

plus a shadow-mode equivalence pass: one full dispatch_ready cycle with
`LOOK_STATE_DB=1` producing identical dispatch decisions to the JSONL path.

## Forbidden

`vendor/truck/**`; any edit to `corpus/ttc/**`; deleting or rewriting
PACKETS.jsonl history; weakening any existing gate; committing to `main`.

## Stop conditions

- SQLite concurrency semantics cannot support the CAS claim under WAL on
  Windows → `SPEC_GAP` naming the failure (evidence, not guesswork)
- shadow mode diverges from JSONL dispatch decisions → fix or `SPEC_GAP`
- three consecutive failed test runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` AT THE WORKTREE ROOT

```json
{"id":"LOOP-MACH-1","status":"DONE","contracts":["LOOP-MACH-1"],
 "notes":"suite 1-7 verdicts; shadow-mode equivalence record; migration plan for the live board"}
```

Commit subject: `loop: machinery redesign - atomic claims, attempt IDs, writeset enforcement, transactional landing (LOOP-MACH-1)`.
