//! BG-SOL-S3-CONTACT — the Contact Layer skeleton.
//!
//! `contact(lhs, rhs)` answers "how do these two boundary strata meet?" for
//! the solver family's Phase 3 funnel (docs/SOLVER_FAMILY_PLAN.md §4 Phase 3 +
//! §5). The flagship differential test `Extrude(P−Q) ≅ Extrude(P)−Extrude(Q)`
//! is the M2 cross-layer gate and needs the 3-D Boolean on its RHS, which the
//! Boundary Rewrite (Phase 4) drives from this oracle: every pair of boundary
//! strata (FF, FE, EE) is dispatched here.
//!
//! This packet establishes the stratum vocabulary (`BoundedStratum`,
//! `ContactComplex`, `ContactLocus`) and the dispatcher's cheapest stages:
//! identity/overlap (C0-C2, coincident canonical carriers), the analytic FF
//! pairs (plan §3.3, which already exist in `truck_evidence::analytic`), and
//! the general validated FF stage (BG-SOL-S7-GFF-WIRE) for the offset mixed
//! quadric cells, which certifies a regular branch cover of the two carriers'
//! shared zero set over the certified AABB intersection of their patches via
//! `gff::cover_branch`. Everything else — FE/EE strata reductions, singular
//! event cells, 2-D overlap — returns an honest
//! `Refusal::UnsupportedEnvelope(EnvelopeCase::ContactReductionDeferred)`, the
//! typed boundary of the funnel the later packets fill in.
//!
//! Strata are geometry-side on purpose: `truck-evidence` cannot name
//! `truck-topology` (the dependency direction is the reverse), so a stratum
//! carries the canonical carrier (from the structural recognizer) plus a
//! parameter-space box, not a topology handle. Trimming to the actual face
//! boundary (wires) is a later strata-reduction refinement.
//!
//! House rules H-1..H-8 apply.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

use self::implicit::ImplicitField;
use crate::analytic::coaxial::{coaxial, CoaxialPair};
use crate::analytic::parallel_cylinders::parallel_cylinders;
use crate::analytic::plane_cone::plane_cone;
use crate::analytic::plane_cylinder::plane_cylinder;
use crate::analytic::plane_plane::plane_plane;
use crate::analytic::plane_sphere::plane_sphere;
use crate::analytic::sphere_sphere::sphere_sphere;
use crate::analytic::{AnalyticIntersection, AnalyticOutcome, ExactCurve};
use crate::enclosure::{Box3, EnclosureSurface, Interval};
use std::f64::consts::TAU;
use truck_base::cgmath64::{EuclideanSpace, InnerSpace, Point3};
use truck_base::contact::{ContactDimension, ContactEventKind};
use truck_base::evidence::{
    Budget, Certificate, Certified, EnvelopeCase, Margin, Method, Modulus, Outcome, Prop, PropMap,
    Refusal, Truth, UnresolvedWitness,
};
use truck_geometry::recognize::{
    CanonicalCarrier, CanonicalCarrierWitness, CanonicalCurve, CanonicalSurface,
};
use truck_geometry::specifieds::Plane;

/// BG-SOL-S4-FE-EE: the FE (Edge × Face) and EE (Edge × Edge) strata reductions.
///
/// All new FE/EE machinery lives in this submodule so the later funnel packets
/// (cylinder × cylinder, general validated FF, 2-D overlap) extend the Contact
/// Layer without colliding on this dispatcher file.
pub mod fe_ee;
pub mod gff;
pub mod implicit;
/// BG-SOL-S7-OVERLAP: the 2-D overlap screen (strict parameter-box interior
/// overlap), consumed by the identity arms and the analytic `Coincident`
/// screen.
pub mod overlap;
pub mod singular;

/// One boundary stratum of a solid, lifted to the canonical-carrier level.
///
/// The "bounded" is a parameter-space box/interval on the canonical carrier;
/// trimming to the actual face boundary (wires) is a later strata-reduction
/// refinement, not this packet. The carrier is always canonical: an
/// unrecognized (e.g. spline) stored surface is refused at the lift boundary
/// [`face_stratum`] — `CanonicalSurface` has no `Unrecognized` arm.
#[derive(Clone, Debug, PartialEq)]
pub enum BoundedStratum {
    /// A face: a canonical analytic surface bounded by a `(u, v)` box.
    Face {
        /// The canonical analytic surface carrier.
        surface: CanonicalSurface,
        /// The `u`-parameter box of the face.
        u_range: (f64, f64),
        /// The `v`-parameter box of the face.
        v_range: (f64, f64),
    },
    /// An edge: a canonical analytic curve bounded by a `t` interval.
    Edge {
        /// The canonical analytic curve carrier.
        curve: CanonicalCurve,
        /// The `t`-parameter interval of the edge.
        t_range: (f64, f64),
    },
    /// A vertex.
    Vertex {
        /// The vertex position.
        point: Point3,
    },
}

/// The certified contact between one stratum pair.
#[derive(Clone, Debug)]
pub struct ContactComplex {
    /// The contact records, one per locus component. Empty means the pair was
    /// decided to make no contact (e.g. a parallel or empty analytic arm).
    pub contacts: Vec<ContactRecord>,
}

/// One component of a certified contact.
#[derive(Clone, Debug)]
pub struct ContactRecord {
    /// The dimension of the contact locus.
    pub dimension: ContactDimension,
    /// The event kind of the contact.
    pub kind: ContactEventKind,
    /// The geometric locus of the contact.
    pub locus: ContactLocus,
}

/// The geometric locus of a certified contact.
#[derive(Clone, Debug)]
#[allow(clippy::large_enum_variant)] // The analytic locus is the exactly-solved FF intersection as booked in the plan's §4 Phase 3 signature; boxing would complicate the later strata-reduction packets' matches.
pub enum ContactLocus {
    /// C1/C2 identity/overlap: the two strata share a canonical carrier.
    Coincident,
    /// An exactly-solved analytic FF pair.
    Analytic(AnalyticIntersection),
    /// An isolated contact point (FE punctures, EE crossings).
    Point(Point3),
    /// An exact curve clipped to a parameter range in the curve's own
    /// parameterization: an Arc1 coincident sub-arc (an edge lying on a face,
    /// overlapping collinear edges). `t_range` is on the curve's own
    /// parameter, so a `Line` sub-segment is `t_range ⊂ [0, 1]` on `subs(t) =
    /// a + t(b−a)` and a circle sub-arc is an angular interval on `[0, TAU)`.
    BoundedCurve {
        curve: ExactCurve,
        t_range: (f64, f64),
    },
    /// A complete regular branch cover from the validated FF engine.
    /// The singular and unresolved lists are empty when this arm is built.
    /// Points are certified cross-sections of one or more Arc1 components;
    /// connectivity and component ordering are deliberately not claimed yet.
    /// Event continuation later produces `RegularContactArc`s.
    ValidatedBranchCover(gff::BranchCover),
}

/// Answers "how do these two boundary strata meet?"
///
/// Dispatches in the plan's §4 Phase 3 order and stops at the first decided
/// stage:
///
/// 1. **C0-C2 identity/overlap** — equal canonical carriers are coincident
///    (`Face`/`Face` → `Region2`/`IdenticalCarrier`, `Edge`/`Edge` →
///    `Arc1`/`IdenticalCarrier`). C0 provenance identity is topology-side and
///    cannot be expressed at the canonical-carrier level.
/// 2. **FF analytic** — both faces carry canonical analytic surfaces from the
///    §3.3 table; the ordered pair is solved by the existing exact pair
///    functions and the arm is mapped onto the shared 2-D ontology. The exact
///    arms ignore the parameter bounds; only the validated stage consumes them.
/// 3. **General validated FF** — the offset mixed-quadric cells
///    (Cylinder/Cone, Cylinder/Sphere, Cone/Cone, Cone/Sphere, both orders)
///    intersect their patches' certified AABBs and run
///    [`gff::cover_branch`] over the world box; a complete regular cover
///    becomes a [`ContactLocus::ValidatedBranchCover`] record, an empty one a
///    certified empty complex, a singular one a deferred refusal, and an
///    unresolved one a typed `NumericallyUnresolved`.
/// 4. **Strata reductions** — an `Edge` × `Face` pair is answered by
///    [`fe_ee::fe_contact`] (order-insensitive: the `(Face, Edge)` order feeds
///    the same solver with the arguments normalized to `(edge, face)`), and an
///    `Edge` × `Edge` pair by [`fe_ee::ee_contact`]. The bounded locus forms
///    (`ContactLocus::Point`, `ContactLocus::BoundedCurve`) are emitted here.
/// 5. **Everything else** — the deferred funnel (any pair involving a
///    `Vertex`, FE/EE carrier families outside the landed tables, singular
///    event cells, 2-D overlap) refuses with `ContactReductionDeferred`.
///
/// The exact and coaxial analytic pairs take no budget; the validated FF stage
/// owns the caller's budget and reports spend as entry minus remaining.
pub fn contact(
    lhs: &BoundedStratum,
    rhs: &BoundedStratum,
    budget: &mut Budget,
) -> Outcome<ContactComplex> {
    match (lhs, rhs) {
        // Stage 1: C0-C2 identity/overlap. The same carrier means the same
        // parameterization, so the record is emitted only when the two
        // patches' parameter boxes overlap with NON-EMPTY INTERIOR; disjoint
        // patches of the same canonical carrier report a certified empty
        // complex (BG-SOL-S7-OVERLAP).
        (
            BoundedStratum::Face {
                surface: l,
                u_range: l_u,
                v_range: l_v,
            },
            BoundedStratum::Face {
                surface: r,
                u_range: r_u,
                v_range: r_v,
            },
        ) if l == r => {
            if identity_face_boxes_overlap(l, *l_u, *l_v, *r_u, *r_v) {
                let mut props = PropMap::new();
                props.set(Prop::AnalyticCarrier, Truth::True);
                Ok(Certified::new(
                    ContactComplex {
                        contacts: vec![ContactRecord {
                            dimension: ContactDimension::Region2,
                            kind: ContactEventKind::IdenticalCarrier,
                            locus: ContactLocus::Coincident,
                        }],
                    },
                    Certificate {
                        props,
                        method: Method::Exact,
                        budget_left: *budget,
                        margin: Margin::UNBOUNDED,
                        modulus: Modulus::Unbounded,
                    },
                ))
            } else {
                Ok(Certified::new(
                    ContactComplex {
                        contacts: Vec::new(),
                    },
                    Certificate {
                        props: PropMap::new(),
                        method: Method::Exact,
                        budget_left: *budget,
                        margin: Margin::UNBOUNDED,
                        modulus: Modulus::Unbounded,
                    },
                ))
            }
        }
        (
            BoundedStratum::Edge {
                curve: l,
                t_range: tl,
            },
            BoundedStratum::Edge {
                curve: r,
                t_range: tr,
            },
        ) if l == r => {
            let overlaps = match l {
                CanonicalCurve::Line(_) => overlap::interior_overlap(*tl, *tr),
                CanonicalCurve::Circle(_) => overlap::periodic_interior_overlap(*tl, *tr, TAU),
            };
            if overlaps {
                let mut props = PropMap::new();
                props.set(Prop::AnalyticCarrier, Truth::True);
                Ok(Certified::new(
                    ContactComplex {
                        contacts: vec![ContactRecord {
                            dimension: ContactDimension::Arc1,
                            kind: ContactEventKind::IdenticalCarrier,
                            locus: ContactLocus::Coincident,
                        }],
                    },
                    Certificate {
                        props,
                        method: Method::Exact,
                        budget_left: *budget,
                        margin: Margin::UNBOUNDED,
                        modulus: Modulus::Unbounded,
                    },
                ))
            } else {
                Ok(Certified::new(
                    ContactComplex {
                        contacts: Vec::new(),
                    },
                    Certificate {
                        props: PropMap::new(),
                        method: Method::Exact,
                        budget_left: *budget,
                        margin: Margin::UNBOUNDED,
                        modulus: Modulus::Unbounded,
                    },
                ))
            }
        }
        // Stage 2: FF analytic. Both `(u, v)` boxes ride into the analytic
        // dispatch; the exact arms ignore them and the validated arms consume
        // them (each carrier stays paired with its bounds).
        (
            BoundedStratum::Face {
                surface: l,
                u_range,
                v_range,
            },
            BoundedStratum::Face {
                surface: r,
                u_range: r_u_range,
                v_range: r_v_range,
            },
        ) => analytic_ff(l, r, *u_range, *v_range, *r_u_range, *r_v_range, budget),
        // Stage 3: FE/EE strata reductions. The FE solver always sees
        // `(edge, face)`; the `(Face, Edge)` order feeds the same solver with
        // the arguments swapped, and the two orders produce structurally equal
        // `ContactComplex` values (the metamorphic property).
        (
            BoundedStratum::Edge { curve, t_range },
            BoundedStratum::Face {
                surface,
                u_range,
                v_range,
            },
        ) => fe_ee::fe_contact(curve, t_range, surface, u_range, v_range, budget),
        (
            BoundedStratum::Face {
                surface,
                u_range,
                v_range,
            },
            BoundedStratum::Edge { curve, t_range },
        ) => fe_ee::fe_contact(curve, t_range, surface, u_range, v_range, budget),
        (
            BoundedStratum::Edge {
                curve: l,
                t_range: tl,
            },
            BoundedStratum::Edge {
                curve: r,
                t_range: tr,
            },
        ) => fe_ee::ee_contact(l, tl, r, tr, budget),
        // Stage 4: everything else is the deferred funnel.
        _ => Err(Refusal::UnsupportedEnvelope(
            EnvelopeCase::ContactReductionDeferred,
        )),
    }
}

