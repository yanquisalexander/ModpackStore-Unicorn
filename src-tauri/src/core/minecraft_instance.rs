// src-tauri/src/minecraft_instance.rs
use crate::core::minecraft_launcher::InstanceLauncher;
use crate::core::tasks_manager::{TaskInfo, TaskStatus, TasksManager};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Result as IoResult;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ModpackInfo {
    pub name: Option<String>,
    pub version: Option<String>,
    pub author: Option<String>,
    // Otros campos según necesites
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MinecraftInstance {
    pub instanceId: String,
    pub usesDefaultIcon: bool,
    pub iconName: Option<String>,
    pub iconUrl: Option<String>,
    pub instanceName: String,
    pub accountUuid: Option<String>,
    pub minecraftPath: String,
    pub modpackId: Option<String>,
    pub modpackInfo: Option<ModpackInfo>,
    pub minecraftVersion: String,
    pub instanceDirectory: Option<String>,
    pub forgeVersion: Option<String>,
}

impl MinecraftInstance {
    pub fn is_forge_instance(&self) -> bool {
        self.forgeVersion.is_some()
    }

    pub fn from_instance_id(instance_id: &str) -> Option<Self> {
        let instance_directory = Path::new("instances").join(instance_id);
        if !instance_directory.exists() {
            return None;
        }

        let config_file = instance_directory.join("instance.json");
        if !config_file.exists() {
            return None;
        }

        match fs::read_to_string(config_file) {
            Ok(content) => {
                match serde_json::from_str::<MinecraftInstance>(&content) {
                    Ok(mut instance) => {
                        // Aseguramos que instanceDirectory sea una ruta válida
                        // y que no esté vacía
                        if instance.instanceDirectory.is_none() {
                            instance.instanceDirectory =
                                Some(instance_directory.to_string_lossy().to_string());
                        } else {
                            instance.instanceDirectory =
                                Some(instance.instanceDirectory.unwrap_or_default());
                        }
                        Some(instance)
                    }
                    Err(_) => None,
                }
            }
            Err(_) => None,
        }
    }

    pub fn from_directory(directory: &Path) -> Option<Self> {
        let config_file = directory.join("instance.json");
        if !config_file.exists() {
            return None;
        }

        match fs::read_to_string(config_file) {
            Ok(content) => {
                match serde_json::from_str::<MinecraftInstance>(&content) {
                    Ok(mut instance) => {
                        // Aseguramos que instanceDirectory sea una ruta válida
                        // y que no esté vacía
                        if instance.instanceDirectory.is_none() {
                            instance.instanceDirectory =
                                Some(directory.to_string_lossy().to_string());
                        } else {
                            instance.instanceDirectory =
                                Some(instance.instanceDirectory.unwrap_or_default());
                        }
                        Some(instance)
                    }
                    Err(_) => None,
                }
            }
            Err(_) => None,
        }
    }

    pub fn save(&self) -> IoResult<()> {
        let config_file = Path::new(&self.instanceDirectory.as_ref().unwrap_or(&String::new()))
            .join("instance.json");
        let content = serde_json::to_string_pretty(self)?;
        fs::write(config_file, content)
    }

    pub fn delete(&self) -> IoResult<()> {
        if let Some(directory) = &self.instanceDirectory {
            fs::remove_dir_all(directory)
        } else {
            Ok(())
        }
    }

    pub fn launch(&self) -> Result<(), String> {
        let launcher = InstanceLauncher::new(self.clone());
        launcher.launch_instance_async();

        println!(
            "[Tauri Command] Successfully initiated async launch for {}",
            self.instanceName
        );
        Ok(())
    }
}

#[tauri::command]
pub fn save_minecraft_instance(instance: MinecraftInstance) -> bool {
    instance.save().is_ok()
}

#[tauri::command]
pub fn revalidate_assets(instance: MinecraftInstance) -> Result<(), String> {
    // Implementar la lógica para revalidar assets
    println!(
        "Revalidating assets for instance: {}",
        instance.instanceName
    );
    Ok(())
}

#[tauri::command]
pub fn open_game_dir(instance_id: String) -> Result<(), String> {
    let instance = MinecraftInstance::from_instance_id(&instance_id);
    if let Some(instance) = instance {
        let path = PathBuf::from(instance.minecraftPath);
        if path.exists() {
            // Abre el directorio del juego con el programa predeterminado del sistema
            if let Err(e) = tauri_plugin_opener::open_path(path,  None::<&str>) {
                return Err(format!("Error opening game directory: {}", e));
            }
            Ok(())
        } else {
            Err("Game directory does not exist".to_string())
        }
    } else {
        Err("Instance not found".to_string())
    }
}
