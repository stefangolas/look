//! The meshability surface traits, lifted out of `truck-meshalgo`'s
//! tessellation module: `PreMeshableSurface`, `MeshableSurface`, and the
//! cfg'd `Parallelizable` shim they bound on.

use truck_geometry::prelude::*;

#[cfg(not(target_arch = "wasm32"))]
mod parallelizable {
    /// Parallelizable by `rayon`.
    pub trait Parallelizable: Send + Sync {}
    impl<T: Send + Sync> Parallelizable for T {}
}

#[cfg(target_arch = "wasm32")]
mod parallelizable {
    /// No parallelization in the case of wasm.
    pub trait Parallelizable {}
    impl<T> Parallelizable for T {}
}

pub use parallelizable::*;

/// It can be meshed, but not necessarily trimmed.
pub trait PreMeshableSurface: ParametricSurface3D + ParameterDivision2D + Parallelizable {}
impl<S: ParametricSurface3D + ParameterDivision2D + Parallelizable> PreMeshableSurface for S {}

/// The generated mesh can be trimmed only if the boundary curves ride strictly on a surface.
pub trait MeshableSurface: PreMeshableSurface + SearchParameter<D2, Point = Point3> {}
impl<S: PreMeshableSurface + SearchParameter<D2, Point = Point3>> MeshableSurface for S {}
