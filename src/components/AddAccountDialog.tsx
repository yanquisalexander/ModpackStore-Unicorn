import { useState, useEffect } from "react"
import { invoke } from "@tauri-apps/api/core"
import { listen } from "@tauri-apps/api/event"
import { LucideUser, Loader2 } from "lucide-react"
import { TauriCommandReturns } from "@/types/TauriCommandReturns"

import {
    Dialog,
    DialogContent,
    DialogDescription,
    DialogFooter,
    DialogHeader,
    DialogTitle,
    DialogTrigger,
} from "@/components/ui/dialog"
import {
    Tabs,
    TabsContent,
    TabsList,
    TabsTrigger,
} from "@/components/ui/tabs"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { toast } from "sonner"
import { MicrosoftIcon } from "@/icons/MicrosoftIcon"
import { Progress } from "@/components/ui/progress"
import { Alert, AlertDescription } from "@/components/ui/alert"
import { CheckCircle } from "lucide-react"

// Tipos para eventos de autenticación
interface AuthProgressEvent {
    step: 'device_code' | 'waiting_auth' | 'microsoft_token' | 'xbox_auth' | 'xsts_token' | 'minecraft_auth' | 'profile' | 'complete';
    message: string;
    percentage: number;
    user_code?: string;
    verification_url?: string;
}

interface MicrosoftAccount {
    username: string;
    uuid: string;
    access_token: string;
    refresh_token: string;
    token_expiration: number;
    account_type: string;
}

