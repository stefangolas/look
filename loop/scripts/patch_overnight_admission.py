"""Session-51: make overnight's RESULT admission semantic, not schema-bound.

Workers drifted twice (status='landed', status absent + outcome='completed');
the scoped check is the real gate, so admit on stop_conditions.triggered
being falsy plus a success-shaped status/outcome word.
"""
import ast

src = open("loop/overnight.py", encoding="utf-8").read()
old = '''    status = (result.get("status") or "").lower()
    fails = result.get("fail_count")
    if status not in ("complete", "partial", "done") or \\
            (isinstance(fails, int) and fails > 0):
        log(f"slot {slot_no}: {pid} status={status!r} fails={fails} - "
            f"LEFT FOR MORNING (judgment required)")
        return'''
new = '''    status = (result.get("status") or result.get("outcome") or "").lower()
    fails = result.get("fail_count")
    stopped = bool((result.get("stop_conditions") or {}).get("triggered"))
    good = status in ("complete", "partial", "done", "landed", "completed")
    if stopped or not good or (isinstance(fails, int) and fails > 0):
        log(f"slot {slot_no}: {pid} status={status!r} fails={fails} "
            f"stopped={stopped} - LEFT FOR MORNING (judgment required)")
        return'''
assert old in src, "admission block not found"
src = src.replace(old, new)
open("loop/overnight.py", "w", encoding="utf-8", newline="\n").write(src)
ast.parse(src)
print("admission patched + parses OK")
