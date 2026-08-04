"""Aggregate an ABC band sweep into reconciled populations.

Reads only what `band_sweep.py` recorded, joins it to the raw STEP entity graph
via `step_entity_chain`, and reconciles. It computes every population twice
where it can — once from the per-face ledger and once from the binary's own
printed summary — and reports a mismatch rather than picking one, because a
census that silently prefers one of two disagreeing sources is how a wrong
number survives.

Face identity is `source_face_id`, the STEP entity id. Never
`declared_face_index`: that is per-shell and collides across shells, so a
mode-to-mode comparison keyed on it compares different faces.

Usage:

    python benchmarks/band_report.py --out sweep-out --corpus C:/.../abc \\
        --json sweep-out/report.json
"""

from __future__ import annotations

import argparse
import collections
import gzip
import json
import os
import sys
from pathlib import Path

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from step_entity_chain import EntityIndex, face_edge_chains  # noqa: E402

MODES = ("gate_closed", "band_enabled")


def read_gz(path: Path) -> str:
    if not path.exists():
        return ""
    with gzip.open(path, "rb") as handle:
        return handle.read().decode("utf-8", "replace")


def face_key(raw: str) -> str:
    """One spelling of a face identity.

    The ledger prints `EntityId`'s `Display` (`#45584`) and the curve probe
    prints its numeric value (`45584`). Joining the two on the raw strings
    silently matches nothing and reports every face as having no unread edge,
    which is indistinguishable from the population being empty.
    """
    return raw.lstrip("#").strip()


def parse_ledger(text: str):
    """`source_face_id -> row` for tessellated faces, plus conversion losses.

    A face lost before conversion never reaches the tessellator and has no band
    verdict; it is kept in its own bucket so the two are never summed.
    """
    faces, converts = {}, []
    for line in text.splitlines():
        if not line.startswith("FACE\t"):
            continue
        row = dict(
            kv.split("=", 1) for kv in line.split("\t")[1:] if "=" in kv
        )
        if row.get("stage") == "convert":
            converts.append(row)
            continue
        faces[face_key(row.get("source_face_id", "-"))] = row
    return faces, converts


