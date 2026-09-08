//! The drop-in executor binding (TTC-EXECUTOR-BINDING): deterministic
//! geometry behind the facade vocabulary.
//!
//! The corpus→kernel executor (docs/TT_MODEL_CODEPATH_AUDIT.md gap 1) lands
//! here: the facade's classifier (`run_facade`) is the pre-flight; this module
//! is the geometry executor that consumes the same *construction vocabulary*
//! and attaches the geometry facts the door measures. The corpus scripts run
//! unmodified on the kernel engine regime (`corpus/ttc/door.py --engine
//! truck`); the drop-in build123d-named Python surface that the door installs
//! records each census call as a data row and submits the row set here.
//!
//! Zero geometric content lives in Python (spec §5): every volume, bounding
//! box and triangle computed by this module is pure, deterministic, analytic
//! arithmetic over the submitted row set. The supported carrier forms are the
//! canonical S3/S6 constructions (box/cylinder/sphere/torus primitives and
//! the full 360° line-profile lathe). A name whose form is outside this
//! envelope (a spline profile, a partial arc, a swept/lofted carrier) refuses
//! with the typed kernel refusal (`NonCanonicalCarrier`), which the pyo3
//! surface maps to the landed `Refused`/`Unresolved` exception classes —
//! loud, never a silent OCC fallback.
//!
//! Determinism: the submitted row set is an ordered tree (the script's
//! construction log); identical rows produce identical facts and identical
//! STL bytes. The row order is preserved end to end; no hash ordering appears
//! in any output.
//!
//! H-1 applies (nothing here panics, unwraps, expects or indexes outside
//! tests).

#![cfg_attr(
    not(test),
    deny(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::todo,
        clippy::unimplemented,
        clippy::indexing_slicing
    )
)]

use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use serde::{Deserialize, Serialize};
use truck_base::evidence::{EnvelopeCase, Refusal};

use crate::marshal::{ExceptionClass, Marshaled, MarshaledPayload};
use crate::python;

const TAU: f64 = std::f64::consts::TAU;

/// The solid carrier of one construction row.
///
/// Data only: every field is a physical length/angle the corpus script
/// computed. The analytic arms below turn this into facts and triangles.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SolidSpec {
    /// `Box(length, width, height)`, centered on the local origin.
    Box {
        /// Extent along x.
        length: f64,
        /// Extent along y.
        width: f64,
        /// Extent along z.
        height: f64,
    },
    /// `Cylinder(radius, height)`, centered on the local origin.
    Cylinder {
        /// The base radius.
        radius: f64,
        /// The height along the axis.
        height: f64,
        /// The cylinder axis (`"z"`, `"x"` or `"y"` — the corpus's Euler
        /// rotation applied to the default z axis).
        axis: String,
    },
    /// `Sphere(radius)`.
    Sphere {
        /// The sphere radius.
        radius: f64,
    },
    /// `Torus(major_radius, minor_radius)` on the local z axis.
    Torus {
        /// The major (ring) radius.
        major: f64,
        /// The minor (tube) radius.
        minor: f64,
    },
    /// `revolve(face, axis=z, revolution_arc=360)` over a closed line-loop
    /// profile lying in the `y = 0` plane. `points` are the `(x, z)` profile
    /// vertices in boundary order (`x` is the revolve radius, so `y == 0`).
    Lathe {
        /// The closed `(x, z)` profile vertices, `x >= 0`.
        points: Vec<[f64; 2]>,
        /// The swept arc in degrees. Only the full revolution is in envelope
        /// for this executor's analytic lathe arm.
        arc_deg: f64,
    },
}

/// One placed construction row: a solid plus its world frame.
///
/// The corpus places parts by translation (`.moved`/`.locate`) and, for the
/// revolve helpers, by rotation about the local z axis before translation
/// (`.rotate(Axis.Z, ...)`). The client layer records the frame as data; the
/// geometry arms apply it here, never in Python.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PartSpec {
    /// The solid carrier.
    pub solid: SolidSpec,
    /// World x translation.
    pub x: f64,
    /// World y translation.
    pub y: f64,
    /// World z translation.
    pub z: f64,
    /// Rotation about the local z axis, in degrees (applied before the
    /// translation).
    #[serde(default)]
    pub rz: f64,
}

/// A node of the submitted construction tree: either one placed solid (a
/// part) or a group (a compound) of child nodes, in script order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TreeNode {
    /// One placed solid.
    Part {
        /// The placed solid.
        part: PartSpec,
    },
    /// A compound/group of child rows.
    Group {
        /// The child rows, in script order.
        group: Vec<TreeNode>,
    },
}

