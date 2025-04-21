use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;

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
            .map(|(_, def)| def.ui_section.clone())
            .collect::<Vec<_>>();

        sections.sort();
        sections.dedup();
        sections
    }
}

fn process_default_value(value: &serde_json::Value) -> serde_json::Value {
    use serde_json::json;

    match value {
        serde_json::Value::String(s) => {
            if s.starts_with('$') {
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
                if let Some(home) = dirs::home_dir() {
                    let home_str = home.to_str().unwrap_or("");
                    let expanded = s.replacen("~", home_str, 1);
                    json!(expanded)
                } else {
                    eprintln!("Advertencia: no se pudo determinar el directorio home.");
                    value.clone()
                }
            } else {
                value.clone()
            }
        }
        _ => value.clone(),
    }
}
