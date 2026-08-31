//! BG-CAD-P1-UTILITY — the Phase 7 utility surface + planar face construction.
//!
//! The build123d coverage program's P1 tier, composed entirely from landed
//! machinery:
//!
//! - [`solid_bounding_box`] — the certified axis-aligned box, derived
//!   per-face over canonical carriers;
//! - the similarity fold — [`translate_solid`], [`uniform_scale_solid`],
//!   [`mirror_solid`], and the P10 general entries [`rotate_solid`],
//!   [`mirror_about_plane`] — one affine map applied over the whole `Vertex`→
//!   `Solid` chain by the landed `Mapped` impls;
//! - [`make_face`] — planar face construction on z = 0 from the landed
//!   arrangement's material regions (build123d semantics: one face per
//!   material region);
//! - [`make_hull`] — the 2-D convex hull through the landed exact predicate
//!   `orient2d`, finished as a planar face.
//!
//! Every operation returns `Outcome<T>` — a certified value or a typed
//! `Refusal`, never an uncertified maybe-answer — and every output stays
//! downstream-consumable: canonical carriers recognized by
//! `recognize_surface`/`recognize_curve`.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

use crate::{
    BoundingBox, Curve, Edge, EuclideanSpace, InnerSpace, Line, Mapped, Matrix4, Plane, Point2,
    Point3, Processor, Rad, SquareMatrix, Surface, Transform, TrimmedCurve, UnitCircle, Vector3,
    Vector4, Vertex, Wire, TOLERANCE,
};
use std::collections::HashMap;
use truck_base::evidence::{
    Budget, Certificate, Certified, Collapse, CollapseReason, ContradictionWitness, EnvelopeCase,
    Margin, Method, Modulus, Outcome, Prop, PropMap, Refusal, Truth, UnresolvedWitness,
};
use truck_base::pred::{orient2d, CertifiedPred, Orientation};
use truck_evidence::{Box3, EnclosureCurve, Interval};
use truck_geometry::arrange::{arrange, ArrRegion, Arrangement};
use truck_geometry::recognize::{
    recognize_curve, recognize_surface, CanonicalCarrier, CanonicalCarrierWitness, CanonicalSurface,
};
use truck_geotrait::{BoundedCurve, ParametricCurve, Transformed};
// The generic topology structs: the packet's `Solid<Point3, Curve, Surface>`
// spellings, not the crate's zero-argument prelude aliases.
use truck_topology::{Face, Solid};

/// The number of samples used to polygonize a circle loop for the material
/// representative / containment predicates.
const CIRCLE_SAMPLES: usize = 32;

// ---------------------------------------------------------------------------
// D2 — the certified bounding box.
// ---------------------------------------------------------------------------

/// The certified axis-aligned bounding box of a solid.
///
/// The accumulator is the landed [`BoundingBox`]; the work is the per-face
/// certified derivation: each face is lifted with `recognize_surface`, and its
/// box is derived by carrier:
///
/// - `Plane` face → hull of its boundary edges' enclosures (sound: a compact
///   planar region's extreme points lie on its boundary);
/// - `Cylinder` face → hull of its boundary edges' enclosures (the wall's
///   extreme xy is achieved on the rims — the radius is constant in `v` — and
///   its z-extent is bracketed by the rim circles);
/// - `Sphere` face → the full carrier box `[c−r, c+r]³` (a cap's pole is off
///   its boundary, so the hull rule would be unsound — this is why the sphere
///   arm exists);
/// - `Cone` face → hull of its boundary edges' enclosures plus the apex point;
/// - `Torus` and `Placed` carriers → `UnsupportedEnvelope(NonCanonicalCarrier)`
///   (P1 emits and consumes bare canonical carriers only; `Torus` is Tier 2).
///
/// The budget is taken for API stability and spent NOT AT ALL: this operation
/// performs no subdivision and no Newton work — every box below is closed-form
/// on the canonical carriers, and the caller's ledger is returned untouched in
/// the certificate.
pub fn solid_bounding_box(
    solid: &Solid<Point3, Curve, Surface>,
    budget: &mut Budget,
) -> Outcome<BoundingBox<Point3>> {
    let mut hull = BoundingBox::new();
    for face in solid.face_iter() {
        hull += face_bounding_box(face)?;
    }
    if hull.is_empty() {
        return Err(Refusal::Empty);
    }
    let mut props = PropMap::new();
    props.set(Prop::AnalyticCarrier, Truth::True);
    Ok(Certified::new(
        hull,
        Certificate {
            props,
            method: Method::Interval,
            budget_left: *budget,
            margin: Margin::UNBOUNDED,
            modulus: Modulus::Unbounded,
        },
    ))
}

/// The box of one face, derived from its recognized canonical carrier.
fn face_bounding_box(face: &Face<Point3, Curve, Surface>) -> Result<BoundingBox<Point3>, Refusal> {
    let surface = face.surface();
    match recognize_surface(&surface) {
        CanonicalCarrierWitness::Unrecognized => Err(Refusal::UnsupportedEnvelope(
            EnvelopeCase::NonCanonicalCarrier,
        )),
        CanonicalCarrierWitness::ExactCanonical { carrier, .. }
        | CanonicalCarrierWitness::Derived { carrier, .. } => face_box_from_carrier(face, &carrier),
    }
}

