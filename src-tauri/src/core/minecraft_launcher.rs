// src-tauri/src/minecraft_launcher.rs
use crate::core::minecraft_instance::MinecraftInstance;
use crate::utils::config_manager::get_config_manager;
use serde_json::Value;
use std::fs;
use std::io::{Error as IoError, ErrorKind as IoErrorKind, Result as IoResult};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus}; // Añadir ExitStatus
use std::thread; // Para monitorizar el proceso hijo

// Asegúrate que estas dependencias están en tu Cargo.toml si no lo están
// chrono = { version = "0.4", features = ["serde"] }
// once_cell = "1.17"

// Asumiendo que TasksManager y otros imports siguen siendo necesarios para otras partes
use crate::core::tasks_manager::{TasksManager, TaskStatus, TaskInfo};
use crate::core::minecraft_account::MinecraftAccount;
use crate::core::vanilla_launcher::VanillaLauncher;
use crate::interfaces::game_launcher::GameLauncher;

use tauri::{Emitter, Manager}; // Manager es necesario para emit_all
use crate::GLOBAL_APP_HANDLE;


pub struct InstanceLauncher {
    instance: MinecraftInstance,
}

impl InstanceLauncher {
    pub fn new(instance: MinecraftInstance) -> Self {
        Self { instance }
    }

    // Función de ayuda refactorizada para emitir estado con el payload correcto
    // Usamos emit_all para notificar a todas las ventanas
    fn emit_status(&self, event_name: &str, message: &str) {
        println!(
            "Emitting event: {} for instance: {} with message: {}",
            event_name, self.instance.instanceId, message
        );
        if let Ok(guard) = GLOBAL_APP_HANDLE.lock() {
            if let Some(app_handle) = guard.as_ref() {
                let payload = serde_json::json!({
                    "id": self.instance.instanceId,
                    "name": self.instance.instanceName, // Asegúrate que instanceName no está vacío
                    "message": message
                });
                // Usar emit_all si quieres que todas las ventanas reciban el evento
                if let Err(e) = app_handle.emit(event_name, payload) {
                     eprintln!(
                        "Failed to emit event '{}' for instance {}: {}",
                        event_name, self.instance.instanceId, e
                    );
                }
            } else {
                eprintln!("Error: GLOBAL_APP_HANDLE is None when trying to emit {}.", event_name);
            }
        } else {
            eprintln!("Error: Could not lock GLOBAL_APP_HANDLE mutex for {}.", event_name);
        }
    }

    // Función de ayuda para emitir errores
    fn emit_error(&self, error_message: &str) {
        self.emit_status("instance-error", error_message);
    }

    // Función para monitorizar el proceso hijo en un hilo separado
    fn monitor_process(instance: MinecraftInstance, mut child: Child) {
        // Clonamos datos necesarios para el hilo
        let instance_id = instance.instanceId.clone();
        let instance_name = instance.instanceName.clone();

        // Creamos una instancia temporal de InstanceLauncher solo para usar emit_status/emit_error
        let emitter_launcher = InstanceLauncher { instance }; // Clonamos la instancia original para esto

        thread::spawn(move || {
            println!("Monitoring process for instance: {}", instance_id);
            match child.wait() {
                Ok(exit_status) => {
                    let message = format!(
                        "Minecraft instance '{}' exited with status: {}",
                        instance_name, exit_status
                    );
                    println!("{}", message);
                    emitter_launcher.emit_status("instance-exited", &message);
                }
                Err(e) => {
                    let error_message = format!(
                        "Failed to wait for Minecraft instance '{}' process: {}",
                        instance_name, e
                    );
                    eprintln!("{}", error_message);
                    // Aunque falló el wait, el proceso probablemente terminó, emitimos error y exited
                    emitter_launcher.emit_error(&error_message);
                    emitter_launcher.emit_status("instance-exited", "Minecraft process ended unexpectedly.");
                }
            }
        });
    }


    // --- Métodos principales ---

