import { Link } from "wouter"
import { LucideCheck, LucidePackage, LucidePlay, LucideSparkles, LucideUser2 } from "lucide-react"

export const ModpackCard = ({ modpack, href = "/prelaunch/", className = "" }: { modpack: any, href?: string, className?: string }) => {
    // Default values if modpack properties aren't provided
    const {
        id = "default-id",
        title = "Modpack Title",
        imageUrl = modpack.icon_url || "https://via.placeholder.com/300",
        mcVersion = "1.16.5",
        tags = []
    } = modpack || {}

    return (
        <article className={`group relative overflow-hidden rounded-xl border border-white/20 h-full
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
                    src={imageUrl}
                    className="absolute inset-0 -z-20 transform-gpu object-cover w-full h-full rounded-xl transition duration-500 group-hover:scale-105 group-hover:opacity-80"
                    alt={title}
                />

                {/* Tags section */}
                <div className="opacity-100 flex transition flex-col gap-2 flex-1">
                    <div className="flex justify-end items-center flex-wrap gap-2 transition group-hover:opacity-100 -translate-y-1 group-hover:translate-y-0 opacity-0 duration-300">
                        <span className="backdrop-blur-2xl bg-gradient-to-br from-blue-500 to-sky-800 text-green-100 text-xs border rounded-full inline-flex items-center gap-1 border-green-200/40 py-1 px-2">
                            <LucideUser2 className="h-4 w-auto" />
                            SaltoUruguay
                        </span>

                        {tags && tags.length > 0 && tags.map((tag: string, index: number) => (
                            <span key={index} className="backdrop-blur-2xl uppercase bg-white text-slate-800 text-xs border rounded-full inline-flex items-center gap-1 border-slate-200/40 py-1 px-2">
                                <LucideSparkles className="h-4 w-auto" />
                                {tag}
                            </span>
                        ))}
                    </div>
                </div>

                {/* Title and actions section */}
                <div className="flex flex-wrap gap-y-6 items-end justify-between mt-8 transition group-hover:opacity-100 translate-y-1 group-hover:translate-y-0 opacity-0 duration-300">
                    <div>
                        <h2 className="text-lg mt-auto text-white leading-snug font-medium text-balance max-w-[28ch] group-hover:text-sky-200">
                            {title}
                        </h2>
                        <div className="flex items-center gap-4 mt-2 text-sm text-gray-300 flex-wrap">
                            <p className="flex items-center gap-1">
                                <span className="p-1 w-6 h-6 aspect-square border border-gray-400/10 bg-gray-800 rounded-full flex items-center justify-center">
                                    <LucidePackage className="text-gray-300 w-3 h-auto" />
                                </span>
                                Versión: <span>{mcVersion}</span>
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