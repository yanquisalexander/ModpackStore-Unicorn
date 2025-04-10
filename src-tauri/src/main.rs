// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod core;
mod utils;

// Learn more about Tauri commands at https://v1.tauri.app/v1/guides/features/command

pub fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            core::network_utilities::check_connection,
            core::instance_manager::get_all_instances,
            core::instance_manager::get_instance_by_name,
            core::instance_manager::delete_instance,
            // Config magager is a singleton
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
