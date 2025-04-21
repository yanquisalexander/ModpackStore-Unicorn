// src/core/NetworkUtilities.rs
use tauri_plugin_http::reqwest;

use crate::API_ENDPOINT;
#[tauri::command]

/* router.get("/ping", (req, res) => {
    res.send("pong");
} */
pub fn check_connection() -> bool {
    std::thread::sleep(std::time::Duration::from_secs(1));
    // Attempt to ping the API endpoint
    let api_url = format!("{}/ping", API_ENDPOINT);
    let response = reqwest::blocking::get(&api_url);

    if let Ok(resp) = response {
        if resp.status().is_success() {
            return true;
        }
    }
    false
}
