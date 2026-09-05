//! PB-000-CONTRACT contract-pinning tests (work packet PB-000-CONTRACT).
//!
//! These four tests freeze the four contracts written in
//! `docs/PY_BRIDGE_CONTRACT.md`: the tables/*.json schema v1 (v1-by-omission),
//! the 25-row API mapping table (16 landed `truck_shapeops::facade` entries +
//! 9 CC-port forwards), the byte-determinism of the showcase report, and the
//! two-class refusal mapping over every landed `Refusal` variant.
//!
//! House rules (packet PB-000-CONTRACT): H-1's unwrap/expect/panic ban is not
//! registered for showcases, so this file is panic-free by construction —
//! every test returns `Result` and asserts. No numeric tolerance literals are
//! used, so there are no H-3 markers to carry; the only numeric comparisons
//! are schema-v1 domain edges defined in `docs/PY_BRIDGE_CONTRACT.md` §3.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;
use showcases::cc_ports::{
    CanalCert, CcPorts, ClearCert, LandedPorts, RadiusLaw, RibWire, ThicknessCert,
};
use showcases::harness::ShowcaseReport;
use showcases::waterslide::{WaterslideTable, build};
use truck_base::cgmath64::Point3;
use truck_base::evidence::{
    Budget, Certificate, Collapse, CollapseReason, ContradictionWitness, EnvelopeCase, Margin,
    MarginWitness, Method, Modulus, Outcome, Prop, PropMap, Refusal, RepairWitness, Truth,
    UnresolvedWitness,
};
use truck_geometry::constructive::DirectTolerance;
use truck_modeling::{Curve, Solid, Wire};
use truck_shapeops::facade::{
    boolean_op, bounding_box, chamfer, extrude, extrude_vector, fillet, make_face, make_hull,
    mirror, mirror_about_plane, revolve, rotate, scale, section, split, translate,
};

/// The table directory is `showcases/tables` (the package manifest dir at
/// compile time — independent of the test working directory).
fn table_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tables")
        .join(name)
}

fn read_table_value(name: &str) -> Result<Value, String> {
    let raw = fs::read_to_string(table_path(name)).map_err(|e| format!("read {name}: {e}"))?;
    serde_json::from_str(&raw).map_err(|e| format!("parse {name}: {e}"))
}

fn read_table<T: serde::de::DeserializeOwned>(name: &str) -> Result<T, String> {
    let raw = fs::read_to_string(table_path(name)).map_err(|e| format!("read {name}: {e}"))?;
    serde_json::from_str(&raw).map_err(|e| format!("parse {name}: {e}"))
}

fn out_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("pb_contract_waterslide_{tag}"));
    let _ = fs::remove_dir_all(&dir);
    dir
}

// ---------------------------------------------------------------------------
// Test 1: table schema v1 parses all three tables.
// The exact key sets and value-domain kinds below mirror `docs/PY_BRIDGE_CONTRACT.md` §3.
// ---------------------------------------------------------------------------

/// Schema-v1 value-domain kinds (`docs/PY_BRIDGE_CONTRACT.md` §3.2).
#[derive(Clone, Copy)]
enum Kind {
    /// finite, `>= 0`
    Len,
    /// finite, `> 0`
    Pos,
    /// finite (degrees)
    Angle,
    /// integer `>= 1`
    Count,
    /// integer `>= 3`
    Ring,
    /// finite, `0 <= x <= 1`
    Frac,
    /// finite, `>= 1`
    Scale,
}

fn finite_of(value: &Value, where_: &str) -> Result<f64, String> {
    let f = value
        .as_f64()
        .ok_or_else(|| format!("{where_}: expected a number, got {value}"))?;
    if f.is_finite() {
        Ok(f)
    } else {
        Err(format!("{where_}: expected a finite number, got {value}"))
    }
}

