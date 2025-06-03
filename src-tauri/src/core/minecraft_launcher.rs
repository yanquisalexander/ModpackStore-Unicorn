use crate::config::get_config_manager;
use crate::core::accounts_manager::AccountsManager;
use crate::core::{minecraft_account::MinecraftAccount, minecraft_instance::MinecraftInstance};
use crate::interfaces::game_launcher::GameLauncher;
use serde_json::{Map, Value};
use std::collections::HashSet;
use std::collections::{BTreeMap, HashMap};
use std::path::MAIN_SEPARATOR;
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
};
use uuid::Uuid;

pub struct MinecraftLauncher {
    instance: MinecraftInstance,
}

impl MinecraftLauncher {
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

    // Build the classpath from the manifest, removing duplicates robustly
    fn build_classpath(
        &self,
        manifest_json: &Value,
        client_jar: &Path,
        libraries_dir: &Path,
    ) -> String {
        let mut entries = Vec::new();
        // 1) Incluir el JAR del cliente
        let client_path = client_jar.to_string_lossy().to_string();
        entries.push(client_path.clone());

        // 2) Para evitar duplicados
        let mut seen: HashSet<String> = HashSet::new();
        seen.insert(client_path);

        // Separador de classpath según OS
        let sep = if cfg!(windows) { ";" } else { ":" };

        // Función auxiliar para añadir si no existe
        let mut add_if_new = |path: &Path| {
            if path.exists() {
                let s = path.to_string_lossy().to_string();
                if seen.insert(s.clone()) {
                    entries.push(s);
                }
            }
        };

        // Procesar todas las librerías del manifest
        if let Some(libs) = manifest_json.get("libraries").and_then(|v| v.as_array()) {
            for lib in libs {
                // Reglas de inclusión opcionales
                let include = lib
                    .get("rules")
                    .and_then(|r| r.as_array())
                    .map(|rules| rules.iter().any(|rule| self.should_apply_rule(rule, None)))
                    .unwrap_or(true);
                if !include {
                    continue;
                }

                // 2.1) Artifact genérico
                if let Some(path_val) = lib
                    .get("downloads")
                    .and_then(|d| d.get("artifact"))
                    .and_then(|a| a.get("path"))
                    .and_then(Value::as_str)
                {
                    let jar =
                        libraries_dir.join(path_val.replace('/', &MAIN_SEPARATOR.to_string()));
                    add_if_new(&jar);
                }

                // 2.2) Classifiers nativos
                if let Some(classifiers) = lib
                    .get("downloads")
                    .and_then(|d| d.get("classifiers"))
                    .and_then(Value::as_object)
                {
                    // Elegir classifier según OS
                    let os_classifier = if cfg!(windows) {
                        "natives-windows"
                    } else if cfg!(target_os = "linux") {
                        "natives-linux"
                    } else {
                        // macOS
                        "natives-macos"
                    };
                    if let Some(info) = classifiers.get(os_classifier) {
                        if let Some(path_val) = info.get("path").and_then(Value::as_str) {
                            let native_jar = libraries_dir
                                .join(path_val.replace('/', &MAIN_SEPARATOR.to_string()));
                            add_if_new(&native_jar);
                        }
                    }
                }
            }
        }

        // Unir en una cadena final
        entries.join(sep)
    }

    // Load and merge vanilla and forge manifests if needed
    fn load_merged_manifest(&self, game_dir: &Path, minecraft_version: &str) -> Option<Value> {
        let version_dir = game_dir.join("versions").join(minecraft_version);
        let manifest_file = version_dir.join(format!("{}.json", minecraft_version));

        log::info!("Loading version manifest from {}", manifest_file.display());

        // Read modded manifest file
        let manifest_data = match fs::read_to_string(&manifest_file) {
            Ok(content) => content,
            Err(e) => {
                println!("Failed to read version manifest file: {}", e);
                return None;
            }
        };

        let mut manifest_json: Value = match serde_json::from_str(&manifest_data) {
            Ok(json) => json,
            Err(e) => {
                println!("Failed to parse version manifest JSON: {}", e);
                return None;
            }
        };

        // Check if this is a modded instance that inherits from vanilla
        if let Some(inherits_from) = manifest_json.get("inheritsFrom").and_then(|v| v.as_str()) {
            println!("Found modded instance inheriting from {}", inherits_from);

            // Load vanilla manifest
            let vanilla_version_dir = game_dir.join("versions").join(inherits_from);
            let vanilla_manifest_file = vanilla_version_dir.join(format!("{}.json", inherits_from));

            let vanilla_manifest_data = match fs::read_to_string(&vanilla_manifest_file) {
                Ok(content) => content,
                Err(e) => {
                    println!("Failed to read vanilla manifest file: {}", e);
                    return Some(manifest_json); // Return modded manifest only if vanilla can't be found
                }
            };

            let vanilla_manifest: Value = match serde_json::from_str(&vanilla_manifest_data) {
                Ok(json) => json,
                Err(e) => {
                    println!("Failed to parse vanilla manifest JSON: {}", e);
                    return Some(manifest_json); // Return modded manifest only if vanilla can't be parsed
                }
            };

            // Merge manifests
            return Some(self.merge_manifests(vanilla_manifest, manifest_json));
        }

        // Return the original manifest if it's not modded or doesn't inherit
        Some(manifest_json)
    }

