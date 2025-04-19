// src-tauri/src/core/instance_manager.rs

use crate::core::minecraft_instance;
use crate::core::minecraft_instance::MinecraftInstance;
use crate::core::models::ModpackInfo;
use crate::utils::config_manager::get_config_manager;
use dirs::config_dir;
use serde_json::from_str;
use std::fs;
use std::path::{Path, PathBuf};
use crate::GLOBAL_APP_HANDLE;
use std::sync::Mutex;
use tauri::Emitter;
use crate::core::tasks_manager::{TasksManager, TaskStatus};

#[tauri::command]
pub fn get_all_instances() -> Result<Vec<MinecraftInstance>, String> {
    let instances_dir = get_config_manager().lock().unwrap().get_instances_dir(); // Obtén el directorio de instancias desde la configuración
    get_instances(instances_dir.to_str().unwrap()) // Pasa la ruta como string
}

#[tauri::command]
pub fn get_instance_by_name(instanceName: String) -> Result<Option<MinecraftInstance>, String> {
    let instances_dir = get_config_manager().lock().unwrap().get_instances_dir(); // Obtén el directorio de instancias desde la configuración
    let instances = get_instances(instances_dir.to_str().unwrap())?;
    Ok(instances
        .into_iter()
        .find(|i| i.instanceName == instanceName))
}

#[tauri::command]
pub fn update_instance(instance: MinecraftInstance) -> Result<(), String> {
    let instances_dir = get_config_manager().lock().unwrap().get_instances_dir(); // Obtén el directorio de instancias desde la configuración
    let binding = instance.instanceDirectory.as_ref().unwrap();
    let instance_path = Path::new(&binding);
    let config_file = instance_path.join("instance.json");

    if config_file.exists() {
        let contents = fs::read_to_string(&config_file)
            .map_err(|e| format!("Error reading JSON: {}", e))?;

        let mut existing_instance: MinecraftInstance =
            from_str(&contents).map_err(|e| format!("Error parsing JSON: {}", e))?;

        existing_instance.instanceName = instance.instanceName;

        // Guardar la instancia actualizada
        existing_instance.save();
    }

    Ok(())
}

#[tauri::command]
pub fn get_instance_by_id(instanceId: String) -> Result<Option<MinecraftInstance>, String> {
    let instances_dir = get_config_manager().lock().unwrap().get_instances_dir(); // Obtén el directorio de instancias desde la configuración
    let instances = get_instances(instances_dir.to_str().unwrap())?;
    Ok(instances.into_iter().find(|i| i.instanceId == instanceId))
}

#[tauri::command]
pub fn delete_instance(instance_path: String) -> Result<(), String> {
    let path = Path::new(&instance_path);
    if path.exists() && path.is_dir() {
        fs::remove_dir_all(path).map_err(|e| format!("Failed to delete instance: {}", e))?;
    }
    Ok(())
}

#[tauri::command]
pub fn launch_mc_instance(instance_id: String) -> Result<(), String> {
    let instances_dir = get_config_manager().lock().unwrap().get_instances_dir();

    let instances = get_instances(instances_dir.to_str().unwrap())?;

    let instance = instances
        .into_iter()
        .find(|i| i.instanceId == instance_id)
        .ok_or_else(|| format!("Instance with ID {} not found", instance_id))?;

    instance
        .launch()
        .map_err(|e| format!("Failed to launch instance: {}", e))?;

    Ok(())
}

