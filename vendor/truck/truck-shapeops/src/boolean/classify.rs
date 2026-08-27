//! BG-SOL-RW3-CLASSIFY — the §12 fragment classifier (the Boundary Rewrite's
//! second topology packet).
//!
//! Every fragment of a [`FragmentMesh`] is classified as inside or outside the
//! OTHER solid's closure, index-aligned, by seed-and-propagate over the parity
//! graph — not per-face ray casting. A per-fragment point-membership test
//! cannot do this correctly: a fragment that straddles nothing still needs the
//! bit, and surface points lie ON the other solid's boundary half the time.
//!
//! One certified seed per connected component (the lowest-index fragment that
//! touches a contact arc, else the lowest-index fragment whose region
//! representative resolves), bits propagated across the [`AdjacencyParity`]
//! edges, and EVERY non-tree edge verified (Same ⇒ equal, Flip ⇒ different);
//! the first violation refuses with `Refusal::Contradictory`. Coincident
//! fragments get their bits by propagation like every other fragment; the
//! [`FragmentMesh`]'s coincident pairs matter only at RW4's decision.
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

use super::split::{
    create_parameter_boundary, near_pt, point_segment_distance, region_contains,
    region_representative, AdjacencyParity, FragmentMesh, FragmentOrigin,
};
use itertools::Itertools;
use rustc_hash::FxHashMap as HashMap;
use truck_base::cgmath64::{InnerSpace, Point2, Point3, Vector3};
use truck_base::evidence::{
    Budget, Certificate, Certified, ContradictionWitness, EnvelopeCase, Margin, Method, Modulus,
    Prop, PropMap, Refusal, Truth, UnresolvedWitness,
};
use truck_evidence::Outcome;
use truck_geometry::canonical::{Curve, Surface};
use truck_geotrait::{
    BoundedCurve, ParametricCurve, ParametricSurface, ParametricSurface3D, SearchParameter,
};
use truck_meshalgo::prelude::PolylineCurve;
use truck_topology::{EdgeID, Face, Shell};

/// The number of Newton trials for a surface `search_parameter` call in the
/// classification geometry (tolerance-class, matching the splitter).
const SEARCH_TRIALS: usize = 100;

/// Dimensionless slack on cross products of unit normals and on the quadratic
/// solve coefficients (H-3): a carrier parallel to the fragment's carries no
/// arc-side information, and a zero quadratic coefficient is no crossing.
const NORMAL_SLACK: f64 = 1.0e-6; // H-3: dimensionless normal slack, not a length

/// Dimensionless slack on signed parameter-polygon areas (H-3): below this a
/// polygon is degenerate (the extrude-wall band signature).
const DEGENERATE_AREA_SLACK: f64 = 1.0e-9; // H-3: dimensionless area slack, not a length

/// Dimensionless slack on full-period parameter spans (H-3): a polygon
/// spanning at least `period − FULL_PERIOD_SLACK` is a full-period wire.
const FULL_PERIOD_SLACK: f64 = 1.0e-9; // H-3: dimensionless span slack, not a length

/// One bit per fragment (index-aligned): inside the OTHER solid's
/// closure. For coincident fragments the bit is computed but NOT used
/// by the decision — the CoincidentPair's witnesses take precedence
/// there (RW4).
#[derive(Clone, Debug)]
pub struct FragmentClassification {
    /// Whether each fragment lies inside the other solid's closure.
    pub inside_other: Vec<bool>,
}

/// Classify every fragment of `mesh` against the other solid.
///
/// Returns one bit per fragment (index-aligned with [`FragmentMesh::fragments`]).
/// A mesh whose parity graph is inconsistent refuses
/// `Contradictory(prop = FragmentInsideOther)`; a ray seed whose other solid
/// carries a non-canonical surface refuses
/// `UnsupportedEnvelope(NonCanonicalCarrier)`; an unresolvable seed refuses
/// `NumericallyUnresolved`.
pub fn classify_fragments(
    shell_a: &Shell<Point3, Curve, Surface>,
    shell_b: &Shell<Point3, Curve, Surface>,
    mesh: &FragmentMesh,
    tol: f64,
) -> Outcome<FragmentClassification> {
    let n = mesh.fragments.len();
    let adjacency = build_adjacency_list(mesh);
    let components = connected_components(&adjacency, n);

    // One seed per component, then propagate from it. The bits are collected
    // before the verification pass so the FIRST violation in `mesh.adjacency`
    // order refuses.
    let mut bits: Vec<Option<bool>> = vec![None; n];
    for comp in &components {
        let (seed, seed_bit) = find_seed(shell_a, shell_b, mesh, comp, &adjacency, tol)?;
        if let Some(slot) = bits.get_mut(seed) {
            *slot = Some(seed_bit);
        }
        let mut stack: Vec<usize> = vec![seed];
        while let Some(u) = stack.pop() {
            let u_bit = match bits.get(u).copied() {
                Some(Some(b)) => b,
                _ => continue,
            };
            let neighbors = match adjacency.get(u) {
                Some(neighbors) => neighbors.clone(),
                None => continue,
            };
            for (v, parity) in neighbors {
                if bits.get(v).copied() == Some(None) {
                    let v_bit = u_bit ^ (parity == AdjacencyParity::Flip);
                    if let Some(slot) = bits.get_mut(v) {
                        *slot = Some(v_bit);
                    }
                    stack.push(v);
                }
            }
        }
    }

    // Verification: EVERY adjacency edge holds (tree edges included, checked
    // anyway — it is cheaper than tracking the tree). The first violation in
    // `mesh.adjacency` order refuses.
    for adj in &mesh.adjacency {
        let lhs_bit = bits.get(adj.lhs).copied().flatten();
        let rhs_bit = bits.get(adj.rhs).copied().flatten();
        let implied_rhs = lhs_bit.map(|b| b ^ (adj.parity == AdjacencyParity::Flip));
        let consistent = match (rhs_bit, implied_rhs) {
            (Some(rhs), Some(implied)) => rhs == implied,
            _ => true,
        };
        if !consistent {
            return Err(Refusal::Contradictory(ContradictionWitness {
                prop: Prop::FragmentInsideOther,
                left: truth_of(rhs_bit),
                right: truth_of(implied_rhs),
            }));
        }
    }

    let inside_other: Vec<bool> = bits.iter().map(|bit| bit.unwrap_or(false)).collect();
    Ok(Certified::new(
        FragmentClassification { inside_other },
        Certificate {
            props: PropMap::new(),
            method: Method::Float,
            budget_left: Budget::new(0, 0, 0),
            margin: Margin::UNBOUNDED,
            modulus: Modulus::Unbounded,
        },
    ))
}

/// The `Truth` of an optional classification bit.
fn truth_of(bit: Option<bool>) -> Truth {
    match bit {
        Some(true) => Truth::True,
        Some(false) => Truth::False,
        None => Truth::Unknown,
    }
}

