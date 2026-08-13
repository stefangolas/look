//! Resolve effective per-face appearance from STEP presentation entities.
//!
//! `truck_stepio` keeps the presentation graph as raw source facts (see
//! `truck_stepio::in::presentation`); this module walks that graph into an
//! effective per-face colour. It deliberately knows nothing about tessellation:
//! the join key is the source face entity id — the same id
//! `FaceProvenance::definition_id` carries — so appearance can be attached to a
//! meshed face without touching how it was meshed.
//!
//! Scope is deliberately narrow. Only presentation semantics that can define a
//! surface/face fill are resolved, through the canonical chain
//!
//! ```text
//! PRESENTATION_STYLE_ASSIGNMENT
//!     → SURFACE_STYLE_USAGE
//!     → SURFACE_SIDE_STYLE
//!     → SURFACE_STYLE_FILL_AREA
//!     → FILL_AREA_STYLE
//!     → FILL_AREA_STYLE_COLOUR
//!     → COLOUR_RGB | DRAUGHTING_PRE_DEFINED_COLOUR
//! ```
//!
//! Curve and annotation presentation never enters the map. A styled
//! `TRIMMED_CURVE`, `CIRCLE`, or `CARTESIAN_POINT` target is not a face, and the
//! only ids that map to colours are ids the shell graph actually reaches as
//! faces, so AP203 curve/annotation styling cannot leak into face appearance.
//!
//! Precedence is structural, never file order:
//!
//! ```text
//! explicit face style > face-level overriding style
//!     > inherited shell/body/solid style > unstyled
//! ```

use std::collections::HashMap;

use truck_stepio::r#in::Table;
use truck_stepio::r#in::ruststep::ast::Name;
use truck_stepio::r#in::ruststep::tables::PlaceHolder;

/// The effective appearance of one source face.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EffectiveAppearance {
    /// Linear RGBA. Alpha is 1 for v1; STEP `COLOUR_RGB` carries no opacity.
    pub color: [f32; 4],
}

/// Source face entity id → effective appearance.
///
/// The key is `FaceProvenance.definition_id`: the `FACE_SURFACE` /
/// `ADVANCED_FACE` entity id the shell referenced. Absence means "no supported
/// source face appearance", never a guessed colour.
pub type FaceAppearanceMap = HashMap<u64, EffectiveAppearance>;

/// Why a presentation chain did not resolve to a colour.
#[derive(Clone, Debug, PartialEq)]
pub struct UnresolvedStyle {
    /// The entity that was meant to carry the colour.
    pub entity_id: u64,
    /// The STEP entity kind.
    pub kind: &'static str,
    /// A short reason.
    pub reason: &'static str,
}

/// The ISO 10303-46 `predefined_colour` enumeration, mapped explicitly.
///
/// Only these eight names exist in the standard. An unknown name is left
/// unresolved rather than guessed, so a file asserting a colour this reader
/// does not know stays visibly unresolved instead of silently re-colouring.
const PREDEFINED_COLOURS: &[(&str, [f32; 3])] = &[
    ("black", [0.0, 0.0, 0.0]),
    ("white", [1.0, 1.0, 1.0]),
    ("red", [1.0, 0.0, 0.0]),
    ("green", [0.0, 1.0, 0.0]),
    ("blue", [0.0, 0.0, 1.0]),
    ("yellow", [1.0, 1.0, 0.0]),
    ("magenta", [1.0, 0.0, 1.0]),
    ("cyan", [0.0, 1.0, 1.0]),
];

