// src-tauri/src/minecraft_instance.rs
use crate::core::instance_launcher::InstanceLauncher;
use crate::core::tasks_manager::{TaskInfo, TaskStatus, TasksManager};
use crate::utils::config_manager::ConfigManager;
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
    pub modpackVersionId: Option<String>, // Can be specific version ID or "latest"
    // Otros campos según necesites
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MinecraftInstance {
    pub instanceId: String,
    pub usesDefaultIcon: bool,
    pub iconUrl: Option<String>,
    pub bannerUrl: Option<String>,
    pub instanceName: String,
    pub accountUuid: Option<String>,
    pub minecraftPath: String,
    pub modpackId: Option<String>,
    pub modpackInfo: Option<ModpackInfo>,
    pub minecraftVersion: String,
    pub instanceDirectory: Option<String>,
    pub forgeVersion: Option<String>,
    pub javaPath: Option<String>, // In the future, we automatically download the correct Java version
}

impl MinecraftInstance {
    pub fn is_forge_instance(&self) -> bool {
        self.forgeVersion.is_some()
    }

    pub fn new() -> Self {
        Self {
            instanceId: String::new(),
            usesDefaultIcon: false,
            iconUrl: None,
            bannerUrl: None,
            instanceName: String::new(),
            accountUuid: None,
            minecraftPath: String::new(),
            modpackId: None,
            modpackInfo: None,
            minecraftVersion: String::new(),
            instanceDirectory: None,
            forgeVersion: None,
            javaPath: None,
        }
    }

    pub fn from_instance_id(instance_id: &str) -> Option<Self> {
        // Get the ConfigManager instance from the singleton
        let config_manager_mutex = crate::utils::config_manager::get_config_manager();

        // Lock the mutex to access the ConfigManager
        let config_manager = match config_manager_mutex.lock() {
            Ok(manager) => manager,
            Err(e) => {
                println!("Error locking ConfigManager mutex: {}", e);
                return None;
            }
        };

        // Get the instances directory from ConfigManager
        // Since get_instances_dir() returns PathBuf directly, not Result<PathBuf, Error>
        let instances_dir = config_manager.get_instances_dir();

        println!(
            "Searching for instance {} in directory: {}",
            instance_id,
            instances_dir.display()
        );

        // Try to read the instances directory
        let dir_entries = match fs::read_dir(&instances_dir) {
            Ok(entries) => entries,
            Err(e) => {
                println!("Error reading instances directory: {}", e);
                return None;
            }
        };

        // Iterate through all directories looking for instance.json
        for entry in dir_entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                if path.is_dir() {
                    let config_file = path.join("instance.json");
                    if config_file.exists() {
                        // Try to read and parse the instance.json file
                        if let Ok(content) = fs::read_to_string(&config_file) {
                            if let Ok(mut instance) =
                                serde_json::from_str::<MinecraftInstance>(&content)
                            {
                                // Check if this is the instance we're looking for
                                if instance.instanceId == instance_id {
                                    // Make sure instanceDirectory is set
                                    if instance.instanceDirectory.is_none() {
                                        let native_path_str = path.to_string_lossy().to_string();
                                        let normalized_to_forward_slash =
                                            native_path_str.replace("\\", "/"); // Reemplazar \ con /
                                        instance.instanceDirectory =
                                            Some(normalized_to_forward_slash);
                                    }
                                    println!("Found instance: {}", instance.instanceName);
                                    return Some(instance);
                                }
                            }
                        }
                    }
                }
            }
        }

        println!("No instance found with ID: {}", instance_id);
        None
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
                            let native_path_str = directory.to_string_lossy().to_string();
                            let normalized_to_forward_slash = native_path_str.replace("\\", "/"); // Reemplazar \ con /
                            instance.instanceDirectory = Some(normalized_to_forward_slash);
                        }
                        // Verificamos si la ruta de la instancia es válida
                        if instance.instanceDirectory.is_none() {
                            println!("Instance directory is not set or invalid.");
                            return None;
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

    pub fn set_java_path(&mut self, java_path: PathBuf) {
        self.javaPath = Some(java_path.to_string_lossy().to_string());

        // Guardar la ruta de Java en el archivo de configuración
        self.save().unwrap_or_else(|e| {
            println!("Error saving Java path: {}", e);
        });
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
pub fn get_instances_by_modpack_id(modpack_id: String) -> Vec<MinecraftInstance> {
    /*
        Gets all instances that match the given modpack ID
    */
    let config_manager = crate::utils::config_manager::get_config_manager();
    let instances_dir = config_manager.lock().unwrap().get_instances_dir();

    let mut instances = Vec::new();
    if let Ok(entries) = fs::read_dir(instances_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let config_file = path.join("instance.json");
                if config_file.exists() {
                    if let Ok(content) = fs::read_to_string(&config_file) {
                        if let Ok(instance) = serde_json::from_str::<MinecraftInstance>(&content) {
                            if instance.modpackId == Some(modpack_id.clone()) {
                                instances.push(instance);
                            }
                        }
                    }
                }
            }
        }
    }
    instances
}

#[tauri::command]
pub fn open_game_dir(instance_id: String) -> Result<(), String> {
    println!(
        "[Tauri Command] Opening game directory for instance ID: {}",
        instance_id
    );
    let instance = MinecraftInstance::from_instance_id(&instance_id);
    if let Some(instance) = instance {
        let path = if cfg!(target_os = "windows") {
            PathBuf::from(instance.minecraftPath.replace("/", "\\"))
        } else {
            PathBuf::from(instance.minecraftPath.replace("\\", "/"))
        };
        println!("[Tauri Command] Opening game directory: {}", path.display());
        if path.exists() {
            // Abre el directorio del juego con el programa predeterminado del sistema
            if let Err(e) = tauri_plugin_opener::open_path(path, None::<&str>) {
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