fn get_instances(instances_dir: &str) -> Result<Vec<MinecraftInstance>, String> {
    let path = Path::new(instances_dir);

    if !path.exists() || !path.is_dir() {
        return Ok(Vec::new());
    }

    let mut instances = Vec::new();

    for entry in fs::read_dir(path).map_err(|e| format!("Error reading directory: {}", e))? {
        let entry = entry.map_err(|e| format!("Error reading entry: {}", e))?;
        let instance_path = entry.path();

        if instance_path.is_dir() {
            let config_file = instance_path.join("instance.json");

            if config_file.exists() {
                let contents = fs::read_to_string(&config_file)
                    .map_err(|e| format!("Error reading JSON: {}", e))?;

                let mut instance: MinecraftInstance =
                    from_str(&contents).map_err(|e| format!("Error parsing JSON: {}", e))?;

                // Normalizamos las rutas usando Path
                instance.instanceDirectory = Some(instance_path.to_string_lossy().to_string());

                // Usamos Path::join para construir rutas de manera segura entre plataformas
                let minecraft_path = instance_path.join("minecraft");
                instance.minecraftPath = minecraft_path.to_string_lossy().to_string();

                // Manejamos los errores al guardar
                if let Err(e) = instance.save() {
                    println!("Warning: Failed to save instance config: {}", e);
                }

                instances.push(instance);
            }
        }
    }

    Ok(instances)
}

#[tauri::command]
pub fn create_local_instance(
    instance_name: String,
    mc_version: String,
    forge_version: Option<String>
) {
    // Creamos una instancia de Minecraft
    let mut instance = MinecraftInstance::new();
    instance.instanceName = instance_name;
    instance.minecraftVersion = mc_version;
    instance.forgeVersion = forge_version;
    instance.instanceId = uuid::Uuid::new_v4().to_string();
    instance.instanceDirectory = Some(format!(
        "{}/{}",
        get_config_manager().lock().unwrap().get_instances_dir().to_str().unwrap(),
        instance.instanceName
    ));
    

    instance.save().map_err(|e| format!("Failed to save instance: {}", e));

    // Aquí hacemos la lógica para descargar lo necesario
    // para crear la instancia local
    // Por ejemplo, descargar assets, librerías, forge (si es necesario), etc.
    // Esto puede incluir la creación de directorios, descarga de archivos, etc.

    // Usamos el TasksManager para ejecutar la tarea de forma asíncrona
    // y permitir que el usuario siga usando la aplicación mientras se descarga.

    let task_manager = TasksManager::new();

    task_manager.add_task(
        &format!("create-instance-{}", instance.instanceName),
        Some(serde_json::json!({
            "instanceName": instance.instanceName.clone(),
            "instanceId": instance.instanceId.clone()
        }))
    );


// Update task to "Creando metadatos"
    // Aquí puedes actualizar el progreso de la tarea
    task_manager.update_task(
        &format!("create-instance-{}", instance.instanceName),
        TaskStatus::Running,
        0.0,
        "Creando metadatos",
        Some(serde_json::json!({
            "instanceName": instance.instanceName.clone(),
            "instanceId": instance.instanceId.clone()
        }))
    );

    // Crear la carpeta de la instancia, y su respectivo instance.json
    let instance_path = PathBuf::from(instance.instanceDirectory.as_ref().unwrap());
    if !instance_path.exists() {
        fs::create_dir_all(&instance_path).map_err(|e| format!("Failed to create instance directory: {}", e));
    }
    let instance_json_path = instance_path.join("instance.json");
    fs::write(&instance_json_path, serde_json::to_string(&instance).unwrap())
        .map_err(|e| format!("Failed to write instance.json: {}", e));

    // Aquí puedes actualizar el progreso de la tarea
    task_manager.update_task(
        &format!("create-instance-{}", instance.instanceName),
        TaskStatus::Running,
        0.5,
        "Descargando archivos",
        Some(serde_json::json!({
            "instanceName": instance.instanceName.clone(),
            "instanceId": instance.instanceId.clone()
        }))
    );

   
    // Por último, imprimimos la instancia creada
    println!("Instance created: {:?}", instance);

    // Finalizamos con fines de demostración
    task_manager.update_task(
        &format!("create-instance-{}", instance.instanceName),
        TaskStatus::Completed,
        1.0,
        "Instancia creada",
        Some(serde_json::json!({
            "instanceName": instance.instanceName.clone(),
            "instanceId": instance.instanceId.clone()
        }))
    );





}