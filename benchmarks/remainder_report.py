"""Assign one primary diagnosis to every post-circle lost face (REMAINDER-DIAG-001).

Reads the three readings `remainder_sweep.py` takes and joins them on
`source_face_id` — the ledger's rendered/lost verdict and band attempt, the
structured `FailedFaceDiagnosis` the same run emitted, and `remainder_probe`'s
record of the source authority the face still carries.

# What the classification is and is not

Every field it reads is a source datum or a production classifier's own tag.
Nothing here infers intent from appearance: no bound becomes "the outer one" for
being largest, no curve becomes a circle for being nearly circular, and no face
becomes malformed for merely failing. Where the retained evidence cannot decide
between two accounts, the face lands in a class that says so rather than in
whichever bucket is more convenient.

# Earliest evidence-bearing obstruction

The diagnosis is the *earliest* stage at which the system can no longer justify
proceeding, not the last error it printed. The two differ, and the difference is
the point of the packet. One case governs the largest population and is worth
stating outright:

    ConstraintInsertionIncomplete / SyntheticSyntheticCrossing
    on a surface of certified deck rank >= 1

means the *synthetic* edges — the closure and seam segments the cut-open plan
invented — cross each other. No source geometry is in conflict. That is what a
wrong cut plan looks like, and the cylinder-band packet demonstrated it
directly: the band route is gated on exactly this bucket, and naming the cell
(two oppositely oriented essential parallels on ordered carriers, modulo the
angular deck) removed the crossing for 15,123 faces without touching the
arrangement code. So the primary diagnosis for that signature is
`AtlasClassification`, and the crossing is retained as the observed later exit.

Where a *named* cell exists and its own certifier refused — the typed band exits
— that finer verdict wins over the generic bucket, because it is later evidence
about the same face and says more.

The later exit is never discarded; `terminal_reason`, `derived_bucket` and the
band exit all travel on every row of the full ledger.
"""

from __future__ import annotations

import argparse
import collections
import gzip
import hashlib
import json
import sys
from pathlib import Path

SCHEMA_VERSION = "remainder-diag-1"

# ---------------------------------------------------------------------------
# Which atlas cells production actually implements.
#
# Not a claim about which cells *exist* mathematically — a claim about which
# ones this system can name and realize today. `plane` has the rank-0 disk and
# the disk-with-holes route; `cylinder` has the essential band. Nothing else has
# a formal cell, so a face on any other family that needs one is blocked on
# mathematics that has not been written, which is a different kind of work from
# a cell that exists and refused.
# ---------------------------------------------------------------------------
FAMILIES_WITH_A_CELL = {"plane", "cylinder"}

# The band's own typed exits, by what the exit establishes about the cell.
BAND_EXIT_DIAGNOSIS = {
    # The curve gate: a bound curve no structural reader reads. Nothing about
    # the cell is decided, because the boundary was never witnessed.
    "unsupported_curve_representation": (
        "CurveBoundaryWitness", "band_bound_curve_unreadable",
        "CellCandidateInsufficientEvidence",
    ),
    # STEP's outer-bound standing was absent or self-contradictory, and no band
    # certificate existed to make the malformed declaration uniquely recoverable.
    "missing_outer_bound_authority": (
        "MaterialAuthority", "band_outer_bound_authority_absent",
        "CellCandidateInsufficientEvidence",
    ),
    # The two bounds are not two essential parallels. The candidate cell is
    # refused on positive geometric evidence — this face is not a band.
    "witness_start_not_on_cylinder": (
        "AtlasClassification", "band_witness_refuted", "ContradictoryCellEvidence",
    ),
    "witness_circle_not_a_cylinder_parallel": (
        "AtlasClassification", "band_witness_refuted", "ContradictoryCellEvidence",
    ),
    "witness_sweep_does_not_reach_endpoint": (
        "AtlasClassification", "band_witness_refuted", "ContradictoryCellEvidence",
    ),
    "witness_not_constant_axial_coordinate": (
        "AtlasClassification", "band_witness_refuted", "ContradictoryCellEvidence",
    ),
    # The cell was certified this far and the *realization* stopped: the lift
    # could not be joined at a compatible deck translate, or the two carriers'
    # orientations do not bound a strip.
    "lift_join_no_compatible_integer": (
        "CutOpenOrArrangement", "band_lift_join_no_compatible_integer",
        "CandidateAtlasCell",
    ),
    "band_orientation_incompatible": (
        "CutOpenOrArrangement", "band_orientation_incompatible", "CandidateAtlasCell",
    ),
}

