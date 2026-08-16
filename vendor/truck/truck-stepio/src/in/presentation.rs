//! Typed STEP presentation entities (ISO 10303-46 subset the corpora use).
//!
//! These holders represent *source facts* only: entity references, enum values
//! such as `.BOTH.`, RGB values, and predefined colour names. Resolving a chain
//! into an effective appearance is the importer's job, not the parser's, so
//! nothing here interprets what the graph means.
//!
//! The records are parsed by hand rather than through the ruststep `Holder`
//! derive because the derive's generated sequence visitor requires every
//! declared field to be present, and the corpus omits optional fields: ABC
//! writes `PRESENTATION_STYLE_ASSIGNMENT( ( #5 ) )` and
//! `SURFACE_STYLE_FILL_AREA( #7 )` without a name, and the standard
//! `SURFACE_STYLE_USAGE` name is absent there too. A tolerant positional reader
//! keeps such records typed instead of dropping them to `dummy`.

use ruststep::ast::{Name, Parameter};

/// `COLOUR_RGB` — a direct RGB colour leaf.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColourRgbHolder {
    pub red: f64,
    pub green: f64,
    pub blue: f64,
}

/// `DRAUGHTING_PRE_DEFINED_COLOUR` — a named colour leaf.
///
/// The name is the source's own word; translating it to RGB is a resolver
/// concern so that an unknown name stays visibly unknown rather than being
/// guessed here.
#[derive(Clone, Debug, PartialEq)]
pub struct DraughtingPreDefinedColourHolder {
    pub predefined_colour_name: String,
}

/// `STYLED_ITEM` — a presentation of a representation item.
#[derive(Clone, Debug, PartialEq)]
pub struct StyledItemHolder {
    /// Referenced `PRESENTATION_STYLE_ASSIGNMENT` / `OVER_RIDING_STYLED_ITEM`
    /// entity ids, in file order.
    pub styles: Vec<u64>,
    /// The styled representation item — a face entity for the corpus's face
    /// chains, a solid or shell for body-level presentation, or any other
    /// geometric item a resolver must decline to treat as a face.
    pub item: Option<u64>,
}

/// `OVER_RIDING_STYLED_ITEM` — a `STYLED_ITEM` that overrides another style.
///
/// A subtype of `STYLED_ITEM`, so under STEP's internal mapping its fields are
/// the supertype's followed by its own: `(name, styles, item, over_ridden_style)`.
#[derive(Clone, Debug, PartialEq)]
pub struct OverRidingStyledItemHolder {
    pub styles: Vec<u64>,
    pub item: Option<u64>,
    pub over_ridden_style: Option<u64>,
}

/// `PRESENTATION_STYLE_ASSIGNMENT` — styles assigned to a representation item.
#[derive(Clone, Debug, PartialEq)]
pub struct PresentationStyleAssignmentHolder {
    pub styles: Vec<u64>,
    /// The styled item, when the record carries one. The corpus's ABC and NIST
    /// witnesses write only the styles list and leave the association to the
    /// `STYLED_ITEM.item` reference.
    pub assigned_item: Option<u64>,
}

/// Which side of a surface a `SURFACE_STYLE_USAGE` applies to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SurfaceSide {
    Positive,
    Negative,
    Both,
}

/// `SURFACE_STYLE_USAGE` — a surface style applied to one or both sides.
#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceStyleUsageHolder {
    pub side: Option<SurfaceSide>,
    pub style: Option<u64>,
}

/// `SURFACE_SIDE_STYLE` — the styles on one side of a surface.
///
/// The list also carries curve and annotation styles (`SURFACE_STYLE_BOUNDARY`,
/// `SURFACE_STYLE_PARAMETER_LINE`, ...). Those ids are preserved — they are
/// source facts — but a resolver only follows ids that name a
/// `SURFACE_STYLE_FILL_AREA`, so they cannot leak into face appearance.
#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceSideStyleHolder {
    pub styles: Vec<u64>,
}

/// `SURFACE_STYLE_FILL_AREA` — a fill-area style on a surface.
#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceStyleFillAreaHolder {
    pub fill_area: Option<u64>,
}

/// `FILL_AREA_STYLE` — a fill style composed of fill-colour styles.
#[derive(Clone, Debug, PartialEq)]
pub struct FillAreaStyleHolder {
    pub styles: Vec<u64>,
}

/// `FILL_AREA_STYLE_COLOUR` — a fill style naming a colour leaf.
#[derive(Clone, Debug, PartialEq)]
pub struct FillAreaStyleColourHolder {
    pub fill_colour: Option<u64>,
}

// --- tolerant positional readers -----------------------------------------

/// The parameters of a simple record.
///
/// A simple record's parameter is a list; a lone parameter (which the
/// presentation entities do not use, but which costs nothing to cover) is
/// treated as a one-element list.
fn fields(parameter: &Parameter) -> &[Parameter] {
    match parameter {
        Parameter::List(params) => params,
        other => std::slice::from_ref(other),
    }
}

/// The entity id a parameter references, if it is a plain entity reference.
fn entity_ref(param: &Parameter) -> Option<u64> {
    match param {
        Parameter::Ref(Name::Entity(id)) => Some(*id),
        _ => None,
    }
}

/// The entity ids a parameter references, in order. A list contributes every
/// reference it holds; a lone reference contributes itself; anything else
/// (an inlined value, a `$`, an `*`) contributes nothing, which is the honest
/// answer for a value that carries no entity id.
fn entity_refs(param: &Parameter) -> Vec<u64> {
    match param {
        Parameter::List(items) => items.iter().filter_map(entity_ref).collect(),
        other => entity_ref(other).into_iter().collect(),
    }
}

