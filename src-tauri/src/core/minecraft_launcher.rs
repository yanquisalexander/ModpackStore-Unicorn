//! src-tauri/src/core/minecraft_launcher.rs
//! Handles the logic for preparing and launching a specific Minecraft instance.

// --- Standard Library Imports ---
use std::io::{Error as IoError, ErrorKind as IoErrorKind, Result as IoResult};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus};
use std::thread; // Crucial for asynchronous operations

// --- Crate Imports ---
// Core components
use crate::core::minecraft_instance::MinecraftInstance; // Instance definition
use crate::core::minecraft_account::MinecraftAccount; // If needed for validation
use crate::core::vanilla_launcher::VanillaLauncher; // Vanilla launch logic
use crate::interfaces::game_launcher::GameLauncher; // Generic launch trait/logic

// Utilities & Managers (adjust paths if needed)
use crate::utils::config_manager::get_config_manager; // Access configuration
// use crate::core::tasks_manager::{TasksManager, TaskStatus, TaskInfo}; // Keep if used elsewhere

// Global App Handle (or use Tauri Managed State)
use crate::GLOBAL_APP_HANDLE; // Accessing the globally stored AppHandle

// --- External Crates ---
use serde_json::Value; // For JSON manipulation, especially in validation/payloads
use tauri::{Emitter, Manager}; // For emitting events to the frontend

//-----------------------------------------------------------------------------
// Struct Definition
//-----------------------------------------------------------------------------

/// Represents the launcher for a specific Minecraft instance.
/// Holds the instance configuration and provides methods to launch it.
pub struct InstanceLauncher {
    instance: MinecraftInstance, // The configuration of the instance to launch
}

//-----------------------------------------------------------------------------
// Implementation
//-----------------------------------------------------------------------------

impl InstanceLauncher {
    /// Creates a new `InstanceLauncher` for the given Minecraft instance.
    ///
    /// # Arguments
    ///
    /// * `instance` - The `MinecraftInstance` struct containing all necessary details.
    ///              This struct must implement `Clone`.
    pub fn new(instance: MinecraftInstance) -> Self {
        Self { instance }
    }

    // --- Helper Methods for Event Emission ---

    /// Emits a status update event to the frontend.
    /// Uses the global `AppHandle` to send events to all windows.
    ///
    /// # Arguments
    ///
    /// * `event_name` - The name of the event (e.g., "instance-launch-start").
    /// * `message` - A descriptive message for the frontend.
    fn emit_status(&self, event_name: &str, message: &str) {
        println!(
            "[Instance: {}] Emitting Event: {} - Message: {}",
            self.instance.instanceId, event_name, message
        );
        if let Ok(guard) = GLOBAL_APP_HANDLE.lock() {
            if let Some(app_handle) = guard.as_ref() {
                let payload = serde_json::json!({
                    "id": self.instance.instanceId,
                    "name": self.instance.instanceName, // Ensure instanceName is populated
                    "message": message
                });
                // Use emit to notify the specific window listening for this event
                if let Err(e) = app_handle.emit(event_name, payload) {
                     eprintln!(
                        "[Instance: {}] Failed to emit event '{}': {}",
                        self.instance.instanceId, event_name, e
                    );
                }
            } else {
                eprintln!(
                    "[Instance: {}] Error: GLOBAL_APP_HANDLE is None when trying to emit '{}'.",
                    self.instance.instanceId, event_name
                );
            }
        } else {
            eprintln!(
                "[Instance: {}] Error: Could not lock GLOBAL_APP_HANDLE mutex for '{}'.",
                self.instance.instanceId, event_name
            );
        }
    }

    /// Emits a specific "instance-error" event.
    /// Convenience function wrapping `emit_status`.
    ///
    /// # Arguments
    ///
    /// * `error_message` - The error description to send to the frontend.
    fn emit_error(&self, error_message: &str) {
        self.emit_status("instance-error", error_message);
    }

    // --- Process Monitoring ---

