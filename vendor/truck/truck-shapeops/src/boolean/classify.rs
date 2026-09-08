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
use truck_geometry::specifieds::Torus;
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
        // The band-form degeneracy (RW-INTERIOR-LOOP): a periodic carrier's
        // full-period wire polygons have zero signed area, so the area rule
        // cannot decide the orientation sign. A band-form face's region is the
        // positive-v-span strip over the full period — always positive — so the
        // sign reduces to the orientation flag.
        let band = match face.surface().u_period() {
            Some(period) => {
                let mut all_polys = Vec::new();
                for wire in face.boundaries() {
                    all_polys.push(
                        create_parameter_boundary(face, &wire, &mut cache, tol)
                            .ok_or_else(numerically_unresolved)?,
                    );
                }
                band_form(&all_polys, Some(period), 0)
            }
            None => false,
        };
        if band {
            return Ok(if face.orientation() { 1.0 } else { -1.0 });
        }
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

/// Whether every face of `shell` is one of the canonical carriers the ray
/// solve implements (the five analytic arms, torus dispatch extended by
/// TOR-B — ring tori only); any other arm refuses `NonCanonicalCarrier`.
fn require_canonical_carriers(shell: &Shell<Point3, Curve, Surface>) -> Result<(), Refusal> {
    for face in shell.face_iter() {
        let canonical = match face.surface() {
            Surface::Plane(_) | Surface::Cylinder(_) | Surface::Cone(_) | Surface::Sphere(_) => {
                true
            }
            // TOR-B FSSI-EXT: dispatch-table extension only. Horn (`large ==
            // small`) and spindle (`large < small`) tori are not regular
            // embedded surfaces and refuse typed, exactly as the landed
            // `formal/torus.rs` identification rule.
            Surface::Torus(torus) => torus.large_radius() > torus.small_radius(),
            Surface::RevolutedCurve(_)
            | Surface::ExtrudedCurve(_)
            | Surface::BSplineSurface(_)
            | Surface::NurbsSurface(_)
            | Surface::Processor(_)
            | Surface::SpineFrameSurface(_) => false,
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
        Surface::Torus(torus) => ray_torus_contacts(torus, p, d)
            .into_iter()
            .filter(|contact| contact.kind == TorusRayContactKind::Crossing)
            .map(|contact| (contact.t, contact.point))
            .collect(),
        Surface::RevolutedCurve(_)
        | Surface::ExtrudedCurve(_)
        | Surface::BSplineSurface(_)
        | Surface::NurbsSurface(_)
        | Surface::Processor(_)
        | Surface::SpineFrameSurface(_) => Vec::new(),
    }
}

// ---------------------------------------------------------------------------
// the ray-torus quartic solve (TOR-B)
// ---------------------------------------------------------------------------

/// The signed classification of one real root of the ray-torus quartic
/// `g(t) = Φ(q₀ + t·v)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TorusRayContactKind {
    /// An odd-multiplicity root: the certified flank signs on the two sides of
    /// the isolated root differ, so the ray crosses the surface. Entering/
    /// exiting is decided downstream by `v·n` exactly as the other arms.
    Crossing,
    /// An even-multiplicity contact (a tangent graze or a higher even contact):
    /// `g` keeps its sign on the two sides of the isolated root, so the ray does
    /// NOT cross. `g(t) = g′(t) = 0` is never taken as proof of non-crossing —
    /// the flank signs certify.
    Grazing,
}

/// A typed ray-torus contact: one real root of the quartic on the ray, its
/// point, and the algebraic crossing bit.
#[derive(Clone, Copy, Debug)]
pub struct TorusRayContact {
    /// The ray parameter of the contact.
    pub t: f64,
    /// The contact point `p + t·d`.
    pub point: Point3,
    /// The algebraic crossing classification of the root.
    pub kind: TorusRayContactKind,
}

/// The subdivision depth beyond which a sign-unresolved quartic cell is a
/// multiple-root candidate (H-3: a dimensionless depth, not a length).
const TORUS_ROOT_MAX_DEPTH: usize = 96;

