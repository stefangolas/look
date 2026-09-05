"""Session-51 gate fix: the battery's clippy parser attributed the
`note: the lint level is defined here --> lib.rs:4:9` arrows to lib.rs,
failing the modified-file check on pre-existing formal/ findings. A finding's
primary location is the arrow IMMEDIATELY following its `error:` line.
"""
import ast

src = open("loop/overnight.py", encoding="utf-8").read()
old = '''            for m in re.finditer(r"--> (\\S+?):\\d+:\\d+",
                                 (r.stdout or "") + (r.stderr or "")):'''
assert old in src, "parser regex not found"
new = '''            # The primary span is the arrow IMMEDIATELY after the error
            # line; later arrows belong to help/note blocks (the lint-level
            # note points at lib.rs and mis-attributed pre-existing
            # formal/ findings to the modified lib.rs - session 51).
            for m in re.finditer(r"^error:[^\\n]*\\n\\s*--> (\\S+?):\\d+:\\d+",
                                 (r.stdout or "") + (r.stderr or ""),
                                 re.MULTILINE):'''
src = src.replace(old, new)
open("loop/overnight.py", "w", encoding="utf-8", newline="\n").write(src)
ast.parse(src)
print("parser fix applied + parses OK")