/// The undirected adjacency lists, one per fragment index.
fn build_adjacency_list(mesh: &FragmentMesh) -> Vec<Vec<(usize, AdjacencyParity)>> {
    let mut adjacency: Vec<Vec<(usize, AdjacencyParity)>> =
        (0..mesh.fragments.len()).map(|_| Vec::new()).collect();
    for adj in &mesh.adjacency {
        if let Some(list) = adjacency.get_mut(adj.lhs) {
            list.push((adj.rhs, adj.parity));
        }
        if let Some(list) = adjacency.get_mut(adj.rhs) {
            list.push((adj.lhs, adj.parity));
        }
    }
    adjacency
}

/// The connected components over the adjacency, each sorted ascending, in
/// order of the component's lowest fragment index.
fn connected_components(adjacency: &[Vec<(usize, AdjacencyParity)>], n: usize) -> Vec<Vec<usize>> {
    let mut components: Vec<Vec<usize>> = Vec::new();
    let mut assigned: Vec<bool> = vec![false; n];
    for start in 0..n {
        if assigned.get(start).copied() == Some(true) {
            continue;
        }
        let mut comp: Vec<usize> = Vec::new();
        let mut stack: Vec<usize> = vec![start];
        if let Some(slot) = assigned.get_mut(start) {
            *slot = true;
        }
        while let Some(u) = stack.pop() {
            comp.push(u);
            if let Some(neighbors) = adjacency.get(u) {
                for &(v, _parity) in neighbors {
                    if assigned.get(v).copied() == Some(false) {
                        if let Some(slot) = assigned.get_mut(v) {
                            *slot = true;
                        }
                        stack.push(v);
                    }
                }
            }
        }
        comp.sort();
        components.push(comp);
    }
    components.sort_by(|a, b| match (a.first(), b.first()) {
        (Some(a0), Some(b0)) => a0.cmp(b0),
        _ => std::cmp::Ordering::Equal,
    });
    components
}

/// The other solid's shell, opposite to `origin`.
fn other_shell<'a>(
    shell_a: &'a Shell<Point3, Curve, Surface>,
    shell_b: &'a Shell<Point3, Curve, Surface>,
    origin: FragmentOrigin,
) -> &'a Shell<Point3, Curve, Surface> {
    match origin {
        FragmentOrigin::A { .. } => shell_b,
        FragmentOrigin::B { .. } => shell_a,
    }
}

/// The one certified seed for a component and its bit.
///
/// Rule (a): if the component has any Flip adjacency, the seed is the
/// component's lowest-index fragment touching one, and the bit comes from the
/// arc-side test. Rule (b): otherwise the seed is the lowest-index fragment
/// whose region representative resolves, and the bit comes from the ray-parity
/// test (on-boundary pre-screen then the deterministic direction table).
fn find_seed(
    shell_a: &Shell<Point3, Curve, Surface>,
    shell_b: &Shell<Point3, Curve, Surface>,
    mesh: &FragmentMesh,
    comp: &[usize],
    adjacency: &[Vec<(usize, AdjacencyParity)>],
    tol: f64,
) -> Result<(usize, bool), Refusal> {
    let flip_touching = comp.iter().copied().find(|&u| {
        adjacency
            .get(u)
            .is_some_and(|neighbors| neighbors.iter().any(|&(_, p)| p == AdjacencyParity::Flip))
    });
    if let Some(seed) = flip_touching {
        let bit = arc_side_seed(shell_a, shell_b, mesh, seed, tol)?;
        return Ok((seed, bit));
    }

    let first = *comp.first().ok_or_else(numerically_unresolved)?;
    let first_origin = mesh
        .fragments
        .get(first)
        .ok_or_else(numerically_unresolved)?
        .origin;
    let other = other_shell(shell_a, shell_b, first_origin);
    require_canonical_carriers(other)?;
    for &u in comp {
        let fragment = mesh.fragments.get(u).ok_or_else(numerically_unresolved)?;
        let face = &fragment.face;
        let Some(polys) = face_parameter_polygons(face, tol) else {
            continue;
        };
        let Some(rep) = region_representative(&polys, tol) else {
            continue;
        };
        let surface = face.surface();
        let rep_3d = surface.subs(rep.x, rep.y);
        let bit = ray_seed(rep_3d, other, tol)?;
        return Ok((u, bit));
    }
    Err(numerically_unresolved())
}

// ---------------------------------------------------------------------------
// the arc-side seed (rule a)
// ---------------------------------------------------------------------------

/// The arc-side bit of the component's arc-touching seed fragment.
fn arc_side_seed(
    shell_a: &Shell<Point3, Curve, Surface>,
    shell_b: &Shell<Point3, Curve, Surface>,
    mesh: &FragmentMesh,
    seed: usize,
    tol: f64,
) -> Result<bool, Refusal> {
    let fragment = mesh
        .fragments
        .get(seed)
        .ok_or_else(numerically_unresolved)?;
    let other = other_shell(shell_a, shell_b, fragment.origin);
    let s_f = wire_orientation_sign(&fragment.face, tol)?;
    let face = &fragment.face;
    let surface = face.surface();
    let flipped = !face.orientation();
    let seed_ids = boundary_edge_ids(face);
    for adj in &mesh.adjacency {
        if adj.parity != AdjacencyParity::Flip {
            continue;
        }
        if adj.lhs != seed && adj.rhs != seed {
            continue;
        }
        let other_idx = if adj.lhs == seed { adj.rhs } else { adj.lhs };
        let other_fragment = mesh
            .fragments
            .get(other_idx)
            .ok_or_else(numerically_unresolved)?;
        let other_ids = boundary_edge_ids(&other_fragment.face);
        for edge_id in &seed_ids {
            if !other_ids.contains(edge_id) {
                continue;
            }
            if let Some(bit) = arc_side_sample(face, &surface, flipped, s_f, other, *edge_id, tol) {
                return Ok(bit);
            }
        }
    }
    Err(numerically_unresolved())
}

/// The edge ids of a fragment face's effective boundary wires.
fn boundary_edge_ids(face: &Face<Point3, Curve, Surface>) -> Vec<EdgeID<Curve>> {
    let mut out = Vec::new();
    for wire in face.boundaries() {
        for edge in wire.edge_iter() {
            out.push(edge.id());
        }
    }
    out
}

/// The wire-orientation sign `s_F` of a fragment: `+1` iff the signed
/// parameter-polygon area of the FIRST effective boundary wire has the same
/// sign as the face's orientation flag. A degenerate first wire
/// (`A == 0.0`) refuses (defensive; a Flip adjacency implies proper regions).
fn wire_orientation_sign(face: &Face<Point3, Curve, Surface>, tol: f64) -> Result<f64, Refusal> {
    let first_wire = face
        .boundaries()
        .into_iter()
        .next()
        .ok_or_else(numerically_unresolved)?;
    let mut cache: HashMap<EdgeID<Curve>, PolylineCurve<Point3>> = HashMap::default();
    let poly = create_parameter_boundary(face, &first_wire, &mut cache, tol)
        .ok_or_else(numerically_unresolved)?;
    let area = poly.area();
    if area == 0.0 {
        return Err(numerically_unresolved());
    }
    Ok(if (area > 0.0) == face.orientation() {
        1.0
    } else {
        -1.0
    })
}

