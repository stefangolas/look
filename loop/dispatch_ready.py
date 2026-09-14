"""dispatch_ready: the mechanical rolling dispatcher (session 50).

One command replaces the manual new_slot/run_packet dances:
  1. READY packets whose deps are LANDED and whose write set is disjoint
     from every RUNNING row.
  2. Free slots (IDLE/FINISHED adjudicated, or a missing slot dir).
  3. gen_packet --check (anchor reality) + packet_lint before dispatch.
  4. new_slot + run_packet with the cargoq shim first in PATH and the
     CARGO_BUILD_JOBS cap.

NOT automated (judgment stays with the orchestrator): adjudicating
RESULTs, merging, amending. This script only fills the machine.

Usage:
  python loop/dispatch_ready.py --dry-run
  python loop/dispatch_ready.py            # dispatch for real
  python loop/dispatch_ready.py --max-workers 6
"""
import json
import os
import re
import sqlite3
import subprocess
import sys
import tempfile
import time
from contextlib import contextmanager
from pathlib import Path

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
QUEUE = os.path.join(ROOT, "loop", "cargoq")

# ---------------------------------------------------------------------------
# Scheduling state (LOOP-MACH-1 item 1).
#
# PACKETS.jsonl is the human-readable packet SPEC. It is regenerated and
# annotated on landing and is NOT a concurrency primitive: the session-58
# lost-update race clobbered a registry row because two readers wrote back
# whole-file snapshots. `state.sqlite` (WAL) is the live scheduling state;
# claims are compare-and-swaps inside `BEGIN IMMEDIATE`, so exactly one
# dispatcher can own an attempt.
#
# Shadow mode first: with LOOK_STATE_DB=1 the JSONL flow runs unchanged and
# SQLite is written alongside it, so the two can be equivalence-checked before
# the dispatch path reads the database.
# ---------------------------------------------------------------------------
DB_NAME = "state.sqlite"
DEFAULT_LEASE_SECONDS = 1800
# States in which an attempt still owns its packet: a dispatcher must not mint
# a competing attempt. SPEC_GAP and SUPERSEDED are deliberately absent --
# those are closed outcomes, and re-booking a gap is a new attempt.
BLOCKING_STATES = ("READY", "CLAIMED", "RUNNING", "FINISHED", "ADJUDICATING",
                   "LANDED")
TERMINAL_STATES = ("LANDED", "SPEC_GAP", "SUPERSEDED")
EXECUTION_STATUSES = ("PENDING", "RUNNING", "COMPLETE", "FAILED")
OUTCOMES = ("LANDED", "SPEC_GAP", "REFUSAL", "NO_CHANGE", "SUPERSEDED",
            "INVALID")
ATTEMPT_STATES = ("READY", "CLAIMED", "RUNNING", "FINISHED", "ADJUDICATING",
                  "LANDED", "SPEC_GAP", "SUPERSEDED")

STATE_SCHEMA = """
CREATE TABLE IF NOT EXISTS attempts (
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
  findings    TEXT,                  -- JSON array
  execution_status TEXT,             -- PENDING | RUNNING | COMPLETE | FAILED
  blocked_by_capability TEXT         -- JSON array of capability names
);
CREATE UNIQUE INDEX IF NOT EXISTS attempts_packet_no
  ON attempts(packet_id, attempt_no);
CREATE TABLE IF NOT EXISTS write_locks (path TEXT PRIMARY KEY, attempt_id TEXT NOT NULL);
"""


def sh(args, **kw):
    return subprocess.run(args, capture_output=True, text=True, **kw)


def rows():
    out = []
    with open(os.path.join(ROOT, "loop", "PACKETS.jsonl"),
              encoding="utf-8-sig") as f:
        for line in f:
            line = line.strip()
            if line:
                out.append(json.loads(line))
    return out


LANDED_RE = re.compile(r"landed [0-9a-f]{7,}")


def landed(r):
    """The LANDED <sha> note marker is the truth (wave_manifest's
    convention), regardless of the status field: session 51 saw a row
    stuck at READY with the LANDED note and dispatch stayed blocked on a
    packet that was already merged."""
    s = (r.get("status") or "").lower()
    note = (r.get("note") or "").lower()
    return s == "done" or LANDED_RE.search(note) is not None


