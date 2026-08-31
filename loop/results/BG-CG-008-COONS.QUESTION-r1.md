# QUESTION.md — BG-CG-008-COONS (SPEC_GAP)

## What I attempted

Transcribed the quoted `CoonsSurface` design verbatim into
`vendor/truck/truck-geometry/src/decorators/coons.rs`, added
`mod coons;` + `pub use coons::CoonsSurface;` to `decorators/mod.rs`, and ran
`cargo check -p truck-geometry`. The struct, constructors, the quoted
evaluation/derivative formulas, `der_mn` as single source of truth,
`jacobian`, the accessors, and every trait on the checklist were written
exactly as PACKET.md quotes them. The build fails on two independent, hard
type-system conflicts. I stopped and reverted the two source edits so the
workspace stays buildable.

## Exact conflict 1 — `IncludeCurve` for each of the four boundary curve parameters

The checklist (plan §3.7) requires four impls on the same generic struct:

```
impl<C0, C1, D0, D1> IncludeCurve<C0> for CoonsSurface<C0, C1, D0, D1>
impl<C0, C1, D0, D1> IncludeCurve<C1> for CoonsSurface<C0, C1, D0, D1>
impl<C0, C1, D0, D1> IncludeCurve<D0> for CoonsSurface<C0, C1, D0, D1>
impl<C0, C1, D0, D1> IncludeCurve<D1> for CoonsSurface<C0, C1, D0, D1>
```

All four have the identical Self shape `CoonsSurface<_, _, _, _>` and
pairwise-unifiable trait arguments (substituting `C0 := C1 := T` makes the
first two impls identical; likewise `C1 := D0 := T`, etc.). Rust coherence
rejects any pair: `error[E0119] conflicting implementations of trait
IncludeCurve for type CoonsSurface<_, _, _, _>`. Where-clauses do not
participate in the overlap check, and Rust has no way to bound
"C0 ≠ C1 ≠ D0 ≠ D1". I verified the minimal case (`impl T<A> for S<A,B>` +
`impl T<B> for S<A,B>`) with a standalone `rustc` file: E0119 reproduces in
isolation. No sibling in the tree implements `IncludeCurve` on a
multi-curve surface, so there is no worked reference that avoids this.

## Exact conflict 2 — `Invertible` on the generic four-param struct

The quoted contract is `self.inverse().subs(u, v) == self.subs(1.0 - u, v)`
pointwise. Deriving the boundary assignment from the quoted formula gives
`left' = right` (the old `D1` curve becomes the new `D0` field) and
`right' = left` (the old `D0` curve becomes the new `D1` field). Assigning a
`D1`-typed value into the `D0`-typed `left` field (and vice versa) is
`error[E0308]` unless `D0 == D1`. The packet names this "the single
derivation duty", but no derivation can change types: the generic impl
`impl<C0, C1, D0, D1> Invertible for CoonsSurface<C0, C1, D0, D1>` cannot
compile; only `impl<C0, C1, D> Invertible for CoonsSurface<C0, C1, D, D>`
is well-typed and preserves the contract exactly.

(Minor, non-gap: the quoted `-> Result<Self, ConstructError>` collides with
the crate's own 1-argument `pub type Result<T>` alias in `errors.rs`
(E0107); the fix `std::result::Result` is a transcription detail, not the
gap.)

## Readings I could not choose between

1. **Restrict both impls to the equal-type subcase.** Write `Invertible` for
   `CoonsSurface<C0, C1, D, D>` and a single `IncludeCurve<C>` for
   `CoonsSurface<C, C, C, C>` (returning `true` iff the curve equals any
   stored boundary), note the narrowing in RESULT.json, and land DONE. This
   compiles, keeps all 7 tests green (test 6 uses an all-`Line` quad, so it
   builds), and keeps the geometry contract exact — but it implements the
   checklist item only for homogeneous patches, not "for each of the four
   boundary curve parameters" on the generic struct, which is the literal
   requirement.
2. **Implement `IncludeCurve` only for one boundary parameter** (e.g.
   `IncludeCurve<C0>`/bottom) and leave the other three out — a silent,
   incomplete checklist.
3. **Report SPEC_GAP** — the quoted design cannot compile as specified
   (the stop-condition trigger). I chose this, since PACKET.md forbids
   redesign and instructs to stop and report rather than improvise.

## What I ask

Which reading should land: (1) the equal-type-subcase restriction with the
deviation noted (I believe this is the only compilable completion), or (3)
SPEC_GAP as the intended outcome? If (1), also confirm the `Invertible`
bound narrowing to `CoonsSurface<C0, C1, D, D>` and the `IncludeCurve`
single homogeneous impl as the accepted shape.
