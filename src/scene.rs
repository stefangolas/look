use std::{collections::HashMap, fs, path::Path, thread, time::Instant};

use anyhow::{Context, bail};
use bytemuck::{Pod, Zeroable};
use glam::{Mat3, Mat4, Vec3};
use serde::{Deserialize, Serialize};

use crate::{
    config::{MaterialMode, UpAxis},
    timing::Timings,
};

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct SourceVertexAttributes {
    pub tex_coord_0: [f32; 2],
    pub tex_coord_1: [f32; 2],
    pub color: [f32; 4],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureWrap {
    ClampToEdge,
    MirroredRepeat,
    Repeat,
}

#[derive(Debug, Clone, Copy)]
pub struct TextureSampler {
    pub mag_linear: bool,
    pub min_linear: bool,
    pub mipmap_linear: bool,
    pub wrap_u: TextureWrap,
    pub wrap_v: TextureWrap,
}

#[derive(Debug, Clone)]
pub struct SourceTexture {
    pub encoded: Vec<u8>,
    pub sampler: TextureSampler,
    pub decoded: Option<DecodedTexture>,
}

#[derive(Debug, Clone)]
pub struct DecodedTexture {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

#[derive(Debug, Clone, Copy)]
pub struct TextureReference {
    pub texture: usize,
    pub tex_coord: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlphaMode {
    Opaque,
    Mask,
    Blend,
}

#[derive(Debug, Clone)]
pub struct SourceMaterial {
    pub name: Option<String>,
    pub base_color_factor: [f32; 4],
    pub base_color_texture: Option<TextureReference>,
    pub metallic_factor: f32,
    pub roughness_factor: f32,
    pub metallic_roughness_texture: Option<TextureReference>,
    pub normal_texture: Option<TextureReference>,
    pub normal_scale: f32,
    pub occlusion_texture: Option<TextureReference>,
    pub occlusion_strength: f32,
    pub emissive_factor: [f32; 3],
    pub emissive_texture: Option<TextureReference>,
    pub alpha_mode: AlphaMode,
    pub alpha_cutoff: f32,
    pub double_sided: bool,
}

#[derive(Debug, Clone)]
pub struct Geometry {
    pub vertices: Vec<Vertex>,
    pub source_attributes: Option<Vec<SourceVertexAttributes>>,
    pub indices: Vec<u32>,
    pub hash: String,
    pub bounds: Bounds,
}

#[derive(Debug, Clone)]
pub struct Instance {
    pub geometry: usize,
    pub material: usize,
    pub transform: Mat4,
    pub normal_transform: Mat3,
    pub node_name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CompiledScene {
    pub source_hash: String,
    pub geometries: Vec<Geometry>,
    pub instances: Vec<Instance>,
    pub materials: Vec<SourceMaterial>,
    pub textures: Vec<SourceTexture>,
    pub bounds: Bounds,
    pub statistics: SceneStatistics,
}

pub fn prepare_source_textures(
    scene: &mut CompiledScene,
    timings: &mut Timings,
) -> anyhow::Result<()> {
    let pending = scene
        .textures
        .iter()
        .enumerate()
        .filter_map(|(index, texture)| texture.decoded.is_none().then_some(index))
        .collect::<Vec<_>>();
    if pending.is_empty() {
        return Ok(());
    }

    let started = Instant::now();
    let worker_count = pending.len().min(
        thread::available_parallelism()
            .map(usize::from)
            .unwrap_or(1),
    );
    let batches = thread::scope(|scope| {
        let mut handles = Vec::with_capacity(worker_count);
        for worker in 0..worker_count {
            let pending = &pending;
            let textures = &scene.textures;
            handles.push(scope.spawn(move || -> anyhow::Result<Vec<_>> {
                let mut batch = Vec::new();
                for pending_index in (worker..pending.len()).step_by(worker_count) {
                    let index = pending[pending_index];
                    let decoded = image::load_from_memory(&textures[index].encoded)
                        .with_context(|| format!("failed to decode look texture {index}"))?
                        .to_rgba8();
                    batch.push((
                        index,
                        DecodedTexture {
                            width: decoded.width(),
                            height: decoded.height(),
                            rgba: decoded.into_raw(),
                        },
                    ));
                }
                Ok(batch)
            }));
        }
        handles
            .into_iter()
            .map(|handle| handle.join().expect("texture decoder thread panicked"))
            .collect::<anyhow::Result<Vec<_>>>()
    })?;
    for batch in batches {
        for (index, decoded) in batch {
            scene.textures[index].decoded = Some(decoded);
        }
    }
    timings.record("texture_decode", started.elapsed());
    Ok(())
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Bounds {
    pub min: [f32; 3],
    pub max: [f32; 3],
}

impl Bounds {
    pub fn empty() -> Self {
        Self {
            min: [f32::INFINITY; 3],
            max: [f32::NEG_INFINITY; 3],
        }
    }

    pub fn from_positions(positions: &[[f32; 3]]) -> Self {
        let mut bounds = Self::empty();
        for position in positions {
            bounds.include(Vec3::from_array(*position));
        }
        bounds
    }

    pub fn include(&mut self, point: Vec3) {
        let min = Vec3::from_array(self.min).min(point);
        let max = Vec3::from_array(self.max).max(point);
        self.min = min.to_array();
        self.max = max.to_array();
    }

    pub fn include_transformed(&mut self, other: &Bounds, transform: Mat4) {
        let min = Vec3::from_array(other.min);
        let max = Vec3::from_array(other.max);
        for x in [min.x, max.x] {
            for y in [min.y, max.y] {
                for z in [min.z, max.z] {
                    self.include(transform.transform_point3(Vec3::new(x, y, z)));
                }
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        !self.min[0].is_finite() || !self.max[0].is_finite()
    }

    pub fn center(&self) -> Vec3 {
        (Vec3::from_array(self.min) + Vec3::from_array(self.max)) * 0.5
    }

    pub fn size(&self) -> Vec3 {
        Vec3::from_array(self.max) - Vec3::from_array(self.min)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneStatistics {
    pub nodes: u64,
    pub mesh_primitives: u64,
    pub unique_geometries: u64,
    pub instances: u64,
    pub vertices: u64,
    pub triangles: u64,
    pub duplicate_geometries_removed: u64,
    pub materials: u64,
    pub textures: u64,
    pub bounds: Bounds,
}

pub fn compile_glb(
    path: &Path,
    up_axis: UpAxis,
    timings: &mut Timings,
) -> anyhow::Result<CompiledScene> {
    compile_glb_internal(path, up_axis, true, timings)
}

pub fn compile_glb_for_render(
    path: &Path,
    up_axis: UpAxis,
    material_mode: MaterialMode,
    timings: &mut Timings,
) -> anyhow::Result<CompiledScene> {
    compile_glb_internal(
        path,
        up_axis,
        material_mode == MaterialMode::Source,
        timings,
    )
}

pub fn compile_scene(
    path: &Path,
    up_axis: UpAxis,
    timings: &mut Timings,
) -> anyhow::Result<CompiledScene> {
    match extension(path).as_str() {
        "glb" => compile_glb(path, up_axis, timings),
        "stl" => compile_stl(path, up_axis, true, timings),
        other => bail!("unsupported scene extension '{other}'; expected GLB or STL"),
    }
}

pub fn compile_scene_for_render(
    path: &Path,
    up_axis: UpAxis,
    material_mode: MaterialMode,
    timings: &mut Timings,
) -> anyhow::Result<CompiledScene> {
    match extension(path).as_str() {
        "glb" => compile_glb_for_render(path, up_axis, material_mode, timings),
        "stl" => compile_stl(
            path,
            up_axis,
            material_mode == MaterialMode::Source,
            timings,
        ),
        other => bail!("unsupported scene extension '{other}'; expected GLB or STL"),
    }
}

fn extension(path: &Path) -> String {
    path.extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
}

fn compile_stl(
    path: &Path,
    up_axis: UpAxis,
    include_source_materials: bool,
    timings: &mut Timings,
) -> anyhow::Result<CompiledScene> {
    let bytes = timings.measure("read", || {
        fs::read(path).with_context(|| format!("failed to read scene '{}'", path.display()))
    })?;
    let source_hash = timings.measure("hash", || blake3::hash(&bytes).to_hex().to_string());
    let parse_started = Instant::now();
    let (vertices, indices) = parse_stl(&bytes)?;
    timings.record("parse", parse_started.elapsed());

    let compile_started = Instant::now();
    let local_bounds = Bounds::from_positions(
        &vertices
            .iter()
            .map(|vertex| vertex.position)
            .collect::<Vec<_>>(),
    );
    let source_attributes = include_source_materials.then(|| {
        vec![
            SourceVertexAttributes {
                tex_coord_0: [0.0; 2],
                tex_coord_1: [0.0; 2],
                color: [1.0; 4],
            };
            vertices.len()
        ]
    });
    let raw_hash = hash_geometry(&vertices, source_attributes.as_deref(), &indices);
    let geometry = Geometry {
        vertices,
        source_attributes,
        indices,
        hash: blake3::Hash::from_bytes(raw_hash).to_hex().to_string(),
        bounds: local_bounds,
    };
    let transform = normalization_transform(up_axis);
    let normal_transform = Mat3::from_mat4(transform).inverse().transpose();
    let mut bounds = Bounds::empty();
    bounds.include_transformed(&geometry.bounds, transform);
    let triangles = geometry.indices.len() as u64 / 3;
    let vertex_count = geometry.vertices.len() as u64;
    let instance = Instance {
        geometry: 0,
        material: 0,
        transform,
        normal_transform,
        node_name: path
            .file_stem()
            .and_then(|value| value.to_str())
            .map(ToOwned::to_owned),
    };
    timings.record("compile", compile_started.elapsed());

    Ok(CompiledScene {
        source_hash,
        geometries: vec![geometry],
        instances: vec![instance],
        materials: vec![default_source_material()],
        textures: Vec::new(),
        bounds,
        statistics: SceneStatistics {
            nodes: 1,
            mesh_primitives: 1,
            unique_geometries: 1,
            instances: 1,
            vertices: vertex_count,
            triangles,
            duplicate_geometries_removed: 0,
            materials: 1,
            textures: 0,
            bounds,
        },
    })
}

fn parse_stl(bytes: &[u8]) -> anyhow::Result<(Vec<Vertex>, Vec<u32>)> {
    if bytes.len() >= 84 {
        let triangle_count = u32::from_le_bytes(bytes[80..84].try_into().unwrap()) as usize;
        if let Some(expected) = triangle_count
            .checked_mul(50)
            .and_then(|payload| 84_usize.checked_add(payload))
            && expected == bytes.len()
        {
            return parse_binary_stl(bytes, triangle_count);
        }
    }
    parse_ascii_stl(bytes)
}

fn parse_binary_stl(
    bytes: &[u8],
    triangle_count: usize,
) -> anyhow::Result<(Vec<Vertex>, Vec<u32>)> {
    let vertex_count = triangle_count
        .checked_mul(3)
        .context("STL vertex count overflowed")?;
    if vertex_count > u32::MAX as usize {
        bail!("STL contains too many vertices");
    }
    let mut vertices = Vec::with_capacity(vertex_count);
    for triangle in 0..triangle_count {
        let offset = 84 + triangle * 50;
        let declared_normal = read_vec3_le(bytes, offset)?;
        let positions = [
            read_vec3_le(bytes, offset + 12)?,
            read_vec3_le(bytes, offset + 24)?,
            read_vec3_le(bytes, offset + 36)?,
        ];
        let normal = Vec3::from_array(declared_normal)
            .try_normalize()
            .unwrap_or_else(|| face_normal(positions));
        vertices.extend(positions.into_iter().map(|position| Vertex {
            position,
            normal: normal.to_array(),
        }));
    }
    let indices = (0..vertex_count as u32).collect();
    Ok((vertices, indices))
}

fn parse_ascii_stl(bytes: &[u8]) -> anyhow::Result<(Vec<Vertex>, Vec<u32>)> {
    let text = std::str::from_utf8(bytes).context("STL is neither valid binary nor UTF-8 ASCII")?;
    let tokens = text.split_ascii_whitespace().collect::<Vec<_>>();
    let mut positions = Vec::<[f32; 3]>::new();
    let mut index = 0;
    while index < tokens.len() {
        if tokens[index].eq_ignore_ascii_case("vertex") {
            let values = tokens
                .get(index + 1..index + 4)
                .context("ASCII STL vertex is incomplete")?;
            positions.push([
                values[0].parse().context("invalid ASCII STL vertex X")?,
                values[1].parse().context("invalid ASCII STL vertex Y")?,
                values[2].parse().context("invalid ASCII STL vertex Z")?,
            ]);
            index += 4;
        } else {
            index += 1;
        }
    }
    if positions.is_empty() || !positions.len().is_multiple_of(3) {
        bail!("ASCII STL contains an incomplete triangle set");
    }
    if positions.len() > u32::MAX as usize {
        bail!("STL contains too many vertices");
    }
    let mut vertices = Vec::with_capacity(positions.len());
    for triangle in positions.chunks_exact(3) {
        let points = [triangle[0], triangle[1], triangle[2]];
        let normal = face_normal(points).to_array();
        vertices.extend(
            points
                .into_iter()
                .map(|position| Vertex { position, normal }),
        );
    }
    let indices = (0..vertices.len() as u32).collect();
    Ok((vertices, indices))
}

fn read_vec3_le(bytes: &[u8], offset: usize) -> anyhow::Result<[f32; 3]> {
    let values = bytes
        .get(offset..offset + 12)
        .context("binary STL ended inside a triangle")?;
    Ok([
        f32::from_le_bytes(values[0..4].try_into().unwrap()),
        f32::from_le_bytes(values[4..8].try_into().unwrap()),
        f32::from_le_bytes(values[8..12].try_into().unwrap()),
    ])
}

fn face_normal(positions: [[f32; 3]; 3]) -> Vec3 {
    let a = Vec3::from_array(positions[0]);
    let b = Vec3::from_array(positions[1]);
    let c = Vec3::from_array(positions[2]);
    (b - a).cross(c - a).try_normalize().unwrap_or(Vec3::Y)
}

fn compile_glb_internal(
    path: &Path,
    up_axis: UpAxis,
    include_source_materials: bool,
    timings: &mut Timings,
) -> anyhow::Result<CompiledScene> {
    let bytes = timings.measure("read", || {
        fs::read(path).with_context(|| format!("failed to read scene '{}'", path.display()))
    })?;
    let source_hash = timings.measure("hash", || blake3::hash(&bytes).to_hex().to_string());
    let gltf = timings
        .measure("parse", || gltf::Gltf::from_slice(&bytes))
        .context("failed to parse GLB")?;
    let blob = gltf
        .blob
        .as_deref()
        .context("GLB does not contain an embedded binary buffer")?;

    let compile_started = std::time::Instant::now();
    let textures = if include_source_materials {
        compile_textures(&gltf, blob)?
    } else {
        Vec::new()
    };
    let mut materials = if include_source_materials {
        gltf.materials()
            .map(compile_material)
            .collect::<anyhow::Result<Vec<_>>>()?
    } else {
        Vec::new()
    };
    let default_material = materials.len();
    materials.push(default_source_material());
    let mut geometries = Vec::<Geometry>::new();
    let mut geometry_candidates = HashMap::<[u8; 32], Vec<usize>>::new();
    let mut primitive_map = HashMap::<(usize, usize), (usize, usize)>::new();
    let mut source_primitives = 0_u64;
    let mut duplicate_geometries_removed = 0_u64;

    for mesh in gltf.meshes() {
        for primitive in mesh.primitives() {
            source_primitives += 1;
            if primitive.mode() != gltf::mesh::Mode::Triangles {
                bail!(
                    "mesh {} primitive {} uses unsupported topology {:?}; only triangles are supported",
                    mesh.index(),
                    primitive.index(),
                    primitive.mode()
                );
            }

            let reader = primitive.reader(|buffer| match buffer.source() {
                gltf::buffer::Source::Bin => Some(blob),
                gltf::buffer::Source::Uri(_) => None,
            });
            let positions = reader
                .read_positions()
                .context("mesh primitive is missing POSITION data")?
                .collect::<Vec<_>>();
            let indices = reader
                .read_indices()
                .map(|indices| indices.into_u32().collect::<Vec<_>>())
                .unwrap_or_else(|| (0..positions.len() as u32).collect());
            if indices.len() % 3 != 0 {
                bail!(
                    "mesh {} primitive {} has an index count that is not divisible by three",
                    mesh.index(),
                    primitive.index()
                );
            }

            let normals = reader
                .read_normals()
                .map(|normals| normals.collect::<Vec<_>>())
                .unwrap_or_else(|| generate_normals(&positions, &indices));
            if normals.len() != positions.len() {
                bail!(
                    "mesh {} primitive {} has mismatched position and normal counts",
                    mesh.index(),
                    primitive.index()
                );
            }

            let tex_coord_0 = include_source_materials.then(|| {
                reader
                    .read_tex_coords(0)
                    .map(|coordinates| coordinates.into_f32().collect::<Vec<_>>())
                    .unwrap_or_else(|| vec![[0.0, 0.0]; positions.len()])
            });
            let tex_coord_1 = include_source_materials.then(|| {
                reader
                    .read_tex_coords(1)
                    .map(|coordinates| coordinates.into_f32().collect::<Vec<_>>())
                    .unwrap_or_else(|| vec![[0.0, 0.0]; positions.len()])
            });
            let colors = include_source_materials.then(|| {
                reader
                    .read_colors(0)
                    .map(|colors| colors.into_rgba_f32().collect::<Vec<_>>())
                    .unwrap_or_else(|| vec![[1.0; 4]; positions.len()])
            });
            if tex_coord_0
                .as_ref()
                .is_some_and(|values| values.len() != positions.len())
                || tex_coord_1
                    .as_ref()
                    .is_some_and(|values| values.len() != positions.len())
                || colors
                    .as_ref()
                    .is_some_and(|values| values.len() != positions.len())
            {
                bail!(
                    "mesh {} primitive {} has mismatched vertex attribute counts",
                    mesh.index(),
                    primitive.index()
                );
            }

            let vertices = positions
                .iter()
                .enumerate()
                .map(|(index, position)| Vertex {
                    position: *position,
                    normal: normals[index],
                })
                .collect::<Vec<_>>();
            let source_attributes = include_source_materials.then(|| {
                (0..positions.len())
                    .map(|index| SourceVertexAttributes {
                        tex_coord_0: tex_coord_0
                            .as_ref()
                            .map(|values| values[index])
                            .unwrap_or([0.0, 0.0]),
                        tex_coord_1: tex_coord_1
                            .as_ref()
                            .map(|values| values[index])
                            .unwrap_or([0.0, 0.0]),
                        color: colors
                            .as_ref()
                            .map(|values| values[index])
                            .unwrap_or([1.0; 4]),
                    })
                    .collect::<Vec<_>>()
            });
            let bounds = Bounds::from_positions(&positions);
            let raw_hash = hash_geometry(&vertices, source_attributes.as_deref(), &indices);

            let existing = geometry_candidates.get(&raw_hash).and_then(|candidates| {
                candidates.iter().copied().find(|index| {
                    geometries[*index].vertices == vertices
                        && geometries[*index].source_attributes == source_attributes
                        && geometries[*index].indices == indices
                })
            });
            let geometry_index = if let Some(index) = existing {
                duplicate_geometries_removed += 1;
                index
            } else {
                let index = geometries.len();
                geometries.push(Geometry {
                    vertices,
                    source_attributes,
                    indices,
                    hash: blake3::Hash::from_bytes(raw_hash).to_hex().to_string(),
                    bounds,
                });
                geometry_candidates.entry(raw_hash).or_default().push(index);
                index
            };
            let material = if include_source_materials {
                primitive.material().index().unwrap_or(default_material)
            } else {
                default_material
            };
            primitive_map.insert(
                (mesh.index(), primitive.index()),
                (geometry_index, material),
            );
        }
    }

    let normalization = normalization_transform(up_axis);
    let mut instances = Vec::new();
    let mut scene_bounds = Bounds::empty();
    let mut node_count = 0_u64;
    let scene = gltf
        .default_scene()
        .or_else(|| gltf.scenes().next())
        .context("GLB contains no scene")?;
    for node in scene.nodes() {
        visit_node(
            node,
            normalization,
            &primitive_map,
            &geometries,
            &mut instances,
            &mut scene_bounds,
            &mut node_count,
        )?;
    }
    if instances.is_empty() || scene_bounds.is_empty() {
        bail!("GLB scene contains no renderable triangle meshes");
    }

    timings.record("compile", compile_started.elapsed());
    let triangles = instances
        .iter()
        .map(|instance| geometries[instance.geometry].indices.len() as u64 / 3)
        .sum();
    let vertices = geometries
        .iter()
        .map(|geometry| geometry.vertices.len() as u64)
        .sum();
    let statistics = SceneStatistics {
        nodes: node_count,
        mesh_primitives: source_primitives,
        unique_geometries: geometries.len() as u64,
        instances: instances.len() as u64,
        vertices,
        triangles,
        duplicate_geometries_removed,
        materials: materials.len() as u64,
        textures: textures.len() as u64,
        bounds: scene_bounds,
    };

    Ok(CompiledScene {
        source_hash,
        geometries,
        instances,
        materials,
        textures,
        bounds: scene_bounds,
        statistics,
    })
}

#[allow(clippy::too_many_arguments)]
fn visit_node(
    node: gltf::Node<'_>,
    parent_transform: Mat4,
    primitive_map: &HashMap<(usize, usize), (usize, usize)>,
    geometries: &[Geometry],
    instances: &mut Vec<Instance>,
    scene_bounds: &mut Bounds,
    node_count: &mut u64,
) -> anyhow::Result<()> {
    *node_count += 1;
    let local = Mat4::from_cols_array_2d(&node.transform().matrix());
    let world = parent_transform * local;
    if let Some(mesh) = node.mesh() {
        for primitive in mesh.primitives() {
            let (geometry, material) = *primitive_map
                .get(&(mesh.index(), primitive.index()))
                .context("internal primitive mapping is incomplete")?;
            let linear = Mat3::from_mat4(world);
            let normal_transform = if linear.determinant().abs() > 1.0e-12 {
                linear.inverse().transpose()
            } else {
                Mat3::IDENTITY
            };
            scene_bounds.include_transformed(&geometries[geometry].bounds, world);
            instances.push(Instance {
                geometry,
                material,
                transform: world,
                normal_transform,
                node_name: node.name().map(ToOwned::to_owned),
            });
        }
    }
    for child in node.children() {
        visit_node(
            child,
            world,
            primitive_map,
            geometries,
            instances,
            scene_bounds,
            node_count,
        )?;
    }
    Ok(())
}

fn compile_textures(gltf: &gltf::Gltf, blob: &[u8]) -> anyhow::Result<Vec<SourceTexture>> {
    gltf.textures()
        .map(|texture| {
            let image = texture.source();
            let encoded = match image.source() {
                gltf::image::Source::View { view, .. } => {
                    let start = view.offset();
                    let end = start
                        .checked_add(view.length())
                        .context("embedded image buffer range overflowed")?;
                    blob.get(start..end).with_context(|| {
                        format!("embedded image {} is outside the GLB buffer", image.index())
                    })?
                }
                gltf::image::Source::Uri { uri, .. } => {
                    bail!("GLB texture URI '{uri}' is unsupported; embed images in the GLB")
                }
            };
            let sampler = texture.sampler();
            Ok(SourceTexture {
                encoded: encoded.to_vec(),
                sampler: TextureSampler {
                    mag_linear: !matches!(
                        sampler.mag_filter(),
                        Some(gltf::texture::MagFilter::Nearest)
                    ),
                    min_linear: !matches!(
                        sampler.min_filter(),
                        Some(
                            gltf::texture::MinFilter::Nearest
                                | gltf::texture::MinFilter::NearestMipmapNearest
                                | gltf::texture::MinFilter::NearestMipmapLinear
                        )
                    ),
                    mipmap_linear: matches!(
                        sampler.min_filter(),
                        Some(
                            gltf::texture::MinFilter::LinearMipmapLinear
                                | gltf::texture::MinFilter::NearestMipmapLinear
                        )
                    ),
                    wrap_u: convert_wrap(sampler.wrap_s()),
                    wrap_v: convert_wrap(sampler.wrap_t()),
                },
                decoded: None,
            })
        })
        .collect()
}

fn compile_material(material: gltf::Material<'_>) -> anyhow::Result<SourceMaterial> {
    let pbr = material.pbr_metallic_roughness();
    let base_color_texture = pbr.base_color_texture().map(|info| TextureReference {
        texture: info.texture().index(),
        tex_coord: info.tex_coord(),
    });
    let metallic_roughness_texture =
        pbr.metallic_roughness_texture()
            .map(|info| TextureReference {
                texture: info.texture().index(),
                tex_coord: info.tex_coord(),
            });
    let normal = material.normal_texture();
    let normal_texture = normal.as_ref().map(|info| TextureReference {
        texture: info.texture().index(),
        tex_coord: info.tex_coord(),
    });
    let occlusion = material.occlusion_texture();
    let occlusion_texture = occlusion.as_ref().map(|info| TextureReference {
        texture: info.texture().index(),
        tex_coord: info.tex_coord(),
    });
    let emissive_texture = material.emissive_texture().map(|info| TextureReference {
        texture: info.texture().index(),
        tex_coord: info.tex_coord(),
    });
    for reference in [
        base_color_texture,
        metallic_roughness_texture,
        normal_texture,
        occlusion_texture,
        emissive_texture,
    ]
    .into_iter()
    .flatten()
    {
        if reference.tex_coord > 1 {
            bail!(
                "material {:?} uses unsupported TEXCOORD_{}; source mode supports sets 0 and 1",
                material.name(),
                reference.tex_coord
            );
        }
    }
    Ok(SourceMaterial {
        name: material.name().map(ToOwned::to_owned),
        base_color_factor: pbr.base_color_factor(),
        base_color_texture,
        metallic_factor: pbr.metallic_factor(),
        roughness_factor: pbr.roughness_factor(),
        metallic_roughness_texture,
        normal_texture,
        normal_scale: normal.map(|info| info.scale()).unwrap_or(1.0),
        occlusion_texture,
        occlusion_strength: occlusion.map(|info| info.strength()).unwrap_or(1.0),
        emissive_factor: material.emissive_factor(),
        emissive_texture,
        alpha_mode: match material.alpha_mode() {
            gltf::material::AlphaMode::Opaque => AlphaMode::Opaque,
            gltf::material::AlphaMode::Mask => AlphaMode::Mask,
            gltf::material::AlphaMode::Blend => AlphaMode::Blend,
        },
        alpha_cutoff: material.alpha_cutoff().unwrap_or(0.5),
        double_sided: material.double_sided(),
    })
}

fn default_source_material() -> SourceMaterial {
    SourceMaterial {
        name: None,
        base_color_factor: [1.0; 4],
        base_color_texture: None,
        metallic_factor: 1.0,
        roughness_factor: 1.0,
        metallic_roughness_texture: None,
        normal_texture: None,
        normal_scale: 1.0,
        occlusion_texture: None,
        occlusion_strength: 1.0,
        emissive_factor: [0.0; 3],
        emissive_texture: None,
        alpha_mode: AlphaMode::Opaque,
        alpha_cutoff: 0.5,
        double_sided: false,
    }
}

fn convert_wrap(mode: gltf::texture::WrappingMode) -> TextureWrap {
    match mode {
        gltf::texture::WrappingMode::ClampToEdge => TextureWrap::ClampToEdge,
        gltf::texture::WrappingMode::MirroredRepeat => TextureWrap::MirroredRepeat,
        gltf::texture::WrappingMode::Repeat => TextureWrap::Repeat,
    }
}

fn normalization_transform(up_axis: UpAxis) -> Mat4 {
    match up_axis {
        UpAxis::Y => Mat4::IDENTITY,
        UpAxis::Z => Mat4::from_rotation_x(-std::f32::consts::FRAC_PI_2),
        UpAxis::X => Mat4::from_rotation_z(std::f32::consts::FRAC_PI_2),
    }
}

fn generate_normals(positions: &[[f32; 3]], indices: &[u32]) -> Vec<[f32; 3]> {
    let mut normals = vec![Vec3::ZERO; positions.len()];
    for triangle in indices.chunks_exact(3) {
        let a = Vec3::from_array(positions[triangle[0] as usize]);
        let b = Vec3::from_array(positions[triangle[1] as usize]);
        let c = Vec3::from_array(positions[triangle[2] as usize]);
        let face = (b - a).cross(c - a);
        for index in triangle {
            normals[*index as usize] += face;
        }
    }
    normals
        .into_iter()
        .map(|normal| normal.try_normalize().unwrap_or(Vec3::Y).to_array())
        .collect()
}

fn hash_geometry(
    vertices: &[Vertex],
    source_attributes: Option<&[SourceVertexAttributes]>,
    indices: &[u32],
) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(&(vertices.len() as u64).to_le_bytes());
    hasher.update(bytemuck::cast_slice(vertices));
    if let Some(attributes) = source_attributes {
        hasher.update(bytemuck::cast_slice(attributes));
    }
    hasher.update(&(indices.len() as u64).to_le_bytes());
    hasher.update(bytemuck::cast_slice(indices));
    *hasher.finalize().as_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_triangle_normals_face_positive_z() {
        let positions = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        let normals = generate_normals(&positions, &[0, 1, 2]);
        assert_eq!(normals, vec![[0.0, 0.0, 1.0]; 3]);
    }

    #[test]
    fn transformed_bounds_include_all_corners() {
        let source = Bounds {
            min: [-1.0, -2.0, -3.0],
            max: [1.0, 2.0, 3.0],
        };
        let mut target = Bounds::empty();
        target.include_transformed(&source, Mat4::from_translation(Vec3::splat(5.0)));
        assert_eq!(target.min, [4.0, 3.0, 2.0]);
        assert_eq!(target.max, [6.0, 7.0, 8.0]);
    }

    #[test]
    fn parses_binary_stl_without_an_import_framework() {
        let mut bytes = vec![0_u8; 84];
        bytes[80..84].copy_from_slice(&1_u32.to_le_bytes());
        for value in [
            0.0_f32, 0.0, 1.0, // normal
            0.0, 0.0, 0.0, // a
            1.0, 0.0, 0.0, // b
            0.0, 1.0, 0.0, // c
        ] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes.extend_from_slice(&0_u16.to_le_bytes());
        let (vertices, indices) = parse_stl(&bytes).unwrap();
        assert_eq!(vertices.len(), 3);
        assert_eq!(indices, [0, 1, 2]);
        assert_eq!(vertices[0].normal, [0.0, 0.0, 1.0]);
    }

    #[test]
    fn parses_ascii_stl_case_insensitively() {
        let bytes = b"SOLID part\nFACET normal 0 0 1\nOUTER LOOP\nVERTEX 0 0 0\nVERTEX 1 0 0\nVERTEX 0 1 0\nENDLOOP\nENDFACET\nENDSOLID";
        let (vertices, indices) = parse_stl(bytes).unwrap();
        assert_eq!(vertices.len(), 3);
        assert_eq!(indices, [0, 1, 2]);
        assert_eq!(vertices[2].position, [0.0, 1.0, 0.0]);
    }
}
