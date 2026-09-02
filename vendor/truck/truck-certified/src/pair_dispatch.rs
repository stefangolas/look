//! Certified analytic surface-pair dispatch (BG-CK-P1-DISPATCH).
//!
//! The class-2 fast path: the dispatcher routes certified surface-pair
//! classes to closed-form certified contact constructions, with exact
//! predicates deciding admission and directed rounding at the evaluation
//! leaves. The landed 2D pipeline (`formal/intersection.rs`) is the
//! implementation model; the landed 2D result shape
//! (`PairIntersectionResult` → `PairContactResult`, `formal/contact.rs`) is
//! the shape this module's result mirrors.
//!
//! # Admitted mass (provenance: `docs/CERTIFIED_PREVALENCE.md`)
//!
//! Corpus pair masses decide admission (the plan's own mass-driven doctrine —
//! prevalence decides). The arms THIS packet lands carry:
//!
//! | arm | corpus count |
//! |---|---|
//! | cylinder~plane | 37,361 |
//! | plane~plane | 26,274 |
//! | cylinder~cylinder (coaxial/parallel subset) | 5,354 |
//! | plane~sphere | 281 |
//! | sphere~sphere | 126 |
//!
//! 64,042 pairs before the coaxial/parallel subset of cylinder~cylinder
//! counts — ~62% of the analytic mass. The cone and torus arms (plane~cone
//! 8,379; plane~torus 5,385; cylinder~sphere 3,249) are certifiable only in
//! special geometric positions and book as **BG-CK-P1-DISPATCH-2** after
//! FLOOR's first measurement (velocity-recalibration doctrine). Classes OUTSIDE
//! the admitted set refuse typed — never swallowed, never downgraded
//! (the no-silent-downgrade doctrine). Zero mesh-derived intersection
//! polylines in the certified path (F1: certified loci, never approximations).
//! Chart (pcurve) emission is NOT in this packet — no Phase-1 consumer needs
//! it (FLOOR measures certify/refuse, it does not consume pcurves) — and books
//! with Phase 3's boolean core as a follow-up.
//!
//! # Pre-made decisions (packet tags; do not relitigate)
//!
//! **H-1.** The crate-level `#![deny(clippy::unwrap_used)]` covers this
//! module: no `unwrap`/`expect`/`panic!`, and no module-level `allow`.
//! `Option`s unpack through `ok_or(...)?` into named refusals.
//!
//! **D-reuse — the refusal class is the LANDED enum, widened by exactly one
//! named variant.** [`formal::intersection::PairUnsupported`] (Overlap /
//! UnrelatedTangency / CoincidentCircles) is the shared pair-refusal witness
//! across the 2D and generic pipelines. This packet adds ONE pre-named
//! variant, [`PairUnsupported::UnsupportedPairClass`], tag
//! `"pair_unsupported_class"` — a certified-layer-local widening booked per
//! `docs/CERTIFICATE_MAPPING.md` section C row 1 (failure witnesses live in
//! `truck-certified`; `contract::Refusal` stays frozen, base evidence
//! untouched).
//!
//! **D-result — a row-3 result type, not a witness-edge.** Mapping section C
//! row 2 (witness-edge) is for certified shell EDGES; a derived pair contact
//! is row-3 branch geometry "carried as a result, not annotated onto shell
//! evidence". [`CertifiedPairResult`] mirrors the landed `PairContactResult`
//! (Disjoint / Contact / Unsupported(PairUnsupported) / Unresolved). The
//! `Unresolved(GenericUnresolved)` arm is carried for shape-parity only: the
//! exact-decision doctrine means the exact arms NEVER produce it. The contact
//! locus is family-tagged, WORLD-space, representation-derived certified
//! geometry.
//!
//! **D-sorted — operand order is canonical.** Mirroring the landed
//! `canonical_sides` discipline (`formal/contact.rs`), the pair is sorted by
//! participant identity and `dispatch_pair(a, b) == dispatch_pair(b, a)` (a
//! required test). The exact comparator: the participant enum's discriminant
//! order (Plane < Cylinder < Sphere) and, within a class, the witness's
//! representation-derived geometry ordering — the representation coordinates
//! compared component-wise lexicographically with `f64::total_cmp`
//! (deterministic, no hash order; coordinates break ties lexicographically).
//!
//! **D-exact — admission is exact-predicate-decided.** Every admission screen
//! (the geometric configuration test that decides which closed form applies)
//! is decided through `formal/exact.rs` exact arithmetic on the witnesses'
//! representation-derived `f64` coordinates (`Expansion` sign decisions via
//! `exact_sq_dist` / `exact_dot2` / `cross_exp` and their obvious 3-D
//! extensions built from the same primitives) — never a floating-point
//! epsilon comparison, never an interval straddle at ADMISSION time. The
//! VALUES of the emitted locus may be enclosure-valued (`Circle`'s radius
//! through `CertifiedInterval::sqrt`); the DECISIONS are exact. A
//! configuration the screens cannot name refuses `UnsupportedPairClass`.
//!
//! **D-routing — one participant enum, built from the landed witnesses.**
//! [`CertifiedPairParticipant`] carries Plane / Cylinder / Sphere only. Cone
//! and torus witnesses are KNOWN to the routing (the enum gains the variant
//! in DISPATCH-2); in this packet a cone/torus side refuses — the enum carries
//! no variant it cannot dispatch, and
//! [`CertifiedPairParticipant::from_cone_identification`] /
//! [`CertifiedPairParticipant::from_torus_identification`] map every
//! identification (certified or refused) to `None`. The from-identification
//! constructors map the landed `NotA*` arms to `None` and the certified arm to
//! `Some(...)`.
//!
//! # Coincidence asymmetries (called out on purpose)
//!
//! Coincident same-center-same-radius spheres and equal-radius coaxial
//! cylinders refuse `UnsupportedPairClass`, NOT `PairUnsupported::Overlap`:
//! `Overlap` is the 2D pipeline's positive-length shared-region cause, and a
//! coincident-surface pair is not a curve contact — the boolean layer's
//! coincidence handling owns it. Coincident planes DO refuse
//! `PairUnsupported::Overlap` (the 2D pipeline's own meaning: a positive-area
//! shared region).

