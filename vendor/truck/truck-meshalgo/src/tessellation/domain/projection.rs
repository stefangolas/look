//! Projection & Continuous Parameter Lifting Engine.
//!
//! Projects 3D boundary curves onto the surface universal cover ℝ² and continuously
//! unwraps periodic coordinates across seam boundaries.

use crate::cgmath::{MetricSpace, Point2, Point3, Vector3};
use crate::tessellation::domain::deck::LatticePotential;
use crate::tessellation::{MeshableSurface, PreMeshableSurface};
use truck_geometry::prelude::*;

/// 3D world-space position paired with its surface 2D parameter coordinate.
#[derive(Debug, Clone, PartialEq)]
pub struct SurfacePoint {
    /// 3D position in world space.
    pub point: Point3<f64>,
    /// 2D parameter coordinate in cover space.
    pub uv: Point2<f64>,
}

/// Traversal classification of a boundary edge curve.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TraversalSemantics {
    /// True degenerate single 3D point (arclength ≈ 0).
    DegeneratePoint,
    /// Ordinary parameter interval [t0, t1].
    Ordinary {
        /// Start curve parameter t0.
        start: f64,
        /// End curve parameter t1.
        end: f64,
    },
    /// One full periodic 360° traversal (t0 == t1 mod period, winding = ±1).
    FullPeriod {
        /// Start curve parameter.
        start: f64,
        /// Period length (e.g. 2π).
        period: f64,
        /// Essential winding (+1 or -1).
        winding: i64,
    },
    /// Multi-period traversal (winding = ±k).
    MultiPeriod {
        /// Start curve parameter.
        start: f64,
        /// Period length.
        period: f64,
        /// Winding count k.
        winding: i64,
    },
    /// Traversal resolution unresolved.
    Unresolved,
}

impl TraversalSemantics {
    /// Resolves traversal semantics for a 3D curve on a surface.
    pub fn resolve<C, S>(curve: &C, surface: &S, _tol: f64) -> Self
    where
        C: BoundedCurve + ParametricCurve3D<Point = Point3<f64>, Vector = Vector3<f64>>,
        S: PreMeshableSurface,
    {
        let (t0, t1) = curve.range_tuple();
        let p0 = curve.subs(t0);
        let p1 = curve.subs(t1);
        let dist = p0.distance(p1);
        let dt = (t1 - t0).abs();

        // Determine curve period from surface periodicity
        let u_period = surface.u_period();
        let v_period = surface.v_period();
        let period = v_period.or(u_period).unwrap_or(2.0 * std::f64::consts::PI);

        // When t0 == t1 (coincident STEP trim params), the usual midpoint probe gives span = 0.
        // For a periodic full-circle edge this is expected: use half-period as the probe instead.
        let t_mid = if dt < 1e-6 {
            t0 + 0.5 * period
        } else {
            t0 + 0.5 * dt
        };
        let p_mid = curve.subs(t_mid);
        let span = p0.distance(p_mid) + p_mid.distance(p1);

        // True degenerate: no arc length even with half-period probe
        if span < 1e-5 {
            return Self::DegeneratePoint;
        }

        // Coincident trim endpoints (dt ≈ 0 or dt ≈ period) with nonzero arc → full-period traversal
        if dist < 1e-4 && (dt < 1e-6 || (dt - period).abs() < 1e-3) && span > 1e-4 {
            return Self::FullPeriod {
                start: t0,
                period,
                winding: 1,
            };
        }

        Self::Ordinary { start: t0, end: t1 }
    }
}

/// Unique ID for a lifted vertex in cover space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LiftedVertexId(pub usize);

/// Lifted vertex record.
#[derive(Debug, Clone, PartialEq)]
pub struct LiftedVertex {
    /// Vertex ID.
    pub id: LiftedVertexId,
    /// 2D parameter coordinate.
    pub uv: Point2<f64>,
}

/// Projection proof certificate.
#[derive(Debug, Clone, PartialEq)]
pub struct ProjectionCertificate {
    /// Maximum residual error from projected UV back to 3D point.
    pub max_residual: f64,
    /// Number of sample points evaluated.
    pub samples_evaluated: usize,
}

/// Lifted half-edge record.
#[derive(Debug, Clone, PartialEq)]
pub struct LiftedHalfEdge {
    /// Source edge entity ID, if known.
    pub source_edge_id: Option<usize>,
    /// Start lifted vertex ID.
    pub start: LiftedVertexId,
    /// End lifted vertex ID.
    pub end: LiftedVertexId,
    /// Polyline samples in cover space ℝ².
    pub samples: Vec<Point2<f64>>,
    /// Deck displacement vector (m, n) ∈ ℤ².
    pub deck_delta: (i64, i64),
    /// Projection certificate.
    pub certificate: ProjectionCertificate,
}

