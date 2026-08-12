#!/usr/bin/env python3
"""Build the machine-readable NoOddParityRegion face census from a face_census
diag JSONL artifact.

Every NoOddParityRegion row is flattened into one CSV/JSON record carrying:

  model, source_face_id, surface_family, bucket (A empty_cdt / B material_empty
  / C validation_empty), bound_count, segment counts, per-piece UV signed area,
  reconstructible UV point set, open/closed carrier counts, periodic axes,
  CDT stage counts (raw / selected / final), overlap-conflict summary
  (DuplicateTraversal count, out-and-back evidence), and validity-cache flags.

This is a pure read of the diagnostic artifact. No geometry is re-derived from
the STEP files here; geometric observables that need surface evaluation live in
the Rust probe (items 3-7 of the audit).
"""
import argparse
import json
import sys
import math


def bucket_of(r):
    c = r.get("cdt_stages", {}) or {}
    raw = c.get("raw_cdt_triangles")
    sel = c.get("material_selected")
    fin = c.get("final_valid")
    if raw in (None, 0):
        return "A"
    if sel in (None, 0):
        return "B"
    if fin in (None, 0):
        return "C"
    return "?"


def model_of(r):
    m = r.get("model_id", "")
    parts = m.replace("\\", "/").split("/")
    return parts[-2] if len(parts) >= 2 else m


def uv_points(r):
    pts = set()
    for bp in r.get("boundary_pieces", []):
        for key in ("start_uv", "end_uv"):
            v = bp.get(key)
            if v and len(v) == 2:
                pts.add((v[0], v[1]))
        rep = bp.get("representative")
        if rep and len(rep) == 2:
            pts.add((rep[0], rep[1]))
    for oc in r.get("overlap_conflicts", []):
        for segkey in ("incoming_segment", "blocking_segment"):
            s = oc.get(segkey)
            if not s:
                continue
            a, b = s.get("a"), s.get("b")
            if a and len(a) == 2:
                pts.add((a[0], a[1]))
            if b and len(b) == 2:
                pts.add((b[0], b[1]))
    return sorted(p for p in pts if all(v is not None for v in p))


def edges_of(r):
    """Reconstruct edges (pairs of consecutive UV points) where possible."""
    edges = []
    for oc in r.get("overlap_conflicts", []):
        for segkey in ("incoming_segment", "blocking_segment"):
            s = oc.get(segkey)
            if s and s.get("a") and s.get("b") and len(s["a"]) == 2 and len(s["b"]) == 2:
                edges.append((tuple(s["a"]), tuple(s["b"])))
    return edges


def analyze(r):
    pieces = r.get("boundary_pieces", [])
    overlaps = r.get("overlap_conflicts", [])
    dup = sum(1 for o in overlaps if o.get("relation") == "DuplicateTraversal")
    vert_ins = sum(1 for o in overlaps if o.get("relation") == "VertexInsertionFailure")
    nclosed = sum(1 for p in pieces if p.get("closure") == "EuclideanClosed")
    nopen = sum(1 for p in pieces if p.get("closure") == "Open")
    signed = [p.get("signed_area") or 0.0 for p in pieces]
    abs_area = sum(abs(a) for a in signed)
    signed_total = sum(signed)
    pts = uv_points(r)
    edges = edges_of(r)
    # out-and-back evidence: an edge whose endpoints are also endpoints of the
    # opposite-direction edge, or repeated same-point spans.
    edge_keys = {}
    for a, b in edges:
        k = (a, b)
        edge_keys[k] = edge_keys.get(k, 0) + 1
        rev = (b, a)
        edge_keys[rev] = edge_keys.get(rev, 0) + 1
    retraced = sum(1 for (k, n) in edge_keys.items() if n > 1)
    self_spans = sum(1 for a, b in edges if a == b)
    c = r.get("cdt_stages", {}) or {}
    return {
        "model": model_of(r),
        "source_face_id": r.get("source_face_id"),
        "surface_family": r.get("surface_family"),
        "bucket": bucket_of(r),
        "bound_count": r.get("bound_count"),
        "source_segment_count": r.get("source_segment_count"),
        "synthetic_segment_count": r.get("synthetic_segment_count"),
        "seam_segment_count": r.get("seam_segment_count"),
        "boundary_piece_count": len(pieces),
        "closed_pieces": nclosed,
        "open_pieces": nopen,
        "piece_start_end": [
            {
                "start_uv": p.get("start_uv"),
                "end_uv": p.get("end_uv"),
                "signed_area": p.get("signed_area"),
                "closure": p.get("closure"),
                "winding_sign": p.get("winding_sign"),
                "point_count": p.get("point_count"),
            }
            for p in pieces
        ],
        "piece_signed_area_sum": signed_total,
        "piece_abs_area_sum": abs_area,
        "winding_sign_any_zero": any(p.get("winding_sign") == 0 for p in pieces),
        "uv_point_count": len(pts),
        "overlap_conflict_count": len(overlaps),
        "duplicate_traversal_count": dup,
        "vertex_insertion_failure_count": vert_ins,
        "retraced_edge_count": retraced,
        "self_span_count": self_spans,
        "chart_rank": r.get("chart_rank"),
        "periodic_u": (r.get("periodic_axes") or {}).get("u", False),
        "periodic_v": (r.get("periodic_axes") or {}).get("v", False),
        "raw_cdt_triangles": c.get("raw_cdt_triangles"),
        "material_selected": c.get("material_selected"),
        "final_valid": c.get("final_valid"),
        "boundary_vertices": c.get("boundary_vertices"),
        "constraints_presented": c.get("constraints_presented"),
        "constraints_inserted": c.get("constraints_inserted"),
        "lift_status": r.get("lift_status"),
        "projection_status": r.get("projection_status"),
        "deck_status": r.get("deck_status"),
        "derived_bucket": r.get("derived_bucket"),
        "arr_material_stage": (r.get("arr") or {}).get("material_stage"),
        "cap_declined_reason": (r.get("cap_activation") or {}).get("declined_reason"),
    }


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("jsonl")
    ap.add_argument("--out-csv", default="no_odd_parity_census.csv")
    ap.add_argument("--out-json", default="no_odd_parity_census.jsonl")
    args = ap.parse_args()

    rows = [json.loads(l) for l in open(args.jsonl, encoding="utf-8")]
    nop = [r for r in rows if r.get("terminal_reason") == "NoOddParityRegion"]
    print(f"rows={len(rows)} NoOddParityRegion={len(nop)}", file=sys.stderr)

    records = [analyze(r) for r in nop]
    fields = sorted(records[0].keys()) if records else []

    with open(args.out_csv, "w", newline="", encoding="utf-8") as f:
        import csv
        w = csv.DictWriter(f, fieldnames=fields)
        w.writeheader()
        for rec in records:
            w.writerow(rec)

    with open(args.out_json, "w", encoding="utf-8") as f:
        for rec in records:
            f.write(json.dumps(rec) + "\n")

    from collections import Counter
    print("bucket:", dict(Counter(r["bucket"] for r in records)), file=sys.stderr)
    print("family:", dict(Counter(r["surface_family"] for r in records)), file=sys.stderr)
    print("models:", len(set(r["model"] for r in records)), file=sys.stderr)
    print(f"wrote {args.out_csv} ({len(records)} records)", file=sys.stderr)


if __name__ == "__main__":
    main()
