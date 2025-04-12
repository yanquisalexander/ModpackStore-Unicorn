import { useEffect, useState } from "react"
import { useGlobalContext } from "../stores/GlobalContext"
import { LucideShoppingBag } from "lucide-react"
import { getModpacks } from "@/services/getModpacks"
import { invoke } from "@tauri-apps/api/core"
import { CategoryHorizontalSection } from "../components/CategoryHorizontalSection"
import { clearActivity, setActivity } from "tauri-plugin-drpc";
import { Activity, Assets, Timestamps } from "tauri-plugin-drpc/activity"

export const ExploreSection = () => {
    const { titleBarState, setTitleBarState } = useGlobalContext()
    const [modpacks, setModpacks] = useState([])
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
                setModpacks(res)
            })
            .catch((err) => {
                console.error(err)
            })
            .finally(() => {
                setLoading(false)
            })

        invoke("get_all_instances").then((res) => {
            console.log("Instances", res)
        })
    }, [])

    // Sample categories - you'd likely get these from your API
    const categories = [
        { id: "featured", name: "Destacados" },
        { id: "popular", name: "Populares" },
        { id: "new", name: "Recién agregados" },
        { id: "tech", name: "Tecnología" }
    ]

    // For demonstration, we'll just divide the modpacks into categories
    // In a real app, your modpacks would have category information
    const getCategoryModpacks = (categoryId: string) => {
        if (loading || !modpacks.length) return []

        // This is just for demo - you'd filter based on actual category data
        return modpacks.slice(0, 8)
    }

    return (
        <div className="mx-auto max-w-7xl px-4 py-10">
            <h1 className="text-3xl font-semibold mb-8 text-white animate-fade-in-up">
                Explorar Modpacks
            </h1>

            {loading ? (
                <div className="flex justify-center items-center py-20">
                    <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-white"></div>
                </div>
            ) : (
                <>


                    {/* Horizontal scrolling categories */}
                    {categories.map(category => (
                        <CategoryHorizontalSection
                            key={category.id}
                            title={category.name}
                            modpacks={getCategoryModpacks(category.id)}
                            href="/prelaunch/"
                            viewAllLink={`/category/${category.id}`}
                        />
                    ))}
                </>
            )}
        </div>
    )
}