/// The arc-side bit from one shared edge of the seed fragment: sample the
/// edge's curve at `[0.5, 0.25, 0.75]` of its range; the first decisive
/// sample wins. `None` is not decisive (continue with the next shared edge /
/// adjacency).
fn arc_side_sample(
    face: &Face<Point3, Curve, Surface>,
    surface: &Surface,
    flipped: bool,
    s_f: f64,
    other: &Shell<Point3, Curve, Surface>,
    edge_id: EdgeID<Curve>,
    tol: f64,
) -> Option<bool> {
    let boundaries = face.boundaries();
    let edge_use = boundaries
        .iter()
        .flat_map(|wire| wire.edge_iter())
        .find(|edge| edge.id() == edge_id)?;
    let curve = edge_use.curve();
    let (t0, t1) = curve.range_tuple();
    for s in [0.5, 0.25, 0.75] {
        let t = t0 + (t1 - t0) * s;
        let p = curve.subs(t);
        // The effective traversal direction of the shared edge in the seed
        // fragment's boundary wire.
        let mut der = curve.der(t);
        if !edge_use.orientation() {
            der = -der;
        }
        let uv = surface.search_parameter(p, None, SEARCH_TRIALS)?;
        let mut n_f = surface.normal(uv.0, uv.1);
        if flipped {
            n_f = -n_f;
        }
        if let Some(bit) = decisive_bit(s_f, n_f, der, p, other, tol) {
            return Some(bit);
        }
    }
    None
}

/// The booked sign convention on the other solid's faces whose carrier
/// contains the sample point: `val = s_F · (n_F × der) · n_B`; a candidate
/// with `|val| <= NORMAL_SLACK` (its carrier parallel to the fragment's) is
/// uninformative and skipped. The sample is decisive iff at least one
/// candidate remains and all remaining candidates agree in sign; the bit is
/// `val < 0.0` (the `(n_F × der) · n_B < 0 ⇒ INSIDE` convention).
fn decisive_bit(
    s_f: f64,
    n_f: Vector3,
    der: Vector3,
    p: Point3,
    other: &Shell<Point3, Curve, Surface>,
    tol: f64,
) -> Option<bool> {
    let mut lo = f64::INFINITY;
    let mut hi = f64::NEG_INFINITY;
    let mut decided = false;
    for face in other.face_iter() {
        let surface = face.surface();
        let Some(uv) = surface.search_parameter(p, None, SEARCH_TRIALS) else {
            continue;
        };
        if !near_pt(surface.subs(uv.0, uv.1), p, tol) {
            continue;
        }
        let mut n_b = surface.normal(uv.0, uv.1);
        if !face.orientation() {
            n_b = -n_b;
        }
        let val = s_f * n_f.cross(der).dot(n_b);
        if val.abs() <= NORMAL_SLACK {
            continue;
        }
        lo = lo.min(val);
        hi = hi.max(val);
        decided = true;
    }
    if !decided || (lo < 0.0 && hi > 0.0) {
        return None;
    }
    Some(hi < 0.0)
}

// ---------------------------------------------------------------------------
// the ray-parity seed (rule b)
// ---------------------------------------------------------------------------

/// The fourteen deterministic ray-seed directions: the six axials
/// `+ẑ, +x̂, +ŷ, −ẑ, −x̂, −ŷ` then the eight body diagonals `(±1, ±1, ±1)/√3`
/// (the diagonal scale is the named reciprocal `1 / √3`).
fn ray_directions() -> [Vector3; 14] {
    let s = 1.0 / 3.0_f64.sqrt();
    [
        Vector3::new(0.0, 0.0, 1.0),
        Vector3::new(1.0, 0.0, 0.0),
        Vector3::new(0.0, 1.0, 0.0),
        Vector3::new(0.0, 0.0, -1.0),
        Vector3::new(-1.0, 0.0, 0.0),
        Vector3::new(0.0, -1.0, 0.0),
        Vector3::new(s, s, s),
        Vector3::new(s, s, -s),
        Vector3::new(s, -s, s),
        Vector3::new(s, -s, -s),
        Vector3::new(-s, s, s),
        Vector3::new(-s, s, -s),
        Vector3::new(-s, -s, s),
        Vector3::new(-s, -s, -s),
    ]
}

/// The ray-parity bit for a contact-free component: the on-boundary
/// containment pre-screen first, then the deterministic direction table with
/// signed winding. A direction with any Boundary-classified crossing is
/// ambiguous; an exhausted table refuses `NumericallyUnresolved`.
fn ray_seed(
    rep_3d: Point3,
    other: &Shell<Point3, Curve, Surface>,
    tol: f64,
) -> Result<bool, Refusal> {
    for face in other.face_iter() {
        let surface = face.surface();
        let Some(uv) = surface.search_parameter(rep_3d, None, SEARCH_TRIALS) else {
            continue;
        };
        if !near_pt(surface.subs(uv.0, uv.1), rep_3d, tol) {
            continue;
        }
        let region = classify_region(face, Point2::new(uv.0, uv.1), tol)
            .ok_or_else(numerically_unresolved)?;
        match region {
            Region::Inside => return Ok(true),
            Region::Boundary => return Err(numerically_unresolved()),
            Region::Outside => {}
        }
    }
    for d in ray_directions() {
        let mut winding = 0i32;
        let mut ambiguous = false;
        for face in other.face_iter() {
            let surface = face.surface();
            for (t, q) in surface_ray_crossings(&surface, rep_3d, d) {
                if t <= tol {
                    continue;
                }
                let Some(uv) = surface.search_parameter(q, None, SEARCH_TRIALS) else {
                    continue;
                };
                if !near_pt(surface.subs(uv.0, uv.1), q, tol) {
                    continue;
                }
                match classify_region(face, Point2::new(uv.0, uv.1), tol) {
                    Some(Region::Inside) => {
                        let mut n = surface.normal(uv.0, uv.1);
                        if !face.orientation() {
                            n = -n;
                        }
                        // The extrude.rs `point_in_solid` sign convention: an
                        // entering crossing (`d·n_eff < 0`) adds +1, an exit −1.
                        if d.dot(n) < 0.0 {
                            winding += 1;
                        } else {
                            winding -= 1;
                        }
                    }
                    Some(Region::Boundary) => {
                        ambiguous = true;
                        break;
                    }
                    Some(Region::Outside) => {}
                    None => {
                        ambiguous = true;
                        break;
                    }
                }
            }
            if ambiguous {
                break;
            }
        }
        if !ambiguous {
            return Ok(winding != 0);
        }
    }
    Err(numerically_unresolved())
}