use crate::formal::contact::GenericUnresolved;
use crate::formal::cylinder::{CertifiedEmbeddedCylinder, CylinderIdentification};
use crate::formal::exact::{CertifiedInterval, CertifiedSign, Expansion};
use crate::formal::intersection::PairUnsupported;
use crate::formal::sphere::{CertifiedEmbeddedSphere, SphereIdentification};
use crate::formal::support::{PlaneSchema, SupportSurfaceSchema};
use crate::formal::{cone::ConeIdentification, torus::TorusIdentification};
use std::cmp::Ordering;
use truck_geometry::prelude::{InnerSpace, Point3, Vector3};

/// One side of a dispatched pair: the certified witness of an identified
/// analytic surface. Constructed from the landed identification enums.
///
/// The `Cylinder` variant carries the certified cylinder witness UNBOXED — the
/// packet's D-routing public signature. `clippy::large_enum_variant`'s perf
/// heuristic fires on the size ratio against the small plane/sphere witnesses;
/// the cylinder witness is the routing's natural payload and boxing it would
/// change the required public surface, so the lint is suppressed at the item
/// (not the module) level.
#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::large_enum_variant)]
pub enum CertifiedPairParticipant {
    /// A certified plane witness.
    Plane(PlaneSchema),
    /// A certified embedded cylinder witness.
    Cylinder(CertifiedEmbeddedCylinder),
    /// A certified embedded sphere witness.
    Sphere(CertifiedEmbeddedSphere),
}

impl CertifiedPairParticipant {
    /// Route a landed support-surface schema: the certified plane arm becomes
    /// a [`Self::Plane`], every non-plane schema maps to `None`.
    pub fn from_support_schema(schema: &SupportSurfaceSchema) -> Option<Self> {
        match schema {
            SupportSurfaceSchema::Plane(plane) => Some(Self::Plane(*plane)),
            SupportSurfaceSchema::NotStructurallyIdentified(_) => None,
        }
    }

    /// Route a landed cylinder identification: the certified arm becomes a
    /// [`Self::Cylinder`], every `NotACylinder` arm maps to `None`.
    pub fn from_cylinder_identification(id: CylinderIdentification) -> Option<Self> {
        match id {
            CylinderIdentification::Cylinder(cylinder) => Some(Self::Cylinder(cylinder)),
            CylinderIdentification::NotACylinder(_) => None,
        }
    }

    /// Route a landed sphere identification: the certified arm becomes a
    /// [`Self::Sphere`], every `NotASphere` arm maps to `None`.
    pub fn from_sphere_identification(id: SphereIdentification) -> Option<Self> {
        match id {
            SphereIdentification::Sphere(sphere) => Some(Self::Sphere(sphere)),
            SphereIdentification::NotASphere(_) => None,
        }
    }

    /// The cone route, known to the routing but not this packet.
    ///
    /// **DISPATCH-2 deferral:** the cone arm (plane~cone 8,379 corpus pairs) is
    /// certifiable only in special geometric positions and books as
    /// BG-CK-P1-DISPATCH-2; this enum carries no cone variant it cannot
    /// dispatch. Every cone identification (certified or refused) maps to
    /// `None` — the typed no-silent-downgrade refusal lives at construction.
    pub fn from_cone_identification(id: ConeIdentification) -> Option<Self> {
        match id {
            ConeIdentification::Cone(_) | ConeIdentification::NotACone(_) => None,
        }
    }

