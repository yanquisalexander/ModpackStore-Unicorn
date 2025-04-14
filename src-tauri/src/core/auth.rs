use crate::GLOBAL_APP_HANDLE;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::Emitter;
use tauri::{Manager, State};
use tauri_plugin_http::reqwest::Client;
use tauri_plugin_opener::OpenerExt;
use tokio::sync::Mutex;

// User session structure
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserSession {
    pub id: String,
    pub name: String,
    pub email: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
    pub roles: Vec<String>,
}

// Token response from API
#[derive(Debug, Serialize, Deserialize, Clone)]
struct TokenResponse {
    access_token: String,
    refresh_token: String,
}

// Auth steps for frontend
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub enum AuthStep {
    StartingAuth,
    WaitingForLogin,
    FetchingSession,
}

// Global auth state
pub struct AuthState {
    pub session: Mutex<Option<UserSession>>,
    pub tokens: Mutex<Option<TokenResponse>>,
}

impl AuthState {
    pub fn new() -> Self {
        Self {
            session: Mutex::new(None),
            tokens: Mutex::new(None),
        }
    }
}

const API_URL: &str = "YOUR_API_ENDPOINT";

// Helper to emit events
fn emit_event<T: Serialize + Clone>(event: &str, payload: Option<T>) -> Result<(), String> {
    let binding = GLOBAL_APP_HANDLE.lock().unwrap();
    if binding.is_none() {
        return Err("AppHandle missing".to_string());
    }
    let app = binding.as_ref().unwrap();
    let main_window = app
        .get_webview_window("main")
        .ok_or("Main window not found")?;
    main_window.emit(event, payload).map_err(|e| e.to_string())
}

// Tauri commands
#[tauri::command]
pub async fn get_current_session(
    auth_state: State<'_, AuthState>,
) -> Result<Option<UserSession>, String> {
    let session = auth_state.session.lock().await;
    Ok(session.clone())
}

#[tauri::command]
pub async fn start_discord_auth(auth_state: State<'_, AuthState>) -> Result<(), String> {
    // Emit start
    emit_event("auth-step-changed", Some(AuthStep::StartingAuth));

    // Get Discord OAuth URL from API
    let url = Client::new()
        .get(format!("{}/oauth/discord/url", API_URL))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .text()
        .await
        .map_err(|e| e.to_string())?;

    // Emit waiting
    emit_event("auth-step-changed", Some(AuthStep::WaitingForLogin));

    if url.is_empty() {
        return Err("Empty URL".to_string());
    }

    // Open URL in default browser
    // Use the global AppHandle to open the URL

    let app = GLOBAL_APP_HANDLE
        .lock()
        .unwrap()
        .as_ref()
        .ok_or("AppHandle missing")?;
    let main_window = app
        .get_webview_window("main")
        .ok_or("Main window not found")?;
    let url = format!("https://discord.com/api/oauth2/authorize?client_id={}&response_type=code&scope=identify&redirect_uri={}", "YOUR_CLIENT_ID", url);

    let opener = tauri_plugin_opener::Opener::new();
    opener.open_url(&url).map_err(|e| e.to_string())?;

    emit_event("discord-auth-started", None)?;

    Ok(())
}

#[tauri::command]
pub async fn poll_session(auth_state: State<'_, AuthState>) -> Result<Option<UserSession>, String> {
    // Called periodically by frontend to check if login done
    emit_event("auth-step-changed", Some(AuthStep::FetchingSession));

    // Call API to get tokens & session
    let resp = Client::new()
        .get(format!("{}/oauth/session", API_URL))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if resp.status().is_success() {
        let tr: TokenResponse = resp.json().await.map_err(|e| e.to_string())?;
        *auth_state.tokens.lock().await = Some(tr.clone());

        // Fetch user session
        let user: UserSession = Client::new()
            .get(format!("{}/me", API_URL))
            .bearer_auth(&tr.access_token)
            .send()
            .await
            .map_err(|e| e.to_string())?
            .error_for_status()
            .map_err(|e| e.to_string())?
            .json()
            .await
            .map_err(|e| e.to_string())?;

        *auth_state.session.lock().await = Some(user.clone());
        emit_event("auth-status-changed", Some(user.clone()));
        return Ok(Some(user));
    }

    Ok(None)
}

#[tauri::command]
pub async fn logout(auth_state: State<'_, AuthState>) -> Result<(), String> {
    let mut sess = auth_state.session.lock().await;
    let mut toks = auth_state.tokens.lock().await;

    if let Some(tr) = toks.clone() {
        let _ = Client::new()
            .post(format!("{}/logout", API_URL))
            .bearer_auth(&tr.access_token)
            .send()
            .await;
    }

    *sess = None;
    *toks = None;
    emit_event("auth-status-changed", Option::<UserSession>::None);
    Ok(())
}
