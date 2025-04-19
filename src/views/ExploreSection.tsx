import { useEffect, useState } from "react"
import { useGlobalContext } from "../stores/GlobalContext"
import { LucideLoader, LucideSearch, LucideShoppingBag } from "lucide-react"
import { getModpacks, searchModpacks } from "@/services/getModpacks"
import { CategoryHorizontalSection } from "../components/CategoryHorizontalSection"
import { clearActivity, setActivity } from "tauri-plugin-drpc"
import { Activity, Assets, Timestamps } from "tauri-plugin-drpc/activity"
import { useDebounce } from 'use-debounce'
import { ModpackCard } from "@/components/ModpackCard"
import { trackEvent } from "@aptabase/web"
import { trackSectionView } from "@/lib/analytics"
import { motion } from "motion/react"

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

        trackSectionView('explore')

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

    useEffect(() => {
        if (debouncedSearch.trim().length < 3) return
        trackEvent("search_performed", {
            name: "Search performed",
            timestamp: new Date().toISOString(),
            searchTerm: debouncedSearch,
            totalResults: searchResults.length,
        })
    }, [debouncedSearch, searchResults.length])

    // Variantes para animaciones
    const containerVariants = {
        hidden: { opacity: 0 },
        visible: {
            opacity: 1,
            transition: {
                staggerChildren: 0.1,
                delayChildren: 0.2
            }
        }
    }

    const itemVariants = {
        hidden: { y: 20, opacity: 0 },
        visible: {
            y: 0,
            opacity: 1,
            transition: { type: "spring", stiffness: 100 }
        }
    }

    const fadeInVariants = {
        hidden: { opacity: 0 },
        visible: {
            opacity: 1,
            transition: { duration: 0.5 }
        }
    }

    const searchInputVariants = {
        focus: {
            scale: 1.02,
            boxShadow: "0 4px 20px rgba(0, 0, 0, 0.2)",
            transition: { type: "spring", stiffness: 400, damping: 25 }
        },
        blur: {
            scale: 1,
            boxShadow: "0 2px 10px rgba(0, 0, 0, 0.1)",
            transition: { type: "spring", stiffness: 400, damping: 25 }
        }
    }

    return (
        <motion.div
            initial="hidden"
            animate="visible"
            variants={fadeInVariants}
            className="mx-auto max-w-7xl px-4 py-10 overflow-y-auto"
        >
            <motion.header
                className="flex flex-col items-center justify-center gap-y-8 mb-16"
                variants={containerVariants}
            >
                <motion.div
                    className="flex flex-col items-center justify-between w-full max-w-3xl"
                    variants={containerVariants}
                >
                    <motion.h1
                        className="text-2xl font-semibold text-white"
                        variants={itemVariants}
                    >
                        Bienvenido a&nbsp;
                        <motion.span
                            className="from-[#bcfe47] to-[#05cc2a] bg-clip-text text-transparent bg-gradient-to-b"
                            animate={{
                                backgroundPosition: ["0% 0%", "100% 100%"],
                            }}
                            transition={{
                                duration: 5,
                                repeat: Infinity,
                                repeatType: "mirror"
                            }}
                        >
                            Modpack Store
                        </motion.span>
                    </motion.h1>

                    <motion.p
                        className="text-gray-400 text-base text-center"
                        variants={itemVariants}
                    >
                        Estás a pocos pasos de descubrir mundos e historias increíbles. <br />
                        Explora, elige y sumérgete en la aventura que más te guste.
                    </motion.p>

                    <motion.div
                        className="relative w-full bg-neutral-800 rounded-md shadow-lg mt-10"
                        variants={{
                            ...itemVariants,
                            ...searchInputVariants
                        }}
                        initial="blur"
                        whileFocus="focus"
                        whileHover="focus"
                        animate="blur"
                    >
                        <LucideSearch className="absolute left-4 top-1/2 -translate-y-1/2 text-gray-400" />
                        <input
                            type="text"
                            value={search}
                            onChange={(e) => setSearch(e.target.value)}
                            placeholder="Buscar modpacks..."
                            className="w-full h-12 pl-12 pr-16 bg-neutral-800 text-white rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 transition duration-200"
                        />
                    </motion.div>
                </motion.div>
            </motion.header>

            {loading ? (
                <motion.div
                    className="absolute inset-0 flex items-center justify-center"
                    initial={{ opacity: 0 }}
                    animate={{ opacity: 1 }}
                    exit={{ opacity: 0 }}
                >
                    <motion.div
                        animate={{
                            rotate: 360,
                        }}
                        transition={{
                            duration: 1.5,
                            repeat: Infinity,
                            ease: "linear"
                        }}
                    >
                        <LucideLoader className="size-10 -mt-12 text-white" />
                    </motion.div>
                </motion.div>
            ) : debouncedSearch.trim() !== "" ? (
                <motion.div
                    className="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-4 gap-6 text-white"
                    variants={containerVariants}
                    initial="hidden"
                    animate="visible"
                >
                    {searchResults.length > 0 ? (
                        searchResults.map((modpack: any, index) => (
                            <motion.div
                                key={modpack.id}
                                variants={itemVariants}
                                custom={index}
                                whileHover={{ scale: 1.05 }}
                                whileTap={{ scale: 0.98 }}
                                transition={{ type: "spring", stiffness: 400, damping: 17 }}
                            >
                                <ModpackCard
                                    modpack={modpack}
                                    href={`/prelaunch/${modpack.id}`}
                                />
                            </motion.div>
                        ))
                    ) : (
                        <motion.p
                            className="col-span-full text-center text-gray-400"
                            initial={{ opacity: 0 }}
                            animate={{ opacity: 1 }}
                            transition={{ delay: 0.5 }}
                        >
                            No se encontraron resultados.
                        </motion.p>
                    )}
                </motion.div>
            ) : (
                <motion.div
                    variants={containerVariants}
                    initial="hidden"
                    animate="visible"
                >
                    {modpackCategories.map((category: any, index) => (
                        <motion.div
                            key={category.id}
                            variants={itemVariants}
                            custom={index}
                        >
                            <CategoryHorizontalSection
                                id={category.id}
                                title={category.name}
                                shortDescription={category.shortDescription}
                                modpacks={category.modpacks}
                                href="/prelaunch/"
                            />
                        </motion.div>
                    ))}

                    <motion.div
                        className="flex flex-col text-white text-center items-center mt-8"
                        variants={fadeInVariants}
                    >
                        <motion.p
                            variants={itemVariants}
                        >
                            Explora una amplia variedad de modpacks y personaliza tu experiencia de juego <br />
                            ¡Descubre nuevos mundos y aventuras!
                        </motion.p>
                        <motion.img
                            src="https://pngimg.com/d/minecraft_PNG2.png"
                            draggable="false"
                            className="h-32 opacity-50 grayscale-100 w-auto object-scale-down mt-4"
                            variants={itemVariants}
                            whileHover={{
                                scale: 1.1,
                                opacity: 0.7,
                                rotate: [0, -3, 3, -2, 0],
                                transition: { duration: 0.5 }
                            }}
                        />
                    </motion.div>
                </motion.div>
            )}
        </motion.div>
    )
}