# kernel/mod.rs is the standard one-line pub-mod registration file: same-file
# work there is the DESIGNED textual conflict, resolved at merge (build spec
# section 4) - it does not block dispatch. The CC program's construct/mod.rs
# is the same convention (session 51).
EXPECTED = {"vendor/truck/truck-certified/src/kernel/mod.rs",
            "vendor/truck/truck-certified/src/construct/mod.rs"}


# ---------------------------------------------------------------------------
# Attempt identities (item 2) and the atomic claim/lease state machine.
# ---------------------------------------------------------------------------

def state_db_path(root=ROOT):
    return os.path.join(root, "loop", DB_NAME)


def state_enabled(env=None):
    env = os.environ if env is None else env
    return env.get("LOOK_STATE_DB") == "1"


def connect_state(path, timeout=30.0):
    """A WAL connection with a busy timeout long enough for claim contention.

    `isolation_level=None` puts the connection in autocommit so the explicit
    `BEGIN IMMEDIATE` below is the transaction boundary, not Python's implicit
    one. A second claimer blocks in `BEGIN IMMEDIATE` until the first commits,
    then its UPDATE finds the state already moved.
    """
    conn = sqlite3.connect(str(path), timeout=timeout, isolation_level=None)
    conn.row_factory = sqlite3.Row
    conn.execute("PRAGMA journal_mode=WAL")
    conn.execute("PRAGMA busy_timeout=30000")
    conn.execute("PRAGMA foreign_keys=ON")
    return conn


def init_state(conn):
    conn.executescript(STATE_SCHEMA)
    return conn


@contextmanager
def immediate(conn):
    conn.execute("BEGIN IMMEDIATE")
    try:
        yield conn
    except BaseException:
        conn.execute("ROLLBACK")
        raise
    conn.execute("COMMIT")


def _now():
    return int(time.time())


def attempt_no(attempt_id):
    """'PKT/0002' -> 2. The number is the immutable evidence slot."""
    return int(str(attempt_id).rsplit("/", 1)[-1])


def mint_attempt(conn, packet_id, base_sha, slot=None, state="READY",
                 lease_owner=None, lease_seconds=0):
    """Insert the next attempt for a packet. The caller owns the transaction."""
    row = conn.execute(
        "SELECT COALESCE(MAX(attempt_no), 0) AS n FROM attempts "
        "WHERE packet_id=?", (packet_id,)).fetchone()
    no = (row["n"] if row else 0) + 1
    aid = f"{packet_id}/{no:04d}"
    expires = (_now() + lease_seconds) if lease_owner else None
    conn.execute(
        "INSERT INTO attempts(attempt_id, packet_id, attempt_no, base_sha, "
        "slot, state, lease_owner, lease_expires) VALUES(?,?,?,?,?,?,?,?)",
        (aid, packet_id, no, base_sha, slot, state, lease_owner, expires))
    return aid


def claim_attempt(conn, attempt_id, slot, lease_owner,
                  lease_seconds=DEFAULT_LEASE_SECONDS):
    """CAS READY -> CLAIMED. Exactly one caller can win."""
    now = _now()
    with immediate(conn):
        cur = conn.execute(
            "UPDATE attempts SET state='CLAIMED', slot=?, lease_owner=?, "
            "lease_expires=? WHERE attempt_id=? AND state='READY'",
            (slot, lease_owner, now + lease_seconds, attempt_id))
        return cur.rowcount == 1


def _reap_in_tx(conn, now):
    return conn.execute(
        "UPDATE attempts SET state='READY', slot=NULL, lease_owner=NULL, "
        "lease_expires=NULL WHERE state IN ('CLAIMED','RUNNING') "
        "AND lease_expires IS NOT NULL AND lease_expires < ?", (now,))


def reap_expired(conn, now=None):
    """Return expired claims to READY. Crash-consistent: lease expiry is a
    timestamp, so a dead dispatcher's claim is reclaimed by the next cycle
    rather than wedging the packet forever."""
    now = _now() if now is None else now
    with immediate(conn):
        return _reap_in_tx(conn, now).rowcount


