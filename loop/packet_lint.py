#!/usr/bin/env python3
"""packet_lint - pre-dispatch checks for packet bookkeeping no other gate reads.

gen_packet.check owns anchors and budgets; run_packet refuses on them. This
script owns the recorded packet-fault classes that have shipped THROUGH those
checks, one check per ledger fault class:

- CRATES_COVER_WRITE_SET  every vendor/truck/<crate>/ file in write_allow names
  its owning crate in `crates:`. BG-CG-004 verify 1: V3 LINT_UNLINTED because
  the r2 amendment brought truck-geometry into the write set without widening
  `crates:`. A write set is a claim about which crates change; V3 grades it.
- TEST_PATH_OWNERSHIP     test files the packet mentions or writes are either
  owned (write_allow) or read-only context. Two ledger classes: a Template
  mandating a NEW test file missing from write_allow (BG-CAD-P1, V1
  SCOPE_VIOLATION), and a new test file booked over a LANDED test file's path
  (BG-CAD-P7, V5 identity guard). Amending a landed test file in place is
  legitimate (r2 amendments do it) and is reported as a confirm-not-fail.
- DEP_KIND                "<crate> depends on <crate>" prose claims are checked
  against the owning crate's manifest, including the [dev-dependencies] trap:
  an edge that exists only as a dev-dependency cannot be reached from non-test
  code (BG-CAD-P8, E0433, one worker round trip to prove a true-in-words claim
  false in kind).
- FORECAST_NUMBER         "becomes N" forecast parentheticals. RW4's yaml held
  a post-landing value; the M2 packet forecast a wrong N in prose. Numbers in
  forecasts are per-packet derivations; the lint surfaces every one for
  re-derivation rather than guessing which are stale.
- RESULT_PLACEMENT        a worker told only "write RESULT.json" infers a
  location from repo conventions and lands outside its allowlist (session-41).
  The packet must say AT THE WORKTREE ROOT.
- DEPENDS_KNOWN           every depends_on id must exist in PACKETS.jsonl. A
  typo'd dependency is invisible until scheduling.
- DIRECT_DEP              external crates in the template's `use` statements
  must be direct dependencies of the crates whose files the packet writes.
  An absent crate is a guaranteed E0433; a dev-dependency-only crate is fine
  in test code and wrong in src (CG-002 r1: `use cgmath::` in truck-geometry,
  which takes cgmath only through truck_base's re-export).
- QUALIFIED_PATH          `truck_<crate>::mod::Item` paths cited in template
  code must resolve against the vendored tree: every module segment exists,
  and the final identifier appears in the resolved file (re-export aware).
  Catches cited APIs that were renamed, moved, or never existed (the
  session-9 survey class: a proposed call whose generic bound supplies no
  such method would not have compiled).

Usage:
    python loop/packet_lint.py [--quiet] PACKET [PACKET ...]

Exit codes: 0 = no findings, 1 = findings reported, 2 = usage/harness error.
This gate is NEW: it has never watched a real dispatch. Run it on every
future packet, but trust it only as far as its findings check out by hand.
"""

import json
import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
PACKETS_JSONL = REPO_ROOT / 'loop' / 'PACKETS.jsonl'
VENDOR = REPO_ROOT / 'vendor' / 'truck'


def front_block(text):
    # Two packet formats are in the repo: the older ```yaml fence
    # (BG-S0-003 ... CG-007 era) and a --- front-matter block (the CAD-era
    # packets). The fence-only form returned '' for every --- packet, so the
    # lint silently no-opped on them - every check read empty lists and the
    # file reported "clean" while nothing was read (the dropped-rows failure
    # mode, found 2026-09-01 when the new CRATES_NONEMPTY check fired on
    # packets whose crates: line was visibly non-empty).
    m = re.search(r"```yaml\n(.*?)```", text, re.S)
    if m:
        return m.group(1)
    m = re.match(r"\A---\s*\n(.*?)\n---\s*\n", text, re.S)
    return m.group(1) if m else ''


def yaml_scalar_list(yaml_text, field):
    m = re.search(rf"(?m)^{field}:\s*\[([^\]]*)\]", yaml_text)
    if m:
        return [x.strip() for x in m.group(1).split(',') if x.strip()]
    m = re.search(rf"(?m)^{field}:\s*\n((?:\s+-\s+\S+\n?)+)", yaml_text)
    if m:
        return re.findall(r"-\s+(\S+)", m.group(1))
    return []


def crate_of(path):
    m = re.match(r"vendor/truck/(truck-[a-z]+)/", path)
    return m.group(1) if m else None


