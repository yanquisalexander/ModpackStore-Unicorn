import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { LucideCpu, LucidePencil, LucideSave, LucideUser } from "lucide-react";
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
import {
    Select,
    SelectContent,
    SelectItem,
    SelectTrigger,
    SelectValue,
} from "@/components/ui/select";
import { Alert, AlertDescription, AlertTitle } from "./ui/alert";

interface EditInstanceInfoProps {
    instanceId: string;
    onUpdate?: () => void;
    defaultShowEditInfo?: boolean;
}

export const EditInstanceInfo = ({ instanceId, onUpdate, defaultShowEditInfo }: EditInstanceInfoProps) => {
    const [open, setOpen] = useState(defaultShowEditInfo || false);
    const [isLoading, setIsLoading] = useState(false);
    const [instance, setInstance] = useState<TauriCommandReturns['get_instance_by_id'] | null>(null);
    const [accounts, setAccounts] = useState<TauriCommandReturns['get_all_accounts']>([]);
    const [formData, setFormData] = useState({
        instanceName: "",
        accountUuid: "",
    });

    // Cargar la instancia y las cuentas cuando se abre el diálogo
    useEffect(() => {
        const loadData = async () => {
            if (!open) return;

            try {
                setIsLoading(true);

                // Cargar la instancia
                const instanceData = await invoke<TauriCommandReturns['get_instance_by_id']>(
                    "get_instance_by_id",
                    { instanceId }
                );

                if (instanceData) {
                    setInstance(instanceData);
                    setFormData({
                        instanceName: instanceData.instanceName || "",
                        accountUuid: instanceData.accountUuid || "",
                    });
                }

                // Cargar las cuentas disponibles
                const accountsData = await invoke<TauriCommandReturns['get_all_accounts']>(
                    "get_all_accounts"
                );

                setAccounts(accountsData);
            } catch (error) {
                console.error("Error al cargar datos:", error);
                toast.error("Error al cargar datos", {
                    description: "No se pudo cargar la información necesaria.",
                });
            } finally {
                setIsLoading(false);
            }
        };

        loadData();
    }, [open, instanceId]);

    const handleInputChange = (e: React.ChangeEvent<HTMLInputElement | HTMLTextAreaElement>) => {
        const { name, value } = e.target;
        setFormData(prev => ({
            ...prev,
            [name]: value
        }));
    };

    const handleAccountChange = (value: string) => {
        setFormData(prev => ({
            ...prev,
            accountUuid: value
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
                    accountUuid: formData.accountUuid,
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

                <Alert

                    className=" bg-blue-900/20 border-blue-700/50 text-blue-300"
                >
                    <LucideCpu className="h-4 w-4" />

                    <AlertDescription>
                        <p className="text-white">
                            Los ajustes de rendimiento y recursos (Como RAM, CPU, etc.) se aplican globalmente a todas las instancias desde la configuración de la aplicación.
                        </p>
                    </AlertDescription>
                </Alert>

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
                            <Label htmlFor="accountUuid">Cuenta de Minecraft</Label>
                            <Select
                                value={formData.accountUuid}
                                onValueChange={handleAccountChange}
                            >
                                <SelectTrigger className="bg-neutral-800 border-neutral-700 text-white">
                                    <SelectValue placeholder="Seleccionar cuenta" />
                                </SelectTrigger>
                                <SelectContent className="bg-neutral-800 border-neutral-700 text-white">
                                    {accounts.length === 0 ? (
                                        <SelectItem value="no-accounts" disabled>
                                            No hay cuentas disponibles
                                        </SelectItem>
                                    ) : (
                                        accounts.map((account) => (
                                            <SelectItem
                                                key={account.uuid}
                                                value={account.uuid}
                                                className="flex items-center gap-2"
                                            >
                                                <div className="flex items-center gap-2">
                                                    <LucideUser className="size-4 text-emerald-400" />
                                                    {account.username}
                                                </div>
                                            </SelectItem>
                                        ))
                                    )}
                                </SelectContent>
                            </Select>
                            <p className="text-xs text-neutral-400">
                                Selecciona la cuenta que se usará para iniciar esta instancia.
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