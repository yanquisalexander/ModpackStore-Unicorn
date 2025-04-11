use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::sync::Mutex;
use uuid::Uuid;
use tauri::{AppHandle, Emitter, Wry}; // Asegúrate de importar Wry si no lo estaba

// --- TaskStatus y TaskInfo permanecen iguales ---

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct TaskInfo {
    pub id: String,
    pub label: String,
    pub status: TaskStatus,
    pub progress: f32,
    pub message: String,
    pub data: Option<serde_json::Value>,
    pub created_at: String,
}


// --- Importa tu variable estática ---
// Asumiendo que main.rs está en la raíz del crate (src/main.rs)
// Si TasksManager está en otro módulo, ajusta la ruta (ej: `crate::main::GLOBAL_APP_HANDLE`)
use crate::GLOBAL_APP_HANDLE;

pub struct TasksManager {
    pub tasks: Mutex<HashMap<String, TaskInfo>>,
}

impl TasksManager {
    pub fn new() -> Self {
        Self {
            tasks: Mutex::new(HashMap::new()),
        }
    }

    // Ya no necesita app_handle como parámetro
    pub fn add_task(&self, label: &str, data: Option<serde_json::Value>) -> String {
        let id = Uuid::new_v4().to_string();
        let task = TaskInfo {
            id: id.clone(),
            label: label.to_string(),
            status: TaskStatus::Pending,
            progress: 0.0,
            message: "En espera...".into(),
            data,
            created_at: chrono::Utc::now().to_rfc3339(), // Asegúrate de tener chrono
        };

        println!("Task created: {}", task.id);

        self.tasks.lock().expect("Failed to lock tasks mutex for add").insert(id.clone(), task.clone());

        // Emitir evento usando el AppHandle global
        println!("Attempting to emit task-created event for task: {}", task.id);
        // Bloquea el Mutex para acceder al Option<AppHandle> global
        if let Ok(guard) = GLOBAL_APP_HANDLE.lock() {
            // Verifica si el AppHandle ya fue inicializado en setup
            if let Some(app_handle) = guard.as_ref() {
                // Usa app_handle para emitir el evento
                if let Err(e) = app_handle.emit("task-created", task.clone()) { // Clonar task aquí
                    eprintln!("Failed to emit task-created event: {}", e);
                } else {
                     println!("Successfully emitted task-created event.");
                }
            } else {
                eprintln!("Error: GLOBAL_APP_HANDLE is None when trying to emit task-created.");
            }
        } else {
             eprintln!("Error: Could not lock GLOBAL_APP_HANDLE mutex for task-created.");
        }


        id
    }

    // Ya no necesita app_handle como parámetro
    pub fn update_task(&self, id: &str, status: TaskStatus, progress: f32, message: &str, data: Option<serde_json::Value>) {
        let mut updated_task_clone = None;

        // Alcance del bloqueo para las tareas
        {
            let mut tasks = self.tasks.lock().expect("Failed to lock tasks mutex for update");
            if let Some(task) = tasks.get_mut(id) {
                task.status = status;
                task.progress = progress;
                task.message = message.to_string();
                task.data = data;
                updated_task_clone = Some(task.clone()); // Clonar dentro del bloqueo
            }
        } // Bloqueo de `tasks` se libera aquí

        // Si la tarea fue actualizada, intenta emitir el evento
        if let Some(task_to_emit) = updated_task_clone {
            println!("Attempting to emit task-updated event for task: {}", task_to_emit.id);
            // Bloquea el Mutex para acceder al Option<AppHandle> global
             if let Ok(guard) = GLOBAL_APP_HANDLE.lock() {
                if let Some(app_handle) = guard.as_ref() {
                    if let Err(e) = app_handle.emit("task-updated", task_to_emit) { // Usar el clon
                        eprintln!("Failed to emit task-updated event: {}", e);
                    } else {
                        println!("Successfully emitted task-updated event.");
                    }
                } else {
                    eprintln!("Error: GLOBAL_APP_HANDLE is None when trying to emit task-updated.");
                }
            } else {
                eprintln!("Error: Could not lock GLOBAL_APP_HANDLE mutex for task-updated.");
            }
        }
    }

    pub fn get_all_tasks(&self) -> Vec<TaskInfo> {
        self.tasks.lock().expect("Failed to lock tasks mutex for get").values().cloned().collect()
    }
}

impl Default for TasksManager {
    fn default() -> Self {
        Self::new()
    }
}