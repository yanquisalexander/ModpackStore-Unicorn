import { useGlobalContext } from "@/stores/GlobalContext"
import { invoke } from "@tauri-apps/api/core"
import { LucideGamepad2, LucideLoaderCircle } from "lucide-react"
import { useEffect, useState } from "react"

export const PreLaunchInstance = ({ instance }: { instance: any }) => {

    const [loadingStatus, setLoadingStatus] = useState({
        isLoading: false,
        message: "Descargando archivos necesarios...",
        progress: 0,
        logs: []
    })

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


    const { titleBarState, setTitleBarState } = useGlobalContext()

    useEffect(() => {
        setTitleBarState({
            ...titleBarState,
            title: "SaltoCraft 3",
            icon: "https://saltouruguayserver.com/favicon.svg",
            canGoBack: true,
            customIconClassName: ""
        })
    }, [])

    return (
        <>
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
                className="absolute opacity-80 inset-0 z-1 h-full w-full object-cover animate-fade-in ease-in-out duration-1000" src="https://images.steamusercontent.com/ugc/2310974141604980016/B4EF3A7A2D1772DE26B1A6F51CE33A04FD8BB917/" />

            {/* Logo del modpack */}

            <img src="https://saltouruguayserver.com/images/logo-saltocraft.webp" className="absolute top-24 left-1/2 -translate-x-1/2 z-10 h-32 drop-shadow-lg animate-delay-1000 animate-fade-in duration-500 ease-in-out" alt="Logo" />
            <footer className="absolute bottom-0 left-0 right-0 z-10 bg-black/50 p-4 text-white flex items-center justify-center">

                <div className="flex flex-col items-center justify-center space-y-4">
                    <button
                        id="play-button"
                        onClick={handlePlayButtonClick}
                        disabled={loadingStatus.isLoading}
                        className="bg-green-600 font-monocraft cursor-pointer active:scale-95 transition active:bg-neutral-800 px-4 py-2  border-3 border-white items-center flex gap-x-2 font-semibold hover:bg-green-700 disabled:bg-neutral-800 disabled:cursor-not-allowed">
                        <LucideGamepad2 className="size-6 text-white" />
                        Jugar ahora
                    </button>

                    <div className="flex items-center justify-center space-x-2">
                        <img src="https://saltouruguayserver.com/favicon.svg" className="h-8 w-8" alt="Logo" />
                        <span className="text-sm">
                            Un modpack de SaltoUruguayServer
                        </span>
                    </div>
                </div>
            </footer>
        </>
    )
}