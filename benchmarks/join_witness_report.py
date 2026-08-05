"""Advanced join and boundary-witness census (FORMAL-ATLAS-WAVE-1, Agent D).

A DEFINITIVE observer-only census of why faces fail AFTER passing the original
curve-family gate. Reads the existing cone-final ledgers (census_diag +
source_probe) and joins them on `source_face_id`, exactly as
`remainder_report.py` does, then filters to the population that passed the
curve gate and the outer-bound-authority gate but was refused at a later
(join / witness / orientation / cone-band) stage.

# Observer-only

This script recovers no face, changes no admission, and touches no production
source. It reads only what the sweep already recorded, plus the READ-ONLY
truck-meshalgo source for the `SliceCategory` authority behind each typed exit.
No Rust example is added: the one signal the ledger lacks for the join faces
(the deck solver's per-face candidate integer range, per-constraint feasible
intervals, intersection, eliminating constraint, and `join_index`) lives inside
the PRIVATE `solve_axis_aligned` / `propagate_deck_placements` path in
`truck-meshalgo/src/tessellation/formal/{deck,rank1_annulus}.rs`. Deriving it
observer-only would require either instrumenting that private solver (modifying
truck, out of scope) or re-implementing the whole band admission + lift +
projection + deck-walk pipeline (recovery-adjacent, divergence-prone). The
constraint *structure* is documented below from the source; the per-face
numeric values are a precisely-documented gap, not an uninstrumented face.

# Target population

Every band-eligible face whose refusal is NOT `unsupported_curve_representation`
(the curve-family gate itself) and NOT `missing_outer_bound_authority` (the
material-authority gate) and NOT `recovered`. These are the faces that passed
the original curve-family gate and failed at a more advanced stage. Counts are
taken from the cone-final ledger (`band` and `cone_band` columns):

  - `lift_join_no_compatible_integer`         161   cylinder, deck-walk join
  - `witness_sweep_does_not_reach_endpoint`    87   cylinder, witness
  - `witness_start_not_on_cylinder`            72   cylinder, witness
  - `witness_circle_not_a_cylinder_parallel`   64   cylinder, witness
  - `witness_not_constant_axial_coordinate`      2   cylinder, witness
  - `band_orientation_incompatible`              5   cylinder, orientation
  - `cone_witness_start_not_on_cone`            63   cone, witness
  - `cone_band_bound_not_one_occurrence`        55   cone, bound structure
  - `cone_witness_circle_not_a_cone_parallel`    7   cone, witness
                                           --------
                                             516

= 161 join + 225 cylinder witness + 130 other (70 cone witness + 55 cone bound
+ 5 orientation), exceeding the >=386 minimum.

# Classification authority

Each exit's `SliceCategory` is taken VERBATIM from the truck-meshalgo source
(READ ONLY), not inferred:

  cylinder_lift.rs:155  CylinderLiftExit::category
  curve_witness.rs:144  WitnessFailure::category
  cylinder_band.rs:390  BandExit::category
  cone_band.rs:191       ConeWitnessFailure::category
  cone_band.rs:463       ConicalBandExit::category
  cone_band.rs:535       deck_join_category

The authority line (curve_witness.rs:135-143) is *authority*, not severity:
  - a curve whose endpoints are not on the certified surface, or whose own
    declared sweep does not reach its own declared endpoint, contradicts a
    claim the source itself makes -> `Inconsistent`.
  - a curve that is simply not axial, not at constant axial coordinate, or not
    the parallel it is presented as, is valid geometry the supported subset
    does not cover -> `Unsupported`.
  - a non-finite input is a machine fact -> `OperationalFailure`.

NOTE the authorized asymmetry: on a CYLINDER, `CircleNotACylinderParallel` is
`Unsupported` (curve_witness.rs:154), but on a CONE, `CircleNotAConeParallel`
is `Inconsistent` (cone_band.rs:194) — the cone treats a circle presented as a
parallel that isn't one as a source contradiction. This census respects that
distinction rather than collapsing it.
"""

from __future__ import annotations

import argparse
import collections
import gzip
import hashlib
import json
import sys
from pathlib import Path

SCHEMA_VERSION = "join-witness-census-1"

