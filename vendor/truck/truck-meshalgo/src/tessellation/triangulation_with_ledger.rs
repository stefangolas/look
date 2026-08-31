#![deny(clippy::unwrap_used)]

//! BG-CG-005-LEDGER — the edge-sample ledger: a mesh position index is a
//! pure function of (entity identity, sample ordinal), never of coordinates
//! (the frozen convention, CG-000 module docs; plan §3.4).

use crate::tessellation::{
    formal, CertifiedLattice, LatticeMeshableShape, MeshedShellOutcome, Parallelizable,
    PolylineableCurve, RobustMeshableSurface,
};
use truck_base::cgmath64::*;
use truck_topology::compress::CompressedShell;

/// One unique edge's sample record: the sampled parameters, and the global
/// position indices of the sampled positions. A reversed edge USE consumes
/// the same integer sequence reversed — no second sampling, ever.
#[derive(Debug, Clone, PartialEq)]
pub struct EdgeSampleLedger {
    /// The compressed edge index — the entity identity in this
    /// representation (the plan's `EdgeID<Curve>`, booked spelling).
    pub edge: usize,
    /// The sampled parameters, ascending.
    pub parameters: Vec<f64>,
    /// The global position index of each sampled position, aligned with
    /// `parameters`.
    pub position_indices: Vec<usize>,
}

/// The whole ledger for one shell: one entry per unique compressed edge,
/// plus the global position table the indices reference.
#[derive(Debug, Clone, PartialEq)]
pub struct EdgeSampleLedgerSet {
    /// One entry per unique edge, ordered by edge index ascending.
    pub entries: Vec<EdgeSampleLedger>,
    /// The global position table. Positions are interned ONCE across the
    /// whole shell: two sampled positions that are exactly equal (f64 `==`
    /// on all three components) share one index. Nothing here merges
    /// near-equal positions — exact equality only; there is no welding.
    pub positions: Vec<Point3>,
}

/// Runs the UNCHANGED robust outcome path and, beside it, returns the
/// edge-sample ledger the watertightness invariant is stated over.
pub fn triangulation_with_ledger<C, S>(
    shell: &CompressedShell<Point3, C, S>,
    tol: f64,
    lattice_of: impl Fn(&S) -> CertifiedLattice + Parallelizable,
    schema_of: impl Fn(&S) -> formal::SupportSurfaceSchema + Parallelizable,
    curve_schema_of: impl Fn(&C) -> formal::CurveSchema + Parallelizable,
) -> (EdgeSampleLedgerSet, MeshedShellOutcome)
where
    C: PolylineableCurve,
    S: RobustMeshableSurface,
{
    // Step 1: the production path, called once, UNCHANGED. Delegated through
    // the existing trait; `cshell_tessellation_inner` is never re-entered by
    // hand.
    let outcome =
        shell.robust_triangulation_with_schema_outcome(tol, lattice_of, schema_of, curve_schema_of);
    // Step 2: the ledger, built independently of the outcome. Every unique
    // compressed edge is sampled with the SAME parameter division
    // `PolylineCurve::from_curve` performs — `curve.parameter_division(range,
    // tol)` over the same `curve.range_tuple()` — so each recorded position is
    // exactly the position the production path samples at that parameter.
    let ledger = build_ledger(shell, tol);
    // Step 3.
    (ledger, outcome)
}

/// Build the edge-sample ledger for one shell.
///
/// The division is the production one: `curve.parameter_division(curve.range_tuple(),
/// tol)`, i.e. the same call `PolylineCurve::from_curve` makes, keeping the
/// parameters as well as the points. Positions are interned into the shared
/// table by exact equality (f64 `==` on all three components).
fn build_ledger<C, S>(shell: &CompressedShell<Point3, C, S>, tol: f64) -> EdgeSampleLedgerSet
where
    C: PolylineableCurve,
{
    // Interning is LOOKUP-ONLY: the `HashMap` maps a position to the index
    // already pushed into `positions`. The key is the IEEE-754 bit pattern of
    // the three components (cgmath's `Point3` implements neither `Hash` nor
    // `Eq`), with `-0.0` normalised to `+0.0` so the key equality is exactly
    // the convention's f64 `==` on all three components. Output order never
    // derives from map iteration — the table is filled in first-appearance
    // order by the fixed iteration below (edge index ascending, parameters
    // ascending), so the whole ledger is deterministic.
    let mut index_of: std::collections::HashMap<[u64; 3], usize> = std::collections::HashMap::new();
    let mut positions: Vec<Point3> = Vec::new();
    let mut entries = Vec::with_capacity(shell.edges.len());
    for (edge_index, edge) in shell.edges.iter().enumerate() {
        let range = edge.curve.range_tuple();
        let (parameters, points) = edge.curve.parameter_division(range, tol);
        let mut position_indices = Vec::with_capacity(points.len());
        for point in points {
            let key = [
                if point.x == 0.0 { 0.0 } else { point.x }.to_bits(),
                if point.y == 0.0 { 0.0 } else { point.y }.to_bits(),
                if point.z == 0.0 { 0.0 } else { point.z }.to_bits(),
            ];
            let position_index = *index_of.entry(key).or_insert_with(|| {
                let index = positions.len();
                positions.push(point);
                index
            });
            position_indices.push(position_index);
        }
        entries.push(EdgeSampleLedger {
            edge: edge_index,
            parameters,
            position_indices,
        });
    }
    EdgeSampleLedgerSet { entries, positions }
}
