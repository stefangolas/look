//! AP242 tessellated geometry.
//!
//! Most STEP files store solids as boundary representations that have to be
//! evaluated and meshed. AP242 also allows a model to ship the mesh directly,
//! as `TESSELLATED_SOLID` referencing faces whose triangles are already listed.
//! Such a file has no shells at all, so the boundary-representation path finds
//! nothing to do and the triangles have to be read straight out of the syntax
//! tree.
//!
//! Faces index into a shared `COORDINATES_LIST` through their own `pnindex`,
//! and carry triangles either explicitly or encoded as strips and fans.

use std::collections::HashMap;

use ruststep::ast::{DataSection, EntityInstance, Name, Parameter, Record};

/// Read every tessellated face in the section into one triangle soup.
///
/// Returns `None` when the file carries no tessellated geometry, so the caller
/// can report the more useful "no geometry at all" rather than an empty mesh.
pub fn read(section: &DataSection) -> Option<(Vec<[f32; 3]>, Vec<u32>)> {
    let mut records: HashMap<u64, &Record> = HashMap::new();
    for entity in &section.entities {
        if let EntityInstance::Simple { id, record } = entity {
            records.insert(*id, record);
        }
    }

    let mut positions = Vec::new();
    let mut indices = Vec::new();
    let mut coordinate_cache: HashMap<u64, Vec<[f64; 3]>> = HashMap::new();
    let mut faces_seen = 0usize;

    for record in records.values() {
        let is_face = matches!(
            record.name.as_str(),
            "TRIANGULATED_FACE" | "COMPLEX_TRIANGULATED_FACE"
        );
        if !is_face {
            continue;
        }
        faces_seen += 1;
        let fields = match &record.parameter {
            Parameter::List(fields) => fields,
            _ => continue,
        };

        // name, coordinates, pnmax, normals, geometric_link, pnindex, then
        // either triangles, or triangle strips followed by fans.
        let Some(coordinates_id) = fields.get(1).and_then(reference) else {
            continue;
        };
        let points = match coordinate_cache.get(&coordinates_id) {
            Some(points) => points,
            None => {
                let Some(points) = records.get(&coordinates_id).and_then(|r| coordinates(r)) else {
                    continue;
                };
                coordinate_cache.entry(coordinates_id).or_insert(points)
            }
        };

        let pnindex = fields.get(5).map(integers).unwrap_or_default();
        // `pnindex` is optional. When absent the face's own indices already
        // address the coordinates list directly.
        let resolve = |local: i64| -> Option<[f64; 3]> {
            let coordinate = if pnindex.is_empty() {
                local
            } else {
                *pnindex.get(usize::try_from(local).ok()?.checked_sub(1)?)?
            };
            points
                .get(usize::try_from(coordinate).ok()?.checked_sub(1)?)
                .copied()
        };

        let mut emit = |a: i64, b: i64, c: i64| {
            if let (Some(a), Some(b), Some(c)) = (resolve(a), resolve(b), resolve(c)) {
                for point in [a, b, c] {
                    indices.push(positions.len() as u32);
                    positions.push([point[0] as f32, point[1] as f32, point[2] as f32]);
                }
            }
        };

        match record.name.as_str() {
            "TRIANGULATED_FACE" => {
                for triangle in fields.get(6).map(lists).unwrap_or_default() {
                    if let [a, b, c] = triangle[..] {
                        emit(a, b, c);
                    }
                }
            }
            _ => {
                // A strip alternates winding so that every triangle faces the
                // same way; a fan pivots on its first vertex.
                for strip in fields.get(6).map(lists).unwrap_or_default() {
                    for (offset, window) in strip.windows(3).enumerate() {
                        if offset % 2 == 0 {
                            emit(window[0], window[1], window[2]);
                        } else {
                            emit(window[1], window[0], window[2]);
                        }
                    }
                }
                for fan in fields.get(7).map(lists).unwrap_or_default() {
                    for window in fan.windows(2).skip(1) {
                        emit(fan[0], window[0], window[1]);
                    }
                }
            }
        }
    }

    (faces_seen > 0 && !positions.is_empty()).then_some((positions, indices))
}

/// `COORDINATES_LIST(name, npoints, ((x, y, z), ...))`
fn coordinates(record: &Record) -> Option<Vec<[f64; 3]>> {
    if record.name != "COORDINATES_LIST" {
        return None;
    }
    let Parameter::List(fields) = &record.parameter else {
        return None;
    };
    let Parameter::List(points) = fields.get(2)? else {
        return None;
    };
    Some(
        points
            .iter()
            .filter_map(|point| {
                let Parameter::List(values) = point else {
                    return None;
                };
                let mut coordinate = [0.0; 3];
                for (slot, value) in coordinate.iter_mut().zip(values) {
                    *slot = real(value)?;
                }
                Some(coordinate)
            })
            .collect(),
    )
}

fn reference(parameter: &Parameter) -> Option<u64> {
    match parameter {
        Parameter::Ref(Name::Entity(id)) => Some(*id),
        _ => None,
    }
}

