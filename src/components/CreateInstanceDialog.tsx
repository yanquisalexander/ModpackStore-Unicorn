import { useState, useEffect, PointerEventHandler } from "react"
import { invoke } from "@tauri-apps/api/core"
import { LucidePlus, Loader2, LucideAnvil } from "lucide-react"
import { TauriCommandReturns } from "@/types/TauriCommandReturns"

import {
    Dialog,
    DialogContent,
    DialogDescription,
    DialogFooter,
    DialogHeader,
    DialogTitle,
    DialogTrigger,
} from "@/components/ui/dialog"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { toast } from "sonner"
import { trackEvent } from "@aptabase/web"
import {
    Select,
    SelectContent,
    SelectItem,
    SelectTrigger,
    SelectValue,
} from "@/components/ui/select"
import { CreeperIcon } from "@/icons/CreeperIcon"


// Types for Minecraft and Forge versions
interface MinecraftVersion {
    id: string;
    type: string;
    url: string;
    time?: string;
    releaseTime?: string;
}

interface ForgeVersion {
    id: string;
    type: string;
    url: string;
}

// Instance type definition
type InstanceType = "vanilla" | "forge";

// Props type definition
interface CreateInstanceDialogProps {
    onInstanceCreated: () => void;
}

export const CreateInstanceDialog = ({ onInstanceCreated }: CreateInstanceDialogProps) => {
    const [open, setOpen] = useState(false);
    const [instanceName, setInstanceName] = useState("");
    const [isLoading, setIsLoading] = useState(false);
    const [minecraftVersions, setMinecraftVersions] = useState<MinecraftVersion[]>([]);
    const [forgeVersionsMap, setForgeVersionsMap] = useState<Record<string, ForgeVersion[]>>({});
    const [selectedType, setSelectedType] = useState<InstanceType>("vanilla");
    const [selectedMinecraftVersion, setSelectedMinecraftVersion] = useState<string>("");
    const [selectedForgeVersion, setSelectedForgeVersion] = useState<string>("");
    const [loadingVersions, setLoadingVersions] = useState(false);
    const [compatibleForgeVersions, setCompatibleForgeVersions] = useState<string[]>([]);

    // Fetch Minecraft versions when dialog opens
    useEffect(() => {
        if (open) {
            fetchMinecraftVersions();
        }
    }, [open]);

    // Update compatible forge versions when Minecraft version changes
    useEffect(() => {
        const forgeVersions = forgeVersionsMap[selectedMinecraftVersion] || [];
        setCompatibleForgeVersions(forgeVersions.map(version => version.id));

        // Set the first compatible forge version as selected if available
        if (forgeVersions.length > 0 && !selectedForgeVersion) {
            setSelectedForgeVersion(forgeVersions[0].id);
        } else if (forgeVersions.length === 0) {
            setSelectedForgeVersion("");
        }
    }, [selectedMinecraftVersion, forgeVersionsMap, selectedForgeVersion]);

    const fetchMinecraftVersions = async (): Promise<void> => {
        setLoadingVersions(true);
        try {
            // Fetch Minecraft versions
            const response = await fetch("https://launchermeta.mojang.com/mc/game/version_manifest.json");
            const data = await response.json();

            // Filter only release versions
            const releaseVersions = data.versions.filter((version: MinecraftVersion) =>
                version.type === "release"
            );

            setMinecraftVersions(releaseVersions);

            // Set default Minecraft version to latest release
            if (releaseVersions.length > 0) {
                setSelectedMinecraftVersion(releaseVersions[0].id);
            }

            // Also fetch forge versions
            await fetchForgeVersions();
        } catch (error) {
            console.error("Error fetching Minecraft versions:", error);
            toast.error("No se pudieron cargar las versiones de Minecraft");
        } finally {
            setLoadingVersions(false);
        }
    };

    const fetchForgeVersions = async (): Promise<void> => {
        try {
            const response = await fetch("https://mrnavastar.github.io/ForgeVersionAPI/forge-versions.json");
            const data = await response.json();
            setForgeVersionsMap(data);
        } catch (error) {
            console.error("Error fetching Forge versions:", error);
            toast.error("No se pudieron cargar las versiones de Forge");
        }
    };

    const handleCreateInstance = async (): Promise<void> => {
        if (!instanceName.trim()) {
            toast.error("Error", {
                description: "El nombre de la instancia no puede estar vacío"
            });
            return;
        }

        if (!selectedMinecraftVersion) {
            toast.error("Error", {
                description: "Debes seleccionar una versión de Minecraft"
            });
            return;
        }

        if (selectedType === "forge" && !selectedForgeVersion) {
            toast.error("Error", {
                description: "Debes seleccionar una versión de Forge"
            });
            return;
        }

        setIsLoading(true);

        try {
            // Prepare instance data based on type
            const instanceData = {
                instanceName: instanceName.trim(),
                mcVersion: selectedMinecraftVersion,
                type: selectedType,
                forgeVersion: selectedType === "forge" ? selectedForgeVersion : undefined
            };

            // Call Tauri command to create instance
            await invoke<TauriCommandReturns['create_instance']>('create_local_instance', instanceData);

            toast.success("Creando instancia...", {
                description: `Tu instancia "${instanceName}" está siendo creada. Verifica el progreso en el Task Manager.`,
            });

            trackEvent("instance_created", {
                name: "Instance Created",
                type: selectedType,
                minecraftVersion: selectedMinecraftVersion,
                forgeVersion: selectedType === "forge" ? selectedForgeVersion : "none",
                timestamp: new Date().toISOString(),
            });

            setInstanceName("");
            setOpen(false);
            onInstanceCreated();
        } catch (error) {
            console.error("Error al crear la instancia:", error);
            toast.error("No se pudo crear la instancia. Inténtalo de nuevo.");
        } finally {
            setIsLoading(false);
        }
    };

    // On open change, reset selected versions
    const handleOpenChange = (isOpen: boolean) => {
        setOpen(isOpen);
        if (!isOpen) {
            setSelectedMinecraftVersion("");
            setSelectedForgeVersion("");
            setInstanceName("");
            setSelectedType("vanilla");
        }
    };

    const onInteractOutside = (event: Event) => {
        // Prevent closing the dialog when clicking outside if the user has some completed data
        if (instanceName.trim() || selectedMinecraftVersion || selectedForgeVersion) {
            event.preventDefault();
        }
    };

    // Determine if the create button should be disabled
    const isCreateButtonDisabled = isLoading ||
        !instanceName.trim() ||
        !selectedMinecraftVersion ||
        (selectedType === "forge" && !selectedForgeVersion);

    return (
        <Dialog open={open} onOpenChange={handleOpenChange}>
            <DialogTrigger asChild>
                <button
                    className="cursor-pointer aspect-video z-10 group relative overflow-hidden rounded-xl border border-dashed border-white/20 h-auto flex flex-col items-center justify-center
                    transition duration-300 hover:border-sky-400/50 hover:bg-gray-800/30"
                >
                    <div className="flex flex-col items-center gap-3">
                        <div className="p-3 rounded-full bg-gray-800/80 group-hover:bg-sky-900/40 transition">
                            <LucidePlus className="h-8 w-8 text-gray-400 group-hover:text-sky-300" />
                        </div>
                        <span className="text-gray-400 group-hover:text-sky-300 font-medium">Nueva instancia</span>
                    </div>
                </button>
            </DialogTrigger>

            <DialogContent
                onInteractOutside={onInteractOutside}
                className="sm:max-w-md dark max-h-[80vh] overflow-y-scroll outline-none">
                <DialogHeader>
                    <DialogTitle className="from-[#4e9fff] to-[#2954ff] bg-clip-text text-transparent bg-gradient-to-b">
                        Crear nueva instancia
                    </DialogTitle>
                    <DialogDescription>
                        Configura una nueva instancia con el tipo de Minecraft que prefieras.
                    </DialogDescription>
                </DialogHeader>

                <div className="mt-6 space-y-6">
                    {/* Instance Name Input */}
                    <div className="space-y-2">
                        <Label className="text-white" htmlFor="instanceName">Nombre de la instancia</Label>
                        <Input
                            id="instanceName"
                            value={instanceName}
                            onChange={(e) => setInstanceName(e.target.value)}
                            placeholder="Mi nueva instancia"
                        />
                    </div>

                    {/* Instance Type Selection */}
                    <div className="space-y-3">
                        <Label className="text-white">Tipo de instancia</Label>
                        <div className="grid grid-cols-2 gap-4">
                            <div
                                className={`flex flex-col items-center gap-3 p-4 rounded-md border cursor-pointer transition-all ${selectedType === "vanilla"
                                    ? "border-blue-500 bg-blue-900/20"
                                    : "border-gray-700 hover:border-gray-500"
                                    }`}
                                onClick={() => setSelectedType("vanilla")}
                            >
                                <CreeperIcon className="h-12 w-12" />
                                <span className="font-medium text-center">Vanilla</span>
                            </div>

                            <div
                                className={`flex flex-col items-center gap-3 p-4 overflow-hidden rounded-md border cursor-pointer transition-all relative ${selectedType === "forge"
                                    ? "border-orange-500 bg-orange-900/20"
                                    : "border-gray-700 hover:border-gray-500"
                                    }`}
                                onClick={() => setSelectedType("forge")}
                            >
                                <LucideAnvil className="h-12 w-12" />
                                <span className="font-medium text-center">Forge</span>
                                <span className="rounded-bl-lg absolute top-0 right-0 bg-blue-500/20 text-xs text-white px-2 py-1">
                                    Para mods
                                </span>
                            </div>
                        </div>
                    </div>

                    {/* Minecraft Version Selector */}
                    <div className="space-y-2">
                        <Label className="text-white" htmlFor="minecraftVersion">
                            Versión de Minecraft
                        </Label>

                        {loadingVersions ? (
                            <div className="flex items-center justify-center p-2">
                                <Loader2 className="h-5 w-5 animate-spin text-blue-500" />
                                <span className="ml-2 text-sm text-gray-400">Cargando versiones...</span>
                            </div>
                        ) : (
                            <Select
                                value={selectedMinecraftVersion}
                                onValueChange={setSelectedMinecraftVersion}
                            >
                                <SelectTrigger>
                                    <SelectValue placeholder="Selecciona una versión" />
                                </SelectTrigger>
                                <SelectContent className="max-h-80">
                                    {minecraftVersions.map((version) => (
                                        <SelectItem key={version.id} value={version.id}>
                                            {version.id}
                                        </SelectItem>
                                    ))}
                                </SelectContent>
                            </Select>
                        )}
                    </div>

                    {/* Forge Version Selector (only if Forge is selected) */}
                    {selectedType === "forge" && (
                        <div className="space-y-2">
                            <Label className="text-white" htmlFor="forgeVersion">
                                Versión de Forge
                            </Label>

                            {loadingVersions ? (
                                <div className="flex items-center justify-center p-2">
                                    <Loader2 className="h-5 w-5 animate-spin text-orange-500" />
                                    <span className="ml-2 text-sm text-gray-400">Cargando versiones...</span>
                                </div>
                            ) : compatibleForgeVersions.length > 0 ? (
                                <Select
                                    value={selectedForgeVersion}
                                    onValueChange={setSelectedForgeVersion}
                                >
                                    <SelectTrigger>
                                        <SelectValue placeholder="Selecciona una versión de Forge" />
                                    </SelectTrigger>
                                    <SelectContent className="max-h-80">
                                        {compatibleForgeVersions.map((version) => (
                                            <SelectItem key={version} value={version}>
                                                {version}
                                            </SelectItem>
                                        ))}
                                    </SelectContent>
                                </Select>
                            ) : (
                                <div className="p-3 rounded-md bg-yellow-900/20 border border-yellow-700/50">
                                    <p className="text-sm text-yellow-300">
                                        No hay versiones de Forge disponibles para Minecraft {selectedMinecraftVersion}
                                    </p>
                                </div>
                            )}
                        </div>
                    )}
                </div>

                <DialogFooter className="mt-6">
                    <Button
                        type="submit"
                        onClick={handleCreateInstance}
                        disabled={isCreateButtonDisabled}
                        className="w-full cursor-pointer disabled:bg-gray-700 disabled:text-gray-400 disabled:cursor-not-allowed"
                    >
                        {isLoading ? (
                            <>
                                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                                Creando...
                            </>
                        ) : (
                            "Crear instancia"
                        )}
                    </Button>
                </DialogFooter>
            </DialogContent>
        </Dialog>
    );
};