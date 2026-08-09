//! Newtype wrappers that apply [`MeshingPolicy`] to STEP geometry without
//! modifying Truck.
//!
//! Truck's tessellation entry point takes a single scalar tolerance and
//! derives every curve's polyline and every surface's interior grid from it,
//! so a per-feature angular floor cannot be expressed through that scalar
//! alone. Rather than reimplement subdivision (which would have to re-derive
//! Truck's handling of partial arcs, full circles, reversed parameter ranges,
//! and periodic seam crossings), these wrappers forward *every* trait method
//! to the underlying STEP geometry unchanged — except
//! [`ParameterDivision1D::parameter_division`] /
//! [`ParameterDivision2D::parameter_division`], which compute an effective
//! tolerance (or, for surfaces, a minimum circumferential sample count) from
//! the policy and then delegate to the inner geometry. Truck's own machinery
//! does the actual subdivision, so the hard cases are handled exactly as
//! before.
//!
//! The wrappers preserve the inner geometry's identity, so the cylinder/cone
//! identification callbacks used by the formal recovery routes still see the
//! real `Surface`/`Curve3D` after a one-line unwrap. [`wrap_shell`] rebuilds
//! a compressed shell with wrapped edges and faces; because a compressed shell
//! samples each edge once by id, the shared-canonical-polyline invariant Truck
//! already relies on is kept.
//!
//! # Topology-safe eligibility
//!
//! Densifying a circular edge flips the constrained-Delaunay triangulator on
//! neighboring surfaces whose realization is density-sensitive — observed in
//! the NIST corpus as bidirectional sphere and B-spline face-recovery flips.
//! The cause is not the targeted cylinder/cone interiors but globally densifying
//! a *shared* circular boundary that also trims a sphere or a spline. So
//! "circular" alone is not sufficient eligibility.
//!
//! A circular edge is densified only when **every** incident face belongs to
//! the stable target set {plane, cylinder, cone}. A typical mechanical hole —
//! planar face ↔ circular edge ↔ cylindrical wall — stays eligible; a blended
//! cylinder ↔ sphere or cylinder ↔ spline transition keeps baseline sampling
//! on the shared edge. Cylindrical/conical interior floors are gated the same
//! way: a revolved face's circumferential floor applies only when its circular
//! boundaries are all eligible, avoiding a dense-interior/coarse-boundary
//! mismatch on mixed neighborhoods. Because the whole connected neighborhood
//! uses one policy per edge, adjacency stays crack-free.

use truck_meshalgo::prelude::{
    BoundedCurve, D2, ParameterDivision1D, ParameterDivision2D, ParameterRange, ParametricCurve,
    ParametricSurface, ParametricSurface3D, Point3, SPHint2D, SearchNearestParameter,
    SearchParameter, Vector3,
};
use truck_stepio::r#in::step_geometry::{Conic3D, Curve3D, ElementarySurface, Surface};
use truck_topology::compress::{CompressedEdge, CompressedFace, CompressedShell};

use crate::step::circular_arc::{decode_source_circle, decode_transformed_circle};
use crate::step::meshing_policy::MeshingPolicy;

/// The stable target surface set: analytic surfaces whose constrained
/// triangulator is not density-sensitive. A circular edge is eligible for the
/// angular floor only when every incident face is one of these.
fn is_target_surface(surface: &Surface) -> bool {
    matches!(
        surface,
        Surface::ElementarySurface(ElementarySurface::Plane(_))
            | Surface::ElementarySurface(ElementarySurface::CylindricalSurface(_))
            | Surface::ElementarySurface(ElementarySurface::ConicalSurface(_))
    )
}

/// Whether a surface is a cylinder or cone (the revolved elementary surfaces
/// whose circumferential direction the policy can floor).
fn is_revolved_target(surface: &Surface) -> bool {
    matches!(
        surface,
        Surface::ElementarySurface(ElementarySurface::CylindricalSurface(_))
            | Surface::ElementarySurface(ElementarySurface::ConicalSurface(_))
    )
}

/// The certified radius of a STEP curve if it is a circle, else `None`.
///
/// A source-declared `circle` is decoded by family; a bare `ellipse` is
/// decoded by the exact Gram predicate, which refuses a genuinely non-circular
/// ellipse. Only certified circles can receive the angular floor.
fn circular_radius(curve: &Curve3D) -> Option<f64> {
    match curve {
        Curve3D::Conic(Conic3D::Circle(ellipse)) => decode_source_circle(ellipse)
            .ok()
            .map(|arc| arc.radius().get()),
        Curve3D::Conic(Conic3D::Ellipse(ellipse)) => decode_transformed_circle(ellipse)
            .ok()
            .map(|arc| arc.radius().get()),
        _ => None,
    }
}

