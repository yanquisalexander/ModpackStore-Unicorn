// src-tauri/src/auth/microsoft.rs

use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::async_runtime;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_http::reqwest;

// Estructuras para respuestas de API
#[derive(Deserialize, Debug)]
struct DeviceCodeResponse {
    user_code: String,
    device_code: String,
    verification_uri: String,
    expires_in: u64,
    interval: u64,
    message: String,
}

#[derive(Deserialize, Debug)]
struct TokenResponse {
    access_token: String,
    refresh_token: String,
    expires_in: u64,
}

#[derive(Deserialize, Debug)]
struct XboxAuthResponse {
    Token: String,
    #[serde(rename = "DisplayClaims")]
    display_claims: XboxDisplayClaims,
}

#[derive(Deserialize, Debug)]
struct XboxDisplayClaims {
    xui: Vec<XboxUserInfo>,
}

#[derive(Deserialize, Debug)]
struct XboxUserInfo {
    uhs: String,
}

#[derive(Deserialize, Debug)]
struct XSTSResponse {
    Token: String,
    #[serde(rename = "DisplayClaims")]
    display_claims: XboxDisplayClaims,
}

#[derive(Deserialize, Debug)]
struct MinecraftAuthResponse {
    access_token: String,
    expires_in: u64,
}

#[derive(Deserialize, Debug)]
struct MinecraftProfileResponse {
    id: String,
    name: String,
    skins: Vec<MinecraftSkin>,
}

#[derive(Deserialize, Debug)]
struct MinecraftSkin {
    id: String,
    state: String,
    url: String,
    variant: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MinecraftAccount {
    pub username: String,
    pub uuid: String,
    pub access_token: String,
    pub refresh_token: String,
    pub token_expiration: u64,
    pub account_type: String,
}

// Estructuras para eventos
#[derive(Serialize, Clone)]
pub struct AuthProgressEvent {
    step: String,
    message: String,
    percentage: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    user_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    verification_url: Option<String>,
}

// Constantes de autenticación
const MICROSOFT_CLIENT_ID: &str = "b999888a-cd19-4e13-8ca4-f276a9ba2a68";
const MICROSOFT_AUTH_URL: &str =
    "https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode";
const MICROSOFT_TOKEN_URL: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/token";
const XBOX_AUTH_URL: &str = "https://user.auth.xboxlive.com/user/authenticate";
const XSTS_AUTH_URL: &str = "https://xsts.auth.xboxlive.com/xsts/authorize";
const MINECRAFT_AUTH_URL: &str = "https://api.minecraftservices.com/authentication/login_with_xbox";
const MINECRAFT_PROFILE_URL: &str = "https://api.minecraftservices.com/minecraft/profile";
const ACCOUNT_OWNS_MINECRAFT_URL: &str = "https://api.minecraftservices.com/entitlements/license";

// Clase principal para autenticación
pub struct MicrosoftAuthenticator {
    client: reqwest::Client,
}

impl MicrosoftAuthenticator {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self { client }
    }

    pub fn start_authentication(&self, app_handle: AppHandle) {
        // Clonamos el cliente HTTP y el app_handle para uso en el hilo
        let client = self.client.clone();
        let app_handle_clone = app_handle.clone();

        // Iniciamos la autenticación en un hilo separado
        thread::spawn(move || {
            let result = async_runtime::block_on(async {
                Self::authenticate(&client, &app_handle_clone).await
            });

            match result {
                Ok(account) => {
                    // Notificamos éxito con la cuenta
                    let _ = app_handle_clone.emit("microsoft-auth-success", account);
                }
                Err(err) => {
                    // Notificamos error
                    let _ = app_handle_clone.emit("microsoft-auth-error", err.to_string());
                }
            }
        });
    }

