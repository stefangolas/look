//! The corpus runner: executes each canonical row end to end through the
//! process-per-script door (`corpus/ttc/door.py`, OCC baseline regime),
//! producing an STL file and a report JSON per row.
//!
//! The runner is a plain door (scope decision 6): one fresh python process
//! per canonical row, never a reimplementation of cadgen's daemon/store. The
//! door's stdout JSON record is parsed into [`RowOutcome`], a report JSON is
//! written beside the produced STL, and the geometry facts are compared
//! against the recorded reference (see [`compare_against_reference`]).

use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};

use super::manifest::{ManifestFile, ManifestRow};
use super::reference::{self, GeometryFacts, ReferenceFile};

/// One parsed geometry fact of a door run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunFacts {
    /// The produced solid count.
    pub solid_count: i64,
    /// The produced volume.
    pub volume: f64,
    /// The produced bounding box `[min, max]`.
    pub bbox: [[f64; 3]; 2],
}

/// The door's STL outcome.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunStl {
    /// The STL file name.
    pub path: String,
    /// The number of triangles in the produced STL.
    pub triangles: i64,
}

/// The per-row report the runner writes (`schema: ttc_report.v1`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReportFile {
    /// Schema tag.
    pub schema: String,
    /// The manifest row id.
    pub row_id: String,
    /// Whether the row ran end to end.
    pub ok: bool,
    /// The door version that produced the run.
    pub door_version: String,
    /// The geometry facts, when the run succeeded.
    pub facts: Option<RunFacts>,
    /// The STL outcome, when the run succeeded.
    pub stl: Option<RunStl>,
    /// A typed error summary when the run failed.
    pub error: Option<String>,
}

/// The result of running one canonical row through the door.
#[derive(Debug, Clone, PartialEq)]
pub struct RowOutcome {
    /// The manifest row id.
    pub row_id: String,
    /// Whether the run produced facts + STL.
    pub ok: bool,
    /// The produced geometry facts.
    pub facts: Option<RunFacts>,
    /// The produced STL file path (absolute).
    pub stl_path: Option<PathBuf>,
    /// The produced report JSON path (absolute).
    pub report_path: Option<PathBuf>,
    /// A typed error summary on failure.
    pub error: Option<String>,
}

/// The fixed report schema tag.
pub const REPORT_SCHEMA: &str = "ttc_report.v1";

/// Runs every canonical row of the manifest through the door, in manifest
/// order, writing each row's STL and report JSON into `out_dir`.
pub fn run_canonical_rows(
    corpus_dir: &Path,
    manifest: &ManifestFile,
    out_dir: &Path,
) -> Vec<RowOutcome> {
    std::fs::create_dir_all(out_dir)
        .map_err(|e| format!("cannot create out dir {}: {e}", out_dir.display()))
        .expect("the harness out dir must be creatable");
    let door = corpus_dir.join("door.py");
    let canonical = super::manifest::canonical_rows(manifest);
    canonical
        .iter()
        .map(|row| run_one(corpus_dir, &door, row, out_dir))
        .collect()
}

/// The STL file name for a row (the row's reference basename, `{id}.stl`).
fn stl_name(row: &ManifestRow) -> String {
    format!("{}.stl", row.id.replace('/', "__"))
}

/// The report JSON file name for a row.
fn report_name(row: &ManifestRow) -> String {
    format!("{}.report.json", row.id.replace('/', "__"))
}

/// Runs one canonical row through the door and returns its outcome.
fn run_one(corpus_dir: &Path, door: &Path, row: &ManifestRow, out_dir: &Path) -> RowOutcome {
    let tree = corpus_dir.join(&row.tree);
    let stl_path = out_dir.join(stl_name(row));
    let args_json = serde_json::to_string(&row.args)
        .map_err(|e| format!("cannot serialize args for {}: {e}", row.id));
    let args_json = match args_json {
        Ok(v) => v,
        Err(e) => {
            return fail(row, out_dir, &e);
        }
    };

    let output = Command::new("python")
        .arg(door)
        .arg(&tree)
        .arg(&row.module)
        .arg(&row.entry)
        .arg(&args_json)
        .arg(&stl_path)
        .output()
        .map_err(|e| format!("cannot spawn door for {}: {e}", row.id));

    let output = match output {
        Ok(o) => o,
        Err(e) => {
            return fail(row, out_dir, &e);
        }
    };

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let record: serde_json::Value = match serde_json::from_str(&stdout) {
        Ok(v) => v,
        Err(e) => {
            return fail(
                row,
                out_dir,
                &format!(
                    "door produced no JSON record for {}: {e}\n{}",
                    row.id,
                    String::from_utf8_lossy(&output.stderr)
                ),
            );
        }
    };

    let parsed = parse_record(&record);
    match parsed {
        Ok((facts, stl, door_version)) => {
            let report = ReportFile {
                schema: REPORT_SCHEMA.to_string(),
                row_id: row.id.clone(),
                ok: true,
                door_version,
                facts: Some(facts.clone()),
                stl: Some(stl.clone()),
                error: None,
            };
            let report_path = out_dir.join(report_name(row));
            if let Err(e) = write_report(&report_path, &report) {
                return fail(row, out_dir, &e);
            }
            RowOutcome {
                row_id: row.id.clone(),
                ok: true,
                facts: Some(facts),
                stl_path: Some(stl_path),
                report_path: Some(report_path),
                error: None,
            }
        }
        Err(e) => fail(
            row,
            out_dir,
            &format!("{e}\n{}", String::from_utf8_lossy(&output.stderr)),
        ),
    }
}

