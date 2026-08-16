//! STEP (ISO 10303-21) tessellation.
//!
//! STEP files usually describe solids as boundary representations — trimmed
//! analytic and NURBS surfaces — so there are no triangles to read out. Each
//! shell is evaluated and meshed, then flattened into the same triangle soup
//! the STL path produces, which lets the rest of the pipeline stay unaware of
//! STEP. AP242 also allows a file to ship its mesh directly, which is handled
//! by [`tessellated`].

pub mod appearance;
pub mod circular_arc;
pub mod cone;
pub mod cylinder;
pub mod lattice;
pub mod meshing_policy;
pub mod part21;
pub mod policy_geometry;
pub mod spline_carrier;
mod tessellated;
pub mod torus_deck;

use std::collections::{BTreeMap, HashMap};
use std::time::Instant;

use rayon::prelude::*;
use truck_assembly::assy::*;
use truck_meshalgo::prelude::*;
use truck_meshalgo::tessellation::diagnosis as truck_diag;
use truck_meshalgo::tessellation::{TessellationFailureReason, TorusAnnulusAttempt};
use truck_stepio::common::PartAttrs;
use truck_stepio::r#in::Table;
use truck_stepio::r#in::convert::{FaceLossReason, NodeMatrix, ProductShape};
use truck_stepio::r#in::ruststep::{ast::Name, tables::PlaceHolder};
use truck_stepio::r#in::step_geometry::{Curve3D, Surface};
use truck_topology::compress::{CompressedShell, FaceProvenance, SourceEntityId};

use crate::timing::Timings;

/// Tolerance for a model with no measurable extent, where there is nothing to
/// scale against. Nothing renders from such a shell anyway; this only keeps a
/// degenerate bounding box from producing a zero or non-finite tolerance.
const DEGENERATE_TOLERANCE: f64 = 1.0e-3;

/// Chord deviation as a fraction of a shell's diagonal. Scaling to the model
/// keeps output density stable across files that use different units.
#[allow(dead_code)]
const RELATIVE_TOLERANCE: f64 = 0.001;

/// How many points to sample along each edge when measuring the model.
///
/// Enough to catch the mid-span bulge of a circle or a spline, which is where
/// a vertex-only box loses the most; more would not change the diameter.
const EDGE_SAMPLES: u32 = 4;

/// truck panics below this, and it is not a tolerance we could honour anyway.
///
/// This is not the millimetre-scale floor the comment on [`model_tolerance`]
/// argues against: it is the hard minimum truck's own division asserts on. A
/// model whose measured extent is tiny — or an isolated shell pulled out of a
/// larger assembly — otherwise computes a tolerance below it and aborts.
const MINIMUM_TOLERANCE: f64 = 1.0e-6;

/// How many lost-face identities one shell contributes to the report.
///
/// Per-shell rather than global so a model whose loss is spread thinly still
/// names faces from several shells instead of exhausting the budget on the
/// first one that fails.
const LOST_FACE_IDS_PER_SHELL: usize = 4;

/// How many lost-face identities the warning prints in total.
///
/// A diagnostic that floods stderr gets piped to `/dev/null`, and then it
/// diagnoses nothing. The count is always exact; only the naming is capped.
const LOST_FACE_IDS_REPORTED: usize = 12;

/// One structured, machine-readable record for a source face lost during STEP
/// conversion, before it ever reached tessellation.
///
/// Component-owned by the import boundary (Look, not truck-meshalgo): it uses
/// the same event conventions as truck's `FaceDiagnosticRecord` — the shared
/// schema-versioning discipline, the default-stderr emission, the
/// `TRUCK_FACE_DIAG_JSONL` redirect and the `TRUCK_FACE_DIAG=off` suppression —
/// but is deliberately a conversion record, never a tessellation record. It is
/// the answer to "this source face existed, and no output face carries its
/// provenance".
#[derive(Clone, Debug, serde::Serialize)]
pub struct ImportDiagnosticRecord {
    /// Schema version, matching truck's diagnostic schema version.
    pub schema_version: u32,
    /// The document/model identifier.
    pub document_id: Option<String>,
    /// The source face entity id, when the reference chain resolved that far.
    pub source_face_id: Option<u64>,
    /// The source face *use* id, when available.
    pub source_use_id: Option<u64>,
    /// The coarse entity kind the resolved reference named.
    pub source_entity_type: Option<String>,
    /// Which conversion stage the face was lost in.
    pub conversion_stage: &'static str,
    /// The coarse conversion failure kind.
    pub conversion_failure_kind: &'static str,
    /// The STEP shell entity the face belonged to.
    pub representation_shell_context: Option<u64>,
    /// The exact typed refusal, from the converter.
    pub refusal_tag: String,
    /// Whether any FaceProvenance was established before the loss.
    pub provenance_established: bool,
    /// Faces the shell declared before conversion.
    pub declared_shell_faces: usize,
    /// Faces that survived conversion.
    pub surviving_shell_faces: usize,
}

/// The coarse conversion stage a [`FaceLossReason`] happened in.
fn conversion_stage_of(reason: FaceLossReason) -> &'static str {
    use FaceLossReason as R;
    match reason {
        R::FaceReferenceUnresolved => "face_lookup",
        R::SurfaceConversionFailed => "surface_conversion",
        R::BoundReferenceUnresolved | R::LoopReferenceUnresolved | R::AllBoundsCollapsed => {
            "bound_conversion"
        }
        R::EdgeUseUnresolved | R::EdgeCurveConversionFailed | R::WireNotClosed => "edge_conversion",
    }
}

/// The coarse entity kind a resolved face provenance names, when it names one.
fn provenance_entity_type(provenance: &FaceProvenance) -> Option<String> {
    if provenance.definition_id.is_some() {
        Some("face".into())
    } else if provenance.use_id.is_some() {
        Some("oriented_face".into())
    } else if provenance.surface_id.is_some() {
        Some("surface".into())
    } else {
        None
    }
}

/// Emit exactly one [`ImportDiagnosticRecord`] for one converted-away face,
/// through truck's shared diagnostic sink.
fn emit_import_diagnostic(
    shell_id: u64,
    declared: usize,
    surviving: usize,
    loss: &truck_stepio::r#in::convert::FaceLoss,
) {
    let line = match serde_json::to_string(&ImportDiagnosticRecord {
        schema_version: truck_diag::DIAGNOSTIC_SCHEMA_VERSION,
        document_id: truck_diag::document_context(),
        source_face_id: loss.provenance.best_id().map(SourceEntityId::get),
        source_use_id: loss.provenance.use_id.map(SourceEntityId::get),
        source_entity_type: provenance_entity_type(&loss.provenance),
        conversion_stage: conversion_stage_of(loss.reason),
        conversion_failure_kind: "conversion_refusal",
        representation_shell_context: Some(shell_id),
        refusal_tag: loss.reason.tag().to_string(),
        provenance_established: !loss.provenance.is_empty(),
        declared_shell_faces: declared,
        surviving_shell_faces: surviving,
    }) {
        Ok(line) => line,
        Err(_) => return,
    };
    truck_diag::emit_json_line(&line);
}

