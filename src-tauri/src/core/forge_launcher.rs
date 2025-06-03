use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs::create_dir_all;
use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
};
use zip::ZipArchive;

use crate::config::get_config_manager;
use crate::core::accounts_manager::AccountsManager;
use crate::core::{minecraft_account::MinecraftAccount, minecraft_instance::MinecraftInstance};
use crate::interfaces::game_launcher::GameLauncher;
use uuid::Uuid;

// Import VanillaLauncher for inheritance
// use super::vanilla_launcher::VanillaLauncher; // Will be removed if VanillaLauncher direct use is removed
use super::minecraft_launcher::MinecraftLauncher; // To use the consolidated launcher

const CREATE_NO_WINDOW: u32 = 0x08000000;

pub struct ForgeLoader {
    instance: MinecraftInstance,
    // vanilla_launcher: VanillaLauncher, // Composition with VanillaLauncher for reusing functionality - Will be removed
}

impl ForgeLoader {
    pub fn new(instance: MinecraftInstance) -> Self {
        // let vanilla_launcher = VanillaLauncher::new(instance.clone()); // Will be removed
        Self {
            instance,
            // vanilla_launcher, // Will be removed
        }
    }

    // Get the parent (vanilla) version from the Forge manifest
    // This logic will be handled by MinecraftLauncher::load_merged_manifest
    /* fn get_inherits_from(&self, forge_manifest_json: &Value) -> Option<String> {
        println!("Checking for inheritsFrom field in Forge manifest");
        forge_manifest_json
            .get("inheritsFrom")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }*/

    // build_forge_classpath will be removed. MinecraftLauncher::build_classpath will be used.
    /* fn build_forge_classpath(...) -> String { ... } */

    // extract_natives should be primarily handled by InstanceBootstrap.
    // The launcher's responsibility is to provide the path to the natives directory.
    /* fn extract_natives(...) -> Result<(), Box<dyn std::error::Error>> { ... } */

    // parse_library_info will be effectively handled by MinecraftLauncher's manifest merging.
    /* fn parse_library_info(lib_name: &str) -> Option<(String, String, String)> { ... } */

    // process_forge_jvm_arguments will be removed. MinecraftLauncher::process_jvm_arguments will be used.
    /* fn process_forge_jvm_arguments(...) -> Vec<String> { ... } */

    // process_forge_game_arguments will be removed. MinecraftLauncher::process_game_arguments will be used.
    /* fn process_forge_game_arguments(...) -> Vec<String> { ... } */

    // is_modern_forge might still be useful if there are specific launch differences not covered by manifest args.
    // However, if arguments are fully specified in the merged manifest, this might become less critical.
    // For now, let's assume MinecraftLauncher handles argument styles transparently.
    /* fn is_modern_forge(&self, forge_manifest_json: &Value) -> bool {
        forge_manifest_json.get("arguments").is_some()
    }*/
}

