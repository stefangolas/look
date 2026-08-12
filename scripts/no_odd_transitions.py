#!/usr/bin/env python3
"""NO_ODD_PARITY transition census: compare the historical NoOddParityRegion
population (diag_r01fix.jsonl) to the final sweep (per-model diag JSONLs).

For every historically-lost face, report its final outcome:
  recovered       -> now renders
  rejected        -> certified RejectedDegenerate (carries a validity certificate)
  still_no_odd    -> still NoOddParityRegion
  other_failure   -> some other terminal reason
Plus aggregate rendered/lost/rejected per model.
"""
import argparse
import json
import os


def model_of(r):
    m = r.get("model_id", "")
    parts = m.replace("\\", "/").split("/")
    return parts[-2] if len(parts) >= 2 else m


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("hist")       # diag_r01fix.jsonl
    ap.add_argument("sw_dir")     # dir of per-model sweep .jsonl
    ap.add_argument("--out-json", default="no_odd_transitions.jsonl")
    args = ap.parse_args()

    hist = [json.loads(l) for l in open(args.hist, encoding="utf-8")]
    hist_lost = {}
    for r in hist:
        hist_lost[(model_of(r), r.get("source_face_id"))] = r.get("terminal_reason")

    sweep = {}
    for fn in sorted(os.listdir(args.sw_dir)):
        if not fn.endswith(".jsonl") or fn.endswith(".meta.json"):
            continue
        for l in open(os.path.join(args.sw_dir, fn), encoding="utf-8"):
            r = json.loads(l)
            key = (model_of(r), r.get("source_face_id"))
            sweep[key] = r

    rows = []
    from collections import Counter
    trans = Counter()
    for key, reason in hist_lost.items():
        m, fid = key
        final = sweep.get(key)
        if final is None:
            # Not in the final sweep's lost set -> it now renders.
            outcome = "recovered"
            final_reason = "rendered"
            cert = None
        else:
            cert = final.get("validity_certificate")
            final_reason = final.get("terminal_reason")
            if cert and final_reason == "RejectedDegenerate":
                outcome = "rejected"
            elif final_reason == "NoOddParityRegion":
                outcome = "still_no_odd"
            else:
                outcome = "other_failure"
        trans[outcome] += 1
        rows.append({
            "model": m,
            "source_face_id": fid,
            "hist_reason": reason,
            "final_outcome": outcome,
            "final_reason": final_reason,
            "cert_reason": (cert or {}).get("reason") if cert else None,
            "cert_world_rank": (cert or {}).get("world_rank") if cert else None,
        })

    with open(args.out_json, "w", encoding="utf-8") as f:
        for r in sorted(rows, key=lambda x: (x["model"], x["source_face_id"])):
            f.write(json.dumps(r) + "\n")

    total = len(rows)
    print(f"historical lost faces: {total}")
    for outcome in ("recovered", "rejected", "still_no_odd", "other_failure"):
        n = trans.get(outcome, 0)
        print(f"  {outcome:14} {n:5}  {n/total*100:5.1f}%")
    print("rejected by reason:", dict(Counter(r["cert_reason"] for r in rows if r["cert_reason"])))
    print("still_no_odd by reason:", dict(Counter(r["final_reason"] for r in rows if r["final_outcome"] == "still_no_odd")))
    print("other_failure by reason:", dict(Counter(r["final_reason"] for r in rows if r["final_outcome"] == "other_failure")))
    by_model = Counter(r["model"] for r in rows if r["final_outcome"] in ("recovered", "rejected"))
    print("classified per model:", dict(sorted(by_model.items())))


if __name__ == "__main__":
    main()
