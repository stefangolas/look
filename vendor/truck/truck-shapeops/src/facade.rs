//! BG-CAD-P8-FACADE — the build123d-shaped facade over the landed kernel
//! entries (plan §1: a NAMING + SEMANTICS table, zero geometric content).
//!
//! Every operation below composes landed primitives (the P1-P7 entries and
//! the P10 fold family) or refuses with the typed [`Refusal`] the landed entry
//! answers — no new solver mathematics, no restricted alternative names, no
//! silent fallbacks (D4/D5). The enum wrappers ([`Mode`], [`BlendSpec`]) are
//! naming, not geometry: `Mode` maps onto the landed [`BoolOp`] variants and
//! every [`BlendSpec`] dispatches verbatim onto the landed `rewrite` entries.
//! Python selectors are NOT part of the facade (booked with the pyo3 program,
//! plan §1).
//!
//! The naming table (every entry adapts to the landed signature; the
//! adaptations are recorded in the packet's RESULT notes):
//!
//! | facade entry | landed composition |
//! |--------------|--------------------|
//! | `extrude` | `truck_modeling::extrude::extrude_profile` |
//! | `extrude_vector` | `truck_modeling::extrude::extrude_profile_vector` |
//! | `revolve` | `truck_modeling::revolve::revolve_profile` |
//! | `fillet` | `rewrite::fillet` + `rewrite::fillet_circle` (grouped `BlendSpec` batches) |
//! | `chamfer` | `rewrite::chamfer` |
//! | `mirror` | `truck_modeling::cad::mirror_solid` |
//! | `mirror_about_plane` | `truck_modeling::cad::mirror_about_plane` |
//! | `rotate` | `truck_modeling::cad::rotate_solid` |
//! | `scale` | `truck_modeling::cad::uniform_scale_solid` |
//! | `translate` | `truck_modeling::cad::translate_solid` |
//! | `section` | `section::section_faces` |
//! | `split` | `section::split_by_plane` |
//! | `bounding_box` | `truck_modeling::cad::solid_bounding_box` |
//! | `boolean_op` | `boolean::assemble::boolean` via [`Mode`] → [`BoolOp`] |
//! | `make_face` | `truck_modeling::cad::make_face` |
//! | `make_hull` | `truck_modeling::cad::make_hull` |

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

use truck_base::bounding_box::BoundingBox;
use truck_base::cgmath64::{Point3, Vector3};
use truck_base::evidence::{
    Budget, Certificate, Certified, Margin, Method, Modulus, Outcome, PropMap, Refusal,
};
use truck_geometry::arrange::Arrangement;
use truck_geometry::canonical::{Curve, Surface};
use truck_geometry::specifieds::Plane;
use truck_topology::{Face, Solid};

use crate::boolean::assemble::boolean;
use crate::boolean::BoolOp;
use crate::rewrite::{
    chamfer as landed_chamfer, fillet as landed_fillet, fillet_circle as landed_fillet_circle,
    ChamferSpec, CircleFilletSpec, FilletSpec,
};
use crate::section::{section_faces, split_by_plane};
use truck_modeling::cad::{
    make_face as landed_make_face, make_hull as landed_make_hull,
    mirror_about_plane as landed_mirror_about_plane, mirror_solid, rotate_solid,
    solid_bounding_box, translate_solid, uniform_scale_solid,
};
use truck_modeling::extrude::{extrude_profile, extrude_profile_vector};
use truck_modeling::revolve::revolve_profile;

/// The build123d workplane boolean modes, mapped onto the landed [`BoolOp`]
/// variants (Add → `Union`, Subtract → `Difference`, Intersect →
/// `Intersection`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Union.
    Add,
    /// Difference: the first solid minus the second.
    Subtract,
    /// Intersection.
    Intersect,
}

impl Mode {
    /// The landed [`BoolOp`] this mode dispatches onto.
    fn bool_op(self) -> BoolOp {
        match self {
            Mode::Add => BoolOp::Union,
            Mode::Subtract => BoolOp::Difference,
            Mode::Intersect => BoolOp::Intersection,
        }
    }
}

/// One fillet request: a plane-plane edge fillet ([`FilletSpec`]) or a
/// circular-rim fillet ([`CircleFilletSpec`]).
#[derive(Clone, Copy, Debug)]
pub enum BlendSpec {
    /// A straight (plane-plane) edge fillet.
    Straight(FilletSpec),
    /// A circular-rim (Torus) fillet.
    Circular(CircleFilletSpec),
}

/// Extrudes the material region of a planar arrangement by `height` along +z.
pub fn extrude(
    profile: &[Curve],
    arrangement: &Arrangement,
    height: f64,
) -> Outcome<Solid<Point3, Curve, Surface>> {
    extrude_profile(profile, arrangement, height)
}

/// Extrudes the material region of a planar arrangement along `dir`
/// (`both == true` spans `[-dir, +dir]`).
pub fn extrude_vector(
    profile: &[Curve],
    arrangement: &Arrangement,
    dir: Vector3,
    both: bool,
) -> Outcome<Solid<Point3, Curve, Surface>> {
    extrude_profile_vector(profile, arrangement, dir, both)
}

/// Revolves the material region of a planar arrangement by `angle` about the
/// z-axis.
pub fn revolve(
    profile: &[Curve],
    arrangement: &Arrangement,
    angle: f64,
) -> Outcome<Solid<Point3, Curve, Surface>> {
    revolve_profile(profile, arrangement, angle)
}

