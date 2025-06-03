use serde_json::{Map, Value};
use std::collections::{BTreeMap, HashMap};

pub struct ManifestMerger;

impl ManifestMerger {
    pub fn merge(vanilla: Value, forge: Value) -> Value {
        let mut result = vanilla.clone();

        // Merge main class
        if let Some(mc) = forge.get("mainClass") {
            result["mainClass"] = mc.clone();
        }

        // Merge libraries
        Self::merge_libraries(&mut result, &vanilla, &forge);

        // Merge arguments
        Self::merge_arguments(&mut result, &vanilla, &forge);

        // Merge legacy arguments
        Self::merge_legacy_arguments(&mut result, &vanilla, &forge);

        result
    }

    fn merge_libraries(result: &mut Value, vanilla: &Value, forge: &Value) {
        let mut libs: BTreeMap<String, Value> = BTreeMap::new();
        let mut duplicates: HashMap<String, Vec<String>> = HashMap::new();

        // Primero agregamos todas las bibliotecas vanilla directamente
        if let Some(arr) = vanilla.get("libraries").and_then(Value::as_array) {
            for lib in arr {
                if let Some((name, ga, vver, _, classifier)) = Self::extract_lib_info(lib) {
                    // Construimos una clave que incluya el clasificador y la versión para garantizar unicidad
                    let key = Self::build_complete_lib_key(&ga, &vver, &classifier);

                    // No consideramos duplicados dentro de vanilla, simplemente las agregamos
                    libs.insert(key, lib.clone());

                    // Guardamos esta versión para posible referencia de debug
                    let version_str = format!("vanilla:{}", vver.unwrap_or_default());
                    duplicates.entry(ga).or_default().push(version_str);
                }
            }
        }

        // Luego procesamos las bibliotecas de forge con reglas especiales para manejar duplicados
        if let Some(arr) = forge.get("libraries").and_then(Value::as_array) {
            for lib in arr {
                if let Some((name, ga, fver, furl, classifier)) = Self::extract_lib_info(lib) {
                    // Construimos la misma clave que usaríamos para vanilla
                    let std_key = Self::build_lib_key(&ga, &classifier);
                    // También construimos una clave única para esta versión específica
                    let forge_key = Self::build_complete_lib_key(&ga, &fver, &classifier);

                    // Verificamos si hay alguna versión de esta biblioteca en vanilla
                    let vanilla_versions: Vec<(&String, &Value)> = libs
                        .iter()
                        .filter(|(k, _)| k.starts_with(&std_key))
                        .collect();

                    let should_add_forge = if !vanilla_versions.is_empty() {
                        // Si hay versiones de vanilla, decidimos si preferimos la de forge
                        let mut prefer_forge = true;

                        for (_, vanilla_lib) in &vanilla_versions {
                            let (_, _, vver, vurl, _) =
                                Self::extract_lib_info(vanilla_lib).unwrap();

                            // Si las URLs son diferentes, mantenemos ambas versiones
                            if furl != vurl {
                                continue;
                            }

                            // Para log4j específicamente, preferimos la versión más alta
                            if ga.contains("log4j") {
                                if let (Some(v), Some(f)) = (&vver, &fver) {
                                    let cmp_v: Vec<i32> =
                                        v.split('.').filter_map(|p| p.parse().ok()).collect();
                                    let cmp_f: Vec<i32> =
                                        f.split('.').filter_map(|p| p.parse().ok()).collect();
                                    prefer_forge = cmp_f > cmp_v;
                                }
                            }

                            // Si decidimos no preferir forge, no necesitamos revisar más versiones de vanilla
                            if !prefer_forge {
                                break;
                            }
                        }

                        prefer_forge
                    } else {
                        // Si no hay versiones en vanilla, siempre agregamos la de forge
                        true
                    };

                    if should_add_forge {
                        // Before adding the Forge library, remove any vanilla versions
                        let keys_to_remove: Vec<String> = libs
                            .keys()
                            .filter(|k| k.starts_with(&std_key))
                            .cloned()
                            .collect();

                        for key_to_remove in keys_to_remove {
                            if let Some(removed_lib) = libs.remove(&key_to_remove) {
                                if let Some((_, removed_ga, removed_ver, _, removed_classifier)) = Self::extract_lib_info(&removed_lib) {
                                    log::debug!(
                                        "Replacing vanilla library {} (version: {:?}, classifier: {:?}) with Forge version.",
                                        removed_ga,
                                        removed_ver,
                                        removed_classifier
                                    );
                                }
                            }
                        }

                        // Registramos esta versión para depuración
                        let forge_version_str =
                            format!("forge:{}", fver.clone().unwrap_or_default());
                        duplicates
                            .entry(ga.clone())
                            .or_default()
                            .push(forge_version_str);

                        // Agregamos la biblioteca de forge
                        libs.insert(forge_key, lib.clone());
                    }
                }
            }
        }

        // Registramos duplicados solo para depuración (bibliotecas con múltiples versiones)
        for (ga, sources) in duplicates.iter().filter(|(_, s)| s.len() > 1) {
            log::info!("Multiple versions of {}: {}", ga, sources.join(", "));
        }

        result["libraries"] = Value::Array(libs.into_values().collect());
    }

