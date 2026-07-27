use std::{collections::BTreeSet, mem, ops::Range, sync::mpsc, thread, time::Instant};

use anyhow::Context;
use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

use crate::{
    camera::PreparedCamera,
    config::{LightingConfig, MaterialMode, RenderConfig, parse_hex_color},
    renderer::{HardwareFingerprint, RenderBatch, RenderedImage, Renderer},
    scene::{AlphaMode, CompiledScene, SourceMaterial, TextureWrap, Vertex},
    timing::Timings,
};

const COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
const COPY_ALIGNMENT: u32 = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct GlobalsRaw {
    view_projection: [[f32; 4]; 4],
    light_direction_ambient: [f32; 4],
    light_color_intensity: [f32; 4],
    base_color: [f32; 4],
    camera_position: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct MaterialRaw {
    base_color_factor: [f32; 4],
    emissive_alpha_cutoff: [f32; 4],
    metallic_roughness_normal_occlusion: [f32; 4],
    tex_coord_sets: [f32; 4],
    occlusion_alpha_mode: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct TechnicalVertex {
    position: [f32; 3],
    normal: [f32; 3],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct InstanceRaw {
    model: [[f32; 4]; 4],
    normal_0: [f32; 4],
    normal_1: [f32; 4],
    normal_2: [f32; 4],
}

struct Draw {
    indices: Range<u32>,
    instances: Range<u32>,
    material: usize,
}

struct GpuTexture {
    _texture: wgpu::Texture,
    linear_view: wgpu::TextureView,
    srgb_view: wgpu::TextureView,
    sampler: wgpu::Sampler,
}

struct GpuMaterial {
    bind_group: wgpu::BindGroup,
    _uniform: wgpu::Buffer,
    alpha_mode: AlphaMode,
    double_sided: bool,
}

struct GpuScene {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    instance_buffer: wgpu::Buffer,
    draws: Vec<Draw>,
    materials: Vec<GpuMaterial>,
    _textures: Vec<GpuTexture>,
}

struct SourcePipelines {
    opaque_culled: wgpu::RenderPipeline,
    opaque_double_sided: wgpu::RenderPipeline,
    blend_culled: wgpu::RenderPipeline,
    blend_double_sided: wgpu::RenderPipeline,
}

impl SourcePipelines {
    fn select(&self, alpha_mode: AlphaMode, double_sided: bool) -> &wgpu::RenderPipeline {
        match (alpha_mode == AlphaMode::Blend, double_sided) {
            (false, false) => &self.opaque_culled,
            (false, true) => &self.opaque_double_sided,
            (true, false) => &self.blend_culled,
            (true, true) => &self.blend_double_sided,
        }
    }
}

struct ViewResources {
    id: String,
    output_texture: wgpu::Texture,
    output_view: wgpu::TextureView,
    _multisample_texture: Option<wgpu::Texture>,
    multisample_view: Option<wgpu::TextureView>,
    _depth_texture: wgpu::Texture,
    depth_view: wgpu::TextureView,
    readback: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    _uniform: wgpu::Buffer,
}

pub struct WgpuRenderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    fingerprint: HardwareFingerprint,
    bind_group_layout: wgpu::BindGroupLayout,
    material_bind_group_layout: wgpu::BindGroupLayout,
    pipeline_single_sample: Option<wgpu::RenderPipeline>,
    pipeline_multisample: Option<wgpu::RenderPipeline>,
    source_single_sample: Option<SourcePipelines>,
    source_multisample: Option<SourcePipelines>,
    initialization_timings: Timings,
}

impl WgpuRenderer {
    pub fn new() -> anyhow::Result<Self> {
        let mut timings = Timings::default();
        let adapter_started = Instant::now();
        let mut instance_descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
        #[cfg(target_os = "windows")]
        {
            instance_descriptor.backends = wgpu::Backends::DX12;
        }
        instance_descriptor = instance_descriptor.with_env();
        let instance = wgpu::Instance::new(instance_descriptor);
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: None,
            apply_limit_buckets: false,
        }))
        .context("no compatible GPU adapter was found")?;
        timings.record("gpu_adapter", adapter_started.elapsed());

        let device_started = Instant::now();
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("v3 device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
            memory_hints: wgpu::MemoryHints::MemoryUsage,
            trace: wgpu::Trace::Off,
        }))
        .context("failed to create GPU device")?;
        timings.record("gpu_device", device_started.elapsed());

        let info = adapter.get_info();
        let fingerprint = HardwareFingerprint {
            backend: format!("{:?}", info.backend).to_ascii_lowercase(),
            adapter: info.name,
            vendor_id: info.vendor,
            device_id: info.device,
            device_type: format!("{:?}", info.device_type).to_ascii_lowercase(),
            driver: info.driver,
            driver_info: info.driver_info,
        };

        let pipeline_started = Instant::now();
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("v3 globals layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let material_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("v3 source material layout"),
                entries: &source_material_layout_entries(),
            });
        timings.record("gpu_layout", pipeline_started.elapsed());

        Ok(Self {
            device,
            queue,
            fingerprint,
            bind_group_layout,
            material_bind_group_layout,
            pipeline_single_sample: None,
            pipeline_multisample: None,
            source_single_sample: None,
            source_multisample: None,
            initialization_timings: timings,
        })
    }

    pub fn initialization_timings(&self) -> &Timings {
        &self.initialization_timings
    }

    fn ensure_pipeline(
        &mut self,
        material_mode: MaterialMode,
        sample_count: u32,
        timings: &mut Timings,
    ) {
        let already_created = match (material_mode, sample_count) {
            (MaterialMode::Technical, 1) => self.pipeline_single_sample.is_some(),
            (MaterialMode::Technical, _) => self.pipeline_multisample.is_some(),
            (MaterialMode::Source, 1) => self.source_single_sample.is_some(),
            (MaterialMode::Source, _) => self.source_multisample.is_some(),
        };
        if already_created {
            return;
        }
        let started = Instant::now();
        match material_mode {
            MaterialMode::Technical => {
                let shader = self
                    .device
                    .create_shader_module(wgpu::ShaderModuleDescriptor {
                        label: Some("v3 technical shader"),
                        source: wgpu::ShaderSource::Wgsl(include_str!("technical.wgsl").into()),
                    });
                let layout = self
                    .device
                    .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                        label: Some("v3 technical pipeline layout"),
                        bind_group_layouts: &[Some(&self.bind_group_layout)],
                        immediate_size: 0,
                    });
                let pipeline = create_pipeline(&self.device, &shader, &layout, sample_count);
                if sample_count == 1 {
                    self.pipeline_single_sample = Some(pipeline);
                } else {
                    self.pipeline_multisample = Some(pipeline);
                }
            }
            MaterialMode::Source => {
                let shader = self
                    .device
                    .create_shader_module(wgpu::ShaderModuleDescriptor {
                        label: Some("v3 source material shader"),
                        source: wgpu::ShaderSource::Wgsl(
                            include_str!("source_material.wgsl").into(),
                        ),
                    });
                let layout = self
                    .device
                    .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                        label: Some("v3 source material pipeline layout"),
                        bind_group_layouts: &[
                            Some(&self.bind_group_layout),
                            Some(&self.material_bind_group_layout),
                        ],
                        immediate_size: 0,
                    });
                let pipelines =
                    create_source_pipelines(&self.device, &shader, &layout, sample_count);
                if sample_count == 1 {
                    self.source_single_sample = Some(pipelines);
                } else {
                    self.source_multisample = Some(pipelines);
                }
            }
        }
        timings.record("gpu_pipeline", started.elapsed());
    }

    fn upload_scene(
        &self,
        scene: &CompiledScene,
        material_mode: MaterialMode,
        timings: &mut Timings,
    ) -> anyhow::Result<GpuScene> {
        let mut vertices = Vec::<Vertex>::new();
        let mut indices = Vec::<u32>::new();
        let mut index_ranges = Vec::<Range<u32>>::with_capacity(scene.geometries.len());
        for geometry in &scene.geometries {
            let vertex_base = vertices.len() as u32;
            let index_start = indices.len() as u32;
            vertices.extend_from_slice(&geometry.vertices);
            indices.extend(
                geometry
                    .indices
                    .iter()
                    .map(|index| index.saturating_add(vertex_base)),
            );
            index_ranges.push(index_start..indices.len() as u32);
        }

        let mut instances = Vec::<InstanceRaw>::new();
        let mut draws = Vec::<Draw>::new();
        for (geometry_index, index_range) in index_ranges.into_iter().enumerate() {
            let material_indices = scene
                .instances
                .iter()
                .filter(|instance| instance.geometry == geometry_index)
                .map(|instance| instance.material)
                .collect::<BTreeSet<_>>();
            for material in material_indices {
                let instance_start = instances.len() as u32;
                for instance in scene.instances.iter().filter(|instance| {
                    instance.geometry == geometry_index && instance.material == material
                }) {
                    let normal = instance.normal_transform.to_cols_array_2d();
                    instances.push(InstanceRaw {
                        model: instance.transform.to_cols_array_2d(),
                        normal_0: [normal[0][0], normal[0][1], normal[0][2], 0.0],
                        normal_1: [normal[1][0], normal[1][1], normal[1][2], 0.0],
                        normal_2: [normal[2][0], normal[2][1], normal[2][2], 0.0],
                    });
                }
                let instance_end = instances.len() as u32;
                if instance_end > instance_start {
                    draws.push(Draw {
                        indices: index_range.clone(),
                        instances: instance_start..instance_end,
                        material,
                    });
                }
            }
        }

        if vertices.is_empty() || indices.is_empty() || instances.is_empty() {
            anyhow::bail!("compiled scene contains no GPU data");
        }
        let vertex_buffer = match material_mode {
            MaterialMode::Technical => {
                let compact_vertices = vertices
                    .iter()
                    .map(|vertex| TechnicalVertex {
                        position: vertex.position,
                        normal: vertex.normal,
                    })
                    .collect::<Vec<_>>();
                self.device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("v3 compact technical vertices"),
                        contents: bytemuck::cast_slice(&compact_vertices),
                        usage: wgpu::BufferUsages::VERTEX,
                    })
            }
            MaterialMode::Source => {
                self.device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("v3 source vertices"),
                        contents: bytemuck::cast_slice(&vertices),
                        usage: wgpu::BufferUsages::VERTEX,
                    })
            }
        };
        let index_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("v3 indices"),
                contents: bytemuck::cast_slice(&indices),
                usage: wgpu::BufferUsages::INDEX,
            });
        let instance_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("v3 instances"),
                contents: bytemuck::cast_slice(&instances),
                usage: wgpu::BufferUsages::VERTEX,
            });
        let (materials, textures) = if material_mode == MaterialMode::Source {
            self.upload_materials(scene, timings)?
        } else {
            (Vec::new(), Vec::new())
        };
        Ok(GpuScene {
            vertex_buffer,
            index_buffer,
            instance_buffer,
            draws,
            materials,
            _textures: textures,
        })
    }

    fn upload_materials(
        &self,
        scene: &CompiledScene,
        timings: &mut Timings,
    ) -> anyhow::Result<(Vec<GpuMaterial>, Vec<GpuTexture>)> {
        let decode_started = Instant::now();
        let worker_count = scene.textures.len().min(
            thread::available_parallelism()
                .map(usize::from)
                .unwrap_or(1),
        );
        let mut decoded_textures = Vec::with_capacity(scene.textures.len());
        decoded_textures.resize_with(scene.textures.len(), || None);
        if worker_count > 0 {
            let decoded_batches = thread::scope(|scope| {
                let mut handles = Vec::with_capacity(worker_count);
                for worker in 0..worker_count {
                    handles.push(scope.spawn(move || -> anyhow::Result<Vec<_>> {
                        let mut batch = Vec::new();
                        for index in (worker..scene.textures.len()).step_by(worker_count) {
                            let texture = &scene.textures[index];
                            let decoded = image::load_from_memory(&texture.encoded)
                                .with_context(|| format!("failed to decode v3 texture {index}"))?
                                .to_rgba8();
                            batch.push((index, decoded));
                        }
                        Ok(batch)
                    }));
                }
                handles
                    .into_iter()
                    .map(|handle| handle.join().expect("texture decoder thread panicked"))
                    .collect::<anyhow::Result<Vec<_>>>()
            })?;
            for batch in decoded_batches {
                for (index, decoded) in batch {
                    decoded_textures[index] = Some(decoded);
                }
            }
        }
        timings.record("texture_decode", decode_started.elapsed());

        let mut textures = Vec::with_capacity(scene.textures.len() + 3);
        for (index, (texture, decoded)) in scene
            .textures
            .iter()
            .zip(decoded_textures.into_iter())
            .enumerate()
        {
            let decoded = decoded.with_context(|| format!("texture {index} was not decoded"))?;
            let texture_started = Instant::now();
            textures.push(self.upload_rgba_texture(
                decoded.width(),
                decoded.height(),
                decoded.as_raw(),
                texture.sampler,
                &format!("v3 texture {index}"),
            ));
            timings.accumulate("texture_upload", texture_started.elapsed());
        }

        let fallback_sampler = crate::scene::TextureSampler {
            mag_linear: true,
            min_linear: true,
            mipmap_linear: false,
            wrap_u: TextureWrap::Repeat,
            wrap_v: TextureWrap::Repeat,
        };
        let white_index = textures.len();
        textures.push(self.upload_rgba_texture(
            1,
            1,
            &[255, 255, 255, 255],
            fallback_sampler,
            "v3 white fallback",
        ));
        let normal_index = textures.len();
        textures.push(self.upload_rgba_texture(
            1,
            1,
            &[128, 128, 255, 255],
            fallback_sampler,
            "v3 normal fallback",
        ));
        let black_index = textures.len();
        textures.push(self.upload_rgba_texture(
            1,
            1,
            &[0, 0, 0, 255],
            fallback_sampler,
            "v3 black fallback",
        ));

        let mut materials = Vec::with_capacity(scene.materials.len());
        for material in &scene.materials {
            let base_index = material
                .base_color_texture
                .map(|reference| reference.texture)
                .unwrap_or(white_index);
            let metallic_index = material
                .metallic_roughness_texture
                .map(|reference| reference.texture)
                .unwrap_or(white_index);
            let material_normal_index = material
                .normal_texture
                .map(|reference| reference.texture)
                .unwrap_or(normal_index);
            let emissive_index = material
                .emissive_texture
                .map(|reference| reference.texture)
                .unwrap_or(black_index);
            let occlusion_index = material
                .occlusion_texture
                .map(|reference| reference.texture)
                .unwrap_or(white_index);
            let raw = material_raw(material);
            let uniform = self
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("v3 source material uniform"),
                    contents: bytemuck::bytes_of(&raw),
                    usage: wgpu::BufferUsages::UNIFORM,
                });
            let base = &textures[base_index];
            let metallic = &textures[metallic_index];
            let normal = &textures[material_normal_index];
            let emissive = &textures[emissive_index];
            let occlusion = &textures[occlusion_index];
            let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("v3 source material bind group"),
                layout: &self.material_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: uniform.as_entire_binding(),
                    },
                    texture_entry(1, &base.srgb_view),
                    sampler_entry(2, &base.sampler),
                    texture_entry(3, &metallic.linear_view),
                    sampler_entry(4, &metallic.sampler),
                    texture_entry(5, &normal.linear_view),
                    sampler_entry(6, &normal.sampler),
                    texture_entry(7, &emissive.srgb_view),
                    sampler_entry(8, &emissive.sampler),
                    texture_entry(9, &occlusion.linear_view),
                    sampler_entry(10, &occlusion.sampler),
                ],
            });
            materials.push(GpuMaterial {
                bind_group,
                _uniform: uniform,
                alpha_mode: material.alpha_mode,
                double_sided: material.double_sided,
            });
        }
        Ok((materials, textures))
    }

    fn upload_rgba_texture(
        &self,
        width: u32,
        height: u32,
        rgba: &[u8],
        sampler: crate::scene::TextureSampler,
        label: &str,
    ) -> GpuTexture {
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[wgpu::TextureFormat::Rgba8UnormSrgb],
        });
        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * 4),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        let linear_view = texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("v3 linear texture view"),
            format: Some(wgpu::TextureFormat::Rgba8Unorm),
            ..Default::default()
        });
        let srgb_view = texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("v3 srgb texture view"),
            format: Some(wgpu::TextureFormat::Rgba8UnormSrgb),
            ..Default::default()
        });
        let sampler = self.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("v3 glTF sampler"),
            address_mode_u: address_mode(sampler.wrap_u),
            address_mode_v: address_mode(sampler.wrap_v),
            mag_filter: filter_mode(sampler.mag_linear),
            min_filter: filter_mode(sampler.min_linear),
            mipmap_filter: mipmap_filter_mode(sampler.mipmap_linear),
            ..Default::default()
        });
        GpuTexture {
            _texture: texture,
            linear_view,
            srgb_view,
            sampler,
        }
    }
}

