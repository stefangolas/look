//! BG-SOL-P0-REC — the structural recognizer: produce a witness, not a type.
//!
//! `recognize_curve` / `recognize_surface` answer "what canonical analytic
//! carrier is this stored curve or surface, and what certified parameter
//! correspondence φ maps the stored parameterization onto the canonical one"
//! (`S_stored = S_canonical ∘ φ`). The witness is `CanonicalCarrierWitness`
//! (docs/SOLVER_FAMILY_PLAN.md §2): coincidence (S5.0) is then a lookup on the
//! witness, never a re-solve.
//!
//! The canonical set is the analytic arms of `Curve`/`Surface` (`Line`,
//! `Circle`; `Plane`, `Cylinder`, `Cone`, `Sphere`, `Torus`), plus the two
//! derived constructions M1 needs — an `ExtrudedCurve` of a line or circle,
//! and a `Processor`-placed analytic carrier. Exact spline→analytic detection
//! is a documented later packet; splines and intersection curves are
//! `Unrecognized` here.
//!
//! The map is a two-armed sum: a curve carries one `ParamMap`, a surface the
//! `(u, v)` pair. The map is certified by construction (the affine
//! correspondence derived from the carriers' exact parameter ranges), never by
//! the sampling the tests perform.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

use crate::prelude::*;
use truck_base::param_map::ParamMap;

/// A canonical analytic curve carrier: the analytic arms of `Curve`.
#[derive(Clone, Debug, PartialEq)]
pub enum CanonicalCurve {
    /// A line.
    Line(Line<Point3>),
    /// A placed analytic circle.
    Circle(Processor<TrimmedCurve<UnitCircle<Point3>>, Matrix4>),
}

/// A canonical analytic surface carrier: the analytic arms of `Surface`.
#[derive(Clone, Debug, PartialEq)]
pub enum CanonicalSurface {
    /// A plane.
    Plane(Plane),
    /// A cylinder.
    Cylinder(Cylinder),
    /// A cone.
    Cone(Cone),
    /// A sphere.
    Sphere(Sphere),
    /// A torus.
    Torus(Torus),
    /// A canonical analytic carrier composed with an affine placement. The
    /// bare carriers (bare `Cylinder`, `Cone`, …) are z-axes-only; a rotated
    /// analytic carrier is representable only as `Placed` (the canonical.rs
    /// `Processor` rule). Exact under affine.
    Placed(Processor<Box<CanonicalSurface>, Matrix4>),
}

/// A canonical carrier: curve or surface.
#[derive(Clone, Debug, PartialEq)]
pub enum CanonicalCarrier {
    /// A canonical curve carrier.
    Curve(CanonicalCurve),
    /// A canonical surface carrier.
    Surface(CanonicalSurface),
}

/// How a derived canonical carrier was obtained.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConstructionWitness {
    /// The carrier is the stored surface's analytic inner carrier under an
    /// affine placement.
    Placed,
    /// The carrier is obtained by sweeping a canonical profile curve.
    Extruded,
}

/// The certified parameter correspondence φ with `S_stored = S_canonical ∘ φ`.
///
/// The plan's §4 single `ParamMap` is the curve case; a surface needs the
/// `(u, v)` pair, so the correspondence is a two-armed sum (recorded deviation
/// in the packet's RESULT).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CanonicalParamMap {
    /// A curve parameter correspondence φ(t).
    Curve(ParamMap),
    /// A surface parameter correspondence (φ_u, φ_v).
    Surface {
        /// The parameter correspondence on `u`.
        u: ParamMap,
        /// The parameter correspondence on `v`.
        v: ParamMap,
    },
}

/// The structural recognizer's witness (plan §2).
#[derive(Clone, Debug, PartialEq)]
pub enum CanonicalCarrierWitness {
    /// `S_stored` IS the canonical carrier under φ (a directly-canonical
    /// variant, φ = IDENTITY).
    ExactCanonical {
        /// The canonical carrier.
        carrier: CanonicalCarrier,
        /// The certified parameter correspondence.
        map: CanonicalParamMap,
    },
    /// `S_stored = S_canonical ∘ φ` by construction, `provenance` says how.
    Derived {
        /// The canonical carrier.
        carrier: CanonicalCarrier,
        /// How the derived carrier was obtained.
        provenance: ConstructionWitness,
        /// The certified parameter correspondence.
        map: CanonicalParamMap,
    },
    /// No canonical carrier recognized; treat as a generic spline carrier.
    Unrecognized,
}