/// The per-carrier box rule for one face.
fn face_box_from_carrier(
    face: &Face<Point3, Curve, Surface>,
    carrier: &CanonicalCarrier,
) -> Result<BoundingBox<Point3>, Refusal> {
    let CanonicalCarrier::Surface(canonical) = carrier else {
        // A surface witness cannot carry a curve carrier; refuse closed.
        return Err(Refusal::UnsupportedEnvelope(
            EnvelopeCase::NonCanonicalCarrier,
        ));
    };
    match canonical {
        CanonicalSurface::Plane(_) | CanonicalSurface::Cylinder(_) => boundary_hull(face),
        CanonicalSurface::Sphere(sphere) => {
            let c = sphere.center();
            let r = sphere.radius();
            let mut hull = BoundingBox::new();
            hull.push(Point3::new(c.x - r, c.y - r, c.z - r));
            hull.push(Point3::new(c.x + r, c.y + r, c.z + r));
            Ok(hull)
        }
        CanonicalSurface::Cone(cone) => {
            let mut hull = boundary_hull(face)?;
            hull.push(cone.apex());
            Ok(hull)
        }
        // Tier 2, and P1 consumes bare canonical carriers only.
        CanonicalSurface::Torus(_) | CanonicalSurface::Placed(_) => Err(
            Refusal::UnsupportedEnvelope(EnvelopeCase::NonCanonicalCarrier),
        ),
    }
}

/// The hull of the face's boundary edges' enclosures, traversed over the
/// STORED wires: `absolute_boundaries` is the wire set the constructor
/// received, while `boundaries` re-inverts it by the face's orientation flag —
/// irrelevant for a box (the session-38 naming trap).
fn boundary_hull(face: &Face<Point3, Curve, Surface>) -> Result<BoundingBox<Point3>, Refusal> {
    let mut hull = BoundingBox::new();
    for wire in face.absolute_boundaries() {
        for edge in wire.edge_iter() {
            let enclosure = edge_enclosure(&edge.curve())?;
            push_box3(&mut hull, &enclosure)?;
        }
    }
    if hull.is_empty() {
        return Err(Refusal::Empty);
    }
    Ok(hull)
}

/// Pushes an enclosure box's bounds into the accumulator. Non-finite bounds
/// (an EMPTY interval from a failed construction) refuse the derivation
/// rather than silently dropping a component.
fn push_box3(hull: &mut BoundingBox<Point3>, b: &Box3) -> Result<(), Refusal> {
    let (x0, x1) = (b.x.inf(), b.x.sup());
    let (y0, y1) = (b.y.inf(), b.y.sup());
    let (z0, z1) = (b.z.inf(), b.z.sup());
    if !(x0.is_finite()
        && x1.is_finite()
        && y0.is_finite()
        && y1.is_finite()
        && z0.is_finite()
        && z1.is_finite())
    {
        return Err(Refusal::Empty);
    }
    hull.push(Point3::new(x0, y0, z0));
    hull.push(Point3::new(x1, y1, z1));
    Ok(())
}

/// The certified 3-D enclosure of one boundary edge over its own bounded
/// parameter range: `EnclosureCurve::enclose` on the landed per-carrier impls.
fn edge_enclosure(curve: &Curve) -> Result<Box3, Refusal> {
    match curve {
        Curve::Line(line) => {
            // `Line(p0, p1)` is the segment: its parameter range is exactly
            // the edge's bounded span.
            let (t0, t1) = line.range_tuple();
            let tt = interval_pair(t0, t1)?;
            Ok(line.enclose(tt))
        }
        Curve::Circle(placed) => {
            let (t0, t1) = placed.range_tuple();
            let tt = interval_pair(t0, t1)?;
            Ok(enclose_placed_circle(placed, tt))
        }
        // P1 emits and consumes bare canonical carriers only.
        Curve::BSplineCurve(_)
        | Curve::NurbsCurve(_)
        | Curve::IntersectionCurve(_)
        | Curve::SpineFrameCurve(_) => Err(Refusal::UnsupportedEnvelope(
            EnvelopeCase::NonCanonicalCarrier,
        )),
    }
}

/// The parameter interval of a bounded range, refusing a malformed one.
fn interval_pair(t0: f64, t1: f64) -> Result<Interval, Refusal> {
    Interval::try_from((t0, t1)).map_err(|_| Refusal::Empty)
}

/// The certified enclosure of a placed canonical circle: the unit circle's
/// interval-trigonometry enclosure composed with the placement matrix. The
/// composition is affine, so per-coordinate interval arithmetic is exact up to
/// outward rounding (BG-ENC-001: over-estimation only). `TrimmedCurve` does
/// not remap the parameter, so the placement's argument IS the angle.
fn enclose_placed_circle(
    placed: &Processor<TrimmedCurve<UnitCircle<Point3>>, Matrix4>,
    tt: Interval,
) -> Box3 {
    let local = UnitCircle::<Point3>::new().enclose(tt);
    let m = *placed.transform();
    // Row r of the affine map is (m.x[r], m.y[r], m.z[r]) with translation
    // m.w[r]: the xyz components of each matrix column are the linear part.
    Box3 {
        x: local.x * at(m.x.x) + local.y * at(m.y.x) + local.z * at(m.z.x) + at(m.w.x),
        y: local.x * at(m.x.y) + local.y * at(m.y.y) + local.z * at(m.z.y) + at(m.w.y),
        z: local.x * at(m.x.z) + local.y * at(m.y.z) + local.z * at(m.z.z) + at(m.w.z),
    }
}

/// A degenerate interval from a runtime `f64`. Finite coordinates always
/// construct; a NaN widens to the empty interval rather than panicking (H-1).
fn at(x: f64) -> Interval {
    Interval::try_from((x, x)).unwrap_or(Interval::EMPTY)
}

