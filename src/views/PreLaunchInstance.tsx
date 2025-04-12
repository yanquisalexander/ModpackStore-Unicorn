import { useGlobalContext } from "@/stores/GlobalContext"
import { PreLaunchAppearance } from "@/types/PreLaunchAppeareance"
import { getDefaultAppeareance } from "@/utils/prelaunch"
import { invoke } from "@tauri-apps/api/core"
import { LucideGamepad2, LucideLoaderCircle } from "lucide-react"
import { CSSProperties, useEffect, useState, useCallback, useRef } from "react"
import { toast } from "sonner"
import { navigate } from "wouter/use-browser-location"
import { useInstances } from "@/stores/InstancesContext"
import { TauriCommandReturns } from "@/types/TauriCommandReturns"
import { Activity, Assets, Timestamps } from "tauri-plugin-drpc/activity"
import { setActivity } from "tauri-plugin-drpc"

// Constantes movidas fuera del componente para evitar recreaciones
const DEFAULT_LOADING_STATE = {
    isLoading: false,
    message: "Descargando archivos necesarios...",
    progress: 0,
    logs: []
};

const RANDOM_MESSAGES = [
    "Descargando archivos necesarios...",
    "Cargando modpack...",
    "Muy pronto estarás jugando...",
    "Seguro que te va a encantar...",
    "Preparando todo para ti...",
];

