"""supervisor: restart the overnight driver whenever it exits, forever.

Session-51 owner direction: walking away must not stop the program. The
driver handles mechanical adjudication + dispatch + the one-verify battery
with its own cooldown; this process is the crash-level outer loop. It also
guards the cargoq server: if the queue dies, the drivers' cargo calls
silently fall back to direct execution (fallback.log), so restart it.

Stdlib only. Log: loop/supervisor.log
"""
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
LOG = ROOT / "loop" / "supervisor.log"
RESTART_DELAY = 60


def log(msg):
    stamp = time.strftime("%m-%d %H:%M:%S")
    with open(LOG, "a", encoding="utf-8") as f:
        f.write(f"{stamp} {msg}\n")


def alive(cmd_fragment):
    out = subprocess.run(
        ["powershell", "-NoProfile", "-Command",
         "Get-CimInstance Win32_Process | "
         "Select-Object -ExpandProperty CommandLine"],
        capture_output=True, text=True).stdout
    return any(cmd_fragment in ln for ln in out.splitlines())


def main():
    log("supervisor start")
    while True:
        try:
            if not alive("overnight.py"):
                log("overnight driver not running - starting")
                subprocess.Popen(
                    [sys.executable, str(ROOT / "loop" / "overnight.py")],
                    cwd=str(ROOT), creationflags=subprocess.CREATE_NO_WINDOW)
            if not alive("cargoq"):
                log("cargoq server not running - restarting")
                subprocess.Popen(
                    [sys.executable, str(ROOT / "loop" / "cargoq" / "server.py")],
                    cwd=str(ROOT), creationflags=subprocess.CREATE_NO_WINDOW)
            if not alive("watchdog.py"):
                log("watchdog not running - restarting")
                subprocess.Popen(
                    [sys.executable, str(ROOT / "loop" / "watchdog.py")],
                    cwd=str(ROOT), creationflags=subprocess.CREATE_NO_WINDOW)
        except Exception as exc:  # never die
            log(f"supervisor cycle error: {exc!r}")
        time.sleep(RESTART_DELAY)


if __name__ == "__main__":
    sys.exit(main())
