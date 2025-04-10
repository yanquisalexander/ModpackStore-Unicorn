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
        <div className="mx-auto max-w-7xl px-4 py-10 h-[3000px] overflow-scroll">
            <img src="https://i.pinimg.com/736x/f7/4c/e6/f74ce6007b53858d32503641f6dd88ba.jpg"
                className="absolute inset-0 -z-10"
            />
            <h1 className="text-3xl font-semibold mb-8 text-white animate-fade-in-up">
                Explorar Modpacks
            </h1>

            <Link href="/prelaunch" className="bg-blue-500 text-white py-2 px-4 rounded hover:bg-blue-600 transition duration-200">
                Prelaunch Instance
            </Link>

            <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
                {modpacks.map((modpack) => (
                    <div key={modpack.id} className="bg-white rounded-lg shadow-lg p-4 flex flex-col items-center">
                        <img src={modpack.icon} alt={modpack.name} className="w-32 h-32 mb-4" />
                        <h2 className="text-xl font-semibold">{modpack.name}</h2>
                        <p className="text-gray-600">{modpack.description}</p>
                        <button className="mt-4 bg-blue-500 text-white py-2 px-4 rounded hover:bg-blue-600 transition duration-200">
                            Install
                        </button>
                    </div>
                ))}
            </div>
        </div>
    )
}