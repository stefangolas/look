"""Session-51 harness gap fix (v2): left-for-morning must preserve evidence."""
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
open("loop/overnight.py", "w", encoding="utf-8", newline="\n").write(src)
ast.parse(src)
print("pending-archive patch applied + parses OK")
