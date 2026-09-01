//! Surface Quotient Schemas, Deck Lattices, and Singular Strata.
//!
//! Provides the core abstraction connecting geometric surface primitives to their
//! topological quotient space structure.

use cgmath::{Point2, Point3, Vector2};

/// Rank 0, Rank 1, or Rank 2 lattice acting on the universal cover ℝ².
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DeckLattice {
    /// Non-periodic planar domain (e.g. Plane, non-periodic Spline).
    Rank0,
    /// Cylindrical/Conical/Spherical domain (1 periodic axis, generator vector).
    Rank1 {
        /// Generator displacement vector.
        generator: Vector2<f64>,
    },
    /// Toroidal domain (2 periodic axes, u and v generators).
    Rank2 {
        /// Periodic generator along u.
        u_generator: Vector2<f64>,
        /// Periodic generator along v.
        v_generator: Vector2<f64>,
    },
}

impl DeckLattice {
    /// Returns the lattice rank (0, 1, or 2).
    pub fn rank(&self) -> usize {
        match self {
            Self::Rank0 => 0,
            Self::Rank1 { .. } => 1,
            Self::Rank2 { .. } => 2,
        }
    }
}

/// Unique identifier for a singular parameter stratum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct StratumId(pub usize);

/// Proof certificate confirming a geometric stratum collapse (e.g. cone apex or sphere pole).
#[derive(Debug, Clone, PartialEq)]
pub enum StratumCertificate {
    /// Exact analytical vector collapse: W(u_apex) = 0 with residual proof <= tol.
    AnalyticLinearRevolution {
        /// Certified apex parameter u.
        apex_u: f64,
        /// Maximum residual magnitude.
        residual: f64,
    },
    /// Projection of source VERTEX_LOOP 3D point.
    ExplicitProjectedVertex {
        /// 3D vertex position.
        point3: Point3<f64>,
        /// Projection residual.
        residual: f64,
    },
    /// Certified 1D/2D numerical root search.
    CertifiedRootSearch {
        /// Parameter coordinate.
        parameter: Point2<f64>,
        /// Residual magnitude.
        residual: f64,
    },
    /// Native primitive boundary endpoint.
    AuthoritativeNativeEndpoint {
        /// Endpoint parameter.
        endpoint: f64,
    },
}

/// A parameter stratum that geometrically collapses to a lower-dimensional set in 3D.
#[derive(Debug, Clone, PartialEq)]
pub struct SingularStratum {
    /// Stratum ID.
    pub id: StratumId,
    /// 3D image point.
    pub image: Point3<f64>,
    /// Stratum collapse proof certificate.
    pub certificate: StratumCertificate,
}

/// The complete topological schema for a parametric surface.
#[derive(Debug, Clone, PartialEq)]
pub struct ParametricQuotient {
    /// Deck lattice of periodicities.
    pub lattice: DeckLattice,
    /// Collapsed parameter strata.
    pub singular_strata: Vec<SingularStratum>,
    /// Fundamental period along u, if any.
    pub u_period: Option<f64>,
    /// Fundamental period along v, if any.
    pub v_period: Option<f64>,
}

/// Reasons why a surface quotient schema could not be established.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchemaFailure {
    /// Unsupported or unhandled surface primitive.
    UnsupportedSurfaceType,
    /// Singularity could not be certified within tolerance.
    SingularityInconclusive,
}

/// Interface that every pre-meshable surface implements to expose its quotient schema.
pub trait ParametricQuotientSurface {
    /// Computes the parametric quotient schema for the surface.
    fn quotient_schema(&self, tol: f64) -> Result<ParametricQuotient, SchemaFailure>;
    /// Evaluates the 3D surface point from cover parameter UV.
    fn evaluate_cover(&self, uv: Point2<f64>) -> Point3<f64>;
}