/// Positions, indices, and per-vertex RGBA for one tessellated STEP model.
///
/// The colour vector is positionally aligned with `positions`: the third entry
/// of the first is the colour of the third entry of the second. Vertices are
/// unwelded, so every vertex of one source face carries that face's colour.
pub type StepTriangleSoup = (Vec<[f32; 3]>, Vec<u32>, Vec<[f32; 4]>);

/// One unique definition's tessellated geometry, definition-local.
pub struct AssemblyDefinition {
    /// The source product name this definition belongs to.
    pub node_name: String,
    pub soup: StepTriangleSoup,
}

/// One source occurrence: which definition to render and where in world space.
pub struct AssemblyOccurrence {
    /// Index into [`StepAssemblyScene::definitions`].
    pub definition: usize,
    /// The world transform `T_world = T_world(parent) * T_local(child)` from
    /// Truck's `Path::matrix()`, applied exactly once by the renderer.
    pub world: glam::Mat4,
    pub node_name: String,
}

/// The resolved shape of an assembly-bearing STEP document.
///
/// Definition geometry stays definition-local and is shared by every
/// occurrence that references it; occurrence placement is applied at render
/// time, once, and never baked into vertices.
pub struct StepAssemblyScene {
    pub definitions: Vec<AssemblyDefinition>,
    pub occurrences: Vec<AssemblyOccurrence>,
    /// Product definitions in the assembly graph (geometry-bearing or not).
    pub nodes: usize,
}

/// Which scene shape a STEP document has.
pub enum StepScene {
    /// No meaningful assembly occurrence graph: today's flat single-part path.
    Flat(StepTriangleSoup),
    /// A source assembly graph, resolved into definitions and occurrences.
    Assembly(StepAssemblyScene),
}

/// Parse a STEP document into the scene shape its source actually declares.
///
/// A file with no `NEXT_ASSEMBLY_USAGE_OCCURRENCE`, or whose assembly graph
/// cannot be built, keeps the ordinary single-part path untouched.
pub fn parse_step_scene(bytes: &[u8], timings: &mut Timings) -> anyhow::Result<StepScene> {
    let (table, text) = parse_step_table_input(bytes, timings)?;

    // Only attempt the assembly path on a file that declares occurrences; a
    // plain part file has no graph and must stay exactly where it was.
    if table.next_assembly_usage_occurrence.is_empty() {
        return parse_step_table(table, &text, timings).map(StepScene::Flat);
    }

    match parse_step_table_assembly(table, timings) {
        Ok(scene) if !scene.occurrences.is_empty() => Ok(StepScene::Assembly(scene)),
        // A file that names occurrences but whose graph does not resolve, or
        // resolves to nothing renderable, degrades to the flat single-part
        // path — today's behavior — rather than failing the render.
        Ok(_) | Err(_) => {
            let (table, text) = parse_step_table_input(bytes, timings)?;
            parse_step_table(table, &text, timings).map(StepScene::Flat)
        }
    }
}

/// Tessellate every shell in a STEP file into one indexed triangle mesh.
///
/// Positions carry no shared topology between faces, so each triangle brings
/// its own vertices and normals are generated by the caller.
pub fn parse_step(bytes: &[u8], timings: &mut Timings) -> anyhow::Result<StepTriangleSoup> {
    let (table, text) = parse_step_table_input(bytes, timings)?;
    parse_step_table(table, &text, timings)
}

/// The STEP table for a document, parsed once and handed to whichever scene
/// shape the source actually has (flat single part or assembly graph). The
/// decoded text is returned too: an AP242 file that ships a pre-triangulated
/// mesh has no shells, and reading the tree again is how its triangles are
/// reached.
fn parse_step_table_input(bytes: &[u8], timings: &mut Timings) -> anyhow::Result<(Table, String)> {
    let text = std::str::from_utf8(bytes)
        .map(std::borrow::Cow::Borrowed)
        // STEP is nominally ASCII, but exporters emit Latin-1 in comments and
        // names often enough that rejecting the file would be unhelpful.
        .unwrap_or_else(|_| bytes.iter().map(|&b| b as char).collect::<String>().into());

    let parse_started = Instant::now();
    let mut exchange = read_exchange(&text)?;
    timings.record("step_parse", parse_started.elapsed());

    if exchange.data.is_empty() {
        anyhow::bail!("STEP file contains no data section");
    }
    let section = exchange.data.swap_remove(0);

    // Hand the syntax tree over rather than lending it. The tree is roughly
    // eight times the size of the file and the table another three, so
    // building the table from a borrowed tree keeps both fully resident and
    // makes peak memory, not speed, the limit on how large a model will load.
    let table_started = Instant::now();
    let table = Table::from_owned_data_section(section);
    timings.record("step_table", table_started.elapsed());
    Ok((table, text.into_owned()))
}

