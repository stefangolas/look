use std::{
    fs,
    io::Cursor,
    path::{Path, PathBuf},
};

use anyhow::Context;
use image::{DynamicImage, ImageFormat, Rgba, RgbaImage};
use serde_json::json;

const LONGITUDE_SEGMENTS: u32 = 128;
const LATITUDE_SEGMENTS: u32 = 64;
const TEXTURE_WIDTH: u32 = 1024;
const TEXTURE_HEIGHT: u32 = 512;

#[derive(Clone, Copy)]
struct Scratch {
    y: f32,
    slope: f32,
    width: f32,
    depth: f32,
}

fn main() -> anyhow::Result<()> {
    let output = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("tests/fixtures/aluminum_ball_bearing.glb"));
    let bytes = generate_glb()?;
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&output, &bytes)
        .with_context(|| format!("failed to write '{}'", output.display()))?;
    println!("wrote {} bytes to {}", bytes.len(), output.display());
    Ok(())
}

fn generate_glb() -> anyhow::Result<Vec<u8>> {
    let (vertices, indices) = sphere_geometry();
    let (base_color, metallic_roughness, normal) = texture_maps()?;
    let mut binary = Vec::new();

    let vertex_offset = append_aligned(&mut binary, &vertices);
    let index_offset = append_aligned(&mut binary, &indices);
    let base_color_offset = append_aligned(&mut binary, &base_color);
    let metallic_roughness_offset = append_aligned(&mut binary, &metallic_roughness);
    let normal_offset = append_aligned(&mut binary, &normal);
    pad_to_four(&mut binary, 0);

    let vertex_count = (LONGITUDE_SEGMENTS + 1) * (LATITUDE_SEGMENTS + 1);
    let index_count = indices.len() / 4;
    let document = json!({
        "asset": {
            "version": "2.0",
            "generator": "look aluminum ball-bearing fixture generator",
            "copyright": "2026 look contributors; MIT OR Apache-2.0"
        },
        "scene": 0,
        "scenes": [{ "name": "Ball Bearing Test Scene", "nodes": [0] }],
        "nodes": [{ "name": "Textured Aluminum Ball Bearing", "mesh": 0 }],
        "meshes": [{
            "name": "Precision Sphere",
            "primitives": [{
                "attributes": { "POSITION": 0, "NORMAL": 1, "TEXCOORD_0": 2 },
                "indices": 3,
                "material": 0,
                "mode": 4
            }]
        }],
        "materials": [{
            "name": "Machined Aluminum",
            "pbrMetallicRoughness": {
                "baseColorFactor": [1.0, 1.0, 1.0, 1.0],
                "baseColorTexture": { "index": 0, "texCoord": 0 },
                "metallicFactor": 1.0,
                "roughnessFactor": 1.0,
                "metallicRoughnessTexture": { "index": 1, "texCoord": 0 }
            },
            "normalTexture": { "index": 2, "texCoord": 0, "scale": 0.72 },
            "occlusionTexture": { "index": 1, "texCoord": 0, "strength": 0.22 },
            "alphaMode": "OPAQUE",
            "doubleSided": false
        }],
        "samplers": [{
            "name": "Repeat Linear Mipmap",
            "magFilter": 9729,
            "minFilter": 9987,
            "wrapS": 10497,
            "wrapT": 10497
        }],
        "textures": [
            { "name": "Aluminum Color Grain", "sampler": 0, "source": 0 },
            { "name": "Aluminum ORM", "sampler": 0, "source": 1 },
            { "name": "Machining Scratch Normals", "sampler": 0, "source": 2 }
        ],
        "images": [
            { "name": "Aluminum Color Grain", "bufferView": 2, "mimeType": "image/png" },
            { "name": "Aluminum Occlusion Roughness Metallic", "bufferView": 3, "mimeType": "image/png" },
            { "name": "Machining Scratch Normals", "bufferView": 4, "mimeType": "image/png" }
        ],
        "accessors": [
            {
                "name": "Sphere Positions",
                "bufferView": 0,
                "byteOffset": 0,
                "componentType": 5126,
                "count": vertex_count,
                "type": "VEC3",
                "min": [-1.0, -1.0, -1.0],
                "max": [1.0, 1.0, 1.0]
            },
            {
                "name": "Sphere Normals",
                "bufferView": 0,
                "byteOffset": 12,
                "componentType": 5126,
                "count": vertex_count,
                "type": "VEC3"
            },
            {
                "name": "Sphere UVs",
                "bufferView": 0,
                "byteOffset": 24,
                "componentType": 5126,
                "count": vertex_count,
                "type": "VEC2"
            },
            {
                "name": "Sphere Indices",
                "bufferView": 1,
                "byteOffset": 0,
                "componentType": 5125,
                "count": index_count,
                "type": "SCALAR"
            }
        ],
        "bufferViews": [
            {
                "name": "Interleaved Sphere Vertices",
                "buffer": 0,
                "byteOffset": vertex_offset,
                "byteLength": vertices.len(),
                "byteStride": 32,
                "target": 34962
            },
            {
                "name": "Sphere Triangle Indices",
                "buffer": 0,
                "byteOffset": index_offset,
                "byteLength": indices.len(),
                "target": 34963
            },
            {
                "name": "Aluminum Color PNG",
                "buffer": 0,
                "byteOffset": base_color_offset,
                "byteLength": base_color.len()
            },
            {
                "name": "Aluminum ORM PNG",
                "buffer": 0,
                "byteOffset": metallic_roughness_offset,
                "byteLength": metallic_roughness.len()
            },
            {
                "name": "Aluminum Normal PNG",
                "buffer": 0,
                "byteOffset": normal_offset,
                "byteLength": normal.len()
            }
        ],
        "buffers": [{ "byteLength": binary.len() }]
    });

    let mut json_bytes = serde_json::to_vec(&document)?;
    pad_to_four(&mut json_bytes, b' ');
    let total_length = 12 + 8 + json_bytes.len() + 8 + binary.len();
    let mut glb = Vec::with_capacity(total_length);
    glb.extend_from_slice(&0x4654_6c67_u32.to_le_bytes());
    glb.extend_from_slice(&2_u32.to_le_bytes());
    glb.extend_from_slice(&(total_length as u32).to_le_bytes());
    glb.extend_from_slice(&(json_bytes.len() as u32).to_le_bytes());
    glb.extend_from_slice(&0x4e4f_534a_u32.to_le_bytes());
    glb.extend_from_slice(&json_bytes);
    glb.extend_from_slice(&(binary.len() as u32).to_le_bytes());
    glb.extend_from_slice(&0x004e_4942_u32.to_le_bytes());
    glb.extend_from_slice(&binary);
    Ok(glb)
}