# The remaining epistemic gap for each (stage, subreason). One line each, in the
# vocabulary the packet asked for.
GAP = {
    ("SourceImport", "AllBoundsCollapsed"):
        "Source genuinely degenerate, or importer must retain the collapsed-bound case",
    ("SourceImport", "EdgeCurveConversionFailed"):
        "Importer must retain the source curve; needs the raw entity to say which",
    ("SourceImport", "SurfaceConversionFailed"):
        "Importer must retain the source surface; needs the raw entity to say which",
    ("CurveBoundaryWitness", "band_bound_curve_unreadable"):
        "Need exact curve-on-surface witness for the unread curve family",
    ("CurveBoundaryWitness", "no_certified_preimage_on_support"):
        "Need exact curve-on-surface witness (boundary preimage on a free-form support)",
    ("CurveBoundaryWitness", "boundary_not_constructed"):
        "Need only better diagnostics: no typed evidence of why the wire is absent",
    ("AtlasClassification", "quotient_cell_not_named"):
        "Need new atlas cell (boundary homology + material-region topology) and its quotient cut plan",
    ("AtlasClassification", "deck_generator_uncertified"):
        "Need a representation-derived period witness: the axis is declared but uncertified, so no deck generator exists",
    ("MeshRealization", "constraint_role_missing"):
        "Need only better diagnostics: a realized constraint carried no role",
    ("AtlasClassification", "periodic_lift_branch_unresolved"):
        "Need boundary homology calculation before the lift branch can be chosen",
    ("AtlasClassification", "band_witness_refuted"):
        "Need a different atlas cell: the band candidate is refuted on positive evidence",
    ("MaterialAuthority", "band_outer_bound_authority_absent"):
        "Need material-region uniqueness proof without an outer-bound declaration",
    ("MaterialAuthority", "parity_contradiction"):
        "Need certified arrangement predicate, or the source is genuinely inconsistent",
    ("MaterialAuthority", "no_material_region"):
        "Need material-region uniqueness proof; parity flood selected nothing",
    ("CutOpenOrArrangement", "source_source_crossing"):
        "Need certified arrangement predicate, or a proof the source self-intersects",
    ("CutOpenOrArrangement", "source_synthetic_crossing"):
        "Need quotient cut plan that cannot cross authoritative source trim",
    ("CutOpenOrArrangement", "synthetic_closure_self_crossing"):
        "Need quotient cut plan; on a rank-0 chart this is an arrangement defect",
    ("CutOpenOrArrangement", "mixed_conflict"):
        "Need certified arrangement predicate; conflict classes are heterogeneous",
    ("CutOpenOrArrangement", "overlap_unsupported"):
        "Need certified arrangement predicate admitting collinear overlap",
    ("CutOpenOrArrangement", "band_lift_join_no_compatible_integer"):
        "Recognized cell, realization gap: no compatible deck translate joins the lift",
    ("CutOpenOrArrangement", "band_orientation_incompatible"):
        "Recognized cell, realization gap: carrier orientations do not bound a strip",
    ("MeshRealization", "vertex_insertion_failed"):
        "Recognized cell, realization gap: a vertex could not enter the triangulation",
    ("MeshRealization", "insertion_unwitnessed"):
        "Need only better diagnostics: the insertion failed with no retained witness",
}


# The coarse kind of work each primary stage implies. Deliberately blunt: it
# exists so "how much of this is provenance and how much is new mathematics" has
# an answer, and it adds nothing the per-face rows do not already state.
WORK_KIND = {
    "SourceImport": "source_or_import",
    "CurveBoundaryWitness": "new_mathematics",
    "AtlasClassification": "new_mathematics",
    "MaterialAuthority": "new_mathematics",
    "CutOpenOrArrangement": "realization_or_arrangement",
    "MeshRealization": "realization_or_arrangement",
    "Validation": "realization_or_arrangement",
    "Operational": "operational",
    "NotYetInstrumented": "diagnostics_only",
}


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
    """The join key. The ledger prints `#48344`; the probe prints `48344`."""
    if raw is None or raw in ("-", "none", ""):
        return None
    return raw.lstrip("#")


