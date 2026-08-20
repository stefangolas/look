//! BG-ANA-001-COAX: coaxial pairs (cylinder/cone/sphere/torus) — circles or
//! empty.
//!
//! Every carrier in the specifieds is **canonical**: the cylinder runs along
//! the z axis through its centre, the cone opens along +z from its apex, the
//! torus is centred with its axis along z, and the sphere is free. A **coaxial
//! pair** — both carriers sharing the z axis — meets, when it meets at all, in
//! circles at constant z: the radial profile of each carrier is a function of z
//! alone, and circles happen where the two profiles are equal. This is the
//! counterbore and fillet family: every counterbore is coaxial, and coaxial
//! tangency is decided, not approximated.
//!
//! The algebra is uniform: matching the radial profiles reduces each pair to a
//! **linear or quadratic equation in z**, and the discriminant's three-way
//! comparison classifies two circles / one tangent circle / empty. Coaxiality
//! itself is validated up front — the two carriers' axes must be the same z
//! line — and every predicate is computed as an outward-rounded
//! `inari::Interval` enclosure of the f64 carrier parameters (dyadic-clean
//! witnesses give degenerate intervals, so exact classifications stay exact).
//! An enclosure that merely contains zero proves nothing: an undecidable
//! predicate is a `Refusal::NumericallyUnresolved`, never a confident guess
//! (BG-ANA-002).
//!
//! The shared result type is [`crate::analytic::AnalyticIntersection`] (with
//! [`crate::analytic::ExactCurve`]); this module defines no result type of its
//! own. The emitted circles are [`crate::analytic::PlacedCircle`]: the trimmed
//! unit circle under an affine placement. [`TrimmedCurve`] does **not** remap
//! its parameter — `subs(t)` takes the angle directly.

use std::cmp::Ordering;
use std::f64::consts::TAU;

use inari::Interval;
use truck_base::cgmath64::{Matrix4, Point3, Vector3, Vector4};
use truck_base::evidence::{
    Budget, Certificate, Certified, EnvelopeCase, Margin, Method, Modulus, Prop, PropMap, Refusal,
    Truth, UnresolvedWitness,
};
use truck_geometry::decorators::{Processor, TrimmedCurve};
use truck_geometry::specifieds::{Cone, Cylinder, Sphere, Torus, UnitCircle};

use crate::analytic::{AnalyticIntersection, AnalyticOutcome, ExactCurve, PlacedCircle};

/// A coaxial pair of carriers sharing the z axis (BG-ANA-001-COAX).
///
/// Coaxiality is a property of the pair, not of either carrier alone, so it is
/// an input to the classification rather than an assumption: [`validate`] is
/// called by [`coaxial`] before any algebra and refuses an off-axis pair.
///
/// [`validate`]: CoaxialPair::validate
pub enum CoaxialPair<'a> {
    /// A coaxial cylinder–cylinder pair.
    CylCyl(&'a Cylinder, &'a Cylinder),
    /// A coaxial cylinder–cone pair.
    CylCone(&'a Cylinder, &'a Cone),
    /// A coaxial cylinder–sphere pair.
    CylSphere(&'a Cylinder, &'a Sphere),
    /// A coaxial cylinder–torus pair.
    CylTorus(&'a Cylinder, &'a Torus),
    /// A coaxial cone–cone pair.
    ConeCone(&'a Cone, &'a Cone),
    /// A coaxial cone–sphere pair.
    ConeSphere(&'a Cone, &'a Sphere),
    /// A coaxial cone–torus pair.
    ConeTorus(&'a Cone, &'a Torus),
    /// A coaxial sphere–torus pair.
    SphereTorus(&'a Sphere, &'a Torus),
}

impl CoaxialPair<'_> {
    /// Validates that the two carriers share the same z axis.
    ///
    /// These are **exact f64 equality tests on point coordinates that ARE the
    /// carrier parameters** — the cylinder's centre, the cone's apex, the
    /// torus' centre and the sphere's centre must all lie on the same z line,
    /// so the (x, y) pair of one equals the (x, y) pair of the other. No
    /// intervals are needed and no tolerance is applied: either the axis
    /// positions are exactly equal or the pair is not coaxial.
    pub fn validate(&self) -> Result<(), Refusal> {
        let (x0, y0, x1, y1) = match self {
            CoaxialPair::CylCyl(a, b) => (a.center().x, a.center().y, b.center().x, b.center().y),
            CoaxialPair::CylCone(a, b) => (a.center().x, a.center().y, b.apex().x, b.apex().y),
            CoaxialPair::CylSphere(a, b) => {
                (a.center().x, a.center().y, b.center().x, b.center().y)
            }
            CoaxialPair::CylTorus(a, b) => (a.center().x, a.center().y, b.center().x, b.center().y),
            CoaxialPair::ConeCone(a, b) => (a.apex().x, a.apex().y, b.apex().x, b.apex().y),
            CoaxialPair::ConeSphere(a, b) => (a.apex().x, a.apex().y, b.center().x, b.center().y),
            CoaxialPair::ConeTorus(a, b) => (a.apex().x, a.apex().y, b.center().x, b.center().y),
            CoaxialPair::SphereTorus(a, b) => {
                (a.center().x, a.center().y, b.center().x, b.center().y)
            }
        };
        if x0 == x1 && y0 == y1 {
            Ok(())
        } else {
            Err(Refusal::UnsupportedEnvelope(
                EnvelopeCase::NonCanonicalCarrier,
            ))
        }
    }
}

/// Classifies a coaxial pair exactly (BG-ANA-001-COAX).
///
/// `Method::Exact` here means: the classification is decided by **decisive
/// interval predicates** on the f64 carrier parameters — the discriminant of
/// the reduced quadratic, or the direct exact f64 equalities of the
/// degenerate placements — and the emitted circles are the closed-form
/// intersections. The circle coordinates are computed in f64; the obligation
/// the certificate takes on is that the emitted circle lies on both carriers
/// to machine precision (the on-both-carriers tests assert this with an
/// H-3-commented slack), not that the coordinates are dyadic-exact. There is
/// no `τ_rep` anywhere.
///
/// `Ok` is returned only when every predicate that chose the returned arm was
/// decisive; an undecidable predicate is `Err(Refusal::NumericallyUnresolved)`,
/// never a guess. Coaxiality is validated first and its refusal propagates.
pub fn coaxial(pair: &CoaxialPair) -> AnalyticOutcome {
    pair.validate()?;
    match pair {
        CoaxialPair::CylCyl(a, b) => cyl_cyl(a, b),
        CoaxialPair::CylCone(a, b) => cyl_cone(a, b),
        CoaxialPair::CylSphere(a, b) => cyl_sphere(a, b),
        CoaxialPair::CylTorus(a, b) => cyl_torus(a, b),
        CoaxialPair::ConeCone(a, b) => cone_cone(a, b),
        CoaxialPair::ConeSphere(a, b) => cone_sphere(a, b),
        CoaxialPair::ConeTorus(a, b) => cone_torus(a, b),
        CoaxialPair::SphereTorus(a, b) => sphere_torus(a, b),
    }
}