/// A multiple-root cell's width floor, in ulps of the cell scale: below this a
/// cell cannot separate roots further (H-3: a dimensionless parameter width).
const TORUS_ROOT_FLOOR_ULPS: f64 = 32.0;

/// The relative flank probe, in units of the contact's parameter scale, used to
/// read the certified flank signs on the two sides of an isolated root
/// (H-3: a dimensionless relative offset in ray parameters).
const TORUS_FLANK_RELATIVE: f64 = 1e-6;

/// The relative isolation window for skipping a critical point already
/// accounted for by an odd crossing root (H-3: dimensionless in ray
/// parameters).
const TORUS_CRITICAL_PROXIMITY: f64 = 1e-2;

/// The five coefficients (constant term first) of the ray-torus quartic
///
/// `g(t) = Φ(p + t·d)`, with `Φ` the scale-invariant torus form
/// `K(q·q + R² − r²)² − 4R²(K(q·q) − (q·a)²)` of the theory (T), `q = p − O`.
/// A [`Surface::Torus`] carrier is always `z`-canonical (`a = ẑ`, `K = 1`), so
/// the coefficients are exact products/sums of the carrier radii and the ray
/// data; `g₄ = ‖d‖⁴ > 0`, so the ray solve is always a genuine quartic.
pub fn ray_torus_quartic(torus: &Torus, p: Point3, d: Vector3) -> [f64; 5] {
    let q = p - torus.center();
    let m0 = q.dot(q);
    let m1 = 2.0 * q.dot(d);
    let m2 = d.dot(d);
    let s0 = q.z;
    let s1 = d.z;
    let r2 = torus.large_radius() * torus.large_radius();
    let s2 = torus.small_radius() * torus.small_radius();
    let c0 = r2 - s2;
    let m0_c0 = m0 + c0;
    let four_r2 = 4.0 * r2;
    [
        m0_c0 * m0_c0 - four_r2 * (m0 - s0 * s0),
        2.0 * m0_c0 * m1 - four_r2 * (m1 - 2.0 * s0 * s1),
        m1 * m1 + 2.0 * m0_c0 * m2 - four_r2 * (m2 - s1 * s1),
        2.0 * m1 * m2,
        m2 * m2,
    ]
}

