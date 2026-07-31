//! Screen 2 — where every missing face went, as an ordered repair queue.
//!
//! "604 of 24202 faces produced no geometry" is a number to worry about, not a
//! number to act on. This turns it into a histogram by *reason*, so the largest
//! actionable category can be fixed, the census re-run, and the reduction
//! measured.
//!
//! The two populations are reported separately and never summed, because they
//! have different causes and different fixes:
//!
//! - **conversion losses** — the source face never became a face at all. The
//!   reason comes from `truck-stepio`'s own conversion path, so it cannot drift
//!   from what the renderer does.
//! - **tessellation losses** — the face converted and then produced no
//!   triangles, either with no surface or with an empty mesh. Grouped by surface
//!   kind, which is what makes them actionable: "227 unsurfaced" says nothing,
//!   "212 of 227 are NURBS" says where to look.
//!
//! ```console
//! cargo run --release --example face_census -- MODEL.step [MORE.step ...]
//! cargo run --release --example face_census -- --csv MODEL.step
//! ```

use std::collections::HashMap;
use std::env;

use truck_meshalgo::prelude::*;
use truck_stepio::r#in::{Table, step_geometry::Surface};

const RELATIVE_TOLERANCE: f64 = 0.001;
const DEGENERATE_TOLERANCE: f64 = 1.0e-3;
const MINIMUM_TOLERANCE: f64 = 1.0e-6;
const EDGE_SAMPLES: u32 = 4;

/// How a face ended up contributing nothing to the render.
///
/// One flat key so conversion and tessellation losses can share a histogram
/// while staying distinguishable — the prefix says which stage.
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct Bucket {
    stage: &'static str,
    reason: String,
    surface_kind: &'static str,
}

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

struct Census {
    declared: usize,
    rendered: usize,
    counts: HashMap<Bucket, usize>,
    /// A few example entities per bucket, so a category can be reproduced.
    examples: HashMap<Bucket, Vec<String>>,
}

impl Census {
    fn new() -> Self {
        Self {
            declared: 0,
            rendered: 0,
            counts: HashMap::new(),
            examples: HashMap::new(),
        }
    }

    fn record(&mut self, bucket: Bucket, example: Option<String>) {
        *self.counts.entry(bucket.clone()).or_default() += 1;
        if let Some(example) = example {
            let slot = self.examples.entry(bucket).or_default();
            if slot.len() < 3 {
                slot.push(example);
            }
        }
    }

