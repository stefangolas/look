"""Post-cone non-band remainder refinement (NONBAND-REFINE-001).

Observer-only. Re-derives the primary/subreason taxonomy from the POST-CONE
ledger at ``cone-final/`` (the band-open census_diag + source_probe reading
taken under truck ``2b4537c4`` / look ``e2bf18ac``), adds the cone-band exit
map that ``remainder_report.py`` predates, joins each lost face to the
``band_curve_probe`` edge-level evidence at ``cone-band/``, and bins the
result into the refined taxonomy the wave asked for:

    recognized_cell_broken_realization   a NAMED cell (plane/cylinder/cone)
                                         was certified but realization stopped
    unnamed_homogeneous_atlas_cell       a homogeneous signature with no cell
                                         named yet (incl. quotient artifacts:
                                         synthetic-synthetic crossings on a
                                         certified-rank chart, and source-
                                         synthetic crossings where the cut
                                         plan crosses authoritative trim)
    general_arrangement_problem          arrangement defect on an aperiodic
                                         (rank-0) chart: synthetic closure
                                         self-crossing, overlap, mixed conflict
    material_semantic_ambiguity          MaterialAuthority: parity / no-region /
                                         outer-bound authority absent
    unsupported_curve_mathematics        CurveBoundaryWitness: spline projection
                                         or unreadable bound curve
    source_contradiction                 source-source crossing (candidate:
                                         needs a certified arrangement predicate
                                         or a proof the source self-intersects)
    insufficient_evidence               lift branch unresolved (needs boundary
                                         homology) or no typed reason retained
    surface_singularity                  cone band refused on apex / opposite-
                                         nappe witness evidence
    operational_failure                  SourceImport or unwitnessed mesh failure

Synthetic-synthetic crossings are treated as quotient artifacts (unnamed
cell), NOT as source contradictions, per the wave's required distinctions.

Nothing in ``src/``, ``truck-fork``, the canonical corpus scripts, or the
corpus itself is touched. Generated ledgers are written outside the repo.
"""

from __future__ import annotations

import argparse
import collections
import gzip
import hashlib
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import remainder_report as rr  # noqa: E402  reuse the pre-cone classify logic

SCHEMA_VERSION = "nonband-refine-1"

CONE_FINAL = Path("C:/Users/stefa/look-corpus/cone-final")
CONE_BAND = Path("C:/Users/stefa/look-corpus/cone-band")

# Cone-band exits, by what the exit establishes. The cone cell is now named,
# so a cone-band refusal is later evidence about the same face than the
# generic legacy bucket and wins over it -- exactly as the cylinder band
# exits do in ``remainder_report.BAND_EXIT_DIAGNOSIS``.
CONE_BAND_EXIT_DIAGNOSIS = {
    # A bound curve no structural reader reads. The cone band never got to
    # decide the cell because the boundary was not witnessed.
    "unsupported_curve_representation": (
        "CurveBoundaryWitness", "band_bound_curve_unreadable",
        "CellCandidateInsufficientEvidence",
    ),
    # The two bounds are not two essential parallels on ONE nappe of the
    # certified cone. Positive geometric evidence: the candidate band cell is
    # refuted. The apex / opposite-nappe stratum is the singularity concern
    # the cone packet's proof obligations flag.
    "cone_witness_start_not_on_cone": (
        "AtlasClassification", "band_witness_refuted", "ContradictoryCellEvidence",
    ),
    "cone_witness_circle_not_a_cone_parallel": (
        "AtlasClassification", "band_witness_refuted", "ContradictoryCellEvidence",
    ),
    "cone_band_bound_not_one_occurrence": (
        "AtlasClassification", "band_witness_refuted", "ContradictoryCellEvidence",
    ),
}

# The refined-taxonomy bin for each (primary, subreason) plus the auxiliary
# fields the assignment needs. Assigned in priority order in ``refined_bin``.
SURFACE_SINGULARITY_CONE_EXITS = {
    "cone_witness_start_not_on_cone", "cone_witness_circle_not_a_cone_parallel",
}


