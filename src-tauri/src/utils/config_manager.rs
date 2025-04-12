use dirs::config_dir;
use once_cell::sync::OnceCell;
use serde_json::{json, Value};
use std::{
    fs::{create_dir_all, read_to_string, write},
    path::{Path, PathBuf},
    sync::Mutex,
};

#[derive(Debug)]
pub struct ConfigManager {
    config_path: PathBuf,
    content: Value,
}

impl ConfigManager {
    fn new() -> Self {
        let config_file = config_dir()
            .expect("No se pudo obtener el directorio de configuración")
            .join("dev.alexitoo.modpackstore")
            .join("config.json");
        let content = if config_file.exists() {
            let file_content = read_to_string(&config_file).unwrap_or_else(|_| "{}".to_string());
            serde_json::from_str(&file_content).unwrap_or(json!({}))
        } else {
            let default = json!({});
            write(
                &config_file,
                serde_json::to_string_pretty(&default).unwrap(),
            )
            .unwrap();
            default
        };

        Self {
            config_path: config_file,
            content,
        }
    }

    pub fn load(&mut self) {
        if let Ok(file_content) = read_to_string(&self.config_path) {
            self.content = serde_json::from_str(&file_content).unwrap_or(json!({}));
        }
    }

    pub fn save(&self) {
        let json = serde_json::to_string_pretty(&self.content).unwrap();
        write(&self.config_path, json).expect("No se pudo guardar la configuración");
    }

    pub fn set_config(&mut self, new_content: Value) {
        self.content = new_content;
    }

    pub fn get_config(&self) -> &Value {
        &self.content
    }

    // Métodos específicos
    pub fn get_instances_dir(&self) -> PathBuf {
        let default = dirs::home_dir()
            .unwrap()
            .join("ModpackStore")
            .join("Instances");
        PathBuf::from(
            self.content
                .get("instancesDir")
                .and_then(Value::as_str)
                .unwrap_or(default.to_str().unwrap()),
        )
    }

    pub fn get_java_dir(&self) -> PathBuf {
        let default = std::env::var("JAVA_HOME").unwrap_or_else(|_| String::from("java"));
        PathBuf::from(
            self.content
                .get("javaDir")
                .and_then(Value::as_str)
                .unwrap_or(&default),
        )
    }

    pub fn check_updates_on_startup(&self) -> bool {
        self.content
            .get("checkUpdatesOnStartup")
            .and_then(Value::as_bool)
            .unwrap_or(true)
    }

    pub fn get_close_on_launch(&self) -> bool {
        self.content
            .get("closeOnLaunch")
            .and_then(Value::as_bool)
            .unwrap_or(false)
    }
}

// Singleton
static INSTANCE: OnceCell<Mutex<ConfigManager>> = OnceCell::new();

pub fn get_config_manager() -> &'static Mutex<ConfigManager> {
    INSTANCE.get_or_init(|| Mutex::new(ConfigManager::new()))
}

#[tauri::command]
pub fn get_config() -> serde_json::Value {
    get_config_manager().lock().unwrap().content.clone()
}