    /// The torus route, known to the routing but not this packet.
    ///
    /// **DISPATCH-2 deferral:** the torus arm (plane~torus 5,385 corpus pairs)
    /// books with DISPATCH-2; this enum carries no torus variant it cannot
    /// dispatch. Every torus identification (certified or refused) maps to
    /// `None` — the typed no-silent-downgrade refusal lives at construction.
    pub fn from_torus_identification(id: TorusIdentification) -> Option<Self> {
        match id {
            TorusIdentification::Torus(_) | TorusIdentification::NotATorus(_) => None,
        }
    }
}

/// The certified contact locus of an admitted pair. Raw-frame doctrine:
/// directions are the surfaces' OWN axes (never orthogonalised, never
/// normalised downstream — the identify_plane retained-basis rule).
#[derive(Debug, Clone, PartialEq)]
pub enum ContactLocus {
    /// A shared line: point on the line + direction (raw magnitude).
    Line {
        /// A point on the shared line.
        point: Point3,
        /// The line direction, raw magnitude, never normalised.
        direction: Vector3,
    },
    /// A shared circle: center, axis direction (raw), and a certified
    /// enclosure of the radius (the sqrt path is enclosure-valued, not exact).
    Circle {
        /// The circle's center.
        center: Point3,
        /// The circle's axis direction, raw, never normalised.
        axis: Vector3,
        /// A certified enclosure of the radius (via `CertifiedInterval::sqrt`).
        radius: CertifiedInterval,
    },
    /// A single tangent point.
    Point {
        /// The tangent point.
        point: Point3,
    },
}

/// The certified contact: the sorted participants and the shared locus.
#[derive(Debug, Clone, PartialEq)]
pub struct CertifiedPairContact {
    /// The canonical first participant (D-sorted).
    pub first: CertifiedPairParticipant,
    /// The canonical second participant (D-sorted).
    pub second: CertifiedPairParticipant,
    /// The certified shared locus of the pair.
    pub locus: ContactLocus,
}

/// The result of dispatching one admitted-or-refused pair. Shape mirrors the
/// landed `PairContactResult` (`formal/contact.rs`).
///
/// `Contact` carries the certified contact by value (the packet's D-result
/// signature), so `clippy::large_enum_variant`'s perf heuristic fires against
/// the unit variants; boxing would change the required public surface, so the
/// lint is suppressed at the item (not the module) level.
#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::large_enum_variant)]
pub enum CertifiedPairResult {
    /// No contact: the pair is exactly disjoint.
    Disjoint,
    /// A certified contact: the sorted participants and their shared locus.
    Contact(CertifiedPairContact),
    /// The pair class (or configuration) is outside the admitted set.
    Unsupported(PairUnsupported),
    /// Carried for shape-parity with the landed result. The exact-decision
    /// doctrine means the exact arms NEVER produce it.
    Unresolved(GenericUnresolved),
}

/// Dispatch one analytic surface pair. Operand order is canonical (D-sorted).
///
/// The pair is sorted by participant identity, so `dispatch_pair(a, b) ==
/// dispatch_pair(b, a)`. Unroutable classes (any side the enum cannot carry,
/// any class outside arms 1–5) refuse
/// [`CertifiedPairResult::Unsupported(PairUnsupported::UnsupportedPairClass)`]
/// — typed, never swallowed, never downgraded.
pub fn dispatch_pair(
    a: &CertifiedPairParticipant,
    b: &CertifiedPairParticipant,
) -> CertifiedPairResult {
    let (first, second) = if participant_cmp(a, b) == Ordering::Greater {
        (b.clone(), a.clone())
    } else {
        (a.clone(), b.clone())
    };
    match (first, second) {
        (CertifiedPairParticipant::Plane(pa), CertifiedPairParticipant::Plane(pb)) => {
            plane_plane(pa, pb)
        }
        (CertifiedPairParticipant::Plane(p), CertifiedPairParticipant::Cylinder(c)) => {
            plane_cylinder(p, c)
        }
        (CertifiedPairParticipant::Plane(p), CertifiedPairParticipant::Sphere(s)) => {
            plane_sphere(p, s)
        }
        (CertifiedPairParticipant::Cylinder(ca), CertifiedPairParticipant::Cylinder(cb)) => {
            cylinder_cylinder(ca, cb)
        }
        (CertifiedPairParticipant::Sphere(sa), CertifiedPairParticipant::Sphere(sb)) => {
            sphere_sphere(sa, sb)
        }
        // After sorting the only remaining combination is cylinder~sphere,
        // which books as DISPATCH-2 and refuses here.
        _ => CertifiedPairResult::Unsupported(PairUnsupported::UnsupportedPairClass),
    }
}

// ---------------------------------------------------------------------------
// Canonical participant ordering (D-sorted)
// ---------------------------------------------------------------------------

