//! The machine check of `docs/PY_BRIDGE_COMPAT_SURFACE.md` against spec §8's
//! seven compat-surface rows. Every row the corpus exercises (spec §8's
//! measured table) must appear in the doc with a landed-status token.

use std::path::PathBuf;

/// The seven surface rows of spec §8's measured table, in table order.
pub const SURFACE_ROWS: [(&str, &str); 7] = [
    ("S1", "Algebra operators + - &"),
    (
        "S2",
        "Plane/Location algebra (plane * shape, plane.offset(d), Pos, Rotation, Location, Axis)",
    ),
    ("S3", "Primitives (Box, Cylinder, Sphere, Torus, Compound)"),
    (
        "S4",
        "make_face / topology types (Edge, Face, Wire, Solid, Shape)",
    ),
    ("S5", "Spline(*pts, periodic=...) sections -> loft"),
    (
        "S6",
        "loft / revolve / extrude / sweep / fillet / chamfer / mirror",
    ),
    (
        "S7",
        "Selectors (.faces(), .edges(), .filter_by(), .take())",
    ),
];

/// The status-token vocabulary a doc row may carry.
pub const STATUS_TOKENS: [&str; 7] = [
    "landed",
    "recorded-client-layer",
    "deferred-bie",
    "boundary-refusal",
    "staged-skip",
    "lift-evidence-recorded",
    "census-recorded",
];

/// The absolute path of the compat-surface doc in this worktree.
pub fn compat_surface_doc_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../docs/PY_BRIDGE_COMPAT_SURFACE.md")
}

/// The result of the doc check.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SurfaceDocCheck {
    /// Rows found as a table row carrying a status token.
    pub rows_ok: usize,
    /// Surface ids with no table row in the doc.
    pub rows_missing: Vec<&'static str>,
    /// Surface ids whose table row carries no status token.
    pub rows_without_status: Vec<&'static str>,
}

/// Checks the doc text: every surface row must have a `| <id> |` table row
/// that contains at least one status token.
pub fn check_doc(doc: &str) -> SurfaceDocCheck {
    let mut out = SurfaceDocCheck::default();
    for (id, _description) in SURFACE_ROWS {
        let marker = format!("| {id} |");
        let row_lines: Vec<&str> = doc
            .lines()
            .filter(|line| line.contains(marker.as_str()))
            .collect();
        if row_lines.is_empty() {
            out.rows_missing.push(id);
            continue;
        }
        let has_status = row_lines
            .iter()
            .any(|line| STATUS_TOKENS.iter().any(|token| line.contains(token)));
        if has_status {
            out.rows_ok += 1;
        } else {
            out.rows_without_status.push(id);
        }
    }
    out
}

/// Loads the doc and runs [`check_doc`].
pub fn load_and_check_doc() -> Result<SurfaceDocCheck, String> {
    let path = compat_surface_doc_path();
    let text = std::fs::read_to_string(&path)
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    Ok(check_doc(&text))
}
