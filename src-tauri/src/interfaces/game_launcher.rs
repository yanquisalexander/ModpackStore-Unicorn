pub trait GameLauncher {
    fn launch(&self, java_path: &str, account: &MinecraftAccount) -> Option<std::process::Child>;
}
