#!/usr/bin/env python3
import re, statistics
import collections

def parse(path):
    rows = []
    for line in open(path, encoding="utf-8", errors="replace"):
        if not line.startswith("GEO"):
            continue
        m = re.match(
            r"GEO\tsource_face_id=(\d+)\tkind=(\S+)\tshell_entity=(\d+)\tface_index=(\d+)\t"
            r"bound_count=(\d+)\tedge_use_count=(\d+)\t3d_boundary_diameter=([^\t]+)\t"
            r"3d_polyline_length=([^\t]+)\tdistinct_3d_vertex_count=(\d+)\t"
            r"repeated_edge_use_count=(\d+)\tuv_extent_u=([^\t]+)\tuv_extent_v=([^\t]+)",
            line,
        )
        if m:
            rows.append(
                {
                    "id": int(m.group(1)),
                    "kind": m.group(2),
                    "bdiam": float(m.group(7)),
                    "blen": float(m.group(8)),
                    "uvu": float(m.group(11)),
                    "uvv": float(m.group(12)),
                }
            )
    return rows


for label, path, model_diam, tol in [
    ("00003172", r"C:\Users\stefa\AppData\Local\Temp\opencode\geo_00003172.log", 1.7211, 0.001721),
    ("00000730", r"C:\Users\stefa\AppData\Local\Temp\opencode\geo_00000730.log", 0.7948, 0.000795),
]:
    rows = parse(path)
    print(f"=== {label}: {len(rows)} faces, model diameter {model_diam}, tolerance {tol:.6e}")
    if not rows:
        continue
    bd = [r["bdiam"] for r in rows]
    print(f"  3d boundary diameter: median {statistics.median(bd):.3e} max {max(bd):.3e} min {min(bd):.3e}")
    print(f"  faces with bdiam < tol: {sum(1 for d in bd if d < tol)} ({sum(1 for d in bd if d < tol)/len(bd)*100:.1f}%)")
    print(f"  faces with bdiam < 0.1*tol: {sum(1 for d in bd if d < 0.1*tol)}")
    print(f"  faces with bdiam > tol: {sum(1 for d in bd if d > tol)}")
    ratios = [d / tol for d in bd]
    print(f"  bdiam/tol: median {statistics.median(ratios):.3e} max {max(ratios):.3e}")
    bl = [r["blen"] for r in rows]
    print(f"  3d polyline length: median {statistics.median(bl):.3e} max {max(bl):.3e}")
    kinds = collections.Counter(r["kind"] for r in rows)
    print(f"  kinds: {dict(kinds)}")
    print()