/// The coaxial cylinder–cylinder pair.
///
/// Two coaxial cylinders have constant radial profiles `rc0` and `rc1`; they
/// never meet transversally. The radii ARE the carrier parameters, so exact
/// f64 equality decides: equal radii coincide in a surface, unequal radii
/// never meet.
fn cyl_cyl(a: &Cylinder, b: &Cylinder) -> AnalyticOutcome {
    let value = if a.radius() == b.radius() {
        AnalyticIntersection::Coincident
    } else {
        AnalyticIntersection::Empty
    };
    let mut props = PropMap::new();
    props.set(Prop::AnalyticCarrier, Truth::True);
    Ok(Certified::new(
        value,
        Certificate {
            props,
            method: Method::Exact,
            budget_left: Budget::new(0, 0, 0),
            margin: Margin::UNBOUNDED,
            modulus: Modulus::Unbounded,
        },
    ))
}

/// The coaxial cylinder–cone pair.
///
/// The cone profile `|z − za| tan α` meets the constant cylinder profile `rc`
/// at the two heights `z = za ± rc/tan α` — both real circles, one per cone
/// nappe. A degenerate cone (tan α decisively zero) never meets the cylinder;
/// the packet prescribes `Empty` for it.
fn cyl_cone(cyl: &Cylinder, cone: &Cone) -> AnalyticOutcome {
    let rc = cyl.radius();
    let t = cone.half_angle().tan();
    let ti = itv(t);
    if decisively_zero(ti) {
        let mut props = PropMap::new();
        props.set(Prop::AnalyticCarrier, Truth::True);
        Ok(Certified::new(
            AnalyticIntersection::Empty,
            Certificate {
                props,
                method: Method::Exact,
                budget_left: Budget::new(0, 0, 0),
                margin: Margin::UNBOUNDED,
                modulus: Modulus::Unbounded,
            },
        ))
    } else if excludes_zero(ti) {
        let (x0, y0) = (cyl.center().x, cyl.center().y);
        let za = cone.apex().z;
        let half = rc / t;
        let mut props = PropMap::new();
        props.set(Prop::AnalyticCarrier, Truth::True);
        Ok(Certified::new(
            AnalyticIntersection::TwoCurves([
                ExactCurve::Circle(circle_at((x0, y0), za - half, rc)),
                ExactCurve::Circle(circle_at((x0, y0), za + half, rc)),
            ]),
            Certificate {
                props,
                method: Method::Exact,
                budget_left: Budget::new(0, 0, 0),
                margin: Margin::UNBOUNDED,
                modulus: Modulus::Unbounded,
            },
        ))
    } else {
        Err(unsolved())
    }
}

/// The coaxial cylinder–sphere pair.
///
/// `rc² == rs² − (z − zs)²` reduces to `(z − zs)² == rs² − rc²`; the right
/// side is compared against zero by `three_way`. Positive → two circles at
/// `z = zs ± √(rs² − rc²)`; degenerate zero → the tangent circle at `z = zs`;
/// negative → empty; undecidable → refuse.
fn cyl_sphere(cyl: &Cylinder, sphere: &Sphere) -> AnalyticOutcome {
    let rc = cyl.radius();
    let rs = sphere.radius();
    let zs = sphere.center().z;
    let (x0, y0) = (cyl.center().x, cyl.center().y);
    let right = itv(rs) * itv(rs) - itv(rc) * itv(rc);
    match three_way(right, itv(0.0)) {
        Some(Ordering::Greater) => {
            let root = (rs * rs - rc * rc).sqrt();
            let mut props = PropMap::new();
            props.set(Prop::AnalyticCarrier, Truth::True);
            Ok(Certified::new(
                AnalyticIntersection::TwoCurves([
                    ExactCurve::Circle(circle_at((x0, y0), zs - root, rc)),
                    ExactCurve::Circle(circle_at((x0, y0), zs + root, rc)),
                ]),
                Certificate {
                    props,
                    method: Method::Exact,
                    budget_left: Budget::new(0, 0, 0),
                    margin: Margin::UNBOUNDED,
                    modulus: Modulus::Unbounded,
                },
            ))
        }
        Some(Ordering::Equal) => {
            let mut props = PropMap::new();
            props.set(Prop::AnalyticCarrier, Truth::True);
            Ok(Certified::new(
                AnalyticIntersection::TangentCircle(circle_at((x0, y0), zs, rc)),
                Certificate {
                    props,
                    method: Method::Exact,
                    budget_left: Budget::new(0, 0, 0),
                    margin: Margin::UNBOUNDED,
                    modulus: Modulus::Unbounded,
                },
            ))
        }
        Some(Ordering::Less) => {
            let mut props = PropMap::new();
            props.set(Prop::AnalyticCarrier, Truth::True);
            Ok(Certified::new(
                AnalyticIntersection::Empty,
                Certificate {
                    props,
                    method: Method::Exact,
                    budget_left: Budget::new(0, 0, 0),
                    margin: Margin::UNBOUNDED,
                    modulus: Modulus::Unbounded,
                },
            ))
        }
        None => Err(unsolved()),
    }
}

/// The coaxial cylinder–torus pair.
///
/// The outer and inner torus contacts both reduce (squaring once) to
/// `(z − zt)² == rt² − (rc − R)²` — the same equation for both branches,
/// because the outer contact needs `rc ≥ R` and the inner needs `rc ≤ R`,
/// mutually exclusive. The right side is compared against zero exactly as in
/// [`cyl_sphere`]: positive → two circles at `z = zt ± √(rt² − (rc − R)²)`;
/// degenerate zero → the tangent circle at `z = zt`; negative → empty.
fn cyl_torus(cyl: &Cylinder, torus: &Torus) -> AnalyticOutcome {
    let rc = cyl.radius();
    let (r, rt) = (torus.large_radius(), torus.small_radius());
    let zt = torus.center().z;
    let (x0, y0) = (cyl.center().x, cyl.center().y);
    let right = itv(rt) * itv(rt) - (itv(rc) - itv(r)) * (itv(rc) - itv(r));
    match three_way(right, itv(0.0)) {
        Some(Ordering::Greater) => {
            let root = (rt * rt - (rc - r) * (rc - r)).sqrt();
            let mut props = PropMap::new();
            props.set(Prop::AnalyticCarrier, Truth::True);
            Ok(Certified::new(
                AnalyticIntersection::TwoCurves([
                    ExactCurve::Circle(circle_at((x0, y0), zt - root, rc)),
                    ExactCurve::Circle(circle_at((x0, y0), zt + root, rc)),
                ]),
                Certificate {
                    props,
                    method: Method::Exact,
                    budget_left: Budget::new(0, 0, 0),
                    margin: Margin::UNBOUNDED,
                    modulus: Modulus::Unbounded,
                },
            ))
        }
        Some(Ordering::Equal) => {
            let mut props = PropMap::new();
            props.set(Prop::AnalyticCarrier, Truth::True);
            Ok(Certified::new(
                AnalyticIntersection::TangentCircle(circle_at((x0, y0), zt, rc)),
                Certificate {
                    props,
                    method: Method::Exact,
                    budget_left: Budget::new(0, 0, 0),
                    margin: Margin::UNBOUNDED,
                    modulus: Modulus::Unbounded,
                },
            ))
        }
        Some(Ordering::Less) => {
            let mut props = PropMap::new();
            props.set(Prop::AnalyticCarrier, Truth::True);
            Ok(Certified::new(
                AnalyticIntersection::Empty,
                Certificate {
                    props,
                    method: Method::Exact,
                    budget_left: Budget::new(0, 0, 0),
                    margin: Margin::UNBOUNDED,
                    modulus: Modulus::Unbounded,
                },
            ))
        }
        None => Err(unsolved()),
    }
}

