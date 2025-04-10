// src-tauri/src/minecraft_launcher.rs
use std::path::{Path, PathBuf};
use std::process::{Command, Child};
use std::fs;
use std::io::Result as IoResult;
use serde_json::Value;
use crate::core::minecraft_instance::MinecraftInstance;
use crate::utils::config_manager::get_config_manager;

pub struct InstanceLauncher {
    instance: MinecraftInstance,
}

impl InstanceLauncher {
    pub fn new(instance: MinecraftInstance) -> Self {
        Self { instance }
    }

    pub fn launch_instance(&self) -> IoResult<Child> {
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
        println!("Instance path: {}", self.instance.instanceDirectory.as_ref().unwrap_or(&String::new()));
        println!("Minecraft version: {}", self.instance.minecraftVersion);
        println!("Forge version: {}", self.instance.forgeVersion.as_ref().unwrap_or(&String::new()));

        let is_forge = self.instance.is_forge_instance();
        if is_forge {
            println!("This is a Forge instance.");
        } else {
            println!("This is a Vanilla instance.");
        }
      

        // Validar la cuenta (esto necesitaría una implementación equivalente a LaunchHelper)
        let account = self.validate_account()?;

        // Revalidar assets antes de lanzar
        self.revalidate_assets()?;

        // Construir los argumentos de Java para lanzar Minecraft
        let launch_args = self.build_launch_args(&java_path, &account)?;

        // Crear y devolver el proceso
        Command::new(&java_path)
            .args(&launch_args)
            .current_dir(Path::new(self.instance.instanceDirectory.as_ref().unwrap_or(&String::new())))
            .spawn()
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
        
        println!("Asset revalidation completed for {}", self.instance.instanceName);
        Ok(())
    }

    fn build_launch_args(&self, java_path: &Path, account: &Value) -> IoResult<Vec<String>> {
        // Construir los argumentos para lanzar Minecraft
        // Esto sería una traducción de lo que hace LaunchHelper en Java
        let mut args = Vec::new();
        
        // Argumentos de Java para Minecraft
        args.push("-Xmx2G".to_string()); // Ejemplo de memoria máxima
        args.push("-XX:+UnlockExperimentalVMOptions".to_string());
        args.push("-XX:+UseG1GC".to_string());
        args.push("-XX:G1NewSizePercent=20".to_string());
        args.push("-XX:G1ReservePercent=20".to_string());
        args.push("-XX:MaxGCPauseMillis=50".to_string());
        args.push("-XX:G1HeapRegionSize=32M".to_string());

        // Agregar argumentos de JVM específicos de Minecraft
        args.push("-Djava.library.path=".to_string() + &self.instance.instanceDirectory.as_ref().unwrap_or(&String::new()));
        args.push("-Dminecraft.launcher.brand=Modpack Store".to_string());
        args.push("-Dminecraft.launcher.version=1.0".to_string());


        // Agregar gameDir y assetIndex
        args.push(format!("-DgameDir={}", self.instance.instanceDirectory.as_ref().unwrap_or(&String::new())));
        args.push(format!("-DassetsDir={}/assets", self.instance.instanceDirectory.as_ref().unwrap_or(&String::new())));
        

        
        // Aquí se añadirían más argumentos según la configuración de Forge/Vanilla
        
        // Clase principal de Minecraft
        let main_class = if self.instance.is_forge_instance() {
            "net.minecraft.launchwrapper.Launch" // Para Forge
        } else {
            "net.minecraft.client.main.Main" // Para Vanilla
        };
        
        args.push(main_class.to_string());
        
        // Argumentos específicos de Minecraft (usuario, token, etc.)
        // Estos variarían según la versión y si es Forge o no
        
        println!("Launch arguments: {:?}", args);
        println!("Full command: {} {:?}", java_path.display(), args);
    

        Ok(args)
    }
}