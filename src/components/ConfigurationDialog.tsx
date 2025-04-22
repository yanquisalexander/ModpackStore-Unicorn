import { useEffect, useState } from 'react';
import { motion, AnimatePresence } from 'motion/react';
import { toast } from "sonner";
import { useAuthentication } from '@/stores/AuthContext';
import { invoke } from '@tauri-apps/api/core';
import { trackSectionView } from "@/lib/analytics";
import { open } from "@tauri-apps/plugin-dialog";

// Lucide Icons
import {
    Settings as LucideSettings,
    Folder as LucideFolder,
    Save as LucideSave,
    Loader as LucideLoader,
    X as LucideX
} from "lucide-react";

// shadcn/ui components
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from "@/components/ui/card";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Switch } from "@/components/ui/switch";
import { Label } from "@/components/ui/label";
import { Separator } from "@/components/ui/separator";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";

// Tipos
interface ConfigurationDialogProps {
    isOpen: boolean;
    onClose: () => void;
}

// Tipos para el esquema de configuración
interface ConfigSchema {
    [key: string]: ConfigDefinition;
}

interface ConfigDefinition {
    type: string;
    default: any;
    description: string;
    ui_section: string;
    client?: boolean;
    min?: number;
    max?: number;
    choices?: any[];
    validator?: string;
}

