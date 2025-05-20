use crate::config::get_config_manager;
use crate::core::minecraft::{classpath::ClasspathBuilder, manifest::ManifestMerger};
use crate::core::minecraft_instance::MinecraftInstance;
use std::path::{Path, PathBuf};

use super::{launcher, ManifestParser};

#[derive(Debug)]
pub struct MinecraftPaths {
    game_dir: PathBuf,
    java_path: PathBuf,
    minecraft_version: String,
    forge_version: Option<String>,
}

impl MinecraftPaths {
    pub fn new(
        instance: &MinecraftInstance,
        config: &crate::config::ConfigManager,
    ) -> Option<Self> {
        log::info!(
            "[MinecraftPaths] Creating paths for instance: {}",
            instance.instanceName
        );

        let java_path = instance
            .javaPath
            .as_ref()
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                config
                    .get_java_dir()
                    .unwrap_or_else(|| PathBuf::from("default_java"))
            })
            .join("bin")
            .join(if cfg!(windows) { "javaw.exe" } else { "java" });

        let java_path = instance
            .javaPath
            .as_ref()
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                config
                    .get_java_dir()
                    .unwrap_or_else(|| PathBuf::from("default_java"))
            })
            .join("bin")
            .join(if cfg!(windows) { "javaw.exe" } else { "java" });

        let game_dir = instance
            .instanceDirectory
            .as_ref()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("default_path"))
            .join("minecraft");

        log::info!("[MinecraftPaths] Game directory: {}", game_dir.display());
        log::info!("[MinecraftPaths] Java path: {}", java_path.display());

        Some(Self {
            game_dir,
            java_path,
            minecraft_version: instance.minecraftVersion.clone(),
            forge_version: instance.forgeVersion.clone(),
        })
    }

    pub fn game_dir(&self) -> &Path {
        &self.game_dir
    }

    pub fn java_path(&self) -> &Path {
        &self.java_path
    }

    pub fn minecraft_version(&self) -> &str {
        &self.minecraft_version
    }

    pub fn forge_version(&self) -> Option<&str> {
        self.forge_version.as_deref()
    }

    pub fn manifest_file(&self) -> PathBuf {
        let version_dir = self.game_dir.join("versions");

        // Check if we need to find the Forge version
        if let Some(forge_ref) = &self.forge_version {
            log::info!(
                "[MinecraftPaths] Searching for Forge version manifest {}",
                forge_ref
            );

            // Try to get Forge version from launcher_profiles.json
            let launcher_profiles_path = self.game_dir.join("launcher_profiles.json");

            if launcher_profiles_path.exists() {
                if let Ok(data) = std::fs::read_to_string(&launcher_profiles_path) {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&data) {
                        log::debug!("[MinecraftPaths] Found launcher_profiles.json structure");

                        // Buscar en la estructura correcta: profiles -> forge -> lastVersionId
                        // O buscar cualquier perfil que tenga una versión Forge compatible
                        if let Some(profiles) = json.get("profiles").and_then(|v| v.as_object()) {
                            // Primero intentamos buscar un perfil explícitamente llamado "forge"
                            if let Some(forge_profile) = profiles.get("forge") {
                                if let Some(version_id) =
                                    forge_profile.get("lastVersionId").and_then(|v| v.as_str())
                                {
                                    if version_id.contains(&self.minecraft_version)
                                        && version_id.contains("forge")
                                    {
                                        log::info!("[MinecraftPaths] Found Forge version in 'forge' profile: {}", version_id);
                                        return version_dir
                                            .join(version_id)
                                            .join(format!("{}.json", version_id));
                                    }
                                }
                            }

                            // Si no encontramos un perfil específico de forge, buscamos en todos los perfiles
                            // una versión compatible con la versión de Minecraft y Forge solicitada
                            for (_, profile) in profiles {
                                if let Some(version_id) =
                                    profile.get("lastVersionId").and_then(|v| v.as_str())
                                {
                                    // Verificar si este perfil contiene la versión de Minecraft y referencia a Forge
                                    if version_id.contains(&self.minecraft_version)
                                        && version_id.contains("forge")
                                    {
                                        // Verificar si este perfil coincide con la versión específica de forge
                                        if version_id.contains(forge_ref) {
                                            log::info!("[MinecraftPaths] Found matching Forge version in profiles: {}", version_id);
                                            return version_dir
                                                .join(version_id)
                                                .join(format!("{}.json", version_id));
                                        }
                                    }
                                }
                            }
                        }

                        log::warn!("[MinecraftPaths] No matching Forge version found in launcher_profiles.json");
                    } else {
                        log::warn!(
                            "[MinecraftPaths] Failed to parse launcher_profiles.json as JSON"
                        );
                    }
                } else {
                    log::warn!("[MinecraftPaths] Failed to read launcher_profiles.json");
                }
            } else {
                log::warn!(
                    "[MinecraftPaths] launcher_profiles.json not found at {}",
                    launcher_profiles_path.display()
                );
            }

            // Fallback: Try to use the provided forge reference directly
            let forge_dir = format!("{}-forge-{}", self.minecraft_version, forge_ref);
            let forge_path = version_dir
                .join(&forge_dir)
                .join(format!("{}.json", forge_dir));

            if forge_path.exists() {
                log::info!(
                    "[MinecraftPaths] Using Forge manifest: {}",
                    forge_path.display()
                );
                return forge_path;
            }

            // Intentar formato alternativo para el directorio de Forge
            let alt_forge_dir = format!("{}-{}", self.minecraft_version, forge_ref);
            let alt_forge_path = version_dir
                .join(&alt_forge_dir)
                .join(format!("{}.json", alt_forge_dir));

            if alt_forge_path.exists() {
                log::info!(
                    "[MinecraftPaths] Using alternative Forge manifest: {}",
                    alt_forge_path.display()
                );
                return alt_forge_path;
            }
        }

        // Default to vanilla manifest
        log::info!("[MinecraftPaths] Using vanilla manifest file");
        version_dir
            .join(&self.minecraft_version)
            .join(format!("{}.json", self.minecraft_version))
    }

    pub fn vanilla_manifest_file(&self, version: &str) -> PathBuf {
        self.game_dir
            .join("versions")
            .join(version)
            .join(format!("{}.json", version))
    }

    pub fn client_jar(&self) -> PathBuf {
        // For simplicity, return vanilla client jar
        // In a real implementation, this would check for Forge client jars
        self.game_dir
            .join("versions")
            .join(&self.minecraft_version)
            .join(format!("{}.jar", self.minecraft_version))
    }

    pub fn libraries_dir(&self) -> PathBuf {
        self.game_dir.join("libraries")
    }

    pub fn assets_dir(&self) -> PathBuf {
        self.game_dir.join("assets")
    }

    pub fn natives_dir(&self) -> PathBuf {
        self.game_dir.join("natives").join(&self.minecraft_version)
    }

    pub fn classpath_str(&self) -> String {
        let binding = ManifestParser::new(self);
        let manifest_json = binding.load_merged_manifest().unwrap_or_default();
        let classpath_builder = ClasspathBuilder::new(&manifest_json, self);
        classpath_builder.build().unwrap_or_default()
    }
}