/// Whether every face of `shell` is one of the four canonical carriers the
/// ray solve implements; any other arm refuses `NonCanonicalCarrier`.
fn require_canonical_carriers(shell: &Shell<Point3, Curve, Surface>) -> Result<(), Refusal> {
    for face in shell.face_iter() {
        let canonical = match face.surface() {
            Surface::Plane(_) | Surface::Cylinder(_) | Surface::Cone(_) | Surface::Sphere(_) => {
                true
            }
            Surface::Torus(_)
            | Surface::RevolutedCurve(_)
            | Surface::ExtrudedCurve(_)
            | Surface::BSplineSurface(_)
            | Surface::NurbsSurface(_)
            | Surface::Processor(_) => false,
        };
        if !canonical {
            return Err(Refusal::UnsupportedEnvelope(
                EnvelopeCase::NonCanonicalCarrier,
            ));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// the analytic ray×carrier solves
// ---------------------------------------------------------------------------

/// The ray-carrier crossings of `p + t·d` with an analytic surface. Only the
/// four canonical carriers have solves; the other arms return no crossings
/// (unreachable here — `require_canonical_carriers` refused them first).
fn surface_ray_crossings(surface: &Surface, p: Point3, d: Vector3) -> Vec<(f64, Point3)> {
    match surface {
        Surface::Plane(plane) => {
            let n = plane.normal();
            let denom = d.dot(n);
            if denom.abs() <= NORMAL_SLACK {
                return Vec::new();
            }
            let t = (plane.origin() - p).dot(n) / denom;
            vec![(t, p + d * t)]
        }
        Surface::Cylinder(cyl) => {
            let c = cyl.center();
            let px = p.x - c.x;
            let py = p.y - c.y;
            let dx = d.x;
            let dy = d.y;
            // The quadratic over the xy-components relative to the center
            // (extrude.rs's `face_ray_crossings` verbatim).
            let a = dx * dx + dy * dy;
            if a <= NORMAL_SLACK {
                return Vec::new();
            }
            let b = 2.0 * (px * dx + py * dy);
            let cc = px * px + py * py - cyl.radius() * cyl.radius();
            let disc = b * b - 4.0 * a * cc;
            if disc < 0.0 {
                return Vec::new();
            }
            let sq = disc.sqrt();
            let t0 = (-b - sq) / (2.0 * a);
            let t1 = (-b + sq) / (2.0 * a);
            let mut out = Vec::new();
            out.push((t0, p + d * t0));
            if t1 != t0 {
                out.push((t1, p + d * t1));
            }
            out
        }
        Surface::Sphere(sphere) => {
            let c = sphere.center();
            let e = p - c;
            // `|p + t·d − c|² = r²`, both roots.
            let a = d.dot(d);
            if a <= NORMAL_SLACK {
                return Vec::new();
            }
            let b = 2.0 * e.dot(d);
            let cc = e.dot(e) - sphere.radius() * sphere.radius();
            let disc = b * b - 4.0 * a * cc;
            if disc < 0.0 {
                return Vec::new();
            }
            let sq = disc.sqrt();
            let t0 = (-b - sq) / (2.0 * a);
            let t1 = (-b + sq) / (2.0 * a);
            let mut out = Vec::new();
            out.push((t0, p + d * t0));
            if t1 != t0 {
                out.push((t1, p + d * t1));
            }
            out
        }
        Surface::Cone(cone) => {
            let k = cone.half_angle().tan();
            let e = p - cone.apex();
            let dx = d.x;
            let dy = d.y;
            let dz = d.z;
            let ex = e.x;
            let ey = e.y;
            let ez = e.z;
            // The double-nappe quadratic: `a·t² + b·t + c = 0` with
            // `a = dx² + dy² − k²·dz²`, `b = 2(ex·dx + ey·dy − k²·ez·dz)`,
            // `c = ex² + ey² − k²·ez²`; the region check filters nappes.
            let a = dx * dx + dy * dy - k * k * dz * dz;
            if a.abs() <= NORMAL_SLACK {
                return Vec::new();
            }
            let b = 2.0 * (ex * dx + ey * dy - k * k * ez * dz);
            let cc = ex * ex + ey * ey - k * k * ez * ez;
            let disc = b * b - 4.0 * a * cc;
            if disc < 0.0 {
                return Vec::new();
            }
            let sq = disc.sqrt();
            let t0 = (-b - sq) / (2.0 * a);
            let t1 = (-b + sq) / (2.0 * a);
            let mut out = Vec::new();
            out.push((t0, p + d * t0));
            if t1 != t0 {
                out.push((t1, p + d * t1));
            }
            out
        }
        Surface::Torus(_)
        | Surface::RevolutedCurve(_)
        | Surface::ExtrudedCurve(_)
        | Surface::BSplineSurface(_)
        | Surface::NurbsSurface(_)
        | Surface::Processor(_) => Vec::new(),
    }
}

// ---------------------------------------------------------------------------
// the crossing/point region classifier (the trichotomy)
// ---------------------------------------------------------------------------

/// The trichotomous classification of a query `(u, v)` against a face's
/// trimmed region.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Region {
    /// Strictly inside the face's trimmed region.
    Inside,
    /// Within `tol` of the face's trimmed boundary.
    Boundary,
    /// On the face's carrier but outside the trimmed region.
    Outside,
}

/// The region classifier (decision 6): planes use the polygon rule; periodic
/// carriers with degenerate full-period wire polygons use the band rule; the
/// sphere swaps the roles (the v-period polygons ⇒ the u-band rule). Any other
/// arm is unreachable (the ray seed refused it first) and returns `None`.
fn classify_region(face: &Face<Point3, Curve, Surface>, uv: Point2, tol: f64) -> Option<Region> {
    let surface = face.surface();
    let polys = face_parameter_polygons(face, tol)?;
    match &surface {
        Surface::Plane(_) => Some(polygon_rule(&polys, uv, surface.u_period(), tol)),
        Surface::Cylinder(_) | Surface::Cone(_) => {
            if band_form(&polys, surface.u_period(), 0) {
                Some(band_rule(&polys, uv, 1, tol))
            } else {
                Some(polygon_rule(&polys, uv, surface.u_period(), tol))
            }
        }
        Surface::Sphere(_) => {
            if band_form(&polys, surface.v_period(), 1) {
                Some(band_rule(&polys, uv, 0, tol))
            } else {
                Some(polygon_rule(&polys, uv, surface.u_period(), tol))
            }
        }
        Surface::Torus(_)
        | Surface::RevolutedCurve(_)
        | Surface::ExtrudedCurve(_)
        | Surface::BSplineSurface(_)
        | Surface::NurbsSurface(_)
        | Surface::Processor(_) => None,
    }
}

/// The wire parameter polygons of a face's absolute boundary wires, in wire
/// order.
fn face_parameter_polygons(
    face: &Face<Point3, Curve, Surface>,
    tol: f64,
) -> Option<Vec<PolylineCurve<Point2>>> {
    let mut cache: HashMap<EdgeID<Curve>, PolylineCurve<Point3>> = HashMap::default();
    let mut out = Vec::new();
    for wire in face.absolute_boundaries() {
        out.push(create_parameter_boundary(face, wire, &mut cache, tol)?);
    }
    Some(out)
}

/// The polygon rule: within `tol` of a boundary segment is `Boundary`, else
/// `region_contains` decides `Inside` vs `Outside`.
fn polygon_rule(
    polys: &[PolylineCurve<Point2>],
    uv: Point2,
    u_period: Option<f64>,
    tol: f64,
) -> Region {
    if boundary_distance(polys, uv) <= tol {
        Region::Boundary
    } else if region_contains(polys, uv, u_period) {
        Region::Inside
    } else {
        Region::Outside
    }
}

/// The minimum point-to-segment distance over all wire-polygon segments
/// (including each polygon's closing segment).
fn boundary_distance(polys: &[PolylineCurve<Point2>], uv: Point2) -> f64 {
    polys
        .iter()
        .flat_map(|poly| {
            poly.iter()
                .circular_tuple_windows()
                .map(move |(a, b)| point_segment_distance(uv, *a, *b))
        })
        .fold(f64::INFINITY, f64::min)
}

/// The band-form test: every polygon degenerate (`|area| <= DEGENERATE_AREA_SLACK`)
/// AND together they span a full period in the periodic coordinate — the
/// extrude-wall signature (cut or uncut). `coordinate` is 0 for u (x) and 1
/// for v (y).
fn band_form(polys: &[PolylineCurve<Point2>], period: Option<f64>, coordinate: usize) -> bool {
    let Some(period) = period else {
        return false;
    };
    if polys.is_empty() {
        return false;
    }
    if !polys
        .iter()
        .all(|poly| poly.area().abs() <= DEGENERATE_AREA_SLACK)
    {
        return false;
    }
    let mut lo = f64::INFINITY;
    let mut hi = f64::NEG_INFINITY;
    for poly in polys {
        for pt in poly.iter() {
            let c = if coordinate == 0 { pt.x } else { pt.y };
            lo = lo.min(c);
            hi = hi.max(c);
        }
    }
    hi - lo >= period - FULL_PERIOD_SLACK
}

/// The band rule: `lo`/`hi` are the min/max of the band coordinate over all
/// polygon points; strictly between (with a `tol` margin) is `Inside`, at the
/// margin is `Boundary`, else `Outside`. `coordinate` is 0 for u (x) and 1 for
/// v (y).
fn band_rule(polys: &[PolylineCurve<Point2>], uv: Point2, coordinate: usize, tol: f64) -> Region {
    let mut lo = f64::INFINITY;
    let mut hi = f64::NEG_INFINITY;
    for poly in polys {
        for pt in poly.iter() {
            let c = if coordinate == 0 { pt.x } else { pt.y };
            lo = lo.min(c);
            hi = hi.max(c);
        }
    }
    let c = if coordinate == 0 { uv.x } else { uv.y };
    if lo + tol < c && c < hi - tol {
        Region::Inside
    } else if (c - lo).abs() <= tol || (c - hi).abs() <= tol {
        Region::Boundary
    } else {
        Region::Outside
    }
}

/// The numerically-unresolved refusal for a seed or classification that cannot
/// be certified.
fn numerically_unresolved() -> Refusal {
    Refusal::NumericallyUnresolved {
        spent: Budget::new(0, 0, 0),
        witness: UnresolvedWitness::UncertifiedContainment,
    }
}

#[cfg(test)]
// Test-only allow: H-1 bans unwrap/expect/panic on paths reachable from
// untrusted geometry. Unit-test assertions on hand-built dyadic witnesses are
// not such a path; the unwraps and indexing below cannot fire for the values
// constructed.
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]
mod tests {
    use super::super::split::{
        split_fragments, AdjacencyParity, ContactEvent, FragmentMesh, SolidRef, StratumRef,
    };
    use super::*;
    use std::f64::consts::{FRAC_PI_4, TAU};
    use truck_base::cgmath64::{Matrix4, Vector4};
    use truck_base::contact::{ContactDimension, ContactEventKind};
    use truck_base::evidence::{Prop, Refusal};
    use truck_evidence::analytic::{AnalyticIntersection, ExactCurve};
    use truck_evidence::contact::{ContactLocus, ContactRecord};
    use truck_geometry::arrange::arrange;
    use truck_geometry::arrange::Arrangement;
    use truck_geometry::prelude::*;
    use truck_modeling::extrude::extrude_profile;
    use truck_topology::{Edge, Vertex, Wire};
    /// The insertion tolerance class for the splitter calls (H-3: dimensionless
    /// relative to the unit-scale witnesses; dyadic geometry decides exactly).
    const TOL: f64 = 1.0e-2; // H-3: tolerance class for insertion geometry

    /// A placed full-period circle at `center` with radius `r`.
    fn placed_circle(
        center: Point3,
        r: f64,
    ) -> Processor<TrimmedCurve<UnitCircle<Point3>>, Matrix4> {
        Processor::with_transform(
            TrimmedCurve::new(UnitCircle::<Point3>::new(), (0.0, TAU)),
            Matrix4 {
                x: Vector4::new(r, 0.0, 0.0, 0.0),
                y: Vector4::new(0.0, r, 0.0, 0.0),
                z: Vector4::new(0.0, 0.0, 1.0, 0.0),
                w: Vector4::new(center.x, center.y, center.z, 1.0),
            },
        )
    }

    /// The 4x4 block profile: four `Curve::Line`s, CCW.
    fn block_profile() -> (Vec<Curve>, Arrangement) {
        let profile = vec![
            Curve::Line(Line(Point3::new(0.0, 0.0, 0.0), Point3::new(4.0, 0.0, 0.0))),
            Curve::Line(Line(Point3::new(4.0, 0.0, 0.0), Point3::new(4.0, 4.0, 0.0))),
            Curve::Line(Line(Point3::new(4.0, 4.0, 0.0), Point3::new(0.0, 4.0, 0.0))),
            Curve::Line(Line(Point3::new(0.0, 4.0, 0.0), Point3::new(0.0, 0.0, 0.0))),
        ];
        let ok = arrange(&profile, None).unwrap();
        (profile, ok.value)
    }

    /// A pure-disk profile: one full circle of radius `r` at `center`.
    fn disk_profile(center: Point2, r: f64) -> (Vec<Curve>, Arrangement) {
        let circle = Curve::Circle(placed_circle(Point3::new(center.x, center.y, 0.0), r));
        let profile = vec![circle];
        let ok = arrange(&profile, None).unwrap();
        (profile, ok.value)
    }

    /// The shell of the `height`-extrude of a profile.
    fn extrude_shell(
        profile: &[Curve],
        arr: &Arrangement,
        height: f64,
    ) -> Shell<Point3, Curve, Surface> {
        let solid = extrude_profile(profile, arr, height).unwrap().value;
        solid.boundaries().first().unwrap().clone()
    }

    /// The index of the orientation-true `Plane` face whose corner sits at z.
    fn plane_face_at_z(shell: &Shell<Point3, Curve, Surface>, z: f64) -> usize {
        shell
            .face_iter()
            .enumerate()
            .find(|(_, face)| {
                matches!(face.surface(), Surface::Plane(_))
                    && (face.surface().subs(0.0, 0.0).z - z).abs() < TOL
            })
            .map(|(i, _)| i)
            .unwrap()
    }

    /// The index of the `Cylinder` face.
    fn cylinder_face(shell: &Shell<Point3, Curve, Surface>) -> usize {
        shell
            .face_iter()
            .enumerate()
            .find(|(_, face)| matches!(face.surface(), Surface::Cylinder(_)))
            .map(|(i, _)| i)
            .unwrap()
    }

    /// The flat edge index (in `face.absolute_boundaries()` wire-by-wire order)
    /// of the edge whose curve's midpoint sits at z.
    fn flat_edge_at_z(shell: &Shell<Point3, Curve, Surface>, face_idx: usize, z: f64) -> usize {
        let face = shell.get(face_idx).unwrap();
        let mut flat = 0usize;
        for wire in face.absolute_boundaries() {
            for edge in wire.edge_iter() {
                let curve = edge.curve();
                let (t0, t1) = curve.range_tuple();
                let mid = curve.subs((t0 + t1) * 0.5);
                if (mid.z - z).abs() < TOL {
                    return flat;
                }
                flat += 1;
            }
        }
        unreachable!("no edge at z = {z}")
    }

    /// The fragment indices whose origin is `(solid, parent)`.
    fn fragments_of_origin(mesh: &FragmentMesh, solid: SolidRef, parent: usize) -> Vec<usize> {
        mesh.fragments
            .iter()
            .enumerate()
            .filter(|(_, fragment)| match (fragment.origin, solid) {
                (FragmentOrigin::A { parent: p }, SolidRef::A)
                | (FragmentOrigin::B { parent: p }, SolidRef::B) => p == parent,
                _ => false,
            })
            .map(|(i, _)| i)
            .collect()
    }

    /// The count of edges in the i-th wire of a fragment face.
    fn wire_edge_counts(mesh: &FragmentMesh, idx: usize) -> Vec<usize> {
        mesh.fragments[idx]
            .face
            .absolute_boundaries()
            .iter()
            .map(|wire| wire.len())
            .collect()
    }

    /// A contact event from its record and two strata.
    fn ev(record: ContactRecord, lhs: StratumRef, rhs: StratumRef) -> ContactEvent {
        ContactEvent { record, lhs, rhs }
    }

    /// The `{Arc1, Transverse, Analytic(Curve(exact))}` record.
    fn ff_curve_record(exact: ExactCurve) -> ContactRecord {
        ContactRecord {
            dimension: ContactDimension::Arc1,
            kind: ContactEventKind::Transverse,
            locus: ContactLocus::Analytic(AnalyticIntersection::Curve(exact)),
        }
    }

    /// A hand-built raised disk solid: the circle self-loop edges are SHARED
    /// between the caps and the wall (each appears in exactly two faces with
    /// opposite orientations — that closes the shell; the BG-TOL-001-MESHALGO
    /// precedent).
    fn raised_disk(center: Point2, r: f64, z_lo: f64, z_hi: f64) -> Shell<Point3, Curve, Surface> {
        let bottom_center = Point3::new(center.x, center.y, z_lo);
        let top_center = Point3::new(center.x, center.y, z_hi);
        let bottom_circle = placed_circle(bottom_center, r);
        let top_circle = placed_circle(top_center, r);
        let v0 = Vertex::new(bottom_circle.subs(0.0));
        let v1 = Vertex::new(top_circle.subs(0.0));
        let bottom_edge = Edge::new_unchecked(&v0, &v0, Curve::Circle(bottom_circle));
        let top_edge = Edge::new_unchecked(&v1, &v1, Curve::Circle(top_circle));

        let bottom_surface = Surface::Plane(Plane::new(
            Point3::new(0.0, 0.0, z_lo),
            Point3::new(1.0, 0.0, z_lo),
            Point3::new(0.0, 1.0, z_lo),
        ));
        let mut bottom_cap =
            Face::try_new(vec![Wire::from(vec![bottom_edge.clone()])], bottom_surface).unwrap();
        bottom_cap.invert();

        let top_surface = Surface::Plane(Plane::new(
            Point3::new(0.0, 0.0, z_hi),
            Point3::new(1.0, 0.0, z_hi),
            Point3::new(0.0, 1.0, z_hi),
        ));
        let top_cap = Face::try_new(vec![Wire::from(vec![top_edge.clone()])], top_surface).unwrap();

        let cyl = Cylinder::new(Point3::new(center.x, center.y, 0.0), r)
            .unwrap()
            .value;
        let wall = Face::try_new(
            vec![
                Wire::from(vec![bottom_edge]),
                Wire::from(vec![top_edge.inverse()]),
            ],
            Surface::Cylinder(cyl),
        )
        .unwrap();

        vec![bottom_cap, top_cap, wall].into()
    }

    /// The index of a's top face by its wire structure plus the bit asserted
    /// by structure (helper used by the flagship test).
    fn flagship_top_bits(
        mesh: &FragmentMesh,
        classification: &FragmentClassification,
        top_a: usize,
    ) -> (usize, bool, usize, bool) {
        let mut annulus = None;
        let mut disk = None;
        for idx in fragments_of_origin(mesh, SolidRef::A, top_a) {
            match wire_edge_counts(mesh, idx).as_slice() {
                [2] => disk = Some(idx),
                [4, 2] => annulus = Some(idx),
                other => unreachable!("unexpected top-face wire structure: {other:?}"),
            }
        }
        let annulus = annulus.unwrap();
        let disk = disk.unwrap();
        (
            annulus,
            classification.inside_other[annulus],
            disk,
            classification.inside_other[disk],
        )
    }

    // ---------------------------------------------------------------------------
    // Test 1: the flagship.
    // ---------------------------------------------------------------------------

    #[test]
    fn classify_flagship_bits_are_exact() {
        // a = the 4x4 block extrude (6 faces: bottom, top, 4 sides).
        let (profile_a, arr_a) = block_profile();
        let shell_a = extrude_shell(&profile_a, &arr_a, 2.0);
        // b = the disk extrude at (2, 2) r=1 (3 faces: bottom cap, top cap, wall).
        let (profile_b, arr_b) = disk_profile(Point2::new(2.0, 2.0), 1.0);
        let shell_b = extrude_shell(&profile_b, &arr_b, 2.0);

        // The FULL event set of `split_flagship_top_face_by_ff_circle`: the FF
        // circle, the FE BoundedCurve sewing oracle, and the Region2 record.
        let top_a = plane_face_at_z(&shell_a, 2.0);
        let wall_b = cylinder_face(&shell_b);
        let cap_b = plane_face_at_z(&shell_b, 2.0);
        let rim_edge = flat_edge_at_z(&shell_b, wall_b, 2.0);
        let exact = ExactCurve::Circle(placed_circle(Point3::new(2.0, 2.0, 2.0), 1.0));

        let ff = ev(
            ff_curve_record(exact.clone()),
            StratumRef::Face {
                solid: SolidRef::A,
                index: top_a,
            },
            StratumRef::Face {
                solid: SolidRef::B,
                index: wall_b,
            },
        );
        let fe = ev(
            ContactRecord {
                dimension: ContactDimension::Arc1,
                kind: ContactEventKind::CoincidentInterval,
                locus: ContactLocus::BoundedCurve {
                    curve: exact,
                    t_range: (0.0, TAU),
                },
            },
            StratumRef::Face {
                solid: SolidRef::A,
                index: top_a,
            },
            StratumRef::Edge {
                solid: SolidRef::B,
                face: wall_b,
                edge: rim_edge,
            },
        );
        let r2 = ev(
            ContactRecord {
                dimension: ContactDimension::Region2,
                kind: ContactEventKind::CoincidentInterval,
                locus: ContactLocus::Coincident,
            },
            StratumRef::Face {
                solid: SolidRef::A,
                index: top_a,
            },
            StratumRef::Face {
                solid: SolidRef::B,
                index: cap_b,
            },
        );

        let mesh = split_fragments(&shell_a, &shell_b, &[ff, fe, r2], TOL)
            .unwrap()
            .value;
        assert_eq!(mesh.fragments.len(), 10);

        let classification = classify_fragments(&shell_a, &shell_b, &mesh, TOL)
            .unwrap()
            .value;
        assert_eq!(classification.inside_other.len(), 10);

        // The measured flagship bit vector, in fragment order:
        //   a's bottom F, annulus F, disk T, four sides F x4;
        //   b's bottom cap T, top cap T, wall T.
        assert_eq!(
            classification.inside_other,
            vec![false, false, true, false, false, false, false, true, true, true]
        );

        // By structure: a's bottom is outside, the annulus outside, the disk
        // inside, each side outside; b's three faces inside.
        let bottom_a = fragments_of_origin(&mesh, SolidRef::A, plane_face_at_z(&shell_a, 0.0));
        assert_eq!(bottom_a.len(), 1);
        assert!(!classification.inside_other[bottom_a[0]]);
        let (annulus, annulus_bit, disk, disk_bit) =
            flagship_top_bits(&mesh, &classification, top_a);
        assert_ne!(annulus, disk);
        assert!(!annulus_bit, "the annulus is outside the disk's column");
        assert!(disk_bit, "the disk is inside the disk's column");
        for side in 0..4 {
            let idx = 2 + side;
            let frags = fragments_of_origin(&mesh, SolidRef::A, idx);
            assert_eq!(frags.len(), 1);
            assert!(!classification.inside_other[frags[0]]);
        }
        for b_idx in fragments_of_origin(&mesh, SolidRef::B, cap_b) {
            assert!(classification.inside_other[b_idx]);
        }
        for b_idx in fragments_of_origin(&mesh, SolidRef::B, wall_b) {
            assert!(classification.inside_other[b_idx]);
        }
        for b_idx in fragments_of_origin(&mesh, SolidRef::B, plane_face_at_z(&shell_b, 0.0)) {
            assert!(classification.inside_other[b_idx]);
        }
    }

    // ---------------------------------------------------------------------------
    // Test 2: disjoint solids — every bit outside.
    // ---------------------------------------------------------------------------

    #[test]
    fn classify_disjoint_solids_all_outside() {
        // a = the block; b = the disk extrude at (6, 6) r=1 (height 2). NO
        // events: the split call with an empty event list leaves every face a
        // single fragment (7 + 3 = 9).
        let (profile_a, arr_a) = block_profile();
        let shell_a = extrude_shell(&profile_a, &arr_a, 2.0);
        let (profile_b, arr_b) = disk_profile(Point2::new(6.0, 6.0), 1.0);
        let shell_b = extrude_shell(&profile_b, &arr_b, 2.0);

        let mesh = split_fragments(&shell_a, &shell_b, &[], TOL).unwrap().value;
        assert_eq!(mesh.fragments.len(), 9);
        assert!(
            mesh.adjacency
                .iter()
                .all(|a| a.parity == AdjacencyParity::Same),
            "no contact arc was inserted, so no Flip adjacency exists"
        );

        // Both components are contact-free: each ray-seeds with winding 0 on
        // direction 1 (+z), so every bit is false.
        let classification = classify_fragments(&shell_a, &shell_b, &mesh, TOL)
            .unwrap()
            .value;
        assert_eq!(classification.inside_other.len(), 9);
        for i in 0..classification.inside_other.len() {
            assert!(
                !classification.inside_other[i],
                "fragment {i} must be outside the disjoint solid"
            );
        }
    }

    // ---------------------------------------------------------------------------
    // Test 3: a strictly contained solid — a false, b true.
    // ---------------------------------------------------------------------------

    #[test]
    fn classify_contained_solid_ray_seed() {
        // a = the block; b = the hand-built raised disk at (2, 2) r=1, z in
        // [0.5, 1.5] — NO caps coplanar with a's. NO events.
        let (profile_a, arr_a) = block_profile();
        let shell_a = extrude_shell(&profile_a, &arr_a, 2.0);
        let shell_b = raised_disk(Point2::new(2.0, 2.0), 1.0, 0.5, 1.5);

        let mesh = split_fragments(&shell_a, &shell_b, &[], TOL).unwrap().value;
        assert_eq!(mesh.fragments.len(), 9);

        let classification = classify_fragments(&shell_a, &shell_b, &mesh, TOL)
            .unwrap()
            .value;
        assert_eq!(classification.inside_other.len(), 9);

        // a's six fragments all false: the ray from a's bottom-face
        // representative (2, 2, 0) crosses b's two caps at (2,2,0.5) and
        // (2,2,1.5), winding +1 − 1 = 0.
        for i in 0..6 {
            assert!(
                !classification.inside_other[i],
                "a's fragment {i} must be outside b"
            );
        }
        // b's three fragments all true: b's bottom-cap representative
        // (2, 2, 0.5) is strictly inside a; the +z ray crosses a's top face
        // once, exiting, winding −1.
        for i in 6..9 {
            assert!(
                classification.inside_other[i],
                "b's fragment {i} must be inside a"
            );
        }
    }

    // ---------------------------------------------------------------------------
    // Test 4: the ambiguous direction retries through the band rule.
    // ---------------------------------------------------------------------------

    #[test]
    fn classify_ray_seed_retries_ambiguous_direction() {
        // a = the block; b = the hand-built raised disk at (2.5, 2) r=0.5, z
        // in [0.5, 1.5] — DYADIC: a's bottom-face representative is (2, 2, 0),
        // which sits at radial distance exactly 0.5 from b's axis, ON b's caps'
        // boundary circle.
        let (profile_a, arr_a) = block_profile();
        let shell_a = extrude_shell(&profile_a, &arr_a, 2.0);
        let shell_b = raised_disk(Point2::new(2.5, 2.0), 0.5, 0.5, 1.5);

        let mesh = split_fragments(&shell_a, &shell_b, &[], TOL).unwrap().value;
        assert_eq!(mesh.fragments.len(), 9);

        // The first direction (+z) classifies both cap crossings Boundary
        // (ambiguous), so the seed retries; the second direction (+x) answers
        // winding 0 — its wall crossings are at t = 0 (skipped, t <= tol) and
        // t = 1 (the point (3, 2, 0), rejected by the BAND rule: v = 0 outside
        // the [0.5, 1.5] band). If the band rule were broken and counted it,
        // d·n > 0 makes it an exit, the winding would be −1, and a's bottom bit
        // would flip to true — this test's real teeth. The pre-screen must NOT
        // fire for a's representative: it lies ON b's wall CARRIER but OUTSIDE
        // the wall's trimmed band, which is not the boundary — succeeding here
        // (instead of a NumericallyUnresolved refusal) distinguishes carrier
        // from region.
        let classification = classify_fragments(&shell_a, &shell_b, &mesh, TOL)
            .unwrap()
            .value;
        for i in 0..6 {
            assert!(
                !classification.inside_other[i],
                "a's fragment {i} must be outside b"
            );
        }
        for i in 6..9 {
            assert!(
                classification.inside_other[i],
                "b's fragment {i} must be inside a"
            );
        }
    }

    // ---------------------------------------------------------------------------
    // Test 5: the open-arc mesh refuses Contradictory.
    // ---------------------------------------------------------------------------

    #[test]
    fn classify_contradictory_mesh_refuses() {
        // a = the block; b = the disk extrude at (4, 2) r=1. The events of
        // `split_open_arc_uses_point_events_for_trimming` (FF TwoCurves + the
        // four Point events; NO Region2 record). The split succeeds; the
        // classification MUST refuse `Contradictory` with
        // `prop == FragmentInsideOther`: both solids' cap fragments straddle
        // the other solid's boundary (the missing Region2 record makes the
        // mesh parity-inconsistent), and the non-tree-edge verification catches
        // it.
        let (profile_a, arr_a) = block_profile();
        let shell_a = extrude_shell(&profile_a, &arr_a, 2.0);
        let (profile_b, arr_b) = disk_profile(Point2::new(4.0, 2.0), 1.0);
        let shell_b = extrude_shell(&profile_b, &arr_b, 2.0);

        let x4_side = shell_a
            .face_iter()
            .enumerate()
            .find(|(_, face)| match face.surface() {
                Surface::Plane(p) => {
                    (p.origin().x - 4.0).abs() < TOL && (p.origin().y - 0.0).abs() < TOL
                }
                _ => false,
            })
            .map(|(i, _)| i)
            .unwrap();
        let wall_b = cylinder_face(&shell_b);
        let bottom_edge = flat_edge_at_z(&shell_a, x4_side, 0.0);
        let top_edge = flat_edge_at_z(&shell_a, x4_side, 2.0);

        let line1 = ExactCurve::Line(Line(Point3::new(4.0, 1.0, 0.0), Point3::new(4.0, 1.0, 2.0)));
        let line2 = ExactCurve::Line(Line(Point3::new(4.0, 3.0, 0.0), Point3::new(4.0, 3.0, 2.0)));
        let ff = ev(
            ContactRecord {
                dimension: ContactDimension::Arc1,
                kind: ContactEventKind::Transverse,
                locus: ContactLocus::Analytic(AnalyticIntersection::TwoCurves([line1, line2])),
            },
            StratumRef::Face {
                solid: SolidRef::A,
                index: x4_side,
            },
            StratumRef::Face {
                solid: SolidRef::B,
                index: wall_b,
            },
        );

        let mut events = vec![ff];
        for (y, z) in [(1.0, 0.0), (3.0, 0.0), (1.0, 2.0), (3.0, 2.0)] {
            let edge = if z == 0.0 { bottom_edge } else { top_edge };
            events.push(ev(
                ContactRecord {
                    dimension: ContactDimension::Point0,
                    kind: ContactEventKind::Transverse,
                    locus: ContactLocus::Point(Point3::new(4.0, y, z)),
                },
                StratumRef::Edge {
                    solid: SolidRef::A,
                    face: x4_side,
                    edge,
                },
                StratumRef::Face {
                    solid: SolidRef::A,
                    index: x4_side,
                },
            ));
        }

        let mesh = split_fragments(&shell_a, &shell_b, &events, TOL)
            .unwrap()
            .value;
        let out = classify_fragments(&shell_a, &shell_b, &mesh, TOL);
        assert!(
            matches!(
                out,
                Err(Refusal::Contradictory(ContradictionWitness {
                    prop: Prop::FragmentInsideOther,
                    ..
                }))
            ),
            "the open-arc mesh must refuse Contradictory(FragmentInsideOther), got {out:?}"
        );
    }

    // ---------------------------------------------------------------------------
    // Test 6: the analytic ray solves (BG-NUM-002 derivations inline).
    // ---------------------------------------------------------------------------

    #[test]
    fn classify_cone_and_sphere_ray_solves() {
        // Sphere: center (0,0,0), r = 2, ray from (0,0,5) along −z. With
        // p = (0,0,5), d = (0,0,−1), c = (0,0,0):
        //   |p + t·d − c|² = r²  →  (5 − t)² = 4  →  t = 3 and t = 7.
        let sphere = Surface::Sphere(Sphere::new(Point3::origin(), 2.0));
        let crossings = surface_ray_crossings(
            &sphere,
            Point3::new(0.0, 0.0, 5.0),
            Vector3::new(0.0, 0.0, -1.0),
        );
        let mut ts: Vec<f64> = crossings.iter().map(|(t, _)| *t).collect();
        ts.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert_eq!(ts.len(), 2);
        assert!((ts[0] - 3.0).abs() < TOL);
        assert!((ts[1] - 7.0).abs() < TOL);

        // Cone: apex (0,0,0), half-angle π/4 (k = tan(π/4) = 1), ray from
        // (5,0,1) along −x. With p = (5,0,1), d = (−1,0,0), e = p − apex:
        //   a = dx² + dy² − k²·dz² = 1, b = 2(ex·dx + ey·dy − k²·ez·dz) = −10,
        //   c = ex² + ey² − k²·ez² = 24, disc = 100 − 96 = 4,
        //   t = (10 ± 2)/2 → t = 4 and t = 6 → points (1,0,1) and (−1,0,1).
        let cone = Surface::Cone(Cone::new(Point3::origin(), FRAC_PI_4).unwrap().value);
        let crossings = surface_ray_crossings(
            &cone,
            Point3::new(5.0, 0.0, 1.0),
            Vector3::new(-1.0, 0.0, 0.0),
        );
        let mut ts: Vec<f64> = crossings.iter().map(|(t, _)| *t).collect();
        ts.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert_eq!(ts.len(), 2);
        assert!((ts[0] - 4.0).abs() < TOL);
        assert!((ts[1] - 6.0).abs() < TOL);
        for (_, q) in &crossings {
            assert!((q.z - 1.0).abs() < TOL);
        }
    }
}
