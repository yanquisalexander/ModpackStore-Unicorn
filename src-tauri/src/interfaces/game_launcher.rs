pub trait GameLauncher {
    fn launch(&self) -> Option<std::process::Child>;
}