export const PreLaunchInstance = ({ instanceId }: { instanceId: string }) => {
    const { titleBarState, setTitleBarState } = useGlobalContext()
    const { instances } = useInstances()

    // Obtenemos la instancia específica del contexto
    const currentInstance = instances.find(inst => inst.id === instanceId) || null
    const isPlaying = currentInstance?.status === "running"

    // Refs para evitar dependencias circulares en efectos
    const audioRef = useRef<HTMLAudioElement | null>(null);
    const messageIntervalRef = useRef<number | null>(null);

    const [appearance, setAppearance] = useState<PreLaunchAppearance | null>(null)

    const [prelaunchState, setPrelaunchState] = useState({
        isLoading: true,
        error: null as string | null,
        instance: null as any | null,
    });

    const [loadingStatus, setLoadingStatus] = useState(DEFAULT_LOADING_STATE);

    // Función memoizada para obtener mensajes aleatorios
    const getRandomMessage = useCallback(() => {
        return RANDOM_MESSAGES[Math.floor(Math.random() * RANDOM_MESSAGES.length)];
    }, []);

    // Carga inicial de la instancia - ejecutada solo una vez
    useEffect(() => {
        const getMinecraftInstance = async () => {
            setTitleBarState(prev => ({
                ...prev,
                canGoBack: true,
            }));

            try {
                const instance = await invoke<TauriCommandReturns['get_instance_by_id']>("get_instance_by_id", { instanceId });

                if (!instance) {
                    throw new Error("Instance not found");
                }

                setPrelaunchState({
                    isLoading: false,
                    error: null,
                    instance,
                });

                setTitleBarState(prev => ({
                    ...prev,
                    title: instance.instanceName,
                    icon: "https://saltouruguayserver.com/favicon.svg",
                    canGoBack: true,
                    customIconClassName: "",
                    opaque: false,
                }));
            } catch (error) {
                console.error("Error fetching instance data:", error);
                setPrelaunchState({
                    isLoading: false,
                    error: "Ocurrió un error al cargar la instancia",
                    instance: null,
                });
            }
        };

        getMinecraftInstance();
    }, [instanceId, setTitleBarState]);

    // Carga de la apariencia - ejecutada solo una vez
    useEffect(() => {
        invoke("get_prelaunch_appearance", { instanceId })
            .then((res) => {
                setAppearance(res as PreLaunchAppearance);
            })
            .catch((err) => {
                console.error("Error fetching appearance:", err);
                setAppearance(getDefaultAppeareance({
                    title: "SaltoCraft 3",
                    description: "Un modpack de SaltoUruguayServer",
                    logoUrl: "https://saltouruguayserver.com/favicon.svg",
                }));
            });
    }, [instanceId]);

    // Actualiza Discord RPC cuando cambia el estado de juego
    useEffect(() => {
        if (!prelaunchState.instance) return;

        const stateText = isPlaying
            ? `Jugando "${prelaunchState.instance.instanceName}"`
            : `Preparando "${prelaunchState.instance.instanceName}"`;

        const activity = new Activity()
            .setState(isPlaying ? "Jugando" : "Preparando")
            .setDetails(stateText)
            .setTimestamps(new Timestamps(Date.now()))
            .setAssets(new Assets().setLargeImage("playing").setSmallImage("playing"));

        setActivity(activity)
            .catch((error) => {
                console.error("Error setting Discord activity:", error);
            })
            .then(() => {
                console.log("Discord activity set successfully");
            });

    }, [isPlaying, prelaunchState.instance]);

    // Manejo del audio
    useEffect(() => {
        if (!appearance?.audio?.url) return;

        // Inicializar el audio si aún no existe
        if (!audioRef.current) {
            audioRef.current = new Audio(appearance.audio.url);
            audioRef.current.loop = true;
            audioRef.current.volume = 0.01;
        }

        const audio = audioRef.current;

        // Controlar la reproducción según el estado del juego
        if (isPlaying) {
            audio.pause();
            audio.currentTime = 0;
        } else {
            audio.play().catch(error => {
                console.error("Error playing audio:", error);
            });
        }

        // Limpieza al desmontar
        return () => {
            audio.pause();
            audio.currentTime = 0;
        };
    }, [appearance?.audio?.url, isPlaying]);

    // Actualiza el estado de carga basado en la instancia actual
    useEffect(() => {
        if (!currentInstance) return;

        const isLoading = currentInstance.status === "preparing" || currentInstance.status === "downloading-assets";

        setLoadingStatus(prev => ({
            ...prev,
            isLoading,
            message: currentInstance.message || getRandomMessage(),
        }));

        // Mostrar toast de error si la instancia tiene un error
        if (currentInstance.status === "error") {
            toast.error("Error en la instancia", {
                id: "instance-runtime-error",
                description: currentInstance.message || "Ha ocurrido un error al ejecutar la instancia.",
                dismissible: false,
            });
        }
    }, [currentInstance, getRandomMessage]);

    const handlePlayButtonClick = useCallback(async () => {
        console.log("Launching Minecraft instance...");

        try {
            await invoke("launch_mc_instance", {
                instanceId,
            });

            // Crear intervalo para mensajes aleatorios durante la carga
            if (messageIntervalRef.current) {
                window.clearInterval(messageIntervalRef.current);
            }

            messageIntervalRef.current = window.setInterval(() => {
                if (currentInstance?.status !== "running" && currentInstance?.status !== "preparing") {
                    if (messageIntervalRef.current) {
                        window.clearInterval(messageIntervalRef.current);
                        messageIntervalRef.current = null;
                    }
                    return;
                }

                setLoadingStatus(prev => ({
                    ...prev,
                    message: getRandomMessage(),
                }));
            }, 5000) as unknown as number;
        } catch (error) {
            console.error("Error launching instance:", error);
            toast.error("Error al iniciar la instancia", {
                description: "No se pudo iniciar la instancia. Intenta nuevamente más tarde.",
                dismissible: true,
            });
        }
    }, [instanceId, currentInstance?.status, getRandomMessage]);

    // Limpieza de intervalos al desmontar
    useEffect(() => {
        return () => {
            if (messageIntervalRef.current) {
                window.clearInterval(messageIntervalRef.current);
                messageIntervalRef.current = null;
            }
        };
    }, []);

    // Renderizado condicional para estados de carga y error
    if (prelaunchState.isLoading) {
        return (
            <div className="flex items-center justify-center min-h-screen h-full w-full">
                <LucideLoaderCircle className="size-10 -mt-12 animate-spin-clockwise animate-iteration-count-infinite animate-duration-1000 text-white" />
            </div>
        );
    }

    if (prelaunchState.error) {
        toast.error(prelaunchState.error, {
            id: "instance-error",
            description: "No se pudo cargar la instancia. Intenta nuevamente más tarde.",
            dismissible: false,
            action: {
                label: "Volver a inicio",
                onClick: () => {
                    navigate("/");
                },
            },
        });

        return (
            <div className="flex items-center justify-center min-h-screen h-full w-full">
                <div className="text-white text-lg">{prelaunchState.error}</div>
            </div>
        );
    }

    // Verifica si hay posiciones personalizadas
    const hasCustomPosition = appearance?.playButton?.position &&
        Object.values(appearance.playButton.position).some(value => value != null);

    const logoHasCustomPosition = appearance?.logo?.position &&
        Object.values(appearance.logo.position).some(value => value != null);

    return (
        <div className="absolute inset-0">
            <div className="relative h-full w-full overflow-hidden">
                {loadingStatus.isLoading && (
                    <div className="flex gap-x-2 absolute animate-fade-in-down animate-duration-400 ease-in-out z-20 top-12 right-4 bg-black/80 px-2 py-1 max-w-xs w-full text-white items-center">
                        <LucideLoaderCircle className="animate-spin-clockwise animate-iteration-count-infinite animate-duration-[2500ms] text-white flex-shrink-0" />
                        {loadingStatus.message}
                    </div>
                )}

                <img
                    style={{
                        maskImage: "linear-gradient(to bottom, white 60% , rgba(0, 0, 0, 0) 100%)",
                    }}
                    className="absolute opacity-80 inset-0 z-1 h-full w-full object-cover animate-fade-in ease-in-out duration-1000"
                    src={appearance?.background?.imageUrl ?? ""}
                    alt="Background"
                />

                {/* Logo del modpack */}
                <img
                    src={appearance?.logo?.url}
                    alt="Logo"
                    style={{
                        top: appearance?.logo?.position?.top,
                        left: appearance?.logo?.position?.left,
                        transform: appearance?.logo?.position?.transform,
                        animationDelay: appearance?.logo?.fadeInDelay,
                        animationDuration: appearance?.logo?.fadeInDuration,
                        height: appearance?.logo?.height,
                    }}
                    className={`absolute z-10 animate-fade-in duration-500 ease-in-out ${logoHasCustomPosition ? "fixed" : ""}`}
                />

                <footer className="absolute bottom-0 left-0 right-0 z-10 bg-black/50 p-4 text-white flex items-center justify-center">
                    <div className="flex flex-col items-center justify-center space-y-4">
                        <button
                            style={{
                                "--bg-color": appearance?.playButton?.backgroundColor,
                                "--hover-color": appearance?.playButton?.hoverColor,
                                "--text-color": appearance?.playButton?.textColor,
                                "--border-color": appearance?.playButton?.borderColor,
                                "--font-family": appearance?.playButton?.fontFamily,
                                top: appearance?.playButton?.position?.top,
                                left: appearance?.playButton?.position?.left,
                                right: appearance?.playButton?.position?.right,
                                bottom: appearance?.playButton?.position?.bottom,
                                transform: appearance?.playButton?.position?.transform,
                            } as CSSProperties}
                            id="play-button"
                            onClick={handlePlayButtonClick}
                            disabled={loadingStatus.isLoading || isPlaying}
                            className={`
                            ${hasCustomPosition ? "fixed" : ""}
                            cursor-pointer
                            active:scale-95 transition
                            px-4 py-2
                            border-3
                            items-center flex gap-x-2 font-semibold
                            disabled:bg-neutral-800 disabled:cursor-not-allowed
                            font-[var(--font-family)]
                            bg-[var(--bg-color)]
                            hover:bg-[var(--hover-color)]
                            active:bg-[var(--hover-color)]
                            text-[var(--text-color)]
                            border-[var(--border-color)]
                          `}
                        >
                            <LucideGamepad2 className="size-6 text-[var(--text-color)]" />
                            {isPlaying
                                ? "Ya estás jugando"
                                : appearance?.playButton?.text ?? "Jugar ahora"}
                        </button>

                        <div className="flex items-center justify-center space-x-2">
                            <img src="https://saltouruguayserver.com/favicon.svg" className="h-8 w-8" alt="Logo" />
                            <span className="text-sm">
                                Un modpack de SaltoUruguayServer
                            </span>
                        </div>
                    </div>
                </footer>
            </div>
        </div>
    );
};