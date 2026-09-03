"""cargoq server: owns ALL cargo invocations for the KV2 swarm.

One heavy job at a time (FIFO), per-job timeout, output buffered and
returned with the exit code. Agents block on the HTTP call exactly as
they would block on cargo itself; peak RAM = agents + ONE cargo spike.

- POST /run  body {"args": [...], "timeout": seconds-optional}
             -> {"exit": int, "stdout": str, "stderr": str}
- GET /ping  -> {"ok": true, "queued": n, "running": bool}
- GET /stats -> the log of finished jobs (tail)

Fallback contract: if the server is down, the client shim runs cargo
DIRECTLY (degraded to the old spike-overlap behavior, never wedged).
"""
import json
import os
import subprocess
import sys
import threading
import time
from collections import deque
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

PORT = int(os.environ.get("CARGOQ_PORT", "8231"))
DEFAULT_TIMEOUT = int(os.environ.get("CARGOQ_TIMEOUT", "2400"))
LOG = os.path.join(os.path.dirname(os.path.abspath(__file__)), "server.log")

_lock = threading.Lock()
_queue = deque()
_running = None          # (args, thread, started)
_finished = deque(maxlen=200)


def reset_running():
    global _running
    _running = None


def log(line):
    with open(LOG, "a", encoding="utf-8") as f:
        f.write(f"{time.strftime('%H:%M:%S')} {line}\n")


def resolve_cargo():
    path = os.environ.get("PATH", "")
    here = os.path.dirname(os.path.abspath(__file__))
    for d in path.split(os.pathsep):
        if os.path.abspath(d or ".") == here:
            continue
        for ext in (".exe", ".cmd", ".bat", ""):
            c = os.path.join(d, "cargo" + ext)
            if os.path.isfile(c):
                return c
    return None


CARGO = resolve_cargo()
env = dict(os.environ)
env.setdefault("CARGO_BUILD_JOBS", "2")
here = os.path.dirname(os.path.abspath(__file__))
env["PATH"] = os.pathsep.join(
    p for p in env.get("PATH", "").split(os.pathsep)
    if os.path.abspath(p or ".") != here)


def worker(job, cond):
    global _running
    args, timeout = job["args"], job.get("timeout") or DEFAULT_TIMEOUT
    t0 = time.time()
    log(f"START cargo {' '.join(args)}")
    try:
        proc = subprocess.Popen(
            [CARGO] + args, env=env, stdout=subprocess.PIPE,
            stderr=subprocess.PIPE)
        try:
            out, err = proc.communicate(timeout=timeout)
        except subprocess.TimeoutExpired:
            proc.kill()
            out, err = proc.communicate()
            log(f"TIMEOUT after {timeout}s: cargo {' '.join(args)}")
            job["result"] = {"exit": 3,
                             "stdout": out.decode(errors="replace"),
                             "stderr": (err.decode(errors="replace") +
                                        f"\n[cargoq] killed after {timeout}s")}
            return
        job["result"] = {"exit": proc.returncode,
                         "stdout": out.decode(errors="replace"),
                         "stderr": err.decode(errors="replace")}
        log(f"DONE exit={proc.returncode} in {int(time.time()-t0)}s: "
            f"cargo {' '.join(args)}")
    except Exception as e:  # noqa: BLE001 - the server must never die
        job["result"] = {"exit": 127, "stdout": "",
                         "stderr": f"[cargoq] server error: {e}"}
        log(f"ERROR {e}: cargo {' '.join(args)}")
    finally:
        with cond:
            reset_running()
            cond.notify()


def pump():
    """Background thread: pop the FIFO, run ONE job at a time."""
    global _running
    cond = _cond
    while True:
        with cond:
            while not _queue or _running is not None:
                cond.wait()
            job = _queue.popleft()
            _running = (job["args"], time.time())
            threading.Thread(target=worker, args=(job, cond),
                             daemon=True).start()


_cond = threading.Condition()
threading.Thread(target=pump, daemon=True).start()


class Handler(BaseHTTPRequestHandler):
    def _send(self, obj, code=200):
        body = json.dumps(obj).encode()
        self.send_response(code)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_GET(self):
        if self.path == "/ping":
            self._send({"ok": True, "queued": len(_queue),
                        "running": _running is not None})
        elif self.path == "/stats":
            self._send({"finished": list(_finished)[-30:],
                        "queued": len(_queue),
                        "running": _running[0] if _running else None})
        else:
            self._send({"error": "not found"}, 404)

    def do_POST(self):
        if self.path != "/run":
            self._send({"error": "not found"}, 404)
            return
        try:
            body = json.loads(self.rfile.read(
                int(self.headers.get("Content-Length", "0"))))
        except Exception:  # noqa: BLE001
            self._send({"error": "bad body"}, 400)
            return
        args = body.get("args") or []
        if not args or CARGO is None:
            self._send({"exit": 127, "stdout": "",
                        "stderr": "[cargoq] no args or no cargo"}, 200)
            return
        job = {"args": args, "timeout": body.get("timeout")}
        cond = _cond
        with cond:
            _queue.append(job)
            cond.notify()  # wake the pump thread (the deadlock class: appending without notifying)
        # Poll for the result (simple; workers are few and jobs are long)
        while not job.get("result"):
            time.sleep(0.5)
        r = job.pop("result")
        _finished.append({"args": args, "exit": r["exit"]})
        self._send(r)

    def log_message(self, format, *args):  # noqa: A002 - stdlib signature
        pass  # silence the default request log


if __name__ == "__main__":
    if CARGO is None:
        print("cargoq: real cargo not found", file=sys.stderr)
        sys.exit(1)
    log(f"SERVER START port={PORT} cargo={CARGO}")
    ThreadingHTTPServer(("127.0.0.1", PORT), Handler).serve_forever()