/// Compare two participants by discriminant order, then within a class by the
/// witness's representation-derived geometry, lexicographically.
fn participant_cmp(a: &CertifiedPairParticipant, b: &CertifiedPairParticipant) -> Ordering {
    match (a, b) {
        (CertifiedPairParticipant::Plane(x), CertifiedPairParticipant::Plane(y)) => plane_cmp(x, y),
        (CertifiedPairParticipant::Cylinder(x), CertifiedPairParticipant::Cylinder(y)) => {
            cylinder_cmp(x, y)
        }
        (CertifiedPairParticipant::Sphere(x), CertifiedPairParticipant::Sphere(y)) => {
            sphere_cmp(x, y)
        }
        (CertifiedPairParticipant::Plane(_), CertifiedPairParticipant::Cylinder(_))
        | (CertifiedPairParticipant::Plane(_), CertifiedPairParticipant::Sphere(_))
        | (CertifiedPairParticipant::Cylinder(_), CertifiedPairParticipant::Sphere(_)) => {
            Ordering::Less
        }
        (CertifiedPairParticipant::Cylinder(_), CertifiedPairParticipant::Plane(_))
        | (CertifiedPairParticipant::Sphere(_), CertifiedPairParticipant::Plane(_))
        | (CertifiedPairParticipant::Sphere(_), CertifiedPairParticipant::Cylinder(_)) => {
            Ordering::Greater
        }
    }
}

/// Lexicographic comparison of coordinate tuples with `f64::total_cmp`.
///
/// Deterministic and total (no hash order; coordinates break ties
/// lexicographically). `total_cmp` is total even for the signed-zero/NaN
/// values the identifying constructors already refuse; NaN can never reach
/// these witnesses, so no comparison ever needs an epsilon.
fn compare_coords(a: &[f64], b: &[f64]) -> Ordering {
    for (x, y) in a.iter().zip(b.iter()) {
        match x.total_cmp(y) {
            Ordering::Equal => continue,
            other => return other,
        }
    }
    Ordering::Equal
}

/// Order two plane witnesses by their retained native basis coordinates.
fn plane_cmp(a: &PlaneSchema, b: &PlaneSchema) -> Ordering {
    compare_coords(
        &[
            a.origin().x,
            a.origin().y,
            a.origin().z,
            a.u_axis().x,
            a.u_axis().y,
            a.u_axis().z,
            a.v_axis().x,
            a.v_axis().y,
            a.v_axis().z,
        ],
        &[
            b.origin().x,
            b.origin().y,
            b.origin().z,
            b.u_axis().x,
            b.u_axis().y,
            b.u_axis().z,
            b.v_axis().x,
            b.v_axis().y,
            b.v_axis().z,
        ],
    )
}

/// Order two cylinder witnesses by their representation-derived schema
/// geometry: origin, axis, radial basis, radius.
fn cylinder_cmp(a: &CertifiedEmbeddedCylinder, b: &CertifiedEmbeddedCylinder) -> Ordering {
    let sa = a.schema();
    let sb = b.schema();
    compare_coords(
        &[
            sa.origin().x,
            sa.origin().y,
            sa.origin().z,
            sa.axis().x,
            sa.axis().y,
            sa.axis().z,
            sa.radial_x().x,
            sa.radial_x().y,
            sa.radial_x().z,
            sa.radial_y().x,
            sa.radial_y().y,
            sa.radial_y().z,
            sa.radius().get(),
        ],
        &[
            sb.origin().x,
            sb.origin().y,
            sb.origin().z,
            sb.axis().x,
            sb.axis().y,
            sb.axis().z,
            sb.radial_x().x,
            sb.radial_x().y,
            sb.radial_x().z,
            sb.radial_y().x,
            sb.radial_y().y,
            sb.radial_y().z,
            sb.radius().get(),
        ],
    )
}

/// Order two sphere witnesses by their representation-derived center and
/// radius.
fn sphere_cmp(a: &CertifiedEmbeddedSphere, b: &CertifiedEmbeddedSphere) -> Ordering {
    compare_coords(
        &[a.center().x, a.center().y, a.center().z, a.radius().get()],
        &[b.center().x, b.center().y, b.center().z, b.radius().get()],
    )
}

// ---------------------------------------------------------------------------
// Exact 3-D expansion primitives (D-exact: built from the landed 2-D
// primitives' construction, never an f64 epsilon)
// ---------------------------------------------------------------------------

/// One vector as an exact expansion vector over its `f64` coordinates.
fn exact_vector(v: Vector3) -> [Expansion; 3] {
    [
        Expansion::zero().grow(v.x),
        Expansion::zero().grow(v.y),
        Expansion::zero().grow(v.z),
    ]
}

/// The exact coordinate difference `a − b` as an expansion vector.
fn exact_diff(a: Point3, b: Point3) -> [Expansion; 3] {
    [
        Expansion::from_sum(a.x, -b.x),
        Expansion::from_sum(a.y, -b.y),
        Expansion::from_sum(a.z, -b.z),
    ]
}

