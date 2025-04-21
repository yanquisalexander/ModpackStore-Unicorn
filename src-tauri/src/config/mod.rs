pub mod schema;
pub mod validation;

use once_cell::sync::OnceCell;
use schema::{ConfigSchema, ConfigValue, ConfigValueType};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    fs::{self, create_dir_all, read_to_string, write},
    path::{Path, PathBuf},
    sync::Mutex,
};
use validation::{validate_config_value, ValidationError};

/// Gestor central de configuración
#[derive(Debug)]
pub struct ConfigManager {
    config_path: PathBuf,
    schema: ConfigSchema,
    values: HashMap<String, Value>,
}

impl ConfigManager {
    /// Crea una nueva instancia del gestor de configuración
    fn new() -> Result<Self, String> {
        // Cargar el esquema de configuración
        let schema = ConfigSchema::load_from_embedded()?;

        // Determinar la ruta del archivo de configuración
        let config_path = dirs::config_dir()
            .ok_or_else(|| "No se pudo obtener el directorio de configuración".to_string())?
            .join("dev.alexitoo.modpackstore")
            .join("config.json");

        // Asegurar que el directorio existe
        if let Some(parent) = config_path.parent() {
            create_dir_all(parent).map_err(|e| format!("Error al crear directorio: {}", e))?;
        }

        // Cargar o crear la configuración
        let values = if config_path.exists() {
            let content = read_to_string(&config_path)
                .map_err(|e| format!("Error al leer configuración: {}", e))?;
            serde_json::from_str(&content).unwrap_or_else(|_| json!({}))
        } else {
            // Si no existe el archivo, creamos uno con valores predeterminados
            let default_values = schema.get_default_values();
            let json_values = json!(default_values);

            // Guardar el nuevo archivo
            write(
                &config_path,
                serde_json::to_string_pretty(&json_values).unwrap(),
            )
            .map_err(|e| format!("Error al crear configuración predeterminada: {}", e))?;

            json_values
        };

        Ok(Self {
            config_path,
            schema,
            values: extract_values_map(values),
        })
    }

    /// Guarda la configuración actual en disco
    pub fn save(&self) -> Result<(), String> {
        let json_values = json!(self.values);
        let json_content = serde_json::to_string_pretty(&json_values)
            .map_err(|e| format!("Error al serializar configuración: {}", e))?;

        write(&self.config_path, json_content)
            .map_err(|e| format!("Error al guardar configuración: {}", e))?;

        Ok(())
    }

    /// Establece un valor de configuración, validándolo contra el esquema
    pub fn set<T: serde::Serialize>(&mut self, key: &str, value: T) -> Result<(), ValidationError> {
        // Convertir el valor a serde_json::Value para procesarlo
        let value_json = json!(value);

        // Validar el valor contra el esquema
        if let Some(config_def) = self.schema.get_config_definition(key) {
            validate_config_value(key, &value_json, config_def)?;

            // Si la validación pasa, actualizar el valor
            self.values.insert(key.to_string(), value_json);
            Ok(())
        } else {
            Err(ValidationError::UnknownKey(key.to_string()))
        }
    }

    /// Obtiene un valor de configuración genérico
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.values.get(key)
    }

    /// Obtiene un valor de configuración con un tipo específico
    pub fn get_typed<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Option<T> {
        self.values
            .get(key)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    /// Obtiene una representación JSON de toda la configuración
    pub fn get_all_json(&self) -> Value {
        json!(self.values)
    }

    /// Obtiene el esquema de configuración
    pub fn get_schema(&self) -> &ConfigSchema {
        &self.schema
    }

    /// Métodos de conveniencia para valores específicos

    /// Obtiene el directorio de instancias
    pub fn get_instances_dir(&self) -> PathBuf {
        let default = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("ModpackStore")
            .join("Instances");

        self.get("instancesDir")
            .and_then(Value::as_str)
            .map(expand_path)
            .unwrap_or(default)
    }

    /// Obtiene el directorio de Java
    pub fn get_java_dir(&self) -> Option<PathBuf> {
        let default = std::env::var("JAVA_HOME").unwrap_or_else(|_| "java".to_string());

        self.get("javaDir")
            .and_then(Value::as_str)
            .map(expand_path)
            .or_else(|| Some(PathBuf::from(default))) // Changed to return Option<PathBuf>
    }

    /// Comprobar si se deben verificar actualizaciones al inicio
    pub fn check_updates_on_startup(&self) -> bool {
        self.get("checkUpdatesOnStartup")
            .and_then(Value::as_bool)
            .unwrap_or(true)
    }

    /// Comprobar si se debe cerrar el launcher al iniciar Minecraft
    pub fn get_close_on_launch(&self) -> bool {
        self.get("closeOnLaunch")
            .and_then(Value::as_bool)
            .unwrap_or(false)
    }
}

// Convierte un Value en un HashMap
fn extract_values_map(value: Value) -> HashMap<String, Value> {
    match value {
        Value::Object(map) => map.into_iter().collect(),
        _ => HashMap::new(),
    }
}

// Expande una ruta con variables de entorno y ~
fn expand_path(path: &str) -> PathBuf {
    let mut result = path.to_string();

    // Reemplazar ~ con la ruta del home
    if result.starts_with("~") {
        if let Some(home) = dirs::home_dir() {
            result = result.replacen("~", home.to_str().unwrap_or(""), 1);
        }
    }

    // Reemplazar variables de entorno
    if result.contains("$") {
        for (key, value) in std::env::vars() {
            result = result.replace(&format!("${}", key), &value);
        }
    }

    PathBuf::from(result)
}

// Singleton para acceder globalmente al ConfigManager
static INSTANCE: OnceCell<Mutex<Result<ConfigManager, String>>> = OnceCell::new();

/// Obtiene el singleton de ConfigManager
pub fn get_config_manager() -> &'static Mutex<Result<ConfigManager, String>> {
    INSTANCE.get_or_init(|| Mutex::new(ConfigManager::new()))
}

// Comandos para la API de Tauri

#[tauri::command]
pub fn get_config() -> Result<Value, String> {
    match get_config_manager().lock() {
        Ok(config_result) => match &*config_result {
            Ok(config) => Ok(config.get_all_json()),
            Err(e) => Err(e.clone()),
        },
        Err(_) => Err("Error al obtener el bloqueo del gestor de configuración".to_string()),
    }
}

#[tauri::command]
pub fn set_config(key: String, value: Value) -> Result<(), String> {
    match get_config_manager().lock() {
        Ok(mut config_result) => match &mut *config_result {
            Ok(config) => match config.set(&key, value) {
                Ok(_) => {
                    if let Err(e) = config.save() {
                        return Err(format!("Error al guardar la configuración: {}", e));
                    }
                    Ok(())
                }
                Err(e) => Err(format!("Error de validación: {}", e)),
            },
            Err(e) => Err(e.clone()),
        },
        Err(_) => Err("Error al obtener el bloqueo del gestor de configuración".to_string()),
    }
}

#[tauri::command]
pub fn get_schema() -> Result<Value, String> {
    match get_config_manager().lock() {
        Ok(config_result) => match &*config_result {
            Ok(config) => Ok(json!(config.get_schema())),
            Err(e) => Err(e.clone()),
        },
        Err(_) => Err("Error al obtener el bloqueo del gestor de configuración".to_string()),
    }
}