def manifest_sections(crate):
    """Returns (deps, dev_deps) name sets from the crate's Cargo.toml."""
    cargo = VENDOR / crate / 'Cargo.toml'
    deps, dev = set(), set()
    section = None
    if not cargo.is_file():
        return None, None
    for line in cargo.read_text(encoding='utf-8', errors='replace').splitlines():
        s = line.strip()
        if s.startswith('['):
            section = s
            continue
        if section == '[dependencies]' and '=' in s and not s.startswith('#'):
            deps.add(s.split('=')[0].strip())
        elif section == '[dev-dependencies]' and '=' in s and not s.startswith('#'):
            dev.add(s.split('=')[0].strip())
    return deps, dev


def test_paths_in(text, write_allow):
    """Test-file paths mentioned anywhere, partitioned by ownership."""
    pat = re.compile(r"(?<![\w/])((?:vendor/truck/truck-[a-z]+/)?tests/[A-Za-z0-9_./-]+\.rs)")
    owned = {p for p in write_allow if '/tests/' in p or p.startswith('tests/')}
    seen, owned_hits, foreign = set(), [], []
    for m in pat.finditer(text):
        p = m.group(1)
        if p in seen:
            continue
        seen.add(p)
        if p in write_allow:
            owned_hits.append(p)
        else:
            foreign.append(p)
    return owned_hits, foreign, owned


class Findings(list):
    def add(self, sev, check, msg):
        self.append((sev, check, msg))


def code_blocks(text):
    return re.findall(r"```[^\n]*\n(.*?)```", text, re.S)


EXTERNAL_USE = re.compile(r"(?m)^\s*use\s+([a-z_][a-z0-9_]*)\s*::")
QUALIFIED = re.compile(
    r"\b(truck_(?:base|geotrait|geometry|topology|polymesh|meshalgo"
    r"|modeling|shapeops|stepio|evidence))::([A-Za-z0-9_:]+)")


def owning_crates(write_allow):
    return sorted({c for p in write_allow if (c := crate_of(p))})


def check_direct_deps(text, write_allow, findings):
    externals = set()
    for block in code_blocks(text):
        externals.update(EXTERNAL_USE.findall(block))
    externals -= {'std', 'core', 'alloc', 'crate', 'self', 'super'}
    externals = {e for e in externals if not e.startswith('truck_')}
    crates = owning_crates(write_allow)
    for e in sorted(externals):
        statuses = []
        for c in crates:
            deps, dev = manifest_sections(c)
            if deps is None:
                continue
            if e in deps:
                statuses.append('dep')
            elif e in dev:
                statuses.append('dev')
            else:
                statuses.append('absent')
        if statuses and 'dep' not in statuses:
            if 'absent' in statuses:
                findings.add('FAIL', 'DIRECT_DEP',
                             f"`use {e}::` in template code but {e} is not a direct "
                             f"dependency of {', '.join(crates)} - guaranteed E0433 "
                             "in the crate that lacks it (CG-002 r1 class)")
            else:
                findings.add('WARN', 'DIRECT_DEP',
                             f"`use {e}::` is dev-dependency-only in {', '.join(crates)} "
                             "- fine in test code, E0433 in src")


def resolve_path(crate, segments):
    """Returns (file, checked_identifier) or None.

    Walks the longest module prefix (directories, seg.rs files, or `pub mod`
    declarations), then checks the next segment as a pub item in the file it
    lands in. Segments after the item are method/function calls - not checked.
    """
    cur = VENDOR / crate / 'src'
    i = 0
    while i < len(segments):
        seg = segments[i]
        if (cur / seg).is_dir():
            cur = cur / seg
            i += 1
            continue
        if (cur / f'{seg}.rs').is_file():
            cur = cur / f'{seg}.rs'
            i += 1
            continue
        break
    container = cur if cur.is_file() else (
        cur / 'mod.rs' if (cur / 'mod.rs').is_file() else cur / 'lib.rs')
    if not container.is_file():
        return None
    if i >= len(segments):
        return (container, None)
    item = segments[i]
    body = container.read_text(errors='replace')
    pat_item = rf"\bpub\s+(?:fn|struct|enum|trait|type|const|use|mod)\s+(?:<[^>]*>\s*)?{item}\b"
    pat_named_use = rf"\bpub use\b[^\n]*\b{item}\b"
    if re.search(pat_item, body) or re.search(pat_named_use, body):
        return (container, item)

    def glob_follow(file, depth):
        if depth > 3:
            return None
        for glob_src in re.findall(r"(?m)^\s*pub use\s+([a-z_][a-z0-9_]*)::\*",
                                   file.read_text(errors='replace')):
            sibling = file.parent / f'{glob_src}.rs'
            if not sibling.is_file() and (file.parent / glob_src).is_dir():
                sibling = file.parent / glob_src / 'mod.rs'
            if not sibling.is_file():
                continue
            sbody = sibling.read_text(errors='replace')
            if re.search(pat_item, sbody) or re.search(pat_named_use, sbody):
                return (sibling, item)
            hit = glob_follow(sibling, depth + 1)
            if hit:
                return hit
        return None

    return glob_follow(container, 0)