def refined_bin(row: dict) -> str:
    """Assign one refined-taxonomy bin. See module docstring for the order."""
    primary = row["primary"]
    subreason = row["subreason"]
    cone_band = row.get("cone_band", "not_eligible")
    band = row.get("band", "not_eligible")
    certified_rank = row.get("certified_rank")

    # 1. Operational: the face never reached a topological face, or a mesh
    #    failure left no witness.
    if primary == "SourceImport":
        return "operational_failure"
    if primary == "MeshRealization":
        if subreason == "vertex_insertion_failed":
            return "recognized_cell_broken_realization"
        return "operational_failure"

    # 2. Surface singularity: the cone band's own certifier refused on apex /
    #    opposite-nappe witness evidence. This is the one population the cone
    #    packet's proof obligations explicitly call out as a singular stratum.
    if cone_band in SURFACE_SINGULARITY_CONE_EXITS:
        return "surface_singularity"

    # 3. Recognized cell, broken realization: a NAMED cell was certified this
    #    far and the realization stopped (lift join / orientation / vertex
    #    insertion). The cell exists; the cut plan or the inserter does not.
    if subreason in (
        "band_lift_join_no_compatible_integer",
        "band_orientation_incompatible",
        "vertex_insertion_failed",
    ):
        return "recognized_cell_broken_realization"

    # 4. Unsupported curve mathematics: the boundary was never witnessed
    #    (spline projection / unreadable bound curve), including cone-band
    #    and cylinder-band unsupported_curve_representation refusals.
    if primary == "CurveBoundaryWitness":
        return "unsupported_curve_mathematics"
    if cone_band == "unsupported_curve_representation" or band == "unsupported_curve_representation":
        return "unsupported_curve_mathematics"

    # 5. Material-semantic ambiguity.
    if primary == "MaterialAuthority":
        return "material_semantic_ambiguity"

    # 6. Source contradiction candidate: source-source crossing. Not a
    #    quotient artifact -- source geometry is in conflict with itself.
    #    Discharged only by a certified arrangement predicate or a proof of
    #    genuine self-intersection.
    if subreason == "source_source_crossing":
        return "source_contradiction"

    # 7. periodic_lift_branch_unresolved: the lift branch could not be chosen
    #    because the boundary homology is not computed. The obstruction is the
    #    missing homology calculation (insufficient evidence), not a missing
    #    cell -- on cone/cylinder the cell already exists; on sphere it does
    #    not, but the *immediate* blocker is still the unchosen lift branch.
    if subreason == "periodic_lift_branch_unresolved":
        return "insufficient_evidence"

    # 8. Quotient artifacts -> unnamed homogeneous atlas cell. Synthetic-
    #    synthetic crossings on a certified-rank chart (the band packet's
    #    signature) and source-synthetic crossings on a certified-rank chart
    #    (the cut plan crosses authoritative source trim) are both symptoms of
    #    a wrong or absent quotient cut plan, NOT source defects.
    #    deck_generator_uncertified belongs here too: a period is declared but
    #    uncertified, so no quotient can form -- the cell (and its witness)
    #    is the missing theorem.
    if primary == "AtlasClassification":
        return "unnamed_homogeneous_atlas_cell"
    if subreason == "source_synthetic_crossing" and certified_rank is not None and certified_rank >= 1:
        return "unnamed_homogeneous_atlas_cell"

    # 9. General arrangement problem: aperiodic (rank-0) chart arrangement
    #    defects -- synthetic closure self-crossing, overlap, mixed conflict,
    #    and source-synthetic crossings where there is no quotient to blame.
    if subreason in (
        "synthetic_closure_self_crossing", "overlap_unsupported", "mixed_conflict",
    ):
        return "general_arrangement_problem"
    if subreason == "source_synthetic_crossing":
        return "general_arrangement_problem"

    # 10. Fallback.
    return "insufficient_evidence"


