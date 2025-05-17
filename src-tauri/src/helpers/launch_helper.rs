pub struct LaunchHelper {
    instance: MinecraftInstance,
    minecraft_launcher: MinecraftLauncher,
}

impl LaunchHelper {
    pub fn new(instance: MinecraftInstance) -> Self {
        LaunchHelper {
            instance: instance.clone(),
            minecraft_launcher: MinecraftLauncher::new(instance.clone()),
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

    pub fn launch_minecraft_process(
        &self,
        java_path: &str,
        account: &MinecraftAccount,
    ) -> Option<std::process::Child> {
        self.minecraft_launcher
            .launch(java_path, account)
            .ok()
            .map(|process| {
                // Handle process lifecycle here if needed
                process
            })
    }

    pub fn handle_process_lifecycle(
        &self,
        process: Option<std::process::Child>,
        config_manager: &ConfigManager,
    ) {
        let Some(mut process) = process else { return };

        // Handle stdout and stderr redirection using threads
        // ...

        // Handle the "close on launch" option
        // ...
    }
}
