#!/usr/bin/env python3
"""Per-face diff of two face_census ledgers.

Ledger lines:
  FACE declared_face_index=i source_face_id=#ID surface_kind=k rendered=0/1 triangles=n stage=... reason=...

Faces are keyed by source_face_id when present, else (declared_face_index,
surface_kind). Both ledgers must come from the same corpus file order.
"""

import sys
import re
from collections import Counter


def parse(path):
    rows = []
    for line in open(path, encoding="utf-8", errors="replace"):
        if not line.startswith("FACE\t"):
            continue
        fields = {}
        for part in line.strip().split("\t"):
            if "=" in part:
                k, _, v = part.partition("=")
                fields[k] = v
        rows.append(fields)
    return rows


def key(fields):
    sid = fields.get("source_face_id")
    if sid and sid != "-":
        return ("id", sid)
    return ("idx", fields.get("declared_face_index", "-"))


def main():
    if len(sys.argv) != 3:
        print("usage: ledger_diff.py LEDGER_A LEDGER_B  (A=before R01/P3b, B=after)")
        sys.exit(1)
    a = parse(sys.argv[1])
    b = parse(sys.argv[2])
    if len(a) != len(b):
        print(f"length mismatch: A={len(a)} B={len(b)} — same corpus order required", file=sys.stderr)
        sys.exit(2)

    rtl = []  # rendered->lost
    ltr = []  # lost->rendered
    rtr = {"same": 0, "diff": 0}
    ltl = 0
    for fa, fb in zip(a, b):
        ra = fa.get("rendered") == "1"
        rb = fb.get("rendered") == "1"
        ka = fa.get("surface_kind", "-")
        kb = fb.get("surface_kind", "-")
        ra_ = fa.get("reason", "-")
        rb_ = fb.get("reason", "-")
        if ra and not rb:
            rtl.append((ka, kb, rb_, key(fb)))
        elif not ra and rb:
            ltr.append((ka, ka, ra_, key(fa)))
        elif ra and rb:
            ta = int(fa.get("triangles", 0))
            tb = int(fb.get("triangles", 0))
            if ta == tb:
                rtr["same"] += 1
            else:
                rtr["diff"] += 1
        else:
            ltl += 1

    print(f"total rows = {len(a)}")
    print(f"rendered->rendered same-tri {rtr['same']} diff-tri {rtr['diff']}")
    print(f"rendered->lost = {len(rtl)}")
    print(f"lost->rendered = {len(ltr)}")
    print(f"lost->lost = {ltl}")
    print()

    print("== rendered->lost by (kind, reason) ==")
    c = Counter((k, r) for _, k, r, _ in rtl)
    for (kind, reason), n in sorted(c.items(), key=lambda x: -x[1]):
        print(f"{n:>7}  {kind:12} {reason}")

    print()
    print("== lost->rendered by (kind, reason) ==")
    c = Counter((k, r) for _, k, r, _ in ltr)
    for (kind, reason), n in sorted(c.items(), key=lambda x: -x[1]):
        print(f"{n:>7}  {kind:12} {reason}")

    print()
    print("== rendered->lost sample face keys ==")
    seen = set()
    shown = 0
    for _, _, reason, k in rtl:
        if k[0] == "id" and k[1] not in seen:
            seen.add(k[1])
            print(f"  {reason:30} {k[1]}")
            shown += 1
            if shown >= 30:
                break


if __name__ == "__main__":
    main()
