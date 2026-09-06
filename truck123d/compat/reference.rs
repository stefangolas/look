//! The recorded OCC-derived reference facts (`corpus/ttc/reference/*.json`)
//! and the tolerance comparison a canonical run is graded against.

use std::path::Path;

use serde::{Deserialize, Serialize};

use super::manifest::ManifestRow;

/// The geometry facts a canonical run must reproduce: solid count, volume and
/// the axis-aligned bounding box.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GeometryFacts {
    /// Number of solids in the produced shape.
    pub solid_count: i64,
    /// Reported volume of the produced shape.
    pub volume: f64,
    /// Axis-aligned bounding box `[min, max]`.
    pub bbox: [[f64; 3]; 2],
}

/// The recorded reference for one canonical row.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReferenceFile {
    /// Schema tag (`ttc_reference.v1`).
    pub schema: String,
    /// The manifest row id this reference belongs to.
    pub row_id: String,
    /// The door version that recorded it.
    pub door_version: String,
    /// The recorded OCC-derived facts.
    pub facts: GeometryFacts,
    /// The comparison tolerances the recorded facts are graded within.
    pub tolerances: Tolerances,
}

/// The comparison tolerances (the door records exact ints; floats compare
/// relative/absolute so OCC's deterministic last-bit noise cannot flake).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tolerances {
    /// `solid_count` comparison: always exact.
    pub solid_count: String,
    /// Relative volume tolerance for `volume != 0` references.
    pub volume_rel: f64,
    /// Absolute volume tolerance for near-zero volumes.
    pub volume_abs: f64,
    /// Absolute per-coordinate bounding-box tolerance.
    pub bbox_abs: f64,
}

/// Absolute path of the reference file for a canonical row.
pub fn reference_path(corpus_dir: &Path, row: &ManifestRow) -> Option<std::path::PathBuf> {
    let name = row.reference.as_ref()?;
    Some(corpus_dir.join("reference").join(name))
}

/// Loads the reference file for a canonical row.
pub fn load_reference(corpus_dir: &Path, row: &ManifestRow) -> Result<ReferenceFile, String> {
    let path = reference_path(corpus_dir, row)
        .ok_or_else(|| format!("row {} has no reference file", row.id))?;
    let text = std::fs::read_to_string(&path)
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let reference: ReferenceFile =
        serde_json::from_str(&text).map_err(|e| format!("invalid {}: {e}", path.display()))?;
    if reference.schema != "ttc_reference.v1" {
        return Err(format!(
            "reference {} has unexpected schema {}",
            path.display(),
            reference.schema
        ));
    }
    if reference.row_id != row.id {
        return Err(format!(
            "reference {} is for row {} not {}",
            path.display(),
            reference.row_id,
            row.id
        ));
    }
    Ok(reference)
}

/// Compares the freshly produced facts against the recorded reference within
/// the recorded tolerances. `Ok(())` means the facts match; `Err` names the
/// first mismatch with both values.
pub fn compare_facts(facts: &GeometryFacts, reference: &ReferenceFile) -> Result<(), String> {
    let tol = &reference.tolerances;
    if tol.solid_count != "exact" {
        return Err(format!(
            "unexpected solid_count tolerance {}",
            tol.solid_count
        ));
    }
    let expected = &reference.facts;
    if facts.solid_count != expected.solid_count {
        return Err(format!(
            "solid count {} != recorded {}",
            facts.solid_count, expected.solid_count
        ));
    }
    let volume_ok = if expected.volume == 0.0 {
        (facts.volume - expected.volume).abs() <= tol.volume_abs
    } else {
        ((facts.volume - expected.volume) / expected.volume).abs() <= tol.volume_rel
    };
    if !volume_ok {
        return Err(format!(
            "volume {} differs from recorded {} beyond tolerance",
            facts.volume, expected.volume
        ));
    }
    for axis in 0..3 {
        for corner in 0..2 {
            let diff = (facts.bbox[corner][axis] - expected.bbox[corner][axis]).abs();
            if diff > tol.bbox_abs {
                return Err(format!(
                    "bbox corner {corner} axis {axis} = {} differs from recorded {} beyond tolerance",
                    facts.bbox[corner][axis], expected.bbox[corner][axis]
                ));
            }
        }
    }
    Ok(())
}
