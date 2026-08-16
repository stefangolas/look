"""The only acceptance authority for a packet (§5). Runs in a slot's worktree,
never trusts the worker's RESULT.json, and gates on what's actually in the
diff. Exit 0 = ACCEPTED, 1 = REJECTED, 2 = BLOCKED.

Usage: python loop/verify.py --slot 0 --packet loop/packets/BG-S0-002.md [--base <ref>]

--base is the ref the diff is computed against (merge-base for a normal
packet run). Defaults to where the slot branch diverged from
integration/kernel-bg, matching how new_slot.py forks a slot branch.

--only V3,V5 runs just those gates and reports the rest SKIP. This exists
because the amend-and-verify path (amend a proven commit rather than pay for
a fresh ~90-minute worker run) is now the common case, and a full verify is a
4-6 minute cargo cycle even when only one gate's input changed. It can never
produce ACCEPTED: a partial run's verdict is PARTIAL (exit 3), because
nothing about re-checking one gate tells you the others still hold, and
acceptance is a claim about the whole packet, not a subset of it. V0
preflight always runs regardless of --only -- every other gate reads the
diff between base and HEAD, so a verdict is meaningless if the run never
finished.
"""
import argparse
import datetime
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
BASELINES_DIR = REPO_ROOT / 'loop' / 'baselines'


def git(wt, *args, check=False):
    """Run git -C <wt> <args> and return (returncode, stdout).

    UTF-8 with replacement, never the locale codec. A worker writing ω or ε in
    a doc comment -- which the BG-EVD-r3 packet actively asked for -- produces
    bytes cp1252 cannot decode, and the resulting UnicodeDecodeError surfaced
    far away as `'NoneType' object has no attribute 'splitlines'` in the V3
    hunk parser. stdout is coerced to '' so a git failure can never present as
    that same None."""
    res = subprocess.run(['git', '-C', str(wt), *args], capture_output=True, text=True, encoding='utf-8', errors='replace')
    if check and res.returncode != 0:
        raise RuntimeError(f"git {' '.join(args)} failed: {res.stderr}")
    return res.returncode, (res.stdout or '')


def git_lines(wt, *args):
    _, out = git(wt, *args)
    return [l for l in out.splitlines() if l != '']


# ---------------------------------------------------------------------------
# V5 baseline comparison. A test failure has no location in the diff the way
# a clippy finding or an added test fn does, so "regression" can only be
# defined by comparing two full test runs -- the packet's HEAD against its
# base commit -- not by scoping to lines the packet touched. See the V5 gate
# below for why this replaced the added-tests-only scoping.
# ---------------------------------------------------------------------------

_TEST_LINE_RE = re.compile(r'^test\s+(\S+)\s+\.\.\.\s+(ok|FAILED|ignored)')
_COMPILE_ERROR_RE = re.compile(r'error\[E\d+\]|could not compile|error: no test target')


def parse_test_statuses(text):
    """Map full test name (module::path::name, as cargo prints it -- distinct
    test binaries can in principle reuse a bare name, so the full path is kept
    rather than stripped) -> 'ok' | 'FAILED' | 'ignored', last line wins."""
    statuses = {}
    for line in text.splitlines():
        m = _TEST_LINE_RE.match(line)
        if m:
            statuses[m.group(1)] = m.group(2)
    return statuses


def has_compile_error(text):
    return _COMPILE_ERROR_RE.search(text) is not None


def baseline_cache_path(base_sha, crate_names):
    key = base_sha[:12] + '__' + '-'.join(sorted(crate_names))
    safe_key = re.sub(r'[^A-Za-z0-9_.-]', '_', key)
    return BASELINES_DIR / f"{safe_key}.json"