/// Tessellate every shell in a STEP file into one indexed triangle mesh.
///
/// Positions carry no shared topology between faces, so each triangle brings
/// its own vertices and normals are generated by the caller.
fn parse_step_table(
    table: Table,
    text: &str,
    timings: &mut Timings,
) -> anyhow::Result<StepTriangleSoup> {
    // The source-declared spline-axis closure of every spline surface entity,
    // read from the STEP table before conversion erases it. Keyed by surface
    // entity id, the same id `FaceProvenance.surface_id` carries per face, so
    // `wrap_shell_with_closure` can attach it to each face's `PolicySurface`.
    let closure_map = lattice::spline_closure_map(&table);

    // Effective face appearance, keyed by the source face entity id —
    // `FaceProvenance.definition_id`. Resolved before any shell is converted so
    // the per-face flatten below can attach colours without touching geometry.
    let (face_appearances, unresolved_styles) = appearance::resolve_face_appearances(&table);

    if table.shell.is_empty() {
        // AP242 lets a file ship its mesh directly instead of a boundary
        // representation, in which case there is nothing to evaluate and the
        // triangles are read straight out of the syntax tree. That tree was
        // consumed above, which is the right trade for the boundary
        // representations that are almost every file; these have to read it
        // again. They are rare and small, and parsing is no longer the
        // expensive part of loading one.
        let tessellated_started = Instant::now();
        let reread = read_exchange(text)?;
        let tessellated = reread.data.first().and_then(tessellated::read);
        timings.record("step_tessellated_read", tessellated_started.elapsed());
        if let Some((positions, indices)) = tessellated {
            // AP242 triangulated data carries no STEP presentation, so every
            // vertex keeps the identity colour and the default material rules.
            let colors = vec![[1.0_f32; 4]; positions.len()];
            return Ok((positions, indices, colors));
        }
        anyhow::bail!("STEP file contains no shells or tessellated faces");
    }

    // Resolve the entity graph first. Tolerance is derived from the whole
    // model rather than per shell: a shell-local tolerance meshes a part split
    // into many small shells far finer than the same part as one shell, and a
    // degenerate shell has no extent to scale by at all.
    let mesh_started = Instant::now();
    let shells = table
        .shell
        .par_iter()
        .map(|(id, shell)| {
            // The declared count is read from the source holder, before
            // conversion, because conversion is allowed to drop a face and
            // nothing downstream can tell that it did.
            let declared = shell.cfs_faces.len();
            table
                .to_compressed_shell_with_losses(*id, shell)
                .map(|(compressed, losses)| {
                    // DIAG-002: every face lost during conversion emits exactly
                    // one import diagnostic. The hard invariant: a source face
                    // that existed and produced no output face/provenance must
                    // leave an explanation here, in the same sink the
                    // tessellation records use.
                    let surviving = compressed.faces.len();
                    for loss in &losses {
                        emit_import_diagnostic(*id, declared, surviving, loss);
                    }
                    (declared, compressed)
                })
                .map_err(|error| format!("shell #{id}: {error}"))
        })
        .collect::<Vec<_>>();

    let mut model = BoundingBox::<Point3>::new();
    for (_, shell) in shells.iter().flatten() {
        for vertex in &shell.vertices {
            model.push(*vertex);
        }
        // Vertices alone understate the model, often badly. A full circle
        // carries a single topological vertex, so a disc or a cylinder adds
        // almost nothing to the box while occupying real space: a washer whose
        // three vertices span 0.2 mm measured as 0.2 mm across, and the whole
        // submarine model measured 3.06 against rendered bounds of 7.33.
        // Since the tolerance is a fraction of this diameter, understating it
        // meshes every model finer than asked. Sampling each edge closes the
        // gap for the cost of a few curve evaluations per edge.
        for edge in &shell.edges {
            let (start, end) = edge.curve.range_tuple();
            for step in 0..=EDGE_SAMPLES {
                let t = start + (end - start) * f64::from(step) / f64::from(EDGE_SAMPLES);
                model.push(edge.curve.subs(t));
            }
        }
    }
    let tolerance = model_tolerance(model.diameter());
    // The targeted linear-plus-angular meshing policy. The scalar `tolerance`
    // remains the geometric-error bound Truck receives; the policy's angular
    // floor is applied per feature by `policy_geometry`'s wrappers, which
    // override only `parameter_division` and delegate everything else to
    // Truck unchanged. See `step/meshing_policy.rs`.
    let policy = meshing_policy::MeshingPolicy::DEFAULT;

    // Shells are independent, so tessellation parallelizes cleanly. A shell
    // that fails is reported rather than silently dropped, so a partial render
    // never passes for a complete one.
    let meshed = shells
        .into_par_iter()
        .map(|shell| {
            let (declared, shell) = shell?;
            mesh_shell(declared, shell, tolerance, &closure_map, policy)
        })
        .collect::<Vec<_>>();
    timings.record("step_tessellate", mesh_started.elapsed());

    let mut positions = Vec::new();
    let mut indices = Vec::new();
    let mut colors = Vec::new();
    let mut failures = Vec::new();
    let mut faces = FaceTally::default();
    let mut lost_faces: Vec<FaceProvenance> = Vec::new();
    let mut reason_counts: BTreeMap<TessellationFailureReason, usize> = BTreeMap::new();
    let mut torus_census_total: BTreeMap<&'static str, usize> = BTreeMap::new();
    for result in meshed {
        match result {
            Ok(mesh) => {
                // One face at a time, still keyed by source provenance, so the
                // effective appearance joins on `definition_id` without
                // touching how the face was meshed. Faces absent from the map
                // keep the identity colour and the default material.
                for (provenance, polygon) in mesh.faces {
                    let color = provenance
                        .definition_id
                        .map(SourceEntityId::get)
                        .and_then(|id| face_appearances.get(&id))
                        .map(|appearance| appearance.color)
                        .unwrap_or([1.0; 4]);
                    append_polygon(&polygon, color, &mut positions, &mut indices, &mut colors);
                }
                faces.declared += mesh.tally.declared;
                faces.total += mesh.tally.total;
                faces.unsurfaced += mesh.tally.unsurfaced;
                faces.empty += mesh.tally.empty;
                for reason in mesh.reasons {
                    *reason_counts.entry(reason).or_insert(0) += 1;
                }
                for (tag, count) in mesh.torus_census {
                    *torus_census_total.entry(tag).or_insert(0) += count;
                }
                if lost_faces.len() < LOST_FACE_IDS_REPORTED {
                    lost_faces.extend(mesh.lost_ids);
                }
            }
            Err(error) => failures.push(error),
        }
    }

    if positions.is_empty() {
        anyhow::bail!(
            "STEP tessellation produced no triangles{}",
            describe(&failures)
        );
    }
    report_step_losses(
        &failures,
        table.shell.len(),
        &faces,
        &lost_faces,
        &reason_counts,
        &torus_census_total,
        &unresolved_styles,
    );

    Ok((positions, indices, colors))
}