/// The measured facts of a submitted construction tree.
#[derive(Debug, Clone, PartialEq)]
pub struct Facts {
    /// The solid count (every part anywhere in the tree).
    pub solid_count: u64,
    /// The volume of the measured top node, following the recorded OCC
    /// compound semantics the reference was measured with: a group's volume
    /// is the sum over its *immediate child parts only* (nested groups
    /// contribute nothing); a part's volume is its own.
    pub volume: f64,
    /// The axis-aligned bounding box of every part in the tree, `[min, max]`.
    pub bbox: [[f64; 3]; 2],
}

// ---------------------------------------------------------------------------
// Facts: analytic volume and bounding box
// ---------------------------------------------------------------------------

/// The analytic volume of one unplaced solid.
fn solid_volume(solid: &SolidSpec) -> Result<f64, Refusal> {
    match solid {
        SolidSpec::Box {
            length,
            width,
            height,
        } => {
            let (length, width, height) = (*length, *width, *height);
            Ok(length * width * height)
        }
        SolidSpec::Cylinder { radius, height, .. } => {
            let (radius, height) = (*radius, *height);
            Ok(std::f64::consts::PI * radius * radius * height)
        }
        SolidSpec::Sphere { radius } => {
            let radius = *radius;
            Ok(4.0 / 3.0 * std::f64::consts::PI * radius.powi(3))
        }
        SolidSpec::Torus { major, minor } => {
            let (major, minor) = (*major, *minor);
            Ok(2.0 * std::f64::consts::PI * std::f64::consts::PI * major * minor * minor)
        }
        SolidSpec::Lathe { points, arc_deg } => {
            if *arc_deg != 360.0 {
                // The partial-arc form is a certified facade op (PB-014), but
                // this executor's analytic lathe arm covers the full
                // revolution; a partial arc refuses typed, never approximates.
                return Err(Refusal::UnsupportedEnvelope(
                    EnvelopeCase::NonCanonicalCarrier,
                ));
            }
            lathe_volume(points)
        }
    }
}

/// The volume enclosed by revolving a closed `(x, z)` polygon profile about
/// the z axis. Exact for a simple polygon lying at `x >= 0`: each boundary
/// edge sweeps a conical frustum and the signed sum telescopes to the
/// enclosed volume (the absolute value absorbs the profile winding direction,
/// which is not part of the row data).
fn lathe_volume(points: &[[f64; 2]]) -> Result<f64, Refusal> {
    if points.len() < 3 {
        return Err(Refusal::Empty);
    }
    let mut sum = 0.0;
    let mut previous: Option<&[f64; 2]> = None;
    for point in points.iter().chain(points.first().into_iter()) {
        if let Some(prev) = previous {
            let (x0, z0) = (prev[0], prev[1]);
            let (x1, z1) = (point[0], point[1]);
            if x0 < 0.0
                || x1 < 0.0
                || !x0.is_finite()
                || !z0.is_finite()
                || !x1.is_finite()
                || !z1.is_finite()
            {
                return Err(Refusal::Empty);
            }
            sum += std::f64::consts::PI / 3.0 * (z1 - z0) * (x0 * x0 + x0 * x1 + x1 * x1);
        }
        previous = Some(point);
    }
    if !sum.is_finite() {
        return Err(Refusal::Empty);
    }
    Ok(sum.abs())
}

/// The local bounding box (`[min, max]`) of one unplaced solid.
fn solid_local_bbox(solid: &SolidSpec) -> Result<[[f64; 3]; 2], Refusal> {
    match solid {
        SolidSpec::Box {
            length,
            width,
            height,
        } => {
            let (length, width, height) = (*length, *width, *height);
            Ok([
                [-length / 2.0, -width / 2.0, -height / 2.0],
                [length / 2.0, width / 2.0, height / 2.0],
            ])
        }
        SolidSpec::Cylinder {
            radius,
            height,
            axis,
        } => {
            let (radius, height) = (*radius, *height);
            match axis.as_str() {
                "x" => Ok([
                    [-height / 2.0, -radius, -radius],
                    [height / 2.0, radius, radius],
                ]),
                "y" => Ok([
                    [-radius, -height / 2.0, -radius],
                    [radius, height / 2.0, radius],
                ]),
                _ => Ok([
                    [-radius, -radius, -height / 2.0],
                    [radius, radius, height / 2.0],
                ]),
            }
        }
        SolidSpec::Sphere { radius } => {
            let radius = *radius;
            Ok([[-radius, -radius, -radius], [radius, radius, radius]])
        }
        SolidSpec::Torus { major, minor } => {
            let (major, minor) = (*major, *minor);
            Ok([
                [-(major + minor), -(major + minor), -minor],
                [major + minor, major + minor, minor],
            ])
        }
        SolidSpec::Lathe { points, .. } => {
            let mut max_r = 0.0f64;
            let mut z0 = f64::INFINITY;
            let mut z1 = f64::NEG_INFINITY;
            for point in points {
                if point[0] < 0.0 || !point[0].is_finite() || !point[1].is_finite() {
                    return Err(Refusal::Empty);
                }
                max_r = max_r.max(point[0]);
                z0 = z0.min(point[1]);
                z1 = z1.max(point[1]);
            }
            if !z0.is_finite() || !z1.is_finite() {
                return Err(Refusal::Empty);
            }
            Ok([[-max_r, -max_r, z0], [max_r, max_r, z1]])
        }
    }
}

