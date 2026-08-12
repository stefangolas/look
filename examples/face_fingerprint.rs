//! Screen 1 — per-face fingerprints, and blame localization by differential.
//!
//! A blob produces *more* geometry, not less, so every diagnostic built around
//! missing faces is structurally silent on it: NIST `ftc_07` renders four fans
//! bursting out of a box while losing zero faces and printing no warning.
//! Finding the face responsible needs a screen that looks at geometry that
//! exists and asks whether it is the right size and in the right place.
//!
//! This is **blame localization, not a certificate**. Every number here is a
//! heuristic diagnostic in the sense of `MATHEMATICAL_FOUNDATION.md` §2.3: it
//! ranks faces for investigation and constructs no proof-bearing state. A face
//! at the top of the ranking is where to look, not a face proven wrong.
//!
//! The oracle is the corpus itself. Most NIST parts ship in three encodings —
//! AP203 geometry-only, AP203 with PMI, AP242 — of the same part, so a correct
//! rendering of a blobbed model is already on disk and no reference CAD kernel
//! is needed. Fingerprints from the two are matched by centroid, and a face
//! present in the bad encoding whose signature has no counterpart in the good
//! one is exactly what we are hunting.
//!
//! ```console
//! # fingerprints for one model
//! cargo run --release --example face_fingerprint -- MODEL.step
//!
//! # differential: rank the faces of BAD that GOOD does not account for
//! cargo run --release --example face_fingerprint -- BAD.step --against GOOD.step
//! ```
//!
//! `--csv` writes the raw table instead of the ranking, for feeding a census.

use std::collections::HashMap;
use std::env;

use truck_meshalgo::prelude::*;
use truck_stepio::r#in::{Table, step_geometry::Surface};
use truck_topology::compress::{CompressedShell, FaceProvenance};

type Curve3D = truck_stepio::r#in::step_geometry::Curve3D;
type Cshell = CompressedShell<Point3, Curve3D, Surface>;

const RELATIVE_TOLERANCE: f64 = 0.001;
const DEGENERATE_TOLERANCE: f64 = 1.0e-3;
const MINIMUM_TOLERANCE: f64 = 1.0e-6;
const EDGE_SAMPLES: u32 = 4;

/// What a face looks like once meshed, in numbers a blob cannot hide behind.
///
/// Deliberately cheap: every field comes from one pass over the face's own
/// triangles, so fingerprinting a whole model costs one tessellation, the same
/// one the renderer performs.
#[derive(Clone, Debug)]
struct FaceFingerprint {
    provenance: FaceProvenance,
    shell_entity: u64,
    shell_index: usize,
    surface_kind: &'static str,
    triangles: usize,
    vertices: usize,
    aabb: BoundingBox<Point3>,
    /// Sum of triangle areas. Not the true surface area — the mesh's.
    area: f64,
    max_triangle_edge: f64,
    centroid: Point3,
}

impl FaceFingerprint {
    fn extent(&self) -> f64 {
        if self.aabb.is_empty() {
            0.0
        } else {
            self.aabb.diameter()
        }
    }
}

/// The surface kind, as a short tag for grouping a census by geometry type.
///
/// A blob class is usually specific to one kind — the corner fans of `ftc_07`
/// are revolved surfaces — so this is the first column worth sorting by.
fn surface_kind(surface: &Surface) -> &'static str {
    use truck_stepio::r#in::step_geometry::{ElementarySurface, SweptCurve};
    match surface {
        Surface::ElementarySurface(e) => match e {
            ElementarySurface::Plane(_) => "plane",
            ElementarySurface::Sphere(_) => "sphere",
            ElementarySurface::CylindricalSurface(_) => "cylinder",
            ElementarySurface::ToroidalSurface(_) => "torus",
            ElementarySurface::ConicalSurface(_) => "cone",
        },
        Surface::SweptCurve(s) => match s {
            SweptCurve::ExtrudedCurve(_) => "extruded",
            SweptCurve::RevolutedCurve(_) => "revolved",
        },
        Surface::BSplineSurface(_) => "bspline",
        Surface::NurbsSurface(_) => "nurbs",
        Surface::OffsetSurface(_) => "offset",
    }
}

