import { useGlobalContext } from "@/stores/GlobalContext"
import { PreLaunchAppearance } from "@/types/PreLaunchAppeareance"
import { getDefaultAppeareance } from "@/utils/prelaunch"
import { invoke } from "@tauri-apps/api/core"
import { LucideGamepad2, LucideLoaderCircle } from "lucide-react"
import { CSSProperties, useEffect, useState } from "react"
import { toast } from "sonner"
import { navigate } from "wouter/use-browser-location"

export const PreLaunchInstance = ({ instanceId }: { instanceId: string }) => {
    const { titleBarState, setTitleBarState } = useGlobalContext()

    const [prelaunchState, setPrelaunchState] = useState<{
        isLoading: boolean,
        instanceId: string,
        error: string | null,
        instance: any | null,
    }>({
        isLoading: true,
        instanceId,
        error: null,
        instance: null,
    })

    const getMinecraftInstance = async (instanceId: string) => {
        setTitleBarState({
            ...titleBarState,
            canGoBack: true,
        })
        try {
            const instance = await invoke("get_instance_by_id", { instanceId })
            console.log("Fetched instance:", instance)
            if (!instance) {
                throw new Error("Instance not found")
            }
            setPrelaunchState({
                ...prelaunchState,
                isLoading: false,
                instance,
            })

            setTitleBarState({
                ...titleBarState,
                title: instance.instanceName,
                icon: "https://saltouruguayserver.com/favicon.svg",
                canGoBack: true,
                customIconClassName: "",
                opaque: false,
            })


        } catch (error) {
            console.error("Error fetching instance data:", error)
            setPrelaunchState({
                ...prelaunchState,
                isLoading: false,
                error: "Ocurrió un error al cargar la instancia",
            })
        }
    }

    useEffect(() => {
        getMinecraftInstance(instanceId)
    }, [])


    const [loadingStatus, setLoadingStatus] = useState({
        isLoading: false,
        message: "Descargando archivos necesarios...",
        progress: 0,
        logs: []
    })

    const [audio, setAudio] = useState<HTMLAudioElement | null>(null)



    const [appearance, setAppearance] = useState<PreLaunchAppearance | null>(null)

    useEffect(() => {
        // Cargar la apariencia del modpack
        invoke("get_prelaunch_appearance", { instanceId }).then((res) => {
            setAppearance(res as PreLaunchAppearance)
        }).catch((err) => {
            console.error("Error fetching appearance:", err)
            setAppearance({
                ...getDefaultAppeareance({
                    title: "SaltoCraft 3",
                    description: "Un modpack de SaltoUruguayServer",
                    logoUrl: "https://saltouruguayserver.com/favicon.svg",
                })
            })
        })
    }
        , [])

    useEffect(() => {
        // Cargar el audio (si existe) y reproducirlo en bucle
        if (!appearance?.audio?.url) return
        const audioElement = new Audio(appearance?.audio?.url)
        setAudio(audioElement)

        audioElement.play().catch((error) => {
            console.error("Error playing audio:", error);
        });

        audioElement.loop = true
        audioElement.volume = 0.01 // 8% de volumen 

        return () => {
            audioElement.pause()
            audioElement.currentTime = 0
        }
    }, [])

    // Verifica si hay una posición personalizada
    const hasCustomPosition = appearance?.playButton?.position &&
        Object.values(appearance.playButton.position).some(value => value != null);

    const logoHasCustomPosition = appearance?.logo?.position &&
        Object.values(appearance.logo.position).some(value => value != null);

    const handlePlayButtonClick = async () => {
        console.log("Launching Minecraft instance...")

        setLoadingStatus((prev) => ({
            ...prev,
            isLoading: true,
            message: getRandomMessage(),
        }))

        // Si después querés lanzar el juego, descomentá esto
        // await invoke("launch_mc_instance", {
        //     instancePath: "C:\\Users\\alexb\\ModpackStore\\Instances\\pepe",
        // })

        setInterval(() => {
            setLoadingStatus((prev) => ({
                ...prev,
                message: getRandomMessage(),
            }))
        }
            , 5000)
    }

    const getRandomMessage = () => {
        const messages = [
            "Descargando archivos necesarios...",
            "Cargando modpack...",
            "Muy pronto estarás jugando...",
            "Seguro que te va a encantar...",
            "Preparando todo para ti...",
        ]
        return messages[Math.floor(Math.random() * messages.length)]
    }






    if (prelaunchState.isLoading) {
        return (
            <div className="flex items-center justify-center min-h-screen h-full w-full">
                <LucideLoaderCircle className="size-10 -mt-12 animate-spin-clockwise animate-iteration-count-infinite animate-duration-1000 text-white" />
            </div>
        )
    }
    if (prelaunchState.error) {
        toast.error(prelaunchState.error, {
            id: "instance-error",
            description: "No se pudo cargar la instancia. Intenta nuevamente más tarde.",
            dismissible: false,
            action: {
                label: "Volver a inicio",
                onClick: () => {
                    navigate("/")
                },
            },
        })

        return (
            <div className="flex items-center justify-center min-h-screen h-full w-full">
                <div className="text-white text-lg">{prelaunchState.error}</div>
            </div>
        )
    }

    return (
        <div className="absolute inset-0">
            <div className="relative h-full w-full overflow-hidden">
                {
                    loadingStatus.isLoading && (
                        <div className="flex gap-x-2 absolute animate-fade-in-down animate-duration-400 ease-in-out z-20 top-12 right-4 bg-black/80 px-2 py-1 max-w-xs w-full text-white items-center"> {/* Añadí items-center para mejor alineación vertical */}
                            <LucideLoaderCircle className="animate-spin-clockwise animate-iteration-count-infinite animate-duration-[2500ms] text-white flex-shrink-0" /> {/* <-- Añadir flex-shrink-0 aquí */}
                            {loadingStatus.message}
                        </div>
                    )
                }
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
                    className={`absolute z-10 animate-fade-in duration-500 ease-in-out ${logoHasCustomPosition && "fixed"}`}
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
                            disabled={loadingStatus.isLoading}
                            className={`
                            ${hasCustomPosition && "fixed"}
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
                            {appearance?.playButton?.text ?? "Jugar ahora"}
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
    )
}