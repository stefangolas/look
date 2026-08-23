"""Generate CONTEXT.md for a worker dispatch: a deterministic, prose-free
code bundle built from the packet's own allow lists at dispatch time.

Motivation (session 20): a fresh worker's first turns are a grep/read spiral
reconstructing its local world -- target symbols, callers, trait shapes --
that the orchestrator can compute mechanically. This is compile-time
retrieval instead of agentic retrieval: the worker starts with the 90%
relevant code and spends its turns on the packet's judgement instead.

Rules that keep this honest:
- Only files the packet already names (write_allow/read_allow) are read; no
  speculative traversal.
- No prose, no speculation, no summaries: signatures, doc first-lines,
  caller sites, test names, and (for amendments) diffstats. If it cannot be
  derived from the tree, it does not go in.
- Hard size cap: a bundle larger than the packet is a cost, not a help.
- Regenerated from the tree at every dispatch: it cannot go stale because it
  is never committed (run_packet writes it beside PACKET.md as scaffolding;
  verify.py ignores it by name).

Usage:
    python loop/gen_context.py --packet loop/packets/BG-XXX.md \
        [--worktree loop/slots/N/wt] [--diff-range fc8925f..HEAD]
"""
import argparse
import re
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent

MAX_LINES = 400

# Item signatures worth showing: pub items and impl headers.
ITEM_RE = re.compile(
    r"^\s*(?:#\[[^\]]*\]\s*)*"
    r"(pub(?:\s*\([^)]*\))?\s+)?"
    r"(?:const\s+)?(?:async\s+)?"
    r"(fn|struct|enum|trait|type|const|mod|impl|macro_rules!)\s+"
    r"([A-Za-z_][A-Za-z0-9_]*)"
)
TEST_RE = re.compile(r"^\s*fn\s+([A-Za-z_][A-Za-z0-9_]*)")
SKIP_DIRS = {'target', '.git'}


def read_front_block(packet: Path):
    """The packet's yaml front block fields we need: write_allow/read_allow."""
    text = packet.read_text(encoding='utf-8')
    m = re.search(r"```yaml\n(.*?)```", text, re.S)
    if not m:
        return [], []
    block = m.group(1)
    def grab(key):
        mm = re.search(rf"^{key}:\s*\n((?:\s+-\s+\S+.*\n)+)", block, re.M)
        if not mm:
            return []
        return [l.strip().lstrip('- ').strip() for l in mm.group(1).strip().splitlines()]
    return grab('write_allow'), grab('read_allow')


def signatures(path: Path, limit_per_file=60):
    """(kind, name, line_no, doc_first_line) for pub items and impls."""
    out = []
    doc = ''
    for no, line in enumerate(path.read_text(encoding='utf-8', errors='replace').splitlines(), 1):
        s = line.strip()
        if s.startswith('///'):
            if not doc and len(s) > 4 and not s.startswith('/// @'):
                doc = s.lstrip('/ ')
            continue
        if s.startswith('#[') or not s:
            continue
        m = ITEM_RE.match(line)
        if m:
            pub, kind, name = m.group(1), m.group(2), m.group(3)
            if pub or kind == 'impl':
                out.append((kind, name, no, doc))
                if len(out) >= limit_per_file:
                    break
        doc = ''
    return out


def test_names(path: Path, limit=40):
    names, in_tests = [], False
    for line in path.read_text(encoding='utf-8', errors='replace').splitlines():
        if re.search(r"mod\s+tests", line):
            in_tests = True
        if in_tests:
            m = TEST_RE.match(line)
            if m and m.group(1).endswith(('_test', 'tests')) or (m and ('_' in m.group(1))):
                pass  # keep any fn in the test module, filter below
            if m:
                names.append(m.group(1))
                if len(names) >= limit:
                    break
    return names


def callers(name: str, defining: Path, crate_root: Path, limit=12):
    """File:line sites calling/mentioning `name` outside its defining file."""
    hits = []
    for f in crate_root.rglob('*.rs'):
        if f == defining or any(p in f.parts for p in SKIP_DIRS):
            continue
        try:
            for no, line in enumerate(
                    f.read_text(encoding='utf-8', errors='replace').splitlines(), 1):
                if name in line and ('(' in line or '::' in line or f'name {name}' in line):
                    hits.append((f, no, line.strip()[:100]))
                    if len(hits) >= limit:
                        return hits
        except OSError:
            continue
    return hits


def generate(packet: Path, wt: Path, diff_range=None, out=None) -> Path:
    """Write <wt>/CONTEXT.md (or `out`) and return its path."""
    write_allow, read_allow = read_front_block(packet)

    lines = ['# CONTEXT.md - mechanically generated from the tree at dispatch time.',
             '# Signatures, callers, tests only. No claims. Regenerated per dispatch.',
             '']

    def emit_file(rel, role):
        f = wt / rel
        if not f.is_file():
            lines.append(f'## {role}: {rel} (MISSING at dispatch time)')
            lines.append('')
            return []
        lines.append(f'## {role}: {rel}')
        sigs = signatures(f)
        if sigs:
            for kind, name, no, doc in sigs:
                d = f' - {doc[:88]}' if doc else ''
                lines.append(f'L{no:<5} {kind:<8} {name}{d}')
        tests = test_names(f)
        if tests:
            lines.append(f'tests: {", ".join(tests)}')
        lines.append('')
        return [name for _, name, _, _ in sigs if _ is not None]

    defined = []
    for rel in write_allow:
        defined += emit_file(rel, 'WRITE')
    for rel in read_allow:
        emit_file(rel, 'READ')

    # Direct callers of the names the packet will define/touch, inside the
    # affected crates only (crate root guessed from the allow-list paths).
    crate_roots = {}
    for rel in write_allow + read_allow:
        m = re.match(r"(vendor/truck/[^/]+)/src", rel.replace('\\', '/'))
        if m:
            crate_roots.setdefault(m.group(1), wt / m.group(1))
    if defined and crate_roots:
        lines.append('## CALLER SITES (grep of defining names outside their file)')
        shown = 0
        for name in defined:
            for root in crate_roots.values():
                for f, no, txt in callers(name, wt / 'x', root):
                    rel = f.relative_to(wt)
                    lines.append(f'{name}  {rel}:{no}  {txt}')
                    shown += 1
        if not shown:
            lines.append('(none found - likely a new module)')
        lines.append('')

    if diff_range:
        lines.append(f'## AMENDMENT DIFF {diff_range}')
        for cmd in (['git', 'log', '--oneline', diff_range],
                    ['git', 'diff', '--stat', diff_range]):
            res = subprocess.run(cmd, capture_output=True, text=True,
                                 encoding='utf-8', errors='replace')
            lines += res.stdout.strip().splitlines()[:40]
        lines.append('')

    if len(lines) > MAX_LINES:
        lines = lines[:MAX_LINES] + [f'... (truncated at {MAX_LINES} lines)']

    out_path = Path(out) if out else wt / 'CONTEXT.md'
    out_path.write_text('\n'.join(lines) + '\n', encoding='utf-8', newline='\n')
    return out_path


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--packet', required=True)
    ap.add_argument('--worktree', default=str(REPO_ROOT))
    ap.add_argument('--diff-range', help='e.g. fc8925f..HEAD for amendments')
    ap.add_argument('--out', help='default: <worktree>/CONTEXT.md')
    args = ap.parse_args()
    out = generate(Path(args.packet), Path(args.worktree), args.diff_range, args.out)
    write_allow, read_allow = read_front_block(Path(args.packet))
    print(f'{out}: {len(write_allow)} write + {len(read_allow)} read files')


if __name__ == '__main__':
    main()