/// Lift a stored surface's structural-recognition witness to a bounded face
/// stratum.
///
/// `BoundedStratum::Face` carries a `CanonicalSurface`, which has no
/// `Unrecognized` arm; the Contact Layer's refusal for a non-canonical (e.g.
/// spline) carrier is therefore enforced at this lift boundary, before
/// `contact()` is ever reached. The caller supplies the parameter-space box
/// (the trimmed wire boundary is a later strata-reduction refinement). A
/// `CanonicalCarrier::Curve` witness is refused here too: an edge lift needs a
/// `t_range`, which this surface lift does not carry.
pub fn face_stratum(
    witness: CanonicalCarrierWitness,
    u_range: (f64, f64),
    v_range: (f64, f64),
) -> Result<BoundedStratum, Refusal> {
    let carrier = match witness {
        CanonicalCarrierWitness::ExactCanonical { carrier, .. }
        | CanonicalCarrierWitness::Derived { carrier, .. } => carrier,
        CanonicalCarrierWitness::Unrecognized => {
            return Err(Refusal::UnsupportedEnvelope(
                EnvelopeCase::ContactReductionDeferred,
            ))
        }
    };
    match carrier {
        CanonicalCarrier::Surface(surface) => Ok(BoundedStratum::Face {
            surface,
            u_range,
            v_range,
        }),
        CanonicalCarrier::Curve(_) => Err(Refusal::UnsupportedEnvelope(
            EnvelopeCase::ContactReductionDeferred,
        )),
    }
}

/// Whether two canonical curved carriers are coaxial: their axis positions
/// (the `(x, y)` of the cylinder's center, the cone's apex, or the sphere's
/// center) are exactly equal. This is `CoaxialPair::validate`'s exact f64
/// equality — no intervals, no tolerance: a pair that is 1-ulp apart in `x`
/// is not coaxial, and the parallel-cell answer (for cylinder × cylinder) or
/// the deferred refusal (for the mixed pairs) is the correct one for it.
fn coaxial_axes(axis0: Point3, axis1: Point3) -> bool {
    axis0.x == axis1.x && axis0.y == axis1.y
}

/// The stage-2 FF analytic dispatch: match the ordered carrier pair against
/// the §3.3 table, solve with the existing exact pair function, and map the
/// arm onto the shared 2-D ontology. Each carrier stays paired with its
/// parameter bounds; the exact arms ignore them and the validated arms
/// consume them.
///
/// Every canonical curved carrier is z-axis-aligned, so any curved × curved
/// pair of canonical carriers has **parallel** axes; the pair is either
/// coaxial (the same-axis `coaxial` family) or parallel-but-offset. The
/// offset cylinder × cylinder cell is `parallel_cylinders`; the offset mixed
/// quadric cells (Cylinder/Cone, Cylinder/Sphere, Cone/Cone, Cone/Sphere)
/// are answered by the general validated FF stage
/// (BG-SOL-S7-GFF-WIRE); `Torus` and `Placed` carriers, and any canonical
/// analytic pair without an exact closed form in §3.3, fall through to the
/// deferred funnel (`ContactReductionDeferred`). A numerically unresolved
/// analytic arm is propagated as-is: it is a stop, not a guess. The dispatch
/// predicate guarantees `CoaxialPair::validate` passes, so a
/// `NonCanonicalCarrier` refusal from `coaxial` can only mean a bug and is
/// propagated, not hidden.
fn analytic_ff(
    l: &CanonicalSurface,
    r: &CanonicalSurface,
    u_range_l: (f64, f64),
    v_range_l: (f64, f64),
    u_range_r: (f64, f64),
    v_range_r: (f64, f64),
    budget: &mut Budget,
) -> Outcome<ContactComplex> {
    let outcome: AnalyticOutcome = match (l, r) {
        (CanonicalSurface::Plane(a), CanonicalSurface::Plane(b)) => plane_plane(a, b),
        (CanonicalSurface::Plane(a), CanonicalSurface::Sphere(b)) => plane_sphere(a, b),
        (CanonicalSurface::Sphere(a), CanonicalSurface::Plane(b)) => plane_sphere(b, a),
        (CanonicalSurface::Sphere(a), CanonicalSurface::Sphere(b)) => sphere_sphere(a, b),
        (CanonicalSurface::Plane(a), CanonicalSurface::Cylinder(b)) => plane_cylinder(a, b),
        (CanonicalSurface::Cylinder(a), CanonicalSurface::Plane(b)) => plane_cylinder(b, a),
        (CanonicalSurface::Plane(a), CanonicalSurface::Cone(b)) => plane_cone(a, b),
        (CanonicalSurface::Cone(a), CanonicalSurface::Plane(b)) => plane_cone(b, a),
        // The cylinder-family analytic pairs (BG-SOL-S5-CYLPAIR). Coaxial iff
        // the axis positions are exactly equal; offset cylinder × cylinder is
        // `parallel_cylinders`, and the offset mixed quadric cells route to
        // the validated FF stage.
        (CanonicalSurface::Cylinder(a), CanonicalSurface::Cylinder(b)) => {
            if coaxial_axes(a.center(), b.center()) {
                coaxial(&CoaxialPair::CylCyl(a, b))
            } else {
                parallel_cylinders(a, b)
            }
        }
        (CanonicalSurface::Cylinder(a), CanonicalSurface::Cone(b)) => {
            if coaxial_axes(a.center(), b.apex()) {
                coaxial(&CoaxialPair::CylCone(a, b))
            } else {
                return validated_ff(a, b, u_range_l, v_range_l, u_range_r, v_range_r, budget);
            }
        }
        (CanonicalSurface::Cone(a), CanonicalSurface::Cylinder(b)) => {
            if coaxial_axes(a.apex(), b.center()) {
                coaxial(&CoaxialPair::CylCone(b, a))
            } else {
                return validated_ff(a, b, u_range_l, v_range_l, u_range_r, v_range_r, budget);
            }
        }
        (CanonicalSurface::Cylinder(a), CanonicalSurface::Sphere(b)) => {
            if coaxial_axes(a.center(), b.center()) {
                coaxial(&CoaxialPair::CylSphere(a, b))
            } else {
                return validated_ff(a, b, u_range_l, v_range_l, u_range_r, v_range_r, budget);
            }
        }
        (CanonicalSurface::Sphere(a), CanonicalSurface::Cylinder(b)) => {
            if coaxial_axes(a.center(), b.center()) {
                coaxial(&CoaxialPair::CylSphere(b, a))
            } else {
                return validated_ff(a, b, u_range_l, v_range_l, u_range_r, v_range_r, budget);
            }
        }
        (CanonicalSurface::Cone(a), CanonicalSurface::Cone(b)) => {
            if coaxial_axes(a.apex(), b.apex()) {
                coaxial(&CoaxialPair::ConeCone(a, b))
            } else {
                return validated_ff(a, b, u_range_l, v_range_l, u_range_r, v_range_r, budget);
            }
        }
        (CanonicalSurface::Cone(a), CanonicalSurface::Sphere(b)) => {
            if coaxial_axes(a.apex(), b.center()) {
                coaxial(&CoaxialPair::ConeSphere(a, b))
            } else {
                return validated_ff(a, b, u_range_l, v_range_l, u_range_r, v_range_r, budget);
            }
        }
        (CanonicalSurface::Sphere(a), CanonicalSurface::Cone(b)) => {
            if coaxial_axes(a.center(), b.apex()) {
                coaxial(&CoaxialPair::ConeSphere(b, a))
            } else {
                return validated_ff(a, b, u_range_l, v_range_l, u_range_r, v_range_r, budget);
            }
        }
        (CanonicalSurface::Torus(_), _)
        | (_, CanonicalSurface::Torus(_))
        | (CanonicalSurface::Placed(_), _)
        | (_, CanonicalSurface::Placed(_)) => {
            return Err(Refusal::UnsupportedEnvelope(
                EnvelopeCase::ContactReductionDeferred,
            ))
        }
    };
    let Certified { value, .. } = outcome?;
    // The analytic `Coincident` arm is screened before `analytic_records`:
    // emit the Region2/`CoincidentInterval` record only when the two patches'
    // parameter boxes overlap with non-empty interior, otherwise certify
    // empty (BG-SOL-S7-OVERLAP).
    let contacts = match &value {
        AnalyticIntersection::Coincident => {
            if analytic_coincident_screen(l, r, u_range_l, v_range_l, u_range_r, v_range_r) {
                analytic_records(&value)
            } else {
                Vec::new()
            }
        }
        _ => analytic_records(&value),
    };
    let mut props = PropMap::new();
    props.set(Prop::AnalyticCarrier, Truth::True);
    Ok(Certified::new(
        ContactComplex { contacts },
        Certificate {
            props,
            method: Method::Exact,
            budget_left: *budget,
            margin: Margin::UNBOUNDED,
            modulus: Modulus::Unbounded,
        },
    ))
}