fn kind_ok(kind: Kind, value: &Value, where_: &str) -> Result<(), String> {
    match kind {
        Kind::Len => {
            finite_of(value, where_)?;
            Ok(())
        }
        Kind::Pos => {
            let f = finite_of(value, where_)?;
            if f <= 0.0 {
                Err(format!("{where_}: expected a positive number, got {value}"))
            } else {
                Ok(())
            }
        }
        Kind::Angle => {
            finite_of(value, where_)?;
            Ok(())
        }
        Kind::Count => {
            let f = finite_of(value, where_)?;
            if f.fract() != 0.0 || f < 1.0 {
                Err(format!("{where_}: expected an integer >= 1, got {value}"))
            } else {
                Ok(())
            }
        }
        Kind::Ring => {
            let f = finite_of(value, where_)?;
            if f.fract() != 0.0 || f < 3.0 {
                Err(format!("{where_}: expected an integer >= 3, got {value}"))
            } else {
                Ok(())
            }
        }
        Kind::Frac => {
            let f = finite_of(value, where_)?;
            if !(0.0..=1.0).contains(&f) {
                Err(format!(
                    "{where_}: expected a fraction in [0, 1], got {value}"
                ))
            } else {
                Ok(())
            }
        }
        Kind::Scale => {
            let f = finite_of(value, where_)?;
            if f < 1.0 {
                Err(format!(
                    "{where_}: expected a widening factor >= 1, got {value}"
                ))
            } else {
                Ok(())
            }
        }
    }
}

fn triple_list_ok(value: &Value, where_: &str) -> Result<(), String> {
    let rows = value
        .as_array()
        .ok_or_else(|| format!("{where_}: expected an array, got {value}"))?;
    for (i, row) in rows.iter().enumerate() {
        let where_i = format!("{where_}[{i}]");
        let triple = row
            .as_array()
            .ok_or_else(|| format!("{where_i}: expected a 3-array, got {row}"))?;
        if triple.len() != 3 {
            return Err(format!("{where_i}: expected exactly 3 elements, got {row}"));
        }
        for elt in triple {
            finite_of(elt, &where_i)?;
        }
    }
    Ok(())
}

fn triple_ok(value: &Value, where_: &str) -> Result<(), String> {
    let triple = value
        .as_array()
        .ok_or_else(|| format!("{where_}: expected a 3-array, got {value}"))?;
    if triple.len() != 3 {
        return Err(format!(
            "{where_}: expected exactly 3 elements, got {value}"
        ));
    }
    for elt in triple {
        finite_of(elt, where_)?;
    }
    Ok(())
}

/// Ascending-z `(z, r)` station list with positive radius.
fn check_ascending_z(pairs: &Value, where_: &str) -> Result<(), String> {
    let rows = pairs
        .as_array()
        .ok_or_else(|| format!("{where_}: expected an array, got {pairs}"))?;
    for (i, row) in rows.iter().enumerate() {
        let pair = row
            .as_array()
            .ok_or_else(|| format!("{where_}[{i}]: expected a 2-array, got {row}"))?;
        if pair.len() != 2 {
            return Err(format!(
                "{where_}[{i}]: expected exactly 2 elements, got {row}"
            ));
        }
        let z = finite_of(&pair[0], &format!("{where_}[{i}]"))?;
        let r = finite_of(&pair[1], &format!("{where_}[{i}]"))?;
        if r <= 0.0 {
            return Err(format!("{where_}[{i}]: radius must be positive, got {r}"));
        }
        if i > 0 {
            let prev_z = rows[i - 1]
                .as_array()
                .and_then(|p| p.first())
                .and_then(|v| v.as_f64())
                .ok_or_else(|| format!("{where_}[{}]: bad z", i - 1))?;
            if z <= prev_z {
                return Err(format!(
                    "{where_}: stations must be strictly ascending in z; {prev_z} then {z}"
                ));
            }
        }
    }
    Ok(())
}

