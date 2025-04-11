import { useGlobalContext } from "@/stores/GlobalContext";
import { LucidePackageOpen } from "lucide-react";
import { useEffect } from "react"


export const MyInstancesSection = () => {
    const { titleBarState, setTitleBarState } = useGlobalContext()

    useEffect(() => {
        setTitleBarState({
            ...titleBarState,
            title: "Mis instancias",
            icon: LucidePackageOpen,
            canGoBack: true,
            customIconClassName: "bg-yellow-500/10",
            opaque: true,
        });
    }, [])

    return (
        <div className="flex items-center justify-center min-h-dvh h-full w-full">
            <div className="text-white text-2xl font-semibold">
                Mis instancias
            </div>
        </div>
    )
}