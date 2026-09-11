# F1 + HYPERCAR FULL COMPLETION — BUILD SPEC

Target: **all 27 F1 rows and all 13 hypercar rows green** under the
kernel-internal predicate (annex C of `MONO_CLOSURE_BOOKING.md`: kernel
constructs + volume certificate bracket + constructive `solid_count` +
carrier-derived `bbox` + mesh emits; OCC references are diagnostics only).

Baseline: R3 census (`docs/TTC_CENSUS_FINAL.md`, HEAD `f827556`,
`door_version 2`). Today: **F1 0/27 green, hypercar 0/13 green.** Every
refusal below is quoted from that census; the row→mechanism mapping is exact.

The door-gap audit (`loop/audits/DOOR_GAP_AUDIT-2026-09-09.md`) remains the
LOC authority; this spec supersedes its sequencing only where R3 moved a row.

---

## 1. Row→mechanism map (complete, from the R3 table)

### F1 — 27 rows in 7 buckets

| bucket | rows | count | blocking carrier (verbatim) |
|---|---|---:|---|
| B1 corners | corner_fl/fr/rl/rr | 4 | drop-in `Face` data row refusal |
| B2 N-station spline loft | beam_wing, drs_flap, power_unit, track_rod_left, track_rod_right | 5 | multi-station spline loft |
| B3 loft-builder DNF | floor, diffuser, cockpit | 3 | `RuntimeError: floor loft failed: None` / `cockpit: loft failed` (builder's `is_valid_shape` swallows the typed refusal) |
| B4 swept-carrier booleans | airbox, details, drivetrain, front_wing, nose, sidepod_left, sidepod_right | 7 | native boolean admission / facts at `_fuse` |
| B5 probe+boolean | monocoque, engine_cover | 2 | OCC-probe data gap + `cut(swept,swept)` / `fuse(swept,swept)` |
| B6 closed-loop loft chain | halo | 1 | closed-loop loft chain |
| B7 trim-extrude | rear_wing, steering_rack, suspension_front, suspension_rear | 4 | trim-extrude constructor |
| — | drs_actuator | 1 | mirror carrier (MONO-3 follow-up; still refusing) |

### Hypercar — 13 rows in 4 buckets

| bucket | rows | count | blocking carrier (verbatim) |
|---|---|---:|---|
| H1 name surface | wheels (`bd.RegularPolygon`), hinge (`bd.Align`), powertrain (`bd.RectangleRounded`) | 3 | drop-in name gap |
| H2 data-row attributes | details (`Face.faces`), lighting (`Face.faces`), suspension_front (`Plane.origin`), suspension_rear (`Edge.edge`) | 4 | drop-in data-row gap |
| H3 spline band loft | aero, body, chassis, glazing, interior | 5 | spline section band (`_xz_band`, `surfaces._mirrored_face`, `chassis._face`) feeding 24–26-station lofts; `aero` refuses `case empty` |
| H4 revolve/profile | brakes | 1 | `revolve needs a closed profile` |

Note the R3 movement already banked: the 2026-09-08 `HYPERCAR_CENSUS`
authoring verbs (`Spline` profiles, `Plane` frames, `Location` placements,
`Plane.XY`, `Vector.normalized`) no longer stop any row — MONO-7/8/9 lifted
them. Hypercar rows now stop at the *same* kernel carriers as F1 (B2/B4) plus
its own name/data-row gaps. **B1/B2/B4 are the whole F1 story; H1/H2 are
mechanical; H3 is B2; H4 is a profile arm.**

---

## 2. The one load-bearing decision

**Resolve the dependency wall via (a): authorize the pyo3 binding program as
the CG program's opening packet.** `truck123d` cannot name `truck-certified`
(STATE session-56 law), so booleans (B4/B5) and kernel-grade smooth-loft facts
(B2/H3) have no lawful in-process route except through pyo3 exports of the
stabilized facade. This is already booked ("deferred behind the CG core",
AGENTS.md) — the booking just needs to become the opening slice. Route (b)
(new manifest edge to truck-evidence) and (c) (per-carrier ports into
bd_bridge) are rejected: (b) forks the facts authority and needs a spec
amendment; (c) cannot match OCC's smooth-loft surface and erodes the
single-substrate doctrine.

---

## 3. Packet graph

Phases are strictly ordered where shown; within a phase, packets are
write-disjoint unless noted. Mechanical bridge packets (door.py / bd_bridge.rs
/ marshal.rs) ride the direct-packet path; anything touching `vendor/truck/**`
rides the packet/worker/`verify.py` loop (see `loop/ORCHESTRATOR.md`).

### Phase 0 — drop-in surface completion (no kernel work, days)

| packet | closes | write set | class |
|---|---|---|---|
| `DP-01-NAME-SURFACE` | wheels, hinge, powertrain reach their real carriers | door.py shim rows: `RegularPolygon`, `Align`, `RectangleRounded` | mechanical |
| `DP-02-DATA-ROWS` | details, lighting, suspension_front, suspension_rear (H2) + corners (B1) reach their carriers | bd_bridge.rs data rows: `Face.faces`, `Plane.origin`, `Edge.edge`, corner `Face` bbox/probe admission; verify `Vector.normalized`/`Plane.XY` answers while in the file | mechanical |
| `DP-03-PROBE-MIRROR-PROFILE` | drs_actuator (mirror), brakes reaches its boolean body, monocoque/engine_cover probe gap | bd_bridge.rs: mirror_y admission for kernel rows; `revolve needs a closed profile` → admit the closed-profile composition brakes authors (or refuse typed naming the exact missing profile verb) | mechanical+ |
| `DP-04-CENSUS-HYGIENE` | honest verdicts for B3 | corpus/ttc: `is_valid_shape`/loft builders must propagate typed refusals instead of raising untyped `RuntimeError` — recorded, never tuned | mechanical |

Expected post-Phase-0 census delta: B1→decided (admit or name the exact
missing carrier), drs_actuator + brakes move or name their real wall, the 7
DNF/untyped sites become typed. **No row goes green yet** — that is correct;
the predicate requires construction, not verdict-class improvements.

### Phase 1 — the binding slice (the wall, the critical path)

| packet | closes | take (audit §1/§2 sizing) | gate |
|---|---|---|---|
| `CG-PYO3-001-BOOL` | airbox, details, drivetrain, front_wing, nose, sidepod×2, monocoque cut/fuse, engine_cover, body, glazing, powertrain (B4/B5 + H3 rows' boolean tails) | pyo3-export the certified funnel boolean entry over the stabilized facade; flip shim `cut`/`fuse`/`intersection` stubs to record boolean rows + submit operands (~100–150 LOC); placed-operand convention in bd_bridge (~100 LOC) | rides `CG-000` contract freeze |
| `CG-PYO3-002-LOFT-FACTS` | beam_wing, drs_flap, power_unit, track_rod×2, floor, diffuser, cockpit, aero, chassis, interior, halo (B2/B3/B6 + H3) | smooth N-station spline-loft facts arm: submit the section carrier set to the kernel volume row, marshal the certified bracket (~200–400 LOC beyond the binding). Surface semantics = the pinned canonical convention (MONO annex A correction: exact C2 across chord-length station parameters, global degree N−1 for N≤9, C2 cubic knots-at-stations for N≥10). OCC references adjudicate as diagnostics only | rides `CG-PYO3-001` |

Pre-made decision required by `CG-PYO3-002`: the sweep/loft **frame transport
law** (fixed-order, exact interpolation of the frame) is pinned before coding —
a small CG-frame-law slice, not the whole program (audit item 4).

**Admission widening is expected work, not failure.** The funnel's first pass
admits ruled-section pairs; spline-section pairs refuse typed at ADM admission
until ADM-002's certificate dispatch covers them. Each such refusal books the
widening. Where a widening needs certified interval evaluation, it rides
**`BG-ENC-001`** — per `scratch/state9b.md` the single most load-bearing
unbuilt item in the spec — so **`BG-ENC-001` is on this program's critical
path and should dispatch before or alongside Phase 1.**

### Phase 2 — trim-extrudes (the CG realization milestone)

B7 (`rear_wing`, `steering_rack`, `suspension_front/rear`) is audit item 5,
the DEEP END: the direct facet realization backend (shared-topology
`PolygonMesh`, no sewing, no welding) — i.e. the CG program's core chain
`CG-000 → CG-001 → CG-002/003 → CG-004 → CG-009` per
`CONSTRUCTIVE_GEOMETRY_PLAN.md` §4/§6. No shortcut port: a trimming
approximation poisons the facts gates. These 4 rows stay honestly typed until
realization lands; they are the program's midpoint deliverable, not extra
scope. `BG-CG-*` packets run through the worker loop with V0–V10 plus the §7
program invariants (no welding, integer identity, determinism, C¹ refusals,
winding-audit FAILED, existing entry points bit-identical).

### Phase 3 — R4 census gate (completion proof)

Re-run all 40 rows, one fresh process per row, serial, quiet machine, release
`truck123d.pyd` staged at the landing HEAD, `door_version 2`:

```console
python corpus/ttc/door.py --engine truck corpus/ttc/trees/<family>/src <module> <entry> <args> <stl>
```

Green predicate per annex C. A row is done when: constructs, bracket present
(lo == hi or an honestly widened bracket), `solid_count` constructive, bbox
carrier-derived, STL emits. Reported columns: solids, volume, bracket, STL
tris, OCC delta (diagnostic). Anchors: `grep -c '"id"' corpus/ttc/MANIFEST.json`
= 54 (40 of them F1+hypercar); manifest + SKIPS validate.

---

## 4. Row-closure accounting (the booking gate, annex B)

Program booking is valid iff the union of packets covers every gap cell:

| mechanism | packets | rows closed |
|---|---|---|
| name surface | DP-01 | 3 |
| data rows | DP-02 | 4 (+4 F1 corners reach adjudication) |
| mirror / profile / probe | DP-03 | 1 direct; unblocks brakes, monocoque, engine_cover |
| booleans via binding | CG-PYO3-001 | 7 (B4) + 2 (B5) + H3 boolean tails |
| smooth loft facts via binding | CG-PYO3-002 | 5 (B2) + 3 (B3) + 1 (B6) + 5 (H3) |
| trim-extrude realization | BG-CG chain | 4 (B7) |

27 F1 + 13 hypercar, every row mapped, no gap cell uncovered.

## 5. Sequencing and constraints

1. **`BG-ENC-001` first or parallel** — Phase 1's admission widening rides it.
2. Phase 0 in full (parallel-eligible; DP-01/02/04 are write-disjoint, DP-03
   shares bd_bridge.rs with DP-02 — serial those two).
3. `CG-PYO3-001` then `CG-PYO3-002` (both sides of one binding surface).
4. Phase 2 CG chain: concurrency capped at ≤3 live packets, write-set-disjoint
   set only; CG-009 enum ripple runs effectively alone.
5. Phase 3 census.
6. **Disk is the binding constraint** (~11 GB free, 8 GB harness floor): run
   `python loop/slot_status.py --disk` before every verify session; verify
   serially and delete between.
7. **Off-path but adjacent:** the BG-TOL-001 Stage A tail (`BG-TOL-004`, the
   `tessellation/formal` census blind spot), V8's negative test, `BG-EVD-004-r2`
   (live soundness defect — dispatch on its own merit) do not gate any row
   here; the BG-CE chain gates generation, not census green.
8. Honesty line (unchanged): no tolerance stretch, no approximant replication
   inside the kernel; refusals stay typed and carry the measured delta.