/// Whether two faces on the SAME canonical surface carrier have `(u, v)`
/// boxes with non-empty interior overlap (BG-SOL-S7-OVERLAP).
///
/// The same carrier means the same parameterization, so the screen is the box
/// test with the per-carrier periodicity, read off the carriers' own
/// `parameter_range`/`u_period` conventions:
///
/// - `Plane`: `interior_overlap` on u AND v (neither periodic).
/// - `Cylinder`/`Cone`: `periodic_interior_overlap(u, TAU)` AND
///   `interior_overlap(v)` (u is the azimuth; v is z relative to the
///   center/apex).
/// - `Sphere`: `interior_overlap(u)` AND `periodic_interior_overlap(v, TAU)`
///   (u is the POLAR angle on `[0, PI]`, v the azimuth — the swap relative to
///   cylinder/cone).
/// - `Torus`: periodic on BOTH u and v.
/// - `Placed`: struct-equal placements carry the same parameter map; screen
///   the inner carrier with its row of the table.
fn identity_face_boxes_overlap(
    surface: &CanonicalSurface,
    u_l: (f64, f64),
    v_l: (f64, f64),
    u_r: (f64, f64),
    v_r: (f64, f64),
) -> bool {
    match surface {
        CanonicalSurface::Plane(_) => {
            overlap::interior_overlap(u_l, u_r) && overlap::interior_overlap(v_l, v_r)
        }
        CanonicalSurface::Cylinder(_) | CanonicalSurface::Cone(_) => {
            overlap::periodic_interior_overlap(u_l, u_r, TAU) && overlap::interior_overlap(v_l, v_r)
        }
        CanonicalSurface::Sphere(_) => {
            overlap::interior_overlap(u_l, u_r) && overlap::periodic_interior_overlap(v_l, v_r, TAU)
        }
        CanonicalSurface::Torus(_) => {
            overlap::periodic_interior_overlap(u_l, u_r, TAU)
                && overlap::periodic_interior_overlap(v_l, v_r, TAU)
        }
        CanonicalSurface::Placed(placed) => {
            identity_face_boxes_overlap(placed.entity(), u_l, v_l, u_r, v_r)
        }
    }
}

/// The analytic `Coincident` screen (BG-SOL-S7-OVERLAP): whether the two
/// patches' parameter boxes overlap with non-empty interior, checked before
/// `analytic_records` emits the Region2/`CoincidentInterval` record.
///
/// The screen is symmetric in its arguments, so the metamorphic property
/// `C(A, B) = C(B, A)` holds for every screened path.
fn analytic_coincident_screen(
    l: &CanonicalSurface,
    r: &CanonicalSurface,
    u_l: (f64, f64),
    v_l: (f64, f64),
    u_r: (f64, f64),
    v_r: (f64, f64),
) -> bool {
    match (l, r) {
        (CanonicalSurface::Cylinder(a), CanonicalSurface::Cylinder(b)) => {
            // The coaxial cell fired, so `(cx, cy, r)` are equal and the
            // structs differ only in `cz`. u is identical; v differs by the
            // center shift: patch 1's absolute z-extent is
            // `[cz1 + v1.0, cz1 + v1.1]`, patch 2's `[cz2 + v2.0, cz2 + v2.1]`
            // (each endpoint ONE exactly-rounded f64 addition).
            let z_l = a.center().z;
            let z_r = b.center().z;
            let abs_l = (z_l + v_l.0, z_l + v_l.1);
            let abs_r = (z_r + v_r.0, z_r + v_r.1);
            overlap::interior_overlap(abs_l, abs_r)
                && overlap::periodic_interior_overlap(u_l, u_r, TAU)
        }
        (CanonicalSurface::Plane(a), CanonicalSurface::Plane(b)) => {
            plane_coincident_screen(a, b, u_l, v_l, u_r, v_r)
        }
        (CanonicalSurface::Cone(_), CanonicalSurface::Cone(_))
        | (CanonicalSurface::Sphere(_), CanonicalSurface::Sphere(_))
        | (CanonicalSurface::Torus(_), CanonicalSurface::Torus(_)) => {
            // Same-type analytic Coincident implies the same surface and the
            // same parameterization (the struct-unequal sphere/torus cases are
            // unreachable — equal carriers hit the identity arm first); apply
            // the identity-arm table as a defensive screen.
            identity_face_boxes_overlap(l, u_l, v_l, u_r, v_r)
        }
        _ => true,
    }
}

/// The struct-unequal coplanar plane screen (BG-SOL-S7-OVERLAP): solve the
/// parameter correspondence by Cramer in plane `a`'s frame.
///
/// With `subs1(u1, v1) = o1 + u1*U1 + v1*V1` (`U1 = p1 - o1`, `V1 = q1 - o1`,
/// same for plane 2), `n = plane1.normal()` and `det = (U1 x V1) . n`, the
/// affine map `(u1, v1) = M (u2, v2) + c` has entries
///
/// ```text
/// M[0][0] = ((U2 x V1) . n) / det    M[0][1] = ((V2 x V1) . n) / det
/// M[1][0] = ((U1 x U2) . n) / det    M[1][1] = ((U1 x V2) . n) / det
/// c[0] = ((o2 - o1) x V1) . n / det  c[1] = (U1 x (o2 - o1)) . n / det
/// ```
///
/// When `M[0][1] == 0.0 && M[1][0] == 0.0` (the PARALLEL-frame signature —
/// exactly zero for construction data whose frames are exact multiples), the
/// image of box 2 is the axis-aligned rectangle `u1 in [c0 + M00*u2.0,
/// c0 + M00*u2.1]` (ordered by M00's sign) and likewise for v1; overlap is
/// `interior_overlap` on both image intervals. If the off-diagonals are NOT
/// exactly zero (rotated frames), today's emission is kept and the decision
/// is deferred to the booked `BG-SOL-S7-OVERLAP-PLANE` follow-up (3-D SAT).
fn plane_coincident_screen(
    a: &Plane,
    b: &Plane,
    u_l: (f64, f64),
    v_l: (f64, f64),
    u_r: (f64, f64),
    v_r: (f64, f64),
) -> bool {
    let o1 = a.origin();
    let o2 = b.origin();
    let u1 = a.u_axis();
    let v1 = a.v_axis();
    let u2 = b.u_axis();
    let v2 = b.v_axis();
    let n = a.normal();
    let det = u1.cross(v1).dot(n);
    let m00 = u2.cross(v1).dot(n) / det;
    let m01 = v2.cross(v1).dot(n) / det;
    let m10 = u1.cross(u2).dot(n) / det;
    let m11 = u1.cross(v2).dot(n) / det;
    let d = o2.to_vec() - o1.to_vec();
    let c0 = d.cross(v1).dot(n) / det;
    let c1 = u1.cross(d).dot(n) / det;
    if m01 != 0.0 || m10 != 0.0 {
        // Rotated frames: not screened; the decision is deferred.
        return true;
    }
    let image_u = ordered_interval(c0 + m00 * u_r.0, c0 + m00 * u_r.1);
    let image_v = ordered_interval(c1 + m11 * v_r.0, c1 + m11 * v_r.1);
    overlap::interior_overlap(u_l, image_u) && overlap::interior_overlap(v_l, image_v)
}

/// The ordered `(min, max)` image interval endpoint pair.
fn ordered_interval(x: f64, y: f64) -> (f64, f64) {
    if x <= y {
        (x, y)
    } else {
        (y, x)
    }
}

