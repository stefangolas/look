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
import atexit
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


def reverse_dep_closure(wt, crate_names):
    """Vendored crates that transitively depend on any of `crate_names`.

    Read off the Cargo.toml files rather than `cargo metadata`: metadata needs
    a resolve, which needs the network on a cold slot, and this has to be
    cheap enough to run on every verify. A dependency here is a line naming
    another vendored crate inside a `[*dependencies]` table -- including
    dev-dependencies, because a dev-dependency is exactly how a downstream
    crate's TESTS reach the code this packet changed, and tests are what V8
    exists to run.

    The packet's own crates are excluded from the result: V5 runs those.
    """
    root = Path(wt) / 'vendor' / 'truck'
    if not root.is_dir():
        return set()
    names = {p.parent.name for p in root.glob('*/Cargo.toml')}
    deps = {n: set() for n in names}
    for n in names:
        section = None
        for line in (root / n / 'Cargo.toml').read_text(encoding='utf-8', errors='replace').splitlines():
            s = line.strip()
            if s.startswith('['):
                section = s
                continue
            if not section or 'dependencies' not in section:
                continue
            m = re.match(r'([A-Za-z0-9_-]+)\s*=', s)
            if m and m.group(1) in names and m.group(1) != n:
                deps[n].add(m.group(1))
    # Transitive closure, upward: who reaches the seed set.
    out, frontier = set(), set(crate_names)
    while frontier:
        nxt = set()
        for n in names:
            if n in out or n in crate_names:
                continue
            if deps.get(n, set()) & frontier:
                nxt.add(n)
        out |= nxt
        frontier = nxt
    return out - set(crate_names)


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


DEFAULT_TEST_ARGS = ['--lib', '--tests']

# A cargo `Running <target source path> (<test exe>)` banner. Everything
# printed after one of these, until the next, belongs to that test target.
_RUNNING_RE = re.compile(r'^\s+Running\s+(.+?)\s+\((.+?)\)\s*$')
_DOCTEST_RE = re.compile(r'^\s+Doc-tests\s+\S+')
# libtest's own header for one test binary's run, on stdout.
_RUNNING_COUNT_RE = re.compile(r'^running (\d+) tests?$')


def _target_of_running(desc, exe, wt, crates):
    """`(crate, target_flags)` for one cargo Running banner, or (None, None).

    Attribution has to be exact, because it decides what V8 compiles at the
    base commit: the whole point of the narrow base query is to build one
    crate's one test target instead of the entire downstream workspace. Where
    it cannot be made exact the caller falls back to the wide query rather
    than guessing -- a mis-attributed test would be measured against the wrong
    crate's base and could acquit a real regression.

    The banner names the target path relative to its *package* directory, not
    the workspace, so `tests\\geometry.rs` alone does not say whose it is. The
    package is recovered from the filesystem instead: exactly one downstream
    crate must own a file at that path.
    """
    root = Path(wt) / 'vendor' / 'truck'
    desc = desc.replace('\\', '/')
    if desc.startswith('unittests'):
        # `unittests src/lib.rs (target/debug/deps/truck_stepio-HASH.exe)`.
        # The lib test binary is named after the package with dashes turned
        # into underscores, which is unambiguous across the workspace.
        stem = Path(exe).name.rsplit('-', 1)[0]
        for c in crates:
            if c.replace('-', '_') == stem:
                return c, ('--lib',) if desc.endswith('lib.rs') else ('--bin', stem)
        return None, None
    m = re.match(r'tests/([^/]+?)(?:\.rs|/main\.rs)$', desc)
    if not m:
        return None, None
    name = m.group(1)
    owners = [c for c in crates
              if (root / c / 'tests' / f'{name}.rs').is_file()
              or (root / c / 'tests' / name / 'main.rs').is_file()]
    if len(owners) != 1:
        return None, None
    return owners[0], ('--test', name)


def attribute_tests(stdout, stderr, wt, crates):
    """{test name: {'status', 'crate', 'target'}} for one cargo test run.

    `parse_test_statuses` answers "what happened"; this also answers "where",
    which is what lets V8 ask the base commit about one test target instead of
    rebuilding every downstream crate to re-derive an answer it already has
    for all the passing ones.

    The two streams cannot be interleaved without corrupting lines (see
    `invoke_native`), so they are matched by POSITION instead: cargo runs test
    binaries one after another, so the Nth `Running <target>` banner on stderr
    is the Nth `running N tests` block on stdout.

    That alignment is checked, not assumed. If the counts disagree, or a
    block's declared test count does not match the number of test lines read
    out of it, attribution for the affected block is dropped to (None, None)
    and the caller falls back to the wide base query. Guessing here would be
    worse than being slow: a test attributed to the wrong target is measured
    against a base that has never heard of it, comes back 'absent', and gets
    charged to the packet as a regression it did not cause.
    """
    targets = [_target_of_running(m.group(1), m.group(2), wt, crates)
               for m in (_RUNNING_RE.match(l) for l in stderr.splitlines()) if m]

    blocks, cur = [], None
    for line in stdout.splitlines():
        m = _RUNNING_COUNT_RE.match(line)
        if m:
            cur = {'declared': int(m.group(1)), 'tests': []}
            blocks.append(cur)
            continue
        m = _TEST_LINE_RE.match(line)
        if m and cur is not None:
            cur['tests'].append((m.group(1), m.group(2)))

    aligned = len(blocks) == len(targets)
    out = {}
    for i, b in enumerate(blocks):
        crate, target = (targets[i] if aligned else (None, None))
        if len(b['tests']) != b['declared']:
            # A line was lost or invented in this block -- do not trust its
            # position either.
            crate, target = None, None
        for name, status in b['tests']:
            out[name] = {'status': status, 'crate': crate, 'target': target}
    return out