def parse_summary(text: str):
    """The binary's own totals, for cross-checking the ledger."""
    out = {}
    for line in text.splitlines():
        if "faces declared," in line:
            parts = line.replace(",", " ").split()
            out["declared"] = int(parts[2])
            out["rendered"] = int(parts[5])
            out["lost"] = int(parts[7])
        if line.startswith("cylinder band:"):
            parts = line.replace(",", " ").split()
            out["band_eligible"] = int(parts[2])
            out["band_recovered"] = int(parts[4])
            out["band_refused"] = int(parts[6])
    return out


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--out", default="sweep-out")
    parser.add_argument("--corpus", default="C:/Users/stefa/look-corpus/abc")
    parser.add_argument("--json", default="")
    args = parser.parse_args()

    out = Path(args.out).resolve()
    index = json.loads((out / "index.json").read_text())

    runs = collections.defaultdict(dict)
    for entry in index.values():
        runs[entry["model_id"]][entry["mode"]] = entry

    report = {"models": {}, "revisions": {}}
    any_entry = next(iter(index.values()))
    report["revisions"] = {
        "look": any_entry["look_rev"],
        "truck": any_entry["truck_rev"],
        "cargo_lock": any_entry["cargo_lock"],
    }

    totals = collections.Counter()
    exits = collections.Counter()
    exits_by_model = collections.defaultdict(collections.Counter)
    conformance = collections.Counter()
    regressions = []
    mismatches = []
    outcomes = collections.Counter()

    # Occurrence-level and face-level populations behind the unsupported exit.
    raw_occurrence = collections.Counter()
    raw_face_signature = collections.Counter()
    cause_occurrence = collections.Counter()
    cause_face_signature = collections.Counter()
    wrapper_chains = collections.Counter()
    pcurve_presence = collections.Counter()
    ulps_buckets = collections.Counter()
    unsupported_faces_by_model = collections.Counter()
    join_faces_by_model = collections.Counter()
    join_signatures = collections.Counter()
    near_circle_faces = collections.Counter()
    spline_only_faces = collections.Counter()
    mixed_or_true_conic_faces = collections.Counter()
    exporters = {}
    representatives = {}

    for model_id in sorted(runs):
        entry = runs[model_id]
        for mode in ("gate_closed", "band_enabled", "curve_probe"):
            if mode in entry:
                outcomes[entry[mode]["outcome"]] += 1
        if not all(mode in entry for mode in MODES):
            report["models"][model_id] = {"status": "incomplete"}
            continue
        if any(entry[mode]["outcome"] != "completed" for mode in MODES):
            report["models"][model_id] = {
                "status": "not_completed",
                "outcomes": {m: entry[m]["outcome"] for m in MODES},
            }
            continue

        ledgers, summaries = {}, {}
        for mode in MODES:
            ledgers[mode] = parse_ledger(read_gz(out / model_id / f"{mode}.ledger.tsv.gz"))
            summaries[mode] = parse_summary(
                read_gz(out / model_id / f"{mode}.summary.txt.gz")
            )

        closed_faces, closed_converts = ledgers["gate_closed"]
        band_faces, band_converts = ledgers["band_enabled"]

        # Cross-check: the ledger's own rendered count against the binary's.
        model_row = {"status": "completed"}
        for mode, (faces, _converts) in ledgers.items():
            rendered = sum(1 for row in faces.values() if row.get("rendered") == "1")
            declared = summaries[mode].get("declared")
            if summaries[mode].get("rendered") not in (None, rendered):
                mismatches.append(
                    f"{model_id} {mode}: ledger rendered={rendered} "
                    f"summary rendered={summaries[mode]['rendered']}"
                )
            model_row[f"{mode}_rendered"] = summaries[mode].get("rendered", rendered)
            model_row[f"{mode}_lost"] = summaries[mode].get("lost")
            model_row["declared"] = declared
        model_row["band_eligible"] = summaries["band_enabled"].get("band_eligible", 0)
        model_row["band_recovered"] = summaries["band_enabled"].get("band_recovered", 0)
        model_row["band_refused"] = summaries["band_enabled"].get("band_refused", 0)
        model_row["net_gain"] = (
            model_row["band_enabled_rendered"] - model_row["gate_closed_rendered"]
        )

        # The stop condition: any face that rendered with the gate closed and
        # does not with it open. Keyed on source_face_id, so this compares the
        # same face and not the same slot.
        for key, row in closed_faces.items():
            if key == "-" or row.get("rendered") != "1":
                continue
            other = band_faces.get(key)
            if other is not None and other.get("rendered") == "0":
                regressions.append(f"{model_id} {key}")
        model_row["regressions"] = sum(1 for r in regressions if r.startswith(model_id))

        # Band verdicts, from the per-face ledger.
        unsupported, joins = [], []
        for key, row in band_faces.items():
            band = row.get("band", "not_eligible")
            if band == "not_eligible":
                continue
            if band.startswith("recovered:"):
                conformance[band.split(":", 1)[1].split(":", 1)[-1]] += 1
                continue
            exits[band] += 1
            exits_by_model[model_id][band] += 1
            representatives.setdefault(band, f"{model_id}:{key}")
            if band == "unsupported_curve_representation":
                unsupported.append(key)
            elif band == "lift_join_no_compatible_integer":
                joins.append(key)
        model_row["unsupported_faces"] = len(unsupported)
        model_row["join_faces"] = len(joins)
        unsupported_faces_by_model[model_id] = len(unsupported)
        join_faces_by_model[model_id] = len(joins)

        totals["declared"] += model_row.get("declared") or 0
        for mode in MODES:
            totals[f"{mode}_rendered"] += model_row[f"{mode}_rendered"] or 0
            totals[f"{mode}_lost"] += model_row[f"{mode}_lost"] or 0
        for field in ("band_eligible", "band_recovered", "band_refused"):
            totals[field] += model_row[field]

        # The imported side: the probe's per-edge verdicts, joined by face.
        probe = collections.defaultdict(list)
        for line in read_gz(out / model_id / "curves.tsv.gz").splitlines():
            if not line.startswith("EDGE\t"):
                continue
            row = dict(kv.split("=", 1) for kv in line.split("\t")[1:] if "=" in kv)
            probe[face_key(row["source_face_id"])].append(row)

        interesting = set(unsupported) | set(joins)
        # The raw side: the entity chain the file declares. Read once per
        # model, only for the faces a band exit named.
        chains = {}
        if interesting:
            model_path = runs[model_id][MODES[0]]["path"]
            entity_index = EntityIndex(model_path)
            exporters[model_id] = entity_index.exporter_association()
            for key in interesting:
                face_id = int(key) if key.isdigit() else None
                if face_id is None:
                    continue
                chains[key] = list(face_edge_chains(entity_index, face_id))

        for key in unsupported:
            raw_types = [c["raw_edge_geometry_type"] for c in chains.get(key, [])]
            raw_face_signature[tuple(sorted(set(raw_types)))] += 1
            for chain in chains.get(key, []):
                raw_occurrence[chain["raw_edge_geometry_type"]] += 1
                wrapper_chains[tuple(chain["wrapper_chain"])] += 1
                pcurve_presence[
                    "pcurve_present" if chain["pcurve_ids"] else "no_pcurve"
                ] += 1
            unread = probe.get(key, [])
            causes = sorted({row["cause"] for row in unread})
            cause_face_signature[tuple(causes)] += 1
            # The class the recommendation turns on: every unread use on this
            # face is a conic the *exact* Gram predicate refused and the ULP
            # classifier still places within its certified-equal bound. Such a
            # face has no other reason to be unreadable. This is a statement
            # about readability of the bounds, not a promise of recovery — the
            # band certificate is a separate obligation and this census does
            # not discharge it.
            if unread and all(row["shadow"] == "shadow_circle" for row in unread):
                near_circle_faces[model_id] += 1
            elif unread and all(
                row["cause"] in ("b_spline_curve", "rational_b_spline_curve")
                for row in unread
            ):
                spline_only_faces[model_id] += 1
            else:
                mixed_or_true_conic_faces[model_id] += 1
            for row in unread:
                cause_occurrence[(row["imported"], row["cause"], row["shadow"])] += 1
                if row.get("circularity_ulps", "-") != "-":
                    value = float(row["circularity_ulps"])
                    bucket = (
                        "<=64eps (within certified-equal bound)"
                        if value <= 64
                        else "64..64e6 eps"
                        if value <= 64e6
                        else ">64e6 eps (certified non-circular)"
                    )
                    ulps_buckets[bucket] += 1

        for key in joins:
            raw_types = tuple(
                sorted(collections.Counter(
                    c["raw_edge_geometry_type"] for c in chains.get(key, [])
                ).items())
            )
            bounds = len({c["bound_index"] for c in chains.get(key, [])})
            bound_types = tuple(sorted({c["bound_type"] for c in chains.get(key, [])}))
            join_signatures[(bounds, bound_types, raw_types)] += 1

        report["models"][model_id] = model_row

    # ---------------------------------------------------------------- output
    print(f"look={report['revisions']['look']} truck={report['revisions']['truck']} "
          f"Cargo.lock={report['revisions']['cargo_lock']}")
    print(f"\nrun outcomes: {dict(outcomes)}")
    completed = [m for m, r in report["models"].items() if r.get("status") == "completed"]
    print(f"models discovered={len(report['models'])} completed={len(completed)}")

    if regressions:
        print(f"\n!! STOP: {len(regressions)} rendered->lost transitions")
        for item in regressions[:20]:
            print(f"   {item}")
    else:
        print("\nno rendered->lost transition on any face in any model")
    if mismatches:
        print(f"\n!! ledger/summary mismatches: {len(mismatches)}")
        for item in mismatches[:10]:
            print(f"   {item}")

    print(f"\n{'':12} {'declared':>9} {'closed_rnd':>11} {'band_rnd':>9} "
          f"{'gain':>6} {'elig':>6} {'recov':>6} {'refus':>6}")
    for model_id in sorted(completed):
        row = report["models"][model_id]
        print(f"  {model_id:10} {row['declared']:>9} {row['gate_closed_rendered']:>11} "
              f"{row['band_enabled_rendered']:>9} {row['net_gain']:>6} "
              f"{row['band_eligible']:>6} {row['band_recovered']:>6} {row['band_refused']:>6}")
    print(f"  {'TOTAL':10} {totals['declared']:>9} {totals['gate_closed_rendered']:>11} "
          f"{totals['band_enabled_rendered']:>9} "
          f"{totals['band_enabled_rendered'] - totals['gate_closed_rendered']:>6} "
          f"{totals['band_eligible']:>6} {totals['band_recovered']:>6} "
          f"{totals['band_refused']:>6}")

    print("\nrecovery tags (face level)")
    for tag, count in conformance.most_common():
        print(f"  {tag:52} {count:7}")

    print("\nBandExit histogram (face level)")
    for tag, count in exits.most_common():
        print(f"  {tag:52} {count:7}   e.g. {representatives.get(tag, '-')}")

    print("\nUnsupportedCurveRepresentation — raw STEP edge_geometry (occurrence level)")
    for tag, count in raw_occurrence.most_common():
        print(f"  {tag:52} {count:7}")
    print("\n  wrapper chains")
    for chain, count in wrapper_chains.most_common():
        print(f"  {str(chain or '(none - bare basis entity)'):52} {count:7}")
    print("\n  p-curve availability (occurrence level)")
    for tag, count in pcurve_presence.most_common():
        print(f"  {tag:52} {count:7}")
    print("\n  per-face raw signature (face level)")
    for signature, count in raw_face_signature.most_common():
        print(f"  {count:7}  {signature}")
    print("\n  imported variant / reader cause / shadow verdict (occurrence level)")
    for (imported, cause, shadow), count in cause_occurrence.most_common():
        print(f"  {imported:16} {cause:32} {shadow:32} {count:7}")
    print("\n  per-face reader-cause signature (face level)")
    for signature, count in cause_face_signature.most_common():
        print(f"  {count:7}  {signature}")
    print("\n  circularity discrepancy of refused conics (occurrence level)")
    for bucket, count in ulps_buckets.most_common():
        print(f"  {bucket:52} {count:7}")

    print("\nJoinNoCompatibleInteger — raw source signatures (face level)")
    for (bounds, bound_types, raw_types), count in join_signatures.most_common():
        print(f"  {count:5}  bounds={bounds} {bound_types} {raw_types}")

    print("\ntop models by unsupported_curve_representation faces")
    for model_id, count in unsupported_faces_by_model.most_common(10):
        if count:
            association = exporters.get(model_id, {})
        print(f"  {model_id:10} {count:6}   originating_system="
              f"{association.get('originating_system', '-')!r} "
              f"schema={association.get('schema', '-')}")
    print("\nmodels with JoinNoCompatibleInteger faces")
    for model_id, count in join_faces_by_model.most_common(10):
        if count:
            print(f"  {model_id:10} {count:6}")

    report["totals"] = dict(totals)
    report["exits"] = dict(exits)
    report["conformance"] = dict(conformance)
    report["raw_occurrence"] = dict(raw_occurrence)
    report["cause_occurrence"] = {str(k): v for k, v in cause_occurrence.items()}
    report["ulps_buckets"] = dict(ulps_buckets)
    report["join_signatures"] = {str(k): v for k, v in join_signatures.items()}
    report["near_circle_faces"] = dict(near_circle_faces)
    report["spline_only_faces"] = dict(spline_only_faces)
    report["mixed_or_true_conic_faces"] = dict(mixed_or_true_conic_faces)
    report["regressions"] = regressions
    report["mismatches"] = mismatches
    report["exporters"] = exporters
    if args.json:
        Path(args.json).write_text(json.dumps(report, indent=1))
        print(f"\nmachine-readable report: {args.json}")
    return 1 if regressions else 0


if __name__ == "__main__":
    sys.exit(main())
