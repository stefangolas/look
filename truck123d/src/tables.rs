//! The v1 showcase-table schema (PB-000 §3), as serde structs.
//!
//! The three tables (`showcases/tables/{waterslide,teapot,amphora}.json`) are
//! the portable model tables the bridge consumes. Schema v1 is exact: the
//! top-level key set of each table is pinned (no missing keys, no unknown
//! keys — `deny_unknown_fields`) and the value domains of §3.2 are enforced
//! by the field types (f64 for `LEN`/`LEN+`/`ANGLE_DEG`/`FRAC`/`SCALE` rows,
//! unsigned integers for `COUNT`/`RING` rows, `[z, r]` pairs, `[x, y, z]`
//! triples). Python dict → table struct → identical dict is the serde round
//! trip this module exists for.
//!
//! H-1 applies.

#![cfg_attr(
    not(test),
    deny(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::todo,
        clippy::unimplemented,
        clippy::indexing_slicing
    )
)]

use serde::{Deserialize, Serialize};

/// `waterslide.json` — 21 keys (§3.3).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WaterslideTable {
    pub drop_length: f64,
    pub drop_angle_deg: f64,
    pub transition_radius: f64,
    pub helix_radius: f64,
    pub helix_turns: f64,
    pub helix_slope_deg: f64,
    pub runout_length: f64,
    pub spine_samples: u32,
    pub chute_width: f64,
    pub chute_wall_height: f64,
    pub chute_top_fraction: f64,
    pub chute_wall_thickness: f64,
    pub chute_floor_thickness: f64,
    pub runout_widening: f64,
    pub stations: u32,
    pub pool_radius: f64,
    pub pool_depth: f64,
    pub pool_rim_height: f64,
    pub pool_center_fraction: f64,
    pub tower_radius: f64,
    pub tower_clearance: f64,
}

/// `teapot.json` — 13 keys (§3.3).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TeapotTable {
    pub wall_thickness: f64,
    pub foot_height: f64,
    pub spout_r0: f64,
    pub spout_r1: f64,
    pub spout_ring: u32,
    pub handle_radius: f64,
    pub handle_ring: u32,
    pub stations: u32,
    pub body_stations: Vec<[f64; 2]>,
    pub spout_points: Vec<[f64; 3]>,
    pub spout_plane_normal: [f64; 3],
    pub handle_points: Vec<[f64; 3]>,
    pub handle_plane_normal: [f64; 3],
}

/// `amphora.json` — 9 keys (§3.3).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AmphoraTable {
    pub y_squash: f64,
    pub rib_ring: u32,
    pub handle_radius: f64,
    pub handle_ring: u32,
    pub handle_azimuth_deg: f64,
    pub stations: u32,
    pub body_stations: Vec<[f64; 2]>,
    pub handle_points: Vec<[f64; 3]>,
    pub foot: [f64; 3],
}

/// The three v1 tables, name-keyed. Test-only: the bridge reads tables from
/// the Python side (PB-005); the Rust suite consumes the real showcase files.
#[cfg(test)]
pub const TABLE_NAMES: [&str; 3] = ["waterslide", "teapot", "amphora"];

/// Absolute path of a showcase table within this worktree (tests read the real
/// files the builders consume). Test-only: see [`TABLE_NAMES`].
#[cfg(test)]
pub fn table_path(name: &str) -> std::path::PathBuf {
    let manifest = env!("CARGO_MANIFEST_DIR");
    let mut path = std::path::PathBuf::from(manifest);
    path.push("..");
    path.push("showcases");
    path.push("tables");
    path.push(format!("{name}.json"));
    path
}
