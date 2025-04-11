// src-tauri/src/core/instance_manager.rs

use crate::core::minecraft_instance;
use crate::core::minecraft_instance::MinecraftInstance;
use crate::core::models::ModpackInfo;
use crate::utils::config_manager::get_config_manager;
use dirs::config_dir;
use serde_json::from_str;
use std::fs;
use std::path::{Path, PathBuf};
#[tauri::command]
pub fn get_all_instances() -> Result<Vec<MinecraftInstance>, String> {
    let instances_dir = get_config_manager().lock().unwrap().get_instances_dir(); // Obtén el directorio de instancias desde la configuración
    get_instances(instances_dir.to_str().unwrap()) // Pasa la ruta como string
}

#[tauri::command]
pub fn get_instance_by_name(instanceName: String) -> Result<Option<MinecraftInstance>, String> {
    let instances_dir = get_config_manager().lock().unwrap().get_instances_dir(); // Obtén el directorio de instancias desde la configuración
    let instances = get_instances(instances_dir.to_str().unwrap())?;
    Ok(instances.into_iter().find(|i| i.instanceName == instanceName))
}

#[tauri::command]
pub fn get_instance_by_id(instanceId: String) -> Result<Option<MinecraftInstance>, String> {
    let instances_dir = get_config_manager().lock().unwrap().get_instances_dir(); // Obtén el directorio de instancias desde la configuración
    let instances = get_instances(instances_dir.to_str().unwrap())?;
    Ok(instances.into_iter().find(|i| i.instanceId == instanceId))
}

#[tauri::command]
pub fn delete_instance(instance_path: String) -> Result<(), String> {
    let path = Path::new(&instance_path);
    if path.exists() && path.is_dir() {
        fs::remove_dir_all(path).map_err(|e| format!("Failed to delete instance: {}", e))?;
    }
    Ok(())
}

#[tauri::command]
pub fn launch_mc_instance(instance_id: String) -> Result<(), String> {
    let instances_dir = get_config_manager()
        .lock()
        .unwrap()
        .get_instances_dir();

    let instances = get_instances(instances_dir.to_str().unwrap())?;

    let instance = instances
        .into_iter()
        .find(|i| i.instanceId == instance_id)
        .ok_or_else(|| format!("Instance with ID {} not found", instance_id))?;

    instance
        .launch()
        .map_err(|e| format!("Failed to launch instance: {}", e))?;

    Ok(())
}

fn get_instances(instances_dir: &str) -> Result<Vec<MinecraftInstance>, String> {
    let path = Path::new(instances_dir);

    if !path.exists() || !path.is_dir() {
        return Ok(Vec::new());
    }

    let mut instances = Vec::new();

    for entry in fs::read_dir(path).map_err(|e| format!("Error reading directory: {}", e))? {
        let entry = entry.map_err(|e| format!("Error reading entry: {}", e))?;
        let instance_path = entry.path();

        if instance_path.is_dir() {
            let config_file = instance_path.join("instance.json");

            if config_file.exists() {
                let contents = fs::read_to_string(&config_file)
                    .map_err(|e| format!("Error reading JSON: {}", e))?;

                let mut instance: MinecraftInstance =
                    from_str(&contents).map_err(|e| format!("Error parsing JSON: {}", e))?;

                // Normalizamos las rutas usando Path
                instance.instanceDirectory = Some(instance_path.to_string_lossy().to_string());

                // Usamos Path::join para construir rutas de manera segura entre plataformas
                let minecraft_path = instance_path.join("minecraft");
                instance.minecraftPath = minecraft_path.to_string_lossy().to_string();

                // Manejamos los errores al guardar
                if let Err(e) = instance.save() {
                    println!("Warning: Failed to save instance config: {}", e);
                }

                instances.push(instance);
            }
        }
    }

    Ok(instances)
}
