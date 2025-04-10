// src/views/ConfigurationSection.tsx
import { useEffect, useState } from 'react';
import configManager, { ConfigKey } from '../utils/ConfigManager';
import { open } from "@tauri-apps/plugin-fs";
import { toast } from "sonner";
import { useGlobalContext } from "../stores/GlobalContext";
import { LucideSettings } from "lucide-react";
import { Input } from "@/components/ui/input";

export const ConfigurationSection = () => {
    const { titleBarState, setTitleBarState } = useGlobalContext()

    const [instancesDir, setInstancesDir] = useState<string>('');
    const [javaDir, setJavaDir] = useState<string>('');
    const [closeOnLaunch, setCloseOnLaunch] = useState<boolean>(true);
    const [loading, setLoading] = useState<boolean>(true);

    useEffect(() => {

        setTitleBarState({
            ...titleBarState,
            title: "Configuración",
            icon: LucideSettings,
            canGoBack: true,
            customIconClassName: "bg-blue-500/10"
        });

        async function loadConfig() {
            try {
                await configManager.init();
                setInstancesDir(configManager.getInstancesDir());
                setJavaDir(configManager.getJavaDir());
                setCloseOnLaunch(configManager.closeOnLaunchMinecraft());
                setLoading(false);
            } catch (error) {
                console.error("Failed to load config:", error);
                setLoading(false);
            }
        }

        loadConfig();
    }, []);

    const handleSaveConfig = async () => {
        try {
            await configManager.setConfig(ConfigKey.INSTANCES_DIR, instancesDir);
            await configManager.setConfig(ConfigKey.JAVA_DIR, javaDir);
            await configManager.setConfig(ConfigKey.CLOSE_ON_LAUNCH, closeOnLaunch);
            toast.success("Configuración guardada correctamente.", {
                richColors: true,
            })
        } catch (error) {
            console.error("Error al guardar configuración:", error);
            alert("Error al guardar configuración.");
        }
    };

    const selectInstancesDir = async () => {
        try {
            const selected = await open({
                directory: true,
                multiple: false,
                title: 'Seleccionar Directorio de Instancias'
            });
            if (selected && !Array.isArray(selected)) {
                setInstancesDir(selected);
            }
        } catch (error) {
            console.error("Error al seleccionar directorio:", error);
        }
    };

    const selectJavaDir = async () => {
        try {
            const selected = await open({
                directory: true,
                multiple: false,
                title: 'Seleccionar Directorio de Java'
            });
            if (selected && !Array.isArray(selected)) {
                setJavaDir(selected);
            }
        } catch (error) {
            console.error("Error al seleccionar directorio:", error);
        }
    };

    if (loading) {
        return <div className="text-white">Cargando configuración...</div>;
    }

    return (
        <div className="max-w-4xl mx-auto py-10 text-white">
            <h2 className="text-3xl font-semibold mb-8">Configuración</h2>

            <section className="space-y-6">
                {/* Directorios */}
                <div>
                    <h3 className="text-lg font-medium text-blue-400 mb-2">Directorios</h3>

                    <div className="mb-4">
                        <label className="block mb-1">Directorio de Instancias:</label>
                        <div className="flex gap-2 !select-text">
                            <Input
                                onInput={() => { }}
                                type="text"
                                value={instancesDir}
                                className=" bg-neutral-800"
                            />
                            <button
                                onClick={selectInstancesDir}
                                className="bg-neutral-700 px-4 py-2 rounded hover:bg-neutral-600 transition"
                            >
                                Examinar...
                            </button>
                        </div>
                    </div>

                    <div>
                        <label className="block mb-1">Directorio de Java:</label>
                        <div className="flex gap-2">
                            <Input
                                onInput={() => { }}
                                type="text"
                                value={javaDir}
                                className=" bg-neutral-800"
                            />
                            <button
                                onClick={selectJavaDir}
                                className="bg-neutral-700 px-4 py-2 rounded hover:bg-neutral-600 transition"
                            >
                                Examinar...
                            </button>
                        </div>
                    </div>
                </div>

                {/* Lanzamiento */}
                <div>
                    <h3 className="text-lg font-medium text-blue-400 mb-2">Lanzamiento</h3>
                    <label className="inline-flex items-center gap-2">
                        <input
                            type="checkbox"
                            checked={closeOnLaunch}
                            onChange={(e) => setCloseOnLaunch(e.target.checked)}
                            className="form-checkbox accent-blue-500"
                        />
                        Cerrar launcher automáticamente
                    </label>
                </div>

                {/* Botón de guardar */}
                <div className="flex justify-end">
                    <button
                        onClick={handleSaveConfig}
                        className="bg-blue-600 hover:bg-blue-500 px-6 py-2 rounded-md text-white font-semibold"
                    >
                        Guardar
                    </button>
                </div>
            </section>
        </div>
    );
};
