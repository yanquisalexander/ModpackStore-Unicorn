use serde_json::Value;
use std::collections::HashMap;

pub struct RuleEvaluator;

impl RuleEvaluator {
    pub fn should_apply_rule(rule: &Value, features: Option<&HashMap<String, bool>>) -> bool {
        let action = rule
            .get("action")
            .and_then(|a| a.as_str())
            .unwrap_or("allow");
        let mut should_apply = action == "allow";

        // Check OS rules
        if let Some(os_obj) = rule.get("os") {
            let mut os_match = true;

            if let Some(os_name) = os_obj.get("name").and_then(|n| n.as_str()) {
                let is_current_os = match os_name {
                    "windows" => cfg!(windows),
                    "osx" => cfg!(target_os = "macos"),
                    "linux" => cfg!(target_os = "linux"),
                    _ => false,
                };
                if !is_current_os {
                    os_match = false;
                }
            }

            if let Some(os_arch) = os_obj.get("arch").and_then(|a| a.as_str()) {
                let is_current_arch = match os_arch {
                    "x86" => cfg!(target_arch = "x86"),
                    "x86_64" => cfg!(target_arch = "x86_64"),
                    "arm" => cfg!(target_arch = "arm"),
                    "arm64" => cfg!(target_arch = "aarch64"),
                    _ => false,
                };
                if !is_current_arch {
                    os_match = false;
                }
            }

            if action == "allow" {
                should_apply = os_match;
            } else {
                should_apply = !os_match;
            }
        }

        // Check feature rules
        if let Some(feature_obj) = rule.get("features") {
            if let Some(features_map) = features {
                for (feature_name, feature_value) in
                    feature_obj.as_object().unwrap_or(&serde_json::Map::new())
                {
                    if let Some(expected_value) = feature_value.as_bool() {
                        let actual_value = *features_map.get(feature_name).unwrap_or(&false);
                        if actual_value != expected_value {
                            should_apply = action != "allow";
                            break;
                        }
                    }
                }
            } else {
                should_apply = action != "allow";
            }
        }

        should_apply
    }
}