    pub fn launch_instance(&self) -> IoResult<()> { // Retorna IoResult<()>

        // 1. Emitir estado inicial "preparing"
        self.emit_status("instance-launch-start", "Iniciando la instancia...");

        println!("Launching instance: {}", self.instance.instanceName);

        // --- Validaciones y Preparación ---
        // (Aquí iría tu código para obtener java_path si fuera necesario específicamente aquí)
        // ...

        // Validar cuenta
        if let Err(e) = self.validate_account() {
             let err_msg = format!("Error validating account: {}", e);
             eprintln!("{}", err_msg);
             self.emit_error(&err_msg); // Emitir error antes de retornar
             return Err(e); // Retornar el error original de validación
        }

        // Revalidar assets
        // revalidate_assets debería emitir sus propios eventos de inicio/error internamente
        if let Err(e) = self.revalidate_assets() {
            // No es estrictamente necesario emitir error aquí si revalidate_assets ya lo hace,
            // pero podría ser útil para trazar dónde falló el flujo general.
            let err_msg = format!("Asset revalidation failed: {}", e);
            eprintln!("{}", err_msg);
            // self.emit_error(&err_msg); // Descomentar si quieres emitir error aquí también
            return Err(e); // Retornar el error original de revalidación
        }

        // --- Lógica de Lanzamiento Principal ---
        // El resultado de este bloque if/else determinará el valor de retorno de la función.
        let launch_result = if self.instance.is_forge_instance() {
            // --- Lanzamiento Forge (No implementado) ---
            println!("Launching Forge instance...");
            let err_msg = "Forge launching is not implemented yet";
            self.emit_error(err_msg); // Emitir error específico de Forge
            Err(IoError::new(IoErrorKind::Other, err_msg)) // Retorna Result<(), IoError>::Err

        } else {
            // --- Lanzamiento Vanilla ---
            println!("Launching Vanilla instance...");
            let launcher = VanillaLauncher::new(self.instance.clone());

            // Llamar a GameLauncher::launch y manejar el Option<Child> resultante
            match GameLauncher::launch(&launcher) { // Asumiendo que devuelve Option<Child>
                Some(child_process) => {
                    // Éxito: Se obtuvo el proceso hijo
                    println!(
                        "Minecraft process started successfully (PID: {:?}) for instance {}",
                        child_process.id(),
                        self.instance.instanceId
                    );
                    // 2. Emitir estado "instance-launched" (nombre correcto para el frontend)
                    self.emit_status("instance-launched", "Minecraft está ejecutándose.");
                    // 3. Iniciar monitorización en segundo plano
                    Self::monitor_process(self.instance.clone(), child_process);
                    // El lanzamiento inicial fue exitoso, la monitorización se encargará del resto.
                    Ok(()) // Retorna Result<(), IoError>::Ok
                }
                None => {
                    // Falla: GameLauncher::launch devolvió None
                    let err_msg = "Failed to launch Minecraft instance (GameLauncher returned None)";
                    eprintln!("{}", err_msg);
                    self.emit_error(err_msg); // Emitir error específico de lanzamiento fallido
                    // Crear y devolver un error apropiado
                    Err(IoError::new(IoErrorKind::Other, err_msg)) // Retorna Result<(), IoError>::Err
                }
            }
        }; // Fin del bloque if/else y asignación a launch_result

        // Devolver el resultado final del proceso de lanzamiento (Ok(()) o Err(...))
        launch_result
    }


    fn validate_account(&self) -> IoResult<Value> {
        println!("Validating account for instance: {}", self.instance.instanceId);
        // Implementa tu lógica real aquí
        // Si falla, devuelve Err(IoError::new(...))
        // Ejemplo éxito:
        Ok(serde_json::json!({}))
        // Ejemplo error:
        // Err(IoError::new(IoErrorKind::PermissionDenied, "Invalid credentials"))
    }

    pub fn revalidate_assets(&self) -> IoResult<()> {
        println!("Revalidating assets for: {}", self.instance.instanceName);

        // Emitir evento de inicio de descarga/revalidación
        self.emit_status("instance-downloading-assets", "Revalidando/Descargando assets...");

        // Verificar versión
        if self.instance.minecraftVersion.is_empty() {
            let err_msg = "No se pudo determinar la versión de Minecraft para revalidar los assets";
            self.emit_error(err_msg);
            return Err(IoError::new(IoErrorKind::InvalidData, err_msg));
        }

        // --- Aquí iría tu lógica real de descarga/revalidación ---
        // Simular trabajo
        thread::sleep(std::time::Duration::from_secs(3)); // Simula descarga
        // Si algo falla durante la descarga:
        // let download_error_msg = "Failed to download asset X";
        // self.emit_error(download_error_msg);
        // return Err(IoError::new(IoErrorKind::Other, download_error_msg));


        // Si todo va bien:
        println!(
            "Asset revalidation completed for {}",
            self.instance.instanceName
        );
        // No emitimos un evento específico de "fin de descarga" aquí,
        // porque el siguiente paso lógico es continuar con el lanzamiento.
        // El estado volverá a "preparing" (o directamente a "running" si el lanzamiento es inmediato).
        // Si la descarga fuera un paso *separado* iniciado por el usuario,
        // sí tendría sentido emitir un evento de finalización aquí.
        Ok(())
    }
}