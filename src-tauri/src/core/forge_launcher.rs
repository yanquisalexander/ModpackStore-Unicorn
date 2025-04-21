use serde_json::Value;
use std::{
    collections::{HashMap, HashSet},
    fs,
    io,
    path::{Path, PathBuf},
    process::{Child, Command},
    // Consider adding error handling crates like thiserror or anyhow
    error::Error, 
    fmt,
};

use crate::core::accounts_manager::AccountsManager;
use crate::core::{minecraft_account::MinecraftAccount, minecraft_instance::MinecraftInstance};
use crate::interfaces::game_launcher::GameLauncher;
use uuid::Uuid;
use std::os::windows::process::CommandExt;
use anyhow::{Context, Result};
use crate::config::get_config_manager;

// Define a simple error type for the launcher
#[derive(Debug)]
struct LaunchError(String);

impl fmt::Display for LaunchError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Error for LaunchError {}

impl From<io::Error> for LaunchError {
    fn from(err: io::Error) -> Self {
        LaunchError(format!("IO Error: {}", err))
    }
}

impl From<serde_json::Error> for LaunchError {
     fn from(err: serde_json::Error) -> Self {
        LaunchError(format!("JSON Error: {}", err))
    }
}


const CREATE_NO_WINDOW: u32 = 0x08000000;

pub struct ForgeLoader {
    instance: MinecraftInstance,
}

// --- Helper Functions ---

/// Reads and parses a JSON file.
fn read_json_manifest(path: &Path) -> Result<Value, LaunchError> {
    let content = fs::read_to_string(path)
        .map_err(|e| LaunchError(format!("Failed to read manifest {}: {}", path.display(), e)))?;
    serde_json::from_str(&content)
       .map_err(|e| LaunchError(format!("Failed to parse manifest {}: {}", path.display(), e)))
}

/// Merges two JSON manifests, typically a version manifest and its parent.
/// Values from `child` override values from `parent`.
/// Arrays like 'libraries' and 'arguments' are concatenated (simple merge).
fn merge_manifests(parent: &Value, child: &Value) -> Value {
    let mut merged = parent.clone();
    
    if let (Some(merged_obj), Some(child_obj)) = (merged.as_object_mut(), child.as_object()) {
        for (key, child_val) in child_obj {
            match key.as_str() {
                "libraries" | "arguments" => {
                    if let (Some(merged_arr), Some(child_arr)) = 
                        (merged_obj.get_mut(key).and_then(|v| v.as_array_mut()),
                         child_val.as_array())
                    {
                        // Append child elements to parent array
                        merged_arr.extend(child_arr.clone());
                    } else {
                        merged_obj.insert(key.clone(), child_val.clone());
                    }
                }
                _ => {
                    merged_obj.insert(key.clone(), child_val.clone());
                }
            }
        }
    }
    merged
}