/// Report every loss a STEP tessellation incurred, in the shape the flat path
/// has always used, so both the flat and assembly paths speak the same
/// diagnostic language.
#[allow(clippy::too_many_arguments)]
fn report_step_losses(
    failures: &[String],
    total_shells: usize,
    faces: &FaceTally,
    lost_faces: &[FaceProvenance],
    reason_counts: &BTreeMap<TessellationFailureReason, usize>,
    torus_census_total: &BTreeMap<&'static str, usize>,
    unresolved_styles: &[appearance::UnresolvedStyle],
) {
    if !failures.is_empty() {
        // Loud on stderr rather than a hard failure: a mostly-complete render
        // is useful, but the user must know it is incomplete.
        eprintln!(
            "warning: {} of {} STEP shells failed to tessellate{}",
            failures.len(),
            total_shells,
            describe(failures)
        );
    }
    if faces.lost() > 0 {
        // A shell can succeed while individual faces within it do not, and
        // those vanish from the mesh silently. Without this a model missing
        // geometry is indistinguishable from a complete one. The split is
        // reported because it points at which defect to chase.
        eprintln!(
            "warning: {} of {} STEP faces produced no geometry and are missing \
             from the render ({} failed to convert, {} had no surface, \
             {} meshed to nothing)",
            faces.lost(),
            faces.declared,
            faces.unconverted(),
            faces.unsurfaced,
            faces.empty
        );
        // G8: the tessellator's own reason for each refusal. Previously every
        // one of these arrived as an empty mesh and the cause had to be guessed
        // from which bucket the face fell into. Ordered by count so the largest
        // population names itself first.
        if !reason_counts.is_empty() {
            let mut by_size: Vec<_> = reason_counts.iter().collect();
            by_size.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
            let summary = by_size
                .iter()
                .map(|(reason, count)| format!("{reason:?} x{count}"))
                .collect::<Vec<_>>()
                .join(", ");
            eprintln!("         tessellation refused: {summary}");
        }
        // Naming even a few of them turns "geometry is missing somewhere" into
        // a place to start: every id here can be grepped straight out of the
        // source file, and the chain says which layer is meant.
        //
        // Only *tessellation-stage* losses can appear. A face lost during
        // conversion never became a face object, so nothing survives to carry
        // its provenance — which is why the count below is deliberately split
        // rather than presented as one total with a dozen examples. Saying "and
        // N more" over a mixed population would imply every unnamed face
        // reached the stage that could have named it.
        if !lost_faces.is_empty() {
            let named = faces.unsurfaced + faces.empty;
            let shown = lost_faces
                .iter()
                .take(LOST_FACE_IDS_REPORTED)
                .map(FaceProvenance::to_string)
                .collect::<Vec<_>>()
                .join("; ");
            let unnamed = named.saturating_sub(lost_faces.len().min(LOST_FACE_IDS_REPORTED));
            eprintln!("         of the {named} lost after conversion: {shown}");
            if unnamed > 0 {
                eprintln!("         and {unnamed} more not listed");
            }
            if faces.unconverted() > 0 {
                eprintln!(
                    "         the {} lost during conversion cannot be named yet \
                     — no face object exists to carry their provenance",
                    faces.unconverted()
                );
            }
        }
    }

    // Torus observer census: the typed outcome of every face the torus annulus
    // route attempted, across all shells. Printed unconditionally when the
    // torus probe or recovery gate is active, so the census can be confirmed
    // against the corrected expected counts.
    if !torus_census_total.is_empty() {
        let mut by_size: Vec<_> = torus_census_total.iter().collect();
        by_size.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
        let summary = by_size
            .iter()
            .map(|(tag, count)| format!("{tag}={count}"))
            .collect::<Vec<_>>()
            .join(", ");
        let total: usize = torus_census_total.values().sum();
        eprintln!("torus observer: {total} faces attempted, {summary}");
    }

    // Presentation diagnostics: styled items that targeted a face or shape but
    // whose style chain produced no colour. A model that mixes styled and
    // unstyled faces is normal and silent; only an asserted style that could
    // not be honoured is named.
    if !unresolved_styles.is_empty() {
        let shown = unresolved_styles
            .iter()
            .take(LOST_FACE_IDS_REPORTED)
            .map(|unresolved| {
                format!(
                    "#{} ({}) {}",
                    unresolved.entity_id, unresolved.kind, unresolved.reason
                )
            })
            .collect::<Vec<_>>()
            .join("; ");
        let unnamed = unresolved_styles
            .len()
            .saturating_sub(LOST_FACE_IDS_REPORTED);
        eprintln!(
            "warning: {} STEP styled item chain(s) did not resolve to a face \
             colour{}{}",
            unresolved_styles.len(),
            if shown.is_empty() {
                String::new()
            } else {
                format!(": {shown}")
            },
            if unnamed > 0 {
                format!("; and {unnamed} more not listed")
            } else {
                String::new()
            }
        );
    }
}

