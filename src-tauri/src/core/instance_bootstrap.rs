// src-tauri/src/instance_bootstrap.rs
use crate::core::minecraft_instance::MinecraftInstance;
use serde_json::{json, Value};
use std::fs;
use std::io::{self, Result as IoResult};
use std::path::{Path, PathBuf};
use std::process::Command;
use tauri_plugin_http::reqwest;
use crate::GLOBAL_APP_HANDLE;
use tauri::Emitter;

pub struct InstanceBootstrap {
    client: reqwest::blocking::Client,
    // Cache para metadatos de versiones
    version_manifest_cache: Option<(Value, u64)>, // (datos, timestamp)
}

impl InstanceBootstrap {
    const MOJANG_VERSION_MANIFEST_URL: &'static str =
        "https://launchermeta.mojang.com/mc/game/version_manifest.json";
    const FORGE_API_BASE_URL: &'static str =
        "https://mrnavastar.github.io/ForgeVersionAPI/forge-versions.json";
    const CACHE_EXPIRY_MS: u64 = 3600000; // 1 hora

    pub fn new() -> Self {
        Self {
            client: reqwest::blocking::Client::new(),
            version_manifest_cache: None,
        }
    }

     // --- Helper Methods for Event Emission ---

    /// Emits a status update event to the frontend.
    /// Uses the global `AppHandle` to send events to all windows.
    ///
    /// # Arguments
    ///
    /// * `event_name` - The name of the event (e.g., "instance-launch-start").
    /// * `message` - A descriptive message for the frontend.
    fn emit_status(instance: &MinecraftInstance, event_name: &str, message: &str) {
        println!(
            "[Instance: {}] Emitting Event: {} - Message: {}",
            instance.instanceId, event_name, message
        );
        if let Ok(guard) = GLOBAL_APP_HANDLE.lock() {
            if let Some(app_handle) = guard.as_ref() {
                let payload = serde_json::json!({
                    "id": instance.instanceId,
                    "name": instance.instanceName, 
                    "message": message
                });
                // Use emit to notify the specific window listening for this event
                if let Err(e) = app_handle.emit(event_name, payload) {
                    eprintln!(
                        "[Bootstrap] Error emitting event '{}': {}",
                        event_name, e
                    );
                }
            } else {
                eprintln!(
                    "[Bootstrap] Error: GLOBAL_APP_HANDLE is None when trying to emit '{}'.",
                    event_name
                );
            }
        } else {
            eprintln!(
                "[Bootstrap] Error: Failed to lock GLOBAL_APP_HANDLE when trying to emit '{}'.",
                
                event_name
            );
        }
    }

    pub fn fetch_minecraft_versions(&mut self) -> Result<Vec<String>, String> {
        let root_node = self
            .get_version_manifest()
            .map_err(|e| format!("Error fetching version manifest: {}", e))?;

        let versions_node = root_node["versions"]
            .as_array()
            .ok_or_else(|| "Invalid version manifest format".to_string())?;

        let versions: Vec<String> = versions_node
            .iter()
            .filter_map(|v| v["id"].as_str().map(String::from))
            .collect();

        Ok(versions)
    }

    pub fn fetch_forge_versions(
        &self,
    ) -> Result<std::collections::HashMap<String, Vec<String>>, String> {
        let forge_data = self
            .client
            .get(Self::FORGE_API_BASE_URL)
            .send()
            .map_err(|e| format!("Error connecting to Forge API: {}", e))?
            .json::<Value>()
            .map_err(|e| format!("Error parsing Forge API response: {}", e))?;

        let mut forge_versions = std::collections::HashMap::new();

        if let Some(obj) = forge_data.as_object() {
            for (mc_version, forge_version_array) in obj {
                let forge_version_list: Vec<String> = forge_version_array
                    .as_array()
                    .unwrap_or(&Vec::new())
                    .iter()
                    .filter_map(|v| v["id"].as_str().map(String::from))
                    .collect();

                forge_versions.insert(mc_version.clone(), forge_version_list);
            }
        }

        Ok(forge_versions)
    }