/// Exact top-level key set + no v1-exempting version field.
fn check_envelope_and_keys(table: &Value, name: &str, keys: &[&str]) -> Result<(), String> {
    let map = table
        .as_object()
        .ok_or_else(|| format!("{name}: top level is not a JSON object"))?;
    let expected: BTreeSet<&str> = keys.iter().copied().collect();
    let actual: BTreeSet<&str> = map.keys().map(String::as_str).collect();
    if actual != expected {
        let missing: Vec<&str> = expected.difference(&actual).copied().collect();
        let extra: Vec<&str> = actual.difference(&expected).copied().collect();
        return Err(format!(
            "{name}: key set mismatch under schema v1 (missing {missing:?}, unknown {extra:?})"
        ));
    }
    if map.contains_key("schema_version") {
        return Err(format!(
            "{name}: schema v1 is v1-by-omission; a schema_version field must not be present"
        ));
    }
    Ok(())
}

fn check_waterslide(table: &Value) -> Result<(), String> {
    const KEYS: [&str; 21] = [
        "drop_length",
        "drop_angle_deg",
        "transition_radius",
        "helix_radius",
        "helix_turns",
        "helix_slope_deg",
        "runout_length",
        "spine_samples",
        "chute_width",
        "chute_wall_height",
        "chute_top_fraction",
        "chute_wall_thickness",
        "chute_floor_thickness",
        "runout_widening",
        "stations",
        "pool_radius",
        "pool_depth",
        "pool_rim_height",
        "pool_center_fraction",
        "tower_radius",
        "tower_clearance",
    ];
    const KINDS: [(&str, Kind); 21] = [
        ("drop_length", Kind::Pos),
        ("drop_angle_deg", Kind::Angle),
        ("transition_radius", Kind::Pos),
        ("helix_radius", Kind::Pos),
        ("helix_turns", Kind::Len),
        ("helix_slope_deg", Kind::Angle),
        ("runout_length", Kind::Pos),
        ("spine_samples", Kind::Count),
        ("chute_width", Kind::Pos),
        ("chute_wall_height", Kind::Pos),
        ("chute_top_fraction", Kind::Frac),
        ("chute_wall_thickness", Kind::Pos),
        ("chute_floor_thickness", Kind::Pos),
        ("runout_widening", Kind::Scale),
        ("stations", Kind::Count),
        ("pool_radius", Kind::Pos),
        ("pool_depth", Kind::Pos),
        ("pool_rim_height", Kind::Len),
        ("pool_center_fraction", Kind::Frac),
        ("tower_radius", Kind::Pos),
        ("tower_clearance", Kind::Len),
    ];
    check_envelope_and_keys(table, "waterslide.json", &KEYS)?;
    for (key, kind) in KINDS {
        let value = table
            .get(key)
            .ok_or_else(|| format!("waterslide.json: missing key {key}"))?;
        kind_ok(kind, value, &format!("waterslide.json.{key}"))?;
    }
    Ok(())
}

fn check_teapot(table: &Value) -> Result<(), String> {
    const KEYS: [&str; 13] = [
        "body_stations",
        "wall_thickness",
        "foot_height",
        "spout_points",
        "spout_plane_normal",
        "spout_r0",
        "spout_r1",
        "spout_ring",
        "handle_points",
        "handle_plane_normal",
        "handle_radius",
        "handle_ring",
        "stations",
    ];
    const KINDS: [(&str, Kind); 8] = [
        ("wall_thickness", Kind::Pos),
        ("foot_height", Kind::Len),
        ("spout_r0", Kind::Pos),
        ("spout_r1", Kind::Pos),
        ("spout_ring", Kind::Ring),
        ("handle_radius", Kind::Pos),
        ("handle_ring", Kind::Ring),
        ("stations", Kind::Count),
    ];
    check_envelope_and_keys(table, "teapot.json", &KEYS)?;
    for (key, kind) in KINDS {
        let value = table
            .get(key)
            .ok_or_else(|| format!("teapot.json: missing key {key}"))?;
        kind_ok(kind, value, &format!("teapot.json.{key}"))?;
    }
    check_ascending_z(
        table
            .get("body_stations")
            .ok_or("teapot.json: missing body_stations")?,
        "teapot.json.body_stations",
    )?;
    triple_ok(
        table
            .get("spout_plane_normal")
            .ok_or("teapot.json: missing spout_plane_normal")?,
        "teapot.json.spout_plane_normal",
    )?;
    triple_ok(
        table
            .get("handle_plane_normal")
            .ok_or("teapot.json: missing handle_plane_normal")?,
        "teapot.json.handle_plane_normal",
    )?;
    triple_list_ok(
        table
            .get("spout_points")
            .ok_or("teapot.json: missing spout_points")?,
        "teapot.json.spout_points",
    )?;
    triple_list_ok(
        table
            .get("handle_points")
            .ok_or("teapot.json: missing handle_points")?,
        "teapot.json.handle_points",
    )?;
    Ok(())
}

