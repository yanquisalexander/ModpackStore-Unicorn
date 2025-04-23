// src-tauri/src/core/instance_manager.rs

use crate::config::get_config_manager;
use crate::core::instance_bootstrap::InstanceBootstrap;
use crate::core::minecraft_instance;
use crate::core::minecraft_instance::MinecraftInstance;
use crate::core::models::ModpackInfo;
use crate::core::tasks_manager::{TaskStatus, TasksManager};
use crate::GLOBAL_APP_HANDLE;
use dirs::config_dir;
use serde_json::from_str;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::Mutex;
use tauri::Emitter;

#[tauri::command]
pub fn get_all_instances() -> Result<Vec<MinecraftInstance>, String> {
    let config_manager = get_config_manager()
        .lock()
        .map_err(|_| "Failed to lock config manager mutex".to_string())?;

    let config = config_manager.as_ref().map_err(|e| e.clone())?;

    let instances_dir = config.get_instances_dir();
    get_instances(instances_dir.to_str().unwrap_or_default())
}

#[tauri::command]
pub fn get_instance_by_name(instance_name: String) -> Result<Option<MinecraftInstance>, String> {
    let config_manager = get_config_manager()
        .lock()
        .map_err(|_| "Failed to lock config manager mutex".to_string())?;

    let config = config_manager.as_ref().map_err(|e| e.clone())?;

    let instances_dir = config.get_instances_dir();

    let instances = get_instances(instances_dir.to_str().unwrap_or_default())?;
    Ok(instances
        .into_iter()
        .find(|i| i.instanceName == instance_name))
}

#[tauri::command]
pub fn update_instance(instance: MinecraftInstance) -> Result<(), String> {
    let config_manager = get_config_manager()
        .lock()
        .map_err(|_| "Failed to lock config manager mutex".to_string())?;

    let config = config_manager.as_ref().map_err(|e| e.clone())?;

    let instances_dir = config.get_instances_dir();

    let instances = get_instances(instances_dir.to_str().unwrap_or_default())?;
    let original_instance = instances
        .into_iter()
        .find(|i| i.instanceId == instance.instanceId)
        .ok_or_else(|| format!("Instance with ID {} not found", instance.instanceId))?;

    let instance_path = match &original_instance.instanceDirectory {
        Some(dir) => Path::new(dir),
        None => return Err("Instance directory is missing".to_string()),
    };

    let config_file = instance_path.join("instance.json");

    if config_file.exists() {
        let contents =
            fs::read_to_string(&config_file).map_err(|e| format!("Error reading JSON: {}", e))?;

        let mut existing_instance: MinecraftInstance =
            from_str(&contents).map_err(|e| format!("Error parsing JSON: {}", e))?;

        existing_instance.instanceName = instance.instanceName;
        existing_instance.accountUuid = instance.accountUuid;

        // Guardar la instancia actualizada
        existing_instance
            .save()
            .map_err(|e| format!("Error saving instance: {}", e))?;
    }

    Ok(())
}