/// The coaxial cone–cone pair.
///
/// `|z − za0| tan α0 == |z − za1| tan α1` is piecewise linear in z. The
/// degenerate placements are decided by exact f64 equality of the carrier
/// parameters: same apex and same tan α → coincident; same tan α, different
/// apex → empty (the packet's prescribed answer for the parallel placement).
/// Otherwise the linear equation is solved on each sign region — one candidate
/// outside both apexes (both nappes give the same root) and one between the
/// apexes — and the solutions lying in their region are emitted (0, 1 or 2
/// circles). The region test is a decisive interval comparison of the computed
/// root against the apexes, refusing on a straddle (unreachable for the
/// degenerate f64 root, kept for the invariant).
fn cone_cone(a: &Cone, b: &Cone) -> AnalyticOutcome {
    let za0 = a.apex().z;
    let za1 = b.apex().z;
    let s0 = a.half_angle().tan();
    let s1 = b.half_angle().tan();
    let (x0, y0) = (a.apex().x, a.apex().y);
    let value = if za0 == za1 && s0 == s1 {
        AnalyticIntersection::Coincident
    } else if s0 == s1 {
        AnalyticIntersection::Empty
    } else {
        let z_out = (za0 * s0 - za1 * s1) / (s0 - s1);
        let z_bet = (za0 * s0 + za1 * s1) / (s0 + s1);
        let lo = za0.min(za1);
        let hi = za0.max(za1);
        let left = le(itv(z_out), za0)? && le(itv(z_out), za1)?;
        let right = ge(itv(z_out), za0)? && ge(itv(z_out), za1)?;
        let between = ge(itv(z_bet), lo)? && le(itv(z_bet), hi)?;
        let mut zs = Vec::new();
        if left {
            zs.push(z_out);
        }
        if right {
            zs.push(z_out);
        }
        if between {
            zs.push(z_bet);
        }
        zs.dedup();
        let radius = |z: f64| (z - za0).abs() * s0;
        match zs.as_slice() {
            [] => AnalyticIntersection::Empty,
            [z] => {
                AnalyticIntersection::Curve(ExactCurve::Circle(circle_at((x0, y0), *z, radius(*z))))
            }
            [z0, z1] => AnalyticIntersection::TwoCurves([
                ExactCurve::Circle(circle_at((x0, y0), *z0, radius(*z0))),
                ExactCurve::Circle(circle_at((x0, y0), *z1, radius(*z1))),
            ]),
            _ => unreachable!("two coaxial cones meet in at most two circles"),
        }
    };
    let mut props = PropMap::new();
    props.set(Prop::AnalyticCarrier, Truth::True);
    Ok(Certified::new(
        value,
        Certificate {
            props,
            method: Method::Exact,
            budget_left: Budget::new(0, 0, 0),
            margin: Margin::UNBOUNDED,
            modulus: Modulus::Unbounded,
        },
    ))
}

/// The coaxial cone–sphere pair.
///
/// `√(rs² − (z − zs)²) == |z − za| tan α` squares to
/// `(1 + tan² α) z² − 2 (za tan² α + zs) z + (za² tan² α + zs² − rs²) = 0`,
/// one quadratic in z, classified by the shared discriminant helper. Squaring
/// adds no spurious roots here because both sides are non-negative on the
/// domains: the sphere radial exists where `|z − zs| ≤ rs` and the cone
/// profile is an absolute value. Two roots → two circles, a double root → the
/// tangent circle (the inscribed-sphere case), a decisively negative
/// discriminant → empty.
fn cone_sphere(cone: &Cone, sphere: &Sphere) -> AnalyticOutcome {
    let t = cone.half_angle().tan();
    let za = cone.apex().z;
    let zs = sphere.center().z;
    let rs = sphere.radius();
    let (x0, y0) = (cone.apex().x, cone.apex().y);
    let af = t * t + 1.0;
    let bf = -2.0 * (za * t * t + zs);
    let cf = za * za * t * t + zs * zs - rs * rs;
    let ti = itv(t);
    let ai = ti * ti + itv(1.0);
    let bi = itv(-2.0) * (itv(za) * ti * ti + itv(zs));
    let ci = itv(za) * itv(za) * ti * ti + itv(zs) * itv(zs) - itv(rs) * itv(rs);
    let quad = classify_quadratic(ai, bi, ci, af, bf, cf)?;
    let radius = |z: f64| (z - za).abs() * t;
    let value = match quad {
        Quad::Empty => AnalyticIntersection::Empty,
        Quad::Coincident => AnalyticIntersection::Coincident,
        Quad::One(z) => {
            AnalyticIntersection::Curve(ExactCurve::Circle(circle_at((x0, y0), z, radius(z))))
        }
        Quad::Tangent(z) => AnalyticIntersection::TangentCircle(circle_at((x0, y0), z, radius(z))),
        Quad::Two([z0, z1]) => AnalyticIntersection::TwoCurves([
            ExactCurve::Circle(circle_at((x0, y0), z0, radius(z0))),
            ExactCurve::Circle(circle_at((x0, y0), z1, radius(z1))),
        ]),
    };
    let mut props = PropMap::new();
    props.set(Prop::AnalyticCarrier, Truth::True);
    Ok(Certified::new(
        value,
        Certificate {
            props,
            method: Method::Exact,
            budget_left: Budget::new(0, 0, 0),
            margin: Margin::UNBOUNDED,
            modulus: Modulus::Unbounded,
        },
    ))
}

