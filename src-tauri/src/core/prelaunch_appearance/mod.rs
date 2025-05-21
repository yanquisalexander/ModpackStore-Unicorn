use crate::core::instance_manager::get_instance_by_id;
use crate::core::minecraft_instance::MinecraftInstance;
use anyhow::Context;
use serde::{Deserialize, Serialize};
use serde_json::from_slice;
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::io::AsyncReadExt;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogoPosition {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub left: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transform: Option<String>,

    // Captura campos desconocidos
    #[serde(flatten)]
    #[serde(skip_serializing)]
    pub unknown_fields: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Logo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<LogoPosition>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fade_in_duration: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fade_in_delay: Option<String>,

    // Captura campos desconocidos
    #[serde(flatten)]
    #[serde(skip_serializing)]
    pub unknown_fields: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayButton {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hover_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub border_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fade_in_duration: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fade_in_delay: Option<String>,

    // Captura campos desconocidos
    #[serde(flatten)]
    #[serde(skip_serializing)]
    pub unknown_fields: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Background {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video_url: Option<Vec<String>>,

    // Captura campos desconocidos
    #[serde(flatten)]
    #[serde(skip_serializing)]
    pub unknown_fields: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewsPosition {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub right: Option<String>,

    // Captura campos desconocidos
    #[serde(flatten)]
    #[serde(skip_serializing)]
    pub unknown_fields: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewsStyle {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub border_radius: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub padding: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_size: Option<String>,

    // Captura campos desconocidos
    #[serde(flatten)]
    #[serde(skip_serializing)]
    pub unknown_fields: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct News {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<NewsPosition>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<NewsStyle>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entries: Option<Vec<String>>,

    // Captura campos desconocidos
    #[serde(flatten)]
    #[serde(skip_serializing)]
    pub unknown_fields: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreLaunchAppearance {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logo: Option<Logo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub play_button: Option<PlayButton>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<Background>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub news: Option<News>,

    // Captura campos desconocidos
    #[serde(flatten)]
    #[serde(skip_serializing)]
    pub unknown_fields: HashMap<String, serde_json::Value>,
}

use std::io::Result;
use tokio::fs::File;
use tokio::io::BufReader;

// Función auxiliar para registrar campos desconocidos
fn log_unknown_fields(parent_name: &str, unknown_fields: &HashMap<String, serde_json::Value>) {
    if !unknown_fields.is_empty() {
        for field in unknown_fields.keys() {
            log::info!("Unsupported attribute for {}: {}", parent_name, field);
        }
    }
}

#[tauri::command]
pub async fn get_prelaunch_appearance(instance_id: String) -> Option<PreLaunchAppearance> {
    let instance = get_instance_by_id(instance_id.clone()).ok()??; // handles Result and Option
    let instance_dir = instance.instanceDirectory?;

    let prelaunch_appearance_path = PathBuf::from(instance_dir).join("prelaunch_appearance.json");
    log::info!("Prelaunch appearance path: {:?}", prelaunch_appearance_path);
    if tokio::fs::metadata(&prelaunch_appearance_path)
        .await
        .is_err()
    {
        log::error!(
            "Prelaunch appearance file not found for instance: {:?}",
            instance_id.to_string()
        );
        log::error!("Path: {:?}", prelaunch_appearance_path);
        return None;
    }

    let mut file = File::open(&prelaunch_appearance_path).await.ok()?;
    let mut contents = Vec::new();
    file.read_to_end(&mut contents).await.ok()?;

    match serde_json::from_slice::<PreLaunchAppearance>(&contents) {
        Ok(data) => {
            // Registrar los campos desconocidos
            log_unknown_fields("prelaunch_appearance", &data.unknown_fields);

            // Registrar campos desconocidos de los componentes anidados
            if let Some(logo) = &data.logo {
                log_unknown_fields("logo", &logo.unknown_fields);
                if let Some(position) = &logo.position {
                    log_unknown_fields("logo.position", &position.unknown_fields);
                }
            }

            if let Some(play_button) = &data.play_button {
                log_unknown_fields("play_button", &play_button.unknown_fields);
            }

            if let Some(background) = &data.background {
                log_unknown_fields("background", &background.unknown_fields);
            }

            if let Some(news) = &data.news {
                log_unknown_fields("news", &news.unknown_fields);
                if let Some(position) = &news.position {
                    log_unknown_fields("news.position", &position.unknown_fields);
                }
                if let Some(style) = &news.style {
                    log_unknown_fields("news.style", &style.unknown_fields);
                }
            }

            Some(data)
        }
        Err(e) => {
            log::error!("Failed to parse prelaunch_appearance.json: {:?}", e);
            None
        }
    }
}
