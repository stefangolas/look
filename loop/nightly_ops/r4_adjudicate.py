"""R4 adjudication - TTC-RECENSUS-F1-R4 verdicts from the collected door records.

Owner directive 2026-09-14 (performance ceiling): a row that constructs but
takes more than 120 seconds is not green - it is a CEILING verdict, booking a
faster solver. The census timeout itself drops to 120s in future rounds.

Classes:
  GREEN           constructed + facts sane (solids>0, finite volume, bbox) +
                  mesh emitted (triangles>0) + wall <= 120s
  CEILING         constructed + facts sane but wall > 120s (or killed past it)
  TYPED-REFUSAL   kernel refusal with a named case
  DNF             anything else (driver errors, no facts)
"""

from __future__ import annotations

import json
import os
import re

OUT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "r4")
CEILING_S = 120.0


def facts_sane(facts: dict) -> bool:
    if not isinstance(facts, dict):
        return False
    sc = facts.get("solid_count")
    vol = facts.get("volume")
    bbox = facts.get("bbox")
    if not isinstance(sc, int) or sc <= 0:
        return False
    if not isinstance(vol, (int, float)) or not (vol > 0):
        return False
    if not (isinstance(bbox, list) and len(bbox) == 2):
        return False
    return True


def classify(rec: dict) -> str:
    if rec.get("ok") and facts_sane(rec.get("facts", {})):
        tris = (rec.get("stl") or {}).get("triangles", 0)
        if not isinstance(tris, int) or tris <= 0:
            return "DNF"
        return "CEILING" if rec.get("r4_wall_seconds", 0) > CEILING_S else "GREEN"
    err = rec.get("error") or {}
    kind = err.get("kind", "")
    if kind == "Refused":
        return "TYPED-REFUSAL"
    if kind == "DriverTimeout":
        return "CEILING"
    return "DNF"


def refusal_case(rec: dict) -> str:
    msg = str((rec.get("error") or {}).get("message", ""))
    m = re.search(r"kernel refusal:\s*(\S+)", msg)
    return m.group(1) if m else "other"


def main() -> int:
    rows = []
    for name in sorted(os.listdir(OUT)):
        if not name.endswith(".json"):
            continue
        with open(os.path.join(OUT, name), encoding="utf-8") as fh:
            rec = json.load(fh)
        rid = rec.get("r4_row_id", name[:-5])
        verdict = classify(rec)
        rows.append(
            {
                "id": rid,
                "verdict": verdict,
                "wall_s": rec.get("r4_wall_seconds"),
                "solids": (rec.get("facts") or {}).get("solid_count") if isinstance(rec.get("facts"), dict) else None,
                "triangles": (rec.get("stl") or {}).get("triangles"),
                "case": refusal_case(rec) if verdict == "TYPED-REFUSAL" else "",
            }
        )

    tally: dict[str, int] = {}
    for r in rows:
        tally[r["verdict"]] = tally.get(r["verdict"], 0) + 1

    print(f"R4 VERDICTS ({len(rows)} rows)\n" + "=" * 60)
    for cls in ("GREEN", "CEILING", "TYPED-REFUSAL", "DNF"):
        print(f"  {cls:14} {tally.get(cls, 0)}")
    print()
    cases: dict[str, int] = {}
    for r in rows:
        if r["verdict"] == "TYPED-REFUSAL":
            cases[r["case"]] = cases.get(r["case"], 0) + 1
    print("REFUSAL TAXONOMY (named kernel refusals):")
    for case, n in sorted(cases.items(), key=lambda kv: -kv[1]):
        print(f"  {n:3}  {case}")
    print("\nCEILING VIOLATIONS (constructed, needs a faster solver):")
    for r in rows:
        if r["verdict"] == "CEILING":
            print(f"  {r['id']}: {r['wall_s']}s")
    print("\nGREEN ROWS:")
    for r in rows:
        if r["verdict"] == "GREEN":
            print(f"  {r['id']}: {r['wall_s']}s, {r['solids']} solids, {r['triangles']} tris")
    print("\nDNF ROWS:")
    for r in rows:
        if r["verdict"] == "DNF":
            print(f"  {r['id']}: wall={r['wall_s']}s")

    with open(os.path.join(os.path.dirname(OUT), "r4_verdicts.json"), "w", encoding="utf-8") as fh:
        json.dump(rows, fh, indent=1)
    print("\nverdicts written: loop/nightly_ops/r4_verdicts.json")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
