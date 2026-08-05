pub mod cache;
pub mod camera;
pub mod cli;
pub mod config;
pub mod ocrt;
pub mod output;
pub mod renderer;
pub mod scene;
pub mod server;
pub mod step;
pub mod timing;
pub mod ui;

pub use step::lattice::curve_schema_of as step_curve_schema_of;
/// The deck lattice of a STEP surface, read from its representation.
///
/// Re-exported so the census exercises exactly the path production does.
pub use step::lattice::lattice_of as step_lattice_of;
pub use step::lattice::support_schema_of as step_support_schema_of;

/// The rank-1 cylinder evidence readers, re-exported so the census exercises
/// exactly the path production does (see the plain re-exports above).
pub use step::cone::identify_source_cone_opt as step_cone_of;
pub use step::cylinder::identify_source_cylinder_opt as step_cylinder_of;
pub use step::lattice::cylinder_curve_family_of as step_cylinder_curve_family_of;
pub use step::lattice::cylinder_curve_schema_of as step_cylinder_curve_schema_of;
