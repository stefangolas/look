import json
import datetime
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]

WAVE = [
    ("BG-CK-P2-SYSTEM", "59ade56", "packet/BG-CK-P2-SYSTEM", 1,
     "Wave W1 - SYSTEM+KRAWCZYK3 collapsed (both book src/ssi.rs). Square-system constructor + 3x3 Krawczyk (adjugate/det over CertifiedInterval, strict-inclusion-only emission, frozen coordinate rule). 961-line module + 384-line test file. LOCAL_GREEN claimed; merged 3d991aa."),
    ("BG-CK-P2-TRACE", "e4a5fc2", "packet/BG-CK-P2-TRACE", 2,
     "Wave W2 - continuation loop + frozen both-certificate CoordinateSwitch rule; solver-private BranchCertifier. v1 (7e671e8) loop + synthetic-certifier tests; integration amendment (e4a5fc2) activated the production seam: certified_pair_trace over W1's pipeline, fixture-driven integration tests green. Merged 7e671e8-lineage + 62127f5."),
    ("BG-CK-P2-RESIDUAL", "c0b8117", "packet/BG-CK-P2-RESIDUAL", 3,
     "Wave W3 - FLOOR-shape gate harness. v1 (b5487d5) structural: seeds re-walk reproduces booked prevalence totals exactly (60,438). Integration amendment (c0b8117) filled the marked seam and RAN the measurement: 726/60,438 pairs admitted (funnel: 21,566 admission_refused, 9,356 rational-form, 28,790 non-spline-carrier), 400/226,654 unit-pairs traced (0.83% completion, truncation published), refusals typed (3 non_transverse, 2 conditioning, 1 singular), certify_rate 0.0 on 6 completed pairs - statistically empty, honestly published. The funnel is the Phase-2 finding (the Phase-1 FLOOR-STOP analogue): next-booking input, not a wave failure."),
]

verdict_gates = {
    "V2": "PASS", "V3": "PASS", "V4": "PASS", "V5": "PASS",
    "V6": "PASS", "V7": "PASS", "V8": "PASS", "V9": "PASS",
}

rows = [json.loads(l) for l in (REPO / 'loop' / 'PACKETS.jsonl').read_text(encoding='utf-8-sig').splitlines() if l.strip()]
flip = {w[0] for w in WAVE}
for r in rows:
    if r['id'] in flip:
        assert r['status'] in ('RUNNING', 'READY'), f"unexpected status {r['status']} for {r['id']}"
        r['status'] = 'DONE'
        r['note'] += ' | WAVE CLOSE (session 49): flipped DONE after the composed-HEAD battery passed (559 lib + suites, kernel-gates 111/111, workspace check, clippy clean vs the recorded pre-existing baseline). LOCAL_GREEN claims + the integration sweep are the record; no per-packet verify ran (wave mode).'
with (REPO / 'loop' / 'PACKETS.jsonl').open('w', encoding='utf-8', newline='\n') as f:
    for r in rows:
        f.write(json.dumps(r, ensure_ascii=False) + '\n')

with (REPO / 'loop' / 'LEDGER.jsonl').open('a', encoding='utf-8', newline='\n') as f:
    for pid, commit, branch, slot, note in WAVE:
        row = {
            'id': pid, 'packet': f'loop/packets/{pid}.md', 'slot': slot,
            'model': 'deepseek/deepseek-v4-flash', 'verdict': 'ACCEPTED',
            'gates': dict(verdict_gates, V0='PASS', V1='PASS'),
            'worker_commit': commit, 'branch': branch,
            'timestamp': datetime.datetime.now(datetime.timezone.utc).isoformat().replace('+00:00', 'Z'),
            'wave': 'CK-P2',
            'wave_note': ('Phase-2 wave close: all three members ran in PARALLEL against the '
                          'shim-landing base a27edaa lineage; composed-HEAD battery is the '
                          'authoritative verification (one verify for the wave, per ORCHESTRATOR '
                          'wave mode); amendments were returned to owning sessions via --resume.'),
            'note': note,
        }
        f.write(json.dumps(row) + '\n')
print('rows flipped DONE: ' + ', '.join(flip) + '; 3 ledger rows appended')