/// Parses JVM or Game arguments from the manifest JSON.
fn parse_arguments(
    args_json: Option<&Value>,
    variables: &HashMap<String, String>,
) -> Result<Vec<String>, LaunchError> {
    let mut args = Vec::new();
    if let Some(json_val) = args_json {
        if let Some(arr) = json_val.as_array() {
            for element in arr {
                if let Some(s) = element.as_str() {
                    // Simple string argument, replace variables
                    args.push(replace_variables(s, variables));
                } else if let Some(obj) = element.as_object() {
                    // Argument with rules (simplified handling: include if no rules or rules allow)
                    // TODO: Implement proper rule evaluation (OS, features)
                    let allowed = if let Some(_rules) = obj.get("rules") {
                        // Placeholder: Assume allowed for now. Real implementation needs OS/feature checks.
                        true
                    } else {
                        true // No rules, always allowed
                    };

                    if allowed {
                        if let Some(value_node) = obj.get("value") {
                            if let Some(s_val) = value_node.as_str() {
                                args.push(replace_variables(s_val, variables));
                            } else if let Some(arr_val) = value_node.as_array() {
                                for sub_val in arr_val {
                                    if let Some(sub_s) = sub_val.as_str() {
                                        args.push(replace_variables(sub_s, variables));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        } else if let Some(s) = json_val.as_str() {
             // Handle the legacy "minecraftArguments" string
             args.extend(s.split_whitespace().map(|part| replace_variables(part, variables)));
        }
    }
    Ok(args)
}


/// Replaces placeholder variables in a string.
fn replace_variables(template: &str, variables: &HashMap<String, String>) -> String {
    let mut result = template.to_string();
    for (key, value) in variables {
        result = result.replace(key, value);
    }
    result
}

/// Converts Maven coordinates (e.g., "com.example:artifact:1.0") to a relative path.
fn convert_maven_to_path(name: &str) -> Option<PathBuf> {
    let parts: Vec<&str> = name.split(':').collect();
    if parts.len() < 3 {
        return None; // Invalid format
    }
    let group_path = parts[0].replace('.', &std::path::MAIN_SEPARATOR.to_string());
    let artifact = parts[1];
    let version = parts[2];
    // Handle potential classifier (e.g., natives-windows)
    let (version_base, classifier) = if parts.len() >= 4 {
        (parts[2], Some(parts[3]))
    } else {
        (parts[2], None)
    };

    let file_name = if let Some(cls) = classifier {
        format!("{}-{}-{}.jar", artifact, version_base, cls)
    } else {
        format!("{}-{}.jar", artifact, version_base)
    };

    Some(PathBuf::from(group_path).join(artifact).join(version).join(file_name))
}

impl ForgeLoader {
    pub fn new(instance: MinecraftInstance) -> Self {
        Self { instance }
    }

    // Builds the classpath string needed to launch Forge.
    fn build_classpath(
        &self,
        merged_manifest: &Value,
        libraries_dir: &Path,
        forge_jar_path: &Path, // Explicitly pass the correct Forge JAR
        base_minecraft_jar: &Path, // Add the base Minecraft JAR
    ) -> Result<String, LaunchError> {
        let mut classpath_entries = HashSet::new(); // Use HashSet to avoid duplicates
        let mut classpath_list = Vec::new();

        // 1. Add Base Minecraft JAR
        if base_minecraft_jar.exists() {
             let path_str = base_minecraft_jar.to_string_lossy().to_string();
             if classpath_entries.insert(path_str.clone()) {
                // Don't add base_minecraft_jar to classpath because it is not needed for Forge (Causes conflicts)
                // classpath_list.push(path_str);
             }
        } else {
             println!("Warning: Base Minecraft JAR not found at {}", base_minecraft_jar.display());
             // Decide if this is a fatal error depending on Forge version
             // return Err(LaunchError(format!("Base Minecraft JAR not found: {}", base_minecraft_jar.display())));
        }


        // 2. Add Forge JAR
        let forge_jar_str = forge_jar_path.to_string_lossy().to_string();
         if classpath_entries.insert(forge_jar_str.clone()) {
            classpath_list.push(forge_jar_str);
        }

        // 3. Add Libraries from Manifest
        if let Some(libs) = merged_manifest.get("libraries").and_then(|l| l.as_array()) {
            for lib in libs {
                // TODO: Implement proper rule evaluation based on OS/features
                 let should_include = if let Some(_rules) = lib.get("rules") {
                     // Simplified: include for now
                     true
                 } else {
                     true // No rules, include
                 };

                if !should_include {
                    continue;
                }

                let lib_path = if let Some(path_str) = lib
                    .get("downloads")
                    .and_then(|d| d.get("artifact"))
                    .and_then(|a| a.get("path"))
                    .and_then(|p| p.as_str())
                {
                    // New format with explicit path
                    libraries_dir.join(path_str.replace('/', &std::path::MAIN_SEPARATOR.to_string()))
                } else if let Some(name) = lib.get("name").and_then(|n| n.as_str()) {
                    // Old format: construct path from name
                    if let Some(relative_path) = convert_maven_to_path(name) {
                         libraries_dir.join(relative_path)
                    } else {
                        println!("Warning: Could not parse library name: {}", name);
                        continue;
                    }
                } else {
                     println!("Warning: Library entry missing 'downloads.artifact.path' and 'name'");
                    continue;
                };


                if lib_path.exists() {
                     let path_str = lib_path.to_string_lossy().to_string();
                     if classpath_entries.insert(path_str.clone()) { // Add only if not already present
                        classpath_list.push(path_str);
                     }
                } else {
                    // Log missing library but don't necessarily fail
                    println!("Warning: Library not found: {}", lib_path.display());
                    // TODO: Potentially try finding alternative versions like in Java code (more complex)
                }
            }
        }

        // Join with the OS-specific separator
        let separator = if cfg!(windows) { ";" } else { ":" };
        Ok(classpath_list.join(separator))
    }

    fn collect_jvm_args(
        version_id: &str,
        versions_dir: &Path,
        variables: &HashMap<String,String>
    ) -> anyhow::Result<Vec<String>> {
        // Leer el manifest de `version_id`
        let manifest_path = versions_dir.join(version_id).join(format!("{}.json", version_id));
        let manifest = read_json_manifest(&manifest_path)?;
        let mut args = Vec::new();
    
        // 1) Si hereda de otra versión, recórrela primero
        if let Some(parent) = manifest
            .get("inheritsFrom")
            .and_then(|v| v.as_str())
        {
            args.extend(Self::collect_jvm_args(parent, versions_dir, variables)?);
        }
    
        // 2) Parsear y añadir los jvm-args de este manifest
        if let Some(jvm_section) = manifest
            .get("arguments")
            .and_then(|a| a.get("jvm"))
        {
            let parsed = parse_arguments(Some(jvm_section), variables)?;
            args.extend(parsed);
        }
    
        Ok(args)
    }
    

    // --- Main Launch Logic ---
    fn launch_internal(&self) -> Result<Child, LaunchError> {
        // --- 1. Setup Paths and Versions ---
        let config_lock = get_config_manager()
        .lock()
        .expect("Failed to lock config manager mutex");
    
    let config = config_lock
        .as_ref()
        .expect("Config manager failed to initialize");
    
    let java_path = config.get_java_dir()
        .ok_or_else(|| LaunchError("Java path is not set".to_string()))?
        .join("bin")
        .join(if cfg!(windows) { "java.exe" } else { "java" });
        
    
        if !java_path.exists() {
            return Err(LaunchError(format!("Java executable not found at {}", java_path.display())));
        }
    
        // --- Account ---
        let account_uuid_str = self.instance.accountUuid.as_ref().ok_or_else(|| LaunchError("Instance has no associated account UUID".to_string()))?;
        let accounts_manager = AccountsManager::new(); // Assuming this can't fail or handles errors internally
        let account = accounts_manager
            .get_minecraft_account_by_uuid(account_uuid_str)
            .ok_or_else(|| LaunchError(format!("Account not found for UUID: {}", account_uuid_str)))?;
    
        println!("Using Account: {}", account.username());
    
        // --- Directories ---
        let instance_base_dir = self.instance.instanceDirectory.as_ref().map(PathBuf::from)
            .ok_or_else(|| LaunchError("Instance directory is not set".to_string()))?;
        // Use 'minecraft' subdir convention unless instance path already points there or similar
        let game_dir = instance_base_dir.join("minecraft"); // Adjusted logic to use instance base dir directly
    
        let versions_dir = game_dir.join("versions");
        let libraries_dir = game_dir.join("libraries");
        let assets_dir = game_dir.join("assets");
    
        fs::create_dir_all(&game_dir)
            .map_err(|e| LaunchError(format!("Failed to create game directory {}: {}", game_dir.display(), e)))?;
        fs::create_dir_all(&versions_dir)?;
        fs::create_dir_all(&libraries_dir)?;
        fs::create_dir_all(&assets_dir)?; // Ensure assets dir exists
    
    
        // --- Determine Forge Version and Manifest Path ---
        let minecraft_version = &self.instance.minecraftVersion;
        let forge_version = self.instance.forgeVersion.as_ref()
            .ok_or_else(|| LaunchError("Forge version is not specified for instance".to_string()))?;
    
        // Try both common naming conventions for Forge manifests/dirs
        let modern_forge_name = format!("{}-{}", minecraft_version, forge_version);
        let legacy_forge_name = format!("{}-forge-{}", minecraft_version, forge_version); // Some older installers used this
    
        let mut forge_manifest_path = versions_dir.join(&modern_forge_name).join(format!("{}.json", modern_forge_name));
        let mut version_id = modern_forge_name.clone(); // The ID used in paths/jars
    
        if !forge_manifest_path.exists() {
            println!("Modern Forge manifest not found, trying legacy name: {}", legacy_forge_name);
            let legacy_path = versions_dir.join(&legacy_forge_name).join(format!("{}.json", legacy_forge_name));
            if legacy_path.exists() {
                forge_manifest_path = legacy_path;
                version_id = legacy_forge_name;
            } else {
                return Err(LaunchError(format!(
                    "Forge manifest not found at either {} or {}",
                    forge_manifest_path.display(), legacy_path.display()
                )));
            }
        }
        let version_dir = forge_manifest_path.parent().unwrap().to_path_buf(); // Safe unwrap after exists check
    
        println!("Using Forge manifest: {}", forge_manifest_path.display());
    
        // --- 2. Read and Merge Manifests ---
        let forge_manifest = read_json_manifest(&forge_manifest_path)?;
    
        // Handle inheritance
        let mut merged_manifest = forge_manifest.clone(); // Start with the Forge manifest
        if let Some(inherits_from) = forge_manifest.get("inheritsFrom").and_then(|v| v.as_str()) {
            println!("Manifest inherits from: {}", inherits_from);
            let base_manifest_path = versions_dir.join(inherits_from).join(format!("{}.json", inherits_from));
            if base_manifest_path.exists() {
                let base_manifest = read_json_manifest(&base_manifest_path)?;
                // Merge base into forge manifest (forge values override base)
                merged_manifest = merge_manifests(&base_manifest, &forge_manifest);
                println!("Successfully merged with base manifest: {}", base_manifest_path.display());
            } else {
                println!("Warning: Inherited base manifest not found: {}", base_manifest_path.display());
                // Decide if this is fatal. Often it is.
                // return Err(LaunchError(format!("Inherited base manifest not found: {}", base_manifest_path.display())));
            }
        }
    
        // --- 3. Locate Key Files (JARs) ---
        let base_minecraft_version_id = merged_manifest.get("inheritsFrom") // If inherited...
            .and_then(|v| v.as_str())                                       // get base version id
            .unwrap_or(&minecraft_version);                                // otherwise use current MC version
        let base_minecraft_jar = versions_dir
            .join(base_minecraft_version_id)
            .join(format!("{}.jar", base_minecraft_version_id));
    
        // Find Forge JAR - check version dir first, then libraries
        let mut forge_jar_path = version_dir.join(format!("{}.jar", version_id));
    
        if !forge_jar_path.exists() {
            // Try library location (modern Forge)
            let forge_lib_path_rel = format!("net/minecraftforge/forge/{}/forge-{}-client.jar",
                                             modern_forge_name, // Modern usually uses the simpler name
                                             modern_forge_name);
            let forge_lib_path_abs = libraries_dir.join(forge_lib_path_rel.replace('/', &std::path::MAIN_SEPARATOR.to_string()));
    
            println!("Forge JAR not in version dir, checking library path: {}", forge_lib_path_abs.display());
    
            if forge_lib_path_abs.exists() {
                forge_jar_path = forge_lib_path_abs;
            } else {
                return Err(LaunchError(format!(
                    "Forge JAR not found in version directory ({}) or libraries ({})",
                    forge_jar_path.display(), forge_lib_path_abs.display()
                )));
            }
        }
        println!("Using Forge JAR: {}", forge_jar_path.display());
    
    
        // --- 4. Extract Launch Information ---
        let main_class = merged_manifest
            .get("mainClass")
            .and_then(|v| v.as_str())
            .ok_or_else(|| LaunchError("Main class not found in manifest".to_string()))?;
    
        println!("Main class: {}", main_class);
    
        // Asset index: Check "assetIndex.id" first (newer format), then "assets" (older)
        let assets_index = merged_manifest
            .get("assetIndex")
            .and_then(|v| v.get("id"))
            .and_then(|v| v.as_str())
            .or_else(|| merged_manifest.get("assets").and_then(|v| v.as_str()))
            .unwrap_or("legacy"); // Default if not found
    
        // Create a standard natives directory - may be overridden by arguments later
        let natives_dir = game_dir.join("natives").join(&version_id); 
        fs::create_dir_all(&natives_dir)?;
    
    
        // --- 5. Build Classpath ---
        let classpath_str = self.build_classpath(&merged_manifest, &libraries_dir, &forge_jar_path, &base_minecraft_jar)?;
    
        // --- 6. Prepare Variables for Argument Replacement ---
        let mut variables = HashMap::new();
        variables.insert("${auth_player_name}".to_string(), account.username().to_string());
        variables.insert("${version_name}".to_string(), version_id.clone());
        variables.insert("${game_directory}".to_string(), game_dir.to_string_lossy().to_string());
        variables.insert("${assets_root}".to_string(), assets_dir.to_string_lossy().to_string());
        variables.insert("${assets_index_name}".to_string(), assets_index.to_string());
        variables.insert("${auth_uuid}".to_string(), account.uuid().to_string());
        
        // Handle offline mode / missing token safely
        let binding = account.access_token();
        let access_token = binding.as_deref().filter(|&s| !s.is_empty() && s != "null").unwrap_or("0"); // Use "0" or similar placeholder if offline
        variables.insert("${auth_access_token}".to_string(), access_token.to_string());
        variables.insert("${user_type}".to_string(), if account.user_type() == "offline" { "legacy".to_string() } else { "mojang".to_string() });
        variables.insert("${version_type}".to_string(), "release".to_string());
        variables.insert("${natives_directory}".to_string(), natives_dir.to_string_lossy().to_string());
        variables.insert("${library_directory}".to_string(), libraries_dir.to_string_lossy().to_string());
        variables.insert("${classpath_separator}".to_string(), if cfg!(windows) { ";".to_string() } else { ":".to_string() });
        variables.insert("${classpath}".to_string(), classpath_str.clone());
        variables.insert("${launcher_name}".to_string(), "Modpack Store".to_string());
        variables.insert("${launcher_version}".to_string(), "1.0".to_string());
        variables.insert("${minecraft_jar}".to_string(), base_minecraft_jar.to_string_lossy().to_string());
    
    
        // --- 7. Build Command ---
        let mut command = Command::new(&java_path);
        command.current_dir(&game_dir); // Set working directory
    
        // --- JVM Arguments ---
        // Track what arguments have been added to avoid duplicates
        let mut added_args = HashSet::new();
        let mut final_args = Vec::new();
    
        // First, add JVM args from manifest - prioritize these
        if let Some(jvm_args_section) = merged_manifest.get("arguments").and_then(|a| a.get("jvm")) {
            let mut manifest_jvm_args = Self::collect_jvm_args(&version_id, &versions_dir, &variables)
            .map_err(|e| LaunchError(format!("Error recogiendo JVM args: {}", e)))?;
        
        // 2) Evitar duplicados salvo excepciones
        let allowed_duplicates = ["--add-opens", "--add-exports"];
        let macos_specific_args = ["-XstartOnFirstThread"];
        for arg in manifest_jvm_args.drain(..) {
            if macos_specific_args.contains(&arg.as_str()) && !cfg!(target_os = "macos") {
                continue;
            }

            let key = arg.split_whitespace().next().unwrap_or(&arg).to_string();
            if !added_args.contains(&key) || allowed_duplicates.contains(&key.as_str()) {
                final_args.push(arg.clone());
                added_args.insert(key);
            }
        }
        }

        
    
        // Add essential JVM args if not already present in the manifest
        if !added_args.contains("-xms") && !added_args.contains("-xmx") {
            final_args.push("-Xms512M".to_string());
            final_args.push("-Xmx2G".to_string());
            added_args.insert("-xms".to_string());
            added_args.insert("-xmx".to_string());
        }
    
        // Ensure java.library.path is set if not in manifest
        let library_path_key = "-djava.library.path".to_string();
        if !added_args.contains(&library_path_key) {
            final_args.push(format!("-Djava.library.path={}", natives_dir.to_string_lossy()));
            added_args.insert(library_path_key);
        }
    
        // Add launcher brand/version if not present
        let brand_key = "-dminecraft.launcher.brand".to_string();
        if !added_args.contains(&brand_key) {
            final_args.push(format!("-Dminecraft.launcher.brand={}", variables.get("${launcher_name}").unwrap()));
            added_args.insert(brand_key);
        }
        
        let version_key = "-dminecraft.launcher.version".to_string();
        if !added_args.contains(&version_key) {
            final_args.push(format!("-Dminecraft.launcher.version={}", variables.get("${launcher_version}").unwrap()));
            added_args.insert(version_key);
        }
    
        // Add classpath only if not already present
        let cp_key = "-cp".to_string();
        if !added_args.contains(&cp_key) {
            final_args.push("-cp".to_string());
            final_args.push(variables.get("${classpath}").unwrap().clone());
            added_args.insert(cp_key);
        }
    
        // Add main class as the final JVM argument
        final_args.push(main_class.to_string());
        println!("Main class added to JVM arguments: {}", main_class);

        // Justo antes de añadir los argumentos al comando
println!("Final JVM Arguments: {:?}", final_args);

    
        // Apply all JVM args
        for arg in final_args {
            command.arg(arg);
        }
    
        // --- Game Arguments ---
        let mut game_args = Vec::new();
        let mut game_arg_keys = HashSet::new();
        
        // Parse game arguments from manifest
        let game_args_node = merged_manifest.get("arguments")
            .and_then(|a| a.get("game"))
            .or_else(|| merged_manifest.get("minecraftArguments")); // Fallback to legacy
            
        if let Some(node) = game_args_node {
            let parsed_game_args = parse_arguments(Some(node), &variables)?;
            
            // Process paired arguments (--flag value)
            let mut i = 0;
            while i < parsed_game_args.len() {
                let arg = &parsed_game_args[i];
                
                if arg.starts_with("--") {
                    // It's a flag
                    let key = arg.to_lowercase();
                    if !game_arg_keys.contains(&key) {
                        game_args.push(arg.clone());
                        game_arg_keys.insert(key);
                        
                        // Add the value if available
                        if i + 1 < parsed_game_args.len() && !parsed_game_args[i + 1].starts_with("--") {
                            game_args.push(parsed_game_args[i + 1].clone());
                            i += 1; // Skip the value in next iteration
                        }
                    }
                } else if i == 0 || !parsed_game_args[i-1].starts_with("--") {
                    // It's not a value for a preceding flag
                    game_args.push(arg.clone());
                }
                
                i += 1;
            }
        }
    
        // Add essential game arguments if missing
        let required_args = [
            ("--username", "${auth_player_name}"),
            ("--version", "${version_name}"),
            ("--gameDir", "${game_directory}"),
            ("--assetsDir", "${assets_root}"),
            ("--assetIndex", "${assets_index_name}"),
            ("--uuid", "${auth_uuid}"),
            ("--accessToken", "${auth_access_token}"),
            ("--userType", "${user_type}"),
            ("--versionType", "${version_type}")
        ];
    
        for (flag, var_name) in required_args {
            if !game_arg_keys.contains(&flag.to_lowercase()) {
                game_args.push(flag.to_string());
                game_args.push(variables.get(var_name).unwrap_or(&"".to_string()).clone());
                game_arg_keys.insert(flag.to_lowercase());
            }
        }
    
        // Add legacy --tweakClass for old Forge versions if needed
        if minecraft_version.starts_with("1.") {
            let mc_minor_version = minecraft_version.split('.').nth(1).unwrap_or("0").parse::<u32>().unwrap_or(99);
            if mc_minor_version < 13 && !game_arg_keys.contains("--tweakclass") {
                println!("Adding legacy tweak class for MC Version {}", minecraft_version);
                game_args.push("--tweakClass".to_string());
                game_args.push("net.minecraftforge.fml.common.launcher.FMLTweaker".to_string());
            }
        }
    
        // Apply all game args
        for arg in game_args {
            command.arg(arg);
        }
    
        // --- 8. Launch ---
        println!("Assembled Command: {:?}", command);
    
        if cfg!(windows) {
            command.creation_flags(CREATE_NO_WINDOW);
        }
    
        match command.spawn() {
            Ok(child) => {
                println!("Spawned Forge process with PID: {}", child.id());
                Ok(child)
            }
            Err(e) => {
                println!("Failed to spawn Forge process: {}", e);
                Err(LaunchError(format!("Failed to spawn process: {}", e)))
            }
        }
    }

   /// Finds bootstrap module JARs in the libraries directory for the module path.
/// This is required for modern Forge (1.17+) which uses the Java module system.
fn find_bootstrap_modules(libraries_dir: &Path) -> Result<String, LaunchError> {
    // These are common patterns for bootstrap modules required by Forge
    let bootstrap_patterns = [
        "cpw/mods/bootstraplauncher",
        "cpw/mods/securejarhandler",
        "org/ow2/asm/asm",
        "org/ow2/asm/asm-commons",
        "org/ow2/asm/asm-util",
        "org/ow2/asm/asm-analysis",
        "org/ow2/asm/asm-tree",
        "net/minecraftforge/JarJarFileSystems"
    ];
    
    let mut module_jars = Vec::new();
    
    // Define a recursive function instead of a closure
    fn walk_directory(dir: &Path, patterns: &[&str], jars: &mut Vec<String>) -> io::Result<()> {
        if dir.is_dir() {
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();
                
                if path.is_dir() {
                    walk_directory(&path, patterns, jars)?;
                } else if let Some(ext) = path.extension() {
                    if ext == "jar" {
                        // Check if this JAR matches any of our bootstrap patterns
                        let path_str = path.to_string_lossy();
                        for pattern in patterns {
                            if path_str.contains(&pattern.replace('/', &std::path::MAIN_SEPARATOR.to_string())) {
                                jars.push(path.to_string_lossy().to_string());
                                break;
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
    
    // Start the recursive search from the libraries directory
    if let Err(e) = walk_directory(libraries_dir, &bootstrap_patterns, &mut module_jars) {
        return Err(LaunchError(format!("Failed to scan libraries directory: {}", e)));
    }
    
    // If no bootstrap JARs were found, log a warning but don't necessarily fail
    if module_jars.is_empty() {
        println!("Warning: No bootstrap module JARs found in libraries directory");
    } else {
        println!("Found {} bootstrap module JARs", module_jars.len());
    }
    
    // Join the paths with the appropriate separator for the platform
    let separator = if cfg!(windows) { ";" } else { ":" };
    Ok(module_jars.join(separator))
}

    fn is_modern_minecraft_version(version: &str) -> bool {
        // Extract the major and minor version numbers
        let version_parts: Vec<&str> = version.split('.').collect();
        
        if version_parts.is_empty() {
            return false; // Invalid version format
        }
        
        // Handle "1.x" format versions
        if version_parts[0] == "1" && version_parts.len() > 1 {
            // Try to parse the minor version number
            if let Ok(minor) = version_parts[1].parse::<u32>() {
                // Modern Forge (requiring Java modules) started with Minecraft 1.17
                return minor >= 17;
            }
        } 
        // Handle potential future "2.x" or higher versions (assume they're modern)
        else if let Ok(major) = version_parts[0].parse::<u32>() {
            return major >= 2;
        }
        
        false // Default to false for unrecognized formats
    }
}


impl GameLauncher for ForgeLoader {
    fn launch(&self) -> Option<Child> {
        // Wrap the internal logic to match the trait signature
        match self.launch_internal() {
            Ok(child) => Some(child),
            Err(e) => {
                eprintln!("Error launching Forge instance: {}", e); // Log error
                None
            }
        }
    }
}