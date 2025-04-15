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
    const [modpackCategories, setModpackCategories] = useState([])
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

                    {
                        modpackCategories.map((category) => (
                            <CategoryHorizontalSection
                                key={category.id}
                                title={category.name}
                                modpacks={category.modpacks}
                                href="/prelaunch/"
                                viewAllLink={`/category/${category.id}`}
                            />
                        ))
                    }

                    <div className="text-center text-white mt-8"> {/* New section for additional content */}
                        <p>
                            Explora una amplia variedad de modpacks y personaliza tu experiencia de juego <br />¡Descubre nuevos mundos y aventuras!
                        </p>
                    </div>


                </>
            )}
        </div>
    )
}