/// The general validated FF stage (BG-SOL-S7-GFF-WIRE): certify the regular
/// crossings of two offset mixed-quadric carriers inside the intersection of
/// their patches' certified AABBs.
///
/// Both carriers stay paired with their `(u, v)` boxes. The certified AABBs
/// (BG-ENC-001) are intersected axiswise into the world search box; a
/// separated axis proves empty contact, and non-finite or empty enclosure
/// data that does not prove separation refuses numerically. For a finite
/// non-degenerate box the resolution floor is `width / 128` and
/// [`gff::cover_branch`] decomposes it into certified crossings, singular
/// cells, and honestly-typed unresolved remainder. The caller owns `budget`:
/// its entry value is captured once, every `cover_branch` refusal is
/// propagated, and no private or replenished budget ever exists here.
fn validated_ff<L, R>(
    l: &L,
    r: &R,
    u_range_l: (f64, f64),
    v_range_l: (f64, f64),
    u_range_r: (f64, f64),
    v_range_r: (f64, f64),
    budget: &mut Budget,
) -> Outcome<ContactComplex>
where
    L: ImplicitField + EnclosureSurface,
    R: ImplicitField + EnclosureSurface,
{
    let initial = *budget;
    let lu = param_interval(u_range_l)?;
    let lv = param_interval(v_range_l)?;
    let ru = param_interval(u_range_r)?;
    let rv = param_interval(v_range_r)?;
    let lhs = l.enclose(lu, lv);
    let rhs = r.enclose(ru, rv);
    // Intersect the certified AABBs axiswise. A separated axis proves empty
    // contact; the endpoints involved are all finite for well-formed data, so
    // bad data reaches this proof only when it is genuinely separated and
    // otherwise refuses numerically below — never a silent no-contact answer.
    let ix = (lhs.x.inf().max(rhs.x.inf()), lhs.x.sup().min(rhs.x.sup()));
    let iy = (lhs.y.inf().max(rhs.y.inf()), lhs.y.sup().min(rhs.y.sup()));
    let iz = (lhs.z.inf().max(rhs.z.inf()), lhs.z.sup().min(rhs.z.sup()));
    if ix.0 > ix.1 || iy.0 > iy.1 || iz.0 > iz.1 {
        return Ok(Certified::new(
            ContactComplex {
                contacts: Vec::new(),
            },
            Certificate {
                props: PropMap::new(),
                method: Method::Interval,
                budget_left: *budget,
                margin: Margin::UNBOUNDED,
                modulus: Modulus::Unbounded,
            },
        ));
    }
    // No separated axis: the intersection is the world search box. Empty or
    // non-finite enclosure data certifies neither separation nor contact.
    if !well_formed_box(&lhs) || !well_formed_box(&rhs) {
        return Err(Refusal::NumericallyUnresolved {
            spent: budget_spent(&initial, budget),
            witness: UnresolvedWitness::KrawczykIndeterminate,
        });
    }
    let Some(domain) = intersect_boxes(&lhs, &rhs) else {
        return Err(Refusal::NumericallyUnresolved {
            spent: budget_spent(&initial, budget),
            witness: UnresolvedWitness::KrawczykIndeterminate,
        });
    };
    // Scale-relative resolution floor: the AABB's widest axis divided by a
    // named dimensionless divisor. A non-finite or non-positive width or tau
    // cannot be certified, so it refuses numerically.
    let width = domain.width();
    let tau = width / TAU_DIVISOR;
    // A NaN width fails the finiteness tests before the `<=` comparisons can
    // be reached, so the two are equivalent to "width and tau are positive".
    if !width.is_finite() || width <= 0.0 || !tau.is_finite() || tau <= 0.0 {
        return Err(Refusal::NumericallyUnresolved {
            spent: budget_spent(&initial, budget),
            witness: UnresolvedWitness::KrawczykIndeterminate,
        });
    }
    // The cover is certified by the caller's budget; every refusal from
    // `cover_branch` (budget exhaustion) is propagated as-is.
    let Certified { value: cover, cert } = gff::cover_branch(l, r, &domain, tau, budget)?;
    // Completion rules (decision 4), applied in order. The singular-event
    // stage (BG-SOL-S7-SING-CLASSIFY) refines every singular cell: it recovers
    // the regular crossings hiding inside broad singular domains, certifies
    // isolated tangencies as `Point0`/`Tangency` records, and defers
    // tangential crossings, carrier-degenerate contacts, and anything it
    // cannot prove with `ContactReductionDeferred`.
    let mut cover = cover;
    let mut tangencies: Vec<Point3> = Vec::new();
    let mut singular_cert: Option<Certificate> = None;
    if !cover.singular_boxes.is_empty() {
        let Certified {
            value: report,
            cert: scert,
        } = singular::singular_events(l, r, &cover.singular_boxes, tau, budget)?;
        let singular::SingularReport {
            regular,
            tangencies: t,
            tangential_crossings,
            degenerate,
            residue,
        } = report;
        tangencies = t;
        cover.points.extend(regular.points);
        cover.unresolved_boxes.extend(regular.unresolved_boxes);
        if !residue.is_empty() || !tangential_crossings.is_empty() || !degenerate.is_empty() {
            return Err(Refusal::UnsupportedEnvelope(
                EnvelopeCase::ContactReductionDeferred,
            ));
        }
        singular_cert = Some(scert);
    }
    if !cover.unresolved_boxes.is_empty() {
        return Err(Refusal::NumericallyUnresolved {
            spent: budget_spent(&initial, budget),
            witness: UnresolvedWitness::KrawczykIndeterminate,
        });
    }
    // The certified isolated tangencies first (discovery order), then the
    // regular branch cover when it certified crossings.
    let mut contacts: Vec<ContactRecord> = Vec::new();
    for p in tangencies {
        contacts.push(ContactRecord {
            dimension: ContactDimension::Point0,
            kind: ContactEventKind::Tangency,
            locus: ContactLocus::Point(p),
        });
    }
    if !cover.points.is_empty() {
        contacts.push(ContactRecord {
            dimension: ContactDimension::Arc1,
            kind: ContactEventKind::Transverse,
            locus: ContactLocus::ValidatedBranchCover(cover),
        });
    }
    // The certificate is the `singular_events` cert (actual `budget_left`)
    // when the singular path ran; otherwise the `cover_branch` cert is
    // returned unchanged.
    let out_cert = singular_cert.unwrap_or(cert);
    Ok(Certified::new(ContactComplex { contacts }, out_cert))
}

/// The dimensionless divisor that scales a certified AABB's width into the
/// resolution floor of `gff::cover_branch`: `tau = width / TAU_DIVISOR`
/// (decision 3).
const TAU_DIVISOR: f64 = 128.0;

/// A certified parameter interval from a face's stored `(f64, f64)` bounds.
///
/// Endpoints must be finite and ordered (`lo <= hi`); a reversed periodic
/// bound may mean seam crossing and is not empty, so it refuses with the
/// deferred envelope. `Interval::try_from` is used, never unwrap.
fn param_interval(range: (f64, f64)) -> Result<Interval, Refusal> {
    let (lo, hi) = range;
    if !lo.is_finite() || !hi.is_finite() || lo > hi {
        return Err(Refusal::UnsupportedEnvelope(
            EnvelopeCase::ContactReductionDeferred,
        ));
    }
    Interval::try_from((lo, hi))
        .map_err(|_| Refusal::UnsupportedEnvelope(EnvelopeCase::ContactReductionDeferred))
}

/// Whether a certified AABB's three axis intervals are finite (an empty
/// interval is NaN on both endpoints, so finiteness implies non-empty).
fn well_formed_box(b: &Box3) -> bool {
    let ok = |i: Interval| i.inf().is_finite() && i.sup().is_finite();
    ok(b.x) && ok(b.y) && ok(b.z)
}

/// The certified axiswise intersection of two AABBs as a `Box3`. `None` when
/// the intersection is not well-formed; the caller has already proven no axis
/// is separated and both boxes are finite, so this is purely defensive.
fn intersect_boxes(a: &Box3, b: &Box3) -> Option<Box3> {
    let axis = |ai: Interval, bi: Interval| {
        Interval::try_from((ai.inf().max(bi.inf()), ai.sup().min(bi.sup()))).ok()
    };
    Some(Box3 {
        x: axis(a.x, b.x)?,
        y: axis(a.y, b.y)?,
        z: axis(a.z, b.z)?,
    })
}

/// Spend since entry: the entry budget minus what remains (mirrored from
/// `gff::cover_branch`). Never the REMAINING budget as `spent` — that hides
/// exhaustion.
fn budget_spent(initial: &Budget, budget: &Budget) -> Budget {
    Budget {
        subdiv: initial.subdiv - budget.subdiv,
        newton: initial.newton - budget.newton,
        depth: initial.depth - budget.depth,
    }
}

/// Map an analytic intersection arm onto the shared 2-D ontology.
///
/// - `Curve`/`TwoCurves` → `Arc1` / `Transverse`
/// - `Tangent*` → `Arc1` / `Tangency`
/// - `Parallel`/`Empty` → no contact: an empty `ContactComplex`
/// - `Coincident` → `Region2` / `CoincidentInterval`
fn analytic_records(value: &AnalyticIntersection) -> Vec<ContactRecord> {
    match value {
        AnalyticIntersection::Curve(_) | AnalyticIntersection::TwoCurves(_) => {
            vec![ContactRecord {
                dimension: ContactDimension::Arc1,
                kind: ContactEventKind::Transverse,
                locus: ContactLocus::Analytic(value.clone()),
            }]
        }
        AnalyticIntersection::TangentPoint(_)
        | AnalyticIntersection::TangentLine(_)
        | AnalyticIntersection::TangentCircle(_) => vec![ContactRecord {
            dimension: ContactDimension::Arc1,
            kind: ContactEventKind::Tangency,
            locus: ContactLocus::Analytic(value.clone()),
        }],
        AnalyticIntersection::Parallel | AnalyticIntersection::Empty => Vec::new(),
        AnalyticIntersection::Coincident => vec![ContactRecord {
            dimension: ContactDimension::Region2,
            kind: ContactEventKind::CoincidentInterval,
            locus: ContactLocus::Analytic(value.clone()),
        }],
    }
}

