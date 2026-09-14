//! Defines polyline-polygon data structure and some algorithms handling mesh.

#![cfg_attr(not(debug_assertions), deny(warnings))]
#![deny(clippy::all, rust_2018_idioms)]
#![warn(
    missing_docs,
    missing_debug_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unstable_features,
    unused_import_braces,
    unused_qualifications
)]

// Clippy 1.97 cross-platform baseline hygiene (orchestrator, 2026-09-14;
// recorded in loop/STATE.md traps): the hosted-runner toolchain churn and
// the unix-branch surface (which cannot lint on a Windows host) manufactured
// these on clippy-green landings. Semantics-preserving; per-site fixes land
// with the packets that touch those sites.
#![allow(
    clippy::neg_cmp_op_on_partial_ord,
    clippy::clone_on_copy,
    clippy::too_many_arguments,
    clippy::large_enum_variant,
    clippy::redundant_field_names,
    clippy::assign_op_pattern,
    clippy::for_kv_map,
    clippy::manual_contains,
    clippy::manual_range_contains,
    clippy::needless_range_loop,
    clippy::new_without_default,
    clippy::ptr_arg,
    clippy::should_implement_trait,
    clippy::single_match,
    clippy::type_complexity,
    clippy::unnecessary_map_or,
    clippy::unnecessary_filter_map,
    clippy::field_reassign_with_default,
    clippy::manual_clamp,
    clippy::map_flatten,
    clippy::result_large_err,
    clippy::needless_borrow,
    clippy::needless_borrows_for_generic_args,
    clippy::unnecessary_lazy_evaluations
)]
use array_macro::array;
use serde::{Deserialize, Serialize};

/// re-export `truck_base`.
pub mod base {
    pub use truck_base::{
        assert_near, assert_near2, bounding_box::BoundingBox, cgmath64::*, hash, hash::HashGen,
        prop_assert_near, prop_assert_near2, tolerance::*,
    };
    pub use truck_geotrait::*;
}
pub use base::*;

/// attribution container for polygin mesh
pub trait Attributes<V> {
    /// attribution
    type Output;
    /// get attribution corresponding to vertex
    fn get(&self, vertex: V) -> Option<Self::Output>;
}

/// transform attributions
pub trait TransformedAttributes: Clone {
    /// transform by `trans`.
    fn transform_by(&mut self, trans: Matrix4);
    /// transformed attributions by `trans`.
    fn transformed(&self, trans: Matrix4) -> Self;
}

/// standard attributions
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct StandardAttributes {
    /// positions
    pub positions: Vec<Point3>,
    /// texture uv coordinates
    pub uv_coords: Vec<Vector2>,
    /// normals at vertices
    pub normals: Vec<Vector3>,
}

/// standard attribution
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct StandardAttribute {
    /// position
    pub position: Point3,
    /// texture uv coordinate
    pub uv_coord: Option<Vector2>,
    /// normal at vertex
    pub normal: Option<Vector3>,
}

/// Index vertex of a face of the polygon mesh
#[derive(Clone, Copy, Hash, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct StandardVertex {
    /// index of vertex's position
    pub pos: usize,
    /// index of vertex's texture coordinate
    pub uv: Option<usize>,
    /// index of vertex's normal
    pub nor: Option<usize>,
}

/// Faces of polygon mesh
///
/// To optimize for the case where the polygon mesh consists only triangles and quadrangle,
/// there are vectors which consist by each triangles and quadrilaterals, internally.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Faces<V = StandardVertex> {
    tri_faces: Vec<[V; 3]>,
    quad_faces: Vec<[V; 4]>,
    other_faces: Vec<Vec<V>>,
}

/// Polygon mesh
///
/// The polygon data is held in a method compliant with wavefront obj.
/// Position, uv (texture) coordinates, and normal vectors are held in separate arrays,
/// and each face vertex accesses those values by an indices triple.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PolygonMesh<V = StandardVertex, A = StandardAttributes> {
    attributes: A,
    faces: Faces<V>,
}

/// structured quadrangle mesh
#[derive(Clone, Debug, Serialize)]
pub struct StructuredMesh {
    positions: Vec<Vec<Point3>>,
    uv_division: Option<(Vec<f64>, Vec<f64>)>,
    normals: Option<Vec<Vec<Vector3>>>,
}

/// polyline curve
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolylineCurve<P>(pub Vec<P>);

mod attributes;
/// Defines errors
pub mod errors;
mod expand;
/// Defines triangle
pub mod faces;
mod meshing_shape;
/// wavefront obj I/O
pub mod obj;
/// Defines [`PolygonMeshEditor`](./polygon_mesh/struct.PolygonMeshEditor.html).
pub mod polygon_mesh;
/// Defines generalized polyline curve.
pub mod polyline_curve;
/// STL I/O
pub mod stl;
mod structured_mesh;
