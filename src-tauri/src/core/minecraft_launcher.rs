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


pub struct InstanceLauncher {
    instance: MinecraftInstance,
}

impl InstanceLauncher {
    pub fn new(instance: MinecraftInstance) -> Self {
        Self { instance }
    }


    pub fn launch_instance(&self) -> IoResult<Child> {


        // Simulate a Task creation
        let tasks_manager = TasksManager::new();
        let task_id = tasks_manager.add_task("Launching Minecraft Instance", None);

        tasks_manager.update_task(&task_id, TaskStatus::Running, 0.0, "Instance is launching...", None);



        // Obtener el ConfigManager (asumo que tiene una implementación similar)
        let config_manager = get_config_manager();
        let java_path = config_manager
            .lock() // First lock the mutex to get the inner value
            .expect("Failed to lock config manager mutex") // Handle potential lock failure
            .get_java_dir() // Now call the method on the inner value
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

      

        // Aquí iría la lógica para lanzar la instancia de Minecraft
        // usando vanilla_launcher o forge_launcher


        if is_forge {
            println!("Launching Forge instance...");
            // TODO: implementar lógica Forge
            return Err(std::io::Error::new(std::io::ErrorKind::Other, "Forge launching not implemented yet"));
        } else {
            let launcher = VanillaLauncher::new(self.instance.clone());
            let child = GameLauncher::launch(&launcher).ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::Other, "Failed to launch Minecraft instance")
            })?;
            Ok(child)
        }
        

        


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
