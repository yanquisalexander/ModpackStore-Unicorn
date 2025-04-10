pub struct LaunchHelper {
    instance: MinecraftInstance,
    vanilla_launcher: VanillaLauncher,
    forge_launcher: ForgeLauncher,
}

impl LaunchHelper {
    pub fn new(instance: MinecraftInstance) -> Self {
        LaunchHelper {
            instance: instance.clone(),
            vanilla_launcher: VanillaLauncher::new(instance.clone()),
            forge_launcher: ForgeLauncher::new(instance),
        }
    }
    
    pub fn validate_account(&self) -> Option<MinecraftAccount> {
        let accounts_manager = AccountsManager::new();
        let account_uuid = self.instance.account_uuid.as_deref()?;
        
        if account_uuid.is_empty() {
            // Show error message about no account selected
            return None;
        }
        
        let account = accounts_manager.get_minecraft_account(account_uuid);
        if account.is_none() {
            // Show error message about account not found
        }
        
        account
    }
    
    pub fn launch_minecraft_process(&self, java_path: &str, account: &MinecraftAccount) -> Option<std::process::Child> {
        if self.instance.forge_version.as_deref().unwrap_or("").trim().is_empty() {
            self.vanilla_launcher.launch(java_path, account)
        } else {
            self.forge_launcher.launch(java_path, account)
        }
    }
    
    pub fn handle_process_lifecycle(&self, process: Option<std::process::Child>, config_manager: &ConfigManager) {
        let Some(mut process) = process else { return };
        
        // Handle stdout and stderr redirection using threads
        // ...
        
        // Handle the "close on launch" option
        // ...
    }
}