/// The exact cross product of a plane's native basis vectors — the plane's
/// exact normal.
fn plane_normal_exp(u: Vector3, v: Vector3) -> [Expansion; 3] {
    [
        Expansion::from_product(u.y, v.z).merge(&Expansion::from_product(u.z, v.y).negate()),
        Expansion::from_product(u.z, v.x).merge(&Expansion::from_product(u.x, v.z).negate()),
        Expansion::from_product(u.x, v.y).merge(&Expansion::from_product(u.y, v.x).negate()),
    ]
}

/// Exact cross product of two expansion vectors.
fn cross_exp(a: &[Expansion; 3], b: &[Expansion; 3]) -> [Expansion; 3] {
    [
        a[1].mul_expansion(&b[2])
            .merge(&a[2].mul_expansion(&b[1]).negate()),
        a[2].mul_expansion(&b[0])
            .merge(&a[0].mul_expansion(&b[2]).negate()),
        a[0].mul_expansion(&b[1])
            .merge(&a[1].mul_expansion(&b[0]).negate()),
    ]
}

/// Exact dot product of two expansion vectors.
fn dot_exp(a: &[Expansion; 3], b: &[Expansion; 3]) -> Expansion {
    a[0].mul_expansion(&b[0])
        .merge(&a[1].mul_expansion(&b[1]))
        .merge(&a[2].mul_expansion(&b[2]))
}

/// Exact dot product of a raw `f64` vector with an exact expansion vector.
fn dot_f64_exp(v: Vector3, e: &[Expansion; 3]) -> Expansion {
    e[0].mul_expansion(&Expansion::zero().grow(v.x))
        .merge(&e[1].mul_expansion(&Expansion::zero().grow(v.y)))
        .merge(&e[2].mul_expansion(&Expansion::zero().grow(v.z)))
}

/// Exact cross product of a raw `f64` vector with an exact expansion vector.
fn cross_f64_exp(v: Vector3, e: &[Expansion; 3]) -> [Expansion; 3] {
    let x = Expansion::zero().grow(v.x);
    let y = Expansion::zero().grow(v.y);
    let z = Expansion::zero().grow(v.z);
    [
        y.mul_expansion(&e[2])
            .merge(&z.mul_expansion(&e[1]).negate()),
        z.mul_expansion(&e[0])
            .merge(&x.mul_expansion(&e[2]).negate()),
        x.mul_expansion(&e[1])
            .merge(&y.mul_expansion(&e[0]).negate()),
    ]
}

/// Exact squared distance between two points.
fn exact_sq_dist3(a: Point3, b: Point3) -> Expansion {
    dot_exp(&exact_diff(a, b), &exact_diff(a, b))
}

/// The sum of the componentwise squares of an exact expansion vector.
fn sum_of_squares(e: &[Expansion; 3]) -> Expansion {
    e[0].mul_expansion(&e[0])
        .merge(&e[1].mul_expansion(&e[1]))
        .merge(&e[2].mul_expansion(&e[2]))
}

/// Whether an exact expansion vector is exactly the zero vector.
fn is_zero_vector(e: &[Expansion; 3]) -> bool {
    e.iter().all(Expansion::is_zero)
}

// ---------------------------------------------------------------------------
// Arm 1: plane~plane (26,274)
// ---------------------------------------------------------------------------

fn plane_plane(a: PlaneSchema, b: PlaneSchema) -> CertifiedPairResult {
    let na = plane_normal_exp(a.u_axis(), a.v_axis());
    let nb = plane_normal_exp(b.u_axis(), b.v_axis());
    // Exact parallelism: the normals' cross expansion is the zero vector.
    if !is_zero_vector(&cross_exp(&na, &nb)) {
        return CertifiedPairResult::Contact(CertifiedPairContact {
            first: CertifiedPairParticipant::Plane(a),
            second: CertifiedPairParticipant::Plane(b),
            locus: plane_plane_line(&a, &b),
        });
    }
    // Parallel. Coincident iff a's origin lies on b's plane (exact point-on-
    // plane test: a positive-area shared region — the 2D pipeline's own
    // `Overlap` meaning).
    if dot_exp(&exact_diff(a.origin(), b.origin()), &nb).is_zero() {
        CertifiedPairResult::Unsupported(PairUnsupported::Overlap)
    } else {
        CertifiedPairResult::Disjoint
    }
}