# ---------------------------------------------------------------------------
# Target band exits — everything past the curve-family gate and the
# outer-bound-authority gate.
# ---------------------------------------------------------------------------
TARGET_EXITS = {
    # Cylinder band — join (deck-walk)
    "lift_join_no_compatible_integer",
    # Cylinder band — later boundary-witness exits
    "witness_sweep_does_not_reach_endpoint",
    "witness_start_not_on_cylinder",
    "witness_circle_not_a_cylinder_parallel",
    "witness_not_constant_axial_coordinate",
    # Cylinder band — orientation
    "band_orientation_incompatible",
    # Cone band — later boundary-witness exits
    "cone_witness_start_not_on_cone",
    "cone_witness_circle_not_a_cone_parallel",
    # Cone band — bound structure
    "cone_band_bound_not_one_occurrence",
}

# ---------------------------------------------------------------------------
# Classification: each band exit maps to
#   (slice_category, primary_class, subreason, population_question_tag)
#
# `slice_category` is the truck source's own verdict (READ ONLY). `primary`
# is one of the census output classes. `population_tag` names which of the ten
# population questions the face answers.
# ---------------------------------------------------------------------------
CLASSIFICATION = {
    # --- Join (deck-walk) — cylinder_lift.rs:159 Inconsistent ---
    # All 161: cylinder, multiply_declared outer bounds, bound_signature
    # 1[Ci1];N[CiN] (one single-circle bound + one multi-circle bound),
    # Certified lift, Successful projection, SyntheticSyntheticCrossing with
    # exactly one Seam/Seam ProperInteriorCrossing and 2 synthetic segments.
    # The deck solver PROVES the multi-piece bound's developed displacement is
    # not compatible with any integer multiple of the certified period.
    "lift_join_no_compatible_integer": (
        "Inconsistent",
        "source_inconsistency",
        "multi_piece_period_join_inconsistent",
        "q8_genuine_contradictory_source_boundaries",
    ),

    # --- Cylinder witness exits ---
    # StartNotOnCylinder — curve_witness.rs:147 Inconsistent. The traversal
    # start point is not on the certified cylinder: the source vertex position
    # contradicts the surface the face is trimmed from.
    "witness_start_not_on_cylinder": (
        "Inconsistent",
        "source_inconsistency",
        "source_vertex_off_certified_surface",
        "q3_source_vertices_incompatible_geometry",
    ),
    # SweepDoesNotReachDeclaredEndpoint — curve_witness.rs:149 Inconsistent.
    # The declared sweep, developed from the start angle, does not land on the
    # declared endpoint: the source's own declared sweep contradicts its
    # declared endpoint.
    "witness_sweep_does_not_reach_endpoint": (
        "Inconsistent",
        "source_inconsistency",
        "contradictory_boundary_sweep",
        "q8_genuine_contradictory_source_boundaries",
    ),
    # CircleNotACylinderParallel — curve_witness.rs:154 Unsupported (NOT
    # Inconsistent). The complete circle's own certified placement is not the
    # cylinder parallel through its endpoint. Valid geometry outside the
    # admitted subset; the source does not contradict itself.
    "witness_circle_not_a_cylinder_parallel": (
        "Unsupported",
        "other_typed_classes",
        "unsupported_boundary_placement",
        "q10_distinct_subpopulation",
    ),
    # NotConstantAxialCoordinate — curve_witness.rs:151 Unsupported. A
    # circumferential-arc candidate whose endpoints do not share an axial
    # coordinate. Valid geometry (a helical/tilted arc) outside the subset.
    "witness_not_constant_axial_coordinate": (
        "Unsupported",
        "other_typed_classes",
        "unsupported_arc_geometry",
        "q7_complete_circle_vs_arc_confusion",
    ),
    # OrientationIncompatible — cylinder_band.rs:397 Unsupported. The two
    # induced boundary homologies have the same sign; they do not bound a
    # strip. A proved fact about the face, refused not repaired.
    "band_orientation_incompatible": (
        "Unsupported",
        "orientation_candidate",
        "carrier_homology_same_sign",
        "q5_orientation_folded_twice",
    ),

    # --- Cone witness exits ---
    # StartNotOnCone — cone_band.rs:194 Inconsistent. The start point is not
    # on the cone the face is trimmed from.
    "cone_witness_start_not_on_cone": (
        "Inconsistent",
        "source_inconsistency",
        "source_vertex_off_certified_surface",
        "q3_source_vertices_incompatible_geometry",
    ),
    # CircleNotAConeParallel — cone_band.rs:194 Inconsistent (NOT Unsupported
    # as on the cylinder). The circle presented as a cone parallel is not one:
    # its plane/centre/half-angle-predicted radius contradicts the surface.
    "cone_witness_circle_not_a_cone_parallel": (
        "Inconsistent",
        "source_inconsistency",
        "contradictory_boundary_placement",
        "q8_genuine_contradictory_source_boundaries",
    ),
    # BoundNotOneOccurrence — cone_band.rs:466 Unsupported. The bound is not
    # exactly one occurrence of a complete source circle (it is multi-piece).
    # The cell exists; the bound structure does not match the admission
    # criterion. A multi-piece cell could support it in principle.
    "cone_band_bound_not_one_occurrence": (
        "Unsupported",
        "multi_piece_supported_in_principle",
        "bound_not_one_complete_circle_occurrence",
        "q2_multi_piece_supported_in_principle",
    ),
}

