//! The PB-004 required Rust suite (four tests).
//!
//! Tests 1–3 are pure Rust (no interpreter): the frozen mapping, the
//! unresolved payload, and the v1 table round trip. Test 4 embeds the
//! CPython interpreter (pyo3's `auto-initialize` dev feature) to prove the
//! GIL policy with a second thread and to smoke the module init + typed
//! hierarchy end to end. All four terminate by construction: every bounded
//! wait in test 4 has a timeout, and a mechanism failure fails the assertion
//! instead of hanging.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::time::Duration;

use pyo3::prelude::*;

use crate::gil::{self, ModulusSpec};
use crate::marshal::{Marshaled, MarshaledPayload, witness_name};
use crate::tables::{AmphoraTable, TeapotTable, WaterslideTable};
use truck_base::evidence::{
    Budget, Certificate, Collapse, ContradictionWitness, EnvelopeCase, Margin, MarginWitness,
    Method, Modulus, ModulusShape, Prop, PropMap, Refusal, RepairWitness, Truth, UnresolvedWitness,
};

/// One representative per `Refusal` variant (all six `EnvelopeCase` payloads
/// included, plus a Collapsed refusal carrying a full certificate). Exhaustive
/// over the landed enum: `Marshaled::from_refusal`'s own match is
/// compile-exhaustive, and this list is what test 1 walks.
fn every_refusal_variant() -> Vec<(Refusal, crate::ExceptionClass)> {
    use crate::ExceptionClass::Refused as RefusedClass;
    use crate::ExceptionClass::Unresolved as UnresolvedClass;

    let mut cases: Vec<(Refusal, crate::ExceptionClass)> = vec![
        (Refusal::Empty, RefusedClass),
        (
            Refusal::CompositionMarginExhausted(MarginWitness {
                stage: "margin-test-stage",
            }),
            RefusedClass,
        ),
        (
            Refusal::InputOutsideBackwardBudget(RepairWitness {
                stage: "repair-test-stage",
            }),
            RefusedClass,
        ),
        (
            Refusal::Contradictory(ContradictionWitness {
                prop: Prop::AnalyticCarrier,
                left: Truth::True,
                right: Truth::False,
            }),
            RefusedClass,
        ),
        (
            Refusal::Collapsed(
                Collapse {
                    reason: truck_base::evidence::CollapseReason::KnifeEdge,
                },
                sample_certificate(),
            ),
            RefusedClass,
        ),
        (
            Refusal::Collapsed(
                Collapse {
                    reason: truck_base::evidence::CollapseReason::ApexVanishing,
                },
                sample_certificate(),
            ),
            RefusedClass,
        ),
        (
            Refusal::ForwardToleranceExceeded {
                bound: 2.5,
                allowed: 1.25,
            },
            RefusedClass,
        ),
    ];
    for case in [
        EnvelopeCase::ChartDegenerate,
        EnvelopeCase::ReachTooSmall,
        EnvelopeCase::NonCanonicalCarrier,
        EnvelopeCase::NonPositiveNurbsWeight,
        EnvelopeCase::ContactReductionDeferred,
        EnvelopeCase::ConstructRefused,
    ] {
        cases.push((Refusal::UnsupportedEnvelope(case), RefusedClass));
    }
    cases.push((
        Refusal::NumericallyUnresolved {
            spent: Budget::new(11, 7, 3),
            witness: UnresolvedWitness::DeviationUncertified,
        },
        UnresolvedClass,
    ));
    cases
}

/// A certificate with every field populated and two set properties, so the
/// `Collapsed` payload exercises the full evidence rendering.
fn sample_certificate() -> Certificate {
    let mut props = PropMap::new();
    props.set(Prop::AnalyticCarrier, Truth::True);
    props.set(Prop::SoundEnclosure, Truth::True);
    Certificate {
        props,
        method: Method::Float,
        budget_left: Budget::new(9, 4, 2),
        margin: Margin::from_log2(3.0),
        modulus: Modulus {
            shape: ModulusShape::Lipschitz(1.5),
            domain: f64::INFINITY,
        },
    }
}