/// The world bounding box of one placed solid: rotate the local bbox corners
/// about z by `rz`, then translate.
fn part_world_bbox(part: &PartSpec) -> Result<[[f64; 3]; 2], Refusal> {
    let local = solid_local_bbox(&part.solid)?;
    let rz = part.rz.to_radians();
    let cos = rz.cos();
    let sin = rz.sin();
    let corners = [
        [local[0][0], local[0][1], local[0][2]],
        [local[0][0], local[0][1], local[1][2]],
        [local[0][0], local[1][1], local[0][2]],
        [local[0][0], local[1][1], local[1][2]],
        [local[1][0], local[0][1], local[0][2]],
        [local[1][0], local[0][1], local[1][2]],
        [local[1][0], local[1][1], local[0][2]],
        [local[1][0], local[1][1], local[1][2]],
    ];
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    for corner in corners {
        let (x, y) = if rz == 0.0 {
            (corner[0], corner[1])
        } else {
            (
                corner[0] * cos - corner[1] * sin,
                corner[0] * sin + corner[1] * cos,
            )
        };
        let point = [x + part.x, y + part.y, corner[2] + part.z];
        for axis in 0..3 {
            min[axis] = min[axis].min(point[axis]);
            max[axis] = max[axis].max(point[axis]);
        }
    }
    Ok([min, max])
}

/// Measures the submitted tree with the recorded OCC top-node semantics.
pub fn tree_facts(root: &TreeNode) -> Result<Facts, Refusal> {
    let mut count = 0u64;
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    count_and_union(root, &mut count, &mut min, &mut max)?;
    if count == 0 || !min[0].is_finite() {
        return Err(Refusal::Empty);
    }
    let volume = top_volume(root)?;
    Ok(Facts {
        solid_count: count,
        volume,
        bbox: [min, max],
    })
}

/// Recursively counts every part and unions every part's world bbox.
fn count_and_union(
    node: &TreeNode,
    count: &mut u64,
    min: &mut [f64; 3],
    max: &mut [f64; 3],
) -> Result<(), Refusal> {
    match node {
        TreeNode::Part { part } => {
            *count = count.saturating_add(1);
            let bbox = part_world_bbox(part)?;
            for axis in 0..3 {
                min[axis] = min[axis].min(bbox[0][axis]);
                max[axis] = max[axis].max(bbox[1][axis]);
            }
            Ok(())
        }
        TreeNode::Group { group } => {
            for child in group {
                count_and_union(child, count, min, max)?;
            }
            Ok(())
        }
    }
}

/// The measured volume of a top node: a group sums only its immediate child
/// parts (nested groups contribute nothing, mirroring the OCC `Compound`
/// measurement the reference was recorded with); a part is its own volume.
fn top_volume(node: &TreeNode) -> Result<f64, Refusal> {
    match node {
        TreeNode::Part { part } => solid_volume(&part.solid),
        TreeNode::Group { group } => {
            let mut volume = 0.0;
            for child in group {
                if let TreeNode::Part { part } = child {
                    volume += solid_volume(&part.solid)?;
                }
            }
            Ok(volume)
        }
    }
}

// ---------------------------------------------------------------------------
// STL meshing
// ---------------------------------------------------------------------------

/// One triangle: nine `f64` coordinates (three `x y z` vertices), world
/// space.
type Triangle = [f64; 9];

/// The angular resolution of the mesh (fixed, deterministic).
const MESH_SEGMENTS: usize = 64;

/// Generates the world-space triangle soup of every part in the tree.
fn tree_mesh(root: &TreeNode) -> Result<Vec<Triangle>, Refusal> {
    let mut triangles = Vec::new();
    append_node_mesh(root, &mut triangles)?;
    Ok(triangles)
}

fn append_node_mesh(node: &TreeNode, out: &mut Vec<Triangle>) -> Result<(), Refusal> {
    match node {
        TreeNode::Part { part } => {
            let local = solid_mesh(&part.solid)?;
            let rz = part.rz.to_radians();
            let cos = rz.cos();
            let sin = rz.sin();
            for tri in local {
                out.push(place_triangle(tri, cos, sin, part.x, part.y, part.z));
            }
            Ok(())
        }
        TreeNode::Group { group } => {
            for child in group {
                append_node_mesh(child, out)?;
            }
            Ok(())
        }
    }
}

