//! The corpus row manifest (`corpus/ttc/MANIFEST.json`).

use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The row stage: a row is either runnable at the current program stage
/// (`canonical`) or carried on the staged-skip list with a machine-checked
/// reason (`skipped`, see the compat `skips` module).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Stage {
    /// Runnable at the current program stage: the runner executes it.
    Canonical,
    /// Not runnable yet; `corpus/ttc/SKIPS.json` carries the typed reason.
    Skipped,
}

/// One corpus row: a geometry entry of a vendored model tree.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManifestRow {
    /// Stable row id (`<family>/<part>`, e.g. `falcon_heavy/nozzle_assembly`).
    pub id: String,
    /// The upstream model family (`f1` or `falcon_heavy`).
    pub family: String,
    /// The vendored tree's `src` directory, relative to `corpus/ttc`.
    pub tree: String,
    /// The corpus module holding the entry (e.g. `lib.merlin_common`).
    pub module: String,
    /// The geometry entry function to call (e.g. `make_nozzle_assembly`).
    pub entry: String,
    /// Positional JSON arguments the entry is called with (door-serialized).
    pub args: Vec<Value>,
    /// The row stage.
    pub stage: Stage,
    /// The recorded-reference file (basename under `corpus/ttc/reference/`),
    /// present for every canonical row.
    pub reference: Option<String>,
}

/// The manifest document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManifestFile {
    /// Schema tag (`ttc_manifest.v1`).
    pub schema: String,
    /// The upstream corpus repo.
    pub corpus: String,
    /// The vendored upstream commit sha.
    pub commit: String,
    /// Path of the provenance doc, relative to the worktree root.
    pub provenance: String,
    /// Path of the compat-surface doc, relative to the worktree root.
    pub compat_surface: String,
    /// Free-form enrollment note (progress state, not a gate).
    pub note: String,
    /// The corpus rows, in stable order.
    pub rows: Vec<ManifestRow>,
}

/// Absolute path of the manifest within this worktree.
pub fn manifest_path(corpus_dir: &Path) -> std::path::PathBuf {
    corpus_dir.join("MANIFEST.json")
}

/// Loads and validates the manifest.
pub fn load_manifest(corpus_dir: &Path) -> Result<ManifestFile, String> {
    let path = manifest_path(corpus_dir);
    let text = std::fs::read_to_string(&path)
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let manifest: ManifestFile =
        serde_json::from_str(&text).map_err(|e| format!("invalid {}: {e}", path.display()))?;
    validate(&manifest).map_err(|e| format!("invalid {}: {e}", path.display()))?;
    Ok(manifest)
}

/// Structural validation that holds for any manifest:
///
/// * the schema tag is `ttc_manifest.v1`;
/// * row ids are unique;
/// * every canonical row names a reference file;
/// * every skipped row names none.
pub fn validate(manifest: &ManifestFile) -> Result<(), String> {
    if manifest.schema != "ttc_manifest.v1" {
        return Err(format!("unexpected manifest schema {}", manifest.schema));
    }
    let mut seen = std::collections::HashSet::new();
    for row in &manifest.rows {
        if !seen.insert(row.id.as_str()) {
            return Err(format!("duplicate row id {}", row.id));
        }
        match row.stage {
            Stage::Canonical if row.reference.is_none() => {
                return Err(format!("canonical row {} has no reference file", row.id));
            }
            Stage::Skipped if row.reference.is_some() => {
                return Err(format!("skipped row {} must not carry a reference", row.id));
            }
            _ => {}
        }
    }
    Ok(())
}

/// The canonical rows of the manifest, in manifest order.
pub fn canonical_rows(manifest: &ManifestFile) -> Vec<&ManifestRow> {
    manifest
        .rows
        .iter()
        .filter(|row| row.stage == Stage::Canonical)
        .collect()
}

/// The skipped rows of the manifest, in manifest order.
pub fn skipped_rows(manifest: &ManifestFile) -> Vec<&ManifestRow> {
    manifest
        .rows
        .iter()
        .filter(|row| row.stage == Stage::Skipped)
        .collect()
}