def curve_class(curves: str) -> str:
    """A coarse class for the face's imported boundary-curve families.

    Reported as a *class*, not as an intent: `spline_present` says a B-spline or
    NURBS curve bounds this face, not that the face is a spline problem.
    """
    if not curves:
        return "none"
    present = {piece.rstrip("0123456789") for piece in curves.split(",") if piece}
    if present & {"Bs", "Nu"}:
        return "spline_present"
    if present & {"El", "Hy", "Pa"}:
        return "noncircular_conic_present"
    if present <= {"Ln", "Pl"}:
        return "linear_only"
    if present <= {"Ci", "Ln", "Pl"}:
        return "circle_and_linear"
    return "other"


def diagnose(row: dict) -> tuple[str, str, str]:
    """(primary stage, exact subreason, atlas status) for one lost face.

    Ordered earliest-first. The first arm that the evidence supports wins; a
    later exit never overrides an earlier obstruction that is already
    established.
    """
    family = row["surface_kind"]
    has_cell = family in FAMILIES_WITH_A_CELL

    # 1. The face never became a topological face. Nothing downstream ran.
    if row["stage"] == "convert":
        return "SourceImport", row["reason"], "NotReached"

    # 2. A named cell's own certifier refused. Later evidence about the same
    #    face than the generic legacy bucket, and more specific, so it wins.
    band = row.get("band", "not_eligible")
    if band in BAND_EXIT_DIAGNOSIS:
        return BAND_EXIT_DIAGNOSIS[band]

    reason = row.get("terminal_reason")
    bucket = row.get("derived_bucket")
    certified_rank = row.get("certified_rank")

    # 3. No diagnosis row reached this face at all.
    if reason is None:
        return "NotYetInstrumented", "no_typed_failure_retained", "NotReached"

    # 4. The boundary was never witnessed. Nothing about the cell is decided.
    if reason in ("BoundaryWireEmpty", "BoundaryConstructionFailed"):
        return "CurveBoundaryWitness", "boundary_not_constructed", "NotReached"
    if reason in ("BoundaryProjectionFailed", "BoundaryPointOffSurface"):
        return (
            "CurveBoundaryWitness",
            "no_certified_preimage_on_support",
            "NotReached",
        )

    # 5. The chart is periodic and the lift branch could not be chosen. The
    #    homology of the boundary is exactly what would choose it.
    if reason == "AmbiguousLift":
        return (
            "AtlasClassification",
            "periodic_lift_branch_unresolved",
            "NotEnoughEvidenceToClassify",
        )

    if reason == "ContradictoryDualParity":
        return "MaterialAuthority", "parity_contradiction", (
            "CandidateAtlasCell" if has_cell else "NoImplementedAtlasCell"
        )
    if reason == "NoOddParityRegion":
        return "MaterialAuthority", "no_material_region", (
            "CandidateAtlasCell" if has_cell else "NoImplementedAtlasCell"
        )
    if reason == "ConstraintOverlapUnsupported":
        return "CutOpenOrArrangement", "overlap_unsupported", (
            "CandidateAtlasCell" if has_cell else "NoImplementedAtlasCell"
        )

    if reason == "ConstraintInsertionIncomplete":
        if bucket == "SyntheticSyntheticCrossing":
            # Only synthetic edges are in conflict. On a periodic chart that is
            # the signature the band packet removed by naming the cell.
            if certified_rank and certified_rank >= 1:
                return (
                    "AtlasClassification",
                    "quotient_cell_not_named",
                    "CandidateAtlasCell" if has_cell else "NoImplementedAtlasCell",
                )
            # A period the surface *declares* but nothing certifies is not a
            # deck generator, so there is no quotient to cut open even in
            # principle. That is a different obstruction from a rank-0 chart
            # whose synthetic edges simply cross, and keeping them apart is the
            # difference between needing a period witness and needing an
            # arrangement.
            if row.get("declared_rank"):
                return (
                    "AtlasClassification",
                    "deck_generator_uncertified",
                    "NotEnoughEvidenceToClassify",
                )
            return (
                "CutOpenOrArrangement",
                "synthetic_closure_self_crossing",
                "CandidateAtlasCell" if has_cell else "NoImplementedAtlasCell",
            )
        if bucket in (
            "SourceSourceSameBoundCrossing",
            "SourceSourceInterBoundCrossing",
        ):
            return "CutOpenOrArrangement", "source_source_crossing", (
                "CandidateAtlasCell" if has_cell else "NoImplementedAtlasCell"
            )
        if bucket == "SourceSyntheticCrossing":
            return "CutOpenOrArrangement", "source_synthetic_crossing", (
                "CandidateAtlasCell" if has_cell else "NoImplementedAtlasCell"
            )
        if bucket == "MixedConstraintConflict":
            return "CutOpenOrArrangement", "mixed_conflict", (
                "CandidateAtlasCell" if has_cell else "NoImplementedAtlasCell"
            )
        if bucket == "VertexInsertionFailure":
            return "MeshRealization", "vertex_insertion_failed", (
                "CandidateAtlasCell" if has_cell else "NoImplementedAtlasCell"
            )
        return "MeshRealization", "insertion_unwitnessed", "NotEnoughEvidenceToClassify"

    if reason == "ConstraintRoleMissing":
        return "MeshRealization", "constraint_role_missing", "NotEnoughEvidenceToClassify"

    return "NotYetInstrumented", f"unmapped:{reason}", "NotReached"


