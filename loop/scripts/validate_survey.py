"""Validate SURVEY.json rows against the tree (the V10 class, run manually
because the owner amendment moved per-packet verifies to the program end but
survey rows feed Wave-2 scoping NOW).

Checks per row: (1) file exists; (2) line is within the file; (3) the row's
symbol OR at least one identifier token of the expression appears on that
line (exact or within +/-2 lines). Prints bad rows and a summary.
"""
import json
import os
import re
import sys

def ident_tokens(s):
    return [t for t in re.findall(r"[A-Za-z_][A-Za-z0-9_]*", s or "") if len(t) > 2]

def main(path):
    with open(path, "rb") as f:
        raw = f.read()
    if raw.startswith(b"\xef\xbb\xbf"):
        raw = raw[3:]
    d = json.loads(raw.decode("utf-8-sig"))
    rows = d.get("rows", [])
    bad = []
    cache = {}
    for r in rows:
        f = r.get("file", "")
        ln = r.get("line")
        expr = r.get("expression", "") or ""
        sym = r.get("symbol", "") or ""
        if not os.path.exists(f):
            bad.append(("NO_FILE", r))
            continue
        if f not in cache:
            with open(f, "r", encoding="utf-8", errors="replace") as fh:
                cache[f] = fh.read().splitlines()
        lines = cache[f]
        if not isinstance(ln, int) or not (1 <= ln <= len(lines)):
            bad.append(("BAD_LINE", r))
            continue
        window = "\n".join(lines[max(0, ln - 3): ln + 2])
        toks = ident_tokens(sym) + ident_tokens(expr)
        code_toks = [t for t in toks if t not in
                     ("interval_at", "self", "let", "fn", "pub")]
        if not code_toks:
            bad.append(("NO_TOKENS", r))
            continue
        hits = sum(1 for t in code_toks if t in window)
        if hits == 0:
            bad.append(("TOKEN_MISS", r))
    print(f"rows={len(rows)} bad={len(bad)}")
    by = {}
    for kind, r in bad:
        by.setdefault(kind, []).append(r)
    for kind, rs in sorted(by.items()):
        print(f"-- {kind}: {len(rs)}")
        for r in rs[:12]:
            print(f"   {r.get('file')}:{r.get('line')} {r.get('symbol')} | {str(r.get('expression'))[:70]}")
    return 1 if bad else 0

if __name__ == "__main__":
    sys.exit(main(sys.argv[1] if len(sys.argv) > 1 else "SURVEY.json"))