/// A STEP edge curve carrying the meshing policy and a precomputed eligibility
/// flag.
///
/// Forwards [`ParametricCurve`] / [`BoundedCurve`] unchanged and overrides
/// [`ParameterDivision1D`] to apply the angular floor — but only when
/// [`eligible`](Self::new) is set, i.e. the edge is a certified circle whose
/// every incident face is in the target set. Ineligible edges (non-circles,
/// or circles shared with a sphere/spline/torus/etc.) divide at the baseline
/// linear tolerance, exactly as before.
#[derive(Clone, Debug)]
pub struct PolicyCurve {
    inner: Curve3D,
    policy: MeshingPolicy,
    eligible: bool,
}

impl PolicyCurve {
    /// Wrap a STEP curve with a policy and an eligibility flag.
    pub fn new(inner: Curve3D, policy: MeshingPolicy, eligible: bool) -> Self {
        Self {
            inner,
            policy,
            eligible,
        }
    }

    /// The underlying STEP curve, for the identification callbacks.
    pub fn inner(&self) -> &Curve3D {
        &self.inner
    }
}

impl ParametricCurve for PolicyCurve {
    type Point = Point3;
    type Vector = Vector3;
    #[inline]
    fn subs(&self, t: f64) -> Self::Point {
        self.inner.subs(t)
    }
    #[inline]
    fn der(&self, t: f64) -> Self::Vector {
        self.inner.der(t)
    }
    #[inline]
    fn der2(&self, t: f64) -> Self::Vector {
        self.inner.der2(t)
    }
    #[inline]
    fn der_n(&self, n: usize, t: f64) -> Self::Vector {
        self.inner.der_n(n, t)
    }
    #[inline]
    fn parameter_range(&self) -> ParameterRange {
        self.inner.parameter_range()
    }
    #[inline]
    fn try_range_tuple(&self) -> Option<(f64, f64)> {
        self.inner.try_range_tuple()
    }
    #[inline]
    fn period(&self) -> Option<f64> {
        self.inner.period()
    }
}

impl BoundedCurve for PolicyCurve {
    #[inline]
    fn evaluation_range(&self) -> (f64, f64) {
        self.inner.evaluation_range()
    }

    #[inline]
    fn basis_is_partition_of_unity(&self, t: f64) -> bool {
        self.inner.basis_is_partition_of_unity(t)
    }
}

impl ParameterDivision1D for PolicyCurve {
    type Point = Point3;
    fn parameter_division(&self, range: (f64, f64), tol: f64) -> (Vec<f64>, Vec<Self::Point>) {
        let eff = if self.eligible {
            match circular_radius(&self.inner) {
                Some(radius) => self.policy.effective_curve_tolerance(tol, radius),
                None => tol,
            }
        } else {
            // Ineligible: the linear tolerance is the only mechanism, and it is
            // already world-space. Forward unchanged.
            tol
        };
        self.inner.parameter_division(range, eff)
    }
}

/// A STEP surface carrying the meshing policy and a precomputed
/// interior-floor eligibility flag.
///
/// Forwards [`ParametricSurface`] / [`SearchParameter`] / [`SearchNearestParameter`]
/// unchanged and overrides [`ParameterDivision2D`] to hold the circumferential
/// direction of cylindrical and conical surfaces to the policy's angular floor
/// — but only when [`interior_eligible`](Self::new) is set, i.e. the face is a
/// cylinder/cone whose circular boundaries are all eligible. Otherwise the
/// surface divides at the baseline tolerance, preserving the
/// dense-interior/coarse-boundary match.
#[derive(Clone, Debug)]
pub struct PolicySurface {
    inner: Surface,
    policy: MeshingPolicy,
    interior_eligible: bool,
}

impl PolicySurface {
    /// Wrap a STEP surface with a policy and an interior-floor eligibility flag.
    pub fn new(inner: Surface, policy: MeshingPolicy, interior_eligible: bool) -> Self {
        Self {
            inner,
            policy,
            interior_eligible,
        }
    }

    /// The underlying STEP surface, for the identification callbacks.
    pub fn inner(&self) -> &Surface {
        &self.inner
    }
}

