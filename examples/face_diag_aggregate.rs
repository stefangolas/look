//! DIAG-001 offline aggregator: reads the JSONL output from `face_census`
//! and emits summary files for opportunity analysis.
//!
//! Usage:
//!   face_diag_aggregate <diag.jsonl> [--meta <meta.json>] [--outdir <dir>]
//!
//! Produces:
//!   summary.json              — full summary with reconciliation
//!   summary.csv               — flat bucket histogram
//!   bucket_by_rank.csv        — bucket × chart-rank
//!   bucket_by_surface.csv     — bucket × surface-family
//!   representative_faces.json — up to 25 deterministic examples per bucket

use std::collections::BTreeMap;
use std::io::BufRead;

use serde::Serialize;
use serde_json::Value;

#[derive(Serialize)]
struct Summary {
    declared: usize,
    successful: usize,
    failed: usize,
    rows_read: usize,
    terminal_reason_histogram: BTreeMap<String, usize>,
    derived_bucket_histogram: BTreeMap<String, usize>,
    insertion_failures_zero_witnesses: usize,
    insertion_failures_one_witness: usize,
    insertion_failures_multiple_witnesses: usize,
    conflict_origin_pair_histogram: BTreeMap<String, usize>,
    same_bound_vs_inter_bound: BTreeMap<String, usize>,
    surface_family_by_bucket: BTreeMap<String, BTreeMap<String, usize>>,
    chart_rank_by_bucket: BTreeMap<String, BTreeMap<String, usize>>,
    periodic_axis_by_bucket: BTreeMap<String, BTreeMap<String, usize>>,
    synthetic_segment_presence_by_bucket: BTreeMap<String, usize>,
    reconciliation: Reconciliation,
    opportunity: Opportunity,
}

#[derive(Serialize)]
struct Reconciliation {
    successful_plus_failed_equals_declared: bool,
    sum_terminal_reasons_equals_failed: bool,
    sum_derived_buckets_equals_failed: bool,
}

#[derive(Serialize)]
struct Opportunity {
    arr003_rank0_source_source_proper_crossings: usize,
    arr003_periodic_source_source_proper_crossings: usize,
    source_source_faces_one_conflict: usize,
    source_source_faces_multiple_conflicts: usize,
    periodic_rank_above_zero: usize,
    ambiguous_lift: usize,
    source_synthetic_conflicts: usize,
    synthetic_synthetic_conflicts: usize,
    faces_with_synthetic_segments: usize,
    projection_failures: usize,
    overlap_failures: usize,
    parity_contradiction: usize,
    no_material_region: usize,
    insertion_unknown: usize,
}

fn str_field(row: &Value, key: &str) -> String {
    row.get(key)
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown")
        .to_string()
}

fn u64_field(row: &Value, key: &str) -> u64 {
    row.get(key).and_then(|v| v.as_u64()).unwrap_or(0)
}

