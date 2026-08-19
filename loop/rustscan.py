"""Line-by-line Rust scanning shared by the loop's two site counters.

Both `census_tol_sites.py` and `gen_packet.py` need to answer "which function is
this line in?", and both answered it by looking for the nearest `fn <name>`
without tracking braces -- the census by remembering the last `fn` seen on a
forward pass, `gen_packet._enclosing_fn` by scanning upward until one matched.
Neither notices that a function has *closed*. A site sitting just after a nested
helper's closing brace is therefore attributed to the helper, and two functions'
sites collapse into one context.

That cost a dispatch. `BG-TOL-001-MESHALGO`'s packet declared a budget of 11
contexts against a true 10: `triangulation.rs` holds a two-line nested `fn
end_pts` closing at 8276, and the sites at 8278 and 8283 belong to the enclosing
`new_with_join`, not to `end_pts`. The worker could not reach 11 honestly, so it
built a shadow context inside a `match` arm of `new_with_join` to satisfy a
number that was wrong. The budget gate was checking a claim against a counter
that shared the claim's defect.

The fix is a stack: a `fn` is pushed when its definition line is seen, armed
once the brace depth actually rises above the depth it was declared at, and
popped when the depth comes back down. The innermost armed entry is the context.
The arming step is load-bearing for exactly the reason it is in the census's
`#[cfg(test)]` tracking: a signature can span lines, so on the definition line
itself the depth has not moved yet and an unguarded `depth <= base` pops the
function immediately.

Braces are counted on code with string literals, char literals, line comments
and (nesting) block comments removed -- a format string like `"{i} {t}"` reads
as two opens otherwise, and `revolved_curve.rs` has block comments with braces
in them. Multi-line string literals are the one case this does not model.
Measured: of 304 `.rs` files under `vendor/truck/`, exactly one ends at a
nonzero brace depth -- `truck-stepio/tests/input/table.rs`, which embeds STEP
files as multi-line literals. Neither consumer reaches it: the census walks
`<crate>/src` only, and `gen_packet` resolves only files a packet's
`write_allow` names. Re-run that balance check if this is ever pointed at
`tests/`.
"""
import re
from collections import namedtuple

# Matches a fn definition line. `pub(crate)`, `const`, `async`, `unsafe` and
# `extern "C"` all sit between `fn` and the start of the line.
FN_DEF = re.compile(
    r"^\s*(?:pub(?:\([^)]*\))?\s+)?(?:default\s+)?(?:const\s+)?(?:async\s+)?"
    r"(?:unsafe\s+)?(?:extern\s+\"[^\"]*\"\s+)?fn\s+(\w+)")

# A char literal is exactly one char (or one escape) between ticks. Matching
# `'...'` greedily instead swallows `'a, 'b` -- a pair of lifetimes -- and takes
# whatever braces sit between them with it.
CHAR_LIT = re.compile(r"'(?:\\.|[^'\\])'")

Line = namedtuple('Line', 'lineno raw code depth_before depth_after fn_name fn_line was_in_comment')


def strip_line(raw, block):
    """(code, block_nesting_after). `code` has literals and comments blanked."""
    out = []
    i, n = 0, len(raw)
    while i < n:
        if block > 0:
            if raw.startswith('*/', i):
                block -= 1
                i += 2
            elif raw.startswith('/*', i):
                block += 1
                i += 2
            else:
                i += 1
            continue
        if raw.startswith('//', i):
            break
        if raw.startswith('/*', i):
            block += 1
            i += 2
            continue
        c = raw[i]
        if c == '"':
            i += 1
            while i < n:
                if raw[i] == '\\':
                    i += 2
                    continue
                if raw[i] == '"':
                    i += 1
                    break
                i += 1
            out.append('""')
            continue
        if c == "'":
            m = CHAR_LIT.match(raw, i)
            if m:
                out.append("''")
                i = m.end()
            else:
                # A lifetime tick, not a literal. Consume just the tick.
                i += 1
            continue
        out.append(c)
        i += 1
    return ''.join(out), block


def scan(text):
    """Yield one `Line` per source line, carrying the enclosing fn.

    `fn_name`/`fn_line` name the innermost function whose body contains the
    line, or `('<file scope>', 0)` outside every function. `depth_before` is the
    brace depth entering the line and `depth_after` the depth leaving it, both
    counted on `code`.
    """
    block = 0
    depth = 0
    stack = []  # [name, def_line, base_depth, armed]
    for lineno, raw in enumerate(text.splitlines(), 1):
        was_in_comment = block > 0
        code, block = strip_line(raw, block)

        m = FN_DEF.match(code)
        if m:
            # A trait's `fn foo(&self);` never opens a body, so it never arms.
            # Drop an unarmed sibling before pushing, or the whole trait block
            # reads as being inside its first declaration.
            while stack and not stack[-1][3]:
                stack.pop()
            stack.append([m.group(1), lineno, depth, False])

        depth_before = depth
        depth += code.count('{') - code.count('}')

        # The context includes an as-yet-unarmed entry, so the lines of a
        # multi-line signature belong to their own function: `fn foo(` opens no
        # brace, and `singular_transition_branch`'s six parameter lines would
        # otherwise read as file scope and fail to resolve at all.
        if stack:
            fn_name, fn_line = stack[-1][0], stack[-1][1]
        else:
            fn_name, fn_line = '<file scope>', 0

        while stack:
            top = stack[-1]
            if depth > top[2]:
                top[3] = True
                break
            if top[3] or depth < top[2]:
                # Armed and closed, or an unarmed declaration whose enclosing
                # block just closed around it.
                stack.pop()
                continue
            if code.rstrip().endswith(';'):
                # `fn foo(&self) -> f64;` in a trait: a signature that never
                # opens a body. It ends here, and what follows is not inside it.
                stack.pop()
                continue
            break

        yield Line(lineno, raw, code, depth_before, depth, fn_name, fn_line, was_in_comment)


def enclosing_fn(text, line):
    """The definition line of the fn containing `line`, or None at file scope."""
    for info in scan(text):
        if info.lineno == line:
            return info.fn_line or None
    return None
