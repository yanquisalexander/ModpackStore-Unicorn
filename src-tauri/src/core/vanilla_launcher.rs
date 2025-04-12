use std::{fs, path::{PathBuf}, process::{Command, Child}};
use serde_json::Value;

use crate::core::{minecraft_instance::MinecraftInstance, minecraft_account::MinecraftAccount};
use crate::interfaces::game_launcher::GameLauncher;
use crate::core::accounts_manager::AccountsManager;
use uuid::Uuid;

pub struct VanillaLauncher {
    instance: MinecraftInstance,
}

impl VanillaLauncher {
    pub fn new(instance: MinecraftInstance) -> Self {
        Self { instance }
    }
}

impl GameLauncher for VanillaLauncher {
    fn launch(&self) -> Option<Child> {
        // Obtener el ConfigManager (asumo que tiene una implementación similar)
        let config_manager = crate::utils::config_manager::get_config_manager();
        let java_path = config_manager
            .lock() // First lock the mutex to get the inner value
            .expect("Failed to lock config manager mutex") // Handle potential lock failure
            .get_java_dir() // Now call the method on the inner value
            .join("bin")
            .join(if cfg!(windows) { "java.exe" } else { "java" });

            let accounts_manager = AccountsManager::new();


            // If instance does not have an account (null), throw an error
            if self.instance.accountUuid.is_none() {
                println!("No account found for this instance.");
                return None;
            }

        let account = accounts_manager
        .get_minecraft_account_by_uuid(self.instance.accountUuid.as_ref().unwrap_or(&"".to_string()))
        .unwrap_or_else(|| {
            println!("Account not found for UUID: {}", self.instance.accountUuid.as_ref().unwrap_or(&"".to_string()));
            MinecraftAccount::new("offline".to_string(), Uuid::new_v4().to_string(), None, "Local".to_string())
        });

        println!("Account: {:?}", account);

        // Obtener el directorio del juego
        // Game Dir is instanceDirectory + "/minecraft"
        let game_dir = self.instance.instanceDirectory
        .as_ref() // Convierte Option<String> a Option<&String>
        .map(|dir| PathBuf::from(dir).join("minecraft")) // Convierte a PathBuf y luego hace el join
        .unwrap_or_else(|| PathBuf::from("default_path").join("minecraft")); // Si es None, usa el valor por defecto

        
        if !game_dir.exists() {
            fs::create_dir_all(&game_dir).expect("Failed to create game directory");
        }

        let minecraft_version = self.instance.minecraftVersion.clone();

        let version_dir = game_dir.join("versions").join(&minecraft_version);
        let client_jar = version_dir.join(format!("{minecraft_version}.jar"));
        let natives_dir = game_dir.join("natives");
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
        println!("account: {:?}", account);


        // Validaciones
        for (desc, path) in &[
            ("Client JAR", &client_jar),
            ("Natives", &natives_dir),
            ("Libraries", &libraries_dir),
            ("Manifest", &manifest_file),
        ] {
            if !path.exists() {
                println!("{} not found: {}", desc, path.display());
                return None;
            }
        }

        // Leer y parsear el JSON
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

        let main_class = manifest_json.get("mainClass")?.as_str()?.to_string();

        // Obtener assets index
        let assets_index = manifest_json.get("assets")
            .and_then(|v| v.as_str())
            .map(String::from)
            .or_else(|| manifest_json.get("assetIndex")?.get("id")?.as_str().map(String::from))
            .unwrap_or_else(|| "legacy".to_string());

        // Construir classpath
        let mut classpath = vec![client_jar.to_string_lossy().to_string()];
        if let Some(libs) = manifest_json.get("libraries")?.as_array() {
            for lib in libs {
                if let Some(path) = lib.get("downloads")?.get("artifact")?.get("path")?.as_str() {
                    let lib_path = libraries_dir.join(path.replace('/', &std::path::MAIN_SEPARATOR.to_string()));
                    if lib_path.exists() {
                        classpath.push(lib_path.to_string_lossy().to_string());
                    } else {
                      println!("Library not found: {}", lib_path.display());
                    }
                }
            }
        }

        let classpath_str = classpath.join(if cfg!(windows) { ";" } else { ":" });

        // Construir comando
        let mut command = Command::new(java_path);
        command.arg("-Xms512M")
            .arg(format!("-Djava.library.path={}", natives_dir.display()))
            .arg(format!("-Dminecraft.client.jar={}", client_jar.display()))
            .arg("-Dminecraft.launcher.brand=modpackstore")
            .arg("-Dminecraft.launcher.version=1.0.0")
            .arg("-cp").arg(&classpath_str)
            .arg(&main_class)
            .args([
                "--username", &account.username(),
                "--version", &minecraft_version,
                "--gameDir", &game_dir.to_string_lossy(),
                "--assetsDir", &assets_dir.to_string_lossy(),
                "--assetIndex", &assets_index,
                "--uuid", &account.uuid(),
                "--accessToken", account.access_token().as_deref().unwrap_or("null"),
                "--versionType", "release"
            ]);

        if account.user_type() != "offline" {
            command.args(["--userType", "mojang"]);
        } else {
            command.args(["--userType", "Local"]);
        }

        command.current_dir(&game_dir);
       println!("Command: {:?}", command);
        

        
        // Ejecutar el comando
        let child = command.spawn().ok()?;
        println!("Spawned child process: {:?}", child.id());
        Some(child)

    }
}
