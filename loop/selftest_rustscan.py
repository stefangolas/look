"""Prove the brace-tracking scanner catches what the old scan could not.

The rule is "watch a gate fail before you trust it". The thing being replaced
is `gen_packet._enclosing_fn`'s upward scan for the nearest `fn`, so this runs
BOTH implementations over the same fixture and asserts they disagree in the
direction the bug predicts: the old one credits a site to a nested helper that
has already closed, the new one credits it to the enclosing function.

    python loop/selftest_rustscan.py
"""
import re
import sys
from pathlib import Path

sys.stdout.reconfigure(encoding='utf-8')
sys.path.insert(0, str(Path(__file__).resolve().parent))
import rustscan  # noqa: E402

REPO_ROOT = Path(__file__).resolve().parent.parent

# The shape that cost BG-TOL-001-MESHALGO a budget of 11 against a true 10.
FIXTURE = '''\
impl Foo {
    fn new_with_join(&self, x: f64) -> f64 {
        let ctx = Ctx::new();
        fn end_pts<T: Copy>(v: &[T]) -> (T, T) {
            (v[0], v[v.len() - 1])
        }
        let (p, q) = end_pts(&self.curve);
        ctx.near(p, q)
    }
}

trait Bare {
    fn decl(&self) -> f64;
    fn other(&self) -> f64;
}

fn multi_line_sig(
    a: f64,
    b: f64,
) -> f64 {
    a.near(&b)
}

fn braces_in_text() -> f64 {
    let s = format!("{a} {b}");           // literal braces
    /* a block comment { with braces } */
    let c = '{';
    s.len() as f64 + c as u8 as f64
}
'''

# Verbatim from gen_packet.py before this change, so the comparison is real.
OLD_FN_DEF = re.compile(
    r"^\s*(?:pub(?:\([^)]*\))?\s+)?(?:const\s+)?(?:async\s+)?(?:unsafe\s+)?"
    r"(?:extern\s+\"[^\"]*\"\s+)?fn\s+(\w+)")


def old_enclosing_fn(lines, line):
    for i in range(line - 1, -1, -1):
        if OLD_FN_DEF.match(lines[i]):
            return i + 1
    return None


def name_at(text, line):
    for info in rustscan.scan(text):
        if info.lineno == line:
            return info.fn_name
    return None


def main():
    lines = FIXTURE.splitlines()
    failures = []

    def check(label, got, want):
        ok = got == want
        print(f"  {'ok  ' if ok else 'FAIL'} {label}: {got!r}" + ('' if ok else f' (want {want!r})'))
        if not ok:
            failures.append(label)

    print('the defect the old scan has, on the fixture:')
    # Line 7 is `let (p, q) = end_pts(...)`, one line after end_pts closes.
    old = old_enclosing_fn(lines, 7)
    old_name = OLD_FN_DEF.match(lines[old - 1]).group(1)
    check('old upward scan attributes line 7 to the CLOSED helper', old_name, 'end_pts')
    check('new scanner attributes line 7 to the enclosing fn', name_at(FIXTURE, 7), 'new_with_join')
    check('the helper still owns its own body (line 5)', name_at(FIXTURE, 5), 'end_pts')

    print('trait declarations do not swallow what follows:')
    check('a bare `fn decl(&self) -> f64;` owns only its own line',
          name_at(FIXTURE, 13), 'decl')
    check('and not the sibling declaration after it', name_at(FIXTURE, 14), 'other')
    check('nor the line after the trait closes', name_at(FIXTURE, 15), '<file scope>')

    print('a multi-line signature belongs to its own fn:')
    check('parameter line', name_at(FIXTURE, 19), 'multi_line_sig')
    check('body line', name_at(FIXTURE, 22), 'multi_line_sig')

    print('braces in literals and comments do not move the depth:')
    check('format-string braces', name_at(FIXTURE, 26), 'braces_in_text')
    check('block-comment braces', name_at(FIXTURE, 28), 'braces_in_text')
    check("char literal '{'", name_at(FIXTURE, 29), 'braces_in_text')

    # The live case, kept because a fixture cannot prove the real file parses.
    live = REPO_ROOT / 'vendor/truck/truck-meshalgo/src/tessellation/triangulation.rs'
    if live.exists():
        print('and on the real file the packet got wrong:')
        text = live.read_text(encoding='utf-8', errors='replace')
        ends = [i.lineno for i in rustscan.scan(text) if i.fn_name == 'end_pts']
        if not ends:
            print('  skip  `fn end_pts` is gone from triangulation.rs')
        else:
            after = max(ends) + 2
            check(f'triangulation.rs:{after} (just past end_pts)',
                  name_at(text, after), 'new_with_join')

    print()
    if failures:
        print(f'{len(failures)} FAILED: ' + ', '.join(failures))
        return 1
    print('all ok')
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