/// Recognize the canonical carrier of a stored curve.
pub fn recognize_curve(c: &Curve) -> CanonicalCarrierWitness {
    match c {
        Curve::Line(line) => CanonicalCarrierWitness::ExactCanonical {
            carrier: CanonicalCarrier::Curve(CanonicalCurve::Line(*line)),
            map: CanonicalParamMap::Curve(ParamMap::IDENTITY),
        },
        Curve::Circle(circle) => CanonicalCarrierWitness::ExactCanonical {
            carrier: CanonicalCarrier::Curve(CanonicalCurve::Circle(*circle)),
            map: CanonicalParamMap::Curve(ParamMap::IDENTITY),
        },
        // Exact spline→analytic detection is a documented later packet; the
        // profile builders emit `Line`/`Circle` directly.
        Curve::BSplineCurve(_) | Curve::NurbsCurve(_) | Curve::IntersectionCurve(_) => {
            CanonicalCarrierWitness::Unrecognized
        }
    }
}

/// Recognize the canonical carrier of a stored surface.
pub fn recognize_surface(s: &Surface) -> CanonicalCarrierWitness {
    match s {
        Surface::Plane(plane) => exact_surface(CanonicalSurface::Plane(*plane)),
        Surface::Cylinder(cylinder) => exact_surface(CanonicalSurface::Cylinder(*cylinder)),
        Surface::Cone(cone) => exact_surface(CanonicalSurface::Cone(*cone)),
        Surface::Sphere(sphere) => exact_surface(CanonicalSurface::Sphere(*sphere)),
        Surface::Torus(torus) => exact_surface(CanonicalSurface::Torus(*torus)),
        // Phase-0 scope: splines are `Unrecognized`, and revolve recognition
        // lands with S2's `revolve_profile`, which is where it is consumed.
        Surface::BSplineSurface(_) | Surface::NurbsSurface(_) | Surface::RevolutedCurve(_) => {
            CanonicalCarrierWitness::Unrecognized
        }
        Surface::Processor(processor) => {
            // The inner carrier's canonical form rides under the same affine
            // placement. `Processor` composes the affine map on output without
            // reparameterizing, so φ = IDENTITY and the placement lives in the
            // carrier.
            let canonical_inner = match &**processor.entity() {
                Surface::Plane(plane) => CanonicalSurface::Plane(*plane),
                Surface::Cylinder(cylinder) => CanonicalSurface::Cylinder(*cylinder),
                Surface::Cone(cone) => CanonicalSurface::Cone(*cone),
                Surface::Sphere(sphere) => CanonicalSurface::Sphere(*sphere),
                Surface::Torus(torus) => CanonicalSurface::Torus(*torus),
                _ => return CanonicalCarrierWitness::Unrecognized,
            };
            CanonicalCarrierWitness::Derived {
                carrier: CanonicalCarrier::Surface(CanonicalSurface::Placed(
                    Processor::with_transform(Box::new(canonical_inner), *processor.transform()),
                )),
                provenance: ConstructionWitness::Placed,
                map: CanonicalParamMap::Surface {
                    u: ParamMap::IDENTITY,
                    v: ParamMap::IDENTITY,
                },
            }
        }
        Surface::ExtrudedCurve(extruded) => {
            let vector = extruded.extruding_vector();
            match extruded.entity_curve() {
                Curve::Line(Line(a, b)) => {
                    let a = *a;
                    let b = *b;
                    // An extrusion parallel to the profile is a degenerate
                    // "surface" that is really a line; refuse it.
                    if (b - a).cross(vector).magnitude() == 0.0 {
                        return CanonicalCarrierWitness::Unrecognized;
                    }
                    // `Line::subs(t) = a + t(b-a)` over `t ∈ [0,1]`, and
                    // `Plane::new(a, b, a+v)` is `a + u(b-a) + w·v` over
                    // `u,w ∈ (0,1)`, so φ(u,w) = (u,w) = IDENTITY.
                    CanonicalCarrierWitness::Derived {
                        carrier: CanonicalCarrier::Surface(CanonicalSurface::Plane(Plane::new(
                            a,
                            b,
                            a + vector,
                        ))),
                        provenance: ConstructionWitness::Extruded,
                        map: CanonicalParamMap::Surface {
                            u: ParamMap::IDENTITY,
                            v: ParamMap::IDENTITY,
                        },
                    }
                }
                Curve::Circle(circle) => {
                    // The exact cylinder test copied from canonical.rs
                    // `to_same_geometry` (lines ~997-1021): the placed circle
                    // is an exact z-preserving placement extruded along ±z.
                    let Matrix4 {
                        x: m1,
                        y: m2,
                        z: m3,
                        w: tw,
                    } = *circle.transform();
                    let radius = m1.magnitude();
                    let center = tw.to_point();
                    let z_preserving = m1.z == 0.0
                        && m2.z == 0.0
                        && m3.x == 0.0
                        && m3.y == 0.0
                        && radius == m2.magnitude()
                        && m1.dot(m2) == 0.0
                        && radius > 0.0;
                    let axis_parallel = vector.x == 0.0 && vector.y == 0.0;
                    let finite_center =
                        center.x.is_finite() && center.y.is_finite() && center.z.is_finite();
                    if !(z_preserving && axis_parallel && finite_center) {
                        return CanonicalCarrierWitness::Unrecognized;
                    }
                    let cylinder = match Cylinder::new(center, radius) {
                        Ok(cylinder) => cylinder.value,
                        Err(_) => return CanonicalCarrierWitness::Unrecognized,
                    };
                    // `UnitCircle::subs(t) = (cos t, sin t, 0)` over
                    // `t ∈ [0, TAU)`, so the placed circle's parameter IS the
                    // angle θ = u. The extrusion direction ±z gives the
                    // canonical cylinder's `v` as `|v|·w` over `w ∈ [0,1]`,
                    // so φ_v sends `[0,1]` onto `[0, |v|]`.
                    let v_map = match ParamMap::from_ranges(0.0, 1.0, 0.0, vector.magnitude()) {
                        Some(map) => map,
                        None => return CanonicalCarrierWitness::Unrecognized,
                    };
                    CanonicalCarrierWitness::Derived {
                        carrier: CanonicalCarrier::Surface(CanonicalSurface::Cylinder(cylinder)),
                        provenance: ConstructionWitness::Extruded,
                        map: CanonicalParamMap::Surface {
                            u: ParamMap::IDENTITY,
                            v: v_map,
                        },
                    }
                }
                // Only `Line`/`Circle` profiles are canonical in Phase 0; a
                // nested extrusion is `Unrecognized` with the rest.
                Curve::BSplineCurve(_) | Curve::NurbsCurve(_) | Curve::IntersectionCurve(_) => {
                    CanonicalCarrierWitness::Unrecognized
                }
            }
        }
    }
}