fn origin_class(origin: &str) -> &'static str {
    match origin {
        "Source" => "source",
        "SyntheticClosure" => "synthetic",
        "Seam" => "synthetic",
        _ => "unknown",
    }
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        eprintln!("usage: face_diag_aggregate <diag.jsonl> [--meta <meta.json>] [--outdir <dir>]");
        return Ok(());
    }
    let jsonl_path = &args[0];
    let meta_path = args
        .iter()
        .position(|a| a == "--meta")
        .and_then(|i| args.get(i + 1))
        .map(|s| s.as_str())
        .unwrap_or("diag.jsonl.meta.json");
    let outdir = args
        .iter()
        .position(|a| a == "--outdir")
        .and_then(|i| args.get(i + 1))
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::path::Path::new(jsonl_path)
                .parent()
                .unwrap_or(std::path::Path::new("."))
                .to_path_buf()
        });

    // Read meta for declared/successful counts.
    let (declared, successful) = match std::fs::read_to_string(meta_path) {
        Ok(text) => {
            let meta: Value = serde_json::from_str(&text)?;
            let d = meta.get("declared").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
            let s = meta.get("rendered").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
            (d, s)
        }
        Err(_) => (0, 0),
    };

    // Read JSONL rows.
    let file = std::fs::File::open(jsonl_path)?;
    let rows: Vec<Value> = std::io::BufReader::new(file)
        .lines()
        .map_while(Result::ok)
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str(&l).ok())
        .collect();
    let failed = rows.len();

    // Histograms.
    let mut terminal_reason_histogram: BTreeMap<String, usize> = BTreeMap::new();
    let mut derived_bucket_histogram: BTreeMap<String, usize> = BTreeMap::new();
    let mut insertion_zero = 0usize;
    let mut insertion_one = 0usize;
    let mut insertion_multiple = 0usize;
    let mut conflict_origin_pair_histogram: BTreeMap<String, usize> = BTreeMap::new();
    let mut same_bound_vs_inter_bound: BTreeMap<String, usize> = BTreeMap::new();
    let mut surface_family_by_bucket: BTreeMap<String, BTreeMap<String, usize>> = BTreeMap::new();
    let mut chart_rank_by_bucket: BTreeMap<String, BTreeMap<String, usize>> = BTreeMap::new();
    let mut periodic_axis_by_bucket: BTreeMap<String, BTreeMap<String, usize>> = BTreeMap::new();
    let mut synthetic_presence_by_bucket: BTreeMap<String, usize> = BTreeMap::new();

    // Representative faces: up to 25 per bucket.
    let mut representatives: BTreeMap<String, Vec<(String, Option<u64>)>> = BTreeMap::new();

    // Opportunity counters.
    let mut arr003_rank0_ss = 0usize;
    let mut arr003_periodic_ss = 0usize;
    let mut ss_faces_one = 0usize;
    let mut ss_faces_multiple = 0usize;
    let mut periodic_rank_above_zero = 0usize;
    let mut ambiguous_lift = 0usize;
    let mut source_synthetic_conflicts = 0usize;
    let mut synthetic_synthetic_conflicts = 0usize;
    let mut faces_with_synthetic_segments = 0usize;
    let mut projection_failures = 0usize;
    let mut overlap_failures = 0usize;
    let mut parity_contradiction = 0usize;
    let mut no_material_region = 0usize;
    let mut insertion_unknown = 0usize;

    for row in &rows {
        let reason = str_field(row, "terminal_reason");
        let bucket = str_field(row, "derived_bucket");
        let chart_rank = u64_field(row, "chart_rank") as u8;
        let source_face_id = row.get("source_face_id").and_then(|v| v.as_u64());
        let model_id = str_field(row, "model_id");

        *terminal_reason_histogram.entry(reason.clone()).or_default() += 1;
        *derived_bucket_histogram.entry(bucket.clone()).or_default() += 1;

        // Surface family by bucket.
        let sf = str_field(row, "surface_family");
        *surface_family_by_bucket
            .entry(bucket.clone())
            .or_default()
            .entry(sf)
            .or_default() += 1;

        // Chart rank by bucket.
        *chart_rank_by_bucket
            .entry(bucket.clone())
            .or_default()
            .entry(chart_rank.to_string())
            .or_default() += 1;

        // Periodic axis by bucket.
        let pa = row.get("periodic_axes").unwrap_or(&Value::Null);
        let pa_u = pa.get("u").and_then(|v| v.as_bool()).unwrap_or(false);
        let pa_v = pa.get("v").and_then(|v| v.as_bool()).unwrap_or(false);
        let pa_label = match (pa_u, pa_v) {
            (false, false) => "none",
            (true, false) => "u",
            (false, true) => "v",
            (true, true) => "uv",
        };
        *periodic_axis_by_bucket
            .entry(bucket.clone())
            .or_default()
            .entry(pa_label.to_string())
            .or_default() += 1;

        // Synthetic segment presence.
        let synth_count = u64_field(row, "synthetic_segment_count") as usize;
        if synth_count > 0 {
            *synthetic_presence_by_bucket
                .entry(bucket.clone())
                .or_default() += 1;
            faces_with_synthetic_segments += 1;
        }

        // Representative faces (stable sort by model_id, source_face_id).
        let rep_key = (model_id.clone(), source_face_id);
        let reps = representatives.entry(bucket.clone()).or_default();
        if reps.len() < 25 {
            reps.push((rep_key.0, rep_key.1));
            reps.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
        }

        // Insertion witness analysis.
        let conflicts = row.get("insertion_conflicts").and_then(|v| v.as_array());
        let conflict_count = conflicts.map(|c| c.len()).unwrap_or(0);
        if reason == "ConstraintInsertionIncomplete" {
            match conflict_count {
                0 => insertion_zero += 1,
                1 => insertion_one += 1,
                _ => insertion_multiple += 1,
            }
        }

        // Conflict origin-pair and same-bound analysis.
        if let Some(conflicts) = conflicts {
            for c in conflicts {
                let incoming_origin = c
                    .get("incoming")
                    .and_then(|v| v.get("origin"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown");
                let blocking_origin = c
                    .get("blocking")
                    .and_then(|v| v.get("origin"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown");
                let pair = format!(
                    "{}/{}",
                    origin_class(incoming_origin),
                    origin_class(blocking_origin)
                );
                *conflict_origin_pair_histogram.entry(pair).or_default() += 1;

                let same_bound = c.get("same_bound").and_then(|v| v.as_bool());
                let bound_label = match same_bound {
                    Some(true) => "same_bound",
                    Some(false) => "inter_bound",
                    None => "unknown_bound",
                };
                *same_bound_vs_inter_bound
                    .entry(bound_label.to_string())
                    .or_default() += 1;

                // Opportunity: source/synthetic and synthetic/synthetic.
                let inc = origin_class(incoming_origin);
                let blk = origin_class(blocking_origin);
                if (inc == "source" && blk == "synthetic")
                    || (inc == "synthetic" && blk == "source")
                {
                    source_synthetic_conflicts += 1;
                }
                if inc == "synthetic" && blk == "synthetic" {
                    synthetic_synthetic_conflicts += 1;
                }
            }
        }

        // Opportunity analysis.
        if chart_rank > 0 {
            periodic_rank_above_zero += 1;
        }
        if reason == "AmbiguousLift" {
            ambiguous_lift += 1;
        }
        if reason == "BoundaryProjectionFailed" || reason == "BoundaryPointOffSurface" {
            projection_failures += 1;
        }
        if reason == "ConstraintOverlapUnsupported" {
            overlap_failures += 1;
        }
        if reason == "ContradictoryDualParity" {
            parity_contradiction += 1;
        }
        if reason == "NoOddParityRegion" {
            no_material_region += 1;
        }
        if bucket == "InsertionUnknown" {
            insertion_unknown += 1;
        }

        // ARR-003 opportunity: source/source proper crossings.
        if bucket == "SourceSourceSameBoundCrossing" || bucket == "SourceSourceInterBoundCrossing" {
            if chart_rank == 0 {
                arr003_rank0_ss += 1;
            } else {
                arr003_periodic_ss += 1;
            }
            match conflict_count {
                1 => ss_faces_one += 1,
                _ if conflict_count > 1 => ss_faces_multiple += 1,
                _ => {}
            }
        }
    }

    let sum_terminal: usize = terminal_reason_histogram.values().sum();
    let sum_buckets: usize = derived_bucket_histogram.values().sum();

    let summary = Summary {
        declared,
        successful,
        failed,
        rows_read: failed,
        terminal_reason_histogram,
        derived_bucket_histogram,
        insertion_failures_zero_witnesses: insertion_zero,
        insertion_failures_one_witness: insertion_one,
        insertion_failures_multiple_witnesses: insertion_multiple,
        conflict_origin_pair_histogram,
        same_bound_vs_inter_bound,
        surface_family_by_bucket,
        chart_rank_by_bucket,
        periodic_axis_by_bucket,
        synthetic_segment_presence_by_bucket: synthetic_presence_by_bucket,
        reconciliation: Reconciliation {
            successful_plus_failed_equals_declared: successful + failed == declared,
            sum_terminal_reasons_equals_failed: sum_terminal == failed,
            sum_derived_buckets_equals_failed: sum_buckets == failed,
        },
        opportunity: Opportunity {
            arr003_rank0_source_source_proper_crossings: arr003_rank0_ss,
            arr003_periodic_source_source_proper_crossings: arr003_periodic_ss,
            source_source_faces_one_conflict: ss_faces_one,
            source_source_faces_multiple_conflicts: ss_faces_multiple,
            periodic_rank_above_zero,
            ambiguous_lift,
            source_synthetic_conflicts,
            synthetic_synthetic_conflicts,
            faces_with_synthetic_segments,
            projection_failures,
            overlap_failures,
            parity_contradiction,
            no_material_region,
            insertion_unknown,
        },
    };

    // Write outputs.
    std::fs::create_dir_all(&outdir)?;

    let summary_path = outdir.join("summary.json");
    std::fs::write(&summary_path, serde_json::to_string_pretty(&summary)?)?;
    eprintln!("wrote {}", summary_path.display());

    // summary.csv — flat bucket histogram.
    let summary_csv = outdir.join("summary.csv");
    let mut csv = String::from("bucket,count\n");
    for (bucket, count) in &summary.derived_bucket_histogram {
        csv.push_str(&format!("{bucket},{count}\n"));
    }
    std::fs::write(&summary_csv, csv)?;
    eprintln!("wrote {}", summary_csv.display());

    // bucket_by_rank.csv.
    let rank_csv = outdir.join("bucket_by_rank.csv");
    let mut csv = String::from("bucket,rank,count\n");
    for (bucket, ranks) in &summary.chart_rank_by_bucket {
        for (rank, count) in ranks {
            csv.push_str(&format!("{bucket},{rank},{count}\n"));
        }
    }
    std::fs::write(&rank_csv, csv)?;
    eprintln!("wrote {}", rank_csv.display());

    // bucket_by_surface.csv.
    let surf_csv = outdir.join("bucket_by_surface.csv");
    let mut csv = String::from("bucket,surface_family,count\n");
    for (bucket, families) in &summary.surface_family_by_bucket {
        for (family, count) in families {
            csv.push_str(&format!("{bucket},{family},{count}\n"));
        }
    }
    std::fs::write(&surf_csv, csv)?;
    eprintln!("wrote {}", surf_csv.display());

    // representative_faces.json.
    let rep_path = outdir.join("representative_faces.json");
    let rep_json: BTreeMap<String, Vec<serde_json::Value>> = representatives
        .iter()
        .map(|(bucket, faces)| {
            let entries: Vec<serde_json::Value> = faces
                .iter()
                .take(25)
                .map(|(model, id)| {
                    serde_json::json!({
                        "model_id": model,
                        "source_face_id": id,
                    })
                })
                .collect();
            (bucket.clone(), entries)
        })
        .collect();
    std::fs::write(&rep_path, serde_json::to_string_pretty(&rep_json)?)?;
    eprintln!("wrote {}", rep_path.display());

    // Print reconciliation to stderr.
    eprintln!("\n=== Reconciliation ===");
    eprintln!(
        "declared={declared} successful={successful} failed={failed} rows_read={}",
        summary.rows_read
    );
    eprintln!(
        "  successful + failed = declared: {} ({successful} + {failed} = {} vs {declared})",
        summary
            .reconciliation
            .successful_plus_failed_equals_declared,
        successful + failed,
    );
    eprintln!(
        "  sum(terminal_reasons) = failed: {} ({sum_terminal} vs {failed})",
        summary.reconciliation.sum_terminal_reasons_equals_failed,
    );
    eprintln!(
        "  sum(derived_buckets) = failed: {} ({sum_buckets} vs {failed})",
        summary.reconciliation.sum_derived_buckets_equals_failed,
    );

    eprintln!("\n=== Derived bucket histogram ===");
    for (bucket, count) in &summary.derived_bucket_histogram {
        eprintln!("  {bucket:45} {count:>6}");
    }

    eprintln!("\n=== Opportunity ===");
    eprintln!("  ARR-003 rank-0 source/source crossings:   {arr003_rank0_ss}");
    eprintln!("  ARR-003 periodic source/source crossings:  {arr003_periodic_ss}");
    eprintln!("  source/source faces with 1 conflict:       {ss_faces_one}");
    eprintln!("  source/source faces with >1 conflicts:     {ss_faces_multiple}");
    eprintln!("  periodic rank > 0:                         {periodic_rank_above_zero}");
    eprintln!("  ambiguous lift:                            {ambiguous_lift}");
    eprintln!("  source/synthetic conflicts:                {source_synthetic_conflicts}");
    eprintln!("  synthetic/synthetic conflicts:             {synthetic_synthetic_conflicts}");
    eprintln!("  faces with synthetic segments:             {faces_with_synthetic_segments}");
    eprintln!("  projection failures:                       {projection_failures}");
    eprintln!("  overlap failures:                          {overlap_failures}");
    eprintln!("  parity contradiction:                      {parity_contradiction}");
    eprintln!("  no material region:                        {no_material_region}");
    eprintln!("  insertion unknown:                         {insertion_unknown}");

    Ok(())
}