/// The typed real contacts of the ray `p + t·d` with the torus surface
/// (`t ≥ 0`), ascending in `t`. Transverse and other odd-multiplicity crossings
/// are [`TorusRayContactKind::Crossing`]; tangent grazes and higher even
/// contacts are [`TorusRayContactKind::Grazing`] — their crossing bit is typed
/// `false` by the certified flank signs, never by assuming tangency.
///
/// Float roots supply the candidates (SFC); the flank-sign comparison on the
/// two sides of each isolated root certifies the crossing bit: different
/// signs ⇒ odd-multiplicity crossing, equal signs ⇒ even contact (typed
/// non-crossing, never assumed from `g(t) = g′(t) = 0`). The
/// bounding-sphere argument bounds the search domain: every torus point lies
/// within `R + r` of the centre, so every positive real root lies before the
/// ray leaves that sphere.
pub fn ray_torus_contacts(torus: &Torus, p: Point3, d: Vector3) -> Vec<TorusRayContact> {
    let Some((lo, hi)) = ray_contact_domain(torus, p, d) else {
        return Vec::new();
    };
    let coeffs = ray_torus_quartic(torus, p, d);

    // Odd-multiplicity candidates: sign-change isolation of `g` itself.
    let mut candidates = subdivide_odd_roots(&coeffs, lo, hi);

    // Even-multiplicity candidates: real critical points of `g` (odd roots of
    // the derivative cubic) that lie on the surface (`g ≈ 0`). A double or
    // quadruple contact of `g` is an odd root of `g'`, so the same sign-change
    // isolation finds its parameter; a triple root of `g` (an odd root) is
    // already in the `g` list, and its `g'` double root is deliberately not
    // chased here.
    let derivative = poly_derivative(&coeffs);
    for t0 in subdivide_odd_roots(&derivative, lo, hi) {
        let scale = t0.abs().max(1.0);
        if candidates
            .iter()
            .any(|&r| (r - t0).abs() <= scale * TORUS_CRITICAL_PROXIMITY)
        {
            continue;
        }
        if poly_eval(&coeffs, t0).abs() > poly_abs_scale(&coeffs, t0) * 1e-9 {
            continue;
        }
        candidates.push(t0);
    }

    // The certified flank signs decide the crossing bit of every isolated
    // root: sample the quartic immediately on the two sides, inside a window
    // free of any other candidate.
    candidates.sort_by(|a, b| a.total_cmp(b));
    let mut unique: Vec<f64> = Vec::new();
    for t in candidates {
        let scale = t.abs().max(1.0);
        let dup = unique
            .last()
            .is_some_and(|&prev| (t - prev).abs() <= scale * 1e-9);
        if !dup {
            unique.push(t);
        }
    }

    let mut contacts = Vec::new();
    for (i, &t) in unique.iter().enumerate() {
        let scale = t.abs().max(1.0);
        let left_gap = match i.checked_sub(1).and_then(|j| unique.get(j)) {
            Some(&prev) => t - prev,
            None => f64::INFINITY,
        };
        let right_gap = match unique.get(i + 1) {
            Some(&next) => next - t,
            None => f64::INFINITY,
        };
        let gap = left_gap.min(right_gap);
        let mut delta = if gap.is_finite() {
            gap * 0.25
        } else {
            scale * 0.25
        };
        delta = delta.max(scale * TORUS_FLANK_RELATIVE);
        if gap.is_finite() && delta > gap * 0.49 {
            delta = gap * 0.49;
        }
        let g_lo = poly_eval(&coeffs, t - delta);
        let g_hi = poly_eval(&coeffs, t + delta);
        let crossing = (g_lo < 0.0) != (g_hi < 0.0);
        contacts.push(TorusRayContact {
            t,
            point: p + d * t,
            kind: if crossing {
                TorusRayContactKind::Crossing
            } else {
                TorusRayContactKind::Grazing
            },
        });
    }
    contacts
}

/// The positive-`t` parameter window that can contain a torus contact: every
/// point of the torus is within `R + r` of its centre, so every real root of
/// the quartic lies before the ray leaves that bounding sphere.
fn ray_contact_domain(torus: &Torus, p: Point3, d: Vector3) -> Option<(f64, f64)> {
    let q0 = p - torus.center();
    let bound = torus.large_radius() + torus.small_radius();
    let bound2 = bound * bound;
    let a = d.dot(d);
    if a <= NORMAL_SLACK {
        return None;
    }
    let b = 2.0 * q0.dot(d);
    let cc = q0.dot(q0) - bound2;
    let disc = b * b - 4.0 * a * cc;
    if disc < 0.0 {
        return None;
    }
    let sq = disc.sqrt();
    let hi_raw = (-b + sq) / (2.0 * a);
    let lo_raw = (-b - sq) / (2.0 * a);
    if hi_raw <= 0.0 {
        return None;
    }
    // Pad past the sphere so a root exactly on the sphere boundary (an outer
    // equator contact) is interior to the search domain.
    let span = (hi_raw - lo_raw).abs().max(1.0);
    let pad = span * 1e-6;
    let lo = if cc <= 0.0 {
        0.0
    } else {
        (lo_raw - pad).max(0.0)
    };
    let hi = hi_raw + pad;
    if hi <= lo {
        return None;
    }
    Some((lo, hi))
}

/// Horner evaluation of the monomial polynomial `coeffs` (constant term first)
/// at `t`.
fn poly_eval(coeffs: &[f64], t: f64) -> f64 {
    coeffs.iter().rev().fold(0.0, |acc, c| acc * t + c)
}

