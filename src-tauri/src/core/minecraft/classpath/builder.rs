use crate::core::minecraft::paths::MinecraftPaths;
use crate::core::minecraft::RuleEvaluator;
use serde_json::Value;
use std::collections::HashSet;
use std::path::{Path, MAIN_SEPARATOR};

pub struct ClasspathBuilder<'a> {
    manifest: &'a Value,
    paths: &'a MinecraftPaths,
}

impl<'a> ClasspathBuilder<'a> {
    pub fn new(manifest: &'a Value, paths: &'a MinecraftPaths) -> Self {
        Self { manifest, paths }
    }

    pub fn build(&self) -> Option<String> {
        let mut entries = Vec::new();
        let mut seen = HashSet::new();

        // Add client JAR
        let client_path = self.paths.client_jar().to_string_lossy().to_string();
        entries.push(client_path.clone());
        seen.insert(client_path);

        // Process libraries
        if let Some(libs) = self.manifest.get("libraries").and_then(|v| v.as_array()) {
            for lib in libs {
                if !self.should_include_library(lib) {
                    continue;
                }

                // Add main artifact
                if let Some(path_val) = lib
                    .get("downloads")
                    .and_then(|d| d.get("artifact"))
                    .and_then(|a| a.get("path"))
                    .and_then(Value::as_str)
                {
                    let jar = self
                        .paths
                        .libraries_dir()
                        .join(path_val.replace('/', &MAIN_SEPARATOR.to_string()));
                    self.add_if_new(&jar, &mut entries, &mut seen);
                }

                // Add native classifiers
                if let Some(classifiers) = lib
                    .get("downloads")
                    .and_then(|d| d.get("classifiers"))
                    .and_then(Value::as_object)
                {
                    let os_classifier = if cfg!(windows) {
                        "natives-windows"
                    } else if cfg!(target_os = "linux") {
                        "natives-linux"
                    } else {
                        "natives-macos"
                    };

                    if let Some(info) = classifiers.get(os_classifier) {
                        if let Some(path_val) = info.get("path").and_then(Value::as_str) {
                            let native_jar = self
                                .paths
                                .libraries_dir()
                                .join(path_val.replace('/', &MAIN_SEPARATOR.to_string()));
                            self.add_if_new(&native_jar, &mut entries, &mut seen);
                        }
                    }
                }
            }
        }

        Some(entries.join(self.classpath_separator()))
    }

    fn should_include_library(&self, lib: &Value) -> bool {
        lib.get("rules")
            .and_then(|r| r.as_array())
            .map(|rules| {
                rules
                    .iter()
                    .any(|rule| RuleEvaluator::should_apply_rule(rule, None))
            })
            .unwrap_or(true)
    }

    fn add_if_new(&self, path: &Path, entries: &mut Vec<String>, seen: &mut HashSet<String>) {
        if path.exists() {
            let s = path.to_string_lossy().to_string();
            if seen.insert(s.clone()) {
                entries.push(s);
            }
        }
    }

    fn classpath_separator(&self) -> &str {
        if cfg!(windows) {
            ";"
        } else {
            ":"
        }
    }
}