def latest_attempt(conn, packet_id):
    return conn.execute(
        "SELECT * FROM attempts WHERE packet_id=? "
        "ORDER BY attempt_no DESC LIMIT 1", (packet_id,)).fetchone()


def active_attempt(conn, packet_id):
    """The latest attempt while it still owns the packet, else None."""
    row = latest_attempt(conn, packet_id)
    if row is None or row["state"] in TERMINAL_STATES:
        return None
    return row


def claim_for_dispatch(conn, packet_id, base_sha, slot, lease_owner,
                       lease_seconds=DEFAULT_LEASE_SECONDS):
    """Mint-and-claim the next attempt, or return None if one is live.

    The reap, the live-attempt check, the attempt-number allocation and the
    insert all happen inside one `BEGIN IMMEDIATE`, so two dispatchers racing
    on the same packet cannot both mint attempt N+1.
    """
    now = _now()
    with immediate(conn):
        _reap_in_tx(conn, now)
        row = conn.execute(
            "SELECT attempt_no, state FROM attempts WHERE packet_id=? "
            "ORDER BY attempt_no DESC LIMIT 1", (packet_id,)).fetchone()
        if row is not None and row["state"] in BLOCKING_STATES:
            return None
        no = (row["attempt_no"] if row else 0) + 1
        aid = f"{packet_id}/{no:04d}"
        conn.execute(
            "INSERT INTO attempts(attempt_id, packet_id, attempt_no, base_sha, "
            "slot, state, lease_owner, lease_expires) VALUES(?,?,?,?,?,?,?,?)",
            (aid, packet_id, no, base_sha, slot, "CLAIMED", lease_owner,
             now + lease_seconds))
        return aid


def transition(conn, attempt_id, frm, to, **columns):
    """Generic guarded state move. Returns True iff a row moved."""
    if to not in ATTEMPT_STATES:
        raise ValueError(f"unknown attempt state {to!r}")
    states = (frm,) if isinstance(frm, str) else tuple(frm)
    sets = ["state=?"] + [f"{key}=?" for key in columns]
    values = [to, *columns.values()]
    placeholders = ",".join("?" for _ in states)
    values.extend([attempt_id, *states])
    sql = (f"UPDATE attempts SET {', '.join(sets)} "
           f"WHERE attempt_id=? AND state IN ({placeholders})")
    with immediate(conn):
        return conn.execute(sql, values).rowcount == 1


def claim_adjudication(conn, attempt_id):
    """FINISHED -> ADJUDICATING, claimed atomically by the landing side.

    Once here, no scheduler touches the packet: a competing dispatch sees
    ADJUDICATING and is refused, so an attempt cannot be landed twice.
    """
    with immediate(conn):
        cur = conn.execute(
            "UPDATE attempts SET state='ADJUDICATING', lease_owner=NULL, "
            "lease_expires=NULL WHERE attempt_id=? AND state='FINISHED'",
            (attempt_id,))
        return cur.rowcount == 1


def finish_attempt(conn, attempt_id, execution_status="COMPLETE", outcome=None,
                   findings=None, branch_sha=None):
    if execution_status not in EXECUTION_STATUSES:
        raise ValueError(f"unknown execution_status {execution_status!r}")
    if outcome is not None and outcome not in OUTCOMES:
        raise ValueError(f"unknown outcome {outcome!r}")
    with immediate(conn):
        cur = conn.execute(
            "UPDATE attempts SET state='FINISHED', execution_status=?, "
            "outcome=?, findings=?, "
            "branch_sha=COALESCE(?, branch_sha), lease_owner=NULL, "
            "lease_expires=NULL "
            "WHERE attempt_id=? AND state IN ('CLAIMED','RUNNING')",
            (execution_status, outcome,
             json.dumps(list(findings)) if findings is not None else None,
             branch_sha, attempt_id))
        return cur.rowcount == 1


def land_attempt(conn, attempt_id, branch_sha=None):
    with immediate(conn):
        cur = conn.execute(
            "UPDATE attempts SET state='LANDED', outcome='LANDED', "
            "execution_status='COMPLETE', "
            "branch_sha=COALESCE(?, branch_sha), lease_owner=NULL, "
            "lease_expires=NULL "
            "WHERE attempt_id=? AND state='ADJUDICATING'",
            (branch_sha, attempt_id))
        return cur.rowcount == 1


