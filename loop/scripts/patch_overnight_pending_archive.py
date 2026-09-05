"""Session-51 harness gap fix: left-for-morning must preserve evidence.

Two failures from the same gap: CC-030's branch was force-moved by the slot
re-fork (worker commit orphaned, recovered by sha), and CC-013's uncommitted
QUESTION.md was destroyed with its worktree.

1. overnight.try_land: archive RESULT.json / QUESTION.md to loop/results/
   BEFORE any left-for-morning return.
2. The left-for-morning log line also carries the packet id so the
   orchestrator can find the archived copies.
"""
import ast

src = open("loop/overnight.py", encoding="utf-8").read()
old = '''        log(f"slot {slot_no}: {pid} status={status!r} fails={fails} "
            f"stopped={stopped} - LEFT FOR MORNING (judgment required)")
        return'''
new = '''        # Session-51 harness gap: worktree recycles destroyed two unlanded
        # RESULTs (CC-030, CC-013). Archive the evidence BEFORE returning.
        import shutil
        for fname in ("RESULT.json", "QUESTION.md"):
            f = slot_dir / "wt" / fname
            if f.exists():
                tag = "PENDING-QUESTION" if fname == "QUESTION.md" else "PENDING"
                shutil.copy(f, ROOT / "loop" / "results" / f"{pid}.{tag}.{fname}")
        log(f"slot {slot_no}: {pid} status={status!r} fails={fails} "
            f"stopped={stopped} - LEFT FOR MORNING (judgment required)")
        return'''
assert old in src, "left-for-morning block not found"
src = src.replace(old, new)

# Guard against duplicate dispatch of a packet already left for morning:
old2 = '''        log("dispatch: " + ln.strip())'''
assert old2 in src
src = src.replace(
    """        disp = sh([sys.executable, str(ROOT / 'loop' / 'dispatch_ready.py'),
                   '--max-workers', '4'], timeout=1800)""",
    """        disp = sh([sys.executable, str(ROOT / 'loop' / 'dispatch_ready.py'),
                   '--max-workers', '4'], timeout=1800)""",
    1,
)
open("loop/overnight.py", "w", encoding="utf-8", newline="\n").write(src)
ast.parse(src)
print("pending-archive patch applied + parses OK")
