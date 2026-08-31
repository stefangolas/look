import json
from pathlib import Path

P = Path(__file__).resolve().parent.parent / 'PACKETS.jsonl'
rows = [
    {
        "id": "BG-CK-P0-CRATE",
        "status": "RUNNING",
        "note": "Certified-kernel Phase 0 (plan D1): promote tessellation/{formal,domain} + source_evidence.rs into new workspace crate truck-certified; meshalgo consumes via compat re-exports (look's truck_meshalgo::tessellation::formal paths ride them); one new manifest edge meshalgo->certified, truck-geometry stays certified-free. All structural decisions pre-made in the packet (meshable.rs trait lift + Parallelizable shim, cgmath direct dep replacing the 4-hop glob accident, 12+14 measured import-rewrite sites). X2: dispatched ALONE among vendor-writing packets. Registered at dispatch time (the CG-002/003/004 rule).",
    },
    {
        "id": "BG-CK-P0-PREVALENCE",
        "status": "RUNNING",
        "note": "Certified-kernel Phase 0 exit-gate measurement: analytic-pair prevalence over the 38-file look-corpus (5 assemblies + NIST PMI) via the landed identify_plane/cylinder/cone/torus constructors + face_adjacency, one #[ignore] census test (tests/certified_prevalence.rs) + published docs/CERTIFIED_PREVALENCE.md. No kernel changes; write set disjoint from BG-CK-P0-CRATE, so both run in parallel. The number decides whether Phase 2 is urgent. Registered at dispatch time.",
    },
]
existing = [json.loads(l) for l in P.read_text(encoding='utf-8-sig').splitlines() if l.strip()]
ids = {r['id'] for r in existing}
for r in rows:
    if r['id'] in ids:
        raise SystemExit(f"{r['id']} already registered - refusing to double-append")
with P.open('a', encoding='utf-8', newline='\n') as f:
    for r in rows:
        f.write(json.dumps(r, ensure_ascii=False) + '\n')
print("appended:", ", ".join(r['id'] for r in rows))