def compute_baseline(base_sha, crate_names, out_file):
    """Run `cargo test -p <crates> --lib --tests --no-fail-fast` at base_sha
    in a throwaway worktree and return {'compile_ok': bool, 'tests': {name:
    status}}. Never touches loop/slots/*; creates and removes its own
    worktree and target dir under the system temp directory, so a baseline
    run cannot collide with (or be mistaken for) a slot's own worktree."""
    tmp_parent = Path(tempfile.mkdtemp(prefix='look-verify-baseline-'))
    wt_path = tmp_parent / 'wt'
    target_path = tmp_parent / 'target'
    p_args = []
    for c in crate_names:
        p_args += ['-p', c]

    with out_file.open('a', encoding='utf-8', newline='\n') as f:
        f.write(f"\n===== V5 baseline: computing at {base_sha[:12]} (worktree {wt_path}) =====\n")

    add_res = subprocess.run(
        ['git', '-C', str(REPO_ROOT), 'worktree', 'add', '--detach', str(wt_path), base_sha],
        capture_output=True, text=True, encoding='utf-8', errors='replace'
    )
    with out_file.open('a', encoding='utf-8', newline='\n') as f:
        f.write(add_res.stdout)
        f.write(add_res.stderr)
    if add_res.returncode != 0:
        shutil.rmtree(tmp_parent, ignore_errors=True)
        raise RuntimeError(f"could not create baseline worktree at {base_sha}: {add_res.stderr}")

    try:
        env = dict(os.environ)
        env['CARGO_INCREMENTAL'] = '0'
        env['CARGO_TARGET_DIR'] = str(target_path)
        test_res = subprocess.run(
            ['cargo', 'test', *p_args, '--lib', '--tests', '--no-fail-fast'],
            cwd=str(wt_path), capture_output=True, text=True, encoding='utf-8', errors='replace', env=env
        )
        chunk = test_res.stdout + test_res.stderr
        with out_file.open('a', encoding='utf-8', newline='\n') as f:
            f.write(chunk)

        compile_ok = not has_compile_error(chunk)
        tests = parse_test_statuses(chunk) if compile_ok else {}
        return {'compile_ok': compile_ok, 'tests': tests}
    finally:
        # Best-effort cleanup. `git worktree remove` first (keeps the repo's
        # worktree list clean); if the target dir is still holding a file
        # open on Windows, at least the git-level registration is gone and a
        # leftover tmp_parent under the OS temp dir is harmless.
        subprocess.run(['git', '-C', str(REPO_ROOT), 'worktree', 'remove', '--force', str(wt_path)],
                        capture_output=True, text=True, encoding='utf-8', errors='replace')
        shutil.rmtree(tmp_parent, ignore_errors=True)