#[cfg(test)]
// Test-only allow: H-1 bans unwrap/expect/panic on paths reachable from
// untrusted geometry. Unit-test assertions on hand-built dyadic witnesses are
// not such a path; the unwraps and the let-else panic below cannot fire for
// the values constructed.
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::analytic::ExactCurve;
    use std::f64::consts::{FRAC_PI_2, FRAC_PI_4, PI, TAU};
    use truck_geometry::prelude::*;
    use truck_geometry::recognize::recognize_surface;

    /// A face stratum on a canonical surface with the unit `(u, v)` box.
    fn face(surface: CanonicalSurface) -> BoundedStratum {
        BoundedStratum::Face {
            surface,
            u_range: (0.0, 1.0),
            v_range: (0.0, 1.0),
        }
    }

    /// A face stratum on a canonical surface with a custom `(u, v)` box
    /// (BG-SOL-S7-GFF-WIRE: each carrier stays paired with its bounds).
    fn face_with_bounds(
        surface: CanonicalSurface,
        u_range: (f64, f64),
        v_range: (f64, f64),
    ) -> BoundedStratum {
        BoundedStratum::Face {
            surface,
            u_range,
            v_range,
        }
    }

    /// The certified crossing points of a validated cover, for set-wise
    /// comparison of two orders.
    fn validated_points(out: &Certified<ContactComplex>) -> Vec<Point3> {
        let mut points = Vec::new();
        for record in &out.value.contacts {
            if let ContactLocus::ValidatedBranchCover(cover) = &record.locus {
                points.extend(cover.points.iter().copied());
            }
        }
        points
    }

    /// A full-range unit circle in the z = 0 plane centered at the origin,
    /// for the Edge identity-arm witnesses.
    fn placed_unit_circle() -> Processor<TrimmedCurve<UnitCircle<Point3>>, Matrix4> {
        let m = Matrix4 {
            x: Vector4::new(1.0, 0.0, 0.0, 0.0),
            y: Vector4::new(0.0, 1.0, 0.0, 0.0),
            z: Vector4::new(0.0, 0.0, 1.0, 0.0),
            w: Vector4::new(0.0, 0.0, 0.0, 1.0),
        };
        Processor::with_transform(
            TrimmedCurve::new(UnitCircle::<Point3>::new(), (0.0, TAU)),
            m,
        )
    }

    #[test]
    fn contact_ff_plane_plane_transverse_returns_analytic_line() {
        // z = 0 (xy-plane) and y = 0 (xz-plane) cross in the x-axis: a dyadic
        // transverse pair whose line is decided exactly.
        let z0 = Plane::new(
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        );
        let y0 = Plane::new(
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 0.0, 1.0),
        );
        let lhs = face(CanonicalSurface::Plane(z0));
        let rhs = face(CanonicalSurface::Plane(y0));
        let mut budget = Budget::new(100, 100, 100);
        let out =
            contact(&lhs, &rhs, &mut budget).expect("a dyadic transverse plane pair is decidable");
        assert_eq!(out.cert.method, Method::Exact);
        assert_eq!(out.cert.props.get(Prop::AnalyticCarrier), Truth::True);
        assert_eq!(out.value.contacts.len(), 1);
        let record = out.value.contacts.first().expect("one record");
        assert_eq!(record.dimension, ContactDimension::Arc1);
        assert_eq!(record.kind, ContactEventKind::Transverse);
        assert!(
            matches!(
                &record.locus,
                ContactLocus::Analytic(AnalyticIntersection::Curve(ExactCurve::Line(_)))
            ),
            "a transverse plane pair emits an exact line locus"
        );
        // The stratum vocabulary is storable: `BoundedStratum` is
        // `Clone + Debug + PartialEq`, so future packets can hold strata.
        assert_eq!(lhs.clone(), lhs);
        let _printed = format!("{lhs:?} {rhs:?}");
    }

    #[test]
    fn contact_ff_coincident_planes_returns_coincident() {
        // `Plane` stores its defining point triple verbatim (no canonical
        // normalization), so two `Plane::new` calls from *distinct* triples on
        // the same geometric plane are not `PartialEq`-equal carriers and the
        // C0-C2 identity stage cannot fire on them (see RESULT.json
        // disagreements). The identity stage is exercised with two construction
        // paths that produce the same carrier; a distinct-triple coincident
        // pair still lands in the analytic stage instead.
        let lhs = face(CanonicalSurface::Plane(Plane::xy()));
        let rhs = face(CanonicalSurface::Plane(Plane::new(
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        )));
        let mut budget = Budget::new(100, 100, 100);
        let out = contact(&lhs, &rhs, &mut budget)
            .expect("equal dyadic carriers decide at the identity stage");
        assert_eq!(out.cert.method, Method::Exact);
        assert_eq!(out.cert.props.get(Prop::AnalyticCarrier), Truth::True);
        assert_eq!(out.value.contacts.len(), 1);
        let record = out.value.contacts.first().expect("one record");
        assert_eq!(record.dimension, ContactDimension::Region2);
        assert_eq!(record.kind, ContactEventKind::IdenticalCarrier);
        assert!(matches!(record.locus, ContactLocus::Coincident));
    }

    #[test]
    fn contact_ff_plane_cylinder_returns_analytic() {
        // A plane perpendicular to the cylinder's z axis through its center
        // cuts a circle: dyadic carrier parameters, decided exactly.
        let plane = Plane::new(
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        );
        let cylinder = Cylinder::new(Point3::new(0.0, 0.0, 0.0), 1.0)
            .expect("a unit cylinder is a valid carrier")
            .value;
        let lhs = face(CanonicalSurface::Plane(plane));
        let rhs = face(CanonicalSurface::Cylinder(cylinder));
        let mut budget = Budget::new(100, 100, 100);
        let out = contact(&lhs, &rhs, &mut budget)
            .expect("a dyadic perpendicular plane/cylinder pair is decidable");
        assert_eq!(out.cert.method, Method::Exact);
        assert_eq!(out.value.contacts.len(), 1);
        let record = out.value.contacts.first().expect("one record");
        assert_eq!(record.dimension, ContactDimension::Arc1);
        assert!(matches!(record.locus, ContactLocus::Analytic(_)));
    }

    #[test]
    fn contact_ff_spline_surface_refuses() {
        // A BSplineSurface is not a canonical analytic carrier: the structural
        // recognizer returns `Unrecognized`. `BoundedStratum::Face` carries a
        // `CanonicalSurface`, which has no `Unrecognized` arm, so the Contact
        // Layer's refusal for this carrier is enforced at the stratum-lift
        // boundary `face_stratum` — the same `ContactReductionDeferred` the
        // dispatcher reports for the rest of the deferred funnel (plan §4
        // Phase 3).
        let bspline = BSplineSurface::try_new(
            (KnotVec::bezier_knot(1), KnotVec::bezier_knot(1)),
            vec![
                vec![Point3::new(0.0, 0.0, 0.0), Point3::new(0.0, 1.0, 0.0)],
                vec![Point3::new(1.0, 0.0, 0.0), Point3::new(1.0, 1.0, 0.0)],
            ],
        )
        .expect("a bilinear patch is a valid B-spline surface");
        let witness = recognize_surface(&Surface::BSplineSurface(bspline));
        assert!(
            matches!(witness, CanonicalCarrierWitness::Unrecognized),
            "a spline carrier has no canonical analytic form"
        );
        let lifted = face_stratum(witness, (0.0, 1.0), (0.0, 1.0));
        assert!(
            matches!(
                lifted,
                Err(Refusal::UnsupportedEnvelope(
                    EnvelopeCase::ContactReductionDeferred
                ))
            ),
            "an unrecognized carrier refuses with ContactReductionDeferred"
        );
    }

    #[test]
    fn contact_fe_stratum_refuses_deferred() {
        // An FE pair from a family outside the landed strata-reduction table:
        // a line edge against a cone face. Line×Cone is not in the §5 FE table,
        // so the pair still hits the deferred funnel.
        let cone = Cone::new(Point3::new(0.0, 0.0, 0.0), 0.5)
            .expect("a dyadic cone is a valid carrier")
            .value;
        let face = face(CanonicalSurface::Cone(cone));
        let edge = BoundedStratum::Edge {
            curve: CanonicalCurve::Line(Line(
                Point3::new(0.0, 0.0, 0.0),
                Point3::new(1.0, 0.0, 0.0),
            )),
            t_range: (0.0, 1.0),
        };
        let mut budget = Budget::new(100, 100, 100);
        let out = contact(&face, &edge, &mut budget);
        assert!(
            matches!(
                out,
                Err(Refusal::UnsupportedEnvelope(
                    EnvelopeCase::ContactReductionDeferred
                ))
            ),
            "a Line×Cone FE stratum pair is the deferred funnel"
        );
    }

    #[test]
    fn contact_ff_cylinder_cylinder_parallel_returns_two_lines() {
        // Two offset parallel cylinders: axes at (0, 0) and (1.5, 0), both
        // radius 1. The axis distance 1.5 lies strictly between r0 + r1 = 2
        // and |r0 − r1| = 0, so the parallel-axis cell emits two transverse
        // lines (the `TwoCurves` arm).
        let cyl0 = Cylinder::new(Point3::new(0.0, 0.0, 0.0), 1.0)
            .expect("a unit cylinder is a valid carrier")
            .value;
        let cyl1 = Cylinder::new(Point3::new(1.5, 0.0, 0.0), 1.0)
            .expect("a unit cylinder is a valid carrier")
            .value;
        let lhs = face(CanonicalSurface::Cylinder(cyl0));
        let rhs = face(CanonicalSurface::Cylinder(cyl1));
        let mut budget = Budget::new(100, 100, 100);
        let out = contact(&lhs, &rhs, &mut budget)
            .expect("a dyadic offset parallel cylinder pair is decidable");
        assert_eq!(out.cert.method, Method::Exact);
        assert_eq!(out.cert.props.get(Prop::AnalyticCarrier), Truth::True);
        assert_eq!(out.value.contacts.len(), 1);
        let record = out.value.contacts.first().expect("one record");
        assert_eq!(record.dimension, ContactDimension::Arc1);
        assert_eq!(record.kind, ContactEventKind::Transverse);
        assert!(
            matches!(
                &record.locus,
                ContactLocus::Analytic(AnalyticIntersection::TwoCurves([
                    ExactCurve::Line(_),
                    ExactCurve::Line(_),
                ]))
            ),
            "an offset parallel cylinder pair emits two transverse lines"
        );
    }

    #[test]
    fn contact_ff_cylinder_cylinder_coaxial_returns_empty() {
        // Two coaxial cylinders of different radii: the carriers are
        // struct-unequal, so the C0-C2 identity stage cannot fire, and the
        // analytic `coaxial(CylCyl)` arm answers `Empty` — no contact.
        let cyl0 = Cylinder::new(Point3::new(0.0, 0.0, 0.0), 1.0)
            .expect("a unit cylinder is a valid carrier")
            .value;
        let cyl1 = Cylinder::new(Point3::new(0.0, 0.0, 0.0), 2.0)
            .expect("a unit cylinder is a valid carrier")
            .value;
        let lhs = face(CanonicalSurface::Cylinder(cyl0));
        let rhs = face(CanonicalSurface::Cylinder(cyl1));
        let mut budget = Budget::new(100, 100, 100);
        let out =
            contact(&lhs, &rhs, &mut budget).expect("a dyadic coaxial cylinder pair is decidable");
        assert_eq!(out.cert.method, Method::Exact);
        assert!(
            out.value.contacts.is_empty(),
            "concentric cylinders of different radii meet nowhere"
        );
    }

    #[test]
    fn contact_ff_cylinder_cone_coaxial_returns_analytic() {
        // A cylinder (0,0,0) r = 1 and a cone apex (0,0,0) tan = 3/4 are
        // coaxial; the cone's lateral surface meets the cylinder in two
        // circles at z = ±4/3 of radius 1 (the `TwoCurves` arm), which maps
        // to exactly one `Arc1` / `Transverse` record.
        let cyl_face = face(CanonicalSurface::Cylinder(
            Cylinder::new(Point3::new(0.0, 0.0, 0.0), 1.0)
                .expect("a unit cylinder is a valid carrier")
                .value,
        ));
        let cone_face = face(CanonicalSurface::Cone(
            Cone::new(Point3::new(0.0, 0.0, 0.0), (3.0 / 4.0f64).atan())
                .expect("a dyadic cone is a valid carrier")
                .value,
        ));
        let mut budget = Budget::new(100, 100, 100);
        let out = contact(&cyl_face, &cone_face, &mut budget)
            .expect("a dyadic coaxial cylinder/cone pair is decidable");
        assert_eq!(out.cert.method, Method::Exact);
        assert_eq!(out.value.contacts.len(), 1);
        let record = out.value.contacts.first().expect("one record");
        assert_eq!(record.dimension, ContactDimension::Arc1);
        assert!(matches!(record.locus, ContactLocus::Analytic(_)));

        // The metamorphic property: the swapped order produces a structurally
        // equal `ContactComplex` (the coaxial cell is order-insensitive).
        let mut budget = Budget::new(100, 100, 100);
        let swapped = contact(&cone_face, &cyl_face, &mut budget)
            .expect("the swapped coaxial pair is decidable");
        assert_eq!(
            format!("{out:?}"),
            format!("{swapped:?}"),
            "contact(cylinder, cone) and contact(cone, cylinder) must agree"
        );
    }

    #[test]
    fn contact_ff_cylinder_sphere_coaxial_returns_analytic() {
        // A cylinder (0,0,0) r = 1 and a sphere centered at the origin
        // r = 2: the wall circle x²+y² = 1 lies in the sphere at z² = 3, so
        // the coaxial cell emits two circles.
        let cyl_face = face(CanonicalSurface::Cylinder(
            Cylinder::new(Point3::new(0.0, 0.0, 0.0), 1.0)
                .expect("a unit cylinder is a valid carrier")
                .value,
        ));
        let sph_face = face(CanonicalSurface::Sphere(Sphere::new(
            Point3::new(0.0, 0.0, 0.0),
            2.0,
        )));
        let mut budget = Budget::new(100, 100, 100);
        let out = contact(&cyl_face, &sph_face, &mut budget)
            .expect("a dyadic coaxial cylinder/sphere pair is decidable");
        assert_eq!(out.cert.method, Method::Exact);
        let record = out.value.contacts.first().expect("at least one record");
        assert_eq!(record.dimension, ContactDimension::Arc1);
        assert!(matches!(record.locus, ContactLocus::Analytic(_)));
    }

    #[test]
    fn contact_ff_cone_cone_coaxial_returns_analytic() {
        // Two coaxial cones, apexes (0,0,0) tan 3/4 and (0,0,1) tan 1/2:
        // different angles on a shared axis, they meet in two circles (the
        // coaxial module's own test proves the `TwoCurves` arm for this
        // witness), one `Arc1` / `Transverse` record.
        let cone0 = face(CanonicalSurface::Cone(
            Cone::new(Point3::new(0.0, 0.0, 0.0), (3.0 / 4.0f64).atan())
                .expect("a dyadic cone is a valid carrier")
                .value,
        ));
        let cone1 = face(CanonicalSurface::Cone(
            Cone::new(Point3::new(0.0, 0.0, 1.0), (1.0 / 2.0f64).atan())
                .expect("a dyadic cone is a valid carrier")
                .value,
        ));
        let mut budget = Budget::new(100, 100, 100);
        let out =
            contact(&cone0, &cone1, &mut budget).expect("a dyadic coaxial cone pair is decidable");
        assert_eq!(out.cert.method, Method::Exact);
        assert_eq!(out.value.contacts.len(), 1);
        let record = out.value.contacts.first().expect("one record");
        assert_eq!(record.dimension, ContactDimension::Arc1);
        assert_eq!(record.kind, ContactEventKind::Transverse);
        assert!(
            matches!(
                &record.locus,
                ContactLocus::Analytic(AnalyticIntersection::TwoCurves(_))
            ),
            "two coaxial cones of different angles meet in two circles"
        );
    }

    /// The shared regular witnesses (BG-SOL-S7-GFF-WIRE): the unit z-cylinder
    /// at the origin, cone A (apex origin, tan α = 1), cone B (apex (1,0,0),
    /// tan α = 1), and the sphere center (2,0,0) radius 2. Every pair's
    /// certified crossing includes p = (1/2, √3/2, 1), which satisfies all
    /// four machine-checked identities.
    fn offset_witnesses() -> (
        BoundedStratum,
        BoundedStratum,
        BoundedStratum,
        BoundedStratum,
    ) {
        let cyl = face_with_bounds(
            CanonicalSurface::Cylinder(
                Cylinder::new(Point3::new(0.0, 0.0, 0.0), 1.0)
                    .expect("a unit cylinder is a valid carrier")
                    .value,
            ),
            (0.8, 1.3),
            (0.8, 1.2),
        );
        let cone_a = face_with_bounds(
            CanonicalSurface::Cone(
                Cone::new(Point3::new(0.0, 0.0, 0.0), FRAC_PI_4)
                    .expect("a dyadic cone is a valid carrier")
                    .value,
            ),
            (0.8, 1.3),
            (0.8, 1.2),
        );
        let cone_b = face_with_bounds(
            CanonicalSurface::Cone(
                Cone::new(Point3::new(1.0, 0.0, 0.0), FRAC_PI_4)
                    .expect("a dyadic cone is a valid carrier")
                    .value,
            ),
            (0.0, PI),
            (0.8, 1.2),
        );
        let sphere = face_with_bounds(
            CanonicalSurface::Sphere(Sphere::new(Point3::new(2.0, 0.0, 0.0), 2.0)),
            (0.0, PI),
            (0.0, TAU),
        );
        (cyl, cone_a, cone_b, sphere)
    }

    /// Unit-scale residual on certified cover point locations, for comparing
    /// two orders' point sets (BG-SOL-S7-GFF-WIRE).
    const COVER_RESIDUAL: f64 = 1.0e-6; // H-3: unit-scale residual on certified crossing locations, not a length

    /// The healthy subdivision budget the regular witnesses certify under.
    const COVER_BUDGET: u32 = 4096; // H-3: a subdivision budget counter, not a length

    #[test]
    fn contact_ff_offset_mixed_quadric_pairs_return_validated_cover() {
        // The four regular offset mixed-quadric cells each emit exactly one
        // `Arc1`/`Transverse` validated branch cover with non-empty points and
        // empty singular/unresolved lists under a healthy budget.
        let (cyl, cone_a, cone_b, sphere) = offset_witnesses();
        let pairs = [
            (&cyl, &cone_b),
            (&cyl, &sphere),
            (&cone_a, &cone_b),
            (&cone_a, &sphere),
        ];
        for (lhs, rhs) in pairs {
            let mut budget = Budget::new(COVER_BUDGET, 0, 0);
            let out = contact(lhs, rhs, &mut budget)
                .expect("a regular offset mixed-quadric pair certifies under healthy budget");
            assert_eq!(out.cert.method, Method::Interval);
            assert_eq!(out.value.contacts.len(), 1);
            let record = out.value.contacts.first().expect("one record");
            assert_eq!(record.dimension, ContactDimension::Arc1);
            assert_eq!(record.kind, ContactEventKind::Transverse);
            let ContactLocus::ValidatedBranchCover(cover) = &record.locus else {
                panic!("an offset mixed-quadric pair emits a validated branch cover");
            };
            assert!(
                !cover.points.is_empty(),
                "the cover certifies crossings on the shared zero set"
            );
            assert!(
                cover.singular_boxes.is_empty(),
                "a regular pair proves no singular cells"
            );
            assert!(
                cover.unresolved_boxes.is_empty(),
                "a regular pair proves no unresolved cells"
            );
        }
    }

    #[test]
    fn contact_ff_offset_mixed_quadric_cover_is_order_insensitive() {
        // Cylinder/Cone B in both orders, bounds kept with the carriers. The
        // two covers' point sets agree order-insensitively within a named
        // unit-scale residual; discovery order need not match.
        let (cyl, _cone_a, cone_b, _sphere) = offset_witnesses();
        let mut budget = Budget::new(COVER_BUDGET, 0, 0);
        let fwd = contact(&cyl, &cone_b, &mut budget)
            .expect("the forward order certifies under healthy budget");
        let mut budget = Budget::new(COVER_BUDGET, 0, 0);
        let rev = contact(&cone_b, &cyl, &mut budget)
            .expect("the reversed order certifies under healthy budget");
        let fwd_points = validated_points(&fwd);
        let rev_points = validated_points(&rev);
        assert_eq!(
            fwd_points.len(),
            rev_points.len(),
            "both orders certify the same number of crossings"
        );
        for p in &fwd_points {
            assert!(
                rev_points
                    .iter()
                    .any(|q| (*p - *q).magnitude() <= COVER_RESIDUAL),
                "forward point {p:?} has no match in the reversed cover"
            );
        }
        for q in &rev_points {
            assert!(
                fwd_points
                    .iter()
                    .any(|p| (*p - *q).magnitude() <= COVER_RESIDUAL),
                "reversed point {q:?} has no match in the forward cover"
            );
        }
    }

    #[test]
    fn contact_ff_offset_disjoint_aabbs_return_empty() {
        // The unit cylinder patch versus a full sphere centered (10,0,0)
        // radius 1: the certified AABBs are separated on x, so the pair proves
        // empty contact without spending anything from the budget.
        let (cyl, _cone_a, _cone_b, _sphere) = offset_witnesses();
        let far_sphere = face_with_bounds(
            CanonicalSurface::Sphere(Sphere::new(Point3::new(10.0, 0.0, 0.0), 1.0)),
            (0.0, PI),
            (0.0, TAU),
        );
        let entry = Budget::new(128, 0, 0);
        let mut budget = entry;
        let out = contact(&cyl, &far_sphere, &mut budget)
            .expect("a separated AABB pair proves empty contact");
        assert!(
            out.value.contacts.is_empty(),
            "a cylinder patch and a far sphere meet nowhere"
        );
        assert_eq!(out.cert.method, Method::Interval);
        assert_eq!(
            out.cert.budget_left, entry,
            "an early disjoint AABB spends nothing"
        );
        assert_eq!(budget, entry, "the caller's budget is untouched");
        assert_eq!(
            out.cert.props.get(Prop::AnalyticCarrier),
            Truth::Unknown,
            "the validated path never stamps AnalyticCarrier"
        );
    }

    #[test]
    fn contact_ff_offset_tangent_pair_stays_deferred_for_singular_stage() {
        // The name is historical; the assertions are the NEW contract
        // (BG-SOL-S7-SING-CLASSIFY). The unit cylinder is externally tangent
        // to the sphere center (2,0,0) radius 1 at exactly (1,0,0). The
        // singular stage now CERTIFIES that isolated tangency: `contact()`
        // returns Ok with exactly one `Point0`/`Tangency` record at (1,0,0)
        // on an interval certificate.
        let cyl = face_with_bounds(
            CanonicalSurface::Cylinder(
                Cylinder::new(Point3::new(0.0, 0.0, 0.0), 1.0)
                    .expect("a unit cylinder is a valid carrier")
                    .value,
            ),
            (-0.4, 0.4),
            (-0.5, 0.5),
        );
        let tangent_sphere = face_with_bounds(
            CanonicalSurface::Sphere(Sphere::new(Point3::new(2.0, 0.0, 0.0), 1.0)),
            (FRAC_PI_2 - 0.3, FRAC_PI_2 + 0.3),
            (PI - 0.3, PI + 0.3),
        );
        let mut budget = Budget::new(COVER_BUDGET, 0, 0);
        let out = contact(&cyl, &tangent_sphere, &mut budget)
            .expect("the singular stage certifies the isolated external tangency");
        assert_eq!(out.cert.method, Method::Interval);
        assert_eq!(out.value.contacts.len(), 1);
        let record = out.value.contacts.first().expect("one record");
        assert_eq!(record.dimension, ContactDimension::Point0);
        assert_eq!(record.kind, ContactEventKind::Tangency);
        match &record.locus {
            ContactLocus::Point(p) => assert!(
                (*p - Point3::new(1.0, 0.0, 0.0)).magnitude() <= COVER_RESIDUAL,
                "the certified tangency is at (1,0,0)"
            ),
            other => panic!("the tangency locus is a Point, got {other:?}"),
        }
    }

    #[test]
    fn contact_ff_internal_tangency_pair_stays_deferred() {
        // Witness 2 at the dispatcher level: the unit cylinder is internally
        // tangent to the sphere center (1,0,0) radius 2 at (-1,0,0), where
        // the contact locus self-crosses. The singular stage classifies the
        // pinch as a tangential crossing (indefinite restricted Hessian) and
        // defers the pair: a saddle is never an isolated tangency.
        let cyl = face_with_bounds(
            CanonicalSurface::Cylinder(
                Cylinder::new(Point3::new(0.0, 0.0, 0.0), 1.0)
                    .expect("a unit cylinder is a valid carrier")
                    .value,
            ),
            (PI - 0.4, PI + 0.4),
            (-0.5, 0.5),
        );
        let internal_sphere = face_with_bounds(
            CanonicalSurface::Sphere(Sphere::new(Point3::new(1.0, 0.0, 0.0), 2.0)),
            (FRAC_PI_2 - 0.3, FRAC_PI_2 + 0.3),
            (PI - 0.3, PI + 0.3),
        );
        let mut budget = Budget::new(COVER_BUDGET, 0, 0);
        let out = contact(&cyl, &internal_sphere, &mut budget);
        assert!(
            matches!(
                out,
                Err(Refusal::UnsupportedEnvelope(
                    EnvelopeCase::ContactReductionDeferred
                ))
            ),
            "an internal tangency stays deferred: the saddle is not an isolated tangency"
        );
    }

    #[test]
    fn contact_ff_non_coaxial_curved_pair_refuses_deferred() {
        // BG-SOL-S7-GFF-WIRE: the offset mixed-quadric cells no longer fall
        // into the plain deferred funnel — `analytic_ff` routes them to the
        // validated FF stage, which then classifies each witness. The cone
        // apex (1,0,0) sits exactly on the cylinder wall, so the world box's
        // slab Jacobian determinant (det = 4y) straddles zero and the cover
        // classifies it singular: the pair still returns
        // ContactReductionDeferred, but from the cover's singular list, not
        // from the dispatch table. The unit-patch cylinder × sphere pair is
        // AABB-separated on x (cylinder x ≤ 1, sphere x ≥ 2), so it certifies
        // empty.
        let cyl_face = face(CanonicalSurface::Cylinder(
            Cylinder::new(Point3::new(0.0, 0.0, 0.0), 1.0)
                .expect("a unit cylinder is a valid carrier")
                .value,
        ));
        let off_cone = face(CanonicalSurface::Cone(
            Cone::new(Point3::new(1.0, 0.0, 0.0), (3.0 / 4.0f64).atan())
                .expect("a dyadic cone is a valid carrier")
                .value,
        ));
        let off_sphere = face(CanonicalSurface::Sphere(Sphere::new(
            Point3::new(2.0, 0.0, 0.0),
            2.0,
        )));
        let mut budget = Budget::new(100, 100, 100);
        let out = contact(&cyl_face, &off_cone, &mut budget);
        assert!(
            matches!(
                out,
                Err(Refusal::UnsupportedEnvelope(
                    EnvelopeCase::ContactReductionDeferred
                ))
            ),
            "an off-axis cylinder/cone pair with its apex on the wall is singular"
        );
        let mut budget = Budget::new(100, 100, 100);
        let out = contact(&cyl_face, &off_sphere, &mut budget)
            .expect("an AABB-separated cylinder/sphere pair certifies empty");
        assert!(
            out.value.contacts.is_empty(),
            "unit patches of the cylinder and the (2,0,0) r=2 sphere meet nowhere"
        );
        assert_eq!(out.cert.method, Method::Interval);
    }

    #[test]
    fn overlap_screen_identity_face_disjoint_boxes_certify_empty() {
        // The same canonical plane carrier with disjoint `(u, v)` boxes: the
        // two sides of a shared wall report NO contact — a certified empty
        // complex on `Method::Exact` with empty props and an untouched budget
        // (BG-SOL-S7-OVERLAP).
        let plane = CanonicalSurface::Plane(Plane::xy());
        let lhs = face_with_bounds(plane.clone(), (0.0, 1.0), (0.0, 1.0));
        let rhs = face_with_bounds(plane, (2.0, 3.0), (2.0, 3.0));
        let entry = Budget::new(100, 100, 100);
        let mut budget = entry;
        let out = contact(&lhs, &rhs, &mut budget)
            .expect("same-carrier disjoint plane boxes decide empty at the identity stage");
        assert!(
            out.value.contacts.is_empty(),
            "disjoint patches of the same plane never touch"
        );
        assert_eq!(out.cert.method, Method::Exact);
        assert_eq!(out.cert.budget_left, entry, "the screen spends nothing");
        assert_eq!(budget, entry, "the caller's budget is untouched");
        assert_eq!(
            out.cert.props.get(Prop::AnalyticCarrier),
            Truth::Unknown,
            "the identity empty complex carries empty props"
        );

        // The same unit cylinder, v ranges (0,1) vs (5,6): absolute z extents
        // [0,1] and [5,6] are disjoint, so the wall patches never touch.
        let cylinder = CanonicalSurface::Cylinder(
            Cylinder::new(Point3::new(0.0, 0.0, 0.0), 1.0)
                .expect("a unit cylinder is a valid carrier")
                .value,
        );
        let low = face_with_bounds(cylinder.clone(), (0.0, TAU), (0.0, 1.0));
        let high = face_with_bounds(cylinder, (0.0, TAU), (5.0, 6.0));
        let mut budget = Budget::new(100, 100, 100);
        let out = contact(&low, &high, &mut budget)
            .expect("same-carrier disjoint cylinder boxes decide empty at the identity stage");
        assert!(
            out.value.contacts.is_empty(),
            "separated patches of the same cylinder never touch"
        );
        assert_eq!(out.cert.method, Method::Exact);
        assert_eq!(out.cert.budget_left, budget, "the screen spends nothing");
    }

    #[test]
    fn overlap_screen_identity_face_periodic_wrap_decides() {
        // Same canonical cylinder: u is the azimuth on the circle, so the seam
        // wrap decides. A near-seam interval overlaps the seam-crossing
        // interval (which wraps onto `(0, 0.1) ∪ (TAU-0.1, TAU)`), while a far
        // interval does not.
        let cylinder = CanonicalSurface::Cylinder(
            Cylinder::new(Point3::new(0.0, 0.0, 0.0), 1.0)
                .expect("a unit cylinder is a valid carrier")
                .value,
        );
        let seam = face_with_bounds(cylinder.clone(), (TAU - 0.1, TAU + 0.1), (0.0, 1.0));
        let near = face_with_bounds(cylinder.clone(), (0.05, 0.2), (0.0, 1.0));
        let far = face_with_bounds(cylinder.clone(), (3.0, 3.1), (0.0, 1.0));
        let mut budget = Budget::new(100, 100, 100);
        let out = contact(&near, &seam, &mut budget)
            .expect("same-carrier seam-wrapped boxes decide at the identity stage");
        assert_eq!(out.cert.method, Method::Exact);
        assert_eq!(out.value.contacts.len(), 1);
        let record = out.value.contacts.first().expect("one record");
        assert_eq!(record.dimension, ContactDimension::Region2);
        assert!(matches!(record.locus, ContactLocus::Coincident));
        let mut budget = Budget::new(100, 100, 100);
        let out = contact(&far, &seam, &mut budget)
            .expect("same-carrier far boxes decide empty at the identity stage");
        assert!(
            out.value.contacts.is_empty(),
            "the far u interval stays disjoint"
        );
        assert_eq!(out.cert.method, Method::Exact);

        // One sphere case exercising the v azimuth wrap: u is the polar
        // angle, v the azimuth, so v wraps across the seam.
        let sphere = CanonicalSurface::Sphere(Sphere::new(Point3::origin(), 1.0));
        let s_near = face_with_bounds(sphere.clone(), (0.2, 0.8), (0.05, 0.2));
        let s_seam = face_with_bounds(sphere, (0.2, 0.8), (TAU - 0.1, TAU + 0.1));
        let mut budget = Budget::new(100, 100, 100);
        let out = contact(&s_near, &s_seam, &mut budget)
            .expect("same-carrier sphere seam-wrapped boxes decide at the identity stage");
        assert_eq!(out.cert.method, Method::Exact);
        assert_eq!(out.value.contacts.len(), 1);
        let record = out.value.contacts.first().expect("one record");
        assert_eq!(record.dimension, ContactDimension::Region2);
        assert!(matches!(record.locus, ContactLocus::Coincident));
    }

    #[test]
    fn overlap_screen_same_axis_cylinder_shift_decides() {
        // Two struct-unequal coaxial unit cylinders: centers (0,0,0) and
        // (0,0,5), same wall. The coaxial cell answers Coincident; the screen
        // compares the absolute z-extents `[cz + v0, cz + v1]` (one exactly
        // rounded f64 addition per endpoint) and the azimuth boxes.
        let cyl0 = CanonicalSurface::Cylinder(
            Cylinder::new(Point3::new(0.0, 0.0, 0.0), 1.0)
                .expect("a unit cylinder is a valid carrier")
                .value,
        );
        let cyl5 = CanonicalSurface::Cylinder(
            Cylinder::new(Point3::new(0.0, 0.0, 5.0), 1.0)
                .expect("a unit cylinder is a valid carrier")
                .value,
        );
        // Disjoint: v (0,1) on both -> absolute z [0,1] vs [5,6] -> empty.
        let lhs = face_with_bounds(cyl0.clone(), (0.0, TAU), (0.0, 1.0));
        let rhs = face_with_bounds(cyl5.clone(), (0.0, TAU), (0.0, 1.0));
        let mut budget = Budget::new(100, 100, 100);
        let out = contact(&lhs, &rhs, &mut budget)
            .expect("same-wall cylinders with disjoint z extents decide empty");
        assert!(
            out.value.contacts.is_empty(),
            "the wall patches never touch"
        );
        assert_eq!(out.cert.method, Method::Exact);
        // Overlapping: v (4,6) on the first, v (0,1) on the second -> absolute
        // z [4,6] vs [5,6] -> the analytic Region2/CoincidentInterval record.
        let lhs = face_with_bounds(cyl0, (0.0, TAU), (4.0, 6.0));
        let rhs = face_with_bounds(cyl5, (0.0, TAU), (0.0, 1.0));
        let mut budget = Budget::new(100, 100, 100);
        let out = contact(&lhs, &rhs, &mut budget)
            .expect("same-wall cylinders with overlapping z extents decide coincident");
        assert_eq!(out.cert.method, Method::Exact);
        assert_eq!(out.value.contacts.len(), 1);
        let record = out.value.contacts.first().expect("one record");
        assert_eq!(record.dimension, ContactDimension::Region2);
        assert_eq!(record.kind, ContactEventKind::CoincidentInterval);
        assert!(matches!(
            &record.locus,
            ContactLocus::Analytic(AnalyticIntersection::Coincident)
        ));
    }

    #[test]
    fn overlap_screen_parallel_frame_planes_decide() {
        // Two struct-unequal coplanar planes with parallel frames: plane A at
        // the origin with unit axes, plane B at the origin with doubled axes
        // ((2,0,0),(0,2,0)). The Cramer map in A's frame is M = [[2,0],[0,2]],
        // c = (0,0) (see RESULT.json notes). Boxes in B's units that map away
        // from A's box prove empty; boxes that map into A's interior stay
        // Region2/CoincidentInterval.
        let plane_a = CanonicalSurface::Plane(Plane::new(
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        ));
        let plane_b = CanonicalSurface::Plane(Plane::new(
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(2.0, 0.0, 0.0),
            Point3::new(0.0, 2.0, 0.0),
        ));
        // Disjoint: A box (0,1)x(0,1), B box (3,4)x(3,4) -> image u in [6,8],
        // v in [6,8] -> empty.
        let lhs = face_with_bounds(plane_a.clone(), (0.0, 1.0), (0.0, 1.0));
        let rhs = face_with_bounds(plane_b.clone(), (3.0, 4.0), (3.0, 4.0));
        let mut budget = Budget::new(100, 100, 100);
        let out = contact(&lhs, &rhs, &mut budget)
            .expect("parallel-frame coplanar planes with disjoint boxes decide empty");
        assert!(
            out.value.contacts.is_empty(),
            "the image box maps away from the patch"
        );
        assert_eq!(out.cert.method, Method::Exact);
        // Overlapping: B box (0.25, 0.4)x(0.25, 0.4) -> image u in [0.5, 0.8],
        // v in [0.5, 0.8], strictly inside A's (0,1)x(0,1) -> Region2.
        let lhs = face_with_bounds(plane_a, (0.0, 1.0), (0.0, 1.0));
        let rhs = face_with_bounds(plane_b, (0.25, 0.4), (0.25, 0.4));
        let mut budget = Budget::new(100, 100, 100);
        let out = contact(&lhs, &rhs, &mut budget)
            .expect("parallel-frame coplanar planes with overlapping boxes decide coincident");
        assert_eq!(out.cert.method, Method::Exact);
        assert_eq!(out.value.contacts.len(), 1);
        let record = out.value.contacts.first().expect("one record");
        assert_eq!(record.dimension, ContactDimension::Region2);
        assert_eq!(record.kind, ContactEventKind::CoincidentInterval);
        assert!(matches!(
            &record.locus,
            ContactLocus::Analytic(AnalyticIntersection::Coincident)
        ));
    }

    #[test]
    fn overlap_screen_edge_disjoint_ranges_certify_empty() {
        // Same canonical line edge, t ranges touching at the endpoint (0.5):
        // interiors disjoint, so no contact. Same placed circle, disjoint
        // arcs: empty; overlapping arcs: the existing Arc1 coincident-carrier
        // record.
        let line =
            CanonicalCurve::Line(Line(Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 0.0, 0.0)));
        let edge_a = BoundedStratum::Edge {
            curve: line.clone(),
            t_range: (0.0, 0.5),
        };
        let edge_b = BoundedStratum::Edge {
            curve: line,
            t_range: (0.5, 1.0),
        };
        let mut budget = Budget::new(100, 100, 100);
        let out = contact(&edge_a, &edge_b, &mut budget)
            .expect("same-carrier disjoint edge ranges decide empty");
        assert!(
            out.value.contacts.is_empty(),
            "touching at the endpoint is not overlap"
        );
        assert_eq!(out.cert.method, Method::Exact);
        assert_eq!(out.cert.budget_left, budget, "the screen spends nothing");

        let circle = CanonicalCurve::Circle(placed_unit_circle());
        let disjoint_a = BoundedStratum::Edge {
            curve: circle.clone(),
            t_range: (0.1, 0.2),
        };
        let disjoint_b = BoundedStratum::Edge {
            curve: circle.clone(),
            t_range: (0.5, 0.6),
        };
        let mut budget = Budget::new(100, 100, 100);
        let out = contact(&disjoint_a, &disjoint_b, &mut budget)
            .expect("same-carrier disjoint circle arcs decide empty");
        assert!(
            out.value.contacts.is_empty(),
            "disjoint arcs on the same circle never touch"
        );
        assert_eq!(out.cert.method, Method::Exact);

        let overlap_a = BoundedStratum::Edge {
            curve: circle.clone(),
            t_range: (0.1, 0.5),
        };
        let overlap_b = BoundedStratum::Edge {
            curve: circle,
            t_range: (0.4, 0.6),
        };
        let mut budget = Budget::new(100, 100, 100);
        let out = contact(&overlap_a, &overlap_b, &mut budget)
            .expect("same-carrier overlapping circle arcs decide coincident");
        assert_eq!(out.cert.method, Method::Exact);
        assert_eq!(out.value.contacts.len(), 1);
        let record = out.value.contacts.first().expect("one record");
        assert_eq!(record.dimension, ContactDimension::Arc1);
        assert!(matches!(record.locus, ContactLocus::Coincident));
    }

    #[test]
    fn overlap_screen_is_order_insensitive() {
        // The shift and parallel-frame witnesses with the strata swapped
        // produce the same outcome: empty stays empty, Coincident stays
        // Coincident with the same dimension/kind.
        let cyl0 = CanonicalSurface::Cylinder(
            Cylinder::new(Point3::new(0.0, 0.0, 0.0), 1.0)
                .expect("a unit cylinder is a valid carrier")
                .value,
        );
        let cyl5 = CanonicalSurface::Cylinder(
            Cylinder::new(Point3::new(0.0, 0.0, 5.0), 1.0)
                .expect("a unit cylinder is a valid carrier")
                .value,
        );
        let cyl0_over = face_with_bounds(cyl0.clone(), (0.0, TAU), (4.0, 6.0));
        let cyl5_over = face_with_bounds(cyl5.clone(), (0.0, TAU), (0.0, 1.0));
        let mut budget = Budget::new(100, 100, 100);
        let fwd = contact(&cyl0_over, &cyl5_over, &mut budget)
            .expect("the forward overlapping shift decides");
        let mut budget = Budget::new(100, 100, 100);
        let rev = contact(&cyl5_over, &cyl0_over, &mut budget)
            .expect("the reversed overlapping shift decides");
        assert_eq!(
            format!("{fwd:?}"),
            format!("{rev:?}"),
            "the shift screen is order-insensitive"
        );
        let cyl0_disj = face_with_bounds(cyl0, (0.0, TAU), (0.0, 1.0));
        let cyl5_disj = face_with_bounds(cyl5, (0.0, TAU), (0.0, 1.0));
        let mut budget = Budget::new(100, 100, 100);
        let fwd = contact(&cyl0_disj, &cyl5_disj, &mut budget)
            .expect("the forward disjoint shift decides");
        let mut budget = Budget::new(100, 100, 100);
        let rev = contact(&cyl5_disj, &cyl0_disj, &mut budget)
            .expect("the reversed disjoint shift decides");
        assert_eq!(
            format!("{fwd:?}"),
            format!("{rev:?}"),
            "the empty shift screen is order-insensitive"
        );

        let plane_a = CanonicalSurface::Plane(Plane::new(
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        ));
        let plane_b = CanonicalSurface::Plane(Plane::new(
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(2.0, 0.0, 0.0),
            Point3::new(0.0, 2.0, 0.0),
        ));
        let a_over = face_with_bounds(plane_a.clone(), (0.0, 1.0), (0.0, 1.0));
        let b_over = face_with_bounds(plane_b.clone(), (0.25, 0.4), (0.25, 0.4));
        let mut budget = Budget::new(100, 100, 100);
        let fwd = contact(&a_over, &b_over, &mut budget)
            .expect("the forward overlapping plane pair decides");
        let mut budget = Budget::new(100, 100, 100);
        let rev = contact(&b_over, &a_over, &mut budget)
            .expect("the reversed overlapping plane pair decides");
        assert_eq!(
            format!("{fwd:?}"),
            format!("{rev:?}"),
            "the plane screen is order-insensitive"
        );
        let a_disj = face_with_bounds(plane_a, (0.0, 1.0), (0.0, 1.0));
        let b_disj = face_with_bounds(plane_b, (3.0, 4.0), (3.0, 4.0));
        let mut budget = Budget::new(100, 100, 100);
        let fwd = contact(&a_disj, &b_disj, &mut budget)
            .expect("the forward disjoint plane pair decides");
        let mut budget = Budget::new(100, 100, 100);
        let rev = contact(&b_disj, &a_disj, &mut budget)
            .expect("the reversed disjoint plane pair decides");
        assert_eq!(
            format!("{fwd:?}"),
            format!("{rev:?}"),
            "the empty plane screen is order-insensitive"
        );
    }
}