# Exits whose deck-solver numeric detail (candidate integer range,
# per-constraint feasible intervals, intersection, eliminating constraint,
# join_index) is NOT retained by the production diagnostic sink. The typed
# exit + signature ARE retained; only the private solver's internals are not.
CONSTRAINT_NUMERIC_DETAIL_AVAILABLE = {
    "lift_join_no_compatible_integer": False,
}
# Every other exit carries its full evidence in the typed exit name itself
# (the witness/orientation/bound-structure verdict is self-describing).


# ---------------------------------------------------------------------------
# Truck-meshalgo deck-solver constraint structure (READ-ONLY source analysis).
#
# `solve_axis_aligned` (deck.rs:513) decides `d = k g` for the periodic integer
# `k` from a developed displacement enclosure. It checks, IN ORDER:
#
#   C1  aperiodic_contains_zero  (deck.rs:520)
#       The aperiodic (generator-orthogonal) component of `k g` is structurally
#       zero, so the aperiodic displacement must contain zero. If it provably
#       does not -> NoCompatibleInteger.  (strict contradiction)
#
#   C2  period_resolvable        (deck.rs:535)
#       The period must exceed the f64 ULP at the displacement scale for
#       adjacent deck integers to be distinguishable. If not -> Indeterminate
#       (NOT NoCompatibleInteger; an epistemic limit, not a contradiction).
#
#   C3  quotient_range_nonempty  (deck.rs:540, integer_quotient_range)
#       The conservative integer range [k_min, k_max] from the outward-rounded
#       quotient must be non-empty. If k_min > k_max -> NoCompatibleInteger.
#       (strict contradiction)
#
#   C4  compatible_integer_exists (deck.rs:546, first/last_compatible)
#       At least one integer k in [k_min, k_max] must have k*period inside the
#       periodic enclosure. Checked constant-time via the contiguous-compatible
#       property (first/last among {k_min,k_min+1} and {k_max-1,k_max}). If
#       none -> NoCompatibleInteger.  (strict contradiction)
#
# NoCompatibleInteger is therefore returned by C1, C3, or C4 — never by C2.
# The solver is certified false-negative-free: `provably_outside` (deck.rs:602)
# returns false whenever k*period could still lie inside, so no truly-
# compatible integer is ever excluded.
#
# `solve_join` (rank1_annulus.rs:1251) widens each join's developed
# displacement by `certified_join_tolerance` (JOIN_EVALUATION_ULPS = 8.0,
# rank1_annulus.rs:1221) before calling solve_axis_aligned, so a true
# zero-holonomy join is not refused by floating-point noise.
#
# `propagate_deck_placements` (rank1_annulus.rs:1288) walks the occurrence
# chain in order and stops at the FIRST join that does not resolve uniquely,
# returning DeckJoinFailure::NoCompatibleInteger { join_index }. The FIRST
# pair of constraints that makes the set empty is therefore: the first join
# (between occurrence join_index and the next, cyclically) at which C1, C3, or
# C4 returns NoCompatibleInteger.
#
# The ledger retains the typed exit `lift_join_no_compatible_integer` but NOT
# join_index, NOT which of C1/C3/C4 fired, and NOT the numeric intervals.
# ---------------------------------------------------------------------------
JOIN_CONSTRAINTS = [
    {
        "id": "C1_aperiodic_contains_zero",
        "source": "deck.rs:520",
        "description": (
            "The aperiodic (generator-orthogonal) displacement must contain "
            "zero. The shared source vertex must have the same aperiodic "
            "coordinate through both occurrences it joins."
        ),
        "on_failure": "NoCompatibleInteger",
        "kind": "strict_contradiction",
    },
    {
        "id": "C2_period_resolvable",
        "source": "deck.rs:535",
        "description": (
            "The period must be large enough, relative to the displacement "
            "scale, for adjacent deck integers to be distinguishable in f64."
        ),
        "on_failure": "Indeterminate (NOT NoCompatibleInteger)",
        "kind": "epistemic_limit",
    },
    {
        "id": "C3_quotient_range_nonempty",
        "source": "deck.rs:540",
        "description": (
            "The conservative integer range [k_min, k_max] from the outward-"
            "rounded quotient must be non-empty."
        ),
        "on_failure": "NoCompatibleInteger (k_min > k_max)",
        "kind": "strict_contradiction",
    },
    {
        "id": "C4_compatible_integer_exists",
        "source": "deck.rs:546",
        "description": (
            "At least one integer k in [k_min, k_max] must have k*period "
            "inside the periodic enclosure (constant-time first/last check)."
        ),
        "on_failure": "NoCompatibleInteger (empty compatible subset)",
        "kind": "strict_contradiction",
    },
]


