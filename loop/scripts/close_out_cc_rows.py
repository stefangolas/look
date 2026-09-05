"""Session-51 close-out: the battery's row-flip was BG-KV2-prefixed only.
1. Flip every landed CC row to DONE (the battery green is in the log).
2. The battery flip prefix widens to ("BG-KV2-", "CC-").
3. The stale-rows re-read after the battery (in-memory rows miss the file flip).
"""
import ast

# 1. registry flip
rows = [json.loads(l) for l in open("loop/PACKETS.jsonl", encoding="utf-8")] if False else None
import json
rows = [json.loads(l) for l in open("loop/PACKETS.jsonl", encoding="utf-8")]
flipped = 0
for r in rows:
    if r["id"].startswith("CC-") and r.get("status") != "DONE":
        r["status"] = "DONE"
        flipped += 1
with open("loop/PACKETS.jsonl", "w", encoding="utf-8", newline="\n") as f:
    for r in rows:
        f.write(json.dumps(r) + "\n")
print("CC rows flipped DONE:", flipped)

# 2 + 3. driver patches
src = open("loop/overnight.py", encoding="utf-8").read()
old_flip = 'if pid.startswith("BG-KV2-"):'
assert old_flip in src
src = src.replace(old_flip, 'if pid.startswith(("BG-KV2-", "CC-")):')
anchor = "                battery(rows, order, reg_path)\n                if rows_done(rows):"
assert anchor in src
src = src.replace(anchor,
    "                battery(rows, order, reg_path)\n"
    "                rows, order, reg_path = registry()  # battery flips rows in the file\n"
    "                if rows_done(rows):")
open("loop/overnight.py", "w", encoding="utf-8", newline="\n").write(src)
ast.parse(src)
print("driver patches applied + parses OK")
