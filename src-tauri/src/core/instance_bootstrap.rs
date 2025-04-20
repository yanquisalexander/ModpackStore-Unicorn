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
use crate::core::tasks_manager::{TaskStatus, TasksManager};
use std::sync::{Arc, Mutex};

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

    pub fn bootstrap_vanilla_instance(
        &mut self, 
        instance: &MinecraftInstance, 
        task_id: Option<String>,
        task_manager: Option<Arc<Mutex<TasksManager>>>
    ) -> Result<(), String> {
        // Emit start event
        Self::emit_status(instance, "instance-bootstrap-start", "Iniciando bootstrap de instancia Vanilla");
        
        // Update task status if task_id exists
        if let (Some(task_id), Some(task_manager)) = (&task_id, &task_manager) {
            if let Ok(mut tm) = task_manager.lock() {
                tm.update_task(
                    task_id,
                    TaskStatus::Running,
                    5.0,
                    "Iniciando bootstrap de instancia Vanilla",
                    Some(serde_json::json!({
                        "instanceName": instance.instanceName.clone(),
                        "instanceId": instance.instanceId.clone()
                    }))
                );
            }
        }
        
        // Create minecraft directory if it doesn't exist
        let instance_dir = Path::new(instance.instanceDirectory.as_deref().unwrap_or(""));
        let minecraft_dir = instance_dir.join("minecraft");
        
        if !minecraft_dir.exists() {
            fs::create_dir_all(&minecraft_dir)
                .map_err(|e| format!("Error creating minecraft directory: {}", e))?;
        }
    
        // Create required subdirectories
        let versions_dir = minecraft_dir.join("versions");
        let libraries_dir = minecraft_dir.join("libraries");
        let assets_dir = minecraft_dir.join("assets");
        let version_dir = versions_dir.join(&instance.minecraftVersion);
        let natives_dir = minecraft_dir.join("natives").join(&instance.minecraftVersion);
        
        for dir in [&versions_dir, &libraries_dir, &assets_dir, &version_dir, &natives_dir] {
            if !dir.exists() {
                fs::create_dir_all(dir)
                    .map_err(|e| format!("Error creating directory {}: {}", dir.display(), e))?;
            }
        }
        
        // Update task status - 15%
        if let (Some(task_id), Some(task_manager)) = (&task_id, &task_manager) {
            if let Ok(mut tm) = task_manager.lock() {
                tm.update_task(
                    task_id,
                    TaskStatus::Running,
                    15.0,
                    "Descargando manifiesto de versión",
                    Some(serde_json::json!({
                        "instanceName": instance.instanceName.clone(),
                        "instanceId": instance.instanceId.clone()
                    }))
                );
            }
        }
        
        // Get version details
        Self::emit_status(instance, "instance-downloading-manifest", "Descargando manifiesto de versión");
        let version_details = self.get_version_details(&instance.minecraftVersion)
            .map_err(|e| format!("Error fetching version details: {}", e))?;
        
        // Download version JSON
        let version_json_path = version_dir.join(format!("{}.json", instance.minecraftVersion));
        if !version_json_path.exists() {
            let version_manifest = self.get_version_manifest()
                .map_err(|e| format!("Error fetching version manifest: {}", e))?;
            
            let versions = version_manifest["versions"].as_array()
                .ok_or_else(|| "Invalid version manifest format".to_string())?;
            
            let version_info = versions.iter()
                .find(|v| v["id"].as_str() == Some(&instance.minecraftVersion))
                .ok_or_else(|| format!("Version {} not found in manifest", instance.minecraftVersion))?;
            
            let version_url = version_info["url"].as_str()
                .ok_or_else(|| "Invalid version info format".to_string())?;
            
            // Update task status - 25%
            if let (Some(task_id), Some(task_manager)) = (&task_id, &task_manager) {
                if let Ok(mut tm) = task_manager.lock() {
                    tm.update_task(
                        task_id,
                        TaskStatus::Running,
                        25.0,
                        &format!("Descargando JSON de versión: {}", instance.minecraftVersion),
                        Some(serde_json::json!({
                            "instanceName": instance.instanceName.clone(),
                            "instanceId": instance.instanceId.clone()
                        }))
                    );
                }
            }
            
            Self::emit_status(instance, "instance-downloading-json", 
                &format!("Descargando JSON de versión: {}", instance.minecraftVersion));
            
            self.download_file(version_url, &version_json_path)
                .map_err(|e| format!("Error downloading version JSON: {}", e))?;
        }
        
        // Download client jar
        let client_jar_path = version_dir.join(format!("{}.jar", instance.minecraftVersion));
        if !client_jar_path.exists() {
            let client_url = version_details["downloads"]["client"]["url"].as_str()
                .ok_or_else(|| "Client download URL not found".to_string())?;
            
            // Update task status - 35%
            if let (Some(task_id), Some(task_manager)) = (&task_id, &task_manager) {
                if let Ok(mut tm) = task_manager.lock() {
                    tm.update_task(
                        task_id,
                        TaskStatus::Running,
                        35.0,
                        &format!("Descargando cliente: {}", instance.minecraftVersion),
                        Some(serde_json::json!({
                            "instanceName": instance.instanceName.clone(),
                            "instanceId": instance.instanceId.clone()
                        }))
                    );
                }
            }
            
            Self::emit_status(instance, "instance-downloading-client", 
                &format!("Descargando cliente: {}", instance.minecraftVersion));
            
            self.download_file(client_url, &client_jar_path)
                .map_err(|e| format!("Error downloading client jar: {}", e))?;
        }
        
        // Update task status - 45%
        if let (Some(task_id), Some(task_manager)) = (&task_id, &task_manager) {
            if let Ok(mut tm) = task_manager.lock() {
                tm.update_task(
                    task_id,
                    TaskStatus::Running,
                    45.0,
                    "Descargando librerías",
                    Some(serde_json::json!({
                        "instanceName": instance.instanceName.clone(),
                        "instanceId": instance.instanceId.clone()
                    }))
                );
            }
        }
        
        // Download and validate libraries
        Self::emit_status(instance, "instance-downloading-libraries", "Descargando librerías");
        self.download_libraries(&version_details, &libraries_dir, instance)
            .map_err(|e| format!("Error downloading libraries: {}", e))?;
        
        // Update task status - 60%
        if let (Some(task_id), Some(task_manager)) = (&task_id, &task_manager) {
            if let Ok(mut tm) = task_manager.lock() {
                tm.update_task(
                    task_id,
                    TaskStatus::Running,
                    60.0,
                    "Validando assets",
                    Some(serde_json::json!({
                        "instanceName": instance.instanceName.clone(),
                        "instanceId": instance.instanceId.clone()
                    }))
                );
            }
        }
        
        // Validate assets
        Self::emit_status(instance, "instance-downloading-assets", "Validando assets");
        self.revalidate_assets(instance)
            .map_err(|e| format!("Error validating assets: {}", e))?;
        
        // Create launcher profiles.json if it doesn't exist
        let launcher_profiles_path = minecraft_dir.join("launcher_profiles.json");
        if !launcher_profiles_path.exists() {
            let default_profiles = json!({
                "profiles": {},
                "settings": {},
                "version": 3
            });
            
            fs::write(&launcher_profiles_path, default_profiles.to_string())
                .map_err(|e| format!("Error creating launcher_profiles.json: {}", e))?;
        }
        
        // No emitimos el 100% aquí porque también usamos este método para
        // crear instancias de Forge, y no queremos que se emita el evento
        // de finalización, así que lo hará la función que llame al proceso
        // de bootstrap.
        
        
        Self::emit_status(instance, "vanilla-instance-bootstrapped",
            &format!("Bootstrap de instancia Vanilla {} completado", instance.minecraftVersion));
        
        Ok(())
    }

    fn download_libraries(&self, version_details: &Value, libraries_dir: &Path, instance: &MinecraftInstance) -> Result<(), String> {
        let libraries = version_details["libraries"].as_array()
            .ok_or_else(|| "Libraries list not found in version details".to_string())?;
        
        let total_libraries = libraries.len();
        let mut downloaded_libraries = 0;
        
        for library in libraries {
            // Check if we should skip this library based on rules
            if let Some(rules) = library.get("rules") {
                let mut allowed = false;
                
                for rule in rules.as_array().unwrap_or(&Vec::new()) {
                    let action = rule["action"].as_str().unwrap_or("disallow");
                    
                    // Handle OS-specific rules
                    if let Some(os) = rule.get("os") {
                        let os_name = os["name"].as_str().unwrap_or("");
                        let current_os = if cfg!(target_os = "windows") {
                            "windows"
                        } else if cfg!(target_os = "macos") {
                            "osx"
                        } else {
                            "linux"
                        };
                        
                        if os_name == current_os {
                            allowed = action == "allow";
                        }
                    } else {
                        // No OS specified, apply to all
                        allowed = action == "allow";
                    }
                }
                
                if !allowed {
                    continue; // Skip this library
                }
            }
            
            // Get library info
            let downloads = library.get("downloads")
                .ok_or_else(|| "Library downloads info not found".to_string())?;
            
            // Handle artifact
            if let Some(artifact) = downloads.get("artifact") {
                let path = artifact["path"].as_str()
                    .ok_or_else(|| "Library artifact path not found".to_string())?;
                let url = artifact["url"].as_str()
                    .ok_or_else(|| "Library artifact URL not found".to_string())?;
                
                let target_path = libraries_dir.join(path);
                
                // Create parent directories if needed
                if let Some(parent) = target_path.parent() {
                    fs::create_dir_all(parent)
                        .map_err(|e| format!("Error creating directory: {}", e))?;
                }
                
                // Download if file doesn't exist
                if !target_path.exists() {
                    self.download_file(url, &target_path)
                        .map_err(|e| format!("Error downloading library: {}", e))?;
                }
            }
            
            // Handle native libraries (classifiers)
            if let Some(classifiers) = downloads.get("classifiers") {
                let current_os = if cfg!(target_os = "windows") {
                    "natives-windows"
                } else if cfg!(target_os = "macos") {
                    "natives-osx"
                } else {
                    "natives-linux"
                };
                
                if let Some(native) = classifiers.get(current_os) {
                    let url = native["url"].as_str()
                        .ok_or_else(|| "Native library URL not found".to_string())?;
                    let path = native["path"].as_str()
                        .ok_or_else(|| "Native library path not found".to_string())?;
                    
                    let target_path = libraries_dir.join(path);
                    
                    // Create parent directories if needed
                    if let Some(parent) = target_path.parent() {
                        fs::create_dir_all(parent)
                            .map_err(|e| format!("Error creating directory: {}", e))?;
                    }
                    
                    // Download if file doesn't exist
                    if !target_path.exists() {
                        self.download_file(url, &target_path)
                            .map_err(|e| format!("Error downloading native library: {}", e))?;
                    }
                }
            }
            
            downloaded_libraries += 1;
            
            // Update progress every 5 libraries or on last library
            if downloaded_libraries % 5 == 0 || downloaded_libraries == total_libraries {
                let progress = (downloaded_libraries as f32 / total_libraries as f32) * 100.0;
                Self::emit_status(
                    instance,
                    "instance-downloading-libraries",
                    &format!("Descargando librerías: {}/{} ({:.1}%)", downloaded_libraries, total_libraries, progress)
                );
            }
        }
        
        Ok(())
    }
    
    pub fn bootstrap_forge_instance(
        &mut self,
        instance: &MinecraftInstance,
        task_id: Option<String>,
        task_manager: Option<Arc<Mutex<TasksManager>>>
    ) -> Result<(), String> {
        // Verificar que tengamos información de Forge
        if instance.forgeVersion.is_none() || instance.forgeVersion.as_ref().unwrap().is_empty() {
            return Err("No se especificó versión de Forge".to_string());
        }

        // Emit start event
        Self::emit_status(instance, "instance-bootstrap-start", "Iniciando bootstrap de instancia Forge");
        
        // Update task status if task_id exists
        if let (Some(task_id), Some(task_manager)) = (&task_id, &task_manager) {
            if let Ok(mut tm) = task_manager.lock() {
                tm.update_task(
                    task_id,
                    TaskStatus::Running,
                    5.0,
                    "Iniciando bootstrap de instancia Forge",
                    Some(serde_json::json!({
                        "instanceName": instance.instanceName.clone(),
                        "instanceId": instance.instanceId.clone()
                    }))
                );
            }
        }

        // Primero, realizar bootstrap de la instancia Vanilla
        Self::emit_status(instance, "instance-forge-vanilla-setup", "Configurando base Vanilla");
        
        // Update task status - 10%
        if let (Some(task_id), Some(task_manager)) = (&task_id, &task_manager) {
            if let Ok(mut tm) = task_manager.lock() {
                tm.update_task(
                    task_id,
                    TaskStatus::Running,
                    10.0,
                    "Configurando base Vanilla",
                    Some(serde_json::json!({
                        "instanceName": instance.instanceName.clone(),
                        "instanceId": instance.instanceId.clone()
                    }))
                );
            }
        }
        
        // Bootstrap Vanilla primero
        self.bootstrap_vanilla_instance(instance, None, None)
            .map_err(|e| format!("Error en bootstrap Vanilla: {}", e))?;
        
        // Update task status - 60%
        if let (Some(task_id), Some(task_manager)) = (&task_id, &task_manager) {
            if let Ok(mut tm) = task_manager.lock() {
                tm.update_task(
                    task_id,
                    TaskStatus::Running,
                    60.0,
                    "Configurando Forge",
                    Some(serde_json::json!({
                        "instanceName": instance.instanceName.clone(),
                        "instanceId": instance.instanceId.clone()
                    }))
                );
            }
        }
        
        // Obtener rutas de directorios
        let instance_dir = Path::new(instance.instanceDirectory.as_deref().unwrap_or(""));
        let minecraft_dir = instance_dir.join("minecraft");
        let versions_dir = minecraft_dir.join("versions");
        let libraries_dir = minecraft_dir.join("libraries");
        
        // Obtener detalles de Forge
        let forge_version = instance.forgeVersion.as_ref().unwrap();
        
        Self::emit_status(
            instance, 
            "instance-downloading-forge", 
            &format!("Descargando Forge {} para Minecraft {}", forge_version, instance.minecraftVersion)
        );
        
        // Crear directorio específico para la versión de Forge
        let forge_version_name = format!("{}-forge-{}", instance.minecraftVersion, forge_version);
        let forge_version_dir = versions_dir.join(&forge_version_name);
        
        if !forge_version_dir.exists() {
            fs::create_dir_all(&forge_version_dir)
                .map_err(|e| format!("Error al crear directorio de versión Forge: {}", e))?;
        }
        
        // Obtener URL de instalador Forge
        let forge_installer_url = self.get_forge_installer_url(&instance.minecraftVersion, forge_version)?;
        
        // Path para el instalador
        let forge_installer_path = minecraft_dir.join("forge-installer.jar");
        
        // Descargar instalador Forge
        Self::emit_status(instance, "instance-downloading-forge-installer", "Descargando instalador de Forge");
        self.download_file(&forge_installer_url, &forge_installer_path)
            .map_err(|e| format!("Error al descargar instalador Forge: {}", e))?;
        
        // Update task status - 70%
        if let (Some(task_id), Some(task_manager)) = (&task_id, &task_manager) {
            if let Ok(mut tm) = task_manager.lock() {
                tm.update_task(
                    task_id,
                    TaskStatus::Running,
                    70.0,
                    "Ejecutando instalador de Forge",
                    Some(serde_json::json!({
                        "instanceName": instance.instanceName.clone(),
                        "instanceId": instance.instanceId.clone()
                    }))
                );
            }
        }
        
        // Ejecutar instalador en modo silencioso
        Self::emit_status(instance, "instance-installing-forge", "Ejecutando instalador de Forge");
        
        // Preparar argumentos para instalar Forge
        let forge_install_result = self.run_forge_installer(
            &forge_installer_path,
            &minecraft_dir,
            &instance.minecraftVersion,
            forge_version,
            instance
        )?;
        
        // Update task status - 85%
        if let (Some(task_id), Some(task_manager)) = (&task_id, &task_manager) {
            if let Ok(mut tm) = task_manager.lock() {
                tm.update_task(
                    task_id,
                    TaskStatus::Running,
                    85.0,
                    "Configurando perfil de Forge",
                    Some(serde_json::json!({
                        "instanceName": instance.instanceName.clone(),
                        "instanceId": instance.instanceId.clone()
                    }))
                );
            }
        }
        
        // Crear/actualizar perfil de Forge en launcher_profiles.json
        let launcher_profiles_path = minecraft_dir.join("launcher_profiles.json");
        self.update_launcher_profiles(&launcher_profiles_path, &forge_version_name, &instance.instanceName)?;
        
        // Limpiar instalador Forge para ahorrar espacio
        if forge_installer_path.exists() {
            if let Err(e) = fs::remove_file(forge_installer_path) {
                println!("Advertencia: No se pudo borrar el instalador de Forge: {}", e);
            }
        }
        
        // Update task status - 100%
        if let (Some(task_id), Some(task_manager)) = (&task_id, &task_manager) {
            if let Ok(mut tm) = task_manager.lock() {
                tm.update_task(
                    task_id,
                    TaskStatus::Completed,
                    100.0,
                    &format!("Instalación completada: Forge {} para Minecraft {}", forge_version, instance.minecraftVersion),
                    Some(serde_json::json!({
                        "instanceName": instance.instanceName.clone(),
                        "instanceId": instance.instanceId.clone()
                    }))
                );
            }
        }
        
        Self::emit_status(
            instance, 
            "forge-instance-bootstrapped", 
            &format!("Bootstrap de instancia Forge {} para Minecraft {} completado", forge_version, instance.minecraftVersion)
        );
        
        Ok(())
    }
    
    fn get_forge_installer_url(&self, minecraft_version: &str, forge_version: &str) -> Result<String, String> {
        // Formato de URL de Forge moderno para versiones actuales
        let url_format = format!(
            "https://maven.minecraftforge.net/net/minecraftforge/forge/{}-{}/forge-{}-{}-installer.jar",
            minecraft_version, forge_version, minecraft_version, forge_version
        );
        
        // Verificar si la URL responde correctamente
        match self.client.head(&url_format).send() {
            Ok(response) => {
                if response.status().is_success() {
                    return Ok(url_format);
                }
                
                // Probar formato alternativo para versiones antiguas
                let legacy_url = format!(
                    "https://maven.minecraftforge.net/net/minecraftforge/forge/{}.{}/forge-{}.{}-installer.jar",
                    minecraft_version, forge_version, minecraft_version, forge_version
                );
                
                if self.client.head(&legacy_url).send().map_or(false, |r| r.status().is_success()) {
                    return Ok(legacy_url);
                }
                
                Err(format!("No se encontró URL de instalador válida para Forge {} - {}", minecraft_version, forge_version))
            },
            Err(e) => Err(format!("Error al verificar URL de Forge: {}", e))
        }
    }
    
    fn run_forge_installer(
        &self,
        installer_path: &Path,
        minecraft_dir: &Path,
        minecraft_version: &str,
        forge_version: &str,
        instance: &MinecraftInstance
    ) -> Result<(), String> {
        // Determinar la ruta de Java
        let java_path = self.find_java_path()?;
        
        // Crear archivo temporal para parámetros de instalación
        let install_profile = minecraft_dir.join("forge-install-profile.json");
        let install_profile_content = json!({
            "profile": format!("forge-{}-{}", minecraft_version, forge_version),
            "version": format!("{}-forge-{}", minecraft_version, forge_version),
            "installDir": minecraft_dir.to_string_lossy(),
            "minecraft": minecraft_version,
            "forge": forge_version
        });
        
        fs::write(&install_profile, install_profile_content.to_string())
            .map_err(|e| format!("Error al crear archivo de perfil de instalación: {}", e))?;
        
        // Preparar comando para ejecutar el instalador
        let mut install_cmd = Command::new(java_path);
        install_cmd
            .arg("-jar")
            .arg(installer_path)
            .arg("--installClient")
            .current_dir(minecraft_dir);
        
        // Ejecutar instalador
        println!("Ejecutando instalador Forge con comando: {:?}", install_cmd);
        
        let output = install_cmd
            .output()
            .map_err(|e| format!("Error al ejecutar instalador de Forge: {}", e))?;
        
        // Verificar resultado
        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Error en instalación de Forge: {}", error_msg));
        }
        
        // Limpiar archivo temporal de instalación
        if install_profile.exists() {
            let _ = fs::remove_file(install_profile);
        }
        
        println!("Instalación de Forge completada con éxito");
        Ok(())
    }

    fn find_java_path(&self) -> Result<String, String> {
        let config_manager = crate::utils::config_manager::get_config_manager();
        let java_path = config_manager
            .lock() // First lock the mutex to get the inner value
            .expect("Failed to lock config manager mutex") // Handle potential lock failure
            .get_java_dir() // Now call the method on the inner value
            .join("bin")
            .join(if cfg!(windows) { "java.exe" } else { "java" });

        if java_path.exists() {
            Ok(java_path.to_string_lossy().to_string())
        } else {
            Err(format!("Java executable not found at path: {}", java_path.display()))
        }
    }


    fn update_launcher_profiles(&self, profiles_path: &Path, version_id: &str, instance_name: &str) -> Result<(), String> {
        // Leer archivo de perfiles actual
        let profiles_content = match fs::read_to_string(profiles_path) {
            Ok(content) => content,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                // Si no existe, crear uno básico
                "{ \"profiles\": {}, \"settings\": {}, \"version\": 3 }".to_string()
            },
            Err(e) => return Err(format!("Error al leer archivo de perfiles: {}", e)),
        };
        
        // Parsear JSON
        let mut profiles_json: Value = serde_json::from_str(&profiles_content)
            .map_err(|e| format!("Error al parsear archivo de perfiles: {}", e))?;
        
        // Crear o actualizar perfil de Forge
        let profile_id = format!("forge-{}", version_id);
        let profiles = profiles_json["profiles"].as_object_mut()
            .ok_or_else(|| "Formato inválido en archivo de perfiles".to_string())?;
        
        // Generar fecha actual en formato ISO
        let now = chrono::Utc::now();
        let date_str = now.format("%Y-%m-%dT%H:%M:%S.%3fZ").to_string();
        
        // Crear nuevo perfil
        profiles.insert(profile_id.clone(), json!({
            "created": date_str,
            "lastUsed": date_str,
            "lastVersionId": version_id,
            "name": format!("{} (Forge)", instance_name),
            "type": "custom"
        }));
        
        // Guardar archivo actualizado
        fs::write(profiles_path, serde_json::to_string_pretty(&profiles_json).unwrap())
            .map_err(|e| format!("Error al guardar archivo de perfiles: {}", e))?;
        
        Ok(())
    }
}