/// Resolve every face this file's presentation graph can colour.
///
/// The map is keyed by `definition_id`, ready to join against
/// `FaceProvenance.definition_id` at tessellation time.
pub fn resolve_face_appearances(table: &Table) -> (FaceAppearanceMap, Vec<UnresolvedStyle>) {
    let mut unresolved = Vec::new();

    // Colour leaves: COLOUR_RGB directly, named colours through the explicit
    // mapping above.
    let mut colours: HashMap<u64, EffectiveAppearance> = HashMap::new();
    for (&id, holder) in &table.colour_rgb {
        colours.insert(
            id,
            EffectiveAppearance {
                color: [
                    holder.red as f32,
                    holder.green as f32,
                    holder.blue as f32,
                    1.0,
                ],
            },
        );
    }
    for (&id, holder) in &table.draughting_pre_defined_colour {
        match PREDEFINED_COLOURS
            .iter()
            .find(|(name, _)| *name == holder.predefined_colour_name)
        {
            Some((_, [r, g, b])) => {
                colours.insert(
                    id,
                    EffectiveAppearance {
                        color: [*r, *g, *b, 1.0],
                    },
                );
            }
            None => unresolved.push(UnresolvedStyle {
                entity_id: id,
                kind: "DRAUGHTING_PRE_DEFINED_COLOUR",
                reason: "unknown predefined colour name",
            }),
        }
    }

    // The chain, built id-by-id. Each step keeps an `Option`: a broken link
    // stays broken and is reported through the styled item that asked for it.
    let fill_colours: HashMap<u64, Option<EffectiveAppearance>> = table
        .fill_area_style_colour
        .iter()
        .map(|(&id, holder)| {
            (
                id,
                holder
                    .fill_colour
                    .and_then(|colour_id| colours.get(&colour_id))
                    .copied(),
            )
        })
        .collect();
    let fill_styles: HashMap<u64, Option<EffectiveAppearance>> = table
        .fill_area_style
        .iter()
        .map(|(&id, holder)| {
            (
                id,
                holder
                    .styles
                    .iter()
                    .find_map(|style_id| fill_colours.get(style_id).copied().flatten()),
            )
        })
        .collect();
    let surface_fills: HashMap<u64, Option<EffectiveAppearance>> = table
        .surface_style_fill_area
        .iter()
        .map(|(&id, holder)| {
            (
                id,
                holder
                    .fill_area
                    .and_then(|style_id| fill_styles.get(&style_id))
                    .copied()
                    .flatten(),
            )
        })
        .collect();
    let side_styles: HashMap<u64, Option<EffectiveAppearance>> = table
        .surface_side_style
        .iter()
        .map(|(&id, holder)| {
            (
                id,
                holder
                    .styles
                    .iter()
                    .find_map(|fill_id| surface_fills.get(fill_id).copied().flatten()),
            )
        })
        .collect();
    let usages: HashMap<u64, Option<EffectiveAppearance>> = table
        .surface_style_usage
        .iter()
        .map(|(&id, holder)| {
            (
                id,
                holder
                    .style
                    .and_then(|side_id| side_styles.get(&side_id))
                    .copied()
                    .flatten(),
            )
        })
        .collect();
    // A presentation style assignment's styles name SURFACE_STYLE_USAGE
    // entities directly.
    let assignments: HashMap<u64, Option<EffectiveAppearance>> = table
        .presentation_style_assignment
        .iter()
        .map(|(&id, holder)| {
            (
                id,
                holder
                    .styles
                    .iter()
                    .find_map(|usage_id| usages.get(usage_id).copied().flatten()),
            )
        })
        .collect();

    let mut direct: HashMap<u64, EffectiveAppearance> = HashMap::new();
    let mut overrides: HashMap<u64, EffectiveAppearance> = HashMap::new();
    let mut parents: Vec<(u64, EffectiveAppearance)> = Vec::new();

    // STYLED_ITEM: the item it names is what it styles.
    for (&id, styled) in &table.styled_item {
        let Some(target) = styled.item else { continue };
        let Some(resolved) = resolve_target_style(table, target, &styled.styles, &assignments)
        else {
            continue;
        };
        let Some(color) = resolved.color else {
            unresolved.push(UnresolvedStyle {
                entity_id: id,
                kind: "STYLED_ITEM",
                reason: "no style in the chain resolved to a colour",
            });
            continue;
        };
        match resolved.target {
            Target::Face(face_id) => {
                direct.insert(face_id, color);
            }
            Target::Shape(shape_id) => parents.push((shape_id, color)),
            Target::Other => unreachable!(),
        }
    }

    // OVER_RIDING_STYLED_ITEM: the item is the styled target and the override
    // colour comes from its own styles.
    for (&id, over) in &table.over_riding_styled_item {
        let Some(target) = over.item else { continue };
        let Some(resolved) = resolve_target_style(table, target, &over.styles, &assignments) else {
            continue;
        };
        let Some(color) = resolved.color else {
            unresolved.push(UnresolvedStyle {
                entity_id: id,
                kind: "OVER_RIDING_STYLED_ITEM",
                reason: "no style in the chain resolved to a colour",
            });
            continue;
        };
        match resolved.target {
            Target::Face(face_id) => {
                overrides.insert(face_id, color);
            }
            Target::Shape(shape_id) => parents.push((shape_id, color)),
            Target::Other => unreachable!(),
        }
    }

    // PRESENTATION_STYLE_ASSIGNMENT with an assigned_item: the standard form,
    // where the assignment itself names what it styles. The corpus writes only
    // the styles list and leaves the association to the STYLED_ITEM, so this
    // is a support path rather than the demonstrated one.
    for (&id, psa) in &table.presentation_style_assignment {
        let Some(target) = psa.assigned_item else {
            continue;
        };
        let Some(resolved) = resolve_target_style(table, target, &psa.styles, &assignments) else {
            continue;
        };
        let Some(color) = resolved.color else {
            unresolved.push(UnresolvedStyle {
                entity_id: id,
                kind: "PRESENTATION_STYLE_ASSIGNMENT",
                reason: "no style in the chain resolved to a colour",
            });
            continue;
        };
        match resolved.target {
            Target::Face(face_id) => {
                direct.insert(face_id, color);
            }
            Target::Shape(shape_id) => parents.push((shape_id, color)),
            Target::Other => unreachable!(),
        }
    }

    // Effective precedence: explicit face style, then face-level override,
    // then inherited shell/body/solid style.
    let mut map = FaceAppearanceMap::new();
    for (face_id, color) in direct {
        map.insert(face_id, color);
    }
    for (face_id, color) in overrides {
        map.entry(face_id).or_insert(color);
    }
    // Inheritance is applied in sorted shape-id order so a face reachable from
    // more than one styled shape gets the same colour on every run.
    parents.sort_by_key(|(shape_id, _)| *shape_id);
    for (shape_id, color) in parents {
        for face_id in shape_face_ids(table, shape_id) {
            map.entry(face_id).or_insert(color);
        }
    }

    (map, unresolved)
}

