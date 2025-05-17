use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Define los posibles tipos de valores de configuración
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ConfigValueType {
    String,
    Integer,
    Float,
    Boolean,
    Path,
    Enum,
    List,
}

/// Define una entrada de configuración
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigValue {
    #[serde(rename = "type")]
    pub type_: ConfigValueType,
    pub default: Value,
    pub description: String,
    #[serde(default)]
    pub ui_section: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub choices: Option<Vec<Value>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub validator: Option<String>,
}

/// Define el esquema completo de configuración
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigSchema {
    #[serde(flatten)]
    pub definitions: HashMap<String, ConfigValue>,
}

impl ConfigSchema {
    /// Carga el esquema desde un archivo YAML incrustado
    pub fn load_from_embedded() -> Result<Self, String> {
        // El esquema está incrustado en el binario para simplificar la distribución
        const CONFIG_SCHEMA_YAML: &str = include_str!("../../resources/config_schema.yml");

        serde_yaml::from_str(CONFIG_SCHEMA_YAML)
            .map_err(|e| format!("Error al cargar el esquema de configuración: {}", e))
    }

    /// Genera un mapa de valores predeterminados según el esquema
    pub fn get_default_values(&self) -> HashMap<String, Value> {
        let mut defaults = HashMap::new();

        for (key, def) in &self.definitions {
            defaults.insert(key.clone(), process_default_value(&def.default));
        }

        defaults
    }

    /// Obtiene la definición de una clave de configuración
    pub fn get_config_definition(&self, key: &str) -> Option<&ConfigValue> {
        self.definitions.get(key)
    }

    /// Obtiene todas las definiciones para una sección de UI específica
    pub fn get_definitions_by_section(&self, section: &str) -> Vec<(&String, &ConfigValue)> {
        self.definitions
            .iter()
            .filter(|(_, def)| def.ui_section == section)
            .collect()
    }

    /// Obtiene todas las secciones de UI disponibles
    pub fn get_ui_sections(&self) -> Vec<String> {
        let mut sections = self
            .definitions
            .iter()
            .filter_map(|(_, def)| {
                if def.ui_section != "internal" {
                    Some(def.ui_section.clone())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        sections.sort();
        sections.dedup();
        sections
    }
}

/// Procesa y normaliza valores de configuración, especialmente rutas
fn process_default_value(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::String(s) => {
            if s.starts_with('$') {
                // Reemplaza variables de entorno
                let var_name = &s[1..];
                match std::env::var(var_name) {
                    Ok(val) => json!(val),
                    Err(_) => {
                        eprintln!(
                            "Advertencia: la variable de entorno {} no está definida.",
                            var_name
                        );
                        value.clone()
                    }
                }
            } else if s.starts_with('~') {
                // Reemplaza el directorio de inicio del usuario y normaliza la ruta
                if let Some(home) = dirs::home_dir() {
                    let path_str = s.replacen("~", home.to_str().unwrap_or(""), 1);
                    // Convertir a PathBuf para normalizar según el OS
                    let path = PathBuf::from(path_str);
                    // Convertir de vuelta a String para JSON
                    if let Some(normalized_path) = path.to_str() {
                        json!(normalized_path)
                    } else {
                        eprintln!("Advertencia: no se pudo convertir la ruta a texto.");
                        value.clone()
                    }
                } else {
                    eprintln!("Advertencia: no se pudo determinar el directorio home.");
                    value.clone()
                }
            } else if value_is_likely_path(s) {
                // Normaliza otras rutas que no comienzan con ~ o $
                let path = PathBuf::from(s);
                if let Some(normalized_path) = path.to_str() {
                    json!(normalized_path)
                } else {
                    value.clone()
                }
            } else {
                value.clone()
            }
        }
        _ => value.clone(),
    }
}

/// Determina si un string probablemente representa una ruta
fn value_is_likely_path(s: &str) -> bool {
    // Comprueba si contiene separadores de ruta comunes
    s.contains('/') || 
    s.contains('\\') || 
    // Comprueba si comienza con una letra de unidad en Windows (e.g., "C:")
    (s.len() >= 2 && s.chars().nth(1) == Some(':')) ||
    // Comprueba si es una ruta absoluta
    Path::new(s).is_absolute()
}

/// Normaliza una ruta según el sistema operativo
pub fn normalize_path(path_str: &str) -> String {
    let path = if path_str.starts_with('~') {
        if let Some(home) = dirs::home_dir() {
            let home_str = home.to_str().unwrap_or("");
            PathBuf::from(path_str.replacen("~", home_str, 1))
        } else {
            PathBuf::from(path_str)
        }
    } else {
        PathBuf::from(path_str)
    };
    
    path.to_string_lossy().to_string()
}

// Extension trait para PathBuf para facilitar la conversión a String
trait PathBufExt {
    fn to_string(&self) -> String;
}

impl PathBufExt for PathBuf {
    fn to_string(&self) -> String {
        self.to_string_lossy().to_string()
    }
}