fn sphere_geometry() -> (Vec<u8>, Vec<u8>) {
    let vertex_count = ((LONGITUDE_SEGMENTS + 1) * (LATITUDE_SEGMENTS + 1)) as usize;
    let mut vertices = Vec::with_capacity(vertex_count * 32);
    for latitude in 0..=LATITUDE_SEGMENTS {
        let v = latitude as f32 / LATITUDE_SEGMENTS as f32;
        let theta = std::f32::consts::PI * v;
        let sin_theta = theta.sin();
        let cos_theta = theta.cos();
        for longitude in 0..=LONGITUDE_SEGMENTS {
            let u = longitude as f32 / LONGITUDE_SEGMENTS as f32;
            let phi = std::f32::consts::TAU * u;
            let position = [sin_theta * phi.cos(), cos_theta, sin_theta * phi.sin()];
            for value in position.into_iter().chain(position).chain([u, v]) {
                vertices.extend_from_slice(&value.to_le_bytes());
            }
        }
    }

    let row = LONGITUDE_SEGMENTS + 1;
    let mut indices =
        Vec::<u32>::with_capacity((LONGITUDE_SEGMENTS * (LATITUDE_SEGMENTS - 1) * 6) as usize);
    for latitude in 0..LATITUDE_SEGMENTS {
        for longitude in 0..LONGITUDE_SEGMENTS {
            let a = latitude * row + longitude;
            let b = (latitude + 1) * row + longitude;
            let c = b + 1;
            let d = a + 1;
            if latitude != 0 {
                indices.extend_from_slice(&[a, d, b]);
            }
            if latitude + 1 != LATITUDE_SEGMENTS {
                indices.extend_from_slice(&[d, c, b]);
            }
        }
    }
    let mut index_bytes = Vec::with_capacity(indices.len() * 4);
    for index in indices {
        index_bytes.extend_from_slice(&index.to_le_bytes());
    }
    (vertices, index_bytes)
}

