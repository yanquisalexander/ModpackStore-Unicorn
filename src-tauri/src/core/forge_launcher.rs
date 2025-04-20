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

    if let (Some(merged_map), Some(child_map)) = (merged.as_object_mut(), child.as_object()) {
        for (key, child_value) in child_map {
            if let Some(parent_value) = merged_map.get_mut(key) {
                // Simple merge strategy: child overrides parent, except for specific keys
                match key.as_str() {
                    "libraries" | "arguments" => {
                        // For arrays like libraries or structured arguments, concatenation might be needed,
                        // but simple override or more complex merging might be required depending on format.
                        // Here, we'll just override for simplicity, but Java code might concatenate.
                        // A more robust merge would be needed for perfect Java parity.
                        *parent_value = child_value.clone();
                    }
                    _ => {
                         *parent_value = child_value.clone();
                    }
                }
            } else {
                merged_map.insert(key.clone(), child_value.clone());
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
                 classpath_list.push(path_str);
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


    // --- Main Launch Logic ---
    fn launch_internal(&self) -> Result<Child, LaunchError> {
        // --- 1. Setup Paths and Versions ---
        let config_manager = crate::utils::config_manager::get_config_manager();
        let java_path = config_manager
            .lock()
            .expect("Failed to lock config manager mutex") // Keep expect for critical locks? Or convert to Result
            .get_java_dir()
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
        // Natives directory often derived later from arguments

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

        // Asset index: Check "assetIndex.id" first (newer format), then "assets" (older)
        let assets_index = merged_manifest
            .get("assetIndex")
            .and_then(|v| v.get("id"))
            .and_then(|v| v.as_str())
            .or_else(|| merged_manifest.get("assets").and_then(|v| v.as_str()))
            .unwrap_or("legacy"); // Default if not found

         // Natives directory needs placeholder replacement later
         let natives_dir_placeholder = "${natives_directory}"; // Standard placeholder
         // Actual natives dir path determined during argument processing or set explicitly
         // For simplicity, let's create a default path now, assuming it will be overridden if needed by JVM args
         let natives_dir = game_dir.join("natives").join(&version_id); // Common pattern
         fs::create_dir_all(&natives_dir)?;


        // --- 5. Build Classpath ---
        let classpath_str = self.build_classpath(&merged_manifest, &libraries_dir, &forge_jar_path, &base_minecraft_jar)?;

        // --- 6. Prepare Variables for Argument Replacement ---
        let mut variables = HashMap::new();
        // Corrección: Convertir todos los valores a String para tener un HashMap<String, String> consistente
        variables.insert("${auth_player_name}".to_string(), account.username().to_string());
        variables.insert("${version_name}".to_string(), version_id.clone()); // Use the actual version ID found
        variables.insert("${game_directory}".to_string(), game_dir.to_string_lossy().to_string());
        variables.insert("${assets_root}".to_string(), assets_dir.to_string_lossy().to_string());
        variables.insert("${assets_index_name}".to_string(), assets_index.to_string());
        variables.insert("${auth_uuid}".to_string(), account.uuid().to_string());
        // Handle offline mode / missing token safely
        let binding = account.access_token();
        let access_token = binding.as_deref().filter(|&s| !s.is_empty() && s != "null").unwrap_or("0"); // Use "0" or similar placeholder if offline
        variables.insert("${auth_access_token}".to_string(), access_token.to_string());
        variables.insert("${user_type}".to_string(), if account.user_type() == "offline" { "legacy".to_string() } else { "mojang".to_string() }); // Or "msa" depending on account type
        variables.insert("${version_type}".to_string(), "release".to_string()); // Commonly "release"
        variables.insert("${natives_directory}".to_string(), natives_dir.to_string_lossy().to_string());
        variables.insert("${library_directory}".to_string(), libraries_dir.to_string_lossy().to_string());
        variables.insert("${classpath_separator}".to_string(), if cfg!(windows) { ";".to_string() } else { ":".to_string() });
        variables.insert("${classpath}".to_string(), classpath_str.clone());
        variables.insert("${launcher_name}".to_string(), "rust_launcher".to_string()); // Or your launcher's name
        variables.insert("${launcher_version}".to_string(), "1.0".to_string()); // Or your launcher's version
        // Add minecraft_jar path if needed by some arguments (less common now)
        variables.insert("${minecraft_jar}".to_string(), base_minecraft_jar.to_string_lossy().to_string());


        // --- 7. Build Command ---
        let mut command = Command::new(&java_path);
        command.current_dir(&game_dir); // Set working directory

        // --- JVM Arguments ---
        let mut jvm_args_set = HashSet::new(); // Track args added to avoid duplicates/conflicts
        let mut final_jvm_args = Vec::new();

        // Add JVM args from manifest first
        if let Some(jvm_section) = merged_manifest.get("arguments").and_then(|a| a.get("jvm")) {
            let manifest_jvm_args = parse_arguments(Some(jvm_section), &variables)?;
            for arg in manifest_jvm_args {
                 // Simple check to avoid adding duplicates of the same argument flag
                 let arg_key = arg.split('=').next().unwrap_or(&arg).to_lowercase();
                 if jvm_args_set.insert(arg_key) {
                     final_jvm_args.push(arg);
                 }
            }
        } else {
            // Add very basic default JVM args if none in manifest
            if jvm_args_set.insert("-xms512m".to_string()) { // Track by lowercasing key
                final_jvm_args.push("-Xms512M".to_string());
            }
        }

        // Ensure required JVM args are present if not added by manifest
        let java_library_path_arg = format!("-Djava.library.path={}", variables.get("${natives_directory}").unwrap());
        if jvm_args_set.insert("-djava.library.path".to_string()) {
             final_jvm_args.push(java_library_path_arg);
        }

        // Add launcher brand/version if not present
        if jvm_args_set.insert("-dminecraft.launcher.brand".to_string()) {
             final_jvm_args.push(format!("-Dminecraft.launcher.brand={}", variables.get("${launcher_name}").unwrap()));
        }
         if jvm_args_set.insert("-dminecraft.launcher.version".to_string()) {
             final_jvm_args.push(format!("-Dminecraft.launcher.version={}", variables.get("${launcher_version}").unwrap()));
         }

        // Add modern Forge specific JVM args (adapt version check as needed)
        let is_modern_forge = minecraft_version.starts_with("1.17") || minecraft_version.starts_with("1.18") || minecraft_version.starts_with("1.19") || minecraft_version.starts_with("1.20") || minecraft_version.starts_with("1.21"); // Example check
        if is_modern_forge {
            println!("Adding Modern Forge JVM arguments...");
            // Add arguments similar to the Java example - these are often crucial
            if jvm_args_set.insert("-dignorelist".to_string()){
                final_jvm_args.push("-DignoreList=bootstraplauncher,securejarhandler,asm-commons,asm-util,asm-analysis,asm-tree,asm,JarJarFileSystems,client-extra,fmlcore,javafmllanguage,lowcodelanguage,mclanguage,forge-,*1.*".to_string() + minecraft_version); // Adjust list as needed
            }
             if jvm_args_set.insert("-dmergemodules".to_string()){
                 final_jvm_args.push("-DmergeModules=jna,jna-platform,java-objc-bridge,jopt-simple,kotlin-stdlib,failureaccess,guava".to_string()); // Add more from Java example if needed
             }
             if jvm_args_set.insert("-dlibrarydirectory".to_string()){
                 final_jvm_args.push(format!("-DlibraryDirectory={}", variables.get("${library_directory}").unwrap()));
             }

            // Module path arguments (-p, --add-modules, --add-opens) - These are complex
            // Finding bootstrap modules requires scanning libraries dir for specific JARs
            // Example: command.arg("-p"); command.arg(find_bootstrap_modules(&libraries_dir)?);
            // command.arg("--add-modules"); command.arg("ALL-MODULE-PATH");
            // command.arg("--add-opens=java.base/java.util.jar=cpw.mods.securejarhandler"); // etc.
            println!("Warning: Modern Forge module path arguments (-p, --add-modules, --add-opens) are complex and not fully implemented here.");

        }

        // Add Classpath and Main Class
        final_jvm_args.push("-cp".to_string());
        final_jvm_args.push(variables.get("${classpath}").unwrap().clone()); // Use variable for consistency
        final_jvm_args.push(main_class.to_string());


        // Apply all JVM args
        for arg in final_jvm_args {
            command.arg(arg);
        }


        // --- Game Arguments ---
         let mut game_args_set = HashSet::new(); // Track args added
         let mut final_game_args = Vec::new();

        // Add Game args from manifest first (new 'arguments.game' or legacy 'minecraftArguments')
         let game_args_node = merged_manifest.get("arguments").and_then(|a| a.get("game"))
             .or_else(|| merged_manifest.get("minecraftArguments")); // Fallback to legacy

        if let Some(node) = game_args_node {
            let manifest_game_args = parse_arguments(Some(node), &variables)?;
             for i in 0..manifest_game_args.len() {
                 let arg = &manifest_game_args[i];
                 // Track flags like --username
                 if arg.starts_with("--") && i + 1 < manifest_game_args.len() {
                     game_args_set.insert(arg.to_lowercase());
                     final_game_args.push(arg.clone());
                     final_game_args.push(manifest_game_args[i+1].clone()); // Add value too
                     // Skip next element as it's the value (basic assumption)
                     // A more robust parser would handle flags without values
                 } else if !arg.starts_with("--") && (i == 0 || !manifest_game_args[i-1].starts_with("--")) {
                     // Argument that isn't a value for a preceding flag
                     final_game_args.push(arg.clone());
                 }
             }
        }

        // Ensure required game arguments are present if not added by manifest
         if game_args_set.insert("--username".to_string()) {
            final_game_args.push("--username".to_string());
            final_game_args.push(variables.get("${auth_player_name}").unwrap().clone());
         }
         if game_args_set.insert("--version".to_string()) {
            final_game_args.push("--version".to_string());
            final_game_args.push(variables.get("${version_name}").unwrap().clone());
         }
         if game_args_set.insert("--gamedir".to_string()) {
            final_game_args.push("--gameDir".to_string());
            final_game_args.push(variables.get("${game_directory}").unwrap().clone());
         }
         if game_args_set.insert("--assetsdir".to_string()) {
            final_game_args.push("--assetsDir".to_string());
            final_game_args.push(variables.get("${assets_root}").unwrap().clone());
         }
        if game_args_set.insert("--assetindex".to_string()) {
            final_game_args.push("--assetIndex".to_string());
            final_game_args.push(variables.get("${assets_index_name}").unwrap().clone());
        }
         if game_args_set.insert("--uuid".to_string()) {
            final_game_args.push("--uuid".to_string());
            final_game_args.push(variables.get("${auth_uuid}").unwrap().clone());
         }
        // Ensure Access Token is passed, even if it's the placeholder "0"
        if game_args_set.insert("--accesstoken".to_string()) {
            final_game_args.push("--accessToken".to_string());
            final_game_args.push(variables.get("${auth_access_token}").unwrap().clone());
        }
        if game_args_set.insert("--usertype".to_string()) {
            final_game_args.push("--userType".to_string());
            final_game_args.push(variables.get("${user_type}").unwrap().clone());
        }
        if game_args_set.insert("--versiontype".to_string()) {
            final_game_args.push("--versionType".to_string());
            final_game_args.push(variables.get("${version_type}").unwrap().clone());
        }


        // Add legacy --tweakClass for old Forge versions if needed and not present
        let mc_minor_version = minecraft_version.split('.').nth(1).unwrap_or("0").parse::<u32>().unwrap_or(99);
        if minecraft_version.starts_with("1.") && mc_minor_version < 13 {
             if game_args_set.insert("--tweakclass".to_string()) {
                 println!("Adding legacy tweak class for MC Version {}", minecraft_version);
                 final_game_args.push("--tweakClass".to_string());
                 final_game_args.push("net.minecraftforge.fml.common.launcher.FMLTweaker".to_string());
             }
        }

        // Apply all Game args
        for arg in final_game_args {
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