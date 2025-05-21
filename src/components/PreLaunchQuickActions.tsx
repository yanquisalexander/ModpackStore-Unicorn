import { useState, useRef, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { LucideFolderOpen, LucideLoaderCircle, LucideSettings, LucideShieldCheck } from "lucide-react";
import { toast } from "sonner";
import { EditInstanceInfo } from "@/components/EditInstanceInfo";

const PreLaunchQuickActions = ({
    instanceId,
    isForge = false,
    onReloadInfo,
    defaultShowEditInfo = false,
}: {
    instanceId: string;
    isForge?: boolean;
    onReloadInfo: () => void;
    defaultShowEditInfo?: boolean;
}) => {
    const [quickActionsOpen, setQuickActionsOpen] = useState(false);
    const quickActionsRef = useRef<HTMLDivElement>(null);

    // Click outside handler for quick actions menu
    useEffect(() => {
        const handleClickOutside = (event: MouseEvent) => {
            if (quickActionsRef.current && !quickActionsRef.current.contains(event.target as Node)) {
                setQuickActionsOpen(false);
            }
        };

        document.addEventListener("mousedown", handleClickOutside);
        return () => document.removeEventListener("mousedown", handleClickOutside);
    }, []);

    const toggleQuickActions = () => {
        setQuickActionsOpen(prev => !prev);
    };

    const openGameDir = async () => {
        try {
            await invoke("open_game_dir", { instanceId });
            toast.success("Abriendo carpeta de la instancia...");
            setQuickActionsOpen(false);
        } catch (error) {
            console.error("Error opening game directory:", error);
            toast.error("Error al abrir la carpeta de la instancia", {
                description: "No se pudo abrir la carpeta de la instancia. Intenta nuevamente.",
                dismissible: true,
            });
        }
    };

    const notAvailable = () => {
        setQuickActionsOpen(false);
        toast.error("Función no disponible aún", {
            description: "Esta función estará disponible en futuras versiones.",
        });
    };

    const verifyIntegrity = async () => {
        const invokeCommand = isForge ? null : "check_vanilla_integrity";
        if (!invokeCommand) {
            toast.error("Función no disponible para Forge", {
                description: "Esta función no está disponible para instancias de Forge.",
                dismissible: true,
            });
            return;
        }

        try {
            await invoke(invokeCommand, { instanceId });
            toast.success("Verificando integridad de archivos...");
            setQuickActionsOpen(false);
        } catch (error) {
            console.error("Error verifying integrity:", error);
            toast.error("Error al verificar la integridad de archivos", {
                description: "No se pudo verificar la integridad de archivos. Intenta nuevamente.",
                dismissible: true,
            });
        }
    }

    return (
        <div className="absolute right-0 bottom-40 z-40 group" ref={quickActionsRef}>
            <div className="flex items-center justify-end relative w-fit">
                {/* Settings button */}
                <button
                    onClick={toggleQuickActions}
                    className="size-12 cursor-pointer group hover:bg-neutral-900 transition bg-neutral-800 rounded-l-md flex items-center justify-center"
                >
                    <LucideSettings
                        style={{
                            transform: quickActionsOpen ? "rotate(90deg)" : "rotate(0deg)",
                            transition: "transform 0.3s ease-in-out",
                        }}
                        className="size-5 text-white"
                    />
                </button>

                {/* Actions menu */}
                <div
                    className={`absolute right-full bottom-0 mr-2 ${quickActionsOpen
                        ? "opacity-100 pointer-events-auto translate-x-0"
                        : "opacity-0 pointer-events-none translate-x-2"
                        } transition-all duration-300`}
                >
                    <div className="bg-neutral-900 border border-neutral-700 rounded-md shadow-md p-2 space-y-2 max-w-xs w-64">
                        <button
                            onClick={openGameDir}
                            className="cursor-pointer flex items-center gap-x-2 text-white w-full hover:bg-neutral-800 px-3 py-2 rounded-md transition"
                        >
                            <LucideFolderOpen className="size-4 text-white" />
                            Abrir .minecraft
                        </button>

                        <EditInstanceInfo
                            instanceId={instanceId}
                            onUpdate={onReloadInfo}
                            defaultShowEditInfo={defaultShowEditInfo}
                        />

                        {isForge && (
                            <button
                                onClick={notAvailable}
                                className="cursor-pointer flex items-center gap-x-2 text-white w-full hover:bg-neutral-800 px-3 py-2 rounded-md transition"
                            >
                                <LucideLoaderCircle className="size-4 text-white" />
                                Descargar mods
                            </button>
                        )}

                        <button
                            onClick={verifyIntegrity}
                            className="cursor-pointer flex items-center gap-x-2 text-white w-full hover:bg-neutral-800 px-3 py-2 rounded-md transition"
                        >
                            <LucideShieldCheck className="size-4 text-white" />
                            Verificar integridad
                        </button>
                    </div>
                </div>
            </div>
        </div>
    );
};

export default PreLaunchQuickActions;