# BG-CK-P0-CRATE — promote tessellation/{formal,domain} into the new truck-certified crate

Certified-kernel plan Phase 0 (D1), first packet. The certified substrate
lives in `truck-meshalgo/src/tessellation/{formal,domain}` behind
`#![allow(dead_code, unused)]`, but its consumers are truck-geometry
(class 1), truck-shapeops (classes 2, 4) and future feature crates; a mesh
crate cannot be a dependency of a geometry crate. This packet performs the
promotion exactly as the plan D1 prescribes, with every structural decision
pre-made below. **The move is verbatim: no semantic change of any moved line.
The Phase-0 gate is "workspace builds, all existing suites unchanged."**

```yaml
id:          BG-CK-P0-CRATE
contract:    [BG-CK-P0-CRATE]
class:       mechanical
crates:      [truck-certified, truck-meshalgo]
depends_on:  []
write_allow:
  - vendor/truck/truck-certified/**
  - vendor/truck/truck-meshalgo/src/tessellation/mod.rs
  - vendor/truck/truck-meshalgo/src/tessellation/source_evidence.rs
  - vendor/truck/truck-meshalgo/Cargo.toml
  - Cargo.toml
  - Cargo.lock
read_allow:
  - CERTIFIED-KERNEL-PLAN.md
  - docs/CERTIFICATE_MAPPING.md
  - vendor/truck/truck-meshalgo/src/tessellation/mod.rs
  - vendor/truck/truck-meshalgo/src/tessellation/formal/**
  - vendor/truck/truck-meshalgo/src/tessellation/domain/**
  - vendor/truck/truck-meshalgo/src/tessellation/source_evidence.rs
  - vendor/truck/truck-meshalgo/src/tessellation/triangulation.rs
  - vendor/truck/truck-meshalgo/src/tessellation/triangulation_with_ledger.rs
  - vendor/truck/truck-meshalgo/src/tessellation/diagnosis.rs
  - vendor/truck/truck-meshalgo/src/lib.rs
  - vendor/truck/truck-meshalgo/Cargo.toml
  - vendor/truck/truck-meshalgo/tests/ledger_identity.rs
  - tests/torus_deck.rs
  - Cargo.toml
budget:      {turns: 40, ctx_tokens: 120000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub mod formal;' vendor/truck/truck-meshalgo/src/tessellation/mod.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub mod domain;' vendor/truck/truck-meshalgo/src/tessellation/mod.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'pub mod source_evidence;' vendor/truck/truck-meshalgo/src/tessellation/mod.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'pub trait PreMeshableSurface' vendor/truck/truck-meshalgo/src/tessellation/mod.rs"}
  - {id: A5, expect: 1, cmd: "grep -c 'pub trait MeshableSurface' vendor/truck/truck-meshalgo/src/tessellation/mod.rs"}
  - {id: A6, expect: 12, cmd: "grep -rh --include=*.rs 'use crate::tessellation' vendor/truck/truck-meshalgo/src/tessellation/formal vendor/truck/truck-meshalgo/src/tessellation/domain | wc -l"}
  - {id: A7, expect: 6, cmd: "grep -rh --include=*.rs 'use crate::cgmath' vendor/truck/truck-meshalgo/src/tessellation/formal vendor/truck/truck-meshalgo/src/tessellation/domain | wc -l"}
  - {id: A8, expect: 1, cmd: "grep -c 'use super::\\*;' vendor/truck/truck-meshalgo/src/tessellation/source_evidence.rs"}
  - {id: A9, expect: 0, cmd: "grep -c 'truck-certified' Cargo.toml"}
  - {id: A10, expect: 14, cmd: "grep -rh --include=*.rs 'use super::super::super::source_evidence' vendor/truck/truck-meshalgo/src/tessellation/formal | wc -l"}
```

## What this packet is