export const AddAccountDialog = ({
    onAccountAdded
}: {
    onAccountAdded: () => void
}) => {
    const [open, setOpen] = useState(false)
    const [username, setUsername] = useState("")
    const [isLoading, setIsLoading] = useState(false)
    const [microsoftLoading, setMicrosoftLoading] = useState(false)
    const [authProgress, setAuthProgress] = useState<AuthProgressEvent | null>(null)
    const [authCode, setAuthCode] = useState<string | null>(null)
    const [verificationUrl, setVerificationUrl] = useState<string | null>(null)

    console.log('authProgress', authProgress)

    // Configurar escuchadores de eventos para la autenticación con Microsoft
    useEffect(() => {
        // Escuchar eventos de progreso de autenticación
        const progressUnlisten = listen<AuthProgressEvent>("microsoft-auth-progress", (event) => {
            setAuthProgress(event.payload);
            if (event.payload.step === 'waiting_auth' && event.payload.user_code) {
                setAuthCode(event.payload.user_code || null);
                setVerificationUrl(event.payload.verification_url || null);
            }
        });

        // Escuchar eventos de éxito de autenticación
        const successUnlisten = listen<MicrosoftAccount>("microsoft-auth-success", async (event) => {
            const account = event.payload;

            try {
                // Guardar la cuenta usando un comando de Tauri
                await invoke("save_microsoft_account", { account });

                toast.success("Cuenta Microsoft añadida", {
                    description: `Se ha añadido la cuenta ${account.username} correctamente`,
                });

                setMicrosoftLoading(false);
                setAuthProgress(null);
                setOpen(false);
                onAccountAdded();
            } catch (error) {
                console.error("Error al guardar la cuenta Microsoft:", error);
                toast.error("No se pudo guardar la cuenta. Inténtalo de nuevo.");
                setMicrosoftLoading(false);
                setAuthProgress(null);
            }
        });

        // Escuchar eventos de error de autenticación
        const errorUnlisten = listen<string>("microsoft-auth-error", (event) => {
            const errorMessage = event.payload;
            toast.error("Error de autenticación", {
                description: errorMessage
            });
            setMicrosoftLoading(false);
            setAuthProgress(null);
        });

        // Limpieza de escuchadores al desmontar
        return () => {
            progressUnlisten.then(unlisten => unlisten());
            successUnlisten.then(unlisten => unlisten());
            errorUnlisten.then(unlisten => unlisten());
        };
    }, [onAccountAdded]);

    const handleAddOfflineAccount = async () => {
        if (!username.trim()) {
            toast.error("Error", {
                description: "El nombre de usuario no puede estar vacío"
            })
            return
        }

        setIsLoading(true)

        try {
            await invoke<TauriCommandReturns['add_offline_account']>('add_offline_account', { username: username.trim() })

            toast("Cuenta añadida", {
                description: `Se ha añadido la cuenta ${username} correctamente`,
            })

            setUsername("")
            setOpen(false)
            onAccountAdded()
        } catch (error) {
            console.error("Error al añadir cuenta offline:", error)
            toast.error("No se pudo añadir la cuenta. Inténtalo de nuevo.")
        } finally {
            setIsLoading(false)
        }
    }

    const handleMicrosoftLogin = async () => {
        setMicrosoftLoading(true);
        setAuthProgress(null);

        try {
            // Invocar el comando de Rust para iniciar la autenticación
            await invoke("start_microsoft_auth");
            // El proceso continuará en los escuchadores de eventos
        } catch (error) {
            console.error("Error al iniciar la autenticación con Microsoft:", error);
            toast.error("No se pudo iniciar la autenticación. Inténtalo de nuevo.");
            setMicrosoftLoading(false);
        }
    }

    const renderMicrosoftAuthProgress = () => {
        if (!authProgress) return null;

        // Mostrar siempre el código de autorización si está disponible
        // independientemente del paso actual
        const showAuthCode = authCode && authProgress.step !== 'complete';

        return (
            <div className="space-y-4 mt-4">
                {/* Sección del código de autorización - siempre visible si existe */}
                {showAuthCode && (
                    <div className="bg-gray-800 rounded-md p-4">
                        <h4 className="text-sm font-medium mb-2">Ingresa este código en Microsoft:</h4>
                        <div className="bg-gray-700 rounded p-3 flex items-center justify-center">
                            <span className="text-xl font-mono tracking-widest text-white">
                                {authCode}
                            </span>
                        </div>
                        <p className="text-sm text-gray-400 mt-3 mb-2">
                            Ve a la siguiente dirección y sigue las instrucciones:
                        </p>

                        <a
                            href={verificationUrl!}
                            target="_blank"
                            rel="noopener noreferrer"
                            className="block w-full bg-blue-600 hover:bg-blue-700 text-center py-2 rounded text-white"
                        >
                            Abrir página de verificación
                        </a>
                    </div>
                )}

                {/* Barra de progreso - siempre visible */}
                <div className="space-y-2">
                    <div className="flex justify-between mb-1">
                        <p className="text-sm text-gray-300">{authProgress.message}</p>
                        <span className="text-sm text-gray-400">{authProgress.percentage}%</span>
                    </div>
                    <Progress value={authProgress.percentage} />

                    {authProgress.step === 'complete' && (
                        <Alert className="bg-green-900/20 border-green-700 mt-3">
                            <CheckCircle className="h-4 w-4 text-green-500" />
                            <AlertDescription className="text-green-300">
                                Autenticación completada con éxito
                            </AlertDescription>
                        </Alert>
                    )}
                </div>
            </div>
        );
    }

    return (
        <Dialog open={open} onOpenChange={setOpen}>
            <DialogTrigger asChild>
                <button
                    className="cursor-pointer z-10 group relative overflow-hidden rounded-xl border border-dashed border-white/20 h-64 flex flex-col items-center justify-center
                    transition duration-300 hover:border-sky-400/50 hover:bg-gray-800/30"
                >
                    <div className="flex flex-col items-center gap-3">
                        <div className="p-3 rounded-full bg-gray-800/80 group-hover:bg-sky-900/40 transition">
                            <LucideUser className="h-8 w-8 text-gray-400 group-hover:text-sky-300" />
                        </div>
                        <span className="text-gray-400 group-hover:text-sky-300 font-medium">Añadir cuenta</span>
                    </div>
                </button>
            </DialogTrigger>

            <DialogContent className="sm:max-w-md dark">
                <DialogHeader>
                    <DialogTitle className="from-[#bcfe47] to-[#05cc2a] bg-clip-text text-transparent bg-gradient-to-b">Añadir una nueva cuenta</DialogTitle>
                    <DialogDescription>
                        Elige el tipo de cuenta que deseas añadir para jugar en los modpacks.
                    </DialogDescription>
                </DialogHeader>

                <Tabs defaultValue="offline" className="mt-4">
                    <TabsList className="grid w-full grid-cols-2">
                        <TabsTrigger value="offline">Cuenta Offline</TabsTrigger>
                        <TabsTrigger value="microsoft">Cuenta Microsoft</TabsTrigger>
                    </TabsList>

                    <TabsContent value="offline" className="mt-4 space-y-4">
                        <div className="space-y-2">
                            <Label className="text-white" htmlFor="username">Nombre de usuario</Label>
                            <Input
                                id="username"
                                value={username}
                                onChange={(e) => {
                                    // Prevent spacing and special characters
                                    const value = e.target.value.replace(/[^a-zA-Z0-9_]/g, "")
                                    setUsername(value)
                                }}
                                placeholder="Ingresa tu nombre de usuario"
                            />
                            <p className="text-sm text-gray-400">
                                Las cuentas offline te permiten jugar sin verificación pero con funcionalidades limitadas.
                            </p>
                        </div>

                        <DialogFooter className="mt-6">
                            <Button
                                type="submit"
                                onClick={handleAddOfflineAccount}
                                disabled={isLoading}
                                className="w-full cursor-pointer disabled:bg-gray-700 disabled:text-gray-400 disabled:cursor-not-allowed"
                            >
                                {isLoading ? "Añadiendo..." : "Añadir cuenta offline"}
                            </Button>
                        </DialogFooter>
                    </TabsContent>

                    <TabsContent value="microsoft" className="mt-4">
                        <div className="flex flex-col items-center justify-center py-4 space-y-4">
                            {!microsoftLoading && !authProgress ? (
                                <>
                                    <div className="p-3 rounded-full bg-blue-900/40">
                                        <MicrosoftIcon className="h-8 w-8" />
                                    </div>
                                    <div className="text-center">
                                        <h3 className="text-lg font-medium text-neutral-50">Iniciar sesión con Microsoft</h3>
                                        <p className="text-sm text-gray-400 mt-2 mb-6">
                                            Conecta tu cuenta de Microsoft para acceder a todas las funcionalidades de Minecraft Premium.
                                        </p>
                                    </div>

                                    <Button
                                        onClick={handleMicrosoftLogin}
                                        className="bg-blue-600 hover:bg-blue-700"
                                    >
                                        Iniciar con Microsoft
                                    </Button>
                                </>
                            ) : (
                                <div className="w-full">
                                    <div className="flex items-center space-x-2 mb-4">
                                        <MicrosoftIcon className="h-5 w-5" />
                                        <h3 className="text-lg font-medium text-neutral-50">Autenticación con Microsoft</h3>
                                    </div>

                                    {microsoftLoading && !authProgress ? (
                                        <div className="flex flex-col items-center py-8">
                                            <Loader2 className="h-8 w-8 text-blue-500 animate-spin mb-4" />
                                            <p className="text-gray-300">Iniciando proceso de autenticación...</p>
                                        </div>
                                    ) : renderMicrosoftAuthProgress()}
                                </div>
                            )}
                        </div>
                    </TabsContent>
                </Tabs>
            </DialogContent>
        </Dialog>
    )
}