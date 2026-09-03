"""janitor: disk-space allocation service for the KV2 swarm.

Agents churning code are cheap on disk; specific OPS (slot warms,
baselines, verifies) eat GB. This service reclaims known-safe caches in
the recorded priority order and reports the allocation picture. It never
touches a LIVE slot's target (the watchdog's guard_disk list is honored:
running pids own their targets).

Usage:
  python loop/janitor.py status
  python loop/janitor.py ensure --need 8      # reclaim until >= N GB free
Exit 0 when the requirement holds, 1 otherwise (caller refuses the op).
"""
import os
import shutil
import subprocess
import sys
import time

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SLOTS = os.path.join(ROOT, "loop", "slots")
RECLAIM_LOG = os.path.join(ROOT, "loop", "janitor.log")

GB = 1024 ** 3


def free_gb():
    drive = os.path.splitdrive(ROOT)[0] or "C:"
    free = shutil.disk_usage(drive + "\\").free
    return free / GB


def dir_size_gb(path):
    total = 0
    for dirpath, _, files in os.walk(path):
        for f in files:
            try:
                total += os.path.getsize(os.path.join(dirpath, f))
            except OSError:
                pass
    return total / GB


def live_slot_pids():
    """Slots with a LIVE worker shim process.

    Detection = Win32_Process command lines containing
    'loop\\slots\\N\\worker-cmd.bat' (the same shape the watchdog uses).
    The 2026-08-19 recorded incident: guard_disk rmtree'd a target under
    a live cargo because detection trusted stale bookkeeping. This
    function trusts RUNNING PROCESSES only.
    """
    live = set()
    try:
        out = subprocess.run(
            ["powershell", "-NoProfile", "-Command",
             "Get-CimInstance Win32_Process -Filter \"Name='cmd.exe'\" "
             "| Select-Object -ExpandProperty CommandLine"],
            capture_output=True, text=True, timeout=30).stdout
    except Exception:
        out = ""
    for line in out.splitlines():
        low = line.lower().replace("/", "\\")
        if "worker-cmd.bat" in low and "loop" in low and "slots" in low:
            try:
                seg = low.split("slots")[1].strip("\\ \t\"'")
                name = seg.split("\\")[0].strip(" \t\"'")
                if name.isdigit():
                    live.add(name)
            except Exception:
                pass
    return live


def reclaim(skip_slots, dry=False):
    """Priority order, recorded in ORCHESTRATOR. Returns GB freed (est)."""
    freed = 0.0
    # 1. repo-root target (regenerates; it is the orchestrator's own check cache)
    t = os.path.join(ROOT, "target")
    if os.path.isdir(t):
        s = dir_size_gb(t)
        if not dry:
            shutil.rmtree(t, ignore_errors=True)
        log(f"reclaim repo-root target {s:.1f} GB")
        freed += s
    # 2. idle slot targets (slot whose worker-cmd.bat is NOT a live process)
    if os.path.isdir(SLOTS):
        live = live_slot_pids()
        for name in sorted(os.listdir(SLOTS)):
            if name in live or name in skip_slots:
                continue
            for cand in (os.path.join(SLOTS, name, "target"),
                         os.path.join(SLOTS, name, "wt", "target")):
                if os.path.isdir(cand):
                    s = dir_size_gb(cand)
                    if not dry:
                        shutil.rmtree(cand, ignore_errors=True)
                    log(f"reclaim idle slot {name} target {s:.1f} GB")
                    freed += s
    # 3. TEMP leaks (proc-macro-srv, look-verify baselines, old build dirs)
    tmp = os.environ.get("TEMP", "")
    if os.path.isdir(tmp):
        cutoff = time.time() - 2 * 3600
        for name in sorted(os.listdir(tmp)):
            if not (name.startswith("proc-macro")
                    or name.startswith("look-verify")):
                continue
            p = os.path.join(tmp, name)
            try:
                if os.path.getmtime(p) > cutoff:
                    continue
                s = dir_size_gb(p)
                if not dry:
                    shutil.rmtree(p, ignore_errors=True)
                log(f"reclaim TEMP {name} {s:.1f} GB")
                freed += s
            except OSError:
                pass
    # 4. worktree prune (metadata; frees little but is safe)
    if not dry:
        subprocess.run(["git", "worktree", "prune"], cwd=ROOT,
                       capture_output=True)
    return freed


