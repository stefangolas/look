use std::{
    env, fs,
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

use anyhow::Context;
use serde::{Deserialize, Serialize};

use crate::{config::UpAxis, scene::SceneStatistics};

const CACHE_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedInspection {
    schema_version: u32,
    compiler_version: String,
    source: PathBuf,
    source_size: u64,
    source_modified_ns: u64,
    up_axis: UpAxis,
    pub source_hash: String,
    pub statistics: SceneStatistics,
}

#[derive(Debug, Clone)]
pub struct MetadataCache {
    root: PathBuf,
}

impl MetadataCache {
    pub fn platform_default() -> Option<Self> {
        if let Some(path) = env::var_os("V3_CACHE_DIR") {
            return Some(Self::at(path));
        }
        #[cfg(target_os = "windows")]
        let root = env::var_os("LOCALAPPDATA").map(PathBuf::from);
        #[cfg(not(target_os = "windows"))]
        let root = env::var_os("XDG_CACHE_HOME")
            .map(PathBuf::from)
            .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")));
        root.map(|root| Self::at(root.join("v3").join("cache")))
    }

    pub fn at(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn load(&self, source: &Path, up_axis: UpAxis) -> anyhow::Result<Option<CachedInspection>> {
        let signature = SourceSignature::read(source)?;
        let path = self.entry_path(&signature.canonical);
        let bytes = match fs::read(path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error).context("failed to read metadata cache entry"),
        };
        let entry: CachedInspection =
            serde_json::from_slice(&bytes).context("invalid metadata cache entry")?;
        let valid = entry.schema_version == CACHE_SCHEMA_VERSION
            && entry.compiler_version == env!("CARGO_PKG_VERSION")
            && entry.source == signature.canonical
            && entry.source_size == signature.size
            && entry.source_modified_ns == signature.modified_ns
            && entry.up_axis == up_axis;
        Ok(valid.then_some(entry))
    }

    pub fn store(
        &self,
        source: &Path,
        up_axis: UpAxis,
        source_hash: &str,
        statistics: &SceneStatistics,
    ) -> anyhow::Result<()> {
        let signature = SourceSignature::read(source)?;
        let directory = self.root.join("inspections");
        fs::create_dir_all(&directory).context("failed to create metadata cache directory")?;
        let destination = self.entry_path(&signature.canonical);
        let temporary = destination.with_extension(format!("{}.tmp", std::process::id()));
        let entry = CachedInspection {
            schema_version: CACHE_SCHEMA_VERSION,
            compiler_version: env!("CARGO_PKG_VERSION").to_owned(),
            source: signature.canonical,
            source_size: signature.size,
            source_modified_ns: signature.modified_ns,
            up_axis,
            source_hash: source_hash.to_owned(),
            statistics: statistics.clone(),
        };
        let bytes = serde_json::to_vec(&entry).context("failed to encode metadata cache entry")?;
        fs::write(&temporary, bytes).context("failed to write metadata cache entry")?;
        if destination.exists() {
            fs::remove_file(&destination).context("failed to replace metadata cache entry")?;
        }
        fs::rename(&temporary, &destination).context("failed to commit metadata cache entry")?;
        Ok(())
    }

    fn entry_path(&self, canonical_source: &Path) -> PathBuf {
        let key = blake3::hash(canonical_source.to_string_lossy().as_bytes())
            .to_hex()
            .to_string();
        self.root.join("inspections").join(format!("{key}.json"))
    }
}

struct SourceSignature {
    canonical: PathBuf,
    size: u64,
    modified_ns: u64,
}

impl SourceSignature {
    fn read(source: &Path) -> anyhow::Result<Self> {
        let canonical = fs::canonicalize(source)
            .with_context(|| format!("failed to resolve source '{}'", source.display()))?;
        let metadata = fs::metadata(&canonical).context("failed to inspect source metadata")?;
        let modified_ns = metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .map(|duration| duration.as_nanos().min(u128::from(u64::MAX)) as u64)
            .unwrap_or(0);
        Ok(Self {
            canonical,
            size: metadata.len(),
            modified_ns,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::Bounds;

    #[test]
    fn metadata_round_trips_and_invalidates_on_source_change() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("model.glb");
        fs::write(&source, b"first").unwrap();
        let cache = MetadataCache::at(directory.path().join("cache"));
        let statistics = SceneStatistics {
            nodes: 1,
            mesh_primitives: 1,
            unique_geometries: 1,
            instances: 1,
            vertices: 3,
            triangles: 1,
            duplicate_geometries_removed: 0,
            materials: 1,
            textures: 0,
            bounds: Bounds {
                min: [0.0; 3],
                max: [1.0; 3],
            },
        };
        cache
            .store(&source, UpAxis::Y, "hash", &statistics)
            .unwrap();
        assert!(cache.load(&source, UpAxis::Y).unwrap().is_some());
        assert!(cache.load(&source, UpAxis::Z).unwrap().is_none());
        fs::write(&source, b"second-and-different-size").unwrap();
        assert!(cache.load(&source, UpAxis::Y).unwrap().is_none());
    }
}