# The exact missing proof obligation per refined bin + signature. One line,
# in the vocabulary the wave asked for.
PROOF_OBLIGATION = {
    ("unnamed_homogeneous_atlas_cell", "torus", "deck_generator_uncertified"):
        "Certify a representation-derived rank-2 toroidal period witness (both "
        "angular generators), then name the toroidal essential-band cell and "
        "its quotient cut plan. The band route already handles the malformed "
        "double-outer-bound normalization; the blocker is the period witness "
        "and the cell, not the arrangement.",
    ("unnamed_homogeneous_atlas_cell", "torus", "quotient_cell_not_named"):
        "Same as deck_generator_uncertified once a generator is certified: name "
        "the toroidal band cell. (No post-cone torus face reaches this bin "
        "today because the generator is uncertified.)",
    ("unnamed_homogeneous_atlas_cell", "cone", "quotient_cell_not_named"):
        "Extend the cone band cell to spline-bounded cones (the 2-circle cell "
        "is named; the spline-bound quotient is not). Needs an exact "
        "curve-on-cone witness for the B-spline bound before the quotient cut "
        "plan can be stated.",
    ("unnamed_homogeneous_atlas_cell", "cylinder", "quotient_cell_not_named"):
        "Extend the cylinder band cell to spline-bounded cylinders, or name a "
        "different cylinder cell for the bound signature. Needs an exact "
        "curve-on-cylinder witness for the spline bound.",
    ("unnamed_homogeneous_atlas_cell", "cone", "source_synthetic_crossing"):
        "State a quotient cut plan for the cone that cannot cross authoritative "
        "source trim. The cone cell exists; the cut plan is the gap.",
    ("unnamed_homogeneous_atlas_cell", "cylinder", "source_synthetic_crossing"):
        "State a quotient cut plan for the cylinder that cannot cross "
        "authoritative source trim. The cylinder cell exists; the cut plan is "
        "the gap.",
    ("unnamed_homogeneous_atlas_cell", "cylinder", "band_witness_refuted"):
        "Name a different cylinder cell: the band candidate is refuted on "
        "positive evidence (witness not on the surface / not a parallel / sweep "
        "short / non-constant axial coordinate).",
    ("unnamed_homogeneous_atlas_cell", "cone", "band_witness_refuted"):
        "Name a different cone cell or normalize the bound: the cone band "
        "refused on bound-not-one-occurrence (the bound is not a single "
        "complete source circle).",
    ("recognized_cell_broken_realization", "cylinder", "band_lift_join_no_compatible_integer"):
        "Prove a compatible deck translate joins the lift, or prove none exists "
        "(the cell is certified; the realization gap is the deck-translate join).",
    ("recognized_cell_broken_realization", "cylinder", "band_orientation_incompatible"):
        "Prove the carrier orientations bound a strip, or prove they do not.",
    ("unsupported_curve_mathematics", "nurbs", "no_certified_preimage_on_support"):
        "Compute an exact curve-on-surface preimage for a NURBS bound on a "
        "NURBS support (free-form / free-form intersection).",
    ("unsupported_curve_mathematics", "bspline", "no_certified_preimage_on_support"):
        "Compute an exact curve-on-surface preimage for a B-spline bound on a "
        "B-spline support (free-form / free-form intersection).",
    ("unsupported_curve_mathematics", "cylinder", "band_bound_curve_unreadable"):
        "Compute an exact curve-on-cylinder witness for the unread spline / "
        "non-circular conic bound.",
    ("unsupported_curve_mathematics", "cone", "band_bound_curve_unreadable"):
        "Compute an exact curve-on-cone witness for the unread spline bound.",
    ("unsupported_curve_mathematics", "torus", "no_certified_preimage_on_support"):
        "Compute an exact curve-on-torus preimage for the spline bound (and "
        "first certify the torus period witness).",
    ("material_semantic_ambiguity", "cylinder", "parity_contradiction"):
        "Certify an arrangement predicate that resolves the dual-parity flood, "
        "or prove the source material regions are genuinely inconsistent.",
    ("material_semantic_ambiguity", "plane", "parity_contradiction"):
        "Certify an arrangement predicate for the rank-0 chart, or prove the "
        "source material regions are genuinely inconsistent.",
    ("material_semantic_ambiguity", "cylinder", "no_material_region"):
        "Prove material-region uniqueness without an outer-bound declaration; "
        "the parity flood selected nothing.",
    ("material_semantic_ambiguity", "cylinder", "band_outer_bound_authority_absent"):
        "Prove material-region uniqueness without an outer-bound declaration.",
    ("unsupported_curve_mathematics", "cylinder", "no_certified_preimage_on_support"):
        "Compute an exact curve-on-cylinder preimage for the bound (the "
        "projection failed on a certified-rank-1 cylinder; the cell is not "
        "the blocker, the curve witness is).",
    ("unsupported_curve_mathematics", "cone", "no_certified_preimage_on_support"):
        "Compute an exact curve-on-cone preimage for the spline bound.",
    ("unsupported_curve_mathematics", "plane", "no_certified_preimage_on_support"):
        "Compute an exact curve-on-plane preimage for the spline bound (a "
        "free-form planar curve that did not project cleanly).",
    ("unsupported_curve_mathematics", "offset", "no_certified_preimage_on_support"):
        "Compute an exact curve-on-offset-surface preimage for the bound.",
    ("unsupported_curve_mathematics", "extruded", "no_certified_preimage_on_support"):
        "Compute an exact curve-on-extruded-surface preimage for the bound.",
    ("unsupported_curve_mathematics", "revolved", "no_certified_preimage_on_support"):
        "Compute an exact curve-on-revolved-surface preimage for the bound.",
    ("insufficient_evidence", "cone", "periodic_lift_branch_unresolved"):
        "Compute the boundary homology of the cone face so the periodic lift "
        "branch can be chosen (the cone cell exists; the homology is the gap).",
    ("insufficient_evidence", "cylinder", "periodic_lift_branch_unresolved"):
        "Compute the boundary homology of the cylinder face so the periodic "
        "lift branch can be chosen (the cylinder cell exists; the homology is "
        "the gap).",
    ("insufficient_evidence", "sphere", "periodic_lift_branch_unresolved"):
        "Compute the boundary homology of the sphere face so the periodic lift "
        "branch can be chosen; a sphere cell and its rank-2 period witness are "
        "also missing.",
    ("insufficient_evidence", "torus", "periodic_lift_branch_unresolved"):
        "Compute the boundary homology of the torus face; the torus period "
        "witness and toroidal cell are also missing.",
    ("material_semantic_ambiguity", "plane", "no_material_region"):
        "Prove material-region uniqueness on the rank-0 chart without an "
        "outer-bound declaration; the parity flood selected nothing.",
    ("material_semantic_ambiguity", "bspline", "no_material_region"):
        "Prove material-region uniqueness on the B-spline chart; the parity "
        "flood selected nothing.",
    ("material_semantic_ambiguity", "torus", "parity_contradiction"):
        "Certify an arrangement predicate on the torus, or prove the source "
        "material regions are genuinely inconsistent (the torus generator is "
        "also uncertified).",
    ("material_semantic_ambiguity", "cone", "parity_contradiction"):
        "Certify an arrangement predicate on the cone, or prove the source "
        "material regions are genuinely inconsistent.",
    ("material_semantic_ambiguity", "bspline", "parity_contradiction"):
        "Certify an arrangement predicate on the B-spline chart, or prove the "
        "source material regions are genuinely inconsistent.",
    ("material_semantic_ambiguity", "nurbs", "parity_contradiction"):
        "Certify an arrangement predicate on the NURBS chart, or prove the "
        "source material regions are genuinely inconsistent.",
    ("material_semantic_ambiguity", "sphere", "parity_contradiction"):
        "Certify an arrangement predicate on the sphere, or prove the source "
        "material regions are genuinely inconsistent.",
    ("material_semantic_ambiguity", "extruded", "parity_contradiction"):
        "Certify an arrangement predicate on the extruded surface, or prove "
        "the source material regions are genuinely inconsistent.",
    ("material_semantic_ambiguity", "revolved", "parity_contradiction"):
        "Certify an arrangement predicate on the revolved surface, or prove "
        "the source material regions are genuinely inconsistent.",
    ("material_semantic_ambiguity", "plane", "parity_contradiction"):
        "Certify an arrangement predicate on the rank-0 chart, or prove the "
        "source material regions are genuinely inconsistent.",
    ("material_semantic_ambiguity", "torus", "no_material_region"):
        "Prove material-region uniqueness on the torus; the parity flood "
        "selected nothing (the torus generator is also uncertified).",
    ("material_semantic_ambiguity", "sphere", "no_material_region"):
        "Prove material-region uniqueness on the sphere; the parity flood "
        "selected nothing.",
    ("material_semantic_ambiguity", "cone", "no_material_region"):
        "Prove material-region uniqueness on the cone; the parity flood "
        "selected nothing.",
    ("material_semantic_ambiguity", "plane", "band_outer_bound_authority_absent"):
        "Prove material-region uniqueness on the rank-0 chart without an "
        "outer-bound declaration.",
    ("source_contradiction", "plane", "source_source_crossing"):
        "Certify an arrangement predicate admitting or refuting source "
        "self-intersection on a rank-0 chart, or prove the source wires "
        "genuinely cross.",
    ("source_contradiction", "cylinder", "source_source_crossing"):
        "Certify an arrangement predicate on the cylinder, or prove the source "
        "wires genuinely cross.",
    ("source_contradiction", "cone", "source_source_crossing"):
        "Certify an arrangement predicate on the cone, or prove the source "
        "wires genuinely cross.",
    ("source_contradiction", "torus", "source_source_crossing"):
        "Certify an arrangement predicate on the torus, or prove the source "
        "wires genuinely cross (the torus generator is also uncertified).",
    ("source_contradiction", "sphere", "source_source_crossing"):
        "Certify an arrangement predicate on the sphere, or prove the source "
        "wires genuinely cross.",
    ("source_contradiction", "bspline", "source_source_crossing"):
        "Certify an arrangement predicate on the B-spline chart, or prove the "
        "source wires genuinely cross.",
    ("source_contradiction", "nurbs", "source_source_crossing"):
        "Certify an arrangement predicate on the NURBS chart, or prove the "
        "source wires genuinely cross.",
    ("source_contradiction", "extruded", "source_source_crossing"):
        "Certify an arrangement predicate on the extruded surface, or prove "
        "the source wires genuinely cross.",
    ("source_contradiction", "revolved", "source_source_crossing"):
        "Certify an arrangement predicate on the revolved surface, or prove "
        "the source wires genuinely cross.",
    ("source_contradiction", "offset", "source_source_crossing"):
        "Certify an arrangement predicate on the offset surface, or prove the "
        "source wires genuinely cross.",
    ("general_arrangement_problem", "plane", "overlap_unsupported"):
        "Certify an arrangement predicate admitting collinear overlap on a "
        "rank-0 chart.",
    ("general_arrangement_problem", "cylinder", "overlap_unsupported"):
        "Certify an arrangement predicate admitting collinear overlap on the "
        "cylinder (rank-0 boundary chart region).",
    ("general_arrangement_problem", "cone", "overlap_unsupported"):
        "Certify an arrangement predicate admitting collinear overlap on the "
        "cone.",
    ("general_arrangement_problem", "torus", "overlap_unsupported"):
        "Certify an arrangement predicate admitting collinear overlap on the "
        "torus (the torus generator is also uncertified).",
    ("general_arrangement_problem", "plane", "synthetic_closure_self_crossing"):
        "Prove a rank-0 cut plan whose synthetic closure does not self-cross, "
        "or prove no such plan exists.",
    ("general_arrangement_problem", "plane", "mixed_conflict"):
        "Certify an arrangement predicate for mixed conflict classes on a "
        "rank-0 chart.",
    ("general_arrangement_problem", "torus", "source_synthetic_crossing"):
        "State a quotient cut plan for the torus that cannot cross "
        "authoritative source trim (prerequisite: the torus period witness).",
    ("general_arrangement_problem", "extruded", "source_synthetic_crossing"):
        "State a quotient cut plan for the extruded surface that cannot cross "
        "authoritative source trim.",
    ("general_arrangement_problem", "bspline", "source_synthetic_crossing"):
        "State a quotient cut plan for the B-spline surface that cannot cross "
        "authoritative source trim.",
    ("general_arrangement_problem", "sphere", "source_synthetic_crossing"):
        "State a quotient cut plan for the sphere that cannot cross "
        "authoritative source trim (prerequisite: a sphere period witness).",
    ("general_arrangement_problem", "nurbs", "source_synthetic_crossing"):
        "State a quotient cut plan for the NURBS surface that cannot cross "
        "authoritative source trim.",
    ("general_arrangement_problem", "revolved", "source_synthetic_crossing"):
        "State a quotient cut plan for the revolved surface that cannot cross "
        "authoritative source trim.",
    ("unnamed_homogeneous_atlas_cell", "sphere", "deck_generator_uncertified"):
        "Certify a representation-derived rank-2 spherical period witness, "
        "then name the spherical atlas cell.",
    ("insufficient_evidence", "cone", "periodic_lift_branch_unresolved"):
        "Compute the boundary homology of the cone face so the periodic lift "
        "branch can be chosen.",
    ("insufficient_evidence", "cylinder", "periodic_lift_branch_unresolved"):
        "Compute the boundary homology of the cylinder face so the periodic "
        "lift branch can be chosen.",
    ("surface_singularity", "cone", "band_witness_refuted"):
        "Prove the two carriers lie on one nappe of the apex (same-nappe "
        "obligation, both radii nonzero), or name a cell for the opposite-nappe "
        "/ apex-crossing case the cone band correctly refuses.",
    ("operational_failure", "-", "AllBoundsCollapsed"):
        "Retain the collapsed-bound case in the importer, or confirm the source "
        "is genuinely degenerate.",
    ("operational_failure", "-", "EdgeCurveConversionFailed"):
        "Retain the source curve in the importer; needs the raw entity to say "
        "which.",
    ("operational_failure", "-", "SurfaceConversionFailed"):
        "Retain the source surface in the importer; needs the raw entity to say "
        "which.",
}