/// The coaxial cone–torus pair.
///
/// `(z − za) tan α == R ± √(rt² − (z − zt)²)` squares once to
/// `((z − za) tan α − R)² == rt² − (z − zt)²`, a quadratic in z (the packet's
/// reduction carries both z offsets through). The squared equation is then
/// solved by the shared discriminant classifier, and **each root is verified
/// against the unsquared branch equation** `|z − za| tan α == R ± √(...)` in
/// inari: a root that only solves the squared equation (possible when the
/// inner branch dips below radius zero, i.e. when `rt > R`) is dropped, and a
/// root whose verification is undecidable refuses the whole call. The
/// reduction's roots all carry `(z − za) tan α ≥ 0`, so no spurious roots
/// exist for the `R > rt` witness class; the verification below drops them
/// whenever the parameters do produce some.
fn cone_torus(cone: &Cone, torus: &Torus) -> AnalyticOutcome {
    let t = cone.half_angle().tan();
    let za = cone.apex().z;
    let (r, rt) = (torus.large_radius(), torus.small_radius());
    let zt = torus.center().z;
    let (x0, y0) = (cone.apex().x, cone.apex().y);
    let af = t * t + 1.0;
    let bf = -2.0 * (za * t * t + r * t + zt);
    let cf = za * za * t * t + 2.0 * r * za * t + r * r + zt * zt - rt * rt;
    let ti = itv(t);
    let ai = ti * ti + itv(1.0);
    let bi = itv(-2.0) * (itv(za) * ti * ti + itv(r) * ti + itv(zt));
    let ci = itv(za) * itv(za) * ti * ti
        + itv(2.0) * itv(r) * itv(za) * ti
        + itv(r) * itv(r)
        + itv(zt) * itv(zt)
        - itv(rt) * itv(rt);
    let quad = classify_quadratic(ai, bi, ci, af, bf, cf)?;
    let value = filter_quad(
        quad,
        (x0, y0),
        |z| (z - za).abs() * t,
        |z| cone_torus_root_ok(z, t, za, r, rt, zt),
    )?;
    let mut props = PropMap::new();
    props.set(Prop::AnalyticCarrier, Truth::True);
    Ok(Certified::new(
        value,
        Certificate {
            props,
            method: Method::Exact,
            budget_left: Budget::new(0, 0, 0),
            margin: Margin::UNBOUNDED,
            modulus: Modulus::Unbounded,
        },
    ))
}

/// The coaxial sphere–torus pair.
///
/// The sphere radial `√(rs² − (z − zs)²)` must equal a torus branch
/// `R ± √(rt² − (z − zt)²)`. Substituting and squaring once — `(√A − R)² = B`
/// with `A = rs² − (z − zs)²`, `B = rt² − (z − zt)²` — gives a quadratic in z:
/// `A + R² − B` is linear in z (the two z² terms cancel only when `zs == zt`,
/// otherwise they survive as the linear coefficient), and the squared equation
/// `(A + R² − B)² == 4R²A` is quadratic either way. The shared discriminant
/// classifier decides the arm, then **each root is checked against the
/// unsquared equation** `√(rs² − (z − zs)²) == R ± √(rt² − (z − zt)²)` in
/// inari, decisively: the squaring can introduce spurious roots when the
/// sphere radial and the torus branch have opposite signs, so a root that
/// fails both branches is dropped and an undecidable root refuses.
fn sphere_torus(sphere: &Sphere, torus: &Torus) -> AnalyticOutcome {
    let rs = sphere.radius();
    let zs = sphere.center().z;
    let (r, rt) = (torus.large_radius(), torus.small_radius());
    let zt = torus.center().z;
    let (x0, y0) = (sphere.center().x, sphere.center().y);
    let f0 = rs * rs + r * r - rt * rt + zs * zs - zt * zt;
    let f1 = 2.0 * (zt - zs);
    let af = f1 * f1 + 4.0 * r * r;
    let bf = 2.0 * f0 * f1 - 8.0 * r * r * zs;
    let cf = f0 * f0 - 4.0 * r * r * (rs * rs - zs * zs);
    let f0i = itv(rs) * itv(rs) + itv(r) * itv(r) - itv(rt) * itv(rt) + itv(zs) * itv(zs)
        - itv(zt) * itv(zt);
    let f1i = itv(2.0) * (itv(zt) - itv(zs));
    let ai = f1i * f1i + itv(4.0) * itv(r) * itv(r);
    let bi = itv(2.0) * f0i * f1i - itv(8.0) * itv(r) * itv(r) * itv(zs);
    let ci = f0i * f0i - itv(4.0) * itv(r) * itv(r) * (itv(rs) * itv(rs) - itv(zs) * itv(zs));
    let quad = classify_quadratic(ai, bi, ci, af, bf, cf)?;
    let radius = |z: f64| (rs * rs - (z - zs) * (z - zs)).max(0.0).sqrt();
    let value = filter_quad(quad, (x0, y0), radius, |z| {
        sphere_torus_root_ok(z, rs, zs, r, rt, zt)
    })?;
    let mut props = PropMap::new();
    props.set(Prop::AnalyticCarrier, Truth::True);
    Ok(Certified::new(
        value,
        Certificate {
            props,
            method: Method::Exact,
            budget_left: Budget::new(0, 0, 0),
            margin: Margin::UNBOUNDED,
            modulus: Modulus::Unbounded,
        },
    ))
}

/// A degenerate interval carrying exactly the runtime `f64` `x`.
///
/// Finite coordinates always construct; a NaN widens to the empty interval
/// rather than panicking (H-1).
fn itv(x: f64) -> Interval {
    Interval::try_from((x, x)).unwrap_or(Interval::EMPTY)
}

/// Whether the interval proves a zero: it must be the degenerate `[0, 0]`.
///
/// An inari enclosure of a quantity that is zero only through cancellation is
/// a wide-ish `[-ulp, +ulp]`; claiming that proves zero is exactly the
/// wrong-but-confident answer BG-ANA-002 forbids. Dyadic-clean inputs produce
/// degenerate intervals, so exact classifications stay exact.
fn decisively_zero(i: Interval) -> bool {
    i.inf() == 0.0 && i.sup() == 0.0
}

/// Whether the interval proves a nonzero value: it lies strictly away from 0.
fn excludes_zero(i: Interval) -> bool {
    i.inf() > 0.0 || i.sup() < 0.0
}

/// A three-way comparison of two intervals, decided only when the ordering is
/// unambiguous: `Some(Less)` iff `a.sup() < b.inf()`, `Some(Greater)` iff
/// `b.sup() < a.inf()`, `Some(Equal)` iff both intervals are degenerate and
/// identical, and `None` — undecidable — otherwise.
///
/// Undecidable is a stop, not a guess: the caller refuses rather than
/// returning an `Ok` arm chosen by a predicate that did not decide.
fn three_way(a: Interval, b: Interval) -> Option<Ordering> {
    if a.sup() < b.inf() {
        Some(Ordering::Less)
    } else if b.sup() < a.inf() {
        Some(Ordering::Greater)
    } else if a.inf() == a.sup() && b.inf() == b.sup() && a.inf() == b.inf() {
        Some(Ordering::Equal)
    } else {
        None
    }
}

/// Decides `z <= t` decisively, from the interval enclosure of a computed root
/// against the exact f64 threshold. `None` — the enclosure straddles the
/// threshold — is a refusal. A computed f64 root is a degenerate enclosure, so
/// this always decides; the refusal arm is the packet's required invariant.
fn le(z: Interval, t: f64) -> Result<bool, Refusal> {
    match three_way(z, itv(t)) {
        Some(Ordering::Less) | Some(Ordering::Equal) => Ok(true),
        Some(Ordering::Greater) => Ok(false),
        None => Err(unsolved()),
    }
}