    async fn authenticate(
        client: &reqwest::Client,
        app_handle: &AppHandle,
    ) -> Result<MinecraftAccount, Box<dyn std::error::Error>> {
        // Paso 1: Obtener código de dispositivo
        Self::emit_progress(
            app_handle,
            "device_code",
            "Solicitando código de dispositivo...",
            0,
            None,
            None,
        );

        let device_code_response = Self::get_device_code(client).await?;

        Self::emit_progress(
            app_handle,
            "waiting_auth",
            "Por favor, visita el sitio web y usa el código para autenticarte",
            10,
            Some(device_code_response.user_code.clone()),
            Some(device_code_response.verification_uri.clone()),
        );

        // Paso 2: Esperar a que el usuario se autentique
        let token_response = Self::poll_for_token(
            client,
            &device_code_response.device_code,
            device_code_response.interval,
            app_handle,
        )
        .await?;

        Self::emit_progress(
            app_handle,
            "microsoft_token",
            "Autenticación con Microsoft completada",
            30,
            None,
            None,
        );

        // Paso 3: Autenticar con Xbox Live
        Self::emit_progress(
            app_handle,
            "xbox_auth",
            "Autenticando con Xbox Live...",
            40,
            None,
            None,
        );
        let xbox_auth_response =
            Self::authenticate_with_xbox_live(client, &token_response.access_token).await?;

        Self::emit_progress(
            app_handle,
            "xsts_token",
            "Obteniendo token XSTS...",
            50,
            None,
            None,
        );
        let xsts_response = Self::get_xsts_token(client, &xbox_auth_response.Token).await?;

        // Paso 4: Autenticar con Minecraft
        Self::emit_progress(
            app_handle,
            "minecraft_auth",
            "Autenticando con Minecraft...",
            70,
            None,
            None,
        );
        let minecraft_token = Self::authenticate_with_minecraft(
            client,
            &xsts_response.Token,
            &xsts_response.display_claims.xui[0].uhs,
        )
        .await?;

        // Paso 5: Obtener perfil de Minecraft
        Self::emit_progress(
            app_handle,
            "profile",
            "Obteniendo perfil de Minecraft...",
            90,
            None,
            None,
        );
        let profile = Self::get_minecraft_profile(client, &minecraft_token.access_token).await?;

        Self::emit_progress(
            app_handle,
            "complete",
            "Autenticación completada con éxito",
            100,
            None,
            None,
        );

        // Calcular tiempo de expiración
        let expiration = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs()
            + minecraft_token.expires_in;

        // Crear y retornar la cuenta
        let account = MinecraftAccount {
            username: profile.name,
            uuid: profile.id,
            access_token: minecraft_token.access_token,
            refresh_token: token_response.refresh_token,
            token_expiration: expiration,
            account_type: "microsoft".to_string(),
        };

        Ok(account)
    }

    // Emite un evento de progreso al frontend
    fn emit_progress(
        app_handle: &AppHandle,
        step: &str,
        message: &str,
        percentage: u8,
        user_code: Option<String>,
        verification_url: Option<String>,
    ) {
        let event = AuthProgressEvent {
            step: step.to_string(),
            message: message.to_string(),
            percentage,
            user_code,
            verification_url,
        };

        let _ = app_handle.emit("microsoft-auth-progress", event);
    }

    // Obtiene un código de dispositivo para iniciar la autenticación
    async fn get_device_code(
        client: &reqwest::Client,
    ) -> Result<DeviceCodeResponse, Box<dyn std::error::Error>> {
        let params = [
            ("client_id", MICROSOFT_CLIENT_ID),
            ("scope", "XboxLive.signin offline_access"),
        ];

        let response = client
            .post(MICROSOFT_AUTH_URL)
            .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
            .form(&params)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(format!(
                "Error al obtener código de dispositivo: {}",
                response.status()
            )
            .into());
        }

