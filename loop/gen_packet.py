"""Make a packet's claims about the repo checkable, and generate the parts of a
packet that are mechanical.

A packet asserts things about the tree -- "circle.rs has 4 matching lines", "this
shard needs 12 contexts" -- and until now nothing checked them at the moment they
mattered. Both halves of that went wrong in session 7 on one packet:

  * three of seven anchor counts were WRONG WHEN WRITTEN, on files unchanged
    since before the packet existed. The worker stops on a mismatch (H-8), which
    is correct and costs a dispatch.
  * `unscaled_legacy_budget` was an estimate ("about 12 here") against a true
    19, so a faithful commit could not pass GATE-4. The worker did the work,
    hit the ceiling, and returned SPEC_GAP -- also correct, also a dispatch.

Neither is a hard problem. Both are commands nobody ran.

    python loop/gen_packet.py --check loop/packets/BG-XXX.md
    python loop/gen_packet.py --check-all
    python loop/gen_packet.py --skeleton loop/surveys/BG-XXX.json --id BG-XXX --crate truck-meshalgo

`--check` executes the packet's `anchors:` block and compares each count, and
checks `unscaled_legacy_budget` against the census. It is wired into
run_packet.py, so a packet whose claims have rotted cannot be dispatched.

`--skeleton` emits the mechanical sections -- front block, anchors table, site
table -- from a REVIEWED survey. It deliberately does not write the prose.
Problem, Decisions-already-made and Stop-conditions are where a packet's value
is, they are what makes a flash-class worker churn instead of design, and
generating them from a template would produce a packet that looks finished and
decides nothing.
"""
import argparse
import json
import re
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
BASH = r'C:\Program Files\Git\bin\bash.exe'


def front_block(text):
    m = re.search(r"(?s)```ya?ml\s*\r?\n(.*?)```", text)
    return m.group(1) if m else text


def parse_anchors(yaml_text):
    """Anchors live in the front block as a list of {id, cmd, expect} so they can
    be RUN. The markdown table in the body is for a human reader; a table is not
    a thing a script can execute, which is the whole reason the counts rotted.

    anchors:
      - {id: A1, expect: 4, cmd: "grep -cE '...' vendor/truck/.../circle.rs"}
    """
    anchors = []
    for line in yaml_text.splitlines():
        m = re.match(r"\s*-\s*\{(.+)\}\s*$", line)
        if not m:
            continue
        body = m.group(1)
        if 'cmd:' not in body:
            continue
        fields = {}
        for key in ('id', 'expect', 'cmd'):
            mk = re.search(rf"{key}:\s*(\"(?:[^\"\\]|\\.)*\"|'[^']*'|[^,]+?)\s*(?:,|$)", body)
            if mk:
                v = mk.group(1).strip()
                if len(v) >= 2 and v[0] in '"\'' and v[-1] == v[0]:
                    v = v[1:-1]
                fields[key] = v
        if 'cmd' in fields and 'expect' in fields:
            anchors.append(fields)
    return anchors


def run_anchor(cmd):
    """Anchors are shell one-liners and run under Git Bash explicitly -- a bare
    `bash` on this host is the WindowsApps WSL stub, which fails with
    execvpe(/bin/bash) and reads as a mismatch rather than a missing shell.
    grep exits 1 on no match, which is a legitimate count of zero, so the exit
    code is not treated as failure."""
    res = subprocess.run([BASH, '-lc', cmd], cwd=str(REPO_ROOT), capture_output=True,
                         text=True, encoding='utf-8', errors='replace')
    got = res.stdout.strip().splitlines()
    if not got:
        return None, res.stderr.strip()
    tail = got[-1].strip()
    if re.fullmatch(r'\d+', tail):
        return int(tail), None
    return None, f"anchor command did not print a bare count, printed {tail[:60]!r}"


def census_functions(fragment):
    res = subprocess.run([sys.executable, str(REPO_ROOT / 'loop' / 'census_tol_sites.py'), fragment],
                         capture_output=True, text=True, encoding='utf-8', errors='replace')
    m = re.search(r"functions with a site under .*?: (\d+)", res.stdout)
    return int(m.group(1)) if m else None


def check(packet_path, quiet=False):
    """Returns a list of problems; empty means the packet's claims still hold."""
    text = Path(packet_path).read_text(encoding='utf-8')
    yaml_text = front_block(text)
    problems = []

    anchors = parse_anchors(yaml_text)
    if not anchors:
        # Not an error. Most packets predate the machine-readable block, and a
        # missing block is reported rather than treated as a pass, because
        # "nothing to check" and "checked, fine" are different facts.
        if not quiet:
            print(f"  no runnable `anchors:` block -- {Path(packet_path).name} is unchecked, not checked")
    for a in anchors:
        got, err = run_anchor(a['cmd'])
        want = int(a['expect'])
        if err:
            problems.append(f"{a.get('id','?')}: {err}")
        elif got != want:
            problems.append(f"{a.get('id','?')}: expected {want}, tree has {got}  [{a['cmd'][:70]}]")
        elif not quiet:
            print(f"  {a.get('id','?'):4} {want:>4}  ok")

    m_budget = re.search(r"(?m)^unscaled_legacy_budget:\s*(\d+)", yaml_text)
    m_frag = re.search(r"(?m)^census_fragment:\s*(\S+)", yaml_text)
    if m_budget and m_frag:
        declared = int(m_budget.group(1))
        measured = census_functions(m_frag.group(1).strip())
        if measured is None:
            problems.append(f"census produced no count for fragment {m_frag.group(1)!r}")
        elif declared != measured:
            problems.append(
                f"unscaled_legacy_budget is {declared}, census measures {measured} "
                f"function(s) with a site under {m_frag.group(1)!r}. The budget is one "
                "context per function; an estimate here costs a whole dispatch.")
        elif not quiet:
            print(f"  budget {declared} == census {measured}  ok")
    elif m_budget and not quiet:
        print("  unscaled_legacy_budget declared without `census_fragment:` -- unchecked")

    return problems


