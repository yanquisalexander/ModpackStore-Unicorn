import { useGlobalContext } from "@/stores/GlobalContext"
import { PreLaunchAppearance } from "@/types/PreLaunchAppeareance"
import { getDefaultAppeareance } from "@/utils/prelaunch"
import { invoke } from "@tauri-apps/api/core"
import { LucideFolderOpen, LucideGamepad2, LucideLoaderCircle, LucideSettings, LucideShieldCheck } from "lucide-react"
import { CSSProperties, useEffect, useState, useCallback, useRef } from "react"
import { toast } from "sonner"
import { navigate } from "wouter/use-browser-location"
import { useInstances } from "@/stores/InstancesContext"
import { TauriCommandReturns } from "@/types/TauriCommandReturns"
import { Activity, Assets, Timestamps } from "tauri-plugin-drpc/activity"
import { setActivity } from "tauri-plugin-drpc"
import { EditInstanceInfo } from "@/components/EditInstanceInfo"

// Constantes
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
    // Context y state
    const { setTitleBarState } = useGlobalContext();
    const { instances } = useInstances();
    const currentInstanceRunning = instances.find(inst => inst.id === instanceId) || null;
    const isPlaying = currentInstanceRunning?.status === "running";

    // Refs
    const audioRef = useRef<HTMLAudioElement | null>(null);
    const messageIntervalRef = useRef<number | null>(null);
    const quickActionsRef = useRef<HTMLDivElement | null>(null);

    // State
    const [appearance, setAppearance] = useState<PreLaunchAppearance | null>(null);
    const [quickActionsOpen, setQuickActionsOpen] = useState(false);
    const [prelaunchState, setPrelaunchState] = useState({
        isLoading: true,
        error: null as string | null,
        instance: null as any | null,
    });
    const [loadingStatus, setLoadingStatus] = useState(DEFAULT_LOADING_STATE);

    // Memoized functions
    const getRandomMessage = useCallback(() => {
        return RANDOM_MESSAGES[Math.floor(Math.random() * RANDOM_MESSAGES.length)];
    }, []);

    const notAvailable = useCallback(() => {
        setQuickActionsOpen(false);
        toast.error("Función no disponible aún", {
            description: "Esta función estará disponible en futuras versiones.",
        });
    }, []);

    const toggleQuickActions = useCallback(() => {
        setQuickActionsOpen(prev => !prev);
    }, []);

    const openGameDir = useCallback(async () => {
        try {
            await invoke("open_game_dir", { instanceId });
            toast.success("Abriendo carpeta de la instancia...");
            setQuickActionsOpen(false);
        } catch (error) {
            console.error("Error opening game directory:", error);
            toast.error("Error al abrir la carpeta de la instancia", {
                description: "No se pudo abrir la carpeta de la instancia. Intenta nuevamente.",
                dismissible: true,
            });
        }
    }, [instanceId]);

    const reloadInfo = useCallback(() => {
        setPrelaunchState(prev => ({
            ...prev,
            isLoading: true,
            error: null,
        }));
    }
        , []);

    const handlePlayButtonClick = useCallback(async () => {
        if (loadingStatus.isLoading || isPlaying) return;

        try {
            await invoke("launch_mc_instance", { instanceId });

            // Limpiar intervalo existente
            if (messageIntervalRef.current) {
                window.clearInterval(messageIntervalRef.current);
            }

            // Crear nuevo intervalo para mensajes aleatorios
            messageIntervalRef.current = window.setInterval(() => {
                // Verificar si debemos detener el intervalo
                if (currentInstanceRunning?.status !== "running" && currentInstanceRunning?.status !== "preparing") {
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
    }, [instanceId, currentInstanceRunning?.status, loadingStatus.isLoading, isPlaying, getRandomMessage]);

    // Effect hooks


    // 2. Click outside handler for quick actions menu
    useEffect(() => {
        const handleClickOutside = (event: MouseEvent) => {
            if (quickActionsRef.current && !quickActionsRef.current.contains(event.target as Node)) {
                setQuickActionsOpen(false);
            }
        };

        document.addEventListener("mousedown", handleClickOutside);
        return () => document.removeEventListener("mousedown", handleClickOutside);
    }, []);

    // 3. Initial instance loading
    useEffect(() => {
        const getMinecraftInstance = async () => {
            setTitleBarState(prev => ({ ...prev, canGoBack: true }));

            try {
                const instance = await invoke<TauriCommandReturns['get_instance_by_id']>("get_instance_by_id", { instanceId });

                if (!instance) throw new Error("Instance not found");

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

    // 4. Load appearance
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

    // 5. Update Discord RPC
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
            .catch(error => console.error("Error setting Discord activity:", error))
            .then(() => console.log("Discord activity set successfully"));
    }, [isPlaying, prelaunchState.instance]);

    // 6. Audio handling
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
            audio.play().catch(error => console.error("Error playing audio:", error));
        }

        return () => {
            audio.pause();
            audio.currentTime = 0;
        };
    }, [appearance?.audio?.url, isPlaying]);

    // 7. Update loading status based on current instance
    useEffect(() => {
        if (!currentInstanceRunning) return;

        const isLoading = currentInstanceRunning.status === "preparing" || currentInstanceRunning.status === "downloading-assets";

        setLoadingStatus(prev => ({
            ...prev,
            isLoading,
            message: currentInstanceRunning.message || getRandomMessage(),
        }));

        // Mostrar toast de error si la instancia tiene un error
        if (currentInstanceRunning.status === "error") {
            toast.error("Error en la instancia", {
                id: "instance-runtime-error",
                description: currentInstanceRunning.message || "Ha ocurrido un error al ejecutar la instancia.",
                dismissible: false,
            });
        }
    }, [currentInstanceRunning, getRandomMessage]);

    // 8. Cleanup intervals
    useEffect(() => {
        return () => {
            if (messageIntervalRef.current) {
                window.clearInterval(messageIntervalRef.current);
                messageIntervalRef.current = null;
            }
        };
    }, []);

    // Loading and error handling
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
                onClick: () => navigate("/")
            },
        });

        return (
            <div className="flex items-center justify-center min-h-screen h-full w-full">
                <div className="text-white text-lg">{prelaunchState.error}</div>
            </div>
        );
    }

    // Helper calculations for layout positions
    const hasCustomPosition = appearance?.playButton?.position &&
        Object.values(appearance.playButton.position).some(value => value != null);

    const logoHasCustomPosition = appearance?.logo?.position &&
        Object.values(appearance.logo.position).some(value => value != null);

    return (
        <div className="absolute inset-0">
            <div className="relative h-full w-full overflow-hidden">
                {/* Loading status indicator */}
                {loadingStatus.isLoading && (
                    <div className="flex gap-x-2 absolute animate-fade-in-down animate-duration-400 ease-in-out z-20 top-12 right-4 bg-black/80 px-2 py-1 max-w-xs w-full text-white items-center">
                        <LucideLoaderCircle className="animate-spin-clockwise animate-iteration-count-infinite animate-duration-[2500ms] text-white flex-shrink-0" />
                        {loadingStatus.message}
                    </div>
                )}

                {/* Background image */}
                <img
                    style={{
                        maskImage: "linear-gradient(to bottom, white 60% , rgba(0, 0, 0, 0) 100%)",
                    }}
                    className="absolute opacity-80 inset-0 z-1 h-full w-full object-cover animate-fade-in ease-in-out duration-1000"
                    src={appearance?.background?.imageUrl ?? ""}
                    alt="Background"
                />

                {/* Logo image */}
                {appearance?.logo?.url && (
                    <img
                        src={appearance.logo.url}
                        alt="Logo"
                        style={{
                            top: appearance.logo.position?.top,
                            left: appearance.logo.position?.left,
                            transform: appearance.logo.position?.transform,
                            animationDelay: appearance.logo.fadeInDelay,
                            animationDuration: appearance.logo.fadeInDuration,
                            height: appearance.logo.height,
                        }}
                        className={`absolute z-10 animate-fade-in duration-500 ease-in-out ${logoHasCustomPosition ? "fixed" : ""}`}
                    />
                )}

                {/* Footer with play button */}
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

                {/* Quick actions menu */}
                {prelaunchState.instance && (
                    <div className="absolute right-0 bottom-40 z-40 group" ref={quickActionsRef}>
                        <div className="flex items-center justify-end relative w-fit">
                            {/* Settings button */}
                            <button
                                onClick={toggleQuickActions}
                                className="size-12 cursor-pointer group hover:bg-neutral-900 transition bg-neutral-800 rounded-l-md flex items-center justify-center">
                                <LucideSettings
                                    style={{
                                        transform: quickActionsOpen ? "rotate(90deg)" : "rotate(0deg)",
                                        transition: "transform 0.3s ease-in-out",
                                    }}
                                    className="size-5 text-white"
                                />
                            </button>

                            {/* Actions menu */}
                            <div className={`absolute right-full bottom-0 mr-2 ${quickActionsOpen
                                ? "opacity-100 pointer-events-auto translate-x-0"
                                : "opacity-0 pointer-events-none translate-x-2"
                                } transition-all duration-300`}
                            >
                                <div className="bg-neutral-900 border border-neutral-700 rounded-md shadow-md p-2 space-y-2 max-w-xs w-64">
                                    <button
                                        onClick={openGameDir}
                                        className="cursor-pointer flex items-center gap-x-2 text-white w-full hover:bg-neutral-800 px-3 py-2 rounded-md transition"
                                    >
                                        <LucideFolderOpen className="size-4 text-white" />
                                        Abrir .minecraft
                                    </button>
                                    <EditInstanceInfo
                                        instanceId={instanceId}
                                        onUpdate={reloadInfo}
                                    />
                                    <button
                                        onClick={notAvailable}
                                        className="cursor-pointer flex items-center gap-x-2 text-white w-full hover:bg-neutral-800 px-3 py-2 rounded-md transition"
                                    >
                                        <LucideLoaderCircle className="size-4 text-white" />
                                        Descargar mods
                                    </button>
                                    <button
                                        onClick={notAvailable}
                                        className="cursor-pointer flex items-center gap-x-2 text-white w-full hover:bg-neutral-800 px-3 py-2 rounded-md transition"
                                    >
                                        <LucideShieldCheck className="size-4 text-white" />
                                        Verificar integridad
                                    </button>
                                </div>
                            </div>
                        </div>
                    </div>
                )}
            </div>
        </div>
    );
};