    fn lost(&self) -> usize {
        self.counts.values().sum()
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

fn census(table: &Table, into: &mut Census) {
    // Every shell, converted once, keeping the loss reasons the conversion
    // produced. Tolerance is derived from the whole model exactly as the
    // renderer derives it, so "meshed to nothing" means the same thing here.
    let mut converted = Vec::new();
    for (_, shell) in table.shell.iter() {
        into.declared += shell.cfs_faces.len();
        if let Ok((cshell, losses)) = table.to_compressed_shell_with_losses(shell) {
            for loss in &losses {
                let example = loss.provenance.best_id().map(|id| id.to_string());
                into.record(
                    Bucket {
                        stage: "convert",
                        reason: loss.reason.tag().to_string(),
                        surface_kind: "-",
                    },
                    example,
                );
            }
            converted.push(cshell);
        }
    }

    let mut model = BoundingBox::<Point3>::new();
    for shell in &converted {
        for v in &shell.vertices {
            model.push(*v);
        }
        for edge in &shell.edges {
            let (a, b) = edge.curve.range_tuple();
            for i in 0..=EDGE_SAMPLES {
                model.push(
                    edge.curve
                        .subs(a + (b - a) * f64::from(i) / f64::from(EDGE_SAMPLES)),
                );
            }
        }
    }
    let scaled = model.diameter() * RELATIVE_TOLERANCE;
    let tolerance = if scaled.is_finite() && scaled > 0.0 {
        scaled.max(MINIMUM_TOLERANCE)
    } else {
        DEGENERATE_TOLERANCE
    };

    for shell in &converted {
        // Kinds are read before tessellation replaces the surface with a mesh.
        let kinds: Vec<&'static str> = shell
            .faces
            .iter()
            .map(|f| surface_kind(&f.surface))
            .collect();
        let meshed = shell.robust_triangulation(tolerance);
        for (i, face) in meshed.faces.iter().enumerate() {
            let kind = if i < kinds.len() { kinds[i] } else { "?" };
            let example = face.provenance.best_id().map(|id| id.to_string());
            match &face.surface {
                None => into.record(
                    Bucket {
                        stage: "tessellate",
                        reason: "NoSurfaceProduced".into(),
                        surface_kind: kind,
                    },
                    example,
                ),
                Some(mesh) if mesh.faces().is_empty() => into.record(
                    Bucket {
                        stage: "tessellate",
                        reason: "MeshedToNothing".into(),
                        surface_kind: kind,
                    },
                    example,
                ),
                Some(_) => into.rendered += 1,
            }
        }
    }
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();
    let csv = args.iter().any(|a| a == "--csv");
    let models: Vec<&String> = args.iter().filter(|a| !a.starts_with("--")).collect();
    if models.is_empty() {
        eprintln!("usage: face_census [--csv] MODEL.step [MORE.step ...]");
        return Ok(());
    }

    let mut total = Census::new();
    for model in &models {
        let mut one = Census::new();
        match load(model) {
            Ok(table) => census(&table, &mut one),
            Err(error) => {
                eprintln!("{model}: {error}");
                continue;
            }
        }
        if models.len() > 1 {
            println!(
                "{:<52} declared={:<7} lost={:<6} ({:.1}%)",
                std::path::Path::new(model)
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_default(),
                one.declared,
                one.lost(),
                100.0 * one.lost() as f64 / one.declared.max(1) as f64
            );
        }
        total.declared += one.declared;
        total.rendered += one.rendered;
        for (bucket, count) in one.counts {
            *total.counts.entry(bucket.clone()).or_default() += count;
            if let Some(examples) = one.examples.get(&bucket) {
                let slot = total.examples.entry(bucket).or_default();
                for e in examples {
                    if slot.len() < 3 {
                        slot.push(e.clone());
                    }
                }
            }
        }
    }

    let lost = total.lost();
    let mut ranked: Vec<(&Bucket, &usize)> = total.counts.iter().collect();
    ranked.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));

    if csv {
        println!("stage,reason,surface_kind,count,percent_of_lost,examples");
        for (bucket, count) in &ranked {
            println!(
                "{},{},{},{},{:.2},{}",
                bucket.stage,
                bucket.reason,
                bucket.surface_kind,
                count,
                100.0 * **count as f64 / lost.max(1) as f64,
                total
                    .examples
                    .get(*bucket)
                    .map(|e| e.join(" "))
                    .unwrap_or_default()
            );
        }
        return Ok(());
    }

    println!(
        "\n{} models, {} faces declared, {} rendered, {} lost ({:.2}%)",
        models.len(),
        total.declared,
        total.rendered,
        lost,
        100.0 * lost as f64 / total.declared.max(1) as f64
    );
    println!(
        "\n  {:<11} {:<28} {:<9} {:>7} {:>7}  examples",
        "stage", "reason", "surface", "count", "share"
    );
    for (bucket, count) in &ranked {
        println!(
            "  {:<11} {:<28} {:<9} {:>7} {:>6.1}%  {}",
            bucket.stage,
            bucket.reason,
            bucket.surface_kind,
            count,
            100.0 * **count as f64 / lost.max(1) as f64,
            total
                .examples
                .get(*bucket)
                .map(|e| e.join(" "))
                .unwrap_or_default()
        );
    }
    Ok(())
}