// ---------------------------------------------------------------------------
// D3 — the similarity fold.
// ---------------------------------------------------------------------------

/// The similarity fold: one affine map carried over the whole `Vertex`→`Solid`
/// chain by the landed `Mapped` impls. The point closure is the affine map of
/// `Point3`; the curve/surface closures are `Transformed::transformed` (the
/// landed canonical dispatch: planes carry bare, analytic carriers are placed
/// exactly under non-identity linear parts, BG-CE-006-r2).
#[derive(Clone, Copy, Debug)]
struct SimilarityFold {
    mat: Matrix4,
}

impl crate::GeometricMapping<Point3> for SimilarityFold {
    fn mapping(self) -> impl Fn(&Point3) -> Point3 {
        move |p: &Point3| self.mat.transform_point(*p)
    }
}

impl crate::GeometricMapping<Curve> for SimilarityFold {
    fn mapping(self) -> impl Fn(&Curve) -> Curve {
        move |c: &Curve| c.transformed(self.mat)
    }
}

impl crate::GeometricMapping<Surface> for SimilarityFold {
    fn mapping(self) -> impl Fn(&Surface) -> Surface {
        move |s: &Surface| s.transformed(self.mat)
    }
}

/// Translates `solid` by `t`: the similarity fold with a pure-translation
/// matrix. The topology STRUCTURE is identical (same face/edge/wire counts,
/// same shared-edge identity pattern `Mapped` already preserves); every vertex
/// point shifts exactly.
pub fn translate_solid(
    solid: &Solid<Point3, Curve, Surface>,
    t: Vector3,
) -> Outcome<Solid<Point3, Curve, Surface>> {
    fold_solid(
        solid,
        SimilarityFold {
            mat: Matrix4::from_translation(t),
        },
    )
}

/// Scales `solid` uniformly about the origin by `s`. Non-finite or
/// non-positive `s` refuses `Refusal::Empty` (the extrude non-positive-height
/// convention).
pub fn uniform_scale_solid(
    solid: &Solid<Point3, Curve, Surface>,
    s: f64,
) -> Outcome<Solid<Point3, Curve, Surface>> {
    if !s.is_finite() || s <= 0.0 {
        return Err(Refusal::Empty);
    }
    fold_solid(
        solid,
        SimilarityFold {
            mat: Matrix4::from_scale(s),
        },
    )
}

/// Mirrors `solid` across an axis-aligned plane.
///
/// ONLY axis-aligned mirror planes are accepted: the plane's normal in
/// {±x, ±y, ±z}, the plane through any point `c` (the axis coordinate maps
/// x ↦ 2cᵢ − x). Anything else would emit `Placed` carriers — refuse
/// `UnsupportedEnvelope(NonCanonicalCarrier)`; no other refusal arm is
/// invented for this.
pub fn mirror_solid(
    solid: &Solid<Point3, Curve, Surface>,
    plane: &Plane,
) -> Outcome<Solid<Point3, Curve, Surface>> {
    let n = plane.normal();
    let c = plane.origin();
    let mat = match axis_aligned_normal(n) {
        Some(0) => Matrix4 {
            x: Vector4::new(-1.0, 0.0, 0.0, 0.0),
            y: Vector4::new(0.0, 1.0, 0.0, 0.0),
            z: Vector4::new(0.0, 0.0, 1.0, 0.0),
            w: Vector4::new(2.0 * c.x, 0.0, 0.0, 1.0),
        },
        Some(1) => Matrix4 {
            x: Vector4::new(1.0, 0.0, 0.0, 0.0),
            y: Vector4::new(0.0, -1.0, 0.0, 0.0),
            z: Vector4::new(0.0, 0.0, 1.0, 0.0),
            w: Vector4::new(0.0, 2.0 * c.y, 0.0, 1.0),
        },
        Some(2) => Matrix4 {
            x: Vector4::new(1.0, 0.0, 0.0, 0.0),
            y: Vector4::new(0.0, 1.0, 0.0, 0.0),
            z: Vector4::new(0.0, 0.0, -1.0, 0.0),
            w: Vector4::new(0.0, 0.0, 2.0 * c.z, 1.0),
        },
        // A non-axis-aligned mirror plane would emit `Placed` carriers.
        _ => {
            return Err(Refusal::UnsupportedEnvelope(
                EnvelopeCase::NonCanonicalCarrier,
            ))
        }
    };
    fold_solid(solid, SimilarityFold { mat })
}

/// Whether the normal is exactly ±x, ±y, or ±z, and which axis.
fn axis_aligned_normal(n: Vector3) -> Option<usize> {
    if n.y == 0.0 && n.z == 0.0 && (n.x == 1.0 || n.x == -1.0) {
        Some(0)
    } else if n.x == 0.0 && n.z == 0.0 && (n.y == 1.0 || n.y == -1.0) {
        Some(1)
    } else if n.x == 0.0 && n.y == 0.0 && (n.z == 1.0 || n.z == -1.0) {
        Some(2)
    } else {
        None
    }
}