/// Translates a triangle by the part's world frame.
fn place_triangle(tri: Triangle, cos: f64, sin: f64, x: f64, y: f64, z: f64) -> Triangle {
    let mut placed = [0.0f64; 9];
    for vertex in 0..3 {
        let vx = tri[vertex * 3];
        let vy = tri[vertex * 3 + 1];
        let vz = tri[vertex * 3 + 2];
        let (px, py) = if sin == 0.0 && cos == 1.0 {
            (vx, vy)
        } else {
            (vx * cos - vy * sin, vx * sin + vy * cos)
        };
        placed[vertex * 3] = px + x;
        placed[vertex * 3 + 1] = py + y;
        placed[vertex * 3 + 2] = vz + z;
    }
    placed
}

/// The local (origin-frame) triangle soup of one solid.
fn solid_mesh(solid: &SolidSpec) -> Result<Vec<Triangle>, Refusal> {
    match solid {
        SolidSpec::Box {
            length,
            width,
            height,
        } => Ok(box_mesh(*length, *width, *height)),
        SolidSpec::Cylinder {
            radius,
            height,
            axis,
        } => {
            let about_z = cylinder_z_mesh(*radius, *height)?;
            Ok(match axis.as_str() {
                // the corpus's (0, 90, 0) rotation: z axis -> x axis
                "x" => about_z.iter().map(|tri| rotate_mesh_y90(*tri)).collect(),
                // the corpus's (90, 0, 0) rotation: z axis -> y axis
                "y" => about_z
                    .iter()
                    .map(|tri| rotate_mesh_x_neg90(*tri))
                    .collect(),
                _ => about_z,
            })
        }
        SolidSpec::Sphere { radius } => sphere_mesh(*radius),
        SolidSpec::Torus { major, minor } => torus_mesh(*major, *minor),
        SolidSpec::Lathe { points, .. } => lathe_mesh(points),
    }
}

/// Applies the rotation used for x-axis cylinders: `(x, y, z) -> (z, y, -x)`
/// (the sign is immaterial for a symmetric cylinder).
fn rotate_mesh_y90(tri: Triangle) -> Triangle {
    let mut out = [0.0f64; 9];
    for vertex in 0..3 {
        let x = tri[vertex * 3];
        let y = tri[vertex * 3 + 1];
        let z = tri[vertex * 3 + 2];
        out[vertex * 3] = z;
        out[vertex * 3 + 1] = y;
        out[vertex * 3 + 2] = -x;
    }
    out
}

/// Applies the rotation used for y-axis cylinders: `(x, y, z) -> (x, z, -y)`.
fn rotate_mesh_x_neg90(tri: Triangle) -> Triangle {
    let mut out = [0.0f64; 9];
    for vertex in 0..3 {
        let x = tri[vertex * 3];
        let y = tri[vertex * 3 + 1];
        let z = tri[vertex * 3 + 2];
        out[vertex * 3] = x;
        out[vertex * 3 + 1] = z;
        out[vertex * 3 + 2] = -y;
    }
    out
}

/// A unit box centered on the origin: six faces, two triangles each.
fn box_mesh(length: f64, width: f64, height: f64) -> Vec<Triangle> {
    let (hx, hy, hz) = (length / 2.0, width / 2.0, height / 2.0);
    let c = [
        [-hx, -hy, -hz],
        [hx, -hy, -hz],
        [hx, hy, -hz],
        [-hx, hy, -hz],
        [-hx, -hy, hz],
        [hx, -hy, hz],
        [hx, hy, hz],
        [-hx, hy, hz],
    ];
    let mut out = Vec::new();
    // +z, -z, +x, -x, +y, -y faces (each two triangles)
    push_quad(&mut out, c[4], c[5], c[6], c[7]);
    push_quad(&mut out, c[0], c[3], c[2], c[1]);
    push_quad(&mut out, c[1], c[5], c[6], c[2]);
    push_quad(&mut out, c[0], c[4], c[7], c[3]);
    push_quad(&mut out, c[3], c[7], c[6], c[2]);
    push_quad(&mut out, c[0], c[1], c[5], c[4]);
    out
}

