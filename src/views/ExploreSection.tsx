import { useEffect, useState } from "react"
import { useGlobalContext } from "../stores/GlobalContext"
import { LucideLoader, LucideSearch, LucideShoppingBag } from "lucide-react"
import { getModpacks, searchModpacks } from "@/services/getModpacks"
import { CategoryHorizontalSection } from "../components/CategoryHorizontalSection"
import { clearActivity, setActivity } from "tauri-plugin-drpc"
import { Activity, Assets, Timestamps } from "tauri-plugin-drpc/activity"
import { useDebounce } from 'use-debounce'
import { ModpackCard } from "@/components/ModpackCard"

export const ExploreSection = () => {
    const { titleBarState, setTitleBarState } = useGlobalContext()
    const [modpackCategories, setModpackCategories] = useState<any[]>([])
    const [searchResults, setSearchResults] = useState<any[]>([])
    const [loading, setLoading] = useState(true)
    const [search, setSearch] = useState("")
    const [debouncedSearch] = useDebounce(search, 300)

    useEffect(() => {
        setTitleBarState({
            ...titleBarState,
            title: "Modpack Store",
            icon: LucideShoppingBag,
            canGoBack: false,
            customIconClassName: "bg-pink-500/20",
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
        if (debouncedSearch.trim() !== "") {
            searchModpacks(debouncedSearch)
                .then(setSearchResults)
                .catch(console.error)
                .finally(() => setLoading(false))
        } else {
            getModpacks()
                .then(setModpackCategories)
                .catch(console.error)
                .finally(() => setLoading(false))
        }
    }, [debouncedSearch])

    return (
        <div className="mx-auto max-w-7xl px-4 py-10 overflow-y-auto">
            <header className="flex flex-col items-center justify-center gap-y-8 mb-16">
                <div className="flex flex-col items-center justify-between w-full max-w-3xl">
                    <h1 className="text-2xl font-semibold text-white">Bienvenido a&nbsp;
                        <span className=" from-[#bcfe47] to-[#05cc2a] bg-clip-text text-transparent bg-gradient-to-b">
                            Modpack Store
                        </span>
                    </h1>
                    <p className="text-gray-400 text-base text-center">
                        Estás a pocos pasos de descubrir mundos e historias increíbles. <br />
                        Explora, elige y sumérgete en la aventura que más te guste.
                    </p>
                    <div className="relative w-full bg-neutral-800 rounded-md shadow-lg mt-10">
                        <LucideSearch className="absolute left-4 top-1/2 -translate-y-1/2 text-gray-400" />
                        <input
                            type="text"
                            value={search}
                            onChange={(e) => setSearch(e.target.value)}
                            placeholder="Buscar modpacks..."
                            className="w-full h-12 pl-12 pr-16 bg-neutral-800 text-white rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 transition duration-200"
                        />
                    </div>
                </div>
            </header>

            {loading ? (
                <div className="absolute inset-0 flex items-center justify-center">
                    <LucideLoader className="size-10 -mt-12 animate-spin text-white" />
                </div>
            ) : debouncedSearch.trim() !== "" ? (
                <div className="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-4 gap-6 text-white">
                    {searchResults.length > 0 ? (
                        searchResults.map((modpack: any) => (
                            <ModpackCard
                                key={modpack.id}
                                modpack={modpack}
                                href={`/prelaunch/${modpack.id}`}
                            />
                        ))
                    ) : (
                        <p className="col-span-full text-center text-gray-400">No se encontraron resultados.</p>
                    )}
                </div>
            ) : (
                <>
                    {modpackCategories.map((category: any) => (
                        <CategoryHorizontalSection
                            key={category.id}
                            id={category.id}
                            title={category.name}
                            shortDescription={category.shortDescription}
                            modpacks={category.modpacks}
                            href="/prelaunch/"
                        />
                    ))}

                    <div className="flex flex-col text-white text-center items-center mt-8">
                        <p>
                            Explora una amplia variedad de modpacks y personaliza tu experiencia de juego <br />
                            ¡Descubre nuevos mundos y aventuras!
                        </p>
                        <img
                            src="https://pngimg.com/d/minecraft_PNG2.png"
                            draggable="false"
                            className="h-32 opacity-50 grayscale-100 w-auto object-scale-down mt-4"
                        />
                    </div>
                </>
            )}
        </div>
    )
}