/// Resolve a styled item's style list to the first colour the chain produces.
fn resolve_styled_styles(
    styles: &[u64],
    assignments: &HashMap<u64, Option<EffectiveAppearance>>,
) -> Option<EffectiveAppearance> {
    styles
        .iter()
        .find_map(|assignment_id| assignments.get(assignment_id).copied().flatten())
}

/// What an item's style list resolved to for its target.
///
/// `None` for a target that is neither a face nor a shape: styling a curve,
/// point, or annotation item is not a face colour and is silently ignored.
/// `Some` carries the target and the colour the chain produced, which may
/// itself be `None` when the chain broke — the caller decides whether to
/// report that.
struct ResolvedStyle {
    target: Target,
    color: Option<EffectiveAppearance>,
}

fn resolve_target_style(
    table: &Table,
    target_id: u64,
    styles: &[u64],
    assignments: &HashMap<u64, Option<EffectiveAppearance>>,
) -> Option<ResolvedStyle> {
    let target = target_kind(table, target_id);
    if matches!(target, Target::Other) {
        return None;
    }
    Some(ResolvedStyle {
        target,
        color: resolve_styled_styles(styles, assignments),
    })
}

/// What kind of thing a styled item names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Target {
    /// A `FACE_SURFACE` / `ADVANCED_FACE` entity — the join key.
    Face(u64),
    /// A solid or shell whose faces inherit the style.
    Shape(u64),
    /// Anything else — a curve, point, surface, or annotation item. Styling it
    /// is not a face colour, and is deliberately ignored.
    Other,
}

fn target_kind(table: &Table, id: u64) -> Target {
    if table.face_surface.contains_key(&id) {
        return Target::Face(id);
    }
    if table.manifold_solid_brep.contains_key(&id)
        || table.shell_based_surface_model.contains_key(&id)
        || table.shell.contains_key(&id)
        || table.oriented_shell.contains_key(&id)
    {
        return Target::Shape(id);
    }
    Target::Other
}

