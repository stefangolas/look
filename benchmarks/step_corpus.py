"""Render a corpus of STEP files and report outcomes, timings, and memory.

This exists because measuring STEP on real CAD files is easy to get wrong in
ways that produce confident, plausible, wrong numbers. Every guard below is
here because its absence already cost a bad result:

- A POSIX-style path handed to a Windows executable fails silently or with a
  confusing error, and once produced a comparison of filenames rather than
  triangle counts that read as a clean pass.
- A machine short of memory turns every timing into a measurement of the page
  file. Peak working set is also trimmed under pressure, so peaks taken under
  different machine states are not comparable and must not be differenced.
- A file that never finishes is its own outcome, and one of them must not cost
  the census of the others.
- Under heavy paging a wait can overshoot its budget by minutes and the process
  still completes, so success is judged by outcome rather than by whether the
  timeout flag fired.

Usage:

    python benchmarks/step_corpus.py --tool look --exe target/release/look.exe \\
        --dir path/to/corpus --out results.json

    python benchmarks/step_corpus.py --tool f3d \\
        --exe "C:/Program Files/F3D/bin/f3d-console.exe" --dir path/to/corpus

Compare two runs of the same set with --baseline.
"""

from __future__ import annotations

import argparse
import ctypes
import json
import os
import subprocess
import sys
import time
from collections import Counter
from ctypes import wintypes

# Below this much free physical memory, a timing says more about the page file
# than about the program. The largest models here peak above 3 GB.
DEFAULT_MIN_FREE_GB = 4.0


class _MemoryCounters(ctypes.Structure):
    _fields_ = [
        ("cb", wintypes.DWORD),
        ("PageFaultCount", wintypes.DWORD),
        ("PeakWorkingSetSize", ctypes.c_size_t),
        ("WorkingSetSize", ctypes.c_size_t),
        ("QuotaPeakPagedPoolUsage", ctypes.c_size_t),
        ("QuotaPagedPoolUsage", ctypes.c_size_t),
        ("QuotaPeakNonPagedPoolUsage", ctypes.c_size_t),
        ("QuotaNonPagedPoolUsage", ctypes.c_size_t),
        ("PagefileUsage", ctypes.c_size_t),
        ("PeakPagefileUsage", ctypes.c_size_t),
    ]


class _MemoryStatus(ctypes.Structure):
    _fields_ = [
        ("dwLength", ctypes.c_ulong),
        ("dwMemoryLoad", ctypes.c_ulong),
        ("ullTotalPhys", ctypes.c_ulonglong),
        ("ullAvailPhys", ctypes.c_ulonglong),
        ("ullTotalPageFile", ctypes.c_ulonglong),
        ("ullAvailPageFile", ctypes.c_ulonglong),
        ("ullTotalVirtual", ctypes.c_ulonglong),
        ("ullAvailVirtual", ctypes.c_ulonglong),
        ("ullAvailExtendedVirtual", ctypes.c_ulonglong),
    ]


def peak_working_set(handle: int) -> int:
    if not sys.platform.startswith("win"):
        return 0
    counters = _MemoryCounters()
    counters.cb = ctypes.sizeof(_MemoryCounters)
    if ctypes.windll.psapi.GetProcessMemoryInfo(handle, ctypes.byref(counters), counters.cb):
        return counters.PeakWorkingSetSize
    return 0


def free_physical_bytes() -> int:
    if not sys.platform.startswith("win"):
        return 0
    status = _MemoryStatus()
    status.dwLength = ctypes.sizeof(_MemoryStatus)
    if ctypes.windll.kernel32.GlobalMemoryStatusEx(ctypes.byref(status)):
        return status.ullAvailPhys
    return 0


