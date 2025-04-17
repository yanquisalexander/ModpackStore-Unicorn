import { useEffect, useState } from "react"
import { useGlobalContext } from "../stores/GlobalContext"
import { LucideLoader, LucideSearch, LucideShoppingBag } from "lucide-react"
import { getModpacks } from "@/services/getModpacks"
import { invoke } from "@tauri-apps/api/core"
import { CategoryHorizontalSection } from "../components/CategoryHorizontalSection"
import { clearActivity, setActivity } from "tauri-plugin-drpc";
import { Activity, Assets, Timestamps } from "tauri-plugin-drpc/activity"

export const ExploreSection = () => {
    const { titleBarState, setTitleBarState } = useGlobalContext()
    const [modpackCategories, setModpackCategories] = useState<any>([])
    const [loading, setLoading] = useState(true)

    useEffect(() => {
        setTitleBarState({
            ...titleBarState,
            title: "Modpack Store",
            icon: LucideShoppingBag,
            canGoBack: false,
            customIconClassName: "bg-pink-500/10",
            opaque: true,
        })

        const activity = new Activity()
            .setState("Explorando Modpacks")
            .setTimestamps(new Timestamps(Date.now()))
            .setAssets(new Assets().setLargeImage("exploring").setSmallImage("exploring"))
        setActivity(activity)


    }, [])

    useEffect(() => {
        setLoading(true)

        getModpacks()
            .then((res) => {
                setModpackCategories(res)
                console.log(res)
            })
            .catch((err) => {
                console.error(err)
            })
            .finally(() => {
                setLoading(false)
            })


    }, [])




    return (
        <div className="mx-auto max-w-7xl px-4 py-10 overflow-y-auto">
            <header className="flex flex-col items-center justify-center gap-y-8 mb-16">
                {/* 
                    Header welcome and search bar
                */}
                <div className="flex flex-col items-center justify-between w-full max-w-3xl">
                    <h1 className="text-2xl font-semibold text-white">Bienvenido a Modpack Store</h1>
                    <p className="text-gray-400 text-base text-center">
                        Estás a pocos pasos de descubrir mundos e historias increíbles. <br />
                        Explora, elige y sumérgete en la aventura que más te guste.
                    </p>
                    <div className="relative w-full bg-neutral-800 rounded-md shadow-lg mt-10">
                        <LucideSearch className="absolute left-4 top-1/2 -translate-y-1/2 text-gray-400" />
                        <input
                            type="text"
                            placeholder="Buscar modpacks..."
                            className="w-full h-12 pl-12 pr-16 bg-neutral-800 text-white rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 transition duration-200"
                        />

                    </div>
                </div>
            </header>


            {loading ? (
                <div className="absolute inset-0 flex items-center justify-center">
                    <LucideLoader className="size-10 -mt-12 animate-spin-clockwise animate-iteration-count-infinite animate-duration-1000 text-white" />
                </div>
            ) : (
                <>

                    {
                        modpackCategories.map((category: any) => (
                            <CategoryHorizontalSection
                                id={category.id}
                                shortDescription={category.shortDescription}
                                key={category.id}
                                title={category.name}
                                modpacks={category.modpacks}
                                href="/prelaunch/"
                            />


                        ))
                    }

                    <div className="flex flex-col text-white text-center items-center mt-8"> {/* New section for additional content */}
                        <p>
                            Explora una amplia variedad de modpacks y personaliza tu experiencia de juego <br />¡Descubre nuevos mundos y aventuras!
                        </p>
                        <img src="https://pngimg.com/d/minecraft_PNG2.png"
                            draggable="false"
                            className="h-32 opacity-50 grayscale-100 w-auto object-scale-down mt-4"
                        />
                    </div>


                </>
            )}


        </div>
    )
}