def skeleton(survey_path, pid, crate):
    doc = json.loads(Path(survey_path).read_text(encoding='utf-8'))
    sites = doc['sites'] if isinstance(doc, dict) else doc
    live = [s for s in sites if s.get('classification') in ('model', 'param')]
    files = sorted({s['file'] for s in live})
    fns = {(s['file'], s['symbol']) for s in live}
    low = [s for s in sites if s.get('confidence') == 'low']

    print(f"# WORK PACKET {pid} — Stage-A tolerance migration, {crate}\n")
    print("<!-- SKELETON. The prose is not generated and must be written: Problem,")
    print("     Decisions-already-made, Template, Forbidden, Stop conditions. Copy")
    print("     BG-TOL-001-SHAPEOPS.md. A packet without them makes the worker design. -->\n")
    print("```yaml")
    print(f"id:          {pid}")
    print("contract:    [BG-TOL-001]")
    print("class:       wide-mechanical")
    print(f"crates:      [{crate}]")
    print("depends_on:  [BG-TOL-001-TYPE, BG-TOL-001-TYPE-r2, BG-TOL-001-TYPE-r3]")
    print("write_allow:")
    for f in files:
        print(f"  - {f}")
    print(f"  - vendor/truck/{crate}/tests/tolerance_{crate.replace('truck-', '')}.rs")
    print("read_allow:\n  - vendor/truck/truck-base/src/tolerance.rs")
    print("tests_required:")
    print(f"  - every_migrated_{crate.replace('truck-', '')}_site_is_marked")
    print("budget:      {turns: 70, ctx_tokens: 150000}")
    print(f"census_fragment: {crate}")
    print(f"unscaled_legacy_budget: {len(fns)}")
    print("anchors:")
    for i, f in enumerate(files, 1):
        rel = f.replace('\\', '/')
        cmd = f"grep -cE '\\\\.near\\\\(|so_small\\\\(|TOLERANCE' {rel}"
        got, _ = run_anchor(cmd)
        print(f"  - {{id: A{i}, expect: {got}, cmd: \"{cmd}\"}}")
    print("```\n")
    print(f"## The sites — {len(live)} migrate, {len(fns)} contexts\n")
    print("Line numbers are provenance for a human reader; locate by the enclosing symbol.\n")
    for f in files:
        print(f"**`{f.split('/')[-1]}`**\n")
        print("| enclosing fn | line | code | class |")
        print("|---|---|---|---|")
        for s in [x for x in live if x['file'] == f]:
            expr = str(s['expression']).replace('|', '\\|').strip()
            flag = ' **(low confidence — REVIEW)**' if s.get('confidence') == 'low' else ''
            print(f"| `{s['symbol']}` | {s['line']} | `{expr}` | **`{s['classification']}`**"
                  f" — {s.get('reason', '')}{flag} |")
        print()
    excluded = [s for s in sites if s.get('classification') == 'excluded']
    if excluded:
        print(f"## Not in this packet — {len(excluded)} excluded\n")
        for s in excluded:
            print(f"- `{s['file'].split('/')[-1]}:{s['line']}` — {s.get('reason','')}")
        print()
    if low:
        print(f"<!-- {len(low)} low-confidence row(s) above. Review each against the")
        print("     source before dispatching; that is the half V10 cannot check. -->")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--check')
    ap.add_argument('--check-all', action='store_true')
    ap.add_argument('--skeleton')
    ap.add_argument('--id')
    ap.add_argument('--crate')
    args = ap.parse_args()

    if args.skeleton:
        if not (args.id and args.crate):
            sys.exit("--skeleton needs --id and --crate")
        skeleton(args.skeleton, args.id, args.crate)
        return

    targets = []
    if args.check_all:
        targets = sorted((REPO_ROOT / 'loop' / 'packets').glob('*.md'))
    elif args.check:
        targets = [Path(args.check)]
    else:
        sys.exit("nothing to do: pass --check, --check-all or --skeleton")

    failed = 0
    for t in targets:
        print(f"{t.name}:")
        problems = check(t)
        for p in problems:
            print(f"  MISMATCH {p}")
        if problems:
            failed += 1
    if failed:
        print(f"\n{failed} packet(s) with stale claims. A mismatch is a stop condition (H-8), "
              "not a nuisance: fix the packet, or the tree moved and the packet must be re-scoped.")
        sys.exit(1)
    print("\nall checked claims hold.")


if __name__ == '__main__':
    main()