/// The entity id a holder reference names, when it is a plain entity reference.
fn resolved_ref_id<T>(holder: &PlaceHolder<T>) -> Option<u64> {
    if let PlaceHolder::Ref(Name::Entity(id)) = holder {
        Some(*id)
    } else {
        None
    }
}

/// The face *definition* ids of a shape — a `MANIFOLD_SOLID_BREP`, a
/// `SHELL_BASED_SURFACE_MODEL`, or a shell — using the same rules
/// `FaceProvenance.definition_id` does: a shell reference to an
/// `ORIENTED_FACE` contributes that use's `FACE_SURFACE` element id, and a
/// direct reference contributes the referenced face id.
fn shape_face_ids(table: &Table, shape_id: u64) -> Vec<u64> {
    let mut ids = Vec::new();
    if let Some(solid) = table.manifold_solid_brep.get(&shape_id) {
        if let Some(outer) = resolved_ref_id(&solid.outer) {
            collect_boundary_shell_face_ids(table, outer, &mut ids);
        }
        for void in &solid.voids {
            if let Some(oriented_id) = resolved_ref_id(void)
                && let Some(shell_id) = table
                    .oriented_shell
                    .get(&oriented_id)
                    .and_then(|oriented| resolved_ref_id(&oriented.shell_element))
            {
                collect_shell_face_ids(table, shell_id, &mut ids);
            }
        }
    }
    if let Some(model) = table.shell_based_surface_model.get(&shape_id) {
        for boundary in &model.sbsm_boundary {
            if let Some(boundary_id) = resolved_ref_id(boundary) {
                collect_boundary_shell_face_ids(table, boundary_id, &mut ids);
            }
        }
    }
    if table.shell.contains_key(&shape_id) {
        collect_shell_face_ids(table, shape_id, &mut ids);
    }
    ids
}

/// A shape boundary that names either a shell directly or an oriented shell
/// wrapping one.
fn collect_boundary_shell_face_ids(table: &Table, id: u64, ids: &mut Vec<u64>) {
    if table.shell.contains_key(&id) {
        collect_shell_face_ids(table, id, ids);
    } else if let Some(shell_id) = table
        .oriented_shell
        .get(&id)
        .and_then(|oriented| resolved_ref_id(&oriented.shell_element))
    {
        collect_shell_face_ids(table, shell_id, ids);
    }
}