fn push_shell_extent(bounds: &mut BoundingBox<Point3>, shell: &Cshell) {
    for vertex in &shell.vertices {
        bounds.push(*vertex);
    }
    for edge in &shell.edges {
        let (start, end) = edge.curve.range_tuple();
        for step in 0..=EDGE_SAMPLES {
            let t = start + (end - start) * f64::from(step) / f64::from(EDGE_SAMPLES);
            bounds.push(edge.curve.subs(t));
        }
    }
}

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

/// Fingerprint every face of every shell, at the tolerance the renderer uses.
fn fingerprint(table: &Table, explicit_tolerance: Option<f64>) -> (Vec<FaceFingerprint>, f64, f64) {
    let shells: Vec<(u64, Cshell)> = table
        .shell
        .iter()
        .filter_map(|(id, shell)| {
            table
                .to_compressed_shell(*id, shell)
                .ok()
                .map(|cs| (*id, cs))
        })
        .collect();

    let mut model_box = BoundingBox::<Point3>::new();
    for (_, shell) in &shells {
        push_shell_extent(&mut model_box, shell);
    }
    let model_size = if model_box.is_empty() {
        0.0
    } else {
        model_box.diameter()
    };
    let scaled = model_size * RELATIVE_TOLERANCE;
    let tolerance = match explicit_tolerance {
        Some(explicit) => explicit,
        None if scaled.is_finite() && scaled > 0.0 => scaled.max(MINIMUM_TOLERANCE),
        None => DEGENERATE_TOLERANCE,
    };

    let mut prints = Vec::new();
    for (shell_index, (shell_entity, shell)) in shells.iter().enumerate() {
        // The kinds are read before tessellation, because tessellation replaces
        // the surface with its polygon and the kind is then unrecoverable.
        let kinds: Vec<&'static str> = shell
            .faces
            .iter()
            .map(|f| surface_kind(&f.surface))
            .collect();
        let meshed = shell.robust_triangulation(tolerance);
        for (face_index, face) in meshed.faces.iter().enumerate() {
            let Some(mesh) = &face.surface else { continue };
            let positions = mesh.positions();
            if positions.is_empty() {
                continue;
            }
            let mut aabb = BoundingBox::<Point3>::new();
            for p in positions {
                aabb.push(*p);
            }
            let mut area = 0.0;
            let mut max_edge: f64 = 0.0;
            let mut centroid = Vector3::zero();
            let mut triangles = 0usize;
            for tri in mesh.face_iter() {
                if tri.len() < 3 {
                    continue;
                }
                let (a, b, c) = (
                    positions[tri[0].pos],
                    positions[tri[1].pos],
                    positions[tri[2].pos],
                );
                area += (b - a).cross(c - a).magnitude() * 0.5;
                max_edge = max_edge
                    .max(a.distance(b))
                    .max(b.distance(c))
                    .max(c.distance(a));
                centroid += (a.to_vec() + b.to_vec() + c.to_vec()) / 3.0;
                triangles += 1;
            }
            if triangles == 0 {
                continue;
            }
            prints.push(FaceFingerprint {
                provenance: face.provenance,
                shell_entity: *shell_entity,
                shell_index,
                surface_kind: if face_index < kinds.len() {
                    kinds[face_index]
                } else {
                    "?"
                },
                triangles,
                vertices: positions.len(),
                aabb,
                area,
                max_triangle_edge: max_edge,
                centroid: Point3::from_vec(centroid / triangles as f64),
            });
        }
    }
    (prints, model_size, tolerance)
}