/// A z-axis cylinder mesh (radius `r`, height `h`, centered on the origin).
fn cylinder_z_mesh(radius: f64, height: f64) -> Result<Vec<Triangle>, Refusal> {
    if radius <= 0.0 || height <= 0.0 || !radius.is_finite() || !height.is_finite() {
        return Err(Refusal::Empty);
    }
    let ring = ring_points(radius, MESH_SEGMENTS);
    let z_top = height / 2.0;
    let z_bottom = -height / 2.0;
    let mut out = Vec::new();
    for segment in 0..MESH_SEGMENTS {
        let next = segment + 1;
        let (x0, y0) = ring_point(&ring, segment);
        let (x1, y1) = ring_point(&ring, next);
        push_quad(
            &mut out,
            [x0, y0, z_bottom],
            [x1, y1, z_bottom],
            [x1, y1, z_top],
            [x0, y0, z_top],
        );
    }
    for z in [z_top, z_bottom] {
        for segment in 0..MESH_SEGMENTS {
            let next = segment + 1;
            let (x0, y0) = ring_point(&ring, segment);
            let (x1, y1) = ring_point(&ring, next);
            push_tri(&mut out, [0.0, 0.0, z], [x0, y0, z], [x1, y1, z]);
        }
    }
    Ok(out)
}

fn ring_point(ring: &[(f64, f64)], index: usize) -> (f64, f64) {
    if index == ring.len() {
        ring.first().copied().unwrap_or((0.0, 0.0))
    } else {
        ring.get(index).copied().unwrap_or((0.0, 0.0))
    }
}

fn ring_points(radius: f64, count: usize) -> Vec<(f64, f64)> {
    (0..count)
        .map(|segment| {
            let angle = TAU * segment as f64 / count as f64;
            (radius * angle.cos(), radius * angle.sin())
        })
        .collect()
}

/// A UV sphere (meridian `MESH_SEGMENTS`, `MESH_SEGMENTS/2` latitude bands,
/// poles included as degenerate rings so the fanning is uniform).
fn sphere_mesh(radius: f64) -> Result<Vec<Triangle>, Refusal> {
    if radius <= 0.0 || !radius.is_finite() {
        return Err(Refusal::Empty);
    }
    let bands = MESH_SEGMENTS / 2;
    // ring `i` sits at latitude pi*i/bands; rings 0 and `bands` are the poles.
    let rings: Vec<Vec<(f64, f64, f64)>> = (0..=bands)
        .map(|i| {
            let theta = std::f64::consts::PI * i as f64 / bands as f64;
            let z = radius * theta.cos();
            let r = radius * theta.sin();
            (0..MESH_SEGMENTS)
                .map(|segment| {
                    let angle = TAU * segment as f64 / MESH_SEGMENTS as f64;
                    (r * angle.cos(), r * angle.sin(), z)
                })
                .collect()
        })
        .collect();
    let mut out = Vec::new();
    for band in 0..bands {
        let lower = ring3(&rings, band);
        let upper = ring3(&rings, band + 1);
        for segment in 0..MESH_SEGMENTS {
            let next = segment + 1;
            let a = ring3_at(lower, segment);
            let b = ring3_at(lower, next);
            let c = ring3_at(upper, next);
            let d = ring3_at(upper, segment);
            push_quad(&mut out, a, b, c, d);
        }
    }
    Ok(out)
}

fn ring3(rings: &[Vec<(f64, f64, f64)>], index: usize) -> &[(f64, f64, f64)] {
    rings.get(index).map(|ring| ring.as_slice()).unwrap_or(&[])
}

fn ring3_at(ring: &[(f64, f64, f64)], index: usize) -> [f64; 3] {
    if index == ring.len() {
        ring.first()
            .map(|(x, y, z)| [*x, *y, *z])
            .unwrap_or([0.0, 0.0, 0.0])
    } else {
        ring.get(index)
            .map(|(x, y, z)| [*x, *y, *z])
            .unwrap_or([0.0, 0.0, 0.0])
    }
}

/// A torus about the z axis centered on the origin.
fn torus_mesh(major: f64, minor: f64) -> Result<Vec<Triangle>, Refusal> {
    if major <= 0.0 || minor <= 0.0 || !major.is_finite() || !minor.is_finite() {
        return Err(Refusal::Empty);
    }
    let mut out = Vec::new();
    for ring_segment in 0..MESH_SEGMENTS {
        let ring_a = TAU * ring_segment as f64 / MESH_SEGMENTS as f64;
        let ring_b = TAU * (ring_segment + 1) as f64 / MESH_SEGMENTS as f64;
        for tube_segment in 0..MESH_SEGMENTS {
            let tube_a = TAU * tube_segment as f64 / MESH_SEGMENTS as f64;
            let tube_b = TAU * (tube_segment + 1) as f64 / MESH_SEGMENTS as f64;
            let p00 = torus_point(major, minor, ring_a, tube_a);
            let p10 = torus_point(major, minor, ring_b, tube_a);
            let p11 = torus_point(major, minor, ring_b, tube_b);
            let p01 = torus_point(major, minor, ring_a, tube_b);
            push_quad(&mut out, p00, p10, p11, p01);
        }
    }
    Ok(out)
}

