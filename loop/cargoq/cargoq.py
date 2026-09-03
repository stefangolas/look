"""cargoq client shim: named `cargo` on PATH, forwards to the queue server.

Falls back to DIRECT execution when the server is unreachable (degraded
to spike-overlap behavior, never wedged). Streams nothing live: the
server buffers and returns the full output with the exit code.
"""
import json
import os
import subprocess
import sys
import time
import urllib.request

PORT = int(os.environ.get("CARGOQ_PORT", "8231"))
TIMEOUT = int(os.environ.get("CARGOQ_TIMEOUT", "2400"))


def direct():
    """Fallback: run cargo directly. LOUD — every bypass is logged."""
    here = os.path.dirname(os.path.abspath(__file__))
    with open(os.path.join(here, "fallback.log"), "a",
              encoding="utf-8") as f:
        f.write(f"{time.strftime('%Y-%m-%d %H:%M:%S')} DIRECT: "
                f"cargo {' '.join(sys.argv[1:])}\n")
    sys.stderr.write("[cargoq] server unreachable, running DIRECT "
                     "(recorded in fallback.log)\n")
    path = [p for p in os.environ.get("PATH", "").split(os.pathsep)
            if os.path.abspath(p or ".") != here]
    env = dict(os.environ)
    env.setdefault("CARGO_BUILD_JOBS", "2")
    env["PATH"] = os.pathsep.join(path)
    cargo = None
    for d in path:
        for ext in (".exe", ".cmd", ".bat", ""):
            c = os.path.join(d, "cargo" + ext)
            if os.path.isfile(c):
                cargo = c
                break
        if cargo:
            break
    if cargo is None:
        sys.stderr.write("[cargoq] real cargo not found\n")
        sys.exit(127)
    proc = subprocess.Popen([cargo] + sys.argv[1:], env=env)
    sys.exit(proc.wait())


def main():
    body = json.dumps({"args": sys.argv[1:], "timeout": TIMEOUT}).encode()
    try:
        req = urllib.request.Request(
            f"http://127.0.0.1:{PORT}/run", data=body,
            headers={"Content-Type": "application/json"})
        with urllib.request.urlopen(req, timeout=TIMEOUT + 60) as r:
            result = json.loads(r.read().decode())
    except Exception:  # noqa: BLE001 - any server failure falls back
        sys.stderr.write("[cargoq] server unreachable, running direct\n")
        direct()
        return
    sys.stdout.write(result.get("stdout", ""))
    sys.stderr.write(result.get("stderr", ""))
    sys.exit(result.get("exit", 1))


if __name__ == "__main__":
    main()