# Fallbacks keyed on (bin, subreason) and (bin,) so no population is left
# without a stated missing proof obligation.
OBLIGATION_FALLBACK = {
    ("general_arrangement_problem", "overlap_unsupported"):
        "Certify an arrangement predicate admitting collinear overlap.",
    ("general_arrangement_problem", "mixed_conflict"):
        "Certify an arrangement predicate for mixed conflict classes.",
    ("general_arrangement_problem", "synthetic_closure_self_crossing"):
        "Prove a rank-0 cut plan whose synthetic closure does not self-cross.",
    ("general_arrangement_problem", "source_synthetic_crossing"):
        "State a quotient cut plan that cannot cross authoritative source trim.",
    ("unnamed_homogeneous_atlas_cell", "deck_generator_uncertified"):
        "Certify a representation-derived period witness, then name the atlas "
        "cell and its quotient cut plan.",
    ("unnamed_homogeneous_atlas_cell", "quotient_cell_not_named"):
        "Name the atlas cell for this (surface, boundary-homology) signature "
        "and its quotient cut plan.",
    ("unnamed_homogeneous_atlas_cell", "band_witness_refuted"):
        "Name a different atlas cell: the band candidate is refuted on "
        "positive evidence.",
    ("unnamed_homogeneous_atlas_cell", "source_synthetic_crossing"):
        "State a quotient cut plan that cannot cross authoritative source trim.",
    ("insufficient_evidence", "periodic_lift_branch_unresolved"):
        "Compute the boundary homology so the periodic lift branch can be "
        "chosen.",
    ("unsupported_curve_mathematics", "no_certified_preimage_on_support"):
        "Compute an exact curve-on-surface preimage for the bound.",
    ("unsupported_curve_mathematics", "band_bound_curve_unreadable"):
        "Compute an exact curve-on-surface witness for the unread bound curve.",
    ("material_semantic_ambiguity", "parity_contradiction"):
        "Certify an arrangement predicate, or prove the source material "
        "regions are genuinely inconsistent.",
    ("material_semantic_ambiguity", "no_material_region"):
        "Prove material-region uniqueness; the parity flood selected nothing.",
    ("material_semantic_ambiguity", "band_outer_bound_authority_absent"):
        "Prove material-region uniqueness without an outer-bound declaration.",
    ("source_contradiction", "source_source_crossing"):
        "Certify an arrangement predicate, or prove the source wires genuinely "
        "cross.",
    ("recognized_cell_broken_realization", "band_lift_join_no_compatible_integer"):
        "Prove a compatible deck translate joins the lift, or prove none exists.",
    ("recognized_cell_broken_realization", "band_orientation_incompatible"):
        "Prove the carrier orientations bound a strip, or prove they do not.",
    ("surface_singularity", "band_witness_refuted"):
        "Prove the carriers avoid the singular stratum, or name a cell for the "
        "apex / opposite-nappe case.",
    ("operational_failure", "AllBoundsCollapsed"):
        "Retain the collapsed-bound case in the importer, or confirm the "
        "source is genuinely degenerate.",
    ("operational_failure", "EdgeCurveConversionFailed"):
        "Retain the source curve in the importer.",
    ("operational_failure", "SurfaceConversionFailed"):
        "Retain the source surface in the importer.",
    ("operational_failure", "constraint_role_missing"):
        "Retain the constraint role in the mesh realizer.",
}