def log(line):
    with open(RECLAIM_LOG, "a", encoding="utf-8") as f:
        f.write(f"{time.strftime('%m-%d %H:%M:%S')} {line}\n")


def free_ram_gb():
    out = subprocess.run(
        ["powershell", "-NoProfile", "-Command",
         "(Get-CimInstance Win32_OperatingSystem).FreePhysicalMemory"],
        capture_output=True, text=True, timeout=30).stdout.strip()
    try:
        return int(out) * 1024 / GB
    except ValueError:
        return -1.0


def kill_worker_language_servers(dry=False):
    """Kill rust-analyzer instances PARENTED by an opencode/bun process.

    The session-51 class: opencode auto-spawns a language server per
    worker session over a cold slot worktree - 1-2+ GB each, a duplicate
    index of the same tree. Worker dispatch now sets lsp:false in the
    config env, so this is the backstop for processes that predate it or
    slip past it. Editor-spawned instances (VS Code parent chain) are
    exempt by parentage - the owner's own rust-analyzer is never touched.
    """
    ps = ("Get-CimInstance Win32_Process -Filter \"Name='rust-analyzer.exe'\" "
          "| ForEach-Object { $p = $_; $par = Get-CimInstance Win32_Process "
          "-Filter ('ProcessId=' + $p.ParentProcessId) -ErrorAction "
          "SilentlyContinue; '{0}|{1}' -f $p.ProcessId, "
          "$(if ($par) { $par.Name } else { '?' }) }")
    out = subprocess.run(["powershell", "-NoProfile", "-Command", ps],
                         capture_output=True, text=True, timeout=30).stdout
    killed = 0
    for line in out.splitlines():
        if "|" not in line:
            continue
        pid, parent = line.split("|", 1)
        if not pid.strip().isdigit():
            continue
        if parent.strip().lower() not in ("opencode.exe", "bun.exe",
                                          "node.exe"):
            continue
        if not dry:
            subprocess.run(["taskkill", "/PID", pid.strip(), "/T", "/F"],
                           capture_output=True)
        log(f"kill worker language server pid {pid.strip()} "
            f"(parent {parent.strip()})")
        killed += 1
    return killed


def main():
    if len(sys.argv) < 2:
        print(__doc__)
        return 2
    if sys.argv[1] == "status":
        f = free_gb()
        print(f"free: {f:.1f} GB disk, {free_ram_gb():.1f} GB RAM")
        if os.path.isdir(SLOTS):
            for name in sorted(os.listdir(SLOTS)):
                for cand in (os.path.join(SLOTS, name, "target"),
                             os.path.join(SLOTS, name, "wt", "target")):
                    if os.path.isdir(cand):
                        print(f"  slot {name}: {dir_size_gb(cand):.1f} GB "
                              f"({os.path.relpath(cand, ROOT)})")
        return 0
    if sys.argv[1] == "ram":
        # RAM reclaim: kill opencode-parented language servers regardless
        # of threshold when invoked directly.
        killed = kill_worker_language_servers()
        print(f"janitor: killed {killed} worker language server(s); "
              f"{free_ram_gb():.1f} GB RAM free")
        return 0
    if sys.argv[1] == "ensure":
        need = float(sys.argv[sys.argv.index("--need") + 1])
        skip = {a for a in sys.argv[2:] if a.startswith("--slot=")}
        # RAM first (session 51: 0xc0000409 under allocation pressure is
        # the memory signature; a worker language server is the one
        # reclaimable consumer the disk sweep never touched).
        if free_ram_gb() < 4.0:
            kill_worker_language_servers()
        f = free_gb()
        if f >= need:
            print(f"janitor: {f:.1f} GB free, need {need} - OK")
            return 0
        print(f"janitor: {f:.1f} GB free < {need} needed - reclaiming")
        freed = reclaim(skip)
        f = free_gb()
        ok = f >= need
        print(f"janitor: reclaimed ~{freed:.1f} GB -> {f:.1f} GB free "
              f"({'OK' if ok else 'STILL SHORT'})")
        return 0 if ok else 1
    print(__doc__)
    return 2


if __name__ == "__main__":
    sys.exit(main())