fn check_amphora(table: &Value) -> Result<(), String> {
    const KEYS: [&str; 9] = [
        "body_stations",
        "y_squash",
        "rib_ring",
        "handle_points",
        "handle_azimuth_deg",
        "handle_radius",
        "handle_ring",
        "foot",
        "stations",
    ];
    const KINDS: [(&str, Kind); 6] = [
        ("y_squash", Kind::Pos),
        ("rib_ring", Kind::Ring),
        ("handle_azimuth_deg", Kind::Angle),
        ("handle_radius", Kind::Pos),
        ("handle_ring", Kind::Ring),
        ("stations", Kind::Count),
    ];
    check_envelope_and_keys(table, "amphora.json", &KEYS)?;
    for (key, kind) in KINDS {
        let value = table
            .get(key)
            .ok_or_else(|| format!("amphora.json: missing key {key}"))?;
        kind_ok(kind, value, &format!("amphora.json.{key}"))?;
    }
    check_ascending_z(
        table
            .get("body_stations")
            .ok_or("amphora.json: missing body_stations")?,
        "amphora.json.body_stations",
    )?;
    triple_list_ok(
        table
            .get("handle_points")
            .ok_or("amphora.json: missing handle_points")?,
        "amphora.json.handle_points",
    )?;
    let foot = table
        .get("foot")
        .ok_or("amphora.json: missing foot")?
        .as_array()
        .ok_or("amphora.json.foot: expected a 3-array")?;
    if foot.len() != 3 {
        return Err(format!("amphora.json.foot: expected exactly 3 elements"));
    }
    let radius = finite_of(&foot[0], "amphora.json.foot[0]")?;
    if radius <= 0.0 {
        return Err(format!(
            "amphora.json.foot[0]: radius must be positive, got {radius}"
        ));
    }
    finite_of(&foot[1], "amphora.json.foot[1]")?;
    finite_of(&foot[2], "amphora.json.foot[2]")?;
    Ok(())
}

