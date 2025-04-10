import { useEffect, useState } from "react"
import { useGlobalContext } from "../stores/GlobalContext"
import { LucideShoppingBag } from "lucide-react"
import { getModpacks } from "@/services/getModpacks"
import { Link } from "wouter"
import { invoke } from "@tauri-apps/api/core"

export const ExploreSection = () => {

    const { titleBarState, setTitleBarState } = useGlobalContext()
    useEffect(() => {
        setTitleBarState({
            ...titleBarState,
            title: "Modpack Store",
            icon: LucideShoppingBag,
            canGoBack: false,
            customIconClassName: "bg-pink-500/10"
        })
    }, [])

    const [modpacks, setModpacks] = useState<any[]>([])

    useEffect(() => {
        getModpacks().then((res) => {
            setModpacks(res)
        }).catch((err) => {
            console.error(err)
        })

        invoke("get_all_instances").then((res) => {
            console.log("Instances", res)
        })

        console.log("Modpacks", modpacks)

    }, [])
    return (
        <div className="mx-auto max-w-7xl px-4 py-10 ">

            <h1 className="text-3xl font-semibold mb-8 text-white animate-fade-in-up">
                Explorar Modpacks
            </h1>


            <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
                <Link href="/prelaunch" className="flex flex-col gap-4">
                    <div className="relative group cursor-pointer rounded-lg overflow-hidden bg-gray-800 hover:bg-gray-700 transition duration-300 ease-in-out">
                        <img src="https://i.ytimg.com/vi/SEd1c4aAjQI/hq720.jpg?sqp=-oaymwEhCK4FEIIDSFryq4qpAxMIARUAAAAAGAElAADIQj0AgKJD&rs=AOn4CLBREzBncZkwqJZDsh2-wMVcEUvzaw" alt="Modpack" className="aspect-video w-full h-38 object-cover group-hover:scale-105 transition duration-300 ease-in-out" />
                        <div className="absolute inset-0 bg-black/50 flex items-center justify-center text-white font-semibold text-lg opacity-0 group-hover:opacity-100 transition duration-300 ease-in-out">
                            SaltoCraft 3
                        </div>
                    </div>
                </Link>
            </div>
        </div>
    )
}