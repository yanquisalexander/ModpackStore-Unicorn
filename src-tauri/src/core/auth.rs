use crate::GLOBAL_APP_HANDLE; // Assuming this is defined elsewhere
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use tauri::Emitter;
use tauri::{Manager, State};
use tauri_plugin_http::reqwest::Client;
use tauri_plugin_opener;
use tokio::sync::Mutex;
// Removed unused FromStr import: use std::str::FromStr;

// Importaciones correctas de Hyper
use hyper::header::HeaderValue;
use hyper::server::Server;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, StatusCode};

use crate::API_ENDPOINT;

// User session structure
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserSession {
   #[serde(flatten)]
   pub extra: serde_json::Value, // Use serde_json::Value for dynamic fields
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
    WaitingCallback,
    ProcessingCallback,
    RequestingSession,
}

// Global auth state (remains the same structure)
// IMPORTANT: This AuthState will now be wrapped in an Arc when managed by Tauri
#[derive(Debug)] // Added Debug derive for easier inspection if needed
pub struct AuthState {
    pub session: Mutex<Option<UserSession>>,
    pub tokens: Mutex<Option<TokenResponse>>,
    pub auth_code: Mutex<Option<String>>,
}

impl AuthState {
    pub fn new() -> Self {
        Self {
            session: Mutex::new(None),
            tokens: Mutex::new(None),
            auth_code: Mutex::new(None),
        }
    }
}

// --- Constants ---
// !!! REPLACE THESE WITH YOUR ACTUAL VALUES !!!
const CLIENT_ID: &str = "943184136976334879";
const REDIRECT_URI: &str = "http://localhost:1957/callback";
// --- /Constants ---

// Helper to emit events
fn emit_event<T: Serialize + Clone>(event: &str, payload: Option<T>) -> Result<(), String> {
    let binding = GLOBAL_APP_HANDLE.lock().unwrap(); // Ensure GLOBAL_APP_HANDLE is correctly setup
    let app = match binding.as_ref() {
        Some(handle) => handle,
        None => return Err("AppHandle missing or not initialized".to_string()),
    };
    // Prefer get_webview_window over deprecated get_window
    let main_window = app
        .get_webview_window("main")
        .ok_or("Main window not found")?;
    main_window.emit(event, payload).map_err(|e| e.to_string())
}