def record_outcome(conn, attempt_id, execution_status, outcome, findings=None,
                   blocked_by_capability=None, branch_sha=None):
    """Write the two-state outcome onto an attempt.

    `execution_status` says whether the worker finished; `outcome` says what
    the finished work was worth. G10 is COMPLETE + SPEC_GAP, not an ambiguous
    "DONE with a spec_gap field" -- that ambiguity is what this separates.
    """
    if execution_status not in EXECUTION_STATUSES:
        raise ValueError(f"unknown execution_status {execution_status!r}")
    if outcome not in OUTCOMES:
        raise ValueError(f"unknown outcome {outcome!r}")
    state = {"LANDED": "LANDED", "SPEC_GAP": "SPEC_GAP",
             "SUPERSEDED": "SUPERSEDED"}.get(outcome, "FINISHED")
    with immediate(conn):
        cur = conn.execute(
            "UPDATE attempts SET state=?, execution_status=?, outcome=?, "
            "findings=?, blocked_by_capability=?, "
            "branch_sha=COALESCE(?, branch_sha), lease_owner=NULL, "
            "lease_expires=NULL WHERE attempt_id=?",
            (state, execution_status, outcome,
             json.dumps(list(findings)) if findings is not None else None,
             json.dumps(list(blocked_by_capability))
             if blocked_by_capability else None,
             branch_sha, attempt_id))
        return cur.rowcount == 1


def build_result(packet_id, attempt_id, execution_status, outcome,
                 findings=None, blocked_by_capability=None, **extra):
    """The RESULT.json payload with the two states kept distinct."""
    if execution_status not in EXECUTION_STATUSES:
        raise ValueError(f"unknown execution_status {execution_status!r}")
    if outcome not in OUTCOMES:
        raise ValueError(f"unknown outcome {outcome!r}")
    result = {
        "id": packet_id,
        "attempt_id": attempt_id,
        "execution_status": execution_status,
        "outcome": outcome,
        "findings": list(findings or []),
    }
    if blocked_by_capability:
        result["blocked_by_capability"] = list(blocked_by_capability)
    result.update(extra)
    return result


def result_path(root, packet_id, attempt_no_value):
    return Path(root) / "loop" / "results" / packet_id / \
        f"{int(attempt_no_value):04d}.json"


def file_result(root, packet_id, attempt_no_value, result):
    """Write one attempt's evidence. Never overwrites an earlier attempt.

    Old attempts' evidence is the only record of what was tried and why it
    was rejected; a landing that reused the path would destroy exactly the
    artifact a later re-booking reads. The existence check makes that a hard
    error, not a convention.
    """
    path = result_path(root, packet_id, attempt_no_value)
    if path.exists():
        raise FileExistsError(
            f"attempt evidence is immutable: {path} already exists")
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n",
                    encoding="utf-8")
    return path


def collect_result(slot_root, packet_id=None):
    """The worker's published result: outbox first, then the worktree root.

    The supervisor publishes to `slots/<N>/outbox/RESULT.json` (item 3); the
    worktree-root location is accepted so pre-machinery slots keep working.
    """
    for candidate in (Path(slot_root) / "outbox" / "RESULT.json",
                      Path(slot_root) / "wt" / "RESULT.json"):
        if candidate.is_file():
            try:
                data = json.loads(candidate.read_text(encoding="utf-8-sig"))
            except (ValueError, OSError):
                return None
            if packet_id is None or data.get("id") == packet_id \
                    or data.get("packet") == packet_id:
                return data
    return None


# ---------------------------------------------------------------------------
# Shadow-mode equivalence (item 1): the JSONL decision and the SQLite decision
# are computed from the same pure predicate; the mirror only has to be
# faithful for the two to agree.
# ---------------------------------------------------------------------------

def derive_state(row, landed_ids):
    if row["id"] in landed_ids:
        return "LANDED"
    status = (row.get("status") or "READY").upper()
    return {"RUNNING": "RUNNING", "BLOCKED": "SPEC_GAP",
            "SUPERSEDED": "SUPERSEDED", "DONE": "LANDED"}.get(status, "READY")