impl Renderer for WgpuRenderer {
    fn fingerprint(&self) -> &HardwareFingerprint {
        &self.fingerprint
    }

    fn render_views(
        &mut self,
        scene: &CompiledScene,
        cameras: &[PreparedCamera],
        render: &RenderConfig,
        lighting: &LightingConfig,
    ) -> anyhow::Result<RenderBatch> {
        if cameras.is_empty() {
            anyhow::bail!("render batch contains no cameras");
        }
        let mut timings = Timings::default();
        timings.merge(&self.initialization_timings);

        let width = render.resolution[0];
        let height = render.resolution[1];
        let sample_count = if render.antialias { 4 } else { 1 };
        self.ensure_pipeline(render.material_mode, sample_count, &mut timings);

        let upload_started = Instant::now();
        let gpu_scene = self.upload_scene(scene, render.material_mode, &mut timings)?;
        timings.record("gpu_upload", upload_started.elapsed());

        let padded_bytes_per_row = align_to(width * 4, COPY_ALIGNMENT);
        let readback_size = u64::from(padded_bytes_per_row) * u64::from(height);
        let background = parse_hex_color(&render.background)?;
        let base_color = parse_hex_color(&render.base_color)?;
        let light_color = parse_hex_color(&lighting.color)?;
        let mut resources = Vec::with_capacity(cameras.len());

        for camera in cameras {
            let output_texture = create_color_texture(&self.device, width, height, 1, "v3 output");
            let output_view = output_texture.create_view(&Default::default());
            let multisample_texture = (sample_count > 1).then(|| {
                create_color_texture(
                    &self.device,
                    width,
                    height,
                    sample_count,
                    "v3 multisample color",
                )
            });
            let multisample_view = multisample_texture
                .as_ref()
                .map(|texture| texture.create_view(&Default::default()));
            let depth_texture = create_depth_texture(&self.device, width, height, sample_count);
            let depth_view = depth_texture.create_view(&Default::default());
            let readback = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("v3 readback"),
                size: readback_size,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });
            let globals = GlobalsRaw {
                view_projection: camera.view_projection.to_cols_array_2d(),
                light_direction_ambient: [
                    lighting.direction[0],
                    lighting.direction[1],
                    lighting.direction[2],
                    lighting.ambient,
                ],
                light_color_intensity: [
                    light_color[0],
                    light_color[1],
                    light_color[2],
                    lighting.intensity,
                ],
                base_color,
                camera_position: [
                    camera.position[0],
                    camera.position[1],
                    camera.position[2],
                    1.0,
                ],
            };
            let uniform = self
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("v3 camera and lighting"),
                    contents: bytemuck::bytes_of(&globals),
                    usage: wgpu::BufferUsages::UNIFORM,
                });
            let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("v3 camera and lighting bind group"),
                layout: &self.bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform.as_entire_binding(),
                }],
            });
            resources.push(ViewResources {
                id: camera.id.clone(),
                output_texture,
                output_view,
                _multisample_texture: multisample_texture,
                multisample_view,
                _depth_texture: depth_texture,
                depth_view,
                readback,
                bind_group,
                _uniform: uniform,
            });
        }

        let render_started = Instant::now();
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("v3 render batch"),
            });
        let technical_pipeline = if render.material_mode == MaterialMode::Technical {
            Some(
                if sample_count > 1 {
                    self.pipeline_multisample.as_ref()
                } else {
                    self.pipeline_single_sample.as_ref()
                }
                .context("technical pipeline was not initialized")?,
            )
        } else {
            None
        };
        let source_pipelines = if sample_count > 1 {
            self.source_multisample.as_ref()
        } else {
            self.source_single_sample.as_ref()
        };
        for resource in &resources {
            let color_view = resource
                .multisample_view
                .as_ref()
                .unwrap_or(&resource.output_view);
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("v3 render pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: color_view,
                        depth_slice: None,
                        resolve_target: resource
                            .multisample_view
                            .as_ref()
                            .map(|_| &resource.output_view),
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: f64::from(background[0]),
                                g: f64::from(background[1]),
                                b: f64::from(background[2]),
                                a: 1.0,
                            }),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &resource.depth_view,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0),
                            store: wgpu::StoreOp::Discard,
                        }),
                        stencil_ops: None,
                    }),
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });
                pass.set_bind_group(0, &resource.bind_group, &[]);
                pass.set_vertex_buffer(0, gpu_scene.vertex_buffer.slice(..));
                pass.set_vertex_buffer(1, gpu_scene.instance_buffer.slice(..));
                pass.set_index_buffer(gpu_scene.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                if render.material_mode == MaterialMode::Technical {
                    pass.set_pipeline(
                        technical_pipeline.context("technical pipeline was not initialized")?,
                    );
                    for draw in &gpu_scene.draws {
                        pass.draw_indexed(draw.indices.clone(), 0, draw.instances.clone());
                    }
                } else {
                    let source_pipelines = source_pipelines
                        .context("source material pipelines were not initialized")?;
                    // Draw opaque and alpha-masked material batches first, then
                    // blended batches. Geometry remains shared between both.
                    for blend_phase in [false, true] {
                        for draw in &gpu_scene.draws {
                            let material =
                                gpu_scene.materials.get(draw.material).with_context(|| {
                                    format!("material {} was not uploaded", draw.material)
                                })?;
                            if (material.alpha_mode == AlphaMode::Blend) != blend_phase {
                                continue;
                            }
                            pass.set_pipeline(
                                source_pipelines.select(material.alpha_mode, material.double_sided),
                            );
                            pass.set_bind_group(1, &material.bind_group, &[]);
                            pass.draw_indexed(draw.indices.clone(), 0, draw.instances.clone());
                        }
                    }
                }
            }
            encoder.copy_texture_to_buffer(
                resource.output_texture.as_image_copy(),
                wgpu::TexelCopyBufferInfo {
                    buffer: &resource.readback,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(padded_bytes_per_row),
                        rows_per_image: Some(height),
                    },
                },
                wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
            );
        }
        self.queue.submit([encoder.finish()]);
        timings.record("gpu_encode_submit", render_started.elapsed());

        let readback_started = Instant::now();
        let mut receivers = Vec::with_capacity(resources.len());
        for resource in &resources {
            let (sender, receiver) = mpsc::sync_channel(1);
            resource
                .readback
                .slice(..)
                .map_async(wgpu::MapMode::Read, move |result| {
                    let _ = sender.send(result);
                });
            receivers.push(receiver);
        }
        self.device
            .poll(wgpu::PollType::wait_indefinitely())
            .context("GPU polling failed")?;
        for receiver in receivers {
            receiver
                .recv()
                .context("GPU readback callback was dropped")?
                .context("GPU readback mapping failed")?;
        }

        let mut images = Vec::with_capacity(resources.len());
        for resource in &resources {
            let mapped = resource
                .readback
                .slice(..)
                .get_mapped_range()
                .context("failed to access mapped GPU readback")?;
            let mut rgba = vec![0_u8; width as usize * height as usize * 4];
            for row in 0..height as usize {
                let source_start = row * padded_bytes_per_row as usize;
                let source_end = source_start + width as usize * 4;
                let target_start = row * width as usize * 4;
                rgba[target_start..target_start + width as usize * 4]
                    .copy_from_slice(&mapped[source_start..source_end]);
            }
            drop(mapped);
            resource.readback.unmap();
            images.push(RenderedImage {
                view: resource.id.clone(),
                width,
                height,
                rgba,
            });
        }
        timings.record("gpu_readback", readback_started.elapsed());
        Ok(RenderBatch { images, timings })
    }
}

