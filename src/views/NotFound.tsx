import { Link } from "wouter"
import { LucideConstruction, LucideTrafficCone, LucideFrown, LucideHome } from "lucide-react"

export const NotFound = () => {
    return (
        <div className="relative flex flex-col items-center justify-center h-full text-white overflow-hidden">
            {/* Íconos de fondo */}
            <LucideConstruction className="absolute text-white opacity-10 w-64 h-64 -top-10 -left-20 rotate-12" />
            <LucideTrafficCone className="absolute text-white opacity-10 w-64 h-64 -bottom-10 -right-20 -rotate-12" />

            {/* Ícono principal */}
            <LucideFrown className="w-24 h-24 text-white mb-4 z-10" />

            {/* Mensaje */}
            <p className="mt-2 text-lg z-10 text-center px-4">
                Intentaste acceder a algo que no existe…<br /> o al menos por ahora.
            </p>

            <Link
                href="/"
                className="z-10 mt-6 px-4 py-2 bg-transparent rounded hover:bg-white/10 transition flex items-center justify-center gap-2 text-white font-medium border border-white/20 hover:border-white/30"
            >
                <LucideHome className="w-4 h-4" />
                Volver al inicio
            </Link>
        </div>
    )
}
