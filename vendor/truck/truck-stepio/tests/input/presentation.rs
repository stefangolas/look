//! Parser-level structural tests for the typed STEP presentation entities.
//!
//! These prove the corpus's presentation records survive as typed data with
//! the source entity ids and references intact, rather than falling through to
//! `dummy`. Resolution into an effective appearance is Look's job and is not
//! tested here.

use ruststep::ast::DataSection;
use std::str::FromStr;
use truck_stepio::r#in::presentation::SurfaceSide;
use truck_stepio::r#in::Table;

fn table_of(data: &str) -> Table {
    let data_section = DataSection::from_str(data).expect("fixture should parse");
    Table::from_data_section(&data_section)
}

/// P1 — a direct `COLOUR_RGB` parses exactly.
#[test]
fn colour_rgb_parses_exactly() {
    let table = table_of(
        "DATA;
        #1 = COLOUR_RGB( '', 0.25, 0.5, 0.75 );
        ENDSEC;",
    );
    let colour = table.colour_rgb.get(&1).expect("colour_rgb #1");
    assert_eq!(colour.red, 0.25);
    assert_eq!(colour.green, 0.5);
    assert_eq!(colour.blue, 0.75);
    assert!(table.dummy.is_empty());
}

/// P2 — the full face-fill chain survives with every reference intact.
#[test]
fn face_fill_chain_parses() {
    let table = table_of(
        "DATA;
        #1 = STYLED_ITEM( '', ( #2 ), #10 );
        #2 = PRESENTATION_STYLE_ASSIGNMENT( ( #3 ) );
        #3 = SURFACE_STYLE_USAGE( .BOTH., #4 );
        #4 = SURFACE_SIDE_STYLE( '', ( #5 ) );
        #5 = SURFACE_STYLE_FILL_AREA( #6 );
        #6 = FILL_AREA_STYLE( '', ( #7 ) );
        #7 = FILL_AREA_STYLE_COLOUR( '', #8 );
        #8 = COLOUR_RGB( '', 0.25, 0.5, 0.75 );
        ENDSEC;",
    );
    let styled = table.styled_item.get(&1).expect("styled_item #1");
    assert_eq!(styled.styles, vec![2]);
    assert_eq!(styled.item, Some(10));

    let psa = table
        .presentation_style_assignment
        .get(&2)
        .expect("presentation_style_assignment #2");
    assert_eq!(psa.styles, vec![3]);
    assert_eq!(psa.assigned_item, None);

    let usage = table
        .surface_style_usage
        .get(&3)
        .expect("surface_style_usage #3");
    assert_eq!(usage.side, Some(SurfaceSide::Both));
    assert_eq!(usage.style, Some(4));

    let side = table
        .surface_side_style
        .get(&4)
        .expect("surface_side_style #4");
    assert_eq!(side.styles, vec![5]);

    let fill = table
        .surface_style_fill_area
        .get(&5)
        .expect("surface_style_fill_area #5");
    assert_eq!(fill.fill_area, Some(6));

    let style = table.fill_area_style.get(&6).expect("fill_area_style #6");
    assert_eq!(style.styles, vec![7]);

    let colour = table
        .fill_area_style_colour
        .get(&7)
        .expect("fill_area_style_colour #7");
    assert_eq!(colour.fill_colour, Some(8));

    let rgb = table.colour_rgb.get(&8).expect("colour_rgb #8");
    assert_eq!((rgb.red, rgb.green, rgb.blue), (0.25, 0.5, 0.75));
    assert!(table.dummy.is_empty());
}

/// P3 — a predefined colour keeps its source name.
#[test]
fn draughting_pre_defined_colour_parses() {
    let table = table_of(
        "DATA;
        #1 = DRAUGHTING_PRE_DEFINED_COLOUR( '', 'black' );
        ENDSEC;",
    );
    let colour = table
        .draughting_pre_defined_colour
        .get(&1)
        .expect("draughting_pre_defined_colour #1");
    assert_eq!(colour.predefined_colour_name, "black");
    assert!(table.dummy.is_empty());
}

/// P4 — an overriding styled item retains both the styled target and the
/// overridden style.
#[test]
fn over_riding_styled_item_parses() {
    let table = table_of(
        "DATA;
        #20 = OVER_RIDING_STYLED_ITEM( '', ( #21 ), #22, #23 );
        ENDSEC;",
    );
    let over = table
        .over_riding_styled_item
        .get(&20)
        .expect("over_riding_styled_item #20");
    assert_eq!(over.styles, vec![21]);
    assert_eq!(over.item, Some(22));
    assert_eq!(over.over_ridden_style, Some(23));
    assert!(table.dummy.is_empty());
}

/// ABC 00000414 witness chain — every entity survives with the exact source
/// ids, and the styled target resolves to a face id the shell also references.
#[test]
fn abc_00000414_chain_survives_typed() {
    let table = table_of(
        "DATA;
        #23896 = STYLED_ITEM( '', ( #78925 ), #78926 );
        #78925 = PRESENTATION_STYLE_ASSIGNMENT( ( #165088 ) );
        #78926 = FACE_SURFACE( 'trim region', ( #165089, #165090 ), #165091, .T. );
        #165088 = SURFACE_STYLE_USAGE( .BOTH., #284223 );
        #284223 = SURFACE_SIDE_STYLE( '', ( #1012602 ) );
        #1012602 = SURFACE_STYLE_FILL_AREA( #1125523 );
        #1125523 = FILL_AREA_STYLE( '', ( #1220402 ) );
        #1220402 = FILL_AREA_STYLE_COLOUR( '', #1384323 );
        #1384323 = COLOUR_RGB( '', 0.498039215800000, 1.00000000000000, 1.00000000000000 );
        ENDSEC;",
    );

    let styled = table.styled_item.get(&23896).expect("styled_item #23896");
    assert_eq!(styled.styles, vec![78925]);
    assert_eq!(styled.item, Some(78926));
    assert!(
        table.face_surface.contains_key(&78926),
        "styled target #78926 must be a parsed face"
    );

    let psa = table
        .presentation_style_assignment
        .get(&78925)
        .expect("presentation_style_assignment #78925");
    assert_eq!(psa.styles, vec![165088]);
    assert_eq!(psa.assigned_item, None);

    let usage = table
        .surface_style_usage
        .get(&165088)
        .expect("surface_style_usage #165088");
    assert_eq!(usage.side, Some(SurfaceSide::Both));
    assert_eq!(usage.style, Some(284223));

    let side = table
        .surface_side_style
        .get(&284223)
        .expect("surface_side_style #284223");
    assert_eq!(side.styles, vec![1012602]);

    let fill = table
        .surface_style_fill_area
        .get(&1012602)
        .expect("surface_style_fill_area #1012602");
    assert_eq!(fill.fill_area, Some(1125523));

    let style = table
        .fill_area_style
        .get(&1125523)
        .expect("fill_area_style #1125523");
    assert_eq!(style.styles, vec![1220402]);

    let colour = table
        .fill_area_style_colour
        .get(&1220402)
        .expect("fill_area_style_colour #1220402");
    assert_eq!(colour.fill_colour, Some(1384323));

    let rgb = table.colour_rgb.get(&1384323).expect("colour_rgb #1384323");
    assert!((rgb.red - 0.4980392158).abs() < 1.0e-9);
    assert_eq!(rgb.green, 1.0);
    assert_eq!(rgb.blue, 1.0);
    assert!(table.dummy.is_empty());
}
