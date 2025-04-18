// src/views/ConfigurationSection.tsx
import { useEffect, useState } from 'react';
import configManager, { ConfigKey } from '../utils/ConfigManager';
import { open } from '@tauri-apps/plugin-dialog';
import { toast } from "sonner";
import { useGlobalContext } from "../stores/GlobalContext";
import { LucideSettings, LucideFolder, LucideSave, LucideLoader } from "lucide-react";

// shadcn/ui components
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from "@/components/ui/card";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Switch } from "@/components/ui/switch";
import { Label } from "@/components/ui/label";
import { Separator } from "@/components/ui/separator";

export const ConfigurationSection = () => {
    const { titleBarState, setTitleBarState } = useGlobalContext();

    const [instancesDir, setInstancesDir] = useState<string>('');
    const [javaDir, setJavaDir] = useState<string>('');
    const [closeOnLaunch, setCloseOnLaunch] = useState<boolean>(true);
    const [loading, setLoading] = useState<boolean>(true);
    const [activeTab, setActiveTab] = useState<string>("directories");

    useEffect(() => {
        setTitleBarState({
            ...titleBarState,
            title: "Configuración",
            icon: LucideSettings,
            canGoBack: true,
            customIconClassName: "bg-blue-500/10",
            opaque: true,
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
                toast.error("Error al cargar la configuración", {
                    description: "No se pudo cargar la configuración. Intenta nuevamente.",
                });
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
            toast.success("Configuración guardada", {
                description: "Los cambios han sido guardados correctamente.",
                richColors: true,
            });
        } catch (error) {
            console.error("Error al guardar configuración:", error);
            toast.error("Error al guardar", {
                description: "No se pudo guardar la configuración. Intenta nuevamente.",
            });
        }
    };

    const selectInstancesDir = async () => {
        try {
            const selected = await open({
                directory: true,
                multiple: false,
                defaultPath: instancesDir,
                title: 'Seleccionar Directorio de Instancias'
            });
            if (selected && !Array.isArray(selected)) {
                setInstancesDir(selected);
            }
        } catch (error) {
            console.error("Error al seleccionar directorio:", error);
            toast.error("Error al seleccionar directorio");
        }
    };

    const selectJavaDir = async () => {
        try {
            const selected = await open({
                directory: true,
                multiple: false,
                title: 'Seleccionar Directorio de Java',
                defaultPath: javaDir,
            });
            console.log("Selected Java Directory:", selected);
            if (selected && !Array.isArray(selected)) {
                setJavaDir(selected);
            }
        } catch (error) {
            console.error("Error al seleccionar directorio:", error);
            toast.error("Error al seleccionar directorio");
        }
    };

    if (loading) {
        return (
            <div className="flex items-center justify-center h-full w-full text-white">
                <div className="flex flex-col items-center gap-2">
                    <LucideLoader className="h-8 w-8 animate-spin-clockwise animate-iteration-count-infinite animate-duration-1000 text-blue-500" />
                    <p className="text-lg">Cargando configuración...</p>
                </div>
            </div>
        );
    }

    return (
        <div className="max-w-4xl mx-auto h-full pt-16 px-4">
            <Card className="bg-neutral-950/50 border-neutral-800">
                <CardHeader>
                    <CardTitle className="text-2xl font-semibold text-white flex items-center gap-2">
                        <LucideSettings className="h-6 w-6 text-blue-500" />
                        Configuración
                    </CardTitle>
                    <CardDescription className="text-neutral-400">
                        Personaliza los ajustes del launcher según tus preferencias
                    </CardDescription>
                </CardHeader>

                <Tabs value={activeTab} onValueChange={setActiveTab} className="w-full">
                    <TabsList className="grid grid-cols-2 max-w-md mx-4 dark">
                        <TabsTrigger value="directories">Directorios</TabsTrigger>
                        <TabsTrigger value="gameplay">Ejecución</TabsTrigger>
                    </TabsList>

                    <CardContent className="pt-6">
                        <TabsContent value="directories" className="space-y-6">
                            <div className="space-y-4">
                                <div className="space-y-2">
                                    <Label htmlFor="instances-dir" className="text-sm font-medium text-white">
                                        Directorio de Instancias
                                    </Label>
                                    <div className="flex gap-2">
                                        <Input
                                            id="instances-dir"
                                            value={instancesDir}
                                            className="bg-neutral-800 border-neutral-700 text-white"
                                            placeholder="Selecciona el directorio de instancias"
                                            readOnly
                                        />
                                        <Button
                                            variant="outline"
                                            onClick={selectInstancesDir}
                                            className="border-neutral-700 hover:bg-neutral-800 hover:text-white"
                                        >
                                            <LucideFolder className="h-4 w-4 mr-2" />
                                            Examinar
                                        </Button>
                                    </div>
                                    <p className="text-xs text-neutral-400">
                                        Ubicación donde se guardarán todas las instancias de Minecraft
                                    </p>
                                </div>

                                <Separator className="bg-neutral-800" />

                                <div className="space-y-2">
                                    <Label htmlFor="java-dir" className="text-sm font-medium text-white">
                                        Directorio de Java
                                    </Label>
                                    <div className="flex gap-2">
                                        <Input
                                            id="java-dir"
                                            value={javaDir}
                                            className="bg-neutral-800 border-neutral-700 text-white"
                                            placeholder="Selecciona el directorio de Java"
                                            readOnly
                                        />
                                        <Button
                                            variant="outline"
                                            onClick={selectJavaDir}
                                            className="border-neutral-700 hover:bg-neutral-800 hover:text-white"
                                        >
                                            <LucideFolder className="h-4 w-4 mr-2" />
                                            Examinar
                                        </Button>
                                    </div>
                                    <p className="text-xs text-neutral-400">
                                        Ubicación de la instalación de Java para ejecutar Minecraft
                                    </p>
                                </div>
                            </div>
                        </TabsContent>

                        <TabsContent value="gameplay" className="space-y-6">
                            <div className="space-y-4">
                                <div className="flex items-center justify-between">
                                    <div className="space-y-0.5">
                                        <Label className="text-base font-medium text-white">
                                            Cerrar launcher automáticamente
                                        </Label>
                                        <p className="text-sm text-neutral-400">
                                            Cierra el launcher cuando se inicie Minecraft
                                        </p>
                                    </div>
                                    <Switch
                                        defaultChecked={closeOnLaunch}
                                        onCheckedChange={setCloseOnLaunch}
                                    />
                                </div>

                                <Separator className="bg-neutral-800" />

                                {/* Espacio para futuras opciones */}
                                <div className="h-32 flex items-center justify-center rounded-md border border-dashed border-neutral-700 bg-neutral-900/50">
                                    <p className="text-sm text-neutral-400">
                                        Más opciones estarán disponibles en futuras versiones
                                    </p>
                                </div>
                            </div>
                        </TabsContent>
                    </CardContent>

                    <CardFooter className="border-t border-neutral-800 pt-6 bg-neutral-900/30">
                        <div className="w-full flex justify-end">
                            <Button
                                onClick={handleSaveConfig}
                                className="bg-blue-600 hover:bg-blue-500 text-white font-medium"
                            >
                                <LucideSave className="h-4 w-4 mr-2" />
                                Guardar cambios
                            </Button>
                        </div>
                    </CardFooter>
                </Tabs>
            </Card>
        </div>
    );
};