/// Builds a failed outcome (no facts) and writes the failed report JSON.
fn fail(row: &ManifestRow, out_dir: &Path, error: &str) -> RowOutcome {
    let report = ReportFile {
        schema: REPORT_SCHEMA.to_string(),
        row_id: row.id.clone(),
        ok: false,
        door_version: String::new(),
        facts: None,
        stl: None,
        error: Some(error.to_string()),
    };
    let report_path = out_dir.join(report_name(row));
    let _ = write_report(&report_path, &report);
    RowOutcome {
        row_id: row.id.clone(),
        ok: false,
        facts: None,
        stl_path: None,
        report_path: Some(report_path),
        error: Some(error.to_string()),
    }
}

/// Parses a door record JSON value into `(facts, stl, door_version)`.
fn parse_record(record: &serde_json::Value) -> Result<(RunFacts, RunStl, String), String> {
    let ok = record
        .get("ok")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    if !ok {
        let error = record.get("error").cloned().unwrap_or_default();
        return Err(format!("door run failed: {error}"));
    }
    let door_version = record
        .get("door_version")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_string();
    let facts = record
        .get("facts")
        .ok_or_else(|| "door record has no facts".to_string())?;
    let solid_count = facts
        .get("solid_count")
        .and_then(serde_json::Value::as_i64)
        .ok_or_else(|| "door record facts.solid_count is not an integer".to_string())?;
    let volume = facts
        .get("volume")
        .and_then(serde_json::Value::as_f64)
        .ok_or_else(|| "door record facts.volume is not a number".to_string())?;
    let bbox = parse_bbox(
        facts
            .get("bbox")
            .ok_or_else(|| "door record facts.bbox is missing".to_string())?,
    )?;
    let stl = record
        .get("stl")
        .ok_or_else(|| "door record has no stl".to_string())?;
    let stl_path = stl
        .get("path")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_string();
    let triangles = stl
        .get("triangles")
        .and_then(serde_json::Value::as_i64)
        .ok_or_else(|| "door record stl.triangles is not an integer".to_string())?;
    Ok((
        RunFacts {
            solid_count,
            volume,
            bbox,
        },
        RunStl {
            path: stl_path,
            triangles,
        },
        door_version,
    ))
}

/// Parses a `[[min],[max]]` bounding-box value.
fn parse_bbox(value: &serde_json::Value) -> Result<[[f64; 3]; 2], String> {
    let corners = value
        .as_array()
        .ok_or_else(|| "door record facts.bbox is not an array".to_string())?;
    if corners.len() != 2 {
        return Err("door record facts.bbox must have two corners".to_string());
    }
    let mut bbox = [[0.0f64; 3]; 2];
    for (corner_idx, corner) in corners.iter().enumerate() {
        let coords = corner
            .as_array()
            .ok_or_else(|| "bbox corner is not an array".to_string())?;
        if coords.len() != 3 {
            return Err("bbox corner must have three coordinates".to_string());
        }
        for (axis, coord) in coords.iter().enumerate() {
            let value = coord
                .as_f64()
                .ok_or_else(|| "bbox coordinate is not a number".to_string())?;
            bbox[corner_idx][axis] = value;
        }
    }
    Ok(bbox)
}

/// Writes a report JSON file deterministically (serde field order is stable).
fn write_report(path: &Path, report: &ReportFile) -> Result<(), String> {
    let text = serde_json::to_string_pretty(report)
        .map_err(|e| format!("cannot serialize report {}: {e}", path.display()))?;
    std::fs::write(path, text).map_err(|e| format!("cannot write {}: {e}", path.display()))
}

/// The reference facts as [`GeometryFacts`] (shared with the reference check).
pub fn as_geometry_facts(run: &RunFacts) -> GeometryFacts {
    GeometryFacts {
        solid_count: run.solid_count,
        volume: run.volume,
        bbox: run.bbox,
    }
}

/// Loads a reference and compares a run outcome's facts against it.
pub fn compare_against_reference(
    corpus_dir: &Path,
    row: &ManifestRow,
    outcome: &RowOutcome,
) -> Result<ReferenceFile, String> {
    let reference = reference::load_reference(corpus_dir, row)?;
    let facts = outcome
        .facts
        .as_ref()
        .ok_or_else(|| format!("row {} produced no facts", row.id))?;
    reference::compare_facts(&as_geometry_facts(facts), &reference)?;
    Ok(reference)
}