/// Fillets the straight and circular specs of a mixed request list.
///
/// The dispatch is SEQUENTIAL per the P12 D4 rule: the list is split into the
/// `Straight` group and the `Circular` group, each processed by one landed
/// entry call, the `Straight` group first and then the `Circular` group on
/// the result. An empty list refuses `Refusal::Empty` exactly as the landed
/// entries do.
pub fn fillet(
    solid: &Solid<Point3, Curve, Surface>,
    specs: &[BlendSpec],
    budget: &mut Budget,
) -> Outcome<Solid<Point3, Curve, Surface>> {
    if specs.is_empty() {
        return Err(Refusal::Empty);
    }
    let straight: Vec<FilletSpec> = specs
        .iter()
        .filter_map(|s| match s {
            BlendSpec::Straight(spec) => Some(*spec),
            BlendSpec::Circular(_) => None,
        })
        .collect();
    let circular: Vec<CircleFilletSpec> = specs
        .iter()
        .filter_map(|s| match s {
            BlendSpec::Straight(_) => None,
            BlendSpec::Circular(spec) => Some(*spec),
        })
        .collect();
    let mut current = solid.clone();
    let mut cert = blank_certificate(*budget);
    if !straight.is_empty() {
        let done = landed_fillet(&current, &straight, budget)?;
        current = done.value;
        cert = done.cert;
    }
    if !circular.is_empty() {
        let done = landed_fillet_circle(&current, &circular, budget)?;
        current = done.value;
        cert = done.cert;
    }
    Ok(Certified::new(current, cert))
}

/// Chamfers the straight edges named by the spec list.
pub fn chamfer(
    solid: &Solid<Point3, Curve, Surface>,
    specs: &[ChamferSpec],
    budget: &mut Budget,
) -> Outcome<Solid<Point3, Curve, Surface>> {
    landed_chamfer(solid, specs, budget)
}

/// Mirrors `solid` across an axis-aligned `plane`.
pub fn mirror(
    solid: &Solid<Point3, Curve, Surface>,
    plane: &Plane,
) -> Outcome<Solid<Point3, Curve, Surface>> {
    mirror_solid(solid, plane)
}

/// Mirrors `solid` about the plane through `plane_point` with `plane_normal`.
pub fn mirror_about_plane(
    solid: &Solid<Point3, Curve, Surface>,
    plane_point: Point3,
    plane_normal: Vector3,
) -> Outcome<Solid<Point3, Curve, Surface>> {
    landed_mirror_about_plane(solid, plane_point, plane_normal)
}

/// Rotates `solid` about the axis through `axis_point` with `axis_dir` by
/// `angle` radians.
pub fn rotate(
    solid: &Solid<Point3, Curve, Surface>,
    axis_point: Point3,
    axis_dir: Vector3,
    angle: f64,
) -> Outcome<Solid<Point3, Curve, Surface>> {
    rotate_solid(solid, axis_point, axis_dir, angle)
}

/// Uniformly scales `solid` about the origin by `factor`.
pub fn scale(
    solid: &Solid<Point3, Curve, Surface>,
    factor: f64,
) -> Outcome<Solid<Point3, Curve, Surface>> {
    uniform_scale_solid(solid, factor)
}

/// Translates `solid` by `t`.
pub fn translate(
    solid: &Solid<Point3, Curve, Surface>,
    t: Vector3,
) -> Outcome<Solid<Point3, Curve, Surface>> {
    translate_solid(solid, t)
}

/// The section faces of `solid` cut by `plane`.
pub fn section(
    solid: &Solid<Point3, Curve, Surface>,
    plane: &Plane,
    budget: &mut Budget,
) -> Outcome<Vec<Face<Point3, Curve, Surface>>> {
    section_faces(solid, plane, budget)
}

/// Splits `solid` by `plane` into the `(plus, minus)` halves.
pub fn split(
    solid: &Solid<Point3, Curve, Surface>,
    plane: &Plane,
    budget: &mut Budget,
) -> Outcome<SplitHalves> {
    split_by_plane(solid, plane, budget)
}

/// The `(plus, minus)` halves of a split by plane (the landed
/// `split_by_plane` return shape; the alias keeps the facade signature under
/// the type-complexity lint).
pub type SplitHalves = (Solid<Point3, Curve, Surface>, Solid<Point3, Curve, Surface>);

/// The certified axis-aligned bounding box of `solid`.
pub fn bounding_box(
    solid: &Solid<Point3, Curve, Surface>,
    budget: &mut Budget,
) -> Outcome<BoundingBox<Point3>> {
    solid_bounding_box(solid, budget)
}

/// The regularized boolean of `a` and `b` under `mode`.
pub fn boolean_op(
    a: &Solid<Point3, Curve, Surface>,
    mode: Mode,
    b: &Solid<Point3, Curve, Surface>,
    budget: &mut Budget,
) -> Outcome<Solid<Point3, Curve, Surface>> {
    boolean(a, mode.bool_op(), b, budget)
}

/// Constructs planar faces on the z = 0 plane from a profile.
pub fn make_face(profile: &[Curve]) -> Outcome<Vec<Face<Point3, Curve, Surface>>> {
    landed_make_face(profile)
}

/// The 2-D convex hull of z = 0 points, as one planar face.
pub fn make_hull(points: &[Point3]) -> Outcome<Face<Point3, Curve, Surface>> {
    landed_make_hull(points)
}

/// The fallback certificate for a multi-entry dispatch: the facade forwards
/// the landed entries' certificates untouched, so the certificate structure is
/// float arithmetic and claims nothing. The fallback is only observed when a
/// facade dispatch performs no landed call (unreachable: the empty list
/// refuses before this).
fn blank_certificate(spent: Budget) -> Certificate {
    Certificate {
        props: PropMap::new(),
        method: Method::Float,
        budget_left: spent,
        margin: Margin::UNBOUNDED,
        modulus: Modulus::Unbounded,
    }
}