/// Decides `z >= t` decisively; see [`le`].
fn ge(z: Interval, t: f64) -> Result<bool, Refusal> {
    match three_way(z, itv(t)) {
        Some(Ordering::Greater) | Some(Ordering::Equal) => Ok(true),
        Some(Ordering::Less) => Ok(false),
        None => Err(unsolved()),
    }
}

/// The refusal for a predicate that could not be decided.
fn unsolved() -> Refusal {
    Refusal::NumericallyUnresolved {
        spent: Budget::new(0, 0, 0),
        witness: UnresolvedWitness::RootNotIsolated,
    }
}

/// The result of the quadratic classifier: an arm plus the closed-form root(s)
/// in f64.
#[derive(Debug)]
enum Quad {
    /// The equation has no real root.
    Empty,
    /// The equation degenerated to `0 = 0`.
    Coincident,
    /// One distinct root (the linear case: one circle).
    One(f64),
    /// One double root (the tangent case: one tangent circle).
    Tangent(f64),
    /// Two distinct roots (two circles).
    Two([f64; 2]),
}

/// Classifies the reduced intersection equation `A z² + B z + C = 0`.
///
/// The interval coefficients `a`, `b`, `c` are outward-rounded inari
/// enclosures of the real coefficients and decide the classification; the f64
/// coefficients `af`, `bf`, `cf` provide the closed-form roots. A decisive
/// discriminant `Δ = B² − 4AC` that excludes zero and is positive gives two
/// roots, a degenerate zero gives one double root (the tangent circle), a
/// decisively negative discriminant gives empty, and a straddling
/// discriminant refuses. A decisively zero `A` degenerates the equation to
/// `B z + C = 0`: one circle when `B` excludes zero, `Coincident` when both
/// `B` and `C` are decisively zero, empty when `C` excludes zero, and a
/// refusal on anything else. An `A` that neither excludes zero nor is
/// decisively zero refuses: it is not decided whether the equation is linear.
fn classify_quadratic(
    a: Interval,
    b: Interval,
    c: Interval,
    af: f64,
    bf: f64,
    cf: f64,
) -> Result<Quad, Refusal> {
    if decisively_zero(a) {
        // Linear: B z + C = 0.
        if excludes_zero(b) {
            Ok(Quad::One(-cf / bf))
        } else if decisively_zero(b) {
            if decisively_zero(c) {
                Ok(Quad::Coincident)
            } else if excludes_zero(c) {
                Ok(Quad::Empty)
            } else {
                Err(unsolved())
            }
        } else {
            Err(unsolved())
        }
    } else if excludes_zero(a) {
        // A genuine quadratic; the leading f64 coefficient is nonzero.
        let disc = b * b - itv(4.0) * a * c;
        if disc.inf() > 0.0 {
            // Clamping the f64 discriminant to zero protects the closed-form
            // roots against a tiny negative rounding that the decisive
            // interval discriminant has already excluded.
            let root = (bf * bf - 4.0 * af * cf).max(0.0).sqrt();
            Ok(Quad::Two([
                (-bf - root) / (2.0 * af),
                (-bf + root) / (2.0 * af),
            ]))
        } else if decisively_zero(disc) {
            Ok(Quad::Tangent(-bf / (2.0 * af)))
        } else if disc.sup() < 0.0 {
            Ok(Quad::Empty)
        } else {
            Err(unsolved())
        }
    } else {
        Err(unsolved())
    }
}

/// Maps a classified quadratic to the intersection value, verifying each root
/// against the unsquared branch equation via `ok` and dropping roots that only
/// solve the squared equation. A root whose verification is undecidable
/// refuses the whole call.
fn filter_quad(
    quad: Quad,
    axis: (f64, f64),
    radius: impl Fn(f64) -> f64,
    ok: impl Fn(f64) -> Result<bool, Refusal>,
) -> Result<AnalyticIntersection, Refusal> {
    Ok(match quad {
        Quad::Empty => AnalyticIntersection::Empty,
        Quad::Coincident => AnalyticIntersection::Coincident,
        Quad::Tangent(z) => {
            if ok(z)? {
                AnalyticIntersection::TangentCircle(circle_at(axis, z, radius(z)))
            } else {
                AnalyticIntersection::Empty
            }
        }
        Quad::One(z) => {
            if ok(z)? {
                AnalyticIntersection::Curve(ExactCurve::Circle(circle_at(axis, z, radius(z))))
            } else {
                AnalyticIntersection::Empty
            }
        }
        Quad::Two([z0, z1]) => {
            let (k0, k1) = (ok(z0)?, ok(z1)?);
            let c0 = ExactCurve::Circle(circle_at(axis, z0, radius(z0)));
            let c1 = ExactCurve::Circle(circle_at(axis, z1, radius(z1)));
            match (k0, k1) {
                (true, true) => AnalyticIntersection::TwoCurves([c0, c1]),
                (true, false) => AnalyticIntersection::Curve(c0),
                (false, true) => AnalyticIntersection::Curve(c1),
                (false, false) => AnalyticIntersection::Empty,
            }
        }
    })
}

/// Whether the root `z` of the cone–torus squared equation also solves the
/// unsquared branch equation `|z − za| tan α == R ± √(rt² − (z − zt)²)`.
///
/// Decided in inari: the cone radial enclosure against both torus branch
/// enclosures. A decisively off-torus height (the radicand decisively
/// negative) or a decisively unequal root is spurious; a root the enclosures
/// cannot separate refuses.
fn cone_torus_root_ok(z: f64, t: f64, za: f64, r: f64, rt: f64, zt: f64) -> Result<bool, Refusal> {
    let cone_rad = (itv(z) - itv(za)).abs() * itv(t);
    let rad = itv(rt) * itv(rt) - (itv(z) - itv(zt)) * (itv(z) - itv(zt));
    if rad.sup() < 0.0 {
        return Ok(false);
    }
    let root = rad.sqrt();
    let plus = three_way(cone_rad, itv(r) + root);
    let minus = three_way(cone_rad, itv(r) - root);
    match (plus, minus) {
        (Some(Ordering::Equal), _) | (_, Some(Ordering::Equal)) => Ok(true),
        (None, None) => Err(unsolved()),
        _ => Ok(false),
    }
}