    /// Merge vanilla and forge manifests, ensuring arguments are combined correctly

    pub fn merge_manifests(&self, vanilla: Value, forge: Value) -> Value {
        fn extract_info(
            lib: &Value,
        ) -> Option<(
            String,
            String,
            Option<String>,
            Option<String>,
            Option<String>,
        )> {
            let name = lib.get("name")?.as_str()?.to_string();
            let parts: Vec<&str> = name.split(':').collect();
            let ga = if parts.len() >= 2 {
                format!("{}:{}", parts[0], parts[1])
            } else {
                name.clone()
            };
            let version = parts.get(2).map(|s| s.to_string());
            let classifier = lib
                .get("downloads")
                .and_then(|d| d.get("artifact"))
                .and_then(|a| a.get("classifier"))
                .or_else(|| lib.get("classifier"))
                .and_then(Value::as_str)
                .map(String::from);
            let url = lib.get("url").and_then(Value::as_str).map(String::from);
            Some((name, ga, version, url, classifier))
        }

        // Defines preference for libraries when merging.
        // Generally, Forge (child) versions are preferred over Vanilla (parent).
        // Specific version comparison can be added if needed (e.g. for log4j or other critical libs).
        fn prefer_child_version(lib_ga_key: &str, _parent_ver: &Option<String>, child_ver: &Option<String>) -> bool {
            log::debug!("[ManifestMerge] Library '{}' found in both manifests. Preferring child version ('{}').", lib_ga_key, child_ver.as_deref().unwrap_or("N/A"));
            // Currently, always prefer the child's definition if it exists.
            // Version comparison logic (like the old log4j specific one) could be added here if necessary.
            // For example, to ensure only newer versions from child are preferred:
            // if let (Some(p_ver_str), Some(c_ver_str)) = (parent_ver, child_ver) {
            //    // Implement version comparison, e.g. simple lexicographical or semver
            //    return c_ver_str >= p_ver_str;
            // }
            true // Default to preferring child if one version is None or comparison is not implemented
        }

        let mut result = vanilla.clone();
        if let Some(mc) = forge.get("mainClass") {
            result["mainClass"] = mc.clone();
        }

        let mut libs: BTreeMap<String, Value> = BTreeMap::new();
        let mut duplicates: HashMap<String, Vec<String>> = HashMap::new();

        if let Some(arr) = vanilla.get("libraries").and_then(Value::as_array) {
            for lib in arr {
                if let Some((_, ga, vver, _, classifier)) = extract_info(lib) {
                    let key = if let Some(c) = &classifier {
                        format!("{}:{}", ga, c)
                    } else {
                        ga.clone()
                    };
                    if libs.contains_key(&key) {
                        duplicates
                            .entry(ga.clone())
                            .or_default()
                            .push(format!("vanilla:{}", vver.clone().unwrap_or_default()));
                    } else {
                        libs.insert(key, lib.clone());
                    }
                }
            }
        }

        if let Some(arr) = forge.get("libraries").and_then(Value::as_array) {
            for lib in arr {
                if let Some((_, ga, fver, furl, classifier)) = extract_info(lib) {
                    let key = if let Some(c) = &classifier {
                        format!("{}:{}", ga, c)
                    } else {
                        ga.clone()
                    };

                    if let Some(existing) = libs.get(&key) {
                        let (_, _, parent_ver, _, _) = extract_info(existing).expect("Existing library info malformed");

                        // Log that we are considering an override.
                        log::info!(
                            "[ManifestMerge] Library '{}' (version: {:?}) from child manifest is overriding version ({:?}) from parent manifest.",
                            ga, fver, parent_ver
                        );

                        if prefer_child_version(&ga, &parent_ver, &fver) {
                            libs.insert(key, lib.clone()); // Child's definition overrides parent's
                        }
                        // If prefer_child_version returns false, the parent's version remains.
                        // Add child version to duplicates for logging.
                        duplicates.entry(ga.clone()).or_default().push(format!("child:{}", fver.as_deref().unwrap_or("N/A")));

                    } else {
                        // Library only in child manifest, add it.
                        libs.insert(key, lib.clone());
                        duplicates.entry(ga.clone()).or_default().push(format!("child:{}", fver.as_deref().unwrap_or("N/A")));
                    }
                }
            }
        }

        // Log actual duplicates that were resolved or noted.
        for (ga, versions) in duplicates.iter() {
            if versions.len() > 1 { // Only log if there were actually multiple versions considered
                 let chosen_lib_info = libs.values().find(|lib_val| {
                    if let Some((_, lib_ga, _, _, lib_classifier)) = extract_info(lib_val) {
                        let current_key_base = lib_ga;
                        let current_key = if let Some(c) = lib_classifier { format!("{}:{}", current_key_base, c) } else { current_key_base };
                        // This check needs to be more robust if classifiers are involved in the `ga` passed to this loop
                        return current_key.starts_with(ga) || ga.starts_with(&current_key); // Approximate match
                    }
                    false
                });
                let chosen_version_str = if let Some(chosen_lib) = chosen_lib_info {
                    if let Some((_, _, cv, _, _)) = extract_info(chosen_lib) {
                        cv.unwrap_or_else(|| "N/A".to_string())
                    } else { "N/A".to_string() }
                } else { "N/A".to_string() };

                log::info!("[ManifestMerge] Library '{}': Versions considered [{}]. Version chosen: '{}'.", ga, versions.join(", "), chosen_version_str);
            }
        }


        result["libraries"] = Value::Array(libs.into_values().collect());

        let mut args_map = Map::default();
        for kind in &["game", "jvm"] {
            let mut list = Vec::new();
            if let Some(v) = vanilla
                .get("arguments")
                .and_then(|a| a.get(kind))
                .and_then(Value::as_array)
            {
                list.extend(v.clone());
            }
            if let Some(f) = forge
                .get("arguments")
                .and_then(|a| a.get(kind))
                .and_then(Value::as_array)
            {
                list.extend(f.clone());
            }
            if !list.is_empty() {
                args_map.insert(kind.to_string(), Value::Array(list));
            }
        }
        if !args_map.is_empty() {
            result["arguments"] = Value::Object(args_map);
        }

        let mut kv = HashMap::new();
        for src in [
            vanilla.get("minecraftArguments"),
            forge.get("minecraftArguments"),
        ] {
            if let Some(Value::String(s)) = src {
                for pair in s.split_whitespace().collect::<Vec<_>>().chunks(2) {
                    if let [k, v] = pair {
                        kv.insert(k.to_string(), v.to_string());
                    }
                }
            }
        }
        if !kv.is_empty() {
            let merged_legacy = kv
                .into_iter()
                .map(|(k, v)| format!("{} {}", k, v))
                .collect::<Vec<_>>()
                .join(" ");
            result["minecraftArguments"] = Value::String(merged_legacy);
        }

        result
    }