/// The classic point+direction construction for transverse planes.
///
/// With raw normals `n1`, `n2` (each the plane's own `u × v`), the line is
/// `L = n1 × n2` and the point is `(h1 (n2 × L) + h2 (L × n1)) / (L · L)`
/// with `h_i = n_i · p_i`. The division is safe: transversality means
/// `L · L > 0`.
fn plane_plane_line(a: &PlaneSchema, b: &PlaneSchema) -> ContactLocus {
    let n1 = a.u_axis().cross(a.v_axis());
    let n2 = b.u_axis().cross(b.v_axis());
    let l = n1.cross(n2);
    let h1 = n1.dot(a.origin() - Point3::new(0.0, 0.0, 0.0));
    let h2 = n2.dot(b.origin() - Point3::new(0.0, 0.0, 0.0));
    let num = h1 * (n2.cross(l)) + h2 * (l.cross(n1));
    let denom = l.dot(l);
    let point = Point3::new(num.x / denom, num.y / denom, num.z / denom);
    ContactLocus::Line {
        point,
        direction: l,
    }
}

// ---------------------------------------------------------------------------
// Arm 2: plane~cylinder (37,361; the admitted axis-normal and tangent-parallel
// configurations only)
// ---------------------------------------------------------------------------

fn plane_cylinder(plane: PlaneSchema, cyl: CertifiedEmbeddedCylinder) -> CertifiedPairResult {
    let n = plane_normal_exp(plane.u_axis(), plane.v_axis());
    let nf = plane.u_axis().cross(plane.v_axis());
    let s = cyl.schema();
    let axis = s.axis();
    let radius = s.radius().get();
    let origin = s.origin();
    // Axis vs plane-normal exact test. The admitted configuration is the
    // axis-normal plane only: `axis × normal` zero AND `axis · normal` nonzero
    // (a perpendicular plane cuts the cylinder in its own radius circle).
    let cross_ax = cross_f64_exp(axis, &n);
    let dot_ax = dot_f64_exp(axis, &n);
    if is_zero_vector(&cross_ax) && !dot_ax.is_zero() {
        let center = axis_plane_pierce(origin, axis, plane.origin(), nf);
        return CertifiedPairResult::Contact(CertifiedPairContact {
            first: CertifiedPairParticipant::Plane(plane),
            second: CertifiedPairParticipant::Cylinder(cyl),
            locus: ContactLocus::Circle {
                center,
                axis: nf,
                // The perpendicular cut's radius is the cylinder's own radius,
                // exactly.
                radius: CertifiedInterval::point(radius),
            },
        });
    }
    if !is_zero_vector(&cross_ax) && dot_ax.is_zero() {
        // Plane parallel to the axis: tangent-parallel, decided by the exact
        // distance-to-axis screen.
        return plane_cylinder_parallel(plane, cyl, nf, n);
    }
    // General oblique cut is an ellipse: not a `ContactLocus` variant, not
    // certifiable closed-form here (books DISPATCH-2 with the cone~plane
    // rational-conic machinery).
    CertifiedPairResult::Unsupported(PairUnsupported::UnsupportedPairClass)
}

/// The pierce point of the cylinder's axis line with an axis-normal plane.
fn axis_plane_pierce(
    axis_origin: Point3,
    axis: Vector3,
    plane_origin: Point3,
    nf: Vector3,
) -> Point3 {
    let t = nf.dot(plane_origin - axis_origin) / nf.dot(axis);
    axis_origin + t * axis
}

/// Plane parallel to the cylinder axis: tangent generatrix or disjoint.
///
/// Exact distance-to-axis screen: sign of `(n·(o_axis − o_plane))² − r²·n·n`.
/// Equal → the shared generatrix through the closest points; greater →
/// `Disjoint`; less → the plane cuts TWO generatrices (not one
/// [`ContactLocus::Line`]) and refuses `UnsupportedPairClass`.
fn plane_cylinder_parallel(
    plane: PlaneSchema,
    cyl: CertifiedEmbeddedCylinder,
    nf: Vector3,
    n: [Expansion; 3],
) -> CertifiedPairResult {
    let s = cyl.schema();
    let r = s.radius().get();
    let axis = s.axis();
    let origin = s.origin();
    let off = dot_exp(&exact_diff(origin, plane.origin()), &n);
    let off_sq = off.mul_expansion(&off);
    let n_sq = dot_exp(&n, &n);
    let rhs = Expansion::from_product(r, r).mul_expansion(&n_sq);
    match off_sq.merge(&rhs.negate()).sign() {
        CertifiedSign::Zero => {
            // Tangent: the generatrix through the foot of the axis origin on
            // the plane (at distance r from the axis, on both surfaces).
            let t = nf.dot(origin - plane.origin()) / nf.dot(nf);
            let point = origin - t * nf;
            CertifiedPairResult::Contact(CertifiedPairContact {
                first: CertifiedPairParticipant::Plane(plane),
                second: CertifiedPairParticipant::Cylinder(cyl),
                locus: ContactLocus::Line {
                    point,
                    direction: axis,
                },
            })
        }
        CertifiedSign::Positive => CertifiedPairResult::Disjoint,
        CertifiedSign::Negative => {
            CertifiedPairResult::Unsupported(PairUnsupported::UnsupportedPairClass)
        }
    }
}

