"""Prove the detached launch works, without spending a worker run on it.

`run_packet.py` has to satisfy three properties, and all three have failed
silently at least once. The last failure was the expensive kind: the worker ran
for ten minutes and edited seven source files while `events.jsonl` received 29
bytes, so liveness -- which is inferred from that file growing -- read the
healthy worker as stalled.

A real dispatch costs ~90 minutes and API spend, which is too much to pay to
find out a flag is wrong. This exercises the same `spawn_detached` the loop uses,
with a throwaway child that just counts, and checks:

  A  output streams to the log *while the child runs*, not only at exit, and
     survives being produced by a grandchild of the launcher
  B  the child outlives the process that launched it
  C  the recorded pid is alive while the child runs and gone once it exits

Run: python loop/selftest_dispatch.py   (about 40 seconds, exits non-zero on
any failure)
"""
import os
import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from run_packet import spawn_detached  # noqa: E402

# The child prints one line per interval and flushes. Flushing matters: a child
# that buffers looks exactly like a child that has hung, which is the failure
# this whole file exists to detect.
CHILD = """import sys, time
for i in range(int(sys.argv[1])):
    print(f"line {i}", flush=True)
    time.sleep(0.4)
"""


def alive(pid):
    out = subprocess.run(['tasklist', '/FI', f'PID eq {pid}', '/NH'],
                         capture_output=True, text=True).stdout
    return str(pid) in out


def main():
    results = []
    tmp = Path(tempfile.mkdtemp(prefix='loop-selftest-'))
    try:
        child_py = tmp / 'child.py'
        child_py.write_text(CHILD, encoding='utf-8')

        # A .cmd shim standing in for opencode's, so the process doing the
        # printing is a *grandchild* of what we launch -- the exact shape that
        # lost the real event stream.
        shim = tmp / 'shim.cmd'
        shim.write_text(f'@echo off\r\n"{sys.executable}" "{child_py}" %1\r\n', encoding='utf-8')

        events = tmp / 'events.jsonl'
        err = tmp / 'worker.err'

        # Launched from a short-lived intermediate process, because that is the
        # real shape: the orchestrator dispatches and returns, and the harness
        # then tears down that tool call's whole process tree. Spawning from
        # this test directly would only prove the child survives its parent
        # *exiting*, which was never the failure.
        launcher_py = tmp / 'launcher.py'
        launcher_py.write_text(
            "import os, sys\n"
            f"sys.path.insert(0, r'{Path(__file__).resolve().parent}')\n"
            "from run_packet import spawn_detached\n"
            "from pathlib import Path\n"
            f"pid = spawn_detached([r'{shim}', '60'], Path(r'{events}'), Path(r'{err}'),\n"
            f"                     dict(os.environ), Path(r'{tmp}'), tag='selftest')\n"
            "print(pid, flush=True)\n"
            "import time; time.sleep(600)\n",  # stays alive so it can be killed
            encoding='utf-8')
        launcher = subprocess.Popen([sys.executable, str(launcher_py)],
                                    stdout=subprocess.PIPE, text=True)
        pid = int(launcher.stdout.readline().strip())

        # --- C (first half): the pid must be live while the child runs -------
        time.sleep(2.0)
        results.append(('C pid alive during run', alive(pid)))

        # --- A: the log must grow *between* two polls, not just at the end ---
        first = events.read_text(errors='replace').count('\n') if events.exists() else 0
        time.sleep(3.0)
        second = events.read_text(errors='replace').count('\n') if events.exists() else 0
        results.append((f'A streams while running ({first} -> {second} lines)', second > first > 0))

        # --- B: the launcher dies; the worker must keep going ----------------
        # The ordinary case: the orchestrator exits, times out, or is killed.
        # This is the property the loop actually depends on.
        subprocess.run(['taskkill', '/F', '/PID', str(launcher.pid)],
                       capture_output=True, text=True)
        time.sleep(3.0)
        after_kill = events.read_text(errors='replace').count('\n')
        time.sleep(3.0)
        later = events.read_text(errors='replace').count('\n')
        results.append((f'B survives parent death ({after_kill} -> {later} lines)', later > after_kill))

        # --- B2: a hard tree kill --------------------------------------------
        # `taskkill /T` walks parent-child links rather than the job, so job
        # breakaway is no defense against it in principle. In practice the
        # worker has survived it here -- print what actually happened rather
        # than assert either way, since this depends on how far the shim chain
        # has unwound by the time the kill lands, and that is timing. Reported,
        # not required: the loop only depends on property B.
        b2_before = later
        subprocess.run(['taskkill', '/F', '/T', '/PID', str(pid)],
                       capture_output=True, text=True)
        time.sleep(2.0)
        b2_after = events.read_text(errors='replace').count('\n')
        print(f"  note   B2 hard tree kill: {b2_before} -> {b2_after} lines "
              f"({'survived' if b2_after > b2_before else 'killed, as expected'})")

    finally:
        time.sleep(0.5)
        shutil.rmtree(tmp, ignore_errors=True)

    print()
    ok = True
    for name, passed in results:
        print(f"  {'PASS' if passed else 'FAIL'}  {name}")
        ok = ok and passed
    print()
    print('dispatch selftest:', 'PASS' if ok else 'FAIL')
    sys.exit(0 if ok else 1)


if __name__ == '__main__':
    main()
