import { Button } from "@/components/ui/button"
import { useState } from "react"
import { invoke } from "@tauri-apps/api/core"
import { LucideDownload, LucideRefreshCw } from "lucide-react"
import { InstallOptionsDialog } from "./InstallOptionsDialog"
import { UpdateInstanceDialog } from "./UpdateInstanceDialog"
import { CreateInstanceDialog } from "./CreateInstanceDialog"
import { TauriCommandReturns } from "@/types/TauriCommandReturns"



interface InstallButtonProps {
    modpackId: string;
    modpackName: string;
    localInstances: TauriCommandReturns["get_instances_by_modpack_id"];
}

export const InstallButton = ({ modpackId, modpackName, localInstances }: InstallButtonProps) => {
    const [isInstallOptionsOpen, setIsInstallOptionsOpen] = useState<boolean>(false)
    const [isUpdateDialogOpen, setIsUpdateDialogOpen] = useState<boolean>(false)
    const [isCreateDialogOpen, setIsCreateDialogOpen] = useState<boolean>(false)
    const [isInstalling, setIsInstalling] = useState<boolean>(false)

    const hasLocalInstances = localInstances.length > 0

    const handleInstallClick = () => {
        if (hasLocalInstances) {
            setIsInstallOptionsOpen(true)
        } else {
            setIsCreateDialogOpen(true)
        }
    }

    const handleUpdateExisting = () => {
        setIsInstallOptionsOpen(false)
        setIsUpdateDialogOpen(true)
    }

    const handleInstallNew = () => {
        setIsInstallOptionsOpen(false)
        setIsCreateDialogOpen(true)
    }

    const handleConfirmUpdate = async (instanceId: string) => {
        setIsInstalling(true)
        try {
            await invoke("update_instance", {
                instanceId,
                modpackId
            })
            // Mostrar mensaje de éxito o redireccionar
            console.log(`Instancia ${instanceId} actualizada exitosamente`)
        } catch (err) {
            console.error("Error al actualizar la instancia:", err)
            // Mostrar mensaje de error
        } finally {
            setIsInstalling(false)
            setIsUpdateDialogOpen(false)
        }
    }

    const handleConfirmCreate = async (instanceName: string) => {
        setIsInstalling(true)
        try {
            await invoke("create_instance", {
                instanceName,
                modpackId
            })
            // Mostrar mensaje de éxito o redireccionar
            console.log(`Nueva instancia creada: ${instanceName}`)
        } catch (err) {
            console.error("Error al crear la instancia:", err)
            // Mostrar mensaje de error
        } finally {
            setIsInstalling(false)
            setIsCreateDialogOpen(false)
        }
    }

    return (
        <>
            <Button
                variant="default"
                className="w-full md:w-auto bg-indigo-600 hover:bg-indigo-700 text-white flex items-center gap-2"
                onClick={handleInstallClick}
                disabled={isInstalling}
            >
                {isInstalling ? (
                    <>
                        <LucideRefreshCw className="w-4 h-4 animate-spin" />
                        Instalando...
                    </>
                ) : (
                    <>
                        <LucideDownload className="w-4 h-4" />
                        Instalar
                    </>
                )}
            </Button>

            <InstallOptionsDialog
                isOpen={isInstallOptionsOpen}
                onClose={() => setIsInstallOptionsOpen(false)}
                modpackId={modpackId}
                modpackName={modpackName}
                localInstances={localInstances}
                onUpdateExisting={handleUpdateExisting}
                onInstallNew={handleInstallNew}
            />

            <UpdateInstanceDialog
                isOpen={isUpdateDialogOpen}
                onClose={() => setIsUpdateDialogOpen(false)}
                modpackId={modpackId}
                modpackName={modpackName}
                localInstances={localInstances}
                onConfirmUpdate={handleConfirmUpdate}
            />

            <CreateInstanceDialog
                isOpen={isCreateDialogOpen}
                onClose={() => setIsCreateDialogOpen(false)}
                modpackId={modpackId}
                modpackName={modpackName}
                onConfirmCreate={handleConfirmCreate}
            />
        </>
    )
}