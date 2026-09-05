"""Arm the BIE overnight handoff (orchestrator harness, not kernel).

Does, in order, idempotently:

1. Preflight: packet_lint on the 9 BIE-program packets, then the
   mechanical anchor re-derivation (bie_anchor_check.py). Any FAIL stops
   the arming - a packet that would be dispatched blind must not be
   committed.
2. Registry: force the 9 rows (8 BIE + SEM-PCURVE-MASTER-001-FIX) to
   READY, preserving every other row byte-for-byte. (They may be BLOCKED
   if the CC-session agent held them - that hold was correct while the
   spine was uncommitted; arming is what lifts it.)
3. Commit, explicit paths only: spine, BIE build spec, the 9 packets,
   the registry, and these two scripts. Orchestrator-labeled message.
   Retries past a held git index lock (a concurrent session may be
   committing).
4. Confirm the overnight driver (loop/overnight.py) is alive; start a
   detached instance if not.

After arming, the ALREADY-RUNNING CC overnight driver owns everything:
it holds dispatch until the CC battery is green (overnight.py verifier-
first gate), then rolls the BIE waves via dispatch_ready (dependency
order, write-set disjointness, LANDED-marker aware), and fires the BIE
program's single full battery when all 9 rows are LANDED. No mid-program
verify is run (spine 4, overnight deviation).

Exit codes: 0 armed, 1 preflight failure, 2 could not commit.
"""

import json
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]

BIE_IDS = [
    "BIE-000-CONTRACT",
    "BIE-001-ARITHMETIC",
    "BIE-002-SSI4",
    "BIE-003-CARRIER",
    "BIE-004-CLOSURE",
    "BIE-005-ARRANGE",
    "BIE-006-CLASSIFY",
    "BIE-007-GATES",
    "SEM-PCURVE-MASTER-001-FIX",
]

PACKET_FILES = [f"loop/packets/{i}.md" for i in BIE_IDS]

COMMIT_PATHS = [
    "docs/BIE_BUILD_SPINE.md",
    "docs/CERTIFIED_INTERACTION_ENGINE_BUILD_SPEC.md",
    "loop/PACKETS.jsonl",
    "loop/scripts/bie_anchor_check.py",
    "loop/scripts/arm_bie_overnight.py",
    *PACKET_FILES,
]

COMMIT_MSG = (
    "loop: arm BIE overnight - spine + 9 packets committed, rows READY\n\n"
    "Spine session deliverable (docs/BIE_BUILD_SPINE.md): waves "
    "BIE-0 shim -> BIE-1 (001/002/003 + SEM-PCURVE-MASTER-001-FIX) -> "
    "BIE-2 (004/005) -> BIE-3 (006/007), one battery at the end. "
    "Anchors re-derived 2026-09-05 post-CC-014 (construct/mod.rs "
    "^pub mod = 26; SEM PcurveS1/S2 arms = 2 sites each incl. the "
    "landed TryFrom<&SurfaceCurve> path, left untouched). Overnight "
    "deviation recorded in spine 4: no mid-program verify; the "
    "overnight driver's single end battery certifies the shim too. "
    "Orchestrator harness commit - the packets themselves are worker "
    "work, not yet dispatched."
)


def log(msg):
    print(f"[arm-bie] {msg}", flush=True)


def sh(args, **kw):
    return subprocess.run(args, capture_output=True, text=True, **kw)


def preflight():
    ok = True
    for i in BIE_IDS:
        r = sh([sys.executable, str(ROOT / "loop" / "packet_lint.py"),
                f"loop/packets/{i}.md"])
        out = (r.stdout or "") + (r.stderr or "")
        bad = [ln for ln in out.splitlines() if ": FAIL" in ln or "BLOCK" in ln]
        if bad:
            ok = False
            log(f"packet_lint {i}: BLOCKING findings:")
            for ln in bad:
                log(f"    {ln}")
        else:
            log(f"packet_lint {i}: clean/warnings-only")
    r = sh([sys.executable, str(ROOT / "loop" / "scripts" / "bie_anchor_check.py")])
    tail = (r.stdout or "").splitlines()[-3:]
    for ln in tail:
        log(f"anchors: {ln.strip()}")
    if r.returncode != 0:
        ok = False
    return ok


