//! Cut-Open Domain Plan & Triangulation Execution.
//!
//! Generates a planar patch representation via spanning-tree / cotree graph cut-open,
//! specifies periodic weld pairs and singular vertex collapse groups, and yields a certified plan.

use super::canonical::{AtlasCellId, CanonicalRegionKey};
use crate::cgmath::Point2;

/// Constraint segment for 2D CDT planar patch triangulation.
#[derive(Debug, Clone, PartialEq)]
pub struct PlanarEdgeConstraint {
    /// Start parameter point.
    pub start: Point2<f64>,
    /// End parameter point.
    pub end: Point2<f64>,
    /// Polyline samples.
    pub polyline: Vec<Point2<f64>>,
}

/// Periodic edge weld pair.
#[derive(Debug, Clone, PartialEq)]
pub struct PeriodicWeldPair {
    /// Side A parameter polyline.
    pub side_a: Vec<Point2<f64>>,
    /// Side B parameter polyline.
    pub side_b: Vec<Point2<f64>>,
}

/// Singular vertex collapse group.
#[derive(Debug, Clone, PartialEq)]
pub struct SingularCollapseGroup {
    /// Parameter points to weld into one 3D vertex.
    pub points: Vec<Point2<f64>>,
    /// Apex u parameter.
    pub apex_u: f64,
}

/// Planar cut-open execution plan.
#[derive(Debug, Clone, PartialEq)]
pub struct CutOpenPlan {
    /// Outer boundary polyline in cover space.
    pub outer_boundary: Vec<Point2<f64>>,
    /// Interior hole polylines.
    pub interior_holes: Vec<Vec<Point2<f64>>>,
    /// Periodic weld pairs.
    pub periodic_welds: Vec<PeriodicWeldPair>,
    /// Singular collapse groups.
    pub singular_collapses: Vec<SingularCollapseGroup>,
}

/// Certification of domain plan correctness.
#[derive(Debug, Clone, PartialEq)]
pub struct DomainCertificate {
    /// Whether Euler characteristic is preserved.
    pub euler_characteristic_preserved: bool,
    /// Maximum projection residual error.
    pub max_projection_residual: f64,
}

/// Complete certified domain plan.
#[derive(Debug, Clone, PartialEq)]
pub struct CertifiedDomainPlan {
    /// Canonical atlas cell label.
    pub atlas_cell: AtlasCellId,
    /// Canonical region key.
    pub key: CanonicalRegionKey,
    /// Cut open plan.
    pub cut_plan: CutOpenPlan,
    /// Certificate.
    pub certificate: DomainCertificate,
}