fn median(values: &mut [f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.sort_by(|a, b| a.partial_cmp(b).unwrap());
    values[values.len() / 2]
}

/// Rank the faces of `bad` that `good` does not account for.
///
/// Matching is by nearest centroid within a fraction of the model size. That is
/// crude on purpose: the two encodings are the same part, so a correct face has
/// a counterpart sitting in the same place with a similar size, and a blob face
/// either has no counterpart at all or one far smaller than itself. Anything
/// cleverer would need a correspondence the files do not provide.
fn differential(
    bad: &[FaceFingerprint],
    good: &[FaceFingerprint],
    model_size: f64,
) -> Vec<(f64, usize, String)> {
    let mut good_extent: Vec<f64> = good.iter().map(FaceFingerprint::extent).collect();
    let mut good_area: Vec<f64> = good.iter().map(|f| f.area).collect();
    let mut good_edge: Vec<f64> = good.iter().map(|f| f.max_triangle_edge).collect();
    let med_extent = median(&mut good_extent).max(f64::MIN_POSITIVE);
    let med_area = median(&mut good_area).max(f64::MIN_POSITIVE);
    let med_edge = median(&mut good_edge).max(f64::MIN_POSITIVE);

    // The good model's own bounding box. A face reaching outside it is the
    // single strongest signal: the part does not go there.
    let mut good_box = BoundingBox::<Point3>::new();
    for f in good {
        good_box.push(f.aabb.max());
        good_box.push(f.aabb.min());
    }
    let good_diag = if good_box.is_empty() {
        model_size
    } else {
        good_box.diameter()
    };
    let match_radius = good_diag * 0.02;

    let mut ranked = Vec::new();
    for (index, face) in bad.iter().enumerate() {
        // Nearest good face by centroid.
        let mut best: Option<(f64, &FaceFingerprint)> = None;
        for g in good {
            let d = g.centroid.distance(face.centroid);
            if best.map(|(bd, _)| d < bd).unwrap_or(true) {
                best = Some((d, g));
            }
        }
        let (distance, twin) = match best {
            Some(b) => b,
            None => continue,
        };
        let matched = distance <= match_radius;

        // How far outside the good model this face reaches, relative to the
        // good model's own size.
        let escape = [face.aabb.max(), face.aabb.min()]
            .iter()
            .map(|corner| {
                let mut worst: f64 = 0.0;
                for axis in 0..3 {
                    let lo = good_box.min()[axis];
                    let hi = good_box.max()[axis];
                    let v = corner[axis];
                    worst = worst.max((lo - v).max(v - hi).max(0.0));
                }
                worst
            })
            .fold(0.0, f64::max)
            / good_diag.max(f64::MIN_POSITIVE);

        let extent_excess = face.extent() / med_extent;
        let area_excess = face.area / med_area;
        let edge_excess = face.max_triangle_edge / med_edge;
        // A face with no counterpart is far more suspicious than a large one,
        // because the good encoding is the same part: something that exists
        // here and nowhere there was not in the design.
        let unmatched_penalty = if matched { 0.0 } else { 10.0 };
        // A face whose twin is much smaller is the classic blob: same place,
        // same intent, wildly more geometry.
        let twin_ratio = if matched {
            face.extent() / twin.extent().max(f64::MIN_POSITIVE)
        } else {
            1.0
        };

        // Absolute size is not evidence. The first version of this score summed
        // extent, area and edge excess, and ranked the box's own walls at the
        // top of every list: a wall really is the largest face on the model,
        // and being large is not being wrong. Those terms are kept for the
        // report but carry almost no weight.
        //
        // What discriminates is *disagreement with the oracle*: a face with no
        // counterpart in an equivalent encoding of the same part, or one whose
        // counterpart is far smaller. Those two terms decide the ranking, with
        // escape from the good bounding box as the tie-break.
        let score = escape * 100.0
            + unmatched_penalty * 10.0
            + (twin_ratio - 1.0).max(0.0) * 5.0
            + 0.01 * (extent_excess + area_excess + edge_excess);

        let detail = format!(
            "{:<9} tri={:<6} extent={:<10.5} area={:<11.5} maxedge={:<9.5} \
             escape={:<7.4} twin={} {}",
            face.surface_kind,
            face.triangles,
            face.extent(),
            face.area,
            face.max_triangle_edge,
            escape,
            if matched {
                format!("{:.2}x", twin_ratio)
            } else {
                "NONE".to_string()
            },
            face.provenance,
        );
        ranked.push((score, index, detail));
    }
    ranked.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
    ranked
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();
    let model = args
        .get(1)
        .cloned()
        .expect("usage: face_fingerprint MODEL.step [--against GOOD.step] [--csv]");
    let against = args
        .iter()
        .position(|a| a == "--against")
        .and_then(|i| args.get(i + 1).cloned());
    let csv = args.iter().any(|a| a == "--csv");
    let tolerance = args
        .iter()
        .position(|a| a == "--tolerance")
        .and_then(|i| args.get(i + 1))
        .and_then(|t| t.parse::<f64>().ok());

    let table = load(&model)?;
    let (bad, model_size, tol) = fingerprint(&table, tolerance);

    if csv {
        println!(
            "shell_entity,shell_index,use_id,definition_id,surface_id,kind,triangles,vertices,extent,area,max_edge,cx,cy,cz"
        );
        for f in &bad {
            println!(
                "{},{},{},{},{},{},{},{},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6}",
                f.shell_entity,
                f.shell_index,
                f.provenance.use_id.map(|i| i.get() as i64).unwrap_or(-1),
                f.provenance
                    .definition_id
                    .map(|i| i.get() as i64)
                    .unwrap_or(-1),
                f.provenance
                    .surface_id
                    .map(|i| i.get() as i64)
                    .unwrap_or(-1),
                f.surface_kind,
                f.triangles,
                f.vertices,
                f.extent(),
                f.area,
                f.max_triangle_edge,
                f.centroid.x,
                f.centroid.y,
                f.centroid.z,
            );
        }
        return Ok(());
    }

    println!(
        "{model}\n  {} faces meshed, model diameter {model_size:.5}, tolerance {tol:.6}",
        bad.len()
    );

    let Some(good_path) = against else {
        // No oracle: fall back to ranking against the model's own medians. Much
        // weaker — a model can be uniformly wrong — but it still surfaces the
        // one face that dwarfs its neighbours.
        let mut extents: Vec<f64> = bad.iter().map(FaceFingerprint::extent).collect();
        let med = median(&mut extents).max(f64::MIN_POSITIVE);
        let mut ranked: Vec<_> = bad.iter().map(|f| (f.extent() / med, f)).collect();
        ranked.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
        println!("\n  no oracle given; ranking by extent against this model's median face");
        for (ratio, f) in ranked.iter().take(15) {
            println!(
                "  {ratio:>8.2}x  {:<9} tri={:<6} extent={:<10.5} shell #{} {}",
                f.surface_kind,
                f.triangles,
                f.extent(),
                f.shell_entity,
                f.provenance
            );
        }
        return Ok(());
    };

    let good_table = load(&good_path)?;
    // The good encoding is meshed at *its own* tolerance, since the two
    // encodings may be in different units — several NIST pairs differ by
    // exactly 25.4 — and forcing one tolerance on both would mesh one of them
    // absurdly.
    let (good, good_size, good_tol) = fingerprint(&good_table, None);
    println!(
        "  oracle {good_path}\n  {} faces meshed, model diameter {good_size:.5}, tolerance {good_tol:.6}",
        good.len()
    );
    let scale = if good_size > 0.0 {
        model_size / good_size
    } else {
        1.0
    };
    println!("  size ratio bad/good = {scale:.5}  (25.4 or 1/25.4 means inch vs mm, not a defect)");

    let ranked = differential(&bad, &good, model_size);
    println!(
        "\n  ranked suspects (score = 50*escape + extents + area + edge + unmatched + twin ratio)\n"
    );
    for (score, index, detail) in ranked.iter().take(20) {
        let f = &bad[*index];
        println!("  {score:>9.2}  shell #{:<8} {detail}", f.shell_entity);
    }

    // The set worth removing first: everything scoring above an order of
    // magnitude more than the median suspect.
    let mut scores: Vec<f64> = ranked.iter().map(|(s, ..)| *s).collect();
    let med_score = median(&mut scores).max(f64::MIN_POSITIVE);
    let suspects: Vec<&(f64, usize, String)> = ranked
        .iter()
        .filter(|(s, ..)| *s > med_score * 10.0)
        .collect();
    println!(
        "\n  {} faces score >10x the median suspect; {} triangles of {} total ({:.1}%)",
        suspects.len(),
        suspects
            .iter()
            .map(|(_, i, _)| bad[*i].triangles)
            .sum::<usize>(),
        bad.iter().map(|f| f.triangles).sum::<usize>(),
        100.0
            * suspects
                .iter()
                .map(|(_, i, _)| bad[*i].triangles)
                .sum::<usize>() as f64
            / bad.iter().map(|f| f.triangles).sum::<usize>().max(1) as f64
    );
    let mut by_kind: HashMap<&str, usize> = HashMap::new();
    for (_, i, _) in &suspects {
        *by_kind.entry(bad[*i].surface_kind).or_default() += 1;
    }
    let mut kinds: Vec<_> = by_kind.into_iter().collect();
    kinds.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
    if !kinds.is_empty() {
        println!("  suspect surface kinds: {kinds:?}");
    }
    Ok(())
}
