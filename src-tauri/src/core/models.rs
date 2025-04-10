// src-tauri/src/core/models.rs
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ModpackInfo {
    pub name: String,
    pub version: String,
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
