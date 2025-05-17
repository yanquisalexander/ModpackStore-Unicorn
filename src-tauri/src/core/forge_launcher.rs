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
use std::os::windows::process::CommandExt;
use uuid::Uuid;

// Import VanillaLauncher for inheritance
use super::vanilla_launcher::VanillaLauncher;

const CREATE_NO_WINDOW: u32 = 0x08000000;

pub struct ForgeLoader {
    instance: MinecraftInstance,
    vanilla_launcher: VanillaLauncher, // Composition with VanillaLauncher for reusing functionality
}

impl ForgeLoader {
    pub fn new(instance: MinecraftInstance) -> Self {
        let vanilla_launcher = VanillaLauncher::new(instance.clone());
        Self {
            instance,
            vanilla_launcher,
        }
    }

    // Get the parent (vanilla) version from the Forge manifest
    fn get_inherits_from(&self, forge_manifest_json: &Value) -> Option<String> {
        println!("Checking for inheritsFrom field in Forge manifest");
        forge_manifest_json
            .get("inheritsFrom")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

    // Build the classpath combining both Forge and Vanilla libraries
    fn build_forge_classpath(
        &self,
        forge_manifest_json: &Value,
        vanilla_manifest_json: &Value,
        forge_client_jar: &Path,
        vanilla_client_jar: &Path,
        libraries_dir: &Path,
    ) -> String {
        let separator = if cfg!(windows) { ";" } else { ":" };
        let mut classpath = Vec::new();
        let mut processed_libraries = HashMap::new(); // Para rastrear bibliotecas y sus versiones

        // Comenzar con el jar cliente de Forge
        classpath.push(forge_client_jar.to_string_lossy().to_string());

        // Añadir el jar cliente de Vanilla (comentado, pero disponible si se necesita posteriormente)
        // classpath.push(vanilla_client_jar.to_string_lossy().to_string());

        // Función auxiliar para extraer identificador único de biblioteca con grupo, artefacto y versión
        fn parse_library_info(lib_name: &str) -> Option<(String, String, String)> {
            let parts: Vec<&str> = lib_name.split(':').collect();
            if parts.len() >= 3 {
                let group_id = parts[0].to_string();
                let artifact_id = parts[1].to_string();
                let version = parts[2].split('@').next().unwrap_or(parts[2]).to_string();
                return Some((group_id, artifact_id, version));
            }
            None
        }

        // Función auxiliar para extraer identificador único de biblioteca (artifact_id:version)
        fn get_library_id(lib: &Value) -> Option<(String, String, String)> {
            if let Some(name) = lib.get("name").and_then(|name| name.as_str()) {
                return parse_library_info(name);
            }
            None
        }

        // Función auxiliar para procesar biblioteca y añadirla a la classpath
        fn process_library(
            lib: &Value,
            libraries_dir: &Path,
            classpath: &mut Vec<String>,
            launcher: &ForgeLoader,
        ) -> Option<String> {
            // Comprobar si esta biblioteca tiene reglas que podrían excluirla
            let should_include = lib
                .get("rules")
                .and_then(|rules| rules.as_array())
                .map(|rules_arr| {
                    rules_arr
                        .iter()
                        .any(|rule| launcher.vanilla_launcher.should_apply_rule(rule, None))
                })
                .unwrap_or(true); // Por defecto incluir si no hay reglas

            if !should_include {
                return None;
            }

            // Intentar obtener la ruta de la biblioteca desde downloads
            if let Some(path) = lib
                .get("downloads")
                .and_then(|downloads| downloads.get("artifact"))
                .and_then(|artifact| artifact.get("path"))
                .and_then(|path| path.as_str())
            {
                let lib_path =
                    libraries_dir.join(path.replace('/', &std::path::MAIN_SEPARATOR.to_string()));
                if lib_path.exists() {
                    classpath.push(lib_path.to_string_lossy().to_string());
                    return Some(lib_path.to_string_lossy().to_string());
                } else {
                    println!("Library not found: {}", lib_path.display());
                }
            }
            // Formato antiguo - construir la ruta a partir del nombre
            else if let Some(name) = lib.get("name").and_then(|name| name.as_str()) {
                // Parse Maven coordinates
                if let Some((group_id, artifact_id, version_raw)) = parse_library_info(name) {
                    let group_path = group_id.replace('.', &std::path::MAIN_SEPARATOR.to_string());

                    // Manejar el formato personalizado de Forge con el símbolo @
                    let (version, classifier) = if version_raw.contains('@') {
                        let v_parts: Vec<&str> = version_raw.split('@').collect();
                        (v_parts[0], Some(v_parts[1]))
                    } else {
                        (&version_raw[..], None)
                    };

                    // Construir el nombre de archivo con clasificador opcional
                    let filename = if let Some(classifier) = classifier {
                        format!("{}-{}-{}.jar", artifact_id, version, classifier)
                    } else {
                        format!("{}-{}.jar", artifact_id, version)
                    };

                    let lib_path = libraries_dir
                        .join(group_path)
                        .join(&artifact_id)
                        .join(version)
                        .join(&filename);

                    if lib_path.exists() {
                        classpath.push(lib_path.to_string_lossy().to_string());
                        return Some(lib_path.to_string_lossy().to_string());
                    } else {
                        println!("Legacy library not found: {}", lib_path.display());
                    }
                }
            }
            None
        }

        // Primer paso: Procesar TODAS las bibliotecas de Forge y almacenar sus artifact_id:version
        if let Some(forge_libs) = forge_manifest_json
            .get("libraries")
            .and_then(|v| v.as_array())
        {
            for lib in forge_libs {
                if let Some((group_id, artifact_id, version)) = get_library_id(lib) {
                    let lib_key = format!("{}:{}", artifact_id, version);
                    // Marcar esta biblioteca como "de Forge" con prioridad alta
                    processed_libraries.insert(lib_key, ("forge", lib.clone()));
                }
            }
        }

        // Segundo paso: Procesar bibliotecas de Vanilla, solo registrando las que no tienen equivalente en Forge
        if let Some(vanilla_libs) = vanilla_manifest_json
            .get("libraries")
            .and_then(|v| v.as_array())
        {
            for lib in vanilla_libs {
                if let Some((group_id, artifact_id, version)) = get_library_id(lib) {
                    let lib_key = format!("{}:{}", artifact_id, version);
                    // Solo insertar si no existe una versión de Forge
                    if !processed_libraries.contains_key(&lib_key) {
                        // Para bibliotecas con mismo artifact_id pero diferente versión, verificar si ya hay una versión de Forge
                        let forge_version_exists = processed_libraries.keys().any(|key| {
                            key.starts_with(&format!("{}:", artifact_id))
                                && processed_libraries.get(key).unwrap().0 == "forge"
                        });

                        if !forge_version_exists {
                            processed_libraries.insert(lib_key, ("vanilla", lib.clone()));
                        } else {
                            // Existe una versión de Forge de esta biblioteca, omitir la de Vanilla
                            println!(
                                "Skipping Vanilla library {} as Forge version exists",
                                artifact_id
                            );
                        }
                    }
                }
            }
        }

        // Ahora procesamos las bibliotecas en orden: primero Forge, luego Vanilla
        let mut forge_paths = Vec::new();
        let mut vanilla_paths = Vec::new();

        for (_, (source, lib)) in processed_libraries.iter() {
            let result = process_library(lib, libraries_dir, &mut classpath, self);
            if let Some(path) = result {
                if *source == "forge" {
                    forge_paths.push(path);
                } else {
                    vanilla_paths.push(path);
                }
            }
        }

        // Combinar las rutas en el orden correcto: Forge primero, luego Vanilla
        classpath.clear();
        classpath.push(forge_client_jar.to_string_lossy().to_string());
        classpath.extend(forge_paths);
        classpath.extend(vanilla_paths);

        println!("Loaded {} libraries:", classpath.len());
        for path in &classpath {
            println!(" - {}", path);
        }

        classpath.join(separator)
    }

    // Extract natives for both Forge and Vanilla
    fn extract_natives(
        &self,
        forge_manifest_json: &Value,
        vanilla_manifest_json: &Value,
        libraries_dir: &Path,
        natives_dir: &Path,
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!("Extracting natives to: {}", natives_dir.display());

        // Create natives directory if it doesn't exist
        if !natives_dir.exists() {
            fs::create_dir_all(&natives_dir)?;
        }

        // Track extracted files to avoid duplicates
        let mut extracted_files = HashSet::new();

        // Helper function to extract natives from a library
        let mut extract_library_natives = |lib: &Value| -> Result<(), Box<dyn std::error::Error>> {
            // Check OS specific rules
            let current_os = if cfg!(windows) {
                "windows"
            } else if cfg!(target_os = "macos") {
                "osx"
            } else if cfg!(target_os = "linux") {
                "linux"
            } else {
                return Ok(()); // Unsupported OS
            };

            // Skip if library doesn't apply to current OS
            let should_apply = lib
                .get("rules")
                .and_then(|rules| rules.as_array())
                .map(|rules_arr| {
                    rules_arr.iter().all(|rule| {
                        let action = rule
                            .get("action")
                            .and_then(|a| a.as_str())
                            .unwrap_or("allow");
                        let os_name = rule
                            .get("os")
                            .and_then(|os| os.get("name"))
                            .and_then(|n| n.as_str());

                        match (action, os_name) {
                            ("allow", None) => true,
                            ("allow", Some(name)) => name == current_os,
                            ("disallow", Some(name)) => name != current_os,
                            _ => true,
                        }
                    })
                })
                .unwrap_or(true);

            if !should_apply {
                return Ok(());
            }

            // Check if library has natives
            let natives_key = format!("natives-{}", current_os);
            let has_natives = lib
                .get("natives")
                .and_then(|natives| {
                    natives
                        .get(current_os)
                        .or_else(|| natives.get(&natives_key))
                })
                .is_some();

            if !has_natives {
                return Ok(());
            }

            // Try to find the path to the native JAR
            let native_path = if let Some(downloads) = lib.get("downloads") {
                if let Some(classifiers) = downloads.get("classifiers") {
                    let classifier_key = format!("natives-{}", current_os);
                    if let Some(native_info) = classifiers.get(&classifier_key) {
                        if let Some(path) = native_info.get("path").and_then(|p| p.as_str()) {
                            let jar_path = libraries_dir
                                .join(path.replace('/', &std::path::MAIN_SEPARATOR.to_string()));
                            if jar_path.exists() {
                                Some(jar_path)
                            } else {
                                println!("Native JAR not found: {}", jar_path.display());
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                // Fallback to legacy format
                if let Some(name) = lib.get("name").and_then(|n| n.as_str()) {
                    if let Some((group_id, artifact_id, version)) = Self::parse_library_info(name) {
                        let group_path =
                            group_id.replace('.', &std::path::MAIN_SEPARATOR.to_string());
                        let classifier = match current_os {
                            "windows" => "natives-windows",
                            "osx" => "natives-osx",
                            "linux" => "natives-linux",
                            _ => return Ok(()),
                        };

                        let native_jar = format!("{}-{}-{}.jar", artifact_id, version, classifier);
                        let jar_path = libraries_dir
                            .join(group_path)
                            .join(&artifact_id)
                            .join(version)
                            .join(&native_jar);

                        if jar_path.exists() {
                            Some(jar_path)
                        } else {
                            println!("Legacy native JAR not found: {}", jar_path.display());
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            };

            // Extract the native JAR if found
            if let Some(jar_path) = native_path {
                println!("Extracting natives from: {}", jar_path.display());

                let file = File::open(&jar_path)?;
                let mut archive = ZipArchive::new(file)?;

                for i in 0..archive.len() {
                    let mut file = archive.by_index(i)?;
                    let outpath = match file.enclosed_name() {
                        Some(path) => path.to_owned(),
                        None => continue,
                    };

                    // Skip directories and unwanted files
                    let file_name = outpath.file_name().unwrap().to_string_lossy();
                    if file_name.contains("META-INF")
                        || file_name.contains("MANIFEST.MF")
                        || file_name.ends_with(".git")
                        || file_name.ends_with(".sha1")
                        || file_name.ends_with(".md5")
                    {
                        continue;
                    }

                    // Only extract DLL, SO, and DYLIB files
                    let is_native_file = file_name.ends_with(".dll")
                        || file_name.ends_with(".so")
                        || file_name.ends_with(".dylib")
                        || file_name.ends_with(".jnilib");

                    if is_native_file {
                        // Construct output path
                        let mut outfile = natives_dir.join(file_name.as_ref());

                        // Skip if already extracted
                        if extracted_files.contains(&file_name.to_string()) {
                            continue;
                        }

                        println!("  Extracting: {}", file_name);

                        if let Some(p) = outfile.parent() {
                            if !p.exists() {
                                fs::create_dir_all(p)?;
                            }
                        }

                        let mut outfile_writer = File::create(&outfile)?;
                        std::io::copy(&mut file, &mut outfile_writer)?;

                        extracted_files.insert(file_name.to_string());
                    }
                }
            }

            Ok(())
        };

        // First process Forge libraries
        if let Some(forge_libs) = forge_manifest_json
            .get("libraries")
            .and_then(|v| v.as_array())
        {
            for lib in forge_libs {
                if let Err(e) = extract_library_natives(lib) {
                    println!("Error extracting Forge natives: {}", e);
                }
            }
        }

        // Then process Vanilla libraries
        if let Some(vanilla_libs) = vanilla_manifest_json
            .get("libraries")
            .and_then(|v| v.as_array())
        {
            for lib in vanilla_libs {
                if let Err(e) = extract_library_natives(lib) {
                    println!("Error extracting Vanilla natives: {}", e);
                }
            }
        }

        println!(
            "Natives extraction complete. Extracted {} files",
            extracted_files.len()
        );
        Ok(())
    }

    // Helper function to parse library info
    fn parse_library_info(lib_name: &str) -> Option<(String, String, String)> {
        let parts: Vec<&str> = lib_name.split(':').collect();
        if parts.len() >= 3 {
            let group_id = parts[0].to_string();
            let artifact_id = parts[1].to_string();
            let version = parts[2].split('@').next().unwrap_or(parts[2]).to_string();
            return Some((group_id, artifact_id, version));
        }
        None
    }

    // Process Forge-specific JVM arguments
    fn process_forge_jvm_arguments(
        &self,
        forge_manifest_json: &Value,
        vanilla_manifest_json: &Value,
        natives_dir: &Path,
        classpath_str: &str,
        mc_memory: u32,
    ) -> Vec<String> {
        // First get the vanilla JVM args
        let mut jvm_args = self.vanilla_launcher.process_jvm_arguments(
            vanilla_manifest_json,
            natives_dir,
            classpath_str,
            mc_memory,
        );

        // Create placeholder map for variable substitution
        let mut placeholders = HashMap::new();
        placeholders.insert(
            "natives_directory".to_string(),
            natives_dir.to_string_lossy().to_string(),
        );
        placeholders.insert("launcher_name".to_string(), "modpackstore".to_string());
        placeholders.insert("launcher_version".to_string(), "1.0.0".to_string());
        placeholders.insert("classpath".to_string(), classpath_str.to_string());

        // Check for Forge-specific arguments
        if let Some(args_obj) = forge_manifest_json
            .get("arguments")
            .and_then(|v| v.get("jvm"))
        {
            // Process JVM args from Forge manifest
            let forge_args = self
                .vanilla_launcher
                .process_arguments(args_obj, &placeholders, None);

            // Add Forge args that aren't already in vanilla args
            for arg in forge_args {
                if !jvm_args.contains(&arg) {
                    jvm_args.push(arg);
                }
            }
        }

        // Asegúrate de que está configurado el directorio de natives en las opciones JVM
        let mut has_natives_dir = false;
        for (i, arg) in jvm_args.iter().enumerate() {
            if arg == "-Djava.library.path" && i + 1 < jvm_args.len() {
                has_natives_dir = true;
                jvm_args[i + 1] = natives_dir.to_string_lossy().to_string();
                break;
            }
        }

        if !has_natives_dir {
            jvm_args.push("-Djava.library.path".to_string());
            jvm_args.push(natives_dir.to_string_lossy().to_string());
        }

        // Remove any existing classpath arguments
        jvm_args
            .retain(|arg| arg != "-cp" && arg != "-classpath" && !arg.starts_with(classpath_str));

        // Add updated classpath
        jvm_args.push("-cp".to_string());
        jvm_args.push(classpath_str.to_string());

        jvm_args
    }

    // Process Forge-specific game arguments
    fn process_forge_game_arguments(
        &self,
        forge_manifest_json: &Value,
        vanilla_manifest_json: &Value,
        account: &MinecraftAccount,
        game_dir: &Path,
        assets_dir: &Path,
        natives_dir: &Path,
        minecraft_version: &str,
        forge_version: &str,
        assets_index: &str,
    ) -> Vec<String> {
        // Create placeholder map for variable substitution
        let mut placeholders = HashMap::new();
        placeholders.insert(
            "auth_player_name".to_string(),
            account.username().to_string(),
        );
        placeholders.insert("version_name".to_string(), forge_version.to_string());
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
        placeholders.insert("vanilla_version".to_string(), minecraft_version.to_string());

        // Define QuickPlay features (disabled by default)
        let mut features = HashMap::new();
        features.insert("has_custom_resolution".to_string(), false);
        features.insert("has_quick_plays_support".to_string(), false);
        features.insert("is_demo_user".to_string(), false);
        features.insert("is_quick_play_singleplayer".to_string(), false);
        features.insert("is_quick_play_multiplayer".to_string(), false);
        features.insert("is_quick_play_realms".to_string(), false);
        features.insert("has_forge".to_string(), true); // Add Forge-specific feature flag

        // First get the vanilla game arguments
        let mut game_args = self.vanilla_launcher.process_game_arguments(
            vanilla_manifest_json,
            account,
            game_dir,
            assets_dir,
            natives_dir,
            minecraft_version,
            assets_index,
        );

        // Check for Forge-specific arguments (modern format)
        if let Some(args_obj) = forge_manifest_json
            .get("arguments")
            .and_then(|v| v.get("game"))
        {
            let forge_args =
                self.vanilla_launcher
                    .process_arguments(args_obj, &placeholders, Some(&features));

            // Merge arguments, taking care of duplicates
            // Índice para recorrer los argumentos en pares
            let mut i = 0;
            while i < forge_args.len() {
                let key = &forge_args[i];
                if i + 1 < forge_args.len() {
                    let value = &forge_args[i + 1];

                    // Verifica si el par clave-valor ya existe
                    let mut found = false;
                    for j in (0..game_args.len()).step_by(2) {
                        if j + 1 < game_args.len()
                            && game_args[j] == *key
                            && game_args[j + 1] == *value
                        {
                            found = true;
                            break;
                        }
                    }
                    if !found {
                        game_args.push(key.clone());
                        game_args.push(value.clone());
                    }
                    i += 2; // Avanza dos posiciones (clave y valor)
                } else {
                    // Si hay un argumento impar (sin valor), agrégalo tal cual
                    if !game_args.contains(key) {
                        game_args.push(key.clone());
                    }
                    i += 1;
                }
            }
        }
        // Check for legacy Forge arguments
        else if let Some(min_args) = forge_manifest_json
            .get("minecraftArguments")
            .and_then(|v| v.as_str())
        {
            // Replace vanilla args with forge args as they're meant to be complete
            game_args = min_args
                .split_whitespace()
                .map(|arg| {
                    self.vanilla_launcher
                        .replace_placeholders(arg, &placeholders)
                })
                .collect();
        }

        // Add Forge-specific tweaker/transformer arguments if needed
        if let Some(tweakers) = forge_manifest_json
            .get("tweakers")
            .and_then(|v| v.as_array())
        {
            for tweaker in tweakers {
                if let Some(tweaker_class) = tweaker.as_str() {
                    game_args.push("--tweakClass".to_string());
                    game_args.push(tweaker_class.to_string());
                }
            }
        }

        game_args
    }

    // Helper method to determine if this is a modern Forge instance (1.13+)
    fn is_modern_forge(&self, forge_manifest_json: &Value) -> bool {
        // Modern Forge versions have the "arguments" structure instead of "minecraftArguments"
        forge_manifest_json.get("arguments").is_some()
    }
}

impl GameLauncher for ForgeLoader {
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

        let forge_version = self.instance.forgeVersion.clone().unwrap_or_else(|| {
            println!("Forge version is not set");
            return "default_forge_version".to_string();
        });

        // Forge version folder and manifest pattern: mcVersion-forge-forgeVersion
        let version_dir = game_dir
            .join("versions")
            .join(format!("{}-forge-{}", minecraft_version, forge_version));
        let forge_client_jar =
            version_dir.join(format!("{}-forge-{}.jar", minecraft_version, forge_version));
        let natives_dir = game_dir.join("natives").join(&minecraft_version); // Uses the same natives directory as vanilla
        let libraries_dir = game_dir.join("libraries");
        let assets_dir = game_dir.join("assets");
        let forge_manifest_file = version_dir.join(format!(
            "{}-forge-{}.json",
            minecraft_version, forge_version
        ));

        // Read and parse the Forge JSON manifest
        let forge_manifest_data = match fs::read_to_string(&forge_manifest_file) {
            Ok(content) => content,
            Err(e) => {
                println!("Failed to read Forge version manifest file: {}", e);
                return None;
            }
        };

        let forge_manifest_json: Value = match serde_json::from_str(&forge_manifest_data) {
            Ok(json) => json,
            Err(e) => {
                println!("Failed to parse Forge version manifest JSON: {}", e);
                return None;
            }
        };

        // Get the vanilla version that this Forge version inherits from
        let vanilla_version = match self.get_inherits_from(&forge_manifest_json) {
            Some(version) => version,
            None => {
                println!("No inheritsFrom field found in Forge manifest");
                return None;
            }
        };

        println!(
            "Forge version: {} inherits from vanilla version: {}",
            forge_version, vanilla_version
        );

        // Set up vanilla-related paths
        let vanilla_version_dir = game_dir.join("versions").join(&vanilla_version);
        let vanilla_client_jar = vanilla_version_dir.join(format!("{}.jar", vanilla_version));
        let vanilla_manifest_file = vanilla_version_dir.join(format!("{}.json", vanilla_version));

        // Read and parse the vanilla JSON manifest
        let vanilla_manifest_data = match fs::read_to_string(&vanilla_manifest_file) {
            Ok(content) => content,
            Err(e) => {
                println!("Failed to read vanilla version manifest file: {}", e);
                return None;
            }
        };

        let vanilla_manifest_json: Value = match serde_json::from_str(&vanilla_manifest_data) {
            Ok(json) => json,
            Err(e) => {
                println!("Failed to parse vanilla version manifest JSON: {}", e);
                return None;
            }
        };

        // Get main class from Forge manifest
        let main_class = match forge_manifest_json
            .get("mainClass")
            .and_then(|v| v.as_str())
        {
            Some(class) => class,
            None => {
                println!("Main class not found in Forge manifest");
                return None;
            }
        };

        // Get assets index from vanilla manifest
        let assets_index = vanilla_manifest_json
            .get("assets")
            .and_then(|v| v.as_str())
            .or_else(|| vanilla_manifest_json.get("assetIndex")?.get("id")?.as_str())
            .unwrap_or("legacy");

        // Build classpath combining Forge and vanilla libraries
        let classpath_str = self.build_forge_classpath(
            &forge_manifest_json,
            &vanilla_manifest_json,
            &forge_client_jar,
            &vanilla_client_jar,
            &libraries_dir,
        );

        // Process JVM arguments from both manifests
        let jvm_args = self.process_forge_jvm_arguments(
            &forge_manifest_json,
            &vanilla_manifest_json,
            &natives_dir,
            &classpath_str,
            mc_memory,
        );

        // Process game arguments from both manifests
        let game_args = self.process_forge_game_arguments(
            &forge_manifest_json,
            &vanilla_manifest_json,
            &account,
            &game_dir,
            &assets_dir,
            &natives_dir,
            &vanilla_version,
            &forge_version,
            assets_index,
        );

        // Check if this is a modern Forge instance
        let is_modern = self.is_modern_forge(&forge_manifest_json);
        println!(
            "Using {} Forge format",
            if is_modern { "modern" } else { "legacy" }
        );

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

        if cfg!(windows) {
            // On Windows, use creation flags to avoid console window
            command.creation_flags(CREATE_NO_WINDOW);
        }

        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());

        // Execute command
        match command.spawn() {
            Ok(child) => {
                println!("Spawned Forge child process: {:?}", child.id());
                Some(child)
            }
            Err(e) => {
                println!("Failed to spawn Forge process: {}", e);
                None
            }
        }
    }
}
