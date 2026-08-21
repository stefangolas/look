# WORK PACKET BG-CE-001 — per-use pcurve payload on the edge handle

You are implementing one item from a formal kernel specification. Everything you
need is in this document. **Do not read `docs/GENERATION_KERNEL_BUILD_SPEC.md`
or any other spec file** — they are not on your allowlist and this packet is
self-contained. If something you need is genuinely missing, that is a SPEC_GAP
(see "Stop conditions"): you stop and report, you do not research it.

```json
{"id":"BG-CE-001","status":"DONE","contracts":["BG-CE-001"],
 "tests_added":4,"sites_migrated":0,"sites_deferred":0,
 "deviations":[],"disagreements":[],"baseline_failures":[],
 "notes":"free text"}
```

Fill `deviations`, `disagreements` and `baseline_failures` as arrays, empty if
empty. `disagreements` is the highest-value field: the claims below were derived
by command against the tree, but they are exactly the kind of claim that can be
confidently wrong. **If anything below contradicts what you find in the code,
say so in `disagreements` rather than making the code match the packet.**

```yaml
id:          BG-CE-001
contract:    [BG-CE-001]
covers:      [BG-CE-001-MIGRATE]
class:       design
crates:      [truck-topology]
write_allow:
  - vendor/truck/truck-topology/src/lib.rs
  - vendor/truck/truck-topology/src/edge.rs
read_allow:
  - vendor/truck/truck-topology/src/imported.rs
  - vendor/truck/truck-topology/src/compress.rs
  - vendor/truck/truck-topology/src/face.rs
  - vendor/truck/truck-topology/src/wire.rs
tests_required:
  - pcurve_defaults_to_none_and_stays_out_of_identity
  - with_pcurve_sets_payload_and_shares_the_curve
  - inverse_absolute_clone_and_clone_carry_pcurve
  - cut_drops_pcurve_on_both_halves
budget:      {turns: 40, ctx_tokens: 100000}
anchors:
  - {id: A1, expect: 0, cmd: "grep -c 'pcurve' vendor/truck/truck-topology/src/edge.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub struct Edge<P, C>' vendor/truck/truck-topology/src/lib.rs"}
  - {id: A3, expect: 8, cmd: "grep -c 'Edge {' vendor/truck/truck-topology/src/edge.rs"}
  - {id: A4, expect: 3, cmd: "grep -c 'Self {' vendor/truck/truck-topology/src/edge.rs"}
  - {id: A5, expect: 1, cmd: "grep -c 'impl<P, C> Clone for Edge<P, C>' vendor/truck/truck-topology/src/edge.rs"}
  - {id: A6, expect: 1, cmd: "grep -c 'impl<P, C> PartialEq for Edge<P, C>' vendor/truck/truck-topology/src/edge.rs"}
  - {id: A7, expect: 4, cmd: "grep -c 'Arc::as_ptr' vendor/truck/truck-topology/src/edge.rs"}
  - {id: A8, expect: 1, cmd: "grep -c 'fn pre_cut' vendor/truck/truck-topology/src/edge.rs"}
  - {id: A9, expect: 1, cmd: "grep -c 'pub const fn orientation' vendor/truck/truck-topology/src/edge.rs"}
  - {id: A10, expect: 1, cmd: "grep -c 'type Edge = ' vendor/truck/truck-topology/src/imported.rs"}
```

## Problem

truck's `Edge` is *already* a coedge: the curve is shared through
`Arc<Mutex<C>>` and `orientation` is per-handle, so two handles to one edge are
already two coedges over one curve. But the handle carries exactly **one**
per-use field. A seam edge's two uses each have their own parametric trace on
the owning face — two different 2D curves on the surface's domain — and today
there is nowhere to put the second one. The same-parameter certificate
(`||Gamma_f(pc_u(t)) - c_e(phi_u(t))|| <= tau_e` over the whole span) that the
next packets in this chain certify is unstatable without it.

This packet adds the second per-use field, `pcurve: Option<PC>`, behind a
**defaulted type parameter** `PC = ()`, so the change is semantically inert for
every existing mention of the type.

**Verified fact — the ripple is two files.** Rust applies defaulted type
parameters in every type position, including `impl` headers (verified against
this toolchain with a scratch crate): `impl<P, C> Edge<P, C>` keeps compiling
and keeps meaning `Edge<P, C, ()>`. There is no turbofish `Edge::<` anywhere in
the vendored tree, no struct-update `Edge { ..base }`, and no exhaustive
`Edge { ... }` pattern of this `Edge` outside `edge.rs` (the `Edge` types in
`truck-shapeops/src/healing/` and `truck-assembly/src/dag.rs` are *local,
different* types with `usize`/node payloads — leave them alone). The complete
list of struct-literal construction sites of `truck_topology::Edge` in the whole
tree:

- `edge.rs:42` — `new_unchecked`
- `edge.rs:116` — `inverse`
- `edge.rs:221` — `absolute_clone` (a `Self {` literal)
- `edge.rs:431` and `edge.rs:436` — `pre_cut`
- `edge.rs:581` — the manual `Clone` impl
- plus the definition itself at `lib.rs:126`

