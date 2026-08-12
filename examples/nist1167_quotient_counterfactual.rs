//! Decisive diagnostic counterfactual: keep the periodic-cover topology and
//! separate it from the *physical evaluation* by mapping cover UV back into the
//! surface's genuine native parameter domain before every evaluator call.
//!
//! The prior counterfactual (`nist1167_vperiod_counterfactual`) proved the
//! causal chain: supplying the source-declared V period fixes parity but emits
//! catastrophic geometry because the tessellator feeds periodic-cover UV
//! (e.g. v in [0.5, 1.5] or [-0.5, 0.5]) straight into the generic B-spline
//! evaluator, which only supports its native knot domain [0, 1].
//!
//! This probe changes *only* the physical evaluation of the two source-closed
//! B-spline surfaces (#506 behind #1167, #507 behind #1169). Every evaluator
//! call (`subs`, `uder`, `vder`, higher derivatives, `normal`, `der_mn`) maps
//! cover v through the quotient `v_eval = v_cover mod 1.0` into [0, 1] before
//! forwarding. Boundary cover coordinates, winding, parity, deck class, CDT
//! topology, projection and search are all left exactly as the period-only
//! counterfactual produced them.
//!
//! Three modes:
//!   clean        - production lattice (V nonperiodic); expected ContradictoryDualParity
//!   period       - V period 1.0 supplied, plain evaluation (baseline garbage)
//!   period+quot  - V period 1.0 supplied AND evaluation through the quotient map
//!
//! This is a probe only; it writes no production code.

use std::env;
use std::sync::atomic::{AtomicUsize, Ordering};

use truck_meshalgo::prelude::*;
use truck_stepio::r#in::{
    Table,
    step_geometry::{Curve3D, Surface},
};
use truck_topology::compress::CompressedShell;

/// How many evaluator calls received a cover coordinate outside the native
/// domain [0,1] (these were all quotiented before reaching the inner surface).
static OOD_EVAL_V: AtomicUsize = AtomicUsize::new(0);
/// And for the u axis.
static OOD_EVAL_U: AtomicUsize = AtomicUsize::new(0);
/// Total evaluator calls through the quotient wrapper.
static TOTAL_EVAL: AtomicUsize = AtomicUsize::new(0);

fn load(path: &str) -> anyhow::Result<Table> {
    let bytes = std::fs::read(path)?;
    let text = std::str::from_utf8(&bytes)
        .map(std::borrow::Cow::Borrowed)
        .unwrap_or_else(|_| bytes.iter().map(|&b| b as char).collect::<String>().into());
    let mut exchange = match look::step::part21::parse(&text) {
        Ok(exchange) => exchange,
        Err(_) => ruststep::parser::parse(&text)
            .map_err(|error| anyhow::anyhow!("failed to parse STEP: {error}"))?,
    };
    let section = exchange.data.swap_remove(0);
    Ok(Table::from_owned_data_section(section))
}

fn area_of(mesh: &PolygonMesh) -> f64 {
    let pos = mesh.positions();
    mesh.tri_faces()
        .iter()
        .map(|tri| {
            let a = pos[tri[0].pos];
            let b = pos[tri[1].pos];
            let c = pos[tri[2].pos];
            (b - a).cross(c - a).magnitude() * 0.5
        })
        .sum()
}

