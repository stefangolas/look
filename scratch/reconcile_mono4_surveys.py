"""Registry/ledger reconciliation to ground truth for MONO-4 + SURVEY-B/C."""

import json

d = {}
for l in open("loop/PACKETS.jsonl", encoding="utf-8"):
    if l.strip():
        r = json.loads(l)
        d[r["id"]] = r

NOTE4 = (
    "recovered-from-recycle: worker work archived at slot recycle, "
    "orchestrator applied + scoped-verified (59/59) + committed AS DELIVERED; "
    "the driver earlier merged a base commit and flipped the row prematurely - reconciled"
)

truth = {
    "MONO-4-TRIM-IDIOMS": ("DONE", "852763c", "2a50c4f", NOTE4),
    "SOLVER-SURVEY-B": ("DONE", "c3bc1a1", "1ec1876",
                        "fragment as delivered committed after the driver base-commit no-op landing; "
                        "13.5k-line rule fragment; RESULT filed"),
    "SOLVER-SURVEY-C": ("DONE", "e6553db", "cce4ec7",
                        "fragment as delivered; root-RESULT add/add resolved; RESULT filed"),
}
for k, (st, wc, mg, note) in truth.items():
    d[k]["status"] = st
    d[k]["note"] = (d[k].get("note") or "") + " | LANDED " + wc + " (" + note + ")"
with open("loop/PACKETS.jsonl", "w", encoding="utf-8") as f:
    for r in d.values():
        f.write(json.dumps(r, ensure_ascii=False) + "\n")

led = [
    {"id": "MONO-4-TRIM-IDIOMS", "slot": "0", "verdict": "LANDED",
     "worker_commit": "852763c (AS DELIVERED, recovered)", "merged_as": "2a50c4f",
     "model": "deepseek/deepseek-v4-flash",
     "note": "recycle race ate the uncommitted work; archive patch recovered + applied; "
             "scoped check 59/59; louvre + suspension-plate idioms compose certified",
     "closed": "2026-09-10"},
    {"id": "SOLVER-SURVEY-B", "slot": "2", "verdict": "LANDED",
     "worker_commit": "c3bc1a1 (AS DELIVERED)", "merged_as": "1ec1876",
     "model": "deepseek/deepseek-v4-flash",
     "note": "admission/bie/formal rule-table fragment; skipped-commit-step repaired; "
             "theory-only rows marked per spec",
     "closed": "2026-09-10"},
    {"id": "SOLVER-SURVEY-C", "slot": "3", "verdict": "LANDED",
     "worker_commit": "e6553db (AS DELIVERED)", "merged_as": "cce4ec7",
     "model": "deepseek/deepseek-v4-flash",
     "note": "boolean-stage rule-table fragment with vertical-gap marking; "
             "skipped-commit-step repaired",
     "closed": "2026-09-10"},
]
with open("loop/LEDGER.jsonl", "a", encoding="utf-8") as f:
    for r in led:
        f.write(json.dumps(r, ensure_ascii=False) + "\n")
print("bookkeeping repaired")