#[test]
fn refusal_maps_to_typed_exception() {
    let cases = every_refusal_variant();
    assert!(cases.len() >= 8);
    for (refusal, expected_class) in cases {
        let marshaled = Marshaled::from_refusal(&refusal);
        // Every variant lands on exactly one of the two typed classes; the
        // class tag is the frozen table's row (never a bare Exception).
        assert_eq!(marshaled.class, expected_class);
        assert!(!marshaled.message.is_empty());
        // Round trip through serde to JSON and back: lossless.
        let json = serde_json::to_string(&marshaled).expect("marshaled refusal serializes");
        let back: Marshaled = serde_json::from_str(&json).expect("marshaled refusal parses");
        assert_eq!(
            back, marshaled,
            "payload lost data across the JSON round trip"
        );
        // The wire form carries the class tag and a payload object.
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(value.get("class").is_some());
        assert!(value.get("payload").is_some());
    }
}

#[test]
fn unresolved_carries_witness_payload() {
    // κ ledger is deliberately distinctive so a dropped counter is caught.
    let ledger = Budget::new(41, 17, 5);
    for witness in [
        UnresolvedWitness::UncertifiedContainment,
        UnresolvedWitness::RootNotIsolated,
        UnresolvedWitness::KrawczykIndeterminate,
        UnresolvedWitness::ContactCurveNotFound,
        UnresolvedWitness::DeviationUncertified,
    ] {
        let refusal = Refusal::NumericallyUnresolved {
            spent: ledger,
            witness,
        };
        let marshaled = Marshaled::from_refusal(&refusal);
        assert_eq!(marshaled.class, crate::ExceptionClass::Unresolved);
        let payload = match &marshaled.payload {
            MarshaledPayload::Unresolved(p) => p,
            other => panic!("expected an Unresolved payload, got {other:?}"),
        };
        assert_eq!(payload.witness, witness_name(witness));
        assert_eq!(payload.kappa.subdiv, 41);
        assert_eq!(payload.kappa.newton, 17);
        assert_eq!(payload.kappa.depth, 5);

        // Full witness fidelity: nothing dropped across serde → JSON → serde.
        let json = serde_json::to_string(&marshaled).expect("unresolved marshals");
        let back: Marshaled = serde_json::from_str(&json).expect("unresolved parses");
        assert_eq!(back, marshaled);

        // The wire JSON carries every field by name — κ (subdiv/newton/depth)
        // and the witness — so a future schema drift is visible in the bytes.
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        let payload_value = value.get("payload").unwrap().as_object().unwrap();
        assert!(payload_value.contains_key("kappa"));
        assert_eq!(
            payload_value.get("witness").unwrap().as_str().unwrap(),
            witness_name(witness)
        );
        let kappa = payload_value.get("kappa").unwrap().as_object().unwrap();
        assert_eq!(kappa.len(), 3);
        assert_eq!(kappa.get("subdiv").unwrap().as_u64(), Some(41));
        assert_eq!(kappa.get("newton").unwrap().as_u64(), Some(17));
        assert_eq!(kappa.get("depth").unwrap().as_u64(), Some(5));
    }
}

#[test]
fn table_serde_round_trip() {
    for name in crate::tables::TABLE_NAMES {
        let path = crate::tables::table_path(name);
        let text =
            std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {path:?}: {e}"));
        let dict: serde_json::Value =
            serde_json::from_str(&text).unwrap_or_else(|e| panic!("{name} is not valid JSON: {e}"));
        match name {
            "waterslide" => round_trip_identity::<WaterslideTable>(name, &dict),
            "teapot" => round_trip_identity::<TeapotTable>(name, &dict),
            "amphora" => round_trip_identity::<AmphoraTable>(name, &dict),
            other => panic!("unexpected table name {other}"),
        }
    }
}

fn round_trip_identity<T>(name: &str, dict: &serde_json::Value)
where
    T: serde::de::DeserializeOwned + serde::Serialize + std::fmt::Debug + PartialEq,
{
    let table: T = serde_json::from_value(dict.clone())
        .unwrap_or_else(|e| panic!("{name} failed schema v1 validation: {e}"));
    let back: serde_json::Value = serde_json::to_value(&table).expect("table serializes");
    assert_eq!(
        &back, dict,
        "{name}: dict -> struct -> dict round trip is not identical"
    );
}

