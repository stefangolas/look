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
    m = re.search(r"```yaml\n(.*?)```", text, re.S)
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