export const ConfigurationDialog = ({ isOpen, onClose }: ConfigurationDialogProps) => {
    const { isAuthenticated } = useAuthentication();

    // Estado para los valores de configuración
    const [configValues, setConfigValues] = useState<Record<string, any>>({});
    const [configSchema, setConfigSchema] = useState<ConfigSchema>({});

    // UI States
    const [loading, setLoading] = useState<boolean>(true);
    const [activeTab, setActiveTab] = useState<string>("");
    const [sections, setSections] = useState<string[]>([]);
    const [saving, setSaving] = useState<boolean>(false);

    // Cargar configuración cuando se abre el diálogo
    useEffect(() => {
        if (isOpen) {
            trackSectionView("configuration");
            loadConfig();
        }
    }, [isOpen]);

    // Detectar tecla Escape
    useEffect(() => {
        const handleKeyDown = (e: KeyboardEvent) => {
            if (e.key === 'Escape' && isOpen) {
                onClose();
            }
        };

        if (isOpen) {
            window.addEventListener('keydown', handleKeyDown);
        }

        return () => {
            window.removeEventListener('keydown', handleKeyDown);
        };
    }, [isOpen, onClose]);

    // Cargar configuración y esquema
    async function loadConfig() {
        try {
            setLoading(true);

            // Cargar esquema
            const schema = await invoke<ConfigSchema>('get_schema');
            setConfigSchema(schema);

            // Extraer secciones del esquema
            const uniqueSections = extractSections(schema);
            setSections(uniqueSections);

            // Establecer la primera sección como activa
            if (uniqueSections.length > 0 && !activeTab) {
                setActiveTab(uniqueSections[0]);
            }

            // Cargar valores actuales
            const config = await invoke<Record<string, any>>('get_config');
            setConfigValues(config);

            setLoading(false);
        } catch (error) {
            console.error("Failed to load config:", error);
            toast.error("Error al cargar la configuración", {
                description: "No se pudo cargar la configuración. Intenta nuevamente.",
            });
            setLoading(false);
        }
    }

    // Extraer secciones únicas del esquema
    const extractSections = (schema: ConfigSchema): string[] => {
        const sections = new Set<string>();

        Object.values(schema).forEach(def => {
            if (def.ui_section) {
                sections.add(def.ui_section);
            }
        });

        return Array.from(sections);
    };

    // Obtener las definiciones de configuración para una sección
    const getConfigsForSection = (section: string): [string, ConfigDefinition][] => {
        return Object.entries(configSchema)
            .filter(([_, def]) => def.ui_section === section)
            .sort(([a], [b]) => a.localeCompare(b));
    };

    // Manejar cambios en los valores de configuración
    const handleConfigChange = (key: string, value: any) => {
        setConfigValues(prev => ({
            ...prev,
            [key]: value
        }));
    };

    // Guardar la configuración
    const handleSaveConfig = async () => {
        try {
            setSaving(true);

            // Guardar cada valor de configuración
            for (const [key, value] of Object.entries(configValues)) {
                await invoke('set_config', { key, value });
            }

            // Continuación del código desde donde se cortó en paste-3.txt, manteniendo el diseño original
            toast.success("Configuración guardada", {
                description: "Los cambios han sido guardados correctamente.",
                richColors: true,
            });

            setSaving(false);
            onClose();
        } catch (error) {
            console.error("Error al guardar configuración:", error);
            toast.error("Error al guardar", {
                description: "No se pudo guardar la configuración. Intenta nuevamente.",
            });
            setSaving(false);
        }
    };

    // Renderizar un control de configuración según su tipo
    const renderConfigControl = (key: string, def: ConfigDefinition) => {
        const value = configValues[key];

        switch (def.type) {
            case "string":
                return (
                    <Input
                        value={value || def.default}
                        onChange={(e) => handleConfigChange(key, e.target.value)}
                        className="bg-neutral-800 border-neutral-700 text-white"
                    />
                );

            case "integer":
            case "float":
                return (
                    <Input
                        type="number"
                        value={value || def.default}
                        min={def.min}
                        max={def.max}
                        onChange={(e) => handleConfigChange(key, parseFloat(e.target.value))}
                        className="bg-neutral-800 border-neutral-700 text-white"
                    />
                );

            case "boolean":
                return (
                    <Switch
                        checked={value === true}
                        onCheckedChange={(checked) => handleConfigChange(key, checked)}
                    />
                );

            case "path":
                return (
                    <div className="flex gap-2">
                        <Input
                            value={value || def.default}
                            onChange={(e) => handleConfigChange(key, e.target.value)}
                            className="bg-neutral-800 border-neutral-700 text-white"
                            readOnly={def.validator !== undefined}
                        />
                        <Button
                            variant="outline"
                            onClick={() => selectDirectory(key, value)}
                            className="border-neutral-700 hover:bg-neutral-800 hover:text-white"
                        >
                            <LucideFolder className="h-4 w-4 mr-2" />
                            Examinar
                        </Button>
                    </div>
                );

            case "enum":
                return (
                    <Select
                        value={value || def.default}
                        onValueChange={(val) => handleConfigChange(key, val)}
                    >

                        <SelectTrigger className="bg-neutral-800 border-neutral-700 text-white">
                            <SelectValue placeholder={def.description} />
                        </SelectTrigger>
                        <SelectContent className="bg-neutral-800 border-neutral-700 text-white z-9999">
                            {def.choices?.map((choice, idx) => (
                                <SelectItem key={idx} value={choice}>
                                    {choice}
                                </SelectItem>
                            ))}
                        </SelectContent>
                    </Select>
                );

            default:
                return (
                    <Input
                        value={String(value) || String(def.default)}
                        onChange={(e) => handleConfigChange(key, e.target.value)}
                        className="bg-neutral-800 border-neutral-700 text-white"
                    />
                );
        }
    };

    // Seleccionar un directorio
    const selectDirectory = async (key: string, currentPath: string) => {
        try {
            const selected = await open({
                directory: true,
                multiple: false,
                defaultPath: currentPath,
                title: `Seleccionar ${configSchema[key]?.description || "directorio"}`
            });

            if (selected && !Array.isArray(selected)) {
                handleConfigChange(key, selected);
            }
        } catch (error) {
            console.error("Error al seleccionar directorio:", error);
            toast.error("Error al seleccionar directorio");
        }
    };

    return (
        <AnimatePresence>
            {isOpen && (
                <motion.div
                    className="fixed inset-0 z-999 flex items-center justify-center bg-black bg-opacity-75 overflow-hidden"
                    initial={{ opacity: 0 }}
                    animate={{ opacity: 1 }}
                    exit={{ opacity: 0 }}
                    transition={{ duration: 0.2 }}
                >
                    <motion.div
                        className="w-full h-full flex flex-col overflow-hidden"
                        initial={{ y: 20, opacity: 0 }}
                        animate={{ y: 0, opacity: 1 }}
                        exit={{ y: 20, opacity: 0 }}
                        transition={{
                            type: "spring",
                            stiffness: 300,
                            damping: 30
                        }}
                    >
                        {/* Header Bar */}
                        <div className="bg-neutral-900 border-b border-neutral-800 p-4 flex justify-between items-center">
                            <div className="flex items-center gap-2">
                                <LucideSettings className="h-5 w-5 text-blue-500" />
                                <h2 className="text-xl font-semibold text-white">Configuración</h2>
                            </div>
                            <Button
                                variant="ghost"
                                size="icon"
                                onClick={onClose}
                                className="rounded-full hover:bg-neutral-800"
                            >
                                <LucideX className="h-5 w-5 text-neutral-400" />
                            </Button>
                        </div>

                        {/* Content */}
                        <div className="flex-1 overflow-y-auto bg-neutral-950 p-4">
                            {loading ? (
                                <div className="flex items-center justify-center h-full w-full text-white">
                                    <div className="flex flex-col items-center gap-2">
                                        <LucideLoader className="h-8 w-8 animate-spin text-blue-500" />
                                        <p className="text-lg">Cargando configuración...</p>
                                    </div>
                                </div>
                            ) : (
                                <div className="max-w-4xl mx-auto">
                                    <Card className="bg-neutral-950/50 border-neutral-800">
                                        <CardHeader>
                                            <CardTitle className="text-2xl font-semibold text-white">
                                                Configuración
                                            </CardTitle>
                                            <CardDescription className="text-neutral-400">
                                                Personaliza los ajustes del launcher según tus preferencias
                                            </CardDescription>
                                        </CardHeader>


                                        {sections.length > 0 && (
                                            <Tabs value={activeTab} onValueChange={setActiveTab} className="w-full">
                                                <TabsList className="bg-neutral-900 border-b border-neutral-800 mx-4">
                                                    {sections.map((section) => (
                                                        <TabsTrigger key={section} value={section}>
                                                            {section.charAt(0).toUpperCase() + section.slice(1)}
                                                        </TabsTrigger>
                                                    ))}
                                                </TabsList>

                                                <CardContent className="pt-6">
                                                    {sections.map((section) => (
                                                        <TabsContent key={section} value={section} className="space-y-6">
                                                            <div className="space-y-4">
                                                                {getConfigsForSection(section).map(([key, def], index, array) => (
                                                                    <div key={key}>
                                                                        {def.type === "boolean" ? (
                                                                            <div className="flex items-center justify-between">
                                                                                <div className="space-y-0.5">
                                                                                    <Label className="text-base font-medium text-white">
                                                                                        {def.description}
                                                                                    </Label>
                                                                                    <p className="text-sm text-neutral-400">
                                                                                        {/* Descripción adicional si hay */}
                                                                                    </p>
                                                                                </div>
                                                                                {renderConfigControl(key, def)}
                                                                            </div>
                                                                        ) : (
                                                                            <div className="space-y-2">
                                                                                <Label htmlFor={key} className="text-sm font-medium text-white">
                                                                                    {def.description}
                                                                                </Label>
                                                                                {renderConfigControl(key, def)}
                                                                                <p className="text-xs text-neutral-400">
                                                                                    {/* Descripción adicional si la hay */}
                                                                                </p>
                                                                            </div>
                                                                        )}

                                                                        {index < array.length - 1 && (
                                                                            <Separator className="bg-neutral-800 my-4" />
                                                                        )}
                                                                    </div>
                                                                ))}

                                                                {/* Usuarios autenticados con opciones adicionales */}
                                                                {isAuthenticated && section === "gameplay" && (
                                                                    <>
                                                                        <Separator className="bg-neutral-800" />
                                                                        <div className="space-y-4">
                                                                            <div className="flex items-center justify-between">
                                                                                <div className="space-y-0.5">
                                                                                    <Label className="text-base font-medium text-white">
                                                                                        Opciones avanzadas
                                                                                    </Label>
                                                                                    <p className="text-sm text-neutral-400">
                                                                                        Opciones adicionales para usuarios autenticados
                                                                                    </p>
                                                                                </div>
                                                                            </div>
                                                                        </div>
                                                                    </>
                                                                )}

                                                                {/* Área para futuras opciones si la sección es gameplay */}
                                                                {section === "gameplay" && (
                                                                    <div className="h-32 flex items-center justify-center rounded-md border border-dashed border-neutral-700 bg-neutral-900/50">
                                                                        <p className="text-sm text-neutral-400">
                                                                            Más opciones estarán disponibles en futuras versiones
                                                                        </p>
                                                                    </div>
                                                                )}
                                                            </div>
                                                        </TabsContent>
                                                    ))}
                                                </CardContent>
                                            </Tabs>
                                        )}
                                    </Card>
                                </div>
                            )}
                        </div>

                        {/* Footer */}
                        <div className="bg-neutral-900 border-t border-neutral-800 p-4 flex justify-end">
                            <div className="flex gap-2">
                                <Button
                                    variant="outline"
                                    onClick={onClose}
                                    className="border-neutral-700 hover:bg-neutral-800 text-white"
                                >
                                    Cancelar
                                </Button>
                                <Button
                                    onClick={handleSaveConfig}
                                    disabled={loading || saving}
                                    className="bg-blue-600 hover:bg-blue-500 text-white font-medium"
                                >
                                    {saving ? (
                                        <>
                                            <LucideLoader className="h-4 w-4 mr-2 animate-spin" />
                                            Guardando...
                                        </>
                                    ) : (
                                        <>
                                            <LucideSave className="h-4 w-4 mr-2" />
                                            Guardar cambios
                                        </>
                                    )}
                                </Button>
                            </div>
                        </div>
                    </motion.div>
                </motion.div>
            )}
        </AnimatePresence>
    );
};