import { useEffect, useState } from "react";
import { useGlobalContext } from "@/stores/GlobalContext";
import {
    LucideDownload,
    LucideCheckCircle2,
    LucideAlertCircle,
    LucideLoader2,
    LucideRocket,
    LucideX,
} from "lucide-react";
import { cn } from "@/lib/utils";

export const UpdateStatus = () => {
    const {
        isUpdating,
        updateProgress,
        updateVersion,
        updateState,
        setIsUpdating,
    } = useGlobalContext();

    const [visible, setVisible] = useState(false);

    // Mostrar el componente cuando se activa
    useEffect(() => {
        if (isUpdating) setVisible(true);
    }, [isUpdating]);

    // Ocultar luego de 6s si no es "downloading"
    useEffect(() => {
        if (!isUpdating || updateState === "downloading") return;

        const hideTimeout = setTimeout(() => {
            setVisible(false); // inicia animación
            setTimeout(() => {
                setIsUpdating(false); // desmonta después de animación
            }, 400);
        }, 6000);

        return () => clearTimeout(hideTimeout);
    }, [isUpdating, updateState, setIsUpdating]);

    if (!isUpdating) return null;

    const stateIcon = {
        downloading: <LucideDownload className="size-5 animate-spin text-blue-400" />,
        "ready-to-install": <LucideRocket className="size-5 text-green-400" />,
        error: <LucideAlertCircle className="size-5 text-red-400" />,
    }[updateState as string] ?? (
            <LucideLoader2 className="size-5 animate-spin text-gray-400" />
        );

    const message = {
        downloading: "Descargando actualización...",
        "ready-to-install": "¡Actualización lista para instalar!",
        error: "Ocurrió un error durante la actualización.",
    }[updateState as string] ?? updateState;

    const bgColor = {
        downloading: "from-blue-900/90 to-zinc-800/90",
        "ready-to-install": "from-green-900/90 to-zinc-800/90",
        error: "from-red-900/90 to-zinc-800/90",
    }[updateState as string] ?? "from-zinc-900/90 to-zinc-800/90";

    return (
        <div
            className={cn(
                "flex items-start gap-3 fixed bottom-6 right-6 z-50 w-80 p-4 rounded-2xl shadow-xl border border-white/10 backdrop-blur-md text-white transition-all duration-400",
                `bg-gradient-to-br ${bgColor}`,
                visible ? "opacity-100 translate-y-0" : "opacity-0 translate-y-4 pointer-events-none"
            )}
        >
            <div className="flex-shrink-0 mt-1">{stateIcon}</div>

            <div className="flex flex-col gap-1 w-full">
                <div className="text-sm font-medium">{message}</div>

                {updateVersion && (
                    <span className="text-xs text-gray-400">
                        Versión: {updateVersion}
                    </span>
                )}

                {updateProgress > 0 && updateProgress < 100 && (
                    <div className="h-1.5 bg-zinc-700 rounded-full overflow-hidden mt-1">
                        <div
                            className="h-full bg-blue-500 transition-all duration-200"
                            style={{ width: `${updateProgress}%` }}
                        ></div>
                    </div>
                )}

                {updateState === "ready-to-install" && (
                    <span className="text-xs text-green-400 mt-1 flex items-center gap-1">
                        <LucideCheckCircle2 className="size-4" />
                        Listo para reiniciar
                    </span>
                )}

                {updateState === "error" && (
                    <span className="text-xs text-red-400 mt-1 flex items-center gap-1">
                        <LucideAlertCircle className="size-4" />
                        Revisa tu conexión o intenta más tarde
                    </span>
                )}
            </div>
        </div>
    );
};
