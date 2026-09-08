# Excluded-six staging annex — demand map and theory wiring

**Status:** authored 2026-09-08 (orchestrator), companion to
[`SWEPT_PAIR_ADMISSION_SPEC.md`](SWEPT_PAIR_ADMISSION_SPEC.md) (theorems
A–D). Every code citation below was measured against the tree at
integration `34f0042`+. This annex corrects two claims made in conversation
before it was written: (1) `surfaces.styled` is metadata, not geometry;
(2) the fillet op is **LANDED and certified** — the orchestrator's earlier
"blend representation cannot close" argument was wrong (it applied
OCC-style rolling-ball reasoning to the landed spine-based blend program;
see `docs/OP_CAPABILITY_MATRIX.md` row 14).

## 1. What the six models actually are

| Model | Script | Real geometry demand |
|---|---|---|
| front_wing | `trees/f1/src/lib/front_wing.py` | 5 wing elements (`surfaces.wing_element` = "loft through spanwise stations"), 5 swept plates, blade path, NACA sections, 15 cuts |
| cockpit | `cockpit.py` | 15 rounded-plate sample sections, superellipse points, 6 `section_face`, 3 `body_loft`, 15 cuts. ("styled ×51" is `surfaces.styled` = "label + colour one body" — pure metadata, zero geometry.) |
| nose | `nose.py` | 9 `body_loft`, 4 `half_section_face`, superellipse sections, 2 blade members, 2 cuts — the monocoque's authoring class |
| sidepods | `sidepods.py` | 26 cuts, 9 `section_face`, 6 `body_loft`, 2 `mirror_y`, 1 `fuse`, 1 `safe_fillet` (LANDED op, see §3) |
| halo | `mono_halo.py` | "one continuous titanium loop on a central blade pillar" — a closed swept loop over station frames |
| cooling | **no script** | cooling content lives inside engine_cover/monocoque; not a standalone row. Drops out of the six. |

None has a manifest row, an OCC reference, or a kernel-door attempt — they
were excluded at booking, before the reference program ran. Staging them is
mechanical (§5).

## 2. The refusal sites the authoring arm must replace (all in `corpus/ttc/door.py`)

Every handler the six models need currently refuses at these measured
lines:

| Census verb | Line | Refusal text |
|---|---|---|
| `loft` | 441 | "loft is not a kernel-engine row" |
| `sweep` | 437 | "sweep is not a kernel-engine row" |
| `make_face` | 424 | (records; wire-carrier checks downstream) |
| `mirror` | 453 | "mirror is not a kernel-engine row" |
| `revolve` (spline profile) | 409 | "a spline-profile revolve is not a kernel-engine row" — already owned by the **FH-SPLINE-LATHE** packet (booked, verified) |
| `fillet` | 445 | "fillet is not a kernel-engine row" — the DOOR lacks the arm; the KERNEL side is landed (§3) |

`make_spline` already records (line 185). The authoring-arm packet extends
the drop-in's recording at these sites and the bridge's `SolidSpec` enum
(`truck123d/src/bd_bridge.rs:58`) with the corresponding variants; the
dispatch plumbing is the landed `run_facade` classifier
(`truck123d/src/facade.rs:484`) behind the same V5 rule (canonical dispatch
stays ahead; new arms fire only where the old path returned
`NonCanonicalCarrier`).

## 3. What each demand consumes from landed/spec'd machinery

- **Lofts (`body_loft`, `wing_element`, swept plates).** Landed loft
  theory: `construct/loft.rs`, `loft_strips.rs`, `loft_validity.rs`,
  `loft_weights.rs`, `gordon.rs` (loft through spline surfaces) — survey
  a-bucket. The resulting faces are tensor-spline patches → theorems A–D
  pipeline (`SWEPT_PAIR_ADMISSION_SPEC.md` §2–3): extraction adapter →
  transversality gate → `Ssi4System` continuation → trim → volume.
- **Section faces / half-section faces / superellipse & rounded-plate
  sections.** Sampled points → interpolating splines (exact data;
  convention derivation owned by FH-SPLINE-LATHE's scope for revolve, same
  method here) → planar faces with spline-trimmed boundaries. The face
  SURFACE is elementary (plane) — canonical side; the spline lives in the
  trim curves.
- **Cuts/fuses.** Landed for canonical tools (2-D path, PB-011B's 12 rows);
  swept×swept = the admission program's census rows.
- **`safe_fillet` (sidepods).** **LANDED, certified, matrix row 14** — the
  CC program's spine-based blend machinery
  (`construct/blend.rs`, `blend_varradius.rs`, `setback.rs`,
  `offset_strata.rs`, `canal.rs`). The matrix stages the fillet row only
  because its enclosing part boolean-composes swept carriers. Consequence:
  **no fillet theory gap, no fillet decision** — sidepods is blocked on the
  same admission as everything else.
- **`mirror_y` (sidepods).** Transform placement — the landed
  `Placed`/processor carrier rule (BG-CE-006-r2); per-segment mirrors of
  spline solids produce placed tensor-spline faces → same admission.
- **Halo (closed swept loop).** The sweep is authored over DISCRETE
  stations with per-station frames (`mono_halo.py::_stations`, `_frame`).
  Between consecutive stations the swept surface is the linear
  interpolation of two transformed sections — i.e. **a loft between two
  station sections**: the sweep reduces to a CHAIN OF LOFTS. Theory needed:
  the bridge records stations+frames as a loft chain (one authoring arm),
  per-segment loft admission (same as above), and LOOP CLOSURE — the chain
  is closed, certified by T1′'s exact seam identification
  (`A₀W₁ − A₁W₀ ≡ 0` on aligned meeting edges). No sweep solver; the
  refusal at `door.py:437` is replaced by a recording arm that emits the
  loft chain.

## 4. Theory delta for the six (authoritative list)

1. The admission program (theorems A–D) — booked, spec committed.
2. The authoring-arm packet — booked; its scope grows by the loft/sweep/
   mirror recording arms measured in §2 (the revolve arm is
   FH-SPLINE-LATHE's, independent).
3. The halo closure certificate — T1′ seam identification applied to a
   loft chain; no new solver.
4. **No fillet decision** — the op is landed; sidepods waits on admission.
5. Staging — no theory.

## 5. Staging protocol (mechanical)

1. Manifest rows for the five real scripts (+ decision: cooling stays
   un-rowed as content of other models).
2. OCC references via the reference-batch machinery (door runs fan out
   4–8 wide; seconds-to-minutes each).
3. Kernel-door census sweep (the PB-011C protocol) — per-row, per-op
   verdict table; typed refusals are valid outcomes.
4. Rows flip from typed-refusal to certified-lift only with facts matching
   the recorded references — the same acceptance instrument as the main
   program.

## 6. Owner decisions requested

1. Admit the five real scripts to staging (cooling drops out — confirm).
2. Authorize the authoring-arm packet's extended scope (§2's five refusal
   sites + halo loft-chain arm).
3. Confirm fillet handling: no action (landed op; blocking issue is
   admission), per §3.