impl ParametricSurface for PolicySurface {
    type Point = Point3;
    type Vector = Vector3;
    #[inline]
    fn subs(&self, u: f64, v: f64) -> Self::Point {
        self.inner.subs(u, v)
    }
    #[inline]
    fn uder(&self, u: f64, v: f64) -> Self::Vector {
        self.inner.uder(u, v)
    }
    #[inline]
    fn vder(&self, u: f64, v: f64) -> Self::Vector {
        self.inner.vder(u, v)
    }
    #[inline]
    fn uuder(&self, u: f64, v: f64) -> Self::Vector {
        self.inner.uuder(u, v)
    }
    #[inline]
    fn uvder(&self, u: f64, v: f64) -> Self::Vector {
        self.inner.uvder(u, v)
    }
    #[inline]
    fn vvder(&self, u: f64, v: f64) -> Self::Vector {
        self.inner.vvder(u, v)
    }
    #[inline]
    fn der_mn(&self, m: usize, n: usize, u: f64, v: f64) -> Self::Vector {
        self.inner.der_mn(m, n, u, v)
    }
    #[inline]
    fn parameter_range(&self) -> (ParameterRange, ParameterRange) {
        self.inner.parameter_range()
    }
    #[inline]
    fn try_range_tuple(&self) -> (Option<(f64, f64)>, Option<(f64, f64)>) {
        self.inner.try_range_tuple()
    }
    #[inline]
    fn u_period(&self) -> Option<f64> {
        self.inner.u_period()
    }
    #[inline]
    fn v_period(&self) -> Option<f64> {
        self.inner.v_period()
    }
}

impl ParametricSurface3D for PolicySurface {
    // Forward the normal-family methods rather than inheriting the trait's
    // `uder × vder` defaults: the inner `Surface`'s derived `ParametricSurface3D`
    // delegates to per-variant impls, and a variant may carry a normal-family
    // override. Using the defaults here would diverge from the unwrapped path
    // and perturb borderline CDTs.
    #[inline]
    fn normal(&self, u: f64, v: f64) -> Vector3 {
        self.inner.normal(u, v)
    }
    #[inline]
    fn normal_uder(&self, u: f64, v: f64) -> Vector3 {
        self.inner.normal_uder(u, v)
    }
    #[inline]
    fn normal_vder(&self, u: f64, v: f64) -> Vector3 {
        self.inner.normal_vder(u, v)
    }
}

impl ParameterDivision2D for PolicySurface {
    fn parameter_division(
        &self,
        range: ((f64, f64), (f64, f64)),
        tol: f64,
    ) -> (Vec<f64>, Vec<f64>) {
        if !self.interior_eligible {
            return self.inner.parameter_division(range, tol);
        }
        // `RevolutedCurve::subs(u, v)` rotates by `v`, so `v` is the
        // circumferential (revolution) direction and `u` is the axial (generatrix)
        // direction. The linear tolerance, capped by the absolute deflection,
        // governs the axial direction and provides the base circumferential grid;
        // the angular floor then lifts the circumferential count if the linear
        // term left it below the policy's proportional share of a revolution.
        let capped_tol = tol.min(self.policy.maximum_absolute_deflection);
        let (udiv, vdiv) = self.inner.parameter_division(range, capped_tol);
        let (v_min, v_max) = range.1;
        let needed = self.policy.surface_floor_segments(v_max - v_min);
        if needed >= 2 && vdiv.len() < needed {
            (udiv, uniform_linspace(v_min, v_max, needed))
        } else {
            (udiv, vdiv)
        }
    }
}

impl SearchParameter<D2> for PolicySurface {
    type Point = Point3;
    #[inline]
    fn search_parameter<H: Into<SPHint2D>>(
        &self,
        point: Self::Point,
        hint: H,
        trials: usize,
    ) -> Option<(f64, f64)> {
        self.inner.search_parameter(point, hint, trials)
    }
    // Forward the structure-derived seeds (e.g. a B-spline's knot-span starts)
    // rather than inheriting the empty default. The default drops the piecewise
    // geometry's own inverse hints, starving the projection Newton iteration
    // and turning faces that render on the unwrapped path into
    // `NoSurfaceProduced`.
    #[inline]
    fn search_parameter_seeds(&self) -> Vec<(f64, f64)> {
        self.inner.search_parameter_seeds()
    }
}

impl SearchNearestParameter<D2> for PolicySurface {
    type Point = Point3;
    #[inline]
    fn search_nearest_parameter<H: Into<SPHint2D>>(
        &self,
        point: Self::Point,
        hint: H,
        trials: usize,
    ) -> Option<(f64, f64)> {
        self.inner.search_nearest_parameter(point, hint, trials)
    }
}