    /// Monitors the launched Minecraft process in a separate thread.
    /// Emits "instance-exited" or "instance-error" when the process terminates.
    ///
    /// # Arguments
    ///
    /// * `instance` - A clone of the `MinecraftInstance` data for context in the thread.
    /// * `child` - The `std::process::Child` representing the running Minecraft game.
    fn monitor_process(instance: MinecraftInstance, mut child: Child) {
        let instance_id = instance.instanceId.clone();
        let instance_name = instance.instanceName.clone();

        // Create a launcher instance specifically for emitting events from the monitor thread.
        // This requires MinecraftInstance to be Clone.
        let emitter_launcher = InstanceLauncher::new(instance);

        // Spawn the monitoring thread
        thread::spawn(move || {
            println!("[Monitor: {}] Started monitoring process.", instance_id);
            match child.wait() {
                Ok(exit_status) => {
                    // Process exited normally (or with an error code)
                    let message = format!(
                        "Minecraft instance '{}' exited with status: {}",
                        instance_name, exit_status
                    );
                    println!("[Monitor: {}] {}", instance_id, message);
                    emitter_launcher.emit_status("instance-exited", &message);
                }
                Err(e) => {
                    // Failed to wait for the process (less common)
                    let error_message = format!(
                        "Failed to wait for Minecraft instance '{}' process: {}",
                        instance_name, e
                    );
                    eprintln!("[Monitor: {}] {}", instance_id, error_message);
                    // Emit both error and exited events as the process state is uncertain but terminated.
                    emitter_launcher.emit_error(&error_message);
                    emitter_launcher.emit_status("instance-exited", "Minecraft process ended unexpectedly.");
                }
            }
             println!("[Monitor: {}] Finished monitoring.", instance_id);
        });
    }

    // --- Placeholder/Implementation for Core Logic Steps ---

    /// Validates the Minecraft account associated with the launch (if necessary).
    /// TODO: Replace with actual account validation logic.
    fn validate_account(&self) -> IoResult<Value> {
        println!("[Instance: {}] Validating account...", self.instance.instanceId);
        // --- Replace with your actual validation logic ---
        // Example: Check credentials, refresh tokens, etc.
        // If validation fails, return an appropriate IoError:
        // return Err(IoError::new(IoErrorKind::PermissionDenied, "Invalid credentials"));
        // --- End Placeholder ---
        Ok(serde_json::json!({ "status": "validated" })) // Placeholder success
    }

    /// Revalidates or downloads necessary game assets, libraries, etc.
    /// TODO: Replace with actual asset checking/downloading logic.
    fn revalidate_assets(&self) -> IoResult<()> {
        println!("[Instance: {}] Revalidating assets...", self.instance.instanceName);
        self.emit_status("instance-downloading-assets", "Verificando/Descargando assets...");

        // Check if Minecraft version is known
        if self.instance.minecraftVersion.is_empty() {
            let err_msg = "Cannot revalidate assets: Minecraft version is not specified.";
            eprintln!("[Instance: {}] {}", self.instance.instanceId, err_msg);
            self.emit_error(err_msg);
            return Err(IoError::new(IoErrorKind::InvalidData, err_msg));
        }

        // --- Replace with your actual asset/library download/check logic ---
        // This could involve checking checksums, downloading missing files, etc.
        // Use libraries like reqwest for downloads, manage progress if possible.
        // If any step fails, emit an error and return Err(...):
        // let download_error_msg = "Failed to download critical asset XYZ";
        // self.emit_error(download_error_msg);
        // return Err(IoError::new(IoErrorKind::Other, download_error_msg));

        // Simulate work
        // thread::sleep(std::time::Duration::from_secs(2)); // Remove in production
        // --- End Placeholder ---

        println!(
            "[Instance: {}] Asset revalidation completed.",
            self.instance.instanceName
        );
        // Optionally emit a different status message upon completion if desired,
        // but "instance-launch-start" will likely follow immediately.
        Ok(())
    }


    // --- Internal Synchronous Launch Logic ---