impl GameLauncher for ForgeLoader {
    fn launch(&self) -> Option<Child> {
        let config_lock = get_config_manager()
            .lock()
            .expect("Failed to lock config manager mutex");

        let config = config_lock
            .as_ref()
            .expect("Config manager failed to initialize");

        let mc_memory = config.get_minecraft_memory().unwrap_or(2048);
        log::info!("[ForgeLaunch] Minecraft memory: {}MB", mc_memory);

        let default_java_path = config.get_java_dir().unwrap_or_else(|| {
            log::warn!("[ForgeLaunch] Java path is not set in global config, using placeholder 'default_java_path'");
            PathBuf::from("default_java_path") // This will likely fail later, but allows flow
        });

        let java_path = match &self.instance.javaPath {
            Some(path_str) if !path_str.is_empty() => PathBuf::from(path_str),
            _ => default_java_path,
        }
        .join("bin")
        .join(if cfg!(windows) { "java.exe" } else { "java" });
        log::info!("[ForgeLaunch] Effective Java path: {}", java_path.display());

        let accounts_manager = AccountsManager::new();
        let account_uuid = match &self.instance.accountUuid {
            Some(uuid) => uuid,
            None => {
                log::error!("[ForgeLaunch] No account UUID found for this instance.");
                return None;
            }
        };

        let account = match accounts_manager.get_minecraft_account_by_uuid(account_uuid) {
            Some(acct) => acct,
            None => {
                log::warn!("[ForgeLaunch] Account not found for UUID: {}. Using offline placeholder.", account_uuid);
                // Consider if proceeding with a dummy offline account is desired or if it should be an error.
                MinecraftAccount::new(
                    "offline_player".to_string(), // Default username
                    Uuid::new_v4().to_string(),    // Random UUID
                    None,                          // No access token
                    "offline".to_string(),         // User type
                )
            }
        };
        log::debug!("[ForgeLaunch] Using account: {:?}", account.username());

        let game_dir = match &self.instance.instanceDirectory {
            Some(dir_str) if !dir_str.is_empty() => PathBuf::from(dir_str).join("minecraft"),
            _ => {
                log::error!("[ForgeLaunch] Instance directory is not set for instance ID {}", self.instance.instanceId);
                return None;
            }
        };

        if !game_dir.exists() {
            if let Err(e) = fs::create_dir_all(&game_dir) {
                log::error!("[ForgeLaunch] Failed to create game directory {}: {}", game_dir.display(), e);
                return None;
            }
        }

        let base_minecraft_version = self.instance.minecraftVersion.clone();
        let forge_version_str = match self.instance.forgeVersion.as_ref() {
            Some(fv) if !fv.is_empty() => fv,
            _ => {
                log::error!("[ForgeLaunch] Forge version is not set for Forge instance ID {}", self.instance.instanceId);
                return None;
            }
        };
        let forge_full_version_id = format!("{}-forge-{}", base_minecraft_version, forge_version_str);

        log::info!("[ForgeLaunch] Launching Forge instance: {}, Base MC: {}, Forge Full ID: {}", self.instance.instanceName, base_minecraft_version, forge_full_version_id);

        let libraries_dir = game_dir.join("libraries");
        let natives_dir = game_dir.join("natives").join(&base_minecraft_version); // Natives are based on vanilla version
        let assets_dir = game_dir.join("assets");

        // Ensure natives directory exists (InstanceBootstrap should have created and populated it)
        if !natives_dir.exists() {
            log::warn!("Natives directory {} does not exist. Attempting to create.", natives_dir.display());
            if let Err(e) = create_dir_all(&natives_dir) {
                log::error!("Failed to create natives directory {}: {}", natives_dir.display(), e);
                // Depending on strictness, might return None here.
            }
        }

        let mc_launcher = MinecraftLauncher::new(self.instance.clone());

        // Load the merged manifest. MinecraftLauncher handles inheritsFrom internally.
        // The version ID passed to load_merged_manifest should be the Forge version ID.
        let merged_manifest = match mc_launcher.load_merged_manifest(&game_dir, &forge_full_version_id) {
            Some(json) => json,
            None => {
                log::error!("Failed to load or merge manifests for Forge version {}", forge_full_version_id);
                return None;
            }
        };

        // Get the client JAR path for the base vanilla version.
        let vanilla_client_jar = mc_launcher.get_client_jar_path(
            &game_dir,
            &merged_manifest, // Pass merged, though get_client_jar_path might only need version string
            &base_minecraft_version,
            self.instance.forgeVersion.as_deref()
        );

        // Get main class from the merged manifest (Forge's mainClass will be preferred by merge_manifests)
        let main_class = match merged_manifest.get("mainClass").and_then(|v| v.as_str()) {
            Some(class) => class,
            None => {
                log::error!("Main class not found in merged manifest for {}", forge_full_version_id);
                return None;
            }
        };

        // Get assets index from the merged manifest (usually from vanilla part)
        let assets_index = merged_manifest
            .get("assets")
            .and_then(|v| v.as_str())
            .or_else(|| merged_manifest.get("assetIndex").and_then(|ai| ai.get("id")?.as_str()))
            .unwrap_or("legacy"); // Fallback for very old versions

        // Build classpath using MinecraftLauncher's robust method
        let classpath_str = mc_launcher.build_classpath(&merged_manifest, &vanilla_client_jar, &libraries_dir);

        // Process JVM arguments using MinecraftLauncher
        let jvm_args = mc_launcher.process_jvm_arguments(
            &merged_manifest,
            &natives_dir,
            &classpath_str,
            mc_memory,
        );

        // Process game arguments using MinecraftLauncher
        // Note: The `minecraft_version` argument to process_game_arguments should be the display version,
        // which for Forge is typically the Forge version string.
        let game_args = mc_launcher.process_game_arguments(
            &merged_manifest,
            &account,
            &game_dir,
            &assets_dir,
            &natives_dir,
            &forge_full_version_id, // Use the full Forge version ID for ${version_name} placeholder
            assets_index,
        );

        // Build command
        let mut command = Command::new(&java_path);
        command.args(&jvm_args);
        command.arg(main_class);
        command.args(&game_args);
        command.current_dir(&game_dir);

        log::info!("[ForgeLaunch] Attempting to launch Minecraft with command: {:?}", java_path);
        log::debug!("[ForgeLaunch] Full command: {:?}", command);
        log::debug!("[ForgeLaunch] JVM Args: {:?}", jvm_args);
        log::debug!("[ForgeLaunch] Main Class: {}", main_class);
        log::debug!("[ForgeLaunch] Game Args: {:?}", game_args);


        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());

        match command.spawn() {
            Ok(child) => {
                log::info!("[ForgeLaunch] Spawned Forge child process with ID: {:?}", child.id());
                Some(child)
            }
            Err(e) => {
                log::error!("[ForgeLaunch] Failed to spawn Forge process: {}", e);
                None
            }
        }
    }
}
