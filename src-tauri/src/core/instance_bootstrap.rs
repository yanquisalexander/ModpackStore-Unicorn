// src-tauri/src/instance_bootstrap.rs
use crate::config::get_config_manager;
use crate::core::instance_manager::get_instance_by_id;
use crate::core::java_manager::JavaManager;
use crate::core::minecraft_instance::MinecraftInstance;
use crate::core::tasks_manager::{TaskStatus, TasksManager};
use crate::GLOBAL_APP_HANDLE;
use serde_json::{json, Value};
use std::fs;
use std::io::{self, Result as IoResult};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use tauri::Emitter;
use tauri_plugin_http::reqwest;

pub struct InstanceBootstrap {
    client: reqwest::blocking::Client,
    // Cache para metadatos de versiones
    version_manifest_cache: Option<(Value, u64)>, // (datos, timestamp)
}

impl InstanceBootstrap {
    const MOJANG_VERSION_MANIFEST_URL: &'static str =
        "https://launchermeta.mojang.com/mc/game/version_manifest.json";
    const FORGE_API_BASE_URL: &'static str = "https://mc-versions-api.net/api/forge";
    const CACHE_EXPIRY_MS: u64 = 3600000; // 1 hora

    pub fn new() -> Self {
        Self {
            client: reqwest::blocking::Client::new(),
            version_manifest_cache: None,
        }
    }

    fn format_bytes(bytes: u64) -> String {
        const KB: u64 = 1024;
        const MB: u64 = KB * 1024;
        const GB: u64 = MB * 1024;

        if bytes >= GB {
            format!("{:.2} GB", bytes as f64 / GB as f64)
        } else if bytes >= MB {
            format!("{:.2} MB", bytes as f64 / MB as f64)
        } else if bytes >= KB {
            format!("{:.2} KB", bytes as f64 / KB as f64)
        } else {
            format!("{} bytes", bytes)
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
                    log::info!("[Bootstrap] Error emitting event '{}': {}", event_name, e);
                }
            } else {
                log::info!(
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

    // Implementación del método extract_natives
    fn extract_natives(
        &self,
        version_details: &Value,
        libraries_dir: &Path,
        natives_dir: &Path,
        instance: &MinecraftInstance,
        task_id: Option<&str>,
        task_manager: Option<&Arc<Mutex<TasksManager>>>,
        base_overall_progress: f32,
        max_progress_span_for_this_step: f32,
    ) -> Result<(), String> {
        // Obtener el sistema operativo actual
        let os = std::env::consts::OS;
        let os_name = match os {
            "windows" => "windows",
            "macos" => "osx",
            "linux" => "linux",
            _ => return Err(format!("Sistema operativo no soportado: {}", os)),
        };

        // Obtener la arquitectura
        let arch = std::env::consts::ARCH;
        let arch_name = match arch {
            "x86_64" => "64",
            "x86" => "32",
            "aarch64" => "arm64",
            _ => return Err(format!("Arquitectura no soportada: {}", arch)),
        };

        // Obtener las bibliotecas del manifiesto de versión
        let libraries = version_details["libraries"]
            .as_array()
            .ok_or_else(|| "No se encontraron bibliotecas en el manifiesto".to_string())?;

        // First, filter libraries that have natives for the current OS & arch to get a total count for progress.
        let relevant_native_libs: Vec<&Value> = libraries
            .iter()
            .filter(|lib| {
                if let Some(natives_map) = lib.get("natives") {
                    natives_map.get(os_name).is_some()
                } else {
                    false
                }
            })
            .collect();

        let total_libs_with_natives = relevant_native_libs.len();
        let mut processed_libs_with_natives = 0;

        for library in relevant_native_libs {
            // Verificar si la biblioteca tiene nativos (already filtered, but good for structure)
            if let Some(natives) = library.get("natives") {
                let os_natives = natives.get(os_name);

                // Si hay nativos para este sistema operativo
                if let Some(os_natives_value) = os_natives {
                    // Obtener información sobre la biblioteca
                    let library_info = library["downloads"]["classifiers"]
                        .get(
                            os_natives_value
                                .as_str()
                                .unwrap_or(&format!("{}-{}", os_name, arch_name)),
                        )
                        .or_else(|| {
                            library["downloads"]["classifiers"]
                                .get(&format!("{}-{}", os_name, arch_name))
                        })
                        .ok_or_else(|| {
                            format!("No se encontró información de nativos para la biblioteca")
                        })?;

                    // Obtener la ruta y URL del archivo JAR
                    let lib_name_for_message = library.get("name").and_then(|n| n.as_str()).unwrap_or("unknown library");
                    let path = library_info["path"]
                        .as_str()
                        .ok_or_else(|| format!("No se encontró la ruta del archivo nativo para {}", lib_name_for_message))?;

                    processed_libs_with_natives += 1;
                    let current_step_progress = if total_libs_with_natives > 0 {
                        (processed_libs_with_natives as f32 / total_libs_with_natives as f32) * max_progress_span_for_this_step
                    } else {
                        0.0
                    };
                    let overall_progress_for_task_update = base_overall_progress + current_step_progress;

                    let extraction_message = format!(
                        "Extrayendo nativos para {}: {}/{}",
                        lib_name_for_message, processed_libs_with_natives, total_libs_with_natives
                    );

                    if let (Some(tid), Some(tm)) = (task_id, task_manager.as_ref()) {
                        if let Ok(mut manager) = tm.lock() {
                            manager.update_task(
                                tid,
                                TaskStatus::Running,
                                overall_progress_for_task_update,
                                &extraction_message,
                                None,
                            );
                        }
                    }
                    Self::emit_status(instance, "instance-extracting-native-library", &extraction_message);


                    let library_path = libraries_dir.join(path);

                    // Native libraries should typically be already downloaded by download_libraries.
                    // If not, it's an error or needs a download here (currently not handled with progress).
                    if !library_path.exists() {
                        log::warn!("Biblioteca nativa {} no encontrada en {}. Es posible que deba descargarse.", lib_name_for_message, library_path.display());
                        // Optionally, could call download_file here, but it would complicate progress for this step
                        // For now, assume it's present from previous steps.
                        // self.download_file(url, &library_path, instance, &format!("Native {}", lib_name_for_message), task_id, task_manager, overall_progress_for_task_update)?;
                        // For simplicity, if it's missing, we might just error out or skip.
                        return Err(format!("Biblioteca nativa requerida {} no encontrada en {}", lib_name_for_message, library_path.display()));
                    }

                    // Verificar si hay reglas de extracción (exclude)
                    let exclude_patterns: Vec<String> =
                        if let Some(extract) = library.get("extract") {
                            if let Some(exclude) = extract.get("exclude") {
                                exclude
                                    .as_array()
                                    .unwrap_or(&Vec::new())
                                    .iter()
                                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                    .collect()
                            } else {
                                Vec::new()
                            }
                        } else {
                            Vec::new()
                        };

                    // Extraer el archivo JAR al directorio de nativos
                    Self::emit_status(
                        instance,
                        "instance-extracting-native-library",
                        &format!("Extrayendo biblioteca nativa: {}", path),
                    );

                    // Abrir el archivo JAR
                    let file = fs::File::open(&library_path)
                        .map_err(|e| format!("Error abriendo archivo JAR: {}", e))?;

                    let reader = std::io::BufReader::new(file);
                    let mut archive = zip::ZipArchive::new(reader)
                        .map_err(|e| format!("Error leyendo archivo ZIP: {}", e))?;

                    // Extraer cada entrada que no esté excluida
                    for i in 0..archive.len() {
                        let mut file = archive
                            .by_index(i)
                            .map_err(|e| format!("Error obteniendo entrada ZIP: {}", e))?;

                        let file_name = file.name().to_string();

                        // Verificar si el archivo está excluido
                        let should_extract = !exclude_patterns.iter().any(|pattern| {
                            if pattern.ends_with("*") {
                                let prefix = &pattern[0..pattern.len() - 1];
                                file_name.starts_with(prefix)
                            } else {
                                file_name == *pattern
                            }
                        });

                        if should_extract && !file.is_dir() {
                            // Crear la ruta de destino
                            let output_path = natives_dir.join(file_name);

                            // Crear directorios padres si no existen
                            if let Some(parent) = output_path.parent() {
                                fs::create_dir_all(parent).map_err(|e| {
                                    format!("Error creando directorio para archivo nativo: {}", e)
                                })?;
                            }

                            // Extraer el archivo
                            let mut output_file = fs::File::create(&output_path)
                                .map_err(|e| format!("Error creando archivo nativo: {}", e))?;

                            std::io::copy(&mut file, &mut output_file)
                                .map_err(|e| format!("Error escribiendo archivo nativo: {}", e))?;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    pub fn revalidate_assets(
        &mut self,
        instance: &MinecraftInstance,
        task_id: Option<&str>,
        task_manager: Option<&Arc<Mutex<TasksManager>>>,
        base_overall_progress: f32,
        max_progress_span_for_this_step: f32
    ) -> IoResult<()> {
        log::info!("Revalidando assets para: {}", instance.instanceName);

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
        let version_details = self
            .get_version_details(&instance.minecraftVersion)
            .map_err(|e| {
                io::Error::new(
                    io::ErrorKind::Other,
                    format!("Error al obtener detalles de versión: {}", e),
                )
            })?;

        // Obtener información del índice de assets
        let asset_index_node = version_details.get("assetIndex").ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "No se encontró información del índice de assets",
            )
        })?;

        let assets_index_id = asset_index_node
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    "ID de índice de assets inválido",
                )
            })?;

        let assets_index_url = asset_index_node
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    "URL de índice de assets inválido",
                )
            })?;

