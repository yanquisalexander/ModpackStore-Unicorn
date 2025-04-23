// src/core/NetworkUtilities.rs
use tauri_plugin_http::reqwest;

use crate::API_ENDPOINT;
#[tauri::command]
pub async fn check_connection() -> bool {
    // Usando tokio para el retardo asíncrono
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    
    // Attempt to ping the API endpoint using async reqwest
    let api_url = format!("{}/ping", API_ENDPOINT);
    
    match reqwest::get(&api_url).await {
        Ok(resp) => resp.status().is_success(),
        Err(_) => false
    }
}

pub fn check_real_connection() -> bool {
  // Backend maybe is not reachable,
  // but the internet connection is ok
  // So let's ping other website, like google

  // This is used internally, for example at the moment of
  // downloading assets (This uses official Minecraft Servers)

  let response = reqwest::blocking::get("https://www.google.com");
    if let Ok(resp) = response {
        if resp.status().is_success() {
            return true;
        }
    }
    false
}