def load_model(directory: Path) -> tuple[list[dict], dict]:
    """One model's joined per-face rows, plus its own totals."""
    probes: dict[str, dict] = {}
    probe_duplicates = 0
    for line in read_gz(directory / "faces.tsv.gz").splitlines():
        fields = parse_kv(line, "FACE\t")
        if fields is None:
            continue
        key = normalise_id(fields.get("source_face_id"))
        if key is None:
            continue
        if key in probes:
            probe_duplicates += 1
            continue
        probes[key] = fields

    diags: dict[str, dict] = {}
    diag_unkeyed: list[dict] = []
    for line in read_gz(directory / "diag.jsonl.gz").splitlines():
        if not line.strip():
            continue
        record = json.loads(line)
        key = normalise_id(
            None if record.get("source_face_id") is None
            else str(record["source_face_id"])
        )
        if key is None or key in diags:
            diag_unkeyed.append(record)
            continue
        diags[key] = record

    rows: list[dict] = []
    declared = rendered = 0
    for line in read_gz(directory / "ledger.tsv.gz").splitlines():
        fields = parse_kv(line, "FACE\t")
        if fields is None:
            continue
        declared += 1
        if fields.get("rendered") == "1":
            rendered += 1
            continue
        key = normalise_id(fields.get("source_face_id"))
        probe = probes.get(key, {}) if key else {}
        diag = diags.get(key, {}) if key else {}
        row = {
            "model_id": directory.name,
            "source_face_id": key or "-",
            "stage": fields.get("stage", "-"),
            "reason": fields.get("reason", "-"),
            "band": fields.get("band", "not_eligible"),
            # The ledger leaves `surface_kind` unset for a convert-stage loss —
            # the face never reached a surface. The probe cannot supply it
            # either, for the same reason, so it stays explicitly unknown.
            "surface_kind": fields.get("surface_kind", "-"),
            "terminal_reason": diag.get("terminal_reason"),
            "derived_bucket": diag.get("derived_bucket"),
            "chart_rank": diag.get("chart_rank"),
            "bound_count": diag.get("bound_count"),
            "lift_status": diag.get("lift_status"),
            "deck_status": diag.get("deck_status"),
            "projection_status": diag.get("projection_status"),
            "conflict_count": len(diag.get("insertion_conflicts", []) or []),
            "source_segments": diag.get("source_segment_count"),
            "synthetic_segments": diag.get("synthetic_segment_count"),
            "outer_standing": probe.get("outer_standing", "-"),
            "outer_declared_count": probe.get("outer_declared_count", "-"),
            "bounds": probe.get("bounds", "-"),
            "edge_uses": probe.get("edge_uses", "-"),
            "declared_rank": int(probe["declared_rank"]) if "declared_rank" in probe else None,
            "certified_rank": int(probe["certified_rank"]) if "certified_rank" in probe else None,
            "support": probe.get("support", "-"),
            "cylinder": probe.get("cylinder", "-"),
            "curves": probe.get("curves", ""),
            "bound_signature": probe.get("bound_signature", "-"),
            "unread_rank1": probe.get("unread_rank1", "-"),
            "has_probe": bool(probe),
            "has_diag": bool(diag),
        }
        row["curve_class"] = curve_class(row["curves"])
        primary, subreason, atlas = diagnose(row)
        row["primary"] = primary
        row["subreason"] = subreason
        row["atlas_status"] = atlas
        rows.append(row)

    return rows, {
        "declared": declared,
        "rendered": rendered,
        "lost": declared - rendered,
        "probe_duplicate_ids": probe_duplicates,
        "diag_unkeyed_rows": len(diag_unkeyed),
    }


