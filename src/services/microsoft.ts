// src/services/microsoft.ts
import { invoke } from "@tauri-apps/api/core";
import { MICROSOFT_CLIENT_ID } from "@/consts";

// Interfaces para las respuestas de las APIs
interface DeviceCodeResponse {
    user_code: string;
    device_code: string;
    verification_uri: string;
    expires_in: number;
    interval: number;
    message: string;
}

interface TokenResponse {
    access_token: string;
    refresh_token: string;
    expires_in: number;
    token_type: string;
}

interface XboxAuthResponse {
    Token: string;
    DisplayClaims: {
        xui: Array<{
            uhs: string;
        }>;
    };
}

interface XSTSResponse {
    Token: string;
    DisplayClaims: {
        xui: Array<{
            uhs: string;
        }>;
    };
}

interface MinecraftAuthResponse {
    access_token: string;
    expires_in: number;
}

interface MinecraftProfileResponse {
    id: string;
    name: string;
    skins: Array<{
        id: string;
        state: string;
        url: string;
        variant: string;
        alias: string;
    }>;
    capes: Array<any>;
}

// Estados para el seguimiento del proceso de autenticación
export interface AuthProgress {
    step: 'device_code' | 'waiting_auth' | 'microsoft_token' | 'xbox_auth' | 'xsts_token' | 'minecraft_auth' | 'profile' | 'complete';
    message: string;
    percentage: number;
    userCode?: string;
    verificationUrl?: string;
}

export interface MicrosoftAccount {
    username: string;
    uuid: string;
    accessToken: string;
    refreshToken: string;
    tokenExpiration: number;
    accountType: 'microsoft';
}

export class MicrosoftAuthService {
    readonly MICROSOFT_AUTH_URL = "https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode";
    readonly MICROSOFT_TOKEN_URL = "https://login.microsoftonline.com/consumers/oauth2/v2.0/token";
    readonly XBOX_AUTH_URL = "https://user.auth.xboxlive.com/user/authenticate";
    readonly XSTS_AUTH_URL = "https://xsts.auth.xboxlive.com/xsts/authorize";
    readonly MINECRAFT_AUTH_URL = "https://api.minecraftservices.com/authentication/login_with_xbox";
    readonly MINECRAFT_PROFILE_URL = "https://api.minecraftservices.com/minecraft/profile";

    private readonly clientId: string;

    constructor(clientId: string = MICROSOFT_CLIENT_ID) {
        this.clientId = clientId;
    }

    /**
     * Inicia el proceso de autenticación mediante device code
     * @param progressCallback Callback para informar del progreso de autenticación
     * @returns La cuenta completa de Microsoft cuando la autenticación sea exitosa
     */
    public async authenticate(
        progressCallback: (progress: AuthProgress) => void
    ): Promise<MicrosoftAccount> {
        try {
            // Paso 1: Obtener código de dispositivo
            progressCallback({
                step: 'device_code',
                message: 'Solicitando código de dispositivo...',
                percentage: 0
            });

            const deviceCodeResponse = await this.getDeviceCode();

            progressCallback({
                step: 'waiting_auth',
                message: 'Por favor, visita el sitio web y usa el código para autenticarte',
                percentage: 10,
                userCode: deviceCodeResponse.user_code,
                verificationUrl: deviceCodeResponse.verification_uri
            });

            // Paso 2: Esperar a que el usuario se autentique y obtener tokens
            const tokenResponse = await this.pollForTokens(
                deviceCodeResponse.device_code,
                deviceCodeResponse.interval,
                progressCallback
            );

            progressCallback({
                step: 'microsoft_token',
                message: 'Autenticación con Microsoft completada',
                percentage: 30
            });

            // Paso 3: Autenticar con Xbox Live
            progressCallback({
                step: 'xbox_auth',
                message: 'Autenticando con Xbox Live...',
                percentage: 40
            });

            const xboxAuthResponse = await this.authenticateWithXboxLive(tokenResponse.access_token);

            progressCallback({
                step: 'xsts_token',
                message: 'Obteniendo token XSTS...',
                percentage: 50
            });

            // Paso 4: Obtener token XSTS
            const xstsResponse = await this.getXSTSToken(xboxAuthResponse.Token);

            // Paso 5: Autenticar con Minecraft
            progressCallback({
                step: 'minecraft_auth',
                message: 'Autenticando con Minecraft...',
                percentage: 70
            });

            const minecraftToken = await this.authenticateWithMinecraft(
                xstsResponse.Token,
                xstsResponse.DisplayClaims.xui[0].uhs
            );

            // Paso 6: Obtener perfil de Minecraft
            progressCallback({
                step: 'profile',
                message: 'Obteniendo perfil de Minecraft...',
                percentage: 90
            });

            const profile = await this.getMinecraftProfile(minecraftToken.access_token);

            progressCallback({
                step: 'complete',
                message: 'Autenticación completada con éxito',
                percentage: 100
            });

            // Crear y retornar la cuenta
            const account: MicrosoftAccount = {
                username: profile.name,
                uuid: profile.id,
                accessToken: minecraftToken.access_token,
                refreshToken: tokenResponse.refresh_token,
                tokenExpiration: Date.now() + (minecraftToken.expires_in * 1000),
                accountType: 'microsoft'
            };

            return account;
        } catch (error) {
            console.error("Error durante el proceso de autenticación de Microsoft:", error);
            throw error;
        }
    }