/// Structure of the GIL proof, as documented in RESULT.json notes:
///
/// 1. The test registers the embedded module (`append_to_inittab`, which
///    requires an un-initialized interpreter) and then attaches on the main
///    test thread — the first interpreter use in this process, so auto-init
///    happens here, never on a worker.
/// 2. Inside `Python::attach` the module smoke runs (import, hierarchy,
///    kernel call-slot success, typed refusal).
/// 3. A second thread is spawned while the main thread holds the GIL; that
///    thread blocks inside `Python::attach` until the GIL is free, then sets
///    a flag and signals a channel. The main thread now issues a kernel call
///    through `crate::gil::with_kernel_gil_released`; the detached closure
///    runs the landed compose entry and then waits on the channel.
/// 4. The flag can only have been set while the kernel closure was running if
///    the GIL was actually released. A broken mechanism (GIL held) fails the
///    assertion after the bounded channel timeout instead of hanging.
#[test]
fn gil_released_during_kernel_call() {
    use crate::truck123d;

    // The embedded CPython needs its home so `Py_InitializeEx` can import the
    // stdlib (`encodings`). pyo3 discovered this interpreter at build time via
    // `python` on PATH, so derive the same prefix here and point PYTHONHOME at
    // it before the interpreter starts. This is the documented bootstrap for
    // running pyo3's embedded-interpreter tests.
    if std::env::var_os("PYTHONHOME").is_none() {
        let probe = std::process::Command::new("python")
            .args(["-c", "import sys; print(sys.prefix)"])
            .output();
        if let Ok(output) = probe
            && output.status.success()
        {
            let home = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !home.is_empty() {
                // SAFETY: single-threaded bootstrap point, before any
                // interpreter is started; no other env access is racing.
                unsafe { std::env::set_var("PYTHONHOME", &home) };
            }
        }
    }

    pyo3::append_to_inittab!(truck123d);

    Python::attach(|py| {
        // Module init + the frozen hierarchy, end to end. A refused kernel
        // call raises the typed `Refused` class, payload included.
        let smoke = cr#"
import truck123d
assert issubclass(truck123d.Refused, truck123d.TruckError)
assert issubclass(truck123d.Unresolved, truck123d.TruckError)
assert issubclass(truck123d.TruckError, Exception)
assert hasattr(truck123d, 'TruckSolid')
solid = truck123d.TruckSolid('compose')
assert solid.kind == 'compose'
import json
res = truck123d.kernel_evidence_compose(
    '{"shape":"lipschitz","k":2.0}', '{"shape":"lipschitz","k":3.0}')
r = json.loads(res)
assert r['ok'] is True
assert r['shape']['kind'] == 'lipschitz'
assert abs(r['shape']['k'] - 6.0) < 1e-9  # H-3: float epsilon between marshaled values, not a length
try:
    truck123d.kernel_evidence_compose(
        '{"shape":"unbounded"}', '{"shape":"lipschitz","k":1.0}')
    raise SystemExit('composing an unbounded modulus must refuse')
except truck123d.Refused as e:
    assert e.case == 'forward_tolerance_exceeded', e
"#;
        py.run(smoke, None, None)
            .expect("embedded truck123d smoke must pass");

        // --- GIL-release proof ---
        let saw_gil = Arc::new(AtomicBool::new(false));
        let saw_gil_observer = Arc::clone(&saw_gil);
        let (tx, rx) = mpsc::channel::<()>();

        let observer = std::thread::spawn(move || {
            // This thread can only run to completion while the GIL is free:
            // `Python::attach` blocks until the main thread's kernel call
            // releases it.
            Python::attach(|_py| {
                saw_gil_observer.store(true, Ordering::SeqCst);
                let _ = tx.send(());
            });
        });

        let outcome = gil::with_kernel_gil_released(py, move || {
            // The kernel call issued from a GIL-holding thread: landed entry
            // `Modulus::compose` (evidence.rs BG-EVD), no geometry.
            let result = gil::compose_kernel(
                ModulusSpec::Lipschitz {
                    k: 2.0,
                    domain: None,
                },
                ModulusSpec::Lipschitz {
                    k: 3.0,
                    domain: None,
                },
            );
            // Hold the detached closure open until the observer has had its
            // chance to prove the GIL was free. Bounded: a broken mechanism
            // times out instead of hanging.
            let _ = rx.recv_timeout(Duration::from_secs(10));
            result
        });

        let composed = outcome.expect("compose of two Lipschitz moduli must certify");
        assert_eq!(
            composed.shape,
            ModulusShape::Lipschitz(6.0),
            "Lipschitz(2.0) composed with Lipschitz(3.0) must certify as Lipschitz(6.0)"
        );
        assert!(
            saw_gil.load(Ordering::SeqCst),
            "the observer thread could not acquire the GIL while the kernel call ran — \
             the GIL was not released (with_kernel_gil_released is a no-op?)"
        );

        observer.join().expect("GIL observer thread must not panic");
    });
}
