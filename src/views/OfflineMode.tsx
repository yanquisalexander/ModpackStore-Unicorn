import { useEffect } from "react";
import { LucideWifiOff, LucideRefreshCw, LucideUnplug } from "lucide-react";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { useGlobalContext } from "@/stores/GlobalContext";
import { MyInstancesSection } from "./MyInstancesSection";

export const OfflineMode = () => {

    const { titleBarState, setTitleBarState } = useGlobalContext();
    useEffect(() => {
        setTitleBarState({
            ...titleBarState,
            title: "Modpack Store (Sin conexión)",
            icon: LucideUnplug,
            customIconClassName: "text-emerald-400 bg-emerald-500/10",
            opaque: true,
            canGoBack: false,
        });

    }, [])

    return (
        <div className="flex flex-col h-screen text-white p-6">
            <Alert>
                <LucideWifiOff className="size-16  !text-red-400" />
                <AlertTitle>Sin conexión</AlertTitle>
                <AlertDescription>
                    Podrás jugar por tus instancias locales, pero no podrás descargar nuevos modpacks ni actualizarlos. <br />
                </AlertDescription>
            </Alert>


            <div className="mt-0">
                <MyInstancesSection offlineMode={true} />
            </div>
        </div>
    );
};