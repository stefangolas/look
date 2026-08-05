#!/usr/bin/env python3
"""Wave-2 Part A1: spline-on-carrier shadow classification report.

Joins the per-edge shadow certifications emitted by
``examples/spline_carrier_shadow.rs`` (``wave2-spline-shadow/shadow.jsonl``)
with the post-cone per-face diagnosis (``cone-final/<model>/diag.jsonl.gz``)
to classify the spline-bearing cylinder/cone lost faces into the Part A1
categories.  Shadow-only: no production admission, no rendering change.
"""
from __future__ import annotations

import collections
import gzip
import json
import os
import sys

SHADOW = r"C:\Users\stefa\look-corpus\wave2-spline-shadow\shadow.jsonl"
DIAG_ROOT = r"C:\Users\stefa\look-corpus\cone-final"
OUT_DIR = r"C:\Users\stefa\look-corpus\wave2-spline-shadow"

CERT_STRAIGHT = "spline_carrier_certified_straight_line"
CERT_COORD = "spline_carrier_certified_constant_coordinate"
CERT_CIRCLE = "spline_carrier_certified_circular_arc"
UNRESOLVED = {
    "spline_carrier_circle_within_rounding",
    "spline_carrier_circle_needs_bezier_form",
    "spline_carrier_trim_cuts_non_constant_span",
    "spline_carrier_denominator_sign_indeterminate",
    "spline_carrier_degenerate_candidate_circle",
}
INCONSISTENT = {
    "spline_carrier_not_collinear",
    "spline_carrier_coordinate_not_constant",
    "spline_carrier_not_on_circle",
    "spline_carrier_endpoint_inconsistent",
    "spline_carrier_denominator_sign_indefinite",
}
OPERATIONAL = "spline_carrier_operational_failure"
UNSUPPORTED = {
    "spline_carrier_not_a_spline_representation",
    "spline_carrier_non_rational_cannot_be_circular",
    "spline_carrier_proved_not_a_carrier",
    "spline_carrier_proved_not_collinear",
}


def stem_of(path: str) -> str:
    return os.path.splitext(os.path.basename(path))[0]


def face_key(source_face_id) -> str:
    return str(source_face_id).lstrip("#")


def load_shadow():
    """(model_stem, face_id) -> list of per-edge cert dicts."""
    faces = collections.defaultdict(list)
    with open(SHADOW, encoding="utf-8-sig") as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            r = json.loads(line)
            faces[(r["model"], r["face_id"].lstrip("#"))].append(r)
    return faces


def load_diag():
    """(model_stem, face_id) -> diag record, for lost cyl/cone faces."""
    records = {}
    all_lost = set()
    by_surface = collections.Counter()
    for dp, _, fns in os.walk(DIAG_ROOT):
        for fn in fns:
            if fn != "diag.jsonl.gz":
                continue
            with gzip.open(os.path.join(dp, fn), "rt", encoding="utf-8") as f:
                for line in f:
                    line = line.strip()
                    if not line:
                        continue
                    r = json.loads(line)
                    stem = stem_of(r["model_id"])
                    key = (stem, face_key(r["source_face_id"]))
                    all_lost.add(key)
                    sf = r.get("surface_family", "?")
                    by_surface[sf] += 1
                    if sf in ("Cylinder", "Cone"):
                        records[key] = r
    return records, all_lost, by_surface


def classify_face(edges):
    """A1 classification for one face from its per-edge cert tags."""
    n = len(edges)
    all_straight = all(e["straight"] == CERT_STRAIGHT for e in edges)
    all_coord = all(e["coord"] == CERT_COORD for e in edges)
    all_circle = all(e["circle"] == CERT_CIRCLE for e in edges)
    any_unresolved = any(
        e["straight"] in UNRESOLVED
        or e["coord"] in UNRESOLVED
        or e["circle"] in UNRESOLVED
        for e in edges
    )
    any_operational = any(
        e["straight"] == OPERATIONAL or e["coord"] == OPERATIONAL for e in edges
    )
    if all_straight:
        return "certified_sufficient_line"
    if all_circle:
        return "certified_sufficient_circle"
    if all_coord:
        return "certified_missing_traversal"
    if any_unresolved:
        return "unresolved_rounding"
    if any_operational:
        return "operational_failure"
    if any(
        e["straight"] in INCONSISTENT
        or e["coord"] in INCONSISTENT
        or e["circle"] in INCONSISTENT
        for e in edges
    ):
        return "inconsistent_geometry"
    return "unsupported_spline_form"


def main():
    shadow = load_shadow()
    diag, all_lost, by_surface = load_diag()

    # Target population: spline-bearing cyl/cone faces that were lost.
    target = []
    not_target_signature = 0
    for key, edges in shadow.items():
        if key in diag:
            target.append((key, edges, diag[key]))
        else:
            not_target_signature += 1  # spline-bearing but not a lost cyl/cone face

    missing = 0  # lost cyl/cone faces with no spline edge (not spline-only)
    for key in diag:
        if key not in shadow:
            missing += 1

    cats = collections.Counter()
    rows = []
    cert_dist = collections.Counter()
    for key, edges, rec in target:
        cat = classify_face(edges)
        cats[cat] += 1
        for e in edges:
            cert_dist[e["straight"]] += 1
            cert_dist[e["coord"]] += 1
            cert_dist[e["circle"]] += 1
        rows.append(
            {
                "model": key[0],
                "face_id": key[1],
                "surface_family": rec.get("surface_family"),
                "terminal_reason": rec.get("terminal_reason"),
                "derived_bucket": rec.get("derived_bucket"),
                "spline_edges": len(edges),
                "category": cat,
            }
        )

    os.makedirs(OUT_DIR, exist_ok=True)
    with open(os.path.join(OUT_DIR, "classified.jsonl"), "w", encoding="utf-8") as f:
        for r in rows:
            f.write(json.dumps(r) + "\n")

    summary = {
        "shadow_edges_total": sum(len(e) for e in shadow.values()),
        "shadow_faces_total": len(shadow),
        "diag_lost_faces_total": len(all_lost),
        "diag_cyl_cone_lost": len(diag),
        "diag_surface_family": dict(by_surface),
        "target_spline_bearing_cyl_cone_lost": len(target),
        "not_target_signature_spline_bearing": not_target_signature,
        "missing_lost_cyl_cone_no_spline": missing,
        "categories": dict(cats),
    }
    with open(os.path.join(OUT_DIR, "report.json"), "w", encoding="utf-8") as f:
        json.dump(summary, f, indent=2)

    print("=== Wave-2 Part A1: spline carrier shadow ===")
    print(json.dumps(summary, indent=2))
    print("\n=== category definitions ===")
    print("  certified_sufficient_line   -> all spline edges are certified straight lines (maps to Line family)")
    print("  certified_sufficient_circle -> all spline edges certified on the parallel circle (maps to CompleteCircle)")
    print("  certified_missing_traversal -> all edges certified constant-coordinate (circumferential parallel, no angular sweep)")
    print("  unresolved_rounding / operational_failure / inconsistent_geometry / unsupported_spline_form")


if __name__ == "__main__":
    main()
