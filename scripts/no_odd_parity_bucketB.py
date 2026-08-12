#!/usr/bin/env python3
"""Item 6: structural census of bucket B (material_empty) from the census JSONL.

Histograms: surface family, bound count, edge count (segment count), distinct
UV vertex count, single-bound?, 2-edge?, same start/end, out-and-back/retraced,
near-zero signed UV area, finite 3D boundary extent proxy, periodic/seam contact.
"""
import json, sys
from collections import Counter, defaultdict

census = [json.loads(l) for l in open(sys.argv[1], encoding="utf-8")]
B = [r for r in census if r["bucket"] == "B"]
print(f"bucket B faces: {len(B)}")

def show(title, fn, top=12):
    c = Counter(fn(r) for r in B)
    print(f"\n## {title}")
    for k, n in c.most_common(top):
        print(f"  {k}: {n}")

show("surface family", lambda r: r["surface_family"])
show("bound_count", lambda r: r["bound_count"])
show("source_segment_count (edges)", lambda r: r["source_segment_count"])
show("boundary_piece_count", lambda r: r["boundary_piece_count"])
show("closed_pieces", lambda r: r["closed_pieces"])
show("open_pieces", lambda r: r["open_pieces"])
show("uv_point_count", lambda r: r["uv_point_count"])

# single-bound?
show("single_bound", lambda r: "yes" if r["bound_count"] == 1 else "multi")
# same start/end vertex (closed piece where start==end in UV)?
def same_start_end(r):
    pieces = []
    if "piece_start_end" not in r:
        return "no-info"
    for bp in r["piece_start_end"]:
        s, e = bp.get("start_uv"), bp.get("end_uv")
        if s and e and len(s) == 2 and len(e) == 2 and abs(s[0]-e[0]) < 1e-12 and abs(s[1]-e[1]) < 1e-12:
            pieces.append("closed_same")
    return "+".join(pieces) if pieces else "no"
show("same_start_end_piece", same_start_end)

# near-zero signed UV area
def area_class(r):
    a = abs(r["piece_abs_area_sum"])
    if a < 1e-12:
        return "abs_area<1e-12"
    if a < 1e-6:
        return "abs_area 1e-12..1e-6"
    if a < 1e-3:
        return "abs_area 1e-6..1e-3"
    return f"abs_area>={1e-3:.0e}"
show("uv abs area class", area_class)

# out-and-back / retraced
show("duplicate_traversal_count>0", lambda r: "retraced" if r["duplicate_traversal_count"] > 0 else "no-dup")
show("retraced_edge_count class", lambda r: ("retraced_edges" if r["retraced_edge_count"] > 0 else "none"))

# periodic / seam contact
show("periodic_u", lambda r: "u-periodic" if r["periodic_u"] else "u-open")
show("periodic_v", lambda r: "v-periodic" if r["periodic_v"] else "v-open")
show("chart_rank", lambda r: r["chart_rank"])

# material stage derived bucket
show("derived_bucket", lambda r: r["derived_bucket"])
show("arr_material_stage", lambda r: r["arr_material_stage"])

# cap activation decline reasons
c = Counter(r["cap_declined_reason"] for r in B)
print("\n## cap_declined_reason")
for k, n in c.most_common(15):
    print(f"  {k}: {n}")

# dominant motif cross-tab: single-bound + u-periodic cylinder + retraced
motifs = Counter()
for r in B:
    if r["bound_count"] == 1 and r["surface_family"] == "Cylinder" and r["periodic_u"] and r["duplicate_traversal_count"] > 0:
        motifs["single-bound u-periodic cylinder + duplicate traversal"] += 1
    elif r["bound_count"] == 1 and r["surface_family"] == "Cylinder" and r["periodic_u"]:
        motifs["single-bound u-periodic cylinder"] += 1
    elif r["surface_family"] == "Plane" and r["abs_area_sum"] if False else (r["surface_family"] == "Plane"):
        motifs["plane"] += 1
    else:
        motifs[f"other:{r['surface_family']}"] += 1
print("\n## coarse motif")
for k, n in motifs.most_common():
    print(f"  {k}: {n}")

# The #35281 case: single-bound cylinder u-periodic, dup traversals, uv_point_count small
print("\n## #35281-style population")
q = [r for r in B if r["surface_family"] == "Cylinder" and r["periodic_u"] and r["bound_count"] == 1]
print(f"  single-bound u-periodic cylinders: {len(q)}")
ret = sum(1 for r in q if r["duplicate_traversal_count"] > 0)
print(f"    with duplicate traversals: {ret}")
small = sum(1 for r in q if r["uv_point_count"] <= 8)
print(f"    with <=8 UV points: {small}")

# By model
show("model", lambda r: r["model"], top=20)

print("\n## example records per motif (first 3 each)")
groups = defaultdict(list)
for r in B:
    key = (r["surface_family"], r["bound_count"], r["periodic_u"], r["periodic_v"])
    if len(groups[key]) < 3:
        groups[key].append((r["model"], r["source_face_id"], r["uv_point_count"], r["duplicate_traversal_count"], r["piece_abs_area_sum"]))
for k, v in sorted(groups.items()):
    print(f"  {k}: {v}")
