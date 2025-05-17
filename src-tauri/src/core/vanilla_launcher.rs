use serde_json::Value;
use std::collections::HashMap;
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
};

use crate::config::get_config_manager;
use crate::core::accounts_manager::AccountsManager;
use crate::core::{minecraft_account::MinecraftAccount, minecraft_instance::MinecraftInstance};
use crate::interfaces::game_launcher::GameLauncher;
use uuid::Uuid;

const CREATE_NO_WINDOW: u32 = 0x08000000;

pub struct VanillaLauncher {
    instance: MinecraftInstance,
}

impl VanillaLauncher {
    pub fn new(instance: MinecraftInstance) -> Self {
        Self { instance }
    }

    // Checks if a rule should apply based on OS, arch, and feature requirements
    pub fn should_apply_rule(
        &self,
        rule: &Value,
        features: Option<&HashMap<String, bool>>,
    ) -> bool {
        let action = rule
            .get("action")
            .and_then(|a| a.as_str())
            .unwrap_or("allow");
        let mut should_apply = action == "allow";

        // Check OS rules
        if let Some(os_obj) = rule.get("os") {
            let mut os_match = true;

            // Check OS name
            if let Some(os_name) = os_obj.get("name").and_then(|n| n.as_str()) {
                let is_current_os = match os_name {
                    "windows" => cfg!(windows),
                    "osx" => cfg!(target_os = "macos"),
                    "linux" => cfg!(target_os = "linux"),
                    _ => false,
                };
                if !is_current_os {
                    os_match = false;
                }
            }

            // Check OS architecture
            if let Some(os_arch) = os_obj.get("arch").and_then(|a| a.as_str()) {
                let is_current_arch = match os_arch {
                    "x86" => cfg!(target_arch = "x86"),
                    "x86_64" => cfg!(target_arch = "x86_64"),
                    "arm" => cfg!(target_arch = "arm"),
                    "arm64" => cfg!(target_arch = "aarch64"),
                    _ => false,
                };
                if !is_current_arch {
                    os_match = false;
                }
            }

            if action == "allow" {
                should_apply = os_match;
            } else {
                should_apply = !os_match;
            }
        }

        // Check feature rules
        if let Some(feature_obj) = rule.get("features") {
            if let Some(features_map) = features {
                for (feature_name, feature_value) in
                    feature_obj.as_object().unwrap_or(&serde_json::Map::new())
                {
                    if let Some(expected_value) = feature_value.as_bool() {
                        let actual_value = *features_map.get(feature_name).unwrap_or(&false);
                        if actual_value != expected_value {
                            should_apply = action != "allow";
                            break;
                        }
                    }
                }
            } else {
                // If feature rules exist but no features are provided, rule doesn't apply
                should_apply = action != "allow";
            }
        }

        should_apply
    }

    // Process values from a rule or argument
    fn process_rule_values(
        &self,
        value: &Value,
        placeholder_map: &HashMap<String, String>,
    ) -> Vec<String> {
        let mut values = Vec::new();

        if let Some(value_str) = value.as_str() {
            values.push(self.replace_placeholders(value_str, placeholder_map));
        } else if let Some(value_arr) = value.as_array() {
            for v in value_arr {
                if let Some(v_str) = v.as_str() {
                    values.push(self.replace_placeholders(v_str, placeholder_map));
                }
            }
        }

        values
    }

    // Replace placeholders in a string using a map
    pub fn replace_placeholders(
        &self,
        input: &str,
        placeholders: &HashMap<String, String>,
    ) -> String {
        let mut result = input.to_string();
        for (key, value) in placeholders {
            result = result.replace(&format!("${{{}}}", key), value);
        }
        result
    }