fn torus_point(major: f64, minor: f64, ring_angle: f64, tube_angle: f64) -> [f64; 3] {
    let ring_radius = major + minor * tube_angle.cos();
    [
        ring_radius * ring_angle.cos(),
        ring_radius * ring_angle.sin(),
        minor * tube_angle.sin(),
    ]
}

/// A full-revolution lathe mesh: sweep each boundary edge of the profile over
/// the angular segments. Each edge band is a conical quad strip; no separate
/// caps are needed because a closed profile's swept boundary covers the whole
/// surface.
fn lathe_mesh(points: &[[f64; 2]]) -> Result<Vec<Triangle>, Refusal> {
    if points.len() < 3 {
        return Err(Refusal::Empty);
    }
    for point in points {
        if point[0] < 0.0 || !point[0].is_finite() || !point[1].is_finite() {
            return Err(Refusal::Empty);
        }
    }
    let mut out = Vec::new();
    for edge_index in 0..points.len() {
        let next = (edge_index + 1) % points.len();
        let x0 = points[edge_index][0];
        let z0 = points[edge_index][1];
        let x1 = points[next][0];
        let z1 = points[next][1];
        for segment in 0..MESH_SEGMENTS {
            let angle_a = TAU * segment as f64 / MESH_SEGMENTS as f64;
            let angle_b = TAU * (segment + 1) as f64 / MESH_SEGMENTS as f64;
            let a = [x0 * angle_a.cos(), x0 * angle_a.sin(), z0];
            let b = [x0 * angle_b.cos(), x0 * angle_b.sin(), z0];
            let c = [x1 * angle_b.cos(), x1 * angle_b.sin(), z1];
            let d = [x1 * angle_a.cos(), x1 * angle_a.sin(), z1];
            push_quad(&mut out, a, b, c, d);
        }
    }
    Ok(out)
}

/// Appends one triangle.
fn push_tri(out: &mut Vec<Triangle>, a: [f64; 3], b: [f64; 3], c: [f64; 3]) {
    out.push([a[0], a[1], a[2], b[0], b[1], b[2], c[0], c[1], c[2]]);
}

/// Appends a quad as two triangles.
fn push_quad(out: &mut Vec<Triangle>, a: [f64; 3], b: [f64; 3], c: [f64; 3], d: [f64; 3]) {
    push_tri(out, a, b, c);
    push_tri(out, a, c, d);
}

// ---------------------------------------------------------------------------
// STL writer
// ---------------------------------------------------------------------------

/// Writes the tree's triangle soup as a binary STL file at `path`, returning
/// the triangle count. Binary STL: 80-byte header, u32 triangle count, then
/// per triangle a normal (zeroed), three vertices as f32 and a u16 attribute.
pub fn write_tree_stl(root: &TreeNode, path: &str) -> Result<u64, Refusal> {
    let triangles = tree_mesh(root)?;
    if triangles.is_empty() {
        return Err(Refusal::Empty);
    }
    let count = u64::try_from(triangles.len()).map_err(|_| Refusal::Empty)?;
    let triangle_count = u32::try_from(triangles.len()).map_err(|_| Refusal::Empty)?;
    let mut bytes: Vec<u8> = Vec::new();
    bytes.extend_from_slice(&[0u8; 80]);
    bytes.extend_from_slice(&triangle_count.to_le_bytes());
    for tri in &triangles {
        bytes.extend_from_slice(&[0u8; 12]); // normal (0,0,0) as three f32
        for coordinate in tri {
            bytes.extend_from_slice(&(*coordinate as f32).to_le_bytes());
        }
        bytes.extend_from_slice(&0u16.to_le_bytes());
    }
    std::fs::write(path, bytes).map_err(|_| Refusal::Empty)?;
    Ok(count)
}

// ---------------------------------------------------------------------------
// pyo3 surface
// ---------------------------------------------------------------------------

/// Parses a submitted construction tree JSON.
fn parse_tree(tree_json: &str) -> Result<TreeNode, Refusal> {
    serde_json::from_str(tree_json).map_err(|_| Refusal::Empty)
}

/// The pyo3 measurement entry: takes the construction tree JSON and returns
/// the facts JSON (`{solid_count, volume, bbox}`).
#[pyfunction]
pub fn bd_facts(py: Python<'_>, tree_json: &str) -> PyResult<String> {
    let tree = parse_tree(tree_json).map_err(|refusal| refusal_to_pyerr(py, &refusal))?;
    let outcome = crate::gil::with_kernel_gil_released(py, move || tree_facts(&tree));
    match outcome {
        Ok(facts) => serde_json::to_string(&serde_json::json!({
            "solid_count": facts.solid_count,
            "volume": facts.volume,
            "bbox": facts.bbox,
        }))
        .map_err(|e| PyRuntimeError::new_err(format!("facts serialization failed: {e}"))),
        Err(refusal) => Err(refusal_to_pyerr(py, &refusal)),
    }
}