    // Get the appropriate client JAR path based on whether it's modded or vanilla
    fn get_client_jar_path(
        &self,
        game_dir: &Path,
        manifest_json: &Value,
        minecraft_version: &str, // This should be the base vanilla version for the .jar file.
        _forge_version: Option<&str>, // Parameter kept for signature compatibility, but not used if logic is simplified.
    ) -> PathBuf {
        // This function should return the path to the base Minecraft client JAR.
        // The `minecraft_version` parameter is assumed to be the base version (e.g., "1.19.2").
        // Forge specific JARs are treated as libraries and included in the classpath via `build_classpath`.
        let version_dir = game_dir
            .join("versions")
            .join(minecraft_version); // Use the passed `minecraft_version`

        let client_jar_name = format!("{}.jar", minecraft_version);
        let client_jar_path = version_dir.join(client_jar_name);

        log::debug!("[ClientJAR] Determined client JAR path: {}", client_jar_path.display());
        client_jar_path
        /*  // Old logic, potentially problematic if `self.instance.minecraftVersion` wasn't the base version.
              // The `minecraft_version` parameter to this function should be the one to use for the JAR.
        let version_dir = game_dir
            .join("versions")
            .join(self.instance.minecraftVersion.clone());
        let client_jar = version_dir.join(format!("{}.jar", self.instance.minecraftVersion));
        return client_jar;

        // Check if this is a modded instance
        if manifest_json.get("inheritsFrom").is_some() {
            // client_jar is not on version folder
            // get it from the libraries folder
            // "C:\Users\alexb\ModpackStore\Instances\forgi16\minecraft\libraries\net\minecraftforge\forge\1.16.2-33.0.61\forge-1.16.2-33.0.61-client.jar"
            if let Some(forge_version) = &self.instance.forgeVersion {
                let libraries_path = game_dir.join("libraries").join(format!(
                    "net/minecraftforge/forge/{}/forge-{}-client.jar",
                    minecraft_version, forge_version
                ));
                return libraries_path;
            }
            // For modded instances, the client JAR is usually in the libraries folder
            let client_jar = game_dir.join("libraries").join(format!(
                "net/minecraftforge/forge/{}/forge-{}-client.jar",
                minecraft_version,
                forge_version.unwrap_or("")
            ));
            return client_jar;
        } else {
            // For vanilla, just use the standard path
            let version_dir = game_dir.join("versions").join(minecraft_version);
            return version_dir.join(format!("{}.jar", minecraft_version));
        } */
    }
}