/// `n` parameters evenly spaced over `[a, b]`, endpoints inclusive. `n >= 2`.
fn uniform_linspace(a: f64, b: f64, n: usize) -> Vec<f64> {
    debug_assert!(n >= 2);
    let step = (b - a) / (n - 1) as f64;
    (0..n).map(|i| a + step * i as f64).collect()
}

/// Per-edge and per-face eligibility for the topology-safe angular floor.
///
/// [`edge_eligible`](Self::edge_eligible) is true for a canonical topological
/// edge exactly when it is a certified circle and every face incident on it is
/// in the target set {plane, cylinder, cone}. [`face_interior_eligible`]
/// `(Self::face_interior_eligible)` is true for a cylinder/cone face exactly
/// when every circular edge in its boundary is edge-eligible, so the
/// circumferential interior floor never creates a dense-interior/coarse-boundary
/// mismatch on a mixed neighborhood.
struct Eligibility {
    edge_eligible: Vec<bool>,
    face_interior_eligible: Vec<bool>,
}

impl Eligibility {
    fn compute(shell: &CompressedShell<Point3, Curve3D, Surface>) -> Self {
        // Map each edge index to the faces that reference it. A face references
        // an edge once per boundary loop occurrence; collecting duplicates is
        // harmless for the all-targets check.
        let mut edge_faces: Vec<Vec<usize>> = vec![Vec::new(); shell.edges.len()];
        for (face_index, face) in shell.faces.iter().enumerate() {
            for boundary in &face.boundaries {
                for edge_index in boundary {
                    edge_faces[edge_index.index].push(face_index);
                }
            }
        }

        let edge_eligible: Vec<bool> = shell
            .edges
            .iter()
            .enumerate()
            .map(|(e, edge)| {
                // Must be a certified circle. `eligible` carrying a non-circle
                // would be harmless (it forwards `tol` unchanged) but computing
                // it here keeps the invariant that eligible ⟹ circle explicit.
                if circular_radius(&edge.curve).is_none() {
                    return false;
                }
                edge_faces[e]
                    .iter()
                    .all(|&f| is_target_surface(&shell.faces[f].surface))
            })
            .collect();

        let face_interior_eligible: Vec<bool> = shell
            .faces
            .iter()
            .enumerate()
            .map(|(f, face)| {
                if !is_revolved_target(&face.surface) {
                    return false;
                }
                // The interior floor applies only when every circular boundary
                // edge of this face is edge-eligible. A circular edge that is
                // shared with a sphere/spline/etc. is ineligible, so the face
                // keeps its baseline interior division to stay matched with
                // that edge's baseline boundary sampling.
                let mut all_circular_eligible = true;
                for boundary in &face.boundaries {
                    for edge_index in boundary {
                        let e = edge_index.index;
                        if circular_radius(&shell.edges[e].curve).is_some() && !edge_eligible[e] {
                            all_circular_eligible = false;
                        }
                    }
                }
                all_circular_eligible
            })
            .collect();

        Self {
            edge_eligible,
            face_interior_eligible,
        }
    }
}

/// Rebuild a compressed shell with policy-wrapped edge curves and surfaces.
///
/// Vertices, topology (edge indices, boundaries, orientation), and face
/// provenance are carried through unchanged; only the curve and surface
/// geometries are wrapped. Eligibility is computed once from the shell's
/// topology and carried into each wrapper, so a shared edge has a single
/// density for all incident faces (crack-free) and the angular floor applies
/// only inside topology-safe plane/cylinder/cone neighborhoods.
pub fn wrap_shell(
    shell: CompressedShell<Point3, Curve3D, Surface>,
    policy: MeshingPolicy,
) -> CompressedShell<Point3, PolicyCurve, PolicySurface> {
    let eligibility = Eligibility::compute(&shell);
    CompressedShell {
        vertices: shell.vertices,
        edges: shell
            .edges
            .into_iter()
            .enumerate()
            .map(|(e, edge)| CompressedEdge {
                vertices: edge.vertices,
                curve: PolicyCurve::new(edge.curve, policy, eligibility.edge_eligible[e]),
            })
            .collect(),
        faces: shell
            .faces
            .into_iter()
            .enumerate()
            .map(|(f, face)| CompressedFace {
                boundaries: face.boundaries,
                orientation: face.orientation,
                surface: PolicySurface::new(
                    face.surface,
                    policy,
                    eligibility.face_interior_eligible[f],
                ),
                provenance: face.provenance,
            })
            .collect(),
    }
}