/// Rotates `solid` about the axis through `axis_point` with direction
/// `axis_dir` (need not be unit; normalized internally) by `angle` radians.
/// The similarity fold with the rigid rotation matrix.
///
/// The rigid rotation `R` is the Rodrigues form about the normalized axis,
/// composed as one matrix: translate the axis point to the origin, rotate,
/// translate back. The fold's emission rule (BG-CE-006-r2) applies
/// unchanged: planes carry bare under any affine map, curved analytic
/// carriers place under a non-identity linear part. A zero-length `axis_dir`
/// or a non-finite `angle` refuses `Refusal::Empty`.
pub fn rotate_solid(
    solid: &Solid<Point3, Curve, Surface>,
    axis_point: Point3,
    axis_dir: Vector3,
    angle: f64,
) -> Outcome<Solid<Point3, Curve, Surface>> {
    if !angle.is_finite() {
        return Err(Refusal::Empty);
    }
    let len = axis_dir.magnitude();
    if !len.is_finite() || len == 0.0 {
        return Err(Refusal::Empty);
    }
    let axis = axis_dir / len;
    let rotation = Matrix4::from_axis_angle(axis, Rad(angle));
    let mat = Matrix4::from_translation(axis_point.to_vec())
        * rotation
        * Matrix4::from_translation(-axis_point.to_vec());
    fold_solid(solid, SimilarityFold { mat })
}

/// Mirrors `solid` about the plane through `plane_point` with normal
/// `plane_normal` (need not be unit; normalized internally). The fold with
/// the Householder reflection `I - 2nn^T` composed with the translation;
/// det < 0 exactly like the landed axis-aligned mirror.
///
/// The reflection is the exact form `I - 2nn^T / (n·n)` — for a unit normal
/// this is `I - 2nn^T`, and the un-normalized form only needs the dot `n·n`
/// (no square root), so a dyadic normal like (1, 1, 0) with `n·n = 2` keeps
/// the whole matrix exactly dyadic. A zero-length normal refuses
/// `Refusal::Empty`.
pub fn mirror_about_plane(
    solid: &Solid<Point3, Curve, Surface>,
    plane_point: Point3,
    plane_normal: Vector3,
) -> Outcome<Solid<Point3, Curve, Surface>> {
    let nn = plane_normal.dot(plane_normal);
    if !nn.is_finite() || nn == 0.0 {
        return Err(Refusal::Empty);
    }
    let two_over_nn = 2.0 / nn;
    let reflection = Matrix4 {
        x: Vector4::new(
            1.0 - two_over_nn * plane_normal.x * plane_normal.x,
            -two_over_nn * plane_normal.y * plane_normal.x,
            -two_over_nn * plane_normal.z * plane_normal.x,
            0.0,
        ),
        y: Vector4::new(
            -two_over_nn * plane_normal.x * plane_normal.y,
            1.0 - two_over_nn * plane_normal.y * plane_normal.y,
            -two_over_nn * plane_normal.z * plane_normal.y,
            0.0,
        ),
        z: Vector4::new(
            -two_over_nn * plane_normal.x * plane_normal.z,
            -two_over_nn * plane_normal.y * plane_normal.z,
            1.0 - two_over_nn * plane_normal.z * plane_normal.z,
            0.0,
        ),
        w: Vector4::new(0.0, 0.0, 0.0, 1.0),
    };
    let mat = Matrix4::from_translation(plane_point.to_vec())
        * reflection
        * Matrix4::from_translation(-plane_point.to_vec());
    fold_solid(solid, SimilarityFold { mat })
}

/// Applies the fold over the whole `Vertex`→`Solid` chain and certifies the
/// result.
fn fold_solid(
    solid: &Solid<Point3, Curve, Surface>,
    fold: SimilarityFold,
) -> Outcome<Solid<Point3, Curve, Surface>> {
    // The landed `Mapped` impl, called as a trait method: the inherent
    // `Solid::mapped` (three raw closures) would shadow it in method position.
    let mut mapped: Solid<Point3, Curve, Surface> = Mapped::mapped(solid, fold);
    // Mirror parity rule: an improper affine map (det < 0) reverses each
    // surface's normal, so every face of every shell has its orientation flag
    // flipped for the shell to stay outward-consistent. Deterministic;
    // `Solid::try_new` below is the acceptance gate either way.
    if fold.mat.determinant() < 0.0 {
        mapped.not();
    }
    // Defensive certificate: the fold preserves the carrier set, so every
    // transformed carrier must still be recognized; anything `Unrecognized`
    // refuses `UnsupportedEnvelope(NonCanonicalCarrier)`. (For these three
    // ops it cannot fire: planes carry bare under any affine map and the
    // analytic carriers are placed exactly by the landed canonical rules —
    // this gate is the fold's certificate that the carrier set was preserved.)
    certify_carriers(&mapped)?;
    // The acceptance gate: the mapped (and parity-flipped) shell must still
    // be a closed, connected, manifold solid. A refusal here is a typed
    // refusal about the transform, never a panic.
    let shells = mapped.into_boundaries();
    let mapped_solid = match Solid::try_new(shells) {
        Ok(mapped_solid) => mapped_solid,
        Err(_) => {
            return Err(Refusal::Contradictory(ContradictionWitness {
                prop: Prop::CoedgePairing,
                left: Truth::True,
                right: Truth::False,
            }));
        }
    };
    let mut props = PropMap::new();
    props.set(Prop::CoedgePairing, Truth::True);
    props.set(Prop::VertexLink, Truth::True);
    props.set(Prop::AnalyticCarrier, Truth::True);
    Ok(Certified::new(
        mapped_solid,
        Certificate {
            props,
            method: Method::Exact,
            budget_left: Budget::new(0, 0, 0),
            margin: Margin::UNBOUNDED,
            modulus: Modulus::Unbounded,
        },
    ))
}

