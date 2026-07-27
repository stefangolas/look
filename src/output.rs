use std::{
    fs::{self, File},
    io::BufWriter,
    path::{Path, PathBuf},
};

use anyhow::Context;

use crate::{config::OutputConfig, renderer::RenderedImage};

pub fn output_path(config: &OutputConfig, view: &str, view_count: usize) -> PathBuf {
    if view_count == 1
        && let Some(path) = &config.single_file
    {
        return path.clone();
    }
    config.directory.join(config.naming.replace("{view}", view))
}

pub fn write_png(path: &Path, image: &RenderedImage) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create output directory '{}'", parent.display()))?;
    }
    let temporary = path.with_extension(format!(
        "{}.tmp",
        path.extension()
            .and_then(|value| value.to_str())
            .unwrap_or("png")
    ));
    let file = File::create(&temporary)
        .with_context(|| format!("failed to create '{}'", temporary.display()))?;
    let writer = BufWriter::new(file);
    let mut encoder = png::Encoder::new(writer, image.width, image.height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    // Fastest selects the ultra-fast DEFLATE path and a fixed Up filter,
    // avoiding adaptive per-row filter searches on the output critical path.
    encoder.set_compression(png::Compression::Fastest);
    let mut writer = encoder
        .write_header()
        .context("failed to write PNG header")?;
    writer
        .write_image_data(&image.rgba)
        .context("failed to encode PNG pixels")?;
    writer.finish().context("failed to finish PNG")?;
    if path.exists() {
        fs::remove_file(path)
            .with_context(|| format!("failed to replace existing output '{}'", path.display()))?;
    }
    fs::rename(&temporary, path).with_context(|| {
        format!(
            "failed to atomically move '{}' to '{}'",
            temporary.display(),
            path.display()
        )
    })?;
    Ok(())
}
