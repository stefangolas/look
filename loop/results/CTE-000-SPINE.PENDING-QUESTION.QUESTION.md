# QUESTION — CTE-000-SPINE ANCHOR_MISMATCH

Status written alongside `RESULT.json`: `ANCHOR_MISMATCH`.

## What the packet's stop condition requires

PACKET.md ("Anchors"): *"Locate by pattern, never by line number. If a count
differs, STOP and report `ANCHOR_MISMATCH` with what you saw."*

Two anchors differ from the table's expectation on the dispatched tree
(branch `packet/CTE-000-SPINE`, HEAD `4a1738f`, clean tree; only the
harness-provided `PACKET.md`/`CONTEXT.md` are untracked). No source file was
touched.

## A4 — `truck-certified/src/lib.rs`, pattern `^pub mod`

- Expected: `13`
- Saw: `16` occurrences:
  - L17 `pub mod certified_map;`
  - L20 `pub mod construct;`
  - L21 `pub mod contract;`
  - L22 `pub mod domain;`
  - L23 `pub mod formal;`
  - L24 `pub mod hull;`
  - L27 `pub mod interval;`
  - L28 `pub mod kernel;`
  - L29 `pub mod meshable;`
  - L30 `pub mod pair_dispatch;`
  - L34 `pub mod patch_admit;`
  - L35 `pub mod source_evidence;`
  - L36 `pub mod ssi;`
  - L38 `pub mod ssi_fixtures;` (`#[doc(hidden)]`)
  - L39 `pub mod ssi_trace;`
  - L40 `pub mod ssi_types;`
- The packet note *"A4 becomes 14 when you add `pub mod tangency;`"* is not
  reachable from this HEAD: adding it would make `17`.

## A5 — `truck-certified/src/kernel/rational.rs`, pattern `cone_torus_carrier_packet_pending`

- Expected: `1`
- Saw: `2` occurrences:
  - L64 module doc: `//! pending refusal `cone_torus_carrier_packet_pending`; ...`
  - L84 code: `const CONE_TORUS_PENDING: Reason = "cone_torus_carrier_packet_pending";`

## A1/A2/A3 — matched

- A1 `pub struct SquareSystem3` in `ssi_types.rs`: `1` (L75) — match.
- A2 `pub struct CertifiedInterval` in `formal/exact.rs`: `1` (L227) — match.
- A3 `pub struct Expansion` in `formal/exact.rs`: `1` (L63) — match.

## Why not proceed

The counts differ, so the packet instructs a hard stop and an `ANCHOR_MISMATCH`
report rather than building against stale anchors. Implementing the shapes,
fixture kit, `lib.rs` module line, and `CERTIFICATE_MAPPING.md` under a wrong
anchor baseline would bake the mismatch into the write-set and later packets.

## Requested decision

Re-base the packet's anchor table (and the "A4 becomes 14" note) against this
HEAD — A4 expected `16` (→ `17` with `pub mod tangency;`), A5 expected `2`
or the A5 pattern restricted to the code declaration (`const CONE_TORUS_PENDING
: Reason = "cone_torus_carrier_packet_pending"` = `1`) — then re-dispatch.