D1 of `CERTIFIED-KERNEL-PLAN.md`: new workspace crate `truck-certified`; move
`tessellation/formal` + `tessellation/domain` (and `source_evidence.rs` — the
two moved band test modules and the two planar test modules import it; see
Section 3) into it; `truck-meshalgo` consumes it through compat re-exports so
every existing path — including look's own
`truck_meshalgo::tessellation::formal::*` uses in `src/step/*` and
`tests/torus_deck.rs` — keeps resolving. **`truck-geometry` must NOT gain a
dependency on `truck-certified`** (D1's no-reintroduced-cycle rule) — and it
will not: this packet adds exactly one new manifest edge,
`truck-meshalgo → truck-certified`.

Every moved file moves verbatim except the mechanical import rewrites
enumerated in Section 4. If you find yourself editing anything else inside a
moved module, STOP (stop conditions).

## Section 1 — the new crate (verbatim skeleton, pre-made)

`vendor/truck/truck-certified/`:

- `Cargo.toml`:

```toml
[package]
name = "truck-certified"
version = "0.1.0"
edition = "2021"
description = "Certified constructive geometry substrate: formal pipeline, quotient domain, evidence."
homepage = "https://github.com/ricosjp/truck"
repository = "https://github.com/ricosjp/truck"
license = "Apache-2.0"

[dependencies]
cgmath = { version = "0.18.0", features = ["serde"] }
robust = "1.1"
spade = "2.15.1"
truck-base = { version = "0.5.0", path = "../truck-base" }
truck-geotrait = { version = "0.4.0", path = "../truck-geotrait" }
truck-geometry = { version = "0.5.0", path = "../truck-geometry" }
truck-polymesh = { version = "0.6.0", path = "../truck-polymesh" }
truck-topology = { version = "0.6.0", path = "../truck-topology" }
```

  (cgmath version matches truck-base's pin; robust and spade match the
  versions truck-meshalgo pins. The moved tree names truck-geometry,
  truck-topology, truck-polymesh, robust, spade directly; truck-base and
  truck-geotrait are booked per plan D1's letter. If rustc proves one of
  base/geotrait unused, record it in RESULT notes — do not silently drop it,
  do not add anything else.)

- `src/lib.rs`: copy truck-meshalgo's `lib.rs` lint header VERBATIM (the
  `#![cfg_attr(not(debug_assertions), deny(warnings))]`, the
  `#![deny(clippy::all, rust_2018_idioms)]` and the same `#![warn(...)]`
  list), then declare:

```rust
pub mod domain;
pub mod formal;
pub mod meshable;
pub mod source_evidence;
```

- `src/formal/**` and `src/domain/**` — moved verbatim with the Section 4
  import rewrites only. The `#![allow(dead_code, unused)]` module attribute
  at the top of `formal/mod.rs` MOVES WITH IT untouched (retiring it is
  per-consumer work the plan books for later phases, not this packet).