Everything else — the 26 files mentioning `Edge<` across six crates, the
`prelude!` macro's `type Edge = $crate::Edge<$point, $curve>;` alias in
`imported.rs`, every `impl` block, every signature, `EdgeID` (which is
`ID<Mutex<C>>` and does not mention the edge at all) — compiles unchanged.
**If you find a file outside `write_allow` that does not compile after the
struct change, that is a stop condition (SPEC_GAP), not something to fix.**

## Decisions already made for you

1. **The struct, verbatim** (field order matters: `pcurve` sits between
   `orientation` and `curve`; the derived `Debug` output gains the field, which
   nothing in the tree asserts):

   ```rust
   #[derive(Debug)]
   pub struct Edge<P, C, PC = ()> {
       vertices: (Vertex<P>, Vertex<P>),
       orientation: bool,      // existing per-use field
       pcurve: Option<PC>,     // NEW per-use field: the parametric trace on the owning face
       curve: Arc<Mutex<C>>,   // shared entity geometry
   }
   ```

2. The inherent impl header becomes `impl<P, C, PC> Edge<P, C, PC>`.

3. **Construction sites, one by one:**
   - `new_unchecked` (edge.rs:42): add `pcurve: None`.
   - `inverse` (edge.rs:116): add `pcurve: self.pcurve.clone()`, and the method
     gains `where PC: Clone`. Rationale: the inverse is the *same use traversed
     backwards* — the trace does not mirror, so it is carried.
   - `absolute_clone` (edge.rs:221): carries `self.pcurve.clone()`, gains
     `where PC: Clone`. Same reasoning: same use, forward orientation.
   - The manual `Clone` impl (edge.rs:578) becomes
     `impl<P, C, PC: Clone> Clone for Edge<P, C, PC>` and clones the field.
     With `PC = ()` everywhere today, `(): Clone` keeps every existing clone
     site compiling.
   - `pre_cut` (edge.rs:426): both halves get `pcurve: None`, with a short
     comment naming the deferred semantics: restricting an arbitrary `PC` needs
     a `Cut` bound this packet does not add, and carrying the *full* trace on
     both halves would over-approximate, so the halves drop it; the packet that
     wires real pcurves owns trace splitting.
   - `invert` (in-place) is untouched — it only flips `orientation`.
   - `mapped` is untouched — it builds via `debug_new`, so the mapped edge has
     `pcurve: None` (mapping to another space invalidates the trace). Add a
     doc note only if you judge one necessary; no code change.

4. **Identity does not change.** `PartialEq`, `Eq` and `Hash`
   (edge.rs:589-605) keep comparing exactly what they compare today: the
   curve's `Arc` pointer and `orientation`. The pcurve is per-use payload, not
   identity — two handles of one curve with the same sense remain equal even
   when their traces differ, and remain hash-equal. Do not touch these impls.

5. **Display does not change.** The `DebugDisplay` impls (edge.rs:607 onward)
   show id/vertices/entity only; the pcurve is invisible. Do not touch them.

6. **New API**, placed immediately after `orientation()` (edge.rs:72), each
   with a doctest in the house style (`P = ()`, `C = ()`, `PC = i32`):

   ```rust
   /// Returns the parametric trace of this edge use on its owning face,
   /// if one has been attached.
   #[inline(always)]
   pub fn pcurve(&self) -> Option<&PC> {
       self.pcurve.as_ref()
   }

   /// Attaches `pcurve` to this edge use, returning the updated handle.
   /// The curve, the vertices and the orientation are untouched: this is
   /// the same use of the same curve, now carrying its trace.
   #[inline(always)]
   pub fn with_pcurve(mut self, pcurve: PC) -> Self {
       self.pcurve = Some(pcurve);
       self
   }
   ```

7. **No other production change.** Not `imported.rs` (the macro alias compiles
   as-is), not `compress.rs`, not `face.rs`/`wire.rs`/`shell.rs`/`solid.rs`,
   not any other crate. `cargo check --workspace --all-targets` passing is part
   of Done-when, and is the evidence that BG-CE-001-MIGRATE — the
   "semantically inert, PC = () default" migration row this packet also closes
   — holds for the whole tree.