fn source_material_layout_entries() -> Vec<wgpu::BindGroupLayoutEntry> {
    let mut entries = vec![wgpu::BindGroupLayoutEntry {
        binding: 0,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }];
    for (texture_binding, sampler_binding) in [(1, 2), (3, 4), (5, 6), (7, 8), (9, 10)] {
        entries.push(wgpu::BindGroupLayoutEntry {
            binding: texture_binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        });
        entries.push(wgpu::BindGroupLayoutEntry {
            binding: sampler_binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            count: None,
        });
    }
    entries
}

fn material_raw(material: &SourceMaterial) -> MaterialRaw {
    let texture_set = |reference: Option<crate::scene::TextureReference>| {
        reference
            .map(|reference| reference.tex_coord as f32)
            .unwrap_or(0.0)
    };
    let alpha_mode = match material.alpha_mode {
        AlphaMode::Opaque => 0.0,
        AlphaMode::Mask => 1.0,
        AlphaMode::Blend => 2.0,
    };
    MaterialRaw {
        base_color_factor: material.base_color_factor,
        emissive_alpha_cutoff: [
            material.emissive_factor[0],
            material.emissive_factor[1],
            material.emissive_factor[2],
            material.alpha_cutoff,
        ],
        metallic_roughness_normal_occlusion: [
            material.metallic_factor,
            material.roughness_factor,
            material.normal_scale,
            material.occlusion_strength,
        ],
        tex_coord_sets: [
            texture_set(material.base_color_texture),
            texture_set(material.metallic_roughness_texture),
            texture_set(material.normal_texture),
            texture_set(material.emissive_texture),
        ],
        occlusion_alpha_mode: [
            texture_set(material.occlusion_texture),
            alpha_mode,
            0.0,
            0.0,
        ],
    }
}