// ---------------------------------------------------------------------------
// Arm 3: plane~sphere (281)
// ---------------------------------------------------------------------------

fn plane_sphere(plane: PlaneSchema, sphere: CertifiedEmbeddedSphere) -> CertifiedPairResult {
    let n = plane_normal_exp(plane.u_axis(), plane.v_axis());
    let nf = plane.u_axis().cross(plane.v_axis());
    let r = sphere.radius().get();
    let c = sphere.center();
    // Exact squared distance from the center to the plane vs `r²`: sign of
    // `(n·(c − o))² − r²·n·n`.
    let off = dot_exp(&exact_diff(c, plane.origin()), &n);
    let off_sq = off.mul_expansion(&off);
    let n_sq = dot_exp(&n, &n);
    let rhs = Expansion::from_product(r, r).mul_expansion(&n_sq);
    let diff = off_sq.merge(&rhs.negate());
    let foot = || {
        let t = nf.dot(c - plane.origin()) / nf.dot(nf);
        c - t * nf
    };
    match diff.sign() {
        CertifiedSign::Negative => {
            // Circle: the foot of the perpendicular is the center; the radius
            // enclosure is `sqrt` of the exact difference's interval image and
            // must contain the true radius.
            let rad_sq = CertifiedInterval::from_expansion(&diff.negate());
            let radius_iv = match rad_sq.div(&CertifiedInterval::from_expansion(&n_sq)) {
                Some(quotient) => match quotient.sqrt() {
                    Some(radius) => radius,
                    None => {
                        return CertifiedPairResult::Unsupported(
                            PairUnsupported::UnsupportedPairClass,
                        );
                    }
                },
                None => {
                    return CertifiedPairResult::Unsupported(PairUnsupported::UnsupportedPairClass);
                }
            };
            CertifiedPairResult::Contact(CertifiedPairContact {
                first: CertifiedPairParticipant::Plane(plane),
                second: CertifiedPairParticipant::Sphere(sphere),
                locus: ContactLocus::Circle {
                    center: foot(),
                    axis: nf,
                    radius: radius_iv,
                },
            })
        }
        CertifiedSign::Zero => CertifiedPairResult::Contact(CertifiedPairContact {
            first: CertifiedPairParticipant::Plane(plane),
            second: CertifiedPairParticipant::Sphere(sphere),
            locus: ContactLocus::Point { point: foot() },
        }),
        CertifiedSign::Positive => CertifiedPairResult::Disjoint,
    }
}

// ---------------------------------------------------------------------------
// Arm 4: sphere~sphere (126)
// ---------------------------------------------------------------------------

fn sphere_sphere(a: CertifiedEmbeddedSphere, b: CertifiedEmbeddedSphere) -> CertifiedPairResult {
    let c1 = a.center();
    let c2 = b.center();
    let r1 = a.radius().get();
    let r2 = b.radius().get();
    // Exact `|c1 − c2|²` vs `(r1 ± r2)²`, all-exact expansions.
    let d = exact_sq_dist3(c1, c2);
    let sum_sq = Expansion::from_sum(r1, r2).mul_expansion(&Expansion::from_sum(r1, r2));
    let diff_sq = Expansion::from_sum(r1, -r2).mul_expansion(&Expansion::from_sum(r1, -r2));
    if d.is_zero() {
        // Same center. Same radius is a coincident-sphere pair — not a curve
        // contact, and NOT the 2D `Overlap` cause: the boolean layer's
        // coincidence handling owns it. Different radii (concentric) never
        // meet.
        return if r1 == r2 {
            CertifiedPairResult::Unsupported(PairUnsupported::UnsupportedPairClass)
        } else {
            CertifiedPairResult::Disjoint
        };
    }
    let cmp_sum = d.merge(&sum_sq.negate()).sign();
    let cmp_diff = d.merge(&diff_sq.negate()).sign();
    if cmp_sum == CertifiedSign::Zero || cmp_diff == CertifiedSign::Zero {
        // External tangency (equal to the sum) or internal tangency (equal to
        // the difference): a single tangent point.
        let delta = c2 - c1;
        let dist = delta.magnitude();
        let point = if cmp_diff == CertifiedSign::Zero {
            // Internal tangency: the touching point is on the side of the
            // smaller sphere away from the larger.
            if Expansion::from_sum(r1, -r2).sign() == CertifiedSign::Positive {
                c2 + (r2 / dist) * delta
            } else {
                c1 - (r1 / dist) * delta
            }
        } else {
            c1 + (r1 / dist) * delta
        };
        return CertifiedPairResult::Contact(CertifiedPairContact {
            first: CertifiedPairParticipant::Sphere(a),
            second: CertifiedPairParticipant::Sphere(b),
            locus: ContactLocus::Point { point },
        });
    }
    if cmp_sum == CertifiedSign::Positive || cmp_diff == CertifiedSign::Negative {
        return CertifiedPairResult::Disjoint;
    }
    // Strictly between the squared distance bounds: the radical-plane circle,
    // radius enclosure via `sqrt`.
    let d_iv = CertifiedInterval::from_expansion(&d);
    let r1_sq_iv = CertifiedInterval::from_expansion(&Expansion::from_product(r1, r1));
    let r2_sq_iv = CertifiedInterval::from_expansion(&Expansion::from_product(r2, r2));
    let num = d_iv.add(&r1_sq_iv).sub(&r2_sq_iv);
    let two_sqrt_d = match d_iv.sqrt() {
        Some(root) => root.scale_pow2(1),
        None => return CertifiedPairResult::Unsupported(PairUnsupported::UnsupportedPairClass),
    };
    let a_iv = match num.div(&two_sqrt_d) {
        Some(a) => a,
        None => return CertifiedPairResult::Unsupported(PairUnsupported::UnsupportedPairClass),
    };
    let radius = match r1_sq_iv.sub(&a_iv.mul(&a_iv)).sqrt() {
        Some(radius) => radius,
        None => return CertifiedPairResult::Unsupported(PairUnsupported::UnsupportedPairClass),
    };
    let delta = c2 - c1;
    let d_f64 = delta.magnitude2();
    let t = (d_f64 + r1 * r1 - r2 * r2) / (2.0 * d_f64);
    CertifiedPairResult::Contact(CertifiedPairContact {
        first: CertifiedPairParticipant::Sphere(a),
        second: CertifiedPairParticipant::Sphere(b),
        locus: ContactLocus::Circle {
            center: c1 + t * delta,
            axis: delta,
            radius,
        },
    })
}