/// The fold's carrier-set certificate: every face surface and edge curve of
/// the mapped solid is still recognized by the structural recognizer.
fn certify_carriers(solid: &Solid<Point3, Curve, Surface>) -> Result<(), Refusal> {
    for face in solid.face_iter() {
        let surface = face.surface();
        if matches!(
            recognize_surface(&surface),
            CanonicalCarrierWitness::Unrecognized
        ) {
            return Err(Refusal::UnsupportedEnvelope(
                EnvelopeCase::NonCanonicalCarrier,
            ));
        }
    }
    for edge in solid.edge_iter() {
        let curve = edge.curve();
        if matches!(
            recognize_curve(&curve),
            CanonicalCarrierWitness::Unrecognized
        ) {
            return Err(Refusal::UnsupportedEnvelope(
                EnvelopeCase::NonCanonicalCarrier,
            ));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// D4 — planar face construction.
// ---------------------------------------------------------------------------

/// Constructs planar faces on the z = 0 plane from a profile.
///
/// v1 frame: the profile lies in the z = 0 plane — every `Line` endpoint with
/// z ≠ 0, every `Circle` whose axis is not ±z or whose center has z ≠ 0, and
/// any non-canonical carrier refuses `UnsupportedEnvelope(NonCanonicalCarrier)`.
///
/// `arrange(profile, None)` (the landed S1) determines the material regions by
/// the session-28 containment rule: bounded `winding == 1` regions not
/// strictly inside another bounded `winding == 1` region's boundary cycle. One
/// face is produced per material region (build123d semantics: multiple
/// disjoint loops produce multiple faces), through the landed extrude cap
/// recipe: explicitly constructed shared vertices, per-cycle wires, closed
/// circle edges built with `Edge::new_unchecked` (the session-28
/// `NotSimpleWire` trap), and the annulus-with-holes shape carrying two
/// boundary wires with NO seam edges. The face's surface is the `Plane` through
/// z = 0 (the landed extrude cap convention), stored uninverted so a CCW outer
/// loop carries the +z normal.
pub fn make_face(profile: &[Curve]) -> Outcome<Vec<Face<Point3, Curve, Surface>>> {
    for curve in profile {
        check_profile_curve(curve)?;
    }
    let ok = arrange(profile, None)?;
    let arrangement = ok.value;
    let materials = material_regions(profile, &arrangement)?;
    if materials.is_empty() {
        // The session-28 material rule selects nothing here: there is nothing
        // to certify.
        return Err(Refusal::Empty);
    }
    let mut faces = Vec::with_capacity(materials.len());
    for region in materials {
        faces.push(region_face(region, profile, &arrangement)?);
    }
    let mut props = PropMap::new();
    props.set(Prop::AnalyticCarrier, Truth::True);
    Ok(Certified::new(
        faces,
        Certificate {
            props,
            method: Method::Exact,
            budget_left: Budget::new(0, 0, 0),
            margin: Margin::UNBOUNDED,
            modulus: Modulus::Unbounded,
        },
    ))
}

/// The v1 frame check: the profile curve lies in the z = 0 plane.
fn check_profile_curve(curve: &Curve) -> Result<(), Refusal> {
    match curve {
        Curve::Line(Line(a, b)) => {
            if a.z != 0.0 || b.z != 0.0 {
                return Err(Refusal::UnsupportedEnvelope(
                    EnvelopeCase::NonCanonicalCarrier,
                ));
            }
            Ok(())
        }
        Curve::Circle(placed) => {
            let m = *placed.transform();
            // The circle's axis: the cross of the linear images of x̂ and ŷ
            // (the xyz components of the matrix's x and y columns).
            let axis = Vector3::new(m.x.x, m.x.y, m.x.z).cross(Vector3::new(m.y.x, m.y.y, m.y.z));
            let center = Point3::new(m.w.x, m.w.y, m.w.z);
            if !(axis.x == 0.0 && axis.y == 0.0 && axis.z != 0.0) || center.z != 0.0 {
                return Err(Refusal::UnsupportedEnvelope(
                    EnvelopeCase::NonCanonicalCarrier,
                ));
            }
            Ok(())
        }
        // P1 emits and consumes bare canonical carriers only.
        Curve::BSplineCurve(_)
        | Curve::NurbsCurve(_)
        | Curve::IntersectionCurve(_)
        | Curve::SpineFrameCurve(_) => Err(Refusal::UnsupportedEnvelope(
            EnvelopeCase::NonCanonicalCarrier,
        )),
    }
}

/// The session-28 material regions: bounded `winding == 1` regions not
/// strictly inside another bounded `winding == 1` region's boundary cycle.
fn material_regions<'a>(
    profile: &[Curve],
    arrangement: &'a Arrangement,
) -> Result<Vec<&'a ArrRegion>, Refusal> {
    let mut material = Vec::new();
    for (idx, region) in arrangement.regions.iter().enumerate() {
        if !region.bounded || region.winding != 1 {
            continue;
        }
        let rep = match region_representative(region, profile, arrangement) {
            Some(p) => p,
            None => return Err(Refusal::Empty),
        };
        let inside_other = arrangement
            .regions
            .iter()
            .enumerate()
            .any(|(other_idx, other)| {
                other_idx != idx
                    && other.bounded
                    && other.winding == 1
                    && other
                        .boundaries
                        .iter()
                        .any(|cycle| point_in_cycle(rep, cycle, profile, arrangement))
            });
        if inside_other {
            continue;
        }
        material.push(region);
    }
    Ok(material)
}

/// One planar face on the z = 0 plane for one material region: the landed
/// extrude cap recipe. The face is stored UNINVERTED — its effective normal is
/// the plane's +z, the build123d face convention.
fn region_face(
    region: &ArrRegion,
    profile: &[Curve],
    arrangement: &Arrangement,
) -> Result<Face<Point3, Curve, Surface>, Refusal> {
    // The distinct arrangement vertices on the region's boundary cycles: one
    // `Vertex::new(point)` per arrangement vertex (z = 0). Distinct instances
    // for coincident geometric points would leave the wires disjoint from
    // their own cycles.
    let mut v_indices: Vec<usize> = Vec::new();
    for cycle in &region.boundaries {
        for &h in cycle {
            let he = arrangement.half_edges.get(h).ok_or(Refusal::Empty)?;
            if !v_indices.contains(&he.origin) {
                v_indices.push(he.origin);
            }
        }
    }
    let mut vertices: HashMap<usize, Vertex> = HashMap::new();
    for &v_idx in &v_indices {
        let point = arrangement.vertices.get(v_idx).ok_or(Refusal::Empty)?.point;
        vertices.insert(v_idx, Vertex::new(point));
    }
    // One explicitly constructed wire per boundary cycle (outer first, holes
    // after, as the arrangement traced them).
    let mut wires = Vec::new();
    for cycle in &region.boundaries {
        wires.push(Wire::from(cycle_edges(
            cycle,
            profile,
            arrangement,
            &vertices,
        )?));
    }
    planar_face(wires)
}

/// The boundary edges of one cycle in cycle order, on the z = 0 plane: a line
/// piece gets `Curve::Line(Line(p0, p1))` between the shared vertex points; a
/// circle piece keeps the profile's `Curve::Circle` processor. Closed circle
/// edges are built with `Edge::new_unchecked` — the self-loop IS the seam, and
/// the sanctioned construction is the session-28 `NotSimpleWire` trap.
fn cycle_edges(
    cycle: &[usize],
    profile: &[Curve],
    arrangement: &Arrangement,
    vertices: &HashMap<usize, Vertex>,
) -> Result<Vec<Edge>, Refusal> {
    let n = cycle.len();
    if n == 0 {
        return Err(Refusal::Empty);
    }
    let mut edges = Vec::with_capacity(n);
    for i in 0..n {
        let h_i = *cycle.get(i).ok_or(Refusal::Empty)?;
        let h_next = *cycle.get((i + 1) % n).ok_or(Refusal::Empty)?;
        let he_i = arrangement.half_edges.get(h_i).ok_or(Refusal::Empty)?;
        let he_next = arrangement.half_edges.get(h_next).ok_or(Refusal::Empty)?;
        let v0 = vertices.get(&he_i.origin).ok_or(Refusal::Empty)?;
        let v1 = vertices.get(&he_next.origin).ok_or(Refusal::Empty)?;
        let circle_piece = matches!(profile.get(he_i.curve), Some(Curve::Circle(_)));
        let curve = match profile.get(he_i.curve) {
            Some(Curve::Line(_)) => {
                let p0 = v0.point();
                let twin = arrangement
                    .half_edges
                    .get(he_i.twin)
                    .ok_or(Refusal::Empty)?;
                let p1 = vertices.get(&twin.origin).ok_or(Refusal::Empty)?.point();
                Curve::Line(Line(p0, p1))
            }
            Some(Curve::Circle(p)) => Curve::Circle(*p),
            _ => return Err(Refusal::Empty),
        };
        let edge = if circle_piece {
            Edge::new_unchecked(v0, v1, curve)
        } else {
            Edge::try_new(v0, v1, curve).map_err(|_| Refusal::Empty)?
        };
        edges.push(edge);
    }
    Ok(edges)
}

/// The z = 0 plane through the origin with the +x/+y basis — the landed
/// extrude cap convention.
fn plane_z0() -> Surface {
    Surface::Plane(Plane::new(
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(1.0, 0.0, 0.0),
        Point3::new(0.0, 1.0, 0.0),
    ))
}

/// The single-region face builder: explicitly constructed boundary wires on
/// the z = 0 plane, validated through `Face::try_new`.
fn planar_face(wires: Vec<Wire>) -> Result<Face<Point3, Curve, Surface>, Refusal> {
    Face::try_new(wires, plane_z0()).map_err(|_| Refusal::Empty)
}

// ---------------------------------------------------------------------------
// D5 — the 2-D convex hull.
// ---------------------------------------------------------------------------

/// The certified 2-D convex hull of z = 0 points, as one planar face.
///
/// All points must have z = 0 (else `NonCanonicalCarrier`). The monotone-chain
/// hull uses the landed exact predicate `orient2d`; any
/// `CertifiedPred::Unresolved` result refuses
/// `NumericallyUnresolved { witness: UncertifiedContainment }` (it cannot fire
/// on dyadic test witnesses; it is the escalation contract). Fewer than three
/// distinct points, or a hull of zero area (all collinear), refuses
/// `Refusal::Collapsed` with the knife-edge reason: the certified collapse of
/// a planar region to a line is a zero-area degeneracy (lfs = 0). The CCW
/// closed wire of `Line` edges is finished through the same single-region face
/// builder `make_face` uses, so a CCW hull carries the +z normal.
pub fn make_hull(points: &[Point3]) -> Outcome<Face<Point3, Curve, Surface>> {
    for p in points {
        if p.z != 0.0 {
            return Err(Refusal::UnsupportedEnvelope(
                EnvelopeCase::NonCanonicalCarrier,
            ));
        }
    }
    let hull = convex_hull_ccw(points)?;
    if hull.len() < 3 {
        // Fewer than 3 distinct points, or a hull of zero area (all
        // collinear): the exact predicates certify the collapse.
        return Err(Refusal::Collapsed(
            Collapse {
                reason: CollapseReason::KnifeEdge,
            },
            Certificate {
                props: PropMap::new(),
                method: Method::Exact,
                budget_left: Budget::new(0, 0, 0),
                margin: Margin::UNBOUNDED,
                modulus: Modulus::Unbounded,
            },
        ));
    }
    let mut vertices = Vec::with_capacity(hull.len());
    for p in &hull {
        vertices.push(Vertex::new(*p));
    }
    let n = hull.len();
    let mut edges = Vec::with_capacity(n);
    for i in 0..n {
        let v0 = vertices.get(i).ok_or(Refusal::Empty)?;
        let v1 = vertices.get((i + 1) % n).ok_or(Refusal::Empty)?;
        let p0 = v0.point();
        let p1 = v1.point();
        edges.push(Edge::try_new(v0, v1, Curve::Line(Line(p0, p1))).map_err(|_| Refusal::Empty)?);
    }
    let face = planar_face(vec![Wire::from(edges)])?;
    let mut props = PropMap::new();
    props.set(Prop::AnalyticCarrier, Truth::True);
    Ok(Certified::new(
        face,
        Certificate {
            props,
            method: Method::Exact,
            budget_left: Budget::new(0, 0, 0),
            margin: Margin::UNBOUNDED,
            modulus: Modulus::Unbounded,
        },
    ))
}

/// The monotone-chain convex hull of the points' (x, y) projections, in CCW
/// order. Duplicate points are exact duplicates; the result of a
/// collinear-or-degenerate input has fewer than 3 vertices.
fn convex_hull_ccw(points: &[Point3]) -> Result<Vec<Point3>, Refusal> {
    let mut pts: Vec<Point3> = points.to_vec();
    pts.sort_by(|a, b| {
        let by_x = a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal);
        by_x.then(a.y.partial_cmp(&b.y).unwrap_or(std::cmp::Ordering::Equal))
    });
    pts.dedup();
    if pts.len() < 3 {
        return Ok(Vec::new());
    }
    // Lower hull over the ascending order, then upper hull over the reverse:
    // a turn that is not proven counterclockwise (clockwise or collinear)
    // pops the middle point. The concatenation minus the shared endpoints is
    // the CCW hull.
    let lower = half_hull(&pts)?;
    let upper = half_hull(&pts.iter().rev().copied().collect::<Vec<Point3>>())?;
    let mut hull = Vec::with_capacity(lower.len() + upper.len());
    for (i, p) in lower.iter().enumerate() {
        if i + 1 < lower.len() {
            hull.push(*p);
        }
    }
    for (i, p) in upper.iter().enumerate() {
        if i + 1 < upper.len() {
            hull.push(*p);
        }
    }
    Ok(hull)
}