fn texture_entry(binding: u32, view: &wgpu::TextureView) -> wgpu::BindGroupEntry<'_> {
    wgpu::BindGroupEntry {
        binding,
        resource: wgpu::BindingResource::TextureView(view),
    }
}

fn sampler_entry(binding: u32, sampler: &wgpu::Sampler) -> wgpu::BindGroupEntry<'_> {
    wgpu::BindGroupEntry {
        binding,
        resource: wgpu::BindingResource::Sampler(sampler),
    }
}

fn address_mode(wrap: TextureWrap) -> wgpu::AddressMode {
    match wrap {
        TextureWrap::ClampToEdge => wgpu::AddressMode::ClampToEdge,
        TextureWrap::MirroredRepeat => wgpu::AddressMode::MirrorRepeat,
        TextureWrap::Repeat => wgpu::AddressMode::Repeat,
    }
}

fn filter_mode(linear: bool) -> wgpu::FilterMode {
    if linear {
        wgpu::FilterMode::Linear
    } else {
        wgpu::FilterMode::Nearest
    }
}

fn mipmap_filter_mode(linear: bool) -> wgpu::MipmapFilterMode {
    if linear {
        wgpu::MipmapFilterMode::Linear
    } else {
        wgpu::MipmapFilterMode::Nearest
    }
}