    /**
     * Obtiene un código de dispositivo para iniciar la autenticación
     */
    private async getDeviceCode(): Promise<DeviceCodeResponse> {
        const response = await fetch(this.MICROSOFT_AUTH_URL, {
            method: "POST",
            headers: {
                "Content-Type": "application/x-www-form-urlencoded"
            },
            body: new URLSearchParams({
                client_id: this.clientId,
                scope: "XboxLive.signin offline_access"
            }).toString()
        });

        if (!response.ok) {
            throw new Error(`Error al obtener código de dispositivo: ${response.status}`);
        }

        const data = await response.json();
        if (!data || !data.user_code || !data.device_code) {

            throw new Error("Código de dispositivo no válido recibido.");
        }

        return data as DeviceCodeResponse;
    }

    /**
     * Espera a que el usuario se autentique con el código proporcionado
     */
    private async pollForTokens(
        deviceCode: string,
        interval: number,
        progressCallback: (progress: AuthProgress) => void
    ): Promise<TokenResponse> {
        return new Promise((resolve, reject) => {
            let elapsedTime = 0;
            const maxWaitTime = 300000; // 5 minutos máximo de espera

            const checkInterval = setInterval(async () => {
                try {
                    elapsedTime += interval * 1000;

                    const response = await fetch(this.MICROSOFT_TOKEN_URL, {
                        method: "POST",
                        headers: {
                            "Content-Type": "application/x-www-form-urlencoded"
                        },
                        body: new URLSearchParams({
                            grant_type: "urn:ietf:params:oauth:grant-type:device_code",
                            device_code: deviceCode,
                            client_id: this.clientId
                        }).toString()
                    });

                    if (response.ok) {
                        clearInterval(checkInterval);
                        const tokenResponse = await response.json();
                        resolve(tokenResponse as TokenResponse);
                    } else {
                        const error = await response.json() as { error: string };

                        // authorization_pending significa que el usuario aún no ha completado la autenticación
                        if (error.error !== "authorization_pending") {
                            clearInterval(checkInterval);
                            reject(new Error(`Error en la autenticación: ${error.error}`));
                        }

                        // Actualizar progreso
                        const percentage = Math.min(25, (elapsedTime / maxWaitTime) * 25);
                        progressCallback({
                            step: 'waiting_auth',
                            message: 'Esperando autenticación del usuario...',
                            percentage: 10 + percentage
                        });
                    }

                    // Si ha pasado demasiado tiempo, cancelar
                    if (elapsedTime >= maxWaitTime) {
                        clearInterval(checkInterval);
                        reject(new Error("Tiempo de espera agotado. Por favor, intenta nuevamente."));
                    }
                } catch (error) {
                    clearInterval(checkInterval);
                    reject(error);
                }
            }, interval * 1000);
        });
    }

    /**
     * Autentica con Xbox Live usando el token de Microsoft
     */
    private async authenticateWithXboxLive(accessToken: string): Promise<XboxAuthResponse> {
        const response = await fetch(this.XBOX_AUTH_URL, {
            method: "POST",
            headers: {
                "Content-Type": "application/json",
                "Accept": "application/json"
            },
            body: JSON.stringify({
                Properties: {
                    AuthMethod: "RPS",
                    SiteName: "user.auth.xboxlive.com",
                    RpsTicket: `d=${accessToken}`
                },
                RelyingParty: "http://auth.xboxlive.com",
                TokenType: "JWT"
            })
        });

        if (!response.ok) {
            throw new Error(`Error al autenticar con Xbox Live: ${response.status}`);
        }

        const data = await response.json();
        if (!data || !data.Token || !data.DisplayClaims) {
            throw new Error("Respuesta de Xbox Live no válida.");
        }
        return data as XboxAuthResponse;
    }

