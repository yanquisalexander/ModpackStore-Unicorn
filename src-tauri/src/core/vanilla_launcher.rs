use serde_json::Value;
use std::io::{BufRead, BufReader};
use std::{
    fs,
    path::PathBuf,
    process::{Child, Command, Stdio},
};

use crate::config::get_config_manager;
use crate::core::accounts_manager::AccountsManager;
use crate::core::{minecraft_account::MinecraftAccount, minecraft_instance::MinecraftInstance};
use crate::interfaces::game_launcher::GameLauncher;
use std::os::windows::process::CommandExt;
use uuid::Uuid;

const CREATE_NO_WINDOW: u32 = 0x08000000;

pub struct VanillaLauncher {
    instance: MinecraftInstance,
}

impl VanillaLauncher {
    pub fn new(instance: MinecraftInstance) -> Self {
        Self { instance }
    }

    // Helper function to parse and process arguments from the manifest
    // Modified process_game_arguments method with proper platform-specific handling
    fn process_game_arguments(
        &self,
        manifest_json: &Value,
        account: &MinecraftAccount,
        game_dir: &PathBuf,
        assets_dir: &PathBuf,
        natives_dir: &PathBuf,
        minecraft_version: &str,
        assets_index: &str,
    ) -> Vec<String> {
        let mut arguments = Vec::new();

        // Check for new-style arguments (1.13+)
        if let Some(args_obj) = manifest_json.get("arguments").and_then(|v| v.get("game")) {
            if let Some(args_array) = args_obj.as_array() {
                let mut i = 0;
                while i < args_array.len() {
                    let arg = &args_array[i];

                    // If it's a simple string argument
                    if let Some(arg_str) = arg.as_str() {
                        arguments.push(arg_str.to_string());
                    }
                    // If it's a complex rule-based argument
                    else if arg.is_object() {
                        // Check if rules allow this argument
                        let should_include = arg
                            .get("rules")
                            .and_then(|rules| rules.as_array())
                            .map(|rules_arr| {
                                // Process rules to determine if this arg should be included
                                // For simplicity, we're skipping complex rule evaluation
                                // In a full implementation, you'd check OS, features, etc.

                                // Check if any rules explicitly exclude the current OS
                                let mut include = true;
                                for rule in rules_arr {
                                    if let Some(action) =
                                        rule.get("action").and_then(|a| a.as_str())
                                    {
                                        if action == "allow" && rule.get("os").is_some() {
                                            // Check if this is an OS-specific rule
                                            if let Some(os_obj) = rule.get("os") {
                                                if let Some(os_name) =
                                                    os_obj.get("name").and_then(|n| n.as_str())
                                                {
                                                    // Only include if the current OS matches
                                                    let is_current_os = match os_name {
                                                        "windows" => cfg!(windows),
                                                        "osx" => cfg!(target_os = "macos"),
                                                        "linux" => cfg!(target_os = "linux"),
                                                        _ => false,
                                                    };

                                                    if !is_current_os {
                                                        include = false;
                                                    }
                                                }
                                            }
                                        } else if action == "disallow" && rule.get("os").is_some() {
                                            // Check if this OS should be excluded
                                            if let Some(os_obj) = rule.get("os") {
                                                if let Some(os_name) =
                                                    os_obj.get("name").and_then(|n| n.as_str())
                                                {
                                                    // Exclude if the current OS matches
                                                    let is_current_os = match os_name {
                                                        "windows" => cfg!(windows),
                                                        "osx" => cfg!(target_os = "macos"),
                                                        "linux" => cfg!(target_os = "linux"),
                                                        _ => false,
                                                    };

                                                    if is_current_os {
                                                        include = false;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                include
                            })
                            .unwrap_or(true);

                        if should_include {
                            if let Some(value) = arg.get("value") {
                                if let Some(value_str) = value.as_str() {
                                    arguments.push(value_str.to_string());
                                } else if let Some(value_arr) = value.as_array() {
                                    for v in value_arr {
                                        if let Some(v_str) = v.as_str() {
                                            arguments.push(v_str.to_string());
                                        }
                                    }
                                }
                            }
                        }
                    }
                    i += 1;
                }
            }
        }
        // Legacy-style arguments (pre-1.13)
        else if let Some(min_args) = manifest_json
            .get("minecraftArguments")
            .and_then(|v| v.as_str())
        {
            // Split the legacy-style argument string
            arguments.extend(min_args.split_whitespace().map(|s| s.to_string()));
        }
        // Fallback to hardcoded arguments for very old versions
        else {
            // Basic arguments that should work with old versions
            arguments.extend(vec![
                "--username".to_string(),
                "${auth_player_name}".to_string(),
                "--version".to_string(),
                "${version_name}".to_string(),
                "--gameDir".to_string(),
                "${game_directory}".to_string(),
                "--assetsDir".to_string(),
                "${assets_root}".to_string(),
            ]);

            // Additional arguments for slightly newer but still old versions
            if !assets_index.is_empty() {
                arguments.extend(vec![
                    "--assetIndex".to_string(),
                    "${assets_index_name}".to_string(),
                ]);
            }

            arguments.extend(vec![
                "--uuid".to_string(),
                "${auth_uuid}".to_string(),
                "--accessToken".to_string(),
                "${auth_access_token}".to_string(),
                "--userType".to_string(),
                "${user_type}".to_string(),
            ]);
        }

        // Replace variables in arguments
        let mut processed_args = Vec::new();
        for arg in arguments {
            let processed = arg
                .replace("${auth_player_name}", &account.username())
                .replace("${version_name}", minecraft_version)
                .replace("${game_directory}", &game_dir.to_string_lossy())
                .replace("${assets_root}", &assets_dir.to_string_lossy())
                .replace("${assets_index_name}", assets_index)
                .replace("${auth_uuid}", &account.uuid())
                .replace(
                    "${auth_access_token}",
                    account.access_token().as_deref().unwrap_or("null"),
                )
                .replace(
                    "${user_type}",
                    if account.user_type() != "offline" {
                        "mojang"
                    } else {
                        "legacy"
                    },
                )
                .replace("${version_type}", "release")
                .replace("${natives_directory}", &natives_dir.to_string_lossy())
                .replace("${launcher_name}", "modpackstore")
                .replace("${launcher_version}", "1.0.0")
                .replace("${classpath}", ""); // Classpath is handled separately

            processed_args.push(processed);
        }

        // Disable demo mode by correcting the flag (it should be a negative flag)
        if processed_args.contains(&"--demo".to_string()) {
            // Remove the existing demo flag if it exists
            let index = processed_args.iter().position(|x| x == "--demo").unwrap();
            // Remove quick world flag

            processed_args.remove(index);
            // Check if there's a value after it and remove that too if needed
            if index < processed_args.len() && processed_args[index] == "true" {
                processed_args.remove(index);
            }
        }

        // Don't add platform-specific arguments here - they should be added to JVM args
        // based on platform detection in the launch method

        processed_args
    }
}

impl GameLauncher for VanillaLauncher {
    fn launch(&self) -> Option<Child> {
        let config_lock = get_config_manager()
            .lock()
            .expect("Failed to lock config manager mutex");

        let config = config_lock
            .as_ref()
            .expect("Config manager failed to initialize");

        let mc_memory = config.get_minecraft_memory().unwrap_or(2048); // Default to 2GB if not set
        println!("Minecraft memory: {}MB", mc_memory);

        // Get Java path from configuration
        let default_java_path = config.get_java_dir().unwrap_or_else(|| {
            println!("Java path is not set");
            PathBuf::from("default_java_path")
        });

        // Get Java path from instance or use default
        let java_path = self
            .instance
            .javaPath
            .as_ref()
            .map(|path| PathBuf::from(path))
            .unwrap_or(default_java_path)
            .join("bin")
            .join(if cfg!(windows) { "java.exe" } else { "java" });

        println!("Java path: {}", java_path.display());

        let accounts_manager = AccountsManager::new();

        // If instance does not have an account (null), throw an error
        if self.instance.accountUuid.is_none() {
            println!("No account found for this instance.");
            return None;
        }

        let account = accounts_manager
            .get_minecraft_account_by_uuid(
                self.instance
                    .accountUuid
                    .as_ref()
                    .unwrap_or(&"".to_string()),
            )
            .unwrap_or_else(|| {
                println!(
                    "Account not found for UUID: {}",
                    self.instance
                        .accountUuid
                        .as_ref()
                        .unwrap_or(&"".to_string())
                );
                MinecraftAccount::new(
                    "offline".to_string(),
                    Uuid::new_v4().to_string(),
                    None,
                    "offline".to_string(),
                )
            });

        println!("Account: {:?}", account);

        // Get game directory
        // Game Dir is instanceDirectory + "/minecraft"
        let game_dir = self
            .instance
            .instanceDirectory
            .as_ref()
            .map(|dir| PathBuf::from(dir).join("minecraft"))
            .unwrap_or_else(|| PathBuf::from("default_path").join("minecraft"));

        if !game_dir.exists() {
            fs::create_dir_all(&game_dir).expect("Failed to create game directory");
        }

        let minecraft_version = self.instance.minecraftVersion.clone();

        let version_dir = game_dir.join("versions").join(&minecraft_version);
        let client_jar = version_dir.join(format!("{minecraft_version}.jar"));
        let natives_dir = game_dir.join("natives").join(&minecraft_version);
        let libraries_dir = game_dir.join("libraries");
        let assets_dir = game_dir.join("assets");
        let manifest_file = version_dir.join(format!("{minecraft_version}.json"));

        println!("version_dir: {}", version_dir.display());
        println!("client_jar: {}", client_jar.display());
        println!("natives_dir: {}", natives_dir.display());
        println!("libraries_dir: {}", libraries_dir.display());
        println!("assets_dir: {}", assets_dir.display());
        println!("manifest_file: {}", manifest_file.display());
        println!("game_dir: {}", game_dir.display());
        println!("java_path: {}", java_path.display());

        // Validate required files and directories
        for (desc, path) in &[
            ("Client JAR", &client_jar),
            ("Natives", &natives_dir),
            ("Libraries", &libraries_dir),
            ("Manifest", &manifest_file),
        ] {
            if !path.exists() {
                // Create the directory if it doesn't exist
                if let Some(parent) = path.parent() {
                    if !parent.exists() {
                        fs::create_dir_all(parent).expect(&format!("Failed to create {}", desc));
                    }
                }
            }
        }

        // Read and parse the JSON manifest
        let manifest_data = match fs::read_to_string(&manifest_file) {
            Ok(content) => content,
            Err(e) => {
                println!("Failed to read version manifest file: {}", e);
                return None;
            }
        };

        let manifest_json: Value = match serde_json::from_str(&manifest_data) {
            Ok(json) => json,
            Err(e) => {
                println!("Failed to parse version manifest JSON: {}", e);
                return None;
            }
        };

        // Get main class from manifest
        let main_class = match manifest_json.get("mainClass").and_then(|v| v.as_str()) {
            Some(class) => class.to_string(),
            None => {
                println!("Main class not found in manifest");
                return None;
            }
        };

        // Get assets index
        let assets_index = manifest_json
            .get("assets")
            .and_then(|v| v.as_str())
            .map(String::from)
            .or_else(|| {
                manifest_json
                    .get("assetIndex")?
                    .get("id")?
                    .as_str()
                    .map(String::from)
            })
            .unwrap_or_else(|| "legacy".to_string());

        // Build classpath
        let mut classpath = vec![client_jar.to_string_lossy().to_string()];

        if let Some(libs) = manifest_json.get("libraries").and_then(|v| v.as_array()) {
            for lib in libs {
                // Check if this library has rules that might exclude it
                let should_include = lib
                    .get("rules")
                    .and_then(|rules| rules.as_array())
                    .map(|rules_arr| {
                        // Simple rule checking - in a full implementation, check OS, etc.
                        true // Default to including
                    })
                    .unwrap_or(true);

                if !should_include {
                    continue;
                }

                // Try to get library path from downloads
                if let Some(path) = lib
                    .get("downloads")
                    .and_then(|downloads| downloads.get("artifact"))
                    .and_then(|artifact| artifact.get("path"))
                    .and_then(|path| path.as_str())
                {
                    let lib_path = libraries_dir
                        .join(path.replace('/', &std::path::MAIN_SEPARATOR.to_string()));
                    if lib_path.exists() {
                        classpath.push(lib_path.to_string_lossy().to_string());
                    } else {
                        println!("Library not found: {}", lib_path.display());
                    }
                }
                // Legacy format - construct path from name
                else if let Some(name) = lib.get("name").and_then(|name| name.as_str()) {
                    // Parse Maven coordinates
                    let parts: Vec<&str> = name.split(':').collect();
                    if parts.len() >= 3 {
                        let group_id =
                            parts[0].replace('.', &std::path::MAIN_SEPARATOR.to_string());
                        let artifact_id = parts[1];
                        let version = parts[2];

                        let lib_path = libraries_dir
                            .join(group_id)
                            .join(artifact_id)
                            .join(version)
                            .join(format!("{}-{}.jar", artifact_id, version));

                        if lib_path.exists() {
                            classpath.push(lib_path.to_string_lossy().to_string());
                        } else {
                            println!("Legacy library not found: {}", lib_path.display());
                        }
                    }
                }
            }
        }

        let classpath_str = classpath.join(if cfg!(windows) { ";" } else { ":" });

        // Process game arguments from manifest
        let game_args = self.process_game_arguments(
            &manifest_json,
            &account,
            &game_dir,
            &assets_dir,
            &natives_dir,
            &minecraft_version,
            &assets_index,
        );

        // Get JVM arguments from manifest (newer versions) or use defaults
        // Improved JVM arguments handling for the launch method
        // Place this code in your launch() method where JVM args are processed

        // Get JVM arguments from manifest (newer versions) or use defaults
        let mut jvm_args = vec![
            "-Xms512M".to_string(),
            format!("-Xmx{}M", mc_memory),
            format!("-Djava.library.path={}", natives_dir.display()),
            format!("-Dminecraft.client.jar={}", client_jar.display()),
            "-Dminecraft.launcher.brand=modpackstore".to_string(),
            "-Dminecraft.launcher.version=1.0.0".to_string(),
        ];

        // Add macOS specific arguments only on macOS
        if cfg!(target_os = "macos") {
            jvm_args.push("-XstartOnFirstThread".to_string());
        }

        // Add custom JVM args from manifest (newer versions)
        if let Some(args_obj) = manifest_json.get("arguments").and_then(|v| v.get("jvm")) {
            if let Some(args_array) = args_obj.as_array() {
                for arg in args_array {
                    // Handle simple string args
                    if let Some(arg_str) = arg.as_str() {
                        let processed = arg_str
                            .replace("${natives_directory}", &natives_dir.to_string_lossy())
                            .replace("${launcher_name}", "modpackstore")
                            .replace("${launcher_version}", "1.0.0")
                            .replace("${classpath}", &classpath_str);

                        // Only add if not already present (avoid duplicates)
                        if !jvm_args.contains(&processed) {
                            jvm_args.push(processed);
                        }
                    }
                    // Handle complex rule-based args
                    else if arg.is_object() {
                        // Check if rules allow this argument
                        let should_include = arg
                            .get("rules")
                            .and_then(|rules| rules.as_array())
                            .map(|rules_arr| {
                                // Process rules to determine if this arg should be included
                                let mut include = true;

                                for rule in rules_arr {
                                    if let Some(action) =
                                        rule.get("action").and_then(|a| a.as_str())
                                    {
                                        if action == "allow" && rule.get("os").is_some() {
                                            // Check if this is an OS-specific rule
                                            if let Some(os_obj) = rule.get("os") {
                                                if let Some(os_name) =
                                                    os_obj.get("name").and_then(|n| n.as_str())
                                                {
                                                    // Only include if the current OS matches
                                                    let is_current_os = match os_name {
                                                        "windows" => cfg!(windows),
                                                        "osx" => cfg!(target_os = "macos"),
                                                        "linux" => cfg!(target_os = "linux"),
                                                        _ => false,
                                                    };

                                                    if !is_current_os {
                                                        include = false;
                                                    }
                                                }
                                            }
                                        } else if action == "disallow" && rule.get("os").is_some() {
                                            // Check if this OS should be excluded
                                            if let Some(os_obj) = rule.get("os") {
                                                if let Some(os_name) =
                                                    os_obj.get("name").and_then(|n| n.as_str())
                                                {
                                                    // Exclude if the current OS matches
                                                    let is_current_os = match os_name {
                                                        "windows" => cfg!(windows),
                                                        "osx" => cfg!(target_os = "macos"),
                                                        "linux" => cfg!(target_os = "linux"),
                                                        _ => false,
                                                    };

                                                    if is_current_os {
                                                        include = false;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                include
                            })
                            .unwrap_or(true);

                        if should_include {
                            if let Some(value) = arg.get("value") {
                                if let Some(value_str) = value.as_str() {
                                    let processed = value_str
                                        .replace(
                                            "${natives_directory}",
                                            &natives_dir.to_string_lossy(),
                                        )
                                        .replace("${launcher_name}", "modpackstore")
                                        .replace("${launcher_version}", "1.0.0")
                                        .replace("${classpath}", &classpath_str);

                                    if !jvm_args.contains(&processed) {
                                        jvm_args.push(processed);
                                    }
                                } else if let Some(value_arr) = value.as_array() {
                                    for v in value_arr {
                                        if let Some(v_str) = v.as_str() {
                                            let processed = v_str
                                                .replace(
                                                    "${natives_directory}",
                                                    &natives_dir.to_string_lossy(),
                                                )
                                                .replace("${launcher_name}", "modpackstore")
                                                .replace("${launcher_version}", "1.0.0")
                                                .replace("${classpath}", &classpath_str);

                                            if !jvm_args.contains(&processed) {
                                                jvm_args.push(processed);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Make sure classpath is included
        if !jvm_args
            .iter()
            .any(|arg| arg == "-cp" || arg == "-classpath")
        {
            jvm_args.push("-cp".to_string());
            jvm_args.push(classpath_str);
        }

        // Build command
        let mut command = Command::new(java_path);

        // Add JVM arguments
        for arg in jvm_args {
            command.arg(arg);
        }

        // Add main class
        command.arg(&main_class);

        // Add game arguments
        for arg in game_args {
            command.arg(arg);
        }

        command.current_dir(&game_dir);
        println!("Command: {:?}", command);

        if cfg!(windows) {
            // On Windows, use creation flags to avoid console window
            command.creation_flags(CREATE_NO_WINDOW);
        }

        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());

        // Execute command
        let child = match command.spawn() {
            Ok(child) => {
                println!("Spawned child process: {:?}", child.id());
                child
            }
            Err(e) => {
                println!("Failed to spawn Minecraft process: {}", e);
                return None;
            }
        };

        Some(child)
    }
}
