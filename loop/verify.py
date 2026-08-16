"""The only acceptance authority for a packet (§5). Runs in a slot's worktree,
never trusts the worker's RESULT.json, and gates on what's actually in the
diff. Exit 0 = ACCEPTED, 1 = REJECTED, 2 = BLOCKED.

Usage: python loop/verify.py --slot 0 --packet loop/packets/BG-S0-002.md [--base <ref>]

--base is the ref the diff is computed against (merge-base for a normal
packet run). Defaults to where the slot branch diverged from
integration/kernel-bg, matching how new_slot.py forks a slot branch.
"""
import argparse
import datetime
import json
import re
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent


def git(wt, *args, check=False):
    """Run git -C <wt> <args> and return (returncode, stdout)."""
    res = subprocess.run(['git', '-C', str(wt), *args], capture_output=True, text=True)
    if check and res.returncode != 0:
        raise RuntimeError(f"git {' '.join(args)} failed: {res.stderr}")
    return res.returncode, res.stdout


def git_lines(wt, *args):
    _, out = git(wt, *args)
    return [l for l in out.splitlines() if l != '']


class Verifier:
    def __init__(self, slot, packet, base):
        self.slot = slot
        self.packet = packet
        self.slot_root = REPO_ROOT / 'loop' / 'slots' / str(slot)
        self.wt = self.slot_root / 'wt'
        self.target_dir = self.slot_root / 'target'
        self.out_file = self.slot_root / 'out.txt'
        self.verdict_file = self.slot_root / 'VERDICT.json'
        self.base = base
        self.gates = []
        self.failed_early = False

    # -- output capture -----------------------------------------------------

    def write_out_section(self, header):
        with self.out_file.open('a', encoding='utf-8', newline='\n') as f:
            f.write(f"\n===== {header} =====\n")

    def invoke_native(self, args, cwd):
        """Run a native command, appending its combined stdout+stderr to
        out.txt, and return its exit code. subprocess.run's own
        stdout/stderr capture (rather than PowerShell's stream redirection)
        has none of the ErrorRecord-wrapping hazard the PS1 comment
        describes, so this is a plain run-and-append."""
        res = subprocess.run(args, cwd=str(cwd), capture_output=True, text=True)
        with self.out_file.open('a', encoding='utf-8', newline='\n') as f:
            f.write(res.stdout)
            f.write(res.stderr)
        return res.returncode

    def add_gate(self, name, status, detail=''):
        self.gates.append({'name': name, 'status': status, 'detail': detail})
        print(f"{name} ... {status}")


# ---------------------------------------------------------------------------
# Packet parsing. gen-packet.ps1 (§10 step 2) does not exist yet, so this
# parser has to cope with a hand-written JSON or YAML/markdown packet: JSON
# is parsed properly; YAML/markdown is scraped with a tolerant regex reader
# rather than a real parser, since a handful of scalar and list fields does
# not justify a YAML dependency (and this stays stdlib-only).
# ---------------------------------------------------------------------------

def yaml_list_field(text, key):
    m = re.search(rf"(?m)^{key}:\s*\[(.*?)\]", text)
    if m:
        return [p.strip().strip('"\'') for p in m.group(1).split(',') if p.strip().strip('"\'') != '']
    lines = re.split(r"\r?\n", text)
    items = []
    capture = False
    for line in lines:
        if not capture:
            if re.match(rf"^{key}:\s*$", line):
                capture = True
            continue
        m = re.match(r"^\s*-\s*(.+?)\s*$", line)
        if m:
            item = m.group(1).split('#')[0].strip().strip('"\'')
            if item != '':
                items.append(item)
        elif line.strip() == '':
            continue
        else:
            break
    return items


def read_packet_fields(path):
    raw = path.read_text(encoding='utf-8')
    ext = path.suffix.lower()

    if ext == '.json':
        obj = json.loads(raw)
        return {
            'crates': list(obj.get('crates', [])),
            'write_allow': list(obj.get('write_allow', [])),
            'tests_required': list(obj.get('tests_required', [])),
        }

    yaml_text = raw
    m = re.search(r"(?s)```ya?ml\s*\r?\n(.*?)```", raw)
    if m:
        yaml_text = m.group(1)

    return {
        'crates': yaml_list_field(yaml_text, 'crates'),
        'write_allow': yaml_list_field(yaml_text, 'write_allow'),
        'tests_required': yaml_list_field(yaml_text, 'tests_required'),
    }