def obligation(row: dict) -> str:
    key = (row["refined_bin"], row["surface_kind"], row["subreason"])
    if key in PROOF_OBLIGATION:
        return PROOF_OBLIGATION[key]
    sub = (row["refined_bin"], row["subreason"])
    if sub in OBLIGATION_FALLBACK:
        return OBLIGATION_FALLBACK[sub]
    if row["refined_bin"] in OBLIGATION_FALLBACK:
        return OBLIGATION_FALLBACK[row["refined_bin"]]
    return f"unmapped obligation for {row['refined_bin']}/{row['surface_kind']}/"
    f"{row['subreason']}"



def diagnose(row: dict) -> tuple[str, str, str]:
    """Classify with cone-band exits mapped. Mirrors ``rr.diagnose`` but
    checks ``cone_band`` before the terminal-reason fall-through."""
    # 1. Convert-stage loss.
    if row["stage"] == "convert":
        return "SourceImport", row["reason"], "NotReached"

    # 2. A named cone cell's own certifier refused.
    cone_band = row.get("cone_band", "not_eligible")
    if row.get("surface_kind") == "cone" and cone_band in CONE_BAND_EXIT_DIAGNOSIS:
        return CONE_BAND_EXIT_DIAGNOSIS[cone_band]

    # 3. A named cylinder cell's own certifier refused (unchanged from rr).
    band = row.get("band", "not_eligible")
    if band in rr.BAND_EXIT_DIAGNOSIS:
        return rr.BAND_EXIT_DIAGNOSIS[band]

    # 4. Fall through to the pre-cone terminal-reason logic.
    return rr.diagnose(row)


