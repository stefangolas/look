"""Land an ACCEPTED packet: merge, file, ledger, ratchet, commit.

Everything here was hand-run for eleven packets, and the sequence is the same
every time -- merge --no-ff, move RESULT.json into loop/results/, append a
LEDGER row, flip PACKETS.jsonl to DONE, reset the GATE-4 ceiling to the new true
count, commit. Six-plus commands, three of them easy to forget, and the one that
rots silently is the ceiling: a ceiling left at its dispatch budget is a licence
rather than a ratchet, and the only thing that ever caught that was someone
remembering.

Usage:
    python loop/land_packet.py --slot 0 --packet loop/packets/BG-XXX.md
    python loop/land_packet.py --slot 0 --packet ... --dry-run

This script does NOT decide acceptance. It reads the verdict verify.py already
wrote and refuses to land anything that is not ACCEPTED -- and refuses just as
hard when the verdict is about a different commit than the branch now points at,
which is the failure an amend-then-land would otherwise walk straight into.
"""
import argparse
import datetime
import json
import re
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
INTEGRATION = 'integration/kernel-bg'


def git(*args, cwd=REPO_ROOT, check=False):
    res = subprocess.run(['git', '-C', str(cwd), *args], capture_output=True,
                         text=True, encoding='utf-8', errors='replace')
    if check and res.returncode != 0:
        sys.exit(f"git {' '.join(args)} failed: {res.stderr.strip()}")
    return res


def out(*args, cwd=REPO_ROOT):
    return git(*args, cwd=cwd).stdout.strip()


def gate4_count():
    """GATE-4's count at the integration branch tip, read the way the gate reads
    it. `git grep` exits 1 on no match, which is the healthy state, so the
    return code is deliberately ignored."""
    res = git('grep', '-oh', 'unscaled_legacy(', 'HEAD', '--',
              'vendor/truck/*/src/*',
              ':(exclude)vendor/truck/truck-base/src/tolerance.rs')
    return len([l for l in res.stdout.splitlines() if l.strip()])