// ---------------------------------------------------------------------------
// Arm 5: cylinder~cylinder (5,354; the coaxial/parallel subset only)
// ---------------------------------------------------------------------------

fn cylinder_cylinder(
    a: CertifiedEmbeddedCylinder,
    b: CertifiedEmbeddedCylinder,
) -> CertifiedPairResult {
    let sa = a.schema();
    let sb = b.schema();
    let axis_a = sa.axis();
    let axis_b = sb.axis();
    let a_vec = exact_vector(axis_a);
    let b_vec = exact_vector(axis_b);
    // Axes parallel: exact cross expansion.
    if !is_zero_vector(&cross_exp(&a_vec, &b_vec)) {
        // General skew-cylinder intersection is a quartic; DISPATCH-2 or
        // Phase 2.
        return CertifiedPairResult::Unsupported(PairUnsupported::UnsupportedPairClass);
    }
    // Collinear (coaxial): `(o2 − o1) × axis` is exactly the zero vector.
    let cross_offset = cross_exp(&a_vec, &exact_diff(sb.origin(), sa.origin()));
    let r1 = sa.radius().get();
    let r2 = sb.radius().get();
    if is_zero_vector(&cross_offset) {
        if r1 == r2 {
            // Coincident cylinder faces — same doctrine as arm 4: NOT the 2D
            // `Overlap` cause.
            CertifiedPairResult::Unsupported(PairUnsupported::UnsupportedPairClass)
        } else {
            // Coaxial cylinders of different radii NEVER meet (annulus gap).
            CertifiedPairResult::Disjoint
        }
    } else {
        // Parallel non-collinear: exact axis-distance vs `r1 + r2`. The
        // distance between the parallel axes is `|(o2 − o1) × a| / |a|`, so
        // compare `|(o2 − o1) × a|²` against `(r1 + r2)² · (a · a)`.
        let dist2 = sum_of_squares(&cross_offset);
        let axis_sq = dot_exp(&a_vec, &a_vec);
        let sum = Expansion::from_sum(r1, r2);
        let sum_sq = sum.mul_expansion(&sum);
        match dist2.merge(&sum_sq.mul_expansion(&axis_sq).negate()).sign() {
            CertifiedSign::Zero => {
                // Tangent: the shared generatrix through the closest points.
                let p1 = sa.origin() + (sb.origin() - sa.origin()).dot(axis_a) * axis_a;
                let w = sb.origin() - p1;
                let wd = w.magnitude();
                let point = p1 + (r1 / wd) * w;
                CertifiedPairResult::Contact(CertifiedPairContact {
                    first: CertifiedPairParticipant::Cylinder(a),
                    second: CertifiedPairParticipant::Cylinder(b),
                    locus: ContactLocus::Line {
                        point,
                        direction: axis_a,
                    },
                })
            }
            CertifiedSign::Positive => CertifiedPairResult::Disjoint,
            CertifiedSign::Negative => {
                // The axes are closer than `r1 + r2`: the cylinders cut in TWO
                // generatrices, not one `Line`.
                CertifiedPairResult::Unsupported(PairUnsupported::UnsupportedPairClass)
            }
        }
    }
}
