use super::rules::RuleEvaluator;
use crate::core::minecraft::paths::MinecraftPaths;
use crate::core::minecraft_account::MinecraftAccount;
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;

pub struct ArgumentProcessor<'a> {
    manifest: &'a Value,
    account: &'a MinecraftAccount,
    paths: &'a MinecraftPaths,
    memory: u32,
}

impl<'a> ArgumentProcessor<'a> {
    pub fn new(
        manifest: &'a Value,
        account: &'a MinecraftAccount,
        paths: &'a MinecraftPaths,
        memory: u32,
    ) -> Self {
        Self {
            manifest,
            account,
            paths,
            memory,
        }
    }

    pub fn process_arguments(&self) -> Option<(Vec<String>, Vec<String>)> {
        let placeholders = self.create_placeholders();
        let features = self.create_features_map();

        let jvm_args = self.process_jvm_arguments(&placeholders)?;
        let game_args = self.process_game_arguments(&placeholders, &features)?;

        Some((jvm_args, game_args))
    }

    fn create_placeholders(&self) -> HashMap<String, String> {
        let mut placeholders = HashMap::new();
        placeholders.insert(
            "auth_player_name".to_string(),
            self.account.username().to_string(),
        );
        placeholders.insert(
            "version_name".to_string(),
            self.paths.minecraft_version().to_string(),
        );
        placeholders.insert(
            "game_directory".to_string(),
            self.paths.game_dir().to_string_lossy().to_string(),
        );
        placeholders.insert(
            "assets_root".to_string(),
            self.paths.assets_dir().to_string_lossy().to_string(),
        );
        placeholders.insert(
            "assets_index_name".to_string(),
            self.manifest
                .get("assets")
                .and_then(|v| v.as_str())
                .or_else(|| self.manifest.get("assetIndex")?.get("id")?.as_str())
                .unwrap_or("legacy")
                .to_string(),
        );
        placeholders.insert("auth_uuid".to_string(), self.account.uuid().to_string());
        placeholders.insert(
            "auth_access_token".to_string(),
            self.account.access_token().unwrap_or("null").to_string(),
        );
        placeholders.insert(
            "user_type".to_string(),
            if self.account.user_type() != "offline" {
                "mojang"
            } else {
                "legacy"
            }
            .to_string(),
        );
        placeholders.insert("version_type".to_string(), "release".to_string());
        placeholders.insert(
            "natives_directory".to_string(),
            self.paths.natives_dir().to_string_lossy().to_string(),
        );
        placeholders.insert("launcher_name".to_string(), "modpackstore".to_string());
        placeholders.insert("launcher_version".to_string(), "1.0.0".to_string());

        placeholders.insert("classpath".to_string(), self.paths.classpath_str());

        placeholders
    }

    fn create_features_map(&self) -> HashMap<String, bool> {
        let mut features = HashMap::new();
        features.insert("has_custom_resolution".to_string(), false);
        features.insert("has_quick_plays_support".to_string(), false);
        features.insert("is_demo_user".to_string(), false);
        features.insert("is_quick_play_singleplayer".to_string(), false);
        features.insert("is_quick_play_multiplayer".to_string(), false);
        features.insert("is_quick_play_realms".to_string(), false);
        features
    }

    fn process_jvm_arguments(&self, placeholders: &HashMap<String, String>) -> Option<Vec<String>> {
        let mut jvm_args = vec![format!("-Xms512M"), format!("-Xmx{}M", self.memory)];

        if let Some(args_obj) = self.manifest.get("arguments").and_then(|v| v.get("jvm")) {
            let manifest_args = self.process_arguments_list(args_obj, placeholders, None);
            // Create a filtered vector first to avoid the borrow conflict
            let filtered_args: Vec<String> = manifest_args
                .into_iter()
                .filter(|arg| !jvm_args.contains(arg))
                .collect();
            // Then extend jvm_args with the filtered arguments
            jvm_args.extend(filtered_args);
        } else {
            // Legacy format
            jvm_args.extend(vec![
                format!("-Djava.library.path={}", self.paths.natives_dir().display()),
                format!("-Dminecraft.launcher.brand=modpackstore"),
                format!("-Dminecraft.launcher.version=1.0.0"),
                format!("-Djna.tmpdir={}", self.paths.natives_dir().display()),
                format!(
                    "-Dorg.lwjgl.system.SharedLibraryExtractPath={}",
                    self.paths.natives_dir().display()
                ),
                format!(
                    "-Dio.netty.native.workdir={}",
                    self.paths.natives_dir().display()
                ),
            ]);

            // OS-specific arguments
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

        // Ensure classpath is included
        if !jvm_args
            .iter()
            .any(|arg| arg == "-cp" || arg == "-classpath")
        {
            let classpath = self.paths.classpath_str();
            jvm_args.push("-cp".to_string());
            jvm_args.push(classpath);
        }

        Some(jvm_args)
    }

    fn process_game_arguments(
        &self,
        placeholders: &HashMap<String, String>,
        features: &HashMap<String, bool>,
    ) -> Option<Vec<String>> {
        if let Some(args_obj) = self.manifest.get("arguments").and_then(|v| v.get("game")) {
            Some(self.process_arguments_list(args_obj, placeholders, Some(features)))
        } else if let Some(min_args) = self
            .manifest
            .get("minecraftArguments")
            .and_then(|v| v.as_str())
        {
            Some(
                min_args
                    .split_whitespace()
                    .map(|arg| self.replace_placeholders(arg, placeholders))
                    .collect(),
            )
        } else {
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
                "--assetIndex".to_string(),
                placeholders["assets_index_name"].clone(),
                "--uuid".to_string(),
                placeholders["auth_uuid"].clone(),
                "--accessToken".to_string(),
                placeholders["auth_access_token"].clone(),
                "--userType".to_string(),
                placeholders["user_type"].clone(),
            ];

            Some(arguments)
        }
    }

    fn process_arguments_list(
        &self,
        args_obj: &Value,
        placeholders: &HashMap<String, String>,
        features: Option<&HashMap<String, bool>>,
    ) -> Vec<String> {
        let mut processed_args = Vec::new();

        if let Some(args_array) = args_obj.as_array() {
            for arg in args_array {
                if let Some(arg_str) = arg.as_str() {
                    processed_args.push(self.replace_placeholders(arg_str, placeholders));
                } else if arg.is_object() {
                    if let Some(rules) = arg.get("rules").and_then(|r| r.as_array()) {
                        let mut should_include = false;

                        for rule in rules {
                            if RuleEvaluator::should_apply_rule(rule, features) {
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

    fn replace_placeholders(&self, input: &str, placeholders: &HashMap<String, String>) -> String {
        let mut result = input.to_string();
        for (key, value) in placeholders {
            result = result.replace(&format!("${{{}}}", key), value);
        }
        result
    }
}