def force_ready():
    reg = ROOT / "loop" / "PACKETS.jsonl"
    lines = reg.read_text(encoding="utf-8").splitlines()
    out, flipped = [], []
    for ln in lines:
        if not ln.strip():
            continue
        row = json.loads(ln)
        if row.get("id") in BIE_IDS and row.get("status") != "READY":
            flipped.append(f"{row['id']}:{row.get('status')}->READY")
            row["status"] = "READY"
        out.append(json.dumps(row, ensure_ascii=False))
    reg.write_text("\n".join(out) + "\n", encoding="utf-8")
    log(f"registry: {len(out)} rows; flipped: {flipped or 'none (all READY)'}")


def commit():
    for attempt in range(12):
        add = sh(["git", "-C", str(ROOT), "add", "--"] + COMMIT_PATHS)
        if add.returncode != 0:
            log(f"git add failed: {add.stderr.strip()[-200:]}")
            return False
        status = sh(["git", "-C", str(ROOT), "status", "--porcelain", "--"] + COMMIT_PATHS)
        staged_any = any(ln[0] in "AM" for ln in status.stdout.splitlines())
        if not staged_any:
            log("nothing to commit (already armed)")
            return True
        c = sh(["git", "-C", str(ROOT), "commit", "-m", COMMIT_MSG, "--"] + COMMIT_PATHS)
        if c.returncode == 0:
            log(f"committed: {c.stdout.strip().splitlines()[0][:80]}")
            return True
        err = (c.stderr or "") + (c.stdout or "")
        if "index.lock" in err:
            log(f"index.lock held (attempt {attempt + 1}/12), retrying in 20s")
            time.sleep(20)
            continue
        log(f"commit failed: {err.strip()[-300:]}")
        return False
    return False


def driver_running():
    r = sh(["powershell", "-NoProfile", "-Command",
            "(Get-CimInstance Win32_Process -Filter \"Name='python.exe'\" | "
            "Where-Object { $_.CommandLine -match 'overnight\\.py' }).Count"])
    try:
        return int(r.stdout.strip().splitlines()[-1]) > 0
    except (ValueError, IndexError):
        return False


def ensure_driver():
    if driver_running():
        log("overnight driver: ALIVE (it owns the BIE handoff)")
        return True
    log("overnight driver: NOT RUNNING - starting detached instance")
    logf = open(ROOT / "loop" / "overnight_driver.log", "ab", buffering=0)
    subprocess.Popen(
        [sys.executable, str(ROOT / "loop" / "overnight.py")],
        stdout=logf, stderr=logf,
        creationflags=subprocess.DETACHED_PROCESS | subprocess.CREATE_NEW_PROCESS_GROUP,
        cwd=str(ROOT),
    )
    time.sleep(5)
    ok = driver_running()
    log(f"overnight driver: {'started OK' if ok else 'FAILED TO START - check loop/overnight_driver.log'}")
    return ok


def main():
    log("arming BIE overnight handoff")
    if not preflight():
        log("PREFLIGHT FAILED - nothing committed, nothing unblocked")
        return 1
    force_ready()
    if not commit():
        log("COMMIT FAILED - rows may be READY but the spine is uncommitted; "
            "BLOCK the 9 rows before walking away")
        return 2
    if not ensure_driver():
        return 2
    log("ARMED. Expected sequence, all by the running driver:")
    log("  1. CC battery finishes green -> dispatch gate opens")
    log("  2. dispatch_ready rolls BIE-000 + SEM-PCURVE-MASTER-001-FIX first")
    log("  3. waves 1-3 roll as deps land (LANDED-marker aware), scoped checks only")
    log("  4. when all 9 LANDED -> the BIE program's ONE battery fires")
    log("  5. anything BLOCKED/REJECTED is left for the morning session")
    return 0


if __name__ == "__main__":
    sys.exit(main())