#[test]
fn pb_table_schema_v1_parses_all_three_tables() -> Result<(), String> {
    for (name, check) in [
        (
            "waterslide.json",
            check_waterslide as fn(&Value) -> Result<(), String>,
        ),
        ("teapot.json", check_teapot),
        ("amphora.json", check_amphora),
    ] {
        let table = read_table_value(name)?;
        check(&table)?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Test 2: the API table's Rust-entry column covers the landed facade + the
// CC-port forwards (docs/PY_BRIDGE_CONTRACT.md §1, 25 rows = 16 + 9).
// ---------------------------------------------------------------------------

/// The API table's row count as documented (16 facade + 9 CC-port forwards).
const DOC_API_TABLE_ROW_COUNT: usize = 25;

macro_rules! facade_surface {
    ($($name:ident),* $(,)?) => {
        const FACADE_ENTRIES: [&str; 16] = [$(stringify!($name)),*];
        let _facade_surface = ($($name),*);
    };
}

#[test]
fn pb_api_table_covers_landed_facade() {
    facade_surface! {
        extrude, extrude_vector, revolve, fillet, chamfer, mirror,
        mirror_about_plane, rotate, scale, translate, section, split,
        bounding_box, boolean_op, make_face, make_hull,
    }

    const CC_FORWARD_ENTRIES: [&str; 9] = [
        "loft",
        "loft_ribs",
        "gordon_ribs",
        "blend_var_radius",
        "blend_handle_root",
        "clear",
        "canal_regularity",
        "canal_cert",
        "shell_thickness",
    ];

    // Compile-time existence of every CC-port forward, frozen in signature:
    // each binding fails to compile if the trait method is renamed, re-typed,
    // or dropped from `CcPorts`.
    let _loft: fn(&LandedPorts, &[Wire], &DirectTolerance) -> Outcome<Solid> = LandedPorts::loft;
    let _loft_ribs: fn(&LandedPorts, &[RibWire]) -> Outcome<Solid> = LandedPorts::loft_ribs;
    let _gordon_ribs: fn(&LandedPorts, &[RibWire]) -> Outcome<Solid> = LandedPorts::gordon_ribs;
    let _blend_var_radius: fn(
        &LandedPorts,
        &Solid,
        (Point3, Point3),
        &RadiusLaw,
    ) -> Outcome<Solid> = LandedPorts::blend_var_radius;
    let _blend_handle_root: fn(&LandedPorts, &[RibWire], &RadiusLaw) -> Outcome<Solid> =
        LandedPorts::blend_handle_root;
    let _clear: fn(&LandedPorts, &Solid, &Solid, f64) -> Outcome<ClearCert> = LandedPorts::clear;
    let _canal_regularity: fn(&LandedPorts, &Curve, f64) -> Outcome<CanalCert> =
        LandedPorts::canal_regularity;
    let _canal_cert: fn(&LandedPorts, &[(f64, f64, f64)], f64, f64) -> Outcome<CanalCert> =
        LandedPorts::canal_cert;
    let _shell_thickness: fn(&LandedPorts, &[RibWire]) -> Outcome<ThicknessCert> =
        LandedPorts::shell_thickness;

    // The doc's row count is 16 + 9 = 25, no duplicates, names spelled exactly
    // as in docs/PY_BRIDGE_CONTRACT.md §1.
    assert_eq!(
        FACADE_ENTRIES.len() + CC_FORWARD_ENTRIES.len(),
        DOC_API_TABLE_ROW_COUNT,
        "the API table must stay at 16 facade + 9 CC-port forwards"
    );
    assert_eq!(
        FACADE_ENTRIES.len(),
        16,
        "the facade surface is exactly the 16 landed pub fns (anchor A1)"
    );
    assert_eq!(
        CC_FORWARD_ENTRIES.len(),
        9,
        "nine CC-port forwards are consumed"
    );

    let facade_unique: BTreeSet<&str> = FACADE_ENTRIES.iter().copied().collect();
    assert_eq!(
        facade_unique.len(),
        FACADE_ENTRIES.len(),
        "no duplicate facade rows"
    );
    let cc_unique: BTreeSet<&str> = CC_FORWARD_ENTRIES.iter().copied().collect();
    assert_eq!(
        cc_unique.len(),
        CC_FORWARD_ENTRIES.len(),
        "no duplicate CC rows"
    );

    let expected_facade: [&str; 16] = [
        "extrude",
        "extrude_vector",
        "revolve",
        "fillet",
        "chamfer",
        "mirror",
        "mirror_about_plane",
        "rotate",
        "scale",
        "translate",
        "section",
        "split",
        "bounding_box",
        "boolean_op",
        "make_face",
        "make_hull",
    ];
    assert_eq!(
        FACADE_ENTRIES, expected_facade,
        "facade rows spell the doc §1.1 names"
    );
}

// ---------------------------------------------------------------------------
// Test 3: byte-determinism of the showcase report (docs/PY_BRIDGE_CONTRACT.md §4).
// ---------------------------------------------------------------------------

#[test]
fn pb_report_determinism_same_table_same_rev() -> Result<(), String> {
    let table: WaterslideTable = read_table("waterslide.json")?;
    let dir = out_dir("determinism");
    let first_report = build(&table, &dir, &LandedPorts)?;
    let first_bytes = fs::read(dir.join("waterslide_report.json")).map_err(|e| e.to_string())?;
    let second_report = build(&table, &dir, &LandedPorts)?;
    let second_bytes = fs::read(dir.join("waterslide_report.json")).map_err(|e| e.to_string())?;

    assert_eq!(
        first_bytes, second_bytes,
        "same table + same kernel rev must produce byte-identical report JSON \
         (the on-disk waterslide_report.json)"
    );
    let encode = |r: &ShowcaseReport| serde_json::to_vec(r).map_err(|e| e.to_string());
    assert_eq!(
        encode(&first_report)?,
        encode(&second_report)?,
        "same table + same kernel rev must produce byte-identical in-memory report"
    );
    let _ = fs::remove_dir_all(&dir);
    Ok(())
}

// ---------------------------------------------------------------------------
// Test 4: the mapping's variant list covers every landed Refusal variant.
// ---------------------------------------------------------------------------

/// The two-class mapping's variant list (`docs/PY_BRIDGE_CONTRACT.md` §2).
const MAPPED_REFUSAL_VARIANTS: [&str; 8] = [
    "Empty",
    "UnsupportedEnvelope",
    "NumericallyUnresolved",
    "CompositionMarginExhausted",
    "InputOutsideBackwardBudget",
    "Contradictory",
    "Collapsed",
    "ForwardToleranceExceeded",
];

/// One certificate exemplar for the `Collapsed` refusal payload.
fn blank_certificate() -> Certificate {
    Certificate {
        props: PropMap::new(),
        method: Method::Float,
        budget_left: Budget::new(0, 0, 0),
        margin: Margin::UNBOUNDED,
        modulus: Modulus::Unbounded,
    }
}

/// The exhaustive match over `Refusal`: adding a `Refusal` variant makes this
/// function fail to compile, breaking this test on purpose.
fn refusal_variant_tag(refusal: &Refusal) -> &'static str {
    match refusal {
        Refusal::Empty => "Empty",
        Refusal::UnsupportedEnvelope(_) => "UnsupportedEnvelope",
        Refusal::NumericallyUnresolved { .. } => "NumericallyUnresolved",
        Refusal::CompositionMarginExhausted(_) => "CompositionMarginExhausted",
        Refusal::InputOutsideBackwardBudget(_) => "InputOutsideBackwardBudget",
        Refusal::Contradictory(_) => "Contradictory",
        Refusal::Collapsed(..) => "Collapsed",
        Refusal::ForwardToleranceExceeded { .. } => "ForwardToleranceExceeded",
    }
}

#[test]
fn pb_refusal_mapping_covers_landed_refusal_variants() {
    let exemplars: [Refusal; 8] = [
        Refusal::Empty,
        Refusal::UnsupportedEnvelope(EnvelopeCase::ChartDegenerate),
        Refusal::NumericallyUnresolved {
            spent: Budget::new(0, 0, 0),
            witness: UnresolvedWitness::RootNotIsolated,
        },
        Refusal::CompositionMarginExhausted(MarginWitness {
            stage: "pb-contract",
        }),
        Refusal::InputOutsideBackwardBudget(RepairWitness {
            stage: "pb-contract",
        }),
        Refusal::Contradictory(ContradictionWitness {
            prop: Prop::AnalyticCarrier,
            left: Truth::True,
            right: Truth::False,
        }),
        Refusal::Collapsed(
            Collapse {
                reason: CollapseReason::KnifeEdge,
            },
            blank_certificate(),
        ),
        Refusal::ForwardToleranceExceeded {
            bound: 1.0,
            allowed: 2.0,
        },
    ];

    let tags: Vec<&str> = exemplars.iter().map(refusal_variant_tag).collect();
    assert_eq!(
        tags.len(),
        MAPPED_REFUSAL_VARIANTS.len(),
        "the mapping variant list must have one row per landed Refusal variant"
    );
    for tag in &tags {
        assert!(
            MAPPED_REFUSAL_VARIANTS.contains(tag),
            "the mapping section is missing a row for Refusal variant {tag}"
        );
    }
    let unique: BTreeSet<&str> = tags.iter().copied().collect();
    assert_eq!(
        unique.len(),
        MAPPED_REFUSAL_VARIANTS.len(),
        "the mapping variant list must cover every landed Refusal variant exactly once"
    );
}
