import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { LucidePencil, LucideSave } from "lucide-react";
import {
    Dialog,
    DialogContent,
    DialogDescription,
    DialogFooter,
    DialogHeader,
    DialogTitle,
    DialogTrigger,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import { TauriCommandReturns } from "@/types/TauriCommandReturns";

interface EditInstanceInfoProps {
    instanceId: string;
    onUpdate?: () => void;
}

export const EditInstanceInfo = ({ instanceId, onUpdate }: EditInstanceInfoProps) => {
    const [open, setOpen] = useState(false);
    const [isLoading, setIsLoading] = useState(false);
    const [instance, setInstance] = useState<TauriCommandReturns['get_instance_by_id'] | null>(null); // Cambia el tipo según tu modelo de instancia
    const [formData, setFormData] = useState({
        instanceName: "",
        icon: "",
    });

    // Cargar la instancia cuando se abre el diálogo
    useEffect(() => {
        const loadInstance = async () => {
            if (!open) return;

            try {
                setIsLoading(true);
                const instance = await invoke<TauriCommandReturns['get_instance_by_id']>(
                    "get_instance_by_id",
                    { instanceId }
                );

                if (instance) {
                    setInstance(instance);
                    setFormData({
                        instanceName: instance.instanceName || "",
                        icon: instance.icon || "",
                    });
                }
            } catch (error) {
                console.error("Error al cargar la instancia:", error);
                toast.error("Error al cargar datos", {
                    description: "No se pudo cargar la información de la instancia.",
                });
            } finally {
                setIsLoading(false);
            }
        };

        loadInstance();
    }, [open, instanceId]);

    const handleInputChange = (e: React.ChangeEvent<HTMLInputElement | HTMLTextAreaElement>) => {
        const { name, value } = e.target;
        setFormData(prev => ({
            ...prev,
            [name]: value
        }));
    };

    const handleSubmit = async (e: React.FormEvent) => {
        e.preventDefault();
        setIsLoading(true);

        try {
            await invoke("update_instance", {
                instance: {
                    ...instance,
                    instanceName: formData.instanceName,
                }
            });

            // Notificar al usuario
            toast.success("Información actualizada", {
                description: "La información de la instancia ha sido actualizada correctamente.",
            });

            // Cerrar el diálogo
            setOpen(false);

            // Llamar al callback si existe
            if (onUpdate) onUpdate();
        } catch (error) {
            console.error("Error al actualizar la instancia:", error);
            toast.error("Error al actualizar", {
                description: "No se pudo actualizar la información de la instancia.",
            });
        } finally {
            setIsLoading(false);
        }
    };

    return (
        <Dialog open={open} onOpenChange={setOpen}>
            <DialogTrigger asChild>
                <button
                    className="cursor-pointer flex items-center gap-x-2 text-white w-full hover:bg-neutral-800 px-3 py-2 rounded-md transition"
                >
                    <LucidePencil className="size-4" />
                    Editar información
                </button>
            </DialogTrigger>
            <DialogContent className="bg-neutral-900 border-neutral-800 text-white">
                <DialogHeader>
                    <DialogTitle>Editar información de la instancia</DialogTitle>
                    <DialogDescription className="text-neutral-400">
                        Modifica los detalles básicos de tu instancia de Minecraft.
                    </DialogDescription>
                </DialogHeader>

                {isLoading && !formData.instanceName ? (
                    <div className="flex items-center justify-center py-8">
                        <div className="flex flex-col items-center gap-2">
                            <div className="animate-spin h-6 w-6 border-2 border-emerald-500 rounded-full border-t-transparent"></div>
                            <p className="text-sm text-neutral-400">Cargando datos...</p>
                        </div>
                    </div>
                ) : (
                    <form onSubmit={handleSubmit} className="space-y-4 mt-4">
                        <div className="space-y-2">
                            <Label htmlFor="instanceName">Nombre de la instancia</Label>
                            <Input
                                id="instanceName"
                                name="instanceName"
                                value={formData.instanceName}
                                onChange={handleInputChange}
                                className="bg-neutral-800 border-neutral-700 text-white"
                                placeholder="Mi instancia de Minecraft"
                                required
                            />
                        </div>



                        <div className="space-y-2">
                            <Label htmlFor="icon">URL del ícono</Label>
                            <Input
                                id="icon"
                                name="icon"
                                value={formData.icon}
                                onChange={handleInputChange}
                                className="bg-neutral-800 border-neutral-700 text-white"
                                placeholder="https://example.com/icon.png"
                            />
                            <p className="text-xs text-neutral-400">
                                Ingresa una URL de imagen para usar como ícono de la instancia.
                            </p>
                        </div>

                        <DialogFooter>
                            <Button
                                type="button"
                                variant="outline"
                                onClick={() => setOpen(false)}
                                className="cursor-pointer bg-neutral-800 hover:bg-neutral-700 text-white border-neutral-700"
                            >
                                Cancelar
                            </Button>
                            <Button
                                type="submit"
                                disabled={isLoading}
                                className="cursor-pointer bg-emerald-600 hover:bg-emerald-700 text-white flex items-center gap-x-2"
                            >
                                {isLoading ? (
                                    <>Guardando...</>
                                ) : (
                                    <>
                                        <LucideSave className="size-4" />
                                        Guardar cambios
                                    </>
                                )}
                            </Button>
                        </DialogFooter>
                    </form>
                )}
            </DialogContent>
        </Dialog>
    );
};