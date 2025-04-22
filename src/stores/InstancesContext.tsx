import { createContext, useContext, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { trackEvent } from "@aptabase/web";
import { toast } from "sonner";
import { playSound } from "@/utils/sounds";

type InstanceState = {
    id: string;
    name: string;
    status: "idle" | "preparing" | "running" | "exited" | "error" | "downloading-assets";
    message: string;
};

// El contexto solo expone las instancias
const InstancesContext = createContext<{
    instances: InstanceState[];
}>({
    instances: [],
});

export const InstancesProvider = ({ children }: { children: React.ReactNode }) => {
    const [instances, setInstances] = useState<InstanceState[]>([]);

    // Estas funciones son internas al provider y no se exponen
    const addInstance = (instance: InstanceState) => {
        setInstances(prev => {
            // Si ya existe una instancia con este ID, la actualizamos en lugar de añadir una nueva
            const exists = prev.some(inst => inst.id === instance.id);
            if (exists) {
                return prev.map(inst =>
                    inst.id === instance.id ? { ...inst, ...instance } : inst
                );
            }
            return [...prev, instance];
        });
    };

    const updateInstance = (id: string, updates: Partial<InstanceState>) => {
        setInstances(prev => {
            const instanceExists = prev.some(instance => instance.id === id);
            if (instanceExists) {
                return prev.map(instance =>
                    instance.id === id ? { ...instance, ...updates } : instance
                );
            } else {
                return [...prev, { id, ...updates } as InstanceState];
            }
        });
    };

    const removeInstance = (id: string) => {
        setInstances(prev => prev.filter(instance => instance.id !== id));
    };

    useEffect(() => {
        const unlistenList: (() => void)[] = [];

        const setupListeners = async () => {
            // Evento para cuando se inicia la preparación de una instancia
            const launchStartUnlisten = await listen("instance-launch-start", (e: any) => {
                const { id, name, message } = e.payload;
                console.log("Launch start event:", { id, name, message });

                addInstance({
                    id,
                    name: name || `Instance ${id}`,
                    status: "preparing",
                    message: message || "Iniciando la instancia..."
                });
            });
            unlistenList.push(launchStartUnlisten);

            // Evento para cuando se están descargando assets
            const downloadingUnlisten = await listen("instance-downloading-assets", (e: any) => {
                const { id, message } = e.payload;
                console.log("Downloading assets event:", { id, message });

                updateInstance(id, {
                    status: "downloading-assets",
                    message: message || "Descargando archivos necesarios..."
                });
            });
            unlistenList.push(downloadingUnlisten);

            const finishAssetsDownloadUnlisten = await listen("instance-finish-assets-download", (e: any) => {
                const { id, message } = e.payload;
                console.log("Finish assets download event:", { id, message });

                updateInstance(id, {
                    status: "idle",

                });
            })
            unlistenList.push(finishAssetsDownloadUnlisten);

            // Evento para cuando la instancia ha sido lanzada
            const launchedUnlisten = await listen("instance-launched", (e: any) => {
                const { id, message } = e.payload;
                console.log("Instance launched event:", { id, message });
                trackEvent("instance_launched", {
                    instanceId: id,
                    message: message || "Minecraft se está ejecutando"
                });

                updateInstance(id, {
                    status: "running",
                    message: message || "Minecraft está ejecutándose"
                });

                // Minima ventana de la aplicación
                const window = getCurrentWindow();
                window.minimize();
            });
            unlistenList.push(launchedUnlisten);

            // Evento para cuando la instancia ha salido
            const exitedUnlisten = await listen("instance-exited", (e: any) => {
                const { id, message, data, name: instanceName } = e.payload;
                const { exitCode } = data || { exitCode: "desconocido" };
                console.log(e.payload);
                console.log("Instance exited event:", { id, message });

                trackEvent("instance_exited", {
                    instanceId: id,
                    message: message || "Minecraft se ha cerrado"
                });

                updateInstance(id, {
                    status: "exited",
                    message: message || "Minecraft se ha cerrado"
                });

                // Unminima la ventana de la aplicación
                const window = getCurrentWindow();
                window.unminimize();
                window.setFocus();

                if (exitCode !== 0) {
                    toast.error(`La instancia "${instanceName}" se ha cerrado con el código de error ${exitCode}`, {
                        duration: 10000,
                        description: "Esto puede ser causado por un error en la configuración de la instancia o un problema con tu instalación de Java."
                    });
                    playSound("ERROR_NOTIFICATION")
                    trackEvent("instance_crash", {
                        instanceId: id,
                        message: message || "Minecraft se ha cerrado inesperadamente"
                    });
                }

                // Opcional: quitar la instancia después de un tiempo
                setTimeout(() => removeInstance(id), 5000);
            });
            unlistenList.push(exitedUnlisten);

            // Evento para cuando hay un error en la instancia
            const errorUnlisten = await listen("instance-error", (e: any) => {
                const { id, message } = e.payload;
                console.log("Instance error event:", { id, message });
                trackEvent("instance_error", {
                    instanceId: id,
                    message: message || "Error al iniciar la instancia"
                });

                updateInstance(id, {
                    status: "error",
                    message: message || "Ha ocurrido un error"
                });

                // Unminima la ventana de la aplicación
                const window = getCurrentWindow();
                window.unminimize();
                window.setFocus();
            });
            unlistenList.push(errorUnlisten);
        };

        setupListeners();
        return () => unlistenList.forEach(unlisten => unlisten());
    }, []); // Sin dependencias para evitar problemas de recreación

    return (
        <InstancesContext.Provider value={{ instances }}>
            {children}
        </InstancesContext.Provider>
    );
};

export const useInstances = () => useContext(InstancesContext);