def observation_key(crate, target, test_name):
    """Cache key for one base observation. The crate and target are part of
    it because a bare `module::name` is not unique across test binaries, and
    because a re-attributed test must be re-measured rather than served an
    answer recorded against a different target."""
    return f"{crate or '?'}|{'/'.join(target) if target else '?'}|{test_name}"


def v8_observations_path(base_sha):
    return BASELINES_DIR / f"{base_sha[:12]}__v8-observations.json"


def load_v8_observations(base_sha):
    p = v8_observations_path(base_sha)
    if not p.is_file():
        return {}
    try:
        return json.loads(p.read_text(encoding='utf-8')).get('observations', {})
    except (json.JSONDecodeError, OSError):
        return {}


def save_v8_observations(base_sha, observations):
    BASELINES_DIR.mkdir(parents=True, exist_ok=True)
    v8_observations_path(base_sha).write_text(json.dumps({
        'base': base_sha,
        'computed_at': datetime.datetime.now(datetime.timezone.utc).isoformat().replace('+00:00', 'Z'),
        'observations': observations,
    }, indent=2, sort_keys=True), encoding='utf-8')


# The narrow base query compiles one crate and its dependencies, not the whole
# downstream workspace, so it needs nowhere near what `compute_baseline` does.
# The floor is still real -- an ENOSPC part-way through a build is what
# corrupts a Cargo target dir -- just proportional to the work.
V8_QUERY_FLOOR_GB = 4.0


def query_base_tests(base_sha, groups, out_file, wide_args):
    """Run only the named tests at `base_sha`; return (statuses, unavailable).

    `groups` maps `(crate, target_flags)` -> [test names]. Each group becomes
    one `cargo test -p <crate> <target> -- --exact <names...>` inside a single
    throwaway worktree, so the base side builds only what the failing tests
    actually live in. A group keyed `(None, None)` could not be attributed and
    falls back to `wide_args` (the full downstream crate set), which is the
    old behaviour, kept as the escape hatch rather than the default path.

    `statuses` maps observation key -> 'ok' | 'FAILED' | 'ignored' | 'absent'.
    'absent' means the target built and ran but has no test by that name --
    a real fact about the base, distinct from "we could not look".

    `unavailable` lists the groups whose build failed. A base build failure is
    an environment fact and never the packet's doing (the packet cannot break
    a commit that predates it), so its output is returned separately and is
    never written to the observation cache.
    """
    free_gb = shutil.disk_usage(str(REPO_ROOT.anchor or 'C:\\')).free / 2**30
    leaked = leaked_baselines()
    if free_gb < V8_QUERY_FLOOR_GB:
        raise RuntimeError(
            f"refusing to query the base: {free_gb:.1f} GB free, below the "
            f"{V8_QUERY_FLOOR_GB:.1f} GB floor for a narrow base build.\n"
            + (f"  {len(leaked)} leaked baseline worktree(s) are holding "
               f"{sum(g for _, g in leaked):.1f} GB: "
               + ', '.join(p.name for p, _ in leaked) + "\n" if leaked else "")
            + "Reclaim %TEMP%/look-verify-baseline-* first and `git worktree prune`; "
            "the slot's own target is warm and rebuilding it costs more than it frees."
        )

    tmp_parent = Path(tempfile.mkdtemp(prefix='look-verify-baseline-'))
    wt_path = tmp_parent / 'wt'
    target_path = tmp_parent / 'target'

    with out_file.open('a', encoding='utf-8', newline='\n') as f:
        f.write(f"\n===== V8 base query: {sum(len(t) for t in groups.values())} test(s) in "
                f"{len(groups)} target(s) at {base_sha[:12]} (worktree {wt_path}) =====\n")

    add_res = subprocess.run(
        ['git', '-C', str(REPO_ROOT), 'worktree', 'add', '--detach', str(wt_path), base_sha],
        capture_output=True, text=True, encoding='utf-8', errors='replace'
    )
    with out_file.open('a', encoding='utf-8', newline='\n') as f:
        f.write(add_res.stdout)
        f.write(add_res.stderr)
    if add_res.returncode != 0:
        shutil.rmtree(tmp_parent, ignore_errors=True)
        raise RuntimeError(f"could not create base worktree at {base_sha}: {add_res.stderr}")

    statuses, unavailable = {}, []
    try:
        env = dict(os.environ)
        env['CARGO_INCREMENTAL'] = '0'
        env['CARGO_TARGET_DIR'] = str(target_path)
        for (crate, target), names in sorted(groups.items(), key=lambda kv: str(kv[0])):
            if crate is None:
                scope = list(wide_args) + list(DEFAULT_TEST_ARGS)
            else:
                scope = ['-p', crate] + list(target)
            cmd = ['cargo', 'test', *scope, '--no-fail-fast', '--', '--exact', *sorted(names)]
            with out_file.open('a', encoding='utf-8', newline='\n') as f:
                f.write('\n$ ' + ' '.join(cmd) + '\n')
            # Streams kept apart: a test's stray stderr write would otherwise
            # land inside a `test name ... ok` line and lose it. Statuses come
            # off stdout; only compile errors need stderr.
            res = subprocess.run(cmd, cwd=str(wt_path), capture_output=True, text=True,
                                 encoding='utf-8', errors='replace', env=env)
            chunk = res.stdout + res.stderr
            with out_file.open('a', encoding='utf-8', newline='\n') as f:
                f.write(chunk)
            if has_compile_error(chunk):
                unavailable.append((crate, target))
                continue
            seen = parse_test_statuses(res.stdout)
            for n in names:
                statuses[observation_key(crate, target, n)] = seen.get(n, 'absent')
        return statuses, unavailable
    finally:
        subprocess.run(['git', '-C', str(REPO_ROOT), 'worktree', 'remove', '--force', str(wt_path)],
                        capture_output=True, text=True, encoding='utf-8', errors='replace')
        shutil.rmtree(tmp_parent, ignore_errors=True)


