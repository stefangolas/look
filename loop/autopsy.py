"""autopsy: mechanical self-diagnosis for a dead worker (session 51).

The owner directive: when a packet dies twice in a row, run a diagnostic
pass over the dead sessions BEFORE redispatching - most death classes
carry their cause in machine-readable evidence, and redispatching blind
into the same killer burns a worker run (paid three times for BG-KV2-501
on 2026-09-03: hang, wedged, uv_spawn - all in the same RAM window, none
diagnosed between deaths).

Evidence read (all machine-local, seconds):
  - loop/slots/N/events.jsonl tail: the last event, its age, and the gap
  - loop/slots/N/worker.err
  - the opencode log's ERROR lines inside the session's window
  - free RAM now (the ENOMEM class kills later runs too)

Classification printed as CLASS: <name> with the remedy:
  RAM_PRESSURE   uv_spawn / 0xc0000409 / ENOMEM in the window -> do not
                 redispatch until RAM recovers; close chrome; shrink caps
  API_ERROR      stream error / balance / socket in the window -> check
                 provider balance before any redispatch
  SILENT_HANG    events frozen, no error recorded -> resume-interrupted
                 (session survives) is the cheap first recovery
  NO_EVIDENCE    events frozen with nothing in the logs -> fresh dispatch
                 is as good as anything; resume risks a poisoned session

Usage: python loop/autopsy.py <slot-number>
Stdlib only (house rule for loop/*.py).
"""
import json
import re
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OPENCODE_LOG = Path.home() / ".local" / "share" / "opencode" / "log" / \
    "opencode.log"


def tail_events(slot):
    events = slot / "events.jsonl"
    if not events.exists():
        return {"last_type": None, "age_s": None, "session": None,
                "lines": 0, "note": "no events.jsonl"}
    lines = events.read_text(encoding="utf-8", errors="replace") \
        .splitlines()
    if not lines:
        return {"last_type": None, "age_s": None, "session": None,
                "lines": 0, "note": "events.jsonl is empty"}
    last = None
    for line in reversed(lines):
        try:
            last = json.loads(line)
            break
        except ValueError:
            continue
    ts = last.get("timestamp") if last else None
    age_s = (time.time() * 1000 - ts) / 1000 if isinstance(ts, (int, float)) \
        else None
    sid = last.get("sessionID") if last else None
    return {"last_type": last.get("type") if last else None,
            "age_s": age_s, "session": sid, "lines": len(lines),
            "note": None}


def opencode_errors(window_s=6 * 3600):
    if not OPENCODE_LOG.exists():
        return []
    out = subprocess.run(
        ["powershell", "-NoProfile", "-Command",
         f"Select-String -Path '{OPENCODE_LOG}' -Pattern 'level=ERROR' | "
         f"Select-Object -Last 12 | Expand-Object -Property Line"],
        capture_output=True, text=True, timeout=60)
    lines = [l for l in (out.stdout or "").splitlines() if l.strip()]
    cutoff = time.time() - window_s
    hits = []
    for line in lines:
        m = re.match(r"timestamp=(\S+)", line)
        if not m:
            continue
        try:
            stamp = time.mktime(time.strptime(
                m.group(1)[:19], "%Y-%m-%dT%H:%M:%S"))
        except ValueError:
            continue
        if stamp >= cutoff:
            hits.append(line[:220])
    return hits


def free_ram_gb():
    out = subprocess.run(
        ["powershell", "-NoProfile", "-Command",
         "(Get-CimInstance Win32_OperatingSystem).FreePhysicalMemory"],
        capture_output=True, text=True, timeout=30).stdout.strip()
    try:
        return int(out) * 1024 / (1000 ** 3)
    except ValueError:
        return -1.0


def classify(slot):
    info = tail_events(slot)
    print(f"== autopsy slot {slot.name} ==")
    if info.get("note"):
        print(f"events: {info['note']}")
    else:
        age = f"{info['age_s']:.0f}s ago" if info["age_s"] is not None \
            else "?"
        print(f"events: last '{info['last_type']}' {age} "
              f"({info['lines']} lines, session {info['session']})")
    werr = slot / "worker.err"
    if werr.exists() and werr.stat().st_size:
        print(f"worker.err: {werr.stat().st_size} bytes (read it)")
    hits = opencode_errors()
    text = "\n".join(hits)
    print(f"opencode log ERROR lines in the last 6h: {len(hits)}")
    for h in hits[-5:]:
        print(f"  {h}")
    ram = free_ram_gb()
    print(f"free RAM now: {ram:.1f} GB")
    blob = text.lower()
    if "uv_spawn" in blob or "0xc0000409" in blob or "enomem" in blob:
        return "RAM_PRESSURE: do NOT redispatch until RAM recovers; " \
               "close chrome, shrink caps, let cargoq drain"
    if "insufficient balance" in blob or "402" in blob:
        return "API_ERROR: check provider balance before any redispatch"
    if "stream error" in blob or "socket" in blob:
        return "API_ERROR: provider socket failures in the window; " \
               "retry is reasonable but resume first if WIP exists"
    if info and info["age_s"] is not None and info["age_s"] > 1200:
        return "SILENT_HANG: events frozen with no logged error; " \
               "resume-interrupted is the cheap first recovery"
    return "NO_EVIDENCE: nothing in the logs; fresh dispatch is as " \
           "good as resume"


def main():
    if len(sys.argv) != 2 or not sys.argv[1].isdigit():
        print(__doc__)
        return 2
    slot = ROOT / "loop" / "slots" / sys.argv[1]
    if not slot.is_dir():
        print(f"no such slot: {slot}")
        return 2
    print(classify(slot))
    return 0


if __name__ == "__main__":
    sys.exit(main())
