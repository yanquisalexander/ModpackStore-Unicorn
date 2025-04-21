import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from "@/components/ui/dialog"
import { Button } from "@/components/ui/button"
import { LucideDownload, LucideRefreshCw } from "lucide-react"
import { TauriCommandReturns } from "@/types/TauriCommandReturns"


interface InstallOptionsDialogProps {
    isOpen: boolean;
    onClose: () => void;
    modpackId: string;
    modpackName: string;
    localInstances: TauriCommandReturns["get_instances_by_modpack_id"];
    onInstallNew: () => void;
    onUpdateExisting: () => void;
}

export const InstallOptionsDialog = ({
    isOpen,
    onClose,
    modpackId,
    modpackName,
    localInstances,
    onInstallNew,
    onUpdateExisting
}: InstallOptionsDialogProps) => {
    return (
        <Dialog open={isOpen} onOpenChange={onClose}>
            <DialogContent className="sm:max-w-md bg-zinc-900 border-zinc-800 text-white">
                <DialogHeader>
                    <DialogTitle>Instalar {modpackName}</DialogTitle>
                    <DialogDescription className="text-zinc-400">
                        Ya tienes instalaciones de este modpack. ¿Qué deseas hacer?
                    </DialogDescription>
                </DialogHeader>
                <div className="flex flex-col gap-4 py-4">
                    <Button
                        variant="outline"
                        className="w-full flex items-center gap-2 bg-zinc-800 text-white hover:bg-zinc-700"
                        onClick={onUpdateExisting}
                    >
                        <LucideRefreshCw className="w-4 h-4" />
                        Actualizar una instancia existente
                    </Button>
                    <Button
                        variant="default"
                        className="w-full flex items-center gap-2 bg-emerald-600 hover:bg-emerald-700"
                        onClick={onInstallNew}
                    >
                        <LucideDownload className="w-4 h-4" />
                        Crear una nueva instancia
                    </Button>
                </div>
            </DialogContent>
        </Dialog>
    )
}