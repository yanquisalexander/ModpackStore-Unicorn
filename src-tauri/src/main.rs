#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod core;
mod utils;

use tauri::Emitter;
use tauri::Manager; // Necesario para get_window y emit

pub fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        

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