    // Process arguments (both JVM and game) using the new rule evaluation system
    pub fn process_arguments(
        &self,
        args_obj: &Value,
        placeholders: &HashMap<String, String>,
        features: Option<&HashMap<String, bool>>,
    ) -> Vec<String> {
        let mut processed_args = Vec::new();

        if let Some(args_array) = args_obj.as_array() {
            for arg in args_array {
                // If it's a simple string argument
                if let Some(arg_str) = arg.as_str() {
                    processed_args.push(self.replace_placeholders(arg_str, placeholders));
                }
                // If it's a complex rule-based argument
                else if arg.is_object() {
                    // Check if rules allow this argument
                    if let Some(rules) = arg.get("rules").and_then(|r| r.as_array()) {
                        let mut should_include = false;

                        for rule in rules {
                            if self.should_apply_rule(rule, features) {
                                should_include = true;
                                break;
                            }
                        }

                        if should_include {
                            if let Some(value) = arg.get("value") {
                                processed_args
                                    .extend(self.process_rule_values(value, placeholders));
                            }
                        }
                    }
                }
            }
        }

        processed_args
    }

    // Helper function to process game arguments from the manifest
    pub fn process_game_arguments(
        &self,
        manifest_json: &Value,
        account: &MinecraftAccount,
        game_dir: &Path,
        assets_dir: &Path,
        natives_dir: &Path,
        minecraft_version: &str,
        assets_index: &str,
    ) -> Vec<String> {
        // Create placeholder map for variable substitution
        let mut placeholders = HashMap::new();
        placeholders.insert(
            "auth_player_name".to_string(),
            account.username().to_string(),
        );
        placeholders.insert("version_name".to_string(), minecraft_version.to_string());
        placeholders.insert(
            "game_directory".to_string(),
            game_dir.to_string_lossy().to_string(),
        );
        placeholders.insert(
            "assets_root".to_string(),
            assets_dir.to_string_lossy().to_string(),
        );
        placeholders.insert("assets_index_name".to_string(), assets_index.to_string());
        placeholders.insert("auth_uuid".to_string(), account.uuid().to_string());
        placeholders.insert(
            "auth_access_token".to_string(),
            account.access_token().unwrap_or("null").to_string(),
        );
        placeholders.insert(
            "user_type".to_string(),
            if account.user_type() != "offline" {
                "mojang"
            } else {
                "legacy"
            }
            .to_string(),
        );
        placeholders.insert("version_type".to_string(), "release".to_string());
        placeholders.insert(
            "natives_directory".to_string(),
            natives_dir.to_string_lossy().to_string(),
        );
        placeholders.insert("launcher_name".to_string(), "modpackstore".to_string());
        placeholders.insert("launcher_version".to_string(), "1.0.0".to_string());

        // Define QuickPlay features (disabled by default)
        let mut features = HashMap::new();
        features.insert("has_custom_resolution".to_string(), false);
        features.insert("has_quick_plays_support".to_string(), false);
        features.insert("is_demo_user".to_string(), false);
        features.insert("is_quick_play_singleplayer".to_string(), false);
        features.insert("is_quick_play_multiplayer".to_string(), false);
        features.insert("is_quick_play_realms".to_string(), false);

        // Check for new-style arguments (1.13+)
        if let Some(args_obj) = manifest_json.get("arguments").and_then(|v| v.get("game")) {
            return self.process_arguments(args_obj, &placeholders, Some(&features));
        }

        // Legacy-style arguments (pre-1.13)
        if let Some(min_args) = manifest_json
            .get("minecraftArguments")
            .and_then(|v| v.as_str())
        {
            return min_args
                .split_whitespace()
                .map(|arg| self.replace_placeholders(arg, &placeholders))
                .collect();
        }

        // Fallback to hardcoded arguments for very old versions
        let mut arguments = vec![
            "--username".to_string(),
            placeholders["auth_player_name"].clone(),
            "--version".to_string(),
            placeholders["version_name"].clone(),
            "--gameDir".to_string(),
            placeholders["game_directory"].clone(),
            "--assetsDir".to_string(),
            placeholders["assets_root"].clone(),
        ];

        // Additional arguments for slightly newer but still old versions
        if !assets_index.is_empty() {
            arguments.extend(vec![
                "--assetIndex".to_string(),
                placeholders["assets_index_name"].clone(),
            ]);
        }

        arguments.extend(vec![
            "--uuid".to_string(),
            placeholders["auth_uuid"].clone(),
            "--accessToken".to_string(),
            placeholders["auth_access_token"].clone(),
            "--userType".to_string(),
            placeholders["user_type"].clone(),
        ]);

        arguments
    }