        let device_code: DeviceCodeResponse = response.json().await?;
        Ok(device_code)
    }

    // Espera a que el usuario se autentique con el código proporcionado
    async fn poll_for_token(
        client: &reqwest::Client,
        device_code: &str,
        interval: u64,
        app_handle: &AppHandle,
    ) -> Result<TokenResponse, Box<dyn std::error::Error>> {
        let params = [
            ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ("device_code", device_code),
            ("client_id", MICROSOFT_CLIENT_ID),
        ];

        let max_wait_time = 300; // 5 minutos
        let mut elapsed_time = 0;

        while elapsed_time < max_wait_time {
            tokio::time::sleep(tokio::time::Duration::from_secs(interval)).await;
            elapsed_time += interval;

            let response = client
                .post(MICROSOFT_TOKEN_URL)
                .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
                .form(&params)
                .send()
                .await?;

            if response.status().is_success() {
                return Ok(response.json().await?);
            }

            let error: serde_json::Value = response.json().await?;
            let error_code = error["error"].as_str().unwrap_or("unknown");

            // Si el error es que aún no se ha completado la autenticación, seguimos esperando
            if error_code != "authorization_pending" {
                return Err(format!("Error en la autenticación: {}", error_code).into());
            }

            // Actualizar progreso
            let percentage = 10 + ((elapsed_time as f32 / max_wait_time as f32) * 15.0) as u8;
            Self::emit_progress(
                app_handle,
                "waiting_auth",
                &format!("Esperando autenticación... {}%", percentage),
                percentage,
                None,
                None,
            );
        }

        Err("Tiempo de espera agotado. Por favor, intenta nuevamente.".into())
    }

    // Autentica con Xbox Live usando el token de Microsoft
    async fn authenticate_with_xbox_live(
        client: &reqwest::Client,
        access_token: &str,
    ) -> Result<XboxAuthResponse, Box<dyn std::error::Error>> {
        let request_body = serde_json::json!({
            "Properties": {
                "AuthMethod": "RPS",
                "SiteName": "user.auth.xboxlive.com",
                "RpsTicket": format!("d={}", access_token)
            },
            "RelyingParty": "http://auth.xboxlive.com",
            "TokenType": "JWT"
        });

        let response = client
            .post(XBOX_AUTH_URL)
            .header(CONTENT_TYPE, "application/json")
            .json(&request_body)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(format!("Error al autenticar con Xbox Live: {}", response.status()).into());
        }

        let xbox_auth: XboxAuthResponse = response.json().await?;
        Ok(xbox_auth)
    }

    async fn get_xsts_token(
        client: &reqwest::Client,
        xbox_token: &str,
    ) -> Result<XSTSResponse, Box<dyn std::error::Error>> {
        let request_body = serde_json::json!({
            "Properties": {
                "SandboxId": "RETAIL",
                "UserTokens": [xbox_token]
            },
            "RelyingParty": "rp://api.minecraftservices.com/",
            "TokenType": "JWT"
        });

        let response = client
            .post(XSTS_AUTH_URL)
            .header(CONTENT_TYPE, "application/json")
            .json(&request_body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status(); // Store the status code before consuming response

            // Manejo de errores específicos
            if status.as_u16() == 401 {
                let error_response: serde_json::Value = response.json().await?;
                if let Some(xerr) = error_response.get("XErr").and_then(|x| x.as_u64()) {
                    match xerr {
                        2148916233 => return Err("Esta cuenta de Microsoft no tiene una cuenta de Xbox. Por favor, crea una cuenta de Xbox antes de continuar.".into()),
                        2148916238 => return Err("Esta cuenta es de un menor de edad y requiere consentimiento parental para juegos online.".into()),
                        _ => return Err(format!("Error de Xbox Live: código {}", xerr).into()),
                    }
                }
            }
            return Err(format!("Error al obtener token XSTS: {}", status).into());
        }

        let xsts_response: XSTSResponse = response.json().await?;
        Ok(xsts_response)
    }

    // Autentica con el servicio de Minecraft usando los tokens de Xbox
    async fn authenticate_with_minecraft(
        client: &reqwest::Client,
        xsts_token: &str,
        user_hash: &str,
    ) -> Result<MinecraftAuthResponse, Box<dyn std::error::Error>> {
        let request_body = serde_json::json!({
            "identityToken": format!("XBL3.0 x={};{}", user_hash, xsts_token)
        });

        let response = client
            .post(MINECRAFT_AUTH_URL)
            .header(CONTENT_TYPE, "application/json")
            .json(&request_body)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(format!("Error al autenticar con Minecraft: {}", response.status()).into());
        }

        let minecraft_auth: MinecraftAuthResponse = response.json().await?;
        Ok(minecraft_auth)
    }

    // Obtiene el perfil de Minecraft del usuario autenticado
    async fn get_minecraft_profile(
        client: &reqwest::Client,
        access_token: &str,
    ) -> Result<MinecraftProfileResponse, Box<dyn std::error::Error>> {
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", access_token))?,
        );

        // First check if the user owns Minecraft
        let license_response = client
            .get(ACCOUNT_OWNS_MINECRAFT_URL)
            .query(&[("requestId", uuid::Uuid::new_v4().to_string())])
            .headers(headers.clone())
            .send()
            .await?;

        if !license_response.status().is_success() {
            return Err(
                format!("Error al verificar licencia: {}", license_response.status()).into(),
            );
        }

        let license_data: serde_json::Value = license_response.json().await?;
        println!("License data: {:?}", license_data);

        // Check if user has valid Java Edition license (not trial)
        let has_valid_license = license_data
            .get("items")
            .and_then(|items| items.as_array())
            .map(|items| {
                items.iter().any(|item| {
                    let name = item.get("name").and_then(|n| n.as_str()).unwrap_or("");
                    let source = item.get("source").and_then(|s| s.as_str()).unwrap_or("");

                    (name == "product_minecraft" || name == "game_minecraft") && source != "TRIAL"
                })
            })
            .unwrap_or(false);

        if !has_valid_license {
            return Err("Esta cuenta de Microsoft no tiene una licencia válida de Minecraft Java Edition. Por favor, adquiere el juego antes de continuar.".into());
        }

        // Now check for the profile
        let profile_response = client
            .get(MINECRAFT_PROFILE_URL)
            .headers(headers)
            .send()
            .await?;

        if profile_response.status().as_u16() == 404 {
            return Err("Tu cuenta tiene Minecraft Java Edition adquirido pero aún no has creado un perfil. Por favor, abre el Launcher oficial de Minecraft al menos una vez para crear tu perfil.".into());
        }

        if !profile_response.status().is_success() {
            return Err(format!(
                "Error al obtener perfil de Minecraft: {}",
                profile_response.status()
            )
            .into());
        }

        let profile: MinecraftProfileResponse = profile_response.json().await?;
        println!("Minecraft profile: {:?}", profile);

        Ok(profile)
    }

    // Refresca un token expirado
    pub async fn refresh_token(
        &self,
        refresh_token: &str,
    ) -> Result<TokenResponse, Box<dyn std::error::Error>> {
        let params = [
            ("client_id", MICROSOFT_CLIENT_ID),
            ("refresh_token", refresh_token),
            ("grant_type", "refresh_token"),
        ];

        let response = self
            .client
            .post(MICROSOFT_TOKEN_URL)
            .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
            .form(&params)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(format!("Error al refrescar token: {}", response.status()).into());
        }

        let token_response: TokenResponse = response.json().await?;
        Ok(token_response)
    }
}

#[tauri::command]
pub fn start_microsoft_auth(app_handle: AppHandle) {
    let authenticator = MicrosoftAuthenticator::new();
    authenticator.start_authentication(app_handle);
}
