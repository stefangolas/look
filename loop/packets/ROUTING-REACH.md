# SURVEY PACKET ROUTING-REACH — door-path reachability census of the executor surface

The audit's routing table (9 gaps) is a surveyor's list, not a
mechanical sweep. This survey makes it mechanical: enumerate EVERY
public entry point of the executor surface and classify whether it is
REACHABLE from the door path. Fragment A's `pub_entry_census` (608 pub
fns, 0 unrepresented) is the template; the deliverable here is
reachability, not contract extraction.

```yaml
id:          ROUTING-REACH
contract:    [ROUTING-REACH]
class:       survey
crates:      []
depends_on:  []
write_allow:
  - loop/routing_reach
read_allow:
  - truck123d/src/binding.rs
  - truck123d/src/bd_bridge.rs
  - truck123d/src/facade.rs
  - corpus/ttc/door.py
tests_required: []
anchors:
  - {id: A1, expect: 2, cmd: "grep -c 'pub fn bd_facts' truck123d/src/bd_bridge.rs"}
budget:      {turns: 45, ctx_tokens: 140000}
```

## Method

1. **The door path roots** (fixed, do not relitigate): `bd_facts`,
   `bd_stl`, `facade_submit`, plus every other pyo3-exported symbol
   reachable from `corpus/ttc/door.py`'s `_T123D` calls. Enumerate the
   roots from door.py's call sites, then the pyo3 export list
   (`#[pyfunction]`/`#[pymethods]` in binding.rs / bd_bridge.rs).
2. **Transitive closure.** From each root, walk the call graph
   (bd_bridge internal fns, binding pub fns, dispatched enums) as far
   as Rust in THESE THREE FILES reaches. vendor/truck internals are a
   single terminal node ("kernel call") — do not descend.
3. **Deliverable** `loop/routing_reach/REACH.json`: one row per public
   entry point of the three files — symbol, file, line, reachable
   (true/false), reach path (root -> ... -> symbol, short), or
   UNREACHABLE with the one-sentence reason. Plus a summary count and
   a list of UNREACHABLE public symbols sorted by "should obviously be
   reachable" plausibility.
4. **Confidence per row**: high (call edge read directly), medium
   (reached via a macro or a dispatch table), low (inferred). The
   orchestrator spot-checks high-confidence rows and reads every low.

## Done when

REACH.json covers 100% of the public entry points of the three files;
every UNREACHABLE row names a reason; anchors hold. Write RESULT.json
AT THE WORKTREE ROOT.

## Forbidden

No code changes of any kind — this survey has no write access outside
loop/routing_reach. No judgements about whether an unreachable symbol
SHOULD be wired (that is the orchestrator's); only the fact and the
path.
