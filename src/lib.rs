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

/// The deck lattice of a STEP surface, read from its representation.
///
/// Re-exported so the census exercises exactly the path production does.
pub use step::lattice::lattice_of as step_lattice_of;
pub use step::lattice::support_schema_of as step_support_schema_of;
