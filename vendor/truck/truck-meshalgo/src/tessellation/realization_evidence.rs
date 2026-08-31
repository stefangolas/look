#![deny(clippy::unwrap_used)]
//! BG-CG-007-CERT — the realization-evidence assembly (mapping A).
//! Builds `RealizationEvidence` over a realized mesh, an optional edge-sample
//! ledger, and the verdict. Types live in truck-base (mapping placement
//! correction, 2026-08-31); this module is the integration, not the type home.

use crate::tessellation::EdgeSampleLedger;
use truck_base::evidence::{
    ConstructErrorSummary, RealizationCertificate, RealizationEvidence, RealizationVerdict,
    SharedEdgePairEvidence,
};

/// Assembles the realization evidence record (mapping A).
///
/// `winding_violations` is the winding-audit violation count: a non-zero count
/// is FAILED, never a warning (mapping A row 4), so it overrides whatever
/// verdict the caller supplied. Everything else rides through untouched — no
/// conversion anywhere, an `Inconclusive` verdict stays `Inconclusive`.
pub fn assemble(
    winding_violations: usize,
    verdict: RealizationVerdict,
    certificate: Option<RealizationCertificate>,
    shared_edge_pairs: Vec<SharedEdgePairEvidence>,
    construct_error: Option<ConstructErrorSummary>,
) -> RealizationEvidence {
    let verdict = if winding_violations > 0 {
        RealizationVerdict::Failed
    } else {
        verdict
    };
    RealizationEvidence {
        construct_error,
        certificate,
        shared_edge_pairs,
        verdict,
    }
}

/// Computes the shared-edge pair evidence over two ledgers for the same edge
/// (mapping A row 3).
///
/// Both ledgers record the shared edge's canonical sample sequence: face A
/// consumes it forward, face B consumes it reversed (the CG-005
/// integer-identity convention, `I(A,E) == reverse(I(B,E))` as integers).
/// Against the landed ledger the comparison is exactly the integer-identity
/// check. Exactness is expressed by absence of rows: when the two consumed
/// sequences are index-identical, no zero-error row is emitted. Any mismatch
/// emits one `SharedEdgePairEvidence` row with the measured per-face index
/// deviation from the shared canonical sequence (face A's forward reading).
/// Nothing is welded, averaged, or rounded.
pub fn ledger_shared_edge_pairs(
    ledger_a: &EdgeSampleLedger,
    ledger_b: &EdgeSampleLedger,
) -> Vec<SharedEdgePairEvidence> {
    let canonical = &ledger_a.position_indices;
    let consumed_b: Vec<usize> = ledger_b.position_indices.iter().rev().copied().collect();
    let deviation_b = index_deviation(canonical, &consumed_b);
    if deviation_b == 0.0 {
        return Vec::new();
    }
    vec![SharedEdgePairEvidence {
        error_a: 0.0,
        error_b: deviation_b,
    }]
}

/// The summed index deviation of `consumed` from `canonical`, ordinal by
/// ordinal; an unpaired sample counts as one index unit of deviation. Non-
/// negative, deterministic, and never averaged.
fn index_deviation(canonical: &[usize], consumed: &[usize]) -> f64 {
    let n = canonical.len().max(consumed.len());
    let mut deviation = 0.0;
    for i in 0..n {
        match (canonical.get(i), consumed.get(i)) {
            (Some(&a), Some(&b)) => deviation += a.abs_diff(b) as f64,
            (Some(_), None) | (None, Some(_)) => deviation += 1.0,
            (None, None) => {}
        }
    }
    deviation
}