/// Mesh one compressed shell with the source policy, collecting every
/// tessellation verdict the caller reports.
///
/// Shared by the flat and assembly STEP paths so the tessellation machinery —
/// the policy wrapper, the outcome-preserving torus route, the face tally, the
/// typed refusal reasons, and the torus observer census — has exactly one
/// implementation. Only the scene organization differs.
fn mesh_shell(
    declared: usize,
    shell: CompressedShell<Point3, Curve3D, Surface>,
    tolerance: f64,
    closure_map: &std::collections::HashMap<u64, lattice::SplineAxisClosure>,
    policy: meshing_policy::MeshingPolicy,
) -> Result<StepShellMesh, String> {
    // Only a shell with no faces carries nothing to render. Counting
    // *vertices* was wrong: a disc or a cylinder is bounded by full
    // circles, which have one topological vertex each, so this
    // discarded valid solids in silence. A 74-entity washer that OCCT
    // renders fine reached look as "tessellation produced no
    // triangles" for exactly this reason.
    if shell.faces.is_empty() {
        // Still report what the file declared. A shell whose faces all
        // failed to convert arrives here looking identical to a shell
        // that genuinely holds none.
        return Ok(StepShellMesh {
            faces: Vec::new(),
            tally: FaceTally {
                declared,
                ..FaceTally::default()
            },
            // Nothing survived to carry an identity: these faces were
            // lost during conversion, before any face object existed.
            lost_ids: Vec::new(),
            // And nothing reached the tessellator, so it has no verdict
            // to offer. An empty list here is not "no failures" — it is
            // "this loss happened upstream of the stage that names
            // reasons", which `unconverted()` already reports.
            reasons: Vec::new(),
            // No torus outcomes: nothing reached the tessellator.
            torus_census: BTreeMap::new(),
        });
    }
    // Periodicity now reaches the tessellator as a descriptor built
    // from the concrete STEP representation, not as a bare accessor
    // result. See src/step/lattice.rs and REFINEMENT_AUDIT.md §6.
    //
    // G8: through the outcome-preserving entry point. The tessellator
    // builds a typed `TessellationFailure` for every face it refuses —
    // including `ContradictoryDualParity`, which is a *proved*
    // inconsistency — and the mesh-only entry point destroyed it one
    // line after constructing it. The reasons now arrive beside the
    // shell, so a face that produced nothing can say why rather than
    // being inferred from the shape of its absence.
    let outcome = policy_geometry::wrap_shell_with_closure(shell, policy, closure_map)
        .robust_triangulation_with_torus_outcome(
            tolerance,
            |s: &policy_geometry::PolicySurface| {
                lattice::lattice_of_with_closure(s.inner(), s.source_closure())
            },
            |s: &policy_geometry::PolicySurface| lattice::support_schema_of(s.inner()),
            |c: &policy_geometry::PolicyCurve| lattice::curve_schema_of(c.inner()),
            |s: &policy_geometry::PolicySurface| cylinder::identify_source_cylinder_opt(s.inner()),
            |c: &policy_geometry::PolicyCurve| lattice::cylinder_curve_schema_of(c.inner()),
            |c: &policy_geometry::PolicyCurve| lattice::cylinder_curve_family_of(c.inner()),
            |s: &policy_geometry::PolicySurface| cone::identify_source_cone_opt(s.inner()),
            |s: &policy_geometry::PolicySurface| torus_deck::identify_source_torus_opt(s.inner()),
        );
    let meshed = outcome.shell;
    // A face that could not be meshed is dropped from the polygon
    // without comment, so count them here while the structure still
    // says which is which. Malformed geometry does reach this: a wire
    // that bounds nothing leaves its face untessellated.
    //
    // There are two ways to lose a face and they do not have the same
    // cause, so they are not summed. `None` is a surface that could not
    // be produced at all, which on real files is mostly a spline
    // failing to converge. `Some` holding no triangles is a surface
    // that meshed to nothing, which happens to *planes* — a separate
    // and unexplained defect. Counting only the first understated the
    // loss on 00009190 by 118 faces, 276 against 394.
    let mut tally = FaceTally {
        declared,
        total: meshed.faces.len(),
        ..FaceTally::default()
    };
    // The entity behind each loss, so the warning can name faces
    // instead of only counting them. Capped: a model that loses
    // thousands does not need thousands of ids on stderr to be
    // diagnosed, and the cap keeps this bounded on the worst input
    // rather than the typical one.
    let mut lost_ids = Vec::new();
    // Why each lost face was lost, by reason. This replaces inference
    // from the *shape* of the loss with the tessellator's own verdict;
    // the `unsurfaced`/`empty` split is kept beside it because the two
    // do not partition the same way — a face can be `Some(empty)` for
    // several distinct reasons.
    let mut reasons: Vec<TessellationFailureReason> = Vec::new();
    for (face, failure) in meshed.faces.iter().zip(&outcome.face_failures) {
        let lost = match &face.surface {
            None => {
                tally.unsurfaced += 1;
                true
            }
            Some(mesh) if mesh.faces().is_empty() => {
                tally.empty += 1;
                true
            }
            Some(_) => false,
        };
        if let Some(failure) = failure {
            reasons.push(failure.reason);
        }
        if lost && lost_ids.len() < LOST_FACE_IDS_PER_SHELL && !face.provenance.is_empty() {
            lost_ids.push(face.provenance);
        }
    }
    // Torus observer: count the typed outcome of each face the torus
    // annulus route attempted. This runs in shadow (TRUCK_PROBE_TORUS)
    // or recovery (TRUCK_FORMAL_RECOVERY_TORUS) mode; in shadow the
    // mesh is not replaced, so the counts reproduce the corrected
    // census without changing production output.
    let mut torus_census: BTreeMap<&'static str, usize> = BTreeMap::new();
    for attempt in &outcome.torus_band_attempts {
        let tag = match attempt {
            Some(TorusAnnulusAttempt::Recovered {
                conformance, ..
            }) => match conformance {
                truck_meshalgo::tessellation::formal::ConformanceTag::SourceClean => {
                    "recovered_clean"
                }
                truck_meshalgo::tessellation::formal::ConformanceTag::MalformedTwoOuterBoundsOnCertifiedTorusAnnulus => {
                    "recovered_malformed"
                }
            },
            Some(TorusAnnulusAttempt::Refused(exit)) => exit.tag(),
            None => continue,
        };
        *torus_census.entry(tag).or_insert(0) += 1;
    }
    // Per-face polygons, kept with their source provenance and in the
    // same order `MeshedShape::to_polygon` would merge them, so the
    // flatten stage reproduces the identical triangle soup while
    // attaching appearance per face.
    let faces = meshed
        .faces
        .iter()
        .filter_map(|face| {
            let surface = face.surface.as_ref()?;
            let polygon = if face.orientation {
                surface.clone()
            } else {
                surface.inverse()
            };
            Some((face.provenance, polygon))
        })
        .collect();
    Ok(StepShellMesh {
        faces,
        tally,
        lost_ids,
        reasons,
        torus_census,
    })
}

