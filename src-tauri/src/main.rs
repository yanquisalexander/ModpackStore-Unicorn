#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod core;
mod utils;

use tauri::Emitter;
use tauri::Manager; // Necesario para get_window y emit

pub fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let window = app.get_webview_window("main").unwrap();

            tauri::async_runtime::spawn(async move {
                use tokio::time::{sleep, Duration};

                sleep(Duration::from_secs(5)).await;

                // Simular update disponible
                window
                    .emit("update-available", {
                        serde_json::json!({ "version": "2.0.0" })
                    })
                    .unwrap();

                // Simular progreso
                for i in 1..=100 {
                    sleep(Duration::from_millis(50)).await;
                    window
                        .emit("update-download-progress", {
                            serde_json::json!({ "percent": i })
                        })
                        .unwrap();
                }

                // Simular completado
                window.emit("update-download-complete", ()).unwrap();

                // Simular instalado
                sleep(Duration::from_secs(2)).await;
                window.emit("update-installed", ()).unwrap();
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            core::network_utilities::check_connection,
            core::instance_manager::get_all_instances,
            core::instance_manager::get_instance_by_name,
            core::instance_manager::delete_instance,
            utils::config_manager::get_config,
            core::instance_manager::launch_mc_instance,
            core::accounts_manager::get_all_accounts,
        ])
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_os::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