def dispatchable(row, state, landed_ids, assigned, dead, running_writes):
    if state != "READY":
        return False
    if row["id"] in assigned or row["id"] in dead:
        return False
    if any(n not in landed_ids for n in row.get("needs", [])):
        return False
    writes = set(row.get("writes", []))
    if (writes - EXPECTED) & (running_writes - EXPECTED):
        return False
    return True


def mirror_rows(conn, rows, landed_ids):
    with immediate(conn):
        for row in rows:
            state = derive_state(row, landed_ids)
            conn.execute(
                "INSERT OR REPLACE INTO attempts(attempt_id, packet_id, "
                "attempt_no, base_sha, state) VALUES(?,?,?,?,?)",
                (f"{row['id']}/0001", row["id"], 1, "", state))


def shadow_compare(rows_, assigned, dead, running_writes, db_path=None):
    """One dispatch cycle, decided twice: JSONL bookkeeping vs SQLite state."""
    jsonl_landed = {r["id"] for r in rows_ if landed(r)}
    jsonl_decisions = [
        r["id"] for r in rows_
        if dispatchable(r, derive_state(r, jsonl_landed), jsonl_landed,
                        assigned, dead, running_writes)]

    temp = None
    if db_path is None:
        handle, db_path = tempfile.mkstemp(prefix="look-state-", suffix=".sqlite")
        os.close(handle)
        temp = db_path
    conn = connect_state(db_path)
    try:
        init_state(conn)
        mirror_rows(conn, rows_, jsonl_landed)
        sqlite_landed = {row["packet_id"] for row in conn.execute(
            "SELECT packet_id FROM attempts WHERE state='LANDED'")}
        sqlite_decisions = []
        for row in rows_:
            state_row = conn.execute(
                "SELECT state FROM attempts WHERE packet_id=? "
                "ORDER BY attempt_no DESC LIMIT 1", (row["id"],)).fetchone()
            state = state_row["state"] if state_row else "READY"
            if dispatchable(row, state, sqlite_landed, assigned, dead,
                            running_writes):
                sqlite_decisions.append(row["id"])
    finally:
        conn.close()
        if temp:
            for suffix in ("", "-wal", "-shm"):
                try:
                    os.remove(temp + suffix)
                except OSError:
                    pass
    return {"equal": jsonl_decisions == sqlite_decisions,
            "jsonl": jsonl_decisions, "sqlite": sqlite_decisions}


def slots():
    d = os.path.join(ROOT, "loop", "slots")
    res = {}
    if os.path.isdir(d):
        for name in sorted(os.listdir(d), key=lambda x: int(x) if
                           x.isdigit() else 999):
            st = sh([sys.executable, os.path.join(ROOT, "loop",
                    "slot_status.py")], )
            break  # slot_status prints all; parse below instead
    return res


def slot_states():
    out = sh([sys.executable, os.path.join(ROOT, "loop", "slot_status.py")])
    states = {}
    assigned = set()
    dead = set()   # FINISHED with no RESULT.json in the wt = dead dispatch
    slot_of = {}   # packet id -> slot number
    for line in out.stdout.splitlines():
        parts = line.split()
        if len(parts) >= 2 and parts[0] == "slot" and parts[1].isdigit():
            states[parts[1]] = parts[2]
            pkt = None
            if len(parts) >= 4 and parts[3].startswith("packet="):
                pkt = parts[3][len("packet="):].removesuffix(".md")
            if pkt:
                slot_of[pkt] = parts[1]
                # Session-54 belt-and-suspenders: a transient liveness
                # misread (OpenProcess failing under peak load) read live
                # slots as free and raced re-dispatches. Events fresher
                # than 3 minutes force RUNNING regardless of the probe.
                try:
                    ev = os.path.join(ROOT, "loop", "slots", parts[1],
                                      "events.jsonl")
                    ev_age = time.time() - os.path.getmtime(ev)
                except OSError:
                    ev_age = None
                state = parts[2]
                if state != "RUNNING" and ev_age is not None and ev_age < 180 \
                        and pkt and not os.path.isfile(
                            os.path.join(ROOT, "loop", "slots", parts[1],
                                         "wt", "RESULT.json")):
                    state = "RUNNING"
                    states[parts[1]] = "RUNNING"
                res = os.path.join(ROOT, "loop", "slots", parts[1],
                                   "wt", "RESULT.json")
                res_id = None
                if os.path.isfile(res):
                    try:
                        with open(res, encoding="utf-8-sig") as f:
                            rj = json.load(f)
                        res_id = (rj.get("packet") or rj.get("id") or "")
                    except Exception:
                        res_id = "?unreadable"
                if parts[2] == "RUNNING" or res_id == pkt:
                    assigned.add(pkt)
                else:
                    # Stale RESULT from another packet (or none): garbage -
                    # the filed copy lives in loop/results/. The slot is
                    # free for THIS packet after the stale file goes.
                    dead.add(pkt)
    return states, assigned, dead, slot_of