        let assets_index_file = assets_indexes_dir.join(format!("{}.json", assets_index_id));

        // Descargar o validar el índice de assets
        if !assets_index_file.exists() {
            log::info!(
                "Descargando índice de assets para la versión {}",
                instance.minecraftVersion
            );
            // For the asset index download, we can use the base_overall_progress,
            // as it's a small, initial part of this step.
            self.download_file(
                assets_index_url,
                &assets_index_file,
                instance,
                &format!("Asset Index ({})", assets_index_id),
                task_id,
                task_manager.as_ref(),
                base_overall_progress,
            )
            .map_err(|e| {
                io::Error::new(
                    io::ErrorKind::Other,
                    format!("Error al descargar índice de assets: {}", e),
                )
            })?;
        }

        // Leer y procesar el índice de assets
        let assets_index_content = fs::read_to_string(&assets_index_file)?;
        let assets_index_root: Value =
            serde_json::from_str(&assets_index_content).map_err(|e| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Error al parsear índice de assets: {}", e),
                )
            })?;

        let objects = assets_index_root
            .get("objects")
            .and_then(|v| v.as_object())
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    "No se encontraron objetos de assets en el índice",
                )
            })?;

        let total_assets = objects.len();
        let mut processed_assets = 0;
        let mut missing_assets = 0;

        log::info!("Validando {} assets...", total_assets);

        // Procesar cada asset
        for (asset_name, asset_info) in objects {
            processed_assets += 1;

            let hash = asset_info
                .get("hash")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("Hash inválido para asset: {}", asset_name),
                    )
                })?;

            let hash_prefix = &hash[0..2];
            let asset_file = assets_objects_dir.join(hash_prefix).join(hash);

            // Informar progreso

            // Calculate progress for task manager updates
            let current_step_progress = if total_assets > 0 {
                (processed_assets as f32 / total_assets as f32) * max_progress_span_for_this_step
            } else {
                0.0
            };
            let overall_progress_for_task_update = base_overall_progress + current_step_progress;

            let progress_message = format!(
                "Validando assets: {}/{} ({:.1}%)",
                processed_assets,
                total_assets,
                if total_assets > 0 { (processed_assets as f32 * 100.0 / total_assets as f32) } else { 0.0 }
            );

            log::info!("{}", progress_message);
            Self::emit_status(instance, "instance-downloading-assets", &progress_message);

            if let (Some(tid), Some(tm)) = (task_id, task_manager) {
                if let Ok(mut manager) = tm.lock() {
                    manager.update_task(
                        tid,
                        TaskStatus::Running,
                        overall_progress_for_task_update,
                        &progress_message,
                        None,
                    );
                }
            }

            if !asset_file.exists() {
                missing_assets += 1;
                let asset_url = format!(
                    "https://resources.download.minecraft.net/{}/{}",
                    hash_prefix, hash
                );
                let target_dir = assets_objects_dir.join(hash_prefix);

                if !target_dir.exists() {
                    fs::create_dir_all(&target_dir)?;
                }

                self.download_file(
                    &asset_url,
                    &asset_file,
                    instance,
                    asset_name, // Use the asset_name (filename from the index)
                    task_id,
                    task_manager.as_ref(),
                    overall_progress_for_task_update, // Pass the calculated overall progress
                )
                .map_err(|e| {
                    io::Error::new(
                        io::ErrorKind::Other,
                        format!("Error al descargar asset {}: {}", asset_name, e),
                    )
                })?;
            }
        }

        if missing_assets > 0 {
            log::info!("Se han descargado {} assets faltantes.", missing_assets);
        } else {
            log::info!("Todos los assets están validados.");
        }

        log::info!("Asset revalidation completed");

        // Emitir evento de finalización
        Self::emit_status(
            instance,
            "instance-finish-assets-download",
            &format!(
                "Validación de assets completada para {}",
                instance.instanceName
            ),
        );
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
        let version_info = versions_node
            .iter()
            .find(|v| v["id"].as_str() == Some(version))
            .ok_or_else(|| format!("Version {} not found in manifest", version))?;

        let version_url = version_info["url"]
            .as_str()
            .ok_or_else(|| "Invalid version info format".to_string())?;

        // Descargar detalles de la versión
        self.client
            .get(version_url)
            .send()
            .map_err(|e| format!("Error fetching version details: {}", e))?
            .json::<Value>()
            .map_err(|e| format!("Error parsing version details: {}", e))
    }

    // Método para descargar archivos
    fn download_file(
        &self,
        url: &str,
        destination: &Path,
        _instance: &MinecraftInstance, // Kept for context, might be used later for specific instance events
        asset_name_for_message: &str,
        task_id: Option<&str>,
        task_manager: Option<&Arc<Mutex<TasksManager>>>,
        current_overall_progress: f32,
    ) -> Result<(), String> {
        use std::io::{Read, Write}; // Ensure Read and Write are in scope

        // Asegurarse de que el directorio padre existe
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("Error creating directory {}: {}", parent.display(), e))?;
        }

        let mut response = self
            .client
            .get(url)
            .send()
            .map_err(|e| format!("Download error for {}: {}", asset_name_for_message, e))?;

        if !response.status().is_success() {
            return Err(format!(
                "Download failed for {} with status: {}",
                asset_name_for_message,
                response.status()
            ));
        }

        let total_size = response
            .headers()
            .get(reqwest::header::CONTENT_LENGTH)
            .and_then(|ct_len| ct_len.to_str().ok())
            .and_then(|ct_len| ct_len.parse::<u64>().ok())
            .unwrap_or(0);

        let mut file = fs::File::create(destination)
            .map_err(|e| format!("Error creating file {}: {}", destination.display(), e))?;

        let mut downloaded_bytes: u64 = 0;
        let mut buffer = [0; 8192]; // 8KB buffer

        loop {
            let bytes_read = response
                .read(&mut buffer)
                .map_err(|e| format!("Error reading response body for {}: {}", asset_name_for_message, e))?;

            if bytes_read == 0 {
                break; // EOF
            }

            file.write_all(&buffer[..bytes_read])
                .map_err(|e| format!("Error writing to file {} for {}: {}", destination.display(), asset_name_for_message, e))?;

            downloaded_bytes += bytes_read as u64;

            if let (Some(tid), Some(tm)) = (task_id, task_manager) {
                let percentage = if total_size > 0 {
                    (downloaded_bytes as f64 * 100.0 / total_size as f64) as f32
                } else {
                    0.0 // Indeterminate if total_size is 0
                };
                let message = if total_size > 0 {
                    format!(
                        "Descargando {}: {} / {} ({:.1}%)",
                        asset_name_for_message,
                        Self::format_bytes(downloaded_bytes),
                        Self::format_bytes(total_size),
                        percentage
                    )
                } else {
                    format!(
                        "Descargando {}: {} (tamaño desconocido)",
                        asset_name_for_message,
                        Self::format_bytes(downloaded_bytes)
                    )
                };
                if let Ok(mut manager) = tm.lock() {
                    manager.update_task(
                        tid,
                        TaskStatus::Running,
                        current_overall_progress, // Keep overall progress, only message changes here
                        &message,
                        None,
                    );
                }
            }
        }

        if let (Some(tid), Some(tm)) = (task_id, task_manager) {
            let message = format!("Descarga completada: {}", asset_name_for_message);
            if let Ok(mut manager) = tm.lock() {
                manager.update_task(
                    tid,
                    TaskStatus::Running, // Still running as part of a larger task
                    current_overall_progress,
                    &message,
                    None,
                );
            }
        }
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
        task_id: Option<&str>, // Changed from Option<String>
        task_manager: Option<&Arc<Mutex<TasksManager>>>,
        overall_task_base_progress: f32, // Base progress for this whole operation
        overall_task_max_span: f32,      // Max percentage this operation will span
    ) -> Result<(), String> {
        // --- Define relative progress points for vanilla bootstrap ---
        // These are percentages *within* the span allocated to vanilla bootstrap.
        let p_start = 0.05; // Initial step
        let p_manifest_download = 0.15;
        let p_version_json_download = 0.25;
        let p_client_jar_download_end = 0.35; // End of client jar download itself
        // Gap between 0.35 and 0.45 (or 0.50 if Java install) is for Java check/install
        let p_java_install_check_end = 0.50; // If Java install happens, it goes up to this
        let p_libraries_download_start = 0.45; // Start of library downloads (base for that step)
        let p_libraries_download_span = 0.15;  // Libraries take 15% of vanilla's span
        let p_assets_validation_start = p_libraries_download_start + p_libraries_download_span; // 0.60
        let p_assets_validation_span = 0.15;   // Assets take 15% of vanilla's span
        let p_natives_extraction_start = p_assets_validation_start + p_assets_validation_span; // 0.75
        let p_natives_extraction_span = 0.10;  // Natives take 10% of vanilla's span
        let p_finalizing_setup = 0.90;         // Finalizing setup

        // Helper to calculate actual progress value
        let calc_progress = |step_percentage: f32| -> f32 {
            overall_task_base_progress + step_percentage * overall_task_max_span
        };

        // Emit start event
        Self::emit_status(
            instance,
            "instance-bootstrap-start",
            "Iniciando bootstrap de instancia Vanilla",
        );

        // Update task status if task_id exists
        if let (Some(tid), Some(tm)) = (task_id, task_manager) {
            if let Ok(mut manager) = tm.lock() {
                manager.update_task(
                    tid,
                    TaskStatus::Running,
                    calc_progress(p_start),
                    "Iniciando bootstrap de instancia Vanilla",
                    Some(serde_json::json!({
                        "instanceName": instance.instanceName.clone(),
                        "instanceId": instance.instanceId.clone()
                    })),
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
        let natives_dir = minecraft_dir
            .join("natives")
            .join(&instance.minecraftVersion);

        for dir in [
            &versions_dir,
            &libraries_dir,
            &assets_dir,
            &version_dir,
            &natives_dir,
        ] {
            if !dir.exists() {
                fs::create_dir_all(dir)
                    .map_err(|e| format!("Error creating directory {}: {}", dir.display(), e))?;
            }
        }

        // Update task status - manifest download
        if let (Some(tid), Some(tm)) = (task_id, task_manager) {
            if let Ok(mut manager) = tm.lock() {
                manager.update_task(
                    tid,
                    TaskStatus::Running,
                    calc_progress(p_manifest_download),
                    "Descargando manifiesto de versión",
                    Some(serde_json::json!({
                        "instanceName": instance.instanceName.clone(),
                        "instanceId": instance.instanceId.clone()
                    })),
                );
            }
        }

        // Get version details
        Self::emit_status(
            instance,
            "instance-downloading-manifest",
            "Descargando manifiesto de versión",
        );
        let version_details = self
            .get_version_details(&instance.minecraftVersion)
            .map_err(|e| format!("Error fetching version details: {}", e))?;

        // Download version JSON
        let version_json_path = version_dir.join(format!("{}.json", instance.minecraftVersion));
        if !version_json_path.exists() {
            let version_manifest = self
                .get_version_manifest()
                .map_err(|e| format!("Error fetching version manifest: {}", e))?;

            let versions = version_manifest["versions"]
                .as_array()
                .ok_or_else(|| "Invalid version manifest format".to_string())?;

            let version_info = versions
                .iter()
                .find(|v| v["id"].as_str() == Some(&instance.minecraftVersion))
                .ok_or_else(|| {
                    format!(
                        "Version {} not found in manifest",
                        instance.minecraftVersion
                    )
                })?;

            let version_url = version_info["url"]
                .as_str()
                .ok_or_else(|| "Invalid version info format".to_string())?;

            let version_json_dl_progress = calc_progress(p_version_json_download);
            if let (Some(tid), Some(tm)) = (task_id, task_manager) {
                if let Ok(mut manager) = tm.lock() {
                    manager.update_task(
                        tid,
                        TaskStatus::Running,
                        version_json_dl_progress,
                        &format!("Descargando JSON de versión: {}", instance.minecraftVersion),
                        Some(serde_json::json!({
                            "instanceName": instance.instanceName.clone(),
                            "instanceId": instance.instanceId.clone()
                        })),
                    );
                }
            }

            Self::emit_status(
                instance,
                "instance-downloading-json",
                &format!("Descargando JSON de versión: {}", instance.minecraftVersion),
            );

            self.download_file(
                version_url,
                &version_json_path,
                instance,
                &format!("Version JSON ({})", instance.minecraftVersion),
                task_id,
                task_manager,
                version_json_dl_progress, // This is the base for this specific download file, it won't change overall progress further
            )
            .map_err(|e| format!("Error downloading version JSON: {}", e))?;
        }

        // Download client jar
        let client_jar_path = version_dir.join(format!("{}.jar", instance.minecraftVersion));
        let client_jar_dl_progress = calc_progress(p_client_jar_download_end);
        if !client_jar_path.exists() {
            let client_url = version_details["downloads"]["client"]["url"]
                .as_str()
                .ok_or_else(|| "Client download URL not found".to_string())?;

            if let (Some(tid), Some(tm)) = (task_id, task_manager) {
                if let Ok(mut manager) = tm.lock() {
                    manager.update_task(
                        tid,
                        TaskStatus::Running,
                        client_jar_dl_progress,
                        &format!("Descargando cliente: {}", instance.minecraftVersion),
                        Some(serde_json::json!({
                            "instanceName": instance.instanceName.clone(),
                            "instanceId": instance.instanceId.clone()
                        })),
                    );
                }
            }

            Self::emit_status(
                instance,
                "instance-downloading-client",
                &format!("Descargando cliente: {}", instance.minecraftVersion),
            );

            self.download_file(
                client_url,
                &client_jar_path,
                instance,
                &format!("Client JAR ({})", instance.minecraftVersion),
                task_id,
                task_manager,
                client_jar_dl_progress,
            )
            .map_err(|e| format!("Error downloading client jar: {}", e))?;
        }

        // --- Java Installation Check ---
        // This step might take some time if Java needs to be downloaded.
        // The progress for this is p_java_install_check_end

        // (Original Java check logic here - progress updated inside if !is_version_installed)

        // Update task status before library download
        let libraries_actual_base_progress = calc_progress(p_libraries_download_start);
        if let (Some(tid), Some(tm)) = (task_id, task_manager) {
            if let Ok(mut manager) = tm.lock() {
                manager.update_task(
                    tid,
                    TaskStatus::Running,
                    libraries_actual_base_progress,
                    "Descargando librerías",
                    Some(serde_json::json!({
                        "instanceName": instance.instanceName.clone(),
                        "instanceId": instance.instanceId.clone()
                    })),
                );
            }
        }

        /*
        "javaVersion": {"majorVersion": 21},
        */
        // Check if correct Java version is installed for this instance
        let java_version = version_details["javaVersion"]
            .as_object()
            .ok_or_else(|| "Java version not found in version details".to_string())?;

        println!("");
        println!("");
        println!("");

        println!("Java Version Details: {:?}", java_version);
        println!("");
        println!("");
        println!("");

        // As string
        let java_major_version = java_version
            .get("majorVersion")
            .and_then(|v| v.as_u64()) // Lo tomás como número primero
            .map(|v| v.to_string()) // Luego lo convertís a String
            .ok_or_else(|| "8".to_string())?; // Valor por defecto si falla

        println!("Java Major Version: {}", java_major_version);

        let java_manager =
            JavaManager::new().map_err(|e| format!("Failed to create JavaManager: {}", e))?; // Convert error to String

        let is_version_installed = java_manager.is_version_installed(&java_major_version);

        if !is_version_installed {
            // Create Tokio runtime for async task execution
            let java_path = tokio::runtime::Runtime::new()
                .expect("Failed to create Tokio runtime")
                .block_on(java_manager.get_java_path(&java_major_version))
                .map_err(|e| {
                    format!(
                        "Error obtaining Java path for version {}: {}",
                        java_major_version, e
                    )
                })?;

            // Update task to indicate Java installation
            if let (Some(tid), Some(tm)) = (task_id, task_manager) {
                if let Ok(mut manager) = tm.lock() {
                    manager.update_task(
                        tid,
                        TaskStatus::Running,
                        calc_progress(p_java_install_check_end), // Progress if Java install happens
                        "Instalando Java",
                        Some(serde_json::json!({
                            "instanceName": instance.instanceName.clone(),
                            "instanceId": instance.instanceId.clone()
                        })),
                    );
                }
            }

            let mut instance_to_modify = instance.clone();
            instance_to_modify.set_java_path(java_path);
        }

        // Download and validate libraries
        let actual_libraries_base_progress = calc_progress(p_libraries_download_start);
        let actual_libraries_span = p_libraries_download_span * overall_task_max_span;

        Self::emit_status(
            instance,
            "instance-downloading-libraries",
            "Descargando librerías",
        );
        self.download_libraries(
            &version_details,
            &libraries_dir,
            instance,
            task_id,
            task_manager,
            actual_libraries_base_progress,
            actual_libraries_span,
        )
        .map_err(|e| format!("Error downloading libraries: {}", e))?;

        // Update task status: After libraries, before assets.
        let actual_assets_base_progress = calc_progress(p_assets_validation_start);
        let actual_assets_span = p_assets_validation_span * overall_task_max_span;

        if let (Some(tid), Some(tm)) = (task_id, task_manager) {
            if let Ok(mut manager) = tm.lock() {
                manager.update_task(
                    tid,
                    TaskStatus::Running,
                    actual_assets_base_progress,
                    "Validando assets",
                    Some(serde_json::json!({
                        "instanceName": instance.instanceName.clone(),
                        "instanceId": instance.instanceId.clone()
                    })),
                );
            }
        }

        // Validate assets
        Self::emit_status(instance, "instance-downloading-assets", "Validando assets");
        self.revalidate_assets(
            instance,
            task_id,
            task_manager,
            actual_assets_base_progress,
            actual_assets_span,
        )
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

        // Extraemos las librerías nativas en el directorio de nativos con el nombre de la versión
        // por ejemplo /natives/1.20.2

        if !natives_dir.exists() {
            fs::create_dir_all(&natives_dir)
                .map_err(|e| format!("Error creating natives directory: {}", e))?;
        }

        let actual_natives_base_progress = calc_progress(p_natives_extraction_start);
        let actual_natives_span = p_natives_extraction_span * overall_task_max_span;

        if let (Some(tid), Some(tm)) = (task_id, task_manager) {
            if let Ok(mut manager) = tm.lock() {
                manager.update_task(
                    tid,
                    TaskStatus::Running,
                    actual_natives_base_progress,
                    "Extrayendo bibliotecas nativas",
                    Some(serde_json::json!({
                        "instanceName": instance.instanceName.clone(),
                        "instanceId": instance.instanceId.clone()
                    })),
                );
            }
        }

        Self::emit_status(
            instance,
            "instance-extracting-natives",
            "Extrayendo bibliotecas nativas",
        );

        // Extraer bibliotecas nativas
        if let Err(e) = self.extract_natives(
            &version_details,
            &libraries_dir,
            &natives_dir,
            instance,
            task_id,
            task_manager,
            actual_natives_base_progress,
            actual_natives_span,
        ) {
            log::error!("Error extrayendo bibliotecas nativas: {}", e);
            // No devolver error aquí, ya que es opcional
        }

        // Update task status for finalizing setup
        if let (Some(tid), Some(tm)) = (task_id, task_manager) {
            if let Ok(mut manager) = tm.lock() {
                manager.update_task(
                    tid,
                    TaskStatus::Running,
                    calc_progress(p_finalizing_setup),
                    "Finalizando configuración",
                    Some(serde_json::json!({
                        "instanceName": instance.instanceName.clone(),
                        "instanceId": instance.instanceId.clone()
                    })),
                );
            }
        }

        // No emitimos el 100% aquí porque también usamos este método para
        // crear instancias de Forge, y no queremos que se emita el evento
        // de finalización, así que lo hará la función que llame al proceso
        // de bootstrap.

        Self::emit_status(
            instance,
            "vanilla-instance-bootstrapped",
            &format!(
                "Bootstrap de instancia Vanilla {} completado",
                instance.minecraftVersion
            ),
        );

        Ok(())
    }

    fn download_forge_libraries(
        &self,
        version_details: &Value, // This should be the Forge version JSON
        libraries_dir: &Path,
        instance: &MinecraftInstance,
        task_id: Option<&str>,
        task_manager: Option<&Arc<Mutex<TasksManager>>>,
        base_overall_progress: f32,
        max_progress_span_for_this_step: f32,
    ) -> Result<(), String> {
        // Verificar que tengamos la sección de librerías
        let libraries = version_details["libraries"].as_array().ok_or_else(|| {
            "Lista de librerías no encontrada en detalles de versión Forge".to_string()
        })?;

        let total_libraries = libraries.len();
        let mut downloaded_libraries = 0;

        // Initial message before loop
        let initial_message = format!("Iniciando descarga de librerías de Forge (0/{})", total_libraries);
        if let (Some(tid), Some(tm)) = (task_id, task_manager.as_ref()) {
            if let Ok(mut manager) = tm.lock() {
                manager.update_task(tid, TaskStatus::Running, base_overall_progress, &initial_message, None);
            }
        }
        Self::emit_status(instance, "instance-downloading-forge-libraries", &initial_message);


        for library in libraries {
            downloaded_libraries += 1;

            let current_step_progress = if total_libraries > 0 {
                (downloaded_libraries as f32 / total_libraries as f32) * max_progress_span_for_this_step
            } else {
                0.0
            };
            let overall_progress_for_task_update = base_overall_progress + current_step_progress;

            // Verificar reglas de exclusión/inclusión para esta librería
            if let Some(rules) = library.get("rules") {
                let mut allowed = false;

                for rule in rules.as_array().unwrap_or(&Vec::new()) {
                    let action = rule["action"].as_str().unwrap_or("disallow");

                    // Manejar reglas específicas de SO
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
                        // Sin SO especificado, aplicar a todos
                        allowed = action == "allow";
                    }
                }

                if !allowed {
                    continue; // Saltar esta librería
                }
            }

            // Manejo de librerías con formato Maven (común en Forge)
            let name = library.get("name").and_then(Value::as_str).unwrap_or("unknown-library");
            let lib_message_name =  library.get("name").and_then(Value::as_str).unwrap_or_else(|| "unknown library");


            // Si la librería tiene información de descarga directa
            if let Some(downloads) = library.get("downloads") {
                // Descargar artefacto principal
                if let Some(artifact) = downloads.get("artifact") {
                    let path = artifact["path"]
                        .as_str()
                        .ok_or_else(|| format!("Ruta de artefacto no encontrada para {}", name))?;
                    let url = artifact["url"]
                        .as_str()
                        .ok_or_else(|| format!("URL de artefacto no encontrada para {}", name))?;

                    let target_path = libraries_dir.join(path);

                    if let Some(parent) = target_path.parent() {
                        fs::create_dir_all(parent)
                            .map_err(|e| format!("Error al crear directorio para {}: {}", path, e))?;
                    }

                    if !target_path.exists() {
                        self.download_file(
                            url,
                            &target_path,
                            instance,
                            lib_message_name,
                            task_id,
                            task_manager.as_ref(),
                            overall_progress_for_task_update,
                        )
                        .map_err(|e| format!("Error al descargar librería {}: {}", name, e))?;
                    }
                }

                // Descargar librerías nativas (classifiers)
                if let Some(classifiers) = downloads.get("classifiers") {
                    let current_os = if cfg!(target_os = "windows") {
                        "natives-windows"
                    } else if cfg!(target_os = "macos") {
                        "natives-osx" // Ensure this matches the JSON (e.g. natives-osx vs natives-macos)
                    } else {
                        "natives-linux"
                    };

                    if let Some(native) = classifiers.get(current_os) {
                        let url = native["url"]
                            .as_str()
                            .ok_or_else(|| format!("URL de librería nativa no encontrada para {}", name))?;
                        let path_str = native["path"]
                            .as_str()
                            .ok_or_else(|| format!("Ruta de librería nativa no encontrada para {}", name))?;

                        let target_path = libraries_dir.join(path_str);

                        if let Some(parent) = target_path.parent() {
                            fs::create_dir_all(parent)
                                .map_err(|e| format!("Error al crear directorio para nativa {}: {}", path_str, e))?;
                        }

                        if !target_path.exists() {
                            let native_lib_name_detail = format!("{} (native: {})", lib_message_name, path_str);
                            self.download_file(
                                url,
                                &target_path,
                                instance,
                                &native_lib_name_detail,
                                task_id,
                                task_manager.as_ref(),
                                overall_progress_for_task_update,
                            )
                            .map_err(|e| format!("Error al descargar librería nativa {}: {}", name, e))?;
                        }
                    }
                }
            }
            // Para librerías sin información de descarga directa, usar formato Maven
            else if !name.is_empty() && name != "unknown-library" {
                // Parsear el nombre en formato Maven: groupId:artifactId:version[:classifier]
                let parts: Vec<&str> = name.split(':').collect();
                if parts.len() >= 3 {
                    let group_id = parts[0];
                    let artifact_id = parts[1];
                    let version = parts[2];
                    let classifier = if parts.len() > 3 {
                        Some(parts[3])
                    } else {
                        None
                    };

                    // Convertir la especificación de grupo en path
                    let group_path = group_id.replace('.', "/");

                    // Construir la ruta al archivo JAR
                    let jar_name = if let Some(classifier) = classifier {
                        format!("{}-{}-{}.jar", artifact_id, version, classifier)
                    } else {
                        format!("{}-{}.jar", artifact_id, version)
                    };

                    let relative_path =
                        format!("{}/{}/{}/{}", group_path, artifact_id, version, jar_name);
                    let target_path = libraries_dir.join(&relative_path);

                    // Crear directorios padre si es necesario
                    if let Some(parent) = target_path.parent() {
                        fs::create_dir_all(parent)
                            .map_err(|e| format!("Error al crear directorio: {}", e))?;
                    }

                    // Construir la URL para la descarga
                    // Probar primero con el repositorio de Forge
                    let repo_url = library["url"]
                        .as_str()
                        .unwrap_or("https://maven.minecraftforge.net/");
                    let download_url = format!("{}{}", repo_url, relative_path);

                    // Descargar si el archivo no existe
                    if !target_path.exists() {
                        if let Err(e_forge) = self.download_file(
                            &download_url, &target_path, instance, &jar_name, task_id, task_manager.as_ref(), overall_progress_for_task_update
                        ) {
                            let maven_url = format!("https://repo1.maven.org/maven2/{}", relative_path);
                            self.download_file(
                                &maven_url, &target_path, instance, &jar_name, task_id, task_manager.as_ref(), overall_progress_for_task_update
                            ).map_err(|e_maven| {
                                format!(
                                    "Error al descargar librería {} desde múltiples repositorios: Forge ('{}': {}), Maven ('{}': {})",
                                    jar_name, download_url, e_forge, maven_url, e_maven
                                )
                            })?;
                        }
                    }
                } else {
                     log::warn!("Nombre de librería Maven inválido: {}", name);
                }
            } else {
                log::warn!("Librería sin información de descarga o nombre Maven: {:?}", library);
            }

            // Update progress message for the overall Forge library download step
            if downloaded_libraries % 1 == 0 || downloaded_libraries == total_libraries {
                 let message = format!(
                    "Descargando librerías de Forge: {}/{} ({:.1}%)",
                    downloaded_libraries, total_libraries,
                    (downloaded_libraries as f32 * 100.0 / total_libraries as f32)
                );
                Self::emit_status(instance, "instance-downloading-forge-libraries", &message);
                if let (Some(tid), Some(tm)) = (task_id, task_manager.as_ref()) {
                     if let Ok(mut manager) = tm.lock() {
                        manager.update_task(
                            tid,
                            TaskStatus::Running,
                            overall_progress_for_task_update,
                            &message,
                            None,
                        );
                    }
                }
            }
        }
        Ok(())
    }

    fn download_libraries(
        &self,
        version_details: &Value,
        libraries_dir: &Path,
        instance: &MinecraftInstance,
        task_id: Option<&str>,
        task_manager: Option<&Arc<Mutex<TasksManager>>>,
        base_overall_progress: f32,
        max_progress_span_for_this_step: f32,
    ) -> Result<(), String> {
        let libraries = version_details["libraries"]
            .as_array()
            .ok_or_else(|| "Libraries list not found in version details".to_string())?;

        let total_libraries = libraries.len();
        let mut downloaded_libraries = 0;

        for library in libraries {
            downloaded_libraries += 1; // Increment at the start of processing each library

            // Calculate progress for this specific library download step
            let current_step_progress = if total_libraries > 0 {
                (downloaded_libraries as f32 / total_libraries as f32) * max_progress_span_for_this_step
            } else {
                0.0
            };
            let overall_progress_for_task_update = base_overall_progress + current_step_progress;

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
            let downloads_node = library.get("downloads"); // Use a different name to avoid conflict

            let artifact_path_op = downloads_node.and_then(|d| d.get("artifact")).and_then(|a| a.get("path")).and_then(|p| p.as_str());
            let library_name_for_message = artifact_path_op.unwrap_or_else(|| library.get("name").and_then(|n| n.as_str()).unwrap_or("unknown library"));


            if let Some(downloads) = downloads_node {
                // Handle artifact
                if let Some(artifact) = downloads.get("artifact") {
                    let path = artifact["path"]
                        .as_str()
                        .ok_or_else(|| "Library artifact path not found".to_string())?;
                    let url = artifact["url"]
                        .as_str()
                        .ok_or_else(|| "Library artifact URL not found".to_string())?;

                    let target_path = libraries_dir.join(path);

                    if let Some(parent) = target_path.parent() {
                        fs::create_dir_all(parent)
                            .map_err(|e| format!("Error creating directory: {}", e))?;
                    }

                    if !target_path.exists() {
                        self.download_file(
                            url,
                            &target_path,
                            instance,
                            library_name_for_message,
                            task_id,
                            task_manager.as_ref(),
                            overall_progress_for_task_update,
                        )
                        .map_err(|e| format!("Error downloading library {}: {}", path, e))?;
                    }
                }

                // Handle native libraries (classifiers)
                if let Some(classifiers) = downloads.get("classifiers") {
                let current_os_key = if cfg!(target_os = "windows") { // Renamed for clarity
                    "natives-windows"
                } else if cfg!(target_os = "macos") {
                    "natives-osx" // Ensure this matches the JSON (e.g. natives-osx vs natives-macos)
                } else {
                    "natives-linux"
                };

                if let Some(native) = classifiers.get(current_os_key) {
                    let url = native["url"]
                        .as_str()
                        .ok_or_else(|| "Native library URL not found".to_string())?;
                    let path_str = native["path"] // Renamed to avoid conflict with outer `path`
                        .as_str()
                        .ok_or_else(|| "Native library path not found".to_string())?;

                    let target_path = libraries_dir.join(path_str);

                    // Create parent directories if needed
                    if let Some(parent) = target_path.parent() {
                        fs::create_dir_all(parent)
                            .map_err(|e| format!("Error creating directory: {}", e))?;
                    }

                    // Download if file doesn't exist
                    if !target_path.exists() {
                        let native_lib_name = format!("{} (native: {})", library_name_for_message, path_str);
                        self.download_file(
                            url,
                            &target_path,
                            instance,
                            &native_lib_name,
                            task_id,
                            task_manager.as_ref(),
                            overall_progress_for_task_update,
                        )
                        .map_err(|e| format!("Error downloading native library {}: {}", path_str, e))?;
                    }
                }
            }
            } else if let Some(lib_name) = library.get("name").and_then(Value::as_str) {
                // This case might be for libraries specified by name only, without explicit download sections
                // This was more common in very old Forge versions or if the manifest assumes libraries are present
                log::warn!("Library {} does not have explicit download information. Skipping download.", lib_name);
            }


            // Update overall task progress message for the library downloading step
            // (not for each individual file download, download_file handles that for its part)
            if downloaded_libraries % 1 == 0 || downloaded_libraries == total_libraries { // Update more frequently or as needed
                let message = format!(
                    "Descargando librerías: {}/{} ({:.1}%)",
                    downloaded_libraries, total_libraries,
                    (downloaded_libraries as f32 * 100.0 / total_libraries as f32) // This is percentage of libraries, not overall
                );
                Self::emit_status(instance, "instance-downloading-libraries", &message);

                if let (Some(tid), Some(tm)) = (task_id, task_manager.as_ref()) {
                    if let Ok(mut manager) = tm.lock() {
                        // Use overall_progress_for_task_update for the actual progress value
                        manager.update_task(
                            tid,
                            TaskStatus::Running,
                            overall_progress_for_task_update,
                            &message, // This message shows lib X/Y, not the download_file specific one
                            None,
                        );
                    }
                }
            }
        }
        Ok(())
    }

    pub fn bootstrap_forge_instance(
        &mut self,
        instance: &MinecraftInstance,
        task_id: Option<String>,
        task_manager: Option<Arc<Mutex<TasksManager>>>,
    ) -> Result<(), String> {
        // Verificar que tengamos información de Forge
        if instance.forgeVersion.is_none() || instance.forgeVersion.as_ref().unwrap().is_empty() {
            return Err("No se especificó versión de Forge".to_string());
        }

        // Emit start event
        Self::emit_status(
            instance,
            "instance-bootstrap-start",
            "Iniciando bootstrap de instancia Forge",
        );

        let forge_overall_start_progress = 0.0; // Forge bootstrap starts at 0% of its own task
        let forge_vanilla_setup_span = 0.50; // Vanilla setup takes 50% of Forge bootstrap
        let forge_dl_installer_span = 0.05;  // Downloading Forge installer 5%
        let forge_run_installer_span = 0.15; // Running Forge installer 15%
        let forge_dl_libs_span = 0.25;       // Downloading Forge libs 25%
                                             // Remaining 5% for final setup.

        // Update task status if task_id exists
        if let (Some(tid), Some(tm)) = (&task_id, &task_manager) {
            if let Ok(mut manager) = tm.lock() {
                manager.update_task(
                    tid,
                    TaskStatus::Running,
                    forge_overall_start_progress + 0.01 * 100.0, // Small initial progress
                    "Iniciando bootstrap de instancia Forge",
                    Some(serde_json::json!({
                        "instanceName": instance.instanceName.clone(),
                        "instanceId": instance.instanceId.clone()
                    })),
                );
            }
        }


        // Primero, realizar bootstrap de la instancia Vanilla
        Self::emit_status(
            instance,
            "instance-forge-vanilla-setup",
            "Configurando base Vanilla",
        );

        // --- Vanilla Setup Step ---
        let vanilla_setup_base_progress = forge_overall_start_progress; // Starts at 0 for Forge
        if let (Some(tid), Some(tm)) = (task_id.as_deref(), task_manager.as_ref()) {
             if let Ok(mut manager) = tm.lock() {
                manager.update_task(
                    tid,
                    TaskStatus::Running,
                    vanilla_setup_base_progress + 0.01 * forge_vanilla_setup_span * 100.0, // Small progress into this step
                    "Configurando base Vanilla",
                    Some(serde_json::json!({
                        "instanceName": instance.instanceName.clone(),
                        "instanceId": instance.instanceId.clone()
                    })),
                );
            }
        }

        // Bootstrap Vanilla primero
        self.bootstrap_vanilla_instance(
            instance,
            task_id.as_deref(), // Pass along task_id if present
            task_manager.as_ref(),  // Pass along task_manager if present
            vanilla_setup_base_progress,
            forge_vanilla_setup_span * 100.0, // bootstrap_vanilla_instance expects span in 0-100 range
        )
        .map_err(|e| format!("Error en bootstrap Vanilla: {}", e))?;

        // --- Forge Installer Download Step ---
        let dl_installer_base_progress = vanilla_setup_base_progress + forge_vanilla_setup_span * 100.0;
        if let (Some(tid), Some(tm)) = (task_id.as_deref(), task_manager.as_ref()) {
            if let Ok(mut manager) = tm.lock() {
                manager.update_task(
                    tid,
                    TaskStatus::Running,
                    dl_installer_base_progress,
                    "Descargando instalador de Forge",
                    Some(serde_json::json!({
                        "instanceName": instance.instanceName.clone(),
                        "instanceId": instance.instanceId.clone()
                    })),
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
            &format!(
                "Descargando Forge {} para Minecraft {}",
                forge_version, instance.minecraftVersion
            ),
        );

        // Crear directorio específico para la versión de Forge
        let forge_version_name = format!("{}-forge-{}", instance.minecraftVersion, forge_version);
        let forge_version_dir = versions_dir.join(&forge_version_name);

        if !forge_version_dir.exists() {
            fs::create_dir_all(&forge_version_dir)
                .map_err(|e| format!("Error al crear directorio de versión Forge: {}", e))?;
        }

        // Obtener URL de instalador Forge
        let forge_installer_url =
            self.get_forge_installer_url(&instance.minecraftVersion, forge_version)?;

        // Path para el instalador
        let forge_installer_path = minecraft_dir.join("forge-installer.jar");

        // Descargar instalador Forge
        Self::emit_status(
            instance,
            "instance-downloading-forge-installer",
            "Descargando instalador de Forge",
        );
        self.download_file(
            &forge_installer_url,
            &forge_installer_path,
            instance,
            "Forge Installer",
            task_id.as_deref(),
            task_manager.as_ref(),
            dl_installer_base_progress, // This is the current overall progress for this specific file download
        )
        .map_err(|e| format!("Error al descargar instalador Forge: {}", e))?;

        // --- Run Forge Installer Step ---
        let run_installer_base_progress = dl_installer_base_progress + forge_dl_installer_span * 100.0;
        if let (Some(tid), Some(tm)) = (task_id.as_deref(), task_manager.as_ref()) {
            if let Ok(mut manager) = tm.lock() {
                manager.update_task(
                    tid,
                    TaskStatus::Running,
                    run_installer_base_progress,
                    "Ejecutando instalador de Forge",
                    Some(serde_json::json!({
                        "instanceName": instance.instanceName.clone(),
                        "instanceId": instance.instanceId.clone()
                    })),
                );
            }
        }

        // Ejecutar instalador en modo silencioso
        let installer_run_message = "Ejecutando instalador de Forge, esto puede tardar...";
        Self::emit_status(
            instance,
            "instance-installing-forge",
            installer_run_message,
        );
        if let (Some(tid), Some(tm)) = (task_id.as_deref(), task_manager.as_ref()) {
            if let Ok(mut manager) = tm.lock() {
                manager.update_task(
                    tid,
                    TaskStatus::Running,
                    run_installer_base_progress, // Progress is already at the base for this step
                    installer_run_message,
                    None, // Payload can be existing or None
                );
            }
        }

        // Preparar argumentos para instalar Forge
        let forge_install_result = self.run_forge_installer(
            &forge_installer_path,
            &minecraft_dir,
            &instance.minecraftVersion,
            forge_version,
            instance,
        )?;

        // Update task status after installer run and before downloading Forge libs
        let dl_forge_libs_base_progress = run_installer_base_progress + forge_run_installer_span * 100.0;
        if let (Some(tid), Some(tm)) = (task_id.as_deref(), task_manager.as_ref()) {
            if let Ok(mut manager) = tm.lock() {
                manager.update_task(
                    tid,
                    TaskStatus::Running,
                    dl_forge_libs_base_progress,
                    "Descargando librerías de Forge",
                    Some(serde_json::json!({
                        "instanceName": instance.instanceName.clone(),
                        "instanceId": instance.instanceId.clone()
                    })),
                );
            }
        }

        // Crear/actualizar perfil de Forge en launcher_profiles.json
        let launcher_profiles_path = minecraft_dir.join("launcher_profiles.json");
        self.update_launcher_profiles(
            &launcher_profiles_path,
            &forge_version_name,
            &instance.instanceName,
        )?;

        // Descargar librerías de Forge
        Self::emit_status(
            instance,
            "instance-downloading-forge-libraries",
            "Descargando librerías de Forge",
        );

        // This emit_status is fine, download_forge_libraries will handle more detailed ones
        Self::emit_status(
            instance,
            "instance-downloading-forge-libraries",
            "Descargando librerías de Forge",
        );

        // Note: The old `emit_status` inside `download_forge_libraries` for 0/total might be redundant now,
        // as we set a message just before calling it.

        if let (Some(tid), Some(tm)) = (task_id.as_deref(), task_manager.as_ref()) {
            // The message update for this step is already done above.
            // download_forge_libraries will provide more granular updates.
            // No immediate task_manager.update_task here unless the message above was insufficient.
            // The base progress for download_forge_libraries is dl_forge_libs_base_progress.
        }

        // Descargar librerías de Forge
        // Leer el archivo de versión para obtener los detalles de las librerías
        let forge_version_json_path =
            forge_version_dir.join(format!("{}.json", forge_version_name));

        if forge_version_json_path.exists() {
            let version_json = fs::read_to_string(&forge_version_json_path)
                .map_err(|e| format!("Error al leer archivo de versión Forge: {}", e))?;

            let version_details: Value = serde_json::from_str(&version_json)
                .map_err(|e| format!("Error al parsear archivo de versión Forge: {}", e))?;

            // Descargar librerías específicas de Forge
            self.download_forge_libraries(
                &version_details,
                &libraries_dir,
                instance,
                task_id.as_deref(),
                task_manager.as_ref(),
                dl_forge_libs_base_progress,
                forge_dl_libs_span * 100.0, // Span is also 0-100 range
            )?;
        } else {
            return Err(format!(
                "No se encontró el archivo de versión Forge: {}",
                forge_version_json_path.display()
            ));
        }

        // Update task status - Finalizing
        let final_setup_progress = dl_forge_libs_base_progress + forge_dl_libs_span * 100.0;
        if let (Some(tid), Some(tm)) = (task_id.as_deref(), task_manager.as_ref()) {
            if let Ok(mut manager) = tm.lock() {
                manager.update_task(
                    tid,
                    TaskStatus::Running,
                    final_setup_progress,
                    "Configurando Forge",
                    Some(serde_json::json!({
                        "instanceName": instance.instanceName.clone(),
                        "instanceId": instance.instanceId.clone()
                    })),
                );
            }
        }

        // Descargar librerías de Forge
        // Leer el archivo de versión para obtener los detalles de las librerías
        let forge_version_json_path =
            forge_version_dir.join(format!("{}.json", forge_version_name));

        if forge_version_json_path.exists() {
            let version_json = fs::read_to_string(&forge_version_json_path)
                .map_err(|e| format!("Error al leer archivo de versión Forge: {}", e))?;

            let version_details: Value = serde_json::from_str(&version_json)
                .map_err(|e| format!("Error al parsear archivo de versión Forge: {}", e))?;

            // Descargar librerías específicas de Forge
            self.download_forge_libraries(&version_details, &libraries_dir, instance)?;
        } else {
            return Err(format!(
                "No se encontró el archivo de versión Forge: {}",
                forge_version_json_path.display()
            ));
        }

        // Update task status - 95%
        if let (Some(task_id), Some(task_manager)) = (&task_id, &task_manager) {
            if let Ok(mut tm) = task_manager.lock() {
                tm.update_task(
                    task_id,
                    TaskStatus::Running,
                    95.0,
                    "Configurando Forge",
                    Some(serde_json::json!({
                        "instanceName": instance.instanceName.clone(),
                        "instanceId": instance.instanceId.clone()
                    })),
                );
            }
        }

        // Limpiar instalador Forge para ahorrar espacio
        if forge_installer_path.exists() {
            if let Err(e) = fs::remove_file(forge_installer_path) {
                log::info!(
                    "Advertencia: No se pudo borrar el instalador de Forge: {}",
                    e
                );
            }
        }

        // Update task status - 100%
        if let (Some(task_id), Some(task_manager)) = (&task_id, &task_manager) {
            if let Ok(mut tm) = task_manager.lock() {
                tm.update_task(
                    task_id,
                    TaskStatus::Completed,
                    100.0,
                    &format!(
                        "Instalación completada: Forge {} para Minecraft {}",
                        forge_version, instance.minecraftVersion
                    ),
                    Some(serde_json::json!({
                        "instanceName": instance.instanceName.clone(),
                        "instanceId": instance.instanceId.clone()
                    })),
                );
            }
        }

        Self::emit_status(
            instance,
            "forge-instance-bootstrapped",
            &format!(
                "Bootstrap de instancia Forge {} para Minecraft {} completado",
                forge_version, instance.minecraftVersion
            ),
        );

        Ok(())
    }

    fn get_forge_installer_url(
        &self,
        minecraft_version: &str,
        forge_version: &str,
    ) -> Result<String, String> {
        let base = "https://maven.minecraftforge.net/net/minecraftforge/forge";

        let mc_compact = format!("mc{}", minecraft_version.replace('.', ""));

        let mut attempts = vec![
            // Modern
            (
                format!("{minecraft_version}-{forge_version}"),
                vec![
                    format!("forge-{minecraft_version}-{forge_version}-installer.jar"),
                    format!("forge-{minecraft_version}-{forge_version}-universal.jar"),
                ],
            ),
            // Dot-separated
            (
                format!("{minecraft_version}.{forge_version}"),
                vec![
                    format!("forge-{minecraft_version}.{forge_version}-installer.jar"),
                    format!("forge-{minecraft_version}.{forge_version}-universal.jar"),
                ],
            ),
            // Only forge version
            (
                forge_version.to_string(),
                vec![
                    format!("forge-{forge_version}-installer.jar"),
                    format!("forge-{forge_version}-universal.jar"),
                ],
            ),
            // Legacy style with full forge version
            (
                forge_version.to_string(),
                vec![
                    format!("forge-{forge_version}-installer.jar"),
                    format!("forge-{forge_version}-universal.jar"),
                ],
            ),
            // 🧠 Caso especial: -mcXYZ
            (
                format!("{minecraft_version}-{forge_version}-{mc_compact}"),
                vec![
                    format!("forge-{minecraft_version}-{forge_version}-{mc_compact}-installer.jar"),
                    format!("forge-{minecraft_version}-{forge_version}-{mc_compact}-universal.jar"),
                ],
            ),
        ];

        for (folder, files) in attempts.drain(..) {
            for file in files {
                let url = format!("{}/{}/{}", base, folder, file);

                log::info!("[Forge] Probando URL: {}", url);

                if self
                    .client
                    .head(&url)
                    .send()
                    .map_or(false, |r| r.status().is_success())
                {
                    return Ok(url);
                }
            }
        }

        log::warn!(
            "No se encontró una URL válida para Forge {} - {}",
            minecraft_version,
            forge_version
        );

        Err(format!(
            "No se encontró una URL válida para Forge {} - {}",
            minecraft_version, forge_version
        ))
    }

    fn run_forge_installer(
        &self,
        installer_path: &Path,
        minecraft_dir: &Path,
        minecraft_version: &str,
        forge_version: &str,
        instance: &MinecraftInstance,
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

        // Lista de opciones de instalación para probar secuencialmente
        let install_options = ["--installClient", "--installDir", "--installServer"];

        let mut success = false;
        let mut last_error = String::new();

        // Intentar cada opción de instalación hasta que una tenga éxito
        for &option in &install_options {
            // Preparar comando para ejecutar el instalador con la opción actual
            let mut install_cmd = Command::new(&java_path);
            install_cmd
                .arg("-jar")
                .arg(installer_path)
                .arg(option)
                .current_dir(minecraft_dir);

            // Ejecutar instalador con la opción actual
            log::info!("Ejecutando instalador Forge con comando: {:?}", install_cmd);

            match install_cmd.output() {
                Ok(output) => {
                    if output.status.success() {
                        success = true;
                        log::info!(
                            "Instalación de Forge completada con éxito usando {}",
                            option
                        );
                        break;
                    } else {
                        let error_msg = String::from_utf8_lossy(&output.stderr);
                        log::warn!(
                            "Fallo en instalación de Forge con {}: {}",
                            option,
                            error_msg
                        );
                        last_error = format!(
                            "Error en instalación de Forge con {}: {}",
                            option, error_msg
                        );
                    }
                }
                Err(e) => {
                    log::warn!(
                        "Error al ejecutar instalador de Forge con {}: {}",
                        option,
                        e
                    );
                    last_error = format!(
                        "Error al ejecutar instalador de Forge con {}: {}",
                        option, e
                    );
                }
            }
        }

        // Limpiar archivo temporal de instalación
        if install_profile.exists() {
            let _ = fs::remove_file(install_profile);
        }

        // Verificar resultado final
        if success {
            Ok(())
        } else {
            log::error!(
                "Todos los métodos de instalación de Forge fallaron. Último error: {}",
                last_error
            );
            Err(format!(
                "Todos los métodos de instalación de Forge fallaron. Último error: {}",
                last_error
            ))
        }
    }

    fn find_java_path(&self) -> Result<String, String> {
        let config_lock = get_config_manager()
            .lock()
            .expect("Failed to lock config manager mutex");

        let config = config_lock
            .as_ref()
            .expect("Config manager failed to initialize");

        let java_path = config
            .get_java_dir()
            .ok_or_else(|| "Java path is not set".to_string())?
            .join("bin")
            .join(if cfg!(windows) { "javaw.exe" } else { "java" });

        if !java_path.exists() {
            return Err(format!(
                "Java executable not found at: {}",
                java_path.display()
            ));
        }
        Ok(java_path.to_string_lossy().to_string())
    }

    fn update_launcher_profiles(
        &self,
        profiles_path: &Path,
        version_id: &str,
        instance_name: &str,
    ) -> Result<(), String> {
        // Leer archivo de perfiles actual
        let profiles_content = match fs::read_to_string(profiles_path) {
            Ok(content) => content,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                // Si no existe, crear uno básico
                "{ \"profiles\": {}, \"settings\": {}, \"version\": 3 }".to_string()
            }
            Err(e) => return Err(format!("Error al leer archivo de perfiles: {}", e)),
        };

        // Parsear JSON
        let mut profiles_json: Value = serde_json::from_str(&profiles_content)
            .map_err(|e| format!("Error al parsear archivo de perfiles: {}", e))?;

        // Crear o actualizar perfil de Forge
        let profile_id = format!("forge-{}", version_id);
        let profiles = profiles_json["profiles"]
            .as_object_mut()
            .ok_or_else(|| "Formato inválido en archivo de perfiles".to_string())?;

        // Generar fecha actual en formato ISO
        let now = chrono::Utc::now();
        let date_str = now.format("%Y-%m-%dT%H:%M:%S.%3fZ").to_string();

        // Crear nuevo perfil
        profiles.insert(
            profile_id.clone(),
            json!({
                "created": date_str,
                "lastUsed": date_str,
                "lastVersionId": version_id,
                "name": format!("{} (Forge)", instance_name),
                "type": "custom"
            }),
        );

        // Guardar archivo actualizado
        fs::write(
            profiles_path,
            serde_json::to_string_pretty(&profiles_json).unwrap(),
        )
        .map_err(|e| format!("Error al guardar archivo de perfiles: {}", e))?;

        Ok(())
    }

    pub fn verify_integrity_vanilla(
        &self,
        instance: Option<&MinecraftInstance>,
        task_id: Option<String>,
        task_manager: Option<Arc<Mutex<TasksManager>>>,
    ) -> Result<(), String> {
        // Verificar integridad de la instancia Vanilla
        let instance = instance.ok_or_else(|| "Instance is not provided".to_string())?;

        Self::emit_status(
            instance,
            "instance-verifying-vanilla",
            "Verificando integridad de la instancia Vanilla",
        );

        // Update task status if task_id exists
        if let (Some(task_id), Some(task_manager)) = (&task_id, &task_manager) {
            if let Ok(mut tm) = task_manager.lock() {
                tm.update_task(
                    task_id,
                    TaskStatus::Running,
                    5.0,
                    "Verificando integridad de la instancia Vanilla",
                    Some(serde_json::json!({
                        "instanceName": instance.instanceName.clone(),
                        "instanceId": instance.instanceId.clone()
                    })),
                );
            }
        }

        // Get manifest for the minecraft version, and check each dependency
        // And download missing files

        let instance_dir = Path::new(instance.instanceDirectory.as_deref().unwrap_or(""));
        let minecraft_dir = instance_dir.join("minecraft");
        let versions_dir = minecraft_dir.join("versions");
        let natives_dir = minecraft_dir
            .join("natives")
            .join(&instance.minecraftVersion);
        let instance_version_dir = versions_dir.join(&instance.minecraftVersion);
        let instance_version_json_path =
            instance_version_dir.join(format!("{}.json", instance.minecraftVersion));
        let libraries_dir = minecraft_dir.join("libraries");

        // Get the version manifest
        let version_manifest_url = "https://launchermeta.mojang.com/mc/game/version_manifest.json";
        let version_manifest: Value = self
            .client
            .get(version_manifest_url)
            .send()
            .map_err(|e| format!("Error al obtener el manifiesto de versiones: {}", e))?
            .json()
            .map_err(|e| format!("Error al parsear el manifiesto de versiones: {}", e))?;
        let versions = version_manifest["versions"]
            .as_array()
            .ok_or_else(|| "No se encontraron versiones en el manifiesto".to_string())?;
        let version_id = instance.minecraftVersion.clone();
        let version_info = versions
            .iter()
            .find(|v| v["id"].as_str() == Some(&version_id))
            .ok_or_else(|| format!("No se encontró la versión {} en el manifiesto", version_id))?;
        let version_url = version_info["url"]
            .as_str()
            .ok_or_else(|| "No se encontró la URL de la versión".to_string())?;
        let version_details: Value = self
            .client
            .get(version_url)
            .send()
            .map_err(|e| format!("Error al obtener los detalles de la versión: {}", e))?
            .json()
            .map_err(|e| format!("Error al parsear los detalles de la versión: {}", e))?;

        // Get the libraries from the version details
        let libraries = version_details["libraries"].as_array().ok_or_else(|| {
            "No se encontraron librerías en los detalles de la versión".to_string()
        })?;
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
            let downloads = library
                .get("downloads")
                .ok_or_else(|| "Library downloads info not found".to_string())?;

            // Handle artifact
            if let Some(artifact) = downloads.get("artifact") {
                let path = artifact["path"]
                    .as_str()
                    .ok_or_else(|| "Library artifact path not found".to_string())?;
                let url = artifact["url"]
                    .as_str()
                    .ok_or_else(|| "Library artifact URL not found".to_string())?;

                let target_path = libraries_dir.join(path);

                // Create parent directories if needed
                if let Some(parent) = target_path.parent() {
                    fs::create_dir_all(parent)
                        .map_err(|e| format!("Error creating directory: {}", e))?;
                }

                // Download if file doesn't exist
                if !target_path.exists() {
                    // In verify_integrity_vanilla, we might not have a top-level task_id,
                    // or the progress calculation might be different.
                    // For now, pass None/0.0, assuming detailed progress here is less critical
                    // or will be handled when this function is refactored for progress.
                    self.download_file(url, &target_path, instance, path, None, None, 0.0)
                        .map_err(|e| format!("Error downloading library: {}", e))?;
                }
            }

            downloaded_libraries += 1;

            // Update progress every 5 libraries or on last library
            if downloaded_libraries % 5 == 0 || downloaded_libraries == total_libraries {
                let progress = (downloaded_libraries as f32 / total_libraries as f32) * 100.0;
                Self::emit_status(
                    instance,
                    "instance-verifying-libraries",
                    &format!(
                        "Verificando librerías: {}/{} ({:.1}%)",
                        downloaded_libraries, total_libraries, progress
                    ),
                );
                // Update task status if task_id exists
                if let (Some(task_id), Some(task_manager)) = (&task_id, &task_manager) {
                    if let Ok(mut tm) = task_manager.lock() {
                        tm.update_task(
                            task_id,
                            TaskStatus::Running,
                            progress,
                            "Verificando librerías",
                            Some(serde_json::json!({
                                "instanceName": instance.instanceName.clone(),
                                "instanceId": instance.instanceId.clone()
                            })),
                        );
                    }
                }
            }
        }

        // Extraer bibliotecas nativas
        if let Err(e) =
            self.extract_natives(&version_details, &libraries_dir, &natives_dir, instance)
        {
            log::error!("Error extrayendo bibliotecas nativas: {}", e);
            // No devolver error aquí, ya que es opcional
        }

        // Emit end event
        Self::emit_status(
            instance,
            "instance-verifying-complete",
            "Verificación de la instancia Vanilla completada",
        );
        // Update task status if task_id exists
        if let (Some(task_id), Some(task_manager)) = (&task_id, &task_manager) {
            if let Ok(mut tm) = task_manager.lock() {
                tm.update_task(
                    task_id,
                    TaskStatus::Completed,
                    100.0,
                    "Verificación de la instancia Vanilla completada",
                    Some(serde_json::json!({
                        "instanceName": instance.instanceName.clone(),
                        "instanceId": instance.instanceId.clone()
                    })),
                );
            }
        }

        Ok(())
    }
}

#[tauri::command]
pub fn check_vanilla_integrity(instance_id: String) -> Result<(), String> {
    // Obtener la instancia de Minecraft
    let instance = get_instance_by_id(instance_id)
        .map_err(|e| format!("Error al obtener la instancia: {}", e))?;

    if instance.is_none() {
        return Err("No se encontró la instancia".to_string());
    }

    let bootstrapper = InstanceBootstrap::new();
    // Verificar que la instancia sea válida

    // Verificar la integridad de la instancia
    bootstrapper
        .verify_integrity_vanilla(instance.as_ref(), None, None)
        .map_err(|e| format!("Error al verificar la integridad de la instancia: {}", e))?;

    Ok(())
}
