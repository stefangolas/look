//! MONO-8-SWEPT-ADMISSION-WIRING required suite -- the executor-boundary
//! admission wiring at the facade boundary.
//!
//! The packet's executor-path wiring (the exact extraction adapter plus the
//! fixed Swept x Swept gate order: extraction -> transversality -> solver)
//! lands in `bd_bridge.rs`; this root suite proves the facade-level admission
//! flip the wiring depends on. A recorded `cut(loft, loft)` / `fuse` pair over
//! two swept carriers now routes into the certified entry (a routed
//! `boolean_events` row) instead of the unconditional class-level
//! `NonCanonicalCarrier` refusal, while the canonical and torus paths are
//! untouched (V5 discipline).

use truck_base::evidence::{EnvelopeCase, Refusal};
use truck123d::{FacadeOp, FacadeReport, FacadeTable, ModeValue, run_facade};

/// A corpus-shaped spline carrier row (S5 section authoring).
fn spline_carrier() -> FacadeOp {
    FacadeOp::Spline {
        points: vec![[0.0, 0.0, 0.0], [1.0, 2.0, 0.0], [0.0, 0.0, 3.0]],
        periodic: false,
    }
}

/// Runs a table to completion: `Ok(report)` or a typed refusal.
fn run(ops: Vec<FacadeOp>) -> Result<FacadeReport, Refusal> {
    run_facade(&FacadeTable { ops })
}

/// The routed swept-carrier boolean rows of a report (the serialized report
/// omits the field when no swept-carrier boolean row ran).
fn routed_boolean_rows(report: &FacadeReport) -> Vec<serde_json::Value> {
    let value = serde_json::to_value(report).expect("the facade report serializes"); // H-3: serde output of a fixed report
    value
        .get("boolean_events")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default()
}

#[test]
fn swept_swept_pair_is_admitted_into_the_certified_gate() {
    // The monocoque-shaped `cut(loft, loft)` cell: the facade's carrier-class
    // consult now admits the pair, so it is recorded as one routed swept-carrier
    // boolean row instead of the old unconditional NonCanonicalCarrier refusal.
    let ops = vec![
        FacadeOp::PushPart,
        spline_carrier(),
        FacadeOp::Loft,
        FacadeOp::Mode {
            value: ModeValue::Subtract,
        },
        spline_carrier(),
        FacadeOp::Loft,
        FacadeOp::ExportStl {
            path: "cut_loft_loft.stl".to_string(),
        },
        FacadeOp::Pop,
    ];
    let report = run(ops).expect("a swept x swept boolean routes into the certified gate"); // H-3: the routed verdict is the assertion
    assert!(report.constructive, "a swept-carrier part is constructive");
    let rows = routed_boolean_rows(&report);
    assert_eq!(
        rows.len(),
        1,
        "the swept x swept pair is recorded as one routed row: {rows:?}"
    );
    let row = rows
        .first()
        .and_then(serde_json::Value::as_object)
        .expect("the routed row serializes as an object"); // H-3: machine-produced report shape
    assert_eq!(
        row.get("mode").and_then(serde_json::Value::as_str),
        Some("subtract")
    );
    assert_eq!(
        row.get("base").and_then(serde_json::Value::as_str),
        Some("swept")
    );
    assert_eq!(
        row.get("tool").and_then(serde_json::Value::as_str),
        Some("swept")
    );
    assert_eq!(report.exports.len(), 1);
}

#[test]
fn swept_admission_keeps_canonical_and_torus_paths() {
    // Canonical x canonical stays on the landed S1 path (no routed row).
    let canonical = vec![
        FacadeOp::PushPart,
        FacadeOp::Box {
            length: 2.0,
            width: 3.0,
            height: 1.0,
        },
        FacadeOp::Mode {
            value: ModeValue::Subtract,
        },
        FacadeOp::Box {
            length: 1.0,
            width: 1.0,
            height: 1.0,
        },
    ];
    let report = run(canonical).expect("canonical x canonical lands"); // H-3: the landed S1 control
    assert!(
        routed_boolean_rows(&report).is_empty(),
        "a canonical x canonical pair records no routed row"
    );
    assert!(!report.constructive);

    // A torus carrier in a swept pair keeps the typed, localized refusal.
    let torus = vec![
        FacadeOp::PushPart,
        spline_carrier(),
        FacadeOp::Loft,
        FacadeOp::Mode {
            value: ModeValue::Subtract,
        },
        FacadeOp::Torus {
            major_radius: 2.0,
            minor_radius: 0.5,
        },
    ];
    let refusal = run(torus).expect_err("a swept x torus boolean must refuse typed"); // H-3: the typed refusal is the assertion
    assert!(
        matches!(
            refusal,
            Refusal::UnsupportedEnvelope(EnvelopeCase::ContactReductionDeferred)
        ),
        "expected the localized ContactReductionDeferred refusal, got {refusal:?}"
    );
}