    // Process JVM arguments from the manifest
    pub fn process_jvm_arguments(
        &self,
        manifest_json: &Value,
        natives_dir: &Path,
        classpath_str: &str,
        mc_memory: u32,
    ) -> Vec<String> {
        // Create placeholder map for variable substitution
        let mut placeholders = HashMap::new();
        placeholders.insert(
            "natives_directory".to_string(),
            natives_dir.to_string_lossy().to_string(),
        );
        placeholders.insert("launcher_name".to_string(), "modpackstore".to_string());
        placeholders.insert("launcher_version".to_string(), "1.0.0".to_string());
        placeholders.insert("classpath".to_string(), classpath_str.to_string());

        // Base memory settings that should always be included
        let mut jvm_args = vec![format!("-Xms512M"), format!("-Xmx{}M", mc_memory)];

        // Check if there are JVM args in the manifest (modern format)
        if let Some(args_obj) = manifest_json.get("arguments").and_then(|v| v.get("jvm")) {
            // Process JVM args from manifest
            let manifest_args = self.process_arguments(args_obj, &placeholders, None);

            // Add all arguments from manifest that aren't already included
            let filtered_args: Vec<_> = manifest_args
                .into_iter()
                .filter(|arg| !jvm_args.contains(arg))
                .collect();
            jvm_args.extend(filtered_args);
        } else {
            // Legacy format: include all standard arguments
            jvm_args.extend(vec![
                format!("-Djava.library.path={}", natives_dir.display()),
                format!("-Dminecraft.launcher.brand=modpackstore"),
                format!("-Dminecraft.launcher.version=1.0.0"),
                format!("-Djna.tmpdir={}", natives_dir.display()),
                format!(
                    "-Dorg.lwjgl.system.SharedLibraryExtractPath={}",
                    natives_dir.display()
                ),
                format!("-Dio.netty.native.workdir={}", natives_dir.display()),
            ]);

            // Add OS-specific arguments for legacy versions
            if cfg!(target_os = "macos") {
                jvm_args.push("-XstartOnFirstThread".to_string());
            }

            if cfg!(windows) {
                jvm_args.push("-XX:HeapDumpPath=MojangTricksIntelDriversForPerformance_javaw.exe_minecraft.exe.heapdump".to_string());
            }

            if cfg!(target_arch = "x86") {
                jvm_args.push("-Xss1M".to_string());
            }
        }

        // Make sure classpath is included
        if !jvm_args
            .iter()
            .any(|arg| arg == "-cp" || arg == "-classpath")
        {
            jvm_args.push("-cp".to_string());
            jvm_args.push(classpath_str.to_string());
        }

        jvm_args
    }