def packet_ids(packet_path):
    """The packet's own id plus any `covers:` ids -- a combined packet closes
    more than one row in PACKETS.jsonl, which is how TOPOLOGY and MODELING were
    both discharged by BG-TOL-001-TOPO-MOD."""
    text = packet_path.read_text(encoding='utf-8')
    m_id = re.search(r"(?m)^id:\s*(\S+)", text)
    if not m_id:
        sys.exit(f"{packet_path} has no `id:` in its front block")
    ids = [m_id.group(1).strip()]
    m_cov = re.search(r"(?m)^covers:\s*\[(.*?)\]", text)
    if m_cov:
        ids += [p.strip().strip('"\'') for p in m_cov.group(1).split(',') if p.strip()]
    return ids[0], ids


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--slot', type=int, required=True)
    ap.add_argument('--packet', required=True)
    ap.add_argument('--dry-run', action='store_true')
    ap.add_argument('--fault', choices=['NONE', 'GATE', 'PACKET', 'WORKER', 'SPEC', 'HARNESS'],
                    help='dominant cause of the round trips this packet took. NONE = accepted '
                         'first time. GATE = a gate was wrong (it failed on the untouched '
                         'baseline, or could not see what it claimed to check). PACKET = the '
                         'orchestrator got a budget, anchor or house-rule instruction wrong. '
                         'WORKER = the worker\'s code was actually wrong. SPEC = the '
                         'specification had not decided something. HARNESS = dispatch/verify '
                         'infrastructure, not a gate\'s judgement.')
    ap.add_argument('--fault-note', help='one clause naming the specific defect')
    args = ap.parse_args()

    packet_path = Path(args.packet)
    if not packet_path.is_file():
        sys.exit(f"packet not found: {args.packet}")
    slot_root = REPO_ROOT / 'loop' / 'slots' / str(args.slot)
    wt = slot_root / 'wt'
    verdict_file = slot_root / 'VERDICT.json'

    pid, all_ids = packet_ids(packet_path)

    # --- refuse early, and for reasons that are about this exact commit -------
    if not verdict_file.is_file():
        sys.exit(f"slot {args.slot} has no VERDICT.json -- run verify.py first")
    verdict = json.loads(verdict_file.read_text(encoding='utf-8'))

    if verdict.get('verdict') != 'ACCEPTED':
        sys.exit(f"slot {args.slot} verdict is {verdict.get('verdict')!r}, not ACCEPTED. "
                 "land_packet does not decide acceptance and will not override it.")
    if verdict.get('packet') != args.packet:
        sys.exit(f"the verdict in slot {args.slot} is for {verdict.get('packet')!r}, "
                 f"not {args.packet!r}")

    branch = out('rev-parse', '--abbrev-ref', 'HEAD', cwd=wt)
    head = out('rev-parse', 'HEAD', cwd=wt)
    # The verdict names the commit it judged. If the branch has moved since --
    # an amend, a rebase, one more worker turn -- the verdict is about code that
    # is no longer what would be merged, and merging it would land unverified
    # work under an ACCEPTED banner.
    if verdict.get('commit') and verdict['commit'] != head:
        sys.exit(f"slot {args.slot} has moved since it was verified:\n"
                 f"  verdict judged {verdict['commit'][:12]}\n"
                 f"  branch is now  {head[:12]}\n"
                 "Re-run verify.py; a verdict is about one commit, not about a branch.")

    cur = out('rev-parse', '--abbrev-ref', 'HEAD')
    if cur != INTEGRATION:
        sys.exit(f"the repo root is on {cur!r}, not {INTEGRATION!r}. Refusing to land.")

    dirty = [l for l in out('status', '--porcelain').splitlines()
             if l and not l.startswith('??')]
    if dirty:
        sys.exit("the main worktree has uncommitted tracked changes; commit or stash first:\n  "
                 + "\n  ".join(dirty[:8]))

    result_src = wt / 'RESULT.json'
    question_src = wt / 'QUESTION.md'
    if not result_src.is_file():
        sys.exit(f"no RESULT.json in {wt}")

    print(f"landing {pid} from slot {args.slot} ({branch}@{head[:7]})")
    if len(all_ids) > 1:
        print(f"  closes {len(all_ids)} rows: {', '.join(all_ids)}")
    if args.dry_run:
        print("DRY RUN -- would merge --no-ff, file results, append the ledger, "
              "set PACKETS.jsonl to DONE, and reset the ceiling")
        print(f"  ceiling would become {gate4_count()} (measured after merge, so this is pre-merge)")
        return

    # --- merge ---------------------------------------------------------------
    res = git('merge', '--no-ff', branch, '-m',
              f"merge: {pid} (ACCEPTED, verified at {head[:7]})")
    if res.returncode != 0:
        sys.exit(f"merge failed, nothing else done:\n{res.stdout}\n{res.stderr}")

    # --- file the worker's own words, out of the repo root -------------------
    # RESULT.json and QUESTION.md ride into integration on the merge, and a
    # stray RESULT.json in the root is exactly what the NEXT packet's V1 sees as
    # a file its worker never wrote.
    results_dir = REPO_ROOT / 'loop' / 'results'
    results_dir.mkdir(exist_ok=True)
    (results_dir / f'{pid}.json').write_text(
        (REPO_ROOT / 'RESULT.json').read_text(encoding='utf-8'), encoding='utf-8')
    git('rm', '-q', 'RESULT.json')
    if (REPO_ROOT / 'QUESTION.md').is_file():
        git('mv', 'QUESTION.md', f'loop/results/{pid}.QUESTION.md')

    # --- ledger --------------------------------------------------------------
    gates = {}
    for g in verdict.get('gates', []):
        gates[g['name'].split()[0]] = g['status']
    # The model that actually ran this packet, recorded by run_packet.py at
    # dispatch. The fallback is the pre-switch default: slots dispatched before
    # worker.model existed all ran it (the historical rows say so).
    model_file = REPO_ROOT / 'loop' / 'slots' / str(args.slot) / 'worker.model'
    model = (model_file.read_text(encoding='utf-8').strip()
             if model_file.is_file() else 'deepseek/deepseek-v4-flash')
    row = {
        'id': pid, 'packet': args.packet, 'slot': args.slot,
        'model': model, 'verdict': 'ACCEPTED', 'gates': gates,
        'worker_commit': head[:7], 'branch': branch,
        'timestamp': datetime.datetime.now(datetime.timezone.utc)
                     .isoformat().replace('+00:00', 'Z'),
    }
    if len(all_ids) > 1:
        row['covers'] = all_ids[1:]

    # Fault attribution. Two sessions running have concluded in PROSE that
    # "every defect found was in a packet or a gate, never in the worker's
    # code" -- BG-TOL-001-MESHALGO alone was rejected three times, for a budget
    # this orchestrator measured wrong, a house rule its packet never stated,
    # and a gate that could not pass on the untouched baseline. That is the
    # number which says where to invest, and it has never been queryable
    # because it lived in narrative. Coded, `--fault GATE` on one landing makes
    # "what fraction of our rejections are self-inflicted" a grep.
    #
    # Recorded per LANDING, not per rejection, so it is the dominant cause of
    # the round trips this packet took; `attempts` already carries the count.
    if args.fault:
        row['fault'] = args.fault
    if args.fault_note:
        row['fault_note'] = args.fault_note
    if verdict.get('amended_by'):
        row['amended_by'] = verdict['amended_by']
    with (REPO_ROOT / 'loop' / 'LEDGER.jsonl').open('a', encoding='utf-8', newline='\n') as f:
        f.write(json.dumps(row) + "\n")

    # --- PACKETS.jsonl -------------------------------------------------------
    pkts = REPO_ROOT / 'loop' / 'PACKETS.jsonl'
    lines, flipped = [], []
    for l in pkts.read_text(encoding='utf-8').splitlines():
        if not l.strip():
            continue
        d = json.loads(l)
        if d['id'] in all_ids and d.get('status') != 'DONE':
            d['status'] = 'DONE'
            flipped.append(d['id'])
        lines.append(json.dumps(d))
    pkts.write_text("\n".join(lines) + "\n", encoding='utf-8', newline='\n')
    missing = [i for i in all_ids if i not in flipped]

    # --- the ratchet ---------------------------------------------------------
    # Measured AFTER the merge, so this is the true post-merge count and the
    # ceiling lands equal to it. Equal is the invariant: a ceiling above the
    # count is headroom nobody is entitled to.
    count = gate4_count()
    ceiling_file = REPO_ROOT / 'scripts' / 'unscaled_legacy_ceiling.txt'
    before = ceiling_file.read_text(encoding='utf-8').strip()
    ceiling_file.write_text(f"{count}\n", encoding='utf-8', newline='\n')

    git('add', 'loop/results', 'loop/LEDGER.jsonl', 'loop/PACKETS.jsonl',
        'scripts/unscaled_legacy_ceiling.txt')
    msg = (f"loop: file {pid} as DONE, ceiling {before} -> {count}\n\n"
           f"Verified ACCEPTED at {head[:7]} on branch {branch}.\n"
           "The ceiling is set to the measured post-merge count, not left at the\n"
           "dispatch budget -- a ceiling above the count is headroom nobody earned.\n\n"
           "Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>\n")
    res = git('-c', 'user.name=Stefan Golas', '-c', 'user.email=stefanmgolas@gmail.com',
              'commit', '-q', '-m', msg)
    if res.returncode != 0:
        sys.exit(f"filing commit failed (the merge already happened):\n{res.stderr}")

    print(f"  merged --no-ff, filed loop/results/{pid}.json")
    print(f"  ledger row appended; PACKETS.jsonl DONE for {', '.join(flipped) or '(none)'}")
    if missing:
        print(f"  WARNING: no PACKETS.jsonl row for {', '.join(missing)} -- "
              "a combined packet whose rows are named differently, or a packet "
              "that predates the graph. Check by hand.")
    print(f"  GATE-4 ceiling {before} -> {count}")
    print(f"  landed as {out('rev-parse', '--short', 'HEAD')}")


if __name__ == '__main__':
    main()