FIELDS = [
    "model_id", "source_face_id", "primary", "subreason", "atlas_status",
    "surface_kind", "curve_class", "stage", "reason", "band",
    "terminal_reason", "derived_bucket", "chart_rank", "certified_rank",
    "declared_rank", "bound_count", "bounds", "edge_uses", "outer_standing",
    "outer_declared_count", "lift_status", "deck_status", "projection_status",
    "conflict_count", "source_segments", "synthetic_segments", "support",
    "cylinder", "curves", "bound_signature", "unread_rank1",
    "has_probe", "has_diag",
]


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--out", default="C:/Users/stefa/look-corpus/remainder-out")
    parser.add_argument("--json", default=None)
    parser.add_argument("--ledger", default=None, help="full per-face TSV.gz")
    parser.add_argument("--populations", type=int, default=25)
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
            "schema": record.get("schema", SCHEMA_VERSION),
        }
        statuses[record["outcome"]] += 1

    rows: list[dict] = []
    totals = collections.Counter()
    per_model = {}
    for directory in sorted(p for p in out.iterdir() if p.is_dir()):
        model_rows, model_totals = load_model(directory)
        if model_totals["declared"] == 0:
            continue
        rows.extend(model_rows)
        per_model[directory.name] = model_totals
        for key, value in model_totals.items():
            totals[key] += value

    if not rows:
        print("no rows — was the sweep run?", file=sys.stderr)
        return 1

    # -- reconciliation ----------------------------------------------------
    assert len(rows) == totals["lost"], (len(rows), totals["lost"])
    primary_hist = collections.Counter(r["primary"] for r in rows)
    assert sum(primary_hist.values()) == len(rows)

    def histogram(key):
        return collections.Counter(
            key(r) if not isinstance(key, str) else r[key] for r in rows
        )

    populations = collections.Counter(
        (r["primary"], r["subreason"], r["surface_kind"], r["atlas_status"],
         r["curve_class"])
        for r in rows
    )
    population_rows = []
    for signature, count in populations.most_common():
        members = [
            r for r in rows
            if (r["primary"], r["subreason"], r["surface_kind"], r["atlas_status"],
                r["curve_class"]) == signature
        ]
        models = collections.Counter(r["model_id"] for r in members)
        representative = min(
            members,
            key=lambda r: (r["model_id"], int(r["source_face_id"])
                           if r["source_face_id"].isdigit() else 0),
        )
        population_rows.append({
            "primary": signature[0], "subreason": signature[1],
            "surface_kind": signature[2], "atlas_status": signature[3],
            "curve_class": signature[4], "faces": count,
            "models": len(models),
            "top_models": models.most_common(4),
            "representative": {
                "model_id": representative["model_id"],
                "source_face_id": representative["source_face_id"],
                "bounds": representative["bounds"],
                "bound_signature": representative["bound_signature"],
                "outer_standing": representative["outer_standing"],
                "certified_rank": representative["certified_rank"],
                "terminal_reason": representative["terminal_reason"],
                "derived_bucket": representative["derived_bucket"],
                "band": representative["band"],
            },
            "gap": GAP.get((signature[0], signature[1]), "unclassified"),
        })

    report = {
        "revisions": revisions,
        "run_outcomes": dict(statuses),
        "totals": {
            "declared": totals["declared"],
            "rendered": totals["rendered"],
            "lost": totals["lost"],
            "classified": len(rows),
            "rows_with_source_probe": sum(1 for r in rows if r["has_probe"]),
            "rows_with_diagnosis": sum(1 for r in rows if r["has_diag"]),
            "probe_duplicate_ids": totals["probe_duplicate_ids"],
            "diag_unkeyed_rows": totals["diag_unkeyed_rows"],
        },
        "per_model": per_model,
        "primary": dict(primary_hist.most_common()),
        "primary_subreason": {
            f"{a}/{b}": c for (a, b), c in
            collections.Counter((r["primary"], r["subreason"]) for r in rows).most_common()
        },
        "surface_family": dict(histogram("surface_kind").most_common()),
        "atlas_status": dict(histogram("atlas_status").most_common()),
        "curve_class": dict(histogram("curve_class").most_common()),
        "terminal_reason": dict(
            collections.Counter(r["terminal_reason"] or "-" for r in rows).most_common()
        ),
        "derived_bucket": dict(
            collections.Counter(r["derived_bucket"] or "-" for r in rows).most_common()
        ),
        "outer_standing": dict(histogram("outer_standing").most_common()),
        "band_exit": dict(
            collections.Counter(
                r["band"] for r in rows if r["band"] not in ("not_eligible", "-")
            ).most_common()
        ),
        "model_concentration": dict(histogram("model_id").most_common()),
        # Declared against certified periodicity, per family. A cell on a
        # periodic surface needs a *generator*, and an axis that is only
        # declared does not supply one — so this table says, per family, how
        # much of the loss cannot form a quotient at all yet.
        "periodicity": dict(
            collections.Counter(
                f"{r['surface_kind']} declared={r['declared_rank']} "
                f"certified={r['certified_rank']}"
                for r in rows
            ).most_common()
        ),
        # What kind of work each face is waiting on. Coarse by construction and
        # derived only from the primary diagnosis, so it adds no claim the
        # per-face rows do not already carry.
        "work_kind": dict(
            collections.Counter(WORK_KIND[r["primary"]] for r in rows).most_common()
        ),
        "populations": population_rows,
    }

    if args.ledger:
        ledger_path = Path(args.ledger)
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
    total = report["totals"]
    print(
        f"look={revisions.get('look')} truck={revisions.get('truck')} "
        f"lock={revisions.get('cargo_lock')} schema={revisions.get('schema')}"
    )
    print(f"run outcomes: {dict(statuses)}")
    print()
    print(
        f"{total['declared']} declared, {total['rendered']} rendered, "
        f"{total['lost']} lost ({total['lost'] / total['declared'] * 100:.2f}%)"
    )
    print(
        f"  classified {total['classified']}/{total['lost']}, "
        f"source probe joined on {total['rows_with_source_probe']}, "
        f"typed diagnosis on {total['rows_with_diagnosis']}"
    )
    print()

    def table(title, mapping, width=52):
        print(f"  {title}")
        for name, count in mapping.items():
            print(f"    {str(name):{width}} {count:7}  {count / len(rows) * 100:5.1f}%")
        print()

    table("primary diagnosis", report["primary"])
    table("primary / subreason", report["primary_subreason"])
    table("surface family", report["surface_family"])
    table("atlas status", report["atlas_status"])
    table("boundary curve class", report["curve_class"])
    table("typed terminal reason", report["terminal_reason"])
    table("legacy loss bucket", report["derived_bucket"])
    table("source outer-bound standing", report["outer_standing"])
    if report["band_exit"]:
        table("band exit (eligible faces only)", report["band_exit"])
    table("declared vs certified periodicity, by family", report["periodicity"], 44)
    table("kind of work implied", report["work_kind"])
    table("model concentration", report["model_concentration"])

    print(f"  major populations (top {args.populations})")
    print(
        f"    {'faces':>7} {'mdl':>3}  {'primary/subreason':52} "
        f"{'surface':9} {'atlas':32} representative"
    )
    for population in population_rows[: args.populations]:
        rep = population["representative"]
        print(
            f"    {population['faces']:7} {population['models']:3}  "
            f"{population['primary'] + '/' + population['subreason']:52} "
            f"{population['surface_kind']:9} {population['atlas_status']:32} "
            f"{rep['model_id']}#{rep['source_face_id']} {rep['bound_signature']}"
        )
    print()
    for population in population_rows[: args.populations]:
        print(
            f"    {population['faces']:7}  {population['primary']}/"
            f"{population['subreason']} [{population['surface_kind']}]: "
            f"{population['gap']}"
        )
    return 0


if __name__ == "__main__":
    sys.exit(main())