    // Build the classpath from the manifest
    fn build_classpath(
        &self,
        manifest_json: &Value,
        client_jar: &Path,
        libraries_dir: &Path,
    ) -> String {
        let mut classpath = vec![client_jar.to_string_lossy().to_string()];
        let separator = if cfg!(windows) { ";" } else { ":" };

        if let Some(libs) = manifest_json.get("libraries").and_then(|v| v.as_array()) {
            for lib in libs {
                // Check if this library has rules that might exclude it
                let should_include = lib
                    .get("rules")
                    .and_then(|rules| rules.as_array())
                    .map(|rules_arr| {
                        rules_arr
                            .iter()
                            .any(|rule| self.should_apply_rule(rule, None))
                    })
                    .unwrap_or(true); // Default to include if no rules

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

        classpath.join(separator)
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
        let java_path = match &self.instance.javaPath {
            Some(path) => PathBuf::from(path),
            None => default_java_path,
        }
        .join("bin")
        .join(if cfg!(windows) { "java.exe" } else { "java" });

        println!("Java path: {}", java_path.display());

        let accounts_manager = AccountsManager::new();

        // If instance does not have an account, return None
        let account_uuid = match &self.instance.accountUuid {
            Some(uuid) => uuid,
            None => {
                println!("No account found for this instance.");
                return None;
            }
        };

        let account = match accounts_manager.get_minecraft_account_by_uuid(account_uuid) {
            Some(acct) => acct,
            None => {
                println!("Account not found for UUID: {}", account_uuid);
                MinecraftAccount::new(
                    "offline".to_string(),
                    Uuid::new_v4().to_string(),
                    None,
                    "offline".to_string(),
                )
            }
        };

        println!("Account: {:?}", account);

        // Get game directory
        let game_dir = match &self.instance.instanceDirectory {
            Some(dir) => PathBuf::from(dir).join("minecraft"),
            None => PathBuf::from("default_path").join("minecraft"),
        };

        if !game_dir.exists() {
            fs::create_dir_all(&game_dir).expect("Failed to create game directory");
        }

        let minecraft_version = self.instance.minecraftVersion.clone();

        let version_dir = game_dir.join("versions").join(&minecraft_version);
        let client_jar = version_dir.join(format!("{}.jar", minecraft_version));
        let natives_dir = game_dir.join("natives").join(&minecraft_version);
        let libraries_dir = game_dir.join("libraries");
        let assets_dir = game_dir.join("assets");
        let manifest_file = version_dir.join(format!("{}.json", minecraft_version));

        //  Extracted from siglauncher code, maybe we can handle all vanilla and forge in a single way

        /*
          let modded = !p["inheritsFrom"].is_null();
         if modded {
            let mut vanilla_json_content = String::new();

            let mut vanilla_json_file = match File::open(format!(
                "{}/versions/{}/{}.json",
                minecraft_dir,
                game_settings.game_version,
                p["inheritsFrom"].as_str().unwrap()
            )) {
                Ok(ok) => ok,
                Err(_) => panic!("no!!!"),
            };

            vanilla_json_file
                .read_to_string(&mut vanilla_json_content)
                .unwrap();

            let content = serde_json::from_str(&vanilla_json_content);
            p = content.unwrap();
        }
         */

        // Log paths for debugging
        log::info!("version_dir: {}", version_dir.display());
        log::info!("client_jar: {}", client_jar.display());
        log::info!("natives_dir: {}", natives_dir.display());
        log::info!("libraries_dir: {}", libraries_dir.display());
        log::info!("assets_dir: {}", assets_dir.display());
        log::info!("manifest_file: {}", manifest_file.display());
        log::info!("game_dir: {}", game_dir.display());

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
            Some(class) => class,
            None => {
                println!("Main class not found in manifest");
                return None;
            }
        };

        // Get assets index
        let assets_index = manifest_json
            .get("assets")
            .and_then(|v| v.as_str())
            .or_else(|| manifest_json.get("assetIndex")?.get("id")?.as_str())
            .unwrap_or("legacy");

        // Build classpath
        let classpath_str = self.build_classpath(&manifest_json, &client_jar, &libraries_dir);

        // Process game arguments from manifest
        let game_args = self.process_game_arguments(
            &manifest_json,
            &account,
            &game_dir,
            &assets_dir,
            &natives_dir,
            &minecraft_version,
            assets_index,
        );

        // Process JVM arguments from manifest
        let jvm_args =
            self.process_jvm_arguments(&manifest_json, &natives_dir, &classpath_str, mc_memory);

        // Build command
        let mut command = Command::new(&java_path);

        // Add JVM arguments
        command.args(&jvm_args);

        // Add main class
        command.arg(main_class);

        // Add game arguments
        command.args(&game_args);

        command.current_dir(&game_dir);
        println!("Command: {:?}", command);

        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());

        // Execute command
        match command.spawn() {
            Ok(child) => {
                println!("Spawned child process: {:?}", child.id());
                Some(child)
            }
            Err(e) => {
                println!("Failed to spawn Minecraft process: {}", e);
                None
            }
        }
    }
}
