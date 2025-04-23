import { Link } from "wouter"
import { LucideCheck, LucidePackage, LucidePlay, LucideSparkles, LucideUser2, LucideVerified } from "lucide-react"

export const ModpackCard = ({ modpack, href = "/prelaunch/", className = "" }: { modpack: any, href?: string, className?: string }) => {
    // Verificamos si debemos mostrar el usuario como publicador
    const { showUserAsPublisher } = modpack

    // Creamos una copia del publisher para mostrar el usuario si es necesario
    let displayPublisher = { ...modpack.publisher }
    const originalPublisherName = displayPublisher.publisherName

    // Si debemos mostrar el usuario como publisher, cambiamos el nombre
    if (showUserAsPublisher && modpack.creatorUser) {
        displayPublisher = {
            ...displayPublisher,
            publisherName: modpack.creatorUser.username || "Desconocido",
        }
    }

    // Definimos clases para los diferentes tipos de publisher
    const publisherTagClasses = {
        NORMAL: "bg-gray-800 text-white border-white/10",
        VERIFIED: "bg-green-700 text-white",
        PARTNERED: "bg-indigo-500 text-white",
        AFFILIATE: "bg-twitch-purple text-white",
    }

    // Determinamos qué clase usar basado en el tipo de publisher
    let publisherClass = publisherTagClasses.NORMAL

    // Solo aplicamos verificado o partner si no estamos mostrando al usuario
    if (!showUserAsPublisher) {
        if (displayPublisher.partnered) {
            publisherClass = publisherTagClasses.PARTNERED
        } else if (displayPublisher.verified) {
            publisherClass = publisherTagClasses.VERIFIED
        }
    }

    // Si es un hosting partner y mostramos al usuario, usamos la clase de afiliado
    if (showUserAsPublisher && displayPublisher.isHostingPartner) {
        publisherClass = publisherTagClasses.AFFILIATE
    }

    return (
        <article className={`z-10 group relative overflow-hidden rounded-xl border border-white/20 h-full
      transition 
      before:left-1/2 before:bottom-0 before:-translate-x-1/2 before:w-full before:h-1/2 
      before:rounded-full before:bg-black before:absolute before:translate-y-full 
      hover:before:translate-y-1/2 before:blur-3xl before:-z-10 before:transition before:duration-200 
      after:left-0 after:bottom-0 after:-translate-x-full after:translate-y-full 
      hover:after:-translate-x-1/2 hover:after:translate-y-1/2 after:w-2/2 after:aspect-square 
      after:rounded-2xl after:bg-black after:absolute after:blur-3xl hover:after:opacity-40 
      after:-z-10 after:opacity-0 after:transition after:duration-200 ${className}`}>

            <Link href={href} className="flex aspect-video flex-col h-full p-4">
                {/* Background image */}
                <img
                    src={modpack.bannerUrl}
                    onError={(e) => { e.currentTarget.src = "/images/modpack-fallback.webp" }}
                    className="absolute inset-0 -z-20 transform-gpu animate-fade-in object-cover w-full h-full rounded-xl transition duration-500 group-hover:scale-105 group-hover:opacity-80"
                    alt={modpack.name}
                />

                {/* Tags section */}
                <div className="opacity-100 flex transition flex-col gap-2 flex-1">
                    <div className="flex justify-end items-start flex-wrap gap-2 transition group-hover:opacity-100 -translate-y-1 group-hover:translate-y-0 opacity-0 duration-300">
                        {/* Publisher Badge con texto que no se rompe */}
                        {showUserAsPublisher && displayPublisher.isHostingPartner ? (
                            <div className="flex flex-col items-end gap-1">
                                <span className={`backdrop-blur-2xl text-xs border rounded-full inline-flex items-center gap-1 py-1 px-2 font-medium ${publisherClass} max-w-full overflow-hidden text-ellipsis`}>
                                    <LucideUser2 className="h-4 w-auto flex-shrink-0" />
                                    <span className="truncate">{displayPublisher.publisherName}</span>
                                </span>

                                {/* Badge separado para "de [Publisher]" */}
                                <span className={`backdrop-blur-2xl text-xs border rounded-full inline-flex items-center gap-1 py-1 px-2 font-medium bg-yellow-500 max-w-full`}>
                                    <span className="whitespace-nowrap text-black text-xs">de {originalPublisherName}</span>
                                    {displayPublisher.verified && (
                                        <LucideVerified className="w-4 h-4 text-black flex-shrink-0" />
                                    )}
                                </span>
                            </div>
                        ) : (
                            <span className={`backdrop-blur-2xl text-xs border rounded-full inline-flex items-center gap-1 py-1 px-2 font-medium ${publisherClass} max-w-full`}>
                                {!showUserAsPublisher && (displayPublisher.partnered || displayPublisher.verified) ? (
                                    <LucideVerified className="h-4 w-auto flex-shrink-0" />
                                ) : (
                                    <LucideUser2 className="h-4 w-auto flex-shrink-0" />
                                )}
                                <span className="truncate">{displayPublisher.publisherName}</span>
                            </span>
                        )}


                    </div>
                </div>

                {/* Title and actions section */}
                <div className="flex flex-wrap gap-y-6 items-end justify-between mt-8 transition group-hover:opacity-100 translate-y-1 group-hover:translate-y-0 opacity-0 duration-300">
                    <div>
                        <h2 className="text-lg mt-auto text-white leading-snug font-medium text-balance max-w-[28ch] group-hover:text-sky-200">
                            {modpack.name}
                        </h2>
                        <div className="flex items-center gap-4 mt-2 text-sm text-gray-300 flex-wrap">
                            <p className="flex items-center gap-1">
                                <span className="p-1 w-6 h-6 aspect-square border border-gray-400/10 bg-gray-800 rounded-full flex items-center justify-center">
                                    <LucidePackage className="text-gray-300 w-3 h-auto" />
                                </span>
                                Modpack
                            </p>
                        </div>
                    </div>
                    <span className="text-white rounded-lg bg-gray-800/20 border border-gray-400/40 py-2 px-4 flex items-center gap-1.5 group-hover:scale-105 transition text-sm group-hover:bg-gray-800/80">
                        <LucidePlay className="h-4 w-auto" />
                        Ver más
                    </span>
                </div>
            </Link>
        </article>
    )
}