// Success HTML page
const SUCCESS_HTML: &str = r#"
<!DOCTYPE html>
<html>
<head>
    <title>Autenticación Exitosa</title>
    <style>
        body { font-family: Arial, sans-serif; text-align: center; padding-top: 50px; background-color: #f0f0f0; color: #333;}
        .success { color: #4CAF50; font-size: 24px; }
        .container { max-width: 500px; margin: 40px auto; padding: 20px; background-color: #fff; border-radius: 8px; box-shadow: 0 2px 4px rgba(0,0,0,0.1); }
        p { font-size: 16px; line-height: 1.5; }
    </style>
</head>
<body>
    <div class="container">
        <h1 class="success">¡Autenticación Exitosa!</h1>
        <p>Has iniciado sesión correctamente. Puedes cerrar esta ventana y volver a la aplicación.</p>
        <p><em>Esta ventana se cerrará automáticamente.</em></p>
    </div>
    <script>
        // Cerrar esta ventana automáticamente después de 2 segundos
        setTimeout(() => window.close(), 2000);
    </script>
</body>
</html>
"#;

// Estructura para compartir el estado de autenticación con los manejadores HTTP
// Note: This AppState now holds an Arc<AuthState>, allowing shared ownership
//       between the server handler and potentially other parts if needed.
struct AppState {
    auth_state: Arc<AuthState>, // This holds the shared AuthState reference
    server_tx: Option<tokio::sync::oneshot::Sender<()>>,
}

// HTTP server handler for OAuth callback
async fn handle_callback(
    req: Request<Body>,
    // The handler receives the shared state wrapped in Arc<Mutex<AppState>>
    app_state_mutex: Arc<Mutex<AppState>>,
) -> Result<Response<Body>, Infallible> {
    let uri = req.uri();
    let path = uri.path();

    // Only process the /callback path
    if path != "/callback" {
        let mut response = Response::new(Body::from("Not Found"));
        *response.status_mut() = StatusCode::NOT_FOUND;
        return Ok(response);
    }

    // Extract the authorization code from the query string
    let query = uri.query().unwrap_or("");
    let code = query.split('&').find_map(|pair| {
        let mut parts = pair.splitn(2, '=');
        if parts.next() == Some("code") {
            parts.next().map(|v| v.to_string())
        } else {
            None
        }
    });

    if let Some(code_str) = code {
        // Lock the AppState mutex to access its fields
        let mut state = app_state_mutex.lock().await;

        // Lock the auth_code Mutex within the shared AuthState
        // state.auth_state is Arc<AuthState>, it derefs to AuthState
        let mut auth_code_guard = state.auth_state.auth_code.lock().await;
        *auth_code_guard = Some(code_str);
        drop(auth_code_guard); // Release lock promptly

        // Send signal to shut down the server
        if let Some(tx) = state.server_tx.take() {
            let _ = tx.send(()); // Ignore error if receiver dropped
        }

        // Return success page
        let mut response = Response::new(Body::from(SUCCESS_HTML));
        response.headers_mut().insert(
            hyper::header::CONTENT_TYPE,
            HeaderValue::from_static("text/html; charset=utf-8"),
        );
        Ok(response)
    } else {
        // Error if no code found
        eprintln!("OAuth Callback Error: No authorization code received."); // Log error server-side
        let mut response = Response::new(Body::from(
            "Error: No authorization code received. Check Discord consent screen.",
        ));
        *response.status_mut() = StatusCode::BAD_REQUEST;
        Ok(response)
    }
}

// Tauri commands

#[tauri::command]
pub async fn get_current_session(
    // State now manages Arc<AuthState>
    auth_state: State<'_, Arc<AuthState>>,
) -> Result<Option<UserSession>, String> {
    // Accessing fields works via Deref on Arc
    let session_guard = auth_state.session.lock().await;
    Ok(session_guard.clone())
}

#[tauri::command]
pub async fn start_discord_auth(
    // State now manages Arc<AuthState>
    auth_state: State<'_, Arc<AuthState>>,
) -> Result<(), String> {
    emit_event("auth-step-changed", Some(AuthStep::StartingAuth))?;

    // Clear previous auth code using Deref on Arc
    let mut auth_code_guard = auth_state.auth_code.lock().await;
    *auth_code_guard = None;
    drop(auth_code_guard); // Release lock

    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    // Create the shared state for HTTP handlers.
    // Clone the Arc managed by Tauri state. auth_state.inner() returns &Arc<AuthState>
    let shared_auth_state = Arc::clone(auth_state.inner());
    let app_state_mutex = Arc::new(Mutex::new(AppState {
        auth_state: shared_auth_state, // Move the cloned Arc here
        server_tx: Some(shutdown_tx),
    }));

    // Configure and start the Hyper server
    let addr = SocketAddr::from(([127, 0, 0, 1], 1957));

    // Clone the Arc<Mutex<AppState>> for the service closure
    let app_state_mutex_clone = app_state_mutex.clone();
    let make_svc = make_service_fn(move |_conn| {
        let app_state = app_state_mutex_clone.clone();
        async move {
            Ok::<_, Infallible>(service_fn(move |req| {
                // Clone the Arc again for the handler call
                handle_callback(req, app_state.clone())
            }))
        }
    });

    let server = Server::bind(&addr)
        .serve(make_svc)
        .with_graceful_shutdown(async {
            shutdown_rx.await.ok(); // Wait for the shutdown signal
            println!("Callback server shutting down."); // Log shutdown
        });

    let auth_state_clone_for_poll = Arc::clone(auth_state.inner());

    let app_handle = {
        let binding = GLOBAL_APP_HANDLE.lock().unwrap();
        match binding.as_ref() {
            Some(handle) => handle.clone(), // Asumiendo que AppHandle implementa Clone
            None => {
                return Err("AppHandle not initialized".to_string());
            }
        }
    };

    // Run the server in a background task
    tokio::spawn(async move {
        println!("Callback server listening on http://{}", addr);
        if let Err(e) = server.await {
            eprintln!("Server error: {}", e);
            let _ =
                emit_event::<String>("auth-error", Some(format!("Callback server failed: {}", e)));
        }
    });

    // Open Discord auth URL in browser
    let discord_url = format!(
        "https://discord.com/api/oauth2/authorize?client_id={}&response_type=code&scope=identify%20email%20guilds&redirect_uri={}", // Added email+guilds scope example
        CLIENT_ID, REDIRECT_URI
    );

    println!("Opening Discord auth URL: {}", discord_url); // Log URL
    tauri_plugin_opener::open_url(discord_url, None::<String>).map_err(|e| {
        eprintln!("Failed to open URL: {}", e);
        "Failed to open Discord auth URL".to_string()
    })?;
    emit_event("auth-step-changed", Some(AuthStep::WaitingCallback))?;

    // Spawn task to poll for the auth code and exchange it for tokens/session
    // Clone the Arc<AuthState> again for this separate task
    let auth_state_clone_for_poll = Arc::clone(auth_state.inner());
    tokio::spawn(async move {
        const MAX_WAIT_SECS: u64 = 120; // 2 minutes timeout
        for i in 0..MAX_WAIT_SECS {
            // Check if code exists
            let code_option = {
                // Scoped lock
                let auth_code_guard = auth_state_clone_for_poll.auth_code.lock().await;
                auth_code_guard.clone() // Clone the Option<String>
            }; // Lock released here

            if let Some(code) = code_option {
                println!("Auth code received. Processing...");
                let _ = emit_event("auth-step-changed", Some(AuthStep::ProcessingCallback));

                // -- Focus to main window --
                // Usar app_handle que ya ha sido clonado y es Send
                match app_handle.get_webview_window("main") {
                    Some(main_window) => {
                        if let Err(e) = main_window.set_focus() {
                            eprintln!("Failed to focus main window: {:?}", e);
                        } else {
                            println!("Main window focused");
                        }
                    }
                    None => {
                        eprintln!("Main window not found");
                        // Continuar sin enfocar
                    }
                };

                // --- Exchange code for tokens ---
                let client = Client::new();
                let token_endpoint = format!("{}/auth/discord/callback?code={}", API_ENDPOINT, code);
                println!("Requesting tokens from: {}", token_endpoint);

                match client.get(&token_endpoint).send().await {
                    Ok(resp) => {
                        if !resp.status().is_success() {
                            let status = resp.status();
                            let error_body = resp
                                .text()
                                .await
                                .unwrap_or_else(|_| "Could not read error body".to_string());
                            eprintln!("Token API error: {} - {}", status, error_body);
                            // Return the raw json object to frontend (Without formatting, just json)
                            let _ = emit_event::<String>("auth-error", Some(error_body.clone()));

                        
                     
                            
                            return; // Stop processing on error
                        }

                        match resp.json::<TokenResponse>().await {
                            Ok(tokens) => {
                                println!("Tokens received successfully.");
                                println!("Tokens: {:?}", tokens);
                                // Store tokens
                                {
                                    // Scoped lock
                                    let mut tokens_guard =
                                        auth_state_clone_for_poll.tokens.lock().await;
                                    *tokens_guard = Some(tokens.clone());
                                } // Lock released

                                // --- Fetch user session ---
                                let _ = emit_event(
                                    "auth-step-changed",
                                    Some(AuthStep::RequestingSession),
                                );
                                let session_endpoint = format!("{}/auth/me", API_ENDPOINT);
                                println!("Requesting user session from: {}", session_endpoint);

                                match client
                                    .get(&session_endpoint)
                                    .bearer_auth(&tokens.access_token)
                                    .send()
                                    .await
                                {
                                    Ok(user_resp) => {
                                        if !user_resp.status().is_success() {
                                            let status = user_resp.status();
                                            let error_body =
                                                user_resp.text().await.unwrap_or_else(|_| {
                                                    "Could not read error body".to_string()
                                                });
                                            eprintln!(
                                                "Session API error: {} - {}",
                                                status, error_body
                                            );
                                            let _ = emit_event::<String>(
                                                "auth-error",
                                                Some(format!(
                                                    "API session error: {} - {}",
                                                    status, error_body
                                                )),
                                            );
                                            // Optionally clear tokens here if session fails?
                                            return; // Stop processing
                                        }

                                        match user_resp.json::<UserSession>().await {
                                            Ok(user) => {
                                                println!("User session received: {}", user.extra);
                                                // Store session
                                                {
                                                    // Scoped lock
                                                    let mut session_guard =
                                                        auth_state_clone_for_poll
                                                            .session
                                                            .lock()
                                                            .await;
                                                    *session_guard = Some(user.clone());
                                                } // Lock released
                                                  // Emit success with user data
                                                let _ =
                                                    emit_event("auth-status-changed", Some(user));
                                                return; // Authentication successful! Exit task.
                                            }
                                            Err(e) => {
                                                eprintln!("Failed to parse user session: {}", e);
                                                let _ = emit_event::<String>(
                                                    "auth-error",
                                                    Some(format!(
                                                        "Failed to parse user session: {}",
                                                        e
                                                    )),
                                                );
                                                return; // Stop processing
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        eprintln!("Failed to request user session: {}", e);
                                        let _ = emit_event::<String>(
                                            "auth-error",
                                            Some(format!("Failed to request session: {}", e)),
                                        );
                                        return; // Stop processing
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!("Failed to parse token response: {}", e);
                                let _ = emit_event::<String>(
                                    "auth-error",
                                    Some(format!("Failed to parse tokens: {}", e)),
                                );
                                return; // Stop processing
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to call token API: {}", e);
                        let _ = emit_event::<String>(
                            "auth-error",
                            Some(format!("Failed to call token API: {}", e)),
                        );
                        return; // Stop processing
                    }
                }
            }

            // Wait 1 second before checking again
            if i % 10 == 0 && i > 0 {
                // Log progress every 10 seconds
                println!("Waiting for auth code... ({}s / {}s)", i, MAX_WAIT_SECS);
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }

        // If loop finishes, timeout occurred
        eprintln!("Authentication timed out after {} seconds.", MAX_WAIT_SECS);
        let _ = emit_event::<String>("auth-error", Some("Authentication timeout".to_string()));
        // Ensure server is shut down if timeout happens before callback
        let mut state = app_state_mutex.lock().await;
        if let Some(tx) = state.server_tx.take() {
            let _ = tx.send(()); // Ignore error if receiver dropped
        }
        println!("Callback server shut down due to timeout.");
    });

    Ok(())
}

#[tauri::command]
pub async fn poll_session(
    // State now manages Arc<AuthState>
    auth_state: State<'_, Arc<AuthState>>,
) -> Result<Option<UserSession>, String> {
    // Same logic as get_current_session, just potentially called at different times
    let session_guard = auth_state.session.lock().await;
    Ok(session_guard.clone())
}

#[tauri::command]
pub async fn logout(
    // State now manages Arc<AuthState>
    auth_state: State<'_, Arc<AuthState>>,
) -> Result<(), String> {
    println!("Logout requested.");

    // Get current tokens before clearing them
    let tokens_to_revoke = {
        // Scoped lock
        let tokens_guard = auth_state.tokens.lock().await;
        tokens_guard.clone()
    }; // Lock released

    // Clear local state first
    {
        // Scoped lock
        let mut session_guard = auth_state.session.lock().await;
        *session_guard = None;
    } // Lock released
    {
        // Scoped lock
        let mut tokens_guard = auth_state.tokens.lock().await;
        *tokens_guard = None;
    } // Lock released
    {
        // Scoped lock
        let mut code_guard = auth_state.auth_code.lock().await;
        *code_guard = None;
    } // Lock released

    println!("Local session cleared.");

    // Attempt to revoke tokens on the backend (best effort)
    if let Some(tokens) = tokens_to_revoke {
        let logout_endpoint = format!("{}/logout", API_ENDPOINT);
        println!("Calling backend logout: {}", logout_endpoint);
        match Client::new()
            .post(&logout_endpoint)
            .bearer_auth(&tokens.access_token)
            // Optionally send refresh token if backend needs it for full revocation
            // .json(&serde_json::json!({ "refresh_token": tokens.refresh_token }))
            .send()
            .await
        {
            Ok(resp) => {
                if resp.status().is_success() {
                    println!("Backend logout successful.");
                } else {
                    eprintln!("Backend logout failed: Status {}", resp.status());
                }
            }
            Err(e) => {
                eprintln!("Failed to call backend logout: {}", e);
            }
        }
    } else {
        println!("No tokens found to revoke on backend.");
    }

    // Notify frontend
    emit_event("auth-status-changed", Option::<UserSession>::None)?; // Explicitly type None
    println!("Logout complete.");
    Ok(())
}

// Optional: Add a function to initialize state if needed, though usually done in main.rs
// pub fn setup_auth_state() -> Arc<AuthState> {
//     Arc::new(AuthState::new())
// }
