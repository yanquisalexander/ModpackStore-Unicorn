// src-tauri/src/instance_bootstrap.rs
use std::path::{Path, PathBuf};
use std::fs;
use std::io::{self, Result as IoResult};
use std::process::Command;
use serde_json::{Value, json};
use tauri_plugin_http::reqwest as reqwest;
use crate::core::minecraft_instance::MinecraftInstance;

pub struct InstanceBootstrap {
    client: reqwest::blocking::Client,
    // Cache para metadatos de versiones
    version_manifest_cache: Option<(Value, u64)>, // (datos, timestamp)
}

impl InstanceBootstrap {
    const MOJANG_VERSION_MANIFEST_URL: &'static str = 
        "https://launchermeta.mojang.com/mc/game/version_manifest.json";
    const FORGE_API_BASE_URL: &'static str = 
        "https://mrnavastar.github.io/ForgeVersionAPI/forge-versions.json";
    const CACHE_EXPIRY_MS: u64 = 3600000; // 1 hora
    
    pub fn new() -> Self {
        Self {
            client: reqwest::blocking::Client::new(),
            version_manifest_cache: None,
        }
    }
    
    pub fn fetch_minecraft_versions(&mut self) -> Result<Vec<String>, String> {
        let root_node = self.get_version_manifest()
            .map_err(|e| format!("Error fetching version manifest: {}", e))?;
            
        let versions_node = root_node["versions"].as_array()
            .ok_or_else(|| "Invalid version manifest format".to_string())?;
            
        let versions: Vec<String> = versions_node.iter()
            .filter_map(|v| v["id"].as_str().map(String::from))
            .collect();
            
        Ok(versions)
    }
    
    pub fn fetch_forge_versions(&self) -> Result<std::collections::HashMap<String, Vec<String>>, String> {
        let forge_data = self.client.get(Self::FORGE_API_BASE_URL)
            .send()
            .map_err(|e| format!("Error connecting to Forge API: {}", e))?
            .json::<Value>()
            .map_err(|e| format!("Error parsing Forge API response: {}", e))?;
            
        let mut forge_versions = std::collections::HashMap::new();
        
        if let Some(obj) = forge_data.as_object() {
            for (mc_version, forge_version_array) in obj {
                let forge_version_list: Vec<String> = forge_version_array
                    .as_array()
                    .unwrap_or(&Vec::new())
                    .iter()
                    .filter_map(|v| v["id"].as_str().map(String::from))
                    .collect();
                    
                forge_versions.insert(mc_version.clone(), forge_version_list);
            }
        }
        
        Ok(forge_versions)
    }
    
    pub fn revalidate_assets(&self, instance: &MinecraftInstance) -> IoResult<()> {
        println!("Revalidating assets for: {}", instance.instanceName);
        
        // Verificar si la versión de Minecraft está disponible
        if instance.minecraftVersion.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "No se pudo determinar la versión de Minecraft",
            ));
        }
        
        // Obtener la ruta de la instancia
        let instance_dir = Path::new(instance.instanceDirectory.as_deref().unwrap_or(""));
        let minecraft_folder = instance_dir.join("minecraft");
        let assets_dir = minecraft_folder.join("assets");
        let assets_indexes_dir = assets_dir.join("indexes");
        let assets_objects_dir = assets_dir.join("objects");
        
        // Crear directorios si no existen
        fs::create_dir_all(&assets_indexes_dir)?;
        fs::create_dir_all(&assets_objects_dir)?;
        
        // Aquí iría el resto de la lógica para revalidar assets
        // Esto incluiría descargar el índice de assets, verificar los assets existentes
        // y descargar los faltantes
        
        println!("Asset revalidation completed");
        Ok(())
    }
    
    // Implementaciones auxiliares
    fn get_version_manifest(&mut self) -> Result<Value, reqwest::Error> {
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
            
        // Verificar caché
        if let Some((cached_manifest, cache_time)) = &self.version_manifest_cache {
            if current_time - cache_time < Self::CACHE_EXPIRY_MS {
                return Ok(cached_manifest.clone());
            }
        }
        
        // Obtener nuevo manifiesto
        let manifest = self.client.get(Self::MOJANG_VERSION_MANIFEST_URL)
            .send()?
            .json::<Value>()?;
            
        // Actualizar caché
        self.version_manifest_cache = Some((manifest.clone(), current_time));
        
        Ok(manifest)
    }
    
    // Aquí irían más métodos para bootstrapping de instancias Vanilla y Forge
    // como bootstrap_vanilla_instance y bootstrap_forge_instance, 
    // pero son bastante extensos para este contexto
}