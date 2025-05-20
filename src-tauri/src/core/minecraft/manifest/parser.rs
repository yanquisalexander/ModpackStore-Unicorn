use super::merger::ManifestMerger;
use crate::core::minecraft::paths::MinecraftPaths;
use serde_json::Value;
use std::fs;

pub struct ManifestParser<'a> {
    paths: &'a MinecraftPaths,
}

impl<'a> ManifestParser<'a> {
    pub fn new(paths: &'a MinecraftPaths) -> Self {
        Self { paths }
    }

    pub fn load_merged_manifest(&self) -> Option<Value> {
        let manifest_file = self.paths.manifest_file();
        log::info!("Loading version manifest from {}", manifest_file.display());

        let manifest_data = fs::read_to_string(&manifest_file).ok()?;
        let manifest_json: Value = serde_json::from_str(&manifest_data).ok()?;

        // Check for inheritance
        if let Some(inherits_from) = manifest_json.get("inheritsFrom").and_then(|v| v.as_str()) {
            log::info!("Found modded instance inheriting from {}", inherits_from);
            let vanilla_manifest_file = self.paths.vanilla_manifest_file(inherits_from);

            let vanilla_manifest_data = fs::read_to_string(&vanilla_manifest_file).ok()?;
            let vanilla_manifest: Value = serde_json::from_str(&vanilla_manifest_data).ok()?;

            return Some(ManifestMerger::merge(vanilla_manifest, manifest_json));
        }

        Some(manifest_json)
    }
}
