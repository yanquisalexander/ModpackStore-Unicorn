use crate::GLOBAL_APP_HANDLE;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use tauri::Emitter;
use tauri::{Manager, State};
use tauri_plugin_http::reqwest::Client;
use tauri_plugin_opener;
use tauri_plugin_store::StoreExt;
use tokio::sync::Mutex;

// Importaciones de Hyper
use hyper::header::HeaderValue;
use hyper::server::Server;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, StatusCode as HyperStatusCode};
use tauri_plugin_http::reqwest::StatusCode;


use crate::API_ENDPOINT;

// Constantes para el almacenamiento
const STORAGE_PATH: &str = "auth_store.json";
const STORAGE_KEY_TOKENS: &str = "auth_tokens";

// User session structure
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserSession {
   #[serde(flatten)]
   pub extra: serde_json::Value,
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

// Auth state refactorizado para minimizar el uso de Mutex
#[derive(Debug)]
pub struct AuthState {
    pub session: Mutex<Option<UserSession>>,
    pub auth_code: Mutex<Option<String>>,
    // Tokens se guardarán en store, no en memoria
}

impl AuthState {
    pub fn new() -> Self {
        Self {
            session: Mutex::new(None),
            auth_code: Mutex::new(None),
        }
    }
}

// --- Constants ---
const CLIENT_ID: &str = "943184136976334879";
const REDIRECT_URI: &str = "http://localhost:1957/callback";
// --- /Constants ---

// Helper para emitir eventos (optimizado para evitar repetición de código)
fn emit_event<T: Serialize + Clone>(event: &str, payload: Option<T>) -> Result<(), String> {
    let binding = GLOBAL_APP_HANDLE.lock().unwrap();
    let app = binding.as_ref().ok_or("AppHandle no inicializado")?;
    
    let main_window = app
        .get_webview_window("main")
        .ok_or("Ventana principal no encontrada")?;
    
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
        setTimeout(() => window.close(), 2000);
    </script>
</body>
</html>
"#;

// Estado para el servidor HTTP
struct AppState {
    auth_state: Arc<AuthState>,
    server_tx: Option<tokio::sync::oneshot::Sender<()>>,
}

// Helper para guardar tokens en el store (nueva sintaxis)
async fn save_tokens_to_store(app_handle: &tauri::AppHandle, tokens: &TokenResponse) -> Result<(), String> {
    // Este sí devuelve Result, así que usamos map_err
    let store = app_handle
        .store(STORAGE_PATH)
        .map_err(|e| e.to_string())?;

    // Este NO devuelve Result, así que no uses `?`
    store.set(STORAGE_KEY_TOKENS.to_string(), json!(tokens));

    // Este sí devuelve Result
    store.save().map_err(|e| e.to_string())?;

    // Cierre del recurso, probablemente sin fallo también
    store.close_resource();

    Ok(())
}


// Helper para cargar tokens desde el store (nueva sintaxis)
async fn load_tokens_from_store(app_handle: &tauri::AppHandle) -> Result<Option<TokenResponse>, String> {
    let store = app_handle.store(STORAGE_PATH)
        .map_err(|e| format!("Error al acceder al store: {}", e))?;
    
    let result = if store.has(STORAGE_KEY_TOKENS) {
        let tokens_value = store.get(STORAGE_KEY_TOKENS)
            .ok_or_else(|| "No se pudieron obtener los tokens del store".to_string())?;
        
        match serde_json::from_value::<TokenResponse>(tokens_value.clone()) {
            Ok(tokens) => Ok(Some(tokens)),
            Err(e) => Err(format!("Error al deserializar tokens: {}", e)),
        }
    } else {
        Ok(None)
    };
    
    // Opcional: cerrar el recurso después de usarlo
    store.close_resource();
    
    result
}

// Helper para eliminar tokens del store (nueva sintaxis)
async fn remove_tokens_from_store(app_handle: &tauri::AppHandle) -> Result<(), String> {
    let store = app_handle.store(STORAGE_PATH)
        .map_err(|e| format!("Error al acceder al store: {}", e))?;
    
    if store.has(STORAGE_KEY_TOKENS) {
        store.delete(STORAGE_KEY_TOKENS.to_string());
    }
    
    store.save()
        .map_err(|e| format!("Error al guardar cambios en el store: {}", e))?;
    
    // Opcional: cerrar el recurso después de usarlo
    store.close_resource();
    
    Ok(())
}

// HTTP server handler for OAuth callback
async fn handle_callback(
    req: Request<Body>,
    app_state_mutex: Arc<Mutex<AppState>>,
) -> Result<Response<Body>, Infallible> {
    let uri = req.uri();
    let path = uri.path();

    // Solo procesar la ruta /callback
    if path != "/callback" {
        let mut response = Response::new(Body::from("Not Found"));
        *response.status_mut() = HyperStatusCode::NOT_FOUND;
        return Ok(response);
    }

    // Extraer el código de autorización del query string
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
        // Obtener acceso al estado compartido
        let mut state = app_state_mutex.lock().await;

        // Guardar el código de autorización
        let mut auth_code_guard = state.auth_state.auth_code.lock().await;
        *auth_code_guard = Some(code_str);
        drop(auth_code_guard);

        // Enviar señal para apagar el servidor
        if let Some(tx) = state.server_tx.take() {
            let _ = tx.send(());
        }

        // Devolver página de éxito
        let mut response = Response::new(Body::from(SUCCESS_HTML));
        response.headers_mut().insert(
            hyper::header::CONTENT_TYPE,
            HeaderValue::from_static("text/html; charset=utf-8"),
        );
        Ok(response)
    } else {
        // Error si no se encuentra un código
        eprintln!("OAuth Callback Error: No se recibió código de autorización.");
        let mut response = Response::new(Body::from(
            "Error: No se recibió código de autorización. Verifica la pantalla de consentimiento de Discord.",
        ));
        *response.status_mut() = HyperStatusCode::BAD_REQUEST;
        Ok(response)
    }
}

// Comandos de Tauri

// Nuevo comando para inicializar la sesión al inicio de la aplicación
#[tauri::command]
pub async fn init_session(
    app_handle: tauri::AppHandle,
    auth_state: State<'_, Arc<AuthState>>,
) -> Result<Option<UserSession>, String> {
    // Intentar cargar tokens desde el store
    match load_tokens_from_store(&app_handle).await {
        Ok(Some(tokens)) => {
            // Si tenemos tokens guardados, verificar la sesión del usuario
            println!("Tokens encontrados en store, verificando sesión...");
            
            let client = Client::new();
            let session_endpoint = format!("{}/auth/me", API_ENDPOINT);
            
            match client
                .get(&session_endpoint)
                .bearer_auth(&tokens.access_token)
                .send()
                .await
            {
                Ok(user_resp) => {
                    if user_resp.status().is_success() {
                        match user_resp.json::<UserSession>().await {
                            Ok(user) => {
                                println!("Sesión recuperada con éxito");
                                // Guardar la sesión en memoria
                                let mut session_guard = auth_state.session.lock().await;
                                *session_guard = Some(user.clone());
                                drop(session_guard);
                                
                                // Notificar al frontend
                                let _ = emit_event("auth-status-changed", Some(user.clone()));
                                
                                return Ok(Some(user));
                            },
                            Err(e) => {
                                eprintln!("Error al parsear datos de sesión: {}", e);
                                // Si hay error de parseo, eliminar tokens
                                let _ = remove_tokens_from_store(&app_handle).await;
                            }
                        }
                    } 
                    // can't compare tauri_plugin_http::reqwest::StatusCode with hyper::StatusCode
                    else if user_resp.status() == StatusCode::UNAUTHORIZED {

                        println!("Tokens expirados, intentando renovar...");
                        // Aquí podrías implementar renovación de tokens con refresh_token
                        // Por ahora solo eliminamos los tokens
                        let _ = remove_tokens_from_store(&app_handle).await;
                    } else {
                        eprintln!("Error al verificar sesión: {}", user_resp.status());
                        let _ = remove_tokens_from_store(&app_handle).await;
                    }
                },
                Err(e) => {
                    eprintln!("Error al contactar API: {}", e);
                }
            }
        },
        Ok(None) => {
            println!("No hay tokens guardados");
        },
        Err(e) => {
            eprintln!("Error al cargar tokens: {}", e);
        }
    }
    
    Ok(None)
}

#[tauri::command]
pub async fn get_current_session(
    auth_state: State<'_, Arc<AuthState>>,
) -> Result<Option<UserSession>, String> {
    let session_guard = auth_state.session.lock().await;
    Ok(session_guard.clone())
}

#[tauri::command]
pub async fn start_discord_auth(
    app_handle: tauri::AppHandle,
    auth_state: State<'_, Arc<AuthState>>,
) -> Result<(), String> {
    emit_event("auth-step-changed", Some(AuthStep::StartingAuth))?;

    // Limpiar código de autorización previo
    let mut auth_code_guard = auth_state.auth_code.lock().await;
    *auth_code_guard = None;
    drop(auth_code_guard);

    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    // Crear estado compartido para los manejadores HTTP
    let shared_auth_state = Arc::clone(auth_state.inner());
    let app_state_mutex = Arc::new(Mutex::new(AppState {
        auth_state: shared_auth_state,
        server_tx: Some(shutdown_tx),
    }));

    // Configurar y iniciar el servidor Hyper
    let addr = SocketAddr::from(([127, 0, 0, 1], 1957));

    let app_state_mutex_clone = app_state_mutex.clone();
    let make_svc = make_service_fn(move |_conn| {
        let app_state = app_state_mutex_clone.clone();
        async move {
            Ok::<_, Infallible>(service_fn(move |req| {
                handle_callback(req, app_state.clone())
            }))
        }
    });

    let server = Server::bind(&addr)
        .serve(make_svc)
        .with_graceful_shutdown(async {
            shutdown_rx.await.ok();
            println!("Servidor de callback apagándose.");
        });

    // Ejecutar servidor en tarea de fondo
    tokio::spawn(async move {
        println!("Servidor de callback escuchando en http://{}", addr);
        if let Err(e) = server.await {
            eprintln!("Error del servidor: {}", e);
            let _ = emit_event::<String>("auth-error", Some(format!("Error del servidor: {}", e)));
        }
    });

    // Abrir URL de autenticación de Discord en el navegador
    let discord_url = format!(
        "https://discord.com/api/oauth2/authorize?client_id={}&response_type=code&scope=identify%20email%20guilds&redirect_uri={}",
        CLIENT_ID, REDIRECT_URI
    );

    println!("Abriendo URL de autenticación: {}", discord_url);
    tauri_plugin_opener::open_url(discord_url, None::<String>).map_err(|e| {
        eprintln!("Error al abrir URL: {}", e);
        "Error al abrir URL de autenticación".to_string()
    })?;
    
    emit_event("auth-step-changed", Some(AuthStep::WaitingCallback))?;

    // Clonar los handles necesarios para la tarea de polling
    let auth_state_clone = Arc::clone(auth_state.inner());
    let app_handle_clone = app_handle.clone();
    
    // Tarea para esperar el código de autorización y procesarlo
    tokio::spawn(async move {
        const MAX_WAIT_SECS: u64 = 120; // 2 minutos de timeout
        
        for i in 0..MAX_WAIT_SECS {
            // Verificar si existe el código
            let code_option = {
                let auth_code_guard = auth_state_clone.auth_code.lock().await;
                auth_code_guard.clone()
            };

            if let Some(code) = code_option {
                println!("Código de autenticación recibido. Procesando...");
                let _ = emit_event("auth-step-changed", Some(AuthStep::ProcessingCallback));

                // Enfocar la ventana principal
                if let Some(main_window) = app_handle_clone.get_webview_window("main") {
                    if let Err(e) = main_window.set_focus() {
                        eprintln!("Error al enfocar ventana principal: {:?}", e);
                    }
                }

                // Intercambiar código por tokens
                let client = Client::new();
                let token_endpoint = format!("{}/auth/discord/callback?code={}", API_ENDPOINT, code);
                println!("Solicitando tokens desde: {}", token_endpoint);

                match client.get(&token_endpoint).send().await {
                    Ok(resp) => {
                        if !resp.status().is_success() {
                            let status = resp.status();
                            let error_body = resp.text().await.unwrap_or_else(|_| "No se pudo leer el cuerpo del error".to_string());
                            eprintln!("Error de API de tokens: {} - {}", status, error_body);
                            let _ = emit_event::<String>("auth-error", Some(error_body));
                            return;
                        }

                        match resp.json::<TokenResponse>().await {
                            Ok(tokens) => {
                                println!("Tokens recibidos correctamente.");
                                
                                // Guardar tokens en el store
                                if let Err(e) = save_tokens_to_store(&app_handle_clone, &tokens).await {
                                    eprintln!("Error al guardar tokens: {}", e);
                                    // Continuar a pesar del error para intentar completar el flujo
                                }

                                // Solicitar sesión de usuario
                                let _ = emit_event("auth-step-changed", Some(AuthStep::RequestingSession));
                                let session_endpoint = format!("{}/auth/me", API_ENDPOINT);
                                println!("Solicitando sesión de usuario desde: {}", session_endpoint);

                                match client
                                    .get(&session_endpoint)
                                    .bearer_auth(&tokens.access_token)
                                    .send()
                                    .await
                                {
                                    Ok(user_resp) => {
                                        if !user_resp.status().is_success() {
                                            let status = user_resp.status();
                                            let error_body = user_resp.text().await
                                                .unwrap_or_else(|_| "No se pudo leer el cuerpo del error".to_string());
                                            eprintln!("Error de API de sesión: {} - {}", status, error_body);
                                            let _ = emit_event::<String>(
                                                "auth-error", 
                                                Some(format!("Error de API de sesión: {} - {}", status, error_body))
                                            );
                                            return;
                                        }

                                        match user_resp.json::<UserSession>().await {
                                            Ok(user) => {
                                                println!("Sesión de usuario recibida: {}", user.extra);
                                                
                                                // Guardar sesión
                                                {
                                                    let mut session_guard = auth_state_clone.session.lock().await;
                                                    *session_guard = Some(user.clone());
                                                }
                                                
                                                // Notificar éxito con datos de usuario
                                                let _ = emit_event("auth-status-changed", Some(user));
                                                return;
                                            },
                                            Err(e) => {
                                                eprintln!("Error al parsear sesión de usuario: {}", e);
                                                let _ = emit_event::<String>(
                                                    "auth-error",
                                                    Some(format!("Error al parsear sesión: {}", e))
                                                );
                                                return;
                                            }
                                        }
                                    },
                                    Err(e) => {
                                        eprintln!("Error al solicitar sesión de usuario: {}", e);
                                        let _ = emit_event::<String>(
                                            "auth-error",
                                            Some(format!("Error al solicitar sesión: {}", e))
                                        );
                                        return;
                                    }
                                }
                            },
                            Err(e) => {
                                eprintln!("Error al parsear respuesta de tokens: {}", e);
                                let _ = emit_event::<String>(
                                    "auth-error",
                                    Some(format!("Error al parsear tokens: {}", e))
                                );
                                return;
                            }
                        }
                    },
                    Err(e) => {
                        eprintln!("Error al llamar API de tokens: {}", e);
                        let _ = emit_event::<String>(
                            "auth-error",
                            Some(format!("Error al llamar API de tokens: {}", e))
                        );
                        return;
                    }
                }
            }

            // Esperar 1 segundo antes de verificar de nuevo
            if i % 10 == 0 && i > 0 {
                println!("Esperando código de autenticación... ({}s / {}s)", i, MAX_WAIT_SECS);
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }

        // Si el bucle termina, ocurrió un timeout
        eprintln!("Autenticación expiró después de {} segundos.", MAX_WAIT_SECS);
        let _ = emit_event::<String>("auth-error", Some("Timeout de autenticación".to_string()));
        
        // Asegurar que el servidor se apague si hay timeout antes del callback
        let mut state = app_state_mutex.lock().await;
        if let Some(tx) = state.server_tx.take() {
            let _ = tx.send(());
        }
        println!("Servidor de callback apagado por timeout.");
    });

    Ok(())
}

#[tauri::command]
pub async fn poll_session(
    auth_state: State<'_, Arc<AuthState>>,
) -> Result<Option<UserSession>, String> {
    let session_guard = auth_state.session.lock().await;
    Ok(session_guard.clone())
}

#[tauri::command]
pub async fn logout(
    app_handle: tauri::AppHandle,
    auth_state: State<'_, Arc<AuthState>>,
) -> Result<(), String> {
    println!("Logout solicitado.");

    // Obtener tokens actuales para revocarlos
    let tokens_to_revoke = load_tokens_from_store(&app_handle).await.ok().flatten();

    // Limpiar estado local
    {
        let mut session_guard = auth_state.session.lock().await;
        *session_guard = None;
    }
    {
        let mut code_guard = auth_state.auth_code.lock().await;
        *code_guard = None;
    }
    
    // Eliminar tokens del store
    if let Err(e) = remove_tokens_from_store(&app_handle).await {
        eprintln!("Error al eliminar tokens del store: {}", e);
    }

    println!("Sesión local eliminada.");

    // Intentar revocar tokens en el backend
    if let Some(tokens) = tokens_to_revoke {
        let logout_endpoint = format!("{}/logout", API_ENDPOINT);
        println!("Llamando logout del backend: {}", logout_endpoint);
        
        match Client::new()
            .post(&logout_endpoint)
            .bearer_auth(&tokens.access_token)
            .send()
            .await
        {
            Ok(resp) => {
                if resp.status().is_success() {
                    println!("Logout en backend exitoso.");
                } else {
                    eprintln!("Logout en backend falló: Estado {}", resp.status());
                }
            },
            Err(e) => {
                eprintln!("Error al llamar logout de backend: {}", e);
            }
        }
    } else {
        println!("No se encontraron tokens para revocar en el backend.");
    }

    // Notificar al frontend
    emit_event("auth-status-changed", Option::<UserSession>::None)?;
    println!("Logout completo.");
    Ok(())
}

// Opcional: función para verificar la validez de los tokens y renovarlos si es necesario
#[tauri::command]
pub async fn refresh_tokens(
    app_handle: tauri::AppHandle,
    auth_state: State<'_, Arc<AuthState>>,
) -> Result<bool, String> {
    // Cargar tokens del store
    let current_tokens = match load_tokens_from_store(&app_handle).await {
        Ok(Some(tokens)) => tokens,
        Ok(None) => return Ok(false), // No hay tokens para renovar
        Err(e) => return Err(e),
    };
    
    // Lógica para renovar tokens (depende de tu API)
    let client = Client::new();
    let refresh_endpoint = format!("{}/auth/refresh", API_ENDPOINT);
    
    match client
        .post(&refresh_endpoint)
        .json(&json!({
            "refresh_token": current_tokens.refresh_token
        }))
        .send()
        .await
    {
        Ok(resp) => {
            if resp.status().is_success() {
                match resp.json::<TokenResponse>().await {
                    Ok(new_tokens) => {
                        // Guardar nuevos tokens
                        if let Err(e) = save_tokens_to_store(&app_handle, &new_tokens).await {
                            return Err(format!("Error al guardar tokens renovados: {}", e));
                        }
                        
                        println!("Tokens renovados exitosamente");
                        Ok(true)
                    },
                    Err(e) => Err(format!("Error al parsear tokens renovados: {}", e))
                }
            } else {
                // Si hay error en la renovación, limpiar tokens
                let _ = remove_tokens_from_store(&app_handle).await;
                
                // Limpiar sesión
                let mut session_guard = auth_state.session.lock().await;
                *session_guard = None;
                
                // Notificar cambio de estado al frontend
                let _ = emit_event("auth-status-changed", Option::<UserSession>::None);
                
                Err(format!("Error al renovar tokens: {}", resp.status()))
            }
        },
        Err(e) => Err(format!("Error al llamar API de renovación: {}", e))
    }
}

// Función para registrar el estado de autenticación en main.rs
pub fn setup_auth(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    // Registrar el estado de autenticación
    app.manage(Arc::new(AuthState::new()));
    println!("Estado de autenticación inicializado");
    Ok(())
}