    /// Contains the core, sequential steps for launching the instance.
    /// This method is intended to be run within a dedicated thread.
    /// It handles validation, asset checks, and the actual game launch command.
    /// Errors encountered stop the process and emit an "instance-error" event.
    fn perform_launch_steps(&self) {
        // Note: Initial "instance-launch-start" event is emitted by this function.
        self.emit_status("instance-launch-start", "Preparando lanzamiento...");
        println!("[Launch Thread: {}] Starting launch steps.", self.instance.instanceId);

        // 1. Validate Account
        if let Err(e) = self.validate_account() {
            let err_msg = format!("Error en validación de cuenta: {}", e);
            eprintln!("[Launch Thread: {}] {}", self.instance.instanceId, err_msg);
            self.emit_error(&err_msg);
            return; // Stop the thread execution
        }
         println!("[Launch Thread: {}] Account validation successful.", self.instance.instanceId);

        // 2. Revalidate Assets
        if let Err(e) = self.revalidate_assets() {
            let err_msg = format!("Error en revalidación de assets: {}", e);
            eprintln!("[Launch Thread: {}] {}", self.instance.instanceId, err_msg);
            // Assuming revalidate_assets already emitted a specific error message
            return; // Stop the thread execution
        }
        println!("[Launch Thread: {}] Asset revalidation successful.", self.instance.instanceId);

        // 3. Determine Launch Type and Execute
        let final_launch_result: Result<(), IoError> = if self.instance.is_forge_instance() {
            // --- Forge Launch ---
            println!("[Launch Thread: {}] Preparing Forge launch...", self.instance.instanceId);
            let err_msg = "Lanzamiento de Forge aún no implementado.";
            self.emit_error(err_msg);
            Err(IoError::new(IoErrorKind::Unsupported, err_msg))

        } else {
            // --- Vanilla Launch ---
            println!("[Launch Thread: {}] Preparing Vanilla launch...", self.instance.instanceId);
            let launcher = VanillaLauncher::new(self.instance.clone());

            // Execute the launch command via the GameLauncher trait/implementation
            match GameLauncher::launch(&launcher) { // Assumes this returns Option<Child>
                Some(child_process) => {
                    // Success! Game process obtained.
                    println!(
                        "[Launch Thread: {}] Minecraft process started successfully (PID: {}).",
                        self.instance.instanceId, child_process.id()
                    );
                    self.emit_status("instance-launched", "Minecraft se está ejecutando.");
                    // Start monitoring the process in its own background thread.
                    Self::monitor_process(self.instance.clone(), child_process);
                    Ok(()) // Indicate successful initiation of the launch.
                }
                None => {
                    // Failure: GameLauncher::launch returned None.
                    let err_msg = "Fallo al iniciar el proceso de Minecraft (GameLauncher retornó None).";
                    eprintln!("[Launch Thread: {}] {}", self.instance.instanceId, err_msg);
                    self.emit_error(err_msg);
                    Err(IoError::new(IoErrorKind::Other, err_msg))
                }
            }
        }; // End if/else for launch type

        // Log final status of the launch attempt within this thread
        if let Err(e) = final_launch_result {
            eprintln!(
                "[Launch Thread: {}] Launch sequence failed: {}",
                self.instance.instanceId, e
            );
        } else {
            println!(
                "[Launch Thread: {}] Launch sequence initiated successfully (monitoring started).",
                self.instance.instanceId
            );
        }
         println!("[Launch Thread: {}] Finishing execution.", self.instance.instanceId);
         // Thread finishes here.
    }


    // --- Public Asynchronous Launch Method ---

    /// Initiates the instance launch process in a separate background thread.
    /// This function returns immediately, allowing the caller (e.g., Tauri command)
    /// to remain responsive. Status updates are sent via events.
    /// Requires `MinecraftInstance` to implement `Clone`.
    pub fn launch_instance_async(&self) {
        // Clone the necessary instance data for the new thread.
        let instance_data_clone = self.instance.clone();
        let instance_id = instance_data_clone.instanceId.clone(); // For logging before spawn

        println!("[Main Thread] Spawning launch thread for instance: {}", instance_id);

        // Spawn the background thread
        thread::spawn(move || {
            // Create a new InstanceLauncher specific to this thread.
            let thread_launcher = InstanceLauncher::new(instance_data_clone);
            // Execute the sequential, potentially blocking launch steps within this thread.
            thread_launcher.perform_launch_steps();
            // The thread will terminate automatically after perform_launch_steps finishes.
        });

        // Return immediately to the caller.
        println!("[Main Thread] Finished spawning thread for {}. Caller continues.", instance_id);
    }
} // end impl InstanceLauncher


//-----------------------------------------------------------------------------
// Example Tauri Command Usage
//-----------------------------------------------------------------------------
// This function would typically live in your main.rs or a commands module,
// not usually directly in minecraft_launcher.rs, but shown here for context.

/*
#[tauri::command]
pub fn launch_mc_instance_async(instance_id: String) -> Result<(), String> {
    println!("[Tauri Command] Received async launch request for instance: {}", instance_id);

    // 1. Retrieve the full MinecraftInstance details using the ID.
    // Replace this with your actual logic to fetch instance data.
    let instance = match crate::core::instance_manager::get_instance_by_id(&instance_id) {
         Ok(inst) => inst,
         Err(e) => {
             let err_msg = format!("Instance '{}' not found: {}", instance_id, e);
             eprintln!("[Tauri Command] {}", err_msg);
             return Err(err_msg); // Report error back to frontend immediately
         }
    };

    // Ensure the retrieved instance implements Clone if not already verified
    // let instance: MinecraftInstance = instance; // Type annotation if needed

    // 2. Create an InstanceLauncher for the specific instance.
    let launcher = InstanceLauncher::new(instance);

    // 3. Call the asynchronous launch method.
    // This spawns the background thread and returns control immediately.
    launcher.launch_instance_async();

    // 4. Return success to the frontend immediately.
    // This indicates the launch *process* has started, not that the game is running yet.
    // The frontend relies on events ("instance-launch-start", etc.) for actual status.
    println!("[Tauri Command] Successfully initiated async launch for {}", instance_id);
    Ok(())
}

// Remember to register this command in your main.rs:
// .invoke_handler(tauri::generate_handler![
//     launch_mc_instance_async,
//     // ... other commands
// ])
*/