/// Lifted boundary complex.
#[derive(Debug, Clone, PartialEq)]
pub struct LiftedBoundaryComplex {
    /// Vertices in universal cover space.
    pub vertices: Vec<LiftedVertex>,
    /// Half-edges in universal cover space.
    pub half_edges: Vec<LiftedHalfEdge>,
}

impl LiftedBoundaryComplex {
    /// Creates a new empty lifted boundary complex.
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            half_edges: Vec::new(),
        }
    }
}

/// Failure modes for shared boundary projection.
#[derive(Debug, Clone, PartialEq)]
pub enum BoundaryProjectionFailure {
    /// Projection did not converge within tolerance.
    Diverged {
        /// Maximum residual encountered.
        max_residual: f64,
    },
    /// Surface parameter search returned None.
    SearchReturnedNone,
}

/// Full projected boundary path containing 3D world points, 2D cover UV parameters, traversal semantics, and deck displacement.
#[derive(Debug, Clone, PartialEq)]
pub struct ProjectedBoundaryPath {
    /// Sample points containing both 3D position and 2D parameter coordinate.
    pub samples: Vec<SurfacePoint>,
    /// Resolved edge traversal semantics.
    pub traversal: TraversalSemantics,
    /// Integer deck displacement.
    pub deck_displacement: LatticePotential,
    /// Maximum projection residual error.
    pub max_residual: f64,
}

/// Shared, production-identical boundary curve projection API.
pub fn project_boundary_curve<S, C>(
    curve: &C,
    surface: &S,
    traversal: TraversalSemantics,
    tol: f64,
) -> std::result::Result<ProjectedBoundaryPath, BoundaryProjectionFailure>
where
    C: BoundedCurve + ParametricCurve3D<Point = Point3<f64>, Vector = Vector3<f64>>,
    S: MeshableSurface,
{
    let mut samples_pts = Vec::new();
    let num_samples = 16;

    match traversal {
        TraversalSemantics::DegeneratePoint => {
            let p3 = curve.subs(curve.range_tuple().0);
            if let Some((u, v)) = surface.search_parameter(p3, None, 100) {
                samples_pts.push(SurfacePoint {
                    point: p3,
                    uv: Point2::new(u, v),
                });
            } else {
                return Err(BoundaryProjectionFailure::SearchReturnedNone);
            }
        }
        TraversalSemantics::FullPeriod {
            start,
            period,
            winding: _,
        } => {
            let mut hint = None;
            for i in 0..=num_samples {
                let t = start + period * (i as f64 / num_samples as f64);
                let p3 = curve.subs(t);
                if let Some((u, v)) = surface.search_parameter(p3, hint, 100) {
                    let uv = Point2::new(u, v);
                    hint = Some((u, v));
                    samples_pts.push(SurfacePoint { point: p3, uv });
                } else {
                    return Err(BoundaryProjectionFailure::SearchReturnedNone);
                }
            }
        }
        TraversalSemantics::Ordinary { start, end } => {
            let mut hint = None;
            for i in 0..=num_samples {
                let t = start + (end - start) * (i as f64 / num_samples as f64);
                let p3 = curve.subs(t);
                if let Some((u, v)) = surface.search_parameter(p3, hint, 100) {
                    let uv = Point2::new(u, v);
                    hint = Some((u, v));
                    samples_pts.push(SurfacePoint { point: p3, uv });
                } else {
                    return Err(BoundaryProjectionFailure::SearchReturnedNone);
                }
            }
        }
        _ => {
            let (t0, t1) = curve.range_tuple();
            let mut hint = None;
            for i in 0..=num_samples {
                let t = t0 + (t1 - t0) * (i as f64 / num_samples as f64);
                let p3 = curve.subs(t);
                if let Some((u, v)) = surface.search_parameter(p3, hint, 100) {
                    let uv = Point2::new(u, v);
                    hint = Some((u, v));
                    samples_pts.push(SurfacePoint { point: p3, uv });
                }
            }
        }
    }

    let mut max_res = 0.0_f64;
    for sp in &samples_pts {
        let p3_eval = surface.subs(sp.uv.x, sp.uv.y);
        max_res = max_res.max(p3_eval.distance(sp.point));
    }

    if max_res > tol && max_res > 1.0 {
        return Err(BoundaryProjectionFailure::Diverged {
            max_residual: max_res,
        });
    }

    let deck_disp = match traversal {
        TraversalSemantics::FullPeriod { winding, .. } => LatticePotential::rank1(winding),
        _ => LatticePotential::zero(),
    };

    Ok(ProjectedBoundaryPath {
        samples: samples_pts,
        traversal,
        deck_displacement: deck_disp,
        max_residual: max_res,
    })
}