fn texture_maps() -> anyhow::Result<(Vec<u8>, Vec<u8>, Vec<u8>)> {
    let scratches = scratches();
    let mut heights = vec![0.0_f32; (TEXTURE_WIDTH * TEXTURE_HEIGHT) as usize];
    let mut scratch_masks = vec![0.0_f32; heights.len()];
    for y in 0..TEXTURE_HEIGHT {
        for x in 0..TEXTURE_WIDTH {
            let index = (y * TEXTURE_WIDTH + x) as usize;
            let scratch = scratch_mask(x as f32, y as f32, &scratches);
            let fine = signed_noise(x, y);
            let brush = brushed_pattern(x, y);
            heights[index] = brush * 0.16 + fine * 0.055 - scratch * 0.7;
            scratch_masks[index] = scratch;
        }
    }

    let mut base = RgbaImage::new(TEXTURE_WIDTH, TEXTURE_HEIGHT);
    let mut orm = RgbaImage::new(TEXTURE_WIDTH, TEXTURE_HEIGHT);
    let mut normals = RgbaImage::new(TEXTURE_WIDTH, TEXTURE_HEIGHT);
    for y in 0..TEXTURE_HEIGHT {
        for x in 0..TEXTURE_WIDTH {
            let index = (y * TEXTURE_WIDTH + x) as usize;
            let fine = signed_noise(x.wrapping_mul(7) + 17, y.wrapping_mul(11) + 3);
            let brush = brushed_pattern(x, y);
            let scratch = scratch_masks[index];
            let brightness = 218.0 + brush * 10.0 + fine * 5.0 - scratch * 30.0;
            base.put_pixel(
                x,
                y,
                Rgba([
                    channel(brightness * 0.965),
                    channel(brightness * 0.985),
                    channel(brightness * 1.015),
                    255,
                ]),
            );

            let roughness = 43.0 + brush.abs() * 22.0 + scratch * 118.0 + fine * 7.0;
            let occlusion = 250.0 - scratch * 20.0;
            let metallic = 252.0 - scratch * 5.0;
            orm.put_pixel(
                x,
                y,
                Rgba([
                    channel(occlusion),
                    channel(roughness),
                    channel(metallic),
                    255,
                ]),
            );

            let left =
                heights[(y * TEXTURE_WIDTH + (x + TEXTURE_WIDTH - 1) % TEXTURE_WIDTH) as usize];
            let right = heights[(y * TEXTURE_WIDTH + (x + 1) % TEXTURE_WIDTH) as usize];
            let up_y = y.saturating_sub(1);
            let down_y = (y + 1).min(TEXTURE_HEIGHT - 1);
            let up = heights[(up_y * TEXTURE_WIDTH + x) as usize];
            let down = heights[(down_y * TEXTURE_WIDTH + x) as usize];
            let mut normal = [-(right - left) * 2.8, -(down - up) * 2.8, 1.0];
            let length = (normal[0] * normal[0] + normal[1] * normal[1] + 1.0).sqrt();
            normal.iter_mut().for_each(|value| *value /= length);
            normals.put_pixel(
                x,
                y,
                Rgba([
                    channel((normal[0] * 0.5 + 0.5) * 255.0),
                    channel((normal[1] * 0.5 + 0.5) * 255.0),
                    channel((normal[2] * 0.5 + 0.5) * 255.0),
                    255,
                ]),
            );
        }
    }
    Ok((png_bytes(base)?, png_bytes(orm)?, png_bytes(normals)?))
}

fn scratches() -> Vec<Scratch> {
    let mut state = 0x6d2b_79f5_u32;
    (0..42)
        .map(|_| {
            let mut next = || {
                state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                state as f32 / u32::MAX as f32
            };
            Scratch {
                y: next() * TEXTURE_HEIGHT as f32,
                slope: (next() - 0.5) * 0.12,
                width: 0.35 + next() * 1.15,
                depth: 0.25 + next() * 0.75,
            }
        })
        .collect()
}

fn scratch_mask(x: f32, y: f32, scratches: &[Scratch]) -> f32 {
    scratches.iter().fold(0.0_f32, |mask, scratch| {
        let line_y = (scratch.y + scratch.slope * x).rem_euclid(TEXTURE_HEIGHT as f32);
        let direct = (y - line_y).abs();
        let distance = direct.min(TEXTURE_HEIGHT as f32 - direct);
        let profile = (-(distance / scratch.width).powi(2) * 2.4).exp() * scratch.depth;
        mask.max(profile)
    })
}

fn brushed_pattern(x: u32, y: u32) -> f32 {
    let u = x as f32 / TEXTURE_WIDTH as f32;
    let v = y as f32 / TEXTURE_HEIGHT as f32;
    (std::f32::consts::TAU * (v * 173.0 + (u * 7.0).sin() * 0.12)).sin() * 0.55
        + (std::f32::consts::TAU * v * 419.0).sin() * 0.25
        + signed_noise(x / 3, y) * 0.2
}

fn signed_noise(x: u32, y: u32) -> f32 {
    let mut value = x
        .wrapping_mul(0x85eb_ca6b)
        .wrapping_add(y.wrapping_mul(0xc2b2_ae35))
        .wrapping_add(0x27d4_eb2d);
    value ^= value >> 15;
    value = value.wrapping_mul(0x2c1b_3c6d);
    value ^= value >> 12;
    value as f32 / u32::MAX as f32 * 2.0 - 1.0
}

fn png_bytes(image: RgbaImage) -> anyhow::Result<Vec<u8>> {
    let mut cursor = Cursor::new(Vec::new());
    DynamicImage::ImageRgba8(image).write_to(&mut cursor, ImageFormat::Png)?;
    Ok(cursor.into_inner())
}

fn append_aligned(target: &mut Vec<u8>, bytes: &[u8]) -> usize {
    pad_to_four(target, 0);
    let offset = target.len();
    target.extend_from_slice(bytes);
    offset
}

fn pad_to_four(bytes: &mut Vec<u8>, value: u8) {
    while !bytes.len().is_multiple_of(4) {
        bytes.push(value);
    }
}

fn channel(value: f32) -> u8 {
    value.round().clamp(0.0, 255.0) as u8
}

#[allow(dead_code)]
fn _assert_fixture_path(path: &Path) {
    assert_eq!(
        path.extension().and_then(|value| value.to_str()),
        Some("glb")
    );
}