def baseline_cache_path(base_sha, crate_names, test_args=None):
    key = base_sha[:12] + '__' + '-'.join(sorted(crate_names))
    # The target selection is part of the cache identity: V5 and V9 both key on
    # the same base and would otherwise share a file while measuring different
    # test sets, so one would silently serve the other's answer.
    if test_args and test_args != DEFAULT_TEST_ARGS:
        key += '__' + '-'.join(a for a in test_args if a != '--test')
    safe_key = re.sub(r'[^A-Za-z0-9_.-]', '_', key)
    return BASELINES_DIR / f"{safe_key}.json"


def leaked_baselines():
    """[(path, size_gb)] for baseline worktrees a killed verify left behind.

    `compute_baseline` removes its own temp dir on the way out, which does not
    happen when the process is killed. Each one is ~2.5 GB and they are
    invisible to every disk recipe in these docs, which name loop/slots/*.
    """
    out = []
    for p in Path(tempfile.gettempdir()).glob('look-verify-baseline-*'):
        if not p.is_dir():
            continue
        try:
            size = sum(f.stat().st_size for f in p.rglob('*') if f.is_file())
        except OSError:
            size = 0
        out.append((p, size / 2**30))
    return out


def compute_baseline(base_sha, crate_names, out_file, test_args=None):
    """Run `cargo test -p <crates> --lib --tests --no-fail-fast` at base_sha
    in a throwaway worktree and return {'compile_ok': bool, 'tests': {name:
    status}}. Never touches loop/slots/*; creates and removes its own
    worktree and target dir under the system temp directory, so a baseline
    run cannot collide with (or be mistaken for) a slot's own worktree."""
    # Same floor, and the same "refuse, don't flag" rule, as new_slot.py -- which
    # had it and was not the problem. A baseline builds an entire extra
    # workspace in a throwaway worktree, and nothing here checked disk before
    # doing it. Session 6 ran baselines against three different base commits in
    # an hour and took the machine from 40 GB free to 0.1 GB. Failing before
    # touching disk is strictly better than failing partway through a 4-minute
    # build and leaving the debris behind.
    leaked = leaked_baselines()
    free_gb = shutil.disk_usage(str(REPO_ROOT.anchor or 'C:\\')).free / 2**30
    if free_gb < 8.0:
        raise RuntimeError(
            f"refusing to compute a baseline: {free_gb:.1f} GB free, below the 8.0 GB floor.\n"
            + (f"  {len(leaked)} leaked baseline worktree(s) are holding "
               f"{sum(g for _, g in leaked):.1f} GB: "
               + ', '.join(p.name for p, _ in leaked) + "\n" if leaked else "")
            + "Delete loop/slots/*/target AND loop/slots/*/wt/target -- workers create a "
            "second target INSIDE the worktree despite CARGO_TARGET_DIR, and it is the "
            "bigger of the two -- plus any %TEMP%/look-verify-baseline-*, then "
            "`git worktree prune`. A slot re-warms in 1-3 min."
        )
    if leaked:
        # An interrupted verify does not run this function's cleanup, so its
        # worktree stays. Two leaked baselines plus one live one took session 9
        # from 9.4 GB to 3.1 GB and cost it a run. Warn while there is still
        # room to act, not only at the floor.
        print(f"WARNING: {len(leaked)} leaked baseline worktree(s) holding "
              f"{sum(g for _, g in leaked):.1f} GB from an interrupted verify: "
              + ', '.join(p.name for p, _ in leaked), file=sys.stderr)

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
            ['cargo', 'test', *p_args, *(test_args or DEFAULT_TEST_ARGS), '--no-fail-fast'],
            cwd=str(wt_path), capture_output=True, text=True,
            encoding='utf-8', errors='replace', env=env
        )
        chunk = test_res.stdout + test_res.stderr
        with out_file.open('a', encoding='utf-8', newline='\n') as f:
            f.write(chunk)

        compile_ok = not has_compile_error(chunk)
        tests = parse_test_statuses(test_res.stdout) if compile_ok else {}
        return {'compile_ok': compile_ok, 'tests': tests}
    finally:
        # Best-effort cleanup. `git worktree remove` first (keeps the repo's
        # worktree list clean); if the target dir is still holding a file
        # open on Windows, at least the git-level registration is gone and a
        # leftover tmp_parent under the OS temp dir is harmless.
        subprocess.run(['git', '-C', str(REPO_ROOT), 'worktree', 'remove', '--force', str(wt_path)],
                        capture_output=True, text=True, encoding='utf-8', errors='replace')
        shutil.rmtree(tmp_parent, ignore_errors=True)


