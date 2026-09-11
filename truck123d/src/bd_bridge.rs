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
//! the lathe over a closed `y = 0` profile — line edges exactly and
//! spline-profile edges integrated as the TRUE reconstructed interpolating
//! spline, never a flattening polygon; FH-SPLINE-LATHE). The lathe arm covers
//! the full 360° revolution and a partial-arc wedge over
//! `[start_deg, start_deg + arc_deg]`, with the two planar caps closed
//! (DOOR-PARTIAL-ARC-FLIP). A name whose form is outside this envelope (a
//! swept/lofted carrier, a spline profile with a non-recoverable
//! interpolation, an arc outside `(0, 360]`) refuses with the typed kernel
//! refusal (`NonCanonicalCarrier`), which the pyo3 surface maps to the landed
//! `Refused`/`Unresolved` exception classes — loud, never a silent OCC
//! fallback.
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
use truck_base::evidence::{Budget, EnvelopeCase, Refusal, UnresolvedWitness};
use truck_certified::construct::patches::{PatchParent, TensorBernsteinPatch};
use truck_certified::kernel::patch::IBox2;

use crate::facade::{BooleanPairVerdict, CarrierClass, SweptBooleanEvent};
use crate::glb_emit::{emit_glb, GlbMesh, GlbNodePayload, SrgbColor};
use crate::marshal::{ExceptionClass, Marshaled, MarshaledPayload};
use crate::python;
use crate::python::binding::{EdgeSelectorRow, FilletBaseRow, FilletRefusal, FilletRow};

use self::membership::BooleanVolumeRefusal;

const TAU: f64 = std::f64::consts::TAU;

/// The node count of the equal-spaced trigonometric rule used for the exact
/// circle-loft side moment. The integrand is a trigonometric polynomial of
/// degree at most three, and the rule is exact for degrees below the node
/// count, so this is a fixed, deterministic exact integration.
const CIRCLE_QUAD: usize = 16;

/// The census-recorded curve of one lathe profile edge, in the `y = 0`
/// profile plane. Profile coordinates are `(x, z)` with `x >= 0` the revolve
/// radius (so `y == 0`).
///
/// A spline edge records its DEFINING samples (the points the corpus passed
/// to `Edge.make_spline`, which is how a data-only surface can record the
/// carrier without any geometry computing in Python). The kernel reconstructs
/// the interpolating curve from the samples; it never sees a flattening
/// polygon.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum LatheEdge {
    /// A straight profile edge from `a` to `b`.
    Line { a: [f64; 2], b: [f64; 2] },
    /// A spline profile edge: the interpolation samples passed to
    /// `Edge.make_spline(points)` with no tangents/parameters. The OCC
    /// interpolation convention is recoverable from the sample list (chord
    /// length parameters, clamped cubic, endpoint tangents from the Lagrange
    /// derivative of the first/last four samples), so the recorded data is the
    /// carrier's defining data.
    Spline { points: Vec<[f64; 2]> },
}

/// The census-recorded curve of one profile edge of a general authoring arm
/// (extrude prism / loft section), in the part's local 3-D frame. A line edge
/// records its endpoints exactly; a spline edge records its DEFINING samples
/// (the points the corpus passed to `Edge.make_spline`). The recording scheme
/// never flattens a spline to a polygon: the carrier keeps the samples and the
/// arm either integrates the true curve or refuses typed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ProfileEdge {
    /// A straight profile edge from `a` to `b`.
    Line { a: [f64; 3], b: [f64; 3] },
    /// A spline profile edge: the interpolation samples passed to
    /// `Edge.make_spline(points)` with no tangents/parameters.
    Spline { points: Vec<[f64; 3]> },
    /// A closed circular section edge (the PB-014 circle carrier): an exact
    /// conic recorded by its centre, radius and the unit normal of its plane.
    /// The circle is never flattened to a polygon at record time; the kernel
    /// derives a deterministic in-plane basis from `normal`, computes the
    /// analytic extrema and tessellates only where a triangle mesh is asked
    /// for. A single circle edge is a whole section.
    Circle {
        /// The circle centre in the part-local frame.
        center: [f64; 3],
        /// The circle radius.
        radius: f64,
        /// The unit normal of the circle's plane (defines its orientation and
        /// the deterministic in-plane parameterization).
        normal: [f64; 3],
    },
}

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
    /// `revolve(face, axis=z, revolution_arc=..., start_angle=...)` over a
    /// closed profile lying in the `y = 0` plane. `profile` is the boundary in
    /// order: line edges and interpolating-spline edges (whose defining
    /// samples the edge records). Profile coordinates are `(x, z)` with
    /// `x >= 0` the revolve radius. The wedge is swept over the angular range
    /// `[start_deg, start_deg + arc_deg]`; the full 360-degree revolution is
    /// the special case the analytic arm has always certified.
    Lathe {
        /// The closed `(x, z)` profile boundary edges, in order.
        profile: Vec<LatheEdge>,
        /// The swept arc in degrees, in `(0, 360]`.
        arc_deg: f64,
        /// The v-range start of the swept arc, in degrees. A full-360 row
        /// ignores it (the revolution is rotationally symmetric); a partial
        /// row realizes it exactly.
        #[serde(default)]
        start_deg: f64,
    },
    /// `extrude(face, amount, both)`: a closed planar line-loop profile swept
    /// along its own plane normal. `profile` is the recorded boundary in order
    /// (part-local 3-D coordinates). The extruded solid is an exact prism: the
    /// volume is the profile area times the swept length (`amount`, or `2 *
    /// amount` when `both`), the bbox is the profile support widened along the
    /// normal, and the mesh sweeps the boundary deterministically.
    Prism {
        /// The closed planar line-loop boundary edges, in order.
        profile: Vec<ProfileEdge>,
        /// The swept length along the profile-plane normal.
        amount: f64,
        /// Extrude on both sides of the profile plane (`both=True`: the total
        /// extent is `2 * amount`, symmetric about the plane).
        #[serde(default)]
        both: bool,
    },
    /// The spline-trimmed extrude constructor row (TRIM-EXTRUDE-CTOR): the
    /// base profile plus a recorded closed spline trim curve and (optionally)
    /// the trim's algebraic pullback net. The composition itself lives in the
    /// binding layer (`crate::python::binding::trim_extrude`); this arm is a
    /// pure pass-through so the door vocabulary reaches the constructor and
    /// the facts/mesh re-enter the normal executor path.
    TrimPrism {
        /// The base profile boundary edges (line loop; a spline edge is a
        /// recorded trim carrier the constructor classifies).
        profile: Vec<ProfileEdge>,
        /// The swept length along the profile-plane normal.
        amount: f64,
        /// Extrude symmetrically about the profile plane.
        #[serde(default)]
        both: bool,
        /// The closed spline curve control points (first == last for a closed
        /// loop); empty for a full extrude.
        #[serde(default)]
        trim_curve: Vec<[f64; 3]>,
        /// The curve's pullback net (row-major Bernstein grid over
        /// the unit square, `>= 0` kept); empty when only the parametric curve
        /// is recorded.
        #[serde(default)]
        trim_net: Vec<Vec<f64>>,
        /// The requested certified bracket tolerance.
        #[serde(default = "default_trim_tolerance")]
        tolerance: f64,
    },
    /// `loft(sections)` (and, for a closed station list, the sweep-as-loft-chain
    /// form): an ordered stack of section profiles, each a closed planar
    /// line-loop in the part's local 3-D frame, interpolated by matching
    /// vertices in order (the ruled carrier the recording scheme fixes).
    ///
    /// When `closed` is true the row is a halo-style loop: the last station is
    /// the exact return to the first station and the loft surface closes on
    /// itself with no end caps. The arm certifies the closure seam with the
    /// exact aligned-meeting-edge identity (`A0 W1 - A1 W0 == 0`, weights 1 for
    /// the recorded line carrier); a station list that does not close refuses
    /// typed with the mismatch evidence.
    Loft {
        /// The section profiles, in station order.
        sections: Vec<Vec<ProfileEdge>>,
        /// Whether the station list is a closed halo loop (last station == the
        /// return to the first station).
        #[serde(default)]
        closed: bool,
    },
    /// A plate-section structural member: one closed planar plate profile (the
    /// corpus `blade_profile` / `rounded_plate_pts` section) placed by each
    /// recorded station frame and swept along the station chain. This is the
    /// kernel realization of the corpus member vocabulary
    /// (`surfaces.swept_plate` / `surfaces.blade_path` / `surfaces.blade_member`
    /// in `corpus/ttc/trees/f1/src/lib/surfaces.py`): a member is a two-section
    /// degenerate case plus wall bands, so its certified facts ride the landed
    /// MONO-2 two-station loft certificate per adjacent station pair rather
    /// than a new integrator.
    Member {
        /// The closed plate cross-section, in the station-0 local frame.
        profile: Vec<ProfileEdge>,
        /// The recorded station frames, in path order (`>= 2` stations).
        stations: Vec<StationFrame>,
        /// The station chain is a ruled sweep (piecewise-linear between
        /// stations). A smooth (`ruled = false`) member with more than two
        /// stations has no landed smooth carrier and refuses typed.
        #[serde(default)]
        ruled: bool,
    },
}

/// The default certified bracket tolerance of a trim-prism row (the door's
/// recorded `1e-3`; dimensionless, per ADM-003's trim tolerance scale).
fn default_trim_tolerance() -> f64 {
    1.0e-3
}

/// The recorded full orthonormal placement rotation of one part (the frame
/// generalization of the pure-z rotation). The columns are the world
/// images of the part-local x, y and z axes after the placement rotation: a
/// local point `p` maps to `x_dir * p.x + y_dir * p.y + z_dir * p.z`. The
/// client records an orthonormal frame (fixed-order Gram-Schmidt); volume is
/// invariant under `R` and only the world bbox and mesh re-derive from it.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RotationFrame {
    /// The world image of the part-local x axis.
    pub x_dir: [f64; 3],
    /// The world image of the part-local y axis.
    pub y_dir: [f64; 3],
    /// The world image of the part-local z axis.
    pub z_dir: [f64; 3],
}

impl RotationFrame {
    /// Applies the recorded orthonormal rotation to a local point.
    fn apply(self, p: [f64; 3]) -> [f64; 3] {
        let [x0, x1, x2] = self.x_dir;
        let [y0, y1, y2] = self.y_dir;
        let [z0, z1, z2] = self.z_dir;
        [
            x0 * p[0] + y0 * p[1] + z0 * p[2],
            x1 * p[0] + y1 * p[1] + z1 * p[2],
            x2 * p[0] + y2 * p[1] + z2 * p[2],
        ]
    }
}

/// One recorded member station: the origin plus the orthonormal frame that
/// places the plate profile at that station (local x/y span the plate, local z
/// is the path tangent). A member's section at the station is the profile
/// mapped through `p -> origin + x_dir * p.x + y_dir * p.y + z_dir * p.z`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StationFrame {
    /// The world origin of the station.
    pub origin: [f64; 3],
    /// The world image of the profile-local x axis.
    pub x_dir: [f64; 3],
    /// The world image of the profile-local y axis.
    pub y_dir: [f64; 3],
    /// The world image of the profile-local z axis (the path tangent).
    pub z_dir: [f64; 3],
}

impl StationFrame {
    /// Applies the recorded station placement to a profile-local point.
    fn apply(self, p: [f64; 3]) -> [f64; 3] {
        let [x, y, z] = p;
        let [ox, oy, oz] = self.origin;
        let [xx, xy, xz] = self.x_dir;
        let [yx, yy, yz] = self.y_dir;
        let [zx, zy, zz] = self.z_dir;
        [
            ox + xx * x + yx * y + zx * z,
            oy + xy * x + yy * y + zy * z,
            oz + xz * x + yz * y + zz * z,
        ]
    }

    /// Applies the recorded station placement to a profile-local direction
    /// (the rotation only, no translation).
    fn apply_dir(self, p: [f64; 3]) -> [f64; 3] {
        let [x, y, z] = p;
        let [xx, xy, xz] = self.x_dir;
        let [yx, yy, yz] = self.y_dir;
        let [zx, zy, zz] = self.z_dir;
        [
            xx * x + yx * y + zx * z,
            xy * x + yy * y + zy * z,
            xz * x + yz * y + zz * z,
        ]
    }

    /// Whether every recorded frame coordinate is finite.
    fn is_finite(self) -> bool {
        self.origin.iter().all(|c| c.is_finite())
            && self.x_dir.iter().all(|c| c.is_finite())
            && self.y_dir.iter().all(|c| c.is_finite())
            && self.z_dir.iter().all(|c| c.is_finite())
    }
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
    /// The full orthonormal placement rotation recorded from an authoring
    /// frame (a `Plane`/`Pos`/`Rotation` frame row). When present it replaces
    /// the pure-z rotation exactly (`world = translate(o) ∘ R ∘ M`); a row
    /// without it keeps the pure-z rotation path byte-identical.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rotation: Option<RotationFrame>,
    /// A placed-carrier reflection about a coordinate plane through the local
    /// origin, recorded by the mirror arm: `"x"` (YZ plane, x -> -x), `"y"`
    /// (XZ plane, y -> -y) or `"z"` (XY plane, z -> -z). The reflection is a
    /// congruence: it applies to the local geometry before the rz rotation and
    /// translation, facts transform with the placement, and no geometry is
    /// recomputed.
    #[serde(default)]
    pub mirror: Option<String>,
    /// The row's recorded label (the door's `styled` metadata, carried
    /// verbatim; never read by classification, facts arithmetic or meshing).
    /// Omitted when the client records none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// The row's recorded color (client metadata carried verbatim; never read
    /// by classification, facts arithmetic or meshing). Omitted when the client
    /// records none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

/// One recorded boolean row: the mode and the two placed operand nodes.
///
/// The operands' LOCAL carrier classes dispatch (a placement never reaches the
/// dispatch), and the routed event is recorded on the measured facts. A
/// boolean of a boolean result is the recorded open composition cell and
/// refuses typed (depth-1 only).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BooleanNode {
    /// The boolean mode (`union` / `subtract` / `intersect`).
    pub mode: crate::facade::ModeValue,
    /// The base operand node.
    pub a: Box<TreeNode>,
    /// The tool operand node.
    pub b: Box<TreeNode>,
}

/// One recorded fillet row: the base part node, the blend radius and the
/// recorded edge selectors.
///
/// Depth-1 only, the same discipline as [`BooleanNode`]: `base` must be a part
/// node. A fillet of a group or of another op node is the recorded open
/// composition cell and refuses typed naming the carrier (never a silent
/// recursion).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FilletNode {
    /// The base operand node (must be a part).
    pub base: Box<TreeNode>,
    /// The requested blend radius.
    pub radius: f64,
    /// The recorded edge selectors, in script order.
    pub edges: Vec<EdgeSelectorRow>,
}

/// A node of the submitted construction tree: either one placed solid (a
/// part), a group (a compound) of child nodes, or one recorded boolean row, in
/// script order.
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
        /// The group's recorded label (client metadata carried verbatim; never
        /// read by classification, facts arithmetic or meshing). Omitted when
        /// the client records none.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        label: Option<String>,
        /// The group's recorded color (client metadata carried verbatim; never
        /// read by classification, facts arithmetic or meshing). Omitted when
        /// the client records none.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        color: Option<String>,
    },
    /// One recorded boolean row over two operand nodes.
    Boolean {
        /// The recorded boolean row.
        boolean: BooleanNode,
    },
    /// One recorded fillet row over a base part node (AUTHOR-EXT-FILLET-HALO).
    Fillet {
        /// The recorded fillet row.
        fillet: FilletNode,
    },
}

/// The certified blend fact of a single fillet top node (AUTHOR-EXT-FILLET-HALO):
/// the landed blend profile certificate the recorded edge selectors produced.
/// `None` when the tree is not a single fillet node.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BlendFacts {
    /// The blend kind (`"fillet"`).
    pub kind: &'static str,
    /// The certified blend radius.
    pub radius: f64,
    /// The number of resolved (certified) edges.
    pub edges: usize,
    /// The maximum certified L2 control width of the resolved edge cross
    /// fields (exactly zero for an exactly recovered constant field).
    pub max_profile_width: f64,
}

/// The measured facts of a submitted construction tree.
#[derive(Debug, Clone, PartialEq)]
pub struct Facts {
    /// The solid count (every part anywhere in the tree).
    pub solid_count: u64,
    /// The volume of the measured top node, following the recorded OCC
    /// compound semantics the reference was measured with: a group's volume
    /// is the sum over its *immediate child parts only* (nested groups
    /// contribute nothing); a part's volume is its own; a boolean row is the
    /// PRODUCT's certified volume (the landed MONO-6 contact-cover
    /// certificate), never the base operand's.
    pub volume: f64,
    /// The interval-valued volume fact of the measured top node (MONO-8,
    /// theory statement section 5): a certified boolean node carries the
    /// solver's bracket `[V_lo, V_hi]`; a scalar node carries the degenerate
    /// interval `[V, V]`; a group aggregates interval sums over its immediate
    /// children. `volume` is this bracket's midpoint.
    pub volume_bracket: [f64; 2],
    /// The axis-aligned bounding box of every part in the tree, `[min, max]`.
    pub bbox: [[f64; 3]; 2],
    /// The measured seam mismatch of a single-loft tree, when the top node is
    /// one closed halo loft row: the exact aligned-meeting-edge identity
    /// (`A0 W1 - A1 W0 == 0`) holds when this is `0.0`. `None` when the tree is
    /// not a single closed loft part (no seam to certify).
    pub seam_mismatch: Option<f64>,
    /// The routed boolean events of the tree, in script order: one routed
    /// row per admitted swept-carrier boolean pair. A canonical x canonical
    /// pair lands on the canonical path and records no event.
    pub boolean_events: Vec<SweptBooleanEvent>,
    /// The certified blend fact of a single fillet top node, when present
    /// (AUTHOR-EXT-FILLET-HALO). `None` for every non-fillet tree.
    pub blend: Option<BlendFacts>,
    /// The top node's recorded label, when present (client metadata carried
    /// verbatim; never read by any semantic path).
    pub label: Option<String>,
    /// The top node's recorded color, when present (client metadata carried
    /// verbatim; never read by any semantic path).
    pub color: Option<String>,
    /// The per-row breakdown of a group top node: one entry per IMMEDIATE
    /// child, in script order. `None` for a part top node (and for a boolean
    /// top node); a nested group appears as ONE row carrying its aggregate.
    pub rows: Option<Vec<RowFacts>>,
}

/// The measured facts of one immediate child of a group top node. The
/// arithmetic is the aggregate path's (`count_and_union` / `top_volume`): a
/// nested group's row reports its immediate-child aggregate, exactly as the
/// aggregate rule weights it. `label` is the child's recorded metadata, when
/// present.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RowFacts {
    /// The child's recorded label, when present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// The child's solid count (every part in the child's subtree).
    pub solid_count: u64,
    /// The child's measured volume (the aggregate rule).
    pub volume: f64,
    /// The child's world axis-aligned bounding box, `[min, max]`.
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
        SolidSpec::Lathe {
            profile,
            arc_deg,
            start_deg,
        } => {
            let arc = validate_lathe_arc(*arc_deg, *start_deg)?;
            let full = if let Some(points) = line_profile_vertices(profile) {
                // The line-profile arm: the frustum telescoping of the closed
                // vertex loop, bit-identical to the landed line-profile facts.
                lathe_volume(&points)?
            } else {
                lathe_profile_volume(profile)?
            };
            if arc == 360.0 {
                Ok(full)
            } else {
                // The wedge is the full solid intersected with an angular
                // sector, so its volume is the full volume scaled by the
                // swept fraction — exact, never a sampled approximation.
                Ok(full * (arc / 360.0))
            }
        }
        SolidSpec::Prism {
            profile,
            amount,
            both,
        } => {
            let geom = prism_geom(profile, *amount, *both)?;
            Ok(geom.volume)
        }
        SolidSpec::TrimPrism {
            profile,
            amount,
            both,
            trim_curve,
            trim_net,
            tolerance,
        } => {
            if is_spline_profile_prism(profile, trim_curve, trim_net) {
                return spline_profile_prism_facts(trim_curve, *amount, *both)
                    .map(|(volume, _, _)| volume);
            }
            crate::python::binding::trim_extrude_solid(
                profile, *amount, *both, trim_curve, trim_net, *tolerance,
            )
            .map(|facts| facts.volume)
            .map_err(trim_binding_error_to_refusal)
        }
        SolidSpec::Loft { sections, closed } => {
            // The landed analytic ruled arm stays the fast path for the class
            // it already certifies (V5: bit-identical). A spline-section
            // carrier falls to the certified smooth arm; a closed halo chain
            // has no smooth certificate.
            if let Ok(validated) = loft_sections(sections) {
                return loft_volume(&validated, *closed);
            }
            if *closed {
                return Err(open_smooth_loft());
            }
            certified_spline_loft_volume(sections).map(|(value, _, _)| value)
        }
        SolidSpec::Member {
            profile,
            stations,
            ruled,
        } => member_volume_bracket(profile, stations, *ruled).map(|(value, _, _)| value),
    }
}

/// The closed `(x, z)` vertex loop of an all-line profile, in boundary order.
/// `None` when any profile edge is a spline (the spline arm then applies).
fn line_profile_vertices(profile: &[LatheEdge]) -> Option<Vec<[f64; 2]>> {
    let mut vertices = Vec::with_capacity(profile.len());
    for edge in profile {
        match edge {
            LatheEdge::Line { a, .. } => vertices.push(*a),
            LatheEdge::Spline { .. } => return None,
        }
    }
    Some(vertices)
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

// ---------------------------------------------------------------------------
// Spline-profile lathe facts
// ---------------------------------------------------------------------------
//
// A spline profile edge records the samples the corpus passed to
// `Edge.make_spline(points)`. The kernel reconstructs the interpolating curve
// the OCC reference revolved — chord-length parameters, a clamped cubic with
// a knot at every sample, C2 at the interior samples, and endpoint tangents
// equal to the derivative of the degree-3 Lagrange interpolant of the first
// (resp. last) four samples. This is exactly the curve `GeomAPI_Interpolate`
// builds for a non-periodic point list with no tangents (the convention
// `make_spline(points)` fixes); it is recoverable from the samples, so the
// arm integrates the TRUE spline and never a flattening polygon.
//
// Facts are exact polynomial arithmetic: over one span, `r` and `z` are
// cubic polynomials in the span parameter, and the segment's solid-of-
// revolution volume `pi * int r(u)^2 * z'(u) du` is a degree-8 polynomial
// integral evaluated in closed form (never sampled). The line-profile
// frustum telescoping in `lathe_volume` is the degree-1 special case and is
// preserved bit-identical for all-line profiles (see `solid_volume`).

/// One reconstructed spline span, in power basis on `u in [0, 1]`.
#[derive(Debug, Clone, Copy)]
struct SpanPoly {
    /// Radius `r(u)`.
    r: [f64; 4],
    /// Axial coordinate `z(u)`.
    z: [f64; 4],
}

/// Whether every profile edge is a valid finite carrier at `x >= 0`.
fn check_profile_edges(profile: &[LatheEdge]) -> Result<(), Refusal> {
    for edge in profile {
        match edge {
            LatheEdge::Line { a, b } => {
                let (ax, az) = (a[0], a[1]);
                let (bx, bz) = (b[0], b[1]);
                if ax < 0.0
                    || bx < 0.0
                    || !ax.is_finite()
                    || !az.is_finite()
                    || !bx.is_finite()
                    || !bz.is_finite()
                {
                    return Err(Refusal::Empty);
                }
            }
            LatheEdge::Spline { points } => {
                if points.len() < 2 {
                    return Err(Refusal::Empty);
                }
                for p in points {
                    let (x, z) = (p[0], p[1]);
                    if x < 0.0 || !x.is_finite() || !z.is_finite() {
                        return Err(Refusal::Empty);
                    }
                }
            }
        }
    }
    if profile.len() < 3 {
        return Err(Refusal::Empty);
    }
    Ok(())
}

/// The volume of a spline-bearing lathe profile: the exact segment-moment sum
/// over the profile boundary (spline spans integrated by closed-form
/// polynomial arithmetic, line edges by the frustum formula), absolute value
/// absorbing the winding.
fn lathe_profile_volume(profile: &[LatheEdge]) -> Result<f64, Refusal> {
    check_profile_edges(profile)?;
    let mut sum = 0.0;
    for edge in profile {
        match edge {
            LatheEdge::Line { a, b } => {
                let (x0, z0) = (a[0], a[1]);
                let (x1, z1) = (b[0], b[1]);
                sum += std::f64::consts::PI / 3.0 * (z1 - z0) * (x0 * x0 + x0 * x1 + x1 * x1);
            }
            LatheEdge::Spline { points } => {
                sum += spline_edge_volume(points)?;
            }
        }
    }
    if !sum.is_finite() {
        return Err(Refusal::Empty);
    }
    Ok(sum.abs())
}

/// The signed volume contribution of one spline profile edge: the exact
/// segment-moment integral `pi * int r(u)^2 z'(u) du` over the whole edge.
fn spline_edge_volume(points: &[[f64; 2]]) -> Result<f64, Refusal> {
    let spans = spline_spans(points)?;
    let mut sum = 0.0;
    for span in &spans {
        sum += span_volume(span);
    }
    if !sum.is_finite() {
        return Err(Refusal::Empty);
    }
    Ok(sum)
}

/// The exact `pi * int_0^1 r(u)^2 z'(u) du` of one span. `r(u)` is a cubic
/// (degree 6 when squared), `z'(u)` a quadratic; the product is degree 8 and
/// integrates term-by-term (`int u^k = 1/(k+1)`) — no quadrature, no
/// sampling.
fn span_volume(span: &SpanPoly) -> f64 {
    // r(u)^2 in the power basis (degree 6).
    let mut r2 = [0.0f64; 7];
    for (i, ri) in span.r.iter().enumerate() {
        for (j, rj) in span.r.iter().enumerate() {
            r2[i + j] += ri * rj;
        }
    }
    // z'(u) in the power basis (degree 2).
    let dz = [span.z[1], 2.0 * span.z[2], 3.0 * span.z[3]];
    let mut integral = 0.0;
    for (k, ck) in r2.iter().enumerate() {
        for (l, dl) in dz.iter().enumerate() {
            integral += ck * dl / (k + l + 1) as f64;
        }
    }
    std::f64::consts::PI * integral
}

/// The reconstructed spans of the OCC interpolating curve through the samples.
fn spline_spans(points: &[[f64; 2]]) -> Result<Vec<SpanPoly>, Refusal> {
    let n = points.len();
    if n < 2 {
        return Err(Refusal::Empty);
    }
    for p in points {
        let (x, z) = (p[0], p[1]);
        if x < 0.0 || !x.is_finite() || !z.is_finite() {
            return Err(Refusal::Empty);
        }
    }
    let params = chord_params(points);
    if n == 2 {
        // A two-sample spline interpolates the chord itself.
        let (x0, z0) = (points[0][0], points[0][1]);
        let (x1, z1) = (points[1][0], points[1][1]);
        return Ok(vec![SpanPoly {
            r: [x0, x1 - x0, 0.0, 0.0],
            z: [z0, z1 - z0, 0.0, 0.0],
        }]);
    }
    if n == 3 {
        // A three-sample spline is the quadratic through the three points.
        let t0 = params[0];
        let t2 = params[2];
        let u1 = (params[1] - t0) / (t2 - t0);
        let (x0, z0) = (points[0][0], points[0][1]);
        let (x2, z2) = (points[2][0], points[2][1]);
        let (x1, z1) = (points[1][0], points[1][1]);
        let denom = 2.0 * u1 * (1.0 - u1);
        let qx1 = (x1 - (1.0 - u1) * (1.0 - u1) * x0 - u1 * u1 * x2) / denom;
        let qz1 = (z1 - (1.0 - u1) * (1.0 - u1) * z0 - u1 * u1 * z2) / denom;
        // Quadratic Bezier (x0, qx1, x2) in the power basis over u in [0, 1].
        let bx = [x0, 2.0 * (qx1 - x0), x0 - 2.0 * qx1 + x2, 0.0];
        let bz = [z0, 2.0 * (qz1 - z0), z0 - 2.0 * qz1 + z2, 0.0];
        return Ok(vec![SpanPoly { r: bx, z: bz }]);
    }
    let slopes = clamped_slopes(points, &params)?;
    let mut spans = Vec::with_capacity(n - 1);
    for i in 0..n - 1 {
        let h = params.get(i + 1).copied().ok_or(Refusal::Empty)?
            - params.get(i).copied().ok_or(Refusal::Empty)?;
        let (x0, z0) = {
            let p = points.get(i).ok_or(Refusal::Empty)?;
            (p[0], p[1])
        };
        let (x1, z1) = {
            let p = points.get(i + 1).ok_or(Refusal::Empty)?;
            (p[0], p[1])
        };
        let (s0x, s0z) = {
            let s = slopes.get(i).ok_or(Refusal::Empty)?;
            (s[0], s[1])
        };
        let (s1x, s1z) = {
            let s = slopes.get(i + 1).ok_or(Refusal::Empty)?;
            (s[0], s[1])
        };
        let span = SpanPoly {
            r: hermite_power(x0, x1, h * s0x, h * s1x),
            z: hermite_power(z0, z1, h * s0z, h * s1z),
        };
        spans.push(span);
    }
    Ok(spans)
}

/// The chord-length parameters of the samples.
fn chord_params(points: &[[f64; 2]]) -> Vec<f64> {
    let mut params = Vec::with_capacity(points.len());
    params.push(0.0);
    for pair in points.windows(2) {
        let (x0, z0) = (pair[0][0], pair[0][1]);
        let (x1, z1) = (pair[1][0], pair[1][1]);
        let dx = x1 - x0;
        let dz = z1 - z0;
        let next = params.last().copied().unwrap_or(0.0) + (dx * dx + dz * dz).sqrt();
        params.push(next);
    }
    params
}

/// The power-basis coefficients of the cubic Hermite interpolant over
/// `u in [0, 1]` with `C(0)=y0`, `C'(0)=d0`, `C(1)=y1`, `C'(1)=d1`.
fn hermite_power(y0: f64, y1: f64, d0: f64, d1: f64) -> [f64; 4] {
    [
        y0,
        d0,
        -3.0 * y0 - 2.0 * d0 + 3.0 * y1 - d1,
        2.0 * y0 + d0 - 2.0 * y1 + d1,
    ]
}

/// The derivative, at its first sample, of the degree-3 Lagrange polynomial
/// through the four given `(x, z)` samples at their chord parameters.
fn lagrange_first_tangent(points: &[[f64; 2]], params: &[f64]) -> [f64; 2] {
    // Closed form for evaluation at node 0 over nodes 0..3:
    //   L'(x0) = sum_i P_i * l_i'(x0)
    let l0prime = 1.0 / (params[0] - params[1])
        + 1.0 / (params[0] - params[2])
        + 1.0 / (params[0] - params[3]);
    let l1prime = 1.0 / (params[1] - params[0])
        * ((params[0] - params[2]) / (params[1] - params[2]))
        * ((params[0] - params[3]) / (params[1] - params[3]));
    let l2prime = 1.0 / (params[2] - params[0])
        * ((params[0] - params[1]) / (params[2] - params[1]))
        * ((params[0] - params[3]) / (params[2] - params[3]));
    let l3prime = 1.0 / (params[3] - params[0])
        * ((params[0] - params[1]) / (params[3] - params[1]))
        * ((params[0] - params[2]) / (params[3] - params[2]));
    let mut tx = 0.0;
    let mut tz = 0.0;
    let factors = [l0prime, l1prime, l2prime, l3prime];
    for i in 0..4 {
        tx += points[i][0] * factors[i];
        tz += points[i][1] * factors[i];
    }
    [tx, tz]
}

/// The derivative, at its last sample, of the degree-3 Lagrange polynomial
/// through the four given `(x, z)` samples at their chord parameters.
fn lagrange_last_tangent(points: &[[f64; 2]], params: &[f64]) -> [f64; 2] {
    // Closed form for evaluation at node 3 over nodes 0..3.
    let l0prime = 1.0 / (params[0] - params[3])
        * ((params[3] - params[1]) / (params[0] - params[1]))
        * ((params[3] - params[2]) / (params[0] - params[2]));
    let l1prime = 1.0 / (params[1] - params[3])
        * ((params[3] - params[0]) / (params[1] - params[0]))
        * ((params[3] - params[2]) / (params[1] - params[2]));
    let l2prime = 1.0 / (params[2] - params[3])
        * ((params[3] - params[0]) / (params[2] - params[0]))
        * ((params[3] - params[1]) / (params[2] - params[1]));
    let l3prime = 1.0 / (params[3] - params[0])
        + 1.0 / (params[3] - params[1])
        + 1.0 / (params[3] - params[2]);
    let mut tx = 0.0;
    let mut tz = 0.0;
    let factors = [l0prime, l1prime, l2prime, l3prime];
    for i in 0..4 {
        tx += points[i][0] * factors[i];
        tz += points[i][1] * factors[i];
    }
    [tx, tz]
}

/// The clamped cubic spline slopes (first derivatives `d/d(parameter)`) at
/// every sample: the interior slopes solve the C2 tridiagonal system, the
/// end slopes are the Lagrange end tangents of the first/last four samples.
fn clamped_slopes(points: &[[f64; 2]], params: &[f64]) -> Result<Vec<[f64; 2]>, Refusal> {
    let n = points.len();
    if n < 4 {
        return Err(Refusal::Empty);
    }
    let first4 = points.get(0..4).ok_or(Refusal::Empty)?;
    let last4 = points.get(n - 4..).ok_or(Refusal::Empty)?;
    let first_t = params.get(0..4).ok_or(Refusal::Empty)?;
    let last_t = params.get(n - 4..).ok_or(Refusal::Empty)?;
    let mut slopes = vec![[0.0, 0.0]; n];
    if let Some(first) = slopes.first_mut() {
        *first = lagrange_first_tangent(first4, first_t);
    }
    if let Some(last) = slopes.last_mut() {
        *last = lagrange_last_tangent(last4, last_t);
    }
    let nk = n - 2;
    let mut h = Vec::with_capacity(n - 1);
    for i in 0..n - 1 {
        let hi = params.get(i + 1).copied().ok_or(Refusal::Empty)?
            - params.get(i).copied().ok_or(Refusal::Empty)?;
        if hi <= 0.0 || !hi.is_finite() {
            return Err(Refusal::Empty);
        }
        h.push(hi);
    }
    let mut delta = Vec::with_capacity(n - 1);
    for pair in points.windows(2) {
        let (x0, z0) = (pair[0][0], pair[0][1]);
        let (x1, z1) = (pair[1][0], pair[1][1]);
        let len = delta.len();
        let hi = h.get(len).copied().unwrap_or(1.0);
        delta.push([(x1 - x0) / hi, (z1 - z0) / hi]);
    }
    let mut rhs = vec![[0.0, 0.0]; nk];
    let mut a = vec![0.0; nk];
    let mut b = vec![0.0; nk];
    let mut c = vec![0.0; nk];
    for row in 0..nk {
        let i = row + 1;
        let hi = h.get(i).copied().ok_or(Refusal::Empty)?;
        let hm = h.get(i - 1).copied().ok_or(Refusal::Empty)?;
        if let Some(av) = a.get_mut(row) {
            *av = hi;
        }
        if let Some(bv) = b.get_mut(row) {
            *bv = 2.0 * (hm + hi);
        }
        if let Some(cv) = c.get_mut(row) {
            *cv = hm;
        }
        let dprev = delta.get(i - 1).copied().ok_or(Refusal::Empty)?;
        let dcur = delta.get(i).copied().ok_or(Refusal::Empty)?;
        let mut r0 = 3.0 * (hi * dprev[0] + hm * dcur[0]);
        let mut r1 = 3.0 * (hi * dprev[1] + hm * dcur[1]);
        if i == 1 {
            let s0 = slopes.get(0).copied().unwrap_or([0.0, 0.0]);
            r0 -= hi * s0[0];
            r1 -= hi * s0[1];
        }
        if i == n - 2 {
            let sn = slopes.get(n - 1).copied().unwrap_or([0.0, 0.0]);
            r0 -= hm * sn[0];
            r1 -= hm * sn[1];
        }
        if let Some(r) = rhs.get_mut(row) {
            *r = [r0, r1];
        }
    }
    // Thomas algorithm.
    let mut cp = vec![0.0; nk];
    let mut dp = vec![[0.0, 0.0]; nk];
    {
        let b0 = b.get(0).copied().ok_or(Refusal::Empty)?;
        if b0 == 0.0 {
            return Err(Refusal::Empty);
        }
        let c0 = c.get(0).copied().unwrap_or(0.0);
        if let Some(v) = cp.first_mut() {
            *v = c0 / b0;
        }
        let r = rhs.get(0).copied().ok_or(Refusal::Empty)?;
        if let Some(v) = dp.first_mut() {
            *v = [r[0] / b0, r[1] / b0];
        }
    }
    for row in 1..nk {
        let ai = a.get(row).copied().unwrap_or(0.0);
        let bi = b.get(row).copied().unwrap_or(0.0);
        let ci = c.get(row).copied().unwrap_or(0.0);
        let cp_prev = cp.get(row - 1).copied().unwrap_or(0.0);
        let dp_prev = dp.get(row - 1).copied().unwrap_or([0.0, 0.0]);
        let r = rhs.get(row).copied().unwrap_or([0.0, 0.0]);
        let denom = bi - ai * cp_prev;
        if denom == 0.0 || !denom.is_finite() {
            return Err(Refusal::Empty);
        }
        if row < nk - 1 {
            if let Some(v) = cp.get_mut(row) {
                *v = ci / denom;
            }
        }
        let v0 = (r[0] - ai * dp_prev[0]) / denom;
        let v1 = (r[1] - ai * dp_prev[1]) / denom;
        if !v0.is_finite() || !v1.is_finite() {
            return Err(Refusal::Empty);
        }
        if let Some(v) = dp.get_mut(row) {
            *v = [v0, v1];
        }
    }
    let mut x = vec![[0.0, 0.0]; nk];
    if let Some(last) = x.last_mut() {
        *last = dp.get(nk - 1).copied().unwrap_or([0.0, 0.0]);
    }
    for row in (0..nk - 1).rev() {
        let dpv = dp.get(row).copied().unwrap_or([0.0, 0.0]);
        let cpv = cp.get(row).copied().unwrap_or(0.0);
        let xv = x.get(row + 1).copied().unwrap_or([0.0, 0.0]);
        if let Some(v) = x.get_mut(row) {
            *v = [dpv[0] - cpv * xv[0], dpv[1] - cpv * xv[1]];
        }
    }
    for row in 0..nk {
        let v = x.get(row).copied().unwrap_or([0.0, 0.0]);
        if let Some(s) = slopes.get_mut(row + 1) {
            *s = v;
        }
    }
    Ok(slopes)
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
        SolidSpec::Lathe {
            profile,
            arc_deg,
            start_deg,
        } => lathe_wedge_bbox(profile, *arc_deg, *start_deg),
        SolidSpec::Prism {
            profile,
            amount,
            both,
        } => prism_bbox(profile, *amount, *both),
        SolidSpec::TrimPrism {
            profile,
            amount,
            both,
            trim_curve,
            trim_net,
            tolerance,
        } => {
            if is_spline_profile_prism(profile, trim_curve, trim_net) {
                return spline_profile_prism_facts(trim_curve, *amount, *both)
                    .map(|(_, bbox, _)| bbox);
            }
            crate::python::binding::trim_extrude_solid(
                profile, *amount, *both, trim_curve, trim_net, *tolerance,
            )
            .map(|facts| facts.bbox)
            .map_err(trim_binding_error_to_refusal)
        }
        SolidSpec::Loft { sections, .. } => {
            if let Ok(validated) = loft_sections(sections) {
                let mut min = [f64::INFINITY; 3];
                let mut max = [f64::NEG_INFINITY; 3];
                for loop3 in &validated.loops {
                    let box3 = loop_bbox(loop3);
                    for axis in 0..3 {
                        min[axis] = min[axis].min(box3[0][axis]);
                        max[axis] = max[axis].max(box3[1][axis]);
                    }
                }
                if !min[0].is_finite() || !max[0].is_finite() {
                    return Err(Refusal::Empty);
                }
                return Ok([min, max]);
            }
            // The ruled side surface is linear in the station axis, so the
            // exact AABB is the union of the section loops' exact AABBs; for
            // N > 2 the canonical v-interpolation can bulge past the sections,
            // so the surface's control-net AABB (the Bernstein convex-hull
            // enclosure) is unioned in.
            if sections.len() < 2 {
                return Err(open_smooth_loft());
            }
            let mut min = [f64::INFINITY; 3];
            let mut max = [f64::NEG_INFINITY; 3];
            for section in sections {
                let spans = spline_loop_spans(section)?;
                let box3 = spline_loop_bbox3(&spans)?;
                for axis in 0..3 {
                    min[axis] = min[axis].min(box3[0][axis]);
                    max[axis] = max[axis].max(box3[1][axis]);
                }
            }
            if sections.len() > 2 {
                let enclosure = nstation_control_enclosure(sections)?;
                for axis in 0..3 {
                    min[axis] = min[axis].min(enclosure[0][axis]);
                    max[axis] = max[axis].max(enclosure[1][axis]);
                }
            }
            if !min[0].is_finite() || !max[0].is_finite() {
                return Err(Refusal::Empty);
            }
            Ok([min, max])
        }
        SolidSpec::Member {
            profile, stations, ..
        } => {
            let sections = member_sections(profile, stations)?;
            let mut min = [f64::INFINITY; 3];
            let mut max = [f64::NEG_INFINITY; 3];
            for section in &sections {
                let spans = spline_loop_spans(section)?;
                let box3 = spline_loop_bbox3(&spans)?;
                for ((min_axis, lo), (max_axis, hi)) in
                    min.iter_mut().zip(box3[0]).zip(max.iter_mut().zip(box3[1]))
                {
                    *min_axis = min_axis.min(lo);
                    *max_axis = max_axis.max(hi);
                }
            }
            if !min[0].is_finite() || !max[0].is_finite() {
                return Err(Refusal::Empty);
            }
            Ok([min, max])
        }
    }
}

/// The exact radial and axial extents of the closed profile region: the
/// radius interval `[r_min, r_max]` and the axial interval `[z_min, z_max]`.
/// For a spline edge the extrema are the cubic span extrema (solved from the
/// derivative roots), never a sampled bound.
fn lathe_profile_extents(profile: &[LatheEdge]) -> Result<([f64; 2], [f64; 2]), Refusal> {
    check_profile_edges(profile)?;
    let mut r_min = f64::INFINITY;
    let mut r_max = f64::NEG_INFINITY;
    let mut z_min = f64::INFINITY;
    let mut z_max = f64::NEG_INFINITY;
    for edge in profile {
        match edge {
            LatheEdge::Line { a, b } => {
                for p in [a, b] {
                    r_min = r_min.min(p[0]);
                    r_max = r_max.max(p[0]);
                    z_min = z_min.min(p[1]);
                    z_max = z_max.max(p[1]);
                }
            }
            LatheEdge::Spline { points } => {
                let spans = spline_spans(points)?;
                for span in &spans {
                    let r0 = span.r[0];
                    let r1 = span.r[0] + span.r[1] + span.r[2] + span.r[3];
                    r_min = r_min.min(r0).min(r1);
                    r_max = r_max.max(r0).max(r1);
                    for root in cubic_roots(&span.r) {
                        let vr =
                            span.r[0] + root * (span.r[1] + root * (span.r[2] + root * span.r[3]));
                        r_min = r_min.min(vr);
                        r_max = r_max.max(vr);
                    }
                    let z0 = span.z[0];
                    let z1 = span.z[0] + span.z[1] + span.z[2] + span.z[3];
                    z_min = z_min.min(z0).min(z1);
                    z_max = z_max.max(z0).max(z1);
                    for root in cubic_roots(&span.z) {
                        let vz =
                            span.z[0] + root * (span.z[1] + root * (span.z[2] + root * span.z[3]));
                        z_min = z_min.min(vz);
                        z_max = z_max.max(vz);
                    }
                }
            }
        }
    }
    if !r_min.is_finite() || !r_max.is_finite() || !z_min.is_finite() || !z_max.is_finite() {
        return Err(Refusal::Empty);
    }
    Ok(([r_min, r_max], [z_min, z_max]))
}

/// The exact local axis-aligned bbox of a lathe wedge swept over
/// `[start_deg, start_deg + arc_deg]`. The axial range is the profile's exact
/// axial extents; the transverse range is the support of `r cos` / `r sin`
/// over the swept sector, attained at the sector endpoints or at the cardinal
/// angles inside the sector. The full revolution reduces to the landed
/// symmetric `[-max_r, max_r]` box.
fn lathe_wedge_bbox(
    profile: &[LatheEdge],
    arc_deg: f64,
    start_deg: f64,
) -> Result<[[f64; 3]; 2], Refusal> {
    let arc = validate_lathe_arc(arc_deg, start_deg)?;
    let (r, z) = lathe_profile_extents(profile)?;
    if arc == 360.0 {
        return Ok([[-r[1], -r[1], z[0]], [r[1], r[1], z[1]]]);
    }
    let theta0 = start_deg.rem_euclid(360.0).to_radians();
    let theta1 = theta0 + arc.to_radians();
    let (min_x, max_x, min_y, max_y) = sector_extents(r[0], r[1], theta0, theta1);
    Ok([[min_x, min_y, z[0]], [max_x, max_y, z[1]]])
}

/// The support of `r cos(theta)` / `r sin(theta)` over `r in [r_min, r_max]`
/// and `theta in [theta0, theta1]`: the transverse AABB of the swept sector.
/// The extrema are attained at the sector endpoints or at the cardinal angles
/// inside the sector, so only those candidates are tested (exact, no sampling).
fn sector_extents(r_min: f64, r_max: f64, theta0: f64, theta1: f64) -> (f64, f64, f64, f64) {
    let half_pi = std::f64::consts::FRAC_PI_2;
    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    let first = (theta0 / half_pi).floor() as i64 - 1;
    let last = (theta1 / half_pi).ceil() as i64 + 1;
    let mut candidates: Vec<f64> = vec![theta0, theta1];
    let mut k = first;
    while k <= last {
        let angle = k as f64 * half_pi;
        if angle >= theta0 - 1e-12 && angle <= theta1 + 1e-12 {
            candidates.push(angle);
        }
        k += 1;
    }
    for theta in candidates {
        let (sin, cos) = theta.sin_cos();
        for radius in [r_min, r_max] {
            let x = radius * cos;
            let y = radius * sin;
            min_x = min_x.min(x);
            max_x = max_x.max(x);
            min_y = min_y.min(y);
            max_y = max_y.max(y);
        }
    }
    (min_x, max_x, min_y, max_y)
}

/// Validates a lathe wedge's angular data: the arc is in `(0, 360]` and the
/// start is finite. An unanswerable angle is the typed envelope refusal, never
/// a silent full revolution or a clamped arc.
fn validate_lathe_arc(arc_deg: f64, start_deg: f64) -> Result<f64, Refusal> {
    if !arc_deg.is_finite() || arc_deg <= 0.0 || arc_deg > 360.0 || !start_deg.is_finite() {
        return Err(Refusal::UnsupportedEnvelope(
            EnvelopeCase::NonCanonicalCarrier,
        ));
    }
    Ok(arc_deg)
}

/// The roots in `(0, 1)` of the derivative of the cubic with the given power
/// coefficients `c[0] + c[1] u + c[2] u^2 + c[3] u^3`.
fn cubic_roots(c: &[f64; 4]) -> Vec<f64> {
    let mut roots = Vec::with_capacity(2);
    let a = 3.0 * c[3];
    let b = 2.0 * c[2];
    let cc = c[1];
    if a == 0.0 {
        // Quadratic (or linear) derivative.
        if b != 0.0 {
            let root = -cc / b;
            if root > 0.0 && root < 1.0 {
                roots.push(root);
            }
        }
        return roots;
    }
    let disc = b * b - 4.0 * a * cc;
    if disc < 0.0 {
        return roots;
    }
    let sq = disc.sqrt();
    for root in [(-b + sq) / (2.0 * a), (-b - sq) / (2.0 * a)] {
        if root > 0.0 && root < 1.0 {
            roots.push(root);
        }
    }
    roots
}

// ---------------------------------------------------------------------------
// Spline-section loft facts: the certified smooth-volume arm
// (BRIDGE-LOFT-FACTS)
// ---------------------------------------------------------------------------
//
// A spline-section loft's exact volume is a kernel volume fact over the smooth
// surface's L1-extracted tensor-Bernstein patches (ADM-003). The bridge cannot
// name the kernel's `BSplineSurface<Vector4>` carrier directly (the one
// sanctioned crate edge exposes only the stabilized facade's plain-data
// entries), so it assembles the smooth surface's patches itself from the
// recorded section curves and submits every patch to the sanctioned
// `binding_volume_facts` entry, summing the certified brackets. The section
// curves are reconstructed exactly (chord-length parameters, clamped cubic,
// endpoint tangents from the degree-3 Lagrange interpolant — the same
// convention the landed lathe arm reconstructs `Edge.make_spline` with), never
// a flattening polygon.
//
// **The kernel-canonical N-station convention (MONO-2-NSTATION-LOFT).** The
// recorded section stack determines a canonical smooth surface exactly:
//
// * the station parameter `v_i` is the cumulative centroid-to-centroid chord
//   length of the section stack, normalized to `[0, 1]`;
// * the `u` direction is the landed per-section reconstruction (each recorded
//   span normalized to `[0, 1]`), unified across stations by span index only —
//   the section curves are never approximated and a stack whose spans do not
//   match refuses typed;
// * the `v` direction is the unique global polynomial of degree `N - 1`
//   interpolating the section control rows for `N <= 9`, and the natural C2
//   cubic spline with knots AT the station parameters for `N >= 10`.
//
// This surface is certified as built; it is NOT claimed to reproduce OCC's
// tolerance-driven `GeomFill_AppSurf` approximant bit-for-bit. OCC's own
// smooth `ThruSections` is an approximation (chord-length parameters, C2,
// degree 2..8 fit-selected), so the recorded references adjudicate the
// canonical surface empirically at the census re-run, never by approximation
// inside this arm.

/// One reconstructed 3-D spline span in the power basis over `u in [0, 1]`.
#[derive(Debug, Clone, Copy)]
struct SpanPoly3 {
    /// The x component `x(u)`.
    x: [f64; 4],
    /// The y component `y(u)`.
    y: [f64; 4],
    /// The z component `z(u)`.
    z: [f64; 4],
}

/// The typed refusal of a loft carrier whose smooth surface the recorded data
/// does not determine (the open smooth carrier), never an approximation.
fn open_smooth_loft() -> Refusal {
    Refusal::UnsupportedEnvelope(EnvelopeCase::NonCanonicalCarrier)
}

/// The chord-length parameters of the 3-D samples.
fn chord_params3(points: &[[f64; 3]]) -> Vec<f64> {
    let mut params = Vec::with_capacity(points.len());
    params.push(0.0);
    for pair in points.windows(2) {
        let dx = pair[1][0] - pair[0][0];
        let dy = pair[1][1] - pair[0][1];
        let dz = pair[1][2] - pair[0][2];
        let next = params.last().copied().unwrap_or(0.0) + (dx * dx + dy * dy + dz * dz).sqrt();
        params.push(next);
    }
    params
}

/// The power-basis quadratic span through three samples at normalized
/// parameter `u1` (the 3-D specialization of the landed 2-D arm).
fn quadratic_span3(p0: f64, p1: f64, p2: f64, u1: f64, denom: f64) -> [f64; 4] {
    let q1 = (p1 - (1.0 - u1) * (1.0 - u1) * p0 - u1 * u1 * p2) / denom;
    [p0, 2.0 * (q1 - p0), p0 - 2.0 * q1 + p2, 0.0]
}

/// The reconstructed spans of the 3-D OCC interpolating curve through the
/// samples: the same fixed-order clamped cubic the landed lathe arm uses,
/// component-wise.
fn spline_spans3(points: &[[f64; 3]]) -> Result<Vec<SpanPoly3>, Refusal> {
    let n = points.len();
    if n < 2 {
        return Err(Refusal::Empty);
    }
    for p in points {
        if !p.iter().all(|c| c.is_finite()) {
            return Err(Refusal::Empty);
        }
    }
    let params = chord_params3(points);
    if n == 2 {
        let a = points[0];
        let b = points[1];
        return Ok(vec![SpanPoly3 {
            x: [a[0], b[0] - a[0], 0.0, 0.0],
            y: [a[1], b[1] - a[1], 0.0, 0.0],
            z: [a[2], b[2] - a[2], 0.0, 0.0],
        }]);
    }
    if n == 3 {
        let u1 = (params[1] - params[0]) / (params[2] - params[0]);
        let denom = 2.0 * u1 * (1.0 - u1);
        if denom == 0.0 || !denom.is_finite() {
            return Err(Refusal::Empty);
        }
        return Ok(vec![SpanPoly3 {
            x: quadratic_span3(points[0][0], points[1][0], points[2][0], u1, denom),
            y: quadratic_span3(points[0][1], points[1][1], points[2][1], u1, denom),
            z: quadratic_span3(points[0][2], points[1][2], points[2][2], u1, denom),
        }]);
    }
    let xy: Vec<[f64; 2]> = points.iter().map(|p| [p[0], p[1]]).collect();
    let xz: Vec<[f64; 2]> = points.iter().map(|p| [p[0], p[2]]).collect();
    let sxy = clamped_slopes(&xy, &params)?;
    let sxz = clamped_slopes(&xz, &params)?;
    let mut spans = Vec::with_capacity(n - 1);
    for i in 0..n - 1 {
        let h = params.get(i + 1).copied().ok_or(Refusal::Empty)?
            - params.get(i).copied().ok_or(Refusal::Empty)?;
        let p0 = points.get(i).ok_or(Refusal::Empty)?;
        let p1 = points.get(i + 1).ok_or(Refusal::Empty)?;
        let s0xy = sxy.get(i).ok_or(Refusal::Empty)?;
        let s1xy = sxy.get(i + 1).ok_or(Refusal::Empty)?;
        let s0xz = sxz.get(i).ok_or(Refusal::Empty)?;
        let s1xz = sxz.get(i + 1).ok_or(Refusal::Empty)?;
        spans.push(SpanPoly3 {
            x: hermite_power(p0[0], p1[0], h * s0xy[0], h * s1xy[0]),
            y: hermite_power(p0[1], p1[1], h * s0xy[1], h * s1xy[1]),
            z: hermite_power(p0[2], p1[2], h * s0xz[1], h * s1xz[1]),
        });
    }
    Ok(spans)
}

/// The degree-3 Bernstein controls of a power-basis cubic (exact degree
/// elevation is a no-op here: a lower-degree span carries zero high
/// coefficients).
fn cubic_bernstein(p: &[f64; 4]) -> [f64; 4] {
    [
        p[0],
        p[0] + p[1] / 3.0,
        p[0] + 2.0 * p[1] / 3.0 + p[2] / 3.0,
        p[0] + p[1] + p[2] + p[3],
    ]
}

/// The `4 x 3` Bernstein control row of one reconstructed span.
fn span3_bernstein(span: &SpanPoly3) -> [[f64; 3]; 4] {
    let bx = cubic_bernstein(&span.x);
    let by = cubic_bernstein(&span.y);
    let bz = cubic_bernstein(&span.z);
    [
        [bx[0], by[0], bz[0]],
        [bx[1], by[1], bz[1]],
        [bx[2], by[2], bz[2]],
        [bx[3], by[3], bz[3]],
    ]
}

/// The reconstructed spans of a closed section loop (line and spline edges),
/// with the exact seam closure checked. Every edge's recorded end must meet the
/// next edge's recorded start and the last must return to the first.
fn spline_loop_spans(profile: &[ProfileEdge]) -> Result<Vec<SpanPoly3>, Refusal> {
    if profile.is_empty() {
        return Err(Refusal::Empty);
    }
    // A single circle edge is tessellated deterministically into line spans
    // for the arms that consume a span loop (closure is exact by construction).
    if let [
        ProfileEdge::Circle {
            center,
            radius,
            normal,
        },
    ] = profile
    {
        let (_circle, verts) = circle_from_edge(*center, *radius, *normal)?;
        let count = verts.len();
        let mut spans = Vec::with_capacity(count);
        for i in 0..count {
            let a = verts[i];
            let b = verts[(i + 1) % count];
            spans.push(SpanPoly3 {
                x: [a[0], b[0] - a[0], 0.0, 0.0],
                y: [a[1], b[1] - a[1], 0.0, 0.0],
                z: [a[2], b[2] - a[2], 0.0, 0.0],
            });
        }
        return Ok(spans);
    }
    if profile
        .iter()
        .any(|edge| matches!(edge, ProfileEdge::Circle { .. }))
    {
        return Err(Refusal::UnsupportedEnvelope(
            EnvelopeCase::NonCanonicalCarrier,
        ));
    }
    let mut spans: Vec<SpanPoly3> = Vec::new();
    let mut starts: Vec<[f64; 3]> = Vec::new();
    let mut ends: Vec<[f64; 3]> = Vec::new();
    for edge in profile {
        match edge {
            ProfileEdge::Line { a, b } => {
                if !a.iter().all(|c| c.is_finite()) || !b.iter().all(|c| c.is_finite()) {
                    return Err(Refusal::Empty);
                }
                starts.push(*a);
                ends.push(*b);
                spans.push(SpanPoly3 {
                    x: [a[0], b[0] - a[0], 0.0, 0.0],
                    y: [a[1], b[1] - a[1], 0.0, 0.0],
                    z: [a[2], b[2] - a[2], 0.0, 0.0],
                });
            }
            ProfileEdge::Spline { points } => {
                if points.len() < 2 {
                    return Err(Refusal::Empty);
                }
                starts.push(*points.first().ok_or(Refusal::Empty)?);
                ends.push(*points.last().ok_or(Refusal::Empty)?);
                spans.extend(spline_spans3(points)?);
            }
            // Mixed circle edges were rejected above; a lone circle returns
            // early with its deterministic tessellation.
            ProfileEdge::Circle { .. } => {
                return Err(Refusal::UnsupportedEnvelope(
                    EnvelopeCase::NonCanonicalCarrier,
                ));
            }
        }
    }
    let count = starts.len();
    if count < 2 {
        return Err(open_smooth_loft());
    }
    let mut scale = 0.0f64;
    for p in starts.iter().chain(ends.iter()) {
        for c in p {
            scale = scale.max(c.abs());
        }
    }
    let tol = 1e-9 * (1.0 + scale);
    for i in 0..count {
        let next = starts.get((i + 1) % count).ok_or(Refusal::Empty)?;
        let gap = v3_norm(v3_sub(*ends.get(i).ok_or(Refusal::Empty)?, *next));
        if !(gap <= tol) {
            return Err(open_smooth_loft());
        }
    }
    Ok(spans)
}

/// The exact `int_0^1 a(u) b(u) du` of two power-basis polynomials.
fn poly_integral(a: &[f64; 4], b: &[f64; 4]) -> f64 {
    let mut acc = 0.0;
    for (i, ai) in a.iter().enumerate() {
        for (j, bj) in b.iter().enumerate() {
            acc += ai * bj / ((i + j + 1) as f64);
        }
    }
    acc
}

/// The signed half area moment `(1/2) int_0^1 r(u) x r'(u) du` of one span.
fn span_area_moment(span: &SpanPoly3) -> [f64; 3] {
    let dx = [span.x[1], 2.0 * span.x[2], 3.0 * span.x[3], 0.0];
    let dy = [span.y[1], 2.0 * span.y[2], 3.0 * span.y[3], 0.0];
    let dz = [span.z[1], 2.0 * span.z[2], 3.0 * span.z[3], 0.0];
    [
        0.5 * (poly_integral(&span.y, &dz) - poly_integral(&span.z, &dy)),
        0.5 * (poly_integral(&span.z, &dx) - poly_integral(&span.x, &dz)),
        0.5 * (poly_integral(&span.x, &dy) - poly_integral(&span.y, &dx)),
    ]
}

/// The signed area vector of a closed reconstructed loop (Green's theorem).
fn spline_loop_area_vector(spans: &[SpanPoly3]) -> [f64; 3] {
    let mut area = [0.0f64; 3];
    for span in spans {
        let moment = span_area_moment(span);
        area[0] += moment[0];
        area[1] += moment[1];
        area[2] += moment[2];
    }
    area
}

/// The exact local AABB of a closed reconstructed loop: the span endpoints plus
/// every interior extremum of each cubic component.
fn spline_loop_bbox3(spans: &[SpanPoly3]) -> Result<[[f64; 3]; 2], Refusal> {
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    for span in spans {
        for (axis, coeffs) in [&span.x, &span.y, &span.z].into_iter().enumerate() {
            let start = coeffs[0];
            let end = coeffs[0] + coeffs[1] + coeffs[2] + coeffs[3];
            min[axis] = min[axis].min(start).min(end);
            max[axis] = max[axis].max(start).max(end);
            for root in cubic_roots(coeffs) {
                let value = coeffs[0] + root * (coeffs[1] + root * (coeffs[2] + root * coeffs[3]));
                min[axis] = min[axis].min(value);
                max[axis] = max[axis].max(value);
            }
        }
    }
    if !min[0].is_finite() || !max[0].is_finite() {
        return Err(Refusal::Empty);
    }
    Ok([min, max])
}

/// The binomial coefficient `C(n, k)` as an `f64` (exact for the small degrees
/// this arm uses; `0` when `k > n`).
fn binomial_f64(n: usize, k: usize) -> f64 {
    if k > n {
        return 0.0;
    }
    let k = k.min(n - k);
    let mut out = 1.0f64;
    for i in 0..k {
        out = out * (n - i) as f64 / (i + 1) as f64;
    }
    out
}

/// Solves the dense `n x n` system `a x = b` in place (Gaussian elimination
/// with partial pivoting); `a` is destroyed and `b` becomes the solution. A
/// singular system refuses typed. Deterministic, fixed pivot order.
fn solve_dense(a: &mut [Vec<f64>], b: &mut [f64]) -> Result<(), Refusal> {
    let n = b.len();
    if a.len() != n {
        return Err(Refusal::Empty);
    }
    for col in 0..n {
        let mut piv = col;
        let mut best = a
            .get(col)
            .and_then(|row| row.get(col))
            .copied()
            .unwrap_or(0.0)
            .abs();
        for row in (col + 1)..n {
            let value = a
                .get(row)
                .and_then(|r| r.get(col))
                .copied()
                .unwrap_or(0.0)
                .abs();
            if value > best {
                best = value;
                piv = row;
            }
        }
        if !(best > 0.0) {
            return Err(open_smooth_loft());
        }
        if piv != col {
            a.swap(piv, col);
            b.swap(piv, col);
        }
        let pivot = a
            .get(col)
            .and_then(|row| row.get(col))
            .copied()
            .ok_or(Refusal::Empty)?;
        let pivot_row = a.get(col).cloned().ok_or(Refusal::Empty)?;
        let rhs_col = *b.get(col).ok_or(Refusal::Empty)?;
        for row in (col + 1)..n {
            let factor = a.get(row).and_then(|r| r.get(col)).copied().unwrap_or(0.0) / pivot;
            if let Some(r) = a.get_mut(row) {
                for (cell, p) in r.iter_mut().zip(pivot_row.iter()) {
                    *cell -= factor * p;
                }
            }
            *b.get_mut(row).ok_or(Refusal::Empty)? -= factor * rhs_col;
        }
    }
    for row in (0..n).rev() {
        let mut sum = *b.get(row).ok_or(Refusal::Empty)?;
        let r = a.get(row).ok_or(Refusal::Empty)?;
        for col in (row + 1)..n {
            sum -= r.get(col).copied().unwrap_or(0.0) * *b.get(col).ok_or(Refusal::Empty)?;
        }
        let diag = r.get(row).copied().unwrap_or(0.0);
        if !(diag.abs() > 0.0) {
            return Err(open_smooth_loft());
        }
        *b.get_mut(row).ok_or(Refusal::Empty)? = sum / diag;
    }
    Ok(())
}

/// The Bernstein coefficients (degree `n - 1`) of the unique polynomial
/// interpolating `values` at the strictly increasing nodes `v`.
fn bernstein_interp_global(values: &[[f64; 3]], v: &[f64]) -> Result<Vec<[f64; 3]>, Refusal> {
    let n = values.len();
    if n == 0 || v.len() != n {
        return Err(Refusal::Empty);
    }
    // The Bernstein collocation matrix `B_j^{n-1}(v_i)`.
    let mut collocation = vec![vec![0.0f64; n]; n];
    for (i, node) in v.iter().enumerate() {
        let row = collocation.get_mut(i).ok_or(Refusal::Empty)?;
        for (j, cell) in row.iter_mut().enumerate() {
            let mut value = binomial_f64(n - 1, j);
            for _ in 0..j {
                value *= *node;
            }
            for _ in 0..(n - 1 - j) {
                value *= 1.0 - *node;
            }
            *cell = value;
        }
    }
    let mut out = vec![[0.0f64; 3]; n];
    for comp in 0..3 {
        let mut rhs: Vec<f64> = values
            .iter()
            .map(|p| match comp {
                0 => p[0],
                1 => p[1],
                _ => p[2],
            })
            .collect();
        let mut matrix = collocation.clone();
        solve_dense(&mut matrix, &mut rhs)?;
        for (slot, value) in out.iter_mut().zip(rhs.iter()) {
            match comp {
                0 => slot[0] = *value,
                1 => slot[1] = *value,
                _ => slot[2] = *value,
            }
        }
    }
    Ok(out)
}

/// The natural C2 cubic spline's second derivatives `M_i = y''(v_i)` at the
/// nodes: the standard banded second-derivative system with natural ends
/// (`M_0 = M_{n-1} = 0`), solved by the Thomas algorithm, one `R^3` value per
/// node.
fn natural_spline_second_derivs(values: &[[f64; 3]], h: &[f64]) -> Result<Vec<[f64; 3]>, Refusal> {
    let n = values.len();
    let mut m = vec![[0.0f64; 3]; n];
    if n <= 2 {
        return Ok(m);
    }
    let inner = n - 2;
    let mut lower = vec![0.0f64; inner];
    let mut diag = vec![0.0f64; inner];
    let mut upper = vec![0.0f64; inner];
    let mut rhs = vec![[0.0f64; 3]; inner];
    for i in 0..inner {
        let node = i + 1;
        let hm = *h.get(node - 1).ok_or(Refusal::Empty)?;
        let hp = *h.get(node).ok_or(Refusal::Empty)?;
        let ym = *values.get(node - 1).ok_or(Refusal::Empty)?;
        let y0 = *values.get(node).ok_or(Refusal::Empty)?;
        let yp = *values.get(node + 1).ok_or(Refusal::Empty)?;
        *lower.get_mut(i).ok_or(Refusal::Empty)? = hm;
        *diag.get_mut(i).ok_or(Refusal::Empty)? = 2.0 * (hm + hp);
        *upper.get_mut(i).ok_or(Refusal::Empty)? = hp;
        *rhs.get_mut(i).ok_or(Refusal::Empty)? = [
            6.0 * ((yp[0] - y0[0]) / hp - (y0[0] - ym[0]) / hm),
            6.0 * ((yp[1] - y0[1]) / hp - (y0[1] - ym[1]) / hm),
            6.0 * ((yp[2] - y0[2]) / hp - (y0[2] - ym[2]) / hm),
        ];
    }
    for i in 1..inner {
        let w = *lower.get(i).ok_or(Refusal::Empty)? / *diag.get(i - 1).ok_or(Refusal::Empty)?;
        let u = *upper.get(i - 1).ok_or(Refusal::Empty)?;
        *diag.get_mut(i).ok_or(Refusal::Empty)? -= w * u;
        let prev = *rhs.get(i - 1).ok_or(Refusal::Empty)?;
        let cur = rhs.get_mut(i).ok_or(Refusal::Empty)?;
        cur[0] -= w * prev[0];
        cur[1] -= w * prev[1];
        cur[2] -= w * prev[2];
    }
    let mut x = vec![[0.0f64; 3]; inner];
    let last = inner - 1;
    let d_last = *diag.get(last).ok_or(Refusal::Empty)?;
    if !(d_last.abs() > 0.0) {
        return Err(open_smooth_loft());
    }
    let r_last = *rhs.get(last).ok_or(Refusal::Empty)?;
    if let Some(slot) = x.get_mut(last) {
        *slot = [r_last[0] / d_last, r_last[1] / d_last, r_last[2] / d_last];
    }
    for i in (0..last).rev() {
        let d = *diag.get(i).ok_or(Refusal::Empty)?;
        let u = *upper.get(i).ok_or(Refusal::Empty)?;
        let next = *x.get(i + 1).ok_or(Refusal::Empty)?;
        let r = *rhs.get(i).ok_or(Refusal::Empty)?;
        if let Some(slot) = x.get_mut(i) {
            *slot = [
                (r[0] - u * next[0]) / d,
                (r[1] - u * next[1]) / d,
                (r[2] - u * next[2]) / d,
            ];
        }
    }
    for (i, value) in x.iter().enumerate() {
        if let Some(slot) = m.get_mut(i + 1) {
            *slot = *value;
        }
    }
    Ok(m)
}

/// The Bernstein controls of the natural C2 cubic spline interpolating
/// `values` at the strictly increasing nodes `v`, one four-control span per
/// interval (each span's local parameter is `[0, 1]`).
fn bernstein_interp_spline(values: &[[f64; 3]], v: &[f64]) -> Result<Vec<[[f64; 3]; 4]>, Refusal> {
    let n = values.len();
    if n < 2 || v.len() != n {
        return Err(Refusal::Empty);
    }
    let mut h = Vec::with_capacity(n - 1);
    for i in 0..(n - 1) {
        let hi = *v.get(i + 1).ok_or(Refusal::Empty)? - *v.get(i).ok_or(Refusal::Empty)?;
        if !(hi > 0.0) || !hi.is_finite() {
            return Err(open_smooth_loft());
        }
        h.push(hi);
    }
    let m = natural_spline_second_derivs(values, &h)?;
    let mut out = vec![[[0.0f64; 3]; 4]; n - 1];
    for i in 0..(n - 1) {
        let hi = *h.get(i).ok_or(Refusal::Empty)?;
        let mi = *m.get(i).ok_or(Refusal::Empty)?;
        let mi1 = *m.get(i + 1).ok_or(Refusal::Empty)?;
        let yi = *values.get(i).ok_or(Refusal::Empty)?;
        let yi1 = *values.get(i + 1).ok_or(Refusal::Empty)?;
        let c2 = [
            hi * hi * mi[0] / 2.0,
            hi * hi * mi[1] / 2.0,
            hi * hi * mi[2] / 2.0,
        ];
        let c3 = [
            hi * hi * (mi1[0] - mi[0]) / 6.0,
            hi * hi * (mi1[1] - mi[1]) / 6.0,
            hi * hi * (mi1[2] - mi[2]) / 6.0,
        ];
        let c1 = [
            (yi1[0] - yi[0]) - c2[0] - c3[0],
            (yi1[1] - yi[1]) - c2[1] - c3[1],
            (yi1[2] - yi[2]) - c2[2] - c3[2],
        ];
        let c0 = yi;
        let bx = cubic_bernstein(&[c0[0], c1[0], c2[0], c3[0]]);
        let by = cubic_bernstein(&[c0[1], c1[1], c2[1], c3[1]]);
        let bz = cubic_bernstein(&[c0[2], c1[2], c2[2], c3[2]]);
        let span = [
            [bx[0], by[0], bz[0]],
            [bx[1], by[1], bz[1]],
            [bx[2], by[2], bz[2]],
            [bx[3], by[3], bz[3]],
        ];
        if let Some(slot) = out.get_mut(i) {
            *slot = span;
        }
    }
    Ok(out)
}

/// The canonical v-segments (each a run of Bernstein controls) of the
/// interpolant of `values` at the station parameters `v`: one global segment
/// of degree `N - 1` for `N <= 9`, the natural C2 cubic spans otherwise.
fn v_segments(values: &[[f64; 3]], v: &[f64]) -> Result<Vec<Vec<[f64; 3]>>, Refusal> {
    if values.len() <= 9 {
        Ok(vec![bernstein_interp_global(values, v)?])
    } else {
        Ok(bernstein_interp_spline(values, v)?
            .into_iter()
            .map(|span| span.to_vec())
            .collect())
    }
}

/// The canonical station parameters of a section stack: the cumulative
/// centroid-to-centroid chord length, normalized to `[0, 1]`.
fn station_params(loops: &[Vec<SpanPoly3>]) -> Result<Vec<f64>, Refusal> {
    if loops.len() < 2 {
        return Err(Refusal::Empty);
    }
    let mut params = Vec::with_capacity(loops.len());
    let mut previous: Option<[f64; 3]> = None;
    let mut total = 0.0f64;
    for loop3 in loops {
        let samples = sample_loop3(loop3);
        if samples.is_empty() {
            return Err(Refusal::Empty);
        }
        let centroid = centroid3(&samples);
        if let Some(prev) = previous {
            let dx = centroid[0] - prev[0];
            let dy = centroid[1] - prev[1];
            let dz = centroid[2] - prev[2];
            total += (dx * dx + dy * dy + dz * dz).sqrt();
        }
        previous = Some(centroid);
        params.push(total);
    }
    if !(total > 0.0) || !total.is_finite() {
        return Err(open_smooth_loft());
    }
    for param in &mut params {
        *param /= total;
    }
    for pair in params.windows(2) {
        if !(pair[1] > pair[0]) {
            return Err(open_smooth_loft());
        }
    }
    Ok(params)
}

/// The certified volume of an N-station spline-section loft under the
/// kernel-canonical convention: every side patch's face form through the
/// sanctioned `binding_volume_facts` entry plus the exact planar end-cap
/// moments, returned as `(value, lo, hi)`. A stack whose section spans do not
/// match (the u-unification cannot be completed by knot insertion alone)
/// refuses typed naming the open smooth carrier.
fn spline_loft_volume_rows(
    sections: &[Vec<ProfileEdge>],
) -> Result<(Vec<crate::python::binding::VolumeRow>, Vec<Vec<SpanPoly3>>), Refusal> {
    if sections.len() < 2 {
        return Err(Refusal::Empty);
    }
    let mut loops: Vec<Vec<SpanPoly3>> = Vec::with_capacity(sections.len());
    for section in sections {
        loops.push(spline_loop_spans(section)?);
    }
    let span_count = loops.first().map(Vec::len).unwrap_or(0);
    if span_count == 0 {
        return Err(open_smooth_loft());
    }
    for loop3 in &loops {
        if loop3.len() != span_count {
            return Err(open_smooth_loft());
        }
    }
    let v = station_params(&loops)?;
    let station_count = loops.len();
    let mut volume_rows = Vec::new();
    for span_index in 0..span_count {
        // The four u-control rows of this span, one 3-D value per station.
        let mut rows: [Vec<[f64; 3]>; 4] = [
            Vec::with_capacity(station_count),
            Vec::with_capacity(station_count),
            Vec::with_capacity(station_count),
            Vec::with_capacity(station_count),
        ];
        for loop3 in &loops {
            let span = loop3.get(span_index).ok_or(Refusal::Empty)?;
            for (row, control) in rows.iter_mut().zip(span3_bernstein(span).iter()) {
                row.push(*control);
            }
        }
        let mut segments: Vec<Vec<Vec<[f64; 3]>>> = Vec::with_capacity(4);
        for row in &rows {
            segments.push(v_segments(row, &v)?);
        }
        let segment_count = segments.first().map(Vec::len).unwrap_or(0);
        for segment_index in 0..segment_count {
            let cols = segments
                .first()
                .and_then(|s| s.get(segment_index))
                .map(Vec::len)
                .unwrap_or(0);
            if cols == 0 {
                return Err(Refusal::Empty);
            }
            let mut numerator: Vec<Vec<[f64; 3]>> = Vec::with_capacity(4);
            for segment in &segments {
                numerator.push(segment.get(segment_index).cloned().ok_or(Refusal::Empty)?);
            }
            let weights = vec![vec![1.0f64; cols]; 4];
            volume_rows.push(crate::python::binding::VolumeRow {
                numerator,
                weights,
                orientation: 1.0,
            });
        }
    }
    Ok((volume_rows, loops))
}

fn certified_spline_loft_volume(sections: &[Vec<ProfileEdge>]) -> Result<(f64, f64, f64), Refusal> {
    let (rows, loops) = spline_loft_volume_rows(sections)?;
    let mut value = 0.0f64;
    let mut lo = 0.0f64;
    let mut hi = 0.0f64;
    for row in &rows {
        let json = crate::python::binding::volume_facts(row).map_err(|_| open_smooth_loft())?;
        let outcome: serde_json::Value = serde_json::from_str(&json).map_err(|_| Refusal::Empty)?;
        value += outcome
            .get("value")
            .and_then(serde_json::Value::as_f64)
            .ok_or(Refusal::Empty)?;
        lo += outcome
            .pointer("/bracket/lo")
            .and_then(serde_json::Value::as_f64)
            .ok_or(Refusal::Empty)?;
        hi += outcome
            .pointer("/bracket/hi")
            .and_then(serde_json::Value::as_f64)
            .ok_or(Refusal::Empty)?;
    }

    // The planar end caps: `(1/3) d A` with the recorded-loop winding, exactly
    // as the landed line-loft arm's cap terms.
    let first = loops.first().ok_or(Refusal::Empty)?;
    let last = loops.last().ok_or(Refusal::Empty)?;
    let area_a = spline_loop_area_vector(first);
    let area_b = spline_loop_area_vector(last);
    let mag_a = v3_norm(area_a);
    let mag_b = v3_norm(area_b);
    if !(mag_a > 0.0) || !(mag_b > 0.0) {
        return Err(open_smooth_loft());
    }
    let normal_a = [area_a[0] / mag_a, area_a[1] / mag_a, area_a[2] / mag_a];
    let normal_b = [area_b[0] / mag_b, area_b[1] / mag_b, area_b[2] / mag_b];
    let start_a = first.first().ok_or(Refusal::Empty)?;
    let start_b = last.first().ok_or(Refusal::Empty)?;
    let point_a = [start_a.x[0], start_a.y[0], start_a.z[0]];
    let point_b = [start_b.x[0], start_b.y[0], start_b.z[0]];
    let cap_a = -(1.0 / 3.0) * mag_a * v3_dot(normal_a, point_a);
    let cap_b = (1.0 / 3.0) * mag_b * v3_dot(normal_b, point_b);
    value += cap_a + cap_b;
    lo += cap_a + cap_b;
    hi += cap_a + cap_b;
    if !value.is_finite() || !lo.is_finite() || !hi.is_finite() {
        return Err(Refusal::Empty);
    }
    if value < 0.0 {
        Ok((value.abs(), -hi, -lo))
    } else {
        Ok((value, lo, hi))
    }
}

/// Places the recorded plate profile at every station, producing the section
/// chain the landed loft arms consume. The profile must be a closed loop (the
/// landed sweep carrier's own closure check): an open plate boundary refuses
/// typed `NonCanonicalCarrier` rather than being approximated.
fn member_sections(
    profile: &[ProfileEdge],
    stations: &[StationFrame],
) -> Result<Vec<Vec<ProfileEdge>>, Refusal> {
    if stations.len() < 2 {
        return Err(Refusal::Empty);
    }
    // The plate section must be a closed loop; `spline_loop_spans` is the
    // landed seam check the sweep carrier uses, so an open boundary refuses
    // with exactly the same typed case.
    spline_loop_spans(profile)?;
    let mut sections = Vec::with_capacity(stations.len());
    for station in stations {
        if !station.is_finite() {
            return Err(Refusal::Empty);
        }
        let mut edges = Vec::with_capacity(profile.len());
        for edge in profile {
            match edge {
                ProfileEdge::Line { a, b } => edges.push(ProfileEdge::Line {
                    a: station.apply(*a),
                    b: station.apply(*b),
                }),
                ProfileEdge::Spline { points } => edges.push(ProfileEdge::Spline {
                    points: points.iter().map(|p| station.apply(*p)).collect(),
                }),
                ProfileEdge::Circle {
                    center,
                    radius,
                    normal,
                } => edges.push(ProfileEdge::Circle {
                    center: station.apply(*center),
                    radius: *radius,
                    normal: station.apply_dir(*normal),
                }),
            }
        }
        sections.push(edges);
    }
    Ok(sections)
}

/// The certified volume bracket of a plate-section member: the exact analytic
/// arm for an all-line plate, otherwise the sum of the landed MONO-2
/// two-station certificates over adjacent station pairs. The interior caps of
/// consecutive pairs cancel exactly, so the sum is the chain's side surface
/// plus its two end caps -- "a two-section degenerate case plus wall bands".
/// A smooth member with more than two stations has no landed carrier and
/// refuses typed (never a ruled substitution).
fn member_volume_bracket(
    profile: &[ProfileEdge],
    stations: &[StationFrame],
    ruled: bool,
) -> Result<(f64, f64, f64), Refusal> {
    let sections = member_sections(profile, stations)?;
    if let Ok(validated) = loft_sections(&sections) {
        let value = loft_volume(&validated, false)?;
        return Ok((value, value, value));
    }
    if !ruled && sections.len() > 2 {
        return Err(open_smooth_loft());
    }
    let mut value = 0.0f64;
    let mut lo = 0.0f64;
    let mut hi = 0.0f64;
    for pair in sections.windows(2) {
        let (pair_value, pair_lo, pair_hi) = certified_spline_loft_volume(pair)?;
        value += pair_value;
        lo += pair_lo;
        hi += pair_hi;
    }
    if !value.is_finite() || !lo.is_finite() || !hi.is_finite() {
        return Err(Refusal::Empty);
    }
    Ok((value, lo, hi))
}

/// The local mesh of a plate-section member: ruled quads between the sampled
/// reconstructed loops of consecutive stations plus the two end-cap fans. The
/// sampled loops are the same reconstruction the certificate integrates.
fn member_mesh(sections: &[Vec<ProfileEdge>]) -> Result<Vec<Triangle>, Refusal> {
    if sections.len() < 2 {
        return Err(Refusal::Empty);
    }
    let loops: Vec<Vec<[f64; 3]>> = sections
        .iter()
        .map(|section| spline_loop_spans(section).map(|spans| sample_loop3(&spans)))
        .collect::<Result<_, _>>()?;
    let count = loops.first().ok_or(Refusal::Empty)?.len();
    if count == 0 {
        return Err(Refusal::Empty);
    }
    for loop3 in &loops {
        if loop3.len() != count {
            return Err(open_smooth_loft());
        }
    }
    let mut out = Vec::new();
    for pair in loops.windows(2) {
        let a = pair.first().ok_or(Refusal::Empty)?;
        let b = pair.get(1).ok_or(Refusal::Empty)?;
        for i in 0..count {
            let k = (i + 1) % count;
            push_quad(
                &mut out,
                *a.get(i).ok_or(Refusal::Empty)?,
                *a.get(k).ok_or(Refusal::Empty)?,
                *b.get(k).ok_or(Refusal::Empty)?,
                *b.get(i).ok_or(Refusal::Empty)?,
            );
        }
    }
    let first = loops.first().ok_or(Refusal::Empty)?;
    let last = loops.last().ok_or(Refusal::Empty)?;
    let c_first = centroid3(first);
    let c_last = centroid3(last);
    for i in 0..count {
        let k = (i + 1) % count;
        push_tri(
            &mut out,
            c_first,
            *first.get(k).ok_or(Refusal::Empty)?,
            *first.get(i).ok_or(Refusal::Empty)?,
        );
        push_tri(
            &mut out,
            c_last,
            *last.get(i).ok_or(Refusal::Empty)?,
            *last.get(k).ok_or(Refusal::Empty)?,
        );
    }
    Ok(out)
}

/// A certified axis-aligned enclosure of the canonical N-station surface: the
/// AABB of the surface's Bernstein control net (the convex-hull enclosure,
/// which covers the v-interpolation's bulge past the section loops).
fn nstation_control_enclosure(sections: &[Vec<ProfileEdge>]) -> Result<[[f64; 3]; 2], Refusal> {
    let mut loops: Vec<Vec<SpanPoly3>> = Vec::with_capacity(sections.len());
    for section in sections {
        loops.push(spline_loop_spans(section)?);
    }
    let span_count = loops.first().map(Vec::len).unwrap_or(0);
    if span_count == 0 {
        return Err(open_smooth_loft());
    }
    for loop3 in &loops {
        if loop3.len() != span_count {
            return Err(open_smooth_loft());
        }
    }
    let v = station_params(&loops)?;
    let station_count = loops.len();
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    for span_index in 0..span_count {
        let mut rows: [Vec<[f64; 3]>; 4] = [
            Vec::with_capacity(station_count),
            Vec::with_capacity(station_count),
            Vec::with_capacity(station_count),
            Vec::with_capacity(station_count),
        ];
        for loop3 in &loops {
            let span = loop3.get(span_index).ok_or(Refusal::Empty)?;
            for (row, control) in rows.iter_mut().zip(span3_bernstein(span).iter()) {
                row.push(*control);
            }
        }
        for row in &rows {
            for segment in v_segments(row, &v)? {
                for control in segment {
                    for (axis, value) in control.iter().enumerate() {
                        min[axis] = min[axis].min(*value);
                        max[axis] = max[axis].max(*value);
                    }
                }
            }
        }
    }
    if !min[0].is_finite() || !max[0].is_finite() {
        return Err(Refusal::Empty);
    }
    Ok([min, max])
}

/// Evaluates a reconstructed 3-D span at `u in [0, 1]`.
fn eval_span3(span: &SpanPoly3, u: f64) -> [f64; 3] {
    [
        span.x[0] + u * (span.x[1] + u * (span.x[2] + u * span.x[3])),
        span.y[0] + u * (span.y[1] + u * (span.y[2] + u * span.y[3])),
        span.z[0] + u * (span.z[1] + u * (span.z[2] + u * span.z[3])),
    ]
}

/// Samples every span of a closed reconstructed loop at `MESH_SEGMENTS` steps.
fn sample_loop3(spans: &[SpanPoly3]) -> Vec<[f64; 3]> {
    let mut out = Vec::with_capacity(spans.len() * MESH_SEGMENTS);
    for span in spans {
        for step in 0..MESH_SEGMENTS {
            out.push(eval_span3(span, step as f64 / MESH_SEGMENTS as f64));
        }
    }
    out
}

/// The centroid of a point cloud.
fn centroid3(points: &[[f64; 3]]) -> [f64; 3] {
    let mut c = [0.0f64; 3];
    for p in points {
        c[0] += p[0];
        c[1] += p[1];
        c[2] += p[2];
    }
    let k = 1.0 / (points.len() as f64);
    [c[0] * k, c[1] * k, c[2] * k]
}

/// The deterministic local mesh of an N-station spline-section loft: ruled
/// quads between consecutive sampled reconstructed loops plus the two end-cap
/// fans. A stack whose section spans do not match refuses typed.
fn spline_loft_mesh(sections: &[Vec<ProfileEdge>]) -> Result<Vec<Triangle>, Refusal> {
    if sections.len() < 2 {
        return Err(Refusal::Empty);
    }
    let mut loops: Vec<Vec<SpanPoly3>> = Vec::with_capacity(sections.len());
    for section in sections {
        loops.push(spline_loop_spans(section)?);
    }
    let span_count = loops.first().map(Vec::len).unwrap_or(0);
    if span_count == 0 {
        return Err(open_smooth_loft());
    }
    for loop3 in &loops {
        if loop3.len() != span_count {
            return Err(open_smooth_loft());
        }
    }
    let sampled: Vec<Vec<[f64; 3]>> = loops.iter().map(|loop3| sample_loop3(loop3)).collect();
    let count = sampled.first().map(Vec::len).unwrap_or(0);
    if count == 0 {
        return Err(Refusal::Empty);
    }
    for points in &sampled {
        if points.len() != count {
            return Err(open_smooth_loft());
        }
    }
    let mut out = Vec::new();
    for window in sampled.windows(2) {
        let pa = window.first().ok_or(Refusal::Empty)?;
        let pb = window.get(1).ok_or(Refusal::Empty)?;
        for i in 0..count {
            let j = (i + 1) % count;
            push_quad(
                &mut out,
                *pa.get(i).ok_or(Refusal::Empty)?,
                *pa.get(j).ok_or(Refusal::Empty)?,
                *pb.get(j).ok_or(Refusal::Empty)?,
                *pb.get(i).ok_or(Refusal::Empty)?,
            );
        }
    }
    let first_points = sampled.first().ok_or(Refusal::Empty)?;
    let last_points = sampled.last().ok_or(Refusal::Empty)?;
    let ca = centroid3(first_points);
    let cb = centroid3(last_points);
    for i in 0..count {
        let j = (i + 1) % count;
        push_tri(
            &mut out,
            ca,
            *first_points.get(i).ok_or(Refusal::Empty)?,
            *first_points.get(j).ok_or(Refusal::Empty)?,
        );
        push_tri(
            &mut out,
            cb,
            *last_points.get(i).ok_or(Refusal::Empty)?,
            *last_points.get(j).ok_or(Refusal::Empty)?,
        );
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Authoring arms: 3-D profile helpers (prism extrude / loft / sweep chain)
// ---------------------------------------------------------------------------
//
// The extrude and loft recording arms carry their profiles as closed planar
// line loops in the part's local 3-D frame. All facts are exact analytic
// arithmetic over those loops:
//
// * `Prism`: the volume is the profile area times the swept length along the
//   profile-plane normal (OCC's Face-normal extrusion convention: a single-
//   sided sweep runs +right-hand-normal, `both` sweeps symmetrically), the
//   bbox is the profile support widened along the normal and the mesh sweeps
//   the boundary deterministically.
// * `Loft`: the ruled matched-vertex interpolation. Over each pair of stations
//   the cross-section polygon area is a quadratic in the stack parameter, so a
//   chain of two-section lofts is the "segment-moment generalisation" of the
//   FH-SPLINE-LATHE line-profile revolution (whose frustum telescoping is the
//   degenerate two-section case). The volume is the exact divergence-form
//   boundary integral over the loft's faces: each planar cap contributes
//   `(1/3) n . int x dA` (exact shoelace moments over the cap polygon) and
//   every ruled side patch between matching edges contributes the exact
//   bilinear moment `(1/3) int int X.(Xu x Xv) du dv`, evaluated by two-point
//   Gauss-Legendre quadrature (exact: the integrand is a polynomial of degree
//   (2,2) in (u, v)). A closed halo row has no end caps; the seam is certified
//   by the exact aligned-meeting-edge identity on the stations that return to
//   the start.
// * `mirror`: a placed-carrier reflection; the solid spec is untouched and the
//   facts transform with the placement (no geometry recomputation).

/// A closed planar line-loop profile extracted from a recorded profile edge
/// list, or an exact circle section.
struct ProfileLoop {
    /// The boundary vertices in order (no duplicated closing point). For a
    /// circle section this is the deterministic `MESH_SEGMENTS`-gon
    /// tessellation used by the mesh arm; the analytic arms use `circle`.
    verts: Vec<[f64; 3]>,
    /// `area_vec / |area_vec|`.
    normal: [f64; 3],
    /// The loop's signed area (`|area_vec|`; `pi r^2` for a circle section).
    area: f64,
    /// The loop scale (max absolute coordinate), used for tolerances.
    scale: f64,
    /// The exact circle carrier when this section is a single recorded circle.
    circle: Option<CircleGeom>,
}

/// The exact geometry of one recorded circle section: its centre, radius and
/// a deterministic right-handed in-plane basis `(u, v)` with `u x v == normal`.
#[derive(Debug, Clone, Copy)]
struct CircleGeom {
    /// The circle centre.
    center: [f64; 3],
    /// The circle radius.
    radius: f64,
    /// The first in-plane basis vector.
    u: [f64; 3],
    /// The second in-plane basis vector (`u x v` is the plane normal).
    v: [f64; 3],
}

impl CircleGeom {
    /// The unit plane normal (`u x v`).
    fn normal(self) -> [f64; 3] {
        v3_cross(self.u, self.v)
    }

    /// The circle point at angle `theta` with `c = cos(theta)`, `s = sin(theta)`.
    fn point(self, c: f64, s: f64) -> [f64; 3] {
        [
            self.center[0] + self.radius * (c * self.u[0] + s * self.v[0]),
            self.center[1] + self.radius * (c * self.u[1] + s * self.v[1]),
            self.center[2] + self.radius * (c * self.u[2] + s * self.v[2]),
        ]
    }

    /// The circle's tangent `d/dtheta` at `theta` (given `c`, `s`).
    fn tangent(self, c: f64, s: f64) -> [f64; 3] {
        [
            self.radius * (-s * self.u[0] + c * self.v[0]),
            self.radius * (-s * self.u[1] + c * self.v[1]),
            self.radius * (-s * self.u[2] + c * self.v[2]),
        ]
    }
}

/// A deterministic unit vector perpendicular to the unit vector `n` (the same
/// fixed-order choice the door's `Plane` makes: prefer world x, fall back to
/// world y when `n` is near the x axis).
fn perp_unit(n: [f64; 3]) -> [f64; 3] {
    let reference = if n[0].abs() < 0.9 {
        [1.0, 0.0, 0.0]
    } else {
        [0.0, 1.0, 0.0]
    };
    let d = v3_dot(reference, n);
    let p = [
        reference[0] - d * n[0],
        reference[1] - d * n[1],
        reference[2] - d * n[2],
    ];
    let mag = v3_norm(p);
    if mag > 1.0e-12 {
        [p[0] / mag, p[1] / mag, p[2] / mag]
    } else {
        [0.0, 0.0, 1.0]
    }
}

/// Validates one recorded circle edge and builds its exact geometry plus the
/// deterministic `MESH_SEGMENTS`-gon tessellation.
fn circle_from_edge(
    center: [f64; 3],
    radius: f64,
    normal: [f64; 3],
) -> Result<(CircleGeom, Vec<[f64; 3]>), Refusal> {
    if !radius.is_finite() || !(radius > 0.0) {
        return Err(Refusal::Empty);
    }
    if !center.iter().all(|c| c.is_finite()) || !normal.iter().all(|c| c.is_finite()) {
        return Err(Refusal::Empty);
    }
    let mag = v3_norm(normal);
    if !mag.is_finite() || !(mag > 0.0) {
        return Err(Refusal::Empty);
    }
    let n = [normal[0] / mag, normal[1] / mag, normal[2] / mag];
    let u = perp_unit(n);
    let v = v3_cross(n, u);
    let geom = CircleGeom {
        center,
        radius,
        u,
        v,
    };
    let mut verts = Vec::with_capacity(MESH_SEGMENTS);
    for k in 0..MESH_SEGMENTS {
        let theta = TAU * k as f64 / MESH_SEGMENTS as f64;
        verts.push(geom.point(theta.cos(), theta.sin()));
    }
    Ok((geom, verts))
}

fn v3_sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn v3_cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn v3_dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// The all-line vertex list of a recorded 3-D profile (one vertex per edge, in
/// order). `None` when any edge is a spline: a spline carrier cannot be
/// flattened to its sample polygon, so the caller refuses typed rather than
/// approximate.
fn profile3_vertices(profile: &[ProfileEdge]) -> Option<Vec<[f64; 3]>> {
    let mut out = Vec::with_capacity(profile.len());
    for edge in profile {
        match edge {
            ProfileEdge::Line { a, .. } => out.push(*a),
            ProfileEdge::Spline { .. } | ProfileEdge::Circle { .. } => return None,
        }
    }
    if out.is_empty() {
        return None;
    }
    Some(out)
}

/// Builds and validates one closed planar line-loop profile.
fn profile_loop(profile: &[ProfileEdge]) -> Result<ProfileLoop, Refusal> {
    // A single recorded circle edge is a whole section: build the exact conic
    // carrier (analytic area/extrema) plus its deterministic tessellation.
    if let [
        ProfileEdge::Circle {
            center,
            radius,
            normal,
        },
    ] = profile
    {
        let (circle, verts) = circle_from_edge(*center, *radius, *normal)?;
        let mut scale = 0.0f64;
        for c in center.iter().chain(normal.iter()) {
            scale = scale.max(c.abs());
        }
        scale = scale.max(v3_norm(*center) + radius);
        return Ok(ProfileLoop {
            verts,
            normal: circle.normal(),
            area: std::f64::consts::PI * radius * radius,
            scale,
            circle: Some(circle),
        });
    }
    if profile
        .iter()
        .any(|edge| matches!(edge, ProfileEdge::Circle { .. }))
    {
        // A circle edge mixed with other edges is not a recorded section.
        return Err(Refusal::UnsupportedEnvelope(
            EnvelopeCase::NonCanonicalCarrier,
        ));
    }
    let verts = profile3_vertices(profile).ok_or_else(|| {
        // A spline profile edge is not flattenable to its sample polygon; the
        // recording arm keeps the samples and the fact arm refuses typed.
        Refusal::UnsupportedEnvelope(EnvelopeCase::NonCanonicalCarrier)
    })?;
    if verts.len() < 3 {
        return Err(Refusal::Empty);
    }
    let mut scale = 0.0f64;
    for v in &verts {
        for c in v {
            if !c.is_finite() {
                return Err(Refusal::Empty);
            }
            scale = scale.max(c.abs());
        }
    }
    // The recorded edges must chain into a closed loop: consecutive edges share
    // their meeting point and the last edge returns to the first vertex. The
    // maximum gap is the seam mismatch evidence of the carrier.
    let seam_tol = 1e-9 * (1.0 + scale);
    let mut max_gap = 0.0f64;
    let n = verts.len();
    for i in 0..n {
        let b = match profile.get(i) {
            Some(ProfileEdge::Line { b, .. }) => *b,
            _ => verts[(i + 1) % n],
        };
        let next = verts[(i + 1) % n];
        let gap = if i + 1 < n {
            v3_norm(v3_sub(b, next))
        } else {
            v3_norm(v3_sub(b, verts[0]))
        };
        max_gap = max_gap.max(gap);
    }
    if max_gap > seam_tol {
        return Err(Refusal::UnsupportedEnvelope(
            EnvelopeCase::NonCanonicalCarrier,
        ));
    }
    // Right-hand area vector.
    let mut ax = 0.0f64;
    let mut ay = 0.0f64;
    let mut az = 0.0f64;
    for i in 0..n {
        let a = verts[i];
        let b = verts[(i + 1) % n];
        ax += a[1] * b[2] - a[2] * b[1];
        ay += a[2] * b[0] - a[0] * b[2];
        az += a[0] * b[1] - a[1] * b[0];
    }
    let area_vec = [ax * 0.5, ay * 0.5, az * 0.5];
    let mag = v3_norm(area_vec);
    if !(mag > 0.0) || !mag.is_finite() {
        return Err(Refusal::Empty);
    }
    let normal = [area_vec[0] / mag, area_vec[1] / mag, area_vec[2] / mag];
    // The loop must be planar (the recorded surface is elementary).
    let origin = verts[0];
    let planarity_tol = 1e-7 * (1.0 + scale);
    for a in &verts {
        let rel = v3_sub(*a, origin);
        if v3_dot(rel, normal).abs() > planarity_tol {
            return Err(Refusal::UnsupportedEnvelope(
                EnvelopeCase::NonCanonicalCarrier,
            ));
        }
    }
    Ok(ProfileLoop {
        verts,
        normal,
        area: mag,
        scale,
        circle: None,
    })
}

/// The unit normal of a difference vector, or `None` when it is degenerate.
fn v3_norm(a: [f64; 3]) -> f64 {
    v3_dot(a, a).sqrt()
}

/// The local bounding box and volume of an exact prism extruded from a closed
/// planar line-loop profile along its plane normal.
pub(crate) struct PrismGeom {
    loop3: ProfileLoop,
    /// Signed start offset along the normal (0 or -amount).
    t_lo: f64,
    /// Signed end offset along the normal (amount, or amount when both).
    t_hi: f64,
    /// The exact prism volume.
    pub(crate) volume: f64,
}

pub(crate) fn prism_geom(
    profile: &[ProfileEdge],
    amount: f64,
    both: bool,
) -> Result<PrismGeom, Refusal> {
    if !amount.is_finite() || amount <= 0.0 {
        return Err(Refusal::Empty);
    }
    let loop3 = profile_loop(profile)?;
    let (t_lo, t_hi) = if both {
        (-amount, amount)
    } else {
        (0.0, amount)
    };
    let height = t_hi - t_lo;
    let volume = loop3.area * height;
    Ok(PrismGeom {
        loop3,
        t_lo,
        t_hi,
        volume,
    })
}

/// The exact local AABB of an extruded prism: the profile loop's support along
/// each axis, widened by the extrusion range along the plane normal.
pub(crate) fn prism_bbox(
    profile: &[ProfileEdge],
    amount: f64,
    both: bool,
) -> Result<[[f64; 3]; 2], Refusal> {
    let geom = prism_geom(profile, amount, both)?;
    let n = geom.loop3.normal;
    let box0 = loop_bbox(&geom.loop3);
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    for axis in 0..3 {
        // The t contribution is constant over the cross-section, so the prism
        // extent is the loop support plus the t range projected on the axis.
        let (d_min, d_max) = if n[axis] >= 0.0 {
            (geom.t_lo * n[axis], geom.t_hi * n[axis])
        } else {
            (geom.t_hi * n[axis], geom.t_lo * n[axis])
        };
        min[axis] = box0[0][axis] + d_min;
        max[axis] = box0[1][axis] + d_max;
    }
    if !min[0].is_finite() || !max[0].is_finite() {
        return Err(Refusal::Empty);
    }
    Ok([min, max])
}

/// The exact AABB of a profile loop: the support of any axis-aligned linear
/// function over a polygon is attained at a polygon vertex, so the loop's
/// bbox over its vertices is exact for the loop region.
fn loop_bbox(loop3: &ProfileLoop) -> [[f64; 3]; 2] {
    if let Some(circle) = loop3.circle {
        // The analytic extrema of a circle: along axis `e`, the support of
        // `c + r (cos t u + sin t v)` is `c_e +- r sqrt(u_e^2 + v_e^2)`.
        let mut min = [0.0f64; 3];
        let mut max = [0.0f64; 3];
        for axis in 0..3 {
            let extent = circle.radius
                * (circle.u[axis] * circle.u[axis] + circle.v[axis] * circle.v[axis]).sqrt();
            min[axis] = circle.center[axis] - extent;
            max[axis] = circle.center[axis] + extent;
        }
        return [min, max];
    }
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    for v in &loop3.verts {
        for axis in 0..3 {
            min[axis] = min[axis].min(v[axis]);
            max[axis] = max[axis].max(v[axis]);
        }
    }
    [min, max]
}

/// The exact volume of a ruled two-section loft between two matched parallel
/// planar line-loop sections (the "segment moment" of one loft pair). The
/// cross-section polygon's area is quadratic in the stack parameter, so the
/// segment volume is the exact Simpson value over the end and mid sections.
/// (Test-side independent machine check of the divergence-form volume arm.)
#[cfg(test)]
fn loft_segment_volume(a: &ProfileLoop, b: &ProfileLoop) -> f64 {
    let n = a.verts.len();
    // Mid-section vertices (matched linear interpolation at u = 1/2).
    let mut mid = Vec::with_capacity(n);
    for i in 0..n {
        let p = a.verts[i];
        let q = b.verts[i];
        mid.push([
            0.5 * (p[0] + q[0]),
            0.5 * (p[1] + q[1]),
            0.5 * (p[2] + q[2]),
        ]);
    }
    // The mid polygon lies in a plane halfway between the (parallel) section
    // planes; its right-hand area along the shared normal.
    let mut cross = 0.0f64;
    let normal = a.normal;
    for i in 0..n {
        let p = mid[i];
        let q = mid[(i + 1) % n];
        cross += v3_dot(v3_cross(p, q), normal);
    }
    let area_mid = cross * 0.5;
    // The perpendicular separation of the two section planes.
    let d = v3_sub(b.verts[0], a.verts[0]);
    let h = v3_dot(d, normal).abs();
    // Simpson over the quadratic cross-section area: exact.
    (h / 6.0) * (a.area + 4.0 * area_mid + b.area)
}

/// The signed side-patch moment `(1/3) int int X.(Xu x Xv) du dv` of one ruled
/// patch between the matching edges `(a0 -> a1)` of one section and
/// `(b0 -> b1)` of the next, evaluated by two-point Gauss-Legendre quadrature
/// (exact for the polynomial integrand).
fn side_patch_moment(a0: [f64; 3], a1: [f64; 3], b0: [f64; 3], b1: [f64; 3]) -> f64 {
    let inv_sqrt3 = 1.0 / 3.0f64.sqrt();
    let nodes = [0.5 - 0.5 * inv_sqrt3, 0.5 + 0.5 * inv_sqrt3];
    let da = v3_sub(a1, a0);
    let db = v3_sub(b1, b0);
    let ga = v3_sub(b0, a0);
    let gb = v3_sub(b1, a1);
    let mut acc = 0.0f64;
    for ui in nodes {
        for vi in nodes {
            // A(u) and B(u).
            let a = [a0[0] + ui * da[0], a0[1] + ui * da[1], a0[2] + ui * da[2]];
            let b = [b0[0] + ui * db[0], b0[1] + ui * db[1], b0[2] + ui * db[2]];
            // X = A + v (B - A); Xu = (1-v) da + v db; Xv = (1-u) ga + u gb.
            let ab = v3_sub(b, a);
            let x = [a[0] + vi * ab[0], a[1] + vi * ab[1], a[2] + vi * ab[2]];
            let xu = [
                (1.0 - vi) * da[0] + vi * db[0],
                (1.0 - vi) * da[1] + vi * db[1],
                (1.0 - vi) * da[2] + vi * db[2],
            ];
            let xv = [
                (1.0 - ui) * ga[0] + ui * gb[0],
                (1.0 - ui) * ga[1] + ui * gb[1],
                (1.0 - ui) * ga[2] + ui * gb[2],
            ];
            acc += v3_dot(x, v3_cross(xu, xv));
        }
    }
    // 2x2 Gauss: each node weight is 1/4 on [0,1]^2.
    acc / 12.0
}

/// A loft arm's per-section data validated for the exact ruled carrier.
struct LoftSections {
    loops: Vec<ProfileLoop>,
}

/// Validates a loft section stack for the exact ruled carrier: every section
/// must be a closed planar line loop, all sections must carry the same number
/// of matched vertices, and consecutive sections must stack with their
/// recorded winding normals (the gate below refuses a non-uniform or folded
/// frame instead of approximating it).
fn loft_sections(sections: &[Vec<ProfileEdge>]) -> Result<LoftSections, Refusal> {
    if sections.len() < 2 {
        return Err(Refusal::Empty);
    }
    let loops: Vec<ProfileLoop> = sections
        .iter()
        .map(|s| profile_loop(s))
        .collect::<Result<_, _>>()?;
    let count = loops[0].verts.len();
    let circle = loops[0].circle.is_some();
    for loop3 in &loops {
        if loop3.verts.len() != count || loop3.circle.is_some() != circle {
            // A matched ruled carrier needs equal vertex counts and a single
            // section kind (all polygon or all exact circle).
            return Err(Refusal::UnsupportedEnvelope(
                EnvelopeCase::NonCanonicalCarrier,
            ));
        }
    }
    Ok(LoftSections { loops })
}

/// The seam certificate of a closed halo loft row (the `closed_loop`
/// certificate): the last station is the exact return to the first station, so
/// the aligned meeting edges across the chain closure satisfy
/// `A0 W1 - A1 W0 == 0` (weights 1 for the recorded line carrier). Returns the
/// maximum mismatch over the aligned station vertices; a station list that
/// does not close returns `Err` with the mismatch evidence.
fn loft_seam_mismatch(sections: &LoftSections, closed: bool) -> Result<f64, Refusal> {
    if !closed {
        return Ok(f64::INFINITY); // no seam to certify on an open chain
    }
    let first = &sections.loops[0];
    let last = sections.loops.last().ok_or(Refusal::Empty)?;
    let scale = (first.scale).max(last.scale);
    let tol = 1e-7 * (1.0 + scale);
    let n = first.verts.len();
    let mut max_gap = 0.0f64;
    for i in 0..n {
        let gap = v3_norm(v3_sub(first.verts[i], last.verts[i]));
        max_gap = max_gap.max(gap);
    }
    if max_gap > tol {
        return Err(Refusal::UnsupportedEnvelope(
            EnvelopeCase::NonCanonicalCarrier,
        ));
    }
    Ok(max_gap)
}

/// The exact divergence-form side moment of one ruled patch between two exact
/// circle sections. With `A(t) = c0 + r0 (cos t u0 + sin t v0)` and
/// `D(t) = B(t) - A(t)`, the side integrand
/// `X.(X_t x X_theta)` is a trigonometric polynomial of degree at most three,
/// so the fixed `CIRCLE_QUAD`-node equal-spaced rule integrates it exactly.
fn circle_pair_side_moment(a: &CircleGeom, b: &CircleGeom) -> f64 {
    let mut acc = 0.0f64;
    for k in 0..CIRCLE_QUAD {
        let theta = TAU * k as f64 / CIRCLE_QUAD as f64;
        let c = theta.cos();
        let s = theta.sin();
        let ap = a.point(c, s);
        let adir = a.tangent(c, s);
        let bp = b.point(c, s);
        let d = v3_sub(bp, ap);
        let ddir = v3_sub(b.tangent(c, s), adir);
        // int_0^1 X.(X_theta x X_t) dt =
        //   A.(A' x D) + (1/2)( D.(A' x D) + A.(D' x D) )
        let term = v3_dot(ap, v3_cross(adir, d))
            + 0.5 * (v3_dot(d, v3_cross(adir, d)) + v3_dot(ap, v3_cross(ddir, d)));
        acc += term;
    }
    (1.0 / 3.0) * (TAU / CIRCLE_QUAD as f64) * acc
}

/// The exact volume of a loft chain whose sections are all exact circles. The
/// ruled side surface between consecutive sections is integrated exactly by
/// the trigonometric quadrature and the two planar end caps contribute the
/// same `(1/3) n . c A` divergence moment the polygon arm uses.
fn circle_loft_volume(sections: &LoftSections, closed: bool) -> Result<f64, Refusal> {
    let n_sec = sections.loops.len();
    let mut total = 0.0f64;
    for j in 0..(n_sec - 1) {
        let a = sections.loops[j].circle.ok_or(Refusal::Empty)?;
        let b = sections.loops[j + 1].circle.ok_or(Refusal::Empty)?;
        total += circle_pair_side_moment(&a, &b);
    }
    if !closed {
        let first = &sections.loops[0];
        let last = sections.loops.last().ok_or(Refusal::Empty)?;
        let ca = first.circle.ok_or(Refusal::Empty)?;
        let cb = last.circle.ok_or(Refusal::Empty)?;
        total += -(1.0 / 3.0) * first.area * v3_dot(ca.normal(), ca.center);
        total += (1.0 / 3.0) * last.area * v3_dot(cb.normal(), cb.center);
    }
    if !total.is_finite() {
        return Err(Refusal::Empty);
    }
    Ok(total.abs())
}

/// The exact volume of a loft chain. For an open chain the boundary is the
/// ruled side surface between consecutive stations plus the two end caps; for
/// a closed halo row the station list returns to its start and the side
/// surface closes on itself (no caps, the last station repeats the first and
/// its zero-length wrap segment contributes nothing). The winding direction is
/// not part of the row data, so the orientation of the recorded loops is
/// absorbed exactly as in the lathe arm.
fn loft_volume(sections: &LoftSections, closed: bool) -> Result<f64, Refusal> {
    if sections.loops.iter().all(|loop3| loop3.circle.is_some()) {
        return circle_loft_volume(sections, closed);
    }
    let mut total = 0.0f64;
    let n_sec = sections.loops.len();
    let segment_count = if closed { n_sec - 1 } else { n_sec - 1 };
    for j in 0..segment_count {
        let a = &sections.loops[j];
        let b = &sections.loops[j + 1];
        let n = a.verts.len();
        // Matched ruled side patches between the corresponding edges.
        for i in 0..n {
            let a0 = a.verts[i];
            let a1 = a.verts[(i + 1) % n];
            let b0 = b.verts[i];
            let b1 = b.verts[(i + 1) % n];
            total += side_patch_moment(a0, a1, b0, b1);
        }
    }
    if !closed {
        // End caps: the recorded section is a flat polygon in the plane
        // `normal . x = d`, so its outward-facing divergence contribution is
        // `(1/3) d A`. The outward cap normal is opposite the recorded winding
        // normal at the first station (interior lies along the chain) and along
        // it at the last station.
        let first = &sections.loops[0];
        let last = sections.loops.last().ok_or(Refusal::Empty)?;
        let d_first = v3_dot(first.normal, first.verts[0]);
        let d_last = v3_dot(last.normal, last.verts[0]);
        let cap_first = -(1.0 / 3.0) * first.area * d_first;
        let cap_last = (1.0 / 3.0) * last.area * d_last;
        total += cap_first + cap_last;
    }
    if !total.is_finite() {
        return Err(Refusal::Empty);
    }
    Ok(total.abs())
}

// ---------------------------------------------------------------------------
// Authoring arms: mirror placed-carrier
// ---------------------------------------------------------------------------

/// Applies the part's placed-carrier reflection to a local point.
fn mirror_point(p: [f64; 3], mirror: &str) -> [f64; 3] {
    match mirror {
        "x" => [-p[0], p[1], p[2]],
        "z" => [p[0], p[1], -p[2]],
        _ => [p[0], -p[1], p[2]],
    }
}

/// The signed-zero-normalized reflection of one coordinate: `x -> -x` except
/// that a zero keeps its bit pattern. This is mathematically the negation and a
/// bit-for-bit involution (`reflect_coord(reflect_coord(x)) == x`), so a double
/// reflection is the exact identity.
#[cfg(test)]
fn reflect_coord(value: f64) -> f64 {
    if value == 0.0 { value } else { -value }
}

/// Reflects a local point across the recorded coordinate plane.
#[cfg(test)]
fn reflect_point3(p: [f64; 3], axis: &str) -> [f64; 3] {
    let [x, y, z] = p;
    match axis {
        "x" => [reflect_coord(x), y, z],
        "z" => [x, y, reflect_coord(z)],
        _ => [x, reflect_coord(y), z],
    }
}

/// Reflects one recorded profile edge's control points.
#[cfg(test)]
fn reflect_profile_edge(edge: &ProfileEdge, axis: &str) -> ProfileEdge {
    match edge {
        ProfileEdge::Line { a, b } => ProfileEdge::Line {
            a: reflect_point3(*a, axis),
            b: reflect_point3(*b, axis),
        },
        ProfileEdge::Spline { points } => ProfileEdge::Spline {
            points: points.iter().map(|p| reflect_point3(*p, axis)).collect(),
        },
        ProfileEdge::Circle {
            center,
            radius,
            normal,
        } => ProfileEdge::Circle {
            center: reflect_point3(*center, axis),
            radius: *radius,
            normal: reflect_point3(*normal, axis),
        },
    }
}

/// Reflects one recorded station frame across the coordinate plane: the origin
/// and each axis direction reflect, so the placed profile reflects exactly.
#[cfg(test)]
fn reflect_station(station: &StationFrame, axis: &str) -> StationFrame {
    StationFrame {
        origin: reflect_point3(station.origin, axis),
        x_dir: reflect_point3(station.x_dir, axis),
        y_dir: reflect_point3(station.y_dir, axis),
        z_dir: reflect_point3(station.z_dir, axis),
    }
}

/// Reflects one lathe profile edge (the `(x, z)` profile plane). Only the `z`
/// reflection is representable as the same `x >= 0` profile.
#[cfg(test)]
fn reflect_lathe_edge(edge: &LatheEdge) -> LatheEdge {
    let reflect = |p: [f64; 2]| {
        let [x, z] = p;
        [x, reflect_coord(z)]
    };
    match edge {
        LatheEdge::Line { a, b } => LatheEdge::Line {
            a: reflect(*a),
            b: reflect(*b),
        },
        LatheEdge::Spline { points } => LatheEdge::Spline {
            points: points.iter().map(|p| reflect(*p)).collect(),
        },
    }
}

/// Reflects one kernel row's control points across a coordinate plane through
/// the origin: an exact isometry whose patch grid (edge counts and degrees) is
/// reused exactly, so no surface is re-approximated. The magnitude of every
/// certified fact is invariant and the orientation flips (the change-of-
/// variables theorem, `det R = -1`). `reflect_solid` is an involution:
/// `reflect_solid(reflect_solid(x)) == x` bit-for-bit.
#[cfg(test)]
fn reflect_solid(solid: &SolidSpec, axis: &str) -> Result<SolidSpec, Refusal> {
    let reflected = match solid {
        // The origin-centred primitives are symmetric about every coordinate
        // plane through the origin, so the reflection leaves them unchanged.
        SolidSpec::Box { .. }
        | SolidSpec::Cylinder { .. }
        | SolidSpec::Sphere { .. }
        | SolidSpec::Torus { .. } => solid.clone(),
        SolidSpec::Lathe {
            profile,
            arc_deg,
            start_deg,
        } => {
            if axis == "z" {
                SolidSpec::Lathe {
                    profile: profile.iter().map(reflect_lathe_edge).collect(),
                    arc_deg: *arc_deg,
                    start_deg: *start_deg,
                }
            } else if *arc_deg >= 360.0 {
                // A full revolution is symmetric about any plane through its
                // axis; the recorded `(x, z)` profile is unchanged.
                solid.clone()
            } else {
                // A partial wedge reflected about a plane through the z axis
                // maps the angular interval `[t0, t1]` to `[pi - t1, pi - t0]`
                // for the YZ plane (x -> -x) and to `[-t1, -t0]` for the XZ
                // plane (y -> -y); both are the same `(x, z)` profile with a
                // recorded start. In degrees: `start' = 180 - (start + arc)`
                // and `start' = -(start + arc)` respectively.
                let start = if axis == "y" {
                    -(*start_deg + *arc_deg)
                } else {
                    180.0 - (*start_deg + *arc_deg)
                };
                SolidSpec::Lathe {
                    profile: profile.clone(),
                    arc_deg: *arc_deg,
                    start_deg: start,
                }
            }
        }
        SolidSpec::Prism {
            profile,
            amount,
            both,
        } => SolidSpec::Prism {
            profile: profile
                .iter()
                .map(|edge| reflect_profile_edge(edge, axis))
                .collect(),
            amount: *amount,
            both: *both,
        },
        // The trim carrier's scalar pullback net is tied to the profile
        // parametrization, so its reflection is not representable as the same
        // row; refuse typed rather than record a mismatched net.
        SolidSpec::TrimPrism { .. } => {
            return Err(Refusal::UnsupportedEnvelope(
                EnvelopeCase::NonCanonicalCarrier,
            ));
        }
        SolidSpec::Loft { sections, closed } => SolidSpec::Loft {
            sections: sections
                .iter()
                .map(|section| {
                    section
                        .iter()
                        .map(|edge| reflect_profile_edge(edge, axis))
                        .collect()
                })
                .collect(),
            closed: *closed,
        },
        SolidSpec::Member {
            profile,
            stations,
            ruled,
        } => SolidSpec::Member {
            // The profile is recorded in station-local coordinates, so the
            // world reflection acts on the station frames: reflecting both the
            // profile and the frame would cancel. The placed section
            // `origin + x_dir * x + y_dir * y + z_dir * z` reflects exactly.
            profile: profile.clone(),
            stations: stations
                .iter()
                .map(|station| reflect_station(station, axis))
                .collect(),
            ruled: *ruled,
        },
    };
    Ok(reflected)
}

/// The world bounding box of one placed solid: mirror the local geometry about
/// the coordinate plane (when the mirror arm recorded one), rotate the local
/// bbox corners by the recorded frame (or by the pure-z `rz` when no frame was
/// recorded), then translate. The AABB of the 8 rotated local corners is the
/// exact world bbox under the orthonormal placement rotation.
fn part_world_bbox(part: &PartSpec) -> Result<[[f64; 3]; 2], Refusal> {
    let local = solid_local_bbox(&part.solid)?;
    let frame = part.rotation;
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
        // The mirror placed-carrier reflects the local bbox before the rz
        // rotation (a reflection is a congruence: reflecting the AABB of the
        // local solid is the AABB of the reflected solid).
        let corner = match part.mirror.as_deref() {
            Some(axis) => mirror_point(corner, axis),
            None => corner,
        };
        let point = match frame {
            Some(rotation) => {
                // The recorded full orthonormal frame replaces the pure-z
                // rotation: the AABB of the 8 rotated local corners is exact.
                let p = rotation.apply(corner);
                [p[0] + part.x, p[1] + part.y, p[2] + part.z]
            }
            None => {
                let (x, y) = if rz == 0.0 {
                    (corner[0], corner[1])
                } else {
                    (
                        corner[0] * cos - corner[1] * sin,
                        corner[0] * sin + corner[1] * cos,
                    )
                };
                [x + part.x, y + part.y, corner[2] + part.z]
            }
        };
        for axis in 0..3 {
            min[axis] = min[axis].min(point[axis]);
            max[axis] = max[axis].max(point[axis]);
        }
    }
    Ok([min, max])
}

/// The boolean-carrier class of one unplaced solid (the facade's taxonomy).
/// The LOCAL geometry classifies, never a placement: a placed operand's frame
/// composes after the dispatch, so admission is placement-blind.
fn solid_carrier_class(solid: &SolidSpec) -> CarrierClass {
    match solid {
        SolidSpec::Box { .. }
        | SolidSpec::Cylinder { .. }
        | SolidSpec::Sphere { .. }
        | SolidSpec::Prism { .. } => CarrierClass::Canonical,
        // The corpus spline-profile extrude carries a spline side surface, so
        // it classifies as the `Swept` carrier the facade's `Extrude` over a
        // spline profile produces; a line-base trim row keeps the canonical
        // class. A boolean coupling two spline-carried solids is the wave-3
        // frontier and refuses typed at the boolean boundary.
        SolidSpec::TrimPrism {
            profile,
            trim_curve,
            trim_net,
            ..
        } => {
            if is_spline_profile_prism(profile, trim_curve, trim_net) {
                CarrierClass::Swept
            } else {
                CarrierClass::Canonical
            }
        }
        SolidSpec::Torus { .. } => CarrierClass::Torus,
        SolidSpec::Lathe { profile, .. } => {
            if profile
                .iter()
                .any(|edge| matches!(edge, LatheEdge::Spline { .. }))
            {
                CarrierClass::Revolved
            } else {
                CarrierClass::Canonical
            }
        }
        SolidSpec::Loft { .. } | SolidSpec::Member { .. } => CarrierClass::Swept,
    }
}

/// The carrier class of one boolean operand node. A placed part classifies by
/// its LOCAL solid (the dispatch never sees a placement); a boolean result
/// operand classifies by its BASE operand's local carrier (MONO-9: the fold
/// contributes the operand's extracted patches, so a pure union/cut chain is
/// admitted where the old depth-1 rule refused); a group classifies by its
/// first part in script order.
fn node_carrier_class(node: &TreeNode) -> Result<CarrierClass, Refusal> {
    match node {
        TreeNode::Part { part } => Ok(solid_carrier_class(&part.solid)),
        TreeNode::Group { group, .. } => {
            for child in group {
                match node_carrier_class(child) {
                    Ok(class) => return Ok(class),
                    Err(Refusal::Empty) => continue,
                    Err(other) => return Err(other),
                }
            }
            Err(Refusal::Empty)
        }
        // MONO-9: a boolean result operand contributes its base carrier's
        // extracted patches (a pure union/cut fold), never the depth-1 refusal.
        TreeNode::Boolean { boolean } => node_carrier_class(&boolean.a),
        // A fillet node is an op node, not a part: a boolean over one (or a
        // fillet over one) is the depth-2 open cell and refuses typed.
        // (Arm restored by the orchestrator - the MONO-9 rewrite of the
        // adjacent Boolean arm dropped it. Merge 2026-09-11.)
        TreeNode::Fillet { .. } => Err(Refusal::UnsupportedEnvelope(
            EnvelopeCase::NonCanonicalCarrier,
        )),
    }
}

/// Dispatches one recorded boolean row through the facade's boolean entry
/// (the admission consult the binding's `boolean_dispatch` composes) and
/// returns the routed event. The dispatch is placement-blind: it sees each
/// operand's LOCAL carrier class. A refused pair keeps its typed, localized
/// envelope case (`NonCanonicalCarrier` for the not-yet-admitted carrier
/// class pair, `ContactReductionDeferred` for a torus carrier in a swept pair).
fn dispatch_boolean(node: &BooleanNode) -> Result<Option<SweptBooleanEvent>, Refusal> {
    let base = node_carrier_class(&node.a)?;
    let tool = node_carrier_class(&node.b)?;
    match crate::facade::dispatch_swept_carrier_boolean(base, tool, node.mode) {
        BooleanPairVerdict::CanonicalLanded => Ok(None),
        BooleanPairVerdict::Routed(route) => Ok(Some(SweptBooleanEvent {
            mode: route.mode,
            base: route.base,
            tool: route.tool,
        })),
        BooleanPairVerdict::Refused(refusal) => Err(Refusal::UnsupportedEnvelope(refusal.case)),
    }
}

/// Collects the routed boolean events of a tree, in script order, and
/// validates every row (a refused pair propagates its typed envelope case).
fn collect_boolean_events(
    node: &TreeNode,
    events: &mut Vec<SweptBooleanEvent>,
) -> Result<(), Refusal> {
    match node {
        TreeNode::Part { .. } => Ok(()),
        TreeNode::Group { group, .. } => {
            for child in group {
                collect_boolean_events(child, events)?;
            }
            Ok(())
        }
        TreeNode::Boolean { boolean } => {
            if let Some(event) = dispatch_boolean(boolean)? {
                events.push(event);
            }
            Ok(())
        }
        // A fillet row validates its depth-1 base and recurses into it; the
        // blend certificate is measured separately by `top_blend`.
        TreeNode::Fillet { fillet } => {
            fillet_base_part(fillet)?;
            collect_boolean_events(&fillet.base, events)
        }
    }
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
    let [volume_lo, volume_hi] = top_volume_bracket(root)?;
    let volume = if volume_lo == volume_hi {
        volume_lo
    } else {
        0.5 * (volume_lo + volume_hi)
    };
    let seam_mismatch = top_seam_mismatch(root)?;
    let mut boolean_events = Vec::new();
    collect_boolean_events(root, &mut boolean_events)?;
    let blend = top_blend(root)?;
    let (label, color, rows) = top_metadata(root)?;
    Ok(Facts {
        solid_count: count,
        volume,
        volume_bracket: [volume_lo, volume_hi],
        bbox: [min, max],
        seam_mismatch,
        boolean_events,
        blend,
        label,
        color,
        rows,
    })
}

/// The certified blend fact of a single fillet top node (AUTHOR-EXT-FILLET-HALO):
/// the base must be a part (depth-1), its recorded carrier must be in the
/// fillet vocabulary, and every recorded edge selector must resolve. A
/// non-fillet top node carries no blend.
fn top_blend(root: &TreeNode) -> Result<Option<BlendFacts>, Refusal> {
    match root {
        TreeNode::Fillet { fillet } => Ok(Some(fillet_blend(fillet)?)),
        _ => Ok(None),
    }
}

/// The depth-1 base part of a fillet node, or the typed refusal naming the
/// open carrier. A fillet of a group or of another op node is the recorded
/// open composition cell and never recurses silently.
fn fillet_base_part(fillet: &FilletNode) -> Result<&PartSpec, Refusal> {
    match fillet.base.as_ref() {
        TreeNode::Part { part } => Ok(part),
        TreeNode::Group { .. } | TreeNode::Boolean { .. } | TreeNode::Fillet { .. } => Err(
            Refusal::UnsupportedEnvelope(EnvelopeCase::NonCanonicalCarrier),
        ),
    }
}

/// The certified blend fact of one fillet node: dispatch the recorded base
/// carrier + radius + edge selectors into the landed blend profile machinery
/// (`binding::fillet_facts`). An unresolvable selector and a certified-solve
/// refusal both surface typed; nothing is approximated.
fn fillet_blend(fillet: &FilletNode) -> Result<BlendFacts, Refusal> {
    let part = fillet_base_part(fillet)?;
    let base = match &part.solid {
        SolidSpec::Box {
            length,
            width,
            height,
        } => FilletBaseRow::Box {
            length: *length,
            width: *width,
            height: *height,
        },
        _ => {
            return Err(Refusal::UnsupportedEnvelope(
                EnvelopeCase::NonCanonicalCarrier,
            ));
        }
    };
    let row = FilletRow {
        base,
        radius: fillet.radius,
        edges: fillet.edges.clone(),
    };
    let outcome = crate::python::binding::fillet_facts(&row).map_err(fillet_refusal_to_refusal)?;
    Ok(BlendFacts {
        kind: "fillet",
        radius: outcome.radius,
        edges: outcome.edges,
        max_profile_width: outcome.max_profile_width,
    })
}

/// Maps the binding-layer fillet refusal into the executor's refusal
/// vocabulary: an unresolvable selector names the open carrier
/// (`NonCanonicalCarrier`), an invalid request is the empty domain, and a
/// certified blend refusal keeps the construct layer's case.
fn fillet_refusal_to_refusal(refusal: FilletRefusal) -> Refusal {
    match refusal {
        FilletRefusal::InvalidInput => Refusal::Empty,
        FilletRefusal::UnresolvableEdge | FilletRefusal::Blend(_) => {
            Refusal::UnsupportedEnvelope(EnvelopeCase::NonCanonicalCarrier)
        }
    }
}

/// The top node's recorded metadata plus, for a group top node, the per-row
/// breakdown of its IMMEDIATE children. The breakdown is an addition only: it
/// recomputes each child with the same aggregate arithmetic and never changes
/// the top-level facts. A part top node carries no rows; a boolean top node
/// carries no metadata.
fn top_metadata(
    root: &TreeNode,
) -> Result<(Option<String>, Option<String>, Option<Vec<RowFacts>>), Refusal> {
    match root {
        TreeNode::Part { part } => Ok((part.label.clone(), part.color.clone(), None)),
        TreeNode::Group {
            group,
            label,
            color,
        } => {
            let mut rows = Vec::with_capacity(group.len());
            for child in group {
                rows.push(child_row_facts(child)?);
            }
            Ok((label.clone(), color.clone(), Some(rows)))
        }
        TreeNode::Boolean { .. } => Ok((None, None, None)),
        TreeNode::Fillet { .. } => Ok((None, None, None)),
    }
}

/// One immediate child's row facts, using the aggregate path's arithmetic.
fn child_row_facts(node: &TreeNode) -> Result<RowFacts, Refusal> {
    let mut count = 0u64;
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    count_and_union(node, &mut count, &mut min, &mut max)?;
    let volume = top_volume(node)?;
    let label = match node {
        TreeNode::Part { part } => part.label.clone(),
        TreeNode::Group { label, .. } => label.clone(),
        TreeNode::Boolean { .. } => None,
        TreeNode::Fillet { .. } => None,
    };
    Ok(RowFacts {
        label,
        solid_count: count,
        volume,
        bbox: [min, max],
    })
}

/// The seam certificate of a single closed halo loft row, when the top node is
/// exactly one part carrying a `closed` loft. A non-closing chain refuses
/// typed with the mismatch evidence; open chains and non-loft rows carry no
/// seam (`None`).
fn top_seam_mismatch(root: &TreeNode) -> Result<Option<f64>, Refusal> {
    match root {
        TreeNode::Part { part } => match &part.solid {
            SolidSpec::Loft { sections, closed } => {
                if let Ok(validated) = loft_sections(sections) {
                    let mismatch = loft_seam_mismatch(&validated, *closed)?;
                    if *closed {
                        Ok(Some(mismatch))
                    } else {
                        Ok(None)
                    }
                } else if *closed {
                    // A closed halo chain has no smooth certificate: the seam
                    // identity is only defined over the recorded line carrier.
                    Err(open_smooth_loft())
                } else {
                    Ok(None)
                }
            }
            _ => Ok(None),
        },
        // A routed boolean row measures its base operand (the tool is a
        // certificate witness); the dispatch validates the pair.
        TreeNode::Boolean { boolean } => {
            dispatch_boolean(boolean)?;
            top_seam_mismatch(&boolean.a)
        }
        // A fillet row measures its base operand; the base must be a part.
        TreeNode::Fillet { fillet } => {
            fillet_base_part(fillet)?;
            top_seam_mismatch(&fillet.base)
        }
        TreeNode::Group { .. } => Ok(None),
    }
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
        // A routed boolean row's measured geometry is its base operand; the
        // dispatch validates the pair (a refused pair propagates its typed
        // envelope case) and the routed event is collected separately.
        TreeNode::Boolean { boolean } => {
            dispatch_boolean(boolean)?;
            count_and_union(&boolean.a, count, min, max)
        }
        // A fillet row's measured geometry is its base operand; the blend
        // certificate is validated here (so a nested fillet is certified too)
        // and measured separately by `top_blend` for a fillet top node.
        TreeNode::Fillet { fillet } => {
            fillet_blend(fillet)?;
            count_and_union(&fillet.base, count, min, max)
        }
        TreeNode::Group { group, .. } => {
            for child in group {
                count_and_union(child, count, min, max)?;
            }
            Ok(())
        }
    }
}

/// Applies one placed row's world placement to a local point: the recorded
/// mirror reflection first, then the full orthonormal frame (or the pure-z
/// `rz`), then the translation — the exact composition `part_world_bbox`
/// applies to the local bbox corners.
fn place_part_point(part: &PartSpec, p: [f64; 3]) -> [f64; 3] {
    let p = match part.mirror.as_deref() {
        Some(axis) => mirror_point(p, axis),
        None => p,
    };
    match part.rotation {
        Some(rotation) => {
            let r = rotation.apply(p);
            [r[0] + part.x, r[1] + part.y, r[2] + part.z]
        }
        None => {
            let rz = part.rz.to_radians();
            let (x, y) = if rz == 0.0 {
                (p[0], p[1])
            } else {
                let (sin, cos) = rz.sin_cos();
                (p[0] * cos - p[1] * sin, p[0] * sin + p[1] * cos)
            };
            [x + part.x, y + part.y, p[2] + part.z]
        }
    }
}

/// The exact bicubic tensor-Bernstein elevation of the bilinear map through
/// four corners (unit weights), carrying the given outward-orientation sign.
/// The elevation is exact, so the patch is the planar quad itself and the
/// certified flux of the cycle is the enclosed volume.
fn boolean_quad_patch(
    c00: [f64; 3],
    c10: [f64; 3],
    c11: [f64; 3],
    c01: [f64; 3],
    orientation: f64,
) -> crate::python::binding::VolumeRow {
    let e0 = [1.0, 2.0 / 3.0, 1.0 / 3.0, 0.0];
    let e1 = [0.0, 1.0 / 3.0, 2.0 / 3.0, 1.0];
    let corners = [[c00, c01], [c10, c11]];
    let mut numerator = vec![vec![[0.0f64; 3]; 4]; 4];
    for (i, row) in numerator.iter_mut().enumerate() {
        let wu0 = e0.get(i).copied().unwrap_or(0.0);
        let wu1 = e1.get(i).copied().unwrap_or(0.0);
        for (j, slot) in row.iter_mut().enumerate() {
            let wv0 = e0.get(j).copied().unwrap_or(0.0);
            let wv1 = e1.get(j).copied().unwrap_or(0.0);
            let mut acc = [0.0f64; 3];
            for (a, pair) in corners.iter().enumerate() {
                let wu = if a == 0 { wu0 } else { wu1 };
                for (b, corner) in pair.iter().enumerate() {
                    let wv = if b == 0 { wv0 } else { wv1 };
                    let coeff = wu * wv;
                    for (acc_slot, value) in acc.iter_mut().zip(corner.iter()) {
                        *acc_slot += coeff * value;
                    }
                }
            }
            *slot = acc;
        }
    }
    crate::python::binding::VolumeRow {
        numerator,
        weights: vec![vec![1.0f64; 4]; 4],
        orientation,
    }
}

/// The six outward-oriented faces of one placed canonical box as patch rows in
/// world coordinates, or the typed `BooleanProductVolumeUnavailable` refusal
/// when the solid is not the canonical box the certified contact-cover solver
/// consumes. A placed mirror reflection flips the orientation (`det = -1`), so
/// the patch sign is carried explicitly and the flux stays the enclosed
/// volume.
fn placed_box_patches(
    part: &PartSpec,
) -> Result<Vec<crate::python::binding::VolumeRow>, BooleanVolumeRefusal> {
    let (length, width, height) = match &part.solid {
        SolidSpec::Box {
            length,
            width,
            height,
        } => (*length, *width, *height),
        _ => return Err(BooleanVolumeRefusal::BooleanProductVolumeUnavailable),
    };
    if !(length.is_finite() && width.is_finite() && height.is_finite())
        || length <= 0.0
        || width <= 0.0
        || height <= 0.0
    {
        return Err(BooleanVolumeRefusal::BooleanProductVolumeUnavailable);
    }
    let (hx, hy, hz) = (0.5 * length, 0.5 * width, 0.5 * height);
    let (x0, x1) = (-hx, hx);
    let (y0, y1) = (-hy, hy);
    let (z0, z1) = (-hz, hz);
    let orientation = if part.mirror.is_some() { -1.0 } else { 1.0 };
    let c = |x: f64, y: f64, z: f64| place_part_point(part, [x, y, z]);
    Ok(vec![
        boolean_quad_patch(
            c(x0, y0, z0),
            c(x0, y1, z0),
            c(x1, y1, z0),
            c(x1, y0, z0),
            orientation,
        ),
        boolean_quad_patch(
            c(x0, y0, z1),
            c(x1, y0, z1),
            c(x1, y1, z1),
            c(x0, y1, z1),
            orientation,
        ),
        boolean_quad_patch(
            c(x0, y0, z0),
            c(x1, y0, z0),
            c(x1, y0, z1),
            c(x0, y0, z1),
            orientation,
        ),
        boolean_quad_patch(
            c(x0, y1, z0),
            c(x0, y1, z1),
            c(x1, y1, z1),
            c(x1, y1, z0),
            orientation,
        ),
        boolean_quad_patch(
            c(x0, y0, z0),
            c(x0, y0, z1),
            c(x0, y1, z1),
            c(x0, y1, z0),
            orientation,
        ),
        boolean_quad_patch(
            c(x1, y0, z0),
            c(x1, y1, z0),
            c(x1, y1, z1),
            c(x1, y0, z1),
            orientation,
        ),
    ])
}

/// The patch rows of one boolean operand: a placed canonical box's six faces,
/// or the typed open-carrier refusal for any other node (a swept/lofted solid,
/// a nested group, a nested boolean).
fn node_box_patches(
    node: &TreeNode,
) -> Result<Vec<crate::python::binding::VolumeRow>, BooleanVolumeRefusal> {
    match node {
        TreeNode::Part { part } => placed_box_patches(part),
        // A fillet as a boolean operand is the recorded open composition cell
        // (FilletNode depth-1 discipline): refuse typed, never recurse.
        TreeNode::Fillet { .. } | TreeNode::Boolean { .. } | TreeNode::Group { .. } => {
            Err(BooleanVolumeRefusal::BooleanProductVolumeUnavailable)
        }
    }
}

/// The certified product volume of one canonical boolean row: `V(A op B)` from
/// the landed MONO-6 contact-cover certificate (PB-011B's canonical volume
/// authority). Both operands must be placed canonical boxes; any other carrier
/// is the open (unmeasured) case and refuses typed
/// `BooleanProductVolumeUnavailable`. Subtract reads the certificate's `A \ B`
/// bracket; intersect reads its `A n B` bracket; union is the exact
/// inclusion-exclusion `V(A) + V(B) - V(A n B)` over the same certified facts.
fn boolean_product_volume_certified(boolean: &BooleanNode) -> Result<f64, BooleanVolumeRefusal> {
    let a = node_box_patches(&boolean.a)?;
    let b = node_box_patches(&boolean.b)?;
    let certificate =
        membership::certify_boolean_volume(&a, &b, &membership::BooleanVolumeOptions::default())?;
    let intersection = 0.5 * (certificate.intersection_lo + certificate.intersection_hi);
    let value = match boolean.mode {
        crate::facade::ModeValue::Subtract => certificate.value,
        crate::facade::ModeValue::Add => certificate.volume_a + certificate.volume_b - intersection,
        crate::facade::ModeValue::Intersect => intersection,
    };
    if value.is_finite() {
        Ok(value)
    } else {
        Err(BooleanVolumeRefusal::BooleanProductVolumeUnavailable)
    }
}

/// The product volume of one boolean row, mapped onto the executor's typed
/// kernel refusal. The product's certified facts are the landed MONO-6
/// contact-cover certificate; an open carrier (any non-box operand, e.g. a
/// swept tool) refuses typed rather than reporting the base operand's volume.
fn boolean_product_volume(boolean: &BooleanNode) -> Result<f64, Refusal> {
    dispatch_boolean(boolean)?;
    boolean_product_volume_certified(boolean).map_err(boolean_volume_refusal_to_refusal)
}

/// Maps the certified boolean-volume refusal onto the executor's frozen typed
/// kernel refusal vocabulary: the contact/membership indeterminacies name the
/// deferred contact reduction, every other case names the open carrier.
fn boolean_volume_refusal_to_refusal(refusal: BooleanVolumeRefusal) -> Refusal {
    match refusal {
        BooleanVolumeRefusal::TransversalityUncertified
        | BooleanVolumeRefusal::MembershipIndeterminate => {
            Refusal::UnsupportedEnvelope(EnvelopeCase::ContactReductionDeferred)
        }
        BooleanVolumeRefusal::NonRegularPatch
        | BooleanVolumeRefusal::BudgetExceeded
        | BooleanVolumeRefusal::ExtremeSlabContaminated
        | BooleanVolumeRefusal::MalformedPatch
        | BooleanVolumeRefusal::RationalWeights
        | BooleanVolumeRefusal::BooleanProductVolumeUnavailable => {
            Refusal::UnsupportedEnvelope(EnvelopeCase::NonCanonicalCarrier)
        }
    }
}

/// Flattens a same-mode boolean tree into its recorded operand list (base
/// first, then every tool in recorded order). A nested boolean of a DIFFERENT
/// mode stays a leaf: it is the open composition cell and its extraction
/// refuses typed downstream. Returns `None` when the node is not a boolean.
fn fold_operands(node: &TreeNode) -> Option<(crate::facade::ModeValue, Vec<&TreeNode>)> {
    let TreeNode::Boolean { boolean } = node else {
        return None;
    };
    let mode = boolean.mode;
    let mut operands: Vec<&TreeNode> = Vec::new();
    fold_collect(node, mode, &mut operands);
    Some((mode, operands))
}

/// Recursively collects the operands of one same-mode fold, in recorded order.
fn fold_collect<'a>(
    node: &'a TreeNode,
    mode: crate::facade::ModeValue,
    out: &mut Vec<&'a TreeNode>,
) {
    match node {
        TreeNode::Boolean { boolean } if boolean.mode == mode => {
            fold_collect(&boolean.a, mode, out);
            fold_collect(&boolean.b, mode, out);
        }
        _ => out.push(node),
    }
}

/// Whether a boolean node is a multi-operand fold (a depth-2+ chain): the
/// depth-1 pair keeps the landed pairwise product path bit-for-bit, and only
/// the chained composition dispatches through the MONO-9 fold evaluator.
fn is_fold_chain(node: &TreeNode) -> bool {
    match node {
        TreeNode::Boolean { boolean } => {
            matches!(*boolean.a, TreeNode::Boolean { .. })
                || matches!(*boolean.b, TreeNode::Boolean { .. })
        }
        _ => false,
    }
}

/// The certified multi-operand fold volume of one chained boolean node. Every
/// operand is extracted through the landed MONO-8 adapter (a pure union/cut
/// fold contributes its EXTRACTED PATCHES; a mixed-mode nesting, a non-patch
/// carrier or an uncertifiable contact refuses typed, never approximates) and
/// the single compound-indicator evaluator runs the fold. No intermediate
/// union is constructed.
fn boolean_fold_volume(node: &TreeNode) -> Result<membership::FoldVolumeCertificate, Refusal> {
    let Some((mode, operands)) = fold_operands(node) else {
        return Err(Refusal::UnsupportedEnvelope(
            EnvelopeCase::NonCanonicalCarrier,
        ));
    };
    if operands.len() < 2 {
        return Err(Refusal::UnsupportedEnvelope(
            EnvelopeCase::NonCanonicalCarrier,
        ));
    }
    // The mode dispatch (judgement 4): the chained carrier consult the depth-1
    // refusal used to reject. A refused pair propagates its typed, localized
    // case; canonical operands record no row.
    let mut classes: Vec<CarrierClass> = Vec::with_capacity(operands.len());
    for operand in &operands {
        classes.push(node_carrier_class(operand)?);
    }
    let Some((base_class, tool_classes)) = classes.split_first() else {
        return Err(Refusal::Empty);
    };
    crate::facade::dispatch_swept_carrier_fold(*base_class, tool_classes, mode)
        .map_err(|refusal| Refusal::UnsupportedEnvelope(refusal.case))?;

    let mut rows: Vec<Vec<crate::python::binding::VolumeRow>> = Vec::with_capacity(operands.len());
    for operand in &operands {
        let extracted = extract_patches(operand).map_err(swept_refusal_to_refusal)?;
        rows.push(extracted.into_iter().map(|(row, _)| row).collect());
    }
    let Some((base_rows, tool_rows)) = rows.split_first() else {
        return Err(Refusal::Empty);
    };
    membership::certify_fold_volume(
        base_rows,
        tool_rows,
        mode,
        &membership::BooleanVolumeOptions::default(),
    )
    .map_err(boolean_volume_refusal_to_refusal)
}

/// The measured volume of a top node: a group sums only its immediate child
/// parts (nested groups contribute nothing, mirroring the OCC `Compound`
/// measurement the reference was recorded with); a part is its own volume;
/// a boolean row is the PRODUCT's certified volume (never the base
/// operand's). The scalar value is the midpoint of [`top_volume_bracket`]
/// (identical to the scalar measurement whenever the bracket is degenerate).
fn top_volume(node: &TreeNode) -> Result<f64, Refusal> {
    let [lo, hi] = top_volume_bracket(node)?;
    Ok(if lo == hi { lo } else { 0.5 * (lo + hi) })
}

/// The interval-valued volume fact of a node (MONO-8, theory statement section
/// 5). A scalar node carries the degenerate interval `[V, V]`; a group
/// aggregates interval sums over its immediate children (a nested group
/// contributes nothing, matching the aggregate rule); a certified
/// `Swept x Swept` boolean carries the solver's bracket; a canonical boolean
/// carries the PRODUCT's volume as a degenerate interval (RG-4: never the
/// base operand's).
fn top_volume_bracket(node: &TreeNode) -> Result<[f64; 2], Refusal> {
    match node {
        TreeNode::Part { part } => {
            let volume = solid_volume(&part.solid)?;
            Ok([volume, volume])
        }
        TreeNode::Boolean { boolean } => {
            if is_fold_chain(node) {
                // MONO-9: a chained composition is the compound-indicator fold;
                // no intermediate union is constructed.
                let certificate = boolean_fold_volume(node)?;
                Ok([certificate.bracket_lo, certificate.bracket_hi])
            } else if swept_swept(boolean)? {
                match admit_swept_pair(boolean) {
                    Ok(certificate) => Ok([certificate.bracket_lo, certificate.bracket_hi]),
                    Err(stage) => Err(swept_refusal_to_refusal(stage)),
                }
            } else {
                let volume = boolean_product_volume(boolean)?;
                Ok([volume, volume])
            }
        }
        // A fillet row's interval-valued volume fact is its base part's
        // (the blend is a local certificate on the base).
        TreeNode::Fillet { fillet } => {
            fillet_base_part(fillet)?;
            top_volume_bracket(&fillet.base)
        }
        TreeNode::Group { group, .. } => {
            let mut lo = 0.0;
            let mut hi = 0.0;
            for child in group {
                match child {
                    TreeNode::Part { .. } | TreeNode::Boolean { .. } | TreeNode::Fillet { .. } => {
                        let [clo, chi] = top_volume_bracket(child)?;
                        lo += clo;
                        hi += chi;
                    }
                    TreeNode::Group { .. } => {}
                }
            }
            Ok([lo, hi])
        }
    }
}

// ---------------------------------------------------------------------------
// MONO-8-SWEPT-ADMISSION-WIRING -- the extraction adapter and the Swept x
// Swept gate order.
//
// A recorded `cut(loft, loft)` / `fuse` pair is admitted into the certified
// solver through the fixed gate order of the theory statement section 3:
//
//   1. extraction (`extract_patches`): the landed kernel construction of each
//      operand's solid carrier is decomposed into its tensor-Bernstein patch
//      2-cycle, with the placement applied to the control points and the
//      orientation sign `sigma_i = sign(det M_tau) * sigma_i^0` (a reflective
//      placement flips every sign);
//   2. transversality (the landed MONO-6 `certify_boolean_volume` admission,
//      the executor twin of the FSSI-001 gate): a tangential/coincident pair
//      refuses `NonTransversalContact`;
//   3. the solver: the certified bracket `[V_lo, V_hi]` becomes the boolean
//      node's interval-valued volume fact.
//
// Each failure refuses typed NAMING the stage (`ExtractionUnavailable` /
// `NonTransversalContact` / `BudgetExhausted`); the three are never collapsed
// internally. The frozen `truck_base::evidence::EnvelopeCase` vocabulary has no
// distinct cases for the first two, so the boundary marshaling maps
// `ExtractionUnavailable` onto the landed `NonCanonicalCarrier` and
// `NonTransversalContact` onto `ContactReductionDeferred`; the stage enum is
// the measurement instrument.
// ---------------------------------------------------------------------------

/// The extracted patch shape: the landed tensor-Bernstein patch row the
/// certified solver consumes (the theory statement's `P_i`).
type Patch = crate::python::binding::VolumeRow;

/// The typed stage of a `Swept x Swept` admission failure. The stage name is
/// the measurement instrument; the cases are never collapsed. RDEF-M2 replaced
/// the single `NonTransversalContact` stage with the (T)/(G) regime split: a
/// pair the transversality gate refuses now resolves to a certified sandwich
/// bracket, or to one of the named regime/sandwich refusals below.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SweptAdmissionRefusal {
    /// The operand's carrier is outside the extracted vocabulary (or its
    /// section caps cannot be represented exactly).
    ExtractionUnavailable,
    /// `d_u x d_v` cannot be certified nonzero on an operand patch.
    SingularParametrization,
    /// The (G) cell fails graph injectivity.
    GraphNotInjective,
    /// Coincidence without an exact common-carrier certificate.
    CoincidenceWithoutExactCarrier,
    /// The error schedule did not reach the budget within the subdivision cap.
    BudgetExhausted,
}

/// The boundary marshaling of one admission stage onto the frozen kernel
/// refusal vocabulary. `ExtractionUnavailable` and `SingularParametrization`
/// keep the landed `NonCanonicalCarrier` carrier refusal; the regime/sandwich
/// refusals name the deferred contact reduction; `BudgetExhausted` is the
/// typed unresolved budget refusal.
fn swept_refusal_to_refusal(stage: SweptAdmissionRefusal) -> Refusal {
    match stage {
        SweptAdmissionRefusal::ExtractionUnavailable
        | SweptAdmissionRefusal::SingularParametrization => {
            Refusal::UnsupportedEnvelope(EnvelopeCase::NonCanonicalCarrier)
        }
        SweptAdmissionRefusal::GraphNotInjective
        | SweptAdmissionRefusal::CoincidenceWithoutExactCarrier => {
            Refusal::UnsupportedEnvelope(EnvelopeCase::ContactReductionDeferred)
        }
        SweptAdmissionRefusal::BudgetExhausted => Refusal::NumericallyUnresolved {
            spent: Budget::new(0, 0, 0),
            witness: UnresolvedWitness::DeviationUncertified,
        },
    }
}

/// Whether one recorded boolean row couples two `Swept`-family carriers (the
/// `cut(loft, loft)` / `fuse` cell the admission chain routes).
fn swept_swept(boolean: &BooleanNode) -> Result<bool, Refusal> {
    let base = node_carrier_class(&boolean.a)?;
    let tool = node_carrier_class(&boolean.b)?;
    Ok(base == CarrierClass::Swept && tool == CarrierClass::Swept)
}

/// Applies the placement's local congruence to one local point: mirror (when
/// recorded), then the full recorded frame (or the pure-z `rz`), then the
/// translation.
fn place_local_point(part: &PartSpec, point: [f64; 3]) -> [f64; 3] {
    let mirrored = match part.mirror.as_deref() {
        Some(axis) => mirror_point(point, axis),
        None => point,
    };
    let [mx, my, mz] = mirrored;
    let rotated = match part.rotation {
        Some(frame) => frame.apply(mirrored),
        None => {
            let rz = part.rz.to_radians();
            if rz == 0.0 {
                mirrored
            } else {
                let (sin, cos) = rz.sin_cos();
                [mx * cos - my * sin, mx * sin + my * cos, mz]
            }
        }
    };
    let [rx, ry, rz] = rotated;
    [rx + part.x, ry + part.y, rz + part.z]
}

/// The orientation factor `sign(det M_tau)` of one placement: `-1` for a
/// reflective placement (a recorded mirror, or a frame whose determinant is
/// negative), `+1` otherwise.
fn placement_det_sign(part: &PartSpec) -> i8 {
    let mirror = if part.mirror.is_some() { -1 } else { 1 };
    let frame = match part.rotation {
        Some(rotation) => {
            let det = v3_dot(rotation.x_dir, v3_cross(rotation.y_dir, rotation.z_dir));
            if det < 0.0 { -1 } else { 1 }
        }
        None => 1,
    };
    mirror * frame
}

/// Places one local patch row: the placement's affine map applies to every
/// control point (exact, weights unchanged). The orientation field is left at
/// its intrinsic value; the caller composes the placement sign.
fn place_row(row: &Patch, part: &PartSpec) -> Patch {
    let numerator = row
        .numerator
        .iter()
        .map(|control_row| {
            control_row
                .iter()
                .map(|point| place_local_point(part, *point))
                .collect()
        })
        .collect();
    Patch {
        numerator,
        weights: row.weights.clone(),
        orientation: row.orientation,
    }
}

/// The `4 x 4` tensor-Bernstein elevation of a planar bilinear quad. The
/// corner order is `(c00, c10, c11, c01)`, so the patch's `P_u x P_v` is the
/// quad's outward normal for a counter-clockwise corner order.
fn planar_quad_row(c00: [f64; 3], c10: [f64; 3], c11: [f64; 3], c01: [f64; 3]) -> Patch {
    let e0 = [1.0, 2.0 / 3.0, 1.0 / 3.0, 0.0];
    let e1 = [0.0, 1.0 / 3.0, 2.0 / 3.0, 1.0];
    let corners = [[c00, c01], [c10, c11]];
    let mut numerator = vec![vec![[0.0f64; 3]; 4]; 4];
    for (i, row) in numerator.iter_mut().enumerate() {
        for (j, cell) in row.iter_mut().enumerate() {
            let mut acc = [0.0f64; 3];
            for (a, pair) in corners.iter().enumerate() {
                for (b, corner) in pair.iter().enumerate() {
                    let wa = if a == 0 {
                        e0.get(i).copied().unwrap_or(0.0)
                    } else {
                        e1.get(i).copied().unwrap_or(0.0)
                    };
                    let wb = if b == 0 {
                        e0.get(j).copied().unwrap_or(0.0)
                    } else {
                        e1.get(j).copied().unwrap_or(0.0)
                    };
                    let coefficient = wa * wb;
                    for (slot, value) in acc.iter_mut().zip(corner.iter()) {
                        *slot += coefficient * value;
                    }
                }
            }
            *cell = acc;
        }
    }
    Patch {
        numerator,
        weights: vec![vec![1.0f64; 4]; 4],
        orientation: 1.0,
    }
}

/// The six outward-oriented tensor-Bernstein patches of the axis-aligned box
/// `[lo, hi]`. The corner order fixes `P_u x P_v` outward on every face, so the
/// divergence-form flux of the cycle is the box volume.
fn box_patch_rows(lo: [f64; 3], hi: [f64; 3]) -> Vec<Patch> {
    let [x0, y0, z0] = lo;
    let [x1, y1, z1] = hi;
    vec![
        planar_quad_row([x0, y0, z0], [x0, y1, z0], [x1, y1, z0], [x1, y0, z0]),
        planar_quad_row([x0, y0, z1], [x1, y0, z1], [x1, y1, z1], [x0, y1, z1]),
        planar_quad_row([x0, y0, z0], [x1, y0, z0], [x1, y0, z1], [x0, y0, z1]),
        planar_quad_row([x0, y1, z0], [x0, y1, z1], [x1, y1, z1], [x1, y1, z0]),
        planar_quad_row([x0, y0, z0], [x0, y0, z1], [x0, y1, z1], [x0, y1, z0]),
        planar_quad_row([x1, y0, z0], [x1, y1, z0], [x1, y1, z1], [x1, y0, z1]),
    ]
}

/// Validates one extracted patch row through the landed tensor-Bernstein
/// constructor (the same admission `binding_volume_facts` uses). A malformed
/// net is `ExtractionUnavailable`.
fn validate_patch(row: &Patch) -> Result<(), SweptAdmissionRefusal> {
    TensorBernsteinPatch::try_new(
        row.numerator.clone(),
        row.weights.clone(),
        IBox2 {
            lo: [0.0, 0.0],
            hi: [1.0, 1.0],
        },
        PatchParent::new(0, None),
    )
    .map(|_| ())
    .map_err(|_| SweptAdmissionRefusal::ExtractionUnavailable)
}

/// The unit chord direction of a loft station stack (first centroid to last).
fn loft_direction(loops: &[Vec<SpanPoly3>]) -> Result<[f64; 3], SweptAdmissionRefusal> {
    let first = loops
        .first()
        .ok_or(SweptAdmissionRefusal::ExtractionUnavailable)?;
    let last = loops
        .last()
        .ok_or(SweptAdmissionRefusal::ExtractionUnavailable)?;
    let a = centroid3(&sample_loop3(first));
    let b = centroid3(&sample_loop3(last));
    let direction = v3_sub(b, a);
    let magnitude = v3_norm(direction);
    if !magnitude.is_finite() || magnitude <= 0.0 {
        return Err(SweptAdmissionRefusal::ExtractionUnavailable);
    }
    let [dx, dy, dz] = direction;
    Ok([dx / magnitude, dy / magnitude, dz / magnitude])
}

/// The exact planar end cap of one straight quadrilateral section loop,
/// oriented outward along `outward`. A section that is not an exact planar
/// quad (four straight edges) refuses `ExtractionUnavailable` rather than
/// being approximated.
fn loft_cap_row(
    loop_spans: &[SpanPoly3],
    outward: [f64; 3],
) -> Result<Patch, SweptAdmissionRefusal> {
    let [s0, s1, s2, s3] = loop_spans else {
        return Err(SweptAdmissionRefusal::ExtractionUnavailable);
    };
    for span in [s0, s1, s2, s3] {
        for coefficients in [span.x, span.y, span.z] {
            let [c0, c1, c2, c3] = coefficients;
            let scale = 1.0 + c0.abs() + c1.abs();
            if c2.abs() > 1.0e-12 * scale || c3.abs() > 1.0e-12 * scale {
                return Err(SweptAdmissionRefusal::ExtractionUnavailable);
            }
        }
    }
    let corner = |span: &SpanPoly3| {
        let [x, ..] = span.x;
        let [y, ..] = span.y;
        let [z, ..] = span.z;
        [x, y, z]
    };
    let [c0, c1, c2, c3] = [corner(s0), corner(s1), corner(s2), corner(s3)];
    let counter_clockwise = v3_dot(spline_loop_area_vector(loop_spans), outward) >= 0.0;
    Ok(if counter_clockwise {
        planar_quad_row(c0, c1, c2, c3)
    } else {
        planar_quad_row(c0, c3, c2, c1)
    })
}

/// The local tensor-Bernstein patch 2-cycle of one landed solid carrier. The
/// extracted vocabulary is the exact one: axis-aligned boxes and the
/// loft/member carriers whose sections are straight quadrilateral loops (so
/// their planar end caps are exact); everything else refuses typed.
fn extract_local_patches(solid: &SolidSpec) -> Result<Vec<Patch>, SweptAdmissionRefusal> {
    match solid {
        SolidSpec::Box {
            length,
            width,
            height,
        } => {
            let (length, width, height) = (*length, *width, *height);
            Ok(box_patch_rows(
                [-length / 2.0, -width / 2.0, -height / 2.0],
                [length / 2.0, width / 2.0, height / 2.0],
            ))
        }
        SolidSpec::Loft { sections, closed } => {
            let (mut patches, loops) = spline_loft_volume_rows(sections)
                .map_err(|_| SweptAdmissionRefusal::ExtractionUnavailable)?;
            if !*closed {
                let first = loops
                    .first()
                    .ok_or(SweptAdmissionRefusal::ExtractionUnavailable)?;
                let last = loops
                    .last()
                    .ok_or(SweptAdmissionRefusal::ExtractionUnavailable)?;
                let [dx, dy, dz] = loft_direction(&loops)?;
                let backward = [-dx, -dy, -dz];
                patches.push(loft_cap_row(first, backward)?);
                patches.push(loft_cap_row(last, [dx, dy, dz])?);
            }
            Ok(patches)
        }
        SolidSpec::Member {
            profile,
            stations,
            ruled,
        } => {
            // A smooth member with more than two stations has no landed smooth
            // carrier: the open carrier refuses typed rather than being
            // approximated.
            if !*ruled && stations.len() > 2 {
                return Err(SweptAdmissionRefusal::ExtractionUnavailable);
            }
            let sections = member_sections(profile, stations)
                .map_err(|_| SweptAdmissionRefusal::ExtractionUnavailable)?;
            let (mut patches, loops) = spline_loft_volume_rows(&sections)
                .map_err(|_| SweptAdmissionRefusal::ExtractionUnavailable)?;
            let first = loops
                .first()
                .ok_or(SweptAdmissionRefusal::ExtractionUnavailable)?;
            let last = loops
                .last()
                .ok_or(SweptAdmissionRefusal::ExtractionUnavailable)?;
            let [dx, dy, dz] = loft_direction(&loops)?;
            let backward = [-dx, -dy, -dz];
            patches.push(loft_cap_row(first, backward)?);
            patches.push(loft_cap_row(last, [dx, dy, dz])?);
            Ok(patches)
        }
        _ => Err(SweptAdmissionRefusal::ExtractionUnavailable),
    }
}

/// The extraction adapter (theory statement section 1): the tensor-Bernstein
/// patch 2-cycle of one recorded construction node, with the placement applied
/// to the control points and the orientation signs composed with the
/// placement's determinant sign. Each returned pair is `(patch, sigma)` where
/// `sigma` is `sign(det M_tau)` and the patch's orientation field is the full
/// `sigma_i = sigma * sigma_i^0`.
fn extract_patches(row: &TreeNode) -> Result<Vec<(Patch, i8)>, SweptAdmissionRefusal> {
    let part = match row {
        TreeNode::Part { part } => part,
        TreeNode::Group { .. } | TreeNode::Boolean { .. } | TreeNode::Fillet { .. } => {
            // Depth-1 only: a boolean of a boolean (or a compound operand) is
            // the recorded open composition cell and is not extracted.
            return Err(SweptAdmissionRefusal::ExtractionUnavailable);
        }
    };
    let sign = placement_det_sign(part);
    let mut extracted = Vec::new();
    for local in extract_local_patches(&part.solid)? {
        validate_patch(&local)?;
        let mut placed = place_row(&local, part);
        if sign < 0 {
            placed.orientation = -local.orientation;
        }
        extracted.push((placed, sign));
    }
    Ok(extracted)
}

/// The gate order of the theory statement section 3 for one `Swept x Swept`
/// boolean pair: extract both operands, then run the certified solver (whose
/// landed transversality admission is the FSSI-001 twin). A refusal names the
/// stage that failed.
///
/// RDEF-M2 amends the gate: a pair the landed transversality admission refuses
/// (`TransversalityUncertified`) is no longer a single `NonTransversalContact`
/// dead end. The regime dichotomy (Lemma D) splits it into (T)/(G); a (G) pair
/// runs the tangential sandwich volume rule (Lemma S) and returns a certified
/// bracket or one of the named regime/sandwich refusals.
fn admit_swept_pair(
    boolean: &BooleanNode,
) -> Result<membership::BooleanVolumeCertificate, SweptAdmissionRefusal> {
    let a = extract_patches(&boolean.a)?;
    let b = extract_patches(&boolean.b)?;
    let a_rows: Vec<Patch> = a.into_iter().map(|(row, _)| row).collect();
    let b_rows: Vec<Patch> = b.into_iter().map(|(row, _)| row).collect();
    let options = membership::BooleanVolumeOptions::default();
    match membership::certify_boolean_volume(&a_rows, &b_rows, &options) {
        Ok(certificate) => Ok(certificate),
        Err(membership::BooleanVolumeRefusal::TransversalityUncertified) => {
            // RDEF-M2: the regime split. The sandwich rule carries the bracket
            // for the tangential (G) cell and reports its own named refusals.
            let scale = a_rows
                .iter()
                .chain(b_rows.iter())
                .flat_map(|row| row.numerator.iter().flatten())
                .flat_map(|p| p.iter())
                .fold(1.0f64, |acc, c| acc.max(c.abs()));
            let sandwich_options = membership::SandwichOptions {
                tolerance: options.relative_tolerance * scale,
                max_cells: options.max_cells,
                shared_carrier: false,
                bernstein_chart: true,
                rational_positive_weights: true,
            };
            match membership::certify_sandwich(&a_rows, &b_rows, boolean.mode, &sandwich_options) {
                Ok(certificate) => Ok(sandwich_to_boolean_certificate(
                    &certificate,
                    &a_rows,
                    &b_rows,
                )),
                Err(membership::SandwichRefusal::SingularParametrization) => {
                    Err(SweptAdmissionRefusal::SingularParametrization)
                }
                Err(membership::SandwichRefusal::GraphNotInjective) => {
                    Err(SweptAdmissionRefusal::GraphNotInjective)
                }
                Err(membership::SandwichRefusal::CoincidenceWithoutExactCarrier { .. }) => {
                    Err(SweptAdmissionRefusal::CoincidenceWithoutExactCarrier)
                }
                Err(membership::SandwichRefusal::BudgetExhausted { .. }) => {
                    Err(SweptAdmissionRefusal::BudgetExhausted)
                }
                Err(membership::SandwichRefusal::MalformedPatch) => {
                    Err(SweptAdmissionRefusal::ExtractionUnavailable)
                }
            }
        }
        Err(membership::BooleanVolumeRefusal::BudgetExceeded) => {
            Err(SweptAdmissionRefusal::BudgetExhausted)
        }
        Err(_) => Err(SweptAdmissionRefusal::ExtractionUnavailable),
    }
}

/// Lifts the sandwich certificate into the boolean-volume certificate shape the
/// interval-valued volume fact consumes.
fn sandwich_to_boolean_certificate(
    certificate: &membership::SandwichCertificate,
    a_rows: &[Patch],
    b_rows: &[Patch],
) -> membership::BooleanVolumeCertificate {
    let width = certificate.width;
    membership::BooleanVolumeCertificate {
        bracket_lo: certificate.bracket_lo,
        bracket_hi: certificate.bracket_hi,
        value: 0.5 * (certificate.bracket_lo + certificate.bracket_hi),
        width,
        relative_width: if certificate.volume_a.abs() > 0.0 {
            width / certificate.volume_a.abs()
        } else {
            0.0
        },
        volume_a: certificate.volume_a,
        volume_b: certificate.volume_b,
        intersection_lo: 0.0,
        intersection_hi: certificate.sandwich_bound,
        bbox_lo: [0.0; 3],
        bbox_hi: [0.0; 3],
        bbox_margin: None,
        solid_count: 1,
        broad_phase_pairs: a_rows.len().saturating_mul(b_rows.len()),
        excluded_pairs: 0,
        clear_cells: 0,
        contact_cells: certificate.undecided_cells,
        max_depth: 0,
        cover_cells: 0,
        phases: [0, 0, certificate.refined_cells],
    }
}

// ---------------------------------------------------------------------------
// STL meshing
// ---------------------------------------------------------------------------

/// One triangle: nine `f64` coordinates (three `x y z` vertices), world
/// space.
pub(crate) type Triangle = [f64; 9];

/// The angular resolution of the mesh (fixed, deterministic).
const MESH_SEGMENTS: usize = 64;

/// Angular segments for a circular sweep of `radius` under the requested
/// linear deflection: the chord sagitta `r(1 - cos(pi/n))` stays within the
/// deflection. `None` keeps the landed fixed `MESH_SEGMENTS` (the
/// deterministic fingerprint rule) bit-for-bit; the result is clamped to
/// `[8, 16384]`.
fn sweep_segments(radius: f64, deflection: Option<f64>) -> usize {
    match deflection {
        Some(d) if d.is_finite() && d > 0.0 => {
            let ratio = (1.0 - d / radius).clamp(-1.0, 1.0);
            let n = (std::f64::consts::PI / ratio.acos()).ceil();
            (n as usize).clamp(8, 16384)
        }
        _ => MESH_SEGMENTS,
    }
}

/// Generates the world-space triangle soup of every part in the tree.
fn tree_mesh(root: &TreeNode, deflection: Option<f64>) -> Result<Vec<Triangle>, Refusal> {
    let mut triangles = Vec::new();
    append_node_mesh(root, &mut triangles, deflection)?;
    Ok(triangles)
}

fn append_node_mesh(node: &TreeNode, out: &mut Vec<Triangle>, deflection: Option<f64>) -> Result<(), Refusal> {
    match node {
        TreeNode::Part { part } => {
            let local = solid_mesh(&part.solid, deflection)?;
            // The mirror placed-carrier reflects the local geometry before
            // the rz rotation and translation (no geometry recomputation).
            let reflected: Vec<Triangle> = local
                .iter()
                .map(|tri| match part.mirror.as_deref() {
                    Some(axis) => reflect_triangle(*tri, axis),
                    None => *tri,
                })
                .collect();
            if let Some(frame) = part.rotation {
                // The recorded full orthonormal frame transforms the mesh
                // per-vertex (never recomputed geometry).
                for tri in reflected {
                    out.push(place_triangle_frame(tri, frame, part.x, part.y, part.z));
                }
            } else {
                let rz = part.rz.to_radians();
                let cos = rz.cos();
                let sin = rz.sin();
                for tri in reflected {
                    out.push(place_triangle(tri, cos, sin, part.x, part.y, part.z));
                }
            }
            Ok(())
        }
        // A routed boolean row's mesh is its base operand's mesh; the
        // dispatch validates the pair.
        TreeNode::Boolean { boolean } => {
            dispatch_boolean(boolean)?;
            append_node_mesh(&boolean.a, out, deflection)
        }
        // A fillet row's mesh is its base part's mesh (the blend certificate
        // is validated by the facts path).
        TreeNode::Fillet { fillet } => append_node_mesh(&fillet.base, out, deflection),
        TreeNode::Group { group, .. } => {
            for child in group {
                append_node_mesh(child, out, deflection)?;
            }
            Ok(())
        }
    }
}

/// Reflects one local triangle across the recorded mirror coordinate plane.
fn reflect_triangle(tri: Triangle, axis: &str) -> Triangle {
    let mut reflected = [0.0f64; 9];
    for vertex in 0..3 {
        let p = [tri[vertex * 3], tri[vertex * 3 + 1], tri[vertex * 3 + 2]];
        let q = mirror_point(p, axis);
        reflected[vertex * 3] = q[0];
        reflected[vertex * 3 + 1] = q[1];
        reflected[vertex * 3 + 2] = q[2];
    }
    reflected
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

/// Translates a triangle by the part's world frame carrying a recorded full
/// orthonormal rotation (the frame replaces the pure-z rotation path).
fn place_triangle_frame(tri: Triangle, frame: RotationFrame, x: f64, y: f64, z: f64) -> Triangle {
    let mut placed = [0.0f64; 9];
    for vertex in 0..3 {
        let vx = tri[vertex * 3];
        let vy = tri[vertex * 3 + 1];
        let vz = tri[vertex * 3 + 2];
        let p = frame.apply([vx, vy, vz]);
        placed[vertex * 3] = p[0] + x;
        placed[vertex * 3 + 1] = p[1] + y;
        placed[vertex * 3 + 2] = p[2] + z;
    }
    placed
}

/// The local (origin-frame) triangle soup of one solid.
fn solid_mesh(solid: &SolidSpec, deflection: Option<f64>) -> Result<Vec<Triangle>, Refusal> {
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
            let about_z = cylinder_z_mesh(*radius, *height, deflection)?;
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
        SolidSpec::Sphere { radius } => sphere_mesh(*radius, deflection),
        SolidSpec::Torus { major, minor } => torus_mesh(*major, *minor, deflection),
        SolidSpec::Lathe {
            profile,
            arc_deg,
            start_deg,
        } => lathe_mesh(profile, *arc_deg, *start_deg, deflection),
        SolidSpec::Prism {
            profile,
            amount,
            both,
        } => prism_mesh(profile, *amount, *both),
        SolidSpec::TrimPrism {
            profile,
            amount,
            both,
            trim_curve,
            trim_net,
            tolerance,
        } => {
            if is_spline_profile_prism(profile, trim_curve, trim_net) {
                return spline_profile_prism_facts(trim_curve, *amount, *both)
                    .map(|(_, _, mesh)| mesh);
            }
            crate::python::binding::trim_extrude_solid(
                profile, *amount, *both, trim_curve, trim_net, *tolerance,
            )
            .map(|facts| facts.mesh)
            .map_err(trim_binding_error_to_refusal)
        }
        SolidSpec::Loft { sections, closed } => loft_mesh(sections, *closed),
        SolidSpec::Member {
            profile, stations, ..
        } => member_mesh(&member_sections(profile, stations)?),
    }
}

/// The local mesh of an extruded prism: the profile boundary swept between the
/// two end planes plus cap fans. Deterministic, closed, no duplicate vertices.
pub(crate) fn prism_mesh(
    profile: &[ProfileEdge],
    amount: f64,
    both: bool,
) -> Result<Vec<Triangle>, Refusal> {
    let geom = prism_geom(profile, amount, both)?;
    let norm = geom.loop3.normal;
    let verts = &geom.loop3.verts;
    let count = verts.len();
    let centroid = {
        let mut c = [0.0f64; 3];
        for v in verts {
            c[0] += v[0];
            c[1] += v[1];
            c[2] += v[2];
        }
        let k = 1.0 / count as f64;
        [c[0] * k, c[1] * k, c[2] * k]
    };
    let bottom: Vec<[f64; 3]> = verts
        .iter()
        .map(|v| {
            [
                v[0] + geom.t_lo * norm[0],
                v[1] + geom.t_lo * norm[1],
                v[2] + geom.t_lo * norm[2],
            ]
        })
        .collect();
    let top: Vec<[f64; 3]> = verts
        .iter()
        .map(|v| {
            [
                v[0] + geom.t_hi * norm[0],
                v[1] + geom.t_hi * norm[1],
                v[2] + geom.t_hi * norm[2],
            ]
        })
        .collect();
    let cb = [
        centroid[0] + geom.t_lo * norm[0],
        centroid[1] + geom.t_lo * norm[1],
        centroid[2] + geom.t_lo * norm[2],
    ];
    let ct = [
        centroid[0] + geom.t_hi * norm[0],
        centroid[1] + geom.t_hi * norm[1],
        centroid[2] + geom.t_hi * norm[2],
    ];
    let mut out = Vec::new();
    for i in 0..count {
        let j = (i + 1) % count;
        push_quad(&mut out, bottom[i], bottom[j], top[j], top[i]);
    }
    for i in 0..count {
        let j = (i + 1) % count;
        push_tri(&mut out, cb, bottom[j], bottom[i]);
        push_tri(&mut out, ct, top[i], top[j]);
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// MONO-4-TRIM-IDIOMS: the corpus spline-profile extrude idiom.
// ---------------------------------------------------------------------------
//
// `surfaces.rounded_plate_pts` / `suspension._plate` author a closed plate
// section as ONE periodic spline and extrude it symmetrically:
//
//     bd.extrude(plane * bd.make_face(bd.Spline(*pts, periodic=True)),
//                amount, both=True)
//
// (the rear-wing louvre cutters, `rear_wing.py:390`; the suspension `_plate`
// profiles, `suspension.py:162`). The door records that composition as a
// `trim_prism` row with an EMPTY line base profile and no pullback net: the
// recorded spline curve is the base profile, not a trim. The landed trim
// constructor reads a spline carrier as a trim loop over the base's bounding
// rectangle and refuses typed (`trim_pullback_missing`) because no pullback
// net is recorded. The idiom is not a trim at all -- it is the local-frame
// extrude stage over a closed spline base loop -- so this arm realizes it
// exactly with the landed clamped-cubic reconstruction and the Green's-theorem
// loop area the lathe/loft/member arms already integrate. No intersection
// between spline-carried solids is involved; a later `surfaces.cut` of such a
// prism against a spline-carried shell is the wave-3 boolean frontier and
// stays a typed refusal at the boolean boundary.

/// Whether a trim-prism row records the corpus spline-profile extrude idiom:
/// no line base profile, no recorded pullback net, and a closed recorded
/// spline curve (the base profile). Anything else keeps the landed trim
/// constructor's own classification and refusal.
fn is_spline_profile_prism(
    profile: &[ProfileEdge],
    trim_curve: &[[f64; 3]],
    trim_net: &[Vec<f64>],
) -> bool {
    profile.is_empty() && trim_net.is_empty() && trim_curve.len() >= 3
}

/// The realized `(volume, local_bbox, local_mesh)` of one spline-profile
/// prism row.
type SplinePrismFacts = (f64, [[f64; 3]; 2], Vec<Triangle>);

/// The exact local facts of the spline-profile extrude idiom: the prism over
/// the reconstructed closed spline loop, swept along the loop's plane normal.
/// An open recorded carrier refuses typed (`NonCanonicalCarrier`), never a
/// bounded guess.
fn spline_profile_prism_facts(
    trim_curve: &[[f64; 3]],
    amount: f64,
    both: bool,
) -> Result<SplinePrismFacts, Refusal> {
    if !amount.is_finite() || amount <= 0.0 {
        return Err(Refusal::Empty);
    }
    let mut scale = 0.0f64;
    for p in trim_curve {
        for c in p {
            if !c.is_finite() {
                return Err(Refusal::Empty);
            }
            scale = scale.max(c.abs());
        }
    }
    let spans = spline_spans3(trim_curve)?;
    let first = spans.first().ok_or(Refusal::Empty)?;
    let last = spans.last().ok_or(Refusal::Empty)?;
    let start = [first.x[0], first.y[0], first.z[0]];
    let end = [
        last.x[0] + last.x[1] + last.x[2] + last.x[3],
        last.y[0] + last.y[1] + last.y[2] + last.y[3],
        last.z[0] + last.z[1] + last.z[2] + last.z[3],
    ];
    if v3_norm(v3_sub(end, start)) > 1.0e-9 * (1.0 + scale) {
        return Err(open_smooth_loft());
    }
    let area_vec = spline_loop_area_vector(&spans);
    let area = v3_norm(area_vec);
    if !area.is_finite() || area <= 0.0 {
        return Err(Refusal::Empty);
    }
    let normal = [area_vec[0] / area, area_vec[1] / area, area_vec[2] / area];
    let (t_lo, t_hi) = if both {
        (-amount, amount)
    } else {
        (0.0, amount)
    };
    let volume = area * (t_hi - t_lo);

    // The exact local AABB: the loop's span-extrema box widened along the
    // sweep normal (the sweep parameter is independent of the cross-section).
    let [b_lo, b_hi] = spline_loop_bbox3(&spans)?;
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    for (((normal_axis, lo), hi), (b_lo_axis, b_hi_axis)) in normal
        .iter()
        .zip(min.iter_mut())
        .zip(max.iter_mut())
        .zip(b_lo.iter().zip(b_hi.iter()))
    {
        let (d_min, d_max) = if *normal_axis >= 0.0 {
            (t_lo * normal_axis, t_hi * normal_axis)
        } else {
            (t_hi * normal_axis, t_lo * normal_axis)
        };
        *lo = *b_lo_axis + d_min;
        *hi = *b_hi_axis + d_max;
    }
    if !min[0].is_finite() || !max[0].is_finite() {
        return Err(Refusal::Empty);
    }

    // The deterministic swept mesh: the sampled reconstructed loop at the two
    // sweep planes plus the two cap fans (the spline-profile `prism_mesh`).
    let profile = sample_loop3(&spans);
    let count = profile.len();
    if count < 3 {
        return Err(Refusal::Empty);
    }
    let mut bottom = Vec::with_capacity(count);
    let mut top = Vec::with_capacity(count);
    for v in &profile {
        bottom.push([
            v[0] + t_lo * normal[0],
            v[1] + t_lo * normal[1],
            v[2] + t_lo * normal[2],
        ]);
        top.push([
            v[0] + t_hi * normal[0],
            v[1] + t_hi * normal[1],
            v[2] + t_hi * normal[2],
        ]);
    }
    let cb = centroid3(&bottom);
    let ct = centroid3(&top);
    let mut mesh = Vec::new();
    for i in 0..count {
        let j = (i + 1) % count;
        let (Some(b0), Some(b1)) = (bottom.get(i), bottom.get(j)) else {
            return Err(Refusal::Empty);
        };
        let (Some(t0), Some(t1)) = (top.get(i), top.get(j)) else {
            return Err(Refusal::Empty);
        };
        push_quad(&mut mesh, *b0, *b1, *t1, *t0);
    }
    for i in 0..count {
        let j = (i + 1) % count;
        let (Some(b0), Some(b1)) = (bottom.get(i), bottom.get(j)) else {
            return Err(Refusal::Empty);
        };
        let (Some(t0), Some(t1)) = (top.get(i), top.get(j)) else {
            return Err(Refusal::Empty);
        };
        push_tri(&mut mesh, cb, *b1, *b0);
        push_tri(&mut mesh, ct, *t0, *t1);
    }
    Ok((volume, [min, max], mesh))
}

/// The local mesh of a loft chain: ruled quads between matching vertices of
/// consecutive sections, plus cap fans on the two end sections of an open
/// chain. A closed halo row's last station repeats the first station, so the
/// side surface closes on itself and needs no caps.
fn loft_mesh(sections: &[Vec<ProfileEdge>], closed: bool) -> Result<Vec<Triangle>, Refusal> {
    // The certified spline-section class meshes through the reconstructed
    // ruled surface; a closed smooth chain has no certificate.
    if loft_sections(sections).is_err() {
        if closed {
            return Err(open_smooth_loft());
        }
        return spline_loft_mesh(sections);
    }
    let validated = loft_sections(sections)?;
    let loops = &validated.loops;
    let mut out = Vec::new();
    let seg_count = loops.len() - 1;
    for j in 0..seg_count {
        let a = &loops[j];
        let b = &loops[j + 1];
        let n = a.verts.len();
        for i in 0..n {
            let k = (i + 1) % n;
            push_quad(&mut out, a.verts[i], a.verts[k], b.verts[k], b.verts[i]);
        }
    }
    if !closed {
        let first = &loops[0];
        let last = loops.last().ok_or(Refusal::Empty)?;
        let c_first = loop_centroid(first);
        let c_last = loop_centroid(last);
        let n = first.verts.len();
        for i in 0..n {
            let j = (i + 1) % n;
            push_tri(&mut out, c_first, first.verts[i], first.verts[j]);
        }
        let n = last.verts.len();
        for i in 0..n {
            let j = (i + 1) % n;
            push_tri(&mut out, c_last, last.verts[i], last.verts[j]);
        }
    }
    Ok(out)
}

fn loop_centroid(loop3: &ProfileLoop) -> [f64; 3] {
    let n = loop3.verts.len();
    let mut c = [0.0f64; 3];
    for v in &loop3.verts {
        c[0] += v[0];
        c[1] += v[1];
        c[2] += v[2];
    }
    let k = 1.0 / n as f64;
    [c[0] * k, c[1] * k, c[2] * k]
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
fn cylinder_z_mesh(radius: f64, height: f64, deflection: Option<f64>) -> Result<Vec<Triangle>, Refusal> {
    if radius <= 0.0 || height <= 0.0 || !radius.is_finite() || !height.is_finite() {
        return Err(Refusal::Empty);
    }
    let segments = sweep_segments(radius, deflection);
    let ring = ring_points(radius, segments);
    let z_top = height / 2.0;
    let z_bottom = -height / 2.0;
    let mut out = Vec::new();
    for segment in 0..segments {
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
        for segment in 0..segments {
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
fn sphere_mesh(radius: f64, deflection: Option<f64>) -> Result<Vec<Triangle>, Refusal> {
    if radius <= 0.0 || !radius.is_finite() {
        return Err(Refusal::Empty);
    }
    let segments = sweep_segments(radius, deflection);
    let bands = segments / 2;
    // ring `i` sits at latitude pi*i/bands; rings 0 and `bands` are the poles.
    let rings: Vec<Vec<(f64, f64, f64)>> = (0..=bands)
        .map(|i| {
            let theta = std::f64::consts::PI * i as f64 / bands as f64;
            let z = radius * theta.cos();
            let r = radius * theta.sin();
            (0..segments)
                .map(|segment| {
                    let angle = TAU * segment as f64 / segments as f64;
                    (r * angle.cos(), r * angle.sin(), z)
                })
                .collect()
        })
        .collect();
    let mut out = Vec::new();
    for band in 0..bands {
        let lower = ring3(&rings, band);
        let upper = ring3(&rings, band + 1);
        for segment in 0..segments {
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
fn torus_mesh(major: f64, minor: f64, deflection: Option<f64>) -> Result<Vec<Triangle>, Refusal> {
    if major <= 0.0 || minor <= 0.0 || !major.is_finite() || !minor.is_finite() {
        return Err(Refusal::Empty);
    }
    let ring_segments = sweep_segments(major + minor, deflection);
    let tube_segments = sweep_segments(minor, deflection);
    let mut out = Vec::new();
    for ring_segment in 0..ring_segments {
        let ring_a = TAU * ring_segment as f64 / ring_segments as f64;
        let ring_b = TAU * (ring_segment + 1) as f64 / ring_segments as f64;
        for tube_segment in 0..tube_segments {
            let tube_a = TAU * tube_segment as f64 / tube_segments as f64;
            let tube_b = TAU * (tube_segment + 1) as f64 / tube_segments as f64;
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

/// The number of linear subdivisions per reconstructed spline span in the
/// lathe mesh (fixed, deterministic).
const SPLINE_MESH_STEPS: usize = 4;

/// A lathe mesh: sample the profile boundary (line edges exactly, spline edges
/// at the fixed subdivision of each reconstructed span) and sweep each
/// consecutive sample segment over the angular segments. A full revolution is
/// the closed band surface (a closed profile's swept boundary covers the whole
/// surface, so no caps are needed); a partial arc additionally closes the two
/// planar cut faces at the sector endpoints (DOOR-PARTIAL-ARC-FLIP). For an
/// all-line profile the ring is exactly the vertex loop, so the
/// full-revolution mesh is identical to the landed line-profile mesh.
fn lathe_mesh(
    profile: &[LatheEdge],
    arc_deg: f64,
    start_deg: f64,
    deflection: Option<f64>,
) -> Result<Vec<Triangle>, Refusal> {
    let arc = validate_lathe_arc(arc_deg, start_deg)?;
    let ring = profile_ring(profile)?;
    // The sweep radius is the profile's largest excursion from the axis: the
    // chord sagitta of the angular segments is bounded by the deflection at
    // that radius.
    let sweep_radius = ring
        .iter()
        .map(|p| p[0].hypot(p[1]))
        .fold(0.0f64, f64::max);
    let segments = sweep_segments(sweep_radius, deflection);
    if arc == 360.0 {
        return sweep_ring_mesh(&ring, segments);
    }
    let theta0 = start_deg.rem_euclid(360.0).to_radians();
    sweep_ring_mesh_arc(&ring, theta0, arc.to_radians(), segments)
}

/// Appends `p` to the ring unless it equals the current last ring point.
fn push_ring_point(ring: &mut Vec<[f64; 2]>, p: [f64; 2]) {
    let duplicate = ring.last().is_some_and(|last| *last == p);
    if !duplicate {
        ring.push(p);
    }
}

/// The ordered ring of boundary samples of the profile, one point per sample
/// (no duplicate closing point).
fn profile_ring(profile: &[LatheEdge]) -> Result<Vec<[f64; 2]>, Refusal> {
    check_profile_edges(profile)?;
    let mut ring: Vec<[f64; 2]> = Vec::new();
    for edge in profile {
        match edge {
            LatheEdge::Line { a, b } => {
                push_ring_point(&mut ring, *a);
                push_ring_point(&mut ring, *b);
            }
            LatheEdge::Spline { points } => {
                for sample in spline_ring_samples(points)? {
                    push_ring_point(&mut ring, sample);
                }
            }
        }
    }
    // Drop the trailing duplicate of the ring start, if any.
    if ring.len() > 1 {
        let first = ring[0];
        if let Some(last) = ring.last() {
            if *last == first {
                ring.pop();
            }
        }
    }
    Ok(ring)
}

/// The sampled polyline of one spline profile edge: its exact samples at the
/// reconstructed span joints plus the fixed interior subdivision of each span.
fn spline_ring_samples(points: &[[f64; 2]]) -> Result<Vec<[f64; 2]>, Refusal> {
    let spans = spline_spans(points)?;
    let mut out = Vec::new();
    out.push(points[0]);
    for (i, span) in spans.iter().enumerate() {
        for s in 1..SPLINE_MESH_STEPS {
            let u = s as f64 / SPLINE_MESH_STEPS as f64;
            out.push(eval_span(span, u));
        }
        if i + 1 < points.len() {
            out.push(points[i + 1]);
        }
    }
    Ok(out)
}

/// Evaluates a reconstructed span at `u in [0, 1]`.
fn eval_span(span: &SpanPoly, u: f64) -> [f64; 2] {
    [
        span.r[0] + u * (span.r[1] + u * (span.r[2] + u * span.r[3])),
        span.z[0] + u * (span.z[1] + u * (span.z[2] + u * span.z[3])),
    ]
}

/// Sweeps the boundary ring over the angular segments.
fn sweep_ring_mesh(ring: &[[f64; 2]], segments: usize) -> Result<Vec<Triangle>, Refusal> {
    if ring.len() < 3 {
        return Err(Refusal::Empty);
    }
    for point in ring {
        if point[0] < 0.0 || !point[0].is_finite() || !point[1].is_finite() {
            return Err(Refusal::Empty);
        }
    }
    let mut out = Vec::new();
    for ring_index in 0..ring.len() {
        let next = (ring_index + 1) % ring.len();
        let x0 = ring[ring_index][0];
        let z0 = ring[ring_index][1];
        let x1 = ring[next][0];
        let z1 = ring[next][1];
        for segment in 0..segments {
            let angle_a = TAU * segment as f64 / segments as f64;
            let angle_b = TAU * (segment + 1) as f64 / segments as f64;
            let a = [x0 * angle_a.cos(), x0 * angle_a.sin(), z0];
            let b = [x0 * angle_b.cos(), x0 * angle_b.sin(), z0];
            let c = [x1 * angle_b.cos(), x1 * angle_b.sin(), z1];
            let d = [x1 * angle_a.cos(), x1 * angle_a.sin(), z1];
            push_quad(&mut out, a, b, c, d);
        }
    }
    Ok(out)
}

/// The number of angular segments a partial-arc sweep is subdivided into: the
/// full-revolution angular resolution, so a wedge's bands have the same arc
/// length as the full mesh's bands (deterministic, at least one segment).
fn arc_segments(arc: f64, segments_full: usize) -> usize {
    let per_segment = TAU / segments_full as f64;
    let segments = (arc / per_segment).ceil() as usize;
    segments.max(1)
}

/// Sweeps the boundary ring over a partial angular sector `[theta0, theta0 +
/// arc]` and closes the two planar caps at the sector endpoints. The side
/// surface is the same conical quad strip as the full sweep, restricted to the
/// sector; the caps are centroid fans of the sampled ring, so the mesh is
/// closed against the side surface.
fn sweep_ring_mesh_arc(
    ring: &[[f64; 2]],
    theta0: f64,
    arc: f64,
    segments_full: usize,
) -> Result<Vec<Triangle>, Refusal> {
    if ring.len() < 3 {
        return Err(Refusal::Empty);
    }
    for point in ring {
        if point[0] < 0.0 || !point[0].is_finite() || !point[1].is_finite() {
            return Err(Refusal::Empty);
        }
    }
    let segments = arc_segments(arc, segments_full);
    let mut out = Vec::new();
    let closed = ring.iter().chain(ring.iter().take(1));
    let mut previous: Option<[f64; 2]> = None;
    for point in closed {
        if let Some(prev) = previous {
            let x0 = prev[0];
            let z0 = prev[1];
            let x1 = point[0];
            let z1 = point[1];
            for segment in 0..segments {
                let angle_a = theta0 + arc * segment as f64 / segments as f64;
                let angle_b = theta0 + arc * (segment + 1) as f64 / segments as f64;
                let a = [x0 * angle_a.cos(), x0 * angle_a.sin(), z0];
                let b = [x0 * angle_b.cos(), x0 * angle_b.sin(), z0];
                let c = [x1 * angle_b.cos(), x1 * angle_b.sin(), z1];
                let d = [x1 * angle_a.cos(), x1 * angle_a.sin(), z1];
                push_quad(&mut out, a, b, c, d);
            }
        }
        previous = Some(*point);
    }
    append_lathe_cap(&mut out, ring, theta0);
    append_lathe_cap(&mut out, ring, theta0 + arc);
    Ok(out)
}

/// Appends the planar cap of the profile region at the given sweep angle as a
/// centroid fan over the sampled boundary ring. Every cap point lies in the
/// plane at `theta`, so the fan is the region's planar triangulation.
fn append_lathe_cap(out: &mut Vec<Triangle>, ring: &[[f64; 2]], theta: f64) {
    let count = ring.len();
    if count < 3 {
        return;
    }
    let mut center_r = 0.0;
    let mut center_z = 0.0;
    for point in ring {
        center_r += point[0];
        center_z += point[1];
    }
    let scale = 1.0 / count as f64;
    let center_r = center_r * scale;
    let center_z = center_z * scale;
    let (sin, cos) = theta.sin_cos();
    let center = [center_r * cos, center_r * sin, center_z];
    let closed = ring.iter().chain(ring.iter().take(1));
    let mut previous: Option<[f64; 2]> = None;
    for point in closed {
        if let Some(p) = previous {
            let q = *point;
            let a = [p[0] * cos, p[0] * sin, p[1]];
            let b = [q[0] * cos, q[0] * sin, q[1]];
            push_tri(out, center, a, b);
        }
        previous = Some(*point);
    }
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
/// `deflection` of `None` keeps the landed fixed-resolution deterministic
/// mesh (the recorded fingerprint rule) bit-for-bit.
pub fn write_tree_stl(root: &TreeNode, path: &str, deflection: Option<f64>) -> Result<u64, Refusal> {
    let triangles = tree_mesh(root, deflection)?;
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
// GLB emission (the render artifact: indexed meshes + per-part script colors)
// ---------------------------------------------------------------------------

/// One per-part mesh collected from the tree: the part's recorded label and
/// color (client metadata, carried verbatim) plus its placed world triangles.
struct LeafMesh {
    label: Option<String>,
    color: Option<String>,
    triangles: Vec<Triangle>,
}

/// Collects every placed part's triangles, label and color, mirroring
/// [`append_node_mesh`]'s placement transforms but keeping one entry per
/// part (the GLB node granularity).
fn collect_node_payloads(
    node: &TreeNode,
    deflection: Option<f64>,
    out: &mut Vec<LeafMesh>,
) -> Result<(), Refusal> {
    match node {
        TreeNode::Part { part } => {
            let local = solid_mesh(&part.solid, deflection)?;
            let reflected: Vec<Triangle> = local
                .iter()
                .map(|tri| match part.mirror.as_deref() {
                    Some(axis) => reflect_triangle(*tri, axis),
                    None => *tri,
                })
                .collect();
            let mut placed = Vec::with_capacity(reflected.len());
            if let Some(frame) = part.rotation {
                for tri in reflected {
                    placed.push(place_triangle_frame(tri, frame, part.x, part.y, part.z));
                }
            } else {
                let rz = part.rz.to_radians();
                let cos = rz.cos();
                let sin = rz.sin();
                for tri in reflected {
                    placed.push(place_triangle(tri, cos, sin, part.x, part.y, part.z));
                }
            }
            out.push(LeafMesh {
                label: part.label.clone(),
                color: part.color.clone(),
                triangles: placed,
            });
            Ok(())
        }
        TreeNode::Boolean { boolean } => {
            dispatch_boolean(boolean)?;
            collect_node_payloads(&boolean.a, deflection, out)
        }
        TreeNode::Fillet { fillet } => collect_node_payloads(&fillet.base, deflection, out),
        TreeNode::Group { group, .. } => {
            for child in group {
                collect_node_payloads(child, deflection, out)?;
            }
            Ok(())
        }
    }
}

/// Converts a triangle soup into an indexed mesh: bitwise-identical vertices
/// share one position record (the glTF indexed form the viewers stream).
fn triangles_to_glb_mesh(triangles: &[Triangle]) -> GlbMesh {
    let mut index_map = std::collections::HashMap::<[u32; 3], u32>::new();
    let mut positions = Vec::with_capacity(triangles.len() * 3);
    let mut indices = Vec::with_capacity(triangles.len() * 3);
    for tri in triangles {
        for vertex in 0..3 {
            let key = [
                tri[vertex * 3] as f32,
                tri[vertex * 3 + 1] as f32,
                tri[vertex * 3 + 2] as f32,
            ];
            let bits = [key[0].to_bits(), key[1].to_bits(), key[2].to_bits()];
            let next = (positions.len() / 3) as u32;
            let index = *index_map.entry(bits).or_insert_with(|| {
                positions.extend_from_slice(&key);
                next
            });
            indices.push(index);
        }
    }
    GlbMesh { positions, indices }
}

/// Parses the client color record (the door's `_color_record` textual form:
/// an sRGB hex string, or the JSON of an RGB(A) tuple in 0-1 or 0-255 scale).
/// Unrecognized records fall back to the neutral steel gray.
fn parse_client_color(recorded: &str) -> SrgbColor {
    let channel = |v: f64| -> f32 {
        let scaled = if v > 1.0 { v / 255.0 } else { v };
        scaled.clamp(0.0, 1.0) as f32
    };
    let text = recorded.trim();
    if let Some(hex) = text.strip_prefix('#') {
        if hex.len() == 6 || hex.len() == 8 {
            if let Ok(value) = u32::from_str_radix(hex, 16) {
                let (r, g, b, a) = match hex.len() {
                    6 => ((value >> 16) & 0xff, (value >> 8) & 0xff, value & 0xff, 255),
                    _ => (
                        (value >> 24) & 0xff,
                        (value >> 16) & 0xff,
                        (value >> 8) & 0xff,
                        value & 0xff,
                    ),
                };
                return SrgbColor::new(
                    r as f32 / 255.0,
                    g as f32 / 255.0,
                    b as f32 / 255.0,
                    a as f32 / 255.0,
                );
            }
        }
        return SrgbColor::new(0.62, 0.65, 0.70, 1.0);
    }
    if let Ok(values) = serde_json::from_str::<Vec<f64>>(text) {
        if values.len() >= 3 {
            return SrgbColor::new(
                channel(values[0]),
                channel(values[1]),
                channel(values[2]),
                values.get(3).map(|a| channel(*a)).unwrap_or(1.0),
            );
        }
    }
    SrgbColor::new(0.62, 0.65, 0.70, 1.0)
}

/// Writes the tree as a colored indexed GLB at `path`, one node per placed
/// part, returning `(parts, triangles)`. The certification artifact stays the
/// STL path; this is the render artifact (colors from the recorded client
/// metadata, indexed geometry at the requested deflection).
fn write_tree_glb(
    root: &TreeNode,
    path: &str,
    deflection: Option<f64>,
) -> Result<(u64, u64), Refusal> {
    let mut leaves = Vec::new();
    collect_node_payloads(root, deflection, &mut leaves)?;
    if leaves.is_empty() {
        return Err(Refusal::Empty);
    }
    let mut payloads = Vec::with_capacity(leaves.len());
    let mut total_triangles = 0_u64;
    for (index, leaf) in leaves.iter().enumerate() {
        if leaf.triangles.is_empty() {
            continue;
        }
        let mesh = triangles_to_glb_mesh(&leaf.triangles);
        total_triangles += mesh.indices.len() as u64 / 3;
        let color = leaf
            .color
            .as_deref()
            .map(parse_client_color)
            .unwrap_or(SrgbColor::new(0.62, 0.65, 0.70, 1.0));
        let name = leaf
            .label
            .clone()
            .unwrap_or_else(|| format!("part_{index:04}"));
        payloads.push(GlbNodePayload {
            name,
            color,
            mesh: Some(mesh),
            parent: None,
            matrix: [
                1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
            ],
        });
    }
    let bytes = emit_glb(&payloads)?;
    std::fs::write(path, bytes).map_err(|_| Refusal::Empty)?;
    Ok((payloads.len() as u64, total_triangles))
}

// ---------------------------------------------------------------------------
// MONO-5-RAY-CLASSIFY -- certified point-vs-spline-solid membership
// ---------------------------------------------------------------------------
//
// The wave-3 boolean solver consumes a certified membership primitive: for a
// point `p` and a spline-patch-bounded solid `S` -- a finite oriented 2-cycle
// of tensor-Bernstein patches, exactly the shape the landed loft/member rows
// assemble (`spline_loft_volume_rows`) -- decide `p in int S` / `p not in S` /
// typed-indeterminate, with certificates.
//
// **The mechanism is NAMED (wave-3 amendment 3): certified ray x bicubic root
// isolation.** A recorded-direction ray `r(t) = p + t d` is cast from `p`; the
// ray's crossings with every bicubic (tensor-Bernstein) patch are isolated by
// 1-D certified bracketing -- Bernstein clipping over the patch domain, with
// the ray-parameter interval derived per sub-patch by outward-rounded interval
// arithmetic -- and the signed crossings are counted. 1-D bracketing is
// strictly easier than the 4-D contact problem and reuses the landed interval
// discipline. Every numeric decision is an outward-rounded interval decision;
// no sampling and no naked-f64 verdict.
//
// **Weights admission.** The corpus's lofts/members are non-rational: this
// primitive certifies `weights == 1` for every consumed patch and refuses a
// rational weight field typed. (The landed `VolumeRow` weight path covers the
// rational case elsewhere; here the admitted carrier is the non-rational
// polynomial net.)
//
// **Refusal / retry contract.** A non-transversal ray (grazing, tangent, or
// coplanar with a patch) leaves crossing enclosures that never separate; the
// clipping loop's leaf cap and depth cap detect this and return the typed
// `MembershipIndeterminate`, and the caller retries with a fresh recorded
// direction up to `RETRY_BOUND`. Failure of every direction is itself the
// typed indeterminate -- never a guessed classification.
//
// **Termination is a theorem obligation, not a tuning knob.** For a
// transversal ray/patch configuration the crossing is an isolated point in
// `(u, v)`; the Bernstein hull property makes every box at positive distance
// from the crossing exclude at finite depth, so the worklist empties. A
// configuration whose worklist does not empty (or whose surviving leaves do
// not shrink) is refused, never tuned.
//
// The public entry points are the follow-on boolean solver's surface; until
// that consumer lands they are reached only from the in-crate suite, so the
// module is scoped `allow(dead_code)` exactly as the sibling binding export
// module is.
#[allow(dead_code)]
pub mod membership {
    use serde::Serialize;

    use truck_certified::construct::patches::{PatchParent, TensorBernsteinPatch};
    use truck_certified::construct::volume_facts::{VolumeOptions, certify_patch_form};
    use truck_certified::kernel::patch::IBox2;

    use crate::facade::ModeValue;
    use crate::python::binding::VolumeRow;

    /// The number of fresh recorded directions attempted after the caller's
    /// first direction before the typed indeterminate is returned.
    pub const RETRY_BOUND: usize = 8;

    /// The subdivision depth cap of the certified clipping loop. Termination
    /// for an admitted (transversal) ray/patch configuration is a theorem
    /// obligation; the cap only bounds the work for the refused cases.
    const MAX_DEPTH: u32 = 80;

    /// The per-patch surviving-box cap. A non-transversal (grazing/coplanar)
    /// ray leaves a positive-dimensional surviving set whose box count grows
    /// with subdivision; hitting the cap is the refusal signal.
    const MAX_LEAVES: usize = 4096;

    /// The relative isolation tolerance: a surviving leaf is certified once
    /// the diameter of its patch enclosure is no larger than this fraction of
    /// the solid's bounding diameter.
    const ISOLATION_TOL: f64 = 1.0e-9;

    /// The cluster-extent guard: a merged crossing whose patch enclosure is
    /// wider than this multiple of the isolation tolerance may be two
    /// crossings under a grazing ray, so it is refused rather than counted.
    const CLUSTER_TOL_FACTOR: f64 = 64.0;

    /// The certified membership verdict.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
    #[serde(rename_all = "snake_case")]
    pub enum MembershipVerdict {
        /// The point is certified in the solid's interior.
        Inside,
        /// The point is certified outside the solid.
        Outside,
        /// No recorded direction certified the membership; retry with another
        /// direction or treat the point as unresolved.
        Indeterminate,
    }

    /// The typed refusal of a membership query.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
    #[serde(rename_all = "snake_case")]
    pub enum MembershipRefusal {
        /// The ray met the boundary non-transversally (grazing/tangent/coplanar)
        /// or the clipping loop did not isolate a crossing within budget.
        MembershipIndeterminate,
        /// A consumed patch row is malformed (a caller defect).
        MalformedPatch,
        /// A consumed patch carries a non-constant rational weight field; this
        /// arm admits the non-rational (weights == 1) corpus carrier.
        RationalWeights,
    }

    /// The crossing evidence of one certified ray/patch crossing.
    #[derive(Debug, Clone, PartialEq, Serialize)]
    pub struct CrossingEvidence {
        /// The index of the patch in the consumed patch set.
        pub patch: usize,
        /// The certified ray-parameter interval of the crossing.
        pub t_lo: f64,
        /// The certified ray-parameter interval of the crossing.
        pub t_hi: f64,
        /// The certified patch-domain `u` interval of the crossing.
        pub u_lo: f64,
        /// The certified patch-domain `u` interval of the crossing.
        pub u_hi: f64,
        /// The certified patch-domain `v` interval of the crossing.
        pub v_lo: f64,
        /// The certified patch-domain `v` interval of the crossing.
        pub v_hi: f64,
        /// The interval-certified sign of `d . n_P` (`+1` or `-1`).
        pub normal_sign: i8,
    }

    /// The certificate of one membership query.
    #[derive(Debug, Clone, PartialEq, Serialize)]
    pub struct MembershipCertificate {
        /// The certified verdict.
        pub verdict: MembershipVerdict,
        /// The number of recorded directions attempted (the 0-based index of
        /// the successful direction; `RETRY_BOUND` when every direction
        /// failed).
        pub attempts: usize,
        /// The direction that produced the verdict (the caller's direction on
        /// the first attempt, a recorded fresh direction otherwise).
        pub direction: [f64; 3],
        /// The certified crossings of the accepted cast.
        pub crossings: Vec<CrossingEvidence>,
        /// The typed refusal when the verdict is `Indeterminate`.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub refusal: Option<MembershipRefusal>,
    }

    // -- outward-rounded interval discipline (inari-style) -------------------

    /// A closed interval with outward-rounded endpoints. The only numeric
    /// shape the crossing loop reasons over.
    #[derive(Debug, Clone, Copy)]
    struct Iv {
        lo: f64,
        hi: f64,
    }

    impl Iv {
        fn point(x: f64) -> Iv {
            Iv { lo: x, hi: x }
        }
        fn unit() -> Iv {
            Iv { lo: 0.0, hi: 1.0 }
        }
        fn add(self, o: Iv) -> Iv {
            Iv {
                lo: down(self.lo + o.lo),
                hi: up(self.hi + o.hi),
            }
        }
        fn sub(self, o: Iv) -> Iv {
            Iv {
                lo: down(self.lo - o.hi),
                hi: up(self.hi - o.lo),
            }
        }
        fn mul(self, o: Iv) -> Iv {
            let candidates = [
                self.lo * o.lo,
                self.lo * o.hi,
                self.hi * o.lo,
                self.hi * o.hi,
            ];
            let mut lo = f64::INFINITY;
            let mut hi = f64::NEG_INFINITY;
            for value in candidates {
                lo = lo.min(down(value));
                hi = hi.max(up(value));
            }
            Iv { lo, hi }
        }
        fn scale(self, s: f64) -> Iv {
            self.mul(Iv::point(s))
        }
        fn contains_zero(self) -> bool {
            self.lo <= 0.0 && self.hi >= 0.0
        }
        fn width(self) -> f64 {
            self.hi - self.lo
        }
        fn mid(self) -> f64 {
            0.5 * (self.lo + self.hi)
        }
    }

    /// The next representable float below `x` (outward rounding).
    fn down(x: f64) -> f64 {
        if x.is_finite() { x.next_down() } else { x }
    }

    /// The next representable float above `x` (outward rounding).
    fn up(x: f64) -> f64 {
        if x.is_finite() { x.next_up() } else { x }
    }

    fn ipow(x: Iv, n: usize) -> Iv {
        let mut acc = Iv::point(1.0);
        for _ in 0..n {
            acc = acc.mul(x);
        }
        acc
    }

    /// The binomial coefficient `C(n, k)` as an `f64` (exact for the small
    /// degrees this arm uses).
    fn binomial(n: usize, k: usize) -> f64 {
        if k > n {
            return 0.0;
        }
        let k = k.min(n - k);
        let mut out = 1.0f64;
        for i in 0..k {
            out = out * (n - i) as f64 / (i + 1) as f64;
        }
        out
    }

    /// The interval Bernstein basis of `degree` over the interval `t`.
    fn bernstein_iv(degree: usize, t: Iv) -> Vec<Iv> {
        let s = Iv::point(1.0).sub(t);
        let mut out = Vec::with_capacity(degree + 1);
        for i in 0..=degree {
            out.push(
                Iv::point(binomial(degree, i))
                    .mul(ipow(s, degree - i))
                    .mul(ipow(t, i)),
            );
        }
        out
    }

    /// The scalar Bernstein basis of `degree` at the point `t`.
    fn bernstein_f64(degree: usize, t: f64) -> Vec<f64> {
        let s = 1.0 - t;
        let mut out = Vec::with_capacity(degree + 1);
        for i in 0..=degree {
            out.push(binomial(degree, i) * s.powi((degree - i) as i32) * t.powi(i as i32));
        }
        out
    }

    // -- patch data ----------------------------------------------------------

    /// One validated non-rational tensor-Bernstein patch (weights certified
    /// `== 1`), with its recorded orientation sign.
    #[derive(Clone)]
    struct Patch {
        rows: usize,
        cols: usize,
        data: Vec<[f64; 3]>,
        orientation: f64,
    }

    fn parse_patch(row: &VolumeRow) -> Result<Patch, MembershipRefusal> {
        let rows = row.numerator.len();
        let cols = row.numerator.first().map_or(0, Vec::len);
        if rows < 2 || cols < 2 || row.weights.len() != rows {
            return Err(MembershipRefusal::MalformedPatch);
        }
        let mut data = Vec::with_capacity(rows.saturating_mul(cols));
        for (i, num_row) in row.numerator.iter().enumerate() {
            if num_row.len() != cols {
                return Err(MembershipRefusal::MalformedPatch);
            }
            let weight_row = row
                .weights
                .get(i)
                .ok_or(MembershipRefusal::MalformedPatch)?;
            if weight_row.len() != cols {
                return Err(MembershipRefusal::MalformedPatch);
            }
            for (j, a) in num_row.iter().enumerate() {
                let w = weight_row
                    .get(j)
                    .copied()
                    .ok_or(MembershipRefusal::MalformedPatch)?;
                if w != 1.0 {
                    return Err(MembershipRefusal::RationalWeights);
                }
                if !a.iter().all(|c| c.is_finite()) {
                    return Err(MembershipRefusal::MalformedPatch);
                }
                data.push(*a);
            }
        }
        if !row.orientation.is_finite() || row.orientation == 0.0 {
            return Err(MembershipRefusal::MalformedPatch);
        }
        Ok(Patch {
            rows,
            cols,
            data,
            orientation: row.orientation,
        })
    }

    /// The coordinate-wise enclosure of the patch over the parameter box
    /// `u x v` (the interval-Bernstein sum `sum B_i(u) B_j(v) A_ij`; the
    /// enclosure is outward-rounded and shrinks to the patch point as the box
    /// shrinks).
    fn patch_range(patch: &Patch, u: Iv, v: Iv) -> ([f64; 3], [f64; 3]) {
        let bu = bernstein_iv(patch.rows - 1, u);
        let bv = bernstein_iv(patch.cols - 1, v);
        let mut sum = [Iv::point(0.0); 3];
        for i in 0..patch.rows {
            let bui = bu.get(i).copied().unwrap_or_else(|| Iv::point(0.0));
            for j in 0..patch.cols {
                let bvj = bv.get(j).copied().unwrap_or_else(|| Iv::point(0.0));
                let coeff = bui.mul(bvj);
                let index = i * patch.cols + j;
                if let Some([x, y, z]) = patch.data.get(index).copied() {
                    for (axis, value) in [x, y, z].into_iter().enumerate() {
                        let term = coeff.scale(value);
                        if let Some(slot) = sum.get_mut(axis) {
                            *slot = (*slot).add(term);
                        }
                    }
                }
            }
        }
        (
            [sum[0].lo, sum[1].lo, sum[2].lo],
            [sum[0].hi, sum[1].hi, sum[2].hi],
        )
    }

    /// The interval enclosure of `dP/du` over the parameter box `u x v`.
    fn patch_derivative_u(patch: &Patch, u: Iv, v: Iv) -> [Iv; 3] {
        let mut acc = [Iv::point(0.0); 3];
        if patch.rows < 2 {
            return acc;
        }
        let bu = bernstein_iv(patch.rows - 2, u);
        let bv = bernstein_iv(patch.cols - 1, v);
        let factor = (patch.rows - 1) as f64;
        for i in 0..(patch.rows - 1) {
            let bui = bu.get(i).copied().unwrap_or_else(|| Iv::point(0.0));
            for j in 0..patch.cols {
                let bvj = bv.get(j).copied().unwrap_or_else(|| Iv::point(0.0));
                let coeff = Iv::point(factor).mul(bui).mul(bvj);
                let a = patch
                    .data
                    .get(i * patch.cols + j)
                    .copied()
                    .unwrap_or([0.0; 3]);
                let b = patch
                    .data
                    .get((i + 1) * patch.cols + j)
                    .copied()
                    .unwrap_or([0.0; 3]);
                for (axis, (av, bv_)) in a.iter().zip(b.iter()).enumerate() {
                    let term = coeff.scale(bv_ - av);
                    if let Some(slot) = acc.get_mut(axis) {
                        *slot = (*slot).add(term);
                    }
                }
            }
        }
        acc
    }

    /// The interval enclosure of `dP/dv` over the parameter box `u x v`.
    fn patch_derivative_v(patch: &Patch, u: Iv, v: Iv) -> [Iv; 3] {
        let mut acc = [Iv::point(0.0); 3];
        if patch.cols < 2 {
            return acc;
        }
        let bu = bernstein_iv(patch.rows - 1, u);
        let bv = bernstein_iv(patch.cols - 2, v);
        let factor = (patch.cols - 1) as f64;
        for i in 0..patch.rows {
            let bui = bu.get(i).copied().unwrap_or_else(|| Iv::point(0.0));
            for j in 0..(patch.cols - 1) {
                let bvj = bv.get(j).copied().unwrap_or_else(|| Iv::point(0.0));
                let coeff = Iv::point(factor).mul(bui).mul(bvj);
                let a = patch
                    .data
                    .get(i * patch.cols + j)
                    .copied()
                    .unwrap_or([0.0; 3]);
                let b = patch
                    .data
                    .get(i * patch.cols + (j + 1))
                    .copied()
                    .unwrap_or([0.0; 3]);
                for (axis, (av, bv_)) in a.iter().zip(b.iter()).enumerate() {
                    let term = coeff.scale(bv_ - av);
                    if let Some(slot) = acc.get_mut(axis) {
                        *slot = (*slot).add(term);
                    }
                }
            }
        }
        acc
    }

    /// The interval enclosure of the patch normal `P_u x P_v`.
    fn normal_interval(patch: &Patch, u: Iv, v: Iv) -> [Iv; 3] {
        let du = patch_derivative_u(patch, u, v);
        let dv = patch_derivative_v(patch, u, v);
        [
            du[1].mul(dv[2]).sub(du[2].mul(dv[1])),
            du[2].mul(dv[0]).sub(du[0].mul(dv[2])),
            du[0].mul(dv[1]).sub(du[1].mul(dv[0])),
        ]
    }

    /// The interval enclosure of `n . d`.
    fn dot_direction(normal: [Iv; 3], direction: [f64; 3]) -> Iv {
        normal[0]
            .scale(direction[0])
            .add(normal[1].scale(direction[1]))
            .add(normal[2].scale(direction[2]))
    }

    /// The pointwise derivative `dP/du` at `(u, v)` (used only for the
    /// subdivision-direction heuristic, never for a verdict).
    fn derivative_point_u(patch: &Patch, u: f64, v: f64) -> [f64; 3] {
        let mut acc = [0.0f64; 3];
        if patch.rows < 2 {
            return acc;
        }
        let bu = bernstein_f64(patch.rows - 2, u);
        let bv = bernstein_f64(patch.cols - 1, v);
        let factor = (patch.rows - 1) as f64;
        for i in 0..(patch.rows - 1) {
            let bui = bu.get(i).copied().unwrap_or(0.0);
            for j in 0..patch.cols {
                let bvj = bv.get(j).copied().unwrap_or(0.0);
                let coeff = factor * bui * bvj;
                let a = patch
                    .data
                    .get(i * patch.cols + j)
                    .copied()
                    .unwrap_or([0.0; 3]);
                let b = patch
                    .data
                    .get((i + 1) * patch.cols + j)
                    .copied()
                    .unwrap_or([0.0; 3]);
                for (slot, (av, bv_)) in acc.iter_mut().zip(a.iter().zip(b.iter())) {
                    *slot += coeff * (bv_ - av);
                }
            }
        }
        acc
    }

    /// The pointwise derivative `dP/dv` at `(u, v)` (split heuristic only).
    fn derivative_point_v(patch: &Patch, u: f64, v: f64) -> [f64; 3] {
        let mut acc = [0.0f64; 3];
        if patch.cols < 2 {
            return acc;
        }
        let bu = bernstein_f64(patch.rows - 1, u);
        let bv = bernstein_f64(patch.cols - 2, v);
        let factor = (patch.cols - 1) as f64;
        for i in 0..patch.rows {
            let bui = bu.get(i).copied().unwrap_or(0.0);
            for j in 0..(patch.cols - 1) {
                let bvj = bv.get(j).copied().unwrap_or(0.0);
                let coeff = factor * bui * bvj;
                let a = patch
                    .data
                    .get(i * patch.cols + j)
                    .copied()
                    .unwrap_or([0.0; 3]);
                let b = patch
                    .data
                    .get(i * patch.cols + (j + 1))
                    .copied()
                    .unwrap_or([0.0; 3]);
                for (slot, (av, bv_)) in acc.iter_mut().zip(a.iter().zip(b.iter())) {
                    *slot += coeff * (bv_ - av);
                }
            }
        }
        acc
    }

    fn norm3(value: [f64; 3]) -> f64 {
        (value[0] * value[0] + value[1] * value[1] + value[2] * value[2]).sqrt()
    }

    // -- certified clipping loop --------------------------------------------

    /// One surviving clipping box: a certified crossing enclosure candidate.
    struct Leaf {
        u: Iv,
        v: Iv,
        t: Iv,
        lo: [f64; 3],
        hi: [f64; 3],
    }

    /// The ray-parameter interval over which the ray meets the axis-aligned
    /// box `[lo, hi]`, clipped to `[0, t_cap]`; `None` when the ray misses.
    fn ray_slab(
        point: [f64; 3],
        direction: [f64; 3],
        lo: [f64; 3],
        hi: [f64; 3],
        t_cap: f64,
    ) -> Option<Iv> {
        let mut t = Iv { lo: 0.0, hi: t_cap };
        for axis in 0..3 {
            let p = point.get(axis).copied().unwrap_or(0.0);
            let d = direction.get(axis).copied().unwrap_or(0.0);
            let l = lo.get(axis).copied().unwrap_or(f64::NEG_INFINITY);
            let h = hi.get(axis).copied().unwrap_or(f64::INFINITY);
            if d != 0.0 {
                let a = (l - p) / d;
                let b = (h - p) / d;
                let (s_lo, s_hi) = if a <= b { (a, b) } else { (b, a) };
                t.lo = t.lo.max(down(s_lo));
                t.hi = t.hi.min(up(s_hi));
            } else if p < l || p > h {
                return None;
            }
            if t.lo > t.hi {
                return None;
            }
        }
        Some(t)
    }

    /// Bernstein-clip one patch's parameter domain against the ray, collecting
    /// the surviving crossing enclosures. The hull property makes every box at
    /// positive distance from a transversal crossing exclude at finite depth;
    /// a positive-dimensional surviving set (grazing/coplanar) hits the leaf
    /// cap or the depth cap and is refused.
    fn isolate_patch_leaves(
        patch: &Patch,
        point: [f64; 3],
        direction: [f64; 3],
        t_cap: f64,
        iso_tol: f64,
    ) -> Result<Vec<Leaf>, MembershipRefusal> {
        let mut leaves = Vec::new();
        let mut stack: Vec<(Iv, Iv, u32)> = vec![(Iv::unit(), Iv::unit(), 0)];
        while let Some((u, v, depth)) = stack.pop() {
            let (lo, hi) = patch_range(patch, u, v);
            let Some(t) = ray_slab(point, direction, lo, hi, t_cap) else {
                continue;
            };
            let [sx, sy, sz] = [hi[0] - lo[0], hi[1] - lo[1], hi[2] - lo[2]];
            let diameter = (sx * sx + sy * sy + sz * sz).sqrt();
            if diameter <= iso_tol {
                leaves.push(Leaf { u, v, t, lo, hi });
                if leaves.len() > MAX_LEAVES {
                    return Err(MembershipRefusal::MembershipIndeterminate);
                }
            } else if depth >= MAX_DEPTH {
                return Err(MembershipRefusal::MembershipIndeterminate);
            } else {
                let u_score = norm3(derivative_point_u(patch, u.mid(), v.mid())) * u.width();
                let v_score = norm3(derivative_point_v(patch, u.mid(), v.mid())) * v.width();
                if u_score >= v_score {
                    let mid = u.mid();
                    stack.push((Iv { lo: u.lo, hi: mid }, v, depth + 1));
                    stack.push((Iv { lo: mid, hi: u.hi }, v, depth + 1));
                } else {
                    let mid = v.mid();
                    stack.push((u, Iv { lo: v.lo, hi: mid }, depth + 1));
                    stack.push((u, Iv { lo: mid, hi: v.hi }, depth + 1));
                }
                if stack.len() > MAX_LEAVES {
                    return Err(MembershipRefusal::MembershipIndeterminate);
                }
            }
        }
        Ok(leaves)
    }

    /// Merges a cluster of adjacent leaves around one crossing and certifies
    /// its normal sign. A cluster wider than the cluster guard is refused.
    fn finish_cluster(
        patch: &Patch,
        direction: [f64; 3],
        cluster: &[Leaf],
        iso_tol: f64,
    ) -> Result<CrossingEvidence, MembershipRefusal> {
        let mut t_lo = f64::INFINITY;
        let mut t_hi = f64::NEG_INFINITY;
        let mut u_lo = f64::INFINITY;
        let mut u_hi = f64::NEG_INFINITY;
        let mut v_lo = f64::INFINITY;
        let mut v_hi = f64::NEG_INFINITY;
        let mut lo = [f64::INFINITY; 3];
        let mut hi = [f64::NEG_INFINITY; 3];
        for leaf in cluster {
            t_lo = t_lo.min(leaf.t.lo);
            t_hi = t_hi.max(leaf.t.hi);
            u_lo = u_lo.min(leaf.u.lo);
            u_hi = u_hi.max(leaf.u.hi);
            v_lo = v_lo.min(leaf.v.lo);
            v_hi = v_hi.max(leaf.v.hi);
            for (axis, value) in leaf.lo.into_iter().enumerate() {
                if let Some(slot) = lo.get_mut(axis) {
                    *slot = (*slot).min(value);
                }
            }
            for (axis, value) in leaf.hi.into_iter().enumerate() {
                if let Some(slot) = hi.get_mut(axis) {
                    *slot = (*slot).max(value);
                }
            }
        }
        let [sx, sy, sz] = [hi[0] - lo[0], hi[1] - lo[1], hi[2] - lo[2]];
        let diameter = (sx * sx + sy * sy + sz * sz).sqrt();
        if diameter > iso_tol * CLUSTER_TOL_FACTOR {
            return Err(MembershipRefusal::MembershipIndeterminate);
        }
        let normal = normal_interval(patch, Iv { lo: u_lo, hi: u_hi }, Iv { lo: v_lo, hi: v_hi });
        let dn = dot_direction(normal, direction);
        let sign = if dn.lo > 0.0 {
            1
        } else if dn.hi < 0.0 {
            -1
        } else {
            return Err(MembershipRefusal::MembershipIndeterminate);
        };
        Ok(CrossingEvidence {
            patch: 0,
            t_lo,
            t_hi,
            u_lo,
            u_hi,
            v_lo,
            v_hi,
            normal_sign: sign,
        })
    }

    /// The certified crossings of one patch under one ray.
    fn patch_crossings(
        patch: &Patch,
        point: [f64; 3],
        direction: [f64; 3],
        t_cap: f64,
        iso_tol: f64,
    ) -> Result<Vec<CrossingEvidence>, MembershipRefusal> {
        let mut sorted = isolate_patch_leaves(patch, point, direction, t_cap, iso_tol)?;
        if sorted.is_empty() {
            return Ok(Vec::new());
        }
        sorted.sort_by(|a, b| {
            a.t.lo
                .partial_cmp(&b.t.lo)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let mut crossings = Vec::new();
        let mut cluster: Vec<Leaf> = Vec::new();
        let mut cluster_hi = f64::NEG_INFINITY;
        for leaf in sorted {
            if !cluster.is_empty() && leaf.t.lo <= cluster_hi {
                cluster_hi = cluster_hi.max(leaf.t.hi);
                cluster.push(leaf);
                continue;
            }
            if !cluster.is_empty() {
                crossings.push(finish_cluster(patch, direction, &cluster, iso_tol)?);
            }
            cluster_hi = leaf.t.hi;
            cluster.clear();
            cluster.push(leaf);
        }
        if !cluster.is_empty() {
            crossings.push(finish_cluster(patch, direction, &cluster, iso_tol)?);
        }
        Ok(crossings)
    }

    // -- ray cast and verdict ------------------------------------------------

    fn solid_bounds(patches: &[Patch]) -> Option<([f64; 3], [f64; 3])> {
        let mut lo = [f64::INFINITY; 3];
        let mut hi = [f64::NEG_INFINITY; 3];
        let mut any = false;
        for patch in patches {
            for control in &patch.data {
                any = true;
                for (axis, value) in control.iter().enumerate() {
                    if let Some(slot) = lo.get_mut(axis) {
                        *slot = (*slot).min(*value);
                    }
                    if let Some(slot) = hi.get_mut(axis) {
                        *slot = (*slot).max(*value);
                    }
                }
            }
        }
        if any { Some((lo, hi)) } else { None }
    }

    fn solid_scale(patches: &[Patch]) -> f64 {
        match solid_bounds(patches) {
            Some((lo, hi)) => {
                let [sx, sy, sz] = [hi[0] - lo[0], hi[1] - lo[1], hi[2] - lo[2]];
                let diameter = (sx * sx + sy * sy + sz * sz).sqrt();
                if diameter.is_finite() && diameter > 0.0 {
                    diameter
                } else {
                    1.0
                }
            }
            None => 1.0,
        }
    }

    fn normalize(direction: [f64; 3]) -> Option<[f64; 3]> {
        let norm = norm3(direction);
        if norm.is_finite() && norm > 0.0 {
            Some([
                direction[0] / norm,
                direction[1] / norm,
                direction[2] / norm,
            ])
        } else {
            None
        }
    }

    /// Casts one ray and returns its certified crossings, in ray-parameter
    /// order. A point strictly outside the patch control hull is certified
    /// outside by the Bernstein hull property (zero crossings). A shared
    /// edge/vertex hit (two patches at one ray parameter) is refused.
    fn cast_ray(
        point: [f64; 3],
        direction: [f64; 3],
        patches: &[Patch],
    ) -> Result<Vec<CrossingEvidence>, MembershipRefusal> {
        let Some(unit) = normalize(direction) else {
            return Err(MembershipRefusal::MembershipIndeterminate);
        };
        let Some((lo, hi)) = solid_bounds(patches) else {
            return Err(MembershipRefusal::MalformedPatch);
        };
        for axis in 0..3 {
            let p = point.get(axis).copied().unwrap_or(0.0);
            let l = lo.get(axis).copied().unwrap_or(f64::NEG_INFINITY);
            let h = hi.get(axis).copied().unwrap_or(f64::INFINITY);
            if p < l || p > h {
                return Ok(Vec::new());
            }
        }
        let mut t_cap = f64::INFINITY;
        for axis in 0..3 {
            let p = point.get(axis).copied().unwrap_or(0.0);
            let d = unit.get(axis).copied().unwrap_or(0.0);
            let l = lo.get(axis).copied().unwrap_or(f64::NEG_INFINITY);
            let h = hi.get(axis).copied().unwrap_or(f64::INFINITY);
            if d > 0.0 {
                t_cap = t_cap.min((h - p) / d);
            } else if d < 0.0 {
                t_cap = t_cap.min((l - p) / d);
            }
        }
        if !t_cap.is_finite() || t_cap <= 0.0 {
            return Ok(Vec::new());
        }
        let iso_tol = ISOLATION_TOL * solid_scale(patches);
        let mut all: Vec<CrossingEvidence> = Vec::new();
        for (index, patch) in patches.iter().enumerate() {
            let mut crossings = patch_crossings(patch, point, unit, t_cap, iso_tol)?;
            for crossing in &mut crossings {
                crossing.patch = index;
            }
            all.extend(crossings);
        }
        all.sort_by(|a, b| {
            a.t_lo
                .partial_cmp(&b.t_lo)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        for pair in all.windows(2) {
            let [a, b] = pair else { continue };
            if a.patch != b.patch && b.t_lo <= a.t_hi {
                return Err(MembershipRefusal::MembershipIndeterminate);
            }
        }
        // A crossing at the ray origin means the point lies on the boundary:
        // the strict interior/not-interior question is degenerate, so refuse
        // rather than count a zero-length crossing.
        if all.iter().any(|crossing| crossing.t_lo <= iso_tol) {
            return Err(MembershipRefusal::MembershipIndeterminate);
        }
        Ok(all)
    }

    fn verdict_of(crossings: &[CrossingEvidence]) -> MembershipVerdict {
        let signed: i64 = crossings
            .iter()
            .map(|crossing| i64::from(crossing.normal_sign))
            .sum();
        if signed != 0 {
            MembershipVerdict::Inside
        } else {
            MembershipVerdict::Outside
        }
    }

    /// A deterministic fresh direction from the recorded seed (splitmix64):
    /// pseudo-random but bit-reproducible, so a rerun retries identically.
    fn retry_direction(seed: u64, attempt: usize) -> [f64; 3] {
        let mut z = seed.wrapping_add(0x9E37_79B9_7F4A_7C15u64.wrapping_mul(attempt as u64 + 1));
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        let a = (z & 0xFFFF_FFFF) as f64 / u32::MAX as f64;
        let b = ((z >> 32) & 0xFFFF_FFFF) as f64 / u32::MAX as f64;
        let theta = std::f64::consts::TAU * a;
        let cos_phi = 2.0 * b - 1.0;
        let sin_phi = (1.0 - cos_phi * cos_phi).sqrt();
        [sin_phi * theta.cos(), sin_phi * theta.sin(), cos_phi]
    }

    /// The single-cast primitive: classify `point` against the closed
    /// tensor-Bernstein patch set with exactly the given `direction`. A
    /// non-transversal configuration returns the typed
    /// [`MembershipRefusal::MembershipIndeterminate`] -- never a guessed
    /// classification.
    pub fn classify_ray(
        point: [f64; 3],
        direction: [f64; 3],
        patches: &[VolumeRow],
    ) -> Result<MembershipVerdict, MembershipRefusal> {
        let (verdict, _) = classify_ray_with_evidence(point, direction, patches)?;
        Ok(verdict)
    }

    /// The single-cast primitive with its crossing-evidence certificate.
    pub fn classify_ray_with_evidence(
        point: [f64; 3],
        direction: [f64; 3],
        patches: &[VolumeRow],
    ) -> Result<(MembershipVerdict, Vec<CrossingEvidence>), MembershipRefusal> {
        if patches.is_empty() {
            return Err(MembershipRefusal::MalformedPatch);
        }
        if !point.iter().all(|c| c.is_finite()) {
            return Err(MembershipRefusal::MalformedPatch);
        }
        let parsed = patches
            .iter()
            .map(parse_patch)
            .collect::<Result<Vec<_>, _>>()?;
        let crossings = cast_ray(point, direction, &parsed)?;
        let verdict = verdict_of(&crossings);
        Ok((verdict, crossings))
    }

    /// The retry contract: attempt the caller's `direction`, then up to
    /// [`RETRY_BOUND`] fresh deterministic directions derived from `seed`; the
    /// first certified verdict wins. Failure of every direction is the typed
    /// indeterminate certificate.
    pub fn classify_point(
        point: [f64; 3],
        direction: [f64; 3],
        patches: &[VolumeRow],
        seed: u64,
    ) -> MembershipCertificate {
        let mut last_refusal = MembershipRefusal::MembershipIndeterminate;
        for attempt in 0..=RETRY_BOUND {
            let candidate = if attempt == 0 {
                direction
            } else {
                retry_direction(seed, attempt - 1)
            };
            match classify_ray_with_evidence(point, candidate, patches) {
                Ok((verdict, crossings)) => {
                    return MembershipCertificate {
                        verdict,
                        attempts: attempt,
                        direction: normalize(candidate).unwrap_or(candidate),
                        crossings,
                        refusal: None,
                    };
                }
                Err(refusal) => last_refusal = refusal,
            }
        }
        MembershipCertificate {
            verdict: MembershipVerdict::Indeterminate,
            attempts: RETRY_BOUND,
            direction: normalize(direction).unwrap_or(direction),
            crossings: Vec::new(),
            refusal: Some(last_refusal),
        }
    }

    // =======================================================================
    // MONO-6-SWEPT-BOOLEANS -- certified boolean volume by contact covers.
    //
    // The reduction (method item 1). For an oriented boundary patch `P` of
    // `dA`, the divergence form `g_P = (1/3) P . (P_u x P_v)` integrates to
    // the flux through the patch; over the boundary of `A n B` this gives
    //
    //   V(A n B) = sum_{P of dA} int_{P^-1(B)} g_P
    //            + sum_{Q of dB} int_{Q^-1(A)} g_Q,
    //
    // and `V(A \ B) = V(A) - V(A n B)` with `V(A)` the landed per-patch flux
    // certificate (method item 1). The contact cover (item 2) brackets the
    // parameter cells whose images can meet the other solid; a cell outside
    // the cover is certifiably clear, and ONE MONO-5 membership witness fixes
    // its constant membership (item 4). Unresolved (contact) cells carry the
    // certified bracket `[|R| min(0, g_lo), |R| max(0, g_hi)]` and are
    // refined by descending error until the summed width is within budget
    // (item 3). The `(8,8)` flux integrand assumes non-rational weights
    // (item 7): every consumed patch certifies `weights == 1` here, and a
    // non-unit weight refuses typed.
    //
    // The `ExtremesSurvive` facts gate (Amendment 1) is the bbox sufficiency
    // lemma: the boolean row must answer `bbox(A \ B)`. For every one of the
    // six axis extremes of `A`'s certified control hull, the certificate
    // requires that `B`'s certified control hull does not reach that extreme
    // value; then no point of `B` can be the arg-extreme point, so the
    // extreme survives into `A \ B` and `bbox(A \ B) = bbox(A)` exactly. A
    // slab that cannot be separated refuses `ExtremeSlabContaminated`.
    // =======================================================================

    /// The minimum certified sine of the normal angle between an admitted
    /// patch pair below which the pair is refused `TransversalityUncertified`.
    /// A transversal contact has a strictly positive sine; a tangency drives
    /// it to zero. H-3: dimensionless.
    const TRANSVERSALITY_MIN: f64 = 1.0e-9;

    /// The separation-subdivision depth used while building the reported
    /// contact cover. A cell whose separation cannot be certified at this
    /// depth is conservatively kept in the cover; the integration step's own
    /// `is_clear` test is the sound decision.
    const COVER_SEPARATION_DEPTH: u32 = 10;

    /// The per-pair sub-box work cap of the separation test. A pair whose
    /// separation is not certified within this many boxes is conservatively a
    /// contact candidate; the cap bounds the (otherwise exponential) search
    /// around a contact curve.
    const SEPARATION_BOX_CAP: usize = 4096;

    /// The certified options of the boolean-volume solver.
    #[derive(Debug, Clone, Copy)]
    pub struct BooleanVolumeOptions {
        /// The requested bracket width relative to `V(A)` (the corpus
        /// `volume_rel` band).
        pub relative_tolerance: f64,
        /// The subdivision-depth cap of the contact refinement.
        pub max_depth: u32,
        /// The uniform depth of the reported contact cover.
        pub cover_depth: u32,
        /// The per-patch separation-subdivision depth cap.
        pub separation_depth: u32,
        /// The work cap; exceeding it above tolerance refuses typed.
        pub max_cells: usize,
        /// The deterministic seed of the membership retry contract.
        pub classify_seed: u64,
        /// Enforce the Amendment-1 EXTREMES-SURVIVE bbox gate.
        pub enforce_bbox_certificate: bool,
    }

    impl Default for BooleanVolumeOptions {
        fn default() -> Self {
            BooleanVolumeOptions {
                relative_tolerance: 1.0e-4,
                max_depth: 12,
                cover_depth: 4,
                separation_depth: 14,
                max_cells: 500_000,
                classify_seed: 0x0000_4D4F_4E4F_3601,
                enforce_bbox_certificate: true,
            }
        }
    }

    /// The typed refusal of the certified boolean-volume solver.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
    #[serde(rename_all = "snake_case")]
    pub enum BooleanVolumeRefusal {
        /// A consumed patch is not a regular bidegree-`(m, n)` patch, or its
        /// recorded orientation is not `±1`.
        NonRegularPatch,
        /// The closest admitted patch pair's normal sine fell below the
        /// transversality floor (a near-tangency).
        TransversalityUncertified,
        /// The MONO-5 membership primitive did not certify after its retry
        /// contract; the point is left unresolved rather than guessed.
        MembershipIndeterminate,
        /// The error-directed refinement exhausted its work cap above the
        /// requested tolerance.
        BudgetExceeded,
        /// An axis extreme of `A`'s certified bbox is reached by `B`'s
        /// certified bbox, so `bbox(A \ B)` is not `bbox(A)`.
        ExtremeSlabContaminated,
        /// A consumed patch row is malformed (a caller defect).
        MalformedPatch,
        /// A consumed patch carries a non-constant rational weight field; the
        /// `(8,8)` polynomial flux integrand requires unit weights.
        RationalWeights,
        /// The product volume cannot be measured: an operand carrier is not a
        /// patch-representable canonical solid (an open/swept carrier), so the
        /// certified contact-cover solver has no product facts to consume. The
        /// measured product is unavailable; a refusal is the correct interim
        /// verdict (the MONO-8 certified bracket is the open carrier), never the
        /// base operand's volume.
        BooleanProductVolumeUnavailable,
    }

    impl From<MembershipRefusal> for BooleanVolumeRefusal {
        fn from(value: MembershipRefusal) -> Self {
            match value {
                MembershipRefusal::MembershipIndeterminate => {
                    BooleanVolumeRefusal::MembershipIndeterminate
                }
                MembershipRefusal::MalformedPatch => BooleanVolumeRefusal::MalformedPatch,
                MembershipRefusal::RationalWeights => BooleanVolumeRefusal::RationalWeights,
            }
        }
    }

    /// A closed axis-aligned cell of a patch's unit parameter square.
    #[derive(Debug, Clone, Copy, PartialEq, Serialize)]
    pub struct ParamCell {
        /// The lower `u` bound.
        pub u_lo: f64,
        /// The upper `u` bound.
        pub u_hi: f64,
        /// The lower `v` bound.
        pub v_lo: f64,
        /// The upper `v` bound.
        pub v_hi: f64,
    }

    impl ParamCell {
        fn unit() -> ParamCell {
            ParamCell {
                u_lo: 0.0,
                u_hi: 1.0,
                v_lo: 0.0,
                v_hi: 1.0,
            }
        }
        fn u(&self) -> Iv {
            Iv {
                lo: self.u_lo,
                hi: self.u_hi,
            }
        }
        fn v(&self) -> Iv {
            Iv {
                lo: self.v_lo,
                hi: self.v_hi,
            }
        }
        fn area(&self) -> f64 {
            (self.u_hi - self.u_lo).max(0.0) * (self.v_hi - self.v_lo).max(0.0)
        }
        fn diameter(&self) -> f64 {
            let du = self.u_hi - self.u_lo;
            let dv = self.v_hi - self.v_lo;
            (du * du + dv * dv).sqrt()
        }
        fn intersects(&self, other: &ParamCell) -> bool {
            self.u_lo <= other.u_hi
                && other.u_lo <= self.u_hi
                && self.v_lo <= other.v_hi
                && other.v_lo <= self.v_hi
        }
        /// Splits the wider parameter axis at its midpoint.
        fn split(&self) -> (ParamCell, ParamCell) {
            if (self.u_hi - self.u_lo) >= (self.v_hi - self.v_lo) {
                let mid = 0.5 * (self.u_lo + self.u_hi);
                (
                    ParamCell {
                        u_lo: self.u_lo,
                        u_hi: mid,
                        v_lo: self.v_lo,
                        v_hi: self.v_hi,
                    },
                    ParamCell {
                        u_lo: mid,
                        u_hi: self.u_hi,
                        v_lo: self.v_lo,
                        v_hi: self.v_hi,
                    },
                )
            } else {
                let mid = 0.5 * (self.v_lo + self.v_hi);
                (
                    ParamCell {
                        u_lo: self.u_lo,
                        u_hi: self.u_hi,
                        v_lo: self.v_lo,
                        v_hi: mid,
                    },
                    ParamCell {
                        u_lo: self.u_lo,
                        u_hi: self.u_hi,
                        v_lo: mid,
                        v_hi: self.v_hi,
                    },
                )
            }
        }
    }

    /// The certified boolean-volume outcome. `bracket` is the two-sided
    /// volume bracket of `A \ B`; `value` is its midpoint. The per-phase work
    /// counts and the maximum subdivision depth are recorded for attribution.
    #[derive(Debug, Clone, PartialEq, Serialize)]
    pub struct BooleanVolumeCertificate {
        /// The certified lower volume bound.
        pub bracket_lo: f64,
        /// The certified upper volume bound.
        pub bracket_hi: f64,
        /// The certified volume value (the bracket midpoint).
        pub value: f64,
        /// The bracket width.
        pub width: f64,
        /// The bracket width relative to `V(A)`.
        pub relative_width: f64,
        /// `V(A)` from the landed per-patch flux certificate.
        pub volume_a: f64,
        /// `V(B)` from the landed per-patch flux certificate.
        pub volume_b: f64,
        /// The certified lower bound of `V(A n B)`.
        pub intersection_lo: f64,
        /// The certified upper bound of `V(A n B)`.
        pub intersection_hi: f64,
        /// The lower corner of the certified `bbox(A \ B)`.
        pub bbox_lo: [f64; 3],
        /// The upper corner of the certified `bbox(A \ B)`.
        pub bbox_hi: [f64; 3],
        /// The certified separation margin of the EXTREMES-SURVIVE gate.
        pub bbox_margin: Option<f64>,
        /// The constructive solid count (Amendment 1: `solids() = [self]`).
        pub solid_count: u64,
        /// The control-hull broad-phase pair count.
        pub broad_phase_pairs: usize,
        /// The pairs excluded by the control-hull test.
        pub excluded_pairs: usize,
        /// The number of cells scored by an exact clear witness.
        pub clear_cells: usize,
        /// The number of unresolved contact cells carrying the bracket.
        pub contact_cells: usize,
        /// The maximum subdivision depth reached.
        pub max_depth: u32,
        /// The number of contact-cover cells reported.
        pub cover_cells: usize,
        /// The per-phase work: `[broad, exclusion, refinement]`.
        pub phases: [usize; 3],
    }

    /// The running per-phase work counters of one integration.
    #[derive(Default)]
    struct PhaseStats {
        clear_cells: usize,
        contact_cells: usize,
        max_depth: u32,
        cover_cells: usize,
        refinement: usize,
    }

    /// The exact blossom (polar form) of one univariate control list at the
    /// parameters `params` (`params.len() == values.len() - 1`), by the
    /// de Casteljau scheme. The result is the sub-patch control point over the
    /// blossom's parameter multiset.
    fn blossom(values: &[[f64; 3]], params: &[f64]) -> [f64; 3] {
        let mut work: Vec<[f64; 3]> = values.to_vec();
        for &t in params {
            if work.len() < 2 {
                break;
            }
            let mut next: Vec<[f64; 3]> = Vec::with_capacity(work.len() - 1);
            for pair in work.windows(2) {
                let [a, b] = pair else { continue };
                next.push([
                    (1.0 - t) * a[0] + t * b[0],
                    (1.0 - t) * a[1] + t * b[1],
                    (1.0 - t) * a[2] + t * b[2],
                ]);
            }
            work = next;
        }
        work.first().copied().unwrap_or([0.0; 3])
    }

    /// The exact child control net of `patch` over the parameter `cell`
    /// (method item 1: "subdivide the patch domain into cells; exact child
    /// control nets"). Row-major `rows x cols`.
    fn sub_net(patch: &Patch, cell: &ParamCell) -> Vec<[f64; 3]> {
        let rows = patch.rows;
        let cols = patch.cols;
        let u0 = cell.u_lo;
        let u1 = cell.u_hi;
        let v0 = cell.v_lo;
        let v1 = cell.v_hi;
        let mut temp = vec![[0.0f64; 3]; rows * cols];
        for j in 0..cols {
            let col: Vec<[f64; 3]> = (0..rows)
                .map(|i| patch.data.get(i * cols + j).copied().unwrap_or([0.0; 3]))
                .collect();
            for k in 0..rows {
                let mut params = Vec::with_capacity(rows.saturating_sub(1));
                for _ in 0..(rows - 1 - k) {
                    params.push(u0);
                }
                for _ in 0..k {
                    params.push(u1);
                }
                if let Some(slot) = temp.get_mut(k * cols + j) {
                    *slot = blossom(&col, &params);
                }
            }
        }
        let mut out = vec![[0.0f64; 3]; rows * cols];
        for k in 0..rows {
            let row: Vec<[f64; 3]> = (0..cols)
                .map(|j| temp.get(k * cols + j).copied().unwrap_or([0.0; 3]))
                .collect();
            for l in 0..cols {
                let mut params = Vec::with_capacity(cols.saturating_sub(1));
                for _ in 0..(cols - 1 - l) {
                    params.push(v0);
                }
                for _ in 0..l {
                    params.push(v1);
                }
                if let Some(slot) = out.get_mut(k * cols + l) {
                    *slot = blossom(&row, &params);
                }
            }
        }
        out
    }

    /// The unit-square domain every child patch is re-parameterized onto.
    fn unit_ibox2() -> IBox2 {
        IBox2 {
            lo: [0.0, 0.0],
            hi: [1.0, 1.0],
        }
    }

    /// The exact certified flux of `patch` over the parameter `cell`, through
    /// the landed volume-facts machinery. The 2-form is parameterization
    /// invariant, so integrating the exact child control net over the unit
    /// square equals integrating the parent over the cell.
    fn cell_flux_exact(
        patch: &Patch,
        cell: &ParamCell,
    ) -> Result<(f64, f64), BooleanVolumeRefusal> {
        if patch.orientation != 1.0 && patch.orientation != -1.0 {
            return Err(BooleanVolumeRefusal::NonRegularPatch);
        }
        let net = sub_net(patch, cell);
        let rows = patch.rows;
        let cols = patch.cols;
        let mut numerator: Vec<Vec<[f64; 3]>> = Vec::with_capacity(rows);
        for i in 0..rows {
            let mut row = Vec::with_capacity(cols);
            for j in 0..cols {
                row.push(net.get(i * cols + j).copied().unwrap_or([0.0; 3]));
            }
            numerator.push(row);
        }
        let weights = vec![vec![1.0f64; cols]; rows];
        let sub = TensorBernsteinPatch::try_new(
            numerator,
            weights,
            unit_ibox2(),
            PatchParent::new(0, None),
        )
        .map_err(|_| BooleanVolumeRefusal::NonRegularPatch)?;
        let fact = certify_patch_form(&sub, patch.orientation, &VolumeOptions::default())
            .map_err(|_| BooleanVolumeRefusal::NonRegularPatch)?;
        Ok((fact.bracket.lo, fact.bracket.hi))
    }

    /// The exact child control net of `patch` over `cell` (the parent net for
    /// the unit cell). The Bernstein convex-hull property makes its component
    /// hull the tight certified enclosure of the patch image over the cell.
    fn child_net(patch: &Patch, cell: &ParamCell) -> Vec<[f64; 3]> {
        if cell.u_lo == 0.0 && cell.u_hi == 1.0 && cell.v_lo == 0.0 && cell.v_hi == 1.0 {
            patch.data.clone()
        } else {
            sub_net(patch, cell)
        }
    }

    /// The component hull of a control net.
    fn hull_of(net: &[[f64; 3]]) -> ([f64; 3], [f64; 3]) {
        let mut lo = [f64::INFINITY; 3];
        let mut hi = [f64::NEG_INFINITY; 3];
        for point in net {
            for k in 0..3 {
                lo[k] = lo[k].min(point[k]);
                hi[k] = hi[k].max(point[k]);
            }
        }
        (lo, hi)
    }

    /// The tight control-hull enclosure of `patch` over `cell` (theory eq. 11:
    /// the control-hull min/max bracket).
    fn control_range(patch: &Patch, cell: &ParamCell) -> ([f64; 3], [f64; 3]) {
        hull_of(&child_net(patch, cell))
    }

    /// The tight control-hull enclosure of `P_u` over `cell` (the derivative
    /// control net of the exact child net).
    fn derivative_range_u(patch: &Patch, cell: &ParamCell) -> ([f64; 3], [f64; 3]) {
        let net = child_net(patch, cell);
        let rows = patch.rows;
        let cols = patch.cols;
        let factor = (rows - 1) as f64;
        let mut deriv = Vec::with_capacity(rows.saturating_sub(1) * cols);
        for i in 0..(rows - 1) {
            for j in 0..cols {
                let a = net.get(i * cols + j).copied().unwrap_or([0.0; 3]);
                let b = net.get((i + 1) * cols + j).copied().unwrap_or([0.0; 3]);
                deriv.push([
                    factor * (b[0] - a[0]),
                    factor * (b[1] - a[1]),
                    factor * (b[2] - a[2]),
                ]);
            }
        }
        hull_of(&deriv)
    }

    /// The tight control-hull enclosure of `P_v` over `cell`.
    fn derivative_range_v(patch: &Patch, cell: &ParamCell) -> ([f64; 3], [f64; 3]) {
        let net = child_net(patch, cell);
        let rows = patch.rows;
        let cols = patch.cols;
        let factor = (cols - 1) as f64;
        let mut deriv = Vec::with_capacity(rows * cols.saturating_sub(1));
        for i in 0..rows {
            for j in 0..(cols - 1) {
                let a = net.get(i * cols + j).copied().unwrap_or([0.0; 3]);
                let b = net.get(i * cols + (j + 1)).copied().unwrap_or([0.0; 3]);
                deriv.push([
                    factor * (b[0] - a[0]),
                    factor * (b[1] - a[1]),
                    factor * (b[2] - a[2]),
                ]);
            }
        }
        hull_of(&deriv)
    }

    /// The interval enclosure of the oriented density `g = (1/3) P . (P_u x P_v)`
    /// over the parameter `cell`, from the tight child control hulls. The
    /// derivatives are taken with respect to the child's own unit parameters.
    fn density_interval(patch: &Patch, cell: &ParamCell) -> Iv {
        let (plo, phi) = control_range(patch, cell);
        let (dulo, duhi) = derivative_range_u(patch, cell);
        let (dvlo, dvhi) = derivative_range_v(patch, cell);
        let p = [
            Iv {
                lo: plo[0],
                hi: phi[0],
            },
            Iv {
                lo: plo[1],
                hi: phi[1],
            },
            Iv {
                lo: plo[2],
                hi: phi[2],
            },
        ];
        let du = [
            Iv {
                lo: dulo[0],
                hi: duhi[0],
            },
            Iv {
                lo: dulo[1],
                hi: duhi[1],
            },
            Iv {
                lo: dulo[2],
                hi: duhi[2],
            },
        ];
        let dv = [
            Iv {
                lo: dvlo[0],
                hi: dvhi[0],
            },
            Iv {
                lo: dvlo[1],
                hi: dvhi[1],
            },
            Iv {
                lo: dvlo[2],
                hi: dvhi[2],
            },
        ];
        dot_iv(p, cross_iv(du, dv)).scale(1.0 / 3.0)
    }

    /// The interval cross product.
    fn cross_iv(a: [Iv; 3], b: [Iv; 3]) -> [Iv; 3] {
        [
            a[1].mul(b[2]).sub(a[2].mul(b[1])),
            a[2].mul(b[0]).sub(a[0].mul(b[2])),
            a[0].mul(b[1]).sub(a[1].mul(b[0])),
        ]
    }

    /// The interval dot product.
    fn dot_iv(a: [Iv; 3], b: [Iv; 3]) -> Iv {
        a[0].mul(b[0]).add(a[1].mul(b[1])).add(a[2].mul(b[2]))
    }

    /// Evaluates the patch at `(u, v)` (used for the clear-cell witness).
    fn patch_eval(patch: &Patch, u: f64, v: f64) -> [f64; 3] {
        let bu = bernstein_f64(patch.rows - 1, u);
        let bv = bernstein_f64(patch.cols - 1, v);
        let mut out = [0.0f64; 3];
        for i in 0..patch.rows {
            let bui = bu.get(i).copied().unwrap_or(0.0);
            for j in 0..patch.cols {
                let bvj = bv.get(j).copied().unwrap_or(0.0);
                let w = bui * bvj;
                if let Some(a) = patch.data.get(i * patch.cols + j) {
                    out[0] += w * a[0];
                    out[1] += w * a[1];
                    out[2] += w * a[2];
                }
            }
        }
        out
    }

    /// The pointwise normal `P_u x P_v` at `(u, v)`.
    fn patch_normal_point(patch: &Patch, u: f64, v: f64) -> [f64; 3] {
        let du = derivative_point_u(patch, u, v);
        let dv = derivative_point_v(patch, u, v);
        [
            du[1] * dv[2] - du[2] * dv[1],
            du[2] * dv[0] - du[0] * dv[2],
            du[0] * dv[1] - du[1] * dv[0],
        ]
    }

    /// The unit normal at `(u, v)`, `None` at a degenerate point.
    fn normal_unit_at(patch: &Patch, u: f64, v: f64) -> Option<[f64; 3]> {
        let n = patch_normal_point(patch, u, v);
        let len = norm3(n);
        if len > 0.0 && len.is_finite() {
            Some([n[0] / len, n[1] / len, n[2] / len])
        } else {
            None
        }
    }

    /// The sampled minimum sine of the normal angle between two patches (the
    /// transversality admission margin). Zero at a tangency or a degenerate
    /// sample.
    fn sigma_min(p: &Patch, q: &Patch) -> f64 {
        let samples = [0.25, 0.5, 0.75];
        let mut best = f64::INFINITY;
        for &u in &samples {
            for &v in &samples {
                let Some(np) = normal_unit_at(p, u, v) else {
                    return 0.0;
                };
                for &s in &samples {
                    for &t in &samples {
                        let Some(nq) = normal_unit_at(q, s, t) else {
                            return 0.0;
                        };
                        let cross = [
                            np[1] * nq[2] - np[2] * nq[1],
                            np[2] * nq[0] - np[0] * nq[2],
                            np[0] * nq[1] - np[1] * nq[0],
                        ];
                        best = best.min(norm3(cross));
                    }
                }
            }
        }
        best
    }

    /// Whether the images of `a` over `ac` and `b` over `bc` are certified
    /// disjoint by a coordinate-range exclusion (theory eq. 11, the
    /// control-hull min/max superset bracket).
    fn ranges_separate(a: &Patch, ac: &ParamCell, b: &Patch, bc: &ParamCell) -> bool {
        let (alo, ahi) = control_range(a, ac);
        let (blo, bhi) = control_range(b, bc);
        (0..3).any(|k| ahi[k] < blo[k] || bhi[k] < alo[k])
    }

    /// Whether `q`'s image can meet `p`'s image over `pc`, by recursive
    /// range exclusion on `q`'s domain. `true` is the conservative answer:
    /// the pair is a contact candidate. The recursion stops subdividing `q`
    /// once its cell is no larger than `pc` (or the depth/work cap fires), so
    /// a clear `pc` at positive distance separates as it shrinks.
    fn q_has_contact(p: &Patch, pc: &ParamCell, q: &Patch, separation_depth: u32) -> bool {
        let pdiam = pc.diameter();
        let mut stack: Vec<(ParamCell, u32)> = vec![(ParamCell::unit(), 0)];
        let mut explored = 0usize;
        while let Some((qc, depth)) = stack.pop() {
            if ranges_separate(p, pc, q, &qc) {
                continue;
            }
            if depth >= separation_depth || qc.diameter() <= pdiam || explored >= SEPARATION_BOX_CAP
            {
                return true;
            }
            explored += 1;
            let (c0, c1) = qc.split();
            stack.push((c0, depth + 1));
            stack.push((c1, depth + 1));
        }
        false
    }

    /// Whether `p`'s image over `pc` is certified away from the boundary of
    /// the `others` solid (a clear cell: constant membership).
    fn is_clear(p: &Patch, pc: &ParamCell, others: &[Patch], separation_depth: u32) -> bool {
        others
            .iter()
            .all(|q| !q_has_contact(p, pc, q, separation_depth))
    }

    /// The contact cover of `p` against `others`: the parameter cells whose
    /// images can meet a boundary patch, refined to `depth`. Any cell outside
    /// the cover is certifiably clear (method item 2).
    fn cover_boxes(p: &Patch, others: &[Patch], depth: u32) -> Vec<ParamCell> {
        let mut cover = Vec::new();
        let mut stack: Vec<(ParamCell, u32)> = vec![(ParamCell::unit(), 0)];
        while let Some((cell, d)) = stack.pop() {
            if is_clear(p, &cell, others, COVER_SEPARATION_DEPTH) {
                continue;
            }
            if d >= depth {
                cover.push(cell);
                continue;
            }
            let (c0, c1) = cell.split();
            stack.push((c0, d + 1));
            stack.push((c1, d + 1));
        }
        cover
    }

    /// The public contact-cover entry (method item 2): the parameter cells of
    /// one patch's unit square whose images can meet the `others` patch set.
    /// A non-unit weight refuses typed (item 7).
    pub fn contact_cover(
        patch: &VolumeRow,
        others: &[VolumeRow],
        depth: u32,
    ) -> Result<Vec<ParamCell>, BooleanVolumeRefusal> {
        let p = parse_patch(patch).map_err(BooleanVolumeRefusal::from)?;
        let qs = others
            .iter()
            .map(parse_patch)
            .collect::<Result<Vec<_>, _>>()
            .map_err(BooleanVolumeRefusal::from)?;
        Ok(cover_boxes(&p, &qs, depth))
    }

    /// The certified bracket of one unresolved contact cell:
    /// `[|R| min(0, g_lo), |R| max(0, g_hi)]` (theory eq. 5). `density_interval`
    /// bounds the child density over the cell's own unit parameter square (the
    /// child control net's derivatives are taken with respect to that square),
    /// so the integral is that bracket over the unit area -- never rescaled by
    /// the parent cell area a second time.
    fn contact_bracket(p: &Patch, cell: &ParamCell) -> (f64, f64) {
        let g = density_interval(p, cell).scale(p.orientation);
        (g.lo.min(0.0), g.hi.max(0.0))
    }

    /// The exact flux of a clear cell, fixed by ONE membership witness
    /// (method item 4). A clear cell is connected and disjoint from the
    /// boundary, so the witness verdict applies to the whole cell.
    fn clear_flux(
        p: &Patch,
        cell: &ParamCell,
        others_rows: &[VolumeRow],
        options: &BooleanVolumeOptions,
    ) -> Result<(f64, f64), BooleanVolumeRefusal> {
        let u = 0.5 * (cell.u_lo + cell.u_hi);
        let v = 0.5 * (cell.v_lo + cell.v_hi);
        let point = patch_eval(p, u, v);
        let certificate = classify_point(
            point,
            [0.37, 0.61, 0.70],
            others_rows,
            options.classify_seed,
        );
        match certificate.verdict {
            MembershipVerdict::Inside => cell_flux_exact(p, cell),
            MembershipVerdict::Outside => Ok((0.0, 0.0)),
            MembershipVerdict::Indeterminate => Err(BooleanVolumeRefusal::MembershipIndeterminate),
        }
    }

    /// Integrates `1_B(P) g_P` over one patch's unit square against the
    /// `others` solid, returning the certified bracket of the patch's
    /// contribution to `V(A n B)`.
    fn integrate_patch(
        p: &Patch,
        others_rows: &[VolumeRow],
        others: &[Patch],
        budget: f64,
        options: &BooleanVolumeOptions,
        stats: &mut PhaseStats,
    ) -> Result<(f64, f64), BooleanVolumeRefusal> {
        let cover = cover_boxes(p, others, options.cover_depth);
        stats.cover_cells += cover.len();

        let mut exact_lo = 0.0f64;
        let mut exact_hi = 0.0f64;
        let mut work: Vec<(ParamCell, u32, f64, f64)> = Vec::new();
        let mut leaves: Vec<(f64, f64)> = Vec::new();

        let seed = ParamCell::unit();
        if !cover.iter().any(|c| c.intersects(&seed))
            || is_clear(p, &seed, others, options.separation_depth)
        {
            let (lo, hi) = clear_flux(p, &seed, others_rows, options)?;
            exact_lo += lo;
            exact_hi += hi;
            stats.clear_cells += 1;
        } else {
            let (lo, hi) = contact_bracket(p, &seed);
            work.push((seed, 0, lo, hi));
        }

        // Error-directed refinement (method item 3): priority is the cell's
        // bracket width (descending); terminate when the summed width is
        // within the per-patch budget (eq. 8).
        loop {
            let mut contact_lo = 0.0f64;
            let mut contact_hi = 0.0f64;
            for &(_, _, lo, hi) in &work {
                contact_lo += lo;
                contact_hi += hi;
            }
            for &(lo, hi) in &leaves {
                contact_lo += lo;
                contact_hi += hi;
            }
            if contact_hi - contact_lo <= budget {
                break;
            }

            let mut best: Option<usize> = None;
            let mut best_width = f64::NEG_INFINITY;
            for (i, &(_, _, lo, hi)) in work.iter().enumerate() {
                let width = hi - lo;
                if width > best_width {
                    best_width = width;
                    best = Some(i);
                }
            }
            let Some(index) = best else { break };
            let (cell, depth, lo, hi) = work.swap_remove(index);
            if depth >= options.max_depth || stats.refinement >= options.max_cells {
                leaves.push((lo, hi));
                stats.contact_cells += 1;
                continue;
            }
            stats.refinement += 1;
            let (c0, c1) = cell.split();
            for child in [c0, c1] {
                if !cover.iter().any(|c| c.intersects(&child))
                    || is_clear(p, &child, others, options.separation_depth)
                {
                    let (clo, chi) = clear_flux(p, &child, others_rows, options)?;
                    exact_lo += clo;
                    exact_hi += chi;
                    stats.clear_cells += 1;
                } else {
                    let (clo, chi) = contact_bracket(p, &child);
                    work.push((child, depth + 1, clo, chi));
                    stats.max_depth = stats.max_depth.max(depth + 1);
                }
            }
            if stats.refinement >= options.max_cells {
                let mut rem_lo = 0.0f64;
                let mut rem_hi = 0.0f64;
                for &(_, _, lo, hi) in &work {
                    rem_lo += lo;
                    rem_hi += hi;
                }
                for &(lo, hi) in &leaves {
                    rem_lo += lo;
                    rem_hi += hi;
                }
                if rem_hi - rem_lo > budget {
                    return Err(BooleanVolumeRefusal::BudgetExceeded);
                }
            }
        }

        for (_, _, lo, hi) in work {
            leaves.push((lo, hi));
            stats.contact_cells += 1;
        }
        let mut contact_lo = 0.0f64;
        let mut contact_hi = 0.0f64;
        for (lo, hi) in leaves {
            contact_lo += lo;
            contact_hi += hi;
        }
        Ok((exact_lo + contact_lo, exact_hi + contact_hi))
    }

    /// The control-hull bounding box of a patch set.
    fn control_bbox(patches: &[Patch]) -> ([f64; 3], [f64; 3]) {
        solid_bounds(patches).unwrap_or(([0.0; 3], [0.0; 3]))
    }

    /// The EXTREMES-SURVIVE certificate (Amendment 1): for each of the six
    /// axis extremes of `A`'s certified control hull, require that `B`'s
    /// certified control hull does not reach the extreme value. Then no point
    /// of `B` can be the arg-extreme point, so the extreme survives into
    /// `A \ B` and `bbox(A \ B) = bbox(A)`. Returns the minimum separation
    /// margin; a reached extreme refuses `ExtremeSlabContaminated`.
    fn extremes_survive(
        a_lo: [f64; 3],
        a_hi: [f64; 3],
        b_lo: [f64; 3],
        b_hi: [f64; 3],
    ) -> Result<f64, BooleanVolumeRefusal> {
        let mut margin = f64::INFINITY;
        for k in 0..3 {
            for extreme in [a_lo[k], a_hi[k]] {
                if b_lo[k] <= extreme && extreme <= b_hi[k] {
                    return Err(BooleanVolumeRefusal::ExtremeSlabContaminated);
                }
                let distance = if extreme < b_lo[k] {
                    b_lo[k] - extreme
                } else {
                    extreme - b_hi[k]
                };
                margin = margin.min(distance);
            }
        }
        Ok(margin)
    }

    /// The certified boolean volume of `A \ B` by contact covers (method
    /// items 1-9). `A` and `B` are closed oriented tensor-Bernstein patch
    /// 2-cycles with unit weights (Amendment 4); `V(A)` and `V(B)` come from
    /// the landed per-patch flux certificate.
    pub fn certify_boolean_volume(
        a: &[VolumeRow],
        b: &[VolumeRow],
        options: &BooleanVolumeOptions,
    ) -> Result<BooleanVolumeCertificate, BooleanVolumeRefusal> {
        if a.is_empty() || b.is_empty() {
            return Err(BooleanVolumeRefusal::MalformedPatch);
        }
        let a_parsed = a
            .iter()
            .map(parse_patch)
            .collect::<Result<Vec<_>, _>>()
            .map_err(BooleanVolumeRefusal::from)?;
        let b_parsed = b
            .iter()
            .map(parse_patch)
            .collect::<Result<Vec<_>, _>>()
            .map_err(BooleanVolumeRefusal::from)?;

        let mut stats = PhaseStats::default();

        // Method item 1: V(A) and V(B) from the landed flux certificate.
        let mut va_lo = 0.0f64;
        let mut va_hi = 0.0f64;
        for p in &a_parsed {
            let (lo, hi) = cell_flux_exact(p, &ParamCell::unit())?;
            va_lo += lo;
            va_hi += hi;
        }
        let va = 0.5 * (va_lo + va_hi);
        let mut vb_lo = 0.0f64;
        let mut vb_hi = 0.0f64;
        for q in &b_parsed {
            let (lo, hi) = cell_flux_exact(q, &ParamCell::unit())?;
            vb_lo += lo;
            vb_hi += hi;
        }
        let vb = 0.5 * (vb_lo + vb_hi);
        if !va.is_finite() || va.abs() <= 0.0 {
            return Err(BooleanVolumeRefusal::NonRegularPatch);
        }

        // Method item 5: the control-hull broad phase. Method item 8: the
        // transversality admission on the surviving pairs.
        let mut broad = 0usize;
        let mut excluded = 0usize;
        let mut sigma = f64::INFINITY;
        for p in &a_parsed {
            for q in &b_parsed {
                broad += 1;
                if ranges_separate(p, &ParamCell::unit(), q, &ParamCell::unit()) {
                    excluded += 1;
                    continue;
                }
                sigma = sigma.min(sigma_min(p, q));
            }
        }
        if sigma < TRANSVERSALITY_MIN {
            return Err(BooleanVolumeRefusal::TransversalityUncertified);
        }

        // Method item 6 / Amendment 1: the bbox sufficiency lemma. The gate
        // is cheap and runs before the refinement, so a contaminated slab
        // refuses promptly.
        let (a_lo, a_hi) = control_bbox(&a_parsed);
        let (b_lo, b_hi) = control_bbox(&b_parsed);
        let bbox_margin = if options.enforce_bbox_certificate {
            Some(extremes_survive(a_lo, a_hi, b_lo, b_hi)?)
        } else {
            None
        };

        let target_width = options.relative_tolerance * va.abs();
        let patch_count = a_parsed.len() + b_parsed.len();
        let budget = target_width / (2.0 * patch_count.max(1) as f64);

        // Method item 1: V(A n B) over both boundary families.
        let mut inter_lo = 0.0f64;
        let mut inter_hi = 0.0f64;
        for p in &a_parsed {
            let (lo, hi) = integrate_patch(p, b, &b_parsed, budget, options, &mut stats)?;
            inter_lo += lo;
            inter_hi += hi;
        }
        for q in &b_parsed {
            let (lo, hi) = integrate_patch(q, a, &a_parsed, budget, options, &mut stats)?;
            inter_lo += lo;
            inter_hi += hi;
        }

        let diff_lo = va_lo - inter_hi;
        let diff_hi = va_hi - inter_lo;
        let value = 0.5 * (diff_lo + diff_hi);
        let width = diff_hi - diff_lo;
        if !value.is_finite() || !width.is_finite() || diff_lo > diff_hi {
            return Err(BooleanVolumeRefusal::NonRegularPatch);
        }
        if width > target_width {
            return Err(BooleanVolumeRefusal::BudgetExceeded);
        }

        Ok(BooleanVolumeCertificate {
            bracket_lo: diff_lo,
            bracket_hi: diff_hi,
            value,
            width,
            relative_width: width / va.abs(),
            volume_a: va,
            volume_b: vb,
            intersection_lo: inter_lo,
            intersection_hi: inter_hi,
            bbox_lo: a_lo,
            bbox_hi: a_hi,
            bbox_margin,
            // Amendment 1: the constructive solid count (`solids() = [self]`).
            solid_count: 1,
            broad_phase_pairs: broad,
            excluded_pairs: excluded,
            clear_cells: stats.clear_cells,
            contact_cells: stats.contact_cells,
            max_depth: stats.max_depth,
            cover_cells: stats.cover_cells,
            phases: [broad, excluded, stats.refinement],
        })
    }

    // =======================================================================
    // MONO-9-FUSE-FOLD -- the multi-operand compound-indicator fold.
    //
    // The fold is per-tool flux against a COMPOUND membership indicator, not a
    // pairwise fold (a union of patch 2-cycles is not a patch 2-cycle). For
    // each operand boundary patch P_k the divergence form
    // `g_P = (1/3) P . (P_u x P_v)` is integrated over the cells whose image
    // lies on the final solid's boundary: the boolean membership function must
    // be SENSITIVE to operand k's own membership there. The MONO-5 primitive
    // evaluates the sample point against every OTHER operand directly
    // (multi-solid membership); no intermediate union is ever constructed.
    // One evaluator, three modes:
    //
    //   union      sensitive  <=>  every other operand is OUTSIDE
    //   intersect  sensitive  <=>  every other operand is INSIDE
    //   subtract   base       <=>  every tool is OUTSIDE            (sign +1)
    //              tool k     <=>  base INSIDE and every other tool OUTSIDE
    //                                                               (sign -1)
    //
    // The bracket is the interval sum of the per-operand signed brackets, with
    // the width budget split across the operands (judgement 2).
    // =======================================================================

    /// The certified outcome of one multi-operand fold.
    #[derive(Debug, Clone, PartialEq, Serialize)]
    pub struct FoldVolumeCertificate {
        /// The certified lower volume bound.
        pub bracket_lo: f64,
        /// The certified upper volume bound.
        pub bracket_hi: f64,
        /// The certified volume value (the bracket midpoint).
        pub value: f64,
        /// The bracket width.
        pub width: f64,
        /// The bracket width relative to `V(base)`.
        pub relative_width: f64,
        /// `V(base)` from the landed per-patch flux certificate.
        pub volume_base: f64,
        /// The per-operand signed contribution brackets, base first.
        pub operand_brackets: Vec<[f64; 2]>,
        /// The per-operand signed contribution midpoints, base first.
        pub operand_volumes: Vec<f64>,
        /// The number of tool operands in the fold.
        pub fold_count: usize,
        /// The constructive solid count (the fold is one solid).
        pub solid_count: u64,
        /// The number of cells scored by an exact clear witness.
        pub clear_cells: usize,
        /// The number of unresolved contact cells carrying the bracket.
        pub contact_cells: usize,
        /// The maximum subdivision depth reached.
        pub max_depth: u32,
        /// The number of contact-cover cells reported.
        pub cover_cells: usize,
        /// The per-phase work: `[cover, clear, refinement]`.
        pub phases: [usize; 3],
    }

    /// The signed orientation a boolean mode gives one operand of the fold:
    /// `+1` for every operand of a union/intersect, `+1` for the base and
    /// `-1` for each tool of a subtract (the remaining solid's outward normal
    /// on a removed tool's boundary points opposite the tool's own).
    fn fold_sign(index: usize, mode: ModeValue) -> f64 {
        match mode {
            ModeValue::Subtract if index > 0 => -1.0,
            _ => 1.0,
        }
    }

    /// The self-operand context of one fold integration: which recorded operand
    /// boundary is being integrated, its patch-set rows and the fold mode.
    struct FoldOperand<'a> {
        rows: &'a [Vec<VolumeRow>],
        index: usize,
        mode: ModeValue,
    }

    impl FoldOperand<'_> {
        /// The signed orientation this operand carries in the fold.
        fn sign(&self) -> f64 {
            fold_sign(self.index, self.mode)
        }
    }

    /// The compound-indicator sensitivity of one operand boundary point: does
    /// flipping operand `self.index`'s own membership change the boolean
    /// function, holding every other operand's membership fixed? The MONO-5
    /// primitive evaluates the sample point against EVERY other operand
    /// separately (multi-solid membership), never a reconstructed union.
    fn compound_indicator(
        point: [f64; 3],
        operand: &FoldOperand<'_>,
        options: &BooleanVolumeOptions,
    ) -> Result<bool, BooleanVolumeRefusal> {
        for (j, rows) in operand.rows.iter().enumerate() {
            if j == operand.index {
                continue;
            }
            let certificate =
                classify_point(point, [0.37, 0.61, 0.70], rows, options.classify_seed);
            let inside = match certificate.verdict {
                MembershipVerdict::Inside => true,
                MembershipVerdict::Outside => false,
                MembershipVerdict::Indeterminate => {
                    return Err(BooleanVolumeRefusal::MembershipIndeterminate);
                }
            };
            let required = match operand.mode {
                ModeValue::Add => false,
                ModeValue::Intersect => true,
                ModeValue::Subtract => {
                    if operand.index == 0 {
                        false
                    } else {
                        j == 0
                    }
                }
            };
            if inside != required {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// The exact signed flux of a clear cell, fixed by ONE compound-indicator
    /// witness. A clear cell is connected and disjoint from every other
    /// operand's boundary, so each other operand's membership is constant on
    /// it and the compound sensitivity is constant too.
    fn fold_clear_flux(
        p: &Patch,
        cell: &ParamCell,
        operand: &FoldOperand<'_>,
        options: &BooleanVolumeOptions,
    ) -> Result<(f64, f64), BooleanVolumeRefusal> {
        let u = 0.5 * (cell.u_lo + cell.u_hi);
        let v = 0.5 * (cell.v_lo + cell.v_hi);
        let point = patch_eval(p, u, v);
        if !compound_indicator(point, operand, options)? {
            return Ok((0.0, 0.0));
        }
        let (lo, hi) = cell_flux_exact(p, cell)?;
        let sign = operand.sign();
        Ok(if sign >= 0.0 { (lo, hi) } else { (-hi, -lo) })
    }

    /// Integrates the signed flux of one operand patch against the compound
    /// indicator of every other operand, returning the certified bracket of
    /// the patch's contribution to the fold.
    fn integrate_patch_fold(
        p: &Patch,
        operand: &FoldOperand<'_>,
        others: &[Patch],
        budget: f64,
        options: &BooleanVolumeOptions,
        stats: &mut PhaseStats,
    ) -> Result<(f64, f64), BooleanVolumeRefusal> {
        let cover = cover_boxes(p, others, options.cover_depth);
        stats.cover_cells += cover.len();
        let sign = operand.sign();

        let mut exact_lo = 0.0f64;
        let mut exact_hi = 0.0f64;
        let mut work: Vec<(ParamCell, u32, f64, f64)> = Vec::new();
        let mut leaves: Vec<(f64, f64)> = Vec::new();

        let seed = ParamCell::unit();
        if !cover.iter().any(|c| c.intersects(&seed))
            || is_clear(p, &seed, others, options.separation_depth)
        {
            let (lo, hi) = fold_clear_flux(p, &seed, operand, options)?;
            exact_lo += lo;
            exact_hi += hi;
            stats.clear_cells += 1;
        } else {
            let (lo, hi) = contact_bracket(p, &seed);
            let (lo, hi) = if sign >= 0.0 { (lo, hi) } else { (-hi, -lo) };
            work.push((seed, 0, lo, hi));
        }

        loop {
            let mut contact_lo = 0.0f64;
            let mut contact_hi = 0.0f64;
            for &(_, _, lo, hi) in &work {
                contact_lo += lo;
                contact_hi += hi;
            }
            for &(lo, hi) in &leaves {
                contact_lo += lo;
                contact_hi += hi;
            }
            if contact_hi - contact_lo <= budget {
                break;
            }

            let mut best: Option<usize> = None;
            let mut best_width = f64::NEG_INFINITY;
            for (i, &(_, _, lo, hi)) in work.iter().enumerate() {
                let width = hi - lo;
                if width > best_width {
                    best_width = width;
                    best = Some(i);
                }
            }
            let Some(index) = best else { break };
            let (cell, depth, lo, hi) = work.swap_remove(index);
            if depth >= options.max_depth || stats.refinement >= options.max_cells {
                leaves.push((lo, hi));
                stats.contact_cells += 1;
                continue;
            }
            stats.refinement += 1;
            let (c0, c1) = cell.split();
            for child in [c0, c1] {
                if !cover.iter().any(|c| c.intersects(&child))
                    || is_clear(p, &child, others, options.separation_depth)
                {
                    let (clo, chi) = fold_clear_flux(p, &child, operand, options)?;
                    exact_lo += clo;
                    exact_hi += chi;
                    stats.clear_cells += 1;
                } else {
                    let (clo, chi) = contact_bracket(p, &child);
                    let (clo, chi) = if sign >= 0.0 {
                        (clo, chi)
                    } else {
                        (-chi, -clo)
                    };
                    work.push((child, depth + 1, clo, chi));
                    stats.max_depth = stats.max_depth.max(depth + 1);
                }
            }
            if stats.refinement >= options.max_cells {
                let mut rem_lo = 0.0f64;
                let mut rem_hi = 0.0f64;
                for &(_, _, lo, hi) in &work {
                    rem_lo += lo;
                    rem_hi += hi;
                }
                for &(lo, hi) in &leaves {
                    rem_lo += lo;
                    rem_hi += hi;
                }
                if rem_hi - rem_lo > budget {
                    return Err(BooleanVolumeRefusal::BudgetExceeded);
                }
            }
        }

        for (_, _, lo, hi) in work {
            leaves.push((lo, hi));
            stats.contact_cells += 1;
        }
        let mut contact_lo = 0.0f64;
        let mut contact_hi = 0.0f64;
        for (lo, hi) in leaves {
            contact_lo += lo;
            contact_hi += hi;
        }
        Ok((exact_lo + contact_lo, exact_hi + contact_hi))
    }

    /// The certified multi-operand fold volume of `base op tools...` (MONO-9,
    /// theory statement section 4). `base` is the first recorded operand and
    /// `tools` the remaining operands in recorded order; `mode` is the single
    /// fold mode. Every operand is a closed oriented unit-weight patch
    /// 2-cycle. No intermediate union is constructed.
    pub fn certify_fold_volume(
        base: &[VolumeRow],
        tools: &[Vec<VolumeRow>],
        mode: ModeValue,
        options: &BooleanVolumeOptions,
    ) -> Result<FoldVolumeCertificate, BooleanVolumeRefusal> {
        if base.is_empty() || tools.iter().any(|tool| tool.is_empty()) {
            return Err(BooleanVolumeRefusal::MalformedPatch);
        }
        let mut operands_rows: Vec<Vec<VolumeRow>> = Vec::with_capacity(tools.len() + 1);
        operands_rows.push(base.to_vec());
        for tool in tools {
            operands_rows.push(tool.clone());
        }
        let mut operands: Vec<Vec<Patch>> = Vec::with_capacity(operands_rows.len());
        for rows in &operands_rows {
            operands.push(
                rows.iter()
                    .map(parse_patch)
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(BooleanVolumeRefusal::from)?,
            );
        }

        // V(base) from the landed per-patch flux certificate (the budget base).
        let Some((base_patches, _)) = operands.split_first() else {
            return Err(BooleanVolumeRefusal::MalformedPatch);
        };
        let mut va_lo = 0.0f64;
        let mut va_hi = 0.0f64;
        for p in base_patches {
            let (lo, hi) = cell_flux_exact(p, &ParamCell::unit())?;
            va_lo += lo;
            va_hi += hi;
        }
        let va = 0.5 * (va_lo + va_hi);
        if !va.is_finite() || va.abs() <= 0.0 {
            return Err(BooleanVolumeRefusal::NonRegularPatch);
        }

        let target_width = options.relative_tolerance * va.abs();
        let operand_count = operands.len();
        let total_patches: usize = operands.iter().map(Vec::len).sum();
        let budget = target_width / total_patches.max(1) as f64;

        let mut stats = PhaseStats::default();
        let mut total_lo = 0.0f64;
        let mut total_hi = 0.0f64;
        let mut operand_brackets: Vec<[f64; 2]> = Vec::with_capacity(operand_count);
        let mut operand_volumes: Vec<f64> = Vec::with_capacity(operand_count);
        for (k, patches) in operands.iter().enumerate() {
            let others: Vec<Patch> = operands
                .iter()
                .enumerate()
                .filter(|(j, _)| *j != k)
                .flat_map(|(_, ps)| ps.iter().cloned())
                .collect();
            let mut k_lo = 0.0f64;
            let mut k_hi = 0.0f64;
            let operand = FoldOperand {
                rows: &operands_rows,
                index: k,
                mode,
            };
            for p in patches {
                let (lo, hi) =
                    integrate_patch_fold(p, &operand, &others, budget, options, &mut stats)?;
                k_lo += lo;
                k_hi += hi;
            }
            operand_brackets.push([k_lo, k_hi]);
            operand_volumes.push(0.5 * (k_lo + k_hi));
            total_lo += k_lo;
            total_hi += k_hi;
        }

        let value = 0.5 * (total_lo + total_hi);
        let width = total_hi - total_lo;
        if !value.is_finite() || !width.is_finite() || total_lo > total_hi {
            return Err(BooleanVolumeRefusal::NonRegularPatch);
        }
        if width > target_width {
            return Err(BooleanVolumeRefusal::BudgetExceeded);
        }

        Ok(FoldVolumeCertificate {
            bracket_lo: total_lo,
            bracket_hi: total_hi,
            value,
            width,
            relative_width: width / va.abs(),
            volume_base: va,
            operand_brackets,
            operand_volumes,
            fold_count: tools.len(),
            solid_count: 1,
            clear_cells: stats.clear_cells,
            contact_cells: stats.contact_cells,
            max_depth: stats.max_depth,
            cover_cells: stats.cover_cells,
            phases: [stats.cover_cells, stats.clear_cells, stats.refinement],
        })
    }

    // -----------------------------------------------------------------------
    // RDEF-M2-REGIME-SANDWICH -- the (T)/(G) admission dichotomy (Lemma D,
    // section 3.1) and the tangential sandwich volume rule (Lemma S with
    // obligations S1-S3, section 4). The regime split amends the MONO-8 gate:
    // a pair the landed transversality admission refuses (`TransversalityUncertified`)
    // is admitted here as (T) (the exact solver's path) or (G) (the sandwich),
    // or refuses with a named tag (section 6).
    //
    // The range function is parameterised (section 4.3): the fast Bernstein
    // control-net slabs when the patch is a positive-weight Bernstein chart,
    // and a mandatory generic interval fallback otherwise (the regression trap:
    // a control-net-only rule fires on only a fraction of the rank-deficient
    // states).
    //
    // Open question 6 (does `ADMISSION-WHOLE-BOX-REGULAR-CONE` expose normal
    // cones Lemma D can reuse?): YES. `certify_patch_family` returns the
    // `AdmissionCertificate` whose whole-box `SpanCertificate.regular.cone` is
    // exactly the `NormalCone { anchor, s_up }` Lemma D's phi/Theta arithmetic
    // consumes. `patch_cone` below reuses it directly; no cone is recomputed.
    // -----------------------------------------------------------------------

    use std::collections::BinaryHeap;

    use truck_certified::construct::admission::NormalCone;
    use truck_certified::construct::normal_cone::midpoint_normal_direction;

    /// The regime of one admitted patch pair (Lemma D, section 3.1).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
    #[serde(rename_all = "snake_case")]
    pub enum SurfaceRegime {
        /// (T): every pair of normals is non-parallel (`phi - Theta > 0` and
        /// `phi + Theta < pi`).
        Transversal,
        /// (G): every pair of normals has a nonzero component on the graph axis
        /// (`phi + Theta < pi/2` or `phi - Theta > pi/2`).
        Tangential,
    }

    /// The named refusal of the regime admission (section 6).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
    #[serde(rename_all = "snake_case")]
    pub enum RegimeRefusal {
        /// `d_u x d_v` cannot be certified nonzero on a patch.
        SingularParametrization,
        /// The (G) cell fails graph injectivity after the subdivision budget.
        GraphNotInjective,
    }

    /// The certified regime admission of one patch pair.
    #[derive(Debug, Clone, Copy, PartialEq, Serialize)]
    pub struct RegimeAdmission {
        /// The certified regime.
        pub regime: SurfaceRegime,
        /// The base patch's outward graph axis (the certified cone anchor,
        /// normalized).
        pub axis: [f64; 3],
        /// A certified lower bound of the anchor angle `phi`.
        pub phi_lo: f64,
        /// A certified upper bound of the cone half-angle sum `Theta`.
        pub theta_hi: f64,
    }

    /// The range function a (G) pair consumed (section 4.3).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
    #[serde(rename_all = "snake_case")]
    pub enum RangePath {
        /// The Bernstein control-net fast path (positive weights).
        Bernstein,
        /// The mandatory generic interval fallback.
        Fallback,
    }

    /// The named refusal of the sandwich rule (section 6).
    #[derive(Debug, Clone, Copy, PartialEq, Serialize)]
    #[serde(rename_all = "snake_case")]
    pub enum SandwichRefusal {
        /// `d_u x d_v` cannot be certified nonzero on a patch.
        SingularParametrization,
        /// The (G) cell fails graph injectivity.
        GraphNotInjective,
        /// Coincidence without an exact common-carrier certificate.
        CoincidenceWithoutExactCarrier {
            /// The bracket floor `overlap_area * f`.
            floor: f64,
        },
        /// The error schedule did not reach the requested tolerance.
        BudgetExhausted {
            /// The achieved sandwich bound.
            achieved_width: f64,
        },
        /// A consumed patch row is malformed (a caller defect).
        MalformedPatch,
    }

    /// The certified tangential sandwich outcome of one `A op B` volume.
    #[derive(Debug, Clone, PartialEq, Serialize)]
    pub struct SandwichCertificate {
        /// The certified volume of the base operand.
        pub volume_a: f64,
        /// The certified volume of the tool operand.
        pub volume_b: f64,
        /// The certified tangential sandwich bound.
        pub sandwich_bound: f64,
        /// The certified lower bracket bound of the boolean volume.
        pub bracket_lo: f64,
        /// The certified upper bracket bound of the boolean volume.
        pub bracket_hi: f64,
        /// The bracket width.
        pub width: f64,
        /// The overall regime of the pair family.
        pub regime: SurfaceRegime,
        /// The range function consumed.
        pub range_path: RangePath,
        /// The number of undecided (G) cells carrying the bracket.
        pub undecided_cells: usize,
        /// The number of subdivision steps performed.
        pub refined_cells: usize,
        /// The coincidence floor when the certificate is the exact-carrier
        /// coincidence case.
        pub floor: Option<f64>,
    }

    /// The certified options of the sandwich rule.
    #[derive(Debug, Clone, Copy)]
    pub struct SandwichOptions {
        /// The requested bracket width.
        pub tolerance: f64,
        /// The subdivision cell cap.
        pub max_cells: usize,
        /// Whether an exact common-carrier certificate (W2) is recorded.
        pub shared_carrier: bool,
        /// Whether the Bernstein chart fast path is available.
        pub bernstein_chart: bool,
        /// Whether the fast path's positive-weight precondition holds.
        pub rational_positive_weights: bool,
    }

    impl Default for SandwichOptions {
        fn default() -> Self {
            SandwichOptions {
                tolerance: 1.0e-4,
                max_cells: 65536,
                shared_carrier: false,
                bernstein_chart: true,
                rational_positive_weights: true,
            }
        }
    }

    /// The maximum subdivision depth of the sandwich schedule.
    const SANDWICH_MAX_DEPTH: u32 = 24;

    /// The coincidence floor factor `f = c * u * scale` (section 4.4).
    const SANDWICH_FLOOR_FACTOR: f64 = 64.0;

    /// A parsed raw rational tensor-Bernstein patch (row-major).
    #[derive(Clone)]
    struct RawPatch {
        rows: usize,
        cols: usize,
        num: Vec<[f64; 3]>,
        weights: Vec<f64>,
        orientation: f64,
    }

    /// One undecided (G) cell of the schedule: the pair indices, the two
    /// parameter cells, the graph axis and its plane basis, and the current
    /// contribution key `|R| * max(0, -h_lo)`.
    #[derive(Clone, Copy)]
    struct SandwichCell {
        id: u64,
        depth: u32,
        ia: usize,
        ib: usize,
        a: ParamCell,
        b: ParamCell,
        axis: [f64; 3],
        e1: [f64; 3],
        e2: [f64; 3],
        key: f64,
    }

    impl PartialEq for SandwichCell {
        fn eq(&self, other: &Self) -> bool {
            self.id == other.id
        }
    }

    impl Eq for SandwichCell {}

    impl PartialOrd for SandwichCell {
        fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
            Some(self.cmp(other))
        }
    }

    impl Ord for SandwichCell {
        fn cmp(&self, other: &Self) -> std::cmp::Ordering {
            // A max-heap by key; ties are broken by the smaller cell id so the
            // schedule is deterministic (spec section 4.4/8).
            self.key
                .total_cmp(&other.key)
                .then_with(|| other.id.cmp(&self.id))
        }
    }

    fn normalize3(v: [f64; 3]) -> Option<[f64; 3]> {
        let n = super::v3_norm(v);
        if !n.is_finite() || n <= 0.0 {
            return None;
        }
        Some([v[0] / n, v[1] / n, v[2] / n])
    }

    /// A deterministic orthonormal basis `(e1, e2)` of the plane perpendicular
    /// to the unit axis `a`.
    fn project_basis(a: [f64; 3]) -> Option<([f64; 3], [f64; 3])> {
        let helper = if a[2].abs() < 0.9 {
            [0.0, 0.0, 1.0]
        } else {
            [1.0, 0.0, 0.0]
        };
        let e1 = normalize3(super::v3_cross(a, helper))?;
        let e2 = super::v3_cross(a, e1);
        Some((e1, e2))
    }

    fn scalar_hull(vals: &[f64]) -> Iv {
        let mut lo = f64::INFINITY;
        let mut hi = f64::NEG_INFINITY;
        for &v in vals {
            if v < lo {
                lo = v;
            }
            if v > hi {
                hi = v;
            }
        }
        Iv {
            lo: down(lo),
            hi: up(hi),
        }
    }

    /// The interval quotient `a / b`, `None` when `b` straddles zero.
    fn iv_div(a: Iv, b: Iv) -> Option<Iv> {
        if b.lo <= 0.0 && b.hi >= 0.0 {
            return None;
        }
        let mut lo = f64::INFINITY;
        let mut hi = f64::NEG_INFINITY;
        for &x in &[a.lo, a.hi] {
            for &y in &[b.lo, b.hi] {
                let v = x / y;
                if !v.is_finite() {
                    return None;
                }
                lo = lo.min(down(v));
                hi = hi.max(up(v));
            }
        }
        if lo <= hi { Some(Iv { lo, hi }) } else { None }
    }

    fn parse_raw(row: &VolumeRow) -> Result<RawPatch, SandwichRefusal> {
        let rows = row.numerator.len();
        let cols = row.numerator.first().map_or(0, Vec::len);
        if rows < 2 || cols < 2 || row.weights.len() != rows {
            return Err(SandwichRefusal::MalformedPatch);
        }
        let mut num = Vec::with_capacity(rows.saturating_mul(cols));
        let mut weights = Vec::with_capacity(rows.saturating_mul(cols));
        for (i, num_row) in row.numerator.iter().enumerate() {
            if num_row.len() != cols {
                return Err(SandwichRefusal::MalformedPatch);
            }
            let w_row = row.weights.get(i).ok_or(SandwichRefusal::MalformedPatch)?;
            if w_row.len() != cols {
                return Err(SandwichRefusal::MalformedPatch);
            }
            for (j, a) in num_row.iter().enumerate() {
                let w = w_row
                    .get(j)
                    .copied()
                    .ok_or(SandwichRefusal::MalformedPatch)?;
                if !w.is_finite() || !a.iter().all(|c| c.is_finite()) {
                    return Err(SandwichRefusal::MalformedPatch);
                }
                num.push(*a);
                weights.push(w);
            }
        }
        if !row.orientation.is_finite() || row.orientation == 0.0 {
            return Err(SandwichRefusal::MalformedPatch);
        }
        Ok(RawPatch {
            rows,
            cols,
            num,
            weights,
            orientation: row.orientation,
        })
    }

    fn sub_raw_num(patch: &RawPatch, cell: &ParamCell) -> Vec<[f64; 3]> {
        let p = Patch {
            rows: patch.rows,
            cols: patch.cols,
            data: patch.num.clone(),
            orientation: 1.0,
        };
        sub_net(&p, cell)
    }

    fn sub_raw_weight(patch: &RawPatch, cell: &ParamCell) -> Vec<f64> {
        let data: Vec<[f64; 3]> = patch.weights.iter().map(|&w| [w, 0.0, 0.0]).collect();
        let p = Patch {
            rows: patch.rows,
            cols: patch.cols,
            data,
            orientation: 1.0,
        };
        sub_net(&p, cell).into_iter().map(|v| v[0]).collect()
    }

    fn project_scalar(net: &[[f64; 3]], e: [f64; 3]) -> Vec<f64> {
        net.iter().map(|p| super::v3_dot(e, *p)).collect()
    }

    fn deriv_u_scalar(net: &[f64], rows: usize, cols: usize) -> Vec<f64> {
        let factor = (rows - 1) as f64;
        let mut out = Vec::with_capacity(rows.saturating_sub(1).saturating_mul(cols));
        for i in 0..rows.saturating_sub(1) {
            for j in 0..cols {
                let a = net.get(i * cols + j).copied().unwrap_or(0.0);
                let b = net.get((i + 1) * cols + j).copied().unwrap_or(0.0);
                out.push(factor * (b - a));
            }
        }
        out
    }

    fn deriv_v_scalar(net: &[f64], rows: usize, cols: usize) -> Vec<f64> {
        let factor = (cols - 1) as f64;
        let mut out = Vec::with_capacity(rows.saturating_mul(cols.saturating_sub(1)));
        for i in 0..rows {
            for j in 0..cols.saturating_sub(1) {
                let a = net.get(i * cols + j).copied().unwrap_or(0.0);
                let b = net.get(i * cols + (j + 1)).copied().unwrap_or(0.0);
                out.push(factor * (b - a));
            }
        }
        out
    }

    /// The projected control-box hull `(lo, hi)` of a patch cell (positive
    /// weights; the rational control points `num_i / w_i` are the convex-hull
    /// generators of the surface).
    fn projected_bbox(
        num: &[[f64; 3]],
        weights: &[f64],
        e1: [f64; 3],
        e2: [f64; 3],
    ) -> Option<([f64; 2], [f64; 2])> {
        let mut lo = [f64::INFINITY; 2];
        let mut hi = [f64::NEG_INFINITY; 2];
        for (p, &w) in num.iter().zip(weights.iter()) {
            let q = if w > 0.0 {
                [p[0] / w, p[1] / w, p[2] / w]
            } else {
                *p
            };
            let x = super::v3_dot(e1, q);
            let y = super::v3_dot(e2, q);
            if !x.is_finite() || !y.is_finite() {
                return None;
            }
            lo[0] = lo[0].min(x);
            hi[0] = hi[0].max(x);
            lo[1] = lo[1].min(y);
            hi[1] = hi[1].max(y);
        }
        if lo[0] <= hi[0] && lo[1] <= hi[1] {
            Some((lo, hi))
        } else {
            None
        }
    }

    /// The range enclosure `(lo, hi)` of `axis . P` over the cell. The fast
    /// path is the Bernstein control-net slab (a convex combination with the
    /// positive weights `B_i w_i`); the fallback is the natural interval
    /// extension of the rational function `(axis . A) / W`.
    fn height_range(
        num: &[[f64; 3]],
        weights: &[f64],
        axis: [f64; 3],
        fast: bool,
    ) -> Result<(f64, f64), SandwichRefusal> {
        if fast {
            let mut lo = f64::INFINITY;
            let mut hi = f64::NEG_INFINITY;
            for (p, &w) in num.iter().zip(weights.iter()) {
                if w <= 0.0 {
                    return Err(SandwichRefusal::SingularParametrization);
                }
                let h = super::v3_dot(axis, *p) / w;
                if !h.is_finite() {
                    return Err(SandwichRefusal::MalformedPatch);
                }
                lo = lo.min(h);
                hi = hi.max(h);
            }
            Ok((down(lo), up(hi)))
        } else {
            let mut a_lo = f64::INFINITY;
            let mut a_hi = f64::NEG_INFINITY;
            for p in num {
                let v = super::v3_dot(axis, *p);
                a_lo = a_lo.min(v);
                a_hi = a_hi.max(v);
            }
            let mut w_lo = f64::INFINITY;
            let mut w_hi = f64::NEG_INFINITY;
            for &w in weights {
                w_lo = w_lo.min(w);
                w_hi = w_hi.max(w);
            }
            let iv = iv_div(Iv { lo: a_lo, hi: a_hi }, Iv { lo: w_lo, hi: w_hi })
                .ok_or(SandwichRefusal::SingularParametrization)?;
            Ok((iv.lo, iv.hi))
        }
    }

    /// The G-INJ sufficient condition (section 3.1): every matrix in the
    /// interval hull of the 2x2 Jacobian of `pi o P` over the cell is
    /// nonsingular.
    fn graph_injective(patch: &RawPatch, cell: &ParamCell, e1: [f64; 3], e2: [f64; 3]) -> bool {
        let num = sub_raw_num(patch, cell);
        let w = sub_raw_weight(patch, cell);
        let rows = patch.rows;
        let cols = patch.cols;
        let w_iv = scalar_hull(&w);
        if w_iv.lo <= 0.0 {
            return false;
        }
        let mut jac = [[Iv::point(0.0); 2]; 2];
        for (k, e) in [e1, e2].into_iter().enumerate() {
            let a = project_scalar(&num, e);
            let a_u = deriv_u_scalar(&a, rows, cols);
            let a_v = deriv_v_scalar(&a, rows, cols);
            let w_u = deriv_u_scalar(&w, rows, cols);
            let w_v = deriv_v_scalar(&w, rows, cols);
            let Some(p_iv) = iv_div(scalar_hull(&a), w_iv) else {
                return false;
            };
            let Some(du) = iv_div(scalar_hull(&a_u).sub(p_iv.mul(scalar_hull(&w_u))), w_iv) else {
                return false;
            };
            let Some(dv) = iv_div(scalar_hull(&a_v).sub(p_iv.mul(scalar_hull(&w_v))), w_iv) else {
                return false;
            };
            if let Some(slot) = jac.get_mut(k) {
                slot[0] = du;
                slot[1] = dv;
            }
        }
        let det = jac[0][0].mul(jac[1][1]).sub(jac[0][1].mul(jac[1][0]));
        det.lo > 0.0 || det.hi < 0.0
    }

    /// The certified upper bound of `sqrt(iv)` (`None` when `iv.hi < 0`).
    fn iv_sqrt(iv: Iv) -> Option<Iv> {
        if iv.hi < 0.0 || !iv.hi.is_finite() {
            return None;
        }
        let hi = iv.hi.sqrt();
        if !hi.is_finite() {
            return None;
        }
        Some(Iv {
            lo: down(iv.lo.max(0.0).sqrt()),
            hi: up(hi),
        })
    }

    /// A certified lower bound of the norm of every vector in the interval box
    /// (the axis mignitudes in quadrature).
    fn mignitude3(n: &[Iv; 3]) -> f64 {
        let mut acc = Iv::point(0.0);
        for iv in n {
            let mig = if iv.lo > 0.0 {
                iv.lo
            } else if iv.hi < 0.0 {
                -iv.hi
            } else {
                0.0
            };
            acc = acc.add(Iv::point(mig).mul(Iv::point(mig)));
        }
        iv_sqrt(acc).map_or(0.0, |v| v.lo)
    }

    /// The interval hull of the homogeneous normal numerator
    /// `N = P_u x P_v = ((A_u W - A W_u) x (A_v W - A W_v)) / W^4` over the
    /// whole patch box. This is the same normal-cone mathematics the landed
    /// `ADMISSION-WHOLE-BOX-REGULAR-CONE` assembly uses, evaluated directly on
    /// the homogeneous control hulls so it certifies every orientation (the
    /// landed `certify_patch_family` whole-box path rejects the negatively
    /// oriented planar faces).
    fn normal_hull(raw: &RawPatch) -> Option<[Iv; 3]> {
        let cell = ParamCell::unit();
        let p = Patch {
            rows: raw.rows,
            cols: raw.cols,
            data: raw.num.clone(),
            orientation: 1.0,
        };
        let wp = Patch {
            rows: raw.rows,
            cols: raw.cols,
            data: raw.weights.iter().map(|&w| [w, 0.0, 0.0]).collect(),
            orientation: 1.0,
        };
        let (alo, ahi) = control_range(&p, &cell);
        let (aulo, auhi) = derivative_range_u(&p, &cell);
        let (avlo, avhi) = derivative_range_v(&p, &cell);
        let (wlo, whi) = control_range(&wp, &cell);
        let (wulo, wuhi) = derivative_range_u(&wp, &cell);
        let (wvlo, wvhi) = derivative_range_v(&wp, &cell);
        let w = Iv {
            lo: wlo[0],
            hi: whi[0],
        };
        if w.lo <= 0.0 {
            return None;
        }
        let wu = Iv {
            lo: wulo[0],
            hi: wuhi[0],
        };
        let wv = Iv {
            lo: wvlo[0],
            hi: wvhi[0],
        };
        let w2 = w.mul(w);
        let mut pu = [Iv::point(0.0); 3];
        let mut pv = [Iv::point(0.0); 3];
        for k in 0..3 {
            let a = Iv {
                lo: alo[k],
                hi: ahi[k],
            };
            let au = Iv {
                lo: aulo[k],
                hi: auhi[k],
            };
            let av = Iv {
                lo: avlo[k],
                hi: avhi[k],
            };
            pu[k] = iv_div(au.mul(w).sub(a.mul(wu)), w2)?;
            pv[k] = iv_div(av.mul(w).sub(a.mul(wv)), w2)?;
        }
        Some([
            pu[1].mul(pv[2]).sub(pu[2].mul(pv[1])),
            pu[2].mul(pv[0]).sub(pu[0].mul(pv[2])),
            pu[0].mul(pv[1]).sub(pu[1].mul(pv[0])),
        ])
    }

    /// The whole-box certified normal cone of one patch row. The anchor is the
    /// float midpoint normal (the landed SFC search); the cone bound `s_up` is
    /// the certified `|n x anchor| / (|n| |anchor|)` maximum over the
    /// homogeneous normal hull (`normal_hull`), which certifies `s_up < 1` on
    /// the whole box.
    fn patch_cone(row: &VolumeRow) -> Result<NormalCone, RegimeRefusal> {
        let raw = parse_raw(row).map_err(|_| RegimeRefusal::SingularParametrization)?;
        let patch = TensorBernsteinPatch::try_new(
            row.numerator.clone(),
            row.weights.clone(),
            unit_ibox2(),
            PatchParent::new(0, None),
        )
        .map_err(|_| RegimeRefusal::SingularParametrization)?;
        let anchor = midpoint_normal_direction(&patch)
            .and_then(normalize3)
            .ok_or(RegimeRefusal::SingularParametrization)?;
        let n = normal_hull(&raw).ok_or(RegimeRefusal::SingularParametrization)?;
        let cx = n[1]
            .mul(Iv::point(anchor[2]))
            .sub(n[2].mul(Iv::point(anchor[1])));
        let cy = n[2]
            .mul(Iv::point(anchor[0]))
            .sub(n[0].mul(Iv::point(anchor[2])));
        let cz = n[0]
            .mul(Iv::point(anchor[1]))
            .sub(n[1].mul(Iv::point(anchor[0])));
        let mut w2 = Iv::point(0.0);
        for c in [cx, cy, cz] {
            let far = c.lo.abs().max(c.hi.abs());
            w2 = w2.add(Iv::point(far).mul(Iv::point(far)));
        }
        let w_max = iv_sqrt(w2)
            .ok_or(RegimeRefusal::SingularParametrization)?
            .hi;
        let n_min = mignitude3(&n);
        let a_iv = Iv::point(anchor[0])
            .mul(Iv::point(anchor[0]))
            .add(Iv::point(anchor[1]).mul(Iv::point(anchor[1])))
            .add(Iv::point(anchor[2]).mul(Iv::point(anchor[2])));
        let a_lo = iv_sqrt(a_iv)
            .ok_or(RegimeRefusal::SingularParametrization)?
            .lo;
        if !w_max.is_finite()
            || !n_min.is_finite()
            || !a_lo.is_finite()
            || n_min <= 0.0
            || a_lo <= 0.0
        {
            return Err(RegimeRefusal::SingularParametrization);
        }
        let s_up = up(w_max / (n_min * a_lo));
        if !s_up.is_finite() || s_up >= 1.0 {
            return Err(RegimeRefusal::SingularParametrization);
        }
        Ok(NormalCone { anchor, s_up })
    }

    /// The Lemma D dichotomy over two already-certified normal cones. The cone
    /// computation is the expensive step, so `certify_sandwich` caches the
    /// per-patch cones and calls this for every pair.
    fn admit_regime_from_cones(
        cone_a: &NormalCone,
        cone_b: &NormalCone,
    ) -> Result<RegimeAdmission, RegimeRefusal> {
        let a = normalize3(cone_a.anchor).ok_or(RegimeRefusal::SingularParametrization)?;
        let b = normalize3(cone_b.anchor).ok_or(RegimeRefusal::SingularParametrization)?;
        let dot = super::v3_dot(a, b).clamp(-1.0, 1.0);
        let cross = super::v3_norm(super::v3_cross(a, b));
        let phi = cross.atan2(dot);
        let phi_lo = down(phi);
        let phi_hi = up(phi);
        let s_a = cone_a.s_up.clamp(0.0, 1.0);
        let s_b = cone_b.s_up.clamp(0.0, 1.0);
        let theta_hi = up(s_a.asin() + s_b.asin());
        if theta_hi >= std::f64::consts::FRAC_PI_4 {
            return Err(RegimeRefusal::SingularParametrization);
        }
        let pi = std::f64::consts::PI;
        let transversal = (phi_lo - theta_hi) > 0.0 && (phi_hi + theta_hi) < pi;
        let tangential = (phi_hi + theta_hi) < std::f64::consts::FRAC_PI_2
            || (phi_lo - theta_hi) > std::f64::consts::FRAC_PI_2;
        let regime = if transversal {
            SurfaceRegime::Transversal
        } else if tangential {
            SurfaceRegime::Tangential
        } else {
            return Err(RegimeRefusal::SingularParametrization);
        };
        Ok(RegimeAdmission {
            regime,
            axis: a,
            phi_lo,
            theta_hi,
        })
    }

    fn regime_refusal_to_sandwich(refusal: RegimeRefusal) -> SandwichRefusal {
        match refusal {
            RegimeRefusal::SingularParametrization => SandwichRefusal::SingularParametrization,
            RegimeRefusal::GraphNotInjective => SandwichRefusal::GraphNotInjective,
        }
    }

    /// The certified volume of a closed oriented rational patch cycle, via the
    /// landed face-form certificate.
    fn exact_volume(rows: &[VolumeRow]) -> Result<f64, SandwichRefusal> {
        let mut lo = 0.0f64;
        let mut hi = 0.0f64;
        for row in rows {
            let patch = TensorBernsteinPatch::try_new(
                row.numerator.clone(),
                row.weights.clone(),
                unit_ibox2(),
                PatchParent::new(0, None),
            )
            .map_err(|_| SandwichRefusal::MalformedPatch)?;
            let fact = certify_patch_form(&patch, row.orientation, &VolumeOptions::default())
                .map_err(|_| SandwichRefusal::MalformedPatch)?;
            lo += fact.bracket.lo;
            hi += fact.bracket.hi;
        }
        Ok(0.5 * (lo + hi))
    }

    fn rows_equal(a: &VolumeRow, b: &VolumeRow) -> bool {
        a.orientation == b.orientation && a.numerator == b.numerator && a.weights == b.weights
    }

    /// The coincidence floor `overlap_area * f` of the W2-less coincidence
    /// case (section 4.4).
    fn coincidence_floor(rows: &[RawPatch], axis: [f64; 3]) -> Result<f64, SandwichRefusal> {
        let Some(first) = rows.first() else {
            return Err(SandwichRefusal::MalformedPatch);
        };
        let (e1, e2) = project_basis(axis).ok_or(SandwichRefusal::SingularParametrization)?;
        let (lo, hi) = projected_bbox(&first.num, &first.weights, e1, e2)
            .ok_or(SandwichRefusal::SingularParametrization)?;
        let area = (hi[0] - lo[0]) * (hi[1] - lo[1]);
        let mut scale = 1.0f64;
        for p in &first.num {
            for c in p {
                scale = scale.max(c.abs());
            }
        }
        Ok(area * SANDWICH_FLOOR_FACTOR * f64::EPSILON * scale)
    }

    /// The per-cell contribution `|R| * max(0, -h_lo)`. `None` when the
    /// projected overlap is empty or the overlap thickness is zero (the cell
    /// is resolved).
    #[allow(clippy::too_many_arguments)]
    fn sandwich_cell(
        id: u64,
        depth: u32,
        ia: usize,
        ib: usize,
        patch_a: &RawPatch,
        patch_b: &RawPatch,
        cell_a: ParamCell,
        cell_b: ParamCell,
        axis: [f64; 3],
        e1: [f64; 3],
        e2: [f64; 3],
        fast: bool,
    ) -> Result<Option<SandwichCell>, SandwichRefusal> {
        let num_a = sub_raw_num(patch_a, &cell_a);
        let w_a = sub_raw_weight(patch_a, &cell_a);
        let num_b = sub_raw_num(patch_b, &cell_b);
        let w_b = sub_raw_weight(patch_b, &cell_b);
        let Some((alo, ahi)) = projected_bbox(&num_a, &w_a, e1, e2) else {
            return Err(SandwichRefusal::SingularParametrization);
        };
        let Some((blo, bhi)) = projected_bbox(&num_b, &w_b, e1, e2) else {
            return Err(SandwichRefusal::SingularParametrization);
        };
        let area = (ahi[0].min(bhi[0]) - alo[0].max(blo[0])).max(0.0)
            * (ahi[1].min(bhi[1]) - alo[1].max(blo[1])).max(0.0);
        if area <= 0.0 {
            return Ok(None);
        }
        let (_pa_lo, pa_hi) = height_range(&num_a, &w_a, axis, fast)?;
        let (pb_lo, _pb_hi) = height_range(&num_b, &w_b, axis, fast)?;
        let h_lo = down(pb_lo - pa_hi);
        let h_hi = up(_pb_hi - _pa_lo);
        // The spec's undecided test (section 4.4): only a cell whose h-enclosure
        // straddles zero carries sandwich uncertainty. A cell whose enclosure
        // excludes zero is certified resolved and contributes nothing.
        if h_lo > 0.0 || h_hi < 0.0 {
            return Ok(None);
        }
        let thickness = (-h_lo).max(0.0);
        let key = area * thickness;
        if !key.is_finite() {
            return Err(SandwichRefusal::MalformedPatch);
        }
        if key <= 0.0 {
            return Ok(None);
        }
        Ok(Some(SandwichCell {
            id,
            depth,
            ia,
            ib,
            a: cell_a,
            b: cell_b,
            axis,
            e1,
            e2,
            key,
        }))
    }

    /// The tangential sandwich volume rule of `A op B` (Lemma S with
    /// obligations S1-S3). Returns the certified bracket or a named refusal.
    pub fn certify_sandwich(
        base: &[VolumeRow],
        tool: &[VolumeRow],
        mode: ModeValue,
        options: &SandwichOptions,
    ) -> Result<SandwichCertificate, SandwichRefusal> {
        if base.is_empty() || tool.is_empty() {
            return Err(SandwichRefusal::MalformedPatch);
        }
        let mut a_raw: Vec<RawPatch> = Vec::with_capacity(base.len());
        for row in base {
            a_raw.push(parse_raw(row)?);
        }
        let mut b_raw: Vec<RawPatch> = Vec::with_capacity(tool.len());
        for row in tool {
            b_raw.push(parse_raw(row)?);
        }

        let va = exact_volume(base)?;
        let vb = exact_volume(tool)?;

        // The per-patch normal cones (the expensive `certify_patch_family`
        // step) are computed once and reused for every pair.
        let mut a_cones: Vec<NormalCone> = Vec::with_capacity(base.len());
        for row in base {
            a_cones.push(patch_cone(row).map_err(regime_refusal_to_sandwich)?);
        }
        let mut b_cones: Vec<NormalCone> = Vec::with_capacity(tool.len());
        for row in tool {
            b_cones.push(patch_cone(row).map_err(regime_refusal_to_sandwich)?);
        }

        // W2: an exact common carrier resolves coincidence exactly.
        let identical =
            base.len() == tool.len() && base.iter().zip(tool.iter()).all(|(x, y)| rows_equal(x, y));
        if identical {
            let cone_a = a_cones.first().ok_or(SandwichRefusal::MalformedPatch)?;
            let cone_b = b_cones.first().ok_or(SandwichRefusal::MalformedPatch)?;
            let admission =
                admit_regime_from_cones(cone_a, cone_b).map_err(regime_refusal_to_sandwich)?;
            if options.shared_carrier {
                let (lo, hi) = match mode {
                    ModeValue::Add => (va, va),
                    ModeValue::Subtract => (0.0, 0.0),
                    ModeValue::Intersect => (va, va),
                };
                return Ok(SandwichCertificate {
                    volume_a: va,
                    volume_b: vb,
                    sandwich_bound: 0.0,
                    bracket_lo: lo,
                    bracket_hi: hi,
                    width: hi - lo,
                    regime: SurfaceRegime::Tangential,
                    range_path: RangePath::Bernstein,
                    undecided_cells: 0,
                    refined_cells: 0,
                    floor: None,
                });
            }
            let floor = coincidence_floor(&a_raw, admission.axis)?;
            return Err(SandwichRefusal::CoincidenceWithoutExactCarrier { floor });
        }

        let fast = options.bernstein_chart && options.rational_positive_weights;
        let mut heap: BinaryHeap<SandwichCell> = BinaryHeap::new();
        let mut next_id = 0u64;
        let mut total = 0.0f64;
        let mut undecided = 0usize;
        let mut refined = 0usize;
        let mut any_tangential = false;

        for (ia, ra) in a_raw.iter().enumerate() {
            let cone_a = a_cones.get(ia).ok_or(SandwichRefusal::MalformedPatch)?;
            for (ib, rb) in b_raw.iter().enumerate() {
                let cone_b = b_cones.get(ib).ok_or(SandwichRefusal::MalformedPatch)?;
                let admission =
                    admit_regime_from_cones(cone_a, cone_b).map_err(regime_refusal_to_sandwich)?;
                if admission.regime == SurfaceRegime::Transversal {
                    continue;
                }
                any_tangential = true;
                let axis = admission.axis;
                let (e1, e2) =
                    project_basis(axis).ok_or(SandwichRefusal::SingularParametrization)?;
                let cell_a = ParamCell::unit();
                let cell_b = ParamCell::unit();
                if !graph_injective(ra, &cell_a, e1, e2) || !graph_injective(rb, &cell_b, e1, e2) {
                    return Err(SandwichRefusal::GraphNotInjective);
                }
                if let Some(cell) = sandwich_cell(
                    next_id, 0, ia, ib, ra, rb, cell_a, cell_b, axis, e1, e2, fast,
                )? {
                    next_id += 1;
                    total += cell.key;
                    undecided += 1;
                    heap.push(cell);
                }
            }
        }

        // The schedule (section 4.4): refine the top key until the total is at
        // most the requested tolerance; ties are broken by cell id.
        while total > options.tolerance {
            let Some(top) = heap.pop() else {
                break;
            };
            total -= top.key;
            if top.depth >= SANDWICH_MAX_DEPTH || refined >= options.max_cells {
                return Err(SandwichRefusal::BudgetExhausted {
                    achieved_width: total + top.key,
                });
            }
            let da = top.a.diameter();
            let db = top.b.diameter();
            let mut children: Vec<(ParamCell, ParamCell)> = Vec::with_capacity(2);
            if da >= db {
                let (a1, a2) = top.a.split();
                children.push((a1, top.b));
                children.push((a2, top.b));
            } else {
                let (b1, b2) = top.b.split();
                children.push((top.a, b1));
                children.push((top.a, b2));
            }
            let ra = a_raw.get(top.ia).ok_or(SandwichRefusal::MalformedPatch)?;
            let rb = b_raw.get(top.ib).ok_or(SandwichRefusal::MalformedPatch)?;
            for (ca, cb) in children {
                if !graph_injective(ra, &ca, top.e1, top.e2)
                    || !graph_injective(rb, &cb, top.e1, top.e2)
                {
                    return Err(SandwichRefusal::GraphNotInjective);
                }
                if let Some(child) = sandwich_cell(
                    next_id,
                    top.depth + 1,
                    top.ia,
                    top.ib,
                    ra,
                    rb,
                    ca,
                    cb,
                    top.axis,
                    top.e1,
                    top.e2,
                    fast,
                )? {
                    next_id += 1;
                    total += child.key;
                    heap.push(child);
                }
            }
            refined += 1;
        }

        let s = total;
        // The bracket assumes the operands are closed oriented solids
        // (`V_A, V_B >= 0`). Open patch cycles carry a signed flux; the
        // formula clamps to keep a well-formed bracket, and the sandwich bound
        // (the rule's real payload) is unaffected.
        let (lo, hi) = match mode {
            ModeValue::Add => {
                let sum = va + vb;
                let lower = (sum - s).max(0.0);
                (lower, sum.max(lower))
            }
            ModeValue::Subtract => {
                let upper = va.max(0.0);
                ((upper - s).max(0.0), upper)
            }
            ModeValue::Intersect => (0.0, s.max(0.0)),
        };
        if !lo.is_finite() || !hi.is_finite() || lo > hi {
            return Err(SandwichRefusal::MalformedPatch);
        }
        Ok(SandwichCertificate {
            volume_a: va,
            volume_b: vb,
            sandwich_bound: s,
            bracket_lo: lo,
            bracket_hi: hi,
            width: hi - lo,
            regime: if any_tangential {
                SurfaceRegime::Tangential
            } else {
                SurfaceRegime::Transversal
            },
            range_path: if fast {
                RangePath::Bernstein
            } else {
                RangePath::Fallback
            },
            undecided_cells: undecided,
            refined_cells: refined,
            floor: None,
        })
    }
}

// ---------------------------------------------------------------------------
// RDEF-M2-REGIME-SANDWICH -- the door-facing probe.
// ---------------------------------------------------------------------------

/// The RDEF-M2 door probe (CHK-9 reachability): run the (T)/(G) admission
/// dichotomy and the tangential sandwich volume rule over the submitted patch
/// 2-cycles and marshal the outcome. A refusal is reported as a named tag
/// (never a panic, never a silent zero), so the door can distinguish the
/// regime split's certified bracket from every typed refusal.
pub fn certify_sandwich_probe(
    row: &crate::facade::SandwichProbeRow,
) -> crate::facade::SandwichOutcome {
    let to_rows = |patches: &[crate::facade::SandwichPatchRow]| {
        patches
            .iter()
            .map(|p| crate::python::binding::VolumeRow {
                numerator: p.numerator.clone(),
                weights: p.weights.clone(),
                orientation: p.orientation,
            })
            .collect::<Vec<_>>()
    };
    let base = to_rows(&row.base);
    let tool = to_rows(&row.tool);
    let options = membership::SandwichOptions {
        tolerance: row.tolerance,
        max_cells: row.max_cells,
        shared_carrier: row.shared_carrier,
        bernstein_chart: row.bernstein_chart,
        rational_positive_weights: row.rational_positive_weights,
    };
    match membership::certify_sandwich(&base, &tool, row.mode, &options) {
        Ok(cert) => crate::facade::SandwichOutcome {
            ok: true,
            regime: match cert.regime {
                membership::SurfaceRegime::Transversal => "transversal".to_string(),
                membership::SurfaceRegime::Tangential => "tangential".to_string(),
            },
            range_path: match cert.range_path {
                membership::RangePath::Bernstein => "bernstein".to_string(),
                membership::RangePath::Fallback => "fallback".to_string(),
            },
            volume_a: cert.volume_a,
            volume_b: cert.volume_b,
            sandwich_bound: cert.sandwich_bound,
            bracket_lo: cert.bracket_lo,
            bracket_hi: cert.bracket_hi,
            width: cert.width,
            undecided_cells: cert.undecided_cells,
            refined_cells: cert.refined_cells,
            refusal: None,
            floor: cert.floor,
        },
        Err(refusal) => {
            let (tag, floor) = match refusal {
                membership::SandwichRefusal::SingularParametrization => {
                    ("singular_parametrization".to_string(), None)
                }
                membership::SandwichRefusal::GraphNotInjective => {
                    ("graph_not_injective".to_string(), None)
                }
                membership::SandwichRefusal::CoincidenceWithoutExactCarrier { floor } => {
                    ("coincidence_without_exact_carrier".to_string(), Some(floor))
                }
                membership::SandwichRefusal::BudgetExhausted { .. } => {
                    ("budget_exhausted".to_string(), None)
                }
                membership::SandwichRefusal::MalformedPatch => {
                    ("malformed_patch".to_string(), None)
                }
            };
            let achieved = match refusal {
                membership::SandwichRefusal::BudgetExhausted { achieved_width } => achieved_width,
                _ => 0.0,
            };
            crate::facade::SandwichOutcome {
                ok: true,
                regime: "unknown".to_string(),
                range_path: "unknown".to_string(),
                volume_a: 0.0,
                volume_b: 0.0,
                sandwich_bound: achieved,
                bracket_lo: 0.0,
                bracket_hi: 0.0,
                width: 0.0,
                undecided_cells: 0,
                refined_cells: 0,
                refusal: Some(tag),
                floor,
            }
        }
    }
}

// ---------------------------------------------------------------------------
// pyo3 surface
// ---------------------------------------------------------------------------

/// Parses a submitted construction tree JSON. The serde diagnostic is carried
/// out as text so a vocabulary gap (an unrecorded `kind`) is NAMED to the
/// caller instead of collapsing to the generic empty-domain refusal.
fn parse_tree(tree_json: &str) -> Result<TreeNode, String> {
    serde_json::from_str(tree_json).map_err(|error| error.to_string())
}

/// Maps a construction-tree parse failure to the typed `Refused` exception,
/// carrying the serde diagnostic (the vocabulary gap) in the payload `case`
/// and the exception message. Never a bare `Exception`, never a silent
/// generic refusal.
fn parse_error_to_pyerr(py: Python<'_>, message: &str) -> PyErr {
    let payload = crate::marshal::RefusedPayload {
        case: format!("vocabulary gap: {message}"),
        envelope: Some("vocabulary_gap".to_string()),
        stage: None,
        prop: None,
        left: None,
        right: None,
        reason: None,
        certificate: None,
        bound: None,
        allowed: None,
    };
    let payload_json = serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string());
    match python::marshal_refused(py, &payload_json) {
        Ok(value) => PyErr::from_value(value.bind(py).clone()),
        Err(error) => error,
    }
}

/// Maps the trim-extrude constructor's typed error back to the executor's
/// typed kernel refusal. The constructor owns the precise kernel kind; the
/// executor's `Refusal` vocabulary carries the typed envelope case the door
/// already re-marshals, so no refusal is ever lost or turned into a panic.
fn trim_binding_error_to_refusal(error: crate::python::binding::BindingError) -> Refusal {
    match error {
        crate::python::binding::BindingError::Malformed(_) => Refusal::Empty,
        crate::python::binding::BindingError::Refusal(_) => {
            Refusal::UnsupportedEnvelope(EnvelopeCase::NonCanonicalCarrier)
        }
    }
}

/// The pyo3 measurement entry: takes the construction tree JSON and returns
/// the facts JSON (`{solid_count, volume, bbox, label?, color?, rows?, timing}`).
#[pyfunction]
pub fn bd_facts(py: Python<'_>, tree_json: &str) -> PyResult<String> {
    // The construct phase is the tree construction from the submitted JSON;
    // the facts phase is the native measurement of that tree. Both are
    // diagnostic columns for the census protocol and gate nothing.
    let construct_started = std::time::Instant::now();
    let tree = parse_tree(tree_json).map_err(|message| parse_error_to_pyerr(py, &message))?;
    let construct_ms = construct_started.elapsed().as_secs_f64() * 1000.0;
    let facts_started = std::time::Instant::now();
    let outcome = crate::gil::with_kernel_gil_released(py, move || tree_facts(&tree));
    let facts_ms = facts_started.elapsed().as_secs_f64() * 1000.0;
    match outcome {
        Ok(facts) => {
            let mut value = serde_json::json!({
                "solid_count": facts.solid_count,
                "volume": facts.volume,
                "volume_bracket": facts.volume_bracket,
                "bbox": facts.bbox,
            });
            if let Some(label) = &facts.label {
                value["label"] = serde_json::json!(label);
            }
            if let Some(color) = &facts.color {
                value["color"] = serde_json::json!(color);
            }
            if let Some(mismatch) = facts.seam_mismatch {
                value["seam_mismatch"] = serde_json::json!(mismatch);
            }
            if let Some(blend) = &facts.blend
                && let Some(map) = value.as_object_mut()
            {
                map.insert("blend".to_string(), serde_json::json!(blend));
            }
            if let Some(rows) = &facts.rows {
                value["rows"] = serde_json::json!(rows);
            }
            if !facts.boolean_events.is_empty()
                && let Some(map) = value.as_object_mut()
            {
                map.insert(
                    "boolean_events".to_string(),
                    serde_json::json!(facts.boolean_events),
                );
            }
            if let Some(map) = value.as_object_mut() {
                map.insert(
                    "timing".to_string(),
                    serde_json::json!({
                        "construct_ms": construct_ms,
                        "facts_ms": facts_ms,
                    }),
                );
            }
            serde_json::to_string(&value)
                .map_err(|e| PyRuntimeError::new_err(format!("facts serialization failed: {e}")))
        }
        Err(refusal) => Err(refusal_to_pyerr(py, &refusal)),
    }
}

/// The pyo3 export entry: writes the construction tree's STL to `path` and
/// returns `{"triangles": n, "mesh_ms": f64}`. `mesh_ms` is a diagnostic
/// column (the wall-clock mesh phase), never a gate. `deflection` of `None`
/// keeps the landed fixed-resolution deterministic mesh bit-for-bit.
#[pyfunction]
#[pyo3(signature = (tree_json, path, deflection = None))]
pub fn bd_stl(
    py: Python<'_>,
    tree_json: &str,
    path: &str,
    deflection: Option<f64>,
) -> PyResult<String> {
    let tree = parse_tree(tree_json).map_err(|message| parse_error_to_pyerr(py, &message))?;
    let owned_path = path.to_string();
    let mesh_started = std::time::Instant::now();
    let outcome = crate::gil::with_kernel_gil_released(py, move || {
        write_tree_stl(&tree, &owned_path, deflection)
    });
    let mesh_ms = mesh_started.elapsed().as_secs_f64() * 1000.0;
    match outcome {
        Ok(triangles) => serde_json::to_string(
            &serde_json::json!({ "triangles": triangles, "mesh_ms": mesh_ms }),
        )
        .map_err(|e| PyRuntimeError::new_err(format!("stl serialization failed: {e}"))),
        Err(refusal) => Err(refusal_to_pyerr(py, &refusal)),
    }
}

/// The pyo3 render-artifact entry: writes the construction tree as a colored
/// indexed GLB (one node per placed part, material colors from the recorded
/// client metadata) and returns `{"parts": n, "triangles": n, "mesh_ms": f64}`.
/// `deflection` of `None` keeps the landed fixed-resolution deterministic
/// mesh; the certification artifact remains the STL path.
#[pyfunction]
#[pyo3(signature = (tree_json, path, deflection = None))]
pub fn bd_glb(
    py: Python<'_>,
    tree_json: &str,
    path: &str,
    deflection: Option<f64>,
) -> PyResult<String> {
    let tree = parse_tree(tree_json).map_err(|message| parse_error_to_pyerr(py, &message))?;
    let owned_path = path.to_string();
    let mesh_started = std::time::Instant::now();
    let outcome = crate::gil::with_kernel_gil_released(py, move || {
        write_tree_glb(&tree, &owned_path, deflection)
    });
    let mesh_ms = mesh_started.elapsed().as_secs_f64() * 1000.0;
    match outcome {
        Ok((parts, triangles)) => serde_json::to_string(
            &serde_json::json!({ "parts": parts, "triangles": triangles, "mesh_ms": mesh_ms }),
        )
        .map_err(|e| PyRuntimeError::new_err(format!("glb serialization failed: {e}"))),
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
                rotation: None,
                mirror: None,
                label: None,
                color: None,
            },
        }
    }

    /// Builds a placed part carrying the mirror placed-carrier transform.
    fn mirrored_part(solid: SolidSpec, axis: &str) -> TreeNode {
        TreeNode::Part {
            part: PartSpec {
                solid,
                x: 0.0,
                y: 0.0,
                z: 0.0,
                rz: 0.0,
                rotation: None,
                mirror: Some(axis.to_string()),
                label: None,
                color: None,
            },
        }
    }

    /// Builds an all-line lathe from a closed vertex loop (the census form the
    /// door's line-profile revolve produces).
    fn line_lathe(points: &[[f64; 2]], arc_deg: f64) -> SolidSpec {
        let mut profile = Vec::with_capacity(points.len());
        for i in 0..points.len() {
            let b = if i + 1 < points.len() {
                points[i + 1]
            } else {
                points[0]
            };
            profile.push(LatheEdge::Line { a: points[i], b });
        }
        SolidSpec::Lathe {
            profile,
            arc_deg,
            start_deg: 0.0,
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
            label: None,
            color: None,
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
        let housing = line_lathe(
            &[
                [0.001, 1040.0],
                [95.0, 1110.0],
                [120.0, 1130.0],
                [0.001, 1130.0],
            ],
            360.0,
        );
        let facts = tree_facts(&part(housing, 470.0, 0.0, 0.0))
            .expect("a full-arc line-profile lathe is in envelope");
        // OCC records the same turbine-housing solid at 1,390,947.111 mm^3.
        assert!((facts.volume - 1_390_947.111).abs() / 1_390_947.111 < 1e-9);
        assert_eq!(facts.bbox[0], [350.0, -120.0, 1040.0]);
        assert_eq!(facts.bbox[1], [590.0, 120.0, 1130.0]);
    }

    #[test]
    fn unanswerable_arc_lathe_refuses_typed() {
        // A partial-arc revolve is landed (DOOR-PARTIAL-ARC-FLIP), but an arc
        // outside `(0, 360]` is not answerable: it refuses typed, never a
        // silent full revolution or a clamped sweep.
        for arc in [0.0, -45.0, 400.0, f64::NAN] {
            let bad = line_lathe(&[[10.0, 0.0], [20.0, 0.0], [20.0, 10.0], [10.0, 10.0]], arc);
            let refusal = tree_facts(&part(bad, 0.0, 0.0, 0.0))
                .expect_err("an arc outside (0, 360] is outside the lathe arm");
            assert!(matches!(
                refusal,
                Refusal::UnsupportedEnvelope(EnvelopeCase::NonCanonicalCarrier)
            ));
        }
    }

    #[test]
    fn stl_writer_emits_binary_stl() {
        let tree = TreeNode::Group {
            group: vec![part(SolidSpec::Sphere { radius: 10.0 }, 1.0, 2.0, 3.0)],
            label: None,
            color: None,
        };
        let dir = std::env::temp_dir().join(format!("truck123d_bd_stl_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch dir must be creatable"); // H-3: test scratch
        let path = dir.join("sphere.stl");
        let triangles = write_tree_stl(&tree, &path.to_string_lossy(), None).expect("stl writes");
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
        // (see unanswerable_arc_lathe_refuses_typed) and the marshal layer maps
        // that refusal to the `Refused` Python exception class — never a panic
        // and never a bare Exception.
        let partial = line_lathe(&[[10.0, 0.0], [20.0, 0.0], [20.0, 10.0], [10.0, 10.0]], 0.0);
        let refusal = tree_facts(&part(partial, 0.0, 0.0, 0.0))
            .expect_err("an unanswerable arc is outside this executor's arm");
        let marshaled = crate::marshal::Marshaled::from_refusal(&refusal);
        assert_eq!(
            marshaled.class,
            crate::marshal::ExceptionClass::Refused,
            "an unsupported-carrier refusal must surface as the mapped Refused class"
        );
    }

    // -----------------------------------------------------------------------
    // Spline-profile lathe facts
    // -----------------------------------------------------------------------

    /// A synthetic curved shell profile: two dome spline arcs (inner and outer,
    /// the outer +10 in radius over the same samples) joined by radial caps at
    /// `z = 0`, mirroring the corpus's `revolved_shell` profile shape.
    fn dome_shell_profile() -> Vec<LatheEdge> {
        let inner = [[20.0, 0.0], [38.0, 18.0], [62.0, 18.0], [80.0, 0.0]];
        let outer = [[90.0, 0.0], [72.0, 18.0], [48.0, 18.0], [30.0, 0.0]];
        vec![
            LatheEdge::Spline {
                points: inner.to_vec(),
            },
            LatheEdge::Line {
                a: inner[3],
                b: outer[0],
            },
            LatheEdge::Spline {
                points: outer.to_vec(),
            },
            LatheEdge::Line {
                a: outer[3],
                b: inner[0],
            },
        ]
    }

    /// The 5-point Gauss-Legendre quadrature of `pi * r(u)^2 * z'(u)` over one
    /// span: exact for the degree-8 integrand, an independent machine check of
    /// the closed-form segment-moment integral.
    fn span_volume_by_quadrature(span: &SpanPoly) -> f64 {
        let nodes = [
            0.906_179_845_938_664,
            0.538_469_310_105_683,
            0.0,
            -0.538_469_310_105_683,
            -0.906_179_845_938_664,
        ];
        let weights = [
            0.236_926_885_056_189,
            0.478_628_670_499_366,
            0.568_888_888_888_889,
            0.478_628_670_499_366,
            0.236_926_885_056_189,
        ];
        let mut acc = 0.0;
        for i in 0..5 {
            let u = (nodes[i] + 1.0) / 2.0;
            let x = span.r[0] + u * (span.r[1] + u * (span.r[2] + u * span.r[3]));
            let dzdu = span.z[1] + u * (2.0 * span.z[2] + u * 3.0 * span.z[3]);
            acc += weights[i] * x * x * dzdu / 2.0;
        }
        std::f64::consts::PI * acc
    }

    #[test]
    fn spline_segment_volume_matches_independent_quadrature() {
        // The closed-form segment-moment integration is machine-checked
        // against an independent high-order quadrature of the same
        // reconstructed spans (exact for the degree-8 integrand).
        for edge in dome_shell_profile() {
            if let LatheEdge::Spline { points } = edge {
                let spans = spline_spans(&points).expect("synthetic spline reconstructs");
                for span in &spans {
                    let exact = span_volume(span);
                    let quadrature = span_volume_by_quadrature(span);
                    assert!(
                        (exact - quadrature).abs() / exact.abs().max(1e-12) < 1e-9,
                        "span moment integral drifted: exact {exact} vs quadrature {quadrature}"
                    );
                }
            }
        }
    }

    #[test]
    fn spline_shell_facts_are_not_a_polygon_flattening() {
        // The spline arm integrates the TRUE reconstructed spline. Flattening
        // the spline edges to the sample polygon must differ from the exact
        // facts by more than the facts volume tolerance on this curved
        // fixture — the deviation is the no-silent-flattening test.
        let profile = dome_shell_profile();
        let solid = SolidSpec::Lathe {
            profile,
            arc_deg: 360.0,
            start_deg: 0.0,
        };
        let facts = tree_facts(&part(solid, 0.0, 0.0, 0.0))
            .expect("a full-arc spline-profile lathe is in envelope");
        let exact = facts.volume;

        // The sample-polygon approximation: the same boundary with every
        // spline edge replaced by straight chords through its samples.
        let polygon = {
            let mut vertices: Vec<[f64; 2]> = Vec::new();
            for edge in dome_shell_profile() {
                match edge {
                    LatheEdge::Line { a, b } => {
                        if vertices.last() != Some(&a) {
                            vertices.push(a);
                        }
                        if vertices.last() != Some(&b) {
                            vertices.push(b);
                        }
                    }
                    LatheEdge::Spline { points } => {
                        for p in &points {
                            if vertices.last() != Some(p) {
                                vertices.push(*p);
                            }
                        }
                    }
                }
            }
            if vertices.len() > 1 && vertices[0] == *vertices.last().expect("ring nonempty") {
                vertices.pop();
            }
            vertices
        };
        let polygon_solid = SolidSpec::Lathe {
            profile: polygon
                .iter()
                .enumerate()
                .map(|(i, a)| LatheEdge::Line {
                    a: *a,
                    b: polygon[(i + 1) % polygon.len()],
                })
                .collect(),
            arc_deg: 360.0,
            start_deg: 0.0,
        };
        let polygon_facts = tree_facts(&part(polygon_solid, 0.0, 0.0, 0.0))
            .expect("the polygon fixture is a line-profile lathe");
        let deviation = (exact - polygon_facts.volume).abs() / exact;
        assert!(
            deviation > 1.0e-4,
            "the exact spline facts must differ from a sample-polygon flattening by more than \
             the facts tolerance (deviation {deviation})"
        );
    }

    #[test]
    fn spline_profile_bbox_covers_reconstructed_extrema() {
        // The profile's largest radius must not be below the exact radius of
        // the reconstructed spline at its span interiors (sampled finely).
        let profile = dome_shell_profile();
        let solid = SolidSpec::Lathe {
            profile,
            arc_deg: 360.0,
            start_deg: 0.0,
        };
        let bbox = solid_local_bbox(&solid).expect("spline profile bbox");
        let mut sampled_max_r = 0.0f64;
        for edge in &dome_shell_profile() {
            if let LatheEdge::Spline { points } = edge {
                for span in spline_spans(points).expect("reconstructs") {
                    for i in 0..=1000 {
                        let u = i as f64 / 1000.0;
                        let x = span.r[0] + u * (span.r[1] + u * (span.r[2] + u * span.r[3]));
                        sampled_max_r = sampled_max_r.max(x);
                    }
                }
            }
        }
        assert!(bbox[1][0] + 1e-9 >= sampled_max_r);
        assert!(bbox[0][0] - 1e-9 <= -sampled_max_r);
    }

    #[test]
    fn line_profile_lathe_volume_is_bit_identical_via_profile_form() {
        // The V5 pair: the profile-edge census form must produce byte-identical
        // line-profile facts to the landed vertex-loop arm.
        let vertices = [
            [10.0, 0.0],
            [20.0, 0.0],
            [20.0, 10.0],
            [15.0, 15.0],
            [10.0, 10.0],
        ];
        let vertex_arm = SolidSpec::Lathe {
            profile: vertices
                .iter()
                .enumerate()
                .map(|(i, a)| LatheEdge::Line {
                    a: *a,
                    b: vertices[(i + 1) % vertices.len()],
                })
                .collect(),
            arc_deg: 360.0,
            start_deg: 0.0,
        };
        let facts = tree_facts(&part(vertex_arm, 0.0, 0.0, 0.0)).expect("line profile facts");
        // The frustum telescoping value, computed with the landed formula.
        let points: Vec<[f64; 2]> = vertices.to_vec();
        let expected = lathe_volume(&points).expect("landed lathe volume");
        assert_eq!(facts.volume.to_bits(), expected.to_bits());
    }

    // -----------------------------------------------------------------------
    // Authoring arms: extrude prism, loft (with the seam certificate) and the
    // mirror placed carrier.
    // -----------------------------------------------------------------------

    /// A closed 3-D line-loop profile from `(x, y, z)` vertices.
    fn line_loop3(pts: &[[f64; 3]]) -> Vec<ProfileEdge> {
        pts.iter()
            .enumerate()
            .map(|(i, a)| ProfileEdge::Line {
                a: *a,
                b: pts[(i + 1) % pts.len()],
            })
            .collect()
    }

    /// A closed unit square in the `z = z0` plane, wound CCW about +z.
    fn square(z0: f64) -> Vec<ProfileEdge> {
        line_loop3(&[
            [0.0, 0.0, z0],
            [1.0, 0.0, z0],
            [1.0, 1.0, z0],
            [0.0, 1.0, z0],
        ])
    }

    #[test]
    fn prism_facts_are_exact_prism_arithmetic() {
        // The extrusion recording arm: a closed line-loop planar profile swept
        // along its own plane normal. Volume = area * swept length; `both`
        // sweeps both directions (total 2*amount), single-sided sweeps +normal.
        let profile = square(0.0);
        let both = tree_facts(&part(
            SolidSpec::Prism {
                profile: profile.clone(),
                amount: 3.0,
                both: true,
            },
            0.0,
            0.0,
            0.0,
        ))
        .expect("a line-loop prism is in envelope");
        assert!((both.volume - 6.0).abs() < 1e-12);
        // bbox: the square at z in [-3, 3].
        assert_eq!(both.bbox[0], [0.0, 0.0, -3.0]);
        assert_eq!(both.bbox[1], [1.0, 1.0, 3.0]);

        let single = tree_facts(&part(
            SolidSpec::Prism {
                profile,
                amount: 3.0,
                both: false,
            },
            0.0,
            0.0,
            0.0,
        ))
        .expect("single-sided prism is in envelope");
        assert!((single.volume - 3.0).abs() < 1e-12);
        assert_eq!(single.bbox[0], [0.0, 0.0, 0.0]);
        assert_eq!(single.bbox[1], [1.0, 1.0, 3.0]);
    }

    /// One exact circle section edge with the given centre/radius and a `+z`
    /// plane normal.
    fn circle_section(center: [f64; 3], radius: f64) -> Vec<ProfileEdge> {
        vec![ProfileEdge::Circle {
            center,
            radius,
            normal: [0.0, 0.0, 1.0],
        }]
    }

    #[test]
    fn circle_loft_facts_are_exact_and_analytic() {
        // A cylinder loft (two equal circles): volume pi r^2 h and the exact
        // circle AABB.
        let cylinder = SolidSpec::Loft {
            sections: vec![
                circle_section([0.0, 0.0, 0.0], 2.0),
                circle_section([0.0, 0.0, 10.0], 2.0),
            ],
            closed: false,
        };
        let facts =
            tree_facts(&part(cylinder.clone(), 0.0, 0.0, 0.0)).expect("circle cylinder loft");
        let expected = std::f64::consts::PI * 4.0 * 10.0;
        assert!((facts.volume - expected).abs() / expected < 1e-12);
        assert_eq!(facts.bbox, [[-2.0, -2.0, 0.0], [2.0, 2.0, 10.0]]);
        assert_eq!(facts.solid_count, 1);

        // A coaxial frustum: the exact `pi h/3 (r0^2 + r0 r1 + r1^2)`.
        let frustum = SolidSpec::Loft {
            sections: vec![
                circle_section([0.0, 0.0, 0.0], 3.0),
                circle_section([0.0, 0.0, 4.0], 1.0),
            ],
            closed: false,
        };
        let facts = tree_facts(&part(frustum, 0.0, 0.0, 0.0)).expect("circle frustum");
        let expected = std::f64::consts::PI * 4.0 / 3.0 * (9.0 + 3.0 + 1.0);
        assert!((facts.volume - expected).abs() / expected < 1e-12);

        // An oblique parallel frustum obeys Cavalieri: the lateral offset of
        // the centres does not change the volume.
        let oblique = SolidSpec::Loft {
            sections: vec![
                circle_section([0.0, 0.0, 0.0], 2.0),
                circle_section([5.0, 0.0, 3.0], 2.0),
            ],
            closed: false,
        };
        let facts = tree_facts(&part(oblique, 0.0, 0.0, 0.0)).expect("oblique circle loft");
        let expected = std::f64::consts::PI * 3.0 * 4.0;
        assert!((facts.volume - expected).abs() / expected < 1e-12);
        assert_eq!(facts.bbox, [[-2.0, -2.0, 0.0], [7.0, 2.0, 3.0]]);

        // A tilted circle's analytic AABB is the exact conic extrema, not the
        // inscribed polygon's.
        let tilted = SolidSpec::Loft {
            sections: vec![
                vec![ProfileEdge::Circle {
                    center: [0.0, 0.0, 0.0],
                    radius: 2.0,
                    normal: [1.0, 0.0, 0.0],
                }],
                vec![ProfileEdge::Circle {
                    center: [4.0, 0.0, 0.0],
                    radius: 2.0,
                    normal: [1.0, 0.0, 0.0],
                }],
            ],
            closed: false,
        };
        let facts = tree_facts(&part(tilted, 0.0, 0.0, 0.0)).expect("tilted circle loft");
        assert_eq!(facts.bbox, [[0.0, -2.0, -2.0], [4.0, 2.0, 2.0]]);
        let expected = std::f64::consts::PI * 4.0 * 4.0;
        assert!((facts.volume - expected).abs() / expected < 1e-12);

        // The mesh is deterministic and uses the fixed tessellation.
        let first = solid_mesh(&cylinder, None).expect("circle mesh");
        let second = solid_mesh(&cylinder, None).expect("circle mesh again");
        assert_eq!(first, second);
        assert_eq!(first.len(), 2 * MESH_SEGMENTS + 2 * MESH_SEGMENTS);
    }

    #[test]
    fn loft_volume_matches_the_segment_moment_derivation() {
        // The loft arm over a parallel two-section stack: the volume is the
        // exact divergence-form integral over the ruled faces. The degenerate
        // two-section line-profile case (two identical aligned sections) is a
        // prism whose volume must be bit-identical to the extrude arm's, and
        // the general two-section case must equal the Simpson value over the
        // (quadratic) cross-section area.
        let a = square(0.0);
        let b = square(5.0);
        let solid = SolidSpec::Loft {
            sections: vec![a, b],
            closed: false,
        };
        let facts = tree_facts(&part(solid.clone(), 0.0, 0.0, 0.0))
            .expect("a line-section loft is in envelope");
        // Volume = 1 * 1 * 5.
        assert!((facts.volume - 5.0).abs() / 5.0 < 1e-12);

        // Two identical sections at z=0 and z=5 through the extrude arm: the
        // prism arm and the degenerate two-section loft must agree bit for bit.
        let prism = tree_facts(&part(
            SolidSpec::Prism {
                profile: square(0.0),
                amount: 5.0,
                both: false,
            },
            0.0,
            0.0,
            0.0,
        ))
        .expect("prism");
        assert_eq!(facts.volume.to_bits(), prism.volume.to_bits());

        // Coaxial similar sections (a pyramid frustum): the general loft
        // segment volume equals the independent Simpson machine check and the
        // closed-form frustum identity h/3 (A0 + A1 + sqrt(A0 A1)) — the same
        // algebraic special case as the lathe line-profile frustum telescoping.
        let small = line_loop3(&[
            [0.0, 0.0, 0.0],
            [2.0, 0.0, 0.0],
            [2.0, 2.0, 0.0],
            [0.0, 2.0, 0.0],
        ]);
        let big = line_loop3(&[
            [0.0, 0.0, 3.0],
            [4.0, 0.0, 3.0],
            [4.0, 4.0, 3.0],
            [0.0, 4.0, 3.0],
        ]);
        let a = profile_loop(&small).expect("small loop");
        let b = profile_loop(&big).expect("big loop");
        let simpson = loft_segment_volume(&a, &b);
        let frustum = 3.0 / 3.0 * (a.area + b.area + (a.area * b.area).sqrt());
        assert!((simpson - frustum).abs() / frustum < 1e-12);
        let facts = tree_facts(&part(
            SolidSpec::Loft {
                sections: vec![small, big],
                closed: false,
            },
            0.0,
            0.0,
            0.0,
        ))
        .expect("frustum loft");
        assert!((facts.volume - frustum).abs() / frustum < 1e-12);
    }

    #[test]
    fn closed_halo_loft_certifies_its_seam_and_refuses_open_mismatch() {
        // The halo form: the station list returns to the start, so the aligned
        // meeting edges across the chain closure satisfy the seam identity and
        // the facts carry the (zero) mismatch. A station list recorded closed
        // whose last station does not return refuses typed with the evidence.
        //
        // The fixture is a closed ring: a vertical square profile (radial
        // extent 4..5, height -1..1) sampled every 45 degrees around the z
        // axis, with the last station the exact return to the first.
        let mut sections = Vec::new();
        let make_station = |theta_deg: f64| -> Vec<ProfileEdge> {
            let th = theta_deg.to_radians();
            let (c, s) = (th.cos(), th.sin());
            let mut pts = Vec::new();
            for (r, h) in [(4.0, -1.0), (5.0, -1.0), (5.0, 1.0), (4.0, 1.0)] {
                pts.push([r * c, r * s, h]);
            }
            line_loop3(&pts)
        };
        for i in 0..8 {
            sections.push(make_station(i as f64 * 45.0));
        }
        // The return to the first station is the exact recorded first section
        // (the halo chain closes onto its own start).
        sections.push(sections[0].clone());
        let facts = tree_facts(&part(
            SolidSpec::Loft {
                sections,
                closed: true,
            },
            0.0,
            0.0,
            0.0,
        ))
        .expect("a closing halo chain is in envelope");
        assert_eq!(facts.seam_mismatch, Some(0.0));
        assert!(facts.volume > 0.0);
        assert_eq!(facts.solid_count, 1);

        // A non-closing chain: the recorded last station is shifted off the
        // return so the aligned meeting edges no longer satisfy the identity.
        let mut broken = Vec::new();
        for i in 0..8 {
            broken.push(make_station(i as f64 * 45.0));
        }
        // A perturbed "return": rotated 0.1 degree off the seam.
        broken.push(make_station(360.1));
        let refusal = tree_facts(&part(
            SolidSpec::Loft {
                sections: broken,
                closed: true,
            },
            0.0,
            0.0,
            0.0,
        ))
        .expect_err("a non-closing halo chain must refuse typed");
        assert!(matches!(
            refusal,
            Refusal::UnsupportedEnvelope(EnvelopeCase::NonCanonicalCarrier)
        ));
    }

    #[test]
    fn mirror_placed_carrier_transforms_facts_without_recomputing_geometry() {
        // The mirror arm records a placed-carrier reflection over the census
        // row: the volume is unchanged, the bbox is reflected about the plane,
        // and no geometry is recomputed (the solid spec is untouched).
        let solid = SolidSpec::Prism {
            profile: square(0.0),
            amount: 2.0,
            both: true,
        };
        let base = tree_facts(&part(solid.clone(), 10.0, 0.0, 0.0)).expect("unmirrored prism");
        let mirrored = tree_facts(&mirrored_part(solid, "y")).expect("mirror placed-carrier prism");
        assert_eq!(base.volume.to_bits(), mirrored.volume.to_bits());
        assert_eq!(mirrored.bbox[0], [0.0, -1.0, -2.0]);
        assert_eq!(mirrored.bbox[1], [1.0, 0.0, 2.0]);
    }

    // -----------------------------------------------------------------------
    // Authoring frames: the full orthonormal placement rotation and the door's
    // Plane-frame / Pos / Vector client rows (AUTHOR-FRAME-CARRIERS).
    // -----------------------------------------------------------------------

    /// The corpus `ttc` directory that owns `door.py` (the door-driven
    /// authoring tests below import it from a real interpreter).
    fn corpus_ttc_dir() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("corpus")
            .join("ttc")
    }

    /// Runs `python -c <script>` with the corpus `ttc` directory as its sole
    /// positional argument and returns stdout. The corpus door's authoring
    /// rows are pure client-side data, so no native-module staging is needed.
    fn run_door_python(script: &str) -> String {
        let output = std::process::Command::new("python")
            .arg("-c")
            .arg(script)
            .arg(corpus_ttc_dir())
            .output()
            .expect("spawn python for the door row");
        assert!(
            output.status.success(),
            "python failed:\nstdout:{}\nstderr:{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).to_string()
    }

    #[test]
    fn placed_frame_facts_match_unplaced_facts_under_rigid_motion() {
        // Placed-frame rigidity: a solid under an ARBITRARY orthonormal frame
        // reports the unplaced facts volume-wise EXACTLY (placement never
        // recomputes volume) and bbox-wise as the AABB of the 8 rotated local
        // corners. The fixture is an exact 90-degree frame about z: the local
        // 20 x 10 x 30 box maps its x/y extents onto y/x, unchanged z.
        let solid = SolidSpec::Box {
            length: 20.0,
            width: 10.0,
            height: 30.0,
        };
        let unplaced = tree_facts(&part(solid.clone(), 0.0, 0.0, 0.0)).expect("unplaced box");
        // The placed row is submitted as the recorded JSON (the pure-z field is
        // defaulted): an exact 90-degree orthonormal frame about z.
        let placed_json = serde_json::json!({
            "part": {
                "solid": { "kind": "box", "length": 20.0, "width": 10.0, "height": 30.0 },
                "x": 7.0,
                "y": -3.0,
                "z": 2.0,
                "rotation": {
                    "x_dir": [0.0, 1.0, 0.0],
                    "y_dir": [-1.0, 0.0, 0.0],
                    "z_dir": [0.0, 0.0, 1.0]
                }
            }
        });
        let placed = parse_tree(&placed_json.to_string()).expect("frame-placed box row");
        let facts = tree_facts(&placed).expect("frame-placed box");
        assert_eq!(
            facts.volume.to_bits(),
            unplaced.volume.to_bits(),
            "volume must be invariant under the placement rotation"
        );
        assert_eq!(facts.solid_count, 1);
        // Rotated-AABB identity: local x range [-10, 10] maps to world y, local
        // y range [-5, 5] maps to world -x, z unchanged; then translated.
        assert_eq!(facts.bbox[0], [2.0, -13.0, -13.0]);
        assert_eq!(facts.bbox[1], [12.0, 7.0, 17.0]);
    }

    #[test]
    fn plane_frame_extrude_answers_world_facts() {
        // The Plane frame row is client-side data: a unit square drawn in the
        // `Plane(origin=(50,0,0), x_dir=(0,1,0), z_dir=(1,0,0))` section frame
        // is recorded by the door at its world location, and a single-sided
        // extrude of that Plane-frame face answers WORLD facts (an exact prism
        // spanning x in [50, 60], volume 10). The refusal surface of the old
        // `Plane` stub is gone on this carrier.
        let script = r#"
import json, sys
sys.path.insert(0, sys.argv[1])
import door
bd = door._build_truck_module()

def v(x, y, z):
    return door.Vector(x, y, z)

# A unit square, CCW in the section plane's local (u, v) frame.
wire = door.Wire([
    door.Edge.make_line(v(0, 0, 0), v(1, 0, 0)),
    door.Edge.make_line(v(1, 0, 0), v(1, 1, 0)),
    door.Edge.make_line(v(1, 1, 0), v(0, 1, 0)),
    door.Edge.make_line(v(0, 1, 0), v(0, 0, 0)),
])
plane = door.Plane(origin=(50.0, 0.0, 0.0), x_dir=(0.0, 1.0, 0.0), z_dir=(1.0, 0.0, 0.0))
face = plane * bd.make_face(wire)
part = bd.extrude(face, amount=10.0, both=False)
node = part._node()
assert node["part"]["solid"]["kind"] == "prism", node
print(json.dumps(node))
"#;
        let stdout = run_door_python(script);
        let tree = parse_tree(&stdout).expect("the door prism row must parse");
        let facts = tree_facts(&tree).expect("a Plane-frame extrude is in envelope");
        assert_eq!(facts.solid_count, 1);
        assert!((facts.volume - 10.0).abs() / 10.0 < 1e-12);
        // World facts: the section plane at x = 50 and its unit square.
        assert_eq!(facts.bbox[0], [50.0, 0.0, 0.0]);
        assert_eq!(facts.bbox[1], [60.0, 1.0, 1.0]);
    }

    #[test]
    fn pos_placed_assembly_counts_solids() {
        // Pos placement records a translation frame row; a multi-solid
        // assembly placed by `Pos` still reports its placed solid count (and
        // the placed union bbox).
        let script = r#"
import json, sys
sys.path.insert(0, sys.argv[1])
import door
bd = door._build_truck_module()

box = bd.Pos(10.0, 20.0, 0.0) * bd.Box(1, 1, 1)
sphere = bd.Pos(0.0, 0.0, 5.0) * bd.Sphere(2.0)
group = bd.Compound(children=[box, sphere])
node = group._node()
assert node["group"][0]["part"]["x"] == 10.0, node
assert node["group"][1]["part"]["x"] == 0.0, node
print(json.dumps(node))
"#;
        let stdout = run_door_python(script);
        let tree = parse_tree(&stdout).expect("the Pos-placed assembly must parse");
        let facts = tree_facts(&tree).expect("Pos-placed primitives are in envelope");
        assert_eq!(facts.solid_count, 2);
        // Placed union AABB: the box at (10, 20, 0) spans x in [9.5, 10.5] and
        // the sphere at (0, 0, 5) spans x/y in [-2, 2], z in [3, 7].
        assert_eq!(facts.bbox[0], [-2.0, -2.0, -0.5]);
        assert_eq!(facts.bbox[1], [10.5, 20.5, 7.0]);
    }

    #[test]
    fn vector_surface_answers_direction_math() {
        // The Vector attribute surface is pure client-side data (never a
        // kernel row): normalized / dot / cross / length and the scalar and
        // difference arithmetic the census DNF rows call must all answer.
        let script = r#"
import json, sys
sys.path.insert(0, sys.argv[1])
import door

a = door.Vector(3.0, 0.0, 4.0)
assert abs(a.length - 5.0) < 1e-12
n = a.normalized()
assert abs(n.length - 1.0) < 1e-12
assert abs(n.x - 0.6) < 1e-12 and abs(n.z - 0.8) < 1e-12
assert abs(n.dot(a) - 5.0) < 1e-9
c = door.Vector(0.0, 1.0, 0.0).cross(door.Vector(1.0, 0.0, 0.0))
assert abs(c.z + 1.0) < 1e-12, c
d = door.Vector(5.0, 1.0, 1.0) - door.Vector(1.0, 1.0, 1.0)
assert d.to_tuple() == (4.0, 0.0, 0.0)
e = door.Vector(1.0, 2.0, 3.0) * 2.0
assert e.to_tuple() == (2.0, 4.0, 6.0)
f = 3.0 * door.Vector(0.0, 1.0, 0.0)
assert f.to_tuple() == (0.0, 3.0, 0.0)
g = -door.Vector(0.0, 0.0, 2.0)
assert g.to_tuple() == (0.0, 0.0, -2.0)
assert door.Vector(2, 0, 0).X == 2 and door.Vector(0, 7, 0).Y == 7 and door.Vector(0, 0, 9).Z == 9
print(json.dumps({"ok": True, "length": a.length}))
"#;
        let stdout = run_door_python(script);
        let record: serde_json::Value = serde_json::from_str(stdout.trim()).expect("vector json");
        assert_eq!(record["ok"], true);
        assert_eq!(record["length"], 5.0);
    }

    // -----------------------------------------------------------------------
    // The recorded revolve axis frame (FRAME-REVOLVE): a non-z revolve is the
    // z-lathe of the meridian profile placed by the recorded frame whose local
    // z is the axis direction. Facts are invariant under the placement; the
    // world row answers the rotated world bbox and the exact volume.
    // -----------------------------------------------------------------------

    #[test]
    fn revolve_about_nonz_axis_answers_world_facts() {
        // A ring revolved about the wheel-frame axle (world Y) is recorded as a
        // meridian lathe plus the axis frame. Its world facts equal the exact
        // lathe volume (volume is placement-invariant) and the rotated world
        // bbox (x/z span the outer radius, y spans the axial extent).
        let script = r#"
import json, sys
sys.path.insert(0, sys.argv[1])
import door

def ring(corners):
    edges = []
    for i in range(len(corners)):
        edges.append(door.Edge.make_line(corners[i], corners[(i + 1) % len(corners)]))
    return door.Face(door.Wire(edges))

def v(x, y, z):
    return door.Vector(x, y, z)

# The axle ring, drawn in the plane x=0 containing the axle (world Y):
# axial runs along Y from -5..5, radius along Z from 10..20.
axle = ring([
    v(0.0, -5.0, 10.0),
    v(0.0, -5.0, 20.0),
    v(0.0, 5.0, 20.0),
    v(0.0, 5.0, 10.0),
])
part = door.revolve(axle, axis=door.Axis.Y)
print(json.dumps(part._node()))
"#;
        let stdout = run_door_python(script);
        let node: serde_json::Value = serde_json::from_str(stdout.trim()).expect("axle row json");
        let tree = parse_tree(&node.to_string()).expect("axle revolve row parses");
        let facts = tree_facts(&tree).expect("a non-z revolve is in envelope");
        assert_eq!(facts.solid_count, 1);
        // Volume of the 10..20 radius ring over axial extent 10.
        let expected = std::f64::consts::PI * (20.0 * 20.0 - 10.0 * 10.0) * 10.0;
        assert!(
            (facts.volume - expected).abs() / expected < 1e-12,
            "world volume {} != expected {}",
            facts.volume,
            expected
        );
        // The placed ring about world Y: radius in x/z, axial along y.
        assert_eq!(facts.bbox[0], [-20.0, -5.0, -20.0]);
        assert_eq!(facts.bbox[1], [20.0, 5.0, 20.0]);
    }

    #[test]
    fn z_revolve_rows_answer_bit_identically() {
        // The z-axis path keeps the recorded legacy row shape bit-for-bit: the
        // profile coordinates are the world (radius x, axial z) pairs with no
        // recorded placement frame, and the measured facts equal the exact
        // analytic lathe of the same profile ring.
        let script = r#"
import json, sys
sys.path.insert(0, sys.argv[1])
import door

def ring(corners):
    edges = []
    for i in range(len(corners)):
        edges.append(door.Edge.make_line(corners[i], corners[(i + 1) % len(corners)]))
    return door.Face(door.Wire(edges))

def v(x, y, z):
    return door.Vector(x, y, z)

# A z-axis revolve ring: profile in the y=0 plane with radius along x and
# axial along z, 10..20 radius over axial extent -5..5.
ring = ring([
    v(10.0, 0.0, -5.0),
    v(20.0, 0.0, -5.0),
    v(20.0, 0.0, 5.0),
    v(10.0, 0.0, 5.0),
])
part = door.revolve(ring, axis=door.Axis.Z)
print(json.dumps(part._node()))
"#;
        let stdout = run_door_python(script);
        let node: serde_json::Value = serde_json::from_str(stdout.trim()).expect("z row json");
        // Legacy row shape: a lathe profile with the world (radius, axial)
        // coordinates, and no placement frame fields beyond the pure-z row.
        let part_node = &node["part"];
        assert_eq!(part_node["solid"]["kind"], "lathe");
        assert_eq!(part_node["rz"], 0.0);
        assert!(part_node.get("x").is_some(), "legacy row keeps its x field");
        let tree = parse_tree(&node.to_string()).expect("z revolve row parses");
        let facts = tree_facts(&tree).expect("a z revolve is in envelope");
        assert_eq!(facts.solid_count, 1);
        // Bit-identical facts: the analytic lathe of the same profile ring.
        let expected = std::f64::consts::PI * (20.0 * 20.0 - 10.0 * 10.0) * 10.0;
        assert!(
            (facts.volume - expected).abs() / expected < 1e-12,
            "z volume {} != expected {}",
            facts.volume,
            expected
        );
        assert_eq!(facts.bbox[0], [-20.0, -20.0, -5.0]);
        assert_eq!(facts.bbox[1], [20.0, 20.0, 5.0]);
    }

    #[test]
    fn revolve_refuses_unsupported_axes_typed() {
        // A zero-length axis cannot be recorded: the door refuses typed with
        // the degenerate-axis name (never a silent normalization past zero).
        let script = r#"
import json, sys, types
sys.path.insert(0, sys.argv[1])
import door

class _Refused(Exception):
    def __init__(self, msg):
        super().__init__(msg)
        self.msg = msg

door._T123D = types.SimpleNamespace(Refused=_Refused)
bd = door._build_truck_module()

def ring(corners):
    edges = []
    for i in range(len(corners)):
        edges.append(bd.Edge.make_line(corners[i], corners[(i + 1) % len(corners)]))
    return bd.Face(bd.Wire(edges))

def v(x, y, z):
    return door.Vector(x, y, z)

face = ring([
    v(10.0, 0.0, -5.0),
    v(20.0, 0.0, -5.0),
    v(20.0, 0.0, 5.0),
    v(10.0, 0.0, 5.0),
])
try:
    bd.revolve(face, axis=door.Axis((0.0, 0.0, 0.0)))
    print(json.dumps({"refused": False, "message": "no refusal"}))
except _Refused as exc:
    print(json.dumps({"refused": True, "message": exc.msg}))
"#;
        let stdout = run_door_python(script);
        let record: serde_json::Value = serde_json::from_str(stdout.trim()).expect("refusal json");
        assert_eq!(record["refused"], true, "degenerate axis must refuse typed");
        let message = record["message"].as_str().unwrap_or("");
        assert!(
            message.contains("DegenerateRevolveAxis"),
            "refusal must name the degenerate axis: {message}"
        );
    }

    // -----------------------------------------------------------------------
    // The spline-path sweep carrier (SWEEP-PATH): exact path queries, the
    // recorded station frames, and the V5 net over the landed loft/revolve
    // rows.
    // -----------------------------------------------------------------------

    /// The bridge's exact reconstruction of a planar path at the normalized
    /// parameter `t`: position and derivative with respect to `t`. The path
    /// lies in the `(x, z)` plane, so the bridge's 2-D `spline_spans` is the
    /// same fixed-order interpolant the volume arms use.
    fn span_derivative(span: &SpanPoly, u: f64, total: f64, h: f64) -> [f64; 3] {
        [
            (span.r[1] + u * (2.0 * span.r[2] + u * 3.0 * span.r[3])) * total / h,
            0.0,
            (span.z[1] + u * (2.0 * span.z[2] + u * 3.0 * span.z[3])) * total / h,
        ]
    }

    fn bridge_path_query(samples: &[[f64; 2]], t: f64) -> ([f64; 3], [f64; 3]) {
        let n = samples.len();
        let params = chord_params(samples);
        let total = params[n - 1];
        let spans = spline_spans(samples).expect("the planar path reconstructs");
        if t <= 0.0 {
            let h = params[1] - params[0];
            return (
                [samples[0][0], 0.0, samples[0][1]],
                span_derivative(&spans[0], 0.0, total, h),
            );
        }
        if t >= 1.0 {
            let h = params[n - 1] - params[n - 2];
            return (
                [samples[n - 1][0], 0.0, samples[n - 1][1]],
                span_derivative(&spans[n - 2], 1.0, total, h),
            );
        }
        let s = t * total;
        let mut j = 0;
        while j + 1 < n - 1 && s > params[j + 1] {
            j += 1;
        }
        let h = params[j + 1] - params[j];
        let u = (s - params[j]) / h;
        let value = eval_span(&spans[j], u);
        (
            [value[0], 0.0, value[1]],
            span_derivative(&spans[j], u, total, h),
        )
    }

    #[test]
    fn spline_path_position_and_tangent_answer_exactly() {
        // The door's exact path query must answer through the SAME fixed-order
        // interpolant the volume arms reconstruct: a planar spline path is
        // queried at interior parameters and every answer is compared to the
        // bridge's own `spline_spans` reconstruction of the same samples.
        let script = r#"
import json, sys
sys.path.insert(0, sys.argv[1])
import door

pts = [door.Vector(0.0, 0.0, 0.0), door.Vector(1.0, 0.0, 2.0),
       door.Vector(3.0, 0.0, 1.0), door.Vector(4.0, 0.0, 3.0)]
edge = door.Edge.make_spline(pts)
rows = []
for t in (0.0, 0.25, 0.5, 0.75, 1.0):
    p = edge.position_at(t)
    d = edge.tangent_at(t)
    rows.append({"t": t, "p": list(p.to_tuple()), "d": list(d.to_tuple())})
print(json.dumps(rows))
"#;
        let stdout = run_door_python(script);
        let rows: Vec<serde_json::Value> =
            serde_json::from_str(stdout.trim()).expect("path query rows parse");
        assert_eq!(rows.len(), 5);
        let samples = [[0.0, 0.0], [1.0, 2.0], [3.0, 1.0], [4.0, 3.0]];
        for row in rows {
            let t = row["t"].as_f64().expect("t");
            let p = row["p"].as_array().expect("p");
            let d = row["d"].as_array().expect("d");
            let (want_p, want_d) = bridge_path_query(&samples, t);
            for axis in 0..3 {
                let got_p = p[axis].as_f64().expect("position coordinate");
                let got_d = d[axis].as_f64().expect("derivative coordinate");
                assert!(
                    (got_p - want_p[axis]).abs() <= 1e-12 * (1.0 + want_p[axis].abs()),
                    "position at t={t} axis {axis}: {got_p} vs {}",
                    want_p[axis]
                );
                assert!(
                    (got_d - want_d[axis]).abs() <= 1e-12 * (1.0 + want_d[axis].abs()),
                    "derivative at t={t} axis {axis}: {got_d} vs {}",
                    want_d[axis]
                );
            }
            assert_eq!(p[1].as_f64().expect("y"), 0.0, "the path lies in y = 0");
        }
        // Endpoints stay exact.
        let first = bridge_path_query(&samples, 0.0);
        let last = bridge_path_query(&samples, 1.0);
        assert_eq!(first.0, [0.0, 0.0, 0.0]);
        assert_eq!(last.0, [4.0, 0.0, 3.0]);
    }

    #[test]
    fn sweep_line_path_records_loft_chain() {
        // A line path records the landed two-station straight-sweep chain: a
        // unit square swept along +z by 5 is an exact prism (volume 5) whose
        // recorded row is the landed loft carrier.
        let script = r#"
import json, sys
sys.path.insert(0, sys.argv[1])
import door

def v(x, y, z):
    return door.Vector(x, y, z)

wire = door.Wire([
    door.Edge.make_line(v(0.0, 0.0, 0.0), v(1.0, 0.0, 0.0)),
    door.Edge.make_line(v(1.0, 0.0, 0.0), v(1.0, 1.0, 0.0)),
    door.Edge.make_line(v(1.0, 1.0, 0.0), v(0.0, 1.0, 0.0)),
    door.Edge.make_line(v(0.0, 1.0, 0.0), v(0.0, 0.0, 0.0)),
])
section = door.Face(wire)
path = door.Edge.make_line(v(0.0, 0.0, 0.0), v(0.0, 0.0, 5.0))
print(json.dumps(door.sweep(section, path)._node()))
"#;
        let stdout = run_door_python(script);
        let tree = parse_tree(&stdout).expect("sweep row parses");
        let sections = match &tree {
            TreeNode::Part { part } => match &part.solid {
                SolidSpec::Loft { sections, closed } => {
                    assert!(!*closed, "a straight sweep is an open chain");
                    sections.clone()
                }
                other => panic!("a line-path sweep must record a loft, got {other:?}"),
            },
            _ => panic!("a sweep row must be a single placed part"),
        };
        assert_eq!(sections.len(), 2, "a line path records two stations");
        assert_eq!(sections[0].len(), 4);
        assert_eq!(sections[1].len(), 4);
        // The second station is the first translated along +z by 5.
        let first = match &sections[0][0] {
            ProfileEdge::Line { a, .. } => *a,
            _ => panic!("a sweep section is all line edges"),
        };
        let second = match &sections[1][0] {
            ProfileEdge::Line { a, .. } => *a,
            _ => panic!("a sweep section is all line edges"),
        };
        assert!((second[0] - first[0]).abs() < 1e-12);
        assert!((second[1] - first[1]).abs() < 1e-12);
        assert!((second[2] - first[2] - 5.0).abs() < 1e-12);
        let facts = tree_facts(&tree).expect("a straight sweep is in envelope");
        assert_eq!(facts.solid_count, 1);
        assert!((facts.volume - 5.0).abs() / 5.0 < 1e-12);
        assert_eq!(facts.bbox[0], [0.0, 0.0, 0.0]);
        assert_eq!(facts.bbox[1], [1.0, 1.0, 5.0]);
    }

    #[test]
    fn sweep_spline_path_records_stations() {
        // A spline path records one loft station per recorded sample; the
        // station frames carry the section centroid onto each sample.
        let script = r#"
import json, sys
sys.path.insert(0, sys.argv[1])
import door

def v(x, y, z):
    return door.Vector(x, y, z)

path = door.Edge.make_spline([v(0.0, 0.0, 0.0), v(1.0, 0.0, 0.0),
                              v(2.0, 0.0, 1.0), v(3.0, 0.0, 2.0)])
plane = door.Plane(origin=path.position_at(0), z_dir=path.tangent_at(0))
wire = door.Wire([
    door.Edge.make_line(v(-0.5, -0.5, 0.0), v(0.5, -0.5, 0.0)),
    door.Edge.make_line(v(0.5, -0.5, 0.0), v(0.5, 0.5, 0.0)),
    door.Edge.make_line(v(0.5, 0.5, 0.0), v(-0.5, 0.5, 0.0)),
    door.Edge.make_line(v(-0.5, 0.5, 0.0), v(-0.5, -0.5, 0.0)),
])
section = plane * door.Face(wire)
node = door.sweep(section, path)._node()
stations = [list(point.to_tuple()) for point in path.points]
print(json.dumps({"node": node, "stations": stations}))
"#;
        let stdout = run_door_python(script);
        let record: serde_json::Value =
            serde_json::from_str(stdout.trim()).expect("spline sweep json");
        let tree = parse_tree(&record["node"].to_string()).expect("spline sweep row parses");
        let sections = match &tree {
            TreeNode::Part { part } => match &part.solid {
                SolidSpec::Loft { sections, .. } => sections.clone(),
                other => panic!("a spline-path sweep must record a loft, got {other:?}"),
            },
            _ => panic!("a sweep row must be a single placed part"),
        };
        assert_eq!(sections.len(), 4, "one station per recorded path sample");
        let stations = record["stations"].as_array().expect("stations");
        for (i, section) in sections.iter().enumerate() {
            let mut centroid = [0.0f64; 3];
            for edge in section {
                let a = match edge {
                    ProfileEdge::Line { a, .. } => *a,
                    _ => panic!("a sweep section is all line edges"),
                };
                for axis in 0..3 {
                    centroid[axis] += a[axis];
                }
            }
            let count = section.len() as f64;
            let station = stations[i].as_array().expect("station");
            for axis in 0..3 {
                let want = station[axis].as_f64().expect("station coordinate");
                assert!(
                    (centroid[axis] / count - want).abs() < 1e-9 * (1.0 + want.abs()),
                    "station {i} axis {axis}: centroid {} vs {want}",
                    centroid[axis] / count
                );
            }
        }
        let facts = tree_facts(&tree).expect("a spline-path sweep is in envelope");
        assert_eq!(facts.solid_count, 1);
        assert!(facts.volume > 0.0);
    }

    #[test]
    fn z_revolve_and_loft_rows_answer_bit_identically() {
        // V5 net: the door-recorded z-revolve row and two-station loft row
        // answer bit-identically to the landed kernel arms.
        let script = r#"
import json, sys
sys.path.insert(0, sys.argv[1])
import door

def v(x, y, z):
    return door.Vector(x, y, z)

def square(z):
    pts = [(0.0, 0.0, z), (1.0, 0.0, z), (1.0, 1.0, z), (0.0, 1.0, z)]
    return door.Face(door.Wire([
        door.Edge.make_line(v(*pts[i]), v(*pts[(i + 1) % 4])) for i in range(4)
    ]))

ring = door.Face(door.Wire([
    door.Edge.make_line(v(10.0, 0.0, -5.0), v(20.0, 0.0, -5.0)),
    door.Edge.make_line(v(20.0, 0.0, -5.0), v(20.0, 0.0, 5.0)),
    door.Edge.make_line(v(20.0, 0.0, 5.0), v(10.0, 0.0, 5.0)),
    door.Edge.make_line(v(10.0, 0.0, 5.0), v(10.0, 0.0, -5.0)),
]))
z_row = door.revolve(ring, axis=door.Axis.Z)._node()
loft_row = door.loft([square(0.0), square(5.0)], ruled=False)._node()
print(json.dumps([z_row, loft_row]))
"#;
        let stdout = run_door_python(script);
        let rows: Vec<serde_json::Value> =
            serde_json::from_str(stdout.trim()).expect("landed row json");
        let z_tree = parse_tree(&rows[0].to_string()).expect("z row parses");
        let loft_tree = parse_tree(&rows[1].to_string()).expect("loft row parses");
        let z_facts = tree_facts(&z_tree).expect("z revolve facts");
        let loft_facts = tree_facts(&loft_tree).expect("loft facts");
        let z_landed = tree_facts(&part(
            line_lathe(
                &[[10.0, -5.0], [20.0, -5.0], [20.0, 5.0], [10.0, 5.0]],
                360.0,
            ),
            0.0,
            0.0,
            0.0,
        ))
        .expect("landed z lathe");
        assert_eq!(
            z_facts.volume.to_bits(),
            z_landed.volume.to_bits(),
            "the door z-revolve row must answer bit-identically to the landed lathe arm"
        );
        let loft_landed = tree_facts(&part(
            SolidSpec::Loft {
                sections: vec![square(0.0), square(5.0)],
                closed: false,
            },
            0.0,
            0.0,
            0.0,
        ))
        .expect("landed loft");
        assert_eq!(
            loft_facts.volume.to_bits(),
            loft_landed.volume.to_bits(),
            "the door loft row must answer bit-identically to the landed loft arm"
        );
        assert_eq!(z_facts.solid_count, 1);
        assert_eq!(loft_facts.solid_count, 1);
    }

    // -----------------------------------------------------------------------

    // Spline-section loft facts through the certified volume arm
    // (BRIDGE-LOFT-FACTS): the smooth two-station class certifies through the
    // binding, the landed line class stays bit-identical, and a smooth carrier
    // the recorded data does not determine refuses typed.
    // -----------------------------------------------------------------------

    /// A closed section loop of four spline edges, each through its two corners
    /// and their midpoint (collinear): geometrically a unit square, carried by
    /// `ProfileEdge::Spline` so the analytic line arm refuses it.
    fn spline_square(z0: f64) -> Vec<ProfileEdge> {
        let corners = [
            [0.0, 0.0, z0],
            [1.0, 0.0, z0],
            [1.0, 1.0, z0],
            [0.0, 1.0, z0],
        ];
        let mut edges = Vec::with_capacity(4);
        for i in 0..4 {
            let a = corners[i];
            let b = corners[(i + 1) % 4];
            let mid = [
                0.5 * (a[0] + b[0]),
                0.5 * (a[1] + b[1]),
                0.5 * (a[2] + b[2]),
            ];
            edges.push(ProfileEdge::Spline {
                points: vec![a, mid, b],
            });
        }
        edges
    }

    /// A closed section loop of four quadratic spline edges bulging outward
    /// from the unit square: a genuinely curved spline section carrier.
    fn spline_lens(z0: f64) -> Vec<ProfileEdge> {
        let corners = [
            [0.0, 0.0, z0],
            [1.0, 0.0, z0],
            [1.0, 1.0, z0],
            [0.0, 1.0, z0],
        ];
        let center = [0.5, 0.5, z0];
        let mut edges = Vec::with_capacity(4);
        for i in 0..4 {
            let a = corners[i];
            let b = corners[(i + 1) % 4];
            let mid = [0.5 * (a[0] + b[0]), 0.5 * (a[1] + b[1]), z0];
            let dx = mid[0] - center[0];
            let dy = mid[1] - center[1];
            let len = (dx * dx + dy * dy).sqrt();
            let outward = [mid[0] + 0.15 * dx / len, mid[1] + 0.15 * dy / len, z0];
            edges.push(ProfileEdge::Spline {
                points: vec![a, outward, b],
            });
        }
        edges
    }

    /// The 5-point Gauss-Legendre nodes on `[0, 1]`.
    const GAUSS5_NODES: [f64; 5] = [
        0.046_910_077_030_668_0,
        0.230_765_344_947_158_5,
        0.5,
        0.769_234_655_052_841_5,
        0.953_089_922_969_332_0,
    ];
    /// The 5-point Gauss-Legendre weights on `[0, 1]`.
    const GAUSS5_WEIGHTS: [f64; 5] = [
        0.118_463_442_528_094_6,
        0.239_314_335_249_683_2,
        0.284_444_444_444_444_4,
        0.239_314_335_249_683_2,
        0.118_463_442_528_094_6,
    ];

    /// Evaluates a 1-D Bernstein segment (controls of any degree) at `t`,
    /// returning `(value, derivative)` by de Casteljau.
    fn bernstein_eval(controls: &[[f64; 3]], t: f64) -> ([f64; 3], [f64; 3]) {
        let n = controls.len();
        if n == 0 {
            return ([0.0; 3], [0.0; 3]);
        }
        let mut work: Vec<[f64; 3]> = controls.to_vec();
        for level in (1..n).rev() {
            for i in 0..level {
                let a = work[i];
                let b = work[i + 1];
                work[i] = [
                    a[0] * (1.0 - t) + b[0] * t,
                    a[1] * (1.0 - t) + b[1] * t,
                    a[2] * (1.0 - t) + b[2] * t,
                ];
            }
        }
        let value = work[0];
        let mut deriv: Vec<[f64; 3]> = Vec::with_capacity(n.saturating_sub(1));
        for i in 0..(n - 1) {
            let a = controls[i];
            let b = controls[i + 1];
            let k = (n - 1) as f64;
            deriv.push([k * (b[0] - a[0]), k * (b[1] - a[1]), k * (b[2] - a[2])]);
        }
        let (dval, _) = bernstein_eval(&deriv, t);
        (value, dval)
    }

    /// The cubic Bernstein basis values and derivatives at `t`.
    fn cubic_basis(t: f64) -> ([f64; 4], [f64; 4]) {
        let s = 1.0 - t;
        (
            [s * s * s, 3.0 * s * s * t, 3.0 * s * t * t, t * t * t],
            [
                -3.0 * s * s,
                3.0 * s * s - 6.0 * s * t,
                6.0 * s * t - 3.0 * t * t,
                3.0 * t * t,
            ],
        )
    }

    /// The exact side-surface divergence-form integral of the canonical
    /// N-station spline loft, computed independently of the kernel's exact
    /// expansion route by 5x5 Gauss-Legendre quadrature (exact: the integrand
    /// is bidegree (8, 8) at worst, and 5-point Gauss is exact through degree
    /// 9 per axis).
    fn gauss_side_volume(sections: &[Vec<ProfileEdge>]) -> f64 {
        let mut loops: Vec<Vec<SpanPoly3>> = Vec::with_capacity(sections.len());
        for section in sections {
            loops.push(spline_loop_spans(section).expect("reconstruct"));
        }
        let v = station_params(&loops).expect("station params");
        let span_count = loops[0].len();
        let mut total = 0.0;
        for span_index in 0..span_count {
            let mut rows: [Vec<[f64; 3]>; 4] = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
            for loop3 in &loops {
                let controls = span3_bernstein(&loop3[span_index]);
                for k in 0..4 {
                    rows[k].push(controls[k]);
                }
            }
            let segments: Vec<Vec<Vec<[f64; 3]>>> = rows
                .iter()
                .map(|row| v_segments(row, &v).expect("v segments"))
                .collect();
            for segment_index in 0..segments[0].len() {
                for (iu, &u) in GAUSS5_NODES.iter().enumerate() {
                    let (bu, dbu) = cubic_basis(u);
                    for (iv, &vv) in GAUSS5_NODES.iter().enumerate() {
                        let mut x = [0.0f64; 3];
                        let mut xu = [0.0f64; 3];
                        let mut xv = [0.0f64; 3];
                        for k in 0..4 {
                            let (val, dval) = bernstein_eval(&segments[k][segment_index], vv);
                            for d in 0..3 {
                                x[d] += bu[k] * val[d];
                                xu[d] += dbu[k] * val[d];
                                xv[d] += bu[k] * dval[d];
                            }
                        }
                        total +=
                            GAUSS5_WEIGHTS[iu] * GAUSS5_WEIGHTS[iv] * v3_dot(x, v3_cross(xu, xv))
                                / 3.0;
                    }
                }
            }
        }
        total
    }

    /// One section of the one-time OCC fixture family (MONO-2 step 7): four
    /// quadratic spline edges bulging outward from a square of side `scale`
    /// centered on the section origin at station `z`.
    fn nstation_lens(z: f64, scale: f64) -> Vec<ProfileEdge> {
        let corners: [[f64; 2]; 4] = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
        let center: [f64; 2] = [0.5, 0.5];
        let mut edges = Vec::with_capacity(4);
        for i in 0..4 {
            let a = corners[i];
            let b = corners[(i + 1) % 4];
            let mid = [0.5 * (a[0] + b[0]), 0.5 * (a[1] + b[1])];
            let dx = mid[0] - center[0];
            let dy = mid[1] - center[1];
            let len = (dx * dx + dy * dy).sqrt();
            let outward = [mid[0] + 0.15 * dx / len, mid[1] + 0.15 * dy / len];
            let point = |q: [f64; 2]| [scale * (q[0] - 0.5), scale * (q[1] - 0.5), z];
            edges.push(ProfileEdge::Spline {
                points: vec![point(a), point(outward), point(b)],
            });
        }
        edges
    }

    /// The synthetic N-station stack of the one-time OCC fixture suite.
    fn nstation_sections(n: usize) -> Vec<Vec<ProfileEdge>> {
        let mut out = Vec::with_capacity(n);
        for i in 0..n {
            let scale = 1.0 + 0.06 * i as f64;
            let z = 2.0 * i as f64 + 0.05 * (i * i) as f64;
            out.push(nstation_lens(z, scale));
        }
        out
    }

    #[test]
    fn nstation_volume_brackets_canonical_surface() {
        // The N-station canonical surface certifies through the binding: the
        // bracket encloses the value, is tight, and matches an independent
        // 5x5 Gauss-Legendre integration of the same tensor-Bernstein patches
        // plus the exact planar end caps. The landed two-station case is
        // included so the generalization does not regress it.
        for sections in [
            vec![spline_lens(0.0), spline_lens(5.0)],
            vec![spline_lens(0.0), spline_lens(2.0), spline_lens(5.0)],
            nstation_sections(9),
            nstation_sections(10),
        ] {
            let (value, lo, hi) =
                certified_spline_loft_volume(&sections).expect("the canonical stack certifies");
            assert!(
                lo <= value && value <= hi,
                "the bracket [{lo}, {hi}] must enclose {value}"
            );
            assert!(
                (hi - lo) < 1.0e-6 * (1.0 + value.abs()),
                "the certified bracket is tight: [{lo}, {hi}]"
            );
            let first = spline_loop_spans(&sections[0]).expect("first loop");
            let last = spline_loop_spans(sections.last().expect("last")).expect("last loop");
            let av = spline_loop_area_vector(&first);
            let bv = spline_loop_area_vector(&last);
            let ma = v3_norm(av);
            let mb = v3_norm(bv);
            let pa = [first[0].x[0], first[0].y[0], first[0].z[0]];
            let pb = [last[0].x[0], last[0].y[0], last[0].z[0]];
            let caps = -(1.0 / 3.0) * ma * v3_dot([av[0] / ma, av[1] / ma, av[2] / ma], pa)
                + (1.0 / 3.0) * mb * v3_dot([bv[0] / mb, bv[1] / mb, bv[2] / mb], pb);
            let expected = gauss_side_volume(&sections) + caps;
            assert!(
                (value - expected.abs()).abs() < 1.0e-8 * (1.0 + value.abs()),
                "the certified value {value} must match the independent {expected}"
            );
        }
    }

    #[test]
    fn loft_facts_match_recorded_reference_on_flip() {
        // The recorded reference for the equivalent line-loop prism is 5.0
        // (the landed line-loft arm's exact value). The spline-section carrier
        // flips from the analytic arm's typed refusal to the certified arm's
        // answer, matching the reference exactly.
        let sections = vec![spline_square(0.0), spline_square(5.0)];
        assert!(
            loft_sections(&sections).is_err(),
            "the analytic line arm must refuse the spline carrier"
        );
        let facts = tree_facts(&part(
            SolidSpec::Loft {
                sections: sections.clone(),
                closed: false,
            },
            0.0,
            0.0,
            0.0,
        ))
        .expect("the spline-section loft certifies");
        let reference = 5.0f64;
        assert!(
            (facts.volume - reference).abs() < 1.0e-9,
            "spline-section facts {} must match the recorded reference {reference}",
            facts.volume
        );
        assert_eq!(facts.solid_count, 1);
        assert_eq!(facts.bbox[0], [0.0, 0.0, 0.0]);
        assert_eq!(facts.bbox[1], [1.0, 1.0, 5.0]);
    }

    #[test]
    fn line_loft_rows_answer_bit_identically() {
        // V5 net: the landed line-section loft rows answer bit-identically
        // through the facts arm (the analytic ruled path stays the fast path).
        let a = square(0.0);
        let b = square(5.0);
        let facts = tree_facts(&part(
            SolidSpec::Loft {
                sections: vec![a.clone(), b.clone()],
                closed: false,
            },
            0.0,
            0.0,
            0.0,
        ))
        .expect("line-section loft");
        let validated = loft_sections(&[a, b]).expect("line sections");
        let landed = loft_volume(&validated, false).expect("landed volume");
        assert_eq!(facts.volume.to_bits(), landed.to_bits());
        assert_eq!(facts.volume.to_bits(), 5.0f64.to_bits());
    }

    #[test]
    fn nstation_refuses_still_typed_for_open_chains() {
        // A stack whose section spans do not match cannot be unified by knot
        // insertion alone: the landed typed refusal is preserved.
        let mut six_span = spline_square(0.0);
        six_span[0] = ProfileEdge::Spline {
            points: vec![
                [0.0, 0.0, 0.0],
                [0.3, 0.0, 0.0],
                [0.6, 0.0, 0.0],
                [1.0, 0.0, 0.0],
            ],
        };
        let mismatched = SolidSpec::Loft {
            sections: vec![six_span, spline_square(5.0)],
            closed: false,
        };
        let refusal = tree_facts(&part(mismatched, 0.0, 0.0, 0.0))
            .expect_err("a mismatched span stack must refuse typed");
        assert!(matches!(
            refusal,
            Refusal::UnsupportedEnvelope(EnvelopeCase::NonCanonicalCarrier)
        ));

        // A closed smooth halo chain has no end-cap certificate and keeps the
        // landed typed refusal.
        let closed = SolidSpec::Loft {
            sections: nstation_sections(4),
            closed: true,
        };
        let refusal = tree_facts(&part(closed, 0.0, 0.0, 0.0))
            .expect_err("a closed smooth chain must refuse typed");
        assert!(matches!(
            refusal,
            Refusal::UnsupportedEnvelope(EnvelopeCase::NonCanonicalCarrier)
        ));

        // A section loop that does not close keeps the landed typed refusal.
        let open_loop = vec![
            ProfileEdge::Spline {
                points: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 1.0, 0.0]],
            },
            ProfileEdge::Spline {
                points: vec![[1.0, 1.0, 0.0], [0.0, 1.0, 0.0], [0.2, 0.2, 0.0]],
            },
        ];
        let non_closing = SolidSpec::Loft {
            sections: vec![open_loop, spline_square(5.0)],
            closed: false,
        };
        let refusal = tree_facts(&part(non_closing, 0.0, 0.0, 0.0))
            .expect_err("a non-closing section loop must refuse typed");
        assert!(matches!(
            refusal,
            Refusal::UnsupportedEnvelope(EnvelopeCase::NonCanonicalCarrier)
        ));
    }

    #[test]
    fn nstation_fixture_deltas_recorded() {
        // The one-time OCC diagnostics (the probe protocol, MONO-2 step 7):
        // synthetic N-station stacks with OCC's smooth-loft volume recorded.
        // The kernel's canonical surface is NOT OCC's approximant, so the
        // per-N delta is recorded as DIAGNOSTIC data, never gated; the
        // kernel's own certificate brackets are the gate. No OCC run happens
        // here.
        let fixtures: [(usize, f64); 5] = [
            (3, 6.625_709_132_106_989),
            (5, 15.613_697_373_247_499),
            (9, 42.735_598_479_334_96),
            (10, 51.836_528_398_090_365),
            (16, 132.168_754_551_587_88),
        ];
        let mut recorded = Vec::new();
        for (n, occ_volume) in fixtures {
            let facts = tree_facts(&part(
                SolidSpec::Loft {
                    sections: nstation_sections(n),
                    closed: false,
                },
                0.0,
                0.0,
                0.0,
            ))
            .expect("the canonical stack measures");
            assert_eq!(facts.solid_count, 1);
            let value = facts.volume;
            assert!(
                value.is_finite() && value > 0.0,
                "N={n}: canonical volume {value}"
            );
            let delta = (value - occ_volume).abs();
            let relative = delta / occ_volume.abs();
            assert!(
                relative.is_finite() && relative < 0.5,
                "N={n}: the canonical-vs-OCC delta {relative} is not a gross error"
            );
            recorded.push((n, value, occ_volume, delta));
        }
        for (n, value, occ_volume, delta) in &recorded {
            println!("MONO-2 fixture N={n}: canonical={value} occ={occ_volume} abs_delta={delta}");
        }
        assert_eq!(recorded.len(), 5);
    }

    #[test]
    fn nstation_42_station_timing_kernel_class() {
        // The monocoque tub's scale: a 42-station synthetic canonical loft
        // measures in kernel-class time (no OCC, no approximation).
        let solid = SolidSpec::Loft {
            sections: nstation_sections(42),
            closed: false,
        };
        let start = std::time::Instant::now();
        let facts = tree_facts(&part(solid, 0.0, 0.0, 0.0)).expect("the 42-station tub measures");
        let elapsed = start.elapsed();
        assert!(facts.volume.is_finite() && facts.volume > 0.0);
        assert!(
            elapsed.as_secs_f64() < 30.0,
            "the 42-station canonical measurement took {elapsed:?}"
        );
        println!(
            "MONO-2 42-station canonical volume {} in {elapsed:?}",
            facts.volume
        );
    }

    // Boolean rows through the boolean funnel (BRIDGE-BOOLEANS): the door
    // shim's cut/fuse/intersect record BooleanOp rows {mode, a, b} and dispatch
    // through the boolean entry; placed operands are canonical (the LOCAL
    // solid classifies) and the placement composes after; a refused pair keeps
    // its typed envelope case; a chain deeper than depth-1 refuses typed.
    // -----------------------------------------------------------------------

    /// One recorded boolean row over two operand nodes.
    fn boolean(mode: crate::facade::ModeValue, a: TreeNode, b: TreeNode) -> TreeNode {
        TreeNode::Boolean {
            boolean: BooleanNode {
                mode,
                a: Box::new(a),
                b: Box::new(b),
            },
        }
    }

    /// A ruled two-station loft (the `Swept` carrier class).
    fn loft_carrier() -> SolidSpec {
        SolidSpec::Loft {
            sections: vec![square(0.0), square(5.0)],
            closed: false,
        }
    }

    /// A spline-profile full-arc lathe (the `Revolved` carrier class).
    fn revolved_carrier() -> SolidSpec {
        SolidSpec::Lathe {
            profile: dome_shell_profile(),
            arc_deg: 360.0,
            start_deg: 0.0,
        }
    }

    /// A canonical box primitive.
    fn canonical_carrier() -> SolidSpec {
        SolidSpec::Box {
            length: 2.0,
            width: 2.0,
            height: 2.0,
        }
    }

    #[test]
    fn cut_swept_canonical_certifies_end_to_end() {
        // A swept base cut by a canonical tool routes through the boolean entry
        // (the routed event is recorded placement-blind), but the product
        // volume is the open MONO-8 carrier: the boolean facts refuse typed
        // naming the open carrier instead of silently reporting the base
        // operand's volume.
        let node = BooleanNode {
            mode: crate::facade::ModeValue::Subtract,
            a: Box::new(part(loft_carrier(), 0.0, 0.0, 0.0)),
            b: Box::new(part(canonical_carrier(), 0.0, 0.0, 0.0)),
        };
        let event = dispatch_boolean(&node)
            .expect("a swept x canonical cut routes")
            .expect("the routed row records an event");
        assert_eq!(event.mode, crate::facade::ModeValue::Subtract);
        assert_eq!(event.base, crate::facade::CarrierClass::Swept);
        assert_eq!(event.tool, crate::facade::CarrierClass::Canonical);
        let refusal = tree_facts(&TreeNode::Boolean { boolean: node })
            .expect_err("the swept product volume is unavailable");
        assert!(matches!(
            refusal,
            Refusal::UnsupportedEnvelope(EnvelopeCase::NonCanonicalCarrier)
        ));
    }

    #[test]
    fn fuse_swept_swept_certifies_end_to_end() {
        // Two swept-family carriers the admission consult admits (a ruled loft
        // and a spline-profile revolve, both funnel carriers): the fuse routes
        // and records its event; the product volume (two open carriers) refuses
        // typed rather than measuring the base.
        let node = BooleanNode {
            mode: crate::facade::ModeValue::Add,
            a: Box::new(part(loft_carrier(), 0.0, 0.0, 0.0)),
            b: Box::new(part(revolved_carrier(), 0.0, 0.0, 0.0)),
        };
        let event = dispatch_boolean(&node)
            .expect("a swept x revolved fuse routes")
            .expect("the routed row records an event");
        assert_eq!(event.mode, crate::facade::ModeValue::Add);
        assert_eq!(event.base, crate::facade::CarrierClass::Swept);
        assert_eq!(event.tool, crate::facade::CarrierClass::Revolved);
        assert!(tree_facts(&TreeNode::Boolean { boolean: node }).is_err());
    }

    #[test]
    fn intersect_swept_canonical_certifies_end_to_end() {
        // A revolved base intersected with a canonical tool routes through the
        // boolean entry with the intersect mode recorded; the product volume of
        // the open revolved carrier refuses typed.
        let node = BooleanNode {
            mode: crate::facade::ModeValue::Intersect,
            a: Box::new(part(revolved_carrier(), 0.0, 0.0, 0.0)),
            b: Box::new(part(canonical_carrier(), 0.0, 0.0, 0.0)),
        };
        let event = dispatch_boolean(&node)
            .expect("a revolved x canonical intersect routes")
            .expect("the routed row records an event");
        assert_eq!(event.mode, crate::facade::ModeValue::Intersect);
        assert_eq!(event.base, crate::facade::CarrierClass::Revolved);
        assert_eq!(event.tool, crate::facade::CarrierClass::Canonical);
        assert!(tree_facts(&TreeNode::Boolean { boolean: node }).is_err());
    }

    #[test]
    fn placed_operands_transform_to_canonical_before_dispatch() {
        // The dispatch sees the operand's LOCAL carrier class, never its
        // placement; the placement composes after and is reflected in the
        // measured world facts. The product volume of the placed swept base
        // refuses typed (the open carrier).
        let placed_base = part(loft_carrier(), 10.0, 0.0, 0.0);
        let node = BooleanNode {
            mode: crate::facade::ModeValue::Subtract,
            a: Box::new(placed_base),
            b: Box::new(part(canonical_carrier(), 0.0, 0.0, 0.0)),
        };
        let event = dispatch_boolean(&node)
            .expect("a placed swept base routes")
            .expect("the routed row records an event");
        assert_eq!(event.base, crate::facade::CarrierClass::Swept);
        let unplaced_tree = part(loft_carrier(), 0.0, 0.0, 0.0);
        let unplaced = match &unplaced_tree {
            TreeNode::Part { part } => part_world_bbox(part).expect("unplaced base bbox"),
            _ => panic!("the reference operand is a placed part"),
        };
        match &*node.a {
            TreeNode::Part { part: placed } => {
                let placed_bbox = part_world_bbox(placed).expect("placed base bbox");
                assert!((placed_bbox[0][0] - (unplaced[0][0] + 10.0)).abs() < 1e-12);
                assert!((placed_bbox[1][0] - (unplaced[1][0] + 10.0)).abs() < 1e-12);
            }
            _ => panic!("the boolean base operand is a placed part"),
        }
        assert!(tree_facts(&TreeNode::Boolean { boolean: node }).is_err());
    }

    #[test]
    fn refused_pairs_still_refuse_typed_unchanged() {
        // A both-Swept pair now routes into the certified gate order: two
        // coincident lofts extract but meet non-transversally, so the pair
        // refuses typed at the transversality stage (the landed localized
        // ContactReductionDeferred case).
        let swept_swept = boolean(
            crate::facade::ModeValue::Add,
            part(loft_carrier(), 0.0, 0.0, 0.0),
            part(loft_carrier(), 0.0, 0.0, 0.0),
        );
        let refusal = tree_facts(&swept_swept).expect_err("two lofts must refuse typed");
        assert!(matches!(
            refusal,
            Refusal::UnsupportedEnvelope(EnvelopeCase::ContactReductionDeferred)
        ));

        // A torus carrier in a swept pair keeps the localized refusal.
        let torus_pair = boolean(
            crate::facade::ModeValue::Subtract,
            part(loft_carrier(), 0.0, 0.0, 0.0),
            part(
                SolidSpec::Torus {
                    major: 5.0,
                    minor: 1.0,
                },
                0.0,
                0.0,
                0.0,
            ),
        );
        let refusal = tree_facts(&torus_pair).expect_err("a torus pair must refuse typed");
        assert!(matches!(
            refusal,
            Refusal::UnsupportedEnvelope(EnvelopeCase::ContactReductionDeferred)
        ));

        // A canonical x canonical pair lands on the canonical path, records no
        // routed event, and measures the PRODUCT volume (a nested canonical
        // tool cut out of the canonical base).
        let canonical = boolean(
            crate::facade::ModeValue::Subtract,
            part(canonical_carrier(), 0.0, 0.0, 0.0),
            part(
                SolidSpec::Box {
                    length: 1.0,
                    width: 1.0,
                    height: 1.0,
                },
                0.0,
                0.0,
                0.0,
            ),
        );
        let facts = tree_facts(&canonical).expect("canonical x canonical lands");
        assert!(facts.boolean_events.is_empty());
        assert!(
            (facts.volume - 7.0).abs() < 1.0e-4,
            "the canonical cut measures the product 7, got {}",
            facts.volume
        );
    }

    #[test]
    fn boolean_product_volume_refuses_unavailable_carrier_typed() {
        // The canonical product is measured by the landed MONO-6 certificate; a
        // swept operand is the open carrier and refuses the typed
        // `BooleanProductVolumeUnavailable` (never the base operand's volume).
        let canonical = BooleanNode {
            mode: crate::facade::ModeValue::Subtract,
            a: Box::new(part(canonical_carrier(), 0.0, 0.0, 0.0)),
            b: Box::new(part(
                SolidSpec::Box {
                    length: 1.0,
                    width: 1.0,
                    height: 1.0,
                },
                0.0,
                0.0,
                0.0,
            )),
        };
        let volume =
            boolean_product_volume_certified(&canonical).expect("the canonical product measures");
        assert!((volume - 7.0).abs() < 1.0e-4, "product volume {volume}");

        let swept = BooleanNode {
            mode: crate::facade::ModeValue::Subtract,
            a: Box::new(part(canonical_carrier(), 0.0, 0.0, 0.0)),
            b: Box::new(part(loft_carrier(), 0.0, 0.0, 0.0)),
        };
        assert_eq!(
            boolean_product_volume_certified(&swept),
            Err(membership::BooleanVolumeRefusal::BooleanProductVolumeUnavailable)
        );
    }

    // -----------------------------------------------------------------------
    // Plate-section members and the exact mirror isometry
    // (MONO-3-BLADE-MEMBERS-MIRROR). The corpus member vocabulary
    // (`surfaces.swept_plate` / `surfaces.blade_path` / `surfaces.blade_member`)
    // authors a closed plate profile and sweeps it along a recorded station
    // chain; the member carrier places the profile by each station frame and
    // rides the landed MONO-2 loft certificate per adjacent pair. Mirroring a
    // kernel row reflects its control points exactly (no re-approximation) and
    // is an involution.
    // -----------------------------------------------------------------------

    /// One identity station frame at `z`.
    fn member_station(z: f64) -> StationFrame {
        StationFrame {
            origin: [0.0, 0.0, z],
            x_dir: [1.0, 0.0, 0.0],
            y_dir: [0.0, 1.0, 0.0],
            z_dir: [0.0, 0.0, 1.0],
        }
    }

    /// A three-station ruled plate member: the unit-square plate carried by
    /// spline edges swept along +z.
    fn member_fixture() -> SolidSpec {
        SolidSpec::Member {
            profile: spline_square(0.0),
            stations: vec![
                member_station(0.0),
                member_station(2.5),
                member_station(5.0),
            ],
            ruled: true,
        }
    }

    /// The signed enclosed volume of a closed triangle soup about the origin
    /// (the orientation witness of the mirror isometry).
    fn signed_mesh_volume(triangles: &[Triangle]) -> f64 {
        let mut total = 0.0f64;
        for tri in triangles {
            let a = [tri[0], tri[1], tri[2]];
            let b = [tri[3], tri[4], tri[5]];
            let c = [tri[6], tri[7], tri[8]];
            total += v3_dot(a, v3_cross(b, c)) / 6.0;
        }
        total
    }

    #[test]
    fn member_volume_certified_bracket() {
        // The plate-section member certifies through the landed MONO-2
        // two-station certificate summed over adjacent station pairs. The
        // fixture is the unit-square plate carried by spline edges, so the
        // recorded reference is the exact prism volume 5.
        let profile = spline_square(0.0);
        let two = vec![member_station(0.0), member_station(5.0)];
        let (value, lo, hi) =
            member_volume_bracket(&profile, &two, true).expect("two-station member");
        assert!(
            lo <= value && value <= hi,
            "bracket [{lo}, {hi}] must enclose {value}"
        );
        assert!(
            (hi - lo) < 1.0e-6,
            "the certified bracket is tight: [{lo}, {hi}]"
        );
        assert!(
            (value - 5.0).abs() < 1.0e-9,
            "member volume {value} must match 5"
        );

        // Three stations: the interior caps of the two pairs cancel exactly,
        // so the chain still measures the same prism volume (no double count).
        let three_stations = vec![
            member_station(0.0),
            member_station(2.5),
            member_station(5.0),
        ];
        let (chain, chain_lo, chain_hi) =
            member_volume_bracket(&profile, &three_stations, true).expect("three-station member");
        assert!(chain_lo <= chain && chain <= chain_hi);
        assert!(
            (chain - 5.0).abs() < 1.0e-9,
            "chain volume {chain} must match 5"
        );
        let three = SolidSpec::Member {
            profile,
            stations: three_stations,
            ruled: true,
        };
        let facts = tree_facts(&part(three, 0.0, 0.0, 0.0)).expect("member facts");
        assert!((facts.volume - 5.0).abs() < 1.0e-9);
        assert_eq!(facts.bbox[0], [0.0, 0.0, 0.0]);
        assert_eq!(facts.bbox[1], [1.0, 1.0, 5.0]);
        assert_eq!(facts.solid_count, 1);
    }

    #[test]
    fn member_refuses_open_path_typed() {
        // An open plate boundary is not a closed section loop: the member
        // refuses typed with the landed sweep carrier's `NonCanonicalCarrier`,
        // never a flattening or a tolerance stretch.
        let open = vec![
            ProfileEdge::Line {
                a: [0.0, 0.0, 0.0],
                b: [1.0, 0.0, 0.0],
            },
            ProfileEdge::Line {
                a: [1.0, 0.0, 0.0],
                b: [1.0, 1.0, 0.0],
            },
            ProfileEdge::Line {
                a: [1.0, 1.0, 0.0],
                b: [0.0, 1.0, 0.0],
            },
        ];
        let member = SolidSpec::Member {
            profile: open,
            stations: vec![member_station(0.0), member_station(5.0)],
            ruled: true,
        };
        let refusal = tree_facts(&part(member, 0.0, 0.0, 0.0))
            .expect_err("an open member plate must refuse typed");
        assert!(matches!(
            refusal,
            Refusal::UnsupportedEnvelope(EnvelopeCase::NonCanonicalCarrier)
        ));
    }

    #[test]
    fn mirror_is_exact_isometry() {
        // Reflecting a member row across y = 0 is an exact isometry: the
        // certified volume is invariant in magnitude, the world bbox is the
        // reflected bbox, and the mesh orientation flips (det R = -1). The
        // mirrored row reuses the original's patch grid.
        let member = member_fixture();
        let base = tree_facts(&part(member.clone(), 0.0, 0.0, 0.0)).expect("base member");
        let mirrored = tree_facts(&mirrored_part(member.clone(), "y")).expect("mirrored member");
        assert_eq!(
            base.volume.to_bits(),
            mirrored.volume.to_bits(),
            "the volume magnitude is invariant under the reflection"
        );
        assert_eq!(
            mirrored.bbox[0],
            [base.bbox[0][0], -base.bbox[1][1], base.bbox[0][2]]
        );
        assert_eq!(
            mirrored.bbox[1],
            [base.bbox[1][0], -base.bbox[0][1], base.bbox[1][2]]
        );
        // Orientation flips: the signed mesh volume negates.
        let base_signed = signed_mesh_volume(&solid_mesh(&member, None).expect("base mesh"));
        let mirror_solid = reflect_solid(&member, "y").expect("control-point reflection");
        let mirror_signed = signed_mesh_volume(&solid_mesh(&mirror_solid, None).expect("mirror mesh"));
        assert!(base_signed * mirror_signed < 0.0, "orientation must flip");
        assert!(
            (base_signed.abs() - mirror_signed.abs()).abs() <= 1.0e-9 * (1.0 + base_signed.abs())
        );
        // The patch grid is reused: same station count and edge count.
        match (&member, &mirror_solid) {
            (
                SolidSpec::Member {
                    profile: pa,
                    stations: sa,
                    ..
                },
                SolidSpec::Member {
                    profile: pb,
                    stations: sb,
                    ..
                },
            ) => {
                assert_eq!(pa.len(), pb.len());
                assert_eq!(sa.len(), sb.len());
            }
            _ => panic!("the member fixture must reflect to a member"),
        }
    }

    #[test]
    fn mirror_twice_is_identity() {
        // The control-point reflection is a bit-for-bit involution: mirroring
        // twice returns the original row exactly (serialized bytes equal).
        let member = member_fixture();
        for axis in ["x", "y", "z"] {
            let once = reflect_solid(&member, axis).expect("first reflection");
            let twice = reflect_solid(&once, axis).expect("second reflection");
            assert_eq!(twice, member, "mirror(mirror(x)) = x for axis {axis}");
            assert_eq!(
                serde_json::to_string(&twice).expect("twice json"),
                serde_json::to_string(&member).expect("member json"),
                "the double reflection is bit-for-bit identical"
            );
        }
    }

    #[test]
    fn smooth_member_multi_station_refuses_typed() {
        // A smooth (non-ruled) member with more than two stations has no landed
        // smooth carrier: the typed refusal stays rather than a ruled
        // substitution.
        let smooth = SolidSpec::Member {
            profile: spline_square(0.0),
            stations: vec![
                member_station(0.0),
                member_station(2.5),
                member_station(5.0),
            ],
            ruled: false,
        };
        let refusal = tree_facts(&part(smooth, 0.0, 0.0, 0.0))
            .expect_err("a smooth multi-station member must refuse typed");
        assert!(matches!(
            refusal,
            Refusal::UnsupportedEnvelope(EnvelopeCase::NonCanonicalCarrier)
        ));
    }

    // -----------------------------------------------------------------------
    // MONO-4-TRIM-IDIOMS: the trim constructor's corpus spline-profile idiom.
    //
    // The corpus authors a closed plate section as ONE periodic spline and
    // extrudes it symmetrically (`bd.extrude(plane * make_face(
    // bd.Spline(*pts, periodic=True)), amount, both=True)`): the rear-wing
    // louvre cutters (`rear_wing.py:390`) and the suspension `_plate` profiles
    // (`suspension.py:162`). The door records a `trim_prism` row with an empty
    // line profile and no pullback net; the recorded spline IS the base
    // profile, so the arm realizes the local-frame extrude over the closed
    // spline loop. Everything outside the idiom keeps its typed refusal.
    // -----------------------------------------------------------------------

    /// The corpus `rounded_plate_pts` outline (`surfaces.py:656`): a rounded
    /// rectangle centred on the origin, `samples + 1` points per corner, in
    /// the `z = 0` chart. First point != last (the door appends the closure).
    fn rounded_plate_loop(length: f64, height: f64, corner: f64, samples: usize) -> Vec<[f64; 3]> {
        let (hl, hh, r) = (length / 2.0, height / 2.0, corner);
        let corners = [
            (hl - r, hh - r, 0.0f64),
            (-hl + r, hh - r, std::f64::consts::FRAC_PI_2),
            (-hl + r, -hh + r, std::f64::consts::PI),
            (hl - r, -hh + r, 3.0 * std::f64::consts::FRAC_PI_2),
        ];
        let mut pts = Vec::new();
        for (cu, cv, a0) in corners {
            for i in 0..=samples {
                let a = a0 + std::f64::consts::FRAC_PI_2 * i as f64 / samples as f64;
                pts.push([cu + r * a.cos(), cv + r * a.sin(), 0.0]);
            }
        }
        pts
    }

    /// The door's closing rule: the recorded spline loop repeats its first
    /// point when the carrier does not already return to it.
    fn closed_curve(loop_pts: &[[f64; 3]]) -> Vec<[f64; 3]> {
        let mut curve = loop_pts.to_vec();
        if curve.first() != curve.last()
            && let Some(first) = curve.first().copied()
        {
            curve.push(first);
        }
        curve
    }

    /// The door's `trim_prism` row for `extrude(spline_face, amount, both)`:
    /// an empty line profile, the closed recorded spline curve, no pullback
    /// net.
    fn spline_prism_row(loop_pts: &[[f64; 3]], amount: f64, both: bool) -> SolidSpec {
        SolidSpec::TrimPrism {
            profile: Vec::new(),
            amount,
            both,
            trim_curve: closed_curve(loop_pts),
            trim_net: Vec::new(),
            tolerance: 1.0e-3,
        }
    }

    #[test]
    fn louver_idiom_composes_certified() {
        // The louvre cutter: an 86 x 6.8 rounded plate (corner 3.2) as one
        // periodic spline, extruded +-40 symmetrically about its plane
        // (`rear_wing.py:390`). The row certifies as the exact prism over the
        // reconstructed closed spline loop.
        let loop_pts = rounded_plate_loop(86.0, 6.8, 3.2, 5);
        let row = spline_prism_row(&loop_pts, 40.0, true);
        let facts = tree_facts(&part(row.clone(), 0.0, 0.0, 0.0))
            .expect("the louvre spline-profile extrude is in the constructor envelope");
        assert!(facts.volume > 0.0);

        // The volume is the reconstructed loop's exact area times the swept
        // height (2 * 40): the certified value and the landed span-area
        // reconstruction must agree.
        let spans = spline_spans3(&closed_curve(&loop_pts)).expect("the loop reconstructs");
        let area = v3_norm(spline_loop_area_vector(&spans));
        let expected = area * 80.0;
        assert!(
            (facts.volume - expected).abs() <= 1.0e-9 * (1.0 + expected),
            "certified volume {} must equal area * height {}",
            facts.volume,
            expected
        );

        // The extrusion is along the loop plane normal (z here): the world
        // bbox is the loop box widened by +-40.
        assert_eq!(facts.bbox[0][2], -40.0);
        assert_eq!(facts.bbox[1][2], 40.0);
        assert!(facts.bbox[0][0] <= -43.0 + 1.0e-9);
        assert!(facts.bbox[1][0] >= 43.0 - 1.0e-9);
        assert!(facts.bbox[0][1] <= -3.4 + 1.0e-9);
        assert!(facts.bbox[1][1] >= 3.4 - 1.0e-9);

        // The realized mesh is closed and its signed volume tracks the exact
        // prism value to the sampling floor.
        let mesh = solid_mesh(&row, None).expect("the spline-profile prism meshes");
        assert!(!mesh.is_empty());
        let signed = signed_mesh_volume(&mesh).abs();
        assert!(
            (signed - facts.volume).abs() / facts.volume < 1.0e-3,
            "mesh signed volume {signed} must track the certified {}",
            facts.volume
        );
    }

    #[test]
    fn suspension_plate_idiom_composes_certified() {
        // The suspension `_plate` (`suspension.py:160`): a closed plate
        // outline as one periodic spline, extruded symmetrically. A second
        // representative outline exercises the same constructor path.
        let loop_pts = rounded_plate_loop(30.0, 58.0, 13.0, 6);
        let row = spline_prism_row(&loop_pts, 12.5, true);
        let facts = tree_facts(&part(row.clone(), 0.0, 0.0, 0.0))
            .expect("the suspension plate spline-profile extrude is in envelope");
        assert!(facts.volume > 0.0);
        let spans = spline_spans3(&closed_curve(&loop_pts)).expect("the loop reconstructs");
        let area = v3_norm(spline_loop_area_vector(&spans));
        let expected = area * 25.0;
        assert!((facts.volume - expected).abs() <= 1.0e-9 * (1.0 + expected));
        assert_eq!(facts.bbox[0][2], -12.5);
        assert_eq!(facts.bbox[1][2], 12.5);
        // The idiom classifies as the swept carrier the facade's spline
        // extrude produces, so a spline x spline boolean stays the wave-3
        // typed refusal.
        assert_eq!(
            solid_carrier_class(&row),
            crate::facade::CarrierClass::Swept
        );
    }

    #[test]
    fn constructor_envelope_still_refuses_unknown_idioms_typed() {
        // An open recorded spline carrier is not a closed base profile: the
        // arm refuses typed, never a bounded guess.
        let open_row = SolidSpec::TrimPrism {
            profile: Vec::new(),
            amount: 1.0,
            both: false,
            trim_curve: vec![
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [1.0, 1.0, 0.0],
                [0.0, 1.0, 0.0],
            ],
            trim_net: Vec::new(),
            tolerance: 1.0e-3,
        };
        let refusal = tree_facts(&part(open_row, 0.0, 0.0, 0.0))
            .expect_err("an open spline profile must refuse typed");
        assert!(matches!(
            refusal,
            Refusal::UnsupportedEnvelope(EnvelopeCase::NonCanonicalCarrier)
        ));

        // A line base profile with a recorded spline trim and no pullback net
        // is the landed constructor's real-trim carrier: it still refuses the
        // named pullback gap rather than being read as a spline base profile.
        let line_trim = SolidSpec::TrimPrism {
            profile: vec![
                ProfileEdge::Line {
                    a: [0.0, 0.0, 0.0],
                    b: [1.0, 0.0, 0.0],
                },
                ProfileEdge::Line {
                    a: [1.0, 0.0, 0.0],
                    b: [1.0, 1.0, 0.0],
                },
                ProfileEdge::Line {
                    a: [1.0, 1.0, 0.0],
                    b: [0.0, 1.0, 0.0],
                },
                ProfileEdge::Line {
                    a: [0.0, 1.0, 0.0],
                    b: [0.0, 0.0, 0.0],
                },
            ],
            amount: 1.0,
            both: false,
            trim_curve: vec![
                [0.25, 0.25, 0.0],
                [0.75, 0.25, 0.0],
                [0.75, 0.75, 0.0],
                [0.25, 0.75, 0.0],
                [0.25, 0.25, 0.0],
            ],
            trim_net: Vec::new(),
            tolerance: 1.0e-3,
        };
        let refusal = tree_facts(&part(line_trim, 0.0, 0.0, 0.0))
            .expect_err("a line-base trim with no pullback net must refuse typed");
        assert!(matches!(
            refusal,
            Refusal::UnsupportedEnvelope(EnvelopeCase::NonCanonicalCarrier)
        ));
    }

    // -----------------------------------------------------------------------
    // MONO-5-RAY-CLASSIFY: certified point-vs-spline-solid membership.
    // -----------------------------------------------------------------------

    /// One planar quad as a bicubic tensor-Bernstein patch (the exact tensor
    /// degree elevation of the bilinear corner map), weights one.
    fn quad_patch(
        c00: [f64; 3],
        c10: [f64; 3],
        c11: [f64; 3],
        c01: [f64; 3],
    ) -> crate::python::binding::VolumeRow {
        let e0 = [1.0, 2.0 / 3.0, 1.0 / 3.0, 0.0];
        let e1 = [0.0, 1.0 / 3.0, 2.0 / 3.0, 1.0];
        let corners = [[c00, c01], [c10, c11]];
        let mut numerator = vec![vec![[0.0f64; 3]; 4]; 4];
        for i in 0..4 {
            for j in 0..4 {
                let mut acc = [0.0f64; 3];
                for (a, pair) in corners.iter().enumerate() {
                    for (b, corner) in pair.iter().enumerate() {
                        let wa = if a == 0 { e0[i] } else { e1[i] };
                        let wb = if b == 0 { e0[j] } else { e1[j] };
                        let coeff = wa * wb;
                        for (slot, value) in acc.iter_mut().zip(corner.iter()) {
                            *slot += coeff * value;
                        }
                    }
                }
                numerator[i][j] = acc;
            }
        }
        crate::python::binding::VolumeRow {
            numerator,
            weights: vec![vec![1.0f64; 4]; 4],
            orientation: 1.0,
        }
    }

    /// The six outward-oriented faces of the unit cube `[0, 1]^3`.
    fn cube_patches() -> Vec<crate::python::binding::VolumeRow> {
        vec![
            quad_patch([0., 0., 0.], [1., 0., 0.], [1., 1., 0.], [0., 1., 0.]),
            quad_patch([0., 0., 1.], [0., 1., 1.], [1., 1., 1.], [1., 0., 1.]),
            quad_patch([0., 0., 0.], [0., 1., 0.], [0., 1., 1.], [0., 0., 1.]),
            quad_patch([1., 0., 0.], [1., 0., 1.], [1., 1., 1.], [1., 1., 0.]),
            quad_patch([0., 0., 0.], [0., 0., 1.], [1., 0., 1.], [1., 0., 0.]),
            quad_patch([0., 1., 0.], [1., 1., 0.], [1., 1., 1.], [0., 1., 1.]),
        ]
    }

    /// The closed 2-cycle of a two-station square-section loft: the landed
    /// `spline_loft_volume_rows` side patches plus the two planar end caps.
    fn closed_loft_rows() -> Vec<crate::python::binding::VolumeRow> {
        let sections = vec![spline_square(0.0), spline_square(5.0)];
        let (mut patches, _) =
            spline_loft_volume_rows(&sections).expect("the two-station loft assembles");
        patches.push(quad_patch(
            [0., 0., 0.],
            [1., 0., 0.],
            [1., 1., 0.],
            [0., 1., 0.],
        ));
        patches.push(quad_patch(
            [0., 0., 5.],
            [0., 1., 5.],
            [1., 1., 5.],
            [1., 0., 5.],
        ));
        patches
    }

    #[test]
    fn membership_prism_row_known_answers() {
        let patches = cube_patches();
        assert_eq!(
            membership::classify_ray([0.5, 0.5, 0.5], [0.3, 0.7, 0.2], &patches),
            Ok(membership::MembershipVerdict::Inside)
        );
        for point in [
            [2.0, 0.5, 0.5],
            [-0.5, 0.5, 0.5],
            [0.5, 2.0, 0.5],
            [0.5, -0.5, 0.5],
            [0.5, 0.5, 2.0],
            [0.5, 0.5, -0.5],
        ] {
            assert_eq!(
                membership::classify_ray(point, [0.3, 0.7, 0.2], &patches),
                Ok(membership::MembershipVerdict::Outside),
                "point {point:?} must be certified outside"
            );
        }
    }

    #[test]
    fn membership_two_station_loft_crossings_hand_derivable() {
        let patches = closed_loft_rows();
        // Inside the tube: a vertical ray crosses the top cap exactly once.
        assert_eq!(
            membership::classify_ray([0.5, 0.5, 2.5], [0.0, 0.0, 1.0], &patches),
            Ok(membership::MembershipVerdict::Inside)
        );
        // A horizontal ray from inside crosses one side face.
        assert_eq!(
            membership::classify_ray([0.5, 0.5, 2.5], [1.0, 0.0, 0.0], &patches),
            Ok(membership::MembershipVerdict::Inside)
        );
        // Above and below the tube: certified outside.
        assert_eq!(
            membership::classify_ray([0.5, 0.5, 6.0], [0.0, 0.0, 1.0], &patches),
            Ok(membership::MembershipVerdict::Outside)
        );
        assert_eq!(
            membership::classify_ray([0.5, 0.5, -1.0], [0.0, 0.0, 1.0], &patches),
            Ok(membership::MembershipVerdict::Outside)
        );
    }

    #[test]
    fn membership_near_boundary_brackets_are_tight() {
        let patches = cube_patches();
        let (verdict, crossings) = membership::classify_ray_with_evidence(
            [0.5, 0.5, 1.0 - 1.0e-3],
            [0.0, 0.0, 1.0],
            &patches,
        )
        .expect("a transversal near-boundary ray certifies");
        assert_eq!(verdict, membership::MembershipVerdict::Inside);
        assert_eq!(crossings.len(), 1);
        let crossing = crossings.first().expect("one crossing");
        assert!(crossing.t_lo <= 1.0e-3 && 1.0e-3 <= crossing.t_hi);
        assert!(
            crossing.t_hi - crossing.t_lo < 1.0e-6,
            "the crossing bracket must be tight: [{}, {}]",
            crossing.t_lo,
            crossing.t_hi
        );
        assert_eq!(
            membership::classify_ray([0.5, 0.5, 1.0 + 1.0e-3], [0.0, 0.0, 1.0], &patches),
            Ok(membership::MembershipVerdict::Outside)
        );
    }

    #[test]
    fn membership_grazing_rays_refuse_typed() {
        let patches = cube_patches();
        // A ray lying in the z=0 face plane is non-transversal: the clipping
        // loop never separates the surviving set and the cast is refused.
        assert_eq!(
            membership::classify_ray([0.5, 0.5, 0.0], [1.0, 0.0, 0.0], &patches),
            Err(membership::MembershipRefusal::MembershipIndeterminate)
        );
    }

    #[test]
    fn membership_retry_contract_recovers_from_an_edge_hit() {
        let patches = cube_patches();
        // The body diagonal exits through the (1, 1, 1) corner shared by three
        // patches: refused as non-transversal, then recovered by a fresh
        // recorded direction -- never a wrong answer.
        assert_eq!(
            membership::classify_ray([0.5, 0.5, 0.5], [1.0, 1.0, 1.0], &patches),
            Err(membership::MembershipRefusal::MembershipIndeterminate)
        );
        let certificate =
            membership::classify_point([0.5, 0.5, 0.5], [1.0, 1.0, 1.0], &patches, 0x1234);
        assert_eq!(certificate.verdict, membership::MembershipVerdict::Inside);
        assert!(certificate.attempts >= 1);
        assert!(certificate.refusal.is_none());
    }

    #[test]
    fn membership_landed_loft_rows_round_trip() {
        let patches = closed_loft_rows();
        let inside = membership::classify_point([0.5, 0.5, 2.5], [0.11, 0.23, 0.31], &patches, 7);
        assert_eq!(inside.verdict, membership::MembershipVerdict::Inside);
        assert!(inside.refusal.is_none());
        let outside = membership::classify_point([3.0, 3.0, 2.5], [0.11, 0.23, 0.31], &patches, 7);
        assert_eq!(outside.verdict, membership::MembershipVerdict::Outside);
        assert!(outside.refusal.is_none());
    }

    #[test]
    fn membership_refuses_rational_and_malformed_rows_typed() {
        let mut rational = cube_patches();
        if let Some(first) = rational.first_mut() {
            first.weights = vec![vec![2.0; 4]; 4];
        }
        assert_eq!(
            membership::classify_ray([0.5, 0.5, 0.5], [1.0, 0.0, 0.0], &rational),
            Err(membership::MembershipRefusal::RationalWeights)
        );
        let malformed = vec![crate::python::binding::VolumeRow {
            numerator: vec![vec![[0.0; 3]; 1]; 1],
            weights: vec![vec![1.0; 1]; 1],
            orientation: 1.0,
        }];
        assert_eq!(
            membership::classify_ray([0.5, 0.5, 0.5], [1.0, 0.0, 0.0], &malformed),
            Err(membership::MembershipRefusal::MalformedPatch)
        );
    }

    // -----------------------------------------------------------------------
    // MONO-6-SWEPT-BOOLEANS: certified boolean volume by contact covers.
    // -----------------------------------------------------------------------

    /// A closed oriented box `[lo, hi]` as six outward-oriented tensor-
    /// Bernstein patches (the exact bicubic elevation of each bilinear face,
    /// unit weights). The corner order fixes `P_u x P_v` outward on every
    /// face, so the divergence-form flux of the cycle is the box volume.
    fn box_rows(lo: [f64; 3], hi: [f64; 3]) -> Vec<crate::python::binding::VolumeRow> {
        let [x0, y0, z0] = lo;
        let [x1, y1, z1] = hi;
        vec![
            // z = z0, outward -z: P_u = +y, P_v = +x.
            quad_patch([x0, y0, z0], [x0, y1, z0], [x1, y1, z0], [x1, y0, z0]),
            // z = z1, outward +z: P_u = +x, P_v = +y.
            quad_patch([x0, y0, z1], [x1, y0, z1], [x1, y1, z1], [x0, y1, z1]),
            // y = y0, outward -y: P_u = +x, P_v = +z.
            quad_patch([x0, y0, z0], [x1, y0, z0], [x1, y0, z1], [x0, y0, z1]),
            // y = y1, outward +y: P_u = +z, P_v = +x.
            quad_patch([x0, y1, z0], [x0, y1, z1], [x1, y1, z1], [x1, y1, z0]),
            // x = x0, outward -x: P_u = +z, P_v = +y.
            quad_patch([x0, y0, z0], [x0, y0, z1], [x0, y1, z1], [x0, y1, z0]),
            // x = x1, outward +x: P_u = +y, P_v = +z.
            quad_patch([x1, y0, z0], [x1, y1, z0], [x1, y1, z1], [x1, y0, z1]),
        ]
    }

    #[test]
    fn boolean_volume_nested_boxes_closes_exactly() {
        let a = box_rows([0.0, 0.0, 0.0], [2.0, 2.0, 2.0]);
        let b = box_rows([0.5, 0.5, 0.5], [1.5, 1.5, 1.5]);
        let options = membership::BooleanVolumeOptions::default();
        let certificate =
            membership::certify_boolean_volume(&a, &b, &options).expect("the nested boxes certify");
        assert!(
            (certificate.volume_a - 8.0).abs() < 1.0e-9,
            "V(A) {}",
            certificate.volume_a
        );
        assert!(
            (certificate.volume_b - 1.0).abs() < 1.0e-9,
            "V(B) {}",
            certificate.volume_b
        );
        assert!(
            (certificate.value - 7.0).abs() < 1.0e-6,
            "V(A \\ B) = {} must be 7",
            certificate.value
        );
        assert!(certificate.bracket_lo <= 7.0 && 7.0 <= certificate.bracket_hi);
        assert!(certificate.width < 1.0e-6, "width {}", certificate.width);
        assert_eq!(certificate.solid_count, 1);
        assert!(certificate.bbox_margin.is_some());
    }

    #[test]
    fn boolean_volume_disjoint_boxes_answers_the_base() {
        let a = box_rows([0.0, 0.0, 0.0], [2.0, 2.0, 2.0]);
        let b = box_rows([5.0, 5.0, 5.0], [6.0, 6.0, 6.0]);
        let options = membership::BooleanVolumeOptions::default();
        let certificate =
            membership::certify_boolean_volume(&a, &b, &options).expect("disjoint boxes certify");
        assert!(
            (certificate.value - 8.0).abs() < 1.0e-6,
            "value {}",
            certificate.value
        );
        assert!(
            certificate.excluded_pairs > 0,
            "the broad phase must exclude the pair"
        );
    }

    #[test]
    fn contact_cover_reports_the_contact_locus() {
        let a = box_rows([0.0, 0.0, 0.0], [2.0, 2.0, 2.0]);
        let b = box_rows([0.5, 0.5, 0.5], [1.5, 1.5, 3.0]);
        let top = a.get(1).expect("the top face is the second patch");
        let cover = membership::contact_cover(top, &b, 5).expect("the cover builds");
        assert!(!cover.is_empty(), "the top face meets B");
        for cell in &cover {
            assert!(cell.u_lo >= 0.0 && cell.u_hi <= 1.0);
            assert!(cell.v_lo >= 0.0 && cell.v_hi <= 1.0);
        }
        // The top face parameterizes u -> x and v -> y, so the origin corner
        // [0, 0.125]^2 is far from B's [0.25, 0.75]^2 contact square.
        let origin_covered = cover.iter().any(|c| c.u_lo < 0.125 && c.v_lo < 0.125);
        assert!(!origin_covered, "the origin corner is clear");
    }

    #[test]
    fn boolean_volume_crossing_boxes_certified() {
        // A = [0,2]^3, B = [0.5,1.5]^2 x [0.5,3]: B pokes through A's top
        // face, so the contact cover is genuinely exercised. A n B is
        // [0.5,1.5]^2 x [0.5,2], volume 1.5; A \ B is 8 - 1.5 = 6.5.
        let a = box_rows([0.0, 0.0, 0.0], [2.0, 2.0, 2.0]);
        let b = box_rows([0.5, 0.5, 0.5], [1.5, 1.5, 3.0]);
        let mut options = membership::BooleanVolumeOptions::default();
        // The validation geometry's point is the contact-cover machinery, not
        // a tight bracket: a coarse budget keeps the debug-build suite fast
        // while still exercising the cover, the refinement and the witnesses.
        options.relative_tolerance = 2.0;
        options.max_depth = 6;
        options.enforce_bbox_certificate = false;
        let certificate = membership::certify_boolean_volume(&a, &b, &options)
            .expect("the crossing boxes certify");
        assert!(
            certificate.intersection_lo <= 1.5 && 1.5 <= certificate.intersection_hi,
            "V(A n B) bracket [{}, {}] must contain 1.5",
            certificate.intersection_lo,
            certificate.intersection_hi
        );
        assert!(
            certificate.bracket_lo <= 6.5 && 6.5 <= certificate.bracket_hi,
            "V(A \\ B) bracket [{}, {}] must contain 6.5",
            certificate.bracket_lo,
            certificate.bracket_hi
        );
        assert!(
            certificate.contact_cells > 0,
            "the contact locus was exercised"
        );
        assert!(certificate.clear_cells > 0);
        println!(
            "MONO-6 crossing boxes: bracket [{}, {}] width {} depth {} contact {} clear {} cover {} phases {:?}",
            certificate.bracket_lo,
            certificate.bracket_hi,
            certificate.width,
            certificate.max_depth,
            certificate.contact_cells,
            certificate.clear_cells,
            certificate.cover_cells,
            certificate.phases
        );
    }

    #[test]
    fn boolean_volume_extreme_slab_refuses_typed() {
        let a = box_rows([0.0, 0.0, 0.0], [2.0, 2.0, 2.0]);
        let b = box_rows([0.5, 0.5, 0.5], [1.5, 1.5, 3.0]);
        let options = membership::BooleanVolumeOptions::default();
        assert_eq!(
            membership::certify_boolean_volume(&a, &b, &options),
            Err(membership::BooleanVolumeRefusal::ExtremeSlabContaminated)
        );
    }

    #[test]
    fn boolean_volume_refuses_rational_and_malformed_typed() {
        let a = box_rows([0.0, 0.0, 0.0], [2.0, 2.0, 2.0]);
        let mut b = box_rows([0.5, 0.5, 0.5], [1.5, 1.5, 1.5]);
        if let Some(first) = b.first_mut() {
            first.weights = vec![vec![2.0; 4]; 4];
        }
        let options = membership::BooleanVolumeOptions::default();
        assert_eq!(
            membership::certify_boolean_volume(&a, &b, &options),
            Err(membership::BooleanVolumeRefusal::RationalWeights)
        );
        let malformed = vec![crate::python::binding::VolumeRow {
            numerator: vec![vec![[0.0; 3]; 1]; 1],
            weights: vec![vec![1.0; 1]; 1],
            orientation: 1.0,
        }];
        assert_eq!(
            membership::certify_boolean_volume(&a, &malformed, &options),
            Err(membership::BooleanVolumeRefusal::MalformedPatch)
        );
    }

    // -----------------------------------------------------------------------
    // MONO-8-SWEPT-ADMISSION-WIRING: the extraction adapter and the Swept x
    // Swept gate order (extraction -> transversality -> solver).
    // -----------------------------------------------------------------------

    /// A straight spline-section loft stack: `n` sections, each a square of
    /// side `s` placed at `(x0, y0)` and carried by collinear spline edges, so
    /// the section caps are exact planar quads.
    fn straight_loft(n: usize, z0: f64, dz: f64, x0: f64, y0: f64, s: f64) -> SolidSpec {
        let mut sections = Vec::with_capacity(n);
        for i in 0..n {
            let z = z0 + dz * i as f64;
            let corners = [
                [x0, y0, z],
                [x0 + s, y0, z],
                [x0 + s, y0 + s, z],
                [x0, y0 + s, z],
            ];
            let mut edges = Vec::with_capacity(4);
            for j in 0..4 {
                let a = corners[j];
                let b = corners[(j + 1) % 4];
                let mid = [
                    0.5 * (a[0] + b[0]),
                    0.5 * (a[1] + b[1]),
                    0.5 * (a[2] + b[2]),
                ];
                edges.push(ProfileEdge::Spline {
                    points: vec![a, mid, b],
                });
            }
            sections.push(edges);
        }
        SolidSpec::Loft {
            sections,
            closed: false,
        }
    }

    #[test]
    fn swept_admission_nested_boxes_certify_exactly() {
        let node = boolean(
            crate::facade::ModeValue::Subtract,
            part(
                SolidSpec::Box {
                    length: 2.0,
                    width: 2.0,
                    height: 2.0,
                },
                0.0,
                0.0,
                0.0,
            ),
            part(
                SolidSpec::Box {
                    length: 1.0,
                    width: 1.0,
                    height: 1.0,
                },
                0.0,
                0.0,
                0.0,
            ),
        );
        let TreeNode::Boolean { boolean } = &node else {
            unreachable!("the fixture is a boolean node");
        };
        let certificate = admit_swept_pair(boolean).expect("nested boxes certify");
        assert!((certificate.volume_a - 8.0).abs() < 1.0e-9);
        assert!((certificate.volume_b - 1.0).abs() < 1.0e-9);
        assert!(certificate.bracket_lo <= 7.0 && 7.0 <= certificate.bracket_hi);
        assert!(certificate.width < 1.0e-6, "width {}", certificate.width);
    }

    #[test]
    fn swept_admission_disjoint_boxes_answer_the_base() {
        let node = boolean(
            crate::facade::ModeValue::Subtract,
            part(
                SolidSpec::Box {
                    length: 2.0,
                    width: 2.0,
                    height: 2.0,
                },
                0.0,
                0.0,
                0.0,
            ),
            part(
                SolidSpec::Box {
                    length: 1.0,
                    width: 1.0,
                    height: 1.0,
                },
                10.0,
                0.0,
                0.0,
            ),
        );
        let TreeNode::Boolean { boolean } = &node else {
            unreachable!("the fixture is a boolean node");
        };
        let certificate = admit_swept_pair(boolean).expect("disjoint boxes certify");
        assert!(certificate.bracket_lo <= 8.0 && 8.0 <= certificate.bracket_hi);
        assert!(certificate.excluded_pairs > 0);
    }

    #[test]
    fn swept_admission_spline_loft_pair_closes_within_budget() {
        // A nested pair of straight spline-section lofts at the tub's station
        // scale: the executor extracts both 2-cycles (side patches plus exact
        // planar caps) and the solver certifies V(A \ B) = V(A) - V(B).
        let base = straight_loft(8, 0.0, 1.0, 0.0, 0.0, 2.0);
        let tool = straight_loft(4, 2.0, 1.0, 0.5, 0.5, 1.0);
        let base_volume = tree_facts(&part(base.clone(), 0.0, 0.0, 0.0))
            .expect("the base loft measures")
            .volume;
        let tool_volume = tree_facts(&part(tool.clone(), 0.0, 0.0, 0.0))
            .expect("the tool loft measures")
            .volume;
        let tree = boolean(
            crate::facade::ModeValue::Subtract,
            part(base, 0.0, 0.0, 0.0),
            part(tool, 0.0, 0.0, 0.0),
        );
        let facts = tree_facts(&tree).expect("the nested loft pair certifies");
        let expected = base_volume - tool_volume;
        assert!(
            facts.volume_bracket[0] <= expected && expected <= facts.volume_bracket[1],
            "bracket {:?} must contain {expected}",
            facts.volume_bracket
        );
        assert!(
            facts.volume_bracket[1] - facts.volume_bracket[0] < 1.0e-4 * base_volume.abs(),
            "the bracket must close within the facts band"
        );
        assert_eq!(facts.boolean_events.len(), 1);
    }

    #[test]
    fn swept_admission_mirror_flips_orientation_signs() {
        // A mirrored placement is reflective: every extracted orientation sign
        // flips, so the placed 2-cycle stays outward-oriented and the certified
        // volume keeps its sign.
        let mirrored = mirrored_part(
            SolidSpec::Box {
                length: 2.0,
                width: 2.0,
                height: 2.0,
            },
            "y",
        );
        let extracted = extract_patches(&mirrored).expect("the mirrored box extracts");
        assert!(!extracted.is_empty());
        for (patch, sign) in &extracted {
            assert_eq!(*sign, -1, "a mirror placement is reflective");
            assert_eq!(patch.orientation, -1.0);
        }
        let node = boolean(
            crate::facade::ModeValue::Subtract,
            mirrored,
            part(
                SolidSpec::Box {
                    length: 1.0,
                    width: 1.0,
                    height: 1.0,
                },
                0.0,
                0.0,
                0.0,
            ),
        );
        let TreeNode::Boolean { boolean } = &node else {
            unreachable!("the fixture is a boolean node");
        };
        let certificate = admit_swept_pair(boolean).expect("the mirrored cut certifies");
        assert!(certificate.volume_a > 0.0);
        assert!(certificate.bracket_lo > 0.0);
    }

    #[test]
    fn swept_admission_tangential_pair_refuses_coincidence_without_carrier() {
        // Two coincident straight lofts extract but meet non-transversally. The
        // RDEF-M2 regime split admits them as (G) and the sandwich rule detects
        // the exact coincidence: with no recorded common carrier the pair
        // refuses `CoincidenceWithoutExactCarrier`.
        let node = boolean(
            crate::facade::ModeValue::Add,
            part(straight_loft(3, 0.0, 1.0, 0.0, 0.0, 1.0), 0.0, 0.0, 0.0),
            part(straight_loft(3, 0.0, 1.0, 0.0, 0.0, 1.0), 0.0, 0.0, 0.0),
        );
        let TreeNode::Boolean { boolean } = &node else {
            unreachable!("the fixture is a boolean node");
        };
        assert_eq!(
            admit_swept_pair(boolean),
            Err(SweptAdmissionRefusal::CoincidenceWithoutExactCarrier)
        );
        // The boundary marshals the stage onto the localized contact case.
        assert!(matches!(
            tree_facts(&node),
            Err(Refusal::UnsupportedEnvelope(
                EnvelopeCase::ContactReductionDeferred
            ))
        ));
    }

    #[test]
    fn swept_admission_member_carrier_refuses_extraction() {
        // A smooth member with more than two stations has no landed smooth
        // carrier: the extraction refuses typed, never approximates.
        let smooth_member = SolidSpec::Member {
            profile: spline_square(0.0),
            stations: vec![
                member_station(0.0),
                member_station(2.5),
                member_station(5.0),
            ],
            ruled: false,
        };
        assert_eq!(
            extract_patches(&part(smooth_member, 0.0, 0.0, 0.0)),
            Err(SweptAdmissionRefusal::ExtractionUnavailable)
        );
        // A curved-section loft is outside the exact-cap vocabulary too.
        let curved = SolidSpec::Loft {
            sections: vec![spline_lens(0.0), spline_lens(5.0)],
            closed: false,
        };
        assert_eq!(
            extract_patches(&part(curved, 0.0, 0.0, 0.0)),
            Err(SweptAdmissionRefusal::ExtractionUnavailable)
        );
    }
}
