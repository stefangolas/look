#!/usr/bin/env python3
import re, statistics, collections, glob, os

def parse(path):
    rows = []
    for line in open(path, encoding="utf-8", errors="replace"):
        if not line.startswith("GEO"):
            continue
        m = re.match(
            r"GEO\tsource_face_id=(\d+)\tkind=(\S+)\tshell_entity=(\d+)\tface_index=(\d+)\t"
            r"bound_count=(\d+)\tedge_use_count=(\d+)\t3d_boundary_diameter=([^\t]+)\t"
            r"3d_polyline_length=([^\t]+)\tdistinct_3d_vertex_count=(\d+)\t"
            r"repeated_edge_use_count=(\d+)\tuv_extent_u=([^\t]+)\tuv_extent_v=([^\t]+)\t"
            r"world_rank=(\d+)\trank_span=([^\t]+)\trank_max_perp=([^\t]+)\trank_tol=([^\t]+)",
            line,
        )
        if m:
            rows.append(
                {
                    "id": int(m.group(1)),
                    "kind": m.group(2),
                    "bdiam": float(m.group(7)),
                    "blen": float(m.group(8)),
                    "distinct": int(m.group(9)),
                    "repeated": int(m.group(10)),
                    "uvu": float(m.group(11)),
                    "uvv": float(m.group(12)),
                    "rank": int(m.group(13)),
                    "span": float(m.group(14)),
                    "max_perp": float(m.group(15)),
                    "tol": float(m.group(16)),
                }
            )
    return rows

all_rows = {}
for path in glob.glob(r"C:\Users\stefa\AppData\Local\Temp\opencode\geoA2_*.log"):
    model = os.path.basename(path).replace("geoA2_", "").replace(".log", "")
    rows = parse(path)
    for r in rows:
        r["model"] = model
    all_rows[model] = rows

total = sum(len(v) for v in all_rows.values())
print(f"bucket A measured: {total} faces across {len(all_rows)} models")

# rank histogram
ranks = collections.Counter(r["rank"] for m in all_rows.values() for r in m)
print("world_rank histogram:", dict(sorted(ranks.items())))
print()

by_model = {}
for model, rows in sorted(all_rows.items()):
    if not rows:
        continue
    rk = collections.Counter(r["rank"] for r in rows)
    kinds = collections.Counter(r["kind"] for r in rows)
    print(f"{model}: {len(rows)} faces rank={dict(sorted(rk.items()))} kinds={dict(kinds)}")

print()
r2 = [r for m in all_rows.values() for r in m if r["rank"] == 2]
r1 = [r for m in all_rows.values() for r in m if r["rank"] <= 1]
print(f"RANK 2 (real 2D region, CDT-empty => real failure candidate): {len(r2)}")
print(f"  by kind: {dict(collections.Counter(r['kind'] for r in r2))}")
if r2:
    bd = [r["bdiam"] for r in r2]
    print(f"  bdiam median {statistics.median(bd):.4e} max {max(bd):.4e}")
print()
print(f"RANK <=1 (collapsed point/line, genuinely degenerate): {len(r1)}")
print(f"  by kind: {dict(collections.Counter(r['kind'] for r in r1))}")
if r1:
    bd = [r["bdiam"] for r in r1]
    print(f"  bdiam median {statistics.median(bd):.4e} max {max(bd):.4e}")
    # max_perp vs span for rank1: is it truly line-like
    ratio = [r["max_perp"] / r["span"] for r in r1 if r["span"] > 0]
    if ratio:
        print(f"  rank1 max_perp/span: median {statistics.median(ratio):.3e} max {max(ratio):.3e}")
print()

# For rank<=1 with large bdiam: the rank tolerance is fp conditioning, so a
# large span with tiny perp ratio IS a line. Check the 00007705 case.
print("--- 00007705 rank detail ---")
r7705 = all_rows.get("00007705", [])
r1a = [r for r in r7705 if r["rank"] == 1]
if r1a:
    print(f"rank1 planes: {len(r1a)}")
    sp = [r["span"] for r in r1a]
    per = [r["max_perp"] for r in r1a]
    print(f"  span median {statistics.median(sp):.4e} max_perp median {statistics.median(per):.3e}")
    print("  sample:", [(r['id'], round(r['span'],4), r['max_perp'], round(r['tol'],2)) for r in r1a[:5]])