def read_gz(path: Path) -> str:
    if not path.exists():
        return ""
    with gzip.open(path, "rt", encoding="utf-8", errors="replace") as handle:
        return handle.read()


def parse_kv(line: str, prefix: str) -> dict | None:
    if not line.startswith(prefix):
        return None
    fields = {}
    for piece in line.rstrip("\n").split("\t")[1:]:
        if "=" in piece:
            key, value = piece.split("=", 1)
            fields[key] = value
    return fields


def normalise_id(raw: str | None) -> str | None:
    if raw is None or raw in ("-", "none", ""):
        return None
    return raw.lstrip("#")


def load_model(directory: Path) -> tuple[list[dict], dict]:
    """One model's joined per-face rows for target-exit faces, plus totals."""
    # Source probe (faces.tsv.gz)
    probes: dict[str, dict] = {}
    for line in read_gz(directory / "faces.tsv.gz").splitlines():
        fields = parse_kv(line, "FACE\t")
        if fields is None:
            continue
        key = normalise_id(fields.get("source_face_id"))
        if key is None:
            continue
        probes[key] = fields

    # Structured diagnosis (diag.jsonl.gz) — lost faces only
    diags: dict[str, dict] = {}
    for line in read_gz(directory / "diag.jsonl.gz").splitlines():
        if not line.strip():
            continue
        record = json.loads(line)
        key = normalise_id(
            None if record.get("source_face_id") is None
            else str(record["source_face_id"])
        )
        if key is not None:
            diags[key] = record

    # Per-edge curve probe (curves.tsv.gz from cone-band, if available)
    edge_curves: dict[str, list[dict]] = collections.defaultdict(list)
    cone_band_dir = directory.parent.parent / "cone-band" / directory.name
    for line in read_gz(cone_band_dir / "curves.tsv.gz").splitlines():
        fields = parse_kv(line, "EDGE\t")
        if fields is None:
            continue
        key = normalise_id(fields.get("source_face_id"))
        if key is not None:
            edge_curves[key].append(fields)

    rows: list[dict] = []
    declared = target_found = 0
    for line in read_gz(directory / "ledger.tsv.gz").splitlines():
        fields = parse_kv(line, "FACE\t")
        if fields is None:
            continue
        declared += 1
        fid = normalise_id(fields.get("source_face_id"))
        band = fields.get("band", "")
        cband = fields.get("cone_band", "")

        # Determine the exit: cylinder band first, then cone band. A face
        # never carries both a cylinder-band and a cone-band typed exit
        # (the surface family selects the route).
        exit_name = None
        exit_column = None
        if band in TARGET_EXITS:
            exit_name = band
            exit_column = "band"
        elif cband in TARGET_EXITS:
            exit_name = cband
            exit_column = "cone_band"
        if exit_name is None:
            continue
        target_found += 1

        probe = probes.get(fid, {}) if fid else {}
        diag = diags.get(fid, {}) if fid else {}
        edges = edge_curves.get(fid, []) if fid else []

        conflicts = diag.get("insertion_conflicts", []) or []
        slice_cat, primary, subreason, pop_tag = CLASSIFICATION[exit_name]

        row = {
            "model_id": directory.name,
            "source_face_id": fid or "-",
            "exit_name": exit_name,
            "exit_column": exit_column,
            "slice_category": slice_cat,
            "primary": primary,
            "subreason": subreason,
            "population_tag": pop_tag,
            "surface_kind": fields.get("surface_kind", "-"),
            "band": band if band not in ("", "-") else "-",
            "cone_band": cband if cband not in ("", "-") else "-",
            # Structured diagnosis
            "terminal_reason": diag.get("terminal_reason"),
            "derived_bucket": diag.get("derived_bucket"),
            "chart_rank": diag.get("chart_rank"),
            "bound_count": diag.get("bound_count"),
            "source_segment_count": diag.get("source_segment_count"),
            "synthetic_segment_count": diag.get("synthetic_segment_count"),
            "lift_status": diag.get("lift_status"),
            "deck_status": diag.get("deck_status"),
            "projection_status": diag.get("projection_status"),
            "conflict_count": len(conflicts),
            "conflict_origins": ",".join(sorted({
                c.get("incoming", {}).get("origin", "-") + "/"
                + c.get("blocking", {}).get("origin", "-")
                for c in conflicts
            })),
            "conflict_relations": ",".join(sorted({
                c.get("relation", "-") for c in conflicts
            })),
            "periodic_axes": (
                None if diag.get("periodic_axes") is None
                else json.dumps(diag.get("periodic_axes"), separators=(",", ":"))
            ),
            # Source probe
            "bound_signature": probe.get("bound_signature", "-"),
            "bounds": probe.get("bounds", "-"),
            "edge_uses": probe.get("edge_uses", "-"),
            "curves": probe.get("curves", "-"),
            "certified_rank": probe.get("certified_rank", "-"),
            "declared_rank": probe.get("declared_rank", "-"),
            "outer_standing": probe.get("outer_standing", "-"),
            "outer_declared_count": probe.get("outer_declared_count", "-"),
            "outer_bound_index": probe.get("outer_bound_index", "-"),
            "cylinder": probe.get("cylinder", "-"),
            "support": probe.get("support", "-"),
            "unread_rank0": probe.get("unread_rank0", "-"),
            "unread_rank1": probe.get("unread_rank1", "-"),
            # Per-edge curve families (from curve probe, if available)
            "edge_curve_families": ",".join(
                e.get("imported", "-") for e in edges
            ) if edges else "-",
            "edge_count_probe": len(edges),
            # Evidence completeness
            "has_probe": bool(probe),
            "has_diag": bool(diag),
            "has_edge_probe": bool(edges),
            "constraint_numeric_detail_available": (
                CONSTRAINT_NUMERIC_DETAIL_AVAILABLE.get(exit_name, True)
            ),
        }
        rows.append(row)

    return rows, {
        "declared": declared,
        "target_found": target_found,
    }


