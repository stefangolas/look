#!/usr/bin/env python3
"""Face-keyed diff of two ledgers, keyed by (model_index, source_face_id).

Both ledgers come from the same corpus file order. Faces without a
source_face_id are keyed by their position in the declared sequence.
"""

import sys
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


def key(i, f):
    sid = f.get("source_face_id", "-")
    if sid and sid != "-":
        return ("id", i, sid)
    return ("idx", i, f.get("declared_face_index", "-"))


def main():
    if len(sys.argv) != 3:
        print("usage: ledger_keyed_diff.py LEDGER_A LEDGER_B")
        sys.exit(1)
    a = parse(sys.argv[1])
    b = parse(sys.argv[2])
    ka = {key(i, f): (i, f) for i, f in enumerate(a)}
    kb = {key(i, f): (i, f) for i, f in enumerate(b)}
    if len(ka) != len(kb):
        print(f"row count differs: A={len(ka)} B={len(kb)}", file=sys.stderr)

    rtl = Counter()   # (kind, reason) rendered->lost
    ltr = Counter()   # (kind, reason) lost->rendered
    rtl_ex = []
    ltr_ex = []
    same = 0
    both_lost = 0
    for k in ka:
        ia, fa = ka[k]
        ib, fb = kb[k]
        ra = fa.get("rendered") == "1"
        rb = fb.get("rendered") == "1"
        if ra and not rb:
            rtl[(fb.get("surface_kind", "-"), fb.get("reason", "-"))] += 1
            rtl_ex.append(fb.get("source_face_id", "-"))
        elif not ra and rb:
            ltr[(fb.get("surface_kind", "-"), fb.get("reason", "-"))] += 1
            ltr_ex.append(fb.get("source_face_id", "-"))
        elif ra and rb:
            same += 1
        else:
            both_lost += 1

    print(f"A rows={len(a)} B rows={len(b)}  same rendered={same} both lost={both_lost}")
    print(f"rendered->lost total={sum(rtl.values())}")
    print(f"lost->rendered total={sum(ltr.values())}")
    print()
    print("== rendered->lost (new losses in B vs A) ==")
    for (kind, reason), n in sorted(rtl.items(), key=lambda x: -x[1]):
        print(f"{n:>6}  {kind:12} {reason}")
    print()
    print("== lost->rendered (recoveries in B vs A) ==")
    for (kind, reason), n in sorted(ltr.items(), key=lambda x: -x[1]):
        print(f"{n:>6}  {kind:12} {reason}")
    if rtl_ex:
        print()
        print("sample new-loss ids:", ", ".join(sorted(set(rtl_ex))[:20]))


if __name__ == "__main__":
    main()