/// One monotone-chain pass. The exact predicate decides every turn; an
/// unresolved one refuses (the escalation contract).
fn half_hull(pts: &[Point3]) -> Result<Vec<Point3>, Refusal> {
    let mut hull: Vec<Point3> = Vec::new();
    for &p in pts {
        while hull.len() >= 2 {
            let b = match hull.pop() {
                Some(b) => b,
                None => return Err(Refusal::Empty),
            };
            let a = match hull.last() {
                Some(a) => *a,
                None => return Err(Refusal::Empty),
            };
            match orient2d(pt2(a), pt2(b), pt2(p)) {
                CertifiedPred::Proven(Orientation::CounterClockwise) => {
                    hull.push(b);
                    break;
                }
                // Clockwise or collinear: keep the middle point popped.
                CertifiedPred::Proven(_) => {}
                CertifiedPred::Unresolved(_) => {
                    return Err(Refusal::NumericallyUnresolved {
                        spent: Budget::new(0, 0, 0),
                        witness: UnresolvedWitness::UncertifiedContainment,
                    });
                }
            }
        }
        hull.push(p);
    }
    Ok(hull)
}

/// The 2-D (x, y) projection of a 3-D point.
fn pt2(p: Point3) -> Point2 {
    Point2::new(p.x, p.y)
}

