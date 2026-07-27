mod wgpu_renderer;

use serde::Serialize;

use crate::{
    camera::PreparedCamera,
    config::{LightingConfig, RenderConfig},
    scene::CompiledScene,
    timing::Timings,
};

pub use wgpu_renderer::WgpuRenderer;

#[derive(Debug, Clone, Serialize)]
pub struct HardwareFingerprint {
    pub backend: String,
    pub adapter: String,
    pub vendor_id: u32,
    pub device_id: u32,
    pub device_type: String,
    pub driver: String,
    pub driver_info: String,
}

#[derive(Debug)]
pub struct RenderedImage {
    pub view: String,
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    pub tiles: Vec<RenderedTile>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RenderedTile {
    pub view: String,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug)]
pub struct RenderBatch {
    pub images: Vec<RenderedImage>,
    pub timings: Timings,
}

pub trait Renderer {
    fn fingerprint(&self) -> &HardwareFingerprint;

    fn render_views(
        &mut self,
        scene: &CompiledScene,
        cameras: &[PreparedCamera],
        render: &RenderConfig,
        lighting: &LightingConfig,
    ) -> anyhow::Result<RenderBatch>;
}
