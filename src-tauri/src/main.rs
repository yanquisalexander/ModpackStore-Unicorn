#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(non_snake_case)]
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(unused_attributes)]
#![allow(unused_macros)]

mod config;
mod core;
mod interfaces;
mod utils;

use core::auth::*;
use serde_json::json;
use std::sync::Arc;
use tauri::Emitter;
use tauri::Manager; // Necesario para get_window y emit
use tauri::Wry;
use tauri_plugin_log::{Target, TargetKind};
use tauri_plugin_store::StoreExt;

static GLOBAL_APP_HANDLE: once_cell::sync::Lazy<std::sync::Mutex<Option<tauri::AppHandle>>> =
    once_cell::sync::Lazy::new(|| std::sync::Mutex::new(None));

static API_ENDPOINT: &str = "https://api-modpackstore.alexitoo.dev/v1";

pub fn main() {
    let logs_dir = dirs::config_dir()
        .expect("No se pudo obtener el directorio de configuración")
        .join("dev.alexitoo.modpackstore")
        .join("logs");

    let log_file_name = format!(
        "mstore_{}",
        chrono::Local::now().format("%Y-%m-%d_%H-%M-%S")
    );

    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::new().build())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_drpc::init())
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Info)
                .level_for("reqwest", log::LevelFilter::Info)
                .rotation_strategy(tauri_plugin_log::RotationStrategy::KeepAll)
                .target(tauri_plugin_log::Target::new(
                    tauri_plugin_log::TargetKind::Folder {
                        path: std::path::PathBuf::from(logs_dir),
                        file_name: Some(log_file_name),
                    },
                ))
                .build(),
        )
        .manage(Arc::new(AuthState::new()))
        .setup(|app| {
            let main_window = app.get_webview_window("main").unwrap();
            // Focus the main window
            main_window.set_focus().unwrap();

            log::info!("Starting Modpack Store...");
            log::info!(
                "Running on: {}, {}",
                std::env::consts::OS,
                std::env::consts::ARCH
            );

            // Store the AppHandle in the static variable
            let mut app_handle = GLOBAL_APP_HANDLE.lock().unwrap();
            *app_handle = Some(app.handle().clone());
            // Emit an event to the main window
            main_window.emit("app-ready", ()).unwrap();

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            config::get_config,
            config::get_schema,
            config::set_config,
            core::network_utilities::check_connection,
            core::network_utilities::check_real_connection,
            core::instance_manager::get_all_instances,
            core::instance_manager::get_instance_by_id,
            core::instance_manager::delete_instance,
            //utils::config_manager::get_config,
            core::instance_manager::launch_mc_instance,
            core::minecraft_instance::open_game_dir,
            core::instance_manager::update_instance,
            core::instance_manager::update_modpack_instance,
            core::instance_manager::create_local_instance,
            core::instance_manager::search_instances,
            core::instance_manager::remove_instance,
            core::instance_bootstrap::check_vanilla_integrity,
            core::instance_bootstrap::validate_modpack_assets,
            core::accounts_manager::get_all_accounts,
            core::accounts_manager::add_offline_account,
            core::accounts_manager::ensure_account_exists,
            core::accounts_manager::remove_account,
            core::minecraft_instance::get_instances_by_modpack_id,
            core::auth::start_discord_auth,
            core::auth::get_current_session,
            core::auth::logout,
            core::auth::init_session,
            core::microsoft_auth::start_microsoft_auth,
            core::prelaunch_appearance::get_prelaunch_appearance,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
