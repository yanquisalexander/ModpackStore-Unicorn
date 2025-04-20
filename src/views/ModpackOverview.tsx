import { getModpackById } from "@/services/getModpacks"
import { useGlobalContext } from "@/stores/GlobalContext"
import { LucideLoader, LucideVerified, LucideVolume2, LucideVolumeX } from "lucide-react"
import { useEffect, useState, useRef } from "react"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { motion, useScroll, useTransform } from "motion/react"
import { TauriCommandReturns } from "@/types/TauriCommandReturns"
import { invoke } from "@tauri-apps/api/core"

export const ModpackOverview = ({ modpackId }: { modpackId: string }) => {

    const [pageState, setPageState] = useState({
        loading: true,
        error: false,
        errorMessage: "",
        modpackData: null as any,
    })

    const [isMuted, setIsMuted] = useState(true)
    const [showVideo, setShowVideo] = useState(false)
    const [videoLoaded, setVideoLoaded] = useState(false)
    const videoRef = useRef<HTMLVideoElement>(null)
    const bannerContainerRef = useRef<HTMLDivElement>(null)
    const [localInstancesOfModpack, setLocalInstancesOfModpack] = useState<TauriCommandReturns["get_instances_by_modpack_id"]>([])

    const { titleBarState, setTitleBarState } = useGlobalContext()
    const { scrollY } = useScroll()

    // Transformaciones basadas en el scroll para el efecto parallax
    const bannerY = useTransform(scrollY, [0, 500], [0, 150])
    const bannerScale = useTransform(scrollY, [0, 300], [1.05, 1.15])
    const bannerOpacity = useTransform(scrollY, [0, 300], [1, 0.3])

    useEffect(() => {
        setTitleBarState({
            ...titleBarState,
            canGoBack: true,
            opaque: true,
            title: pageState.modpackData?.name || "Modpack Overview",
            icon: pageState.modpackData?.iconUrl || "/images/modpack-fallback.webp",
            customIconClassName: "rounded-sm",
        })
    }, [pageState.modpackData])



    useEffect(() => {
        const fetchLocalInstances = async () => {
            try {
                const instances = await invoke<TauriCommandReturns["get_instances_by_modpack_id"]>("get_instances_by_modpack_id", { modpackId });
                setLocalInstancesOfModpack(instances);
                console.log("Local instances of modpack:", instances);
            } catch (err) {
                console.error("Failed to fetch local instances:", err);
            }
        };

        fetchLocalInstances();
    }, [modpackId]);

    // Efecto para cargar el video con retraso
    useEffect(() => {
        if (pageState.loading || !pageState.modpackData?.trailerUrl) return;

        const timer = setTimeout(() => {
            setShowVideo(true);
        }, 3000);

        return () => clearTimeout(timer);
    }, [pageState.loading, pageState.modpackData]);

    // Efecto para manejar la visibilidad del video y pausarlo cuando no es visible
    useEffect(() => {
        if (!videoRef.current || !bannerContainerRef.current || !showVideo) return;

        const observer = new IntersectionObserver(
            ([entry]) => {
                if (entry.isIntersecting) {
                    if (videoLoaded) {
                        videoRef.current?.play();
                    }
                } else {
                    videoRef.current?.pause();
                }
            },
            { threshold: 0.1 }
        );

        observer.observe(bannerContainerRef.current);

        return () => {
            if (bannerContainerRef.current) {
                observer.unobserve(bannerContainerRef.current);
            }
        };
    }, [pageState.loading, showVideo, videoLoaded]);

    useEffect(() => {
        const fetchModpack = async () => {
            try {
                const modpack = await getModpackById(modpackId)
                setPageState({
                    loading: false,
                    error: false,
                    errorMessage: "",
                    modpackData: modpack
                })
            } catch (err: any) {
                setPageState({
                    loading: false,
                    error: true,
                    errorMessage: err?.message || "Failed to load modpack",
                    modpackData: null
                })
            }
        }

        fetchModpack()
    }, [modpackId])

    const toggleMute = () => {
        if (videoRef.current) {
            videoRef.current.muted = !videoRef.current.muted;
            setIsMuted(!isMuted);
        }
    };

    const handleVideoLoaded = () => {
        setVideoLoaded(true);
        if (videoRef.current && bannerContainerRef.current) {
            const observer = new IntersectionObserver(
                ([entry]) => {
                    if (entry.isIntersecting) {
                        videoRef.current?.play();
                    }
                },
                { threshold: 0.1 }
            );
            observer.observe(bannerContainerRef.current);
            return () => {
                if (bannerContainerRef.current) {
                    observer.unobserve(bannerContainerRef.current);
                }
            };
        }
    };

    const handleVideoEnd = () => {
        // Back again to banner image
        setShowVideo(false);
        setVideoLoaded(false);
    }

    if (pageState.loading) {
        return (
            <div className="flex items-center justify-center min-h-screen w-full">
                <LucideLoader className="size-10 animate-spin text-white" />
            </div>
        )
    }

    if (pageState.error) {
        return (
            <div className="flex flex-col items-center justify-center min-h-screen w-full text-red-500">
                <p className="text-lg font-semibold">Error:</p>
                <p>{pageState.errorMessage}</p>
            </div>
        )
    }

    const { modpackData } = pageState
    const publisher = modpackData.publisher
    const hasVideo = modpackData.trailerUrl && modpackData.trailerUrl.length > 0;

    return (
        <div className="relative w-full h-full">
            {/* Banner con parallax usando Framer Motion */}
            <div
                ref={bannerContainerRef}
                className="absolute inset-0 z-11 overflow-hidden w-full h-[60vh] aspect-video"
            >
                <motion.div
                    className="absolute inset-0 w-full h-full"
                    style={{
                        y: bannerY,
                        scale: bannerScale,
                        opacity: bannerOpacity
                    }}
                >
                    {/* Banner de imagen siempre presente */}
                    <div
                        className={`w-full h-full animate-fade-in bg-cover bg-center transition-opacity duration-1000 ${showVideo && videoLoaded ? 'opacity-0' : 'opacity-100'}`}
                        style={{ backgroundImage: `url(${modpackData.bannerUrl})` }}
                    />

                    {/* Video con fade in cuando está listo */}
                    {hasVideo && showVideo && (
                        <>
                            <div className={`absolute inset-0 w-full h-full transition-opacity duration-1000 ${videoLoaded ? 'opacity-100' : 'opacity-0'}`}>
                                <video
                                    ref={videoRef}
                                    muted
                                    playsInline
                                    autoPlay
                                    onEnded={handleVideoEnd}
                                    className="w-full h-full object-cover"
                                    src={modpackData.trailerUrl}
                                    onLoadedData={handleVideoLoaded}
                                />
                            </div>

                            {/* Botón visible siempre que haya video y esté activo */}
                            <button
                                onClick={toggleMute}
                                className="cursor-pointer absolute top-4 right-8 p-2 bg-black/50 backdrop-blur-sm rounded-full z-999"
                            >
                                {isMuted ? (
                                    <LucideVolumeX className="size-6 text-white" />
                                ) : (
                                    <LucideVolume2 className="size-6 text-white" />
                                )}
                            </button>
                        </>
                    )}

                </motion.div>

                {/* Capa de gradiente */}
                <div className="absolute inset-0 bg-gradient-to-b from-black/20 via-black/40 to-[#181818] pointer-events-none" />
            </div>

            {/* Contenido principal - con scroll normal */}
            <div className="relative z-10 min-h-screen">
                <motion.main
                    initial={{ opacity: 0, y: 20 }}
                    animate={{ opacity: 1, y: 0 }}
                    transition={{ duration: 0.5 }}
                    className="px-4 py-8 md:px-12 lg:px-24"
                >
                    <div className="flex flex-col gap-6 pt-[60vh]">
                        <motion.div
                            initial={{ opacity: 0, y: 20 }}
                            animate={{ opacity: 1, y: 0 }}
                            transition={{ duration: 0.6, delay: 0.1 }}
                            className="flex items-center gap-4"
                        >
                            <img
                                src={modpackData.iconUrl}
                                onError={(e) => {
                                    e.currentTarget.onerror = null; // Prevent infinite loop
                                    e.currentTarget.src = "/images/modpack-fallback.webp"; // Fallback image
                                }}
                                alt="Modpack Icon"
                                className="w-20 h-20 rounded-2xl shadow-md"
                            />
                            <div>
                                <h1 className="text-4xl font-bold text-white">{modpackData.name}</h1>
                                <div className="flex items-center gap-2 text-white/90 text-sm">
                                    <span>{publisher.publisherName}</span>
                                    {publisher.verified && <LucideVerified className="w-4 h-4 text-blue-400" />}
                                    {publisher.partnered && (
                                        <span className="bg-yellow-400 text-black text-xs font-medium px-2 py-0.5 rounded-md ml-2">
                                            Partner
                                        </span>
                                    )}
                                </div>
                            </div>
                        </motion.div>

                        {/* Tabs */}
                        <motion.div
                            initial={{ opacity: 0, y: 20 }}
                            animate={{ opacity: 1, y: 0 }}
                            transition={{ duration: 0.6, delay: 0.2 }}
                        >
                            <Tabs defaultValue="overview" className="w-full pb-16">
                                <TabsList className="w-full justify-start bg-black/40 backdrop-blur-md">
                                    <TabsTrigger value="overview">Descripción</TabsTrigger>
                                    <TabsTrigger value="mods">Mods</TabsTrigger>
                                    <TabsTrigger value="changelog">Changelog</TabsTrigger>
                                    <TabsTrigger value="versions">Versiones</TabsTrigger>
                                </TabsList>

                                <TabsContent value="overview" className="mt-6">
                                    <h2 className="text-xl font-semibold text-white">Descripción</h2>
                                    <p className="text-white/80 mt-2">
                                        {modpackData.description ?? "Este modpack aún no tiene una descripción."}
                                    </p>
                                </TabsContent>

                                <TabsContent value="mods" className="mt-6">
                                    <p className="text-white/80">Todavía no hay mods listados. (Coming soon)</p>
                                </TabsContent>

                                <TabsContent value="changelog" className="mt-6">
                                    <p className="text-white/80">No hay changelog disponible aún.</p>
                                </TabsContent>

                                <TabsContent value="versions" className="mt-6">
                                    <p className="text-white/80">Versión inicial: 1.0.0 (placeholder)</p>
                                </TabsContent>
                            </Tabs>
                        </motion.div>


                    </div>
                </motion.main>
            </div>
        </div>
    )
}