def load_or_compute_baseline(base_sha, crate_names, out_file):
    cache_path = baseline_cache_path(base_sha, crate_names)
    if cache_path.is_file():
        obj = json.loads(cache_path.read_text(encoding='utf-8'))
        with out_file.open('a', encoding='utf-8', newline='\n') as f:
            f.write(f"\n===== V5 baseline: loaded cache {cache_path.name} "
                     f"(computed {obj.get('computed_at', '?')}) =====\n")
        return obj

    BASELINES_DIR.mkdir(parents=True, exist_ok=True)
    result = compute_baseline(base_sha, crate_names, out_file)
    obj = {
        'base': base_sha,
        'crates': sorted(crate_names),
        'computed_at': datetime.datetime.now(datetime.timezone.utc).isoformat().replace('+00:00', 'Z'),
        'compile_ok': result['compile_ok'],
        'tests': result['tests'],
    }
    cache_path.write_text(json.dumps(obj, indent=2, sort_keys=True), encoding='utf-8')
    return obj


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
        res = subprocess.run(args, cwd=str(cwd), capture_output=True, text=True, encoding='utf-8', errors='replace')
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
        elif line.strip() == '' or line.strip().startswith('#'):
            # A comment inside the list is not the end of the list. Before this,
            # a `#` line silently truncated write_allow at that point -- and a
            # write set that is quietly shorter than it reads rejects the very
            # edits the packet authorised, which is how BG-NUM-001-FILLET was
            # rejected twice for a file its allowlist appeared to name.
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
    ap.add_argument('--only', help='comma-separated gate ids (e.g. V3,V5) -- runs only these, '
                                    'SKIPs the rest, and the verdict is PARTIAL, never ACCEPTED')
    args = ap.parse_args()

    only_gates = None
    if args.only:
        only_gates = set(g.strip().upper() for g in args.only.split(',') if g.strip())
        valid_ids = {f'V{n}' for n in range(1, 9)}
        unknown = only_gates - valid_ids
        if unknown:
            sys.exit(f"--only names unknown gate id(s): {', '.join(sorted(unknown))} (valid: {', '.join(sorted(valid_ids))})")

    def gate_wanted(gate_id):
        return only_gates is None or gate_id in only_gates

    def skip_not_requested(name):
        v.add_gate(name, 'SKIP', f"not requested (--only {args.only})")

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

    # Branch + exact commit this run judged, so a VERDICT.json can be traced
    # back to the work it verified rather than reconstructed from prose in
    # STATE.md (that reconstruction is exactly what happened landing
    # BG-S0-002: the slot's branch had moved past what got verified, and
    # nothing on disk said so).
    branch_lines = git_lines(v.wt, 'rev-parse', '--abbrev-ref', 'HEAD')
    branch = branch_lines[0] if branch_lines else '?'
    commit_lines = git_lines(v.wt, 'rev-parse', 'HEAD')
    commit_sha = commit_lines[0] if commit_lines else '?'

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
    # Untracked files used to be ignored wholesale here, on the claim that an
    # uncommitted new source file "is still caught by V1/V6 via the committed
    # diff." That's wrong on the facts: V1/V6 read `git diff <base>...HEAD`,
    # which by construction only shows committed changes -- an uncommitted
    # file is invisible to it. That is exactly the hole V0 exists to close: a
    # worker that died after creating files but before committing.
    #
    # The real reason untracked files were blanket-ignored is that verify's
    # own `cargo test` run (V5, below) drops build artifacts into the
    # worktree it is about to judge -- confirmed by grepping the vendored
    # tree: every hit is a relative std::fs::File::create("*.obj") in a
    # tests/*.rs or src/**/tests.rs (truck-shapeops's fillet tests, mainly).
    # No other extension turned up. Fixing the cause (stop those tests from
    # writing into the worktree) would mean patching upstream-vendored test
    # code outside any packet's write_allow, which is worse than the disease;
    # narrowing the ignore list to exactly what's demonstrated to appear is
    # the smaller, honest fix. If a future crate's tests drop some other
    # artifact, that will show up here as a false BLOCKED and the pattern
    # gets added then, with evidence, not preemptively.
    def _ignorable_untracked(rel_path):
        name = Path(rel_path).name
        if name in ('PACKET.md', 'RESULT.json', 'QUESTION.md'):
            return True
        if rel_path.endswith('.obj'):
            return True
        return False

    uncommitted = []
    for l in porcelain:
        if l.startswith('?? '):
            rel = l[3:].strip().strip('"')
            if _ignorable_untracked(rel):
                continue
            uncommitted.append(l)
        elif not re.search(r'(?i)\s(PACKET\.md|worker\.(pid|err|packet))$', l):
            uncommitted.append(l)
    has_result = (v.wt / 'RESULT.json').is_file()
    has_question = (v.wt / 'QUESTION.md').is_file()

    # amended_by marks a RESULT.json the orchestrator rewrote on top of the
    # worker's own claim -- e.g. dropping a test the worker correctly proved
    # unreachable, once the spec was fixed to agree, rather than paying for a
    # fresh dispatch. That's the cheap path landing BG-S0-002 took, and until
    # now nothing recorded that the commit under verification wasn't the
    # worker's unmodified output. Surfaced here so a verdict can never present
    # amended work as untouched.
    amended_by = None
    if has_result:
        try:
            amended_by = json.loads((v.wt / 'RESULT.json').read_text(encoding='utf-8')).get('amended_by')
        except (json.JSONDecodeError, OSError):
            amended_by = None

    preflight = []
    if commits_ahead == 0:
        preflight.append(f"no commit since {base[:7]} -- the worker never finished")
    if len(uncommitted) > 0:
        preflight.append(f"{len(uncommitted)} uncommitted change(s) left in the worktree: "
                          + ', '.join(l.strip() for l in uncommitted[:8]))
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
            'branch': branch, 'commit': commit_sha, 'amended_by': amended_by,
            'verdict': 'BLOCKED', 'gates': v.gates,
            'timestamp': datetime.datetime.now(datetime.timezone.utc).isoformat().replace('+00:00', 'Z'),
        }
        v.verdict_file.write_text(json.dumps(verdict, indent=4), encoding='utf-8')
        sys.exit(2)

    v.add_gate('V0 preflight', 'PASS',
               f"{commits_ahead} commit(s), worktree clean, {'RESULT.json' if has_result else 'QUESTION.md'} written"
               + (f"; RESULT.json amended_by={amended_by!r}" if amended_by else ''))

    # -----------------------------------------------------------------------
    # V1 scope — git diff --name-only <base>...HEAD must be a subset of write_allow.
    # -----------------------------------------------------------------------
    changed = git_lines(v.wt, 'diff', '--name-only', diff_range)

    # write_allow entries are repo-relative, the same form `git diff --name-only`
    # prints, so they compare directly once separators are normalised. RESULT.json
    # is the packet's own required output and is always in scope.
    allowed_rel = set(p.replace('\\', '/').strip() for p in pkt['write_allow']) | {'RESULT.json', 'QUESTION.md', 'PACKET.md'}

    offenders = [f for f in changed if f.replace('\\', '/') not in allowed_rel]

    # `changed` above is cheap (a name-only diff) and V3/V6 both read it, so
    # it's always computed even when V1 itself isn't in --only -- only the
    # gate verdict is conditional on being requested.
    if not gate_wanted('V1'):
        skip_not_requested('V1 scope')
    elif offenders:
        v.add_gate('V1 scope', 'FAIL', "out-of-allowlist paths: " + ', '.join(offenders))
        v.failed_early = True
    else:
        v.add_gate('V1 scope', 'PASS', f"{len(changed)} changed file(s), all within write_allow")

    env_extra = {'CARGO_INCREMENTAL': '0', 'CARGO_TARGET_DIR': str(v.target_dir)}
    os.environ.update(env_extra)

    # -----------------------------------------------------------------------
    # V2 build — cargo check --locked -p <crate>
    # -----------------------------------------------------------------------
    if not gate_wanted('V2'):
        skip_not_requested('V2 build')
    elif not v.failed_early:
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
    if not gate_wanted('V3'):
        skip_not_requested('V3 lint')
    elif not v.failed_early:
        v.write_out_section('V3 lint: cargo fmt --check -p (scoped to changed files)')
        len_before_fmt = v.out_file.stat().st_size
        fmt_exit = v.invoke_native(['cargo', 'fmt', '--check', *p_args], v.wt)
        fmt_text = v.out_file.read_bytes()[len_before_fmt:].decode('utf-8', errors='replace')

        # Scoped to the files the diff changed, for the same reason the clippy
        # half below is scoped to the lines it added: the vendored tree is not
        # rustfmt-clean at the baseline. BG-NUM-001-FILLET was rejected five
        # times, the last of them here, on
        # truck-geometry/src/decorators/revolved_curve.rs:690 -- a stray blank
        # line that is present at the base commit and that the packet never
        # opened. A whole-crate fmt gate rejects every packet touching
        # truck-geometry for a defect it inherited, which is the same as no
        # gate. Formatting the file to get green is not available either: that
        # file is outside the packet's write_allow, so the fix would be
        # rejected by V1.
        #
        # File granularity, not line granularity, deliberately. rustfmt reports
        # the line where its diff *context* starts, which routinely sits
        # several lines above the text it actually wants to reformat, so
        # intersecting those numbers with the diff's added lines would both
        # miss real findings and invent absent ones. "Every file this packet
        # touched is rustfmt-clean" is a property fmt can state exactly.
        changed_keys = [f.replace('\\', '/').lower() for f in changed]
        fmt_findings = []
        for line in fmt_text.splitlines():
            fm = re.match(r'^Diff in (.+?):(\d+)', line.strip())
            if not fm:
                continue
            path = fm.group(1).replace('\\', '/').lower()
            if any(path.endswith(k) for k in changed_keys):
                fmt_findings.append(f"{fm.group(1)}:{fm.group(2)}")

        # Coverage guard, the same shape as clippy's `unlinted` below: rustfmt
        # exits 1 both when it found diffs and when it could not parse a file,
        # and in the second case there are no `Diff in` lines to scope, so a
        # scoped gate would report PASS on something it never read.
        fmt_broke = [ln for ln in fmt_text.splitlines()
                     if re.match(r'^\s*error(\[|:)', ln)]

        if fmt_broke:
            v.add_gate('V3 lint', 'FAIL',
                       "cargo fmt could not read the tree, so nothing was checked: "
                       + fmt_broke[0].strip())
            v.failed_early = True
        elif fmt_exit not in (0, 1):
            v.add_gate('V3 lint', 'FAIL', f"cargo fmt exit {fmt_exit}; see out.txt")
            v.failed_early = True
        elif fmt_findings:
            v.add_gate('V3 lint', 'FAIL',
                       "cargo fmt --check wants changes in files this packet changed: "
                       + ', '.join(sorted(set(fmt_findings))[:5]))
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
            # --no-deps is load-bearing, not tidiness. The vendored crates are
            # workspace path dependencies, so without it clippy lints them too
            # -- and truck-meshalgo fails with 93 denied lints, which means
            # cargo gives up before it ever reaches the packet's own crate.
            # BG-S0-002 passed V3 that way: the run died in truck-modeling and
            # truck-geometry, truck-shapeops was never linted at all, and an
            # unused import the diff introduced went unseen until an editor
            # pointed at it. This is the same shape as V5's fail-fast bug --
            # a gate reporting PASS on something it never looked at.
            v.write_out_section('V3 lint: cargo clippy (diff-scoped, --no-deps)')
            len_before = v.out_file.stat().st_size
            v.invoke_native(['cargo', 'clippy', *p_args, '--all-targets',
                             '--message-format=short', '--no-deps'], v.wt)

            clippy_text = v.out_file.read_bytes()[len_before:].decode('utf-8', errors='replace')

            # Coverage, checked before findings: if clippy could not build a
            # crate this packet owns, "no findings" means "nothing was looked
            # at". A gate that cannot see must not report PASS.
            unlinted = [c for c in crate_names
                        if re.search(r'could not compile `' + re.escape(c) + '`', clippy_text)]
            # A packet may also change a file in a crate it did not name, which
            # -p never reaches. V1 allows it, so V3 has to notice it.
            changed_crates = set()
            for f in changed:
                cm = re.match(r'vendor/truck/([^/]+)/', f.replace('\\', '/'))
                if cm:
                    changed_crates.add(cm.group(1))
            unnamed = sorted(changed_crates - set(crate_names))

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

            if unlinted or unnamed:
                why = []
                if unlinted:
                    why.append("clippy could not build " + ', '.join(unlinted)
                               + ", so findings in it were never produced")
                if unnamed:
                    why.append("packet changed " + ', '.join(unnamed)
                               + " but does not name it in `crates`, so clippy never linted it")
                v.add_gate('V3 lint', 'FAIL', '; '.join(why))
                v.failed_early = True
            elif our_findings:
                v.add_gate('V3 lint', 'FAIL', "clippy findings in changed files: " + ' ; '.join(our_findings[:5]))
                v.failed_early = True
            else:
                v.add_gate('V3 lint', 'PASS',
                           f"fmt clean in all {len(changed)} changed file(s); "
                           f"{', '.join(crate_names)} linted, no finding on any added line")
    else:
        v.add_gate('V3 lint', 'SKIP', 'earlier gate failed')

    # -----------------------------------------------------------------------
    # V4 house rules — scripts/kernel-gates.sh <base>, diff-scoped, H-1/H-3/H-4.
    # -----------------------------------------------------------------------
    if not gate_wanted('V4'):
        skip_not_requested('V4 house rules')
    elif not v.failed_early:
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
    # V5 tests — cargo test -p <crate> --lib --tests --no-fail-fast, compared
    # against a real baseline run at `base`. Never bare `cargo test`.
    #
    # The whole-crate `--lib --tests` form catches pre-existing baseline
    # failures the packet never touched -- truck-shapeops's
    # healing::tests::step_import needs a STEP data file absent on this
    # machine, and tests/fillet.rs::complex_surface triangulates to
    # ShellCondition::Irregular -- and cargo's default fail-fast stops at the
    # first failing binary before reaching the packet's own tests/*.rs (that
    # is how BG-S0-002's first verify ran every crate except fillet.rs).
    # --no-fail-fast runs every binary.
    #
    # This used to scope the FAIL decision to test fns the packet *added*,
    # by analogy with V3's added-lines clippy scoping. That analogy doesn't
    # hold: a clippy finding carries a file:line, so scoping to added lines
    # filters noise precisely; a cargo test failure carries only a test name,
    # which is not "located" anywhere in the diff, so scoping to added tests
    # doesn't filter noise -- it throws away every regression a packet causes
    # in a test it didn't write. That gap was handed to V8, a stub that
    # always passes, i.e. handed to nobody.
    #
    # The real baseline comparison: run the same command at `base` once (see
    # load_or_compute_baseline / compute_baseline above), cache the failing
    # test names per (base, crate set) under loop/baselines/ so repeated
    # packets against the same base don't pay for it again, and then apply
    # three rules to this run's result:
    #   - failed now, not failed (or absent) at base -> regression, FAIL.
    #     Covers both "packet broke an existing test" and "packet's own new
    #     test fails" -- the latter is just the base-absent case.
    #   - failed at base and still fails -> pre-existing baseline noise,
    #     reported but does not reject (unchanged from before).
    #   - passed at base and is absent from this run's output entirely, or
    #     present but now `ignored` -> also a regression (deleted or
    #     #[ignore]d to get green, which the house rules forbid).
    # Known blind spot: a test renamed (same behaviour, new name) is
    # indistinguishable from delete-old-add-new by this method and will read
    # as one disappearance + one new-test-pass; there is no reliable way to
    # tell those apart from cargo's text output alone, so this is a false
    # positive this gate can produce, not a case it silently misses.
    # -----------------------------------------------------------------------
    if not gate_wanted('V5'):
        skip_not_requested('V5 tests')
    elif not v.failed_early:
        baseline = load_or_compute_baseline(base, crate_names, v.out_file)

        v.write_out_section('V5 tests: cargo test -p --lib --tests --no-fail-fast')
        len_before = v.out_file.stat().st_size
        v.invoke_native(['cargo', 'test', *p_args, '--lib', '--tests', '--no-fail-fast'], v.wt)
        chunk = v.out_file.read_bytes()[len_before:].decode('utf-8', errors='replace')

        compile_error = has_compile_error(chunk)

        if compile_error:
            v.add_gate('V5 tests', 'FAIL', 'test target(s) failed to compile; see out.txt')
            v.failed_early = True
        elif not baseline['compile_ok']:
            # The baseline itself didn't compile -- we have no reliable
            # failing-set to diff against. Conservative direction: treat
            # every failure now as unexplained rather than silently passing.
            now = parse_test_statuses(chunk)
            failing_now = sorted(n for n, s in now.items() if s == 'FAILED')
            if failing_now:
                v.add_gate('V5 tests', 'FAIL',
                            f"baseline at {base[:7]} would not compile, so no comparison is possible; "
                            'failing test(s) now: ' + ', '.join(failing_now[:8]) + '; see out.txt')
                v.failed_early = True
            else:
                v.add_gate('V5 tests', 'PASS',
                            f"baseline at {base[:7]} would not compile (no comparison possible); "
                            'no test failures in this run')
        else:
            now = parse_test_statuses(chunk)
            base_tests = baseline['tests']

            newly_failing = sorted(n for n, s in now.items()
                                    if s == 'FAILED' and base_tests.get(n) != 'FAILED')
            still_failing = sorted(n for n, s in now.items()
                                    if s == 'FAILED' and base_tests.get(n) == 'FAILED')
            disappeared = sorted(n for n, s in base_tests.items()
                                  if s == 'ok' and n not in now)
            newly_ignored = sorted(n for n, s in now.items()
                                    if s == 'ignored' and base_tests.get(n) == 'ok')

            regressions = newly_failing + disappeared + newly_ignored

            if regressions:
                parts = []
                if newly_failing:
                    parts.append('newly failing: ' + ', '.join(newly_failing[:8]))
                if disappeared:
                    parts.append('passed at base, absent now (deleted?): ' + ', '.join(disappeared[:8]))
                if newly_ignored:
                    parts.append('passed at base, #[ignore]d now: ' + ', '.join(newly_ignored[:8]))
                v.add_gate('V5 tests', 'FAIL', '; '.join(parts) + '; see out.txt')
                v.failed_early = True
            else:
                detail = f"no regressions vs baseline at {base[:7]}"
                if still_failing:
                    detail += (f"; {len(still_failing)} baseline failure(s) ignored (failed at base too): "
                               + ', '.join(still_failing[:8]))
                v.add_gate('V5 tests', 'PASS', detail)
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
    if not gate_wanted('V6'):
        skip_not_requested('V6 test-reality')
    elif not v.failed_early:
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
    if gate_wanted('V7'):
        v.add_gate('V7 mutation spot-check', 'PASS', 'TODO stub — always passes; see comment in verify.py')
    else:
        skip_not_requested('V7 mutation spot-check')

    # -----------------------------------------------------------------------
    # V8 no-regression — TODO. Per §5: the previously accepted wave's tests still
    # pass. Not implemented: this needs LEDGER.jsonl / PACKETS.jsonl to know what
    # "the previous wave" is and which crates it touched, which is orchestrator
    # state that doesn't exist yet. Always passes until then.
    # -----------------------------------------------------------------------
    if gate_wanted('V8'):
        v.add_gate('V8 no-regression', 'PASS', 'TODO stub — always passes; see comment in verify.py')
    else:
        skip_not_requested('V8 no-regression')

    # -----------------------------------------------------------------------
    # Verdict
    #
    # --only forces PARTIAL no matter what the requested gates found. This is
    # deliberate and not just a default: the whole point of the flag is to
    # make re-checking one gate cheap, and the moment a partial run could
    # report ACCEPTED, someone will land a packet on the strength of "V3
    # passed" without V2/V5/the rest having been re-run against the amended
    # commit. Exit code 3 (distinct from 0/1/2) so a caller can't mistake it
    # for any full-run outcome by accident.
    # -----------------------------------------------------------------------
    if only_gates is not None:
        print()
        print(f"VERDICT: PARTIAL (--only {args.only} -- not a substitute for a full run)")
        print(f"packet={args.packet} slot={v.slot} crates={','.join(crate_names)} base={base}")

        verdict = {
            'packet': args.packet,
            'slot': v.slot,
            'crates': crate_names,
            'base': base,
            'branch': branch,
            'commit': commit_sha,
            'amended_by': amended_by,
            'verdict': 'PARTIAL',
            'only': sorted(only_gates),
            'gates': v.gates,
            'timestamp': datetime.datetime.now(datetime.timezone.utc).isoformat().replace('+00:00', 'Z'),
        }
        v.verdict_file.write_text(json.dumps(verdict, indent=4), encoding='utf-8')
        sys.exit(3)

    accepted = not any(g['status'] == 'FAIL' for g in v.gates)

    print()
    print(f"VERDICT: {'ACCEPTED' if accepted else 'REJECTED'}")
    print(f"packet={args.packet} slot={v.slot} crates={','.join(crate_names)} base={base}")

    verdict = {
        'packet': args.packet,
        'slot': v.slot,
        'crates': crate_names,
        'base': base,
        'branch': branch,
        'commit': commit_sha,
        'amended_by': amended_by,
        'verdict': 'ACCEPTED' if accepted else 'REJECTED',
        'gates': v.gates,
        'timestamp': datetime.datetime.now(datetime.timezone.utc).isoformat().replace('+00:00', 'Z'),
    }
    v.verdict_file.write_text(json.dumps(verdict, indent=4), encoding='utf-8')

    sys.exit(0 if accepted else 1)


if __name__ == '__main__':
    main()