/// The first list-valued field, read as entity references.
///
/// The styles field of every style-carrying presentation entity is the
/// record's list, whether the record is the corpus's abbreviated shape or the
/// standard one with a leading name.
fn first_styles_list(fields: &[Parameter]) -> Vec<u64> {
    fields
        .iter()
        .find(|param| matches!(param, Parameter::List(_)))
        .map(entity_refs)
        .unwrap_or_default()
}

/// The last entity reference among the top-level fields.
///
/// For `STYLED_ITEM`, `PRESENTATION_STYLE_ASSIGNMENT`, `SURFACE_STYLE_USAGE`,
/// `SURFACE_STYLE_FILL_AREA` and `FILL_AREA_STYLE_COLOUR` the reference of
/// interest sits at the end whether or not a leading name was written.
fn last_ref(fields: &[Parameter]) -> Option<u64> {
    fields.iter().rev().find_map(entity_ref)
}

fn enumeration(param: &Parameter) -> Option<String> {
    match param {
        Parameter::Enumeration(value) => Some(value.clone()),
        _ => None,
    }
}

fn surface_side(param: &Parameter) -> Option<SurfaceSide> {
    match enumeration(param)?.as_str() {
        "POSITIVE" => Some(SurfaceSide::Positive),
        "NEGATIVE" => Some(SurfaceSide::Negative),
        "BOTH" => Some(SurfaceSide::Both),
        _ => None,
    }
}

fn real(param: &Parameter) -> Option<f64> {
    match param {
        Parameter::Real(value) => Some(*value),
        // Exporters write whole-number colours as `1` rather than `1.0`; both
        // are the same source fact.
        Parameter::Integer(value) => Some(*value as f64),
        _ => None,
    }
}

fn string_value(param: &Parameter) -> Option<String> {
    match param {
        Parameter::String(value) => Some(value.clone()),
        _ => None,
    }
}

/// Parse `COLOUR_RGB(name, red, green, blue)`.
pub fn colour_rgb(parameter: &Parameter) -> Option<ColourRgbHolder> {
    let fields = fields(parameter);
    let red = fields.get(1).and_then(real)?;
    let green = fields.get(2).and_then(real)?;
    let blue = fields.get(3).and_then(real)?;
    Some(ColourRgbHolder { red, green, blue })
}

/// Parse `DRAUGHTING_PRE_DEFINED_COLOUR(name, predefined_colour_name)`.
pub fn draughting_pre_defined_colour(
    parameter: &Parameter,
) -> Option<DraughtingPreDefinedColourHolder> {
    let fields = fields(parameter);
    let predefined_colour_name = fields.get(1).and_then(string_value)?;
    Some(DraughtingPreDefinedColourHolder {
        predefined_colour_name,
    })
}

/// Parse `STYLED_ITEM(name, styles, item)`.
pub fn styled_item(parameter: &Parameter) -> Option<StyledItemHolder> {
    let fields = fields(parameter);
    Some(StyledItemHolder {
        styles: first_styles_list(fields),
        item: last_ref(fields),
    })
}

/// Parse `OVER_RIDING_STYLED_ITEM(name, styles, item, over_ridden_style)`.
pub fn over_riding_styled_item(parameter: &Parameter) -> Option<OverRidingStyledItemHolder> {
    let fields = fields(parameter);
    let refs = fields.iter().filter_map(entity_ref).collect::<Vec<_>>();
    // Under the subtype's internal mapping the item is the reference before the
    // overridden style, so the last reference is the override and the one
    // before it is the item.
    let item = refs.iter().rev().nth(1).copied();
    let over_ridden_style = refs.last().copied();
    Some(OverRidingStyledItemHolder {
        styles: first_styles_list(fields),
        item,
        over_ridden_style,
    })
}

/// Parse `PRESENTATION_STYLE_ASSIGNMENT(name?, styles, assigned_item?)`.
pub fn presentation_style_assignment(
    parameter: &Parameter,
) -> Option<PresentationStyleAssignmentHolder> {
    let fields = fields(parameter);
    Some(PresentationStyleAssignmentHolder {
        styles: first_styles_list(fields),
        assigned_item: last_ref(fields),
    })
}

/// Parse `SURFACE_STYLE_USAGE(name?, side, style)`.
pub fn surface_style_usage(parameter: &Parameter) -> Option<SurfaceStyleUsageHolder> {
    let fields = fields(parameter);
    Some(SurfaceStyleUsageHolder {
        side: fields.iter().find_map(surface_side),
        style: last_ref(fields),
    })
}

/// Parse `SURFACE_SIDE_STYLE(name, styles)`.
pub fn surface_side_style(parameter: &Parameter) -> Option<SurfaceSideStyleHolder> {
    let fields = fields(parameter);
    Some(SurfaceSideStyleHolder {
        styles: first_styles_list(fields),
    })
}

/// Parse `SURFACE_STYLE_FILL_AREA(name?, fill_area)`.
pub fn surface_style_fill_area(parameter: &Parameter) -> Option<SurfaceStyleFillAreaHolder> {
    let fields = fields(parameter);
    Some(SurfaceStyleFillAreaHolder {
        fill_area: last_ref(fields),
    })
}

/// Parse `FILL_AREA_STYLE(name, styles)`.
pub fn fill_area_style(parameter: &Parameter) -> Option<FillAreaStyleHolder> {
    let fields = fields(parameter);
    Some(FillAreaStyleHolder {
        styles: first_styles_list(fields),
    })
}

/// Parse `FILL_AREA_STYLE_COLOUR(name, fill_colour)`.
pub fn fill_area_style_colour(parameter: &Parameter) -> Option<FillAreaStyleColourHolder> {
    let fields = fields(parameter);
    Some(FillAreaStyleColourHolder {
        fill_colour: last_ref(fields),
    })
}