// ---------------------------------------------------------------------------
// The arrangement-side helpers (the session-28 material machinery, local to
// this module: `extrude.rs`'s copies are private by design and stay private).
// ---------------------------------------------------------------------------

/// A representative point of the region's material: strictly inside the outer
/// boundary cycle and strictly outside every hole cycle.
fn region_representative(
    region: &ArrRegion,
    profile: &[Curve],
    arrangement: &Arrangement,
) -> Option<Point2> {
    let outer = region.boundaries.first()?;
    let outer_poly = cycle_polygon(outer, profile, arrangement);
    if outer_poly.is_empty() {
        return None;
    }
    let holes: Vec<Vec<Point2>> = region
        .boundaries
        .iter()
        .skip(1)
        .map(|c| cycle_polygon(c, profile, arrangement))
        .collect();
    let mut candidates = Vec::new();
    if let Some(c) = polygon_centroid(&outer_poly) {
        candidates.push(c);
    }
    // Inward-nudged edge midpoints (the outer cycle is CCW, so the left normal
    // of each edge points into the region).
    let mut first: Option<Point2> = None;
    let mut prev: Option<Point2> = None;
    for cur in &outer_poly {
        if let Some(a) = prev {
            push_left_midpoint(a, *cur, &mut candidates);
        }
        if first.is_none() {
            first = Some(*cur);
        }
        prev = Some(*cur);
    }
    if let (Some(a), Some(b)) = (prev, first) {
        push_left_midpoint(a, b, &mut candidates);
    }
    if let Some((lo, hi)) = bbox_limits(&outer_poly) {
        const GRID: usize = 8;
        for gi in 0..=GRID {
            for gj in 0..=GRID {
                candidates.push(Point2::new(
                    lo.x + (hi.x - lo.x) * (gi as f64 / GRID as f64),
                    lo.y + (hi.y - lo.y) * (gj as f64 / GRID as f64),
                ));
            }
        }
    }
    for c in candidates {
        if point_in_poly(c, &outer_poly) {
            let in_hole = holes.iter().any(|h| point_in_poly(c, h));
            if !in_hole {
                return Some(c);
            }
        }
    }
    None
}

