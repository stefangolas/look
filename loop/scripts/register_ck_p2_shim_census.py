import json
from pathlib import Path

P = Path(__file__).resolve().parent.parent / 'PACKETS.jsonl'
rows = [json.loads(l) for l in P.read_text(encoding='utf-8-sig').splitlines() if l.strip()]

shim = {
    "id": "BG-CK-P2-CONTRACT",
    "wave": "CK-P2",
    "class": "design",
    "needs": ["BG-CK-P1-DISPATCH", "BG-CK-P1-FLOOR"],
    "status": "RUNNING",
    "writes": [
        "vendor/truck/truck-certified/src/ssi_types.rs",
        "vendor/truck/truck-certified/src/ssi_fixtures.rs",
        "vendor/truck/truck-certified/src/lib.rs",
        "vendor/truck/truck-certified/tests/ssi_contract.rs",
        "Cargo.toml",
        "Cargo.lock",
    ],
    "packet": "loop/packets/BG-CK-P2-CONTRACT.md",
    "slot": 0,
    "note": "Pre-wave contract shim for the Phase-2 wave (ORCHESTRATOR wave mode: shim through the NORMAL loop; wave base = its landing merge, NOT 37b0267). SESSION-49 CORRECTION OF RECORD: 37b0267 committed only the packet doc - the shim code never landed, the row was never registered, and 'LANDED at 37b0267' in the session-48 close was wrong on all three counts (machine-checked: git show --stat, git log --all -S SquareSystem3, PACKETS.jsonl tail). Authored packet dispatched as-is. Registered at dispatch time.",
}
census = {
    "id": "BG-CK-SPLINE-CENSUS",
    "wave": "CK-P2",
    "class": "mechanical",
    "needs": ["BG-CK-P0-PREVALENCE"],
    "status": "RUNNING",
    "writes": ["tests/certified_spline_census.rs", "docs/CERTIFIED_SPLINE_CENSUS.md"],
    "packet": "loop/packets/BG-CK-SPLINE-CENSUS.md",
    "slot": 1,
    "note": "Booking gate 4 measurement (spline-bucket structural census; interleave decided in docs/CERTIFIED_INTERLEAVE_BUILD_SPEC.md). RESULT gates the Phase-2 wave open alongside the shim. Runs in parallel with the shim on disjoint write sets (pagefile now size-restrained 6-12 GB; sccache installed first per wave-mode law). packet_lint TEST_PATH_OWNERSHIP WARN accepted as the documented read-only-context case: tests/certified_prevalence.rs stays byte-identical (V5-guarded). Registered at dispatch time.",
}

existing = {r['id'] for r in rows}
for row in (shim, census):
    if row['id'] in existing:
        raise SystemExit(f"row already exists: {row['id']}")
    rows.append(row)

with P.open('w', encoding='utf-8', newline='\n') as f:
    for r in rows:
        f.write(json.dumps(r, ensure_ascii=False) + '\n')
print('registered BG-CK-P2-CONTRACT (slot 0) and BG-CK-SPLINE-CENSUS (slot 1), both RUNNING')