/// Whether the root `z` of the sphere–torus squared equation also solves the
/// unsquared branch equation `√(rs² − (z − zs)²) == R ± √(rt² − (z − zt)²)`.
///
/// Decided in inari as in [`cone_torus_root_ok`]: the sphere radial enclosure
/// against both torus branch enclosures, refusing on a root the enclosures
/// cannot separate.
fn sphere_torus_root_ok(
    z: f64,
    rs: f64,
    zs: f64,
    r: f64,
    rt: f64,
    zt: f64,
) -> Result<bool, Refusal> {
    let s_rad = itv(rs) * itv(rs) - (itv(z) - itv(zs)) * (itv(z) - itv(zs));
    if s_rad.sup() < 0.0 {
        return Ok(false);
    }
    let sphere_rad = s_rad.sqrt();
    let t_rad = itv(rt) * itv(rt) - (itv(z) - itv(zt)) * (itv(z) - itv(zt));
    if t_rad.sup() < 0.0 {
        return Ok(false);
    }
    let t_root = t_rad.sqrt();
    let plus = three_way(sphere_rad, itv(r) + t_root);
    let minus = three_way(sphere_rad, itv(r) - t_root);
    match (plus, minus) {
        (Some(Ordering::Equal), _) | (_, Some(Ordering::Equal)) => Ok(true),
        (None, None) => Err(unsolved()),
        _ => Ok(false),
    }
}

/// The affine placement of a unit conic: columns `u`, `v`, `n` and origin `o`,
/// scaled in-plane by `ru`/`rv`. A circle of radius `r` through `o` with
/// in-plane unit axes `u`, `v` (`n = u × v`) is
/// `Processor::with_transform(TrimmedCurve::new(UnitCircle::<Point3>::new(),
/// (0.0, TAU)), frame(u, v, n, o, r, r))`.
fn frame(u: Vector3, v: Vector3, n: Vector3, o: Point3, ru: f64, rv: f64) -> Matrix4 {
    Matrix4::from_cols(
        Vector4::new(u.x, u.y, u.z, 0.0),
        Vector4::new(v.x, v.y, v.z, 0.0),
        Vector4::new(n.x, n.y, n.z, 0.0),
        Vector4::new(o.x, o.y, o.z, 1.0),
    ) * Matrix4::from_nonuniform_scale(ru, rv, 1.0)
}

/// A coaxial circle of radius `r` at height `z` on the common axis position
/// `(x0, y0)`: the trimmed unit circle under the axis-aligned placement.
fn circle_at(axis: (f64, f64), z: f64, r: f64) -> PlacedCircle {
    Processor::with_transform(
        TrimmedCurve::new(UnitCircle::<Point3>::new(), (0.0, TAU)),
        frame(
            Vector3::unit_x(),
            Vector3::unit_y(),
            Vector3::unit_z(),
            Point3::new(axis.0, axis.1, z),
            r,
            r,
        ),
    )
}