/// The face definition ids of one shell's `cfs_faces`.
fn collect_shell_face_ids(table: &Table, shell_id: u64, ids: &mut Vec<u64>) {
    let Some(shell) = table.shell.get(&shell_id) else {
        return;
    };
    for face in &shell.cfs_faces {
        let Some(face_ref) = resolved_ref_id(face) else {
            continue;
        };
        if let Some(oriented) = table.oriented_face.get(&face_ref) {
            if let Some(def_id) = resolved_ref_id(&oriented.face_element) {
                ids.push(def_id);
            }
        } else if table.face_surface.contains_key(&face_ref) {
            ids.push(face_ref);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;
    use truck_stepio::r#in::ruststep::ast::DataSection;

    fn table_of(data: &str) -> Table {
        let data_section = DataSection::from_str(data).expect("fixture should parse");
        Table::from_data_section(&data_section)
    }

    const RED: [f32; 4] = [1.0, 0.0, 0.0, 1.0];
    const BLUE: [f32; 4] = [0.0, 0.0, 1.0, 1.0];
    const LAVENDER: [f32; 4] = [0.8235294, 0.8235294, 1.0, 1.0];
    const BLACK: [f32; 4] = [0.0, 0.0, 0.0, 1.0];

    /// A face stub: referenced entities need not exist for parsing, only the
    /// face record itself, because the parser keeps references unresolved.
    fn face(id: u64, label: &str) -> String {
        format!("#{id} = FACE_SURFACE( '{label}', ( #90001 ), #90002, .T. );")
    }

    /// A full STYLED_ITEM → FILL_AREA_STYLE_COLOUR chain targeting `target`.
    /// The colour leaf itself is written by the caller, so each test can name
    /// its own `COLOUR_RGB` or `DRAUGHTING_PRE_DEFINED_COLOUR` at `colour_id`.
    fn styled_chain(item_id: u64, colour_id: u64, target: u64) -> String {
        let psa = item_id + 1;
        let usage = item_id + 2;
        let side = item_id + 3;
        let fill = item_id + 4;
        let style = item_id + 5;
        let colour = item_id + 6;
        format!(
            "#{item_id} = STYLED_ITEM( '', ( #{psa} ), #{target} );\n\
             #{psa} = PRESENTATION_STYLE_ASSIGNMENT( ( #{usage} ) );\n\
             #{usage} = SURFACE_STYLE_USAGE( .BOTH., #{side} );\n\
             #{side} = SURFACE_SIDE_STYLE( '', ( #{fill} ) );\n\
             #{fill} = SURFACE_STYLE_FILL_AREA( #{style} );\n\
             #{style} = FILL_AREA_STYLE( '', ( #{colour} ) );\n\
             #{colour} = FILL_AREA_STYLE_COLOUR( '', #{colour_id} );"
        )
    }

    fn rgb(id: u64, r: f64, g: f64, b: f64) -> String {
        format!("#{id} = COLOUR_RGB( '', {r}, {g}, {b} );")
    }

    /// T1 — one styled face resolves to its exact RGB.
    #[test]
    fn a_direct_rgb_face_resolves_exactly() {
        let table = table_of(&format!(
            "DATA;\n{}\n{}\n{}\nENDSEC;",
            styled_chain(1, 8, 100),
            rgb(8, 1.0, 0.0, 0.0),
            face(100, "faceA")
        ));
        let (map, unresolved) = resolve_face_appearances(&table);
        assert!(unresolved.is_empty(), "unexpected: {unresolved:?}");
        assert_eq!(
            map.get(&100).expect("face #100"),
            &EffectiveAppearance { color: RED }
        );
    }

    /// T2 — two neighbouring faces keep distinct colours.
    #[test]
    fn adjacent_faces_keep_distinct_colours() {
        let table = table_of(&format!(
            "DATA;\n{}\n{}\n{}\n{}\n{}\n{}\nENDSEC;",
            styled_chain(1, 8, 100),
            rgb(8, 1.0, 0.0, 0.0),
            styled_chain(11, 18, 110),
            rgb(18, 0.0, 0.0, 1.0),
            face(100, "faceA"),
            face(110, "faceB")
        ));
        let (map, _) = resolve_face_appearances(&table);
        assert_eq!(map.get(&100).expect("face #100").color, RED);
        assert_eq!(map.get(&110).expect("face #110").color, BLUE);
        assert_ne!(
            map.get(&100).expect("face #100").color,
            map.get(&110).expect("face #110").color
        );
    }

    /// T3 — a face without its own style inherits the parent solid's colour.
    #[test]
    fn an_unstyled_face_inherits_the_parent_solid_colour() {
        let table = table_of(&format!(
            "DATA;\n{}\n{}\n{}\n{}\n#200 = MANIFOLD_SOLID_BREP( '', #201 );\n\
             #201 = CLOSED_SHELL( '', ( #100, #110 ) );\nENDSEC;",
            styled_chain(1, 8, 200),
            rgb(8, 0.823529411764706, 0.823529411764706, 1.0),
            face(100, "faceA"),
            face(110, "faceB")
        ));
        let (map, unresolved) = resolve_face_appearances(&table);
        assert!(unresolved.is_empty(), "unexpected: {unresolved:?}");
        assert_eq!(map.get(&100).expect("face #100").color, LAVENDER);
        assert_eq!(map.get(&110).expect("face #110").color, LAVENDER);
    }

    /// T4 — an explicit face style beats the parent solid's colour.
    #[test]
    fn an_explicit_face_style_overrides_the_parent() {
        let table = table_of(&format!(
            "DATA;\n{}\n{}\n{}\n{}\n{}\n{}\n#200 = MANIFOLD_SOLID_BREP( '', #201 );\n\
             #201 = CLOSED_SHELL( '', ( #100, #110 ) );\nENDSEC;",
            styled_chain(1, 8, 200),
            rgb(8, 0.823529411764706, 0.823529411764706, 1.0),
            styled_chain(11, 18, 100),
            rgb(18, 0.0, 0.0, 0.0),
            face(100, "faceA"),
            face(110, "faceB")
        ));
        let (map, _) = resolve_face_appearances(&table);
        assert_eq!(map.get(&100).expect("face #100").color, BLACK);
        assert_eq!(map.get(&110).expect("face #110").color, LAVENDER);
    }

    /// T5 — an OVER_RIDING_STYLED_ITEM gives the face its own colour, beating
    /// the parent style it overrides.
    #[test]
    fn an_over_riding_styled_item_colours_the_face() {
        let table = table_of(&format!(
            "DATA;\n{}\n{}\n{}\n{}\n{}\n{}\n#200 = MANIFOLD_SOLID_BREP( '', #201 );\n\
             #201 = CLOSED_SHELL( '', ( #100 ) );\nENDSEC;",
            styled_chain(1, 8, 200),
            rgb(8, 0.823529411764706, 0.823529411764706, 1.0),
            face(100, "faceA"),
            // OVER_RIDING_STYLED_ITEM( name, styles, item, over_ridden_style )
            "#10 = OVER_RIDING_STYLED_ITEM( '', ( #11 ), #100, #1 );",
            styled_chain(11, 18, 100),
            rgb(18, 0.0, 0.0, 0.0)
        ));
        let (map, _) = resolve_face_appearances(&table);
        assert_eq!(map.get(&100).expect("face #100").color, BLACK);
    }

    /// T6 — a known DRAUGHTING_PRE_DEFINED_COLOUR resolves through the
    /// explicit name map.
    #[test]
    fn a_predefined_colour_resolves() {
        let table = table_of(&format!(
            "DATA;\n{}\n#8 = DRAUGHTING_PRE_DEFINED_COLOUR( '', 'black' );\n{}\nENDSEC;",
            styled_chain(1, 8, 100),
            face(100, "faceA")
        ));
        let (map, unresolved) = resolve_face_appearances(&table);
        assert!(unresolved.is_empty(), "unexpected: {unresolved:?}");
        assert_eq!(map.get(&100).expect("face #100").color, BLACK);
    }

    /// An unknown predefined name stays unresolved and is diagnosed, not
    /// guessed.
    #[test]
    fn an_unknown_predefined_colour_is_unresolved() {
        let table = table_of(&format!(
            "DATA;\n{}\n#8 = DRAUGHTING_PRE_DEFINED_COLOUR( '', 'lavender' );\n{}\nENDSEC;",
            styled_chain(1, 8, 100),
            face(100, "faceA")
        ));
        let (map, unresolved) = resolve_face_appearances(&table);
        assert!(
            !map.contains_key(&100),
            "unknown colour must not be guessed"
        );
        assert!(
            unresolved
                .iter()
                .any(|u| u.kind == "DRAUGHTING_PRE_DEFINED_COLOUR"),
            "expected a diagnostic, got: {unresolved:?}"
        );
    }

    /// T7 — styling a curve does not colour faces.
    #[test]
    fn a_styled_curve_does_not_colour_faces() {
        let table = table_of(&format!(
            "DATA;\n{}\n{}\n#9 = TRIMMED_CURVE( '', #300, ( #301, #302 ), .T., .PARAMETER. );\n\
             #301 = CARTESIAN_POINT( '', ( 0., 0., 0. ) );\n\
             #302 = CARTESIAN_POINT( '', ( 1., 1., 1. ) );\n\
             #300 = LINE( '', #303, #304 );\n\
             #303 = CARTESIAN_POINT( '', ( 0., 0., 0. ) );\n\
             #304 = VECTOR( '', #305, 1. );\n\
             #305 = DIRECTION( '', ( 1., 0., 0. ) );\n\
             {}\nENDSEC;",
            styled_chain(1, 8, 9),
            rgb(8, 0.0, 1.0, 0.0),
            face(100, "faceA")
        ));
        let (map, _) = resolve_face_appearances(&table);
        assert!(
            map.is_empty(),
            "styling a curve must not populate the face map: {map:?}"
        );
    }

    /// T8 — a face with no style is absent from the map.
    #[test]
    fn an_unstyled_face_is_absent() {
        let table = table_of(&format!("DATA;\n{}\nENDSEC;", face(900, "bare")));
        let (map, _) = resolve_face_appearances(&table);
        assert_eq!(map.get(&900), None);
    }
}