/// Resolve the assembly graph into unique definition geometry and occurrences.
///
/// Truck owns every semantic step: the source product/occurrence
/// relationships, the `ITEM_DEFINED_TRANSFORMATION` placement direction, the
/// source-linked definition BREPs, and the `Path::matrix()` world composition.
/// Look only tessellates each unique definition once and places occurrences.
fn parse_step_table_assembly(
    table: Table,
    timings: &mut Timings,
) -> anyhow::Result<StepAssemblyScene> {
    let closure_map = lattice::spline_closure_map(&table);
    let (face_appearances, unresolved_styles) = appearance::resolve_face_appearances(&table);

    let assy = table
        .step_assy()
        .map_err(|error| anyhow::anyhow!("failed to build the STEP assembly graph: {error}"))?;
    // A degenerate source placement is the one way an assembly edge transform
    // can fail; check before mapping so no closure unwraps.
    for edge in assy.all_edges() {
        Matrix4::try_from(edge.matrix()).map_err(|error| {
            anyhow::anyhow!("a STEP assembly edge transform is degenerate: {error}")
        })?;
    }
    // The node keeps only its source shell ids. Truck's node shape also carries
    // the converted `CompressedSolid`, but Look never reads it — every shell is
    // re-converted from the table below, for the declared-face count and the
    // DIAG-002 loss stream that conversion emits. Mapping to the ids alone
    // keeps `Dag::map` from deep copying every solid in the assembly, vertices
    // and curves and surfaces, to be dropped unread.
    let mapped = assy.map(
        |node: &NodeEntity<Vec<ProductShape>, PartAttrs>| NodeEntity {
            shape: node
                .shape
                .iter()
                .filter_map(|shape| match shape {
                    ProductShape::Solid(_, ids) | ProductShape::Shells(_, ids) => Some(ids),
                    ProductShape::Matrix(_) => None,
                })
                .flatten()
                .copied()
                .collect::<Vec<u64>>(),
            attrs: node.attrs.clone(),
        },
        |edge: &EdgeEntity<NodeMatrix, PartAttrs>| EdgeEntity {
            matrix: Matrix4::try_from(&edge.matrix).unwrap(),
            attrs: edge.attrs.clone(),
        },
    );

    // Per node (in graph index order): the definition's source shell entity
    // ids. A subassembly node carries none and renders nothing of its own.
    let mut node_names = Vec::with_capacity(mapped.len());
    let mut definition_shells = Vec::with_capacity(mapped.len());
    for node in mapped.all_nodes() {
        node_names.push(node.entity().attrs.name.clone());
        definition_shells.push(node.shape().clone());
    }

    // One task per *unique* shell. Two definition nodes that name the same
    // source shell are the same geometry both times, so converting and meshing
    // it twice is duplicated work; the per-definition flatten below still
    // appends it once per definition, so every soup and every tally is what it
    // was. Only the conversion warnings and the DIAG-002 records change: a
    // shell that fails now reports once, for the one conversion that ran.
    let mut tasks = definition_shells
        .iter()
        .flatten()
        .copied()
        .collect::<Vec<u64>>();
    tasks.sort_unstable();
    tasks.dedup();
    if tasks.is_empty() {
        anyhow::bail!("STEP assembly declares occurrences but no definition geometry");
    }

    // Convert every definition shell exactly once, emitting the DIAG-002
    // conversion-loss stream exactly as the flat path does. The conversion is
    // the same one Truck ran to attach the geometry, so the tessellated result
    // is the definition geometry the graph exposed.
    let mesh_started = Instant::now();
    let converted = tasks
        .par_iter()
        .map(|&shell_id| {
            convert_definition_shell(&table, shell_id)
                .map(|(declared, shell)| (shell_id, declared, shell))
        })
        .collect::<Vec<_>>();

    // Model tolerance from the definition geometry that will actually render.
    let mut model = BoundingBox::<Point3>::new();
    let mut converted_ok = Vec::with_capacity(converted.len());
    for entry in converted {
        match entry {
            Ok((shell_id, declared, shell)) => {
                for vertex in &shell.vertices {
                    model.push(*vertex);
                }
                for edge in &shell.edges {
                    let (start, end) = edge.curve.range_tuple();
                    for step in 0..=EDGE_SAMPLES {
                        let t = start + (end - start) * f64::from(step) / f64::from(EDGE_SAMPLES);
                        model.push(edge.curve.subs(t));
                    }
                }
                converted_ok.push((shell_id, declared, shell));
            }
            Err(error) => eprintln!("warning: {error}"),
        }
    }
    let tolerance = model_tolerance(model.diameter());
    let policy = meshing_policy::MeshingPolicy::DEFAULT;

    // Keyed by source shell id: the flatten below looks each definition's
    // shells up directly, instead of rescanning every mesh once per definition.
    let meshed = converted_ok
        .into_par_iter()
        .map(|(shell_id, declared, shell)| {
            (
                shell_id,
                mesh_shell(declared, shell, tolerance, &closure_map, policy),
            )
        })
        .collect::<HashMap<_, _>>();
    timings.record("step_tessellate", mesh_started.elapsed());

    // Flatten per definition, attaching face colours exactly as the flat path
    // does, and accumulate one loss report across the whole scene. A
    // definition that tessellates to nothing contributes no geometry and no
    // geometry slot, so occurrences map to definitions only after this.
    let node_indices = mapped
        .all_nodes()
        .map(|node| node.index())
        .collect::<Vec<_>>();
    let mut definitions = Vec::new();
    let mut definition_of_node = HashMap::new();
    let mut failures = Vec::new();
    let mut faces = FaceTally::default();
    let mut lost_faces: Vec<FaceProvenance> = Vec::new();
    let mut reason_counts: BTreeMap<TessellationFailureReason, usize> = BTreeMap::new();
    let mut torus_census_total: BTreeMap<&'static str, usize> = BTreeMap::new();
    for (definition_index, ids) in definition_shells.iter().enumerate() {
        if ids.is_empty() {
            continue;
        }
        let mut positions = Vec::new();
        let mut indices = Vec::new();
        let mut colors = Vec::new();
        for mesh in ids.iter().filter_map(|shell_id| meshed.get(shell_id)) {
            match mesh {
                Ok(mesh) => {
                    for (provenance, polygon) in &mesh.faces {
                        let color = provenance
                            .definition_id
                            .map(SourceEntityId::get)
                            .and_then(|id| face_appearances.get(&id))
                            .map(|appearance| appearance.color)
                            .unwrap_or([1.0; 4]);
                        append_polygon(polygon, color, &mut positions, &mut indices, &mut colors);
                    }
                    faces.declared += mesh.tally.declared;
                    faces.total += mesh.tally.total;
                    faces.unsurfaced += mesh.tally.unsurfaced;
                    faces.empty += mesh.tally.empty;
                    for reason in &mesh.reasons {
                        *reason_counts.entry(*reason).or_insert(0) += 1;
                    }
                    for (tag, count) in &mesh.torus_census {
                        *torus_census_total.entry(tag).or_insert(0) += count;
                    }
                    if lost_faces.len() < LOST_FACE_IDS_REPORTED {
                        lost_faces.extend(mesh.lost_ids.iter().copied());
                    }
                }
                Err(error) => failures.push(error.clone()),
            }
        }
        if positions.is_empty() {
            eprintln!(
                "warning: STEP assembly definition '{}' (node #{definition_index}) tessellated to nothing",
                node_names[definition_index]
            );
            continue;
        }
        definition_of_node.insert(node_indices[definition_index], definitions.len());
        definitions.push(AssemblyDefinition {
            node_name: node_names[definition_index].clone(),
            soup: (positions, indices, colors),
        });
    }
    report_step_losses(
        &failures,
        definition_shells.iter().map(Vec::len).sum(),
        &faces,
        &lost_faces,
        &reason_counts,
        &torus_census_total,
        &unresolved_styles,
    );
    if definitions.is_empty() {
        anyhow::bail!("STEP assembly tessellation produced no triangles");
    }

    // One Look Instance per source occurrence, geometry = its definition slot.
    let mut occurrences = Vec::new();
    for top in mapped.top_nodes() {
        for path in mapped.paths_iter(top.index()) {
            if path.edges().is_empty() {
                continue;
            }
            let Some(definition) = definition_of_node.get(&path.terminal_node().index()) else {
                // An occurrence of a subassembly carries no definition
                // geometry of its own; its children are separate occurrences.
                continue;
            };
            occurrences.push(AssemblyOccurrence {
                definition: *definition,
                world: cgmath_to_glam(&path.matrix()),
                node_name: path.terminal_node().entity().attrs.name.clone(),
            });
        }
    }
    if occurrences.is_empty() {
        anyhow::bail!("STEP assembly produced no renderable occurrences");
    }

    Ok(StepAssemblyScene {
        definitions,
        occurrences,
        nodes: node_names.len(),
    })
}

