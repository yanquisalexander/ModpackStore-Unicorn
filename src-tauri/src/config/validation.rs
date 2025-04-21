use super::schema::{ConfigValue, ConfigValueType};
use serde_json::Value;
use std::fmt;
use std::path::Path;

/// Errores posibles en la validación
#[derive(Debug, Clone)]
pub enum ValidationError {
    TypeMismatch {
        expected: String,
        got: String,
    },
    ValueOutOfRange {
        min: Option<Value>,
        max: Option<Value>,
        value: Value,
    },
    InvalidChoice {
        value: Value,
        choices: Vec<Value>,
    },
    UnknownKey(String),
    DirectoryNotExists(String),
    DirectoryNotCreatable(String),
    InvalidValidator(String),
    Other(String),
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationError::TypeMismatch { expected, got } => {
                write!(
                    f,
                    "Tipo incorrecto. Se esperaba {}, se recibió {}",
                    expected, got
                )
            }
            ValidationError::ValueOutOfRange { min, max, value } => {
                write!(f, "Valor fuera de rango: {}. ", value)?;
                if let Some(min) = min {
                    write!(f, "Mín: {}. ", min)?;
                }
                if let Some(max) = max {
                    write!(f, "Máx: {}. ", max)?;
                }
                Ok(())
            }
            ValidationError::InvalidChoice { value, choices } => {
                write!(f, "Valor '{}' no permitido. Opciones: {:?}", value, choices)
            }
            ValidationError::UnknownKey(key) => {
                write!(f, "Clave de configuración desconocida: {}", key)
            }
            ValidationError::DirectoryNotExists(path) => {
                write!(f, "El directorio no existe: {}", path)
            }
            ValidationError::DirectoryNotCreatable(path) => {
                write!(f, "No se puede crear el directorio: {}", path)
            }
            ValidationError::InvalidValidator(validator) => {
                write!(f, "Validador desconocido: {}", validator)
            }
            ValidationError::Other(msg) => {
                write!(f, "{}", msg)
            }
        }
    }
}

/// Valida un valor de configuración según su definición
pub fn validate_config_value(
    key: &str,
    value: &Value,
    def: &ConfigValue,
) -> Result<(), ValidationError> {
    // Validar tipo
    validate_type(value, &def.type_)?;

    // Validar rango para números
    if let Some(min) = &def.min {
        if let (Value::Number(min_val), Value::Number(val)) = (min, value) {
            if let (Some(val_f64), Some(min_f64)) = (val.as_f64(), min_val.as_f64()) {
                if val_f64 < min_f64 {
                    return Err(ValidationError::ValueOutOfRange {
                        min: Some(min.clone()),
                        max: None,
                        value: value.clone(),
                    });
                }
            }
        }
    }

    if let Some(max) = &def.max {
        if let (Value::Number(max_val), Value::Number(val)) = (max, value) {
            if let (Some(val_f64), Some(max_f64)) = (val.as_f64(), max_val.as_f64()) {
                if val_f64 > max_f64 {
                    return Err(ValidationError::ValueOutOfRange {
                        min: def.min.clone(),
                        max: Some(max.clone()),
                        value: value.clone(),
                    });
                }
            }
        }
    }

    // Validar opciones para enums
    if def.type_ == ConfigValueType::Enum {
        if let Some(choices) = &def.choices {
            if !choices.contains(value) {
                return Err(ValidationError::InvalidChoice {
                    value: value.clone(),
                    choices: choices.clone(),
                });
            }
        }
    }

    // Ejecutar validador personalizado si existe
    if let Some(validator) = &def.validator {
        match validator.as_str() {
            "directory_exists" => validate_directory_exists(value)?,
            "directory_exists_or_creatable" => validate_directory_exists_or_creatable(value)?,
            _ => return Err(ValidationError::InvalidValidator(validator.clone())),
        }
    }

    Ok(())
}

/// Valida que el tipo de valor corresponda al tipo esperado
fn validate_type(value: &Value, expected_type: &ConfigValueType) -> Result<(), ValidationError> {
    let valid = match expected_type {
        ConfigValueType::String => value.is_string(),
        ConfigValueType::Integer => value.is_i64(),
        ConfigValueType::Float => value.is_f64(),
        ConfigValueType::Boolean => value.is_boolean(),
        ConfigValueType::Path => value.is_string(),
        ConfigValueType::Enum => true, // Se valida por separado con choices
        ConfigValueType::List => value.is_array(),
    };

    if !valid {
        Err(ValidationError::TypeMismatch {
            expected: format!("{:?}", expected_type),
            got: match value {
                Value::Null => "null".to_string(),
                Value::Bool(_) => "boolean".to_string(),
                Value::Number(_) => "number".to_string(),
                Value::String(_) => "string".to_string(),
                Value::Array(_) => "array".to_string(),
                Value::Object(_) => "object".to_string(),
            },
        })
    } else {
        Ok(())
    }
}

/// Validador para directorio existente
fn validate_directory_exists(value: &Value) -> Result<(), ValidationError> {
    if let Value::String(path_str) = value {
        let path = expand_path(path_str);
        if !path.exists() || !path.is_dir() {
            return Err(ValidationError::DirectoryNotExists(path_str.clone()));
        }
    }
    Ok(())
}

/// Validador para directorio existente o que se pueda crear
fn validate_directory_exists_or_creatable(value: &Value) -> Result<(), ValidationError> {
    if let Value::String(path_str) = value {
        let path = expand_path(path_str);

        // Si ya existe, perfecto
        if path.exists() && path.is_dir() {
            return Ok(());
        }

        // Si no existe, verificar si se puede crear
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                return Err(ValidationError::DirectoryNotCreatable(path_str.clone()));
            }
        }
    }
    Ok(())
}

/// Expande una ruta con variables de entorno y ~
fn expand_path(path: &str) -> std::path::PathBuf {
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

    std::path::PathBuf::from(result)
}
