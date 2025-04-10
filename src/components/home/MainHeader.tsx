import { LucideLayoutGrid, LucideServer, LucideUsers, LucideWrench } from "lucide-react"
import { Link, useLocation, useRoute } from "wouter"

export const HomeMainHeader = ({ session }: { session: any }) => {
    const [location] = useLocation();
    const SECTIONS = [
        {
            path: "/",
            title: "Explorar",
            icon: LucideLayoutGrid
        },
        {
            path: "/my-instances",
            title: "Mis Instancias",
            icon: LucideServer
        },
        {
            path: "/mc-accounts",
            title: "Cuentas",
            icon: LucideUsers
        },
        {
            path: "/settings",
            title: "Configuración",
            icon: LucideWrench
        }
    ]

    //(alias) matchRoute<undefined, PathPattern>(parser: Parser, pattern: PathPattern, path: string, loose?: boolean): Match<RegexRouteParams | {


    const SHOULD_SHOW_HEADER = SECTIONS.some((section) => section.path === location)




    if (!SHOULD_SHOW_HEADER) {
        return null
    }


    return (
        <header className="flex h-16 w-full items-center justify-between bg-ms-primary text-white select-none px-4">
            <nav className="flex items-center gap-x-2">
                {
                    SECTIONS.map((section) => (
                        <Link
                            href={section.path}
                            key={section.path}
                            className={`flex items-center transition hover:bg-neutral-800 gap-2 text-white py-2 px-3 ${location === section.path ? "bg-neutral-500/10" : ""}`}
                        >
                            <section.icon className="h-5 w-5" />
                            {section.title}
                        </Link>
                    ))
                }
            </nav>
            <div className="flex-1"></div>
            <div className="flex items-center gap-2">

            </div>
        </header>
    )
}