def load_or_compute_baseline(base_sha, crate_names, out_file, test_args=None):
    cache_path = baseline_cache_path(base_sha, crate_names, test_args)
    if cache_path.is_file():
        obj = json.loads(cache_path.read_text(encoding='utf-8'))
        with out_file.open('a', encoding='utf-8', newline='\n') as f:
            f.write(f"\n===== baseline: loaded cache {cache_path.name} "
                     f"(computed {obj.get('computed_at', '?')}) =====\n")
        return obj

    BASELINES_DIR.mkdir(parents=True, exist_ok=True)
    result = compute_baseline(base_sha, crate_names, out_file, test_args)
    obj = {
        'base': base_sha,
        'crates': sorted(crate_names),
        'test_args': test_args or DEFAULT_TEST_ARGS,
        'computed_at': datetime.datetime.now(datetime.timezone.utc).isoformat().replace('+00:00', 'Z'),
        'compile_ok': result['compile_ok'],
        'tests': result['tests'],
    }
    # Never cache evidence from a build that did not compile. A baseline whose
    # cargo run died on `error[E0786] invalid metadata` or a half-written
    # .fingerprint is not a measurement of the base commit, it is a
    # measurement of the disk -- and once written it is indistinguishable from
    # a real one and is trusted by every later verify against that base. The
    # 2026-08-19 ENUM-r3 session lost hours to exactly one such file. Recompute
    # is cheap; a poisoned baseline is not.
    if result['compile_ok']:
        cache_path.write_text(json.dumps(obj, indent=2, sort_keys=True), encoding='utf-8')
    else:
        with out_file.open('a', encoding='utf-8', newline='\n') as f:
            f.write(f"\n===== baseline at {base_sha[:12]} did not compile: NOT cached =====\n")
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

    # -- liveness ------------------------------------------------------------

    def claim_slot(self):
        """Announce, in the slot itself, that a verify is building here.

        The slot's `worker.pid` goes away the moment the worker finishes, so
        anything reading only that concludes the slot is idle -- while verify
        is midway through a multi-gigabyte build in `target/`. On 2026-08-19
        `loop/watchdog.py` did exactly that and `rmtree`d
        `loop/slots/0/target` under a live cargo, three times; the resulting
        `error[E0786] found invalid metadata files` and `failed to write
        .fingerprint` were read as code regressions and cost the session its
        evening. A pid file the reaper can check is the fix.
        """
        self.pid_file = self.slot_root / 'verify.pid'
        try:
            self.pid_file.write_text(str(os.getpid()), encoding='utf-8')
            atexit.register(self.release_slot)
        except OSError:
            self.pid_file = None

    def release_slot(self):
        try:
            if self.pid_file and self.pid_file.is_file():
                self.pid_file.unlink()
        except OSError:
            pass

    # -- output capture -----------------------------------------------------

    def write_out_section(self, header):
        with self.out_file.open('a', encoding='utf-8', newline='\n') as f:
            f.write(f"\n===== {header} =====\n")

    def invoke_native(self, args, cwd):
        """Run a native command, appending its output to out.txt, and return
        its exit code. subprocess.run's own stdout/stderr capture (rather than
        PowerShell's stream redirection) has none of the ErrorRecord-wrapping
        hazard the PS1 comment describes, so this is a plain run-and-append.

        The two streams are captured SEPARATELY and kept that way in
        `last_stdout` / `last_stderr`, because callers need two properties and
        no single capture gives both:

        - Merging them (`stderr=subprocess.STDOUT`) keeps the order but not
          the lines. A test that writes to the process's stderr from outside
          libtest's capture splices its text into the middle of libtest's own
          stdout line -- `test foo ... ****** banner ******` -- and that test
          then parses as absent. Tried on 2026-08-20; it made V5 reject r3 for
          "deleting" decorators::processor::hint_axis_tests::
          upright_processor_forwards_the_hint_unchanged, a truck-geometry test
          the packet never touched.
        - Concatenating them keeps the lines but not the order, and cargo
          splits test output across both streams: the `Running <target>
          (<exe>)` banners go to stderr, the `test name ... ok` lines to
          stdout.

        So both are kept, and `attribute_tests` lines them up by position
        rather than by interleaving.
        """
        res = subprocess.run(args, cwd=str(cwd), capture_output=True, text=True,
                             encoding='utf-8', errors='replace')
        self.last_stdout, self.last_stderr = res.stdout, res.stderr
        with self.out_file.open('a', encoding='utf-8', newline='\n') as f:
            f.write(res.stdout)
            f.write(res.stderr)
        return res.returncode

    def add_gate(self, name, status, detail='', code=''):
        """Record a gate outcome.

        `code` is a machine-readable reason, `detail` is the sentence a human
        reads. Both, not one: the code is what an orchestrator branches on and
        the detail is what tells it why.

        The code exists because triaging one V3 rejection cost six tool calls --
        read VERDICT.json, grep out.txt, extract the diff hunk ranges, check
        whether the flagged line was added, read lib.rs for a deny attribute,
        count error[E####] -- to arrive at a single fact: this was a lint abort,
        not a build failure, and therefore the gate's fault rather than the
        worker's. `LINT_ABORT` says that in one field. The rule that decides
        what gets a code is: code what is branched on or counted, keep prose for
        what is reasoned about.
        """
        row = {'name': name, 'status': status, 'detail': detail}
        if code:
            row['code'] = code
        self.gates.append(row)
        print(f"{name} ... {status}" + (f"  [{code}]" if code else ''))


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

    m_class = re.search(r"(?m)^class:\s*(\S+)", yaml_text)
    return {
        'crates': yaml_list_field(yaml_text, 'crates'),
        'write_allow': yaml_list_field(yaml_text, 'write_allow'),
        'tests_required': yaml_list_field(yaml_text, 'tests_required'),
        'class': m_class.group(1).strip() if m_class else 'mechanical',
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
        valid_ids = {f'V{n}' for n in range(1, 10)}
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

    v.claim_slot()
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
    # The paragraph above said the next artifact would show up as a false
    # BLOCKED and get added with evidence. BG-TOL-001-TOPO-MOD is that case:
    # `vendor/truck/truck-stepio/tests/proptest-regressions/geometry.txt`,
    # written by proptest when a property test fails. The worker hit it running
    # the baseline suite this packet asked it to confirm, and the failure was
    # one of truck-stepio's *pre-existing* ones -- nothing to do with its work.
    #
    # Worth knowing before removing this: proptest's own header recommends
    # committing these files, and in a normal crate that is right. Here it is
    # not, because the tree does not track a single one (`git ls-files` finds
    # none) and the seed came from a failure the packet neither caused nor is
    # allowed to fix.
    def _ignorable_untracked(rel_path):
        name = Path(rel_path).name
        if name in ('PACKET.md', 'CONTEXT.md', 'RESULT.json', 'QUESTION.md'):
            return True
        if rel_path.endswith('.obj'):
            return True
        if 'proptest-regressions' in rel_path.replace('\\', '/').split('/'):
            return True
        # ...and the same artifact under its other name. proptest's
        # FileFailurePersistence::SourceParallel wants the seed in a
        # `proptest-regressions/` directory beside the source root, and when it
        # cannot find lib.rs or main.rs to locate that root it falls back to a
        # sibling file, `<test>.proptest-regressions`. truck-geometry's
        # tests/bspcurve.rs takes the fallback, so the directory check above
        # never matched: one flaky randomized failure then BLOCKED every later
        # verify on that slot, including the re-run measuring whether the
        # failure was real.
        if name.endswith('.proptest-regressions'):
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
        v.add_gate('V0 preflight', 'FAIL', '; '.join(preflight), code='RUN_INCOMPLETE')
        for n in ('V1 scope', 'V2 build', 'V3 lint', 'V4 house rules', 'V5 tests',
                  'V6 test-reality', 'V9 geometry'):
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
        v.add_gate('V1 scope', 'FAIL', "out-of-allowlist paths: " + ', '.join(offenders), code='SCOPE_VIOLATION')
        v.failed_early = True
    else:
        v.add_gate('V1 scope', 'PASS', f"{len(changed)} changed file(s), all within write_allow")

    # -----------------------------------------------------------------------
    # V10 survey shape — the acceptance gate for `class: survey`.
    #
    # A survey packet delegates the expensive half of writing a shard: reading
    # every call site and proposing a classification for it. Its deliverable is
    # SURVEY.json, a *proposal*, and it gets no write access to vendor/truck at
    # all -- so nothing it says can reach the kernel except through a packet the
    # orchestrator writes afterwards.
    #
    # A judgement cannot be graded mechanically. Its ANCHORS can, and that is
    # exactly the half that has gone wrong twice: BG-TOL-001-GEOM-SPECIFIEDS
    # shipped with three of seven anchor counts wrong, and the SHAPEOPS site
    # table listed a line inside a /* */ block. So this gate checks that every
    # (file, symbol, line) the survey names actually exists and that the quoted
    # expression is really on that line, and leaves the classification to the
    # orchestrator's review. A survey whose sites are real is cheap to review;
    # one whose sites are invented is worse than nothing, because it reads as
    # authoritative.
    # -----------------------------------------------------------------------
    if pkt['class'] == 'survey':
        if not gate_wanted('V10'):
            skip_not_requested('V10 survey shape')
        else:
            survey_path = v.wt / 'SURVEY.json'
            problems = []
            rows = []
            if not survey_path.is_file():
                problems.append('no SURVEY.json in the worktree root')
            else:
                try:
                    doc = json.loads(survey_path.read_text(encoding='utf-8'))
                    rows = doc['sites'] if isinstance(doc, dict) else doc
                    if not isinstance(rows, list) or not rows:
                        problems.append('SURVEY.json holds no sites')
                        rows = []
                except (json.JSONDecodeError, OSError, KeyError, TypeError) as e:
                    problems.append(f'SURVEY.json does not parse as expected: {e}')
                    rows = []

            required = ('file', 'line', 'symbol', 'expression', 'classification', 'reason')
            seen = 0
            for i, r in enumerate(rows):
                if not isinstance(r, dict):
                    problems.append(f'site {i} is not an object')
                    continue
                missing = [k for k in required if k not in r]
                if missing:
                    problems.append(f"site {i} missing {','.join(missing)}")
                    continue
                if r['classification'] not in ('model', 'param', 'excluded'):
                    problems.append(f"site {i} classification {r['classification']!r} "
                                    "is not model|param|excluded")
                src = v.wt / str(r['file']).replace('\\', '/')
                if not src.is_file():
                    problems.append(f"site {i} names a file that does not exist: {r['file']}")
                    continue
                try:
                    lines = src.read_text(encoding='utf-8', errors='replace').splitlines()
                except OSError as e:
                    problems.append(f"site {i} file unreadable: {e}")
                    continue
                ln = r['line']
                if not isinstance(ln, int) or ln < 1 or ln > len(lines):
                    problems.append(f"site {i} line {ln} is outside {r['file']} (1..{len(lines)})")
                    continue
                # The expression has to be ON the line the survey claims. A
                # fragment is enough -- whitespace and trailing comments differ
                # -- but an invented line number will not match.
                frag = ' '.join(str(r['expression']).split())[:40]
                hay = ' '.join(lines[ln - 1].split())
                if frag and frag not in hay:
                    problems.append(f"site {i} expression not on {r['file']}:{ln} "
                                    f"(line reads {hay[:60]!r})")
                    continue
                seen += 1

            if problems:
                v.add_gate('V10 survey shape', 'FAIL',
                           f'{len(problems)} problem(s): ' + '; '.join(problems[:6]))
            else:
                v.add_gate('V10 survey shape', 'PASS',
                           f'{seen} site(s), every file/line/expression verified against the tree; '
                           'classifications are a proposal and are NOT verified here')
    else:
        v.add_gate('V10 survey shape', 'SKIP', 'not a survey packet')

    # A survey commits no Rust, so every gate that shells out to cargo would be
    # measuring an empty diff. Marking them SKIP is honest; running them would
    # manufacture a PASS that means nothing, which is the V7/V8 mistake.
    if pkt['class'] == 'survey':
        for n in ('V2 build', 'V3 lint', 'V4 house rules', 'V5 tests',
                  'V6 test-reality', 'V7 mutation spot-check', 'V8 no-regression',
                  'V9 geometry'):
            v.add_gate(n, 'SKIP', 'survey packet — no kernel code to build or test')
        verdict_name = 'ACCEPTED' if all(
            g['status'] in ('PASS', 'SKIP') for g in v.gates) else 'REJECTED'
        print()
        print(f'VERDICT: {verdict_name}')
        verdict = {
            'packet': args.packet, 'slot': v.slot, 'crates': crate_names, 'base': base,
            'branch': branch, 'commit': commit_sha, 'amended_by': amended_by,
            'class': 'survey', 'verdict': verdict_name, 'gates': v.gates,
            'timestamp': datetime.datetime.now(datetime.timezone.utc).isoformat().replace('+00:00', 'Z'),
        }
        v.verdict_file.write_text(json.dumps(verdict, indent=4), encoding='utf-8')
        sys.exit(0 if verdict_name == 'ACCEPTED' else 1)

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
            v.add_gate('V2 build', 'FAIL', f"cargo check exit {exit_code}; see out.txt", code='BUILD_FAIL')
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
            v.add_gate('V3 lint', 'FAIL', f"cargo fmt exit {fmt_exit}; see out.txt", code='FMT_ERROR')
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
            # ...but "could not compile" does NOT always mean "was not linted".
            # truck-meshalgo's lib.rs carries `#![deny(clippy::all,
            # rust_2018_idioms)]`, so its ~93 PRE-EXISTING lints are hard errors
            # no matter what this gate puts on the command line, and cargo then
            # reports "could not compile `truck-meshalgo` (lib) due to 93
            # previous errors". The crate was linted -- exhaustively -- and every
            # finding is right there in the output; the build merely aborted
            # afterwards. Treating that as "never produced" made V3 unpassable
            # for ANY packet touching this crate, at base or otherwise, which is
            # the baseline-failure signature the whole gate design warns about:
            # BG-TOL-001-MESHALGO was rejected this way with all 93 findings in
            # files its diff never opened.
            #
            # The distinction that matters is lint-abort vs. genuine build
            # failure, and rustc marks the difference: a real compile error
            # carries an `error[E####]` code, a lint never does. So only treat a
            # crate as unlinted when an E-coded diagnostic is present -- then
            # "no findings" really does mean "nothing was looked at" and the
            # coverage guard keeps doing its job.
            if unlinted and not re.search(r'error\[E\d+\]', clippy_text):
                unlinted = []
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
                v.add_gate('V3 lint', 'FAIL', '; '.join(why), code='LINT_UNLINTED')
                v.failed_early = True
            elif our_findings:
                v.add_gate('V3 lint', 'FAIL', "clippy findings in changed files: " + ' ; '.join(our_findings[:5]), code='LINT_FINDINGS_IN_DIFF')
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
            v.add_gate('V4 house rules', 'FAIL', f"kernel-gates.sh exit {gates_exit}; see out.txt", code='HOUSE_RULES')
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
            v.add_gate('V5 tests', 'FAIL', 'test target(s) failed to compile; see out.txt', code='TEST_BUILD_FAIL')
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
            # A disappeared full path whose leaf test fn passes elsewhere in
            # this run is a MOVE (e.g. a test module crossing crates in a
            # stacked packet), not a deletion. Deleting-to-cheat does not
            # re-add the name anywhere; a move must. This keeps the property
            # the disappearance charge exists for -- you cannot delete or
            # #[ignore] your way to green -- while not charging moves, which
            # the comment above already recorded as this gate's known false
            # positive. First exercised live by BG-CE-006-ENUM-r2, whose six
            # BG-S0-001/S0-003 tests moved from truck-modeling/src/geometry.rs
            # to truck-geometry/src/canonical.rs and were charged as deleted.
            leaf_now_ok = {n.rsplit('::', 1)[-1] for n, s in now.items() if s == 'ok'}
            moved = [n for n in disappeared if n.rsplit('::', 1)[-1] in leaf_now_ok]
            disappeared = [n for n in disappeared if n not in set(moved)]
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
                v.add_gate('V5 tests', 'FAIL', '; '.join(parts) + '; see out.txt', code='ADDED_TEST_FAILED')
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
            v.add_gate('V6 test-reality', 'FAIL', '; '.join(detail_parts), code='TEST_MISSING')
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
    # V8 no-regression — the same base-vs-HEAD test comparison V5 makes, run
    # over the crates DOWNSTREAM of the ones the packet names.
    #
    # V5 already compares against a real baseline, so the "a packet broke a
    # pre-existing test" property the old stub comment promised is discharged
    # there. What V5 cannot see is a crate the packet does not list: it runs
    # `cargo test -p <pkt crates>` and nothing else. Every BG-TOL-001 shard
    # tightens a predicate from componentwise `.near()` to Euclidean, in a
    # crate the whole kernel depends on, and the test that notices lives
    # downstream by construction.
    #
    # This is not hypothetical. Two `cone_topology_tests` invariant tests have
    # been failing in truck-meshalgo since the tree was vendored at da72cd5,
    # with every gate reporting green the whole time, for exactly this reason:
    # no packet had ever listed truck-meshalgo in `crates:`, so no verify run
    # ever executed them.
    #
    # Reverse dependencies are read from the vendored Cargo.toml files and
    # closed transitively. The packet's own crates are removed -- V5 owns
    # those, and running them twice would double the slowest gate for nothing.
    # An empty downstream set is a PASS that says so, not a silent one.
    # -----------------------------------------------------------------------
    # V8 is ON by default since 2026-08-19, when its negative test ran.
    #
    # The probe: `RevolutedCurve::subs` offset by 1e-3 on branch
    # probe/V8-negative (base 3ca4518), one commit plus a RESULT.json stub.
    # V8 FAILed with DOWNSTREAM_REGRESSION naming eight tests across
    # truck-modeling and truck-shapeops -- builder::partial_torus::
    # partial_torus, geometry::{conical_surface, cylindrical_surface,
    # surface_of_revolution}, healing::tests::{double_closed_boundary_cylinder,
    # many_closed_boundary_cylinder, ...} -- none of which V5 can reach,
    # because V5 runs only the packet's own crates. The gate has been
    # watched failing; the opt-in branch is deleted in the commit that
    # records this.
    # -----------------------------------------------------------------------
    # Base work is LAZY as of 2026-08-20. The gate used to build a second
    # complete Cargo world at the base commit and run the entire downstream
    # suite there, just to discover which of the downstream tests were already
    # red. That is ~10 GB of transient disk for an answer that is only needed
    # about the tests that are failing NOW, and it is what put the 2026-08-19
    # session into repeated ENOSPC and corrupt-metadata rebuilds.
    #
    # The algorithm instead:
    #   1. Run the FULL downstream suite at HEAD. Coverage is unchanged --
    #      this is the half of V8 that catches things, and it is not narrowed.
    #   2. Everything green -> PASS, and the base is never built at all. This
    #      is the common case and it now costs nothing.
    #   3. Otherwise interrogate the base about the failing (and newly
    #      `ignored`) tests ONLY, one `cargo test -p <crate> --test <target>
    #      -- --exact <names>` per target, so the base build is one crate's
    #      dependency cone rather than the workspace.
    #   4. Fails at both -> pre-existing, reported, not charged. Passes at
    #      base -> a real regression, charged, same as before.
    #   5. Each base observation is cached per (base, crate, target, test) in
    #      loop/baselines/<base>__v8-observations.json, so a re-verify against
    #      the same base pays nothing for a failure it has already explained.
    #
    # Two properties this must not lose, and does not: the HEAD suite is still
    # the whole downstream closure, and a test that passes at base and fails
    # here is still a FAIL. What it drops is only the work of re-measuring
    # tests that are green at HEAD, whose base status cannot change any
    # verdict.
    # -----------------------------------------------------------------------
    if not gate_wanted('V8'):
        skip_not_requested('V8 no-regression')
    elif v.failed_early:
        v.add_gate('V8 no-regression', 'SKIP', 'earlier gate failed')
    else:
        downstream = sorted(reverse_dep_closure(v.wt, crate_names))
        if not downstream:
            v.add_gate('V8 no-regression', 'PASS',
                       'no vendored crate depends on ' + ', '.join(crate_names)
                       + '; V5 already covers everything this packet can reach')
        else:
            d_args = []
            for c in downstream:
                d_args += ['-p', c]
            v.write_out_section('V8 no-regression: downstream crates ' + ', '.join(downstream))
            len_before = v.out_file.stat().st_size
            v.invoke_native(['cargo', 'test', *d_args, '--lib', '--tests', '--no-fail-fast'], v.wt)
            chunk = v.out_file.read_bytes()[len_before:].decode('utf-8', errors='replace')

            if has_compile_error(chunk):
                # A downstream crate that no longer compiles is the loudest
                # possible regression: the packet changed something its own
                # V2 could not see because V2 only checks the packet's crates.
                v.add_gate('V8 no-regression', 'FAIL',
                           'downstream crate(s) failed to compile: ' + ', '.join(downstream)
                           + '; see out.txt', code='DOWNSTREAM_BUILD_FAIL')
                v.failed_early = True
            else:
                attributed = attribute_tests(v.last_stdout, v.last_stderr, v.wt, downstream)
                # Only a test that is FAILED or `ignored` at HEAD can possibly
                # be a downstream regression. Everything else passed here and
                # needs no opinion from the base, which is the whole saving:
                # a green downstream suite costs zero base work.
                suspects = {n: a for n, a in attributed.items()
                            if a['status'] in ('FAILED', 'ignored')}
                total_seen = len(attributed)

                if not suspects:
                    v.add_gate('V8 no-regression', 'PASS',
                               f"{len(downstream)} downstream crate(s) green at HEAD "
                               f"({total_seen} test(s)): " + ', '.join(downstream)
                               + '; base not built (nothing to explain)')
                else:
                    obs = load_v8_observations(base)
                    groups = {}
                    for n, a in sorted(suspects.items()):
                        if observation_key(a['crate'], a['target'], n) in obs:
                            continue
                        groups.setdefault((a['crate'], a['target']), []).append(n)

                    unavailable = []
                    if groups:
                        try:
                            fresh, unavailable = query_base_tests(base, groups, v.out_file, d_args)
                        except RuntimeError as exc:
                            fresh, unavailable = {}, list(groups)
                            v.write_out_section('V8 base query refused: ' + str(exc))
                        if fresh:
                            obs.update(fresh)
                            save_v8_observations(base, obs)

                    if unavailable:
                        # The base commit predates the packet, so a base build
                        # failure is never the packet's doing -- it is disk,
                        # toolchain or a corrupt target dir. Rejecting the
                        # worker for it is the exact mistake that cost the
                        # 2026-08-19 session its evening. BLOCKED is the
                        # verdict that says "ask the harness, not the worker".
                        v.add_gate('V8 no-regression', 'SKIP',
                                   'base unavailable for '
                                   + ', '.join(f"{c or '?'} {' '.join(t) if t else ''}".strip()
                                               for c, t in unavailable[:4])
                                   + '; nothing cached; see out.txt')
                        print()
                        print('VERDICT: BLOCKED')
                        verdict = {
                            'packet': args.packet, 'slot': v.slot, 'crates': crate_names,
                            'base': base, 'branch': branch, 'commit': commit_sha,
                            'amended_by': amended_by, 'verdict': 'BLOCKED', 'gates': v.gates,
                            'timestamp': datetime.datetime.now(datetime.timezone.utc)
                                .isoformat().replace('+00:00', 'Z'),
                        }
                        v.verdict_file.write_text(json.dumps(verdict, indent=4), encoding='utf-8')
                        sys.exit(2)

                    newly_failing, still_failing, newly_ignored = [], [], []
                    for n, a in sorted(suspects.items()):
                        at_base = obs.get(observation_key(a['crate'], a['target'], n))
                        if a['status'] == 'FAILED':
                            # 'absent' -- the target built and ran at base and
                            # has no such test -- is charged, exactly as the
                            # old whole-suite comparison charged a name the
                            # baseline had never seen. A packet cannot add a
                            # downstream test (V1 forbids the write), so an
                            # absence here is a claim that needs explaining.
                            (still_failing if at_base == 'FAILED' else newly_failing).append(n)
                        elif at_base == 'ok':
                            newly_ignored.append(n)

                    if newly_failing or newly_ignored:
                        parts = []
                        if newly_failing:
                            parts.append('newly failing downstream: ' + ', '.join(newly_failing[:8]))
                        if newly_ignored:
                            parts.append('passed at base, #[ignore]d now: ' + ', '.join(newly_ignored[:8]))
                        v.add_gate('V8 no-regression', 'FAIL', '; '.join(parts) + '; see out.txt',
                                   code='DOWNSTREAM_REGRESSION')
                        v.failed_early = True
                    else:
                        detail = (f"{len(downstream)} downstream crate(s) unchanged vs base "
                                  f"{base[:7]} ({total_seen} test(s) at HEAD): "
                                  + ', '.join(downstream))
                        if still_failing:
                            detail += (f"; {len(still_failing)} pre-existing failure(s) confirmed "
                                       'at base: ' + ', '.join(still_failing[:8]))
                        queried = sum(len(t) for t in groups.values())
                        detail += (f"; {queried} base observation(s) measured, "
                                   f"{len(suspects) - queried} served from cache")
                        v.add_gate('V8 no-regression', 'PASS', detail)

    # -----------------------------------------------------------------------
    # V9 geometry — the root crate's integration tests, which are the only
    # thing in this harness that touches a real part.
    #
    # Every other gate is a build, a lint, a house rule, or a unit test on a
    # type. Nine contracts landed before this gate existed and not one had been
    # shown to change what the kernel does to a STEP file. `tests/step.rs`,
    # `tests/torus_deck.rs` and `tests/spline_carrier.rs` in the root `look`
    # crate import real fixtures and drive them through import and
    # tessellation -- and no packet has ever listed `look` in its `crates:`,
    # so V5 has never once run them.
    #
    # This runs ALWAYS, regardless of which crates the packet names, because
    # the packets that most need it are exactly the ones that do not think
    # they touch geometry. The BG-TOL-001 Stage-A shards are the case in
    # point: they migrate call sites while deliberately changing no threshold,
    # so a shard that quietly broke one produces identical results on every
    # other gate.
    #
    # Same baseline comparison as V5 -- run the same targets at `base`, cache,
    # and fail only on what newly fails -- so a pre-existing failure in the
    # tree is not charged to the packet. gpu_smoke and assembly are excluded:
    # they depend on an adapter that may not exist on the runner, and a gate
    # that fails for want of a GPU is noise, not signal.
    # -----------------------------------------------------------------------
    # geometry_fingerprint is the load-bearing one and the others are context.
    # V9's first version ran only step/torus_deck/spline_carrier and PASSED with
    # truck_base::TOLERANCE loosened from 1e-6 to 1e-1 -- a five-order-of-
    # magnitude change on the covered path. Their assertions turned out to be
    # structural (one geometry, one instance, indices a multiple of 3, a colour
    # present) and torus_deck asserts on the *declared* torus parameters rather
    # than on anything tessellated, so all of them hold for an arbitrarily wrong
    # mesh. geometry_fingerprint asserts triangle count, vertex count and
    # bounds, which is what actually moves.
    GEOM_TESTS = ['--test', 'geometry_fingerprint', '--test', 'step',
                  '--test', 'torus_deck', '--test', 'spline_carrier']
    if not gate_wanted('V9'):
        skip_not_requested('V9 geometry')
    elif not v.failed_early:
        geom_base = load_or_compute_baseline(base, ['look'], v.out_file, GEOM_TESTS)
        v.write_out_section('V9 geometry: cargo test -p look on real STEP fixtures')
        len_before = v.out_file.stat().st_size
        v.invoke_native(['cargo', 'test', '-p', 'look', *GEOM_TESTS, '--no-fail-fast'], v.wt)
        geom_text = v.out_file.read_bytes()[len_before:].decode('utf-8', errors='replace')

        if has_compile_error(geom_text):
            v.add_gate('V9 geometry', 'FAIL',
                       'the root crate\'s geometry tests failed to compile; see out.txt')
        elif not geom_base['compile_ok']:
            v.add_gate('V9 geometry', 'PASS',
                       f"baseline at {base[:7]} would not compile, so no comparison is possible")
        else:
            now = parse_test_statuses(geom_text)
            was = geom_base['tests']
            newly = sorted(n for n, s in now.items()
                           if s != 'ok' and was.get(n) == 'ok')
            vanished = sorted(n for n, s in was.items()
                              if s == 'ok' and n not in now)
            if newly or vanished:
                parts = []
                if newly:
                    parts.append('now failing on real geometry: ' + ', '.join(newly[:5]))
                if vanished:
                    parts.append('disappeared: ' + ', '.join(vanished[:5]))
                v.add_gate('V9 geometry', 'FAIL', '; '.join(parts) + '; see out.txt', code='GEOMETRY_MOVED')
            else:
                v.add_gate('V9 geometry', 'PASS',
                           f"{len(now)} geometry test(s) on real STEP fixtures, "
                           f"no regression vs baseline at {base[:7]}")
    else:
        v.add_gate('V9 geometry', 'SKIP', 'earlier gate failed')

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
