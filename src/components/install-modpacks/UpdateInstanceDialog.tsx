import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog"
import { Button } from "@/components/ui/button"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { useState, useEffect } from "react"
import { TauriCommandReturns } from "@/types/TauriCommandReturns";



interface UpdateInstanceDialogProps {
    isOpen: boolean;
    onClose: () => void;
    modpackId: string;
    modpackName: string;
    localInstances: TauriCommandReturns["get_instances_by_modpack_id"];
    onConfirmUpdate: (instanceId: string) => void;
}

export const UpdateInstanceDialog = ({
    isOpen,
    onClose,
    modpackId,
    modpackName,
    localInstances,
    onConfirmUpdate
}: UpdateInstanceDialogProps) => {
    const [selectedInstance, setSelectedInstance] = useState<string>("")

    useEffect(() => {
        // Seleccionar la primera instancia por defecto cuando se abre el diálogo
        if (isOpen && localInstances.length > 0 && !selectedInstance) {
            setSelectedInstance(localInstances[0].instanceId)
        }
    }, [isOpen, localInstances, selectedInstance])

    return (
        <Dialog open={isOpen} onOpenChange={onClose}>
            <DialogContent className="sm:max-w-md bg-zinc-900 border-zinc-800 text-white">
                <DialogHeader>
                    <DialogTitle>Actualizar instancia de {modpackName}</DialogTitle>
                    <DialogDescription className="text-zinc-400">
                        Selecciona la instancia que deseas actualizar.
                    </DialogDescription>
                </DialogHeader>
                <div className="flex flex-col gap-4 py-4">
                    <Select
                        value={selectedInstance}
                        onValueChange={setSelectedInstance}
                    >
                        <SelectTrigger className="bg-zinc-800 border-zinc-700 text-white">
                            <SelectValue placeholder="Seleccionar instancia" />
                        </SelectTrigger>
                        <SelectContent className="bg-zinc-800 border-zinc-700 text-white">
                            {localInstances.map(instance => (
                                <SelectItem key={instance.instanceId} value={instance.instanceId}>
                                    {instance.instanceName}
                                </SelectItem>
                            ))}
                        </SelectContent>
                    </Select>
                </div>
                <DialogFooter>
                    <Button
                        variant="outline"
                        className="bg-zinc-800 text-white hover:bg-zinc-700"
                        onClick={onClose}
                    >
                        Cancelar
                    </Button>
                    <Button
                        variant="default"
                        className="bg-indigo-600 hover:bg-indigo-700"
                        onClick={() => onConfirmUpdate(selectedInstance)}
                        disabled={!selectedInstance}
                    >
                        Actualizar
                    </Button>
                </DialogFooter>
            </DialogContent>
        </Dialog>
    )
}