/// An exact-canonical surface witness with an IDENTITY map.
fn exact_surface(carrier: CanonicalSurface) -> CanonicalCarrierWitness {
    CanonicalCarrierWitness::ExactCanonical {
        carrier: CanonicalCarrier::Surface(carrier),
        map: CanonicalParamMap::Surface {
            u: ParamMap::IDENTITY,
            v: ParamMap::IDENTITY,
        },
    }
}

#[cfg(test)]
#[allow(clippy::panic)]
mod tests {
    use super::*;
    use std::f64::consts::TAU;

    /// A placed full-range unit circle with the given center and radius.
    ///
    /// The placement matrix is an exact z-preserving uniform placement, the
    /// shape `to_same_geometry`'s cylinder test recognizes.
    fn placed_circle(
        center: Point3,
        radius: f64,
    ) -> Processor<TrimmedCurve<UnitCircle<Point3>>, Matrix4> {
        let m = Matrix4 {
            x: Vector4::new(radius, 0.0, 0.0, 0.0),
            y: Vector4::new(0.0, radius, 0.0, 0.0),
            z: Vector4::new(0.0, 0.0, 1.0, 0.0),
            w: Vector4::new(center.x, center.y, center.z, 1.0),
        };
        Processor::with_transform(
            TrimmedCurve::new(UnitCircle::<Point3>::new(), (0.0, TAU)),
            m,
        )
    }

    /// Evaluate a canonical surface carrier at `(u, v)`. A `Placed` carrier is
    /// the inner carrier's evaluation under its affine placement.
    fn canonical_subs(carrier: &CanonicalSurface, u: f64, v: f64) -> Point3 {
        match carrier {
            CanonicalSurface::Plane(surface) => surface.subs(u, v),
            CanonicalSurface::Cylinder(surface) => surface.subs(u, v),
            CanonicalSurface::Cone(surface) => surface.subs(u, v),
            CanonicalSurface::Sphere(surface) => surface.subs(u, v),
            CanonicalSurface::Torus(surface) => surface.subs(u, v),
            CanonicalSurface::Placed(placed) => {
                placed
                    .transform()
                    .transform_point(canonical_subs(placed.entity(), u, v))
            }
        }
    }