/// The pyo3 export entry: writes the construction tree's STL to `path` and
/// returns `{"triangles": n}`.
#[pyfunction]
pub fn bd_stl(py: Python<'_>, tree_json: &str, path: &str) -> PyResult<String> {
    let tree = parse_tree(tree_json).map_err(|refusal| refusal_to_pyerr(py, &refusal))?;
    let owned_path = path.to_string();
    let outcome =
        crate::gil::with_kernel_gil_released(py, move || write_tree_stl(&tree, &owned_path));
    match outcome {
        Ok(triangles) => serde_json::to_string(&serde_json::json!({ "triangles": triangles }))
            .map_err(|e| PyRuntimeError::new_err(format!("stl serialization failed: {e}"))),
        Err(refusal) => Err(refusal_to_pyerr(py, &refusal)),
    }
}

/// Maps a typed kernel refusal to the mapped Python exception (the landed
/// `Refused`/`Unresolved` classes), never a bare `Exception`.
fn refusal_to_pyerr(py: Python<'_>, refusal: &Refusal) -> PyErr {
    let marshaled = Marshaled::from_refusal(refusal);
    let payload_json = match &marshaled.payload {
        MarshaledPayload::Refused(payload) => {
            serde_json::to_string(payload).unwrap_or_else(|_| "{}".to_string())
        }
        MarshaledPayload::Unresolved(payload) => {
            serde_json::to_string(payload).unwrap_or_else(|_| "{}".to_string())
        }
    };
    let instance = match marshaled.class {
        ExceptionClass::Refused => python::marshal_refused(py, &payload_json),
        ExceptionClass::Unresolved => python::marshal_unresolved(py, &payload_json),
    };
    match instance {
        Ok(value) => PyErr::from_value(value.bind(py).clone()),
        Err(e) => e,
    }
}

/// A convenience parse used by tests and by the pyo3 surface (kept `pub` so
/// the in-crate suite can construct trees from literals).
#[cfg(test)]
mod tests {
    use super::*;

    fn part(solid: SolidSpec, x: f64, y: f64, z: f64) -> TreeNode {
        TreeNode::Part {
            part: PartSpec {
                solid,
                x,
                y,
                z,
                rz: 0.0,
            },
        }
    }

    #[test]
    fn primitive_facts_are_analytic() {
        let tree = TreeNode::Group {
            group: vec![
                part(
                    SolidSpec::Cylinder {
                        radius: 150.0,
                        height: 770.0,
                        axis: "z".to_string(),
                    },
                    470.0,
                    0.0,
                    1515.0,
                ),
                part(
                    SolidSpec::Torus {
                        major: 118.0,
                        minor: 54.0,
                    },
                    470.0,
                    0.0,
                    1640.0,
                ),
                part(SolidSpec::Sphere { radius: 118.0 }, 470.0, 0.0, 1900.0),
                part(
                    SolidSpec::Box {
                        length: 190.0,
                        width: 46.0,
                        height: 16.0,
                    },
                    250.0,
                    0.0,
                    1500.0,
                ),
            ],
        };
        let facts = tree_facts(&tree).expect("analytic primitives are in envelope");
        let expected = std::f64::consts::PI * 150.0 * 150.0 * 770.0
            + 2.0 * std::f64::consts::PI * std::f64::consts::PI * 118.0 * 54.0 * 54.0
            + 4.0 / 3.0 * std::f64::consts::PI * 118.0f64.powi(3)
            + 190.0 * 46.0 * 16.0;
        assert!((facts.volume - expected).abs() / expected < 1e-12);
        assert_eq!(facts.solid_count, 4);
        // Union bbox across the four parts (box 155..345 in x, torus ±172 in
        // y, cylinder 1130..1900 and sphere 1782..2018 in z).
        assert_eq!(facts.bbox[0], [155.0, -172.0, 1130.0]);
        assert_eq!(facts.bbox[1], [642.0, 172.0, 2018.0]);
    }

    #[test]
    fn lathe_facts_match_occt_frustum_volume() {
        let housing = SolidSpec::Lathe {
            points: vec![
                [0.001, 1040.0],
                [95.0, 1110.0],
                [120.0, 1130.0],
                [0.001, 1130.0],
            ],
            arc_deg: 360.0,
        };
        let facts = tree_facts(&part(housing, 470.0, 0.0, 0.0))
            .expect("a full-arc line-profile lathe is in envelope");
        // OCC records the same turbine-housing solid at 1,390,947.111 mm^3.
        assert!((facts.volume - 1_390_947.111).abs() / 1_390_947.111 < 1e-9);
        assert_eq!(facts.bbox[0], [350.0, -120.0, 1040.0]);
        assert_eq!(facts.bbox[1], [590.0, 120.0, 1130.0]);
    }