/// A scale for `|g(t)|` comparisons: the sum of the absolute monomial terms,
/// robust where cancellation makes the value itself unreliable.
fn poly_abs_scale(coeffs: &[f64], t: f64) -> f64 {
    let mut scale = 0.0;
    let mut pow = 1.0;
    for &c in coeffs {
        scale += c.abs() * pow;
        pow *= t;
        if !pow.is_finite() {
            break;
        }
    }
    if !scale.is_finite() {
        return f64::INFINITY;
    }
    scale
}

/// The derivative coefficients (constant term first) of `coeffs`.
fn poly_derivative(coeffs: &[f64]) -> Vec<f64> {
    coeffs
        .iter()
        .enumerate()
        .skip(1)
        .map(|(i, c)| c * i as f64)
        .collect()
}

/// Coefficients of `p(lo + w·s)` as a polynomial in `s` (ascending), by Horner
/// composition with the linear factor `lo + w·s`.
fn shift_scale(coeffs: &[f64], lo: f64, w: f64) -> Vec<f64> {
    let mut acc: Vec<f64> = Vec::new();
    for &c in coeffs.iter().rev() {
        if acc.is_empty() {
            acc.push(c);
            continue;
        }
        let mut out = vec![0.0; acc.len() + 1];
        for (i, v) in acc.iter().copied().enumerate() {
            if let Some(slot) = out.get_mut(i) {
                *slot += v * lo;
            }
            if let Some(slot) = out.get_mut(i + 1) {
                *slot += v * w;
            }
        }
        if let Some(slot) = out.first_mut() {
            *slot += c;
        }
        acc = out;
    }
    acc
}

/// The binomial coefficient `C(n, k)`.
fn comb(n: usize, k: usize) -> u64 {
    let k = k.min(n - k);
    let mut c: u64 = 1;
    for i in 0..k {
        c = c * (n - i) as u64 / (i + 1) as u64;
    }
    c
}

/// The Bernstein coefficients (over `[0, 1]`) of the monomial polynomial
/// `power` (ascending), via the exact power-to-Bernstein matrix.
fn power_to_bernstein(power: &[f64]) -> Vec<f64> {
    let n = power.len() - 1;
    let mut out = vec![0.0; power.len()];
    for (i, slot) in out.iter_mut().enumerate() {
        for (j, &aj) in power.iter().enumerate().take(i + 1) {
            *slot += aj * (comb(i, j) as f64) / (comb(n, j) as f64);
        }
    }
    out
}

/// The Bernstein coefficients of `coeffs` over `[lo, hi]`.
fn bernstein_over(coeffs: &[f64], lo: f64, hi: f64) -> Vec<f64> {
    power_to_bernstein(&shift_scale(coeffs, lo, hi - lo))
}

/// Strict sign changes over `coeffs` after deleting exact zeros, plus whether
/// any exact zero remains. `(0, false)` is the certified-no-root signature.
fn bernstein_sign_changes(coeffs: &[f64]) -> (u32, bool) {
    let mut changes: u32 = 0;
    let mut has_zero = false;
    let mut prev: Option<f64> = None;
    for &c in coeffs {
        if c == 0.0 {
            has_zero = true;
            continue;
        }
        if let Some(p) = prev {
            if (p > 0.0) != (c > 0.0) {
                changes += 1;
            }
        }
        prev = Some(c);
    }
    (changes, has_zero)
}

/// de Casteljau subdivision at the midpoint (the two child coefficient
/// sequences, left then right).
fn split_bernstein(coeffs: &[f64]) -> (Vec<f64>, Vec<f64>) {
    let mut left = Vec::with_capacity(coeffs.len());
    let mut right = Vec::with_capacity(coeffs.len());
    let mut row: Vec<f64> = coeffs.to_vec();
    left.push(row.first().copied().unwrap_or(0.0));
    right.push(row.last().copied().unwrap_or(0.0));
    while row.len() > 1 {
        let next: Vec<f64> = row
            .iter()
            .zip(row.iter().skip(1))
            .map(|(a, b)| 0.5 * *a + 0.5 * *b)
            .collect();
        left.push(next.first().copied().unwrap_or(0.0));
        right.push(next.last().copied().unwrap_or(0.0));
        row = next;
    }
    right.reverse();
    (left, right)
}