    /**
     * Obtiene un token XSTS necesario para autenticar con Minecraft
     */
    private async getXSTSToken(xboxToken: string): Promise<XSTSResponse> {
        const response = await fetch(this.XSTS_AUTH_URL, {
            method: "POST",
            headers: {
                "Content-Type": "application/json",
                "Accept": "application/json"
            },
            body: JSON.stringify({
                Properties: {
                    SandboxId: "RETAIL",
                    UserTokens: [xboxToken]
                },
                RelyingParty: "rp://api.minecraftservices.com/",
                TokenType: "JWT"
            })
        });

        if (!response.ok) {
            const data = await response.json();

            // Código 2148916233 significa que la cuenta no tiene Xbox Game Pass
            if (data.XErr === 2148916233) {
                throw new Error("Esta cuenta de Microsoft no tiene una cuenta de Xbox. Por favor, crea una cuenta de Xbox antes de continuar.");
            }
            // Código 2148916238 significa que la cuenta es de un menor y requiere consentimiento parental
            else if (data.XErr === 2148916238) {
                throw new Error("Esta cuenta es de un menor de edad y requiere consentimiento parental para juegos online.");
            }
            throw new Error(`Error al obtener token XSTS: ${response.status}`);
        }

        const data = await response.json();
        if (!data || !data.Token || !data.DisplayClaims) {
            throw new Error("Respuesta de XSTS no válida.");
        }
        return data as XSTSResponse;
    }

    /**
     * Autentica con el servicio de Minecraft usando los tokens de Xbox
     */
    private async authenticateWithMinecraft(xstsToken: string, userHash: string): Promise<MinecraftAuthResponse> {
        const response = await fetch(this.MINECRAFT_AUTH_URL, {
            method: "POST",
            headers: {
                "Content-Type": "application/json"
            },
            body: JSON.stringify({
                identityToken: `XBL3.0 x=${userHash};${xstsToken}`
            })
        });

        if (!response.ok) {
            throw new Error(`Error al autenticar con Minecraft: ${response.status}`);
        }

        const data = await response.json();
        if (!data || !data.access_token) {
            throw new Error("Respuesta de Minecraft no válida.");
        }
        return data as MinecraftAuthResponse;
    }

    /**
     * Obtiene el perfil de Minecraft del usuario autenticado
     */
    private async getMinecraftProfile(accessToken: string): Promise<MinecraftProfileResponse> {
        const response = await fetch(this.MINECRAFT_PROFILE_URL, {
            method: "GET",
            headers: {
                "Authorization": `Bearer ${accessToken}`
            }
        });

        if (!response.ok) {
            // Si el código es 404, significa que el usuario no tiene Minecraft comprado
            if (response.status === 404) {
                throw new Error("Esta cuenta de Microsoft no ha comprado Minecraft. Por favor, compra el juego para continuar.");
            }
            throw new Error(`Error al obtener perfil de Minecraft: ${response.status}`);
        }

        const data = await response.json();
        if (!data || !data.id || !data.name) {
            throw new Error("Respuesta de perfil de Minecraft no válida.");
        }
        return data as MinecraftProfileResponse;
    }

    /**
     * Refresca un token expirado usando el refresh token
     */
    public async refreshAccessToken(refreshToken: string): Promise<TokenResponse> {
        const response = await fetch(this.MICROSOFT_TOKEN_URL, {
            method: "POST",
            headers: {
                "Content-Type": "application/x-www-form-urlencoded"
            },
            body: new URLSearchParams({
                client_id: this.clientId,
                refresh_token: refreshToken,
                grant_type: "refresh_token"
            }).toString()
        });

        if (!response.ok) {
            throw new Error(`Error al refrescar token: ${response.status}`);
        }

        const data = await response.json();
        if (!data || !data.access_token) {
            throw new Error("Respuesta de refresco de token no válida.");
        }
        return data as TokenResponse;
    }
}

// Exportar una instancia por defecto
const microsoftAuthService = new MicrosoftAuthService();
export default microsoftAuthService;