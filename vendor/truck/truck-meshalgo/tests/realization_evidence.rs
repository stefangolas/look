#![deny(clippy::unwrap_used)]

//! BG-CG-007-CERT — tests for the realization-evidence assembly module
//! (`tessellation::realization_evidence`).

use truck_base::evidence::{Method, RealizationCertificate, RealizationVerdict};
use truck_meshalgo::tessellation::realization_evidence::{assemble, ledger_shared_edge_pairs};
use truck_meshalgo::tessellation::EdgeSampleLedger;

#[test]
fn ledger_assembly_fills_shared_edge_pairs() {
    // Identical integer sequences (the exact CG-005 identity): empty.
    let ledger_a = EdgeSampleLedger {
        edge: 7,
        parameters: vec![0.0, 0.5, 1.0],
        position_indices: vec![10, 11, 12],
    };
    let ledger_b = EdgeSampleLedger {
        edge: 7,
        parameters: vec![0.0, 0.5, 1.0],
        position_indices: vec![12, 11, 10],
    };
    assert!(
        ledger_shared_edge_pairs(&ledger_a, &ledger_b).is_empty(),
        "index-identical sequences emit no row"
    );

    // Mismatched sequences: exactly one row, both errors non-negative.
    let broken = EdgeSampleLedger {
        edge: 7,
        parameters: vec![0.0, 0.5, 1.0],
        position_indices: vec![12, 99, 10],
    };
    let rows = ledger_shared_edge_pairs(&ledger_a, &broken);
    assert_eq!(rows.len(), 1, "one row per mismatched edge");
    let row = match rows.first() {
        Some(row) => row,
        None => panic!("a mismatch must emit exactly one row"),
    };
    assert!(row.error_a >= 0.0); // H-3: non-negativity of a deviation measure, not a length
    assert!(row.error_b >= 0.0); // H-3: non-negativity of a deviation measure, not a length
}

#[test]
fn evidence_method_is_float_not_exact() {
    let certificate = RealizationCertificate {
        method: Method::Float,
        max_cell_twist: 1e-7,
        extent: 2.0,
    };
    let evidence = assemble(
        0,
        RealizationVerdict::CertifiedWithinTolerance,
        Some(certificate),
        Vec::new(),
        None,
    );
    let out = match evidence.certificate {
        Some(cert) => cert,
        None => panic!("the certificate must ride through assembly"),
    };
    assert_ne!(out.method, Method::Exact);
    assert_eq!(out.method, Method::Float);
}

#[test]
fn inconclusive_never_becomes_certified() {
    // An Inconclusive verdict assembled in stays Inconclusive out — with and
    // without a certificate.
    let bare = assemble(0, RealizationVerdict::Inconclusive, None, Vec::new(), None);
    assert_eq!(bare.verdict, RealizationVerdict::Inconclusive);
    assert!(bare.certificate.is_none());

    let certified = assemble(
        0,
        RealizationVerdict::Inconclusive,
        Some(RealizationCertificate {
            method: Method::Float,
            max_cell_twist: 0.0,
            extent: 1.0,
        }),
        Vec::new(),
        None,
    );
    assert_eq!(certified.verdict, RealizationVerdict::Inconclusive);
    assert_eq!(
        certified.certificate.map(|cert| cert.method),
        Some(Method::Float)
    );
}