def check_path(path: str, what: str) -> str:
    """Reject a POSIX-style path before it silently produces nothing.

    Git Bash renders `C:\\x` as `/c/x`, which every Windows executable and
    Python itself will fail to open. The failure is quiet enough to be mistaken
    for an empty corpus.
    """
    if sys.platform.startswith("win") and path.startswith("/"):
        raise SystemExit(
            f"{what} looks like a POSIX path: {path}\n"
            "Windows executables cannot open these. Use a drive-letter path, "
            'e.g. "C:/Users/...".'
        )
    return path


def collect(directory: str) -> list[str]:
    found = []
    for root, _, names in os.walk(directory):
        for name in names:
            if name.lower().endswith((".step", ".stp")):
                found.append(os.path.join(root, name))
    found.sort(key=os.path.getsize)
    return found


def look_command(exe: str, model: str, out: str, resolution: str) -> list[str]:
    return [exe, "render", model, "--resolution", resolution, "--output", out, "--json"]


def f3d_command(exe: str, model: str, out: str, resolution: str) -> list[str]:
    # F3D does not select its OpenCASCADE reader for .stp/.step on its own.
    width, _, height = resolution.partition("x")
    return [
        exe, model, "--no-config", "--force-reader=STEP", "--output", out,
        "--resolution", f"{width},{height}", "--camera-direction=-Z",
        "--camera-orthographic", "--anti-aliasing=none", "--ambient-occlusion=0",
        "--tone-mapping=0", "--background-color", "#252525",
    ]


def classify(record: dict) -> str:
    """Cluster an outcome by root cause. The value of a corpus run is the set
    of distinct failure modes, not the count of failing files."""
    error = record.get("error")
    if not error:
        return "ok"
    lowered = error.lower()
    for needle, label in [
        ("timeout", "timeout"),
        ("radius of sphere", "panic: tolerance exceeds sphere radius"),
        ("notsortedvector", "panic: unsorted knot vector"),
        ("index out of bounds", "panic: index out of bounds"),
        ("memory allocation", "out of memory"),
        ("no shells or tessellated", "no renderable geometry"),
    ]:
        if needle in lowered:
            return label
    return "other"