def find_bash():
    # Git Bash explicitly. A bare `bash` on PATH resolves to the WindowsApps
    # WSL stub first, which fails with "execvpe(/bin/bash) failed" -- an
    # exit 1 that reads as a house-rule violation rather than a missing
    # interpreter.
    for candidate in (r'C:\Program Files\Git\bin\bash.exe', r'C:\Program Files (x86)\Git\bin\bash.exe'):
        if Path(candidate).exists():
            return candidate
    return None


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--slot', type=int, required=True)
    ap.add_argument('--packet', required=True)
    ap.add_argument('--base')
    args = ap.parse_args()

    v = Verifier(args.slot, args.packet, args.base)

    if not v.wt.is_dir():
        sys.exit(f"slot {v.slot} has no worktree at {v.wt}; run new_slot.py first")
    packet_path = Path(args.packet)
    if not packet_path.is_file():
        sys.exit(f"packet not found: {args.packet}")

    v.slot_root.mkdir(parents=True, exist_ok=True)
    if v.out_file.exists():
        v.out_file.unlink()
    v.out_file.touch()

    pkt = read_packet_fields(packet_path)
    if not pkt['crates']:
        sys.exit(f"could not read 'crates' field from packet {args.packet}")

    # `crates` holds cargo package names, not paths -- cargo -p wants the name
    # and the vendored directory happens to match it, so the directory is
    # only used to catch a typo'd package name before we spend a build on it.
    for c in pkt['crates']:
        probe = v.wt / 'vendor' / 'truck' / c / 'Cargo.toml'
        if c != 'look' and not probe.is_file():
            sys.exit(f"packet names crate '{c}' but {probe} does not exist")
    crate_names = list(pkt['crates'])
    # Repeated as -p a -p b ... in every cargo invocation below.
    p_args = []
    for c in crate_names:
        p_args += ['-p', c]

    base = v.base
    if not base:
        rc, out = git(v.wt, 'merge-base', 'HEAD', 'integration/kernel-bg')
        base = out.strip()
        if not base:
            sys.exit("could not compute a default --base (no merge-base with integration/kernel-bg); pass --base explicitly")
    v.base = base

    diff_range = f"{base}...HEAD"

    # -----------------------------------------------------------------------
    # V0 preflight — did a run actually finish? Every gate below reads the
    # diff between base and HEAD, so a worker that died mid-packet -- killed,
    # out of turns, or holding a connection that dropped -- leaves its edits
    # uncommitted and presents an EMPTY diff. Empty passes V1 through V6 on
    # nothing at all and reports ACCEPTED. A verifier that certifies an
    # interrupted run is worse than no verifier, so incompleteness is checked
    # before anything is measured.
    #
    # BLOCKED, not REJECTED: nothing here is a judgement about the worker's
    # code. The packet is redispatchable as-is once the worktree is reset.
    # -----------------------------------------------------------------------
    commits_ahead = len(git_lines(v.wt, 'rev-list', f"{base}..HEAD"))
    porcelain = git_lines(v.wt, 'status', '--porcelain')
    uncommitted = [l for l in porcelain if not re.search(r'(?i)\s(PACKET\.md|worker\.(pid|err|packet))$', l)]
    has_result = (v.wt / 'RESULT.json').is_file()
    has_question = (v.wt / 'QUESTION.md').is_file()

    preflight = []
    if commits_ahead == 0:
        preflight.append(f"no commit since {base[:7]} -- the worker never finished")
    if len(uncommitted) > 0:
        preflight.append(f"{len(uncommitted)} uncommitted change(s) left in the worktree")
    if not has_result and not has_question:
        preflight.append("no RESULT.json and no QUESTION.md")

    if preflight:
        v.add_gate('V0 preflight', 'FAIL', '; '.join(preflight))
        for n in ('V1 scope', 'V2 build', 'V3 lint', 'V4 house rules', 'V5 tests', 'V6 test-reality'):
            v.add_gate(n, 'SKIP', 'run incomplete')
        print()
        print('VERDICT: BLOCKED')
        verdict = {
            'packet': args.packet, 'slot': v.slot, 'crates': crate_names, 'base': base,
            'verdict': 'BLOCKED', 'gates': v.gates,
            'timestamp': datetime.datetime.now(datetime.timezone.utc).isoformat().replace('+00:00', 'Z'),
        }
        v.verdict_file.write_text(json.dumps(verdict, indent=4), encoding='utf-8')
        sys.exit(2)

    v.add_gate('V0 preflight', 'PASS',
               f"{commits_ahead} commit(s), worktree clean, {'RESULT.json' if has_result else 'QUESTION.md'} written")

    # -----------------------------------------------------------------------
    # V1 scope — git diff --name-only <base>...HEAD must be a subset of write_allow.
    # -----------------------------------------------------------------------
    changed = git_lines(v.wt, 'diff', '--name-only', diff_range)

    # write_allow entries are repo-relative, the same form `git diff --name-only`
    # prints, so they compare directly once separators are normalised. RESULT.json
    # is the packet's own required output and is always in scope.
    allowed_rel = set(p.replace('\\', '/').strip() for p in pkt['write_allow']) | {'RESULT.json', 'QUESTION.md', 'PACKET.md'}

    offenders = [f for f in changed if f.replace('\\', '/') not in allowed_rel]

    if offenders:
        v.add_gate('V1 scope', 'FAIL', "out-of-allowlist paths: " + ', '.join(offenders))
        v.failed_early = True
    else:
        v.add_gate('V1 scope', 'PASS', f"{len(changed)} changed file(s), all within write_allow")

    env_extra = {'CARGO_INCREMENTAL': '0', 'CARGO_TARGET_DIR': str(v.target_dir)}
    import os
    os.environ.update(env_extra)

    # -----------------------------------------------------------------------
    # V2 build — cargo check --locked -p <crate>
    # -----------------------------------------------------------------------
    if not v.failed_early:
        v.write_out_section('V2 build: cargo check --locked -p')
        exit_code = v.invoke_native(['cargo', 'check', '--locked', *p_args], v.wt)
        if exit_code == 0:
            v.add_gate('V2 build', 'PASS')
        else:
            v.add_gate('V2 build', 'FAIL', f"cargo check exit {exit_code}; see out.txt")
            v.failed_early = True
    else:
        v.add_gate('V2 build', 'SKIP', 'earlier gate failed')

    # -----------------------------------------------------------------------
    # V3 lint — cargo fmt --check, then cargo clippy -D warnings
    # -----------------------------------------------------------------------
    if not v.failed_early:
        v.write_out_section('V3 lint: cargo fmt --check -p')
        fmt_exit = v.invoke_native(['cargo', 'fmt', '--check', *p_args], v.wt)

        if fmt_exit != 0:
            v.add_gate('V3 lint', 'FAIL', "cargo fmt --check failed; see out.txt")
            v.failed_early = True
        else:
            # Diff-scoped, for the same reason kernel-gates.sh is: the vendored
            # tree is nowhere near clippy-clean. truck-meshalgo alone carries
            # ~93 lints and truck-modeling's own geometry.rs trips borrowed_box
            # on a line BG-S0-001 landed. A whole-crate `-D warnings` gate
            # therefore fails on every packet regardless of its work, which is
            # the same as no gate -- it cannot tell a worker's defect from the
            # baseline's.
            #
            # So: no `-D warnings` (other crates' lints stay warnings and do
            # not abort the run before ours is linted), --message-format=short
            # for one greppable `path:line:col: level: msg` per finding, and a
            # FAIL only when a finding names a file this packet actually
            # changed.
            v.write_out_section('V3 lint: cargo clippy (diff-scoped)')
            len_before = v.out_file.stat().st_size
            v.invoke_native(['cargo', 'clippy', *p_args, '--all-targets', '--message-format=short'], v.wt)

            clippy_text = v.out_file.read_bytes()[len_before:].decode('utf-8', errors='replace')

            # Scoped to added lines, not merely to touched files: a packet
            # that edits a file inheriting a lint would otherwise be rejected
            # for its predecessor's work. BG-S0-003 was, on geometry.rs:294 --
            # a borrowed_box BG-S0-001 wrote and this packet never looked at.
            added_lines = {}
            for f in changed:
                key = f.replace('\\', '/').lower()
                lineset = set()
                _, hunk_out = git(v.wt, 'diff', '-U0', diff_range, '--', f)
                for h in hunk_out.splitlines():
                    hm = re.match(r'^@@ -\S+ \+(\d+)(?:,(\d+))? @@', h)
                    if hm:
                        start = int(hm.group(1))
                        count = int(hm.group(2)) if hm.group(2) else 1
                        for i in range(count):
                            lineset.add(start + i)
                added_lines[key] = lineset

            our_findings = []
            for line in clippy_text.splitlines():
                fm = re.match(r'^(.+?):(\d+):\d+:\s+(error|warning)', line)
                if not fm:
                    continue
                path = fm.group(1).replace('\\', '/').lower()
                line_no = int(fm.group(2))
                for key, lineset in added_lines.items():
                    if path.endswith(key) and line_no in lineset:
                        our_findings.append(line)
                        break

            if our_findings:
                v.add_gate('V3 lint', 'FAIL', "clippy findings in changed files: " + ' ; '.join(our_findings[:5]))
                v.failed_early = True
            else:
                v.add_gate('V3 lint', 'PASS', 'fmt clean; no clippy finding in any changed file')
    else:
        v.add_gate('V3 lint', 'SKIP', 'earlier gate failed')

    # -----------------------------------------------------------------------
    # V4 house rules — scripts/kernel-gates.sh <base>, diff-scoped, H-1/H-3/H-4.
    # -----------------------------------------------------------------------
    if not v.failed_early:
        v.write_out_section('V4 house rules: kernel-gates.sh')
        bash = find_bash()
        if not bash:
            sys.exit("no Git Bash found; V4 needs it to run scripts/kernel-gates.sh")
        gates_exit = v.invoke_native([bash, 'scripts/kernel-gates.sh', base], v.wt)
        if gates_exit == 0:
            v.add_gate('V4 house rules', 'PASS')
        else:
            v.add_gate('V4 house rules', 'FAIL', f"kernel-gates.sh exit {gates_exit}; see out.txt")
            v.failed_early = True
    else:
        v.add_gate('V4 house rules', 'SKIP', 'earlier gate failed')

    # -----------------------------------------------------------------------
    # V5 tests — cargo test -p <crate> --lib --tests. Never bare `cargo test`.
    # -----------------------------------------------------------------------
    if not v.failed_early:
        v.write_out_section('V5 tests: cargo test -p --lib --tests')
        test_exit = v.invoke_native(['cargo', 'test', *p_args, '--lib', '--tests'], v.wt)
        if test_exit == 0:
            v.add_gate('V5 tests', 'PASS')
        else:
            v.add_gate('V5 tests', 'FAIL', f"cargo test exit {test_exit}; see out.txt")
            v.failed_early = True
    else:
        v.add_gate('V5 tests', 'SKIP', 'earlier gate failed')

    # -----------------------------------------------------------------------
    # V6 test-reality — every tests_required entry must correspond to a test fn
    # name actually present in the diff, and if the crate disables autotests
    # (the truck-polymesh precedent), any new tests/*.rs file must be declared
    # as a [[test]] target or it silently never runs.
    #
    # NOTE / known limitation: tests_required entries in the spec are prose
    # descriptions ("property: 10^4 sampled points ... lie in enclose(box)"),
    # not literal fn names, and no gen-packet.ps1 exists yet to establish a
    # fn-naming convention between the packet and the worker. Absent that
    # convention this gate can only apply a heuristic — normalized keyword
    # overlap between each tests_required string and the #[test]-attributed fn
    # names added in the diff — rather than an exact name match. That heuristic
    # is intentionally conservative in the FAIL direction (it flags rather than
    # rubber-stamps when in doubt); tightening it to exact-name matching is
    # gen-packet.ps1's job, once it defines the naming convention it will emit.
    # -----------------------------------------------------------------------
    if not v.failed_early:
        _, diff_text = git(v.wt, 'diff', diff_range, '--', '*.rs')
        added_lines_rs = [l for l in re.split(r'\r?\n', diff_text) if re.match(r'^\+[^+]', l)]

        # Collect fn names on or immediately after a #[test]/#[proptest]-style
        # attribute line among the added lines (order-preserving scan).
        test_fn_names = []
        pending_test_attr = False
        for line in added_lines_rs:
            body = line[1:]
            if re.search(r'#\[\s*(test|proptest|test_case|tokio::test)', body):
                pending_test_attr = True
                continue
            fm = re.search(r'fn\s+([A-Za-z0-9_]+)', body)
            if fm:
                if pending_test_attr:
                    test_fn_names.append(fm.group(1))
                    pending_test_attr = False
                continue
            if body.strip() != '' and not re.match(r'^\s*(#\[|//)', body):
                # non-attribute, non-comment line breaks the "immediately
                # after" adjacency assumption for a pending attribute.
                pending_test_attr = False

        missing_required = []
        for req in pkt['tests_required']:
            keywords = [w.lower() for w in re.split(r'\s+', re.sub(r'[^A-Za-z0-9 ]', ' ', req)) if len(w) >= 4]
            hit = False
            for fn in test_fn_names:
                fn_lower = fn.lower()
                if any(kw in fn_lower for kw in keywords):
                    hit = True
                    break
            if not hit:
                missing_required.append(req)

        # autotests=false precedent: new tests/*.rs files must be declared.
        added_files = git_lines(v.wt, 'diff', '--name-only', '--diff-filter=A', diff_range)
        undeclared = []
        for c in crate_names:
            ctoml = v.wt / 'vendor' / 'truck' / c / 'Cargo.toml'
            if not ctoml.is_file():
                continue
            ctext = ctoml.read_text(encoding='utf-8')
            if not re.search(r'autotests\s*=\s*false', ctext):
                continue
            new_test_files = [f for f in added_files if f"truck/{c}/" in f and re.search(r'[\\/]tests[\\/].+\.rs$', f)]
            for f in new_test_files:
                base_name = Path(f).stem
                if base_name not in ctext:
                    undeclared.append(f)

        if missing_required or undeclared:
            detail_parts = []
            if missing_required:
                detail_parts.append("no matching test fn for: " + ' | '.join(missing_required))
            if undeclared:
                detail_parts.append("new test file(s) not declared as [[test]] under autotests=false: " + ', '.join(undeclared))
            v.add_gate('V6 test-reality', 'FAIL', '; '.join(detail_parts))
            v.failed_early = True
        else:
            v.add_gate('V6 test-reality', 'PASS',
                       f"{len(pkt['tests_required'])} required test(s) matched, {len(test_fn_names)} test fn(s) found in diff")
    else:
        v.add_gate('V6 test-reality', 'SKIP', 'earlier gate failed')

    # -----------------------------------------------------------------------
    # V7 mutation spot-check — TODO. Per §5: for the enumerated items (BG-FID-008
    # double cover, BG-NUM-002 double root, BG-NUM-004 F-2 both directions,
    # BG-CE-002 offset pcurve), re-run the packet's negative test against a
    # deliberately weakened implementation (a #[cfg(test)] mod vacuity harness
    # checked in beside the test) and assert it fails. Not implemented: doing
    # this correctly needs the packet schema to name which test is the negative
    # test and where the weakened-impl harness lives, and no packet has that
    # field yet (gen-packet.ps1 doesn't exist). Always passes until then.
    # -----------------------------------------------------------------------
    v.add_gate('V7 mutation spot-check', 'PASS', 'TODO stub — always passes; see comment in verify.py')

    # -----------------------------------------------------------------------
    # V8 no-regression — TODO. Per §5: the previously accepted wave's tests still
    # pass. Not implemented: this needs LEDGER.jsonl / PACKETS.jsonl to know what
    # "the previous wave" is and which crates it touched, which is orchestrator
    # state that doesn't exist yet. Always passes until then.
    # -----------------------------------------------------------------------
    v.add_gate('V8 no-regression', 'PASS', 'TODO stub — always passes; see comment in verify.py')

    # -----------------------------------------------------------------------
    # Verdict
    # -----------------------------------------------------------------------
    accepted = not any(g['status'] == 'FAIL' for g in v.gates)

    print()
    print(f"VERDICT: {'ACCEPTED' if accepted else 'REJECTED'}")
    print(f"packet={args.packet} slot={v.slot} crates={','.join(crate_names)} base={base}")

    verdict = {
        'packet': args.packet,
        'slot': v.slot,
        'crates': crate_names,
        'base': base,
        'verdict': 'ACCEPTED' if accepted else 'REJECTED',
        'gates': v.gates,
        'timestamp': datetime.datetime.now(datetime.timezone.utc).isoformat().replace('+00:00', 'Z'),
    }
    v.verdict_file.write_text(json.dumps(verdict, indent=4), encoding='utf-8')

    sys.exit(0 if accepted else 1)


if __name__ == '__main__':
    main()
