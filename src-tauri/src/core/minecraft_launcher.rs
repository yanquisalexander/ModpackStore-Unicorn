// src-tauri/src/minecraft_launcher.rs
use crate::core::minecraft_instance::MinecraftInstance;
use crate::utils::config_manager::get_config_manager;
use serde_json::Value;
use std::fs;
use std::io::Result as IoResult;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use crate::core::tasks_manager::{TasksManager, TaskStatus, TaskInfo};
use crate::core::minecraft_account::MinecraftAccount;
use crate::core::vanilla_launcher::VanillaLauncher;
use crate::interfaces::game_launcher::GameLauncher;
use tauri::{Emitter};
use crate::GLOBAL_APP_HANDLE;



pub struct InstanceLauncher {
    instance: MinecraftInstance,
}

impl InstanceLauncher {
    pub fn new(instance: MinecraftInstance) -> Self {
        Self { instance }
    }


    pub fn launch_instance(&self) -> IoResult<Child> {
       
    
        // Obtener el ConfigManager
        let config_manager = get_config_manager();
        let java_path = config_manager
            .lock() // Lock the mutex to access inner value
            .expect("Failed to lock config manager mutex") // Handle lock failure
            .get_java_dir() // Call method on inner value
            .join("bin")
            .join(if cfg!(windows) { "java.exe" } else { "java" });
    
        println!("Launching instance: {}", self.instance.instanceName);
        println!("Java path: {}", java_path.display());
        println!(
            "Instance path: {}",
            self.instance
                .instanceDirectory
                .as_ref()
                .unwrap_or(&String::new())
        );
        println!("Minecraft version: {}", self.instance.minecraftVersion);
        println!(
            "Forge version: {}",
            self.instance
                .forgeVersion
                .as_ref()
                .unwrap_or(&String::new())
        );
    
        let is_forge = self.instance.is_forge_instance();
        if is_forge {
            println!("This is a Forge instance.");
        } else {
            println!("This is a Vanilla instance.");
        }
    
        let account = self.validate_account()?;
    
        // Revalidar assets antes de lanzar
        self.revalidate_assets()?;
    
        // Lógica para lanzar la instancia
        let child = if is_forge {
            println!("Launching Forge instance...");
            // TODO: Implement Forge launch logic here
            return Err(std::io::Error::new(std::io::ErrorKind::Other, "Forge launching not implemented yet"));
        } else {
            let launcher = VanillaLauncher::new(self.instance.clone());
            GameLauncher::launch(&launcher).ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::Other, "Failed to launch Minecraft instance")
            })?
        };
    
        // Emitir que la instancia ha sido lanzada
        if let Ok(guard) = GLOBAL_APP_HANDLE.lock() {
            if let Some(app_handle) = guard.as_ref() {
                app_handle.emit("instance-launched", self.instance.instanceId.clone()).unwrap_or_else(|e| {
                    eprintln!("Failed to emit instance-launched event: {}", e);
                });
                println!("Successfully emitted instance-launched event.");
            }
        } else {
            eprintln!("Error: Could not lock GLOBAL_APP_HANDLE mutex for instance-launched.");
        }
    
    
        Ok(child) 
    }
    

    fn validate_account(&self) -> IoResult<Value> {
        // Implementar la validación de la cuenta de Minecraft
        // Esto dependería de cómo manejas las cuentas de Minecraft en tu aplicación
        // Por ahora devolvemos un objeto JSON vacío
        Ok(serde_json::json!({}))
    }

    pub fn revalidate_assets(&self) -> IoResult<()> {
        println!("Revalidating assets for: {}", self.instance.instanceName);

        // Verificar si la versión de Minecraft está disponible
        if self.instance.minecraftVersion.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "No se pudo determinar la versión de Minecraft para revalidar los assets",
            ));
        }

        // Aquí iría la lógica para revalidar assets
        // La implementación completa dependería de la replicación de InstanceBootstrap

        println!(
            "Asset revalidation completed for {}",
            self.instance.instanceName
        );
        Ok(())
    }
   
}
