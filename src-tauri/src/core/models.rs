// src-tauri/src/core/models.rs
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ModpackInfo {
    pub name: Option<String>,
    pub version: Option<String>,
    pub author: Option<String>,
    pub modpackVersionId: Option<String>, // Can be specific version ID or "latest"
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MinecraftInstance {
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