def run_one(command: list[str], model: str, out: str, timeout_s: int) -> dict:
    if os.path.exists(out):
        os.remove(out)
    free_before = free_physical_bytes()
    started = time.time()
    process = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    timed_out = False
    try:
        stdout, stderr = process.communicate(timeout=timeout_s)
    except subprocess.TimeoutExpired:
        timed_out = True
        process.kill()
        stdout, stderr = process.communicate()
    wall_ms = (time.time() - started) * 1000.0

    record = {
        "file": os.path.basename(model),
        "mb": os.path.getsize(model) / 1e6,
        "exit": process.returncode,
        "wall_ms": wall_ms,
        "peak_mb": peak_working_set(int(process._handle)) / 1e6,
        "free_gb_before": free_before / 2 ** 30,
        "free_gb_after": free_physical_bytes() / 2 ** 30,
    }

    produced = os.path.exists(out) and os.path.getsize(out) > 0
    if produced and process.returncode == 0:
        # Judged by outcome: under paging the wait can overshoot and the
        # process still finish, which otherwise reads as a spurious timeout.
        if timed_out or wall_ms > timeout_s * 1000 * 1.1:
            record["note"] = "completed past its budget; timing unreliable"
        if stdout.strip():
            try:
                parsed = json.loads(stdout)
                timings = parsed.get("timings_ms", {})
                for key in ("step_parse", "step_table", "step_tessellate", "total"):
                    if key in timings:
                        record[key] = timings[key]
                record["triangles"] = (
                    parsed.get("scene", {}).get("statistics", {}).get("triangles")
                )
            except json.JSONDecodeError:
                pass
        # Warnings matter even on success: faces can go missing silently.
        for line in stderr.splitlines():
            if line.startswith("warning:"):
                record.setdefault("warnings", []).append(" ".join(line.split())[:200])
    elif timed_out:
        record["error"] = f"TIMEOUT after {timeout_s}s, killed"
    else:
        record["error"] = " ".join(stderr.split())[-400:] or "(no stderr)"
    return record


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--tool", choices=("look", "f3d"), default="look")
    parser.add_argument("--exe", required=True)
    parser.add_argument("--dir", help="directory searched recursively for STEP files")
    parser.add_argument("--files", nargs="*", default=[])
    parser.add_argument("--out", default="step_corpus_results.json")
    parser.add_argument("--resolution", default="512x512")
    parser.add_argument("--timeout", type=int, default=300)
    parser.add_argument("--min-free-gb", type=float, default=DEFAULT_MIN_FREE_GB)
    parser.add_argument(
        "--allow-low-memory", action="store_true",
        help="record timings anyway, marking every sample untrusted")
    parser.add_argument("--baseline", help="a previous results file to compare against")
    args = parser.parse_args()

    check_path(args.exe, "--exe")
    # Windows does not resolve a relative executable against the working
    # directory the way a shell does, so a relative --exe fails with a bare
    # "cannot find the file specified" that reads like a missing corpus.
    args.exe = os.path.abspath(args.exe)
    if not os.path.isfile(args.exe):
        raise SystemExit(f"--exe does not exist: {args.exe}")
    if args.dir:
        check_path(args.dir, "--dir")
    for path in args.files:
        check_path(path, "a --files entry")

    models = list(args.files) + (collect(args.dir) if args.dir else [])
    if not models:
        raise SystemExit("no STEP files found")

    free_gb = free_physical_bytes() / 2 ** 30
    untrusted = free_gb < args.min_free_gb
    if untrusted and not args.allow_low_memory:
        raise SystemExit(
            f"only {free_gb:.1f} GB physical memory free, below --min-free-gb "
            f"{args.min_free_gb}.\nTimings taken here measure the page file, and "
            "peak working set is trimmed under pressure so peaks cannot be "
            "compared against another run. Free memory, or pass "
            "--allow-low-memory to record anyway with every sample marked "
            "untrusted."
        )

    out_png = os.path.join(os.path.dirname(os.path.abspath(args.out)) or ".", "_corpus_out.png")
    build = look_command if args.tool == "look" else f3d_command

    results = []
    for model in models:
        record = run_one(build(args.exe, model, out_png, args.resolution),
                         model, out_png, args.timeout)
        record["tool"] = args.tool
        if untrusted:
            record["untrusted"] = "recorded below the free-memory threshold"
        record["outcome"] = classify(record)
        results.append(record)
        print(json.dumps(record), flush=True)

    with open(args.out, "w") as handle:
        json.dump(results, handle, indent=2)

    print(f"\n{len(results)} files, {args.tool}")
    for outcome, count in Counter(r["outcome"] for r in results).most_common():
        print(f"  {count:3}  {outcome}")
    missing = [r for r in results if r.get("warnings")]
    if missing:
        print(f"  {len(missing)} rendered with warnings (geometry may be incomplete)")
    if untrusted:
        print("  NOTE: every sample is marked untrusted; free memory was below "
              "the threshold.")

    if args.baseline:
        with open(args.baseline) as handle:
            before = {r["file"]: r for r in json.load(handle)}
        print("\nagainst baseline:")
        for record in results:
            was = before.get(record["file"])
            if not was:
                continue
            if was["outcome"] != record["outcome"]:
                print(f"  {record['file'][:8]}: {was['outcome']} -> {record['outcome']}")
            a, b = was.get("triangles"), record.get("triangles")
            if a is not None and b is not None and a != b:
                print(f"  {record['file'][:8]}: triangles {a} -> {b}")
        # Timings are only comparable when both runs had comparable headroom.
        if any(r.get("untrusted") for r in results) or any(
                r.get("untrusted") for r in before.values()):
            print("  timings not compared: one of the runs is marked untrusted")


if __name__ == "__main__":
    main()