8. **Tests: one new `#[cfg(test)]` module named `coedge_tests` at the very end
   of `edge.rs`**, opening with `#![deny(clippy::unwrap_used)]` (house
   convention for new test modules) and `use super::*;`. Four tests:

   - `pcurve_defaults_to_none_and_stays_out_of_identity`: `Edge::new(...)` has
     `pcurve() == None`; two clones of one edge with *different* pcurves are
     `==` by `PartialEq`, and inserting both into a `HashSet` leaves exactly
     one element; an edge and its `inverse()` remain unequal (pre-existing
     semantics, unchanged).
   - `with_pcurve_sets_payload_and_shares_the_curve`: after `with_pcurve`,
     `pcurve()` is `Some(&…)`, `is_same` and `id()` match the original, and the
     original's `pcurve()` is still `None`.
   - `inverse_absolute_clone_and_clone_carry_pcurve`: an edge with `Some`
     pcurve — `inverse()`, `absolute_clone()` and `clone()` all still see it.
   - `cut_drops_pcurve_on_both_halves`: call `pre_cut` (private; reachable
     from the child module) on an edge with `Some` pcurve; both returned
     halves have `pcurve() == None`. You need a local `Cut` curve — copy this
     skeleton, a newtype over the `(usize, usize)` test curve that
     truck-geotrait already implements `ParametricCurve`/`BoundedCurve` for
     (but not `Cut`, and the orphan rule forbids adding that impl here):

     ```rust
     #[derive(Clone, Debug)]
     struct TestCutCurve(usize, usize);
     impl ParametricCurve for TestCutCurve {
         type Point = usize;
         type Vector = usize;
         fn subs(&self, t: f64) -> usize {
             if t < 0.5 { self.0 } else { self.1 }
         }
         fn der(&self, _: f64) -> usize { self.1 - self.0 }
         fn der2(&self, _: f64) -> usize { self.1 - self.0 }
         fn der_n(&self, _: usize, _: f64) -> usize { self.1 - self.0 }
         fn parameter_range(&self) -> ParameterRange {
             (Bound::Included(0.0), Bound::Included(1.0))
         }
     }
     impl BoundedCurve for TestCutCurve {}
     impl Cut for TestCutCurve {
         fn cut(&mut self, _t: f64) -> Self { *self }
     }
     ```

     The method set mirrors the geotrait tuple impl exactly, so it is exactly
     sufficient. The traits (`ParametricCurve`, `BoundedCurve`, `Cut`,
     `ParameterRange`, `Bound`) come from `truck_geotrait`, which
     truck-topology depends on — import what is not already in scope (edge.rs
     already uses `truck_geotrait::` paths, e.g. `ConcatError`). The edge is
     `Edge<usize, TestCutCurve, i32>`; build vertices with
     `Vertex::news(&[0usize, 1usize])` and the cut vertex with
     `Vertex::new(2usize)`.

9. Also add the two doctests from decision 6 — they run in `--doc` and are
   part of Done-when.

## H-3, the house rule that rejects bare float literals

GATE-2 of `scripts/kernel-gates.sh` fails any **added** line carrying a bare
`1e-N` literal unless that same line ends with an `// H-3` comment. It is a
text gate on the diff: it does not know your literal is a parameter value, and
it does not care that the line is in a test. This packet's tests compare ints,
`Option`s and collection sizes — the only float literals are the `0.5`s and
`0.0`/`1.0` bounds in the test-curve skeleton, none in `1e-N` form, so H-3
should never bite. If you ever do write a bare `1e-N` literal, the line must
end with a same-line `// H-3:` comment naming the dimensionless quantity being
compared. Run `bash scripts/kernel-gates.sh` yourself before you write
`RESULT.json`; it is the same script V4 runs.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-topology
cargo clippy -p truck-topology --all-targets --no-deps
cargo test -p truck-topology --lib --tests --no-fail-fast
cargo test -p truck-topology --doc
cargo check --workspace --all-targets
```

Never run a bare `cargo test` — it builds 56 examples. Send cargo output to a
file and read the tail.

**This crate is clean at baseline** — measured at the tree this packet was
written against: all 7 lib/integration tests pass (one `#[ignore]`d test in
`tests/large-solid-torus.rs` is pre-existing — leave it ignored), all 112
doctests pass, and clippy reports **zero** findings on the whole crate. Your
bar: everything above stays green, plus your four new tests and two new
doctests. There are no baseline failures to tolerate — any failure you did not
cause is a stop condition, and any failure you did cause is yours to fix.

## Forbidden

Editing any file outside `write_allow` — `imported.rs`, `compress.rs`,
`face.rs`, `wire.rs`, `shell.rs`, `solid.rs`, and every other crate
(truck-shapeops, truck-meshalgo, truck-modeling, truck-stepio,
truck-assembly) especially. Changing `PartialEq`/`Eq`/`Hash`/Display/
`DebugDisplay` semantics. Changing any constructor signature — `new`,
`try_new`, `new_unchecked`, `debug_new` keep their exact signatures. Touching
the pre-existing `#[ignore]`d test. Adding `#[ignore]`. Adding `unwrap()`/
`expect()` on fallible paths in production code. Committing to `main`.

## Stop conditions

- an anchor count differs → `ANCHOR_MISMATCH`, naming the file and what you saw
- **a file outside `write_allow` fails to compile after the struct change** →
  `SPEC_GAP`: report the file and the exact error; do not fix it. The
  defaults-in-impl-headers claim this packet is built on has been verified, but
  it is exactly the kind of claim that can be confidently wrong.
- three consecutive failed `cargo` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit on the current branch with subject
`feat(topology): per-use pcurve payload on the edge handle (BG-CE-001)`.