    /// The map-verification helper: sample `stored.subs(u, w)` against
    /// `canonical.subs(map.u.apply_f64(u), map.v.apply_f64(w))` on a grid and
    /// assert `diff <= 64.0 * TOLERANCE`, returning the observed maximum.
    ///
    /// Sampling is a regression witness for the map; the map is certified by
    /// its construction, never by this sampling.
    fn max_map_deviation(
        stored: &Surface,
        carrier: &CanonicalSurface,
        map: &CanonicalParamMap,
        (u0, u1): (f64, f64),
        (v0, v1): (f64, f64),
    ) -> f64 {
        let CanonicalParamMap::Surface { u, v } = map else {
            panic!("a surface witness must carry a two-armed parameter map");
        };
        const SAMPLES: usize = 32;
        let mut max_deviation: f64 = 0.0;
        for i in 0..=SAMPLES {
            for j in 0..=SAMPLES {
                let us = u0 + (u1 - u0) * i as f64 / SAMPLES as f64;
                let vs = v0 + (v1 - v0) * j as f64 / SAMPLES as f64;
                let stored_point = stored.subs(us, vs);
                let canonical_point = canonical_subs(carrier, u.apply_f64(us), v.apply_f64(vs));
                let deviation = (stored_point - canonical_point).magnitude();
                assert!(
                    deviation <= 64.0 * TOLERANCE,
                    "sampled deviation {deviation} exceeds 64 * TOLERANCE at ({us}, {vs})"
                );
                max_deviation = max_deviation.max(deviation);
            }
        }
        max_deviation
    }

    /// A `Derived` witness's carrier and map, asserting the provenance.
    fn derived_parts(
        witness: CanonicalCarrierWitness,
        expected: ConstructionWitness,
    ) -> (CanonicalCarrier, CanonicalParamMap) {
        match witness {
            CanonicalCarrierWitness::Derived {
                carrier,
                provenance,
                map,
            } => {
                assert_eq!(provenance, expected);
                (carrier, map)
            }
            other => panic!("expected a Derived witness, got {other:?}"),
        }
    }

    #[test]
    fn recognize_line_and_plane_are_exact_canonical() {
        let line = Line(Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 0.0, 0.0));
        let curve = Curve::Line(line);
        let witness = recognize_curve(&curve);
        let (carrier, map) = match witness {
            CanonicalCarrierWitness::ExactCanonical { carrier, map } => (carrier, map),
            other => panic!("expected ExactCanonical, got {other:?}"),
        };
        assert_eq!(map, CanonicalParamMap::Curve(ParamMap::IDENTITY));
        match carrier {
            CanonicalCarrier::Curve(CanonicalCurve::Line(got)) => assert_eq!(got, line),
            other => panic!("expected a Line carrier, got {other:?}"),
        }