#[tauri::command]
pub fn get_instance_by_id(instance_id: String) -> Result<Option<MinecraftInstance>, String> {
    let config_manager = get_config_manager()
        .lock()
        .map_err(|_| "Failed to lock config manager mutex".to_string())?;

    let config = config_manager.as_ref().map_err(|e| e.clone())?;

    let instances_dir = config.get_instances_dir();

    let instances = get_instances(instances_dir.to_str().unwrap_or_default())?;
    Ok(instances.into_iter().find(|i| i.instanceId == instance_id))
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
    let config_manager = get_config_manager()
        .lock()
        .map_err(|_| "Failed to lock config manager mutex".to_string())?;

    let config = config_manager.as_ref().map_err(|e| e.clone())?;

    let instances_dir = config.get_instances_dir();

    let instances = get_instances(instances_dir.to_str().unwrap_or_default())?;

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
pub async fn create_local_instance(
    instance_name: String,
    mc_version: String,
    forge_version: Option<String>,
) -> Result<String, String> {
    // Obtener el directorio de instancias
    let instances_dir = {
        let config_manager = get_config_manager()
            .lock()
            .map_err(|_| "Failed to lock config manager mutex".to_string())?;

        let config = config_manager.as_ref().map_err(|e| e.clone())?;

        config.get_instances_dir()
    };

    // Creamos una instancia de Minecraft
    let mut instance = MinecraftInstance::new();
    instance.instanceName = instance_name.clone();
    instance.minecraftVersion = mc_version;
    instance.forgeVersion = forge_version.clone();
    instance.instanceId = uuid::Uuid::new_v4().to_string();

    let instance_dir = instances_dir.join(&instance.instanceName);
    instance.minecraftPath = instance_dir.join("minecraft").to_string_lossy().to_string();

    // If the instance directory doesn't exist, create it
    if !instance_dir.exists() {
        fs::create_dir_all(&instance_dir)
            .map_err(|e| format!("Failed to create instance directory: {}", e))?;
    }
    // Set the instance directory
    instance.instanceDirectory = Some(instance_dir.to_string_lossy().to_string());

    // Guardamos la instancia inicialmente
    instance
        .save()
        .map_err(|e| format!("Failed to save instance: {}", e))?;

    // Creamos el task manager y lo envolvemos en Arc<Mutex<>> para compartirlo entre hilos
    let task_manager = Arc::new(Mutex::new(TasksManager::new()));
    let task_id = {
        let mut tm = task_manager.lock().unwrap();
        tm.add_task(
            &format!("Creando instancia {}", instance.instanceName),
            Some(serde_json::json!({
                "instanceName": instance.instanceName.clone(),
                "instanceId": instance.instanceId.clone()
            })),
        )
    };

    // Actualizamos el estado a "Creando metadatos"
    {
        let mut tm = task_manager.lock().unwrap();
        tm.update_task(
            &task_id,
            TaskStatus::Running,
            10.0,
            "Creando metadatos",
            Some(serde_json::json!({
                "instanceName": instance.instanceName.clone(),
                "instanceId": instance.instanceId.clone()
            })),
        );
    }

    // Crear la carpeta de la instancia y su respectivo instance.json
    let instance_path = PathBuf::from(instance.instanceDirectory.as_ref().unwrap());
    if !instance_path.exists() {
        fs::create_dir_all(&instance_path)
            .map_err(|e| format!("Failed to create instance directory: {}", e))?;
    }
    let instance_json_path = instance_path.join("instance.json");
    fs::write(
        &instance_json_path,
        serde_json::to_string(&instance).unwrap(),
    )
    .map_err(|e| format!("Failed to write instance.json: {}", e))?;

    // Clone los datos necesarios para el hilo
    let instance_clone = instance.clone();
    let task_id_clone = task_id.clone();
    let task_manager_clone = Arc::clone(&task_manager);

    // Lanzar el proceso en segundo plano
    std::thread::spawn(move || {
        // Iniciar el bootstrap de la instancia
        let mut bootstrap = InstanceBootstrap::new();

        // Determinar si es una instancia vanilla o forge
        let result = if instance_clone.forgeVersion.is_some() {
            // Si tiene forge version, usar el método para instancias forge
            bootstrap.bootstrap_forge_instance(
                &instance_clone,
                Some(task_id_clone.clone()),
                Some(Arc::clone(&task_manager_clone)),
            )
        } else {
            // Si no tiene forge version, usar el método para instancias vanilla
            bootstrap.bootstrap_vanilla_instance(
                &instance_clone,
                Some(task_id_clone.clone()),
                Some(Arc::clone(&task_manager_clone)),
            )
        };

        match result {
            Ok(_) => {
                // Emit task completion event
                if let Ok(mut tm) = task_manager_clone.lock() {
                    tm.update_task(
                        &task_id_clone,
                        TaskStatus::Completed,
                        100.0,
                        &format!("Instancia {} creada", instance_clone.instanceName),
                        Some(serde_json::json!({
                            "instanceName": instance_clone.instanceName.clone(),
                            "instanceId": instance_clone.instanceId.clone()
                        })),
                    );
                }

                println!("Instance creation completed: {:?}", instance_clone);
            }
            Err(e) => {
                eprintln!("Error during bootstrap: {}", e);
                // Actualizar el estado de la tarea a fallido
                if let Ok(mut tm) = task_manager_clone.lock() {
                    tm.update_task(
                        &task_id_clone,
                        TaskStatus::Failed,
                        0.0,
                        &format!("Error en bootstrap: {}", e),
                        Some(serde_json::json!({
                            "instanceName": instance_clone.instanceName.clone(),
                            "instanceId": instance_clone.instanceId.clone(),
                            "error": e
                        })),
                    );
                }
            }
        }

        // Remover la tarea eventualmente (Quizá después de 2 minutos)
        std::thread::sleep(std::time::Duration::from_secs(120));
        if let Ok(mut tm) = task_manager_clone.lock() {
            tm.remove_task(&task_id_clone);
            println!("Task removed: {:?}", task_id_clone);
        }
    });

    // Devolvemos inmediatamente una respuesta con el ID de la instancia
    Ok(instance.instanceId)
}


#[tauri::command]
// Returns bool
pub async fn remove_instance(instance_id: String) -> Result<bool, String> {
    // Obtener la información necesaria antes de las operaciones asíncronas
    let instance_directory = {
        let config_manager = get_config_manager()
            .lock()
            .map_err(|_| "Failed to lock config manager mutex".to_string())?;

        let config = config_manager.as_ref().map_err(|e| e.clone())?;

        let instances_dir = config.get_instances_dir();

        let instances = get_instances(instances_dir.to_str().unwrap_or_default())?;

        let instance = instances
            .into_iter()
            .find(|i| i.instanceId == instance_id)
            .ok_or_else(|| format!("Instance with ID {} not found", instance_id))?;

        // Obtener el directorio y clonarlo para uso posterior
        instance.instanceDirectory.clone()
    };

    // Delete the instance directory asynchronously
    if let Some(directory) = instance_directory {
        // Usar spawn_blocking para operaciones de I/O intensivas
        let result = tokio::task::spawn_blocking(move || {
            std::fs::remove_dir_all(&directory)
        }).await
        .map_err(|e| format!("Task join error: {}", e))?
        .map_err(|e| format!("Failed to delete instance directory: {}", e))?;
    }

    Ok(true)
}

#[tauri::command]
pub async fn search_instances(
    query: String
) -> Result<Vec<MinecraftInstance>, String> {
    let config_manager = get_config_manager()
        .lock()
        .map_err(|_| "Failed to lock config manager mutex".to_string())?;

    let config = config_manager.as_ref().map_err(|e| e.clone())?;

    let instances_dir = config.get_instances_dir();
    
    // Obtener la ruta segura como str
    let dir_path = instances_dir
        .to_str()
        .ok_or_else(|| "Invalid instances directory path".to_string())?;
    
    // Convertir la consulta a minúsculas para hacer la búsqueda case-insensitive
    let query_lowercase = query.to_lowercase();
    
    // Buscar instancias
    let instances = get_instances(dir_path)?;
    
    // Filtrar instancias de manera más flexible
    let filtered_instances: Vec<MinecraftInstance> = if query.is_empty() {
        // Si la consulta está vacía, devolver todas las instancias
        instances
    } else {
        instances
            .into_iter()
            .filter(|instance| {
                // Buscar en nombre (case-insensitive)
                instance.instanceName.to_lowercase().contains(&query_lowercase) ||
                // Buscar en version
                instance.minecraftVersion.to_lowercase().contains(&query_lowercase)
                
            })
            .collect()
    };

    // Devuelve resultados con un límite para evitar sobrecarga
    // pero solo si hay muchas instancias
    let max_results = 20;
    let results = if filtered_instances.len() > max_results {
        filtered_instances.into_iter().take(max_results).collect()
    } else {
        filtered_instances
    };

    Ok(results)
}