def check_qualified_paths(text, findings):
    seen = set()
    for block in code_blocks(text):
        for crate, path in QUALIFIED.findall(block):
            segments = [s for s in path.split('::') if s]
            if (crate, path) in seen:
                continue
            seen.add((crate, path))
            dashed = f"truck-{crate.split('_', 1)[1]}"
            resolved = resolve_path(dashed, segments)
            if resolved is None:
                findings.add('FAIL', 'QUALIFIED_PATH',
                             f"{crate}::{path} does not resolve against the vendored "
                             "tree - a cited API that was renamed, moved, or never "
                             "existed; the template will not compile as written")
                continue
            container, item = resolved
            if item and not re.search(rf"\b{item}\b", container.read_text(errors='replace')):
                findings.add('FAIL', 'QUALIFIED_PATH',
                             f"{crate}::{path}: final identifier not found in "
                             f"{container.relative_to(REPO_ROOT)}")


def lint_packet(packet_path, known_ids):
    text = Path(packet_path).read_text(encoding='utf-8', errors='replace')
    yaml_text = front_block(text)
    findings = Findings()

    crates = yaml_scalar_list(yaml_text, 'crates')
    write_allow = yaml_scalar_list(yaml_text, 'write_allow')
    depends_on = yaml_scalar_list(yaml_text, 'depends_on')

    # CRATES_COVER_WRITE_SET
    unowned = sorted({c for p in write_allow if (c := crate_of(p)) and c not in crates})
    if unowned:
        findings.add('FAIL', 'CRATES_COVER_WRITE_SET',
                        f"write_allow touches {', '.join(unowned)} but `crates:` is "
                        f"[{', '.join(crates)}] - V3 will refuse to lint the changed "
                        "crate (BG-CG-004 r3)")
    vendor_crates = {c for p in write_allow if (c := crate_of(p))}
    if vendor_crates and not crates:
        findings.add('FAIL', 'CRATES_COVER_WRITE_SET',
                        'write_allow touches vendor crates but `crates:` is empty')

    packet_class = ''
    cm = re.search(r'(?m)^class:\s*(\S+)', yaml_text)
    if cm:
        packet_class = cm.group(1)

    # CRATES_NONEMPTY (session 46, BG-CK-P0-PREVALENCE r1): verify.py exits
    # before any gate when `crates:` is falsy, and a packet writing
    # root-crate files needs `look` in crates for cargo -p to have a target.
    # Survey packets write no Rust and are exempt.
    if packet_class != 'survey':
        if not crates:
            findings.add('FAIL', 'CRATES_NONEMPTY',
                            '`crates:` is empty - verify.py exits before any gate '
                            '(PREVALENCE r1 shipped this way and burned a verify)')
        root_files = [p for p in write_allow
                      if re.match(r'^(tests|src|examples|benchmarks)/', p)]
        if root_files and 'look' not in crates:
            findings.add('FAIL', 'CRATES_NONEMPTY',
                            f"write_allow writes root-crate files ({root_files[0]}...) "
                            "but `look` is not in `crates:` - the verify's cargo -p "
                            "list has no target for them")

    # H1_NEW_MODULE (session 46, CRATE Section 1 + FREEZE Section 3 - two
    # GATE-1 round trips, identical class): a packet that creates NEW
    # vendor/truck .rs files must state the H-1 requirement
    # (deny(clippy::unwrap_used)) in prose, or the worker's new modules
    # arrive without the header and kernel-gates rejects them post-hoc.
    # Survey packets write no Rust; skipped.
    if packet_class != 'survey':
        new_rs = [p for p in write_allow
                  if p.startswith('vendor/truck/') and p.endswith('.rs')
                  and not (REPO_ROOT / p).exists()]
        glob_new = [p for p in write_allow
                    if p.startswith('vendor/truck/') and p.endswith('/**')]
        if (new_rs or glob_new) and not re.search(r'unwrap_used|H-1', text):
            detail = ', '.join(new_rs[:3]) if new_rs else glob_new[0]
            findings.add('FAIL', 'H1_NEW_MODULE',
                            f"write_allow creates new vendor .rs files ({detail}) but "
                            "the packet never states the H-1 requirement - add "
                            "'new modules carry #![deny(clippy::unwrap_used)]' to the "
                            "Template/house rules (CRATE Section 1 and FREEZE "
                            "Section 3 each burned a GATE-1 round trip on this)")

    # TEST_PATH_OWNERSHIP
    owned_hits, foreign, owned = test_paths_in(text, write_allow)
    for p in sorted(owned_hits):
        if (REPO_ROOT / p).is_file():
            findings.add('WARN', 'TEST_PATH_OWNERSHIP',
                            f"{p} exists at base and is in write_allow - amending a "
                            "landed test file. Legitimate for in-place amendments, "
                            "but renamed landed tests fire the V5 identity guard.")
    for p in sorted(foreign):
        exists = (REPO_ROOT / p).is_file()
        findings.add('WARN', 'TEST_PATH_OWNERSHIP',
                        f"{p} is mentioned but NOT in write_allow "
                        f"({'a LANDED file' if exists else 'does not exist at base'}). "
                        "If the worker must write it, add it to write_allow (P1 miss); "
                        "if it must not clobber a landed file, say so (P7 miss); "
                        "ignore only if it is read-only context.")

    # DEP_KIND
    for m in re.finditer(r"\b(truck-[a-z]+)\s+depends\s+(?:on|upon)\s+(truck-[a-z]+)", text):
        x, y = m.group(1), m.group(2)
        deps, dev = manifest_sections(x)
        line_no = text[:m.start()].count('\n') + 1
        if deps is None:
            findings.add('FAIL', 'DEP_KIND', f"line {line_no}: no manifest for {x}")
        elif y in deps:
            pass
        elif y in dev:
            findings.add('FAIL', 'DEP_KIND',
                            f"line {line_no}: {x}'s edge to {y} is a "
                            "[dev-dependencies] entry - non-test code cannot reach "
                            "it (the P8 E0433 class)")
        else:
            findings.add('FAIL', 'DEP_KIND',
                            f"line {line_no}: {x} has no manifest edge to {y} - "
                            "check KIND, version and direction before dispatch")

    # DIRECT_DEP + QUALIFIED_PATH (template-code claims about the tree)
    check_direct_deps(text, write_allow, findings)
    check_qualified_paths(text, findings)

    # FORECAST_NUMBER
    for m in re.finditer(r"\bbecomes\s+\d+\b", text):
        line_no = text[:m.start()].count('\n') + 1
        findings.add('WARN', 'FORECAST_NUMBER',
                        f"line {line_no}: forecast '{m.group(0)}' - re-derive against "
                        "the tree; forecasts have shipped wrong (RW4 yaml, M2 prose)")

    # RESULT_PLACEMENT
    if 'RESULT.json' in text and not re.search(
            r"(?i)RESULT\.json`?\s+(?:AT\s+)?THE\s+(?:WORKTREE|wt)\s+ROOT", text):
        findings.add('WARN', 'RESULT_PLACEMENT',
                        'no "RESULT.json AT THE WORKTREE ROOT" instruction - a worker '
                        'left to infer the location has written it elsewhere (session-41)')

    # DEPENDS_KNOWN
    for dep in depends_on:
        if known_ids is not None and dep not in known_ids:
            findings.add('WARN', 'DEPENDS_KNOWN',
                            f"depends_on {dep} has no PACKETS.jsonl row - typo, or a "
                            "dependency that was never registered")

    return findings


def main():
    args = [a for a in sys.argv[1:] if a != '--quiet']
    quiet = '--quiet' in sys.argv
    if not args:
        print(__doc__)
        return 2
    known_ids = None
    if PACKETS_JSONL.is_file():
        known_ids = set()
        for line in PACKETS_JSONL.read_text(encoding='utf-8').splitlines():
            if line.strip():
                try:
                    known_ids.add(json.loads(line)['id'])
                except (json.JSONDecodeError, KeyError):
                    pass
    total = 0
    for packet in args:
        findings = lint_packet(packet, known_ids)
        total += len(findings)
        if not findings and not quiet:
            print(f"{Path(packet).name}: clean")
        for sev, check, msg in findings:
            print(f"{Path(packet).name}: {sev} {check}: {msg}")
    return 1 if total else 0


if __name__ == '__main__':
    sys.exit(main())