    #[test]
    fn partial_arc_lathe_refuses_typed() {
        let partial = SolidSpec::Lathe {
            points: vec![[10.0, 0.0], [20.0, 0.0], [20.0, 10.0], [10.0, 10.0]],
            arc_deg: 270.0,
        };
        let refusal = tree_facts(&part(partial, 0.0, 0.0, 0.0))
            .expect_err("a partial-arc lathe is outside this executor's arm");
        assert!(matches!(
            refusal,
            Refusal::UnsupportedEnvelope(EnvelopeCase::NonCanonicalCarrier)
        ));
    }

    #[test]
    fn stl_writer_emits_binary_stl() {
        let tree = TreeNode::Group {
            group: vec![part(SolidSpec::Sphere { radius: 10.0 }, 1.0, 2.0, 3.0)],
        };
        let dir = std::env::temp_dir().join(format!("truck123d_bd_stl_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch dir must be creatable"); // H-3: test scratch
        let path = dir.join("sphere.stl");
        let triangles = write_tree_stl(&tree, &path.to_string_lossy()).expect("stl writes");
        assert!(triangles > 0);
        let bytes = std::fs::read(&path).expect("stl reads back"); // H-3: output of our own writer
        assert!(bytes.len() > 84);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The census vocabulary the drop-in answers name-for-name (spec 8 rows
    /// S1–S8's corpus-facing names). Every name must resolve on the truck
    /// drop-in module surface (`corpus/ttc/door.py` installs it under
    /// `cadgen.build123d` in the `--engine truck` regime) — never a silent
    /// OCC fallback.
    const CENSUS_NAMES: &[&str] = &[
        "Box",
        "Cylinder",
        "Sphere",
        "Torus",
        "Compound",
        "Vector",
        "Location",
        "Axis",
        "Edge",
        "Wire",
        "Face",
        "revolve",
        "extrude",
        "sweep",
        "loft",
        "fillet",
        "chamfer",
        "mirror",
        "Spline",
        "Circle",
        "Polygon",
        "Polyline",
        "make_face",
        "export_stl",
        "Plane",
        "Pos",
        "Rotation",
        "Mode",
    ];

    #[test]
    fn drop_in_module_answers_census_vocabulary_name_for_name() {
        // The names the drop-in must answer, name for name (spec 8): every
        // census name must be wired on the truck surface source AND map to an
        // executor/facade arm in this module (a geometry arm, or the typed
        // refusal path for an unmapped carrier form).
        let door_source = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../corpus/ttc/door.py"
        ))
        .expect("the corpus door source must be readable"); // H-3: fixture under test, read-only
        for name in CENSUS_NAMES {
            assert!(
                door_source.contains(name),
                "census name {name} must be wired on the truck drop-in module surface"
            );
        }
        // The truck regime installs the truck bridge module in place of
        // build123d and the OCC baseline stays the untouched reference path.
        assert!(door_source.contains("def install_truck_alias"));
        assert!(door_source.contains("install_cadgen_alias"));
        assert!(door_source.contains("--engine"));
        assert_eq!(
            door_source.matches("def install_cadgen_alias").count(),
            1,
            "anchor A1: the OCC alias installer stays a single definition"
        );
        // The geometry bridge entries this module registers on truck123d are
        // the ones the truck regime drives (facts + STL).
        assert!(door_source.contains("bd_facts"));
        assert!(door_source.contains("bd_stl"));
        // Signature discipline: the drop-in never wraps/improves a census
        // name beyond the corpus call patterns (no extra convenience sugar in
        // the truck regime beyond the answered names).
        assert!(door_source.contains("_TRUCK_NAMES"));
    }

    #[test]
    fn unsupported_carrier_refusal_maps_to_the_refused_exception_class() {
        // A refusal on an unmapped/unsupported carrier form is typed end to
        // end: the executor produces `UnsupportedEnvelope(NonCanonicalCarrier)`
        // (see partial_arc_lathe_refuses_typed) and the marshal layer maps that
        // refusal to the `Refused` Python exception class — never a panic and
        // never a bare Exception.
        let partial = SolidSpec::Lathe {
            points: vec![[10.0, 0.0], [20.0, 0.0], [20.0, 10.0], [10.0, 10.0]],
            arc_deg: 270.0,
        };
        let refusal = tree_facts(&part(partial, 0.0, 0.0, 0.0))
            .expect_err("the partial-arc lathe is outside this executor's arm");
        let marshaled = crate::marshal::Marshaled::from_refusal(&refusal);
        assert_eq!(
            marshaled.class,
            crate::marshal::ExceptionClass::Refused,
            "an unsupported-carrier refusal must surface as the mapped Refused class"
        );
    }
}