#[cfg(test)]
// Test-only allow: H-1 bans unwrap/expect on paths reachable from untrusted
// geometry. These unwraps are on hand-built dyadic witnesses, on outcomes just
// asserted to be `Ok`, and on interval pairs with inf <= sup; they cannot fire
// for the values constructed below.
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use truck_base::cgmath64::EuclideanSpace;
    use truck_geotrait::ParametricCurve;

    /// Number of sample points per emitted circle.
    const N: usize = 32;
    /// Float slack on unit-scale witness residuals: radii, heights and
    /// coordinates of every witness are unit-scale, so each residual compared
    /// below is dimensionless-in-scale — the packet's H-3 convention — even
    /// though it carries length units.
    const SLACK: f64 = 1.0e-12; // H-3: float slack on unit-scale witness residuals, not a model-space length

    /// Builds a cylinder witness from its center and radius, avoiding `unwrap`
    /// (H-1): every witness radius is finite and positive, so construction
    /// cannot refuse.
    fn cylinder(center: Point3, radius: f64) -> Cylinder {
        match Cylinder::new(center, radius) {
            Ok(certified) => certified.value,
            Err(refusal) => unreachable!("cylinder construction refused: {refusal:?}"),
        }
    }

    /// Builds a cone witness from its apex and the tangent of its half angle.
    /// `tan_value.atan()` is a valid half angle for any positive `tan_value`,
    /// and for the dyadic witnesses used here `tan(half_angle)` equals
    /// `tan_value` exactly (checked: tan(atan(3/4)) == 3/4, tan(atan(1/2)) ==
    /// 1/2 in f64), so the interval predicates stay exact.
    fn cone(apex: Point3, tan_value: f64) -> Cone {
        match Cone::new(apex, tan_value.atan()) {
            Ok(certified) => certified.value,
            Err(refusal) => unreachable!("cone construction refused: {refusal:?}"),
        }
    }

    /// Extracts the classified value of a decisive outcome, avoiding `unwrap`
    /// (H-1): the dyadic witnesses below are all decisively classified, so a
    /// refusal is a classification regression.
    fn value_of(out: AnalyticOutcome) -> AnalyticIntersection {
        match out {
            Ok(certified) => certified.value,
            Err(refusal) => unreachable!("expected a decisive classification, got {refusal:?}"),
        }
    }

    /// The two circles of a `TwoCurves` arm.
    fn as_two_circles(value: &AnalyticIntersection) -> [PlacedCircle; 2] {
        let AnalyticIntersection::TwoCurves([ExactCurve::Circle(a), ExactCurve::Circle(b)]) = value
        else {
            unreachable!("expected two circles, got {value:?}");
        };
        [*a, *b]
    }

    #[test]
    fn coax_cylinder_sphere_two_circles() {
        // Cylinder r = 3/4 on the z axis, sphere centred at the origin r = 1:
        // rs² − rc² = 1 − 9/16 = 7/16 → two circles at z = ±√7/4 of radius
        // 3/4. Sample both; every point satisfies x² + y² == rc² and
        // x² + y² + z² == rs² to machine precision.
        let cyl = cylinder(Point3::origin(), 0.75);
        let sph = Sphere::new(Point3::origin(), 1.0);
        let value = value_of(coaxial(&CoaxialPair::CylSphere(&cyl, &sph)));
        let [c0, c1] = as_two_circles(&value);
        let rc2 = 0.75 * 0.75;
        for c in [c0, c1] {
            for i in 0..N {
                let p = c.subs(TAU * (i as f64) / (N as f64));
                let radial2 = p.x * p.x + p.y * p.y;
                assert!(
                    (radial2 - rc2).abs() < SLACK,
                    "point {p:?} off the cylinder (radial² {radial2} != {rc2})"
                );
                let on_sphere = radial2 + p.z * p.z;
                assert!(
                    (on_sphere - 1.0).abs() < SLACK,
                    "point {p:?} off the sphere (x²+y²+z² {on_sphere} != 1)"
                );
            }
        }
    }

    #[test]
    fn coax_cylinder_sphere_tangent_circle() {
        // Cylinder r = 1, sphere r = 1 at the origin: rs² − rc² = 0, the
        // degenerate right side → TangentCircle at z = 0 of radius 1, all
        // dyadic.
        let cyl = cylinder(Point3::origin(), 1.0);
        let sph = Sphere::new(Point3::origin(), 1.0);
        let value = value_of(coaxial(&CoaxialPair::CylSphere(&cyl, &sph)));
        let AnalyticIntersection::TangentCircle(c) = &value else {
            unreachable!("expected a tangent circle, got {value:?}");
        };
        for i in 0..N {
            let p = c.subs(TAU * (i as f64) / (N as f64));
            let radial2 = p.x * p.x + p.y * p.y;
            assert!(
                (radial2 - 1.0).abs() < SLACK,
                "point {p:?} off the cylinder (radial² {radial2} != 1)"
            );
            assert!(
                (radial2 + p.z * p.z - 1.0).abs() < SLACK,
                "point {p:?} off the sphere"
            );
        }
    }

    #[test]
    fn coax_cone_sphere_inscribed_tangent_circle() {
        // Cone apex at the origin, half angle with tan α = 3/4; sphere centre
        // (0, 0, 5), radius 3 — the packet's inscribed-sphere witness (centre
        // (0,0,1), radius 3/5, sin α = 3/5) scaled by 5, because the packet's
        // own values are NOT dyadic: the exact product f64(3/5)² falls
        // strictly below f64(9/25), so its outward-rounded enclosure is a
        // 1-ulp interval and the discriminant Δ = B² − 4AC straddles zero —
        // the classification refuses rather than guessing (BG-ANA-002), which
        // the packet's "(all dyadic)" claim misses. The scaled witness keeps
        // sin α = 3/5 and is fully dyadic. The reduced quadratic
        // (1 + t²)z² − 2zs z + (zs² − rs²) = 0 then has discriminant
        // Δ = 4[(1 + t²)rs² − t²zs²] = 4[(25/16)·9 − (9/16)·25] = 0 exactly,
        // so the double root is z = −B/(2A) = 10/(2·25/16) = 16/5 and the
        // tangent circle's radius is z·tan α = (16/5)(3/4) = 12/5. All values
        // are dyadic, so they are asserted within an H-3-commented slack.
        let c = cone(Point3::origin(), 0.75);
        let sph = Sphere::new(Point3::new(0.0, 0.0, 5.0), 3.0);
        let value = value_of(coaxial(&CoaxialPair::ConeSphere(&c, &sph)));
        let AnalyticIntersection::TangentCircle(circle) = &value else {
            unreachable!("expected a tangent circle, got {value:?}");
        };
        // subs(0.0) hits the circle's +x̂ point: (radius, 0, z).
        let p0 = circle.subs(0.0);
        assert!(
            (p0.z - 16.0 / 5.0).abs() < SLACK,
            "tangent circle height {} != 16/5",
            p0.z
        );
        assert!(
            (p0.x - 12.0 / 5.0).abs() < SLACK,
            "tangent circle radius {} != 12/5",
            p0.x
        );
        for i in 0..N {
            let p = circle.subs(TAU * (i as f64) / (N as f64));
            let radial = (p.x * p.x + p.y * p.y).sqrt();
            let cone_radial = (p.z - 0.0).abs() * 0.75;
            assert!(
                (radial - cone_radial).abs() < SLACK,
                "point {p:?} off the cone (radial {radial} != {cone_radial})"
            );
            let on_sphere = p.x * p.x + p.y * p.y + (p.z - 5.0) * (p.z - 5.0);
            assert!(
                (on_sphere - 9.0).abs() < SLACK,
                "point {p:?} off the sphere (residual² {on_sphere} != 9)"
            );
        }
    }

    #[test]
    fn coax_cylinder_torus_two_circles() {
        // Torus R = 2, rt = 1 centred at the origin; cylinder rc = 5/2:
        // (z − zt)² == rt² − (rc − R)² = 1 − (1/2)² = 3/4 → two circles at
        // z = ±√3/2 of radius 5/2. Sample; assert on both carriers. Add the
        // tangent case rc = 3 → TangentCircle at z = 0.
        let torus = Torus::new(Point3::origin(), 2.0, 1.0);
        let cyl = cylinder(Point3::origin(), 2.5);
        let value = value_of(coaxial(&CoaxialPair::CylTorus(&cyl, &torus)));
        let [c0, c1] = as_two_circles(&value);
        for c in [c0, c1] {
            for i in 0..N {
                let p = c.subs(TAU * (i as f64) / (N as f64));
                let radial = (p.x * p.x + p.y * p.y).sqrt();
                assert!(
                    (radial - 2.5).abs() < SLACK,
                    "point {p:?} off the cylinder (radial {radial} != 5/2)"
                );
                let torus_residual = (radial - 2.0) * (radial - 2.0) + p.z * p.z - 1.0;
                assert!(
                    torus_residual.abs() < SLACK,
                    "point {p:?} off the torus (residual {torus_residual})"
                );
            }
        }
        let cyl = cylinder(Point3::origin(), 3.0);
        let value = value_of(coaxial(&CoaxialPair::CylTorus(&cyl, &torus)));
        let AnalyticIntersection::TangentCircle(c) = &value else {
            unreachable!("expected a tangent circle, got {value:?}");
        };
        for i in 0..N {
            let p = c.subs(TAU * (i as f64) / (N as f64));
            let radial = (p.x * p.x + p.y * p.y).sqrt();
            assert!(
                (radial - 3.0).abs() < SLACK,
                "point {p:?} off the cylinder (radial {radial} != 3)"
            );
            let torus_residual = (radial - 2.0) * (radial - 2.0) + p.z * p.z - 1.0;
            assert!(
                torus_residual.abs() < SLACK,
                "point {p:?} off the torus (residual {torus_residual})"
            );
        }
    }

    #[test]
    fn coax_same_kind_pairs_classify_exactly() {
        // CylCyl: equal radii → Coincident; different radii → Empty. The
        // radii ARE the carrier parameters: exact f64 equality, no intervals.
        let a = cylinder(Point3::origin(), 1.0);
        let b = cylinder(Point3::origin(), 1.0);
        let value = value_of(coaxial(&CoaxialPair::CylCyl(&a, &b)));
        assert!(matches!(value, AnalyticIntersection::Coincident));
        let c = cylinder(Point3::origin(), 2.0);
        let value = value_of(coaxial(&CoaxialPair::CylCyl(&a, &c)));
        assert!(matches!(value, AnalyticIntersection::Empty));

        // ConeCone: identical → Coincident; same angle, different apex →
        // Empty (the packet's prescribed parallel placement).
        let cone0 = cone(Point3::origin(), 0.75);
        let cone1 = cone(Point3::origin(), 0.75);
        let value = value_of(coaxial(&CoaxialPair::ConeCone(&cone0, &cone1)));
        assert!(matches!(value, AnalyticIntersection::Coincident));
        let cone2 = cone(Point3::new(0.0, 0.0, 2.0), 0.75);
        let value = value_of(coaxial(&CoaxialPair::ConeCone(&cone0, &cone2)));
        assert!(matches!(value, AnalyticIntersection::Empty));

        // Different angles, apexes apart: cone (0,0,0) with tan α0 = 3/4 and
        // cone (0,0,1) with tan α1 = 1/2. The profiles |z|·3/4 and
        // |z − 1|·1/2 meet in TWO circles (not the packet's "one"): the
        // left-region root z = (0·¾ − 1·½)/(¾ − ½) = −2 with radius
        // |−2|·¾ = 3/2, and the between-apexes root
        // z = (0·¾ + 1·½)/(¾ + ½) = 2/5 with radius (2/5)·¾ = 3/10. Both
        // circles are verified on both cones below (the on-both-carriers test
        // is the authority on the reduction).
        let cone3 = cone(Point3::new(0.0, 0.0, 1.0), 0.5);
        let value = value_of(coaxial(&CoaxialPair::ConeCone(&cone0, &cone3)));
        let [d0, d1] = as_two_circles(&value);
        for c in [d0, d1] {
            for i in 0..N {
                let p = c.subs(TAU * (i as f64) / (N as f64));
                let radial = (p.x * p.x + p.y * p.y).sqrt();
                let on0 = radial - (p.z - 0.0).abs() * 0.75;
                let on1 = radial - (p.z - 1.0).abs() * 0.5;
                assert!(
                    on0.abs() < SLACK,
                    "point {p:?} off cone 0 (radial {radial} vs |z|·3/4)"
                );
                assert!(
                    on1.abs() < SLACK,
                    "point {p:?} off cone 1 (radial {radial} vs |z−1|·1/2)"
                );
            }
        }
    }

    #[test]
    fn coax_undecidable_predicates_refuse() {
        // The quadratic classifier refuses a straddling discriminant: a = [1,1],
        // b = [1,1], c = [1/4 − w, 1/4 + w] gives Δ = 1 − 4c = [−4w, 4w],
        // which contains zero without being degenerate.
        let a = Interval::try_from((1.0, 1.0)).expect("valid interval");
        let b = Interval::try_from((1.0, 1.0)).expect("valid interval");
        let c = Interval::try_from((0.25 - 1.0e-9, 0.25 + 1.0e-9)).expect("valid interval"); // H-3: interval half-width around a dimensionless quadratic coefficient, not a length
        let out = classify_quadratic(a, b, c, 1.0, 1.0, 0.25);
        assert!(
            matches!(out, Err(Refusal::NumericallyUnresolved { .. })),
            "a straddling discriminant must refuse, got {out:?}"
        );

        // The decisive helpers: a [-w, w] enclosure is neither decisively zero
        // nor does it exclude zero.
        let straddle = Interval::try_from((-1.0e-9, 1.0e-9)).expect("valid interval"); // H-3: interval half-width, dimensionless
        assert!(!decisively_zero(straddle));
        assert!(!excludes_zero(straddle));
        let lo = Interval::try_from((0.0, 1.0)).expect("valid interval");
        let hi = Interval::try_from((2.0, 3.0)).expect("valid interval");
        let deg = Interval::try_from((0.0, 0.0)).expect("valid interval");
        assert_eq!(three_way(lo, hi), Some(Ordering::Less));
        assert_eq!(three_way(hi, lo), Some(Ordering::Greater));
        assert_eq!(three_way(deg, deg), Some(Ordering::Equal));
        let overlap_a = Interval::try_from((0.0, 2.0)).expect("valid interval");
        let overlap_b = Interval::try_from((1.0, 3.0)).expect("valid interval");
        assert_eq!(three_way(overlap_a, overlap_b), None);

        // The packet's literal inscribed-sphere witness — cone tan α = 3/4 at
        // the origin, sphere centre (0,0,1) radius 3/5 — is a genuine
        // tangency in real arithmetic, but 3/5 is not dyadic: the outward
        // enclosure of its square is a 1-ulp interval, so the discriminant
        // B² − 4AC straddles zero and the classification must refuse rather
        // than guess the tangent arm (see the note in
        // coax_cone_sphere_inscribed_tangent_circle).
        let near = cone(Point3::origin(), 0.75);
        let near_sph = Sphere::new(Point3::new(0.0, 0.0, 1.0), 3.0 / 5.0);
        let out = coaxial(&CoaxialPair::ConeSphere(&near, &near_sph));
        assert!(
            matches!(out, Err(Refusal::NumericallyUnresolved { .. })),
            "a non-dyadic inscribed tangency must refuse, got {out:?}"
        );

        // validate refuses off-axis placements with the exact-coordinate check.
        let off = cylinder(Point3::new(1.0, 0.0, 0.0), 1.0);
        let on = cylinder(Point3::origin(), 1.0);
        assert!(matches!(
            CoaxialPair::CylCyl(&off, &on).validate(),
            Err(Refusal::UnsupportedEnvelope(
                EnvelopeCase::NonCanonicalCarrier
            ))
        ));
        assert!(CoaxialPair::CylCyl(&on, &on).validate().is_ok());
        let sph_off = Sphere::new(Point3::new(0.0, 1.0, 0.0), 1.0);
        assert!(matches!(
            CoaxialPair::CylSphere(&on, &sph_off).validate(),
            Err(Refusal::UnsupportedEnvelope(
                EnvelopeCase::NonCanonicalCarrier
            ))
        ));
        // An off-axis pair refuses through `coaxial` itself.
        let out = coaxial(&CoaxialPair::CylCyl(&off, &on));
        assert!(matches!(
            out,
            Err(Refusal::UnsupportedEnvelope(
                EnvelopeCase::NonCanonicalCarrier
            ))
        ));
    }

    #[test]
    fn coax_certificate_is_exact() {
        // A two-circles, a tangent-circle and an empty outcome each carry
        // method == Exact and the AnalyticCarrier prop set true, field-by-field
        // at every return site (BG-EVD-002).
        let cyl = cylinder(Point3::origin(), 0.75);
        let sph = Sphere::new(Point3::origin(), 1.0);
        let out = coaxial(&CoaxialPair::CylSphere(&cyl, &sph)).expect("dyadic witness");
        assert_eq!(out.cert.method, Method::Exact);
        assert_eq!(out.cert.props.get(Prop::AnalyticCarrier), Truth::True);
        assert_eq!(out.cert.props.get(Prop::SoundEnclosure), Truth::Unknown);
        assert_eq!(out.cert.props.get(Prop::Provisional), Truth::Unknown);
        assert_eq!(out.cert.props.get(Prop::AnalyticPreserved), Truth::Unknown);

        let cyl = cylinder(Point3::origin(), 1.0);
        let sph = Sphere::new(Point3::origin(), 1.0);
        let out = coaxial(&CoaxialPair::CylSphere(&cyl, &sph)).expect("dyadic witness");
        assert_eq!(out.cert.method, Method::Exact);
        assert_eq!(out.cert.props.get(Prop::AnalyticCarrier), Truth::True);

        let cyl = cylinder(Point3::origin(), 2.0);
        let out = coaxial(&CoaxialPair::CylSphere(&cyl, &sph)).expect("dyadic witness");
        assert!(matches!(out.value, AnalyticIntersection::Empty));
        assert_eq!(out.cert.method, Method::Exact);
        assert_eq!(out.cert.props.get(Prop::AnalyticCarrier), Truth::True);
    }
}