/// Pushes the midpoint of `a→b` nudged along the left normal by the
/// representation tolerance onto `candidates`.
fn push_left_midpoint(a: Point2, b: Point2, candidates: &mut Vec<Point2>) {
    let mid = Point2::new((a.x + b.x) * 0.5, (a.y + b.y) * 0.5);
    let dir = Vector3::new(b.x - a.x, b.y - a.y, 0.0);
    let left = Vector3::new(-dir.y, dir.x, 0.0);
    let len = left.magnitude();
    if len > 0.0 {
        let nudge = 64.0 * TOLERANCE;
        candidates.push(Point2::new(
            mid.x + left.x / len * nudge,
            mid.y + left.y / len * nudge,
        ));
    }
}

/// The signed-area polygon centroid of a (not necessarily closed) polygon.
fn polygon_centroid(poly: &[Point2]) -> Option<Point2> {
    let mut iter = poly.iter();
    let first = match iter.next() {
        Some(&f) => f,
        None => return None,
    };
    let mut area = 0.0;
    let mut cx = 0.0;
    let mut cy = 0.0;
    let mut prev = first;
    for &cur in iter {
        let cross = prev.x * cur.y - prev.y * cur.x;
        area += cross;
        cx += (prev.x + cur.x) * cross;
        cy += (prev.y + cur.y) * cross;
        prev = cur;
    }
    let cross = prev.x * first.y - prev.y * first.x;
    area += cross;
    cx += (prev.x + first.x) * cross;
    cy += (prev.y + first.y) * cross;
    if area == 0.0 {
        return None;
    }
    Some(Point2::new(cx / (3.0 * area), cy / (3.0 * area)))
}

/// The polygonized parameter-space loop of a boundary cycle: each half-edge is
/// sampled over its parameter window (lines at their endpoints, arcs finely).
fn cycle_polygon(cycle: &[usize], profile: &[Curve], arrangement: &Arrangement) -> Vec<Point2> {
    let mut out = Vec::new();
    for &h in cycle {
        let he = match arrangement.half_edges.get(h) {
            Some(he) => he,
            None => continue,
        };
        let curve = match profile.get(he.curve) {
            Some(c) => c,
            None => continue,
        };
        let (u0, u1) = he.u_range;
        match curve {
            Curve::Line(_) => {
                out.push(pt2(curve.subs(u0)));
                out.push(pt2(curve.subs(u1)));
            }
            Curve::Circle(_) => {
                for k in 0..=CIRCLE_SAMPLES {
                    let t = u0 + (u1 - u0) * (k as f64 / CIRCLE_SAMPLES as f64);
                    out.push(pt2(curve.subs(t)));
                }
            }
            _ => {}
        }
    }
    out
}

/// Whether the point `p` is strictly inside the polygonized cycle (nonzero
/// winding / odd parity).
fn point_in_cycle(
    p: Point2,
    cycle: &[usize],
    profile: &[Curve],
    arrangement: &Arrangement,
) -> bool {
    let poly = cycle_polygon(cycle, profile, arrangement);
    point_in_poly(p, &poly)
}

/// Even-odd point-in-polygon by horizontal ray casting, without any indexing.
fn point_in_poly(p: Point2, poly: &[Point2]) -> bool {
    let mut inside = false;
    let mut iter = poly.iter();
    let first = match iter.next() {
        Some(&f) => f,
        None => return false,
    };
    let mut prev = first;
    for &cur in iter {
        if (prev.y > p.y) != (cur.y > p.y) {
            let x_cross = (cur.x - prev.x) * (p.y - prev.y) / (cur.y - prev.y) + prev.x;
            if p.x < x_cross {
                inside = !inside;
            }
        }
        prev = cur;
    }
    if (prev.y > p.y) != (first.y > p.y) {
        let x_cross = (first.x - prev.x) * (p.y - prev.y) / (first.y - prev.y) + prev.x;
        if p.x < x_cross {
            inside = !inside;
        }
    }
    inside
}

/// The bounding-box limits of a polygon, `None` if any coordinate is non-finite.
fn bbox_limits(poly: &[Point2]) -> Option<(Point2, Point2)> {
    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for p in poly {
        min_x = min_x.min(p.x);
        max_x = max_x.max(p.x);
        min_y = min_y.min(p.y);
        max_y = max_y.max(p.y);
    }
    if !min_x.is_finite() || !min_y.is_finite() {
        return None;
    }
    Some((Point2::new(min_x, min_y), Point2::new(max_x, max_y)))
}
