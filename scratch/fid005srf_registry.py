import json
import io

p = "loop/PACKETS.jsonl"
rows = [json.loads(l) for l in io.open(p, encoding="utf-8") if l.strip()]
for r in rows:
    if r["id"] == "BG-FID-005-SRF":
        r["packet"] = "loop/packets/BG-FID-005-SRF.md"
        r["note"] = (
            "r1 packet written session 22: tensor-product bicubic Hermite emitter "
            "(16-point net, twist signs, sliver routing), surface scale components "
            "(lfs reuse; Chebyshev wrapped-gap separation; relative convergence + "
            "level cap 7), per-axis refine loop, bivariate grid-vertex Krawczyk "
            "(first-box requirement), Chebyshev-1 wrap adjacency, double-sheet "
            "witness a=eps/2; witnesses machine-checked orchestrator-side"
        )
with io.open(p, "w", encoding="utf-8", newline="\n") as f:
    for r in rows:
        f.write(json.dumps(r, ensure_ascii=False) + "\n")

# verify the watchdog's read path: a known-DONE id still reads DONE
rows2 = {json.loads(l)["id"]: json.loads(l)["status"]
         for l in io.open(p, encoding="utf-8") if l.strip()}
print("BG-ENC-004-PCURVE:", rows2["BG-ENC-004-PCURVE"])
print("BG-FID-005-SRF:", rows2["BG-FID-005-SRF"])
print("BG-FID-005:", rows2["BG-FID-005"])
print("total rows:", len(rows2))
