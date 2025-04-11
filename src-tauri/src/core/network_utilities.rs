// src/core/NetworkUtilities.rs
use tauri_plugin_http::reqwest;

#[tauri::command]
pub fn check_connection() -> bool {
    std::thread::sleep(std::time::Duration::from_secs(1));
    // Attempt to make a GET request to a reliable server (e.g., Google)
    let response = reqwest::blocking::get("https://www.google.com");
    match response {
        Ok(_) => true,
        Err(_) => false,
    }
}