        let plane = Plane::xy();
        let surface = Surface::Plane(plane);
        let witness = recognize_surface(&surface);
        let (carrier, map) = match witness {
            CanonicalCarrierWitness::ExactCanonical { carrier, map } => (carrier, map),
            other => panic!("expected ExactCanonical, got {other:?}"),
        };
        assert_eq!(
            map,
            CanonicalParamMap::Surface {
                u: ParamMap::IDENTITY,
                v: ParamMap::IDENTITY,
            }
        );
        match carrier {
            CanonicalCarrier::Surface(CanonicalSurface::Plane(got)) => assert_eq!(got, plane),
            other => panic!("expected a Plane carrier, got {other:?}"),
        }
    }

    #[test]
    fn recognize_extruded_line_is_plane() {
        let surface = Surface::ExtrudedCurve(ExtrudedCurve::by_extrusion(
            Curve::Line(Line(Point3::new(0.0, 0.0, 0.0), Point3::new(2.0, 0.0, 0.0))),
            Vector3::unit_z(),
        ));
        let witness = recognize_surface(&surface);
        let (carrier, map) = derived_parts(witness, ConstructionWitness::Extruded);
        assert_eq!(
            map,
            CanonicalParamMap::Surface {
                u: ParamMap::IDENTITY,
                v: ParamMap::IDENTITY,
            }
        );
        let CanonicalCarrier::Surface(CanonicalSurface::Plane(plane)) = carrier else {
            panic!("expected a Plane carrier");
        };
        let max_deviation = max_map_deviation(
            &surface,
            &CanonicalSurface::Plane(plane),
            &map,
            (0.0, 1.0),
            (0.0, 1.0),
        );
        println!("extruded line -> plane: max sampled deviation {max_deviation}");
        assert!(max_deviation <= 64.0 * TOLERANCE);
    }

    #[test]
    fn recognize_extruded_circle_is_cylinder() {
        let surface = Surface::ExtrudedCurve(ExtrudedCurve::by_extrusion(
            Curve::Circle(placed_circle(Point3::new(1.0, 2.0, 0.0), 3.0)),
            Vector3::new(0.0, 0.0, 5.0),
        ));
        let witness = recognize_surface(&surface);
        let (carrier, map) = derived_parts(witness, ConstructionWitness::Extruded);
        let CanonicalCarrier::Surface(CanonicalSurface::Cylinder(cylinder)) = carrier else {
            panic!("expected a Cylinder carrier");
        };
        assert_eq!(cylinder.center(), Point3::new(1.0, 2.0, 0.0));
        assert_eq!(cylinder.radius(), 3.0);
        let CanonicalParamMap::Surface { u: _, v } = map else {
            panic!("a surface witness must carry a two-armed parameter map");
        };
        assert_eq!(v.apply_f64(1.0), 5.0);
        let max_deviation = max_map_deviation(
            &surface,
            &CanonicalSurface::Cylinder(cylinder),
            &map,
            (0.0, TAU),
            (0.0, 1.0),
        );
        println!("extruded circle -> cylinder: max sampled deviation {max_deviation}");
        assert!(max_deviation <= 64.0 * TOLERANCE);
    }

    #[test]
    fn recognize_skew_or_degenerate_extrude_is_unrecognized() {
        // (a) a circle extruded not along its axis.
        let skew = Surface::ExtrudedCurve(ExtrudedCurve::by_extrusion(
            Curve::Circle(placed_circle(Point3::new(1.0, 2.0, 0.0), 3.0)),
            Vector3::new(1.0, 0.0, 0.0),
        ));
        assert!(
            matches!(
                recognize_surface(&skew),
                CanonicalCarrierWitness::Unrecognized
            ),
            "an off-axis circle extrusion must be unrecognized"
        );

        // (b) an extrusion parallel to the profile line — a degenerate
        // "surface" that is really a line.
        let degenerate = Surface::ExtrudedCurve(ExtrudedCurve::by_extrusion(
            Curve::Line(Line(Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 0.0, 0.0))),
            Vector3::new(2.0, 0.0, 0.0),
        ));
        assert!(
            matches!(
                recognize_surface(&degenerate),
                CanonicalCarrierWitness::Unrecognized
            ),
            "a profile-parallel extrusion must be unrecognized"
        );
    }

    #[test]
    fn recognize_processor_places_the_inner_carrier() {
        let translation = Matrix4::from_translation(Vector3::new(1.0, 2.0, 3.0));
        let surface = Surface::Processor(Processor::with_transform(
            Box::new(Surface::Plane(Plane::xy())),
            translation,
        ));
        let witness = recognize_surface(&surface);
        let (carrier, map) = derived_parts(witness, ConstructionWitness::Placed);
        let CanonicalCarrier::Surface(CanonicalSurface::Placed(placed)) = carrier else {
            panic!("expected a Placed carrier");
        };
        assert_eq!(*placed.transform(), translation);
        let CanonicalSurface::Plane(inner) = &**placed.entity() else {
            panic!("the placed inner carrier must be a plane");
        };
        assert_eq!(*inner, Plane::xy());
        assert_eq!(
            map,
            CanonicalParamMap::Surface {
                u: ParamMap::IDENTITY,
                v: ParamMap::IDENTITY,
            }
        );
        // The placed surface's own `subs` composes the affine map exactly, so
        // sampling the stored surface against the placed carrier checks
        // `S_stored = S_canonical ∘ φ` (equivalently, the inner plane's
        // `subs(u, v) + (1,2,3)`).
        let max_deviation = max_map_deviation(
            &surface,
            &CanonicalSurface::Placed(placed),
            &map,
            (0.0, 1.0),
            (0.0, 1.0),
        );
        println!("processor placed plane: max sampled deviation {max_deviation}");
        assert!(max_deviation <= 64.0 * TOLERANCE);
    }
}
