#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod core;
mod interfaces;
mod utils;

use core::auth::*;
use std::sync::Arc;
use tauri::Emitter;
use tauri::Manager; // Necesario para get_window y emit
use tauri::Wry;
use tauri_plugin_store::StoreExt;
use serde_json::json;

static GLOBAL_APP_HANDLE: once_cell::sync::Lazy<std::sync::Mutex<Option<tauri::AppHandle>>> =
    once_cell::sync::Lazy::new(|| std::sync::Mutex::new(None));

static API_ENDPOINT: &str = "http://localhost:3000/v1";

pub fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_drpc::init())
        .manage(Arc::new(AuthState::new()))
        .setup(|app| {
            let main_window = app.get_webview_window("main").unwrap();
            // Focus the main window
            main_window.set_focus().unwrap();

            // Store the AppHandle in the static variable
            let mut app_handle = GLOBAL_APP_HANDLE.lock().unwrap();
            *app_handle = Some(app.handle().clone());
            // Emit an event to the main window
            main_window.emit("app-ready", ()).unwrap();
          
           
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            core::network_utilities::check_connection,
            core::instance_manager::get_all_instances,
            core::instance_manager::get_instance_by_id,
            core::instance_manager::delete_instance,
            utils::config_manager::get_config,
            core::instance_manager::launch_mc_instance,
            core::minecraft_instance::open_game_dir,
            core::instance_manager::update_instance,
            core::accounts_manager::get_all_accounts,
            core::accounts_manager::add_offline_account,
            core::accounts_manager::remove_account,
            core::accounts_manager::add_microsoft_account,
            core::auth::start_discord_auth,
            core::auth::get_current_session,
            core::auth::logout,
            core::auth::init_session,
            core::microsoft_auth::start_microsoft_auth,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
