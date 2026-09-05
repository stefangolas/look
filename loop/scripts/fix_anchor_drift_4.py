"""Session-51: three anchor drifts from the post-wifi tree re-measure."""
import ast

# CC-014 A4: rank_margin has curve+surface impls = 2
p = "loop/packets/CC-014-LOFT-VALIDITY.md"
s = open(p, encoding="utf-8").read()
old = "{id: A4, expect: 1, cmd: \"grep -c 'pub fn rank_margin'"
assert old in s
s = s.replace(old, "{id: A4, expect: 2, cmd: \"grep -c 'pub fn rank_margin'")
open(p, "w", encoding="utf-8", newline="\n").write(s)

# CC-024 A1: '[Curve]' is a grep character class - the literal can never match.
# Anchor the function BODY's first line instead (unique, regex-safe).
p = "loop/packets/CC-024-OFFSET-EXACT.md"
s = open(p, encoding="utf-8").read()
i = s.index("{id: A1,")
j = s.index("\n", i)
s = s[:i] + (
    "{id: A1, expect: 1, cmd: \"grep -c 'let mut carriers = "
    "Vec::with_capacity(profile.len());' vendor/truck/truck-geometry/src/arrange.rs\"}"
) + s[j:]
open(p, "w", encoding="utf-8", newline="\n").write(s)

# CC-032 A2: arrange has doc-comment + real definition = 2
p = "loop/packets/CC-032-FACE-CONSUMPTION.md"
s = open(p, encoding="utf-8").read()
old = "{id: A2, expect: 1, cmd: \"grep -c 'pub fn arrange'"
assert old in s
s = s.replace(old, "{id: A2, expect: 2, cmd: \"grep -c 'pub fn arrange'")
open(p, "w", encoding="utf-8", newline="\n").write(s)

for p in ("loop/packets/CC-014-LOFT-VALIDITY.md",
          "loop/packets/CC-024-OFFSET-EXACT.md",
          "loop/packets/CC-032-FACE-CONSUMPTION.md"):
    open(p, encoding="utf-8").read()
print("three anchor fixes applied")
