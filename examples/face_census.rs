//! Screen 2 — face_census: Order-of-magnitude loss breakdown with explicit failure instrumentation.

use std::collections::HashMap;
use std::env;

use truck_meshalgo::prelude::*;
use truck_stepio::r#in::{Table, step_geometry::Surface};

const RELATIVE_TOLERANCE: f64 = 0.001;
const DEGENERATE_TOLERANCE: f64 = 1.0e-3;
const MINIMUM_TOLERANCE: f64 = 1.0e-6;
const EDGE_SAMPLES: u32 = 4;

/// How a face ended up contributing nothing to the render.
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

fn census(table: &Table, into: &mut Census, ledger: bool) {
    let mut converted = Vec::new();
    for (_, shell) in table.shell.iter() {
        into.declared += shell.cfs_faces.len();
        if let Ok((cshell, losses)) = table.to_compressed_shell_with_losses(shell) {
            for loss in &losses {
                let example = loss.provenance.best_id().map(|id| id.to_string());
                if ledger {
                    let id = loss
                        .provenance
                        .best_id()
                        .map(|id| id.to_string())
                        .unwrap_or_else(|| "-".into());
                    eprintln!(
                        "FACE\tdeclared_face_index=-\tsource_face_id={id}\t\
                         surface_kind=-\trendered=0\ttriangles=0\tstage=convert\treason={}",
                        loss.reason.tag()
                    );
                }
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
        let kinds: Vec<&'static str> = shell
            .faces
            .iter()
            .map(|f| surface_kind(&f.surface))
            .collect();
        // Must exercise the same path production does, or it measures a
        // build nobody ships (REFINEMENT_AUDIT.md section 6).
        let meshed = shell.robust_triangulation_with_lattice(tolerance, look::step_lattice_of);
        for (i, face) in meshed.faces.iter().enumerate() {
            let kind = if i < kinds.len() { kinds[i] } else { "?" };
            let example = face.provenance.best_id().map(|id| id.to_string());
            let (rendered, reason): (u8, &str) = match &face.surface {
                None => (0, "NoSurfaceProduced"),
                Some(mesh) if mesh.faces().is_empty() => (0, "MeshedToNothing"),
                Some(_) => (1, "-"),
            };
            // Audit A1 requires a per-face triangle delta, not only a
            // rendered/lost flag: the material-parity defect is expected to
            // change *which* triangles a face keeps far more often than it
            // changes whether the face survives at all. A face that renders
            // both before and after while retaining a different triangle count
            // is invisible to `rendered`.
            let triangles = face
                .surface
                .as_ref()
                .map_or(0, |mesh| mesh.tri_faces().len());
            if rendered == 0 {
                into.record(
                    Bucket {
                        stage: "tessellate",
                        reason: reason.into(),
                        surface_kind: kind,
                    },
                    example,
                );
            } else {
                into.rendered += 1;
            }
            if ledger {
                let id = face
                    .provenance
                    .best_id()
                    .map(|id| id.to_string())
                    .unwrap_or_else(|| "-".into());
                eprintln!(
                    "FACE\tdeclared_face_index={i}\tsource_face_id={id}\t\
                     surface_kind={kind}\trendered={rendered}\ttriangles={triangles}\t\
                     stage=tessellate\treason={reason}"
                );
            }
        }
    }
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();
    let csv = args.iter().any(|a| a == "--csv");
    let ledger = args.iter().any(|a| a == "--ledger");
    let models: Vec<&String> = args.iter().filter(|a| !a.starts_with("--")).collect();
    if models.is_empty() {
        eprintln!("usage: face_census [--csv] MODEL.step [MORE.step ...]");
        return Ok(());
    }

    let mut overall = Census::new();
    for path in &models {
        let table = load(path)?;
        census(&table, &mut overall, ledger);
    }

    let mut rows: Vec<(Bucket, usize)> = overall.counts.clone().into_iter().collect();
    rows.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

    if csv {
        println!("stage,reason,surface,count,share_pct,examples");
        for (bucket, count) in &rows {
            let share = *count as f64 / overall.declared as f64 * 100.0;
            let ex = overall
                .examples
                .get(bucket)
                .map(|v| v.join(" "))
                .unwrap_or_default();
            println!(
                "{},{},{},{},{:.1},{}",
                bucket.stage, bucket.reason, bucket.surface_kind, count, share, ex
            );
        }
    } else {
        println!(
            "{} models, {} faces declared, {} rendered, {} lost ({:.2}%)\n",
            models.len(),
            overall.declared,
            overall.rendered,
            overall.lost(),
            overall.lost() as f64 / overall.declared as f64 * 100.0
        );
        println!(
            "  {:11} {:28} {:10} {:5}   {:5}  examples",
            "stage", "reason", "surface", "count", "share"
        );
        for (bucket, count) in &rows {
            let share = *count as f64 / overall.declared as f64 * 100.0;
            let ex = overall
                .examples
                .get(bucket)
                .map(|v| v.join(" "))
                .unwrap_or_default();
            println!(
                "  {:11} {:28} {:10} {:5}  {:4.1}%  {}",
                bucket.stage, bucket.reason, bucket.surface_kind, count, share, ex
            );
        }
    }

    Ok(())
}