impl GameLauncher for MinecraftLauncher {
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
        .join(if cfg!(windows) { "javaw.exe" } else { "java" });

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

        let vanilla_mc_version = self.instance.minecraftVersion.clone();
        let mut minecraft_version = self.instance.minecraftVersion.clone();

        // Check if this is a Forge instance
        let is_forge = self.instance.forgeVersion.is_some();
        if is_forge {
            println!("Detected Forge version: {:?}", self.instance.forgeVersion);

            let launcher_profiles_file = game_dir.join("launcher_profiles.json");
            // Get "forge" version from launcher_profiles.json

            if launcher_profiles_file.exists() {
                let mut file = fs::File::open(&launcher_profiles_file)
                    .expect("Failed to open launcher_profiles.json");
                let mut contents = String::new();
                file.read_to_string(&mut contents)
                    .expect("Failed to read launcher_profiles.json");

                // Parse the JSON and extract the Forge version
                let json: Value = serde_json::from_str(&contents).unwrap();
                if let Some(profiles) = json.get("profiles") {
                    if let Some(forge_profile) = profiles.get("forge") {
                        if let Some(version) = forge_profile.get("lastVersionId") {
                            minecraft_version = version
                                .as_str()
                                .unwrap_or(minecraft_version.as_str())
                                .to_string();
                        }
                    }
                }
            }
        }

        let natives_dir = game_dir.join("natives").join(&vanilla_mc_version);
        let libraries_dir = game_dir.join("libraries");
        let assets_dir = game_dir.join("assets");

        println!("Launching Minecraft with the following directories:");
        println!("Libraries directory: {}", libraries_dir.display());
        println!("Assets directory: {}", assets_dir.display());
        println!("Game directory: {}", game_dir.display());

        // Load and possibly merge manifests
        let manifest_json = match self.load_merged_manifest(&game_dir, &minecraft_version) {
            Some(json) => json,
            None => {
                println!("Failed to load or merge manifests");
                return None;
            }
        };

        // Get the appropriate client jar path
        // The `minecraft_version` here should be the base vanilla version (e.g. "1.12.2")
        // because even for Forge, the vanilla JAR is typically the one named `version.jar`.
        // Forge's own JARs are loaded as libraries.
        let client_jar = self.get_client_jar_path(
            &game_dir,
            &manifest_json, // manifest_json is the merged one if forge
            &vanilla_mc_version, // Always use the base vanilla version for the client JAR name.
            self.instance.forgeVersion.as_deref(),
        );

        // Log paths for debugging
        println!("client_jar: {}", client_jar.display());
        println!("natives_dir: {}", natives_dir.display());
        println!("libraries_dir: {}", libraries_dir.display());
        println!("assets_dir: {}", assets_dir.display());
        println!("game_dir: {}", game_dir.display());

        // Validate required files and directories
        for (desc, path) in &[
            ("Client JAR", &client_jar),
            ("Natives", &natives_dir),
            ("Libraries", &libraries_dir),
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
        log::info!("Launching Minecraft with command: {:?}", command);
        log::info!("Arguments: {:?}", command.get_args());

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