def main():
    dry = "--dry-run" in sys.argv
    # 4, not 6: the session-51 re-derived RAM arithmetic caps clean warm
    # builds at 4 workers; above that, concurrent prewarms re-enter the
    # 0xc0000409 zone (recorded 2026-09-05: CTE-000 lost the warm-build
    # lottery twice and CTE-006/CL-005 collided at the same second while 6
    # ran). Dead workers make the machine slower, not faster.
    max_workers = 4
    for a in sys.argv:
        if a.startswith("--max-workers="):
            max_workers = int(a.split("=")[1])
    rs = rows()
    by_id = {r["id"]: r for r in rs}
    states, assigned, dead, slot_of = slot_states()
    # Session-54 fix (paid by CFP-001 x BREP-002): the running write-set
    # came from rows with status == "RUNNING", but this program's rows stay
    # READY with the LANDED-marker-in-note convention, so every in-flight
    # packet was invisible to the clash check and BREP-002 dispatched into
    # assemble.rs while CFP-001 (same defect, same file) was live. Slot
    # ground truth beats row bookkeeping (the session-50 doctrine).
    running = [by_id[p] for p, s in slot_of.items()
               if states.get(s) == "RUNNING" and p in by_id
               and not landed(by_id[p])]
    running_writes = {w for r in running for w in r.get("writes", [])}
    free = [s for s, st in states.items() if st in ("IDLE", "FINISHED")]
    next_slot = max((int(s) for s in states), default=-1) + 1
    busy = sum(1 for st in states.values() if st == "RUNNING")

    # Shadow-mode equivalence (item 1): decide the same cycle from the JSONL
    # bookkeeping and from a mirrored SQLite state, and refuse to call the
    # migration equivalent until the two agree.
    if "--shadow-compare" in sys.argv:
        report = shadow_compare(rs, assigned, dead, running_writes)
        print(f"shadow-compare jsonl:  {report['jsonl']}")
        print(f"shadow-compare sqlite: {report['sqlite']}")
        print("shadow-compare: " + ("EQUAL" if report["equal"] else "DIVERGED"))
        return 0 if report["equal"] else 1

    state_conn = None
    if state_enabled():
        state_conn = init_state(connect_state(state_db_path()))
        reaped = reap_expired(state_conn)
        if reaped:
            print(f"state.sqlite: reaped {reaped} expired claim(s)")

    print(f"slots: {len(states)} ({busy} running, {len(free)} free); "
          f"slot-assigned packets: {len(assigned)}")
    acted = 0
    for r in rs:
        if acted + busy >= max_workers or not free and acted == 0:
            if acted + busy >= max_workers:
                break
        if r.get("status") != "READY":
            continue
        if landed(r):
            # LANDED marker beats a stale READY status (session-51: 307
            # was re-dispatched onto a slot minutes after its merge
            # because only the status field was read).
            continue
        if r["id"] in assigned:
            continue  # already in a slot (ground truth beats row status)
        if r["id"] in dead:
            # A dead dispatch holds nothing (no RESULT, no code): reset is
            # safe and the re-dispatch proceeds through the normal path.
            # BUT reset-only RESTORES the filed RESULT.json (its semantics),
            # so the stale file is removed AFTER the reset - otherwise the
            # machinery resurrects the very file that marks the slot dead.
            s = slot_of.get(r["id"])
            if s and dry:
                print(f"  {r['id']}: DEAD dispatch (slot {s} holds no "
                      f"matching RESULT) - would reset + delete + redispatch")
                continue
            if s:
                sh([sys.executable, os.path.join(ROOT, "loop",
                    "run_packet.py"), "--slot", s, "--reset-only",
                    "--packet", os.path.join(ROOT, "loop", "packets",
                    r["id"] + ".md")])
                res = os.path.join(ROOT, "loop", "slots", s, "wt",
                                   "RESULT.json")
                if os.path.isfile(res):
                    os.remove(res)
                free.append(s)
        needs = r.get("needs", [])
        unmet = [n for n in needs
                 if n not in by_id or not landed(by_id[n])]
        if unmet:
            print(f"  {r['id']}: blocked on {unmet}")
            continue
        writes = set(r.get("writes", []))
        # kernel/mod.rs is the standard one-line pub-mod registration file:
        # same-file work there is the DESIGNED textual conflict, resolved at
        # merge (build spec section 4) - it does not block dispatch. The CC
        # program's construct/mod.rs is the same convention (session 51).
        clash = (writes - EXPECTED) & (running_writes - EXPECTED)
        if clash:
            print(f"  {r['id']}: write-set clash with a RUNNING row: "
                  f"{sorted(clash)[:2]}")
            continue
        pk = os.path.join(ROOT, "loop", "packets", r["id"] + ".md")
        chk = sh([sys.executable, os.path.join(ROOT, "loop",
                  "gen_packet.py"), "--check", pk])
        if chk.returncode != 0:
            print(f"  {r['id']}: ANCHOR CHECK FAILED - fix before "
                  f"dispatch:\n{chk.stdout[-400:]}")
            continue
        lint = sh([sys.executable, os.path.join(ROOT, "loop",
                   "packet_lint.py"), pk])
        if "FAIL" in lint.stdout:
            print(f"  {r['id']}: LINT FAIL:\n{lint.stdout[-300:]}")
            continue
        slot = free.pop(0) if free else str(next_slot)
        next_slot = max(next_slot, int(slot) + 1)
        branch = "packet/" + r["id"]
        print(f"  {r['id']} -> slot {slot} ({branch})")
        if dry:
            acted += 1
            continue
        shared_target = os.environ.get(
            "LOOK_SHARED_TARGET",
            os.path.join(ROOT, "loop", "target-shared"))
        sh([sys.executable, os.path.join(ROOT, "loop", "run_packet.py"),
            "--slot", slot, "--reset-only", "--packet", pk],
           env={**os.environ, "LOOK_SHARED_TARGET": shared_target})
        ns = sh([sys.executable, os.path.join(ROOT, "loop", "new_slot.py"),
                 "--slot", slot, "--branch", branch],
                env={**os.environ, "LOOK_SHARED_TARGET": shared_target})
        if ns.returncode != 0:
            print(f"  {r['id']}: new_slot FAILED:\n{ns.stderr[-300:]}")
            continue
        env_path = QUEUE + os.pathsep + os.environ.get("PATH", "")
        # LOOK_SHARED_TARGET (session-50 machinery, switched on 2026-09-13):
        # workers share ONE warm target tree - cargoq serializes invocations
        # so it is race-free, re-dispatches stop paying the full workspace
        # warm build (new_slot skips it when the shared tree has content),
        # and the worker's first scoped check catches up incrementally.
        # Per-attempt full compiles violate the one-verify architecture.
        rp = subprocess.run(
            [sys.executable, os.path.join(ROOT, "loop", "run_packet.py"),
             "--slot", slot, "--packet", pk],
            env={**os.environ, "PATH": env_path,
                 "CARGO_BUILD_JOBS": "2",
                 "LOOK_SHARED_TARGET": shared_target})
        if rp.returncode == 0:
            # register the dispatch in PACKETS.jsonl
            r["status"] = "RUNNING"
            acted += 1
            running_writes |= writes
            # The attempt itself was minted and claimed by run_packet.py in
            # this same environment (item 2); the dispatcher's SQLite role in
            # shadow mode is the lease reaper above.
        else:
            print(f"  {r['id']}: run_packet FAILED (see above)")
    if state_conn is not None:
        state_conn.close()
    print(f"dispatched {acted}; workers now ~{busy + acted}/{max_workers}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