fn create_source_pipelines(
    device: &wgpu::Device,
    shader: &wgpu::ShaderModule,
    layout: &wgpu::PipelineLayout,
    sample_count: u32,
) -> SourcePipelines {
    SourcePipelines {
        opaque_culled: create_source_pipeline(device, shader, layout, sample_count, false, false),
        opaque_double_sided: create_source_pipeline(
            device,
            shader,
            layout,
            sample_count,
            false,
            true,
        ),
        blend_culled: create_source_pipeline(device, shader, layout, sample_count, true, false),
        blend_double_sided: create_source_pipeline(
            device,
            shader,
            layout,
            sample_count,
            true,
            true,
        ),
    }
}

fn create_source_pipeline(
    device: &wgpu::Device,
    shader: &wgpu::ShaderModule,
    layout: &wgpu::PipelineLayout,
    sample_count: u32,
    blend: bool,
    double_sided: bool,
) -> wgpu::RenderPipeline {
    const VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 5] = [
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x3,
            offset: 0,
            shader_location: 0,
        },
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x3,
            offset: 12,
            shader_location: 1,
        },
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x2,
            offset: 24,
            shader_location: 9,
        },
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x2,
            offset: 32,
            shader_location: 10,
        },
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x4,
            offset: 40,
            shader_location: 11,
        },
    ];
    const INSTANCE_ATTRIBUTES: [wgpu::VertexAttribute; 7] = [
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x4,
            offset: 0,
            shader_location: 2,
        },
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x4,
            offset: 16,
            shader_location: 3,
        },
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x4,
            offset: 32,
            shader_location: 4,
        },
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x4,
            offset: 48,
            shader_location: 5,
        },
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x4,
            offset: 64,
            shader_location: 6,
        },
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x4,
            offset: 80,
            shader_location: 7,
        },
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x4,
            offset: 96,
            shader_location: 8,
        },
    ];
    let buffers = [
        Some(wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Vertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &VERTEX_ATTRIBUTES,
        }),
        Some(wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<InstanceRaw>() as u64,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &INSTANCE_ATTRIBUTES,
        }),
    ];
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("v3 source material pipeline"),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vertex_main"),
            compilation_options: Default::default(),
            buffers: &buffers,
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: if double_sided {
                None
            } else {
                Some(wgpu::Face::Back)
            },
            unclipped_depth: false,
            polygon_mode: wgpu::PolygonMode::Fill,
            conservative: false,
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: Some(!blend),
            depth_compare: Some(wgpu::CompareFunction::Less),
            stencil: Default::default(),
            bias: Default::default(),
        }),
        multisample: wgpu::MultisampleState {
            count: sample_count,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fragment_main"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: COLOR_FORMAT,
                blend: blend.then_some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}