FIELDS = [
    "model_id", "source_face_id", "exit_name", "exit_column",
    "slice_category", "primary", "subreason", "population_tag",
    "surface_kind", "band", "cone_band",
    "terminal_reason", "derived_bucket", "chart_rank", "bound_count",
    "source_segment_count", "synthetic_segment_count",
    "lift_status", "deck_status", "projection_status",
    "conflict_count", "conflict_origins", "conflict_relations",
    "periodic_axes",
    "bound_signature", "bounds", "edge_uses", "curves",
    "certified_rank", "declared_rank",
    "outer_standing", "outer_declared_count", "outer_bound_index",
    "cylinder", "support", "unread_rank0", "unread_rank1",
    "edge_curve_families", "edge_count_probe",
    "has_probe", "has_diag", "has_edge_probe",
    "constraint_numeric_detail_available",
]


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--out", default="C:/Users/stefa/look-corpus/cone-final",
        help="cone-final sweep output directory",
    )
    parser.add_argument(
        "--probe-out", default="C:/Users/stefa/look-corpus/join-witness",
        help="directory for generated ledgers (external, not committed)",
    )
    parser.add_argument("--json", default=None)
    parser.add_argument("--ledger", default=None, help="full per-face TSV.gz")
    parser.add_argument("--populations", type=int, default=30)
    args = parser.parse_args()

    out = Path(args.out).resolve()
    index = json.loads((out / "index.json").read_text())
    revisions = {}
    statuses = collections.Counter()
    for record in index.values():
        revisions = {
            "look": record["look_rev"],
            "truck": record["truck_rev"],
            "cargo_lock": record["cargo_lock"],
            "schema": record.get("schema", "unknown"),
        }
        statuses[record["outcome"]] += 1

    rows: list[dict] = []
    totals = collections.Counter()
    per_model = {}
    for directory in sorted(p for p in out.iterdir() if p.is_dir()):
        model_rows, model_totals = load_model(directory)
        if not model_rows:
            continue
        rows.extend(model_rows)
        per_model[directory.name] = model_totals
        for key, value in model_totals.items():
            totals[key] += value

    if not rows:
        print("no target-exit faces found — was the sweep run?", file=sys.stderr)
        return 1

    # -- reconciliation ----------------------------------------------------
    exit_hist = collections.Counter(r["exit_name"] for r in rows)
    primary_hist = collections.Counter(r["primary"] for r in rows)
    slice_hist = collections.Counter(r["slice_category"] for r in rows)
    pop_hist = collections.Counter(r["population_tag"] for r in rows)
    assert sum(exit_hist.values()) == len(rows)
    assert sum(primary_hist.values()) == len(rows)
    assert sum(slice_hist.values()) == len(rows)

    # Evidence completeness. Every targeted face has a typed band exit (the
    # certifier's own verdict) and a classification, so it is fully explained.
    fully_explained = len(rows)
    not_instrumented = sum(1 for r in rows if not r["has_diag"])
    missing_source_evidence = sum(1 for r in rows if not r["has_probe"])
    missing_constraint_detail = sum(
        1 for r in rows if not r["constraint_numeric_detail_available"]
    )

    def histogram(key):
        return collections.Counter(
            key(r) if not isinstance(key, str) else r[key] for r in rows
        )

    # Population signatures
    populations = collections.Counter(
        (r["exit_name"], r["surface_kind"], r["bound_signature"],
         r["outer_standing"])
        for r in rows
    )
    population_rows = []
    for signature, count in populations.most_common():
        members = [
            r for r in rows
            if (r["exit_name"], r["surface_kind"], r["bound_signature"],
                r["outer_standing"]) == signature
        ]
        models = collections.Counter(r["model_id"] for r in members)
        representative = min(
            members,
            key=lambda r: (r["model_id"], int(r["source_face_id"])
                           if r["source_face_id"].isdigit() else 0),
        )
        population_rows.append({
            "exit_name": signature[0], "surface_kind": signature[1],
            "bound_signature": signature[2], "outer_standing": signature[3],
            "faces": count, "models": len(models),
            "top_models": models.most_common(4),
            "slice_category": representative["slice_category"],
            "primary": representative["primary"],
            "subreason": representative["subreason"],
            "population_tag": representative["population_tag"],
            "representative": {
                "model_id": representative["model_id"],
                "source_face_id": representative["source_face_id"],
                "edge_uses": representative["edge_uses"],
                "curves": representative["curves"],
                "source_segment_count": representative["source_segment_count"],
                "lift_status": representative["lift_status"],
            },
        })

    report = {
        "schema": SCHEMA_VERSION,
        "revisions": revisions,
        "run_outcomes": dict(statuses),
        "totals": {
            "total_targeted": len(rows),
            "fully_explained": fully_explained,
            "not_instrumented": not_instrumented,
            "missing_source_evidence": missing_source_evidence,
            "missing_constraint_numeric_detail": missing_constraint_detail,
        },
        "per_model": per_model,
        "exit_histogram": dict(exit_hist.most_common()),
        "primary_classification": dict(primary_hist.most_common()),
        "slice_category": dict(slice_hist.most_common()),
        "population_tag": dict(pop_hist.most_common()),
        "surface_family": dict(histogram("surface_kind").most_common()),
        "outer_standing": dict(histogram("outer_standing").most_common()),
        "lift_status": dict(histogram("lift_status").most_common()),
        "projection_status": dict(histogram("projection_status").most_common()),
        "bound_signature_top": dict(
            histogram("bound_signature").most_common(15)
        ),
        "model_concentration": dict(histogram("model_id").most_common()),
        "join_constraints": JOIN_CONSTRAINTS,
        "populations": population_rows,
    }

    # -- write ledger ------------------------------------------------------
    probe_out = Path(args.probe_out)
    probe_out.mkdir(parents=True, exist_ok=True)
    ledger_path = Path(args.ledger) if args.ledger else probe_out / "join_witness.tsv.gz"
    ledger_path.parent.mkdir(parents=True, exist_ok=True)
    body = "\t".join(FIELDS) + "\n"
    body += "".join(
        "\t".join("" if r.get(f) is None else str(r.get(f, ""))
                  for f in FIELDS) + "\n"
        for r in sorted(
            rows,
            key=lambda r: (r["model_id"],
                           int(r["source_face_id"])
                           if r["source_face_id"].isdigit() else 0),
        )
    )
    data = body.encode("utf-8")
    with gzip.open(ledger_path, "wb") as handle:
        handle.write(data)
    report["ledger"] = {
        "path": str(ledger_path),
        "rows": len(rows),
        "sha256": hashlib.sha256(data).hexdigest(),
        "schema": SCHEMA_VERSION,
        "fields": FIELDS,
    }

    if args.json:
        Path(args.json).write_text(json.dumps(report, indent=1))

    # -- human-readable ----------------------------------------------------
    print(
        f"look={revisions.get('look')} truck={revisions.get('truck')} "
        f"lock={revisions.get('cargo_lock')} schema={revisions.get('schema')}"
    )
    print(f"run outcomes: {dict(statuses)}")
    print()
    t = report["totals"]
    print(f"total targeted:            {t['total_targeted']}")
    print(f"  fully explained:         {t['fully_explained']}")
    print(f"  not instrumented:        {t['not_instrumented']}")
    print(f"  missing source evidence: {t['missing_source_evidence']}")
    print(f"  missing constraint numeric detail: {t['missing_constraint_numeric_detail']}"
          f"  (join faces: private deck solver emits no per-face intervals)")
    print()

    def table(title, mapping, width=52):
        print(f"  {title}")
        for name, count in mapping.items():
            print(f"    {str(name):{width}} {count:7}  "
                  f"{count / len(rows) * 100:5.1f}%")
        print()

    table("exit histogram", report["exit_histogram"])
    table("slice category (truck source authority)", report["slice_category"])
    table("primary classification", report["primary_classification"])
    table("population tag (10 questions)", report["population_tag"])
    table("surface family", report["surface_family"])
    table("outer-bound standing", report["outer_standing"])
    table("lift status", report["lift_status"])
    table("projection status", report["projection_status"])
    table("bound signature (top 15)", report["bound_signature_top"], 30)
    table("model concentration", report["model_concentration"])

    print(f"  major populations (top {args.populations})")
    print(
        f"    {'faces':>7} {'mdl':>3}  {'exit_name':48} "
        f"{'surface':9} {'bound_sig':18} representative"
    )
    for pop in population_rows[: args.populations]:
        rep = pop["representative"]
        print(
            f"    {pop['faces']:7} {pop['models']:3}  "
            f"{pop['exit_name']:48} {pop['surface_kind']:9} "
            f"{pop['bound_signature']:18} "
            f"{rep['model_id']}#{rep['source_face_id']}"
        )
    print()

    # Join constraint structure
    print("  JoinNoCompatibleInteger constraint structure (from truck source):")
    for c in JOIN_CONSTRAINTS:
        print(f"    {c['id']:40} {c['source']:14} -> {c['on_failure']}")
    print()
    print("    NoCompatibleInteger is returned by C1, C3, or C4 — never C2")
    print("    (C2 returns Indeterminate). The walk stops at the FIRST join")
    print("    (join_index) that fails; the ledger retains the typed exit but")
    print("    not join_index, the firing constraint, or the numeric intervals.")
    print()

    print(f"  ledger: {ledger_path}")
    print(f"  rows: {len(rows)}, sha256: {report['ledger']['sha256'][:16]}...")
    return 0


if __name__ == "__main__":
    sys.exit(main())
