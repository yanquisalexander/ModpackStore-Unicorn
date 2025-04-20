import { useGlobalContext } from "@/stores/GlobalContext"
import { PreLaunchAppearance } from "@/types/PreLaunchAppeareance"
import { getDefaultAppeareance } from "@/utils/prelaunch"
import { invoke } from "@tauri-apps/api/core"
import { LucideFolderOpen, LucideGamepad2, LucideLoaderCircle, LucideSettings, LucideShieldCheck, LucideUnplug } from "lucide-react"
import { CSSProperties, useEffect, useState, useCallback, useRef } from "react"
import { toast } from "sonner"
import { navigate } from "wouter/use-browser-location"
import { useInstances } from "@/stores/InstancesContext"
import { TauriCommandReturns } from "@/types/TauriCommandReturns"
import { Activity, Assets, Timestamps } from "tauri-plugin-drpc/activity"
import { setActivity } from "tauri-plugin-drpc"
import { EditInstanceInfo } from "@/components/EditInstanceInfo"
import { playSound, SOUNDS } from "@/utils/sounds"
import { trackEvent } from "@aptabase/web"

// Constants
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
    // Context and state
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

    // Helper functions
    const getRandomMessage = useCallback(() => {
        return RANDOM_MESSAGES[Math.floor(Math.random() * RANDOM_MESSAGES.length)];
    }, []);

    // Instance loading functions
    const fetchInstanceData = useCallback(async () => {
        setPrelaunchState(prev => ({
            ...prev,
            isLoading: true,
            error: null,
        }));

        try {
            const instance = await invoke<TauriCommandReturns['get_instance_by_id']>("get_instance_by_id", { instanceId });

            if (!instance) throw new Error("Instance not found");

            setPrelaunchState({
                isLoading: false,
                error: null,
                instance,
            });

            updateTitleBar(instance);
            return instance;
        } catch (error) {
            console.error("Error fetching instance data:", error);
            setPrelaunchState({
                isLoading: false,
                error: "Ocurrió un error al cargar la instancia",
                instance: null,
            });
            return null;
        }
    }, [instanceId]);

    const updateTitleBar = useCallback((instance: any) => {
        setTitleBarState(prev => ({
            ...prev,
            title: instance.instanceName,
            icon: "https://saltouruguayserver.com/favicon.svg",
            canGoBack: true,
            customIconClassName: "",
            opaque: false,
        }));
    }, [setTitleBarState]);

    const loadAppearance = useCallback(async () => {
        try {
            const appearanceData = await invoke("get_prelaunch_appearance", { instanceId }) as PreLaunchAppearance;
            setAppearance(appearanceData);
        } catch (err) {
            console.error("Error fetching appearance:", err);
            setAppearance(getDefaultAppeareance({
                logoUrl: "/images/mc_logo.svg",
            }));
        }
    }, [instanceId, prelaunchState.instance]);

    // Action handlers
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

    const reloadInfo = useCallback(async () => {
        await fetchInstanceData();
    }, [fetchInstanceData]);

    // Launch instance handling
    const startMessageInterval = useCallback(() => {
        // Clear existing interval
        if (messageIntervalRef.current) {
            window.clearInterval(messageIntervalRef.current);
        }

        // Create new interval for random messages
        messageIntervalRef.current = window.setInterval(() => {
            // Check if we should stop the interval
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
    }, [currentInstanceRunning?.status, getRandomMessage]);

    const handlePlayButtonClick = useCallback(async () => {
        // Evitar iniciar si ya está cargando o jugando
        if (loadingStatus.isLoading || isPlaying) return;

        const instance = prelaunchState.instance;

        // Validar que la instancia exista
        if (!instance) {
            playSound('ERROR_NOTIFICATION');
            toast.error("Error al iniciar la instancia", {
                description: "No se pudo iniciar la instancia. Intenta nuevamente más tarde.",
                dismissible: true,
            });
            return;
        }

        // Validar que haya una cuenta seleccionada
        if (!instance.accountUuid) {
            playSound('ERROR_NOTIFICATION');
            toast.error("Sin cuenta seleccionada", {
                description: "Debes seleccionar una cuenta de Minecraft para jugar (puedes hacerlo desde la configuración de instancia).",
                dismissible: true,
                icon: <LucideUnplug className="size-4 text-white" />,
            });
            return;
        }

        const accountExists = await invoke("ensure_account_exists", { uuid: instance.accountUuid });
        if (!accountExists) {
            playSound('ERROR_NOTIFICATION');
            toast.error("Cuenta no encontrada", {
                description: "La cuenta asociada a esta instancia no existe. Por favor, verifica la configuración de la instancia.",
                dismissible: true,
            });
            return;
        }

        try {
            // Registrar evento y actualizar estado antes de lanzar
            trackEvent("play_instance_clicked", {
                name: "Play Minecraft Instance Clicked",
                modpackId: "null",
                timestamp: new Date().toISOString(),
            });

            setLoadingStatus(prev => ({ ...prev, isLoading: true }));
            await invoke("launch_mc_instance", { instanceId });
            startMessageInterval();
        } catch (error) {
            console.error("Error launching instance:", error);
            playSound('ERROR_NOTIFICATION'); // Añadido sonido de error
            toast.error("Error al iniciar la instancia", {
                description: "No se pudo iniciar la instancia. Intenta nuevamente más tarde.",
                dismissible: true,
            });
        }
    }, [instanceId, loadingStatus.isLoading, isPlaying, prelaunchState.instance, startMessageInterval]);


    // Discord RPC handling
    const updateDiscordRPC = useCallback(() => {
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

    // Audio handling
    const handleAudio = useCallback(() => {
        if (!appearance?.audio?.url) return;

        // Initialize audio if it doesn't exist yet
        if (!audioRef.current) {
            audioRef.current = new Audio(appearance.audio.url);
            audioRef.current.loop = true;
            audioRef.current.volume = 0.01;
        }

        const audio = audioRef.current;

        // Control playback based on game state
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

    // Update loading status based on instance
    const updateLoadingStatus = useCallback(() => {
        if (!currentInstanceRunning) return;

        const isLoading = currentInstanceRunning.status === "preparing" || currentInstanceRunning.status === "downloading-assets";

        setLoadingStatus(prev => ({
            ...prev,
            isLoading,
            message: currentInstanceRunning.message || getRandomMessage(),
        }));

        // Show error toast if instance has an error
        if (currentInstanceRunning.status === "error") {
            toast.error("Error en la instancia", {
                id: "instance-runtime-error",
                description: currentInstanceRunning.message || "Ha ocurrido un error al ejecutar la instancia.",
                dismissible: false,
            });
        }
    }, [currentInstanceRunning, getRandomMessage]);

    // Effect hooks
    // 1. Initial setup and cleanup
    useEffect(() => {
        setTitleBarState(prev => ({ ...prev, canGoBack: true }));

        // Cleanup function
        return () => {
            if (messageIntervalRef.current) {
                window.clearInterval(messageIntervalRef.current);
                messageIntervalRef.current = null;
            }
        };
    }, [setTitleBarState]);

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
        fetchInstanceData();
    }, [fetchInstanceData]);

    // 4. Load appearance
    useEffect(() => {
        loadAppearance();
    }, [loadAppearance]);

    // 5. Update Discord RPC
    useEffect(() => {
        updateDiscordRPC();
    }, [updateDiscordRPC]);

    // 6. Audio handling
    useEffect(() => {
        return handleAudio();
    }, [handleAudio]);

    // 7. Update loading status based on current instance
    useEffect(() => {
        updateLoadingStatus();
    }, [updateLoadingStatus]);

    // Render functions
    const renderLoading = () => (
        <div className="flex items-center justify-center min-h-screen h-full w-full">
            <LucideLoaderCircle className="size-10 -mt-12 animate-spin-clockwise animate-iteration-count-infinite animate-duration-1000 text-white" />
        </div>
    );

    const renderError = () => {
        toast.error(prelaunchState.error || "Error desconocido", {
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
    };

    const renderLoadingIndicator = () => (
        loadingStatus.isLoading && (
            <div className="flex gap-x-2 absolute animate-fade-in-down animate-duration-400 ease-in-out z-20 top-12 right-4 bg-black/80 px-2 py-1 max-w-xs w-full text-white items-center">
                <LucideLoaderCircle className="animate-spin-clockwise animate-iteration-count-infinite animate-duration-[2500ms] text-white flex-shrink-0" />
                {loadingStatus.message}
            </div>
        )
    );

    const renderBackground = () => (
        appearance?.background?.imageUrl ? (
            <img
                className="absolute inset-0 z-0 h-full w-full object-cover animate-fade-in ease-in-out duration-1000"
                src={appearance.background.imageUrl}
                alt="Background"
            />
        ) : appearance?.background?.videoUrl ? (
            <video
                className="absolute inset-0 z-0 h-full w-full object-cover animate-fade-in ease-in-out duration-1000"
                autoPlay
                loop
                muted
            >
                {Array.isArray(appearance.background.videoUrl)
                    ? appearance.background.videoUrl.map((url, index) => (
                        <source key={index} src={url} type="video/mp4" />
                    ))
                    : <source src={appearance.background.videoUrl} type="video/mp4" />
                }
            </video>
        ) : null
    );

    const renderLogo = () => {
        if (!appearance?.logo?.url) return null;

        const logoHasCustomPosition = appearance?.logo?.position &&
            Object.values(appearance.logo.position).some(value => value != null);

        return (
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
        );
    };

    const renderFooter = () => {
        const hasCustomPosition = appearance?.playButton?.position &&
            Object.values(appearance.playButton.position).some(value => value != null);

        return (
            <footer className="absolute bottom-0 left-0 right-0 z-10 bg-black/50 p-4 text-white flex items-center justify-center">
                <div className="flex flex-col items-center justify-center space-y-4">
                    <button
                        style={{
                            "--bg-color": appearance?.playButton?.backgroundColor,
                            "--hover-color": appearance?.playButton?.hoverColor,
                            "--text-color": appearance?.playButton?.textColor,
                            "--border-color": appearance?.playButton?.borderColor,
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
                        font-minecraft-ten
                        not-disabled:mc-play-btn
                        disabled:border-3
                        tracking-wide
                        text-shadow-[0_3px_0_rgba(0,0,0,0.25)]
                        items-center flex gap-x-2
                        disabled:bg-neutral-800 disabled:cursor-not-allowed
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

                    {/* Footer content */}
                    <div className="flex items-center justify-center space-x-2">
                        {
                            appearance?.logo?.url ? null : (
                                <img
                                    src="https://saltouruguayserver.com/favicon.svg"
                                    className="h-8 w-8"
                                    alt="Logo"
                                />
                            )
                        }
                        {
                            appearance?.footerText ? (
                                <span className="text-sm text-center">
                                    {appearance.footerText}
                                </span>
                            ) : null
                        }

                    </div>
                </div>
            </footer>
        );
    };

    const renderQuickActions = () => {
        if (!prelaunchState.instance) return null;

        return (
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
        );
    };

    // Loading and error handling
    if (prelaunchState.isLoading) {
        return renderLoading();
    }

    if (prelaunchState.error) {
        return renderError();
    }

    // Main render
    return (
        <div className="absolute inset-0">
            <div className="relative h-full w-full overflow-hidden">
                {renderLoadingIndicator()}
                {renderBackground()}
                {renderLogo()}
                {renderFooter()}
                {renderQuickActions()}
            </div>
        </div>
    );
};