fn bbox_diag_of(mesh: &PolygonMesh) -> f64 {
    let mut lo = Point3::new(f64::INFINITY, f64::INFINITY, f64::INFINITY);
    let mut hi = Point3::new(f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
    for v in mesh.positions() {
        lo.x = lo.x.min(v.x);
        lo.y = lo.y.min(v.y);
        lo.z = lo.z.min(v.z);
        hi.x = hi.x.max(v.x);
        hi.y = hi.y.max(v.y);
        hi.z = hi.z.max(v.z);
    }
    (hi - lo).magnitude()
}

fn max_edge_of(mesh: &PolygonMesh) -> f64 {
    let pos = mesh.positions();
    mesh.tri_faces()
        .iter()
        .map(|tri| {
            let a = pos[tri[0].pos];
            let b = pos[tri[1].pos];
            let c = pos[tri[2].pos];
            (b - a)
                .magnitude()
                .max((c - b).magnitude())
                .max((a - c).magnitude())
        })
        .fold(0.0f64, f64::max)
}

/// A surface wrapper whose physical evaluation maps periodic-cover coordinates
/// back into the native evaluator domain, while topology/search stay untouched.
///
/// `u_quotient`/`v_quotient` are the periods of the axes the surface's *source*
/// declares closed. Evaluation of a coordinate on such an axis goes through
/// `c -> c - floor(c / P) * P` (the representative in [0, P]). All other axes
/// forward unchanged.
#[derive(Clone, Debug)]
struct QuotientSurface {
    inner: look::step::policy_geometry::PolicySurface,
    u_quotient: Option<f64>,
    v_quotient: Option<f64>,
}

impl QuotientSurface {
    fn new(
        inner: look::step::policy_geometry::PolicySurface,
        u_quotient: Option<f64>,
        v_quotient: Option<f64>,
    ) -> Self {
        Self {
            inner,
            u_quotient,
            v_quotient,
        }
    }

    fn inner(&self) -> &Surface {
        self.inner.inner()
    }

    /// Whether a projected UV lies inside the surface's native parameter domain
    /// on its non-quotiented axes. A result outside the domain is a spurious
    /// root of the inverse search, never a valid representative: the lift that
    /// produces cover coordinates runs *after* projection.
    fn in_native_domain(&self, uv: (f64, f64)) -> bool {
        let (ur, vr) = self.inner.try_range_tuple();
        let in_range = |x: f64, r: Option<(f64, f64)>| r.is_none_or(|(lo, hi)| x >= lo && x <= hi);
        in_range(uv.0, ur) && in_range(uv.1, vr)
    }

    fn qu(u: f64, v: f64, self_: &Self) -> (f64, f64) {
        TOTAL_EVAL.fetch_add(1, Ordering::Relaxed);
        let uq = self_.u_quotient;
        let vq = self_.v_quotient;
        let u_out = uq.is_some_and(|p| u < 0.0 || u > p);
        let v_out = vq.is_some_and(|p| v < 0.0 || v > p);
        if u_out {
            OOD_EVAL_U.fetch_add(1, Ordering::Relaxed);
        }
        if v_out {
            OOD_EVAL_V.fetch_add(1, Ordering::Relaxed);
        }
        let wrap = |c: f64, p: f64| c - p * f64::floor(c / p);
        let u = match uq {
            Some(p) => wrap(u, p),
            None => u,
        };
        let v = match vq {
            Some(p) => wrap(v, p),
            None => v,
        };
        (u, v)
    }
}

impl ParametricSurface for QuotientSurface {
    type Point = Point3;
    type Vector = Vector3;
    #[inline]
    fn subs(&self, u: f64, v: f64) -> Self::Point {
        let (u, v) = Self::qu(u, v, self);
        self.inner.subs(u, v)
    }
    #[inline]
    fn uder(&self, u: f64, v: f64) -> Self::Vector {
        let (u, v) = Self::qu(u, v, self);
        self.inner.uder(u, v)
    }
    #[inline]
    fn vder(&self, u: f64, v: f64) -> Self::Vector {
        let (u, v) = Self::qu(u, v, self);
        self.inner.vder(u, v)
    }
    #[inline]
    fn uuder(&self, u: f64, v: f64) -> Self::Vector {
        let (u, v) = Self::qu(u, v, self);
        self.inner.uuder(u, v)
    }
    #[inline]
    fn uvder(&self, u: f64, v: f64) -> Self::Vector {
        let (u, v) = Self::qu(u, v, self);
        self.inner.uvder(u, v)
    }
    #[inline]
    fn vvder(&self, u: f64, v: f64) -> Self::Vector {
        let (u, v) = Self::qu(u, v, self);
        self.inner.vvder(u, v)
    }
    #[inline]
    fn der_mn(&self, m: usize, n: usize, u: f64, v: f64) -> Self::Vector {
        let (u, v) = Self::qu(u, v, self);
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

impl ParametricSurface3D for QuotientSurface {
    #[inline]
    fn normal(&self, u: f64, v: f64) -> Vector3 {
        let (u, v) = Self::qu(u, v, self);
        self.inner.normal(u, v)
    }
    #[inline]
    fn normal_uder(&self, u: f64, v: f64) -> Vector3 {
        let (u, v) = Self::qu(u, v, self);
        self.inner.normal_uder(u, v)
    }
    #[inline]
    fn normal_vder(&self, u: f64, v: f64) -> Vector3 {
        let (u, v) = Self::qu(u, v, self);
        self.inner.normal_vder(u, v)
    }
}

impl ParameterDivision2D for QuotientSurface {
    fn parameter_division(
        &self,
        range: ((f64, f64), (f64, f64)),
        tol: f64,
    ) -> (Vec<f64>, Vec<f64>) {
        self.inner.parameter_division(range, tol)
    }
}

impl SearchParameter<D2> for QuotientSurface {
    type Point = Point3;
    #[inline]
    fn search_parameter<H: Into<SPHint2D>>(
        &self,
        point: Self::Point,
        hint: H,
        trials: usize,
    ) -> Option<(f64, f64)> {
        // DIAG-1167: a B-spline's parameter inverse can converge to a spurious
        // stationary point outside the surface's native domain (Newton escapes
        // past the knot ends). On a non-periodic axis that is never a valid
        // answer, so it is rejected here and the chain falls through to the
        // hintless/nearest/seed links, which stay inside the domain.
        let uv = self.inner.search_parameter(point, hint, trials)?;
        self.in_native_domain(uv).then_some(uv)
    }
    #[inline]
    fn search_parameter_seeds(&self) -> Vec<(f64, f64)> {
        self.inner.search_parameter_seeds()
    }
}

impl SearchNearestParameter<D2> for QuotientSurface {
    type Point = Point3;
    #[inline]
    fn search_nearest_parameter<H: Into<SPHint2D>>(
        &self,
        point: Self::Point,
        hint: H,
        trials: usize,
    ) -> Option<(f64, f64)> {
        let uv = self.inner.search_nearest_parameter(point, hint, trials)?;
        self.in_native_domain(uv).then_some(uv)
    }
}

fn report_face(
    tag: &str,
    mode: &str,
    mesh_opt: &Option<PolygonMesh>,
    failure: Option<truck_meshalgo::tessellation::TessellationFailureReason>,
    src_surface: &Surface,
    u_quotient: Option<f64>,
    v_quotient: Option<f64>,
) {
    match mesh_opt {
        Some(mesh) if !mesh.faces().is_empty() => {
            let ntri = mesh.tri_faces().len();
            let npos = mesh.positions().len();
            let area = area_of(mesh);
            let diag = bbox_diag_of(mesh);
            let max_edge = max_edge_of(mesh);
            let mut vmax = 0.0f64;
            let mut vsum = 0.0f64;
            let mut von = 0usize;
            let mut von_sample = 0usize;
            let mut off: Vec<f64> = Vec::new();
            let mut uv_lo = Point2::new(f64::INFINITY, f64::INFINITY);
            let mut uv_hi = Point2::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
            let mut far_out = 0usize;
            let mut proj_fail_uvs: Vec<String> = Vec::new();
            let vstride = (npos / 8000).max(1);
            for (vi, v) in mesh.positions().iter().enumerate() {
                // search_nearest_parameter always converges for a point on the
                // surface; search_parameter alone is unreliable on a B-spline.
                // Sampled: the recovered mesh is dense and each solve is a
                // Newton iteration.
                if vi % vstride == 0 {
                    let res = match src_surface.search_nearest_parameter(*v, None, 200) {
                        Some(uv) => src_surface.subs(uv.0, uv.1).distance(*v),
                        None => f64::INFINITY,
                    };
                    vmax = vmax.max(res);
                    vsum += res;
                    if res <= 1.0e-3 {
                        von += 1;
                    } else if res.is_finite() {
                        off.push(res);
                    } else {
                        off.push(f64::INFINITY);
                    }
                    von_sample += 1;
                }
                if let Some(uv) = mesh.uv_coords().get(vi) {
                    uv_lo.x = uv_lo.x.min(uv.x);
                    uv_lo.y = uv_lo.y.min(uv.y);
                    uv_hi.x = uv_hi.x.max(uv.x);
                    uv_hi.y = uv_hi.y.max(uv.y);
                    if uv.y > 1.1 || uv.y < -0.1 || uv.x > 1.1 || uv.x < -0.1 {
                        far_out += 1;
                    }
                    let (ux, uy) = (uv.x, uv.y);
                    let (vx, vy, vz) = (v.x, v.y, v.z);
                    if ux > 1.05 || ux < -0.05 {
                        proj_fail_uvs.push(format!(
                            "[{vi}] uv=({ux:.3},{uy:.3}) pos=({vx:.3},{vy:.3},{vz:.3})"
                        ));
                    }
                }
            }
            // Triangle interior / long-edge midpoint distance to the surface.
            // Sampled: the recovered mesh is dense, and one nearest-parameter
            // solve per vertex/centroid is the dominant cost.
            let mut centroid_worst = 0.0f64;
            let mut midpoint_worst = 0.0f64;
            let stride = (mesh.tri_faces().len() / 4000).max(1);
            for (ti, tri) in mesh.tri_faces().iter().enumerate().step_by(stride) {
                let _ = ti;
                let a = mesh.positions()[tri[0].pos];
                let b = mesh.positions()[tri[1].pos];
                let c = mesh.positions()[tri[2].pos];
                let centroid = Point3::new(
                    (a.x + b.x + c.x) / 3.0,
                    (a.y + b.y + c.y) / 3.0,
                    (a.z + b.z + c.z) / 3.0,
                );
                if let Some(uv) = src_surface.search_nearest_parameter(centroid, None, 200) {
                    centroid_worst =
                        centroid_worst.max(src_surface.subs(uv.0, uv.1).distance(centroid));
                } else {
                    centroid_worst = f64::INFINITY;
                }
                for (p, q) in [(a, b), (b, c), (c, a)] {
                    let mid = Point3::new((p.x + q.x) / 2.0, (p.y + q.y) / 2.0, (p.z + q.z) / 2.0);
                    if let Some(uv) = src_surface.search_nearest_parameter(mid, None, 200) {
                        midpoint_worst =
                            midpoint_worst.max(src_surface.subs(uv.0, uv.1).distance(mid));
                    } else {
                        midpoint_worst = f64::INFINITY;
                    }
                }
            }
            let off_str = if off.is_empty() {
                "none".to_string()
            } else {
                let mut shown: Vec<String> =
                    off.iter().take(6).map(|r| format!("{r:.2e}")).collect();
                if off.len() > 6 {
                    shown.push(format!("...{} of {} sampled", off.len(), von_sample));
                }
                shown.join(",")
            };
            // Direct evaluator residual: the emitted position against the
            // surface evaluated at the vertex's OWN cover UV quotiented into
            // the native domain. This is exactly what the quotient wrapper
            // produced, so it is the honest on-surface measure (no search).
            let mut direct_worst = 0.0f64;
            let mut direct_mean = 0.0f64;
            let mut direct_off = 0usize;
            let mut direct_worst_uv: Option<(f64, f64, Point3)> = None;
            for (vi, v) in mesh.positions().iter().enumerate() {
                let Some(uv) = mesh.uv_coords().get(vi) else {
                    continue;
                };
                let wrap = |c: f64, p: Option<f64>| match p {
                    Some(p) => c - p * f64::floor(c / p),
                    None => c,
                };
                let (ue, ve) = (wrap(uv.x, u_quotient), wrap(uv.y, v_quotient));
                let evaled = src_surface.subs(ue, ve);
                let res = evaled.distance(*v);
                direct_worst = direct_worst.max(res);
                direct_mean += res;
                if res > 1.0e-3 {
                    direct_off += 1;
                    if direct_worst_uv.is_none()
                        || res
                            > direct_worst_uv
                                .as_ref()
                                .map(|t| t.2.distance(*v))
                                .unwrap_or(0.0)
                    {
                        direct_worst_uv = Some((uv.x, uv.y, evaled));
                    }
                }
            }
            let _ = (direct_mean, direct_off, direct_worst_uv);
            let direct_str = match direct_worst_uv {
                Some((uw, vw, _)) => {
                    format!(
                        "direct_off={direct_off} direct_worst={direct_worst:.3e} worst_uv=({uw:.3},{vw:.3})"
                    )
                }
                None => format!("direct_off={direct_off} direct_worst={direct_worst:.3e}"),
            };
            let (ulox, uloy) = (uv_lo.x, uv_lo.y);
            let (uhix, uhixy) = (uv_hi.x, uv_hi.y);
            println!(
                "FACE {tag} [{mode}]: RENDERED tris={ntri} verts={npos} area={area:.3} \
                 diag={diag:.3} max_edge={max_edge:.3} on_surface={von}/{von_sample} \
                 off_surface={off_str} uv_lo=({ulox:.3},{uloy:.3}) uv_hi=({uhix:.3},{uhixy:.3}) \
                 out_of_domain_uv={far_out} residual_max={vmax:.3e} residual_mean={:.3e} \
                 centroid_worst={centroid_worst:.3e} edge_midpoint_worst={midpoint_worst:.3e} \
                 {direct_str}",
                vsum / von_sample.max(1) as f64,
            );
            // The longest edges and where they sit.
            let mut edges: Vec<(f64, usize, usize, usize)> = mesh
                .tri_faces()
                .iter()
                .map(|tri| {
                    let a = mesh.positions()[tri[0].pos];
                    let b = mesh.positions()[tri[1].pos];
                    let c = mesh.positions()[tri[2].pos];
                    let dab = (b - a).magnitude();
                    let dbc = (c - b).magnitude();
                    let dca = (a - c).magnitude();
                    let (d, i, j) = if dab >= dbc && dab >= dca {
                        (dab, tri[0].pos, tri[1].pos)
                    } else if dbc >= dca {
                        (dbc, tri[1].pos, tri[2].pos)
                    } else {
                        (dca, tri[2].pos, tri[0].pos)
                    };
                    (d, i, j, tri[0].pos)
                })
                .collect::<Vec<_>>();
            edges.sort_by(|x, y| y.0.partial_cmp(&x.0).unwrap());
            for (d, i, j, k) in edges.iter().take(5) {
                let p = mesh.positions()[*i];
                let q = mesh.positions()[*j];
                let r = mesh.positions()[*k];
                let puv = mesh.uv_coords().get(*i).map(|v| (v.x, v.y));
                let quv = mesh.uv_coords().get(*j).map(|v| (v.x, v.y));
                let puv = puv.unwrap_or((f64::NAN, f64::NAN));
                let quv = quv.unwrap_or((f64::NAN, f64::NAN));
                let (ax, ay, az) = (p.x, p.y, p.z);
                let (bx, by, bz) = (q.x, q.y, q.z);
                let (cx, cy, cz) = (r.x, r.y, r.z);
                println!(
                    "  LONGEDGE {d:.3} A=({ax:.2},{ay:.2},{az:.2}) uv=({:.3},{:.3}) \
                     B=({bx:.2},{by:.2},{bz:.2}) uv=({:.3},{:.3}) C=({cx:.2},{cy:.2},{cz:.2})",
                    puv.0, puv.1, quv.0, quv.1,
                );
            }
            // Vertices whose u leaves the native domain (u is NOT periodic,
            // so no quotient can fix them).
            if !proj_fail_uvs.is_empty() {
                println!("  U_OUT_OF_DOMAIN ({}):", proj_fail_uvs.len());
                for s in proj_fail_uvs.iter().take(8) {
                    println!("    {s}");
                }
            }
        }
        Some(mesh) => {
            let f = failure
                .map(|r| format!("{r:?}"))
                .unwrap_or_else(|| "-".into());
            println!(
                "FACE {tag} [{mode}]: EMPTY tris=0 verts={} failure={f}",
                mesh.positions().len()
            );
        }
        None => {
            let f = failure
                .map(|r| format!("{r:?}"))
                .unwrap_or_else(|| "-".into());
            println!("FACE {tag} [{mode}]: NO SURFACE failure={f}");
        }
    }
}

fn main() -> anyhow::Result<()> {
    use look::step::policy_geometry::PolicyCurve;
    use truck_meshalgo::tessellation::domain::lattice::{AxisPeriodStatus, CertifiedLattice};

    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() {
        eprintln!("usage: nist1167_quotient_counterfactual MODEL.step");
        return Ok(());
    }
    let model_path = &args[0];
    let table = load(model_path)?;
    let mut done = false;
    for (shell_idx, (&shell_id, shell)) in table.shell.iter().enumerate() {
        let (cshell, losses) = table
            .to_compressed_shell_with_losses(shell_id, shell)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        println!(
            "=== shell #{shell_idx} conversion losses: {} ===",
            losses.len()
        );

        let mut target_surfaces: Vec<Surface> = Vec::new();
        let mut target_indices: Vec<(u64, usize)> = Vec::new();
        for (i, f) in cshell.faces.iter().enumerate() {
            if let Some(id) = f.provenance.best_id() {
                let g = id.get();
                if g == 1167 || g == 1169 {
                    target_surfaces.push(f.surface.clone());
                    target_indices.push((g, i));
                }
            }
        }
        if target_surfaces.is_empty() {
            continue;
        }

        let mut model_bbox = BoundingBox::<Point3>::new();
        for v in &cshell.vertices {
            model_bbox.push(*v);
        }
        for edge in &cshell.edges {
            let (a, b) = edge.curve.range_tuple();
            for i in 0..=4 {
                model_bbox.push(edge.curve.subs(a + (b - a) * f64::from(i) / 4.0));
            }
        }
        let scaled = model_bbox.diameter() * 0.001;
        let tol = if scaled.is_finite() && scaled > 0.0 {
            scaled.max(1.0e-6)
        } else {
            1.0e-3
        };
        println!("shell #{shell_idx}: model tol = {tol:.6e}");

        let is_target = |s: &Surface| target_surfaces.iter().any(|t| t == s);

        let period_lattice = CertifiedLattice {
            u: AxisPeriodStatus::NonPeriodic,
            v: AxisPeriodStatus::Uncertified { declared: 1.0 },
        };

        let modes: [(&str, bool); 3] = [("clean", false), ("period", true), ("period+quot", true)];

        for (mode, supply_period) in modes {
            println!("--- MODE {mode} ---");
            OOD_EVAL_V.store(0, Ordering::Relaxed);
            OOD_EVAL_U.store(0, Ordering::Relaxed);
            TOTAL_EVAL.store(0, Ordering::Relaxed);
            let apply_quotient = mode == "period+quot";
            // Re-wrap the shell with a quotient wrapper on the two target
            // surfaces. Every other surface forwards unchanged.
            let wrapped = look::step::policy_geometry::wrap_shell(
                cshell.clone(),
                look::step::meshing_policy::MeshingPolicy::DEFAULT,
            );
            let quot_shell = CompressedShell::<Point3, PolicyCurve, QuotientSurface> {
                vertices: wrapped.vertices,
                edges: wrapped.edges,
                faces: wrapped
                    .faces
                    .into_iter()
                    .enumerate()
                    .map(|(_f, face)| {
                        let target = is_target(&face.surface.inner());
                        let (u_quot, v_quot) = match (target, apply_quotient) {
                            (true, true) => (None, Some(1.0)),
                            _ => (None, None),
                        };
                        truck_topology::compress::CompressedFace {
                            boundaries: face.boundaries,
                            orientation: face.orientation,
                            surface: QuotientSurface::new(face.surface, u_quot, v_quot),
                            provenance: face.provenance,
                        }
                    })
                    .collect(),
                source_geometric_uncertainty: wrapped.source_geometric_uncertainty,
            };

            let lattice_of = |s: &QuotientSurface| {
                let inner = s.inner();
                if is_target(inner) && supply_period {
                    period_lattice
                } else {
                    look::step_lattice_of(inner)
                }
            };
            let outcome = quot_shell.robust_triangulation_with_torus_outcome(
                tol,
                lattice_of,
                |s: &QuotientSurface| look::step_support_schema_of(s.inner()),
                |c: &PolicyCurve| look::step_curve_schema_of(c.inner()),
                |s: &QuotientSurface| look::step_cylinder_of(s.inner()),
                |c: &PolicyCurve| look::step_cylinder_curve_schema_of(c.inner()),
                |c: &PolicyCurve| look::step_cylinder_curve_family_of(c.inner()),
                |s: &QuotientSurface| look::step_cone_of(s.inner()),
                |s: &QuotientSurface| look::step::torus_deck::identify_source_torus_opt(s.inner()),
            );
            let meshed = &outcome.shell;
            println!(
                "EVAL_WRAP counts: total_eval_calls={} ood_u={} ood_v={}",
                TOTAL_EVAL.load(Ordering::Relaxed),
                OOD_EVAL_U.load(Ordering::Relaxed),
                OOD_EVAL_V.load(Ordering::Relaxed),
            );
            for &(g, i) in &target_indices {
                let tag = format!("#{g}");
                let failure = outcome.face_failures.get(i).cloned().flatten();
                let fail_str = failure
                    .as_ref()
                    .map(|f| format!("{:?}", f.reason))
                    .unwrap_or_else(|| "-".into());
                println!("META {tag} [{mode}] outcome={fail_str}");
                let (u_quot, v_quot) = match mode {
                    "period+quot" => (None, Some(1.0)),
                    _ => (None, None),
                };
                report_face(
                    &tag,
                    mode,
                    &meshed.faces[i].surface,
                    failure.as_ref().map(|f| f.reason),
                    &cshell.faces[i].surface,
                    u_quot,
                    v_quot,
                );
            }
        }
        done = true;
        break;
    }
    if !done {
        anyhow::bail!("no shell found");
    }
    Ok(())
}