fn real(parameter: &Parameter) -> Option<f64> {
    match parameter {
        Parameter::Real(value) => Some(*value),
        Parameter::Integer(value) => Some(*value as f64),
        _ => None,
    }
}

fn integers(parameter: &Parameter) -> Vec<i64> {
    match parameter {
        Parameter::List(values) => values
            .iter()
            .filter_map(|value| match value {
                Parameter::Integer(value) => Some(*value),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

fn lists(parameter: &Parameter) -> Vec<Vec<i64>> {
    match parameter {
        Parameter::List(values) => values.iter().map(integers).collect(),
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    fn section(body: &str) -> DataSection {
        let text = format!(
            "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
             FILE_NAME('t','2024-01-01T00:00:00',(''),(''),'','','');\n\
             FILE_SCHEMA(('AP242'));\nENDSEC;\nDATA;\n{body}\nENDSEC;\nEND-ISO-10303-21;\n"
        );
        let exchange = ruststep::parser::parse(&text).expect("fixture should parse");
        exchange.data.into_iter().next().expect("one data section")
    }

    /// A square from two strip triangles, indexed through `pnindex`.
    #[test]
    fn decodes_a_triangle_strip() {
        let parsed = read(&section(
            "#1=COORDINATES_LIST('',4,((0.,0.,0.),(1.,0.,0.),(0.,1.,0.),(1.,1.,0.)));\n\
             #2=COMPLEX_TRIANGULATED_FACE('',#1,4,(),$,(1,2,3,4),((1,2,3,4)),());",
        ));
        let (positions, indices) = parsed.expect("should read tessellated geometry");
        assert_eq!(indices.len(), 6, "a four-vertex strip is two triangles");
        assert_eq!(positions.len(), 6);
        // First triangle keeps the strip order.
        assert_eq!(positions[0], [0.0, 0.0, 0.0]);
        assert_eq!(positions[1], [1.0, 0.0, 0.0]);
        assert_eq!(positions[2], [0.0, 1.0, 0.0]);
        // The second swaps its first two vertices so both faces wind the same
        // way, which is what makes a strip a strip.
        assert_eq!(positions[3], [0.0, 1.0, 0.0]);
        assert_eq!(positions[4], [1.0, 0.0, 0.0]);
        assert_eq!(positions[5], [1.0, 1.0, 0.0]);
    }

    #[test]
    fn decodes_a_triangle_fan() {
        let parsed = read(&section(
            "#1=COORDINATES_LIST('',4,((0.,0.,0.),(1.,0.,0.),(1.,1.,0.),(0.,1.,0.)));\n\
             #2=COMPLEX_TRIANGULATED_FACE('',#1,4,(),$,(1,2,3,4),(),((1,2,3,4)));",
        ));
        let (_, indices) = parsed.expect("should read tessellated geometry");
        assert_eq!(indices.len(), 6, "a four-vertex fan is two triangles");
    }

    #[test]
    fn decodes_explicit_triangles() {
        let parsed = read(&section(
            "#1=COORDINATES_LIST('',3,((0.,0.,0.),(1.,0.,0.),(0.,1.,0.)));\n\
             #2=TRIANGULATED_FACE('',#1,3,(),$,(1,2,3),((1,2,3)));",
        ));
        let (positions, indices) = parsed.expect("should read tessellated geometry");
        assert_eq!(indices.len(), 3);
        assert_eq!(positions[2], [0.0, 1.0, 0.0]);
    }

    /// `pnindex` is optional; without it the face addresses coordinates
    /// directly.
    #[test]
    fn empty_pnindex_addresses_coordinates_directly() {
        let parsed = read(&section(
            "#1=COORDINATES_LIST('',3,((5.,0.,0.),(0.,5.,0.),(0.,0.,5.)));\n\
             #2=TRIANGULATED_FACE('',#1,3,(),$,(),((1,2,3)));",
        ));
        let (positions, _) = parsed.expect("should read tessellated geometry");
        assert_eq!(positions[0], [5.0, 0.0, 0.0]);
        assert_eq!(positions[2], [0.0, 0.0, 5.0]);
    }

    /// Out-of-range indices must be skipped rather than panicking, because a
    /// malformed file should not take the renderer down.
    #[test]
    fn out_of_range_indices_are_skipped() {
        let parsed = read(&section(
            "#1=COORDINATES_LIST('',3,((0.,0.,0.),(1.,0.,0.),(0.,1.,0.)));\n\
             #2=TRIANGULATED_FACE('',#1,3,(),$,(1,2,3),((1,2,99),(1,2,3)));",
        ));
        let (_, indices) = parsed.expect("should read tessellated geometry");
        assert_eq!(indices.len(), 3, "only the valid triangle survives");
    }

    #[test]
    fn a_file_without_tessellated_faces_reads_as_none() {
        assert!(read(&section("#1=CARTESIAN_POINT('',(0.,0.,0.));")).is_none());
    }

    #[test]
    fn parameter_helpers_reject_wrong_shapes() {
        assert_eq!(integers(&Parameter::Integer(3)), Vec::<i64>::new());
        assert!(reference(&Parameter::NotProvided).is_none());
        assert!(real(&Parameter::from_str("'text'").unwrap()).is_none());
    }
}
