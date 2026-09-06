//! The staged-skip machinery (`corpus/ttc/SKIPS.json`).
//!
//! A skip without a reason row is a harness failure (scope decision 4): every
//! manifest row staged `skipped` must carry exactly one skip row whose reason
//! code resolves in the reason vocabulary, and a reason code must name the
//! packet/program that unblocks it (`resolved_by`) — the skip list is an
//! OUTPUT of program progress, never a threshold to tune.

use std::path::Path;

use serde::{Deserialize, Serialize};

use super::manifest::{ManifestFile, Stage};

/// One reason-vocabulary entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReasonRow {
    /// The packet/program whose landing unblocks rows carrying this reason.
    pub resolved_by: String,
    /// Human label of the reason.
    pub label: String,
    /// Why the reason applies at the current stage.
    pub note: String,
}

/// One skip row: a manifest row id plus its machine-checked reason code.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkipRow {
    /// The manifest row id this skip applies to.
    pub id: String,
    /// The reason code (a key of the `reasons` vocabulary). `None` is the
    /// reasonless-skip failure the harness exists to catch.
    pub reason: Option<String>,
    /// Evidence note: which corpus module/helper exercises the blocked form.
    pub note: String,
}

/// The skips document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkipsFile {
    /// Schema tag (`ttc_skips.v1`).
    pub schema: String,
    /// The upstream corpus repo.
    pub corpus: String,
    /// The machine-checked reason vocabulary, keyed by reason code.
    pub reasons: std::collections::HashMap<String, ReasonRow>,
    /// The skip rows, one per skipped manifest row.
    pub rows: Vec<SkipRow>,
}

/// Absolute path of the skips file within this worktree.
pub fn skips_path(corpus_dir: &Path) -> std::path::PathBuf {
    corpus_dir.join("SKIPS.json")
}

/// Loads and validates the skips file against the manifest.
pub fn load_skips(corpus_dir: &Path, manifest: &ManifestFile) -> Result<SkipsFile, String> {
    let path = skips_path(corpus_dir);
    let text = std::fs::read_to_string(&path)
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let skips: SkipsFile =
        serde_json::from_str(&text).map_err(|e| format!("invalid {}: {e}", path.display()))?;
    validate(manifest, &skips).map_err(|e| format!("invalid {}: {e}", path.display()))?;
    Ok(skips)
}

/// The outcome of validating skips against the manifest, with the census that
/// made it: counts of skipped rows, skip rows, reasonless skips and
/// unresolvable reasons. The test asserts on both the result and the census.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SkipsCensus {
    /// Manifest rows staged `skipped`.
    pub manifest_skipped: usize,
    /// Skip rows present in the skips file.
    pub skip_rows: usize,
    /// Skip rows whose reason code is missing (reasonless skips).
    pub reasonless: usize,
    /// Skip rows whose reason code is not a vocabulary key.
    pub unresolvable: usize,
    /// Skipped manifest rows with no matching skip row.
    pub missing_row: usize,
    /// Skip rows naming a manifest row that is not staged skipped.
    pub unknown_row: usize,
    /// Vocabulary entries whose `resolved_by` is empty.
    pub unresolved_reason: usize,
}

/// Validates every skip:
///
/// * the schema tag is `ttc_skips.v1`;
/// * every vocabulary reason resolves (`resolved_by` non-empty);
/// * every manifest row staged `skipped` has exactly one skip row;
/// * every skip row names a manifest row staged `skipped`;
/// * no skip row is reasonless (reason present) and every reason is a key of
///   the vocabulary.
pub fn validate(manifest: &ManifestFile, skips: &SkipsFile) -> Result<(), String> {
    let census = census(manifest, skips);
    if skips.schema != "ttc_skips.v1" {
        return Err(format!("unexpected skips schema {}", skips.schema));
    }
    if census.unresolved_reason > 0 {
        return Err("a skip reason vocabulary entry carries no resolved_by".to_string());
    }
    if census.reasonless > 0 {
        return Err("a skip without a reason row is a harness failure".to_string());
    }
    if census.unresolvable > 0 {
        return Err("a skip row names a reason code outside the vocabulary".to_string());
    }
    if census.missing_row > 0 {
        return Err("a skipped manifest row has no skip row".to_string());
    }
    if census.unknown_row > 0 {
        return Err("a skip row names a manifest row that is not staged skipped".to_string());
    }
    Ok(())
}

/// Computes the skip census (see [`SkipsCensus`]).
pub fn census(manifest: &ManifestFile, skips: &SkipsFile) -> SkipsCensus {
    let manifest_skipped = manifest
        .rows
        .iter()
        .filter(|r| r.stage == Stage::Skipped)
        .count();
    let mut by_id: std::collections::HashMap<&str, &SkipRow> = std::collections::HashMap::new();
    for row in &skips.rows {
        let _ = by_id.insert(row.id.as_str(), row);
    }
    let mut out = SkipsCensus {
        manifest_skipped,
        skip_rows: skips.rows.len(),
        ..SkipsCensus::default()
    };
    for row in &skips.rows {
        match &row.reason {
            None => out.reasonless += 1,
            Some(reason) => {
                if !skips.reasons.contains_key(reason) {
                    out.unresolvable += 1;
                }
            }
        }
        let in_manifest = manifest
            .rows
            .iter()
            .any(|m| m.id == row.id && m.stage == Stage::Skipped);
        if !in_manifest {
            out.unknown_row += 1;
        }
    }
    for reason in skips.reasons.values() {
        if reason.resolved_by.trim().is_empty() {
            out.unresolved_reason += 1;
        }
    }
    for manifest_row in manifest.rows.iter().filter(|r| r.stage == Stage::Skipped) {
        if !by_id.contains_key(manifest_row.id.as_str()) {
            out.missing_row += 1;
        }
    }
    out
}
