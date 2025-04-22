//! src-tauri/src/core/minecraft_launcher.rs
//! Handles the logic for preparing and launching a specific Minecraft instance.

// --- Standard Library Imports ---
use std::io::{Error as IoError, ErrorKind as IoErrorKind, Result as IoResult, BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus};
use std::thread; // Crucial for asynchronous operations
use log::{info, error};
use std::sync::{Arc, Mutex}; // For thread-safe shared state


// --- Crate Imports ---
// Core components
use crate::core::network_utilities; // Network utilities for checking internet connection
use crate::core::forge_launcher::ForgeLoader; // Forge launch logic
use crate::core::instance_bootstrap::InstanceBootstrap;
use crate::core::minecraft_account::MinecraftAccount; // If needed for validation
use crate::core::minecraft_instance::MinecraftInstance; // Instance definition
use crate::core::vanilla_launcher::VanillaLauncher; // Vanilla launch logic
use crate::interfaces::game_launcher::GameLauncher; // Generic launch trait/logic // Asset revalidation logic

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

enum PossibleErrorCode {
    IncompatibleJavaVersion,
    MissingLibraries,
    CorruptedMod,
    UnknownError,
}

impl PossibleErrorCode {
    fn as_str(&self) -> &'static str {
        match self {
            PossibleErrorCode::IncompatibleJavaVersion => "INCOMPATIBLE_JAVA_VERSION",
            PossibleErrorCode::MissingLibraries => "MISSING_LIBRARIES",
            PossibleErrorCode::CorruptedMod => "CORRUPTED_MOD",
            PossibleErrorCode::UnknownError => "UNKNOWN_ERROR",
        }
    }
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
    /// * `data` - Optional additional data to send with the event.
    ///   This can be a JSON object or any other serializable type.
    fn emit_status(&self, event_name: &str, message: &str, data: Option<Value>) {
        println!(
            "[Instance: {}] Emitting Event: {} - Message: {}",
            self.instance.instanceId, event_name, message
        );
        if let Ok(guard) = GLOBAL_APP_HANDLE.lock() {
            if let Some(app_handle) = guard.as_ref() {
                let payload = serde_json::json!({
                    "id": self.instance.instanceId,
                    "name": self.instance.instanceName, // Ensure instanceName is populated
                    "message": message,
                    "data": data.unwrap_or(serde_json::json!({})) // Use empty JSON if no data provided
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
    fn emit_error(&self, error_message: &str, data: Option<Value>) {
        println!(
            "[Instance: {}] Emitting Error Event: {}",
            self.instance.instanceId, error_message
        );
        self.emit_status("instance-error", error_message, data);
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
        let emitter_launcher = InstanceLauncher::new(instance);
        
        // Use Arc and Mutex to share the error code between threads
        let detected_error = Arc::new(Mutex::new(None::<PossibleErrorCode>));
    
        // Spawn the monitoring thread
        thread::spawn(move || {
            println!("[Monitor: {}] Started monitoring process.", instance_id);
            
            // Set up stdout redirection if stdout is available
            if let Some(stdout) = child.stdout.take() {
                let instance_id_clone = instance_id.clone();
                let detected_error_clone = Arc::clone(&detected_error);
                thread::spawn(move || {
                    let reader = BufReader::new(stdout);
                    for line in reader.lines() {
                        if let Ok(line) = line {
                            log::info!("[Minecraft: {}] {}", instance_id_clone, line);
                            // Check for specific error messages
                            let mut error = detected_error_clone.lock().unwrap();
                            if line.contains("Unsupported major.minor version") || line.contains("UnsupportedClassVersionError") {
                                *error = Some(PossibleErrorCode::IncompatibleJavaVersion);
                            } else if line.contains("Could not find or load main class") {
                                *error = Some(PossibleErrorCode::MissingLibraries);
                            } else if line.contains("Exception in thread") && line.contains("mod") {
                                *error = Some(PossibleErrorCode::CorruptedMod);
                            }
                        }
                    }
                });
            }
            
            // Set up stderr redirection if stderr is available
            if let Some(stderr) = child.stderr.take() {
                let instance_id_clone = instance_id.clone();
                let detected_error_clone = Arc::clone(&detected_error);
                thread::spawn(move || {
                    let reader = BufReader::new(stderr);
                    for line in reader.lines() {
                        if let Ok(line) = line {
                            log::error!("[Minecraft: {}] {}", instance_id_clone, line);
                            // Check for specific error messages
                            let mut error = detected_error_clone.lock().unwrap();
                            if line.contains("Unsupported major.minor version") || line.contains("UnsupportedClassVersionError") {
                                *error = Some(PossibleErrorCode::IncompatibleJavaVersion);
                            } else if line.contains("Could not find or load main class") {
                                *error = Some(PossibleErrorCode::MissingLibraries);
                            } else if line.contains("Exception in thread") && line.contains("mod") {
                                *error = Some(PossibleErrorCode::CorruptedMod);
                            }
                        }
                    }
                });
            }
    
            match child.wait() {
                Ok(exit_status) => {
                    // Process exited normally (or with an error code)
                    let message = format!(
                        "Minecraft instance '{}' exited with status: {}",
                        instance_name, exit_status
                    );
    
                    log::info!("[Monitor: {}] {}", instance_id, message);
    
                    // Get the detected error, if any, as a string
                    let possible_error_code = {
                        let error_guard = detected_error.lock().unwrap();
                        match *error_guard {
                            // Explicitly convert each enum variant to a string
                            Some(PossibleErrorCode::IncompatibleJavaVersion) => "INCOMPATIBLE_JAVA_VERSION".to_string(),
                            Some(PossibleErrorCode::MissingLibraries) => "MISSING_LIBRARIES".to_string(),
                            Some(PossibleErrorCode::CorruptedMod) => "CORRUPTED_MOD".to_string(),
                            Some(PossibleErrorCode::UnknownError) => "UNKNOWN_ERROR".to_string(),
                            // Add any other variants your enum might have
                            None => {
                                // If no specific error was detected, use the exit code
                                match exit_status.code() {
                                    Some(code) => code.to_string(),
                                    None => "UNKNOWN".to_string(),
                                }
                            }
                        }
                    };
    
                    emitter_launcher.emit_status(
                        "instance-exited",
                        &message,
                        Some(serde_json::json!({
                            "instanceName": instance_name,
                            "exitCode": exit_status.code(),
                            "possibleErrorCode": possible_error_code
                        })),
                    );
                }
                Err(e) => {
                    // Failed to wait for the process (less common)
                    let error_message = format!(
                        "Failed to wait for Minecraft instance '{}' process: {}",
                        instance_name, e
                    );
                    log::error!("[Monitor: {}] {}", instance_id, error_message);
                    // Emit both error and exited events as the process state is uncertain but terminated.
                    emitter_launcher.emit_error(&error_message, None);
                    emitter_launcher.emit_status(
                        "instance-exited",
                        "Minecraft process ended unexpectedly.",
                        Some(serde_json::json!({
                            "instanceName": instance_name,
                            "error": error_message
                        })),
                    );
                }
            }
            log::info!("[Monitor: {}] Finished monitoring.", instance_id);
        });
    }

   

    /// Revalidates or downloads necessary game assets, libraries, etc.
    /// TODO: Replace with actual asset checking/downloading logic.
    fn revalidate_assets(&mut self) -> IoResult<()> {
        println!(
            "[Instance: {}] Revalidating assets...",
            self.instance.instanceName
        );
        self.emit_status(
            "instance-downloading-assets",
            "Verificando/Descargando assets...",
            None,
        );

        // Check if Minecraft version is known
        if self.instance.minecraftVersion.is_empty() {
            let err_msg = "Cannot revalidate assets: Minecraft version is not specified.";
            eprintln!("[Instance: {}] {}", self.instance.instanceId, err_msg);
            self.emit_error(err_msg, None);
            return Err(IoError::new(IoErrorKind::InvalidData, err_msg));
        }

        // ¿Has internet connection? Continue with asset revalidation
        // Otherwise, skip this step (¿Maybe user has downloaded assets before?)

        let has_internet = network_utilities::check_real_connection();


        if !has_internet {
            let warning_msg = "No internet connection. Skipping asset revalidation.";
            eprintln!("[Instance: {}] {}", self.instance.instanceId, warning_msg);
            return Ok(());
        }

        // Call revalidate_assets from InstanceBootstrap (We pass MinecraftInstance to it)

        let mut instance_bootstrap = InstanceBootstrap::new();
        let result = instance_bootstrap.revalidate_assets(&mut self.instance)?;

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
    fn perform_launch_steps(&mut self) {
        // Note: Initial "instance-launch-start" event is emitted by this function.
        self.emit_status("instance-launch-start", "Preparando lanzamiento...", None);
        println!(
            "[Launch Thread: {}] Starting launch steps.",
            self.instance.instanceId
        );

    

        // 2. Revalidate Assets
        if let Err(e) = self.revalidate_assets() {
            let err_msg = format!("Error en revalidación de assets: {}", e);
            eprintln!("[Launch Thread: {}] {}", self.instance.instanceId, err_msg);
            // Assuming revalidate_assets already emitted a specific error message
            return; // Stop the thread execution
        }
        println!(
            "[Launch Thread: {}] Asset revalidation successful.",
            self.instance.instanceId
        );

        // 3. Determine Launch Type and Execute
        let final_launch_result: Result<(), IoError> = if self.instance.is_forge_instance() {
            // --- Forge Launch ---
            println!(
                "[Launch Thread: {}] Preparing Forge launch...",
                self.instance.instanceId
            );

            let launcher = ForgeLoader::new(self.instance.clone());
            // Execute the launch command via the GameLauncher trait/implementation
            match GameLauncher::launch(&launcher) {
                // Assumes this returns Option<Child>
                Some(child_process) => {
                    // Success! Game process obtained.
                    log::info!(
                        "[Launch Thread: {}] Minecraft process started successfully (PID: {}).",
                        self.instance.instanceId,
                        child_process.id()
                    );
                    self.emit_status("instance-launched", "Minecraft se está ejecutando.", None);
                    // Start monitoring the process in its own background thread.
                    Self::monitor_process(self.instance.clone(), child_process);
                    Ok(()) // Indicate successful initiation of the launch.
                }
                None => {
                    // Failure: GameLauncher::launch returned None.
                    let err_msg =
                        "Fallo al iniciar el proceso de Minecraft (GameLauncher retornó None).";
                    log::error!("[Launch Thread: {}] {}", self.instance.instanceId, err_msg);
                    self.emit_error(err_msg, None);
                    Err(IoError::new(IoErrorKind::Other, err_msg))
                }
            }
        } else {
            // --- Vanilla Launch ---
            log::info!(
                "[Launch Thread: {}] Preparing Vanilla launch...",
                self.instance.instanceId
            );
            let launcher = VanillaLauncher::new(self.instance.clone());

            // Execute the launch command via the GameLauncher trait/implementation
            match GameLauncher::launch(&launcher) {
                // Assumes this returns Option<Child>
                Some(child_process) => {
                    // Success! Game process obtained.
                    println!(
                        "[Launch Thread: {}] Minecraft process started successfully (PID: {}).",
                        self.instance.instanceId,
                        child_process.id()
                    );
                    self.emit_status("instance-launched", "Minecraft se está ejecutando.", None);
                    // Start monitoring the process in its own background thread.
                    Self::monitor_process(self.instance.clone(), child_process);
                    Ok(()) // Indicate successful initiation of the launch.
                }
                None => {
                    // Failure: GameLauncher::launch returned None.
                    let err_msg =
                        "Fallo al iniciar el proceso de Minecraft (GameLauncher retornó None).";
                    eprintln!("[Launch Thread: {}] {}", self.instance.instanceId, err_msg);
                    self.emit_error(err_msg, None);
                    Err(IoError::new(IoErrorKind::Other, err_msg))
                }
            }
        }; // End if/else for launch type

        // Log final status of the launch attempt within this thread
        if let Err(e) = final_launch_result {
            log::error!(
                "[Launch Thread: {}] Launch sequence failed: {}",
                self.instance.instanceId, e
            );
        } else {
            let config_manager = get_config_manager();
            let close_on_launch = config_manager
                .lock()
                .expect("Failed to lock config manager mutex")
                .get_close_on_launch();

            if close_on_launch {
                // Close the main process if configured to do so
                println!(
                    "[Launch Thread: {}] Closing main process as per configuration.",
                    self.instance.instanceId
                );
                // Use the global app handle to close the main process
                if let Ok(guard) = GLOBAL_APP_HANDLE.lock() {
                    if let Some(app_handle) = guard.as_ref() {
                        app_handle.exit(0);
                    } else {
                        eprintln!(
                            "[Launch Thread: {}] Error: GLOBAL_APP_HANDLE is None when trying to close.",
                            self.instance.instanceId
                        );
                    }
                } else {
                    eprintln!(
                        "[Launch Thread: {}] Error: Could not lock GLOBAL_APP_HANDLE mutex for closing.",
                        self.instance.instanceId
                    );
                }
            }

            log::info!(
                "[Launch Thread: {}] Launch sequence initiated successfully (monitoring started).",
                self.instance.instanceId
            );
        }
        println!(
            "[Launch Thread: {}] Finishing execution.",
            self.instance.instanceId
        );
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

        log::info!(
            "[Main Thread] Spawning launch thread for instance: {}",
            instance_id
        );

        // Spawn the background thread
        thread::spawn(move || {
            // Create a new InstanceLauncher specific to this thread.
            let mut thread_launcher = InstanceLauncher::new(instance_data_clone);
            // Execute the sequential, potentially blocking launch steps within this thread.
            thread_launcher.perform_launch_steps();
            // The thread will terminate automatically after perform_launch_steps finishes.
        });

        // Return immediately to the caller.
        log::info!(
            "[Main Thread] Finished spawning thread for {}. Caller continues.",
            instance_id
        );
    }
}