fn create_pipeline(
    device: &wgpu::Device,
    shader: &wgpu::ShaderModule,
    layout: &wgpu::PipelineLayout,
    sample_count: u32,
) -> wgpu::RenderPipeline {
    const VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 2] = [
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x3,
            offset: 0,
            shader_location: 0,
        },
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x3,
            offset: 12,
            shader_location: 1,
        },
    ];
    const INSTANCE_ATTRIBUTES: [wgpu::VertexAttribute; 7] = [
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x4,
            offset: 0,
            shader_location: 2,
        },
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x4,
            offset: 16,
            shader_location: 3,
        },
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x4,
            offset: 32,
            shader_location: 4,
        },
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x4,
            offset: 48,
            shader_location: 5,
        },
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x4,
            offset: 64,
            shader_location: 6,
        },
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x4,
            offset: 80,
            shader_location: 7,
        },
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x4,
            offset: 96,
            shader_location: 8,
        },
    ];
    let vertex_buffers = [
        Some(wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<TechnicalVertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &VERTEX_ATTRIBUTES,
        }),
        Some(wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<InstanceRaw>() as u64,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &INSTANCE_ATTRIBUTES,
        }),
    ];
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("v3 technical pipeline"),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vertex_main"),
            compilation_options: Default::default(),
            buffers: &vertex_buffers,
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: Some(wgpu::Face::Back),
            unclipped_depth: false,
            polygon_mode: wgpu::PolygonMode::Fill,
            conservative: false,
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::Less),
            stencil: Default::default(),
            bias: Default::default(),
        }),
        multisample: wgpu::MultisampleState {
            count: sample_count,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fragment_main"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: COLOR_FORMAT,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}

fn create_color_texture(
    device: &wgpu::Device,
    width: u32,
    height: u32,
    sample_count: u32,
    label: &str,
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count,
        dimension: wgpu::TextureDimension::D2,
        format: COLOR_FORMAT,
        usage: if sample_count == 1 {
            wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC
        } else {
            wgpu::TextureUsages::RENDER_ATTACHMENT
        },
        view_formats: &[],
    })
}

fn create_depth_texture(
    device: &wgpu::Device,
    width: u32,
    height: u32,
    sample_count: u32,
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("v3 depth"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count,
        dimension: wgpu::TextureDimension::D2,
        format: DEPTH_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    })
}

fn align_to(value: u32, alignment: u32) -> u32 {
    value.div_ceil(alignment) * alignment
}