/// Convert one definition source shell with the lossy path the flat render
/// uses, so the DIAG-002 conversion-loss records are identical.
fn convert_definition_shell(
    table: &Table,
    shell_id: u64,
) -> Result<(usize, CompressedShell<Point3, Curve3D, Surface>), String> {
    if let Some(shell) = table.shell.get(&shell_id) {
        let declared = shell.cfs_faces.len();
        let (compressed, losses) = table
            .to_compressed_shell_with_losses(shell_id, shell)
            .map_err(|error| format!("definition shell #{shell_id}: {error}"))?;
        emit_definition_losses(shell_id, declared, &compressed, &losses);
        Ok((declared, compressed))
    } else if let Some(oriented) = table.oriented_shell.get(&shell_id) {
        let declared = oriented_shell_declared(table, oriented);
        let (compressed, losses) = table
            .to_compressed_shell_with_losses(shell_id, oriented)
            .map_err(|error| format!("definition shell #{shell_id}: {error}"))?;
        emit_definition_losses(shell_id, declared, &compressed, &losses);
        Ok((declared, compressed))
    } else {
        Err(format!(
            "definition shell #{shell_id} not found in the STEP table"
        ))
    }
}

fn emit_definition_losses(
    shell_id: u64,
    declared: usize,
    compressed: &CompressedShell<Point3, Curve3D, Surface>,
    losses: &[truck_stepio::r#in::convert::FaceLoss],
) {
    let surviving = compressed.faces.len();
    for loss in losses {
        emit_import_diagnostic(shell_id, declared, surviving, loss);
    }
}

fn oriented_shell_declared(
    table: &Table,
    oriented: &truck_stepio::r#in::OrientedShellHolder,
) -> usize {
    let PlaceHolder::Ref(Name::Entity(element)) = &oriented.shell_element else {
        return 0;
    };
    table
        .shell
        .get(element)
        .map(|shell| shell.cfs_faces.len())
        .unwrap_or(0)
}

/// Convert a cgmath `Matrix4<f64>` (Truck's transform type) to a `glam::Mat4`.
///
/// cgmath stores columns in `.x/.y/.z/.w`; glam reads columns the same way, so
/// the conversion is a per-column widening. This is the single place the two
/// matrix conventions meet on the STEP assembly path.
fn cgmath_to_glam(m: &Matrix4) -> glam::Mat4 {
    glam::Mat4::from_cols(
        glam::Vec4::new(m.x.x as f32, m.x.y as f32, m.x.z as f32, m.x.w as f32),
        glam::Vec4::new(m.y.x as f32, m.y.y as f32, m.y.z as f32, m.y.w as f32),
        glam::Vec4::new(m.z.x as f32, m.z.y as f32, m.z.z as f32, m.z.w as f32),
        glam::Vec4::new(m.w.x as f32, m.w.y as f32, m.w.z as f32, m.w.w as f32),
    )
}

/// One shell's meshed faces, still keyed by source provenance so the flatten
/// stage can attach appearance without disturbing the geometry.
#[derive(Debug)]
struct StepShellMesh {
    /// `(provenance, polygon)` for every face that produced geometry, in the
    /// same order [`MeshedShape`] merges them.
    faces: Vec<(FaceProvenance, PolygonMesh)>,
    tally: FaceTally,
    lost_ids: Vec<FaceProvenance>,
    reasons: Vec<TessellationFailureReason>,
    torus_census: BTreeMap<&'static str, usize>,
}

/// How many faces a shell held, and how many of them yielded no mesh.
///
/// The three loss causes are kept apart because they are different bugs, and a
/// single "dropped" number hid that. See the counting site for which is which.
#[derive(Debug, Default, Clone, Copy)]
struct FaceTally {
    /// Faces the STEP file names on this shell, before any conversion.
    ///
    /// This is the only denominator that cannot move. `total` counts what
    /// survived the entity graph, so measuring loss against it lets a
    /// conversion that drops faces outright *improve* the reported ratio while
    /// deleting geometry — which is exactly what happened when bound conversion
    /// was made all-or-nothing: 274 faces left the model and the warning got
    /// quieter.
    declared: usize,
    total: usize,
    /// No surface could be produced for the face at all.
    unsurfaced: usize,
    /// A surface was produced, and it meshed to no triangles.
    empty: usize,
}

impl FaceTally {
    /// Faces the file declared that never reached tessellation at all.
    fn unconverted(&self) -> usize {
        self.declared.saturating_sub(self.total)
    }

    /// Faces that reach the render carrying no geometry, however they got there.
    fn lost(&self) -> usize {
        self.unconverted() + self.unsurfaced + self.empty
    }
}

/// Read the exchange structure.
///
/// [`part21`] reads it far faster than ruststep's nom grammar, but covers a
/// little less of the standard. Whatever it turns down is handed to ruststep,
/// so the fast reader can only add speed and never narrows what look accepts.
fn read_exchange(text: &str) -> anyhow::Result<ruststep::ast::Exchange> {
    match part21::parse(text) {
        Ok(exchange) => Ok(exchange),
        Err(_) => ruststep::parser::parse(text)
            .map_err(|error| anyhow::anyhow!("failed to parse STEP: {error}")),
    }
}

/// Scale the chord tolerance to the model so that a millimetre part and a
/// metre part mesh to comparable density.
///
/// This deliberately has no absolute floor. It used to be held at 1e-3 to stay
/// clear of truck's own 1e-6 minimum, since curves are tessellated in their own
/// parameter space and a placement scales the tolerance down by its radius
/// before that check happens. But an absolute floor silently assumes the file
/// is measured in millimetres. The NIST corpus is, uniformly, so the assumption
/// held there and nowhere else: an Onshape export in metres puts a half
/// millimetre fillet at a radius of 0.0005, well under a 1e-3 floor, which both
/// meshed the part absurdly coarsely and asked truck to approximate a feature
/// to worse than its own size.
///
/// The headroom the floor provided is already in the relative term. Against a
/// radius bounded by the model, `diameter * 1e-3` sits three orders of
/// magnitude above `diameter * 1e-6`, which is the same margin the floor was
/// chosen to give — only expressed in the units the file actually uses.
fn model_tolerance(diameter: f64) -> f64 {
    // The linear deflection comes from the meshing policy so a policy edit is
    // the single source of truth for the geometric-error bound (and so the
    // cache identity, which keys on the policy, actually tracks it).
    let scaled = diameter * meshing_policy::MeshingPolicy::DEFAULT.relative_linear_deflection;
    if scaled.is_finite() && scaled > 0.0 {
        scaled.max(MINIMUM_TOLERANCE)
    } else {
        DEGENERATE_TOLERANCE
    }
}

/// Flatten a meshed shell, dropping the per-face indexing in favour of one
/// flat vertex buffer. Triangles are emitted unwelded so that creased edges
/// keep their own normals when the caller generates them, and every vertex of
/// one source face carries that face's effective colour — adjacent faces keep
/// separate vertex records, so colour discontinuities survive naturally.
fn append_polygon(
    polygon: &PolygonMesh,
    color: [f32; 4],
    positions: &mut Vec<[f32; 3]>,
    indices: &mut Vec<u32>,
    colors: &mut Vec<[f32; 4]>,
) {
    let vertices = polygon.positions();
    // The triangle count is known, and these buffers reach tens of megabytes
    // on a large assembly. Growing them by doubling holds the old and the new
    // allocation at once at every step, so the high-water mark ends up well
    // above what the mesh actually needs.
    let incoming = polygon.tri_faces().len() * 3;
    positions.reserve(incoming);
    indices.reserve(incoming);
    colors.reserve(incoming);
    for face in polygon.tri_faces() {
        for vertex in face {
            let point = vertices[vertex.pos];
            indices.push(positions.len() as u32);
            positions.push([point.x as f32, point.y as f32, point.z as f32]);
            colors.push(color);
        }
    }
}

fn describe(failures: &[String]) -> String {
    match failures.first() {
        Some(first) if failures.len() == 1 => format!(": {first}"),
        Some(first) => format!(": {first} (and {} more)", failures.len() - 1),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A metre-scale export is the case an absolute floor got wrong. Onshape
    /// writes `SI_UNIT($, .METRE.)`, so a part 100 mm across has a diameter of
    /// 0.1 and its half-millimetre fillets have a radius of 0.0005. The old
    /// 1e-3 floor exceeded both the model's own relative tolerance and those
    /// radii, which meshed the part far too coarsely and asked truck to
    /// approximate a sphere to worse than its own size.
    #[test]
    fn a_metre_scale_model_is_not_held_to_a_millimetre_floor() {
        let tolerance = model_tolerance(0.1);
        assert!(
            tolerance < 0.0005,
            "tolerance {tolerance} must stay under a half-millimetre feature"
        );
        assert_eq!(tolerance, 0.1 * RELATIVE_TOLERANCE);
    }

    /// The millimetre-scale corpus this was originally tuned against must mesh
    /// exactly as it did before, since its tolerance was always the scaled term
    /// rather than the floor.
    #[test]
    fn a_millimetre_scale_model_is_unchanged() {
        assert_eq!(model_tolerance(200.0), 0.2);
        assert_eq!(model_tolerance(6.0), 0.006);
    }

    /// Tolerance has to keep tracking the model across every unit a CAD system
    /// might write, rather than flattening onto a constant at small scales.
    #[test]
    fn tolerance_tracks_the_model_across_unit_scales() {
        for diameter in [1.0e-3, 1.0, 1.0e3, 1.0e6] {
            assert_eq!(model_tolerance(diameter), diameter * RELATIVE_TOLERANCE);
        }
    }

    /// A shell with no extent has nothing to scale against, and must not yield
    /// a zero or non-finite tolerance for truck to divide by.
    #[test]
    fn a_degenerate_model_falls_back_to_a_usable_tolerance() {
        for diameter in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            let tolerance = model_tolerance(diameter);
            assert!(
                tolerance.is_finite() && tolerance > 0.0,
                "diameter {diameter} produced tolerance {tolerance}"
            );
        }
    }

    /// D9: a source face lost during conversion emits exactly one
    /// `ImportDiagnosticRecord` in the shared diagnostic sink.
    ///
    /// The hard invariant of the conversion-loss contract: a source face that
    /// existed and produced no output face/provenance must leave an
    /// explanation. This drives the emission path directly with a synthetic
    /// converter loss, exactly as `parse_step` does per real loss.
    #[test]
    fn d9_conversion_loss_emits_one_import_record() {
        let path =
            std::env::temp_dir().join(format!("look_diag002_{}_d9.jsonl", std::process::id()));
        let _ = std::fs::remove_file(&path);
        // Rust 2024 makes env mutation unsafe; the test is single-threaded
        // within this binary's scope.
        unsafe {
            std::env::set_var("TRUCK_FACE_DIAG_JSONL", &path);
            std::env::remove_var("TRUCK_FACE_DIAG");
        }
        truck_diag::set_document_context(Some("d9.step".into()));

        // A synthetic loss: face use #12 of face #7 lost when its edge curve
        // failed to convert.
        let loss = truck_stepio::r#in::convert::FaceLoss {
            provenance: FaceProvenance {
                use_id: Some(SourceEntityId::new(12)),
                definition_id: Some(SourceEntityId::new(7)),
                surface_id: None,
                outer_bound: truck_topology::compress::OuterBoundStanding::NotRetained,
            },
            reason: FaceLossReason::EdgeCurveConversionFailed,
        };
        emit_import_diagnostic(100, 3, 2, &loss);

        let text = std::fs::read_to_string(&path).unwrap_or_default();
        unsafe { std::env::remove_var("TRUCK_FACE_DIAG_JSONL") };
        truck_diag::set_document_context(None);
        let _ = std::fs::remove_file(&path);

        let rows: Vec<serde_json::Value> = text
            .lines()
            .filter(|line| !line.is_empty())
            .filter_map(|line| serde_json::from_str(line).ok())
            .collect();
        assert_eq!(rows.len(), 1, "exactly one conversion record");
        let row = &rows[0];
        assert_eq!(row["schema_version"], 1);
        assert_eq!(row["document_id"], "d9.step");
        assert_eq!(row["source_face_id"], 7, "definition id survives");
        assert_eq!(row["source_use_id"], 12, "use id survives");
        assert_eq!(row["conversion_stage"], "edge_conversion");
        assert_eq!(row["conversion_failure_kind"], "conversion_refusal");
        assert_eq!(row["refusal_tag"], "EdgeCurveConversionFailed");
        assert_eq!(row["representation_shell_context"], 100);
        assert_eq!(row["provenance_established"], true);
        assert_eq!(row["declared_shell_faces"], 3);
        assert_eq!(row["surviving_shell_faces"], 2);
    }
}