def load_curve_probe(model_dir: Path) -> dict[str, list[dict]]:
    """per-face edge-level verdicts from band_curve_probe (cone-band run)."""
    path = CONE_BAND / model_dir.name / "curves.tsv.gz"
    out: dict[str, list[dict]] = collections.defaultdict(list)
    text = rr.read_gz(path)
    for line in text.splitlines():
        if not line.startswith("EDGE\t"):
            continue
        fields = {}
        for piece in line.split("\t")[1:]:
            if "=" in piece:
                k, v = piece.split("=", 1)
                fields[k] = v
        fid = rr.normalise_id(fields.get("source_face_id"))
        if fid:
            out[fid].append(fields)
    return out


def load_model(model_dir: Path, curve_probes: dict[str, list[dict]]):
    probes: dict[str, dict] = {}
    for line in rr.read_gz(model_dir / "faces.tsv.gz").splitlines():
        f = rr.parse_kv(line, "FACE\t")
        if f is None:
            continue
        k = rr.normalise_id(f.get("source_face_id"))
        if k and k not in probes:
            probes[k] = f

    diags: dict[str, dict] = {}
    for line in rr.read_gz(model_dir / "diag.jsonl.gz").splitlines():
        if not line.strip():
            continue
        rec = json.loads(line)
        k = rr.normalise_id(
            None if rec.get("source_face_id") is None else str(rec["source_face_id"])
        )
        if k and k not in diags:
            diags[k] = rec

    rows = []
    declared = rendered = 0
    for line in rr.read_gz(model_dir / "ledger.tsv.gz").splitlines():
        f = rr.parse_kv(line, "FACE\t")
        if f is None:
            continue
        declared += 1
        if f.get("rendered") == "1":
            rendered += 1
            continue
        k = rr.normalise_id(f.get("source_face_id"))
        probe = probes.get(k, {}) if k else {}
        diag = diags.get(k, {}) if k else {}
        row = {
            "model_id": model_dir.name,
            "source_face_id": k or "-",
            "stage": f.get("stage", "-"),
            "reason": f.get("reason", "-"),
            "band": f.get("band", "not_eligible"),
            "cone_band": f.get("cone_band", "not_eligible"),
            "surface_kind": f.get("surface_kind", "-"),
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
        row["curve_class"] = rr.curve_class(row["curves"])
        primary, subreason, atlas = diagnose(row)
        row["primary"], row["subreason"], row["atlas_status"] = primary, subreason, atlas
        row["refined_bin"] = refined_bin(row)
        row["obligation"] = obligation({**row, "refined_bin": row["refined_bin"]})
        # curve-probe summary
        edges = curve_probes.get(k, []) if k else []
        row["probe_edges"] = len(edges)
        row["probe_causes"] = ",".join(
            sorted({e.get("cause", "-") for e in edges}))
        row["probe_shadows"] = ",".join(
            sorted({e.get("shadow", "-") for e in edges}))
        row["has_spline_cause"] = any(
            e.get("cause") in ("b_spline_curve", "rational_b_spline_curve")
            for e in edges)
        rows.append(row)
    return rows, {"declared": declared, "rendered": rendered,
                  "lost": declared - rendered}


FIELDS = [
    "model_id", "source_face_id", "refined_bin", "primary", "subreason",
    "atlas_status", "surface_kind", "curve_class", "stage", "reason",
    "band", "cone_band", "terminal_reason", "derived_bucket", "chart_rank",
    "certified_rank", "declared_rank", "bound_count", "bounds", "edge_uses",
    "outer_standing", "outer_declared_count", "lift_status", "deck_status",
    "projection_status", "conflict_count", "source_segments",
    "synthetic_segments", "support", "cylinder", "curves", "bound_signature",
    "unread_rank1", "probe_edges", "probe_causes", "probe_shadows",
    "has_spline_cause", "has_probe", "has_diag", "obligation",
]


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--cone-final", default=str(CONE_FINAL))
    parser.add_argument("--cone-band", default=str(CONE_BAND))
    parser.add_argument("--json", default=None)
    parser.add_argument("--ledger", default=None,
                        help="full per-face TSV.gz (outside the repo)")
    parser.add_argument("--populations", type=int, default=30)
    args = parser.parse_args()

    out = Path(args.cone_final).resolve()
    band = Path(args.cone_band).resolve()
    index = json.loads((out / "index.json").read_text())
    revisions = {}
    statuses = collections.Counter()
    for record in index.values():
        revisions = {
            "look": record["look_rev"], "truck": record["truck_rev"],
            "cargo_lock": record["cargo_lock"],
            "schema": record.get("schema", SCHEMA_VERSION),
        }
        statuses[record["outcome"]] += 1

    rows: list[dict] = []
    totals = collections.Counter()
    per_model = {}
    for directory in sorted(p for p in out.iterdir() if p.is_dir()):
        model_rows, model_totals = load_model(
            directory, load_curve_probe(directory))
        if model_totals["declared"] == 0:
            continue
        rows.extend(model_rows)
        per_model[directory.name] = model_totals
        for k, v in model_totals.items():
            totals[k] += v

    assert len(rows) == totals["lost"], (len(rows), totals["lost"])

    def hist(key):
        return collections.Counter(
            key(r) if not isinstance(key, str) else r[key] for r in rows)

    primary_hist = hist("primary")
    refined_hist = hist("refined_bin")

    # Refined bin x surface x subreason populations.
    populations = collections.Counter(
        (r["refined_bin"], r["surface_kind"], r["subreason"]) for r in rows)
    population_rows = []
    for sig, count in populations.most_common():
        members = [r for r in rows
                   if (r["refined_bin"], r["surface_kind"], r["subreason"]) == sig]
        models = collections.Counter(r["model_id"] for r in members)
        bsig = collections.Counter(r["bound_signature"] for r in members)
        rep = min(members, key=lambda r: (
            r["model_id"], int(r["source_face_id"])
            if r["source_face_id"].isdigit() else 0))
        spline_members = sum(1 for r in members if r["has_spline_cause"])
        population_rows.append({
            "refined_bin": sig[0], "surface_kind": sig[1], "subreason": sig[2],
            "faces": count, "models": len(models),
            "top_models": models.most_common(4),
            "top_bound_signatures": bsig.most_common(3),
            "spline_probe_faces": spline_members,
            "certified_rank": rep["certified_rank"],
            "declared_rank": rep["declared_rank"],
            "outer_standing": rep["outer_standing"],
            "representative": {
                "model_id": rep["model_id"],
                "source_face_id": rep["source_face_id"],
                "bound_signature": rep["bound_signature"],
                "terminal_reason": rep["terminal_reason"],
                "derived_bucket": rep["derived_bucket"],
                "band": rep["band"], "cone_band": rep["cone_band"],
                "certified_rank": rep["certified_rank"],
                "probe_causes": rep["probe_causes"],
            },
            "obligation": members[0]["obligation"],
        })

    report = {
        "revisions": revisions, "run_outcomes": dict(statuses),
        "totals": {
            "declared": totals["declared"], "rendered": totals["rendered"],
            "lost": totals["lost"], "classified": len(rows),
            "rows_with_source_probe": sum(1 for r in rows if r["has_probe"]),
            "rows_with_diagnosis": sum(1 for r in rows if r["has_diag"]),
            "rows_with_curve_probe": sum(1 for r in rows if r["probe_edges"]),
        },
        "per_model": per_model,
        "primary": dict(primary_hist.most_common()),
        "primary_subreason": {
            f"{a}/{b}": c for (a, b), c in
            collections.Counter((r["primary"], r["subreason"])
                                for r in rows).most_common()},
        "refined_bin": dict(refined_hist.most_common()),
        "refined_bin_x_surface": {
            f"{a}/{b}": c for (a, b), c in
            collections.Counter((r["refined_bin"], r["surface_kind"])
                                for r in rows).most_common()},
        "surface_family": dict(hist("surface_kind").most_common()),
        "atlas_status": dict(hist("atlas_status").most_common()),
        "curve_class": dict(hist("curve_class").most_common()),
        "cone_band_exit": dict(collections.Counter(
            r["cone_band"] for r in rows
            if r["surface_kind"] == "cone"
            and r["cone_band"] != "not_eligible").most_common()),
        "torus_periodicity": dict(collections.Counter(
            f"declared={r['declared_rank']} certified={r['certified_rank']}"
            for r in rows if r["surface_kind"] == "torus").most_common()),
        "spline_faces": sum(1 for r in rows if r["has_spline_cause"]),
        "spline_faces_by_surface": dict(collections.Counter(
            r["surface_kind"] for r in rows if r["has_spline_cause"]).most_common()),
        "populations": population_rows,
    }

    if args.ledger:
        ledger_path = Path(args.ledger)
        ledger_path.parent.mkdir(parents=True, exist_ok=True)
        body = "\t".join(FIELDS) + "\n"
        body += "".join(
            "\t".join("" if r.get(f) is None else str(r.get(f, ""))
                      for f in FIELDS) + "\n"
            for r in sorted(rows, key=lambda r: (
                r["model_id"], int(r["source_face_id"])
                if r["source_face_id"].isdigit() else 0)))
        data = body.encode("utf-8")
        with gzip.open(ledger_path, "wb") as h:
            h.write(data)
        report["ledger"] = {
            "path": str(ledger_path), "rows": len(rows),
            "sha256": hashlib.sha256(data).hexdigest(),
            "schema": SCHEMA_VERSION, "fields": FIELDS,
        }

    if args.json:
        Path(args.json).write_text(json.dumps(report, indent=1))

    # -- human-readable ---------------------------------------------------
    t = report["totals"]
    print(f"look={revisions.get('look')} truck={revisions.get('truck')} "
          f"lock={revisions.get('cargo_lock')} schema={revisions.get('schema')}")
    print(f"run outcomes: {dict(statuses)}")
    print(f"\n{t['declared']} declared, {t['rendered']} rendered, "
          f"{t['lost']} lost ({t['lost']/t['declared']*100:.2f}%)")
    print(f"  classified {t['classified']}/{t['lost']}, "
          f"source probe {t['rows_with_source_probe']}, "
          f"diag {t['rows_with_diagnosis']}, "
          f"curve probe {t['rows_with_curve_probe']}")

    def table(title, mapping, width=46):
        print(f"\n  {title}")
        for name, count in mapping.items():
            print(f"    {str(name):{width}} {count:7}  "
                  f"{count/len(rows)*100:5.1f}%")

    table("primary diagnosis (post-cone, cone_band mapped)", report["primary"])
    table("primary / subreason", report["primary_subreason"], 52)
    table("refined taxonomy bin", report["refined_bin"])
    table("refined bin x surface", report["refined_bin_x_surface"], 52)
    table("surface family", report["surface_family"])
    table("atlas status", report["atlas_status"])
    table("boundary curve class", report["curve_class"])
    print(f"\n  cone_band exits (lost cone faces only)")
    for k, v in report["cone_band_exit"].items():
        print(f"    {k:44} {v:7}")
    print(f"\n  torus periodicity (lost torus faces)")
    for k, v in report["torus_periodicity"].items():
        print(f"    {k:30} {v:7}")
    print(f"\n  spline-bound lost faces (curve probe): {report['spline_faces']}")
    for k, v in report["spline_faces_by_surface"].items():
        print(f"    {k:14} {v:7}")

    print(f"\n  ranked populations (top {args.populations})")
    print(f"    {'faces':>6} {'mdl':>3}  {'bin/surface/subreason':60} "
          f"{'cr':>3} representative")
    for p in population_rows[:args.populations]:
        rep = p["representative"]
        print(f"    {p['faces']:6} {p['models']:3}  "
              f"{p['refined_bin']+'/'+p['surface_kind']+'/'+p['subreason']:60} "
              f"{str(rep['certified_rank']):>3} "
              f"{rep['model_id']}#{rep['source_face_id']} {rep['bound_signature']}")
    print()
    for p in population_rows[:args.populations]:
        print(f"    {p['faces']:6}  {p['refined_bin']}/{p['surface_kind']}/"
              f"{p['subreason']}: {p['obligation']}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