- `src/meshable.rs` — NEW file holding the two trait definitions lifted
  verbatim out of `tessellation/mod.rs` (the `PreMeshableSurface` and
  `MeshableSurface` trait + their blanket impls, with doc comments). They
  move because `domain/projection.rs` bounds a generic on `PreMeshableSurface`
  and a certified-side module cannot name a meshalgo trait (orphan rule; D1
  forbids certified→meshalgo). `RobustMeshableSurface` and
  `PolylineableCurve` STAY in meshalgo. `meshable.rs` also carries the
  `Parallelizable` shim copied VERBATIM from `tessellation/mod.rs` lines 5–19
  (the cfg'd rayon/no-op pair plus `pub use parallelizable::*;`) — the moved
  trait bounds need it, and it must stay a no-op on wasm exactly as today.
  (Two blanket `Parallelizable` traits coexist harmlessly: both are
  blanket-impl'd for every type.)
- `src/source_evidence.rs` — moved verbatim (Section 3).

## Section 2 — meshalgo compat surface (exact)

`truck-meshalgo/src/tessellation/mod.rs`:

1. Delete the `PreMeshableSurface` and `MeshableSurface` trait definitions
   and their blanket impls (they moved to `truck_certified::meshable`).
2. Replace the three module declarations with re-exports:

```rust
pub use truck_certified::meshable::{MeshableSurface, PreMeshableSurface};
pub use truck_certified::{domain, formal, source_evidence};
```

   Nothing else in the file changes. Every sibling path keeps resolving:
   `triangulation.rs`'s unqualified `formal::…` references, `diagnosis.rs`,
   `use super::…` sites, the `prelude` glob (`pub use crate::tessellation::*;`
   in meshalgo's lib.rs), look's `truck_meshalgo::tessellation::formal::*`
   uses and `tests/ledger_identity.rs` all ride the re-exports. Do NOT touch
   `triangulation.rs`, `triangulation_with_ledger.rs`, `diagnosis.rs`,
   `source_edge.rs`, `realization_evidence.rs`, `validity.rs`, or
   `tests/ledger_identity.rs` — if one of them fails to compile, that is a
   stop condition, not an edit invitation.

`truck-meshalgo/Cargo.toml`: add
`truck-certified = { version = "0.1.0", path = "../truck-certified" }` to
`[dependencies]`. Nothing else changes there.

Root `Cargo.toml`: add `"vendor/truck/truck-certified",` to the workspace
`members` list. Nothing else (no `[patch.crates-io]` entry: the name is not
published upstream, same as truck-evidence). `Cargo.lock` regenerates from the
build; commit the updated lock.

## Section 3 — source_evidence.rs moves with the tree

The plan's letter moves `formal/` + `domain/`, but four moved test modules
import `source_evidence` (`formal/cone_band/tests.rs`,
`formal/cylinder_band/tests.rs` via `crate::tessellation::source_evidence`;
`formal/planar_holes/tests.rs`, `formal/planar_slice/tests.rs` via
`super::super::super::source_evidence`), and a certified-side module cannot
name a meshalgo path. Pre-made decision: `source_evidence.rs` moves into
`truck-certified/src/` (it is representation-reading substrate — exactly what
the certified layer owns), and meshalgo re-exports it (Section 2) so
`triangulation.rs` and look's `src/step/*` keep their paths.

One non-verbatim line inside it: its `use super::*;` (line 1) must become the
explicit import set rustc demands (`super` is now the certified crate root,
whose pub items are the four modules — nothing like what tessellation/mod.rs
exported). Replace it with explicit `use` lines; record the exact set in
RESULT.json notes. If the demanded set reaches for something the certified
crate cannot name, STOP — that is stop condition 2.

## Section 4 — the complete import-rewrite table (measured, verbatim census)

All 12 `use crate::tessellation` sites in the moved tree (anchor A6 = 12):

| File | Now | Becomes |
|---|---|---|
| formal/ambient.rs | `use crate::tessellation::domain::lattice::{Axis, AxisPeriodStatus, CertifiedLattice};` | `use crate::domain::lattice::{Axis, AxisPeriodStatus, CertifiedLattice};` |
| formal/torus_circle.rs | `use crate::tessellation::formal::torus::{identify_torus_world, TorusIdentification};` | `use crate::formal::torus::{identify_torus_world, TorusIdentification};` |
| formal/cone_band/tests.rs | `use crate::tessellation::formal::cone::{…}` | `use crate::formal::cone::{…}` |
| formal/cone_band/tests.rs | `use crate::tessellation::formal::support::identify_line_segment;` | `use crate::formal::support::identify_line_segment;` |
| formal/cone_band/tests.rs | `use crate::tessellation::source_evidence::{…}` | `use crate::source_evidence::{…}` |
| formal/cylinder_band/tests.rs | `use crate::tessellation::formal::curve_witness::CompleteCirclePlacement;` | `use crate::formal::curve_witness::CompleteCirclePlacement;` |
| formal/cylinder_band/tests.rs | `use crate::tessellation::formal::cylinder::{…}` | `use crate::formal::cylinder::{…}` |
| formal/cylinder_band/tests.rs | `use crate::tessellation::formal::support::identify_line_segment;` | `use crate::formal::support::identify_line_segment;` |
| formal/cylinder_band/tests.rs | `use crate::tessellation::source_evidence::{…}` | `use crate::source_evidence::{…}` |
| domain/projection.rs | `use crate::tessellation::domain::deck::LatticePotential;` | `use crate::domain::deck::LatticePotential;` |
| domain/projection.rs | `use crate::tessellation::{MeshableSurface, PreMeshableSurface};` | `use crate::meshable::{MeshableSurface, PreMeshableSurface};` |
| domain/adapters/revolution.rs | `use crate::tessellation::domain::schema::{…}` | `use crate::domain::schema::{…}` |

Plus 14 further sites NOT in the A6 count — relative paths my `crate::`
census cannot see (anchor A10 = 14): every
`use super::super::super::source_evidence::…` line becomes
`use crate::source_evidence::…`. The textual substitution is uniform:
`use super::super::super::source_evidence` → `use crate::source_evidence`
(the moved tree keeps its internal shape, so `super::super` forms that stay
inside `formal/` are correct as-is and must NOT be touched). The 14 sites:
formal/{bezier.rs:386, bezier_isect.rs:1360, common_arc.rs:2019,
contact.rs:675, cylinder_face.rs:80, cylinder_lift.rs:378,
cylinder_mesh.rs:117, intersection.rs:2181, planar_developed.rs:698,
quotient.rs:746, span.rs:197, xmonotone.rs:912, planar_holes/tests.rs:10,
planar_slice/tests.rs:9}.

All 6 `use crate::cgmath::…` sites in `domain/` (anchor A7 = 6; files:
ambient.rs, plan.rs, projection.rs, quotient.rs, schema.rs,
adapters/revolution.rs) become `use cgmath::…` — the certified crate takes a
DIRECT cgmath dependency (Section 1 manifest). Background, so the rewrite
reads as sane rather than mysterious: inside truck-meshalgo, `crate::cgmath`
resolves only through a four-hop glob chain
(`matext4cgmath` does `pub use cgmath;` → `truck_base::cgmath64::*` →
`truck_polymesh::base::*` → meshalgo's `use truck_polymesh::{…, *}` glob).
That chain is an accident of the host crate; do not replicate it.

Everything else in every moved file is byte-identical to its source. Use
`git mv` (or equivalent) so renames stay traceable.

## Done-when

- `cargo fmt --all -- --check` clean.
- `cargo clippy -p truck-certified -p truck-meshalgo --all-targets
  --message-format=short --no-deps` — zero findings (the moved code was
  clippy-gated inside meshalgo; it should arrive clean).
- `cargo test -p truck-certified --lib --tests --no-fail-fast` green — the
  moved modules' own test modules (cone_band, cylinder_band, planar_holes,
  planar_slice, and every other `#[cfg(test)]` in the tree) are the test
  contract; they must pass unchanged in their new home.
- `cargo test -p truck-meshalgo --lib --tests --no-fail-fast` green —
  unchanged suites, including `tests/ledger_identity.rs`.
- `cargo check --workspace --all-targets` green (covers look's own
  `src/step/*` consumers and every downstream crate).

## Stop conditions

Stop, commit nothing beyond WIP evidence, write RESULT.json (AT THE WORKTREE
ROOT) with the finding verbatim if:

1. A staying meshalgo file (`triangulation.rs`, `triangulation_with_ledger.rs`,
   `diagnosis.rs`, `validity.rs`, `source_edge.rs`,
   `realization_evidence.rs`, `tests/ledger_identity.rs`) fails to compile
   against the compat re-exports — name the path and the missing item; do NOT
   edit it (it is outside write_allow by design).
2. `source_evidence.rs`'s explicit-import set (Section 3) reaches for anything
   the certified crate cannot name without new machinery.
3. A moved module needs a change beyond Section 4's table to compile.
4. The workspace check surfaces a consumer outside the two crates and the
   root `Cargo.toml` (look's `src/step/*` should ride the re-exports).

Deviations are expected to be small and mechanical; record every one in
RESULT.json notes with the derivation.

## Finish by writing `RESULT.json` AT THE WORKTREE ROOT

Commit your work on the current branch (subject: `feat(certified): promote
formal/+domain/ into truck-certified (BG-CK-P0-CRATE)`) BEFORE writing
`RESULT.json`. There are no new tests; the contract is that every existing
suite passes unchanged, and the moved tests must exist in the new crate with
their landed names verbatim.
