"""Session-51, second harness gap: archive uncommitted partial code.

The pending-archive patch saves RESULT/QUESTION before a left-for-morning
return, but not uncommitted CODE (the skipped-commit class: worker wrote
code, exited before committing). A recycled worktree wipes it. This extends
the same path: `git diff HEAD` (tracked) plus untracked-file listing into a
patch file beside the archived RESULT.
"""
import ast

src = open("loop/overnight.py", encoding="utf-8").read()
old = '''                tag = "PENDING-QUESTION" if fname == "QUESTION.md" else "PENDING"
                shutil.copy(f, ROOT / "loop" / "results" / f"{pid}.{tag}.{fname}")'''
new = '''                tag = "PENDING-QUESTION" if fname == "QUESTION.md" else "PENDING"
                shutil.copy(f, ROOT / "loop" / "results" / f"{pid}.{tag}.{fname}")
        # uncommitted partial code: tracked diff + untracked files
        diff = git(["diff", "HEAD"], cwd=slot_dir / "wt")
        untracked = git(["ls-files", "--others", "--exclude-standard"],
                        cwd=slot_dir / "wt")
        if (diff or "").strip() or (untracked or "").strip():
            patch_path = ROOT / "loop" / "results" / f"{pid}.PENDING.partial.patch"
            with open(patch_path, "w", encoding="utf-8", errors="replace") as pf:
                pf.write(diff or "")
                for uf in (untracked or "").splitlines():
                    if not uf.strip() or fname_guard(uf):
                        continue
                    uf_path = slot_dir / "wt" / uf
                    try:
                        content = uf_path.read_text(encoding="utf-8",
                                                    errors="replace")
                    except Exception:
                        continue
                    pf.write(f"\\n--- /dev/null\\n+++ /dev/null/{uf}\\n")
                    pf.write("".join("+" + ln + "\\n" for ln in
                                     content.splitlines()))
            log(f"slot {slot_no}: partial work archived to {patch_path.name}")'''
assert old in src
src = src.replace(old, new)
# helper: skip harness-owned files from untracked archival
src = src.replace("def try_land(", '''def fname_guard(uf):
    return uf in ("RESULT.json", "QUESTION.md", "CONTEXT.md", "PACKET.md")


def try_land(''', 1)
open("loop/overnight.py", "w", encoding="utf-8", newline="\n").write(src)
ast.parse(src)
print("partial-code archive patch applied + parses OK")