/// The real odd-multiplicity roots of `coeffs` over `(lo, hi)`, each refined to
/// machine precision.
///
/// Descartes on the Bernstein coefficients prunes sign-definite cells
/// (`(0, false)`), emits a cell with exactly one sign change (one odd root,
/// refined), and keeps subdividing everything else. A cell that cannot separate
/// further (even-multiplicity contact or an unresolved odd cluster) contributes
/// its midpoint only when its sign variation is odd — parity of the root count
/// is preserved without ever guessing a multiplicity.
fn subdivide_odd_roots(coeffs: &[f64], lo: f64, hi: f64) -> Vec<f64> {
    let mut out: Vec<f64> = Vec::new();
    let mut stack: Vec<(f64, f64, Vec<f64>)> = Vec::new();
    stack.push((lo, hi, bernstein_over(coeffs, lo, hi)));
    let mut depth = 0usize;
    while let Some((blo, bhi, bern)) = stack.pop() {
        let (v, has_zero) = bernstein_sign_changes(&bern);
        if v == 0 && !has_zero {
            continue;
        }
        if v == 1 {
            out.push(refine_root(coeffs, blo, bhi));
            continue;
        }
        let scale = blo.abs().max(bhi.abs()).max(1.0);
        if bhi - blo <= scale * TORUS_ROOT_FLOOR_ULPS * f64::EPSILON
            || depth >= TORUS_ROOT_MAX_DEPTH
        {
            if v % 2 == 1 {
                out.push(0.5 * (blo + bhi));
            }
            continue;
        }
        let (left, right) = split_bernstein(&bern);
        let mid = 0.5 * blo + 0.5 * bhi;
        depth += 1;
        stack.push((mid, bhi, right));
        stack.push((blo, mid, left));
    }
    out
}

/// Bisection then Newton refinement of the single odd root bracketed in
/// `[lo, hi]` (the function changes sign across the bracket).
fn refine_root(coeffs: &[f64], lo: f64, hi: f64) -> f64 {
    let mut a = lo;
    let mut b = hi;
    let fa = poly_eval(coeffs, a);
    if fa == 0.0 {
        return a;
    }
    if poly_eval(coeffs, b) == 0.0 {
        return b;
    }
    for _ in 0..200 {
        let m = 0.5 * (a + b);
        let fm = poly_eval(coeffs, m);
        if fm == 0.0 {
            return m;
        }
        if (fa > 0.0) == (fm > 0.0) {
            a = m;
        } else {
            b = m;
        }
        let scale = a.abs().max(b.abs()).max(1.0);
        if b - a <= scale * 1e-14 {
            break;
        }
    }
    let mut x = 0.5 * (a + b);
    let der = poly_derivative(coeffs);
    for _ in 0..8 {
        let f = poly_eval(coeffs, x);
        let fp = poly_eval(&der, x);
        if fp == 0.0 || !f.is_finite() || !fp.is_finite() {
            break;
        }
        let step = f / fp;
        if !step.is_finite() {
            break;
        }
        x -= step;
        if x < lo || x > hi {
            x = 0.5 * (a + b);
            break;
        }
        if step.abs() <= x.abs().max(1.0) * 1e-15 {
            break;
        }
    }
    x
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
        // TOR-B: a torus face mirrors the cylinder rule — the `u` coordinate
        // is the revolution period, so a full-period degenerate wire set is a
        // band (the region is the `v` span); a proper patch uses the polygon
        // rule with the `u` period for seam deduplication.
        Surface::Torus(_) => {
            if band_form(&polys, surface.u_period(), 0) {
                Some(band_rule(&polys, uv, 1, tol))
            } else {
                Some(polygon_rule(&polys, uv, surface.u_period(), tol))
            }
        }
        Surface::RevolutedCurve(_)
        | Surface::ExtrudedCurve(_)
        | Surface::BSplineSurface(_)
        | Surface::NurbsSurface(_)
        | Surface::Processor(_)
        | Surface::SpineFrameSurface(_) => None,
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