    fn merge_arguments(result: &mut Value, vanilla: &Value, forge: &Value) {
        let mut args_map = Map::default();

        for kind in &["game", "jvm"] {
            let mut list = Vec::new();

            if let Some(v) = vanilla
                .get("arguments")
                .and_then(|a| a.get(kind))
                .and_then(Value::as_array)
            {
                list.extend(v.clone());
            }

            if let Some(f) = forge
                .get("arguments")
                .and_then(|a| a.get(kind))
                .and_then(Value::as_array)
            {
                list.extend(f.clone());
            }

            if !list.is_empty() {
                args_map.insert(kind.to_string(), Value::Array(list));
            }
        }

        if !args_map.is_empty() {
            result["arguments"] = Value::Object(args_map);
        }
    }

    fn merge_legacy_arguments(result: &mut Value, vanilla: &Value, forge: &Value) {
        let mut kv = HashMap::new();

        for src in [
            vanilla.get("minecraftArguments"),
            forge.get("minecraftArguments"),
        ] {
            if let Some(Value::String(s)) = src {
                for pair in s.split_whitespace().collect::<Vec<_>>().chunks(2) {
                    if let [k, v] = pair {
                        kv.insert(k.to_string(), v.to_string());
                    }
                }
            }
        }

        if !kv.is_empty() {
            let merged_legacy = kv
                .into_iter()
                .map(|(k, v)| format!("{} {}", k, v))
                .collect::<Vec<_>>()
                .join(" ");
            result["minecraftArguments"] = Value::String(merged_legacy);
        }
    }

    fn extract_lib_info(
        lib: &Value,
    ) -> Option<(
        String,
        String,
        Option<String>,
        Option<String>,
        Option<String>,
    )> {
        let name = lib.get("name")?.as_str()?.to_string();
        let parts: Vec<&str> = name.split(':').collect();
        let ga = if parts.len() >= 2 {
            format!("{}:{}", parts[0], parts[1])
        } else {
            name.clone()
        };
        let version = parts.get(2).map(|s| s.to_string());
        let classifier = lib
            .get("downloads")
            .and_then(|d| d.get("artifact"))
            .and_then(|a| a.get("classifier"))
            .or_else(|| lib.get("classifier"))
            .and_then(Value::as_str)
            .map(String::from);
        let url = lib.get("url").and_then(Value::as_str).map(String::from);
        Some((name, ga, version, url, classifier))
    }

    // Construye una clave básica basada en groupId:artifactId y clasificador opcional
    fn build_lib_key(ga: &str, classifier: &Option<String>) -> String {
        if let Some(c) = classifier {
            format!("{}:{}", ga, c)
        } else {
            ga.to_string()
        }
    }

    // Construye una clave completa que incluye versión para garantizar unicidad
    fn build_complete_lib_key(
        ga: &str,
        version: &Option<String>,
        classifier: &Option<String>,
    ) -> String {
        let ver_part = version.as_ref().map_or("", |v| v.as_str());
        if let Some(c) = classifier {
            format!("{}:{}:{}", ga, ver_part, c)
        } else {
            format!("{}:{}", ga, ver_part)
        }
    }

    fn prefer_forge(ga: &str, vver: &Option<String>, fver: &Option<String>) -> bool {
        if ga.contains("log4j") {
            if let (Some(v), Some(f)) = (vver, fver) {
                let cmp_v: Vec<i32> = v.split('.').filter_map(|p| p.parse().ok()).collect();
                let cmp_f: Vec<i32> = f.split('.').filter_map(|p| p.parse().ok()).collect();
                return cmp_f > cmp_v;
            }
        }
        true
    }
}
