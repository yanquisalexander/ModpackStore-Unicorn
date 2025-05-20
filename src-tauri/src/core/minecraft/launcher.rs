use crate::config::get_config_manager;
use crate::core::accounts_manager::AccountsManager;
use crate::core::minecraft::{
    arguments::ArgumentProcessor,
    classpath::ClasspathBuilder,
    manifest::{ManifestMerger, ManifestParser},
    paths::MinecraftPaths,
};
use crate::core::{minecraft_account::MinecraftAccount, minecraft_instance::MinecraftInstance};
use crate::interfaces::game_launcher::GameLauncher;
use std::process::{Child, Command, Stdio};
use uuid::Uuid;

pub struct MinecraftLauncher {
    instance: MinecraftInstance,
}

impl MinecraftLauncher {
    pub fn new(instance: MinecraftInstance) -> Self {
        Self { instance }
    }
}

impl GameLauncher for MinecraftLauncher {
    fn launch(&self) -> Option<Child> {
        let config_manager = match get_config_manager().lock() {
            Ok(manager) => manager,
            Err(_) => return None,
        };

        let config = match config_manager.as_ref() {
            Ok(cfg) => cfg,
            Err(_) => return None,
        };

        log::info!("[MinecraftLauncher] Config loaded");
        log::info!(
            "[MinecraftLauncher] Starting {} Minecraft instance",
            self.instance.instanceName
        );

        let mc_memory = match config.get_minecraft_memory() {
            Some(mem) => mem,
            None => {
                log::warn!("No Minecraft memory config found, using default 2048MB");
                2048
            }
        };

        log::info!("Minecraft memory: {}MB", mc_memory);

        // Get account
        let accounts_manager = AccountsManager::new();
        let account_uuid = self.instance.accountUuid.as_ref()?;
        let account = accounts_manager.get_minecraft_account_by_uuid(account_uuid)?;

        log::info!(
            "[MinecraftLauncher] Launching Minecraft using account: {}",
            account.username()
        );

        // Setup paths
        let paths = MinecraftPaths::new(&self.instance, config)?;

        log::info!("[MinecraftLauncher] Minecraft paths: {:?}", paths);
        log::info!("[MinecraftLauncher] Java path: {:?}", paths.java_path());
        // Load and merge manifests if needed
        let manifest_parser = ManifestParser::new(&paths);
        let manifest_json = manifest_parser.load_merged_manifest()?;

        log::info!("[MinecraftLauncher] Manifest loaded");
        log::info!("[MinecraftLauncher] Manifest JSON: {:?}", manifest_json);

        // Build classpath
        let classpath_builder = ClasspathBuilder::new(&manifest_json, &paths);
        let classpath_str = classpath_builder.build()?;

        log::info!("[MinecraftLauncher] Classpath: {}", classpath_str);

        // Process arguments
        let argument_processor =
            ArgumentProcessor::new(&manifest_json, &account, &paths, mc_memory);
        let (jvm_args, game_args) = argument_processor.process_arguments()?;

        // Get main class
        let main_class = manifest_json.get("mainClass")?.as_str()?;

        // Build and execute command
        let mut command = Command::new(paths.java_path());
        command
            .args(&jvm_args)
            .arg(main_class)
            .args(&game_args)
            .current_dir(paths.game_dir())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        log::info!("Launching Minecraft with command: {:?}", command);

        match command.spawn() {
            Ok(child) => Some(child),
            Err(e) => {
                log::error!("Failed to launch Minecraft: {}", e);
                None
            }
        }
    }
}
