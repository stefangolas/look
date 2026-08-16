//! Quotient Region Complex Representation.
//!
//! Represents the 2-manifold region after welding periodic deck orbits ((u,v) ~ (u,v+2πk))
//! and attaching singular collapse strata.

use super::schema::StratumId;
use crate::cgmath::{Point2, Point3};

/// Unique identifier for a region vertex in the quotient complex.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RegionVertexId(pub usize);

/// Region vertex classification (Regular parameter point vs Singular stratum).
#[derive(Debug, Clone, PartialEq)]
pub enum RegionVertex {
    /// Regular surface point.
    Regular {
        /// Representative parameter point in cover space.
        representative: Point2<f64>,
        /// Orbit ID under periodic deck lattice translations.
        deck_orbit_id: usize,
    },
    /// Singular stratum attachment point (apex/pole).
    Singular {
        /// Associated singular stratum ID.
        stratum: StratumId,
        /// 3D image point.
        image_point3: Point3<f64>,
    },
}

/// Boundary edge connecting two region vertices.
#[derive(Debug, Clone, PartialEq)]
pub struct RegionEdge {
    /// Start vertex ID.
    pub start: RegionVertexId,
    /// End vertex ID.
    pub end: RegionVertexId,
    /// Sampled UV parameter polyline points.
    pub samples: Vec<Point2<f64>>,
}

/// Boundary cycle in the quotient complex.
#[derive(Debug, Clone, PartialEq)]
pub struct BoundaryCycle {
    /// Sequence of boundary region edges.
    pub edges: Vec<RegionEdge>,
    /// Integer essential periodic winding count.
    pub essential_winding: i64,
    /// Whether the cycle is contractible to a point in the quotient domain.
    pub is_contractible: bool,
}

/// Attachment of a singular stratum to a parameter coordinate.
#[derive(Debug, Clone, PartialEq)]
pub struct StratumAttachment {
    /// Associated singular stratum ID.
    pub stratum: StratumId,
    /// Apex/pole parameter u.
    pub apex_u: f64,
}

/// Complete quotient region complex representation.
#[derive(Debug, Clone, PartialEq)]
pub struct QuotientRegionComplex {
    /// Vertices in the quotient complex.
    pub vertices: Vec<RegionVertex>,
    /// Edges in the quotient complex.
    pub edges: Vec<RegionEdge>,
    /// Boundary cycles.
    pub boundary_cycles: Vec<BoundaryCycle>,
    /// Attached singular strata.
    pub singular_attachments: Vec<StratumAttachment>,
}

impl QuotientRegionComplex {
    /// Computes the Euler characteristic χ = V - E + F of the quotient complex.
    pub fn euler_characteristic(&self) -> i32 {
        let v = self.vertices.len() as i32;
        let e = self.edges.len() as i32;
        let f = 1; // Single connected face region
        v - e + f
    }
}