    pub fn revalidate_assets(&mut self, instance: &MinecraftInstance) -> IoResult<()> {
        println!("Revalidando assets para: {}", instance.instanceName);
    
        // Verificar si la versión de Minecraft está disponible
        if instance.minecraftVersion.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "No se pudo determinar la versión de Minecraft",
            ));
        }
    
        // Obtener la ruta de la instancia
        let instance_dir = Path::new(instance.instanceDirectory.as_deref().unwrap_or(""));
        let minecraft_folder = instance_dir.join("minecraft");
        let assets_dir = minecraft_folder.join("assets");
        let assets_indexes_dir = assets_dir.join("indexes");
        let assets_objects_dir = assets_dir.join("objects");
    
        // Crear directorios si no existen
        fs::create_dir_all(&assets_indexes_dir)?;
        fs::create_dir_all(&assets_objects_dir)?;
    
        // Obtener detalles de la versión
        let version_details = self.get_version_details(&instance.minecraftVersion)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Error al obtener detalles de versión: {}", e)))?;
    
        // Obtener información del índice de assets
        let asset_index_node = version_details.get("assetIndex")
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "No se encontró información del índice de assets"))?;
    
        let assets_index_id = asset_index_node.get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "ID de índice de assets inválido"))?;
    
        let assets_index_url = asset_index_node.get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "URL de índice de assets inválido"))?;
    
        let assets_index_file = assets_indexes_dir.join(format!("{}.json", assets_index_id));
    
        // Descargar o validar el índice de assets
        if !assets_index_file.exists() {
            println!("Descargando índice de assets para la versión {}", instance.minecraftVersion);
            self.download_file(assets_index_url, &assets_index_file)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Error al descargar índice de assets: {}", e)))?;
        }
    
        // Leer y procesar el índice de assets
        let assets_index_content = fs::read_to_string(&assets_index_file)?;
        let assets_index_root: Value = serde_json::from_str(&assets_index_content)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("Error al parsear índice de assets: {}", e)))?;
    
        let objects = assets_index_root.get("objects")
            .and_then(|v| v.as_object())
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "No se encontraron objetos de assets en el índice"))?;
    
        let total_assets = objects.len();
        let mut processed_assets = 0;
        let mut missing_assets = 0;
    
        println!("Validando {} assets...", total_assets);
    
        // Procesar cada asset
        for (asset_name, asset_info) in objects {
            processed_assets += 1;
    
            let hash = asset_info.get("hash")
                .and_then(|v| v.as_str())
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, format!("Hash inválido para asset: {}", asset_name)))?;
    
            let hash_prefix = &hash[0..2];
            let asset_file = assets_objects_dir.join(hash_prefix).join(hash);
    
            // Informar progreso
            
                println!(
                    "Validando assets: {}/{} ({:.1}%)",
                    processed_assets,
                    total_assets,
                    (processed_assets as f64 * 100.0 / total_assets as f64)
                );
                Self::emit_status(
                    instance,
                    "instance-downloading-assets",
                    &format!("Validando assets: {}/{} ({:.1}%)", processed_assets, total_assets, (processed_assets as f64 * 100.0 / total_assets as f64)),
                );
            
    
            if !asset_file.exists() {
                missing_assets += 1;
                let asset_url = format!("https://resources.download.minecraft.net/{}/{}", hash_prefix, hash);
                let target_dir = assets_objects_dir.join(hash_prefix);
                
                if !target_dir.exists() {
                    fs::create_dir_all(&target_dir)?;
                }
                
                self.download_file(&asset_url, &asset_file)
                    .map_err(|e| io::Error::new(
                        io::ErrorKind::Other,
                        format!("Error al descargar asset {}: {}", asset_name, e)
                    ))?;
            }
        }
    
        if missing_assets > 0 {
            println!("Se han descargado {} assets faltantes.", missing_assets);
        } else {
            println!("Todos los assets están validados.");
        }
    
        println!("Asset revalidation completed");
        Ok(())
    }
    
    // Método para obtener detalles de la versión
    fn get_version_details(&mut self, version: &str) -> Result<Value, String> {
        // Obtener el manifiesto de versiones
        let version_manifest = self
            .get_version_manifest()
            .map_err(|e| format!("Error fetching version manifest: {}", e))?;
    
        let versions_node = version_manifest["versions"]
            .as_array()
            .ok_or_else(|| "Invalid version manifest format".to_string())?;
    
        // Buscar la versión específica
        let version_info = versions_node.iter()
            .find(|v| v["id"].as_str() == Some(version))
            .ok_or_else(|| format!("Version {} not found in manifest", version))?;
    
        let version_url = version_info["url"]
            .as_str()
            .ok_or_else(|| "Invalid version info format".to_string())?;
    
        // Descargar detalles de la versión
        self.client.get(version_url)
            .send()
            .map_err(|e| format!("Error fetching version details: {}", e))?
            .json::<Value>()
            .map_err(|e| format!("Error parsing version details: {}", e))
    }
    
    // Método para descargar archivos
    fn download_file(&self, url: &str, destination: &Path) -> Result<(), String> {
        // Asegurarse de que el directorio padre existe
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Error creating directory: {}", e))?;
        }
    
        let mut response = self.client.get(url)
            .send()
            .map_err(|e| format!("Download error: {}", e))?;
    
        if !response.status().is_success() {
            return Err(format!("Download failed with status: {}", response.status()));
        }
    
        let mut file = fs::File::create(destination)
            .map_err(|e| format!("Error creating file: {}", e))?;
    
        response.copy_to(&mut file)
            .map_err(|e| format!("Error writing file: {}", e))?;
    
        Ok(())
    }

    // Implementaciones auxiliares
    fn get_version_manifest(&mut self) -> Result<Value, reqwest::Error> {
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        // Verificar caché
        if let Some((cached_manifest, cache_time)) = &self.version_manifest_cache {
            if current_time - cache_time < Self::CACHE_EXPIRY_MS {
                return Ok(cached_manifest.clone());
            }
        }

        // Obtener nuevo manifiesto
        let manifest = self
            .client
            .get(Self::MOJANG_VERSION_MANIFEST_URL)
            .send()?
            .json::<Value>()?;

        // Actualizar caché
        self.version_manifest_cache = Some((manifest.clone(), current_time));

        Ok(manifest)
    }

    // Aquí irían más métodos para bootstrapping de instancias Vanilla y Forge
    // como bootstrap_vanilla_instance y bootstrap_forge_instance,
    // pero son bastante extensos para este contexto
}
